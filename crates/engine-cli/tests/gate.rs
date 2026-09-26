//! `engine-cli gate`: the special fixtures show what they exist for (extra time and a
//! shoot-out; an applied substitution and an applied tactics change), a broken golden file
//! exits 1 naming the fault, a second bootstrap is refused, a regeneration with no reason
//! exits 1 and leaves the golden file byte-identical (AC-8), the write modes refuse a fixture,
//! a second mode, and an existing set, and, in release, all 22 matches of the committed golden
//! file match and a regeneration rewrites a copy with one new ledger entry.

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
    // The watchdog mark: the gate plays with the real clock, so a busy machine may mark the
    // match; the key is always there, `invalid` names the reason only when marked, and the
    // verdict above still follows the hashes.
    let slow = knockout["slow_calls"]
        .as_u64()
        .unwrap_or_else(|| panic!("slow_calls is a count: {knockout}"));
    if slow == 0 {
        assert!(knockout["invalid"].is_null(), "{knockout}");
    } else {
        assert_eq!(knockout["invalid"], "slow script", "{knockout}");
    }
    assert_eq!(change["slow_calls"], 0, "the change fixture has no pack");
    assert!(change["invalid"].is_null(), "{change}");
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

fn sha256(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).unwrap();
    Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A temporary copy of the committed golden file.
fn golden_copy(name: &str) -> PathBuf {
    let path = temp(name).join("golden.json");
    std::fs::copy(golden(), &path).unwrap();
    path
}

#[test]
fn a_regeneration_with_no_reason_exits_one_and_leaves_the_file_byte_identical() {
    let path = golden_copy("no-reason");
    let before = sha256(&path);
    let golden_arg = path.to_str().unwrap();
    for (label, extra) in [
        ("absent", &[][..]),
        ("empty", &["--reason", ""][..]),
        ("blank", &["--reason", "   "][..]),
    ] {
        let mut args = vec!["--regenerate", "--golden", golden_arg];
        args.extend_from_slice(extra);
        let out = gate(&format!("no-reason-{label}"), &args);
        assert_eq!(out.status.code(), Some(1), "{label}");
        let stderr = text(&out.stderr);
        assert!(
            stderr.contains("--regenerate needs --reason \"<why the hashes change>\""),
            "{label}: {stderr}"
        );
        assert!(text(&out.stdout).is_empty(), "{label}: no match is played");
        assert_eq!(sha256(&path), before, "{label}: the file is byte-identical");
    }
    assert!(
        !path.with_file_name("golden.json.tmp").exists(),
        "no temporary file is left"
    );

    let out = gate(
        "no-reason-add",
        &["--add-machine-set", "--golden", golden_arg, "--reason", " "],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("--add-machine-set needs --reason"));
    assert_eq!(sha256(&path), before);
}

#[test]
fn the_write_modes_refuse_a_fixture_a_second_mode_and_an_existing_set() {
    let path = golden_copy("refusals");
    let before = sha256(&path);
    let golden_arg = path.to_str().unwrap();
    let cases: [(&str, &[&str], &str); 4] = [
        (
            "fixture",
            &["--regenerate", "--reason", "r", "--fixture", "seed-42"],
            "--regenerate plays every fixture; leave out --fixture",
        ),
        (
            "two-modes",
            &["--regenerate", "--add-machine-set", "--reason", "r"],
            "--regenerate and --add-machine-set cannot be used together",
        ),
        (
            "reason-alone",
            &["--reason", "r"],
            "--reason goes with --regenerate, --add-machine-set, or --bootstrap",
        ),
        (
            "bootstrap-and-regenerate",
            &["--bootstrap", "--regenerate", "--reason", "r"],
            "--bootstrap and --regenerate cannot be used together",
        ),
    ];
    for (label, extra, message) in cases {
        let mut args = vec!["--golden", golden_arg];
        args.extend_from_slice(extra);
        let out = gate(&format!("refuse-{label}"), &args);
        assert_eq!(out.status.code(), Some(1), "{label}");
        let stderr = text(&out.stderr);
        assert!(stderr.contains(message), "{label}: {stderr}");
        assert_eq!(sha256(&path), before, "{label}");
    }

    // A machine that already has a set is refused before any match is played.
    let machine = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    if file["hash_sets"].get(&machine).is_none() {
        println!("note: skipping the existing-set refusal: no {machine} set is committed");
        return;
    }
    let out = gate(
        "refuse-existing",
        &[
            "--add-machine-set",
            "--golden",
            golden_arg,
            "--reason",
            "again",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = text(&out.stderr);
    assert!(
        stderr.contains("golden file already has a hash set for this machine"),
        "{stderr}"
    );
    assert!(text(&out.stdout).is_empty());
    assert_eq!(sha256(&path), before);
}

/// A regeneration end to end on a temporary copy: 22 matches, one new entry, the set of this
/// machine only. Run it in release:
/// `cargo test --release -p engine-cli --test gate -- --ignored`.
#[test]
#[ignore = "plays all 22 gate matches; run in release"]
fn a_regeneration_rewrites_a_copy_with_one_new_entry() {
    let path = golden_copy("regenerate");
    let before: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let old_entries = before["ledger"].as_array().map_or(1, Vec::len);
    let out = gate(
        "regenerate",
        &[
            "--regenerate",
            "--golden",
            path.to_str().unwrap(),
            "--reason",
            "end-to-end test of the regeneration",
        ],
    );
    let stderr = text(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("regenerated 22 matches"), "{stderr}");
    let after: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let ledger = after["ledger"].as_array().unwrap();
    assert_eq!(ledger.len(), old_entries + 1);
    let entry = ledger.last().unwrap();
    assert_eq!(entry["kind"], "regenerate");
    assert_eq!(entry["reason"], "end-to-end test of the regeneration");
    assert_eq!(
        entry["band_result"],
        format!("gate/bands/ledger-{old_entries}.json")
    );
    assert_eq!(after["hash_sets"].as_object().unwrap().len(), 1);
    // The regenerated copy passes the gate.
    let out = gate("regenerate-check", &["--golden", path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
}
