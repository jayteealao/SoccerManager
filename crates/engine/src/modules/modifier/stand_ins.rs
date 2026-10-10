//! The weather modifier, a named stand-in with no effect yet, and the off versions of the
//! pressure and momentum slots, whose first versions read mood ([`super::mood`]). Each has
//! no effect. The weather's effect comes with its own piece.

use super::{Family, Neutral};
use crate::modules::ModuleCard;

pub const PRESSURE_OFF: Neutral = Neutral(Family::Mind);
pub const MOMENTUM_OFF: Neutral = Neutral(Family::Mind);
pub const WEATHER_V1: Neutral = Neutral(Family::Surroundings);
pub const WEATHER_OFF: Neutral = Neutral(Family::Surroundings);

/// The card of a stand-in or an off version.
const fn stand_in_card(purpose: &'static str, calibration: &'static str) -> ModuleCard {
    ModuleCard {
        purpose,
        inputs: "Nothing.",
        outputs: "A delta of 0 on every attribute group.",
        tuning: &["none"],
        calibration,
        keys: &[],
    }
}

pub const PRESSURE_OFF_CARD: ModuleCard = stand_in_card(
    "Own mood switched off (mind family): no effect.",
    "none: off version, no mood effect",
);
pub const MOMENTUM_OFF_CARD: ModuleCard = stand_in_card(
    "Team mood switched off (mind family): no effect.",
    "none: off version, no team mood effect",
);
pub const WEATHER_V1_CARD: ModuleCard = stand_in_card(
    "The weather around a player (surroundings family): a stand-in with no effect until its own piece.",
    "none: no-op stand-in, no effect until its own piece",
);
pub const WEATHER_OFF_CARD: ModuleCard = stand_in_card(
    "Weather switched off (surroundings family): no effect.",
    "none: no-op stand-in, no effect until its own piece",
);
