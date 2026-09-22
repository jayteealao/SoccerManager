//! AC-f: a match resumed from a snapshot continues tick for tick, and a snapshot written,
//! read, and written again is the same bytes. AC-g: a damaged snapshot, or one from another
//! format version, build, or content, is refused with the reason named.

mod common;

use common::full_match;
use engine::record::RECORD_BYTES;
use engine::{EngineError, Simulation, Snapshot, Stoppage, TickRecord, TickSink, VecSink};

/// The stoppage the continuation test resumes from. The seed-42 match has 34.
const AT_STOPPAGE: u32 = 30;
const OWNER: [u8; 16] = [7; 16];

/// Keeps every record and captures the snapshot at one stoppage.
#[derive(Default)]
struct Capture {
    records: Vec<TickRecord>,
    stoppages: u32,
    snapshot: Option<Snapshot>,
}

impl TickSink for Capture {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.records.push(*record);
        Ok(())
    }

    fn on_stoppage(&mut self, _stoppage: &Stoppage, sim: &Simulation) -> Result<(), EngineError> {
        self.stoppages += 1;
        if self.stoppages == AT_STOPPAGE {
            self.snapshot = Some(Snapshot::capture(sim, OWNER, 1_700_000_000_000));
        }
        Ok(())
    }
}

fn bytes(records: &[TickRecord]) -> Vec<u8> {
    let mut out = Vec::with_capacity(records.len() * RECORD_BYTES);
    let mut buf = [0u8; RECORD_BYTES];
    for record in records {
        record.write_to(&mut buf);
        out.extend_from_slice(&buf);
    }
    out
}

fn whole_match() -> (Capture, Simulation) {
    let mut capture = Capture::default();
    let mut sim = Simulation::new(full_match()).unwrap();
    sim.run(&mut capture).unwrap();
    (capture, sim)
}

fn refusal(result: Result<Snapshot, EngineError>) -> String {
    match result {
        Err(EngineError::Snapshot { reason, .. }) => reason,
        Err(other) => panic!("refused for another reason: {other}"),
        Ok(_) => panic!("the snapshot was accepted"),
    }
}

#[test]
fn a_resumed_match_continues_tick_for_tick() {
    let (whole, whole_sim) = whole_match();
    assert!(
        whole.stoppages >= AT_STOPPAGE,
        "{} stoppages",
        whole.stoppages
    );
    let snapshot = whole.snapshot.expect("a snapshot at the chosen stoppage");
    let from = snapshot.tick();

    // Through the file format, as a resume reads it.
    let read = Snapshot::from_bytes(&snapshot.to_bytes(), "snapshot.smsn").unwrap();
    let mut resumed = Simulation::from_snapshot(full_match(), &read).unwrap();
    assert_eq!(resumed.tick(), from);
    let mut sink = VecSink::default();
    resumed.run(&mut sink).unwrap();

    let tail: Vec<TickRecord> = whole
        .records
        .iter()
        .copied()
        .filter(|r| r.tick > from)
        .collect();
    assert!(!tail.is_empty());
    assert_eq!(sink.records.len(), tail.len());
    assert!(
        bytes(&sink.records) == bytes(&tail),
        "the resumed records differ from the uninterrupted match"
    );
    assert_eq!(resumed.summary(), whole_sim.summary());
}

#[test]
fn a_snapshot_written_read_and_written_again_is_the_same_bytes() {
    let (whole, _) = whole_match();
    let first = whole.snapshot.unwrap().to_bytes();
    let read = Snapshot::from_bytes(&first, "snapshot.smsn").unwrap();
    let rebuilt = Simulation::from_snapshot(full_match(), &read).unwrap();
    let second = Snapshot::capture(&rebuilt, OWNER, 1_700_000_000_000).to_bytes();
    assert!(first == second, "a field does not round-trip");
}

#[test]
fn a_damaged_or_foreign_snapshot_is_refused_by_name() {
    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..500 {
        sim.step();
    }
    let snapshot = Snapshot::capture(&sim, OWNER, 1);
    let good = snapshot.to_bytes();
    assert!(Snapshot::from_bytes(&good, "s").is_ok());

    let mut flipped = good.clone();
    flipped[200] ^= 0x01;
    assert!(refusal(Snapshot::from_bytes(&flipped, "s")).starts_with("checksum mismatch"));

    let truncated = &good[..good.len() - 1];
    assert!(refusal(Snapshot::from_bytes(truncated, "s")).starts_with("truncated"));
    assert!(refusal(Snapshot::from_bytes(&good[..5], "s")).starts_with("truncated"));

    let mut magic = good.clone();
    magic[0] = b'X';
    assert!(refusal(Snapshot::from_bytes(&magic, "s")).starts_with("bad magic"));

    let mut version = good.clone();
    version[4] = 99;
    assert!(refusal(Snapshot::from_bytes(&version, "s")).starts_with("unknown version 99"));

    let mut other_build = snapshot.clone();
    other_build.build_hash = "0000000".into();
    let reason = refusal(Snapshot::from_bytes(&other_build.to_bytes(), "s"));
    assert!(reason.starts_with("build mismatch"), "{reason}");
    assert!(reason.contains(engine::build_hash()), "{reason}");

    let mut other_content = snapshot.clone();
    other_content.content_hash = "000000000000".into();
    match Simulation::from_snapshot(full_match(), &other_content) {
        Err(EngineError::Snapshot { reason, .. }) => {
            assert!(reason.starts_with("content mismatch"), "{reason}")
        }
        Err(other) => panic!("refused for another reason: {other}"),
        Ok(_) => panic!("a snapshot of other content was accepted"),
    }
}
