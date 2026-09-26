//! `engine-cli gate`: the special fixtures show what they exist for (extra time and a
//! shoot-out; an applied substitution and an applied tactics change), a broken golden file
//! exits 1 naming the fault, a second bootstrap is refused, and, in release, all 22 matches
//! of the committed golden file match.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn golden() -> PathBuf {
    repo().join("gate/golden.json")
}

/// A scratch folder for one test, emptied first.
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("engine-cli-gate-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs `engine-cli gate` on the shipped content with a data folder of its own, named after
/// `name`.
fn gate(name: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", temp(&format!("data-{name}")))
        .env("SM_LOG", "warn")
        .env("SM_CONTENT_DIR", repo().join("content"))
        .arg("gate")
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `true` when the committed golden file has no hash set for this machine; the caller
/// prints a note and skips its golden-file part.
fn no_hash_set(out: &Output) -> bool {
    let stderr = text(&out.stderr);
    if out.status.code() == Some(1) && stderr.contains("has no hash set for this machine") {
        println!("note: skipping: {stderr}");
        return true;
    }
    false
}

#[test]
fn the_special_fixtures_show_extra_time_a_shootout_and_applied_changes() {
    let golden = golden();
    let out = gate(
        "special",
        &[
            "--golden",
            golden.to_str().unwrap(),
            "--fixture",
            "change",
            "--fixture",
            "knockout",
            "--json",
        ],
    );
    if no_hash_set(&out) {
        return;
    }
    let stdout = text(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}{}", text(&out.stderr));
    let lines: Vec<Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    let change = &lines[0];
    assert_eq!(change["fixture"], "change");
    assert_eq!(change["verdict"], "match");
    assert!(
        change["substitutions_applied"].as_u64().unwrap() >= 1,
        "{change}"
    );
    assert!(
        change["tactics_changes_applied"].as_u64().unwrap() >= 1,
        "{change}"
    );
    let knockout = &lines[1];
    assert_eq!(knockout["fixture"], "knockout");
    assert_eq!(knockout["verdict"], "match");
    assert_eq!(knockout["extra_time"], true, "{knockout}");
    assert_eq!(knockout["shootout"], true, "{knockout}");
    assert_eq!(knockout["decided_by"], "shoot-out", "{knockout}");
}

#[test]
fn a_broken_golden_file_exits_one_naming_the_fault() {
    let dir = temp("broken");
    let path = dir.join("golden.json");
    std::fs::write(&path, "{ \"gate_schema\": 1,").unwrap();
    let out = gate(
        "broken",
        &["--golden", path.to_str().unwrap(), "--fixture", "seed-42"],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = text(&out.stderr);
    assert!(stderr.contains("golden file is malformed"), "{stderr}");
    assert!(text(&out.stdout).is_empty(), "no match is played");

    let out = gate(
        "absent",
        &["--golden", dir.join("absent.json").to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("cannot read it"));
}

#[test]
fn a_second_bootstrap_is_refused() {
    let dir = temp("bootstrap");
    let path = dir.join("golden.json");
    std::fs::write(&path, "{}").unwrap();
    let out = gate(
        "bootstrap",
        &["--bootstrap", "--golden", path.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = text(&out.stderr);
    assert!(stderr.contains("already exists"), "{stderr}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{}");

    let out = gate(
        "bootstrap-fixture",
        &["--bootstrap", "--fixture", "seed-42"],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("--bootstrap plays every fixture"));
}

#[test]
fn an_unknown_fixture_exits_one_naming_the_fixtures() {
    let golden = golden();
    let out = gate(
        "unknown",
        &["--golden", golden.to_str().unwrap(), "--fixture", "seed-5"],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = text(&out.stderr);
    assert!(
        stderr.contains("no gate fixture is named seed-5"),
        "{stderr}"
    );
    assert!(stderr.contains("seed-42, seed-1"), "{stderr}");
}

/// The full gate: 22 matches, each `match`, exit 0. Run it in release:
/// `cargo test --release -p engine-cli --test gate -- --ignored`.
#[test]
#[ignore = "plays all 22 gate matches; run in release"]
fn all_22_matches_match_the_committed_golden_file() {
    let golden = golden();
    let out = gate("full", &["--golden", golden.to_str().unwrap()]);
    if no_hash_set(&out) {
        return;
    }
    let stdout = text(&out.stdout);
    println!("{stdout}");
    assert_eq!(out.status.code(), Some(0), "{stdout}{}", text(&out.stderr));
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 22, "{stdout}");
    for line in lines {
        let verdict = line.split_whitespace().nth(1);
        assert_eq!(verdict, Some("match"), "{line}");
    }
}
