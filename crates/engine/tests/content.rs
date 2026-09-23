//! AC-a: the shipped attribute schema loads with 30 to 50 named, grouped attributes on the
//! 1 to 100 scale. AC-c: a rule pack with an unknown schema version is refused naming the
//! version, and the current version loads. Plus: the tuning file equals `Tuning::default()`
//! and a missing file names its path. The rule pack pins the added-time allowance and the
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
