//! The home ground's size on the command line: `simulate` refuses a home team file whose
//! ground is outside the Laws, naming the file, the club's ground, the value and the limit,
//! and plays one at each edge the Laws allow; `serve` names a ground that is not 105 by 68
//! in its hello and leaves the default out.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use protocol::ServerMessage;
use stream::{Client, Incoming};

fn content_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("engine-cli-ground-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the test folder is created");
    dir
}

fn bin(data_dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_DATA_DIR", data_dir);
    cmd.env("SM_CONTENT_DIR", content_dir());
    cmd
}

/// A copy of the shipped home team file in `dir` with its club's ground set, and the club's
/// name.
fn home_on(dir: &Path, length: f64, width: f64) -> (PathBuf, String) {
    let text = std::fs::read_to_string(content_dir().join("teams/default-a.json")).unwrap();
    let mut doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    doc["club"]["ground"] = serde_json::json!({ "length": length, "width": width });
    let path = dir.join(format!("home-{length}x{width}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(&doc).unwrap()).unwrap();
    (path, doc["club"]["name"].as_str().unwrap().to_string())
}

#[test]
fn simulate_refuses_a_ground_outside_the_laws_and_plays_one_at_each_edge() {
    let dir = temp("laws");
    let cases = [
        (
            89.0,
            68.0,
            Some("the ground is 89 m long; the Laws allow 90 to 120 m"),
        ),
        (90.0, 68.0, None),
        (120.0, 68.0, None),
        (
            121.0,
            68.0,
            Some("the ground is 121 m long; the Laws allow 90 to 120 m"),
        ),
        (
            105.0,
            44.0,
            Some("the ground is 44 m wide; the Laws allow 45 to 90 m"),
        ),
        (105.0, 45.0, None),
        (105.0, 90.0, None),
        (
            105.0,
            91.0,
            Some("the ground is 91 m wide; the Laws allow 45 to 90 m"),
        ),
    ];
    for (length, width, refusal) in cases {
        let (team, club) = home_on(&dir, length, width);
        let ticks = dir.join("match.ticks");
        let out = bin(&dir)
            .args([
                "simulate",
                "--seed",
                "42",
                "--minutes",
                "2",
                "--no-snapshot",
                "--team-a",
            ])
            .arg(&team)
            .arg("--ticks-out")
            .arg(&ticks)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        let file = team.file_name().unwrap().to_string_lossy().to_string();
        println!(
            "simulate --team-a {file}: exit {:?}\n{}",
            out.status.code(),
            stderr.trim()
        );
        match refusal {
            Some(reason) => {
                assert_ne!(
                    out.status.code(),
                    Some(0),
                    "{length} by {width} must be refused"
                );
                for part in [file.as_str(), "club.ground", club.as_str(), reason] {
                    assert!(
                        stderr.contains(part),
                        "{length} by {width}: {part:?} not in {stderr}"
                    );
                }
            }
            None => {
                assert_eq!(out.status.code(), Some(0), "{length} by {width}: {stderr}");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The hello `serve` sends with `team_a` at home, as parsed and as raw JSON.
fn served_hello(dir: &Path, team_a: Option<&Path>) -> protocol::Hello {
    let mut cmd = bin(dir);
    cmd.args(["serve", "--seed", "42", "--minutes", "1"]);
    if let Some(team) = team_a {
        cmd.arg("--team-a").arg(team);
    }
    let mut child = cmd
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
    let _ = child.kill();
    let _ = child.wait();
    *hello
}

#[test]
fn serve_names_the_home_ground_in_its_hello() {
    let dir = temp("hello");
    let (team, _) = home_on(&dir, 100.0, 64.0);
    let hello = served_hello(&dir, Some(&team));
    assert_eq!((hello.ground_length, hello.ground_width), (100.0, 64.0));
    let json = serde_json::to_string(&ServerMessage::Hello(Box::new(hello))).unwrap();
    assert!(
        json.contains("\"ground.length\":100.0,\"ground.width\":64.0"),
        "{json}"
    );

    let plain = served_hello(&dir, None);
    assert_eq!((plain.ground_length, plain.ground_width), (105.0, 68.0));
    let json = serde_json::to_string(&ServerMessage::Hello(Box::new(plain))).unwrap();
    assert!(!json.contains("ground."), "{json}");
    let _ = std::fs::remove_dir_all(&dir);
}
