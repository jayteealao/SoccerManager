//! `engine-cli fast-model`: a small fit runs end to end and records the golden results' id;
//! a stale fit, or a golden file whose results changed, fails and names both ids, with the
//! unchanged fit as the control; the shipped fit matches the golden results; and no runtime
//! path outside the fit and check commands resolves the fast-model slot (a source scan, with
//! a planted call as its control).

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::OnceLock;

use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run(data: &Path, args: &[&str]) -> Output {
    common::bin(data)
        .current_dir(repo())
        .args(args)
        .output()
        .unwrap()
}

/// The small fit, played once per test binary: its folder and its report.
fn small_fit() -> &'static (PathBuf, Value) {
    static FIT: OnceLock<(PathBuf, Value)> = OnceLock::new();
    FIT.get_or_init(|| {
        let dir = common::temp("fast-model", "small-fit");
        let out = dir.join("fit.json");
        let report = dir.join("report");
        let done = run(
            &dir,
            &[
                "fast-model",
                "fit",
                "--matches",
                "4",
                "--draws",
                "2",
                "--minutes",
                "10",
                "--jobs",
                "4",
                "--gate-fixture",
                "seed-1",
                "--out",
                out.to_str().unwrap(),
                "--report",
                report.to_str().unwrap(),
            ],
        );
        let stderr = String::from_utf8_lossy(&done.stderr);
        // Four matches a pairing is too few for the check to mean anything: it may pass or
        // fail (exit 0 or 2), and the mechanics are what this test holds.
        assert!(
            matches!(done.status.code(), Some(0 | 2)),
            "exit {:?}: {stderr}",
            done.status.code()
        );
        assert_eq!(out.exists(), done.status.code() == Some(0), "{stderr}");
        let text = std::fs::read_to_string(report.join("report.json")).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        // The fit the report carries, written as a fit file for the stale checks.
        std::fs::write(
            dir.join("from-report.json"),
            serde_json::to_string_pretty(&value["fit"]).unwrap(),
        )
        .unwrap();
        (dir, value)
    })
}

fn golden_ledger_index() -> usize {
    let text = std::fs::read_to_string(repo().join("gate/golden.json")).unwrap();
    let golden: Value = serde_json::from_str(&text).unwrap();
    golden["ledger"].as_array().unwrap().len() - 1
}

/// The fit confirms the engine, plays both batches, and reports 49
/// figures; the fit it reports records the golden results' id.
#[test]
fn a_small_fit_runs_and_records_the_golden_results_id() {
    let (_, report) = small_fit();
    assert_eq!(report["figures"].as_array().unwrap().len(), 49);
    let id = report["engine_id"].as_str().unwrap();
    assert!(
        id.starts_with(&format!("golden-{}-", golden_ledger_index())),
        "{id}"
    );
    assert_eq!(report["fit"]["engine_id"], id);
    assert_eq!(report["fit"]["model"], "fitted-scores@1");
    assert_eq!(report["fit"]["batch"]["matches_per_pairing"], 4);
    assert_eq!(report["fit"]["check"]["figures"], 49);
    let shares = report["fit"]["fit"]["minute_shares"].as_array().unwrap();
    assert_eq!(shares.len(), 90);
}

/// A fit whose engine id differs from the golden results' id fails and names both;
/// the unchanged fit passes.
#[test]
fn a_stale_fit_fails_and_names_both_ids() {
    let (dir, report) = small_fit();
    let good = dir.join("from-report.json");
    let current = report["engine_id"].as_str().unwrap();

    let control = run(
        dir,
        &["fast-model", "stale", "--fit", good.to_str().unwrap()],
    );
    assert_eq!(
        control.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&control.stderr)
    );
    assert!(String::from_utf8_lossy(&control.stdout).contains(current));

    let mut fit: Value = serde_json::from_str(&std::fs::read_to_string(&good).unwrap()).unwrap();
    let old = "golden-2-0000000-000000000000";
    fit["engine_id"] = Value::String(old.into());
    let stale = dir.join("stale.json");
    std::fs::write(&stale, serde_json::to_string_pretty(&fit).unwrap()).unwrap();
    let done = run(
        dir,
        &["fast-model", "stale", "--fit", stale.to_str().unwrap()],
    );
    let stderr = String::from_utf8_lossy(&done.stderr);
    assert_eq!(done.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("the fast-model fit is stale"), "{stderr}");
    assert!(stderr.contains(old), "{stderr}");
    assert!(stderr.contains(current), "{stderr}");
    assert!(stderr.contains("run engine-cli fast-model fit"), "{stderr}");
}

/// A golden file whose results changed (one final hash) makes the same fit stale.
#[test]
fn a_changed_golden_file_makes_the_fit_stale() {
    let (dir, report) = small_fit();
    let good = dir.join("from-report.json");
    let text = std::fs::read_to_string(repo().join("gate/golden.json")).unwrap();
    let mut golden: Value = serde_json::from_str(&text).unwrap();
    golden["hash_sets"]["portable"][0]["final_hash"] = Value::String("0".repeat(64));
    let copy = dir.join("golden-changed.json");
    std::fs::write(&copy, serde_json::to_string_pretty(&golden).unwrap()).unwrap();
    let done = run(
        dir,
        &[
            "fast-model",
            "stale",
            "--fit",
            good.to_str().unwrap(),
            "--golden",
            copy.to_str().unwrap(),
        ],
    );
    let stderr = String::from_utf8_lossy(&done.stderr);
    assert_eq!(done.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains(report["engine_id"].as_str().unwrap()),
        "{stderr}"
    );
}

/// The files that may name the fast model's module: the module itself, the registry
/// plumbing, and the fit and check commands.
const ALLOWED: &[&str] = &[
    "crates/engine/src/modules/fast_model.rs",
    "crates/engine/src/modules/registry.rs",
    "crates/engine/src/modules/config.rs",
    "crates/engine/src/modules/stand_in.rs",
    "crates/engine/src/modules/mod.rs",
    "crates/engine-cli/src/fast_model/",
];

/// What reaches the module.
const PATTERNS: &[&str] = &[
    "fast_model::resolve",
    "dyn FastModel",
    "dyn fast_model::FastModel",
    "ModuleRef::FastModel",
];

/// Every `path: pattern` in `files` (repository-relative path with `/`, and text) that
/// reaches the fast model outside the allowed files. Comment lines are skipped.
fn reaches(files: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in files {
        if ALLOWED
            .iter()
            .any(|a| path == a || (a.ends_with('/') && path.starts_with(a)))
        {
            continue;
        }
        for line in text.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for p in PATTERNS {
                if line.contains(p) {
                    found.push(format!("{path}: {p}"));
                }
            }
        }
    }
    found
}

fn rust_files(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, root, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, std::fs::read_to_string(&path).unwrap()));
        }
    }
}

/// No runtime path outside the fit and check commands resolves
/// the fast-model slot. The scan reads every crate's `src/`; a planted call in the serve
/// command's text proves it fails.
#[test]
fn no_runtime_path_resolves_the_fast_model() {
    let root = repo();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root.join("crates")).unwrap() {
        let src = entry.unwrap().path().join("src");
        if src.is_dir() {
            rust_files(&src, &root, &mut files);
        }
    }
    assert!(
        files
            .iter()
            .any(|(p, _)| p == "crates/engine-cli/src/serve.rs"),
        "the scan reads the serve command"
    );
    assert!(
        files
            .iter()
            .any(|(p, t)| p.starts_with("crates/engine-cli/src/fast_model/")
                && t.contains("fast_model::resolve")),
        "the fit and check commands are the callers"
    );
    let found = reaches(&files);
    assert!(found.is_empty(), "the fast model is reached from {found:?}");

    let mut planted = files.clone();
    let serve = planted
        .iter_mut()
        .find(|(p, _)| p == "crates/engine-cli/src/serve.rs")
        .unwrap();
    serve
        .1
        .push_str("\nfn fallback(c: &Content) { let _ = fast_model::resolve(&c.modules); }\n");
    assert_eq!(
        reaches(&planted),
        ["crates/engine-cli/src/serve.rs: fast_model::resolve"]
    );
}

/// In every test job: the fit the content folder ships records the engine id of the
/// committed golden results, so a change that regenerates the golden file without a refit
/// fails here as well as in the CI step.
#[test]
fn the_shipped_fit_records_the_golden_results_id() {
    let dir = common::temp("fast-model", "shipped");
    let done = run(&dir, &["fast-model", "stale"]);
    let stderr = String::from_utf8_lossy(&done.stderr);
    assert_eq!(done.status.code(), Some(0), "{stderr}");
    let stdout = String::from_utf8_lossy(&done.stdout);
    let fit: Value = serde_json::from_str(
        &std::fs::read_to_string(repo().join("content/fast-model.json")).unwrap(),
    )
    .unwrap();
    assert!(
        fit["check"]["pass"].as_bool().unwrap(),
        "the shipped fit passed its check"
    );
    assert_eq!(fit["check"]["figures"], 49);
    assert_eq!(fit["batch"]["matches_per_pairing"], 1000);
    assert_eq!(fit["batch"]["minutes"], 90);
    assert!(
        stdout.contains(fit["engine_id"].as_str().unwrap()),
        "{stdout}"
    );
}
