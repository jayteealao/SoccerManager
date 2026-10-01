//! `engine-cli bench`: time whole matches on one thread and print one run-report record.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use engine::observe::identity::{MatchId, data_dir, load_or_create_owner_id};
use engine::observe::{RunReport, emit_line, machine_hash, process};
use engine::{EngineError, MatchConfig, NullSink, Simulation};
use protocol::{Hello, PROTOCOL_VERSION, Queue, ServerMessage};
use stream::session::{MatchState, SessionConfig};
use stream::{Client, CommandContext, Gate, Incoming, Server, Session};

use crate::cli::BenchOpts;
use crate::stream_run::{Drive, drive, hello_substitutions, hello_tactics, hello_teams};

/// Wall-time budget for one 90-minute match on one thread, in milliseconds (NFR-1).
pub const BUDGET_MATCH_WALL_MS: u64 = 2000;

pub fn run(content_dir: Option<&Path>, opts: &BenchOpts) -> anyhow::Result<i32> {
    if opts.matches == 0 {
        return Err(EngineError::InvalidConfig("--matches must be at least 1".into()).into());
    }
    // Content, teams, and identity load once here, before the warm-up, so nothing on the
    // timed path reads a file or hashes bytes.
    let loaded = crate::content::load(content_dir, None, None, opts.script_pack.as_deref())?;
    let [team_a, team_b] = &loaded.teams;
    let mut config = MatchConfig::new(opts.seed, opts.minutes, &loaded.content, [team_a, team_b])?;
    if opts.knockout {
        config = config.with_knockout();
    }
    loaded.fold(&mut config);
    let owner_id = load_or_create_owner_id(&data_dir())?;
    let ticks = config.max_ticks();

    let figures = measure(&config, loaded.script.as_ref(), opts.matches)?;
    let stream = if opts.stream {
        Some(measure_stream(&config, &loaded, ticks, opts.seed)?)
    } else {
        None
    };
    let median_ms = figures.match_wall_ms;
    let ticks_per_s = f64::from(figures.ticks_per_match) / (median_ms.max(1) as f64 / 1000.0);
    let budget_pass = median_ms <= BUDGET_MATCH_WALL_MS;

    let report = RunReport {
        owner_id,
        run_id: format!("bench-{}", engine::observe::unix_millis()),
        seed: opts.seed,
        content_hash: config.content_hash.clone(),
        outcome: "success",
        matches: opts.matches,
        match_wall_ms: median_ms,
        ticks_per_match: figures.ticks_per_match,
        cpu_us_per_tick: figures.cpu_us_per_tick,
        cpu_ms: figures.cpu_ms,
        // Read after the streamed match, as the process peak of the whole run.
        peak_mem_mb: process::peak_memory_mb(),
        ticks_per_s,
        cpu_wall_ratio: figures.cpu_wall_ratio,
        stream_ticks_per_s: stream.map(|(rate, _)| rate),
        stream_pauses: stream.map(|(_, pauses)| pauses),
        machine_hash: machine_hash(),
        cpu_model: cpu_model(),
        power_plan: power_plan(),
        budget_pass,
        script_pack: loaded.script.as_ref().map(script::LoadedPack::identity),
    };
    emit_line(&report)?;
    Ok(if budget_pass { 0 } else { 2 })
}

/// What `measure` timed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BenchFigures {
    /// The median wall time of one match, in milliseconds.
    pub match_wall_ms: u64,
    /// Ticks in the median match: added time makes the length vary.
    pub ticks_per_match: u32,
    pub cpu_ms: Option<u64>,
    /// Processor time per simulated tick over the timed matches, in microseconds.
    pub cpu_us_per_tick: Option<f64>,
    pub cpu_wall_ratio: Option<f64>,
    pub peak_mem_mb: Option<f64>,
}

/// Plays one discarded warm-up match, then times `matches` whole matches on this thread.
/// No snapshot sink is attached: the benchmark times the engine. With a script pack, every
/// match runs with fresh hooks from it.
pub(crate) fn measure(
    config: &MatchConfig,
    script: Option<&script::LoadedPack>,
    matches: u32,
) -> anyhow::Result<BenchFigures> {
    run_one(config, script)?;
    let cpu_before = process::cpu_time_ms();
    let started = Instant::now();
    let mut samples_ms: Vec<u64> = Vec::with_capacity(matches as usize);
    let mut samples_ticks: Vec<u32> = Vec::with_capacity(matches as usize);
    for _ in 0..matches.max(1) {
        let (ms, played) = run_one(config, script)?;
        samples_ms.push(ms);
        samples_ticks.push(played);
    }
    let wall_total_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let cpu_after = process::cpu_time_ms();
    let total_ticks: u64 = samples_ticks.iter().map(|&t| u64::from(t)).sum();
    samples_ms.sort_unstable();
    samples_ticks.sort_unstable();
    let cpu_ms = match (cpu_before, cpu_after) {
        (Some(a), Some(b)) => Some(b.saturating_sub(a)),
        _ => None,
    };
    Ok(BenchFigures {
        match_wall_ms: samples_ms[samples_ms.len() / 2],
        ticks_per_match: samples_ticks[samples_ticks.len() / 2],
        cpu_ms,
        cpu_us_per_tick: cpu_ms.map(|c| round4(c as f64 * 1000.0 / total_ticks.max(1) as f64)),
        cpu_wall_ratio: cpu_ms.map(|c| c as f64 / wall_total_ms.max(1) as f64),
        peak_mem_mb: process::peak_memory_mb(),
    })
}

/// One whole match. Returns its wall time in milliseconds and the ticks it played.
fn run_one(
    config: &MatchConfig,
    script: Option<&script::LoadedPack>,
) -> anyhow::Result<(u64, u32)> {
    let mut sim = Simulation::new(config.clone())?;
    if let Some(pack) = script {
        sim.set_plugins(pack.plugins());
    }
    let mut sink = NullSink;
    let started = Instant::now();
    sim.run(&mut sink)?;
    let ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    Ok((ms, sim.tick()))
}

fn round4(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

/// One match streamed over the socket to a client that reads as fast as it can. Returns the
/// ticks delivered per second and how many times the producer paused at the buffer bound.
fn measure_stream(
    config: &MatchConfig,
    loaded: &crate::content::Loaded,
    ticks: u32,
    seed: u64,
) -> anyhow::Result<(f64, u32)> {
    let data = data_dir();
    let owner_id = load_or_create_owner_id(&data)?;
    let match_id = MatchId::now(seed).to_string();
    let club_ids = [
        config.teams[0].club_id.clone(),
        config.teams[1].club_id.clone(),
    ];
    let mut sim = Simulation::new(config.clone())?;
    loaded.attach(&mut sim);
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION,
        engine_version: engine::version().to_string(),
        build_hash: engine::build_hash().to_string(),
        owner_id: owner_id.clone(),
        match_id: match_id.clone(),
        seed,
        dt_ms: config.tuning.dt * 1000.0,
        ticks_expected: ticks,
        keyframe_interval: 50,
        teams: hello_teams(&sim, false),
        tactics: hello_tactics(&sim),
        substitutions: hello_substitutions(&sim),
        knockout: false,
        ground_length: config.pitch.length(),
        ground_width: config.pitch.width(),
    };

    let server = Server::bind(&data, &match_id)?;
    let port = server.port();
    let reader = std::thread::spawn(move || -> Result<u64, stream::StreamError> {
        let mut client = Client::connect_local(port)?;
        loop {
            match client.read()? {
                Incoming::Closed => return Ok(client.ticks()),
                _ => continue,
            }
        }
    });
    let connection = server.accept(&match_id)?;
    let events = stream::session::shared_events(&data, &match_id)?;
    let gate = Arc::new(Gate::new());
    let state = Arc::new(MatchState::default());
    let session = Session::start(
        connection,
        SessionConfig {
            buffer_ticks: 500,
            keyframe_interval: 50,
            hello,
            commands: CommandContext {
                owner_id: owner_id.clone(),
                match_id: match_id.clone(),
                gate: Arc::clone(&gate),
                state: Arc::clone(&state),
                events,
                queue: Queue::new(Vec::new()),
                pre_match: Arc::new(stream::PreMatch::none()),
                inbox: Arc::new(stream::Inbox::default()),
            },
            drop_at: None,
        },
    )?;
    let mut sink = session.sink();
    let started = Instant::now();
    drive(
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
            inbox: None,
            page_changes: None,
            planned: &[],
            observe: None,
        },
        &mut |message: ServerMessage| session.send(&message),
    )?;
    let elapsed = started.elapsed();
    let pauses = session.gauge().pauses();
    session.finish()?;
    let delivered = reader
        .join()
        .map_err(|_| anyhow::anyhow!("the benchmark client thread panicked"))??;
    Ok((delivered as f64 / elapsed.as_secs_f64().max(1e-9), pauses))
}

fn powershell(script: &str) -> Option<String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

pub(crate) fn cpu_model() -> String {
    powershell("(Get-CimInstance Win32_Processor).Name")
        .or_else(|| std::env::var("PROCESSOR_IDENTIFIER").ok())
        .unwrap_or_else(|| "unknown".to_string())
}

fn power_plan() -> String {
    powershell("powercfg /getactivescheme")
        .and_then(|line| {
            line.split('(')
                .nth(1)
                .map(|s| s.trim_end_matches(')').trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}
