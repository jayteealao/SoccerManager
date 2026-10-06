//! `engine-cli calibrate --pair <flag>`: both arms play the same fixtures on the same match
//! seeds, each into its own folder, and one run report shows every band of both arms side by
//! side with a verdict. `--flag` sets a state for a whole run, and a bad name or state is
//! refused before any match is played.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use common::{RecordSchemas, bin, record, temp};
use serde_json::{Value, json};

const FLAG: &str = "probe_short_range";
const REMOVAL: &str = "removed after one paired run of 1000 matches per suite";

/// Copies `from` into `to`, folders included.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// The shipped content folder with one declared flag that shortens the shot range.
fn flagged_content(data: &Path) -> PathBuf {
    let content = data.join("content");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        &content,
    );
    let path = content.join("tuning.json");
    let mut tuning: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    tuning["flags"] = json!({
        FLAG: {
            "owner": "engine team",
            "hypothesis": "a shorter shot range lowers goals per match toward 2.8",
            "removal_condition": REMOVAL,
            "state": "off",
            "overrides": { "engine.shot_range": 12.0 }
        }
    });
    std::fs::write(&path, serde_json::to_string_pretty(&tuning).unwrap()).unwrap();
    content
}

/// A short run of the equal suite only, so the test stays fast.
fn calibrate(data: &Path, content: &Path, run: &Path, extra: &[&str]) -> std::process::Output {
    bin(data)
        .arg("--content-dir")
        .arg(content)
        .args([
            "calibrate",
            "--seed",
            "1",
            "--suite",
            "equal",
            "--matches",
            "4",
            "--minutes",
            "5",
            "--jobs",
            "2",
            "--keep-events",
            "all",
        ])
        .args(extra)
        .arg("--out")
        .arg(run)
        .output()
        .unwrap()
}

/// Every statistics record in `dir`, each checked against its schema, keyed by match seed,
/// without the keys that measure time or name the match.
fn stats_by_seed(dir: &Path) -> BTreeMap<u64, Value> {
    let schemas = RecordSchemas::load();
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let path = e.unwrap().path();
            let mut s = record(&std::fs::read_to_string(&path).unwrap());
            schemas
                .stats(&s)
                .unwrap_or_else(|e| panic!("{e}\n{}", path.display()));
            let obj = s.as_object_mut().unwrap();
            for key in ["match.id", "ts", "duration_ms", "engine.ticks_per_s"] {
                obj.remove(key);
            }
            (s["seed"].as_u64().unwrap(), s)
        })
        .collect()
}

#[test]
fn a_paired_run_plays_both_arms_on_the_same_seeds_and_compares_every_band() {
    let data = temp("engine-cli-pair", "paired");
    let content = flagged_content(&data);
    let run = data.join("run");
    let out = calibrate(&data, &content, &run, &["--pair", FLAG]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let code = out.status.code();
    // Five-minute matches are not judged against the bands; exit 2 means an arm was not
    // trustworthy, which the counters below rule out for this run.
    assert!(
        matches!(code, Some(0 | 2)),
        "exit {code:?}; stderr: {stderr}"
    );

    let schemas = RecordSchemas::load();
    let off = stats_by_seed(&run.join("arms/off/stats"));
    let on = stats_by_seed(&run.join("arms/on/stats"));
    assert_eq!(off.len(), 4);
    assert_eq!(on.len(), 4);
    assert_eq!(
        off.keys().collect::<Vec<_>>(),
        on.keys().collect::<Vec<_>>(),
        "both arms play the same match seeds"
    );
    for (arm, records, flags_on) in [("off", &off, json!([])), ("on", &on, json!([FLAG]))] {
        for s in records.values() {
            assert_eq!(s["tuning.flags_on"], flags_on, "{arm}");
        }
        // Every event file an arm kept is a valid event stream.
        for entry in std::fs::read_dir(run.join("arms").join(arm).join("events")).unwrap() {
            let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
            for line in text.lines() {
                schemas.event(&record(line)).unwrap();
            }
        }
    }

    let printed = record(&String::from_utf8_lossy(&out.stdout));
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    assert_eq!(printed, report);
    schemas
        .report(&report)
        .unwrap_or_else(|e| panic!("{e}\n{report}"));
    assert_eq!(report["calib.pair"]["flag"], FLAG);
    assert_eq!(report["calib.pair"]["removal_condition"], REMOVAL);
    assert_eq!(report["calib.arms"]["off"]["tuning.flags_on"], json!([]));
    assert_eq!(report["calib.arms"]["on"]["tuning.flags_on"], json!([FLAG]));
    assert_ne!(
        report["calib.arms"]["off"]["content.hash"],
        report["calib.arms"]["on"]["content.hash"]
    );
    // The top-level figures are the off arm's.
    assert_eq!(
        report["calib.suites"],
        report["calib.arms"]["off"]["calib.suites"]
    );
    assert_eq!(
        report["calib.flags"],
        json!([{ "name": FLAG, "owner": "engine team", "state": "paired", "source": "cli" }])
    );

    // One row per realism band of the equal suite, sorted by name, each with both values
    // side by side.
    let rows = report["calib.compare"].as_array().unwrap();
    assert!(rows.iter().all(|r| r["suite"] == "equal"), "{rows:?}");
    let names: Vec<&str> = rows.iter().map(|r| r["band"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "corners_per_team",
            "goal_kicks_per_match",
            "goalless_share",
            "goals_per_match",
            "goals_per_xg",
            "pass_accuracy_pct",
            "passes_per_team",
            "possession_away_pct",
            "possession_home_pct",
            "sending_off_share",
            "shots_on_target_share",
            "shots_per_team",
            "ten_plus_goals_share",
            "throw_ins_per_match",
            "yellow_cards_per_team",
        ]
    );
    for row in rows {
        assert!(row["off"].is_number() && row["on"].is_number(), "{row}");
        let band = |arm: &str| {
            report["calib.arms"][arm]["calib.bands"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["suite"] == row["suite"] && b["band"] == row["band"])
                .unwrap()["value"]
                .clone()
        };
        assert_eq!(row["off"], band("off"), "{row}");
        assert_eq!(row["on"], band("on"), "{row}");
    }
    assert!(report["calib.verdict"].is_string());
    assert!(stderr.contains("verdict: "), "{stderr}");
    assert!(stderr.contains("goals_per_match"), "{stderr}");
    // Each arm names its failing bands before the table.
    let failed = stderr
        .find("calibrate.band_failed")
        .expect("a failing band line");
    assert!(
        failed < stderr.find("paired run of flag").unwrap(),
        "{stderr}"
    );
    let trusted = ["off", "on"].iter().all(|arm| {
        let a = &report["calib.arms"][*arm];
        a["darkpath.match_without_stats"] == 0
            && a["darkpath.change_never_applied"] == 0
            && a["validate.violations"] == 0
            && a["calib.workers_failed"] == 0
    });
    assert_eq!(code == Some(0), trusted);

    // An unpaired run on the same seed plays exactly the off arm's matches.
    let single = data.join("single");
    let out = calibrate(&data, &content, &single, &[]);
    assert!(matches!(out.status.code(), Some(0 | 2)));
    assert_eq!(stats_by_seed(&single.join("stats")), off);
    let unpaired = record(&std::fs::read_to_string(single.join("report.json")).unwrap());
    assert!(unpaired.get("calib.pair").is_none());
    assert_eq!(unpaired["calib.flags"][0]["state"], "off");
    assert_eq!(unpaired["calib.flags"][0]["source"], "file");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_single_state_run_plays_every_match_with_the_flag_on() {
    let data = temp("engine-cli-pair", "flag-on");
    let content = flagged_content(&data);
    let run = data.join("run");
    let out = bin(&data)
        .arg("--content-dir")
        .arg(&content)
        .args([
            "calibrate",
            "--seed",
            "2",
            "--matches",
            "2",
            "--minutes",
            "3",
            "--jobs",
            "2",
            "--suite",
            "equal",
            "--keep-events",
            "all",
            "--flag",
        ])
        .arg(format!("{FLAG}=on"))
        .arg("--out")
        .arg(&run)
        .output()
        .unwrap();
    assert!(matches!(out.status.code(), Some(0 | 2)));
    for s in stats_by_seed(&run.join("stats")).values() {
        assert_eq!(s["tuning.flags_on"], json!([FLAG]));
    }
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    RecordSchemas::load().report(&report).unwrap();
    assert_eq!(
        report["calib.flags"],
        json!([{ "name": FLAG, "owner": "engine team", "state": "on", "source": "cli" }])
    );
    assert!(report.get("calib.compare").is_none());
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_bad_flag_name_or_state_is_refused_before_any_match() {
    let data = temp("engine-cli-pair", "refused");
    let content = flagged_content(&data);
    for (args, named) in [
        (vec!["--flag", "nope=on"], "nope"),
        (vec!["--flag", "probe_short_range=maybe"], "maybe"),
        (vec!["--pair", "nope"], "nope"),
        (
            vec!["--pair", FLAG, "--flag", "probe_short_range=on"],
            "both states",
        ),
    ] {
        let run = data.join("run");
        let out = calibrate(&data, &content, &run, &args);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_ne!(out.status.code(), Some(0), "{args:?}");
        assert!(stderr.contains(named), "{args:?}: {stderr}");
        assert!(!run.join("stats").exists(), "{args:?} played a match");
        assert!(!run.join("arms").exists(), "{args:?} played a match");
    }
    let _ = std::fs::remove_dir_all(&data);
}
