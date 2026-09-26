//! One version string names a release: the workspace version, what `--version` prints, what
//! the engine's `hello` message carries, and the version in every release file name. The
//! build scripts read it from `--version` and never write it themselves.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use protocol::ServerMessage;
use stream::{Client, Incoming};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The `version` of `[workspace.package]` in the workspace manifest.
fn workspace_version() -> String {
    let manifest = std::fs::read_to_string(repo().join("Cargo.toml")).unwrap();
    let table = manifest
        .split("[workspace.package]")
        .nth(1)
        .expect("the workspace manifest has a package table");
    let table = table.split("\n[").next().unwrap();
    table
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "version").then(|| value.trim().trim_matches('"').to_string())
        })
        .expect("the package table names a version")
}

/// What `engine-cli --version` prints after the program name.
fn printed_version() -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let mut words = text.split_whitespace();
    assert_eq!(words.next(), Some("engine-cli"), "printed {text:?}");
    let version = words.next().expect("a version after the name").to_string();
    assert_eq!(words.next(), None, "printed {text:?}");
    version
}

#[test]
fn the_printed_version_is_the_workspace_version() {
    assert_eq!(printed_version(), workspace_version());
}

#[test]
fn the_hello_message_carries_the_printed_version() {
    let data = std::env::temp_dir().join(format!("engine-cli-version-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    std::fs::create_dir_all(&data).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", &data)
        .env("SM_LOG", "warn")
        .env("SM_CONTENT_DIR", repo().join("content"))
        .args(["serve", "--seed", "3", "--minutes", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("serve prints its port"))
        .read_line(&mut line)
        .unwrap();
    let port: u16 = line.trim().parse().unwrap();
    let mut client = Client::connect_local(port).unwrap();
    let Incoming::Message(message) = client.read().unwrap() else {
        panic!("the first frame is the hello");
    };
    let ServerMessage::Hello(hello) = *message else {
        panic!("the first message is the hello");
    };
    drop(client);
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&data);
    assert_eq!(hello.engine_version, printed_version());
}

#[test]
fn the_build_scripts_read_the_version_and_never_write_it() {
    let version = workspace_version();
    for script in ["packaging/windows/build.ps1", "packaging/unix/build.sh"] {
        let text = std::fs::read_to_string(repo().join(script))
            .unwrap_or_else(|e| panic!("cannot read {script}: {e}"));
        assert!(
            text.contains("--version"),
            "{script} reads the version from the program"
        );
        assert!(
            !text.contains(&version),
            "{script} writes the version {version} itself"
        );
    }
}
