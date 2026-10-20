//! Proposed changes: what a module asks the central loop to write.

use crate::rules::offside::OffsideSet;
use crate::sim::Simulation;

/// One change a module proposes. Only [`Simulation::apply_proposal`] writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proposal {
    /// The players in an offside position since the kick.
    SetOffside(OffsideSet),
    /// Side `team`'s game-state rules that hold changed to `active`: the rule evaluator's
    /// output, queued as a rule change.
    QueueRuleChange { team: usize, active: u8 },
}

impl Simulation {
    /// Applies one proposed change: the one place a module's output is written.
    #[inline]
    pub(crate) fn apply_proposal(&mut self, proposal: Proposal) {
        match proposal {
            Proposal::SetOffside(set) => self.referee.offside = set,
            Proposal::QueueRuleChange { team, active } => self.rule_change(team, active),
        }
    }
}
