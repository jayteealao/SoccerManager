//! The pressure, momentum, and weather modifiers: named stand-ins with no effect yet. Each
//! has an off version, which also has no effect. Their effects come with their own pieces.

use super::{Family, Neutral};
use crate::modules::ModuleCard;

pub const PRESSURE_V1: Neutral = Neutral(Family::Mind);
pub const PRESSURE_OFF: Neutral = Neutral(Family::Mind);
pub const MOMENTUM_V1: Neutral = Neutral(Family::Mind);
pub const MOMENTUM_OFF: Neutral = Neutral(Family::Mind);
pub const WEATHER_V1: Neutral = Neutral(Family::Surroundings);
pub const WEATHER_OFF: Neutral = Neutral(Family::Surroundings);

/// The card of a stand-in or its off version.
const fn stand_in_card(purpose: &'static str) -> ModuleCard {
    ModuleCard {
        purpose,
        inputs: "Nothing.",
        outputs: "A delta of 0 on every attribute group.",
        tuning: &["none"],
        calibration: "none: no-op stand-in, no effect until its own piece",
        keys: &[],
    }
}

pub const PRESSURE_V1_CARD: ModuleCard = stand_in_card(
    "Pressure on a player's mind (mind family): a stand-in with no effect until its own piece.",
);
pub const PRESSURE_OFF_CARD: ModuleCard =
    stand_in_card("Pressure switched off (mind family): no effect.");
pub const MOMENTUM_V1_CARD: ModuleCard = stand_in_card(
    "A team's momentum on a player's mind (mind family): a stand-in with no effect until its own piece.",
);
pub const MOMENTUM_OFF_CARD: ModuleCard =
    stand_in_card("Momentum switched off (mind family): no effect.");
pub const WEATHER_V1_CARD: ModuleCard = stand_in_card(
    "The weather around a player (surroundings family): a stand-in with no effect until its own piece.",
);
pub const WEATHER_OFF_CARD: ModuleCard =
    stand_in_card("Weather switched off (surroundings family): no effect.");
