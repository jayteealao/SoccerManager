//! The `match-event` record kind. The observability contract reserves it and no code has
//! written it until now. Rows append to `matches/<match.id>/events.jsonl` through one
//! writer opened at kick-off, so a crash keeps every event up to the last write.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use engine::observe::{Record, to_json};
use protocol::MatchEvent;
use serde::{Serialize, Serializer};

use crate::StreamError;

/// File that holds one match's events, beside `stats.json`.
pub const EVENTS_FILE: &str = "events.jsonl";

/// A match event wearing the engine's observability envelope. The wrapper lives here
/// because `crates/protocol` performs no input or output and must not depend on the
/// engine; a local type is also what the orphan rule allows.
pub struct EventRecord<'a>(pub &'a MatchEvent);

impl Serialize for EventRecord<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl Record for EventRecord<'_> {
    fn kind(&self) -> &'static str {
        "match-event"
    }

    fn operation(&self) -> &'static str {
        "simulate"
    }

    fn owner_id(&self) -> &str {
        &self.0.owner_id
    }
}

/// The event as one JSON Lines row, with the envelope keys first.
pub fn to_row(event: &MatchEvent) -> Result<String, StreamError> {
    Ok(to_json(&EventRecord(event))?)
}

/// The append-only writer for one match.
pub struct EventWriter {
    writer: BufWriter<File>,
    path: PathBuf,
    written: u32,
}

impl EventWriter {
    /// Creates `matches/<match_id>/events.jsonl` under `data_dir`.
    pub fn open(data_dir: &Path, match_id: &str) -> Result<Self, StreamError> {
        Self::create_at(&data_dir.join("matches").join(match_id).join(EVENTS_FILE))
    }

    /// Creates the events file at `path`, and its folder when absent.
    pub fn create_at(path: &Path) -> Result<Self, StreamError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| StreamError::io("cannot create the match folder", e))?;
        }
        let path = path.to_path_buf();
        let file = File::create(&path)
            .map_err(|e| StreamError::io(format!("cannot create {EVENTS_FILE}"), e))?;
        Ok(Self {
            writer: BufWriter::new(file),
            path,
            written: 0,
        })
    }

    /// Opens the events file of a match resumed at `tick`. Every row at or before that tick
    /// is kept and every later row is dropped, because the resumed match plays those ticks
    /// again. The kept rows go to a temporary file that then replaces the old one, so a crash
    /// part-way never leaves a half-written file. A missing file starts empty.
    pub fn resume(data_dir: &Path, match_id: &str, tick: u32) -> Result<Self, StreamError> {
        let path = data_dir.join("matches").join(match_id).join(EVENTS_FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(StreamError::io(format!("cannot read {EVENTS_FILE}"), e)),
        };
        let mut kept = String::with_capacity(text.len());
        let (mut rows, mut dropped) = (0u32, 0u32);
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let row_tick = serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|row| row.get("tick").and_then(serde_json::Value::as_u64));
            if row_tick.is_some_and(|t| t <= u64::from(tick)) {
                kept.push_str(line);
                kept.push('\n');
                rows += 1;
            } else {
                dropped += 1;
            }
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| StreamError::io("cannot create the match folder", e))?;
        }
        let temporary = path.with_extension("jsonl.tmp");
        std::fs::write(&temporary, kept.as_bytes())
            .and_then(|()| std::fs::rename(&temporary, &path))
            .map_err(|e| StreamError::io(format!("cannot rewrite {EVENTS_FILE}"), e))?;
        let file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .map_err(|e| StreamError::io(format!("cannot open {EVENTS_FILE}"), e))?;
        tracing::info!(
            signal = "events.resumed",
            match.id = %match_id,
            tick,
            kept = rows,
            dropped
        );
        Ok(Self {
            writer: BufWriter::new(file),
            path,
            written: rows,
        })
    }

    /// Appends one row and flushes it, so a crash keeps every event already written.
    pub fn write(&mut self, event: &MatchEvent) -> Result<(), StreamError> {
        let row = to_row(event)?;
        self.writer
            .write_all(row.as_bytes())
            .and_then(|()| self.writer.write_all(b"\n"))
            .and_then(|()| self.writer.flush())
            .map_err(|e| StreamError::io(format!("cannot write {EVENTS_FILE}"), e))?;
        self.written += 1;
        Ok(())
    }

    /// The number of rows written.
    pub fn written(&self) -> u32 {
        self.written
    }

    /// The file this writer appends to.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::EventType;

    #[test]
    fn a_goal_row_carries_the_envelope_and_both_scores() {
        let event = MatchEvent::play(
            "0123456789abcdef0123456789abcdef",
            "000000000000002a-1700000000000",
            9_000,
            EventType::Goal,
            Some("club-a".into()),
            [1, 0],
        );
        let row = to_row(&event).unwrap();
        for key in [
            "\"record.kind\":\"match-event\"",
            "\"schema.version\":\"1\"",
            "\"owner.id\":\"0123456789abcdef0123456789abcdef\"",
            "\"service\":\"engine-cli\"",
            "\"build.hash\"",
            "\"operation\":\"simulate\"",
            "\"event.type\":\"goal\"",
            "\"home.score\":1",
            "\"away.score\":0",
            "\"minute\":3",
        ] {
            assert!(row.contains(key), "missing {key} in {row}");
        }
    }

    #[test]
    fn rows_append_one_per_line() {
        let dir = std::env::temp_dir().join(format!("stream-events-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut writer = EventWriter::open(&dir, "match-1").unwrap();
        for tick in [1u32, 9_000] {
            writer
                .write(&MatchEvent::play(
                    "owner",
                    "match-1",
                    tick,
                    EventType::KickOff,
                    None,
                    [0, 0],
                ))
                .unwrap();
        }
        let text = std::fs::read_to_string(writer.path()).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert_eq!(writer.written(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
