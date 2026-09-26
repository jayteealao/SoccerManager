//! AC-a: a 90-minute simulation writes one record per tick, each with one ball position and
//! 22 player positions, from kick-off to full time. The header announces the most ticks the
//! match can last (regulation plus the cap on added time in both halves); the match lasts
//! regulation plus the added time it earned. The law counts sit in a sanity band only; the
//! calibrated rates come later. Offsides and corners need forward runs and deflections that
//! open play does not make yet, so this match only counts them; the scripted criterion tests
//! prove each law. Both sides are managed by the AI manager: shots are counted, no side
//! makes more substitutions than the rule pack allows, and no change is left waiting.

mod common;

use engine::{EngineEventKind, FileSink, Simulation, read_ticks, ticks_for_minutes};

#[test]
fn ninety_minutes_play_to_full_time_under_the_laws() {
    let path = common::temp_path("full");
    let config = common::full_match();
    let most = config.max_ticks();
    assert_eq!(most, 360_000);
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut sink = FileSink::create(&path, &common::header(&config, most)).unwrap();
    sim.run(&mut sink).unwrap();
    let written = sink.finish().unwrap();
    let summary = sim.summary();
    eprintln!("{summary:?} ticks {written}");

    let regulation = ticks_for_minutes(90);
    let added = (summary.added_s[0] + summary.added_s[1]) * 50;
    assert!(
        (regulation..=most).contains(&written),
        "{written} ticks written"
    );
    assert!(!sim.abandoned());
    assert_eq!(written, regulation + added);

    let file = read_ticks(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(file.records.len() as u32, written);
    assert_eq!(file.header.expected_ticks, most);
    assert_eq!(file.header.owner_id, [0x42; 16]);
    for (i, r) in file.records.iter().enumerate() {
        assert_eq!(r.tick as usize, i + 1);
        assert_eq!(r.ball.len(), 3);
        assert_eq!(r.players.len(), 22);
    }

    let events = sim.take_events();
    let count = |kind| events.iter().filter(|e| e.kind == kind).count();
    assert_eq!(count(EngineEventKind::HalfTime), 1);
    assert_eq!(count(EngineEventKind::FullTime), 1);
    let full_time = events.last().unwrap();
    assert_eq!(full_time.kind, EngineEventKind::FullTime);
    assert_eq!(full_time.added_time_s, Some(summary.added_s[1]));

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
    for team in 0..2 {
        assert!(
            (4..=20).contains(&summary.fouls[team]),
            "team {team} committed {} fouls",
            summary.fouls[team]
        );
        assert!(
            summary.free_kicks[team] > 0,
            "team {team} took no free kick"
        );
    }
    let out_of_play: u32 = [summary.throw_ins, summary.corners, summary.goal_kicks]
        .iter()
        .flatten()
        .sum();
    assert!(out_of_play > 0, "the ball never left the pitch");
    for team in 0..2 {
        assert!(summary.shots[team] > 0, "team {team} took no shot");
        assert!(
            summary.substitutions[team] <= 5,
            "team {team} made {} substitutions",
            summary.substitutions[team]
        );
        assert_eq!(
            u32::from(sim.ledgers()[team].used),
            summary.substitutions[team]
        );
    }
    assert!(
        sim.pending_changes().is_empty(),
        "a change waited to full time: {:?}",
        sim.pending_changes()
    );
    assert_eq!(
        summary.changes_queued,
        summary.changes_applied + summary.changes_rejected
    );
    assert_eq!(
        summary.stoppages as usize,
        events.iter().filter(|e| e.spot.is_some()).count() - 2 + 1,
        "one stoppage per restart after the opening kick-off, plus half-time"
    );
}
