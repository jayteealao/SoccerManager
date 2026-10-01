//! `engine-cli resume`: a match continues from its latest snapshot to the same full time, and
//! a damaged snapshot is refused by name.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("engine-cli-resume-{}-{name}", std::process::id()))
}

fn bin(data: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_DATA_DIR", data);
    cmd.env(
        "SM_CONTENT_DIR",
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
    );
    cmd
}

fn stats(out: &Output) -> serde_json::Value {
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).unwrap()
}

/// Simulates one whole match with snapshots on. Returns its statistics and the snapshot
/// path.
fn simulate(data: &Path) -> (serde_json::Value, PathBuf) {
    let ticks = data.join("match.ticks");
    let out = bin(data)
        .args(["simulate", "--seed", "42", "--ticks-out"])
        .arg(&ticks)
        .output()
        .unwrap();
    let stats = stats(&out);
    let match_id = stats["match.id"].as_str().unwrap().to_string();
    let snapshot = data.join("matches").join(match_id).join("snapshot.smsn");
    assert!(snapshot.is_file(), "no snapshot at {}", snapshot.display());
    assert!(stats["snapshot.writes"].as_u64().unwrap() > 0);
    (stats, snapshot)
}

#[test]
fn a_resumed_match_reaches_the_same_full_time() {
    let data = temp("same");
    let _ = std::fs::remove_dir_all(&data);
    let (whole, snapshot) = simulate(&data);
    let resumed_ticks = data.join("resumed.ticks");
    let out = bin(&data)
        .arg("resume")
        .arg("--snapshot")
        .arg(&snapshot)
        .arg("--ticks-out")
        .arg(&resumed_ticks)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let resumed = stats(&out);
    let _ = std::fs::remove_dir_all(&data);

    assert!(stderr.contains("match.resumed"), "stderr: {stderr}");
    assert_eq!(resumed["match.id"], whole["match.id"]);
    assert_eq!(resumed["validate.violations"], 0);
    // The score, the clock, the lineups, and the cards carry over: every count at full time
    // matches the uninterrupted match.
    for key in [
        "goals",
        "ticks.played",
        "added_time.s",
        "cards.yellow",
        "cards.red",
        "stats.fouls",
        "stats.offsides",
        "stats.corners",
        "stats.throw_ins",
        "stats.goal_kicks",
        "rules.stoppages",
        "possession.changes",
    ] {
        assert_eq!(resumed[key], whole[key], "{key}");
    }
    let played = whole["ticks.played"].as_u64().unwrap();
    let written = resumed["ticks.written"].as_u64().unwrap();
    assert!(written > 0 && written < played, "{written} of {played}");
}

#[test]
fn a_damaged_snapshot_is_refused_by_name_and_the_valid_one_resumes() {
    let data = temp("damaged");
    let _ = std::fs::remove_dir_all(&data);
    let (_, snapshot) = simulate(&data);
    let bytes = std::fs::read(&snapshot).unwrap();

    let mut flipped = bytes.clone();
    flipped[100] ^= 0xff;
    let truncated = bytes[..bytes.len() - 10].to_vec();
    let mut magic = bytes.clone();
    magic[0] = b'X';
    for (name, damaged, reason) in [
        ("flipped", flipped, "checksum mismatch"),
        ("truncated", truncated, "truncated"),
        ("magic", magic, "bad magic"),
    ] {
        let path = data.join(format!("{name}.smsn"));
        std::fs::write(&path, damaged).unwrap();
        let out = bin(&data)
            .arg("resume")
            .arg("--snapshot")
            .arg(&path)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{name}: {stderr}");
        assert!(stderr.contains("snapshot refused"), "{name}: {stderr}");
        assert!(stderr.contains(reason), "{name}: {stderr}");
    }

    let out = bin(&data)
        .arg("resume")
        .arg("--snapshot")
        .arg(&snapshot)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&data);
    assert_eq!(out.status.code(), Some(0));
}

/// The committed save stamped as written by release 0.1.0 (format 8, build 3ba8fed).
fn saved_0_1_0() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../e2e/match/fixtures/saved-0.1.0.smsn")
}

/// A file in the layout release 0.2.0-beta.1 wrote (format 7, a 64-byte header) whose
/// build hash is `build`. Its body is not a match: the resolver reads only the header.
fn old_save(dir: &Path, build: &str) -> PathBuf {
    use sha2::{Digest, Sha256};
    let mut bytes = vec![0u8; 64];
    bytes[0..4].copy_from_slice(b"SMSN");
    bytes[4..6].copy_from_slice(&7u16.to_le_bytes());
    bytes[8..8 + build.len()].copy_from_slice(build.as_bytes());
    bytes.extend_from_slice(&[0u8; 120]);
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    bytes.extend_from_slice(b"SMSE");
    bytes.extend_from_slice(&120u32.to_le_bytes());
    bytes.extend_from_slice(&digest);
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("old-{build}.smsn"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn refused(data: &Path, snapshot: &Path, previous: &Path) -> String {
    let out = bin(data)
        .arg("resume")
        .arg("--snapshot")
        .arg(snapshot)
        .arg("--previous")
        .arg(previous)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(out.stdout.is_empty(), "no match was played");
    stderr
}

#[test]
fn a_save_from_two_or_more_versions_back_is_refused_naming_its_version() {
    let data = temp("older");
    let stderr = refused(&data, &saved_0_1_0(), &data.join("no-previous"));
    let _ = std::fs::remove_dir_all(&data);
    assert!(
        stderr.contains(
            "error: snapshot refused: this match was saved by Touchline 0.1.0, two or more \
             versions back"
        ),
        "{stderr}"
    );
}

#[test]
fn a_previous_release_save_is_refused_when_its_engine_is_missing() {
    let data = temp("missing");
    let save = old_save(&data, "669f68b");
    let stderr = refused(&data, &save, &data.join("no-previous"));
    let unreleased = refused(
        &data,
        &old_save(&data, "1234567"),
        &data.join("no-previous"),
    );
    let _ = std::fs::remove_dir_all(&data);
    assert!(
        stderr.contains(
            "this match was saved by Touchline 0.2.0-beta.1, and the engine of 0.2.0-beta.1 is \
             missing from this copy of the game"
        ),
        "{stderr}"
    );
    assert!(
        unreleased.contains("saved by an unreleased build 1234567"),
        "{unreleased}"
    );
}
