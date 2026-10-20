//! The version-4 replay file: its inputs, record, and frames read back as written; a file
//! read and written back is the same file byte for byte; and every damaged or inconsistent
//! file is refused by name. The command-line tests hold the recorded match itself (the
//! re-simulation, the stored fields, and the identity check); these hold the format.

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
        game_state: None,
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
    let dir = common::temp_dir("replay-file-read");
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
    let dir = common::temp_dir("replay-file-write-back");
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
    let dir = common::temp_dir("replay-file-refusals");
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
    // The trailer hash is written again over both, so the entry order is what refuses it.
    let mut raw = stream::RawReplay::parse(&bytes).unwrap();
    raw.entries.push(raw.entries.last().unwrap().clone());
    let doubled = raw.to_bytes();
    let err = refused(&dir, "doubled.smfx", &doubled);
    assert!(err.contains("two record entries"), "{err}");

    // An unknown entry kind.
    let mut unknown = bytes.clone();
    unknown[record_at] = 7;
    let err = refused(&dir, "unknown.smfx", &unknown);
    assert!(err.contains("unknown entry kind 7"), "{err}");

    // Frames of another protocol version.
    let mut protocol = bytes.clone();
    protocol[6..8].copy_from_slice(&10u16.to_le_bytes());
    let err = refused(&dir, "protocol.smfx", &protocol);
    assert!(
        err.contains("frames protocol version 10; this build reads 3, 4, 5, 6, 7, 8 and 9"),
        "{err}"
    );

    // Another format.
    let mut format = bytes.clone();
    format[4..6].copy_from_slice(&5u16.to_le_bytes());
    let err = refused(&dir, "format.smfx", &format);
    assert!(
        err.contains("format 5 is newer than this reader knows; this build reads formats 3 and 4"),
        "{err}"
    );

    // Control: the unchanged file reads.
    assert_eq!(read_fixture(&path).unwrap(), good);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A replay of protocol 6 still plays back: its frames decode, and its hello, whose squad
/// entries lack the protocol 7 person fields, reads as a current hello with those fields empty.
#[test]
fn a_protocol_6_replay_reads_and_its_hello_lacks_only_the_person_fields() {
    let dir = common::temp_dir("replay-file-protocol-6");
    let path = record(&dir);
    let bytes = std::fs::read(&path).unwrap();
    let mut raw = stream::RawReplay::parse(&bytes).unwrap();
    assert_eq!(raw.protocol, PROTOCOL_VERSION);
    raw.protocol = 6;
    let squad_entry = serde_json::json!({
        "player.id": "a-1",
        "player.name": "Keeper One",
        "player.shirt": 1,
        "player.position": "GK",
        "player.natural_fitness": 142,
        "player.consistency": { "confidence": "not_yet_known" },
        "player.injury_proneness": { "confidence": "not_yet_known" },
        "role_fit": [160],
        "player.attributes": { "pace": 142 },
        "player.level": 130,
        "player.plays_between": [124, 130]
    });
    let hello = serde_json::json!({
        "type": "hello",
        "protocol.version": 6,
        "engine.version": "0.2.0-beta.4",
        "build.hash": "abc1234",
        "owner.id": "0123456789abcdef0123456789abcdef",
        "match.id": "000000000000002a-1700000000000",
        "seed": 42,
        "dt_ms": 20.0,
        "ticks_expected": 3000,
        "keyframe_interval": 50,
        "teams": [
            { "team.id": "a", "team.name": "A", "team.kit.primary": "#c8102e",
              "team.kit.secondary": "#000000", "squad": [squad_entry] },
            { "team.id": "b", "team.name": "B", "team.kit.primary": "#6a0dad",
              "team.kit.secondary": "#ff6a13" }
        ],
        "tactics": null
    });
    let at = raw
        .entries
        .iter()
        .position(|e| e.payload.starts_with(b"{\"type\":\"hello\""))
        .expect("the recorded file holds a hello");
    raw.entries[at].payload = serde_json::to_vec(&hello).unwrap();
    let old = dir.join("protocol-6.smfx");
    std::fs::write(&old, raw.to_bytes()).unwrap();

    let fixture = read_fixture(&old).unwrap();
    assert_eq!(fixture.protocol_version, 6);
    let Frame::Text(text) = &fixture.frames[0].frame else {
        panic!("the first frame is the hello");
    };
    let protocol::ServerMessage::Hello(read) = serde_json::from_str(text).unwrap() else {
        panic!("the first frame reads as a hello");
    };
    assert_eq!(read.protocol_version, 6);
    let entry = &read.teams[0].squad[0];
    assert_eq!(entry.level, Some(130));
    assert!(entry.positions.is_empty());
    assert_eq!(
        (entry.strong_foot.as_deref(), entry.weaker_foot),
        (None, None)
    );
    assert!(entry.traits.is_empty());
    assert!(entry.personality.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
