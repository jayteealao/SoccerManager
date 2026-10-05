//! The tactics model: a formation, a mentality, six team instructions, and a role and duty
//! per slot, each an index into the tactics file. A [`TeamPlan`] turns a team's tactics into
//! the numbers play reads (block depth, width, pressing, option offsets). It is computed once
//! each time the tactics change, never per tick.

pub mod change;
pub mod planned;
pub mod verdict;

use crate::data::tactics::{
    DIRECTNESS, LINE_HEIGHT, PRESSING, TEMPO, TIME_WASTING, TacticsSchema, WIDTH,
};
use crate::team::PLAYERS_PER_TEAM;
use crate::tuning::Tuning;

/// A role and a duty, as indices into the tactics file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RoleDuty {
    pub role: u8,
    pub duty: u8,
}

/// One team's tactics, as indices into the tactics file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tactics {
    pub formation: u8,
    pub mentality: u8,
    /// Levels in the order of [`crate::data::tactics::INSTRUCTIONS`].
    pub instructions: [u8; 6],
    /// Per formation slot.
    pub roles: [RoleDuty; PLAYERS_PER_TEAM],
}

impl Tactics {
    /// The tactics file's defaults for the AI manager: its formation, mentality, and duty,
    /// every instruction at its default, and each slot's first suitable role.
    pub fn defaults(schema: &TacticsSchema) -> Self {
        // A checked schema resolves every AI name and holds at most 64 roles and 16
        // formations, so the indices fit a byte.
        let formation = schema.formation_index(&schema.ai.formation).unwrap_or(0);
        let duty = schema.duty_index(&schema.ai.duty).unwrap_or(0) as u8;
        let mut roles = [RoleDuty::default(); PLAYERS_PER_TEAM];
        for (slot, rd) in roles.iter_mut().enumerate() {
            let position = schema.formations[formation].slots[slot].position;
            *rd = RoleDuty {
                role: schema.default_role(position) as u8,
                duty,
            };
        }
        Self {
            formation: formation as u8,
            mentality: schema.mentality_index(&schema.ai.mentality).unwrap_or(0) as u8,
            instructions: schema.instructions.defaults(),
            roles,
        }
    }

    /// Switches to `formation`. A slot whose role does not suit its new position takes the
    /// position's first suitable role and keeps its duty.
    pub fn set_formation(&mut self, formation: u8, schema: &TacticsSchema) {
        self.formation = formation;
        let slots = &schema.formations[usize::from(formation)].slots;
        for (rd, slot) in self.roles.iter_mut().zip(slots) {
            let suits = schema.roles[usize::from(rd.role)]
                .positions
                .contains(&slot.position);
            if !suits {
                rd.role = schema.default_role(slot.position) as u8;
            }
        }
    }
}

/// A change to a team's tactics. Every field is optional; a role change names the player by
/// squad index (the player's place in the team file).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TacticsPatch {
    pub formation: Option<u8>,
    pub mentality: Option<u8>,
    pub instructions: [Option<u8>; 6],
    pub roles: Vec<(usize, RoleDuty)>,
}

impl TacticsPatch {
    /// A patch that sets only the mentality.
    pub fn mentality(m: u8) -> Self {
        Self {
            mentality: Some(m),
            ..Self::default()
        }
    }

    /// A patch that sets only instruction `index` to `level`.
    pub fn instruction(index: usize, level: u8) -> Self {
        let mut p = Self::default();
        p.instructions[index] = Some(level);
        p
    }

    /// `tactics` with this patch applied for a team whose slots hold `lineup` (squad indices
    /// in slot order): the formation first, then the mentality, the instructions, and each
    /// named player's role. A role for a player not in `lineup` is skipped; the caller checks
    /// that first.
    pub fn applied_to(
        &self,
        mut tactics: Tactics,
        lineup: &[usize; PLAYERS_PER_TEAM],
        schema: &TacticsSchema,
    ) -> Tactics {
        if let Some(f) = self.formation {
            tactics.set_formation(f, schema);
        }
        if let Some(m) = self.mentality {
            tactics.mentality = m;
        }
        for (level, set) in tactics.instructions.iter_mut().zip(self.instructions) {
            if let Some(l) = set {
                *level = l;
            }
        }
        for (squad, rd) in &self.roles {
            if let Some(slot) = lineup.iter().position(|s| s == squad) {
                tactics.roles[slot] = *rd;
            }
        }
        tactics
    }

    /// `true` when every index lies inside the tactics file.
    pub fn in_range(&self, schema: &TacticsSchema) -> bool {
        let formation = self
            .formation
            .is_none_or(|f| usize::from(f) < schema.formations.len());
        let mentality = self
            .mentality
            .is_none_or(|m| usize::from(m) < schema.mentalities.len());
        let instructions =
            self.instructions.iter().enumerate().all(|(i, l)| {
                l.is_none_or(|l| usize::from(l) < schema.instructions.level_count(i))
            });
        let roles = self.roles.iter().all(|(_, rd)| {
            usize::from(rd.role) < schema.roles.len() && usize::from(rd.duty) < schema.duties.len()
        });
        formation && mentality && instructions && roles
    }
}

/// What one slot's role and duty add to play.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SlotPlan {
    /// Metres the anchor moves up the pitch.
    pub depth: f64,
    pub shoot: f64,
    pub dribble: f64,
    /// Added to a forward pass in proportion to its progress.
    pub progress: f64,
}

/// The numbers play reads from a team's tactics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeamPlan {
    /// Metres the whole block moves up: mentality plus line height.
    pub block_depth: f64,
    /// A scale on every slot's distance from the centre line.
    pub width: f64,
    /// Opponents who press this team's opponents' carrier, and from how far.
    pub press_count: usize,
    pub press_distance: f64,
    /// The widest gap between two neighbours in the back line, from the tuning.
    pub back_line_gap: f64,
    pub tempo: f64,
    pub directness: f64,
    /// A factor on the team's restart delay while it leads.
    pub time_wasting: f64,
    pub shoot: f64,
    pub progress: f64,
    pub hold: f64,
    pub slots: [SlotPlan; PLAYERS_PER_TEAM],
}

impl TeamPlan {
    /// The plan of `tactics` under the tactics file and the tuning.
    pub fn from(tactics: &Tactics, schema: &TacticsSchema, t: &Tuning) -> Self {
        let level = |index: usize| usize::from(tactics.instructions[index]);
        let ins = &schema.instructions;
        let press = &ins.pressing.levels[level(PRESSING)];
        let mentality = &schema.mentalities[usize::from(tactics.mentality)];
        let mut slots = [SlotPlan::default(); PLAYERS_PER_TEAM];
        for (plan, rd) in slots.iter_mut().zip(&tactics.roles) {
            let role = &schema.roles[usize::from(rd.role)];
            let duty = &schema.duties[usize::from(rd.duty)];
            *plan = SlotPlan {
                depth: duty.depth,
                shoot: role.shoot,
                dribble: role.dribble + duty.risk,
                progress: role.progress + duty.risk,
            };
        }
        Self {
            block_depth: mentality.block_depth + ins.line_height.levels[level(LINE_HEIGHT)].value,
            width: ins.width.levels[level(WIDTH)].value,
            press_count: usize::from(press.press_count),
            press_distance: t.press_distance * press.press_distance_scale,
            back_line_gap: t.back_line_gap,
            tempo: ins.tempo.levels[level(TEMPO)].value,
            directness: ins.passing_directness.levels[level(DIRECTNESS)].value,
            time_wasting: ins.time_wasting.levels[level(TIME_WASTING)].value,
            shoot: mentality.shoot,
            progress: mentality.progress,
            hold: mentality.hold,
            slots,
        }
    }

    /// The plan of the tactics file's defaults: the neutral plan the geometry tests use.
    pub fn neutral(t: &Tuning) -> Self {
        Self {
            block_depth: 0.0,
            width: 1.0,
            press_count: 2,
            press_distance: t.press_distance,
            back_line_gap: t.back_line_gap,
            tempo: 0.0,
            directness: 0.0,
            time_wasting: 1.0,
            shoot: 0.0,
            progress: 0.0,
            hold: 0.0,
            slots: [SlotPlan::default(); PLAYERS_PER_TEAM],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;
    use crate::team::FORMATION_442;

    #[test]
    fn balanced_four_four_two_with_support_duties_is_the_neutral_plan() {
        let content = shipped_content();
        let schema = &content.tactics;
        let t = &content.tuning.engine;
        let tactics = Tactics::defaults(schema);
        assert_eq!(schema.slots(usize::from(tactics.formation)), FORMATION_442);
        let plan = TeamPlan::from(&tactics, schema, t);
        assert_eq!(plan.block_depth, 0.0);
        assert_eq!(plan.width, 1.0);
        assert_eq!(plan.press_count, 2);
        assert_eq!(plan.press_distance, t.press_distance);
        assert_eq!(plan.time_wasting, 1.0);
        assert!(plan.slots.iter().all(|s| s.depth == 0.0));
    }

    #[test]
    fn attacking_moves_the_block_forward_and_high_pressing_sends_three() {
        let content = shipped_content();
        let schema = &content.tactics;
        let t = &content.tuning.engine;
        let mut tactics = Tactics::defaults(schema);
        tactics.mentality = schema.mentality_index("attacking").unwrap() as u8;
        tactics.instructions[PRESSING] = 2;
        let plan = TeamPlan::from(&tactics, schema, t);
        assert!(plan.block_depth > 0.0);
        assert!(plan.shoot > 0.0);
        assert_eq!(plan.press_count, 3);
        assert!(plan.press_distance > t.press_distance);
    }

    #[test]
    fn a_new_formation_resets_only_the_roles_that_no_longer_suit() {
        let content = shipped_content();
        let schema = &content.tactics;
        let mut tactics = Tactics::defaults(schema);
        let before = tactics.roles;
        tactics.set_formation(schema.formation_index("4-3-3").unwrap() as u8, schema);
        // The back four and the goalkeeper keep their roles.
        assert_eq!(&tactics.roles[..5], &before[..5]);
        // Slot 10 was a striker and stays one; slot 8 turns from a right winger's slot into
        // a left winger's, which the winger role also suits.
        let slot6 = schema.formations[1].slots[6].position;
        assert!(
            schema.roles[usize::from(tactics.roles[6].role)]
                .positions
                .contains(&slot6)
        );
    }
}
