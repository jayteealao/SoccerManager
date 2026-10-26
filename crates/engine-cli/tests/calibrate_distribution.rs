//! The whole-distribution bands in `engine-cli calibrate`: goals and scorelines, passes per
//! team, shot distance, and possession chains in passes and in seconds each have a band in
//! the gating table, judged by its distance from the real sample with an upper bound and a
//! noise floor; the real column reads "whole sample". A build without the measures judges
//! the three bands read from the band record and reads the other three not sure; a measures
//! build judges all six. At an absent real-data folder each band is not sure, with no value.
//! The test runs in both builds:
//!
//! ```text
//! cargo test -p engine-cli --test calibrate_distribution
//! cargo test -p engine-cli --features measures --test calibrate_distribution
//! ```

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{RecordSchemas, bin, copy_tree, edited_content, hold_floor, record};
use serde_json::{Value, json};

/// The bands read from the band record, in every build.
const RECORD: [&str; 3] = [
    "goals_distribution",
    "scoreline_cells",
    "passes_per_team_distribution",
];

/// The bands read from a measures build's rows.
const MEASURES: [&str; 3] = [
    "shot_distance_distribution",
    "chain_passes_distribution",
    "chain_seconds_distribution",
];

/// The fixture folder, copied to `data`.
fn fixture(data: &Path) -> PathBuf {
    let dir = data.join("real-data");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-data"),
        &dir,
    );
    dir
}

/// The StatsBomb distribution targets the three measures bands name, as fixtures: the
/// committed reference file goes to `data/gate/` beside the content folder with each
/// `sb.men.dist.*` target replaced by one over two invented competition-seasons of two
/// matches each, its bins and width the band's, its shares pooled from the per-match
/// histograms written to the side folder `data/real-targets/`. No figure comes from real
/// matches.
fn statsbomb_targets(data: &Path) {
    let committed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/real-targets.json");
    let mut file: Value =
        serde_json::from_str(&std::fs::read_to_string(committed).unwrap()).unwrap();
    let labels = ["fx_1", "fx_2"];
    let mut rows = String::from("label,match,kind,bin,count\n");
    for kind in ["shot_distance", "chain_passes", "chain_seconds"] {
        let target = &mut file["targets"][format!("sb.men.dist.{kind}")];
        let bins = target["bins"].as_u64().unwrap();
        let mut pooled = vec![0.0; bins as usize];
        for m in 1..=4u64 {
            let label = labels[usize::from(m > 2)];
            for (bin, count) in [((m * 3) % bins, m), ((m * 7 + 1) % bins, 1)] {
                rows.push_str(&format!("{label},{m},{kind},{bin},{count}\n"));
                pooled[bin as usize] += count as f64;
            }
        }
        let total: f64 = pooled.iter().sum();
        target["shares"] = json!(pooled.iter().map(|c| c / total).collect::<Vec<f64>>());
        target["n"] = json!(4);
        target["seasons"] = json!(labels);
        target["leagues"] = json!(labels);
        target["source_label"] = json!("fixture, two invented competition-seasons");
    }
    let gate = data.join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    std::fs::write(
        gate.join("real-targets.json"),
        serde_json::to_string_pretty(&file).unwrap(),
    )
    .unwrap();
    let side = data.join("real-targets");
    std::fs::create_dir_all(&side).unwrap();
    std::fs::write(side.join("statsbomb-match-histograms.csv"), rows).unwrap();
}

/// The shipped content, its five-league filters widened to the fixture's competitions A and
/// B: the shipped pointers otherwise read the fixture files as they are.
fn content(data: &Path) -> PathBuf {
    edited_content(data, "realism-bands.json", |doc| {
        for b in doc["bands"].as_array_mut().unwrap() {
            if RECORD.contains(&b["band"].as_str().unwrap()) {
                let source = &mut b["real"]["source"];
                if source.get("where").is_some() {
                    source["where"] = json!({ "comp": ["A", "B"] });
                }
            }
        }
    })
}

fn calibrate(data: &Path, content: &Path, out: &Path, real: &Path) -> Output {
    let run = || {
        bin(data)
            .arg("--content-dir")
            .arg(content)
            .arg("calibrate")
            .args(["--seed", "4", "--suite", "equal", "--minutes", "5"])
            .args(["--jobs", "4", "--matches", "6"])
            .arg("--real-data")
            .arg(real)
            .arg("--out")
            .arg(out)
            .output()
            .unwrap()
    };
    // The fixture's small real samples put some margins under their floor: widen them.
    let mut out = run();
    for _ in 0..3 {
        if !hold_floor(content, &String::from_utf8_lossy(&out.stderr)) {
            break;
        }
        out = run();
    }
    out
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
    assert_eq!(report["schema.version"], "12");
    report
}

fn gating<'a>(report: &'a Value, band: &str) -> Option<&'a Value> {
    report["calib.verdicts"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["band"] == band))
}

/// The console line of `band` in a table.
fn line<'a>(text: &'a str, band: &str) -> Option<&'a str> {
    text.lines().find(|l| l.starts_with(&format!("{band} ")))
}

/// The bands this build reports: the three read from the band record, and in a measures
/// build the three read from the rows.
fn expected() -> Vec<&'static str> {
    let mut out = RECORD.to_vec();
    if cfg!(feature = "measures") {
        out.extend(MEASURES);
    }
    out
}

#[test]
fn each_whole_distribution_has_a_band_that_gates() {
    let tmp = tempfile::TempDir::new().unwrap();
    let data = tmp.path().to_path_buf();
    let real = fixture(&data);
    let content = content(&data);
    statsbomb_targets(&data);

    let run = data.join("run");
    let out = calibrate(&data, &content, &run, &real);
    let text = stderr(&out);
    let r = report(&out, &run);
    assert!(
        text.contains("reported bands (they never change the verdict):"),
        "{text}"
    );
    for band in expected() {
        let row = gating(&r, band).unwrap_or_else(|| panic!("no gating row {band}: {r}"));
        let value = row["value"].as_f64().unwrap();
        let upper = row["upper"].as_f64().unwrap_or_else(|| panic!("{row}"));
        let floor = row["noise_floor"]
            .as_f64()
            .unwrap_or_else(|| panic!("{row}"));
        assert!(value >= 0.0 && upper >= 0.0 && floor >= 0.0, "{row}");
        assert_eq!(row["distance"].as_f64(), Some(value), "{row}");
        assert!(
            row.get("real").is_none(),
            "a whole sample has no single value: {row}"
        );
        assert!(
            row.get("report_only").is_none(),
            "a distribution band gates: {row}"
        );
        let note = row["note"].as_str().unwrap_or_default();
        assert!(
            note.contains("upper bound") || note.contains("margin below noise floor"),
            "{row}"
        );
        let l = line(&text, band).unwrap_or_else(|| panic!("no table line {band}: {text}"));
        assert!(l.contains("whole sample"), "{l}");
        let e = r["calib.real"]["bands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["band"] == band)
            .unwrap();
        assert_eq!(e["status"], "read", "{e}");
    }
    // A build without the measures still holds the three bands read from the rows, not sure
    // for want of the measures.
    if !cfg!(feature = "measures") {
        for band in MEASURES {
            let row = gating(&r, band).unwrap_or_else(|| panic!("no gating row {band}"));
            assert_eq!(row["word"], "not sure", "{row}");
            assert!(
                row["note"]
                    .as_str()
                    .is_some_and(|n| n.contains("this build has no measures")),
                "{row}"
            );
        }
    }
    // The distribution bands gate: none of them sits in the reported rows.
    for v in r["calib.reported"].as_array().unwrap() {
        let band = v["band"].as_str().unwrap();
        assert!(!RECORD.contains(&band) && !MEASURES.contains(&band), "{v}");
    }

    // An absent folder with no side folder beside it: every distribution band is not sure
    // with no value.
    let absent = data.join("elsewhere").join("absent");
    let run2 = data.join("run-absent");
    let out2 = calibrate(&data, &content, &run2, &absent);
    let text2 = stderr(&out2);
    let r2 = report(&out2, &run2);
    assert_eq!(r2["calib.pass"], r["calib.pass"]);
    for band in expected() {
        let row = gating(&r2, band).unwrap_or_else(|| panic!("no gating row {band}"));
        assert_eq!(row["word"], "not sure", "{row}");
        assert!(
            row["note"]
                .as_str()
                .is_some_and(|n| n.contains("no real data here")),
            "{row}"
        );
        assert!(
            row.get("upper").is_none() && row.get("distance").is_none(),
            "{row}"
        );
        let l = line(&text2, band).unwrap_or_else(|| panic!("no table line {band}: {text2}"));
        assert!(l.contains("no real data here"), "{l}");
    }
}
