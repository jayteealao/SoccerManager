//! AC-c and AC-d: the validator finds no violation over a seeded full match played under the
//! laws, with every restart spot checked, and reports a hand-corrupted stream. Fatigue,
//! injuries, and the AI manager's substitutions and tactics changes are on in that match.

mod common;

use engine::{Simulation, StreamRules, Validator, VecSink};

#[test]
fn a_seeded_full_match_has_no_violations() {
    let config = common::full_match();
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut sink = VecSink::default();
    sim.run(&mut sink).unwrap();
    let events = sim.take_events();
    assert!(events.iter().any(|e| e.spot.is_some() && e.tick > 1));
    assert!(
        events
            .iter()
            .any(|e| e.kind == engine::EngineEventKind::Substitution),
        "the match made a substitution"
    );
    assert!(sim.fatigue_mean_pct() < 100.0, "fatigue is on");
    let validator = Validator::for_match(config.tuning.clone(), sim.team_timeline(), &events);
    let violations = validator.check(&sink.records);
    let sample: Vec<_> = violations.iter().take(5).collect();
    assert!(
        violations.is_empty(),
        "{} violations; first: {sample:?}",
        violations.len()
    );
}

#[test]
fn a_corrupted_stream_reports_the_overlap() {
    let config = common::short_match(1);
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut sink = VecSink::default();
    sim.run(&mut sink).unwrap();
    sink.records[10].players[1] = sink.records[10].players[0];
    let events = sim.take_events();
    let validator = Validator::for_match(config.tuning.clone(), sim.team_timeline(), &events);
    let violations = validator.check(&sink.records);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "separation");
    assert_eq!(violations[0].tick, sink.records[10].tick);
}

/// The full engine's event stream of a seeded match, with the rules it is judged by.
fn engine_events(config: engine::MatchConfig) -> (Vec<engine::EngineEvent>, StreamRules) {
    let rules = StreamRules::for_config(&config);
    let mut sim = Simulation::new(config).unwrap();
    sim.run(&mut engine::record::NullSink).unwrap();
    (sim.take_events(), rules)
}

/// The event-stream rules hold for the full engine before they judge anything else: seeds 1
/// to 5 over 90 minutes; seeds 7, 9 and 11, which bring injuries, a substitution the rule
/// pack refuses and a straight red; a 10-minute match, which adds no time; a knockout match
/// (seed 2) played through extra time and a shoot-out; and seed 29 between two generated
/// top-flight clubs (league seed 1), which brings a second yellow.
#[test]
fn the_full_engines_event_streams_keep_the_event_rules() {
    use engine::EngineEventKind::{Card, ChangeRejected, HalfTime, Injury, Substitution};
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let seeds = [1, 2, 3, 4, 5, 7, 9, 11];
    let mut streams: Vec<(String, (Vec<engine::EngineEvent>, StreamRules))> =
        common::run_many(0..=seeds.len() as u64 - 1, |i| {
            let seed = seeds[i as usize];
            let config = engine::MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
            (format!("seed {seed}"), engine_events(config))
        });
    let short = engine::MatchConfig::new(1, 10, &content, [&a, &b]).unwrap();
    streams.push(("10 minutes seed 1".into(), engine_events(short)));
    let knockout = engine::MatchConfig::new(2, 90, &content, [&a, &b])
        .unwrap()
        .with_knockout();
    let (events, rules) = engine_events(knockout);
    assert!(
        events.iter().any(|e| e.shootout_scored == Some(true)),
        "the knockout match reached a shoot-out with a scored kick"
    );
    streams.push(("knockout seed 2".into(), (events, rules)));
    let league = engine::data::generate_league(1, 2, &content);
    let generated = engine::MatchConfig::new(29, 90, &content, [&league[0], &league[1]]).unwrap();
    streams.push(("generated seed 29".into(), engine_events(generated)));
    let all: Vec<&engine::EngineEvent> = streams.iter().flat_map(|(_, (e, _))| e).collect();
    let has = |f: &dyn Fn(&engine::EngineEvent) -> bool| all.iter().any(|e| f(e));
    assert!(has(&|e| e.kind == Injury), "an injury is judged");
    assert!(has(&|e| e.kind == Substitution), "a substitution is judged");
    assert!(
        has(&|e| e.kind == ChangeRejected),
        "a refused change is judged"
    );
    for card in [
        engine::Card::Yellow,
        engine::Card::SecondYellow,
        engine::Card::Red,
    ] {
        assert!(has(&|e| e.kind == Card && e.card == Some(card)), "{card:?}");
    }
    for (name, (events, rules)) in &streams {
        assert!(events.iter().any(|e| e.kind == HalfTime), "{name}");
        let violations = Validator::check_events(events, rules);
        assert!(violations.is_empty(), "{name}: {violations:?}");
    }
}

/// The rules of a regulation match on the shipped rule pack: 5 substitutions in 3 windows,
/// half-time free, 60 to 900 added seconds, every squad index in its own slot.
fn planted_rules() -> StreamRules {
    StreamRules {
        substitutions: 5,
        windows: 3,
        half_time_exempt: true,
        added_s: Some((60, 900)),
        lineups: [std::array::from_fn(|s| s), std::array::from_fn(|s| s)],
    }
}

/// Each event-stream rule fails on one planted stream.
#[test]
fn each_event_rule_fails_on_a_planted_stream() {
    use engine::EngineEventKind::{FullTime, Goal, HalfTime, KickOff};
    let rules = planted_rules();
    let ev = |kind, tick, team: Option<usize>, scores: [u32; 2]| engine::EngineEvent {
        tick,
        kind,
        team,
        scores,
        minute: tick / 3000,
        minute_added: None,
        player: None,
        secondary: None,
        card: None,
        advantage: None,
        added_time_s: None,
        spot: None,
        detail: None,
        period: None,
        shootout_round: None,
        shootout_scored: None,
        shootout_scores: None,
        decided_by: None,
    };
    let good = vec![
        ev(KickOff, 1, Some(0), [0, 0]),
        ev(Goal, 100, Some(1), [0, 1]),
        ev(KickOff, 101, Some(0), [0, 1]),
        ev(HalfTime, 200, None, [0, 1]),
        ev(FullTime, 400, None, [0, 1]),
    ];
    assert!(Validator::check_events(&good, &rules).is_empty());
    let rules = |events: &[engine::EngineEvent]| -> Vec<&'static str> {
        Validator::check_events(events, &rules)
            .iter()
            .map(|v| v.rule)
            .collect()
    };

    let mut s = good.clone();
    s[2].tick = 50;
    assert_eq!(rules(&s), ["event_order"]);

    let mut s = good.clone();
    s[0].kind = Goal;
    assert!(rules(&s).contains(&"kick_off_first"), "{:?}", rules(&s));

    let mut s = good.clone();
    s.push(ev(KickOff, 401, Some(1), [0, 1]));
    assert_eq!(rules(&s), ["full_time_last"]);

    let mut s = good.clone();
    s[1].scores = [1, 1];
    s[2].scores = [1, 1];
    s[3].scores = [1, 1];
    s[4].scores = [1, 1];
    assert_eq!(rules(&s), ["goal_score"]);

    let mut s = good.clone();
    s[3].scores = [0, 2];
    s[4].scores = [0, 2];
    assert_eq!(rules(&s), ["score_kept"]);

    let mut s = good.clone();
    s[3].team = Some(0);
    assert_eq!(rules(&s), ["period_end_team"]);
}

/// A legal stream with every kind the new rules judge: a foul with its fouled player, a
/// booking, a substitution after it, and added time on both period ends.
fn player_stream() -> Vec<engine::EngineEvent> {
    use engine::EngineEventKind::{Card, Foul, FullTime, HalfTime, KickOff, Substitution};
    let ev = |kind, tick: u32, team: Option<usize>| engine::EngineEvent {
        tick,
        kind,
        team,
        scores: [0, 0],
        minute: tick / 3000,
        minute_added: None,
        player: None,
        secondary: None,
        card: None,
        advantage: None,
        added_time_s: None,
        spot: None,
        detail: None,
        period: None,
        shootout_round: None,
        shootout_scored: None,
        shootout_scores: None,
        decided_by: None,
    };
    let mut kick_off = ev(KickOff, 0, Some(0));
    kick_off.player = Some(10);
    let mut foul = ev(Foul, 10, Some(0));
    foul.player = Some(4);
    foul.secondary = Some(15);
    foul.advantage = Some(false);
    let mut card = ev(Card, 10, Some(0));
    card.player = Some(4);
    card.card = Some(engine::Card::Yellow);
    let mut sub = ev(Substitution, 20, Some(1));
    sub.player = Some(13);
    sub.detail = Some(engine::EventDetail::Substitution { off: 2, on: 14 });
    let mut half_time = ev(HalfTime, 30, None);
    half_time.added_time_s = Some(75);
    let mut full_time = ev(FullTime, 60, None);
    full_time.added_time_s = Some(120);
    vec![kick_off, foul, card, sub, half_time, full_time]
}

/// Each new rule (players, cards, sendings-off, substitutions and added time) fails on one
/// planted change to a legal stream.
#[test]
fn each_player_rule_fails_on_a_planted_stream() {
    use engine::EngineEventKind::{Card, Foul, HalfTime, Substitution};
    let rules = planted_rules();
    let good = player_stream();
    assert!(Validator::check_events(&good, &rules).is_empty());
    let found = |events: &[engine::EngineEvent], rules: &StreamRules| -> Vec<&'static str> {
        Validator::check_events(events, rules)
            .iter()
            .map(|v| v.rule)
            .collect()
    };
    let at = |events: &mut Vec<engine::EngineEvent>, i: usize, e: engine::EngineEvent| {
        events.insert(i, e);
    };

    // A foul named on the wrong side, and a fouled player of the offender's own team.
    let mut s = good.clone();
    s[1].player = Some(14);
    assert_eq!(found(&s, &rules), ["player_side"]);
    let mut s = good.clone();
    s[1].secondary = Some(3);
    assert_eq!(found(&s, &rules), ["player_side"]);

    // A second yellow with no yellow before it, and a card with no card.
    let mut s = good.clone();
    s[2].card = Some(engine::Card::SecondYellow);
    assert_eq!(found(&s, &rules), ["card_kind"]);
    let mut s = good.clone();
    s[2].card = None;
    assert_eq!(found(&s, &rules), ["card_kind"]);

    // A player sent off who fouls again.
    let mut s = good.clone();
    s[2].card = Some(engine::Card::Red);
    let mut again = s[1];
    again.tick = 25;
    at(&mut s, 4, again);
    assert_eq!(found(&s, &rules), ["sent_off_silent"]);

    // A substitution that takes off a player not in the slot, and one that brings on a
    // player who has played.
    let mut s = good.clone();
    s[3].detail = Some(engine::EventDetail::Substitution { off: 5, on: 14 });
    assert_eq!(found(&s, &rules), ["substitution_squad"]);
    let mut s = good.clone();
    s[3].detail = Some(engine::EventDetail::Substitution { off: 2, on: 7 });
    assert_eq!(found(&s, &rules), ["substitution_squad"]);

    // A sixth substitution, and a fourth window; the same four windows pass when one of
    // them is the half-time stoppage.
    let sub = |tick: u32, slot: usize, on: usize| {
        let mut e = good[3];
        e.tick = tick;
        e.player = Some(11 + slot);
        e.detail = Some(engine::EventDetail::Substitution { off: slot, on });
        e
    };
    let mut s = good.clone();
    for (k, slot) in [5usize, 6, 7, 8, 9].into_iter().enumerate() {
        at(&mut s, 4 + k, sub(20, slot, 15 + k));
    }
    assert_eq!(found(&s, &rules), ["substitution_limits"]);
    // Windows at ticks 20, 21 and 22, then one at the half-time tick 30: half-time is free.
    let mut s = good.clone();
    at(&mut s, 4, sub(21, 5, 15));
    at(&mut s, 5, sub(22, 6, 16));
    assert_eq!(s[6].kind, HalfTime);
    at(&mut s, 7, sub(30, 7, 17));
    assert!(found(&s, &rules).is_empty(), "{:?}", found(&s, &rules));
    // The same substitution at tick 25 opens a fourth window.
    let mut s = good.clone();
    at(&mut s, 4, sub(21, 5, 15));
    at(&mut s, 5, sub(22, 6, 16));
    at(&mut s, 6, sub(25, 7, 17));
    assert_eq!(found(&s, &rules), ["substitution_limits"]);

    // Added seconds outside the range, on an event that is not a period end, and in a
    // match that adds no time.
    let mut s = good.clone();
    s[4].added_time_s = Some(30);
    assert_eq!(found(&s, &rules), ["added_time"]);
    let mut s = good.clone();
    s[1].added_time_s = Some(60);
    assert_eq!(found(&s, &rules), ["added_time"]);
    let short = StreamRules {
        added_s: None,
        ..planted_rules()
    };
    assert_eq!(found(&good, &short), ["added_time", "added_time"]);

    // The control: every kind in the legal stream is the kind the rules judge.
    assert_eq!(
        good.iter().map(|e| e.kind).collect::<Vec<_>>()[1..4],
        [Foul, Card, Substitution]
    );
}
