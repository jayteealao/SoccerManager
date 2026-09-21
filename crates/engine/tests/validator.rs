//! AC-c and AC-d: the validator finds no violation over a seeded full match, and reports a
//! hand-corrupted stream.

mod common;

use engine::{Simulation, Validator, VecSink, ticks_for_minutes};

#[test]
fn a_seeded_full_match_has_no_violations() {
    let config = common::full_match();
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut sink = VecSink::default();
    sim.run(ticks_for_minutes(90), &mut sink).unwrap();
    let validator = Validator::new(config.tuning.clone(), sim.teams());
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
    let config = common::full_match();
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut sink = VecSink::default();
    sim.run(ticks_for_minutes(1), &mut sink).unwrap();
    sink.records[10].players[1] = sink.records[10].players[0];
    let validator = Validator::new(config.tuning.clone(), sim.teams());
    let violations = validator.check(&sink.records);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "separation");
    assert_eq!(violations[0].tick, sink.records[10].tick);
}
