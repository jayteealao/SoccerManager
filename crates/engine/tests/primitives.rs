//! The tactical primitives frame: the primitive entries of the tactics file load and refuse
//! a bad entry by name; no engine code names a style; no decision site reads an instruction
//! by name; and every plan input a decision site reads is the input of a hook in the hook
//! list, the fixed interface of the choice model.

mod common;

use std::path::{Path, PathBuf};

use engine::data::primitives::{
    Direction, GRADED_LEVELS, PrimitiveKind, SIGNATURE_STATS, held_out, level_value,
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
        assert_eq!(p.kind, PrimitiveKind::Graded, "{}", p.name);
        let levels = p.level_names(t);
        assert_eq!(levels.len(), GRADED_LEVELS, "{}", p.name);
        assert!(levels.contains(&p.default.as_str()), "{}", p.name);
        assert!(!p.inputs.is_empty());
        for ih in &p.inputs {
            let hook = hooks::hook(&ih.hook).unwrap_or_else(|| panic!("{}", ih.hook));
            assert_eq!(hook.input, ih.input, "{}", p.name);
            for level in 0..GRADED_LEVELS {
                assert!(level_value(t, &ih.input, level).is_some());
            }
        }
        assert!(!p.signature.is_empty());
        assert!(
            p.signature
                .iter()
                .all(|s| SIGNATURE_STATS.contains(&s.stat.as_str()))
        );
        let source = p.data.source.as_deref().expect("a real data source");
        assert!(p.data.competitions.iter().all(|c| !held_out(source, c)));
    }
    let tempo = t.primitives.iter().find(|p| p.name == "tempo").unwrap();
    assert_eq!(
        tempo.level_names(t),
        [
            "very_direct",
            "direct",
            "balanced",
            "patient",
            "very_patient"
        ]
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
    // Each fitted primitive holds a fit record for every level but its default (the base of
    // every ratio), and each fit landed within its paired noise of the real ratio, or names
    // the target as out of reach.
    for p in &t.primitives {
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

/// Each bad entry in `tests/fixtures/primitives/` is refused with its own reason: a graded
/// primitive with other than 5 levels, a choice primitive with no choices, an on/off
/// primitive without two states, a default that is not a level, no input, an unknown hook, a
/// hook that reads another input, an unknown statistic, no signature, neither a source nor a
/// reason, a held-out competition, and a fit record that differs from its level.
#[test]
fn every_bad_primitive_entry_is_refused_by_name() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/primitives");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 16);
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

/// The tactics store at most 16 primitives that own their levels or choices: a file with more
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
        reason.contains("17 primitives own their levels or choices; the tactics store at most 16"),
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

/// The names a decision site must not spell: every instruction key, every instruction level
/// and every mentality of the shipped tactics file.
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

/// Shape P2 (b): no decision site reads an instruction, a level or a mentality by its name;
/// a planted copy of a decision file that spells a level name is caught.
#[test]
fn no_decision_site_reads_an_instruction_by_name() {
    let names = tactic_names();
    assert!(names.contains(&"patient".to_string()));
    let mut found = Vec::new();
    for path in decision_sites() {
        for f in decision_site_findings(&non_test_source(&path), &names) {
            found.push(format!("{}: {f}", path.display()));
        }
    }
    assert!(found.is_empty(), "{found:?}");
    let planted = format!(
        "{}\nfn planted(level: &str) -> bool {{ level == \"patient\" }}\n",
        non_test_source(&src().join("decision.rs"))
    );
    assert_eq!(
        decision_site_findings(&planted, &names),
        ["literal \"patient\""]
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
    for path in decision_sites() {
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
    let missing: Vec<&String> = read
        .iter()
        .filter(|f| !inputs.contains(&f.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "plan inputs read with no hook: {missing:?}"
    );
    assert_eq!(HOOKS.len(), 33);
}
