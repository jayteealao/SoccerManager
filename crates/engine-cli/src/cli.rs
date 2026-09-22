//! Command-line definition (clap derive). Help lines stay under 80 columns.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Headless command line for the football match engine.
#[derive(Debug, Parser)]
#[command(
    name = "engine-cli",
    version,
    about = "Headless command line for the football match engine."
)]
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
    /// Stream one match live over the local socket to one viewer.
    Serve(ServeOpts),
    /// Record one whole match stream to a fixture file.
    Record(RecordOpts),
    /// Replay a recorded fixture over the same socket protocol.
    Replay(ReplayOpts),
    /// Continue a match from its latest snapshot to full time.
    Resume(ResumeOpts),
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
    /// Do not write a snapshot at each stoppage.
    #[arg(
        long,
        long_help = "Do not write a snapshot at each stoppage.\n\n\
                     By default the latest snapshot is written to\n\
                     SM_DATA_DIR/matches/<match.id>/snapshot.smsn."
    )]
    pub no_snapshot: bool,
}

#[derive(Debug, Args)]
pub struct ResumeOpts {
    /// Snapshot file written during a match (snapshot.smsn).
    #[arg(long, value_name = "FILE")]
    pub snapshot: PathBuf,
    /// Also write the resumed ticks to this file.
    #[arg(long, value_name = "FILE")]
    pub ticks_out: Option<PathBuf>,
    /// Also write a JSON Lines dump beside the tick file.
    #[arg(long, requires = "ticks_out")]
    pub json: bool,
    /// Home team file the match was started with; default as for simulate.
    #[arg(long, value_name = "FILE")]
    pub team_a: Option<PathBuf>,
    /// Away team file the match was started with; default as for simulate.
    #[arg(long, value_name = "FILE")]
    pub team_b: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ServeOpts {
    /// Seed for the engine's random-number generator.
    #[arg(long)]
    pub seed: u64,
    /// Minutes of play to simulate.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Also write every tick to this file while streaming.
    #[arg(long, value_name = "FILE")]
    pub ticks_out: Option<PathBuf>,
    /// Home team file; default teams/default-a.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_a: Option<PathBuf>,
    /// Away team file; default teams/default-b.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_b: Option<PathBuf>,
    /// Also serve this folder as the viewer page.
    #[arg(long, value_name = "DIR")]
    pub web: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct RecordOpts {
    /// Seed for the engine's random-number generator.
    #[arg(long)]
    pub seed: u64,
    /// Path of the fixture file to write.
    #[arg(long, value_name = "FILE")]
    pub out: PathBuf,
    /// Minutes of play to record.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Home team file; default teams/default-a.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_a: Option<PathBuf>,
    /// Away team file; default teams/default-b.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_b: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ReplayOpts {
    /// Fixture file written by the record command.
    #[arg(long, value_name = "FILE")]
    pub fixture: PathBuf,
    /// Playback speed; 1.0 is real time and 8.0 is eight times faster.
    #[arg(long, default_value_t = 1.0)]
    pub speed: f32,
    /// Cap the delivered rate; tests the lag notice.
    #[arg(long, value_name = "SPEED")]
    pub sustain: Option<f32>,
    /// Also serve this folder as the viewer page.
    #[arg(long, value_name = "DIR")]
    pub web: Option<PathBuf>,
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
    /// Also stream one match to a client that reads as fast as it can.
    #[arg(long)]
    pub stream: bool,
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
