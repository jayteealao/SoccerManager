//! The rules stage of `engine-cli calibrate --base`: with the `sensitivity` feature, a change
//! run plays the sensitivity rules of its content folder and reports one row per rule and one
//! for the five-match rule. Run with
//! `cargo test -p engine-cli --features sensitivity --test calibrate_rules`.

mod common;

use std::path::Path;
use std::process::Output;

use common::{RecordSchemas, bin, edited_content, record, temp};
use serde_json::{Value, json};

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_engine-cli")
}

/// A short change run of the equal suite into `out`, on `content`, against this build.
fn change_run(data: &Path, content: &Path, out: &Path) -> Output {
    bin(data)
        .arg("--content-dir")
        .arg(content)
        .arg("calibrate")
        .args(["--seed", "9", "--suite", "equal", "--minutes", "5"])
        .args(["--jobs", "4", "--matches", "4", "--base-binary", exe()])
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The shipped rules file at a size a test can play: a few design and arm matches, and the
/// five-match rule for strikers only.
fn small(rules: &mut Value) {
    rules["design"] = json!({ "matches": 8, "arm_matches": 4, "seed": 42, "resamples": 99 });
    rules["five_match"]["roles"] = json!(["ST"]);
    rules["five_match"]["runs"] = json!(2);
    rules["five_match"]["matches_per_run"] = json!(1);
}

/// Every rule of the content folder is one row, in the file's order, with the five-match rule
/// last; a rule whose threshold no move can meet reads fail; the counts, the console line and
/// the table agree with the rows; and the report keeps to its schema.
#[test]
fn a_change_run_reports_one_row_per_sensitivity_rule_and_the_five_match_rule() {
    let data = temp("engine-cli-rules", "rows");
    let content = edited_content(&data, "sensitivity.json", |rules| {
        small(rules);
        let passing = rules["rules"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["job"] == "passing")
            .unwrap();
        passing["min_move"] = json!(10.0);
        passing["reason"] = json!("a test threshold no move can meet");
    });
    let jobs: Vec<String> = {
        let file: Value =
            serde_json::from_slice(&std::fs::read(content.join("sensitivity.json")).unwrap())
                .unwrap();
        file["rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["job"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(jobs.len(), 40, "37 attribute jobs and 3 body jobs");

    let run = data.join("run");
    let out = change_run(&data, &content, &run);
    let text = stderr(&out);
    assert!(matches!(out.status.code(), Some(0 | 2)), "{text}");
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    RecordSchemas::load()
        .report(&report)
        .unwrap_or_else(|e| panic!("{e}\n{report}"));
    assert_eq!(report["schema.version"], "3");

    let stage = &report["calib.rules"];
    assert_eq!(stage["built"], true);
    assert_eq!(stage["checked"], 41);
    assert_eq!(stage["levels"], 82);
    let rows = stage["rows"].as_array().unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r["job"].as_str().unwrap()).collect();
    assert_eq!(
        names[..40],
        jobs.iter().map(String::as_str).collect::<Vec<_>>()[..]
    );
    assert_eq!(names[40], "five-match");

    let five = &rows[40];
    assert_eq!(five["levels"], json!([10.0, 16.0]));
    assert!(five["roles"]["ST"].is_number(), "{five}");
    assert!(five["share"]["lo"].as_f64() <= five["share"]["hi"].as_f64());

    let passing = rows.iter().find(|r| r["job"] == "passing").unwrap();
    assert_eq!(passing["word"], "fail", "{passing}");
    assert!(passing["move"]["hi"].as_f64().unwrap() < 10.0, "{passing}");

    let count = |w: &str| rows.iter().filter(|r| r["word"] == w).count();
    assert_eq!(
        count("pass") + count("fail") + count("not_sure"),
        41,
        "every row has a word"
    );
    assert_eq!(stage["failed"], count("fail"));
    assert_eq!(stage["not_sure"], count("not_sure"));
    assert!(
        text.contains(&format!(
            "rules stage: 41 touched sensitivity rules checked at 82 levels, {} failed, {} not sure",
            count("fail"),
            count("not_sure")
        )),
        "{text}"
    );
    assert!(
        text.contains("five-match"),
        "the stage prints its table: {text}"
    );
    let _ = std::fs::remove_dir_all(&data);
}

/// A rules file the loader refuses stops a change run before any match plays.
#[test]
fn a_refused_rules_file_stops_the_change_run_before_play() {
    let data = temp("engine-cli-rules", "refused");
    let content = edited_content(&data, "sensitivity.json", |rules| {
        small(rules);
        let list = rules["rules"].as_array_mut().unwrap();
        list.retain(|r| r["job"] != "vision");
    });
    let run = data.join("run");
    let out = change_run(&data, &content, &run);
    let text = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("job vision has no rule"), "{text}");
    assert!(!run.join("report.json").exists(), "no run was made");
    let _ = std::fs::remove_dir_all(&data);
}
