//! The background matchday on the real program and socket: every other match of the round
//! plays to full time on the full engine, no ground event reaches the page before the
//! player's match does (through a pause, a speed change, a skip and a dropped connection),
//! a resumed match meets the same round, and a background match that fails shows "result
//! unavailable" with a bug report while everything else plays on.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use protocol::{ClientCommand, GroundEvent, GroundKind, ServerMessage};
use stream::{Client, Incoming};

fn content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn temp(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("engine-cli-matchday-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn bin(data: &Path, content: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_DATA_DIR", data);
    cmd.env("SM_CONTENT_DIR", content);
    cmd.env("SM_LOG", "info");
    cmd
}

/// A running `serve` with its port and the reader of its log.
struct Serving {
    child: Child,
    port: u16,
    logs: std::thread::JoinHandle<String>,
}

fn serve(data: &Path, content: &Path, args: &[&str]) -> Serving {
    let mut child = bin(data, content)
        .arg("serve")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = child.stderr.take().expect("serve logs to stderr");
    let logs = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let mut stdout = BufReader::new(child.stdout.take().expect("serve prints its port"));
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let port = line
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("serve must print a port, printed {line:?}: {e}"));
    Serving { child, port, logs }
}

impl Serving {
    /// Waits for the run to end and returns its exit code and its log.
    fn end(mut self) -> (Option<i32>, String) {
        let status = self.child.wait().unwrap();
        let text = self.logs.join().expect("the log reader does not panic");
        (status.code(), text)
    }
}

/// One thing a page received, in order.
#[derive(Debug, Clone)]
enum Got {
    Tick(u32),
    Hello(String),
    Matchday(protocol::Matchday),
    Ground(GroundEvent),
    Progress(Vec<u32>),
    /// The player's match reached full time.
    FullTime,
    Other,
}

fn read(client: &mut Client) -> Option<Got> {
    match client.read() {
        Ok(Incoming::Tick(_, q)) => Some(Got::Tick(q.tick)),
        Ok(Incoming::Message(message)) => Some(match *message {
            ServerMessage::Hello(hello) => Got::Hello(hello.match_id),
            ServerMessage::Matchday(m) => Got::Matchday(m),
            ServerMessage::GroundEvent(e) => Got::Ground(e),
            ServerMessage::GroundProgress(p) => Got::Progress(p.reached),
            ServerMessage::Event(e) if e.event_type == protocol::EventType::FullTime => {
                Got::FullTime
            }
            ServerMessage::Reject(r) => panic!("the engine refused {}: {}", r.command, r.reason),
            _ => Got::Other,
        }),
        Ok(Incoming::Closed) | Err(_) => None,
    }
}

/// Reads the hello and the matchday message that follows it.
fn open(client: &mut Client) -> (String, protocol::Matchday) {
    let Some(Got::Hello(match_id)) = read(client) else {
        panic!("the first message is the hello");
    };
    let Some(Got::Matchday(matchday)) = read(client) else {
        panic!("the matchday message follows the hello");
    };
    (match_id, matchday)
}

/// Checks the reveal on the engine's side: every ground event comes after a tick frame of
/// the player's match at or past its tick, or after the player's full time, when the other
/// grounds play out their own added time. Returns the ground events.
fn revealed_in_order(reads: &[Got], from_tick: u32) -> Vec<GroundEvent> {
    let mut last = from_tick;
    let mut out = Vec::new();
    for r in reads {
        match r {
            Got::Tick(t) => last = *t,
            Got::FullTime => last = u32::MAX,
            Got::Ground(e) => {
                assert!(
                    e.tick <= last,
                    "a ground event at tick {} came when the match had reached {last}",
                    e.tick
                );
                out.push(e.clone());
            }
            _ => {}
        }
    }
    out
}

/// The `key=value` fields of every log line carrying `signal`.
fn signals(log: &str, signal: &str) -> Vec<BTreeMap<String, String>> {
    let marker = format!("signal=\"{signal}\"");
    log.lines()
        .filter(|l| l.contains(&marker))
        .map(|l| {
            l.split_whitespace()
                .filter_map(|w| w.split_once('='))
                .map(|(k, v)| (k.to_string(), v.trim_matches('"').to_string()))
                .collect()
        })
        .collect()
}

/// Plays `seed` for `minutes` to the end, flat out to `fast_forward`, and returns what the
/// page read and the run's log.
fn played_through(
    name: &str,
    seed: u64,
    minutes: u32,
    extra: &[&str],
) -> (Vec<Got>, String, PathBuf) {
    let data = temp(name);
    let seed = seed.to_string();
    let minutes = minutes.to_string();
    let mut args = vec![
        "--seed",
        &seed,
        "--minutes",
        &minutes,
        "--match-millis",
        "1",
        "--fast-forward-to",
        "400000",
    ];
    args.extend_from_slice(extra);
    let served = serve(&data, &content(), &args);
    let mut client = Client::connect_local(served.port).unwrap();
    let (_, matchday) = open(&mut client);
    let mut reads = vec![Got::Matchday(matchday)];
    client.send(&ClientCommand::Start).unwrap();
    while let Some(r) = read(&mut client) {
        reads.push(r);
    }
    let (code, log) = served.end();
    assert_eq!(code, Some(0), "serve exited with {code:?}: {log}");
    (reads, log, data)
}

fn full_times(events: &[GroundEvent]) -> BTreeMap<u32, [u32; 2]> {
    events
        .iter()
        .filter(|e| e.kind == GroundKind::FullTime)
        .map(|e| (e.fixture, e.score))
        .collect()
}

/// The final score `engine-cli simulate` plays for one fixture's seed and clubs.
fn simulated(data: &Path, seed: &str, minutes: u32, home: &str, away: &str) -> [u32; 2] {
    let teams = content().join("teams");
    let out = bin(data, &content())
        .args([
            "simulate",
            "--seed",
            seed,
            "--minutes",
            &minutes.to_string(),
        ])
        .arg("--team-a")
        .arg(teams.join(format!("{home}.json")))
        .arg("--team-b")
        .arg(teams.join(format!("{away}.json")))
        .arg("--ticks-out")
        .arg(data.join(format!("sim-{seed}.ticks")))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .find(|l| l.contains("\"match-stats\""))
        .expect("simulate prints its statistics record");
    let stats: serde_json::Value = serde_json::from_str(line).unwrap();
    [0, 1].map(|i| stats["goals"][i].as_u64().unwrap() as u32)
}

#[test]
fn every_background_match_completes_on_the_full_engine() {
    let (reads, log, data) = played_through("complete", 42, 10, &[]);
    let Got::Matchday(matchday) = &reads[0] else {
        unreachable!()
    };
    assert_eq!(matchday.round, 1);
    assert_eq!(
        matchday.fixtures.len(),
        4,
        "ten sample clubs give four fixtures"
    );
    let events = revealed_in_order(&reads[1..], 0);
    let finals = full_times(&events);
    assert_eq!(
        finals.len(),
        4,
        "every fixture reaches full time: {events:?}"
    );
    assert!(!events.iter().any(|e| e.kind == GroundKind::Unavailable));
    // How far each ground has played comes every simulated second.
    let progress: Vec<&Vec<u32>> = reads
        .iter()
        .filter_map(|r| match r {
            Got::Progress(reached) => Some(reached),
            _ => None,
        })
        .collect();
    assert!(
        progress.len() >= 500,
        "{} progress messages",
        progress.len()
    );
    assert!(progress.iter().all(|p| p.len() == 4));
    // Each fixture's result is the full engine's: simulate with the fixture's seed and clubs
    // plays the same score.
    let done = signals(&log, "matchday.match_done");
    assert_eq!(done.len(), 4, "{log}");
    for row in &done {
        let fixture: u32 = row["fixture"].parse().unwrap();
        let f = &matchday.fixtures[fixture as usize];
        let score = simulated(&data, &row["seed"], 10, &f.home.id, &f.away.id);
        assert_eq!(finals[&fixture], score, "fixture {fixture}");
        assert_eq!(row["outcome"], "success");
    }
    let summary = signals(&log, "matchday.summary");
    assert_eq!(summary.len(), 1, "{log}");
    assert_eq!(summary[0]["completed"], "4");
    assert_eq!(summary[0]["failed"], "0");
    // The control of the fault test: with no fault, no bug report exists.
    let reports = std::fs::read_dir(data.join("matches"))
        .unwrap()
        .flatten()
        .flat_map(|m| std::fs::read_dir(m.path()).unwrap().flatten())
        .filter(|f| f.file_name().to_string_lossy().starts_with("matchday-bug"))
        .count();
    assert_eq!(reports, 0);
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn the_matchday_message_lists_every_fixture_before_kick_off() {
    let data = temp("before-kick-off");
    let served = serve(&data, &content(), &["--seed", "7", "--minutes", "2"]);
    let mut client = Client::connect_local(served.port).unwrap();
    let (_, matchday) = open(&mut client);
    assert_eq!(matchday.fixtures.len(), 4);
    for (i, f) in matchday.fixtures.iter().enumerate() {
        assert_eq!(f.fixture as usize, i);
        assert!(f.home.id.starts_with("club-000007ea-"), "{}", f.home.id);
        assert!(f.away.id.starts_with("club-000007ea-"), "{}", f.away.id);
        assert!(f.home.roster.is_empty());
    }
    // Nothing about the grounds before kick-off: the first thing after the start is a tick.
    client.send(&ClientCommand::Start).unwrap();
    let first = loop {
        match read(&mut client).expect("the match plays") {
            Got::Other => continue,
            other => break other,
        }
    };
    assert!(matches!(first, Got::Tick(1)), "{first:?}");
    drop(client);
    let _ = served.end();
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn no_ground_event_is_sent_before_the_player_reaches_its_tick() {
    let data = temp("reveal");
    let served = serve(&data, &content(), &["--seed", "42", "--minutes", "10"]);
    let mut client = Client::connect_local(served.port).unwrap();
    let (_, _) = open(&mut client);
    client.send(&ClientCommand::Start).unwrap();
    let mut reads = Vec::new();
    let (mut paused, mut fast, mut slow, mut skipped) = (false, false, false, false);
    while let Some(r) = read(&mut client) {
        if let Got::Tick(t) = r {
            // A pause, held for a moment while the pool keeps playing.
            if !paused && t >= 6_000 {
                paused = true;
                client.send(&ClientCommand::Pause).unwrap();
                std::thread::sleep(Duration::from_millis(300));
                client.send(&ClientCommand::Start).unwrap();
            }
            if !fast && t >= 9_000 {
                fast = true;
                client
                    .send(&ClientCommand::SetSpeed(protocol::SetSpeed { speed: 8.0 }))
                    .unwrap();
            }
            if !slow && t >= 12_000 {
                slow = true;
                client
                    .send(&ClientCommand::SetSpeed(protocol::SetSpeed { speed: 1.0 }))
                    .unwrap();
            }
            if !skipped && t >= 15_000 {
                skipped = true;
                client.send(&ClientCommand::Pause).unwrap();
                client.send(&ClientCommand::Skip).unwrap();
            }
        }
        reads.push(r);
    }
    assert!(paused && fast && slow && skipped);
    let events = revealed_in_order(&reads, 0);
    assert_eq!(
        full_times(&events).len(),
        4,
        "every ground ends after the skip"
    );
    // The planted control: the check fails when an event comes one tick early.
    let mut early = reads.clone();
    let at = early
        .iter()
        .position(|r| matches!(r, Got::Ground(_)))
        .expect("a ground event");
    let Got::Ground(e) = early[at].clone() else {
        unreachable!()
    };
    early.insert(at, Got::Tick(e.tick - 1));
    early.insert(at + 1, Got::Ground(e));
    assert!(std::panic::catch_unwind(|| revealed_in_order(&early, 0)).is_err());
    let (code, log) = served.end();
    assert_eq!(code, Some(0), "{log}");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_reconnecting_page_catches_up_on_every_ground_event_up_to_the_resume_tick() {
    let data = temp("catch-up");
    let served = serve(
        &data,
        &content(),
        &[
            "--seed",
            "25",
            "--minutes",
            "10",
            "--reconnect-wait",
            "30",
            "--drop-client-at",
            "12000",
        ],
    );
    let mut client = Client::connect_local(served.port).unwrap();
    let (match_id, first_round) = open(&mut client);
    client.send(&ClientCommand::Start).unwrap();
    let mut before = Vec::new();
    while let Some(r) = read(&mut client) {
        before.push(r);
    }
    let seen = revealed_in_order(&before, 0);
    let mut client = Client::connect_local(served.port).unwrap();
    let (again, round) = open(&mut client);
    assert_eq!(again, match_id);
    assert_eq!(round, first_round, "the same fixtures after a reconnect");
    // The catch-up comes before the first tick frame of the resumed match.
    let mut caught = Vec::new();
    let resumed_at = loop {
        match read(&mut client).expect("the match goes on") {
            Got::Ground(e) => caught.push(e),
            Got::Tick(t) => break t - 1,
            _ => {}
        }
    };
    assert!(
        resumed_at > 0 && resumed_at < 12_000,
        "resumed at {resumed_at}"
    );
    assert!(caught.iter().all(|e| !e.late && e.tick <= resumed_at));
    let expected: Vec<(u32, u32, GroundKind)> = seen
        .iter()
        .filter(|e| e.tick <= resumed_at)
        .map(|e| (e.tick, e.fixture, e.kind))
        .collect();
    let got: Vec<(u32, u32, GroundKind)> =
        caught.iter().map(|e| (e.tick, e.fixture, e.kind)).collect();
    // Every event the first connection was sent up to the resume tick comes again; a ground
    // that was behind the match then may add events it computed since.
    for e in &expected {
        assert!(
            got.contains(e),
            "{e:?} is missing from the catch-up {got:?}"
        );
    }
    let mut sorted = got.clone();
    sorted.sort_unstable_by_key(|g| (g.0, g.1));
    assert_eq!(got, sorted, "the catch-up is in tick order");
    let mut rest = Vec::new();
    while let Some(r) = read(&mut client) {
        rest.push(r);
    }
    let after = revealed_in_order(&rest, resumed_at);
    assert_eq!(full_times(&after).len(), 4);
    let (code, log) = served.end();
    assert_eq!(code, Some(0), "{log}");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_resumed_match_rebuilds_its_round_from_the_mark() {
    // The reference: the whole match with its round, played through.
    let (reads, _, reference_data) = played_through("resume-reference", 42, 10, &[]);
    let reference: Vec<(u32, u32, GroundKind, [u32; 2])> = revealed_in_order(&reads[1..], 0)
        .iter()
        .map(|e| (e.fixture, e.tick, e.kind, e.score))
        .collect();
    let _ = std::fs::remove_dir_all(&reference_data);

    // A run cut at tick 15 000 leaves a snapshot with the matchday mark.
    let data = temp("resume");
    let served = serve(
        &data,
        &content(),
        &["--seed", "42", "--minutes", "10", "--match-millis", "1"],
    );
    let mut client = Client::connect_local(served.port).unwrap();
    let (match_id, round) = open(&mut client);
    client.send(&ClientCommand::Start).unwrap();
    while let Some(r) = read(&mut client) {
        if matches!(r, Got::Tick(t) if t >= 15_000) {
            break;
        }
    }
    drop(client);
    let (code, _) = served.end();
    assert_eq!(code, Some(2), "the viewer left");
    let snapshot = data.join("matches").join(&match_id).join("snapshot.smsn");
    let saved = engine::Snapshot::read(&snapshot, "snapshot").unwrap();
    let mark = saved
        .matchday
        .clone()
        .expect("the snapshot carries the matchday mark");
    assert_eq!(mark.fixtures.len(), 4);
    assert_eq!(mark.reveal_tick, saved.tick());
    for (f, m) in round.fixtures.iter().zip(&mark.fixtures) {
        assert_eq!([&f.home.id, &f.away.id], [&m.clubs[0], &m.clubs[1]]);
    }

    // The resumed match meets the same round and every ground plays the same match again.
    let resumed = serve(&data, &content(), &["--resume", snapshot.to_str().unwrap()]);
    let mut client = Client::connect_local(resumed.port).unwrap();
    let (again, same) = open(&mut client);
    assert_eq!(again, match_id);
    assert_eq!(same, round);
    let mut rest = Vec::new();
    while let Some(r) = read(&mut client) {
        rest.push(r);
    }
    let events = revealed_in_order(&rest, saved.tick());
    assert!(
        events
            .iter()
            .filter(|e| e.tick <= saved.tick())
            .all(|e| !e.late),
        "nothing up to the resume tick is late"
    );
    let mut got: Vec<(u32, u32, GroundKind, [u32; 2])> = events
        .iter()
        .map(|e| (e.fixture, e.tick, e.kind, e.score))
        .collect();
    got.sort_unstable_by_key(|g| (g.0, g.1));
    let mut expected = reference;
    expected.sort_unstable_by_key(|g| (g.0, g.1));
    assert_eq!(got, expected);
    let (code, log) = resumed.end();
    assert_eq!(code, Some(0), "{log}");
    let _ = std::fs::remove_dir_all(&data);
}

/// A copy of the content folder whose `teams/` holds only the two default clubs.
fn content_with_two_clubs(dir: &Path) -> PathBuf {
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap().flatten() {
            let path = entry.path();
            let target = to.join(entry.file_name());
            if path.is_dir() {
                copy(&path, &target);
            } else {
                let name = entry.file_name().to_string_lossy().into_owned();
                let other_club = path.parent().is_some_and(|p| p.ends_with("teams"))
                    && !name.starts_with("default-");
                if !other_club {
                    std::fs::copy(&path, &target).unwrap();
                }
            }
        }
    }
    let to = dir.join("content");
    copy(&content(), &to);
    to
}

#[test]
fn an_empty_round_sends_no_fixtures() {
    let data = temp("empty-round");
    let two = content_with_two_clubs(&data);
    let served = serve(
        &data,
        &two,
        &[
            "--seed",
            "42",
            "--minutes",
            "1",
            "--fast-forward-to",
            "10000",
        ],
    );
    let mut client = Client::connect_local(served.port).unwrap();
    let (_, matchday) = open(&mut client);
    assert!(matchday.fixtures.is_empty());
    client.send(&ClientCommand::Start).unwrap();
    let mut reads = Vec::new();
    while let Some(r) = read(&mut client) {
        reads.push(r);
    }
    assert!(
        !reads
            .iter()
            .any(|r| matches!(r, Got::Ground(_) | Got::Progress(_))),
        "no ground message with no other match"
    );
    let (code, log) = served.end();
    assert_eq!(code, Some(0), "{log}");
    assert!(signals(&log, "matchday.started").is_empty(), "{log}");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn no_matchday_sends_no_matchday_message() {
    let data = temp("no-matchday");
    let served = serve(
        &data,
        &content(),
        &[
            "--seed",
            "42",
            "--minutes",
            "1",
            "--fast-forward-to",
            "10000",
            "--no-matchday",
        ],
    );
    let mut client = Client::connect_local(served.port).unwrap();
    let Some(Got::Hello(_)) = read(&mut client) else {
        panic!("the first message is the hello");
    };
    client.send(&ClientCommand::Start).unwrap();
    let mut reads = Vec::new();
    while let Some(r) = read(&mut client) {
        reads.push(r);
    }
    assert!(
        !reads
            .iter()
            .any(|r| matches!(r, Got::Matchday(_) | Got::Ground(_) | Got::Progress(_)))
    );
    let (code, _) = served.end();
    assert_eq!(code, Some(0));
    let _ = std::fs::remove_dir_all(&data);
}

/// AC-37: a fault in one background match makes it unavailable with a bug report; the
/// player's match and the other grounds play on to full time.
#[test]
fn a_failed_background_match_shows_unavailable_and_the_rest_play_on() {
    let (reads, log, data) = played_through("fault", 42, 90, &["--matchday-fault", "1@102000"]);
    let ticks: Vec<u32> = reads
        .iter()
        .filter_map(|r| match r {
            Got::Tick(t) => Some(*t),
            _ => None,
        })
        .collect();
    assert!(
        *ticks.last().unwrap() >= 270_000,
        "the player's match reached full time"
    );
    let events = revealed_in_order(&reads[1..], 0);
    let unavailable: Vec<&GroundEvent> = events
        .iter()
        .filter(|e| e.kind == GroundKind::Unavailable)
        .collect();
    assert_eq!(unavailable.len(), 1, "{events:?}");
    assert_eq!(
        (
            unavailable[0].fixture,
            unavailable[0].tick,
            unavailable[0].minute
        ),
        (1, 102_000, 34)
    );
    let finals = full_times(&events);
    assert_eq!(
        finals.keys().copied().collect::<Vec<_>>(),
        vec![0, 2, 3],
        "the other fixtures reach full time"
    );
    let failed = signals(&log, "matchday.match_failed");
    assert_eq!(failed.len(), 1, "{log}");
    assert_eq!(failed[0]["error.type"], "panic");
    let match_dir = std::fs::read_dir(data.join("matches"))
        .unwrap()
        .flatten()
        .next()
        .unwrap()
        .path();
    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(match_dir.join("matchday-bug-1.json"))
            .expect("the bug report of fixture 1"),
    )
    .unwrap();
    assert_eq!(report["seed"].to_string(), failed[0]["seed"]);
    assert_eq!(report["engine.version"], engine::version());
    assert_eq!(report["build.hash"], engine::build_hash());
    assert_eq!(report["tick"], 102_000);
    assert_eq!(report["fixture"], 1);
    let summary = signals(&log, "matchday.summary");
    assert_eq!(summary[0]["completed"], "3");
    assert_eq!(summary[0]["failed"], "1");
    let _ = std::fs::remove_dir_all(&data);
}
