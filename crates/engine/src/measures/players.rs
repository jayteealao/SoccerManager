//! Each player's own counts of the signature statistics a player instruction moves, by roster
//! index (side times 11 plus formation slot; a substitute takes his slot's index), counted at
//! the same sites as the side's counts in [`crate::measures`]:
//!
//! - take-ons, by the dribbler;
//! - crosses, high crosses and the sum of the crosses' distances from the goal line, by the
//!   crosser;
//! - open-play shots and those from outside the penalty area, by the shooter;
//! - pressures, by the presser: his first press of a carrier in that carrier's spell;
//! - tackles, by the tackler: each tackle he chooses to make at the tackle decision (the
//!   take-on duels a dribbler draws him into are not his choice and do not count);
//! - the open-play passes a player kicks within [`crate::decision::COUNTER_WINDOW_S`] of his
//!   side's open-play regain, and the forward ones among them;
//! - his pressures within [`crate::decision::COUNTERPRESS_S`] of an open-play loss of his
//!   side still in progress (a loss a short regain later cancels counts too, in both arms of
//!   a pair alike).
//!
//! [`PlayersSink`] adds what a step shows: each player's distance from the centre line while
//! his side has the ball in play, and his attack-line runs and the runs in behind among them
//! by the definitions of [`crate::measures::runs`].

use crate::record::{TickRecord, TickSink};
use crate::run_kind::RunEnds;
pub use crate::run_kind::{RUN_MIN_S, RUN_SPEED};
use crate::sim::Simulation;
use crate::{EngineError, TICKS_PER_SECOND};

/// Players on the roster of a match: two sides of 11.
pub const ROSTER: usize = 2 * crate::team::PLAYERS_PER_TEAM;

/// One player's counts at the measure points.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerCounts {
    pub takeons: u32,
    pub crosses: u32,
    pub high_crosses: u32,
    pub cross_origin_m: f64,
    pub open_shots: u32,
    pub outside_shots: u32,
    pub pressures: u32,
    pub tackles: u32,
    /// Open-play passes kicked inside the regain window, and the forward ones.
    pub window_passes: u32,
    pub window_forward: u32,
    /// Pressures inside the counter-press window of a loss of his side.
    pub counterpress_pressures: u32,
}

/// What a step shows of each player: in-possession width and off-ball runs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayersSink {
    /// Per roster index, the sum of his distance from the centre line over the steps his side
    /// had the ball in play while he was on the pitch, and those steps.
    pub width_m: Vec<f64>,
    pub width_steps: Vec<u32>,
    /// Per roster index, his attack-line runs and the runs in behind among them.
    pub runs: Vec<u32>,
    pub behind: Vec<u32>,
    running: Vec<Option<Run>>,
    last_live: u32,
    steps: u32,
}

/// A run in progress: the step it began at and its ends' depths.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Run {
    start: u32,
    depth: f64,
    line: f64,
    ball: f64,
    ball_m: f64,
}

impl PlayersSink {
    fn step(&mut self, sim: &Simulation, phase: Option<usize>) {
        let min_steps = (RUN_MIN_S * f64::from(TICKS_PER_SECOND)).round() as u32;
        let carrier = sim.carrier();
        let ball = sim.ball.xy();
        for (i, p) in sim.players().iter().enumerate() {
            let on_ball = phase == Some(p.team) && p.active();
            if on_ball {
                self.width_m[i] += p.pos.y.abs();
                self.width_steps[i] += 1;
            }
            let side = &sim.teams[p.team];
            let attack_line = crate::decision::attack_line(side.plan.slots[p.slot].position);
            let running =
                on_ball && Some(i) != carrier && attack_line && p.vel.length() >= RUN_SPEED;
            let ax = side.attack_x;
            let line = crate::rules::offside::second_last_depth(p.team, sim.players(), ax);
            let now = Run {
                start: self.steps,
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
                    self.runs[i] += 1;
                    if !line.is_finite() || !run.line.is_finite() {
                        continue;
                    }
                    let ends = RunEnds {
                        depth: [run.depth, now.depth],
                        line: [run.line, now.line],
                        ball: [run.ball, now.ball],
                        ball_m: [run.ball_m, now.ball_m],
                    };
                    self.behind[i] += u32::from(ends.in_behind());
                }
                _ => {}
            }
        }
    }
}

impl TickSink for PlayersSink {
    fn on_tick(&mut self, _record: &TickRecord) -> Result<(), EngineError> {
        Ok(())
    }

    fn reads_records(&self) -> bool {
        false
    }

    fn on_step(&mut self, sim: &Simulation) -> Result<(), EngineError> {
        if self.running.is_empty() {
            let n = sim.players().len();
            self.running = vec![None; n];
            self.width_m = vec![0.0; n];
            self.width_steps = vec![0; n];
            self.runs = vec![0; n];
            self.behind = vec![0; n];
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

/// The roster indices set in `bits`.
pub(crate) fn roster(bits: u32) -> impl Iterator<Item = usize> {
    (0..ROSTER).filter(move |i| bits & (1 << i) != 0)
}
