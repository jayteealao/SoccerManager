//! AC-a: the shipped attribute schema loads with 30 to 50 named, grouped attributes on the
//! 1 to 100 scale. AC-c: a rule pack with an unknown schema version is refused naming the
//! version, and the current version loads. Plus: the tuning file equals `Tuning::default()`
//! and a missing file names its path; an older tuning block without the defending and
//! discipline values loads with their defaults, and one without the shot values loads with
//! the shipped ones. The rule pack pins the added-time allowance and the
//! minimum team size, and a schema without `aggression` is refused naming it.

mod common;

use engine::EngineError;
use engine::data::{
    ATTRIBUTES_VERSION, AttributeSchema, RULES_VERSION, RulePack, StoppageKind, load_json,
};
use engine::{ContentDir, Tuning};

#[test]
fn the_shipped_attribute_schema_has_thirty_to_fifty_grouped_names() {
    let content = common::content();
    let n = content.attributes.len();
    assert!((30..=50).contains(&n), "{n} attributes");
    for def in &content.attributes.attributes {
        assert!(!def.name.is_empty());
        // The group is an enum: every loaded definition carries one of the four.
        let _ = def.group;
    }
    // Every generated value on every default player lies on the 1 to 100 scale.
    for team in common::default_teams(&content) {
        for p in &team.players {
            assert_eq!(p.attributes.len(), n, "player {} attribute count", p.id);
            for (name, v) in &p.attributes {
                assert!((1..=100).contains(v), "player {} {name} = {v}", p.id);
            }
        }
    }
}

#[test]
fn an_unknown_rule_pack_version_is_refused_naming_the_version() {
    let path = common::fixture_path("rules-unknown-version.json");
    let err = load_json::<RulePack>(
        "rules",
        &path,
        "rules-unknown-version.json",
        RULES_VERSION,
        &(),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            EngineError::Version {
                found: 99,
                expected: 4,
                ..
            }
        ),
        "{err}"
    );
    assert_eq!(
        err.to_string(),
        "content refused: rules rules-unknown-version.json: schema_version 99; this build reads 4"
    );
}

#[test]
fn the_current_rule_pack_version_loads() {
    let content = common::content();
    assert_eq!(content.rules.schema_version, RULES_VERSION);
    assert_eq!(content.rules.halves, 2);
    assert_eq!(content.rules.stoppages.len(), 9);
    let added = &content.rules.added_time;
    assert_eq!(added.per_kind.len(), StoppageKind::ALL.len());
    assert_eq!(added.seconds(StoppageKind::Goal), 40);
    assert_eq!(added.seconds(StoppageKind::ThrowIn), 1);
    assert_eq!(
        (added.card_s, added.variance_s, added.min_s, added.max_s),
        (15, 30, 60, 900)
    );
    assert_eq!(content.rules.min_players, 7);
    let extra = &content.rules.extra_time;
    assert_eq!(
        (
            extra.periods,
            extra.period_minutes,
            extra.added_max_s,
            extra.extra_substitutions,
            extra.extra_windows
        ),
        (2, 15, 300, 1, 1)
    );
    let shootout = &content.rules.shootout;
    assert_eq!((shootout.kicks, shootout.allowance_rounds), (5, 10));
}

/// A version 3 rule pack has no extra-time or shoot-out block, so this build refuses it by
/// its version before it reads any field.
#[test]
fn a_version_three_rule_pack_is_refused_by_its_version() {
    let shipped = std::fs::read_to_string(common::content_dir().path("rules/default.json"))
        .expect("the shipped rule pack reads");
    let old = shipped.replacen("\"schema_version\": 4", "\"schema_version\": 3", 1);
    assert_ne!(old, shipped, "the shipped pack names version 4");
    let path = std::env::temp_dir().join(format!("engine-rules-v3-{}.json", std::process::id()));
    std::fs::write(&path, old).unwrap();
    let err =
        load_json::<RulePack>("rules", &path, "rules/v3.json", RULES_VERSION, &()).unwrap_err();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        err.to_string(),
        "content refused: rules rules/v3.json: schema_version 3; this build reads 4"
    );
}

#[test]
fn a_schema_without_aggression_is_refused_naming_it() {
    let shipped = common::content_dir().path("attributes.json");
    let text = std::fs::read_to_string(&shipped).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let list = value["attributes"].as_array_mut().unwrap();
    list.retain(|a| a["name"] != "aggression");
    let path = std::env::temp_dir().join(format!(
        "engine-test-{}-no-aggression.json",
        std::process::id()
    ));
    std::fs::write(&path, serde_json::to_string(&value).unwrap()).unwrap();
    let err = load_json::<AttributeSchema>(
        "attributes",
        &path,
        "attributes.json",
        ATTRIBUTES_VERSION,
        &(),
    )
    .unwrap_err();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        err.to_string(),
        "content refused: attributes attributes.json: attributes: required attribute aggression is missing"
    );
}

#[test]
fn the_shipped_tuning_equals_the_documented_default() {
    let content = common::content();
    assert_eq!(content.tuning.engine, Tuning::default());
    assert_eq!(content.tuning.generator.squad_size, 22);
}

#[test]
fn the_shipped_defending_and_discipline_values_are_pinned() {
    let t = common::content().tuning.engine;
    assert_eq!(t.cover_distance, 3.0);
    assert_eq!(t.cover_channel, 12.0);
    assert_eq!(t.back_line_gap, 12.0);
    assert_eq!(t.foul_cooldown_ticks, 150);
    assert_eq!(t.foul_booked_factor, 0.15);
}

/// A tuning block written before the defending and discipline values existed still loads,
/// with the shipped values for them.
#[test]
fn a_tuning_block_without_the_defending_values_loads_with_the_defaults() {
    let mut json = serde_json::to_value(Tuning::default()).unwrap();
    let block = json.as_object_mut().unwrap();
    for key in [
        "cover_distance",
        "cover_channel",
        "back_line_gap",
        "foul_cooldown_ticks",
        "foul_booked_factor",
    ] {
        assert!(block.remove(key).is_some(), "{key} is a tuning field");
    }
    let t: Tuning = serde_json::from_value(json).unwrap();
    assert_eq!(t, Tuning::default());
}

#[test]
fn the_shipped_shot_values_are_pinned() {
    let t = common::content().tuning.engine;
    assert_eq!(t.shot_noise, 0.5);
    let s = &t.shots;
    assert_eq!((s.loft_max, s.save_high, s.save_low), (7.0, 0.891, 0.272));
    assert_eq!(
        (
            s.quality.intercept,
            s.quality.distance_coef,
            s.quality.angle_coef
        ),
        (-1.0, -0.1, 1.0)
    );
    assert_eq!(
        (s.save_hold, s.parry_speed, s.parry_spread, s.parry_loft),
        (0.333, 0.8, 1.2, 4.0)
    );
    assert_eq!(
        (s.block_reach, s.block_chance, s.block_speed, s.block_spread),
        (3.0, 0.5, 0.8, 3.2)
    );
    assert_eq!(
        (
            s.penalty_xg,
            s.penalty_spread,
            s.keeper_dive_m,
            s.keeper_stays
        ),
        (0.76, 0.25, 2.0, 0.1)
    );
    assert_eq!(
        (t.xg.intercept, t.xg.distance_coef, t.xg.angle_coef),
        (-4.191, -0.0441, 6.3836)
    );
}

/// A tuning block written before the shot values existed still loads, with the shipped
/// shot values.
#[test]
fn a_tuning_block_without_the_shot_values_loads_with_the_defaults() {
    let mut json = serde_json::to_value(Tuning::default()).unwrap();
    let block = json.as_object_mut().unwrap();
    assert!(block.remove("shots").is_some(), "shots is a tuning block");
    let t: Tuning = serde_json::from_value(json).unwrap();
    assert_eq!(t, Tuning::default());
}

#[test]
fn the_shipped_lone_forward_and_tackle_values_are_pinned() {
    let t = common::content().tuning.engine;
    assert_eq!(t.tackle_win_base, 0.5);
    assert_eq!(t.lone_line_hold, 0.0);
    assert_eq!(t.decision.lone_layoff, 0.0);
    assert_eq!(t.decision.lone_hold, 0.5);
    assert_eq!(t.decision.lone_dribble, -0.5);
}

/// A tuning block written before the lone-forward and tackle values existed still loads,
/// with the values that reproduce the behaviour before them.
#[test]
fn a_tuning_block_without_the_lone_forward_values_loads_with_the_defaults() {
    let mut json = serde_json::to_value(Tuning::default()).unwrap();
    let block = json.as_object_mut().unwrap();
    for key in ["tackle_win_base", "lone_line_hold"] {
        assert!(block.remove(key).is_some(), "{key} is a tuning field");
    }
    let decision = block["decision"].as_object_mut().unwrap();
    for key in ["lone_layoff", "lone_hold", "lone_dribble"] {
        assert!(decision.remove(key).is_some(), "{key} is a decision weight");
    }
    let t: Tuning = serde_json::from_value(json).unwrap();
    assert_eq!(t.tackle_win_base, 0.05);
    assert_eq!(t.lone_line_hold, 0.0);
    assert_eq!(
        (
            t.decision.lone_layoff,
            t.decision.lone_hold,
            t.decision.lone_dribble
        ),
        (0.0, 0.0, 0.0)
    );
}

/// A tuning block written before the contest values existed still loads, with the values
/// that reproduce play before them.
#[test]
fn a_tuning_block_without_the_contest_values_loads_with_the_defaults() {
    let mut json = serde_json::to_value(Tuning::default()).unwrap();
    let block = json.as_object_mut().unwrap();
    for key in ["tackle_reach", "press_engage", "tackle_dribble_win"] {
        assert!(block.remove(key).is_some(), "{key} is a tuning field");
    }
    let t: Tuning = serde_json::from_value(json).unwrap();
    assert_eq!(
        (t.tackle_reach, t.press_engage, t.tackle_dribble_win),
        (1.0, 3.0, 0.0)
    );
}

#[test]
fn the_shipped_contest_values_are_pinned() {
    let t = common::content().tuning.engine;
    assert_eq!(
        (t.tackle_reach, t.press_engage, t.tackle_dribble_win),
        (1.0, 3.0, 0.0)
    );
}

#[test]
fn the_shipped_carry_clearance_and_restart_values_are_pinned() {
    let t = common::content().tuning.engine;
    assert_eq!((t.decision.carry_s, t.decision.carry_cost), (0.0, 0.0));
    let c = &t.clearances;
    assert_eq!(
        (
            c.aim_spread,
            c.cross_reach,
            c.cross_chance,
            c.cross_speed,
            c.cross_spread,
            c.cross_loft
        ),
        (0.6, 2.0, 0.0, 0.7, 1.6, 4.0)
    );
    let d = &t.restart_delay_s;
    assert_eq!(
        (
            d.kick_off,
            d.throw_in,
            d.corner,
            d.goal_kick,
            d.free_kick,
            d.penalty,
            d.drop_ball
        ),
        (5.0, 3.0, 8.0, 6.0, 8.0, 15.0, 20.0)
    );
}

/// A tuning block written before the carry window and the clearance values existed still
/// loads, with the values that reproduce play before them.
#[test]
fn a_tuning_block_without_the_carry_and_clearance_values_loads_with_the_defaults() {
    let mut json = serde_json::to_value(Tuning::default()).unwrap();
    let block = json.as_object_mut().unwrap();
    assert!(
        block.remove("clearances").is_some(),
        "clearances is a tuning block"
    );
    let decision = block["decision"].as_object_mut().unwrap();
    for key in ["carry_s", "carry_cost"] {
        assert!(decision.remove(key).is_some(), "{key} is a decision weight");
    }
    let t: Tuning = serde_json::from_value(json).unwrap();
    assert_eq!((t.decision.carry_s, t.decision.carry_cost), (0.0, 0.0));
    assert_eq!(t.clearances.aim_spread, 0.6);
    assert_eq!(t.clearances.cross_chance, 0.0);
}

#[test]
fn a_missing_content_file_names_its_path() {
    let dir = ContentDir::at(std::env::temp_dir().join("engine-no-such-content"));
    let err = engine::Content::load(&dir).unwrap_err();
    assert_eq!(err.to_string(), "cannot read attributes.json");
    assert!(matches!(err, EngineError::Read { .. }));
}

/// Every flag engine code reads is declared in the shipped tuning file, and the shipped file
/// declares no flag of its own: a candidate model lands with its flag, and the flag leaves
/// with the losing model.
#[test]
fn the_shipped_flags_block_matches_the_code_flags() {
    let content = common::content();
    let declared = &content.written_tuning().flags;
    for name in engine::CODE_FLAGS {
        assert!(declared.contains_key(*name), "{name} is not declared");
    }
    assert!(
        declared.is_empty(),
        "the shipped file declares {declared:?}"
    );
    assert!(content.flags.is_empty());
}
