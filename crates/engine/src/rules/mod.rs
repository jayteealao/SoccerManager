//! The referee (named mechanism): one state machine inside the loop that owns the phase of
//! play. After the ball moves and possession resolves, the referee decides out of play,
//! offside, fouls with advantage, cards, injuries, restarts, half-time, full time, and
//! abandonment.
//!
//! Every time the ball goes dead, the referee places the ball at the restart spot, marks the
//! tick as a restart, and announces the stoppage through the stoppage hook
//! (`TickSink::on_stoppage`). The laws themselves are pure functions in the sub-modules.

pub mod clock;
pub mod discipline;
pub mod fouls;
pub mod offside;
pub mod restart;

use crate::TICKS_PER_SECOND;
use crate::ball::Ball;
use crate::data::rules::{RulePack, StoppageKind};
use crate::decision::Kick;
use crate::fatigue::InjurySource;
use crate::math::DVec2;
use crate::pitch::{self, Exit, Line};
use crate::player::Status;
use crate::sim::{EngineEventKind, EventDetail, Simulation};
use crate::tuning::Tuning;
use clock::{MatchClock, Tally};
use fouls::Card;
use offside::OffsideSet;
pub use restart::DeadBall;

/// The phase of play.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Live,
    /// The ball is dead and waits at the restart spot.
    DeadBall(DeadBall),
    FullTime,
}

/// One stoppage, as the stoppage hook announces it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stoppage {
    /// The tick the ball went dead.
    pub tick: u32,
    /// What stopped play: the restart kind, a goal, or half-time.
    pub kind: StoppageKind,
    /// The team that restarts play, or `None` at half-time.
    pub team: Option<usize>,
    /// Where the ball waits.
    pub spot: DVec2,
}

/// A card held back while play continued with advantage; the referee shows it at the next
/// stoppage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingCard {
    pub player: usize,
    pub card: Card,
}

/// The referee's state.
#[derive(Debug, Clone, PartialEq)]
pub struct Referee {
    pub phase: Phase,
    /// Players in an offside position since the last kick in open play.
    pub offside: OffsideSet,
    pub pending: Vec<PendingCard>,
    /// Stoppages and cards in the current half, for added time.
    pub tally: Tally,
    pub clock: MatchClock,
    pub abandoned: bool,
}

impl Referee {
    pub fn new(minutes: u32, rules: &RulePack) -> Self {
        Self {
            phase: Phase::Live,
            offside: 0,
            pending: Vec::new(),
            tally: Tally::default(),
            clock: MatchClock::new(minutes, rules),
            abandoned: false,
        }
    }
}

impl Simulation {
    /// Places every player for a kick-off by `team` at once, as at the start of a half, and
    /// gives the centre-forward the ball.
    pub(crate) fn place_kick_off(&mut self, team: usize) {
        self.restart = true;
        self.last_kicker = None;
        self.pass_in_flight = None;
        self.shot_in_flight = None;
        for i in 0..self.players.len() {
            let p = self.players[i];
            if !p.active() {
                continue;
            }
            let side = &self.teams[p.team];
            let pos = restart::kick_off_position(side, p.slot);
            let facing = DVec2::new(side.attack_x, 0.0);
            let p = &mut self.players[i];
            p.pos = pos;
            p.vel = DVec2::ZERO;
            p.target = pos;
            p.facing = facing;
        }
        let kicker = restart::taker(StoppageKind::KickOff, team, DVec2::ZERO, &self.players);
        let mut event = self.event(EngineEventKind::KickOff, Some(team));
        event.spot = Some(DVec2::ZERO);
        event.player = Some(kicker);
        self.events.push(event);
        let attack_x = self.teams[team].attack_x;
        self.players[kicker].pos = DVec2::new(-0.5 * attack_x, 0.0);
        self.players[kicker].target = self.players[kicker].pos;
        self.ball = Ball::at(DVec2::ZERO);
        self.carrier = Some(kicker);
        self.control_since = self.tick;
        self.last_touch = Some(team);
        self.referee.offside = 0;
        self.referee.phase = crate::rules::Phase::Live;
    }

    /// `team` scored. The event names the player who kicked the ball last; a player of the
    /// other club makes it an own goal. The other team kicks off once everyone is back in
    /// place.
    pub(crate) fn goal(&mut self, team: usize) {
        self.summary.goals[team] += 1;
        if self.shot_in_flight == Some(team) {
            self.summary.shots_on_target[team] += 1;
        }
        // Both managers react to the new score on the goal's own stoppage, so a change they
        // queue applies at once instead of waiting for the next stoppage.
        for ai in &mut self.ai {
            ai.due = true;
        }
        tracing::debug!(signal = "match.goal", tick = self.tick, team, score = ?self.summary.goals);
        let mut event = self.event(EngineEventKind::Goal, Some(team));
        event.player = self.last_kicker;
        self.events.push(event);
        self.open_dead_ball(
            StoppageKind::KickOff,
            StoppageKind::Goal,
            1 - team,
            DVec2::ZERO,
            true,
        );
    }

    /// The whole ball crossed a line: a throw-in, a goal kick, or a corner against the team
    /// that touched it last.
    pub(crate) fn ball_out(&mut self, exit: Exit) {
        let last = self.last_touch.unwrap_or(0);
        match exit.line {
            Line::Touch { side } => self.open_dead_ball(
                StoppageKind::ThrowIn,
                StoppageKind::ThrowIn,
                1 - last,
                pitch::throw_in_spot(exit.point, side),
                false,
            ),
            Line::Goal { side } => {
                let attacking = if self.teams[0].attack_x == side { 0 } else { 1 };
                if last == attacking {
                    self.open_dead_ball(
                        StoppageKind::GoalKick,
                        StoppageKind::GoalKick,
                        1 - attacking,
                        pitch::goal_kick_spot(side, exit.point.y),
                        true,
                    );
                } else {
                    self.open_dead_ball(
                        StoppageKind::Corner,
                        StoppageKind::Corner,
                        attacking,
                        pitch::corner_spot(side, exit.point.y),
                        true,
                    );
                }
            }
        }
    }

    /// Player `i` fouled carrier `c`. Outside the offender's penalty area, the referee plays
    /// advantage when the fouled team kept the ball and holds any card for the next
    /// stoppage; otherwise the fouled team gets a free kick, or a penalty inside the area.
    pub(crate) fn foul(&mut self, i: usize, c: usize, ball_lost: bool, t: &Tuning) {
        let offender = self.players[i];
        let fouled_team = self.players[c].team;
        self.summary.fouls[offender.team] += 1;
        let own_end = -self.teams[offender.team].attack_x;
        let at = self.players[c].pos;
        let penalty = pitch::in_penalty_area(at, own_end);
        let advantage = !ball_lost && !penalty;
        let draw = self.rng.referee_draw();
        let card = fouls::card_outcome(offender.derived.aggression, offender.yellow, t, draw);
        let mut event = self.event(EngineEventKind::Foul, Some(offender.team));
        event.player = Some(i);
        event.secondary = Some(c);
        event.advantage = Some(advantage);
        self.events.push(event);
        if advantage {
            if let Some(card) = card {
                self.referee.pending.push(PendingCard { player: i, card });
            }
            return;
        }
        if let Some(card) = card {
            self.show_card(i, card);
        }
        if penalty {
            self.open_dead_ball(
                StoppageKind::Penalty,
                StoppageKind::Penalty,
                fouled_team,
                pitch::penalty_spot(own_end),
                true,
            );
        } else {
            self.open_dead_ball(
                StoppageKind::FreeKick,
                StoppageKind::FreeKick,
                fouled_team,
                pitch::clamp(at, 0.5),
                true,
            );
        }
    }

    /// Player `i` touched the ball from an offside position: an indirect free kick to the
    /// other team where the player became involved.
    pub(crate) fn offside_offence(&mut self, i: usize) {
        let team = self.players[i].team;
        self.summary.offsides[team] += 1;
        self.referee.offside = 0;
        let mut event = self.event(EngineEventKind::Offside, Some(team));
        event.player = Some(i);
        self.events.push(event);
        let spot = pitch::clamp(self.players[i].pos, 0.5);
        self.open_dead_ball(
            StoppageKind::FreeKick,
            StoppageKind::FreeKick,
            1 - team,
            spot,
            false,
        );
    }

    /// Shows `card` to player `i`, sends the player off when the card says so, and ends the
    /// match when the team falls below the rule pack's minimum.
    pub(crate) fn show_card(&mut self, i: usize, card: Card) {
        if !self.players[i].active() {
            return;
        }
        let team = self.players[i].team;
        let sent_off = discipline::book(&mut self.players[i], card);
        if card != Card::Red {
            self.summary.yellow[team] += 1;
        }
        self.referee.tally.cards += 1;
        let mut event = self.event(EngineEventKind::Card, Some(team));
        event.player = Some(i);
        event.card = Some(card);
        self.events.push(event);
        if !sent_off {
            return;
        }
        self.summary.red[team] += 1;
        if self.carrier == Some(i) {
            self.carrier = None;
        }
        discipline::send_off(&mut self.players, &mut self.teams, i);
        self.timeline.push((self.tick + 1, self.teams.clone()));
        if let Some(short) = discipline::abandoned(&self.players, self.config.rules.min_players) {
            tracing::warn!(
                signal = "rules.abandoned",
                tick = self.tick + 1,
                team.id = %self.teams[short].club_id,
                on_pitch = discipline::on_pitch_count(&self.players, short)
            );
            self.referee.abandoned = true;
            self.referee.phase = Phase::FullTime;
        }
    }

    /// Player `i` is injured and leaves play at once: the player stands beside the pitch like
    /// a sent-off player and the line closes up. In open play the referee stops play for a
    /// dropped ball (IFAB Law 8) to the team that last touched the ball, at the ball, or to
    /// that team's goalkeeper when the ball is inside its penalty area. When play already
    /// stopped on this tick (a foul), the injury rides that stoppage. The team's AI manager
    /// checks at once.
    pub(crate) fn injure(&mut self, i: usize, source: InjurySource) {
        if !self.players[i].active() {
            return;
        }
        let p = self.players[i];
        let team = p.team;
        tracing::info!(
            signal = "injury.occurred",
            tick = self.tick + 1,
            team,
            player = %self.teams[team].player_ids[p.squad],
            source = source.code(),
            energy = (p.energy * 100.0).round() / 100.0
        );
        self.summary.injuries[team] += 1;
        self.ai[team].due = true;
        if self.carrier == Some(i) {
            self.carrier = None;
        }
        {
            let p = &mut self.players[i];
            p.status = Status::Injured;
            p.pos = pitch::parking_spot(p.team, p.slot);
            p.vel = DVec2::ZERO;
            p.target = p.pos;
        }
        self.teams[team].reshape(p.slot);
        self.timeline.push((self.tick + 1, self.teams.clone()));
        let detail = Some(EventDetail::Injury { source });
        match self.referee.phase {
            Phase::Live => {
                let with = self.last_touch.unwrap_or(team);
                let spot = pitch::clamp(self.ball.xy(), 0.5);
                self.open_dead_ball(
                    StoppageKind::Injury,
                    StoppageKind::Injury,
                    with,
                    spot,
                    false,
                );
                if let Some(event) = self.events.last_mut()
                    && event.kind == EngineEventKind::Injury
                {
                    event.player = Some(i);
                    event.team = Some(team);
                    event.detail = detail;
                }
            }
            Phase::DeadBall(mut dead) => {
                let mut event = self.event(EngineEventKind::Injury, Some(team));
                event.player = Some(i);
                event.detail = detail;
                self.events.push(event);
                if dead.taker == i {
                    dead.taker = restart::taker(dead.kind, dead.team, dead.spot, &self.players);
                    self.referee.phase = Phase::DeadBall(dead);
                }
            }
            Phase::FullTime => {}
        }
        if let Some(short) = discipline::abandoned(&self.players, self.config.rules.min_players) {
            tracing::warn!(
                signal = "rules.abandoned",
                tick = self.tick + 1,
                team.id = %self.teams[short].club_id,
                on_pitch = discipline::on_pitch_count(&self.players, short)
            );
            self.referee.abandoned = true;
            self.referee.phase = Phase::FullTime;
        }
    }

    /// Shows every card held back for advantage.
    fn show_pending_cards(&mut self) {
        for pending in std::mem::take(&mut self.referee.pending) {
            self.show_card(pending.player, pending.card);
        }
    }

    /// Stops play: shows held-back cards, places the ball at `spot`, marks the tick as a
    /// restart, names the taker, and announces the stoppage. `cause` is what stopped play;
    /// `kind` is the restart that follows.
    pub(crate) fn open_dead_ball(
        &mut self,
        kind: StoppageKind,
        cause: StoppageKind,
        team: usize,
        spot: DVec2,
        direct: bool,
    ) {
        self.show_pending_cards();
        if self.referee.abandoned {
            return;
        }
        self.referee.offside = 0;
        self.ball = Ball::at(spot);
        self.carrier = None;
        self.keeper_beaten = false;
        self.restart = true;
        self.last_kicker = None;
        self.pass_in_flight = None;
        self.shot_in_flight = None;
        let since = self.tick + 1;
        let taker = restart::taker(kind, team, spot, &self.players);
        let mut delay = restart::delay_ticks(kind, &self.config.tuning);
        if self.summary.goals[team] > self.summary.goals[1 - team] {
            // A leading team with time wasting on takes longer over its restarts.
            let factor = self.teams[team].plan.time_wasting;
            // The factor is at most 20 and the delay under 3000 ticks, so the cast fits.
            delay = (f64::from(delay) * factor).round() as u32;
        }
        let ready_at = since + delay;
        self.referee.phase = Phase::DeadBall(DeadBall {
            kind,
            team,
            spot,
            direct,
            since,
            ready_at,
            taker,
        });
        self.referee.tally.add(cause);
        self.summary.stoppages += 1;
        let count = match kind {
            StoppageKind::ThrowIn => Some(&mut self.summary.throw_ins),
            StoppageKind::Corner => Some(&mut self.summary.corners),
            StoppageKind::GoalKick => Some(&mut self.summary.goal_kicks),
            StoppageKind::FreeKick => Some(&mut self.summary.free_kicks),
            StoppageKind::Penalty => Some(&mut self.summary.penalties),
            _ => None,
        };
        if let Some(count) = count {
            count[team] += 1;
        }
        let mut event = self.event(EngineEventKind::restart(kind), Some(team));
        event.spot = Some(spot);
        event.player = Some(taker);
        self.events.push(event);
        self.stoppage = Some(Stoppage {
            tick: since,
            kind: cause,
            team: Some(team),
            spot,
        });
    }

    /// One tick of a dead ball: every player steers to a restart target, and the taker
    /// restarts play when ready, or at the hard limit whatever the players are doing.
    pub(crate) fn dead_ball_tick(&mut self, dead: &DeadBall, t: &Tuning) {
        for i in 0..self.players.len() {
            if self.players[i].active() {
                let target = restart::target(dead, i, &self.players, &self.teams, t);
                self.players[i].target = target;
            }
        }
        let now = self.tick + 1;
        let forced = now >= dead.hard_limit();
        if !forced && !restart::is_ready(dead, now, &self.players, &self.teams, t) {
            return;
        }
        if forced {
            let taker_distance_m = (self.players[dead.taker].pos - dead.spot).length();
            tracing::warn!(
                signal = "rules.dead_ball_stalled",
                tick = now,
                kind = dead.kind.code(),
                team.id = %self.teams[dead.team].club_id,
                waited_ticks = now - dead.since,
                taker_distance_m = (taker_distance_m * 100.0).round() / 100.0
            );
        }
        self.take_restart(dead, t);
    }

    /// The taker restarts play. A throw-in, a corner, a goal kick, and an indirect free kick
    /// are kicked at once, and only the free kick can put a player offside; after any other
    /// restart the taker holds the ball and decides on the next tick.
    fn take_restart(&mut self, dead: &DeadBall, t: &Tuning) {
        self.referee.phase = Phase::Live;
        self.carrier = Some(dead.taker);
        self.control_since = self.tick;
        self.last_touch = Some(dead.team);
        let kick = match dead.kind {
            StoppageKind::ThrowIn | StoppageKind::Corner => {
                Some((self.restart_pass(dead.taker, dead.kind), false))
            }
            StoppageKind::GoalKick => {
                let kick = self
                    .decide_carrier(dead.taker)
                    .unwrap_or_else(|| self.restart_pass(dead.taker, dead.kind));
                Some((kick, false))
            }
            StoppageKind::FreeKick if !dead.direct => {
                Some((self.restart_pass(dead.taker, dead.kind), true))
            }
            _ => None,
        };
        if let Some((kick, offside_counts)) = kick {
            let kick: Kick = kick;
            self.apply_kick(kick, t, offside_counts);
        }
    }

    /// Ends the half when its regulation time plus added time has passed. Added time is fixed
    /// on the first tick after regulation time.
    pub(crate) fn check_clock(&mut self) {
        if self.is_over() {
            return;
        }
        let now = self.tick + 1;
        let mut clock = self.referee.clock;
        if !clock.regulation_over(now) {
            return;
        }
        let half = clock.half as usize;
        if clock.added_ticks[half].is_none() {
            let added_s = if clock.plays_added {
                let added = &self.config.rules.added_time;
                let draw = self.rng.referee_draw();
                let seconds = clock::added_seconds(&self.referee.tally, added, draw);
                tracing::info!(
                    signal = "rules.added_time",
                    half,
                    tally_s = self.referee.tally.seconds(added),
                    variance_s = ((2.0 * draw - 1.0) * f64::from(added.variance_s)).round(),
                    added_s = seconds,
                    announced_min = clock::announced_minutes(seconds)
                );
                seconds
            } else {
                0
            };
            clock.added_ticks[half] = Some(added_s * TICKS_PER_SECOND);
            self.summary.added_s[half.min(1)] = added_s;
            self.referee.clock = clock;
        }
        if clock.half_end().is_some_and(|end| now >= end) {
            if clock.last_half() {
                self.referee.phase = Phase::FullTime;
            } else {
                self.half_time();
            }
        }
    }

    /// Half-time: the teams change ends and the team that did not kick off the first half
    /// kicks off the second, placed at once.
    fn half_time(&mut self) {
        let now = self.tick + 1;
        let half = self.referee.clock.half as usize;
        let mut event = self.event(EngineEventKind::HalfTime, None);
        event.added_time_s = self
            .referee
            .clock
            .plays_added
            .then(|| self.summary.added_s[half.min(1)]);
        self.events.push(event);
        self.show_pending_cards();
        if self.referee.abandoned {
            return;
        }
        self.half_time_recovery();
        self.summary.stoppages += 1;
        self.stoppage = Some(Stoppage {
            tick: now,
            kind: StoppageKind::HalfTime,
            team: None,
            spot: DVec2::ZERO,
        });
        self.referee.clock.next_half(now);
        self.referee.tally = Tally::default();
        for team in &mut self.teams {
            team.switch_ends();
        }
        self.timeline.push((now, self.teams.clone()));
        self.place_kick_off(1);
    }
}
