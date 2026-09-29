//! The named parts of the per-tick match state: the field marks cover every byte of a tick,
//! once each and in order, every name is unique within a tick, and the bytes are the same
//! with the marks on or off, so the gate hash does not depend on them. Between two
//! consecutive ticks of a running match, the tick and the ball change and the managers and
//! the team shapes do not, which shows the names point at the right bytes.

mod common;

use std::collections::HashSet;

use engine::Simulation;
use engine::gate::{FieldKind, StateField, StateWriter};

/// One kept tick: the tick, its bytes with the marks on, its fields, and its bytes with the
/// marks off.
type Kept = (u32, Vec<u8>, Vec<StateField>, Vec<u8>);

/// Plays a one-minute seed-42 match and keeps each tick in `keep`.
fn ticks(keep: &[u32]) -> Vec<Kept> {
    let mut sim = Simulation::new(common::short_match(1)).unwrap();
    let mut on = StateWriter::new(true);
    let mut off = StateWriter::new(false);
    let mut out = Vec::new();
    while !sim.is_over() {
        sim.step();
        if sim.is_over() {
            sim.finish();
        }
        let events = sim.take_events();
        let marked = on.tick(&sim, &events).to_vec();
        let plain = off.tick(&sim, &events).to_vec();
        if keep.contains(&sim.tick()) {
            out.push((sim.tick(), marked, on.fields(), plain));
        }
    }
    assert_eq!(on.non_finite(), None);
    out
}

fn value<'a>(bytes: &'a [u8], fields: &[StateField], name: &str) -> &'a [u8] {
    let field = fields
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no field {name}"));
    &bytes[field.range.clone()]
}

#[test]
fn the_fields_cover_every_byte_once_with_unique_names_and_the_bytes_do_not_change() {
    let all: Vec<u32> = (1..=3_000).step_by(499).collect();
    let seen = ticks(&all);
    assert_eq!(seen.len(), all.len());
    for (tick, marked, fields, plain) in &seen {
        assert_eq!(marked, plain, "tick {tick}: marks change the bytes");
        assert_eq!(fields.first().map(|f| f.range.start), Some(0));
        assert_eq!(fields.last().map(|f| f.range.end), Some(marked.len()));
        for pair in fields.windows(2) {
            assert_eq!(pair[0].range.end, pair[1].range.start, "tick {tick}");
        }
        let names: HashSet<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names.len(), fields.len(), "tick {tick}: a name repeats");
        for name in [
            "tick",
            "ball.vel",
            "players[0].pos",
            "players[21].derived",
            "streams",
            "events",
        ] {
            assert!(names.contains(name), "tick {tick}: no {name}");
        }
        let kind = |name: &str| fields.iter().find(|f| f.name == name).unwrap().kind;
        assert_eq!(kind("ball.vel"), FieldKind::Floats);
        assert_eq!(kind("streams"), FieldKind::Streams);
        assert_eq!(kind("summary"), FieldKind::Bytes);
        assert_eq!(value(marked, fields, "tick"), tick.to_le_bytes());
        assert_eq!(value(marked, fields, "ball.vel").len(), 24);
    }
}

#[test]
fn between_two_ticks_the_tick_and_the_ball_change_and_the_managers_do_not() {
    let seen = ticks(&[100, 101]);
    let [(_, a, fa, _), (_, b, fb, _)] = &seen[..] else {
        panic!("two ticks expected");
    };
    let names = |f: &[StateField]| f.iter().map(|f| f.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(fa), names(fb));
    let differ: Vec<&str> = fa
        .iter()
        .filter(|f| value(a, fa, &f.name) != value(b, fb, &f.name))
        .map(|f| f.name.as_str())
        .collect();
    assert!(differ.contains(&"tick"), "{differ:?}");
    assert!(differ.contains(&"ball.pos"), "{differ:?}");
    assert!(!differ.contains(&"managers"), "{differ:?}");
    assert!(!differ.contains(&"teams[0]"), "{differ:?}");
}
