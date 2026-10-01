//! `engine-cli launch`: serve the viewer page and run the engine as a separate worker.
//!
//! A browser page cannot start a program, and a page served by the engine loses its server,
//! its socket, and every way to ask for help when the engine stops. The launcher is the part
//! that survives: it serves the page, starts `engine-cli serve` as a child process, watches
//! it, and reports its state in `/engine.json`. After a crash the page asks for a restart
//! with a POST, and the launcher starts the worker again from the match's latest snapshot.
//! The page sends no match state: the snapshot on disk is the only copy the engine needs.
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

use engine::Snapshot;
use engine::observe::identity::{MatchId, data_dir};

use crate::cli::LaunchOpts;
use crate::engines::{Choice, PreviousEngine, Refusal};
use crate::web::{Action, Status};

/// Seconds a worker waits for a page that lost its connection.
const RECONNECT_WAIT_S: &str = "120";
/// How often the watcher looks at the worker.
const WATCH_EVERY: Duration = Duration::from_millis(50);

/// Where the worker stands, as `/engine.json` names it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
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
}

impl State {
    fn word(&self) -> &'static str {
        match self {
            State::Starting => "starting",
            State::Running { .. } => "running",
            State::Finished => "finished",
            State::Crashed { .. } => "crashed",
            State::Refused { .. } => "refused",
            State::Abandoned => "abandoned",
            State::NotFound => "not-found",
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

/// The launcher: its configuration and the one worker it supervises.
pub struct Launcher {
    worker: Mutex<Worker>,
    engine: PathBuf,
    /// The previous release's program, for saves of that release.
    previous: PreviousEngine,
    /// The content folder this program's workers receive, when one was named.
    content_dir: Option<OsString>,
    /// The team files every worker receives.
    teams: Vec<OsString>,
    /// The skin folder the viewer loads, from the `viewer.skin` slot.
    skin: &'static str,
    seed: u64,
    minutes: u32,
    drop_client_at: Mutex<Option<u32>>,
}

/// The snapshot file of a match in the data folder.
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
    let content = engine::Content::load(&engine::ContentDir::resolve(content_dir)?)?;
    let skin = content.modules.skin.skin();
    let seed = opts.seed.unwrap_or_else(|| {
        let seed = clock_seed();
        // Stdout carries the page address alone, so the chosen seed goes to stderr, where a
        // person can read it back to play the same match again.
        eprintln!("seed {seed}");
        seed
    });
    let started = MatchId::now(seed);
    let match_id = started.to_string();
    let mut teams = Vec::new();
    for (flag, file) in [("--team-a", &opts.team_a), ("--team-b", &opts.team_b)] {
        if let Some(file) = file {
            teams.push(OsString::from(flag));
            teams.push(file.as_os_str().to_os_string());
        }
    }
    let launcher = Arc::new(Launcher {
        worker: Mutex::new(Worker {
            state: State::Starting,
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
        }),
        engine,
        previous: PreviousEngine::locate(opts.previous.as_deref()),
        content_dir: content_dir.map(|dir| dir.as_os_str().to_os_string()),
        teams,
        skin,
        seed,
        minutes: opts.minutes,
        drop_client_at: Mutex::new(opts.drop_client_at),
    });

    let page = crate::web::start(&web, Arc::new(Arc::clone(&launcher)))?;
    if launcher.engine.is_file() {
        match opts.resume.as_deref() {
            Some(file) => launcher.resume_saved(file),
            None => launcher.start(None),
        }
    } else {
        // The page still loads: it is where the manager reads the path and what to do.
        launcher.lock().state = State::NotFound;
        tracing::warn!(
            signal = "launch.not_found",
            path = %shown_file(&launcher.engine)
        );
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

impl Launcher {
    fn lock(&self) -> std::sync::MutexGuard<'_, Worker> {
        self.worker
            .lock()
            .expect("the worker lock is never poisoned")
    }

    /// Continues the saved match in `file` on the program its version names, or refuses it
    /// with the reason and the `resume` block the page shows. No worker starts on a refusal.
    fn resume_saved(self: &Arc<Self>, file: &Path) {
        let (choice, identity) = crate::engines::resolve(file, &self.previous);
        {
            let mut worker = self.lock();
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
        let (program, version, content, fresh_millis) = {
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
                    ("--seed", self.seed.to_string()),
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
        // The test seam is this program's own; the previous program does not take it.
        if program == self.engine
            && let Some(tick) = self
                .drop_client_at
                .lock()
                .expect("the drop seam lock is never poisoned")
                .take()
        {
            args.push("--drop-client-at".into());
            args.push(tick.to_string().into());
        }
        if let Some(dir) = content {
            args.push("--content-dir".into());
            args.push(dir);
        }
        args.extend(self.teams.iter().cloned());

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

    /// Starts a fresh match with the launch's seed and teams on this program, once the match
    /// before it is no longer running: after a refused save, a full time, or an abandon.
    fn new_match(self: &Arc<Self>) {
        {
            let mut worker = self.lock();
            if matches!(worker.state, State::Running { .. } | State::Starting)
                && worker.child.is_some()
            {
                return;
            }
            let fresh = MatchId::now(self.seed);
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
        serde_json::json!({
            "engine.state": worker.state.word(),
            "socket.port": port,
            "protocol.version": protocol::PROTOCOL_VERSION,
            "engine.pid": worker.pid,
            "engine.path": self.engine.display().to_string(),
            "engine.reason": reason,
            "engine.code": code,
            "engine.version": worker.program.version(),
            "launcher.version": engine::version(),
            "snapshot.tick": worker.snapshot_tick,
            "match.id": worker.match_id,
            "match.resumed_from": worker.resumed_from,
            "resume": resume,
            "launcher": true,
            "viewer.skin": self.skin,
        })
        .to_string()
    }

    fn act(&self, action: Action) -> Option<String> {
        match action {
            Action::Restart => self.restart(),
            Action::Abandon => self.abandon(),
            Action::NewMatch => self.new_match(),
        }
        Some(self.json())
    }
}
