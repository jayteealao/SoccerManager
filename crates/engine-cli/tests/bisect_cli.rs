//! `engine-cli bisect` on prepared builds. Two builds that differ from one known tick are
//! reported at that tick with the differing state parts and both debug trace excerpts; two
//! equal builds give no difference, including the real engine against a copy of itself; a
//! build that crashes, lacks the command or the options, stops early, skips a tick, hangs,
//! or does not exist gives an incomplete result with its reason, never "no difference"; and
//! a version-3 file is refused before any build runs.
//!
//! The prepared builds are one small program, `tests/fakes/bisect_fake.rs`, compiled here
//! with rustc and copied under behaviour names. Every incomplete case has a complete twin
//! against twin control.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use std::env::consts::EXE_SUFFIX;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn v4_file() -> PathBuf {
    repo().join("viewer/tests/data/one-minute-v4.smfx")
}

/// The folder of this test run's prepared builds, with the fake compiled once.
fn fakes() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("engine-cli-bisect-fakes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fakes/bisect_fake.rs");
        let out = dir.join(format!("compiled{EXE_SUFFIX}"));
        let built = Command::new("rustc")
            .args(["--edition", "2021", "-O", "-o"])
            .arg(&out)
            .arg(&source)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .unwrap_or_else(|e| panic!("rustc cannot be started, and the fake needs it: {e}"));
        assert!(
            built.status.success(),
            "the fake does not compile: {}",
            String::from_utf8_lossy(&built.stderr)
        );
        dir
    })
}

/// A prepared build that acts as `name` says.
fn fake(name: &str) -> PathBuf {
    let dir = fakes();
    let path = dir.join(format!("{name}{EXE_SUFFIX}"));
    std::fs::copy(dir.join(format!("compiled{EXE_SUFFIX}")), &path).unwrap();
    path
}

fn bisect(fixture: &Path, a: &Path, b: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_LOG", "warn")
        .arg("bisect")
        .arg("--fixture")
        .arg(fixture)
        .arg("--a-binary")
        .arg(a)
        .arg("--b-binary")
        .arg(b)
        .args(extra)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}{}", stdout(out), stderr(out)))
}

#[test]
fn two_builds_that_differ_from_one_tick_are_reported_at_that_tick() {
    // Control: twin against twin is no difference.
    let (a, twin) = (fake("twin-a"), fake("twin-b"));
    let same = bisect(&v4_file(), &a, &twin, &["--json"]);
    assert_eq!(
        same.status.code(),
        Some(0),
        "{}{}",
        stdout(&same),
        stderr(&same)
    );
    assert_eq!(json(&same)["verdict"], "no difference");
    assert_eq!(json(&same)["ticks"], 3000);

    let b = fake("differs-at-2000");
    let out = bisect(&v4_file(), &a, &b, &["--json"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    let report = json(&out);
    assert_eq!(report["verdict"], "differs");
    assert_eq!(report["tick"], 2000);
    let fields = report["fields"].as_array().unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["ball.vel", "streams"]);
    assert_eq!(fields[0]["a"], serde_json::json!([1.25, -0.5, 0.0]));
    assert_eq!(fields[0]["b"], serde_json::json!([1.25 + 1e-9, -0.5, 0.0]));
    assert_eq!(fields[1]["a"]["stream 0x0000000000000009"], "200");
    assert_eq!(fields[1]["b"]["stream 0x0000000000000009"], "201");
    for side in ["a", "b"] {
        let trace = report["trace"][side].as_array().unwrap();
        assert_eq!(trace.len(), 1, "{side}");
        assert_eq!(trace[0]["t"], 2000);
        assert_eq!(report[side]["scheme"], 1);
    }
    assert_eq!(report["trace"]["a"][0]["value"], 0.25);
    assert_eq!(report["trace"]["b"][0]["value"], 0.75);
    assert_eq!(report["a"]["source"], "ready binary");

    let text = bisect(&v4_file(), &a, &b, &[]);
    assert_eq!(text.status.code(), Some(2));
    let text = stdout(&text);
    assert!(text.starts_with("bisect: differs at tick 2000"), "{text}");
    assert!(
        text.contains("  ball.vel\n    a: [1.25,-0.5,0.0]\n    b: [1.250000001,-0.5,0.0]"),
        "{text}"
    );
    assert!(
        text.contains("debug trace of a at tick 2000: 1 records"),
        "{text}"
    );
    assert!(
        text.contains("debug trace of b at tick 2000: 1 records"),
        "{text}"
    );
}

#[test]
fn the_real_engine_against_a_copy_of_itself_gives_no_difference() {
    let engine = PathBuf::from(env!("CARGO_BIN_EXE_engine-cli"));
    let copy = fakes().join(format!("engine-copy{EXE_SUFFIX}"));
    std::fs::copy(&engine, &copy).unwrap();
    let out = bisect(&v4_file(), &engine, &copy, &[]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{text}{}", stderr(&out));
    assert!(
        text.starts_with("bisect: no difference: the state is the same after each of 3000 ticks"),
        "{text}"
    );
    assert!(text.contains(", ready binary"), "{text}");
}

#[test]
fn a_build_that_fails_in_any_way_is_incomplete_and_never_no_difference() {
    let a = fake("twin-inc");
    let cases = [
        ("crash", "crashed"),
        ("nocommand", "lacks the resimulate command"),
        ("nooption", "lacks the state-digest options"),
        ("short", "ends early"),
        ("gap", "gap in the state digests"),
        ("hang", "timed out"),
    ];
    for (name, reason) in cases {
        let b = fake(name);
        let out = bisect(&v4_file(), &a, &b, &["--timeout", "2"]);
        let text = stdout(&out);
        assert_eq!(out.status.code(), Some(3), "{name}: {text}{}", stderr(&out));
        assert!(text.starts_with("bisect: incomplete"), "{name}: {text}");
        assert!(!text.contains("bisect: no difference"), "{name}: {text}");
        let line = text
            .lines()
            .find(|l| l.starts_with("b ("))
            .unwrap_or_else(|| panic!("{name}: no reason for b: {text}"));
        assert!(line.contains(reason), "{name}: {line}");
        assert!(
            !text.lines().any(|l| l.starts_with("a (")),
            "{name}: {text}"
        );
        let as_json = json(&bisect(&v4_file(), &a, &b, &["--timeout", "2", "--json"]));
        assert_eq!(as_json["verdict"], "incomplete", "{name}");
    }
    let missing = fakes().join(format!("no-such-build{EXE_SUFFIX}"));
    let out = bisect(&v4_file(), &a, &missing, &[]);
    assert_eq!(out.status.code(), Some(3));
    assert!(stdout(&out).contains("does not exist"), "{}", stdout(&out));
}

#[test]
fn a_version_three_file_is_refused_before_any_build_runs() {
    let a = fake("marker-a");
    let b = fake("marker-b");
    let v3 = repo().join("viewer/tests/data/one-minute.smfx");
    let out = bisect(&v3, &a, &b, &[]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains("holds no inputs: it is a version-3 replay file"),
        "{err}"
    );
    assert!(
        err.contains("bisect needs the inputs to re-simulate it"),
        "{err}"
    );
    for bin in [&a, &b] {
        let mut marker = bin.clone().into_os_string();
        marker.push(".started");
        assert!(
            !PathBuf::from(marker).exists(),
            "{} was started",
            bin.display()
        );
    }
    // Control: on a version-4 file the marker build is started.
    let out = bisect(&v4_file(), &a, &b, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    let mut marker = a.into_os_string();
    marker.push(".started");
    assert!(PathBuf::from(marker).exists());
}
