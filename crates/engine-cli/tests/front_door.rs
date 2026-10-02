//! The launcher's start screen: it opens with no match, starts the fixture the page chooses,
//! stops a match and keeps its save, resumes that save to the result the match would have
//! had, keeps the player's settings across launches, and quits. These tests run the real
//! launcher and its worker processes.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use protocol::{ClientCommand, Quantised, ServerMessage};
use serde_json::Value;
use stream::{Client, Incoming};

/// The launcher, killed with the worker it names whatever happens.
struct Launched {
    child: Child,
    port: u16,
}

impl Drop for Launched {
    fn drop(&mut self) {
        if let Some(pid) = status(self.port)["engine.pid"].as_u64() {
            kill(pid);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "engine-cli-front-door-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the test folder is created");
    dir
}

/// A page folder shaped like the built viewer, so the tests need no viewer build.
fn web(dir: &Path) -> PathBuf {
    let web = dir.join("page");
    std::fs::create_dir_all(&web).expect("the page folder is created");
    std::fs::write(web.join("index.html"), "<!doctype html><title>t</title>\n")
        .expect("the page is written");
    web
}

/// Starts `engine-cli launch` with `args` on the data folder `data`.
fn launch(data: &Path, args: &[&str]) -> Launched {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", data)
        .env(
            "SM_CONTENT_DIR",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        )
        .arg("launch")
        .arg("--web")
        .arg(web(data))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().expect("the launcher prints"));
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let port = line
        .trim()
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches('/')
        .parse()
        .unwrap_or_else(|e| panic!("launch must print the page address, printed {line:?}: {e}"));
    Launched { child, port }
}

/// One HTTP request with an optional origin and body; the status code and the body.
fn http(port: u16, method: &str, target: &str, origin: bool, body: &str) -> (u16, String) {
    let Ok(mut socket) = TcpStream::connect(("127.0.0.1", port)) else {
        return (0, String::new());
    };
    let origin = if origin {
        format!("Origin: http://127.0.0.1:{port}\r\n")
    } else {
        "Origin: http://evil.example\r\n".to_string()
    };
    let request = format!(
        "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n{origin}Content-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    if socket.write_all(request.as_bytes()).is_err() {
        return (0, String::new());
    }
    let mut response = Vec::new();
    let _ = socket.read_to_end(&mut response);
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
    let (_, body) = http(port, "GET", "/engine.json", false, "");
    serde_json::from_str(&body).unwrap_or(Value::Null)
}

fn post(launched: &Launched, path: &str, body: &str) -> (u16, String) {
    http(launched.port, "POST", path, true, body)
}

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

/// `true` while a process with `pid` exists.
fn alive(pid: u64) -> bool {
    if cfg!(windows) {
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&format!(" {pid} "))
    } else {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
}

fn team_id(status: &Value, name: &str) -> String {
    status["teams"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .unwrap_or_else(|| panic!("no team named {name}"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// The two default clubs, home first.
fn default_pair(status: &Value) -> String {
    format!(
        r#"{{"home":"{}","away":"{}"}}"#,
        team_id(status, "Oakmere Rangers"),
        team_id(status, "Eldstead City")
    )
}

/// What a page read of a match: the hello's clubs and every tick's positions.
struct Watched {
    clubs: [String; 2],
    ticks: Vec<Quantised>,
}

/// Connects to the running worker and kicks off. With `skip`, the page skips at once and
/// reads to the close; without, it returns after the hello and leaves the match running.
fn watch(port: u16, skip: bool) -> Watched {
    let mut client = Client::connect_local(port).unwrap();
    let Incoming::Message(message) = client.read().unwrap() else {
        panic!("the first frame is the hello");
    };
    let ServerMessage::Hello(hello) = *message else {
        panic!("the first message is the hello");
    };
    let clubs = hello.teams.clone().map(|t| t.name);
    client.send(&ClientCommand::Start).unwrap();
    let mut ticks = Vec::new();
    if !skip {
        std::thread::spawn(
            move || while !matches!(client.read(), Ok(Incoming::Closed) | Err(_)) {},
        );
        return Watched { clubs, ticks };
    }
    let mut skipped = false;
    loop {
        match client.read().unwrap() {
            Incoming::Tick(_, q) => {
                if !skipped {
                    client.send(&ClientCommand::Skip).unwrap();
                    skipped = true;
                }
                ticks.push(q);
            }
            Incoming::Message(m) => {
                if let ServerMessage::Reject(reject) = *m {
                    panic!("the engine refused {}: {}", reject.command, reject.reason);
                }
            }
            Incoming::Closed => break,
        }
    }
    let _ = client.close();
    Watched { clubs, ticks }
}

fn socket_port(status: &Value) -> u16 {
    u16::try_from(status["socket.port"].as_u64().unwrap()).unwrap()
}

#[test]
fn an_idle_launch_offers_the_sample_teams_and_starts_nothing() {
    let data = temp("idle");
    let launched = launch(&data, &["--seed", "42"]);
    let now = status(launched.port);
    assert_eq!(now["front-door"], true, "{now}");
    assert_eq!(now["engine.state"], "idle");
    assert!(now["engine.pid"].is_null());
    assert_eq!(now["teams"].as_array().unwrap().len(), 10);
    assert!(now["saved"].is_null());
    assert_eq!(now["settings"]["speed"], 1);
    assert_eq!(now["settings"]["motion"], "follow");
    assert_eq!(now["settings"]["commentary"], true);
    assert!(now["previous.version"].is_string());

    // The round preview leaves the two picked clubs out and pairs the other eight.
    let (home, away) = (
        team_id(&now, "Oakmere Rangers"),
        team_id(&now, "Eldstead City"),
    );
    let (code, body) = http(
        launched.port,
        "GET",
        &format!("/engine/round?home={home}&away={away}"),
        false,
        "",
    );
    assert_eq!(code, 200, "{body}");
    let round: Value = serde_json::from_str(&body).unwrap();
    let fixtures = round["fixtures"].as_array().unwrap();
    assert_eq!(fixtures.len(), 4);
    for f in fixtures {
        for side in ["home", "away"] {
            assert_ne!(f[side]["id"], home.as_str());
            assert_ne!(f[side]["id"], away.as_str());
        }
    }
    let (code, _) = http(
        launched.port,
        "GET",
        &format!("/engine/round?home={home}&away={home}"),
        false,
        "",
    );
    assert_eq!(code, 400);
}

#[test]
fn a_refused_pick_starts_nothing() {
    let data = temp("refused");
    let launched = launch(&data, &["--seed", "42"]);
    let now = status(launched.port);
    let home = team_id(&now, "Oakmere Rangers");
    let (code, body) = post(
        &launched,
        "/engine/new-match",
        &format!(r#"{{"home":"{home}","away":"{home}"}}"#),
    );
    assert_eq!(code, 400);
    assert!(body.contains("cannot play itself"), "{body}");
    let (code, body) = post(
        &launched,
        "/engine/new-match",
        &format!(r#"{{"home":"{home}","away":"club-nowhere"}}"#),
    );
    assert_eq!(code, 400);
    assert!(body.contains("club-nowhere"), "{body}");
    let (code, _) = post(&launched, "/engine/settings", &" ".repeat(5000));
    assert_eq!(code, 413);
    let (code, _) = http(
        launched.port,
        "POST",
        "/engine/new-match",
        false,
        &default_pair(&now),
    );
    assert_eq!(code, 403);
    let (code, body) = post(&launched, "/engine/resume", "");
    assert_eq!(code, 400);
    assert!(body.contains("no saved match"), "{body}");
    let after = status(launched.port);
    assert_eq!(after["engine.state"], "idle");
    assert!(after["engine.pid"].is_null());
}

#[test]
fn the_chosen_fixture_plays_with_the_two_chosen_clubs() {
    let data = temp("fixture");
    let launched = launch(&data, &["--seed", "42", "--minutes", "2"]);
    let now = status(launched.port);
    // A generated club at home to the first default club.
    let generated = now["teams"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] != "Oakmere Rangers" && t["name"] != "Eldstead City")
        .unwrap()
        .clone();
    let home = generated["id"].as_str().unwrap();
    let away = team_id(&now, "Oakmere Rangers");
    let pick = format!(r#"{{"home":"{home}","away":"{away}"}}"#);
    let (code, body) = post(&launched, "/engine/new-match", &pick);
    assert_eq!(code, 202, "{body}");
    let running = wait_for(launched.port, 30, |s| s["engine.state"] == "running");
    let watched = watch(socket_port(&running), true);
    assert_eq!(
        watched.clubs,
        [
            generated["name"].as_str().unwrap().to_string(),
            "Oakmere Rangers".to_string()
        ]
    );
    let done = wait_for(launched.port, 60, |s| s["engine.state"] == "finished");
    assert!(done["engine.pid"].is_null());
}

/// AC-6, the result half: a match stopped from the start screen and resumed later ends with
/// every tick after the save equal to the same fixture and seed played through.
#[test]
fn return_to_start_keeps_the_save_and_resume_finishes_the_same_match() {
    // The match played through, from the direct start.
    let whole = {
        let data = temp("whole");
        let launched = launch(
            &data,
            &["--no-start-screen", "--seed", "42", "--minutes", "3"],
        );
        let running = wait_for(launched.port, 30, |s| s["engine.state"] == "running");
        watch(socket_port(&running), true).ticks
    };
    assert!(whole.len() >= 9_000, "{} ticks", whole.len());

    let data = temp("stopped");
    let launched = launch(
        &data,
        &[
            "--seed",
            "42",
            "--minutes",
            "3",
            "--fast-forward-to",
            "4000",
        ],
    );
    let now = status(launched.port);
    let (code, body) = post(&launched, "/engine/new-match", &default_pair(&now));
    assert_eq!(code, 202, "{body}");
    let running = wait_for(launched.port, 30, |s| s["engine.state"] == "running");
    let _left_running = watch(socket_port(&running), false);
    wait_for(launched.port, 60, |s| {
        s["snapshot.tick"].as_u64().unwrap_or(0) > 0
    });
    let (code, body) = post(&launched, "/engine/stop", "");
    assert_eq!(code, 202, "{body}");
    let stopped: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(stopped["engine.state"], "idle");
    let saved_tick = stopped["saved"]["tick"]
        .as_u64()
        .unwrap_or_else(|| panic!("the stop keeps the save: {stopped}"));
    assert_eq!(stopped["saved"]["kind"], "current");
    assert_eq!(
        stopped["saved"]["teams"],
        serde_json::json!(["Oakmere Rangers", "Eldstead City"])
    );
    assert_eq!(
        stopped["saved"]["positions"]["players"]
            .as_array()
            .map(Vec::len),
        Some(22),
        "{stopped}"
    );
    // The places are measured from the pitch's corner, so the page draws them as they are.
    let pitch = &stopped["saved"]["positions"]["pitch"];
    let (length, width) = (pitch[0].as_f64().unwrap(), pitch[1].as_f64().unwrap());
    for player in stopped["saved"]["positions"]["players"].as_array().unwrap() {
        let (x, y) = (player[1].as_f64().unwrap(), player[2].as_f64().unwrap());
        assert!(
            (-5.0..=length + 5.0).contains(&x) && (-5.0..=width + 5.0).contains(&y),
            "a player stands at {x}, {y} on a {length} by {width} pitch"
        );
    }

    let (code, body) = post(&launched, "/engine/resume", "");
    assert_eq!(code, 202, "{body}");
    let resumed = wait_for(launched.port, 30, |s| s["engine.state"] == "running");
    assert_eq!(resumed["match.resumed_from"].as_u64(), Some(saved_tick));
    let rest = watch(socket_port(&resumed), true).ticks;
    let first = rest.first().expect("the resumed match streams").tick;
    assert!(u64::from(first) <= saved_tick + 1, "resumed at {first}");
    assert_eq!(rest.last().unwrap().tick, whole.last().unwrap().tick);
    for q in &rest {
        let played = whole
            .iter()
            .find(|w| w.tick == q.tick)
            .unwrap_or_else(|| panic!("tick {} was not played through", q.tick));
        assert_eq!(q, played, "tick {} differs after the resume", q.tick);
    }
    let done = wait_for(launched.port, 60, |s| s["engine.state"] == "finished");
    assert!(
        done["saved"].is_null(),
        "a finished match is no save: {done}"
    );
}

/// AC-7, the process half: quit answers, the launcher exits 0 and the worker is gone.
#[test]
fn quit_saves_the_match_and_ends_both_processes() {
    let data = temp("quit");
    let mut launched = launch(
        &data,
        &[
            "--seed",
            "42",
            "--minutes",
            "3",
            "--fast-forward-to",
            "4000",
        ],
    );
    let now = status(launched.port);
    post(&launched, "/engine/new-match", &default_pair(&now));
    let running = wait_for(launched.port, 30, |s| s["engine.state"] == "running");
    let worker = running["engine.pid"].as_u64().unwrap();
    let _left_running = watch(socket_port(&running), false);
    wait_for(launched.port, 60, |s| {
        s["snapshot.tick"].as_u64().unwrap_or(0) > 0
    });
    let (code, body) = post(&launched, "/engine/quit", "");
    assert_eq!(code, 202, "{body}");
    let answer: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(answer["engine.state"], "closed");
    assert_eq!(answer["closed"]["match"], true);
    assert!(answer["closed"]["saved"]["tick"].as_u64().unwrap() > 0);
    let deadline = Instant::now() + Duration::from_secs(5);
    let code = loop {
        if let Some(status) = launched.child.try_wait().unwrap() {
            break status.code();
        }
        assert!(Instant::now() < deadline, "the launcher still runs");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(code, Some(0));
    assert!(!alive(worker), "the worker {worker} still runs");
    let matches = data.join("matches");
    let saved = std::fs::read_dir(&matches)
        .unwrap()
        .filter_map(Result::ok)
        .any(|e| e.path().join(engine::snapshot::FILE_NAME).is_file());
    assert!(saved, "the snapshot stays on disk");
}

/// AC-5, the storage half: the settings hold across a relaunch on the same data folder.
#[test]
fn the_settings_hold_across_launches() {
    let data = temp("settings");
    {
        let launched = launch(&data, &["--seed", "42"]);
        let (code, body) = post(
            &launched,
            "/engine/settings",
            r#"{"speed":4,"motion":"reduce","commentary":false}"#,
        );
        assert_eq!(code, 202, "{body}");
        let (code, body) = post(
            &launched,
            "/engine/settings",
            r#"{"speed":3,"motion":"reduce","commentary":false}"#,
        );
        assert_eq!(code, 400);
        assert!(body.contains("speed 3"), "{body}");
    }
    let launched = launch(&data, &["--seed", "42"]);
    let now = status(launched.port);
    assert_eq!(
        now["settings"],
        serde_json::json!({"schema_version": 1, "speed": 4, "motion": "reduce", "commentary": false})
    );
    drop(launched);
    std::fs::write(data.join("settings.json"), "{ not json").unwrap();
    let launched = launch(&data, &["--seed", "42"]);
    assert_eq!(status(launched.port)["settings"]["speed"], 1);
}
