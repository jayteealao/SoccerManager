//! `engine-cli restart-waits` writes the restart wait table from the real dead spells less the
//! engine's measured visible steps, and reads no ball-in-play figure: a real-data folder whose
//! ball-in-play column and file say something else gives the same table, while a changed
//! spell length gives another. The table it writes loads as content.

mod common;

use std::path::{Path, PathBuf};

use common::{bin, temp};

/// The synthetic real-data folder: 42 dead spells of every kind, a `ball_in_play` column
/// beside the four the command reads, and an `idsse_ball.csv` of ball-in-play minutes.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/restart-waits")
}

/// Runs the command on `real` and returns the table it wrote.
fn table(data: &Path, real: &Path, name: &str) -> String {
    let out = data.join(format!("{name}.json"));
    let run = bin(data)
        .args(["restart-waits", "--seeds", "1", "--minutes", "10"])
        .arg("--real-data")
        .arg(real)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let summary: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&run.stdout).trim()).unwrap();
    assert_eq!(summary["kind"], "restart-waits");
    assert_eq!(summary["spells"], 42);
    std::fs::read_to_string(out).unwrap()
}

/// A copy of the fixture folder in `to`, with each line of the dead spells passed through
/// `line` and the ball-in-play file replaced by `ball`.
fn variant(to: &Path, line: impl Fn(usize, &str) -> String, ball: &str) -> PathBuf {
    std::fs::create_dir_all(to).unwrap();
    let text = std::fs::read_to_string(fixture().join("idsse_dead.csv")).unwrap();
    let lines: Vec<String> = text.lines().enumerate().map(|(n, l)| line(n, l)).collect();
    std::fs::write(to.join("idsse_dead.csv"), lines.join("\n") + "\n").unwrap();
    std::fs::write(to.join("idsse_ball.csv"), ball).unwrap();
    to.to_path_buf()
}

#[test]
fn the_table_follows_the_waits_and_never_the_ball_in_play() {
    let data = temp("engine-cli-restart-waits", "table");
    let first = table(&data, &fixture(), "first");
    let loaded = engine::data::restart_waits::load_restart_waits_bytes(
        first.as_bytes(),
        "restart-waits.json",
    )
    .expect("the written table loads as content");
    assert!(
        loaded
            .value
            .source
            .iter()
            .any(|s| s.contains("idsse_dead.csv")),
        "{:?}",
        loaded.value.source
    );

    // Other ball-in-play figures everywhere: the same table.
    let other_ball = variant(
        &data.join("other-ball"),
        |n, l| {
            if n == 0 {
                return l.to_string();
            }
            let mut cells: Vec<String> = l.split(',').map(str::to_string).collect();
            cells[1] = format!("{}", 90 - n);
            cells.join(",")
        },
        "mid,ball_in_play_min\nS0,89.9\nS1,12.0\n",
    );
    assert_eq!(table(&data, &other_ball, "other-ball"), first);

    // The control: one spell's length changed gives another table.
    let other_wait = variant(
        &data.join("other-wait"),
        |n, l| {
            if n != 1 {
                return l.to_string();
            }
            let mut cells: Vec<String> = l.split(',').map(str::to_string).collect();
            cells[3] = "95.0".into();
            cells.join(",")
        },
        "mid,ball_in_play_min\nS0,55.1\nS1,58.4\n",
    );
    assert_ne!(table(&data, &other_wait, "other-wait"), first);
    std::fs::remove_dir_all(&data).unwrap();
}

#[test]
fn the_command_source_names_no_ball_in_play_input() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/restart_waits.rs");
    let text = std::fs::read_to_string(path).unwrap();
    // The unit tests below the module plant a ball-in-play column to prove it is not read.
    let code = text.split("#[cfg(test)]").next().unwrap().to_lowercase();
    for name in ["ball_in_play", "idsse_ball", "bip"] {
        assert!(!code.contains(name), "the command names {name}");
    }
    // The guard reads the planted name in the tests, so a scan of the whole file would fail.
    assert!(text.to_lowercase().contains("ball_in_play"));
}
