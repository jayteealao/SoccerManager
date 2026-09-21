//! `engine-cli bench`: time whole matches on one thread and print one run-report record.

use std::process::Command;
use std::time::Instant;

use engine::observe::{RunReport, emit_line, machine_hash, process};
use engine::{MatchConfig, NullSink, Simulation, ticks_for_minutes};

use crate::cli::BenchOpts;

/// Wall-time budget for one 90-minute match on one thread, in milliseconds (NFR-1).
pub const BUDGET_MATCH_WALL_MS: u64 = 2000;

pub fn run(opts: &BenchOpts) -> anyhow::Result<i32> {
    if opts.matches == 0 {
        anyhow::bail!("--matches must be at least 1");
    }
    let ticks = ticks_for_minutes(opts.minutes);
    let config = MatchConfig::new(opts.seed, opts.minutes)?;

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
    let ticks_per_s = f64::from(ticks) / (median_ms.max(1) as f64 / 1000.0);
    let budget_pass = median_ms <= BUDGET_MATCH_WALL_MS;

    let report = RunReport {
        run_id: format!("bench-{}", engine::observe::unix_millis()),
        seed: opts.seed,
        outcome: "success",
        matches: opts.matches,
        match_wall_ms: median_ms,
        cpu_ms,
        peak_mem_mb: process::peak_memory_mb(),
        ticks_per_s,
        cpu_wall_ratio,
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
