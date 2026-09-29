//! The referee (named mechanism): one state machine inside the loop that owns the phase of
//! play. After the ball moves and possession resolves, the referee decides out of play,
//! offside, fouls with advantage, cards, injuries, restarts, half-time, extra time and the
//! penalty shoot-out of a knockout match, full time, and abandonment.
//!
//! Every time the ball goes dead, the referee places the ball at the restart spot, marks the
//! tick as a restart, and announces the stoppage through the stoppage hook
//! (`TickSink::on_stoppage`). The laws themselves are pure functions in the sub-modules.

pub mod clock;
pub mod discipline;
pub mod fouls;
pub mod injury;
pub mod offside;
pub mod restart;
pub mod shootout;

use crate::TICKS_PER_SECOND;
use crate::ball::Ball;
use crate::data::rules::{RulePack, StoppageKind};
use crate::decision::Kick;
use crate::fatigue::InjurySource;
use crate::math::DVec2;
use crate::modules::{PeriodEnd, ShootoutLineup};
use crate::pitch::{self, Exit, Line};
use crate::player::Status;
use crate::sim::{DecidedBy, EngineEventKind, EventDetail, Simulation};
use crate::streams::{Action, Key};
use crate::trace::Point;
use crate::tuning::Tuning;
use clock::{KICK_LIVE_TICKS, MatchClock, Tally};
use fouls::Card;
use offside::OffsideSet;
pub use restart::DeadBall;
use serde_json::json;

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

/// A penalty shoot-out in progress (IFAB Law 10). Every kick is played on the pitch.
#[derive(Debug, Clone, PartialEq)]
pub struct Shootout {
    /// Each team's kickers in order, as roster indices.
    pub order: [Vec<usize>; 2],
    /// The roster index of each team's keeper.
    pub keepers: [usize; 2],
    /// Each team's place in its order: the next kicker is `order[team][cursor[team] % len]`.
    pub cursor: [usize; 2],
    /// The team that won the toss and kicks first.
    pub first: usize,
    /// The end every kick is taken at: the sign of the goal line's `x`.
    pub end: f64,
    pub scores: [u32; 2],
    /// Kicks each team has taken.
    pub taken: [u32; 2],
    /// The kicker of the kick being taken, from the moment its dead ball opens.
    pub kicker: Option<usize>,
    /// The tick the kick in progress went live.
    pub live_since: Option<u32>,
}

impl Shootout {
    /// The team of the kick being taken.
    fn kicking_team(&self) -> usize {
        shootout::next_team(self.first, self.taken)
    }
}

/// The referee's state.
#[derive(Debug, Clone, PartialEq)]
pub struct Referee {
    pub phase: Phase,
    /// Players in an offside position since the last kick in open play.
    pub offside: OffsideSet,
    pub pending: Vec<PendingCard>,
    /// Stoppages and cards in the current period, for added time.
    pub tally: Tally,
    pub clock: MatchClock,
    pub abandoned: bool,
    /// The team that kicked off the first period of extra time, once it has started.
    pub extra_kick_off: Option<usize>,
    /// The shoot-out, once it has started.
    pub shootout: Option<Shootout>,
}

impl Referee {
    pub fn new(minutes: u32, rules: &RulePack, knockout: bool) -> Self {
        Self {
            phase: Phase::Live,
            offside: 0,
            pending: Vec::new(),
            tally: Tally::default(),
            clock: MatchClock::new(minutes, rules, knockout),
            abandoned: false,
            extra_kick_off: None,
            shootout: None,
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
        self.restart_taker = None;
        self.clearers_tried = 0;
        self.end_shot();
        let restarts = self.config.modules.restarts;
        for i in 0..self.players.len() {
            let p = self.players[i];
            if !p.active() {
                continue;
            }
            let pos = restarts.kick_off_position(&self.view(), p.team, p.slot);
            let facing = DVec2::new(self.teams[p.team].attack_x, 0.0);
            let p = &mut self.players[i];
            p.pos = pos;
            p.vel = DVec2::ZERO;
            p.target = pos;
            p.facing = facing;
        }
        let kicker = restarts.taker(&self.view(), StoppageKind::KickOff, team, DVec2::ZERO);
        let mut event = self.event(EngineEventKind::KickOff, Some(team));
        event.spot = Some(DVec2::ZERO);
        event.player = Some(kicker);
        self.events.push(event);
        if self.trace_on() {
            self.trace_point(Point::KickOff, json!({"team": team, "kicker": kicker}));
        }
        let attack_x = self.teams[team].attack_x;
        self.players[kicker].pos = DVec2::new(-0.5 * attack_x, 0.0);
        self.players[kicker].target = self.players[kicker].pos;
        self.ball = Ball::at(DVec2::ZERO);
        self.carrier = Some(kicker);
        self.control_since = self.tick;
        self.last_touch = Some(team);
        self.restart_taker = Some(kicker);
        self.referee.offside = 0;
        self.referee.phase = crate::rules::Phase::Live;
    }

    /// `team` scored. The event names the player who kicked the ball last; a player of the
    /// other club makes it an own goal. The other team kicks off once everyone is back in
    /// place.
    pub(crate) fn goal(&mut self, team: usize) {
        self.summary.goals[team] += 1;
        #[cfg(feature = "scenario")]
        if self.shot_in_flight == Some(team) {
            self.census.scored += 1;
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
        if self.trace_on() {
            self.trace_point(
                Point::Goal,
                json!({"team": team, "scorer": self.last_kicker, "score": self.summary.goals}),
            );
        }
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
        if self.trace_on() {
            let (line, restart) = match exit.line {
                Line::Touch { .. } => ("touch", StoppageKind::ThrowIn),
                Line::Goal { side } => {
                    let attacking = if self.teams[0].attack_x == side { 0 } else { 1 };
                    let kind = if last == attacking {
                        StoppageKind::GoalKick
                    } else {
                        StoppageKind::Corner
                    };
                    ("goal", kind)
                }
            };
            self.trace_point(
                Point::BallOut,
                json!({
                    "line": line,
                    "point": [exit.point.x, exit.point.y],
                    "last_touch": last,
                    "restart": restart.code(),
                }),
            );
        }
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
                #[cfg(feature = "scenario")]
                if last == attacking && self.shot_in_flight == Some(attacking) {
                    self.census.wide += 1;
                }
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
        // The offender makes no tackle attempt until the cooldown has passed, advantage or not.
        self.players[i].foul_ready = self.tick.saturating_add(t.foul_cooldown_ticks);
        let own_end = -self.teams[offender.team].attack_x;
        let at = self.players[c].pos;
        let penalty = pitch::in_penalty_area(at, own_end);
        let advantage = !ball_lost && !penalty;
        let fouls = self.config.modules.fouls;
        let thresholds = fouls.card_thresholds(&self.view(), i);
        let draw = self
            .streams
            .tested(Key::player(Action::FoulCard, &offender), &thresholds);
        let mut card = fouls.card(&self.view(), i, draw);
        let decided = card;
        let mut event = self.event(EngineEventKind::Foul, Some(offender.team));
        event.player = Some(i);
        event.secondary = Some(c);
        event.advantage = Some(advantage);
        self.events.push(event);
        if let Some(hook) = self.plugins.rule.as_mut() {
            let ctx = crate::plugin::FoulContext {
                tick: self.tick,
                minute: self.referee.clock.minute(self.tick).0,
                team: offender.team,
                slot: offender.slot,
                yellows: offender.yellow,
                aggression: offender.derived.aggression,
                advantage,
                penalty,
            };
            let outcome = hook.card(&ctx, card);
            let (value, notes) =
                self.plugins
                    .settle(crate::plugin::HookPoint::Rule, outcome, self.tick);
            self.push_script_notes(notes);
            if let Some(scripted) = value {
                card = scripted;
            }
        }
        if self.trace_on() {
            self.trace_point(
                Point::Foul,
                json!({
                    "offender": i,
                    "fouled": c,
                    "advantage": advantage,
                    "penalty": penalty,
                    "ball_lost": ball_lost,
                    "card_decided": decided.map(|k| k.code()),
                    "card": card.map(|k| k.code()),
                }),
            );
        }
        if advantage {
            if let Some(card) = card {
                self.hold_card(i, card);
            }
            return;
        }
        // One card per player per stoppage: a card held back for this player merges with the
        // card for this foul, and the more severe of the two is shown.
        let held = self
            .referee
            .pending
            .iter()
            .position(|p| p.player == i)
            .map(|k| self.referee.pending.remove(k).card);
        let card = match (card, held) {
            (Some(a), Some(b)) => Some(self.config.modules.discipline.more_severe(a, b)),
            (a, b) => a.or(b),
        };
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

    /// Holds `card` for player `i` until the next stoppage. A player holds at most one card in
    /// a spell of advantage: a more severe card replaces the one held, and a less severe one
    /// is dropped.
    fn hold_card(&mut self, i: usize, card: Card) {
        let discipline = self.config.modules.discipline;
        match self.referee.pending.iter_mut().find(|p| p.player == i) {
            Some(held) => held.card = discipline.more_severe(held.card, card),
            None => self.referee.pending.push(PendingCard { player: i, card }),
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
        if self.trace_on() {
            self.trace_point(Point::Offside, json!({"player": i, "team": team}));
        }
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
        self.show_card_as(i, card, false);
    }

    /// Shows `card` to player `i`; `held` marks a card held back for advantage until this
    /// stoppage.
    fn show_card_as(&mut self, i: usize, card: Card, held: bool) {
        let judge = self.config.modules.discipline;
        let Some(card) = judge.shown(&self.view(), i, card) else {
            return;
        };
        let team = self.players[i].team;
        discipline::book(&mut self.players[i], card);
        let sent_off = judge.sends_off(card);
        if card != Card::Red {
            self.summary.yellow[team] += 1;
        }
        self.referee.tally.cards += 1;
        let mut event = self.event(EngineEventKind::Card, Some(team));
        event.player = Some(i);
        event.card = Some(card);
        self.events.push(event);
        if self.trace_on() {
            self.trace_point(
                Point::Card,
                json!({"player": i, "team": team, "card": card.code(), "held": held}),
            );
        }
        if !sent_off {
            return;
        }
        self.summary.red[team] += 1;
        if self.carrier == Some(i) {
            self.carrier = None;
        }
        discipline::send_off(&mut self.players, &mut self.teams, i);
        if self.trace_on() {
            self.trace_point(Point::SendOff, json!({"player": i, "team": team}));
        }
        // The team's manager reacts at once, as after an injury: a keeper sent off asks for
        // the bench keeper.
        self.ai[team].due = true;
        self.timeline.push((self.tick + 1, self.teams.clone()));
        if let Some(short) = self.config.modules.clock.abandoned(&self.view()) {
            tracing::warn!(
                signal = "rules.abandoned",
                tick = self.tick + 1,
                team.id = %self.teams[short].club_id,
                on_pitch = discipline::on_pitch_count(&self.players, short)
            );
            self.trace_abandoned(short);
            self.referee.abandoned = true;
            self.referee.phase = Phase::FullTime;
        }
    }

    /// Records a match abandoned because `short` fell below the rule pack's minimum.
    fn trace_abandoned(&mut self, short: usize) {
        if self.trace_on() {
            let on_pitch = discipline::on_pitch_count(&self.players, short);
            self.trace_point(
                Point::Abandoned,
                json!({"short_team": short, "on_pitch": on_pitch, "reason": "below_minimum"}),
            );
        }
    }

    /// Player `i` is injured and leaves play at once: the player stands beside the pitch like
    /// a sent-off player and the line closes up. In open play the referee stops play for a
    /// dropped ball (IFAB Law 8) to the team that last touched the ball, at the ball, or to
    /// that team's goalkeeper when the ball is inside its penalty area. When play already
    /// stopped on this tick (a foul), the injury rides that stoppage. The team's AI manager
    /// checks at once.
    pub(crate) fn injure(&mut self, i: usize, source: InjurySource) {
        let injuries = self.config.modules.injuries;
        if !injuries.leaves(&self.view(), i, source) {
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
        if self.trace_on() {
            let live = self.referee.phase == Phase::Live;
            self.trace_point(
                Point::Injury,
                json!({
                    "player": i,
                    "team": team,
                    "source": source.code(),
                    "injured": true,
                    "stops_play": live,
                }),
            );
        }
        let detail = Some(EventDetail::Injury { source });
        match self.referee.phase {
            Phase::Live => {
                let (with, spot) = injuries.dropped_ball(&self.view(), team);
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
                    dead.taker = self.config.modules.restarts.taker(
                        &self.view(),
                        dead.kind,
                        dead.team,
                        dead.spot,
                    );
                    self.referee.phase = Phase::DeadBall(dead);
                }
            }
            Phase::FullTime => {}
        }
        if let Some(short) = self.config.modules.clock.abandoned(&self.view()) {
            tracing::warn!(
                signal = "rules.abandoned",
                tick = self.tick + 1,
                team.id = %self.teams[short].club_id,
                on_pitch = discipline::on_pitch_count(&self.players, short)
            );
            self.trace_abandoned(short);
            self.referee.abandoned = true;
            self.referee.phase = Phase::FullTime;
        }
    }

    /// Shows every card held back for advantage.
    fn show_pending_cards(&mut self) {
        for pending in std::mem::take(&mut self.referee.pending) {
            self.show_card_as(pending.player, pending.card, true);
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
        self.restart_taker = None;
        self.clearers_tried = 0;
        self.end_shot();
        let since = self.tick + 1;
        let restarts = self.config.modules.restarts;
        let taker = restarts.taker(&self.view(), kind, team, spot);
        if self.trace_on() {
            let preferred = restart::preferred_taker(kind, team, spot, &self.teams);
            self.trace_point(
                Point::DeadBall,
                json!({
                    "kind": kind.code(),
                    "cause": cause.code(),
                    "team": team,
                    "spot": [spot.x, spot.y],
                    "direct": direct,
                }),
            );
            self.trace_point(
                Point::RestartTaker,
                json!({
                    "kind": kind.code(),
                    "team": team,
                    "taker": taker,
                    "preferred": preferred == Some(taker),
                }),
            );
        }
        let ready_at = since + restarts.delay_ticks(&self.view(), kind, team);
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
        let restarts = self.config.modules.restarts;
        for i in 0..self.players.len() {
            if self.players[i].active() {
                self.players[i].target = restarts.target(&self.view(), dead, i);
            }
        }
        let now = self.tick + 1;
        let forced = now >= dead.hard_limit();
        let ready = restarts.ready(&self.view(), dead, now);
        if !forced && !ready {
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

    /// The taker restarts play. A throw-in, a corner, a goal kick, an indirect free kick, and a
    /// penalty are kicked at once, and only the free kick can put a player offside; after any other
    /// restart the taker holds the ball and decides on the next tick.
    fn take_restart(&mut self, dead: &DeadBall, t: &Tuning) {
        if self.referee.shootout.is_some() {
            self.take_shootout_kick(dead, t);
            return;
        }
        self.referee.phase = Phase::Live;
        self.carrier = Some(dead.taker);
        self.control_since = self.tick;
        self.last_touch = Some(dead.team);
        self.restart_taker = Some(dead.taker);
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
            // IFAB Law 14: a penalty is a direct kick at goal from the mark, never a pass, a
            // dribble, or a held ball, and nobody may close the taker down before it is taken.
            StoppageKind::Penalty => {
                let goal = self.teams[dead.team].target_goal();
                let keeper = self.keeper(1 - dead.team);
                let kick = self.shot_kick(dead.taker, goal, keeper, t.shots.penalty_spread);
                self.trace_restart_taken(dead, "penalty");
                self.kick_ball(kick, t, false, true);
                return;
            }
            _ => None,
        };
        let label = match kick {
            Some((Kick::Pass { .. }, _)) => "pass",
            Some((Kick::Shot { .. }, _)) => "shot",
            Some((Kick::Clear { .. }, _)) => "clear",
            None => "none",
        };
        self.trace_restart_taken(dead, label);
        if let Some((kick, offside_counts)) = kick {
            let kick: Kick = kick;
            self.apply_kick(kick, t, offside_counts);
        }
    }

    /// Records a restart taken: its kind, the taker, and the kick.
    fn trace_restart_taken(&mut self, dead: &DeadBall, kick: &str) {
        if self.trace_on() {
            self.trace_point(
                Point::RestartTaken,
                json!({
                    "kind": dead.kind.code(),
                    "team": dead.team,
                    "taker": dead.taker,
                    "kick": kick,
                }),
            );
        }
    }

    /// The seconds added to period `period`: a regulation half or an extra-time period.
    pub(crate) fn period_added_s(&self, period: u32) -> u32 {
        let halves = self.referee.clock.halves;
        if period < halves {
            self.summary.added_s[(period as usize).min(1)]
        } else {
            self.summary.extra_added_s[((period - halves) as usize).min(1)]
        }
    }

    /// Ends the period when its own time plus added time has passed. Added time is fixed on
    /// the first tick after the period's own time. After the last regulation half a level
    /// knockout match goes on to extra time and, still level after it, to the shoot-out;
    /// every other match ends.
    pub(crate) fn check_clock(&mut self) {
        if self.is_over() || self.referee.shootout.is_some() {
            return;
        }
        let now = self.tick + 1;
        let mut clock = self.referee.clock;
        if !clock.regulation_over(now) {
            return;
        }
        let half = clock.half as usize;
        let rules = self.config.modules.clock;
        if clock.added_ticks[half].is_none() {
            let added_s = if !clock.plays_added {
                0
            } else if clock.in_extra_time() {
                let draw = self.streams.draw(Key::of_match(Action::ExtraTimeAdded));
                let seconds = rules.added_seconds(&self.view(), true, draw);
                tracing::info!(
                    signal = "rules.extra_time",
                    period = half,
                    tally_s = rules.tally_seconds(&self.view(), true),
                    added_s = seconds,
                    announced_min = clock::announced_minutes(seconds)
                );
                seconds
            } else {
                let draw = self.streams.draw(Key::of_match(Action::AddedTime));
                let seconds = rules.added_seconds(&self.view(), false, draw);
                let variance_s = f64::from(self.config.rules.added_time.variance_s);
                tracing::info!(
                    signal = "rules.added_time",
                    half,
                    tally_s = rules.tally_seconds(&self.view(), false),
                    variance_s = ((2.0 * draw - 1.0) * variance_s).round(),
                    added_s = seconds,
                    announced_min = clock::announced_minutes(seconds)
                );
                seconds
            };
            clock.added_ticks[half] = Some(added_s * TICKS_PER_SECOND);
            if clock.in_extra_time() {
                let period = half - clock.halves as usize;
                self.summary.extra_added_s[period.min(1)] = added_s;
            } else {
                self.summary.added_s[half.min(1)] = added_s;
            }
            self.referee.clock = clock;
            if self.trace_on() {
                let extra = clock.in_extra_time();
                let point = if extra {
                    Point::ExtraTimeAdded
                } else {
                    Point::AddedTime
                };
                let tally_s = rules.tally_seconds(&self.view(), extra);
                self.trace_point(
                    point,
                    json!({
                        "period": half,
                        "plays_added": clock.plays_added,
                        "tally_s": tally_s,
                        "seconds": added_s,
                        "announced_min": clock::announced_minutes(added_s),
                    }),
                );
            }
        }
        if clock.half_end().is_some_and(|end| now >= end) {
            match rules.period_end(&self.view()) {
                PeriodEnd::Break { recover } => self.half_time(recover),
                PeriodEnd::Shootout => self.start_shootout(),
                PeriodEnd::FullTime { decided_by } => {
                    if decided_by.is_some() {
                        self.summary.decided_by = decided_by;
                    }
                    self.referee.phase = Phase::FullTime;
                }
            }
        }
    }

    /// Half-time, and the breaks before and inside extra time: the teams change ends and the
    /// next period's kick-off is placed at once. The team that did not kick off the first half
    /// kicks off the second; a toss decides who kicks off extra time, and the other team kicks
    /// off its second period. Only the regulation half-time gives energy back (`recover`).
    fn half_time(&mut self, recover: bool) {
        let now = self.tick + 1;
        let half = self.referee.clock.half;
        let next = half + 1;
        let extra = next >= self.referee.clock.halves;
        let mut event = self.event(EngineEventKind::HalfTime, None);
        event.added_time_s = self
            .referee
            .clock
            .plays_added
            .then(|| self.period_added_s(half));
        if extra {
            event.period = Some(next);
        }
        self.events.push(event);
        if self.trace_on() {
            self.trace_point(
                Point::HalfTime,
                json!({"period": half, "next": next, "extra_time": extra, "recover": recover}),
            );
        }
        self.show_pending_cards();
        if self.referee.abandoned {
            return;
        }
        if recover {
            self.half_time_recovery();
        }
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
        let kick_off = if !extra {
            1
        } else if let Some(first) = self.referee.extra_kick_off {
            1 - first
        } else {
            self.summary.extra_time = true;
            let draw = self
                .streams
                .tested(Key::of_match(Action::ExtraKickOff), &[0.5]);
            let team = self.config.modules.clock.extra_kick_off(draw);
            self.referee.extra_kick_off = Some(team);
            if self.trace_on() {
                self.trace_point(Point::ExtraTimeKickOff, json!({ "team": team }));
            }
            team
        };
        self.place_kick_off(kick_off);
        if extra
            && let Some(event) = self.events.last_mut()
            && event.kind == EngineEventKind::KickOff
        {
            event.period = Some(next);
        }
    }

    /// Starts the penalty shoot-out after extra time: the players on the pitch take part, the
    /// larger side drops players until the numbers are equal, one toss decides who kicks
    /// first and a second toss the end every kick is taken at. The first kick is set up at
    /// once.
    fn start_shootout(&mut self) {
        let rules = self.config.modules.clock;
        let ShootoutLineup { order, keepers } = rules.shootout_lineup(&self.view());
        let first = rules.shootout_first(
            self.streams
                .tested(Key::of_match(Action::ShootoutFirstTeam), &[0.5]),
        );
        let end = rules.shootout_end(
            self.streams
                .tested(Key::of_match(Action::ShootoutEnd), &[0.5]),
        );
        if self.trace_on() {
            self.trace_point(
                Point::ShootoutOrder,
                json!({"order": order, "keepers": keepers}),
            );
            self.trace_point(
                Point::ShootoutStart,
                json!({"first_team": first, "end": end}),
            );
        }
        tracing::info!(
            signal = "rules.shootout_start",
            tick = self.tick + 1,
            first_team = first,
            kickers = order[0].len()
        );
        self.summary.shootout = Some([0, 0]);
        self.referee.shootout = Some(Shootout {
            order,
            keepers,
            cursor: [0, 0],
            first,
            end,
            scores: [0, 0],
            taken: [0, 0],
            kicker: None,
            live_since: None,
        });
        self.shootout_next_kick();
    }

    /// Sets up the next shoot-out kick: the kicking team's next player on the pitch in its
    /// order, at the penalty mark of the shoot-out end, after the unscaled penalty delay.
    fn shootout_next_kick(&mut self) {
        let Some(state) = self.referee.shootout.as_mut() else {
            return;
        };
        let team = state.kicking_team();
        let order = &state.order[team];
        let mut kicker = state.keepers[team];
        for step in 0..order.len() {
            let k = order[(state.cursor[team] + step) % order.len()];
            if self.players[k].active() {
                kicker = k;
                state.cursor[team] += step + 1;
                break;
            }
        }
        state.kicker = Some(kicker);
        state.live_since = None;
        let round = state.taken[team] + 1;
        let spot = pitch::penalty_spot(state.end);
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
        self.restart_taker = None;
        self.clearers_tried = 0;
        self.end_shot();
        let since = self.tick + 1;
        let delay = self
            .config
            .modules
            .restarts
            .shootout_delay_ticks(&self.view());
        self.referee.phase = Phase::DeadBall(DeadBall {
            kind: StoppageKind::Penalty,
            team,
            spot,
            direct: true,
            since,
            ready_at: since + delay,
            taker: kicker,
        });
        self.summary.stoppages += 1;
        let mut event = self.event(EngineEventKind::Penalty, Some(team));
        event.spot = Some(spot);
        event.player = Some(kicker);
        event.shootout_round = Some(round);
        self.events.push(event);
        self.stoppage = Some(Stoppage {
            tick: since,
            kind: StoppageKind::Penalty,
            team: Some(team),
            spot,
        });
    }

    /// The kicker shoots from the mark at once: a shoot-out kick is always a shot at goal.
    /// The defending keeper commits to a dive as the ball is struck: to one side of the goal,
    /// or rarely staying in the middle, from a seeded draw. Everyone else holds the place the
    /// kick was set up with.
    fn take_shootout_kick(&mut self, dead: &DeadBall, t: &Tuning) {
        let Some(state) = self.referee.shootout.as_mut() else {
            return;
        };
        state.live_since = Some(self.tick + 1);
        let keeper = state.keepers[1 - dead.team];
        let end = state.end;
        let goal = pitch::goal_centre(end);
        self.referee.phase = Phase::Live;
        let kick = self.shot_kick(dead.taker, goal, keeper, t.shots.penalty_spread);
        let diver = self.players[keeper];
        let dive = self
            .streams
            .range(Key::player(Action::KeeperDive, &diver), -1.0, 1.0);
        let side = self.config.modules.clock.keeper_dive(&self.view(), dive);
        if self.trace_on() {
            self.trace_point(
                Point::RestartTaken,
                json!({
                    "kind": "shootout_kick",
                    "team": dead.team,
                    "taker": dead.taker,
                    "kick": "shot",
                    "keeper": keeper,
                    "dive": side,
                }),
            );
        }
        self.players[keeper].target = DVec2::new(
            end * (pitch::HALF_LENGTH - 0.3),
            side * t.shots.keeper_dive_m,
        );
        let (dir, speed, loft) = kick.flight();
        self.ball = self
            .config
            .modules
            .ball
            .kick(&self.view(), self.ball, dir, speed, loft);
        self.end_shot();
        self.shot_on_target = self
            .config
            .modules
            .shot
            .on_target(&self.view(), self.ball, end);
        self.carrier = None;
        self.keeper_beaten = false;
        self.last_touch = Some(dead.team);
        self.last_kicker = Some(dead.taker);
        self.control_since = self.tick;
    }

    /// The ball of a shoot-out kick moved from `prev` to `xy`: a ball wholly inside the goal
    /// scores, and a ball that leaves play anywhere else misses.
    pub(crate) fn shootout_ball(&mut self, prev: DVec2, xy: DVec2, t: &Tuning) {
        let Some(end) = self.referee.shootout.as_ref().map(|s| s.end) else {
            return;
        };
        if pitch::in_goal(prev, xy, end) && self.ball.pos.z < t.crossbar_height {
            self.shootout_outcome(true, "goal");
        } else if pitch::exit(prev, xy).is_some() {
            self.shootout_outcome(false, "off_target");
        }
    }

    /// The defending keeper saves a shoot-out kick. Nobody else may touch it. A fast kick is
    /// saved only when it heads on target, with the save chance of a penalty, one roll per
    /// kick: a held save is a miss, and a parried ball plays on and may still go in. A kick
    /// the keeper does not save plays on. A slow ball the keeper reaches is his.
    pub(crate) fn shootout_save(&mut self, t: &Tuning) {
        let Some(state) = self.referee.shootout.as_ref() else {
            return;
        };
        let keeper = state.keepers[1 - state.kicking_team()];
        if self.ball.pos.z >= t.crossbar_height || !self.players[keeper].active() {
            return;
        }
        if (self.players[keeper].pos - self.ball.xy()).length() >= t.keeper_reach {
            return;
        }
        if self.ball.speed() > t.control_speed {
            if self.keeper_beaten || !self.shot_on_target {
                return;
            }
            let save = self
                .config
                .modules
                .shot
                .save_chance(&self.view(), t.shots.penalty_xg);
            let k = self.players[keeper];
            if self
                .streams
                .tested(Key::player(Action::ShootoutSave, &k), &[save])
                >= save
            {
                self.trace_save(Point::ShootoutSave, keeper, "beaten");
                self.keeper_beaten = true;
                return;
            }
            let hold = self.config.modules.clock.shootout_save_hold(&self.view());
            if self
                .streams
                .tested(Key::player(Action::ShootoutSaveHold, &k), &[hold])
                >= hold
            {
                self.trace_save(Point::ShootoutSave, keeper, "parried");
                self.parry(keeper, t);
                return;
            }
            self.trace_save(Point::ShootoutSave, keeper, "held");
        } else if self.ball.pos.z > t.reach_height {
            return;
        } else {
            self.trace_save(Point::ShootoutSave, keeper, "reached");
        }
        self.shootout_outcome(false, "keeper");
    }

    /// Ends a live shoot-out kick as a miss once the ball has stopped or the live cap of
    /// `KICK_LIVE_TICKS` has passed.
    pub(crate) fn shootout_kick_expiry(&mut self) {
        let Some(since) = self.referee.shootout.as_ref().and_then(|s| s.live_since) else {
            return;
        };
        let now = self.tick + 1;
        let stopped = self.ball.speed() < STOPPED_BALL_SPEED && now > since + 1;
        if stopped || now >= since + KICK_LIVE_TICKS {
            self.shootout_outcome(false, if stopped { "stopped" } else { "time" });
        }
    }

    /// Records the outcome of the kick in progress, then ends the match when the shoot-out is
    /// decided, abandons it after `SAFETY_ROUNDS` rounds, or sets up the next kick.
    fn shootout_outcome(&mut self, scored: bool, how: &str) {
        #[cfg(feature = "scenario")]
        let scored = self.forced_kicks.pop_front().unwrap_or(scored);
        let Some(state) = self.referee.shootout.as_mut() else {
            return;
        };
        let team = state.kicking_team();
        let kicker = state.kicker.take();
        state.live_since = None;
        state.taken[team] += 1;
        if scored {
            state.scores[team] += 1;
        }
        let (scores, taken) = (state.scores, state.taken);
        let round = taken[team];
        self.summary.shootout = Some(scores);
        self.summary.shootout_kicks += 1;
        self.carrier = None;
        tracing::info!(
            signal = "rules.shootout_kick",
            tick = self.tick + 1,
            team.id = %self.teams[team].club_id,
            round,
            scored,
            home.score = scores[0],
            away.score = scores[1]
        );
        let mut event = self.event(EngineEventKind::Penalty, Some(team));
        event.player = kicker;
        event.shootout_round = Some(round);
        event.shootout_scored = Some(scored);
        event.shootout_scores = Some(scores);
        self.events.push(event);
        if self.trace_on() {
            self.trace_point(
                Point::ShootoutKick,
                json!({
                    "team": team,
                    "kicker": kicker,
                    "round": round,
                    "scored": scored,
                    "how": how,
                    "scores": scores,
                }),
            );
        }
        if self
            .config
            .modules
            .clock
            .shootout_decided(&self.view(), scores, taken)
        {
            if self.trace_on() {
                let winner = usize::from(scores[1] > scores[0]);
                self.trace_point(
                    Point::ShootoutDecided,
                    json!({"scores": scores, "taken": taken, "winner": winner}),
                );
            }
            tracing::info!(
                signal = "rules.shootout_result",
                tick = self.tick + 1,
                home.score = scores[0],
                away.score = scores[1],
                kicks = taken[0] + taken[1]
            );
            self.summary.decided_by = Some(DecidedBy::Shootout);
            self.referee.phase = Phase::FullTime;
        } else if taken[0].min(taken[1]) >= shootout::SAFETY_ROUNDS {
            tracing::error!(
                signal = "rules.shootout_round_limit",
                tick = self.tick + 1,
                rounds = shootout::SAFETY_ROUNDS,
                home.score = scores[0],
                away.score = scores[1]
            );
            if self.trace_on() {
                self.trace_point(
                    Point::Abandoned,
                    json!({"reason": "shootout_round_limit", "scores": scores}),
                );
            }
            self.referee.abandoned = true;
            self.referee.phase = Phase::FullTime;
        } else {
            self.shootout_next_kick();
        }
    }
}

/// Ball speed, in metres per second, below which a shoot-out kick has stopped.
const STOPPED_BALL_SPEED: f64 = 0.3;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;

    /// A card held back for advantage is judged when the foul happens. When the player is
    /// booked in between, the held-back caution is the player's second and sends the player
    /// off (IFAB Law 12).
    #[test]
    fn a_held_back_caution_for_a_booked_player_is_a_second_yellow() {
        let mut sim = Simulation::new(shipped_config(1, 90).unwrap()).unwrap();
        let player = 16;
        let team = sim.players[player].team;
        sim.show_card(player, Card::Yellow);
        sim.show_card(player, Card::Yellow);
        let cards: Vec<_> = sim
            .events
            .iter()
            .filter(|e| e.kind == EngineEventKind::Card)
            .map(|e| e.card)
            .collect();
        assert_eq!(cards, vec![Some(Card::Yellow), Some(Card::SecondYellow)]);
        assert_eq!(sim.players[player].status, Status::SentOff);
        assert_eq!(sim.summary.red[team], 1);
        assert_eq!(sim.summary.yellow[team], 2);
    }

    /// A penalty in play is a direct kick at goal from the mark (IFAB Law 14): the taker
    /// shoots at once and never keeps the ball to pass, dribble, or hold.
    #[test]
    fn a_penalty_in_play_is_shot_at_goal_at_once() {
        let mut sim = Simulation::new(shipped_config(1, 90).unwrap()).unwrap();
        let team = 0;
        let taker = 9;
        let attack_x = sim.teams[team].attack_x;
        let spot = pitch::penalty_spot(attack_x);
        sim.players[taker].pos = spot;
        sim.ball = Ball::at(spot);
        let dead = DeadBall {
            kind: StoppageKind::Penalty,
            team,
            spot,
            direct: true,
            since: sim.tick,
            ready_at: sim.tick,
            taker,
        };
        let shots = sim.summary.shots[team];
        let t = sim.config.tuning.clone();
        sim.take_restart(&dead, &t);
        assert_eq!(sim.carrier, None, "the taker kicked the ball");
        assert_eq!(
            sim.summary.shots[team],
            shots + 1,
            "the kick counts as a shot"
        );
        assert!(
            sim.ball.vel.x * attack_x > 0.0,
            "the ball goes toward the goal the team attacks"
        );
    }
}
