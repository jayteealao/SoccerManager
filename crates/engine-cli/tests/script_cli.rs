//! `--script-pack` on the command line: the pack identity reaches the event stream and the
//! match statistics, a script's print never reaches standard output, a bad pack is refused
//! naming the field, and a snapshot of a scripted match resumes only with the same pack.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("engine-cli-script-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
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

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../script/tests/fixtures")
        .join(name)
}

fn sample() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/scripts/sample")
}

fn stats(out: &Output) -> serde_json::Value {
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout holds one record: {stdout}");
    serde_json::from_str(lines[0]).unwrap()
}

fn simulate(data: &Path, minutes: &str, pack: &Path) -> Output {
    bin(data)
        .args([
            "simulate",
            "--seed",
            "42",
            "--minutes",
            minutes,
            "--ticks-out",
        ])
        .arg(data.join("match.ticks"))
        .arg("--script-pack")
        .arg(pack)
        .output()
        .unwrap()
}

#[test]
fn the_pack_identity_reaches_the_first_kick_off_and_the_match_statistics() {
    let data = temp("identity");
    let out = simulate(&data, "10", &fixture("shoot-bias"));
    let stats = stats(&out);
    let pack = stats["script.pack"].as_str().unwrap().to_string();
    assert!(pack.starts_with("shoot-bias@1.0.0+"), "{pack}");
    assert!(stats["script.calls"].as_u64().unwrap() > 0, "{stats}");
    assert_eq!(stats["script.aborts"], 0);
    let match_id = stats["match.id"].as_str().unwrap();
    let dir = data.join("matches").join(match_id);
    let saved: serde_json::Value = serde_json::from_str(
        std::fs::read_to_string(dir.join("stats.json"))
            .unwrap()
            .trim(),
    )
    .unwrap();
    assert_eq!(saved["script.pack"], pack.as_str());
    let events = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
    let rows: Vec<serde_json::Value> = events
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let first_kick_off = rows.iter().find(|r| r["event.type"] == "kick-off").unwrap();
    assert_eq!(first_kick_off["script.pack"], pack.as_str());
    assert_eq!(
        rows.iter()
            .filter(|r| r.get("script.pack").is_some())
            .count(),
        1,
        "only the first kick-off names the pack"
    );
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_denied_hook_is_a_script_event_in_the_stream() {
    let data = temp("denied");
    let out = simulate(&data, "5", &fixture("network-call"));
    let stats = stats(&out);
    assert_eq!(stats["script.denials"], 3);
    assert_eq!(stats["script.disabled"], 1);
    let dir = data
        .join("matches")
        .join(stats["match.id"].as_str().unwrap());
    let events = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
    let script: Vec<serde_json::Value> = events
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .filter(|r| r["event.type"] == "script")
        .collect();
    assert_eq!(script.len(), 4, "{script:?}");
    assert_eq!(script[0]["script.hook"], "decision");
    assert_eq!(script[0]["script.outcome"], "denied");
    assert_eq!(
        script[0]["script.detail"],
        "function http_get is not available"
    );
    assert_eq!(script[3]["script.outcome"], "disabled");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_script_that_prints_leaves_standard_output_to_the_record() {
    let data = temp("print");
    let pack = data.join("printer");
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::copy(
        fixture("shoot-bias").join("pack.json"),
        pack.join("pack.json"),
    )
    .unwrap();
    std::fs::write(
        pack.join("main.rhai"),
        r#"fn decide(ctx) { print("hello from the script"); #{} }"#,
    )
    .unwrap();
    let out = simulate(&data, "1", &pack);
    let stats = stats(&out);
    assert!(stats["script.calls"].as_u64().unwrap() > 0);
    assert!(!String::from_utf8_lossy(&out.stdout).contains("hello from the script"));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_bad_pack_is_refused_naming_the_field() {
    let data = temp("bad");
    let pack = data.join("bad");
    std::fs::create_dir_all(&pack).unwrap();
    let manifest = std::fs::read_to_string(fixture("shoot-bias").join("pack.json"))
        .unwrap()
        .replace(r#""plugin_api": 1"#, r#""plugin_api": 2"#);
    std::fs::write(pack.join("pack.json"), manifest).unwrap();
    std::fs::copy(
        fixture("shoot-bias").join("main.rhai"),
        pack.join("main.rhai"),
    )
    .unwrap();
    let out = simulate(&data, "1", &pack);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(stderr.contains("plugin_api"), "{stderr}");
    assert!(stderr.contains("pack.json"), "{stderr}");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_scripted_snapshot_resumes_only_with_the_same_pack() {
    let data = temp("resume");
    let out = simulate(&data, "10", &sample());
    let whole = stats(&out);
    let snapshot = data
        .join("matches")
        .join(whole["match.id"].as_str().unwrap())
        .join("snapshot.smsn");
    assert!(snapshot.is_file());
    let resume = |pack: Option<PathBuf>| {
        let mut cmd = bin(&data);
        cmd.arg("resume").arg("--snapshot").arg(&snapshot);
        if let Some(pack) = pack {
            cmd.arg("--script-pack").arg(pack);
        }
        cmd.output().unwrap()
    };
    for other in [Some(fixture("shoot-bias")), None] {
        let out = resume(other.clone());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{other:?}: {stderr}");
        assert!(stderr.contains("content"), "{other:?}: {stderr}");
    }
    let resumed = stats(&resume(Some(sample())));
    assert_eq!(resumed["script.pack"], whole["script.pack"]);
    let _ = std::fs::remove_dir_all(&data);
}
