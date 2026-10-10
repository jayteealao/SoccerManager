//! Mood in play: a goal lifts the scorer and a big miss lowers the misser, each by his swing
//! (temperament and pressure handling); a full mood stays inside the mind cap; the team mood
//! weighs each player by his leadership; a substitute enters at his side's bench mood; and a
//! match resumed from a snapshot moves moods as the unbroken match does.

mod common;

use common::{calm_match, index, spread};
use engine::data::rules::StoppageKind;
use engine::math::{DVec2, DVec3};
use engine::scenario::Scene;
use engine::sim::mood::FULL;
use engine::{MatchConfig, Simulation};

/// The mental group's index in a player's deltas.
const MENTAL: usize = 1;

/// Both sides far from the away goal, the away keeper off his line, and home player `i`
/// rated `v` in temperament and pressure handling, with the ball at `at`.
fn shooter(config: MatchConfig, i: usize, v: f64, at: DVec2) -> Scene {
    spread(Scene::new(config), -45.0, -30.0)
        .place(index(1, 0), DVec2::new(30.0, 30.0))
        .rating(i, "temperament", v)
        .rating(i, "pressure_handling", v)
        .place(i, at)
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(i))
}

/// Steps `sim` until `done` holds, at most `n` ticks.
fn step_until(sim: &mut Simulation, n: u32, done: impl Fn(&Simulation) -> bool) {
    for _ in 0..n {
        if done(sim) {
            return;
        }
        sim.step();
    }
    panic!("the scene did not reach its end in {n} ticks");
}

/// Home striker rated `v` in temperament and pressure handling shoots into the empty net from
/// 12.5 m: his mood after the goal.
fn scorer_mood(v: f64) -> f64 {
    let a = index(0, 9);
    let mut sim = shooter(calm_match(90), a, v, DVec2::new(40.0, 0.0))
        .shoot(DVec2::new(12.5, 1.0), 25.0, 0.0)
        .build();
    assert_eq!(sim.mood(a), 0.0);
    step_until(&mut sim, 200, |s| s.summary().goals[0] == 1);
    sim.mood(a)
}

/// Home forward rated `v` in temperament and pressure handling shoots wide from 6.5 m, a
/// chance of at least the big-chance expected goals: his mood once the ball is dead.
fn misser_mood(v: f64) -> f64 {
    let b = index(0, 10);
    let mut sim = shooter(calm_match(90), b, v, DVec2::new(46.0, 0.0))
        .shoot(DVec2::new(6.5, 12.0), 25.0, 0.0)
        .build();
    let big = sim.tuning().contract.person().mood.big_chance_xg;
    assert!(
        sim.summary().xg[0] >= big,
        "{} < {big}",
        sim.summary().xg[0]
    );
    step_until(&mut sim, 200, |s| s.dead_ball().is_some());
    assert_eq!(sim.summary().goals, [0, 0]);
    assert!(matches!(
        sim.dead_ball().map(|d| d.kind),
        Some(StoppageKind::GoalKick)
    ));
    sim.mood(b)
}

/// The scorer's mood rises and the misser's falls, each by more for a volatile
/// player (temperament and pressure handling 5) than for a calm one (18).
#[test]
fn a_goal_lifts_the_scorer_and_a_big_miss_lowers_the_misser_by_their_swing() {
    let (up_volatile, up_calm) = (scorer_mood(5.0), scorer_mood(18.0));
    let (down_volatile, down_calm) = (misser_mood(5.0), misser_mood(18.0));
    println!("scorer: 5 {up_volatile:+.2}, 18 {up_calm:+.2}");
    println!("misser: 5 {down_volatile:+.2}, 18 {down_calm:+.2}");
    assert!(up_calm > 0.0 && up_volatile > up_calm);
    assert!(down_calm < 0.0 && down_volatile < down_calm);
}

/// Every home player at its natural position, rated `leadership` in leadership, at `mood`;
/// the mental delta of each after the refresh.
fn mental_deltas(mood: f64, leadership: f64) -> Vec<i8> {
    let mut scene = Scene::new(calm_match(90));
    for slot in 0..11 {
        let i = index(0, slot);
        scene = scene
            .familiar(i)
            .rating(i, "leadership", leadership)
            .mood(i, mood);
    }
    let sim = scene.build();
    (0..11)
        .map(|slot| sim.players()[index(0, slot)].deltas[MENTAL])
        .collect()
}

/// With every player of a side at a full mood either way, own mood and team mood
/// together pass the mind cap, and every mental delta stops at the cap's edge.
#[test]
fn a_full_mood_stays_inside_the_mind_cap() {
    let config = calm_match(90);
    let [lo, hi] = config.tuning.contract.states.caps.mind;
    let m = config.tuning.contract.person().mood;
    assert!(
        m.own_weight + m.team_weight > hi,
        "the weights reach past the cap"
    );
    for (mood, edge) in [(FULL, hi), (-FULL, lo)] {
        let edge = (edge * 10.0).round() as i8;
        for d in mental_deltas(mood, 20.0) {
            assert_eq!(d, edge, "mood {mood}");
        }
    }
}

/// The team mood is the leadership-weighted mean: one player at a full mood pulls it further
/// as a leader (18) than as a follower (5).
#[test]
fn the_team_mood_weighs_each_player_by_his_leadership() {
    let me = index(0, 5);
    let team_mood = |leadership: f64| {
        let sim = Scene::new(calm_match(90))
            .rating(me, "leadership", leadership)
            .mood(me, FULL)
            .build();
        sim.team_mood(0)
    };
    let (follower, leader) = (team_mood(5.0), team_mood(18.0));
    assert!(follower > 0.0 && leader > follower, "{follower} {leader}");
}

/// A substitute's mood starts from his kick-off value (0) plus his side's
/// events while he was on the bench. A substitution waits for the next stoppage; the home
/// side scores, and the substitute comes on at the goal's stoppage.
#[test]
fn a_substitute_enters_at_his_sides_bench_mood() {
    let a = index(0, 9);
    let off = index(0, 7);
    let config = calm_match(90);
    let on = config.teams[0].bench[0];
    let squad_off = config.players[off].squad;
    let mut sim = shooter(config, a, 10.0, DVec2::new(40.0, 0.0))
        .queue(
            0,
            engine::tactics::change::Change::Substitution { off: squad_off, on },
        )
        .shoot(DVec2::new(12.5, 1.0), 25.0, 0.0)
        .build();
    step_until(&mut sim, 3_000, |s| s.players()[off].squad == on);
    let bench = sim.bench_mood(0);
    assert!(bench > 0.0, "the bench took the goal: {bench}");
    let entered = sim.mood(off);
    assert!(entered > 0.0, "the substitute enters lifted: {entered}");
}

/// Captures a snapshot at the first stoppage from minute 30.
#[derive(Default)]
struct At30 {
    snapshot: Option<engine::snapshot::Snapshot>,
}

impl engine::record::TickSink for At30 {
    fn on_tick(&mut self, _: &engine::record::TickRecord) -> Result<(), engine::EngineError> {
        Ok(())
    }

    fn on_stoppage(
        &mut self,
        _: &engine::Stoppage,
        sim: &Simulation,
    ) -> Result<(), engine::EngineError> {
        let minute = engine::rules::clock::TICKS_PER_MINUTE;
        if self.snapshot.is_none() && sim.tick() >= 30 * minute {
            self.snapshot = Some(engine::snapshot::Snapshot::capture(sim, [7; 16], 0));
        }
        Ok(())
    }
}

/// The snapshot keeps every mood: a match captured at a stoppage after minute 30 and resumed
/// ends with every mood and bench mood as the unbroken match does, bit for bit.
#[test]
fn a_resumed_match_moves_moods_as_the_unbroken_one() {
    use engine::snapshot::Snapshot;
    let mut whole = Simulation::new(common::scoring_match()).unwrap();
    let mut capture = At30::default();
    whole.run(&mut capture).unwrap();
    let snapshot = capture.snapshot.expect("a stoppage after minute 30");
    let read = Snapshot::from_bytes(&snapshot.to_bytes(), "s.smsn").unwrap();
    let mut resumed = Simulation::from_snapshot(common::scoring_match(), &read).unwrap();
    resumed.run(&mut engine::record::NullSink).unwrap();
    assert!(
        (0..22).any(|i| whole.mood(i) != 0.0),
        "the match moved some mood"
    );
    for i in 0..22 {
        assert_eq!(whole.mood(i).to_bits(), resumed.mood(i).to_bits(), "{i}");
    }
    for team in 0..2 {
        assert_eq!(
            whole.bench_mood(team).to_bits(),
            resumed.bench_mood(team).to_bits()
        );
    }
}
