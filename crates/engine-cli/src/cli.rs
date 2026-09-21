//! Command-line definition (clap derive).

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Headless football match engine.
#[derive(Debug, Parser)]
#[command(name = "engine-cli", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Simulate one match and write every tick to a file.
    Simulate(SimulateOpts),
    /// Time whole matches on one thread and print a run report.
    Bench(BenchOpts),
}

#[derive(Debug, Args)]
pub struct SimulateOpts {
    /// Seed for the engine's random-number generator.
    #[arg(long)]
    pub seed: u64,
    /// Path of the tick file to write.
    #[arg(long, value_name = "FILE")]
    pub ticks_out: PathBuf,
    /// Minutes of play to simulate.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Also write a JSON Lines dump of every tick next to the tick file (`<FILE>.jsonl`).
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct BenchOpts {
    /// Seed for the engine's random-number generator.
    #[arg(long)]
    pub seed: u64,
    /// Number of timed matches after one warm-up match.
    #[arg(long, default_value_t = 5)]
    pub matches: u32,
    /// Minutes of play per match.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Print the run report as one JSON line (the default output is the same record).
    #[arg(long)]
    pub json: bool,
}
