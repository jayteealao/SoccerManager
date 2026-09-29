//! The AI manager's pass of the central loop.

use serde_json::json;

use crate::TICKS_PER_SECOND;
use crate::ai::AiCode;
use crate::sim::{EngineEventKind, EventDetail, Simulation};
use crate::tactics::change::Change;
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

    /// One check of the AI manager for `team`: the manager module decides, then each change
    /// it proposes is queued and announced in order. `at_stoppage` is `true` for the check an
    /// injury or a goal asks for on its own stoppage.
    fn ai_check(&mut self, team: usize, at_stoppage: bool) {
        let plan = self
            .config
            .modules
            .manager
            .check(&self.view(), team, at_stoppage);
        self.ai[team] = plan.memory;
        for (change, code) in plan.changes {
            self.ai_queue(team, change, code, plan.minute);
        }
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
