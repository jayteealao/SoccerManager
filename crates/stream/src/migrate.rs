//! Replay file migration: a file of an older format is lifted one format at a time, then
//! decoded as the newest format.
//!
//! A reader works in three parts. The envelope ([`RawReplay`]) reads the header, the entry
//! framing, and the trailer hash, and knows no entry kind. The chain ([`Chain`]) runs one
//! [`Step`] per format change, each a pure function from one raw file to the raw file of the
//! next format. The decode turns the newest format into a [`Fixture`]. Version-3 files are
//! never lifted: they play from their frames only.
//!
//! Every format so far keeps the 32-byte header, the entry framing, and the 16-byte trailer.
//! A later format that changes them adds a branch to [`RawReplay::parse`].

use engine::data::hex12;
use sha2::{Digest, Sha256};

use crate::StreamError;
use crate::record::{
    FIXTURE_HEADER_BYTES, FIXTURE_MAGIC, FIXTURE_TRAILER_BYTES, FIXTURE_TRAILER_MAGIC,
    FORMAT_LEGACY, FORMAT_VERSION, Fixture, decode_v4,
};
use protocol::PROTOCOL_VERSION;

/// The oldest format the chain lifts. Older files are version 3, which play only.
pub const FORMAT_FIRST_MIGRATED: u16 = 4;

/// One entry of a replay file, undecoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    pub kind: u8,
    pub tick: u32,
    pub payload: Vec<u8>,
}

/// A replay file as its envelope holds it: the header fields and the entries in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawReplay {
    /// The u16 at offset 4: the file format (the protocol version in a version-3 file).
    pub format: u16,
    /// The u16 at offset 6: the frames' protocol version (zero in a version-3 file).
    pub protocol: u16,
    pub match_millis: u64,
    /// The frame count the header and the trailer declare.
    pub frames: u32,
    pub ticks: u32,
    pub seed: u64,
    pub entries: Vec<RawEntry>,
    /// The short hash of the bytes that were read: the first six bytes of the SHA-256 over
    /// every payload, as hex. A lifted file keeps it.
    pub hash: String,
}

impl RawReplay {
    /// Reads the envelope. Refuses a short file, a bad magic, a missing trailer, header and
    /// trailer frame counts that differ, a truncated entry, and a trailer hash that does not
    /// match the payloads.
    pub fn parse(bytes: &[u8]) -> Result<Self, StreamError> {
        let refuse = |reason: String| StreamError::Fixture(reason);
        if bytes.len() < FIXTURE_HEADER_BYTES + FIXTURE_TRAILER_BYTES {
            return Err(refuse("file shorter than header plus trailer".into()));
        }
        if &bytes[0..4] != FIXTURE_MAGIC {
            return Err(refuse("bad magic".into()));
        }
        let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"));
        let u64_at = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().expect("8 bytes"));
        let frames = u32_at(16);
        let trailer_at = bytes.len() - FIXTURE_TRAILER_BYTES;
        let trailer = &bytes[trailer_at..];
        if &trailer[0..4] != FIXTURE_TRAILER_MAGIC {
            return Err(refuse("missing trailer: the fixture is incomplete".into()));
        }
        let trailer_frames = u32_at(trailer_at + 4);
        if trailer_frames != frames {
            return Err(refuse(format!(
                "frame count mismatch: header says {frames}, trailer says {trailer_frames}"
            )));
        }

        let body = &bytes[FIXTURE_HEADER_BYTES..trailer_at];
        let mut hasher = Sha256::new();
        // The header's count is not trusted for the allocation: every entry takes at least
        // its 9-byte head, so the body cannot hold more entries than that.
        let mut entries = Vec::with_capacity((frames as usize).min(body.len() / 9));
        let mut at = 0usize;
        while at < body.len() {
            if at + 9 > body.len() {
                return Err(refuse("a frame entry is truncated".into()));
            }
            let kind = body[at];
            let tick = u32::from_le_bytes(body[at + 1..at + 5].try_into().expect("4 bytes"));
            let len =
                u32::from_le_bytes(body[at + 5..at + 9].try_into().expect("4 bytes")) as usize;
            at += 9;
            if len > body.len() - at {
                return Err(refuse(format!(
                    "frame at tick {tick} claims {len} bytes past the end of the file"
                )));
            }
            let payload = &body[at..at + len];
            at += len;
            hasher.update(payload);
            entries.push(RawEntry {
                kind,
                tick,
                payload: payload.to_vec(),
            });
        }
        let digest = hasher.finalize();
        if digest[..6] != trailer[8..14] {
            return Err(refuse(
                "the frame bytes do not match the trailer hash".into(),
            ));
        }
        Ok(Self {
            format: u16_at(4),
            protocol: u16_at(6),
            match_millis: u64_at(8),
            frames,
            ticks: u32_at(20),
            seed: u64_at(24),
            entries,
            hash: hex12(&digest),
        })
    }

    /// The file's bytes: the header, every entry in order, and a trailer whose hash is
    /// computed again over the payloads.
    pub fn to_bytes(&self) -> Vec<u8> {
        let size = FIXTURE_HEADER_BYTES
            + self
                .entries
                .iter()
                .map(|e| 9 + e.payload.len())
                .sum::<usize>()
            + FIXTURE_TRAILER_BYTES;
        let mut out = Vec::with_capacity(size);
        out.extend_from_slice(FIXTURE_MAGIC);
        out.extend_from_slice(&self.format.to_le_bytes());
        out.extend_from_slice(&self.protocol.to_le_bytes());
        out.extend_from_slice(&self.match_millis.to_le_bytes());
        out.extend_from_slice(&self.frames.to_le_bytes());
        out.extend_from_slice(&self.ticks.to_le_bytes());
        out.extend_from_slice(&self.seed.to_le_bytes());
        let mut hasher = Sha256::new();
        for entry in &self.entries {
            out.push(entry.kind);
            out.extend_from_slice(&entry.tick.to_le_bytes());
            out.extend_from_slice(&(entry.payload.len() as u32).to_le_bytes());
            out.extend_from_slice(&entry.payload);
            hasher.update(&entry.payload);
        }
        let digest = hasher.finalize();
        out.extend_from_slice(FIXTURE_TRAILER_MAGIC);
        out.extend_from_slice(&self.frames.to_le_bytes());
        out.extend_from_slice(&digest[..6]);
        out.extend_from_slice(&[0, 0]);
        out
    }
}

/// Lifts a raw file of format `from` to format `to`.
pub type Lift = fn(RawReplay) -> Result<RawReplay, StreamError>;

/// Decodes a raw file of the chain's current format.
pub type Decode = fn(RawReplay) -> Result<Fixture, StreamError>;

/// One format change: a raw file of format `from` becomes a raw file of format `to`.
#[derive(Debug, Clone, Copy)]
pub struct Step {
    pub from: u16,
    pub to: u16,
    /// A short name for messages and the shared step list.
    pub name: &'static str,
    pub lift: Lift,
}

/// The steps of this build: one per format change after format 4. None exists yet.
pub const STEPS: &[Step] = &[];

/// The steps from [`FORMAT_FIRST_MIGRATED`] to the format the decode reads.
#[derive(Debug, Clone, Copy)]
pub struct Chain<'a> {
    /// The format `decode` reads, and the newest format this reader knows.
    pub current: u16,
    pub steps: &'a [Step],
    pub decode: Decode,
}

impl Chain<'static> {
    /// The chain every production read uses: [`STEPS`] up to [`FORMAT_VERSION`].
    pub fn production() -> Self {
        Self {
            current: FORMAT_VERSION,
            steps: STEPS,
            decode: decode_v4,
        }
    }
}

impl Chain<'_> {
    /// The formats this reader reads, for messages.
    fn reads(&self) -> String {
        if self.current == FORMAT_FIRST_MIGRATED {
            format!("formats {FORMAT_LEGACY} and {}", self.current)
        } else {
            format!("formats {FORMAT_LEGACY} to {}", self.current)
        }
    }

    /// Refuses a chain whose steps do not run one format at a time, with no gap, from
    /// [`FORMAT_FIRST_MIGRATED`] to `current`.
    pub fn check(&self) -> Result<(), StreamError> {
        let held = || {
            self.steps
                .iter()
                .map(|s| format!("{}→{}", s.from, s.to))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut next = FORMAT_FIRST_MIGRATED;
        for step in self.steps {
            if step.from != next {
                return Err(StreamError::Fixture(format!(
                    "the migration chain has no step from format {next} (it holds {})",
                    held()
                )));
            }
            if step.to != step.from + 1 {
                return Err(StreamError::Fixture(format!(
                    "migration step {}→{} ({}) does not lift exactly one format",
                    step.from, step.to, step.name
                )));
            }
            next = step.to;
        }
        if next != self.current {
            return Err(StreamError::Fixture(format!(
                "the migration chain ends at format {next}, and the reader decodes format {}",
                self.current
            )));
        }
        Ok(())
    }

    /// Lifts `raw` to `current`, one step at a time. Refuses a format newer than `current`
    /// or older than [`FORMAT_FIRST_MIGRATED`], a broken chain, and a step whose output is
    /// not the format it names. Logs `replay.migrated` when at least one step ran.
    pub fn lift(&self, mut raw: RawReplay) -> Result<RawReplay, StreamError> {
        let from = raw.format;
        // Older writers put the protocol version at offset 4, so the bracket says how the
        // file reads as a version-3 file.
        let as_legacy = format!(
            "(read as a version-3 file: protocol version {from}; this build speaks \
             {PROTOCOL_VERSION})"
        );
        if from > self.current {
            return Err(StreamError::Fixture(format!(
                "format {from} is newer than this reader knows; this build reads {} {as_legacy}",
                self.reads()
            )));
        }
        if from < FORMAT_FIRST_MIGRATED {
            return Err(StreamError::Fixture(format!(
                "format {from}; this build reads {} {as_legacy}",
                self.reads()
            )));
        }
        self.check()?;
        let mut ran = 0usize;
        for step in self.steps.iter().filter(|s| s.from >= from) {
            raw = (step.lift)(raw)?;
            if raw.format != step.to {
                return Err(StreamError::Fixture(format!(
                    "migration step {}→{} ({}) returned format {}",
                    step.from, step.to, step.name, raw.format
                )));
            }
            ran += 1;
        }
        if ran > 0 {
            tracing::info!(
                signal = "replay.migrated",
                from,
                to = raw.format,
                steps = ran
            );
        }
        Ok(raw)
    }

    /// Reads a file through this chain: the envelope, then the legacy decode for a
    /// version-3 file, or the lift and `decode` for any later format.
    pub fn read(&self, bytes: &[u8]) -> Result<Fixture, StreamError> {
        let raw = RawReplay::parse(bytes)?;
        if raw.format == FORMAT_LEGACY {
            return crate::record::decode_legacy(raw);
        }
        (self.decode)(self.lift(raw)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../web/tests/data")
            .join(name);
        std::fs::read(path).unwrap()
    }

    fn same(raw: RawReplay) -> Result<RawReplay, StreamError> {
        Ok(raw)
    }

    fn step(from: u16, to: u16) -> Step {
        Step {
            from,
            to,
            name: "test",
            lift: same,
        }
    }

    #[test]
    fn the_envelope_writes_back_a_version_3_and_a_version_4_file_byte_for_byte() {
        for (name, format) in [("one-minute.smfx", 3), ("one-minute-v4.smfx", 4)] {
            let bytes = committed(name);
            let raw = RawReplay::parse(&bytes).unwrap();
            assert_eq!(raw.format, format, "{name}");
            assert_eq!(raw.to_bytes(), bytes, "{name}");
        }
    }

    #[test]
    fn a_chain_with_a_gap_a_long_step_or_a_wrong_end_is_refused() {
        let decode = decode_v4;
        let chain = |current, steps: &'static [Step]| Chain {
            current,
            steps,
            decode,
        };
        let gap: &'static [Step] = Box::leak(Box::new([step(4, 5), step(6, 7)]));
        let err = chain(7, gap).check().unwrap_err().to_string();
        assert!(
            err.contains("no step from format 5 (it holds 4→5, 6→7)"),
            "{err}"
        );
        let long: &'static [Step] = Box::leak(Box::new([step(4, 6)]));
        let err = chain(6, long).check().unwrap_err().to_string();
        assert!(err.contains("does not lift exactly one format"), "{err}");
        let short: &'static [Step] = Box::leak(Box::new([step(4, 5)]));
        let err = chain(6, short).check().unwrap_err().to_string();
        assert!(err.contains("ends at format 5"), "{err}");
        // Controls: a contiguous chain and the production chain pass.
        let whole: &'static [Step] = Box::leak(Box::new([step(4, 5), step(5, 6)]));
        chain(6, whole).check().unwrap();
        Chain::production().check().unwrap();
    }
}
