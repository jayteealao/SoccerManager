//! The off-ball runs of one match, by the definitions the real-team table shares
//! (`engine-cli signatures`):
//!
//! - an attack-line run (a winger or a striker by the formation) is a spell of
//!   [`RUN_MIN_S`] or more at [`RUN_SPEED`] or faster while his side has the ball in play and
//!   he is not the carrier; it is in behind when it ends within [`BEHIND_M`] of the
//!   second-last defender or beyond him;
//! - a full-back run is an entry of a left- or right-back into the overlap or the underlap
//!   stand option while his side has the ball in play.
//!
//! The sink reads the match after each step and writes nothing to it; it recomputes the shape
//! pass of the side in possession to see which option each full-back takes.

use crate::data::tactics::StandOption;
use crate::data::team::Position;
use crate::math::DVec2;
use crate::modules::ROSTER;
use crate::record::{TickRecord, TickSink};
use crate::sim::Simulation;
use crate::{EngineError, TICKS_PER_SECOND};

/// The speed of a run, in metres per second.
pub const RUN_SPEED: f64 = 4.0;
/// The shortest run, in seconds.
pub const RUN_MIN_S: f64 = 1.0;
/// A run that ends within this many metres of the second-last defender, or beyond him, is in
/// behind.
pub const BEHIND_M: f64 = 2.0;

/// The runs of both sides, home first.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunsSink {
    /// Attack-line runs, and those in behind among them.
    pub runs: [u32; 2],
    pub behind: [u32; 2],
    /// Full-back entries into the overlap and the underlap.
    pub overlaps: [u32; 2],
    pub underlaps: [u32; 2],
    /// The step each player's run began at, while he runs.
    started: Vec<Option<u32>>,
    /// The option each player took at the last step.
    option: Vec<Option<StandOption>>,
    last_live: u32,
    steps: u32,
}

impl RunsSink {
    /// The full-back runs of `team`: overlaps plus underlaps.
    pub fn fullback_runs(&self, team: usize) -> u32 {
        self.overlaps[team] + self.underlaps[team]
    }

    /// One step's runs of the attack line of `phase`, the side with the ball (`None` when no
    /// run can go on).
    fn attack_line(&mut self, sim: &Simulation, phase: Option<usize>) {
        let min_steps = (RUN_MIN_S * f64::from(TICKS_PER_SECOND)).round() as u32;
        let carrier = sim.carrier();
        for (i, p) in sim.players().iter().enumerate() {
            let side = &sim.teams[p.team];
            let running = phase == Some(p.team)
                && Some(i) != carrier
                && p.active()
                && crate::decision::attack_line(side.plan.slots[p.slot].position)
                && p.vel.length() >= RUN_SPEED;
            match (running, self.started[i]) {
                (true, None) => self.started[i] = Some(self.steps),
                (false, Some(start)) => {
                    self.started[i] = None;
                    if self.steps - start >= min_steps {
                        let t = p.team;
                        self.runs[t] += 1;
                        let line = crate::rules::offside::second_last_depth(
                            t,
                            sim.players(),
                            side.attack_x,
                        );
                        if line.is_finite() && p.pos.x * side.attack_x >= line - BEHIND_M {
                            self.behind[t] += 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// One step's full-back options of `phase`, the side with the ball.
    fn full_backs(&mut self, sim: &Simulation, phase: Option<usize>) {
        let mut now = [None; ROSTER];
        if let Some(team) = phase {
            let view = sim.view();
            let anchors = crate::shape::anchors(&view, phase);
            let mut targets = [DVec2::ZERO; ROSTER];
            for (t, p) in targets.iter_mut().zip(sim.players()) {
                *t = p.target;
            }
            let out = crate::shape::team_shape(&view, team, true, &anchors, &mut targets);
            let first = team * crate::team::PLAYERS_PER_TEAM;
            let side = &sim.teams[team];
            for (slot, option) in out.option.iter().enumerate() {
                let fullback = matches!(
                    side.plan.slots[slot].position,
                    Some(Position::LB | Position::RB)
                );
                if fullback {
                    now[first + slot] = *option;
                }
            }
        }
        for (i, &o) in now.iter().enumerate() {
            if o != self.option[i] {
                let team = i / crate::team::PLAYERS_PER_TEAM;
                match o {
                    Some(StandOption::Overlap) => self.overlaps[team] += 1,
                    Some(StandOption::Underlap) => self.underlaps[team] += 1,
                    _ => {}
                }
            }
        }
        self.option = now.to_vec();
    }
}

impl TickSink for RunsSink {
    fn on_tick(&mut self, _record: &TickRecord) -> Result<(), EngineError> {
        Ok(())
    }

    fn on_step(&mut self, sim: &Simulation) -> Result<(), EngineError> {
        if self.started.is_empty() {
            self.started = vec![None; sim.players().len()];
            self.option = vec![None; ROSTER];
        }
        self.steps += 1;
        let live_ticks = sim.summary().live_ticks;
        let live = live_ticks > self.last_live;
        self.last_live = live_ticks;
        let phase = sim.possession_phase().filter(|_| live);
        self.attack_line(sim, phase);
        self.full_backs(sim, phase);
        Ok(())
    }
}
