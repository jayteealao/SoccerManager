//! Teams, formations, and the formation anchor mechanism.
//!
//! A formation anchor is a slot position plus a fraction of the ball offset, so the whole
//! team shifts toward the ball while keeping its shape. It replaces free-roaming agents. The
//! team's plan (its tactics turned into numbers) moves the block up or down, widens or
//! narrows it, and moves each slot by its duty. A team is built from a validated team file:
//! the whole file is the squad, and the lineup names which squad player fills each formation
//! slot. When a player leaves play (sent off or injured) the rest of that player's line
//! spreads evenly across the line's width, so the shape stays balanced with ten; a substitute
//! who replaces an injured player puts the line back.

use crate::data::attributes::AttributeSchema;
use crate::data::tactics::TacticsSchema;
use crate::data::team::{Kit, Position, TeamFile};
use crate::error::EngineError;
use crate::math::DVec2;
use crate::pitch;
use crate::player::{Attributes, Derived, Player, Status};
use crate::tactics::{RoleDuty, Tactics, TeamPlan};
use crate::tuning::Tuning;

/// Number of players per team.
pub const PLAYERS_PER_TEAM: usize = 11;

/// A 4-4-2 formation in attack coordinates: `x` metres from the own goal line, `y` across.
/// The shipped tactics file's 4-4-2 has these slots.
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

/// One squad player as the team file gives it.
#[derive(Debug, Clone)]
pub struct SquadPlayer {
    pub shirt: u8,
    pub position: Position,
    pub attributes: Attributes,
    pub derived: Derived,
}

/// A team: club identity, an attack direction, the squad, the lineup and bench, the
/// tactics, and the formation in force.
#[derive(Debug, Clone)]
pub struct Team {
    pub index: usize,
    /// +1.0 attacks toward positive `x`; -1.0 toward negative `x`.
    pub attack_x: f64,
    /// The slots in force: the formation's slots with each line closed up around the slots
    /// whose players left play.
    pub formation: [(f64, f64); PLAYERS_PER_TEAM],
    /// The formation's slots as the tactics file gives them.
    pub base_formation: [(f64, f64); PLAYERS_PER_TEAM],
    pub club_id: String,
    pub name: String,
    pub kit: Kit,
    /// Every player id in the team file, in file order (the squad index).
    pub player_ids: Vec<String>,
    /// Every player's display name in the team file, in file order (the squad index).
    pub player_names: Vec<String>,
    /// The squad, in file order.
    pub squad: Vec<SquadPlayer>,
    /// The squad index of the player in each formation slot.
    pub lineup: [usize; PLAYERS_PER_TEAM],
    /// Squad indices of the substitutes still available, in the order they were named.
    pub bench: Vec<usize>,
    pub tactics: Tactics,
    pub plan: TeamPlan,
    /// `false` for a slot whose player left play (sent off, or injured and not replaced).
    pub active: [bool; PLAYERS_PER_TEAM],
}

impl Team {
    /// A team with a club identity and no players yet; `index` 0 attacks positive `x`. It
    /// plays a 4-4-2 with the neutral plan.
    pub fn new(index: usize, club_id: String, name: String, kit: Kit) -> Self {
        let t = Tuning::default();
        Self {
            index,
            attack_x: if index == 0 { 1.0 } else { -1.0 },
            formation: FORMATION_442,
            base_formation: FORMATION_442,
            club_id,
            name,
            kit,
            player_ids: Vec::new(),
            player_names: Vec::new(),
            squad: Vec::new(),
            lineup: std::array::from_fn(|slot| slot),
            bench: Vec::new(),
            tactics: Tactics {
                formation: 0,
                mentality: 0,
                instructions: [0; 6],
                roles: [RoleDuty::default(); PLAYERS_PER_TEAM],
            },
            plan: TeamPlan::neutral(&t),
            active: [true; PLAYERS_PER_TEAM],
        }
    }

    /// Builds the team and its eleven starters from a validated team file: the first eleven
    /// entries in file order fill the slots and the rest form the bench. Player ids run from
    /// `index * 11`. The AI manager's pre-match setup replaces the lineup and the tactics.
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
        team.player_names = file.players.iter().map(|p| p.name.clone()).collect();
        team.squad = file
            .players
            .iter()
            .map(|entry| {
                let attributes = Attributes::from_entry(&entry.attributes, schema);
                SquadPlayer {
                    shirt: entry.shirt,
                    position: entry.position,
                    derived: Derived::from_attributes(&attributes, schema, tuning),
                    attributes,
                }
            })
            .collect();
        team.bench = (PLAYERS_PER_TEAM..file.players.len()).collect();
        let players = team.starters();
        Ok((team, players))
    }

    /// The eleven players of the lineup, each at its slot's base position, fresh.
    pub fn starters(&self) -> Vec<Player> {
        (0..PLAYERS_PER_TEAM)
            .map(|slot| {
                let pos = self.slot_base(slot);
                self.player(slot, self.lineup[slot], pos)
            })
            .collect()
    }

    /// Squad player `squad` in `slot`, fresh and at rest at `pos`.
    pub fn player(&self, slot: usize, squad: usize, pos: DVec2) -> Player {
        let s = &self.squad[squad];
        Player {
            id: self.index * PLAYERS_PER_TEAM + slot,
            team: self.index,
            slot,
            squad,
            shirt: s.shirt,
            attributes: s.attributes,
            derived: s.derived,
            base: s.derived,
            energy: 1.0,
            pos,
            vel: DVec2::ZERO,
            target: pos,
            facing: DVec2::new(self.attack_x, 0.0),
            status: Status::OnPitch,
            yellow: 0,
        }
    }

    /// Puts `tactics` in force: the formation's slots from the tactics file with every line
    /// closed up around the slots out of play, and the plan play reads.
    pub fn set_tactics(&mut self, tactics: Tactics, schema: &TacticsSchema, t: &Tuning) {
        self.tactics = tactics;
        self.base_formation = schema.slots(usize::from(tactics.formation));
        self.plan = TeamPlan::from(&tactics, schema, t);
        self.relayout();
    }

    /// The formation's slots, with each line closed up around every inactive slot.
    fn relayout(&mut self) {
        self.formation = self.base_formation;
        let inactive: Vec<usize> = (0..PLAYERS_PER_TEAM).filter(|&s| !self.active[s]).collect();
        for slot in inactive {
            self.reshape(slot);
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

    /// Removes `slot` from the formation. The other active slots of its line (the slots with
    /// the same depth) spread evenly across the line's width, in their order across it.
    pub fn reshape(&mut self, slot: usize) {
        self.active[slot] = false;
        let depth = self.formation[slot].0;
        let line: Vec<usize> = (0..PLAYERS_PER_TEAM)
            .filter(|&s| self.formation[s].0 == depth)
            .collect();
        let (lo, hi) = line
            .iter()
            .map(|&s| self.formation[s].1)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), y| {
                (lo.min(y), hi.max(y))
            });
        let mut left: Vec<usize> = line.into_iter().filter(|&s| self.active[s]).collect();
        left.sort_by(|a, b| self.formation[*a].1.total_cmp(&self.formation[*b].1));
        let n = left.len();
        for (k, s) in left.into_iter().enumerate() {
            self.formation[s].1 = if n == 1 {
                (lo + hi) / 2.0
            } else {
                lo + (hi - lo) * k as f64 / (n - 1) as f64
            };
        }
    }

    /// Puts `slot` back into the formation, as when a substitute replaces an injured player:
    /// every line is laid out again from the formation around the slots still out of play.
    pub fn restore(&mut self, slot: usize) {
        self.active[slot] = true;
        self.relayout();
    }

    /// Turns the team round to attack the other goal, as at half-time.
    pub fn switch_ends(&mut self) {
        self.attack_x = -self.attack_x;
    }

    /// The formation anchor for `slot` given the ball position: the slot moved up by the
    /// plan's block depth and the slot's duty, widened by the plan, plus the ball shift.
    pub fn anchor(&self, slot: usize, ball: DVec2, t: &Tuning) -> DVec2 {
        if slot == 0 {
            // The keeper stands on the line from the goal centre toward the ball.
            let goal = self.own_goal();
            let out = crate::math::toward(goal, ball) * t.keeper_depth;
            let at = DVec2::new(goal.x + out.x, out.y.clamp(-3.0, 3.0));
            return pitch::clamp(at, 0.5);
        }
        let (fx, fy) = self.formation[slot];
        let depth = fx + self.plan.block_depth + self.plan.slots[slot].depth;
        let base = DVec2::new(
            (depth - pitch::HALF_LENGTH) * self.attack_x,
            fy * self.plan.width,
        );
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
    use crate::data::test_support::shipped_content;

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
    fn anchors_stay_inside_the_pitch_for_every_formation_mentality_and_duty() {
        let content = shipped_content();
        let schema = &content.tactics;
        let t = &content.tuning.engine;
        for index in 0..2 {
            let mut team = bare(index);
            for f in 0..schema.formations.len() {
                for m in 0..schema.mentalities.len() {
                    for d in 0..schema.duties.len() {
                        for line in 0..3 {
                            let mut tactics = Tactics::defaults(schema);
                            tactics.set_formation(f as u8, schema);
                            tactics.mentality = m as u8;
                            tactics.instructions[crate::data::tactics::LINE_HEIGHT] = line;
                            tactics.instructions[crate::data::tactics::WIDTH] = line;
                            for rd in &mut tactics.roles {
                                rd.duty = d as u8;
                            }
                            team.set_tactics(tactics, schema, t);
                            for ball in [
                                DVec2::new(52.5, 34.0),
                                DVec2::new(-52.5, -34.0),
                                DVec2::ZERO,
                            ] {
                                for slot in 0..PLAYERS_PER_TEAM {
                                    assert!(pitch::contains(team.anchor(slot, ball, t)));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_back_four_with_one_sent_off_spreads_three_across_the_width() {
        let mut team = bare(0);
        team.reshape(2);
        let ys: Vec<f64> = [1, 3, 4].iter().map(|&s| team.formation[s].1).collect();
        assert_eq!(ys, vec![-22.0, 0.0, 22.0]);
        assert!(!team.active[2]);
        let t = Tuning::default();
        for slot in [1, 3, 4] {
            assert!(pitch::contains(team.anchor(slot, DVec2::ZERO, &t)));
        }
        team.reshape(9);
        assert_eq!(team.formation[10].1, 0.0);
    }

    #[test]
    fn restoring_a_slot_puts_its_line_back_and_keeps_the_others_closed() {
        let mut team = bare(0);
        team.reshape(2);
        team.reshape(9);
        team.restore(2);
        assert_eq!(&team.formation[1..5], &FORMATION_442[1..5]);
        assert!(team.active[2]);
        assert!(!team.active[9]);
        assert_eq!(team.formation[10].1, 0.0);
    }

    #[test]
    fn switching_ends_flips_the_attack() {
        let mut team = bare(0);
        let before = team.slot_base(9);
        team.switch_ends();
        assert_eq!(team.attack_x, -1.0);
        assert_eq!(team.slot_base(9).x, -before.x);
    }

    #[test]
    fn away_team_mirrors_home() {
        let home = bare(0);
        let away = bare(1);
        assert_eq!(home.slot_base(0).x, -away.slot_base(0).x);
        assert!(home.slot_base(9).x > 0.0 && away.slot_base(9).x < 0.0);
    }
}
