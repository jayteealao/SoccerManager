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

use std::path::Path;

use protocol::{Frame, ServerMessage};
use script::Backstop;
use sha2::{Digest, Sha256};
use stream::session::{FrameOut, FrameSink, MatchState};
use stream::{
    ChangeEntry, ChangeSource, EngineIdentity, ReplayRecord, StreamError, outcome_of, read_fixture,
};

use crate::cli::ResimulateOpts;
use crate::replay_inputs::{Built, InputFiles, build};
use crate::stream_run::{Drive, Planned, drive};

pub fn run(content_dir: Option<&Path>, opts: &ResimulateOpts) -> anyhow::Result<i32> {
    if content_dir.is_some() {
        eprintln!("error: resimulate reads every input from the replay file; drop --content-dir");
        return Ok(1);
    }
    let shown = opts.fixture.display().to_string();
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
    } = build(&inputs, &record.settings, Backstop::Never)?;
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
    let ticks = sim.config().max_ticks();
    let club_ids = [
        sim.config().teams[0].club_id.clone(),
        sim.config().teams[1].club_id.clone(),
    ];
    let state = MatchState::default();
    let mut sink = FrameSink::new(Compare::new(&stored), keyframe_interval);
    let mut text_frames = 0u64;
    drive(
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
        },
        &mut |_: ServerMessage| {
            text_frames += 1;
            Ok(())
        },
    )?;
    let compare = sink.into_inner().finish();
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
    tracing::debug!(signal = "resimulate.text_frames", count = text_frames);
    println!("{line}");
    Ok(if identical { 0 } else { 2 })
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
