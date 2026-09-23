//! Snapshots for a streamed match, written only once the page holds everything before them.
//!
//! A snapshot does not store the events already handed out, and the events of a tick travel
//! after that tick's frame. A snapshot written the moment play stops could therefore describe
//! a goal the page never received, and a match resumed from it would show the wrong score.
//! This sink captures a snapshot at every stoppage, exactly as the plain snapshot sink does,
//! and writes it to disk only after the socket has flushed a later tick frame. The producer
//! buffer keeps order, so by then every event of the stoppage tick is on the wire too.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine::record::{TickRecord, TickSink};
use engine::snapshot::FILE_NAME;
use engine::{EngineError, Simulation, Snapshot, Stoppage};

use crate::session::MatchState;

/// Captured snapshots kept for a reconnect. A stoppage every few seconds and a producer at
/// most ten seconds ahead of the socket keep far fewer than this in flight.
pub const RING: usize = 16;

/// One captured snapshot and whether it is on disk.
struct Held {
    snapshot: Snapshot,
    written: bool,
}

/// Captures at every stoppage; persists the newest capture the socket has passed.
pub struct GatedSnapshots {
    path: PathBuf,
    match_id: String,
    owner_id: [u8; 16],
    match_millis: u64,
    state: Arc<MatchState>,
    ring: VecDeque<Held>,
    /// Snapshots written.
    pub writes: u32,
}

impl GatedSnapshots {
    /// A sink for one match, writing `SM_DATA_DIR/matches/<match_id>/snapshot.smsn`.
    pub fn new(
        data_dir: &Path,
        match_id: &str,
        owner_id: [u8; 16],
        match_millis: u64,
        state: Arc<MatchState>,
    ) -> Self {
        Self {
            path: data_dir.join("matches").join(match_id).join(FILE_NAME),
            match_id: match_id.to_string(),
            owner_id,
            match_millis,
            state,
            ring: VecDeque::with_capacity(RING),
            writes: 0,
        }
    }

    /// Where the snapshot is written.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The newest captured snapshot taken before `sent_tick`: the page holds every frame and
    /// every event up to and including its tick. `None` when no stoppage came before it.
    pub fn newest_before(&self, sent_tick: u32) -> Option<&Snapshot> {
        self.ring
            .iter()
            .rev()
            .map(|held| &held.snapshot)
            .find(|snapshot| snapshot.tick() < sent_tick)
    }

    /// Snapshots held in memory.
    pub fn held(&self) -> usize {
        self.ring.len()
    }

    /// Captures `sim` as it stands. Every stoppage does; the match does once more at
    /// kick-off, so a crash before the first stoppage still has a point to restart from.
    pub fn capture(&mut self, sim: &Simulation) {
        if self.ring.len() == RING {
            self.ring.pop_front();
        }
        self.ring.push_back(Held {
            snapshot: Snapshot::capture(sim, self.owner_id, self.match_millis),
            written: false,
        });
    }

    /// Forgets every capture after `tick`, where a match resumes. Those captures describe
    /// play the resumed match is about to produce again.
    pub fn rewind(&mut self, tick: u32) {
        self.ring.retain(|held| held.snapshot.tick() <= tick);
    }

    /// Writes the newest capture the socket has passed, when it is not on disk yet.
    fn persist(&mut self, now: u32) {
        let sent = self.state.sent_tick();
        let Some(index) = self
            .ring
            .iter()
            .rposition(|held| held.snapshot.tick() < sent)
        else {
            return;
        };
        if self.ring[index].written {
            return;
        }
        // Every older capture is superseded by this one.
        for held in self.ring.iter_mut().take(index + 1) {
            held.written = true;
        }
        let snapshot = &self.ring[index].snapshot;
        let tick = snapshot.tick();
        match snapshot.write_atomic(&self.path) {
            Ok(bytes) => {
                self.writes += 1;
                tracing::debug!(
                    signal = "snapshot.written",
                    match.id = %self.match_id,
                    snapshot.tick = tick,
                    lag_ticks = now.saturating_sub(tick),
                    bytes
                );
            }
            Err(err) => {
                tracing::error!(
                    signal = "snapshot.write_failed",
                    match.id = %self.match_id,
                    snapshot.tick = tick,
                    path = %format!("matches/{}/{FILE_NAME}", self.match_id),
                    reason = %err
                );
            }
        }
    }
}

impl TickSink for GatedSnapshots {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.persist(record.tick);
        Ok(())
    }

    fn on_stoppage(&mut self, _stoppage: &Stoppage, sim: &Simulation) -> Result<(), EngineError> {
        self.capture(sim);
        Ok(())
    }
}

/// The sink outlives each connection of a match that reconnects, so a session borrows it.
impl TickSink for &mut GatedSnapshots {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        (**self).on_tick(record)
    }

    fn on_stoppage(&mut self, stoppage: &Stoppage, sim: &Simulation) -> Result<(), EngineError> {
        (**self).on_stoppage(stoppage, sim)
    }
}
