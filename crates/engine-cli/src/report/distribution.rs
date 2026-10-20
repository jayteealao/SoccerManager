//! The whole-distribution test of the realism bands. A distribution band's value is a
//! distance between two whole samples, each a list of per-match histograms: the Wasserstein-1
//! distance for a count or a length (the area between the two cumulative distributions), or
//! the total-variation distance for the cells of a table (half the summed absolute
//! difference of the two shares). Both are computed as scipy computes them
//! (`scipy.stats.wasserstein_distance`, and `scipy.spatial.distance.cityblock` halved), which
//! the fixture test checks.
//!
//! The test is an equivalence test: the engine's sample and the real sample are each
//! resampled by whole match; the 97th percentile of the resampled distance is the one-sided
//! upper bound, and the band passes when it is within the design margin; it fails when the
//! 3rd percentile is beyond the margin, and is not sure otherwise. A perfect engine still
//! shows a distance at these sample sizes: the run's noise floor is the 97th percentile of
//! the centred resampled distance, and a margin at or under it can never pass, so the band
//! reads not sure with "margin below noise floor".
//!
//! A promoted band joins the change run's max-t family through [`PairedHist`]: the distance
//! between the two engines over the paired matches, standardized by the root mean square of
//! its centred replicates on the same resampled pair weights.

use engine::rng::EngineRng;

use super::verdict::{FALSE_ALARM, Word};

/// A per-match histogram: each non-empty bin and its count. Bin `k` stands at the point
/// `k · width`; the last bin holds that point and everything above it.
pub type Hist = Vec<(u32, f64)>;

/// Which distance a band uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    /// Wasserstein-1: the area between the two cumulative distributions.
    W1,
    /// Total variation: half the summed absolute difference of the two shares.
    Tv,
}

/// A whole sample: one histogram per match, over `bins` bins of `width`.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub per_match: Vec<Hist>,
    pub bins: u32,
    pub width: f64,
}

/// The bin of `value`: its multiple of `width`, rounded down, the last bin for every value
/// at or above it, and bin 0 for a negative value.
pub fn bin_of(value: f64, width: f64, bins: u32) -> u32 {
    let k = (value / width).floor();
    if k.is_nan() || k <= 0.0 {
        0
    } else if k >= f64::from(bins - 1) {
        bins - 1
    } else {
        // Below `bins - 1`, so it fits.
        k as u32
    }
}

/// The Wasserstein-1 distance between two weighted samples of points, as scipy computes it:
/// over the sorted union of the points, the sum of the absolute difference of the two
/// normalized cumulative weights times the gap to the next point. `NaN` when a sample is
/// empty or weighs nothing. The bands bin their samples, so the run reads the binned form;
/// this point form is the reference the binned form is checked against.
#[cfg(test)]
pub fn wasserstein1(u: &[(f64, f64)], v: &[(f64, f64)]) -> f64 {
    let sorted = |s: &[(f64, f64)]| {
        let mut s = s.to_vec();
        s.sort_by(|a, b| a.0.total_cmp(&b.0));
        s
    };
    let (u, v) = (sorted(u), sorted(v));
    let (tu, tv): (f64, f64) = (u.iter().map(|p| p.1).sum(), v.iter().map(|p| p.1).sum());
    if u.is_empty() || v.is_empty() || tu <= 0.0 || tv <= 0.0 {
        return f64::NAN;
    }
    let mut all: Vec<f64> = u.iter().chain(&v).map(|p| p.0).collect();
    all.sort_by(f64::total_cmp);
    let (mut iu, mut iv, mut cu, mut cv) = (0, 0, 0.0f64, 0.0f64);
    let mut total = 0.0;
    for w in all.windows(2) {
        let x = w[0];
        while iu < u.len() && u[iu].0 <= x {
            cu += u[iu].1;
            iu += 1;
        }
        while iv < v.len() && v[iv].0 <= x {
            cv += v[iv].1;
            iv += 1;
        }
        total += (cu / tu - cv / tv).abs() * (w[1] - x);
    }
    total
}

/// The Wasserstein-1 distance between two dense histograms over the same bins of `width`:
/// `width · Σ |cumA/ΣA − cumB/ΣB|`, equal to [`wasserstein1`] on the bin points. `NaN` when
/// a histogram is empty.
#[cfg(test)]
pub fn w1_bins(a: &[f64], b: &[f64], width: f64) -> f64 {
    distance(Metric::W1, a, b, width).unwrap_or(f64::NAN)
}

/// The total-variation distance between two dense histograms over the same cells: half the
/// summed absolute difference of their shares. `NaN` when a histogram is empty.
#[cfg(test)]
pub fn total_variation(a: &[f64], b: &[f64]) -> f64 {
    distance(Metric::Tv, a, b, 1.0).unwrap_or(f64::NAN)
}

/// The profile a distance compares: the normalized cumulative histogram (Wasserstein-1) or
/// the normalized histogram (total variation); `None` when it weighs nothing.
fn profile(metric: Metric, h: &[f64]) -> Option<Vec<f64>> {
    let total: f64 = h.iter().sum();
    if total <= 0.0 || !total.is_finite() {
        return None;
    }
    let mut out = Vec::with_capacity(h.len());
    let mut run = 0.0;
    for x in h {
        match metric {
            Metric::W1 => {
                run += x;
                out.push(run / total);
            }
            Metric::Tv => out.push(x / total),
        }
    }
    Some(out)
}

/// The factor on the summed absolute difference of two profiles: the bin width for
/// Wasserstein-1, a half for total variation.
fn scale(metric: Metric, width: f64) -> f64 {
    match metric {
        Metric::W1 => width,
        Metric::Tv => 0.5,
    }
}

/// The distance between two profiles: `k` times their summed absolute difference.
fn gap(k: f64, pa: &[f64], pb: &[f64]) -> f64 {
    k * pa.iter().zip(pb).map(|(x, y)| (x - y).abs()).sum::<f64>()
}

/// The distance between two dense histograms; `None` when either weighs nothing.
#[cfg(test)]
fn distance(metric: Metric, a: &[f64], b: &[f64], width: f64) -> Option<f64> {
    let (pa, pb) = (profile(metric, a)?, profile(metric, b)?);
    Some(gap(scale(metric, width), &pa, &pb))
}

/// Adds `times` copies of the histogram `h` into the dense `into`.
fn add(into: &mut [f64], h: &Hist, times: f64) {
    for &(k, n) in h {
        if let Some(slot) = into.get_mut(k as usize) {
            *slot += n * times;
        }
    }
}

/// The dense histogram of every match of `per_match`, each once.
fn pooled(per_match: &[Hist], bins: u32) -> Vec<f64> {
    let mut out = vec![0.0; bins as usize];
    for h in per_match {
        add(&mut out, h, 1.0);
    }
    out
}

/// The `q` quantile of sorted values by the bootstrap's rule: the `ceil((B + 1) q)`-th
/// smallest of `B`.
fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let k = ((sorted.len() as f64 + 1.0) * q).ceil() as usize;
    sorted[k.clamp(1, sorted.len()) - 1]
}

/// A distribution band judged against its real sample.
#[derive(Debug, Clone, PartialEq)]
pub struct DistCheck {
    /// The distance between the engine's whole sample and the real one.
    pub distance: f64,
    /// The standard deviation of the resampled distance.
    pub se: f64,
    /// The one-sided upper bound: the 97th percentile of the resampled distance.
    pub upper: f64,
    /// The 3rd percentile of the resampled distance.
    pub lower: f64,
    /// The noise floor: the 97th percentile of the centred resampled distance, about the
    /// distance a perfect engine shows at these sample sizes.
    pub floor: f64,
    pub word: Word,
    pub note: String,
}

/// The word of a distance with its bounds against `margin`: not sure when the margin is at
/// or under the noise floor, as the band can never pass; pass when the upper bound is within
/// the margin; fail when the lower bound is beyond it; not sure otherwise.
pub fn word(upper: f64, lower: f64, floor: f64, margin: f64) -> (Word, String) {
    let r = |x: f64| engine::observe::round_to(x, 5);
    if margin <= floor {
        (
            Word::NotSure,
            format!("margin below noise floor (floor {})", r(floor)),
        )
    } else {
        let w = if upper <= margin {
            Word::Pass
        } else if lower > margin {
            Word::Fail
        } else {
            Word::NotSure
        };
        (
            w,
            format!("upper bound {}, noise floor {}", r(upper), r(floor)),
        )
    }
}

/// The stream label mixed into the run seed for a band's range test, apart from every
/// match's stream and from the paired test's.
const DIST_LABEL: u64 = 0x6469_7374_5f77_315f;

/// The seed of the band `name`'s range test: its own stream, so bands share no draws and
/// their order does not matter.
pub fn stream(seed: u64, name: &str) -> u64 {
    // FNV-1a over the name.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    seed ^ DIST_LABEL ^ h
}

/// The equivalence test of an engine's sample against the real sample, `resamples` whole-
/// match resamples of both from the stream `seed`. `Err` says why there is no value: fewer
/// than two matches on a side, or a side that holds nothing.
pub fn judge_to_real(
    engine: &Sample,
    real: &Sample,
    metric: Metric,
    margin: f64,
    seed: u64,
    resamples: usize,
) -> Result<DistCheck, String> {
    let bins = engine.bins;
    if real.bins != bins {
        return Err(format!(
            "the real sample has {} bins, the band {bins}",
            real.bins
        ));
    }
    let (ne, nr) = (engine.per_match.len(), real.per_match.len());
    if ne < 2 || nr < 2 {
        return Err(format!(
            "a whole sample needs at least 2 matches a side ({ne} in this run, {nr} real)"
        ));
    }
    let (e, r) = (
        pooled(&engine.per_match, bins),
        pooled(&real.per_match, bins),
    );
    let (Some(pe), Some(pr)) = (profile(metric, &e), profile(metric, &r)) else {
        return Err("a sample holds nothing to compare".into());
    };
    let k = scale(metric, engine.width);
    let observed_diff: Vec<f64> = pe.iter().zip(&pr).map(|(a, b)| a - b).collect();
    let observed = gap(k, &pe, &pr);
    let mut rng = EngineRng::from_seed(seed);
    let (mut ds, mut cs) = (Vec::with_capacity(resamples), Vec::with_capacity(resamples));
    let mut he = vec![0.0; bins as usize];
    let mut hr = vec![0.0; bins as usize];
    for _ in 0..resamples {
        he.iter_mut().for_each(|x| *x = 0.0);
        hr.iter_mut().for_each(|x| *x = 0.0);
        for _ in 0..ne {
            add(&mut he, &engine.per_match[rng.range_usize(ne)], 1.0);
        }
        for _ in 0..nr {
            add(&mut hr, &real.per_match[rng.range_usize(nr)], 1.0);
        }
        let (Some(a), Some(b)) = (profile(metric, &he), profile(metric, &hr)) else {
            continue;
        };
        let c: f64 = a
            .iter()
            .zip(&b)
            .zip(&observed_diff)
            .map(|((x, y), o)| (x - y - o).abs())
            .sum();
        ds.push(gap(k, &a, &b));
        cs.push(k * c);
    }
    if ds.is_empty() {
        return Err("no resample holds anything to compare".into());
    }
    let mean = ds.iter().sum::<f64>() / ds.len() as f64;
    let se = (ds.iter().map(|d| (d - mean) * (d - mean)).sum::<f64>() / ds.len() as f64).sqrt();
    ds.sort_by(f64::total_cmp);
    cs.sort_by(f64::total_cmp);
    let upper = quantile(&ds, 1.0 - FALSE_ALARM);
    let lower = quantile(&ds, FALSE_ALARM);
    let floor = quantile(&cs, 1.0 - FALSE_ALARM);
    let (w, note) = word(upper, lower, floor, margin);
    Ok(DistCheck {
        distance: observed,
        se,
        upper,
        lower,
        floor,
        word: w,
        note,
    })
}

/// One distribution band of a change run: each engine's histogram per paired match, in the
/// order of the suite set's pairs, and the changed engine's bounds against the real sample.
#[derive(Debug, Clone, PartialEq)]
pub struct PairedHist {
    pub changed: Vec<Hist>,
    pub old: Vec<Hist>,
    pub metric: Metric,
    pub width: f64,
    pub bins: u32,
    /// The changed engine's upper bound and noise floor against the real sample, when it
    /// has one.
    pub upper: Option<f64>,
    pub floor: Option<f64>,
}

impl PairedHist {
    /// The changed engine's profile minus the old engine's, each pair counted `weights[i]`
    /// times (every pair once without weights); `None` when either holds nothing.
    pub fn diff(&self, weights: Option<&[u32]>) -> Option<Vec<f64>> {
        let n = self.bins as usize;
        let (mut c, mut o) = (vec![0.0; n], vec![0.0; n]);
        for (i, (hc, ho)) in self.changed.iter().zip(&self.old).enumerate() {
            let w = weights.map_or(1, |w| w[i]);
            if w > 0 {
                add(&mut c, hc, f64::from(w));
                add(&mut o, ho, f64::from(w));
            }
        }
        let (pc, po) = (profile(self.metric, &c)?, profile(self.metric, &o)?);
        Some(pc.iter().zip(&po).map(|(a, b)| a - b).collect())
    }

    fn scale(&self) -> f64 {
        scale(self.metric, self.width)
    }

    /// The distance between the two engines' pooled samples, every pair once.
    pub fn observed(&self) -> Option<f64> {
        let d = self.diff(None)?;
        Some(self.scale() * d.iter().map(|x| x.abs()).sum::<f64>())
    }

    /// The centred replicate under `weights`: the distance of the resampled difference from
    /// the observed one. It is 0 with every weight 1, and it draws no random number.
    pub fn centred(&self, weights: &[u32], observed_diff: &[f64]) -> Option<f64> {
        let d = self.diff(Some(weights))?;
        Some(
            self.scale()
                * d.iter()
                    .zip(observed_diff)
                    .map(|(x, o)| (x - o).abs())
                    .sum::<f64>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sample of `n` matches of `per` values each from `draw`, binned by 1 over `bins`.
    fn sample(
        rng: &mut EngineRng,
        n: usize,
        per: usize,
        bins: u32,
        draw: impl Fn(&mut EngineRng) -> f64,
    ) -> Sample {
        let per_match = (0..n)
            .map(|_| {
                let mut h: std::collections::BTreeMap<u32, f64> = Default::default();
                for _ in 0..per {
                    *h.entry(bin_of(draw(rng), 1.0, bins)).or_default() += 1.0;
                }
                h.into_iter().collect()
            })
            .collect();
        Sample {
            per_match,
            bins,
            width: 1.0,
        }
    }

    /// A count from 0 to 15: the sum of five draws from 0 to 3.
    fn count(rng: &mut EngineRng) -> f64 {
        (0..5).map(|_| rng.range_usize(4) as f64).sum()
    }

    #[test]
    fn hand_cases_of_the_two_distances() {
        // Equal samples: 0.
        assert_eq!(
            wasserstein1(&[(1.0, 1.0), (2.0, 1.0)], &[(2.0, 3.0), (1.0, 3.0)]),
            0.0
        );
        // A point mass moved by 2.5: 2.5.
        assert_eq!(wasserstein1(&[(1.0, 1.0)], &[(3.5, 4.0)]), 2.5);
        // Half the mass moved by 2: 1.
        assert_eq!(wasserstein1(&[(0.0, 1.0), (2.0, 1.0)], &[(2.0, 1.0)]), 1.0);
        assert!(wasserstein1(&[], &[(1.0, 1.0)]).is_nan());
        // Disjoint cells: 1; the same shares at another scale: 0.
        assert_eq!(total_variation(&[1.0, 0.0], &[0.0, 5.0]), 1.0);
        assert_eq!(total_variation(&[1.0, 3.0], &[2.0, 6.0]), 0.0);
        assert_eq!(total_variation(&[1.0, 1.0, 0.0], &[0.0, 1.0, 1.0]), 0.5);
        // The binned form equals the point form on the bin points.
        let (a, b) = ([3.0, 0.0, 1.0, 4.0], [1.0, 2.0, 2.0, 0.0]);
        let points = |h: &[f64]| -> Vec<(f64, f64)> {
            h.iter()
                .enumerate()
                .map(|(k, w)| (k as f64 * 2.0, *w))
                .collect()
        };
        let direct = wasserstein1(&points(&a), &points(&b));
        assert!((w1_bins(&a, &b, 2.0) - direct).abs() < 1e-12, "{direct}");
        assert_eq!(bin_of(7.0, 2.0, 31), 3);
        assert_eq!(bin_of(99.0, 2.0, 31), 30);
        assert_eq!(bin_of(-1.0, 1.0, 5), 0);
        assert_eq!(bin_of(4.0, 1.0, 5), 4);
    }

    /// The word on synthetic samples: a pass, a clear fail, and not sure near the margin.
    #[test]
    fn the_equivalence_test_passes_the_same_distribution_and_fails_a_shift() {
        let mut rng = EngineRng::from_seed(20);
        let bins = 20;
        let a = sample(&mut rng, 2000, 3, bins, count);
        let b = sample(&mut rng, 2000, 3, bins, count);
        let probe = judge_to_real(&a, &b, Metric::W1, 1.0, 1, 999).unwrap();
        // (a) Two samples from one distribution, a margin at least five times the floor:
        // pass. The margin is 1, so a shift of two counts is twice it.
        let margin = 1.0;
        assert!(margin >= 5.0 * probe.floor, "{probe:?}");
        let same = judge_to_real(&a, &b, Metric::W1, margin, 1, 999).unwrap();
        assert_eq!(same.word, Word::Pass, "{same:?}");
        // (b) A sample shifted by twice the margin: fail.
        let shifted = sample(&mut rng, 2000, 3, bins, |r| count(r) + 2.0 * margin);
        let fail = judge_to_real(&shifted, &b, Metric::W1, margin, 1, 999).unwrap();
        assert!((fail.distance - 2.0 * margin).abs() < 0.1, "{fail:?}");
        assert_eq!(fail.word, Word::Fail, "{fail:?}");
        // (c) A shift of the margin itself with 40 matches a side: not sure.
        let few_a = sample(&mut rng, 40, 3, bins, |r| count(r) + 1.0);
        let few_b = sample(&mut rng, 40, 3, bins, count);
        let near = judge_to_real(&few_a, &few_b, Metric::W1, 1.0, 1, 999).unwrap();
        assert_eq!(near.word, Word::NotSure, "{near:?}");
        assert!(near.floor < 1.0, "the margin is above the floor: {near:?}");
        // (d) A margin under the floor: not sure, by name.
        let under = judge_to_real(&a, &b, Metric::W1, probe.floor / 2.0, 1, 999).unwrap();
        assert_eq!(under.word, Word::NotSure);
        assert!(
            under.note.starts_with("margin below noise floor"),
            "{under:?}"
        );
        // The same stream gives the same check.
        assert_eq!(
            same,
            judge_to_real(&a, &b, Metric::W1, margin, 1, 999).unwrap()
        );
        // Too few matches: no value.
        let one = Sample {
            per_match: vec![vec![(1, 1.0)]],
            ..b.clone()
        };
        assert!(judge_to_real(&one, &b, Metric::W1, 1.0, 1, 99).is_err());
    }

    /// The same three cases by total variation on a 6 by 6 table of cells.
    #[test]
    fn the_equivalence_test_on_cells_by_total_variation() {
        let mut rng = EngineRng::from_seed(21);
        let cell = |r: &mut EngineRng| {
            let side = |r: &mut EngineRng| ((0..3).map(|_| r.range_usize(3)).sum::<usize>()).min(5);
            (side(r) * 6 + side(r)) as f64
        };
        // Moves about 80 percent (or 40 percent) of the mass to the cell 5-5.
        let moved = |r: &mut EngineRng| if r.chance(0.8) { 35.0 } else { cell(r) };
        let nudged = |r: &mut EngineRng| if r.chance(0.4) { 35.0 } else { cell(r) };
        let a = sample(&mut rng, 3000, 1, 36, cell);
        let b = sample(&mut rng, 3000, 1, 36, cell);
        let probe = judge_to_real(&a, &b, Metric::Tv, 1.0, 2, 999).unwrap();
        let margin = 0.4;
        assert!(margin >= 5.0 * probe.floor, "{probe:?}");
        assert_eq!(
            judge_to_real(&a, &b, Metric::Tv, margin, 2, 999)
                .unwrap()
                .word,
            Word::Pass
        );
        let far = sample(&mut rng, 3000, 1, 36, moved);
        let fail = judge_to_real(&far, &b, Metric::Tv, margin, 2, 999).unwrap();
        assert!((fail.distance - 2.0 * margin).abs() < 0.1, "{fail:?}");
        assert_eq!(fail.word, Word::Fail, "{fail:?}");
        // A shift near the margin with few matches: not sure.
        let few_a = sample(&mut rng, 150, 1, 36, nudged);
        let few_b = sample(&mut rng, 150, 1, 36, cell);
        // The margin sits between the bounds of the shift, which is about 0.4 plus the
        // noise of 150 matches over 36 cells.
        let bounds = judge_to_real(&few_a, &few_b, Metric::Tv, 1.0, 2, 999).unwrap();
        let margin = (bounds.lower + bounds.upper) / 2.0;
        let near = judge_to_real(&few_a, &few_b, Metric::Tv, margin, 2, 999).unwrap();
        assert_eq!(near.word, Word::NotSure, "{near:?}");
        assert!(
            near.floor < margin,
            "the margin is above the floor: {near:?}"
        );
        assert!((near.distance - margin).abs() < 0.1, "{near:?}");
        let under = judge_to_real(&a, &b, Metric::Tv, probe.floor / 2.0, 2, 999).unwrap();
        assert!(
            under.note.starts_with("margin below noise floor"),
            "{under:?}"
        );
    }

    /// The in-house distances agree with scipy's within 1e-9 on the fixture's
    /// synthetic cases. The fixture is read at run time, so no test runs Python.
    #[test]
    fn the_distances_agree_with_scipy_within_1e_9() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/distribution/scipy.json");
        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let list = |v: &serde_json::Value| -> Vec<f64> {
            v.as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect()
        };
        let close = |name: &str, ours: f64, theirs: &serde_json::Value| {
            let theirs = theirs.as_f64().unwrap();
            assert!(
                (ours - theirs).abs() <= 1e-9,
                "{name}: ours {ours}, scipy {theirs}"
            );
        };
        let mut checked = 0;
        for c in doc["points"].as_array().unwrap() {
            let (u, v) = (list(&c["u"]), list(&c["v"]));
            let weights = |key: &str, n: usize| c.get(key).map_or_else(|| vec![1.0; n], list);
            let (uw, vw) = (weights("u_weights", u.len()), weights("v_weights", v.len()));
            let pair = |x: &[f64], w: &[f64]| -> Vec<(f64, f64)> {
                x.iter().copied().zip(w.iter().copied()).collect()
            };
            let name = c["name"].as_str().unwrap();
            close(
                name,
                wasserstein1(&pair(&u, &uw), &pair(&v, &vw)),
                &c["wasserstein"],
            );
            checked += 1;
        }
        for c in doc["bins"].as_array().unwrap() {
            let (a, b) = (list(&c["a"]), list(&c["b"]));
            let width = c["width"].as_f64().unwrap();
            let name = c["name"].as_str().unwrap();
            // The binned form the run reads, and the point form on the bin points.
            close(name, w1_bins(&a, &b, width), &c["wasserstein"]);
            let at = |h: &[f64]| -> Vec<(f64, f64)> {
                h.iter()
                    .enumerate()
                    .map(|(k, w)| (k as f64 * width, *w))
                    .collect()
            };
            close(name, wasserstein1(&at(&a), &at(&b)), &c["wasserstein"]);
            checked += 1;
        }
        for c in doc["cells"].as_array().unwrap() {
            let name = c["name"].as_str().unwrap();
            close(
                name,
                total_variation(&list(&c["a"]), &list(&c["b"])),
                &c["total_variation"],
            );
            checked += 1;
        }
        assert_eq!(checked, 12, "every case of the fixture");
    }

    #[test]
    fn the_centred_replicate_is_zero_with_every_weight_one() {
        let p = PairedHist {
            changed: vec![vec![(0, 1.0), (2, 1.0)], vec![(1, 3.0)], vec![]],
            old: vec![vec![(2, 2.0)], vec![(0, 1.0)], vec![(1, 1.0)]],
            metric: Metric::W1,
            width: 1.0,
            bins: 3,
            upper: None,
            floor: None,
        };
        let d = p.diff(None).unwrap();
        assert_eq!(p.centred(&[1, 1, 1], &d), Some(0.0));
        assert!(p.centred(&[3, 0, 0], &d).unwrap() > 0.0);
        // Pooled: changed 1, 3, 1 at 0, 1, 2; old 1, 1, 2.
        let direct = wasserstein1(
            &[(0.0, 1.0), (1.0, 3.0), (2.0, 1.0)],
            &[(0.0, 1.0), (1.0, 1.0), (2.0, 2.0)],
        );
        assert!((p.observed().unwrap() - direct).abs() < 1e-12);
        assert_ne!(stream(1, "a"), stream(1, "b"));
    }
}
