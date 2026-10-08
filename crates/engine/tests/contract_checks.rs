//! The whole-contract checks, run once every slot is on the contract: every optional slot
//! switched off in turn, and all of them at once, plays a batch the event validator accepts;
//! a stand-in in each slot leaves every action key it does not own drawing the same
//! sequence, and the events unchanged, while a changed module is caught by the same
//! comparison; and every stand-in keeps one owner per key and a complete card.

mod common;

use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use engine::modules::stand_in;
use engine::modules::{
    MOVED_KEYS, ModuleCard, REGISTRY, SlotDecl, SlotEntry, SlotFile, check_card, check_ownership,
    resolve,
};
use engine::record::{TickRecord, TickSink};
use engine::streams::Action;
use engine::trace::TraceRecord;
use engine::{EngineError, EngineEvent, MatchConfig, Simulation, Validator, VecSink};

/// The gate's seed, for the swap comparison.
const SWAP_SEED: u64 = 42;

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

/// The default registration of each slot, with the name the ownership check reports.
fn default_cards() -> Vec<(&'static str, &'static ModuleCard)> {
    REGISTRY
        .iter()
        .map(|d| (d.slot.id, d.registrations[0].card))
        .collect()
}

fn optional_slots() -> Vec<&'static str> {
    REGISTRY
        .iter()
        .filter(|d| !d.slot.required)
        .map(|d| d.slot.id)
        .collect()
}

/// Plays three full matches (seeds 1 to 3) per selection on every core and asserts that
/// each reaches full time and that the event validator accepts its stream.
fn assert_batch_accepted(selections: &[(String, engine::Content)]) {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let runs = selections.len() as u64 * 3;
    let results = common::run_many(0..=runs - 1, |k| {
        let (label, content) = &selections[k as usize / 3];
        let seed = k % 3 + 1;
        let config = MatchConfig::new(seed, 90, content, [&a, &b]).unwrap();
        let mut sim = Simulation::new(config.clone()).unwrap();
        let mut sink = VecSink::default();
        sim.run(&mut sink).unwrap();
        let events = sim.take_events();
        let violations = Validator::for_match(config.tuning.clone(), sim.team_timeline(), &events)
            .check(&sink.records);
        (
            label.clone(),
            seed,
            sim.is_over(),
            violations.len(),
            format!("{:?}", violations.first()),
        )
    });
    assert_eq!(results.len() as u64, runs);
    for (label, seed, over, violations, first) in results {
        assert!(over, "{label}, seed {seed}: the match reaches full time");
        assert_eq!(
            violations, 0,
            "{label}, seed {seed}: first violation {first}"
        );
    }
}

/// Each optional slot switched off in turn, three full matches each.
#[test]
fn every_optional_slot_switched_off_plays_a_batch_the_validator_accepts() {
    let content = common::content();
    let optional = optional_slots();
    assert!(optional.len() >= 16, "{optional:?}");
    let selections: Vec<(String, engine::Content)> = optional
        .iter()
        .map(|&slot| {
            let mut off = content.clone();
            off.modules = resolve(&default_with(&[(slot, "off", None)]), REGISTRY).unwrap();
            assert_eq!(off.modules.picked_for(slot).unwrap().module, "off");
            (format!("{slot} off"), off)
        })
        .collect();
    assert_batch_accepted(&selections);
}

/// The edge case of the off-switch check: every optional slot switched off at once still plays to the end.
#[test]
fn every_optional_slot_off_at_once_plays_a_batch_the_validator_accepts() {
    let optional = optional_slots();
    let changes: Vec<(&str, &str, Option<u32>)> =
        optional.iter().map(|&slot| (slot, "off", None)).collect();
    let mut off = common::content();
    off.modules = resolve(&default_with(&changes), REGISTRY).unwrap();
    for slot in &optional {
        assert_eq!(
            off.modules.picked_for(slot).unwrap().module,
            "off",
            "{slot}"
        );
    }
    assert_batch_accepted(&[("every optional slot off".to_string(), off)]);
}

/// What one match drew and did: per stream key, the draw count and a running digest of
/// `(tick, index, value)`, and the event list.
#[derive(Default)]
struct DrawDigest {
    keys: BTreeMap<u64, (Action, u16, u64, u64)>,
    draws: u64,
}

impl TickSink for DrawDigest {
    fn on_tick(&mut self, _record: &TickRecord) -> Result<(), EngineError> {
        Ok(())
    }

    fn on_trace(&mut self, records: &[TraceRecord]) -> Result<(), EngineError> {
        for record in records {
            let TraceRecord::Draw(d) = record else {
                continue;
            };
            self.draws += 1;
            let entry =
                self.keys
                    .entry(d.key.stream_id())
                    .or_insert((d.key.action, d.key.player.0, 0, 0));
            entry.2 += 1;
            let mut h = DefaultHasher::new();
            (entry.3, d.tick, d.index, d.value.to_bits()).hash(&mut h);
            entry.3 = h.finish();
        }
        Ok(())
    }
}

/// One traced 90-minute match on [`SWAP_SEED`] with `slot` resolved to `module@version`
/// against `registry`, or the default selection when `slot` is `None`.
fn traced(
    registry: &[SlotDecl],
    pick: Option<(&str, &str, u32)>,
) -> (bool, DrawDigest, Vec<EngineEvent>) {
    let mut content = common::content();
    let [a, b] = common::default_teams(&content);
    if let Some((slot, module, version)) = pick {
        content.modules = resolve(&default_with(&[(slot, module, Some(version))]), registry)
            .unwrap_or_else(|e| panic!("{slot}: {e}"));
        assert_eq!(content.modules.picked_for(slot).unwrap().module, module);
    }
    let config = MatchConfig::new(SWAP_SEED, 90, &content, [&a, &b]).unwrap();
    let mut sim = Simulation::new_traced(config).unwrap();
    assert!(
        sim.debug_trace_on(),
        "the swap comparison needs the debug trace"
    );
    let mut digest = DrawDigest::default();
    sim.run(&mut digest).unwrap();
    (sim.is_over(), digest, sim.take_events())
}

/// The first key outside `owned` whose draws differ between `default` and `other`, as its
/// action and player, or `None`.
fn first_difference(
    default: &DrawDigest,
    other: &DrawDigest,
    owned: &[Action],
) -> Option<(Action, u16)> {
    let ids: std::collections::BTreeSet<u64> = default
        .keys
        .keys()
        .chain(other.keys.keys())
        .copied()
        .collect();
    ids.into_iter().find_map(|id| {
        let (d, o) = (default.keys.get(&id), other.keys.get(&id));
        let (action, player) = d.or(o).map(|e| (e.0, e.1)).unwrap();
        let same = d.map(|e| (e.2, e.3)) == o.map(|e| (e.2, e.3));
        (!owned.contains(&action) && !same).then_some((action, player))
    })
}

/// A stand-in in each slot in turn; the match plays to the end, every key the stand-in
/// does not own draws the same sequence as with the default module, and the events match.
#[test]
fn a_stand_in_in_each_slot_keeps_every_other_key_and_the_events() {
    let (over, default, default_events) = traced(REGISTRY, None);
    assert!(over, "the default match reaches full time");
    assert!(default.draws > 0, "the default match records its draws");
    let results = common::run_many(0..=REGISTRY.len() as u64 - 1, |k| {
        let slot = REGISTRY[k as usize].slot.id;
        let stand_in = stand_in::registration(slot).unwrap();
        let registry = stand_in::registry_with(slot).unwrap();
        let (over, digest, events) = traced(&registry, Some((slot, stand_in.name, 1)));
        (
            slot,
            over,
            first_difference(&default, &digest, stand_in.card.keys),
            events == default_events,
        )
    });
    assert_eq!(results.len(), REGISTRY.len());
    for (slot, over, differs, same_events) in results {
        assert!(over, "{slot} stand-in: the match reaches full time");
        assert_eq!(differs, None, "{slot} stand-in: the first differing key");
        assert!(same_events, "{slot} stand-in: the events differ");
    }
}

/// The control of the stand-in check: the same comparison finds the faulty possession module, which
/// changes one output, so the stand-in passes above are not empty.
#[test]
fn the_swap_comparison_finds_a_changed_module() {
    let slot = "engine.possession";
    let faulty = REGISTRY
        .iter()
        .find(|d| d.slot.id == slot)
        .unwrap()
        .registrations
        .iter()
        .find(|r| r.name == "possession-faulty")
        .expect("test builds register possession-faulty@1");
    let (_, default, default_events) = traced(REGISTRY, None);
    let (over, digest, events) = traced(REGISTRY, Some((slot, faulty.name, faulty.version)));
    assert!(over, "the faulty match reaches full time");
    let differs = first_difference(&default, &digest, faulty.card.keys);
    assert!(
        differs.is_some(),
        "the comparison finds no key outside the possession keys that differs"
    );
    assert_ne!(events, default_events);
}

/// Key ownership with stand-ins: each stand-in card in place of its slot's default keeps exactly one
/// owner for every key, and every stand-in card is complete.
#[test]
fn every_stand_in_keeps_one_owner_per_key_and_a_complete_card() {
    let names = band_names();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    for decl in REGISTRY {
        let slot = decl.slot.id;
        let stand_in = stand_in::registration(slot).unwrap();
        check_card(stand_in.card, &names).unwrap_or_else(|e| panic!("{slot}: {e}"));
        let cards: Vec<(&str, &ModuleCard)> = default_cards()
            .into_iter()
            .map(|(id, card)| (id, if id == slot { stand_in.card } else { card }))
            .collect();
        check_ownership(&cards, MOVED_KEYS).unwrap_or_else(|e| panic!("{slot}: {e}"));
        check_ownership(&cards, &Action::ALL).unwrap_or_else(|e| panic!("{slot}: {e}"));
    }
}
