//! `engine-cli serve`: stream one match live over the local socket.
//!
//! The session holds before kick-off. The page receives the hello, may send the home lineup
//! and its pre-match tactics with `set-lineup`, and starts the match with `start`; only then is
//! the simulation built, so the chosen lineup is the one that kicks off. Without a lineup the
//! computer manager's pre-match setup stands.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use engine::data::team::Position;
use engine::observe::identity::{MatchId, data_dir, load_or_create_owner_id, owner_bytes};
use engine::{FanoutSink, FileSink, Manager, MatchConfig, Simulation, SnapshotSink, TickHeader};
use protocol::{ChangeKind, Hello, PROTOCOL_VERSION, Queue, ServerMessage};
use stream::events::EventWriter;
use stream::session::{MatchState, SessionConfig};
use stream::{CommandContext, Gate, Inbox, LineupRules, PageSetup, PreMatch, Server, Session};

use crate::cli::ServeOpts;
use crate::stream_run::{Drive, drive, hello_substitutions, hello_tactics, hello_teams};

/// The team the page manages.
const HOME: usize = 0;

pub fn run(content_dir: Option<&Path>, opts: &ServeOpts) -> anyhow::Result<i32> {
    let loaded = crate::content::load(content_dir, opts.team_a.as_deref(), opts.team_b.as_deref())?;
    let [team_a, team_b] = &loaded.teams;
    // The home team is the page's: it starts with the AI manager's pre-match setup, unless
    // the page sends its own, and makes no in-match AI decisions. The AI manager runs the
    // away team.
    let config = MatchConfig::new(opts.seed, opts.minutes, &loaded.content, [team_a, team_b])?
        .with_manager(HOME, Manager::Human);
    let stream_tuning = loaded.content.tuning.stream.clone();
    let data = data_dir();
    let owner_id = load_or_create_owner_id(&data)?;
    let started = MatchId::now(opts.seed);
    let match_millis = started.millis;
    let match_id = started.to_string();
    let ticks = config.max_ticks();
    let club_ids = [
        config.teams[0].club_id.clone(),
        config.teams[1].club_id.clone(),
    ];
    let dt = config.tuning.dt;
    // The hello describes the computer manager's pre-match setup, which the page's lineup
    // editor starts from. The match itself is built after `start`.
    let preview = Simulation::new(config.clone())?;
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION,
        engine_version: engine::version().to_string(),
        build_hash: engine::build_hash().to_string(),
        owner_id: owner_id.clone(),
        match_id: match_id.clone(),
        seed: opts.seed,
        dt_ms: dt * 1000.0,
        ticks_expected: ticks,
        keyframe_interval: stream_tuning.keyframe_interval,
        teams: hello_teams(&preview, true),
        tactics: hello_tactics(&preview),
        substitutions: hello_substitutions(&preview),
    };
    drop(preview);
    let pre_match = Arc::new(PreMatch::new(LineupRules {
        keepers: config.teams[HOME]
            .squad
            .iter()
            .map(|p| p.position == Position::GK)
            .collect(),
        bench_size: usize::from(config.tactics.ai.bench_size),
        tactics: config.tactics.clone(),
    }));

    let server = Server::bind(&data, &match_id)?;
    println!("{}", server.port());
    // The page address is printed after the port, because it is the line a reader copies.
    let page = match opts.web.as_deref() {
        Some(dir) => Some(crate::web::start(dir, server.port())?),
        None => None,
    };
    if let Some(page) = &page {
        println!("{}", page.address());
    }
    let connection = server.accept(&match_id)?;

    let events = Arc::new(Mutex::new(EventWriter::open(&data, &match_id)?));
    let gate = Arc::new(Gate::held());
    // A page that reports the tick it draws keeps the engine within the buffer bound of it,
    // so a change the manager queues reaches the engine before the stoppage on screen.
    gate.set_lead_bound(u32::try_from(stream_tuning.buffer_ticks).unwrap_or(u32::MAX));
    let state = Arc::new(MatchState::default());
    let inbox = Arc::new(Inbox::default());
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
                pre_match: Arc::clone(&pre_match),
                inbox: Arc::clone(&inbox),
            },
        },
    )?;

    // The hold. A page that leaves before kick-off ends the run short.
    if !gate.wait_until_running() {
        session.finish()?;
        tracing::info!(
            signal = "socket.client_gone",
            written = 0,
            reason = "the viewer left before kick-off"
        );
        return Ok(2);
    }
    let config = match pre_match.take() {
        Some(setup) => with_page_setup(config, setup),
        None => config,
    };
    let mut sim = Simulation::new(config)?;

    let ticks_file = match opts.ticks_out.as_deref() {
        Some(path) => Some(
            FileSink::create(
                path,
                &TickHeader {
                    seed: opts.seed,
                    dt,
                    expected_ticks: ticks,
                    owner_id: owner_bytes(&owner_id)?,
                    match_millis,
                },
            )
            .with_context(|| format!("cannot create {}", path.display()))?,
        ),
        None => None,
    };
    let snapshots = SnapshotSink::new(&data, &match_id, owner_bytes(&owner_id)?, match_millis);
    let mut sink = FanoutSink::new(FanoutSink::new(session.sink(), ticks_file), snapshots);
    let driven = drive(
        &mut sim,
        &mut sink,
        &Drive {
            ticks,
            owner_id: &owner_id,
            match_id: &match_id,
            club_ids: [&club_ids[0], &club_ids[1]],
            state: &state,
            gate: Some(&gate),
            commentary: &loaded.commentary,
            inbox: Some(&inbox),
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

    let (streams, _) = sink.into_parts();
    let (_, ticks_file) = streams.into_parts();
    if let Some(file) = ticks_file {
        file.finish()?;
    }
    let gauge = Arc::clone(session.gauge());
    session.finish()?;
    tracing::info!(
        signal = "socket.session_closed",
        ticks = driven.written,
        high_water = gauge.high_water(),
        bound = gauge.bound(),
        pauses = gauge.pauses(),
        paused_ms = gauge.paused_ms()
    );
    Ok(if driven.full_time { 0 } else { 2 })
}

/// The match with the page's lineup, bench, and pre-match tactics for the home team. The
/// socket checked the lineup and the patch's indices before it stored them.
pub(crate) fn with_page_setup(config: MatchConfig, setup: PageSetup) -> MatchConfig {
    let config = config.with_setup(HOME, setup.lineup, setup.bench);
    match setup.patch {
        Some(patch) => {
            let tactics =
                patch.applied_to(config.teams[HOME].tactics, &setup.lineup, &config.tactics);
            config.with_tactics(HOME, tactics)
        }
        None => config,
    }
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
