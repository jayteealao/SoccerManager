//! Keeper and shots. A shot is on target when its flight crosses the goal line between the
//! posts and under the bar, counted as it is struck. A keeper tries to save only such a
//! shot, and holds or parries the save; an outfield defender near the ball can block a
//! shot; a parry or a block leaves the defending side as the last touch, so a corner comes
//! only from a ball that crossed the goal line off a defender. Penalties use the same save
//! model.
//!
//! The fast scenes play one shot each: wide, over the bar, blocked, held, parried, scored.
//! The criterion tests are slow: `cargo test --release -p engine --all-features --test
//! keeper_and_shots -- --include-ignored --nocapture`.
//! - Set pieces arise from play: over 200 matches, at least 1.2 corners per team and 10 goal
//!   kicks per match, every corner after the ball crossed the goal line off a defender. The
//!   floor covers corners from saves and blocks only; corners from clearances and crosses
//!   are not modelled yet, so the real-football band of 3.5 to 6.5 is not asserted here.
//! - The outcome census and the expected-goals refit print figures for the tuning loop.

mod common;

use common::{calm_match, content, default_teams, index, run_many, spread};
use engine::data::StoppageKind;
use engine::math::{DVec2, DVec3};
use engine::pitch::{self, Pitch};
use engine::rules::restart;
use engine::scenario::Scene;
use engine::{MatchConfig, Simulation};

/// The home striker, the away keeper, and an away centre-back.
const SHOOTER: usize = 9;
const AWAY_KEEPER: usize = 11;
const DEFENDER: usize = 13;

/// A calm match on `seed` with everyone spread away from the away goal, the home striker on
/// the ball at `(30, 0)` and the away keeper at `keeper`, 2.5 m off his line. The home side
/// attacks `x = 52.5` in the first half.
fn shot_scene(seed: u64, keeper: DVec2) -> Scene {
    let mut config = calm_match(90);
    config.seed = seed;
    let probe = Scene::new(config.clone()).build();
    assert_eq!(probe.teams()[0].attack_x, 1.0);
    let from = DVec2::new(30.0, 0.0);
    spread(Scene::new(config), -30.0, -30.0)
        .place(index(0, SHOOTER), from)
        .place(AWAY_KEEPER, keeper)
        .ball(DVec3::new(from.x, from.y, 0.0))
        .carrier(Some(index(0, SHOOTER)))
        .tick(1_001)
}

/// The direction from the shooter at `(30, 0)` to `aim` on the goal line.
fn toward(aim_y: f64) -> DVec2 {
    DVec2::new(Pitch::DEFAULT.half_length() - 30.0, aim_y)
}

/// Steps `sim` until play stops, the keeper holds the ball, or `ticks` pass.
fn play(sim: &mut Simulation, ticks: u32) {
    for _ in 0..ticks {
        sim.step();
        if sim.dead_ball().is_some() || sim.carrier().is_some() {
            break;
        }
    }
}

#[test]
fn a_wide_shot_is_not_on_target_is_not_saved_and_gives_a_goal_kick() {
    let post = pitch::GOAL_WIDTH / 2.0;
    // The keeper stands in the ball's path: a save roll, scripted to succeed, would stop it.
    let mut sim = shot_scene(42, DVec2::new(50.0, 5.0))
        .rolls(&[0.0])
        .shoot(toward(post + 2.0), 27.0, 0.0)
        .build();
    assert_eq!(sim.summary().shots, [1, 0]);
    assert_eq!(sim.summary().shots_on_target, [0, 0]);
    assert_eq!(sim.shot_flight(), Some(false));
    play(&mut sim, 200);
    assert_eq!(sim.carrier(), None, "the keeper never took the ball");
    let dead = sim.dead_ball().expect("the ball went out");
    assert_eq!(dead.kind, StoppageKind::GoalKick);
    assert_eq!(dead.team, 1, "the defending side takes the goal kick");
    assert_eq!(sim.summary().shots_on_target, [0, 0]);
    assert_eq!(sim.shot_census().wide, 1);
}

#[test]
fn a_shot_over_the_bar_is_not_on_target_and_gives_a_goal_kick() {
    let mut sim = shot_scene(42, DVec2::new(50.0, 0.0))
        .rolls(&[0.0, 0.0])
        .shoot(toward(0.0), 27.0, 9.0)
        .build();
    assert_eq!(sim.summary().shots_on_target, [0, 0]);
    play(&mut sim, 200);
    assert_eq!(sim.carrier(), None, "the keeper never took the ball");
    let dead = sim.dead_ball().expect("the ball went out");
    assert_eq!(dead.kind, StoppageKind::GoalKick);
    assert_eq!(dead.team, 1);
    assert_eq!(sim.summary().goals, [0, 0]);
}

#[test]
fn a_defender_in_the_lane_blocks_the_shot() {
    let mut sim = shot_scene(42, DVec2::new(50.0, 0.0))
        .place(DEFENDER, DVec2::new(34.0, 0.0))
        .rolls(&[0.0])
        .shoot(toward(0.0), 27.0, 0.0)
        .build();
    assert_eq!(sim.summary().shots_on_target, [1, 0]);
    for _ in 0..20 {
        sim.step();
        if sim.last_touch() == Some(1) {
            break;
        }
    }
    assert_eq!(sim.last_touch(), Some(1), "the defender touched it last");
    assert_eq!(sim.shot_flight(), None, "the shot is over");
    assert_eq!(sim.shot_census().blocked, 1);
    assert!(
        sim.record().ball[0] < 36.0,
        "the ball came back off the block"
    );
    assert_eq!(sim.summary().shots_on_target, [1, 0], "counted once");
}

#[test]
fn a_saved_shot_the_keeper_holds_is_his() {
    let mut sim = shot_scene(42, DVec2::new(50.0, 0.0))
        .rolls(&[0.0, 0.0])
        .shoot(toward(0.0), 27.0, 0.0)
        .build();
    play(&mut sim, 200);
    assert_eq!(sim.carrier(), Some(AWAY_KEEPER));
    assert_eq!(sim.summary().shots_on_target, [1, 0]);
    assert_eq!(sim.summary().goals, [0, 0]);
    assert_eq!(sim.shot_census().held, 1);
}

#[test]
fn a_parried_shot_leaves_the_defending_side_last_and_can_give_a_corner() {
    let mut corners = 0;
    for seed in 1..=40 {
        let mut sim = shot_scene(seed, DVec2::new(50.0, 1.0))
            .rolls(&[0.0, 0.99])
            .shoot(toward(1.5), 27.0, 0.0)
            .build();
        let mut parried = false;
        let mut before = sim.record();
        for _ in 0..300 {
            let touch = sim.last_touch();
            sim.step();
            if !parried && sim.shot_census().parried == 1 {
                parried = true;
                assert_eq!(sim.last_touch(), Some(1), "seed {seed}");
                assert_eq!(sim.shot_flight(), None, "seed {seed}");
            }
            if let Some(dead) = sim.dead_ball() {
                if dead.kind == StoppageKind::Corner {
                    corners += 1;
                    assert_eq!(touch, Some(1), "seed {seed}: a defender touched it last");
                    assert!(
                        before.ball[0] >= (Pitch::DEFAULT.half_length() - 0.81) as f32,
                        "seed {seed}: the ball was at {:?} before the corner",
                        before.ball
                    );
                }
                break;
            }
            if sim.carrier().is_some() {
                break;
            }
            before = sim.record();
        }
        assert!(parried, "seed {seed}: no parry");
        assert_eq!(sim.summary().shots_on_target, [1, 0], "seed {seed}");
    }
    assert!(corners >= 1, "no parry went behind for a corner");
    eprintln!("parries that gave a corner: {corners} of 40");
}

#[test]
fn a_shot_that_beats_the_keeper_scores_and_is_on_target() {
    let mut sim = shot_scene(42, DVec2::new(50.0, 0.0))
        .rolls(&[0.99])
        .shoot(toward(0.0), 27.0, 0.0)
        .build();
    play(&mut sim, 200);
    assert_eq!(sim.summary().goals, [1, 0]);
    assert_eq!(sim.summary().shots_on_target, [1, 0]);
    assert_eq!(sim.shot_census().scored, 1);
}

/// Plays the penalty of seed `seed`: the home side on odd seeds, the away side on even, taken
/// by the player the engine picks. `true` when it scored.
fn penalty(seed: u64) -> bool {
    let mut config = calm_match(90);
    config.seed = seed;
    let team = usize::from(seed.is_multiple_of(2));
    let probe = Scene::new(config.clone()).build();
    let spot = Pitch::DEFAULT.penalty_spot(probe.teams()[team].attack_x);
    let teams = probe.teams();
    let taker = restart::taker(StoppageKind::Penalty, team, spot, probe.players(), &teams);
    let mut sim = Scene::new(config).tick(1_001).penalty(team, taker).build();
    for _ in 0..250 {
        sim.step();
        if sim.summary().goals[team] > 0 {
            return true;
        }
        if sim.dead_ball().is_some() || sim.carrier().is_some() {
            return false;
        }
    }
    false
}

#[test]
fn penalties_convert_seventy_to_eighty_five_percent() {
    let scored = run_many(1..=500, penalty)
        .into_iter()
        .filter(|&s| s)
        .count();
    let rate = scored as f64 / 500.0;
    eprintln!("penalties: {scored} of 500 scored, {rate:.3}");
    assert!((0.70..=0.85).contains(&rate), "{rate}");
}

/// The 90-minute match of the default clubs on `seed`.
fn full(seed: u64) -> MatchConfig {
    let content = content();
    let [a, b] = default_teams(&content);
    MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap()
}

#[test]
#[ignore = "slow: cargo test --release -p engine --all-features -- --ignored"]
fn set_pieces_arise_from_play() {
    // Per match: corners, goal kicks, and corners that did not follow a defender's touch
    // over the goal line.
    let results = run_many(1..=200, |seed| {
        let mut sim = Simulation::new(full(seed)).unwrap();
        let mut bad = Vec::new();
        while !sim.is_over() {
            let before = sim.record();
            let touch = sim.last_touch();
            sim.step();
            if let Some(stoppage) = sim.stoppage()
                && stoppage.kind == StoppageKind::Corner
            {
                let attacking = stoppage.team.expect("a corner has a team");
                let near_line =
                    f64::from(before.ball[0].abs()) >= Pitch::DEFAULT.half_length() - 0.81;
                if !near_line || touch != Some(1 - attacking) {
                    bad.push(format!(
                        "seed {seed} tick {}: ball {:?}, last touch {touch:?}",
                        sim.tick(),
                        before.ball
                    ));
                }
            }
        }
        sim.finish();
        let s = sim.summary();
        (s.corners, s.goal_kicks, bad)
    });
    let n = results.len() as f64;
    let corners: u32 = results.iter().map(|r| r.0[0] + r.0[1]).sum();
    let goal_kicks: u32 = results.iter().map(|r| r.1[0] + r.1[1]).sum();
    let bad: Vec<&String> = results.iter().flat_map(|r| &r.2).collect();
    let per_team = f64::from(corners) / (2.0 * n);
    let per_match = f64::from(goal_kicks) / n;
    eprintln!("corners per team {per_team:.3}, goal kicks per match {per_match:.3}");
    assert!(
        bad.is_empty(),
        "{} corners without a crossing: {bad:?}",
        bad.len()
    );
    assert!(per_team >= 1.2, "corners per team {per_team:.3}");
    assert!(per_match >= 10.0, "goal kicks per match {per_match:.3}");
}

#[test]
#[ignore = "slow: cargo test --release -p engine --all-features -- --ignored"]
fn shot_outcome_census() {
    let results = run_many(1..=200, |seed| {
        let mut sim = Simulation::new(full(seed)).unwrap();
        sim.run(&mut engine::record::NullSink).unwrap();
        let s = sim.summary();
        (
            sim.shot_census(),
            s.shots[0] + s.shots[1],
            s.shots_on_target[0] + s.shots_on_target[1],
            s.goals[0] + s.goals[1],
        )
    });
    let shots: u32 = results.iter().map(|r| r.1).sum();
    let on_target: u32 = results.iter().map(|r| r.2).sum();
    let goals: u32 = results.iter().map(|r| r.3).sum();
    let sum = |f: fn(&engine::sim::ShotCensus) -> u32| results.iter().map(|r| f(&r.0)).sum::<u32>();
    let rows = [
        ("wide or over", sum(|c| c.wide)),
        ("blocked", sum(|c| c.blocked)),
        ("held", sum(|c| c.held)),
        ("parried", sum(|c| c.parried)),
        ("scored", sum(|c| c.scored)),
    ];
    let named: u32 = rows.iter().map(|r| r.1).sum();
    let share = |k: u32| f64::from(k) / f64::from(shots.max(1));
    eprintln!(
        "shots {shots} ({:.2} per team per match), on target {:.3}, goals {goals}",
        f64::from(shots) / 400.0,
        share(on_target)
    );
    for (name, k) in rows {
        eprintln!("  {name:<13} {k:>6} {:.3}", share(k));
    }
    eprintln!(
        "  {:<13} {:>6} {:.3}",
        "other",
        shots - named,
        share(shots - named)
    );
    assert!(named <= shots);
}

/// The distance to the goal centre and the angle the goal mouth subtends, as the
/// expected-goals model reads them.
fn features(from: DVec2, attack_x: f64) -> (f64, f64) {
    let x = Pitch::DEFAULT.half_length() * attack_x;
    let half = pitch::GOAL_WIDTH / 2.0;
    let a = DVec2::new(x, half) - from;
    let b = DVec2::new(x, -half) - from;
    let angle = engine::math::atan2(a.perp_dot(b), a.dot(b)).abs();
    let distance = (Pitch::DEFAULT.goal_centre(attack_x) - from).length();
    (distance, angle)
}

/// Fits `p = 1 / (1 + exp(-(b0 + b1 d + b2 a)))` by iteratively reweighted least squares.
fn fit_logistic(rows: &[(f64, f64, bool)]) -> [f64; 3] {
    let mut beta = [0.0f64; 3];
    for _ in 0..50 {
        let mut h = [[0.0f64; 3]; 3];
        let mut g = [0.0f64; 3];
        for &(d, a, y) in rows {
            let x = [1.0, d, a];
            let z: f64 = (0..3).map(|k| beta[k] * x[k]).sum();
            let p = 1.0 / (1.0 + engine::math::exp(-z));
            let w = p * (1.0 - p);
            let r = f64::from(u8::from(y)) - p;
            for i in 0..3 {
                g[i] += x[i] * r;
                for j in 0..3 {
                    h[i][j] += w * x[i] * x[j];
                }
            }
        }
        let step = solve3(h, g);
        for k in 0..3 {
            beta[k] += step[k];
        }
        if step.iter().all(|s| s.abs() < 1e-10) {
            break;
        }
    }
    beta
}

/// Solves the 3 by 3 system `m x = v` by Cramer's rule.
fn solve3(m: [[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(m);
    [0, 1, 2].map(|c| {
        let mut mc = m;
        for r in 0..3 {
            mc[r][c] = v[r];
        }
        det(mc) / d
    })
}

#[test]
#[ignore = "slow: cargo test --release -p engine --all-features -- --ignored"]
fn xg_refit() {
    // Each open-play shot: its distance and angle, and whether that shot scored.
    let rows: Vec<(f64, f64, bool)> = run_many(1..=1000, |seed| {
        let mut sim = Simulation::new(full(seed)).unwrap();
        let mut rows = Vec::new();
        let mut pending: Option<(usize, f64, f64)> = None;
        while !sim.is_over() {
            let before = sim.record();
            let penalty = sim
                .dead_ball()
                .is_some_and(|d| d.kind == StoppageKind::Penalty);
            let shots = sim.summary().shots;
            let goals = sim.summary().goals;
            sim.step();
            if let Some((team, d, a)) = pending {
                if sim.summary().goals[team] > goals[team] {
                    rows.push((d, a, true));
                    pending = None;
                } else if sim.shot_flight().is_none() || sim.summary().shots != shots {
                    rows.push((d, a, false));
                    pending = None;
                }
            }
            let now = sim.summary().shots;
            for (team, (&after, &was)) in now.iter().zip(&shots).enumerate() {
                if after > was && !penalty {
                    let from = DVec2::new(f64::from(before.ball[0]), f64::from(before.ball[1]));
                    let (d, a) = features(from, sim.teams()[team].attack_x);
                    pending = Some((team, d, a));
                }
            }
        }
        rows
    })
    .into_iter()
    .flatten()
    .collect();
    let goals = rows.iter().filter(|r| r.2).count();
    let beta = fit_logistic(&rows);
    eprintln!(
        "open-play shots {}, goals {goals}; intercept {:.4}, distance_coef {:.4}, angle_coef {:.4}",
        rows.len(),
        beta[0],
        beta[1],
        beta[2]
    );
    let predicted: f64 = rows
        .iter()
        .map(|&(d, a, _)| 1.0 / (1.0 + engine::math::exp(-(beta[0] + beta[1] * d + beta[2] * a))))
        .sum();
    eprintln!("fitted expected goals {predicted:.1} against {goals} goals");
    assert!(rows.len() > 1000);
}
