//! The substitution report of `engine-cli calibrate`: a short run on the three-substitution
//! rule fixture (`--rules`) reports each team's substitutions beside the real era of its
//! limit (the Wyscout leagues, never a held-out set), and a run on the shipped pack beside the
//! five-substitution tournaments; the results digest of the shipped run does not depend on
//! the report.

mod common;

use std::path::Path;

use common::{RecordSchemas, bin, record, temp};
use serde_json::Value;

/// A short equal-suite run of `matches` matches into `out`, with `extra` arguments.
fn run(data: &Path, out: &Path, extra: &[&str]) -> Value {
    let output = bin(data)
        .arg("calibrate")
        .args([
            "--seed",
            "9",
            "--suite",
            "equal",
            "--matches",
            "4",
            "--jobs",
            "4",
        ])
        .args(extra)
        .arg("--out")
        .arg(out)
        .output()
        .unwrap();
    let code = output.status.code();
    assert!(
        matches!(code, Some(0 | 2)),
        "exit {code:?}; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = record(&std::fs::read_to_string(out.join("report.json")).unwrap());
    RecordSchemas::load()
        .report(&report)
        .unwrap_or_else(|e| panic!("{e}\n{report}"));
    report
}

#[test]
fn a_run_reports_its_substitutions_beside_the_real_era_of_its_rule_pack() {
    let data = temp("engine-cli-calibrate-subs", "eras");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rules/three-substitutions.json");
    let three = run(
        &data,
        &data.join("three"),
        &["--rules", fixture.to_str().unwrap()],
    );
    let five = run(&data, &data.join("five"), &[]);
    let s3 = &three["calib.substitutions"];
    let s5 = &five["calib.substitutions"];
    assert_eq!(s3["limit"], 3);
    assert_eq!(s5["limit"], 5);
    assert_eq!(s3["matches"], 4);
    // No side makes more substitutions than its pack allows.
    assert!(s3["count_shares"].as_array().unwrap().len() <= 4, "{s3}");
    assert_eq!(s3["real"]["source"], "wyscout_2017_18");
    assert_eq!(
        s3["real"]["competitions"],
        serde_json::json!(["England", "France", "Italy", "Spain"])
    );
    assert_eq!(s5["real"]["source"], "statsbomb_open");
    for held_out in [
        "Germany",
        "World_Cup",
        "European_Championship",
        "2018",
        "2016",
    ] {
        assert!(!s3.to_string().contains(held_out), "{held_out}: {s3}");
        assert!(!s5.to_string().contains(held_out), "{held_out}: {s5}");
    }
    // The rule pack is part of the content: the two runs' content differs.
    assert_ne!(three["content.hash"], five["content.hash"]);
    std::fs::remove_dir_all(&data).unwrap();
}
