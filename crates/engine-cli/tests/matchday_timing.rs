//! A whole matchday on four cores keeps ahead of the player's match: with the process limited
//! to four logical cores (three background threads) and the player's match played flat out,
//! every other ground's event is computed before the moment the player's clock reaches it at
//! the fastest playback speed. A control at an impossible speed shows the check can fail.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The cores the reference run is limited to.
const CORES: usize = 4;

fn content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "engine-cli-matchday-timing-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn timing(dir: &Path, speed: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", dir)
        .env("SM_CONTENT_DIR", content())
        .env("SM_LOG", "info")
        .args(["matchday-timing", "--seed", "42", "--cores", "4"])
        .args(["--speed", speed, "--out"])
        .arg(dir.join("timing.json"))
        .output()
        .unwrap()
}

fn enough_cores() -> bool {
    let n = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    if n < CORES {
        eprintln!("skipped: this machine has {n} logical cores, the timing run needs {CORES}");
        return false;
    }
    true
}

#[test]
fn a_full_matchday_on_four_cores_computes_every_event_before_its_reveal() {
    if !enough_cores() {
        return;
    }
    let dir = temp("eight");
    let out = timing(&dir, "8");
    let stderr = String::from_utf8_lossy(&out.stderr);
    println!("{}", String::from_utf8_lossy(&out.stdout));
    for line in stderr.lines().filter(|l| l.contains("matchday.")) {
        println!("{line}");
    }
    assert_eq!(out.status.code(), Some(0), "{stderr}");

    let record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("timing.json")).unwrap()).unwrap();
    assert_eq!(record["cores"], 4);
    assert_eq!(record["threads"], 3);
    assert_eq!(record["late_events"], 0);
    let fixtures = record["fixtures"].as_array().unwrap();
    assert_eq!(fixtures.len(), 4);
    for fixture in fixtures {
        assert_eq!(fixture["outcome"], "success", "{fixture}");
        assert!(fixture["duration_ms"].as_u64().unwrap() > 0, "{fixture}");
    }
    let events = record["events"].as_array().unwrap();
    assert!(
        events.iter().filter(|e| e["kind"] == "full-time").count() == 4,
        "every fixture reaches full time"
    );
    for event in events {
        let computed = event["computed_at_ms"].as_f64().unwrap();
        let reveal = event["reveal_at_ms"].as_f64().unwrap();
        assert!(computed <= reveal, "computed after its reveal: {event}");
    }
    // The signal records each match's completion time.
    let done: Vec<&str> = stderr
        .lines()
        .filter(|l| l.contains("signal=\"matchday.match_done\""))
        .collect();
    assert_eq!(done.len(), 4, "{stderr}");
    assert!(done.iter().all(|l| l.contains("duration_ms=")));
    assert!(stderr.contains("signal=\"matchday.summary\""));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_impossible_speed_reports_late_events() {
    if !enough_cores() {
        return;
    }
    let dir = temp("control");
    let out = timing(&dir, "100000");
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("timing.json")).unwrap()).unwrap();
    assert!(record["late_events"].as_u64().unwrap() > 0);
    let _ = std::fs::remove_dir_all(&dir);
}
