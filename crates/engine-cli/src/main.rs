//! `engine-cli`: simulate a match headless, benchmark the engine, calibrate it over many
//! matches, or generate teams.

mod bench;
mod calibrate;
mod cli;
mod content;
mod generate;
mod launch;
mod record;
mod replay;
mod report;
mod resume;
mod serve;
mod simulate;
mod stream_run;
mod web;

use std::io::IsTerminal;

use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() {
    let args = cli::Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("SM_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .init();
    let content_dir = args.content_dir.as_deref();
    let result = match args.command {
        cli::Command::Simulate(opts) => simulate::run(content_dir, &opts),
        cli::Command::Bench(opts) => bench::run(content_dir, &opts),
        cli::Command::Generate(opts) => generate::run(content_dir, &opts),
        cli::Command::Serve(opts) => serve::run(content_dir, &opts),
        cli::Command::Launch(opts) => launch::run(content_dir, &opts),
        cli::Command::Record(opts) => record::run(content_dir, &opts),
        cli::Command::Replay(opts) => replay::run(&opts),
        cli::Command::Resume(opts) => resume::run(content_dir, &opts),
        cli::Command::Calibrate(opts) => calibrate::run(content_dir, &opts),
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            // `{err:#}` prints the chain once: each error's own message, joined by ": ".
            eprintln!("error: {err:#}");
            std::process::exit(1);
        }
    }
}
