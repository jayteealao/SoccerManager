//! Command-line definition (clap derive). Help lines stay under 80 columns.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use engine::FlagSetting;

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
    /// Serve the viewer page and run the engine; restart it after a crash.
    Launch(LaunchOpts),
    /// Record one whole match stream to a fixture file.
    Record(RecordOpts),
    /// Replay a recorded fixture over the same socket protocol.
    Replay(ReplayOpts),
    /// Continue a match from its latest snapshot to full time.
    Resume(ResumeOpts),
    /// Play many AI-managed matches and check the realism bands.
    Calibrate(CalibrateOpts),
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
    /// Play extra time and a shoot-out when level after regulation time.
    #[arg(long)]
    pub knockout: bool,
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
    /// Script pack folder (pack.json and a .rhai script) to run.
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Script pack folder (pack.json and a .rhai script) to run.

                     The script can change decisions, cards,
                     and commentary in a sandbox. See
                     content/scripts/README.md."
    )]
    pub script_pack: Option<PathBuf>,
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
    /// Script pack folder (pack.json and a .rhai script) to run.
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Script pack folder (pack.json and a .rhai script) to run.

                     The script can change decisions, cards,
                     and commentary in a sandbox. See
                     content/scripts/README.md."
    )]
    pub script_pack: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ServeOpts {
    /// Seed for the engine's random-number generator.
    #[arg(long, required_unless_present = "resume")]
    pub seed: Option<u64>,
    /// Minutes of play to simulate.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Play extra time and a shoot-out when level after regulation time.
    #[arg(long)]
    pub knockout: bool,
    /// Also write every tick to this file while streaming.
    #[arg(long, value_name = "FILE")]
    pub ticks_out: Option<PathBuf>,
    /// Home team file; default teams/default-a.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_a: Option<PathBuf>,
    /// Away team file; default teams/default-b.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_b: Option<PathBuf>,
    /// Script pack folder (pack.json and a .rhai script) to run.
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Script pack folder (pack.json and a .rhai script) to run.

                     The script can change decisions, cards,
                     and commentary in a sandbox. See
                     content/scripts/README.md."
    )]
    pub script_pack: Option<PathBuf>,
    /// Also serve this folder as the viewer page.
    #[arg(long, value_name = "DIR")]
    pub web: Option<PathBuf>,
    /// Continue the match in this snapshot file instead of starting one.
    #[arg(long, value_name = "FILE", conflicts_with = "seed")]
    pub resume: Option<PathBuf>,
    /// Seconds to wait for a viewer that lost its connection; 0 ends the run.
    #[arg(
        long,
        value_name = "SECONDS",
        default_value_t = 0,
        long_help = "Seconds to wait for a viewer that lost its connection; 0 ends the run.\n\n\
                     A viewer that closes the page on purpose always ends the run.\n\
                     A connection lost without a close resumes from the newest\n\
                     stoppage the viewer received."
    )]
    pub reconnect_wait: u64,
    /// The match stamp in milliseconds; set by the launcher.
    #[arg(long, hide = true)]
    pub match_millis: Option<u64>,
    /// Drop the viewer's connection once this tick is sent; a test seam.
    #[arg(long, hide = true, value_name = "TICK")]
    pub drop_client_at: Option<u32>,
}

#[derive(Debug, Args)]
pub struct LaunchOpts {
    /// Seed for the engine's random-number generator.
    #[arg(long)]
    pub seed: u64,
    /// Minutes of play to simulate.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Home team file; default teams/default-a.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_a: Option<PathBuf>,
    /// Away team file; default teams/default-b.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_b: Option<PathBuf>,
    /// Folder holding the viewer page.
    #[arg(long, value_name = "DIR")]
    pub web: PathBuf,
    /// Engine program to run; default SM_ENGINE_PATH, then this program.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Engine program to run; default SM_ENGINE_PATH, then this program.\n\n\
                     When the file is missing, the page shows the path it looked\n\
                     for and how to build the engine."
    )]
    pub engine: Option<PathBuf>,
    /// Drop the viewer's connection once this tick is sent; a test seam.
    #[arg(long, hide = true, value_name = "TICK")]
    pub drop_client_at: Option<u32>,
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
    /// Play extra time and a shoot-out when level after regulation time.
    #[arg(long)]
    pub knockout: bool,
    /// Home team file; default teams/default-a.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_a: Option<PathBuf>,
    /// Away team file; default teams/default-b.json in the content folder.
    #[arg(long, value_name = "FILE")]
    pub team_b: Option<PathBuf>,
    /// Script pack folder (pack.json and a .rhai script) to run.
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Script pack folder (pack.json and a .rhai script) to run.

                     The script can change decisions, cards,
                     and commentary in a sandbox. See
                     content/scripts/README.md."
    )]
    pub script_pack: Option<PathBuf>,
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
    /// Play extra time and a shoot-out when level after regulation time.
    #[arg(long)]
    pub knockout: bool,
    /// Print the run report as one JSON line (the default output is the same).
    #[arg(long)]
    pub json: bool,
    /// Also stream one match to a client that reads as fast as it can.
    #[arg(long)]
    pub stream: bool,
    /// Script pack folder (pack.json and a .rhai script) to run.
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Script pack folder (pack.json and a .rhai script) to run.

                     The script can change decisions, cards,
                     and commentary in a sandbox. See
                     content/scripts/README.md."
    )]
    pub script_pack: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct CalibrateOpts {
    /// Seed of the run: the leagues, the fixtures, and every match seed.
    #[arg(long)]
    pub seed: u64,
    /// Matches in each suite.
    #[arg(long, default_value_t = 1000)]
    pub matches: u32,
    /// Minutes of play per match.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Worker processes; default the number of logical cores.
    #[arg(long)]
    pub jobs: Option<u32>,
    /// Suites to run: equal strength, a stronger club, or both.
    #[arg(long, value_enum, default_value = "all")]
    pub suite: SuiteArg,
    /// Run folder; default SM_DATA_DIR/runs/<run.id>.
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,
    /// Event files to keep at the end of the run.
    #[arg(
        long,
        value_enum,
        default_value = "outliers",
        long_help = "Event files to keep at the end of the run.\n\n\
                     outliers keeps the files of failed, slow, and out-of-band\n\
                     matches and of dark-path hits; all keeps every file."
    )]
    pub keep_events: KeepEvents,
    /// Set a feature flag of the tuning file for this run; repeatable.
    #[arg(
        long = "flag",
        value_name = "NAME=on|off",
        long_help = "Set a feature flag of the tuning file for this run; repeatable.\n\n\
                     The flag must be declared in the flags block of tuning.json.\n\
                     A flag not set here keeps the state the file gives it."
    )]
    pub flags: Vec<FlagSetting>,
    /// Play every fixture with the flag off, then on; compare the bands.
    #[arg(
        long,
        value_name = "NAME",
        long_help = "Play every fixture with the flag off, then on; compare the bands.\n\n\
                     Both arms play the same fixtures on the same match seeds,\n\
                     into arms/off and arms/on in the run folder. The report\n\
                     holds both arms, a row per band, and a verdict."
    )]
    pub pair: Option<String>,
    /// Run as a worker of a calibration run (set by the parent process).
    #[arg(long, hide = true)]
    pub worker: bool,
    #[arg(long, hide = true, default_value_t = 0)]
    pub shard: u32,
    #[arg(long, hide = true, default_value_t = 1)]
    pub shards: u32,
    #[arg(long, hide = true, value_name = "DIR")]
    pub run_dir: Option<PathBuf>,
    #[arg(long, hide = true, default_value_t = 0)]
    pub run_millis: u64,
}

/// The suites a calibration run plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum SuiteArg {
    All,
    Equal,
    Strength,
}

/// Which event files a calibration run keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum KeepEvents {
    Outliers,
    All,
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
