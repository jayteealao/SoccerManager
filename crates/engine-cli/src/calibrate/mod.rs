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
//! The bands come from the band registry, `realism-bands.json`. A run without an old engine
//! judges each band by its range. `--base <REV>` makes it a change run: the old engine's
//! results on the same fixtures come from a cache of its rows (the hidden `--base-binary
//! <EXE>` names a ready executable instead), a pilot of 200 matches per suite sets each
//! suite's power target (`--matches` is the cap), the run grows to it, and one paired max-t
//! bootstrap gives each band pass, fail or not sure, and one joint verdict. The target is
//! kept in `target.json`; a run into a finished folder plays nothing and judges again from
//! the stored rows. Every run reports the time and memory of each stage.

pub mod base;
pub mod fixtures;
pub mod rows;
pub mod rules;
pub mod run_folder;
pub mod runner;
pub mod stages;
pub mod worker;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use engine::data::{TeamFile, hex12};
use engine::flags::{FlagState, FlagStates, effective};
use engine::observe::identity::{data_dir, load_or_create_owner_id};
use engine::observe::{MatchStats, emit_line, machine_hash, unix_millis, write_record_at};
use engine::{Content, ContentDir, MatchConfig};
use sha2::{Digest, Sha256};

use crate::bench::BenchFigures;
use crate::cli::{CalibrateOpts, InjectFailure, SuiteArg};
use crate::report::bands::{REGISTRY_VERSION, Registry};
use crate::report::baseline::{self, Identity};
use crate::report::compare::{self, Guard};
use crate::report::verdict;
use crate::report::{
    ArmReport, BandCheck, BaseCache, BaseReport, CalibrationReport, FlagEntry, MEASURES_VERSION,
    PairInfo, PairingFigures, PowerInfo, RED_CARD_ARMS, Recorded, RedCardFigures, RegistryInfo,
    RowsInfo, RunBuilder, Selection, Suite, SuiteFigures, Units,
};
use fixtures::{FIXTURE_SCHEME, FixtureKey, Keyed};
use rows::{Outcome, ROWS_FORMAT, Row, reason};
use run_folder::{Opened, RunIdentity, TARGET_VERSION, Target};
use stages::Stage;

/// Matches the single-thread benchmark times after its warm-up.
const BENCH_MATCHES: u32 = 5;
/// Set by a change run on the old engine's own `calibrate`: no verdict or stage table.
pub const QUIET_ENV: &str = "SM_CALIBRATE_QUIET";

pub fn run(content_dir: Option<&Path>, opts: &CalibrateOpts) -> anyhow::Result<i32> {
    let run_started = Instant::now();
    if opts.matches == 0 {
        anyhow::bail!("--matches must be at least 1");
    }
    let dir = ContentDir::resolve(content_dir)?;
    let states = FlagStates::from_settings(&opts.flags);
    if opts.baseline.is_some() && opts.pair.is_some() {
        anyhow::bail!(
            "--baseline cannot be used with --pair: a paired run already compares its two arms"
        );
    }

    // Every flag name and state is checked before any match plays.
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
    let registry = Registry::load(&dir)?;
    let (suites, selected) = select(&requested, opts, &pairings, &registry)?;

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
    // The largest selected suite sets how many threads can have work. At most 136
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
    let bands = registry;
    // nosemgrep: rust.lang.security.current-exe.current-exe -- only used to hash the build for the run's identity
    let exe = std::env::current_exe().context("cannot find this program to hash it")?;
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
        registry: REGISTRY_VERSION,
    };
    let change = opts.pair.is_none() && (opts.base.is_some() || opts.base_binary.is_some());
    let fresh_id = format!("calib-{:016x}-{millis}", opts.seed);
    let run_dir = opts
        .out
        .clone()
        .unwrap_or_else(|| data.join("runs").join(&fresh_id));
    // A run folder of the same identity keeps its target: a change run resumes to it, and
    // the bands it was last judged with name what a re-judgement changed.
    let known = run_folder::holds(&run_dir, &identity)
        .then(|| run_folder::read_target(&run_dir))
        .flatten();
    let pilot = opts.pilot.clamp(1, opts.matches);
    let known_change = known.as_ref().filter(|t| t.change);
    let matches: BTreeMap<Suite, u32> = suites
        .iter()
        .map(|&s| {
            let n = match (change, known_change) {
                (true, Some(t)) => t
                    .target
                    .get(s.code())
                    .copied()
                    .unwrap_or(pilot)
                    .min(opts.matches),
                (true, None) => pilot,
                (false, _) => opts.matches,
            };
            (s, n)
        })
        .collect();
    let mut ctx = RunCtx {
        dir: &dir,
        opts,
        loaded: &loaded,
        bands,
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
        matches,
        millis,
        run_id: String::new(),
        session: format!("{millis}-{}", std::process::id()),
    };

    // The old engine's results come before the run folder exists and before any match of
    // the changed engine plays, so a base that cannot build stops the run with nothing done.
    let mut old = ctx.old_engine(&data)?;

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
    // The old engine's own runs, started by a change run, leave their console to the parent.
    let quiet = std::env::var_os(QUIET_ENV).is_some();
    match &session.opened {
        Opened::New => {}
        Opened::Resumed if quiet => {}
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
    // The single-thread figure is taken before the threads play: they share this process,
    // so its peak memory afterwards would hold every thread's matches.
    let benches = arms
        .iter()
        .map(|(_, s)| ctx.bench(s))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let Some(mut played) = ctx.play_all(&arms, &arm_dirs, &inputs, &benches, &owner_id)? else {
        return stopped();
    };
    // A change run without a target judges its pilot, sets each suite's target from the
    // spread it saw, and grows to it in the same command.
    let mut pilot_used = known_change.and_then(|t| t.pilot);
    if change && known_change.is_none() && pilot < opts.matches {
        let targets = {
            let _judge = stages::enter(Stage::Judge);
            let (sets, rows) = ctx.paired(&played[0], old.as_ref().map(|(_, f)| f));
            let (found, c) = verdict::bootstrap(&sets, &rows, opts.seed, verdict::RESAMPLES);
            verdict::targets(&sets, &rows, &found, c, pilot, opts.matches)
        };
        pilot_used = Some(pilot);
        for (suite, n) in ctx.matches.iter_mut() {
            *n = targets.get(suite).copied().unwrap_or(pilot);
        }
        ctx.write_target(&run_dir, true, pilot_used)?;
        if ctx.matches.values().any(|&n| n > pilot) {
            let plan: Vec<String> = ctx
                .matches
                .iter()
                .map(|(s, n)| format!("{} {n}", s.code()))
                .collect();
            eprintln!(
                "pilot of {pilot} matches judged; growing to the power target: {}",
                plan.join(", ")
            );
            old = ctx.old_engine(&data)?;
            // The growth is a session of its own: a session's work list counts only what
            // earlier sessions finished, and the pilot is finished now.
            ctx.session = format!("{}-grow", ctx.session);
            let Some(grown) = ctx.play_all(&arms, &arm_dirs, &inputs, &benches, &owner_id)? else {
                return stopped();
            };
            played = grown;
        }
    }
    let first = &played[0];
    let (total, finished_after) = ctx.progress(&arm_dirs)?;
    let units = Units {
        total,
        finished_before,
        played: finished_after.saturating_sub(finished_before),
    };
    if session.opened == Opened::Resumed && units.played == 0 && finished_before == total {
        eprintln!("judged again from {total} stored rows; 0 matches played");
        let changed = ctx.changed_bands(known.as_ref());
        if !changed.is_empty() {
            eprintln!(
                "bands changed since the last judgement: {}",
                changed.join(", ")
            );
        }
    }
    let results_digest = rows::rows_digest(
        arms.iter()
            .zip(&played)
            .map(|((name, _), arm)| (*name, &arm.rows)),
    );
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

    // The verdict: a change run's paired max-t test, or each band's range check.
    let (verdicts, joint, power, rules) = {
        let _judge = stages::enter(Stage::Judge);
        if change {
            let (sets, mut rows) = ctx.paired(first, old.as_ref().map(|(_, f)| f));
            if !opts.bands.is_empty() {
                rows.retain(|r| opts.bands.contains(&r.band));
            }
            let (found, c) = verdict::bootstrap(&sets, &rows, opts.seed, verdict::RESAMPLES);
            let (rows, joint) = verdict::judge(&sets, &rows, &found, c, first.guards());
            let power: BTreeMap<String, PowerInfo> = ctx
                .matches
                .iter()
                .map(|(&s, &target)| {
                    let reached = rows
                        .iter()
                        .filter(|r| r.suite == s.code())
                        .all(|r| r.power == Some(true));
                    let info = PowerInfo {
                        pilot: pilot_used,
                        target,
                        cap: opts.matches,
                        reached,
                    };
                    (s.code().to_string(), info)
                })
                .collect();
            let rules = rules::run(rules::RULES, &|_| true);
            (rows, Some(joint), Some(power), Some(rules))
        } else if opts.pair.is_none() {
            (verdict::range_only(&first.checks), None, None, None)
        } else {
            (Vec::new(), None, None, None)
        }
    };
    if !verdicts.is_empty() && !quiet {
        eprint!("{}", verdict::render_table(&verdicts, joint.as_ref()));
    }
    if let Some(r) = &rules {
        eprintln!("{}", rules::line(r));
    }
    ctx.write_target(&run_dir, change, pilot_used)?;

    let mut report = CalibrationReport {
        owner_id,
        run_id: run_id.clone(),
        seed: opts.seed,
        content_hash: first.content_hash.clone(),
        fixtures_hash,
        fixtures_scheme: FIXTURE_SCHEME,
        identity: serde_json::to_value(&identity)?,
        outcome: "success",
        error_type: None,
        error_code: None,
        error_retriable: None,
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
        runner: "threads",
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
        base: old.map(|(b, _)| b),
        verdicts,
        joint,
        power,
        registry: RegistryInfo {
            version: REGISTRY_VERSION,
            digest: ctx.bands.digest(),
            migrated_from: ctx.bands.migrated_from,
        },
        stages: stages::snapshot(run_started.elapsed()),
        rules,
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
        _ => match &report.joint {
            // A change run passes only on a joint pass: a fail or a not sure exits 2.
            Some(joint) => i32::from(joint.word != verdict::Word::Pass) * 2,
            None => {
                if first.pass {
                    0
                } else {
                    2
                }
            }
        },
    };
    if !quiet {
        eprint!("{}", stages::render_table(&report.stages));
    }
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
    bands: Registry,
    jobs: u32,
    selection: Selection,
    suites: Vec<Suite>,
    /// The formation names of every pairing of the formations suite.
    pairings: Vec<[String; 2]>,
    /// The formations of the tactics file, in order.
    formation_names: Vec<String>,
    /// The numbers of the pairings this run plays, in order.
    selected: Vec<usize>,
    /// Matches per suite unit (per pairing, per arm) this phase plays: `--matches`, or a
    /// change run's pilot or power target.
    matches: BTreeMap<Suite, u32>,
    /// The run's start, from its `run.json`: part of every match identifier.
    millis: u64,
    run_id: String,
    /// This invocation: the ledger files its threads write carry it.
    session: String,
}

/// The suites a run plays, and the numbers of the formation pairings it plays: `--suite`
/// narrowed to the suites that check the `--band` names, and to the formations suite when
/// `--pairing` names a pairing. Every name is checked before any work.
fn select(
    requested: &[Suite],
    opts: &CalibrateOpts,
    pairings: &[[String; 2]],
    registry: &Registry,
) -> anyhow::Result<(Vec<Suite>, Vec<usize>)> {
    let mut suites = requested.to_vec();
    for band in &opts.bands {
        if registry.band(band).is_none() {
            anyhow::bail!(
                "--band {band}: no such band; the bands of {} are {}",
                crate::report::bands::BANDS_FILE,
                registry.names().join(", ")
            );
        }
    }
    if !opts.bands.is_empty() {
        suites.retain(|s| {
            opts.bands.iter().any(|b| {
                registry
                    .suites_of(b)
                    .is_some_and(|owners| owners.contains(s))
            })
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

/// The outcome of one arm: every suite played on the threads under one set of flag states,
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
    change_never_applied: u32,
    change_expired_at_full_time: u32,
    match_without_stats: u32,
    match_panicked: u32,
    violations: usize,
    events_written: u32,
    events_kept: u32,
    bench: BenchFigures,
    /// Every planned match's row, by fixture key, for the results digest.
    rows: BTreeMap<FixtureKey, Row>,
    rows_info: Option<RowsInfo>,
    /// Every match folded, for the paired verdict, with its fixture key by suite.
    builder: RunBuilder,
    keys: BTreeMap<Suite, Vec<u64>>,
}

impl Arm {
    /// What fails a change run outside the bands: panics, missing results, unapplied
    /// changes, and rule violations.
    fn guards(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (n, what) in [
            (self.match_panicked, "matches panicked"),
            (self.match_without_stats, "planned matches have no result"),
            (
                self.change_never_applied,
                "tactics changes were never applied",
            ),
        ] {
            if n > 0 {
                out.push(format!("{n} {what}"));
            }
        }
        if self.violations > 0 {
            out.push(format!("{} rule violations", self.violations));
        }
        out
    }

    /// No match panicked, no record is missing, no change was left unapplied, and the rule
    /// checker found nothing.
    fn trusted(&self) -> bool {
        self.match_panicked == 0
            && self.match_without_stats == 0
            && self.change_never_applied == 0
            && self.violations == 0
    }

    fn guard(&self) -> Guard {
        Guard {
            dark_paths: self.match_without_stats + self.change_never_applied + self.match_panicked,
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
            // Worker processes are gone; the key stays for readers of earlier reports.
            workers_failed: 0,
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
    /// The fixture key of each match in the builder, by suite, in the builder's order.
    keys: BTreeMap<Suite, Vec<u64>>,
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
            self.matches_for(suite),
            &self.formation_names,
            &self.selected,
        )
    }

    /// Matches per suite unit `suite` plays in this phase.
    fn matches_for(&self, suite: Suite) -> u32 {
        self.matches
            .get(&suite)
            .copied()
            .unwrap_or(self.opts.matches)
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
    fn old_engine(&self, data: &Path) -> anyhow::Result<Option<(BaseReport, Folded)>> {
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
            matches: &self.matches,
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
        let folded = {
            let _judge = stages::enter(Stage::Judge);
            self.fold_rows(&old.dir)?
        };
        let mut checks = folded.builder.checks(&BTreeMap::new());
        if !opts.bands.is_empty() {
            checks.retain(|c| opts.bands.contains(&c.band));
        }
        checks
            .sort_by(|a, b| (&a.suite, &a.band, &a.pairing).cmp(&(&b.suite, &b.band, &b.pairing)));
        let report = BaseReport {
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
        };
        Ok(Some((report, folded)))
    }

    /// A builder with every selected suite planned.
    fn planned_builder(&self) -> RunBuilder {
        let opts = self.opts;
        let mut builder = RunBuilder::new(self.bands.clone());
        builder.minutes = opts.minutes;
        for &suite in &self.suites {
            let matches = self.matches_for(suite);
            match suite {
                Suite::Formations => builder.plan_pairings(
                    matches,
                    self.selected
                        .iter()
                        .map(|&p| self.pairings[p].clone())
                        .collect(),
                ),
                Suite::RedCard => {
                    builder.plan(suite, matches.saturating_mul(RED_CARD_ARMS.len() as u32));
                }
                Suite::Equal | Suite::Strength => builder.plan(suite, matches),
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
    /// compact row, in fixture order; a planned match with no ledger entry or no row is the
    /// dark path. Each match's band record is rebuilt from its row with the statistics
    /// files' rounding, so the bands equal those folded from the statistics files.
    fn fold_rows(&self, dir: &Path) -> anyhow::Result<Folded> {
        let done = run_folder::finished(dir, None)?;
        let read = rows::read_rows(dir, &done)?;
        let mut builder = self.planned_builder();
        let mut keys: BTreeMap<Suite, Vec<u64>> = BTreeMap::new();
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
                keys.entry(suite).or_default().push(k.key.as_u64());
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
            keys,
            rows: planned,
            rows_info: Some(rows_info),
            panicked,
        })
    }

    /// The single-thread figure: the default teams timed on one thread under `states`.
    fn bench(&self, states: &FlagStates) -> anyhow::Result<BenchFigures> {
        let content = self.loaded.content.with_flags(states)?;
        let [team_a, team_b] = &self.loaded.teams;
        let config = MatchConfig::new(
            self.opts.seed,
            self.opts.minutes,
            &content,
            [team_a, team_b],
        )?;
        crate::bench::measure(&config, None, BENCH_MATCHES)
    }

    /// Judges an arm's folded matches against the bands, given each suite's wall time and
    /// the whole run's, with the arm's single-thread figure.
    fn finish_arm(
        &self,
        states: &FlagStates,
        wall: &BTreeMap<Suite, u64>,
        suites_ms: u64,
        folded: Folded,
        (events_written, events_kept): (u32, u32),
        bench: BenchFigures,
    ) -> anyhow::Result<Arm> {
        let opts = self.opts;
        let content = self.loaded.content.with_flags(states)?;
        let [team_a, team_b] = &self.loaded.teams;
        let config = MatchConfig::new(opts.seed, opts.minutes, &content, [team_a, team_b])?;

        let Folded {
            builder,
            keys,
            rows,
            rows_info,
            panicked: match_panicked,
        } = folded;
        let figures = builder.figures();
        let formations = builder.pairing_figures();
        let mut checks = builder.checks(wall);
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
            && match_panicked == 0;
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
            change_never_applied,
            change_expired_at_full_time: builder.change_expired_at_full_time(),
            match_without_stats,
            match_panicked,
            violations: builder.violations(),
            events_written,
            events_kept,
            bench,
            rows,
            rows_info,
            builder,
            keys,
        })
    }

    /// Plays every arm's unfinished fixtures of this phase on the threads. `None` comes
    /// back when the stop seam cut the phase short.
    fn play_all(
        &self,
        arms: &[(&str, FlagStates)],
        arm_dirs: &[PathBuf],
        inputs: &[runner::ArmInput<'_>],
        benches: &[BenchFigures],
        owner_id: &str,
    ) -> anyhow::Result<Option<Vec<Arm>>> {
        let mut played = Vec::with_capacity(arms.len());
        let ran = runner::run_arms(self, inputs, owner_id)?;
        if ran.stopped {
            return Ok(None);
        }
        for ((((_, s), arm_dir), wall), bench) in
            arms.iter().zip(arm_dirs).zip(&ran.wall).zip(benches)
        {
            let events = count_files(&arm_dir.join("events"));
            let folded = {
                let _judge = stages::enter(Stage::Judge);
                self.fold_rows(arm_dir)?
            };
            played.push(self.finish_arm(
                s,
                wall,
                ran.total_ms,
                folded,
                (events, events),
                *bench,
            )?);
        }
        Ok(Some(played))
    }

    /// The paired rows of a change run: the changed engine's matches and the old engine's
    /// on the fixtures both finished.
    fn paired(
        &self,
        changed: &Arm,
        old: Option<&Folded>,
    ) -> (Vec<verdict::SuiteSet>, Vec<verdict::PairedRow>) {
        let Some(old) = old else {
            return (Vec::new(), Vec::new());
        };
        verdict::paired(
            (&changed.builder, &changed.keys),
            (&old.builder, &old.keys),
            &self.matches,
        )
    }

    /// Writes the run's `target.json`: this phase's matches per suite, and the registry the
    /// run is judged with.
    fn write_target(&self, dir: &Path, change: bool, pilot: Option<u32>) -> anyhow::Result<()> {
        run_folder::write_target(
            dir,
            &Target {
                version: TARGET_VERSION,
                change,
                target: self
                    .matches
                    .iter()
                    .map(|(s, n)| (s.code().to_string(), *n))
                    .collect(),
                pilot,
                cap: self.opts.matches,
                registry: self.bands.digest(),
                bands: self
                    .bands
                    .bands
                    .iter()
                    .map(|b| (b.band.clone(), b.digest()))
                    .collect(),
            },
        )
    }

    /// The bands whose measure, suites, range or smallest shift differ from those the run
    /// was last judged with, and the bands added or removed since.
    fn changed_bands(&self, known: Option<&Target>) -> Vec<String> {
        let Some(t) = known else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for b in &self.bands.bands {
            match t.bands.get(&b.band) {
                Some(d) if *d == b.digest() => {}
                Some(_) => out.push(b.band.clone()),
                None => out.push(format!("{} (new)", b.band)),
            }
        }
        for name in t.bands.keys() {
            if self.bands.band(name).is_none() {
                out.push(format!("{name} (removed)"));
            }
        }
        out
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
