//! The one-process runner of `engine-cli calibrate`: a panicking match is caught and
//! recorded as failed while the run goes on; every match leaves one compact row of
//! unrounded counts; only the 1-in-16 sample and the outliers get statistics and event
//! files; the single-thread figure leaves out the threads' matches; and the worker-process
//! runner is gone. Every run here is short.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Output;

use common::{RecordSchemas, TestRow, bin, read_rows, record, stems, temp};
use serde_json::Value;

fn calibrate(data: &Path, run: &Path, args: &[&str]) -> Output {
    bin(data)
        .arg("calibrate")
        .args(args)
        .arg("--out")
        .arg(run)
        .output()
        .unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The saved report, checked against its schema.
fn report(run: &Path) -> Value {
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    RecordSchemas::load()
        .report(&report)
        .unwrap_or_else(|e| panic!("{e}\n{report}"));
    report
}

/// The fixture keys of the files in `dir`: the first 16 characters of each match id.
fn keys_of(dir: &Path) -> BTreeSet<String> {
    stems(dir)
        .into_iter()
        .map(|s| s[..16].to_string())
        .collect()
}

/// `x` rounded to `places` decimals, as `docs/reference/data-files.md` says the statistics
/// files round it.
fn round(x: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (x * scale).round() / scale
}

/// `part` over `whole` as a percentage with one decimal; 0 when `whole` is 0.
fn pct(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        round(100.0 * part as f64 / whole as f64, 1)
    }
}

const SAMPLE: u64 = 1;
const ERROR: u64 = 2;
const VIOLATION: u64 = 4;
const EXTREME: u64 = 8;

#[test]
fn a_panicking_match_is_recorded_as_failed_and_the_other_matches_finish() {
    let data = temp("engine-cli-runner", "panic");
    let run = data.join("run");
    let out = calibrate(
        &data,
        &run,
        &[
            "--seed",
            "1",
            "--suite",
            "equal",
            "--matches",
            "4",
            "--minutes",
            "5",
            "--jobs",
            "2",
            "--inject-failure",
            "panic",
        ],
    );
    let text = stderr(&out);
    // The verdict fails.
    assert_eq!(out.status.code(), Some(2), "{text}");
    let report = report(&run);
    assert_eq!(report["calib.runner"], "threads");
    assert_eq!(report["darkpath.match_panicked"], 1, "{report}");
    assert_eq!(report["calib.pass"], false);
    assert_eq!(report["outcome"], "success", "the run itself finished");
    // The other matches finished: every planned match has a record, one of them failed.
    assert_eq!(report["calib.suites"]["equal"]["recorded"], 4);
    assert_eq!(report["calib.suites"]["equal"]["failures"], 1);
    assert_eq!(report["darkpath.match_without_stats"], 0);

    let rows = read_rows(&run);
    assert_eq!(rows.len(), 4);
    let panicked: Vec<&TestRow> = rows
        .iter()
        .filter(|r| r.get("outcome").int() == 2)
        .collect();
    assert_eq!(panicked.len(), 1);
    let key = panicked[0].key();
    assert!(
        rows.iter()
            .filter(|r| r.key() != key)
            .all(|r| r.get("outcome").int() == 0),
        "{rows:?}"
    );
    // Standard error names the key and the panic.
    assert!(
        text.contains(&format!("match {key} (equal) panicked: injected panic")),
        "{text}"
    );
    assert!(text.contains("recorded as failed"), "{text}");
    // The failed match has a statistics file with the panic's error keys.
    let stats_dir = run.join("stats");
    let id = stems(&stats_dir)
        .into_iter()
        .find(|s| s.starts_with(&key))
        .expect("the panicked match has a statistics file");
    let stats = record(&std::fs::read_to_string(stats_dir.join(format!("{id}.json"))).unwrap());
    RecordSchemas::load().stats(&stats).unwrap();
    assert_eq!(stats["outcome"], "error");
    assert_eq!(stats["error.type"], "panic");
    assert_eq!(stats["error.code"], "match-panicked");
    assert_eq!(stats["error.retriable"], false);
    assert_ne!(panicked[0].get("record.reasons").int() & ERROR, 0);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn every_match_leaves_one_unrounded_row_and_the_statistics_files_keep_their_rounding() {
    let data = temp("engine-cli-runner", "rows");
    let run = data.join("run");
    let out = calibrate(
        &data,
        &run,
        &[
            "--seed",
            "6",
            "--suite",
            "equal",
            "--matches",
            "8",
            "--minutes",
            "10",
            "--jobs",
            "2",
            "--keep-events",
            "all",
        ],
    );
    assert!(matches!(out.status.code(), Some(0 | 2)), "{}", stderr(&out));
    let report = report(&run);
    let rows = read_rows(&run);
    assert_eq!(rows.len(), 8);
    assert_eq!(report["calib.rows"]["rows"], 8);
    assert_eq!(report["calib.rows"]["format"], 1);
    // One row per planned key: the ledger's keys.
    let row_keys: BTreeSet<String> = rows.iter().map(TestRow::key).collect();
    assert_eq!(row_keys.len(), 8);
    assert_eq!(row_keys, keys_of(&run.join("stats")));

    let mut more_than_two_decimals = 0;
    for row in &rows {
        let id = stems(&run.join("stats"))
            .into_iter()
            .find(|s| s.starts_with(&row.key()))
            .unwrap();
        let stats =
            record(&std::fs::read_to_string(run.join("stats").join(format!("{id}.json"))).unwrap());
        let int = |name: &str| row.get(name).int();
        // Counts, numerators and denominators, unrounded.
        assert_eq!(
            stats["goals"],
            serde_json::json!([int("goals.home"), int("goals.away")])
        );
        assert_eq!(
            stats["stats.passes"],
            serde_json::json!([int("passes.home"), int("passes.away")])
        );
        assert_eq!(
            stats["passes.completed"],
            serde_json::json!([int("passes_completed.home"), int("passes_completed.away")])
        );
        // The statistics file keeps today's rounding of the same counts.
        let possession = int("possession_ticks.home") + int("possession_ticks.away");
        assert_eq!(
            stats["stats.possession_pct"],
            serde_json::json!([
                pct(int("possession_ticks.home"), possession),
                pct(int("possession_ticks.away"), possession)
            ])
        );
        assert_eq!(
            stats["stats.pass_accuracy_pct"],
            serde_json::json!([
                pct(int("passes_completed.home"), int("passes.home")),
                pct(int("passes_completed.away"), int("passes.away"))
            ])
        );
        let xg = [row.get("xg.home").float(), row.get("xg.away").float()];
        assert_eq!(
            stats["stats.xg"],
            serde_json::json!([round(xg[0], 2), round(xg[1], 2)])
        );
        assert_eq!(
            stats["stats.ball_in_play_s"],
            (int("live_ticks") + 25) / 50,
            "{id}"
        );
        more_than_two_decimals += xg.iter().filter(|&&x| round(x, 2) != x).count();
    }
    assert!(more_than_two_decimals > 0, "the rows hold unrounded xG");
    let _ = std::fs::remove_dir_all(&data);
}

/// The per-match measures of the percentile rule, as `docs/reference/cli.md` names them.
fn measures(r: &TestRow) -> [f64; 12] {
    let both = |name: &str| {
        (r.get(&format!("{name}.home")).int() + r.get(&format!("{name}.away")).int()) as f64
    };
    let share = |part: f64, whole: f64| if whole == 0.0 { 0.0 } else { part / whole };
    [
        both("goals"),
        both("shots"),
        both("shots_on_target"),
        r.get("xg.home").float() + r.get("xg.away").float(),
        both("passes"),
        share(both("passes_completed"), both("passes")),
        share(
            r.get("possession_ticks.home").int() as f64,
            both("possession_ticks"),
        ),
        both("yellow"),
        both("red"),
        both("corners"),
        both("throw_ins"),
        both("goal_kicks"),
    ]
}

#[test]
fn only_the_sample_and_the_outliers_have_a_full_recording() {
    let data = temp("engine-cli-runner", "record");
    let run = data.join("run");
    let out = calibrate(
        &data,
        &run,
        &[
            "--seed",
            "3",
            "--suite",
            "all",
            "--matches",
            "2",
            "--minutes",
            "5",
            "--jobs",
            "1",
        ],
    );
    assert!(matches!(out.status.code(), Some(0 | 2)), "{}", stderr(&out));
    let report = report(&run);
    // One thread: one row file, blocks in the order the matches were played.
    let rows = read_rows(&run);
    assert_eq!(rows.len(), 114);
    assert_eq!(
        rows.iter().map(|r| &r.file).collect::<BTreeSet<_>>().len(),
        1
    );

    // The percentile rule, recomputed in play order per suite.
    let mut seen: BTreeMap<String, Vec<Vec<f64>>> = BTreeMap::new();
    let mut extreme_count = 0;
    for row in &rows {
        if row.get("outcome").int() != 0 {
            continue;
        }
        let lists = seen
            .entry(row.suite.clone())
            .or_insert_with(|| vec![Vec::new(); 12]);
        let values = measures(row);
        let extreme = values.iter().zip(lists.iter()).any(|(&v, l)| {
            let n = l.len();
            n >= 100 && (v < l[n.div_ceil(100) - 1] || v > l[(99 * n).div_ceil(100) - 1])
        });
        assert_eq!(
            row.get("record.reasons").int() & EXTREME != 0,
            extreme,
            "{}",
            row.key()
        );
        extreme_count += u64::from(extreme);
        for (v, l) in values.into_iter().zip(lists.iter_mut()) {
            let at = l.partition_point(|&x| x < v);
            l.insert(at, v);
        }
    }

    let mut recorded = BTreeSet::new();
    for row in &rows {
        let reasons = row.get("record.reasons").int();
        let key = row.key();
        let sampled = row.get("fixture.key").int() % 16 == 0;
        assert_eq!(reasons & SAMPLE != 0, sampled, "{key}");
        let failed = row.get("outcome").int() != 0 || row.get("change_never_applied").int() > 0;
        assert_eq!(reasons & ERROR != 0, failed, "{key}");
        assert_eq!(
            reasons & VIOLATION != 0,
            row.get("validate.violations").int() > 0,
            "{key}"
        );
        if reasons != 0 {
            recorded.insert(key);
        }
    }
    // The event files and the statistics files are exactly the recorded matches.
    assert_eq!(keys_of(&run.join("events")), recorded);
    assert_eq!(keys_of(&run.join("stats")), recorded);
    let r = &report["calib.rows"]["recorded"];
    assert_eq!(r["matches"], recorded.len() as u64);
    assert_eq!(r["extreme"], extreme_count);
    assert_eq!(r["all"], 0);
    assert_eq!(report["events.files_written"], recorded.len() as u64);
    assert_eq!(report["events.files_kept"], recorded.len() as u64);
    // Far fewer matches than all of them are recorded.
    assert!(recorded.len() < rows.len(), "{recorded:?}");
    let _ = std::fs::remove_dir_all(&data);
}

/// `bench.peak_mem_mb` is the single-thread figure: the process peak after the default
/// teams are timed on one thread. The matches of every thread share that process, so the
/// figure must be taken before they play, or it would hold their memory too and no longer
/// compare with the same figure of `engine-cli bench`, which plays nothing else.
#[test]
fn the_single_thread_peak_memory_leaves_out_the_matches_of_the_threads() {
    let data = temp("engine-cli-runner", "bench-memory");
    let dir = data.join("threads");
    let out = calibrate(
        &data,
        &dir,
        &[
            "--seed",
            "5",
            "--suite",
            "equal",
            "--matches",
            "16",
            "--minutes",
            "10",
            "--jobs",
            "8",
        ],
    );
    assert!(matches!(out.status.code(), Some(0 | 2)), "{}", stderr(&out));
    if !cfg!(windows) {
        // Peak memory is measured only on Windows; elsewhere the figure is null.
        assert!(report(&dir)["bench.peak_mem_mb"].is_null());
        let _ = std::fs::remove_dir_all(&data);
        return;
    }
    let threads = report(&dir)["bench.peak_mem_mb"]
        .as_f64()
        .expect("a peak memory figure");
    let out = common::bin(&data)
        .args(["bench", "--seed", "5", "--minutes", "10", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let alone = record(&String::from_utf8_lossy(&out.stdout))["bench.peak_mem_mb"]
        .as_f64()
        .expect("a peak memory figure");
    assert!(
        threads <= alone * 2.0,
        "calibrate {threads} MB against bench alone {alone} MB"
    );
    let _ = std::fs::remove_dir_all(&data);
}

/// The worker-process runner and its hidden options are gone: each option is refused as a
/// usage error that names it (exit 1, as every usage error), and a plain run plays on
/// threads.
#[test]
fn the_old_worker_path_is_gone() {
    let data = temp("engine-cli-runner", "no-workers");
    for gone in ["--worker-processes", "--worker"] {
        let out = calibrate(&data, &data.join("refused"), &["--seed", "1", gone]);
        assert_eq!(out.status.code(), Some(1), "{gone}: {}", stderr(&out));
        let text = stderr(&out);
        assert!(text.contains("unexpected argument"), "{gone}: {text}");
        assert!(text.contains(gone), "{gone}: {text}");
    }
    let run = data.join("run");
    let out = calibrate(
        &data,
        &run,
        &[
            "--seed",
            "2",
            "--suite",
            "equal",
            "--matches",
            "2",
            "--minutes",
            "5",
            "--jobs",
            "1",
        ],
    );
    assert!(matches!(out.status.code(), Some(0 | 2)), "{}", stderr(&out));
    assert_eq!(report(&run)["calib.runner"], "threads");
    let _ = std::fs::remove_dir_all(&data);
}
