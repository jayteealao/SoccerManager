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
    // The hello, 3,000 tick frames, seven events (the two kick-offs, half-time, full time,
    // and this seed's foul, card, and corner), 60 running statistics and 60 condition messages
    // (one of each every simulated second), and the closing statistics. The hello is stored
    // so a replay forwards the recorded match rather than describing the replaying build.
    assert!(stdout.contains("\"frames\":3129"), "stdout: {stdout}");
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
            "18",
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
    // Seed 18 stops play at ticks 2,248 and 3,000 (half-time of a two-minute match), so the
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
