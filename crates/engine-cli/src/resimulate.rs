//! `engine-cli resimulate`: play a recorded match again from its replay file alone and
//! compare it with the stored frames.
//!
//! Every input comes from the file. The match is built by the same function `record` used
//! and driven by the same driver, each manager change of the log is queued again at its
//! recorded tick, and every regenerated tick frame is compared with the stored one, byte for
//! byte. Text frames are not compared: they carry the recording's stamp, and the verdict rows
//! of rejected changes, which the log does not keep. After full time the applied changes must
//! equal the log, entry by entry.
//!
//! Strict mode first checks the engine identity: another executable SHA-256 or another
//! stream scheme is refused, naming each difference. Comparison mode never refuses on the
//! identity; it reports both identities and both schemes, so two engine versions can be
//! compared on one record.
//!
//! Three outputs let another tool compare two engine versions tick by tick, each off by
//! default: `--state-digests` writes the SHA-256 of the full match state after every tick
//! (the bytes the replay gate hashes), `--state-fields` with `--at-tick` writes the named
//! parts of the state of one tick and stops there, and `--debug-trace` plays the match with
//! debug mode on and writes its trace.

use std::cell::RefCell;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use engine::gate::{GATE_SCHEMA, INVENTORY_VERSION, STATE_DIGEST_FORMAT, StateWriter};
use engine::{EngineEvent, FanoutSink, Simulation, trace};
use protocol::{Frame, ServerMessage};
use script::Backstop;
use sha2::{Digest, Sha256};
use stream::session::{FrameOut, FrameSink, MatchState};
use stream::{
    ChangeEntry, ChangeSource, EngineIdentity, ReplayRecord, StreamError, outcome_of, read_fixture,
};

use crate::cli::ResimulateOpts;
use crate::replay_inputs::{Built, InputFiles, build};
use crate::stream_run::{Drive, Observe, Planned, drive};
use crate::trace_file::TraceFile;

pub fn run(content_dir: Option<&Path>, opts: &ResimulateOpts) -> anyhow::Result<i32> {
    if content_dir.is_some() {
        eprintln!("error: resimulate reads every input from the replay file; drop --content-dir");
        return Ok(1);
    }
    let shown = opts.fixture.display().to_string();
    if opts.debug_trace.is_some() && !trace::COMPILED {
        eprintln!(
            "error: --debug-trace needs a build with the debug trace (the debug-trace feature)"
        );
        return Ok(1);
    }
    let fixture = match read_fixture(&opts.fixture) {
        Ok(fixture) => fixture,
        Err(err) => {
            eprintln!("error: cannot re-simulate {shown}: {err}");
            return Ok(1);
        }
    };
    let Some(record) = fixture.record.clone() else {
        eprintln!(
            "error: {shown} holds no inputs: it is a version-3 replay file, which plays from \
             its frames only"
        );
        return Ok(1);
    };
    let here = EngineIdentity::current()?;
    if opts.compare {
        eprintln!("engine (record): {}", described(&record.engine));
        eprintln!("engine (this binary): {}", described(&here));
        eprintln!(
            "scheme {} (record), scheme {} (this binary)",
            record.engine.scheme, here.scheme
        );
    } else {
        let differences = identity_differences(&record, &here);
        if !differences.is_empty() {
            for difference in &differences {
                eprintln!("error: {difference}");
            }
            eprintln!(
                "error: {shown} was recorded on another engine; --compare runs it anyway and \
                 reports both"
            );
            return Ok(1);
        }
        if record.engine.dirty {
            eprintln!(
                "warning: recorded on a dirty build ({}); that version cannot be rebuilt",
                record.engine.build
            );
        }
    }

    let inputs = InputFiles::from_record(&fixture)?;
    // The watchdog mark depends on wall-clock time and never changes play, so the re-run
    // reads no clock and the recorded mark is reported.
    let Built {
        mut sim,
        commentary,
        keyframe_interval,
    } = build(
        &inputs,
        &record.settings,
        Backstop::Never,
        opts.debug_trace.is_some(),
    )?;
    let mut manager_changes: Vec<&ChangeEntry> = record
        .changes
        .iter()
        .filter(|c| c.source == ChangeSource::Manager)
        .collect();
    manager_changes.sort_by_key(|c| (c.queued_tick, c.queue_number));
    let planned: Vec<Planned> = manager_changes
        .iter()
        .map(|c| Planned::Exact {
            tick: c.queued_tick,
            team: c.team,
            change: c.change.to_change(),
        })
        .collect();

    let stored: Vec<(u32, &[u8])> = fixture
        .frames
        .iter()
        .filter(|f| !f.frame.is_text())
        .map(|f| (f.tick, f.frame.payload()))
        .collect();
    let ticks = opts.at_tick.unwrap_or_else(|| sim.config().max_ticks());
    let club_ids = [
        sim.config().teams[0].club_id.clone(),
        sim.config().teams[1].club_id.clone(),
    ];
    let state = MatchState::default();
    let traced = match &opts.debug_trace {
        Some(path) => Some(TraceFile::create(path, record.settings.seed, &sim)?),
        None => None,
    };
    let mut sink = FanoutSink::new(
        FrameSink::new(Compare::new(&stored), keyframe_interval),
        traced,
    );
    let mut outputs = Outputs::new(opts, &sim, &here)?;
    let observed = RefCell::new(|sim: &Simulation, events: &[EngineEvent], finished: bool| {
        outputs.on_state(sim, events, finished);
    });
    let mut text_frames = 0u64;
    let driven = drive(
        &mut sim,
        &mut sink,
        &Drive {
            ticks,
            owner_id: "00000000000000000000000000000000",
            match_id: "resimulate",
            club_ids: [&club_ids[0], &club_ids[1]],
            state: &state,
            gate: None,
            commentary: &commentary,
            inbox: None,
            page_changes: None,
            planned: &planned,
            observe: outputs_wanted(opts).then_some(&observed as &Observe<'_>),
        },
        &mut |_: ServerMessage| {
            text_frames += 1;
            Ok(())
        },
    )?;
    let (frames, traced) = sink.into_parts();
    if let Some(traced) = traced {
        traced.finish(sim.draws())?;
    }
    if let Err(message) = outputs.finish(driven.full_time, &here) {
        eprintln!("error: {message:#}");
        return Ok(1);
    }
    let compare = frames.into_inner().finish();
    let now = outcome_of(&sim).changes;
    let change_difference = first_change_difference(&record.changes, &now);
    let identical = compare.first.is_none() && change_difference.is_none();
    let first_difference = match (compare.first, change_difference) {
        (Some((frame, tick)), _) => serde_json::json!({ "frame": frame, "tick": tick }),
        (None, Some(order)) => serde_json::json!({ "change": order }),
        (None, None) => serde_json::Value::Null,
    };
    let mut line = serde_json::json!({
        "fixture": shown,
        "mode": if opts.compare { "compare" } else { "strict" },
        "verdict": if identical { "identical" } else { "differs" },
        "tick_frames": stored.len(),
        "stored_sha256": compare.stored_sha256,
        "resimulated_sha256": compare.resimulated_sha256,
        "first_difference": first_difference,
        "changes_applied": now.len(),
        "inputs_bytes": record.inputs_bytes,
        "watchdog": record.watchdog,
    });
    if opts.compare {
        line["engines"] = serde_json::json!({ "record": record.engine, "this_binary": here });
    }
    if let Some(tick) = opts.at_tick {
        line["stopped_at"] = serde_json::json!(tick);
    }
    tracing::debug!(signal = "resimulate.text_frames", count = text_frames);
    println!("{line}");
    Ok(if identical { 0 } else { 2 })
}

/// `true` when a state output is asked for, so the state is written after each tick.
fn outputs_wanted(opts: &ResimulateOpts) -> bool {
    opts.state_digests.is_some() || opts.state_fields.is_some()
}

/// The state outputs of a re-simulation: the digest of each tick's state, and the named
/// parts of one tick's state.
struct Outputs {
    state: StateWriter,
    digests: Option<(PathBuf, BufWriter<File>)>,
    /// Tick lines written so far.
    tick_lines: u64,
    fields: Option<(PathBuf, u32)>,
    /// The fields of the asked tick, once it has played.
    found: Option<serde_json::Value>,
    scheme: u8,
    /// The first write error; the run stops reporting after it.
    failed: Option<std::io::Error>,
}

impl Outputs {
    /// Opens the asked outputs and writes the digest file's header line.
    fn new(opts: &ResimulateOpts, sim: &Simulation, here: &EngineIdentity) -> anyhow::Result<Self> {
        let scheme = sim.stream_scheme();
        let digests = match &opts.state_digests {
            Some(path) => {
                let file = File::create(path)
                    .with_context(|| format!("cannot create {}", path.display()))?;
                let mut w = BufWriter::with_capacity(1 << 16, file);
                let header = serde_json::json!({
                    "state_digests": STATE_DIGEST_FORMAT,
                    "inventory": INVENTORY_VERSION,
                    "gate_schema": GATE_SCHEMA,
                    "scheme": scheme,
                    "engine": here,
                });
                writeln!(w, "{header}")
                    .with_context(|| format!("cannot write {}", path.display()))?;
                Some((path.clone(), w))
            }
            None => None,
        };
        let fields = match (&opts.state_fields, opts.at_tick) {
            (Some(path), Some(tick)) => Some((path.clone(), tick)),
            _ => None,
        };
        Ok(Self {
            state: StateWriter::new(fields.is_some()),
            digests,
            tick_lines: 0,
            fields,
            found: None,
            scheme,
            failed: None,
        })
    }

    /// Writes the state after one step, or after full time when `finished`.
    fn on_state(&mut self, sim: &Simulation, events: &[EngineEvent], finished: bool) {
        if self.failed.is_some() {
            return;
        }
        let bytes = self.state.tick(sim, events);
        if let Some((_, w)) = &mut self.digests {
            let digest = stream::record::hex(&Sha256::digest(bytes));
            let written = if finished {
                writeln!(w, "finish {digest}")
            } else {
                self.tick_lines += 1;
                writeln!(w, "{} {digest}", sim.tick())
            };
            if let Err(err) = written {
                self.failed = Some(err);
                return;
            }
        }
        if let Some((_, at)) = self.fields
            && !finished
            && sim.tick() == at
        {
            let bytes = self.state.bytes();
            let fields: Vec<serde_json::Value> = self
                .state
                .fields()
                .into_iter()
                .map(|f| {
                    serde_json::json!({
                        "name": f.name,
                        "kind": f.kind.as_str(),
                        "hex": stream::record::hex(&bytes[f.range]),
                    })
                })
                .collect();
            self.found = Some(serde_json::json!({ "tick": at, "fields": fields }));
        }
    }

    /// Closes the digest file with its `end` line, written only after every other line is
    /// flushed, and writes the fields file.
    fn finish(self, full_time: bool, here: &EngineIdentity) -> anyhow::Result<()> {
        if let Some(err) = self.failed {
            let path = self.digests.map(|(path, _)| path).unwrap_or_default();
            return Err(err).with_context(|| format!("cannot write {}", path.display()));
        }
        if let Some((path, mut w)) = self.digests {
            let cannot = || format!("cannot write {}", path.display());
            w.flush().with_context(cannot)?;
            writeln!(w, "end {} {full_time}", self.tick_lines).with_context(cannot)?;
            w.flush().with_context(cannot)?;
        }
        if let Some((path, at)) = self.fields {
            let Some(mut found) = self.found else {
                anyhow::bail!("the match ended before tick {at}; no state fields were written");
            };
            found["scheme"] = serde_json::json!(self.scheme);
            found["engine"] = serde_json::json!(here);
            std::fs::write(&path, format!("{found}\n"))
                .with_context(|| format!("cannot write {}", path.display()))?;
        }
        Ok(())
    }
}

/// Each way the running binary differs from the one that recorded the match, in words.
fn identity_differences(record: &ReplayRecord, here: &EngineIdentity) -> Vec<String> {
    let mut out = Vec::new();
    if record.engine.executable_sha256 != here.executable_sha256 {
        out.push(format!(
            "executable SHA-256 differs: the record has {}, this binary is {}",
            record.engine.executable_sha256, here.executable_sha256
        ));
    }
    if record.engine.scheme != here.scheme {
        out.push(format!(
            "scheme {} differs from this build's {}",
            record.engine.scheme, here.scheme
        ));
    }
    out
}

/// One engine identity on one line.
fn described(engine: &EngineIdentity) -> String {
    format!(
        "sha256 {} commit {}{} crate {} scheme {} maths {}",
        engine.executable_sha256,
        engine.commit,
        if engine.dirty { " (dirty)" } else { "" },
        engine.crate_version,
        engine.scheme,
        engine.maths
    )
}

/// The order of the first log entry the re-run did not apply the same way: the same team,
/// source, change, tick, stoppage, and place. The queue numbers are not compared: rejected
/// changes took numbers in the recording and are not queued again.
fn first_change_difference(recorded: &[ChangeEntry], now: &[ChangeEntry]) -> Option<usize> {
    let same = |a: &ChangeEntry, b: &ChangeEntry| {
        (a.order, a.team, a.source, a.tick, &a.stoppage, &a.change)
            == (b.order, b.team, b.source, b.tick, &b.stoppage, &b.change)
    };
    recorded
        .iter()
        .zip(now)
        .position(|(a, b)| !same(a, b))
        .or_else(|| (recorded.len() != now.len()).then(|| recorded.len().min(now.len())))
}

/// Compares every regenerated tick frame with the stored one and keeps the first
/// difference: the frame's place among the tick frames and its stored tick.
struct Compare<'a> {
    stored: &'a [(u32, &'a [u8])],
    next: usize,
    first: Option<(usize, u32)>,
    resimulated: Sha256,
}

/// What a comparison found.
struct Compared {
    first: Option<(usize, u32)>,
    stored_sha256: String,
    resimulated_sha256: String,
}

impl<'a> Compare<'a> {
    fn new(stored: &'a [(u32, &'a [u8])]) -> Self {
        Self {
            stored,
            next: 0,
            first: None,
            resimulated: Sha256::new(),
        }
    }

    /// The first difference, counting a missing or an extra frame, and both hashes.
    fn finish(mut self) -> Compared {
        if self.first.is_none() && self.next != self.stored.len() {
            let at = self.next.min(self.stored.len());
            let tick = self.stored.get(at).map_or(0, |&(tick, _)| tick);
            self.first = Some((at, tick));
        }
        let mut stored = Sha256::new();
        for (_, payload) in self.stored {
            stored.update(payload);
        }
        Compared {
            first: self.first,
            stored_sha256: stream::record::hex(&stored.finalize()),
            resimulated_sha256: stream::record::hex(&self.resimulated.finalize()),
        }
    }
}

impl FrameOut for Compare<'_> {
    fn send(&mut self, frame: Frame) -> Result<(), StreamError> {
        let payload = frame.payload();
        self.resimulated.update(payload);
        let at = self.next;
        self.next += 1;
        if self.first.is_none() {
            match self.stored.get(at) {
                Some(&(_, stored)) if stored == payload => {}
                Some(&(tick, _)) => self.first = Some((at, tick)),
                None => self.first = Some((at, 0)),
            }
        }
        Ok(())
    }
}
