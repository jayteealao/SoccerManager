//! AC-2: the rule pack's limit and windows. With five substitutions used a sixth is rejected
//! with the reason; with four used over two windows a fifth applies at the next dead ball and
//! the roster slot carries the substitute; a half-time substitution uses no window.
//! AC-3: a substitution and a role change for the same player, queued in either order, give
//! the substitution applied and the role change rejected with the player named.

mod common;

use common::{calm_match, index, spread};
use engine::math::DVec3;
use engine::rules::clock::TICKS_PER_MINUTE;
use engine::scenario::Scene;
use engine::{
    Change, EngineEvent, EngineEventKind, EventDetail, Manager, RejectReason, RoleDuty, Simulation,
    TacticsPatch,
};

/// The home slot the tests substitute.
const SLOT: usize = 5;

/// The home side's squad index in `SLOT` and its first bench player.
fn off_and_on() -> (usize, usize) {
    let sim = Simulation::new(calm_match(90)).unwrap();
    let team = &sim.teams()[0];
    (team.lineup[SLOT], team.bench[0])
}

/// Hand-managed teams and a ball rolling over the touchline, with `changes` queued for the
/// home side and `used` substitutions over `windows` windows already made.
fn scene(used: u8, windows: u8, changes: Vec<Change>) -> Scene {
    let mut scene = spread(Scene::new(calm_match(90)), -30.0, 30.0)
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .carrier(None)
        .ball(DVec3::new(-10.0, 32.0, 0.0))
        .ball_velocity(DVec3::new(0.0, 5.0, 0.0))
        .last_touch(1)
        .tick(1_001)
        .subs_used(0, used, windows);
    for change in changes {
        scene = scene.queue(0, change);
    }
    scene
}

/// Steps until the first stoppage and returns the events of that tick.
fn to_stoppage(sim: &mut Simulation) -> Vec<EngineEvent> {
    for _ in 0..200_000 {
        sim.step();
        let events = sim.take_events();
        if sim.stoppage().is_some() {
            return events;
        }
    }
    panic!("play never stopped");
}

fn verdicts(events: &[EngineEvent]) -> Vec<(EngineEventKind, Option<RejectReason>)> {
    events
        .iter()
        .filter_map(|e| match e.detail {
            Some(EventDetail::Change { reason, .. }) => Some((e.kind, reason)),
            _ => None,
        })
        .collect()
}

fn ids(sim: &Simulation) -> Vec<String> {
    sim.teams()[0].player_ids.clone()
}

#[test]
fn a_sixth_substitution_is_rejected_with_the_limit() {
    let (off, on) = off_and_on();
    let mut sim = scene(5, 3, vec![Change::Substitution { off, on }]).build();
    let events = to_stoppage(&mut sim);
    let v = verdicts(&events);
    assert_eq!(v.len(), 1, "{events:?}");
    assert_eq!(v[0].0, EngineEventKind::ChangeRejected);
    let reason = v[0].1.expect("a rejection names its reason");
    assert_eq!(
        reason.text(&ids(&sim)),
        "substitution limit reached (5 of 5)"
    );
    assert_eq!(
        sim.players()[index(0, SLOT)].squad,
        off,
        "the player stays on"
    );
    assert_eq!(sim.ledgers()[0].used, 5);
    assert!(
        sim.pending_changes().is_empty(),
        "a rejected change leaves the queue"
    );
}

#[test]
fn a_fifth_substitution_applies_at_the_next_dead_ball() {
    let (off, on) = off_and_on();
    let mut sim = scene(4, 2, vec![Change::Substitution { off, on }]).build();
    let events = to_stoppage(&mut sim);
    let v = verdicts(&events);
    assert_eq!(v, [(EngineEventKind::ChangeApplied, None)], "{events:?}");
    let sub = events
        .iter()
        .find(|e| e.kind == EngineEventKind::Substitution)
        .expect("a substitution event");
    assert_eq!(sub.team, Some(0));
    assert_eq!(sub.player, Some(index(0, SLOT)), "the roster slot");
    assert_eq!(sub.detail, Some(EventDetail::Substitution { off, on }));
    let player = sim.players()[index(0, SLOT)];
    assert_eq!(player.squad, on, "the roster slot carries the substitute");
    assert!(player.active());
    assert_eq!(sim.teams()[0].lineup[SLOT], on);
    assert!(sim.teams()[0].bench.iter().all(|&b| b != on && b != off));
    assert_eq!(sim.player_ids()[index(0, SLOT)], ids(&sim)[on]);
    let ledger = sim.ledgers()[0];
    assert_eq!((ledger.used, ledger.windows), (5, 3));
    assert_eq!(sim.summary().substitutions, [1, 0]);
}

#[test]
fn a_substitution_with_no_window_left_is_rejected_outside_half_time() {
    let (off, on) = off_and_on();
    let mut sim = scene(4, 3, vec![Change::Substitution { off, on }]).build();
    let events = to_stoppage(&mut sim);
    let v = verdicts(&events);
    assert_eq!(v.len(), 1, "{events:?}");
    assert_eq!(v[0].0, EngineEventKind::ChangeRejected);
    assert_eq!(
        v[0].1.unwrap().text(&ids(&sim)),
        "no substitution window left (3 of 3)"
    );
}

#[test]
fn a_half_time_substitution_uses_no_window() {
    let (off, on) = off_and_on();
    // Every window used, a minute before the end of the first half, with the ball at rest
    // and nobody near it: the next stoppage is half-time.
    let mut sim = spread(Scene::new(calm_match(90)), -30.0, 30.0)
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .carrier(None)
        .ball(DVec3::new(0.0, 0.5, 0.0))
        .tick(44 * TICKS_PER_MINUTE)
        .subs_used(0, 4, 3)
        .queue(0, Change::Substitution { off, on })
        .build();
    let events = to_stoppage(&mut sim);
    assert!(
        events.iter().any(|e| e.kind == EngineEventKind::HalfTime),
        "{events:?}"
    );
    assert_eq!(
        verdicts(&events),
        [(EngineEventKind::ChangeApplied, None)],
        "{events:?}"
    );
    assert_eq!(sim.players()[index(0, SLOT)].squad, on);
    let ledger = sim.ledgers()[0];
    assert_eq!((ledger.used, ledger.windows), (5, 3), "no window used");
}

/// The role change for the player in `SLOT`: the same role with the attacking duty.
fn role_change(sim: &Simulation, off: usize) -> Change {
    let role = sim.teams()[0].tactics.roles[SLOT].role;
    Change::Tactics(TacticsPatch {
        roles: vec![(off, RoleDuty { role, duty: 2 })],
        ..TacticsPatch::default()
    })
}

#[test]
fn a_substitution_and_a_role_change_for_the_same_player_in_either_order() {
    let (off, on) = off_and_on();
    let probe = Simulation::new(calm_match(90)).unwrap();
    let before = probe.teams()[0].tactics;
    let orders = [
        vec![Change::Substitution { off, on }, role_change(&probe, off)],
        vec![role_change(&probe, off), Change::Substitution { off, on }],
    ];
    for (n, changes) in orders.into_iter().enumerate() {
        let mut sim = scene(0, 0, changes).build();
        let events = to_stoppage(&mut sim);
        let v = verdicts(&events);
        assert_eq!(v.len(), 2, "order {n}: {events:?}");
        assert_eq!(
            v[0],
            (EngineEventKind::ChangeApplied, None),
            "order {n}: the substitution applies first"
        );
        assert_eq!(v[1].0, EngineEventKind::ChangeRejected, "order {n}");
        let reason = v[1].1.unwrap();
        assert_eq!(
            reason,
            RejectReason::LeftThePitch { squad: off },
            "order {n}"
        );
        let text = reason.text(&ids(&sim));
        assert!(text.contains(&ids(&sim)[off]), "order {n}: {text}");
        assert_eq!(sim.players()[index(0, SLOT)].squad, on, "order {n}");
        assert_eq!(
            sim.teams()[0].tactics,
            before,
            "order {n}: the tactics are unchanged"
        );
    }
}
