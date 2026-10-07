//! Checks the streaming commands on the command line: `record` writes a fixture and prints
//! its counts, `replay` refuses a missing fixture naming the path, and `serve` prints a
//! port, writes `engine.port`, and streams to a client that connects to it. The last test
//! holds the exit code when a viewer closes its page in mid-match.

use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use protocol::{ClientCommand, ServerMessage};
use stream::{Client, Incoming};

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("engine-cli-stream-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the test folder is created");
    dir
}

fn bin(data_dir: &PathBuf) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_DATA_DIR", data_dir);
    cmd.env(
        "SM_CONTENT_DIR",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
    );
    cmd
}

#[test]
fn record_writes_a_fixture_and_prints_its_counts() {
    let dir = temp("record");
    let fixture = dir.join("match.smfx");
    let out = bin(&dir)
        .args(["record", "--seed", "7", "--minutes", "1", "--out"])
        .arg(&fixture)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"ticks\":3000"), "stdout: {stdout}");
    // The hello, 3,000 tick frames, five events (the two kick-offs, half-time, full time,
    // and this seed's throw-in), 60 running statistics and 60 condition messages
    // (one of each every simulated second), and the closing statistics. The hello is stored
    // so a replay forwards the recorded match rather than describing the replaying build.
    assert!(stdout.contains("\"frames\":3127"), "stdout: {stdout}");
    assert!(stdout.contains("\"hash\":\""), "stdout: {stdout}");

    let read = stream::read_fixture(&fixture).unwrap();
    assert_eq!(read.seed, 7);
    assert_eq!(read.ticks, 3_000);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn replay_refuses_a_missing_fixture_naming_the_path() {
    let dir = temp("replay-missing");
    let missing = dir.join("absent.smfx");
    let out = bin(&dir)
        .args(["replay", "--fixture"])
        .arg(&missing)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot replay") && stderr.contains(&missing.display().to_string()),
        "stderr: {stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn serve_prints_a_port_writes_the_port_file_and_streams_a_match() {
    let dir = temp("serve");
    let mut child = bin(&dir)
        .args(["serve", "--seed", "42", "--minutes", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().expect("serve prints its port"));
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let port: u16 = line.trim().parse().unwrap_or_else(|e| {
        panic!("serve must print a port, printed {line:?}: {e}");
    });
    let written = std::fs::read_to_string(dir.join("engine.port")).unwrap();
    assert_eq!(written.trim().parse::<u16>().unwrap(), port);

    let mut client = Client::connect_local(port).unwrap();
    let Incoming::Message(message) = client.read().unwrap() else {
        panic!("the first frame is the hello");
    };
    let ServerMessage::Hello(hello) = *message else {
        panic!("the first message is the hello");
    };
    assert_eq!(hello.ticks_expected, 3_000);
    // A served match holds before kick-off until the page starts it.
    client.send(&ClientCommand::Start).unwrap();
    let mut ticks = 0u32;
    loop {
        match client.read().unwrap() {
            Incoming::Tick(_, _) => ticks += 1,
            Incoming::Message(_) => continue,
            Incoming::Closed => break,
        }
    }
    assert_eq!(ticks, 3_000);
    client.close().unwrap();

    let status = child.wait().unwrap();
    assert!(status.success(), "serve exited with {status}");
    // The port file is removed on a clean exit, so no page reaches an engine that is gone.
    assert!(!dir.join("engine.port").exists());
    // A served match leaves the same records as a simulated one: the event rows and, at full
    // time, the statistics record.
    let folder = dir.join("matches").join(&hello.match_id);
    assert!(
        folder.join("events.jsonl").exists(),
        "no events.jsonl in {}",
        folder.display()
    );
    let stats: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(folder.join("stats.json"))
            .expect("serve writes stats.json at full time"),
    )
    .unwrap();
    assert_eq!(stats["record.kind"], "match-stats");
    assert_eq!(stats["match.id"], hello.match_id.as_str());
    assert_eq!(stats["outcome"], "success");
    assert_eq!(stats["ticks.written"], 3_000);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_dropped_viewer_reconnects_on_the_same_port_and_the_match_goes_on() {
    let dir = temp("serve-reconnect");
    let mut child = bin(&dir)
        .args([
            "serve",
            "--seed",
            "39",
            "--minutes",
            "2",
            "--reconnect-wait",
            "30",
            "--drop-client-at",
            "3000",
        ])
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
    let port: u16 = line.trim().parse().unwrap();

    let mut client = Client::connect_local(port).unwrap();
    let Incoming::Message(message) = client.read().unwrap() else {
        panic!("the first frame is the hello");
    };
    let ServerMessage::Hello(first) = *message else {
        panic!("the first message is the hello");
    };
    client.send(&ClientCommand::Start).unwrap();
    let mut last_tick = 0u32;
    loop {
        match client.read() {
            Ok(Incoming::Tick(_, q)) => last_tick = q.tick,
            Ok(Incoming::Message(_)) => continue,
            Ok(Incoming::Closed) | Err(_) => break,
        }
    }
    assert!(
        last_tick >= 3_000,
        "the drop comes at tick 3,000; read {last_tick}"
    );

    // The same port, the same hello, and play from a stoppage the viewer already holds.
    let mut client = Client::connect_local(port).unwrap();
    let Incoming::Message(message) = client.read().unwrap() else {
        panic!("the first frame after a reconnect is the hello");
    };
    let ServerMessage::Hello(again) = *message else {
        panic!("the first message after a reconnect is the hello");
    };
    assert_eq!(again.match_id, first.match_id);
    let mut first_tick = None;
    let mut ticks = 0u32;
    loop {
        match client.read().unwrap() {
            Incoming::Tick(frame, q) => {
                if first_tick.is_none() {
                    assert_ne!(frame.kind(), protocol::frame::KIND_DELTA);
                    first_tick = Some(q.tick);
                }
                ticks += 1;
            }
            Incoming::Message(_) => continue,
            Incoming::Closed => break,
        }
    }
    let first_tick = first_tick.expect("the match goes on after the reconnect");
    // Seed 39 stops play at ticks 2,280 and 3,000 (half-time of a two-minute match), so the
    // match goes back to a stoppage the viewer held, never to kick-off and never past it.
    assert!(
        first_tick > 2_000 && first_tick <= last_tick + 1,
        "resumed at {first_tick}; the viewer held up to {last_tick}"
    );
    assert!(ticks > 0);
    client.close().unwrap();

    let status = child.wait().unwrap();
    let text = logs.join().expect("the log reader does not panic");
    assert!(status.success(), "serve exited with {status}: {text}");
    assert!(text.contains("signal=\"socket.reconnected\""), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_drop_before_the_first_stoppage_goes_back_to_kick_off() {
    let dir = temp("serve-reconnect-early");
    let mut child = bin(&dir)
        .args([
            "serve",
            "--seed",
            "42",
            "--minutes",
            "2",
            "--reconnect-wait",
            "30",
            "--drop-client-at",
            "1000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().expect("serve prints its port"));
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let port: u16 = line.trim().parse().unwrap();

    let mut client = Client::connect_local(port).unwrap();
    client.send(&ClientCommand::Start).unwrap();
    while let Ok(incoming) = client.read() {
        if matches!(incoming, Incoming::Closed) {
            break;
        }
    }
    let mut client = Client::connect_local(port).unwrap();
    let first = loop {
        match client.read().unwrap() {
            Incoming::Tick(_, q) => break q.tick,
            Incoming::Message(_) => continue,
            Incoming::Closed => panic!("the match closed before it resumed"),
        }
    };
    // The capture taken at kick-off, with the same seed and lineup, is the restart point.
    assert_eq!(first, 1);
    drop(client);
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_help_line_of_the_streaming_commands_exceeds_eighty_columns() {
    let dir = temp("help");
    for args in [
        vec!["serve", "--help"],
        vec!["record", "--help"],
        vec!["replay", "--help"],
        vec!["bench", "--help"],
    ] {
        let out = bin(&dir).args(&args).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            assert!(line.len() <= 80, "{args:?}: {} columns: {line}", line.len());
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_viewer_that_closes_in_mid_match_ends_the_run_without_an_error() {
    let dir = temp("serve-viewer-gone");
    let mut child = bin(&dir)
        .args(["serve", "--seed", "42", "--minutes", "90"])
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
    let port: u16 = line.trim().parse().unwrap();

    let mut client = Client::connect_local(port).unwrap();
    client.send(&ClientCommand::Start).unwrap();
    let mut ticks = 0u32;
    while ticks < 200 {
        match client.read().unwrap() {
            Incoming::Tick(_, _) => ticks += 1,
            Incoming::Message(_) => continue,
            Incoming::Closed => break,
        }
    }
    // A closed page drops the socket; it sends no close frame.
    drop(client);

    let status = child.wait().unwrap();
    let text = logs.join().expect("the log reader does not panic");
    // 2 is the short-run code. 1 would say the run failed, and a viewer that closes its
    // page has not failed anything.
    assert_eq!(status.code(), Some(2), "serve exited with {status}: {text}");
    assert!(
        text.contains("signal=\"socket.client_gone\""),
        "the run must name the viewer that went away: {text}"
    );
    assert!(!dir.join("engine.port").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// What one served match sent and wrote: every binary tick frame in order, the `--ticks-out`
/// file, and the rows of its events file.
struct Served {
    frames: Vec<Vec<u8>>,
    ticks_file: Vec<u8>,
    rows: Vec<String>,
    /// The tick the page skipped or jumped at; `None` when it watched to the end.
    skipped_at: Option<u32>,
}

/// How the page plays a served match.
#[derive(Debug, Clone, Copy)]
enum Pace {
    /// It kicks off and watches to the end.
    Watch,
    /// Once a tick frame at or past `from` arrives it pauses, skips, and reads the rest.
    Skip { from: u32 },
    /// Once a tick frame at or past `from` arrives it pauses, jumps to `to`, and stops
    /// reporting drawn ticks. When the frame at `to` arrives, still paused, it reports it,
    /// starts again and watches to the end.
    Jump { from: u32, to: u32 },
}

impl Pace {
    fn start_tick(self) -> Option<u32> {
        match self {
            Pace::Watch => None,
            Pace::Skip { from } | Pace::Jump { from, .. } => Some(from),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Pace::Watch => "watch",
            Pace::Skip { .. } => "skip",
            Pace::Jump { .. } => "jump",
        }
    }
}

/// Serves `seed` for ten minutes in `dir`, with `extra` arguments, and plays the page's part
/// over the real socket as `pace` says, reading to the close.
fn serve_and_read(dir: &std::path::Path, seed: u64, pace: Pace, extra: &[&str]) -> Served {
    let ticks_out = dir.join(format!("served-{seed}-{}.ticks", pace.label()));
    let mut child = bin(&dir.to_path_buf())
        .args([
            "serve",
            "--seed",
            &seed.to_string(),
            "--minutes",
            "10",
            "--match-millis",
            "1",
        ])
        .args(extra)
        .arg("--ticks-out")
        .arg(&ticks_out)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().expect("serve prints its port"));
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let port: u16 = line.trim().parse().unwrap();
    let mut client = Client::connect_local(port).unwrap();
    let Incoming::Message(message) = client.read().unwrap() else {
        panic!("the first frame is the hello");
    };
    let ServerMessage::Hello(hello) = *message else {
        panic!("the first message is the hello");
    };
    client.send(&ClientCommand::Start).unwrap();
    let mut frames = Vec::new();
    let mut skipped_at = None;
    let mut read_at = None;
    // `true` from the jump until the page starts again: it reports no drawn tick then.
    let mut held = false;
    let mut resumed_at = None;
    loop {
        match client.read().unwrap() {
            Incoming::Tick(frame, q) => {
                frames.push(frame.as_bytes().to_vec());
                if let Pace::Jump { to, .. } = pace
                    && held
                    && q.tick >= to
                {
                    // Every frame up to `to` came while the page was paused and reported
                    // nothing new: the engine produced them without its pacing.
                    held = false;
                    resumed_at = Some(q.tick);
                    client
                        .send(&ClientCommand::Seen(protocol::Seen { tick: q.tick }))
                        .unwrap();
                    client.send(&ClientCommand::Start).unwrap();
                }
                // Like a page, report the drawn tick, so the engine stays within its lead
                // bound and a pause holds it.
                let reporting = match pace {
                    Pace::Skip { .. } => skipped_at.is_none(),
                    _ => !held,
                };
                if reporting && q.tick.is_multiple_of(50) {
                    client
                        .send(&ClientCommand::Seen(protocol::Seen { tick: q.tick }))
                        .unwrap();
                }
                if skipped_at.is_none() && pace.start_tick().is_some_and(|from| q.tick >= from) {
                    client.send(&ClientCommand::Pause).unwrap();
                    match pace {
                        Pace::Skip { .. } => client.send(&ClientCommand::Skip).unwrap(),
                        Pace::Jump { to, .. } => {
                            client
                                .send(&ClientCommand::Jump(protocol::Jump { tick: to }))
                                .unwrap();
                            held = true;
                        }
                        Pace::Watch => unreachable!("a watched match has no from"),
                    }
                    skipped_at = Some(q.tick);
                }
            }
            Incoming::Message(message) => match *message {
                ServerMessage::Reject(reject) => {
                    panic!("the engine refused {}: {}", reject.command, reject.reason)
                }
                ServerMessage::Ack(ack) if ack.command == "skip" || ack.command == "jump" => {
                    read_at = Some(ack.queued_tick);
                }
                _ => {}
            },
            Incoming::Closed => break,
        }
    }
    match pace {
        Pace::Watch => {}
        Pace::Skip { .. } => {
            // The skip reached the engine mid-match, held by the pause and the lead bound.
            let read_at = read_at.expect("the skip was acknowledged");
            assert!(read_at < 12_000, "the skip was read at tick {read_at}");
        }
        Pace::Jump { from, to } => {
            // The jump reached the engine mid-match, held by the pause and the lead bound,
            // and the paused page then received every frame up to its tick.
            let read_at = read_at.expect("the jump was acknowledged");
            assert!(
                read_at >= from && read_at < from + 1_000,
                "the jump was read at tick {read_at}"
            );
            assert_eq!(resumed_at, Some(to), "the paused page received tick {to}");
        }
    }
    client.close().unwrap();
    let status = child.wait().unwrap();
    assert!(status.success(), "serve exited with {status}");
    let folder = dir.join("matches").join(&hello.match_id);
    let rows = std::fs::read_to_string(folder.join("events.jsonl"))
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    // The next run of the same seed writes the same match folder.
    std::fs::remove_dir_all(&folder).unwrap();
    Served {
        frames,
        ticks_file: std::fs::read(&ticks_out).unwrap(),
        rows,
        skipped_at,
    }
}

/// Holds `left` and `right` to the same tick frames, `--ticks-out` file and events rows.
fn assert_same_match(left: &Served, right: &Served, what: &str) {
    assert_eq!(
        left.frames.len(),
        30_000,
        "the {what} run reached full time"
    );
    assert_eq!(
        left.frames.len(),
        right.frames.len(),
        "the same number of tick frames"
    );
    if let Some(i) = (0..left.frames.len()).find(|&i| left.frames[i] != right.frames[i]) {
        panic!("tick frame {i} differs between the {what} and the watched run");
    }
    assert!(
        left.ticks_file == right.ticks_file,
        "the --ticks-out files differ"
    );
    assert!(!left.rows.is_empty());
    assert_eq!(left.rows, right.rows, "the events files differ");
}

#[test]
fn a_skipped_served_match_streams_the_same_ticks_and_events_as_one_played_through() {
    let dir = temp("serve-skip");
    let skipped = serve_and_read(&dir, 42, Pace::Skip { from: 9_000 }, &[]);
    let watched = serve_and_read(&dir, 42, Pace::Watch, &[]);
    let at = skipped.skipped_at.expect("the page skipped");
    assert!(at >= 9_000, "skipped at {at}");
    assert_same_match(&skipped, &watched, "skipped");

    // The control: another seed's match differs, so the comparison can fail.
    let other = serve_and_read(&dir, 43, Pace::Watch, &[]);
    assert_ne!(
        other.frames, watched.frames,
        "seed 43 must stream another match"
    );
    assert_ne!(other.ticks_file, watched.ticks_file);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_jumped_served_match_streams_the_same_ticks_and_events_as_one_played_through() {
    let dir = temp("serve-jump");
    let jump = Pace::Jump {
        from: 3_000,
        to: 20_000,
    };
    let jumped = serve_and_read(&dir, 42, jump, &["--test-jump"]);
    let watched = serve_and_read(&dir, 42, Pace::Watch, &["--test-jump"]);
    let at = jumped.skipped_at.expect("the page jumped");
    assert!(at >= 3_000, "jumped at {at}");
    assert_same_match(&jumped, &watched, "jumped");

    // The control: another seed's match differs, so the comparison can fail.
    let other = serve_and_read(&dir, 43, Pace::Watch, &[]);
    assert_ne!(
        other.frames, watched.frames,
        "seed 43 must stream another match"
    );
    assert_ne!(other.ticks_file, watched.ticks_file);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Starts `serve` for `minutes` in `dir` with `extra` arguments; returns the child and a
/// client that has read the hello.
fn serve_with(dir: &PathBuf, minutes: &str, extra: &[&str]) -> (std::process::Child, Client) {
    let mut child = bin(dir)
        .args([
            "serve",
            "--seed",
            "42",
            "--minutes",
            minutes,
            "--no-matchday",
        ])
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().expect("serve prints its port"));
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let port: u16 = line.trim().parse().unwrap();
    let mut client = Client::connect_local(port).unwrap();
    let Incoming::Message(message) = client.read().unwrap() else {
        panic!("the first frame is the hello");
    };
    assert!(matches!(*message, ServerMessage::Hello(_)));
    (child, client)
}

#[test]
fn a_jump_without_the_test_flag_is_refused_by_a_served_match() {
    let dir = temp("serve-jump-refused");
    let (mut child, mut client) = serve_with(&dir, "10", &[]);
    client.send(&ClientCommand::Start).unwrap();
    let mut last_seen = 0u32;
    // Read and report drawn ticks up to 3,000, then pause and ask for a jump.
    loop {
        let Incoming::Tick(_, q) = client.read().unwrap() else {
            continue;
        };
        if q.tick.is_multiple_of(50) {
            last_seen = q.tick;
            client
                .send(&ClientCommand::Seen(protocol::Seen { tick: q.tick }))
                .unwrap();
        }
        if q.tick >= 3_000 {
            break;
        }
    }
    client.send(&ClientCommand::Pause).unwrap();
    client
        .send(&ClientCommand::Jump(protocol::Jump { tick: 20_000 }))
        .unwrap();
    let reject = loop {
        if let Incoming::Message(message) = client.read().unwrap() {
            match *message {
                ServerMessage::Reject(reject) => break reject,
                ServerMessage::Ack(ack) if ack.command == "jump" => {
                    // End the run before failing, so a broken guard fails instead of hanging.
                    let _ = child.kill();
                    panic!("an engine without --test-jump accepted the jump");
                }
                _ => {}
            }
        }
    };
    assert_eq!(reject.command, "jump");
    assert!(
        reject.reason.contains("--test-jump"),
        "the refusal names the flag: {}",
        reject.reason
    );
    // Paused and refused, the engine holds: one second later it has produced no tick past
    // the lead bound of the newest reported tick. The start's ack names the tick it holds at.
    std::thread::sleep(std::time::Duration::from_secs(1));
    client.send(&ClientCommand::Start).unwrap();
    let held_at = loop {
        if let Incoming::Message(message) = client.read().unwrap()
            && let ServerMessage::Ack(ack) = *message
            && ack.command == "start"
        {
            break ack.queued_tick;
        }
    };
    assert!(
        held_at <= last_seen + 500,
        "held at {held_at}; the newest reported tick was {last_seen}"
    );
    // End the match quickly and cleanly.
    client.send(&ClientCommand::Skip).unwrap();
    while !matches!(client.read().unwrap(), Incoming::Closed) {}
    client.close().unwrap();
    assert!(child.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_jump_past_full_time_ends_the_match_at_full_time() {
    let dir = temp("serve-jump-past");
    let (mut child, mut client) = serve_with(&dir, "1", &["--test-jump"]);
    client.send(&ClientCommand::Start).unwrap();
    let mut ticks = 0u32;
    let mut jumped = false;
    loop {
        match client.read().unwrap() {
            Incoming::Tick(_, q) => {
                ticks += 1;
                // Report drawn ticks only before the jump: after it the paused page reports
                // nothing, so only the jump can carry the match to its end.
                if !jumped && q.tick.is_multiple_of(50) {
                    client
                        .send(&ClientCommand::Seen(protocol::Seen { tick: q.tick }))
                        .unwrap();
                }
                if !jumped && q.tick >= 100 {
                    client.send(&ClientCommand::Pause).unwrap();
                    client
                        .send(&ClientCommand::Jump(protocol::Jump { tick: 1_000_000 }))
                        .unwrap();
                    jumped = true;
                }
            }
            Incoming::Message(message) => {
                if let ServerMessage::Reject(reject) = *message {
                    panic!("the engine refused {}: {}", reject.command, reject.reason);
                }
            }
            Incoming::Closed => break,
        }
    }
    assert_eq!(ticks, 3_000, "the match ended at full time");
    client.close().unwrap();
    let status = child.wait().unwrap();
    assert!(status.success(), "serve exited with {status}");
    let _ = std::fs::remove_dir_all(&dir);
}
