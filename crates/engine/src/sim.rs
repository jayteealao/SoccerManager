//! The fixed-timestep simulation loop (named mechanism: tick count is a function of match
//! time alone). Order per tick: decisions, steering, ball, possession, overlap resolution.

use crate::ball::Ball;
use crate::decision::Kick;
use crate::error::EngineError;
use crate::math::{DVec2, DVec3};
use crate::pitch;
use crate::player::{Attributes, Player};
use crate::record::{TickRecord, TickSink};
use crate::rng::EngineRng;
use crate::steering;
use crate::team::{PLAYERS_PER_TEAM, Team};
use crate::tuning::Tuning;

/// Everything a match needs to start.
#[derive(Debug, Clone)]
pub struct MatchConfig {
    pub seed: u64,
    pub minutes: u32,
    pub tuning: Tuning,
}

impl MatchConfig {
    /// A configuration with the default tuning. `minutes` must be 1 to 200.
    pub fn new(seed: u64, minutes: u32) -> Result<Self, EngineError> {
        if !(1..=200).contains(&minutes) {
            return Err(EngineError::InvalidConfig(format!(
                "minutes must be 1 to 200, got {minutes}"
            )));
        }
        Ok(Self {
            seed,
            minutes,
            tuning: Tuning::default(),
        })
    }
}

/// Aggregate counters kept during a match.
#[derive(Debug, Clone, Copy, Default)]
pub struct Summary {
    pub possession_changes: u32,
    pub ball_max_speed: f64,
    pub ball_idle_ticks: u32,
    pub goals: [u32; 2],
}

/// One running match.
pub struct Simulation {
    pub(crate) config: MatchConfig,
    pub(crate) teams: [Team; 2],
    pub(crate) players: Vec<Player>,
    pub(crate) ball: Ball,
    pub(crate) rng: EngineRng,
    pub(crate) tick: u32,
    pub(crate) carrier: Option<usize>,
    pub(crate) control_since: u32,
    last_touch: Option<usize>,
    summary: Summary,
    scratch: Vec<DVec2>,
}

impl Simulation {
    /// Places both built-in teams for kick-off.
    pub fn new(config: MatchConfig) -> Result<Self, EngineError> {
        if config.tuning.dt <= 0.0 || config.tuning.decision_interval_ticks == 0 {
            return Err(EngineError::InvalidConfig(
                "dt and decision_interval_ticks must be positive".into(),
            ));
        }
        let teams = [Team::builtin(0), Team::builtin(1)];
        let mut players = teams[0].players(0, Attributes::uniform(60));
        players.extend(teams[1].players(PLAYERS_PER_TEAM, Attributes::uniform(60)));
        let rng = EngineRng::from_seed(config.seed);
        let mut sim = Self {
            config,
            teams,
            players,
            ball: Ball::at(DVec2::ZERO),
            rng,
            tick: 0,
            carrier: None,
            control_since: 0,
            last_touch: None,
            summary: Summary::default(),
            scratch: Vec::with_capacity(2 * PLAYERS_PER_TEAM),
        };
        sim.kick_off(0);
        Ok(sim)
    }

    pub fn tuning(&self) -> &Tuning {
        &self.config.tuning
    }

    pub fn teams(&self) -> [Team; 2] {
        self.teams.clone()
    }

    pub fn tick(&self) -> u32 {
        self.tick
    }

    pub fn summary(&self) -> Summary {
        self.summary
    }

    /// Runs `ticks` ticks, emitting one record per tick.
    pub fn run<S: TickSink>(&mut self, ticks: u32, sink: &mut S) -> Result<(), EngineError> {
        for _ in 0..ticks {
            self.step();
            sink.on_tick(&self.record())?;
        }
        Ok(())
    }

    /// Advances the match by one tick.
    pub fn step(&mut self) {
        let t = self.config.tuning.clone();
        let kick = if self.tick % t.decision_interval_ticks == 0 {
            self.decide()
        } else {
            None
        };
        if let Some(kick) = kick {
            self.apply_kick(kick, &t);
        }
        steering::step_all(&mut self.players, &mut self.scratch, &t);
        self.move_ball(&t);
        self.resolve_possession(&t);
        steering::resolve_overlaps(&mut self.players, &t);
        self.tick += 1;
    }

    /// The current tick as a record with 32-bit positions.
    pub fn record(&self) -> TickRecord {
        let mut players = [[0.0f32; 2]; 2 * PLAYERS_PER_TEAM];
        for (slot, p) in players.iter_mut().zip(self.players.iter()) {
            *slot = [p.pos.x as f32, p.pos.y as f32];
        }
        TickRecord {
            tick: self.tick,
            ball: [
                self.ball.pos.x as f32,
                self.ball.pos.y as f32,
                self.ball.pos.z as f32,
            ],
            players,
        }
    }

    fn apply_kick(&mut self, kick: Kick, t: &Tuning) {
        let (dir, speed, loft) = match kick {
            Kick::Pass { dir, speed, loft } | Kick::Shot { dir, speed, loft } => (dir, speed, loft),
        };
        if let Some(c) = self.carrier {
            self.last_touch = Some(self.players[c].team);
        }
        self.ball.kick(dir, speed, loft, t);
        self.carrier = None;
    }

    fn move_ball(&mut self, t: &Tuning) {
        match self.carrier {
            Some(c) => {
                let p = &self.players[c];
                let at = pitch::clamp(p.pos + p.facing * 0.5, 0.1);
                let step = crate::math::clamp_len(at - self.ball.xy(), t.carry_step);
                let next = self.ball.xy() + step;
                self.ball.pos = DVec3::new(next.x, next.y, 0.0);
                self.ball.vel = DVec3::new(p.vel.x, p.vel.y, 0.0);
            }
            None => {
                self.ball.integrate(t);
                let xy = self.ball.xy();
                for team in 0..2 {
                    if pitch::in_goal(xy, self.teams[team].attack_x)
                        && self.ball.pos.z < t.crossbar_height
                    {
                        self.summary.goals[team] += 1;
                        tracing::debug!(signal = "match.goal", tick = self.tick, team, score = ?self.summary.goals);
                        self.kick_off(1 - team);
                        return;
                    }
                }
                // sdlc-debt: touchlines and goal lines bounce the ball like walls until the
                // match-rules slice adds throw-ins, corners, and goal kicks.
                if xy.x.abs() > pitch::HALF_LENGTH {
                    self.ball.pos.x = xy.x.clamp(-pitch::HALF_LENGTH, pitch::HALF_LENGTH);
                    self.ball.vel.x = -self.ball.vel.x * t.restitution;
                }
                if xy.y.abs() > pitch::HALF_WIDTH {
                    self.ball.pos.y = xy.y.clamp(-pitch::HALF_WIDTH, pitch::HALF_WIDTH);
                    self.ball.vel.y = -self.ball.vel.y * t.restitution;
                }
            }
        }
        let speed = self.ball.speed();
        if speed > self.summary.ball_max_speed {
            self.summary.ball_max_speed = speed;
        }
        if speed == 0.0 && self.carrier.is_none() {
            self.summary.ball_idle_ticks += 1;
        }
    }

    /// Places every player at its base and gives `team`'s centre-forward the ball.
    fn kick_off(&mut self, team: usize) {
        for i in 0..self.players.len() {
            let p = self.players[i];
            let pos = self.teams[p.team].slot_base(p.slot);
            self.players[i].pos = pos;
            self.players[i].vel = DVec2::ZERO;
            self.players[i].target = pos;
            self.players[i].facing = DVec2::new(self.teams[p.team].attack_x, 0.0);
        }
        let kicker = team * PLAYERS_PER_TEAM + PLAYERS_PER_TEAM - 1;
        let attack_x = self.teams[team].attack_x;
        self.players[kicker].pos = DVec2::new(-0.5 * attack_x, 0.0);
        self.players[kicker].target = self.players[kicker].pos;
        self.ball = Ball::at(DVec2::ZERO);
        self.carrier = Some(kicker);
        self.control_since = self.tick;
        self.last_touch = Some(team);
    }

    fn resolve_possession(&mut self, t: &Tuning) {
        let ball_xy = self.ball.xy();
        match self.carrier {
            None => {
                if self.ball.pos.z > t.reach_height {
                    return;
                }
                let fast = self.ball.speed() > t.control_speed;
                let mut best: Option<(f64, usize)> = None;
                for (i, p) in self.players.iter().enumerate() {
                    let keeper = p.slot == 0;
                    if fast && !keeper {
                        continue;
                    }
                    let reach = if keeper {
                        t.keeper_reach
                    } else {
                        t.reach_radius
                    };
                    let d = (p.pos - ball_xy).length();
                    if d < reach && best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, i));
                    }
                }
                if let Some((_, i)) = best {
                    if fast && !self.rng.chance(t.keeper_catch_chance) {
                        return;
                    }
                    self.gain(i, t);
                }
            }
            Some(c) => {
                if self.tick.saturating_sub(self.control_since) < t.control_cooldown_ticks {
                    return;
                }
                let carrier = self.players[c];
                for i in 0..self.players.len() {
                    let p = self.players[i];
                    if p.team == carrier.team || (p.pos - ball_xy).length() > t.reach_radius {
                        continue;
                    }
                    let tackle = f64::from(p.attributes.tackling);
                    let dribble = f64::from(carrier.attributes.dribbling);
                    let p_win = 0.05 * tackle / (tackle + dribble);
                    if self.rng.chance(p_win) {
                        self.gain(i, t);
                        return;
                    }
                }
            }
        }
    }

    fn gain(&mut self, i: usize, _t: &Tuning) {
        let team = self.players[i].team;
        if self.last_touch != Some(team) {
            self.summary.possession_changes += 1;
        }
        self.last_touch = Some(team);
        self.carrier = Some(i);
        self.control_since = self.tick;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::VecSink;

    #[test]
    fn five_minutes_produce_fifteen_thousand_ticks() {
        let mut sim = Simulation::new(MatchConfig::new(42, 5).unwrap()).unwrap();
        let mut sink = VecSink::default();
        sim.run(crate::ticks_for_minutes(5), &mut sink).unwrap();
        assert_eq!(sink.records.len(), 15_000);
        assert_eq!(sink.records[0].tick, 1);
        assert!(
            sim.summary().possession_changes > 0,
            "nobody ever gained possession"
        );
    }

    #[test]
    fn zero_minutes_is_rejected() {
        assert!(MatchConfig::new(1, 0).is_err());
    }
}
