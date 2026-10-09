//! `engine-cli calibrate` and the realism measures. A build without the `measures` feature
//! says so as a run starts, reports `calib.rows.measures` false, and judges the bands of the
//! band file only; a measures build prints no such line and writes the measures into the
//! rows. The test runs in both builds:
//!
//! ```text
//! cargo test -p engine-cli --test calibrate_measures
//! cargo test -p engine-cli --features measures --test calibrate_measures
//! ```

mod common;

use std::collections::BTreeSet;

use common::{RecordSchemas, bin, read_rows, record, temp};
use serde_json::Value;

const LINE: &str = "this build has no measures";

/// The band names of the shipped band file, all of them or only those that gate.
fn shipped_bands(gating: bool) -> BTreeSet<String> {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/realism-bands.json");
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    file["bands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| !gating || b["gates"] != false)
        .map(|b| b["band"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn the_build_says_whether_it_records_the_measures() {
    let data = temp("engine-cli-calibrate-measures", "line");
    let run = data.join("run");
    let out = bin(&data)
        .args([
            "calibrate",
            "--suite",
            "equal",
            "--matches",
            "2",
            "--minutes",
            "5",
            "--seed",
            "3",
            "--jobs",
            "1",
        ])
        .arg("--out")
        .arg(&run)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        matches!(out.status.code(), Some(0 | 2)),
        "exit {:?}; stderr: {stderr}",
        out.status.code()
    );
    let measured = cfg!(feature = "measures");
    assert_eq!(stderr.contains(LINE), !measured, "stderr: {stderr}");
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    RecordSchemas::load()
        .report(&report)
        .unwrap_or_else(|e| panic!("{e}\n{report}"));
    assert_eq!(report["calib.rows"]["measures"], measured);
    assert_eq!(report["calib.rows"]["format"], 2);

    // The bands judged are those of the band file: the 18 that gate, no other, beside the
    // run's own time budget; the 24 that only report are reported apart.
    assert_eq!(shipped_bands(false).len(), 18 + 24);
    let shipped = shipped_bands(true);
    assert_eq!(shipped.len(), 18);
    let judged: BTreeSet<String> = report["calib.bands"]
        .as_array()
        .unwrap()
        .iter()
        .chain(report["calib.verdicts"].as_array().into_iter().flatten())
        .map(|b| b["band"].as_str().unwrap().to_string())
        .filter(|b| b != "wall_ms")
        .collect();
    assert!(!judged.is_empty());
    assert!(judged.is_subset(&shipped), "{judged:?}");

    // The rows carry the measures in a measures build, zeros otherwise.
    let rows = read_rows(&run);
    assert_eq!(rows.len(), 2);
    for r in &rows {
        assert_eq!(r.get("measures.present").int(), u64::from(measured));
        let shots: u64 = (0..41)
            .map(|b| r.get(&format!("shot_bin.home.{b:02}")).int())
            .sum();
        let ticks_on = r.get("player.home.00.ticks_on").int();
        if measured {
            assert_eq!(shots, r.get("shots.home").int());
            assert!(ticks_on > 0, "the first home player played");
            assert!(r.get("chains.home").int() + r.get("chains.away").int() > 0);
            assert!(r.get("player.away.00.distance_m").float() > 0.0);
        } else {
            assert_eq!((shots, ticks_on), (0, 0));
        }
    }
    let _ = std::fs::remove_dir_all(&data);
}
