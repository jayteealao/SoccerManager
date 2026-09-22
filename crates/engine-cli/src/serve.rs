//! `engine-cli serve`: stream one match live over the local socket.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use engine::observe::identity::{MatchId, data_dir, load_or_create_owner_id, owner_bytes};
use engine::{FanoutSink, FileSink, MatchConfig, Simulation, TickHeader, ticks_for_minutes};
use protocol::{ChangeKind, Hello, PROTOCOL_VERSION, Queue, ServerMessage, TeamRef};
use stream::events::EventWriter;
use stream::session::{MatchState, SessionConfig};
use stream::{CommandContext, Gate, Server, Session};

use crate::cli::ServeOpts;
use crate::stream_run::{Drive, drive};

pub fn run(content_dir: Option<&Path>, opts: &ServeOpts) -> anyhow::Result<i32> {
    let loaded = crate::content::load(content_dir, opts.team_a.as_deref(), opts.team_b.as_deref())?;
    let [team_a, team_b] = &loaded.teams;
    let config = MatchConfig::new(opts.seed, opts.minutes, &loaded.content, [team_a, team_b])?;
    let stream_tuning = loaded.content.tuning.stream.clone();
    let data = data_dir();
    let owner_id = load_or_create_owner_id(&data)?;
    let match_id = MatchId::now(opts.seed).to_string();
    let ticks = ticks_for_minutes(opts.minutes);
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
        seed: opts.seed,
        dt_ms: config.tuning.dt * 1000.0,
        ticks_expected: ticks,
        keyframe_interval: stream_tuning.keyframe_interval,
        teams: [
            TeamRef {
                id: club_ids[0].clone(),
                name: config.teams[0].name.clone(),
            },
            TeamRef {
                id: club_ids[1].clone(),
                name: config.teams[1].name.clone(),
            },
        ],
    };

    let server = Server::bind(&data, &match_id)?;
    println!("{}", server.port());
    let connection = server.accept(&match_id)?;

    let events = Arc::new(Mutex::new(EventWriter::open(&data, &match_id)?));
    let gate = Arc::new(Gate::new());
    let state = Arc::new(MatchState::default());
    let session = Session::start(
        connection,
        SessionConfig {
            buffer_ticks: stream_tuning.buffer_ticks,
            keyframe_interval: stream_tuning.keyframe_interval,
            hello,
            commands: CommandContext {
                owner_id: owner_id.clone(),
                match_id: match_id.clone(),
                gate: Arc::clone(&gate),
                state: Arc::clone(&state),
                events: Arc::clone(&events),
                queue: Queue::new(admitted_kinds(&loaded.content.rules)),
            },
        },
    )?;

    let ticks_file = match opts.ticks_out.as_deref() {
        Some(path) => Some(
            FileSink::create(
                path,
                &TickHeader {
                    seed: opts.seed,
                    dt: config.tuning.dt,
                    expected_ticks: ticks,
                    owner_id: owner_bytes(&owner_id)?,
                    match_millis: match_id.parse::<MatchId>()?.millis,
                },
            )
            .with_context(|| format!("cannot create {}", path.display()))?,
        ),
        None => None,
    };
    let mut sink = FanoutSink::new(session.sink(), ticks_file);
    let mut sim = Simulation::new(config)?;
    let written = drive(
        &mut sim,
        &mut sink,
        &Drive {
            ticks,
            owner_id: &owner_id,
            match_id: &match_id,
            club_ids: [&club_ids[0], &club_ids[1]],
            state: &state,
            gate: Some(&gate),
        },
        &mut |message: ServerMessage| {
            if let ServerMessage::Event(event) = &message {
                events
                    .lock()
                    .expect("the event writer lock is never poisoned")
                    .write(event)?;
            }
            session.send(&message)
        },
    )?;

    let (_, ticks_file) = sink.into_parts();
    if let Some(file) = ticks_file {
        file.finish()?;
    }
    let gauge = Arc::clone(session.gauge());
    session.finish()?;
    tracing::info!(
        signal = "socket.session_closed",
        ticks = written,
        high_water = gauge.high_water(),
        bound = gauge.bound(),
        pauses = gauge.pauses(),
        paused_ms = gauge.paused_ms()
    );
    Ok(if written == ticks { 0 } else { 2 })
}

/// The change kinds the rule pack admits at some stoppage.
pub fn admitted_kinds(rules: &engine::data::RulePack) -> Vec<ChangeKind> {
    let mut kinds = Vec::new();
    if rules.stoppages.iter().any(|s| s.admits_tactics) {
        kinds.push(ChangeKind::Tactics);
    }
    if rules.stoppages.iter().any(|s| s.admits_substitution) {
        kinds.push(ChangeKind::Substitution);
    }
    kinds
}
