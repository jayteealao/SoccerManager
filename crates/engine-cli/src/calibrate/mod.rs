//! `engine-cli calibrate`: play many matches with the AI manager on both sides, in worker
//! processes of this binary, and write one run report that checks the aggregate figures
//! against the accepted realism bands.
//!
//! `--suite all` plays three suites: equal clubs, a stronger club, and every formation
//! pairing (`--matches` matches per pairing). Each failing band is named on standard error.
//! `--suite red-card` plays the controlled sending-off experiment instead.
//!
//! A targeted run plays only what `--pairing` and `--band` select, and each selected
//! fixture keeps its place in the full list, so its figures equal a full run's. With
//! `--baseline`, the run is checked against an earlier report before it plays and a diff
//! is printed band by band, each change with its sampling error.
//!
//! Run folder: `report.json`, `stats/<match.id>.json` for every match, and
//! `events/<match.id>.jsonl` for the matches kept (outliers by default). A paired run
//! (`--pair <flag>`) plays every fixture twice, into `arms/off/` and `arms/on/`, and its one
//! report compares the two arms band by band.

pub mod fixtures;
pub mod worker;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use anyhow::Context;
use engine::data::{TeamFile, hex12};
use engine::flags::{FlagSetting, FlagState, FlagStates, effective};
use engine::observe::identity::{data_dir, load_or_create_owner_id};
use engine::observe::{emit_line, machine_hash, read_stats, unix_millis, write_record_at};
use engine::{Content, ContentDir, MatchConfig};
use sha2::{Digest, Sha256};

use crate::bench::BenchFigures;
use crate::cli::{CalibrateOpts, KeepEvents, SuiteArg};
use crate::report::bands::Bands;
use crate::report::baseline::{self, Identity};
use crate::report::compare::{self, Guard};
use crate::report::{
    ArmReport, BAND_NAMES, BandCheck, CalibrationReport, FlagEntry, PairInfo, PairingFigures,
    RED_CARD_ARMS, RedCardFigures, RunBuilder, Selection, Suite, SuiteFigures, band_suites,
};

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
    let run_id = format!("calib-{:016x}-{millis}", opts.seed);
    let run_dir = opts
        .out
        .clone()
        .unwrap_or_else(|| data.join("runs").join(&run_id));
    std::fs::create_dir_all(&run_dir)
        .with_context(|| format!("cannot create the run folder runs/{run_id}"))?;
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
    let ctx = RunCtx {
        dir: &dir,
        opts,
        loaded: &loaded,
        bands: Bands::load(&dir)?,
        // nosemgrep: rust.lang.security.current-exe.current-exe -- only used to start worker copies of this program
        exe: std::env::current_exe().context("cannot find this program to start workers")?,
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
        selected,
        millis,
        run_id: &run_id,
    };

    let started = Instant::now();
    let mut played: Vec<Arm> = Vec::with_capacity(arms.len());
    for (name, s) in &arms {
        let arm_dir = if name.is_empty() {
            run_dir.clone()
        } else {
            run_dir.join("arms").join(name)
        };
        played.push(ctx.play_arm(s, &arm_dir)?);
    }
    let first = &played[0];

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
        violations: first.violations,
        events_written: first.events_written,
        events_kept: first.events_kept,
        machine_hash: machine_hash(),
        cpu_model: crate::bench::cpu_model(),
        flags,
        pair: None,
        arms: None,
        compare: None,
        verdict: None,
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
    /// The numbers of the pairings this run plays, in order.
    selected: Vec<usize>,
    millis: u64,
    run_id: &'a str,
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
    violations: usize,
    events_written: u32,
    events_kept: u32,
    bench: BenchFigures,
}

impl Arm {
    /// No worker failed, no record is missing, no change was left unapplied, and the
    /// validator found nothing.
    fn trusted(&self) -> bool {
        self.workers_failed == 0
            && self.match_without_stats == 0
            && self.change_never_applied == 0
            && self.violations == 0
    }

    fn guard(&self) -> Guard {
        Guard {
            dark_paths: self.match_without_stats + self.change_never_applied + self.workers_failed,
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

impl RunCtx<'_> {
    /// Plays every suite into `arm_dir` with `states`, then times the default teams on one
    /// thread under the same states.
    fn play_arm(&self, states: &FlagStates, arm_dir: &Path) -> anyhow::Result<Arm> {
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
            let children = (0..self.jobs)
                .map(|shard| {
                    worker_command(self, suite, shard, arm_dir, &settings)
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
            let wall_ms = u64::try_from(suite_started.elapsed().as_millis()).unwrap_or(u64::MAX);
            tracing::info!(
                signal = "calibrate.suite",
                run.id = %self.run_id,
                suite = suite.code(),
                matches = self.planned(suite).len(),
                wall_ms,
                jobs = self.jobs
            );
            wall.insert(suite, wall_ms);
        }
        let suites_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

        // The single-thread figure, on the default teams, once the workers are done.
        let content = self.loaded.content.with_flags(states)?;
        let [team_a, team_b] = &self.loaded.teams;
        let config = MatchConfig::new(opts.seed, opts.minutes, &content, [team_a, team_b])?;
        let bench = crate::bench::measure(&config, None, BENCH_MATCHES)?;

        let events_dir = arm_dir.join("events");
        let events_written = count_files(&events_dir);
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
            for planned in self.planned(suite) {
                let id = fixtures::match_id(opts.seed, suite, planned.index, self.millis);
                match read_stats(&arm_dir.join("stats").join(format!("{id}.json"))) {
                    Ok(stats) => {
                        let outlier = match (planned.pairing, planned.arm) {
                            (Some((pairing, side)), _) => builder.add_pairing(stats, pairing, side),
                            (None, Some(arm)) => builder.add_red_card(stats, arm),
                            (None, None) => builder.add(suite, stats, planned.boosted),
                        };
                        if !outlier && opts.keep_events == KeepEvents::Outliers {
                            let _ = std::fs::remove_file(events_dir.join(format!("{id}.jsonl")));
                        }
                    }
                    // A missing record is the dark path; its event file, if any, is kept.
                    Err(_) => builder.add_missing(),
                }
            }
        }
        let events_kept = count_files(&events_dir);

        let figures = builder.figures();
        let formations = builder.pairing_figures();
        let mut checks = builder.checks(&figures, &formations, &wall);
        // `--band` judges only the named bands; each suite's time budget stays.
        if !opts.bands.is_empty() {
            checks.retain(|c| c.band == "wall_ms" || opts.bands.contains(&c.band));
        }
        let change_never_applied = builder.change_never_applied();
        let match_without_stats = builder.missing;
        for (counter, value) in [
            ("darkpath.change_never_applied", change_never_applied),
            ("darkpath.match_without_stats", match_without_stats),
        ] {
            if value > 0 {
                tracing::warn!(signal = "calibrate.darkpath", run.id = %self.run_id, counter, value);
            }
        }
        let pass = checks.iter().all(|c| c.pass)
            && change_never_applied == 0
            && match_without_stats == 0
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
            violations: builder.violations(),
            events_written,
            events_kept,
            bench,
        })
    }
}

/// One planned match of a suite, as the parent reads it back.
struct Planned {
    index: u32,
    /// The strength suite: the side of the boosted club.
    boosted: Option<usize>,
    /// The formations suite: the pairing's place in the selection, and the side of its
    /// first formation.
    pairing: Option<(usize, usize)>,
    /// The red-card suite: the arm's number in [`RED_CARD_ARMS`].
    arm: Option<usize>,
}

impl RunCtx<'_> {
    /// Every match `suite` plays, in fixture order.
    fn planned(&self, suite: Suite) -> Vec<Planned> {
        let matches = self.opts.matches;
        match suite {
            Suite::Formations => {
                fixtures::formation_fixtures_for(matches, self.pairings.len(), &self.selected)
                    .into_iter()
                    .map(|f| Planned {
                        index: f.fixture.index,
                        boosted: None,
                        pairing: self
                            .selected
                            .iter()
                            .position(|&p| p == f.pairing)
                            .map(|place| (place, f.first_side)),
                        arm: None,
                    })
                    .collect()
            }
            Suite::RedCard => fixtures::red_card_fixtures(self.opts.seed, matches)
                .into_iter()
                .map(|f| Planned {
                    index: f.index,
                    boosted: None,
                    pairing: None,
                    arm: Some(f.arm),
                })
                .collect(),
            Suite::Equal | Suite::Strength => fixtures::fixtures(matches)
                .into_iter()
                .map(|f| Planned {
                    index: f.index,
                    boosted: (suite == Suite::Strength).then(|| f.boosted_side()),
                    pairing: None,
                    arm: None,
                })
                .collect(),
        }
    }
}

fn suite_codes(figures: &BTreeMap<Suite, SuiteFigures>) -> BTreeMap<String, SuiteFigures> {
    figures
        .iter()
        .map(|(s, f)| (s.code().to_string(), f.clone()))
        .collect()
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
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if suite == Suite::Formations && ctx.selected.len() < ctx.pairings.len() {
        let numbers: Vec<String> = ctx.selected.iter().map(usize::to_string).collect();
        cmd.arg("--pairing-numbers").arg(numbers.join(","));
    }
    // The test seam reaches the first worker only.
    if let (Some(inject), 0) = (opts.inject_failure, shard) {
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
