//! Tests of the slide pass ([`crate::sim::slide`]) through the real decision pass: the ball
//! goes wide and up, the block slides toward it, and the far players follow late.

use crate::ball::Ball;
use crate::data::test_support::{shipped_config, shipped_config_with_stands};
use crate::math::DVec2;
use crate::modules::ROSTER;
use crate::shape::anchors;
use crate::sim::Simulation;
use crate::team::PLAYERS_PER_TEAM;

/// The away central midfielder who carries the ball.
const CARRIER: usize = PLAYERS_PER_TEAM + 6;

/// A seeded match on the shipped content with the far-player delay set to `k`.
fn scene(k: f64) -> Simulation {
    let mut config = shipped_config(7, 90).unwrap();
    config.tuning.far_delay_s_per_m = k;
    Simulation::new(config).unwrap()
}

/// [`scene`] with the team stands on, so a player's places in and out of possession differ.
fn scene_with_stands(k: f64) -> Simulation {
    let mut config = shipped_config_with_stands(7, 90).unwrap();
    config.tuning.far_delay_s_per_m = k;
    Simulation::new(config).unwrap()
}

/// The carrier holds the ball at `at`; his team is in possession.
fn hold(sim: &mut Simulation, at: DVec2) {
    let p = &mut sim.players[CARRIER];
    p.pos = at;
    p.vel = DVec2::ZERO;
    p.target = at;
    sim.ball = Ball::at(at);
    sim.carrier = Some(CARRIER);
    sim.control_since = sim.tick;
    sim.last_touch = Some(1);
    sim.possession_phase = Some(1);
}

/// One open-play tick of the slide pass, the decision's targets and steering, with the
/// carrier holding the ball where he stands.
fn tick(sim: &mut Simulation) {
    sim.slide_tick();
    let decided = sim.config.modules.decision.targets(&sim.view());
    for (p, &t) in sim.players.iter_mut().zip(decided.targets.iter()) {
        p.target = t;
    }
    let at = sim.players[CARRIER].pos;
    sim.players[CARRIER].target = at;
    let t = std::sync::Arc::clone(&sim.tuning);
    sim.steer_players(&t);
    sim.players[CARRIER].pos = at;
    sim.players[CARRIER].vel = DVec2::ZERO;
    sim.tick += 1;
}

fn ticks(sim: &Simulation, seconds: f64) -> u32 {
    (seconds / sim.tuning.dt).round() as u32
}

/// The active outfield players of both teams.
fn outfield(sim: &Simulation) -> Vec<usize> {
    (0..ROSTER)
        .filter(|&i| sim.players[i].active() && i != sim.keeper(i / PLAYERS_PER_TEAM))
        .collect()
}

/// The mean of `points` over `who`.
fn mean(points: &[DVec2; ROSTER], who: &[usize]) -> DVec2 {
    who.iter().map(|&i| points[i]).sum::<DVec2>() / who.len() as f64
}

/// What one second after the ball went wide shows: each team's mean outfield anchor before
/// and after, the followed points at the jump and after, the players within 20 m of the
/// ball and those beyond 30 m at the jump, and the match.
struct Wide {
    sim: Simulation,
    old: DVec2,
    ball: DVec2,
    before: [DVec2; ROSTER],
    after: [DVec2; ROSTER],
    near: Vec<usize>,
    far: Vec<usize>,
    anchors_before: [DVec2; 2],
    anchors_after: [DVec2; 2],
}

/// A settled block around a ball held on the centre spot; then the ball and its carrier go
/// to the right touchline 20 m farther up while every followed point is still at the old
/// ball, and one second of play follows.
fn ball_goes_wide(k: f64) -> Wide {
    let mut sim = scene(k);
    let old = DVec2::ZERO;
    hold(&mut sim, old);
    for _ in 0..ticks(&sim, 6.0) {
        tick(&mut sim);
    }
    let ball = DVec2::new(20.0, sim.config.pitch().half_width() - 1.0);
    hold(&mut sim, ball);
    sim.slide_ball = [old; ROSTER];
    let who = outfield(&sim);
    let team_mean = |sim: &Simulation| {
        let a = anchors(&sim.view(), sim.possession_phase);
        [0, 1].map(|team| {
            let mine: Vec<usize> = who
                .iter()
                .copied()
                .filter(|&i| i / PLAYERS_PER_TEAM == team)
                .collect();
            mean(&a, &mine)
        })
    };
    let anchors_before = team_mean(&sim);
    let distance = |i: usize| (sim.players[i].pos - ball).length();
    let near = who
        .iter()
        .copied()
        .filter(|&i| distance(i) < 20.0)
        .collect();
    let far = who
        .iter()
        .copied()
        .filter(|&i| distance(i) > 30.0)
        .collect();
    let before = sim.slide_ball;
    for _ in 0..ticks(&sim, 1.0) {
        tick(&mut sim);
    }
    let anchors_after = team_mean(&sim);
    let after = sim.slide_ball;
    Wide {
        sim,
        old,
        ball,
        before,
        after,
        near,
        far,
        anchors_before,
        anchors_after,
    }
}

/// The mean distance the followed points of `who` moved in the second after the jump.
fn mean_shift(w: &Wide, who: &[usize]) -> f64 {
    who.iter()
        .map(|&i| (w.after[i] - w.before[i]).length())
        .sum::<f64>()
        / who.len() as f64
}

#[test]
fn the_block_slides_toward_a_wide_ball_and_far_players_follow_late() {
    let k = shipped_config(7, 90).unwrap().tuning.far_delay_s_per_m;
    assert!(k > 0.0, "the shipped far-player delay is on");
    let mut w = ball_goes_wide(k);
    assert!(
        !w.near.is_empty() && !w.far.is_empty(),
        "{:?} {:?}",
        w.near,
        w.far
    );
    // (i) Each team's block moves toward the ball: up the pitch and across.
    for team in 0..2 {
        let moved = w.anchors_after[team] - w.anchors_before[team];
        let toward = w.ball - w.old;
        assert!(
            moved.x * toward.x > 0.0 && moved.y * toward.y > 0.0,
            "team {team} block moved {moved}"
        );
    }
    // (ii) Players near the ball follow it sooner than players far from it.
    let (near, far) = (mean_shift(&w, &w.near), mean_shift(&w, &w.far));
    assert!(near > far, "near {near:.2} m, far {far:.2} m");
    // (iii) Far players follow late but do not stay behind: after 10 s every followed point
    // is at the ball.
    for _ in 0..ticks(&w.sim, 9.0) {
        tick(&mut w.sim);
    }
    for i in outfield(&w.sim) {
        let off = (w.sim.slide_ball[i] - w.ball).length();
        assert!(off < 0.5, "player {i} follows {off:.2} m behind the ball");
    }
}

/// The control: with the delay off every player's followed point is the ball at once, so
/// near and far players shift alike and the near-against-far check above would fail.
#[test]
fn with_the_delay_off_near_and_far_players_shift_alike() {
    let w = ball_goes_wide(0.0);
    assert!(!w.near.is_empty() && !w.far.is_empty());
    let (near, far) = (mean_shift(&w, &w.near), mean_shift(&w, &w.far));
    assert!((near - far).abs() < 1e-9, "near {near} far {far}");
    assert!((near - (w.ball - w.old).length()).abs() < 1e-9);
}

/// A player left far from play and from both his places heads for the nearer place, even
/// when the phase sends him to the other; within reach of a place the shape pass decides.
#[test]
fn a_player_stranded_far_from_play_heads_for_his_nearer_place() {
    const LEFT_BACK: usize = 1;
    let target_from = |offset: DVec2| {
        let mut sim = scene_with_stands(0.04);
        hold(&mut sim, DVec2::new(35.0, 25.0));
        let ball = sim.ball.xy();
        let t = sim.tuning.clone();
        let team = sim.teams[0].clone();
        let a_in = team.anchor(LEFT_BACK, ball, true, &t);
        let a_out = team.anchor(LEFT_BACK, ball, false, &t);
        let pos = a_in + DVec2::new(offset.x * team.attack_x, offset.y);
        sim.players[LEFT_BACK].pos = pos;
        sim.players[LEFT_BACK].lapse_until = 0;
        assert!((pos - ball).length() > t.anchor_ball_distance);
        let decided = sim.config.modules.decision.targets(&sim.view());
        (
            decided.targets[LEFT_BACK],
            pos,
            a_in,
            a_out,
            t.anchor_tolerance,
        )
    };
    // Ahead of his in-possession place and wide, beyond reach of both places: the other team
    // has the ball, yet he heads for the nearer place, his in-possession one.
    let (target, pos, a_in, a_out, reach) = target_from(DVec2::new(10.0, -12.0));
    assert!((pos - a_in).length() > reach && (pos - a_out).length() > (pos - a_in).length());
    assert_eq!(target, a_in);
    // Within reach of that place the shape pass decides: out of possession, his target lies
    // toward his out-of-possession place.
    let (target, pos, a_in, a_out, reach) = target_from(DVec2::new(6.0, -6.0));
    assert!((pos - a_in).length() <= reach);
    assert!(target != a_in);
    assert!((target - a_out).length() < (pos - a_out).length());
}

/// A concentration lapse holds a player in the team's shape where he stands, but it never
/// holds one left far from play and from both his places: he heads for the nearer place, as
/// he would without the lapse.
#[test]
fn a_lapse_holds_a_player_in_shape_but_not_one_far_from_play_and_his_place() {
    const LEFT_BACK: usize = 1;
    let lapsing_target = |offset: DVec2| {
        let mut sim = scene_with_stands(0.04);
        hold(&mut sim, DVec2::new(35.0, 25.0));
        let ball = sim.ball.xy();
        let t = sim.tuning.clone();
        let team = sim.teams[0].clone();
        let a_in = team.anchor(LEFT_BACK, ball, true, &t);
        let a_out = team.anchor(LEFT_BACK, ball, false, &t);
        let pos = a_in + DVec2::new(offset.x * team.attack_x, offset.y);
        sim.players[LEFT_BACK].pos = pos;
        sim.players[LEFT_BACK].lapse_until = sim.tick + 100;
        assert!((pos - ball).length() > t.anchor_ball_distance);
        let decided = sim.config.modules.decision.targets(&sim.view());
        let reach = t.anchor_tolerance;
        let far = (pos - a_in).length() > reach && (pos - a_out).length() > reach;
        (decided.targets[LEFT_BACK], pos, a_in, far)
    };
    // Far from play and beyond reach of both places: the lapse does not hold him; he heads
    // for the nearer place, his in-possession one.
    let (target, pos, a_in, far) = lapsing_target(DVec2::new(10.0, -12.0));
    assert!(far);
    assert!(target != pos, "a lapse held a player far out of shape");
    assert_eq!(target, a_in);
    // Within reach of that place he is in the shape, and the lapse holds him where he stands.
    let (target, pos, _, far) = lapsing_target(DVec2::new(6.0, -6.0));
    assert!(!far);
    assert_eq!(target, pos, "a lapse did not hold a player in shape");
}
