//! The wide clearance. Inside his own penalty area, a clearance by a carrier or of a fast
//! pass goes wide toward the clearer's own goal line with `clearances.wide_chance`. The ball
//! then goes behind or not as it flies, and a corner still comes only from a ball that
//! crossed the goal line off a defender.

mod common;

use common::{calm_match, index, spread};
use engine::math::{DVec2, DVec3};
use engine::scenario::Scene;
use engine::{Simulation, StoppageKind};

/// An away centre-back, and the away keeper.
const DEFENDER: usize = 13;
const AWAY_KEEPER: usize = 11;

/// A calm match on `seed` with every pass in the away penalty area cleared, a `wide` chance
/// that the clearance goes wide, and a cleared ball that keeps `speed` of its speed. The
/// home striker passes from `(30, 0)` along the x axis at 20 m/s; the away centre-back
/// clears it at `(40, 0.5)`. The home side attacks `x = 52.5` in the first half.
fn cleared_pass(seed: u64, wide: f64, speed: f64) -> Simulation {
    let mut config = calm_match(90);
    config.seed = seed;
    config.tuning.clearances.cross_chance = 1.0;
    config.tuning.clearances.wide_chance = wide;
    config.tuning.clearances.cross_speed = speed;
    spread(Scene::new(config), -30.0, -30.0)
        .place(index(0, 9), DVec2::new(30.0, 0.0))
        .place(DEFENDER, DVec2::new(40.0, 0.5))
        // The keeper stands off every line the cleared ball can take.
        .place(AWAY_KEEPER, DVec2::new(30.0, -25.0))
        .ball(DVec3::new(30.0, 0.0, 0.0))
        .carrier(Some(index(0, 9)))
        .tick(1_001)
        .kick(DVec2::X, 20.0, 0.0)
        .build()
}

/// What happened after the clearance: the dead ball it gave and the side that touched the
/// ball last before play stopped, or `None` when a player took the ball or 300 ticks passed.
fn after_clearance(sim: &mut Simulation) -> Option<(StoppageKind, Option<usize>)> {
    for _ in 0..300 {
        let touch = sim.last_touch();
        sim.step();
        if let Some(dead) = sim.dead_ball() {
            return Some((dead.kind, touch));
        }
        if sim.carrier().is_some() {
            return None;
        }
    }
    None
}

/// A ball that stops on the line stays in play, so not every seed gives a corner.
#[test]
fn a_wide_clearance_of_a_pass_goes_behind_for_a_corner() {
    let mut corners = 0;
    for seed in 1..=20 {
        let mut sim = cleared_pass(seed, 1.0, 0.7);
        let dead = after_clearance(&mut sim);
        assert_eq!(
            sim.summary().clearances,
            [0, 1],
            "seed {seed}: no clearance"
        );
        if let Some(dead) = dead {
            assert_eq!(
                dead,
                (StoppageKind::Corner, Some(1)),
                "seed {seed}: a corner off the defender"
            );
            corners += 1;
        }
    }
    assert!(corners >= 18, "{corners} of 20 clearances gave a corner");
}

#[test]
fn with_no_wide_chance_the_same_clearance_gives_no_corner() {
    for seed in 1..=20 {
        let mut sim = cleared_pass(seed, 0.0, 0.7);
        let dead = after_clearance(&mut sim);
        assert_eq!(
            sim.summary().clearances,
            [0, 1],
            "seed {seed}: no clearance"
        );
        assert_ne!(
            dead.map(|(kind, _)| kind),
            Some(StoppageKind::Corner),
            "seed {seed}"
        );
    }
}

#[test]
fn a_wide_clearance_that_stops_short_of_the_line_gives_no_corner() {
    for seed in 1..=20 {
        let mut sim = cleared_pass(seed, 1.0, 0.1);
        let dead = after_clearance(&mut sim);
        assert_eq!(
            sim.summary().clearances,
            [0, 1],
            "seed {seed}: no clearance"
        );
        assert_eq!(dead, None, "seed {seed}: the ball stayed in play");
        assert!(
            sim.record().ball[0] < 52.5,
            "seed {seed}: short of the line"
        );
    }
}

/// A calm match where every carrier clears, with a `wide` chance that the clearance goes
/// wide. The away centre-back has the ball at `at`; the away side defends `x = 52.5`.
fn clearing_carrier(wide: f64, at: DVec2) -> Simulation {
    let mut config = calm_match(90);
    config.tuning.decision_interval_ticks = 1;
    config.tuning.clearances.wide_chance = wide;
    let d = &mut config.tuning.decision;
    (d.clear, d.lane, d.shot_base, d.dribble_base, d.hold) = (5.0, -5.0, -5.0, -5.0, -5.0);
    spread(Scene::new(config), -30.0, -30.0)
        .place(DEFENDER, at)
        .place(AWAY_KEEPER, DVec2::new(30.0, -25.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(DEFENDER))
        .last_touch(1)
        .tick(1_001)
        .build()
}

#[test]
fn a_wide_clearance_by_a_carrier_in_his_area_goes_behind_for_a_corner() {
    let mut sim = clearing_carrier(1.0, DVec2::new(45.0, 5.0));
    let dead = after_clearance(&mut sim);
    assert_eq!(sim.summary().clearances, [0, 1], "the carrier cleared");
    assert_eq!(dead, Some((StoppageKind::Corner, Some(1))));
}

#[test]
fn a_carrier_outside_his_area_never_clears_wide() {
    let mut sim = clearing_carrier(1.0, DVec2::new(20.0, 5.0));
    let dead = after_clearance(&mut sim);
    assert_eq!(sim.summary().clearances, [0, 1], "the carrier cleared");
    assert_ne!(dead.map(|(kind, _)| kind), Some(StoppageKind::Corner));
    assert!(
        sim.record().ball[0] < 20.0,
        "the clearance went up the pitch"
    );
}
