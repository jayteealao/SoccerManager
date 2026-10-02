//! Changes planned for a fixed tick, by lineup slot and bench place: the gate's change
//! fixture, a recorded match's change list, and a streamed match's planned changes all queue
//! them through the same type.

use serde::{Deserialize, Serialize};

use crate::sim::Simulation;
use crate::tactics::TacticsPatch;
use crate::tactics::change::Change;

/// A change planned for a tick: the gate's change fixture and a recorded or streamed match
/// that carries a change list queue it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedWhat {
    /// The player in lineup `slot` off, the bench player at `bench` on.
    Substitution { slot: usize, bench: usize },
    /// The team's mentality set to this index of the tactics file.
    Mentality(u8),
}

/// One planned change: queued before the step after tick `tick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedChange {
    pub tick: u32,
    pub team: usize,
    pub change: PlannedWhat,
}

impl PlannedChange {
    /// The engine change this planned change names in `sim` as it stands now: a lineup slot
    /// or a bench place outside the team becomes a squad index no team holds, which the
    /// engine rejects at the stoppage.
    pub fn to_change(&self, sim: &Simulation) -> Change {
        match self.change {
            PlannedWhat::Substitution { slot, bench } => {
                let team = &sim.teams[self.team];
                Change::Substitution {
                    off: team.lineup.get(slot).copied().unwrap_or(usize::MAX),
                    on: team.bench.get(bench).copied().unwrap_or(usize::MAX),
                }
            }
            PlannedWhat::Mentality(m) => Change::Tactics(TacticsPatch::mentality(m)),
        }
    }
}
