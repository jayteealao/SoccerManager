//! The tactical primitives frame: the primitive entries of the tactics file load and refuse
//! a bad entry by name; no engine code names a style; no decision site reads an instruction
//! by name; and every plan input a decision site reads is the input of a hook in the hook
//! list, the fixed interface of the choice model.

mod common;

use std::path::{Path, PathBuf};

use engine::data::primitives::{
    Direction, GRADED_LEVELS, PrimitiveKind, SIGNATURE_STATS, Steps, held_out, level_value,
};
use engine::tactics::hooks::{self, HOOKS};

fn shipped_tactics_json() -> serde_json::Value {
    let path = common::content_dir().path(engine::data::TACTICS_FILE);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// The refusal of the shipped tactics file with `planted` added to its primitives, or `None`
/// when it loads and passes every check.
fn refusal_with(planted: serde_json::Value) -> Option<String> {
    let mut v = shipped_tactics_json();
    v["primitives"].as_array_mut().unwrap().push(planted);
    let bytes = serde_json::to_vec(&v).unwrap();
    let loaded = match engine::data::load_tactics_bytes(&bytes, "tactics.json") {
        Ok(l) => l,
        Err(e) => return Some(e.to_string()),
    };
    let attributes = common::content().attributes;
    loaded
        .value
        .check(&attributes)
        .err()
        .map(|(field, reason)| format!("{field}: {reason}"))
}

/// The shipped primitives: tempo and mentality are graded, each with 5 ordered levels, a
/// default among them, its plan inputs at known hooks whose input matches, a signature of
/// known statistics, and a real data source outside the held-out sets.
#[test]
fn the_shipped_primitives_are_complete() {
    let content = common::content();
    let t = &content.tactics;
    let names: Vec<&str> = t.primitives.iter().map(|p| p.name.as_str()).collect();
    assert!(
        names.contains(&"tempo") && names.contains(&"mentality"),
        "{names:?}"
    );
    for p in &t.primitives {
        let levels = p.level_names(t);
        if p.kind == PrimitiveKind::Graded {
            assert_eq!(levels.len(), GRADED_LEVELS, "{}", p.name);
        } else if p.kind == PrimitiveKind::OnOff {
            assert_eq!(levels.len(), 2, "{}", p.name);
        } else {
            assert!(levels.len() >= 2, "{}", p.name);
        }
        assert!(levels.contains(&p.default.as_str()), "{}", p.name);
        assert!(!p.inputs.is_empty());
        for ih in &p.inputs {
            let hook = hooks::hook(&ih.hook).unwrap_or_else(|| panic!("{}", ih.hook));
            let second = hooks::SECOND_INPUTS
                .iter()
                .any(|(h, i)| *h == hook.name && *i == ih.input);
            assert!(hook.input == ih.input || second, "{}", p.name);
            if p.kind == PrimitiveKind::Graded {
                for level in 0..GRADED_LEVELS {
                    assert!(level_value(t, &ih.input, level).is_some());
                }
            }
        }
        assert!(!p.signature.is_empty());
        assert!(
            p.signature
                .iter()
                .all(|s| SIGNATURE_STATS.contains(&s.stat.as_str()))
        );
        match p.data.source.as_deref() {
            Some(source) => assert!(p.data.competitions.iter().all(|c| !held_out(source, c))),
            None => assert!(
                p.data.none.as_deref().is_some_and(|r| !r.is_empty()) && p.fits.is_empty(),
                "{}: a primitive with no real data names why",
                p.name
            ),
        }
    }
    let tempo = t.primitives.iter().find(|p| p.name == "tempo").unwrap();
    assert_eq!(
        tempo.level_names(t),
        ["very_fast", "fast", "normal", "slow", "very_slow"]
    );
    assert_eq!(tempo.signature[0].stat, "passes_per_possession");
    assert_eq!(tempo.signature[0].direction, Direction::Up);
    let mentality = t.primitives.iter().find(|p| p.name == "mentality").unwrap();
    assert_eq!(mentality.signature[0].stat, "turnovers_per_possession");
    // The loss weight falls as the mentality rises: an attacking side fears a loss least.
    let weights: Vec<f64> = t.mentalities.iter().map(|m| m.loss_weight).collect();
    assert!(weights.windows(2).all(|w| w[0] >= w[1]), "{weights:?}");
    // Patience rises with the tempo level.
    let patience: Vec<f64> = (0..GRADED_LEVELS)
        .map(|l| level_value(t, "patience", l).unwrap())
        .collect();
    assert!(patience.windows(2).all(|w| w[0] < w[1]), "{patience:?}");
    // Each graded primitive with real data that is not deferred holds a fit record for every
    // level but its default (the base of every ratio), and each fit landed within its paired
    // noise of the real ratio, or names the target as out of reach. A choice, a primitive with
    // no real data and a deferred one hold none.
    for p in &t.primitives {
        let fitted =
            p.kind == PrimitiveKind::Graded && p.data.source.is_some() && p.data.deferred.is_none();
        if !fitted {
            assert!(p.fits.is_empty(), "{}: no fit records", p.name);
            continue;
        }
        let levels = p.level_names(t);
        let fitted: Vec<&str> = p.fits.iter().map(|f| f.level.as_str()).collect();
        let others: Vec<&str> = levels.iter().copied().filter(|l| *l != p.default).collect();
        assert_eq!(fitted, others, "{}: the fit records", p.name);
        for fit in &p.fits {
            assert!(
                fit.matches >= 200,
                "{} {}: {} matches",
                p.name,
                fit.level,
                fit.matches
            );
            assert!(
                fit.unreachable || (fit.achieved - fit.target).abs() <= fit.noise,
                "{} {}: achieved {} against {} (noise {})",
                p.name,
                fit.level,
                fit.achieved,
                fit.target,
                fit.noise
            );
        }
    }
    assert!(
        refusal_with(serde_json::json!({
            "name": "planted_ok",
            "kind": "on_off",
            "states": ["off", "on"],
            "default": "off",
            "inputs": [{ "input": "pace_lift", "hook": "pace.lift" }],
            "signature": [{ "stat": "shot_share", "direction": "up" }],
            "data": { "none": "a planted entry" }
        }))
        .is_none()
    );
}

/// The in-possession primitives: each is on the shipped list with its kind; width and passing
/// directness keep their block and gain the outer levels; the crossing primitives are deferred
/// with their reason and keep seeded values; creative freedom names why no real data fits it;
/// every fitted input is read at an option weight, a threshold or a shape offset (a decision
/// weight), never at the one hook that reaches a kick; the keeper follows build-up, and no
/// in-possession primitive sets a keeper hook; and the dribble score's take-on pressure weight
/// is the tuning value, beside the shot's own.
#[test]
fn the_in_possession_primitives_are_shipped_as_decided() {
    let content = common::content();
    let t = &content.tactics;
    let find = |n: &str| {
        t.primitives
            .iter()
            .find(|p| p.name == n)
            .unwrap_or_else(|| panic!("{n} is shipped"))
    };
    let graded = [
        "width",
        "passing_directness",
        "build_up",
        "final_third",
        "dribbling",
        "crossing",
        "creative_freedom",
        "forward_runs",
        "fullback_run_frequency",
    ];
    let choices = ["attacking_channel", "cross_type", "fullback_runs"];
    for n in graded {
        assert_eq!(find(n).kind, PrimitiveKind::Graded, "{n}");
    }
    for n in choices {
        assert_eq!(find(n).kind, PrimitiveKind::Choice, "{n}");
    }
    assert_eq!(
        find("width").level_names(t),
        ["very_narrow", "narrow", "normal", "wide", "very_wide"]
    );
    assert_eq!(
        find("passing_directness").level_names(t),
        ["very_short", "short", "mixed", "direct", "very_direct"]
    );
    assert_eq!(
        find("attacking_channel").level_names(t),
        [
            "left",
            "right",
            "both_wings",
            "centre",
            "centre_left",
            "centre_right",
            "balanced"
        ]
    );
    assert_eq!(
        find("fullback_runs").level_names(t),
        ["stay_back", "underlap", "overlap"]
    );
    for n in ["crossing", "cross_type", "attacking_channel"] {
        let p = find(n);
        assert!(p.data.source.is_some(), "{n}");
        assert!(
            p.data.deferred.as_deref().is_some_and(|r| !r.is_empty()),
            "{n}: deferred with its reason"
        );
        assert!(p.fits.is_empty(), "{n}");
    }
    let freedom = find("creative_freedom");
    assert!(freedom.data.source.is_none() && freedom.data.none.is_some());
    // Forward runs: the upper levels are judged on the share of runs in behind, the lower on
    // the share of runs coming short, both from the World Cup's tracking data; the start
    // distance rises with the level and is 0 at the default.
    let runs = find("forward_runs");
    let scoped: Vec<(&str, Steps)> = runs
        .signature
        .iter()
        .map(|s| (s.stat.as_str(), s.steps))
        .collect();
    assert_eq!(
        scoped,
        [
            ("runs_in_behind_share", Steps::AboveDefault),
            ("coming_short_share", Steps::BelowDefault)
        ]
    );
    assert_eq!(runs.data.source.as_deref(), Some("pff_wc2022"));
    let start: Vec<f64> = (0..GRADED_LEVELS)
        .map(|l| level_value(t, "run_start_m", l).unwrap())
        .collect();
    assert!(start.windows(2).all(|w| w[0] < w[1]), "{start:?}");
    assert_eq!(start[2], 0.0);
    // Every fitted input is a decision weight: no fit sets ball physics.
    for p in &t.primitives {
        for fit in &p.fits {
            for input in fit.values.keys() {
                let ih = p.inputs.iter().find(|i| &i.input == input).unwrap();
                let hook = hooks::hook(&ih.hook).unwrap();
                assert!(
                    matches!(
                        hook.kind,
                        hooks::HookKind::OptionWeight
                            | hooks::HookKind::Threshold
                            | hooks::HookKind::ShapeOffset
                    ) && hook.name != "cross.trajectory",
                    "{} {}: {input} at {}",
                    p.name,
                    fit.level,
                    hook.name
                );
            }
        }
    }
    // The keeper reads the team's build-up: no separate keeper setting.
    let build_up = find("build_up");
    assert_eq!(build_up.inputs.len(), 1);
    assert_eq!(build_up.inputs[0].hook, "pass.build_up");
    for n in graded.iter().chain(&choices) {
        assert!(
            find(n)
                .inputs
                .iter()
                .all(|i| !i.hook.starts_with("keeper.")),
            "{n}"
        );
    }
    let w = &content.tuning.engine.decision;
    assert_eq!(w.shot_under_pressure, -0.8);
    assert!(w.takeon_under_pressure.is_finite());
}

/// The transition primitives: on winning the ball, on losing the ball and counter-press
/// length are graded with their own five levels. On winning the ball raises its weight level
/// by level from a neutral default, lifts running pace only above its default, and is fitted
/// to the Wyscout fast-break row. On losing the ball is neutral at its default (its
/// counter-press factor is 0, so the press stays the pressing level's), and both counter-press
/// primitives keep seeded values with their deferred reason and no fit. The five counter-press
/// lengths rise from about 2 s at the lowest level to about 8 s at the highest.
#[test]
fn the_transition_primitives_are_shipped_as_decided() {
    let content = common::content();
    let t = &content.tactics;
    let find = |n: &str| {
        t.primitives
            .iter()
            .find(|p| p.name == n)
            .unwrap_or_else(|| panic!("{n} is shipped"))
    };
    let on_winning = find("on_winning_the_ball");
    assert_eq!(
        on_winning.level_names(t),
        [
            "always_keep_shape",
            "keep_shape",
            "mixed",
            "counter",
            "always_counter"
        ]
    );
    assert_eq!(on_winning.default, "mixed");
    let at = |input: &str| -> Vec<f64> {
        (0..GRADED_LEVELS)
            .map(|l| level_value(t, input, l).unwrap())
            .collect()
    };
    let counter = at("counter");
    assert!(counter.windows(2).all(|w| w[0] < w[1]), "{counter:?}");
    assert_eq!(counter[2], 0.0, "neutral at the default");
    assert_eq!(at("pace_lift"), [0.0, 0.0, 0.0, 1.0, 1.0]);
    assert_eq!(on_winning.data.source.as_deref(), Some("wyscout_2017_18"));
    assert!(on_winning.data.deferred.is_none());
    let primary = &on_winning.signature[0];
    assert_eq!(
        (primary.stat.as_str(), primary.direction),
        ("counter_attacks_per_match", Direction::Up)
    );
    let table: serde_json::Value = serde_json::from_slice(
        &std::fs::read(common::content_dir().path("signatures.json")).unwrap(),
    )
    .unwrap();
    for s in &on_winning.signature {
        assert!(
            table["rows"].as_array().unwrap().iter().any(|r| {
                r["primitive"] == "on_winning_the_ball" && r["stat"] == s.stat.as_str()
            }),
            "{}: a real row",
            s.stat
        );
    }
    let on_losing = find("on_losing_the_ball");
    assert_eq!(
        on_losing.level_names(t),
        [
            "always_regroup",
            "regroup",
            "mixed",
            "counter_press",
            "always_counter_press"
        ]
    );
    let press = at("counter_press");
    assert!(press.windows(2).all(|w| w[0] < w[1]), "{press:?}");
    assert_eq!(press[2], 0.0, "neutral at the default");
    assert_eq!(
        press[0], -1.0,
        "the lowest level presses nobody in its window"
    );
    let length = find("counter_press_length");
    let seconds = at("counter_press_s");
    assert!(seconds.windows(2).all(|w| w[0] < w[1]), "{seconds:?}");
    assert!((1.5..=3.0).contains(&seconds[0]), "{seconds:?}");
    assert!((6.5..=10.0).contains(&seconds[4]), "{seconds:?}");
    assert_eq!(length.signature[0].stat, "last_counterpress_s");
    for p in [on_losing, length] {
        assert!(
            p.data.deferred.as_deref().is_some_and(|r| !r.is_empty()),
            "{}: deferred with its reason",
            p.name
        );
        assert!(p.fits.is_empty(), "{}", p.name);
    }
}

/// Each bad entry in `tests/fixtures/primitives/` is refused with its own reason: a graded
/// primitive with other than 5 levels, a choice primitive with no choices, an on/off
/// primitive without two states, a default that is not a level, no input, an unknown hook, a
/// hook that reads another input, an unknown statistic, no signature, neither a source nor a
/// reason, a held-out competition, a fit record that differs from its level, a deferred
/// primitive that holds a fit, and statistics scoped below or above the default that leave a
/// step unjudged.
#[test]
fn every_bad_primitive_entry_is_refused_by_name() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/primitives");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 20);
    for file in files {
        let case: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        let expect = case["refusal"].as_str().unwrap();
        let err = refusal_with(case["primitive"].clone())
            .unwrap_or_else(|| panic!("{} loaded", file.display()));
        assert!(
            err.contains("primitives.planted") && err.contains(expect),
            "{}: {err}",
            file.display()
        );
    }
}

/// The tactics store at most 32 primitives that own their levels or choices: a file with more
/// is refused, naming the count and the limit.
#[test]
fn more_settings_than_the_tactics_store_are_refused() {
    let mut v = shipped_tactics_json();
    let owning = common::content()
        .tactics
        .primitives
        .iter()
        .filter(|p| p.is_setting())
        .count();
    let list = v["primitives"].as_array_mut().unwrap();
    for i in 0..=(engine::data::primitives::SETTINGS_MAX - owning) {
        list.push(serde_json::json!({
            "name": format!("planted_{i}"),
            "kind": "choice",
            "choices": [
                { "name": "a", "values": { "patience": 0.0 }, "stat": "passes_per_possession" },
                { "name": "b", "values": { "patience": 1.0 }, "stat": "passes_per_possession" }
            ],
            "default": "a",
            "inputs": [{ "input": "patience", "hook": "pass.recycle" }],
            "signature": [{ "stat": "passes_per_possession", "direction": "up" }],
            "data": { "none": "a planted entry" }
        }));
    }
    let bytes = serde_json::to_vec(&v).unwrap();
    let loaded = engine::data::load_tactics_bytes(&bytes, "tactics.json").unwrap();
    let (field, reason) = loaded
        .value
        .check(&common::content().attributes)
        .unwrap_err();
    assert_eq!(field, "primitives");
    assert!(
        reason.contains("33 primitives own their levels or choices; the tactics store at most 32"),
        "{reason}"
    );
}

/// The source of `path` with every `#[cfg(test)]` module cut: the tests sit at the end of a
/// file, so the cut runs from the first `#[cfg(test)]` that opens a module to the end.
fn non_test_source(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    let mut out = String::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == "#[cfg(test)]"
            && lines
                .peek()
                .is_some_and(|next| next.trim_start().starts_with("mod "))
        {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The string literals of `source`, without comments.
fn literals(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
                break;
            }
            // A character literal such as '"' is not a string.
            if bytes[i] == b'\'' && bytes.get(i + 2) == Some(&b'\'') {
                i += 3;
                continue;
            }
            if bytes[i] == b'"' {
                let mut j = i + 1;
                let mut s = String::new();
                while j < bytes.len() && bytes[j] != b'"' {
                    if bytes[j] == b'\\' {
                        j += 1;
                    }
                    if j < bytes.len() {
                        s.push(bytes[j] as char);
                    }
                    j += 1;
                }
                // A key of a `json!` object (a literal followed by one colon) names a trace
                // field, not a setting.
                let rest = line[(j + 1).min(line.len())..].trim_start();
                if !rest.starts_with(':') || rest.starts_with("::") {
                    out.push(s);
                }
                i = j + 1;
                continue;
            }
            i += 1;
        }
    }
    out
}

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The decision sites: the files whose code chooses what a player does.
fn decision_sites() -> Vec<PathBuf> {
    let src = src();
    let mut out: Vec<PathBuf> = [
        "decision.rs",
        "shape.rs",
        "pace.rs",
        "steering.rs",
        "possession.rs",
        "shot.rs",
        "sim.rs",
        "rules/restart.rs",
    ]
    .iter()
    .map(|f| src.join(f))
    .collect();
    for entry in std::fs::read_dir(src.join("sim")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name.ends_with(".rs") && !name.ends_with("_tests.rs") {
            out.push(path);
        }
    }
    out
}

/// The names a decision site must not spell: every instruction key, every instruction level,
/// every mentality and every formation of the shipped tactics file.
fn tactic_names() -> Vec<String> {
    let t = common::content().tactics;
    let mut names: Vec<String> = engine::data::tactics::INSTRUCTIONS
        .iter()
        .map(|s| s.to_string())
        .collect();
    for i in 0..engine::data::tactics::INSTRUCTIONS.len() {
        names.extend(t.instructions.level_names(i).iter().map(|s| s.to_string()));
    }
    names.extend(t.mentalities.iter().map(|m| m.name.clone()));
    names.extend(t.formations.iter().map(|f| f.name.clone()));
    names.sort();
    names.dedup();
    names
}

/// What a decision site may not hold: a literal equal to a tactics name, an instruction
/// variant path, or a lookup by name. Each finding as `<what>`.
fn decision_site_findings(source: &str, names: &[String]) -> Vec<String> {
    let mut found: Vec<String> = literals(source)
        .into_iter()
        .filter(|l| names.contains(l))
        .map(|l| format!("literal \"{l}\""))
        .collect();
    for banned in [
        "Instruction::",
        "level_names(",
        "mentality_index(",
        "instruction_index(",
    ] {
        if source.contains(banned) {
            found.push(banned.to_string());
        }
    }
    found
}

/// Shape P2 (a): no engine code outside its tests spells a style's name. Two literals share a
/// word with a style and are not styles: the possession module's name in the module registry,
/// and the free-kick key of the rules.
#[test]
fn no_engine_code_names_a_style() {
    let styles: serde_json::Value =
        serde_json::from_slice(&std::fs::read(common::content_dir().path("styles.json")).unwrap())
            .unwrap();
    let names: Vec<String> = styles["styles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names.len(), 8);
    // (file, literal, why it is not a style)
    let allowed = [
        (
            "modules/registry.rs",
            "possession",
            "the possession module's name",
        ),
        (
            "rules/mod.rs",
            "direct",
            "the direct free kick's key in the rule pack",
        ),
        (
            "data/convert.rs",
            "direct",
            "a version 4 tempo level name that the converter renames",
        ),
    ];
    let mut files = Vec::new();
    rust_files(&src(), &mut files);
    let mut found = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(src())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if rel.ends_with("_tests.rs") {
            continue;
        }
        for l in literals(&non_test_source(&path)) {
            if names.contains(&l) && !allowed.iter().any(|(f, a, _)| *f == rel && *a == l) {
                found.push(format!("{rel}: \"{l}\""));
            }
        }
    }
    assert!(found.is_empty(), "engine code names a style: {found:?}");
}

/// Shape P2 (b): no decision site reads an instruction, a level, a mentality or a formation
/// by its name; a planted copy of a decision file that spells a level name is caught.
#[test]
fn no_decision_site_reads_an_instruction_by_name() {
    let names = tactic_names();
    assert!(names.contains(&"slow".to_string()));
    let mut found = Vec::new();
    for path in decision_sites() {
        for f in decision_site_findings(&non_test_source(&path), &names) {
            found.push(format!("{}: {f}", path.display()));
        }
    }
    assert!(found.is_empty(), "{found:?}");
    let planted = format!(
        "{}\nfn planted(level: &str) -> bool {{ level == \"slow\" }}\n",
        non_test_source(&src().join("decision.rs"))
    );
    assert_eq!(
        decision_site_findings(&planted, &names),
        ["literal \"slow\""]
    );
}

/// The fields of the team plan, read from its declaration.
fn team_plan_fields() -> Vec<String> {
    let text = std::fs::read_to_string(src().join("tactics/mod.rs")).unwrap();
    let start = text.find("pub struct TeamPlan {").unwrap();
    let body = &text[start..];
    let end = body.find("\n}").unwrap();
    body[..end]
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub "))
        .filter_map(|l| l.split(':').next())
        .filter(|f| !f.contains(' '))
        .map(str::to_string)
        .collect()
}

/// Shape P2 (c): every team-plan field a decision site reads is the input of a hook of the
/// hook list.
#[test]
fn every_plan_input_a_decision_site_reads_has_a_hook() {
    let fields = team_plan_fields();
    assert!(fields.contains(&"patience".to_string()), "{fields:?}");
    assert!(fields.contains(&"loss_weight".to_string()));
    let inputs: Vec<&str> = hooks::inputs().collect();
    let mut read = std::collections::BTreeSet::new();
    // The anchors' shape offsets are read where the team places its slots.
    let sites = decision_sites().into_iter().chain([src().join("team.rs")]);
    for path in sites {
        let source = non_test_source(&path);
        for (at, _) in source.match_indices("plan.") {
            let before = source[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let field: String = source[at + 5..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if fields.contains(&field) {
                read.insert(field);
            }
        }
    }
    assert!(
        read.contains("patience") && read.contains("loss_weight"),
        "{read:?}"
    );
    // The transition primitives reach play through their hooks only.
    for input in ["counter", "pace_lift", "counter_press", "counter_press_s"] {
        assert!(read.contains(input), "{input} is read: {read:?}");
    }
    // So do the out-of-possession primitives.
    for input in [
        "press_count",
        "press_distance",
        "block_depth",
        "engage_depth_m",
        "engage_line",
        "defensive_width",
        "force",
        "tackle_willingness",
        "tackle_risk",
        "offside_trap",
        "screen_short",
        "trigger_back_pass",
        "trigger_poor_touch",
        "trigger_facing_back",
        "trigger_wide",
    ] {
        assert!(read.contains(input), "{input} is read: {read:?}");
    }
    let missing: Vec<&String> = read
        .iter()
        .filter(|f| !inputs.contains(&f.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "plan inputs read with no hook: {missing:?}"
    );
    assert_eq!(HOOKS.len(), 48);
}

/// The tackle decision replaces the automatic tackle: in the possession pass, each tackle's
/// draw comes after the decision module's `tackle`, and a declined tackle skips the draw.
#[test]
fn a_tackle_draw_follows_the_tackle_decision() {
    let source = non_test_source(&src().join("sim/possession.rs"));
    let decision = source
        .find("decision.tackle(")
        .expect("the tackle decision is asked");
    let draw = source
        .find("Key::player(Action::Tackle, p)")
        .expect("the tackle draw");
    assert!(decision < draw, "the decision comes before the draw");
    let between = &source[decision..draw];
    assert!(
        between.contains("continue;"),
        "a declined tackle skips the draw"
    );
    assert_eq!(source.matches("Action::Tackle").count(), 1);
}

/// Tackling moves fouls and cards only through the tackle decision: the shipped card and foul
/// values are the ones the automatic tackle had, and no tackling level sets a card value.
#[test]
fn tackling_changes_no_card_value() {
    let content = common::content();
    let t = &content.tuning.engine;
    assert_eq!(
        (
            t.foul_base,
            t.yellow_base,
            t.yellow_aggression_weight,
            t.red_base
        ),
        (0.1, 0.05, 0.15, 0.005)
    );
    assert_eq!(t.foul_booked_factor, 0.15);
    let tackling = content
        .tactics
        .primitives
        .iter()
        .find(|p| p.name == "tackling")
        .unwrap();
    let inputs: Vec<&str> = tackling.inputs.iter().map(|i| i.input.as_str()).collect();
    assert_eq!(inputs, ["tackle_risk", "tackle_willingness"]);
}
