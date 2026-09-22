//! Command-line definition (clap derive). Help lines stay under 80 columns.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Headless football match engine.
#[derive(Debug, Parser)]
#[command(name = "engine-cli", version, about)]
pub struct Cli {
    /// Folder holding the content files (attributes, tuning, rules, teams).
    #[arg(
        long,
        global = true,
        value_name = "DIR",
        long_help = "Folder holding the content files (attributes, tuning, rules, teams).\n\n\
                     When absent: SM_CONTENT_DIR, then ./content, then the content\n\
                     folder beside the binary."
    )]
    pub content_dir: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Simulate one match and write every tick to a file.
    Simulate(SimulateOpts),
    /// Time whole matches on one thread and print a run report.
    Bench(BenchOpts),
    /// Generate fictional clubs as team files, one file per club.
    Generate(GenerateOpts),
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
    /// Also write a JSON Lines dump beside the tick file.
    #[arg(
        long,
        long_help = "Also write a JSON Lines dump beside the tick file.\n\n\
                     The dump path is the tick file path with its extension\n\
                     replaced by .jsonl (match.ticks becomes match.jsonl)."
    )]
    pub json: bool,
    /// Home team file; default teams/default-a.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_a: Option<PathBuf>,
    /// Away team file; default teams/default-b.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_b: Option<PathBuf>,
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
    /// Print the run report as one JSON line (the default output is the same).
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct GenerateOpts {
    /// Seed for the generator; the same seed gives the same clubs.
    #[arg(long)]
    pub seed: u64,
    /// Number of clubs to generate.
    #[arg(long, default_value_t = 20)]
    pub clubs: u32,
    /// Folder to write the team files into; created when absent.
    #[arg(long, value_name = "DIR")]
    pub out: PathBuf,
    /// Overwrite team files that already exist.
    #[arg(long)]
    pub force: bool,
}
