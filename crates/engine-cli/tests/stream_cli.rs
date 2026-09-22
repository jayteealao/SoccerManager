//! AC-f (stream-protocol) on the command line: `record` writes a fixture and prints its
//! counts, `replay` refuses a missing fixture naming the path, and `serve` prints a port,
//! writes `engine.port`, and streams to a client that connects to it. The last test holds
//! the exit code when a viewer closes its page in mid-match.

use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use protocol::ServerMessage;
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
    // The hello, 3,000 tick frames, the two kick-offs, half-time, full time, and the closing
    // statistics. The hello is stored so a replay forwards the recorded match rather than
    // describing the replaying build.
    assert!(stdout.contains("\"frames\":3006"), "stdout: {stdout}");
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
