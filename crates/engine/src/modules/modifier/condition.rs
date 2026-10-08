//! The sharpness and adaptation modifiers: the match condition inputs of the team file acting
//! on a player's ratings. Sharpness (body family) is the timing of technical actions, so it
//! lowers the technical group; adaptation to a new country (mind family) lowers the mental
//! group. A player with no input is fully sharp and fully adapted. Each has an off version
//! with no effect.

use super::{Effect, Family, Modifier, Neutral};
use crate::contract::body::BodyJobs;
use crate::contract::states::GROUP_COUNT;
use crate::data::attributes::Group;
use crate::modules::{MatchView, ModuleCard};

/// `drop` rating points on `group` alone.
fn on(group: Group, drop: f64) -> Effect {
    let mut groups = [0.0; GROUP_COUNT];
    groups[group as usize] = drop;
    Effect { groups }
}

/// Sharpness version 1 (body family): `−(1 − sharpness / 100) · max_drop` on the technical
/// group.
pub struct SharpnessV1;

impl Modifier for SharpnessV1 {
    fn family(&self) -> Family {
        Family::Body
    }

    fn effect(&self, view: &MatchView<'_>, i: usize) -> Effect {
        let body = &view.tuning().contract.body;
        on(
            Group::Technical,
            BodyJobs::drop(&body.sharpness, view.condition(i).sharpness),
        )
    }
}

pub const SHARPNESS_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Lowers a player's technical ratings when he lacks match sharpness (body family): the timing of technical actions.",
    inputs: "Each player's sharpness input (0 to 100 percent, from the team file's condition block; absent reads 100) and the body tuning.",
    outputs: "A delta on the technical group, from 0 at full sharpness to minus max_drop at none.",
    tuning: &["contract.body.sharpness.max_drop"],
    calibration: "none: no sharpness band in realism-bands.json",
    keys: &[],
};

/// Sharpness switched off: no effect at any sharpness.
pub const SHARPNESS_OFF: Neutral = Neutral(Family::Body);

pub const SHARPNESS_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "The sharpness modifier switched off (body family): no effect at any sharpness.",
    inputs: "Nothing.",
    outputs: "A delta of 0 on every attribute group.",
    tuning: &["none"],
    calibration: "none: off version, no sharpness effect",
    keys: &[],
};

/// Adaptation version 1 (mind family): `−(1 − adaptation / 100) · max_drop` on the mental
/// group. Nationality acts through it: a player new to a country is given a low adaptation.
pub struct AdaptationV1;

impl Modifier for AdaptationV1 {
    fn family(&self) -> Family {
        Family::Mind
    }

    fn effect(&self, view: &MatchView<'_>, i: usize) -> Effect {
        let body = &view.tuning().contract.body;
        on(
            Group::Mental,
            BodyJobs::drop(&body.adaptation, view.condition(i).adaptation),
        )
    }
}

pub const ADAPTATION_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Lowers a player's mental ratings while he adapts to a new country (mind family): language, climate and the league's style.",
    inputs: "Each player's adaptation input (0 to 100 percent, from the team file's condition block; absent reads 100) and the body tuning.",
    outputs: "A delta on the mental group, from 0 when fully adapted to minus max_drop when not at all.",
    tuning: &["contract.body.adaptation.max_drop"],
    calibration: "none: no adaptation band in realism-bands.json",
    keys: &[],
};

/// Adaptation switched off: no effect at any adaptation.
pub const ADAPTATION_OFF: Neutral = Neutral(Family::Mind);

pub const ADAPTATION_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "The adaptation modifier switched off (mind family): no effect at any adaptation.",
    inputs: "Nothing.",
    outputs: "A delta of 0 on every attribute group.",
    tuning: &["none"],
    calibration: "none: off version, no adaptation effect",
    keys: &[],
};
