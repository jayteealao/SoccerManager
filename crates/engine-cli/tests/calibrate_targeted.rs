//! The tuning loop of `engine-cli calibrate`: a targeted run plays only the pairing, suite,
//! or band it names, with the figures a full run gives the same fixtures; `--baseline`
//! refuses a report on other fixtures before any match plays and prints a diff that marks a
//! change inside two sampling errors as noise; and `--suite red-card` runs the controlled
//! sending-off experiment. Every run here is short, so the file stays in the default run.

mod common;

use std::path::Path;
use std::process::Output;

use common::{RecordSchemas, bin, edited_content, record, temp};
use serde_json::{Value, json};

/// The pairing the slice names, stored as `4-4-2 v 4-4-1-1` (4-4-2 is formation 0).
const PAIRING: &str = "4-4-1-1 v 4-4-2";
const STORED: &str = "4-4-2 v 4-4-1-1";

/// A calibrate run into `run`, with `args` after the subcommand.
fn calibrate(data: &Path, run: &Path, args: &[&str]) -> Output {
    calibrate_with(data, None, run, args)
}

fn calibrate_with(data: &Path, content: Option<&Path>, run: &Path, args: &[&str]) -> Output {
    let mut cmd = bin(data);
    if let Some(content) = content {
        cmd.arg("--content-dir").arg(content);
    }
    cmd.arg("calibrate")
        .args(["--minutes", "3", "--jobs", "2"])
        .args(args)
        .arg("--out")
        .arg(run)
        .output()
        .unwrap()
}

/// The saved report of a run that finished; either band verdict is fine for 3-minute
/// matches.
fn finished(out: &Output, run: &Path) -> Value {
    let code = out.status.code();
    assert!(
        matches!(code, Some(0 | 2)),
        "exit {code:?}; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    RecordSchemas::load()
        .report(&report)
        .unwrap_or_else(|e| panic!("{e}\n{report}"));
    report
}

fn count(dir: &Path) -> usize {
    std::fs::read_dir(dir).map_or(0, |d| d.count())
}

/// The band rows of `suite`, without the time budget, which measures the machine.
fn rows(report: &Value, suite: &str) -> Vec<Value> {
    report["calib.bands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["suite"] == suite && b["band"] != "wall_ms")
        .cloned()
        .collect()
}

/// A suite's figures without the outlier count, which depends on match wall time.
fn figures(report: &Value, suite: &str) -> Value {
    let mut f = report["calib.suites"][suite].clone();
    f.as_object_mut().unwrap().remove("outliers");
    f
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_targeted_pairing_and_suite_play_only_their_selection_and_agree_with_a_full_run() {
    let data = temp("engine-cli-targeted", "agree");
    let full_dir = data.join("full");
    let full = finished(
        &calibrate(&data, &full_dir, &["--seed", "5", "--matches", "2"]),
        &full_dir,
    );

    let pair_dir = data.join("pairing");
    let pair = finished(
        &calibrate(
            &data,
            &pair_dir,
            &[
                "--seed",
                "5",
                "--matches",
                "2",
                "--suite",
                "formations",
                "--pairing",
                PAIRING,
            ],
        ),
        &pair_dir,
    );
    // That pairing only, and 2 matches played.
    assert_eq!(count(&pair_dir.join("stats")), 2);
    let entries = pair["calib.formations"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["pairing"], json!(["4-4-2", "4-4-1-1"]));
    assert_eq!(pair["calib.suites"]["formations"]["matches"], 2);
    assert_eq!(
        pair["calib.selection"],
        json!({"suites": ["formations"], "pairings": [STORED], "bands": []})
    );
    // The same figures as the same pairing of the full run.
    let in_full = full["calib.formations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pairing"] == json!(["4-4-2", "4-4-1-1"]))
        .unwrap();
    assert_eq!(&entries[0], in_full);
    let full_rows: Vec<Value> = rows(&full, "formations")
        .into_iter()
        .filter(|r| r["pairing"] == STORED)
        .collect();
    assert_eq!(rows(&pair, "formations"), full_rows);

    let equal_dir = data.join("equal");
    let equal = finished(
        &calibrate(
            &data,
            &equal_dir,
            &["--seed", "5", "--matches", "2", "--suite", "equal"],
        ),
        &equal_dir,
    );
    assert_eq!(figures(&equal, "equal"), figures(&full, "equal"));
    assert_eq!(rows(&equal, "equal"), rows(&full, "equal"));
    assert_eq!(equal["fixtures.hash"], full["fixtures.hash"]);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_band_narrows_the_run_to_the_suites_that_check_it() {
    let data = temp("engine-cli-targeted", "band");
    let run = data.join("run");
    let report = finished(
        &calibrate(
            &data,
            &run,
            &[
                "--seed",
                "2",
                "--matches",
                "2",
                "--band",
                "stronger_team_win_rate",
            ],
        ),
        &run,
    );
    let suites: Vec<&String> = report["calib.suites"].as_object().unwrap().keys().collect();
    assert_eq!(suites, ["strength"]);
    assert_eq!(count(&run.join("stats")), 2);
    let bands: Vec<&str> = report["calib.bands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["band"].as_str().unwrap())
        .collect();
    assert_eq!(bands, ["stronger_team_win_rate", "wall_ms"]);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_bad_selection_is_refused_before_any_match() {
    let data = temp("engine-cli-targeted", "refuse-selection");
    let run = data.join("run");
    for (args, says) in [
        (
            vec!["--suite", "formations", "--pairing", "4-4-2 v 9-9-9"],
            "no such pairing",
        ),
        (vec!["--band", "no_such_band"], "no such band"),
        (
            vec!["--suite", "equal", "--pairing", PAIRING],
            "needs the formations suite",
        ),
        (
            vec!["--suite", "red-card", "--band", "goals_per_match"],
            "no suite of --suite red-card checks it",
        ),
        (
            vec!["--baseline", "report.json", "--pair", "any"],
            "cannot be used with --pair",
        ),
    ] {
        let mut all = vec!["--seed", "1", "--matches", "2"];
        all.extend(args);
        let out = calibrate(&data, &run, &all);
        let text = stderr(&out);
        assert_eq!(out.status.code(), Some(1), "{all:?}: {text}");
        assert!(text.contains(says), "{all:?}: {text}");
        assert!(!run.exists(), "{all:?} made the run folder");
    }
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_baseline_on_other_fixtures_is_refused_before_any_match_and_names_the_difference() {
    let data = temp("engine-cli-targeted", "refuse-baseline");
    let base_dir = data.join("base");
    let base = finished(
        &calibrate(
            &data,
            &base_dir,
            &["--seed", "1", "--matches", "2", "--suite", "equal"],
        ),
        &base_dir,
    );
    let baseline = base_dir.join("report.json");
    let fixtures = base["fixtures.hash"].as_str().unwrap().to_string();
    // A report made before fixtures hashes.
    let mut old = base.clone();
    old.as_object_mut().unwrap().remove("fixtures.hash");
    let old_path = data.join("old.json");
    std::fs::write(&old_path, old.to_string()).unwrap();
    // One tactics formation edited: other fixtures.
    let edited = edited_content(&data, "tactics.json", |t| {
        t["formations"][1]["slots"][10]["x"] = json!(65.0);
    });

    let run = data.join("run");
    let path = baseline.to_str().unwrap();
    for (content, args, says) in [
        (
            None,
            vec!["--seed", "2", "--matches", "2", "--baseline", path],
            "seed 2 differs from the baseline's 1".to_string(),
        ),
        (
            None,
            vec!["--seed", "1", "--matches", "3", "--baseline", path],
            "match count 3 differs from the baseline's 2".to_string(),
        ),
        (
            Some(edited.as_path()),
            vec!["--seed", "1", "--matches", "2", "--baseline", path],
            format!("differs from the baseline's {fixtures}"),
        ),
        (
            None,
            vec![
                "--seed",
                "1",
                "--matches",
                "2",
                "--baseline",
                old_path.to_str().unwrap(),
            ],
            "made before fixtures hashes".to_string(),
        ),
    ] {
        let mut all = vec!["--suite", "equal"];
        all.extend(args);
        let out = calibrate_with(&data, content, &run, &all);
        let text = stderr(&out);
        assert_eq!(out.status.code(), Some(1), "{all:?}: {text}");
        assert!(text.contains(&says), "{all:?}: {text}");
        assert!(!run.join("stats").exists(), "{all:?} played a match");
    }
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_rerun_against_its_own_report_is_noise_on_every_row() {
    let data = temp("engine-cli-targeted", "rerun");
    let args = ["--seed", "4", "--matches", "3", "--suite", "equal"];
    let base_dir = data.join("base");
    let base = finished(&calibrate(&data, &base_dir, &args), &base_dir);
    let run = data.join("run");
    let mut again = args.to_vec();
    let path = base_dir.join("report.json");
    again.extend(["--baseline", path.to_str().unwrap()]);
    let out = calibrate(&data, &run, &again);
    let report = finished(&out, &run);
    let diff = &report["calib.diff"];
    let rows = diff["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 15, "every equal-suite band but the time budget");
    assert!(
        rows.iter()
            .all(|r| r["noise"] == true && r["change"] == 0.0),
        "{rows:?}"
    );
    assert_eq!(diff["content_changed"], false);
    assert_eq!(report["calib.baseline"]["run.id"], base["run.id"]);
    let text = stderr(&out);
    assert!(text.contains("content unchanged"), "{text}");
    assert!(text.contains("baseline"), "{text}");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_changed_content_hash_on_the_same_fixtures_is_the_change_under_test() {
    let data = temp("engine-cli-targeted", "content");
    // A flag whose override leaves the generator alone: same fixtures, other content.
    let content = edited_content(&data, "tuning.json", |t| {
        t["flags"] = json!({
            "probe_short_range": {
                "owner": "engine team",
                "hypothesis": "a shorter shot range lowers goals per match",
                "removal_condition": "removed after one tuning run",
                "state": "off",
                "overrides": { "engine.shot_range": 12.0 }
            }
        });
    });
    let args = ["--seed", "6", "--matches", "2", "--suite", "equal"];
    let base_dir = data.join("base");
    let base = finished(
        &calibrate_with(&data, Some(&content), &base_dir, &args),
        &base_dir,
    );
    let run = data.join("run");
    let path = base_dir.join("report.json");
    let mut on = args.to_vec();
    on.extend([
        "--flag",
        "probe_short_range=on",
        "--baseline",
        path.to_str().unwrap(),
    ]);
    let out = calibrate_with(&data, Some(&content), &run, &on);
    let report = finished(&out, &run);
    assert_eq!(report["fixtures.hash"], base["fixtures.hash"]);
    assert_ne!(report["content.hash"], base["content.hash"]);
    let diff = &report["calib.diff"];
    assert_eq!(diff["content_changed"], true);
    assert_eq!(
        diff["content_hashes"],
        json!([base["content.hash"], report["content.hash"]])
    );
    let text = stderr(&out);
    assert!(text.contains("content changed"), "{text}");
    for hash in [&base["content.hash"], &report["content.hash"]] {
        assert!(text.contains(hash.as_str().unwrap()), "{text}");
    }
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn the_red_card_suite_reports_every_arm_the_control_and_the_verdict() {
    let data = temp("engine-cli-targeted", "red-card");
    let run = data.join("run");
    let report = finished(
        &calibrate(
            &data,
            &run,
            &[
                "--seed",
                "1",
                "--matches",
                "2",
                "--suite",
                "red-card",
                "--keep-events",
                "all",
            ],
        ),
        &run,
    );
    // Four arms of 2 matches: the control and three send-offs.
    assert_eq!(count(&run.join("stats")), 8);
    let schemas = RecordSchemas::load();
    for entry in std::fs::read_dir(run.join("stats")).unwrap() {
        let s = record(&std::fs::read_to_string(entry.unwrap().path()).unwrap());
        schemas.stats(&s).unwrap_or_else(|e| panic!("{e}\n{s}"));
        assert_eq!(s["outcome"], "success");
        assert_eq!(s["validate.violations"], 0);
    }
    for entry in std::fs::read_dir(run.join("events")).unwrap() {
        let path = entry.unwrap().path();
        for line in std::fs::read_to_string(&path).unwrap().lines() {
            schemas
                .event(&record(line))
                .unwrap_or_else(|e| panic!("{e}\n{}", path.display()));
        }
    }
    assert_eq!(report["calib.selection"]["suites"], json!(["red-card"]));
    let red = &report["calib.red_card"];
    assert_eq!(red["matches"], 2);
    assert_eq!(red["control_matches"], 2);
    let arms: Vec<&str> = red["arms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["arm"].as_str().unwrap())
        .collect();
    assert_eq!(arms, ["keeper", "centre-back", "striker"]);
    assert!(red["pass"].is_boolean());
    assert_eq!(report["validate.violations"], 0);
    let rows = rows(&report, "red-card");
    assert_eq!(rows.len(), 6);
    for arm in &arms {
        for band in ["reduced_minus_full", "full_over_control"] {
            assert!(
                rows.iter()
                    .any(|r| r["pairing"] == *arm && r["band"] == band && r["se"].is_number()),
                "no {band} row for {arm}: {rows:?}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_baseline_from_the_old_seeding_scheme_is_refused_by_name_before_any_match() {
    let data = temp("engine-cli-targeted", "old-scheme");
    let run = data.join("run");
    let ledger = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/bands/ledger-3.json");
    let out = calibrate(
        &data,
        &run,
        &[
            "--seed",
            "42",
            "--matches",
            "1000",
            "--baseline",
            ledger.to_str().unwrap(),
        ],
    );
    let text = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("old seeding scheme"), "{text}");
    assert!(text.contains("fixture-key-1"), "{text}");
    assert!(text.contains("ledger-3.json"), "{text}");
    assert!(!run.exists(), "the refused run made its folder");
    let _ = std::fs::remove_dir_all(&data);
}
