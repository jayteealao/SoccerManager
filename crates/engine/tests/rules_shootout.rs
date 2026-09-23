//! The penalty shoot-out (IFAB Law 10), played on the pitch. A knockout match still level
//! after extra time goes to a shoot-out that produces exactly one winner. Every kick moves the
//! ball and the players tick by tick and ends in one outcome event. The teams alternate, the
//! shoot-out ends early once one team cannot catch up, sudden death follows five kicks each,
//! and the order starts again once every eligible player has kicked. The numbers are made
//! equal after a sending-off, nothing a manager queues applies during the shoot-out, ten
//! rounds fit in the announced maximum, and one seed gives one result.

mod common;

use common::{calm_match, index};
use engine::record::{RECORD_BYTES, TickRecord};
use engine::rules::clock::TICKS_PER_MINUTE;
use engine::scenario::Scene;
use engine::{Change, DecidedBy, EngineEvent, EngineEventKind, Manager, Simulation};

/// Steps `sim` to full time, calling `each` after every tick, and returns every event, full
/// time included.
fn play_out(sim: &mut Simulation, mut each: impl FnMut(&Simulation)) -> Vec<EngineEvent> {
    let mut events = Vec::new();
    for _ in 0..800_000 {
        if sim.is_over() {
            break;
        }
        sim.step();
        each(sim);
        events.extend(sim.take_events());
    }
    assert!(sim.is_over(), "the match never ended");
    sim.finish();
    events.extend(sim.take_events());
    events
}

/// A shortened, hand-managed, level knockout match: regulation ends after 5 minutes with no
/// extra time, and the shoot-out follows at once.
fn short_knockout() -> Scene {
    Scene::new(calm_match(5).with_knockout())
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
}

/// The set-up event of every kick.
fn set_ups(events: &[EngineEvent]) -> Vec<EngineEvent> {
    events
        .iter()
        .copied()
        .filter(|e| e.shootout_round.is_some() && e.shootout_scored.is_none())
        .collect()
}

/// The outcome event of every kick.
fn outcomes(events: &[EngineEvent]) -> Vec<EngineEvent> {
    events
        .iter()
        .copied()
        .filter(|e| e.shootout_scored.is_some())
        .collect()
}

fn full_time(events: &[EngineEvent]) -> EngineEvent {
    let all: Vec<EngineEvent> = events
        .iter()
        .copied()
        .filter(|e| e.kind == EngineEventKind::FullTime)
        .collect();
    assert_eq!(all.len(), 1);
    all[0]
}

#[test]
fn level_after_extra_time_a_shootout_on_the_pitch_produces_one_winner() {
    let mut sim = Scene::new(calm_match(90).with_knockout())
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .at_minute(89)
        .build();
    let mut records: Vec<TickRecord> = Vec::new();
    let events = play_out(&mut sim, |sim| records.push(sim.record()));
    assert!(sim.summary().extra_time, "extra time came first");
    let end = full_time(&events);
    assert_eq!(end.decided_by, Some(DecidedBy::Shootout));
    let scores = end
        .shootout_scores
        .expect("full time names the shoot-out score");
    assert_ne!(scores[0], scores[1], "exactly one winner");
    assert_eq!(sim.summary().shootout, Some(scores));
    assert_eq!(end.scores, [0, 0], "the score of play is unchanged");
    assert!(!sim.abandoned());

    // Every kick has one set-up and one outcome, in turn, and the teams alternate.
    let set_ups = set_ups(&events);
    let outcomes = outcomes(&events);
    assert_eq!(set_ups.len(), outcomes.len());
    assert_eq!(outcomes.len() as u32, sim.summary().shootout_kicks);
    for (k, (set_up, outcome)) in set_ups.iter().zip(&outcomes).enumerate() {
        assert!(set_up.tick < outcome.tick, "kick {k}");
        if let Some(next) = set_ups.get(k + 1) {
            assert!(outcome.tick <= next.tick, "kick {k} ends before the next");
        }
        assert_eq!(set_up.team, outcome.team);
        assert_eq!(set_up.player, outcome.player);
        assert_eq!(set_up.kind, EngineEventKind::Penalty);
        if k > 0 {
            assert_ne!(outcome.team, outcomes[k - 1].team, "kick {k} alternates");
        }
    }
    let last = outcomes.last().unwrap();
    assert_eq!(last.shootout_scores, Some(scores));

    // The ball and the players move on the ticks of every kick.
    let first_tick = records[0].tick;
    for (k, (set_up, outcome)) in set_ups.iter().zip(&outcomes).enumerate() {
        let span =
            &records[(set_up.tick - first_tick) as usize..=(outcome.tick - first_tick) as usize];
        let ball_moved = span.windows(2).any(|w| w[0].ball != w[1].ball);
        let players_moved = span.windows(2).any(|w| w[0].players != w[1].players);
        assert!(ball_moved, "kick {k}: the ball never moved");
        assert!(players_moved, "kick {k}: no player moved");
        assert!(span.len() > 2, "kick {k} took {} ticks", span.len());
    }
}

#[test]
fn a_team_that_cannot_catch_up_loses_early() {
    // The first kicker's team scores three, the other misses three.
    let mut sim = short_knockout()
        .shootout_kicks(&[true, false, true, false, true, false])
        .build();
    let events = play_out(&mut sim, |_| {});
    let outcomes = outcomes(&events);
    assert_eq!(outcomes.len(), 6);
    let first = outcomes[0].team.unwrap();
    let mut expected = [0, 0];
    expected[first] = 3;
    assert_eq!(full_time(&events).shootout_scores, Some(expected));
}

#[test]
fn level_after_five_kicks_each_sudden_death_decides_it() {
    let mut forced = vec![true; 10];
    forced.extend([true, false]);
    let mut sim = short_knockout().shootout_kicks(&forced).build();
    let events = play_out(&mut sim, |_| {});
    let outcomes = outcomes(&events);
    assert_eq!(outcomes.len(), 12);
    let first = outcomes[0].team.unwrap();
    let mut expected = [5, 5];
    expected[first] = 6;
    assert_eq!(full_time(&events).shootout_scores, Some(expected));
    assert_eq!(outcomes[10].shootout_round, Some(6));
}

#[test]
fn the_order_starts_again_once_every_eligible_player_has_kicked() {
    let mut forced = vec![true; 22];
    forced.extend([true, false]);
    let mut sim = short_knockout().shootout_kicks(&forced).build();
    let events = play_out(&mut sim, |_| {});
    let outcomes = outcomes(&events);
    assert_eq!(outcomes.len(), 24);
    for team in 0..2 {
        let kickers: Vec<usize> = outcomes
            .iter()
            .filter(|e| e.team == Some(team))
            .map(|e| e.player.unwrap())
            .collect();
        let mut distinct = kickers[..11].to_vec();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 11, "team {team}: every player kicks once");
        assert_eq!(
            kickers[11], kickers[0],
            "team {team}: the order starts again"
        );
        // The goalkeeper kicks last.
        assert_eq!(kickers[10], index(team, 0));
    }
}

#[test]
fn a_team_with_more_players_drops_players_to_equal_the_numbers() {
    // The away side ends with ten players; the home side leaves one out.
    let sent_off = index(1, 9);
    let mut forced = vec![true; 20];
    forced.extend([true, false]);
    let mut sim = short_knockout()
        .sent_off(sent_off)
        .shootout_kicks(&forced)
        .build();
    let events = play_out(&mut sim, |_| {});
    let outcomes = outcomes(&events);
    for team in 0..2 {
        let kickers: Vec<usize> = outcomes
            .iter()
            .filter(|e| e.team == Some(team))
            .map(|e| e.player.unwrap())
            .collect();
        let mut distinct = kickers.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 10, "team {team} kicks with ten");
        assert!(!kickers.contains(&sent_off));
        assert_eq!(
            kickers[10], kickers[0],
            "team {team}: the order starts again"
        );
    }
}

#[test]
fn a_change_queued_during_the_shootout_is_not_applied() {
    let mut sim = short_knockout().build();
    while !sim.in_shootout() {
        sim.step();
    }
    sim.take_events();
    let team = &sim.teams()[0];
    let (off, on) = (team.lineup[5], team.bench[0]);
    sim.queue_change(0, Change::Substitution { off, on });
    let events = play_out(&mut sim, |_| {});
    assert!(
        events.iter().all(|e| !matches!(
            e.kind,
            EngineEventKind::ChangeApplied
                | EngineEventKind::ChangeRejected
                | EngineEventKind::Substitution
        )),
        "{events:?}"
    );
    assert_eq!(sim.pending_changes().len(), 1);
    assert_eq!(sim.unapplied_changes().never_applied, 0);
    assert_eq!(sim.ledgers()[0].used, 0);
}

#[test]
fn ten_rounds_of_kicks_fit_in_the_announced_maximum() {
    let mut forced = vec![true; 18];
    forced.extend([true, false]);
    let config = calm_match(5).with_knockout();
    let max = config.max_ticks();
    let mut sim = Scene::new(config)
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .shootout_kicks(&forced)
        .build();
    let events = play_out(&mut sim, |_| {});
    assert_eq!(outcomes(&events).len(), 20);
    assert!(sim.tick() <= max, "{} ticks against {max}", sim.tick());
    assert!(sim.tick() > 5 * TICKS_PER_MINUTE);
}

#[test]
fn one_seed_gives_one_shootout() {
    let run = || {
        let mut sim = short_knockout().build();
        let mut bytes = Vec::new();
        let mut buf = [0u8; RECORD_BYTES];
        let events = play_out(&mut sim, |sim| {
            sim.record().write_to(&mut buf);
            bytes.extend_from_slice(&buf);
        });
        (events, bytes)
    };
    let (a_events, a_bytes) = run();
    let (b_events, b_bytes) = run();
    assert_eq!(a_events, b_events);
    assert!(a_bytes == b_bytes, "the two runs' ticks differ");
    assert!(!outcomes(&a_events).is_empty());
}
