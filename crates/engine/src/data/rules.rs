//! The rule pack: stoppage kinds and what each admits, substitution limits, and half lengths.
//! The match-rules slice enforces it; this slice loads and validates it.

use garde::Validate;
use serde::{Deserialize, Serialize};

/// Schema version this build reads.
pub const RULES_VERSION: u32 = 1;

/// Every kind of stoppage the rules name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoppageKind {
    KickOff,
    ThrowIn,
    Corner,
    GoalKick,
    FreeKick,
    Penalty,
    Goal,
    HalfTime,
    Injury,
}

impl StoppageKind {
    /// Every kind, in declaration order.
    pub const ALL: [StoppageKind; 9] = [
        StoppageKind::KickOff,
        StoppageKind::ThrowIn,
        StoppageKind::Corner,
        StoppageKind::GoalKick,
        StoppageKind::FreeKick,
        StoppageKind::Penalty,
        StoppageKind::Goal,
        StoppageKind::HalfTime,
        StoppageKind::Injury,
    ];

    /// The kind as written in a data file.
    pub fn code(&self) -> &'static str {
        match self {
            StoppageKind::KickOff => "kick_off",
            StoppageKind::ThrowIn => "throw_in",
            StoppageKind::Corner => "corner",
            StoppageKind::GoalKick => "goal_kick",
            StoppageKind::FreeKick => "free_kick",
            StoppageKind::Penalty => "penalty",
            StoppageKind::Goal => "goal",
            StoppageKind::HalfTime => "half_time",
            StoppageKind::Injury => "injury",
        }
    }
}

/// One stoppage kind and what the manager may do during it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[garde(allow_unvalidated)]
pub struct Stoppage {
    pub kind: StoppageKind,
    pub admits_tactics: bool,
    pub admits_substitution: bool,
}

/// Substitution limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Substitutions {
    /// Players a team may replace during a match.
    #[garde(range(min = 0, max = 11))]
    pub limit: u8,
    /// Stoppages during which a team may make substitutions.
    #[garde(range(min = 0, max = 5))]
    pub windows: u8,
}

/// The rule pack file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RulePack {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(range(min = 1, max = 2))]
    pub halves: u8,
    #[garde(range(min = 1, max = 60))]
    pub half_minutes: u8,
    #[garde(dive)]
    pub substitutions: Substitutions,
    #[garde(dive, custom(every_kind_once))]
    pub stoppages: Vec<Stoppage>,
}

fn every_kind_once(stoppages: &[Stoppage], _ctx: &()) -> garde::Result {
    for kind in StoppageKind::ALL {
        let count = stoppages.iter().filter(|s| s.kind == kind).count();
        if count != 1 {
            return Err(garde::Error::new(format!(
                "stoppage {} appears {count} times; expected once",
                kind.code()
            )));
        }
    }
    Ok(())
}
