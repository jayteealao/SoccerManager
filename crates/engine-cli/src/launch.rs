//! `engine-cli launch`: serve the viewer page and run the engine as a separate worker.
//!
//! A browser page cannot start a program, and a page served by the engine loses its server,
//! its socket, and every way to ask for help when the engine stops. The launcher is the part
//! that survives: it serves the page, starts `engine-cli serve` as a child process, watches
//! it, and reports its state in `/engine.json`. After a crash the page asks for a restart
//! with a POST, and the launcher starts the worker again from the match's latest snapshot.
//! The page sends no match state: the snapshot on disk is the only copy the engine needs.

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
}

/// The launcher: its configuration and the one worker it supervises.
pub struct Launcher {
    worker: Mutex<Worker>,
    engine: PathBuf,
    /// Arguments every worker receives: the content folder and the team files.
    common: Vec<OsString>,
    snapshot: PathBuf,
    match_id: String,
    match_millis: u64,
    seed: u64,
    minutes: u32,
    drop_client_at: Mutex<Option<u32>>,
}

pub fn run(content_dir: Option<&Path>, opts: &LaunchOpts) -> anyhow::Result<i32> {
    let engine = engine_path(opts.engine.as_deref());
    let web = crate::web::resolve_web_dir(opts.web.as_deref())?;
    let seed = opts.seed.unwrap_or_else(|| {
        let seed = clock_seed();
        // Stdout carries the page address alone, so the chosen seed goes to stderr, where a
        // person can read it back to play the same match again.
        eprintln!("seed {seed}");
        seed
    });
    let started = MatchId::now(seed);
    let match_id = started.to_string();
    let mut common = Vec::new();
    if let Some(dir) = content_dir {
        common.push(OsString::from("--content-dir"));
        common.push(dir.as_os_str().to_os_string());
    }
    for (flag, file) in [("--team-a", &opts.team_a), ("--team-b", &opts.team_b)] {
        if let Some(file) = file {
            common.push(OsString::from(flag));
            common.push(file.as_os_str().to_os_string());
        }
    }
    let launcher = Arc::new(Launcher {
        worker: Mutex::new(Worker {
            state: State::Starting,
            child: None,
            pid: None,
            generation: 0,
            snapshot_tick: None,
        }),
        snapshot: data_dir()
            .join("matches")
            .join(&match_id)
            .join(engine::snapshot::FILE_NAME),
        engine,
        common,
        match_id,
        match_millis: started.millis,
        seed,
        minutes: opts.minutes,
        drop_client_at: Mutex::new(opts.drop_client_at),
    });

    let page = crate::web::start(&web, Arc::new(Arc::clone(&launcher)))?;
    if launcher.engine.is_file() {
        launcher.start(None);
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

    /// Starts a worker: a new match, or the match in `resume` continued from its snapshot.
    fn start(self: &Arc<Self>, resume: Option<u32>) {
        let mut args: Vec<OsString> = vec!["serve".into()];
        match resume {
            Some(_) => {
                args.push("--resume".into());
                args.push(self.snapshot.as_os_str().to_os_string());
            }
            None => {
                for (flag, value) in [
                    ("--seed", self.seed.to_string()),
                    ("--minutes", self.minutes.to_string()),
                    ("--match-millis", self.match_millis.to_string()),
                ] {
                    args.push(flag.into());
                    args.push(value.into());
                }
            }
        }
        args.push("--reconnect-wait".into());
        args.push(RECONNECT_WAIT_S.into());
        if let Some(tick) = self
            .drop_client_at
            .lock()
            .expect("the drop seam lock is never poisoned")
            .take()
        {
            args.push("--drop-client-at".into());
            args.push(tick.to_string().into());
        }
        args.extend(self.common.iter().cloned());

        let spawned = Command::new(&self.engine)
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
                let reason = format!("cannot start {}: {err}", self.engine.display());
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
            snapshot.tick = resume.unwrap_or(0)
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

    /// The tick of the snapshot on disk, when it reads cleanly.
    fn read_snapshot_tick(&self) -> Option<u32> {
        Snapshot::read(&self.snapshot, engine::snapshot::FILE_NAME)
            .ok()
            .map(|s| s.tick())
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
        if !self.snapshot.is_file() {
            let reason =
                "no stoppage was saved before the engine stopped, so there is nothing to restart from"
                    .to_string();
            tracing::warn!(signal = "launch.refused", reason = %reason);
            self.lock().state = State::Refused { reason };
            return;
        }
        let shown = self.snapshot.display().to_string();
        match Snapshot::read(&self.snapshot, &shown) {
            Ok(snapshot) => {
                self.lock().snapshot_tick = Some(snapshot.tick());
                self.start(Some(snapshot.tick()));
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
        serde_json::json!({
            "engine.state": worker.state.word(),
            "socket.port": port,
            "protocol.version": protocol::PROTOCOL_VERSION,
            "engine.pid": worker.pid,
            "engine.path": self.engine.display().to_string(),
            "engine.reason": reason,
            "engine.code": code,
            "snapshot.tick": worker.snapshot_tick,
            "match.id": self.match_id,
            "launcher": true,
        })
        .to_string()
    }

    fn act(&self, action: Action) -> Option<String> {
        match action {
            Action::Restart => self.restart(),
            Action::Abandon => self.abandon(),
        }
        Some(self.json())
    }
}
