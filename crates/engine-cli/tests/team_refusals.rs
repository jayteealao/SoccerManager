//! Version 2 team files on the command line: `simulate` refuses a home team file with a
//! rating outside 1.0 to 20.0, a rating off the tenth grid, or a missing body field, naming
//! the file, the player and the field on stderr, and plays a valid version 2 file.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> PathBuf {
    repo().join("crates/engine/tests/fixtures/teams").join(name)
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "engine-cli-team-refusals-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the test folder is created");
    dir
}

fn simulate(dir: &Path, team: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_DATA_DIR", dir)
        .env("SM_CONTENT_DIR", repo().join("content"))
        .args(["simulate", "--seed", "1", "--minutes", "1", "--no-snapshot"])
        .arg("--team-a")
        .arg(fixture(team))
        .arg("--ticks-out")
        .arg(dir.join("match.ticks"))
        .output()
        .expect("engine-cli runs")
}

#[test]
fn a_bad_version_2_file_is_refused_by_player_and_field_and_a_good_one_plays() {
    let dir = temp("all");
    for (file, reason) in [
        (
            "v2-out-of-range.json",
            "player p-club-00000001-00-04: attribute pace is 20.5; allowed 1.0 to 20.0",
        ),
        (
            "v2-off-grid.json",
            "player p-club-00000001-00-04: attribute pace is 12.34; not on a tenth",
        ),
        (
            "v2-missing-height.json",
            "player p-club-00000001-00-04: body field height is missing",
        ),
    ] {
        let out = simulate(&dir, file);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{file}: {stderr}");
        let expected = format!("error: content refused: team {file}: players: {reason}");
        assert!(
            stderr.lines().any(|l| l == expected),
            "{file}: no line `{expected}` in\n{stderr}"
        );
    }
    let out = simulate(&dir, "v2-good.json");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir.join("match.ticks").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}
