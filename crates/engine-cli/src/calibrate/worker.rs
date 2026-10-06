//! One calibration worker of the old worker-process path (the hidden
//! `--worker-processes`): the same binary started by the parent with hidden flags. It plays
//! its share of one suite's fixtures, each with the AI manager on both sides, and writes
//! one statistics file and one event file per match into the run folder. The match itself,
//! [`play_match`], is the one both runners play. In the formations suite, each side starts
//! in its pairing's formation and keeps it. In the red-card suite, the default clubs play
//! with cards otherwise off, in both home and away orders, and the arm's away player is
//! sent off at kick-off.
//!
//! The fixtures earlier sessions of the run left unfinished are cut into work units of
//! [`run_folder::UNIT`] in fixture order; the worker plays the units whose place modulo the
//! number of workers is its shard, and appends one ledger line when a unit's last match is
//! written.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::observe::identity::{data_dir, load_or_create_owner_id};
use engine::observe::{
    LawStats, MatchFigures, MatchStats, ScriptFigures, TacticsStats, TeamRef, write_stats_at,
};
use engine::{
    Commentary, Commentator, Content, ContentDir, EngineError, MatchConfig, RunningCheck,
    Simulation, StreamRules, Tactics, TickRecord, TickSink,
};
use stream::EventWriter;

use super::fixtures::{self, FixtureKey, Keyed, Leagues};
use super::rows::Row;
use super::run_folder::{self, Done, LedgerWriter, UnitLine};
use super::stages::{self, Stage};
use crate::cli::InjectFailure;
use crate::report::bands::Registry;
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
    /// The parent's session: the ledger files it names are this session's own.
    pub session: &'a str,
    /// A test seam: units from this place on are not played, and the unit at this place
    /// plays its first half and writes no ledger line, as a run killed mid-unit leaves it.
    pub stop: Option<u32>,
}

/// The fixtures of `planned` that earlier sessions of the run in `run_dir` left
/// unfinished, in fixture order: the list the parent and every worker cut into the same
/// units.
pub fn remaining(run_dir: &Path, session: &str, planned: Vec<Keyed>) -> anyhow::Result<Vec<Keyed>> {
    let done = run_folder::finished(run_dir, Some(session))?;
    Ok(planned
        .into_iter()
        .filter(|k| !done.contains_key(&k.key))
        .collect())
}

/// The default clubs of `dir`, loaded once: the clubs of the red-card suite.
pub fn default_clubs(content: &Content, dir: &ContentDir) -> anyhow::Result<[TeamFile; 2]> {
    Ok([
        content.load_team(dir, &dir.path(TEAM_A_FILE))?.value,
        content.load_team(dir, &dir.path(TEAM_B_FILE))?.value,
    ])
}

/// Plays the share's units. A failed match still writes its statistics record, with
/// `outcome` `error` and the `error.*` keys, and the worker goes on with the next fixture.
pub fn run(share: &Share<'_>) -> anyhow::Result<i32> {
    if share.inject == Some(InjectFailure::Worker) {
        tracing::error!(signal = "calibrate.injected_failure", shard = share.shard);
        return Ok(1);
    }
    let dir = ContentDir::at(share.content_dir);
    let content = Content::load(&dir)?.with_flags(share.states)?;
    let commentary = Commentary::load(&dir)?;
    let bands = Registry::load(&dir)?;
    let owner_id = load_or_create_owner_id(&data_dir())?;
    let stats_dir = share.run_dir.join("stats");
    let events_dir = share.run_dir.join("events");
    let mut leagues = Leagues::new(share.seed, &content);
    // The red-card suite plays the default clubs, loaded once.
    let defaults: Option<[TeamFile; 2]> = if share.suite == Suite::RedCard {
        Some(default_clubs(&content, &dir)?)
    } else {
        None
    };
    let names: Vec<String> = content
        .tactics
        .formations
        .iter()
        .map(|f| f.name.clone())
        .collect();
    let pairings = fixtures::pairings(names.len());
    let selected: Vec<usize> = if share.pairings.is_empty() {
        (0..pairings.len()).collect()
    } else {
        share.pairings.to_vec()
    };
    let planned = fixtures::keyed(share.suite, share.seed, share.matches, &names, &selected);
    let remaining = remaining(share.run_dir, share.session, planned)?;
    let shards = share.shards.max(1);
    let mut ledger: Option<LedgerWriter> = None;
    let mut inject_match = share.inject == Some(InjectFailure::Match);
    let mut inject_panic = share.inject == Some(InjectFailure::Panic);
    let size = run_folder::unit_size(remaining.len(), shards);
    for (place, unit) in remaining.chunks(size).enumerate() {
        let place = u32::try_from(place).unwrap_or(u32::MAX);
        if place % shards != share.shard {
            continue;
        }
        let (unit, finished) = match share.stop {
            Some(stop) if place > stop => break,
            Some(stop) if place == stop => (&unit[..unit.len() / 2], false),
            _ => (unit, true),
        };
        let mut done = Vec::with_capacity(unit.len());
        for keyed in unit {
            let match_id = fixtures::match_id(keyed.key, share.run_millis);
            let (teams, formations, red_card) = setup(
                keyed,
                &mut leagues,
                defaults.as_ref(),
                &pairings,
                bands.stronger_team.attribute_boost,
            );
            let seed = keyed.engine_seed;
            // A worker process does not catch a panic: it stops, and the parent counts a
            // failed worker.
            assert!(!std::mem::take(&mut inject_panic), "injected panic");
            let played = if std::mem::take(&mut inject_match) {
                Err(EngineError::InvalidConfig("injected failure".into()))
            } else {
                let path = events_dir.join(format!("{match_id}.jsonl"));
                play_match(
                    &MatchInput {
                        content: &content,
                        commentary: &commentary,
                        teams: &teams,
                        formations,
                        red_card,
                        key: keyed.key,
                        seed,
                        minutes: share.minutes,
                        owner_id: &owner_id,
                        match_id: &match_id,
                    },
                    &mut |_| Some(path.clone()),
                )
            };
            let stats = match played {
                Ok(played) => played.stats,
                Err(err) => {
                    tracing::error!(
                        signal = "calibrate.match_failed",
                        match.id = %match_id,
                        error = %err
                    );
                    failure(
                        (&owner_id, &match_id, seed),
                        &teams,
                        &Failure::of(&err),
                        &content,
                    )
                }
            };
            write_stats_at(&stats_dir, &stats)?;
            done.push(Done {
                key: keyed.key,
                seed,
                match_id,
                session: String::new(),
            });
        }
        if finished {
            let writer = match &mut ledger {
                Some(w) => w,
                None => ledger.insert(LedgerWriter::open(
                    share.run_dir,
                    share.session,
                    share.suite.code(),
                    share.shard,
                )?),
            };
            writer.append(&UnitLine {
                suite: share.suite.code().to_string(),
                unit: place,
                fixtures: done,
            })?;
        }
    }
    Ok(0)
}

/// The teams, the formations, and the sending-off of the match `keyed`: the generated
/// league's clubs (the stronger one boosted by `boost` in the strength suite), or the
/// default clubs in the red-card suite; each side's starting formation in the formations
/// suite; and, in the red-card suite, the player sent off at kick-off.
pub fn setup(
    keyed: &Keyed,
    leagues: &mut Leagues<'_>,
    defaults: Option<&[TeamFile; 2]>,
    pairings: &[[usize; 2]],
    boost: f64,
) -> ([TeamFile; 2], Option<[u8; 2]>, Option<Option<usize>>) {
    let teams = match defaults {
        Some([first, second]) if keyed.fixture.clubs == [1, 0] => [second.clone(), first.clone()],
        Some([first, second]) => [first.clone(), second.clone()],
        None => {
            let mut teams = leagues.teams(&keyed.fixture);
            if let Some(side) = keyed.boosted {
                teams[side] = fixtures::boosted(&teams[side], boost);
            }
            teams
        }
    };
    // A tactics file holds at most 16 formations.
    let formations = keyed.pairing.map(|(pairing, first_side)| {
        let [first, second] = pairings[pairing].map(|i| i as u8);
        if first_side == 0 {
            [first, second]
        } else {
            [second, first]
        }
    });
    let red_card = keyed.arm.map(|arm| RED_CARD_ARMS[arm].1);
    (teams, formations, red_card)
}

/// One match to play, as both runners describe it.
pub struct MatchInput<'a> {
    pub content: &'a Content,
    pub commentary: &'a Commentary,
    pub teams: &'a [TeamFile; 2],
    /// Each side's starting formation, in the formations suite.
    pub formations: Option<[u8; 2]>,
    /// The red-card suite: cards otherwise off, and the player sent off at kick-off.
    pub red_card: Option<Option<usize>>,
    pub key: FixtureKey,
    pub seed: u64,
    pub minutes: u32,
    pub owner_id: &'a str,
    pub match_id: &'a str,
}

/// A match played to full time: its statistics record and its compact row.
pub struct Played {
    pub stats: MatchStats,
    pub row: Row,
}

/// One match to full time, checked by the running rule checker as it plays (every tick
/// and event rule, with no tick list kept). `events_to` sees the match's row once it is over and
/// names the event file to write, with every event and its commentary line, or `None` for
/// no event file; nothing else is written. `formations`, when present, is each side's
/// starting formation. `red_card`, when present, plays the match with cards otherwise off
/// and sends the named player off at kick-off, as the slow test
/// `a_sending_off_gives_no_advantage` does.
pub fn play_match(
    m: &MatchInput<'_>,
    events_to: &mut dyn FnMut(&Row) -> Option<PathBuf>,
) -> Result<Played, EngineError> {
    let (content, teams) = (m.content, m.teams);
    let mut config = MatchConfig::new(m.seed, m.minutes, content, [&teams[0], &teams[1]])?;
    if let Some(sides) = m.formations {
        for (team, formation) in sides.into_iter().enumerate() {
            let mut tactics = Tactics::defaults(&content.tactics);
            tactics.set_formation(formation, &content.tactics);
            config = config.with_tactics(team, tactics);
        }
    }
    if m.red_card.is_some() {
        // Set on the match, not in the content, so the content hash is unchanged.
        config.tuning.red_base = 0.0;
        config.tuning.yellow_base = 0.0;
        config.tuning.yellow_aggression_weight = 0.0;
    }
    let pack_version = config.rules.schema_version;
    let refs = team_refs(teams);
    let content_hash = config.content_hash.clone();
    let rules = StreamRules::for_config(&config);
    let mut sim = Simulation::new(config)?;
    if let Some(Some(player)) = m.red_card {
        sim.send_off_before_kickoff(player);
    }
    // Each tick and event is judged as the match plays; no record is kept.
    let mut check = Timed {
        check: RunningCheck::for_match(&sim, rules),
        calls: 0,
        sampled: Duration::ZERO,
    };
    let started = Instant::now();
    {
        let _play = stages::enter(Stage::Play);
        sim.run(&mut check)?;
    }
    let elapsed = started.elapsed();
    stages::shift(Stage::Play, Stage::Checks, check.sampled * CHECK_STRIDE);
    let written = check.check.ticks();
    // Full time comes after the last record: the checker sees it before the events go.
    let violations = {
        let _checks = stages::enter(Stage::Checks);
        check.check.finish_match(&sim).len()
    };
    let events = sim.take_events();
    let summary = sim.summary();
    let tactics = TacticsStats::new(&sim);
    let row = Row::played(
        (m.key, m.seed),
        &summary,
        &tactics,
        violations,
        sim.tick(),
        u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX),
    );
    if let Some(path) = events_to(&row) {
        let _commentary = stages::enter(Stage::Commentary);
        let mut ids = Ids::new(&sim);
        let mut commentator = Commentator::for_match(m.commentary, &sim);
        let mut writer =
            EventWriter::create_at(&path).map_err(|e| EngineError::Sink(e.to_string()))?;
        for event in &events {
            let line = match_event(
                event,
                m.owner_id,
                m.match_id,
                [&refs[0].id, &refs[1].id],
                &mut ids,
            );
            writer
                .write(&line.commentary(commentator.line(event)))
                .map_err(|e| EngineError::Sink(e.to_string()))?;
        }
    }
    let stats = MatchStats {
        owner_id: m.owner_id.to_string(),
        match_id: m.match_id.to_string(),
        seed: m.seed,
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
        tactics,
        figures: MatchFigures::new(&summary, sim.managers()),
        script: ScriptFigures::new(sim.plugins()),
    };
    Ok(Played { stats, row })
}

/// The checker calls timed: one in this many, scaled up, so the clock costs less than the
/// checker on a short tick.
const CHECK_STRIDE: u32 = 16;

/// The running checker, with every [`CHECK_STRIDE`]th call timed.
struct Timed {
    check: RunningCheck,
    calls: u32,
    sampled: Duration,
}

impl Timed {
    fn call<T>(&mut self, f: impl FnOnce(&mut RunningCheck) -> T) -> T {
        self.calls = self.calls.wrapping_add(1);
        if self.calls.is_multiple_of(CHECK_STRIDE) {
            let t = Instant::now();
            let out = f(&mut self.check);
            self.sampled += t.elapsed();
            out
        } else {
            f(&mut self.check)
        }
    }
}

impl TickSink for Timed {
    fn on_step(&mut self, sim: &Simulation) -> Result<(), EngineError> {
        self.call(|c| c.on_step(sim))
    }

    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.call(|c| c.on_tick(record))
    }
}

fn team_refs(teams: &[TeamFile; 2]) -> [TeamRef; 2] {
    [0, 1].map(|t| TeamRef {
        id: teams[t].club.id.clone(),
        name: teams[t].club.name.clone(),
    })
}

/// Why a match failed, as its statistics record names it.
pub struct Failure {
    pub error_type: &'static str,
    pub error_code: &'static str,
    pub retriable: bool,
}

impl Failure {
    /// A match the engine could not play.
    pub fn of(err: &EngineError) -> Self {
        Self {
            error_type: err.error_type(),
            error_code: err.error_code(),
            retriable: err.retriable(),
        }
    }

    /// A match that panicked and was caught.
    pub const PANIC: Self = Self {
        error_type: "panic",
        error_code: "match-panicked",
        retriable: false,
    };
}

/// The statistics record of a match that failed. The figures are zero; the rule pack
/// version and the managers are the ones the match would have used.
pub fn failure(
    (owner_id, match_id, seed): (&str, &str, u64),
    teams: &[TeamFile; 2],
    why: &Failure,
    content: &Content,
) -> MatchStats {
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
            error_type: Some(why.error_type.to_string()),
            error_code: Some(why.error_code.to_string()),
            error_retriable: Some(why.retriable),
            ..MatchFigures::default()
        },
        script: ScriptFigures::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Shipped {
        dir: ContentDir,
        content: Content,
        commentary: Commentary,
    }

    fn shipped() -> Shipped {
        let dir = ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
        let content = Content::load(&dir).unwrap();
        let commentary = Commentary::load(&dir).unwrap();
        Shipped {
            dir,
            content,
            commentary,
        }
    }

    fn play(
        s: &Shipped,
        teams: &[TeamFile; 2],
        formations: Option<[u8; 2]>,
        red_card: Option<Option<usize>>,
        seed: u64,
        minutes: u32,
    ) -> Played {
        play_match(
            &MatchInput {
                content: &s.content,
                commentary: &s.commentary,
                teams,
                formations,
                red_card,
                key: FixtureKey::from_u64(seed),
                seed,
                minutes,
                owner_id: "owner",
                match_id: "match",
            },
            &mut |_| None,
        )
        .unwrap()
    }

    /// A match checked as it plays counts every tick it played, and a clean match has no
    /// violation.
    #[test]
    fn a_match_checked_as_it_plays_counts_every_tick() {
        let s = shipped();
        let teams = default_clubs(&s.content, &s.dir).unwrap();
        let played = play(&s, &teams, None, None, 7, 5);
        let config = MatchConfig::new(7, 5, &s.content, [&teams[0], &teams[1]]).unwrap();
        let mut sim = Simulation::new(config).unwrap();
        sim.run(&mut engine::NullSink).unwrap();
        assert_eq!(played.stats.ticks_written, sim.tick());
        assert_eq!(played.row.ticks, sim.tick());
        assert_eq!(played.stats.validate_violations, 0);
        assert_eq!(played.row.violations, 0);
    }

    /// One 90-minute match of each suite's kind (a generated league, a boosted club, a
    /// formations pairing, and every red-card arm) breaks no tick or event rule, so the
    /// event rules add no violation to calibrate's count.
    #[test]
    fn every_suite_kind_plays_without_a_violation() {
        let s = shipped();
        let bands = Registry::load(&s.dir).unwrap();
        let defaults = default_clubs(&s.content, &s.dir).unwrap();
        let names: Vec<String> = s
            .content
            .tactics
            .formations
            .iter()
            .map(|f| f.name.clone())
            .collect();
        let pairings = fixtures::pairings(names.len());
        let selected: Vec<usize> = (0..pairings.len()).collect();
        let mut leagues = Leagues::new(2026, &s.content);
        let mut cases: Vec<(Suite, Keyed)> = Vec::new();
        for suite in [Suite::Equal, Suite::Strength, Suite::Formations] {
            let first = fixtures::keyed(suite, 2026, 1, &names, &selected)
                .into_iter()
                .next()
                .unwrap();
            cases.push((suite, first));
        }
        let red = fixtures::keyed(Suite::RedCard, 2026, 4, &names, &selected);
        for arm in 0..RED_CARD_ARMS.len() {
            let keyed = red.iter().find(|k| k.arm == Some(arm)).unwrap();
            cases.push((Suite::RedCard, *keyed));
        }
        for (suite, keyed) in &cases {
            let defaults = (*suite == Suite::RedCard).then_some(&defaults);
            let (teams, formations, red_card) = setup(
                keyed,
                &mut leagues,
                defaults,
                &pairings,
                bands.stronger_team.attribute_boost,
            );
            let played = play(&s, &teams, formations, red_card, keyed.engine_seed, 90);
            assert_eq!(
                played.row.violations, 0,
                "{suite:?} arm {:?}: a violation",
                keyed.arm
            );
        }
    }
}
