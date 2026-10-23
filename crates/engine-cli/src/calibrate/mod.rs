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
//! The bands come from the band registry, `realism-bands.json`. `--kind change|milestone|
//! release` (default change) picks the run's seed set and its rare-band margins; a seed
//! outside the kind's set is refused before anything plays. A run without an old engine
//! judges each band by the pass rule. `--base <REV>` makes it a change run: the old engine's
//! results on the same fixtures come from a cache of its rows (the hidden `--base-binary
//! <EXE>` names a ready executable instead), the run plays exactly its kind's fixed count of
//! paired matches per suite unit from the band file (`--matches` is refused), and each band
//! gets two words: against real football, which alone decides the joint verdict, and
//! against the old engine (better, worse, equivalent or not sure). The count is kept in
//! `target.json`; a run into a finished folder plays nothing and judges again from the
//! stored rows. Every run reports the time and memory of each stage.
//!
//! Each judgement also reads every band's real value in place from the real-data folder
//! (`--real-data`, default the main checkout's `.scratch/out`) and reports it beside the
//! verdict with its distance from the run's value; it never changes a verdict. A band that
//! only reports (`gates: false`, a real sample under its floor, or a generalisation under
//! the floor) is judged by the same rules in a second table, "reported bands", and in
//! `calib.reported`, apart from the gating bands: it never moves the joint threshold, a
//! suite's power target, the joint word or `calib.pass`. A band that reads the realism
//! measures, or a per-club spread, is judged on its range alone when the old engine's rows
//! do not carry what it reads. `calibrate accept <band> --run <dir>` records the developer's
//! acceptance of a not-sure band in the folder; the next judgement marks it "accepted, not
//! shown equivalent", and its word stays not sure.

pub mod base;
pub mod fixtures;
pub mod gate_proof;
#[cfg(feature = "measures")]
pub mod physical;
#[cfg(feature = "measures")]
pub mod primitives;
pub mod rows;
pub mod rules;
pub mod run_folder;
pub mod runner;
#[cfg(test)]
mod spread;
pub mod stages;
pub mod styles;
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
use crate::measure_rows::SHAPE_FIELDS;
use crate::report::bands::{BandDef, Measure};
use crate::report::bands::{REGISTRY_VERSION, Registry};
use crate::report::baseline::{self, Identity};
use crate::report::compare::{self, Guard};
use crate::report::kinds::Kind;
use crate::report::measures::Extras;
use crate::report::{
    ArmReport, BandCheck, BaseCache, BaseReport, CalibrationReport, Counts, FlagEntry,
    MEASURES_VERSION, Observations, PairInfo, PairingFigures, RED_CARD_ARMS, Recorded,
    RedCardFigures, RegistryInfo, RowsInfo, RunBuilder, Selection, Suite, SuiteFigures, Units,
};
use crate::report::{accepted, nash, proof, real, verdict};
use fixtures::{FIXTURE_SCHEME, FixtureKey, Keyed};
use rows::{Outcome, ROWS_FORMAT, Row, reason};
use run_folder::{Opened, RunIdentity, TARGET_VERSION, Target};
use stages::Stage;

/// Matches per ordered pairing of styles in a change run's styles sample.
pub const STYLES_SAMPLE: u32 = 1;

/// Matches the single-thread benchmark times after its warm-up.
const BENCH_MATCHES: u32 = 5;
/// Set by a change run on the old engine's own `calibrate`: no verdict or stage table.
pub const QUIET_ENV: &str = "SM_CALIBRATE_QUIET";
/// The line a build without the `measures` feature prints as a run starts: it records no
/// realism measures and judges the bands of the band file only.
pub const NO_MEASURES: &str =
    "this build has no measures; build engine-cli with --features measures to record them";

pub fn run(content_dir: Option<&Path>, opts: &CalibrateOpts) -> anyhow::Result<i32> {
    let run_started = Instant::now();
    stages::enable();
    if opts.matches() == 0 {
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
    let mut loaded = crate::content::load(Some(dir.root()), None, None, None)?;
    if let Some(path) = &opts.rules {
        let bytes =
            std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
        loaded.content = engine::Content::load_with_rules(&dir, bytes)?;
    }
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
        SuiteArg::Styles => return styles_alone(&dir, opts, &loaded, &arms[0].1),
    };
    if !opts.style_pairings.is_empty() {
        anyhow::bail!("--style-pairing needs --suite styles");
    }
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
    let change = opts.pair.is_none() && (opts.base.is_some() || opts.base_binary.is_some());
    // The kind and its seed set come first: a refused seed costs nothing. Then the band
    // file at the kind's count and margins; the real values and the floor again come later,
    // still before any match of the changed engine plays.
    let registry = Registry::load(&dir)?;
    let kind = opts.kind;
    let kind_def = registry
        .kinds
        .as_ref()
        .map(|t| t.get(kind).clone())
        .context("the band file has no kinds table")?;
    if let Some(table) = &registry.kinds {
        table
            .check_seed(kind, opts.seed())
            .map_err(anyhow::Error::msg)?;
    }
    if change && opts.matches_arg.is_some() {
        anyhow::bail!(
            "--matches: a change run plays the fixed count of its kind, {}; the count \
             changes only by an edit of {} with its CHANGELOG line",
            crate::report::kinds::changelog_line(kind, &kind_def),
            crate::report::bands::BANDS_FILE
        );
    }
    let registry = registry.with_kind(kind);
    let (suites, selected) = select(&requested, opts, &pairings, &registry)?;
    let per_suite = |s: Suite| {
        if change {
            kind_def.count_for(s)
        } else {
            opts.matches()
        }
    };
    let run_matches = suites
        .iter()
        .map(|&s| per_suite(s))
        .max()
        .unwrap_or(opts.matches());
    let real_folder = real::resolve(opts.real_data.as_deref(), dir.root());

    // The baseline is checked before the run folder exists and before any match plays.
    let fixtures_hash = fixtures_hash(&loaded.content.with_flags(&arms[0].1)?, &loaded.teams)?;
    let base = opts
        .baseline
        .as_deref()
        .map(|path| {
            baseline::load(
                path,
                Identity {
                    seed: opts.seed(),
                    matches: run_matches,
                    fixtures_hash: &fixtures_hash,
                    fixtures_scheme: FIXTURE_SCHEME,
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
        .map(|&suite| match suite {
            Suite::Formations => per_suite(suite).saturating_mul(selected.len() as u32),
            Suite::RedCard => per_suite(suite).saturating_mul(RED_CARD_ARMS.len() as u32),
            Suite::Equal | Suite::Strength => per_suite(suite),
        })
        .max()
        .unwrap_or(1);
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let jobs = opts
        .jobs
        .unwrap_or(u32::try_from(cores).unwrap_or(u32::MAX))
        .clamp(1, most);
    let bands = registry;
    let identity = run_identity(
        opts,
        &states,
        &content_identity(&loaded.content, &dir)?,
        &fixtures_hash,
        &bands,
    )?;
    // A change run checks the sensitivity rules last; a refused rules file stops it here.
    let rule_set = if change {
        Some(rules::load(&loaded.content, &dir)?)
    } else {
        None
    };
    let fresh_id = format!("calib-{:016x}-{millis}", opts.seed());
    let run_dir = opts
        .out
        .clone()
        .unwrap_or_else(|| data.join("runs").join(&fresh_id));
    // A run folder of the same identity keeps the bands it was last judged with, which
    // name what a re-judgement changed.
    let known = run_folder::holds(&run_dir, &identity)
        .then(|| run_folder::read_target(&run_dir))
        .flatten();
    let matches = run_folder::phase_matches(&suites, change, &kind_def, opts.matches());
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
        kind,
        millis,
        run_id: String::new(),
        session: format!("{millis}-{}", std::process::id()),
        real_folder: real_folder.clone(),
        quiet: std::env::var_os(QUIET_ENV).is_some(),
        samples: std::sync::OnceLock::new(),
        sides: std::sync::OnceLock::new(),
    };

    // The old engine's results come before the run folder exists and before any match of
    // the changed engine plays, so a base that cannot build stops the run with nothing done.
    // Its events name the run id the folder holds once it opens: the resumed run's, or the
    // fresh one.
    ctx.run_id = run_folder::holds(&run_dir, &identity)
        .then(|| base::run_id_in(&run_dir))
        .flatten()
        .unwrap_or_else(|| fresh_id.clone());
    let old = ctx.old_engine(&data)?;

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
    let mode = if change {
        "change"
    } else if opts.pair.is_some() {
        "pair"
    } else {
        "evaluation"
    };
    tracing::info!(
        signal = "calibrate.run_started",
        run.id = %run_id,
        mode,
        seed = opts.seed(),
        kind = kind.code(),
        matches = run_matches,
        threads = jobs,
        resumed = session.opened == Opened::Resumed,
        fixtures_total = total,
        fixtures_done = finished_before
    );
    if let Opened::Superseded { old_run_id, .. } = &session.opened {
        tracing::warn!(
            signal = "calibrate.run_superseded",
            run.id = %run_id,
            old_run_id = %old_run_id,
            moved_to = %run_dir.join(run_folder::SUPERSEDED).join(old_run_id).display()
        );
    }
    // The end of a run that did not fail: its outcome, exit code and wall time.
    let finished = |outcome: &str, pass: Option<bool>, verdict: Option<&str>, code: i32| {
        tracing::info!(
            signal = "calibrate.run_finished",
            run.id = %run_id,
            mode,
            outcome,
            pass,
            verdict,
            exit = code,
            elapsed_s = (run_started.elapsed().as_secs_f64() * 10.0).round() / 10.0
        );
    };
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

    if !quiet && !cfg!(feature = "measures") {
        eprintln!("{NO_MEASURES}");
    }
    // Whether the gate proof in the content folder measured this gate; it decides nothing.
    if !quiet {
        let gate = proof::current(&ctx.bands);
        let check = proof::check(&dir, &gate);
        if let Some(lines) = check.lines() {
            eprint!("{lines}");
        }
        let state = check.state();
        if check == proof::Check::Matches {
            tracing::info!(signal = "calibrate.gate_proof", run.id = %run_id, state,
                registry = %gate.registry, measures = gate.measures,
                verdict_code = %gate.verdict_code);
        } else {
            tracing::warn!(signal = "calibrate.gate_proof", run.id = %run_id, state,
                registry = %gate.registry, measures = gate.measures,
                verdict_code = %gate.verdict_code);
        }
    }
    let started = Instant::now();
    let stopped = || {
        eprintln!(
            "stopped after {} work units; run the same command again to resume",
            opts.stop_after_units.unwrap_or(0)
        );
        finished("stopped", None, None, 1);
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
    // The real values, with the pass rule's real sides, are read before any match of the
    // changed engine plays: a band whose margin is under its floor with the real error read
    // here stops the run now. They are read after the single-thread figure, whose process
    // peak memory would otherwise hold them. The old engine's own quiet run leaves them to
    // the parent.
    let reference = {
        let path = crate::real_targets::default_file(dir.root());
        path.is_file()
            .then(|| crate::real_targets::ReferenceFile::load(&path))
            .transpose()?
    };
    let real_info = (!ctx.quiet).then(|| {
        let _judge = stages::enter(Stage::Judge);
        real::read_all_with(
            &real_folder,
            &ctx.bands,
            &ctx.real_samples(),
            reference.as_ref(),
        )
    });
    if let Some(info) = &real_info {
        floor_with_real(&ctx.bands, info)?;
        let sides = info
            .bands
            .iter()
            .filter_map(|b| b.side.clone().map(|side| (b.band.clone(), side)))
            .collect();
        let _ = ctx.sides.set(std::sync::Arc::new(sides));
    }

    let Some(played) = ctx.play_all(&arms, &arm_dirs, &inputs, &benches, &owner_id)? else {
        return stopped();
    };
    let first = &played[0];
    let (total, finished_after) = ctx.progress(&arm_dirs)?;
    let units = Units {
        total,
        finished_before,
        played: finished_after.saturating_sub(finished_before),
    };
    let rejudged =
        session.opened == Opened::Resumed && units.played == 0 && finished_before == total;
    if rejudged {
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
    let rows_info = rows_of_every_arm(&played);

    let written = loaded.content.written_tuning();
    let flags = flag_entries(written, &states, opts.pair.as_deref())?;

    // The verdict: a change run's paired max-t test, or each band's range check.
    let Judged {
        mut verdicts,
        mut reported,
        mut joint,
        agreement,
    } = ctx.judge_run(first, old.as_ref().map(|(_, f)| f), change);
    // A change run judges the pairs both engines finished; more than one percent dropped is
    // never a pass.
    let dropped = old.as_ref().map(|(_, f)| dropped_pairs(first, f));
    if let (Some(d), Some(j)) = (&dropped, joint.as_mut()) {
        verdict::under_count(j, d.planned, d.dropped);
    }
    // The real values, read in place at each judgement; the old engine's own quiet run
    // leaves them to the parent.
    if let Some(info) = &real_info {
        attach_real(&mut verdicts, info, &ctx.bands);
        attach_real(&mut reported, info, &ctx.bands);
    }
    let identity_value = serde_json::to_value(&identity)?;
    // The developer's acceptances in the run folder mark not-sure rows; a word never moves.
    let acceptances = (!quiet)
        .then(|| {
            accepted::read(
                &run_dir,
                &run_id,
                &accepted::identity_digest(&identity_value),
            )
        })
        .flatten();
    if let Some(a) = &acceptances {
        mark_accepted(&mut verdicts, a);
        mark_accepted(&mut reported, a);
        if let Some(why) = &a.ignored {
            eprintln!("acceptances ignored: {why}");
            tracing::warn!(
                signal = "calibrate.accepted_ignored",
                run.id = %run_id,
                file = %a.file,
                reason = %why
            );
        }
    }
    let counts = match_counts(
        &played,
        old.as_ref().map(|(b, f)| (b, f)),
        joint.as_ref(),
        &identity.build.executable_sha256,
    );
    let realism = real_info
        .as_ref()
        .map(|_| real::realism(&ctx.bands, &verdicts));
    if let Some(info) = &real_info {
        eprint!("{}", info.headline());
        tracing::info!(
            signal = "calibrate.real_data",
            run.id = %run_id,
            folder = info.folder.as_deref(),
            from = info.from,
            present = info.present,
            missing = info.missing,
            no_counterpart = info.no_counterpart
        );
    }
    if !verdicts.is_empty() && !quiet {
        eprint!("{}", verdict::render_table(&verdicts, joint.as_ref()));
    }
    // The headline of a change run: its kind's counts and what stayed open.
    let headline = joint.as_ref().map(|_| {
        let gating: Vec<&verdict::VerdictRow> = verdicts
            .iter()
            .filter(|r| r.report_only.is_none())
            .collect();
        crate::report::headline(
            &crate::report::kinds::headline_count(kind, &kind_def),
            gating
                .iter()
                .filter(|r| r.word == verdict::Word::NotSure)
                .count(),
            gating.len(),
            accepted::count(&verdicts),
            dropped.map_or(0, |d| d.dropped),
        )
    });
    if let (Some(h), false) = (&headline, quiet) {
        eprintln!("{h}");
    }
    // The formation field rating: report only, read by nothing that decides the verdict.
    // It describes the same arm as `calib.formations`, and needs every pairing.
    let nash_rating = (!quiet && ctx.suites.contains(&Suite::Formations)).then(|| {
        if ctx.selected.len() < ctx.pairings.len() {
            Err((ctx.selected.len(), ctx.pairings.len()))
        } else {
            first
                .builder
                .formation_table()
                .map(|(names, wins, matches)| nash::rate(&names, &wins, matches))
        }
    });
    if !quiet {
        eprint!("{}", verdict::render_reported(&reported, joint.is_some()));
        eprint!("{}", crate::report::render_figures(&first.figure_rows));
        match &nash_rating {
            Some(Ok(r)) => eprint!("{}", nash::render(r)),
            Some(Err((played, total))) => eprint!("{}", nash::incomplete_line(*played, *total)),
            None => {}
        }
        eprint!("{}", counts.line());
    }
    if let Some(Ok(r)) = &nash_rating {
        tracing::info!(
            signal = "calibrate.nash",
            run.id = %run_id,
            formations = r.formations.len(),
            flagged = r.formations.iter().filter(|f| f.flagged).count(),
            converged = r.converged
        );
    }
    if !reported.is_empty() {
        let count = |w: verdict::Word| reported.iter().filter(|r| r.word == w).count();
        tracing::info!(
            signal = "calibrate.reported",
            run.id = %run_id,
            bands = reported.len(),
            pass = count(verdict::Word::Pass),
            fail = count(verdict::Word::Fail),
            not_sure = count(verdict::Word::NotSure),
            accepted = reported.iter().filter(|r| r.accepted == Some(true)).count()
        );
    }
    if let Some(r) = &realism {
        eprintln!("{}", r.claim);
    }
    let provisional_margins = crate::report::ProvisionalMargins::of(&ctx.bands);
    let report_only_floor = real_info
        .as_ref()
        .map(|info| ctx.bands.report_only_floor(&|b| real_error(info, b)))
        .unwrap_or_default();
    if !quiet {
        eprint!("{}", provisional_margins.line());
        eprint!(
            "{}",
            crate::report::render_report_only_floor(&report_only_floor)
        );
    }
    let rules = rule_set.map(|set| rules::run(&loaded.content, &set));
    if let Some(r) = &rules {
        if !quiet && !r.rows.is_empty() {
            eprint!("{}", rules::render_table(r));
        }
        eprintln!("{}", rules::line(r));
    }
    ctx.write_target(&run_dir, change)?;
    // A change run's styles sample: one match per ordered pairing on the changed engine, and
    // never on the old one; report only.
    let styles_sample = if change {
        let content = loaded.content.with_flags(&arms[0].1)?;
        let sample = styles::play(&styles::SuiteInput {
            content: &content,
            commentary: &loaded.commentary,
            seed: opts.seed(),
            matches: STYLES_SAMPLE,
            minutes: opts.minutes,
            jobs,
            owner_id: &owner_id,
            pairings: None,
        })?;
        if let Some(s) = &sample {
            if !quiet {
                eprint!("{}", crate::report::styles::render(s));
            }
        }
        sample
    } else {
        None
    };

    let mut report = CalibrationReport {
        owner_id,
        run_id: run_id.clone(),
        seed: opts.seed(),
        content_hash: first.content_hash.clone(),
        fixtures_hash,
        fixtures_scheme: FIXTURE_SCHEME,
        identity: identity_value,
        outcome: "success",
        error_type: None,
        error_code: None,
        error_retriable: None,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        matches: run_matches,
        minutes: opts.minutes,
        jobs,
        suites: suite_codes(&first.figures),
        wall_ms: first.wall_ms.clone(),
        bands: if change {
            informational(first.checks.clone())
        } else {
            first.checks.clone()
        },
        formations: first.formations.clone(),
        nash: nash_rating.and_then(Result::ok),
        selection: ctx.selection.clone(),
        red_card: first.red_card.clone(),
        baseline: None,
        diff: None,
        // A change run's answer is its joint word; any other run's is its range checks.
        pass: joint
            .as_ref()
            .map_or(first.pass, |j| j.word == verdict::Word::Pass),
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
        pair_verdict: None,
        units,
        results_digest,
        base: old.map(|(b, _)| b),
        verdicts,
        joint,
        kind: kind.code(),
        dropped,
        headline,
        pair_agreement: agreement,
        count: if change {
            ctx.matches
                .iter()
                .map(|(s, n)| (s.code().to_string(), *n))
                .collect()
        } else {
            BTreeMap::new()
        },
        reported,
        figures: first.figure_rows.clone(),
        accepted: acceptances,
        counts,
        registry: RegistryInfo {
            version: REGISTRY_VERSION,
            digest: ctx.bands.gate_digest(),
        },
        real: real_info,
        realism,
        provisional_margins,
        report_only_floor,
        drift: None,
        stages: stages::snapshot(run_started.elapsed()),
        rules,
        styles: None,
        substitutions: None,
    };
    if let Some(sample) = styles_sample {
        report.stages.insert(
            "styles".to_string(),
            crate::report::StageCost {
                ms: sample.wall_ms,
                peak_mb: None,
            },
        );
        report.styles = Some(sample);
    }
    // The substitutions of the first arm's matches against the real era of the rule pack's
    // limit; report only.
    if !quiet {
        let table: Option<serde_json::Value> = std::fs::read(dir.path("signatures.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        report.substitutions = crate::report::substitutions::report(
            &first.rows,
            loaded.content.rules.substitutions.limit,
            table.as_ref(),
        );
        if let Some(s) = &report.substitutions {
            eprint!("{}", crate::report::substitutions::render(s));
        }
    }
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

    report.drift = band_history(&dir, &ctx.bands, &report);
    if let (Some(d), false) = (&report.drift, quiet) {
        eprint!("{}", crate::report::history::render(d));
    }

    let code = match (&opts.pair, played.as_slice()) {
        (Some(flag), [off, on]) => ctx.compare_arms(&mut report, (flag, written), &states, off, on),
        // A change run passes only on a joint pass, any other run on its range checks: a
        // fail or a not sure exits 2.
        _ => i32::from(!report.pass) * 2,
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
    finished(
        if rejudged { "judged again" } else { "judged" },
        Some(report.pass),
        report.joint.as_ref().map(|j| j.word.code()),
        code,
    );
    Ok(code)
}

/// Adds this run's band rows to the band history and reads the drifts back. A run whose
/// content folder is outside any checkout, or with `SM_BAND_HISTORY=off`, keeps none; a
/// history that cannot be read or written is named in a warning and the run goes on.
fn band_history(
    dir: &ContentDir,
    registry: &Registry,
    report: &CalibrationReport,
) -> Option<crate::report::history::DriftInfo> {
    use crate::report::history;
    let path = history::path(dir.root())?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    let stamp = history::RunStamp {
        run: &report.run_id,
        date: history::date_of(millis),
        build: engine::build_hash(),
        content: &report.content_hash,
    };
    let points = history::points(&stamp, registry, &report.bands, report.matches);
    match history::record(&path, &points) {
        Ok(all) => Some(history::DriftInfo {
            history: path.display().to_string(),
            added: points.len(),
            drifts: history::drifts(&all),
        }),
        Err(e) => {
            tracing::warn!(signal = "calibrate.history", error = %format!("{e:#}"));
            eprintln!("band history not kept: {e:#}");
            None
        }
    }
}

/// `--suite styles`: the styles suite alone, `--matches` matches per ordered pairing. It
/// judges no band and keeps no rows: it writes its report to `report.json` in the run
/// folder, prints the table, and exits 0.
fn styles_alone(
    dir: &ContentDir,
    opts: &CalibrateOpts,
    loaded: &crate::content::Loaded,
    states: &FlagStates,
) -> anyhow::Result<i32> {
    if opts.base.is_some() || opts.base_binary.is_some() || opts.pair.is_some() {
        anyhow::bail!(
            "--suite styles runs alone: a change run (--base) already plays a styles sample, \
             and a paired run compares bands, which the styles suite does not judge"
        );
    }
    if opts.baseline.is_some() || !opts.bands.is_empty() || !opts.pairings.is_empty() {
        anyhow::bail!("--suite styles takes no --baseline, --band or --pairing");
    }
    let content = loaded.content.with_flags(states)?;
    let Some(styles_file) = content.styles.as_ref() else {
        anyhow::bail!(
            "--suite styles: the content folder {} has no styles.json",
            dir.root().display()
        );
    };
    let pairings = opts
        .style_pairings
        .iter()
        .map(|name| style_pairing(name, styles_file))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let data = data_dir();
    let owner_id = load_or_create_owner_id(&data)?;
    let millis = u64::try_from(unix_millis()).unwrap_or(u64::MAX);
    let run_id = format!("styles-{:016x}-{millis}", opts.seed());
    let run_dir = opts
        .out
        .clone()
        .unwrap_or_else(|| data.join("runs").join(&run_id));
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let jobs = opts
        .jobs
        .unwrap_or(u32::try_from(cores).unwrap_or(u32::MAX))
        .max(1);
    let report = styles::play(&styles::SuiteInput {
        content: &content,
        commentary: &loaded.commentary,
        seed: opts.seed(),
        matches: opts.matches(),
        minutes: opts.minutes,
        jobs,
        owner_id: &owner_id,
        pairings: (!pairings.is_empty()).then_some(pairings.as_slice()),
    })?
    .context("the content has no styles")?;
    eprint!("{}", crate::report::styles::render(&report));
    let record = serde_json::json!({
        "record.kind": "styles-report",
        "operation": "calibrate",
        "run.id": run_id,
        "seed": opts.seed(),
        "content.hash": hex12(&content.digest),
        "calib.matches": opts.matches(),
        "calib.minutes": opts.minutes,
        "calib.jobs": jobs,
        "calib.styles": report,
    });
    std::fs::create_dir_all(&run_dir)
        .with_context(|| format!("cannot create the run folder {}", run_dir.display()))?;
    let line = serde_json::to_string(&record)?;
    std::fs::write(run_dir.join("report.json"), format!("{line}\n"))
        .with_context(|| format!("cannot write {}", run_dir.join("report.json").display()))?;
    println!("{line}");
    tracing::info!(
        signal = "calibrate.styles",
        run.id = %run_id,
        matches = report.matches,
        played = report.played,
        dominant = report.dominant.len(),
        identity_flags = report.identity_flags.len(),
        violations = report.violations,
        gate_pass = report.gate.pass,
        pairings_outside = report.gate.pairings_outside.len()
    );
    Ok(0)
}

/// The styles of `--style-pairing` `name`, "<home> v <away>", as indices into the styles
/// file.
fn style_pairing(name: &str, styles: &engine::data::styles::Styles) -> anyhow::Result<[usize; 2]> {
    let index = |s: &str| {
        styles
            .styles
            .iter()
            .position(|x| x.name == s.trim())
            .with_context(|| {
                let all: Vec<&str> = styles.styles.iter().map(|x| x.name.as_str()).collect();
                format!(
                    "--style-pairing {name}: no style {}; the styles are {}",
                    s.trim(),
                    all.join(", ")
                )
            })
    };
    let (home, away) = name
        .split_once(" v ")
        .with_context(|| format!("--style-pairing {name}: name it as \"<home> v <away>\""))?;
    Ok([index(home)?, index(away)?])
}

/// The identity of a run: what decides its results, so a run folder of the same identity
/// resumes and one of another identity starts again.
fn run_identity(
    opts: &CalibrateOpts,
    states: &FlagStates,
    content: &str,
    fixtures_hash: &str,
    bands: &Registry,
) -> anyhow::Result<RunIdentity> {
    // nosemgrep: rust.lang.security.current-exe.current-exe -- only used to hash the build for the run's identity
    let exe = std::env::current_exe().context("cannot find this program to hash it")?;
    Ok(RunIdentity {
        build: run_folder::Build {
            hash: engine::build_hash().to_string(),
            executable_sha256: crate::bisect::cache::file_sha256(&exe)
                .context("cannot read this program to hash it")?,
        },
        content: content.to_string(),
        fixtures: fixtures_hash.to_string(),
        flags: states
            .0
            .iter()
            .map(|(n, s)| (n.clone(), s.code().to_string()))
            .collect(),
        pair: opts.pair.clone(),
        seed: opts.seed(),
        minutes: opts.minutes,
        strength_boost: bands.stronger_team.attribute_boost,
        rng_scheme: engine::rng::STREAM_SCHEME,
        fixture_scheme: FIXTURE_SCHEME.to_string(),
        measures: MEASURES_VERSION,
        registry: REGISTRY_VERSION,
        reseed: opts.reseed.filter(|&s| s != 0),
    })
}

/// Every flag the run resolved, with its owner, state and where the state came from; the
/// paired flag of a paired run is `paired`, from the command line.
fn flag_entries(
    written: &engine::data::TuningFile,
    states: &FlagStates,
    pair: Option<&str>,
) -> anyhow::Result<Vec<FlagEntry>> {
    let resolved = effective(written, states)?;
    Ok(resolved
        .iter()
        .map(|(name, (state, source))| {
            let paired = pair == Some(name.as_str());
            FlagEntry {
                name: name.clone(),
                owner: written.flags[name].owner.clone(),
                state: if paired { "paired" } else { state.code() },
                source: if paired { "cli" } else { source.code() },
            }
        })
        .collect())
}

/// The rows of every arm: their files, and the matches with a full recording.
fn rows_of_every_arm(played: &[Arm]) -> Option<RowsInfo> {
    played
        .iter()
        .filter_map(|a| a.rows_info)
        .reduce(|a, b| RowsInfo {
            format: a.format,
            measures: a.measures,
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
        })
}

/// A change run's band checks: the formations rows of one pairing are information only,
/// because the change run judges and powers each formations band pooled over every pairing.
fn informational(mut checks: Vec<BandCheck>) -> Vec<BandCheck> {
    for c in &mut checks {
        if c.suite == Suite::Formations.code() && c.pairing.is_some() {
            c.informational = Some(true);
        }
    }
    checks
}

/// Marks each row of a band the acceptances name: a not-sure row is accepted, not shown
/// equivalent, and keeps its word; a row that no longer reads not sure gets a note instead.
fn mark_accepted(rows: &mut [verdict::VerdictRow], a: &accepted::AcceptedInfo) {
    for r in rows.iter_mut().filter(|r| a.bands.contains(&r.band)) {
        if r.word == verdict::Word::NotSure {
            r.accepted = Some(true);
        } else {
            r.note = Some(format!(
                "acceptance no longer applies: it reads {}",
                r.word.code()
            ));
        }
    }
}

/// `engine-cli calibrate count`: each kind's count computed from the band file, the real
/// errors read here and the head pair agreement, beside the count the band file holds.
/// Exit 0 when every computed count equals the committed one, 1 when one differs, 2 when a
/// band's floor no count meets.
pub fn count(
    content_dir: Option<&Path>,
    opts: &CalibrateOpts,
    only: Option<Kind>,
) -> anyhow::Result<i32> {
    let dir = ContentDir::resolve(content_dir)?;
    let loaded = crate::content::load(Some(dir.root()), None, None, None)?;
    let registry = Registry::load(&dir)?;
    let pairings = fixtures::pairings(loaded.content.tactics.formations.len()).len();
    let folder = real::resolve(opts.real_data.as_deref(), dir.root());
    let reference = {
        let path = crate::real_targets::default_file(dir.root());
        path.is_file()
            .then(|| crate::real_targets::ReferenceFile::load(&path))
            .transpose()?
    };
    let samples = real::samples(&folder, &registry);
    let info = real::read_all_with(&folder, &registry, &samples, reference.as_ref());
    let units = crate::report::kinds::units(pairings);
    let computed = match crate::report::kinds::compute(&registry, &units, &|b| real_error(&info, b))
    {
        Ok(c) => c,
        Err(refused) => {
            for r in &refused {
                eprintln!(
                    "refused: {} {} {}: the {} test needs a margin of {:.5}, the band holds {:.5}",
                    r.kind.code(),
                    r.suite,
                    r.band,
                    r.which,
                    r.needed,
                    r.margin
                );
            }
            return Ok(2);
        }
    };
    let computed: BTreeMap<_, _> = computed
        .into_iter()
        .filter(|(k, _)| only.is_none_or(|o| o == *k))
        .collect();
    print!("{}", crate::report::kinds::render(&computed));
    let table = registry
        .kinds
        .as_ref()
        .context("the band file has no kinds table")?;
    let mut differ = 0;
    for (kind, suites) in &computed {
        for (suite, c) in suites {
            let held = table.get(*kind).count.get(suite).copied().unwrap_or(0);
            if held != c.count {
                differ += 1;
                println!(
                    "{} {suite}: the band file holds {held}, computed {}",
                    kind.code(),
                    c.count
                );
            }
        }
    }
    Ok(i32::from(differ > 0))
}

/// The pairs of a change run: every fixture either engine planned, kept when both finished
/// its match.
fn dropped_pairs(changed: &Arm, old: &Folded) -> crate::report::Dropped {
    let ok = |rows: &BTreeMap<FixtureKey, Row>, k: &FixtureKey| {
        rows.get(k).map(|r| r.outcome == rows::Outcome::Success)
    };
    let keys: std::collections::BTreeSet<&FixtureKey> =
        changed.rows.keys().chain(old.rows.keys()).collect();
    crate::report::Dropped::of(
        keys.into_iter()
            .map(|k| (ok(&changed.rows, k), ok(&old.rows, k))),
    )
}

/// The matches a run judged: every engine's rows, the pairs of a change run, and each
/// engine's matches with its build (the first 16 hex characters of its executable's hash).
fn match_counts(
    played: &[Arm],
    old: Option<(&BaseReport, &Folded)>,
    joint: Option<&verdict::Joint>,
    exe_sha256: &str,
) -> Counts {
    let n = |rows: usize| u32::try_from(rows).unwrap_or(u32::MAX);
    let build = exe_sha256.get(..16).unwrap_or(exe_sha256).to_string();
    let mine = n(played.iter().map(|a| a.rows.len()).sum());
    let observations = match old {
        Some((base, folded)) => vec![
            Observations {
                engine: "changed",
                build,
                matches: mine,
            },
            Observations {
                engine: "old",
                build: base.build_id.clone(),
                matches: n(folded.rows.len()),
            },
        ],
        None => vec![Observations {
            engine: "this",
            build,
            matches: mine,
        }],
    };
    Counts {
        total_matches: observations.iter().map(|o| o.matches).sum(),
        pairs: joint.map_or(0, |j| j.pairs),
        observations,
    }
}

/// Each band's real error as the pass rule will see it: the error of its real side read
/// here.
fn real_error(info: &real::RealInfo, b: &BandDef) -> Option<f64> {
    info.band(&b.band)
        .and_then(|r| r.side.as_ref())
        .map(|side| side.estimate().1)
        .filter(|e| e.is_finite())
}

/// The margin floor again, with each band's real error read here. A band under its floor
/// refuses the run before any match plays, naming the band, its margin and the margin it
/// needs.
fn floor_with_real(bands: &Registry, info: &real::RealInfo) -> anyhow::Result<()> {
    match floor_refused(bands, &|b| real_error(info, b)) {
        None => Ok(()),
        Some(why) => anyhow::bail!(why),
    }
}

/// The refusal of a run whose bands do not all hold their floor with the real errors
/// `real_se` gives; `None` when every band holds it.
fn floor_refused(bands: &Registry, real_se: &dyn Fn(&BandDef) -> Option<f64>) -> Option<String> {
    let refused = bands.floor_refusals(real_se);
    if refused.is_empty() {
        return None;
    }
    let lines: Vec<String> = refused
        .iter()
        .map(|f| format!("  {}: {}", f.band, f.reason()))
        .collect();
    Some(format!(
        "{} band(s) of {} hold a margin under the 97 percent floor with the real errors \
         read here; no match played. Widen each margin with a reason that cites a \
         noticeable error, or add real data:\n{}",
        refused.len(),
        crate::report::bands::BANDS_FILE,
        lines.join("\n")
    ))
}

/// Sets each verdict row's real value and distance from the band's real entry, or the note
/// the console prints in their place. A whole-distribution band whose real sample was read
/// reads "whole sample", its distance its value.
fn attach_real(rows: &mut [verdict::VerdictRow], info: &real::RealInfo, bands: &Registry) {
    for r in rows.iter_mut() {
        let entry = info.band(&r.band);
        let note = match entry.map(|e| e.status) {
            Some("no_counterpart") => "no real counterpart",
            Some("no_record") => "no real record",
            _ => "no real data here",
        };
        let whole = bands.band(&r.band).is_some_and(BandDef::is_distribution);
        if whole && entry.is_some_and(|e| e.status == "read") {
            r.set_whole_sample();
        } else if whole {
            r.set_real(None, note);
        } else {
            r.set_real(entry.and_then(|e| e.value), note);
        }
    }
}

/// A run's verdict: each gating band's word, each reported band's, and a change run's joint
/// word.
struct Judged {
    verdicts: Vec<verdict::VerdictRow>,
    reported: Vec<verdict::VerdictRow>,
    joint: Option<verdict::Joint>,
    agreement: Option<crate::report::agreement::RunAgreement>,
}

/// A paired judgement: the gating rows, the reported rows, the joint word, and the
/// agreement of the two sides' matches.
struct PairedJudged {
    rows: Vec<verdict::VerdictRow>,
    reported: Vec<verdict::VerdictRow>,
    joint: verdict::Joint,
    agreement: Option<crate::report::agreement::RunAgreement>,
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
    /// Matches per suite unit (per pairing, per arm) the run plays: `--matches`, or a change
    /// run's fixed count of its kind.
    matches: BTreeMap<Suite, u32>,
    /// The run's kind.
    kind: Kind,
    /// The run's start, from its `run.json`: part of every match identifier.
    millis: u64,
    run_id: String,
    /// This invocation: the ledger files its threads write carry it.
    session: String,
    /// The real-data folder the distribution bands' samples are read from.
    real_folder: real::RealFolder,
    /// The old engine's own run, started by a change run: it reads no real data.
    quiet: bool,
    /// The distribution bands' real samples, read once, in the judge stage, on first use.
    samples: std::sync::OnceLock<std::sync::Arc<real::Samples>>,
    /// Each band's real side for the pass rule, read with the real values before play.
    sides: std::sync::OnceLock<std::sync::Arc<BTreeMap<String, pass_rule::interval::Real>>>,
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
/// attributes, rules, tactics and trait catalogue content (a generated player's traits are
/// drawn from the catalogue), the generator block after the flag states, and the two default
/// clubs. Parsed values are hashed, so a change of layout alone does not
/// change it. Tuning values outside the generator block and flag states that leave the
/// generator alone do not change it: they are the change a baseline diff measures.
pub fn fixtures_hash(content: &Content, teams: &[TeamFile; 2]) -> anyhow::Result<String> {
    let mut hasher = Sha256::new();
    for part in [
        serde_json::to_vec(&content.attributes)?,
        serde_json::to_vec(&content.rules)?,
        serde_json::to_vec(&content.tactics)?,
        serde_json::to_vec(&content.traits)?,
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
    /// The checks of the bands that only report.
    reported: Vec<BandCheck>,
    /// The registry's figures beside an earlier build's values.
    figure_rows: Vec<crate::report::FigureRow>,
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
    /// Matches that ended with an engine error: every failed match but the panicked ones.
    /// They leave the bands, so a change run must fail on them, not judge without them.
    fn match_failed(&self) -> u32 {
        let failed: u32 = self.figures.values().map(|f| f.failures).sum();
        failed.saturating_sub(self.match_panicked)
    }

    /// What fails a change run outside the bands: panics, engine errors, missing results,
    /// unapplied changes, and rule violations.
    fn guards(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (n, what) in [
            (self.match_panicked, "matches panicked"),
            (self.match_failed(), "matches ended with an engine error"),
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
/// The old side of a paired judgement: its folded matches and their fixture keys by suite.
type Side<'a> = (&'a RunBuilder, &'a BTreeMap<Suite, Vec<u64>>);

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
            self.opts.seed(),
            self.matches_for(suite),
            &self.formation_names,
            &self.selected,
        )
    }

    /// Every match `suite` plays on this engine: [`Self::keyed`], each engine seed re-derived
    /// under `--reseed` when it is set. The old engine always plays [`Self::keyed`].
    fn own_keyed(&self, suite: Suite) -> Vec<Keyed> {
        let mut list = self.keyed(suite);
        if let Some(salt) = self.opts.reseed {
            for k in &mut list {
                k.engine_seed = fixtures::reseeded(k.engine_seed, salt);
            }
        }
        list
    }

    /// Matches per suite unit `suite` plays in this phase.
    fn matches_for(&self, suite: Suite) -> u32 {
        self.matches
            .get(&suite)
            .copied()
            .unwrap_or(self.opts.matches())
    }

    /// The planned fixtures of every arm and suite, and those the arms' ledgers have
    /// finished with the planned engine seed.
    fn progress(&self, arm_dirs: &[PathBuf]) -> anyhow::Result<(u32, u32)> {
        let (mut total, mut done) = (0usize, 0usize);
        for arm_dir in arm_dirs {
            let finished = run_folder::finished(arm_dir, None)?;
            for &suite in &self.suites {
                for k in self.own_keyed(suite) {
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
            run_id: &self.run_id,
            source,
            data,
            content_dir: self.dir.root(),
            seed: opts.seed(),
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
            self.fold_rows(&old.dir, true)?
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
            features: old.features,
            missing_features: old.missing_features,
            cache: BaseCache {
                hits: old.hits,
                played: old.played,
            },
            bands: checks,
        };
        Ok(Some((report, folded)))
    }

    /// The distribution bands' real samples, read on the first call and kept for the run;
    /// none in the old engine's own run, which judges nothing shown. Every caller is in the
    /// judge stage (folding the rows, or reading the real values), so the read is its time.
    fn real_samples(&self) -> std::sync::Arc<real::Samples> {
        self.samples
            .get_or_init(|| {
                std::sync::Arc::new(if self.quiet {
                    real::Samples::new()
                } else {
                    real::samples(&self.real_folder, &self.bands)
                })
            })
            .clone()
    }

    /// A builder with every selected suite planned, holding the real samples.
    fn planned_builder(&self) -> RunBuilder {
        let opts = self.opts;
        let mut builder = RunBuilder::new(self.bands.clone());
        builder.set_real_samples(self.real_samples(), opts.seed());
        if let Some(sides) = self.sides.get() {
            builder.set_real_sides(sides.clone());
        }
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
    /// files' rounding, so the bands equal those folded from the statistics files; its
    /// extras (measures, players' running, clubs) come with it. `old` marks the old engine's
    /// rows, whose measures the test seam `--inject-failure base-without-measures` drops.
    fn fold_rows(&self, dir: &Path, old: bool) -> anyhow::Result<Folded> {
        let drop_measures =
            old && self.opts.inject_failure == Some(InjectFailure::BaseWithoutMeasures);
        let done = run_folder::finished(dir, None)?;
        let read = rows::read_rows(dir, &done)?;
        let mut builder = self.planned_builder();
        let mut keys: BTreeMap<Suite, Vec<u64>> = BTreeMap::new();
        let mut planned = BTreeMap::new();
        let mut recorded = Recorded::default();
        let mut panicked = 0;
        for &suite in &self.suites {
            let mut outliers = 0;
            let list = if old {
                self.keyed(suite)
            } else {
                self.own_keyed(suite)
            };
            for k in list {
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
                builder.set_league(suite, k.fixture.league);
                let club = |c: usize| (k.fixture.league, u32::try_from(c).unwrap_or(u32::MAX));
                let measured = row.measured.as_deref();
                builder.set_extras(
                    suite,
                    Extras {
                        measures: match measured {
                            Some(m) if !drop_measures => m.measures,
                            _ => rows::Measures::default(),
                        },
                        physical: measured.map(|m| m.physical).unwrap_or_default(),
                        shape: measured.map_or([[0.0; SHAPE_FIELDS]; 2], |m| m.shape),
                        clubs: k.fixture.clubs.map(club),
                    },
                );
                planned.insert(k.key, row.clone());
            }
            builder.set_outliers(suite, outliers);
        }
        let rows_info = RowsInfo {
            format: ROWS_FORMAT,
            measures: cfg!(feature = "measures"),
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
            self.opts.seed(),
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
        let config = MatchConfig::new(opts.seed(), opts.minutes, &content, [team_a, team_b])?;

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
        let mut reported = builder.reported_checks();
        let figure_rows = builder.figure_rows();
        // `--band` judges only the named bands; each suite's time budget stays.
        if !opts.bands.is_empty() {
            checks.retain(|c| c.band == "wall_ms" || opts.bands.contains(&c.band));
            reported.retain(|c| opts.bands.contains(&c.band));
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
        // A band that only reports never fails the run.
        let pass = checks
            .iter()
            .all(|c| c.pass || self.bands.report_only(&c.band).is_some())
            && change_never_applied == 0
            && match_without_stats == 0
            && match_panicked == 0;
        checks
            .sort_by(|a, b| (&a.suite, &a.band, &a.pairing).cmp(&(&b.suite, &b.band, &b.pairing)));
        reported
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
            reported,
            figure_rows,
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
                self.fold_rows(arm_dir, false)?
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

    /// Judges the run: a change run (`change`) by the paired max-t test against the old
    /// engine's matches, a plain run by each band's range, a paired flag run not here.
    fn judge_run(&self, first: &Arm, old: Option<&Folded>, change: bool) -> Judged {
        let opts = self.opts;
        let _judge = stages::enter(Stage::Judge);
        let why = |b: &str| self.bands.report_only(b);
        if !change {
            let (verdicts, mut reported) = if opts.pair.is_none() {
                (
                    verdict::range_only(&first.checks),
                    verdict::range_rows(&first.reported, None),
                )
            } else {
                (Vec::new(), Vec::new())
            };
            verdict::mark_report_only(&mut reported, None, why);
            return Judged {
                verdicts,
                reported,
                joint: None,
                agreement: None,
            };
        }
        let old = old.map(|o| (&o.builder, &o.keys));
        let PairedJudged {
            rows,
            reported,
            joint,
            agreement,
        } = self.judge_paired(first, old);
        self.log_verdict(&rows, &joint);
        Judged {
            verdicts: rows,
            reported,
            joint: Some(joint),
            agreement,
        }
    }

    /// The rows of `first` judged against `old` on the fixtures both played: the gating
    /// rows, the reported rows and the joint word. The old side is the old engine of a change
    /// run or the off arm of a paired run.
    fn judge_paired(&self, first: &Arm, old: Option<Side<'_>>) -> PairedJudged {
        let opts = self.opts;
        let why = |b: &str| self.bands.report_only(b);
        // The gating rows are judged exactly as if no band only reported: the reported
        // rows get a bootstrap and a threshold of their own, so they never move `c`.
        let (sets, mut rows) = self.paired(first, old);
        keep_bands(&mut rows, &opts.bands);
        let (gating, reported) = self.split_paired(rows, first, old.map(|o| o.0));
        let alpha = self.bands.pass_rule.map_or(0.05, |p| p.alpha);
        let target = |band: &str| self.target_of(band);
        let (found, c) = verdict::bootstrap(&sets, &gating, opts.seed(), verdict::RESAMPLES);
        let (mut rows, mut joint) = verdict::judge(&sets, &gating, &found, c, first.guards());
        verdict::set_old_words(&mut rows, &gating, &found, c, alpha, &target);
        let agreement = crate::report::agreement::of_rows(&gating, opts.seed());
        let (found_r, c_r) = verdict::bootstrap(&sets, &reported, opts.seed(), verdict::RESAMPLES);
        let (mut reported_rows, _) = verdict::judge(&sets, &reported, &found_r, c_r, Vec::new());
        verdict::set_old_words(&mut reported_rows, &reported, &found_r, c_r, alpha, &target);
        let mut reported = reported_rows;
        // The rows a paired test cannot judge: their range alone, on the changed engine.
        if let Some((old, _)) = old {
            for spec in first.builder.row_specs(true) {
                let selected = opts.bands.is_empty() || opts.bands.contains(&spec.def.band);
                let Some(note) = self.range_reason(spec.def, spec.suite, first, old) else {
                    continue;
                };
                let only = spec.def.report_only.is_some();
                // A build without the measures judges a reported measures band nowhere.
                if !selected || (only && note == verdict::NO_MEASURES_NOTE) {
                    continue;
                }
                let mut row = verdict::range_rows(&[first.builder.check(&spec)], Some(note));
                for r in &mut row {
                    r.old_reason = Some(self.no_old_word(note));
                }
                if only { &mut reported } else { &mut rows }.extend(row);
            }
        }
        // A version 6 band's word is its word against real football by the pass rule on the
        // changed engine; the paired test fills the change columns only.
        verdict::set_rule_words(&mut rows, &first.checks);
        verdict::set_rule_words(&mut reported, &first.reported);
        joint.word = verdict::joint_word(&rows, &joint.guards);
        verdict::mark_report_only(&mut reported, None, why);
        PairedJudged {
            rows,
            reported,
            joint,
            agreement,
        }
    }

    /// A paired run's comparison of its two arms, into `report`: the on arm judged as a
    /// change run judges its engine, with the off arm in the old engine's place. The verdict
    /// is the comparison's answer; the exit code says whether both arms can be trusted.
    fn compare_arms(
        &self,
        report: &mut CalibrationReport,
        (flag, written): (&String, &engine::data::TuningFile),
        states: &FlagStates,
        off: &Arm,
        on: &Arm,
    ) -> i32 {
        let PairedJudged {
            mut rows,
            joint,
            agreement,
            ..
        } = self.judge_paired(on, Some((&off.builder, &off.keys)));
        rows.sort_by(|a, b| (&a.suite, &a.band, &a.pairing).cmp(&(&b.suite, &b.band, &b.pairing)));
        let pair = compare::verdict(&rows, joint.word, &off.guard(), &on.guard());
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
        eprint!(
            "paired run of flag {flag}\n{}{}",
            verdict::render_table(&rows, Some(&joint)),
            compare::render_verdict(flag, &pair)
        );
        tracing::info!(
            signal = "calibrate.pair",
            run.id = %report.run_id,
            flag = %flag,
            verdict = pair.verdict.code()
        );
        if let Some(a) = &agreement {
            eprint!("{}", crate::report::agreement::render(a));
        }
        report.pair_agreement = agreement;
        report.compare = Some(rows);
        report.verdict = Some(pair.verdict);
        report.pair_verdict = Some(pair);
        if off.trusted() && on.trusted() { 0 } else { 2 }
    }

    /// The target a band's old-engine word measures closeness to: its design value, or the
    /// real value read here before play.
    fn target_of(&self, band: &str) -> Option<f64> {
        let def = self.bands.band(band)?;
        match def.rule.as_ref()?.target {
            crate::report::bands::Target::Design { value, .. } => Some(value),
            crate::report::bands::Target::Real => {
                self.sides.get()?.get(band).map(|s| s.estimate().0)
            }
        }
    }

    /// Why a row judged on the changed engine alone has no word against the old engine.
    fn no_old_word(&self, note: &str) -> String {
        if note == verdict::BASE_WITHOUT_MEASURES {
            let rev = self
                .opts
                .base
                .clone()
                .unwrap_or_else(|| "the base binary".to_string());
            format!("the old engine of {rev} has no measures feature")
        } else if note == verdict::PER_CLUB {
            "a per-club spread has no paired test".to_string()
        } else {
            note.to_string()
        }
    }

    /// Why a change run judges `def` in `suite` on its range alone, on the changed engine:
    /// a per-club spread has no paired form; a measures band needs the measures in both
    /// engines' rows. `None` when the paired test judges it.
    fn range_reason(
        &self,
        def: &BandDef,
        suite: Suite,
        first: &Arm,
        old: &RunBuilder,
    ) -> Option<&'static str> {
        if matches!(def.measure, Measure::ClubRange { .. }) {
            Some(verdict::PER_CLUB)
        } else if !def.needs_measures() {
            None
        } else if !first.builder.has_measures(suite) {
            Some(verdict::NO_MEASURES_NOTE)
        } else if !old.has_measures(suite) {
            Some(verdict::BASE_WITHOUT_MEASURES)
        } else {
            None
        }
    }

    /// The paired rows the paired test judges, split into the bands that gate and those
    /// that only report; a row judged on its range alone is in neither.
    fn split_paired(
        &self,
        rows: Vec<verdict::PairedRow>,
        first: &Arm,
        old: Option<&RunBuilder>,
    ) -> (Vec<verdict::PairedRow>, Vec<verdict::PairedRow>) {
        let Some(old) = old else {
            return (Vec::new(), Vec::new());
        };
        rows.into_iter()
            .filter(|r| {
                self.bands
                    .band(&r.band)
                    .is_some_and(|d| self.range_reason(d, r.suite, first, old).is_none())
            })
            .partition(|r| self.bands.report_only(&r.band).is_none())
    }

    /// One structured event for a change run's joint word with its counts, and one for each
    /// band that failed, so a log reader sees the answer without parsing the table.
    fn log_verdict(&self, rows: &[verdict::VerdictRow], joint: &verdict::Joint) {
        let count = |w: verdict::Word| rows.iter().filter(|r| r.word == w).count();
        tracing::info!(
            signal = "calibrate.verdict",
            run.id = %self.run_id,
            joint = joint.word.code(),
            pass = count(verdict::Word::Pass),
            fail = count(verdict::Word::Fail),
            not_sure = count(verdict::Word::NotSure),
            critical = joint.critical,
            pairs = joint.pairs,
            guards = %joint.guards.join("; ")
        );
        for r in rows.iter().filter(|r| r.word == verdict::Word::Fail) {
            tracing::warn!(
                signal = "calibrate.verdict_failed",
                run.id = %self.run_id,
                band = %r.band,
                suite = %r.suite,
                pairing = r.pairing.as_deref(),
                value = r.value,
                lo = r.lo,
                hi = r.hi,
                diff = r.diff,
                se = r.se,
                smallest_shift = r.smallest_shift
            );
        }
    }

    /// The paired rows of a change run: the changed engine's matches and the old engine's
    /// on the fixtures both finished.
    fn paired(
        &self,
        changed: &Arm,
        old: Option<Side<'_>>,
    ) -> (Vec<verdict::SuiteSet>, Vec<verdict::PairedRow>) {
        let Some(old) = old else {
            return (Vec::new(), Vec::new());
        };
        verdict::paired((&changed.builder, &changed.keys), old)
    }

    /// Writes the run's `target.json`: its kind, its matches per suite, and the registry the
    /// run is judged with.
    fn write_target(&self, dir: &Path, change: bool) -> anyhow::Result<()> {
        run_folder::write_target(
            dir,
            &Target {
                version: TARGET_VERSION,
                change,
                kind: self.kind.code().to_string(),
                target: self
                    .matches
                    .iter()
                    .map(|(s, n)| (s.code().to_string(), *n))
                    .collect(),
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

/// Keeps the rows of the bands `--band` selects; every row when it selects none.
fn keep_bands(rows: &mut Vec<verdict::PairedRow>, bands: &[String]) {
    if !bands.is_empty() {
        rows.retain(|r| bands.contains(&r.band));
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

        let mut traits = content.clone();
        traits.traits.traits.pop();
        assert_ne!(
            base,
            fixtures_hash(&traits, &teams).unwrap(),
            "the trait catalogue"
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

    #[test]
    fn a_band_under_its_floor_with_the_real_error_read_here_refuses_the_run() {
        let doc = serde_json::json!({
            "schema_version": 6,
            "sample_size": 1000,
            "wall_minutes_per_sample": 30.0,
            "stronger_team": {"attribute_boost": 1.15},
            "generalisation_floor": 2,
            "pass_rule": {"alpha": 0.05, "sound_pass": 0.97},
            "kinds": crate::report::kinds::design_kinds(),
            "bands": [{
                "band": "goals_per_match",
                "suites": ["equal"],
                "measure": {"kind": "mean", "per": "match", "of": ["goals.home", "goals.away"]},
                "target": {"real": {}},
                "margin": {"value": 0.4, "league_spread": 0.0, "noticeable_error": 0.4,
                           "reason": "a player notices it", "spread_reason": "one population",
                           "design_choice": true},
                "definition_version": 1,
                "population": "generated leagues",
                "estimator": "mean per match",
                "resampling": {"engine": "league", "real": "match"},
                "min_exposure": {"unit": "matches", "value": 100.0},
                "rare": false,
                "engine_spread": {"per_match_sd": 1.7, "between_league_sd": 0.1, "measured": "a fixture"},
                "real": {"source": {"dataset": "a fixture", "file": "m.csv", "key": "hg+ag"},
                         "sample": 100, "date": "2026-10-21"},
                "evidence": "calibration",
                "confidence": {"measurement": {"count": 100},
                               "generalisation": {"competitions": 1, "seasons": 1, "clubs": 20}},
                "condition": "dry",
                "gates": true
            }]
        });
        let bands = Registry::from_bytes(doc.to_string().as_bytes()).unwrap();
        // The engine's error alone holds the floor; a large real error does not.
        assert_eq!(floor_refused(&bands, &|_| Some(0.001)), None);
        let why = floor_refused(&bands, &|_| Some(0.2)).unwrap();
        assert!(why.contains("1 band(s)"), "{why}");
        assert!(
            why.contains("goals_per_match: margin 0.4000 is below the 97 percent floor"),
            "{why}"
        );
        assert!(why.contains("real error 0.2000"), "{why}");
    }
}
