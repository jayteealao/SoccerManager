//! The replay file (`.smfx`): the wire bytes of one whole match, exactly as they were
//! sent. A replay writes them back with no re-encoding, so the replayed stream is
//! byte-identical by construction and a viewer can be verified against a fixed recording.
//!
//! Two formats exist. Format 3, the legacy file, holds the frames only. Format 4 also holds
//! every match input by value (one input entry per file, before the frames) and one record
//! entry at the end: the full engine identity, the match settings, the list of inputs with
//! their SHA-256, the applied-change log, and the watchdog mark. A format-4 file is enough to
//! re-simulate its match on the engine that recorded it.
//!
//! Layout, little-endian: a 32-byte header (magic `SMFX`, the format version u16, the
//! frames' protocol version u16 (zero in format 3, whose format field is the protocol
//! version), the match stamp u64, the frame count u32, the tick count u32, the seed u64);
//! entries of kind (u8), tick (u32), length (u32), and payload; a 16-byte trailer (magic
//! `SMFE`, the frame count u32, the first six bytes of the SHA-256 over every payload in
//! file order, two zero bytes). The frame count counts tick and text frames only.

use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use engine::data::hex12;
use engine::{AppliedChange, Change, Manager, RoleDuty, Simulation, TacticsPatch};
use protocol::{Frame, PROTOCOL_VERSION, TickFrame};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::StreamError;
use crate::migrate::{Chain, RawEntry, RawReplay};
use crate::session::FrameOut;

/// Magic of a fixture file.
pub const FIXTURE_MAGIC: &[u8; 4] = b"SMFX";
/// Magic of the trailer.
pub const FIXTURE_TRAILER_MAGIC: &[u8; 4] = b"SMFE";
/// Header bytes.
pub const FIXTURE_HEADER_BYTES: usize = 32;
/// Trailer bytes.
pub const FIXTURE_TRAILER_BYTES: usize = 16;
/// The legacy format: frames only, of protocol version 3. It plays and never re-simulates.
pub const FORMAT_LEGACY: u16 = 3;
/// The format this build writes for a recorded match: frames, inputs, and the record.
pub const FORMAT_VERSION: u16 = 4;
/// Header offset of the frame count.
const FRAMES_AT: u64 = 16;
/// A binary tick frame.
pub(crate) const ENTRY_BINARY: u8 = 0;
/// A JSON text frame.
pub(crate) const ENTRY_TEXT: u8 = 1;
/// One input file: name length (u16), the UTF-8 name, and the file's bytes.
const ENTRY_INPUT: u8 = 2;
/// The record: one JSON document.
const ENTRY_RECORD: u8 = 3;

/// What a finished fixture holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSummary {
    pub frames: u32,
    pub ticks: u32,
    pub bytes: u64,
    pub hash: String,
}

/// One match input, stored by value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputFile {
    /// What the file is to the match: `attributes`, `tuning`, `rules`, `tactics`,
    /// `commentary`, `team_a`, `team_b`, `pack_manifest`, or `pack_script`.
    pub role: String,
    /// The file's name as it was loaded, relative to its folder.
    pub name: String,
    pub bytes: Vec<u8>,
}

/// The engine that recorded a match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineIdentity {
    /// The full git commit, or `unknown`.
    pub commit: String,
    /// `true` when the working tree held changes at build time.
    pub dirty: bool,
    pub crate_version: String,
    /// The random-stream scheme.
    pub scheme: u8,
    /// The maths library and its version.
    pub maths: String,
    /// SHA-256 of the executable file, 64 hex characters.
    pub executable_sha256: String,
    /// The short build hash, as the hello names it.
    pub build: String,
}

impl EngineIdentity {
    /// The running binary's identity. The executable is read once, to hash it.
    pub fn current() -> Result<Self, StreamError> {
        // nosemgrep: rust.lang.security.current-exe.current-exe -- only hashes the running program to name it in the replay file; not a security decision
        let exe = std::env::current_exe()
            .map_err(|e| StreamError::io("cannot find the running executable", e))?;
        let mut file = File::open(&exe)
            .map_err(|e| StreamError::io("cannot read the running executable", e))?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)
            .map_err(|e| StreamError::io("cannot read the running executable", e))?;
        Ok(Self {
            commit: engine::commit().to_string(),
            dirty: engine::dirty(),
            crate_version: engine::version().to_string(),
            scheme: engine::rng::STREAM_SCHEME,
            maths: engine::trace::MATHS.to_string(),
            executable_sha256: hex(&hasher.finalize()),
            build: engine::build_hash().to_string(),
        })
    }
}

/// Who manages a team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagerKind {
    Ai,
    Human,
}

impl ManagerKind {
    pub fn of(manager: Manager) -> Self {
        match manager {
            Manager::Ai => ManagerKind::Ai,
            Manager::Human => ManagerKind::Human,
        }
    }

    pub fn manager(self) -> Manager {
        match self {
            ManagerKind::Ai => Manager::Ai,
            ManagerKind::Human => Manager::Human,
        }
    }
}

/// The match settings a re-simulation needs beyond the input files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchSettings {
    /// A decimal string: a 64-bit seed does not fit a JavaScript number.
    #[serde(with = "decimal")]
    pub seed: u64,
    pub minutes: u32,
    pub knockout: bool,
    /// Home first.
    pub managers: [ManagerKind; 2],
}

/// One input as the record lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputInfo {
    pub role: String,
    pub name: String,
    pub bytes: u64,
    /// 64 hex characters.
    pub sha256: String,
}

/// One role and duty a tactics change sets, by squad index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggedRole {
    pub squad: usize,
    pub role: u8,
    pub duty: u8,
}

/// A change as the log stores it. Players are named by squad index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LoggedChange {
    Substitution {
        off: usize,
        on: usize,
    },
    Tactics {
        #[serde(deserialize_with = "required::deserialize")]
        formation: Option<u8>,
        #[serde(deserialize_with = "required::deserialize")]
        mentality: Option<u8>,
        /// Six levels, in the tactics file's instruction order.
        instructions: [Option<u8>; 6],
        roles: Vec<LoggedRole>,
    },
}

impl From<&Change> for LoggedChange {
    fn from(change: &Change) -> Self {
        match change {
            Change::Substitution { off, on } => LoggedChange::Substitution { off: *off, on: *on },
            Change::Tactics(patch) => LoggedChange::Tactics {
                formation: patch.formation,
                mentality: patch.mentality,
                instructions: patch.instructions,
                roles: patch
                    .roles
                    .iter()
                    .map(|&(squad, rd)| LoggedRole {
                        squad,
                        role: rd.role,
                        duty: rd.duty,
                    })
                    .collect(),
            },
        }
    }
}

impl LoggedChange {
    /// The engine change this entry names.
    pub fn to_change(&self) -> Change {
        match self {
            LoggedChange::Substitution { off, on } => Change::Substitution { off: *off, on: *on },
            LoggedChange::Tactics {
                formation,
                mentality,
                instructions,
                roles,
            } => Change::Tactics(TacticsPatch {
                formation: *formation,
                mentality: *mentality,
                instructions: *instructions,
                roles: roles
                    .iter()
                    .map(|r| {
                        (
                            r.squad,
                            RoleDuty {
                                role: r.role,
                                duty: r.duty,
                            },
                        )
                    })
                    .collect(),
            }),
        }
    }
}

/// Where an applied change came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSource {
    /// The manager of a human-managed team; re-simulation queues it again.
    Manager,
    /// The computer manager; re-simulation lets the engine make it again.
    Ai,
}

/// One entry of the applied-change log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeEntry {
    /// Its place among the applied changes, from 0.
    pub order: u32,
    pub team: usize,
    pub source: ChangeSource,
    /// The tick it was queued on.
    pub queued_tick: u32,
    /// The queue's running number when it was queued.
    pub queue_number: u32,
    /// The tick it applied on.
    pub tick: u32,
    /// The stoppage it applied at, by the rule pack's code.
    pub stoppage: String,
    pub change: LoggedChange,
}

impl ChangeEntry {
    /// The log entry for `applied`, made for a team whose manager is `manager`.
    pub fn of(applied: &AppliedChange, manager: Manager) -> Self {
        Self {
            order: applied.order,
            team: applied.team,
            source: match manager {
                Manager::Human => ChangeSource::Manager,
                Manager::Ai => ChangeSource::Ai,
            },
            queued_tick: applied.id.tick,
            queue_number: applied.id.n,
            tick: applied.tick,
            stoppage: applied.stoppage.code().to_string(),
            change: LoggedChange::from(&applied.change),
        }
    }
}

/// The watchdog mark: script calls past the wall-clock limit, and the reason the match is
/// invalid when there was one.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Watchdog {
    pub slow_calls: u32,
    #[serde(deserialize_with = "required::deserialize")]
    pub invalid: Option<String>,
}

/// What only full time knows: the applied-change log and the watchdog mark.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outcome {
    pub changes: Vec<ChangeEntry>,
    pub watchdog: Watchdog,
}

/// The outcome of the match `sim` played.
pub fn outcome_of(sim: &Simulation) -> Outcome {
    let managers = sim.managers();
    Outcome {
        changes: sim
            .applied_changes()
            .iter()
            .map(|a| ChangeEntry::of(a, managers[a.team]))
            .collect(),
        watchdog: Watchdog {
            slow_calls: sim.plugins().slow_calls(),
            invalid: sim.plugins().invalid().map(str::to_string),
        },
    }
}

/// The record entry of a format-4 file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRecord {
    pub engine: EngineIdentity,
    pub settings: MatchSettings,
    /// One entry per input entry, in file order.
    pub inputs: Vec<InputInfo>,
    /// The sum of the input files' sizes.
    pub inputs_bytes: u64,
    /// The applied-change log, in the order the changes applied.
    pub changes: Vec<ChangeEntry>,
    pub watchdog: Watchdog,
}

/// The list entries of `inputs`, and their total size.
fn listed(inputs: &[InputFile]) -> (Vec<InputInfo>, u64) {
    let list: Vec<InputInfo> = inputs
        .iter()
        .map(|f| InputInfo {
            role: f.role.clone(),
            name: f.name.clone(),
            bytes: f.bytes.len() as u64,
            sha256: hex(&Sha256::digest(&f.bytes)),
        })
        .collect();
    let total = list.iter().map(|i| i.bytes).sum();
    (list, total)
}

/// The payload of an input entry.
fn input_payload(file: &InputFile) -> Result<Vec<u8>, StreamError> {
    let name = file.name.as_bytes();
    let len = u16::try_from(name.len())
        .map_err(|_| StreamError::Fixture(format!("input name {} is too long", file.name)))?;
    let mut payload = Vec::with_capacity(2 + name.len() + file.bytes.len());
    payload.extend_from_slice(&len.to_le_bytes());
    payload.extend_from_slice(name);
    payload.extend_from_slice(&file.bytes);
    Ok(payload)
}

/// The record entry's payload.
fn record_payload(record: &ReplayRecord) -> Result<Vec<u8>, StreamError> {
    serde_json::to_vec(record)
        .map_err(|e| StreamError::Fixture(format!("cannot write the record entry: {e}")))
}

/// The 32-byte header, with the counts still zero.
fn header(format: u16, protocol: u16, match_millis: u64, seed: u64) -> [u8; FIXTURE_HEADER_BYTES] {
    let mut header = [0u8; FIXTURE_HEADER_BYTES];
    header[0..4].copy_from_slice(FIXTURE_MAGIC);
    if format == FORMAT_LEGACY {
        // Offset 4 of a legacy file is its protocol version, which is also 3.
        header[4..6].copy_from_slice(&protocol.to_le_bytes());
    } else {
        header[4..6].copy_from_slice(&format.to_le_bytes());
        header[6..8].copy_from_slice(&protocol.to_le_bytes());
    }
    header[8..16].copy_from_slice(&match_millis.to_le_bytes());
    header[24..32].copy_from_slice(&seed.to_le_bytes());
    header
}

/// Writes a fixture. It is a `FrameOut`, so the same encoder feeds it and the socket.
pub struct Recorder {
    writer: BufWriter<File>,
    path: PathBuf,
    shown: String,
    frames: u32,
    ticks: u32,
    bytes: u64,
    last_tick: u32,
    hasher: Sha256,
    /// The record a format-4 file ends with, still without its outcome; `None` for a
    /// format-3 file.
    record: Option<ReplayRecord>,
}

impl Recorder {
    /// Creates `path` and writes the header of a format-3 file, which holds frames only.
    /// The frame and tick counts are filled in when the recording finishes.
    pub fn create(path: &Path, match_millis: u64, seed: u64) -> Result<Self, StreamError> {
        Self::open(
            path,
            &header(FORMAT_LEGACY, PROTOCOL_VERSION, match_millis, seed),
            None,
        )
    }

    /// Creates `path` as a format-4 file: the header, then one input entry per file of
    /// `inputs`, in order. [`Recorder::finish_record`] ends it with the record.
    pub fn create_record(
        path: &Path,
        match_millis: u64,
        engine: EngineIdentity,
        settings: MatchSettings,
        inputs: &[InputFile],
    ) -> Result<Self, StreamError> {
        let (list, inputs_bytes) = listed(inputs);
        let seed = settings.seed;
        let record = ReplayRecord {
            engine,
            settings,
            inputs: list,
            inputs_bytes,
            changes: Vec::new(),
            watchdog: Watchdog::default(),
        };
        let mut recorder = Self::open(
            path,
            &header(FORMAT_VERSION, PROTOCOL_VERSION, match_millis, seed),
            Some(record),
        )?;
        for file in inputs {
            recorder.entry(ENTRY_INPUT, 0, &input_payload(file)?)?;
        }
        Ok(recorder)
    }

    fn open(
        path: &Path,
        header: &[u8; FIXTURE_HEADER_BYTES],
        record: Option<ReplayRecord>,
    ) -> Result<Self, StreamError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|e| StreamError::io("cannot create the fixture folder", e))?;
        }
        let file = File::create(path)
            .map_err(|e| StreamError::io(format!("cannot create {}", path.display()), e))?;
        let mut writer = BufWriter::with_capacity(1 << 20, file);
        writer
            .write_all(header)
            .map_err(|e| StreamError::io("cannot write the fixture header", e))?;
        Ok(Self {
            writer,
            path: path.to_path_buf(),
            shown: shown_path(path),
            frames: 0,
            ticks: 0,
            bytes: FIXTURE_HEADER_BYTES as u64,
            last_tick: 0,
            hasher: Sha256::new(),
            record,
        })
    }

    /// Writes one entry and hashes its payload.
    fn entry(&mut self, kind: u8, tick: u32, payload: &[u8]) -> Result<(), StreamError> {
        let mut entry = [0u8; 9];
        entry[0] = kind;
        entry[1..5].copy_from_slice(&tick.to_le_bytes());
        entry[5..9].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        self.writer
            .write_all(&entry)
            .and_then(|()| self.writer.write_all(payload))
            .map_err(|e| StreamError::io("cannot write a fixture entry", e))?;
        self.hasher.update(payload);
        self.bytes += (entry.len() + payload.len()) as u64;
        Ok(())
    }

    /// Writes the trailer, fills the header counts, and reports what a format-3 fixture
    /// holds. A format-4 file ends with its record instead: see [`Recorder::finish_record`].
    pub fn finish(self) -> Result<FixtureSummary, StreamError> {
        if self.record.is_some() {
            return Err(StreamError::Fixture(
                "a version-4 replay file ends with its record; finish it with the outcome".into(),
            ));
        }
        self.close()
    }

    /// Writes the record entry of a format-4 file with the match's `outcome`, then the
    /// trailer.
    pub fn finish_record(mut self, outcome: Outcome) -> Result<FixtureSummary, StreamError> {
        let Some(mut record) = self.record.take() else {
            return Err(StreamError::Fixture(
                "a version-3 replay file holds no record".into(),
            ));
        };
        record.changes = outcome.changes;
        record.watchdog = outcome.watchdog;
        self.entry(ENTRY_RECORD, 0, &record_payload(&record)?)?;
        self.close()
    }

    fn close(mut self) -> Result<FixtureSummary, StreamError> {
        let digest = self.hasher.finalize_reset();
        let hash = hex12(&digest);
        let mut trailer = [0u8; FIXTURE_TRAILER_BYTES];
        trailer[0..4].copy_from_slice(FIXTURE_TRAILER_MAGIC);
        trailer[4..8].copy_from_slice(&self.frames.to_le_bytes());
        trailer[8..14].copy_from_slice(&digest[..6]);
        self.writer
            .write_all(&trailer)
            .and_then(|()| self.writer.flush())
            .map_err(|e| StreamError::io("cannot write the fixture trailer", e))?;
        let bytes = self.bytes + FIXTURE_TRAILER_BYTES as u64;

        let mut file = self
            .writer
            .into_inner()
            .map_err(|e| StreamError::io("cannot flush the fixture", e.into_error()))?;
        file.seek(SeekFrom::Start(FRAMES_AT))
            .and_then(|_| file.write_all(&self.frames.to_le_bytes()))
            .and_then(|()| file.write_all(&self.ticks.to_le_bytes()))
            .and_then(|()| file.flush())
            .map_err(|e| StreamError::io("cannot fill the fixture header counts", e))?;

        tracing::info!(
            signal = "fixture.recorded",
            path = %self.shown,
            frames = self.frames,
            ticks = self.ticks,
            bytes,
            hash = %hash
        );
        Ok(FixtureSummary {
            frames: self.frames,
            ticks: self.ticks,
            bytes,
            hash,
        })
    }

    /// The file being written.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl FrameOut for Recorder {
    fn send(&mut self, frame: Frame) -> Result<(), StreamError> {
        let tick = match &frame {
            Frame::Tick(tick) => {
                self.ticks += 1;
                self.last_tick = tick_of(tick).unwrap_or(self.last_tick + 1);
                self.last_tick
            }
            Frame::Text(_) => self.last_tick,
        };
        let kind = if frame.is_text() {
            ENTRY_TEXT
        } else {
            ENTRY_BINARY
        };
        self.entry(kind, tick, frame.payload())?;
        self.frames += 1;
        Ok(())
    }
}

/// A recorder two writers share: the tick encoder and the message router both hold one.
/// The fixture is written from one thread, so a reference count is enough.
#[derive(Clone)]
pub struct SharedRecorder(std::rc::Rc<std::cell::RefCell<Recorder>>);

impl SharedRecorder {
    /// Shares one recorder.
    pub fn new(recorder: Recorder) -> Self {
        Self(std::rc::Rc::new(std::cell::RefCell::new(recorder)))
    }

    fn only(self) -> Result<Recorder, StreamError> {
        std::rc::Rc::try_unwrap(self.0)
            .map_err(|_| StreamError::Fixture("the fixture is still being written".into()))
            .map(std::cell::RefCell::into_inner)
    }

    /// Finishes a format-3 fixture. Every other handle must be dropped first.
    pub fn finish(self) -> Result<FixtureSummary, StreamError> {
        self.only()?.finish()
    }

    /// Finishes a format-4 file with the match's outcome. Every other handle must be
    /// dropped first.
    pub fn finish_record(self, outcome: Outcome) -> Result<FixtureSummary, StreamError> {
        self.only()?.finish_record(outcome)
    }
}

impl FrameOut for SharedRecorder {
    fn send(&mut self, frame: Frame) -> Result<(), StreamError> {
        self.0.borrow_mut().send(frame)
    }
}

/// One frame as the fixture stored it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFrame {
    pub tick: u32,
    pub frame: Frame,
}

/// A fixture read back from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    /// The file format: [`FORMAT_LEGACY`] or [`FORMAT_VERSION`].
    pub format: u16,
    /// The protocol version of the frames.
    pub protocol_version: u16,
    pub match_millis: u64,
    pub seed: u64,
    pub ticks: u32,
    pub frames: Vec<StoredFrame>,
    /// The inputs of a format-4 file, in file order, each with the role the record gives
    /// it; empty in format 3.
    pub inputs: Vec<InputFile>,
    /// The record of a format-4 file; `None` in format 3.
    pub record: Option<ReplayRecord>,
    pub hash: String,
}

/// Reads a fixture and fails closed on any layout problem, a count mismatch, a hash
/// mismatch, an unknown format or protocol version, or inputs the record does not list.
pub fn read_fixture(path: &Path) -> Result<Fixture, StreamError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| StreamError::io(format!("cannot read {}", path.display()), e))?;
    parse_fixture(&bytes)
}

/// [`read_fixture`] over bytes in memory, through the production migration chain.
pub fn parse_fixture(bytes: &[u8]) -> Result<Fixture, StreamError> {
    Chain::production().read(bytes)
}

/// [`parse_fixture`] through `chain`: the envelope, then the legacy decode for a version-3
/// file, or the chain's lift and decode for any later format. A lifted file reports the
/// chain's current format, and its hash is the hash of the bytes that were read.
#[cfg(feature = "test-support")]
pub fn parse_fixture_with(bytes: &[u8], chain: &Chain<'_>) -> Result<Fixture, StreamError> {
    chain.read(bytes)
}

/// Decodes a version-3 file: frames only, of protocol version 3.
pub(crate) fn decode_legacy(raw: RawReplay) -> Result<Fixture, StreamError> {
    decode_entries(raw, FORMAT_LEGACY)
}

/// Decodes a version-4 file: the inputs, the frames, and the record last.
pub(crate) fn decode_v4(raw: RawReplay) -> Result<Fixture, StreamError> {
    decode_entries(raw, FORMAT_VERSION)
}

fn decode_entries(raw: RawReplay, format: u16) -> Result<Fixture, StreamError> {
    let refuse = |reason: String| StreamError::Fixture(reason);
    if raw.format != format {
        return Err(refuse(format!(
            "a version-{format} decode was given a version-{} file",
            raw.format
        )));
    }
    let protocol_version = if format == FORMAT_LEGACY {
        raw.format
    } else {
        raw.protocol
    };
    if protocol_version != PROTOCOL_VERSION {
        return Err(refuse(format!(
            "frames protocol version {protocol_version}; this build speaks {PROTOCOL_VERSION}"
        )));
    }
    let mut frames = Vec::with_capacity(raw.entries.len());
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    let mut record: Option<ReplayRecord> = None;
    for entry in &raw.entries {
        let kind = entry.kind;
        if record.is_some() {
            return Err(refuse(match kind {
                ENTRY_RECORD => "the file holds two record entries".into(),
                _ => format!("an entry of kind {kind} follows the record entry"),
            }));
        }
        if format == FORMAT_LEGACY && (kind == ENTRY_INPUT || kind == ENTRY_RECORD) {
            return Err(refuse(format!(
                "entry kind {kind} in a version-3 file, which holds frames only"
            )));
        }
        match kind {
            ENTRY_INPUT => {
                if !frames.is_empty() {
                    return Err(refuse("an input entry follows the frames".into()));
                }
                inputs.push(split_input(&entry.payload)?);
            }
            ENTRY_RECORD => record = Some(decode_record(&entry.payload)?),
            _ => frames.push(decode_frame(entry)?),
        }
    }
    let fixture = Fixture {
        format,
        protocol_version,
        match_millis: raw.match_millis,
        seed: raw.seed,
        ticks: raw.ticks,
        frames,
        inputs: Vec::new(),
        record,
        hash: raw.hash,
    };
    check_frame_count(&fixture, raw.frames)?;
    check_tick_count(&fixture)?;
    let inputs = if format == FORMAT_VERSION {
        let Some(record) = &fixture.record else {
            return Err(refuse(
                "a version-4 file must end with its record entry; this one has none".into(),
            ));
        };
        if record.settings.seed != fixture.seed {
            return Err(refuse(format!(
                "seed mismatch: the header says {}, the record says {}",
                fixture.seed, record.settings.seed
            )));
        }
        check_inputs(record, inputs)?
    } else {
        Vec::new()
    };
    Ok(Fixture { inputs, ..fixture })
}

/// Refuses a fixture whose frames are not the `declared` count of its header.
pub fn check_frame_count(fixture: &Fixture, declared: u32) -> Result<(), StreamError> {
    if fixture.frames.len() != declared as usize {
        return Err(StreamError::Fixture(format!(
            "frame count mismatch: the header says {declared}, the body holds {}",
            fixture.frames.len()
        )));
    }
    Ok(())
}

/// Refuses a fixture whose tick frames are not the tick count of its header.
fn check_tick_count(fixture: &Fixture) -> Result<(), StreamError> {
    let held = fixture
        .frames
        .iter()
        .filter(|f| matches!(f.frame, Frame::Tick(_)))
        .count();
    if held != fixture.ticks as usize {
        return Err(StreamError::Fixture(format!(
            "tick count mismatch: the header says {}, the body holds {held}",
            fixture.ticks
        )));
    }
    Ok(())
}

/// One frame entry: a binary tick frame (kind 0) or a JSON text frame (kind 1).
pub fn decode_frame(entry: &RawEntry) -> Result<StoredFrame, StreamError> {
    let frame = match entry.kind {
        ENTRY_BINARY => Frame::Tick(TickFrame::from_bytes(&entry.payload)?),
        ENTRY_TEXT => Frame::Text(
            String::from_utf8(entry.payload.clone())
                .map_err(|e| StreamError::Fixture(format!("a text frame is not UTF-8: {e}")))?,
        ),
        other => {
            return Err(StreamError::Fixture(format!(
                "unknown entry kind {other} at tick {}",
                entry.tick
            )));
        }
    };
    Ok(StoredFrame {
        tick: entry.tick,
        frame,
    })
}

/// The record entry's payload as a record. Every field must be present: a missing field is
/// refused by name, never filled with a default.
pub fn decode_record(payload: &[u8]) -> Result<ReplayRecord, StreamError> {
    serde_json::from_slice(payload)
        .map_err(|e| StreamError::Fixture(format!("the record entry is not a record: {e}")))
}

/// An input entry's name and bytes.
pub fn split_input(payload: &[u8]) -> Result<(String, Vec<u8>), StreamError> {
    let refuse = |reason: &str| StreamError::Fixture(reason.into());
    if payload.len() < 2 {
        return Err(refuse("an input entry is truncated"));
    }
    let len = usize::from(u16::from_le_bytes([payload[0], payload[1]]));
    if payload.len() < 2 + len {
        return Err(refuse("an input entry's name is truncated"));
    }
    let name = std::str::from_utf8(&payload[2..2 + len])
        .map_err(|_| refuse("an input entry's name is not UTF-8"))?;
    Ok((name.to_string(), payload[2 + len..].to_vec()))
}

/// The inputs with their roles, when every entry is the one the record lists at its place.
pub fn check_inputs(
    record: &ReplayRecord,
    inputs: Vec<(String, Vec<u8>)>,
) -> Result<Vec<InputFile>, StreamError> {
    if inputs.len() != record.inputs.len() {
        return Err(StreamError::Fixture(format!(
            "the file holds {} input entries and the record lists {}",
            inputs.len(),
            record.inputs.len()
        )));
    }
    let mut out = Vec::with_capacity(inputs.len());
    for (listed, (name, bytes)) in record.inputs.iter().zip(inputs) {
        let sha256 = hex(&Sha256::digest(&bytes));
        if listed.name != name || listed.sha256 != sha256 || listed.bytes != bytes.len() as u64 {
            return Err(StreamError::Fixture(format!(
                "input {name} (SHA-256 {sha256}) does not match the record's {} {} \
                 (SHA-256 {})",
                listed.role, listed.name, listed.sha256
            )));
        }
        out.push(InputFile {
            role: listed.role.clone(),
            name,
            bytes,
        });
    }
    let total: u64 = out.iter().map(|f| f.bytes.len() as u64).sum();
    if total != record.inputs_bytes {
        return Err(StreamError::Fixture(format!(
            "the inputs hold {total} bytes and the record says {}",
            record.inputs_bytes
        )));
    }
    Ok(out)
}

/// Writes `fixture` as a file, entry for entry: a fixture read and written back is the
/// same file byte for byte. Tests use it to build patched and damaged files.
pub fn write_fixture(path: &Path, fixture: &Fixture) -> Result<(), StreamError> {
    let mut entries = Vec::with_capacity(fixture.inputs.len() + fixture.frames.len() + 1);
    for file in &fixture.inputs {
        entries.push(RawEntry {
            kind: ENTRY_INPUT,
            tick: 0,
            payload: input_payload(file)?,
        });
    }
    for stored in &fixture.frames {
        entries.push(RawEntry {
            kind: if stored.frame.is_text() {
                ENTRY_TEXT
            } else {
                ENTRY_BINARY
            },
            tick: stored.tick,
            payload: stored.frame.payload().to_vec(),
        });
    }
    if let Some(record) = &fixture.record {
        entries.push(RawEntry {
            kind: ENTRY_RECORD,
            tick: 0,
            payload: record_payload(record)?,
        });
    }
    let legacy = fixture.format == FORMAT_LEGACY;
    let raw = RawReplay {
        // Offset 4 of a legacy file is its protocol version, which is also 3.
        format: if legacy {
            fixture.protocol_version
        } else {
            fixture.format
        },
        protocol: if legacy { 0 } else { fixture.protocol_version },
        match_millis: fixture.match_millis,
        frames: fixture.frames.len() as u32,
        ticks: fixture.ticks,
        seed: fixture.seed,
        entries,
        hash: String::new(),
    };
    std::fs::write(path, raw.to_bytes())
        .map_err(|e| StreamError::io(format!("cannot write {}", path.display()), e))
}

/// A digest as lower-case hex.
pub fn hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// A 64-bit number as a decimal string.
mod decimal {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// An `Option` field that must be present: `null` reads as `None`, and a missing field is
/// an error. Without it, serde reads a missing `Option` field as `None` (source:
/// serde-1.0.229/src/private/de.rs:24-49, `missing_field`); a field with `deserialize_with`
/// returns the missing-field error instead (source: serde_derive-1.0.229/src/de.rs:791-803).
mod required {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Option::<T>::deserialize(d)
    }
}

/// The tick a keyframe carries, or `None` for a delta.
pub(crate) fn tick_of(frame: &TickFrame) -> Option<u32> {
    let bytes = frame.as_bytes();
    if frame.kind() == protocol::frame::KIND_DELTA {
        return None;
    }
    Some(u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]))
}

/// A path for signals: the file name alone, never an absolute path.
fn shown_path(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::record::TickRecord;

    use crate::session::FrameSink;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("stream-fixture-{}-{name}.smfx", std::process::id()))
    }

    fn record(tick: u32, shift: f32) -> TickRecord {
        TickRecord {
            tick,
            ball: [shift, 0.0, 0.0],
            players: [[shift, -shift]; 22],
            restart: tick == 1,
        }
    }

    #[test]
    fn a_fixture_round_trips_every_frame() {
        let path = temp("roundtrip");
        let recorder = Recorder::create(&path, 1_700_000_000_000, 7).unwrap();
        let mut sink = FrameSink::new(recorder, 50);
        for tick in 1..=120u32 {
            sink.push(&record(tick, tick as f32 * 0.05)).unwrap();
        }
        let mut recorder = sink.into_inner();
        recorder
            .send(Frame::Text("{\"type\":\"stats\"}".into()))
            .unwrap();
        let summary = recorder.finish().unwrap();
        assert_eq!(summary.frames, 121);
        assert_eq!(summary.ticks, 120);

        let fixture = read_fixture(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(fixture.seed, 7);
        assert_eq!(fixture.ticks, 120);
        assert_eq!(fixture.hash, summary.hash);
        assert_eq!(fixture.frames.len(), 121);
        assert_eq!(fixture.frames[0].tick, 1);
        assert!(fixture.frames[120].frame.is_text());
    }

    #[test]
    fn a_truncated_fixture_is_refused() {
        let path = temp("truncated");
        let mut recorder = Recorder::create(&path, 1, 1).unwrap();
        recorder.send(Frame::Text("{}".into())).unwrap();
        recorder.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::write(&path, &bytes[..bytes.len() - FIXTURE_TRAILER_BYTES - 1]).unwrap();
        let err = read_fixture(&path).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(err, StreamError::Fixture(_)), "{err}");
    }

    #[test]
    fn a_huge_declared_frame_count_is_refused_without_allocating_it() {
        let path = temp("huge-count");
        let mut recorder = Recorder::create(&path, 1, 1).unwrap();
        recorder.send(Frame::Text("{}".into())).unwrap();
        recorder.finish().unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        let trailer = bytes.len() - FIXTURE_TRAILER_BYTES;
        bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        bytes[trailer + 4..trailer + 8].copy_from_slice(&u32::MAX.to_le_bytes());
        std::fs::write(&path, &bytes).unwrap();
        let err = read_fixture(&path).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(err, StreamError::Fixture(_)), "{err}");
    }

    #[test]
    fn a_changed_byte_fails_the_hash() {
        let path = temp("tampered");
        let mut recorder = Recorder::create(&path, 1, 1).unwrap();
        recorder.send(Frame::Text("{\"a\":1}".into())).unwrap();
        recorder.finish().unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        let at = FIXTURE_HEADER_BYTES + 9;
        bytes[at] = b'X';
        std::fs::write(&path, &bytes).unwrap();
        let err = read_fixture(&path).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert!(err.to_string().contains("trailer hash"), "{err}");
    }

    #[test]
    fn a_logged_change_round_trips_a_substitution_and_a_full_tactics_patch() {
        let sub = Change::Substitution { off: 9, on: 14 };
        let tactics = Change::Tactics(TacticsPatch {
            formation: Some(2),
            mentality: Some(4),
            instructions: [Some(0), Some(1), Some(2), Some(1), Some(0), Some(2)],
            roles: vec![
                (3, RoleDuty { role: 5, duty: 1 }),
                (7, RoleDuty { role: 2, duty: 0 }),
            ],
        });
        for change in [sub, tactics] {
            let logged = LoggedChange::from(&change);
            let json = serde_json::to_string(&logged).unwrap();
            let back: LoggedChange = serde_json::from_str(&json).unwrap();
            assert_eq!(back, logged, "{json}");
            assert_eq!(back.to_change(), change, "{json}");
        }
        let json =
            serde_json::to_string(&LoggedChange::from(&Change::Substitution { off: 1, on: 2 }))
                .unwrap();
        assert_eq!(json, r#"{"substitution":{"off":1,"on":2}}"#);
    }

    #[test]
    fn a_legacy_file_reads_as_format_three_with_no_record() {
        let path = temp("legacy");
        let mut recorder = Recorder::create(&path, 1, 5).unwrap();
        recorder.send(Frame::Text("{}".into())).unwrap();
        recorder.finish().unwrap();
        let fixture = read_fixture(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(fixture.format, FORMAT_LEGACY);
        assert_eq!(fixture.protocol_version, PROTOCOL_VERSION);
        assert!(fixture.record.is_none() && fixture.inputs.is_empty());
    }
}
