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
