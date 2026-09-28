//! The debug trace: every random draw of a gate match is recorded with its stream, key,
//! index, value, and, at a chance draw, the probability it is tested against (AC-32).
//! Controls prove the checker fails on a dropped draw and on a missing probability.
#![cfg(feature = "debug-trace")]

mod common;

use std::sync::OnceLock;

use engine::data::TeamFile;
use engine::gate::{self, Fixture, Inputs, Played};
use engine::trace::{self, TraceRecord};
use engine::{Content, EngineEvent};

fn loaded() -> &'static (Content, [TeamFile; 2]) {
    static LOADED: OnceLock<(Content, [TeamFile; 2])> = OnceLock::new();
    LOADED.get_or_init(|| {
        let content = common::content();
        let teams = common::default_teams(&content);
        (content, teams)
    })
}

fn inputs() -> Inputs<'static> {
    let (content, [a, b]) = loaded();
    Inputs {
        content,
        teams: [a, b],
        pack: None,
    }
}

/// The seed-42 gate match played traced: its result, its records, its events, and the
/// registry's draw count. Played once per test binary.
fn traced() -> &'static (Played, Vec<TraceRecord>, Vec<EngineEvent>, u64) {
    static TRACED: OnceLock<(Played, Vec<TraceRecord>, Vec<EngineEvent>, u64)> = OnceLock::new();
    TRACED.get_or_init(|| {
        let mut records = Vec::new();
        let mut events = Vec::new();
        let (played, draws) = gate::play_traced(&Fixture::seed(42), &inputs(), |r, e| {
            records.extend_from_slice(r);
            events.extend_from_slice(e);
        })
        .unwrap();
        (played, records, events, draws)
    })
}

#[test]
fn every_draw_of_a_gate_match_is_recorded_with_its_fields() {
    let (_, records, _, registry) = traced();
    assert!(
        *registry > 100_000,
        "a 90-minute match takes {registry} draws"
    );
    trace::check_draws(records, *registry).unwrap();
    let draws: Vec<_> = records
        .iter()
        .filter(|r| matches!(r, TraceRecord::Draw(_)))
        .collect();
    assert_eq!(draws.len() as u64, *registry);
    // The file form of every draw has the fields AC-32 names.
    let mut chance = 0;
    for r in &draws {
        let v = r.to_json();
        assert_eq!(v["k"], "draw");
        for field in [
            "t",
            "subsystem",
            "stream_id",
            "key",
            "index",
            "value",
            "scripted",
        ] {
            assert!(!v[field].is_null(), "{field} missing in {v}");
        }
        assert!(v["stream_id"].as_str().unwrap().starts_with("0x"));
        if !v["p"].is_null() {
            chance += 1;
            assert!(
                v["p"]
                    .as_array()
                    .is_some_and(|p| (1..=2).contains(&p.len()))
            );
        }
    }
    assert!(chance > 0, "the match took chance draws");
    // Ticks never go back.
    assert!(records.windows(2).all(|w| w[0].tick() <= w[1].tick()));
}

#[test]
fn the_traced_match_keeps_the_untraced_hashes() {
    let (played, _, _, _) = traced();
    let plain = gate::play_fixture(&Fixture::seed(42), &inputs()).unwrap();
    assert_eq!(played.hashes, plain.hashes);
}

#[test]
fn the_draw_checker_fails_on_a_dropped_draw() {
    let (_, records, _, registry) = traced();
    let at = records
        .iter()
        .position(|r| matches!(r, TraceRecord::Draw(_)))
        .unwrap()
        + 1000;
    let mut dropped = records.clone();
    // The 1,000th record after the first draw, or the next draw after it.
    let k = (at..dropped.len())
        .find(|&k| matches!(dropped[k], TraceRecord::Draw(_)))
        .unwrap();
    dropped.remove(k);
    let err = trace::check_draws(&dropped, *registry).unwrap_err();
    assert!(
        err.contains("index") || err.contains("draws recorded"),
        "{err}"
    );
    // The last draw leaves no index gap: the count alone catches it.
    let mut last = records.clone();
    let k = last
        .iter()
        .rposition(|r| matches!(r, TraceRecord::Draw(_)))
        .unwrap();
    last.remove(k);
    let err = trace::check_draws(&last, *registry).unwrap_err();
    assert!(err.contains("draws recorded"), "{err}");
}

#[test]
fn the_draw_checker_fails_on_a_missing_probability() {
    let (_, records, _, registry) = traced();
    let mut stripped = records.clone();
    let k = stripped
        .iter()
        .position(|r| matches!(r, TraceRecord::Draw(d) if !d.thresholds.is_empty()))
        .unwrap();
    if let TraceRecord::Draw(d) = &mut stripped[k] {
        d.thresholds.clear();
    }
    let err = trace::check_draws(&stripped, *registry).unwrap_err();
    assert!(err.contains("has no probability"), "{err}");
}

#[test]
fn an_untraced_match_records_nothing() {
    let config = common::short_match(2);
    let mut sim = engine::Simulation::new(config).unwrap();
    assert!(!sim.debug_trace_on());
    while !sim.is_over() {
        sim.step();
        assert!(sim.take_trace().is_empty());
    }
}

#[test]
fn the_header_names_the_maths_library_in_the_lock_file() {
    let lock = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock"),
    )
    .unwrap();
    let version = trace::MATHS.strip_prefix("libm ").unwrap();
    assert!(
        lock.contains(&format!("name = \"libm\"\nversion = \"{version}\"")),
        "Cargo.lock has another libm than {}",
        trace::MATHS
    );
    let header = trace::TraceHeader { seed: 7, scheme: 1 }.to_json();
    assert_eq!(header["trace_version"], 1);
    assert_eq!(header["seed"], 7);
    assert_eq!(header["maths"], trace::MATHS);
    assert_eq!(header["engine"], engine::build_hash());
}
