//! The game-state pass of the central loop: each side's rules evaluated on the match after
//! the tick's events, a rule change queued and applied when the set that holds moves, the
//! players taking a change up over time, and the run-of-play record the rules read.

use crate::contract::Stage;
use crate::modules::proposal::Proposal;
use crate::player::Status;
use crate::sim::Simulation;
use crate::tactics::change::{Change, ChangeId};
use crate::tactics::rules::{self, Situation};
use crate::team::PLAYERS_PER_TEAM;
use crate::{TICKS_PER_SECOND, ticks_for_minutes};

impl Simulation {
    /// `true` when a side has game-state rules: only then does the match keep the run-of-play
    /// record.
    pub(crate) fn rules_on(&self) -> bool {
        self.teams.iter().any(|t| !t.game_state.rules.is_empty())
    }

    /// Records a shot of `team` worth `xg` for the run of play, when a side has rules.
    pub(crate) fn record_chance(&mut self, team: usize, xg: f64) {
        if self.rules_on() {
            self.chances.push((self.tick + 1, team, xg));
        }
    }

    /// The match as side `team`'s rules read it on tick `now`.
    pub fn situation(&self, team: usize, now: u32) -> Situation {
        let other = 1 - team;
        let goals = self.summary.goals;
        let sent_off = |side: usize| {
            self.players
                .iter()
                .filter(|p| p.team == side && p.status == Status::SentOff)
                .count()
                .min(usize::from(u8::MAX)) as u8
        };
        let run_of_play = self
            .chances
            .iter()
            .filter(|(at, ..)| *at <= now)
            .map(|&(_, side, xg)| if side == team { xg } else { -xg })
            .sum();
        let keeper = self.keeper(team);
        let outfield: Vec<f64> = self
            .players
            .iter()
            .enumerate()
            .filter(|(i, p)| p.team == team && p.active() && *i != keeper)
            .map(|(_, p)| p.energy)
            .collect();
        let energy = if outfield.is_empty() {
            1.0
        } else {
            outfield.iter().sum::<f64>() / outfield.len() as f64
        };
        Situation {
            goal_difference: goals[team] as i32 - goals[other] as i32,
            minutes_left: self.minutes_left(now),
            sent_off: [sent_off(team), sent_off(other)],
            run_of_play,
            energy,
        }
    }

    /// Minutes left on tick `now` to the end of the regulation time of the match, or of extra
    /// time once it has started; 0 in added time.
    fn minutes_left(&self, now: u32) -> f64 {
        let c = &self.referee.clock;
        let length = |p: u32| c.period_ticks(p);
        let played: u32 = (0..c.half).map(length).sum::<u32>()
            + now.saturating_sub(c.half_start).min(length(c.half));
        let periods = if c.in_extra_time() {
            c.halves + c.extra_periods
        } else {
            c.halves
        };
        let end: u32 = (0..periods).map(length).sum();
        f64::from(end.saturating_sub(played)) / f64::from(ticks_for_minutes(1))
    }

    /// One tick of the game-state pass: each change being taken up moves on, the run-of-play
    /// record drops the shots older than its window, and each side whose rules that hold
    /// changed proposes a rule change.
    pub(crate) fn rules_tick(&mut self) {
        let now = self.tick + 1;
        for team in &mut self.teams {
            team.advance_take_up(now);
        }
        if !self.rules_on() {
            return;
        }
        let window = self
            .config
            .tactics
            .game_state
            .as_ref()
            .map_or(15, |g| g.run_of_play_window_min);
        let window = ticks_for_minutes(window);
        self.chances.retain(|&(at, ..)| at + window > now);
        for team in 0..2 {
            if self.teams[team].game_state.rules.is_empty() {
                continue;
            }
            let active = self.teams[team]
                .game_state
                .active(&self.situation(team, now));
            if active != self.teams[team].rules_active {
                self.apply_proposal(Proposal::QueueRuleChange { team, active });
            }
        }
    }

    /// Queues the rule change of `team` to the rules `active` and applies its part that
    /// applies at once: the tactics in force become the base with those rules applied, in the
    /// formation and roles in force, and each player takes them up after his delay. A new
    /// formation or role waits in the queue for a stoppage; with none the change applied whole.
    /// A rule change of the side still waiting is replaced.
    pub(crate) fn rule_change(&mut self, team: usize, active: u8) {
        let id = self.queue_change(team, Change::Rule { active });
        let before = self.queue.pending.len();
        self.queue
            .pending
            .retain(|q| q.team != team || q.id == id || !matches!(q.change, Change::Rule { .. }));
        let replaced = (before - self.queue.pending.len()) as u32;
        self.summary.changes_queued = self.summary.changes_queued.saturating_sub(replaced);
        let now = self.tick + 1;
        let schema = self.config.tactics.clone();
        let tuning = std::sync::Arc::clone(&self.tuning);
        let side = &self.teams[team];
        let target = rules::effective(&side.base, &side.game_state.rules, active, &schema);
        let at_once = rules::at_once(&target, &side.tactics);
        let changed = at_once != side.tactics;
        let waits = rules::shape_differs(&target, &at_once);
        self.teams[team].rules_active = active;
        if changed {
            let at = self.take_up_ticks(team, now);
            self.teams[team].begin_take_up(at_once, at, now, &schema, &tuning);
        }
        if !waits {
            self.rule_applied(team, id, now);
        }
    }

    /// The whole of `team`'s rule change at a stoppage that admits tactics: the tactics in
    /// force, formation and roles included, every player at once.
    pub(crate) fn rule_in_full(&mut self, team: usize) {
        let schema = self.config.tactics.clone();
        let tuning = std::sync::Arc::clone(&self.tuning);
        let side = &mut self.teams[team];
        let tactics = rules::effective(
            &side.base,
            &side.game_state.rules,
            side.rules_active,
            &schema,
        );
        side.set_tactics(tactics, &schema, &tuning);
    }

    /// The rule change `id` of `team` applied whole on tick `now`: it leaves the queue, and its
    /// verdict is an event like any applied change.
    fn rule_applied(&mut self, team: usize, id: ChangeId, now: u32) {
        let Some(k) = self
            .queue
            .pending
            .iter()
            .position(|q| q.team == team && q.id == id)
        else {
            return;
        };
        let q = self.queue.pending.remove(k);
        self.verdict(&q, now, None, None);
        self.timeline.push((now, self.teams.clone()));
    }

    /// The tick each player of `team` takes a change on tick `now` up on: after the tactics
    /// file's take-up time scaled by how far his shape stage (keeping to instructions) falls
    /// short of its top: at once at the top, the whole time at the bottom.
    fn take_up_ticks(&self, team: usize, now: u32) -> [u32; PLAYERS_PER_TEAM] {
        let take_up_s = self
            .config
            .tactics
            .game_state
            .as_ref()
            .map_or(0.0, |g| g.take_up_s);
        std::array::from_fn(|slot| {
            let i = team * PLAYERS_PER_TEAM + slot;
            let share = self.skills(i).share(Stage::SHAPE_CHOOSE);
            let delay = (take_up_s * (1.0 - share) * f64::from(TICKS_PER_SECOND)).round();
            now + delay.max(0.0) as u32
        })
    }
}
