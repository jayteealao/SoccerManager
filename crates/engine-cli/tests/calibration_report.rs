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
    cmd.env("SM_BAND_HISTORY", data.join("band-history.jsonl"));
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

    // A reference file from the invented fixture data; its side folder stays in the TempDir.
    let targets = tmp.path().join("gate/real-targets.json");
    let side = tmp.path().join("side");
    let built = bin(&data)
        .args(["real-targets", "build", "--data"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-targets"))
        .arg("--out")
        .arg(&targets)
        .arg("--side-dir")
        .arg(&side)
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
        .arg("--side-dir")
        .arg(&side)
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

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dest = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &dest);
        } else {
            std::fs::copy(e.path(), dest).unwrap();
        }
    }
}

#[test]
fn each_line_names_its_source_and_the_side_file_joins_when_there() {
    let tmp = TempDir::new().unwrap();
    let data = tmp.path().join("data");
    let run = tmp.path().join("run");
    let real = tmp.path().join("no-real-data");
    std::fs::create_dir_all(&real).unwrap();

    // The reference fixture with the open-data fixture laid over it, built into a reference
    // file and a side folder of its own.
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let source = tmp.path().join("source");
    copy_dir(&fixtures.join("real-targets"), &source);
    copy_dir(&fixtures.join("open-data"), &source);
    let targets = tmp.path().join("gate/real-targets.json");
    let side = tmp.path().join(".scratch/real-targets");
    let built = bin(&data)
        .args(["real-targets", "build", "--data"])
        .arg(&source)
        .arg("--out")
        .arg(&targets)
        .arg("--side-dir")
        .arg(&side)
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", text(&built));
    assert!(side.join("impect-targets.json").is_file());

    calibrate(&data, &run, &real);
    let before = std::fs::read(run.join("report.json")).unwrap();

    let with = |side_dir: &Path| {
        let o = bin(&data)
            .args(["calibrate", "report", "--run"])
            .arg(&run)
            .arg("--targets")
            .arg(&targets)
            .arg("--side-dir")
            .arg(side_dir)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", text(&o));
        text(&o)
    };

    // With the side file: each line names its source, the men's senior pool beside the
    // other labels, the per-season targets counted apart, and the side file's targets in.
    let out = with(&side);
    assert!(
        out.contains("StatsBomb open data, men's senior matches, 2 competition-seasons"),
        "{out}"
    );
    assert!(out.contains("sb.men.shots_on_target_share"), "{out}");
    assert!(
        out.contains("StatsBomb competition-season targets in calibration.json"),
        "{out}"
    );
    assert!(!out.contains("sb.2_27."), "{out}");
    assert!(
        out.contains("IMPECT open data, 1. Bundesliga 2023/24"),
        "{out}"
    );
    let file = side.join("impect-targets.json");
    let side_file: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let n = side_file["targets"].as_object().unwrap().len();
    assert!(
        out.contains(&format!(
            "IMPECT side file: read from {} ({n} targets)",
            file.display()
        )),
        "{out}"
    );
    let cal: Value =
        serde_json::from_str(&std::fs::read_to_string(run.join("calibration.json")).unwrap())
            .unwrap();
    assert_eq!(cal["side"]["targets"], n);
    let lines: Vec<&Value> = cal["bands"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|b| b["targets"].as_array().unwrap())
        .collect();
    assert!(lines.iter().all(|t| t["source_label"].is_string()));
    assert!(
        lines
            .iter()
            .any(|t| t["target"].as_str().unwrap().starts_with("imp."))
    );

    // An empty side folder: the report says the side targets are not shown.
    let empty = TempDir::new().unwrap();
    let out = with(empty.path());
    assert!(out.contains("IMPECT side file absent at "), "{out}");
    assert!(out.contains("IMPECT-derived targets not shown"), "{out}");
    assert!(!out.contains("IMPECT open data, 1. Bundesliga"), "{out}");

    // The run's report stays byte-equal.
    assert_eq!(std::fs::read(run.join("report.json")).unwrap(), before);
}
