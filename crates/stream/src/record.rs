//! The fixture file (`.smfx`): the wire bytes of one whole match, exactly as they were
//! sent. A replay writes them back with no re-encoding, so the replayed stream is
//! byte-identical by construction and a viewer can be verified against a fixed recording.

use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use engine::data::hex12;
use protocol::{Frame, PROTOCOL_VERSION, TickFrame};
use sha2::{Digest, Sha256};

use crate::StreamError;
use crate::session::FrameOut;

/// Magic of a fixture file.
pub const FIXTURE_MAGIC: &[u8; 4] = b"SMFX";
/// Magic of the trailer.
pub const FIXTURE_TRAILER_MAGIC: &[u8; 4] = b"SMFE";
/// Header bytes.
pub const FIXTURE_HEADER_BYTES: usize = 32;
/// Trailer bytes.
pub const FIXTURE_TRAILER_BYTES: usize = 16;
/// Header offset of the frame count.
const FRAMES_AT: u64 = 16;
/// A binary tick frame.
const ENTRY_BINARY: u8 = 0;
/// A JSON text frame.
const ENTRY_TEXT: u8 = 1;

/// What a finished fixture holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSummary {
    pub frames: u32,
    pub ticks: u32,
    pub bytes: u64,
    pub hash: String,
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
}

impl Recorder {
    /// Creates `path` and writes the header. The frame and tick counts are filled in when
    /// the recording finishes.
    pub fn create(path: &Path, match_millis: u64, seed: u64) -> Result<Self, StreamError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|e| StreamError::io("cannot create the fixture folder", e))?;
        }
        let file = File::create(path)
            .map_err(|e| StreamError::io(format!("cannot create {}", path.display()), e))?;
        let mut writer = BufWriter::with_capacity(1 << 20, file);
        let mut header = [0u8; FIXTURE_HEADER_BYTES];
        header[0..4].copy_from_slice(FIXTURE_MAGIC);
        header[4..6].copy_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        header[8..16].copy_from_slice(&match_millis.to_le_bytes());
        header[24..32].copy_from_slice(&seed.to_le_bytes());
        writer
            .write_all(&header)
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
        })
    }

    /// Writes the trailer, fills the header counts, and reports what the fixture holds.
    pub fn finish(mut self) -> Result<FixtureSummary, StreamError> {
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
        let payload = frame.payload();
        let tick = match &frame {
            Frame::Tick(tick) => {
                self.ticks += 1;
                self.last_tick = tick_of(tick).unwrap_or(self.last_tick + 1);
                self.last_tick
            }
            Frame::Text(_) => self.last_tick,
        };
        let mut entry = [0u8; 9];
        entry[0] = if frame.is_text() {
            ENTRY_TEXT
        } else {
            ENTRY_BINARY
        };
        entry[1..5].copy_from_slice(&tick.to_le_bytes());
        entry[5..9].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        self.writer
            .write_all(&entry)
            .and_then(|()| self.writer.write_all(payload))
            .map_err(|e| StreamError::io("cannot write a fixture frame", e))?;
        self.hasher.update(payload);
        self.frames += 1;
        self.bytes += (entry.len() + payload.len()) as u64;
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

    /// Finishes the fixture. Every other handle must be dropped first.
    pub fn finish(self) -> Result<FixtureSummary, StreamError> {
        std::rc::Rc::try_unwrap(self.0)
            .map_err(|_| StreamError::Fixture("the fixture is still being written".into()))?
            .into_inner()
            .finish()
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
    pub protocol_version: u16,
    pub match_millis: u64,
    pub seed: u64,
    pub ticks: u32,
    pub frames: Vec<StoredFrame>,
    pub hash: String,
}

/// Reads a fixture and fails closed on any layout problem, a count mismatch, a hash
/// mismatch, or an unknown protocol version.
pub fn read_fixture(path: &Path) -> Result<Fixture, StreamError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| StreamError::io(format!("cannot read {}", path.display()), e))?;
    if bytes.len() < FIXTURE_HEADER_BYTES + FIXTURE_TRAILER_BYTES {
        return Err(StreamError::Fixture(
            "file shorter than header plus trailer".into(),
        ));
    }
    if &bytes[0..4] != FIXTURE_MAGIC {
        return Err(StreamError::Fixture("bad magic".into()));
    }
    let protocol_version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if protocol_version != PROTOCOL_VERSION {
        return Err(StreamError::Fixture(format!(
            "protocol version {protocol_version}; this build speaks {PROTOCOL_VERSION}"
        )));
    }
    let match_millis = u64::from_le_bytes(bytes[8..16].try_into().expect("8 bytes"));
    let declared_frames = u32::from_le_bytes(bytes[16..20].try_into().expect("4 bytes"));
    let ticks = u32::from_le_bytes(bytes[20..24].try_into().expect("4 bytes"));
    let seed = u64::from_le_bytes(bytes[24..32].try_into().expect("8 bytes"));

    let trailer = &bytes[bytes.len() - FIXTURE_TRAILER_BYTES..];
    if &trailer[0..4] != FIXTURE_TRAILER_MAGIC {
        return Err(StreamError::Fixture(
            "missing trailer: the fixture is incomplete".into(),
        ));
    }
    let trailer_frames = u32::from_le_bytes(trailer[4..8].try_into().expect("4 bytes"));
    if trailer_frames != declared_frames {
        return Err(StreamError::Fixture(format!(
            "frame count mismatch: header says {declared_frames}, trailer says {trailer_frames}"
        )));
    }

    let body = &bytes[FIXTURE_HEADER_BYTES..bytes.len() - FIXTURE_TRAILER_BYTES];
    let mut hasher = Sha256::new();
    // The header's count is not trusted for the allocation: every entry takes at least its
    // 9-byte head, so the body cannot hold more frames than that.
    let mut frames = Vec::with_capacity((declared_frames as usize).min(body.len() / 9));
    let mut at = 0usize;
    while at < body.len() {
        if at + 9 > body.len() {
            return Err(StreamError::Fixture("a frame entry is truncated".into()));
        }
        let kind = body[at];
        let tick = u32::from_le_bytes(body[at + 1..at + 5].try_into().expect("4 bytes"));
        let len = u32::from_le_bytes(body[at + 5..at + 9].try_into().expect("4 bytes")) as usize;
        at += 9;
        if at + len > body.len() {
            return Err(StreamError::Fixture(format!(
                "frame at tick {tick} claims {len} bytes past the end of the file"
            )));
        }
        let payload = &body[at..at + len];
        at += len;
        hasher.update(payload);
        let frame = match kind {
            ENTRY_BINARY => Frame::Tick(TickFrame::from_bytes(payload)?),
            ENTRY_TEXT => Frame::Text(
                String::from_utf8(payload.to_vec())
                    .map_err(|e| StreamError::Fixture(format!("a text frame is not UTF-8: {e}")))?,
            ),
            other => {
                return Err(StreamError::Fixture(format!(
                    "unknown entry kind {other} at tick {tick}"
                )));
            }
        };
        frames.push(StoredFrame { tick, frame });
    }
    if frames.len() != declared_frames as usize {
        return Err(StreamError::Fixture(format!(
            "frame count mismatch: the header says {declared_frames}, the body holds {}",
            frames.len()
        )));
    }
    let digest = hasher.finalize();
    if digest[..6] != trailer[8..14] {
        return Err(StreamError::Fixture(
            "the frame bytes do not match the trailer hash".into(),
        ));
    }
    Ok(Fixture {
        protocol_version,
        match_millis,
        seed,
        ticks,
        frames,
        hash: hex12(&digest),
    })
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
}
