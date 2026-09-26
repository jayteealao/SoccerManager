//! Test faults for the gate (engine tests only, under the `scenario` feature). A fault
//! changes one named state value right after the state of its tick is hashed, so its first
//! effect shows in the next tick's state and at the next checkpoint.

use crate::sim::Simulation;
use crate::tactics::TacticsPatch;
use crate::tactics::change::{Change, ChangeId, QueuedChange};

use super::{Fixture, Inputs, MatchHashes, Played, Probe};

/// One fault at one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fault {
    pub at_tick: u32,
    pub kind: FaultKind,
}

/// What a fault changes, by inventory group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FaultKind {
    /// G3: flips bit `bit` of the x velocity of roster player `player`.
    VelocityBit { player: usize, bit: u32 },
    /// G10: one extra draw on the match's random stream.
    ExtraDraw,
    /// G3: adds `delta` to the energy of roster player `player`.
    Stamina { player: usize, delta: f64 },
    /// G5: counts one more yellow card for the team of roster player `player` in the summary
    /// and in the half's card tally.
    Card { player: usize },
    /// G4: names roster player `player` as the restart taker, or clears the taker when
    /// one is named.
    Restart { player: usize },
    /// G8: puts an empty tactics change for the home team into the change queue.
    PendingChange,
    /// Applies `Card` and undoes it one tick later.
    OneTickCard { player: usize },
    /// Sets the x velocity of roster player `player` to NaN.
    Nan { player: usize },
}

/// What undoes a one-tick fault.
pub(crate) struct Restore {
    team: usize,
}

impl Restore {
    pub(crate) fn apply(self, sim: &mut Simulation) {
        sim.summary.yellow[self.team] -= 1;
        sim.referee.tally.cards -= 1;
    }
}

impl Fault {
    /// Changes the state; returns what undoes it when the fault lasts one tick.
    pub(crate) fn apply(&self, sim: &mut Simulation) -> Option<Restore> {
        match self.kind {
            FaultKind::VelocityBit { player, bit } => {
                let vel = &mut sim.players[player].vel;
                vel.x = f64::from_bits(vel.x.to_bits() ^ (1u64 << bit));
            }
            FaultKind::ExtraDraw => {
                sim.rng.next_f64();
            }
            FaultKind::Stamina { player, delta } => sim.players[player].energy += delta,
            FaultKind::Card { player } => card(sim, player),
            FaultKind::Restart { player } => {
                sim.restart_taker = match sim.restart_taker {
                    None => Some(player),
                    Some(_) => None,
                };
            }
            FaultKind::PendingChange => {
                let id = ChangeId {
                    tick: sim.tick,
                    n: sim.queue.next,
                };
                sim.queue.next += 1;
                sim.queue.pending.push(QueuedChange {
                    id,
                    team: 0,
                    change: Change::Tactics(TacticsPatch::default()),
                });
            }
            FaultKind::OneTickCard { player } => {
                card(sim, player);
                return Some(Restore {
                    team: sim.players[player].team,
                });
            }
            FaultKind::Nan { player } => sim.players[player].vel.x = f64::NAN,
        }
        None
    }
}

fn card(sim: &mut Simulation, player: usize) {
    let team = sim.players[player].team;
    sim.summary.yellow[team] += 1;
    sim.referee.tally.cards += 1;
}

/// Plays `fixture` with `fault` and stops at the first checkpoint that differs from
/// `against`, the hashes of a clean run of the same fixture.
pub fn play_faulted(
    fixture: &Fixture,
    inputs: &Inputs<'_>,
    fault: Fault,
    against: &MatchHashes,
) -> Result<Played, super::GateError> {
    super::run(
        fixture,
        inputs,
        &Probe {
            against: Some(against),
            fault: Some(fault),
        },
    )
}
