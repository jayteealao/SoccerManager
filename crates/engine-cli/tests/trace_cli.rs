//! The debug options: `simulate --debug-trace` writes a JSON Lines trace whose header names
//! the trace version and whose draw lines match the count it prints; `gate --debug` refuses
//! the write modes before any file is read, and in compare mode prints one count line per
//! match with equal draw counts.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch folder for one test, emptied first.
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("engine-cli-trace-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cli(name: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", temp(&format!("data-{name}")))
        .env("SM_LOG", "warn")
        .env("SM_CONTENT_DIR", repo().join("content"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The first number after `word` in `line`.
fn number_after(line: &str, word: &str) -> u64 {
    let at = line.find(word).unwrap() + word.len();
    line[at..]
        .trim_start()
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn simulate_writes_a_debug_trace_with_a_header_and_every_draw() {
    let dir = temp("simulate");
    let ticks = dir.join("m.ticks");
    let trace = dir.join("m.trace.jsonl");
    let out = cli(
        "simulate",
        &[
            "simulate",
            "--seed",
            "42",
            "--minutes",
            "2",
            "--ticks-out",
            ticks.to_str().unwrap(),
            "--no-snapshot",
            "--debug-trace",
            trace.to_str().unwrap(),
        ],
    );
    let stderr = text(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    let body = std::fs::read_to_string(&trace).unwrap();
    let mut lines = body.lines();
    let header: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(header["trace_version"], 1);
    assert_eq!(header["seed"], 42);
    assert_eq!(header["scheme"], 1);
    assert!(header["engine"].is_string());
    assert!(header["maths"].as_str().unwrap().starts_with("libm "));
    let records: Vec<Value> = lines.map(|l| serde_json::from_str(l).unwrap()).collect();
    let draws = records.iter().filter(|r| r["k"] == "draw").count() as u64;
    assert!(draws > 0);
    let line = stderr
        .lines()
        .find(|l| l.starts_with("debug trace: "))
        .unwrap_or_else(|| panic!("no count line in {stderr}"));
    assert_eq!(number_after(line, ".jsonl,"), draws, "{line}");
    assert_eq!(number_after(line, "(registry"), draws, "{line}");
    // The first record is the opening kick-off, on tick 1.
    assert_eq!(records[0]["t"], 1);
    assert!(
        records
            .windows(2)
            .all(|w| w[0]["t"].as_u64() <= w[1]["t"].as_u64())
    );
}

#[test]
fn gate_debug_refuses_the_write_modes_and_leaves_the_golden_file() {
    let dir = temp("refuse");
    let copy = dir.join("golden.json");
    std::fs::copy(repo().join("gate/golden.json"), &copy).unwrap();
    let before = std::fs::read(&copy).unwrap();
    for mode in [vec!["--regenerate", "--reason", "x"], vec!["--bootstrap"]] {
        let mut args = vec!["gate", "--debug", "--golden", copy.to_str().unwrap()];
        args.extend(mode.iter().copied());
        let out = cli("refuse", &args);
        let stderr = text(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{mode:?}: {stderr}");
        assert!(
            stderr.contains("--debug plays the compare only"),
            "{mode:?}: {stderr}"
        );
        assert_eq!(std::fs::read(&copy).unwrap(), before, "{mode:?}");
    }
}

#[test]
fn gate_debug_prints_equal_draw_counts_and_keeps_the_verdict() {
    let golden = repo().join("gate/golden.json");
    let out = cli(
        "compare",
        &[
            "gate",
            "--debug",
            "--golden",
            golden.to_str().unwrap(),
            "--fixture",
            "seed-42",
        ],
    );
    let stdout = text(&out.stdout);
    let stderr = text(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stdout}{stderr}");
    assert!(
        stdout.starts_with("seed-42") && stdout.contains(" match "),
        "{stdout}"
    );
    let line = stderr
        .lines()
        .find(|l| l.starts_with("trace: seed-42 "))
        .unwrap_or_else(|| panic!("no trace line in {stderr}"));
    let recorded = number_after(line, "trace: seed-42");
    let registry = number_after(line, "registry");
    assert!(recorded > 100_000, "{line}");
    assert_eq!(recorded, registry, "{line}");
    assert!(stderr.contains("none differ"), "{stderr}");
}
