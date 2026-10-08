//! The tick-stream validator: pitch bounds, player separation, ball speed, formation anchor
//! tolerance in open play, and the ball's place at a restart. The anchor rule flags a player
//! that is far from the ball, far from its anchor, and closing on neither for the grace
//! period (idle drift). A sent-off or injured player at its parking spot beside the pitch is
//! exempt from every player rule. A substitute enters at the halfway line on the touchline,
//! inside the pitch, on a restart tick; no rule limits a player's jump, so the entry breaks
//! none. The anchors follow the team shapes in force at each tick: the teams change ends at
//! half-time, a line closes up after a sending-off or an injury and reopens when a
//! substitute replaces the injured player, and a tactics change moves every anchor. From the
//! first kick of a penalty shoot-out on, the players wait in the centre circle and the anchor
//! rule no longer applies.
//!
//! [`Validator::check_events`] judges a match's event stream on its own rules, whatever
//! produced it: the full engine or the fast model, which has no ticks to check.

use crate::data::rules::StoppageKind;
use crate::math::DVec2;
use crate::record::{PLAYER_COUNT, TickRecord};
use crate::rules::clock::plays_added_time;
use crate::rules::fouls::Card;
use crate::sim::{EngineEvent, EngineEventKind, EventDetail, MatchConfig};
use crate::team::{PLAYERS_PER_TEAM, Team};
use crate::tuning::Tuning;

mod running;
pub use running::RunningCheck;

/// Minimum distance between two players.
pub const MIN_SEPARATION: f64 = 0.1;
/// Maximum ball speed in metres per second.
pub const MAX_BALL_SPEED: f64 = 40.0;
/// How far the ball may lie from the restart spot a restart event names.
pub const RESTART_SPOT_TOLERANCE: f64 = 0.5;

/// One rule violation.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub tick: u32,
    pub rule: &'static str,
    pub player: Option<usize>,
    pub value: f64,
}

/// What the event-stream rules read beside the events: the rule pack's substitution limits
/// and added time, and each side's line-up at kick-off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamRules {
    /// Substitutions a side may make in the match.
    pub substitutions: u32,
    /// Stoppages at which a side may substitute, half-time aside when it is exempt.
    pub windows: u32,
    /// `true` when a substitution at half-time uses no window.
    pub half_time_exempt: bool,
    /// The seconds a period may add, `(min, max)`, or `None` when the match is shorter than
    /// regulation and adds no time.
    pub added_s: Option<(u32, u32)>,
    /// The squad index in each roster slot at kick-off, home first.
    pub lineups: [[usize; PLAYERS_PER_TEAM]; 2],
}

impl StreamRules {
    /// The rules of the match `config` describes. A knockout match may also use the extra
    /// time's substitution and window, and its extra-time periods add at most the extra
    /// time's cap.
    pub fn for_config(config: &MatchConfig) -> Self {
        let rules = &config.rules;
        let extra = &rules.extra_time;
        let (more, more_windows) = if config.knockout {
            (
                u32::from(extra.extra_substitutions),
                u32::from(extra.extra_windows),
            )
        } else {
            (0, 0)
        };
        Self {
            substitutions: u32::from(rules.substitutions.limit) + more,
            windows: u32::from(rules.substitutions.windows) + more_windows,
            half_time_exempt: rules.substitutions.exempt(StoppageKind::HalfTime),
            added_s: plays_added_time(config.minutes, rules).then(|| {
                let added = &rules.added_time;
                let lo = if config.knockout {
                    added.min_s.min(extra.added_max_s)
                } else {
                    added.min_s
                };
                (lo, added.max_s)
            }),
            lineups: [config.teams[0].lineup, config.teams[1].lineup],
        }
    }
}

/// Validates a stream of records against the tuning and the team formations.
pub struct Validator {
    tuning: Tuning,
    /// The team shapes in force from each tick on, in tick order.
    timeline: Vec<(u32, [Team; 2])>,
    /// The restart spots the events name, in tick order.
    spots: Vec<(u32, DVec2)>,
    /// The tick of the first shoot-out kick, if the match went to a shoot-out.
    shootout_from: Option<u32>,
}

impl Validator {
    /// A validator for teams whose shape never changes.
    pub fn new(tuning: Tuning, teams: [Team; 2]) -> Self {
        Self {
            tuning,
            timeline: vec![(0, teams)],
            spots: Vec::new(),
            shootout_from: None,
        }
    }

    /// A validator for a played match: the team shapes over time and the events, whose
    /// restart spots the ball must lie on at each restart tick.
    pub fn for_match(
        tuning: Tuning,
        timeline: &[(u32, [Team; 2])],
        events: &[EngineEvent],
    ) -> Self {
        let mut spots: Vec<(u32, DVec2)> = events
            .iter()
            .filter_map(|e| e.spot.map(|s| (e.tick, s)))
            .collect();
        spots.sort_by_key(|(tick, _)| *tick);
        let shootout_from = events
            .iter()
            .filter(|e| e.shootout_round.is_some())
            .map(|e| e.tick)
            .min();
        Self {
            tuning,
            timeline: timeline.to_vec(),
            spots,
            shootout_from,
        }
    }

    /// Returns every violation in `records`, in tick order.
    pub fn check(&self, records: &[TickRecord]) -> Vec<Violation> {
        let mut out = Vec::new();
        let mut far_ticks = [0u32; PLAYER_COUNT];
        let mut prev_dist = [[f64::INFINITY; 2]; PLAYER_COUNT];
        let mut prev_ball: Option<DVec2> = None;
        let mut shape = 0;
        for r in records {
            while shape + 1 < self.timeline.len() && self.timeline[shape + 1].0 <= r.tick {
                shape += 1;
            }
            let teams = &self.timeline[shape].1;
            let pitch = teams[0].pitch();
            let ball = DVec2::new(f64::from(r.ball[0]), f64::from(r.ball[1]));
            // Fixed-size, so a whole match of records allocates nothing per tick.
            let pos: [DVec2; PLAYER_COUNT] = std::array::from_fn(|i| {
                DVec2::new(f64::from(r.players[i][0]), f64::from(r.players[i][1]))
            });
            let parked: [bool; PLAYER_COUNT] =
                std::array::from_fn(|i| pitch.is_parking_spot(pos[i]));

            for (i, p) in pos.iter().enumerate() {
                if !parked[i] && !pitch.contains(*p) {
                    out.push(Violation {
                        tick: r.tick,
                        rule: "in_bounds",
                        player: Some(i),
                        value: p.x.abs().max(p.y.abs()),
                    });
                }
            }
            for i in 0..PLAYER_COUNT {
                for j in (i + 1)..PLAYER_COUNT {
                    if parked[i] || parked[j] {
                        continue;
                    }
                    let d = (pos[i] - pos[j]).length();
                    if d < MIN_SEPARATION {
                        out.push(Violation {
                            tick: r.tick,
                            rule: "separation",
                            player: Some(i),
                            value: d,
                        });
                    }
                }
            }
            if let Some(prev) = prev_ball
                && !r.restart
            {
                let speed = (ball - prev).length() / self.tuning.dt;
                if speed > MAX_BALL_SPEED + 0.5 {
                    out.push(Violation {
                        tick: r.tick,
                        rule: "ball_speed",
                        player: None,
                        value: speed,
                    });
                }
            }
            prev_ball = Some(ball);
            if r.restart
                && let Ok(at) = self.spots.binary_search_by_key(&r.tick, |(tick, _)| *tick)
            {
                let off = (ball - self.spots[at].1).length();
                if off > RESTART_SPOT_TOLERANCE {
                    out.push(Violation {
                        tick: r.tick,
                        rule: "restart_spot",
                        player: None,
                        value: off,
                    });
                }
            }

            let shootout = self.shootout_from.is_some_and(|from| r.tick >= from);
            for (i, p) in pos.iter().enumerate() {
                if parked[i] || shootout {
                    far_ticks[i] = 0;
                    continue;
                }
                let team = i / PLAYERS_PER_TEAM;
                let slot = i % PLAYERS_PER_TEAM;
                let anchor = teams[team].anchor(slot, ball, &self.tuning);
                let d_ball = (*p - ball).length();
                let d_anchor = (*p - anchor).length();
                let closing = d_ball < prev_dist[i][0] - 1e-3 || d_anchor < prev_dist[i][1] - 1e-3;
                prev_dist[i] = [d_ball, d_anchor];
                let drifting = d_ball > self.tuning.anchor_ball_distance
                    && d_anchor > self.tuning.anchor_tolerance
                    && !closing;
                far_ticks[i] = if drifting { far_ticks[i] + 1 } else { 0 };
                if far_ticks[i] >= self.tuning.anchor_grace_ticks {
                    out.push(Violation {
                        tick: r.tick,
                        rule: "anchor_tolerance",
                        player: Some(i),
                        value: d_anchor,
                    });
                }
            }
        }
        for v in &out {
            tracing::warn!(signal = "validate.violation", tick = v.tick, rule = v.rule, player = ?v.player, value = v.value);
        }
        out
    }

    /// Returns every violation of the event-stream rules in `events`, in stream order. The
    /// rules hold for any producer of a match's events, the full engine or the fast model:
    ///
    /// - `event_order`: no event is stamped before the one before it;
    /// - `kick_off_first`: the first event is a kick-off with the score 0-0 (the full engine
    ///   kicks off on tick 1, the fast model on tick 0);
    /// - `full_time_last`: exactly one full time, and it is the last event;
    /// - `goal_score`: a goal names a team and adds one to that team's score, the other
    ///   score unchanged (a shoot-out kick keeps the score of play);
    /// - `score_kept`: every other event keeps the score;
    /// - `period_end_team`: half-time and full time name no team;
    /// - `player_side`: a named player is in a roster slot of the event's team, and a fouled
    ///   player in one of the other team (a goal's scorer may be either: an own goal);
    /// - `card_kind`: a card event carries its card and a player, and a second yellow follows
    ///   a yellow to the player in that roster slot;
    /// - `sent_off_silent`: a player sent off is named by no later event;
    /// - `substitution_squad`: a substitution takes off the squad player in that roster slot
    ///   and brings on a squad player who has not played;
    /// - `substitution_limits`: a side makes at most `rules.substitutions` substitutions, at
    ///   most `rules.windows` stoppages, half-time aside when it is exempt;
    /// - `added_time`: only half-time and full time carry added seconds, within the rule
    ///   pack's range (none in a match shorter than regulation).
    ///
    /// `value` holds the event's index in the stream.
    pub fn check_events(events: &[EngineEvent], rules: &StreamRules) -> Vec<Violation> {
        let mut out = Vec::new();
        let mut bad = |i: usize, rule: &'static str| {
            out.push(Violation {
                tick: events[i].tick,
                rule,
                player: events[i].player,
                value: i as f64,
            });
        };
        match events.first() {
            Some(first) if first.kind == EngineEventKind::KickOff && first.scores == [0, 0] => {}
            Some(_) => bad(0, "kick_off_first"),
            None => return Vec::new(),
        }
        let full_times: Vec<usize> = (0..events.len())
            .filter(|&i| events[i].kind == EngineEventKind::FullTime)
            .collect();
        if full_times != [events.len() - 1] {
            bad(
                full_times.first().copied().unwrap_or(events.len() - 1),
                "full_time_last",
            );
        }
        let half_times: Vec<u32> = events
            .iter()
            .filter(|e| e.kind == EngineEventKind::HalfTime)
            .map(|e| e.tick)
            .collect();
        let side_of = |index: usize| index / PLAYERS_PER_TEAM;
        // The squad player in each roster slot now, every squad player who has played, the
        // booked and the sent-off roster slots, and each side's substitutions and windows.
        let mut lineup = rules.lineups;
        let mut played: [Vec<usize>; 2] = rules.lineups.map(|l| l.to_vec());
        let mut booked = [false; PLAYER_COUNT];
        let mut sent_off = [false; PLAYER_COUNT];
        let mut used = [0u32; 2];
        let mut windows: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
        let mut shootout = false;
        for (i, e) in events.iter().enumerate() {
            shootout |= e.shootout_round.is_some();
            if matches!(
                e.kind,
                EngineEventKind::HalfTime | EngineEventKind::FullTime
            ) && e.team.is_some()
            {
                bad(i, "period_end_team");
            }
            if [e.player, e.secondary]
                .iter()
                .flatten()
                .any(|&p| p < PLAYER_COUNT && sent_off[p])
            {
                bad(i, "sent_off_silent");
            }
            if let Some(team) = e.team {
                let own = |p: usize| p < PLAYER_COUNT && side_of(p) == team;
                let other = |p: usize| p < PLAYER_COUNT && side_of(p) != team;
                let player_ok = e.player.is_none_or(|p| {
                    own(p) || (e.kind == EngineEventKind::Goal && p < PLAYER_COUNT)
                });
                if team > 1 || !player_ok || !e.secondary.is_none_or(other) {
                    bad(i, "player_side");
                }
            }
            match (e.kind, e.added_time_s) {
                (_, None) => {}
                (EngineEventKind::HalfTime | EngineEventKind::FullTime, Some(s)) => {
                    let ok = match rules.added_s {
                        Some((lo, hi)) => (lo..=hi).contains(&s),
                        None => s == 0,
                    };
                    if !ok {
                        bad(i, "added_time");
                    }
                }
                (_, Some(_)) => bad(i, "added_time"),
            }
            if e.kind == EngineEventKind::Card {
                match (e.card, e.player) {
                    (Some(card), Some(p)) if p < PLAYER_COUNT => {
                        if card == Card::SecondYellow && !booked[p] {
                            bad(i, "card_kind");
                        }
                        if card == Card::Yellow {
                            booked[p] = true;
                        } else {
                            sent_off[p] = true;
                        }
                    }
                    _ => bad(i, "card_kind"),
                }
            }
            if e.kind == EngineEventKind::Substitution {
                match (e.detail, e.team, e.player) {
                    (Some(EventDetail::Substitution { off, on }), Some(team), Some(p))
                        if team < 2 && p < PLAYER_COUNT && side_of(p) == team =>
                    {
                        let slot = p % PLAYERS_PER_TEAM;
                        if lineup[team][slot] != off || played[team].contains(&on) {
                            bad(i, "substitution_squad");
                        }
                        lineup[team][slot] = on;
                        played[team].push(on);
                        booked[p] = false;
                        used[team] += 1;
                        let exempt = rules.half_time_exempt && half_times.contains(&e.tick);
                        if !exempt && !windows[team].contains(&e.tick) {
                            windows[team].push(e.tick);
                        }
                        if used[team] > rules.substitutions
                            || windows[team].len() as u32 > rules.windows
                        {
                            bad(i, "substitution_limits");
                        }
                    }
                    _ => bad(i, "substitution_squad"),
                }
            }
            let Some(prev) = i.checked_sub(1).map(|p| &events[p]) else {
                continue;
            };
            if e.tick < prev.tick {
                bad(i, "event_order");
            }
            if e.kind == EngineEventKind::Goal && !shootout {
                let mut expected = prev.scores;
                match e.team {
                    Some(team) if team < 2 => expected[team] += 1,
                    _ => {
                        bad(i, "goal_score");
                        continue;
                    }
                }
                if e.scores != expected {
                    bad(i, "goal_score");
                }
            } else if e.scores != prev.scores {
                bad(i, "score_kept");
            }
        }
        out.sort_by_key(|v| v.value as usize);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team::test_support::bare;

    fn record(tick: u32) -> TickRecord {
        let teams = [bare(0), bare(1)];
        let mut players = [[0.0f32; 2]; PLAYER_COUNT];
        for (i, p) in players.iter_mut().enumerate() {
            let base = teams[i / PLAYERS_PER_TEAM].slot_base(i % PLAYERS_PER_TEAM);
            *p = [base.x as f32, base.y as f32];
        }
        TickRecord {
            tick,
            ball: [0.0, 0.0, 0.0],
            players,
            restart: false,
        }
    }

    #[test]
    fn a_hand_built_overlap_is_one_violation() {
        let v = Validator::new(Tuning::default(), [bare(0), bare(1)]);
        let mut r = record(10);
        r.players[1] = r.players[0];
        let out = v.check(&[record(9), r]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule, "separation");
        assert_eq!(out[0].tick, 10);
    }

    #[test]
    fn a_ball_faster_than_the_cap_is_a_violation() {
        let v = Validator::new(Tuning::default(), [bare(0), bare(1)]);
        let mut r = record(2);
        r.ball = [0.9, 0.0, 0.0];
        let out = v.check(&[record(1), r]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule, "ball_speed");
    }

    #[test]
    fn a_restart_tick_exempts_the_jump_and_the_same_jump_without_the_flag_does_not() {
        let v = Validator::new(Tuning::default(), [bare(0), bare(1)]);
        let mut jump = record(2);
        jump.ball = [20.0, 0.0, 0.0];

        let mut restart = jump;
        restart.restart = true;
        assert!(v.check(&[record(1), restart]).is_empty());

        let out = v.check(&[record(1), jump]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].rule, "ball_speed");
    }

    #[test]
    fn a_parked_player_breaks_no_rule() {
        let v = Validator::new(Tuning::default(), [bare(0), bare(1)]);
        let mut r = record(2);
        let spot = crate::pitch::Pitch::DEFAULT.parking_spot(0, 3);
        r.players[3] = [spot.x as f32, spot.y as f32];
        assert!(v.check(&[record(1), r]).is_empty());
    }

    #[test]
    fn a_ball_away_from_the_named_restart_spot_is_a_violation() {
        let config = crate::data::test_support::shipped_config(1, 90).unwrap();
        let sim = crate::sim::Simulation::new(config).unwrap();
        let mut event = sim.event(crate::sim::EngineEventKind::ThrowIn, Some(0));
        event.tick = 2;
        event.spot = Some(DVec2::new(10.0, 34.0));
        let v = Validator::for_match(Tuning::default(), &[(0, [bare(0), bare(1)])], &[event]);
        let mut restart = record(2);
        restart.restart = true;
        restart.ball = [10.0, 34.0, 0.0];
        assert!(v.check(&[record(1), restart]).is_empty());
        restart.ball = [11.0, 34.0, 0.0];
        let out = v.check(&[record(1), restart]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].rule, "restart_spot");
    }
}
