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

/// A quiet match with no background or tackle injuries, so a scene stops only where the
/// test arranges it.
pub fn calm_match(minutes: u32) -> MatchConfig {
    let mut config = quiet_match(minutes);
    config.tuning.injury_per_minute = 0.0;
    config.tuning.injury_per_tackle = 0.0;
    config
}

/// Every event kind in `events`, in order.
pub fn kinds(events: &[engine::EngineEvent]) -> Vec<engine::EngineEventKind> {
    events.iter().map(|e| e.kind).collect()
}

/// A copy of `file` with every attribute times 1.15, rounded and clamped to 100, under a new
/// club id.
pub fn stronger(file: &engine::data::TeamFile) -> engine::data::TeamFile {
    let mut out = file.clone();
    out.club.id = format!("{}-strong", file.club.id);
    out.club.name = format!("{} Strong", file.club.name);
    for p in &mut out.players {
        for v in p.attributes.values_mut() {
            *v = (f64::from(*v) * 1.15).round().min(100.0) as u8;
        }
    }
    out
}

/// Runs `job` for every seed in `seeds` on all cores and returns the results in seed order.
pub fn run_many<T: Send>(
    seeds: std::ops::RangeInclusive<u64>,
    job: impl Fn(u64) -> T + Sync,
) -> Vec<T> {
    let seeds: Vec<u64> = seeds.collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = seeds.len().div_ceil(threads).max(1);
    let job = &job;
    std::thread::scope(|scope| {
        let handles: Vec<_> = seeds
            .chunks(chunk)
            .map(|part| scope.spawn(move || part.iter().map(|&s| job(s)).collect::<Vec<T>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("a match thread panicked"))
            .collect()
    })
}
