//! AC-c and AC-d: the validator finds no violation over a seeded full match played under the
//! laws, with every restart spot checked, and reports a hand-corrupted stream. Fatigue,
//! injuries, and the AI manager's substitutions and tactics changes are on in that match.

mod common;

use engine::{Simulation, Validator, VecSink};

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

/// The full engine's event stream of a seeded 90-minute match.
fn engine_events(config: engine::MatchConfig) -> Vec<engine::EngineEvent> {
    let mut sim = Simulation::new(config).unwrap();
    sim.run(&mut engine::record::NullSink).unwrap();
    sim.take_events()
}

/// The event-stream rules hold for the full engine before they judge anything else: seeds 1
/// to 5 over 90 minutes, and a knockout match played through extra time and a shoot-out.
#[test]
fn the_full_engines_event_streams_keep_the_event_rules() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let mut streams: Vec<(String, Vec<engine::EngineEvent>)> = common::run_many(1..=5, |seed| {
        let config = engine::MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
        (format!("seed {seed}"), engine_events(config))
    });
    let knockout = engine::MatchConfig::new(1, 90, &content, [&a, &b])
        .unwrap()
        .with_knockout();
    let events = engine_events(knockout);
    assert!(
        events.iter().any(|e| e.shootout_scored == Some(true)),
        "the knockout match reached a shoot-out with a scored kick"
    );
    streams.push(("knockout seed 1".into(), events));
    for (name, events) in &streams {
        assert!(
            events
                .iter()
                .any(|e| e.kind == engine::EngineEventKind::HalfTime),
            "{name}"
        );
        let violations = Validator::check_events(events);
        assert!(violations.is_empty(), "{name}: {violations:?}");
    }
}

/// Each event-stream rule fails on one planted stream.
#[test]
fn each_event_rule_fails_on_a_planted_stream() {
    use engine::EngineEventKind::{FullTime, Goal, HalfTime, KickOff};
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
    assert!(Validator::check_events(&good).is_empty());
    let rules = |events: &[engine::EngineEvent]| -> Vec<&'static str> {
        Validator::check_events(events)
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
