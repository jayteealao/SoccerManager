//! The engine spread of each band, read from a finished run folder's stored rows: the
//! per-match standard deviation, the spread between the run's generated leagues and, for a
//! whole-distribution band, its noise floor. The figures set a band's `engine_spread`.
//!
//! A hidden probe, run by hand on a release run folder:
//! `SM_SPREAD_DIR=<run folder> SM_SPREAD_SUITE=equal SM_SPREAD_SEED=9 SM_SPREAD_MATCHES=1000
//! cargo test -p engine-cli --features measures --release spread -- --ignored --nocapture`.
//! It prints one JSON line per band and judges nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::fixtures::{self, Keyed};
use super::rows;
use super::run_folder;
use crate::measure_rows::SHAPE_FIELDS;
use crate::report::bands::Registry;
use crate::report::measures::Extras;
use crate::report::tracked_shape::TRACKED_FIELDS;
use crate::report::{BandCheck, RunBuilder, Suite, real};
use engine::ContentDir;

/// One band's spread in one suite.
#[derive(Debug, Serialize)]
struct Spread {
    band: String,
    suite: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pairing: Option<String>,
    matches: usize,
    leagues: usize,
    value: f64,
    /// The band's standard error over the run times the square root of its matches (of one
    /// arm, for a red-card band).
    per_match_sd: f64,
    /// The spread of the league values beyond what their own errors explain, 0 when none.
    between_league_sd: f64,
    /// The league values themselves.
    league_values: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    noise_floor: Option<f64>,
}

/// The builder of the rows of `keyed` found in `dir`, kept to the leagues `keep` allows.
fn fold(
    registry: &Registry,
    suite: Suite,
    keyed: &[Keyed],
    read: &rows::ReadRows,
    seed: u64,
    samples: &std::sync::Arc<real::Samples>,
    keep: impl Fn(u32) -> bool,
) -> (RunBuilder, usize) {
    let mut builder = RunBuilder::new(registry.clone());
    builder.set_real_samples(samples.clone(), seed);
    let planned = keyed.iter().filter(|k| keep(k.fixture.league)).count();
    let planned = u32::try_from(planned).unwrap_or(u32::MAX);
    builder.plan(suite, planned);
    let mut folded = 0;
    for k in keyed.iter().filter(|k| keep(k.fixture.league)) {
        let Some(row) = read.rows.get(&k.key).filter(|r| r.seed == k.engine_seed) else {
            builder.add_missing();
            continue;
        };
        let stats = row.band_record();
        match k.arm {
            Some(arm) => builder.add_red_card(stats, arm),
            None => builder.add(suite, stats, k.boosted),
        };
        builder.set_league(suite, k.fixture.league);
        let club = |c: usize| (k.fixture.league, u32::try_from(c).unwrap_or(u32::MAX));
        let measured = row.measured.as_deref();
        builder.set_extras(
            suite,
            Extras {
                measures: measured.map(|m| m.measures).unwrap_or_default(),
                physical: measured.map(|m| m.physical).unwrap_or_default(),
                shape: measured.map_or([[0.0; SHAPE_FIELDS]; 2], |m| m.shape),
                tracked: measured.map_or([[0.0; TRACKED_FIELDS]; 2], |m| m.tracked),
                clubs: k.fixture.clubs.map(club),
            },
        );
        folded += 1;
    }
    (builder, folded)
}

/// Every band check of `builder`, keyed by band and pairing.
fn checks(builder: &RunBuilder) -> BTreeMap<(String, Option<String>), BandCheck> {
    builder
        .row_specs(true)
        .iter()
        .map(|spec| {
            let c = builder.check(spec);
            ((c.band.clone(), c.pairing.clone()), c)
        })
        .collect()
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

#[test]
#[ignore = "a probe run by hand on a release run folder"]
fn spread() {
    let Some(dir) = env("SM_SPREAD_DIR").map(PathBuf::from) else {
        return;
    };
    let suite = match env("SM_SPREAD_SUITE").as_deref().unwrap_or("equal") {
        "strength" => Suite::Strength,
        "red-card" => Suite::RedCard,
        _ => Suite::Equal,
    };
    let seed: u64 = env("SM_SPREAD_SEED").map_or(9, |s| s.parse().unwrap());
    let matches: u32 = env("SM_SPREAD_MATCHES").map_or(1000, |s| s.parse().unwrap());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let registry = Registry::load(&ContentDir::at(&root)).unwrap();
    let folder = real::resolve(None, &root);
    let samples = std::sync::Arc::new(real::samples(&folder, &registry));
    let keyed = fixtures::keyed(suite, seed, matches, &[], &[]);
    let done = run_folder::finished(&dir, None).unwrap();
    let read = rows::read_rows(&dir, &done).unwrap();
    let leagues: Vec<u32> = {
        let mut l: Vec<u32> = keyed.iter().map(|k| k.fixture.league).collect();
        l.dedup();
        l
    };
    let (whole, folded) = fold(&registry, suite, &keyed, &read, seed, &samples, |_| true);
    let per_league: Vec<_> = leagues
        .iter()
        .map(|&l| checks(&fold(&registry, suite, &keyed, &read, seed, &samples, |x| x == l).0))
        .collect();
    for ((band, pairing), c) in checks(&whole) {
        if band == "wall_ms" {
            continue;
        }
        let values: Vec<(f64, f64)> = per_league
            .iter()
            .filter_map(|m| m.get(&(band.clone(), pairing.clone())))
            .filter(|x| x.value.is_finite())
            .map(|x| (x.value, x.se))
            .collect();
        let g = values.len() as f64;
        let between = if values.len() > 1 {
            let mean = values.iter().map(|v| v.0).sum::<f64>() / g;
            let var = values.iter().map(|v| (v.0 - mean).powi(2)).sum::<f64>() / (g - 1.0);
            let own = values.iter().map(|v| v.1 * v.1).sum::<f64>() / g;
            (var - own).max(0.0).sqrt()
        } else {
            0.0
        };
        // A red-card band compares two arms of one match count each.
        let per_band = match suite {
            Suite::RedCard => folded / crate::report::RED_CARD_ARMS.len(),
            _ => folded,
        };
        let spread = Spread {
            band,
            suite: c.suite.clone(),
            pairing,
            matches: folded,
            leagues: values.len(),
            value: c.value,
            per_match_sd: c.se * (per_band as f64).sqrt(),
            between_league_sd: between,
            league_values: values.iter().map(|v| v.0).collect(),
            noise_floor: c.dist.as_ref().map(|d| d.floor),
        };
        println!("SPREAD {}", serde_json::to_string(&spread).unwrap());
    }
}
