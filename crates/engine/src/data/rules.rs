//! The rule pack: stoppage kinds and what each admits, substitution limits, half lengths,
//! the added-time allowance, the fewest players a team may field, and the extra time and
//! penalty shoot-out a knockout match plays when it is level. The engine reads all of it
//! during a match.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

/// Schema version this build reads. Version 2 adds `added_time` and `min_players`; version 3
/// adds `substitutions.windows_exempt`; version 4 adds `extra_time` and `shootout`.
pub const RULES_VERSION: u32 = 4;

/// Every kind of stoppage the rules name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

    /// The position of the kind in `ALL`.
    pub fn index(&self) -> usize {
        *self as usize
    }

    /// The kind at position `index` of `ALL`, or `None`.
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
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
    /// Stoppage kinds whose substitutions use no window (half-time in the shipped pack). A
    /// substitution there still counts toward `limit`. Each kind appears at most once.
    #[garde(custom(each_kind_once))]
    pub windows_exempt: Vec<StoppageKind>,
}

impl Substitutions {
    /// `true` when a substitution at a stoppage of `kind` uses no window.
    pub fn exempt(&self, kind: StoppageKind) -> bool {
        self.windows_exempt.contains(&kind)
    }
}

fn each_kind_once(kinds: &[StoppageKind], _ctx: &()) -> garde::Result {
    for (i, kind) in kinds.iter().enumerate() {
        if kinds[..i].contains(kind) {
            return Err(garde::Error::new(format!(
                "stoppage {} appears twice",
                kind.code()
            )));
        }
    }
    Ok(())
}

/// The time a referee adds at the end of each half: seconds per stoppage of each kind and
/// per card, plus a seeded variance of up to `variance_s` either way, clamped between
/// `min_s` and `max_s`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AddedTime {
    /// Seconds added for each stoppage of a kind. Every kind is named once.
    #[garde(custom(every_kind_priced))]
    pub per_kind: BTreeMap<StoppageKind, u32>,
    /// Seconds added for each card shown.
    #[garde(range(min = 0, max = 120))]
    pub card_s: u32,
    /// The largest variance either way, in seconds.
    #[garde(range(min = 0, max = 300))]
    pub variance_s: u32,
    /// The least time added to a half, in seconds.
    #[garde(range(min = 0, max = 900))]
    pub min_s: u32,
    /// The most time added to a half, in seconds. It sets the announced maximum match length.
    #[garde(range(min = 0, max = 1800), custom(at_least(self.min_s)))]
    pub max_s: u32,
}

impl AddedTime {
    /// Seconds for one stoppage of `kind`.
    pub fn seconds(&self, kind: StoppageKind) -> u32 {
        self.per_kind.get(&kind).copied().unwrap_or(0)
    }
}

fn every_kind_priced(per_kind: &BTreeMap<StoppageKind, u32>, _ctx: &()) -> garde::Result {
    for kind in StoppageKind::ALL {
        match per_kind.get(&kind) {
            None => {
                return Err(garde::Error::new(format!(
                    "stoppage {} has no seconds",
                    kind.code()
                )));
            }
            Some(&s) if s > 600 => {
                return Err(garde::Error::new(format!(
                    "stoppage {} adds {s} seconds; at most 600",
                    kind.code()
                )));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

fn at_least(min: u32) -> impl FnOnce(&u32, &()) -> garde::Result {
    move |value, _| {
        if *value < min {
            return Err(garde::Error::new(format!("lower than min_s ({min})")));
        }
        Ok(())
    }
}

/// Extra time in a knockout match that is level at the end of regulation time (IFAB Law 7):
/// periods of equal length with their own added time, and the extra substitution and window
/// each team gains (IFAB Law 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ExtraTime {
    /// Periods of extra time; 0 goes straight to the shoot-out.
    #[garde(range(min = 0, max = 2))]
    pub periods: u8,
    /// Minutes in each period.
    #[garde(range(min = 1, max = 30))]
    pub period_minutes: u8,
    /// The most time added to one extra-time period, in seconds.
    #[garde(range(min = 0, max = 900))]
    pub added_max_s: u32,
    /// Substitutions each team gains in extra time, on top of `substitutions.limit`.
    #[garde(range(min = 0, max = 3))]
    pub extra_substitutions: u8,
    /// Windows each team gains in extra time, on top of `substitutions.windows`.
    #[garde(range(min = 0, max = 3))]
    pub extra_windows: u8,
}

/// The penalty shoot-out that decides a knockout match still level after extra time (IFAB
/// Law 10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Shootout {
    /// Kicks each team takes before sudden death.
    #[garde(range(min = 1, max = 10))]
    pub kicks: u8,
    /// Rounds of kicks the announced maximum match length allows for. A shoot-out that runs
    /// longer still plays to its end.
    #[garde(range(min = 1, max = 30))]
    pub allowance_rounds: u8,
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
    #[garde(dive)]
    pub added_time: AddedTime,
    /// A team with fewer players on the pitch cannot continue, and the match is abandoned.
    #[garde(range(min = 1, max = 11))]
    pub min_players: u8,
    #[garde(dive)]
    pub extra_time: ExtraTime,
    #[garde(dive)]
    pub shootout: Shootout,
}

impl RulePack {
    /// Minutes of regulation play: halves times half length.
    pub fn regulation_minutes(&self) -> u32 {
        u32::from(self.halves) * u32::from(self.half_minutes)
    }

    /// What a stoppage of `kind` admits: `(tactics, substitution)`. A kind the pack does not
    /// list admits nothing; a validated pack lists every kind.
    pub fn admits(&self, kind: StoppageKind) -> (bool, bool) {
        self.stoppages
            .iter()
            .find(|s| s.kind == kind)
            .map_or((false, false), |s| {
                (s.admits_tactics, s.admits_substitution)
            })
    }
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
