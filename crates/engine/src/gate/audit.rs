//! The carrier-options audit (engine tests only, under the `scenario` feature). With the
//! audit on, every carrier decision scores its options twice: once with a verbatim copy of
//! the options code before the pass-lane early exit, and once with the live code, from the
//! same stream position. The audit compares how each team-mate was treated, every score by
//! its bits, the chosen pass, and the stream position after the call. The match goes on from
//! the live call, and the audit is not in the hashed state, so an audited match has the same
//! hashes as a plain one.

use std::cell::RefCell;

use crate::player::Player;

use super::{Fixture, GateError, Inputs, Played, Probe};

/// How the pass loop treated one team-mate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Candidate {
    /// Rejected: nearer than 4 m or farther than 45 m.
    Distance(usize),
    /// Rejected: an opponent is nearer the pass lane than the minimum lane.
    Lane(usize),
    /// Scored; the score's bits.
    Scored { mate: usize, score: u64 },
}

/// A change the reference copy makes on purpose, so a test can prove that the audit sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// The reference takes one extra draw after scoring.
    ExtraDraw,
    /// The reference adds one ulp to the first team-mate score of each call.
    OneUlp,
    /// The reference rejects the first team-mate that passes the lane check.
    RejectMate,
}

/// One call whose two results differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    pub tick: u32,
    pub carrier: usize,
    pub what: &'static str,
}

/// The audit's counts.
#[derive(Debug, Clone, Default)]
pub struct OptionsAudit {
    /// The control the reference applies, if any.
    pub control: Option<Control>,
    /// Audited carrier decisions.
    pub calls: u64,
    /// Opponent loops the live code left early.
    pub breaks: u64,
    /// Active opponents the early exits skipped.
    pub skipped_opponents: u64,
    /// Calls with at least one rejected and one scored team-mate.
    pub mixed_calls: u64,
    /// Calls whose two results differ.
    pub mismatches: u64,
    /// The first eight of them.
    pub first_mismatches: Vec<Mismatch>,
    /// The live call's team-mate records, filled while it runs.
    pub(crate) candidates: Vec<Candidate>,
}

impl OptionsAudit {
    pub(crate) fn mismatch(&mut self, tick: u32, carrier: usize, what: &'static str) {
        self.mismatches += 1;
        if self.first_mismatches.len() < 8 {
            self.first_mismatches.push(Mismatch {
                tick,
                carrier,
                what,
            });
        }
    }
}

/// Records how the live pass loop treated one team-mate, when the audit is on.
pub(crate) fn record(audit: &mut Option<OptionsAudit>, candidate: Candidate) {
    if let Some(a) = audit {
        a.candidates.push(candidate);
    }
}

/// Records one early exit after `visited` of the opponents of `team`, when the audit is on.
pub(crate) fn record_break(
    audit: &mut Option<OptionsAudit>,
    players: &[Player],
    team: usize,
    visited: u32,
) {
    if let Some(a) = audit {
        let opponents = players
            .iter()
            .filter(|p| p.team != team && p.active())
            .count() as u64;
        a.breaks += 1;
        a.skipped_opponents += opponents - u64::from(visited);
    }
}

/// Plays `fixture` with the options audit on and returns the result and the audit.
pub fn play_audited(
    fixture: &Fixture,
    inputs: &Inputs<'_>,
    control: Option<Control>,
) -> Result<(Played, OptionsAudit), GateError> {
    let audit = RefCell::new(OptionsAudit {
        control,
        ..OptionsAudit::default()
    });
    let played = super::run(
        fixture,
        inputs,
        &Probe {
            audit: Some(&audit),
            ..Probe::default()
        },
    )?;
    Ok((played, audit.into_inner()))
}
