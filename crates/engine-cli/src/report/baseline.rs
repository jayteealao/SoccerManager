//! A calibration run measured against an earlier report: the guard that refuses a baseline
//! played on other fixtures, and the diff that prints each band's change with its sampling
//! error and marks a change inside two errors as noise.
//!
//! A baseline must share the run's fixture scheme, seed, match count, and fixtures hash. A
//! report from the old seeding scheme, whose match seeds came from the fixture's place in
//! the run, played other matches, so it is refused first, by name. A different content hash
//! (tuning values or flag states) is the change under test, so it is allowed and named in
//! the diff's header.

use std::path::Path;

use engine::observe::round_to;
use serde::Serialize;
use serde_json::Value;

use super::BandCheck;
use super::compare;
use crate::calibrate::fixtures::FIXTURE_SCHEME;

/// A change is noise when it is at most this many sampling errors.
pub const NOISE_ERRORS: f64 = 2.0;

/// The report a run was compared with.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BaselineInfo {
    pub path: String,
    #[serde(rename = "run.id")]
    pub run_id: String,
    pub seed: u64,
    #[serde(rename = "content.hash")]
    pub content_hash: String,
}

/// A baseline report that passed the guard.
#[derive(Debug, Clone)]
pub struct Baseline {
    pub info: BaselineInfo,
    pub bands: Vec<BandCheck>,
}

/// What a run must share with its baseline.
#[derive(Debug, Clone, Copy)]
pub struct Identity<'a> {
    pub seed: u64,
    pub matches: u32,
    pub fixtures_hash: &'a str,
}

/// Reads the report at `path` and refuses it, naming every difference, when it is not a
/// calibrate run report, was made by another fixture scheme, was made before sampling
/// errors or fixtures hashes, or differs from `run` in seed, match count, or fixtures hash.
pub fn load(path: &Path, run: Identity<'_>) -> anyhow::Result<Baseline> {
    let shown = path.display();
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read the baseline {shown}: {e}"))?;
    let report: Value = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("the baseline {shown} is not JSON: {e}"))?;
    check(&report, run).map_err(|why| anyhow::anyhow!("the baseline {shown} is refused: {why}"))?;
    let bands: Vec<BandCheck> =
        serde_json::from_value(report["calib.bands"].clone()).map_err(|e| {
            anyhow::anyhow!("the baseline {shown} has a band row that cannot be read: {e}")
        })?;
    let text_of = |key: &str| report[key].as_str().unwrap_or_default().to_string();
    Ok(Baseline {
        info: BaselineInfo {
            path: shown.to_string(),
            run_id: text_of("run.id"),
            seed: run.seed,
            content_hash: text_of("content.hash"),
        },
        bands,
    })
}

/// The reason `report` cannot be a baseline for `run`, or `Ok`.
fn check(report: &Value, run: Identity<'_>) -> Result<(), String> {
    let calibrate = report["record.kind"] == "run-report" && report["operation"] == "calibrate";
    let Some(bands) = report["calib.bands"].as_array().filter(|_| calibrate) else {
        return Err("it is not the run report of a calibrate run".into());
    };
    match report["fixtures.scheme"].as_str() {
        Some(scheme) if scheme == FIXTURE_SCHEME => {}
        Some(scheme) => {
            return Err(format!(
                "it was made with the fixture scheme {scheme}; this run uses fixture keys \
                 ({FIXTURE_SCHEME}), so the two runs played different matches; make a new \
                 baseline"
            ));
        }
        None => {
            return Err(format!(
                "it was made with the old seeding scheme (match seeds from the fixture's \
                 place in the run); this run uses fixture keys ({FIXTURE_SCHEME}), so the \
                 two runs played different matches; make a new baseline"
            ));
        }
    }
    if bands.iter().any(|b| b.get("se").is_none()) {
        return Err("the baseline was made before sampling errors; make a new one".into());
    }
    let Some(fixtures) = report["fixtures.hash"].as_str() else {
        return Err("the baseline was made before fixtures hashes; make a new one".into());
    };
    let mut differences = Vec::new();
    match report["seed"].as_u64() {
        Some(seed) if seed == run.seed => {}
        seed => differences.push(format!(
            "seed {} differs from the baseline's {}",
            run.seed,
            shown(seed)
        )),
    }
    match report["calib.matches"].as_u64() {
        Some(m) if m == u64::from(run.matches) => {}
        m => differences.push(format!(
            "match count {} differs from the baseline's {}",
            run.matches,
            shown(m)
        )),
    }
    if fixtures != run.fixtures_hash {
        differences.push(format!(
            "fixtures hash {} differs from the baseline's {fixtures}",
            run.fixtures_hash
        ));
    }
    if differences.is_empty() {
        Ok(())
    } else {
        Err(differences.join("; "))
    }
}

fn shown(v: Option<u64>) -> String {
    v.map_or_else(|| "none".into(), |v| v.to_string())
}

/// One band both runs judged.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiffRow {
    pub suite: String,
    pub band: String,
    /// The formation pairing, or the red-card arm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairing: Option<String>,
    pub baseline: f64,
    pub new: f64,
    /// `new` minus `baseline`.
    pub change: f64,
    /// The sampling error of the change: sqrt(se_baseline^2 + se_new^2).
    pub error: f64,
    /// The change is at most [`NOISE_ERRORS`] errors.
    pub noise: bool,
    /// The new run's verdict on the band.
    pub pass: bool,
}

/// The diff of a run against its baseline.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diff {
    /// The baseline's content hash, then the run's.
    pub content_hashes: [String; 2],
    pub content_changed: bool,
    pub rows: Vec<DiffRow>,
    /// Rows marked noise, and rows marked change.
    pub noise: u32,
    pub changes: u32,
}

/// The rows of every band, pairing, and arm both runs judged, the time budget left out.
/// The two runs are treated as independent, so the error is conservative on shared
/// fixtures.
pub fn diff(baseline: &Baseline, content_hash: &str, new: &[BandCheck]) -> Diff {
    let se = |checks: &[BandCheck], r: &compare::CompareRow| {
        checks
            .iter()
            .find(|c| c.suite == r.suite && c.band == r.band && c.pairing == r.pairing)
            .map_or(0.0, |c| c.se)
    };
    let rows: Vec<DiffRow> = compare::compare(&baseline.bands, new)
        .into_iter()
        .map(|r| {
            let error = (se(&baseline.bands, &r).powi(2) + se(new, &r).powi(2)).sqrt();
            DiffRow {
                noise: r.delta.abs() <= NOISE_ERRORS * error,
                error: round_to(error, 5),
                suite: r.suite,
                band: r.band,
                pairing: r.pairing,
                baseline: r.off,
                new: r.on,
                change: r.delta,
                pass: r.on_pass,
            }
        })
        .collect();
    let noise = rows.iter().filter(|r| r.noise).count() as u32;
    Diff {
        content_hashes: [baseline.info.content_hash.clone(), content_hash.to_string()],
        content_changed: baseline.info.content_hash != content_hash,
        changes: rows.len() as u32 - noise,
        noise,
        rows,
    }
}

/// A fixed-width table of the diff, for a person reading the terminal.
pub fn render_table(info: &BaselineInfo, diff: &Diff) -> String {
    let [was, now] = &diff.content_hashes;
    let mut out = format!(
        "diff against baseline {} (seed {}): content {was} -> {now}, {}\n",
        info.run_id,
        info.seed,
        if diff.content_changed {
            "content changed"
        } else {
            "content unchanged"
        }
    );
    out.push_str(&format!(
        "{:<10} {:<23} {:<16} {:>9} {:>9} {:>9} {:>8}  {:<6} {}\n",
        "suite", "band", "pairing", "baseline", "new", "change", "error", "mark", "verdict"
    ));
    for r in &diff.rows {
        out.push_str(&format!(
            "{:<10} {:<23} {:<16} {:>9.4} {:>9.4} {:>+9.4} {:>8.4}  {:<6} {}\n",
            r.suite,
            r.band,
            r.pairing.as_deref().unwrap_or("-"),
            r.baseline,
            r.new,
            r.change,
            r.error,
            if r.noise { "noise" } else { "change" },
            if r.pass { "pass" } else { "miss" }
        ));
    }
    out.push_str(&format!(
        "{} rows: {} change, {} noise (a change inside {NOISE_ERRORS} errors is noise)\n",
        diff.rows.len(),
        diff.changes,
        diff.noise
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(band: &str, value: f64, se: f64) -> BandCheck {
        BandCheck {
            band: band.into(),
            suite: "equal".into(),
            pairing: None,
            value,
            lo: 0.0,
            hi: 10.0,
            pass: true,
            se,
        }
    }

    fn report(seed: u64, matches: u32, fixtures: &str) -> Value {
        json!({
            "record.kind": "run-report",
            "operation": "calibrate",
            "run.id": "calib-1",
            "seed": seed,
            "content.hash": "aaaaaaaaaaaa",
            "fixtures.hash": fixtures,
            "fixtures.scheme": FIXTURE_SCHEME,
            "calib.matches": matches,
            "calib.bands": [row("goals_per_match", 2.8, 0.05)],
        })
    }

    const RUN: Identity<'static> = Identity {
        seed: 42,
        matches: 1000,
        fixtures_hash: "0c44e1a9f2b3",
    };

    #[test]
    fn the_same_seed_matches_and_fixtures_pass_the_guard() {
        assert_eq!(check(&report(42, 1000, "0c44e1a9f2b3"), RUN), Ok(()));
    }

    #[test]
    fn every_difference_is_named() {
        let why = check(&report(7, 500, "3f2a9c01b7de"), RUN).unwrap_err();
        assert_eq!(
            why,
            "seed 42 differs from the baseline's 7; match count 1000 differs from the \
             baseline's 500; fixtures hash 0c44e1a9f2b3 differs from the baseline's \
             3f2a9c01b7de"
        );
    }

    #[test]
    fn an_old_or_foreign_report_is_refused() {
        let mut old = report(42, 1000, "0c44e1a9f2b3");
        old["calib.bands"][0].as_object_mut().unwrap().remove("se");
        assert!(
            check(&old, RUN)
                .unwrap_err()
                .contains("before sampling errors")
        );
        let mut old = report(42, 1000, "0c44e1a9f2b3");
        old.as_object_mut().unwrap().remove("fixtures.hash");
        assert!(
            check(&old, RUN)
                .unwrap_err()
                .contains("before fixtures hashes")
        );
        let mut bench = report(42, 1000, "0c44e1a9f2b3");
        bench["operation"] = json!("benchmark");
        assert!(
            check(&bench, RUN)
                .unwrap_err()
                .contains("not the run report")
        );
    }

    #[test]
    fn a_report_of_the_old_seeding_scheme_or_another_scheme_is_refused_by_name() {
        let mut old = report(42, 1000, "0c44e1a9f2b3");
        old.as_object_mut().unwrap().remove("fixtures.scheme");
        // The other differences are not reached: the scheme alone is named.
        old["seed"] = json!(7);
        let why = check(&old, RUN).unwrap_err();
        assert!(why.contains("old seeding scheme"), "{why}");
        assert!(why.contains(FIXTURE_SCHEME), "{why}");
        assert!(!why.contains("seed 42"), "{why}");
        let mut other = report(42, 1000, "0c44e1a9f2b3");
        other["fixtures.scheme"] = json!("fixture-key-0");
        let why = check(&other, RUN).unwrap_err();
        assert!(why.contains("fixture scheme fixture-key-0"), "{why}");
    }

    fn baseline(bands: Vec<BandCheck>) -> Baseline {
        Baseline {
            info: BaselineInfo {
                path: "report.json".into(),
                run_id: "calib-1".into(),
                seed: 42,
                content_hash: "aaaaaaaaaaaa".into(),
            },
            bands,
        }
    }

    #[test]
    fn a_change_inside_two_errors_is_noise() {
        // Errors 0.03 and 0.04 give a change error of 0.05: noise up to 0.1.
        let b = baseline(vec![
            row("goals_per_match", 2.8, 0.03),
            row("shots_per_team", 12.0, 0.03),
            row("wall_ms", 10.0, 0.0),
        ]);
        let new = vec![
            row("goals_per_match", 2.9, 0.04),
            row("shots_per_team", 12.2, 0.04),
            row("wall_ms", 99.0, 0.0),
        ];
        let d = diff(&b, "bbbbbbbbbbbb", &new);
        assert_eq!(d.rows.len(), 2, "the time budget is left out");
        assert_eq!(d.rows[0].error, 0.05);
        assert!(d.rows[0].noise, "{:?}", d.rows[0]);
        assert!(!d.rows[1].noise, "{:?}", d.rows[1]);
        assert_eq!((d.noise, d.changes), (1, 1));
        assert!(d.content_changed);
        let table = render_table(&b.info, &d);
        assert!(table.contains("content aaaaaaaaaaaa -> bbbbbbbbbbbb, content changed"));
        assert!(table.contains("noise"), "{table}");
    }

    #[test]
    fn a_rerun_against_itself_is_all_noise_with_no_change() {
        let bands = vec![
            row("goals_per_match", 2.8, 0.03),
            row("goalless_share", 0.1, 0.0),
        ];
        let d = diff(&baseline(bands.clone()), "aaaaaaaaaaaa", &bands);
        assert!(d.rows.iter().all(|r| r.noise && r.change == 0.0));
        assert!(!d.content_changed);
    }
}
