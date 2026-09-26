//! Checks the command-line argument handling: an invalid seed exits non-zero with a message
//! naming the argument, and a valid seed exits zero. `generate` writes team files, and
//! `simulate --team-a --team-b` on them completes with the tick count. Plus: a missing
//! content folder exits 1 naming every path tried; help stays within 80 columns; the JSON
//! dump path, the operating-system error text, and redirected stderr; and the one error
//! record a failed `simulate`, `bench`, or calibration match or worker writes.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{RecordSchemas, record};
use serde_json::Value;

fn bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_DATA_DIR", temp("data"));
    cmd.env("SM_CONTENT_DIR", content_dir());
    cmd
}

fn content_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("engine-cli-{}-{name}", std::process::id()))
}

#[test]
fn invalid_seed_exits_non_zero_and_names_the_argument() {
    let out = bin()
        .args(["simulate", "--seed", "abc", "--ticks-out", "unused.ticks"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--seed"),
        "stderr did not name --seed: {stderr}"
    );
}

#[test]
fn valid_seed_exits_zero() {
    let path = temp("valid.ticks");
    let out = bin()
        .args(["simulate", "--seed", "42", "--minutes", "1", "--ticks-out"])
        .arg(&path)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"record.kind\":\"match-stats\""),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("\"content.hash\":\""), "stdout: {stdout}");
    assert!(
        stdout.contains("\"team.id\":\"club-00000001-00\""),
        "stdout: {stdout}"
    );
}

#[test]
fn generated_teams_simulate_a_full_match_with_the_engine_core_tick_count() {
    let out_dir = temp("league");
    let _ = std::fs::remove_dir_all(&out_dir);
    let out = bin()
        .args(["generate", "--seed", "9", "--clubs", "2", "--out"])
        .arg(&out_dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "generate stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let a = out_dir.join("club-00000009-00.json");
    let b = out_dir.join("club-00000009-01.json");
    assert!(a.is_file() && b.is_file());

    let ticks = temp("league.ticks");
    let out = bin()
        .args(["simulate", "--seed", "42", "--team-a"])
        .arg(&a)
        .arg("--team-b")
        .arg(&b)
        .arg("--ticks-out")
        .arg(&ticks)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&ticks);
    let _ = std::fs::remove_dir_all(&out_dir);
    let _ = std::fs::remove_dir_all(temp("data"));
    assert!(
        out.status.success(),
        "simulate stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Ninety minutes of regulation play, then the time added to each half.
    let stats: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let added: u64 = stats["added_time.s"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_u64().unwrap())
        .sum();
    assert_eq!(
        stats["ticks.written"].as_u64(),
        Some(270_000 + added * 50),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("\"team.id\":\"club-00000009-00\""),
        "stdout: {stdout}"
    );
}

#[test]
fn a_missing_content_folder_exits_one_naming_the_paths_tried() {
    let missing = temp("no-content");
    let out = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", temp("data"))
        .env_remove("SM_CONTENT_DIR")
        .current_dir(std::env::temp_dir())
        .args(["--content-dir"])
        .arg(&missing)
        .args([
            "simulate",
            "--seed",
            "1",
            "--minutes",
            "1",
            "--ticks-out",
            "unused.ticks",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot read content folder")
            && stderr.contains(&missing.display().to_string()),
        "stderr: {stderr}"
    );
}

#[test]
fn an_explicit_content_dir_without_attributes_is_not_ignored_for_cwd_content() {
    let missing = temp("no-attributes-flag");
    let _ = std::fs::remove_dir_all(&missing);
    std::fs::create_dir_all(&missing).unwrap();
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", temp("data"))
        .env_remove("SM_CONTENT_DIR")
        .current_dir(&repo_root)
        .args(["--content-dir"])
        .arg(&missing)
        .args([
            "simulate",
            "--seed",
            "1",
            "--minutes",
            "1",
            "--ticks-out",
            "unused.ticks",
        ])
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&missing);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot read content folder")
            && stderr.contains(&missing.display().to_string()),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("content.loaded"), "stderr: {stderr}");
}

#[test]
fn no_help_line_exceeds_eighty_columns() {
    let surfaces = [
        None,
        Some("simulate"),
        Some("bench"),
        Some("generate"),
        Some("serve"),
        Some("launch"),
        Some("record"),
        Some("replay"),
        Some("resume"),
        Some("calibrate"),
        Some("gate"),
        Some("guard"),
    ];
    let mut checks = 0;
    for surface in surfaces {
        for flag in ["-h", "--help"] {
            let args: Vec<&str> = surface.into_iter().chain([flag]).collect();
            let out = bin().args(&args).output().unwrap();
            assert!(out.status.success(), "{args:?} exited {:?}", out.status);
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(stdout.contains("Usage:"), "{args:?}: {stdout}");
            for line in stdout.lines() {
                assert!(line.len() <= 80, "{args:?}: {} columns: {line}", line.len());
            }
            checks += 1;
        }
    }
    assert_eq!(checks, 24);
}

#[test]
fn json_dump_replaces_the_tick_file_extension() {
    let dir = temp("dump");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out = bin()
        .args([
            "simulate",
            "--seed",
            "7",
            "--minutes",
            "1",
            "--no-snapshot",
            "--json",
            "--ticks-out",
        ])
        .arg(dir.join("match.ticks"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let dump = std::fs::read_to_string(dir.join("match.jsonl"));
    let appended = dir.join("match.ticks.jsonl").exists();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!appended, "the dump path appended .jsonl");
    assert_eq!(dump.unwrap().lines().count(), 3000);
}

#[test]
fn an_unwritable_tick_path_names_the_os_error_once() {
    let dir = temp("unwritable");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out = bin()
        .args(["simulate", "--seed", "7", "--minutes", "1", "--no-snapshot"])
        .arg("--ticks-out")
        .arg(&dir)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&dir.display().to_string()),
        "stderr: {stderr}"
    );
    assert_eq!(stderr.matches("os error").count(), 1, "stderr: {stderr}");
}

#[test]
fn redirected_stderr_carries_no_ansi_escape() {
    let ticks = temp("no-ansi.ticks");
    let out = bin()
        .args(["simulate", "--seed", "7", "--minutes", "1", "--no-snapshot"])
        .arg("--ticks-out")
        .arg(&ticks)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&ticks);
    assert!(out.status.success());
    assert!(!out.stderr.is_empty(), "no log lines to check");
    assert!(
        !out.stderr.contains(&0x1b),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The one line on stdout of a failed run, and its stderr.
fn failed_run(args: &[&str], extra: Option<&Path>) -> (Value, String) {
    let data = common::temp("engine-cli-failure", &args.join("-").replace(' ', ""));
    let mut cmd = common::bin(&data);
    cmd.args(args);
    if let Some(path) = extra {
        cmd.arg(path);
    }
    let out = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    // Nothing is saved for a failed run: no match folder under the data folder.
    let saved = data.join("matches").exists();
    let _ = std::fs::remove_dir_all(&data);
    assert_eq!(out.status.code(), Some(1), "stderr: {stderr}");
    assert_eq!(stdout.lines().count(), 1, "stdout: {stdout}");
    assert!(stderr.contains("error: "), "stderr: {stderr}");
    assert!(!saved, "a failed run saved a match folder");
    (record(&stdout), stderr)
}

fn assert_error_keys(rec: &Value, error_type: &str) {
    assert_eq!(rec["outcome"], "error", "{rec}");
    assert_eq!(rec["error.type"], error_type, "{rec}");
    assert!(rec["error.code"].is_string(), "{rec}");
    assert!(rec["error.retriable"].is_boolean(), "{rec}");
}

#[test]
fn a_failed_simulate_prints_one_error_record() {
    let schemas = RecordSchemas::load();
    let (rec, stderr) = failed_run(
        &["simulate", "--seed", "7", "--minutes", "0", "--ticks-out"],
        Some(&temp("zero.ticks")),
    );
    assert!(stderr.contains("minutes must be"), "stderr: {stderr}");
    assert_eq!(rec["record.kind"], "match-stats");
    assert_eq!(rec["operation"], "simulate");
    assert_error_keys(&rec, "invalid-config");
    assert_eq!(rec["error.code"], "invalid-config");
    assert!(rec.get("stats.goals").is_none(), "{rec}");
    assert!(rec.get("ticks.written").is_none(), "{rec}");
    schemas.stats(&rec).unwrap_or_else(|e| panic!("{e}\n{rec}"));

    let dir = temp("ticks-dir");
    std::fs::create_dir_all(&dir).unwrap();
    let (rec, _) = failed_run(
        &[
            "simulate",
            "--seed",
            "7",
            "--minutes",
            "1",
            "--no-snapshot",
            "--ticks-out",
        ],
        Some(&dir),
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_error_keys(&rec, "io");
    schemas.stats(&rec).unwrap_or_else(|e| panic!("{e}\n{rec}"));
}

#[test]
fn a_failed_bench_prints_one_error_record() {
    let (rec, stderr) = failed_run(&["bench", "--seed", "7", "--matches", "0"], None);
    assert!(
        stderr.contains("invalid configuration: --matches must be at least 1"),
        "stderr: {stderr}"
    );
    assert_eq!(rec["record.kind"], "run-report");
    assert_eq!(rec["operation"], "benchmark");
    assert_error_keys(&rec, "invalid-config");
    assert!(rec.get("bench.match_wall_ms").is_none(), "{rec}");
    RecordSchemas::load()
        .report(&rec)
        .unwrap_or_else(|e| panic!("{e}\n{rec}"));
}

/// A two-match calibration run of the equal suite on one worker, with a failure injected.
fn calibrate_with(inject: &str) -> (std::process::Output, PathBuf, PathBuf) {
    let data = common::temp("engine-cli-inject", inject);
    let run = data.join("run");
    let out = common::bin(&data)
        .args([
            "calibrate",
            "--seed",
            "1",
            "--matches",
            "2",
            "--minutes",
            "1",
            "--jobs",
            "1",
            "--suite",
            "equal",
            "--keep-events",
            "all",
            "--inject-failure",
            inject,
            "--out",
        ])
        .arg(&run)
        .output()
        .unwrap();
    (out, data, run)
}

#[test]
fn a_failed_calibration_match_writes_an_error_record() {
    let schemas = RecordSchemas::load();
    let (out, data, run) = calibrate_with("match");
    let mut records = Vec::new();
    for entry in std::fs::read_dir(run.join("stats")).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(!text.contains("\"failure\""), "{text}");
        records.push(record(&text));
    }
    // The match that played still writes events that validate; the failed one writes none.
    let mut event_files = 0;
    for entry in std::fs::read_dir(run.join("events")).unwrap() {
        event_files += 1;
        for line in std::fs::read_to_string(entry.unwrap().path())
            .unwrap()
            .lines()
        {
            let row = record(line);
            schemas.event(&row).unwrap_or_else(|e| {
                panic!(
                    "{e}
{row}"
                )
            });
        }
    }
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    let _ = std::fs::remove_dir_all(&data);
    assert!(
        matches!(out.status.code(), Some(0 | 2)),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(records.len(), 2);
    assert_eq!(event_files, 1);
    for rec in &records {
        schemas.stats(rec).unwrap_or_else(|e| panic!("{e}\n{rec}"));
    }
    let errors: Vec<&Value> = records.iter().filter(|r| r["outcome"] == "error").collect();
    assert_eq!(errors.len(), 1);
    assert_error_keys(errors[0], "invalid-config");
    // One failed match is not a failed run: every worker finished.
    assert_eq!(report["outcome"], "success", "{report}");
    assert!(report.get("error.type").is_none(), "{report}");
    schemas
        .report(&report)
        .unwrap_or_else(|e| panic!("{e}\n{report}"));
}

#[test]
fn a_failed_calibration_worker_makes_the_report_an_error() {
    let (out, data, run) = calibrate_with("worker");
    let printed = record(&String::from_utf8_lossy(&out.stdout));
    let saved = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    let _ = std::fs::remove_dir_all(&data);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(printed, saved);
    assert_eq!(saved["outcome"], "error", "{saved}");
    assert_eq!(saved["error.type"], "worker");
    assert_eq!(saved["error.code"], "worker-failed");
    assert_eq!(saved["error.retriable"], false);
    RecordSchemas::load()
        .report(&saved)
        .unwrap_or_else(|e| panic!("{e}\n{saved}"));
}

#[test]
fn the_test_seams_stay_out_of_the_help() {
    for command in ["launch", "serve"] {
        let out = bin().args([command, "--help"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        for hidden in ["--drop-client-at", "--match-millis"] {
            assert!(!stdout.contains(hidden), "{command} --help shows {hidden}");
        }
    }
    for flag in ["-h", "--help"] {
        let out = bin().args(["calibrate", flag]).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        for hidden in [
            "--worker",
            "--shard",
            "--run-dir",
            "--run-millis",
            "--inject-failure",
            "--pairing-numbers",
        ] {
            assert!(!stdout.contains(hidden), "calibrate {flag} shows {hidden}");
        }
    }
    let out = bin().args(["launch", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    for shown in ["--engine", "--web", "--seed"] {
        assert!(stdout.contains(shown), "launch --help must show {shown}");
    }
}
