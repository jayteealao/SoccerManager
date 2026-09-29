//! The tick record and the `.ticks` file: a 64-byte header, one 192-byte record per tick,
//! and a 16-byte trailer with the written count. A reader refuses a file without the
//! trailer or with a count mismatch. Header schema 2 carries `owner.id` and the match
//! stamp in the bytes schema 1 reserved (product-owner choice, plan Q3). Header schema 3
//! carries the restart flag in the top bit of the stored tick number, so a reader of the
//! file sees the kick-offs the validator needs. Header schema 4 changes the meaning of
//! `expected_ticks` from the exact count to the most a match can last: added time makes the
//! real length known only at full time, and the trailer carries it.

use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::rules::Stoppage;
use crate::sim::Simulation;
use crate::team::PLAYERS_PER_TEAM;
use crate::trace::TraceRecord;

/// Players per record.
pub const PLAYER_COUNT: usize = 2 * PLAYERS_PER_TEAM;
/// Bytes per record: tick (4) + ball (12) + 22 players × 8.
pub const RECORD_BYTES: usize = 4 + 12 + PLAYER_COUNT * 8;
/// Header bytes.
pub const HEADER_BYTES: usize = 64;
/// Trailer bytes.
pub const TRAILER_BYTES: usize = 16;
/// Schema version of the file layout.
pub const SCHEMA_VERSION: u16 = 4;
/// Bit 31 of the stored tick number carries the restart flag. A tick number never reaches
/// 2^31: the longest match the engine accepts is 200 minutes, which is 600,000 ticks.
const RESTART_BIT: u32 = 1 << 31;
/// Header offset of the 4-byte announced maximum tick count.
const EXPECTED_AT: u64 = 24;
/// Header offset of the 16-byte owner identifier.
const OWNER_AT: usize = 28;
/// Header offset of the 8-byte match stamp (milliseconds since the Unix epoch).
const MILLIS_AT: usize = 44;

const MAGIC: &[u8; 4] = b"SMTK";
const TRAILER_MAGIC: &[u8; 4] = b"SMTE";

/// One tick: the ball in three dimensions and every player on the ground plane.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TickRecord {
    pub tick: u32,
    pub ball: [f32; 3],
    pub players: [[f32; 2]; PLAYER_COUNT],
    /// True on a tick that places the ball for a restart (a kick-off, a throw-in, a corner, a
    /// goal kick, a free kick, or a penalty). The ball jumps to the restart spot, so the
    /// validator must not read the jump as a kick.
    #[serde(default)]
    pub restart: bool,
}

impl TickRecord {
    /// Serializes into a fixed little-endian layout.
    pub fn write_to(&self, buf: &mut [u8; RECORD_BYTES]) {
        let stamped = if self.restart {
            self.tick | RESTART_BIT
        } else {
            self.tick
        };
        buf[0..4].copy_from_slice(&stamped.to_le_bytes());
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
        let stamped = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let tick = stamped & !RESTART_BIT;
        let restart = stamped & RESTART_BIT != 0;
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
            restart,
        }
    }
}

/// The header of a `.ticks` file: what the reader needs to interpret the records, plus the
/// identity of the match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickHeader {
    pub seed: u64,
    pub dt: f64,
    /// The most ticks the match can last. The trailer holds the count actually written.
    pub expected_ticks: u32,
    /// The owner identifier as 16 bytes; see `observe::identity`.
    pub owner_id: [u8; 16],
    /// The match stamp; with `seed` it forms `match.id`.
    pub match_millis: u64,
}

impl TickHeader {
    fn write_to(&self, header: &mut [u8; HEADER_BYTES]) {
        header[0..4].copy_from_slice(MAGIC);
        header[4..6].copy_from_slice(&SCHEMA_VERSION.to_le_bytes());
        header[6] = 4;
        header[7] = PLAYER_COUNT as u8;
        header[8..16].copy_from_slice(&self.seed.to_le_bytes());
        header[16..24].copy_from_slice(&self.dt.to_le_bytes());
        header[24..28].copy_from_slice(&self.expected_ticks.to_le_bytes());
        header[OWNER_AT..OWNER_AT + 16].copy_from_slice(&self.owner_id);
        header[MILLIS_AT..MILLIS_AT + 8].copy_from_slice(&self.match_millis.to_le_bytes());
    }

    fn read_from(bytes: &[u8]) -> Result<Self, EngineError> {
        if &bytes[0..4] != MAGIC {
            return Err(EngineError::Format("bad magic".into()));
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != SCHEMA_VERSION {
            return Err(EngineError::Format(format!(
                "unknown schema version {version}; this build reads {SCHEMA_VERSION}"
            )));
        }
        if bytes[6] != 4 || bytes[7] as usize != PLAYER_COUNT {
            return Err(EngineError::Format(
                "unexpected float width or player count".into(),
            ));
        }
        Ok(Self {
            seed: u64::from_le_bytes(bytes[8..16].try_into().expect("8 bytes")),
            dt: f64::from_le_bytes(bytes[16..24].try_into().expect("8 bytes")),
            expected_ticks: u32::from_le_bytes(bytes[24..28].try_into().expect("4 bytes")),
            owner_id: bytes[OWNER_AT..OWNER_AT + 16].try_into().expect("16 bytes"),
            match_millis: u64::from_le_bytes(
                bytes[MILLIS_AT..MILLIS_AT + 8].try_into().expect("8 bytes"),
            ),
        })
    }
}

/// A consumer of tick records.
pub trait TickSink {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError>;

    /// The stoppage hook (named mechanism): called once for every stoppage, after the record
    /// of the tick the ball went dead on, with the match as it stands. The default does
    /// nothing.
    fn on_stoppage(&mut self, _stoppage: &Stoppage, _sim: &Simulation) -> Result<(), EngineError> {
        Ok(())
    }

    /// The debug trace hook: in debug mode, called after each step with the step's trace
    /// records (and once more after full time), before the tick's record. The default does
    /// nothing.
    fn on_trace(&mut self, _records: &[TraceRecord]) -> Result<(), EngineError> {
        Ok(())
    }
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

/// Feeds every record to two sinks, `a` then `b`, and returns the first error.
pub struct FanoutSink<A: TickSink, B: TickSink> {
    pub a: A,
    pub b: B,
}

impl<A: TickSink, B: TickSink> FanoutSink<A, B> {
    /// Joins two sinks. `a` receives each record first.
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }

    /// Returns both sinks, so each one can be finished on its own.
    pub fn into_parts(self) -> (A, B) {
        (self.a, self.b)
    }
}

impl<A: TickSink, B: TickSink> TickSink for FanoutSink<A, B> {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.a.on_tick(record)?;
        self.b.on_tick(record)
    }

    fn on_stoppage(&mut self, stoppage: &Stoppage, sim: &Simulation) -> Result<(), EngineError> {
        self.a.on_stoppage(stoppage, sim)?;
        self.b.on_stoppage(stoppage, sim)
    }

    fn on_trace(&mut self, records: &[TraceRecord]) -> Result<(), EngineError> {
        self.a.on_trace(records)?;
        self.b.on_trace(records)
    }
}

/// An absent sink discards every record, so a caller can hold an optional second sink
/// without branching on the hot path.
impl<S: TickSink> TickSink for Option<S> {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        match self {
            Some(sink) => sink.on_tick(record),
            None => Ok(()),
        }
    }

    fn on_stoppage(&mut self, stoppage: &Stoppage, sim: &Simulation) -> Result<(), EngineError> {
        match self {
            Some(sink) => sink.on_stoppage(stoppage, sim),
            None => Ok(()),
        }
    }

    fn on_trace(&mut self, records: &[TraceRecord]) -> Result<(), EngineError> {
        match self {
            Some(sink) => sink.on_trace(records),
            None => Ok(()),
        }
    }
}

/// Writes a `.ticks` file.
pub struct FileSink {
    writer: BufWriter<File>,
    written: u32,
    expected: u32,
    buf: [u8; RECORD_BYTES],
}

impl FileSink {
    /// Creates the file and writes the header.
    pub fn create(path: &Path, header: &TickHeader) -> Result<Self, EngineError> {
        let mut writer = BufWriter::with_capacity(1 << 20, File::create(path)?);
        let mut bytes = [0u8; HEADER_BYTES];
        header.write_to(&mut bytes);
        writer.write_all(&bytes)?;
        tracing::info!(
            signal = "tickfile.header",
            schema_version = SCHEMA_VERSION,
            seed = header.seed,
            dt_ms = header.dt * 1000.0,
            ticks_expected = header.expected_ticks,
            float_width = 4,
            owner_id = %hex(&header.owner_id),
            match_id = %format!("{:016x}-{}", header.seed, header.match_millis)
        );
        Ok(Self {
            writer,
            written: 0,
            expected: header.expected_ticks,
            buf: [0u8; RECORD_BYTES],
        })
    }

    /// Keeps the first `ticks` records and drops the rest, so a match rewound to an earlier
    /// tick writes on from there. A served match rewinds this way after a lost connection.
    pub fn rewind(&mut self, ticks: u32) -> Result<(), EngineError> {
        if ticks >= self.written {
            return Ok(());
        }
        self.writer.flush()?;
        let end = (HEADER_BYTES + ticks as usize * RECORD_BYTES) as u64;
        let file = self.writer.get_mut();
        file.set_len(end)?;
        file.seek(SeekFrom::Start(end))?;
        self.written = ticks;
        Ok(())
    }

    /// Writes the trailer and flushes. Returns the number of records written. A knockout
    /// match whose sudden death ran past the announced maximum raises the header's maximum
    /// to the count written, so the file still reads.
    pub fn finish(mut self) -> Result<u32, EngineError> {
        let mut trailer = [0u8; TRAILER_BYTES];
        trailer[0..4].copy_from_slice(TRAILER_MAGIC);
        trailer[4..8].copy_from_slice(&self.written.to_le_bytes());
        self.writer.write_all(&trailer)?;
        self.writer.flush()?;
        if self.written > self.expected {
            let file = self.writer.get_mut();
            file.seek(SeekFrom::Start(EXPECTED_AT))?;
            file.write_all(&self.written.to_le_bytes())?;
            file.flush()?;
        }
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

/// Lower-case hex of a byte slice.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A parsed `.ticks` file.
#[derive(Debug)]
pub struct TickFile {
    pub header: TickHeader,
    pub records: Vec<TickRecord>,
}

/// Reads and validates a `.ticks` file. Fails closed on any layout problem, and on a written
/// count above the most the header announced.
pub fn read_ticks(path: &Path) -> Result<TickFile, EngineError> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    if bytes.len() < HEADER_BYTES + TRAILER_BYTES {
        return Err(EngineError::Format(
            "file shorter than header plus trailer".into(),
        ));
    }
    let header = TickHeader::read_from(&bytes)?;
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
    if written > header.expected_ticks {
        return Err(EngineError::Format(format!(
            "record count {written} exceeds the {} the header announced",
            header.expected_ticks
        )));
    }
    let records = body
        .chunks_exact(RECORD_BYTES)
        .map(TickRecord::read_from)
        .collect();
    Ok(TickFile { header, records })
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

    fn header(seed: u64, expected_ticks: u32) -> TickHeader {
        TickHeader {
            seed,
            dt: 0.02,
            expected_ticks,
            owner_id: [0xab; 16],
            match_millis: 1_700_000_000_000,
        }
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
            restart: tick.is_multiple_of(3),
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
        let mut sink = FileSink::create(&path, &header(42, 1000)).unwrap();
        for i in 0..1000 {
            sink.on_tick(&sample(i)).unwrap();
        }
        assert_eq!(sink.finish().unwrap(), 1000);
        let file = read_ticks(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(file.header, header(42, 1000));
        assert_eq!(file.records.len(), 1000);
        assert_eq!(file.records[999], sample(999));
    }

    #[test]
    fn a_rewound_file_keeps_its_first_records_and_writes_on_from_there() {
        let path = temp("rewind");
        let mut sink = FileSink::create(&path, &header(9, 10)).unwrap();
        for i in 1..=8 {
            sink.on_tick(&sample(i)).unwrap();
        }
        sink.rewind(5).unwrap();
        for i in 6..=10 {
            sink.on_tick(&sample(i)).unwrap();
        }
        assert_eq!(sink.finish().unwrap(), 10);
        let file = read_ticks(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let ticks: Vec<u32> = file.records.iter().map(|r| r.tick).collect();
        assert_eq!(ticks, (1..=10).collect::<Vec<u32>>());
    }

    #[test]
    fn the_restart_flag_survives_the_file_and_leaves_the_tick_number_alone() {
        let path = temp("restart");
        let mut sink = FileSink::create(&path, &header(3, 4)).unwrap();
        for i in 1..=4 {
            sink.on_tick(&sample(i)).unwrap();
        }
        sink.finish().unwrap();
        let file = read_ticks(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let flags: Vec<(u32, bool)> = file.records.iter().map(|r| (r.tick, r.restart)).collect();
        assert_eq!(flags, vec![(1, false), (2, false), (3, true), (4, false)]);
    }

    #[test]
    fn a_file_past_its_announced_maximum_reads_once_the_writer_raises_it() {
        let path = temp("maximum");
        let mut sink = FileSink::create(&path, &header(5, 10)).unwrap();
        for i in 1..=4 {
            sink.on_tick(&sample(i)).unwrap();
        }
        sink.finish().unwrap();
        assert_eq!(read_ticks(&path).unwrap().records.len(), 4);
        // A writer that runs past its announcement (a long sudden death) raises the header's
        // figure, so the file reads.
        let mut sink = FileSink::create(&path, &header(5, 2)).unwrap();
        for i in 1..=3 {
            sink.on_tick(&sample(i)).unwrap();
        }
        sink.finish().unwrap();
        let file = read_ticks(&path).unwrap();
        assert_eq!(file.header.expected_ticks, 3);
        assert_eq!(file.records.len(), 3);
        // A file whose header still announces fewer ticks than it holds is refused.
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[EXPECTED_AT as usize..EXPECTED_AT as usize + 4].copy_from_slice(&2u32.to_le_bytes());
        std::fs::write(&path, bytes).unwrap();
        let err = read_ticks(&path).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert!(err.to_string().contains("exceeds the 2"), "{err}");
    }

    #[test]
    fn a_fan_out_feeds_both_sinks_in_order() {
        let mut sink = FanoutSink::new(VecSink::default(), VecSink::default());
        sink.on_tick(&sample(1)).unwrap();
        sink.on_tick(&sample(2)).unwrap();
        let (a, b) = sink.into_parts();
        assert_eq!(a.records, b.records);
        assert_eq!(a.records.len(), 2);
    }

    #[test]
    fn an_absent_optional_sink_discards_every_record() {
        let mut none: Option<VecSink> = None;
        none.on_tick(&sample(1)).unwrap();
        let mut some = Some(VecSink::default());
        some.on_tick(&sample(1)).unwrap();
        assert_eq!(some.unwrap().records.len(), 1);
    }

    #[test]
    fn a_truncated_file_is_refused() {
        let path = temp("truncated");
        let mut sink = FileSink::create(&path, &header(1, 10)).unwrap();
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

    #[test]
    fn a_version_three_file_is_refused_naming_both_versions() {
        let path = temp("v1");
        let sink = FileSink::create(&path, &header(1, 0)).unwrap();
        sink.finish().unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[4..6].copy_from_slice(&3u16.to_le_bytes());
        std::fs::write(&path, &bytes).unwrap();
        let err = read_ticks(&path).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            err.to_string(),
            "tick file format error: unknown schema version 3; this build reads 4"
        );
    }
}
