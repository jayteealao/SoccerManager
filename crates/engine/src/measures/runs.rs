//! The off-ball runs of one match, by the definitions the real-team table shares
//! (`engine-cli signatures`: forwards' runs from tracking data, full-backs' runs from the
//! SkillCorner off-ball runs):
//!
//! - a run is a spell of [`RUN_MIN_S`] or more at [`RUN_SPEED`] or faster by a player of the
//!   side with the ball in play who is not the carrier;
//! - an attack-line run (a winger or a striker by the formation) is in behind or coming short
//!   by [`crate::run_kind::RunEnds`];
//! - a full-back's run (a left- or right-back) that starts behind the ball or level with it
//!   ([`LEVEL_M`]) and ends level with it or ahead, within [`NEAR_BALL_M`] of it, passes the
//!   ball: an overlap when it ends farther from the centre line than the ball, an underlap
//!   when it ends nearer.
//!
//! The sink reads the match after each step and writes nothing to it.

use crate::data::team::Position;
use crate::math::DVec2;
use crate::record::{TickRecord, TickSink};
use crate::run_kind::RunEnds;
pub use crate::run_kind::{RUN_MIN_S, RUN_SPEED};
use crate::sim::Simulation;
use crate::{EngineError, TICKS_PER_SECOND};

/// A full-back within this many metres of the ball's depth stands level with it.
pub const LEVEL_M: f64 = 2.0;
/// A full-back's run that passes the ball ends within this many metres of it.
pub const NEAR_BALL_M: f64 = 15.0;

/// One player's run in progress: the step it began at, and where it began.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Run {
    start: u32,
    /// How far ahead of the ball he stood (metres toward the goal his side attacks).
    ahead_m: f64,
    /// His depth, the second-last defender's and the ball's, and his distance to the ball.
    depth: f64,
    line: f64,
    ball: f64,
    ball_m: f64,
}

/// The runs of both sides, home first.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunsSink {
    /// Attack-line runs, and those in behind and those coming short among them.
    pub runs: [u32; 2],
    pub behind: [u32; 2],
    pub coming_short: [u32; 2],
    /// Full-backs' runs that pass the ball on the outside and on the inside.
    pub overlaps: [u32; 2],
    pub underlaps: [u32; 2],
    /// Each player's run in progress.
    running: Vec<Option<Run>>,
    last_live: u32,
    steps: u32,
}

impl RunsSink {
    /// The full-back runs of `team`: overlaps plus underlaps.
    pub fn fullback_runs(&self, team: usize) -> u32 {
        self.overlaps[team] + self.underlaps[team]
    }

    /// One step's runs of the side with the ball, `phase` (`None` when no run can go on).
    fn step(&mut self, sim: &Simulation, phase: Option<usize>) {
        let min_steps = (RUN_MIN_S * f64::from(TICKS_PER_SECOND)).round() as u32;
        let carrier = sim.carrier();
        let ball = sim.ball.xy();
        for (i, p) in sim.players().iter().enumerate() {
            let side = &sim.teams[p.team];
            let position = side.plan.slots[p.slot].position;
            let attack_line = crate::decision::attack_line(position);
            let fullback = matches!(position, Some(Position::LB | Position::RB));
            let running = phase == Some(p.team)
                && Some(i) != carrier
                && p.active()
                && (attack_line || fullback)
                && p.vel.length() >= RUN_SPEED;
            let ax = side.attack_x;
            let line = crate::rules::offside::second_last_depth(p.team, sim.players(), ax);
            let now = Run {
                start: self.steps,
                ahead_m: (p.pos.x - ball.x) * ax,
                depth: p.pos.x * ax,
                line,
                ball: ball.x * ax,
                ball_m: (p.pos - ball).length(),
            };
            match (running, self.running[i]) {
                (true, None) => self.running[i] = Some(now),
                (false, Some(run)) => {
                    self.running[i] = None;
                    if self.steps - run.start < min_steps {
                        continue;
                    }
                    let t = p.team;
                    if attack_line {
                        self.runs[t] += 1;
                        if !line.is_finite() || !run.line.is_finite() {
                            continue;
                        }
                        let ends = RunEnds {
                            depth: [run.depth, now.depth],
                            line: [run.line, now.line],
                            ball: [run.ball, now.ball],
                            ball_m: [run.ball_m, now.ball_m],
                        };
                        if ends.in_behind() {
                            self.behind[t] += 1;
                        }
                        if ends.coming_short() {
                            self.coming_short[t] += 1;
                        }
                    } else if passes_ball(run.ahead_m, now.ahead_m, p.pos, ball) {
                        if p.pos.y.abs() > ball.y.abs() {
                            self.overlaps[t] += 1;
                        } else {
                            self.underlaps[t] += 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// `true` when a full-back's run that began `from_m` metres ahead of the ball (below 0 behind
/// it) and ends `to_m` ahead of it, at `at`, with the ball at `ball`, passes the ball: it
/// begins behind or level and ends level or ahead, within [`NEAR_BALL_M`].
fn passes_ball(from_m: f64, to_m: f64, at: DVec2, ball: DVec2) -> bool {
    from_m <= LEVEL_M && to_m >= -LEVEL_M && to_m > from_m && (at - ball).length() <= NEAR_BALL_M
}

impl TickSink for RunsSink {
    fn on_tick(&mut self, _record: &TickRecord) -> Result<(), EngineError> {
        Ok(())
    }

    fn on_step(&mut self, sim: &Simulation) -> Result<(), EngineError> {
        if self.running.is_empty() {
            self.running = vec![None; sim.players().len()];
        }
        self.steps += 1;
        let live_ticks = sim.summary().live_ticks;
        let live = live_ticks > self.last_live;
        self.last_live = live_ticks;
        let phase = sim.possession_phase().filter(|_| live);
        self.step(sim, phase);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A full-back's run passes the ball when it starts behind or level and ends level or
    /// ahead, near the ball; a run that stays behind, starts ahead or ends far away does not.
    #[test]
    fn a_full_back_run_passes_the_ball_by_the_shared_definition() {
        let ball = DVec2::new(20.0, -15.0);
        assert!(passes_ball(-8.0, 3.0, DVec2::new(23.0, -25.0), ball));
        assert!(passes_ball(1.0, 1.5, DVec2::new(21.5, -10.0), ball));
        assert!(
            !passes_ball(-8.0, -4.0, DVec2::new(16.0, -25.0), ball),
            "behind"
        );
        assert!(
            !passes_ball(5.0, 9.0, DVec2::new(29.0, -25.0), ball),
            "ahead already"
        );
        assert!(
            !passes_ball(-8.0, 3.0, DVec2::new(23.0, 5.0), ball),
            "far from the ball"
        );
    }
}
