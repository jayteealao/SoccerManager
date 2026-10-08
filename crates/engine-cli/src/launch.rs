//! `engine-cli launch`: serve the viewer page and run the engine as a separate worker.
//!
//! A browser page cannot start a program, and a page served by the engine loses its server,
//! its socket, and every way to ask for help when the engine stops. The launcher is the part
//! that survives: it serves the page, starts `engine-cli serve` as a child process, watches
//! it, and reports its state in `/engine.json`. After a crash the page asks for a restart
//! with a POST, and the launcher starts the worker again from the match's latest snapshot.
//! The page sends no match state: the snapshot on disk is the only copy the engine needs.
//!
//! By default the launcher opens on the start screen: no worker runs (`engine.state` is
//! `idle`) until the page asks for a match with two chosen clubs, or for the saved match.
//! The page can also stop a match and keep its snapshot, quit (the launcher ends itself),
//! and save the player's settings. `--no-start-screen` starts a match at once, as the
//! launcher did before the start screen existed.
//!
//! With `--resume <file>` the launcher continues a saved match instead of starting one. It
//! picks the program by the release version the save records ([`crate::engines`]): this
//! program, the previous release's program with its own content folder, or none, in which
//! case no worker starts and `/engine.json` carries a `resume` block that names the save's
//! version for the Resume a saved match screen.

use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use engine::observe::identity::{MatchId, data_dir};
use engine::{Content, ContentDir, Snapshot};
use stream::events::EVENTS_FILE;

use crate::cli::LaunchOpts;
use crate::engines::{Choice, PreviousEngine, Refusal};
use crate::front_door::{SampleTeam, SavedMatch, Settings, Views};
use crate::matchday::round::Round;
use crate::web::{Action, Status};

/// Seconds a worker waits for a page that lost its connection.
const RECONNECT_WAIT_S: &str = "120";
/// How often the watcher looks at the worker.
const WATCH_EVERY: Duration = Duration::from_millis(50);
/// How long the launcher keeps running after it answered a quit, so the answer reaches the
/// page before the process ends.
const QUIT_AFTER: Duration = Duration::from_millis(500);

/// Where the worker stands, as `/engine.json` names it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    /// The start screen: no match is running.
    Idle,
    /// Started, and its socket is not open yet.
    Starting,
    /// Serving the match on this socket port.
    Running { port: u16 },
    /// The match reached full time and the worker ended normally.
    Finished,
    /// The worker stopped with a failure while the match was running.
    Crashed { code: Option<i32> },
    /// The worker refused to start; the reason is the engine's own.
    Refused { reason: String },
    /// The manager gave the match up.
    Abandoned,
    /// No engine program at the configured path.
    NotFound,
    /// The player quit: the launcher is ending.
    Closed,
}

impl State {
    fn word(&self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Starting => "starting",
            State::Running { .. } => "running",
            State::Finished => "finished",
            State::Crashed { .. } => "crashed",
            State::Refused { .. } => "refused",
            State::Abandoned => "abandoned",
            State::NotFound => "not-found",
            State::Closed => "closed",
        }
    }
}

/// The mutable half, behind one lock.
struct Worker {
    state: State,
    child: Option<Child>,
    pid: Option<u32>,
    /// Bumped at every start, so a watcher of an older worker leaves the state alone.
    generation: u64,
    /// The newest snapshot tick read from disk.
    snapshot_tick: Option<u32>,
    /// The match the worker plays: its id and the snapshot file its worker writes.
    match_id: String,
    snapshot: PathBuf,
    /// The stamp of a fresh match.
    match_millis: u64,
    /// The program that plays the match.
    program: Program,
    /// The tick a saved match was resumed from, for the page.
    resumed_from: Option<u32>,
    /// Why a saved match cannot resume, for the Resume a saved match screen.
    refusal: Option<Refusal>,
    /// The seed of the next fresh match, and of the round match setup previews.
    seed: u64,
    /// The team-file flags every worker of this program receives.
    teams: Vec<OsString>,
    /// The unfinished match the start screen offers to resume.
    saved: Option<SavedMatch>,
    /// The player's settings.
    settings: Settings,
    /// The squad screen's store: named views and match ratings per club.
    views: Views,
}

/// The program that plays the match: this program, or the previous release's program with
/// its own content folder.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Program {
    Current,
    Previous {
        engine: PreviousEngine,
        version: String,
    },
}

impl Program {
    fn version(&self) -> &str {
        match self {
            Program::Current => engine::version(),
            Program::Previous { version, .. } => version,
        }
    }
}

/// What the start screen needs: the content the round preview and the saved match's pitch
/// read, and the sample teams match setup offers.
struct FrontDoor {
    dir: ContentDir,
    content: Content,
    teams: Vec<SampleTeam>,
}

/// The launcher: its configuration and the one worker it supervises.
pub struct Launcher {
    worker: Mutex<Worker>,
    engine: PathBuf,
    /// The previous release's program, for saves of that release.
    previous: PreviousEngine,
    /// The content folder this program's workers receive, when one was named.
    content_dir: Option<OsString>,
    /// The skin folder the viewer loads, from the `viewer.skin` slot.
    skin: &'static str,
    /// The seed every match takes when the launch names one.
    fixed_seed: Option<u64>,
    minutes: u32,
    /// The data folder: saves, settings.
    data: PathBuf,
    /// The start screen, unless the launch starts a match at once or resumes a save.
    front_door: Option<FrontDoor>,
    drop_client_at: Mutex<Option<u32>>,
    fast_forward_to: Option<u32>,
    test_jump: bool,
}

/// The snapshot file of a match in the data folder.
/// The rows of the events file `path` at or before `tick`, in file order: the events a match
/// resumed from `tick` played before its save, written by whichever engine played them. A
/// missing file, or a row that does not read, gives nothing for it.
fn earlier_events(path: &Path, tick: u32) -> Vec<serde_json::Value> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|row| {
            row.get("tick")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|t| t <= u64::from(tick))
        })
        .collect()
}

fn snapshot_of(match_id: &str) -> PathBuf {
    data_dir()
        .join("matches")
        .join(match_id)
        .join(engine::snapshot::FILE_NAME)
}

pub fn run(content_dir: Option<&Path>, opts: &LaunchOpts) -> anyhow::Result<i32> {
    let engine = engine_path(opts.engine.as_deref());
    let web = crate::web::resolve_web_dir(opts.web.as_deref())?;
    // The content folder is loaded once here, the way the worker loads it, so a bad slot file
    // (an unknown skin included) refuses the start before any page is served.
    let dir = ContentDir::resolve(content_dir)?;
    let content = Content::load(&dir)?;
    let skin = content.modules.skin.skin();
    let door = opts.resume.is_none() && !opts.no_start_screen;
    let seed = opts.seed.unwrap_or_else(clock_seed);
    if opts.seed.is_none() && !door {
        // Stdout carries the page address alone, so the chosen seed goes to stderr, where a
        // person can read it back to play the same match again.
        eprintln!("seed {seed}");
    }
    let started = MatchId::now(seed);
    let match_id = started.to_string();
    let mut teams = Vec::new();
    for (flag, file) in [("--team-a", &opts.team_a), ("--team-b", &opts.team_b)] {
        if let Some(file) = file {
            teams.push(OsString::from(flag));
            teams.push(file.as_os_str().to_os_string());
        }
    }
    let data = data_dir();
    let front_door = if door {
        let teams = crate::front_door::sample_teams(&dir, &content)?;
        Some(FrontDoor {
            dir,
            content,
            teams,
        })
    } else {
        None
    };
    let saved = front_door.as_ref().and_then(|door| {
        SavedMatch::newest(&data).and_then(|p| SavedMatch::read(&p, &door.teams, &door.content))
    });
    let launcher = Arc::new(Launcher {
        worker: Mutex::new(Worker {
            state: if door { State::Idle } else { State::Starting },
            child: None,
            pid: None,
            generation: 0,
            snapshot_tick: None,
            snapshot: snapshot_of(&match_id),
            match_id,
            match_millis: started.millis,
            program: Program::Current,
            resumed_from: None,
            refusal: None,
            seed,
            teams,
            saved,
            settings: Settings::load(&data),
            views: Views::load(&data),
        }),
        engine,
        previous: PreviousEngine::locate(opts.previous.as_deref()),
        content_dir: content_dir.map(|dir| dir.as_os_str().to_os_string()),
        skin,
        fixed_seed: opts.seed,
        minutes: opts.minutes,
        data,
        front_door,
        drop_client_at: Mutex::new(opts.drop_client_at),
        fast_forward_to: opts.fast_forward_to,
        test_jump: opts.test_jump,
    });

    let page = crate::web::start(&web, Arc::new(Arc::clone(&launcher)))?;
    if !launcher.engine.is_file() {
        // The page still loads: it is where the manager reads the path and what to do.
        launcher.lock().state = State::NotFound;
        tracing::warn!(
            signal = "launch.not_found",
            path = %shown_file(&launcher.engine)
        );
    } else if let Some(file) = opts.resume.as_deref() {
        launcher.resume_saved(file);
    } else if launcher.front_door.is_some() {
        let worker = launcher.lock();
        tracing::info!(
            signal = "launch.idle",
            teams = launcher.front_door.as_ref().map_or(0, |d| d.teams.len()),
            saved = worker.saved.is_some()
        );
    } else {
        launcher.start(None);
    }
    // The page address is the one line on stdout, because it is the line a person copies.
    println!("{}", page.address());
    if opts.open {
        open_browser(&page.address());
    }
    loop {
        std::thread::park();
    }
}

/// A seed for a player who gave none: the clock's nanoseconds, folded into 64 bits.
fn clock_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    (nanos as u64) ^ ((nanos >> 64) as u64)
}

/// Opens `address` in the default browser. The address is already printed, so a browser
/// that does not open is logged and the launcher keeps running.
fn open_browser(address: &str) {
    // The address is `http://127.0.0.1:<port>/`, which holds no character a shell reads.
    #[cfg(windows)]
    let mut command = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", address]);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(address);
        c
    };
    #[cfg(not(any(windows, target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(address);
        c
    };
    let result = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let reason = match result {
        Ok(status) if status.success() => return,
        Ok(status) => format!(
            "the browser opener exited with code {}",
            status
                .code()
                .map_or_else(|| "none".to_string(), |c| c.to_string())
        ),
        Err(err) => format!("cannot run the browser opener: {err}"),
    };
    tracing::warn!(signal = "launch.open_failed", reason = %reason);
}

/// `--engine`, then `SM_ENGINE_PATH`, then this program.
fn engine_path(flag: Option<&Path>) -> PathBuf {
    if let Some(path) = flag {
        return path.to_path_buf();
    }
    if let Some(path) = std::env::var_os("SM_ENGINE_PATH").filter(|p| !p.is_empty()) {
        return PathBuf::from(path);
    }
    // nosemgrep: rust.lang.security.current-exe.current-exe -- only used to start the engine that sits next to this program
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("engine-cli"))
}

/// The file name alone, for a signal: a signal never carries an absolute path.
fn shown_file(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The two clubs a new-match body names: `{"home": <club id>, "away": <club id>}`.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair {
    home: String,
    away: String,
}

impl Launcher {
    fn lock(&self) -> std::sync::MutexGuard<'_, Worker> {
        self.worker
            .lock()
            .expect("the worker lock is never poisoned")
    }

    /// `true` while a worker plays a match.
    fn busy(worker: &Worker) -> bool {
        matches!(worker.state, State::Running { .. } | State::Starting) && worker.child.is_some()
    }

    /// The two sample teams `home` and `away` name, home first, or the reason the pair is
    /// refused.
    fn pair(&self, home: &str, away: &str) -> Result<[&SampleTeam; 2], String> {
        let door = self
            .front_door
            .as_ref()
            .ok_or("this launch started its match at once; it has no match setup")?;
        let find = |id: &str| {
            door.teams
                .iter()
                .find(|t| t.id() == id)
                .ok_or_else(|| format!("no sample team has the club id {id:?}"))
        };
        let (a, b) = (find(home)?, find(away)?);
        if a.id() == b.id() {
            return Err(format!(
                "{} cannot play itself; choose another team",
                a.name()
            ));
        }
        Ok([a, b])
    }

    /// Continues the saved match in `file` on the program its version names, or refuses it
    /// with the reason and the `resume` block the page shows. No worker starts on a refusal.
    fn resume_saved(self: &Arc<Self>, file: &Path) {
        let (choice, identity) = crate::engines::resolve(file, &self.previous);
        {
            let mut worker = self.lock();
            worker.refusal = None;
            worker.resumed_from = None;
            if let Some(id) = &identity {
                if let Some(seed) = id.seed {
                    let match_id = MatchId {
                        seed,
                        millis: id.match_millis,
                    }
                    .to_string();
                    worker.snapshot = snapshot_of(&match_id);
                    worker.match_id = match_id;
                }
                worker.snapshot_tick = id.tick;
                worker.resumed_from = id.tick;
            }
            match choice {
                Choice::Refused(refusal) => {
                    tracing::warn!(
                        signal = "launch.refused",
                        kind = refusal.kind.word(),
                        saved.version = refusal.identity.engine_version.as_deref().unwrap_or(""),
                        saved.build = %refusal.identity.build_hash
                    );
                    worker.state = State::Refused {
                        reason: refusal.reason.clone(),
                    };
                    worker.refusal = Some(refusal);
                    return;
                }
                Choice::Current => worker.program = Program::Current,
                Choice::Previous { engine, version } => {
                    worker.program = Program::Previous { engine, version };
                }
            }
        }
        self.start(Some(file.to_path_buf()));
    }

    /// Starts a worker: a new match, or the saved match in `resume` continued from it.
    fn start(self: &Arc<Self>, resume: Option<PathBuf>) {
        let (program, version, content, fresh_millis, seed, teams) = {
            let worker = self.lock();
            let (program, content) = match &worker.program {
                Program::Current => (self.engine.clone(), self.content_dir.clone()),
                Program::Previous { engine, .. } => (
                    engine.program.clone(),
                    Some(engine.content.as_os_str().to_os_string()),
                ),
            };
            (
                program,
                worker.program.version().to_string(),
                content,
                worker.match_millis,
                worker.seed,
                worker.teams.clone(),
            )
        };
        let mut args: Vec<OsString> = vec!["serve".into()];
        match &resume {
            Some(file) => {
                args.push("--resume".into());
                args.push(file.as_os_str().to_os_string());
            }
            None => {
                for (flag, value) in [
                    ("--seed", seed.to_string()),
                    ("--minutes", self.minutes.to_string()),
                    ("--match-millis", fresh_millis.to_string()),
                ] {
                    args.push(flag.into());
                    args.push(value.into());
                }
            }
        }
        args.push("--reconnect-wait".into());
        args.push(RECONNECT_WAIT_S.into());
        // The test seams are this program's own; the previous program does not take them.
        if program == self.engine {
            if let Some(tick) = self
                .drop_client_at
                .lock()
                .expect("the drop seam lock is never poisoned")
                .take()
            {
                args.push("--drop-client-at".into());
                args.push(tick.to_string().into());
            }
            if let Some(tick) = self.fast_forward_to {
                args.push("--fast-forward-to".into());
                args.push(tick.to_string().into());
            }
            if self.test_jump {
                args.push("--test-jump".into());
            }
        }
        if let Some(dir) = content {
            args.push("--content-dir".into());
            args.push(dir);
        }
        args.extend(teams);

        let spawned = Command::new(&program)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut worker = self.lock();
        worker.generation += 1;
        let generation = worker.generation;
        let mut child = match spawned {
            Ok(child) => child,
            Err(err) => {
                let reason = format!("cannot start {}: {err}", program.display());
                tracing::error!(signal = "launch.refused", reason = %reason);
                worker.state = State::Refused { reason };
                return;
            }
        };
        let pid = child.id();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        worker.state = State::Starting;
        worker.child = Some(child);
        worker.pid = Some(pid);
        drop(worker);
        tracing::info!(
            signal = "launch.worker_started",
            pid,
            resume = resume.is_some(),
            engine.version = %version
        );

        // The first stdout line is the socket port.
        let me = Arc::clone(self);
        let reader = std::thread::spawn(move || {
            let Some(stdout) = stdout else { return };
            let mut lines = BufReader::new(stdout).lines();
            if let Some(Ok(line)) = lines.next()
                && let Ok(port) = line.trim().parse::<u16>()
            {
                let mut worker = me.lock();
                if worker.generation == generation && worker.state == State::Starting {
                    worker.state = State::Running { port };
                }
            }
            lines.for_each(drop);
        });
        // Every stderr line reaches the launcher's own stderr; the last error line is kept,
        // because it is the engine's own reason when a worker refuses to start.
        let last_error = Arc::new(Mutex::new(None::<String>));
        let kept = Arc::clone(&last_error);
        let errors = std::thread::spawn(move || {
            let Some(stderr) = stderr else { return };
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                eprintln!("{line}");
                if let Some(reason) = line.strip_prefix("error: ") {
                    *kept.lock().expect("the error lock is never poisoned") =
                        Some(reason.to_string());
                }
            }
        });
        let me = Arc::clone(self);
        std::thread::spawn(move || me.watch(generation, reader, errors, &last_error));
    }

    /// Waits for the worker of `generation` to end and names how it ended.
    fn watch(
        &self,
        generation: u64,
        reader: JoinHandle<()>,
        errors: JoinHandle<()>,
        last_error: &Mutex<Option<String>>,
    ) {
        let status = loop {
            {
                let mut worker = self.lock();
                if worker.generation != generation {
                    return;
                }
                let Some(child) = worker.child.as_mut() else {
                    return;
                };
                match child.try_wait() {
                    Ok(Some(status)) => break status,
                    Ok(None) => {}
                    Err(_) => return,
                }
            }
            std::thread::sleep(WATCH_EVERY);
        };
        // Both pipes are at their end once the process is gone; reading them to the end
        // settles whether a port was ever printed and what the last error said.
        let _ = reader.join();
        let _ = errors.join();
        let code = status.code();
        let snapshot_tick = self.read_snapshot_tick();
        let mut worker = self.lock();
        if worker.generation != generation {
            return;
        }
        worker.child = None;
        worker.pid = None;
        if snapshot_tick.is_some() {
            worker.snapshot_tick = snapshot_tick;
        }
        worker.state = match (&worker.state, code) {
            (State::Abandoned, _) => State::Abandoned,
            (_, Some(0)) => State::Finished,
            (State::Starting, _) => State::Refused {
                reason: last_error
                    .lock()
                    .expect("the error lock is never poisoned")
                    .clone()
                    .unwrap_or_else(|| {
                        format!(
                            "the engine stopped before it opened its socket (exit code {})",
                            code.map_or_else(|| "none".to_string(), |c| c.to_string())
                        )
                    }),
            },
            _ => State::Crashed { code },
        };
        tracing::info!(
            signal = "launch.worker_exited",
            code = code.unwrap_or(-1),
            state = worker.state.word()
        );
        if worker.state == State::Finished {
            self.after_match(&mut worker);
        }
    }

    /// After a match ends or stops: the next match takes a new seed (unless the launch named
    /// one), and the start screen reads the saves again.
    fn after_match(&self, worker: &mut Worker) {
        let Some(door) = &self.front_door else {
            return;
        };
        worker.seed = self.fixed_seed.unwrap_or_else(clock_seed);
        worker.saved = SavedMatch::newest(&self.data)
            .and_then(|p| SavedMatch::read(&p, &door.teams, &door.content));
    }

    /// The tick of the snapshot on disk, when it reads cleanly. The file may be the previous
    /// program's, so it is identified rather than read with this build's check.
    fn read_snapshot_tick(&self) -> Option<u32> {
        let path = self.lock().snapshot.clone();
        let bytes = std::fs::read(path).ok()?;
        Snapshot::identify(&bytes).ok()?.tick
    }

    /// Restarts a stopped match from its snapshot. A snapshot that does not read is refused
    /// with the engine's own reason, and the page offers only to abandon.
    fn restart(self: &Arc<Self>) {
        {
            let worker = self.lock();
            if !matches!(worker.state, State::Crashed { .. }) {
                return;
            }
        }
        tracing::info!(signal = "launch.restart");
        let snapshot = self.lock().snapshot.clone();
        if !snapshot.is_file() {
            let reason =
                "no stoppage was saved before the engine stopped, so there is nothing to restart from"
                    .to_string();
            tracing::warn!(signal = "launch.refused", reason = %reason);
            self.lock().state = State::Refused { reason };
            return;
        }
        // A save of the previous release is that program's to check.
        if let Program::Previous { .. } = self.lock().program {
            let tick = self.read_snapshot_tick();
            if tick.is_some() {
                self.lock().snapshot_tick = tick;
                self.start(Some(snapshot));
                return;
            }
        }
        let shown = snapshot.display().to_string();
        match Snapshot::read(&snapshot, &shown) {
            Ok(read) => {
                self.lock().snapshot_tick = Some(read.tick());
                self.start(Some(snapshot));
            }
            Err(err) => {
                let reason = err.to_string();
                tracing::warn!(
                    signal = "launch.refused",
                    reason = %engine::snapshot::shorten_for_log(
                        &reason,
                        Some(&data_dir())
                    )
                );
                self.lock().state = State::Refused { reason };
            }
        }
    }

    /// Stops a live worker and gives the match up.
    fn abandon(&self) {
        let mut worker = self.lock();
        if let Some(child) = worker.child.as_mut() {
            let _ = child.kill();
        }
        worker.state = State::Abandoned;
        worker.pid = None;
        tracing::info!(signal = "launch.abandoned");
    }

    /// Starts a fresh match on this program, once the match before it is no longer running:
    /// with the launch's seed and teams, or with the two clubs `body` names.
    fn new_match(self: &Arc<Self>, body: Option<&str>) -> Result<(), String> {
        let chosen = match body {
            Some(text) => {
                let pair: Pair = serde_json::from_str(text)
                    .map_err(|e| format!("the new-match body does not read: {e}"))?;
                let [home, away] = self.pair(&pair.home, &pair.away)?;
                Some((home, away))
            }
            None => None,
        };
        {
            let mut worker = self.lock();
            if Self::busy(&worker) {
                return Err("a match is already running".into());
            }
            if let Some((home, away)) = chosen {
                worker.teams = vec![
                    "--team-a".into(),
                    home.path.as_os_str().to_os_string(),
                    "--team-b".into(),
                    away.path.as_os_str().to_os_string(),
                ];
                tracing::info!(
                    signal = "launch.fixture_chosen",
                    home = home.id(),
                    away = away.id(),
                    seed = worker.seed
                );
            }
            let fresh = MatchId::now(worker.seed);
            worker.match_id = fresh.to_string();
            worker.snapshot = snapshot_of(&worker.match_id);
            worker.match_millis = fresh.millis;
            worker.program = Program::Current;
            worker.snapshot_tick = None;
            worker.resumed_from = None;
            worker.refusal = None;
        }
        tracing::info!(signal = "launch.new_match");
        self.start(None);
        Ok(())
    }

    /// Continues the newest unfinished saved match, with the team files its club names name.
    fn resume(self: &Arc<Self>) -> Result<(), String> {
        let (path, files) = {
            let mut worker = self.lock();
            if Self::busy(&worker) {
                return Err("a match is already running".into());
            }
            let door = self
                .front_door
                .as_ref()
                .ok_or("this launch started its match at once; it has no saved match to offer")?;
            worker.saved = SavedMatch::newest(&self.data)
                .and_then(|p| SavedMatch::read(&p, &door.teams, &door.content));
            let saved = worker.saved.as_ref().ok_or("no saved match yet")?;
            let files = if saved.kind() == "current" {
                saved.team_files(&door.teams)
            } else {
                None
            };
            (saved.path.clone(), files)
        };
        self.lock().teams = files.map_or_else(Vec::new, |[a, b]| {
            vec![
                "--team-a".into(),
                a.into_os_string(),
                "--team-b".into(),
                b.into_os_string(),
            ]
        });
        tracing::info!(signal = "launch.resume_saved");
        self.resume_saved(&path);
        Ok(())
    }

    /// Ends a running worker and keeps the snapshot it wrote, so Resume can finish the match.
    /// The launcher goes back to the start screen.
    fn stop(&self) {
        let mut worker = self.lock();
        // A newer generation tells the watcher of this worker to leave the state alone.
        worker.generation += 1;
        if let Some(mut child) = worker.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        worker.pid = None;
        worker.state = State::Idle;
        worker.refusal = None;
        self.after_match(&mut worker);
        tracing::info!(
            signal = "launch.stopped",
            saved.tick = worker.saved.as_ref().and_then(|s| s.identity.tick)
        );
    }

    /// Stops the match and keeps its snapshot, then ends the launcher once the answer has
    /// reached the page. The answer names what the closed page shows.
    fn quit(&self) -> String {
        let played = matches!(self.lock().state, State::Running { .. } | State::Starting);
        self.stop();
        let body = {
            let mut worker = self.lock();
            worker.state = State::Closed;
            let saved = worker
                .saved
                .as_ref()
                .filter(|_| played)
                .map(SavedMatch::json);
            tracing::info!(signal = "launch.quit", saved = saved.is_some());
            let mut status: serde_json::Value =
                serde_json::from_str(&self.json_locked(&mut worker)).expect("the status is JSON");
            status["closed"] = serde_json::json!({ "match": played, "saved": saved });
            status.to_string()
        };
        std::thread::spawn(|| {
            std::thread::sleep(QUIT_AFTER);
            std::process::exit(0);
        });
        body
    }

    /// Saves the player's settings from `body`.
    fn save_settings(&self, body: &str) -> Result<(), String> {
        let settings = Settings::parse(body)?;
        settings
            .save(&self.data)
            .map_err(|e| format!("cannot save the settings: {e}"))?;
        self.lock().settings = settings;
        tracing::info!(
            signal = "launch.settings_saved",
            speed = settings.speed,
            commentary = settings.commentary
        );
        Ok(())
    }

    /// Saves one club's named views from `body`, keeping the club's match ratings.
    fn save_views(&self, body: &str) -> Result<(), String> {
        let mut worker = self.lock();
        let mut views = worker.views.clone();
        let club = views.set_views(body)?;
        views
            .save(&self.data)
            .map_err(|e| format!("cannot save the views: {e}"))?;
        worker.views = views;
        tracing::info!(signal = "launch.views_saved", club = %club);
        Ok(())
    }

    /// Adds the home ratings of the match the launcher last ran, once its record exists, and
    /// saves the store when anything was added. A record that is missing or refused adds
    /// nothing; a refused one says why.
    fn ingest_last_match(&self, worker: &mut Worker) {
        let Some(folder) = worker.snapshot.parent() else {
            return;
        };
        let file = folder.join("stats.json");
        if !file.is_file() {
            return;
        }
        match engine::observe::read_stats(&file) {
            Ok(stats) => {
                if worker.views.ingest(&stats) > 0 {
                    if let Err(e) = worker.views.save(&self.data) {
                        tracing::warn!(signal = "launch.views_refused", reason = %e);
                    } else {
                        tracing::info!(
                            signal = "launch.views_rated",
                            match_id = %stats.match_id
                        );
                    }
                }
            }
            Err(e) => tracing::warn!(signal = "launch.views_refused", reason = %e),
        }
    }

    /// The `/engine.json` body, under the lock the caller holds.
    fn json_locked(&self, worker: &mut Worker) -> String {
        let (port, reason, code) = match &worker.state {
            State::Running { port } => (Some(*port), None, None),
            State::Refused { reason } => (None, Some(reason.clone()), None),
            State::Crashed { code } => (None, None, *code),
            _ => (None, None, None),
        };
        let resume = worker.refusal.as_ref().map(|refusal| {
            let id = &refusal.identity;
            serde_json::json!({
                "kind": refusal.kind.word(),
                "saved.version": id.engine_version,
                "saved.build": id.build_hash,
                "saved.tick": id.tick,
                "saved.teams": id.teams,
                "saved.score": id.score,
                "saved.millis": id.match_millis,
                "engines": [engine::version(), crate::engines::previous_version()],
                "reason": refusal.reason,
            })
        });
        let mut status = serde_json::json!({
            "engine.state": worker.state.word(),
            "socket.port": port,
            "protocol.version": protocol::PROTOCOL_VERSION,
            "engine.pid": worker.pid,
            "engine.path": self.engine.display().to_string(),
            "engine.reason": reason,
            "engine.code": code,
            "engine.version": worker.program.version(),
            "launcher.version": engine::version(),
            "previous.version": crate::engines::previous_version(),
            "snapshot.tick": worker.snapshot_tick,
            "match.id": worker.match_id,
            "match.resumed_from": worker.resumed_from,
            "resume": resume,
            "launcher": true,
            "viewer.skin": self.skin,
            "settings": worker.settings.json(),
        });
        if let Some(door) = &self.front_door {
            status["front-door"] = true.into();
            status["teams"] = door.teams.iter().map(SampleTeam::json).collect();
            status["saved"] = worker
                .saved
                .as_ref()
                .map_or(serde_json::Value::Null, SavedMatch::json);
        }
        status.to_string()
    }
}

impl Status for Arc<Launcher> {
    fn json(&self) -> String {
        // Only while a worker runs does the snapshot move, and it is read before the lock
        // because a read touches the disk. A stopped match keeps the tick read at its end.
        let running = matches!(self.lock().state, State::Running { .. });
        let tick = if running {
            self.read_snapshot_tick()
        } else {
            None
        };
        let mut worker = self.lock();
        if tick.is_some() {
            worker.snapshot_tick = tick;
        }
        self.json_locked(&mut worker)
    }

    fn act(&self, action: Action) -> Option<Result<String, String>> {
        let done = match action {
            Action::Restart => {
                self.restart();
                Ok(())
            }
            Action::Abandon => {
                self.abandon();
                Ok(())
            }
            Action::NewMatch(body) => self.new_match(body.as_deref()),
            Action::Resume => self.resume(),
            Action::Stop => {
                self.stop();
                Ok(())
            }
            Action::Quit => return Some(Ok(self.quit())),
            Action::Settings(body) => self.save_settings(&body),
            Action::Views(body) => self.save_views(&body),
        };
        Some(done.map(|()| self.json()))
    }

    fn earlier_events(&self) -> Option<String> {
        let (resumed_from, folder) = {
            let worker = self.lock();
            (
                worker.resumed_from,
                worker.snapshot.parent().map(Path::to_path_buf),
            )
        };
        let rows = match (resumed_from, folder) {
            (Some(tick), Some(folder)) => earlier_events(&folder.join(EVENTS_FILE), tick),
            _ => Vec::new(),
        };
        Some(serde_json::Value::Array(rows).to_string())
    }

    fn views(&self, club: &str) -> Option<String> {
        let mut worker = self.lock();
        self.ingest_last_match(&mut worker);
        Some(worker.views.club_json(club).to_string())
    }

    fn round(&self, home: &str, away: &str) -> Option<Result<String, String>> {
        let door = self.front_door.as_ref()?;
        let answer = self.pair(home, away).map(|_| {
            let seed = self.lock().seed;
            let round = Round::for_match(&door.dir, &door.content, [home, away], seed);
            let club = |id: &str| {
                door.teams
                    .iter()
                    .find(|t| t.id() == id)
                    .map_or(serde_json::Value::Null, SampleTeam::json)
            };
            serde_json::json!({
                "fixtures": round
                    .fixtures
                    .iter()
                    .map(|f| serde_json::json!({
                        "home": club(&f.club_ids[0]),
                        "away": club(&f.club_ids[1]),
                    }))
                    .collect::<Vec<_>>(),
            })
            .to_string()
        });
        Some(answer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A resumed match's earlier events are the rows at or before the save's tick; later
    /// rows, a row that does not read, and a missing file give nothing.
    #[test]
    fn earlier_events_are_the_rows_up_to_the_save() {
        let dir = std::env::temp_dir().join(format!("earlier_events_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(EVENTS_FILE);
        std::fs::write(
            &path,
            concat!(
                "{\"tick\":0,\"event.type\":\"kick-off\"}\n",
                "{\"tick\":900,\"event.type\":\"goal\",\"home.score\":1}\n",
                "not json\n",
                "{\"tick\":1200,\"event.type\":\"goal\",\"home.score\":2}\n",
            ),
        )
        .unwrap();
        let rows = earlier_events(&path, 1000);
        let ticks: Vec<u64> = rows.iter().map(|r| r["tick"].as_u64().unwrap()).collect();
        assert_eq!(ticks, [0, 900]);
        assert!(earlier_events(&dir.join("missing.jsonl"), 1000).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
