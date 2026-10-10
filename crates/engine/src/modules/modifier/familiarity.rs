//! The familiarity modifier (familiarity family): a player away from his natural position
//! loses some of his technical and mental ratings, by the gap between his natural rating and
//! his rating at the position his formation slot plays, scaled down by his versatility and
//! adaptability ([`crate::contract::states::familiarity_drop`]). A player at his natural
//! position loses nothing. Its off version has no effect.

use super::{Effect, Family, Modifier, Neutral};
use crate::contract::states::{GROUP_COUNT, familiarity_drop};
use crate::data::attributes::Group;
use crate::modules::{MatchView, ModuleCard};

/// Familiarity version 1: the familiarity drop on the technical and mental groups.
pub struct FamiliarityV1;

impl Modifier for FamiliarityV1 {
    fn family(&self) -> Family {
        Family::Familiarity
    }

    fn effect(&self, view: &MatchView<'_>, i: usize) -> Effect {
        let drop = familiarity_drop(
            view.position_ratings(i),
            view.played_position(i),
            &view.personality(i),
            &view.tuning().contract.person().familiarity,
        );
        let mut groups = [0.0; GROUP_COUNT];
        groups[Group::Technical as usize] = drop;
        groups[Group::Mental as usize] = drop;
        Effect { groups }
    }
}

pub const FAMILIARITY_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Lowers a player's technical and mental ratings when he plays away from his natural position (familiarity family).",
    inputs: "The player's rating at each position (his natural position the highest), the position of the formation slot he fills, his versatility and adaptability, and the person tuning.",
    outputs: "A delta on the technical and mental groups: minus cost_per_point per point of the gap, scaled down by versatility and adaptability, 0 at his natural position, inside the familiarity cap.",
    tuning: &[
        "contract.person.familiarity.cost_per_point",
        "contract.person.familiarity.versatility_weight",
        "contract.person.familiarity.adaptability_weight",
    ],
    calibration: "none: no familiarity band in realism-bands.json",
    keys: &[],
};

/// Familiarity switched off: no effect at any position.
pub const FAMILIARITY_OFF: Neutral = Neutral(Family::Familiarity);

pub const FAMILIARITY_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "The familiarity modifier switched off (familiarity family): no effect at any position.",
    inputs: "Nothing.",
    outputs: "A delta of 0 on every attribute group.",
    tuning: &["none"],
    calibration: "none: off version, no familiarity effect",
    keys: &[],
};
