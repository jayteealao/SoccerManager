//! `engine-cli simulate`: run one match to a tick file, save `stats.json` under the runtime
//! data folder, and print one match-stats record.

use std::path::Path;
use std::time::Instant;

use anyhow::Context;
use engine::observe::identity::{MatchId, data_dir, load_or_create_owner_id, owner_bytes};
use engine::observe::{MatchStats, TeamRef, emit_line, write_stats};
use engine::{
    FileSink, MatchConfig, Simulation, TickHeader, Validator, read_ticks, ticks_for_minutes,
};
use tracing::info_span;

use crate::cli::SimulateOpts;

pub fn run(content_dir: Option<&Path>, opts: &SimulateOpts) -> anyhow::Result<i32> {
    let span = info_span!("simulate", seed = opts.seed, minutes = opts.minutes);
    let _guard = span.enter();
    let loaded = crate::content::load(content_dir, opts.team_a.as_deref(), opts.team_b.as_deref())?;
    let [team_a, team_b] = &loaded.teams;
    let config = MatchConfig::new(opts.seed, opts.minutes, &loaded.content, [team_a, team_b])?;
    let data = data_dir();
    let owner_id = load_or_create_owner_id(&data)?;
    let match_id = MatchId::now(opts.seed);
    let ticks = ticks_for_minutes(opts.minutes);
    let header = TickHeader {
        seed: opts.seed,
        dt: config.tuning.dt,
        expected_ticks: ticks,
        owner_id: owner_bytes(&owner_id)?,
        match_millis: match_id.millis,
    };
    let teams = [
        TeamRef {
            id: config.teams[0].club_id.clone(),
            name: config.teams[0].name.clone(),
        },
        TeamRef {
            id: config.teams[1].club_id.clone(),
            name: config.teams[1].name.clone(),
        },
    ];
    let content_hash = config.content_hash.clone();
    let started = Instant::now();
    let mut sim = Simulation::new(config)?;
    let mut sink = FileSink::create(&opts.ticks_out, &header)
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
        owner_id,
        match_id: match_id.to_string(),
        seed: opts.seed,
        content_hash,
        teams,
        duration_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        outcome: "success".into(),
        ticks_per_s: f64::from(written) / elapsed.as_secs_f64().max(1e-9),
        ticks_written: written,
        validate_ran: true,
        validate_violations: violations.len(),
        possession_changes: summary.possession_changes,
        ball_max_speed: summary.ball_max_speed,
        ball_idle_ticks: summary.ball_idle_ticks,
        goals: summary.goals,
    };
    write_stats(&data, &stats)?;
    emit_line(&stats)?;
    Ok(0)
}
