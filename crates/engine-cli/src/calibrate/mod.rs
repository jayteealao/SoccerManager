//! `engine-cli calibrate`: play many matches with the AI manager on both sides, in worker
//! processes of this binary, and write one run report that checks the aggregate figures
//! against the accepted realism bands.
//!
//! Run folder: `report.json`, `stats/<match.id>.json` for every match, and
//! `events/<match.id>.jsonl` for the matches kept (outliers by default).

pub mod fixtures;
pub mod worker;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

use anyhow::Context;
use engine::observe::identity::{data_dir, load_or_create_owner_id};
use engine::observe::{emit_line, machine_hash, read_stats, unix_millis, write_record_at};
use engine::{ContentDir, MatchConfig};

use crate::cli::{CalibrateOpts, KeepEvents, SuiteArg};
use crate::report::bands::Bands;
use crate::report::{CalibrationReport, RunBuilder, Suite};

/// Matches the single-thread benchmark times after its warm-up.
const BENCH_MATCHES: u32 = 5;

pub fn run(content_dir: Option<&Path>, opts: &CalibrateOpts) -> anyhow::Result<i32> {
    if opts.matches == 0 {
        anyhow::bail!("--matches must be at least 1");
    }
    let dir = ContentDir::resolve(content_dir)?;
    if opts.worker {
        let suite = match opts.suite {
            SuiteArg::Equal => Suite::Equal,
            SuiteArg::Strength => Suite::Strength,
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
        });
    }

    let bands = Bands::load(&dir)?;
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
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let jobs = opts
        .jobs
        .unwrap_or(u32::try_from(cores).unwrap_or(u32::MAX))
        .clamp(1, opts.matches);
    let suites: Vec<Suite> = match opts.suite {
        SuiteArg::All => Suite::ALL.to_vec(),
        SuiteArg::Equal => vec![Suite::Equal],
        SuiteArg::Strength => vec![Suite::Strength],
    };
    let exe = std::env::current_exe().context("cannot find this program to start workers")?;

    let started = Instant::now();
    let mut wall = BTreeMap::new();
    let mut workers_failed = 0u32;
    for &suite in &suites {
        let suite_started = Instant::now();
        let children = (0..jobs)
            .map(|shard| {
                worker_command(
                    &exe,
                    dir.root(),
                    opts,
                    suite,
                    (shard, jobs),
                    &run_dir,
                    millis,
                )
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
                    run.id = %run_id,
                    suite = suite.code(),
                    code = ?status.code()
                );
            }
        }
        let wall_ms = u64::try_from(suite_started.elapsed().as_millis()).unwrap_or(u64::MAX);
        tracing::info!(
            signal = "calibrate.suite",
            run.id = %run_id,
            suite = suite.code(),
            matches = opts.matches,
            wall_ms,
            jobs
        );
        wall.insert(suite, wall_ms);
    }
    let suites_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    // The single-thread figure, on the default teams, once the workers are done.
    let loaded = crate::content::load(Some(dir.root()), None, None)?;
    let [team_a, team_b] = &loaded.teams;
    let config = MatchConfig::new(opts.seed, opts.minutes, &loaded.content, [team_a, team_b])?;
    let bench = crate::bench::measure(&config, BENCH_MATCHES)?;

    let events_dir = run_dir.join("events");
    let events_written = count_files(&events_dir);
    let mut builder = RunBuilder::new(bands);
    for &suite in &suites {
        builder.plan(suite, opts.matches);
        for fixture in fixtures::fixtures(opts.matches) {
            let id = fixtures::match_id(opts.seed, suite, fixture.index, millis);
            let boosted = (suite == Suite::Strength).then(|| fixture.boosted_side());
            match read_stats(&run_dir.join("stats").join(format!("{id}.json"))) {
                Ok(stats) => {
                    let outlier = builder.add(suite, stats, boosted);
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
    let mut checks = builder.checks(&figures, &wall);
    let change_never_applied = builder.change_never_applied();
    let match_without_stats = builder.missing;
    for (counter, value) in [
        ("darkpath.change_never_applied", change_never_applied),
        ("darkpath.match_without_stats", match_without_stats),
    ] {
        if value > 0 {
            tracing::warn!(signal = "calibrate.darkpath", run.id = %run_id, counter, value);
        }
    }
    let pass = checks.iter().all(|c| c.pass)
        && change_never_applied == 0
        && match_without_stats == 0
        && workers_failed == 0;
    checks.sort_by(|a, b| (&a.suite, &a.band).cmp(&(&b.suite, &b.band)));
    let mut wall_ms: BTreeMap<String, u64> = wall
        .iter()
        .map(|(s, &ms)| (s.code().to_string(), ms))
        .collect();
    wall_ms.insert("total".into(), suites_ms);
    let report = CalibrationReport {
        owner_id,
        run_id: run_id.clone(),
        seed: opts.seed,
        content_hash: config.content_hash.clone(),
        outcome: if workers_failed == 0 {
            "success"
        } else {
            "failure"
        },
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        matches: opts.matches,
        minutes: opts.minutes,
        jobs,
        suites: figures
            .into_iter()
            .map(|(s, f)| (s.code().to_string(), f))
            .collect(),
        wall_ms,
        bands: checks,
        pass,
        bench_matches: BENCH_MATCHES,
        match_wall_ms: bench.match_wall_ms,
        ticks_per_match: bench.ticks_per_match,
        cpu_us_per_tick: bench.cpu_us_per_tick,
        cpu_ms: bench.cpu_ms,
        peak_mem_mb: bench.peak_mem_mb,
        change_never_applied,
        match_without_stats,
        violations: builder.violations(),
        events_written,
        events_kept,
        machine_hash: machine_hash(),
        cpu_model: crate::bench::cpu_model(),
    };
    write_record_at(
        &run_dir.join("report.json"),
        &format!("runs/{run_id}/report.json"),
        &report,
    )?;
    emit_line(&report)?;
    Ok(if pass { 0 } else { 2 })
}

/// The command line of one worker. Workers log warnings and errors only unless `SM_LOG`
/// says otherwise, and their standard output is discarded.
fn worker_command(
    exe: &Path,
    content: &Path,
    opts: &CalibrateOpts,
    suite: Suite,
    (shard, shards): (u32, u32),
    run_dir: &Path,
    millis: u64,
) -> Command {
    let mut cmd = Command::new(exe);
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
