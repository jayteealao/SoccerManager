//! AC-1: a tactics change queued in open play waits on every tick until the next stoppage
//! that admits it, applies on the tick that opens that stoppage, and the applied event
//! carries that tick. A penalty admits no change, so a change waits through it; the next
//! stoppage that admits one applies it, as the first test shows for a throw-in.

mod common;

use common::{calm_match, spread};
use engine::math::{DVec2, DVec3};
use engine::scenario::Scene;
use engine::{
    Change, EngineEvent, EngineEventKind, EventDetail, Manager, Simulation, StoppageKind,
    TacticsPatch,
};

/// The mentality index of "attacking" in the shipped tactics file.
const ATTACKING: u8 = 4;

/// The applied or rejected verdicts in `events`.
fn verdicts(events: &[EngineEvent]) -> Vec<EngineEvent> {
    events
        .iter()
        .copied()
        .filter(|e| {
            matches!(
                e.kind,
                EngineEventKind::ChangeApplied | EngineEventKind::ChangeRejected
            )
        })
        .collect()
}

/// Both teams managed by hand, the ball rolling slowly toward the touchline from the middle
/// of the home half with every player far from it, and a mentality change queued for the
/// home side on the first tick.
fn rolling_scene() -> Simulation {
    spread(Scene::new(calm_match(90)), -30.0, 30.0)
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .carrier(None)
        .ball(DVec3::new(-10.0, 32.0, 0.0))
        .ball_velocity(DVec3::new(0.0, 5.0, 0.0))
        .last_touch(1)
        .tick(1_001)
        .queue(0, Change::Tactics(TacticsPatch::mentality(ATTACKING)))
        .build()
}

#[test]
fn a_change_waits_in_open_play_and_applies_on_the_stoppage_tick() {
    let mut sim = rolling_scene();
    let before = sim.teams()[0].tactics;
    assert_ne!(before.mentality, ATTACKING);
    let queued_at = sim.pending_changes()[0].id.tick;
    for open_ticks in 0..500 {
        sim.step();
        let events = sim.take_events();
        if let Some(restart) = events.iter().find(|e| e.spot.is_some()).copied() {
            assert_eq!(restart.kind, EngineEventKind::ThrowIn);
            let verdicts = verdicts(&events);
            assert_eq!(verdicts.len(), 1, "{events:?}");
            let applied = verdicts[0];
            assert_eq!(applied.kind, EngineEventKind::ChangeApplied);
            assert_eq!(applied.team, Some(0));
            assert_eq!(applied.tick, restart.tick, "applied on the stoppage tick");
            match applied.detail {
                Some(EventDetail::Change { id, reason, .. }) => {
                    assert_eq!(id.tick, queued_at);
                    assert_eq!(reason, None);
                }
                other => panic!("an applied change carries its detail: {other:?}"),
            }
            assert!(sim.pending_changes().is_empty());
            assert_eq!(sim.teams()[0].tactics.mentality, ATTACKING);
            assert!(open_ticks > 10, "the ball rolled for a while first");
            assert_eq!(sim.summary().changes_applied, 1);
            return;
        }
        assert!(verdicts(&events).is_empty(), "no verdict in open play");
        assert_eq!(
            sim.pending_changes().len(),
            1,
            "pending on tick {}",
            sim.tick()
        );
        assert_eq!(sim.teams()[0].tactics.mentality, before.mentality);
    }
    panic!("the ball never left the pitch");
}

#[test]
fn a_penalty_admits_no_change_so_the_change_waits_through_it() {
    // A clean foul inside the away side's penalty area, with the ball lost.
    const CARRIER: usize = 5;
    const TACKLER: usize = 16;
    let config = calm_match(90);
    let bands = {
        let sim = Simulation::new(config.clone()).unwrap();
        let tackler = sim.players()[TACKLER].derived;
        let carrier = sim.players()[CARRIER].derived;
        let p_win = 0.05 * tackler.tackling / (tackler.tackling + carrier.dribbling);
        (
            p_win,
            engine::rules::fouls::foul_chance(&tackler, &config.tuning),
        )
    };
    let at = DVec2::new(45.0, 0.0);
    let mut sim = spread(Scene::new(config), -30.0, 30.0)
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .place(CARRIER, at)
        .place(TACKLER, at + DVec2::new(0.3, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001)
        .rolls(&[bands.0 + 0.1 * bands.1, 0.99])
        .queue(0, Change::Tactics(TacticsPatch::mentality(ATTACKING)))
        .build();
    sim.step();
    let events = sim.take_events();
    assert!(
        events.iter().any(|e| e.kind == EngineEventKind::Penalty),
        "{events:?}"
    );
    assert_eq!(sim.dead_ball().unwrap().kind, StoppageKind::Penalty);
    assert!(verdicts(&events).is_empty(), "a penalty admits no change");
    assert_eq!(sim.pending_changes().len(), 1);
    // Through the whole penalty the change waits.
    while sim.dead_ball().is_some() {
        sim.step();
        let events = sim.take_events();
        assert!(verdicts(&events).is_empty(), "{events:?}");
        assert_eq!(sim.pending_changes().len(), 1);
    }
    assert_ne!(sim.teams()[0].tactics.mentality, ATTACKING);
    assert_eq!(
        sim.config().rules.admits(StoppageKind::Penalty),
        (false, false),
        "the shipped rule pack admits no change at a penalty"
    );
    assert_eq!(
        sim.config().rules.admits(StoppageKind::ThrowIn),
        (true, true)
    );
}

/// A 2-minute match managed by hand on both sides. With no decisions nobody kicks the ball,
/// so half-time is the only stoppage that admits a change.
fn still_match() -> Simulation {
    Scene::new(calm_match(2))
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .build()
}

#[test]
fn a_change_queued_after_the_last_admitting_stoppage_expires_at_full_time() {
    let mut sim = still_match();
    sim.queue_change(0, Change::Tactics(TacticsPatch::mentality(ATTACKING)));
    let mut half_time = false;
    while !half_time {
        sim.step();
        half_time = sim
            .stoppage()
            .is_some_and(|s| s.kind == StoppageKind::HalfTime);
    }
    assert!(
        sim.pending_changes().is_empty(),
        "half-time applied the change queued before it"
    );
    sim.queue_change(1, Change::Tactics(TacticsPatch::mentality(ATTACKING)));
    while !sim.is_over() {
        sim.step();
    }
    sim.finish();
    assert_eq!(sim.pending_changes().len(), 1);
    let stats = engine::observe::TacticsStats::new(&sim);
    assert_eq!(
        stats.change_never_applied, 0,
        "no admitting stoppage came after it"
    );
    assert_eq!(stats.change_expired_at_full_time, 1);
}
