//! The engine's feed into the shared shape report ([`crate::report::tracked_shape`]): a tick
//! sink that turns every second tick record of a calibration match, counted from its half's
//! first record, into a report frame, with what the match knows that a record does not: each
//! record place's player and position group, whether the ball is in play, the side in
//! possession, the restarts since the last frame and each side's attack direction. A real
//! tracked match converted into the tick format reaches the same report through its side
//! file, so both are measured by one code. Only a `measures` build has it.

use engine::data::team::Position;
use engine::{EngineError, Simulation, StoppageKind, TickRecord, TickSink};

use crate::report::tracked_shape::{Frame, Holder, Mode, Restart, TrackedShape, TrackedValues};
use crate::tracking::Group;

const ROSTER: usize = 2 * engine::team::PLAYERS_PER_TEAM;

/// The position group of a slot's position.
pub fn group_of(p: Position) -> Group {
    match p {
        Position::GK => Group::Keeper,
        Position::CB => Group::Cb,
        Position::LB | Position::RB => Group::Fb,
        Position::DM | Position::CM | Position::AM => Group::Cm,
        Position::LW | Position::RW => Group::W,
        Position::ST => Group::St,
    }
}

/// The restart a stoppage of `kind` counts as: a goal's kick-off is a kick-off; half time
/// and an injury restart nothing the report counts.
pub fn restart_of(kind: StoppageKind) -> Option<Restart> {
    Some(match kind {
        StoppageKind::ThrowIn => Restart::ThrowIn,
        StoppageKind::FreeKick => Restart::FreeKick,
        StoppageKind::GoalKick => Restart::GoalKick,
        StoppageKind::Corner => Restart::Corner,
        StoppageKind::Goal | StoppageKind::KickOff => Restart::KickOff,
        StoppageKind::Penalty => Restart::Penalty,
        StoppageKind::HalfTime | StoppageKind::Injury => return None,
    })
}

/// What a step of the match adds to its record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepState {
    pub half: usize,
    pub on: [Option<Holder>; ROSTER],
    pub live: bool,
    pub possession: Option<usize>,
    pub attack_x: [f64; 2],
}

impl StepState {
    /// The state of `sim` after a step; `live` when the ball was in play in it; `attack_x`
    /// each side's attack direction this half.
    pub fn of(sim: &Simulation, live: bool, attack_x: [f64; 2]) -> Self {
        let mut on = [None; ROSTER];
        let keepers = [sim.keeper(0), sim.keeper(1)];
        for (i, p) in sim.players().iter().enumerate().take(ROSTER) {
            if !p.active() {
                continue;
            }
            let team = p.team;
            let group = if keepers[team] == i {
                Group::Keeper
            } else {
                group_of(sim.slot_position(team, p.slot))
            };
            on[i] = Some(Holder {
                person: u32::try_from(team * 100 + p.squad).unwrap_or(u32::MAX),
                group,
            });
        }
        Self {
            half: usize::try_from(sim.half()).unwrap_or(usize::MAX),
            on,
            live,
            possession: sim.possession_phase(),
            attack_x,
        }
    }
}

/// Builds the report frames of a match from its records and step states: every second
/// record from each half's first, with the restarts counted since the frame before.
#[derive(Debug, Clone, Default)]
pub struct Framer {
    /// The half followed and its first record's tick.
    half: Option<(usize, u32)>,
    restarts: Vec<(Restart, usize)>,
}

impl Framer {
    /// A restart taken by `team`, counted on the next frame.
    pub fn restart(&mut self, r: Restart, team: usize) {
        self.restarts.push((r, team));
    }

    /// The frame of `record`, when it falls on an even step from its half's first record.
    pub fn frame(&mut self, record: &TickRecord, state: &StepState) -> Option<Frame> {
        let start = match self.half {
            Some((h, start)) if h == state.half => start,
            _ => {
                self.half = Some((state.half, record.tick));
                record.tick
            }
        };
        if !(record.tick - start).is_multiple_of(2) {
            return None;
        }
        Some(Frame {
            tick: record.tick,
            half: state.half,
            ball: Some(record.ball),
            players: record.players,
            on: state.on,
            live: Some(state.live),
            possession: state.possession.filter(|_| state.live),
            restarts: std::mem::take(&mut self.restarts),
            attack_x: state.attack_x,
        })
    }
}

/// The feed: a tick sink that measures a match with the shared report as it plays.
#[derive(Debug, Clone)]
pub struct TrackedFeed {
    report: TrackedShape,
    framer: Framer,
    state: Option<StepState>,
    last_live: u32,
    /// The half whose attack directions are known, and the directions.
    directions: Option<(u32, [f64; 2])>,
    error: Option<String>,
    /// Every frame fed, when kept (the same-code test).
    pub kept: Option<Vec<Frame>>,
}

impl TrackedFeed {
    /// A feed for a match on a pitch `length` metres long.
    pub fn new(length: f64) -> Self {
        Self {
            report: TrackedShape::new(Mode::Full, length),
            framer: Framer::default(),
            state: None,
            last_live: 0,
            directions: None,
            error: None,
            kept: None,
        }
    }

    /// The same feed keeping every frame it feeds.
    pub fn keeping(mut self) -> Self {
        self.kept = Some(Vec::new());
        self
    }

    /// Every figure of the match; an error when the report refused a frame.
    pub fn finish(self) -> Result<TrackedValues, String> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(self.report.finish()),
        }
    }
}

impl TickSink for TrackedFeed {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        let Some(state) = self.state.as_ref() else {
            return Ok(());
        };
        if let Some(frame) = self.framer.frame(record, state) {
            if self.error.is_none()
                && let Err(e) = self.report.frame(&frame)
            {
                self.error = Some(e);
            }
            if let Some(kept) = &mut self.kept {
                kept.push(frame);
            }
        }
        Ok(())
    }

    fn on_step(&mut self, sim: &Simulation) -> Result<(), EngineError> {
        let live_ticks = sim.summary().live_ticks;
        let live = live_ticks > self.last_live;
        self.last_live = live_ticks;
        // The sides change ends only between halves: their directions are read once a half.
        let half = sim.half();
        let attack_x = match self.directions {
            Some((h, a)) if h == half => a,
            _ => {
                let teams = sim.teams();
                let a = [teams[0].attack_x, teams[1].attack_x];
                self.directions = Some((half, a));
                a
            }
        };
        self.state = Some(StepState::of(sim, live, attack_x));
        Ok(())
    }

    fn on_stoppage(
        &mut self,
        stoppage: &engine::Stoppage,
        _sim: &Simulation,
    ) -> Result<(), EngineError> {
        if let (Some(r), Some(team)) = (restart_of(stoppage.kind), stoppage.team) {
            self.framer.restart(r, team);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(tick: u32) -> TickRecord {
        TickRecord {
            tick,
            ball: [1.0, 2.0, 0.0],
            players: [[0.0; 2]; ROSTER],
            restart: false,
        }
    }

    fn state(half: usize, live: bool) -> StepState {
        StepState {
            half,
            on: [None; ROSTER],
            live,
            possession: Some(1),
            attack_x: [1.0, -1.0],
        }
    }

    /// Every second record from each half's first is a frame, whatever tick the half starts
    /// on; a restart goes on the next frame; possession counts only while the ball is live.
    #[test]
    fn every_second_record_of_each_half_is_a_frame() {
        let mut f = Framer::default();
        let mut ticks = Vec::new();
        for t in 3..9 {
            if t == 4 {
                f.restart(Restart::Corner, 0);
            }
            if let Some(fr) = f.frame(&record(t), &state(0, t != 5)) {
                ticks.push((fr.tick, fr.restarts.len(), fr.possession));
            }
        }
        for t in 10..14 {
            if let Some(fr) = f.frame(&record(t), &state(1, true)) {
                ticks.push((fr.tick, fr.restarts.len(), fr.possession));
            }
        }
        assert_eq!(
            ticks,
            [
                (3, 0, Some(1)),
                (5, 1, None),
                (7, 0, Some(1)),
                (10, 0, Some(1)),
                (12, 0, Some(1))
            ]
        );
    }

    #[test]
    fn the_groups_and_restarts_are_the_engine_shapes() {
        assert_eq!(group_of(Position::DM), Group::Cm);
        assert_eq!(group_of(Position::RB), Group::Fb);
        assert_eq!(restart_of(StoppageKind::Goal), Some(Restart::KickOff));
        assert_eq!(restart_of(StoppageKind::Injury), None);
    }
}
