//! The pace pass of the central loop: each player's job becomes a grade and a pace level, and
//! the level's speed is what steering arrives at ([`crate::pace`]). The pass runs in the live
//! phase after the decision pass; at a dead ball every player's pace is the off state.

use crate::pace::{self, Floor, Grade, Pace, TeamPace};
use crate::rules::Phase;
use crate::sim::Simulation;
use crate::streams::{Action, Key, PlayerKey};

impl Simulation {
    /// One tick of the pace selector for every player on the pitch. An URGENT job draws its
    /// sprint once, when it starts; a job that is not URGENT keeps no sprint.
    pub(crate) fn pace_tick(&mut self) {
        let t = &self.config.tuning.pace;
        if !t.enabled {
            return;
        }
        let tick = self.tick;
        let goals = self.summary.goals;
        let minute = self.referee.clock.minute(tick).0;
        let late = minute + t.late_minutes >= self.config.rules.regulation_minutes();
        let dead = self.referee.phase != Phase::Live;
        let possession = self.possession_phase;
        let ctx: [TeamPace; 2] = std::array::from_fn(|team| {
            let plan = &self.teams[team].plan;
            let (mine, theirs) = (goals[team], goals[1 - team]);
            // A team wastes time while the ball is dead or while it has the ball; out of
            // possession its players keep their grades, so nobody walks back while the
            // opponent attacks.
            let wasting = plan.time_wasting > 1.0 && (dead || possession == Some(team));
            TeamPace {
                lift: plan.pace_lift,
                trailing_late: late && mine < theirs,
                wasting_late: late && mine > theirs && wasting,
            }
        });
        for i in 0..self.players.len() {
            let p = &self.players[i];
            if !p.active() {
                continue;
            }
            let t = &self.config.tuning.pace;
            let job = p.pace.job;
            let grade = pace::adjust(pace::base_grade(job), job, ctx[p.team], Floor::Keep);
            let starts =
                grade == Grade::Urgent && (p.pace.since == tick || p.pace.grade != Grade::Urgent);
            let sprint = if starts {
                let chance = pace::sprint_chance(p.derived.work_share, t);
                let key = Key::player(Action::PaceSprint, PlayerKey::of(p));
                self.streams.chance(key, chance)
            } else {
                grade == Grade::Urgent && p.pace.sprint
            };
            let p = &self.players[i];
            let t = &self.config.tuning.pace;
            let saving = pace::saving(p.energy, t);
            let level = pace::level(
                grade,
                job,
                (p.target - p.pos).length(),
                sprint,
                saving,
                p.derived.tired_mix,
                t,
            );
            let speed = pace::speed(level, p.max_speed(), t);
            let since = p.pace.since;
            self.players[i].pace = Pace {
                job,
                grade,
                level,
                speed,
                since,
                sprint,
            };
        }
    }

    /// At a dead ball every player's pace is the off state: he runs at his top speed, as before
    /// the selector, until the restart.
    pub(crate) fn pace_off(&mut self) {
        for p in &mut self.players {
            p.pace = Pace::OFF;
        }
    }
}
