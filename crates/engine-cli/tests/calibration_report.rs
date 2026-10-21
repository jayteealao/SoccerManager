//! The calibration report shows the engine against the real targets from a finished run
//! folder, and only reports: the run's report and its verdict stay as they were.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bin(data: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_DATA_DIR", data);
    cmd.env("SM_LOG", "warn");
    cmd.env("SM_CONTENT_DIR", repo().join("content"));
    cmd
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// A short equal-suite run into `run`; `output()` waits for the child, so no process holds
/// the folder when the TempDir drops.
fn calibrate(data: &Path, run: &Path, real: &Path) -> Output {
    let o = bin(data)
        .arg("calibrate")
        .args(["--seed", "9", "--suite", "equal", "--minutes", "5"])
        .args(["--jobs", "4", "--matches", "4"])
        .arg("--real-data")
        .arg(real)
        .arg("--out")
        .arg(run)
        .output()
        .unwrap();
    assert!(matches!(o.status.code(), Some(0 | 2)), "{}", text(&o));
    o
}

fn report(run: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(run.join("report.json")).unwrap()).unwrap()
}

#[test]
fn the_report_shows_engine_against_real_and_leaves_the_verdict_alone() {
    let tmp = TempDir::new().unwrap();
    let data = tmp.path().join("data");
    let run = tmp.path().join("run");
    let real = tmp.path().join("no-real-data");
    std::fs::create_dir_all(&real).unwrap();

    // A reference file from the invented fixture data.
    let targets = tmp.path().join("gate/real-targets.json");
    let built = bin(&data)
        .args(["real-targets", "build", "--data"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-targets"))
        .arg("--out")
        .arg(&targets)
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", text(&built));

    calibrate(&data, &run, &real);
    let before = std::fs::read(run.join("report.json")).unwrap();
    let verdict = report(&run)["calib.pass"].clone();

    let o = bin(&data)
        .args(["calibrate", "report", "--run"])
        .arg(&run)
        .arg("--targets")
        .arg(&targets)
        .output()
        .unwrap();
    let out = text(&o);
    assert!(o.status.success(), "{out}");
    assert!(out.contains("calibration report: engine against real, 4 matches of the equal suite"));
    assert!(out.contains("report only"));
    // A scalar band: the engine's mean beside each real target.
    assert!(out.contains("goals_per_match: engine "), "{out}");
    assert!(out.contains("fd.E0.goals_per_match"), "{out}");
    assert!(out.contains("tier.1.goals_per_match"), "{out}");
    assert!(out.contains("engine less real"), "{out}");
    // A whole distribution: both share tables and their distance.
    assert!(
        out.contains("scoreline_cells (whole distribution, total variation)"),
        "{out}"
    );
    assert!(
        out.contains("goals_distribution (whole distribution, wasserstein-1)"),
        "{out}"
    );
    assert!(
        out.contains("engine shares:") && out.contains("real shares:"),
        "{out}"
    );
    assert!(
        out.contains("banding-group targets in calibration.json"),
        "{out}"
    );
    assert!(out.contains("bands without a real target here:"), "{out}");

    let cal: Value =
        serde_json::from_str(&std::fs::read_to_string(run.join("calibration.json")).unwrap())
            .unwrap();
    assert_eq!(cal["matches"], 4);
    let goals = cal["bands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["band"] == "goals_per_match")
        .unwrap();
    let engine = goals["engine"].as_f64().unwrap();
    let e0 = goals["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["target"] == "fd.E0.goals_per_match")
        .unwrap();
    let diff = e0["difference"].as_f64().unwrap();
    assert!((engine - e0["real"].as_f64().unwrap() - diff).abs() < 1e-9);
    let cells = cal["bands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["band"] == "scoreline_cells")
        .unwrap();
    let shares = cells["engine_shares"].as_array().unwrap();
    assert_eq!(shares.len(), 36);
    let sum: f64 = shares.iter().map(|s| s.as_f64().unwrap()).sum();
    assert!((sum - 1.0).abs() < 1e-9);
    assert!(cells["targets"][0]["distance"].as_f64().is_some());

    // The run's report is untouched, and judging the folder again keeps its verdict.
    assert_eq!(std::fs::read(run.join("report.json")).unwrap(), before);
    let again = calibrate(&data, &run, &real);
    assert!(
        text(&again).contains("0 matches played"),
        "{}",
        text(&again)
    );
    assert_eq!(report(&run)["calib.pass"], verdict);
}

#[test]
fn a_folder_without_finished_matches_is_refused() {
    let tmp = TempDir::new().unwrap();
    let o = bin(tmp.path())
        .args(["calibrate", "report", "--run"])
        .arg(tmp.path())
        .arg("--targets")
        .arg(repo().join("gate/real-targets.json"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(
        text(&o).contains("holds no finished match of the equal suite"),
        "{}",
        text(&o)
    );
}
