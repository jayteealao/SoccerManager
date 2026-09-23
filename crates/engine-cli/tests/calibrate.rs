//! `engine-cli calibrate`: many AI-managed matches in worker processes, one statistics record
//! and one event file per match, and one run report checked against the realism bands.
//!
//! The smoke test runs in the default suite. The slow test plays the two 1000-match suites of
//! the accepted criteria; run it in release with `--ignored`.

mod common;

use std::path::Path;

use common::{RecordSchemas, bin, record, temp};
use serde_json::Value;

/// Files in `dir`, sorted.
fn files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .map(|entries| entries.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default();
    out.sort();
    out
}

fn band<'a>(report: &'a Value, suite: &str, name: &str) -> &'a Value {
    report["calib.bands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["suite"] == suite && b["band"] == name)
        .unwrap_or_else(|| panic!("no {suite} {name} band in {report}"))
}

#[test]
fn a_small_run_writes_a_record_per_match_and_a_report_that_validate() {
    let data = temp("engine-cli-calibrate", "smoke");
    let run = data.join("run");
    let out = bin(&data)
        .args([
            "calibrate",
            "--seed",
            "1",
            "--matches",
            "4",
            "--minutes",
            "5",
            "--jobs",
            "2",
            "--keep-events",
            "all",
        ])
        .arg("--out")
        .arg(&run)
        .output()
        .unwrap();
    let code = out.status.code();
    // Five-minute matches are not judged against the bands, so either verdict is fine.
    assert!(
        matches!(code, Some(0 | 2)),
        "exit {code:?}; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let schemas = RecordSchemas::load();
    let printed = record(&String::from_utf8_lossy(&out.stdout));
    let saved = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    assert_eq!(printed, saved);
    schemas
        .report(&saved)
        .unwrap_or_else(|e| panic!("{e}\n{saved}"));
    assert_eq!(saved["record.kind"], "run-report");
    assert_eq!(saved["operation"], "calibrate");
    assert_eq!(saved["calib.matches"], 4);
    assert_eq!(saved["calib.jobs"], 2);
    assert_eq!(saved["darkpath.match_without_stats"], 0);
    assert_eq!(saved["events.files_written"], 8);
    assert_eq!(saved["events.files_kept"], 8);
    assert!(saved["bench.match_wall_ms"].is_u64());
    assert!(saved["bench.cpu_us_per_tick"].is_number());
    assert_eq!(code == Some(0), saved["calib.pass"] == true);

    let stats = files(&run.join("stats"));
    assert_eq!(stats.len(), 8);
    let mut change_never_applied = 0;
    let mut change_expired = 0;
    for path in &stats {
        let s = record(&std::fs::read_to_string(path).unwrap());
        schemas
            .stats(&s)
            .unwrap_or_else(|e| panic!("{e}\n{}", path.display()));
        assert_eq!(s["outcome"], "success");
        assert_eq!(s["validate.ran"], true);
        assert_eq!(s["manager.kind"], serde_json::json!(["ai", "ai"]));
        change_never_applied += s["darkpath.change_never_applied"].as_u64().unwrap();
        change_expired += s["change.expired_at_full_time"].as_u64().unwrap();
    }
    assert_eq!(saved["darkpath.change_never_applied"], change_never_applied);
    assert_eq!(saved["change.expired_at_full_time"], change_expired);
    let events = files(&run.join("events"));
    assert_eq!(events.len(), 8);
    for path in &events {
        for line in std::fs::read_to_string(path).unwrap().lines() {
            schemas
                .event(&record(line))
                .unwrap_or_else(|e| panic!("{e}\n{}", path.display()));
        }
    }
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_run_keeps_only_the_event_files_of_outliers_by_default() {
    let data = temp("engine-cli-calibrate", "prune");
    let run = data.join("run");
    let out = bin(&data)
        .args([
            "calibrate",
            "--seed",
            "3",
            "--matches",
            "2",
            "--minutes",
            "3",
            "--jobs",
            "2",
            "--suite",
            "equal",
        ])
        .arg("--out")
        .arg(&run)
        .output()
        .unwrap();
    assert!(matches!(out.status.code(), Some(0 | 2)));
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    assert_eq!(report["events.files_written"], 2);
    let outliers = report["calib.suites"]["equal"]["outliers"]
        .as_u64()
        .unwrap();
    assert_eq!(report["events.files_kept"], outliers);
    assert_eq!(files(&run.join("events")).len() as u64, outliers);
    assert_eq!(files(&run.join("stats")).len(), 2);
    assert!(report["calib.suites"].get("strength").is_none());
    let _ = std::fs::remove_dir_all(&data);
}

/// The accepted criteria over two suites of 1000 full matches: the realism bands over equal
/// strength, the stronger club winning more than half, the dark paths at zero, and each
/// suite inside the 30-minute budget with the single-thread figure recorded.
#[test]
#[ignore = "slow: 2000 full matches; run in release with --ignored"]
fn a_thousand_matches_hold_the_realism_bands() {
    let data = temp("engine-cli-calibrate", "slow");
    let run = data.join("run");
    let out = bin(&data)
        .args(["calibrate", "--seed", "2026", "--matches", "1000"])
        .arg("--out")
        .arg(&run)
        .output()
        .unwrap();
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    eprintln!("{report:#}");
    RecordSchemas::load().report(&report).unwrap();
    let within = |b: &Value| b["pass"] == true;
    // Equal strength: goals, shots, and possession.
    for name in [
        "goals_per_match",
        "shots_per_team",
        "possession_home_pct",
        "possession_away_pct",
    ] {
        assert!(within(band(&report, "equal", name)), "{name} misses");
    }
    // The stronger club wins more than half.
    assert!(within(band(&report, "strength", "stronger_team_win_rate")));
    // The dark paths read zero.
    assert_eq!(report["darkpath.match_without_stats"], 0);
    assert_eq!(report["darkpath.change_never_applied"], 0);
    // The time budget and the single-thread figure.
    for suite in ["equal", "strength"] {
        assert!(within(band(&report, suite, "wall_ms")), "{suite} too slow");
    }
    assert!(report["bench.match_wall_ms"].as_u64().unwrap() > 0);
    assert!(report["bench.cpu_us_per_tick"].is_number());
    assert_eq!(out.status.code(), Some(0));
    let _ = std::fs::remove_dir_all(&data);
}
