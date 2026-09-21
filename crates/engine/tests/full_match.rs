//! AC-a: a 90-minute simulation writes 270,000 ticks, each with one ball position and
//! 22 player positions.

mod common;

use engine::{FileSink, Simulation, read_ticks, ticks_for_minutes};

#[test]
fn ninety_minutes_write_270_000_records() {
    let path = common::temp_path("full");
    let config = common::full_match();
    let ticks = ticks_for_minutes(90);
    assert_eq!(ticks, 270_000);
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut sink = FileSink::create(&path, config.seed, config.tuning.dt, ticks).unwrap();
    sim.run(ticks, &mut sink).unwrap();
    assert_eq!(sink.finish().unwrap(), 270_000);

    let file = read_ticks(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(file.records.len(), 270_000);
    assert_eq!(file.expected_ticks, 270_000);
    for (i, r) in file.records.iter().enumerate() {
        assert_eq!(r.tick as usize, i + 1);
        assert_eq!(r.ball.len(), 3);
        assert_eq!(r.players.len(), 22);
    }
    let summary = sim.summary();
    assert!(
        summary.possession_changes > 20,
        "possession changed {} times",
        summary.possession_changes
    );
    assert!(
        summary.ball_idle_ticks < 5_000,
        "ball idle for {} ticks",
        summary.ball_idle_ticks
    );
}
