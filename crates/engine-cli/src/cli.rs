//! Command-line definition (clap derive). Help lines stay under 80 columns.

use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand};
use engine::FlagSetting;

/// Headless command line for the football match engine.
#[derive(Debug, Parser)]
#[command(
    name = "engine-cli",
    version,
    about = "Headless command line for the football match engine."
)]
pub struct Cli {
    /// Folder holding the content files.
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
    /// Play a recorded match again from its replay file alone.
    Resimulate(ResimulateOpts),
    /// Find the first tick where two engine versions differ on a replay.
    Bisect(BisectOpts),
    /// Continue a match from its latest snapshot to full time.
    Resume(ResumeOpts),
    /// Play many AI-managed matches and check the realism bands.
    Calibrate(CalibrateOpts),
    /// Replay the 22 gate matches and compare them with the golden file.
    Gate(GateOpts),
    /// Check each commit's golden-file change against the ledger rules.
    Guard(GuardOpts),
    /// Fit the fast model, check it, or refuse a stale fit.
    FastModel(FastModelOpts),
    /// Time a whole matchday on a limited number of cores; a test seam.
    #[command(hide = true)]
    MatchdayTiming(TimingOpts),
}

/// The hidden timing run of a matchday.
#[derive(Debug, Args)]
pub struct TimingOpts {
    /// Seed of the player's match; the round derives from it.
    #[arg(long, default_value_t = 42)]
    pub seed: u64,
    /// Minutes of play of every match.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Limit the process to its first N logical cores before anything starts.
    #[arg(long, value_name = "N")]
    pub cores: Option<usize>,
    /// Background worker threads; default usable cores minus one.
    #[arg(long, value_name = "N")]
    pub threads: Option<usize>,
    /// Playback speed of the reveal schedule; 8 is the fastest the viewer offers.
    #[arg(long, default_value_t = 8.0)]
    pub speed: f64,
    /// Where the timing record goes.
    #[arg(long, value_name = "FILE")]
    pub out: PathBuf,
}

#[derive(Debug, Args)]
pub struct FastModelOpts {
    /// What to do.
    #[arg(
        value_enum,
        long_help = "What to do.\n\
                     fit: confirm the engine reproduces the golden results, play\n\
                     the fit batch and the check batch on the full engine, fit the\n\
                     model, check it, and write the fit file when every figure is\n\
                     within its tolerance.\n\
                     check: play the check batch again for the fit file and compare.\n\
                     stale: refuse a fit whose engine id is not the golden results'."
    )]
    pub action: FastModelAction,
    /// Fit file to read; default in the content folder.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "The fit file check and stale read. Default: fast-model.json\n\
                     in the content folder."
    )]
    pub fit: Option<PathBuf>,
    /// Where fit writes the fit file.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Where fit writes the fit file when the check passes.\n\
                     Default: fast-model.json in the content folder."
    )]
    pub out: Option<PathBuf>,
    /// Golden file the engine id comes from.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "The golden file whose results the engine id names; default\n\
                     gate/golden.json in the current folder (the repository root)."
    )]
    pub golden: Option<PathBuf>,
    /// Full matches per pairing (fit only).
    #[arg(
        long,
        default_value_t = 1000,
        long_help = "Full-engine matches per strength pairing in each batch (fit\n\
                     only; check reads the count from the fit file). Nine pairings."
    )]
    pub matches: u32,
    /// Fast-model draws per check match.
    #[arg(
        long,
        default_value_t = 20,
        long_help = "Fast-model matches played for each check-batch match (fit\n\
                     only; check reads the count from the fit file)."
    )]
    pub draws: u32,
    /// Minutes per full match (fit only).
    #[arg(
        long,
        default_value_t = 90,
        long_help = "Minutes of play per full-engine match (fit only). A fit for\n\
                     the game uses 90; a shorter match is for tests."
    )]
    pub minutes: u32,
    /// Threads that play full matches.
    #[arg(
        long,
        long_help = "Threads that play the full-engine matches; default the\n\
                     number of logical cores."
    )]
    pub jobs: Option<u32>,
    /// Confirm only this gate fixture; repeatable.
    #[arg(
        long = "gate-fixture",
        value_name = "ID",
        long_help = "Before fit and check play, the engine replays the gate\n\
                     fixtures and stops when a hash differs from the golden file.\n\
                     Name a fixture to replay only it; repeatable. Default: all 22."
    )]
    pub gate_fixtures: Vec<String>,
    /// Folder for report.json.
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Write report.json with every figure, its tolerance and the\n\
                     fit into this folder (fit and check)."
    )]
    pub report: Option<PathBuf>,
}

/// What `fast-model` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FastModelAction {
    Fit,
    Check,
    Stale,
}

#[derive(Debug, Args)]
pub struct GateOpts {
    /// Golden file to compare with; default gate/golden.json.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Golden file to compare with; default gate/golden.json in the\n\
                     current folder (the repository root)."
    )]
    pub golden: Option<PathBuf>,
    /// Play only this fixture; repeat for more.
    #[arg(
        long,
        value_name = "ID",
        long_help = "Play only this fixture, such as seed-42, change, or knockout.\n\
                     Repeat the flag for more. Default: all 22 fixtures."
    )]
    pub fixture: Vec<String>,
    /// Print one JSON object per match instead of text.
    #[arg(long)]
    pub json: bool,
    /// Write the first golden file; refused when one exists.
    #[arg(
        long,
        long_help = "Play every fixture and write the first golden file with the\n\
                     portable hash set. Refused when the file already exists."
    )]
    pub bootstrap: bool,
    /// Rewrite the hashes with this build's; needs --reason.
    #[arg(
        long,
        long_help = "Play every fixture and rewrite the golden file with this build's\n\
                     hashes as the portable hash set. Appends one regenerate entry\n\
                     to the ledger and drops every other hash set. Needs --reason."
    )]
    pub regenerate: bool,
    /// Why the file is written; recorded in the ledger.
    #[arg(
        long,
        value_name = "TEXT",
        long_help = "Why the golden file is written. Recorded in the new ledger entry.\n\
                     Needed by --regenerate; optional for --bootstrap."
    )]
    pub reason: Option<String>,
    /// Play every match with debug mode on; compare only.
    #[arg(
        long,
        long_help = "Play every match with debug mode on. The compare and the report\n\
                     are unchanged; after each match, standard error gets the draws\n\
                     the trace recorded, the registry's draw count, and the decisions\n\
                     and rule outcomes recorded. Unequal draw counts fail the gate\n\
                     (exit 2). The records are not written. Refused with --bootstrap\n\
                     and --regenerate."
    )]
    pub debug: bool,
}

#[derive(Debug, Args)]
pub struct GuardOpts {
    /// The base revision of the range, such as main.
    #[arg(
        long,
        value_name = "REV",
        long_help = "The base revision: the commits after it that change the golden\n\
                     file are checked, such as main or the pull request's base."
    )]
    pub base: String,
    /// The last revision of the range.
    #[arg(long, value_name = "REV", default_value = "HEAD")]
    pub head: String,
    /// Golden file; default gate/golden.json.
    #[arg(
        long,
        value_name = "PATH",
        default_value = "gate/golden.json",
        hide_default_value = true,
        long_help = "The golden file's path from the repository root, with forward\n\
                     slashes."
    )]
    pub golden: String,
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
    /// Play extra time and a shoot-out if level.
    #[arg(
        long,
        long_help = "Play extra time and a shoot-out when level after regulation time."
    )]
    pub knockout: bool,
    /// Also write a JSON Lines dump beside the tick file.
    #[arg(
        long,
        long_help = "Also write a JSON Lines dump beside the tick file.\n\n\
                     The dump path is the tick file path with its extension\n\
                     replaced by .jsonl (match.ticks becomes match.jsonl)."
    )]
    pub json: bool,
    /// Home team file; default teams/default-a.json.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Home team file; default teams/default-a.json in the content folder."
    )]
    pub team_a: Option<PathBuf>,
    /// Away team file; default teams/default-b.json.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Away team file; default teams/default-b.json in the content folder."
    )]
    pub team_b: Option<PathBuf>,
    /// Do not write a snapshot at each stoppage.
    #[arg(
        long,
        long_help = "Do not write a snapshot at each stoppage.\n\n\
                     By default the latest snapshot is written to\n\
                     SM_DATA_DIR/matches/<match.id>/snapshot.smsn."
    )]
    pub no_snapshot: bool,
    /// Script pack folder to run.
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Script pack folder (pack.json and a .rhai script) to run.

                     The script can change decisions, cards,
                     and commentary in a sandbox. See
                     content/scripts/README.md."
    )]
    pub script_pack: Option<PathBuf>,
    /// Write the match's debug trace to this file.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Play the match with debug mode on and write its debug trace to\n\
                     this file as JSON Lines: a header line, then every random draw,\n\
                     decision point, and rule outcome in the order the engine ran\n\
                     them. A 90-minute match gives about 160 MB. See\n\
                     docs/reference/cli.md."
    )]
    pub debug_trace: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ResumeOpts {
    /// Snapshot file of a match (snapshot.smsn).
    #[arg(long, value_name = "FILE")]
    pub snapshot: PathBuf,
    /// The previous release's engine program.
    #[arg(
        long = "previous",
        value_name = "FILE",
        long_help = "Previous release's engine program, which finishes the matches that\n\
                     release saved; default SM_PREVIOUS_ENGINE_PATH, then\n\
                     previous/engine-cli beside this program. It plays with the\n\
                     content folder beside it."
    )]
    pub previous_engine: Option<PathBuf>,
    /// Also write the resumed ticks to this file.
    #[arg(long, value_name = "FILE")]
    pub ticks_out: Option<PathBuf>,
    /// Also write a JSON Lines dump beside the tick file.
    #[arg(
        long,
        requires = "ticks_out",
        long_help = "Also write a JSON Lines dump beside the tick file.\n\n\
                     The dump path is the tick file path with its extension\n\
                     replaced by .jsonl (match.ticks becomes match.jsonl)."
    )]
    pub json: bool,
    /// Home team file the match was started with.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Home team file the match was started with; default as for simulate."
    )]
    pub team_a: Option<PathBuf>,
    /// Away team file the match was started with.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Away team file the match was started with; default as for simulate."
    )]
    pub team_b: Option<PathBuf>,
    /// Script pack folder to run.
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
    /// Play extra time and a shoot-out if level.
    #[arg(
        long,
        long_help = "Play extra time and a shoot-out when level after regulation time."
    )]
    pub knockout: bool,
    /// Also write every tick to this file.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Also write every tick to this file while streaming."
    )]
    pub ticks_out: Option<PathBuf>,
    /// Home team file; default teams/default-a.json.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Home team file; default teams/default-a.json in the content folder."
    )]
    pub team_a: Option<PathBuf>,
    /// Away team file; default teams/default-b.json.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Away team file; default teams/default-b.json in the content folder."
    )]
    pub team_b: Option<PathBuf>,
    /// Script pack folder to run.
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
    /// Continue the match in this snapshot file.
    #[arg(
        long,
        value_name = "FILE",
        conflicts_with = "seed",
        long_help = "Continue the match in this snapshot file instead of starting one."
    )]
    pub resume: Option<PathBuf>,
    /// Seconds to wait for a lost viewer.
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
    /// Produce every tick up to this one without waiting for the viewer; a test seam.
    #[arg(long, hide = true, value_name = "TICK")]
    pub fast_forward_to: Option<u32>,
    /// Accept the jump command, which fast-forwards a started match; a test seam.
    #[arg(long, hide = true)]
    pub test_jump: bool,
    /// Play no other match of the matchday; a test seam.
    #[arg(long, hide = true)]
    pub no_matchday: bool,
    /// Background worker threads of the matchday; a test seam.
    #[arg(long, hide = true, value_name = "N")]
    pub matchday_threads: Option<usize>,
    /// Make one background match fail at a tick (FIXTURE@TICK); a test seam.
    #[arg(long, hide = true, value_name = "FIXTURE@TICK")]
    pub matchday_fault: Option<crate::matchday::Fault>,
}

#[derive(Debug, Args)]
pub struct LaunchOpts {
    /// Random-number seed; default from the clock.
    #[arg(
        long,
        long_help = "Seed for the engine's random-number generator; default from the clock.\n\n\
                     Every match started from match setup uses it.\n\
                     Without it, each match takes a seed from the clock."
    )]
    pub seed: Option<u64>,
    /// Minutes of play to simulate.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Home team file; default teams/default-a.json.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Home team file; default teams/default-a.json in the content folder."
    )]
    pub team_a: Option<PathBuf>,
    /// Away team file; default teams/default-b.json.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Away team file; default teams/default-b.json in the content folder."
    )]
    pub team_b: Option<PathBuf>,
    /// Folder holding the viewer page.
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Folder holding the viewer page; default SM_WEB_DIR, viewer/dist, web/.\n\n\
                     Without the flag, SM_WEB_DIR is used alone when it is set.\n\
                     Otherwise ./viewer/dist (the built viewer in a checkout),\n\
                     then the web folder beside this program (an installed game).\n\
                     A folder counts only when it holds index.html."
    )]
    pub web: Option<PathBuf>,
    /// Open the page in the default browser.
    #[arg(long)]
    pub open: bool,
    /// Engine program to run.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Engine program to run; default SM_ENGINE_PATH, then this program.\n\n\
                     When the file is missing, the page shows the path it looked\n\
                     for and how to build the engine."
    )]
    pub engine: Option<PathBuf>,
    /// Continue the saved match in this snapshot file.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Continue the saved match in this snapshot file instead of starting\n\
                     one. The engine that wrote the save finishes it: this program, or\n\
                     the previous release's program. A save from any other version is\n\
                     refused, and the page shows its version."
    )]
    pub resume: Option<PathBuf>,
    /// The previous release's engine program.
    #[arg(
        long = "previous",
        value_name = "FILE",
        long_help = "Previous release's engine program, which finishes the matches that\n\
                     release saved; default SM_PREVIOUS_ENGINE_PATH, then\n\
                     previous/engine-cli beside this program."
    )]
    pub previous: Option<PathBuf>,
    /// Start a match at once; no start screen.
    #[arg(
        long,
        conflicts_with = "resume",
        long_help = "Start a match at once with the seed and the team\n\
                     files given, instead of opening the start screen."
    )]
    pub no_start_screen: bool,
    /// Drop the viewer's connection once this tick is sent; a test seam.
    #[arg(long, hide = true, value_name = "TICK")]
    pub drop_client_at: Option<u32>,
    /// Make every match's worker produce ticks up to this one at once; a test seam.
    #[arg(long, hide = true, value_name = "TICK")]
    pub fast_forward_to: Option<u32>,
    /// Pass --test-jump to every match's engine; a test seam.
    #[arg(long, hide = true)]
    pub test_jump: bool,
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
    /// Play extra time and a shoot-out if level.
    #[arg(
        long,
        long_help = "Play extra time and a shoot-out when level after regulation time."
    )]
    pub knockout: bool,
    /// Home team file; default teams/default-a.json.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Home team file; default teams/default-a.json in the content folder."
    )]
    pub team_a: Option<PathBuf>,
    /// Away team file; default teams/default-b.json.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Away team file; default teams/default-b.json in the content folder."
    )]
    pub team_b: Option<PathBuf>,
    /// Script pack folder to run.
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Script pack folder (pack.json and a .rhai script) to run.

                     The script can change decisions, cards,
                     and commentary in a sandbox. See
                     content/scripts/README.md."
    )]
    pub script_pack: Option<PathBuf>,
    /// JSON file of manager changes to queue.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "JSON file of manager changes to queue, each before the\n\
                     step that starts on its tick. Each team the file names\n\
                     is managed by hand. See docs/reference/cli.md."
    )]
    pub changes: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ResimulateOpts {
    /// Replay file written by the record command.
    #[arg(long, value_name = "FILE")]
    pub fixture: PathBuf,
    /// Run on another engine and report both.
    #[arg(
        long,
        long_help = "Run even when this binary did not record the file, and\n\
                     report both engine identities and both stream schemes.\n\
                     The file is never written."
    )]
    pub compare: bool,
    /// Write the SHA-256 of the full match state after every tick.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Write the SHA-256 of the full match state after every tick\n\
                     to this file: a JSON header line, one '<tick> <sha256>'\n\
                     line per tick, a 'finish' line, and a closing 'end' line.\n\
                     See docs/reference/cli.md."
    )]
    pub state_digests: Option<PathBuf>,
    /// Write the named parts of the state of the tick --at-tick names.
    #[arg(
        long,
        value_name = "FILE",
        requires = "at_tick",
        long_help = "Write the named parts of the match state after the tick\n\
                     --at-tick names to this file as one JSON object, and stop\n\
                     the match after that tick."
    )]
    pub state_fields: Option<PathBuf>,
    /// The tick --state-fields writes; the match stops after it.
    #[arg(
        long,
        value_name = "TICK",
        requires = "state_fields",
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    pub at_tick: Option<u32>,
    /// Play with debug mode on and write the debug trace to this file.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Play the match with debug mode on and write its debug trace\n\
                     to this file as JSON Lines, as simulate --debug-trace does."
    )]
    pub debug_trace: Option<PathBuf>,
}

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("side_a").required(true).args(["a", "a_binary"])))]
#[command(group(ArgGroup::new("side_b").required(true).args(["b", "b_binary"])))]
#[command(
    override_usage = "engine-cli bisect [OPTIONS] --fixture <FILE>\n       \
                            <--a <REV>|--a-binary <PATH>> <--b <REV>|--b-binary <PATH>>"
)]
pub struct BisectOpts {
    /// Version-4 replay file to re-simulate on both builds.
    #[arg(long, value_name = "FILE")]
    pub fixture: PathBuf,
    /// Build A: a commit, built into the bisect build cache.
    #[arg(long, value_name = "REV")]
    pub a: Option<String>,
    /// Build A: a ready engine-cli executable.
    #[arg(long, value_name = "PATH")]
    pub a_binary: Option<PathBuf>,
    /// Build B: a commit, built into the bisect build cache.
    #[arg(long, value_name = "REV")]
    pub b: Option<String>,
    /// Build B: a ready engine-cli executable.
    #[arg(long, value_name = "PATH")]
    pub b_binary: Option<PathBuf>,
    /// Git repository to build commits from.
    #[arg(long, value_name = "DIR", default_value = ".")]
    pub repo: PathBuf,
    /// Build cache folder; default <repo>/target/bisect.
    #[arg(long, value_name = "DIR")]
    pub cache: Option<PathBuf>,
    /// Cargo profile for the builds.
    #[arg(long, value_name = "NAME", default_value = "release")]
    pub profile: String,
    /// Cargo features to build commits with.
    #[arg(
        long,
        value_name = "LIST",
        default_value = "",
        hide_default_value = true
    )]
    pub features: String,
    /// Seconds before a run is stopped.
    #[arg(long, value_name = "SECONDS", default_value_t = 1800)]
    pub timeout: u64,
    /// Print the report as one JSON object.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ReplayOpts {
    /// Fixture file written by the record command.
    #[arg(long, value_name = "FILE")]
    pub fixture: PathBuf,
    /// Playback speed; 1.0 is real time.
    #[arg(
        long,
        default_value_t = 1.0,
        long_help = "Playback speed; 1.0 is real time and 8.0 is eight times faster."
    )]
    pub speed: f32,
    /// Cap the delivered rate; tests the lag notice.
    #[arg(long, value_name = "SPEED")]
    pub sustain: Option<f32>,
    /// Also serve this folder as the viewer page.
    #[arg(long, value_name = "DIR")]
    pub web: Option<PathBuf>,
    /// Send every frame before this tick at once, then pace from it; a test seam.
    #[arg(long, hide = true, value_name = "TICK")]
    pub fast_forward_to: Option<u32>,
}

#[derive(Debug, Args)]
pub struct BenchOpts {
    /// Seed for the engine's random-number generator.
    #[arg(long)]
    pub seed: u64,
    /// Timed matches after one warm-up match.
    #[arg(
        long,
        default_value_t = 5,
        long_help = "Number of timed matches after one warm-up match."
    )]
    pub matches: u32,
    /// Minutes of play per match.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Play extra time and a shoot-out if level.
    #[arg(
        long,
        long_help = "Play extra time and a shoot-out when level after regulation time."
    )]
    pub knockout: bool,
    /// Print the run report as one JSON line.
    #[arg(
        long,
        long_help = "Print the run report as one JSON line.\n\n\
                     The default output is the same record."
    )]
    pub json: bool,
    /// Also stream one match to a fast client.
    #[arg(
        long,
        long_help = "Also stream one match to a client that reads as fast as it can."
    )]
    pub stream: bool,
    /// Script pack folder to run.
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
    /// Seed of the run and of every match.
    #[arg(
        long,
        long_help = "Seed of the run: the leagues, the fixtures, and every match seed."
    )]
    pub seed: u64,
    /// Matches per suite or pairing.
    #[arg(
        long,
        default_value_t = 1000,
        long_help = "Matches in each suite and in each formation pairing.\n\n\
                     In a change run (--base) it is the cap: the run plays a pilot,\n\
                     sizes each suite for the power to see each band's smallest\n\
                     shift, and plays at most this many."
    )]
    pub matches: u32,
    /// Minutes of play per match.
    #[arg(long, default_value_t = 90)]
    pub minutes: u32,
    /// Matches played at once; default one per core.
    #[arg(
        long,
        long_help = "Matches played at once, each on its own thread; default the number\n\
                     of logical cores."
    )]
    pub jobs: Option<u32>,
    /// Suites to run.
    #[arg(
        long,
        value_enum,
        default_value = "all",
        hide_possible_values = true,
        long_help = "Suites to run: equal strength, a stronger club, every formation\n\
                     pairing, all three, or the sending-off experiment.\n\n\
                     Values: all, equal, strength, formations, red-card.\n\
                     red-card is not part of all: it plays the default clubs\n\
                     with cards off and an away player sent off at kick-off."
    )]
    pub suite: SuiteArg,
    /// Play only this formation pairing; repeatable.
    #[arg(
        long = "pairing",
        value_name = "A v B",
        long_help = "Play only this formation pairing of the formations suite; repeatable.\n\n\
                     Name it as two formations joined by v, in either order,\n\
                     for example \"4-4-1-1 v 4-4-2\". Each pairing plays the\n\
                     same matches as in a full run."
    )]
    pub pairings: Vec<String>,
    /// Judge only this band; repeatable.
    #[arg(
        long = "band",
        value_name = "NAME",
        long_help = "Judge and show only this realism band; repeatable.\n\n\
                     The bands are those of the band registry, realism-bands.json\n\
                     in the content folder; an unknown name is refused with the\n\
                     list. The run plays only the suites that check the band."
    )]
    pub bands: Vec<String>,
    /// Compare with an earlier run report.
    #[arg(
        long,
        value_name = "FILE",
        long_help = "Compare the run with an earlier report.json, band by band.\n\n\
                     The baseline must have the same seed, match count, and\n\
                     fixtures hash, or the run stops before it plays. A change\n\
                     inside two sampling errors is marked as noise."
    )]
    pub baseline: Option<PathBuf>,
    /// Run folder; default SM_DATA_DIR/runs/<run.id>.
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Run folder; default SM_DATA_DIR/runs/<run.id>.\n\n\
                     Run again into the same folder to resume a stopped run or\n\
                     grow it with a larger --matches; a run with another\n\
                     identity moves the old files to superseded/."
    )]
    pub out: Option<PathBuf>,
    /// Event files to keep.
    #[arg(
        long,
        value_enum,
        default_value = "outliers",
        hide_possible_values = true,
        long_help = "Matches whose statistics and event files are written.\n\n\
                     Values: outliers, all. outliers (default) keeps the files of\n\
                     about 1 in 16 matches, chosen by fixture key, and of every\n\
                     failed, panicked or violating match, dark-path hit, and match\n\
                     with a band measure outside the 1st to 99th percentile of its\n\
                     suite so far; all keeps every match's files. Every match has\n\
                     its compact row either way."
    )]
    pub keep_events: KeepEvents,
    /// Set a tuning-file feature flag; repeatable.
    #[arg(
        long = "flag",
        value_name = "NAME=on|off",
        long_help = "Set a feature flag of the tuning file for this run; repeatable.\n\n\
                     The flag must be declared in the flags block of tuning.json.\n\
                     A flag not set here keeps the state the file gives it."
    )]
    pub flags: Vec<FlagSetting>,
    /// Play each fixture with the flag off, then on.
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
    /// Play in worker processes of this binary, the old path, instead of on threads.
    #[arg(long, hide = true, conflicts_with = "worker")]
    pub worker_processes: bool,
    #[arg(long, hide = true, default_value_t = 0)]
    pub shard: u32,
    #[arg(long, hide = true, default_value_t = 1)]
    pub shards: u32,
    #[arg(long, hide = true, value_name = "DIR")]
    pub run_dir: Option<PathBuf>,
    #[arg(long, hide = true, default_value_t = 0)]
    pub run_millis: u64,
    /// A worker of the formations suite plays only these pairings, by number.
    #[arg(long = "pairing-numbers", hide = true, value_delimiter = ',')]
    pub pairing_numbers: Vec<usize>,
    /// Make one match or one worker fail, to exercise the error records; a test seam.
    #[arg(long, hide = true, value_enum, value_name = "WHAT")]
    pub inject_failure: Option<InjectFailure>,
    /// The session of the parent run a worker belongs to (set by the parent process).
    #[arg(long, hide = true, default_value = "")]
    pub session: String,
    /// Stop after this many work units, leaving the next one half played; a test seam.
    #[arg(long, hide = true, value_name = "K")]
    pub stop_after_units: Option<u32>,
    /// Judge a change against the old engine of REV.
    #[arg(
        long,
        value_name = "REV",
        conflicts_with_all = ["pair", "base_binary"],
        long_help = "Make it a change run: compare this build with the old engine\n\
                     of REV (a branch, tag or commit of this repository), built once\n\
                     and cached with its results.\n\n\
                     Both engines play the same fixtures. A pilot sizes each suite\n\
                     for 80 percent power to see each band's smallest shift, the\n\
                     run grows to it (at most --matches), and each band reports\n\
                     pass, fail or not sure, with one joint verdict. Needs a git\n\
                     checkout of this repository as the working folder."
    )]
    pub base: Option<String>,
    /// A change run's pilot, in matches per suite unit; a test seam (default 200).
    #[arg(long, hide = true, value_name = "N", default_value_t = 200)]
    pub pilot: u32,
    /// The old engine of a change run: a ready executable, played on the run's content.
    #[arg(long, hide = true, value_name = "EXE", conflicts_with = "pair")]
    pub base_binary: Option<PathBuf>,
}

/// The failure `calibrate --inject-failure` makes: an error in the first match of the work
/// list, a panic in it, the first worker process before it plays (`--worker-processes`), or
/// the build of the old engine (`--base`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum InjectFailure {
    Match,
    Panic,
    Worker,
    BaseBuild,
}

impl InjectFailure {
    /// The value as the command line spells it.
    pub fn code(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::Panic => "panic",
            Self::Worker => "worker",
            Self::BaseBuild => "base-build",
        }
    }
}

/// The suites a calibration run plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum SuiteArg {
    All,
    Equal,
    Strength,
    Formations,
    RedCard,
}

/// Which event files a calibration run keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum KeepEvents {
    Outliers,
    All,
}

#[derive(Debug, Args)]
pub struct GenerateOpts {
    /// Seed for the generator.
    #[arg(
        long,
        long_help = "Seed for the generator; the same seed gives the same clubs."
    )]
    pub seed: u64,
    /// Number of clubs to generate.
    #[arg(long, default_value_t = 20)]
    pub clubs: u32,
    /// Folder for the team files; created when absent.
    #[arg(long, value_name = "DIR")]
    pub out: PathBuf,
    /// Overwrite team files that already exist.
    #[arg(long)]
    pub force: bool,
}
