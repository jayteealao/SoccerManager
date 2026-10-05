//! AC-f: a match resumed from a snapshot continues tick for tick, and a snapshot written,
//! read, and written again is the same bytes. AC-g: a damaged snapshot, or one from another
//! format version, build, or content, is refused with the reason named. The continuation
//! runs past substitutions and an applied tactics change, so the lineups, benches, tactics,
//! ledgers, energy, and the change queue all come back from the snapshot.

mod common;

use common::{full_match, scoring_match};
use engine::record::RECORD_BYTES;
use engine::rules::clock::TICKS_PER_MINUTE;
use engine::snapshot::{MarkFixture, MatchdayMark};
use engine::{
    ChangeKind, EngineError, EngineEventKind, EventDetail, Simulation, Snapshot, Stoppage,
    TickRecord, TickSink, VecSink,
};
use sha2::{Digest, Sha256};

/// The continuation test resumes the seed-7 match from the first stoppage at or after this
/// tick, early in the second half, before the AI manager's substitutions and tactics changes.
const FROM_TICK: u32 = 50 * TICKS_PER_MINUTE;
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
        if self.snapshot.is_none() && sim.tick() >= FROM_TICK {
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
    let mut sim = Simulation::new(scoring_match()).unwrap();
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
    let (whole, mut whole_sim) = whole_match();
    let snapshot = whole.snapshot.expect("a snapshot at the chosen stoppage");
    let from = snapshot.tick();
    // The uninterrupted match changes after the snapshot: substitutions and a tactics change.
    let after: Vec<_> = whole_sim
        .take_events()
        .into_iter()
        .filter(|e| e.tick > from)
        .collect();
    assert!(
        after
            .iter()
            .any(|e| e.kind == EngineEventKind::Substitution),
        "no substitution after tick {from}"
    );
    assert!(
        after
            .iter()
            .any(|e| e.kind == EngineEventKind::ChangeApplied
                && matches!(
                    e.detail,
                    Some(EventDetail::Change {
                        kind: ChangeKind::Tactics,
                        ..
                    })
                )),
        "no applied tactics change after tick {from}"
    );

    // Through the file format, as a resume reads it.
    let read = Snapshot::from_bytes(&snapshot.to_bytes(), "snapshot.smsn").unwrap();
    let mut resumed = Simulation::from_snapshot(scoring_match(), &read).unwrap();
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
    assert_eq!(resumed.teams()[0].lineup, whole_sim.teams()[0].lineup);
    assert_eq!(resumed.teams()[1].lineup, whole_sim.teams()[1].lineup);
    assert_eq!(resumed.teams()[0].tactics, whole_sim.teams()[0].tactics);
    assert_eq!(resumed.teams()[1].tactics, whole_sim.teams()[1].tactics);
    assert_eq!(resumed.ledgers(), whole_sim.ledgers());
}

#[test]
fn a_snapshot_written_read_and_written_again_is_the_same_bytes() {
    let (whole, _) = whole_match();
    let first = whole.snapshot.unwrap().to_bytes();
    let read = Snapshot::from_bytes(&first, "snapshot.smsn").unwrap();
    let rebuilt = Simulation::from_snapshot(scoring_match(), &read).unwrap();
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
    // A snapshot from before squads, tactics, and the change queue were saved.
    let mut first_format = good.clone();
    first_format[4] = 1;
    let reason = refusal(Snapshot::from_bytes(&first_format, "s"));
    assert!(
        reason.starts_with("unknown version 1; this build reads 9"),
        "{reason}"
    );

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

/// Keeps every record and captures the first snapshot `pick` accepts.
struct CaptureWhen<F: Fn(&Stoppage, &Simulation) -> bool> {
    records: Vec<TickRecord>,
    pick: F,
    snapshot: Option<Snapshot>,
}

impl<F: Fn(&Stoppage, &Simulation) -> bool> TickSink for CaptureWhen<F> {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.records.push(*record);
        Ok(())
    }

    fn on_stoppage(&mut self, stoppage: &Stoppage, sim: &Simulation) -> Result<(), EngineError> {
        if self.snapshot.is_none() && (self.pick)(stoppage, sim) {
            self.snapshot = Some(Snapshot::capture(sim, OWNER, 1_700_000_000_000));
        }
        Ok(())
    }
}

/// A level knockout match from minute 89, hand-managed so nothing but the laws moves it.
fn knockout_config() -> engine::MatchConfig {
    let mut config = common::calm_match(90).with_knockout();
    config.managers = [engine::Manager::Human; 2];
    config
}

/// Plays the knockout match whole, snapshots it where `pick` says, resumes the snapshot
/// through the file format, and checks the resumed ticks and result equal the whole match's.
fn resumes_tick_for_tick(pick: impl Fn(&Stoppage, &Simulation) -> bool) -> Snapshot {
    let mut sink = CaptureWhen {
        records: Vec::new(),
        pick,
        snapshot: None,
    };
    let mut whole = engine::scenario::Scene::new(knockout_config())
        .at_minute(89)
        .build();
    whole.run(&mut sink).unwrap();
    let snapshot = sink.snapshot.expect("a snapshot at the chosen stoppage");
    assert!(snapshot.knockout());
    let from = snapshot.tick();
    let read = Snapshot::from_bytes(&snapshot.to_bytes(), "snapshot.smsn").unwrap();
    let mut resumed = Simulation::from_snapshot(knockout_config(), &read).unwrap();
    let mut tail = VecSink::default();
    resumed.run(&mut tail).unwrap();
    let expected: Vec<TickRecord> = sink
        .records
        .iter()
        .copied()
        .filter(|r| r.tick > from)
        .collect();
    assert!(!expected.is_empty());
    assert!(
        bytes(&tail.records) == bytes(&expected),
        "the resumed records differ from the whole match"
    );
    assert_eq!(resumed.summary(), whole.summary());
    assert_eq!(
        resumed.summary().decided_by,
        Some(engine::DecidedBy::Shootout)
    );
    // The same match configured as a regular match refuses the snapshot.
    let mut regular = knockout_config();
    regular.knockout = false;
    match Simulation::from_snapshot(regular, &read) {
        Err(EngineError::Snapshot { reason, .. }) => {
            assert!(reason.starts_with("configuration mismatch"), "{reason}")
        }
        Err(other) => panic!("refused for another reason: {other}"),
        Ok(_) => panic!("a knockout snapshot resumed as a regular match"),
    }
    snapshot
}

#[test]
fn a_snapshot_at_the_break_before_extra_time_resumes_tick_for_tick() {
    let snapshot = resumes_tick_for_tick(|stoppage, sim| {
        stoppage.kind == engine::StoppageKind::HalfTime && sim.half() == 2
    });
    assert!(snapshot.tick() > 90 * TICKS_PER_MINUTE);
}

#[test]
fn a_snapshot_at_a_shootout_kick_resumes_tick_for_tick() {
    // The third kick: the shoot-out state is part way through.
    resumes_tick_for_tick(|stoppage, sim| {
        stoppage.kind == engine::StoppageKind::Penalty
            && sim.in_shootout()
            && sim.summary().shootout_kicks == 2
    });
}

#[test]
fn a_version_three_snapshot_is_refused_by_name() {
    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..10 {
        sim.step();
    }
    let mut bytes = Snapshot::capture(&sim, OWNER, 1).to_bytes();
    bytes[4..6].copy_from_slice(&3u16.to_le_bytes());
    let reason = refusal(Snapshot::from_bytes(&bytes, "s"));
    assert!(
        reason.starts_with("unknown version 3; this build reads 9"),
        "{reason}"
    );
}

/// A snapshot taken while a player who just fouled still waits out the foul cooldown keeps
/// that cooldown: the resumed match plays on tick for tick through the cooldown's end.
#[test]
fn a_resumed_match_keeps_a_foul_cooldown_running() {
    let mut sim = Simulation::new(full_match()).unwrap();
    let mut waiting = None;
    for _ in 0..(90 * TICKS_PER_MINUTE) {
        sim.step();
        let fouled = sim
            .take_events()
            .iter()
            .any(|e| e.kind == EngineEventKind::Foul);
        if fouled && sim.stoppage().is_some() {
            let now = sim.tick();
            waiting = sim.players().iter().position(|p| p.foul_ready > now);
            break;
        }
    }
    let player = waiting.expect("a foul stopped play with its offender cooling down");
    let ready = sim.players()[player].foul_ready;
    let read =
        Snapshot::from_bytes(&Snapshot::capture(&sim, OWNER, 1).to_bytes(), "s.smsn").unwrap();
    let mut resumed = Simulation::from_snapshot(full_match(), &read).unwrap();
    assert_eq!(resumed.players()[player].foul_ready, ready);
    let span = ready - sim.tick() + 100;
    for _ in 0..span {
        sim.step();
        resumed.step();
        assert_eq!(
            bytes(&[resumed.record()]),
            bytes(&[sim.record()]),
            "tick {}",
            sim.tick()
        );
    }
}

#[test]
fn a_version_six_snapshot_is_refused_by_name() {
    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..10 {
        sim.step();
    }
    let mut bytes = Snapshot::capture(&sim, OWNER, 1).to_bytes();
    bytes[4..6].copy_from_slice(&6u16.to_le_bytes());
    let reason = refusal(Snapshot::from_bytes(&bytes, "s"));
    assert!(
        reason.starts_with("unknown version 6; this build reads 9"),
        "{reason}"
    );
}

/// The snapshot tick of the keyed resume test: the first stoppage at or after it, once the
/// match has used at least `MIN_KEYS` streams.
const KEYED_FROM_TICK: u32 = 20_000;
const MIN_KEYS: usize = 20;

/// Hashes one tick: its record and the position of every stream.
fn hash_tick(hasher: &mut Sha256, sim: &Simulation) {
    let mut buf = [0u8; RECORD_BYTES];
    sim.record().write_to(&mut buf);
    hasher.update(buf);
    hasher.update(sim.stream_state().to_bytes());
}

#[test]
fn a_keyed_snapshot_resumes_every_stream() {
    let mut whole = Simulation::new(full_match()).unwrap();
    let mut snapshot: Option<Snapshot> = None;
    let mut tail = Sha256::new();
    while !whole.is_over() {
        whole.step();
        if snapshot.is_some() {
            hash_tick(&mut tail, &whole);
        } else if whole.stoppage().is_some()
            && whole.tick() >= KEYED_FROM_TICK
            && whole.stream_state().entries.len() >= MIN_KEYS
        {
            snapshot = Some(Snapshot::capture(&whole, OWNER, 1));
        }
    }
    whole.finish();
    let snapshot = snapshot.expect("a stoppage after tick 20,000");
    let at_snapshot = Simulation::from_snapshot(full_match(), &snapshot)
        .unwrap()
        .stream_state();
    assert_eq!(at_snapshot.scheme, engine::streams::KEYED_SCHEME);
    assert!(at_snapshot.entries.len() >= MIN_KEYS);

    // Through the file format, as a resume reads it.
    let read = Snapshot::from_bytes(&snapshot.to_bytes(), "snapshot.smsn").unwrap();
    let mut resumed = Simulation::from_snapshot(full_match(), &read).unwrap();
    assert_eq!(resumed.stream_state(), at_snapshot);
    let mut resumed_tail = Sha256::new();
    while !resumed.is_over() {
        resumed.step();
        hash_tick(&mut resumed_tail, &resumed);
    }
    resumed.finish();
    assert_eq!(
        resumed_tail.finalize(),
        tail.finalize(),
        "the resumed match differs from the uninterrupted one"
    );
    assert_eq!(resumed.summary(), whole.summary());
    let end = resumed.stream_state();
    assert_eq!(end, whole.stream_state());
    let known: Vec<u64> = at_snapshot.entries.iter().map(|&(id, _)| id).collect();
    assert!(
        end.entries.iter().any(|(id, _)| !known.contains(id)),
        "a key first used after the restore"
    );
}

#[test]
fn the_snapshot_records_the_scheme_of_the_match() {
    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..500 {
        sim.step();
    }
    let snapshot = Snapshot::capture(&sim, OWNER, 1);
    let scheme = Simulation::from_snapshot(full_match(), &snapshot)
        .unwrap()
        .stream_state()
        .scheme;
    assert_eq!(scheme, engine::streams::STREAM_SCHEME);
    assert_eq!(scheme, engine::streams::KEYED_SCHEME);
}

/// The refused-save fixture of the resume browser suite: seed 42 stopped at 52:10, stamped
/// as written by release 0.1.0 (build `3ba8fed`), with an owner id of zeros.
const FIXTURE: &str = "../../e2e/match/fixtures/saved-0.1.0.smsn";
const FIXTURE_TICK: u32 = (52 * 60 + 10) * engine::TICKS_PER_SECOND;

fn stamped(sim: &Simulation, version: &str, build: &str) -> Snapshot {
    let mut snapshot = Snapshot::capture(sim, [0; 16], 1_700_000_000_000);
    snapshot.engine_version = version.into();
    snapshot.build_hash = build.into();
    snapshot
}

#[test]
fn a_snapshot_records_the_engine_version_and_a_display_summary() {
    let mut sim = Simulation::new(scoring_match()).unwrap();
    while sim.summary().goals == [0, 0] {
        sim.step();
    }
    let snapshot = Snapshot::capture(&sim, OWNER, 1);
    assert_eq!(snapshot.engine_version, engine::version());
    let bytes = snapshot.to_bytes();
    let read = Snapshot::from_bytes(&bytes, "s").unwrap();
    assert_eq!(read, snapshot);
    let id = Snapshot::identify(&bytes).unwrap();
    assert_eq!(id.format, engine::snapshot::VERSION);
    assert_eq!(id.format, 9);
    assert_eq!(id.engine_version.as_deref(), Some(engine::version()));
    assert_eq!(id.build_hash, engine::build_hash());
    assert_eq!(id.tick, Some(sim.tick()));
    let goals = sim.summary().goals;
    assert_eq!(id.score, Some([goals[0] as u8, goals[1] as u8]));
    let names = [
        sim.config().teams[0].name.clone(),
        sim.config().teams[1].name.clone(),
    ];
    assert_eq!(id.teams, Some(names));
}

/// The identity sits at the same offsets in every version from 8 on, so a later version that
/// grows the header still names its writer to an older launcher.
#[test]
fn the_identity_offsets_are_fixed() {
    use engine::snapshot::{
        AWAY_AT, ENGINE_VERSION_AT, ENGINE_VERSION_BYTES, HOME_AT, SCORE_AT, TEAM_BYTES, TICK_AT,
    };
    assert_eq!(
        (ENGINE_VERSION_AT, ENGINE_VERSION_BYTES, TICK_AT, SCORE_AT),
        (64, 32, 96, 100)
    );
    assert_eq!((HOME_AT, AWAY_AT, TEAM_BYTES), (104, 148, 44));

    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..300 {
        sim.step();
    }
    let bytes = stamped(&sim, "0.3.0", "abcdef0").to_bytes();
    assert_eq!(&bytes[ENGINE_VERSION_AT..ENGINE_VERSION_AT + 5], b"0.3.0");
    assert_eq!(&bytes[TICK_AT..TICK_AT + 4], &300u32.to_le_bytes());

    // A later version with 16 more header bytes: the identity still reads.
    let header_len = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    assert_eq!(header_len, 192 + EMPTY_MARK_BYTES);
    let body_len = bytes.len() - header_len - 40;
    let mut later = bytes[..header_len].to_vec();
    later[4..6].copy_from_slice(&10u16.to_le_bytes());
    later[6..8].copy_from_slice(&((header_len + 16) as u16).to_le_bytes());
    later.extend_from_slice(&[0u8; 16]);
    later.extend_from_slice(&bytes[header_len..header_len + body_len]);
    let digest: [u8; 32] = Sha256::digest(&later).into();
    later.extend_from_slice(b"SMSE");
    later.extend_from_slice(&(body_len as u32).to_le_bytes());
    later.extend_from_slice(&digest);
    let id = Snapshot::identify(&later).unwrap();
    assert_eq!(id.format, 10);
    assert_eq!(id.engine_version.as_deref(), Some("0.3.0"));
    assert_eq!(id.tick, Some(300));
    let reason = refusal(Snapshot::from_bytes(&later, "s"));
    assert!(
        reason.starts_with("unknown version 10; this build reads 9 (written by Touchline 0.3.0"),
        "{reason}"
    );
}

/// The header block of a snapshot with no round: its length, the round seed, the reveal tick
/// and a fixture count of 0.
const EMPTY_MARK_BYTES: usize = 2 + 8 + 4 + 1;

fn mark() -> MatchdayMark {
    MatchdayMark {
        round_seed: 0x1234_5678_9abc_def0,
        reveal_tick: 300,
        fixtures: (0..4u8)
            .map(|i| MarkFixture {
                clubs: [
                    format!("club-000007ea-0{i}"),
                    format!("club-000007ea-0{}", i + 4),
                ],
                digests: [[i; 32], [i + 10; 32]],
            })
            .collect(),
    }
}

#[test]
fn a_matchday_mark_round_trips() {
    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..300 {
        sim.step();
    }
    let snapshot = Snapshot::capture(&sim, OWNER, 1).with_matchday(mark());
    let bytes = snapshot.to_bytes();
    let read = Snapshot::from_bytes(&bytes, "s").unwrap();
    assert_eq!(read.matchday, Some(mark()));
    assert_eq!(read, snapshot);
    assert_eq!(read.to_bytes(), bytes, "written again, the same bytes");
    // The match itself is untouched by the mark.
    let rebuilt = Simulation::from_snapshot(full_match(), &read).unwrap();
    assert_eq!(rebuilt.tick(), 300);
    let header_len = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    let per_fixture = 2 * (1 + 16 + 32);
    assert_eq!(header_len, 192 + EMPTY_MARK_BYTES + 4 * per_fixture);
}

#[test]
fn a_snapshot_with_no_round_writes_an_empty_mark() {
    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..50 {
        sim.step();
    }
    let plain = Snapshot::capture(&sim, OWNER, 1);
    let empty = plain.clone().with_matchday(MatchdayMark::default());
    assert_eq!(empty.matchday, None, "a mark with no fixture is no round");
    let bytes = plain.to_bytes();
    let header_len = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    assert_eq!(header_len, 192 + EMPTY_MARK_BYTES);
    assert_eq!(
        &bytes[192..194],
        &((EMPTY_MARK_BYTES - 2) as u16).to_le_bytes()
    );
    assert_eq!(
        bytes[192 + EMPTY_MARK_BYTES - 1],
        0,
        "the fixture count is 0"
    );
    assert_eq!(Snapshot::from_bytes(&bytes, "s").unwrap().matchday, None);
}

/// Version 9 only appends: the engine version, the tick, the score and the names stay where
/// version 8 put them, so the version resolver reads both alike.
#[test]
fn the_version_8_offsets_hold_in_version_9() {
    use engine::snapshot::{AWAY_AT, ENGINE_VERSION_AT, HOME_AT, SCORE_AT, TICK_AT};
    assert_eq!(
        (ENGINE_VERSION_AT, TICK_AT, SCORE_AT, HOME_AT, AWAY_AT),
        (64, 96, 100, 104, 148)
    );
    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..300 {
        sim.step();
    }
    let bytes = stamped(&sim, "0.3.0", "abcdef0")
        .with_matchday(mark())
        .to_bytes();
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 9);
    assert_eq!(&bytes[ENGINE_VERSION_AT..ENGINE_VERSION_AT + 5], b"0.3.0");
    assert_eq!(&bytes[TICK_AT..TICK_AT + 4], &300u32.to_le_bytes());
    let id = Snapshot::identify(&bytes).unwrap();
    assert_eq!(id.format, 9);
    assert_eq!(id.engine_version.as_deref(), Some("0.3.0"));
    assert_eq!(id.tick, Some(300));
    assert_eq!(id.seed, Some(sim.seed()));
    assert_eq!(id.writer(), "Touchline 0.3.0 (build abcdef0)");
}

/// A version 8 file, written before the matchday mark, still reads: it has no round.
#[test]
fn a_version_8_snapshot_still_reads_with_no_round() {
    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..300 {
        sim.step();
    }
    let bytes = Snapshot::capture(&sim, OWNER, 1).to_bytes();
    // The same file in the layout of version 8: the fixed 192 bytes, then the body.
    let header_len = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    let body_len = bytes.len() - header_len - 40;
    let mut v8 = bytes[..192].to_vec();
    v8[4..6].copy_from_slice(&8u16.to_le_bytes());
    v8[6..8].copy_from_slice(&192u16.to_le_bytes());
    v8.extend_from_slice(&bytes[header_len..header_len + body_len]);
    let digest: [u8; 32] = Sha256::digest(&v8).into();
    v8.extend_from_slice(b"SMSE");
    v8.extend_from_slice(&(body_len as u32).to_le_bytes());
    v8.extend_from_slice(&digest);
    let read = Snapshot::from_bytes(&v8, "s").unwrap();
    assert_eq!(read.matchday, None);
    assert_eq!(read.tick(), 300);
    assert_eq!(Snapshot::identify(&v8).unwrap().format, 8);
}

#[test]
fn a_damaged_matchday_mark_is_refused() {
    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..300 {
        sim.step();
    }
    let bytes = Snapshot::capture(&sim, OWNER, 1)
        .with_matchday(mark())
        .to_bytes();
    // One flipped byte inside a club id: the checksum refuses it.
    let mut flipped = bytes.clone();
    flipped[192 + EMPTY_MARK_BYTES + 3] ^= 1;
    assert!(refusal(Snapshot::from_bytes(&flipped, "s")).starts_with("checksum mismatch"));
    // A block whose length disagrees with the header, with a checksum that matches.
    let header_len = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    let mut short = bytes[..bytes.len() - 40].to_vec();
    short[192] = short[192].wrapping_sub(1);
    let body_len = bytes.len() - header_len - 40;
    let digest: [u8; 32] = Sha256::digest(&short).into();
    short.extend_from_slice(b"SMSE");
    short.extend_from_slice(&(body_len as u32).to_le_bytes());
    short.extend_from_slice(&digest);
    let reason = refusal(Snapshot::from_bytes(&short, "s"));
    assert!(reason.starts_with("malformed header"), "{reason}");
}

/// A file in the layout of versions 6 and 7: a 64-byte header, then the body.
fn old_layout(format: u16, build: &str, body: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 64];
    out[0..4].copy_from_slice(b"SMSN");
    out[4..6].copy_from_slice(&format.to_le_bytes());
    out[8..8 + build.len()].copy_from_slice(build.as_bytes());
    out.extend_from_slice(body);
    let digest: [u8; 32] = Sha256::digest(&out).into();
    out.extend_from_slice(b"SMSE");
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&digest);
    out
}

#[test]
fn older_files_are_named_through_the_released_builds() {
    let mut body = vec![0u8; 120];
    body[76..80].copy_from_slice(&156_500u32.to_le_bytes());
    for (format, build, named) in [
        (7, "669f68b", Some("0.2.0-beta.1")),
        (6, "3ba8fed", Some("0.1.0")),
        (7, "669f68bb3229", Some("0.2.0-beta.1")),
        (7, "669f68b-dirty", None),
        (7, "669f68", None),
        (7, "1234567", None),
        (7, "unknown", None),
    ] {
        let id = Snapshot::identify(&old_layout(format, build, &body)).unwrap();
        assert_eq!(id.format, format);
        assert_eq!(id.build_hash, build);
        assert_eq!(id.engine_version.as_deref(), named, "{build}");
        assert_eq!(id.tick, Some(156_500));
        assert_eq!(id.teams, None);
    }
    let id = Snapshot::identify(&old_layout(7, "1234567", &body)).unwrap();
    assert_eq!(id.writer(), "an unreleased build 1234567");

    // The strict reader refuses a version 7 file and names its writer.
    let reason = refusal(Snapshot::from_bytes(&old_layout(7, "669f68b", &body), "s"));
    assert_eq!(
        reason,
        "unknown version 7; this build reads 9 (written by Touchline 0.2.0-beta.1 (build 669f68b))"
    );

    // A damaged or too old file is not identified.
    let mut damaged = old_layout(7, "669f68b", &body);
    damaged[100] ^= 1;
    assert!(
        Snapshot::identify(&damaged)
            .unwrap_err()
            .starts_with("checksum mismatch")
    );
    let mut ancient = old_layout(7, "669f68b", &body);
    ancient[4] = 3;
    assert!(
        Snapshot::identify(&ancient)
            .unwrap_err()
            .starts_with("unknown version 3")
    );
    assert!(
        Snapshot::identify(b"XXXX")
            .unwrap_err()
            .starts_with("bad magic")
    );
}

#[test]
fn a_build_mismatch_names_both_engine_versions() {
    let mut sim = Simulation::new(full_match()).unwrap();
    for _ in 0..50 {
        sim.step();
    }
    let reason = refusal(Snapshot::from_bytes(
        &stamped(&sim, "0.2.0-beta.1", "669f68b").to_bytes(),
        "s",
    ));
    assert_eq!(
        reason,
        format!(
            "build mismatch: written by Touchline 0.2.0-beta.1 (build 669f68b); this build is \
             Touchline {} (build {})",
            engine::version(),
            engine::build_hash()
        )
    );
}

/// Writes the fixture with `SM_WRITE_FIXTURES=1`; otherwise checks that the committed file
/// names release 0.1.0 and the match at 52:10.
#[test]
fn the_refused_save_fixture_names_release_0_1_0() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    if std::env::var_os("SM_WRITE_FIXTURES").is_some_and(|v| v == "1") {
        let mut sim = Simulation::new(full_match()).unwrap();
        while sim.tick() < FIXTURE_TICK {
            sim.step();
        }
        std::fs::write(&path, stamped(&sim, "0.1.0", "3ba8fed").to_bytes()).unwrap();
    }
    let bytes = std::fs::read(&path).expect("the committed fixture");
    let id = Snapshot::identify(&bytes).unwrap();
    assert_eq!(id.format, 8);
    assert_eq!(id.engine_version.as_deref(), Some("0.1.0"));
    assert_eq!(id.build_hash, "3ba8fed");
    assert_eq!(id.tick, Some(FIXTURE_TICK));
    let config = full_match();
    assert_eq!(
        id.teams,
        Some([config.teams[0].name.clone(), config.teams[1].name.clone()])
    );
    assert_eq!(&bytes[36..52], &[0u8; 16], "the owner id is zeros");
    assert_eq!(id.seed, Some(42));
    assert_eq!(id.match_millis, 1_700_000_000_000);
    let reason = refusal(Snapshot::from_bytes(&bytes, "saved-0.1.0.smsn"));
    assert!(
        reason.starts_with("build mismatch: written by Touchline 0.1.0 (build 3ba8fed)"),
        "{reason}"
    );
}
