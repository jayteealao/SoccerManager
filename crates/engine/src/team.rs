//! Teams, the 4-4-2 formation, and the formation anchor mechanism.
//!
//! A formation anchor is a slot position plus a fraction of the ball offset, so the whole
//! team shifts toward the ball while keeping its shape. It replaces free-roaming agents.
//! A team is built from a validated team file; the first eleven entries in file order fill
//! the formation slots.

use crate::data::attributes::AttributeSchema;
use crate::data::team::{Kit, TeamFile};
use crate::error::EngineError;
use crate::math::DVec2;
use crate::pitch;
use crate::player::{Attributes, Derived, Player};
use crate::tuning::Tuning;

/// Number of players per team.
pub const PLAYERS_PER_TEAM: usize = 11;

/// A 4-4-2 formation in attack coordinates: `x` metres from the own goal line, `y` across.
pub const FORMATION_442: [(f64, f64); PLAYERS_PER_TEAM] = [
    (5.0, 0.0),
    (25.0, -22.0),
    (25.0, -8.0),
    (25.0, 8.0),
    (25.0, 22.0),
    (45.0, -24.0),
    (45.0, -8.0),
    (45.0, 8.0),
    (45.0, 24.0),
    (65.0, -8.0),
    (65.0, 8.0),
];

/// A team: club identity, an attack direction, and a formation.
#[derive(Debug, Clone)]
pub struct Team {
    pub index: usize,
    /// +1.0 attacks toward positive `x`; -1.0 toward negative `x`.
    pub attack_x: f64,
    pub formation: [(f64, f64); PLAYERS_PER_TEAM],
    pub club_id: String,
    pub name: String,
    pub kit: Kit,
    /// Every player id in the team file, in file order; the first eleven start.
    pub player_ids: Vec<String>,
}

impl Team {
    /// A team with a club identity and no players yet; `index` 0 attacks positive `x`.
    pub fn new(index: usize, club_id: String, name: String, kit: Kit) -> Self {
        Self {
            index,
            attack_x: if index == 0 { 1.0 } else { -1.0 },
            formation: FORMATION_442,
            club_id,
            name,
            kit,
            player_ids: Vec::new(),
        }
    }

    /// Builds the team and its eleven starters from a validated team file. Player ids run
    /// from `first_id`.
    pub fn from_file(
        index: usize,
        file: &TeamFile,
        schema: &AttributeSchema,
        tuning: &Tuning,
    ) -> Result<(Self, Vec<Player>), EngineError> {
        if file.players.len() < PLAYERS_PER_TEAM {
            return Err(EngineError::InvalidConfig(format!(
                "club {} has {} players; a match needs {PLAYERS_PER_TEAM}",
                file.club.id,
                file.players.len()
            )));
        }
        let mut team = Self::new(
            index,
            file.club.id.clone(),
            file.club.name.clone(),
            file.club.kit.clone(),
        );
        team.player_ids = file.players.iter().map(|p| p.id.clone()).collect();
        let first_id = index * PLAYERS_PER_TEAM;
        let players = file
            .players
            .iter()
            .take(PLAYERS_PER_TEAM)
            .enumerate()
            .map(|(slot, entry)| {
                let attributes = Attributes::from_entry(&entry.attributes, schema);
                let derived = Derived::from_attributes(&attributes, schema, tuning);
                let pos = team.slot_base(slot);
                Player {
                    id: first_id + slot,
                    team: index,
                    slot,
                    shirt: entry.shirt,
                    attributes,
                    derived,
                    pos,
                    vel: DVec2::ZERO,
                    target: pos,
                    facing: DVec2::new(team.attack_x, 0.0),
                }
            })
            .collect();
        Ok((team, players))
    }

    /// The goal this team attacks.
    pub fn target_goal(&self) -> DVec2 {
        pitch::goal_centre(self.attack_x)
    }

    /// The goal this team defends.
    pub fn own_goal(&self) -> DVec2 {
        pitch::goal_centre(-self.attack_x)
    }

    /// A slot position in pitch coordinates with the ball at the centre.
    pub fn slot_base(&self, slot: usize) -> DVec2 {
        let (fx, fy) = self.formation[slot];
        DVec2::new((fx - pitch::HALF_LENGTH) * self.attack_x, fy)
    }

    /// The formation anchor for `slot` given the ball position.
    pub fn anchor(&self, slot: usize, ball: DVec2, t: &Tuning) -> DVec2 {
        if slot == 0 {
            // The keeper stands on the line from the goal centre toward the ball.
            let goal = self.own_goal();
            let out = crate::math::toward(goal, ball) * t.keeper_depth;
            let at = DVec2::new(goal.x + out.x, out.y.clamp(-3.0, 3.0));
            return pitch::clamp(at, 0.5);
        }
        let base = self.slot_base(slot);
        let shift = DVec2::new(ball.x * t.compactness_x, ball.y * t.compactness_y);
        pitch::clamp(base + shift, 0.5)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A team with a placeholder identity, for tests of geometry alone.
    pub(crate) fn bare(index: usize) -> Team {
        Team::new(
            index,
            format!("test-{index}"),
            format!("Test {index}"),
            Kit {
                primary: "#ffffff".into(),
                secondary: "#000000".into(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::bare;
    use super::*;

    #[test]
    fn anchors_stay_inside_the_pitch_for_every_corner() {
        let t = Tuning::default();
        for index in 0..2 {
            let team = bare(index);
            for ball in [
                DVec2::new(52.5, 34.0),
                DVec2::new(-52.5, 34.0),
                DVec2::new(52.5, -34.0),
                DVec2::new(-52.5, -34.0),
            ] {
                for slot in 0..PLAYERS_PER_TEAM {
                    assert!(pitch::contains(team.anchor(slot, ball, &t)));
                }
            }
        }
    }

    #[test]
    fn away_team_mirrors_home() {
        let home = bare(0);
        let away = bare(1);
        assert_eq!(home.slot_base(0).x, -away.slot_base(0).x);
        assert!(home.slot_base(9).x > 0.0 && away.slot_base(9).x < 0.0);
    }
}
