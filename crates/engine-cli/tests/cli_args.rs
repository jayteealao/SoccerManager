//! AC-f (engine-core): an invalid seed exits non-zero with a message naming the argument; a
//! valid seed exits zero. AC-f (data-schemas-generator): `generate` writes team files and
//! `simulate --team-a --team-b` on them completes with the engine-core tick count. Plus: a
//! missing content folder exits 1 naming every path tried.

use std::path::{Path, PathBuf};
use std::process::Command;

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
    assert!(
        stdout.contains("\"ticks.written\":270000"),
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
fn no_help_line_exceeds_eighty_columns() {
    for args in [
        vec!["--help"],
        vec!["simulate", "--help"],
        vec!["bench", "--help"],
        vec!["generate", "--help"],
    ] {
        let out = bin().args(&args).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            assert!(line.len() <= 80, "{args:?}: {} columns: {line}", line.len());
        }
    }
}
