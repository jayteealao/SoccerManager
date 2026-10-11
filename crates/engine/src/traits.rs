//! Traits of play in a match. A player carries each trait of the catalogue
//! (`content/traits.json`) at a strength, moderate or strong, and every trait he carries acts
//! on its engine choice through a weight: a strong trait at its full weight, a moderate one at
//! `traits.moderate_share` of it (the weights table of `engine.contract.traits`).
//!
//! A strong trait can also override an instruction it goes against. At a moment of play (a
//! release of the ball, the start of a carry, a new opposing carrier, his side gaining the
//! ball, a loose ball near a keeper), when the player carries the trait strong, his team holds
//! the conflicting instruction for him, he is under pressure (an active opponent within
//! [`crate::contract::PRESSED_M`]) or his mood is at or below `traits.low_mood`, and the
//! trait's choice differs from the one play took, one draw on his `trait_override` key below
//! the trait's chance makes the trait's choice win. Nothing else enters: no count of earlier
//! overrides, no time since the last one, no phase of play, no score or minute test. An
//! override that holds an action (a carry, a press, an anchor, a chase) lasts as long as that
//! action does, never as a wait.
//!
//! With no traits block in the tuning every weight and chance is 0, so play is as before.

use crate::contract::params::TraitTuning;
use crate::data::traits::TraitStrength;
use crate::tactics::Instruction;

/// The metres toward the opponents' goal a killer ball gains at least.
pub const KILLER_GAIN_M: f64 = 15.0;
/// A switch is a pass longer than this, across at least this much of the pitch's width.
pub const SWITCH_M: f64 = 25.0;
/// An early cross starts farther than this from the goal line it attacks.
pub const EARLY_CROSS_M: f64 = 18.0;
/// The distance from the centre line, in metres on a 68 m wide pitch, of the channel between
/// a full-back and a centre-back that a player who moves into channels drifts toward.
pub const CHANNEL_Y_M: f64 = 15.0;
/// The depth of a late run's target short of the goal line it attacks, and the farthest it
/// lies from the centre line, in metres.
pub const LATE_RUN_DEPTH_M: f64 = 11.0;
pub const LATE_RUN_WIDTH_M: f64 = 9.0;

/// The traits the engine knows: the shipped catalogue's.
pub const TRAIT_COUNT: usize = 31;

/// One trait of play, as the engine reads it. A catalogue name the engine does not know is
/// carried in the team file but plays no part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TraitKind {
    CutsInside,
    HugsTouchline,
    ShootsFromDistance,
    PlacesShots,
    PowersShots,
    TriesLobsAndChips,
    TriesKillerBalls,
    PlaysShortSimplePasses,
    SwitchesPlay,
    CrossesEarly,
    RunsWithBallOften,
    RarelyDribbles,
    TriesTricks,
    PlaysOneTwos,
    HoldsUpBall,
    ReleasesBallEarly,
    DwellsOnBall,
    GetsForward,
    StaysBack,
    ArrivesLateInBox,
    MovesIntoChannels,
    PressesRelentlessly,
    DivesIntoTackles,
    StaysOnFeet,
    GoesDownEasily,
    ArguesWithOfficials,
    RushesOut,
    StaysOnLine,
    PunchesRatherThanCatches,
    KicksLong,
    ThrowsShort,
}

impl TraitKind {
    /// Every trait, in catalogue order.
    pub const ALL: [TraitKind; TRAIT_COUNT] = [
        TraitKind::CutsInside,
        TraitKind::HugsTouchline,
        TraitKind::ShootsFromDistance,
        TraitKind::PlacesShots,
        TraitKind::PowersShots,
        TraitKind::TriesLobsAndChips,
        TraitKind::TriesKillerBalls,
        TraitKind::PlaysShortSimplePasses,
        TraitKind::SwitchesPlay,
        TraitKind::CrossesEarly,
        TraitKind::RunsWithBallOften,
        TraitKind::RarelyDribbles,
        TraitKind::TriesTricks,
        TraitKind::PlaysOneTwos,
        TraitKind::HoldsUpBall,
        TraitKind::ReleasesBallEarly,
        TraitKind::DwellsOnBall,
        TraitKind::GetsForward,
        TraitKind::StaysBack,
        TraitKind::ArrivesLateInBox,
        TraitKind::MovesIntoChannels,
        TraitKind::PressesRelentlessly,
        TraitKind::DivesIntoTackles,
        TraitKind::StaysOnFeet,
        TraitKind::GoesDownEasily,
        TraitKind::ArguesWithOfficials,
        TraitKind::RushesOut,
        TraitKind::StaysOnLine,
        TraitKind::PunchesRatherThanCatches,
        TraitKind::KicksLong,
        TraitKind::ThrowsShort,
    ];

    /// The trait's catalogue name.
    pub fn name(self) -> &'static str {
        match self {
            TraitKind::CutsInside => "cuts_inside",
            TraitKind::HugsTouchline => "hugs_touchline",
            TraitKind::ShootsFromDistance => "shoots_from_distance",
            TraitKind::PlacesShots => "places_shots",
            TraitKind::PowersShots => "powers_shots",
            TraitKind::TriesLobsAndChips => "tries_lobs_and_chips",
            TraitKind::TriesKillerBalls => "tries_killer_balls",
            TraitKind::PlaysShortSimplePasses => "plays_short_simple_passes",
            TraitKind::SwitchesPlay => "switches_play",
            TraitKind::CrossesEarly => "crosses_early",
            TraitKind::RunsWithBallOften => "runs_with_ball_often",
            TraitKind::RarelyDribbles => "rarely_dribbles",
            TraitKind::TriesTricks => "tries_tricks",
            TraitKind::PlaysOneTwos => "plays_one_twos",
            TraitKind::HoldsUpBall => "holds_up_ball",
            TraitKind::ReleasesBallEarly => "releases_ball_early",
            TraitKind::DwellsOnBall => "dwells_on_ball",
            TraitKind::GetsForward => "gets_forward",
            TraitKind::StaysBack => "stays_back",
            TraitKind::ArrivesLateInBox => "arrives_late_in_box",
            TraitKind::MovesIntoChannels => "moves_into_channels",
            TraitKind::PressesRelentlessly => "presses_relentlessly",
            TraitKind::DivesIntoTackles => "dives_into_tackles",
            TraitKind::StaysOnFeet => "stays_on_feet",
            TraitKind::GoesDownEasily => "goes_down_easily",
            TraitKind::ArguesWithOfficials => "argues_with_officials",
            TraitKind::RushesOut => "rushes_out",
            TraitKind::StaysOnLine => "stays_on_line",
            TraitKind::PunchesRatherThanCatches => "punches_rather_than_catches",
            TraitKind::KicksLong => "kicks_long",
            TraitKind::ThrowsShort => "throws_short",
        }
    }

    /// The trait named `name`, when the engine knows it.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|k| k.name() == name)
    }

    /// The trait's place in [`TraitKind::ALL`], as the snapshot and the event store it.
    pub fn index(self) -> u8 {
        self as u8
    }

    /// The trait at place `i` of [`TraitKind::ALL`].
    pub fn from_index(i: u8) -> Option<Self> {
        Self::ALL.get(usize::from(i)).copied()
    }

    /// The instructions the trait goes against (the conflict table). Ten traits have a row.
    ///
    /// The other 21 act through their weight only: nine because no instruction asks the
    /// opposite of them, and twelve because the moments their override needs are too rare
    /// for the rate the overrides aim at. Played strong at a chance of 1 against the
    /// instruction they go against, over 20 seeded matches, each of the twelve met fewer than
    /// 2 moments a match: killer balls, lobs and chips, early crosses, the keeper's long kick,
    /// short simple passes, one-twos, the keeper's short throw, running with the ball, tricks,
    /// switches of play, shots from distance and staying on the line. No chance reaches 2
    /// overrides a match for them, and a throttle that forced more is ruled out, so they
    /// leave the table. Their moments stay in the decision code: a row here brings one back.
    pub fn conflicts(self) -> &'static [Instruction] {
        use Instruction as I;
        match self {
            TraitKind::DwellsOnBall | TraitKind::HoldsUpBall => &[I::TempoFaster],
            TraitKind::ReleasesBallEarly => &[I::TempoSlower],
            TraitKind::CutsInside => &[I::WidthWide],
            TraitKind::HugsTouchline => &[I::WidthNarrow],
            TraitKind::PressesRelentlessly => &[I::PressingLow],
            TraitKind::GetsForward | TraitKind::ArrivesLateInBox => &[I::DutyDefend],
            TraitKind::StaysBack => &[I::DutyAttack],
            TraitKind::RushesOut => &[I::LineDeep],
            TraitKind::TriesKillerBalls
            | TraitKind::TriesLobsAndChips
            | TraitKind::CrossesEarly
            | TraitKind::KicksLong
            | TraitKind::PlaysShortSimplePasses
            | TraitKind::PlaysOneTwos
            | TraitKind::ThrowsShort
            | TraitKind::RunsWithBallOften
            | TraitKind::TriesTricks
            | TraitKind::SwitchesPlay
            | TraitKind::ShootsFromDistance
            | TraitKind::StaysOnLine
            | TraitKind::RarelyDribbles
            | TraitKind::MovesIntoChannels
            | TraitKind::PlacesShots
            | TraitKind::PowersShots
            | TraitKind::DivesIntoTackles
            | TraitKind::StaysOnFeet
            | TraitKind::GoesDownEasily
            | TraitKind::ArguesWithOfficials
            | TraitKind::PunchesRatherThanCatches => &[],
        }
    }

    /// The first instruction of the conflict table that `held` (the bits of
    /// [`Instruction::bit`] a team holds for the player) holds.
    pub fn conflict_in(self, held: u16) -> Option<Instruction> {
        self.conflicts()
            .iter()
            .copied()
            .find(|i| held & i.bit() != 0)
    }
}

/// The traits one player carries, each at its strength; built once when the squad loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TraitSet {
    strengths: [Option<TraitStrength>; TRAIT_COUNT],
    any: bool,
}

impl TraitSet {
    /// No trait.
    pub const EMPTY: TraitSet = TraitSet {
        strengths: [None; TRAIT_COUNT],
        any: false,
    };

    /// The set of a squad entry's named traits; a name the engine does not know is ignored.
    pub fn of(named: &[(String, TraitStrength)]) -> Self {
        let mut set = Self::EMPTY;
        for (name, strength) in named {
            if let Some(k) = TraitKind::from_name(name) {
                set.set(k, Some(*strength));
            }
        }
        set
    }

    /// The strength he carries trait `k` at.
    #[inline]
    pub fn get(&self, k: TraitKind) -> Option<TraitStrength> {
        self.strengths[k as usize]
    }

    /// Sets trait `k` to `strength`, or removes it.
    pub fn set(&mut self, k: TraitKind, strength: Option<TraitStrength>) {
        self.strengths[k as usize] = strength;
        self.any = self.strengths.iter().any(Option::is_some);
    }

    /// `true` when he carries no trait the engine knows.
    #[inline]
    pub fn is_empty(&self) -> bool {
        !self.any
    }

    /// `true` when he carries trait `k` strong.
    #[inline]
    pub fn is_strong(&self, k: TraitKind) -> bool {
        self.get(k) == Some(TraitStrength::Strong)
    }

    /// The share of trait `k`'s weight he plays: 1 strong, `moderate_share` moderate, 0
    /// without it.
    #[inline]
    pub fn share(&self, k: TraitKind, t: &TraitTuning) -> f64 {
        match self.get(k) {
            Some(TraitStrength::Strong) => 1.0,
            Some(TraitStrength::Moderate) => t.moderate_share,
            None => 0.0,
        }
    }

    /// The weight trait `k` plays at for him.
    #[inline]
    pub fn weight(&self, k: TraitKind, t: &TraitTuning) -> f64 {
        match self.get(k) {
            None => 0.0,
            Some(_) => self.share(k, t) * t.weights.of(k),
        }
    }
}

/// Why a player overrode an instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverrideCause {
    /// An active opponent within the pressing distance.
    Pressure,
    /// His mood at or below `traits.low_mood`.
    LowMood,
}

impl OverrideCause {
    /// Every cause, in declaration order.
    pub const ALL: [OverrideCause; 2] = [OverrideCause::Pressure, OverrideCause::LowMood];

    /// The cause as the event contract spells it.
    pub fn code(self) -> &'static str {
        match self {
            OverrideCause::Pressure => "pressure",
            OverrideCause::LowMood => "low-mood",
        }
    }
}

/// Condition (3) of an override: `pressure` when he is pressed, else `low-mood` when his mood
/// (percent of a full mood) is at or below `low_mood`, else none.
#[inline]
pub fn cause(pressed: bool, mood: f64, t: &TraitTuning) -> Option<OverrideCause> {
    if pressed {
        Some(OverrideCause::Pressure)
    } else if mood <= t.low_mood {
        Some(OverrideCause::LowMood)
    } else {
        None
    }
}

/// When an override that holds an action ends: the action's own end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActEnd {
    /// A carry or a hold: at this tick, or as soon as he no longer has the ball.
    Tick(u32),
    /// A press: when the opposing carrier (this roster index) no longer has the ball.
    Carrier(usize),
    /// An anchor: when this side no longer has the ball.
    Possession(usize),
    /// A keeper's chase or his holding his line: when someone wins the loose ball.
    LooseBall,
}

/// An override in force: the trait whose action the player plays, until `ends`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraitAct {
    pub kind: TraitKind,
    pub ends: ActEnd,
}

impl TraitAct {
    /// The act as four bytes and a word for the snapshot: the trait, the end's kind, and its
    /// value.
    pub fn encode(act: Option<TraitAct>) -> (u8, u8, u32) {
        match act {
            None => (0xFF, 0, 0),
            Some(a) => {
                let (end, value) = match a.ends {
                    ActEnd::Tick(t) => (0, t),
                    // A roster index and a side index fit a word.
                    ActEnd::Carrier(c) => (1, c as u32),
                    ActEnd::Possession(team) => (2, team as u32),
                    ActEnd::LooseBall => (3, 0),
                };
                (a.kind.index(), end, value)
            }
        }
    }

    /// The act [`TraitAct::encode`] wrote, or `None` for a damaged value.
    pub fn decode(kind: u8, end: u8, value: u32) -> Result<Option<TraitAct>, String> {
        if kind == 0xFF {
            return Ok(None);
        }
        let kind = TraitKind::from_index(kind).ok_or_else(|| format!("trait {kind} is unknown"))?;
        let ends = match end {
            0 => ActEnd::Tick(value),
            1 => ActEnd::Carrier(value as usize),
            2 => ActEnd::Possession(value as usize),
            3 => ActEnd::LooseBall,
            _ => return Err(format!("trait act end {end} is unknown")),
        };
        Ok(Some(TraitAct { kind, ends }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_trait_has_its_catalogue_name_and_its_place() {
        let content = crate::data::test_support::shipped_content();
        assert_eq!(content.traits.traits.len(), TRAIT_COUNT);
        for (k, def) in TraitKind::ALL.iter().zip(&content.traits.traits) {
            assert_eq!(k.name(), def.name);
            assert_eq!(TraitKind::from_index(k.index()), Some(*k));
            assert_eq!(TraitKind::from_name(&def.name), Some(*k));
        }
        assert_eq!(TraitKind::from_name("plays_it_safe"), None);
    }

    #[test]
    fn ten_traits_conflict_with_an_instruction_and_twenty_one_act_by_weight_only() {
        let conflicting = TraitKind::ALL
            .iter()
            .filter(|k| !k.conflicts().is_empty())
            .count();
        assert_eq!(conflicting, 10);
    }

    #[test]
    fn a_set_reads_strengths_and_ignores_an_unknown_name() {
        let set = TraitSet::of(&[
            ("cuts_inside".into(), TraitStrength::Strong),
            ("plays_it_safe".into(), TraitStrength::Strong),
            ("holds_up_ball".into(), TraitStrength::Moderate),
        ]);
        assert!(set.is_strong(TraitKind::CutsInside));
        assert_eq!(
            set.get(TraitKind::HoldsUpBall),
            Some(TraitStrength::Moderate)
        );
        assert!(!set.is_empty());
        assert!(TraitSet::of(&[("plays_it_safe".into(), TraitStrength::Strong)]).is_empty());
    }

    #[test]
    fn the_cause_is_pressure_first_then_low_mood() {
        let t = TraitTuning::shipped();
        assert_eq!(cause(true, -80.0, &t), Some(OverrideCause::Pressure));
        assert_eq!(cause(false, t.low_mood, &t), Some(OverrideCause::LowMood));
        assert_eq!(cause(false, 0.0, &t), None);
    }

    #[test]
    fn an_act_round_trips_through_its_bytes() {
        for act in [
            None,
            Some(TraitAct {
                kind: TraitKind::ThrowsShort,
                ends: ActEnd::Tick(9_000),
            }),
            Some(TraitAct {
                kind: TraitKind::PressesRelentlessly,
                ends: ActEnd::Carrier(17),
            }),
            Some(TraitAct {
                kind: TraitKind::StaysBack,
                ends: ActEnd::Possession(1),
            }),
            Some(TraitAct {
                kind: TraitKind::RushesOut,
                ends: ActEnd::LooseBall,
            }),
        ] {
            let (k, e, v) = TraitAct::encode(act);
            assert_eq!(TraitAct::decode(k, e, v), Ok(act));
        }
        assert!(TraitAct::decode(40, 0, 0).is_err());
        assert!(TraitAct::decode(0, 9, 0).is_err());
    }
}
