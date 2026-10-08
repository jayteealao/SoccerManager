//! The attribute contract: the one place play reads a player's attributes.
//!
//! Every action the engine has runs in stages (see, choose, execute, under pressure). Each
//! stage blends one main attribute and up to three supporting ones, with weights from the
//! attribute file ([`stages`]). Each rating passes through the exponential curve
//! `F(r) = 8 · e^((r − 10) / 8)` before the blend ([`curve`]), and the stage value is the
//! weighted mean of those `F` values, which is `F` of the stage's blended rating, so the curve
//! acts on the stage result as well.
//!
//! A chance between two players is a contest in log-odds anchored on the chance two average
//! players have today: `σ(logit(base) + k · (F_a − F_b))`, kept inside a floor and a ceiling.
//! A value with no opponent reads a skill share `σ(k · (F − F(10)))`, which is 0.5 at rating
//! 10. So a rating-10 player plays as every player played before the contract; the spread
//! between players is what the curve and `k` change.
//!
//! The stage values are computed once per player at load ([`stage_values`]). The states move
//! a player's ratings within caps ([`states`]); when his deltas change, the 50-tick refresh
//! blends his stage values again from his effective ratings. No blend runs inside a tick.
//! Consistency spreads each player's play within and between matches through the same
//! rebuild ([`consistency`]); the hidden values reach a page only as words ([`hidden`]).

pub mod body;
pub mod consistency;
pub mod curve;
pub mod hidden;
pub mod level;
pub mod params;
pub mod stages;
pub mod states;

pub use curve::{contest, factor, logistic, logit, share};
pub use params::{ActionMap, ActionParams, ContractTuning, CurveTuning, LapseTuning, SpeedTuning};
pub use stages::{GateDef, GateDefs, StageTable, Weighted, stage_values};

use serde::{Deserialize, Serialize};

use crate::rating::Rating;

/// The kinds of action, each with its own stage table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Pass,
    Chip,
    Cross,
    Shot,
    LongShot,
    Header,
    Penalty,
    Dribble,
    Receive,
    Tackle,
    Intercept,
    Press,
    Shape,
    Block,
    Sprint,
    Endure,
    Shield,
    Turn,
    StayUp,
    AerialReach,
    Recover,
    Injury,
    Hold,
    Save,
    Claim,
    OneOnOne,
    KeeperKick,
    KeeperThrow,
    Organise,
    Rush,
}

/// The number of action kinds.
pub const ACTION_COUNT: usize = 30;

impl ActionKind {
    /// Every action, in declaration order.
    pub const ALL: [ActionKind; ACTION_COUNT] = [
        ActionKind::Pass,
        ActionKind::Chip,
        ActionKind::Cross,
        ActionKind::Shot,
        ActionKind::LongShot,
        ActionKind::Header,
        ActionKind::Penalty,
        ActionKind::Dribble,
        ActionKind::Receive,
        ActionKind::Tackle,
        ActionKind::Intercept,
        ActionKind::Press,
        ActionKind::Shape,
        ActionKind::Block,
        ActionKind::Sprint,
        ActionKind::Endure,
        ActionKind::Shield,
        ActionKind::Turn,
        ActionKind::StayUp,
        ActionKind::AerialReach,
        ActionKind::Recover,
        ActionKind::Injury,
        ActionKind::Hold,
        ActionKind::Save,
        ActionKind::Claim,
        ActionKind::OneOnOne,
        ActionKind::KeeperKick,
        ActionKind::KeeperThrow,
        ActionKind::Organise,
        ActionKind::Rush,
    ];

    /// The action's name in the content files.
    pub fn name(self) -> &'static str {
        match self {
            ActionKind::Pass => "pass",
            ActionKind::Chip => "chip",
            ActionKind::Cross => "cross",
            ActionKind::Shot => "shot",
            ActionKind::LongShot => "long_shot",
            ActionKind::Header => "header",
            ActionKind::Penalty => "penalty",
            ActionKind::Dribble => "dribble",
            ActionKind::Receive => "receive",
            ActionKind::Tackle => "tackle",
            ActionKind::Intercept => "intercept",
            ActionKind::Press => "press",
            ActionKind::Shape => "shape",
            ActionKind::Block => "block",
            ActionKind::Sprint => "sprint",
            ActionKind::Endure => "endure",
            ActionKind::Shield => "shield",
            ActionKind::Turn => "turn",
            ActionKind::StayUp => "stay_up",
            ActionKind::AerialReach => "aerial_reach",
            ActionKind::Recover => "recover",
            ActionKind::Injury => "injury",
            ActionKind::Hold => "hold",
            ActionKind::Save => "save",
            ActionKind::Claim => "claim",
            ActionKind::OneOnOne => "one_on_one",
            ActionKind::KeeperKick => "keeper_kick",
            ActionKind::KeeperThrow => "keeper_throw",
            ActionKind::Organise => "organise",
            ActionKind::Rush => "rush",
        }
    }

    /// The action named `name`, or `None`.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }

    /// The stages play reads for this action, in stage order.
    pub fn stages(self) -> impl Iterator<Item = StageKind> {
        STAGES
            .iter()
            .filter(move |(a, _)| *a == self)
            .map(|&(_, s)| s)
    }
}

/// The stages of an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKind {
    /// What the player sees: the options he notices and how early.
    See,
    /// Which option he picks, and when.
    Choose,
    /// How well he carries it out.
    Execute,
    /// How well he carries it out with an opponent close.
    Pressure,
}

impl StageKind {
    /// Every stage, in order.
    pub const ALL: [StageKind; 4] = [
        StageKind::See,
        StageKind::Choose,
        StageKind::Execute,
        StageKind::Pressure,
    ];

    /// The stage's name in the content files.
    pub fn name(self) -> &'static str {
        match self {
            StageKind::See => "see",
            StageKind::Choose => "choose",
            StageKind::Execute => "execute",
            StageKind::Pressure => "pressure",
        }
    }
}

impl std::fmt::Display for ActionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::fmt::Display for StageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Map keys in a validation path: a refusal names `actions.tackle.choose...`.
impl garde::error::PathComponentKind for ActionKind {
    fn component_kind() -> garde::error::Kind {
        garde::error::Kind::Key
    }
}

impl garde::error::PathComponentKind for StageKind {
    fn component_kind() -> garde::error::Kind {
        garde::error::Kind::Key
    }
}

/// Every stage play reads, as `(action, stage)`, in [`Stage`] index order. An attribute file
/// must give a table for each of these and for no other.
pub const STAGES: [(ActionKind, StageKind); STAGE_COUNT] = {
    use ActionKind as A;
    use StageKind as S;
    [
        (A::Pass, S::See),
        (A::Pass, S::Choose),
        (A::Pass, S::Execute),
        (A::Pass, S::Pressure),
        (A::Chip, S::Choose),
        (A::Chip, S::Execute),
        (A::Chip, S::Pressure),
        (A::Cross, S::Choose),
        (A::Cross, S::Execute),
        (A::Cross, S::Pressure),
        (A::Shot, S::Choose),
        (A::Shot, S::Execute),
        (A::Shot, S::Pressure),
        (A::LongShot, S::Choose),
        (A::LongShot, S::Execute),
        (A::LongShot, S::Pressure),
        (A::Header, S::Execute),
        (A::Penalty, S::Execute),
        (A::Penalty, S::Pressure),
        (A::Dribble, S::Choose),
        (A::Dribble, S::Execute),
        (A::Dribble, S::Pressure),
        (A::Receive, S::Execute),
        (A::Tackle, S::Choose),
        (A::Tackle, S::Execute),
        (A::Intercept, S::See),
        (A::Press, S::Choose),
        (A::Shape, S::Choose),
        (A::Shape, S::Execute),
        (A::Block, S::Execute),
        (A::Sprint, S::Execute),
        (A::Endure, S::Execute),
        (A::Shield, S::Execute),
        (A::Turn, S::Execute),
        (A::StayUp, S::Execute),
        (A::AerialReach, S::Execute),
        (A::Recover, S::Execute),
        (A::Injury, S::Execute),
        (A::Hold, S::Execute),
        (A::Save, S::Execute),
        (A::Claim, S::Choose),
        (A::Claim, S::Execute),
        (A::OneOnOne, S::Execute),
        (A::KeeperKick, S::Execute),
        (A::KeeperThrow, S::Execute),
        (A::Organise, S::Execute),
        (A::Rush, S::Choose),
    ]
};

/// The number of stages play reads.
pub const STAGE_COUNT: usize = 47;

/// One stage play reads: an index into [`STAGES`] and into [`StageValues`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stage(u8);

impl Stage {
    /// The stage `(action, stage)`. Evaluated at compile time for the constants below, so a
    /// constant naming a stage play does not read fails to build.
    pub const fn of(action: ActionKind, stage: StageKind) -> Stage {
        let mut i = 0;
        while i < STAGE_COUNT {
            if STAGES[i].0 as u8 == action as u8 && STAGES[i].1 as u8 == stage as u8 {
                return Stage(i as u8);
            }
            i += 1;
        }
        panic!("the stage is not in STAGES")
    }

    /// The stage `(action, stage)`, or `None` when play does not read it.
    pub fn find(action: ActionKind, stage: StageKind) -> Option<Stage> {
        STAGES
            .iter()
            .position(|&(a, s)| a == action && s == stage)
            .map(|i| Stage(i as u8))
    }

    /// The stage's index in [`STAGES`].
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// The action and stage kind.
    pub fn kind(self) -> (ActionKind, StageKind) {
        STAGES[self.index()]
    }

    /// `action.stage`, as the content files and messages name it.
    pub fn name(self) -> String {
        let (a, s) = self.kind();
        format!("{}.{}", a.name(), s.name())
    }

    pub const PASS_SEE: Stage = Stage::of(ActionKind::Pass, StageKind::See);
    pub const PASS_CHOOSE: Stage = Stage::of(ActionKind::Pass, StageKind::Choose);
    pub const PASS_EXECUTE: Stage = Stage::of(ActionKind::Pass, StageKind::Execute);
    pub const PASS_PRESSURE: Stage = Stage::of(ActionKind::Pass, StageKind::Pressure);
    pub const CHIP_CHOOSE: Stage = Stage::of(ActionKind::Chip, StageKind::Choose);
    pub const CHIP_EXECUTE: Stage = Stage::of(ActionKind::Chip, StageKind::Execute);
    pub const CHIP_PRESSURE: Stage = Stage::of(ActionKind::Chip, StageKind::Pressure);
    pub const CROSS_CHOOSE: Stage = Stage::of(ActionKind::Cross, StageKind::Choose);
    pub const CROSS_EXECUTE: Stage = Stage::of(ActionKind::Cross, StageKind::Execute);
    pub const CROSS_PRESSURE: Stage = Stage::of(ActionKind::Cross, StageKind::Pressure);
    pub const SHOT_CHOOSE: Stage = Stage::of(ActionKind::Shot, StageKind::Choose);
    pub const SHOT_EXECUTE: Stage = Stage::of(ActionKind::Shot, StageKind::Execute);
    pub const SHOT_PRESSURE: Stage = Stage::of(ActionKind::Shot, StageKind::Pressure);
    pub const LONG_SHOT_CHOOSE: Stage = Stage::of(ActionKind::LongShot, StageKind::Choose);
    pub const LONG_SHOT_EXECUTE: Stage = Stage::of(ActionKind::LongShot, StageKind::Execute);
    pub const LONG_SHOT_PRESSURE: Stage = Stage::of(ActionKind::LongShot, StageKind::Pressure);
    pub const HEADER_EXECUTE: Stage = Stage::of(ActionKind::Header, StageKind::Execute);
    pub const PENALTY_EXECUTE: Stage = Stage::of(ActionKind::Penalty, StageKind::Execute);
    pub const PENALTY_PRESSURE: Stage = Stage::of(ActionKind::Penalty, StageKind::Pressure);
    pub const DRIBBLE_CHOOSE: Stage = Stage::of(ActionKind::Dribble, StageKind::Choose);
    pub const DRIBBLE_EXECUTE: Stage = Stage::of(ActionKind::Dribble, StageKind::Execute);
    pub const DRIBBLE_PRESSURE: Stage = Stage::of(ActionKind::Dribble, StageKind::Pressure);
    pub const RECEIVE_EXECUTE: Stage = Stage::of(ActionKind::Receive, StageKind::Execute);
    pub const TACKLE_CHOOSE: Stage = Stage::of(ActionKind::Tackle, StageKind::Choose);
    pub const TACKLE_EXECUTE: Stage = Stage::of(ActionKind::Tackle, StageKind::Execute);
    pub const INTERCEPT_SEE: Stage = Stage::of(ActionKind::Intercept, StageKind::See);
    pub const PRESS_CHOOSE: Stage = Stage::of(ActionKind::Press, StageKind::Choose);
    pub const SHAPE_CHOOSE: Stage = Stage::of(ActionKind::Shape, StageKind::Choose);
    pub const SHAPE_EXECUTE: Stage = Stage::of(ActionKind::Shape, StageKind::Execute);
    pub const BLOCK_EXECUTE: Stage = Stage::of(ActionKind::Block, StageKind::Execute);
    pub const SPRINT_EXECUTE: Stage = Stage::of(ActionKind::Sprint, StageKind::Execute);
    pub const ENDURE_EXECUTE: Stage = Stage::of(ActionKind::Endure, StageKind::Execute);
    pub const SHIELD_EXECUTE: Stage = Stage::of(ActionKind::Shield, StageKind::Execute);
    pub const TURN_EXECUTE: Stage = Stage::of(ActionKind::Turn, StageKind::Execute);
    pub const STAY_UP_EXECUTE: Stage = Stage::of(ActionKind::StayUp, StageKind::Execute);
    pub const AERIAL_REACH_EXECUTE: Stage = Stage::of(ActionKind::AerialReach, StageKind::Execute);
    pub const RECOVER_EXECUTE: Stage = Stage::of(ActionKind::Recover, StageKind::Execute);
    pub const INJURY_EXECUTE: Stage = Stage::of(ActionKind::Injury, StageKind::Execute);
    pub const HOLD_EXECUTE: Stage = Stage::of(ActionKind::Hold, StageKind::Execute);
    pub const SAVE_EXECUTE: Stage = Stage::of(ActionKind::Save, StageKind::Execute);
    pub const CLAIM_CHOOSE: Stage = Stage::of(ActionKind::Claim, StageKind::Choose);
    pub const CLAIM_EXECUTE: Stage = Stage::of(ActionKind::Claim, StageKind::Execute);
    pub const ONE_ON_ONE_EXECUTE: Stage = Stage::of(ActionKind::OneOnOne, StageKind::Execute);
    pub const KEEPER_KICK_EXECUTE: Stage = Stage::of(ActionKind::KeeperKick, StageKind::Execute);
    pub const KEEPER_THROW_EXECUTE: Stage = Stage::of(ActionKind::KeeperThrow, StageKind::Execute);
    pub const ORGANISE_EXECUTE: Stage = Stage::of(ActionKind::Organise, StageKind::Execute);
    pub const RUSH_CHOOSE: Stage = Stage::of(ActionKind::Rush, StageKind::Choose);
}

/// One player's stage values: each stage's value on the curve (`F` of the blended rating)
/// and its skill share (0.5 at rating 10). His base values live in the squad entry, computed
/// once at load; his effective values are blended the same way from his effective ratings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageValues {
    pub f: [f64; STAGE_COUNT],
    pub share: [f64; STAGE_COUNT],
}

impl StageValues {
    /// Every stage at `f` with share `share`.
    pub const fn flat(f: f64, share: f64) -> Self {
        Self {
            f: [f; STAGE_COUNT],
            share: [share; STAGE_COUNT],
        }
    }

    /// The stage's value on the curve.
    #[inline]
    pub fn f(&self, s: Stage) -> f64 {
        self.f[s.index()]
    }

    /// The stage's skill share, 0 to 1, 0.5 at rating 10.
    #[inline]
    pub fn share(&self, s: Stage) -> f64 {
        self.share[s.index()]
    }

    /// The stage's skill from −1 to 1: `2 · share − 1`, 0 at rating 10.
    #[inline]
    pub fn skill(&self, s: Stage) -> f64 {
        2.0 * self.share(s) - 1.0
    }

    /// The mean of two stages' shares: an execution read with an opponent close blends the
    /// execute stage with the pressure stage.
    #[inline]
    pub fn pressed_share(&self, execute: Stage, pressure: Stage) -> f64 {
        (self.share(execute) + self.share(pressure)) / 2.0
    }
}

/// One player's stage values as play reads them, with the values derived beside them: his
/// effective ones in play, or his base ones fresh.
#[derive(Debug, Clone, Copy)]
pub struct Skills<'a> {
    /// The stage values.
    pub stages: &'a StageValues,
    /// The derived values: the gates and knobs a contest reads.
    pub derived: &'a crate::player::Derived,
}

impl<'a> Skills<'a> {
    #[inline]
    pub fn new(stages: &'a StageValues, derived: &'a crate::player::Derived) -> Self {
        Self { stages, derived }
    }

    /// The stage's value on the curve.
    #[inline]
    pub fn f(&self, s: Stage) -> f64 {
        self.stages.f(s)
    }

    /// The stage's skill share, 0.5 at rating 10.
    #[inline]
    pub fn share(&self, s: Stage) -> f64 {
        self.stages.share(s)
    }

    /// The stage's skill from −1 to 1: `2 · share − 1`, 0 at rating 10.
    #[inline]
    pub fn skill(&self, s: Stage) -> f64 {
        2.0 * self.share(s) - 1.0
    }

    /// The mean of two stages' shares: an execution read with an opponent close blends the
    /// execute stage with the pressure stage.
    #[inline]
    pub fn pressed_share(&self, execute: Stage, pressure: Stage) -> f64 {
        (self.share(execute) + self.share(pressure)) / 2.0
    }
}

/// The skill gates of one player: whether he tries a gated skill (technique), and the
/// log-odds his execution loses when his body cannot pull it off (agility).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gates {
    /// He tries a chip: a lofted pass over 25 m.
    pub chip_try: bool,
    /// Log-odds his chip's execution loses; 0 when he pulls it off.
    pub chip_penalty: f64,
    /// He tries a take-on: a dribble with an opponent close.
    pub take_on_try: bool,
    /// Log-odds his take-on loses in a tackle; 0 when he pulls it off.
    pub take_on_penalty: f64,
}

impl Gates {
    /// No gate: every skill is tried and pulled off.
    pub const OPEN: Gates = Gates {
        chip_try: true,
        chip_penalty: 0.0,
        take_on_try: true,
        take_on_penalty: 0.0,
    };
}

/// The four ratings play reads directly rather than through a stage, on the 1 to 20 scale:
/// pace (the speed map), technique (whether a skill is tried), agility (whether it is pulled
/// off), and consistency (the spread of every action).
pub fn direct_ratings(
    a: &crate::player::Attributes,
    schema: &crate::data::attributes::AttributeSchema,
) -> [f64; 4] {
    schema.required_indices().map(|i| a.get(i).decimal())
}

/// A rating as a share from 0 to 1, `tenths / 200`: the value a plugin hook sees for a
/// rating, as it saw it before the contract (old scale / 100).
pub fn share_of(r: Rating) -> f64 {
    f64::from(r.tenths()) / 200.0
}

/// A shot from within this many metres of the keeper is a one-on-one for him.
pub const ONE_ON_ONE_M: f64 = 12.0;

/// Passes longer than this, in metres, are lofted: a chip.
pub const CHIP_M: f64 = 25.0;

/// Across-the-pitch distance, in metres, beyond which a pass into the penalty area is a
/// cross.
pub const CROSS_WIDE_M: f64 = 20.0;

/// Height in metres above which a ball is met with the head.
pub const HEAD_HEIGHT_M: f64 = 1.8;

/// A keeper's pass of at most this many metres is thrown; a longer one is kicked.
pub const THROW_M: f64 = 30.0;

/// Distance from the carrier, in metres, within which an opponent presses him.
pub const PRESSED_M: f64 = 2.5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stage_list_has_one_entry_per_pair_and_the_count_matches() {
        assert_eq!(STAGES.len(), STAGE_COUNT);
        for (i, &(a, s)) in STAGES.iter().enumerate() {
            assert_eq!(Stage::find(a, s), Some(Stage(i as u8)));
        }
        for a in ActionKind::ALL {
            assert_eq!(ActionKind::from_name(a.name()), Some(a));
            assert!(a.stages().count() > 0, "{} has no stage", a.name());
        }
        assert_eq!(Stage::RUSH_CHOOSE.name(), "rush.choose");
    }
}
