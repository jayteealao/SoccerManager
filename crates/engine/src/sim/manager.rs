//! The AI manager's pass of the central loop.

use serde_json::json;

use crate::ai::AiCode;
use crate::sim::{EngineEventKind, EventDetail, Simulation};
use crate::tactics::change::Change;
use crate::trace::Point;

impl Simulation {
    /// Runs the AI manager for every AI-managed team whose check is due on this tick: on the
    /// tick a stoppage that admits a substitution opens, and after an injury, a goal or a
    /// sending-off. No check runs on a timer.
    pub(crate) fn ai_tick(&mut self) {
        let admitted = self.view().substitutions_admitted();
        for team in 0..2 {
            if self.managers[team] != crate::ai::Manager::Ai {
                self.ai[team].due = false;
                continue;
            }
            if self.ai[team].due || admitted {
                self.ai[team].due = false;
                self.ai_check(team, true);
            }
        }
    }

    /// One check of the AI manager for `team`: the manager module decides, then each change
    /// it proposes is queued and announced in order.
    fn ai_check(&mut self, team: usize, at_stoppage: bool) {
        let plan = self
            .config
            .modules
            .manager
            .check(&self.view(), team, at_stoppage);
        let minute = self.referee.clock.minute(self.tick + 1).0;
        for (change, code) in plan.changes {
            self.ai_queue(team, change, code, minute);
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
