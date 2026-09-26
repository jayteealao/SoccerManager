//! Extra time (IFAB Laws 3 and 7). A knockout match level at the end of regulation time plays
//! two periods of 15 minutes, each with its own added time, and then a shoot-out; a
//! knockout match with a winner, and any match that is not a knockout match, ends at 90
//! minutes plus the added time. Once extra time starts each team may make one more
//! substitution in one more window, and the breaks use no window.

mod common;

use common::{calm_match, index};
use engine::rules::clock::TICKS_PER_MINUTE;
use engine::scenario::Scene;
use engine::{
    Change, DecidedBy, EngineEvent, EngineEventKind, EventDetail, Manager, RejectReason, Simulation,
};

/// The home slot the substitution tests take off.
const SLOT: usize = 5;

/// Steps `sim` to full time and returns every event, full time included.
fn play_out(sim: &mut Simulation) -> Vec<EngineEvent> {
    let mut events = Vec::new();
    for _ in 0..400_000 {
        if sim.is_over() {
            break;
        }
        sim.step();
        events.extend(sim.take_events());
    }
    assert!(sim.is_over(), "the match never ended");
    sim.finish();
    events.extend(sim.take_events());
    events
}

fn of(events: &[EngineEvent], kind: EngineEventKind) -> Vec<EngineEvent> {
    events.iter().copied().filter(|e| e.kind == kind).collect()
}

/// A level, hand-managed knockout match at minute 89.
fn level_knockout_at_89() -> Simulation {
    Scene::new(calm_match(90).with_knockout())
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .at_minute(89)
        .build()
}

#[test]
fn a_level_knockout_match_plays_two_periods_of_extra_time() {
    let mut sim = level_knockout_at_89();
    let events = play_out(&mut sim);
    let summary = sim.summary();
    assert!(summary.extra_time);

    // Two more breaks, before each extra-time period, name the period that starts.
    let breaks = of(&events, EngineEventKind::HalfTime);
    let periods: Vec<Option<u32>> = breaks.iter().map(|e| e.period).collect();
    assert_eq!(periods, vec![Some(2), Some(3)]);
    assert_eq!(breaks[0].minute, 90);
    assert_eq!(breaks[1].minute, 105);
    let kick_offs: Vec<EngineEvent> = of(&events, EngineEventKind::KickOff)
        .into_iter()
        .filter(|e| e.period.is_some())
        .collect();
    assert_eq!(kick_offs.len(), 2);
    // The team that did not kick off the first period kicks off the second.
    assert_ne!(kick_offs[0].team, kick_offs[1].team);
    // The second period starts at minute 105.
    assert_eq!(
        (kick_offs[1].minute, kick_offs[1].minute_added),
        (105, None)
    );

    // Each period's added time is priced, capped at the rule pack's extra-time maximum, and
    // the next period starts when it has run.
    let cap = sim.config().rules.extra_time.added_max_s;
    for (k, added) in summary.extra_added_s.iter().enumerate() {
        assert!(*added > 0 && *added <= cap, "period {k}: {added} s");
    }
    let period_ticks = 15 * TICKS_PER_MINUTE;
    assert_eq!(
        breaks[1].tick,
        breaks[0].tick + period_ticks + summary.extra_added_s[0] * 50
    );
    assert_eq!(breaks[1].added_time_s, Some(summary.extra_added_s[0]));

    // Still level after extra time: the shoot-out decides it.
    let full_time = of(&events, EngineEventKind::FullTime);
    assert_eq!(full_time.len(), 1);
    assert_eq!(full_time[0].decided_by, Some(DecidedBy::Shootout));
    assert_eq!(full_time[0].minute, 120);
    assert_eq!(summary.goals, [0, 0]);
}

#[test]
fn a_level_match_that_is_not_a_knockout_match_ends_after_ninety_minutes() {
    let mut sim = Scene::new(calm_match(90))
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .at_minute(89)
        .build();
    let events = play_out(&mut sim);
    assert!(of(&events, EngineEventKind::HalfTime).is_empty());
    assert!(events.iter().all(|e| e.period.is_none()));
    let full_time = of(&events, EngineEventKind::FullTime);
    assert_eq!(full_time[0].minute, 90);
    assert_eq!(full_time[0].decided_by, None);
    assert_eq!(sim.summary().shootout, None);
    assert_eq!(
        sim.tick(),
        90 * TICKS_PER_MINUTE + sim.summary().added_s[1] * 50
    );
}

#[test]
fn a_knockout_match_with_a_winner_at_ninety_minutes_plays_no_extra_time() {
    let mut sim = Scene::new(calm_match(90).with_knockout())
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .at_minute(89)
        .score([1, 0])
        .build();
    let events = play_out(&mut sim);
    assert!(of(&events, EngineEventKind::HalfTime).is_empty());
    let full_time = of(&events, EngineEventKind::FullTime);
    assert_eq!(full_time[0].decided_by, Some(DecidedBy::Regulation));
    assert_eq!(full_time[0].minute, 90);
    assert!(!sim.summary().extra_time);
}

#[test]
fn a_shortened_level_knockout_match_goes_straight_to_the_shootout() {
    let mut sim = Simulation::new(calm_match(5).with_knockout()).unwrap();
    let events = play_out(&mut sim);
    // The regulation half-time only.
    let breaks = of(&events, EngineEventKind::HalfTime);
    assert_eq!(breaks.len(), 1);
    assert_eq!(breaks[0].period, None);
    assert!(!sim.summary().extra_time);
    let first_kick = events
        .iter()
        .find(|e| e.shootout_round.is_some())
        .expect("a shoot-out");
    assert_eq!(first_kick.tick, 5 * TICKS_PER_MINUTE);
    assert_eq!(first_kick.minute, 5);
    let full_time = of(&events, EngineEventKind::FullTime);
    assert_eq!(full_time[0].decided_by, Some(DecidedBy::Shootout));
}

/// The home side's squad index in `slot` and its first bench player.
fn off_and_on(sim: &Simulation, slot: usize, bench: usize) -> (usize, usize) {
    let team = &sim.teams()[0];
    (team.lineup[slot], team.bench[bench])
}

fn verdicts(events: &[EngineEvent]) -> Vec<(u32, EngineEventKind, Option<RejectReason>)> {
    events
        .iter()
        .filter_map(|e| match e.detail {
            Some(EventDetail::Change { reason, .. }) => Some((e.tick, e.kind, reason)),
            _ => None,
        })
        .collect()
}

#[test]
fn extra_time_adds_one_substitution_in_one_window() {
    // Five substitutions in three windows: the rule pack's limit and windows are used up.
    let sim = Scene::new(calm_match(90).with_knockout())
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .at_minute(89)
        .subs_used(0, 5, 3)
        .build();
    let (off, on) = off_and_on(&sim, SLOT, 0);
    let mut sim = Scene::new(calm_match(90).with_knockout())
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .at_minute(89)
        .subs_used(0, 5, 3)
        .queue(0, Change::Substitution { off, on })
        .build();
    // Play to the kick-off of the first period of extra time.
    let mut events = Vec::new();
    while sim.half() < 2 {
        sim.step();
        events.extend(sim.take_events());
    }
    let first = verdicts(&events);
    assert_eq!(first.len(), 1, "{events:?}");
    assert_eq!(
        first[0].1,
        EngineEventKind::ChangeApplied,
        "the sixth applies"
    );
    let ledger = sim.ledgers()[0];
    assert_eq!(ledger.used, 6);
    assert_eq!(
        ledger.windows, 3,
        "the break before extra time uses no window"
    );
    assert_eq!(sim.players()[index(0, SLOT)].squad, on);

    // A seventh is over the raised limit.
    let (off, on) = off_and_on(&sim, SLOT + 1, 0);
    sim.queue_change(0, Change::Substitution { off, on });
    let mut events = Vec::new();
    while sim.half() < 3 {
        sim.step();
        events.extend(sim.take_events());
    }
    let second = verdicts(&events);
    assert_eq!(second.len(), 1, "{events:?}");
    assert_eq!(second[0].1, EngineEventKind::ChangeRejected);
    assert_eq!(second[0].2, Some(RejectReason::LimitReached { limit: 6 }));
    assert_eq!(sim.ledgers()[0].used, 6);
}

#[test]
fn the_second_period_of_extra_time_starts_at_minute_one_hundred_and_five() {
    let mut sim = Scene::new(calm_match(90))
        .knockout()
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .period(3)
        .build();
    assert_eq!(sim.half(), 3);
    assert_eq!(sim.minute(), (105, None));
    assert!(sim.config().knockout);
    // The last period runs its 15 minutes and its added time, then the shoot-out decides.
    let events = play_out(&mut sim);
    assert!(of(&events, EngineEventKind::HalfTime).is_empty());
    let full_time = of(&events, EngineEventKind::FullTime);
    assert_eq!(full_time[0].decided_by, Some(DecidedBy::Shootout));
    assert_eq!(full_time[0].minute, 120);
    assert!(sim.summary().extra_added_s[1] > 0);
}
