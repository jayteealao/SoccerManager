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
use crate::pitch::Pitch;
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

/// Metres beyond the deepest outfield slot within which a slot still belongs to the back line:
/// a back three or five with its centre-back a little deeper is one line.
const BACK_LINE_BAND: f64 = 4.0;

/// The lowest and highest of `ys`.
fn span(ys: impl Iterator<Item = f64>) -> (f64, f64) {
    ys.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), y| {
        (lo.min(y), hi.max(y))
    })
}

/// One squad player as the team file gives it.
#[derive(Debug, Clone)]
pub struct SquadPlayer {
    pub shirt: u8,
    pub position: Position,
    pub attributes: Attributes,
    /// What his attributes give him fresh.
    pub derived: Derived,
    /// His base stage values.
    pub stages: crate::contract::StageValues,
    /// His height and age, as the team file gives them.
    pub body: crate::contract::body::Body,
    /// His match condition inputs; absent fields have no effect.
    pub condition: crate::data::team::Condition,
    /// His energy at kick-off, from his days of rest ([`Team::set_start_energy`]); 1.0
    /// when none are given.
    pub start_energy: f64,
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
    /// The ground of the match: the home team's. The formation, drawn for 105 by 68, scales
    /// to it. Private, so the ground is written only with the match's own copy
    /// (`MatchConfig::set_pitch`), and one match never holds two grounds.
    pitch: Pitch,
}

impl Team {
    /// A team with a club identity and no players yet; `index` 0 attacks positive `x`. It
    /// plays a 4-4-2 with the neutral plan.
    pub fn new(index: usize, club_id: String, name: String, kit: Kit) -> Self {
        let t = Tuning::default();
        let mut team = Self {
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
            pitch: Pitch::DEFAULT,
        };
        team.relayout();
        team
    }

    /// The ground of the match.
    pub fn pitch(&self) -> &Pitch {
        &self.pitch
    }

    /// Sets the ground. `MatchConfig` is the one caller, so the match and both teams always
    /// hold the same ground.
    pub(crate) fn set_pitch(&mut self, pitch: Pitch) {
        self.pitch = pitch;
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
        let blend = crate::contract::stages::Blend::of(schema);
        team.squad = file
            .players
            .iter()
            .map(|entry| {
                let attributes = Attributes::from_entry(&entry.attributes, schema);
                let body = crate::contract::body::Body {
                    height_cm: entry.height,
                    age: entry.age,
                };
                let (derived, stages) =
                    Derived::from_blend(&attributes, &blend, schema, tuning, body);
                SquadPlayer {
                    shirt: entry.shirt,
                    position: entry.position,
                    derived,
                    stages,
                    attributes,
                    body,
                    condition: entry.condition.unwrap_or_default(),
                    start_energy: 1.0,
                }
            })
            .collect();
        team.bench = (PLAYERS_PER_TEAM..file.players.len()).collect();
        let players = team.starters();
        Ok((team, players))
    }

    /// Sets every squad player's energy at kick-off from his days of rest, his age, and the
    /// fatigue tuning's recovery per rest day; a player with no rest days given starts full.
    pub fn set_start_energy(&mut self, tuning: &Tuning, recovery_per_day: f64) {
        let jobs = &tuning.contract.body;
        for s in &mut self.squad {
            s.start_energy = jobs.start_energy(s.condition.rest_days, s.body, recovery_per_day);
        }
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

    /// Squad player `squad` in `slot`, at his base values, with his kick-off energy, and at
    /// rest at `pos`.
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
            deltas: [0; crate::contract::states::GROUP_COUNT],
            form: 0,
            form_match: 0,
            energy: s.start_energy,
            pos,
            vel: DVec2::ZERO,
            target: pos,
            facing: DVec2::new(self.attack_x, 0.0),
            status: Status::OnPitch,
            yellow: 0,
            foul_ready: 0,
            lapse_until: 0,
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

    /// The formation's slots, with each line closed up around every slot out of the outfield
    /// shape: the inactive slots and an acting keeper who is not in slot 0. A line with a
    /// player removed spreads its survivors evenly across its width, and a lone survivor keeps
    /// its own place across the pitch. The back line is then kept compact: no two neighbours
    /// in it stand more than `back_line_gap` apart, so it narrows as players leave it. The
    /// acting keeper takes the keeper's place.
    pub(crate) fn relayout(&mut self) {
        self.formation = self.base_formation;
        let keeper = self.keeper_slot();
        let active = self.active;
        let out = |s: usize| !active[s] || (s == keeper && s != 0);
        let removed: Vec<usize> = (1..PLAYERS_PER_TEAM).filter(|&s| out(s)).collect();
        let mut done: Vec<f64> = Vec::new();
        for slot in removed {
            let depth = self.base_formation[slot].0;
            if done.contains(&depth) {
                continue;
            }
            done.push(depth);
            let line: Vec<usize> = (1..PLAYERS_PER_TEAM)
                .filter(|&s| self.base_formation[s].0 == depth)
                .collect();
            let (lo, hi) = span(line.iter().map(|&s| self.base_formation[s].1));
            let mut left: Vec<usize> = line.into_iter().filter(|&s| !out(s)).collect();
            left.sort_by(|a, b| {
                self.base_formation[*a]
                    .1
                    .total_cmp(&self.base_formation[*b].1)
            });
            let n = left.len();
            if n > 1 {
                for (k, s) in left.into_iter().enumerate() {
                    self.formation[s].1 = lo + (hi - lo) * k as f64 / (n - 1) as f64;
                }
            }
        }
        let mut back: Vec<usize> = self.back_line();
        back.retain(|&s| !out(s));
        back.sort_by(|a, b| self.formation[*a].1.total_cmp(&self.formation[*b].1));
        // The acting keeper organises the line: a better organiser keeps it tighter.
        let organise = self
            .squad
            .get(self.lineup[self.keeper_slot()])
            .map_or(1.0, |k| k.derived.knobs.organise);
        let gap = self.plan.back_line_gap / organise;
        let too_wide = back
            .windows(2)
            .any(|w| self.formation[w[1]].1 - self.formation[w[0]].1 > gap);
        if too_wide {
            let n = back.len();
            let (lo, hi) = span(back.iter().map(|&s| self.formation[s].1));
            let width = (hi - lo).min((n - 1) as f64 * gap);
            let from = (lo + hi) / 2.0 - width / 2.0;
            for (k, s) in back.into_iter().enumerate() {
                self.formation[s].1 = from + width * k as f64 / (n - 1) as f64;
            }
        }
        if keeper != 0 {
            self.formation[keeper] = self.base_formation[0];
        }
    }

    /// The back line: the outfield slots of the formation within `BACK_LINE_BAND` metres of
    /// the deepest outfield slot, in slot order. It is read from the formation as the tactics
    /// file gives it, so it does not change as players leave.
    pub fn back_line(&self) -> Vec<usize> {
        let deepest = (1..PLAYERS_PER_TEAM)
            .map(|s| self.base_formation[s].0)
            .fold(f64::INFINITY, f64::min);
        (1..PLAYERS_PER_TEAM)
            .filter(|&s| self.base_formation[s].0 <= deepest + BACK_LINE_BAND)
            .collect()
    }

    /// The team's lone forward: the single active slot, other than the one keeping goal, in
    /// the front line of the formation as the tactics file gives it. The front line is the
    /// outfield slots less than `BACK_LINE_BAND` metres behind the most advanced one. `None`
    /// when the front line has no active player or more than one.
    pub fn lone_forward(&self) -> Option<usize> {
        let front = (1..PLAYERS_PER_TEAM)
            .map(|s| self.base_formation[s].0)
            .fold(f64::NEG_INFINITY, f64::max);
        let keeper = self.keeper_slot();
        let mut lone = None;
        for s in 1..PLAYERS_PER_TEAM {
            if self.base_formation[s].0 > front - BACK_LINE_BAND && self.active[s] && s != keeper {
                if lone.is_some() {
                    return None;
                }
                lone = Some(s);
            }
        }
        lone
    }

    /// The slot of the player who keeps goal: slot 0 while it is active; otherwise the lowest
    /// active slot holding a goalkeeper by position (a keeper who came on); otherwise the most
    /// advanced active outfield slot of the formation, then the one nearest the centre line
    /// across the pitch, then the lowest slot. It is derived from the team's state, so a
    /// resumed match finds the same keeper.
    pub fn keeper_slot(&self) -> usize {
        if self.active[0] {
            return 0;
        }
        if let Some(slot) = (1..PLAYERS_PER_TEAM).find(|&s| {
            self.active[s]
                && self
                    .squad
                    .get(self.lineup[s])
                    .is_some_and(|p| p.position == Position::GK)
        }) {
            return slot;
        }
        (1..PLAYERS_PER_TEAM)
            .filter(|&s| self.active[s])
            .min_by(|&a, &b| {
                let (ax, ay) = self.base_formation[a];
                let (bx, by) = self.base_formation[b];
                bx.total_cmp(&ax)
                    .then(ay.abs().total_cmp(&by.abs()))
                    .then(a.cmp(&b))
            })
            .unwrap_or(0)
    }

    /// Opponents this team sends to press the carrier: the plan's count less one for each
    /// player out of play, and at least one while the plan presses at all.
    pub fn pressers(&self) -> usize {
        let out = self.active.iter().filter(|&&a| !a).count();
        let count = self.plan.press_count;
        count.saturating_sub(out).max(count.min(1))
    }

    /// The goal this team attacks.
    pub fn target_goal(&self) -> DVec2 {
        self.pitch.goal_centre(self.attack_x)
    }

    /// The goal this team defends.
    pub fn own_goal(&self) -> DVec2 {
        self.pitch.goal_centre(-self.attack_x)
    }

    /// A slot position in pitch coordinates with the ball at the centre. The formation is
    /// drawn for 105 by 68 and scales to the ground: along the touchline by its length over
    /// 105, across by its width over 68.
    pub fn slot_base(&self, slot: usize) -> DVec2 {
        let (fx, fy) = self.formation[slot];
        let (sx, sy) = self.pitch.scale();
        DVec2::new(
            (fx * sx - self.pitch.half_length()) * self.attack_x,
            fy * sy,
        )
    }

    /// Removes `slot` from the formation, as when its player leaves play: every line is laid
    /// out again around the slots out of play.
    pub fn reshape(&mut self, slot: usize) {
        self.active[slot] = false;
        self.relayout();
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
    /// plan's block depth, the slot's duty, and the role's offset for the phase (the team has
    /// the ball when `in_possession`), widened by the plan, plus the ball shift.
    pub fn anchor(&self, slot: usize, ball: DVec2, in_possession: bool, t: &Tuning) -> DVec2 {
        if slot == self.keeper_slot() {
            // The keeper stands on the line from the goal centre toward the ball.
            let goal = self.own_goal();
            let out = crate::math::toward(goal, ball) * t.keeper_depth;
            let at = DVec2::new(goal.x + out.x, out.y.clamp(-3.0, 3.0));
            return self.pitch.clamp(at, 0.5);
        }
        let (fx, fy) = self.formation[slot];
        let (sx, sy) = self.pitch.scale();
        let plan = &self.plan.slots[slot];
        let offset = if in_possession {
            plan.in_possession
        } else {
            plan.out_of_possession
        };
        // The role's offset comes after the existing sums, so an offset of 0 keeps every
        // anchor's bits.
        let depth = (fx + self.plan.block_depth + plan.depth + offset.x) * sx;
        let side = if fy < 0.0 { -1.0 } else { 1.0 };
        let base = DVec2::new(
            (depth - self.pitch.half_length()) * self.attack_x,
            (fy * self.plan.width + side * offset.y) * sy,
        );
        let shift = DVec2::new(ball.x * t.compactness_x, ball.y * t.compactness_y);
        self.pitch.clamp(base + shift, 0.5)
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
                    assert!(Pitch::DEFAULT.contains(team.anchor(slot, ball, false, &t)));
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
                                    assert!(
                                        Pitch::DEFAULT.contains(team.anchor(slot, ball, false, t))
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_back_four_is_kept_compact_at_eleven_against_eleven() {
        let team = bare(0);
        let t = Tuning::default();
        let ys: Vec<f64> = (1..5).map(|s| team.formation[s].1).collect();
        let g = t.back_line_gap;
        assert_eq!(ys, vec![-1.5 * g, -0.5 * g, 0.5 * g, 1.5 * g]);
        assert_eq!(&team.formation[5..], &FORMATION_442[5..]);
    }

    #[test]
    fn a_back_four_with_one_sent_off_closes_to_three_and_a_lone_striker_keeps_his_side() {
        let mut team = bare(0);
        team.reshape(2);
        let ys: Vec<f64> = [1, 3, 4].iter().map(|&s| team.formation[s].1).collect();
        assert_eq!(ys, vec![-12.0, 0.0, 12.0]);
        assert!(!team.active[2]);
        let t = Tuning::default();
        for slot in [1, 3, 4] {
            assert!(Pitch::DEFAULT.contains(team.anchor(slot, DVec2::ZERO, false, &t)));
        }
        team.reshape(9);
        assert_eq!(
            team.formation[10].1, 8.0,
            "the lone striker is not moved into the gap"
        );
    }

    #[test]
    fn restoring_a_slot_puts_its_line_back_and_keeps_the_others_closed() {
        let mut team = bare(0);
        let full = team.formation;
        team.reshape(2);
        team.reshape(9);
        team.restore(2);
        assert_eq!(&team.formation[1..5], &full[1..5]);
        assert!(team.active[2]);
        assert!(!team.active[9]);
        assert_eq!(team.formation[10].1, 8.0);
    }

    #[test]
    fn slot_zero_keeps_goal_while_it_is_active() {
        let team = bare(0);
        assert_eq!(team.keeper_slot(), 0);
    }

    #[test]
    fn a_goalkeeper_who_came_on_keeps_goal() {
        let content = shipped_content();
        let [a, _] = crate::data::test_support::default_teams(&content);
        let (mut team, _) =
            Team::from_file(0, &a, &content.attributes, &content.tuning.engine).unwrap();
        let gk = (PLAYERS_PER_TEAM..team.squad.len())
            .find(|&s| team.squad[s].position == Position::GK)
            .expect("a bench goalkeeper");
        team.reshape(0);
        team.lineup[9] = gk;
        team.relayout();
        assert_eq!(team.keeper_slot(), 9);
        assert_eq!(team.formation[9], team.base_formation[0]);
    }

    #[test]
    fn the_most_advanced_outfield_player_stands_in_goal_and_stays_there() {
        // 4-4-2: the strikers are level, so the one nearer the middle, then the lower slot.
        let mut team = bare(0);
        team.reshape(0);
        assert_eq!(team.keeper_slot(), 9);
        assert_eq!(team.formation[9], FORMATION_442[0]);
        assert_eq!(team.formation[10].1, 8.0, "the forward line is one short");
        for slot in [1, 5, 10, 3] {
            team.reshape(slot);
            assert_eq!(team.keeper_slot(), 9, "after slot {slot} left");
        }
        // 3-5-2 from the tactics file.
        let content = shipped_content();
        let schema = &content.tactics;
        let f = schema
            .formations
            .iter()
            .position(|f| f.name == "3-5-2")
            .unwrap();
        let mut team = bare(1);
        let mut tactics = Tactics::defaults(schema);
        tactics.set_formation(f as u8, schema);
        team.set_tactics(tactics, schema, &content.tuning.engine);
        team.reshape(0);
        let k = team.keeper_slot();
        let depth = team.base_formation[k].0;
        assert!((1..PLAYERS_PER_TEAM).all(|s| team.base_formation[s].0 <= depth));
    }

    /// A home team playing the tactics file's formation `name`.
    fn playing(name: &str) -> Team {
        let content = shipped_content();
        let schema = &content.tactics;
        let f = schema
            .formations
            .iter()
            .position(|f| f.name == name)
            .unwrap();
        let mut team = bare(0);
        let mut tactics = Tactics::defaults(schema);
        tactics.set_formation(f as u8, schema);
        team.set_tactics(tactics, schema, &content.tuning.engine);
        team
    }

    #[test]
    fn a_formation_with_one_striker_has_a_lone_forward() {
        for name in ["4-4-1-1", "3-4-3"] {
            let team = playing(name);
            let s = team.lone_forward().expect(name);
            assert!(
                (1..PLAYERS_PER_TEAM).all(|o| team.base_formation[o].0 <= team.base_formation[s].0),
                "{name}: the lone forward is the most advanced slot"
            );
            assert_eq!(team.base_formation[s].1, 0.0, "{name}: he is central");
        }
    }

    #[test]
    fn two_strikers_are_not_a_lone_forward_until_one_leaves() {
        let mut team = playing("4-4-2");
        assert_eq!(team.lone_forward(), None);
        team.reshape(9);
        assert_eq!(team.lone_forward(), Some(10));
        team.restore(9);
        team.reshape(0);
        assert_eq!(team.keeper_slot(), 9, "a striker stands in goal");
        assert_eq!(team.lone_forward(), Some(10), "the other striker is alone");
        team.reshape(10);
        assert_eq!(team.lone_forward(), None, "no forward is left");
    }

    #[test]
    fn a_team_presses_with_one_fewer_for_each_player_lost() {
        let mut team = bare(0);
        team.plan.press_count = 3;
        assert_eq!(team.pressers(), 3);
        team.reshape(4);
        assert_eq!(team.pressers(), 2);
        team.reshape(5);
        team.reshape(6);
        assert_eq!(team.pressers(), 1, "never fewer than one");
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
