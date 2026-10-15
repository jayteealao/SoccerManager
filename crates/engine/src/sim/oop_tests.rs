//! Tests of the out-of-possession hooks: the press triggers fire whatever the plan and a side
//! reads only those it switches on, the line of engagement stops the press beyond it, force
//! the opponent moves only the presser's aim, the offside trap steps the back line up, and the
//! goal-kick screen stands the front players on the short options.

use crate::data::rules::StoppageKind;
use crate::data::test_support::shipped_config;
use crate::decision::{
    FORCE_M, TRAP_STEP_M, TRIGGER_BACK_PASS, TRIGGER_FACING_BACK, TRIGGER_POOR_TOUCH,
    TRIGGER_PRESSERS, TRIGGER_REACH, TRIGGER_WIDE, engaged, trigger_factor, trigger_pressers,
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

/// Inside a trigger's window the press reach takes its factor and the press its extra players
/// only for a side whose plan switches that trigger on, and only while the other side keeps
/// the ball and the window lasts.
#[test]
fn a_side_reads_only_the_triggers_its_plan_switches_on() {
    let mut sim = scene();
    sim.possession_phase = Some(0);
    sim.trigger_at[1][TRIGGER_WIDE] = Some(3_000);
    assert_eq!(trigger_factor(&sim.view(), 1), 1.0);
    sim.teams[1].plan.trigger_back_pass = true;
    assert_eq!(trigger_factor(&sim.view(), 1), 1.0);
    assert_eq!(trigger_pressers(&sim.view(), 1), 0);
    sim.teams[1].plan.trigger_wide = true;
    assert_eq!(trigger_factor(&sim.view(), 1), TRIGGER_REACH);
    assert_eq!(trigger_pressers(&sim.view(), 1), TRIGGER_PRESSERS);
    sim.tick += 1_000;
    assert_eq!(trigger_factor(&sim.view(), 1), 1.0);
    assert_eq!(trigger_pressers(&sim.view(), 1), 0);
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
/// faces forward with nobody of the side near him, once in his spell on the ball, and its back
/// line steps up together while the window lasts.
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
    // Later in the same spell no new trap starts; a new spell on the ball may start one.
    sim.tick += 200;
    sim.arm_trap(1, 9);
    assert_eq!(sim.trap_at[1], Some(3_000));
    sim.control_since = sim.tick;
    sim.arm_trap(1, 9);
    assert_eq!(sim.trap_at[1], Some(3_200));
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

/// Everyone far from the centre and still; home player 9 at the centre facing the goal he
/// attacks.
fn clear(sim: &mut Simulation) {
    let ax = sim.teams[0].attack_x;
    for i in 0..2 * PLAYERS_PER_TEAM {
        sim.players[i].pos = DVec2::new(-40.0 + i as f64 * 0.1, 30.0);
        sim.players[i].vel = DVec2::ZERO;
    }
    sim.players[9].pos = DVec2::ZERO;
    sim.players[9].facing = DVec2::new(ax, 0.0);
}

/// The pressure model's pass penalty in play: the sideways share of home carrier 9's pass to
/// team-mate 10, `length` metres straight ahead, at the widest aim, with the away players
/// `opps` (each at a place and a velocity, both written for the side attacking +x) and
/// everyone else far away; the pass and pressure stage shares set to `passing` and
/// `composure` when given.
fn pass_noise(
    length: f64,
    opps: &[(DVec2, DVec2)],
    passing: Option<f64>,
    composure: Option<f64>,
) -> f64 {
    use crate::contract::Stage;
    let mut stages = Vec::new();
    if let Some(s) = passing {
        stages.push((Stage::PASS_EXECUTE, s));
    }
    if let Some(s) = composure {
        stages.push((Stage::PASS_PRESSURE, s));
    }
    pass_noise_with(length, opps, &stages)
}

/// [`pass_noise`] with any of the passer's stage shares set.
fn pass_noise_with(
    length: f64,
    opps: &[(DVec2, DVec2)],
    stages: &[(crate::contract::Stage, f64)],
) -> f64 {
    let mut sim = scene();
    clear(&mut sim);
    let ax = sim.teams[0].attack_x;
    sim.players[10].pos = DVec2::new(ax * length, 0.0);
    for (k, &(at, vel)) in opps.iter().enumerate() {
        let o = PLAYERS_PER_TEAM + 2 + k;
        sim.players[o].pos = DVec2::new(at.x * ax, at.y);
        sim.players[o].vel = DVec2::new(vel.x * ax, vel.y);
    }
    for &(stage, s) in stages {
        sim.stages[9].share[stage.index()] = s;
    }
    let kick = crate::decision::DecisionV1.pass_kick(&sim.view(), 9, 10, 1.0, 0.5);
    let crate::decision::Kick::Pass { dir, .. } = kick else {
        panic!("a pass");
    };
    DVec2::new(ax, 0.0).perp_dot(dir).abs()
}

/// Each factor of the pressure model moves a pass's penalty its way (every scene stays below
/// the pressed threshold, so only the graded penalty differs): a nearer opponent, a second
/// one, one closing on the passer, one level with him rather than in front (one behind less),
/// one on the blind side of a passer who does not read the press, a shorter pass (the real
/// data's pressed passes lose most when short) and lower composure each widen the pass; with
/// no opponent within the far distance the pass is as free.
#[test]
fn each_pressure_factor_moves_a_pass_its_way() {
    let still = DVec2::ZERO;
    let front = |m: f64| (DVec2::new(m, 0.0), still);
    let free = pass_noise(12.0, &[], None, None);
    assert!(free > 0.0);
    assert_eq!(
        pass_noise(12.0, &[front(crate::pressure::FAR_M + 0.5)], None, None),
        free,
        "beyond the far distance"
    );
    let at_4 = pass_noise(12.0, &[front(4.0)], None, None);
    let at_3_5 = pass_noise(12.0, &[front(3.5)], None, None);
    assert!(at_4 > free && at_3_5 > at_4, "closer");
    let two = pass_noise(
        12.0,
        &[
            (DVec2::new(4.0, 0.5), still),
            (DVec2::new(4.0, -0.5), still),
        ],
        None,
        None,
    );
    assert!(two > at_4, "more");
    let closing = pass_noise(
        12.0,
        &[(DVec2::new(3.5, 0.0), DVec2::new(-4.0, 0.0))],
        None,
        None,
    );
    assert!(closing > at_3_5, "faster");
    // Where he stands against the direction the passer attacks: level presses more than in
    // front, behind less, as in the real flags.
    let behind = pass_noise(12.0, &[front(-3.5)], None, None);
    let level = pass_noise(12.0, &[(DVec2::new(0.0, 3.5), still)], None, None);
    assert!(behind < at_3_5 && at_3_5 < level, "where he stands");
    // On his blind side, a presser presses a passer who does not read the press harder.
    let read = |s: f64| {
        pass_noise_with(
            12.0,
            &[front(-3.5)],
            &[(crate::contract::Stage::INTERCEPT_SEE, s)],
        )
    };
    assert!(read(0.1) > read(0.9), "blind side, unread");
    let ratio = |length: f64| {
        pass_noise(length, &[front(3.5)], None, None) / pass_noise(length, &[], None, None)
    };
    assert!(ratio(8.0) > ratio(24.0), "shorter pass");
    assert!(ratio(8.0) > ratio(36.0), "shorter pass than a long ball");
    let composed = |s: f64| {
        pass_noise(12.0, &[front(3.5)], None, Some(s)) / pass_noise(12.0, &[], None, Some(s))
    };
    assert!(composed(0.8) < composed(0.3), "higher composure");
    let passer = |s: f64| {
        pass_noise(12.0, &[front(3.5)], Some(s), None) / pass_noise(12.0, &[], Some(s), None)
    };
    assert!(passer(0.8) < passer(0.3), "better passing");
}

/// The pressure model's reception penalty in play: a faster ball, a receiver facing his own
/// goal and a lower reception stage (first touch leading) each raise it; no opponent near
/// leaves none.
#[test]
fn each_reception_factor_moves_the_touch_its_way() {
    use crate::contract::Stage;
    let penalty = |speed: f64, back: bool, first_touch: Option<f64>, opp: f64| {
        let mut sim = scene();
        clear(&mut sim);
        let ax = sim.teams[0].attack_x;
        if back {
            sim.players[9].facing = DVec2::new(-ax, 0.0);
        }
        sim.players[PLAYERS_PER_TEAM + 9].pos = DVec2::new(0.0, opp);
        if let Some(s) = first_touch {
            sim.stages[9].share[Stage::RECEIVE_EXECUTE.index()] = s;
        }
        crate::possession::reception_penalty(&sim.view(), &sim.players[9], speed)
    };
    assert_eq!(penalty(0.5, false, None, crate::pressure::FAR_M + 1.0), 0.0);
    let base = penalty(0.3, false, None, 2.0);
    assert!(base > 0.0);
    assert!(penalty(0.9, false, None, 2.0) > base, "faster ball");
    assert!(penalty(0.3, true, None, 2.0) > base, "facing back");
    assert!(
        penalty(0.3, false, Some(0.8), 2.0) < penalty(0.3, false, Some(0.3), 2.0),
        "better first touch"
    );
    assert!(penalty(0.3, false, None, 1.0) > base, "closer");
}

/// Each attribute of a pass's resistance narrows a pressed pass against the same pass free,
/// at the same pressure: passing (the pass execute stage), composure (its pressure stage),
/// vision (pass see) and decisions (pass choose). Anticipation (intercept see) lets the
/// passer feel a presser on his blind side less, and changes nothing with the presser in
/// front of him.
#[test]
fn each_resistance_attribute_narrows_a_pressed_pass() {
    use crate::contract::Stage;
    let still = DVec2::ZERO;
    let ratio = |opp: (DVec2, DVec2), stage: Stage, s: f64| {
        pass_noise_with(12.0, &[opp], &[(stage, s)]) / pass_noise_with(12.0, &[], &[(stage, s)])
    };
    let front = (DVec2::new(3.5, 0.0), still);
    for stage in [
        Stage::PASS_EXECUTE,
        Stage::PASS_PRESSURE,
        Stage::PASS_SEE,
        Stage::PASS_CHOOSE,
    ] {
        assert!(
            ratio(front, stage, 0.8) < ratio(front, stage, 0.3),
            "{stage:?}"
        );
    }
    let behind = (DVec2::new(-3.5, 0.0), still);
    assert!(
        ratio(behind, Stage::INTERCEPT_SEE, 0.8) < ratio(behind, Stage::INTERCEPT_SEE, 0.3),
        "reads the blind side"
    );
    assert_eq!(
        ratio(front, Stage::INTERCEPT_SEE, 0.8),
        ratio(front, Stage::INTERCEPT_SEE, 0.3),
        "nothing to read in front"
    );
}

/// Each attribute of a reception's resistance lowers a pressed touch's penalty at the same
/// pressure: first touch (receive execute), composure (dribble pressure), strength (shield)
/// and balance (stay-up). Anticipation (intercept see) lowers it for a receiver facing his
/// own goal with the opponent at his side: he feels the presser less and sets his body.
#[test]
fn each_resistance_attribute_lowers_a_pressed_touch() {
    use crate::contract::Stage;
    let penalty = |back: bool, stage: Stage, s: f64| {
        let mut sim = scene();
        clear(&mut sim);
        let ax = sim.teams[0].attack_x;
        if back {
            sim.players[9].facing = DVec2::new(-ax, 0.0);
        }
        sim.players[PLAYERS_PER_TEAM + 9].pos = DVec2::new(0.0, 2.0);
        sim.stages[9].share[stage.index()] = s;
        crate::possession::reception_penalty(&sim.view(), &sim.players[9], 0.3)
    };
    for stage in [
        Stage::RECEIVE_EXECUTE,
        Stage::DRIBBLE_PRESSURE,
        Stage::SHIELD_EXECUTE,
        Stage::STAY_UP_EXECUTE,
    ] {
        assert!(
            penalty(false, stage, 0.8) < penalty(false, stage, 0.3),
            "{stage:?}"
        );
    }
    assert!(
        penalty(true, Stage::INTERCEPT_SEE, 0.8) < penalty(true, Stage::INTERCEPT_SEE, 0.3),
        "reads the press"
    );
}

/// A pressed carrier who dribbles out of the press loses fewer of the tackles it brings: a
/// higher dribble execute stage (dribbling, agility, balance) or dribble pressure stage
/// (composure) lowers the running tackle's win chance against him.
#[test]
fn a_press_resistant_dribbler_loses_fewer_running_tackles() {
    use crate::contract::Stage;
    let d = PLAYERS_PER_TEAM + 9;
    for stage in [Stage::DRIBBLE_EXECUTE, Stage::DRIBBLE_PRESSURE] {
        let chance = |raise: f64| {
            let mut sim = scene();
            sim.stages[9].f[stage.index()] *= raise;
            let t = std::sync::Arc::clone(&sim.tuning);
            crate::rules::fouls::running_win_chance(sim.skills(d), sim.skills(9), &t)
        };
        assert!(chance(1.5) < chance(1.0), "{stage:?}");
        assert!(chance(1.0) < chance(0.6), "{stage:?}");
    }
}

/// Home carrier 9 at the centre running at `speed` toward the goal he attacks, the ball at his
/// feet, and away player `d` at `at` facing `facing` (both written for the side attacking +x);
/// everyone else far away.
fn running_at(speed: f64, d: usize, at: DVec2, facing: DVec2) -> Simulation {
    let mut sim = scene();
    clear(&mut sim);
    let ax = sim.teams[0].attack_x;
    sim.carrier = Some(9);
    sim.control_since = 0;
    sim.possession_phase = Some(0);
    sim.last_touch = Some(0);
    sim.players[9].vel = DVec2::new(speed * ax, 0.0);
    sim.ball.pos = DVec2::new(0.3 * ax, 0.0).extend(0.0);
    sim.ball.vel = DVec2::new(speed * ax, 0.0).extend(0.0);
    sim.players[d].pos = DVec2::new(at.x * ax, at.y);
    sim.players[d].facing = DVec2::new(facing.x * ax, facing.y);
    sim
}

/// A defender in front of a carrier who runs with the ball meets him in a take-on once he is
/// within the pressed distance: his chance is highest straight in the carrier's path, standing
/// square to him, against a carrier who does not run at him fast; it falls at an angle, with
/// his back turned and as the carrier closes faster; level with or behind the carrier, beyond
/// the pressed distance, against a walking carrier, or once already met, there is no contest.
/// The tackler's tackle stage raises it and the carrier's dribble stage lowers it. With the
/// scale raised so the chance is certain, the defender takes the ball in play.
#[test]
fn a_defender_in_front_contests_a_take_on_by_angle_stance_pace_and_attributes() {
    use crate::contract::Stage;
    use crate::modules::PossessionModule;
    let d = PLAYERS_PER_TEAM + 4;
    let facing = DVec2::new(-1.0, 0.0);
    let meet = |sim: &Simulation| crate::possession::PossessionV1.take_on(&sim.view(), 9, d);
    let square = meet(&running_at(3.0, d, DVec2::new(2.0, 0.0), facing)).expect("in his path");
    assert!(square > 0.0);
    let angled = meet(&running_at(3.0, d, DVec2::new(1.4, 1.4), facing)).expect("at an angle");
    assert!(angled < square, "at an angle");
    assert!(
        meet(&running_at(3.0, d, DVec2::new(0.0, 2.0), facing)).is_none(),
        "level with him"
    );
    assert!(
        meet(&running_at(3.0, d, DVec2::new(-2.0, 0.0), facing)).is_none(),
        "behind him"
    );
    assert!(
        meet(&running_at(3.0, d, DVec2::new(3.0, 0.0), facing)).is_none(),
        "beyond the pressed distance"
    );
    assert!(
        meet(&running_at(1.0, d, DVec2::new(2.0, 0.0), facing)).is_none(),
        "a walking carrier"
    );
    let side_on = meet(&running_at(
        3.0,
        d,
        DVec2::new(2.0, 0.0),
        DVec2::new(0.0, 1.0),
    ))
    .unwrap();
    assert!(side_on < square, "standing side-on");
    let fast = meet(&running_at(7.0, d, DVec2::new(2.0, 0.0), facing)).unwrap();
    assert!(fast < square, "a carrier closing faster");
    let mut met = running_at(3.0, d, DVec2::new(2.0, 0.0), facing);
    met.duelled = 1 << d;
    assert!(meet(&met).is_none(), "already met");
    let with = |who: usize, stage: Stage, raise: f64| {
        let mut sim = running_at(3.0, d, DVec2::new(2.0, 0.0), facing);
        sim.stages[who].f[stage.index()] *= raise;
        meet(&sim).unwrap()
    };
    assert!(
        with(d, Stage::TACKLE_EXECUTE, 1.3) > square,
        "a better tackler"
    );
    assert!(
        with(9, Stage::DRIBBLE_EXECUTE, 1.3) < square,
        "a better dribbler"
    );
    // In play: a certain contest takes the ball for the defender.
    let mut sim = running_at(3.0, d, DVec2::new(2.0, 0.0), facing);
    sim.config.tuning.decision.take_on_contest = 5.0;
    let t = std::sync::Arc::clone(&sim.tuning);
    sim.resolve_possession(&t);
    assert_eq!(sim.carrier, Some(d));
}

/// At the same place, a defender who reads the take-on better (anticipation, positioning,
/// decisions and concentration, through the press choose and cover execute stages) wins more
/// of them: standing half-turned, against a carrier closing fast, his chance rises with
/// either stage and falls with a poor read; standing square to a carrier who does not close
/// on him, there is nothing for the read to win back.
#[test]
fn a_defender_who_reads_the_take_on_wins_more_of_them() {
    use crate::contract::Stage;
    use crate::modules::PossessionModule;
    let d = PLAYERS_PER_TEAM + 4;
    let half_turned = DVec2::new(-1.0, 1.0).normalize();
    let meet = |speed: f64, facing: DVec2, read: Option<(Stage, f64)>| {
        let mut sim = running_at(speed, d, DVec2::new(2.0, 0.0), facing);
        if let Some((stage, s)) = read {
            sim.stages[d].share[stage.index()] = s;
        }
        crate::possession::PossessionV1
            .take_on(&sim.view(), 9, d)
            .unwrap()
    };
    for stage in [Stage::PRESS_CHOOSE, Stage::COVER_EXECUTE] {
        let neutral = meet(6.0, half_turned, Some((stage, 0.5)));
        assert!(
            meet(6.0, half_turned, Some((stage, 0.85))) > neutral,
            "{stage:?} a better read"
        );
        assert!(
            meet(6.0, half_turned, Some((stage, 0.2))) < neutral,
            "{stage:?} a poor read"
        );
    }
    let mut both = running_at(6.0, d, DVec2::new(2.0, 0.0), half_turned);
    both.stages[d].share[Stage::PRESS_CHOOSE.index()] = 1.0;
    both.stages[d].share[Stage::COVER_EXECUTE.index()] = 1.0;
    let full = crate::possession::PossessionV1
        .take_on(&both.view(), 9, d)
        .unwrap();
    assert!(
        full > meet(6.0, half_turned, None),
        "anticipation and positioning together"
    );
    // Square to a carrier who barely closes: the read has nothing to win back.
    let still = |s: f64| {
        let mut sim = running_at(3.0, d, DVec2::new(2.0, 0.0), DVec2::new(-1.0, 0.0));
        sim.players[d].vel = DVec2::new(3.0 * sim.teams[0].attack_x, 0.0);
        sim.stages[d].share[Stage::COVER_EXECUTE.index()] = s;
        crate::possession::PossessionV1
            .take_on(&sim.view(), 9, d)
            .unwrap()
    };
    assert!((still(0.9) - still(0.5)).abs() < 1e-12);
}

/// How much more home carrier 9, on the ball in his own third with a free team-mate 10 m away,
/// rates keeping the ball (his best pass, dribble or hold) than clearing it, with one away
/// player pressing him from `gap` metres in front (none when `None`); every stage his keep
/// options' resistance reads (passing, composure, vision, decisions, dribbling, strength,
/// balance) set to `resistance`.
fn keep_margin(resistance: f64, gap: Option<f64>) -> f64 {
    use crate::contract::Stage;
    let mut sim = scene();
    clear(&mut sim);
    let ax = sim.teams[0].attack_x;
    let at = DVec2::new(-30.0 * ax, 0.0);
    sim.players[9].pos = at;
    sim.players[10].pos = at + DVec2::new(8.0 * ax, 6.0);
    if let Some(g) = gap {
        sim.players[PLAYERS_PER_TEAM + 9].pos = at + DVec2::new(g * ax, 0.0);
    }
    sim.ball.pos = at.extend(0.0);
    sim.carrier = Some(9);
    sim.control_since = 0;
    sim.possession_phase = Some(0);
    sim.last_touch = Some(0);
    for stage in [
        Stage::PASS_EXECUTE,
        Stage::PASS_PRESSURE,
        Stage::PASS_SEE,
        Stage::PASS_CHOOSE,
        Stage::DRIBBLE_EXECUTE,
        Stage::DRIBBLE_PRESSURE,
        Stage::DRIBBLE_CHOOSE,
        Stage::SHIELD_EXECUTE,
        Stage::STAY_UP_EXECUTE,
    ] {
        sim.stages[9].share[stage.index()] = resistance;
    }
    let d = crate::decision::DecisionV1.options(&sim.view(), 9);
    let pass = d.passes[..d.pass_count]
        .iter()
        .map(|&(_, s)| s)
        .fold(f64::NEG_INFINITY, f64::max);
    let (dribble, hold) = d.dribble_hold.expect("an outfield carrier");
    let keep = pass.max(hold).max(dribble.unwrap_or(f64::NEG_INFINITY));
    keep - d.clear
}

/// The pressed player's choice: as the press on him rises, a carrier who resists it badly
/// (low composure and decisions, poor passing and dribbling) rates keeping the ball lower
/// against a clearance, so he goes long or clears more often; a press-resistant carrier
/// loses less of it, and keeps the ball.
#[test]
fn a_carrier_who_resists_the_press_badly_clears_more_as_it_rises() {
    let drop = |r: f64, gap: f64| keep_margin(r, None) - keep_margin(r, Some(gap));
    for gap in [1.0, 2.0, 3.0] {
        assert!(drop(0.15, gap) > 0.0, "a weak carrier at {gap} m");
        assert!(
            drop(0.15, gap) > drop(0.85, gap),
            "a weak carrier loses more at {gap} m"
        );
    }
    // Closer is harder for the weak carrier.
    assert!(drop(0.15, 1.0) > drop(0.15, 3.0));
    // At the closest press the resistant carrier still rates keeping it above the weak one.
    assert!(keep_margin(0.85, Some(1.0)) > keep_margin(0.15, Some(1.0)));
}

/// A pass just kicked by home carrier 9 at the centre: the ball `ahead` metres in front of
/// him, running away at `speed`, team-mate 10 waiting 8 m ahead and away player `d` at `at`
/// (written for the side attacking +x).
fn pass_on_its_way(ahead: f64, speed: f64, d: usize, at: DVec2) -> Simulation {
    let mut sim = scene();
    clear(&mut sim);
    let ax = sim.teams[0].attack_x;
    sim.players[10].pos = DVec2::new(8.0 * ax, 0.0);
    sim.players[d].pos = DVec2::new(at.x * ax, at.y);
    sim.ball.pos = DVec2::new(ahead * ax, 0.0).extend(0.0);
    sim.ball.vel = DVec2::new(speed * ax, 0.0).extend(0.0);
    sim.carrier = None;
    sim.last_kicker = Some(9);
    sim.pass_in_flight = Some(0);
    sim.possession_phase = Some(0);
    sim
}

/// The kicker neither plays nor chases his own pass on its way: right after home carrier 9's
/// short pass the ball is within his reach but running away from him, so he cannot touch it
/// and his side's chaser is another player; once it has slowed to a pass's arrival speed, or
/// comes back toward him, it is his again.
#[test]
fn a_passer_does_not_play_his_own_pass_on_its_way() {
    use crate::modules::PossessionModule;
    let far = PLAYERS_PER_TEAM + 9;
    let best = |sim: &Simulation| {
        crate::possession::PossessionV1
            .loose_ball(&sim.view())
            .and_then(|l| l.best)
            .map(|(_, i)| i)
    };
    let sim = pass_on_its_way(0.4, 9.0, far, DVec2::new(-30.0, 20.0));
    assert_eq!(best(&sim), None, "his own pass on its way");
    let chasers = match crate::decision::DecisionV1.targets(&sim.view()).trace {
        crate::modules::TargetsTrace::Loose { nearest, .. } => nearest,
        _ => panic!("a loose ball"),
    };
    assert_ne!(chasers[0], 9, "he does not chase it");
    let slow = pass_on_its_way(0.4, 2.0, far, DVec2::new(-30.0, 20.0));
    assert_eq!(best(&slow), Some(9), "slowed to the arrival speed");
    let back = pass_on_its_way(0.4, -9.0, far, DVec2::new(-30.0, 20.0));
    assert_eq!(best(&back), Some(9), "coming back to him");
}

/// A defender in or near the lane of a short pass can cut it out as it passes him: his chance
/// falls as the ball passes farther from him and as it runs faster; a ball beyond his reach,
/// or faster than he can control, he can only block (a leg out); beyond his stretch he has no
/// try. Every chance grows with his read of the pass (intercept see: anticipation leading) or
/// his block (positioning leading). With the scale raised so the chance is certain, the
/// defender in the lane wins the ball in play.
#[test]
fn a_defender_in_the_lane_can_cut_out_a_short_pass() {
    use crate::contract::Stage;
    use crate::modules::PossessionModule;
    let d = PLAYERS_PER_TEAM + 9;
    let cut = |off: f64, speed: f64, stage: Option<(Stage, f64)>| {
        let mut sim = pass_on_its_way(3.05, speed, d, DVec2::new(3.0, off));
        if let Some((stage, s)) = stage {
            sim.stages[d].share[stage.index()] = s;
        }
        crate::possession::PossessionV1.pass_cut(&sim.view(), d)
    };
    let near = cut(0.2, 8.0, None).expect("in the lane");
    assert!(near.control && near.chance > 0.0);
    assert!(cut(0.6, 8.0, None).unwrap().chance < near.chance, "farther");
    let fast = cut(0.2, 20.0, None).expect("a fast ball");
    assert!(
        !fast.control && fast.chance < near.chance,
        "faster: a block"
    );
    let stretch = cut(1.5, 8.0, None).expect("within his stretch");
    assert!(
        !stretch.control && stretch.chance < near.chance,
        "beyond his reach: a block"
    );
    assert!(cut(2.5, 8.0, None).is_none(), "beyond his stretch");
    let read = |s: f64| {
        cut(0.2, 8.0, Some((Stage::INTERCEPT_SEE, s)))
            .unwrap()
            .chance
    };
    assert!(read(0.8) > read(0.3), "reads the pass");
    let block = |s: f64| {
        cut(0.2, 20.0, Some((Stage::BLOCK_EXECUTE, s)))
            .unwrap()
            .chance
    };
    assert!(block(0.8) > block(0.3), "blocks it");
    // In play: a certain cut wins the ball for the defender.
    let mut sim = pass_on_its_way(3.05, 8.0, d, DVec2::new(3.0, 0.0));
    sim.config.tuning.decision.pass_cut = 5.0;
    let t = std::sync::Arc::clone(&sim.tuning);
    sim.resolve_possession(&t);
    assert_eq!(sim.carrier, Some(d));
}

/// The pressure on the passer spoils the weight of his pass as well as its direction: at the
/// same pressure the pass of a passer with low resistance runs farther from its planned speed,
/// for the same weight draw, than one with high resistance; free, the draw does not move it.
#[test]
fn a_pressed_pass_runs_too_hard_or_too_soft_by_the_passers_resistance() {
    use crate::contract::Stage;
    let speed = |opp: bool, s: f64, weight: f64| {
        let mut sim = scene();
        clear(&mut sim);
        let ax = sim.teams[0].attack_x;
        sim.players[10].pos = DVec2::new(12.0 * ax, 0.0);
        if opp {
            sim.players[PLAYERS_PER_TEAM + 2].pos = DVec2::new(2.0 * ax, 0.0);
        }
        for stage in [Stage::PASS_EXECUTE, Stage::PASS_PRESSURE] {
            sim.stages[9].share[stage.index()] = s;
        }
        let crate::decision::Kick::Pass { speed, .. } =
            crate::decision::DecisionV1.pass_kick(&sim.view(), 9, 10, 0.5, weight)
        else {
            panic!("a pass");
        };
        speed
    };
    assert_eq!(speed(false, 0.5, 1.0), speed(false, 0.5, 0.5), "free");
    let error = |s: f64| (speed(true, s, 1.0) - speed(true, s, 0.5)).abs();
    assert!(error(0.5) > 0.0);
    assert!(error(0.3) > error(0.8), "lower resistance");
    assert!(speed(true, 0.5, 0.0) < speed(true, 0.5, 0.5), "too soft");
}

/// The pressure model's touch: a slow ball near a home receiver. Receiving his side's pass
/// free, he reaches it; pressed hard enough (an away player 2 m away, the scale raised so the
/// cut passes the ball's distance) he reaches it only within the cut share of his reach, so
/// the ball runs on; with the opponent beyond the far distance he reaches it; on the other
/// side's pass he defends it and keeps his full reach, since the model reads only the
/// receiving side.
#[test]
fn a_pressed_receiver_reaches_his_pass_only_within_the_cut_share() {
    use crate::modules::PossessionModule;
    let r = 9;
    let d = PLAYERS_PER_TEAM + 9;
    let place = |touch: f64, gap: f64, passing: usize| {
        let mut sim = scene();
        sim.config.tuning.decision.touch_under_pressure = touch;
        let reach = sim.config.tuning.reach_radius * sim.players[r].derived.knobs.intercept;
        clear(&mut sim);
        sim.players[d].pos = DVec2::new(0.0, gap);
        sim.ball.pos = DVec2::new(0.8 * reach, 0.0).extend(0.0);
        sim.ball.vel = DVec2::new(1.0, 0.0).extend(0.0);
        sim.carrier = None;
        sim.pass_in_flight = Some(passing);
        sim.possession_phase = Some(passing);
        crate::possession::PossessionV1
            .loose_ball(&sim.view())
            .and_then(|l| l.best)
            .map(|(_, i)| i)
    };
    assert_eq!(place(0.0, 2.0, 0), Some(r), "free of the model");
    assert_eq!(
        place(4.0, crate::pressure::FAR_M + 1.0, 0),
        Some(r),
        "no opponent within the far distance"
    );
    assert_eq!(place(4.0, 2.0, 0), None, "pressed: the ball runs on");
    assert_eq!(
        place(4.0, 2.0, 1),
        Some(r),
        "defending the other side's pass"
    );
}

/// A press trigger's urgent press job: inside the window of a trigger the side switches on,
/// its nearest outfield player goes for the carrier even from beyond his reach; with the
/// trigger off he keeps his place.
#[test]
fn inside_a_trigger_window_the_nearest_player_goes_for_the_carrier() {
    let d = PLAYERS_PER_TEAM + 9;
    let at = DVec2::new(0.0, 12.0);
    let gap = |sim: &Simulation| {
        let t = crate::decision::DecisionV1.targets(&sim.view());
        (t.targets[d] - at).length()
    };
    let mut off = press_scene(9, d, at, 25.0);
    off.trigger_at[1][TRIGGER_WIDE] = Some(3_000);
    let mut on = press_scene(9, d, at, 25.0);
    on.trigger_at[1][TRIGGER_WIDE] = Some(3_000);
    on.teams[1].plan.trigger_wide = true;
    assert!(gap(&off) > 10.0, "{}", gap(&off));
    assert!(gap(&on) < 3.0, "{}", gap(&on));
}

/// Home carrier 6 on the ball at the centre spot, every away outfield player standing at
/// 8 m deep in the home side's attacking direction and moving toward the halfway line at
/// `step` m/s (a line that steps up), home player 10's off-ball run stage share at `share`;
/// returns how far beyond the live onside line home player 10's target lies.
fn beyond_the_line(share: f64, step: f64) -> f64 {
    use crate::contract::Stage;
    let mut sim = scene();
    let ax = sim.teams[0].attack_x;
    let line = 8.0;
    let keeper = sim.keeper(1);
    for i in PLAYERS_PER_TEAM..2 * PLAYERS_PER_TEAM {
        if i == keeper {
            continue;
        }
        sim.players[i].pos = DVec2::new(ax * line, -25.0 + (i - PLAYERS_PER_TEAM) as f64 * 5.0);
        sim.players[i].vel = DVec2::new(-ax * step, 0.0);
    }
    sim.players[6].pos = DVec2::ZERO;
    sim.players[6].vel = DVec2::ZERO;
    sim.ball.pos = DVec2::ZERO.extend(0.0);
    sim.ball.vel = DVec2::ZERO.extend(0.0);
    sim.carrier = Some(6);
    sim.possession_phase = Some(0);
    sim.last_touch = Some(0);
    sim.stages[10].share[Stage::OFF_BALL_RUN_EXECUTE.index()] = share;
    let t = crate::decision::DecisionV1.targets(&sim.view());
    t.targets[10].x * ax - (line - crate::decision::ONSIDE_MARGIN)
}

/// The runner's late read of the line: while the line steps up, an attacker who times his
/// runs less well (a lower off-ball run share) reads the line where it stood up to
/// `OFFSIDE_LAG_S` seconds ago, so his read trails the live line farther and he stands
/// farther beyond it: he is caught offside more. One at the highest share reads it live; a
/// line that drops back is read late the other way.
#[test]
fn a_runner_who_reads_the_line_late_is_caught_beyond_it() {
    use crate::contract::Stage;
    use crate::decision::{OFFSIDE_LAG_S, line_lag};
    let mut sim = scene();
    let read = |sim: &mut Simulation, share: f64, speed: f64| {
        sim.stages[10].share[Stage::OFF_BALL_RUN_EXECUTE.index()] = share;
        line_lag(&sim.view(), 10, speed)
    };
    let (low, mid, high) = (
        read(&mut sim, 0.6, -2.0),
        read(&mut sim, 0.8, -2.0),
        read(&mut sim, 1.0, -2.0),
    );
    assert!(low > mid && mid > high, "{low} {mid} {high}");
    assert!((read(&mut sim, 0.0, -2.0) - 2.0 * OFFSIDE_LAG_S).abs() < 1e-9);
    assert!(high.abs() < 1e-9, "{high}");
    assert!(read(&mut sim, 0.6, 2.0) < 0.0);
    assert_eq!(read(&mut sim, 0.6, 0.0), 0.0);
    // In play, on a line that steps up: the lower his share, the farther beyond the live
    // onside line his target; at the highest share the step changes nothing.
    let (low, mid, high) = (
        beyond_the_line(0.6, 2.0),
        beyond_the_line(0.8, 2.0),
        beyond_the_line(1.0, 2.0),
    );
    assert!(low > mid && mid > high, "{low} {mid} {high}");
    assert!(beyond_the_line(0.6, 2.0) > beyond_the_line(0.6, 0.0));
    assert!((beyond_the_line(1.0, 2.0) - beyond_the_line(1.0, 0.0)).abs() < 1e-9);
}
