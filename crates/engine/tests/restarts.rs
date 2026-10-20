//! Restarts with the shipped wait table: the ball waits where it left play for the drawn
//! extra and then appears at the spot; each stoppage lasts its visible steps plus that extra;
//! a quick restart skips the extra and never shortens the taker's steps; a stalled restart is
//! taken at the hard limit and logged; players with no job stand or walk at a dead ball; and
//! each half opens with a walk-on from the tunnel that takes no match time.

mod common;

use std::io::Write;
use std::sync::{Arc, Mutex};

use common::{quiet_match, spread};
use engine::data::restart_waits::WaitKind;
use engine::math::{DVec2, DVec3};
use engine::pace::{Job, Level};
use engine::record::{NullSink, TickRecord, TickSink};
use engine::rules::DeadBall;
use engine::scenario::Scene;
use engine::{EngineError, EngineEventKind, MatchConfig, Simulation, WalkOn};

/// A ball rolled over the near touchline after the away side touched it, every player far
/// away, with the next restart's draws pinned as `quick` and `wait_draw` say. Returns the match
/// on the tick play stopped.
fn throw_in(config: MatchConfig, quick: Option<bool>, wait_draw: Option<f64>) -> Simulation {
    let mut sim = spread(Scene::new(config), -30.0, 30.0)
        .carrier(None)
        .ball(DVec3::new(10.0, 33.8, 0.0))
        .ball_velocity(DVec3::new(0.0, 4.0, 0.0))
        .last_touch(1)
        .tick(1_001)
        .pin_restart(quick, wait_draw)
        .build();
    for _ in 0..50 {
        sim.step();
        if sim.dead_ball().is_some() {
            return sim;
        }
    }
    panic!("the ball never left the pitch");
}

/// The ball on a record.
fn ball(r: &TickRecord) -> DVec2 {
    DVec2::new(f64::from(r.ball[0]), f64::from(r.ball[1]))
}

/// The ball stays at its exit point for the drawn extra, then appears at the spot on a
/// tick that carries the restart flag; a quick restart puts it there at once.
#[test]
fn the_ball_waits_where_it_left_play_then_appears_at_the_spot() {
    let mut sim = throw_in(quiet_match(90), Some(false), Some(0.5));
    let dead = sim.dead_ball().unwrap();
    assert!(!dead.quick);
    assert!(
        dead.extra > 0,
        "the median extra of a throw-in is above zero"
    );
    assert_eq!(sim.tick(), dead.since);
    assert_eq!(dead.appear_at, dead.since + dead.extra);
    while sim.tick() < dead.appear_at {
        let r = sim.record();
        assert!(
            !r.restart,
            "tick {} is flagged before the ball appears",
            r.tick
        );
        assert!(
            (ball(&r) - dead.exit).length() <= 1e-3,
            "tick {}: the ball at {} left its exit point {}",
            r.tick,
            ball(&r),
            dead.exit
        );
        sim.step();
    }
    let r = sim.record();
    assert!(r.restart, "the tick the ball appears is flagged");
    assert!((ball(&r) - dead.spot).length() <= 1e-3);

    let sim = throw_in(quiet_match(90), Some(true), None);
    let dead = sim.dead_ball().unwrap();
    assert!(dead.quick);
    assert_eq!(dead.extra, 0);
    assert_eq!(dead.appear_at, dead.since);
    let r = sim.record();
    assert!(
        r.restart,
        "a quick restart is at the spot on the tick play stopped"
    );
    assert!((ball(&r) - dead.spot).length() <= 1e-3);
}

/// The tick the visible steps were first done, the tick the restart was taken, and the drawn
/// extra of the throw-in.
fn steps_and_take(quick: bool) -> (u32, u32, u32) {
    let mut sim = throw_in(quiet_match(90), Some(quick), Some(0.5));
    let extra = sim.dead_ball().unwrap().extra;
    let mut steps = None;
    for _ in 0..20_000 {
        sim.step();
        match sim.dead_ball() {
            Some(d) if d.ready_at != DeadBall::UNSET && steps.is_none() => {
                steps = Some(d.ready_at - d.extra);
            }
            Some(_) => {}
            None => return (steps.unwrap_or(sim.tick()), sim.tick(), extra),
        }
    }
    panic!("the throw-in was never taken");
}

/// A quick restart leaves the taker's visible steps as they are and only drops
/// the drawn extra.
#[test]
fn a_quick_restart_never_shortens_the_takers_steps() {
    let (slow_steps, slow_taken, extra) = steps_and_take(false);
    let (quick_steps, quick_taken, none) = steps_and_take(true);
    assert_eq!(none, 0);
    assert!(extra > 0);
    assert_eq!(
        quick_steps, slow_steps,
        "the taker reaches the ball on the same tick"
    );
    assert_eq!(quick_taken, quick_steps);
    assert_eq!(slow_taken, slow_steps + extra);
    assert_eq!(
        slow_taken - quick_taken,
        extra,
        "taken the drawn extra earlier"
    );
}

/// The lowest and the highest extra, in ticks, the table allows for `kind` with every factor
/// that may apply: the late lead and the highest time-wasting level of the tactics file.
fn extra_bounds(config: &MatchConfig, kind: engine::StoppageKind) -> (u32, u32) {
    let waits = config.restart_waits.as_ref().expect("the shipped table");
    let q = &waits.kinds.get(WaitKind::of(kind)).extra_s;
    let wasting = config
        .tactics
        .instructions
        .time_wasting
        .levels
        .iter()
        .map(|l| l.value)
        .fold(1.0, f64::max);
    let dt = config.tuning.dt;
    let lo = (q[0] / dt).floor() as u32;
    let hi = (q[q.len() - 1] * waits.late_lead_factor * wasting / dt).ceil() as u32;
    (lo.saturating_sub(1), hi + 1)
}

/// Over four seeded 90-minute matches, every stoppage not forced by its hard limit is
/// taken exactly its drawn extra after its visible steps are done, so it lasts its visible
/// steps plus the extra, unless a player stands out of place again once the extra has run, when
/// it waits for him; each extra lies inside what the table allows.
#[test]
fn every_stoppage_lasts_its_visible_steps_plus_its_drawn_extra() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let checked = common::run_many(1..=4, |seed| {
        let config = MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
        let mut sim = Simulation::new(config.clone()).unwrap();
        let mut open: Option<DeadBall> = None;
        // A tick after the drawn extra began on which a player stood out of place again.
        let mut undone = false;
        let mut checked = 0u32;
        while !sim.is_over() {
            sim.step();
            let events = sim.take_events();
            let breaks = events.iter().any(|e| {
                matches!(
                    e.kind,
                    EngineEventKind::HalfTime | EngineEventKind::FullTime
                )
            });
            if sim.in_shootout() {
                open = None;
                continue;
            }
            match (open, sim.dead_ball()) {
                (_, Some(d)) => {
                    // The step to tick `n` judges the steps on the positions of tick `n - 1`,
                    // so the positions read here count for the restart from the next tick.
                    if d.ready_at != DeadBall::UNSET
                        && sim.tick() + 1 >= d.ready_at
                        && !engine::rules::restart::steps_done(
                            &d,
                            sim.players(),
                            &sim.teams(),
                            &config.tuning,
                        )
                    {
                        undone = true;
                    }
                    open = Some(d);
                }
                (Some(d), None) => {
                    open = None;
                    let taken = sim.tick();
                    let undone = std::mem::take(&mut undone);
                    // A stoppage the half or the match ends was never taken: the full-time
                    // event of a match that ends at a dead ball comes when it finishes.
                    if taken >= d.hard_limit() || breaks || sim.is_over() {
                        continue;
                    }
                    let steps = if d.ready_at == DeadBall::UNSET {
                        // The steps were done on the tick the restart was taken.
                        assert_eq!(d.extra, 0, "seed {seed} tick {taken}: {d:?}");
                        taken
                    } else {
                        d.ready_at - d.extra
                    };
                    assert!(steps >= d.since, "seed {seed}: {d:?}");
                    // Taken the drawn extra after the steps were done; later only when a player
                    // stood out of place again once the extra had run, as the Laws wait for him.
                    let due = (steps - d.since) + d.extra;
                    assert!(
                        taken - d.since == due || (taken - d.since > due && undone),
                        "seed {seed}: taken after {} ticks, due after {due}: {d:?}",
                        taken - d.since
                    );
                    if !d.quick {
                        let (lo, hi) = extra_bounds(&config, d.kind);
                        assert!(
                            (lo..=hi).contains(&d.extra),
                            "seed {seed}: extra {} outside {lo} to {hi}: {d:?}",
                            d.extra
                        );
                    }
                    checked += 1;
                }
                (None, None) => undone = false,
            }
        }
        checked
    });
    for (seed, n) in (1..=4).zip(&checked) {
        assert!(*n > 15, "seed {seed} checked only {n} restarts");
    }
}

/// On every dead tick, every player whose job is to wait stands or walks.
#[test]
fn players_with_no_job_stand_or_walk_at_a_dead_ball() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let walk = content.tuning.engine.pace.limits_ms[1];
    let counted = common::run_many(1..=4, |seed| {
        let config = MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
        let mut sim = Simulation::new(config).unwrap();
        let mut waiting = 0u64;
        while !sim.is_over() {
            sim.step();
            // A substitute who came on this tick has no job yet: the dead-ball tick set the
            // jobs before the change put him on, in the slot of the player who left.
            let entered: Vec<usize> = sim
                .take_events()
                .iter()
                .filter(|e| e.kind == EngineEventKind::Substitution)
                .filter_map(|e| e.player)
                .collect();
            if sim.dead_ball().is_none() || sim.in_shootout() {
                continue;
            }
            for (i, p) in sim.players().iter().enumerate().filter(|(_, p)| p.active()) {
                if p.pace.job != Job::Wait {
                    continue;
                }
                if p.pace.speed == f64::MAX {
                    assert!(
                        entered.contains(&i),
                        "seed {seed} tick {} slot {}: no pace and no substitution",
                        sim.tick(),
                        p.slot
                    );
                    continue;
                }
                waiting += 1;
                assert!(
                    matches!(p.pace.level, Level::Stand | Level::Walk),
                    "seed {seed} tick {} slot {}: {:?}",
                    sim.tick(),
                    p.slot,
                    p.pace
                );
                assert!(
                    p.vel.length() <= walk + 1e-9,
                    "seed {seed} tick {} slot {}: {} m/s",
                    sim.tick(),
                    p.slot,
                    p.vel.length()
                );
            }
        }
        waiting
    });
    assert!(counted.iter().all(|n| *n > 10_000), "{counted:?}");
}

/// A writer the tracing subscriber shares with the test.
#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<u8>>>);

impl Write for Log {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A restart whose visible steps never finish (no taker can ever stand close
/// enough to the ball) is taken at its hard limit, `since` plus the extra plus the kind's
/// steps limit, and logged.
#[test]
fn a_stalled_restart_is_taken_at_the_hard_limit_and_logged() {
    let mut config = quiet_match(90);
    config.tuning.restart_ready_radius = -1.0;
    let mut sim = throw_in(config.clone(), Some(false), Some(0.5));
    let dead = sim.dead_ball().unwrap();
    let limit = config
        .restart_waits
        .as_ref()
        .unwrap()
        .kinds
        .get(WaitKind::ThrowIn)
        .steps_limit_s;
    let limit = (limit / config.tuning.dt).round() as u32;
    assert_eq!(dead.hard_limit(), dead.since + dead.extra + limit);
    let log = Log::default();
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    let taken = tracing::subscriber::with_default(subscriber, || {
        while sim.dead_ball().is_some() {
            assert_eq!(sim.dead_ball().unwrap().ready_at, DeadBall::UNSET);
            sim.step();
        }
        sim.tick()
    });
    assert_eq!(taken, dead.hard_limit());
    let text = String::from_utf8_lossy(&log.0.lock().unwrap()).into_owned();
    assert!(text.contains("rules.dead_ball_stalled"), "{text}");
    assert!(text.contains("steps_done=false"), "{text}");
}

#[derive(Debug)]
enum Call {
    Walk(WalkOn),
    Tick(TickRecord),
}

/// Records the order of the sink's calls.
#[derive(Default)]
struct Order(Vec<Call>);

impl TickSink for Order {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.0.push(Call::Tick(*record));
        Ok(())
    }

    fn on_walk_on(&mut self, walk_on: &WalkOn) -> Result<(), EngineError> {
        self.0.push(Call::Walk(walk_on.clone()));
        Ok(())
    }
}

/// A level knockout match gets one walk-on before the first record of
/// each of its four halves. Each starts at the tunnel on the near touchline, ends on the
/// positions of that record, and never moves a player faster than the walk; the records stay
/// consecutive across it, and the match plays exactly as with no one listening, so the
/// walk-on takes no match time and no ball-in-play time. Seed 1 is the lowest knockout seed
/// level after 90 minutes with all 22 players on through extra time (seed 11 until
/// clearances, shots and misreads were scored by their cost; seed 2 until the styles
/// became presets of primitives; seed 5 until a player
/// going for a loose ball stopped being pushed off it; seed 1 until the running
/// pace lifted on a counter rather than with the press; seed 9 until opponents walked round
/// the restart spot; seed 0 until the in-possession settings came in; seed 8 until tempo
/// became patience and the mentality an appetite for risk; seed 0 until a player stepped aside
/// from a team-mate walking at him; seed 2 until a passer read the offside line; seed 0 until a passer read the press his receiver would meet;
/// seed 2 until a kick stayed on its way from its kicker along the line he sent it; seed 0 until
/// the AI manager stopped making tactics changes of its own; seed 1 until its substitutions
/// followed the situation rather than the clock).
#[test]
fn each_half_opens_with_a_walk_on_that_takes_no_match_time() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let config = MatchConfig::new(1, 90, &content, [&a, &b])
        .unwrap()
        .with_knockout();
    let dt = config.tuning.dt;
    let half_width = config.pitch().half_width();
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut order = Order::default();
    sim.run(&mut order).unwrap();
    let mut plain = Simulation::new(config).unwrap();
    plain.run(&mut NullSink).unwrap();
    assert_eq!(plain.tick(), sim.tick());
    assert_eq!(
        format!("{:?}", plain.summary()),
        format!("{:?}", sim.summary())
    );

    let mut periods = Vec::new();
    let mut ticks = 0u32;
    for (i, call) in order.0.iter().enumerate() {
        match call {
            Call::Tick(r) => {
                ticks += 1;
                assert_eq!(r.tick, ticks, "the records are consecutive");
            }
            Call::Walk(w) => {
                periods.push(w.period);
                let Some(Call::Tick(first)) = order.0.get(i + 1) else {
                    panic!(
                        "the walk-on of period {} is not followed by a record",
                        w.period
                    );
                };
                assert_eq!(first.tick, w.first_tick);
                assert_eq!(w.paths.len(), 22);
                let start = w.at(0, dt);
                let end = w.at(w.ticks, dt);
                for p in &w.paths {
                    assert_eq!(start[p.player], Some(p.from));
                    assert_eq!(p.from.y, -half_width, "out of the tunnel");
                    let to = end[p.player].unwrap();
                    let at = first.players[p.player];
                    assert!(
                        (to - DVec2::new(f64::from(at[0]), f64::from(at[1]))).length() <= 1e-3,
                        "period {} player {}",
                        w.period,
                        p.player
                    );
                }
                for frame in 1..=w.ticks {
                    let (x, y) = (w.at(frame - 1, dt), w.at(frame, dt));
                    for p in &w.paths {
                        let step = (y[p.player].unwrap() - x[p.player].unwrap()).length();
                        assert!(step <= w.speed * dt + 1e-9, "frame {frame}");
                    }
                }
            }
        }
    }
    assert_eq!(periods, [0, 1, 2, 3], "one walk-on before each half");
    assert!(matches!(order.0.first(), Some(Call::Walk(w)) if w.first_tick == 1));
}
