//! AC-a: a player beyond the second-last defender when a team-mate plays the ball is
//! penalised at the first touch, with an indirect free kick where the player became
//! involved. Level is onside, the own half is onside, and a throw-in puts nobody offside.

mod common;

use common::{quiet_match, spread};
use engine::math::{DVec2, DVec3};
use engine::scenario::Scene;
use engine::{EngineEventKind, Simulation};

const PASSER: usize = 9;
const ATTACKER: usize = 10;

/// The home side attacks positive `x`. The passer plays a slow ball to the attacker 0.8 m
/// ahead of it, with the away outfield players on `x = defenders_x`.
fn pass_to(attacker_x: f64, defenders_x: f64) -> Simulation {
    let scene = spread(Scene::new(quiet_match(90)), -30.0, defenders_x)
        .place(PASSER, DVec2::new(attacker_x - 4.0, 0.0))
        .place(ATTACKER, DVec2::new(attacker_x, 0.0))
        .carrier(Some(PASSER))
        .ball(DVec3::new(attacker_x - 0.8, 0.0, 0.0))
        .tick(1_001)
        .kick(DVec2::X, 1.0, 0.0);
    scene.build()
}

#[test]
fn a_player_beyond_the_second_last_defender_is_penalised_at_the_first_touch() {
    let mut sim = pass_to(29.8, 20.0);
    sim.step();
    let events = sim.take_events();
    let offside = events
        .iter()
        .find(|e| e.kind == EngineEventKind::Offside)
        .expect("an offside event");
    assert_eq!(offside.team, Some(0));
    assert_eq!(offside.player, Some(ATTACKER));
    let free_kick = events
        .iter()
        .find(|e| e.kind == EngineEventKind::FreeKick)
        .expect("a free kick follows");
    assert_eq!(free_kick.team, Some(1));
    let spot = free_kick.spot.expect("the free kick names its spot");
    let touch = sim.players()[ATTACKER].pos;
    assert!((spot - touch).length() <= 0.5, "{spot} against {touch}");
    let dead = sim.dead_ball().expect("play has stopped");
    assert!(!dead.direct, "an offside free kick is indirect");
    assert_eq!(sim.summary().offsides, [1, 0]);
}

#[test]
fn a_player_level_with_the_second_last_defender_is_onside() {
    let mut sim = pass_to(29.8, 29.8);
    sim.step();
    assert!(
        sim.take_events()
            .iter()
            .all(|e| e.kind != EngineEventKind::Offside)
    );
    assert_eq!(sim.carrier(), Some(ATTACKER));
}

#[test]
fn a_player_in_the_own_half_is_onside() {
    let mut sim = pass_to(-9.2, -20.0);
    sim.step();
    assert!(
        sim.take_events()
            .iter()
            .all(|e| e.kind != EngineEventKind::Offside)
    );
    assert_eq!(sim.carrier(), Some(ATTACKER));
}

#[test]
fn a_throw_in_puts_nobody_offside() {
    // The away side touched the ball last; it crosses the touchline, and the home side
    // throws in with its attacker far beyond every defender.
    let mut sim = spread(Scene::new(quiet_match(90)), -20.0, 20.0)
        .place(ATTACKER, DVec2::new(40.0, 20.0))
        .carrier(None)
        .ball(DVec3::new(10.0, 33.8, 0.0))
        .ball_velocity(DVec3::new(0.0, 4.0, 0.0))
        .last_touch(1)
        .tick(1_001)
        .build();
    let mut events = Vec::new();
    for _ in 0..10 {
        sim.step();
        events.extend(sim.take_events());
    }
    assert!(
        events.iter().any(|e| e.kind == EngineEventKind::ThrowIn),
        "{events:?}"
    );
    let checks = sim.summary().offside_checks;
    assert!(
        sim.dead_ball().is_some(),
        "the throw-in waits for its taker"
    );
    let mut thrown = false;
    for _ in 0..3_000 {
        sim.step();
        thrown |= sim.dead_ball().is_none();
        events.extend(sim.take_events());
    }
    assert!(thrown, "the throw-in was taken");
    assert_eq!(sim.summary().offside_checks, checks);
    assert!(events.iter().all(|e| e.kind != EngineEventKind::Offside));
}
