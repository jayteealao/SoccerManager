//! A script pack that overrides the decision scoring hook changes the seeded match, the match
//! stays reproducible with the pack, and a match without a pack is untouched. The Rhai
//! adapter runs through the engine's hook slots: with the three slots switched off, a pack is
//! never called, and the adapter's module card is complete.

mod common;

use std::path::Path;

use common::{config, pack, play};
use engine::math::{DVec2, DVec3};
use engine::modules::check_card;
use engine::scenario::Scene;
use engine::{EngineEventKind, Plugins, Simulation, VecSink};
use script::LoadedPack;
use script::hooks::RHAI_ADAPTER_CARD;

// Over a whole match the pack's bias is lost in the match's own variance: across seeds it
// takes more shots in some matches and fewer in others. The shot choice itself is pinned by
// the scene test below; this test pins that the pack runs, and changes the seeded match.
#[test]
fn the_shoot_bias_pack_changes_the_seeded_match() {
    let (plain, plain_ticks, _) = play(90, None);
    let biased = pack("shoot-bias");
    let (scripted, scripted_ticks, events) = play(90, Some(&biased));
    let shots = |sim: &Simulation| sim.summary().shots.iter().sum::<u32>();
    assert!(
        plain_ticks.len() != scripted_ticks.len()
            || plain_ticks
                .iter()
                .zip(&scripted_ticks)
                .any(|(a, b)| a.ball != b.ball),
        "the pack left the seeded match unchanged ({} shots with it, {} without)",
        shots(&scripted),
        shots(&plain)
    );
    let stats = scripted.plugins().stats;
    assert!(stats.calls > 1_000, "{stats:?}");
    assert_eq!((stats.aborts, stats.denials, stats.disabled), (0, 0, 0));
    assert!(events.iter().all(|e| e.kind != EngineEventKind::Script));
    assert!(
        scripted
            .plugins()
            .pack
            .as_deref()
            .is_some_and(|p| p.starts_with("shoot-bias@1.0.0+")),
        "{:?}",
        scripted.plugins().pack
    );
}

#[test]
fn a_carrier_in_range_shoots_instead_of_passing_with_the_pack() {
    let scene = |plugins: Option<Plugins>| {
        let mut config = config(90);
        config.tuning.decision.noise = 0.0;
        // The home striker 20 m from goal with a defender in the shooting lane and a
        // team-mate open to its side; the other away outfield players far away.
        let mut scene = Scene::new(config);
        for i in 13..22 {
            scene = scene.place(i, DVec2::new(-40.0, (i as f64 - 17.0) * 6.0));
        }
        let mut scene = scene
            .place(12, DVec2::new(35.5, 0.0))
            .place(9, DVec2::new(32.5, 0.0))
            .place(10, DVec2::new(38.0, 14.0))
            .ball(DVec3::new(32.5, 0.0, 0.0))
            .carrier(Some(9))
            .tick(1_001);
        if let Some(plugins) = plugins {
            scene = scene.plugins(plugins);
        }
        let mut sim = scene.build();
        sim.step();
        sim.summary()
    };
    let native = scene(None);
    assert_eq!(
        (native.passes[0], native.shots[0]),
        (1, 0),
        "the native choice is the pass"
    );
    let scripted = scene(Some(pack("shoot-bias").plugins()));
    assert_eq!(
        scripted.shots[0], 1,
        "the pack's offset turns it into a shot"
    );
}

#[test]
fn the_same_seed_and_pack_twice_give_the_same_ticks() {
    let biased = pack("shoot-bias");
    let (_, a, events_a) = play(30, Some(&biased));
    let (_, b, events_b) = play(30, Some(&biased));
    assert_eq!(a, b);
    assert_eq!(events_a, events_b);
}

#[test]
fn a_match_without_hooks_is_the_match_without_a_plugin() {
    let (_, plain, plain_events) = play(30, None);
    let mut sim = Simulation::new(config(30)).unwrap();
    sim.set_plugins(Plugins::new("empty@1.0.0+000000000000"));
    let mut sink = VecSink::default();
    sim.run(&mut sink).unwrap();
    assert_eq!(sink.records, plain);
    assert_eq!(sim.take_events(), plain_events);
}

fn content_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content")
        .join(name)
}

#[test]
fn the_rhai_adapter_card_is_complete() {
    let text = std::fs::read_to_string(content_path("realism-bands.json")).unwrap();
    let bands: serde_json::Value = serde_json::from_str(&text).unwrap();
    // Every band of the registry by name, and every group of bands too.
    let names: Vec<&str> = bands["bands"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|b| [b["band"].as_str(), b["group"].as_str()])
        .flatten()
        .collect();
    assert!(!names.is_empty());
    check_card(&RHAI_ADAPTER_CARD, &names).unwrap();
    assert!(RHAI_ADAPTER_CARD.keys.is_empty(), "no hook call draws");
}

/// The sample pack has all three hooks. With the three hook slots switched off through the
/// slot file, none of them is called, no line is rewritten, and the match plays as the same
/// match without a plugin: the same final score and the same events.
#[test]
fn a_pack_under_switched_off_hook_slots_is_never_called() {
    let shipped = std::fs::read_to_string(content_path("slots.json")).unwrap();
    let mut slots: serde_json::Value = serde_json::from_str(&shipped).unwrap();
    for slot in [
        "engine.hook.decision",
        "engine.hook.rule",
        "engine.hook.commentary",
    ] {
        slots["slots"][slot] = serde_json::json!({ "module": "off" });
    }
    let bytes = serde_json::to_vec(&slots).unwrap();
    let pack = LoadedPack::load(&content_path("scripts/sample")).unwrap();
    assert!(pack.plugins().decision.is_some());
    assert!(pack.plugins().rule.is_some());
    assert!(pack.plugins().commentary.is_some());

    let mut plain = config(30);
    let mut off = config(30);
    let content = engine::Content::load(&engine::ContentDir::at(content_path(""))).unwrap();
    let hooks_off = content.with_slots(&bytes).unwrap();
    assert_ne!(
        hooks_off.digest, content.digest,
        "the selection folds into the hash"
    );
    off.modules = hooks_off.modules;
    plain.modules = content.modules;

    let mut native = Simulation::new(plain).unwrap();
    native.run(&mut VecSink::default()).unwrap();
    let native_events = native.take_events();

    off.fold_pack_hash(pack.sha());
    let mut sim = Simulation::new(off).unwrap();
    sim.set_plugins(pack.plugins());
    sim.run(&mut VecSink::default()).unwrap();
    let events = sim.take_events();
    for event in &events {
        let (line, script) = sim.offer_line(event, Some("native".to_string()));
        assert_eq!(line.as_deref(), Some("native"));
        assert!(script.is_empty());
    }
    assert_eq!(sim.plugins().stats.calls, 0, "{:?}", sim.plugins().stats);
    assert_eq!(sim.summary().goals, native.summary().goals);
    let kinds = |events: &[engine::EngineEvent]| events.iter().map(|e| e.kind).collect::<Vec<_>>();
    assert_eq!(kinds(&events), kinds(&native_events));
    assert_eq!(events, native_events);
}
