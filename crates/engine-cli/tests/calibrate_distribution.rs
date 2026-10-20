//! The whole-distribution bands in `engine-cli calibrate`: goals and scorelines, passes per
//! team, shot distance, and possession chains in passes and in seconds each have a band in
//! the reported table, judged by its distance from the real sample with an upper bound and a
//! noise floor; the real column reads "whole sample". A build without the measures reports
//! the three bands read from the band record; a measures build reports all six. At an absent
//! real-data folder each band is not sure, with no value. The test runs in both builds:
//!
//! ```text
//! cargo test -p engine-cli --test calibrate_distribution
//! cargo test -p engine-cli --features measures --test calibrate_distribution
//! ```

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{RecordSchemas, bin, copy_tree, edited_content, record, temp};
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

fn gz(path: &Path, text: &str) {
    let file = std::fs::File::create(path).unwrap();
    let mut gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    gz.write_all(text.as_bytes()).unwrap();
    gz.finish().unwrap();
}

/// The fixture folder, copied to `data`, with two compressed StatsBomb-shaped event files
/// written now: synthetic shots and possessions, no figure from real matches.
fn fixture(data: &Path) -> PathBuf {
    let dir = data.join("real-data");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-data"),
        &dir,
    );
    gz(
        &dir.join("sb_events.csv.gz"),
        "mid,type,x,y\n\
         1,Shot,93,34\n1,Shot,88,40\n1,Pass,50,30\n\
         2,Shot,99,30\n2,Shot,75,20\n\
         3,Shot,90,50\n",
    );
    gz(
        &dir.join("sb2_events.csv.gz"),
        "mid,period,ts,type,team,poss,poss_team,dur\n\
         1,1,00:00:00.000,Pass,A,1,A,1.0\n\
         1,1,00:00:02.000,Pass,A,1,A,1.0\n\
         1,1,00:00:04.000,Carry,B,2,B,3.0\n\
         2,1,00:00:00.000,Pass,A,1,A,2.0\n\
         2,1,00:00:03.000,Pass,B,1,A,1.0\n\
         2,1,00:00:05.000,Pass,B,2,B,10.0\n\
         3,1,00:00:00.000,Pass,A,1,A,1.0\n",
    );
    dir
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
    assert_eq!(report["schema.version"], "9");
    report
}

fn reported<'a>(report: &'a Value, band: &str) -> Option<&'a Value> {
    report["calib.reported"]
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
fn each_whole_distribution_has_a_band_in_the_reported_table() {
    let data = temp("engine-cli-distribution", "table");
    let real = fixture(&data);
    let content = content(&data);

    let run = data.join("run");
    let out = calibrate(&data, &content, &run, &real);
    let text = stderr(&out);
    let r = report(&out, &run);
    assert!(
        text.contains("reported bands (they never change the verdict):"),
        "{text}"
    );
    for band in expected() {
        let row = reported(&r, band).unwrap_or_else(|| panic!("no reported row {band}: {r}"));
        let value = row["value"].as_f64().unwrap();
        let upper = row["upper"].as_f64().unwrap_or_else(|| panic!("{row}"));
        let floor = row["noise_floor"]
            .as_f64()
            .unwrap_or_else(|| panic!("{row}"));
        assert!(value >= 0.0 && upper >= 0.0 && floor >= 0.0, "{row}");
        assert_eq!(row["lo"], 0.0, "{row}");
        assert_eq!(row["distance"].as_f64(), Some(value), "{row}");
        assert!(
            row.get("real").is_none(),
            "a whole sample has no single value: {row}"
        );
        assert_eq!(row["judged"], "range", "{row}");
        assert!(
            row["report_only"]
                .as_str()
                .is_some_and(|w| w.contains("gates: false")),
            "{row}"
        );
        let note = row["note"].as_str().unwrap_or_default();
        assert!(
            note.contains("upper bound") || note.starts_with("margin below noise floor"),
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
    // A build without the measures leaves the three bands read from the rows out.
    for band in MEASURES {
        assert_eq!(
            reported(&r, band).is_some(),
            cfg!(feature = "measures"),
            "{band}"
        );
    }
    // The distribution bands never move the run's answer: the gating rows hold none.
    for v in r["calib.verdicts"].as_array().unwrap() {
        let band = v["band"].as_str().unwrap();
        assert!(!RECORD.contains(&band) && !MEASURES.contains(&band), "{v}");
    }

    // An absent folder: every distribution band is not sure with no value.
    let absent = data.join("absent");
    let run2 = data.join("run-absent");
    let out2 = calibrate(&data, &content, &run2, &absent);
    let text2 = stderr(&out2);
    let r2 = report(&out2, &run2);
    assert_eq!(r2["calib.pass"], r["calib.pass"]);
    for band in expected() {
        let row = reported(&r2, band).unwrap_or_else(|| panic!("no reported row {band}"));
        assert_eq!(row["word"], "not sure", "{row}");
        assert!(
            row["note"]
                .as_str()
                .is_some_and(|n| n.starts_with("no value: no real data here")),
            "{row}"
        );
        assert!(
            row.get("upper").is_none() && row.get("distance").is_none(),
            "{row}"
        );
        let l = line(&text2, band).unwrap_or_else(|| panic!("no table line {band}: {text2}"));
        assert!(l.contains("no real data here"), "{l}");
    }
    let _ = std::fs::remove_dir_all(&data);
}
