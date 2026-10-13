//! `engine-cli primitives`: the monotonic harness judges a primitive from played matches. A
//! graded primitive planted to move its signature against its direction fails and names the
//! step; one planted to move with it passes; a planted choice primitive whose one choice lowers
//! its own statistic fails and names that choice. The ignored tests play the shipped steps and
//! one shipped fit at full size; verify runs them.

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{bin, edited_content, temp};
use serde_json::{Value, json};

/// The content folder in `data` with tempo's `patient` level set to `patience`. The planted
/// value is no fit, so tempo's fit records go: content refuses a level that differs from its
/// own fit.
fn tempo_patient(data: &Path, patience: f64) -> PathBuf {
    edited_content(data, "tactics.json", |t| {
        t["instructions"]["tempo"]["levels"][3]["value"] = json!(patience);
        let tempo = t["primitives"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p["name"] == "tempo")
            .unwrap();
        tempo["fits"] = json!([]);
    })
}

/// `engine-cli primitives <args>` on `content`.
fn primitives(data: &Path, content: &Path, args: &[&str]) -> Output {
    bin(data)
        .arg("--content-dir")
        .arg(content)
        .arg("primitives")
        .args(args)
        .output()
        .unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// A short paired run of one tempo step: strong enough planted values move passes per
/// possession far beyond the noise of a few short matches.
const SHORT: [&str; 6] = ["--pairs", "8", "--minutes", "30", "--jobs", "4"];

#[test]
fn a_graded_primitive_moving_against_its_direction_fails_and_names_the_step() {
    let data = temp("engine-cli-primitives", "against");
    let content = tempo_patient(&data, -2.0);
    let mut args = vec!["check", "--primitive", "tempo", "--step", "balanced"];
    args.extend(SHORT);
    let run = primitives(&data, &content, &args);
    assert_eq!(run.status.code(), Some(2), "{}", stderr(&run));
    assert!(
        stderr(&run).contains("tempo: balanced -> patient fails: passes_per_possession"),
        "{}",
        stderr(&run)
    );
    std::fs::remove_dir_all(&data).unwrap();
}

#[test]
fn a_graded_primitive_moving_with_its_direction_passes() {
    let data = temp("engine-cli-primitives", "with");
    let content = tempo_patient(&data, 2.0);
    let out = data.join("check.json");
    let mut args = vec!["check", "--primitive", "tempo", "--step", "balanced"];
    args.extend(SHORT);
    args.extend(["--out", out.to_str().unwrap()]);
    let run = primitives(&data, &content, &args);
    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    let result: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let step = &result["steps"][0];
    assert_eq!(step["step"], "balanced -> patient");
    assert_eq!(step["stats"][0]["stat"], "passes_per_possession");
    assert!(step["stats"][0]["difference"].as_f64().unwrap() > 0.0);
    std::fs::remove_dir_all(&data).unwrap();
}

#[test]
fn a_choice_that_lowers_its_own_statistic_fails_and_is_named() {
    let data = temp("engine-cli-primitives", "choice");
    let content = edited_content(&data, "tactics.json", |t| {
        t["primitives"].as_array_mut().unwrap().push(json!({
            "name": "planted_choice",
            "kind": "choice",
            "choices": [
                { "name": "neutral", "values": { "patience": 0.0 }, "stat": "passes_per_possession" },
                { "name": "keep", "values": { "patience": 2.0 }, "stat": "passes_per_possession" },
                { "name": "rush", "values": { "patience": -2.0 }, "stat": "passes_per_possession" }
            ],
            "default": "neutral",
            "inputs": [{ "input": "patience", "hook": "pass.recycle" }],
            "signature": [{ "stat": "passes_per_possession", "direction": "up" }],
            "data": { "none": "a planted test primitive" }
        }));
    });
    let mut args = vec!["check", "--primitive", "planted_choice"];
    args.extend(SHORT);
    let run = primitives(&data, &content, &args);
    assert_eq!(run.status.code(), Some(2), "{}", stderr(&run));
    let text = stderr(&run);
    assert!(
        text.contains("planted_choice: neutral -> rush fails"),
        "{text}"
    );
    assert!(!text.contains("neutral -> keep fails"), "{text}");
    std::fs::remove_dir_all(&data).unwrap();
}

/// The shipped tempo and mentality: every step one level up moves the primary statistic in its
/// direction by more than the paired noise of 200 pairs (verify runs it; about half an hour).
#[test]
#[ignore = "plays 3,200 full matches; verify runs it"]
fn every_shipped_tempo_and_mentality_step_moves_its_primary_statistic() {
    let data = temp("engine-cli-primitives", "shipped");
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    for primitive in ["tempo", "mentality"] {
        let run = primitives(
            &data,
            &content,
            &["check", "--primitive", primitive, "--pairs", "200"],
        );
        assert_eq!(run.status.code(), Some(0), "{primitive}: {}", stderr(&run));
    }
    std::fs::remove_dir_all(&data).unwrap();
}

/// One shipped level refitted: the search lands within the paired noise of the real ratio and
/// near the value the tactics file holds (verify runs it).
#[test]
#[ignore = "plays up to 2,600 full matches; verify runs it"]
fn the_fit_of_tempo_patient_lands_within_the_noise_of_its_real_ratio() {
    let data = temp("engine-cli-primitives", "fit");
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let out = data.join("fit.json");
    let run = primitives(
        &data,
        &content,
        &[
            "fit",
            "--primitive",
            "tempo",
            "--level",
            "patient",
            "--matches",
            "200",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    let fit: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let search = &fit["search"];
    assert_eq!(search["unreachable"], false);
    let (achieved, noise, target) = (
        search["achieved"].as_f64().unwrap(),
        search["noise"].as_f64().unwrap(),
        fit["target"].as_f64().unwrap(),
    );
    assert!((achieved - target).abs() <= noise, "{fit}");
    assert_eq!(fit["source"], "wyscout_2017_18");
    std::fs::remove_dir_all(&data).unwrap();
}
