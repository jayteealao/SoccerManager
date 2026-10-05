//! The launcher: it serves the page, runs the engine as a worker, and restarts a match that
//! crashed from its latest snapshot. These tests kill a real worker process.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use protocol::{ClientCommand, ServerMessage};
use serde_json::Value;
use stream::{Client, Incoming};

/// Kills the launcher, and the worker it names, whatever happens.
struct Launched {
    child: Child,
    page_port: u16,
    logs: Arc<Mutex<String>>,
}

impl Drop for Launched {
    fn drop(&mut self) {
        if let Some(pid) = status(self.page_port)["engine.pid"].as_u64() {
            kill(pid);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("engine-cli-launch-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the test folder is created");
    dir
}

/// A page folder shaped like the built viewer, written inside `dir`, so the tests need no
/// viewer build: an `index.html` that loads one module.
fn web(dir: &Path) -> PathBuf {
    let web = dir.join("page");
    std::fs::create_dir_all(web.join("assets")).expect("the page folder is created");
    std::fs::write(
        web.join("index.html"),
        "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"utf-8\" />\n    \
         <script type=\"module\" src=\"/assets/index-test.js\"></script>\n  </head>\n  \
         <body><div id=\"app\"></div></body>\n</html>\n",
    )
    .expect("the page is written");
    std::fs::write(web.join("assets").join("index-test.js"), "export {};\n")
        .expect("the module is written");
    web
}

/// Starts `engine-cli launch` with `args` and reads the page address it prints.
fn launch(dir: &Path, args: &[&str]) -> Launched {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", dir)
        .env("SM_LOG", "info")
        .env(
            "SM_CONTENT_DIR",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        )
        .arg("launch")
        .arg("--web")
        .arg(web(dir))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = child.stderr.take().expect("the launcher logs to stderr");
    let logs = Arc::new(Mutex::new(String::new()));
    let kept = Arc::clone(&logs);
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut buf) {
            if n == 0 {
                break;
            }
            kept.lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&buf[..n]));
        }
    });
    let mut stdout = BufReader::new(child.stdout.take().expect("the launcher prints"));
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let page_port: u16 = line
        .trim()
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches('/')
        .parse()
        .unwrap_or_else(|e| panic!("launch must print the page address, printed {line:?}: {e}"));
    Launched {
        child,
        page_port,
        logs,
    }
}

/// One HTTP request; the status code and the body.
fn http(port: u16, method: &str, target: &str, origin: Option<&str>) -> (u16, String) {
    let mut socket = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let origin = origin
        .map(|o| format!("Origin: {o}\r\n"))
        .unwrap_or_default();
    socket
        .write_all(
            format!(
                "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n{origin}Content-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .unwrap();
    let mut response = Vec::new();
    socket.read_to_end(&mut response).unwrap();
    let text = String::from_utf8_lossy(&response).into_owned();
    let code = text
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let body = text
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or_default()
        .to_string();
    (code, body)
}

fn status(port: u16) -> Value {
    let (_, body) = http(port, "GET", "/engine.json", None);
    serde_json::from_str(&body).unwrap_or(Value::Null)
}

/// Polls `/engine.json` until `done` holds, for at most `secs` seconds.
fn wait_for(port: u16, secs: u64, done: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        let now = status(port);
        if done(&now) {
            return now;
        }
        assert!(Instant::now() < deadline, "timed out; last status {now}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn post(launched: &Launched, path: &str) -> (u16, Value) {
    let origin = format!("http://127.0.0.1:{}", launched.page_port);
    let (code, body) = http(launched.page_port, "POST", path, Some(&origin));
    (code, serde_json::from_str(&body).unwrap_or(Value::Null))
}

/// Kills a process the way a crash would: no chance to clean up.
fn kill(pid: u64) {
    let pid = pid.to_string();
    let _ = if cfg!(windows) {
        Command::new("taskkill")
            .args(["/F", "/PID", &pid])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    } else {
        Command::new("kill").args(["-9", &pid]).status()
    };
}

/// Every event a client read, as (tick, home score, away score).
type Seen = Arc<Mutex<Vec<(u32, u32, u32)>>>;

/// Connects to the running worker, starts the match, and keeps reading on a thread.
fn watch_match(port: u16) -> (Seen, Arc<Mutex<Option<String>>>) {
    let mut client = Client::connect_local(port).unwrap();
    let seen: Seen = Arc::default();
    let match_id = Arc::new(Mutex::new(None));
    let (events, id) = (Arc::clone(&seen), Arc::clone(&match_id));
    client.send(&ClientCommand::Start).unwrap();
    std::thread::spawn(move || {
        while let Ok(incoming) = client.read() {
            match incoming {
                Incoming::Message(message) => match *message {
                    ServerMessage::Hello(hello) => *id.lock().unwrap() = Some(hello.match_id),
                    ServerMessage::Event(event) => events.lock().unwrap().push((
                        event.tick,
                        event.home_score,
                        event.away_score,
                    )),
                    _ => {}
                },
                Incoming::Tick(..) => {}
                Incoming::Closed => break,
            }
        }
    });
    (seen, match_id)
}

/// Runs a match until a snapshot is on disk, then kills the worker. Returns the status the
/// launcher reports after the crash and every event the client read.
fn crash(launched: &Launched) -> (Value, Seen, String) {
    let running = wait_for(launched.page_port, 30, |s| s["engine.state"] == "running");
    let port = u16::try_from(running["socket.port"].as_u64().unwrap()).unwrap();
    let (seen, _) = watch_match(port);
    let at = wait_for(launched.page_port, 60, |s| {
        s["snapshot.tick"].as_u64().unwrap_or(0) > 0
    });
    kill(at["engine.pid"].as_u64().unwrap());
    let crashed = wait_for(launched.page_port, 30, |s| s["engine.state"] == "crashed");
    (
        crashed,
        seen,
        running["match.id"].as_str().unwrap().to_string(),
    )
}

#[test]
fn a_missing_engine_is_reported_with_its_path() {
    let dir = temp("not-found");
    let missing = dir.join("absent").join("engine-cli.exe");
    let launched = launch(
        &dir,
        &[
            "--no-start-screen",
            "--seed",
            "42",
            "--engine",
            missing.to_str().unwrap(),
        ],
    );
    let now = status(launched.page_port);
    assert_eq!(now["engine.state"], "not-found", "{now}");
    assert_eq!(now["engine.path"], missing.display().to_string());
    assert!(now["socket.port"].is_null());
    // The page itself is still served, so the manager can read the path.
    let (code, body) = http(launched.page_port, "GET", "/", None);
    assert_eq!(code, 200);
    assert!(body.contains("<script type=\"module\""));
}

#[test]
fn a_killed_engine_restarts_from_its_snapshot_with_the_same_score() {
    let dir = temp("kill-restart");
    let launched = launch(&dir, &["--no-start-screen", "--seed", "42"]);
    let (crashed, seen, match_id) = crash(&launched);
    let snapshot_tick = u32::try_from(crashed["snapshot.tick"].as_u64().unwrap()).unwrap();
    assert!(snapshot_tick > 0);
    assert!(crashed["engine.pid"].is_null());

    let (code, after) = post(&launched, "/engine/restart");
    assert_eq!(code, 202, "{after}");
    let running = wait_for(launched.page_port, 30, |s| s["engine.state"] == "running");
    assert_eq!(running["match.id"], match_id.as_str());

    let port = u16::try_from(running["socket.port"].as_u64().unwrap()).unwrap();
    let mut client = Client::connect_local(port).unwrap();
    let Incoming::Message(message) = client.read().unwrap() else {
        panic!("the first frame is the hello");
    };
    let ServerMessage::Hello(hello) = *message else {
        panic!("the first message is the hello");
    };
    assert_eq!(hello.match_id, match_id, "the same match continues");
    let first = loop {
        match client.read().unwrap() {
            Incoming::Tick(frame, q) => break (frame.kind(), q.tick),
            Incoming::Message(_) => continue,
            Incoming::Closed => panic!("the resumed match closed before its first tick"),
        }
    };
    assert_eq!(first.1, snapshot_tick + 1, "play resumes at the stoppage");
    assert_ne!(
        first.0,
        protocol::frame::KIND_DELTA,
        "the first frame is a keyframe"
    );

    // The score the manager saw at the stoppage is the score the resumed match holds.
    let before = seen
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|(tick, ..)| *tick <= snapshot_tick)
        .map_or((0, 0), |&(_, home, away)| (home, away));
    let logs = launched.logs.lock().unwrap().clone();
    let resumed = logs
        .lines()
        .find(|l| l.contains("signal=\"match.resumed\""))
        .unwrap_or_else(|| panic!("the resumed worker names its score: {logs}"));
    assert!(
        resumed.contains(&format!("home.score={}", before.0))
            && resumed.contains(&format!("away.score={}", before.1)),
        "resumed {resumed}; the client saw {before:?}"
    );
    drop(client);
}

#[test]
fn a_corrupt_snapshot_is_refused_by_name() {
    let dir = temp("corrupt");
    let launched = launch(&dir, &["--no-start-screen", "--seed", "42"]);
    let (_, _, match_id) = crash(&launched);
    let snapshot = dir.join("matches").join(&match_id).join("snapshot.smsn");
    let mut bytes = std::fs::read(&snapshot).unwrap();
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0xff;
    std::fs::write(&snapshot, bytes).unwrap();

    let (code, after) = post(&launched, "/engine/restart");
    assert_eq!(code, 202);
    assert_eq!(after["engine.state"], "refused", "{after}");
    let reason = after["engine.reason"].as_str().unwrap();
    assert!(reason.contains("checksum"), "{reason}");
}

#[test]
fn abandon_stops_the_worker() {
    let dir = temp("abandon");
    let launched = launch(&dir, &["--no-start-screen", "--seed", "42"]);
    let running = wait_for(launched.page_port, 30, |s| s["engine.state"] == "running");
    let pid = running["engine.pid"].as_u64().unwrap();
    let (code, after) = post(&launched, "/engine/abandon");
    assert_eq!(code, 202);
    assert_eq!(after["engine.state"], "abandoned", "{after}");
    assert!(after["engine.pid"].is_null());
    let settled = wait_for(launched.page_port, 10, |s| s["engine.state"] == "abandoned");
    assert!(settled["socket.port"].is_null());
    assert!(pid > 0);
}

#[test]
fn an_action_from_another_origin_is_refused() {
    let dir = temp("origin");
    let launched = launch(&dir, &["--no-start-screen", "--seed", "42"]);
    wait_for(launched.page_port, 30, |s| s["engine.state"] == "running");
    let (code, _) = http(
        launched.page_port,
        "POST",
        "/engine/abandon",
        Some("http://example.com"),
    );
    assert_eq!(code, 403);
    let (code, _) = http(launched.page_port, "POST", "/engine/abandon", None);
    assert_eq!(code, 403);
    assert_eq!(status(launched.page_port)["engine.state"], "running");
}

/// The committed save stamped as written by release 0.1.0 (format 8, build 3ba8fed).
fn saved_0_1_0() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../e2e/match/fixtures/saved-0.1.0.smsn")
}

#[test]
fn a_save_two_or_more_versions_back_is_refused_naming_its_version() {
    let dir = temp("older");
    let fixture = saved_0_1_0();
    let launched = launch(
        &dir,
        &["--seed", "42", "--resume", fixture.to_str().unwrap()],
    );
    let refused = wait_for(launched.page_port, 10, |s| s["engine.state"] == "refused");
    assert!(
        refused["engine.pid"].is_null(),
        "no worker started: {refused}"
    );
    let reason = refused["engine.reason"].as_str().unwrap();
    assert!(
        reason.starts_with("this match was saved by Touchline 0.1.0, two or more versions back"),
        "{reason}"
    );
    let resume = &refused["resume"];
    assert_eq!(resume["kind"], "older");
    assert_eq!(resume["saved.version"], "0.1.0");
    assert_eq!(resume["saved.build"], "3ba8fed");
    assert_eq!(resume["saved.tick"], (52 * 60 + 10) * 50);
    assert_eq!(resume["saved.millis"], 1_700_000_000_000u64);
    assert_eq!(resume["saved.teams"].as_array().map(Vec::len), Some(2));
    assert_eq!(resume["engines"][1], "0.2.0-beta.1");
    assert_eq!(resume["engines"][0], refused["launcher.version"]);
    assert_eq!(refused["engine.version"], refused["launcher.version"]);
    // The save stays on disk.
    assert!(fixture.is_file());

    // NEW MATCH starts a fresh match on this program.
    let (code, after) = post(&launched, "/engine/new-match");
    assert_eq!(code, 202);
    assert!(after["resume"].is_null(), "{after}");
    let running = wait_for(launched.page_port, 30, |s| s["engine.state"] == "running");
    assert!(running["match.resumed_from"].is_null());
    assert_ne!(running["match.id"], refused["match.id"]);
}

#[test]
fn a_save_of_this_version_resumes_on_this_program() {
    let dir = temp("same-version");
    let out = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", &dir)
        .env(
            "SM_CONTENT_DIR",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        )
        .args(["simulate", "--seed", "42", "--minutes", "20", "--ticks-out"])
        .arg(dir.join("m.ticks"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let stats: Value = serde_json::from_slice(&out.stdout).unwrap();
    let match_id = stats["match.id"].as_str().unwrap().to_string();
    let snapshot = dir.join("matches").join(&match_id).join("snapshot.smsn");
    let tick = engine::Snapshot::read(&snapshot, "s").unwrap().tick();

    let launched = launch(&dir, &["--resume", snapshot.to_str().unwrap()]);
    let running = wait_for(launched.page_port, 30, |s| s["engine.state"] == "running");
    assert_eq!(running["match.id"], match_id.as_str());
    assert_eq!(running["match.resumed_from"], tick);
    assert_eq!(running["engine.version"], running["launcher.version"]);
    assert!(running["resume"].is_null());
}
