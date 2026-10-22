//! The equivalence pass rule end to end: one short run plays, then its stored rows are judged
//! again under edited band files, with no match played. A band passes only when its whole
//! interval sits inside its margin (pass), fails when the whole interval sits outside it, and
//! is not sure otherwise; a rare band with no events keeps an interval; a design target with
//! a cap fails on the cap; the whole-distribution bands carry margins above their noise
//! floors; and each run adds one point per band to the band history, whose drifts are listed
//! without changing the joint word.

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{RecordSchemas, bin, copy_tree, hold_floor, record, set_design};
use serde_json::{Value, json};
use tempfile::TempDir;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A short run of the equal suite on `content`, its history in `history`. The fixture's
/// small real samples put some margins under their floor: the run widens them in `content`,
/// a test's own copy, and plays.
fn calibrate(data: &Path, content: &Path, out: &Path, history: &Path) -> Output {
    let run = || {
        bin(data)
            .env("SM_BAND_HISTORY", history)
            .arg("--content-dir")
            .arg(content)
            .arg("calibrate")
            .args(["--seed", "9", "--suite", "equal", "--minutes", "5"])
            .args(["--matches", "12", "--jobs", "4"])
            .arg("--real-data")
            .arg(real_data(data))
            .arg("--out")
            .arg(out)
            .output()
            .unwrap()
    };
    let mut out = run();
    for _ in 0..3 {
        if !hold_floor(content, &String::from_utf8_lossy(&out.stderr)) {
            break;
        }
        out = run();
    }
    out
}

/// The real-data folder of a run: a test's own under `data` when it wrote one, the fixture
/// folder otherwise.
fn real_data(data: &Path) -> PathBuf {
    let own = data.join("real-data");
    if own.exists() {
        own
    } else {
        repo().join("crates/engine-cli/tests/fixtures/real-data")
    }
}

/// Writes a real-data folder under `data`: the fixture folder with a team-match file of 400
/// rows, so that a band read from it has a small real error.
fn wide_real_data(data: &Path) {
    let own = data.join("real-data");
    copy_tree(
        &repo().join("crates/engine-cli/tests/fixtures/real-data"),
        &own,
    );
    let mut rows = String::from("match,team,passes,acc,shots,corners,yellows,poss,home\n");
    for m in 0..200 {
        let passes = 440 + (m % 5) * 5;
        let (home, away) = (2 * m, 2 * m + 1);
        rows.push_str(&format!("{m},{home},{passes},0.8,12,5,1,0.55,True\n"));
        rows.push_str(&format!("{m},{away},{passes},0.8,10,4,2,0.45,False\n"));
    }
    std::fs::write(own.join("wy2_team_matches.csv"), rows).unwrap();
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The run's report, checked against the run-report schema.
fn report(out: &Output, run: &Path) -> Value {
    let code = out.status.code();
    assert!(
        matches!(code, Some(0 | 2)),
        "exit {code:?}; stderr: {}",
        stderr(out)
    );
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    RecordSchemas::load()
        .report(&report)
        .unwrap_or_else(|e| panic!("{e}\n{report}"));
    report
}

/// The checked row of `band` in the equal suite.
fn check<'a>(report: &'a Value, band: &str) -> &'a Value {
    report["calib.bands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["band"] == band && b["suite"] == "equal")
        .unwrap_or_else(|| panic!("no check of {band}"))
}

/// The verdict row of `band`, with its word and the pass rule's interval.
fn verdict<'a>(report: &'a Value, band: &str) -> &'a Value {
    report["calib.verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["band"] == band)
        .unwrap_or_else(|| panic!("no verdict for {band}"))
}

/// Edits the band `name` of the band file in `content`.
fn edit_band(content: &Path, name: &str, edit: impl FnOnce(&mut Value)) {
    let path = content.join("realism-bands.json");
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let band = doc["bands"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|b| b["band"] == name)
        .unwrap_or_else(|| panic!("no band {name}"));
    edit(band);
    std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
}

/// Gives `band` a design target at `target` with a margin of `margin`.
fn design(band: &mut Value, target: f64, margin: f64) {
    set_design(band, target, margin);
}

/// The shipped content, copied.
fn content(dir: &Path) -> PathBuf {
    let content = dir.join("content");
    copy_tree(&repo().join("content"), &content);
    content
}

#[test]
fn a_band_passes_only_with_its_whole_interval_inside_its_margin() {
    let tmp = TempDir::new().unwrap();
    let data = tmp.path().join("data");
    let content = content(tmp.path());
    let history = tmp.path().join("history.jsonl");
    let run = tmp.path().join("run");
    wide_real_data(&data);
    let first = report(&calibrate(&data, &content, &run, &history), &run);
    let accuracy = check(&first, "pass_accuracy_pct")["value"]
        .as_f64()
        .unwrap();
    let se = check(&first, "pass_accuracy_pct")["se"].as_f64().unwrap();
    assert!(se > 0.0, "{se}");

    // AC-1: the target at the engine's own value and a wide margin: pass.
    edit_band(&content, "pass_accuracy_pct", |b| {
        design(b, accuracy, 20.0 * se + 1.0)
    });
    // AC-3: a target far beyond the margin: fail (a count every match has, so no resample
    // of the short run is without spread).
    edit_band(&content, "throw_ins_per_match", |b| design(b, 1000.0, 1.0));
    // AC-2: the target at the engine's value and a margin inside the interval: not sure.
    let shots = check(&first, "shots_per_team")["value"].as_f64().unwrap();
    let shots_se = check(&first, "shots_per_team")["se"].as_f64().unwrap();
    edit_band(&content, "shots_per_team", |b| {
        design(b, shots, shots_se / 4.0);
    });
    // AC-4: a rare share with no event in the run keeps an interval whose upper end is
    // above zero; under its minimum exposure it reads not sure.
    edit_band(&content, "ten_plus_goals_share", |b| {
        design(b, 0.0, 0.05);
        b["min_exposure"]["value"] = json!(1000.0);
    });
    // AC-9: the design target met, but the cap against real football broken: fail. The
    // test's 400 team rows give the real side a small spread to resample.
    let passes = check(&first, "passes_per_team")["value"].as_f64().unwrap();
    let passes_se = check(&first, "passes_per_team")["se"].as_f64().unwrap();
    edit_band(&content, "passes_per_team", |b| {
        design(b, passes, 20.0 * passes_se + 1.0);
        b["target"]["cap"] = json!(5.0);
        b["resampling"]["real"] = json!("match");
    });

    let out = calibrate(&data, &content, &run, &history);
    let text = stderr(&out);
    let again = report(&out, &run);
    assert!(text.contains("0 matches played"), "{text}");
    assert_eq!(again["calib.units"]["played"], 0);

    let word = |band: &str| verdict(&again, band)["word"].as_str().unwrap().to_string();
    assert_eq!(
        word("pass_accuracy_pct"),
        "pass",
        "{}",
        verdict(&again, "pass_accuracy_pct")
    );
    assert_eq!(word("throw_ins_per_match"), "fail");
    let unsure = verdict(&again, "shots_per_team");
    assert_eq!(word("shots_per_team"), "not sure", "{unsure}");
    let rule = &unsure["pass_rule"];
    assert!(rule["lo"].is_number() && rule["hi"].is_number(), "{unsure}");
    assert_eq!(rule["margin"].as_f64(), Some(shots_se / 4.0));
    assert!(text.contains("shots_per_team"), "{text}");

    let rare = verdict(&again, "ten_plus_goals_share");
    assert_eq!(check(&first, "ten_plus_goals_share")["value"], 0.0);
    assert_eq!(word("ten_plus_goals_share"), "not sure", "{rare}");
    let hi = rare["pass_rule"]["hi"].as_f64().unwrap();
    assert!(
        hi > 0.0,
        "zero events keep an upper bound above zero: {rare}"
    );
    assert_eq!(rare["pass_rule"]["lo"].as_f64(), Some(0.0), "{rare}");

    assert_eq!(
        word("passes_per_team"),
        "fail",
        "{}",
        verdict(&again, "passes_per_team")
    );
    assert_eq!(again["calib.pass"], false);
}

#[test]
fn every_whole_distribution_band_has_a_margin_above_its_noise_floor() {
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(repo().join("content/realism-bands.json")).unwrap(),
    )
    .unwrap();
    let mut seen = Vec::new();
    for b in file["bands"].as_array().unwrap() {
        let kind = b["measure"]["kind"].as_str().unwrap();
        if kind != "distribution" && kind != "cells" {
            continue;
        }
        let name = b["band"].as_str().unwrap();
        let margin = b["margin"]["value"].as_f64().unwrap();
        let floor = b["engine_spread"]["noise_floor"].as_f64();
        if let Some(floor) = floor {
            assert!(
                margin > floor,
                "{name}: margin {margin}, noise floor {floor}"
            );
        }
        seen.push(name.to_string());
    }
    for band in [
        "goals_distribution",
        "scoreline_cells",
        "shots_total_distribution",
        "cards_total_distribution",
    ] {
        assert!(seen.iter().any(|s| s == band), "no band {band}: {seen:?}");
    }
}

#[test]
fn each_run_adds_a_point_per_band_and_a_drift_never_changes_the_word() {
    let tmp = TempDir::new().unwrap();
    let data = tmp.path().join("data");
    let content = content(tmp.path());
    let history = tmp.path().join("history.jsonl");
    let lines = || {
        std::fs::read_to_string(&history)
            .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
            .unwrap_or(0)
    };
    let one = tmp.path().join("one");
    let first = report(&calibrate(&data, &content, &one, &history), &one);
    let added = first["calib.drift"]["added"].as_u64().unwrap() as usize;
    assert!(added > 0);
    assert_eq!(lines(), added);
    let two = tmp.path().join("two");
    let second = report(&calibrate(&data, &content, &two, &history), &two);
    assert_eq!(
        second["calib.drift"]["added"].as_u64().unwrap() as usize,
        added
    );
    assert_eq!(lines(), 2 * added);
    // Judging a run again replaces its points.
    report(&calibrate(&data, &content, &two, &history), &two);
    assert_eq!(lines(), 2 * added);

    // A planted history: goals per match steady for ten runs, then a step of five errors.
    let mut planted = String::new();
    for i in 0..10 {
        let value = if i < 6 { 2.7 } else { 3.2 };
        planted.push_str(
            &json!({
                "run": format!("planted-{i:02}"), "date": "2026-10-01", "build": "x",
                "content": "y", "band": "goals_per_match", "suite": "equal",
                "definition_version": 1, "value": value, "se": 0.1, "word": "pass",
                "matches": 1000
            })
            .to_string(),
        );
        planted.push('\n');
    }
    let drifting = tmp.path().join("drifting.jsonl");
    std::fs::write(&drifting, &planted).unwrap();
    let three = tmp.path().join("three");
    let out = calibrate(&data, &content, &three, &drifting);
    let third = report(&out, &three);
    let drifts = third["calib.drift"]["drifts"].as_array().unwrap();
    let flagged = drifts
        .iter()
        .find(|d| d["band"] == "goals_per_match")
        .unwrap_or_else(|| panic!("no drift of goals_per_match: {drifts:?}"));
    assert_eq!(flagged["direction"], "up");
    assert_eq!(flagged["since_run"], "planted-06");
    assert!(
        stderr(&out).contains("drift up: goals_per_match"),
        "{}",
        stderr(&out)
    );
    // The same matches judged with no history give the same words.
    let quiet = tmp.path().join("quiet");
    let plain = report(
        &calibrate(&data, &content, &quiet, Path::new("off")),
        &quiet,
    );
    assert!(plain.get("calib.drift").is_none());
    assert_eq!(plain["calib.pass"], third["calib.pass"]);
    let words = |r: &Value| -> Vec<(String, String)> {
        r["calib.verdicts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| (v["band"].to_string(), v["word"].to_string()))
            .collect()
    };
    assert_eq!(words(&plain), words(&third));
}
