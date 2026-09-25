//! Tempo and restarts. Only open-play passes count in `stats.passes`: a clearance and a
//! restart taker's first kick are counted under their own keys. A throw-in waits for its
//! tuned delay. The ball-in-play figure is the live ticks of the match. A
//! defender can clear a fast pass inside his own penalty area, and a corner still comes only
//! from a ball that crossed the goal line off a defender.
//!
//! The criterion tests are slow: `cargo test --release -p engine --all-features --test
//! tempo_and_restarts -- --include-ignored --nocapture`. They share one run of 200 matches
//! of the default clubs on seeds 1 to 200.
//! - Passes are realistic: 350 to 550 passes per team, 75 to 88% completed.
//! - The ball is in play for about an hour: 52 to 65 minutes per 90.
//! - Throw-ins stay in band: 35 to 55 per match.
//! - The restart census prints each restart's median dead time against its source and the
//!   corners by origin, and asserts only that every corner followed a crossing.

mod common;

use std::sync::OnceLock;

use common::{calm_match, content, default_teams, index, run_many, short_match, spread};
use engine::math::{DVec2, DVec3};
use engine::observe::MatchFigures;
use engine::pitch;
use engine::record::NullSink;
use engine::rules::restart;
use engine::scenario::Scene;
use engine::{MatchConfig, Simulation, StoppageKind};

/// The ball rolls from `at` with velocity `v` after `last` touched it, with every player far
/// away, in a match where nobody decides. Steps until the restart the ball gives is taken.
fn restart_after(at: DVec2, v: DVec2, last: usize) -> (Simulation, StoppageKind, usize) {
    let mut sim = spread(Scene::new(calm_match(90)), -30.0, 30.0)
        .carrier(None)
        .ball(DVec3::new(at.x, at.y, 0.0))
        .ball_velocity(DVec3::new(v.x, v.y, 0.0))
        .last_touch(last)
        .tick(1_001)
        .build();
    let mut dead = None;
    for _ in 0..10_000 {
        sim.step();
        match (sim.dead_ball(), dead) {
            (Some(d), None) => dead = Some(d),
            (None, Some(d)) => return (sim, d.kind, d.team),
            _ => {}
        }
    }
    panic!("no restart was taken");
}

#[test]
fn a_clearance_is_not_a_pass_and_is_never_completed() {
    let clearer = index(0, 9);
    let mate = index(0, 10);
    let mut sim = spread(Scene::new(calm_match(90)), -30.0, 30.0)
        .place(clearer, DVec2::new(0.0, 10.0))
        .place(mate, DVec2::new(0.0, 15.0))
        .carrier(Some(clearer))
        .ball(DVec3::new(0.0, 11.2, 0.0))
        .tick(1_001)
        .clear(DVec2::Y, 7.0, 0.0)
        .build();
    let s = sim.summary();
    assert_eq!(
        (s.passes, s.clearances, s.restart_kicks),
        ([0, 0], [1, 0], [0, 0])
    );
    for _ in 0..200 {
        sim.step();
        if sim.carrier().is_some() {
            break;
        }
    }
    assert_eq!(
        sim.carrier(),
        Some(mate),
        "a team-mate gained the cleared ball"
    );
    let s = sim.summary();
    assert_eq!(
        (s.passes, s.passes_completed, s.clearances),
        ([0, 0], [0, 0], [1, 0])
    );
}

#[test]
fn a_throw_in_a_corner_and_a_goal_kick_are_restart_kicks() {
    // Over the touchline off the home side: the away side throws in.
    let (sim, kind, team) = restart_after(DVec2::new(0.0, 32.0), DVec2::new(0.0, 8.0), 0);
    assert_eq!((kind, team), (StoppageKind::ThrowIn, 1));
    let s = sim.summary();
    assert_eq!(
        (s.restart_kicks, s.passes, s.clearances),
        ([0, 1], [0, 0], [0, 0])
    );
    // Over the away goal line off the away side: a corner to the home side.
    let (sim, kind, team) = restart_after(DVec2::new(50.0, 25.0), DVec2::new(8.0, 0.0), 1);
    assert_eq!((kind, team), (StoppageKind::Corner, 0));
    let s = sim.summary();
    assert_eq!(
        (s.restart_kicks, s.passes, s.clearances),
        ([1, 0], [0, 0], [0, 0])
    );
    // Over the away goal line off the home side: a goal kick to the away side.
    let (sim, kind, team) = restart_after(DVec2::new(50.0, 25.0), DVec2::new(8.0, 0.0), 0);
    assert_eq!((kind, team), (StoppageKind::GoalKick, 1));
    let s = sim.summary();
    assert_eq!(
        (s.restart_kicks, s.passes, s.clearances),
        ([0, 1], [0, 0], [0, 0])
    );
}

#[test]
fn the_kick_off_is_a_restart_kick_and_the_next_pass_is_a_pass() {
    let mut sim = Simulation::new(short_match(5)).unwrap();
    let kicks = |s: &engine::sim::Summary| {
        (0..2)
            .map(|t| s.passes[t] + s.clearances[t] + s.restart_kicks[t] + s.shots[t])
            .sum::<u32>()
    };
    while kicks(&sim.summary()) == 0 {
        sim.step();
    }
    let s = sim.summary();
    assert_eq!(
        (s.restart_kicks, s.passes),
        ([1, 0], [0, 0]),
        "the kick-off"
    );
    let mut guard = 0;
    while kicks(&sim.summary()) == 1 && sim.dead_ball().is_none() {
        sim.step();
        guard += 1;
        assert!(guard < 15_000, "no second kick");
    }
    let s = sim.summary();
    assert_eq!(
        s.restart_kicks,
        [1, 0],
        "the next kick is not a restart kick"
    );
    assert_eq!(kicks(&s), 2);
}

#[test]
fn a_throw_in_waits_for_its_delay() {
    let mut sim = spread(Scene::new(calm_match(90)), -30.0, 30.0)
        .carrier(None)
        .ball(DVec3::new(0.0, 32.0, 0.0))
        .ball_velocity(DVec3::new(0.0, 8.0, 0.0))
        .last_touch(0)
        .tick(1_001)
        .build();
    let mut dead = None;
    for _ in 0..10_000 {
        sim.step();
        match (sim.dead_ball(), dead) {
            (Some(d), None) => dead = Some(d),
            (None, Some(d)) => {
                assert_eq!(d.kind, StoppageKind::ThrowIn);
                let delay = restart::delay_ticks(StoppageKind::ThrowIn, sim.tuning());
                assert!(delay > 0);
                assert!(
                    sim.tick() >= d.since + delay,
                    "taken at tick {}, dead since {}",
                    sim.tick(),
                    d.since
                );
                return;
            }
            _ => {}
        }
    }
    panic!("the throw-in was never taken");
}

#[test]
fn the_ball_in_play_figure_is_the_live_ticks() {
    let mut sim = Simulation::new(short_match(5)).unwrap();
    sim.run(&mut NullSink).unwrap();
    let s = sim.summary();
    assert!(s.live_ticks > 0 && s.dead_ball_ticks > 0);
    assert_eq!(s.live_ticks + s.dead_ball_ticks, sim.tick());
    let figures = MatchFigures::new(&s, sim.managers());
    assert_eq!(figures.ball_in_play_s, (s.live_ticks + 25) / 50);
}

/// The home carrier holds the ball with an away player `gap` metres from it, and a tackle
/// may come from `reach` metres. The first step judges any tackle with a roll that wins the
/// ball.
fn contested(gap: f64, reach: f64) -> Simulation {
    let mut config = calm_match(90);
    config.tuning.tackle_reach = reach;
    let at = DVec2::new(0.0, 10.0);
    let mut sim = spread(Scene::new(config), -30.0, 30.0)
        .place(index(0, 5), at)
        .place(index(1, 5), at + DVec2::new(gap, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(index(0, 5)))
        .tick(1_001)
        .rolls(&[0.0])
        .build();
    sim.step();
    sim
}

#[test]
fn an_opponent_inside_the_tackle_reach_can_win_the_ball() {
    assert_eq!(
        contested(1.6, 1.0).carrier(),
        Some(index(0, 5)),
        "1.6 m is outside a 1 m reach"
    );
    assert_eq!(
        contested(1.6, 2.0).carrier(),
        Some(index(1, 5)),
        "1.6 m is inside a 2 m reach"
    );
}

/// The home carrier runs at `speed` metres per second with an away player 0.5 m from the
/// ball. The tackle roll lies between the standing win chance and the running win chance.
fn running(speed: f64) -> Simulation {
    let mut config = calm_match(90);
    config.tuning.tackle_dribble_win = 0.5;
    let sim = Simulation::new(config.clone()).unwrap();
    let (tackler, carrier) = (
        sim.players()[index(1, 5)].derived,
        sim.players()[index(0, 5)].derived,
    );
    let base = engine::rules::fouls::win_chance(&tackler, &carrier, &config.tuning);
    let extra = engine::rules::fouls::dribble_win_chance(&tackler, &carrier, &config.tuning);
    let at = DVec2::new(0.0, 10.0);
    let mut sim = spread(Scene::new(config), -30.0, 30.0)
        .place(index(0, 5), at)
        .velocity(index(0, 5), DVec2::new(0.0, speed))
        .place(index(1, 5), at + DVec2::new(0.5, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(index(0, 5)))
        .tick(1_001)
        .rolls(&[base + 0.5 * extra, 0.99])
        .build();
    sim.step();
    sim
}

#[test]
fn a_running_carrier_is_easier_to_tackle_than_a_standing_one() {
    assert_eq!(running(0.0).carrier(), Some(index(0, 5)), "standing: kept");
    assert_eq!(running(5.0).carrier(), Some(index(1, 5)), "running: won");
}

/// A calm match on `seed` with the cross clearance on, everyone spread away, the home
/// striker on the ball at `from`. The home side attacks `x = 52.5` in the first half.
fn pass_scene(seed: u64, from: DVec2) -> Scene {
    let mut config = calm_match(90);
    config.seed = seed;
    config.tuning.clearances.cross_chance = 0.3;
    spread(Scene::new(config), -30.0, -30.0)
        .place(index(0, 9), from)
        .ball(DVec3::new(from.x, from.y, 0.0))
        .carrier(Some(index(0, 9)))
        .tick(1_001)
}

/// An away centre-back, and the away keeper.
const DEFENDER: usize = 13;
const AWAY_KEEPER: usize = 11;

#[test]
fn a_defender_clears_a_fast_pass_in_his_area() {
    let mut sim = pass_scene(42, DVec2::new(30.0, 0.0))
        .place(DEFENDER, DVec2::new(40.0, 1.0))
        .rolls(&[0.0])
        .kick(DVec2::X, 20.0, 0.0)
        .build();
    assert_eq!(sim.summary().passes, [1, 0]);
    for _ in 0..30 {
        sim.step();
        if sim.summary().clearances[1] > 0 {
            break;
        }
    }
    let s = sim.summary();
    assert_eq!(s.clearances, [0, 1]);
    assert_eq!(sim.last_touch(), Some(1), "the defender touched it last");
    assert_eq!(s.passes_completed, [0, 0]);
}

#[test]
fn a_failed_clearance_roll_lets_the_pass_go_on() {
    let mut sim = pass_scene(42, DVec2::new(30.0, 0.0))
        .place(DEFENDER, DVec2::new(40.0, 1.0))
        .rolls(&[0.99])
        .kick(DVec2::X, 20.0, 0.0)
        .build();
    for _ in 0..60 {
        sim.step();
        if sim.record().ball[0] > 43.0 {
            break;
        }
    }
    assert_eq!(sim.summary().clearances, [0, 0]);
    assert_eq!(sim.last_touch(), Some(0));
    assert!(
        sim.record().ball[0] > 43.0,
        "the pass went on past the defender"
    );
}

#[test]
fn a_cleared_pass_along_the_goal_line_can_give_a_corner() {
    let mut corners = 0;
    for seed in 1..=40 {
        let mut sim = pass_scene(seed, DVec2::new(50.0, -15.0))
            .place(DEFENDER, DVec2::new(51.0, 10.0))
            // The keeper stands off the pass's path, so only the defender can touch it.
            .place(AWAY_KEEPER, DVec2::new(40.0, -10.0))
            .rolls(&[0.0])
            .kick(DVec2::Y, 20.0, 0.0)
            .build();
        let mut before = sim.record();
        for _ in 0..300 {
            let touch = sim.last_touch();
            sim.step();
            if let Some(dead) = sim.dead_ball() {
                if dead.kind == StoppageKind::Corner {
                    corners += 1;
                    assert_eq!(touch, Some(1), "seed {seed}: a defender touched it last");
                    assert!(
                        before.ball[0] >= (pitch::HALF_LENGTH - 0.81) as f32,
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
        assert_eq!(
            sim.summary().clearances,
            [0, 1],
            "seed {seed}: no clearance"
        );
    }
    assert!(corners >= 1, "no clearance went behind for a corner");
    eprintln!("clearances that gave a corner: {corners} of 40");
}

fn full(seed: u64) -> MatchConfig {
    let content = content();
    let [a, b] = default_teams(&content);
    MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap()
}

/// What caused the last defending touch before a corner.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    Save,
    Block,
    Clearance,
    Other,
}

/// The figures of one match the slow tests read.
struct MatchRun {
    minutes: u32,
    summary: engine::sim::Summary,
    ball_in_play_s: u32,
    /// Each restart taken: its kind and the seconds the ball was dead before it.
    dead: Vec<(StoppageKind, f64)>,
    corners: Vec<Origin>,
    /// Corners that did not follow a defending touch over the goal line.
    bad: Vec<String>,
}

fn play_full(seed: u64) -> MatchRun {
    let mut sim = Simulation::new(full(seed)).unwrap();
    let mut dead = Vec::new();
    let mut open: Option<engine::rules::DeadBall> = None;
    let mut corners = Vec::new();
    let mut bad = Vec::new();
    let mut origin = Origin::Other;
    while !sim.is_over() {
        let before = sim.record();
        let touch = sim.last_touch();
        let (census, cleared) = (sim.shot_census(), sim.summary().clearances);
        sim.step();
        let after = sim.shot_census();
        if after.parried > census.parried {
            origin = Origin::Save;
        } else if after.blocked > census.blocked {
            origin = Origin::Block;
        } else if sim.summary().clearances != cleared {
            origin = Origin::Clearance;
        }
        if sim.carrier().is_some() {
            origin = Origin::Other;
        }
        if let Some(stoppage) = sim.stoppage()
            && stoppage.kind == StoppageKind::Corner
        {
            corners.push(origin);
            let attacking = stoppage.team.expect("a corner has a team");
            let near_line = f64::from(before.ball[0].abs()) >= pitch::HALF_LENGTH - 0.81;
            if !near_line || touch != Some(1 - attacking) {
                bad.push(format!(
                    "seed {seed} tick {}: ball {:?}, last touch {touch:?}",
                    sim.tick(),
                    before.ball
                ));
            }
        }
        match (sim.dead_ball(), open) {
            (Some(d), None) => open = Some(d),
            (None, Some(d)) => {
                dead.push((d.kind, f64::from(sim.tick() - d.since) * 0.02));
                open = None;
            }
            _ => {}
        }
    }
    sim.finish();
    let summary = sim.summary();
    MatchRun {
        minutes: sim.minutes(),
        summary,
        ball_in_play_s: MatchFigures::new(&summary, sim.managers()).ball_in_play_s,
        dead,
        corners,
        bad,
    }
}

/// The 200 matches every slow test reads, played once per test binary.
fn runs() -> &'static [MatchRun] {
    static RUNS: OnceLock<Vec<MatchRun>> = OnceLock::new();
    RUNS.get_or_init(|| run_many(1..=200, play_full))
}

#[test]
#[ignore = "slow: cargo test --release -p engine --all-features -- --ignored"]
fn passes_are_realistic() {
    let runs = runs();
    let n = runs.len() as f64;
    let passes: u32 = runs
        .iter()
        .map(|r| r.summary.passes[0] + r.summary.passes[1])
        .sum();
    let completed: u32 = runs
        .iter()
        .map(|r| r.summary.passes_completed[0] + r.summary.passes_completed[1])
        .sum();
    let per_team = f64::from(passes) / (2.0 * n);
    let accuracy = 100.0 * f64::from(completed) / f64::from(passes.max(1));
    eprintln!("passes per team {per_team:.1}, accuracy {accuracy:.2}%");
    assert!(
        (350.0..=550.0).contains(&per_team),
        "passes per team {per_team:.1}"
    );
    assert!((75.0..=88.0).contains(&accuracy), "accuracy {accuracy:.2}%");
}

#[test]
#[ignore = "slow: cargo test --release -p engine --all-features -- --ignored"]
fn the_ball_is_in_play_for_about_an_hour() {
    let runs = runs();
    let per_90: f64 = runs
        .iter()
        .map(|r| f64::from(r.ball_in_play_s) / 60.0 * 90.0 / f64::from(r.minutes))
        .sum::<f64>()
        / runs.len() as f64;
    eprintln!("ball in play {per_90:.2} minutes per 90");
    assert!((52.0..=65.0).contains(&per_90), "ball in play {per_90:.2}");
}

#[test]
#[ignore = "slow: cargo test --release -p engine --all-features -- --ignored"]
fn throw_ins_stay_in_band() {
    let runs = runs();
    let throw_ins: u32 = runs
        .iter()
        .map(|r| r.summary.throw_ins[0] + r.summary.throw_ins[1])
        .sum();
    let per_match = f64::from(throw_ins) / runs.len() as f64;
    eprintln!("throw-ins per match {per_match:.2}");
    assert!(
        (35.0..=55.0).contains(&per_match),
        "throw-ins per match {per_match:.2}"
    );
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

#[test]
#[ignore = "slow: cargo test --release -p engine --all-features -- --ignored"]
fn restart_census() {
    let runs = runs();
    let n = runs.len() as f64;
    // The sourced medians of the dead time before each restart.
    for (kind, source) in [
        (StoppageKind::ThrowIn, Some(13.8)),
        (StoppageKind::GoalKick, Some(23.2)),
        (StoppageKind::Corner, Some(31.8)),
        (StoppageKind::FreeKick, Some(32.5)),
        (StoppageKind::KickOff, None),
        (StoppageKind::Penalty, None),
        (StoppageKind::Injury, None),
    ] {
        let times: Vec<f64> = runs
            .iter()
            .flat_map(|r| r.dead.iter().filter(|d| d.0 == kind).map(|d| d.1))
            .collect();
        let count = times.len() as f64 / n;
        let src = source.map_or("no source".to_string(), |s: f64| format!("source {s:.1} s"));
        eprintln!(
            "  {:<10} {count:>6.2} per match, median {:>5.1} s ({src})",
            kind.code(),
            median(times)
        );
    }
    let sum =
        |f: &dyn Fn(&engine::sim::Summary) -> u32| runs.iter().map(|r| f(&r.summary)).sum::<u32>();
    eprintln!(
        "clearances per team {:.2}, restart kicks per team {:.2}, fouls per match {:.2}",
        f64::from(sum(&|s| s.clearances[0] + s.clearances[1])) / (2.0 * n),
        f64::from(sum(&|s| s.restart_kicks[0] + s.restart_kicks[1])) / (2.0 * n),
        f64::from(sum(&|s| s.fouls[0] + s.fouls[1])) / n,
    );
    let corners: Vec<Origin> = runs
        .iter()
        .flat_map(|r| r.corners.iter().copied())
        .collect();
    for (name, origin) in [
        ("save or parry", Origin::Save),
        ("block", Origin::Block),
        ("clearance", Origin::Clearance),
        ("other", Origin::Other),
    ] {
        let k = corners.iter().filter(|&&o| o == origin).count();
        eprintln!(
            "  corners from {name:<14} {k:>6} {:.3} per team",
            k as f64 / (2.0 * n)
        );
    }
    let bad: Vec<&String> = runs.iter().flat_map(|r| &r.bad).collect();
    assert!(
        bad.is_empty(),
        "{} corners without a crossing: {bad:?}",
        bad.len()
    );
}
