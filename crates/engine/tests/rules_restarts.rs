//! AC-d: a ball over a touchline gives a throw-in, over the goal line off an attacker a goal
//! kick, and off a defender a corner, each with the ball placed at the law's spot. At a
//! free kick, every opponent is at least 9.15 m from the ball when it is taken.

mod common;

use common::{quiet_match, spread};
use engine::math::{DVec2, DVec3};
use engine::pitch::{self, KICK_DISTANCE};
use engine::rules::fouls::foul_chance;
use engine::scenario::Scene;
use engine::{EngineEvent, EngineEventKind, Simulation, StoppageKind};

/// The ball rolls from `at` with velocity `v` after `last` touched it, with every player far
/// away. Returns the first restart event and the match on the tick of that event.
fn roll_out(at: DVec2, v: DVec2, last: usize) -> (Simulation, EngineEvent) {
    let mut sim = spread(Scene::new(quiet_match(90)), -30.0, 30.0)
        .carrier(None)
        .ball(DVec3::new(at.x, at.y, 0.0))
        .ball_velocity(DVec3::new(v.x, v.y, 0.0))
        .last_touch(last)
        .tick(1_001)
        .build();
    for _ in 0..50 {
        sim.step();
        if let Some(event) = sim.take_events().into_iter().find(|e| e.spot.is_some()) {
            return (sim, event);
        }
    }
    panic!("the ball never left the pitch");
}

/// The ball on the record of the current tick.
fn ball(sim: &Simulation) -> DVec2 {
    let record = sim.record();
    DVec2::new(f64::from(record.ball[0]), f64::from(record.ball[1]))
}

fn assert_placed(sim: &Simulation, event: &EngineEvent, spot: DVec2) {
    let placed = event.spot.expect("a restart names its spot");
    assert!((placed - spot).length() <= 0.5, "{placed} against {spot}");
    assert!(
        (ball(sim) - placed).length() <= 0.5,
        "the ball waits at the spot"
    );
    assert!(sim.record().restart, "the tick is marked as a restart");
}

#[test]
fn a_ball_over_the_touchline_gives_a_throw_in_where_it_crossed() {
    let (sim, event) = roll_out(DVec2::new(10.0, 33.8), DVec2::new(0.0, 4.0), 1);
    assert_eq!(event.kind, EngineEventKind::ThrowIn);
    assert_eq!(event.team, Some(0), "against the team that touched it last");
    let crossed = DVec2::new(10.0, pitch::HALF_WIDTH);
    assert_placed(&sim, &event, pitch::throw_in_spot(crossed, 1.0));
    assert!((event.spot.unwrap() - crossed).length() <= 0.5);
}

#[test]
fn a_ball_over_the_goal_line_off_an_attacker_gives_a_goal_kick() {
    // The home side attacks positive `x` and touched the ball last.
    let (sim, event) = roll_out(DVec2::new(52.0, 20.0), DVec2::new(4.0, 0.0), 0);
    assert_eq!(event.kind, EngineEventKind::GoalKick);
    assert_eq!(event.team, Some(1));
    assert_placed(&sim, &event, pitch::goal_kick_spot(1.0, 20.0));
    assert_eq!(sim.dead_ball().unwrap().kind, StoppageKind::GoalKick);
}

#[test]
fn a_ball_over_the_goal_line_off_a_defender_gives_a_corner() {
    let (sim, event) = roll_out(DVec2::new(52.0, 20.0), DVec2::new(4.0, 0.0), 1);
    assert_eq!(event.kind, EngineEventKind::Corner);
    assert_eq!(event.team, Some(0));
    let spot = pitch::corner_spot(1.0, 20.0);
    assert_placed(&sim, &event, spot);
    assert!(
        spot.x > 51.0 && spot.y > 32.0,
        "the corner on the side it crossed: {spot}"
    );
}

#[test]
fn opponents_are_ten_yards_away_when_a_free_kick_is_taken() {
    const CARRIER: usize = 5;
    const TACKLER: usize = 16;
    let config = quiet_match(90);
    let plain = Simulation::new(config.clone()).unwrap();
    let tackler = plain.players()[TACKLER].derived;
    let carrier = plain.players()[CARRIER].derived;
    let p_win = engine::rules::fouls::win_chance(&tackler, &carrier, &config.tuning);
    let p_foul = foul_chance(&tackler, 0, &config.tuning);
    let at = DVec2::new(0.0, 10.0);
    // Crowd the spot with the away side, so every opponent must walk away from it.
    let mut scene = spread(Scene::new(config), -30.0, 30.0);
    for (k, i) in (12..22).enumerate() {
        let angle = k as f64 * 0.6;
        let (sin, cos) = engine::math::sin_cos(angle);
        scene = scene.place(i, at + DVec2::new(cos, sin) * 3.0);
    }
    let mut sim = scene
        .place(CARRIER, at)
        .place(TACKLER, at + DVec2::new(0.3, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001)
        .rolls(&[p_win + 0.1 * p_foul, 0.99])
        .build();
    sim.step();
    let dead = sim.dead_ball().expect("a free kick");
    assert_eq!(dead.kind, StoppageKind::FreeKick);
    let mut taken = None;
    for _ in 0..dead.hard_limit() - dead.since + 2 {
        let before = sim.players().to_vec();
        sim.step();
        if sim.dead_ball().is_none() {
            taken = Some(before);
            break;
        }
    }
    let players = taken.expect("the free kick was taken");
    for p in players.iter().filter(|p| p.team == 1 && p.active()) {
        let d = (p.pos - dead.spot).length();
        assert!(d >= KICK_DISTANCE, "away slot {} at {d:.2} m", p.slot);
    }
}

/// A free kick given close to a touchline, near the halfway line, used to stall to the hard
/// time limit: an opponent's formation anchor there fell just off the pitch, and clamping it
/// back onto the pitch pulled it inside the ten-yard circle around the spot again, so it could
/// never satisfy the distance the law requires and just stood there instead of clearing.
#[test]
fn opponents_are_ten_yards_away_when_a_free_kick_is_taken_near_a_touchline() {
    const CARRIER: usize = 5;
    const TACKLER: usize = 16;
    let config = quiet_match(90);
    let plain = Simulation::new(config.clone()).unwrap();
    let tackler = plain.players()[TACKLER].derived;
    let carrier = plain.players()[CARRIER].derived;
    let p_win = engine::rules::fouls::win_chance(&tackler, &carrier, &config.tuning);
    let p_foul = foul_chance(&tackler, 0, &config.tuning);
    // Close to the touchline (34 m) but not on it, and not near a corner: the ten-yard circle
    // around this spot still spills off the pitch, which is what traps an opponent's anchor.
    let at = DVec2::new(4.0, 29.2);
    let mut sim = spread(Scene::new(config), -30.0, 30.0)
        .place(CARRIER, at)
        .place(TACKLER, at + DVec2::new(0.3, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001)
        .rolls(&[p_win + 0.1 * p_foul, 0.99])
        .build();
    sim.step();
    let dead = sim.dead_ball().expect("a free kick");
    assert_eq!(dead.kind, StoppageKind::FreeKick);
    assert_eq!(dead.team, 0, "the fouled side takes the kick");
    let mut taken = None;
    for _ in 0..dead.hard_limit() - dead.since + 2 {
        let before = sim.players().to_vec();
        let now = sim.tick() + 1;
        sim.step();
        if sim.dead_ball().is_none() {
            taken = Some((before, now));
            break;
        }
    }
    let (players, taken_at) = taken.expect("the free kick was taken");
    assert!(
        taken_at < dead.hard_limit(),
        "the kick waited for the hard limit at {taken_at} instead of being taken on time"
    );
    for p in players.iter().filter(|p| p.team == 1 && p.active()) {
        let d = (p.pos - dead.spot).length();
        assert!(d >= KICK_DISTANCE, "away slot {} at {d:.2} m", p.slot);
    }
}
