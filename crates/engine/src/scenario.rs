//! Test scenes. A scene starts from a placed kick-off and moves players (and sets their
//! running velocity and foul cooldown), the ball, the
//! carrier, the clock and its period, the score, the cards, energy, substitutions used, the
//! managers, the change queue, the knockout switch, the outcomes of the next shoot-out kicks,
//! the referee's and injury rolls' next draws, and the plugin hooks to where a test needs
//! them.
//! Only this crate's tests build it (the `scenario` feature), so a release build never
//! contains it.

use crate::ai::Manager;
use crate::data::rules::StoppageKind;
use crate::decision::Kick;
use crate::fatigue::InjurySource;
use crate::math::{DVec2, DVec3};
use crate::rules::clock::{MatchClock, TICKS_PER_MINUTE};
use crate::sim::{MatchConfig, Simulation};
use crate::tactics::change::Change;

/// A match being arranged for a test.
pub struct Scene {
    sim: Simulation,
}

impl Scene {
    /// A match placed for kick-off, with the kick-off event cleared.
    pub fn new(config: MatchConfig) -> Self {
        let mut sim = Simulation::new(config).expect("a valid configuration");
        sim.events.clear();
        // A scene arranges open play: the kick-off taker's first kick is not a restart kick.
        sim.restart_taker = None;
        Self { sim }
    }

    /// Switches debug mode on from the scene's next step: the trace records every draw,
    /// decision point, and rule outcome from here on (`Simulation::take_trace`).
    pub fn traced(mut self) -> Self {
        self.sim.streams.enable_trace(self.sim.tick + 1);
        self
    }

    /// Puts player `i` at rest at `at`.
    pub fn place(mut self, i: usize, at: DVec2) -> Self {
        let p = &mut self.sim.players[i];
        p.pos = at;
        p.target = at;
        p.vel = DVec2::ZERO;
        self
    }

    /// Puts the ball at rest at `at`.
    pub fn ball(mut self, at: DVec3) -> Self {
        self.sim.ball.pos = at;
        self.sim.ball.vel = DVec3::ZERO;
        self
    }

    pub fn ball_velocity(mut self, v: DVec3) -> Self {
        self.sim.ball.vel = v;
        self
    }

    /// Gives the ball to player `c`, or to nobody. The carrier has held the ball since tick 0,
    /// so a tackle is possible once the scene's tick passes the control cooldown.
    pub fn carrier(mut self, c: Option<usize>) -> Self {
        self.sim.carrier = c;
        self.sim.restart_taker = None;
        self.sim.control_since = 0;
        if let Some(c) = c {
            self.sim.last_touch = Some(self.sim.players[c].team);
        }
        self
    }

    /// The team that touched the ball last.
    pub fn last_touch(mut self, team: usize) -> Self {
        self.sim.last_touch = Some(team);
        self
    }

    /// Sets the tick the next step starts from.
    pub fn tick(mut self, tick: u32) -> Self {
        self.sim.tick = tick;
        self
    }

    /// The referee's next draws, before the seeded stream: tackles, cards, and added time
    /// (every referee-class key of the stream table).
    pub fn rolls(mut self, draws: &[f64]) -> Self {
        self.sim.streams.script_referee(draws);
        self
    }

    /// Gives player `i` yellow cards already shown.
    pub fn yellow(mut self, i: usize, cards: u8) -> Self {
        self.sim.players[i].yellow = cards;
        self
    }

    /// Sends player `i` off before the scene starts, as a card shown would: the player
    /// loses the ball and the team is laid out again.
    pub fn sent_off(mut self, i: usize) -> Self {
        self.sim.send_off_before_kickoff(i);
        self
    }

    /// Sets player `i`'s velocity, as for a player already running.
    pub fn velocity(mut self, i: usize, v: DVec2) -> Self {
        let p = &mut self.sim.players[i];
        p.vel = v;
        if v != DVec2::ZERO {
            p.facing = v.normalize();
        }
        self
    }

    /// Player `i` may attempt a tackle again from `tick`, as after a foul.
    pub fn foul_ready(mut self, i: usize, tick: u32) -> Self {
        self.sim.players[i].foul_ready = tick;
        self
    }

    /// Counts `n` stoppages of `kind` in the current half.
    pub fn tally(mut self, kind: StoppageKind, n: u32) -> Self {
        self.sim.referee.tally.kinds[kind.index()] += n;
        self
    }

    /// Counts `n` video reviews in the current half, as the video referee would.
    pub fn reviews(mut self, n: u32) -> Self {
        self.sim.referee.tally.reviews += n;
        self
    }

    /// Counts `n` cards in the current half.
    pub fn cards(mut self, n: u32) -> Self {
        self.sim.referee.tally.cards += n;
        self
    }

    /// The carrier kicks the ball in open play toward `dir` at `speed` with vertical speed
    /// `loft`, which fixes who is in an offside position.
    pub fn kick(mut self, dir: DVec2, speed: f64, loft: f64) -> Self {
        let t = self.sim.config.tuning.clone();
        self.sim.apply_kick(
            Kick::Pass {
                dir: dir.normalize(),
                speed,
                loft,
            },
            &t,
            true,
        );
        self
    }

    /// The carrier clears the ball toward `dir` at `speed` with vertical speed `loft`.
    pub fn clear(mut self, dir: DVec2, speed: f64, loft: f64) -> Self {
        let t = self.sim.config.tuning.clone();
        self.sim.apply_kick(
            Kick::Clear {
                dir: dir.normalize(),
                speed,
                loft,
            },
            &t,
            true,
        );
        self
    }

    /// The carrier shoots toward `dir` at `speed` with vertical speed `loft`.
    pub fn shoot(mut self, dir: DVec2, speed: f64, loft: f64) -> Self {
        let t = self.sim.config.tuning.clone();
        self.sim.apply_kick(
            Kick::Shot {
                dir: dir.normalize(),
                speed,
                loft,
            },
            &t,
            true,
        );
        self
    }

    /// Opens a penalty for `team` taken by player `taker`: the taker and the ball on the
    /// penalty mark of the goal `team` attacks, the defending side's acting keeper on his goal
    /// line, and the kick due on the next step.
    pub fn penalty(mut self, team: usize, taker: usize) -> Self {
        let attack_x = self.sim.teams[team].attack_x;
        let spot = self.sim.config.pitch().penalty_spot(attack_x);
        let keeper = self.sim.keeper(1 - team);
        let line_x = attack_x * (self.sim.config.pitch().half_length() - 0.5);
        self = self
            .place(taker, spot)
            .place(keeper, DVec2::new(line_x, 0.0))
            .ball(DVec3::new(spot.x, spot.y, 0.0))
            .carrier(None);
        let sim = &mut self.sim;
        sim.last_touch = Some(team);
        sim.seat_phase(crate::rules::Phase::DeadBall(crate::rules::DeadBall {
            kind: StoppageKind::Penalty,
            team,
            spot,
            direct: true,
            since: sim.tick,
            ready_at: sim.tick,
            taker,
        }));
        self
    }

    /// The injury rolls' next draws, before the seeded stream.
    pub fn injury_rolls(mut self, draws: &[f64]) -> Self {
        self.sim.streams.script_injuries(draws);
        self
    }

    /// Sets player `i`'s energy and the effective values that follow from it.
    pub fn energy(mut self, i: usize, energy: f64) -> Self {
        self.sim.players[i].energy = energy;
        self.sim.refresh_effective();
        self
    }

    /// Sets the score, home first.
    pub fn score(mut self, goals: [u32; 2]) -> Self {
        self.sim.summary.goals = goals;
        self
    }

    /// Moves the clock to minute `minute` of a match that plays two halves: in the second
    /// half the first half is over with no time added, the teams have changed ends, and the
    /// away team's kick-off is placed. The kick-off event is cleared.
    pub fn at_minute(mut self, minute: u32) -> Self {
        let clock = self.sim.referee.clock;
        let half_minutes = clock.half_ticks / TICKS_PER_MINUTE;
        if minute >= half_minutes && clock.halves > 1 {
            let clock = &mut self.sim.referee.clock;
            clock.added_ticks[0] = Some(0);
            clock.next_half(clock.half_ticks);
            for team in &mut self.sim.teams {
                team.switch_ends();
            }
            // The scene skips to the break, so the kick-off starts the period from it.
            self.sim.seat_break();
            self.sim.place_kick_off(1);
            self.sim.events.clear();
        }
        self.sim.tick = minute * TICKS_PER_MINUTE;
        self.sim.control_since = self.sim.tick;
        self
    }

    /// The match is a knockout match: level after regulation time, it plays extra time and
    /// then a shoot-out. The clock keeps its place.
    pub fn knockout(mut self) -> Self {
        let config = &mut self.sim.config;
        config.knockout = true;
        let clock = MatchClock::new(config.minutes, &config.rules, true);
        self.sim.referee.clock.extra_periods = clock.extra_periods;
        self
    }

    /// Moves the clock to the start of period `period` (2 and 3 are the extra-time periods
    /// of a knockout match) with no time added before it. The teams have changed ends once a
    /// break, and the kick-off is placed: the away team's in the second half, the home team's
    /// in the first extra-time period, and the away team's in the second. The kick-off event
    /// is cleared.
    pub fn period(mut self, period: u32) -> Self {
        let mut clock = self.sim.referee.clock;
        let mut start = 0;
        for p in 0..period {
            start += clock.period_ticks(p);
            clock.added_ticks[p as usize] = Some(0);
        }
        clock.half = period;
        clock.half_start = start;
        self.sim.referee.clock = clock;
        for _ in 0..period {
            for team in &mut self.sim.teams {
                team.switch_ends();
            }
        }
        let kick_off = if period >= clock.halves {
            self.sim.summary.extra_time = true;
            self.sim.referee.extra_kick_off = Some(0);
            usize::from(period > clock.halves)
        } else {
            usize::from(period % 2 == 1)
        };
        self.sim.tick = start;
        // The scene skips to the break, so the kick-off starts the period from it.
        self.sim.seat_break();
        self.sim.place_kick_off(kick_off);
        self.sim.events.clear();
        self.sim.control_since = self.sim.tick;
        self
    }

    /// The next shoot-out kicks score or miss as `outcomes` says, in order, whatever the ball
    /// does; each kick is still played on the pitch.
    pub fn shootout_kicks(mut self, outcomes: &[bool]) -> Self {
        self.sim.forced_kicks.extend(outcomes.iter().copied());
        self
    }

    /// `team` has made `used` substitutions in `windows` windows.
    pub fn subs_used(mut self, team: usize, used: u8, windows: u8) -> Self {
        let ledger = &mut self.sim.ledgers[team];
        ledger.used = used;
        ledger.windows = windows;
        self
    }

    /// Injures player `i` through the engine's own path, as a background roll would.
    pub fn injure(mut self, i: usize) -> Self {
        self.sim.injure(i, InjurySource::Background);
        self
    }

    /// `team` is managed by `manager`.
    pub fn manager(mut self, team: usize, manager: Manager) -> Self {
        self.sim.managers[team] = manager;
        self
    }

    /// Queues `change` for `team`.
    pub fn queue(mut self, team: usize, change: Change) -> Self {
        self.sim.queue_change(team, change);
        self
    }

    /// Attaches plugin hooks to the match.
    pub fn plugins(mut self, plugins: crate::plugin::Plugins) -> Self {
        self.sim.set_plugins(plugins);
        self
    }

    pub fn build(mut self) -> Simulation {
        self.sim.timeline = vec![(self.sim.tick, self.sim.teams.clone())];
        self.sim
    }
}
