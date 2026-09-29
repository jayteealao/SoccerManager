//! A bad slot file refuses `simulate` at start-up: for an unknown module, an unbuilt
//! version, an empty module name, and a required slot switched off, the program exits 1 and
//! names the slot, the bad value, and the valid names.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Runs `simulate` on a copy of the shipped content whose slot file is `fixture`, and
/// returns the exit code and stderr.
fn simulate_with(fixture: &str) -> (Option<i32>, String) {
    let root = std::env::temp_dir().join(format!(
        "engine-cli-slots-{}-{}",
        std::process::id(),
        fixture.trim_end_matches(".json")
    ));
    let _ = std::fs::remove_dir_all(&root);
    let content = root.join("content");
    copy_tree(&repo().join("content"), &content);
    std::fs::copy(
        repo()
            .join("crates/engine/tests/fixtures/slots")
            .join(fixture),
        content.join("slots.json"),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", root.join("data"))
        .env_remove("SM_CONTENT_DIR")
        .current_dir(&root)
        .arg("--content-dir")
        .arg(&content)
        .args([
            "simulate",
            "--seed",
            "1",
            "--minutes",
            "1",
            "--no-snapshot",
            "--ticks-out",
        ])
        .arg(root.join("match.ticks"))
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&root);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn assert_refused(fixture: &str, slot: &str, value: &str, valid: &str) {
    let (code, stderr) = simulate_with(fixture);
    assert_eq!(code, Some(1), "{fixture}: stderr: {stderr}");
    let line = stderr
        .lines()
        .find(|l| l.starts_with("error: slot configuration refused"))
        .unwrap_or_else(|| panic!("{fixture}: no refusal line in stderr: {stderr}"));
    assert!(line.contains(&format!("slot {slot}:")), "{line}");
    assert!(line.contains(value), "{line}");
    // A test build that unifies the engine's test features also lists the test-only
    // modules, so the valid list is matched by its first and last names.
    let listed = line.rsplit_once("; valid: ").map_or("", |(_, v)| v);
    let (first, last) = valid.split_once(" .. ").unwrap_or((valid, valid));
    assert!(
        listed.starts_with(first) && listed.ends_with(last),
        "{line}"
    );
    assert!(!stderr.contains("match.finished"), "{stderr}");
}

#[test]
fn an_unknown_module_refuses_start_up() {
    assert_refused(
        "unknown-module.json",
        "engine.offside",
        "module \"ofside\" is not registered",
        "offside@1 .. off",
    );
}

#[test]
fn an_unbuilt_version_refuses_start_up() {
    assert_refused(
        "unbuilt-version.json",
        "engine.fouls",
        "fouls version 2 is not built",
        "fouls@1 .. off",
    );
}

#[test]
fn an_empty_module_refuses_start_up() {
    assert_refused(
        "empty-slot.json",
        "engine.fouls",
        "the module name \"\" is empty",
        "fouls@1 .. off",
    );
}

#[test]
fn a_clock_switched_off_refuses_start_up() {
    // A test build that unifies the engine's test features lists `clock-faulty@1` after
    // `clock@1`, so only the first name is pinned.
    assert_refused(
        "clock-off.json",
        "engine.clock",
        "off is not allowed: the slot is required",
        "clock@1 .. ",
    );
}

#[test]
fn restarts_switched_off_refuses_start_up() {
    assert_refused(
        "restarts-off.json",
        "engine.restarts",
        "off is not allowed: the slot is required",
        "restarts@1",
    );
}

#[test]
fn a_required_slot_switched_off_refuses_start_up() {
    assert_refused(
        "steering-off.json",
        "engine.steering",
        "off is not allowed: the slot is required",
        "steering@1",
    );
}

#[test]
fn ball_switched_off_refuses_start_up() {
    assert_refused(
        "ball-off.json",
        "engine.ball",
        "off is not allowed: the slot is required",
        "ball@1",
    );
}

#[test]
fn possession_switched_off_refuses_start_up() {
    // A test build that unifies the engine's test features lists `possession-faulty@1`
    // after `possession@1`, so only the first name is pinned.
    assert_refused(
        "possession-off.json",
        "engine.possession",
        "off is not allowed: the slot is required",
        "possession@1 .. ",
    );
}

#[test]
fn decision_switched_off_refuses_start_up() {
    assert_refused(
        "decision-off.json",
        "engine.decision",
        "off is not allowed: the slot is required",
        "decision@1",
    );
}
