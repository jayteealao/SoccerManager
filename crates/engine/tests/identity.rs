//! AC-e: a match record saved and loaded keeps the owner identifier and the match
//! identifier unchanged, in `stats.json` and in the tick-file header.

mod common;

use engine::observe::identity::{MatchId, load_or_create_owner_id, owner_bytes, owner_hex};
use engine::observe::{MatchStats, TeamRef, read_stats, write_stats};
use engine::{FileSink, Simulation, TickHeader, read_ticks, ticks_for_minutes};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("engine-identity-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn owner_and_match_ids_round_trip_through_stats_json() {
    let data = temp_dir("stats");
    let owner_id = load_or_create_owner_id(&data).unwrap();
    let match_id = MatchId::now(common::SEED);
    let stats = MatchStats {
        owner_id: owner_id.clone(),
        match_id: match_id.to_string(),
        seed: common::SEED,
        content_hash: common::content().hash(),
        teams: [
            TeamRef {
                id: "club-a".into(),
                name: "A".into(),
            },
            TeamRef {
                id: "club-b".into(),
                name: "B".into(),
            },
        ],
        duration_ms: 0,
        outcome: "success".into(),
        ticks_per_s: 0.0,
        ticks_written: 0,
        validate_ran: false,
        validate_violations: 0,
        possession_changes: 0,
        ball_max_speed: 0.0,
        ball_idle_ticks: 0,
        goals: [0, 0],
    };
    let path = write_stats(&data, &stats).unwrap();
    let read = read_stats(&path).unwrap();
    std::fs::remove_dir_all(&data).unwrap();
    assert_eq!(read.owner_id, owner_id);
    assert_eq!(read.match_id, match_id.to_string());
    assert_eq!(read.match_id.parse::<MatchId>().unwrap(), match_id);
    assert_eq!(read, stats);
}

#[test]
fn owner_and_match_ids_round_trip_through_the_tick_header() {
    let data = temp_dir("header");
    let owner_id = load_or_create_owner_id(&data).unwrap();
    std::fs::remove_dir_all(&data).unwrap();
    let match_id = MatchId::now(common::SEED);
    let config = common::full_match();
    let ticks = ticks_for_minutes(1);
    let header = TickHeader {
        seed: config.seed,
        dt: config.tuning.dt,
        expected_ticks: ticks,
        owner_id: owner_bytes(&owner_id).unwrap(),
        match_millis: match_id.millis,
    };
    let path = common::temp_path("identity");
    let mut sim = Simulation::new(config).unwrap();
    let mut sink = FileSink::create(&path, &header).unwrap();
    sim.run(ticks, &mut sink).unwrap();
    sink.finish().unwrap();
    let file = read_ticks(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(file.header, header);
    assert_eq!(owner_hex(&file.header.owner_id), owner_id);
    assert_eq!(
        MatchId {
            seed: file.header.seed,
            millis: file.header.match_millis
        },
        match_id
    );
}
