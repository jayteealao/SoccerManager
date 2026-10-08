//! One calibration match, as the one-process runner plays it: [`play_match`] plays the
//! match with the AI manager on both sides, checks its rules as it plays, and gives its
//! statistics record and compact row. In the formations suite, each side starts in its
//! pairing's formation, with the lineup its AI picks for that formation, and keeps it. In
//! the red-card suite, the default clubs play with cards otherwise off, in both home and
//! away orders, and the arm's away player is sent off at kick-off.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::observe::{
    LawStats, MatchFigures, MatchStats, RatingEntry, ScriptFigures, TacticsStats, TeamRef,
};
use engine::{
    Commentary, Commentator, Content, ContentDir, EngineError, MatchConfig, RunningCheck,
    Simulation, StreamRules, Tactics, TickRecord, TickSink,
};
use stream::EventWriter;

use super::fixtures::{self, FixtureKey, Keyed, Leagues};
use super::rows::Row;
use super::run_folder;
use super::stages::{self, Stage};
use crate::report::RED_CARD_ARMS;
use crate::stream_run::{Ids, match_event};

/// The fixtures of `planned` that earlier sessions of the run in `run_dir` left
/// unfinished, in fixture order: the list the runner cuts into work units.
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

/// One match to play.
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
            config = config.with_ai_tactics(team, tactics);
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
    let mut check = Timed::new(RunningCheck::for_match(&sim, rules));
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
        ratings: RatingEntry::of_match(&sim),
    };
    Ok(Played { stats, row })
}

/// The checker calls timed: one in this many, scaled up, so the clock costs less than the
/// checker on a short tick.
const CHECK_STRIDE: u32 = 16;

/// The two kinds of checker call: the step check, then the tick check, once per tick.
const STEP: usize = 0;
const TICK: usize = 1;

/// The running checker, with every [`CHECK_STRIDE`]th call of each kind timed. Each kind
/// keeps its own count: the two kinds alternate, so one shared count would time only one
/// kind and scale its cost onto both.
struct Timed {
    check: RunningCheck,
    calls: [u32; 2],
    timed: [u32; 2],
    sampled: Duration,
}

impl Timed {
    fn new(check: RunningCheck) -> Self {
        Self {
            check,
            calls: [0; 2],
            timed: [0; 2],
            sampled: Duration::ZERO,
        }
    }

    fn call<T>(&mut self, kind: usize, f: impl FnOnce(&mut RunningCheck) -> T) -> T {
        self.calls[kind] = self.calls[kind].wrapping_add(1);
        if self.calls[kind].is_multiple_of(CHECK_STRIDE) {
            self.timed[kind] += 1;
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
        self.call(STEP, |c| c.on_step(sim))
    }

    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.call(TICK, |c| c.on_tick(record))
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
        ratings: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Suite;
    use crate::report::bands::Registry;

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

    /// The checker's time is sampled from both kinds of call: every 16th step check and
    /// every 16th tick check, each counted on its own, so neither kind's cost stands in for
    /// the other's.
    #[test]
    fn the_checker_time_samples_every_sixteenth_call_of_each_kind() {
        let s = shipped();
        let teams = default_clubs(&s.content, &s.dir).unwrap();
        let config = MatchConfig::new(7, 1, &s.content, [&teams[0], &teams[1]]).unwrap();
        let rules = StreamRules::for_config(&config);
        let mut sim = Simulation::new(config).unwrap();
        let mut check = Timed::new(RunningCheck::for_match(&sim, rules));
        sim.run(&mut check).unwrap();
        let ticks = sim.tick();
        assert!(ticks >= 32, "{ticks} ticks");
        assert_eq!(
            check.calls,
            [ticks, ticks],
            "one step and one tick check per tick"
        );
        for kind in [STEP, TICK] {
            assert_eq!(check.timed[kind], ticks / CHECK_STRIDE, "kind {kind}");
        }
    }

    /// An A/A comparison on played matches: both arms are full matches of the shipped
    /// engine on different fixtures of the equal suite, so every change is noise. Each
    /// replicate draws two arms of 150 matches from one pool of played matches, pairs them,
    /// and judges them with the change run's max-t bootstrap. Over 1000 replicates the test
    /// flags at most 3 percent, and every band of the equal suite has its own row. Slow (it
    /// plays 400 matches and runs 1000 bootstraps): run it in release with
    /// `cargo test --release -p engine-cli --bin engine-cli -- --ignored a_a_on_played`.
    #[test]
    #[ignore = "slow: plays 400 full matches and judges 1000 replicates; run in release"]
    fn an_a_a_on_played_matches_flags_at_most_three_percent_of_replicates() {
        use std::collections::BTreeMap;

        use engine::rng::EngineRng;

        use crate::report::RunBuilder;
        use crate::report::verdict::{self, FALSE_ALARM, RESAMPLES};

        let (pool_size, n, replicates) = (400u32, 150usize, 1000u32);
        let s = shipped();
        let bands = Registry::load(&s.dir).unwrap();
        let names: Vec<String> = s
            .content
            .tactics
            .formations
            .iter()
            .map(|f| f.name.clone())
            .collect();
        let keyed = fixtures::keyed(Suite::Equal, 2026, pool_size, &names, &[]);
        let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
        let (s, bands) = (&s, &bands);
        let pool: Vec<MatchStats> = std::thread::scope(|scope| {
            let handles: Vec<_> = keyed
                .chunks(keyed.len().div_ceil(threads))
                .map(|chunk| {
                    scope.spawn(move || {
                        let mut leagues = Leagues::new(2026, &s.content);
                        chunk
                            .iter()
                            .map(|k| {
                                let (teams, formations, red_card) = setup(
                                    k,
                                    &mut leagues,
                                    None,
                                    &[],
                                    bands.stronger_team.attribute_boost,
                                );
                                play(s, &teams, formations, red_card, k.engine_seed, 90)
                                    .row
                                    .band_record()
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap())
                .collect()
        });
        assert_eq!(pool.len(), pool_size as usize);
        let equal_bands: Vec<&str> = bands
            .bands
            .iter()
            .filter(|b| b.judged_in(Suite::Equal))
            .map(|b| b.band.as_str())
            .collect();
        let keys: BTreeMap<Suite, Vec<u64>> = [(Suite::Equal, (0..n as u64).collect())].into();
        let per_unit: BTreeMap<Suite, u32> = [(Suite::Equal, n as u32)].into();
        let mut rng = EngineRng::from_seed(2026);
        let mut alarms = 0u32;
        for rep in 0..replicates {
            // 2n different matches of the pool, in a random order, split into two arms.
            let mut picks: Vec<usize> = (0..pool.len()).collect();
            for i in 0..2 * n {
                let j = i + rng.range_usize(picks.len() - i);
                picks.swap(i, j);
            }
            let arm = |chosen: &[usize]| {
                let mut b = RunBuilder::new(bands.clone());
                b.plan(Suite::Equal, n as u32);
                for &p in chosen {
                    b.add(Suite::Equal, pool[p].clone(), None);
                }
                b
            };
            let (first, second) = (arm(&picks[..n]), arm(&picks[n..2 * n]));
            let (sets, rows) = verdict::paired((&first, &keys), (&second, &keys), &per_unit);
            let (found, c) = verdict::bootstrap(&sets, &rows, 1000 + u64::from(rep), RESAMPLES);
            let (judged, _) = verdict::judge(&sets, &rows, &found, c, Vec::new(), &BTreeMap::new());
            for band in &equal_bands {
                assert!(
                    judged.iter().any(|r| r.band == *band),
                    "{band} has its own row"
                );
            }
            // The move rule of the verdict: a change beyond `c` errors, or any change of a
            // band with no error.
            let moved = found.iter().any(|f| match f.diff {
                Some(d) if f.se > 0.0 => d.abs() > c * f.se,
                Some(d) => d != 0.0,
                None => false,
            });
            alarms += u32::from(moved);
        }
        let rate = f64::from(alarms) / f64::from(replicates);
        eprintln!("A/A rate on played matches {rate} over {replicates} replicates of {n} pairs");
        assert!(rate <= FALSE_ALARM, "joint false-alarm rate {rate}");
    }
}
