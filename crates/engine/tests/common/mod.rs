#![allow(dead_code)]

//! Shared fixture: the shipped content, the two default team files, the seed-42
//! configuration built from them, and temp paths.

use std::path::{Path, PathBuf};

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::{Content, ContentDir, MatchConfig, TickHeader};

pub const SEED: u64 = 42;

/// The `content/` folder at the workspace root.
pub fn content_dir() -> ContentDir {
    ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
}

pub fn content() -> Content {
    Content::load(&content_dir()).expect("the shipped content loads")
}

pub fn default_teams(content: &Content) -> [TeamFile; 2] {
    let dir = content_dir();
    let a = content
        .load_team(&dir, &dir.path(TEAM_A_FILE))
        .expect("default-a loads");
    let b = content
        .load_team(&dir, &dir.path(TEAM_B_FILE))
        .expect("default-b loads");
    [a.value, b.value]
}

/// A path inside `tests/fixtures/`.
pub fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The seed-42, 90-minute match on the shipped content and default teams.
pub fn full_match() -> MatchConfig {
    let content = content();
    let [a, b] = default_teams(&content);
    MatchConfig::new(SEED, 90, &content, [&a, &b]).unwrap()
}

/// A seed-42 match of `minutes` on the shipped content and default teams. Shorter than the
/// rule pack's regulation length, it plays no added time.
pub fn short_match(minutes: u32) -> MatchConfig {
    let content = content();
    let [a, b] = default_teams(&content);
    MatchConfig::new(SEED, minutes, &content, [&a, &b]).unwrap()
}

/// A tick-file header for `config` with a fixed test identity.
pub fn header(config: &MatchConfig, ticks: u32) -> TickHeader {
    TickHeader {
        seed: config.seed,
        dt: config.tuning.dt,
        expected_ticks: ticks,
        owner_id: [0x42; 16],
        match_millis: 1_700_000_000_000,
    }
}

pub fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("engine-test-{}-{name}.ticks", std::process::id()))
}

/// A seed-42 match of `minutes` in which no player decides anything, so a law scene moves
/// only as the test arranges it and as the referee restarts it.
pub fn quiet_match(minutes: u32) -> MatchConfig {
    let mut config = short_match(minutes);
    config.tuning.decision_interval_ticks = u32::MAX;
    config
}

/// Roster index of `slot` in `team`: the home side first.
pub fn index(team: usize, slot: usize) -> usize {
    team * 11 + slot
}

/// Places both sides far from a scene: every outfield player of the home side on the line
/// `x = home_x` and of the away side on `x = away_x`, spread across the pitch 6 m apart and
/// never on `y = 0`; the keepers on their goal lines' 50 m marks.
pub fn spread(
    mut scene: engine::scenario::Scene,
    home_x: f64,
    away_x: f64,
) -> engine::scenario::Scene {
    use engine::math::DVec2;
    for team in 0..2 {
        let x = if team == 0 { home_x } else { away_x };
        scene = scene.place(
            index(team, 0),
            DVec2::new(if team == 0 { -50.0 } else { 50.0 }, 0.0),
        );
        for slot in 1..11 {
            let y = -27.0 + 6.0 * (slot - 1) as f64;
            scene = scene.place(index(team, slot), DVec2::new(x, y));
        }
    }
    scene
}
