//! `engine-cli simulate`: run one match to a tick file and print one match-stats record.

use std::time::Instant;

use anyhow::Context;
use engine::observe::{MatchStats, emit_line};
use engine::{FileSink, MatchConfig, Simulation, Validator, read_ticks, ticks_for_minutes};
use tracing::info_span;

use crate::cli::SimulateOpts;

pub fn run(opts: &SimulateOpts) -> anyhow::Result<i32> {
    let span = info_span!("simulate", seed = opts.seed, minutes = opts.minutes);
    let _guard = span.enter();
    let config = MatchConfig::new(opts.seed, opts.minutes)?;
    let ticks = ticks_for_minutes(opts.minutes);
    let started = Instant::now();
    let mut sim = Simulation::new(config)?;
    let mut sink = FileSink::create(&opts.ticks_out, opts.seed, sim.tuning().dt, ticks)
        .with_context(|| format!("cannot create {}", opts.ticks_out.display()))?;
    sim.run(ticks, &mut sink)?;
    let written = sink.finish()?;
    let elapsed = started.elapsed();

    if opts.json {
        let json_path = opts.ticks_out.with_extension("jsonl");
        let records = read_ticks(&opts.ticks_out)?;
        engine::record::write_jsonl(&json_path, &records.records)?;
    }

    let file = read_ticks(&opts.ticks_out)?;
    let validator = Validator::new(sim.tuning().clone(), sim.teams());
    let violations = validator.check(&file.records);
    let summary = sim.summary();
    let stats = MatchStats {
        match_id: engine::observe::match_id(opts.seed),
        seed: opts.seed,
        duration_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        outcome: "success",
        ticks_per_s: f64::from(written) / elapsed.as_secs_f64().max(1e-9),
        ticks_written: written,
        validate_ran: true,
        validate_violations: violations.len(),
        possession_changes: summary.possession_changes,
        ball_max_speed: summary.ball_max_speed,
        ball_idle_ticks: summary.ball_idle_ticks,
        goals: summary.goals,
    };
    emit_line(&stats)?;
    Ok(0)
}
