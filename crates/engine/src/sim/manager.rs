//! The AI manager's pass of the central loop.

use serde_json::json;

use crate::TICKS_PER_SECOND;
use crate::ai::{AiCode, best_for};
use crate::data::rules::StoppageKind;
use crate::data::tactics::{PRESSING, TIME_WASTING};
use crate::data::team::Position;
use crate::player::Status;
use crate::sim::{EngineEventKind, EventDetail, Simulation};
use crate::tactics::change::Change;
use crate::tactics::{Tactics, TacticsPatch};
use crate::team::PLAYERS_PER_TEAM;
use crate::trace::Point;

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
                let at_stoppage = self.ai[team].due;
                self.ai[team].due = false;
                self.ai_check(team, at_stoppage);
            }
        }
    }

    /// One check of the AI manager for `team`. `at_stoppage` is `true` for the check an
    /// injury or a goal asks for on its own stoppage.
    fn ai_check(&mut self, team: usize, at_stoppage: bool) {
        let now = self.tick + 1;
        let minute = self.referee.clock.minute(now).0;
        let ai = self.config.tactics.ai.clone();
        let limit = usize::from(self.substitution_limits().0);
        let used = usize::from(self.ledgers[team].used);
        // Injuries: every injured player on the lineup without a substitute queued. The check
        // on the injury's own stoppage always asks, so a refusal names its reason there; a
        // later check asks again only while the team could still make a substitution.
        let may_substitute = at_stoppage || self.substitution_possible(team);
        for slot in 0..PLAYERS_PER_TEAM {
            let i = team * PLAYERS_PER_TEAM + slot;
            let off = self.teams[team].lineup[slot];
            if !may_substitute
                || self.players[i].status != Status::Injured
                || self.queue.has_substitution(team, off)
            {
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
        // The goal: an outfield player keeping goal makes way for the bench keeper, unless a
        // goalkeeper is already on the way (for an injured keeper, say).
        let keeper_slot = self.teams[team].keeper_slot();
        let side = &self.teams[team];
        let keeper_off = side.lineup[keeper_slot];
        let keeper_coming = self
            .queue
            .incoming(team)
            .iter()
            .any(|&s| side.squad[s].position == Position::GK);
        if side.squad[keeper_off].position != Position::GK
            && !keeper_coming
            && !self.queue.has_substitution(team, keeper_off)
            && self.substitution_possible(team)
            && let Some(on) = self.bench_keeper(team)
        {
            self.ai_queue(
                team,
                Change::Substitution {
                    off: keeper_off,
                    on,
                },
                AiCode::SubKeeper,
                minute,
            );
        }
        // Fatigue.
        let windows_left = self.ledgers[team].windows < self.substitution_limits().1;
        if minute >= ai.fatigue_from_minute && windows_left {
            let reserve = usize::from(minute < ai.keep_for_injury_until_minute);
            let mut tired: Vec<(f64, usize)> = (1..PLAYERS_PER_TEAM)
                .map(|slot| (slot, team * PLAYERS_PER_TEAM + slot))
                .filter(|&(slot, i)| {
                    let p = &self.players[i];
                    slot != keeper_slot
                        && p.active()
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

    /// `true` while a substitution for `team` could still apply at some later stoppage: the
    /// limit is not used up by substitutions made and waiting, and a window is left or a
    /// window-exempt stoppage (half-time in the shipped pack) is still to come.
    fn substitution_possible(&self, team: usize) -> bool {
        let rules = &self.config.rules.substitutions;
        let (limit, windows) = self.substitution_limits();
        let ledger = self.ledgers[team];
        let taken = usize::from(ledger.used) + self.queue.substitutions(team);
        if taken >= usize::from(limit) {
            return false;
        }
        let exempt_ahead = rules
            .windows_exempt
            .iter()
            .any(|&kind| kind != StoppageKind::HalfTime || !self.referee.clock.last_half());
        ledger.windows < windows || exempt_ahead
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

    /// The best goalkeeper on the bench by fit to the goalkeeper's role, leaving out anyone
    /// already queued to come on.
    fn bench_keeper(&self, team: usize) -> Option<usize> {
        let side = &self.teams[team];
        let incoming = self.queue.incoming(team);
        let keepers: Vec<usize> = side
            .bench
            .iter()
            .copied()
            .filter(|s| !incoming.contains(s) && side.squad[*s].position == Position::GK)
            .collect();
        let schema = &self.config.tactics;
        let role = schema.default_role(Position::GK);
        best_for(side, &keepers, role, schema, &self.config.attributes)
    }

    /// Queues an AI choice and announces it.
    fn ai_queue(&mut self, team: usize, change: Change, code: AiCode, minute: u32) {
        if self.trace_on() {
            let score = self.summary.goals;
            self.trace_point(
                Point::AiManager,
                json!({
                    "team": team,
                    "change": format!("{change:?}"),
                    "code": code.code(),
                    "minute": minute,
                    "score": [score[team], score[1 - team]],
                }),
            );
        }
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
