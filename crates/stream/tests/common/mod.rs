#![allow(dead_code)]

//! Shared harness: a real seeded match served over a real loopback socket, in this
//! process. Every test gets its own data folder, so the tests run in parallel and never
//! read the runtime folder.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread::JoinHandle;

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::record::TickSink;
use engine::{Content, ContentDir, MatchConfig, Simulation};
use protocol::{
    ChangeKind, EventType, Hello, MatchEvent, PROTOCOL_VERSION, Queue, ServerMessage, Stats,
    TeamRef,
};
use stream::session::{MatchState, SessionConfig};
use stream::{CommandContext, Gate, Gauge, Server, Session, StreamError};

pub const SEED: u64 = 42;
/// Simulated ticks in one minute of play.
pub const TICKS_PER_MINUTE: u32 = 50 * 60;

/// The `content/` folder at the workspace root.
pub fn content_dir() -> ContentDir {
    ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
}

/// The seeded match on the shipped content and the two default teams.
pub fn match_config(minutes: u32) -> MatchConfig {
    let dir = content_dir();
    let content = Content::load(&dir).expect("the shipped content loads");
    let teams: Vec<TeamFile> = [TEAM_A_FILE, TEAM_B_FILE]
        .iter()
        .map(|rel| {
            content
                .load_team(&dir, &dir.path(rel))
                .expect("a default team loads")
                .value
        })
        .collect();
    MatchConfig::new(SEED, minutes, &content, [&teams[0], &teams[1]])
        .expect("the seeded match is valid")
}

/// A data folder of this test's own.
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("stream-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the test folder is created");
    dir
}

/// A match being served to one client.
pub struct Served {
    pub port: u16,
    pub data_dir: PathBuf,
    pub ticks_expected: u32,
    gauge_rx: Receiver<Arc<Gauge>>,
    handle: Option<JoinHandle<Result<u32, String>>>,
}

impl Served {
    /// Binds a port and waits for one client on a thread. Returns as soon as the port is
    /// known, so the test can connect.
    pub fn start(name: &str, minutes: u32, buffer_ticks: usize) -> Self {
        let data_dir = temp_dir(name);
        let ticks = match_config(minutes).max_ticks();
        let (port_tx, port_rx) = channel();
        let (gauge_tx, gauge_rx) = channel();
        let thread_dir = data_dir.clone();
        let handle = std::thread::spawn(move || {
            serve(thread_dir, minutes, buffer_ticks, port_tx, gauge_tx).map_err(|e| format!("{e}"))
        });
        let port = port_rx.recv().expect("the server reports its port");
        Self {
            port,
            data_dir,
            ticks_expected: ticks,
            gauge_rx,
            handle: Some(handle),
        }
    }

    /// The producer buffer gauge. Available once a client has connected.
    pub fn gauge(&self) -> Arc<Gauge> {
        self.gauge_rx
            .recv()
            .expect("the session reports its gauge once a client connects")
    }

    /// Waits for the match to end and returns the ticks it wrote.
    pub fn join(mut self) -> u32 {
        self.handle
            .take()
            .expect("the server thread is joined once")
            .join()
            .expect("the server thread does not panic")
            .expect("the server thread succeeds")
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        let _ = std::fs::remove_dir_all(&self.data_dir);
    }
}

fn serve(
    data_dir: PathBuf,
    minutes: u32,
    buffer_ticks: usize,
    port_tx: std::sync::mpsc::Sender<u16>,
    gauge_tx: std::sync::mpsc::Sender<Arc<Gauge>>,
) -> Result<u32, StreamError> {
    let config = match_config(minutes);
    let ticks = config.max_ticks();
    let owner_id = "0123456789abcdef0123456789abcdef".to_string();
    let match_id = format!("{SEED:016x}-1700000000000");
    let club_ids = [
        config.teams[0].club_id.clone(),
        config.teams[1].club_id.clone(),
    ];
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION,
        engine_version: engine::version().to_string(),
        build_hash: engine::build_hash().to_string(),
        owner_id: owner_id.clone(),
        match_id: match_id.clone(),
        seed: SEED,
        dt_ms: config.tuning.dt * 1000.0,
        ticks_expected: ticks,
        keyframe_interval: 50,
        teams: [
            TeamRef {
                id: club_ids[0].clone(),
                name: config.teams[0].name.clone(),
                kit_primary: config.teams[0].kit.primary.clone(),
                kit_secondary: config.teams[0].kit.secondary.clone(),
            },
            TeamRef {
                id: club_ids[1].clone(),
                name: config.teams[1].name.clone(),
                kit_primary: config.teams[1].kit.primary.clone(),
                kit_secondary: config.teams[1].kit.secondary.clone(),
            },
        ],
    };

    let server = Server::bind(&data_dir, &match_id)?;
    port_tx.send(server.port()).expect("the test is listening");
    let connection = server.accept(&match_id)?;
    let events = stream::session::shared_events(&data_dir, &match_id)?;
    let gate = Arc::new(Gate::new());
    let state = Arc::new(MatchState::default());
    let session = Session::start(
        connection,
        SessionConfig {
            buffer_ticks,
            keyframe_interval: 50,
            hello,
            commands: CommandContext {
                owner_id: owner_id.clone(),
                match_id: match_id.clone(),
                gate: Arc::clone(&gate),
                state: Arc::clone(&state),
                events: Arc::clone(&events),
                queue: Queue::new(ChangeKind::ALL.to_vec()),
            },
        },
    )?;
    let _ = gauge_tx.send(Arc::clone(session.gauge()));

    let mut sink = session.sink();
    let mut sim = Simulation::new(config)?;
    let mut written = 0u32;
    'play: while !sim.is_over() && written < ticks {
        if !gate.wait_until_running() {
            break;
        }
        sim.step();
        let record = sim.record();
        state.set_tick(record.tick);
        if sink.on_tick(&record).is_err() {
            break 'play;
        }
        written += 1;
        for event in sim.take_events() {
            state.set_scores(event.scores);
            let message = ServerMessage::Event(play_event(&event, &owner_id, &match_id, &club_ids));
            if let ServerMessage::Event(e) = &message {
                events
                    .lock()
                    .expect("the event writer lock is never poisoned")
                    .write(e)?;
            }
            if session.send(&message).is_err() {
                break 'play;
            }
        }
    }
    sim.finish();
    for event in sim.take_events() {
        let message = ServerMessage::Event(play_event(&event, &owner_id, &match_id, &club_ids));
        let _ = session.send(&message);
    }
    let summary = sim.summary();
    let _ = session.send(&ServerMessage::Stats(Stats {
        tick: sim.tick(),
        minute: sim.minute().0,
        home_score: summary.goals[0],
        away_score: summary.goals[1],
        possession_changes: summary.possession_changes,
        ball_max_speed: summary.ball_max_speed,
        ball_idle_ticks: summary.ball_idle_ticks,
    }));
    drop(sink);
    session.finish()?;
    Ok(written)
}

fn play_event(
    event: &engine::EngineEvent,
    owner_id: &str,
    match_id: &str,
    club_ids: &[String; 2],
) -> MatchEvent {
    use engine::EngineEventKind as K;
    let event_type = match event.kind {
        K::KickOff => EventType::KickOff,
        K::Goal => EventType::Goal,
        K::HalfTime => EventType::HalfTime,
        K::FullTime => EventType::FullTime,
        K::Offside => EventType::Offside,
        K::Foul => EventType::Foul,
        K::Card => EventType::Card,
        K::ThrowIn => EventType::ThrowIn,
        K::Corner => EventType::Corner,
        K::GoalKick => EventType::GoalKick,
        K::FreeKick => EventType::FreeKick,
        K::Penalty => EventType::Penalty,
        K::Injury => EventType::Injury,
        K::Substitution => EventType::Substitution,
        K::AiDecision => EventType::AiDecision,
        K::ChangeApplied | K::ChangeRejected => EventType::TacticsChange,
    };
    MatchEvent::play(
        owner_id,
        match_id,
        event.tick,
        event_type,
        event.team.map(|t| club_ids[t].clone()),
        event.scores,
    )
    .at_minute(event.minute, event.minute_added)
}
