//! The one-process runner of a calibration run: every arm's and suite's unfinished
//! fixtures, cut into work units, go into one work list, and `--jobs` threads pull the next
//! unit from it until it is empty. No thread waits for a suite to finish, so a suite's wall
//! time is the span from the first start of any of its units to the last finish.
//!
//! Each match runs inside `catch_unwind` and owns its simulation, so a panic touches
//! nothing shared: the match is recorded as failed with its fixture key, and the thread
//! goes on with the next fixture. Each finished unit appends its compact rows as one block
//! to the thread's row file, then its ledger line.
//!
//! A match gets a full recording (its statistics file, and its event file with commentary)
//! when its key falls in the 1-in-16 sample, or when it is an outlier: it failed, panicked
//! or hit a dark path, the rule checker found a violation, or one of its measures lies outside
//! the 1st to 99th percentile of the same suite's earlier matches in this session. The
//! decision comes before anything is written, so no file is written and removed again.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use engine::data::TeamFile;
use engine::observe::write_stats_at;
use engine::{Commentary, Content, EngineError};

use super::RunCtx;
use super::fixtures::{self, Keyed, Leagues};
use super::rows::{Outcome, Row, RowsWriter, reason};
use super::run_folder::{self, Done, LedgerWriter, UnitLine};
use super::worker::{self, Failure, MatchInput};
use crate::cli::{InjectFailure, KeepEvents};
use crate::matchday::record::panic_message;
use crate::report::Suite;

/// Earlier matches of a suite before the percentile rule judges a match.
pub const WARM_UP: usize = 100;
/// One match in this many is recorded, chosen by fixture key.
pub const SAMPLE_EVERY: u64 = 16;

/// One arm of the run: its folder and its content under its flag states.
pub struct ArmInput<'a> {
    pub dir: &'a Path,
    pub content: Content,
}

/// One work unit: fixtures of one suite of one arm, in fixture order.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub arm: usize,
    pub suite: Suite,
    /// The unit's place among the units of its arm and suite in this session.
    pub place: u32,
    pub fixtures: Vec<Keyed>,
}

/// The work list: for each arm, for each suite, the fixtures earlier sessions left
/// unfinished, cut into units of [`run_folder::unit_size`].
pub fn work_list(
    arms: &[&Path],
    suites: &[Suite],
    jobs: u32,
    remaining: &dyn Fn(&Path, Suite) -> anyhow::Result<Vec<Keyed>>,
) -> anyhow::Result<Vec<Job>> {
    let mut out = Vec::new();
    for (arm, dir) in arms.iter().enumerate() {
        for &suite in suites {
            let left = remaining(dir, suite)?;
            let size = run_folder::unit_size(left.len(), jobs);
            for (place, unit) in left.chunks(size).enumerate() {
                out.push(Job {
                    arm,
                    suite,
                    place: u32::try_from(place).unwrap_or(u32::MAX),
                    fixtures: unit.to_vec(),
                });
            }
        }
    }
    Ok(out)
}

/// The per-match measures the percentile rule reads: goals, shots, shots on target,
/// expected goals, passes, pass accuracy, home possession, yellow cards, red cards,
/// corners, throw-ins and goal kicks, each over both teams.
fn measures(r: &Row) -> [f64; 12] {
    let both = |a: [u32; 2]| f64::from(a[0] + a[1]);
    let share = |part: u32, whole: u32| {
        if whole == 0 {
            0.0
        } else {
            f64::from(part) / f64::from(whole)
        }
    };
    [
        both(r.goals),
        both(r.shots),
        both(r.shots_on_target),
        r.xg[0] + r.xg[1],
        both(r.passes),
        share(
            r.passes_completed[0] + r.passes_completed[1],
            r.passes[0] + r.passes[1],
        ),
        share(
            r.possession_ticks[0],
            r.possession_ticks[0] + r.possession_ticks[1],
        ),
        both(r.yellow),
        both(r.red),
        both(r.corners),
        both(r.throw_ins),
        both(r.goal_kicks),
    ]
}

/// `true` when `v` lies strictly outside the 1st to 99th percentile of `sorted` (nearest
/// rank), once `sorted` holds at least [`WARM_UP`] values.
pub fn extreme(sorted: &[f64], v: f64) -> bool {
    let n = sorted.len();
    if n < WARM_UP {
        return false;
    }
    let lo = sorted[n.div_ceil(100) - 1];
    let hi = sorted[(99 * n).div_ceil(100) - 1];
    v < lo || v > hi
}

/// Decides which matches get a full recording. The percentile rule keeps, per arm and
/// suite, every earlier finished match's measures, sorted.
#[derive(Debug, Default)]
pub struct Recorder {
    pub keep_all: bool,
    seen: BTreeMap<(usize, Suite), Vec<Vec<f64>>>,
}

impl Recorder {
    /// The record reasons of `row`, a match of `suite` in arm `arm`; a finished match's
    /// measures join the suite's lists after it is judged.
    pub fn reasons(&mut self, arm: usize, suite: Suite, row: &Row) -> u8 {
        let mut out = 0;
        if row.key.as_u64().is_multiple_of(SAMPLE_EVERY) {
            out |= reason::SAMPLE;
        }
        if row.outcome != Outcome::Success || row.change_never_applied > 0 {
            out |= reason::ERROR;
        }
        if row.violations > 0 {
            out |= reason::VIOLATION;
        }
        if self.keep_all {
            out |= reason::ALL;
        }
        if row.outcome == Outcome::Success {
            let lists = self
                .seen
                .entry((arm, suite))
                .or_insert_with(|| vec![Vec::new(); 12]);
            let values = measures(row);
            if values.iter().zip(lists.iter()).any(|(&v, l)| extreme(l, v)) {
                out |= reason::EXTREME;
            }
            // sdlc-debt: a sorted Vec per measure costs O(n) per insert (a memmove of up to
            // 55 000 values in the formations suite of a full run); an order-statistic tree
            // would make it O(log n) if a profile of a full run shows the cost.
            for (v, list) in values.into_iter().zip(lists.iter_mut()) {
                let at = list.partition_point(|&x| x < v);
                list.insert(at, v);
            }
        }
        out
    }
}

/// What the runner did.
#[derive(Debug, Default)]
pub struct Ran {
    /// Each arm's suites: the wall time from the first unit's start to the last unit's
    /// finish, in milliseconds; 0 for a suite with nothing left to play.
    pub wall: Vec<BTreeMap<Suite, u64>>,
    /// The work list's wall time, in milliseconds.
    pub total_ms: u64,
    /// The stop seam cut the run short.
    pub stopped: bool,
}

/// What a thread keeps for one arm.
struct ArmState<'a> {
    leagues: Leagues<'a>,
    rows: Option<RowsWriter>,
    ledgers: BTreeMap<Suite, LedgerWriter>,
}

/// Plays the work list of `arms` on `ctx.jobs` threads. The first error of any thread (a
/// row, ledger or statistics file that cannot be written) stops every thread and comes back.
pub(super) fn run_arms(
    ctx: &RunCtx<'_>,
    arms: &[ArmInput<'_>],
    owner_id: &str,
) -> anyhow::Result<Ran> {
    let opts = ctx.opts;
    let dirs: Vec<&Path> = arms.iter().map(|a| a.dir).collect();
    let list = work_list(&dirs, &ctx.suites, ctx.jobs, &|dir, suite| {
        worker::remaining(dir, &ctx.session, ctx.keyed(suite))
    })?;
    let commentary = Commentary::load(ctx.dir)?;
    let defaults: Vec<Option<[TeamFile; 2]>> = arms
        .iter()
        .map(|a| {
            ctx.suites
                .contains(&Suite::RedCard)
                .then(|| worker::default_clubs(&a.content, ctx.dir))
                .transpose()
        })
        .collect::<anyhow::Result<_>>()?;
    let pairings = fixtures::pairings(ctx.formation_names.len());
    let boost = ctx.bands.stronger_team.attribute_boost;
    let stop = opts.stop_after_units.map(|k| k as usize);
    let recorder = Mutex::new(Recorder {
        keep_all: opts.keep_events == KeepEvents::All,
        ..Recorder::default()
    });
    let spans: Mutex<BTreeMap<(usize, Suite), (Instant, Instant)>> = Mutex::new(BTreeMap::new());
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let first_error: Mutex<Option<anyhow::Error>> = Mutex::new(None);
    let started = Instant::now();
    let threads = ctx.jobs.max(1);

    std::thread::scope(|scope| {
        for thread in 0..threads {
            let (list, arms, defaults, pairings) = (&list, arms, &defaults, &pairings);
            let (recorder, spans, next, failed, first_error) =
                (&recorder, &spans, &next, &failed, &first_error);
            let commentary = &commentary;
            scope.spawn(move || {
                let mut state: Vec<ArmState<'_>> = arms
                    .iter()
                    .map(|a| ArmState {
                        leagues: Leagues::new(opts.seed, &a.content),
                        rows: None,
                        ledgers: BTreeMap::new(),
                    })
                    .collect();
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= list.len() || failed.load(Ordering::Relaxed) {
                        break;
                    }
                    let (fixtures, finished) = match stop {
                        Some(k) if index > k => break,
                        Some(k) if index == k => {
                            let all = &list[index].fixtures;
                            (&all[..all.len() / 2], false)
                        }
                        _ => (&list[index].fixtures[..], true),
                    };
                    let job = &list[index];
                    let unit_started = Instant::now();
                    let done = play_unit(
                        &UnitCtx {
                            ctx,
                            arm: &arms[job.arm],
                            arm_index: job.arm,
                            suite: job.suite,
                            commentary,
                            defaults: defaults[job.arm].as_ref(),
                            pairings,
                            boost,
                            owner_id,
                            recorder,
                            inject: if index == 0 {
                                opts.inject_failure
                            } else {
                                None
                            },
                        },
                        fixtures,
                        &mut state[job.arm].leagues,
                    )
                    .and_then(|(rows, dones)| {
                        if finished {
                            commit(
                                ctx,
                                &mut state[job.arm],
                                (arms[job.arm].dir, thread),
                                job,
                                &rows,
                                dones,
                            )?;
                        }
                        Ok(())
                    });
                    if let Err(e) = done {
                        failed.store(true, Ordering::Relaxed);
                        first_error
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .get_or_insert(e);
                        break;
                    }
                    let now = Instant::now();
                    spans
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .entry((job.arm, job.suite))
                        .and_modify(|(first, last)| {
                            *first = (*first).min(unit_started);
                            *last = (*last).max(now);
                        })
                        .or_insert((unit_started, now));
                }
            });
        }
    });
    if let Some(e) = first_error
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
    {
        return Err(e);
    }
    let spans = spans
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let millis = |d: std::time::Duration| u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
    let wall = (0..arms.len())
        .map(|arm| {
            ctx.suites
                .iter()
                .map(|&suite| {
                    let ms = spans
                        .get(&(arm, suite))
                        .map_or(0, |(first, last)| millis(last.duration_since(*first)));
                    (suite, ms)
                })
                .collect()
        })
        .collect();
    Ok(Ran {
        wall,
        total_ms: millis(started.elapsed()),
        stopped: stop.is_some_and(|k| k < list.len()),
    })
}

/// What every match of a unit shares.
struct UnitCtx<'a> {
    ctx: &'a RunCtx<'a>,
    arm: &'a ArmInput<'a>,
    arm_index: usize,
    suite: Suite,
    commentary: &'a Commentary,
    defaults: Option<&'a [TeamFile; 2]>,
    pairings: &'a [[usize; 2]],
    boost: f64,
    owner_id: &'a str,
    recorder: &'a Mutex<Recorder>,
    /// The failure seam, for the first unit of the work list only.
    inject: Option<InjectFailure>,
}

/// Plays `fixtures` and writes the full recording of the matches the recorder picks; the
/// rows and the ledger entries come back for [`commit`].
fn play_unit(
    u: &UnitCtx<'_>,
    fixtures: &[Keyed],
    leagues: &mut Leagues<'_>,
) -> anyhow::Result<(Vec<Row>, Vec<Done>)> {
    let ctx = u.ctx;
    let stats_dir = u.arm.dir.join("stats");
    let events_dir = u.arm.dir.join("events");
    let mut rows = Vec::with_capacity(fixtures.len());
    let mut dones = Vec::with_capacity(fixtures.len());
    for (i, keyed) in fixtures.iter().enumerate() {
        let match_id = fixtures::match_id(keyed.key, ctx.millis);
        let (teams, formations, red_card) =
            worker::setup(keyed, leagues, u.defaults, u.pairings, u.boost);
        let seed = keyed.engine_seed;
        let inject = if i == 0 { u.inject } else { None };
        let mut reasons = 0u8;
        let played = catch_unwind(AssertUnwindSafe(|| {
            match inject {
                Some(InjectFailure::Panic) => panic!("injected panic"),
                Some(InjectFailure::Match) => {
                    return Err(EngineError::InvalidConfig("injected failure".into()));
                }
                _ => {}
            }
            worker::play_match(
                &MatchInput {
                    content: &u.arm.content,
                    commentary: u.commentary,
                    teams: &teams,
                    formations,
                    red_card,
                    key: keyed.key,
                    seed,
                    minutes: ctx.opts.minutes,
                    owner_id: u.owner_id,
                    match_id: &match_id,
                },
                &mut |row| {
                    reasons = lock(u.recorder).reasons(u.arm_index, u.suite, row);
                    (reasons != 0).then(|| events_dir.join(format!("{match_id}.jsonl")))
                },
            )
        }));
        let (mut row, stats) = match played {
            Ok(Ok(played)) => (played.row, (reasons != 0).then_some(played.stats)),
            Ok(Err(err)) => {
                tracing::error!(
                    signal = "calibrate.match_failed",
                    match.id = %match_id,
                    error = %err
                );
                let row = Row::failed((keyed.key, seed), Outcome::Error);
                reasons = lock(u.recorder).reasons(u.arm_index, u.suite, &row);
                let ids = (u.owner_id, match_id.as_str(), seed);
                let stats = worker::failure(ids, &teams, &Failure::of(&err), &u.arm.content);
                (row, Some(stats))
            }
            Err(payload) => {
                eprintln!(
                    "match {} ({}) panicked: {}; recorded as failed",
                    keyed.key,
                    u.suite.code(),
                    panic_message(&*payload)
                );
                let row = Row::failed((keyed.key, seed), Outcome::Panic);
                reasons = lock(u.recorder).reasons(u.arm_index, u.suite, &row);
                let ids = (u.owner_id, match_id.as_str(), seed);
                let stats = worker::failure(ids, &teams, &Failure::PANIC, &u.arm.content);
                (row, Some(stats))
            }
        };
        row.reasons = reasons;
        if let Some(stats) = stats {
            write_stats_at(&stats_dir, &stats)?;
        }
        rows.push(row);
        dones.push(Done {
            key: keyed.key,
            seed,
            match_id,
            session: String::new(),
        });
    }
    Ok((rows, dones))
}

/// The recorder, also after a thread panicked holding it: its lists stay sorted.
fn lock(recorder: &Mutex<Recorder>) -> std::sync::MutexGuard<'_, Recorder> {
    recorder
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Writes a finished unit: its rows as one synced block, then its ledger line.
fn commit(
    ctx: &RunCtx<'_>,
    state: &mut ArmState<'_>,
    (dir, thread): (&Path, u32),
    job: &Job,
    rows: &[Row],
    dones: Vec<Done>,
) -> anyhow::Result<()> {
    let writer = match &mut state.rows {
        Some(w) => w,
        None => state
            .rows
            .insert(RowsWriter::open(dir, &ctx.session, thread)?),
    };
    writer.append_block(job.suite.code(), job.place, rows)?;
    let ledger = match state.ledgers.entry(job.suite) {
        std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
        std::collections::btree_map::Entry::Vacant(e) => e.insert(LedgerWriter::open(
            dir,
            &ctx.session,
            job.suite.code(),
            thread,
        )?),
    };
    ledger.append(&UnitLine {
        suite: job.suite.code().to_string(),
        unit: job.place,
        fixtures: dones,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibrate::fixtures::{Fixture, FixtureKey};

    fn row(key: u64, goals: u32) -> Row {
        Row {
            key: FixtureKey::from_u64(key),
            goals: [goals, 0],
            ..Row::default()
        }
    }

    #[test]
    fn the_sample_takes_keys_divisible_by_sixteen_and_errors_and_violations_always_record() {
        let mut r = Recorder::default();
        assert_eq!(r.reasons(0, Suite::Equal, &row(32, 1)), reason::SAMPLE);
        assert_eq!(r.reasons(0, Suite::Equal, &row(33, 1)), 0);
        let mut failed = row(33, 0);
        failed.outcome = Outcome::Panic;
        assert_eq!(r.reasons(0, Suite::Equal, &failed), reason::ERROR);
        let mut dark = row(35, 1);
        dark.change_never_applied = 1;
        assert_eq!(r.reasons(0, Suite::Equal, &dark), reason::ERROR);
        let mut violating = row(37, 1);
        violating.violations = 2;
        assert_eq!(r.reasons(0, Suite::Equal, &violating), reason::VIOLATION);
        let mut all = Recorder {
            keep_all: true,
            ..Recorder::default()
        };
        assert_eq!(all.reasons(0, Suite::Equal, &row(33, 1)), reason::ALL);
    }

    #[test]
    fn the_percentile_rule_waits_for_its_warm_up_and_flags_only_strict_tails() {
        // 99 earlier values: too few.
        let few: Vec<f64> = (0..99).map(f64::from).collect();
        assert!(!extreme(&few, 1000.0));
        // 100 values 0 to 99: the 1st percentile is 0, the 99th is 98.
        let hundred: Vec<f64> = (0..100).map(f64::from).collect();
        assert!(!extreme(&hundred, 0.0), "a tie with the 1st percentile");
        assert!(!extreme(&hundred, 98.0), "a tie with the 99th percentile");
        assert!(extreme(&hundred, 98.5));
        assert!(extreme(&hundred, -0.5));
        // Ties: a count every match shares is never extreme.
        let flat = vec![2.0; 500];
        assert!(!extreme(&flat, 2.0));
        assert!(extreme(&flat, 3.0));

        // Through the recorder: 100 matches of 1 goal, then 9 goals is extreme in its own
        // suite and arm only.
        let mut r = Recorder::default();
        for k in 0..100 {
            r.reasons(0, Suite::Equal, &row(16 * k + 1, 1));
        }
        assert_eq!(r.reasons(0, Suite::Equal, &row(3, 9)), reason::EXTREME);
        assert_eq!(r.reasons(1, Suite::Equal, &row(3, 9)), 0, "another arm");
        assert_eq!(
            r.reasons(0, Suite::Strength, &row(3, 9)),
            0,
            "another suite"
        );
        assert_eq!(r.reasons(0, Suite::Equal, &row(5, 1)), 0);
    }

    fn keyed(n: u32) -> Keyed {
        Keyed {
            key: FixtureKey::from_u64(u64::from(n)),
            engine_seed: u64::from(n),
            fixture: Fixture {
                index: n,
                league: 0,
                clubs: [0, 1],
            },
            boosted: None,
            pairing: None,
            arm: None,
        }
    }

    #[test]
    fn the_work_list_runs_arm_by_arm_and_suite_by_suite_in_fixture_order() {
        let (off, on) = (Path::new("off"), Path::new("on"));
        let list = work_list(
            &[off, on],
            &[Suite::Equal, Suite::Strength],
            2,
            &|dir, suite| {
                let n = if suite == Suite::Equal { 20 } else { 3 };
                let base = if dir == off { 0 } else { 100 };
                Ok((0..n).map(|i| keyed(base + i)).collect())
            },
        )
        .unwrap();
        let shape: Vec<(usize, Suite, u32, usize)> = list
            .iter()
            .map(|j| (j.arm, j.suite, j.place, j.fixtures.len()))
            .collect();
        // 20 fixtures over 2 threads: units of 8; 3 fixtures: units of 2.
        assert_eq!(
            shape,
            [
                (0, Suite::Equal, 0, 8),
                (0, Suite::Equal, 1, 8),
                (0, Suite::Equal, 2, 4),
                (0, Suite::Strength, 0, 2),
                (0, Suite::Strength, 1, 1),
                (1, Suite::Equal, 0, 8),
                (1, Suite::Equal, 1, 8),
                (1, Suite::Equal, 2, 4),
                (1, Suite::Strength, 0, 2),
                (1, Suite::Strength, 1, 1),
            ]
        );
        assert_eq!(list[1].fixtures[0], keyed(8));
        assert_eq!(list[5].fixtures[0], keyed(100));
    }

    #[test]
    fn a_caught_panic_names_its_message() {
        let payload = catch_unwind(|| panic!("injected panic")).unwrap_err();
        assert_eq!(panic_message(&*payload), "injected panic");
    }
}
