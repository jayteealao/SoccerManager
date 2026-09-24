//! One calibration worker: the same binary started by the parent with hidden flags. It
//! plays its share of one suite's fixtures, each with the AI manager on both sides, and
//! writes one statistics file and one event file per match into the run folder. In the
//! formations suite, each side starts in its pairing's formation and keeps it. In the
//! red-card suite, the default clubs play with cards otherwise off, in both home and away
//! orders, and the arm's away player is sent off at kick-off.

use std::path::Path;
use std::time::Instant;

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::observe::identity::{data_dir, load_or_create_owner_id};
use engine::observe::{
    LawStats, MatchFigures, MatchStats, ScriptFigures, TacticsStats, TeamRef, write_stats_at,
};
use engine::{
    Commentary, Commentator, Content, ContentDir, EngineError, MatchConfig, Simulation, Tactics,
    Validator, VecSink,
};
use stream::EventWriter;

use super::fixtures::{self, Fixture, Leagues};
use crate::cli::InjectFailure;
use crate::report::bands::Bands;
use crate::report::{RED_CARD_ARMS, Suite};
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
    /// The flag states the parent resolved for this arm.
    pub states: &'a engine::FlagStates,
    /// A test seam: make this worker's first match or the worker itself fail.
    pub inject: Option<InjectFailure>,
    /// The formations suite: the numbers of the pairings to play; empty plays every one.
    pub pairings: &'a [usize],
}

/// One match of a worker's share.
struct Job {
    fixture: Fixture,
    /// The formations suite: each side's starting formation.
    formations: Option<[u8; 2]>,
    /// The red-card suite: the engine seed, the away player sent off (none for the
    /// control), and whether the default clubs play in the other order.
    red_card: Option<(u64, Option<usize>, bool)>,
}

/// Plays every fixture of the share. A failed match still writes its statistics record, with
/// `outcome` `error` and the `error.*` keys, and the worker goes on with the next fixture.
pub fn run(share: &Share<'_>) -> anyhow::Result<i32> {
    if share.inject == Some(InjectFailure::Worker) {
        tracing::error!(signal = "calibrate.injected_failure", shard = share.shard);
        return Ok(1);
    }
    let dir = ContentDir::at(share.content_dir);
    let content = Content::load(&dir)?.with_flags(share.states)?;
    let commentary = Commentary::load(&dir)?;
    let bands = Bands::load(&dir)?;
    let owner_id = load_or_create_owner_id(&data_dir())?;
    let stats_dir = share.run_dir.join("stats");
    let events_dir = share.run_dir.join("events");
    let mut leagues = Leagues::new(share.seed, &content);
    // The red-card suite plays the default clubs, loaded once.
    let defaults: Option<[TeamFile; 2]> = if share.suite == Suite::RedCard {
        Some([
            content.load_team(&dir, &dir.path(TEAM_A_FILE))?.value,
            content.load_team(&dir, &dir.path(TEAM_B_FILE))?.value,
        ])
    } else {
        None
    };
    let shards = share.shards.max(1);
    let mut inject_match = share.inject == Some(InjectFailure::Match);
    for job in planned(share, &content)
        .into_iter()
        .filter(|j| j.fixture.index % shards == share.shard)
    {
        let fixture = job.fixture;
        let match_id = fixtures::match_id(share.seed, share.suite, fixture.index, share.run_millis);
        let (teams, seed) = match (&defaults, job.red_card) {
            (Some([first, second]), Some((engine_seed, _, swapped))) => {
                let teams = if swapped {
                    [second.clone(), first.clone()]
                } else {
                    [first.clone(), second.clone()]
                };
                (teams, engine_seed)
            }
            _ => {
                let mut teams = leagues.teams(&fixture);
                if share.suite == Suite::Strength {
                    let side = fixture.boosted_side();
                    teams[side] =
                        fixtures::boosted(&teams[side], bands.stronger_team.attribute_boost);
                }
                (
                    teams,
                    fixtures::match_seed(share.seed, share.suite, fixture.index),
                )
            }
        };
        let played = if std::mem::take(&mut inject_match) {
            Err(EngineError::InvalidConfig("injected failure".into()))
        } else {
            play(
                share,
                (&content, &commentary),
                (&teams, job.formations, job.red_card.map(|(_, off, _)| off)),
                seed,
                &owner_id,
                &match_id,
                &events_dir,
            )
        };
        let stats = match played {
            Ok(stats) => stats,
            Err(err) => {
                tracing::error!(
                    signal = "calibrate.match_failed",
                    match.id = %match_id,
                    error = %err
                );
                failure(
                    (&owner_id, &match_id, seed),
                    &teams,
                    &fixture,
                    &err,
                    &content,
                )
            }
        };
        write_stats_at(&stats_dir, &stats)?;
    }
    Ok(0)
}

/// Every fixture of the share's suite, with the formation of each side in the formations
/// suite and the arm in the red-card suite.
fn planned(share: &Share<'_>, content: &Content) -> Vec<Job> {
    let matches = share.matches;
    match share.suite {
        Suite::Formations => {
            let pairings = fixtures::pairings(content.tactics.formations.len());
            let selected: Vec<usize> = if share.pairings.is_empty() {
                (0..pairings.len()).collect()
            } else {
                share.pairings.to_vec()
            };
            fixtures::formation_fixtures_for(matches, pairings.len(), &selected)
                .into_iter()
                .map(|f| {
                    // A tactics file holds at most 16 formations.
                    let [first, second] = pairings[f.pairing].map(|i| i as u8);
                    let sides = if f.first_side == 0 {
                        [first, second]
                    } else {
                        [second, first]
                    };
                    Job {
                        fixture: f.fixture,
                        formations: Some(sides),
                        red_card: None,
                    }
                })
                .collect()
        }
        Suite::RedCard => fixtures::red_card_fixtures(share.seed, matches)
            .into_iter()
            .map(|f| Job {
                // The default clubs play every red-card match; no league is generated.
                fixture: Fixture {
                    index: f.index,
                    league: 0,
                    clubs: if f.swapped { [1, 0] } else { [0, 1] },
                },
                formations: None,
                red_card: Some((f.engine_seed, RED_CARD_ARMS[f.arm].1, f.swapped)),
            })
            .collect(),
        Suite::Equal | Suite::Strength => fixtures::fixtures(matches)
            .into_iter()
            .map(|fixture| Job {
                fixture,
                formations: None,
                red_card: None,
            })
            .collect(),
    }
}

/// One match to full time, validated, with its events and their commentary lines written.
/// `formations`, when present, is each side's starting formation. `red_card`, when present,
/// plays the match with cards otherwise off and sends the named player off at kick-off,
/// as the slow test `a_sending_off_gives_no_advantage` does.
fn play(
    share: &Share<'_>,
    (content, commentary): (&Content, &Commentary),
    (teams, formations, red_card): (&[TeamFile; 2], Option<[u8; 2]>, Option<Option<usize>>),
    seed: u64,
    owner_id: &str,
    match_id: &str,
    events_dir: &Path,
) -> Result<MatchStats, EngineError> {
    let mut config = MatchConfig::new(seed, share.minutes, content, [&teams[0], &teams[1]])?;
    if let Some(sides) = formations {
        for (team, formation) in sides.into_iter().enumerate() {
            let mut tactics = Tactics::defaults(&content.tactics);
            tactics.set_formation(formation, &content.tactics);
            config = config.with_tactics(team, tactics);
        }
    }
    if red_card.is_some() {
        // Set on the match, not in the content, so the content hash is unchanged.
        config.tuning.red_base = 0.0;
        config.tuning.yellow_base = 0.0;
        config.tuning.yellow_aggression_weight = 0.0;
    }
    let pack_version = config.rules.schema_version;
    let refs = team_refs(teams);
    let content_hash = config.content_hash.clone();
    let max_ticks = config.max_ticks() as usize;
    let mut sim = Simulation::new(config)?;
    if let Some(Some(player)) = red_card {
        sim.send_off_before_kickoff(player);
    }
    let mut ids = Ids::new(&sim);
    let mut commentator = Commentator::for_match(commentary, &sim);
    // The validator reads the whole match once it is over, so the records are kept; sized
    // up front, the buffer never doubles, and the timed run pays for no copies.
    let mut sink = VecSink {
        records: Vec::with_capacity(max_ticks),
    };
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
            .write(&row.commentary(commentator.line(event)))
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
        flags_on: sim.config().flags.names().to_vec(),
        laws: LawStats::new(&summary, pack_version, sim.tick(), 0),
        tactics: TacticsStats::new(&sim),
        figures: MatchFigures::new(&summary, sim.managers()),
        script: ScriptFigures::new(sim.plugins()),
    })
}

fn team_refs(teams: &[TeamFile; 2]) -> [TeamRef; 2] {
    [0, 1].map(|t| TeamRef {
        id: teams[t].club.id.clone(),
        name: teams[t].club.name.clone(),
    })
}

/// The statistics record of a match the engine could not play. The figures are zero; the
/// rule pack version and the managers are the ones the match would have used.
fn failure(
    (owner_id, match_id, seed): (&str, &str, u64),
    teams: &[TeamFile; 2],
    fixture: &Fixture,
    err: &EngineError,
    content: &Content,
) -> MatchStats {
    tracing::debug!(fixture = fixture.index, "recording a failed match");
    MatchStats {
        owner_id: owner_id.to_string(),
        match_id: match_id.to_string(),
        seed,
        content_hash: String::new(),
        teams: team_refs(teams),
        duration_ms: 0,
        outcome: "error".into(),
        ticks_per_s: 0.0,
        ticks_written: 0,
        validate_ran: false,
        validate_violations: 0,
        possession_changes: 0,
        ball_max_speed: 0.0,
        ball_idle_ticks: 0,
        goals: [0, 0],
        flags_on: content.flags.names().to_vec(),
        laws: LawStats {
            pack_version: content.rules.schema_version,
            ..LawStats::default()
        },
        tactics: TacticsStats::default(),
        figures: MatchFigures {
            // Calibration plays the AI manager on both sides.
            manager_kind: ["ai".to_string(), "ai".to_string()],
            error_type: Some(err.error_type().to_string()),
            error_code: Some(err.error_code().to_string()),
            error_retriable: Some(err.retriable()),
            ..MatchFigures::default()
        },
        script: ScriptFigures::default(),
    }
}
