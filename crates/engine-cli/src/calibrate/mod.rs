//! `engine-cli calibrate`: play many matches with the AI manager on both sides, on threads
//! of one process that pull work units from one list, and write one run report that checks
//! the aggregate figures against the accepted realism bands.
//!
//! `--suite all` plays three suites: equal clubs, a stronger club, and every formation
//! pairing (`--matches` matches per pairing). Each failing band is named on standard error.
//! `--suite red-card` plays the controlled sending-off experiment instead.
//!
//! A targeted run plays only what `--pairing` and `--band` select, and each selected
//! fixture keeps its key and engine seed, so its figures equal a full run's. With
//! `--baseline`, the run is checked against an earlier report before it plays and a diff
//! is printed band by band, each change with its sampling error.
//!
//! Run folder: `run.json` with the run's identity, `ledger/` with one line per finished
//! work unit, `rows/` with one compact row per match, `report.json`, and
//! `stats/<match.id>.json` and `events/<match.id>.jsonl` for the matches with a full
//! recording: about 1 in 16 and every outlier by default, or every match with
//! `--keep-events all`. A paired run
//! (`--pair <flag>`) plays every fixture twice, into `arms/off/` and `arms/on/`, and its one
//! report compares the two arms band by band. The same command into the same folder
//! resumes a stopped run and grows it to a larger `--matches`; a run of another identity
//! moves the old files to `superseded/<run.id>/` and starts again.
//!
//! The hidden `--base <REV>` and `--base-binary <EXE>` name an old engine; its results on
//! the same fixtures come from a cache of its rows and are reported under `calib.base`. The
//! hidden `--worker-processes` plays in worker processes of this binary instead, the old
//! path, kept until the threads give the same band values on the same fixtures.

pub mod base;
pub mod fixtures;
pub mod rows;
pub mod run_folder;
pub mod runner;
pub mod worker;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use anyhow::Context;
use engine::data::{TeamFile, hex12};
use engine::flags::{FlagSetting, FlagState, FlagStates, effective};
use engine::observe::identity::{data_dir, load_or_create_owner_id};
use engine::observe::{
    MatchStats, emit_line, machine_hash, read_stats, unix_millis, write_record_at,
};
use engine::{Content, ContentDir, MatchConfig};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::bench::BenchFigures;
use crate::cli::{CalibrateOpts, InjectFailure, KeepEvents, SuiteArg};
use crate::report::bands::Bands;
use crate::report::baseline::{self, Identity};
use crate::report::compare::{self, Guard};
use crate::report::{
    ArmReport, BAND_NAMES, BandCheck, BaseCache, BaseReport, CalibrationReport, FlagEntry,
    MEASURES_VERSION, PairInfo, PairingFigures, RED_CARD_ARMS, Recorded, RedCardFigures, RowsInfo,
    RunBuilder, Selection, Suite, SuiteFigures, Units, band_suites,
};
use fixtures::{FIXTURE_SCHEME, FixtureKey, Keyed};
use rows::{Outcome, ROWS_FORMAT, Row, reason};
use run_folder::{Opened, RunIdentity};

/// Matches the single-thread benchmark times after its warm-up.
const BENCH_MATCHES: u32 = 5;

pub fn run(content_dir: Option<&Path>, opts: &CalibrateOpts) -> anyhow::Result<i32> {
    if opts.matches == 0 {
        anyhow::bail!("--matches must be at least 1");
    }
    let dir = ContentDir::resolve(content_dir)?;
    let states = FlagStates::from_settings(&opts.flags);
    if opts.worker {
        let suite = match opts.suite {
            SuiteArg::Equal => Suite::Equal,
            SuiteArg::Strength => Suite::Strength,
            SuiteArg::Formations => Suite::Formations,
            SuiteArg::RedCard => Suite::RedCard,
            SuiteArg::All => anyhow::bail!("a worker plays one suite"),
        };
        let run_dir = opts
            .run_dir
            .as_deref()
            .context("a worker needs its run folder")?;
        return worker::run(&worker::Share {
            content_dir: dir.root(),
            run_dir,
            seed: opts.seed,
            matches: opts.matches,
            minutes: opts.minutes,
            suite,
            shard: opts.shard,
            shards: opts.shards,
            run_millis: opts.run_millis,
            states: &states,
            inject: opts.inject_failure,
            pairings: &opts.pairing_numbers,
            session: &opts.session,
            stop: opts.stop_after_units,
        });
    }
    if opts.baseline.is_some() && opts.pair.is_some() {
        anyhow::bail!(
            "--baseline cannot be used with --pair: a paired run already compares its two arms"
        );
    }

    // Every flag name and state is checked before a worker starts.
    let loaded = crate::content::load(Some(dir.root()), None, None, None)?;
    let arms: Vec<(&str, FlagStates)> = match &opts.pair {
        None => vec![("", states.clone())],
        Some(flag) => {
            if states.0.contains_key(flag) {
                anyhow::bail!("--pair {flag}: a paired flag takes both states; drop its --flag");
            }
            [("off", FlagState::Off), ("on", FlagState::On)]
                .into_iter()
                .map(|(arm, state)| {
                    let mut s = states.clone();
                    s.0.insert(flag.clone(), state);
                    (arm, s)
                })
                .collect()
        }
    };
    for (_, s) in &arms {
        loaded.content.with_flags(s)?;
    }
    let requested = match opts.suite {
        SuiteArg::All => Suite::ALL.to_vec(),
        SuiteArg::Equal => vec![Suite::Equal],
        SuiteArg::Strength => vec![Suite::Strength],
        SuiteArg::Formations => vec![Suite::Formations],
        SuiteArg::RedCard => vec![Suite::RedCard],
    };
    let formation_names: Vec<String> = loaded
        .content
        .tactics
        .formations
        .iter()
        .map(|f| f.name.clone())
        .collect();
    let pairings: Vec<[String; 2]> = fixtures::pairings(formation_names.len())
        .into_iter()
        .map(|p| p.map(|i| formation_names[i].clone()))
        .collect();
    let (suites, selected) = select(&requested, opts, &pairings)?;

    // The baseline is checked before the run folder exists and before any match plays.
    let fixtures_hash = fixtures_hash(&loaded.content.with_flags(&arms[0].1)?, &loaded.teams)?;
    let base = opts
        .baseline
        .as_deref()
        .map(|path| {
            baseline::load(
                path,
                Identity {
                    seed: opts.seed,
                    matches: opts.matches,
                    fixtures_hash: &fixtures_hash,
                },
            )
        })
        .transpose()?;

    let data = data_dir();
    let owner_id = load_or_create_owner_id(&data)?;
    let millis = u64::try_from(unix_millis()).unwrap_or(u64::MAX);
    // The largest selected suite sets how many workers can have work. At most 136
    // pairings, and four red-card arms.
    let most = suites
        .iter()
        .map(|suite| match suite {
            Suite::Formations => opts.matches.saturating_mul(selected.len() as u32),
            Suite::RedCard => opts.matches.saturating_mul(RED_CARD_ARMS.len() as u32),
            Suite::Equal | Suite::Strength => opts.matches,
        })
        .max()
        .unwrap_or(1);
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let jobs = opts
        .jobs
        .unwrap_or(u32::try_from(cores).unwrap_or(u32::MAX))
        .clamp(1, most);
    let bands = Bands::load(&dir)?;
    // nosemgrep: rust.lang.security.current-exe.current-exe -- only used to start worker copies of this program and to hash the build
    let exe = std::env::current_exe().context("cannot find this program to start workers")?;
    let identity = RunIdentity {
        build: run_folder::Build {
            hash: engine::build_hash().to_string(),
            executable_sha256: crate::bisect::cache::file_sha256(&exe)
                .context("cannot read this program to hash it")?,
        },
        content: content_identity(&loaded.content, &dir)?,
        fixtures: fixtures_hash.clone(),
        flags: states
            .0
            .iter()
            .map(|(n, s)| (n.clone(), s.code().to_string()))
            .collect(),
        pair: opts.pair.clone(),
        seed: opts.seed,
        minutes: opts.minutes,
        strength_boost: bands.stronger_team.attribute_boost,
        rng_scheme: engine::rng::STREAM_SCHEME,
        fixture_scheme: FIXTURE_SCHEME.to_string(),
        measures: MEASURES_VERSION,
        registry: bands.schema_version,
    };
    let mut ctx = RunCtx {
        dir: &dir,
        opts,
        loaded: &loaded,
        bands,
        exe,
        jobs,
        selection: Selection {
            suites: suites.iter().map(|s| s.code().to_string()).collect(),
            pairings: if opts.pairings.is_empty() {
                Vec::new()
            } else {
                selected
                    .iter()
                    .map(|&p| format!("{} v {}", pairings[p][0], pairings[p][1]))
                    .collect()
            },
            bands: opts.bands.clone(),
        },
        suites,
        pairings,
        formation_names,
        selected,
        millis,
        run_id: String::new(),
        session: format!("{millis}-{}", std::process::id()),
    };

    // The old engine's results come before the run folder exists and before any match of
    // the changed engine plays, so a base that cannot build stops the run with nothing done.
    let old = ctx.old_engine(&data)?;

    let fresh_id = format!("calib-{:016x}-{millis}", opts.seed);
    let run_dir = opts
        .out
        .clone()
        .unwrap_or_else(|| data.join("runs").join(&fresh_id));
    let session = run_folder::open(&run_dir, &identity, &fresh_id, millis)?;
    ctx.millis = session.millis;
    ctx.run_id = session.run_id.clone();
    let run_id = session.run_id.clone();
    let arm_dirs: Vec<PathBuf> = arms
        .iter()
        .map(|(name, _)| {
            if name.is_empty() {
                run_dir.clone()
            } else {
                run_dir.join("arms").join(name)
            }
        })
        .collect();
    let (total, finished_before) = ctx.progress(&arm_dirs)?;
    match &session.opened {
        Opened::New => {}
        Opened::Resumed => eprintln!(
            "resuming run {run_id}: {finished_before} of {total} fixtures done, {} to play",
            total - finished_before
        ),
        Opened::Superseded {
            old_run_id,
            differences,
        } => eprintln!(
            "the run folder held run {old_run_id} of another identity ({}); its files moved \
             to {}/{old_run_id}/, and a new run {run_id} starts",
            differences.join("; "),
            run_folder::SUPERSEDED
        ),
    }

    let started = Instant::now();
    let stopped = || {
        eprintln!(
            "stopped after {} work units; run the same command again to resume",
            opts.stop_after_units.unwrap_or(0)
        );
        Ok(1)
    };
    let mut played: Vec<Arm> = Vec::with_capacity(arms.len());
    if opts.worker_processes {
        let mut budget = opts.stop_after_units;
        for ((_, s), arm_dir) in arms.iter().zip(&arm_dirs) {
            match ctx.play_arm(s, arm_dir, &mut budget)? {
                Some(arm) => played.push(arm),
                None => return stopped(),
            }
        }
    } else {
        let inputs = arms
            .iter()
            .zip(&arm_dirs)
            .map(|((_, s), dir)| {
                std::fs::create_dir_all(dir)
                    .with_context(|| format!("cannot create the run folder runs/{run_id}"))?;
                Ok(runner::ArmInput {
                    dir,
                    content: loaded.content.with_flags(s)?,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let ran = runner::run_arms(&ctx, &inputs, &owner_id)?;
        if ran.stopped {
            return stopped();
        }
        for (((_, s), arm_dir), wall) in arms.iter().zip(&arm_dirs).zip(&ran.wall) {
            let events = count_files(&arm_dir.join("events"));
            let folded = ctx.fold_rows(arm_dir)?;
            played.push(ctx.finish_arm(s, wall, ran.total_ms, 0, folded, (events, events))?);
        }
    }
    let first = &played[0];
    let (_, finished_after) = ctx.progress(&arm_dirs)?;
    let units = Units {
        total,
        finished_before,
        played: finished_after.saturating_sub(finished_before),
    };
    let results_digest = if opts.worker_processes {
        let digest_input: Vec<(String, Vec<(FixtureKey, Value)>)> = arms
            .iter()
            .zip(&played)
            .map(|((name, _), arm)| (name.to_string(), arm.records.clone()))
            .collect();
        run_folder::results_digest(&digest_input)
    } else {
        rows::rows_digest(
            arms.iter()
                .zip(&played)
                .map(|((name, _), arm)| (*name, &arm.rows)),
        )
    };
    // The rows of every arm: their files, and the matches with a full recording.
    let rows_info = played
        .iter()
        .filter_map(|a| a.rows_info)
        .reduce(|a, b| RowsInfo {
            format: a.format,
            files: a.files + b.files,
            rows: a.rows + b.rows,
            recorded: Recorded {
                matches: a.recorded.matches + b.recorded.matches,
                sample: a.recorded.sample + b.recorded.sample,
                error: a.recorded.error + b.recorded.error,
                violation: a.recorded.violation + b.recorded.violation,
                extreme: a.recorded.extreme + b.recorded.extreme,
                all: a.recorded.all + b.recorded.all,
            },
        });

    let written = loaded.content.written_tuning();
    let resolved = effective(written, &states)?;
    let flags: Vec<FlagEntry> = resolved
        .iter()
        .map(|(name, (state, source))| {
            let paired = opts.pair.as_deref() == Some(name.as_str());
            FlagEntry {
                name: name.clone(),
                owner: written.flags[name].owner.clone(),
                state: if paired { "paired" } else { state.code() },
                source: if paired { "cli" } else { source.code() },
            }
        })
        .collect();

    // A failed worker is not an engine error in this process; it has its own error keys.
    let worker_failed = played.iter().any(|a| a.workers_failed > 0);
    let mut report = CalibrationReport {
        owner_id,
        run_id: run_id.clone(),
        seed: opts.seed,
        content_hash: first.content_hash.clone(),
        fixtures_hash,
        fixtures_scheme: FIXTURE_SCHEME,
        identity: serde_json::to_value(&identity)?,
        outcome: if worker_failed { "error" } else { "success" },
        error_type: worker_failed.then_some("worker"),
        error_code: worker_failed.then_some("worker-failed"),
        error_retriable: worker_failed.then_some(false),
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        matches: opts.matches,
        minutes: opts.minutes,
        jobs,
        suites: suite_codes(&first.figures),
        wall_ms: first.wall_ms.clone(),
        bands: first.checks.clone(),
        formations: first.formations.clone(),
        selection: ctx.selection.clone(),
        red_card: first.red_card.clone(),
        baseline: None,
        diff: None,
        pass: first.pass,
        bench_matches: BENCH_MATCHES,
        match_wall_ms: first.bench.match_wall_ms,
        ticks_per_match: first.bench.ticks_per_match,
        cpu_us_per_tick: first.bench.cpu_us_per_tick,
        cpu_ms: first.bench.cpu_ms,
        peak_mem_mb: first.bench.peak_mem_mb,
        change_never_applied: first.change_never_applied,
        change_expired_at_full_time: first.change_expired_at_full_time,
        match_without_stats: first.match_without_stats,
        match_panicked: first.match_panicked,
        violations: first.violations,
        events_written: first.events_written,
        events_kept: first.events_kept,
        runner: if opts.worker_processes {
            "processes"
        } else {
            "threads"
        },
        rows: rows_info,
        machine_hash: machine_hash(),
        cpu_model: crate::bench::cpu_model(),
        flags,
        pair: None,
        arms: None,
        compare: None,
        verdict: None,
        units,
        results_digest,
        base: old,
    };
    if let Some(base) = &base {
        let diff = baseline::diff(base, &report.content_hash, &report.bands);
        eprint!("{}", baseline::render_table(&base.info, &diff));
        tracing::info!(
            signal = "calibrate.diff",
            run.id = %run_id,
            baseline = %base.info.run_id,
            changes = diff.changes,
            noise = diff.noise
        );
        report.baseline = Some(base.info.clone());
        report.diff = Some(diff);
    }

    let code = match (&opts.pair, played.as_slice()) {
        (Some(flag), [off, on]) => {
            let rows = compare::compare(&off.checks, &on.checks);
            let verdict = compare::verdict(&rows, &off.guard(), &on.guard());
            let def = &written.flags[flag];
            report.pair = Some(PairInfo {
                flag: flag.clone(),
                owner: def.owner.clone(),
                hypothesis: def.hypothesis.clone(),
                removal_condition: def.removal_condition.clone(),
                pinned: states
                    .0
                    .iter()
                    .map(|(n, s)| (n.clone(), s.code()))
                    .collect(),
            });
            report.arms = Some(BTreeMap::from([
                ("off".to_string(), off.report()),
                ("on".to_string(), on.report()),
            ]));
            eprint!("{}", compare::render_table(flag, &rows, verdict));
            tracing::info!(
                signal = "calibrate.pair",
                run.id = %run_id,
                flag = %flag,
                verdict = verdict.code()
            );
            report.compare = Some(rows);
            report.verdict = Some(verdict);
            // The verdict is the comparison's answer; the exit code says whether both arms
            // can be trusted.
            if off.trusted() && on.trusted() { 0 } else { 2 }
        }
        _ => {
            if first.pass {
                0
            } else {
                2
            }
        }
    };
    write_record_at(
        &run_dir.join("report.json"),
        &format!("runs/{run_id}/report.json"),
        &report,
    )?;
    emit_line(&report)?;
    Ok(code)
}

/// What every arm of a run shares.
struct RunCtx<'a> {
    dir: &'a ContentDir,
    opts: &'a CalibrateOpts,
    loaded: &'a crate::content::Loaded,
    bands: Bands,
    exe: PathBuf,
    jobs: u32,
    selection: Selection,
    suites: Vec<Suite>,
    /// The formation names of every pairing of the formations suite.
    pairings: Vec<[String; 2]>,
    /// The formations of the tactics file, in order.
    formation_names: Vec<String>,
    /// The numbers of the pairings this run plays, in order.
    selected: Vec<usize>,
    /// The run's start, from its `run.json`: part of every match identifier.
    millis: u64,
    run_id: String,
    /// This invocation: the ledger files its workers write carry it.
    session: String,
}

/// The suites a run plays, and the numbers of the formation pairings it plays: `--suite`
/// narrowed to the suites that check the `--band` names, and to the formations suite when
/// `--pairing` names a pairing. Every name is checked before any work.
fn select(
    requested: &[Suite],
    opts: &CalibrateOpts,
    pairings: &[[String; 2]],
) -> anyhow::Result<(Vec<Suite>, Vec<usize>)> {
    let mut suites = requested.to_vec();
    for band in &opts.bands {
        if band_suites(band).is_none() {
            anyhow::bail!(
                "--band {band}: no such band; the bands are {}",
                BAND_NAMES.join(", ")
            );
        }
    }
    if !opts.bands.is_empty() {
        suites.retain(|s| {
            opts.bands
                .iter()
                .any(|b| band_suites(b).is_some_and(|owners| owners.contains(s)))
        });
        if suites.is_empty() {
            anyhow::bail!(
                "--band {}: no suite of --suite {} checks it",
                opts.bands.join(", "),
                codes(requested)
            );
        }
    }
    let mut selected: Vec<usize> = (0..pairings.len()).collect();
    if !opts.pairings.is_empty() {
        if !suites.contains(&Suite::Formations) {
            anyhow::bail!(
                "--pairing needs the formations suite, but the run plays {}; \
                 use --suite formations",
                codes(&suites)
            );
        }
        suites.retain(|s| *s == Suite::Formations);
        selected = opts
            .pairings
            .iter()
            .map(|name| pairing_number(name, pairings))
            .collect::<anyhow::Result<_>>()?;
        selected.sort_unstable();
        selected.dedup();
    }
    Ok((suites, selected))
}

fn codes(suites: &[Suite]) -> String {
    suites
        .iter()
        .map(|s| s.code())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The number of the pairing `name` names, as `A v B` in either order.
fn pairing_number(name: &str, pairings: &[[String; 2]]) -> anyhow::Result<usize> {
    let sides: Vec<&str> = name.split(" v ").map(str::trim).collect();
    let found = match sides.as_slice() {
        [a, b] => pairings
            .iter()
            .position(|p| (p[0] == *a && p[1] == *b) || (p[0] == *b && p[1] == *a)),
        _ => None,
    };
    found.ok_or_else(|| {
        let names: Vec<String> = pairings
            .iter()
            .map(|p| format!("{} v {}", p[0], p[1]))
            .collect();
        anyhow::anyhow!(
            "--pairing {name}: no such pairing; the pairings are {}",
            names.join(", ")
        )
    })
}

/// SHA-256, as 12 hex characters, over the inputs that decide a run's fixtures: the
/// attributes, rules and tactics content, the generator block after the flag states, and
/// the two default clubs. Parsed values are hashed, so a change of layout alone does not
/// change it. Tuning values outside the generator block and flag states that leave the
/// generator alone do not change it: they are the change a baseline diff measures.
pub fn fixtures_hash(content: &Content, teams: &[TeamFile; 2]) -> anyhow::Result<String> {
    let mut hasher = Sha256::new();
    for part in [
        serde_json::to_vec(&content.attributes)?,
        serde_json::to_vec(&content.rules)?,
        serde_json::to_vec(&content.tactics)?,
        serde_json::to_vec(&content.tuning.generator)?,
        serde_json::to_vec(&teams[0])?,
        serde_json::to_vec(&teams[1])?,
    ] {
        hasher.update(part.len().to_le_bytes());
        hasher.update(part);
    }
    Ok(hex12(&hasher.finalize()))
}

/// The outcome of one arm: every suite played in workers under one set of flag states,
/// folded and checked, and the single-thread figure under the same states.
struct Arm {
    content_hash: String,
    flags_on: Vec<String>,
    figures: BTreeMap<Suite, SuiteFigures>,
    formations: Vec<PairingFigures>,
    red_card: Option<RedCardFigures>,
    checks: Vec<BandCheck>,
    wall_ms: BTreeMap<String, u64>,
    pass: bool,
    workers_failed: u32,
    change_never_applied: u32,
    change_expired_at_full_time: u32,
    match_without_stats: u32,
    match_panicked: u32,
    violations: usize,
    events_written: u32,
    events_kept: u32,
    bench: BenchFigures,
    /// The worker processes: every match's statistics record, by fixture key, for the
    /// results digest.
    records: Vec<(FixtureKey, Value)>,
    /// The threads: every planned match's row, by fixture key, for the results digest.
    rows: BTreeMap<FixtureKey, Row>,
    rows_info: Option<RowsInfo>,
}

impl Arm {
    /// No worker failed, no match panicked, no record is missing, no change was left
    /// unapplied, and the validator found nothing.
    fn trusted(&self) -> bool {
        self.workers_failed == 0
            && self.match_panicked == 0
            && self.match_without_stats == 0
            && self.change_never_applied == 0
            && self.violations == 0
    }

    fn guard(&self) -> Guard {
        Guard {
            dark_paths: self.match_without_stats
                + self.change_never_applied
                + self.workers_failed
                + self.match_panicked,
            violations: self.violations,
            match_wall_ms: self.bench.match_wall_ms,
        }
    }

    fn report(&self) -> ArmReport {
        ArmReport {
            content_hash: self.content_hash.clone(),
            flags_on: self.flags_on.clone(),
            suites: suite_codes(&self.figures),
            wall_ms: self.wall_ms.clone(),
            bands: self.checks.clone(),
            formations: self.formations.clone(),
            pass: self.pass,
            change_never_applied: self.change_never_applied,
            change_expired_at_full_time: self.change_expired_at_full_time,
            match_without_stats: self.match_without_stats,
            match_panicked: self.match_panicked,
            violations: self.violations,
            workers_failed: self.workers_failed,
            match_wall_ms: self.bench.match_wall_ms,
            ticks_per_match: self.bench.ticks_per_match,
            cpu_us_per_tick: self.bench.cpu_us_per_tick,
            events_written: self.events_written,
            events_kept: self.events_kept,
        }
    }
}

/// The finished matches of a run folder, folded.
struct Folded {
    builder: RunBuilder,
    records: Vec<(FixtureKey, Value)>,
    rows: BTreeMap<FixtureKey, Row>,
    rows_info: Option<RowsInfo>,
    /// Matches that panicked.
    panicked: u32,
}

impl RunCtx<'_> {
    /// Every match `suite` plays, in fixture order, with its key and engine seed.
    fn keyed(&self, suite: Suite) -> Vec<Keyed> {
        fixtures::keyed(
            suite,
            self.opts.seed,
            self.opts.matches,
            &self.formation_names,
            &self.selected,
        )
    }

    /// The planned fixtures of every arm and suite, and those the arms' ledgers have
    /// finished with the planned engine seed.
    fn progress(&self, arm_dirs: &[PathBuf]) -> anyhow::Result<(u32, u32)> {
        let (mut total, mut done) = (0usize, 0usize);
        for arm_dir in arm_dirs {
            let finished = run_folder::finished(arm_dir, None)?;
            for &suite in &self.suites {
                for k in self.keyed(suite) {
                    total += 1;
                    if finished
                        .get(&k.key)
                        .is_some_and(|d| d.seed == k.engine_seed)
                    {
                        done += 1;
                    }
                }
            }
        }
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        Ok((count(total), count(done)))
    }

    /// The old engine's band rows on the run's fixtures, from its cache, played first where
    /// the cache misses; `None` without `--base` or `--base-binary`.
    fn old_engine(&self, data: &Path) -> anyhow::Result<Option<BaseReport>> {
        let opts = self.opts;
        let source = match (&opts.base, &opts.base_binary) {
            (Some(rev), _) => base::Source::Rev(rev.clone()),
            (None, Some(path)) => base::Source::Binary(path.clone()),
            (None, None) => return Ok(None),
        };
        let planned: Vec<(FixtureKey, u64)> = self
            .suites
            .iter()
            .flat_map(|&s| self.keyed(s))
            .map(|k| (k.key, k.engine_seed))
            .collect();
        let old = base::run(&base::Request {
            source,
            data,
            content_dir: self.dir.root(),
            seed: opts.seed,
            minutes: opts.minutes,
            matches: opts.matches,
            jobs: self.jobs,
            suites: &self.suites,
            pairings: &self.selection.pairings,
            planned: &planned,
            fail_build: opts.inject_failure == Some(InjectFailure::BaseBuild),
        })?;
        eprintln!(
            "old engine {}: {} results from the cache, {} played",
            old.build_id, old.hits, old.played
        );
        let folded = self.fold_rows(&old.dir)?;
        let figures = folded.builder.figures();
        let formations = folded.builder.pairing_figures();
        let mut checks = folded
            .builder
            .checks(&figures, &formations, &BTreeMap::new());
        if !opts.bands.is_empty() {
            checks.retain(|c| opts.bands.contains(&c.band));
        }
        checks
            .sort_by(|a, b| (&a.suite, &a.band, &a.pairing).cmp(&(&b.suite, &b.band, &b.pairing)));
        Ok(Some(BaseReport {
            source: old.source,
            rev: old.rev,
            commit: old.commit,
            build_id: old.build_id,
            content_hash: old.content_hash,
            cache: BaseCache {
                hits: old.hits,
                played: old.played,
            },
            bands: checks,
        }))
    }

    /// A builder with every selected suite planned.
    fn planned_builder(&self) -> RunBuilder {
        let opts = self.opts;
        let mut builder = RunBuilder::new(self.bands.clone());
        builder.minutes = opts.minutes;
        for &suite in &self.suites {
            match suite {
                Suite::Formations => builder.plan_pairings(
                    opts.matches,
                    self.selected
                        .iter()
                        .map(|&p| self.pairings[p].clone())
                        .collect(),
                ),
                Suite::RedCard => builder.plan(
                    suite,
                    opts.matches.saturating_mul(RED_CARD_ARMS.len() as u32),
                ),
                Suite::Equal | Suite::Strength => builder.plan(suite, opts.matches),
            }
        }
        builder
    }

    /// Folds the record of the match `k` of `suite` in; `true` when the builder's rule
    /// calls it an outlier.
    fn add(&self, builder: &mut RunBuilder, suite: Suite, k: &Keyed, stats: MatchStats) -> bool {
        match (k.pairing, k.arm) {
            (Some((pairing, side)), _) => {
                let place = self
                    .selected
                    .iter()
                    .position(|&p| p == pairing)
                    .unwrap_or(0);
                builder.add_pairing(stats, place, side)
            }
            (None, Some(arm)) => builder.add_red_card(stats, arm),
            (None, None) => builder.add(suite, stats, k.boosted),
        }
    }

    /// Folds every planned match the ledger of `dir` finished into a builder, from its
    /// statistics file (the worker processes); a planned match with no ledger entry or no
    /// statistics file is the dark path. With `prune_events`, the event file of a match
    /// that is not an outlier is removed.
    fn fold(&self, dir: &Path, prune_events: bool) -> anyhow::Result<Folded> {
        let done = run_folder::finished(dir, None)?;
        let events_dir = dir.join("events");
        let mut builder = self.planned_builder();
        let mut records = Vec::new();
        for &suite in &self.suites {
            for k in self.keyed(suite) {
                let Some(entry) = done.get(&k.key).filter(|d| d.seed == k.engine_seed) else {
                    builder.add_missing();
                    continue;
                };
                let id = &entry.match_id;
                match read_stats(&dir.join("stats").join(format!("{id}.json"))) {
                    Ok(stats) => {
                        records.push((k.key, serde_json::to_value(&stats)?));
                        let outlier = self.add(&mut builder, suite, &k, stats);
                        if prune_events && !outlier {
                            let _ = std::fs::remove_file(events_dir.join(format!("{id}.jsonl")));
                        }
                    }
                    // A missing record is the dark path; its event file, if any, is kept.
                    Err(_) => builder.add_missing(),
                }
            }
        }
        Ok(Folded {
            builder,
            records,
            rows: BTreeMap::new(),
            rows_info: None,
            panicked: 0,
        })
    }

    /// Folds every planned match the ledger of `dir` finished into a builder, from its
    /// compact row, in fixture order; a planned match with no ledger entry or no row is the
    /// dark path. Each match's band record is rebuilt from its row with the statistics
    /// files' rounding, so the bands equal those of [`Self::fold`] on the same matches.
    fn fold_rows(&self, dir: &Path) -> anyhow::Result<Folded> {
        let done = run_folder::finished(dir, None)?;
        let read = rows::read_rows(dir, &done)?;
        let mut builder = self.planned_builder();
        let mut planned = BTreeMap::new();
        let mut recorded = Recorded::default();
        let mut panicked = 0;
        for &suite in &self.suites {
            let mut outliers = 0;
            for k in self.keyed(suite) {
                let row = done
                    .get(&k.key)
                    .filter(|d| d.seed == k.engine_seed)
                    .and_then(|_| read.rows.get(&k.key));
                let Some(row) = row else {
                    builder.add_missing();
                    continue;
                };
                panicked += u32::from(row.outcome == Outcome::Panic);
                let has = |bit: u8| u32::from(row.reasons & bit != 0);
                recorded.matches += u32::from(row.reasons != 0);
                recorded.sample += has(reason::SAMPLE);
                recorded.error += has(reason::ERROR);
                recorded.violation += has(reason::VIOLATION);
                recorded.extreme += has(reason::EXTREME);
                recorded.all += has(reason::ALL);
                outliers += has(reason::OUTLIER);
                self.add(&mut builder, suite, &k, row.band_record());
                planned.insert(k.key, *row);
            }
            builder.set_outliers(suite, outliers);
        }
        let rows_info = RowsInfo {
            format: ROWS_FORMAT,
            files: read.files,
            rows: u32::try_from(planned.len()).unwrap_or(u32::MAX),
            recorded,
        };
        Ok(Folded {
            builder,
            records: Vec::new(),
            rows: planned,
            rows_info: Some(rows_info),
            panicked,
        })
    }

    /// Plays every suite's unfinished fixtures into `arm_dir` with `states`, then times the
    /// default teams on one thread under the same states. `budget`, the stop seam, counts
    /// the work units left to play; `None` comes back when it ran out.
    fn play_arm(
        &self,
        states: &FlagStates,
        arm_dir: &Path,
        budget: &mut Option<u32>,
    ) -> anyhow::Result<Option<Arm>> {
        let opts = self.opts;
        std::fs::create_dir_all(arm_dir)
            .with_context(|| format!("cannot create the run folder runs/{}", self.run_id))?;
        let settings: Vec<FlagSetting> = states
            .0
            .iter()
            .map(|(name, &state)| FlagSetting {
                name: name.clone(),
                state,
            })
            .collect();
        let started = Instant::now();
        let mut wall = BTreeMap::new();
        let mut workers_failed = 0u32;
        for &suite in &self.suites {
            let suite_started = Instant::now();
            let remaining = worker::remaining(arm_dir, &self.session, self.keyed(suite))?;
            let unit = run_folder::unit_size(remaining.len(), self.jobs);
            let units = u32::try_from(remaining.len().div_ceil(unit)).unwrap_or(u32::MAX);
            if !remaining.is_empty() {
                let children = (0..self.jobs)
                    .map(|shard| {
                        worker_command(self, suite, shard, arm_dir, &settings, *budget)
                            .spawn()
                            .context("cannot start a calibration worker")
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                for mut child in children {
                    let status = child.wait().context("a calibration worker was lost")?;
                    if !status.success() {
                        workers_failed += 1;
                        tracing::warn!(
                            signal = "calibrate.worker_failed",
                            run.id = %self.run_id,
                            suite = suite.code(),
                            code = ?status.code()
                        );
                    }
                }
            }
            if let Some(left) = *budget {
                if left < units {
                    return Ok(None);
                }
                *budget = Some(left - units);
            }
            let wall_ms = u64::try_from(suite_started.elapsed().as_millis()).unwrap_or(u64::MAX);
            tracing::info!(
                signal = "calibrate.suite",
                run.id = %self.run_id,
                suite = suite.code(),
                matches = remaining.len(),
                wall_ms,
                jobs = self.jobs
            );
            wall.insert(suite, wall_ms);
        }
        let suites_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let events_dir = arm_dir.join("events");
        let events_written = count_files(&events_dir);
        let folded = self.fold(arm_dir, opts.keep_events == KeepEvents::Outliers)?;
        let events_kept = count_files(&events_dir);
        self.finish_arm(
            states,
            &wall,
            suites_ms,
            workers_failed,
            folded,
            (events_written, events_kept),
        )
        .map(Some)
    }

    /// Judges an arm's folded matches against the bands, given each suite's wall time and
    /// the whole run's, and times the default teams on one thread under the arm's states.
    fn finish_arm(
        &self,
        states: &FlagStates,
        wall: &BTreeMap<Suite, u64>,
        suites_ms: u64,
        workers_failed: u32,
        folded: Folded,
        (events_written, events_kept): (u32, u32),
    ) -> anyhow::Result<Arm> {
        let opts = self.opts;
        // The single-thread figure, on the default teams, once every match is played.
        let content = self.loaded.content.with_flags(states)?;
        let [team_a, team_b] = &self.loaded.teams;
        let config = MatchConfig::new(opts.seed, opts.minutes, &content, [team_a, team_b])?;
        let bench = crate::bench::measure(&config, None, BENCH_MATCHES)?;

        let Folded {
            builder,
            records,
            rows,
            rows_info,
            panicked: match_panicked,
        } = folded;
        let figures = builder.figures();
        let formations = builder.pairing_figures();
        let mut checks = builder.checks(&figures, &formations, wall);
        // `--band` judges only the named bands; each suite's time budget stays.
        if !opts.bands.is_empty() {
            checks.retain(|c| c.band == "wall_ms" || opts.bands.contains(&c.band));
        }
        let change_never_applied = builder.change_never_applied();
        let match_without_stats = builder.missing;
        for (counter, value) in [
            ("darkpath.change_never_applied", change_never_applied),
            ("darkpath.match_without_stats", match_without_stats),
            ("darkpath.match_panicked", match_panicked),
        ] {
            if value > 0 {
                tracing::warn!(signal = "calibrate.darkpath", run.id = %self.run_id, counter, value);
            }
        }
        let pass = checks.iter().all(|c| c.pass)
            && change_never_applied == 0
            && match_without_stats == 0
            && match_panicked == 0
            && workers_failed == 0;
        checks
            .sort_by(|a, b| (&a.suite, &a.band, &a.pairing).cmp(&(&b.suite, &b.band, &b.pairing)));
        for c in checks.iter().filter(|c| !c.pass) {
            tracing::warn!(
                signal = "calibrate.band_failed",
                run.id = %self.run_id,
                suite = %c.suite,
                band = %c.band,
                pairing = c.pairing.as_deref(),
                value = c.value,
                lo = c.lo,
                hi = c.hi
            );
        }
        let mut wall_ms: BTreeMap<String, u64> = wall
            .iter()
            .map(|(s, &ms)| (s.code().to_string(), ms))
            .collect();
        wall_ms.insert("total".into(), suites_ms);
        Ok(Arm {
            content_hash: config.content_hash.clone(),
            flags_on: content.flags.names().to_vec(),
            figures,
            formations,
            red_card: builder.red_card_figures(),
            checks,
            wall_ms,
            pass,
            workers_failed,
            change_never_applied,
            change_expired_at_full_time: builder.change_expired_at_full_time(),
            match_without_stats,
            match_panicked,
            violations: builder.violations(),
            events_written,
            events_kept,
            bench,
            records,
            rows,
            rows_info,
        })
    }
}

fn suite_codes(figures: &BTreeMap<Suite, SuiteFigures>) -> BTreeMap<String, SuiteFigures> {
    figures
        .iter()
        .map(|(s, f)| (s.code().to_string(), f.clone()))
        .collect()
}

/// SHA-256 over the content files' digest and the slot file: the content part of a run's
/// identity. The bands file is left out: its ranges are judged again, never replayed.
fn content_identity(content: &Content, dir: &ContentDir) -> anyhow::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(content.digest);
    let slots = dir.path(engine::data::SLOTS_FILE);
    if slots.is_file() {
        hasher.update(
            std::fs::read(&slots).with_context(|| format!("cannot read {}", slots.display()))?,
        );
    }
    Ok(stream::record::hex(&hasher.finalize()))
}

/// The command line of one worker. Workers log warnings and errors only unless `SM_LOG`
/// says otherwise, and their standard output is discarded. The flag states the parent
/// resolved are passed on, so every worker of an arm plays under the same states.
fn worker_command(
    ctx: &RunCtx<'_>,
    suite: Suite,
    shard: u32,
    run_dir: &Path,
    settings: &[FlagSetting],
    stop: Option<u32>,
) -> Command {
    let (opts, content, shards, millis) = (ctx.opts, ctx.dir.root(), ctx.jobs, ctx.millis);
    let mut cmd = Command::new(&ctx.exe);
    cmd.arg("--content-dir")
        .arg(content)
        .arg("calibrate")
        .arg("--worker")
        .args(["--seed", &opts.seed.to_string()])
        .args(["--matches", &opts.matches.to_string()])
        .args(["--minutes", &opts.minutes.to_string()])
        .args(["--suite", suite.code()])
        .args(["--shard", &shard.to_string()])
        .args(["--shards", &shards.to_string()])
        .arg("--run-dir")
        .arg(run_dir)
        .args(["--run-millis", &millis.to_string()])
        .args(["--session", &ctx.session])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if suite == Suite::Formations && ctx.selected.len() < ctx.pairings.len() {
        let numbers: Vec<String> = ctx.selected.iter().map(usize::to_string).collect();
        cmd.arg("--pairing-numbers").arg(numbers.join(","));
    }
    if let Some(stop) = stop {
        cmd.args(["--stop-after-units", &stop.to_string()]);
    }
    // The match, panic and worker seams reach the first worker only.
    if let (
        Some(inject @ (InjectFailure::Match | InjectFailure::Worker | InjectFailure::Panic)),
        0,
    ) = (opts.inject_failure, shard)
    {
        cmd.args(["--inject-failure", inject.code()]);
    }
    for setting in settings {
        cmd.arg("--flag").arg(setting.to_string());
    }
    if std::env::var_os("SM_LOG").is_none() {
        cmd.env("SM_LOG", "warn");
    }
    cmd
}

/// Files in `dir`, or 0 when it does not exist.
fn count_files(dir: &Path) -> u32 {
    std::fs::read_dir(dir).map_or(0, |entries| {
        let n = entries
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .count();
        u32::try_from(n).unwrap_or(u32::MAX)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> (Content, [TeamFile; 2]) {
        let dir = ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
        let content = Content::load(&dir).unwrap();
        let teams = [engine::data::TEAM_A_FILE, engine::data::TEAM_B_FILE]
            .map(|f| content.load_team(&dir, &dir.path(f)).unwrap().value);
        (content, teams)
    }

    #[test]
    fn the_fixtures_hash_follows_the_fixtures_and_not_the_tuning() {
        let (content, teams) = shipped();
        let base = fixtures_hash(&content, &teams).unwrap();
        assert_eq!(base.len(), 12);
        assert_eq!(base, fixtures_hash(&content, &teams).unwrap(), "stable");

        let mut tactics = content.clone();
        tactics.tactics.formations[0].name.push('x');
        assert_ne!(
            base,
            fixtures_hash(&tactics, &teams).unwrap(),
            "a formation"
        );

        let mut generator = content.clone();
        generator.tuning.generator.squad_size += 1;
        assert_ne!(
            base,
            fixtures_hash(&generator, &teams).unwrap(),
            "the generator"
        );

        let mut club = teams.clone();
        club[1].club.name.push('x');
        assert_ne!(
            base,
            fixtures_hash(&content, &club).unwrap(),
            "a default club"
        );

        let mut tuning = content.clone();
        tuning.tuning.engine.red_base += 0.01;
        assert_eq!(
            base,
            fixtures_hash(&tuning, &teams).unwrap(),
            "an engine value"
        );
    }

    #[test]
    fn a_pairing_is_named_in_either_order() {
        let names = vec![
            ["4-4-2".to_string(), "4-4-2".to_string()],
            ["4-4-2".to_string(), "4-4-1-1".to_string()],
        ];
        assert_eq!(pairing_number("4-4-1-1 v 4-4-2", &names).unwrap(), 1);
        assert_eq!(pairing_number("4-4-2 v 4-4-1-1", &names).unwrap(), 1);
        assert_eq!(pairing_number("4-4-2 v 4-4-2", &names).unwrap(), 0);
        let err = pairing_number("4-4-2 v 9-9-9", &names)
            .unwrap_err()
            .to_string();
        assert!(err.contains("no such pairing"), "{err}");
        assert!(err.contains("4-4-2 v 4-4-1-1"), "{err}");
    }
}
