//! The AI manager (named mechanism). Before kick-off it picks a lineup, a bench, and the
//! tactics file's default tactics. During the match it checks every `check_interval_s`
//! simulated seconds and at once after an injury:
//!
//! - an injured player is replaced by the best-fitting substitute;
//! - from `fatigue_from_minute`, a tired outfield player (energy below `fatigue_energy`) is
//!   replaced, keeping one substitution back for an injury until
//!   `keep_for_injury_until_minute`;
//! - trailing from `trailing_minute`, the mentality rises one step and pressing goes high,
//!   once per score;
//! - leading from `leading_minute`, the mentality drops one step and time wasting goes on,
//!   once per score.
//!
//! It draws no random numbers, so a match with the AI draws the same generator stream as
//! the rules alone would. Every choice goes through the same change queue a human manager
//! uses, and emits an `AiDecision` event with a short code.

use crate::TICKS_PER_SECOND;
use crate::data::attributes::AttributeSchema;
use crate::data::tactics::{PRESSING, TIME_WASTING, TacticsSchema};
use crate::data::team::Position;
use crate::player::Status;
use crate::sim::{EngineEventKind, EventDetail, Simulation};
use crate::tactics::change::Change;
use crate::tactics::{Tactics, TacticsPatch};
use crate::team::{PLAYERS_PER_TEAM, SquadPlayer, Team};

/// Who manages a team during the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    /// The AI manager: the pre-match setup and every in-match decision.
    Ai,
    /// A person: the AI's pre-match setup until a lineup message exists, and no in-match
    /// AI decisions.
    Human,
}

/// What an AI choice was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCode {
    MentalityUpTrailing,
    MentalityDownLeading,
    SubInjury,
    SubFatigue,
}

impl AiCode {
    /// The `ai.decision` value.
    pub fn code(&self) -> &'static str {
        match self {
            AiCode::MentalityUpTrailing => "mentality-up-trailing",
            AiCode::MentalityDownLeading => "mentality-down-leading",
            AiCode::SubInjury => "sub-injury",
            AiCode::SubFatigue => "sub-fatigue",
        }
    }
}

/// The AI manager's memory: the score it last acted on while trailing and while leading.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AiState {
    pub trailing_acted: Option<[u32; 2]>,
    pub leading_acted: Option<[u32; 2]>,
    /// An injury asks for a check on this tick.
    pub due: bool,
}

/// The pre-match setup: tactics, the squad index of each slot, and the bench.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    pub tactics: Tactics,
    pub lineup: [usize; PLAYERS_PER_TEAM],
    pub bench: Vec<usize>,
}

/// How well `p` fits role `role`: the weighted mean of the role's attributes.
pub fn role_fit(
    p: &SquadPlayer,
    role: usize,
    schema: &TacticsSchema,
    attrs: &AttributeSchema,
) -> f64 {
    let mut sum = 0.0;
    let mut weight = 0.0;
    for (name, w) in &schema.roles[role].attributes {
        if let Some(i) = attrs.index(name) {
            sum += w * f64::from(p.attributes.get(i));
            weight += w;
        }
    }
    if weight > 0.0 { sum / weight } else { 0.0 }
}

/// The best player in `candidates` for `role`: players whose position the role suits come
/// first, then any outfield player (or any player for a goalkeeper's role). Ties go to the
/// lower squad index.
fn best_for(
    team: &Team,
    candidates: &[usize],
    role: usize,
    schema: &TacticsSchema,
    attrs: &AttributeSchema,
) -> Option<usize> {
    let suits = &schema.roles[role].positions;
    let keeper_role = suits.contains(&Position::GK);
    let pick = |filter: &dyn Fn(Position) -> bool| {
        candidates
            .iter()
            .copied()
            .filter(|&s| filter(team.squad[s].position))
            .map(|s| (role_fit(&team.squad[s], role, schema, attrs), s))
            .fold(None::<(f64, usize)>, |best, (fit, s)| match best {
                Some((b, _)) if b >= fit => best,
                _ => Some((fit, s)),
            })
            .map(|(_, s)| s)
    };
    pick(&|p| suits.contains(&p))
        .or_else(|| pick(&|p| keeper_role || p != Position::GK))
        .or_else(|| pick(&|_| true))
}

/// The AI manager's pre-match setup for `team`: the tactics file's default tactics, the
/// best-fitting player for each slot in slot order, and a bench of `bench_size` with the best
/// remaining goalkeeper first and then the best remaining players by fit to their own
/// position's first role.
pub fn pre_match(team: &Team, schema: &TacticsSchema, attrs: &AttributeSchema) -> Setup {
    let tactics = Tactics::defaults(schema);
    let mut free: Vec<usize> = (0..team.squad.len()).collect();
    let mut lineup = [0usize; PLAYERS_PER_TEAM];
    for (slot, place) in lineup.iter_mut().enumerate() {
        let role = usize::from(tactics.roles[slot].role);
        // A validated squad holds at least eleven players.
        let pick = best_for(team, &free, role, schema, attrs).unwrap_or(free[0]);
        *place = pick;
        free.retain(|&s| s != pick);
    }
    let mut bench = Vec::new();
    let size = usize::from(schema.ai.bench_size);
    let keeper_role = schema.default_role(Position::GK);
    let keepers: Vec<usize> = free
        .iter()
        .copied()
        .filter(|&s| team.squad[s].position == Position::GK)
        .collect();
    if size > 0
        && let Some(gk) = best_for(team, &keepers, keeper_role, schema, attrs)
    {
        bench.push(gk);
        free.retain(|&s| s != gk);
    }
    let mut rest: Vec<(f64, usize)> = free
        .iter()
        .map(|&s| {
            let role = schema.default_role(team.squad[s].position);
            (role_fit(&team.squad[s], role, schema, attrs), s)
        })
        .collect();
    rest.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    bench.extend(
        rest.into_iter()
            .map(|(_, s)| s)
            .take(size.saturating_sub(bench.len())),
    );
    Setup {
        tactics,
        lineup,
        bench,
    }
}

impl Simulation {
    /// Runs the AI manager for every AI-managed team whose check is due on this tick.
    pub(crate) fn ai_tick(&mut self) {
        let now = self.tick + 1;
        let interval = self.config.tactics.ai.check_interval_s * TICKS_PER_SECOND;
        for team in 0..2 {
            if self.managers[team] != crate::ai::Manager::Ai {
                self.ai[team].due = false;
                continue;
            }
            if self.ai[team].due || now.is_multiple_of(interval) {
                self.ai[team].due = false;
                self.ai_check(team);
            }
        }
    }

    /// One check of the AI manager for `team`.
    fn ai_check(&mut self, team: usize) {
        let now = self.tick + 1;
        let minute = self.referee.clock.minute(now).0;
        let ai = self.config.tactics.ai.clone();
        let limit = usize::from(self.config.rules.substitutions.limit);
        let used = usize::from(self.ledgers[team].used);
        // Injuries: every injured player on the lineup without a substitute queued.
        for slot in 0..PLAYERS_PER_TEAM {
            let i = team * PLAYERS_PER_TEAM + slot;
            let off = self.teams[team].lineup[slot];
            if self.players[i].status != Status::Injured || self.queue.has_substitution(team, off) {
                continue;
            }
            if let Some(on) = self.substitute_for(team, slot) {
                self.ai_queue(
                    team,
                    Change::Substitution { off, on },
                    AiCode::SubInjury,
                    minute,
                );
            }
        }
        // Fatigue.
        let windows_left = self.ledgers[team].windows < self.config.rules.substitutions.windows;
        if minute >= ai.fatigue_from_minute && windows_left {
            let reserve = usize::from(minute < ai.keep_for_injury_until_minute);
            let mut tired: Vec<(f64, usize)> = (1..PLAYERS_PER_TEAM)
                .map(|slot| (slot, team * PLAYERS_PER_TEAM + slot))
                .filter(|&(slot, i)| {
                    let p = &self.players[i];
                    p.active()
                        && p.energy < ai.fatigue_energy
                        && !self
                            .queue
                            .has_substitution(team, self.teams[team].lineup[slot])
                })
                .map(|(slot, i)| (self.players[i].energy, slot))
                .collect();
            tired.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            for (_, slot) in tired {
                let left = limit.saturating_sub(used + self.queue.substitutions(team));
                if left <= reserve {
                    break;
                }
                let off = self.teams[team].lineup[slot];
                if let Some(on) = self.substitute_for(team, slot) {
                    self.ai_queue(
                        team,
                        Change::Substitution { off, on },
                        AiCode::SubFatigue,
                        minute,
                    );
                }
            }
        }
        // The score.
        let score = self.summary.goals;
        let (mine, theirs) = (score[team], score[1 - team]);
        let projected = self.projected_tactics(team);
        let schema = &self.config.tactics;
        if mine < theirs
            && minute >= ai.trailing_minute
            && self.ai[team].trailing_acted != Some(score)
        {
            self.ai[team].trailing_acted = Some(score);
            let mut patch = TacticsPatch::default();
            if usize::from(projected.mentality) + 1 < schema.mentalities.len() {
                patch.mentality = Some(projected.mentality + 1);
            }
            let high = (schema.instructions.level_count(PRESSING) - 1) as u8;
            if projected.instructions[PRESSING] != high {
                patch.instructions[PRESSING] = Some(high);
            }
            if patch != TacticsPatch::default() {
                self.ai_queue(
                    team,
                    Change::Tactics(patch),
                    AiCode::MentalityUpTrailing,
                    minute,
                );
            }
        } else if mine > theirs
            && minute >= ai.leading_minute
            && self.ai[team].leading_acted != Some(score)
        {
            self.ai[team].leading_acted = Some(score);
            let mut patch = TacticsPatch::default();
            if projected.mentality > 0 {
                patch.mentality = Some(projected.mentality - 1);
            }
            let on = (schema.instructions.level_count(TIME_WASTING) - 1) as u8;
            if projected.instructions[TIME_WASTING] != on {
                patch.instructions[TIME_WASTING] = Some(on);
            }
            if patch != TacticsPatch::default() {
                self.ai_queue(
                    team,
                    Change::Tactics(patch),
                    AiCode::MentalityDownLeading,
                    minute,
                );
            }
        }
    }

    /// `team`'s tactics once every waiting tactics change has applied.
    fn projected_tactics(&self, team: usize) -> Tactics {
        let mut t = self.teams[team].tactics;
        for q in self.queue.pending.iter().filter(|q| q.team == team) {
            if let Change::Tactics(p) = &q.change {
                if let Some(m) = p.mentality {
                    t.mentality = m;
                }
                for (level, set) in t.instructions.iter_mut().zip(p.instructions) {
                    if let Some(l) = set {
                        *level = l;
                    }
                }
            }
        }
        t
    }

    /// The best substitute on the bench for `slot`'s role, leaving out anyone already queued
    /// to come on.
    fn substitute_for(&self, team: usize, slot: usize) -> Option<usize> {
        let side = &self.teams[team];
        let incoming = self.queue.incoming(team);
        let free: Vec<usize> = side
            .bench
            .iter()
            .copied()
            .filter(|s| !incoming.contains(s))
            .collect();
        let role = usize::from(side.tactics.roles[slot].role);
        best_for(
            side,
            &free,
            role,
            &self.config.tactics,
            &self.config.attributes,
        )
    }

    /// Queues an AI choice and announces it.
    fn ai_queue(&mut self, team: usize, change: Change, code: AiCode, minute: u32) {
        self.queue_change(team, change);
        self.summary.ai_decisions += 1;
        let mut event = self.event(EngineEventKind::AiDecision, Some(team));
        event.detail = Some(EventDetail::Ai { code });
        self.events.push(event);
        let score = self.summary.goals;
        tracing::info!(
            signal = "ai.decision",
            tick = self.tick + 1,
            minute,
            team,
            code = code.code(),
            score_state = %format!("{}-{}", score[team], score[1 - team])
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::{default_teams, shipped_content};

    #[test]
    fn the_pre_match_lineup_puts_a_goalkeeper_in_goal_and_one_on_the_bench() {
        let content = shipped_content();
        let [a, _] = default_teams(&content);
        let (team, _) =
            Team::from_file(0, &a, &content.attributes, &content.tuning.engine).unwrap();
        let setup = pre_match(&team, &content.tactics, &content.attributes);
        assert_eq!(team.squad[setup.lineup[0]].position, Position::GK);
        assert_eq!(setup.bench.len(), 7);
        assert_eq!(team.squad[setup.bench[0]].position, Position::GK);
        let mut all: Vec<usize> = setup.lineup.to_vec();
        all.extend(&setup.bench);
        let n = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), n, "a player is named twice");
        for slot in 1..PLAYERS_PER_TEAM {
            assert_ne!(team.squad[setup.lineup[slot]].position, Position::GK);
        }
    }

    #[test]
    fn the_pre_match_setup_is_the_same_every_time() {
        let content = shipped_content();
        let [a, _] = default_teams(&content);
        let (team, _) =
            Team::from_file(0, &a, &content.attributes, &content.tuning.engine).unwrap();
        let one = pre_match(&team, &content.tactics, &content.attributes);
        let two = pre_match(&team, &content.tactics, &content.attributes);
        assert_eq!(one, two);
    }
}
