//! The fast-model slot (`engine.fast-model`): the shipped slot file picks
//! `fitted-scores@1`, which only `fast_model::resolve` reaches; `off` refuses to play; the
//! slot never changes the content hash; an unknown module is refused with the valid names;
//! a team's strength is the mean attribute of the eleven it kicks off with; and the fast
//! model's event stream keeps the event-stream rules.

mod common;

use engine::modules::fast_model::{self, FastFit, FastParams, KickOff, MINUTES};
use engine::modules::registry::{FAST_MODEL, FOULS};
use engine::modules::{REGISTRY, SlotDecl, SlotEntry, SlotFile, check_card};
use engine::{EngineEventKind, MatchConfig, Validator};

fn slot_bytes(changes: &[(&str, &str, Option<u32>)]) -> Vec<u8> {
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
    serde_json::to_vec(&slots).unwrap()
}

fn decl() -> &'static SlotDecl {
    REGISTRY
        .iter()
        .find(|d| d.slot.id == FAST_MODEL.id)
        .expect("the fast-model slot is declared")
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

fn test_fit() -> FastFit {
    FastFit {
        params: FastParams {
            base: -0.28,
            home: 0.05,
            attack: 1.3,
            curve: 0.12,
            defence: -0.1,
            dispersion: 5.5,
            rho: -0.09,
            draw: 0.3,
        },
        minute_shares: vec![1.0 / MINUTES as f64; MINUTES],
    }
}

#[test]
fn the_shipped_slot_file_picks_the_fitted_model_and_off_refuses_to_play() {
    let content = common::content();
    let picked = content.modules.picked_for(FAST_MODEL.id).unwrap();
    assert_eq!((picked.module, picked.version), ("fitted-scores", 1));
    let ko = KickOff::even([50.0, 50.0]);
    let played = fast_model::resolve(&content.modules).play(&test_fit(), &ko, 3);
    assert!(played.is_ok(), "{played:?}");

    let off = content
        .with_slots(&slot_bytes(&[(FAST_MODEL.id, "off", None)]))
        .unwrap();
    assert_eq!(off.modules.picked_for(FAST_MODEL.id).unwrap().module, "off");
    let err = fast_model::resolve(&off.modules)
        .play(&test_fit(), &ko, 3)
        .unwrap_err()
        .to_string();
    assert!(err.contains("the fast-model slot is off"), "{err}");
}

/// The slot never plays a full-engine match, so switching it off moves no hash; switching
/// fouls off, the control, does.
#[test]
fn the_slot_never_changes_the_content_hash() {
    let content = common::content();
    let off = content
        .with_slots(&slot_bytes(&[(FAST_MODEL.id, "off", None)]))
        .unwrap();
    assert_eq!(off.digest, content.digest);
    let fouls_off = content
        .with_slots(&slot_bytes(&[(FOULS.id, "off", None)]))
        .unwrap();
    assert_ne!(fouls_off.digest, content.digest);
    let both = content
        .with_slots(&slot_bytes(&[
            (FOULS.id, "off", None),
            (FAST_MODEL.id, "off", None),
        ]))
        .unwrap();
    assert_eq!(both.digest, fouls_off.digest);
}

#[test]
fn an_unknown_fast_model_is_refused_with_the_valid_names() {
    let content = common::content();
    let bytes = std::fs::read(common::fixture_path("slots/fast-model-unknown.json")).unwrap();
    let text = content.with_slots(&bytes).unwrap_err().to_string();
    assert!(text.contains("engine.fast-model"), "{text}");
    assert!(text.contains("nope"), "{text}");
    assert!(text.contains("fitted-scores@1, off"), "{text}");
}

#[test]
fn both_cards_are_complete_and_own_no_key() {
    let names = band_names();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let decl = decl();
    assert!(!decl.slot.required);
    let off = decl.off.expect("the fast-model slot has an off version");
    for reg in decl.registrations.iter().chain([&off]) {
        check_card(reg.card, &names).unwrap_or_else(|e| panic!("{}: {e}", reg.name));
        assert!(reg.card.keys.is_empty(), "{}: owns no key", reg.name);
    }
}

/// A side's strength is the mean of every attribute of the eleven it starts with, so a team
/// with every attribute times 1.15 is stronger by about 15 percent.
#[test]
fn the_kick_off_strength_is_the_starting_elevens_mean_attribute() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let config = MatchConfig::new(1, 90, &content, [&a, &b]).unwrap();
    let ko = fast_model::kick_off(&config);
    let starters: Vec<_> = config.players.iter().filter(|p| p.team == 0).collect();
    assert_eq!(starters.len(), 11);
    let values: Vec<f64> = starters
        .iter()
        .flat_map(|p| p.attributes.iter().map(f64::from))
        .collect();
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    assert!((ko.strength[0] - mean).abs() < 1e-12);

    let strong = common::stronger(&a);
    let config = MatchConfig::new(1, 90, &content, [&strong, &b]).unwrap();
    let boosted = fast_model::kick_off(&config);
    let ratio = boosted.strength[0] / ko.strength[0];
    assert!((1.10..1.16).contains(&ratio), "{ratio}");
    assert!((boosted.strength[1] - ko.strength[1]).abs() < 1e-12);
}
/// AC-31, first half: 2 000 seeded matches across strengths keep every event-stream rule,
/// and the final score equals the goal events.
#[test]
fn the_fast_models_event_stream_keeps_the_rules() {
    let content = common::content();
    let model = fast_model::resolve(&content.modules);
    let fit = test_fit();
    for seed in 0..2_000u64 {
        let gap = (seed % 21) as f64 - 10.0;
        let ko = KickOff::even([50.0 + gap / 2.0, 50.0 - gap / 2.0]);
        let m = model.play(&fit, &ko, seed).unwrap();
        let violations = Validator::check_events(&m.events);
        assert!(violations.is_empty(), "seed {seed}: {violations:?}");
        let goals = |team: usize| {
            m.events
                .iter()
                .filter(|e| e.kind == EngineEventKind::Goal && e.team == Some(team))
                .count() as u32
        };
        assert_eq!([goals(0), goals(1)], m.scores, "seed {seed}");
    }
}
