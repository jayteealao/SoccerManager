//! How closely paired matches agree.
//!
//! A change run plays each fixture on both engines with the same seed. The closer the two
//! matches of a pair agree, the less the paired change wanders, and the fewer matches the
//! old-engine word needs. A band's agreement is the Pearson correlation of its per-match
//! values over the kept pairs, with a Fisher z interval; a run's agreement is the median over
//! bands, with a percentile interval from resampling the bands.

use serde::{Deserialize, Serialize};

use crate::interval::quantile_rank;
use crate::special::norm_quantile;

/// The confidence of an agreement interval.
pub const CONFIDENCE: f64 = 0.90;

/// The fewest pairs a correlation interval needs (Fisher's interval divides by `n - 3`).
pub const MIN_PAIRS: usize = 4;

/// An agreement with its interval and the number of pairs (or bands) behind it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Agreement {
    pub r: f64,
    pub lo: f64,
    pub hi: f64,
    pub n: usize,
}

/// The Pearson correlation of paired values, or why there is none.
pub fn pearson(pairs: &[(f64, f64)]) -> Result<f64, String> {
    let pairs: Vec<(f64, f64)> = pairs
        .iter()
        .copied()
        .filter(|(a, b)| a.is_finite() && b.is_finite())
        .collect();
    if pairs.len() < 2 {
        return Err(format!("{} pair(s); a correlation needs 2", pairs.len()));
    }
    let n = pairs.len() as f64;
    let (ma, mb) = pairs
        .iter()
        .fold((0.0, 0.0), |(x, y), (a, b)| (x + a / n, y + b / n));
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (a, b) in &pairs {
        sab += (a - ma) * (b - mb);
        saa += (a - ma).powi(2);
        sbb += (b - mb).powi(2);
    }
    if saa <= 0.0 || sbb <= 0.0 {
        return Err("one side has no spread".to_string());
    }
    Ok((sab / (saa * sbb).sqrt()).clamp(-1.0, 1.0))
}

/// Fisher's z interval of a correlation `r` over `n` pairs at `confidence`, two-sided.
pub fn fisher_interval(r: f64, n: usize, confidence: f64) -> (f64, f64) {
    let edge = 1.0 - 1e-12;
    let z = r.clamp(-edge, edge).atanh();
    let half = norm_quantile((1.0 + confidence) / 2.0) / ((n as f64) - 3.0).sqrt();
    ((z - half).tanh(), (z + half).tanh())
}

/// One band's agreement over its pairs (changed, old), at [`CONFIDENCE`].
pub fn pair_agreement(pairs: &[(f64, f64)]) -> Result<Agreement, String> {
    let n = pairs
        .iter()
        .filter(|(a, b)| a.is_finite() && b.is_finite())
        .count();
    if n < MIN_PAIRS {
        return Err(format!("{n} pair(s); an interval needs {MIN_PAIRS}"));
    }
    let r = pearson(pairs)?;
    let (lo, hi) = fisher_interval(r, n, CONFIDENCE);
    Ok(Agreement { r, lo, hi, n })
}

fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// A run's agreement: the median of the bands' correlations, with a percentile interval at
/// [`CONFIDENCE`] from `replicates` resamples of the bands. `draw` returns uniform values in
/// `[0, 1)` from the caller's seeded stream, so the same run gives the same interval.
pub fn summary(
    per_band: &[f64],
    replicates: usize,
    draw: &mut dyn FnMut() -> f64,
) -> Result<Agreement, String> {
    let rs: Vec<f64> = per_band.iter().copied().filter(|r| r.is_finite()).collect();
    if rs.is_empty() {
        return Err("no band has an agreement".to_string());
    }
    let mut sorted = rs.clone();
    sorted.sort_by(f64::total_cmp);
    let r = median(&sorted);
    let mut medians = Vec::with_capacity(replicates);
    for _ in 0..replicates {
        let mut sample: Vec<f64> = (0..rs.len())
            .map(|_| rs[((draw() * rs.len() as f64) as usize).min(rs.len() - 1)])
            .collect();
        sample.sort_by(f64::total_cmp);
        medians.push(median(&sample));
    }
    medians.sort_by(f64::total_cmp);
    let tail = (1.0 - CONFIDENCE) / 2.0;
    let (lo, hi) = if medians.is_empty() {
        (r, r)
    } else {
        (
            quantile_rank(&medians, tail),
            quantile_rank(&medians, 1.0 - tail),
        )
    };
    Ok(Agreement {
        r,
        lo,
        hi,
        n: rs.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fisher_interval_of_one_half_over_403_pairs() {
        let (lo, hi) = fisher_interval(0.5, 403, CONFIDENCE);
        assert!((lo - 0.4358).abs() < 1e-4, "lo {lo}");
        assert!((hi - 0.5591).abs() < 1e-4, "hi {hi}");
    }

    #[test]
    fn perfect_agreement_is_one_and_its_interval_stays_below_one() {
        let pairs: Vec<(f64, f64)> = (0..10).map(|i| (i as f64, 2.0 * i as f64 + 1.0)).collect();
        let a = pair_agreement(&pairs).unwrap();
        assert!((a.r - 1.0).abs() < 1e-12);
        assert!(a.lo < 1.0 && a.hi <= 1.0 && a.lo > 0.99);
    }

    #[test]
    fn the_correlation_of_known_pairs() {
        // x = 1..5, y = 2, 4, 5, 4, 5: sab 6, saa 10, sbb 6, so r = 6 / sqrt(60) = 0.7746.
        let pairs = [(1.0, 2.0), (2.0, 4.0), (3.0, 5.0), (4.0, 4.0), (5.0, 5.0)];
        let r = pearson(&pairs).unwrap();
        assert!((r - 6.0 / (10.0_f64 * 6.0).sqrt()).abs() < 1e-12, "r {r}");
    }

    #[test]
    fn too_few_pairs_or_no_spread_give_no_agreement() {
        assert!(pair_agreement(&[(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]).is_err());
        assert!(pair_agreement(&[(1.0, 1.0), (1.0, 2.0), (1.0, 3.0), (1.0, 4.0)]).is_err());
    }

    #[test]
    fn the_run_agreement_is_the_median_over_bands_with_an_interval() {
        let mut state = 7_u64;
        let mut draw = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        let a = summary(&[0.2, 0.4, 0.5, 0.6, 0.9], 999, &mut draw).unwrap();
        assert_eq!(a.r, 0.5);
        assert_eq!(a.n, 5);
        assert!(a.lo <= 0.5 && a.hi >= 0.5 && a.lo >= 0.2 && a.hi <= 0.9);
    }
}
