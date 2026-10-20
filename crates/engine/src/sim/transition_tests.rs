//! Tests of the transition windows: the ticks at which a side wins and loses the ball in open
//! play, and the dead balls that end them.

use crate::data::rules::StoppageKind;
use crate::data::test_support::shipped_config;
use crate::math::DVec2;
use crate::sim::Simulation;
use crate::team::PLAYERS_PER_TEAM;

/// A seed-7 match on the shipped content a minute in, the home side on the ball from the
/// kick-off.
fn scene() -> Simulation {
    let mut sim = Simulation::new(shipped_config(7, 90).unwrap()).unwrap();
    sim.tick = 3_000;
    sim
}

/// An interception opens the winner's regain window and the loser's counter-press window;
/// each reads the seconds since the gain while its side has (or lacks) the ball, and a
/// throw-in closes both.
#[test]
fn an_interception_opens_both_windows_and_a_throw_in_closes_them() {
    let mut sim = scene();
    let t = std::sync::Arc::clone(&sim.tuning);
    assert_eq!(sim.possession_phase, Some(0));
    sim.gain(PLAYERS_PER_TEAM + 5, &t);
    assert_eq!(sim.windows.regained_at, [None, Some(3_000)]);
    assert_eq!(sim.windows.lost_at, [Some(3_000), None]);
    sim.tick += 50;
    let dt = sim.config.tuning.dt;
    assert_eq!(sim.since_regain_s(1), Some(50.0 * dt));
    assert_eq!(sim.since_loss_s(0), Some(50.0 * dt));
    assert_eq!(sim.since_regain_s(0), None);
    assert_eq!(sim.since_loss_s(1), None);
    let touchline = DVec2::new(0.0, sim.config.pitch().half_width());
    sim.open_dead_ball(
        StoppageKind::ThrowIn,
        StoppageKind::ThrowIn,
        0,
        touchline,
        false,
    );
    assert_eq!(sim.windows.regained_at, [None; 2]);
    assert_eq!(sim.windows.lost_at, [None; 2]);
    assert_eq!(sim.since_regain_s(1), None);
    assert_eq!(sim.since_loss_s(0), None);
}

/// A gain by the side already in possession (after the kick-off, a team-mate's touch) opens
/// no window; and once the side that lost the ball wins it back, its counter-press window
/// closes and its regain window opens.
#[test]
fn no_window_without_a_change_of_side_and_a_win_back_swaps_the_windows() {
    let mut sim = scene();
    let t = std::sync::Arc::clone(&sim.tuning);
    sim.gain(5, &t);
    assert_eq!(sim.windows.regained_at, [None; 2]);
    assert_eq!(sim.windows.lost_at, [None; 2]);
    sim.gain(PLAYERS_PER_TEAM + 5, &t);
    sim.tick += 40;
    sim.gain(6, &t);
    assert_eq!(sim.since_loss_s(0), None);
    assert_eq!(sim.since_regain_s(0), Some(0.0));
    assert!(sim.since_loss_s(1).is_some());
    assert_eq!(sim.since_regain_s(1), None);
}
