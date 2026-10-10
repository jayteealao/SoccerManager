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
    let performance = content
        .attributes
        .attributes
        .iter()
        .filter(|d| !d.is_personality())
        .count();
    assert!((30..=50).contains(&performance), "{performance} attributes");
    for def in &content.attributes.attributes {
        assert!(!def.name.is_empty());
        // The group is an enum: every loaded definition carries one of the four.
        let _ = def.group;
    }
    // The default teams are version 1 files: every value converts to 2 to 200 tenths
    // (0.2 to 20.0), and only a converted value may sit under 1.0.
    for team in common::default_teams(&content) {
        for p in &team.players {
            assert_eq!(p.attributes.len(), n, "player {} attribute count", p.id);
            for (name, v) in &p.attributes {
                assert!(
                    (2..=200).contains(&v.tenths()) && v.tenths() % 2 == 0,
                    "player {} {name} = {v}",
                    p.id
                );
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
        "content refused: attributes attributes.json: actions: tackle.choose: attribute aggression is not in the file"
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
            c.cross_loft,
            c.wide_chance,
            c.wide_depth
        ),
        (0.6, 2.0, 0.0, 0.7, 1.6, 4.0, 0.0, 16.5)
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
    assert_eq!(t.clearances.wide_chance, 0.0);
}

/// A clearances block written before the wide clearance existed still loads, with no
/// clearance going wide.
#[test]
fn a_clearances_block_without_the_wide_values_loads_with_none() {
    let mut json = serde_json::to_value(Tuning::default()).unwrap();
    let clearances = json["clearances"].as_object_mut().unwrap();
    for key in ["wide_chance", "wide_depth"] {
        assert!(
            clearances.remove(key).is_some(),
            "{key} is a clearance value"
        );
    }
    let t: Tuning = serde_json::from_value(json).unwrap();
    assert_eq!(
        (t.clearances.wide_chance, t.clearances.wide_depth),
        (0.0, 16.5)
    );
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

/// The shipped tuning file as JSON.
fn shipped_tuning_json() -> serde_json::Value {
    let path = common::content_dir().path(engine::data::TUNING_FILE);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// A version 3 tuning file (no state caps, body jobs, fatigue group weights or sprint cap)
/// converts to version 4 with the values of the frozen copy, which are the shipped ones.
#[test]
fn a_version_3_tuning_file_converts_to_the_shipped_version_4() {
    let mut v3 = shipped_tuning_json();
    v3["schema_version"] = 3.into();
    let contract = v3["engine"]["contract"].as_object_mut().unwrap();
    contract.remove("states").unwrap();
    contract.remove("body").unwrap();
    let fatigue = v3["fatigue"].as_object_mut().unwrap();
    fatigue.remove("group_weights").unwrap();
    fatigue.remove("sprint_cap").unwrap();
    let bytes = serde_json::to_vec(&v3).unwrap();
    let loaded = engine::data::load_tuning_bytes(&bytes, "tuning-v3.json").unwrap();
    assert_eq!(loaded.converted_from, Some(3));
    let shipped = engine::data::load_tuning_bytes(
        &serde_json::to_vec(&shipped_tuning_json()).unwrap(),
        "tuning.json",
    )
    .unwrap();
    assert_eq!(shipped.converted_from, None);
    assert_eq!(loaded.value, shipped.value);
}

#[test]
fn a_tuning_file_of_version_9_is_refused_naming_both_versions() {
    let mut v9 = shipped_tuning_json();
    v9["schema_version"] = 9.into();
    let err =
        engine::data::load_tuning_bytes(&serde_json::to_vec(&v9).unwrap(), "t.json").unwrap_err();
    assert_eq!(
        err.to_string(),
        format!(
            "content refused: tuning t.json: schema_version 9; this build reads {}",
            engine::data::TUNING_VERSION
        )
    );
}

/// A version 1 slot file (no sharpness or adaptation slot) converts to version 2 with both
/// slots at their first version, and resolves to the built-in selection.
#[test]
fn a_version_1_slot_file_converts_and_resolves_to_the_built_in_selection() {
    use engine::modules::{REGISTRY, ResolvedModules, SlotFile, resolve};
    let path = common::content_dir().path(engine::data::SLOTS_FILE);
    let mut v1: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    v1["schema_version"] = 1.into();
    let slots = v1["slots"].as_object_mut().unwrap();
    slots.remove("engine.modifier.sharpness").unwrap();
    slots.remove("engine.modifier.adaptation").unwrap();
    let loaded = engine::data::load_slots_bytes(&serde_json::to_vec(&v1).unwrap()).unwrap();
    assert_eq!(loaded.converted_from, Some(1));
    assert_eq!(loaded.value, SlotFile::builtin_default());
    let resolved = resolve(&loaded.value, REGISTRY).unwrap();
    assert_eq!(
        resolved.picked(),
        ResolvedModules::builtin_default().picked()
    );
    v1["schema_version"] = 9.into();
    let err = engine::data::load_slots_bytes(&serde_json::to_vec(&v1).unwrap()).unwrap_err();
    assert!(
        matches!(
            err,
            EngineError::Version {
                found: 9,
                expected: 2,
                ..
            }
        ),
        "{err}"
    );
}

/// The version 2 attribute schema (the one shipped before the hidden values, kept as a
/// fixture) converts through version 3: injury resistance becomes injury proneness, hidden,
/// with its job turned up, and consistency is added from the frozen copy. The result is the
/// version 3 file the build shipped before (`attributes-v3.json`), converted the same way.
#[test]
fn a_version_2_attribute_schema_converts_as_the_shipped_version_3_did() {
    let bytes = std::fs::read(common::fixture_path("attributes-v2.json")).unwrap();
    let v2: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v2["schema_version"], 2);
    let loaded = engine::data::load_attributes_bytes(&bytes, "attributes-v2.json").unwrap();
    assert_eq!(loaded.converted_from, Some(2));
    let schema = &loaded.value;
    assert_eq!(
        schema.schema_version,
        engine::data::attributes::ATTRIBUTES_VERSION
    );
    assert!(schema.index("injury_resistance").is_none());
    let hidden: Vec<&str> = schema
        .attributes
        .iter()
        .filter(|d| d.hidden && !d.is_personality())
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(hidden, ["injury_proneness", "consistency"]);
    let v3 = std::fs::read(common::fixture_path("attributes-v3.json")).unwrap();
    let v3 = engine::data::load_attributes_bytes(&v3, "attributes-v3.json").unwrap();
    assert_eq!(schema, &v3.value);
}

// --- The version 4 record: counts, groups, jobs, the version 3 conversion, the traits.

/// The shipped catalogue as JSON, edited by `edit`, loaded.
fn edited_catalogue(
    edit: impl FnOnce(&mut serde_json::Value),
) -> Result<AttributeSchema, EngineError> {
    let path = common::content_dir().path("attributes.json");
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    edit(&mut v);
    engine::data::load_attributes_bytes(&serde_json::to_vec(&v).unwrap(), "attributes.json")
        .map(|l| l.value)
}

fn def_of<'a>(v: &'a mut serde_json::Value, name: &str) -> &'a mut serde_json::Value {
    v["attributes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["name"] == name)
        .unwrap()
}

/// `count` copies of the hidden consistency value under new names: each counts toward the 50
/// and, with the spread for its job, needs no stage table.
fn add_copies(v: &mut serde_json::Value, count: usize) {
    let consistency = def_of(v, "consistency").clone();
    let list = v["attributes"].as_array_mut().unwrap();
    for i in 0..count {
        let mut copy = consistency.clone();
        copy["name"] = format!("consistency_copy_{i}").into();
        list.push(copy);
    }
}

/// Shape AC-1: 45 visible and 2 hidden attributes, 11 personality values; communication,
/// determination and bravery in the mental group, eccentricity in goalkeeping.
#[test]
fn the_shipped_catalogue_holds_forty_five_visible_two_hidden_and_eleven_personality_values() {
    use engine::data::attributes::Group;
    let content = common::content();
    let s = &content.attributes;
    assert_eq!(s.schema_version, ATTRIBUTES_VERSION);
    assert_eq!(ATTRIBUTES_VERSION, 4);
    let visible = s.attributes.iter().filter(|d| !d.hidden).count();
    let hidden = s
        .attributes
        .iter()
        .filter(|d| d.hidden && !d.is_personality())
        .count();
    let personality: Vec<&str> = s
        .attributes
        .iter()
        .filter(|d| d.is_personality())
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!((visible, hidden, personality.len()), (45, 2, 11));
    assert!(visible + hidden <= 50);
    assert!(s.len() <= engine::data::MAX_ATTRIBUTES);
    let group = |n: &str| s.attributes[s.index(n).unwrap()].group;
    for n in ["communication", "determination", "bravery"] {
        assert_eq!(group(n), Group::Mental, "{n}");
    }
    assert_eq!(group("eccentricity"), Group::Goalkeeping);
    assert_eq!(
        personality,
        [
            "professionalism",
            "temperament",
            "pressure_handling",
            "sportsmanship",
            "leadership",
            "adaptability",
            "versatility",
            "ambition",
            "loyalty",
            "money_motivation",
            "controversy"
        ]
    );
    for d in s.attributes.iter().filter(|d| d.is_personality()) {
        assert!(d.hidden, "{}", d.name);
    }
    // The hidden-name list carries the personality values too, so no page shows them.
    let hidden_names: Vec<&str> = s.hidden_names().collect();
    assert_eq!(hidden_names.len(), 13);
    assert!(hidden_names.contains(&"temperament"));
}

/// Shape AC-2: 51 visible and hidden attributes are refused with the count; 50 load.
#[test]
fn a_catalogue_with_fifty_one_visible_and_hidden_attributes_is_refused_with_the_count() {
    let err = edited_catalogue(|v| add_copies(v, 4))
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("51 visible and hidden attributes; at most 50"),
        "{err}"
    );
    let fifty = edited_catalogue(|v| add_copies(v, 3)).unwrap();
    assert_eq!(fifty.len(), 50 + 11);
}

/// A fifteenth personality value is refused with the count.
#[test]
fn a_fifteenth_personality_value_is_refused() {
    let err = edited_catalogue(|v| {
        let loyalty = def_of(v, "loyalty").clone();
        let list = v["attributes"].as_array_mut().unwrap();
        for i in 0..4 {
            let mut copy = loyalty.clone();
            copy["name"] = format!("character_{i}").into();
            list.push(copy);
        }
    })
    .unwrap_err()
    .to_string();
    assert!(err.contains("15 personality values; at most 14"), "{err}");
}

/// Shape AC-9: every attribute added in version 4 names a stage job, and every personality
/// value an occasion job or "inert in a match".
#[test]
fn every_new_attribute_names_a_stage_job_and_every_personality_value_an_occasion() {
    use engine::data::attributes::JobStage;
    use engine::data::convert::V4_ADDED;
    let content = common::content();
    let s = &content.attributes;
    for name in V4_ADDED {
        let def = &s.attributes[s.index(name).unwrap_or_else(|| panic!("{name} missing"))];
        if def.is_personality() {
            assert!(def.job.action.is_none(), "{name}");
            match def.job.stage {
                JobStage::Occasion => assert!(!def.job.statistic.is_empty(), "{name}"),
                JobStage::Inert => assert_eq!(def.job.statistic, "inert in a match", "{name}"),
                other => panic!("{name}: {other:?}"),
            }
        } else {
            let stage = def
                .job
                .stage
                .stage()
                .unwrap_or_else(|| panic!("{name}: no stage"));
            let action = def
                .job
                .action
                .unwrap_or_else(|| panic!("{name}: no action"));
            assert!(s.in_stage(name, action, stage), "{name}");
        }
    }
    let inert: Vec<&str> = s
        .attributes
        .iter()
        .filter(|d| d.job.stage == JobStage::Inert)
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(inert, ["loyalty", "money_motivation", "controversy"]);
}

/// A personality value given a stage job, an action, a table place, or `hidden: false` is
/// refused, naming it.
#[test]
fn a_personality_value_outside_its_rules_is_refused_by_name() {
    let refusal = |edit: &dyn Fn(&mut serde_json::Value)| {
        edited_catalogue(|v| edit(v)).unwrap_err().to_string()
    };
    let err = refusal(&|v| {
        let d = def_of(v, "temperament");
        d["job"]["stage"] = "execute".into();
        d["job"]["action"] = "tackle".into();
    });
    assert!(err.contains("personality value temperament"), "{err}");
    let err = refusal(&|v| def_of(v, "temperament")["job"]["action"] = "tackle".into());
    assert!(
        err.contains("personality value temperament: it acts on an occasion"),
        "{err}"
    );
    let err = refusal(&|v| {
        v["actions"]["tackle"]["choose"]["minor"] =
            serde_json::json!([{ "attribute": "temperament", "weight": 0.05 }]);
    });
    assert!(
        err.contains("personality value temperament: it is in a stage table"),
        "{err}"
    );
    let err = refusal(&|v| def_of(v, "temperament")["hidden"] = false.into());
    assert!(
        err.contains("personality value temperament: a personality value is hidden"),
        "{err}"
    );
    let err = refusal(&|v| def_of(v, "loyalty")["job"]["statistic"] = "transfer requests".into());
    assert!(
        err.contains("personality value loyalty: an inert job"),
        "{err}"
    );
}

/// The version 3 file the build shipped before converts to version 4 with the same names,
/// groups, jobs, tables, consistency degrees and gates as the shipped file; only the order of
/// the attributes differs, as the new ones are appended.
#[test]
fn the_version_3_catalogue_converts_to_the_shipped_version_4() {
    let bytes = std::fs::read(common::fixture_path("attributes-v3.json")).unwrap();
    let v3: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v3["schema_version"], 3);
    let loaded = engine::data::load_attributes_bytes(&bytes, "attributes-v3.json").unwrap();
    assert_eq!(loaded.converted_from, Some(3));
    let converted = &loaded.value;
    let content = common::content();
    let shipped = &content.attributes;
    assert_eq!(converted.schema_version, shipped.schema_version);
    let by_name = |s: &AttributeSchema| {
        let mut defs = s.attributes.clone();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    };
    assert_eq!(by_name(converted), by_name(shipped));
    assert_eq!(converted.actions, shipped.actions);
    assert_eq!(converted.consistency_degrees, shipped.consistency_degrees);
    assert_eq!(converted.gates, shipped.gates);
}

/// Every action with a table has a consistency degree, 1.0 in this build.
#[test]
fn every_action_has_a_consistency_degree_of_one() {
    let content = common::content();
    let degrees = &content.attributes.consistency_degrees;
    assert_eq!(degrees.len(), engine::contract::ACTION_COUNT);
    assert!(degrees.values().all(|&d| d == 1.0), "{degrees:?}");
}

/// The trait catalogue loads with its 31 traits, nine opposite pairs named from both sides,
/// and a hook each; a broken opposite is refused naming the trait.
#[test]
fn the_trait_catalogue_loads_with_thirty_one_traits() {
    use engine::data::traits::load_traits_bytes;
    let path = common::content_dir().path("traits.json");
    let bytes = std::fs::read(path).unwrap();
    let c = load_traits_bytes(&bytes, "traits.json").unwrap().value;
    assert_eq!(c.traits.len(), 31);
    let pairs = c.traits.iter().filter(|t| t.opposite.is_some()).count() / 2;
    assert_eq!(pairs, 9);
    assert!(c.traits.iter().all(|t| !t.hook.is_empty()));
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for t in v["traits"].as_array_mut().unwrap() {
        if t["name"] == "kicks_long" {
            t["opposite"] = "plays_out".into();
        }
    }
    let err = load_traits_bytes(&serde_json::to_vec(&v).unwrap(), "traits.json")
        .unwrap_err()
        .to_string();
    assert!(err.contains("trait kicks_long"), "{err}");
}

/// A version 4 tuning file (no consistency, hidden-value or match-rating block) converts to
/// version 5 with the values of the frozen copy, which are the shipped ones.
#[test]
fn a_version_4_tuning_file_converts_to_the_shipped_version_5() {
    let mut v4 = shipped_tuning_json();
    v4["schema_version"] = 4.into();
    v4["engine"]["contract"]
        .as_object_mut()
        .unwrap()
        .remove("consistency")
        .unwrap();
    let top = v4.as_object_mut().unwrap();
    top.remove("hidden").unwrap();
    top.remove("match_rating").unwrap();
    v4["generator"]["world"]
        .as_object_mut()
        .unwrap()
        .remove("hidden")
        .unwrap();
    let bytes = serde_json::to_vec(&v4).unwrap();
    let loaded = engine::data::load_tuning_bytes(&bytes, "tuning-v4.json").unwrap();
    assert_eq!(loaded.converted_from, Some(4));
    let shipped = engine::data::load_tuning_bytes(
        &serde_json::to_vec(&shipped_tuning_json()).unwrap(),
        "tuning.json",
    )
    .unwrap();
    assert_eq!(loaded.value, shipped.value);
}

/// A hidden value's word bands must run from the highest down and end at 1.0, so every rating
/// has a word; a list that does not is refused naming the attribute.
#[test]
fn word_bands_out_of_order_or_short_of_one_are_refused_by_name() {
    let mut unordered = shipped_tuning_json();
    let bands = unordered["hidden"]["words"]["consistency"]
        .as_array_mut()
        .unwrap();
    bands.swap(0, 1);
    let err = engine::data::load_tuning_bytes(&serde_json::to_vec(&unordered).unwrap(), "t.json")
        .unwrap_err()
        .to_string();
    assert!(err.contains("consistency"), "{err}");

    let mut short = shipped_tuning_json();
    let bands = short["hidden"]["words"]["injury_proneness"]
        .as_array_mut()
        .unwrap();
    bands.last_mut().unwrap()["from"] = 3.0.into();
    let err = engine::data::load_tuning_bytes(&serde_json::to_vec(&short).unwrap(), "t.json")
        .unwrap_err()
        .to_string();
    assert!(err.contains("injury_proneness"), "{err}");
}

/// The shipped tactics file as JSON.
fn shipped_tactics_json() -> serde_json::Value {
    let path = common::content_dir().path(engine::data::TACTICS_FILE);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// The names that would set a gap between lines or a share of the slide.
const PLANTED: [&str; 3] = ["line_gap", "target_gap", "slide_share"];

/// No content field sets a target gap between the lines or a share of the slide: the line
/// gaps and the block's length come from where each role stands and the slide. A tactics
/// file with such a field at the top level, in a mentality, a role, a formation or the stand
/// table, and a tuning file with one in its engine block, are each refused by name.
#[test]
fn a_content_file_that_sets_a_line_gap_or_a_slide_share_is_refused() {
    type Place = fn(&mut serde_json::Value) -> &mut serde_json::Value;
    let places: [(&str, Place); 5] = [
        ("top level", |v| v),
        ("a mentality", |v| &mut v["mentalities"][0]),
        ("a role", |v| &mut v["roles"][0]),
        ("a formation", |v| &mut v["formations"][0]),
        ("the stand table", |v| &mut v["stands"]),
    ];
    for name in PLANTED {
        for (place, at) in places {
            let mut v = shipped_tactics_json();
            at(&mut v)
                .as_object_mut()
                .unwrap()
                .insert(name.into(), 12.into());
            let err = engine::data::load_tactics_bytes(&serde_json::to_vec(&v).unwrap(), "t.json")
                .err()
                .unwrap_or_else(|| panic!("{name} in {place} loaded"))
                .to_string();
            assert!(err.contains(name), "{name} in {place}: {err}");
        }
    }
    for name in ["slide_share", "line_gap_m"] {
        let mut v = shipped_tuning_json();
        v["engine"]
            .as_object_mut()
            .unwrap()
            .insert(name.into(), 0.5.into());
        let err = engine::data::load_tuning_bytes(&serde_json::to_vec(&v).unwrap(), "t.json")
            .err()
            .unwrap_or_else(|| panic!("{name} in the engine block loaded"))
            .to_string();
        assert!(err.contains(name), "{name}: {err}");
    }
    // The shipped files load.
    let shipped = serde_json::to_vec(&shipped_tactics_json()).unwrap();
    assert!(engine::data::load_tactics_bytes(&shipped, "tactics.json").is_ok());
}

/// The keys allowed to name a gap or a share, each with why it sets no gap between lines
/// and no share of the slide.
const ALLOWED: [(&str, &str); 3] = [
    (
        "back_line_gap",
        "the widest gap between neighbours inside the deepest line, a ceiling for the respread when a player leaves",
    ),
    (
        "drop_gap_m",
        "how far the carrier must be behind a player before he drops, a trigger distance",
    ),
    (
        "foreign_share",
        "the share of foreign players the club generator draws, not a match value",
    ),
];

/// Every key of `v`, at any depth, whose name holds `gap`, `share` or `slide` and is not
/// allowed, with its path.
fn gap_or_share_keys(v: &serde_json::Value, path: &str, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let here = format!("{path}.{key}");
                let named = ["gap", "share", "slide"].iter().any(|w| key.contains(w));
                if named && !ALLOWED.iter().any(|(allowed, _)| allowed == key) {
                    out.push(here.clone());
                }
                gap_or_share_keys(value, &here, out);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                gap_or_share_keys(item, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

/// The shipped tactics and tuning files hold no key that names a gap or a share beyond the
/// allowed ones; the scan reports a planted `midfield_gap`, so it is proven able to fail.
#[test]
fn no_shipped_key_names_a_line_gap_or_a_slide_share() {
    for (file, v) in [
        ("tactics.json", shipped_tactics_json()),
        ("tuning.json", shipped_tuning_json()),
    ] {
        let mut found = Vec::new();
        gap_or_share_keys(&v, file, &mut found);
        assert!(found.is_empty(), "{found:?}");
    }
    let mut planted = shipped_tactics_json();
    planted["roles"][2]
        .as_object_mut()
        .unwrap()
        .insert("midfield_gap".into(), 12.into());
    let mut found = Vec::new();
    gap_or_share_keys(&planted, "tactics.json", &mut found);
    assert_eq!(found, ["tactics.json.roles[2].midfield_gap"]);
}
