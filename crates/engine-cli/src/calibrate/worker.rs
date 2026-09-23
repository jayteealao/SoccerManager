//! One calibration worker: the same binary started by the parent with hidden flags. It
//! plays its share of one suite's fixtures, each with the AI manager on both sides, and
//! writes one statistics file and one event file per match into the run folder.

use std::path::Path;
use std::time::Instant;

use engine::observe::identity::{data_dir, load_or_create_owner_id};
use engine::observe::{LawStats, MatchFigures, MatchStats, TacticsStats, TeamRef, write_stats_at};
use engine::{Content, ContentDir, EngineError, MatchConfig, Simulation, Validator, VecSink};
use stream::EventWriter;

use super::fixtures::{self, Fixture, Leagues};
use crate::report::Suite;
use crate::report::bands::Bands;
use crate::stream_run::{Ids, match_event};

/// What one worker plays.
pub struct Share<'a> {
    pub content_dir: &'a Path,
    pub run_dir: &'a Path,
    pub seed: u64,
    pub matches: u32,
    pub minutes: u32,
    pub suite: Suite,
    pub shard: u32,
    pub shards: u32,
    pub run_millis: u64,
}

/// Plays every fixture of the share. A failed match still writes its statistics record, with
/// `outcome` `failure` and `error.type`, and the worker goes on with the next fixture.
pub fn run(share: &Share<'_>) -> anyhow::Result<i32> {
    let dir = ContentDir::at(share.content_dir);
    let content = Content::load(&dir)?;
    let bands = Bands::load(&dir)?;
    let owner_id = load_or_create_owner_id(&data_dir())?;
    let stats_dir = share.run_dir.join("stats");
    let events_dir = share.run_dir.join("events");
    let mut leagues = Leagues::new(share.seed, &content);
    let shards = share.shards.max(1);
    for fixture in fixtures::fixtures(share.matches)
        .into_iter()
        .filter(|f| f.index % shards == share.shard)
    {
        let match_id = fixtures::match_id(share.seed, share.suite, fixture.index, share.run_millis);
        let mut teams = leagues.teams(&fixture);
        if share.suite == Suite::Strength {
            let side = fixture.boosted_side();
            teams[side] = fixtures::boosted(&teams[side], bands.stronger_team.attribute_boost);
        }
        let seed = fixtures::match_seed(share.seed, share.suite, fixture.index);
        let stats = match play(
            share,
            &content,
            &teams,
            seed,
            &owner_id,
            &match_id,
            &events_dir,
        ) {
            Ok(stats) => stats,
            Err(err) => {
                tracing::error!(
                    signal = "calibrate.match_failed",
                    match.id = %match_id,
                    error = %err
                );
                failure(&owner_id, &match_id, seed, &teams, &fixture, &err)
            }
        };
        write_stats_at(&stats_dir, &stats)?;
    }
    Ok(0)
}

/// One match to full time, validated, with its events written.
fn play(
    share: &Share<'_>,
    content: &Content,
    teams: &[engine::data::TeamFile; 2],
    seed: u64,
    owner_id: &str,
    match_id: &str,
    events_dir: &Path,
) -> Result<MatchStats, EngineError> {
    let config = MatchConfig::new(seed, share.minutes, content, [&teams[0], &teams[1]])?;
    let pack_version = config.rules.schema_version;
    let refs = team_refs(teams);
    let content_hash = config.content_hash.clone();
    let mut sim = Simulation::new(config)?;
    let mut ids = Ids::new(&sim);
    let mut sink = VecSink::default();
    let started = Instant::now();
    sim.run(&mut sink)?;
    let elapsed = started.elapsed();
    let events = sim.take_events();
    let mut writer = EventWriter::create_at(&events_dir.join(format!("{match_id}.jsonl")))
        .map_err(|e| EngineError::Sink(e.to_string()))?;
    for event in &events {
        let row = match_event(
            event,
            owner_id,
            match_id,
            [&refs[0].id, &refs[1].id],
            &mut ids,
        );
        writer
            .write(&row)
            .map_err(|e| EngineError::Sink(e.to_string()))?;
    }
    let validator = Validator::for_match(sim.tuning().clone(), sim.team_timeline(), &events);
    let violations = validator.check(&sink.records).len();
    let summary = sim.summary();
    let written = u32::try_from(sink.records.len()).unwrap_or(u32::MAX);
    Ok(MatchStats {
        owner_id: owner_id.to_string(),
        match_id: match_id.to_string(),
        seed,
        content_hash,
        teams: refs,
        duration_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        outcome: "success".into(),
        ticks_per_s: f64::from(written) / elapsed.as_secs_f64().max(1e-9),
        ticks_written: written,
        validate_ran: true,
        validate_violations: violations,
        possession_changes: summary.possession_changes,
        ball_max_speed: summary.ball_max_speed,
        ball_idle_ticks: summary.ball_idle_ticks,
        goals: summary.goals,
        laws: LawStats::new(&summary, pack_version, sim.tick(), 0),
        tactics: TacticsStats::new(&sim),
        figures: MatchFigures::new(&summary, sim.managers()),
    })
}

fn team_refs(teams: &[engine::data::TeamFile; 2]) -> [TeamRef; 2] {
    [0, 1].map(|t| TeamRef {
        id: teams[t].club.id.clone(),
        name: teams[t].club.name.clone(),
    })
}

/// The statistics record of a match the engine could not play.
fn failure(
    owner_id: &str,
    match_id: &str,
    seed: u64,
    teams: &[engine::data::TeamFile; 2],
    fixture: &Fixture,
    err: &EngineError,
) -> MatchStats {
    tracing::debug!(fixture = fixture.index, "recording a failed match");
    MatchStats {
        owner_id: owner_id.to_string(),
        match_id: match_id.to_string(),
        seed,
        content_hash: String::new(),
        teams: team_refs(teams),
        duration_ms: 0,
        outcome: "failure".into(),
        ticks_per_s: 0.0,
        ticks_written: 0,
        validate_ran: false,
        validate_violations: 0,
        possession_changes: 0,
        ball_max_speed: 0.0,
        ball_idle_ticks: 0,
        goals: [0, 0],
        laws: LawStats::default(),
        tactics: TacticsStats::default(),
        figures: MatchFigures {
            error_type: Some(error_type(err).to_string()),
            ..MatchFigures::default()
        },
    }
}

/// The error's kind as `error.type` names it.
fn error_type(err: &EngineError) -> &'static str {
    match err {
        EngineError::InvalidConfig(_) => "invalid-config",
        EngineError::Io(_) | EngineError::Read { .. } => "io",
        EngineError::Format(_) => "format",
        EngineError::Sink(_) => "sink",
        EngineError::Data { .. } | EngineError::Version { .. } => "content",
        EngineError::Snapshot { .. } => "snapshot",
    }
}
