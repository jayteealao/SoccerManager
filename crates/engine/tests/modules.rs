//! The module contract: the shipped slot file resolves every declared slot and a match
//! finishes on it; a bad slot entry refuses start-up and names the slot, the bad value, and
//! the valid names; every moved action key has exactly one owner; every module card is
//! complete; and one changed module output fails the replay gate at a named window.

mod common;

use std::collections::BTreeMap;

use engine::modules::registry::{FOULS, ModuleRef, OFFSIDE};
use engine::modules::{
    CardError, MOVED_KEYS, ModuleCard, OwnershipError, REGISTRY, Registration, SlotDecl, SlotEntry,
    SlotFile, check_card, check_ownership, resolve,
};
use engine::streams::Action;
use engine::{EngineError, MatchConfig, NullSink, Simulation};

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
        ["engine.fouls=fouls@1", "engine.offside=offside@1"],
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
        &file(&[
            ("engine.fouls", "fouls", Some(1)),
            ("engine.offside", "ofside", Some(1)),
        ]),
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
        &file(&[
            ("engine.fouls", "fouls", Some(2)),
            ("engine.offside", "offside", Some(1)),
        ]),
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
    let (slot, value, valid, text) = refusal(
        &file(&[
            ("engine.fouls", "", Some(1)),
            ("engine.offside", "offside", Some(1)),
        ]),
        REGISTRY,
    );
    assert_eq!((slot.as_str(), value.as_str()), ("engine.fouls", ""));
    assert_eq!(valid, "fouls@1, off");
    assert!(text.contains("the module name \"\" is empty"), "{text}");
}

#[test]
fn a_missing_version_is_refused() {
    let (slot, _, _, text) = refusal(
        &file(&[
            ("engine.fouls", "fouls", None),
            ("engine.offside", "offside", Some(1)),
        ]),
        REGISTRY,
    );
    assert_eq!(slot, "engine.fouls");
    assert!(text.contains("has no version"), "{text}");
}

#[test]
fn off_on_a_required_slot_is_refused() {
    // The two slots declared so far are optional, so a fixture registry declares the fouls
    // slot required.
    let mut required = REGISTRY[0];
    required.slot.required = true;
    required.off = None;
    let registry = [required, REGISTRY[1]];
    let (slot, value, valid, text) = refusal(
        &file(&[
            ("engine.fouls", "off", None),
            ("engine.offside", "offside", Some(1)),
        ]),
        &registry,
    );
    assert_eq!((slot.as_str(), value.as_str()), ("engine.fouls", "off"));
    assert_eq!(valid, "fouls@1");
    assert!(
        text.contains("off is not allowed: the slot is required"),
        "{text}"
    );
}

#[test]
fn off_on_an_optional_slot_resolves_to_its_off_version() {
    let modules = resolve(
        &file(&[
            ("engine.fouls", "off", None),
            ("engine.offside", "off", None),
        ]),
        REGISTRY,
    )
    .unwrap();
    for p in modules.picked() {
        assert_eq!((p.module, p.version), ("off", 0), "{}", p.slot);
    }
}

#[test]
fn an_undeclared_or_missing_slot_is_refused() {
    let (slot, _, valid, text) = refusal(
        &file(&[
            ("engine.fouls", "fouls", Some(1)),
            ("engine.offside", "offside", Some(1)),
            ("engine.weather", "sunny", Some(1)),
        ]),
        REGISTRY,
    );
    assert_eq!(slot, "engine.weather");
    assert_eq!(valid, "engine.fouls, engine.offside");
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
}

#[test]
fn every_registered_card_is_complete() {
    let names = band_names();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    assert!(names.contains(&"yellow_cards_per_team"), "{names:?}");
    let registrations = every_registration();
    assert!(registrations.len() >= 4);
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
    assert_eq!(ids, [FOULS.id, OFFSIDE.id]);
    assert!(matches!(
        REGISTRY[0].registrations[0].module,
        ModuleRef::Fouls(_)
    ));
    assert!(matches!(
        REGISTRY[1].registrations[0].module,
        ModuleRef::Offside(_)
    ));
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
        &file(&[
            ("engine.fouls", "fouls", Some(1)),
            ("engine.offside", "offside-faulty", Some(1)),
        ]),
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
