//! The own-mood and team-mood modifiers (mind family): a player's mood and his side's mood
//! acting on his mental ratings, inside the mind cap with adaptation. They fill the pressure
//! and momentum slots under their first names and versions, so every slot file still
//! resolves; with no person block in the tuning both weights are 0 and neither acts.

use super::{Effect, Family, Modifier};
use crate::contract::states::GROUP_COUNT;
use crate::data::attributes::Group;
use crate::modules::{MatchView, ModuleCard};

/// `points` on the mental group alone.
fn mental(points: f64) -> Effect {
    let mut groups = [0.0; GROUP_COUNT];
    groups[Group::Mental as usize] = points;
    Effect { groups }
}

/// Own mood version 1 (`pressure@1`, mind family): `own_weight · mood / 100` on the mental
/// group.
pub struct OwnMoodV1;

impl Modifier for OwnMoodV1 {
    fn family(&self) -> Family {
        Family::Mind
    }

    fn effect(&self, view: &MatchView<'_>, i: usize) -> Effect {
        let w = view.tuning().contract.person().mood.own_weight;
        mental(w * view.mood(i) / crate::contract::mood::FULL)
    }
}

pub const OWN_MOOD_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Moves a player's mental ratings with his own mood (mind family): his events and the score lift or lower him.",
    inputs: "The player's mood (-100 to 100, starting at his kick-off mood from his recent form and season-goal progress weighted by his ambition, moved by his goals, assists, big misses, errors, cards, fouls suffered, duels and his side's goals, and drifting with the score) and the person tuning.",
    outputs: "A delta on the mental group, own_weight points at a full mood either way, inside the mind cap.",
    tuning: &["contract.person.mood.own_weight"],
    calibration: "none: no mood band in realism-bands.json",
    keys: &[],
};

/// Team mood version 1 (`momentum@1`, mind family): `team_weight · team mood / 100` on the
/// mental group, the team mood being his side's leadership-weighted mean.
pub struct TeamMoodV1;

impl Modifier for TeamMoodV1 {
    fn family(&self) -> Family {
        Family::Mind
    }

    fn effect(&self, view: &MatchView<'_>, i: usize) -> Effect {
        let w = view.tuning().contract.person().mood.team_weight;
        if w == 0.0 {
            return mental(0.0);
        }
        let team = view.player(i).team;
        mental(w * view.team_mood(team) / crate::contract::mood::FULL)
    }
}

pub const TEAM_MOOD_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Moves a player's mental ratings with his side's mood (mind family): the mean of his team-mates' moods, each weighted by his leadership.",
    inputs: "Every active team-mate's mood and leadership, and the person tuning.",
    outputs: "A delta on the mental group, team_weight points at a full team mood either way, inside the mind cap.",
    tuning: &[
        "contract.person.mood.team_weight",
        "contract.person.mood.leadership_weight",
    ],
    calibration: "none: no mood band in realism-bands.json",
    keys: &[],
};
