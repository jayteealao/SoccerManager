//! The game-wide slots: the rule pack loads through the rules slot (`game.rules`), whose
//! off version plays under the standard Laws built into the program; and each stub slot
//! (`game.world`, `game.season`, `game.people`, `game.presentation`) resolves from the slot
//! file to its no-op default with a complete card, and switches off to a no-op off version.

mod common;

use engine::data::ContentFiles;
use engine::modules::game::GameChanges;
use engine::modules::registry::{PEOPLE, PRESENTATION, RULES, SEASON, WORLD};
use engine::modules::{
    GameDay, ModuleRef, REGISTRY, Registration, SlotDecl, SlotEntry, SlotFile, check_card,
};
use engine::{Content, EngineError, MatchConfig, Simulation, Validator, VecSink};

/// The built-in default selection with `slot` switched off, as slot file bytes.
fn off_bytes(slot: &str) -> Vec<u8> {
    let mut slots = SlotFile::builtin_default();
    slots.slots.insert(
        slot.to_string(),
        SlotEntry {
            module: "off".to_string(),
            version: None,
        },
    );
    serde_json::to_vec(&slots).unwrap()
}

fn default_bytes() -> Vec<u8> {
    serde_json::to_vec(&SlotFile::builtin_default()).unwrap()
}

fn band_names() -> Vec<String> {
    let text = std::fs::read_to_string(common::content_dir().path("realism-bands.json")).unwrap();
    let bands: serde_json::Value = serde_json::from_str(&text).unwrap();
    // Every band of the registry by name, and every group of bands (possession_pct) too.
    bands["bands"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|b| [b["band"].as_str(), b["group"].as_str()])
        .flatten()
        .map(str::to_string)
        .collect()
}

fn decl(slot: &str) -> &'static SlotDecl {
    REGISTRY
        .iter()
        .find(|d| d.slot.id == slot)
        .unwrap_or_else(|| panic!("{slot} is declared"))
}

/// The shipped content files with the rule file's text changed by `edit`.
fn files_with_rules(edit: impl Fn(&str) -> String) -> ContentFiles {
    let mut files = ContentFiles::read(&common::content_dir()).unwrap();
    let text = String::from_utf8(files.rules.clone()).unwrap();
    let edited = edit(&text);
    assert_ne!(edited, text, "the edit changes the rule file");
    files.rules = edited.into_bytes();
    files
}

/// The content folder's rule file reaches the match only through the rules slot. The
/// default module reads a changed file; the off version does not, and the selection changes
/// the content hash. A bad rule file is refused through the slot with the loader's message.
#[test]
fn the_rule_pack_loads_through_the_rules_slot() {
    let files = files_with_rules(|t| t.replace("\"half_minutes\": 45", "\"half_minutes\": 40"));
    let written = Content::from_files(&files).unwrap();
    let default = written.with_slots(&default_bytes()).unwrap();
    assert_eq!(
        default.modules.picked_for(RULES.id).unwrap().module,
        "rule-pack"
    );
    assert_eq!(default.rules.half_minutes, 40, "rule-pack@1 reads the file");
    assert_eq!(
        default.digest, written.digest,
        "the default selection keeps the hash"
    );

    let off = written.with_slots(&off_bytes(RULES.id)).unwrap();
    assert_eq!(off.modules.picked_for(RULES.id).unwrap().module, "off");
    assert_eq!(
        off.rules.half_minutes, 45,
        "the off version plays the built-in Laws"
    );
    assert_ne!(
        off.digest, default.digest,
        "the selection folds into the hash"
    );

    // Switching back reloads the written file through the default module.
    let back = off.with_slots(&default_bytes()).unwrap();
    assert_eq!(back.rules.half_minutes, 40);

    let bad = files_with_rules(|t| t.replace("\"halves\": 2", "\"halves\": 3"));
    let err = Content::from_files(&bad).expect_err("three halves are refused");
    let text = err.to_string();
    let EngineError::Data {
        kind, path, field, ..
    } = err
    else {
        panic!("not a content refusal: {text}");
    };
    assert_eq!((kind, path.as_str()), ("rules", "rules/default.json"));
    assert!(field.contains("halves"), "{field}");
}

/// The Laws built into the program are the shipped rule pack.
#[test]
fn the_built_in_laws_equal_the_shipped_rule_pack() {
    let content = common::content();
    let off = content.with_slots(&off_bytes(RULES.id)).unwrap();
    assert_eq!(off.rules, content.rules);
}

/// The off switch for the rules slot: switched off through the slot file, a full match on seed 1
/// reaches full time and the event validator accepts its stream.
#[test]
fn the_rules_slot_switched_off_plays_a_match_the_validator_accepts() {
    let content = common::content().with_slots(&off_bytes(RULES.id)).unwrap();
    let [a, b] = common::default_teams(&content);
    let config = MatchConfig::new(1, 90, &content, [&a, &b]).unwrap();
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut sink = VecSink::default();
    sim.run(&mut sink).unwrap();
    assert!(sim.is_over(), "the match reaches full time");
    let events = sim.take_events();
    let violations = Validator::for_match(config.tuning.clone(), sim.team_timeline(), &events)
        .check(&sink.records);
    assert!(violations.is_empty(), "{:?}", violations.first());
}

/// The one no-op entry point of a stub registration.
fn on_day(reg: &Registration, day: GameDay) -> GameChanges {
    match reg.module {
        ModuleRef::World(m) => m.on_day(day),
        ModuleRef::Season(m) => m.on_day(day),
        ModuleRef::People(m) => m.on_day(day),
        ModuleRef::Presentation(m) => m.on_day(day),
        _ => panic!("{} is not a game-wide stub", reg.name),
    }
}

/// Each stub slot resolves from the shipped slot file to `<name>-stub@1`, whose card
/// is complete and owns no key, and which proposes no change; `off` resolves to its off
/// version, which proposes no change either.
#[test]
fn each_stub_slot_resolves_to_its_no_op_default_with_a_complete_card() {
    let names = band_names();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let content = common::content();
    for (slot, name) in [
        (WORLD.id, "world-stub"),
        (SEASON.id, "season-stub"),
        (PEOPLE.id, "people-stub"),
        (PRESENTATION.id, "presentation-stub"),
    ] {
        let picked = content.modules.picked_for(slot).unwrap();
        assert_eq!((picked.module, picked.version), (name, 1), "{slot}");
        let decl = decl(slot);
        assert!(!decl.slot.required, "{slot} is optional");
        let v1 = decl.registrations[0];
        assert_eq!((v1.name, v1.version), (name, 1));
        let off = decl.off.expect("a stub slot has an off version");
        for reg in [&v1, &off] {
            check_card(reg.card, &names).unwrap_or_else(|e| panic!("{slot} {}: {e}", reg.name));
            assert!(reg.card.keys.is_empty(), "{slot} {}: owns no key", reg.name);
            assert_eq!(on_day(reg, GameDay(1)), GameChanges::default(), "{slot}");
        }

        let switched = content.with_slots(&off_bytes(slot)).unwrap();
        let picked = switched.modules.picked_for(slot).unwrap();
        assert_eq!((picked.module, picked.version), ("off", 0), "{slot}");
        assert_ne!(
            switched.digest, content.digest,
            "{slot}: the selection folds in"
        );
        assert_eq!(
            switched.rules, content.rules,
            "{slot}: the rule pack is unchanged"
        );
    }
    let resolved = content.modules;
    for m in [
        resolved.world.on_day(GameDay(7)),
        resolved.season.on_day(GameDay(7)),
        resolved.people.on_day(GameDay(7)),
        resolved.presentation.on_day(GameDay(7)),
    ] {
        assert_eq!(m, GameChanges::default());
    }
}

/// Both rules cards are complete and own no key.
#[test]
fn the_rules_slot_card_is_complete() {
    let names = band_names();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let decl = decl(RULES.id);
    assert!(!decl.slot.required, "the rules slot is optional");
    let v1 = decl.registrations[0];
    assert_eq!((v1.name, v1.version), ("rule-pack", 1));
    assert!(matches!(v1.module, ModuleRef::Rules(_)));
    let off = decl.off.expect("the rules slot has an off version");
    for reg in [&v1, &off] {
        check_card(reg.card, &names).unwrap_or_else(|e| panic!("{}: {e}", reg.name));
        assert!(reg.card.keys.is_empty(), "{}: owns no key", reg.name);
    }
}
