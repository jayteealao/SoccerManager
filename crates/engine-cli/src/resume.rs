//! `engine-cli resume`: continue a match from its latest snapshot to full time, and print one
//! match-stats record. A snapshot resumes only on the build and the content that wrote it.

use std::path::Path;
use std::time::Instant;

use anyhow::Context;
use engine::observe::identity::{MatchId, data_dir, owner_hex};
use engine::observe::{LawStats, MatchStats, TacticsStats, TeamRef, emit_line, write_stats};
use engine::{
    EngineError, FanoutSink, FileSink, MatchConfig, Simulation, Snapshot, SnapshotSink, TickHeader,
    Validator, read_ticks,
};
use tracing::info_span;

use crate::cli::ResumeOpts;

pub fn run(content_dir: Option<&Path>, opts: &ResumeOpts) -> anyhow::Result<i32> {
    let shown = opts.snapshot.display().to_string();
    let snapshot = match Snapshot::read(&opts.snapshot, &shown) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            eprintln!("error: {err}");
            return Ok(1);
        }
    };
    let span = info_span!("resume", seed = snapshot.seed(), tick = snapshot.tick());
    let _guard = span.enter();
    let loaded = crate::content::load(content_dir, opts.team_a.as_deref(), opts.team_b.as_deref())?;
    let [team_a, team_b] = &loaded.teams;
    let config = MatchConfig::new(
        snapshot.seed(),
        snapshot.minutes(),
        &loaded.content,
        [team_a, team_b],
    )?;
    let match_id = MatchId {
        seed: snapshot.seed(),
        millis: snapshot.match_millis,
    };
    let owner = snapshot.owner_id;
    let pack_version = config.rules.schema_version;
    let header = TickHeader {
        seed: snapshot.seed(),
        dt: config.tuning.dt,
        expected_ticks: config.max_ticks(),
        owner_id: owner,
        match_millis: snapshot.match_millis,
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
    // The managers, lineups, benches, tactics, and substitutions used come from the snapshot,
    // not from a fresh pre-match setup.
    let mut sim = match Simulation::from_snapshot(config, &snapshot) {
        Ok(sim) => sim,
        Err(err) => {
            if let EngineError::Snapshot { reason, .. } = &err {
                let data_dir = std::env::var_os(engine::observe::identity::DATA_DIR_ENV)
                    .map(std::path::PathBuf::from);
                tracing::error!(
                    signal = "snapshot.refused",
                    path = %engine::snapshot::shorten_for_log(&shown, data_dir.as_deref()),
                    reason = %reason
                );
            }
            eprintln!("error: {err}");
            return Ok(1);
        }
    };
    let summary = sim.summary();
    let on_pitch = |team: usize| {
        sim.players()
            .iter()
            .filter(|p| p.team == team && p.active())
            .count()
    };
    tracing::info!(
        signal = "match.resumed",
        match.id = %match_id,
        snapshot.tick = sim.tick(),
        half = sim.half(),
        home.score = summary.goals[0],
        away.score = summary.goals[1],
        home.on_pitch = on_pitch(0),
        away.on_pitch = on_pitch(1),
        home.yellow = summary.yellow[0],
        away.yellow = summary.yellow[1],
        home.red = summary.red[0],
        away.red = summary.red[1],
        build.hash = engine::build_hash()
    );

    let data = data_dir();
    let snapshots = SnapshotSink::new(&data, &match_id.to_string(), owner, match_id.millis);
    let file = match opts.ticks_out.as_deref() {
        Some(path) => Some(
            FileSink::create(path, &header)
                .with_context(|| format!("cannot create {}", path.display()))?,
        ),
        None => None,
    };
    let resumed_at = sim.tick();
    let started = Instant::now();
    let mut sink = FanoutSink::new(file, snapshots);
    sim.run(&mut sink)?;
    let elapsed = started.elapsed();
    let (file, snapshots) = sink.into_parts();
    let written = match file {
        Some(file) => file.finish()?,
        None => sim.tick() - resumed_at,
    };
    let events = sim.take_events();

    let mut violations = 0;
    if let Some(path) = opts.ticks_out.as_deref() {
        let records = read_ticks(path)?;
        if opts.json {
            engine::record::write_jsonl(&path.with_extension("jsonl"), &records.records)?;
        }
        let validator = Validator::for_match(sim.tuning().clone(), sim.team_timeline(), &events);
        violations = validator.check(&records.records).len();
    }

    let summary = sim.summary();
    let stats = MatchStats {
        owner_id: owner_hex(&owner),
        match_id: match_id.to_string(),
        seed: snapshot.seed(),
        content_hash,
        teams,
        duration_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        outcome: "success".into(),
        ticks_per_s: f64::from(written) / elapsed.as_secs_f64().max(1e-9),
        ticks_written: written,
        validate_ran: opts.ticks_out.is_some(),
        validate_violations: violations,
        possession_changes: summary.possession_changes,
        ball_max_speed: summary.ball_max_speed,
        ball_idle_ticks: summary.ball_idle_ticks,
        goals: summary.goals,
        laws: LawStats::new(&summary, pack_version, sim.tick(), snapshots.writes),
        tactics: TacticsStats::new(&sim),
    };
    write_stats(&data, &stats)?;
    emit_line(&stats)?;
    Ok(0)
}
