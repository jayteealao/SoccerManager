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
