//! The run folder of `engine-cli calibrate`: the same command into the same folder grows a
//! run to a larger `--matches` without touching its earlier matches, resumes a stopped run
//! to the result an uninterrupted run gives, and starts again, naming why, when the run's
//! identity changed. Every run here is short, so the file stays in the default run.

mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Output;

use common::{RecordSchemas, bin, edited_content, read_rows, record, temp};
use serde_json::{Value, json};

/// A calibrate run into `run` with `args` after the subcommand, on `content` when given.
fn calibrate(data: &Path, content: Option<&Path>, run: &Path, args: &[&str]) -> Output {
    let mut cmd = bin(data);
    if let Some(content) = content {
        cmd.arg("--content-dir").arg(content);
    }
    cmd.arg("calibrate")
        .args(["--jobs", "2"])
        .args(args)
        .arg("--out")
        .arg(run)
        .output()
        .unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The saved report of a run that finished; either band verdict is fine for short matches.
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

/// Every ledger entry of the run folder: `(key, seed, match id)`, in file order.
fn ledger(run: &Path) -> Vec<(String, u64, String)> {
    let mut files: Vec<_> = std::fs::read_dir(run.join("ledger"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        for line in std::fs::read_to_string(&file).unwrap().lines() {
            let unit: Value = serde_json::from_str(line).unwrap();
            for f in unit["fixtures"].as_array().unwrap() {
                out.push((
                    f["key"].as_str().unwrap().to_string(),
                    f["seed"].as_u64().unwrap(),
                    f["match"].as_str().unwrap().to_string(),
                ));
            }
        }
    }
    out
}

/// Every compact row of the run folder by fixture key, with its wall time and record
/// reasons removed: the match's result.
fn rows(run: &Path) -> BTreeMap<String, String> {
    read_rows(run)
        .into_iter()
        .map(|r| {
            let mut columns = r.columns.clone();
            columns.remove("duration_us");
            columns.remove("record.reasons");
            (r.key(), format!("{columns:?}"))
        })
        .collect()
}

#[test]
fn a_bigger_run_in_the_same_folder_keeps_every_earlier_match_and_plays_only_the_new_ones() {
    let data = temp("engine-cli-resume", "grow");
    let run = data.join("run");
    let args = ["--seed", "3", "--minutes", "3", "--suite", "all"];
    let small = finished(
        &calibrate(
            &data,
            None,
            &run,
            &[&args[..], &["--matches", "2"]].concat(),
        ),
        &run,
    );
    let units = &small["calib.units"];
    // Equal and strength 2 each, and 55 formation pairings of 2.
    assert_eq!(
        units,
        &json!({"total": 114, "finished_before": 0, "played": 114})
    );
    let before_ledger = ledger(&run);
    let before_rows = rows(&run);
    assert_eq!(before_ledger.len(), 114);
    assert_eq!(before_rows.len(), 114);

    let out = calibrate(
        &data,
        None,
        &run,
        &[&args[..], &["--matches", "4"]].concat(),
    );
    let big = finished(&out, &run);
    assert_eq!(
        big["calib.units"],
        json!({"total": 228, "finished_before": 114, "played": 114})
    );
    assert!(stderr(&out).contains("resuming run"), "{}", stderr(&out));
    assert_eq!(big["run.id"], small["run.id"]);
    // The first run's keys, seeds, and results are untouched, and nothing played twice.
    let after_ledger = ledger(&run);
    assert_eq!(after_ledger.len(), 228);
    for entry in &before_ledger {
        assert!(after_ledger.contains(entry), "{entry:?} changed");
    }
    let after_rows = rows(&run);
    assert_eq!(after_rows.len(), 228);
    for (key, row) in &before_rows {
        assert_eq!(after_rows.get(key), Some(row), "{key} changed");
    }
    assert_eq!(big["fixtures.scheme"], "fixture-key-1");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_stopped_run_resumes_to_the_result_of_an_uninterrupted_run_with_no_match_twice() {
    let data = temp("engine-cli-resume", "stop");
    let args = [
        "--seed",
        "8",
        "--matches",
        "20",
        "--minutes",
        "3",
        "--suite",
        "equal",
    ];
    let whole_dir = data.join("whole");
    let whole = finished(&calibrate(&data, None, &whole_dir, &args), &whole_dir);

    let run = data.join("run");
    let stopped = calibrate(
        &data,
        None,
        &run,
        &[&args[..], &["--stop-after-units", "1"]].concat(),
    );
    assert_eq!(stopped.status.code(), Some(1), "{}", stderr(&stopped));
    assert!(stderr(&stopped).contains("stopped after 1 work units"));
    assert!(!run.join("report.json").exists());
    // One unit of 8 finished; half the next unit played but left no rows block and no
    // ledger line.
    assert_eq!(ledger(&run).len(), 8);
    assert_eq!(rows(&run).len(), 8);

    let out = calibrate(&data, None, &run, &args);
    let resumed = finished(&out, &run);
    assert!(
        stderr(&out).contains("resuming run") && stderr(&out).contains("8 of 20 fixtures done"),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        resumed["calib.units"],
        json!({"total": 20, "finished_before": 8, "played": 12})
    );
    assert_eq!(
        resumed["calib.results_digest"],
        whole["calib.results_digest"]
    );
    let entries = ledger(&run);
    let mut keys: Vec<&String> = entries.iter().map(|(k, _, _)| k).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 20, "a key twice or one missing");
    assert_eq!(entries.len(), 20);
    assert_eq!(rows(&run).len(), 20);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_folder_of_another_identity_starts_a_new_run_and_names_what_differs() {
    let data = temp("engine-cli-resume", "identity");
    let run = data.join("run");
    let args = [
        "--seed",
        "4",
        "--matches",
        "2",
        "--minutes",
        "3",
        "--suite",
        "equal",
    ];
    let first = finished(&calibrate(&data, None, &run, &args), &run);
    let old_id = first["run.id"].as_str().unwrap().to_string();

    // A changed tuning value: the content differs.
    let content = edited_content(&data, "tuning.json", |t| {
        let v = t["engine"]["red_base"].as_f64().unwrap();
        t["engine"]["red_base"] = json!(v + 0.001);
    });
    let out = calibrate(&data, Some(&content), &run, &args);
    let second = finished(&out, &run);
    let text = stderr(&out);
    assert!(
        text.contains("another identity") && text.contains("content "),
        "{text}"
    );
    assert!(!text.contains("resuming"), "{text}");
    assert_eq!(second["calib.units"]["finished_before"], 0);
    assert_ne!(second["run.id"], first["run.id"]);
    let moved = run.join("superseded").join(&old_id);
    assert!(moved.join("run.json").is_file());
    assert!(moved.join("report.json").is_file());
    assert_eq!(read_rows(&moved).len(), 2);

    // Other minutes.
    let out = calibrate(
        &data,
        Some(&content),
        &run,
        &[&args[..4], &["--minutes", "4"], &args[6..]].concat(),
    );
    finished(&out, &run);
    assert!(stderr(&out).contains("minutes 3 -> 4"), "{}", stderr(&out));

    // Another build: run.json names another executable.
    let path = run.join("run.json");
    let mut file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    file["identity"]["build"]["executable_sha256"] = json!("0".repeat(64));
    std::fs::write(&path, file.to_string()).unwrap();
    let out = calibrate(
        &data,
        Some(&content),
        &run,
        &[&args[..4], &["--minutes", "4"], &args[6..]].concat(),
    );
    let last = finished(&out, &run);
    assert!(stderr(&out).contains("build "), "{}", stderr(&out));
    assert_eq!(last["calib.units"]["finished_before"], 0);
    let _ = std::fs::remove_dir_all(&data);
}
