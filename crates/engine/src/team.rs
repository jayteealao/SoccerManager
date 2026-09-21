//! Teams, the built-in 4-4-2 formation, and the formation anchor mechanism.
//!
//! A formation anchor is a slot position plus a fraction of the ball offset, so the whole
//! team shifts toward the ball while keeping its shape. It replaces free-roaming agents.

use crate::math::DVec2;
use crate::pitch;
use crate::player::{Attributes, Player};
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

/// A team: an attack direction and a formation.
#[derive(Debug, Clone)]
pub struct Team {
    pub index: usize,
    /// +1.0 attacks toward positive `x`; -1.0 toward negative `x`.
    pub attack_x: f64,
    pub formation: [(f64, f64); PLAYERS_PER_TEAM],
}

impl Team {
    /// The built-in home (index 0) or away (index 1) team.
    pub fn builtin(index: usize) -> Self {
        Self {
            index,
            attack_x: if index == 0 { 1.0 } else { -1.0 },
            formation: FORMATION_442,
        }
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

    /// The eleven players of this team at their kick-off positions.
    pub fn players(&self, first_id: usize, attributes: Attributes) -> Vec<Player> {
        (0..PLAYERS_PER_TEAM)
            .map(|slot| {
                let pos = self.slot_base(slot);
                Player {
                    id: first_id + slot,
                    team: self.index,
                    slot,
                    shirt: u8::try_from(slot + 1).unwrap_or(0),
                    attributes,
                    pos,
                    vel: DVec2::ZERO,
                    target: pos,
                    facing: DVec2::new(self.attack_x, 0.0),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_stay_inside_the_pitch_for_every_corner() {
        let t = Tuning::default();
        for index in 0..2 {
            let team = Team::builtin(index);
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
        let home = Team::builtin(0);
        let away = Team::builtin(1);
        assert_eq!(home.slot_base(0).x, -away.slot_base(0).x);
        assert!(home.slot_base(9).x > 0.0 && away.slot_base(9).x < 0.0);
    }
}
