//! `engine-cli simulate`: run one match to a tick file, save `stats.json` and `events.jsonl`
//! under the runtime data folder, and print one match-stats record.

use std::path::Path;
use std::time::Instant;

use anyhow::Context;
use engine::observe::identity::{MatchId, data_dir, load_or_create_owner_id, owner_bytes};
use engine::observe::{
    LawStats, MatchFigures, MatchStats, ScriptFigures, TacticsStats, TeamRef, emit_line,
    write_stats,
};
use engine::{
    Commentator, FanoutSink, FileSink, MatchConfig, Simulation, SnapshotSink, TickHeader,
    Validator, read_ticks,
};
use tracing::info_span;

use stream::EventWriter;

use crate::cli::SimulateOpts;
use crate::stream_run::{Ids, rows};

pub fn run(content_dir: Option<&Path>, opts: &SimulateOpts) -> anyhow::Result<i32> {
    let span = info_span!("simulate", seed = opts.seed, minutes = opts.minutes);
    let _guard = span.enter();
    let loaded = crate::content::load(
        content_dir,
        opts.team_a.as_deref(),
        opts.team_b.as_deref(),
        opts.script_pack.as_deref(),
    )?;
    let [team_a, team_b] = &loaded.teams;
    // Both teams are managed by the AI manager (the configuration's default).
    let mut config = MatchConfig::new(opts.seed, opts.minutes, &loaded.content, [team_a, team_b])?;
    if opts.knockout {
        config = config.with_knockout();
    }
    loaded.fold(&mut config);
    let data = data_dir();
    let owner_id = load_or_create_owner_id(&data)?;
    let match_id = MatchId::now(opts.seed);
    let owner = owner_bytes(&owner_id)?;
    let pack_version = config.rules.schema_version;
    let header = TickHeader {
        seed: opts.seed,
        dt: config.tuning.dt,
        expected_ticks: config.max_ticks(),
        owner_id: owner,
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
    let club_ids = [teams[0].id.clone(), teams[1].id.clone()];
    let mut sim = Simulation::new(config)?;
    loaded.attach(&mut sim);
    let mut ids = Ids::new(&sim);
    let mut commentator = Commentator::for_match(&loaded.commentary, &sim);
    let file = FileSink::create(&opts.ticks_out, &header)
        .with_context(|| format!("cannot create {}", opts.ticks_out.display()))?;
    let snapshots = (!opts.no_snapshot)
        .then(|| SnapshotSink::new(&data, &match_id.to_string(), owner, match_id.millis));
    let mut sink = FanoutSink::new(file, snapshots);
    sim.run(&mut sink)?;
    let (file, snapshots) = sink.into_parts();
    let written = file.finish()?;
    let snapshot_writes = snapshots.map_or(0, |s| s.writes);
    let elapsed = started.elapsed();

    if opts.json {
        let json_path = opts.ticks_out.with_extension("jsonl");
        let records = read_ticks(&opts.ticks_out)?;
        engine::record::write_jsonl(&json_path, &records.records)?;
    }

    let file = read_ticks(&opts.ticks_out)?;
    let events = sim.take_events();
    let match_id_text = match_id.to_string();
    let mut writer = EventWriter::open(&data, &match_id_text)?;
    for event in &events {
        for row in rows(
            &mut sim,
            &mut commentator,
            event,
            &owner_id,
            &match_id_text,
            [&club_ids[0], &club_ids[1]],
            &mut ids,
        ) {
            writer.write(&row)?;
        }
    }
    let validator = Validator::for_match(sim.tuning().clone(), sim.team_timeline(), &events);
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
        flags_on: sim.config().flags.names().to_vec(),
        laws: LawStats::new(&summary, pack_version, written, snapshot_writes),
        tactics: TacticsStats::new(&sim),
        figures: MatchFigures::new(&summary, sim.managers()),
        script: ScriptFigures::new(sim.plugins()),
    };
    write_stats(&data, &stats)?;
    emit_line(&stats)?;
    Ok(0)
}
