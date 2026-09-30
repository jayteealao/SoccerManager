//! The module contract: the shipped slot file resolves every declared slot and a match
//! finishes on it; a bad slot entry refuses start-up and names the slot, the bad value, and
//! the valid names; every moved action key has exactly one owner; every module card is
//! complete; one changed module output fails the replay gate at a named window; every
//! modifier names its family and can be switched off; the required slots of the core list
//! refuse `off` while the optional slots play a match to the end switched off; every row of
//! the stream table has an owner; and the three hook slots are optional, their adapters own
//! no key, and a match with each switched off plays to the end.

/// The four modifier slots, in registry order.
const MODIFIER_SLOTS: [&str; 4] = [
    MODIFIER_FATIGUE.id,
    MODIFIER_PRESSURE.id,
    MODIFIER_MOMENTUM.id,
    MODIFIER_WEATHER.id,
];

mod common;

use std::collections::BTreeMap;

use engine::modules::modifier::Family;
use engine::modules::registry::{
    BALL, CHANGES, CLOCK, DECISION, DISCIPLINE, FATIGUE, FOULS, HOOK_COMMENTARY, HOOK_DECISION,
    HOOK_RULE, INJURIES, MANAGER, MODIFIER_FATIGUE, MODIFIER_MOMENTUM, MODIFIER_PRESSURE,
    MODIFIER_WEATHER, ModuleRef, OFFSIDE, PEOPLE, POSSESSION, PRE_MATCH, PRESENTATION, RESTARTS,
    RULES, SEASON, SHOT, SKIN, STEERING, WORLD,
};
use engine::modules::{
    CardError, MOVED_KEYS, ModuleCard, OwnershipError, REGISTRY, Registration, SlotDecl, SlotEntry,
    SlotFile, check_card, check_ownership, resolve,
};
use engine::streams::Action;
use engine::{EngineError, MatchConfig, NullSink, Simulation, Snapshot, Validator, VecSink};

fn file(entries: &[(&str, &str, Option<u32>)]) -> SlotFile {
    SlotFile {
        schema_version: 1,
        slots: entries
            .iter()
            .map(|(slot, module, version)| {
                (
                    (*slot).to_string(),
                    SlotEntry {
                        module: (*module).to_string(),
                        version: *version,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
}

/// The built-in default selection with each `(slot, module, version)` of `changes` put in.
fn default_with(changes: &[(&str, &str, Option<u32>)]) -> SlotFile {
    let mut slots = SlotFile::builtin_default();
    for (slot, module, version) in changes {
        slots.slots.insert(
            (*slot).to_string(),
            SlotEntry {
                module: (*module).to_string(),
                version: *version,
            },
        );
    }
    slots
}

/// The refusal for `slots`, as its parts and its message.
fn refusal(slots: &SlotFile, registry: &[SlotDecl]) -> (String, String, String, String) {
    let err = resolve(slots, registry).expect_err("the slot file is refused");
    let text = err.to_string();
    let EngineError::SlotRefused {
        slot, value, valid, ..
    } = err
    else {
        panic!("not a slot refusal: {text}");
    };
    (slot, value, valid, text)
}

fn band_names() -> Vec<String> {
    let text = std::fs::read_to_string(common::content_dir().path("realism-bands.json")).unwrap();
    let bands: serde_json::Value = serde_json::from_str(&text).unwrap();
    bands
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, v)| v.is_object())
        .map(|(k, _)| k.clone())
        .collect()
}

fn every_registration() -> Vec<Registration> {
    REGISTRY
        .iter()
        .flat_map(|d| d.registrations.iter().copied().chain(d.off))
        .collect()
}

/// The default registration of each slot, with the name the ownership check reports.
fn default_cards() -> Vec<(&'static str, &'static ModuleCard)> {
    REGISTRY
        .iter()
        .map(|d| (d.slot.id, d.registrations[0].card))
        .collect()
}

#[test]
fn default_selection_resolves_and_a_match_finishes() {
    let content = common::content();
    let picked: Vec<String> = content
        .modules
        .picked()
        .iter()
        .map(|p| format!("{}={}@{}", p.slot, p.module, p.version))
        .collect();
    assert_eq!(
        picked,
        [
            "engine.fouls=fouls@1",
            "engine.offside=offside@1",
            "engine.shot=shot@1",
            "engine.fatigue=fatigue@1",
            "engine.steering=steering@1",
            "engine.pre-match=pre-match@1",
            "engine.modifier.fatigue=fatigue-curve@1",
            "engine.modifier.pressure=pressure@1",
            "engine.modifier.momentum=momentum@1",
            "engine.modifier.weather=weather@1",
            "engine.clock=clock@1",
            "engine.restarts=restarts@1",
            "engine.discipline=discipline@1",
            "engine.injuries=injuries@1",
            "engine.ball=ball@1",
            "engine.possession=possession@1",
            "engine.decision=decision@1",
            "engine.manager=ai-manager@1",
            "engine.changes=changes@1",
            "engine.hook.decision=decision-hook@1",
            "engine.hook.rule=rule-hook@1",
            "engine.hook.commentary=commentary-hook@1",
            "game.rules=rule-pack@1",
            "game.world=world-stub@1",
            "game.season=season-stub@1",
            "game.people=people-stub@1",
            "game.presentation=presentation-stub@1",
            "viewer.skin=broadcast-blue@1",
        ],
        "every declared slot resolves, in registry order"
    );
    assert_eq!(picked.len(), REGISTRY.len());
    let [a, b] = common::default_teams(&content);
    let config = MatchConfig::new(common::SEED, 90, &content, [&a, &b]).unwrap();
    assert_eq!(config.modules, content.modules);
    let mut sim = Simulation::new(config).unwrap();
    sim.run(&mut NullSink).unwrap();
    assert!(sim.is_over(), "the match reaches full time");
}

#[test]
fn the_builtin_default_equals_the_shipped_slot_file() {
    let text = std::fs::read_to_string(common::content_dir().path("slots.json")).unwrap();
    let shipped: SlotFile = serde_json::from_str(&text).unwrap();
    assert_eq!(shipped, SlotFile::builtin_default());
    assert_eq!(
        engine::modules::ResolvedModules::builtin_default(),
        common::content().modules
    );
}

#[test]
fn an_unknown_module_is_refused_with_the_valid_names() {
    let (slot, value, valid, text) = refusal(
        &default_with(&[("engine.offside", "ofside", Some(1))]),
        REGISTRY,
    );
    assert_eq!(
        (slot.as_str(), value.as_str()),
        ("engine.offside", "ofside")
    );
    assert!(valid.starts_with("offside@1"), "{valid}");
    assert!(valid.ends_with("off"), "{valid}");
    assert!(
        text.contains("slot engine.offside: module \"ofside\" is not registered; valid: offside@1"),
        "{text}"
    );
}

#[test]
fn an_unbuilt_version_is_refused_with_the_valid_names() {
    let (slot, value, valid, text) = refusal(
        &default_with(&[("engine.fouls", "fouls", Some(2))]),
        REGISTRY,
    );
    assert_eq!((slot.as_str(), value.as_str()), ("engine.fouls", "fouls"));
    assert_eq!(valid, "fouls@1, off");
    assert_eq!(
        text,
        "slot configuration refused: slot engine.fouls: fouls version 2 is not built; valid: fouls@1, off"
    );
}

#[test]
fn an_empty_module_is_refused_with_the_valid_names() {
    let (slot, value, valid, text) =
        refusal(&default_with(&[("engine.fouls", "", Some(1))]), REGISTRY);
    assert_eq!((slot.as_str(), value.as_str()), ("engine.fouls", ""));
    assert_eq!(valid, "fouls@1, off");
    assert!(text.contains("the module name \"\" is empty"), "{text}");
}

#[test]
fn a_missing_version_is_refused() {
    let (slot, _, _, text) = refusal(&default_with(&[("engine.fouls", "fouls", None)]), REGISTRY);
    assert_eq!(slot, "engine.fouls");
    assert!(text.contains("has no version"), "{text}");
}

#[test]
fn off_on_a_required_slot_is_refused() {
    // Movement and steering is on the core list, so its slot is required.
    let (slot, value, valid, text) =
        refusal(&default_with(&[("engine.steering", "off", None)]), REGISTRY);
    assert_eq!((slot.as_str(), value.as_str()), ("engine.steering", "off"));
    assert_eq!(valid, "steering@1");
    assert_eq!(
        text,
        "slot configuration refused: slot engine.steering: off is not allowed: the slot is required; valid: steering@1"
    );
    // The clock and match end, restarts, ball physics, possession, and the decision maker
    // are on the core list too (AC-10). Test builds also list the faulty clock after
    // `clock@1` and the faulty possession module after `possession@1`.
    for (id, first) in [
        (CLOCK.id, "clock@1"),
        (RESTARTS.id, "restarts@1"),
        (BALL.id, "ball@1"),
        (POSSESSION.id, "possession@1"),
        (DECISION.id, "decision@1"),
    ] {
        let (slot, value, valid, text) = refusal(&default_with(&[(id, "off", None)]), REGISTRY);
        assert_eq!((slot.as_str(), value.as_str()), (id, "off"));
        assert!(valid.starts_with(first), "{valid}");
        assert!(!valid.contains("off"), "{valid}");
        assert!(
            text.contains(&format!(
                "slot {id}: off is not allowed: the slot is required; valid: {first}"
            )),
            "{text}"
        );
    }
}

#[test]
fn off_on_an_optional_slot_resolves_to_its_off_version() {
    let optional: Vec<&str> = REGISTRY
        .iter()
        .filter(|d| !d.slot.required)
        .map(|d| d.slot.id)
        .collect();
    let mut expected = vec![FOULS.id, OFFSIDE.id, SHOT.id, FATIGUE.id, PRE_MATCH.id];
    expected.extend(MODIFIER_SLOTS);
    expected.extend([
        DISCIPLINE.id,
        INJURIES.id,
        MANAGER.id,
        CHANGES.id,
        HOOK_DECISION.id,
        HOOK_RULE.id,
        HOOK_COMMENTARY.id,
        RULES.id,
        WORLD.id,
        SEASON.id,
        PEOPLE.id,
        PRESENTATION.id,
        SKIN.id,
    ]);
    assert_eq!(optional, expected);
    let changes: Vec<(&str, &str, Option<u32>)> =
        optional.iter().map(|&slot| (slot, "off", None)).collect();
    let modules = resolve(&default_with(&changes), REGISTRY).unwrap();
    for p in modules.picked() {
        let required = [
            (STEERING.id, "steering"),
            (CLOCK.id, "clock"),
            (RESTARTS.id, "restarts"),
            (BALL.id, "ball"),
            (POSSESSION.id, "possession"),
            (DECISION.id, "decision"),
        ];
        match required.iter().find(|(slot, _)| *slot == p.slot) {
            Some(&(_, module)) => assert_eq!((p.module, p.version), (module, 1)),
            None => assert_eq!((p.module, p.version), ("off", 0), "{}", p.slot),
        }
    }
}

#[test]
fn an_undeclared_or_missing_slot_is_refused() {
    let (slot, _, valid, text) = refusal(
        &default_with(&[("engine.weather", "sunny", Some(1))]),
        REGISTRY,
    );
    assert_eq!(slot, "engine.weather");
    assert_eq!(
        valid,
        "engine.fouls, engine.offside, engine.shot, engine.fatigue, engine.steering, \
         engine.pre-match, engine.modifier.fatigue, engine.modifier.pressure, \
         engine.modifier.momentum, engine.modifier.weather, engine.clock, engine.restarts, \
         engine.discipline, engine.injuries, engine.ball, engine.possession, engine.decision, \
         engine.manager, engine.changes, engine.hook.decision, engine.hook.rule, \
         engine.hook.commentary, game.rules, game.world, game.season, game.people, \
         game.presentation, viewer.skin"
    );
    assert!(text.contains("is not a declared slot"), "{text}");

    let (slot, value, valid, text) =
        refusal(&file(&[("engine.fouls", "fouls", Some(1))]), REGISTRY);
    assert_eq!((slot.as_str(), value.as_str()), ("engine.offside", ""));
    assert!(valid.starts_with("offside@1"), "{valid}");
    assert!(text.contains("missing from slots.json"), "{text}");
}

#[test]
fn every_moved_key_has_exactly_one_owner() {
    check_ownership(&default_cards(), MOVED_KEYS).unwrap();
    // Each off version owns no key outside its slot's default keys.
    for decl in REGISTRY {
        for reg in decl.registrations.iter().chain(decl.off.as_ref()) {
            for key in reg.card.keys {
                assert!(
                    decl.registrations[0].card.keys.contains(key),
                    "{}: {} claims {key:?}",
                    decl.slot.id,
                    reg.name
                );
            }
        }
    }
}

/// Every row of the stream table is a moved key, so every draw of a match has one owning
/// module.
#[test]
fn every_stream_row_has_an_owner() {
    for key in Action::ALL {
        assert!(MOVED_KEYS.contains(&key), "{key:?} has no owner");
    }
    assert_eq!(MOVED_KEYS.len(), Action::ALL.len());
    check_ownership(&default_cards(), &Action::ALL).unwrap();
}

#[test]
fn a_key_claimed_twice_or_unclaimed_fails_the_ownership_check() {
    let fouls = *REGISTRY[0].registrations[0].card;
    let thief = ModuleCard {
        keys: &[Action::Tackle],
        ..fouls
    };
    let twice = check_ownership(
        &[("engine.fouls", &fouls), ("engine.thief", &thief)],
        MOVED_KEYS,
    );
    assert_eq!(
        twice,
        Err(OwnershipError::ClaimedTwice {
            key: Action::Tackle,
            owners: vec!["engine.fouls", "engine.thief"],
        })
    );
    let partial = ModuleCard {
        keys: &[Action::Tackle],
        ..fouls
    };
    assert_eq!(
        check_ownership(&[("engine.fouls", &partial)], MOVED_KEYS),
        Err(OwnershipError::Unclaimed {
            key: Action::FoulCard
        })
    );
    // The shot card without its save key leaves `Save` with no owner.
    let shot = *REGISTRY[2].registrations[0].card;
    let no_save = ModuleCard {
        keys: &[Action::ShootoutSave],
        ..shot
    };
    let cards: Vec<(&str, &ModuleCard)> = default_cards()
        .into_iter()
        .map(|(slot, card)| (slot, if slot == SHOT.id { &no_save } else { card }))
        .collect();
    assert_eq!(
        check_ownership(&cards, MOVED_KEYS),
        Err(OwnershipError::Unclaimed { key: Action::Save })
    );
}

#[test]
fn every_registered_card_is_complete() {
    let names = band_names();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    assert!(names.contains(&"yellow_cards_per_team"), "{names:?}");
    assert!(names.contains(&"goals_per_xg"), "{names:?}");
    let registrations = every_registration();
    // Fouls, offside, shot, fatigue, pre-match, the four modifiers, discipline, injuries,
    // the manager, changes, and the three hook slots each have version 1 and off; steering,
    // the clock, restarts, ball physics, possession, and the decision maker have version 1
    // only.
    assert!(registrations.len() >= 40, "{}", registrations.len());
    for reg in registrations {
        check_card(reg.card, &names)
            .unwrap_or_else(|e| panic!("{}@{}: {e}", reg.name, reg.version));
    }
}

#[test]
fn an_incomplete_card_fails_the_card_check() {
    let names = band_names();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let good = *REGISTRY[0].registrations[0].card;
    let empty_purpose = ModuleCard {
        purpose: "",
        ..good
    };
    assert_eq!(
        check_card(&empty_purpose, &names),
        Err(CardError::Empty { field: "purpose" })
    );
    let bare_none = ModuleCard {
        calibration: "none:",
        ..good
    };
    assert!(matches!(
        check_card(&bare_none, &names),
        Err(CardError::Calibration { .. })
    ));
    let unknown_band = ModuleCard {
        calibration: "fouls_per_match",
        ..good
    };
    assert!(matches!(
        check_card(&unknown_band, &names),
        Err(CardError::Calibration { .. })
    ));
    let no_tuning = ModuleCard {
        tuning: &[],
        ..good
    };
    assert_eq!(
        check_card(&no_tuning, &names),
        Err(CardError::Empty { field: "tuning" })
    );
}

#[test]
fn the_registry_declares_the_moved_slots_in_order() {
    let ids: Vec<&str> = REGISTRY.iter().map(|d| d.slot.id).collect();
    let mut expected = vec![
        FOULS.id,
        OFFSIDE.id,
        SHOT.id,
        FATIGUE.id,
        STEERING.id,
        PRE_MATCH.id,
    ];
    expected.extend(MODIFIER_SLOTS);
    expected.extend([
        CLOCK.id,
        RESTARTS.id,
        DISCIPLINE.id,
        INJURIES.id,
        BALL.id,
        POSSESSION.id,
        DECISION.id,
        MANAGER.id,
        CHANGES.id,
        HOOK_DECISION.id,
        HOOK_RULE.id,
        HOOK_COMMENTARY.id,
        RULES.id,
        WORLD.id,
        SEASON.id,
        PEOPLE.id,
        PRESENTATION.id,
        SKIN.id,
    ]);
    assert_eq!(ids, expected);
    assert_eq!(REGISTRY.len(), engine::modules::SLOT_COUNT);
    let required: Vec<&str> = REGISTRY
        .iter()
        .filter(|d| d.slot.required)
        .map(|d| d.slot.id)
        .collect();
    assert_eq!(
        required,
        [
            STEERING.id,
            CLOCK.id,
            RESTARTS.id,
            BALL.id,
            POSSESSION.id,
            DECISION.id
        ],
        "the core list is required: steering, the clock, restarts, the ball, possession, and \
         the decision maker"
    );
    for decl in REGISTRY {
        assert_eq!(
            decl.off.is_none(),
            decl.slot.required,
            "{}: a slot has an off version exactly when it is optional",
            decl.slot.id
        );
    }
    let default = |i: usize| REGISTRY[i].registrations[0].module;
    assert!(matches!(default(0), ModuleRef::Fouls(_)));
    assert!(matches!(default(1), ModuleRef::Offside(_)));
    assert!(matches!(default(2), ModuleRef::Shot(_)));
    assert!(matches!(default(3), ModuleRef::Fatigue(_)));
    assert!(matches!(default(4), ModuleRef::Steering(_)));
    assert!(matches!(default(5), ModuleRef::PreMatch(_)));
    for i in 6..10 {
        assert!(matches!(default(i), ModuleRef::Modifier(_)), "{i}");
    }
    assert!(matches!(default(10), ModuleRef::Clock(_)));
    assert!(matches!(default(11), ModuleRef::Restarts(_)));
    assert!(matches!(default(12), ModuleRef::Discipline(_)));
    assert!(matches!(default(13), ModuleRef::Injuries(_)));
    assert!(matches!(default(14), ModuleRef::Ball(_)));
    assert!(matches!(default(15), ModuleRef::Possession(_)));
    assert!(matches!(default(16), ModuleRef::Decision(_)));
    assert!(matches!(default(17), ModuleRef::Manager(_)));
    assert!(matches!(default(18), ModuleRef::Changes(_)));
    assert!(matches!(default(19), ModuleRef::DecisionHook(_)));
    assert!(matches!(default(20), ModuleRef::RuleHook(_)));
    assert!(matches!(default(21), ModuleRef::CommentaryHook(_)));
    assert!(matches!(default(22), ModuleRef::Rules(_)));
    assert!(matches!(default(23), ModuleRef::World(_)));
    assert!(matches!(default(24), ModuleRef::Season(_)));
    assert!(matches!(default(25), ModuleRef::People(_)));
    assert!(matches!(default(26), ModuleRef::Presentation(_)));
    assert!(matches!(default(27), ModuleRef::Skin(_)));
    // The hook adapters, the game-wide modules, and the skins draw nothing, so none of their
    // cards owns an action key.
    for decl in &REGISTRY[19..] {
        for reg in decl.registrations.iter().chain(decl.off.as_ref()) {
            assert!(reg.card.keys.is_empty(), "{}: {}", decl.slot.id, reg.name);
        }
    }
}

#[test]
fn every_modifier_slot_names_its_family() {
    let family = |reg: &Registration| match reg.module {
        ModuleRef::Modifier(m) => m.family(),
        _ => panic!("{} is not a modifier", reg.name),
    };
    let expected = [
        (MODIFIER_FATIGUE.id, Family::Body),
        (MODIFIER_PRESSURE.id, Family::Mind),
        (MODIFIER_MOMENTUM.id, Family::Mind),
        (MODIFIER_WEATHER.id, Family::Surroundings),
    ];
    for (slot, want) in expected {
        let decl = REGISTRY.iter().find(|d| d.slot.id == slot).unwrap();
        let off = decl.off.expect("every modifier has an off version");
        for reg in decl.registrations.iter().chain([&off]) {
            assert_eq!(family(reg), want, "{slot}: {}@{}", reg.name, reg.version);
        }
    }
    // No modifier is in the familiarity family yet.
    let modifiers = REGISTRY
        .iter()
        .flat_map(|d| d.registrations.iter().copied().chain(d.off))
        .filter(|r| matches!(r.module, ModuleRef::Modifier(_)));
    assert_eq!(
        modifiers.clone().count(),
        8,
        "four modifiers, each with an off version"
    );
    assert!(
        modifiers
            .into_iter()
            .all(|r| family(&r) != Family::Familiarity)
    );
}

/// AC-6 for modifiers: each modifier switched off in turn, three full matches each; every
/// match reaches full time and the event validator accepts every event stream.
#[test]
fn each_modifier_switched_off_plays_a_batch_the_validator_accepts() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let selections: Vec<engine::Content> = MODIFIER_SLOTS
        .iter()
        .map(|&slot| {
            let mut off = content.clone();
            off.modules = resolve(&default_with(&[(slot, "off", None)]), REGISTRY).unwrap();
            assert_eq!(off.modules.picked_for(slot).unwrap().module, "off");
            off
        })
        .collect();
    let results = common::run_many(0..=11, |k| {
        let (slot, seed) = (MODIFIER_SLOTS[k as usize / 3], k % 3 + 1);
        let config = MatchConfig::new(seed, 90, &selections[k as usize / 3], [&a, &b]).unwrap();
        let mut sim = Simulation::new(config.clone()).unwrap();
        let mut sink = VecSink::default();
        sim.run(&mut sink).unwrap();
        let events = sim.take_events();
        let violations = Validator::for_match(config.tuning.clone(), sim.team_timeline(), &events)
            .check(&sink.records);
        (
            slot,
            seed,
            sim.is_over(),
            violations.len(),
            format!("{:?}", violations.first()),
        )
    });
    assert_eq!(results.len(), 12);
    for (slot, seed, over, violations, first) in results {
        assert!(over, "{slot} off, seed {seed}: the match reaches full time");
        assert_eq!(
            violations, 0,
            "{slot} off, seed {seed}: first violation {first}"
        );
    }
}

#[test]
fn faulty_offside_fails_the_gate_at_a_named_window() {
    use engine::gate::{self, Fixture, Inputs, Verdict, compare, report_line};

    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let fixture = Fixture::seed(42);
    let clean = gate::play_fixture(
        &fixture,
        &Inputs {
            content: &content,
            teams: [&a, &b],
            pack: None,
        },
    )
    .unwrap();
    let mut faulty = content.clone();
    faulty.modules = resolve(
        &default_with(&[("engine.offside", "offside-faulty", Some(1))]),
        REGISTRY,
    )
    .unwrap();
    let played = gate::play_fixture(
        &fixture,
        &Inputs {
            content: &faulty,
            teams: [&a, &b],
            pack: None,
        },
    )
    .unwrap();
    let verdict = compare(&clean.hashes, &played.hashes);
    let report = report_line(&fixture, &played, verdict);
    println!("{report}");
    let Verdict::Differs { from, to } = verdict else {
        panic!("the gate did not fail: {verdict:?}\n{report}");
    };
    assert!(from >= 30_000, "{report}");
    assert!(
        report.contains(&format!("between tick {from} and tick {to}")),
        "{report}"
    );
}

/// The slot file that switches `engine.fouls` off and keeps every other slot on its default.
const FOULS_OFF: &str = r#"{
  "schema_version": 1,
  "slots": {
    "engine.fouls": { "module": "off" },
    "engine.offside": { "module": "offside", "version": 1 },
    "engine.shot": { "module": "shot", "version": 1 },
    "engine.fatigue": { "module": "fatigue", "version": 1 },
    "engine.steering": { "module": "steering", "version": 1 },
    "engine.pre-match": { "module": "pre-match", "version": 1 },
    "engine.modifier.fatigue": { "module": "fatigue-curve", "version": 1 },
    "engine.modifier.pressure": { "module": "pressure", "version": 1 },
    "engine.modifier.momentum": { "module": "momentum", "version": 1 },
    "engine.modifier.weather": { "module": "weather", "version": 1 },
    "engine.clock": { "module": "clock", "version": 1 },
    "engine.restarts": { "module": "restarts", "version": 1 },
    "engine.discipline": { "module": "discipline", "version": 1 },
    "engine.injuries": { "module": "injuries", "version": 1 },
    "engine.ball": { "module": "ball", "version": 1 },
    "engine.possession": { "module": "possession", "version": 1 },
    "engine.decision": { "module": "decision", "version": 1 },
    "engine.manager": { "module": "ai-manager", "version": 1 },
    "engine.changes": { "module": "changes", "version": 1 },
    "engine.hook.decision": { "module": "decision-hook", "version": 1 },
    "engine.hook.rule": { "module": "rule-hook", "version": 1 },
    "engine.hook.commentary": { "module": "commentary-hook", "version": 1 },
    "game.rules": { "module": "rule-pack", "version": 1 },
    "game.world": { "module": "world-stub", "version": 1 },
    "game.season": { "module": "season-stub", "version": 1 },
    "game.people": { "module": "people-stub", "version": 1 },
    "game.presentation": { "module": "presentation-stub", "version": 1 },
    "viewer.skin": { "module": "broadcast-blue", "version": 1 }
  }
}"#;

#[test]
fn faulty_shot_fails_the_gate_at_a_named_window() {
    use engine::gate::{self, Fixture, Inputs, Verdict, compare, report_line};

    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let fixture = Fixture::seed(42);
    let play = |content: &engine::Content| {
        gate::play_fixture(
            &fixture,
            &Inputs {
                content,
                teams: [&a, &b],
                pack: None,
            },
        )
        .unwrap()
    };
    let clean = play(&content);
    // The control: the default selection plays the same hashes again.
    let again = play(&content);
    assert_eq!(compare(&clean.hashes, &again.hashes), Verdict::Same);

    let mut faulty = content.clone();
    faulty.modules = resolve(
        &default_with(&[("engine.shot", "shot-faulty", Some(1))]),
        REGISTRY,
    )
    .unwrap();
    let played = play(&faulty);
    let verdict = compare(&clean.hashes, &played.hashes);
    let report = report_line(&fixture, &played, verdict);
    println!("{report}");
    let Verdict::Differs { from, to } = verdict else {
        panic!("the gate did not fail: {verdict:?}\n{report}");
    };
    assert!(from >= 30_000, "{report}");
    assert!(
        report.contains(&format!("between tick {from} and tick {to}")),
        "{report}"
    );
}

/// AC-5 for the clock: one second more added time in the first half fails the gate in the
/// window that holds tick 135,000, where the first half's own 45 minutes end and its added
/// time is fixed, and the report names the window.
#[test]
fn faulty_clock_fails_the_gate_at_a_named_window() {
    use engine::gate::{self, Fixture, Inputs, Verdict, compare, report_line};

    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let fixture = Fixture::seed(42);
    let play = |content: &engine::Content| {
        gate::play_fixture(
            &fixture,
            &Inputs {
                content,
                teams: [&a, &b],
                pack: None,
            },
        )
        .unwrap()
    };
    let clean = play(&content);
    let mut faulty = content.clone();
    faulty.modules = resolve(
        &default_with(&[(CLOCK.id, "clock-faulty", Some(1))]),
        REGISTRY,
    )
    .unwrap();
    let played = play(&faulty);
    let verdict = compare(&clean.hashes, &played.hashes);
    let report = report_line(&fixture, &played, verdict);
    println!("{report}");
    let Verdict::Differs { from, to } = verdict else {
        panic!("the gate did not fail: {verdict:?}\n{report}");
    };
    assert!(from < 135_000 && 135_000 <= to, "{report}");
    assert!(
        report.contains(&format!("between tick {from} and tick {to}")),
        "{report}"
    );
}

/// AC-5 for possession: an outfield player who reaches a loose ball from 1 cm further fails
/// the gate at the first loose ball that centimetre decides (on seed 42, in the window from
/// tick 2,000 to tick 3,000), and the report names the window.
#[test]
fn faulty_possession_fails_the_gate_at_a_named_window() {
    use engine::gate::{self, Fixture, Inputs, Verdict, compare, report_line};

    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let fixture = Fixture::seed(42);
    let play = |content: &engine::Content| {
        gate::play_fixture(
            &fixture,
            &Inputs {
                content,
                teams: [&a, &b],
                pack: None,
            },
        )
        .unwrap()
    };
    let clean = play(&content);
    let mut faulty = content.clone();
    faulty.modules = resolve(
        &default_with(&[(POSSESSION.id, "possession-faulty", Some(1))]),
        REGISTRY,
    )
    .unwrap();
    let played = play(&faulty);
    let verdict = compare(&clean.hashes, &played.hashes);
    let report = report_line(&fixture, &played, verdict);
    println!("{report}");
    let Verdict::Differs { from, to } = verdict else {
        panic!("the gate did not fail: {verdict:?}\n{report}");
    };
    assert!(from >= 2_000, "{report}");
    assert!(
        report.contains(&format!("between tick {from} and tick {to}")),
        "{report}"
    );
}

#[test]
fn each_new_optional_slot_switched_off_plays_a_match_to_the_end() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    for slot in [
        SHOT.id,
        FATIGUE.id,
        PRE_MATCH.id,
        DISCIPLINE.id,
        INJURIES.id,
        MANAGER.id,
        CHANGES.id,
        HOOK_DECISION.id,
        HOOK_RULE.id,
        HOOK_COMMENTARY.id,
    ] {
        let mut off = content.clone();
        off.modules = resolve(&default_with(&[(slot, "off", None)]), REGISTRY).unwrap();
        assert_eq!(off.modules.picked_for(slot).unwrap().module, "off");
        let config = MatchConfig::new(common::SEED, 10, &off, [&a, &b]).unwrap();
        let mut sim = Simulation::new(config).unwrap();
        sim.run(&mut NullSink).unwrap();
        assert!(sim.is_over(), "{slot} off: the match reaches full time");
    }
}

#[test]
fn only_a_non_default_selection_changes_the_content_hash() {
    let shipped = std::fs::read(common::content_dir().path("slots.json")).unwrap();
    let content = common::content();
    let default = content.with_slots(&shipped).unwrap();
    // The shipped selection is the default: every hash stays as it was.
    assert_eq!(default.digest, content.digest);
    assert_eq!(default.hash(), content.hash());

    let off = content.with_slots(FOULS_OFF.as_bytes()).unwrap();
    assert_eq!(
        off.modules.picked_for("engine.fouls").unwrap().module,
        "off"
    );
    assert_ne!(off.digest, content.digest);
    assert_ne!(off.hash(), content.hash());

    // A flag state applied afterwards folds over the selection, and a match built from
    // either content carries the difference into its own hash.
    let flagged = off
        .with_flags(&engine::flags::FlagStates::default())
        .unwrap();
    assert_eq!(flagged.digest, off.digest);
    let [a, b] = common::default_teams(&content);
    let hash = |c: &engine::Content| MatchConfig::new(42, 90, c, [&a, &b]).unwrap().content_hash;
    assert_ne!(hash(&off), hash(&content));
    assert_eq!(hash(&default), hash(&content));
}

#[test]
fn a_bad_slot_file_is_refused_through_with_slots_too() {
    let bad = FOULS_OFF.replace("\"off\"", "\"ofs\"");
    let err = common::content().with_slots(bad.as_bytes()).unwrap_err();
    assert!(matches!(err, EngineError::SlotRefused { .. }), "{err}");
}

#[test]
fn a_snapshot_resumes_only_under_the_selection_it_was_written_with() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let config = |c: &engine::Content| MatchConfig::new(42, 90, c, [&a, &b]).unwrap();
    let snapshot = Snapshot::capture(
        &Simulation::new(config(&content)).unwrap(),
        [7; 16],
        1_700_000_000_000,
    );
    Simulation::from_snapshot(config(&content), &snapshot).expect("the same selection resumes");

    let off = content.with_slots(FOULS_OFF.as_bytes()).unwrap();
    match Simulation::from_snapshot(config(&off), &snapshot) {
        Err(EngineError::Snapshot { reason, .. }) => {
            assert!(reason.starts_with("content mismatch"), "{reason}");
        }
        Err(other) => panic!("refused for another reason: {other}"),
        Ok(_) => panic!("a snapshot resumed under another module selection"),
    }
}
