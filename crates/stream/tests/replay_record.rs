//! The version-4 replay file: its inputs, record, and frames read back as written; a file
//! read and written back is the same file byte for byte; and every damaged or inconsistent
//! file is refused by name. The command-line tests hold the recorded match itself (AC-39 to
//! AC-41); these hold the format.

mod common;

use std::path::{Path, PathBuf};

use engine::record::TickSink;
use engine::{Change, Manager, Simulation, TacticsPatch};
use protocol::{Frame, PROTOCOL_VERSION};
use stream::session::{FrameOut, FrameSink};
use stream::{
    ChangeSource, EngineIdentity, FORMAT_VERSION, InputFile, LoggedChange, ManagerKind,
    MatchSettings, Recorder, SharedRecorder, outcome_of, read_fixture, write_fixture,
};

/// The four content files, as inputs of the recorded match.
fn inputs() -> Vec<InputFile> {
    let dir = common::content_dir();
    [
        ("attributes", engine::data::ATTRIBUTES_FILE),
        ("tuning", engine::data::TUNING_FILE),
        ("rules", engine::data::RULES_FILE),
        ("tactics", engine::data::TACTICS_FILE),
    ]
    .into_iter()
    .map(|(role, name)| InputFile {
        role: role.into(),
        name: name.into(),
        bytes: std::fs::read(dir.path(name)).unwrap(),
    })
    .collect()
}

/// Records a one-minute match with both managers human, a substitution and a mentality
/// change queued at kick-off for the home team, and a substitution the away team has no
/// bench place for. Returns the path.
fn record(dir: &Path) -> PathBuf {
    let path = dir.join("match.smfx");
    let config = common::match_config(1)
        .with_manager(0, Manager::Human)
        .with_manager(1, Manager::Human);
    let settings = MatchSettings {
        seed: common::SEED,
        minutes: 1,
        knockout: false,
        managers: [ManagerKind::Human, ManagerKind::Human],
    };
    let recorder = SharedRecorder::new(
        Recorder::create_record(
            &path,
            1_700_000_000_000,
            EngineIdentity::current().unwrap(),
            settings,
            &inputs(),
        )
        .unwrap(),
    );
    let mut sink = FrameSink::new(recorder.clone(), 50);
    let mut messages = recorder.clone();
    messages
        .send(Frame::Text("{\"type\":\"hello\"}".into()))
        .unwrap();
    let mut sim = Simulation::new(config).unwrap();
    let (home, away) = (sim.teams()[0].clone(), sim.teams()[1].clone());
    sim.queue_change(
        0,
        Change::Substitution {
            off: home.lineup[9],
            on: home.bench[0],
        },
    );
    sim.queue_change(0, Change::Tactics(TacticsPatch::mentality(4)));
    sim.queue_change(
        1,
        Change::Substitution {
            off: away.lineup[9],
            on: usize::MAX,
        },
    );
    while !sim.is_over() {
        sim.step();
        sink.on_tick(&sim.record()).unwrap();
    }
    sim.finish();
    messages
        .send(Frame::Text("{\"type\":\"stats\"}".into()))
        .unwrap();
    drop(sink);
    drop(messages);
    recorder.finish_record(outcome_of(&sim)).unwrap();
    path
}

#[test]
fn a_version_four_file_reads_back_its_inputs_record_and_frames() {
    let dir = common::temp_dir("replay-record-read");
    let path = record(&dir);
    let fixture = read_fixture(&path).unwrap();
    assert_eq!(fixture.format, FORMAT_VERSION);
    assert_eq!(fixture.protocol_version, PROTOCOL_VERSION);
    assert_eq!(fixture.seed, common::SEED);
    assert_eq!(fixture.ticks, 3_000);
    // The frames hold the hello, the ticks, and the closing message; no input or record.
    assert_eq!(fixture.frames.len(), 3_002);
    assert!(fixture.frames[0].frame.is_text());

    let record = fixture
        .record
        .as_ref()
        .expect("a version-4 file has a record");
    assert_eq!(record.engine, EngineIdentity::current().unwrap());
    assert_eq!(fixture.inputs, inputs());
    assert_eq!(record.inputs.len(), 4);
    let total: u64 = inputs().iter().map(|f| f.bytes.len() as u64).sum();
    assert_eq!(record.inputs_bytes, total);
    for (listed, file) in record.inputs.iter().zip(&fixture.inputs) {
        assert_eq!(listed.role, file.role);
        assert_eq!(listed.sha256, stream::record::hex(&sha2_of(&file.bytes)));
    }

    // Both home changes applied at one stoppage, the substitution first; the away
    // substitution was rejected and is not in the log.
    let changes = &record.changes;
    assert_eq!(changes.len(), 2, "{changes:?}");
    assert!(matches!(
        changes[0].change,
        LoggedChange::Substitution { .. }
    ));
    assert!(matches!(changes[1].change, LoggedChange::Tactics { .. }));
    assert_eq!((changes[0].order, changes[1].order), (0, 1));
    assert_eq!(changes[0].tick, changes[1].tick);
    assert_eq!(changes[0].stoppage, changes[1].stoppage);
    assert!(
        changes
            .iter()
            .all(|c| c.team == 0 && c.source == ChangeSource::Manager && c.queued_tick == 0)
    );
    assert_eq!(record.watchdog.slow_calls, 0);
    assert_eq!(record.watchdog.invalid, None);

    let _ = std::fs::remove_dir_all(&dir);
}

fn sha2_of(bytes: &[u8]) -> Vec<u8> {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).to_vec()
}

#[test]
fn a_file_read_and_written_back_is_byte_identical() {
    let dir = common::temp_dir("replay-record-write-back");
    let path = record(&dir);
    let fixture = read_fixture(&path).unwrap();
    let again = dir.join("again.smfx");
    write_fixture(&again, &fixture).unwrap();
    assert_eq!(
        std::fs::read(&again).unwrap(),
        std::fs::read(&path).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Refuses `bytes` and returns the message.
fn refused(dir: &Path, name: &str, bytes: &[u8]) -> String {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    read_fixture(&path)
        .map(|_| panic!("{name} was read"))
        .unwrap_err()
        .to_string()
}

#[test]
fn every_damaged_or_inconsistent_file_is_refused_by_name() {
    let dir = common::temp_dir("replay-record-refusals");
    let path = record(&dir);
    let good = read_fixture(&path).unwrap();
    let written = |name: &str, fixture: &stream::Fixture| {
        let at = dir.join(name);
        write_fixture(&at, fixture).unwrap();
        std::fs::read(&at).unwrap()
    };

    // A changed input byte: the record's list names the file.
    let mut changed = good.clone();
    changed.inputs[1].bytes[10] ^= 0x01;
    let err = refused(&dir, "changed.smfx", &written("changed-src.smfx", &changed));
    assert!(
        err.contains("input tuning.json") && err.contains("does not match"),
        "{err}"
    );

    // No record entry.
    let mut none = good.clone();
    none.record = None;
    let err = refused(&dir, "none.smfx", &written("none-src.smfx", &none));
    assert!(err.contains("has none"), "{err}");

    // Two record entries: the record entry doubled before the trailer.
    let bytes = std::fs::read(&path).unwrap();
    let payload = serde_json::to_vec(good.record.as_ref().unwrap()).unwrap();
    let record_at = bytes.len() - 16 - payload.len() - 9;
    assert_eq!(bytes[record_at], 3, "the record entry is last");
    let mut doubled = bytes[..bytes.len() - 16].to_vec();
    doubled.extend_from_slice(&bytes[record_at..bytes.len() - 16]);
    doubled.extend_from_slice(&bytes[bytes.len() - 16..]);
    let err = refused(&dir, "doubled.smfx", &doubled);
    assert!(err.contains("two record entries"), "{err}");

    // An unknown entry kind.
    let mut unknown = bytes.clone();
    unknown[record_at] = 7;
    let err = refused(&dir, "unknown.smfx", &unknown);
    assert!(err.contains("unknown entry kind 7"), "{err}");

    // Frames of another protocol version.
    let mut protocol = bytes.clone();
    protocol[6..8].copy_from_slice(&9u16.to_le_bytes());
    let err = refused(&dir, "protocol.smfx", &protocol);
    assert!(
        err.contains("frames protocol version 9; this build speaks 3"),
        "{err}"
    );

    // Another format.
    let mut format = bytes.clone();
    format[4..6].copy_from_slice(&5u16.to_le_bytes());
    let err = refused(&dir, "format.smfx", &format);
    assert!(
        err.contains("format 5; this build reads formats 3 and 4"),
        "{err}"
    );

    // Control: the unchanged file reads.
    assert_eq!(read_fixture(&path).unwrap(), good);
    let _ = std::fs::remove_dir_all(&dir);
}
