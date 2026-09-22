//! AC-e: added time is the sum of the half's stoppages and cards plus a seeded variance,
//! clamped to the rule pack's range. The teams change ends at half-time, and the match ends
//! at regulation time plus the time added to each half. A match shorter than regulation
//! plays no added time.

mod common;

use common::{full_match, quiet_match, short_match};
use engine::scenario::Scene;
use engine::{EngineEvent, EngineEventKind, NullSink, Simulation, StoppageKind};

/// Ticks in 45 minutes.
const HALF: u32 = 45 * 60 * 50;

fn find(events: &[EngineEvent], kind: EngineEventKind) -> Vec<EngineEvent> {
    events.iter().copied().filter(|e| e.kind == kind).collect()
}

#[test]
fn added_time_follows_the_stoppages_the_cards_and_the_variance_draw() {
    let config = quiet_match(90);
    let added = config.rules.added_time.clone();
    let draw = 0.75;
    let base = 2 * added.seconds(StoppageKind::Goal)
        + 5 * added.seconds(StoppageKind::FreeKick)
        + added.card_s;
    let expected = (f64::from(base) + (2.0 * draw - 1.0) * f64::from(added.variance_s))
        .round()
        .clamp(f64::from(added.min_s), f64::from(added.max_s)) as u32;
    assert!(expected > added.min_s, "the scene is above the minimum");

    let mut sim = Scene::new(config)
        .tick(HALF - 10)
        .tally(StoppageKind::Goal, 2)
        .tally(StoppageKind::FreeKick, 5)
        .cards(1)
        .rolls(&[draw])
        .build();
    let attack_before = sim.teams().map(|t| t.attack_x);
    let mut events = Vec::new();
    while events
        .iter()
        .all(|e: &EngineEvent| e.kind != EngineEventKind::HalfTime)
    {
        assert!(sim.tick() < HALF + 20 * 60 * 50, "half-time never came");
        sim.step();
        events.extend(sim.take_events());
    }
    let half_time = find(&events, EngineEventKind::HalfTime)[0];
    let added_s = half_time
        .added_time_s
        .expect("half-time names the added time");
    assert!(
        added_s.abs_diff(expected) <= 1,
        "{added_s} s against {expected} s"
    );
    assert_eq!(half_time.tick, HALF + added_s * 50);
    assert_eq!(half_time.minute, 45);
    assert!(half_time.minute_added.is_some());

    // The ends change and the away side kicks off the second half.
    let attack_after = sim.teams().map(|t| t.attack_x);
    assert_eq!(attack_after, attack_before.map(|x| -x));
    let kick_off = find(&events, EngineEventKind::KickOff);
    assert_eq!(kick_off.last().unwrap().team, Some(1));
    assert_eq!(sim.half(), 1);
}

#[test]
fn a_full_match_ends_at_ninety_minutes_plus_the_added_time() {
    let mut sim = Simulation::new(full_match()).unwrap();
    sim.run(&mut NullSink).unwrap();
    let added = sim.summary().added_s;
    assert!(added.iter().all(|&s| s >= 60), "{added:?}");
    assert_eq!(sim.tick(), 2 * HALF + (added[0] + added[1]) * 50);
    let events = sim.take_events();
    let full_time = find(&events, EngineEventKind::FullTime);
    assert_eq!(full_time.len(), 1);
    assert_eq!(full_time[0].added_time_s, Some(added[1]));
    assert_eq!(full_time[0].minute, 90);
    let half_time = find(&events, EngineEventKind::HalfTime);
    assert_eq!(half_time[0].tick, HALF + added[0] * 50);
}

#[test]
fn a_shortened_match_plays_no_added_time() {
    let mut sim = Simulation::new(short_match(10)).unwrap();
    sim.run(&mut NullSink).unwrap();
    assert_eq!(sim.tick(), 10 * 60 * 50);
    assert_eq!(sim.summary().added_s, [0, 0]);
    let events = sim.take_events();
    let half_time = find(&events, EngineEventKind::HalfTime);
    assert_eq!(half_time.len(), 1);
    assert_eq!(half_time[0].tick, 5 * 60 * 50);
    assert_eq!(half_time[0].added_time_s, None);
}
