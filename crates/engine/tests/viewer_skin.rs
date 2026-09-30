//! The viewer's skin slot (`viewer.skin`): the shipped slot file picks `broadcast-blue@1`;
//! `interim-light@1` and `off` resolve; a skin never changes the content hash, so a save or
//! a replay does not split on a look change; an unknown skin is refused with the slot, the
//! value, and the valid names; and the registered skins are exactly the skin folders the
//! viewer project ships.

mod common;

use engine::modules::registry::SKIN;
use engine::modules::{ModuleRef, REGISTRY, SKIN_NAMES, SlotDecl, SlotEntry, SlotFile, check_card};

fn skin_bytes(module: &str, version: Option<u32>) -> Vec<u8> {
    let mut slots = SlotFile::builtin_default();
    slots.slots.insert(
        SKIN.id.to_string(),
        SlotEntry {
            module: module.to_string(),
            version,
        },
    );
    serde_json::to_vec(&slots).unwrap()
}

fn decl() -> &'static SlotDecl {
    REGISTRY
        .iter()
        .find(|d| d.slot.id == SKIN.id)
        .expect("the skin slot is declared")
}

fn skin_of(content: &engine::Content) -> &'static str {
    content.modules.skin.skin()
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

#[test]
fn the_default_skin_is_broadcast_blue_and_the_others_resolve() {
    let content = common::content();
    let picked = content.modules.picked_for(SKIN.id).unwrap();
    assert_eq!((picked.module, picked.version), ("broadcast-blue", 1));
    assert_eq!(skin_of(&content), "broadcast-blue");
    assert!(!decl().slot.required, "the skin slot is optional");

    let light = content
        .with_slots(&skin_bytes("interim-light", Some(1)))
        .unwrap();
    let picked = light.modules.picked_for(SKIN.id).unwrap();
    assert_eq!((picked.module, picked.version), ("interim-light", 1));
    assert_eq!(skin_of(&light), "interim-light");

    let off = content.with_slots(&skin_bytes("off", None)).unwrap();
    let picked = off.modules.picked_for(SKIN.id).unwrap();
    assert_eq!((picked.module, picked.version), ("off", 0));
    assert_eq!(
        skin_of(&off),
        "broadcast-blue",
        "off shows the built-in default look"
    );
}

#[test]
fn a_skin_never_changes_the_content_hash() {
    let content = common::content();
    for (module, version) in [("interim-light", Some(1)), ("off", None)] {
        let other = content.with_slots(&skin_bytes(module, version)).unwrap();
        assert_eq!(
            other.digest, content.digest,
            "{module}: the look is not the match"
        );
        assert_eq!(other.rules, content.rules, "{module}");
    }
    // A skin next to an engine change still folds only the engine change in: the hash equals
    // the same engine change under the default skin.
    let mut slots = SlotFile::builtin_default();
    slots.slots.insert(
        "engine.fouls".to_string(),
        SlotEntry {
            module: "off".to_string(),
            version: None,
        },
    );
    let fouls_off = content
        .with_slots(&serde_json::to_vec(&slots).unwrap())
        .unwrap();
    slots.slots.insert(
        SKIN.id.to_string(),
        SlotEntry {
            module: "interim-light".to_string(),
            version: Some(1),
        },
    );
    let fouls_off_light = content
        .with_slots(&serde_json::to_vec(&slots).unwrap())
        .unwrap();
    assert_ne!(fouls_off.digest, content.digest);
    assert_eq!(fouls_off_light.digest, fouls_off.digest);
}

/// AC-21: an unknown skin is refused with the slot, the bad value, and the valid names.
#[test]
fn an_unknown_skin_is_refused_with_the_valid_names() {
    let content = common::content();
    let err = content
        .with_slots(&skin_bytes("nope", Some(1)))
        .unwrap_err();
    let text = err.to_string();
    assert!(text.contains("viewer.skin"), "{text}");
    assert!(text.contains("nope"), "{text}");
    assert!(
        text.contains("broadcast-blue@1, interim-light@1, off"),
        "{text}"
    );
}

#[test]
fn every_skin_card_is_complete_and_owns_no_key() {
    let names = band_names();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let decl = decl();
    let off = decl.off.expect("the skin slot has an off version");
    assert_eq!(decl.registrations.len(), SKIN_NAMES.len());
    for (reg, name) in decl.registrations.iter().zip(SKIN_NAMES) {
        assert_eq!((reg.name, reg.version), (name, 1));
        let ModuleRef::Skin(m) = reg.module else {
            panic!("{} is not a skin", reg.name);
        };
        assert_eq!(m.skin(), name, "a skin names its own folder");
    }
    for reg in decl.registrations.iter().chain([&off]) {
        check_card(reg.card, &names).unwrap_or_else(|e| panic!("{}: {e}", reg.name));
        assert!(reg.card.keys.is_empty(), "{}: owns no key", reg.name);
    }
}
