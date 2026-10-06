//! The old engine of a change run: its results on the run's fixtures come from a cache
//! keyed by build, content, and fixture, so a second run against the same old engine plays
//! none of its matches; and an old engine that cannot be built stops the run before any
//! match plays, naming the build error.

mod common;

use std::path::Path;
use std::process::Output;

use common::{RecordSchemas, bin, record, temp};
use serde_json::Value;

fn calibrate(data: &Path, run: &Path, args: &[&str]) -> Output {
    bin(data)
        .arg("calibrate")
        .args(["--seed", "9", "--matches", "2", "--minutes", "3"])
        .args(["--suite", "equal", "--jobs", "2"])
        .args(args)
        .arg("--out")
        .arg(run)
        .output()
        .unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn finished(out: &Output, run: &Path) -> Value {
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

#[test]
fn a_second_change_run_takes_every_old_engine_result_from_the_cache() {
    let data = temp("engine-cli-base", "cache");
    let old = env!("CARGO_BIN_EXE_engine-cli");
    let first_dir = data.join("first");
    let out = calibrate(&data, &first_dir, &["--base-binary", old]);
    let first = finished(&out, &first_dir);
    assert!(
        stderr(&out).contains(": 0 results from the cache, 2 played"),
        "{}",
        stderr(&out)
    );
    let base = &first["calib.base"];
    assert_eq!(base["source"], "binary");
    assert_eq!(base["cache"]["hits"], 0);
    assert_eq!(base["cache"]["played"], 2);
    // The same engine on the same fixtures: the old engine's rows equal the run's.
    let rows = |bands: &Value| -> Vec<Value> {
        bands
            .as_array()
            .unwrap()
            .iter()
            .filter(|b| b["band"] != "wall_ms")
            .cloned()
            .collect()
    };
    assert_eq!(rows(&base["calib.bands"]), rows(&first["calib.bands"]));

    let second_dir = data.join("second");
    let out = calibrate(&data, &second_dir, &["--base-binary", old]);
    let second = finished(&out, &second_dir);
    assert!(
        stderr(&out).contains(": 2 results from the cache, 0 played"),
        "{}",
        stderr(&out)
    );
    assert_eq!(second["calib.base"]["cache"]["hits"], 2);
    assert_eq!(second["calib.base"]["cache"]["played"], 0);
    assert_eq!(second["calib.base"]["calib.bands"], base["calib.bands"]);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn an_old_engine_that_cannot_build_stops_the_run_before_any_match() {
    let data = temp("engine-cli-base", "broken");
    let run = data.join("run");
    let out = calibrate(
        &data,
        &run,
        &["--base", "HEAD", "--inject-failure", "base-build"],
    );
    let text = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(
        text.contains("error[E0425]: injected base build failure"),
        "{text}"
    );
    assert!(text.contains("no match was played"), "{text}");
    assert!(!run.join("stats").exists(), "a match was played");
    assert!(!run.join("run.json").exists(), "the run folder was started");
    let _ = std::fs::remove_dir_all(&data);
}

/// The real path: build HEAD through the bisect build cache and play it. Slow (a release
/// build on a cold cache), so it runs by hand: `cargo test -p engine-cli --release --test
/// calibrate_base -- --ignored`.
#[test]
#[ignore = "builds the old engine with cargo; run by hand"]
fn the_old_engine_of_a_revision_is_built_once_and_cached() {
    let data = temp("engine-cli-base", "rev");
    let first_dir = data.join("first");
    let out = calibrate(&data, &first_dir, &["--base", "HEAD"]);
    let first = finished(&out, &first_dir);
    assert_eq!(first["calib.base"]["source"], "rev");
    assert_eq!(first["calib.base"]["cache"]["played"], 2);
    let second_dir = data.join("second");
    let second = finished(
        &calibrate(&data, &second_dir, &["--base", "HEAD"]),
        &second_dir,
    );
    assert_eq!(second["calib.base"]["cache"]["hits"], 2);
    assert_eq!(second["calib.base"]["cache"]["played"], 0);
    let _ = std::fs::remove_dir_all(&data);
}
