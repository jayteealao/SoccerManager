//! `engine-cli`: simulate a match headless, or benchmark the engine.

mod bench;
mod cli;
mod simulate;

use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() {
    let args = cli::Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("SM_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
    let result = match args.command {
        cli::Command::Simulate(opts) => simulate::run(&opts),
        cli::Command::Bench(opts) => bench::run(&opts),
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("error: {err:#}");
            std::process::exit(1);
        }
    }
}
