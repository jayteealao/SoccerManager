//! The debug trace file that `simulate --debug-trace` and `resimulate --debug-trace` write:
//! JSON Lines, a header line first, then every record as the match hands it over after each
//! step. At the end the recorded draw count is checked against the registry's.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use engine::trace::{self, Counts, TraceHeader, TraceRecord};
use engine::{EngineError, Simulation, TickRecord, TickSink};

/// Writes the debug trace of one match.
pub struct TraceFile {
    path: PathBuf,
    writer: BufWriter<File>,
    counts: Counts,
}

impl TraceFile {
    pub fn create(path: &Path, seed: u64, sim: &Simulation) -> anyhow::Result<Self> {
        let file =
            File::create(path).with_context(|| format!("cannot create {}", path.display()))?;
        let mut writer = BufWriter::with_capacity(1 << 20, file);
        let header = TraceHeader {
            seed,
            scheme: sim.stream_scheme(),
        };
        trace::write_header(&mut writer, &header)?;
        Ok(Self {
            path: path.to_path_buf(),
            writer,
            counts: Counts::default(),
        })
    }

    /// Flushes the file and reports the counts; unequal draw counts are an error.
    pub fn finish(mut self, registry: u64) -> anyhow::Result<()> {
        self.writer
            .flush()
            .with_context(|| format!("cannot write {}", self.path.display()))?;
        let c = self.counts;
        eprintln!(
            "debug trace: {}, {} draws (registry {registry}), {} decisions, {} rule outcomes",
            self.path.display(),
            c.draws,
            c.decisions,
            c.rules
        );
        if c.draws != registry {
            bail!(
                "the debug trace recorded {} draws but the registry served {registry}",
                c.draws
            );
        }
        Ok(())
    }
}

impl TickSink for TraceFile {
    fn on_tick(&mut self, _record: &TickRecord) -> Result<(), EngineError> {
        Ok(())
    }

    fn on_trace(&mut self, records: &[TraceRecord]) -> Result<(), EngineError> {
        self.counts.add(records);
        trace::write_records(&mut self.writer, records)?;
        Ok(())
    }
}

/// The key of a trace record's tick, as `engine::trace` writes it.
const TICK_KEY: &str = "t";

/// The records of `tick` in trace file text: every line after the header whose tick is
/// `tick`. Lines that are not JSON are skipped.
pub fn tick_records(text: &str, tick: u32) -> Vec<serde_json::Value> {
    text.lines()
        .skip(1)
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|r| r[TICK_KEY].as_u64() == Some(u64::from(tick)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::trace::{Point, PointRecord};

    #[test]
    fn written_records_are_found_by_their_tick() {
        let header = TraceHeader { seed: 1, scheme: 1 };
        let record = |tick| {
            TraceRecord::Point(PointRecord {
                tick,
                point: Point::Carrier,
                detail: serde_json::json!({}),
            })
        };
        let mut w = Vec::new();
        trace::write_header(&mut w, &header).unwrap();
        trace::write_records(&mut w, &[record(2), record(3), record(3)]).unwrap();
        let text = String::from_utf8(w).unwrap();
        assert_eq!(tick_records(&text, 3).len(), 2);
        assert_eq!(tick_records(&text, 2).len(), 1);
        assert!(tick_records(&text, 4).is_empty());
    }
}
