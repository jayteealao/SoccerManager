//! AC-b: one draw decides a tackle. Inside the foul band with the ball lost, the referee
//! gives a free kick; outside the band there is no foul; with the ball kept, the referee
//! plays advantage; inside the penalty area, the foul gives a penalty at the mark.

mod common;

use common::{quiet_match, spread};
use engine::math::{DVec2, DVec3};
use engine::rules::fouls::foul_chance;
use engine::scenario::Scene;
use engine::{EngineEvent, EngineEventKind, MatchConfig, Simulation, StoppageKind};

/// The home carrier, and the away player who tackles.
const CARRIER: usize = 5;
const TACKLER: usize = 16;

/// No card: above every card band.
const NO_CARD: f64 = 0.99;

/// The win chance and the foul chance of the tackle in these scenes.
fn bands(config: &MatchConfig) -> (f64, f64) {
    let sim = Simulation::new(config.clone()).unwrap();
    let tackler = sim.players()[TACKLER].derived;
    let carrier = sim.players()[CARRIER].derived;
    let p_win = 0.05 * tackler.tackling / (tackler.tackling + carrier.dribbling);
    (p_win, foul_chance(&tackler, 0, &config.tuning))
}

/// The home carrier holds the ball at `at` and the tackler stands 0.3 m from it. The first
/// step judges the tackle with `rolls`.
fn tackle(at: DVec2, rolls: impl Fn(f64, f64) -> Vec<f64>) -> (Simulation, Vec<EngineEvent>) {
    tackle_in(quiet_match(90), at, rolls)
}

/// `tackle` in a match played under `config`.
fn tackle_in(
    config: MatchConfig,
    at: DVec2,
    rolls: impl Fn(f64, f64) -> Vec<f64>,
) -> (Simulation, Vec<EngineEvent>) {
    let (p_win, p_foul) = bands(&config);
    let mut sim = spread(Scene::new(config), -30.0, 30.0)
        .place(CARRIER, at)
        .place(TACKLER, at + DVec2::new(0.3, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001)
        .rolls(&rolls(p_win, p_foul))
        .build();
    sim.step();
    let events = sim.take_events();
    (sim, events)
}

fn kinds(events: &[EngineEvent]) -> Vec<EngineEventKind> {
    events.iter().map(|e| e.kind).collect()
}

#[test]
fn a_foul_with_the_ball_lost_gives_a_free_kick_where_it_happened() {
    let at = DVec2::new(0.0, 10.0);
    let (sim, events) = tackle(at, |w, f| vec![w + 0.1 * f, NO_CARD]);
    assert_eq!(
        kinds(&events),
        [EngineEventKind::Foul, EngineEventKind::FreeKick]
    );
    let foul = events[0];
    assert_eq!(foul.team, Some(1));
    assert_eq!(foul.player, Some(TACKLER));
    assert_eq!(foul.secondary, Some(CARRIER));
    assert_eq!(foul.advantage, Some(false));
    let free_kick = events[1];
    assert_eq!(free_kick.team, Some(0));
    let spot = free_kick.spot.unwrap();
    assert!((spot - at).length() <= 0.5, "{spot}");
    let dead = sim.dead_ball().expect("play has stopped");
    assert_eq!(dead.kind, StoppageKind::FreeKick);
    assert!(dead.direct);
    assert_eq!(sim.summary().fouls, [0, 1]);
    assert_eq!(sim.summary().free_kicks, [1, 0]);
}

#[test]
fn a_draw_outside_the_foul_band_is_no_foul() {
    let (sim, events) = tackle(DVec2::new(0.0, 10.0), |w, f| vec![w + f + 0.01]);
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(sim.carrier(), Some(CARRIER));
    assert!(sim.dead_ball().is_none());
    assert_eq!(sim.summary().fouls, [0, 0]);
}

#[test]
fn a_foul_with_the_ball_kept_plays_advantage() {
    let (sim, events) = tackle(DVec2::new(0.0, 10.0), |w, f| vec![w + 0.9 * f, NO_CARD]);
    assert_eq!(kinds(&events), [EngineEventKind::Foul]);
    assert_eq!(events[0].advantage, Some(true));
    assert!(sim.dead_ball().is_none(), "play continues");
    assert_eq!(sim.carrier(), Some(CARRIER));
    assert_eq!(sim.summary().fouls, [0, 1]);
}

#[test]
fn a_card_held_for_advantage_is_shown_at_the_next_stoppage() {
    // A yellow-card draw on a foul played on with advantage, then a second foul with the
    // ball lost and no card of its own. The foul cooldown is off, so the same player can
    // foul on the next tick.
    let mut config = quiet_match(90);
    config.tuning.foul_cooldown_ticks = 0;
    let (mut sim, events) = tackle_in(config, DVec2::new(0.0, 10.0), |w, f| {
        vec![w + 0.9 * f, 0.01, w + 0.1 * f, NO_CARD]
    });
    assert_eq!(kinds(&events), [EngineEventKind::Foul]);
    assert_eq!(sim.summary().yellow, [0, 0], "the card waits");
    sim.step();
    let events = sim.take_events();
    assert_eq!(
        kinds(&events),
        [
            EngineEventKind::Foul,
            EngineEventKind::Card,
            EngineEventKind::FreeKick
        ]
    );
    assert_eq!(events[1].player, Some(TACKLER));
    assert_eq!(events[1].card, Some(engine::Card::Yellow));
    assert_eq!(sim.summary().yellow, [0, 1]);
}

#[test]
fn a_foul_in_the_penalty_area_gives_a_penalty_at_the_mark() {
    // The away side defends positive `x` in the first half.
    let at = DVec2::new(45.0, 0.0);
    for draw in [0.1, 0.9] {
        let (sim, events) = tackle(at, |w, f| vec![w + draw * f, NO_CARD]);
        assert_eq!(
            kinds(&events),
            [EngineEventKind::Foul, EngineEventKind::Penalty],
            "draw {draw}"
        );
        assert_eq!(events[0].advantage, Some(false), "no advantage in the area");
        let spot = events[1].spot.unwrap();
        let mark = engine::pitch::penalty_spot(1.0);
        assert!((spot - mark).length() <= 0.5, "{spot} against {mark}");
        assert_eq!(sim.dead_ball().unwrap().kind, StoppageKind::Penalty);
        assert_eq!(sim.summary().penalties, [1, 0]);
    }
}
