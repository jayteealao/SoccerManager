//! `engine-cli`: simulate a match headless, benchmark the engine, calibrate it over many
//! matches, check it against the replay gate, fit the fast model, or generate teams.

mod advice;
mod bench;
mod bisect;
mod calibrate;
mod cli;
mod content;
mod engines;
mod fast_model;
mod gate;
mod generate;
mod guard;
mod launch;
mod record;
mod replay;
mod replay_inputs;
mod report;
mod resimulate;
mod resume;
mod serve;
mod simulate;
mod state_files;
mod stream_run;
mod trace_file;
mod web;

use std::io::IsTerminal;
use std::time::Instant;

use clap::Parser;
use clap::error::ErrorKind;
use engine::EngineError;
use engine::observe::identity::{MatchId, data_dir, load_or_create_owner_id};
use engine::observe::{FailureRecord, emit_line, machine_hash, unix_millis};
use tracing_subscriber::EnvFilter;

fn main() {
    // A usage error exits 1, like any other run error: exit 2 is only a verdict (frames
    // differ, builds differ, hashes differ, a rule broken). `--help` and `--version` exit 0.
    let args = match cli::Cli::try_parse() {
        Ok(args) => args,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => err.exit(),
            _ => {
                let _ = err.print();
                std::process::exit(1);
            }
        },
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("SM_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .init();
    let content_dir = args.content_dir.as_deref();
    // `simulate` and `bench` print a structured record when they fail; the rest print prose.
    let on_failure = match &args.command {
        cli::Command::Simulate(opts) => Some(("match-stats", "simulate", opts.seed)),
        cli::Command::Bench(opts) => Some(("run-report", "benchmark", opts.seed)),
        _ => None,
    };
    let started = Instant::now();
    let result = match args.command {
        cli::Command::Simulate(opts) => simulate::run(content_dir, &opts),
        cli::Command::Bench(opts) => bench::run(content_dir, &opts),
        cli::Command::Generate(opts) => generate::run(content_dir, &opts),
        cli::Command::Serve(opts) => serve::run(content_dir, &opts),
        cli::Command::Launch(opts) => launch::run(content_dir, &opts),
        cli::Command::Record(opts) => record::run(content_dir, &opts),
        cli::Command::Replay(opts) => replay::run(&opts),
        cli::Command::Resimulate(opts) => resimulate::run(content_dir, &opts),
        cli::Command::Bisect(opts) => bisect::run(content_dir, &opts),
        cli::Command::Resume(opts) => resume::run(content_dir, &opts),
        cli::Command::Calibrate(opts) => calibrate::run(content_dir, &opts),
        cli::Command::Gate(opts) => gate::run(content_dir, &opts),
        cli::Command::Guard(opts) => guard::run(&opts),
        cli::Command::FastModel(opts) => fast_model::run(content_dir, &opts),
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            if let Some((kind, operation, seed)) = on_failure {
                emit_failure(kind, operation, seed, started, &err);
            }
            // `{err:#}` prints the chain once: each error's own message, joined by ": ".
            eprintln!("error: {err:#}");
            std::process::exit(1);
        }
    }
}

/// Prints the failure record of a `simulate` or `bench` run on stdout. It is not saved under
/// the data folder. Without an owner identity, or when stdout fails, it logs one warning and
/// prints nothing.
fn emit_failure(
    kind: &'static str,
    operation: &'static str,
    seed: u64,
    started: Instant,
    err: &anyhow::Error,
) {
    let (error_type, error_code, error_retriable) = err
        .chain()
        .find_map(|e| e.downcast_ref::<EngineError>())
        .map_or(("internal", "unclassified", false), |e| {
            (e.error_type(), e.error_code(), e.retriable())
        });
    let owner_id = match load_or_create_owner_id(&data_dir()) {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(signal = "record.failure_not_emitted", error = %e);
            return;
        }
    };
    let bench = kind == "run-report";
    let record = FailureRecord {
        kind,
        operation,
        owner_id,
        match_id: (!bench).then(|| MatchId::now(seed).to_string()),
        run_id: bench.then(|| format!("bench-{}", unix_millis())),
        machine_hash: bench.then(machine_hash),
        seed,
        content_hash: String::new(),
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        outcome: "error",
        error_type,
        error_code,
        error_retriable,
    };
    if let Err(e) = emit_line(&record) {
        tracing::warn!(signal = "record.failure_not_emitted", error = %e);
    }
}
