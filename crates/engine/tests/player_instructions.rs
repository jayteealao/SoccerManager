//! Game-state rules and player instructions as written: a tactic of six rules loads and one
//! of seven is refused; an instruction on a primitive outside the override list (marking
//! among them) is refused; an unknown primitive, level, choice, formation, slot or condition
//! kind is refused by name; forward runs are refused for a slot off the attack line.

mod common;

use engine::data::game_state::{self, GameStateFile};
use engine::data::tactics::TacticsSchema;
use engine::tactics::rules::{Action, Condition, Score};

fn schema() -> TacticsSchema {
    common::content().tactics
}

/// The 4-4-2 the default tactics play.
fn four_four_two(schema: &TacticsSchema) -> u8 {
    schema.formation_index("4-4-2").unwrap() as u8
}

fn fixture(name: &str) -> GameStateFile {
    let bytes = std::fs::read(common::fixture_path(&format!("game_state/{name}"))).unwrap();
    game_state::parse(&bytes).unwrap()
}

fn refusal(file: &GameStateFile) -> String {
    let schema = schema();
    let (field, reason) =
        game_state::resolve(file, &schema, &[four_four_two(&schema)]).unwrap_err();
    format!("{field}: {reason}")
}

fn edited(name: &str, edit: impl FnOnce(&mut serde_json::Value)) -> GameStateFile {
    let mut v = serde_json::to_value(fixture(name)).unwrap();
    edit(&mut v);
    serde_json::from_value(v).unwrap()
}

#[test]
fn a_tactic_of_six_rules_loads_with_every_condition_kind() {
    let schema = schema();
    let state = game_state::resolve(
        &fixture("six-rules.json"),
        &schema,
        &[four_four_two(&schema)],
    )
    .unwrap();
    assert_eq!(state.rules.len(), 6);
    assert_eq!(state.instructions.len(), 4);
    let first = &state.rules[0];
    assert_eq!(
        first.when,
        [
            Condition::Score(Score::Leading(1)),
            Condition::TimeLeft { minutes: 15.0 }
        ]
    );
    let mentality = schema
        .primitives
        .iter()
        .position(|p| p.name == "mentality")
        .unwrap() as u8;
    assert_eq!(
        first.set[0],
        Action::Level {
            primitive: mentality,
            level: 1
        }
    );
    let mut kinds: Vec<&str> = state
        .rules
        .iter()
        .flat_map(|r| r.when.iter().map(|c| c.kind()))
        .collect();
    kinds.sort();
    kinds.dedup();
    assert_eq!(kinds.len(), 6, "{kinds:?}");
    assert_eq!(
        state.rules[1].formation(),
        schema.formation_index("4-3-3").map(|f| f as u8)
    );
    // The written form reads back to the same file.
    let again = game_state::resolve(
        &game_state::written(&state, &schema),
        &schema,
        &[four_four_two(&schema)],
    )
    .unwrap();
    assert_eq!(again, state);
}

#[test]
fn a_tactic_of_seven_rules_is_refused() {
    let err = refusal(&fixture("seven-rules.json"));
    assert!(err.contains("7 rules; a tactic holds at most 6"), "{err}");
}

#[test]
fn an_instruction_outside_the_override_list_is_refused_marking_among_them() {
    let err = refusal(&fixture("marking-instruction.json"));
    assert!(
        err.contains("marking is not a primitive and not on the player override list"),
        "{err}"
    );
    let err = refusal(&fixture("tempo-instruction.json"));
    assert!(
        err.contains("tempo is not on the player override list"),
        "{err}"
    );
}

#[test]
fn forward_runs_are_refused_off_the_attack_line() {
    let err = refusal(&fixture("forward-runs-off-the-line.json"));
    assert!(
        err.contains("forward_runs reaches only LW, RW, ST; slot 6 is CM in 4-4-2"),
        "{err}"
    );
    // A rule's formation joins the check: slot 9 is a striker in the 4-4-2 and the attacking
    // midfielder in the 4-4-1-1.
    let file = edited("six-rules.json", |v| {
        v["instructions"][0]["slot"] = 9.into();
    });
    let err = refusal(&file);
    assert!(err.contains("slot 9 is AM in 4-4-1-1"), "{err}");
}

#[test]
fn unknown_names_and_slots_are_refused_by_name() {
    let err = refusal(&edited("six-rules.json", |v| {
        v["rules"][0]["set"][0]["primitive"] = "swagger".into();
    }));
    assert!(err.contains("swagger is not a primitive"), "{err}");
    let err = refusal(&edited("six-rules.json", |v| {
        v["rules"][0]["set"][0]["level"] = "reckless".into();
    }));
    assert!(
        err.contains("reckless is not a level of mentality"),
        "{err}"
    );
    let err = refusal(&edited("six-rules.json", |v| {
        v["instructions"][3]["level"] = "curled".into();
    }));
    assert!(err.contains("curled is not a level of cross_type"), "{err}");
    let err = refusal(&edited("six-rules.json", |v| {
        v["rules"][1]["set"][1]["formation"] = "2-2-6".into();
    }));
    assert!(err.contains("2-2-6 is not a formation"), "{err}");
    let err = refusal(&edited("six-rules.json", |v| {
        v["rules"][2]["when"][0]["kind"] = "weather".into();
    }));
    assert!(err.contains("weather is not a condition kind"), "{err}");
    let err = refusal(&edited("six-rules.json", |v| {
        v["instructions"][1]["slot"] = 11.into();
    }));
    assert!(err.contains("slot 11 is not a slot"), "{err}");
}

#[test]
fn a_malformed_rule_is_refused_by_name() {
    let err = refusal(&edited("leading-late.json", |v| {
        v["rules"][0]["when"] = serde_json::json!([]);
    }));
    assert!(err.contains("a rule needs a condition"), "{err}");
    let err = refusal(&edited("leading-late.json", |v| {
        v["rules"][0]["set"] = serde_json::json!([]);
    }));
    assert!(err.contains("a rule needs an action"), "{err}");
    let err = refusal(&edited("leading-late.json", |v| {
        let first = v["rules"][0]["when"][0].clone();
        v["rules"][0]["when"][1] = first;
    }));
    assert!(err.contains("score appears twice in the rule"), "{err}");
    let err = refusal(&edited("leading-late.json", |v| {
        let first = v["rules"][0]["set"][0].clone();
        v["rules"][0]["set"][1] = first;
    }));
    assert!(err.contains("mentality appears twice in the rule"), "{err}");
    let err = refusal(&edited("leading-late.json", |v| {
        v["rules"][0]["when"][1]["minutes"] = 90.into();
    }));
    assert!(err.contains("allowed 1 to 45"), "{err}");
    let err = refusal(&edited("leading-late.json", |v| {
        v["rules"][0]["when"][1]["by"] = 1.into();
    }));
    assert!(err.contains("time_left takes minutes, not by"), "{err}");
    let err = refusal(&edited("six-rules.json", |v| {
        let first = v["instructions"][1].clone();
        v["instructions"][2] = first;
    }));
    assert!(
        err.contains("slot 5 has two instructions on dribbling"),
        "{err}"
    );
}

#[test]
fn a_tactics_file_without_the_block_takes_no_rule() {
    let mut schema = schema();
    schema.game_state = None;
    let (_, reason) =
        game_state::resolve(&fixture("leading-late.json"), &schema, &[0]).unwrap_err();
    assert!(reason.contains("no game_state block"), "{reason}");
    assert!(
        game_state::resolve(&GameStateFile::default(), &schema, &[0])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_style_preset_may_carry_rules_and_instructions_and_the_shipped_ones_carry_two_rules() {
    let content = common::content();
    let styles = content.styles.as_ref().expect("the styles ship");
    // Each shipped style chases a late deficit and holds a late lead, with no instruction.
    assert!(
        styles
            .styles
            .iter()
            .all(|s| s.game_state.rules.len() == 2 && s.game_state.instructions.is_empty())
    );
    let path = common::content_dir().path("styles.json");
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    v["styles"][0]["rules"] = serde_json::to_value(fixture("leading-late.json").rules).unwrap();
    let loaded = engine::data::styles::load_styles_bytes(
        &serde_json::to_vec(&v).unwrap(),
        "styles.json",
        &content.tactics,
        &content.attributes,
    )
    .unwrap();
    assert_eq!(loaded.value.styles[0].game_state.rules.len(), 1);
    v["styles"][0]["player_instructions"] =
        serde_json::to_value(fixture("marking-instruction.json").instructions).unwrap();
    let err = engine::data::styles::load_styles_bytes(
        &serde_json::to_vec(&v).unwrap(),
        "styles.json",
        &content.tactics,
        &content.attributes,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("styles[0].instructions[0].primitive"), "{err}");
}
