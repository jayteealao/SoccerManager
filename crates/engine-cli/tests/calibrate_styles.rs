//! `engine-cli calibrate --suite styles`: every ordered pairing of the eight named styles,
//! identical squads on both sides, reported as points per match, the dominance check and the
//! identity flags; and the styles sample of a change run, played on the changed engine only.

mod common;

use std::path::Path;
use std::process::Output;

use common::{RecordSchemas, bin, edited_content, record, set_counts, temp};
use serde_json::Value;

/// The shipped styles and their ordered pairings.
const STYLES: usize = 8;
const PAIRINGS: usize = STYLES * (STYLES - 1);

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn report_of(run: &Path) -> Value {
    record(&std::fs::read_to_string(run.join("report.json")).unwrap())
}

/// The styles report's shape: a row per style with its points, share and flags; a row per
/// ordered pairing with `per` matches.
fn check_styles(styles: &Value, per: u64) {
    assert_eq!(styles["matches_per_pairing"], per);
    assert_eq!(styles["matches"], per * PAIRINGS as u64);
    assert_eq!(styles["played"], per * PAIRINGS as u64, "every match plays");
    assert_eq!(styles["violations"], 0);
    let rows = styles["styles"].as_array().unwrap();
    assert_eq!(rows.len(), STYLES);
    let mut share = 0.0;
    for r in rows {
        // Each style plays every other style home and away.
        assert_eq!(r["matches"], per * 2 * (STYLES as u64 - 1), "{r}");
        assert!(r["points_per_match"].as_f64().unwrap() <= 3.0);
        share += r["share"].as_f64().unwrap();
        let identity = r["identity"].as_array().unwrap();
        assert!((2..=3).contains(&identity.len()), "{r}");
        assert!(r["dominant"].is_boolean() && r["clear_losses"].is_array());
    }
    assert!((share - 1.0).abs() < 1e-3, "shares sum to {share}");
    let pairings = styles["pairings"].as_array().unwrap();
    assert_eq!(pairings.len(), PAIRINGS);
    let mut seen = std::collections::BTreeSet::new();
    for p in pairings {
        assert_ne!(p["home"], p["away"]);
        assert_eq!(p["matches"], per);
        seen.insert((p["home"].to_string(), p["away"].to_string()));
    }
    assert_eq!(seen.len(), PAIRINGS, "every ordered pairing once");
    assert!(styles["dominant"].is_array() && styles["identity_flags"].is_array());
    let gate = &styles["gate"];
    assert!(gate["pass"].is_boolean(), "{gate}");
    assert_eq!(gate["dominant"], styles["dominant"]);
    assert_eq!(gate["identity_flags"], styles["identity_flags"]);
    assert_eq!(gate["violations"], styles["violations"]);
    assert_eq!(gate["unplayed"], 0);
    for p in gate["pairings_outside"].as_array().unwrap() {
        let shots = p["shots_per_match"].as_f64().unwrap();
        assert!(!(4.0..=40.0).contains(&shots), "{p}");
    }
}

/// One match per ordered pairing plays 56 matches, each pairing once, and reports
/// points, dominance and identity.
#[test]
fn the_styles_suite_plays_every_ordered_pairing_and_reports() {
    let data = temp("engine-cli-styles", "suite");
    let run = data.join("run");
    let out = bin(&data)
        .args(["calibrate", "--seed", "42", "--suite", "styles"])
        .args(["--matches", "1", "--minutes", "3", "--jobs", "4"])
        .arg("--out")
        .arg(&run)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let report = report_of(&run);
    assert_eq!(report["record.kind"], "styles-report");
    check_styles(&report["calib.styles"], 1);
    let printed = record(&String::from_utf8_lossy(&out.stdout));
    assert_eq!(printed, report);
    assert!(stderr(&out).contains("dominant:"), "{}", stderr(&out));
    assert!(stderr(&out).contains("identity flags:"), "{}", stderr(&out));
    assert!(stderr(&out).contains("styles gate: "), "{}", stderr(&out));
}

/// The styles gate on the shipped presets: 200 matches per ordered pairing on seed 9
/// (11,200 full matches), no dominant style, no identity flag, every ordered pairing between
/// 4 and 40 shots a match, no violation and every match played. Slow; run with
/// `--ignored`.
#[test]
#[ignore = "plays 11,200 full matches"]
fn the_styles_gate_passes_on_the_shipped_presets() {
    let data = temp("engine-cli-styles", "gate");
    let run = data.join("run");
    let out = bin(&data)
        .args(["calibrate", "--seed", "9", "--suite", "styles"])
        .args(["--matches", "200"])
        .arg("--out")
        .arg(&run)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let styles = &report_of(&run)["calib.styles"];
    check_styles(styles, 200);
    assert_eq!(styles["gate"]["pass"], true, "{}", styles["gate"]);
}

/// The styles suite runs alone: not beside a base engine or a paired flag.
#[test]
fn the_styles_suite_refuses_a_base_or_a_pair() {
    let data = temp("engine-cli-styles", "refuse");
    let out = bin(&data)
        .args([
            "calibrate",
            "--seed",
            "1",
            "--suite",
            "styles",
            "--matches",
            "1",
        ])
        .args(["--base-binary", env!("CARGO_BIN_EXE_engine-cli")])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stderr(&out).contains("runs alone"), "{}", stderr(&out));
}

/// A change run adds the styles sample, one match per ordered pairing, and the old engine
/// plays only the run's own suite: it never gets the styles suite.
#[test]
fn a_change_run_adds_a_styles_sample_the_old_engine_never_plays() {
    let data = temp("engine-cli-styles", "change");
    let content = edited_content(&data, "tuning.json", |_| {});
    set_counts(&content, 2);
    let run = data.join("run");
    let out = bin(&data)
        .arg("--content-dir")
        .arg(&content)
        .arg("calibrate")
        .args(["--seed", "9", "--minutes", "3"])
        .args(["--suite", "equal", "--jobs", "4"])
        .args(["--base-binary", env!("CARGO_BIN_EXE_engine-cli")])
        .arg("--intent")
        .arg(common::intent_file(&data, &[]))
        .arg("--out")
        .arg(&run)
        .output()
        .unwrap();
    let code = out.status.code();
    assert!(
        matches!(code, Some(0 | 2)),
        "exit {code:?}; stderr: {}",
        stderr(&out)
    );
    let report = report_of(&run);
    RecordSchemas::load()
        .report(&report)
        .unwrap_or_else(|e| panic!("{e}\n{report}"));
    // The old engine played the equal suite's two matches and nothing else.
    assert!(
        stderr(&out).contains(": 0 results from the cache, 2 played"),
        "{}",
        stderr(&out)
    );
    check_styles(&report["calib.styles"], 1);
    assert!(
        report["calib.stages"]["styles"]["ms"].is_u64(),
        "{}",
        report["calib.stages"]
    );
    // The sample judges nothing: no band row names it.
    assert!(
        report["calib.bands"]
            .as_array()
            .unwrap()
            .iter()
            .all(|b| b["suite"] != "styles")
    );
}
