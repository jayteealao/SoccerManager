//! The agreement of paired matches: how closely the changed engine's matches follow the old
//! engine's on the same fixtures, band by band, and over the run.
//!
//! A band's agreement is the correlation, over the pairs both engines finished, of each
//! match's part in the band's value (its influence on the value's ratio: the same terms the
//! paired error sums), so a band that is a ratio of sums correlates as its paired error
//! sees it. The run's agreement is the median over the gating scalar bands, with a
//! percentile interval from resampling the bands ([`pass_rule::agreement::summary`]). The
//! head measure's lower bounds size the old-engine count; the agreement never decides a run.
//!
//! `engine-cli calibrate agreement --a <DIR> --b <DIR>` measures two run folders of the same
//! seed from their per-match statistics files (`stats/<match.id>.json`, kept with
//! `--keep-events all`), joined on match seed, on the figures those files carry at every
//! revision since the stream split.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context;
use engine::rng::EngineRng;
use serde::Serialize;
use serde_json::Value;

use super::kinds::BandAgreement;
use super::verdict::PairedRow;

/// The resamples of the bands behind the run's interval.
pub const REPLICATES: usize = 2000;
/// The label of the agreement's random stream, apart from the bootstrap's.
const STREAM_LABEL: u64 = 0x6167_7265_656d_656e;

/// `calib.pair_agreement`: the run's agreement and each band's.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunAgreement {
    /// The median of the bands' correlations, with its 90 percent interval.
    pub median: f64,
    pub lo: f64,
    pub hi: f64,
    /// The most pairs any band was measured over.
    pub pairs: u32,
    pub bands: BTreeMap<String, BandAgreement>,
    /// The bands with no agreement, and why.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub without: BTreeMap<String, String>,
}

/// Each pair's part in a scalar row's value, (changed, old); empty for a distribution row
/// or a side whose denominators sum to 0.
pub fn influence_pairs(r: &PairedRow) -> Vec<(f64, f64)> {
    if r.dist.is_some() {
        return Vec::new();
    }
    let part = |t: &[Vec<f64>; 4]| -> Option<Vec<f64>> {
        let s: Vec<f64> = t.iter().map(|v| v.iter().sum()).collect();
        if s[1] <= 0.0 || s[2] <= 0.0 {
            return None;
        }
        // A difference band's numerator sum can be negative: only a zero sum has no term,
        // as in the paired error.
        let term = |k: usize, i: usize| if s[k] != 0.0 { t[k][i] / s[k] } else { 0.0 };
        Some(
            (0..t[0].len())
                .map(|i| term(0, i) - term(1, i) + term(3, i) - term(2, i))
                .collect(),
        )
    };
    match (part(&r.changed), part(&r.old)) {
        (Some(c), Some(o)) => c.into_iter().zip(o).collect(),
        _ => Vec::new(),
    }
}

/// The agreement of a run's gating scalar rows. A band judged in several suites keeps the
/// row of the most pairs; a row of one formation pairing takes no part.
pub fn of_rows(rows: &[PairedRow], seed: u64) -> Option<RunAgreement> {
    let mut per_band: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::new();
    for r in rows
        .iter()
        .filter(|r| r.dist.is_none() && r.pairing.is_none())
    {
        let pairs = influence_pairs(r);
        let keep = per_band
            .get(&r.band)
            .is_none_or(|have| pairs.len() > have.len());
        if keep {
            per_band.insert(r.band.clone(), pairs);
        }
    }
    summarise(per_band, seed)
}

/// The agreement of bands from their pairs, with the run's median and interval; `None` when
/// no band has one.
pub fn summarise(per_band: BTreeMap<String, Vec<(f64, f64)>>, seed: u64) -> Option<RunAgreement> {
    let mut bands = BTreeMap::new();
    let mut without = BTreeMap::new();
    for (band, pairs) in per_band {
        match pass_rule::agreement::pair_agreement(&pairs) {
            Ok(a) => {
                bands.insert(
                    band,
                    BandAgreement {
                        r: round(a.r),
                        lo: round(a.lo),
                        hi: round(a.hi),
                        pairs: u32::try_from(a.n).unwrap_or(u32::MAX),
                    },
                );
            }
            Err(why) => {
                without.insert(band, why);
            }
        }
    }
    let rs: Vec<f64> = bands.values().map(|b| b.r).collect();
    let mut rng = EngineRng::from_seed(seed ^ STREAM_LABEL);
    let s = pass_rule::agreement::summary(&rs, REPLICATES, &mut || rng.next_f64()).ok()?;
    Some(RunAgreement {
        median: round(s.r),
        lo: round(s.lo),
        hi: round(s.hi),
        pairs: bands.values().map(|b| b.pairs).max().unwrap_or(0),
        bands,
        without,
    })
}

fn round(v: f64) -> f64 {
    engine::observe::round_to(v, 4)
}

/// The figures a statistics file carries at every revision since the stream split, each
/// as the per-match value of one band: the two teams' total for goals, the mean of the two
/// teams for the rest.
const STATS_FIGURES: [(&str, &str, bool); 7] = [
    ("stats.goals", "goals_per_match", true),
    ("stats.shots", "shots_per_team", false),
    ("stats.shots_on_target", "shots_on_target_per_team", false),
    ("stats.fouls", "fouls_per_team", false),
    ("stats.corners", "corners_per_team", false),
    ("stats.passes", "passes_per_team", false),
    ("stats.xg", "xg_per_team", false),
];

/// Each successful match's figures in a run folder's `stats/` folder, by match seed.
fn stats_by_seed(dir: &Path) -> anyhow::Result<BTreeMap<u64, BTreeMap<&'static str, f64>>> {
    let stats = dir.join("stats");
    let entries = std::fs::read_dir(&stats).with_context(|| {
        format!(
            "{} has no stats folder: run it with --keep-events all",
            dir.display()
        )
    })?;
    let mut out = BTreeMap::new();
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let record: Value = serde_json::from_str(text.trim())
            .with_context(|| format!("{} is not a statistics record", path.display()))?;
        if record["outcome"] != "success" {
            continue;
        }
        let Some(seed) = record["seed"].as_u64() else {
            continue;
        };
        let mut figures = BTreeMap::new();
        for (key, band, total) in STATS_FIGURES {
            let pair: Option<Vec<f64>> = record[key]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_f64).collect());
            if let Some(p) = pair.filter(|p| p.len() == 2) {
                let sum = p[0] + p[1];
                figures.insert(band, if total { sum } else { sum / 2.0 });
            }
        }
        out.insert(seed, figures);
    }
    Ok(out)
}

/// The agreement of two run folders of the same seed, from their statistics files.
pub fn of_folders(a: &Path, b: &Path, seed: u64) -> anyhow::Result<RunAgreement> {
    let (sa, sb) = (stats_by_seed(a)?, stats_by_seed(b)?);
    let mut per_band: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::new();
    for (s, fa) in &sa {
        let Some(fb) = sb.get(s) else { continue };
        for (band, va) in fa {
            if let Some(vb) = fb.get(band) {
                per_band
                    .entry((*band).to_string())
                    .or_default()
                    .push((*va, *vb));
            }
        }
    }
    if per_band.is_empty() {
        anyhow::bail!(
            "{} and {} share no successful match seed: run both with the same --seed",
            a.display(),
            b.display()
        );
    }
    summarise(per_band, seed).context("no band has four pairs or more")
}

/// The console lines of an agreement.
pub fn render(a: &RunAgreement) -> String {
    let mut out = format!(
        "pair agreement: median {} (90 percent interval {} to {}) over {} band(s), up to {} \
         pairs\n",
        a.median,
        a.lo,
        a.hi,
        a.bands.len(),
        a.pairs
    );
    for (band, b) in &a.bands {
        out.push_str(&format!(
            "  {band}: r {} ({} to {}), {} pairs\n",
            b.r, b.lo, b.hi, b.pairs
        ));
    }
    for (band, why) in &a.without {
        out.push_str(&format!("  {band}: no agreement: {why}\n"));
    }
    out
}

/// `engine-cli calibrate agreement`: measures the two folders, prints the agreement, and
/// writes it to `out` when given.
pub fn command(a: &Path, b: &Path, out: Option<&Path>) -> anyhow::Result<i32> {
    let seed = |dir: &Path| -> Option<u64> {
        let text = std::fs::read_to_string(dir.join("report.json")).ok()?;
        serde_json::from_str::<Value>(text.trim()).ok()?["seed"].as_u64()
    };
    let agreement = of_folders(a, b, seed(a).unwrap_or(0))?;
    eprint!("{}", render(&agreement));
    let line = serde_json::to_string(&agreement)?;
    if let Some(path) = out {
        std::fs::write(path, format!("{line}\n"))
            .with_context(|| format!("cannot write {}", path.display()))?;
    }
    println!("{line}");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Suite;

    /// A mean band of one figure per match: changed and old per-match values.
    fn row(band: &str, changed: &[f64], old: &[f64]) -> PairedRow {
        let ones = vec![1.0; changed.len()];
        PairedRow {
            band: band.into(),
            suite: Suite::Equal,
            pairing: None,
            lo: 0.0,
            hi: 10.0,
            shift: 0.1,
            changed: [changed.to_vec(), ones.clone(), ones.clone(), ones.clone()],
            old: [old.to_vec(), ones.clone(), ones.clone(), ones],
            value: None,
            base_value: None,
            outside: 0.0,
            set: 0,
            dist: None,
        }
    }

    #[test]
    fn a_mean_band_correlates_its_per_match_values() {
        let c = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let o = [2.0, 4.0, 6.0, 8.0, 10.0, 12.0];
        let pairs = influence_pairs(&row("goals_per_match", &c, &o));
        assert_eq!(pairs.len(), 6);
        let r = pass_rule::agreement::pearson(&pairs).unwrap();
        assert!((r - 1.0).abs() < 1e-12, "{r}");
    }

    #[test]
    fn a_band_whose_numerators_sum_below_zero_keeps_its_terms() {
        let c = [-1.0, -3.0, 0.0, -2.0, -4.0, -1.0];
        let o = [-2.0, -1.0, -1.0, -2.0, -2.0, 0.0];
        let pairs = influence_pairs(&row("home_goal_difference", &c, &o));
        assert_eq!(pairs.len(), 6);
        assert!(pairs.iter().any(|p| (p.0 - pairs[0].0).abs() > 1e-12));
        let r = pass_rule::agreement::pearson(&pairs).unwrap();
        let shift = |v: &[f64]| v.iter().map(|x| x + 10.0).collect::<Vec<_>>();
        let shifted = influence_pairs(&row("home_goal_difference", &shift(&c), &shift(&o)));
        let r10 = pass_rule::agreement::pearson(&shifted).unwrap();
        assert!((r - r10).abs() < 1e-9, "{r} against {r10}");
    }

    #[test]
    fn the_run_agreement_is_the_median_of_its_bands_and_the_same_for_the_same_seed() {
        let c = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0, 8.0, 7.0];
        let rows = vec![
            row("a", &c, &c),
            row("b", &c, &[2.0, 1.0, 3.0, 4.0, 6.0, 5.0, 7.0, 8.0]),
            row("c", &c, &[8.0, 1.0, 2.0, 7.0, 3.0, 6.0, 4.0, 5.0]),
            // One pairing's row takes no part.
            PairedRow {
                pairing: Some("4-4-2 v 4-3-3".into()),
                ..row("d", &c, &c)
            },
        ];
        let a = of_rows(&rows, 9).unwrap();
        assert_eq!(a.bands.len(), 3);
        assert_eq!(a.bands["a"].r, 1.0);
        let mut rs: Vec<f64> = a.bands.values().map(|b| b.r).collect();
        rs.sort_by(f64::total_cmp);
        assert_eq!(a.median, rs[1]);
        assert!(a.lo <= a.median && a.median <= a.hi, "{a:?}");
        assert_eq!(a.pairs, 8);
        assert_eq!(of_rows(&rows, 9), Some(a));
    }

    #[test]
    fn a_band_under_four_pairs_has_no_agreement_and_says_why() {
        let per_band = BTreeMap::from([
            ("few".to_string(), vec![(1.0, 1.0), (2.0, 2.0)]),
            (
                "many".to_string(),
                vec![(1.0, 1.0), (2.0, 2.5), (3.0, 2.0), (4.0, 4.0)],
            ),
        ]);
        let a = summarise(per_band, 1).unwrap();
        assert!(a.bands.contains_key("many"));
        assert!(a.without["few"].contains("needs 4"), "{:?}", a.without);
    }

    #[test]
    fn two_folders_are_joined_on_match_seed() {
        let dir = tempfile::TempDir::new().unwrap();
        let write = |run: &str, seed: u64, goals: [u32; 2], shots: [u32; 2]| {
            let stats = dir.path().join(run).join("stats");
            std::fs::create_dir_all(&stats).unwrap();
            let record = serde_json::json!({
                "seed": seed,
                "outcome": "success",
                "stats.goals": goals,
                "stats.shots": shots,
            });
            std::fs::write(stats.join(format!("m{seed}.json")), record.to_string()).unwrap();
        };
        for (i, s) in [11u64, 12, 13, 14, 15].iter().enumerate() {
            let i = i as u32;
            write("a", *s, [i, 1], [10 + i, 8]);
            write("b", *s, [i, 2], [9 + 2 * i, 8]);
        }
        // A seed only one folder played is left out.
        write("b", 99, [5, 5], [1, 1]);
        let a = of_folders(&dir.path().join("a"), &dir.path().join("b"), 3).unwrap();
        assert_eq!(a.bands["goals_per_match"].pairs, 5);
        assert_eq!(a.bands["goals_per_match"].r, 1.0);
        assert_eq!(a.bands["shots_per_team"].r, 1.0);
        assert!(of_folders(&dir.path().join("a"), &dir.path().join("none"), 3).is_err());
    }
}
