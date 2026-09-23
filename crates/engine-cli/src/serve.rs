//! `engine-cli serve`: stream one match live over the local socket.
//!
//! The session holds before kick-off. The page receives the hello, may send the home lineup
//! and its pre-match tactics with `set-lineup`, and starts the match with `start`; only then is
//! the simulation built, so the chosen lineup is the one that kicks off. Without a lineup the
//! computer manager's pre-match setup stands.
//!
//! With `--resume` the match continues from a snapshot instead, with no hold: it was kicked
//! off before the snapshot was taken. With `--reconnect-wait` a connection lost without a
//! close frame does not end the run: the match goes back to the newest stoppage the viewer
//! received in full and waits for the viewer on the same port.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use engine::data::team::Position;
use engine::observe::identity::{
    DATA_DIR_ENV, MatchId, data_dir, load_or_create_owner_id, owner_bytes, owner_hex,
};
use engine::observe::{
    LawStats, MatchFigures, MatchStats, ScriptFigures, TacticsStats, TeamRef, write_stats,
};
use engine::{EngineError, FanoutSink, FileSink, Manager, MatchConfig, Simulation, Snapshot};
use engine::{TickHeader, snapshot::shorten_for_log};
use protocol::{ChangeKind, Hello, PROTOCOL_VERSION, Queue, ServerMessage};
use stream::events::EventWriter;
use stream::session::{MatchState, SessionConfig};
use stream::{
    CommandContext, Gate, GatedSnapshots, Inbox, LineupRules, PageSetup, PreMatch, Server, Session,
    SessionEnd, StreamError,
};

use crate::cli::ServeOpts;
use crate::stream_run::{Drive, drive, hello_substitutions, hello_tactics, hello_teams};

/// The team the page manages.
const HOME: usize = 0;

/// Who the match belongs to and where its files live.
struct Identity {
    owner_id: String,
    owner: [u8; 16],
    match_id: String,
    match_millis: u64,
    seed: u64,
}

/// The statistics record of a served match at full time, as `simulate` writes it. The ticks
/// were streamed rather than written to a tick file, so no validator ran over them.
fn match_stats(
    identity: &Identity,
    config: &MatchConfig,
    sim: &Simulation,
    written: u32,
    played_from: Instant,
    snapshot_writes: u32,
) -> MatchStats {
    let elapsed = played_from.elapsed();
    let summary = sim.summary();
    let team = |i: usize| TeamRef {
        id: config.teams[i].club_id.clone(),
        name: config.teams[i].name.clone(),
    };
    MatchStats {
        owner_id: identity.owner_id.clone(),
        match_id: identity.match_id.clone(),
        seed: identity.seed,
        content_hash: config.content_hash.clone(),
        teams: [team(0), team(1)],
        duration_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        outcome: "success".into(),
        ticks_per_s: f64::from(written) / elapsed.as_secs_f64().max(1e-9),
        ticks_written: sim.tick(),
        validate_ran: false,
        validate_violations: 0,
        possession_changes: summary.possession_changes,
        ball_max_speed: summary.ball_max_speed,
        ball_idle_ticks: summary.ball_idle_ticks,
        goals: summary.goals,
        flags_on: config.flags.names().to_vec(),
        laws: LawStats::new(
            &summary,
            config.rules.schema_version,
            sim.tick(),
            snapshot_writes,
        ),
        tactics: TacticsStats::new(sim),
        figures: MatchFigures::new(&summary, sim.managers()),
        script: ScriptFigures::new(sim.plugins()),
    }
}

/// A match ready to serve: fresh and held before kick-off, or resumed and ready to play.
struct Opened {
    identity: Identity,
    /// The match as configured. A fresh match gains the page's lineup at kick-off.
    config: MatchConfig,
    /// The resumed match; `None` until a fresh match kicks off.
    sim: Option<Simulation>,
    /// The tick a resumed match continues from.
    resume_tick: Option<u32>,
    hello: Hello,
}

pub fn run(content_dir: Option<&Path>, opts: &ServeOpts) -> anyhow::Result<i32> {
    let loaded = crate::content::load(
        content_dir,
        opts.team_a.as_deref(),
        opts.team_b.as_deref(),
        opts.script_pack.as_deref(),
    )?;
    let data = data_dir();
    let stream_tuning = loaded.content.tuning.stream.clone();
    let opened = match opts.resume.as_deref() {
        Some(path) => match open_resumed(&loaded, path) {
            Ok(opened) => opened,
            Err(err) => {
                // Before the port line, so a launcher reading the output can tell a refused
                // snapshot from a crash: the process ends without ever printing a port.
                let shown = path.display().to_string();
                let data_dir = std::env::var_os(DATA_DIR_ENV).map(std::path::PathBuf::from);
                tracing::error!(
                    signal = "snapshot.refused",
                    path = %shorten_for_log(&shown, data_dir.as_deref()),
                    reason = %err
                );
                eprintln!("error: {err}");
                return Ok(1);
            }
        },
        None => open_fresh(&loaded, opts, &data)?,
    };
    let Opened {
        identity,
        mut config,
        mut sim,
        resume_tick,
        hello,
    } = opened;
    let ticks = config.max_ticks();
    let club_ids = [
        config.teams[0].club_id.clone(),
        config.teams[1].club_id.clone(),
    ];
    let match_id = identity.match_id.clone();
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
        Some(dir) => Some(crate::web::start(
            dir,
            Arc::new(crate::web::Fixed {
                socket_port: server.port(),
                match_id: match_id.clone(),
            }),
        )?),
        None => None,
    };
    if let Some(page) = &page {
        println!("{}", page.address());
    }

    let state = Arc::new(MatchState::default());
    let mut gated = GatedSnapshots::new(
        &data,
        &match_id,
        identity.owner,
        identity.match_millis,
        Arc::clone(&state),
    );
    // The events file: a resumed match keeps the rows up to its snapshot.
    let mut writer = Some(match resume_tick {
        Some(tick) => EventWriter::resume(&data, &match_id, tick)?,
        None => EventWriter::open(&data, &match_id)?,
    });
    if let Some(tick) = resume_tick {
        state.set_tick(tick);
        state.set_sent_tick(tick);
    }
    let mut ticks_file = match opts.ticks_out.as_deref() {
        Some(path) => Some(
            FileSink::create(
                path,
                &TickHeader {
                    seed: identity.seed,
                    dt: config.tuning.dt,
                    expected_ticks: ticks,
                    owner_id: identity.owner,
                    match_millis: identity.match_millis,
                },
            )
            .with_context(|| format!("cannot create {}", path.display()))?,
        ),
        None => None,
    };
    let wait = Duration::from_secs(opts.reconnect_wait);
    let mut drop_at = opts.drop_client_at;
    let mut connection = server.accept(&match_id)?;
    loop {
        let events = Arc::new(Mutex::new(
            writer
                .take()
                .expect("each connection opens the events file"),
        ));
        // A match that has kicked off runs at once; a fresh one holds for the page's lineup.
        let gate = Arc::new(if sim.is_some() {
            Gate::new()
        } else {
            Gate::held()
        });
        // A page that reports the tick it draws keeps the engine within the buffer bound of
        // it, so a change the manager queues reaches the engine before the stoppage on screen.
        gate.set_lead_bound(u32::try_from(stream_tuning.buffer_ticks).unwrap_or(u32::MAX));
        let inbox = Arc::new(Inbox::default());
        let session = Session::start(
            connection,
            SessionConfig {
                buffer_ticks: stream_tuning.buffer_ticks,
                keyframe_interval: stream_tuning.keyframe_interval,
                hello: hello.clone(),
                commands: CommandContext {
                    owner_id: identity.owner_id.clone(),
                    match_id: match_id.clone(),
                    gate: Arc::clone(&gate),
                    state: Arc::clone(&state),
                    events: Arc::clone(&events),
                    queue: Queue::new(admitted_kinds(&loaded.content.rules)),
                    pre_match: if sim.is_some() {
                        Arc::new(PreMatch::none())
                    } else {
                        Arc::clone(&pre_match)
                    },
                    inbox: Arc::clone(&inbox),
                },
                drop_at: drop_at.take(),
            },
        )?;

        let mut full_time = false;
        let mut written = 0;
        let started = sim.is_some() || gate.wait_until_running();
        if started && sim.is_none() {
            if let Some(setup) = pre_match.take() {
                config = with_page_setup(config, setup);
            }
            let mut kicked_off = Simulation::new(config.clone())?;
            loaded.attach(&mut kicked_off);
            // The kick-off state, with the page's lineup: the restart point until the first
            // stoppage.
            gated.capture(&kicked_off);
            sim = Some(kicked_off);
        }
        if let Some(sim) = sim.as_mut().filter(|_| started) {
            let played_from = Instant::now();
            let mut sink = FanoutSink::new(
                FanoutSink::new(session.sink(), ticks_file.take()),
                &mut gated,
            );
            let driven = drive(
                sim,
                &mut sink,
                &Drive {
                    ticks,
                    owner_id: &identity.owner_id,
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
            );
            match driven {
                Ok(driven) => {
                    full_time = driven.full_time;
                    written = driven.written;
                }
                // A message the socket could not take: the viewer is gone.
                Err(StreamError::ClientGone) => {}
                Err(other) => return Err(other.into()),
            }
            let (streams, _) = sink.into_parts();
            let (_, file) = streams.into_parts();
            if let Some(file) = file {
                file.finish()?;
            }
            if full_time {
                let path = write_stats(
                    &data,
                    &match_stats(&identity, &config, sim, written, played_from, gated.writes),
                )?;
                tracing::info!(signal = "stats.written", path = %path.display());
            }
        }

        let gauge = Arc::clone(session.gauge());
        let end = session.finish()?;
        tracing::info!(
            signal = "socket.session_closed",
            ticks = written,
            high_water = gauge.high_water(),
            bound = gauge.bound(),
            pauses = gauge.pauses(),
            paused_ms = gauge.paused_ms()
        );
        if full_time {
            return Ok(0);
        }
        if end != SessionEnd::Dropped || wait.is_zero() {
            if !started {
                tracing::info!(
                    signal = "socket.client_gone",
                    written = 0,
                    reason = "the viewer left before kick-off"
                );
            }
            return Ok(2);
        }

        // The connection dropped. Go back to the newest stoppage the viewer holds in full.
        let sent_tick = state.sent_tick();
        tracing::info!(signal = "socket.dropped", tick = state.tick(), sent_tick);
        drop(events);
        let resume_at = if started {
            let mut resumed = match gated.newest_before(sent_tick) {
                Some(snapshot) => Simulation::from_snapshot(config.clone(), snapshot)?,
                // No stoppage reached the viewer yet: the match starts again from kick-off,
                // with the same seed, lineup, and identity.
                None => Simulation::new(config.clone())?,
            };
            loaded.attach(&mut resumed);
            let tick = resumed.tick();
            sim = Some(resumed);
            tick
        } else {
            0
        };
        gated.rewind(resume_at);
        state.set_tick(resume_at);
        state.set_sent_tick(resume_at);
        writer = Some(EventWriter::resume(&data, &match_id, resume_at)?);
        match server.accept_within(&match_id, wait)? {
            Some(next) => {
                tracing::info!(signal = "socket.reconnected", resume_tick = resume_at);
                connection = next;
            }
            None => {
                tracing::info!(
                    signal = "socket.client_gone",
                    written = 0,
                    reason = "the viewer did not reconnect in time"
                );
                return Ok(2);
            }
        }
    }
}

/// A new match, held before kick-off for the page's lineup.
fn open_fresh(
    loaded: &crate::content::Loaded,
    opts: &ServeOpts,
    data: &Path,
) -> anyhow::Result<Opened> {
    let [team_a, team_b] = &loaded.teams;
    let seed = opts
        .seed
        .context("a seed is required unless the match resumes")?;
    // The home team is the page's: it starts with the AI manager's pre-match setup, unless
    // the page sends its own, and makes no in-match AI decisions. The AI manager runs the
    // away team.
    let mut config = MatchConfig::new(seed, opts.minutes, &loaded.content, [team_a, team_b])?
        .with_manager(HOME, Manager::Human);
    if opts.knockout {
        config = config.with_knockout();
    }
    loaded.fold(&mut config);
    let owner_id = load_or_create_owner_id(data)?;
    let started = match opts.match_millis {
        // The launcher chooses the stamp, so it knows where this match's snapshot is.
        Some(millis) => MatchId { seed, millis },
        None => MatchId::now(seed),
    };
    let identity = Identity {
        owner: owner_bytes(&owner_id)?,
        owner_id,
        match_id: started.to_string(),
        match_millis: started.millis,
        seed,
    };
    // The hello describes the computer manager's pre-match setup, which the page's lineup
    // editor starts from. The match itself is built after `start`.
    let preview = Simulation::new(config.clone())?;
    let hello = hello_for(
        &preview,
        &identity,
        &config,
        loaded.content.tuning.stream.keyframe_interval,
    );
    Ok(Opened {
        identity,
        config,
        sim: None,
        resume_tick: None,
        hello,
    })
}

/// A match continued from `path`. Any refusal names the check that failed.
fn open_resumed(loaded: &crate::content::Loaded, path: &Path) -> Result<Opened, EngineError> {
    let shown = path.display().to_string();
    let snapshot = Snapshot::read(path, &shown)?;
    let [team_a, team_b] = &loaded.teams;
    let mut config = MatchConfig::new(
        snapshot.seed(),
        snapshot.minutes(),
        &loaded.content,
        [team_a, team_b],
    )?
    .with_manager(HOME, Manager::Human);
    if snapshot.knockout() {
        config = config.with_knockout();
    }
    loaded.fold(&mut config);
    let started = MatchId {
        seed: snapshot.seed(),
        millis: snapshot.match_millis,
    };
    let identity = Identity {
        owner_id: owner_hex(&snapshot.owner_id),
        owner: snapshot.owner_id,
        match_id: started.to_string(),
        match_millis: started.millis,
        seed: snapshot.seed(),
    };
    // The managers, lineups, benches, tactics, and substitutions come from the snapshot.
    let mut sim = Simulation::from_snapshot(config.clone(), &snapshot)?;
    loaded.attach(&mut sim);
    let hello = hello_for(
        &sim,
        &identity,
        &config,
        loaded.content.tuning.stream.keyframe_interval,
    );
    tracing::info!(
        signal = "match.resumed",
        match.id = %identity.match_id,
        snapshot.tick = sim.tick(),
        home.score = sim.summary().goals[0],
        away.score = sim.summary().goals[1],
        build.hash = engine::build_hash()
    );
    Ok(Opened {
        identity,
        resume_tick: Some(sim.tick()),
        sim: Some(sim),
        config,
        hello,
    })
}

/// The hello for one match. A reconnecting page receives the same one, so it can tell by
/// `match.id` that it is the match it was watching.
fn hello_for(
    sim: &Simulation,
    identity: &Identity,
    config: &MatchConfig,
    keyframe_interval: u32,
) -> Hello {
    Hello {
        protocol_version: PROTOCOL_VERSION,
        engine_version: engine::version().to_string(),
        build_hash: engine::build_hash().to_string(),
        owner_id: identity.owner_id.clone(),
        match_id: identity.match_id.clone(),
        seed: identity.seed,
        dt_ms: config.tuning.dt * 1000.0,
        ticks_expected: config.max_ticks(),
        keyframe_interval,
        teams: hello_teams(sim, true),
        tactics: hello_tactics(sim),
        substitutions: hello_substitutions(sim),
    }
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
