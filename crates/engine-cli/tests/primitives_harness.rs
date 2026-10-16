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

/// The content folder in `data` with tempo's `slow` level set to `patience`. The planted
/// value is no fit, so tempo's fit records go: content refuses a level that differs from its
/// own fit.
fn tempo_slow(data: &Path, patience: f64) -> PathBuf {
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

/// A short paired run of one tempo step: planted values move passes per possession beyond the
/// noise of 48 short matches. A patience of 0.5 raises it; above about 1 a side that waits that
/// long passes into receivers who are pressed on arrival and loses the ball sooner, so a
/// patience of 2 lowers it.
const SHORT: [&str; 6] = ["--pairs", "48", "--minutes", "30", "--jobs", "4"];

#[test]
fn a_graded_primitive_moving_against_its_direction_fails_and_names_the_step() {
    let data = temp("engine-cli-primitives", "against");
    let content = tempo_slow(&data, -2.0);
    let mut args = vec!["check", "--primitive", "tempo", "--step", "normal"];
    args.extend(SHORT);
    let run = primitives(&data, &content, &args);
    assert_eq!(run.status.code(), Some(2), "{}", stderr(&run));
    assert!(
        stderr(&run).contains("tempo: normal -> slow fails: passes_per_possession"),
        "{}",
        stderr(&run)
    );
    std::fs::remove_dir_all(&data).unwrap();
}

#[test]
fn a_graded_primitive_moving_with_its_direction_passes() {
    let data = temp("engine-cli-primitives", "with");
    let content = tempo_slow(&data, 0.5);
    let out = data.join("check.json");
    let mut args = vec!["check", "--primitive", "tempo", "--step", "normal"];
    args.extend(SHORT);
    args.extend(["--out", out.to_str().unwrap()]);
    let run = primitives(&data, &content, &args);
    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    let result: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let step = &result["steps"][0];
    assert_eq!(step["step"], "normal -> slow");
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
                { "name": "keep", "values": { "patience": 0.375 }, "stat": "passes_per_possession" },
                { "name": "rush", "values": { "patience": -2.0 }, "stat": "passes_per_possession" }
            ],
            "default": "neutral",
            "inputs": [{ "input": "patience", "hook": "pass.recycle" }],
            "signature": [{ "stat": "passes_per_possession", "direction": "up" }],
            "data": { "none": "a planted test primitive" }
        }));
    });
    // The weak keep choice needs twice the short run's pairs to clear its noise.
    let args = [
        "check",
        "--primitive",
        "planted_choice",
        "--pairs",
        "96",
        "--minutes",
        "30",
        "--jobs",
        "4",
    ];
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

/// The content folder in `data` with dribbling's `dribble_more` level planted at `dribble`;
/// the planted value is no fit, so dribbling's fit records go.
fn dribble_more(data: &Path, dribble: f64) -> PathBuf {
    edited_content(data, "tactics.json", |t| {
        let p = t["primitives"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p["name"] == "dribbling")
            .unwrap();
        p["steps"][3]["values"]["dribble"] = json!(dribble);
        p["fits"] = json!([]);
    })
}

/// The per-player check: a planted dribble value for the left winger (slot 5 of the 4-4-2)
/// raises his own take-ons beyond the paired noise while his ten team-mates' pooled take-ons
/// stay inside it.
#[test]
fn a_planted_player_instruction_moves_his_own_statistic_and_not_the_others() {
    let data = temp("engine-cli-primitives", "player-with");
    let content = dribble_more(&data, 0.5);
    let out = data.join("player.json");
    let mut args = vec!["check", "--primitive", "dribbling", "--player", "5"];
    args.extend(SHORT);
    args.extend(["--out", out.to_str().unwrap()]);
    let run = primitives(&data, &content, &args);
    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    let result: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(result["kind"], "primitives-check-player");
    let step = &result["steps"][0];
    assert_eq!(step["step"], "dribble_normally -> dribble_more");
    assert_eq!(step["slot"], 5);
    assert_eq!(step["own"]["stat"], "takeons_per_90");
    assert!(step["own"]["difference"].as_f64().unwrap() > 0.0);
    assert_eq!(step["others"]["pass"], true);
    std::fs::remove_dir_all(&data).unwrap();
}

/// A planted value against the instruction's direction fails and names the slot's statistic;
/// a forward-runs instruction for a slot off the attack line is refused by name.
#[test]
fn a_player_instruction_moving_against_its_direction_fails_and_one_off_the_line_is_refused() {
    let data = temp("engine-cli-primitives", "player-against");
    let content = dribble_more(&data, -0.5);
    let mut args = vec!["check", "--primitive", "dribbling", "--player", "5"];
    args.extend(SHORT);
    let run = primitives(&data, &content, &args);
    assert_eq!(run.status.code(), Some(2), "{}", stderr(&run));
    assert!(
        stderr(&run)
            .contains("dribbling: dribble_normally -> dribble_more fails: slot 5 takeons_per_90"),
        "{}",
        stderr(&run)
    );
    let run = primitives(
        &data,
        &content,
        &[
            "check",
            "--primitive",
            "forward_runs",
            "--player",
            "6",
            "--pairs",
            "2",
        ],
    );
    assert_eq!(run.status.code(), Some(1), "{}", stderr(&run));
    assert!(stderr(&run).contains("forward_runs"), "{}", stderr(&run));
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

/// `engine-cli primitives check` of a shipped primitive at 1,000 pairs, its JSON result. The
/// fitted steps of the real spread are small and the engine's runs are rare, so 200 pairs
/// cannot see every correct step.
fn shipped_check(data: &Path, primitive: &str) -> Value {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let out = data.join(format!("check-{primitive}.json"));
    let run = primitives(
        data,
        &content,
        &[
            "check",
            "--primitive",
            primitive,
            "--pairs",
            "1000",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert!(
        matches!(run.status.code(), Some(0 | 2)),
        "{primitive}: {}",
        stderr(&run)
    );
    serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap()
}

/// The step named `step` of a check result.
fn step<'a>(result: &'a Value, step: &str) -> &'a Value {
    result["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["step"] == step)
        .unwrap_or_else(|| panic!("no step {step}: {result}"))
}

/// The paired difference and noise of statistic `stat` in a step.
fn moved(step: &Value, stat: &str) -> (f64, f64) {
    let s = step["stats"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stat"] == stat)
        .unwrap_or_else(|| panic!("no {stat}: {step}"));
    (
        s["difference"].as_f64().unwrap(),
        s["noise"].as_f64().unwrap(),
    )
}

/// The shipped in-possession graded primitives that are fitted or judged: every step one level
/// up moves the primary statistic in its direction by more than the paired noise of 1,000
/// pairs (forward runs: the share of runs in behind above the default, of runs coming short
/// below it); build-up's steps toward going long also lengthen the goal kick and raise the
/// keeper's long share (the keeper follows build-up). Crossing is deferred and not judged
/// (verify runs it).
#[test]
#[ignore = "plays 64,000 full matches; verify runs it"]
fn every_in_possession_step_moves_its_primary_statistic() {
    let data = temp("engine-cli-primitives", "possession-steps");
    for primitive in [
        "width",
        "passing_directness",
        "build_up",
        "final_third",
        "dribbling",
        "creative_freedom",
        "forward_runs",
        "fullback_run_frequency",
    ] {
        let result = shipped_check(&data, primitive);
        for s in result["steps"].as_array().unwrap() {
            assert_eq!(s["pass"], true, "{primitive}: {s}");
        }
        if primitive == "build_up" {
            for s in result["steps"].as_array().unwrap() {
                for stat in ["goal_kick_m", "keeper_long_share"] {
                    let (d, _) = moved(s, stat);
                    assert!(d > 0.0, "build_up {}: {stat} {d}", s["step"]);
                }
            }
        }
    }
    std::fs::remove_dir_all(&data).unwrap();
}

/// The judged choices raise their own statistic above the default: the left, right and both
/// wings channels above balanced, and underlap and overlap above staying back. The channel
/// mirrors: left's rise in the left share and right's rise in the right share differ by no
/// more than their paired noise. The central channel choices are deferred and not judged
/// (verify runs it).
#[test]
#[ignore = "plays 16,000 full matches; verify runs it"]
fn the_judged_choices_raise_their_own_share_and_the_wings_mirror() {
    let data = temp("engine-cli-primitives", "possession-choices");
    let channel = shipped_check(&data, "attacking_channel");
    for s in [
        "balanced -> left",
        "balanced -> right",
        "balanced -> both_wings",
    ] {
        assert_eq!(step(&channel, s)["pass"], true, "{s}: {channel}");
    }
    let (left, nl) = moved(step(&channel, "balanced -> left"), "entries_left_share");
    let (right, nr) = moved(step(&channel, "balanced -> right"), "entries_right_share");
    assert!((left - right).abs() <= nl.hypot(nr), "{left} {right}");
    let fullback = shipped_check(&data, "fullback_runs");
    for s in fullback["steps"].as_array().unwrap() {
        assert_eq!(s["pass"], true, "{s}");
    }
    std::fs::remove_dir_all(&data).unwrap();
}

/// One quintile fit (passing directness `direct`) and one thin-sample fit (forward runs
/// `push_up`) refitted: each lands within the paired noise of its real ratio (verify runs it).
#[test]
#[ignore = "plays up to 5,200 full matches; verify runs it"]
fn a_quintile_fit_and_a_thin_sample_fit_land_within_their_noise() {
    let data = temp("engine-cli-primitives", "possession-fits");
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    for (primitive, level, source) in [
        ("passing_directness", "direct", "wyscout_2017_18"),
        ("forward_runs", "push_up", "pff_wc2022"),
    ] {
        let out = data.join(format!("fit-{primitive}.json"));
        let run = primitives(
            &data,
            &content,
            &[
                "fit",
                "--primitive",
                primitive,
                "--level",
                level,
                "--matches",
                "200",
                "--out",
                out.to_str().unwrap(),
            ],
        );
        assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
        let fit: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        let search = &fit["search"];
        assert_eq!(search["unreachable"], false, "{fit}");
        let (achieved, noise, target) = (
            search["achieved"].as_f64().unwrap(),
            search["noise"].as_f64().unwrap(),
            fit["target"].as_f64().unwrap(),
        );
        assert!((achieved - target).abs() <= noise, "{fit}");
        assert_eq!(fit["source"], source);
    }
    std::fs::remove_dir_all(&data).unwrap();
}

/// On winning the ball: every step one level up raises the fast breaks a match by more than
/// the paired noise of 1,000 pairs (verify runs it). The counter-press primitives are deferred
/// and not judged.
#[test]
#[ignore = "plays 8,000 full matches; verify runs it"]
fn every_on_winning_step_raises_the_fast_breaks() {
    let data = temp("engine-cli-primitives", "transition-steps");
    let result = shipped_check(&data, "on_winning_the_ball");
    let steps = result["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 4);
    for s in steps {
        assert_eq!(s["stats"][0]["stat"], "counter_attacks_per_match", "{s}");
        assert_eq!(s["pass"], true, "{s}");
    }
    std::fs::remove_dir_all(&data).unwrap();
}

/// The on-winning level `counter` refitted: the search lands within the paired noise of the
/// real ratio of the Wyscout fast-break row (verify runs it).
#[test]
#[ignore = "plays up to 2,600 full matches; verify runs it"]
fn the_fit_of_on_winning_counter_lands_within_its_noise() {
    let data = temp("engine-cli-primitives", "transition-fit");
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let out = data.join("fit.json");
    let run = primitives(
        &data,
        &content,
        &[
            "fit",
            "--primitive",
            "on_winning_the_ball",
            "--level",
            "counter",
            "--matches",
            "200",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    let fit: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let search = &fit["search"];
    assert_eq!(search["unreachable"], false, "{fit}");
    let (achieved, noise, target) = (
        search["achieved"].as_f64().unwrap(),
        search["noise"].as_f64().unwrap(),
        fit["target"].as_f64().unwrap(),
    );
    assert!((achieved - target).abs() <= noise, "{fit}");
    assert_eq!(fit["source"], "wyscout_2017_18");
    assert_eq!(fit["stat"], "counter_attacks_per_match");
    std::fs::remove_dir_all(&data).unwrap();
}

/// One shipped level refitted: the search lands within the paired noise of the real ratio and
/// near the value the tactics file holds (verify runs it).
#[test]
#[ignore = "plays up to 2,600 full matches; verify runs it"]
fn the_fit_of_tempo_slow_lands_within_the_noise_of_its_real_ratio() {
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
            "slow",
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
