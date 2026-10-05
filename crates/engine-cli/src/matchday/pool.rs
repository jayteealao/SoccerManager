//! The worker pool that plays the other matches of the matchday on the full engine.
//!
//! Each fixture is pinned to one worker, and a worker steps its fixtures round robin one
//! simulated second at a time, so every ground advances together and none waits behind
//! another. Each worker runs below normal priority. A worker never blocks the player's match:
//! it only sends what it computed on a channel and publishes the tick each fixture reached.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::Instant;

use engine::{EngineEvent, EngineEventKind, MatchConfig, Plugins, Simulation};
use protocol::{GroundEvent, GroundKind, Side};

use super::record::{BugReport, Failure, panic_message};
use super::{Fault, cores};

/// One background match, as the main thread hands it to the pool.
pub struct Job {
    pub fixture: u32,
    pub seed: u64,
    pub club_ids: [String; 2],
    /// The match, or why it cannot be built.
    pub config: Result<MatchConfig, String>,
    /// The script pack's hooks, when the player's match plays with one.
    pub plugins: Option<Plugins>,
}

/// What a worker reports.
pub enum Note {
    /// An event at a ground, and the moment it was computed.
    Event(GroundEvent, Instant),
    /// A fixture ended: at full time, or with no result.
    Ended(Ended),
}

/// How a fixture ended.
#[derive(Debug, Clone)]
pub struct Ended {
    pub fixture: u32,
    pub seed: u64,
    /// `None` at full time; the failure otherwise.
    pub failure: Option<Failure>,
    pub ticks: u32,
    pub events: u32,
    /// Wall time from the pool's start to the fixture's end.
    pub duration_ms: u64,
}

/// What the workers and the main thread share.
pub struct Shared {
    /// The tick each fixture has reached.
    pub reached: Vec<AtomicU32>,
    /// `true` once a fixture has ended or failed.
    pub ended: Vec<AtomicBool>,
    /// Set when the matchday stops: every worker ends at its next simulated second.
    pub stop: AtomicBool,
    pub data_dir: PathBuf,
    pub match_id: String,
    pub fault: Option<Fault>,
}

impl Shared {
    pub fn new(fixtures: usize, data_dir: PathBuf, match_id: String, fault: Option<Fault>) -> Self {
        Self {
            reached: (0..fixtures).map(|_| AtomicU32::new(0)).collect(),
            ended: (0..fixtures).map(|_| AtomicBool::new(false)).collect(),
            stop: AtomicBool::new(false),
            data_dir,
            match_id,
            fault,
        }
    }
}

/// Starts `threads` workers over `jobs`: worker `w` plays the fixtures whose index modulo
/// `threads` is `w`.
pub fn start(
    jobs: Vec<Job>,
    threads: usize,
    shared: &Arc<Shared>,
    notes: &Sender<Note>,
    started: Instant,
) -> Vec<JoinHandle<()>> {
    let threads = threads.max(1);
    let mut lanes: Vec<Vec<Job>> = (0..threads).map(|_| Vec::new()).collect();
    for job in jobs {
        let lane = job.fixture as usize % threads;
        lanes[lane].push(job);
    }
    lanes
        .into_iter()
        .enumerate()
        .filter(|(_, lane)| !lane.is_empty())
        .map(|(w, lane)| {
            let shared = Arc::clone(shared);
            let notes = notes.clone();
            std::thread::Builder::new()
                .name(format!("matchday-{w}"))
                .spawn(move || worker(lane, &shared, &notes, started))
                .expect("a worker thread starts")
        })
        .collect()
}

/// One fixture as a worker plays it.
struct Ground {
    job: Job,
    sim: Option<Simulation>,
    /// The period whose kick-off was last reported: 0 for the first half.
    period: u32,
    events: u32,
    over: bool,
}

fn worker(lane: Vec<Job>, shared: &Shared, notes: &Sender<Note>, started: Instant) {
    cores::lower_priority();
    let mut grounds: Vec<Ground> = lane
        .into_iter()
        .map(|job| Ground {
            job,
            sim: None,
            period: 0,
            events: 0,
            over: false,
        })
        .collect();
    for ground in &mut grounds {
        let built = match &ground.job.config {
            Ok(config) => catch_unwind(AssertUnwindSafe(|| Simulation::new(config.clone())))
                .map_err(|p| panic_message(&*p))
                .and_then(|r| r.map_err(|e| e.to_string())),
            Err(why) => Err(why.clone()),
        };
        match built {
            Ok(mut sim) => {
                if let Some(plugins) = ground.job.plugins.take() {
                    sim.set_plugins(plugins);
                }
                ground.sim = Some(sim);
            }
            Err(why) => fail(ground, Failure::TeamFile, &why, shared, notes, started),
        }
    }
    while grounds.iter().any(|g| !g.over) {
        if shared.stop.load(Ordering::Relaxed) {
            return;
        }
        for ground in grounds.iter_mut().filter(|g| !g.over) {
            let played = catch_unwind(AssertUnwindSafe(|| one_second(ground, shared.fault, notes)));
            let index = ground.job.fixture as usize;
            let tick = ground.sim.as_ref().map_or(0, Simulation::tick);
            shared.reached[index].store(tick, Ordering::Release);
            match played {
                Ok(true) => {
                    ground.over = true;
                    shared.ended[index].store(true, Ordering::Release);
                    let _ = notes.send(Note::Ended(Ended {
                        fixture: ground.job.fixture,
                        seed: ground.job.seed,
                        failure: None,
                        ticks: tick,
                        events: ground.events,
                        duration_ms: millis(started),
                    }));
                }
                Ok(false) => {}
                Err(payload) => {
                    let why = panic_message(&*payload);
                    fail(ground, Failure::Panic, &why, shared, notes, started);
                }
            }
        }
    }
}

/// Plays one simulated second of `ground`, or to its end. `true` once the match is over.
fn one_second(ground: &mut Ground, fault: Option<Fault>, notes: &Sender<Note>) -> bool {
    let (cap, until) = {
        let sim = ground.sim.as_ref().expect("a playing ground has a match");
        let second = ((1.0 / sim.tuning().dt).round() as u32).max(1);
        (sim.config().max_ticks(), sim.tick() + second)
    };
    loop {
        let fixture = ground.job.fixture;
        let sim = ground.sim.as_mut().expect("a playing ground has a match");
        if sim.is_over() || (sim.tick() >= cap && !sim.in_shootout()) || sim.tick() >= until {
            break;
        }
        if let Some(f) = fault
            && f.fixture == fixture
            && sim.tick() == f.tick
        {
            panic!("induced fault in fixture {} at tick {}", f.fixture, f.tick);
        }
        sim.step();
        let events = sim.take_events();
        report(ground, &events, notes);
    }
    let sim = ground.sim.as_mut().expect("a playing ground has a match");
    if sim.is_over() || (sim.tick() >= cap && !sim.in_shootout()) {
        sim.finish();
        let events = sim.take_events();
        report(ground, &events, notes);
        return true;
    }
    false
}

/// Sends the events of one step that a ground row shows: goals, period changes, full time.
fn report(ground: &mut Ground, events: &[EngineEvent], notes: &Sender<Note>) {
    let sim = ground.sim.as_ref().expect("a playing ground has a match");
    for event in events {
        let kind = match event.kind {
            EngineEventKind::Goal => GroundKind::Goal,
            EngineEventKind::HalfTime => GroundKind::HalfTime,
            EngineEventKind::FullTime => GroundKind::FullTime,
            // The kick-off that starts a new period; a kick-off after a goal is not news.
            EngineEventKind::KickOff if sim.half() > ground.period => {
                ground.period = sim.half();
                if sim.half() == 1 {
                    GroundKind::SecondHalf
                } else {
                    GroundKind::ExtraTime
                }
            }
            _ => continue,
        };
        let goal = kind == GroundKind::Goal;
        let message = GroundEvent {
            fixture: ground.job.fixture,
            tick: event.tick,
            kind,
            minute: event.minute,
            added: event.minute_added,
            side: event
                .team
                .filter(|_| goal)
                .map(|t| if t == 0 { Side::Home } else { Side::Away }),
            scorer: event
                .player
                .filter(|_| goal)
                .and_then(|p| sim.player_names().get(p).cloned()),
            score: event.scores,
            late: false,
        };
        ground.events += 1;
        let _ = notes.send(Note::Event(message, Instant::now()));
    }
}

/// Ends a ground with no result: the row shows "result unavailable" at the tick it reached,
/// a bug report keeps its seed and engine, and the worker goes on with its other grounds.
fn fail(
    ground: &mut Ground,
    failure: Failure,
    why: &str,
    shared: &Shared,
    notes: &Sender<Note>,
    started: Instant,
) {
    let tick = ground.sim.as_ref().map_or(0, Simulation::tick);
    let (minute, added) = ground.sim.as_ref().map_or((0, None), Simulation::minute);
    let score = ground.sim.as_ref().map_or([0, 0], |s| s.summary().goals);
    ground.over = true;
    ground.sim = None;
    let fixture = ground.job.fixture;
    failed(
        &FailedGround {
            fixture,
            seed: ground.job.seed,
            club_ids: &ground.job.club_ids,
            tick,
            minute,
            added,
            score,
            events: ground.events,
        },
        failure,
        why,
        shared,
        notes,
        started,
    );
}

/// A ground that stopped, as its failure reports it.
pub struct FailedGround<'a> {
    pub fixture: u32,
    pub seed: u64,
    pub club_ids: &'a [String; 2],
    pub tick: u32,
    pub minute: u32,
    pub added: Option<u32>,
    pub score: [u32; 2],
    pub events: u32,
}

/// Writes the bug report, emits the failure signal, and sends the unavailable event and the
/// end of the fixture.
pub fn failed(
    ground: &FailedGround<'_>,
    failure: Failure,
    why: &str,
    shared: &Shared,
    notes: &Sender<Note>,
    started: Instant,
) {
    let index = ground.fixture as usize;
    shared.reached[index].store(ground.tick, Ordering::Release);
    let report = BugReport {
        match_id: &shared.match_id,
        fixture: ground.fixture,
        seed: ground.seed,
        club_ids: ground.club_ids,
        tick: ground.tick,
        failure,
        message: why,
    };
    let written = report.write(&shared.data_dir);
    tracing::error!(
        signal = "matchday.match_failed",
        match.id = %shared.match_id,
        error.type = failure.code(),
        fixture = ground.fixture,
        seed = ground.seed,
        version = engine::version(),
        build.hash = engine::build_hash(),
        tick = ground.tick,
        report = %written.as_deref().unwrap_or("not written"),
    );
    if let Err(err) = &written {
        tracing::error!(
            signal = "matchday.bug_report_failed",
            fixture = ground.fixture,
            reason = %err
        );
    }
    let _ = notes.send(Note::Event(
        GroundEvent {
            fixture: ground.fixture,
            tick: ground.tick,
            kind: GroundKind::Unavailable,
            minute: ground.minute,
            added: ground.added,
            side: None,
            scorer: None,
            score: ground.score,
            late: false,
        },
        Instant::now(),
    ));
    shared.ended[index].store(true, Ordering::Release);
    let _ = notes.send(Note::Ended(Ended {
        fixture: ground.fixture,
        seed: ground.seed,
        failure: Some(failure),
        ticks: ground.tick,
        events: ground.events,
        duration_ms: millis(started),
    }));
}

fn millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
