//! The tick record and the `.ticks` file: a 64-byte header, one 192-byte record per tick,
//! and a 16-byte trailer with the written count. A reader refuses a file without the
//! trailer or with a count mismatch.

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::team::PLAYERS_PER_TEAM;

/// Players per record.
pub const PLAYER_COUNT: usize = 2 * PLAYERS_PER_TEAM;
/// Bytes per record: tick (4) + ball (12) + 22 players × 8.
pub const RECORD_BYTES: usize = 4 + 12 + PLAYER_COUNT * 8;
/// Header bytes.
pub const HEADER_BYTES: usize = 64;
/// Trailer bytes.
pub const TRAILER_BYTES: usize = 16;
/// Schema version of the file layout.
pub const SCHEMA_VERSION: u16 = 1;

const MAGIC: &[u8; 4] = b"SMTK";
const TRAILER_MAGIC: &[u8; 4] = b"SMTE";

/// One tick: the ball in three dimensions and every player on the ground plane.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TickRecord {
    pub tick: u32,
    pub ball: [f32; 3],
    pub players: [[f32; 2]; PLAYER_COUNT],
}

impl TickRecord {
    /// Serializes into a fixed little-endian layout.
    pub fn write_to(&self, buf: &mut [u8; RECORD_BYTES]) {
        buf[0..4].copy_from_slice(&self.tick.to_le_bytes());
        let mut at = 4;
        for v in self.ball {
            buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
            at += 4;
        }
        for p in self.players {
            for v in p {
                buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
                at += 4;
            }
        }
    }

    /// Parses the fixed layout. `buf` must be at least `RECORD_BYTES` long.
    pub fn read_from(buf: &[u8]) -> Self {
        let f32_at =
            |at: usize| f32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
        let tick = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let ball = [f32_at(4), f32_at(8), f32_at(12)];
        let mut players = [[0.0f32; 2]; PLAYER_COUNT];
        for (i, p) in players.iter_mut().enumerate() {
            let at = 16 + i * 8;
            *p = [f32_at(at), f32_at(at + 4)];
        }
        Self {
            tick,
            ball,
            players,
        }
    }
}

/// A consumer of tick records.
pub trait TickSink {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError>;
}

/// Discards every record (benchmarks).
pub struct NullSink;

impl TickSink for NullSink {
    fn on_tick(&mut self, _record: &TickRecord) -> Result<(), EngineError> {
        Ok(())
    }
}

/// Keeps every record in memory (tests and the validator).
#[derive(Default)]
pub struct VecSink {
    pub records: Vec<TickRecord>,
}

impl TickSink for VecSink {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.records.push(*record);
        Ok(())
    }
}

/// Writes a `.ticks` file.
pub struct FileSink {
    writer: BufWriter<File>,
    written: u32,
    buf: [u8; RECORD_BYTES],
}

impl FileSink {
    /// Creates the file and writes the header.
    pub fn create(
        path: &Path,
        seed: u64,
        dt: f64,
        expected_ticks: u32,
    ) -> Result<Self, EngineError> {
        let mut writer = BufWriter::with_capacity(1 << 20, File::create(path)?);
        let mut header = [0u8; HEADER_BYTES];
        header[0..4].copy_from_slice(MAGIC);
        header[4..6].copy_from_slice(&SCHEMA_VERSION.to_le_bytes());
        header[6] = 4;
        header[7] = PLAYER_COUNT as u8;
        header[8..16].copy_from_slice(&seed.to_le_bytes());
        header[16..24].copy_from_slice(&dt.to_le_bytes());
        header[24..28].copy_from_slice(&expected_ticks.to_le_bytes());
        writer.write_all(&header)?;
        tracing::info!(
            signal = "tickfile.header",
            schema_version = SCHEMA_VERSION,
            seed,
            dt_ms = dt * 1000.0,
            ticks_expected = expected_ticks,
            float_width = 4
        );
        Ok(Self {
            writer,
            written: 0,
            buf: [0u8; RECORD_BYTES],
        })
    }

    /// Writes the trailer and flushes. Returns the number of records written.
    pub fn finish(mut self) -> Result<u32, EngineError> {
        let mut trailer = [0u8; TRAILER_BYTES];
        trailer[0..4].copy_from_slice(TRAILER_MAGIC);
        trailer[4..8].copy_from_slice(&self.written.to_le_bytes());
        self.writer.write_all(&trailer)?;
        self.writer.flush()?;
        let bytes = HEADER_BYTES + self.written as usize * RECORD_BYTES + TRAILER_BYTES;
        tracing::info!(
            signal = "tickfile.trailer",
            ticks_written = self.written,
            bytes
        );
        Ok(self.written)
    }
}

impl TickSink for FileSink {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        record.write_to(&mut self.buf);
        self.writer.write_all(&self.buf)?;
        self.written += 1;
        Ok(())
    }
}

/// A parsed `.ticks` file.
#[derive(Debug)]
pub struct TickFile {
    pub seed: u64,
    pub dt: f64,
    pub expected_ticks: u32,
    pub records: Vec<TickRecord>,
}

/// Reads and validates a `.ticks` file. Fails closed on any layout problem.
pub fn read_ticks(path: &Path) -> Result<TickFile, EngineError> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    if bytes.len() < HEADER_BYTES + TRAILER_BYTES {
        return Err(EngineError::Format(
            "file shorter than header plus trailer".into(),
        ));
    }
    if &bytes[0..4] != MAGIC {
        return Err(EngineError::Format("bad magic".into()));
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != SCHEMA_VERSION {
        return Err(EngineError::Format(format!(
            "unknown schema version {version}"
        )));
    }
    if bytes[6] != 4 || bytes[7] as usize != PLAYER_COUNT {
        return Err(EngineError::Format(
            "unexpected float width or player count".into(),
        ));
    }
    let seed = u64::from_le_bytes(bytes[8..16].try_into().expect("8 bytes"));
    let dt = f64::from_le_bytes(bytes[16..24].try_into().expect("8 bytes"));
    let expected_ticks = u32::from_le_bytes(bytes[24..28].try_into().expect("4 bytes"));
    let trailer = &bytes[bytes.len() - TRAILER_BYTES..];
    if &trailer[0..4] != TRAILER_MAGIC {
        return Err(EngineError::Format(
            "missing trailer: the file is incomplete".into(),
        ));
    }
    let written = u32::from_le_bytes([trailer[4], trailer[5], trailer[6], trailer[7]]);
    let body = &bytes[HEADER_BYTES..bytes.len() - TRAILER_BYTES];
    if body.len() != written as usize * RECORD_BYTES {
        return Err(EngineError::Format(format!(
            "record count mismatch: trailer says {written}, body holds {} bytes",
            body.len()
        )));
    }
    let records = body
        .chunks_exact(RECORD_BYTES)
        .map(TickRecord::read_from)
        .collect();
    Ok(TickFile {
        seed,
        dt,
        expected_ticks,
        records,
    })
}

/// Writes records as JSON Lines, one object per tick, positions in roster order.
pub fn write_jsonl(path: &Path, records: &[TickRecord]) -> Result<(), EngineError> {
    let mut writer = BufWriter::with_capacity(1 << 20, File::create(path)?);
    for r in records {
        serde_json::to_writer(&mut writer, r).map_err(|e| EngineError::Format(e.to_string()))?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("engine-record-{}-{name}.ticks", std::process::id()))
    }

    fn sample(tick: u32) -> TickRecord {
        let mut players = [[0.0f32; 2]; PLAYER_COUNT];
        for (i, p) in players.iter_mut().enumerate() {
            *p = [i as f32, -(i as f32)];
        }
        TickRecord {
            tick,
            ball: [1.5, -2.5, 0.25],
            players,
        }
    }

    #[test]
    fn record_round_trips_through_bytes() {
        let r = sample(7);
        let mut buf = [0u8; RECORD_BYTES];
        r.write_to(&mut buf);
        assert_eq!(TickRecord::read_from(&buf), r);
    }

    #[test]
    fn file_round_trips_a_thousand_records() {
        let path = temp("roundtrip");
        let mut sink = FileSink::create(&path, 42, 0.02, 1000).unwrap();
        for i in 0..1000 {
            sink.on_tick(&sample(i)).unwrap();
        }
        assert_eq!(sink.finish().unwrap(), 1000);
        let file = read_ticks(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(file.seed, 42);
        assert_eq!(file.expected_ticks, 1000);
        assert_eq!(file.records.len(), 1000);
        assert_eq!(file.records[999], sample(999));
    }

    #[test]
    fn a_truncated_file_is_refused() {
        let path = temp("truncated");
        let mut sink = FileSink::create(&path, 1, 0.02, 10).unwrap();
        for i in 0..10 {
            sink.on_tick(&sample(i)).unwrap();
        }
        sink.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::write(&path, &bytes[..bytes.len() - TRAILER_BYTES - 5]).unwrap();
        let err = read_ticks(&path).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(err, EngineError::Format(_)), "{err}");
    }
}
