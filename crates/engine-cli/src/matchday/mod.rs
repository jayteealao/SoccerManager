//! The background matchday: the other matches of the player's matchday, played on the full
//! engine beside the player's match and revealed on the player's match clock.
//!
//! Every ground kicks off with the player's match, so an event's tick in its own match is
//! also its moment on the player's clock. The reveal has two layers. Here, the engine sends a
//! ground event only once its own tick of the player's match has reached the event's tick
//! (the reveal cursor): a pause, a speed change and a skip all move that tick. The page then
//! shows the event only once the tick it draws reaches it, so a seek moves only the page's
//! layer. A new connection receives every event up to the cursor again.
//!
//! The pool never holds the player's match: the driver only drains what is ready. An event
//! computed after the player's match passed its moment is sent marked late; the fast model
//! is never used for a background match.

pub mod cores;
mod pool;
pub mod record;
pub mod round;
pub mod timing;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use engine::MatchConfig;
use protocol::{GroundEvent, GroundProgress, ServerMessage, TeamRef};

pub use round::Round;

use pool::{FailedGround, Job, Note, Shared};
use record::Failure;

/// The longest the engine waits after the player's full time for the other grounds to end.
pub const FINISH_WAIT: Duration = Duration::from_secs(120);

/// The matchday's number while the generated round is the only matchday.
pub const ROUND_NUMBER: u32 = 1;

/// A test seam: the fixture that panics, and the tick it panics at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fault {
    pub fixture: u32,
    pub tick: u32,
}

impl std::str::FromStr for Fault {
    type Err = String;

    /// Reads `FIXTURE@TICK`, for example `1@102000`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (fixture, tick) = text
            .split_once('@')
            .ok_or_else(|| format!("expected FIXTURE@TICK, got {text}"))?;
        Ok(Self {
            fixture: fixture
                .parse()
                .map_err(|_| format!("the fixture in {text} is not a number"))?,
            tick: tick
                .parse()
                .map_err(|_| format!("the tick in {text} is not a number"))?,
        })
    }
}

/// How a matchday plays beside one served match.
pub struct Options {
    /// The data folder, where a failed match's bug report goes.
    pub data_dir: PathBuf,
    pub match_id: String,
    /// The worker count; `None` uses usable cores minus one, at most one per fixture.
    pub threads: Option<usize>,
    pub fault: Option<Fault>,
    /// No event at or before this tick is late: the tick a resumed match continues from, or
    /// the tick a test fast-forward runs the player's match to.
    pub not_late_through: u32,
    /// A test seam: the tick a fast-forward runs the player's match to. There the player's
    /// match waits for every ground to reach it (see `Matchday::due`).
    pub fast_forward_to: Option<u32>,
}

/// The matchday message for `round`: every fixture with its two clubs.
pub fn message(round: &Round) -> ServerMessage {
    ServerMessage::Matchday(protocol::Matchday {
        round: ROUND_NUMBER,
        fixtures: round
            .fixtures
            .iter()
            .map(|f| protocol::MatchdayFixture {
                fixture: f.index,
                home: team_ref(f, 0),
                away: team_ref(f, 1),
            })
            .collect(),
    })
}

/// One club of a fixture as the matchday message names it. A club whose file is missing
/// keeps its id as its name.
fn team_ref(fixture: &round::Fixture, side: usize) -> TeamRef {
    let (name, primary, secondary) = match &fixture.clubs[side] {
        Some(club) => (
            club.file.club.name.clone(),
            club.file.club.kit.primary.clone(),
            club.file.club.kit.secondary.clone(),
        ),
        None => (
            fixture.club_ids[side].clone(),
            "#808080".into(),
            "#808080".into(),
        ),
    };
    TeamRef {
        id: fixture.club_ids[side].clone(),
        name,
        kit_primary: primary,
        kit_secondary: secondary,
        roster: Vec::new(),
        squad: Vec::new(),
        setup: None,
        formation: String::new(),
    }
}

/// One computed ground event.
struct Stored {
    event: GroundEvent,
    /// When the worker computed it.
    computed_at: Instant,
    /// `true` once this connection has been sent it.
    sent: bool,
}

/// What the main thread keeps as the pool reports.
struct Inner {
    notes: Receiver<Note>,
    events: Vec<Stored>,
    ended: Vec<Option<pool::Ended>>,
    late_events: u32,
    max_late_ticks: u32,
    not_late_through: u32,
}

/// The other matches of one served match's matchday, playing.
pub struct Matchday {
    inner: RefCell<Inner>,
    shared: Arc<Shared>,
    workers: RefCell<Vec<JoinHandle<()>>>,
    round: Round,
    /// Ticks in one simulated second.
    second: u32,
    started: Instant,
    threads: usize,
    cores: usize,
    /// A test seam: the tick the player's match runs flat out to, set at launch and moved on
    /// by a jump.
    fast_forward_to: Cell<Option<u32>>,
}

impl Matchday {
    /// Starts every fixture of `round` at kick-off on the full engine, each as the player's
    /// match is configured: its minutes and knockout setting, the same content and script
    /// pack, both teams managed by the computer.
    pub fn start(
        round: &Round,
        loaded: &crate::content::Loaded,
        player: &MatchConfig,
        opts: Options,
    ) -> Self {
        let cores = cores::usable();
        let fixtures = round.fixtures.len();
        let threads = opts
            .threads
            .unwrap_or_else(|| cores.saturating_sub(1))
            .clamp(1, fixtures.max(1));
        let shared = Arc::new(Shared::new(
            fixtures,
            opts.data_dir,
            opts.match_id,
            opts.fault,
        ));
        let (tx, rx) = channel();
        let started = Instant::now();
        let jobs: Vec<Job> = round
            .fixtures
            .iter()
            .map(|f| Job {
                fixture: f.index,
                seed: f.seed,
                club_ids: f.club_ids.clone(),
                config: fixture_config(f, loaded, player),
                plugins: loaded.script.as_ref().map(script::LoadedPack::plugins),
            })
            .collect();
        tracing::info!(
            signal = "matchday.started",
            match.id = %shared.match_id,
            seed = round.seed,
            fixtures,
            threads,
            cores
        );
        let workers = pool::start(jobs, threads, &shared, &tx, started);
        Self {
            inner: RefCell::new(Inner {
                notes: rx,
                events: Vec::new(),
                ended: vec![None; fixtures],
                late_events: 0,
                max_late_ticks: 0,
                not_late_through: opts.not_late_through,
            }),
            shared,
            workers: RefCell::new(workers),
            round: round.clone(),
            second: ((1.0 / player.tuning.dt).round() as u32).max(1),
            started,
            threads,
            cores,
            fast_forward_to: Cell::new(opts.fast_forward_to),
        }
    }

    /// The worker count.
    pub fn threads(&self) -> usize {
        self.threads
    }

    /// Takes in what the pool computed. An event is late when the player's match on a paced
    /// run had already passed its tick by more than a simulated second when it arrived.
    fn drain(&self, inner: &mut Inner, player_tick: u32, paced: bool) {
        while let Ok(note) = inner.notes.try_recv() {
            self.take(inner, note, player_tick, paced);
        }
    }

    fn take(&self, inner: &mut Inner, note: Note, player_tick: u32, paced: bool) {
        match note {
            Note::Event(mut event, computed_at) => {
                let behind = player_tick.saturating_sub(event.tick);
                if paced && event.tick > inner.not_late_through && behind > self.second {
                    event.late = true;
                    inner.late_events += 1;
                    inner.max_late_ticks = inner.max_late_ticks.max(behind);
                    tracing::info!(
                        signal = "matchday.event_late",
                        fixture = event.fixture,
                        tick = event.tick,
                        player_tick,
                        late_ticks = behind
                    );
                }
                inner.events.push(Stored {
                    event,
                    computed_at,
                    sent: false,
                });
            }
            Note::Ended(ended) => {
                let index = ended.fixture as usize;
                if inner.ended[index].is_some() {
                    return;
                }
                if ended.failure.is_none() {
                    let late: Vec<u32> = inner
                        .events
                        .iter()
                        .filter(|s| s.event.fixture == ended.fixture && s.event.late)
                        .map(|s| player_tick.saturating_sub(s.event.tick))
                        .collect();
                    tracing::info!(
                        signal = "matchday.match_done",
                        match.id = %self.shared.match_id,
                        fixture = ended.fixture,
                        seed = ended.seed,
                        duration_ms = ended.duration_ms,
                        ticks = ended.ticks,
                        events = ended.events,
                        late_events = late.len(),
                        max_late_ticks = late.iter().copied().max().unwrap_or(0),
                        outcome = "success"
                    );
                }
                inner.ended[index] = Some(ended);
            }
        }
    }

    /// Every computed ground event this connection has not been sent whose tick the player's
    /// match has reached, in tick order. It only drains what the pool sent, except at the tick
    /// of a test fast-forward.
    pub fn due(&self, player_tick: u32, paced: bool) -> Vec<ServerMessage> {
        let mut inner = self.inner.borrow_mut();
        if self.fast_forward_to.get() == Some(player_tick) {
            self.catch_up_to(&mut inner, player_tick, paced);
        }
        self.drain(&mut inner, player_tick, paced);
        take_unsent(&mut inner, player_tick)
    }

    /// `true` at the tick of a test fast-forward, where the grounds' progress is sent too.
    pub fn at_fast_forward(&self, player_tick: u32) -> bool {
        self.fast_forward_to.get() == Some(player_tick)
    }

    /// A test seam: follows the playback gate's fast-forward, which a jump moves on in
    /// mid-match. A later target becomes the tick where the grounds are waited for, and no
    /// event up to it is late, as for a fast-forward set at launch.
    pub fn follow_fast_forward(&self, to: Option<u32>) {
        let Some(to) = to else {
            return;
        };
        if self.fast_forward_to.get().is_some_and(|at| at >= to) {
            return;
        }
        self.fast_forward_to.set(Some(to));
        let mut inner = self.inner.borrow_mut();
        inner.not_late_through = inner.not_late_through.max(to);
    }

    /// A test seam: waits, at most `FINISH_WAIT`, until every ground has reached `tick` or
    /// ended. A fast-forward outruns the pool, and the player's match then waits for the page
    /// and sends no more progress, so on a loaded machine the page would never see the
    /// grounds reach the tick. The pool sends a ground's events before it publishes the tick
    /// the ground reached, so every event up to `tick` is in the channel after the wait.
    fn catch_up_to(&self, inner: &mut Inner, tick: u32, paced: bool) {
        let deadline = Instant::now() + FINISH_WAIT;
        let behind = |i: usize| {
            !self.shared.ended[i].load(Ordering::Acquire)
                && self.shared.reached[i].load(Ordering::Acquire) < tick
        };
        while (0..self.round.fixtures.len()).any(behind) {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            match inner
                .notes
                .recv_timeout((deadline - now).min(Duration::from_millis(50)))
            {
                Ok(note) => self.take(inner, note, tick, paced),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    /// Every computed ground event up to `cursor`, for a connection that starts there: a
    /// new page, a reconnect or a resume. None is marked late: the page did not wait for it.
    pub fn catch_up(&self, cursor: u32) -> Vec<ServerMessage> {
        let mut inner = self.inner.borrow_mut();
        self.drain(&mut inner, cursor, false);
        inner.not_late_through = inner.not_late_through.max(cursor);
        for stored in &mut inner.events {
            stored.sent = false;
            stored.event.late = false;
        }
        take_unsent(&mut inner, cursor)
    }

    /// How far each ground has played, for the message sent every simulated second.
    pub fn progress(&self, player_tick: u32) -> Option<ServerMessage> {
        if self.round.fixtures.is_empty() {
            return None;
        }
        Some(ServerMessage::GroundProgress(GroundProgress {
            tick: player_tick,
            reached: self
                .shared
                .reached
                .iter()
                .map(|r| r.load(Ordering::Acquire))
                .collect(),
        }))
    }

    /// After the player's full time: waits until every ground has ended, at most `wait`,
    /// then ends any still running with no result and a bug report, and returns every event
    /// not yet sent. Emits the matchday summary.
    pub fn finish(&self, player_tick: u32, paced: bool, wait: Duration) -> Vec<ServerMessage> {
        let full_time = Instant::now();
        let mut inner = self.inner.borrow_mut();
        let deadline = full_time + wait;
        while inner.ended.iter().any(Option::is_none) {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            match inner
                .notes
                .recv_timeout((deadline - now).min(Duration::from_millis(50)))
            {
                Ok(note) => self.take(&mut inner, note, player_tick, paced),
                Err(RecvTimeoutError::Timeout) => self.drain(&mut inner, player_tick, paced),
                // Every worker is gone, so no ground can still end: the grounds with no end
                // fail now rather than at the deadline.
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        self.drain(&mut inner, player_tick, paced);
        let running: Vec<usize> = (0..inner.ended.len())
            .filter(|&i| inner.ended[i].is_none())
            .collect();
        if !running.is_empty() {
            self.shared.stop.store(true, Ordering::Relaxed);
            let (tx, rx) = channel();
            for i in running {
                let fixture = &self.round.fixtures[i];
                let tick = self.shared.reached[i].load(Ordering::Acquire);
                let score = inner
                    .events
                    .iter()
                    .rev()
                    .find(|s| s.event.fixture == fixture.index)
                    .map_or([0, 0], |s| s.event.score);
                let second = self.second.max(1);
                pool::failed(
                    &FailedGround {
                        fixture: fixture.index,
                        seed: fixture.seed,
                        club_ids: &fixture.club_ids,
                        tick,
                        minute: tick / (second * 60),
                        added: None,
                        score,
                        events: 0,
                    },
                    Failure::DidNotFinish,
                    "the match was still running when the wait after full time ran out",
                    &self.shared,
                    &tx,
                    self.started,
                );
            }
            drop(tx);
            while let Ok(note) = rx.try_recv() {
                self.take(&mut inner, note, player_tick, false);
            }
        }
        let ended: Vec<&pool::Ended> = inner.ended.iter().flatten().collect();
        tracing::info!(
            signal = "matchday.summary",
            match.id = %self.shared.match_id,
            fixtures = self.round.fixtures.len(),
            completed = ended.iter().filter(|e| e.failure.is_none()).count(),
            failed = ended.iter().filter(|e| e.failure.is_some()).count(),
            threads = self.threads,
            cores = self.cores,
            duration_ms = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
            late_events = inner.late_events,
            max_late_ticks = inner.max_late_ticks,
            waited_after_full_time_ms =
                u64::try_from(full_time.elapsed().as_millis()).unwrap_or(u64::MAX)
        );
        take_unsent(&mut inner, u32::MAX)
    }

    /// Every event computed so far with the time it was computed, counted from the pool's
    /// start, in the order the pool sent them. Drains first. For the timing run.
    pub fn computed(&self) -> Vec<(GroundEvent, Duration)> {
        let mut inner = self.inner.borrow_mut();
        self.drain(&mut inner, 0, false);
        inner
            .events
            .iter()
            .map(|s| (s.event.clone(), s.computed_at.duration_since(self.started)))
            .collect()
    }

    /// When the pool started: kick-off for every ground.
    pub fn started(&self) -> Instant {
        self.started
    }

    /// `true` once every ground has ended or failed.
    #[cfg(test)]
    pub fn all_ended(&self) -> bool {
        self.shared.ended.iter().all(|e| e.load(Ordering::Acquire))
    }

    /// How each fixture ended, once the pool reported it, in fixture order.
    pub fn outcomes(&self) -> Vec<Option<(Option<&'static str>, u32, u64)>> {
        let mut inner = self.inner.borrow_mut();
        self.drain(&mut inner, 0, false);
        inner
            .ended
            .iter()
            .map(|e| {
                e.as_ref()
                    .map(|e| (e.failure.map(Failure::code), e.ticks, e.duration_ms))
            })
            .collect()
    }

    /// Stops every worker at its next simulated second and waits for them.
    pub fn stop(&self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        for worker in self.workers.borrow_mut().drain(..) {
            let _ = worker.join();
        }
    }
}

impl Drop for Matchday {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The unsent events at or before `cursor`, marked sent, in tick then fixture order.
fn take_unsent(inner: &mut Inner, cursor: u32) -> Vec<ServerMessage> {
    let mut out: Vec<&mut Stored> = inner
        .events
        .iter_mut()
        .filter(|s| !s.sent && s.event.tick <= cursor)
        .collect();
    out.sort_by_key(|s| (s.event.tick, s.event.fixture));
    out.into_iter()
        .map(|s| {
            s.sent = true;
            ServerMessage::GroundEvent(s.event.clone())
        })
        .collect()
}

/// The match one fixture plays, configured as the player's match is, or why it cannot be
/// built.
fn fixture_config(
    fixture: &round::Fixture,
    loaded: &crate::content::Loaded,
    player: &MatchConfig,
) -> Result<MatchConfig, String> {
    if let Some(why) = &fixture.refused {
        return Err(why.clone());
    }
    let [Some(home), Some(away)] = &fixture.clubs else {
        return Err("a club file is missing".into());
    };
    let mut config = MatchConfig::new(
        fixture.seed,
        player.minutes,
        &loaded.content,
        [&home.file, &away.file],
    )
    .map_err(|e| e.to_string())?;
    if player.knockout {
        config = config.with_knockout();
    }
    loaded.fold(&mut config);
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::data::{TEAM_A_FILE, TEAM_B_FILE};
    use engine::{ContentDir, Simulation};
    use protocol::GroundKind;
    use std::path::Path;

    fn content_dir() -> ContentDir {
        ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
    }

    fn loaded() -> crate::content::Loaded {
        crate::content::load(Some(content_dir().root()), None, None, None).unwrap()
    }

    fn player(loaded: &crate::content::Loaded, seed: u64, minutes: u32) -> MatchConfig {
        let [a, b] = &loaded.teams;
        MatchConfig::new(seed, minutes, &loaded.content, [a, b]).unwrap()
    }

    fn round(loaded: &crate::content::Loaded, seed: u64) -> Round {
        let dir = content_dir();
        let ids = [TEAM_A_FILE, TEAM_B_FILE].map(|f| {
            loaded
                .content
                .load_team(&dir, &dir.path(f))
                .unwrap()
                .value
                .club
                .id
        });
        Round::for_match(&dir, &loaded.content, [&ids[0], &ids[1]], seed)
    }

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("engine-cli-matchday-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn options(name: &str, threads: usize, fault: Option<Fault>) -> Options {
        Options {
            data_dir: temp(name),
            match_id: format!("matchday-test-{name}"),
            threads: Some(threads),
            fault,
            not_late_through: 0,
            fast_forward_to: None,
        }
    }

    /// The score `simulate` plays for one fixture: the same seed, clubs and minutes.
    fn played_alone(loaded: &crate::content::Loaded, f: &round::Fixture, minutes: u32) -> [u32; 2] {
        let [Some(home), Some(away)] = &f.clubs else {
            panic!("a fixture of the shipped round has both clubs");
        };
        let config =
            MatchConfig::new(f.seed, minutes, &loaded.content, [&home.file, &away.file]).unwrap();
        let mut sim = Simulation::new(config).unwrap();
        let cap = sim.config().max_ticks();
        while !sim.is_over() && (sim.tick() < cap || sim.in_shootout()) {
            sim.step();
        }
        sim.finish();
        sim.summary().goals
    }

    fn finals(messages: &[ServerMessage]) -> Vec<(u32, [u32; 2])> {
        let mut out: Vec<(u32, [u32; 2])> = messages
            .iter()
            .filter_map(|m| match m {
                ServerMessage::GroundEvent(e) if e.kind == GroundKind::FullTime => {
                    Some((e.fixture, e.score))
                }
                _ => None,
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn one_two_and_four_threads_play_every_fixture_to_the_result_of_the_full_engine() {
        let loaded = loaded();
        let player = player(&loaded, 42, 10);
        let round = round(&loaded, 42);
        assert_eq!(round.fixtures.len(), 4);
        let expected: Vec<(u32, [u32; 2])> = round
            .fixtures
            .iter()
            .map(|f| (f.index, played_alone(&loaded, f, 10)))
            .collect();
        for threads in [1, 2, 4] {
            let md = Matchday::start(
                &round,
                &loaded,
                &player,
                options(&format!("threads-{threads}"), threads, None),
            );
            assert_eq!(md.threads(), threads);
            let all = md.finish(u32::MAX, false, Duration::from_secs(300));
            assert_eq!(finals(&all), expected, "{threads} threads");
            assert!(
                md.outcomes()
                    .iter()
                    .all(|o| o.is_some_and(|o| o.0.is_none()))
            );
        }
    }

    #[test]
    fn at_the_fast_forward_tick_every_ground_has_reached_it_before_due_returns() {
        let loaded = loaded();
        let player = player(&loaded, 42, 10);
        let round = round(&loaded, 42);
        let to = 20_000;
        // One thread plays the four grounds in turn, so the player's match, here asking at
        // once, is far ahead of them.
        let md = Matchday::start(
            &round,
            &loaded,
            &player,
            Options {
                fast_forward_to: Some(to),
                ..options("fast-forward", 1, None)
            },
        );
        assert!(!md.at_fast_forward(to - 1) && md.at_fast_forward(to));
        let sent = md.due(to, false);
        let Some(ServerMessage::GroundProgress(progress)) = md.progress(to) else {
            panic!("a matchday with fixtures reports progress");
        };
        assert!(
            progress.reached.iter().all(|&r| r >= to),
            "reached {:?} at the fast-forward tick {to}",
            progress.reached
        );
        // Every event up to the tick was sent, and none after it.
        md.finish(u32::MAX, false, Duration::from_secs(300));
        let due: Vec<u32> = md
            .computed()
            .iter()
            .map(|(e, _)| e.tick)
            .filter(|&t| t <= to)
            .collect();
        assert_eq!(sent.len(), due.len());
    }

    #[test]
    fn a_moved_fast_forward_marks_nothing_late_up_to_it_and_waits_there() {
        let loaded = loaded();
        let player = player(&loaded, 42, 10);
        let round = round(&loaded, 42);
        let to = 20_000;
        // Started with no fast-forward, as a served match with --test-jump is; a jump then
        // moves the target on in mid-match.
        let md = Matchday::start(&round, &loaded, &player, options("jump", 1, None));
        md.follow_fast_forward(None);
        assert!(!md.at_fast_forward(to));
        md.follow_fast_forward(Some(to));
        assert!(!md.at_fast_forward(to - 1) && md.at_fast_forward(to));
        md.follow_fast_forward(Some(to - 1_000));
        assert!(
            md.at_fast_forward(to),
            "an earlier target never pulls it back"
        );
        // A paced run asking at once is far ahead of the one worker: due waits there.
        let sent = md.due(to, true);
        let Some(ServerMessage::GroundProgress(progress)) = md.progress(to) else {
            panic!("a matchday with fixtures reports progress");
        };
        assert!(
            progress.reached.iter().all(|&r| r >= to),
            "reached {:?} at the jump's tick {to}",
            progress.reached
        );
        assert!(!sent.is_empty());
        for m in &sent {
            let ServerMessage::GroundEvent(e) = m else {
                panic!("due sends ground events only");
            };
            assert!(e.tick <= to && !e.late, "event at {} sent late", e.tick);
        }
        md.finish(u32::MAX, false, Duration::from_secs(300));
        let due = md.computed().iter().filter(|(e, _)| e.tick <= to).count();
        assert_eq!(sent.len(), due);

        // The control: without the moved target the same paced read marks them late.
        let control = Matchday::start(&round, &loaded, &player, options("jump-control", 1, None));
        while !control.all_ended() {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            control
                .due(to, true)
                .iter()
                .any(|m| matches!(m, ServerMessage::GroundEvent(e) if e.late))
        );
    }

    #[test]
    fn due_never_returns_an_event_past_the_cursor() {
        let loaded = loaded();
        let player = player(&loaded, 7, 10);
        let round = round(&loaded, 7);
        let md = Matchday::start(&round, &loaded, &player, options("due", 2, None));
        while !md.all_ended() {
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut sent = Vec::new();
        for cursor in (0..=player.max_ticks() + 50).step_by(50) {
            for m in md.due(cursor, false) {
                let ServerMessage::GroundEvent(e) = &m else {
                    panic!("due sends ground events only");
                };
                assert!(e.tick <= cursor, "tick {} sent at cursor {cursor}", e.tick);
                assert!(!e.late, "nothing is late on an unpaced run");
                sent.push(e.clone());
            }
        }
        let computed = md.computed();
        assert_eq!(sent.len(), computed.len(), "every event is sent once");
        // The control: one tick before an event, it is not due.
        let first = computed.iter().map(|(e, _)| e.tick).min().unwrap();
        let fresh = Matchday::start(&round, &loaded, &player, options("due-control", 2, None));
        while !fresh.all_ended() {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(fresh.due(first - 1, false).is_empty());
        assert!(!fresh.due(first, false).is_empty());
    }

    #[test]
    fn catch_up_returns_exactly_the_events_up_to_the_cursor_and_none_late() {
        let loaded = loaded();
        let player = player(&loaded, 7, 10);
        let round = round(&loaded, 7);
        let md = Matchday::start(&round, &loaded, &player, options("catch-up", 4, None));
        while !md.all_ended() {
            std::thread::sleep(Duration::from_millis(5));
        }
        // A paced run far ahead of every event: all of them are late.
        let late = md.due(u32::MAX - 1, true);
        assert!(
            late.iter()
                .all(|m| matches!(m, ServerMessage::GroundEvent(e) if e.late))
        );
        let cursor = 5 * 3_000;
        let again = md.catch_up(cursor);
        let expected = md
            .computed()
            .iter()
            .filter(|(e, _)| e.tick <= cursor)
            .count();
        assert_eq!(again.len(), expected);
        for m in &again {
            let ServerMessage::GroundEvent(e) = m else {
                panic!("catch-up sends ground events only");
            };
            assert!(e.tick <= cursor && !e.late);
        }
        // After the catch-up the rest come at their ticks, none late.
        let rest = md.due(u32::MAX - 1, true);
        assert!(
            rest.iter()
                .all(|m| matches!(m, ServerMessage::GroundEvent(e) if !e.late))
        );
        assert_eq!(again.len() + rest.len(), md.computed().len());
    }

    #[test]
    fn a_planted_panic_makes_its_fixture_unavailable_while_the_others_finish() {
        let loaded = loaded();
        let player = player(&loaded, 42, 10);
        let round = round(&loaded, 42);
        let opts = options(
            "fault",
            2,
            Some(Fault {
                fixture: 1,
                tick: 6_000,
            }),
        );
        let data = opts.data_dir.clone();
        let match_id = opts.match_id.clone();
        let md = Matchday::start(&round, &loaded, &player, opts);
        let all = md.finish(u32::MAX, false, Duration::from_secs(300));
        let unavailable: Vec<&GroundEvent> = all
            .iter()
            .filter_map(|m| match m {
                ServerMessage::GroundEvent(e) if e.kind == GroundKind::Unavailable => Some(e),
                _ => None,
            })
            .collect();
        assert_eq!(unavailable.len(), 1);
        assert_eq!((unavailable[0].fixture, unavailable[0].tick), (1, 6_000));
        assert_eq!(unavailable[0].minute, 2);
        let done: Vec<u32> = finals(&all).iter().map(|f| f.0).collect();
        assert_eq!(done, vec![0, 2, 3], "the other fixtures reach full time");
        let report = data
            .join("matches")
            .join(&match_id)
            .join("matchday-bug-1.json");
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
        assert_eq!(json["seed"], round.fixtures[1].seed);
        assert_eq!(json["error.type"], "panic");
        assert_eq!(json["build.hash"], engine::build_hash());
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn stop_ends_every_worker_within_a_simulated_second() {
        let loaded = loaded();
        let player = player(&loaded, 42, 90);
        let round = round(&loaded, 42);
        let md = Matchday::start(&round, &loaded, &player, options("stop", 4, None));
        std::thread::sleep(Duration::from_millis(20));
        let asked = Instant::now();
        md.stop();
        assert!(
            asked.elapsed() < Duration::from_secs(5),
            "stop took {:?}",
            asked.elapsed()
        );
        assert!(md.workers.borrow().is_empty());
    }

    #[test]
    fn a_fault_reads_fixture_at_tick() {
        assert_eq!(
            "1@102000".parse::<Fault>(),
            Ok(Fault {
                fixture: 1,
                tick: 102_000
            })
        );
        assert!("1-102000".parse::<Fault>().is_err());
        assert!("x@1".parse::<Fault>().is_err());
    }

    #[test]
    fn the_matchday_message_lists_every_fixture_with_both_clubs() {
        let loaded = loaded();
        let round = round(&loaded, 42);
        let ServerMessage::Matchday(md) = message(&round) else {
            panic!("not a matchday message");
        };
        assert_eq!(md.round, 1);
        assert_eq!(md.fixtures.len(), 4);
        for (i, f) in md.fixtures.iter().enumerate() {
            assert_eq!(f.fixture as usize, i);
            assert!(f.home.kit_primary.starts_with('#'));
            assert!(f.home.roster.is_empty());
            assert_ne!(f.home.id, f.away.id);
        }
    }
}
