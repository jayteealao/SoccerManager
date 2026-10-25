//! The confidence interval of a band's engine value minus its real value.
//!
//! One bootstrap resamples both sides. The engine side draws its outer units (whole
//! generated leagues, or whole matches for a band that plays fixed clubs) with replacement.
//! The real side is drawn by its source: whole matches within each stratum for a file with
//! match rows; the mean plus its leave-one-out error times a t variate for tracked matches;
//! the value plus its error times a normal variate for a summary with no rows; nothing for a
//! design target. The interval is the studentized (percentile-t) interval: each replicate's
//! difference is divided by its own error, so a run with few leagues gets the wide interval
//! it deserves. Each side of the interval is at `1 - alpha`, the equivalence convention.

use crate::bounds::{clopper_pearson, garwood};
use crate::special::{norm_quantile, t_quantile};

/// One outer unit's totals: the band's numerator and denominator summed over the unit's
/// matches, and, for a measure scaled by a second ratio (the factorial-moment ratio, a red
/// card arm over its control), that ratio's numerator and denominator. The unit set's value
/// is `(sum num / sum den) * (sum scale_num / sum scale_den)`; a plain ratio or mean has both
/// scale terms equal (one per match). A mean band has a denominator of one per match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    pub num: f64,
    pub den: f64,
    pub scale_num: f64,
    pub scale_den: f64,
}

impl Unit {
    /// A unit of a plain ratio (or a mean, with `den` its match count).
    pub fn ratio(num: f64, den: f64) -> Self {
        Unit {
            num,
            den,
            scale_num: 1.0,
            scale_den: 1.0,
        }
    }
}

/// The real side of a band.
#[derive(Debug, Clone, PartialEq)]
pub enum Real {
    /// Whole real matches by stratum (a division, a league). The estimate is the mean of the
    /// strata's ratios, each stratum weighted equally, and a replicate resamples matches
    /// within each stratum.
    Matches(Vec<Vec<Unit>>),
    /// A value and its error from a file with no match rows.
    Summary { value: f64, se: f64 },
    /// One value per tracked match; the error leaves one match out.
    Tracked(Vec<f64>),
    /// A design target, known exactly.
    Fixed(f64),
}

/// A band's interval of engine minus real.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    /// The difference on the unresampled data.
    pub point: f64,
    pub lo: f64,
    pub hi: f64,
    /// The combined error of the point.
    pub se: f64,
    /// Replicates dropped because their error was zero.
    pub dropped: usize,
}

/// A band whose interval cannot be formed reads not sure with this reason.
pub type NoInterval = String;

/// The value of the units and its error, studentized at unit level: the delta-method
/// (linearized) error with the units as clusters. Each unit's influence is
/// `v (num/N - den/D + scale_num/S - scale_den/C)` over the totals, and the squared error is
/// `G / (G - 1)` times the sum of the centred squared influences. For a plain ratio it is
/// `sqrt(G / (G - 1) * sum((num - r den)^2)) / sum(den)`.
pub fn ratio_and_se(units: &[Unit]) -> (f64, f64) {
    let total = |f: fn(&Unit) -> f64| units.iter().map(f).sum::<f64>();
    let (n, d) = (total(|u| u.num), total(|u| u.den));
    let (sn, sd) = (total(|u| u.scale_num), total(|u| u.scale_den));
    if d == 0.0 || sd == 0.0 {
        return (f64::NAN, f64::INFINITY);
    }
    let v = n / d * (sn / sd);
    if units.len() < 2 {
        return (v, f64::INFINITY);
    }
    let part = |x: f64, t: f64| if t != 0.0 { x / t } else { 0.0 };
    let psi: Vec<f64> = units
        .iter()
        .map(|u| v * (part(u.num, n) - u.den / d + part(u.scale_num, sn) - u.scale_den / sd))
        .collect();
    let g = units.len() as f64;
    let mean = psi.iter().sum::<f64>() / g;
    let ss: f64 = psi.iter().map(|p| (p - mean).powi(2)).sum();
    (v, (g / (g - 1.0) * ss).sqrt())
}

/// The leave-one-out (jackknife) error of the mean of `values`; for a mean it equals the
/// sample standard deviation over the square root of `n`.
pub fn loo_error(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return f64::INFINITY;
    }
    let nf = n as f64;
    let total: f64 = values.iter().sum();
    let loo: Vec<f64> = values.iter().map(|v| (total - v) / (nf - 1.0)).collect();
    let mean = loo.iter().sum::<f64>() / nf;
    ((nf - 1.0) / nf * loo.iter().map(|m| (m - mean).powi(2)).sum::<f64>()).sqrt()
}

/// The interval of the mean of tracked matches alone: the mean plus and minus
/// `t(1 - alpha, n - 1)` leave-one-out errors.
pub fn tracked_interval(values: &[f64], alpha: f64) -> (f64, f64) {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let half = t_quantile(1.0 - alpha, n - 1.0) * loo_error(values);
    (mean - half, mean + half)
}

impl Real {
    /// The real estimate and its error on the unresampled data.
    pub fn estimate(&self) -> (f64, f64) {
        match self {
            Real::Matches(strata) => {
                let parts: Vec<(f64, f64)> = strata.iter().map(|s| ratio_and_se(s)).collect();
                let k = parts.len() as f64;
                let value = parts.iter().map(|p| p.0).sum::<f64>() / k;
                let se = parts.iter().map(|p| p.1.powi(2)).sum::<f64>().sqrt() / k;
                (value, se)
            }
            Real::Summary { value, se } => (*value, *se),
            Real::Tracked(values) => {
                let n = values.len() as f64;
                (values.iter().sum::<f64>() / n, loo_error(values))
            }
            Real::Fixed(value) => (*value, 0.0),
        }
    }

    fn replicate(&self, draw: &mut dyn FnMut() -> f64) -> (f64, f64) {
        match self {
            Real::Matches(strata) => {
                let resampled: Vec<Vec<Unit>> = strata.iter().map(|s| resample(s, draw)).collect();
                Real::Matches(resampled).estimate()
            }
            Real::Summary { value, se } => (value + se * normal(draw), *se),
            Real::Tracked(values) => {
                let n = values.len();
                let mean = values.iter().sum::<f64>() / n as f64;
                let se = loo_error(values);
                (mean + se * student(n.saturating_sub(1).max(1), draw), se)
            }
            Real::Fixed(value) => (*value, 0.0),
        }
    }
}

fn index(n: usize, draw: &mut dyn FnMut() -> f64) -> usize {
    ((draw() * n as f64) as usize).min(n - 1)
}

fn resample(units: &[Unit], draw: &mut dyn FnMut() -> f64) -> Vec<Unit> {
    (0..units.len())
        .map(|_| units[index(units.len(), draw)])
        .collect()
}

fn normal(draw: &mut dyn FnMut() -> f64) -> f64 {
    norm_quantile(draw().clamp(1e-12, 1.0 - 1e-12))
}

/// A Student t variate with `df` degrees of freedom: a normal over the root of a chi-square
/// made of `df` squared normals, divided by `df`.
fn student(df: usize, draw: &mut dyn FnMut() -> f64) -> f64 {
    let z = normal(draw);
    let chi2: f64 = (0..df).map(|_| normal(draw).powi(2)).sum();
    z / (chi2 / df as f64).sqrt()
}

/// The `q` quantile of sorted values: the `ceil((B + 1) q)`-th smallest of `B`, the rank rule
/// the existing bootstrap bands use.
pub fn quantile_rank(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let k = ((sorted.len() as f64 + 1.0) * q).ceil() as usize;
    sorted[k.clamp(1, sorted.len()) - 1]
}

/// The studentized bootstrap interval of engine minus real, each side at `1 - alpha`, over
/// `replicates` resamples. `draw` returns uniform values in `[0, 1)` from the caller's own
/// seeded stream, so the same run gives the same interval.
///
/// A band with fewer than two engine units, or with more than a tenth of its replicates
/// dropped for a zero error, has no interval; the reason says which.
pub fn interval(
    engine: &[Unit],
    real: &Real,
    alpha: f64,
    replicates: usize,
    draw: &mut dyn FnMut() -> f64,
) -> Result<Interval, NoInterval> {
    if engine.len() < 2 {
        return Err(format!(
            "{} outer unit(s); at least 2 are needed",
            engine.len()
        ));
    }
    let (e, se_e) = ratio_and_se(engine);
    let (r, se_r) = real.estimate();
    let point = e - r;
    let se = (se_e.powi(2) + se_r.powi(2)).sqrt();
    if !point.is_finite() {
        return Err("the engine or the real value is not a number".to_string());
    }
    if !(se.is_finite() && se > 0.0) {
        return Err("the combined error is zero or unknown".to_string());
    }
    let mut ts = Vec::with_capacity(replicates);
    let mut dropped = 0;
    for _ in 0..replicates {
        let (e_b, se_e_b) = ratio_and_se(&resample(engine, draw));
        let (r_b, se_r_b) = real.replicate(draw);
        let se_b = (se_e_b.powi(2) + se_r_b.powi(2)).sqrt();
        let t = (e_b - r_b - point) / se_b;
        if se_b > 0.0 && t.is_finite() {
            ts.push(t);
        } else {
            dropped += 1;
        }
    }
    if dropped * 10 > replicates {
        return Err(format!(
            "{dropped} of {replicates} resamples had a zero error; too few to judge"
        ));
    }
    ts.sort_by(f64::total_cmp);
    Ok(Interval {
        point,
        lo: point - quantile_rank(&ts, 1.0 - alpha) * se,
        hi: point - quantile_rank(&ts, alpha) * se,
        se,
        dropped,
    })
}

/// Whether a rare band's engine side is a share (events over trials) or a rate per match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RareKind {
    Share,
    Rate,
}

/// The interval of a rare band from exact bounds: the engine's Clopper-Pearson (share) or
/// Garwood (rate) bounds and the real side's bounds, each at `1 - alpha / 2`, combined as
/// `[engine_lo - real_hi, engine_hi - real_lo]`, which covers the difference with at least
/// `1 - alpha`. `exposure` is the trials of a share or the matches of a rate.
pub fn rare_interval(
    kind: RareKind,
    events: u64,
    exposure: f64,
    real: (f64, f64, f64),
    alpha: f64,
) -> Interval {
    let confidence = 1.0 - alpha / 2.0;
    let (e_lo, e_hi) = match kind {
        RareKind::Share => clopper_pearson(events, exposure.round().max(0.0) as u64, confidence),
        RareKind::Rate => garwood(events, exposure, confidence),
    };
    let (r_value, r_lo, r_hi) = real;
    let e_point = if exposure > 0.0 {
        events as f64 / exposure
    } else {
        f64::NAN
    };
    Interval {
        point: e_point - r_value,
        lo: e_lo - r_hi,
        hi: e_hi - r_lo,
        se: f64::NAN,
        dropped: 0,
    }
}

/// The real side's bounds of a rare band from its value and error, each at `1 - alpha / 2`,
/// held at zero or above: `(value, lo, hi)`.
pub fn real_bounds(value: f64, se: f64, alpha: f64) -> (f64, f64, f64) {
    let z = norm_quantile(1.0 - alpha / 2.0);
    (value, (value - z * se).max(0.0), value + z * se)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small seeded uniform stream for the tests (SplitMix64).
    pub(crate) fn stream(seed: u64) -> impl FnMut() -> f64 {
        let mut s = seed;
        move || {
            s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            (z >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    fn units(values: &[f64]) -> Vec<Unit> {
        values.iter().map(|&v| Unit::ratio(v, 1.0)).collect()
    }

    #[test]
    fn the_ratio_error_is_the_cluster_error_of_a_mean_for_unit_denominators() {
        let (r, se) = ratio_and_se(&units(&[1.0, 2.0, 3.0, 4.0]));
        assert_eq!(r, 2.5);
        // sd of 1..4 is sqrt(5/3); over sqrt(4).
        assert!((se - (5.0_f64 / 3.0).sqrt() / 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_tracked_form_is_the_mean_plus_t_leave_one_out_errors() {
        let v = [3.1, 2.7, 3.4, 2.9, 3.6, 3.0, 2.8];
        let n = v.len() as f64;
        let mean = v.iter().sum::<f64>() / n;
        let sd = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        assert!((loo_error(&v) - sd / n.sqrt()).abs() < 1e-12);
        let (lo, hi) = tracked_interval(&v, 0.05);
        let half = t_quantile(0.95, 6.0) * sd / n.sqrt();
        assert!((lo - (mean - half)).abs() < 1e-12);
        assert!((hi - (mean + half)).abs() < 1e-12);
    }

    #[test]
    fn a_scaled_ratio_takes_its_error_from_each_units_influence() {
        // Two arms over a control: the value is (A / B) * (S / C).
        let u = [
            Unit {
                num: 3.0,
                den: 2.0,
                scale_num: 2.0,
                scale_den: 4.0,
            },
            Unit {
                num: 5.0,
                den: 2.0,
                scale_num: 2.0,
                scale_den: 2.0,
            },
            Unit {
                num: 4.0,
                den: 2.0,
                scale_num: 2.0,
                scale_den: 3.0,
            },
        ];
        let (v, se) = ratio_and_se(&u);
        assert!((v - 12.0 / 6.0 * (6.0 / 9.0)).abs() < 1e-12);
        assert!(se > 0.0 && se.is_finite());
        // A plain ratio's influence error equals the cluster ratio error.
        let plain = [
            Unit::ratio(3.0, 2.0),
            Unit::ratio(5.0, 2.0),
            Unit::ratio(1.0, 1.0),
        ];
        let (r, se) = ratio_and_se(&plain);
        let ss: f64 = plain.iter().map(|u| (u.num - r * u.den).powi(2)).sum();
        assert!((se - (1.5 * ss).sqrt() / 5.0).abs() < 1e-12);
    }

    #[test]
    fn one_outer_unit_has_no_interval() {
        let mut draw = stream(1);
        let err = interval(&units(&[2.0]), &Real::Fixed(2.0), 0.05, 199, &mut draw);
        assert!(err.unwrap_err().contains("at least 2"));
    }

    #[test]
    fn the_interval_holds_the_point_and_widens_with_fewer_units() {
        let many: Vec<f64> = (0..30)
            .map(|i| 2.0 + 0.1 * ((i * 7 % 11) as f64 - 5.0))
            .collect();
        let few = &many[..6];
        let real = Real::Summary {
            value: 2.0,
            se: 0.02,
        };
        let wide = interval(&units(few), &real, 0.05, 1999, &mut stream(3)).unwrap();
        let narrow = interval(&units(&many), &real, 0.05, 1999, &mut stream(3)).unwrap();
        assert!(narrow.lo < narrow.point && narrow.point < narrow.hi);
        assert!(wide.hi - wide.lo > narrow.hi - narrow.lo);
        // The same stream gives the same interval.
        let again = interval(&units(&many), &real, 0.05, 1999, &mut stream(3)).unwrap();
        assert_eq!(again, narrow);
    }

    #[test]
    fn real_matches_resample_within_each_stratum_with_equal_weight() {
        let a = units(&[1.0, 1.2, 0.8, 1.0]);
        let b = units(&[3.0, 3.2, 2.8, 3.0]);
        let (value, se) = Real::Matches(vec![a, b]).estimate();
        assert!((value - 2.0).abs() < 1e-12);
        assert!(se > 0.0);
    }

    #[test]
    fn zero_events_give_the_exact_engine_bound() {
        let iv = rare_interval(RareKind::Rate, 0, 1000.0, (0.0, 0.0, 0.0), 0.10);
        // One-sided 95 percent: -ln(0.05) / 1000.
        assert!((iv.hi - -(0.05_f64).ln() / 1000.0).abs() < 1e-9);
        assert_eq!(iv.lo, 0.0);
    }
}
