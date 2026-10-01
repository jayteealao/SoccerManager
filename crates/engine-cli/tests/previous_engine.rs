//! A match saved by the previous release finishes on that release's engine, and its result
//! equals the result the previous build reaches without the stop.
//!
//! These tests need the previous release's program, built from its tag by
//! `packaging/windows/previous-engine.ps1` or `packaging/unix/previous-engine.sh`. Set
//! `SM_PREVIOUS_ENGINE_PATH` to it and run with `--ignored`:
//!
//! ```text
//! SM_PREVIOUS_ENGINE_PATH=<cache>/previous/engine-cli cargo test -p engine-cli --test previous_engine -- --ignored
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use engine::{Snapshot, read_ticks};

/// The full-time keys `resume.rs` compares: the score, the clock, the cards, and the laws.
const KEYS: [&str; 12] = [
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
];

fn previous() -> (PathBuf, PathBuf) {
    let program = std::env::var_os("SM_PREVIOUS_ENGINE_PATH")
        .map(PathBuf::from)
        .expect("SM_PREVIOUS_ENGINE_PATH names the previous release's engine-cli");
    assert!(program.is_file(), "no program at {}", program.display());
    let content = program.parent().unwrap().join("content");
    assert!(
        content.is_dir(),
        "no content folder at {}",
        content.display()
    );
    (program, content)
}

fn temp(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("engine-cli-previous-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn stats(out: &std::process::Output) -> serde_json::Value {
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).unwrap()
}

/// One whole match played by the previous program, with its snapshot and tick file.
struct Whole {
    stats: serde_json::Value,
    snapshot: PathBuf,
    ticks: PathBuf,
}

fn play_whole(program: &Path, content: &Path, data: &Path, seed: u64) -> Whole {
    let ticks = data.join(format!("whole-{seed}.ticks"));
    let out = Command::new(program)
        .env("SM_DATA_DIR", data)
        .arg("--content-dir")
        .arg(content)
        .args(["simulate", "--seed", &seed.to_string(), "--ticks-out"])
        .arg(&ticks)
        .output()
        .unwrap();
    let stats = stats(&out);
    let id = stats["match.id"].as_str().unwrap();
    let snapshot = data.join("matches").join(id).join("snapshot.smsn");
    Whole {
        stats,
        snapshot,
        ticks,
    }
}

fn differences(a: &serde_json::Value, b: &serde_json::Value) -> Vec<&'static str> {
    KEYS.into_iter().filter(|key| a[key] != b[key]).collect()
}

#[test]
#[ignore = "needs SM_PREVIOUS_ENGINE_PATH, the previous release's engine built from its tag"]
fn a_previous_release_save_finishes_on_its_engine_with_the_same_result() {
    let (program, content) = previous();
    let data = temp("finish");
    let whole = play_whole(&program, &content, &data, 42);

    // The save is the previous release's own: format 7, its released build.
    let bytes = std::fs::read(&whole.snapshot).unwrap();
    let id = Snapshot::identify(&bytes).unwrap();
    assert_eq!(id.format, 7);
    assert_eq!(id.engine_version.as_deref(), Some("0.2.0-beta.1"));
    let from = id.tick.unwrap();

    // The current release resumes it: `engine-cli resume` with no content folder of the
    // previous release named, so the previous program's own is used.
    let resumed_ticks = data.join("resumed.ticks");
    let out = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", &data)
        .env(
            "SM_CONTENT_DIR",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        )
        .arg("resume")
        .arg("--snapshot")
        .arg(&whole.snapshot)
        .arg("--previous")
        .arg(&program)
        .arg("--ticks-out")
        .arg(&resumed_ticks)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let resumed = stats(&out);
    assert!(stderr.contains("resume.delegated"), "{stderr}");
    assert!(stderr.contains("0.2.0-beta.1"), "{stderr}");
    assert_eq!(
        resumed["build.hash"], "669f68b",
        "the previous program played"
    );
    assert_eq!(resumed["match.id"], whole.stats["match.id"]);
    assert_eq!(resumed["validate.violations"], 0);
    assert_eq!(differences(&resumed, &whole.stats), Vec::<&str>::new());

    // Every tick after the save, byte for byte.
    let whole_records = read_ticks(&whole.ticks).unwrap().records;
    let resumed_records = read_ticks(&resumed_ticks).unwrap().records;
    let tail: Vec<_> = whole_records.iter().filter(|r| r.tick > from).collect();
    assert!(!resumed_records.is_empty());
    assert_eq!(resumed_records.len(), tail.len());
    for (a, b) in resumed_records.iter().zip(tail) {
        assert!(a == b, "tick {} differs", a.tick);
    }

    // The control: the same comparison against another seed's match finds differences, so
    // the comparison above can fail.
    let other = play_whole(&program, &content, &data, 43);
    assert!(
        !differences(&resumed, &other.stats).is_empty(),
        "seed 43 reached the same full time as seed 42"
    );
    let _ = std::fs::remove_dir_all(&data);
}
