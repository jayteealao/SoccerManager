//! A script pack that overrides the decision scoring hook changes the seeded match, the match
//! stays reproducible with the pack, and a match without a pack is untouched.

mod common;

use common::{config, pack, play};
use engine::math::{DVec2, DVec3};
use engine::scenario::Scene;
use engine::{EngineEventKind, Plugins, Simulation, VecSink};

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
