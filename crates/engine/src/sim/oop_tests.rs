//! Tests of the out-of-possession hooks: the press triggers fire whatever the plan and a side
//! reads only those it switches on, the line of engagement stops the press beyond it, force
//! the opponent moves only the presser's aim, the offside trap steps the back line up, and the
//! goal-kick screen stands the front players on the short options.

use crate::data::rules::StoppageKind;
use crate::data::test_support::shipped_config;
use crate::decision::{
    FORCE_M, TRAP_STEP_M, TRIGGER_BACK_PASS, TRIGGER_FACING_BACK, TRIGGER_POOR_TOUCH,
    TRIGGER_REACH, TRIGGER_WIDE, engaged, trigger_factor,
};
use crate::math::DVec2;
use crate::modules::DecisionModule;
use crate::sim::Simulation;
use crate::team::PLAYERS_PER_TEAM;

/// A seed-7 match on the shipped content a minute in, the home side on the ball from the
/// kick-off.
fn scene() -> Simulation {
    let mut sim = Simulation::new(shipped_config(7, 90).unwrap()).unwrap();
    sim.tick = 3_000;
    sim
}

/// A home pass received by player `i` at `at`, facing `facing`.
fn receive(sim: &mut Simulation, i: usize, at: DVec2, facing: DVec2) {
    let t = std::sync::Arc::clone(&sim.tuning);
    sim.players[i].pos = at;
    sim.players[i].facing = facing;
    sim.possession_phase = Some(0);
    sim.pass_in_flight = Some(0);
    sim.gain(i, &t);
}

/// A pass received facing the own goal in a wide channel after a too-fast ball fires the
/// poor-touch, facing-back and wide triggers for the other side, whatever its plan; one
/// received facing forward in the centre fires none; a dead ball clears them.
#[test]
fn a_reception_fires_its_triggers_for_the_other_side() {
    let mut sim = scene();
    let ax = sim.teams[0].attack_x;
    let wide = DVec2::new(0.0, 25.0 * ax);
    sim.heavy_mask = 1 << 6;
    receive(&mut sim, 6, wide, DVec2::new(-ax, 0.0));
    let fired = sim.trigger_at[1];
    assert_eq!(fired[TRIGGER_POOR_TOUCH], Some(3_000));
    assert_eq!(fired[TRIGGER_FACING_BACK], Some(3_000));
    assert_eq!(fired[TRIGGER_WIDE], Some(3_000));
    assert_eq!(fired[TRIGGER_BACK_PASS], None);
    assert_eq!(sim.trigger_at[0], [None; 4]);
    assert_eq!(sim.heavy_mask, 0);
    let mut sim = scene();
    receive(&mut sim, 9, DVec2::new(0.0, 0.0), DVec2::new(ax, 0.0));
    assert_eq!(sim.trigger_at, [[None; 4]; 2]);
    sim.trigger_at[1][TRIGGER_WIDE] = Some(3_000);
    let touchline = DVec2::new(0.0, sim.config.pitch().half_width());
    sim.open_dead_ball(
        StoppageKind::ThrowIn,
        StoppageKind::ThrowIn,
        0,
        touchline,
        false,
    );
    assert_eq!(sim.trigger_at, [[None; 4]; 2]);
}

/// Inside a trigger's window the press reach takes its factor only for a side whose plan
/// switches that trigger on, and only while the other side keeps the ball and the window
/// lasts.
#[test]
fn a_side_reads_only_the_triggers_its_plan_switches_on() {
    let mut sim = scene();
    sim.possession_phase = Some(0);
    sim.trigger_at[1][TRIGGER_WIDE] = Some(3_000);
    assert_eq!(trigger_factor(&sim.view(), 1), 1.0);
    sim.teams[1].plan.trigger_back_pass = true;
    assert_eq!(trigger_factor(&sim.view(), 1), 1.0);
    sim.teams[1].plan.trigger_wide = true;
    assert_eq!(trigger_factor(&sim.view(), 1), TRIGGER_REACH);
    sim.tick += 1_000;
    assert_eq!(trigger_factor(&sim.view(), 1), 1.0);
    sim.tick = 3_000;
    sim.possession_phase = Some(1);
    assert_eq!(trigger_factor(&sim.view(), 1), 1.0);
}

/// The line of engagement: at the default share every ball is within it; a lower share stops
/// the press while the ball lies beyond it from the side's own goal line.
#[test]
fn the_press_starts_only_inside_the_line_of_engagement() {
    let mut sim = scene();
    let own = -sim.teams[1].attack_x;
    let half = sim.config.pitch().half_length();
    let near_own = DVec2::new(own * (half - 20.0), 0.0);
    let far = DVec2::new(-own * (half - 20.0), 0.0);
    assert!(engaged(&sim.view(), 1, far));
    sim.teams[1].plan.engage_line = 0.4;
    assert!(engaged(&sim.view(), 1, near_own));
    assert!(!engaged(&sim.view(), 1, far));
}

/// Places a home carrier `c` on the ball at `at` facing up the pitch, one away player `d`
/// pressing him from `gap` metres goal-side, and every other away outfield player far off.
fn press_scene(c: usize, d: usize, at: DVec2, gap: f64) -> Simulation {
    let mut sim = scene();
    let ax = sim.teams[0].attack_x;
    for i in PLAYERS_PER_TEAM + 1..2 * PLAYERS_PER_TEAM {
        sim.players[i].pos = DVec2::new(-ax * 45.0, 0.0);
        sim.players[i].vel = DVec2::ZERO;
    }
    sim.players[c].pos = at;
    sim.players[c].vel = DVec2::ZERO;
    sim.players[c].facing = DVec2::new(ax, 0.0);
    sim.players[d].pos = at + DVec2::new(ax * gap, 0.0);
    sim.ball.pos = at.extend(0.0);
    sim.ball.vel = DVec2::ZERO.extend(0.0);
    sim.carrier = Some(c);
    sim.possession_phase = Some(0);
    sim.last_touch = Some(0);
    sim
}

/// Force the opponent moves only the presser's aim across, toward the centre to show the
/// carrier wide: at force 0 the aim is the show alone, at force 1 it is `FORCE_M` nearer the
/// centre line, every other target unchanged.
#[test]
fn force_moves_only_the_pressers_aim_across() {
    let d = PLAYERS_PER_TEAM + 9;
    let at = DVec2::new(0.0, 12.0);
    let neither = press_scene(9, d, at, 8.0);
    let base = crate::decision::DecisionV1.targets(&neither.view());
    let mut wide = press_scene(9, d, at, 8.0);
    wide.teams[1].plan.force = 1.0;
    let forced = crate::decision::DecisionV1.targets(&wide.view());
    let moved = forced.targets[d] - base.targets[d];
    assert!((moved.x).abs() < 1e-9, "{moved:?}");
    assert!((moved.y + FORCE_M).abs() < 1e-9, "{moved:?}");
    for i in (0..2 * PLAYERS_PER_TEAM).filter(|&i| i != d) {
        assert_eq!(forced.targets[i], base.targets[i], "player {i}");
    }
}

/// The offside trap: a side whose plan sets it starts its window when an opposing carrier
/// faces forward with nobody of the side near him, and its back line steps up together while
/// the window lasts; a side without it never starts one.
#[test]
fn the_offside_trap_steps_the_back_line_up_together() {
    // A home carrier deep in his own half facing up the pitch, every player at his anchor.
    let mut sim = scene();
    let ax = sim.teams[0].attack_x;
    let at = DVec2::new(-ax * 25.0, 0.0);
    sim.players[9].pos = at;
    sim.players[9].vel = DVec2::ZERO;
    sim.players[9].facing = DVec2::new(ax, 0.0);
    sim.ball.pos = at.extend(0.0);
    sim.ball.vel = DVec2::ZERO.extend(0.0);
    sim.carrier = Some(9);
    sim.possession_phase = Some(0);
    sim.last_touch = Some(0);
    for i in 0..2 * PLAYERS_PER_TEAM {
        sim.players[i].vel = DVec2::ZERO;
        sim.players[i].target = sim.players[i].pos;
    }
    let before = crate::decision::DecisionV1.targets(&sim.view());
    sim.arm_trap(1, 9);
    assert_eq!(sim.trap_at[1], Some(3_000));
    let after = crate::decision::DecisionV1.targets(&sim.view());
    let def_ax = sim.teams[1].attack_x;
    let mut stepped = 0;
    for slot in sim.teams[1].back_line() {
        let i = PLAYERS_PER_TEAM + slot;
        let step = (after.targets[i].x - before.targets[i].x) * def_ax;
        // A presser, the cover defender and a player sent back keep their targets; the rest
        // step up together.
        assert!(
            step.abs() < 1e-9 || (step - TRAP_STEP_M).abs() < 1e-9,
            "player {i}: {step}"
        );
        stepped += usize::from(step > 0.0);
    }
    assert!(stepped >= 2, "{stepped} stepped up");
    // With a player of the side near him the carrier is pressed: no trap starts.
    let mut pressed = scene();
    pressed.players[9].pos = at;
    pressed.players[9].facing = DVec2::new(ax, 0.0);
    pressed.players[PLAYERS_PER_TEAM + 9].pos = at + DVec2::new(1.0, 0.0);
    pressed.possession_phase = Some(0);
    pressed.arm_trap(1, 9);
    assert_eq!(pressed.trap_at[1], None);
}
/// Stop short goal kicks: at the other side's goal kick, a screening side's players in front of
/// its back line stand beside the kicking side's short options, outside the penalty area; a
/// side that does not screen stands at its anchors.
#[test]
fn the_screen_stands_the_front_players_on_the_short_options() {
    let mut sim = scene();
    let kicking = 0;
    let own = -sim.teams[kicking].attack_x;
    let half = sim.config.pitch().half_length();
    let spot = DVec2::new(own * (half - 5.5), 3.0);
    let short = DVec2::new(own * (half - 20.0), 14.0);
    sim.players[4].pos = short;
    sim.open_dead_ball(
        StoppageKind::GoalKick,
        StoppageKind::GoalKick,
        kicking,
        spot,
        false,
    );
    let dead = sim.dead_ball().expect("a dead ball");
    let t = std::sync::Arc::clone(&sim.tuning);
    let plain: Vec<DVec2> = (PLAYERS_PER_TEAM..2 * PLAYERS_PER_TEAM)
        .map(|i| crate::rules::restart::target(&dead, i, &sim.players, &sim.teams, &t))
        .collect();
    sim.teams[1].plan.screen_short = true;
    let screened: Vec<DVec2> = (PLAYERS_PER_TEAM..2 * PLAYERS_PER_TEAM)
        .map(|i| crate::rules::restart::target(&dead, i, &sim.players, &sim.teams, &t))
        .collect();
    let near = screened
        .iter()
        .filter(|p| (**p - short).length() < crate::decision::SCREEN_M + 1.0)
        .count();
    let near_before = plain
        .iter()
        .filter(|p| (**p - short).length() < crate::decision::SCREEN_M + 1.0)
        .count();
    assert!(near > near_before, "{near} against {near_before}");
    let back = sim.teams[1].back_line();
    for slot in back {
        assert_eq!(screened[slot], plain[slot], "slot {slot}");
    }
}

/// The tackle decision: a defender tackles with the fouls module's chances unless his win
/// chance is far below the danger-weighted cost of a foul; an unwilling side declines more;
/// a riskier side fouls more in the tackles it makes, never at the cost of a clean win; and a
/// declined tackle draws nothing, so the carrier keeps the ball.
#[test]
fn a_tackle_happens_only_when_the_decision_chooses_it() {
    use crate::decision::tackle_decision;
    use crate::modules::TackleChances;
    let d = PLAYERS_PER_TEAM + 9;
    let mut sim = press_scene(9, d, DVec2::new(0.0, 0.0), 0.5);
    let fair = TackleChances {
        p_win: 0.25,
        p_foul: 0.1,
        ball_loss: 0.5,
    };
    assert_eq!(tackle_decision(&sim.view(), d, 9, fair), Some(fair));
    let hopeless = TackleChances {
        p_win: 0.001,
        p_foul: 0.3,
        ball_loss: 0.5,
    };
    assert_eq!(tackle_decision(&sim.view(), d, 9, hopeless), None);
    sim.teams[1].plan.tackle_willingness = -3.0;
    assert_eq!(tackle_decision(&sim.view(), d, 9, fair), None);
    sim.teams[1].plan.tackle_willingness = 0.0;
    sim.teams[1].plan.tackle_risk = 1.0;
    let risky = tackle_decision(&sim.view(), d, 9, fair).unwrap();
    assert_eq!(risky.p_win, fair.p_win);
    assert!(risky.p_foul > fair.p_foul && risky.p_win + risky.p_foul <= 1.0);
    // A side that declines every tackle: the carrier keeps the ball and no tackle is drawn.
    let mut sim = press_scene(9, d, DVec2::new(0.0, 0.0), 0.5);
    sim.teams[1].plan.tackle_willingness = -1e9;
    sim.control_since = 0;
    let t = std::sync::Arc::clone(&sim.tuning);
    let draws = sim.streams.draws();
    sim.resolve_possession(&t);
    assert_eq!(sim.carrier, Some(9));
    assert_eq!(sim.streams.draws(), draws);
}
