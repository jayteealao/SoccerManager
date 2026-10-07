//! The fast-model slot (`engine.fast-model`): the shipped slot file picks
//! `fitted-scores@1`, which only `fast_model::resolve` reaches; `off` refuses to play; the
//! slot never changes the content hash; an unknown module is refused with the valid names;
//! a team's strength is the mean attribute of the eleven it kicks off with; and the fast
//! model's event stream keeps the event-stream rules.

mod common;

use engine::modules::fast_events::{EventFit, FitRules};
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
        events: EventFit::plain(FitRules::standard()),
    }
}

#[test]
fn the_shipped_slot_file_picks_the_fitted_model_and_off_refuses_to_play() {
    let content = common::content();
    let picked = content.modules.picked_for(FAST_MODEL.id).unwrap();
    assert_eq!((picked.module, picked.version), ("fitted-scores", 1));
    let ko = KickOff::even([10.0, 10.0]);
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

/// A side's strength is the level of the eleven it starts with, read through the attribute
/// contract: the rating whose curve value is the mean of each starter's mean curve value
/// over the stages of his role (the keeper's stages for the keeper in slot 0). A team with
/// every attribute times 1.15 is stronger; the other side does not move.
#[test]
fn the_kick_off_strength_is_the_starting_elevens_level_through_the_contract() {
    use engine::contract::curve::{f, f_inv};
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let config = MatchConfig::new(1, 90, &content, [&a, &b]).unwrap();
    let ko = fast_model::kick_off(&config);
    let curve = &config.tuning.contract.curve;
    let starters: Vec<_> = config.players.iter().filter(|p| p.team == 0).collect();
    assert_eq!(starters.len(), 11);
    let values: Vec<f64> = starters
        .iter()
        .map(|p| {
            let stages = &config.teams[0].squad[p.squad].stages;
            fast_model::role_curve_value(stages, p.slot == 0)
        })
        .collect();
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    assert!((ko.strength[0] - f_inv(mean, curve)).abs() < 1e-12);
    // A player rated 10 throughout has the level 10.
    assert!((f_inv(f(10.0, curve), curve) - 10.0).abs() < 1e-12);

    let strong = common::stronger(&a);
    let config = MatchConfig::new(1, 90, &content, [&strong, &b]).unwrap();
    let boosted = fast_model::kick_off(&config);
    let gain = boosted.strength[0] - ko.strength[0];
    assert!((0.5..4.0).contains(&gain), "{gain}");
    assert!(boosted.attack[0] > ko.attack[0] && boosted.defence[0] > ko.defence[0]);
    assert!((boosted.strength[1] - ko.strength[1]).abs() < 1e-12);
}

/// Each side's starters are the eleven the match kicks off with, slot for slot, with the
/// keeper in slot 0; its bench is the team's bench in order, a keeper flagged by position;
/// and six starters are advanced.
#[test]
fn the_kick_off_carries_both_line_ups_and_benches() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let config = MatchConfig::new(1, 90, &content, [&a, &b]).unwrap();
    let ko = fast_model::kick_off(&config);
    for team in 0..2 {
        let side = &ko.sides[team];
        let lineup: Vec<(usize, usize)> = config
            .players
            .iter()
            .filter(|p| p.team == team)
            .map(|p| (p.slot, p.squad))
            .collect();
        assert_eq!(lineup.len(), 11);
        for (slot, squad) in lineup {
            assert_eq!(side.starters[slot].squad, squad, "team {team} slot {slot}");
            assert_eq!(side.starters[slot].keeper, slot == 0);
        }
        let bench: Vec<usize> = side.bench.iter().map(|p| p.squad).collect();
        assert_eq!(bench, config.teams[team].bench, "team {team}");
        assert!(!bench.is_empty());
        let squad = &config.teams[team].squad;
        for p in &side.bench {
            assert_eq!(
                p.keeper,
                squad[p.squad].position == engine::data::Position::GK
            );
        }
        assert!(
            side.bench.iter().any(|p| p.keeper),
            "team {team} has a keeper"
        );
        assert_eq!(side.advanced.iter().filter(|a| **a).count(), 6);
        assert!(!side.advanced[0], "the keeper is not advanced");
        assert!(side.starters.iter().all(|p| p.foul > 0.0 && p.attack > 0.0));
    }
}

/// 2 000 seeded matches across strengths, half on the generated sides and half on the
/// default teams' line-ups, keep every event-stream rule; the final score equals the goal
/// events; and every kind the full engine emits in a regulation match shows, except a
/// plugin's script note and a refused change.
#[test]
fn the_fast_models_event_stream_keeps_the_rules() {
    use EngineEventKind::{ChangeRejected, Goal, Script};
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let config = MatchConfig::new(1, 90, &content, [&a, &b]).unwrap();
    let teams = fast_model::kick_off(&config);
    let model = fast_model::resolve(&content.modules);
    let fit = test_fit();
    let mut seen = std::collections::BTreeSet::new();
    let mut cards = std::collections::BTreeSet::new();
    for seed in 0..2_000u64 {
        let gap = (seed % 21) as f64 - 10.0;
        let mut ko = fast_model::KickOff::even([10.0 + gap / 10.0, 10.0 - gap / 10.0]);
        if seed % 2 == 1 {
            ko.sides = teams.sides.clone();
        }
        let m = model.play(&fit, &ko, seed).unwrap();
        let rules = fast_model::stream_rules(&fit, &ko);
        let violations = Validator::check_events(&m.events, &rules);
        assert!(violations.is_empty(), "seed {seed}: {violations:?}");
        let goals = |team: usize| {
            m.events
                .iter()
                .filter(|e| e.kind == Goal && e.team == Some(team))
                .count() as u32
        };
        assert_eq!([goals(0), goals(1)], m.scores, "seed {seed}");
        for e in &m.events {
            seen.insert(e.kind.code());
            if let Some(card) = e.card {
                cards.insert(format!("{card:?}"));
            }
        }
    }
    let regulation = EngineEventKind::ALL
        .iter()
        .filter(|k| !matches!(k, Script | ChangeRejected))
        .map(|k| k.code())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(seen, regulation);
    assert_eq!(cards.len(), 3, "{cards:?}");
}
