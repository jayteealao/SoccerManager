//! `engine-cli bench`: time whole matches on one thread and print one run-report record.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use engine::observe::identity::{MatchId, data_dir, load_or_create_owner_id};
use engine::observe::{RunReport, emit_line, machine_hash, process};
use engine::{MatchConfig, NullSink, Simulation, ticks_for_minutes};
use protocol::{Hello, PROTOCOL_VERSION, Queue, ServerMessage, TeamRef};
use stream::session::{MatchState, SessionConfig};
use stream::{Client, CommandContext, Gate, Incoming, Server, Session};

use crate::cli::BenchOpts;
use crate::stream_run::{Drive, drive};

/// Wall-time budget for one 90-minute match on one thread, in milliseconds (NFR-1).
pub const BUDGET_MATCH_WALL_MS: u64 = 2000;

pub fn run(content_dir: Option<&Path>, opts: &BenchOpts) -> anyhow::Result<i32> {
    if opts.matches == 0 {
        anyhow::bail!("--matches must be at least 1");
    }
    let ticks = ticks_for_minutes(opts.minutes);
    // Content, teams, and identity load once here, before the warm-up, so nothing on the
    // timed path reads a file or hashes bytes.
    let loaded = crate::content::load(content_dir, None, None)?;
    let [team_a, team_b] = &loaded.teams;
    let config = MatchConfig::new(opts.seed, opts.minutes, &loaded.content, [team_a, team_b])?;
    let owner_id = load_or_create_owner_id(&data_dir())?;

    // Warm-up run, discarded.
    run_one(&config, ticks)?;

    let cpu_before = process::cpu_time_ms();
    let started = Instant::now();
    let mut samples_ms: Vec<u64> = Vec::with_capacity(opts.matches as usize);
    for _ in 0..opts.matches {
        samples_ms.push(run_one(&config, ticks)?);
    }
    let wall_total_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let cpu_after = process::cpu_time_ms();
    samples_ms.sort_unstable();
    let median_ms = samples_ms[samples_ms.len() / 2];
    let cpu_ms = match (cpu_before, cpu_after) {
        (Some(a), Some(b)) => Some(b.saturating_sub(a)),
        _ => None,
    };
    let cpu_wall_ratio = cpu_ms.map(|c| c as f64 / wall_total_ms.max(1) as f64);
    let stream = if opts.stream {
        Some(measure_stream(&config, ticks, opts.seed)?)
    } else {
        None
    };
    let ticks_per_s = f64::from(ticks) / (median_ms.max(1) as f64 / 1000.0);
    let budget_pass = median_ms <= BUDGET_MATCH_WALL_MS;

    let report = RunReport {
        owner_id,
        run_id: format!("bench-{}", engine::observe::unix_millis()),
        seed: opts.seed,
        content_hash: config.content_hash.clone(),
        outcome: "success",
        matches: opts.matches,
        match_wall_ms: median_ms,
        cpu_ms,
        peak_mem_mb: process::peak_memory_mb(),
        ticks_per_s,
        cpu_wall_ratio,
        stream_ticks_per_s: stream.map(|(rate, _)| rate),
        stream_pauses: stream.map(|(_, pauses)| pauses),
        machine_hash: machine_hash(),
        cpu_model: cpu_model(),
        power_plan: power_plan(),
        budget_pass,
    };
    emit_line(&report)?;
    Ok(if budget_pass { 0 } else { 2 })
}

fn run_one(config: &MatchConfig, ticks: u32) -> anyhow::Result<u64> {
    let mut sim = Simulation::new(config.clone())?;
    let mut sink = NullSink;
    let started = Instant::now();
    sim.run(ticks, &mut sink)?;
    Ok(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX))
}

/// One match streamed over the socket to a client that reads as fast as it can. Returns the
/// ticks delivered per second and how many times the producer paused at the buffer bound.
fn measure_stream(config: &MatchConfig, ticks: u32, seed: u64) -> anyhow::Result<(f64, u32)> {
    let data = data_dir();
    let owner_id = load_or_create_owner_id(&data)?;
    let match_id = MatchId::now(seed).to_string();
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
        seed,
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
            },
        },
    )?;
    let mut sink = session.sink();
    let mut sim = Simulation::new(config.clone())?;
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

fn cpu_model() -> String {
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
