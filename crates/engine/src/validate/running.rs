//! The running rule checker: the tick rules of [`Validator::check`] and the event rules of
//! [`Validator::check_events`], judged while the match plays. It keeps no tick list: each
//! record is judged when it arrives, and only small state stays (the team shapes in force,
//! the previous ball, each player's drift count, the restart ticks and spots, the
//! substitutions, the half-time ticks and the violations found). Three rules need the whole
//! match and are judged at [`RunningCheck::finish`]: `restart_spot` (an event naming a
//! restart's spot can come after the record in the stream), `full_time_last` (full time
//! comes after the last record) and the half-time exemption of `substitution_limits`.
//!
//! Its report equals the old validator's on the same match: the tick-rule violations of
//! [`Validator::check`], then the event-rule violations of [`Validator::check_events`],
//! each list in the same order with the same values.
//!
//! [`Validator::check`]: super::Validator::check
//! [`Validator::check_events`]: super::Validator::check_events

use std::collections::VecDeque;

use super::{MAX_BALL_SPEED, MIN_SEPARATION, RESTART_SPOT_TOLERANCE, StreamRules, Violation};
use crate::error::EngineError;
use crate::math::DVec2;
use crate::record::{PLAYER_COUNT, TickRecord, TickSink};
use crate::rules::fouls::Card;
use crate::sim::{EngineEvent, EngineEventKind, EventDetail, Simulation};
use crate::team::{PLAYERS_PER_TEAM, Team};
use crate::tuning::Tuning;

/// Where a violation sorts: the record or event index, then the rule's place in the order
/// the old validator pushes rules for one record or event.
type Key = (usize, u8);

/// Each tick rule's place among the rules of one record.
const IN_BOUNDS: u8 = 0;
const SEPARATION: u8 = 1;
const BALL_SPEED: u8 = 2;
const RESTART_SPOT: u8 = 3;
const ANCHOR_TOLERANCE: u8 = 4;

/// A squared distance a little above `MIN_SEPARATION²`: a pair at or beyond it is far
/// enough apart that its exact distance need not be taken.
const SEPARATION_BOUND_SQ: f64 = MIN_SEPARATION * MIN_SEPARATION * 1.0001;

/// Each event rule's place among the rules of one event.
const KICK_OFF_FIRST: u8 = 0;
const FULL_TIME_LAST: u8 = 1;
const PERIOD_END_TEAM: u8 = 2;
const SENT_OFF_SILENT: u8 = 3;
const PLAYER_SIDE: u8 = 4;
const ADDED_TIME: u8 = 5;
const CARD_KIND: u8 = 6;
const SUBSTITUTION_SQUAD: u8 = 7;
const SUBSTITUTION_LIMITS: u8 = 8;
const EVENT_ORDER: u8 = 9;
const GOAL_SCORE: u8 = 10;
const SCORE_KEPT: u8 = 11;

/// The tick, player and index of one event, as a violation of it reports them.
#[derive(Debug, Clone, Copy)]
struct At {
    index: usize,
    tick: u32,
    player: Option<usize>,
}

impl At {
    fn violation(self, rule: &'static str) -> Violation {
        Violation {
            tick: self.tick,
            rule,
            player: self.player,
            value: self.index as f64,
        }
    }
}

/// One accepted substitution, kept for the window rule at finish.
#[derive(Debug, Clone, Copy)]
struct Sub {
    at: At,
    team: usize,
    /// The side's substitutions so far, this one included.
    used: u32,
}

/// Checks a match's records and events as they arrive. Feed it in tick order: for each
/// tick, every team shape and event of that tick, then the tick's record; full time last.
/// [`RunningCheck::for_match`] and its [`TickSink`] face do that for a played match.
pub struct RunningCheck {
    tuning: Tuning,
    rules: StreamRules,

    // Tick rules.
    shapes: [Team; 2],
    pending_shapes: VecDeque<(u32, [Team; 2])>,
    records: usize,
    prev_ball: Option<DVec2>,
    far_ticks: [u32; PLAYER_COUNT],
    prev_dist: [[f64; 2]; PLAYER_COUNT],
    shootout_from: Option<u32>,
    /// Each restart record: its index, tick and ball.
    restarts: Vec<(usize, u32, DVec2)>,
    /// The restart spots the events name, in stream order.
    spots: Vec<(u32, DVec2)>,
    tick_found: Vec<(Key, Violation)>,

    // Event rules.
    events: usize,
    prev_event: Option<(u32, [u32; 2])>,
    last_event: Option<(At, bool)>,
    first_full_time: Option<At>,
    full_times: u32,
    half_times: Vec<u32>,
    lineup: [[usize; PLAYERS_PER_TEAM]; 2],
    played: [Vec<usize>; 2],
    booked: [bool; PLAYER_COUNT],
    sent_off: [bool; PLAYER_COUNT],
    used: [u32; 2],
    subs: Vec<Sub>,
    shootout: bool,
    event_found: Vec<(Key, Violation)>,

    // The live face's place in the match's timeline and event list.
    timeline_seen: usize,
    events_seen: usize,
}

impl RunningCheck {
    /// Every rule it judges: the five tick rules, then the twelve event rules.
    pub const RULES: [&'static str; 17] = [
        "in_bounds",
        "separation",
        "ball_speed",
        "restart_spot",
        "anchor_tolerance",
        "kick_off_first",
        "full_time_last",
        "period_end_team",
        "sent_off_silent",
        "player_side",
        "added_time",
        "card_kind",
        "substitution_squad",
        "substitution_limits",
        "event_order",
        "goal_score",
        "score_kept",
    ];

    /// A checker for a match judged by `tuning` and `rules`, whose teams start in `teams`.
    pub fn new(tuning: Tuning, teams: [Team; 2], rules: StreamRules) -> Self {
        Self {
            tuning,
            shapes: teams,
            pending_shapes: VecDeque::new(),
            records: 0,
            prev_ball: None,
            far_ticks: [0; PLAYER_COUNT],
            prev_dist: [[f64::INFINITY; 2]; PLAYER_COUNT],
            shootout_from: None,
            restarts: Vec::new(),
            spots: Vec::new(),
            tick_found: Vec::new(),
            events: 0,
            prev_event: None,
            last_event: None,
            first_full_time: None,
            full_times: 0,
            half_times: Vec::new(),
            lineup: rules.lineups,
            played: rules.lineups.map(|l| l.to_vec()),
            booked: [false; PLAYER_COUNT],
            sent_off: [false; PLAYER_COUNT],
            used: [0; 2],
            subs: Vec::new(),
            shootout: false,
            event_found: Vec::new(),
            timeline_seen: 1,
            events_seen: 0,
            rules,
        }
    }

    /// A checker for `sim` before its first step: its tuning, its team shapes at kick-off,
    /// and `rules`. Run the match into it, then call [`RunningCheck::finish_match`] before
    /// the match's events are taken.
    pub fn for_match(sim: &Simulation, rules: StreamRules) -> Self {
        let teams = sim.team_timeline()[0].1.clone();
        let mut check = Self::new(sim.tuning().clone(), teams, rules);
        check.catch_up(sim);
        check
    }

    /// The records checked so far.
    pub fn ticks(&self) -> u32 {
        u32::try_from(self.records).unwrap_or(u32::MAX)
    }

    /// The team shapes in force from `tick` on.
    pub fn shapes(&mut self, tick: u32, teams: [Team; 2]) {
        self.pending_shapes.push_back((tick, teams));
    }

    /// The next event of the stream.
    pub fn event(&mut self, e: &EngineEvent) {
        if let Some(spot) = e.spot {
            self.spots.push((e.tick, spot));
        }
        if e.shootout_round.is_some() {
            self.shootout_from = Some(self.shootout_from.map_or(e.tick, |t| t.min(e.tick)));
        }

        let i = self.events;
        self.events += 1;
        let at = At {
            index: i,
            tick: e.tick,
            player: e.player,
        };
        let found = &mut self.event_found;
        let mut bad = |rank: u8, rule: &'static str| found.push(((i, rank), at.violation(rule)));
        if i == 0 && !(e.kind == EngineEventKind::KickOff && e.scores == [0, 0]) {
            bad(KICK_OFF_FIRST, "kick_off_first");
        }
        let is_full_time = e.kind == EngineEventKind::FullTime;
        if is_full_time {
            self.full_times += 1;
            self.first_full_time.get_or_insert(at);
        }
        self.last_event = Some((at, is_full_time));
        if e.kind == EngineEventKind::HalfTime {
            self.half_times.push(e.tick);
        }

        let side_of = |index: usize| index / PLAYERS_PER_TEAM;
        self.shootout |= e.shootout_round.is_some();
        if matches!(
            e.kind,
            EngineEventKind::HalfTime | EngineEventKind::FullTime
        ) && e.team.is_some()
        {
            bad(PERIOD_END_TEAM, "period_end_team");
        }
        if [e.player, e.secondary]
            .iter()
            .flatten()
            .any(|&p| p < PLAYER_COUNT && self.sent_off[p])
        {
            bad(SENT_OFF_SILENT, "sent_off_silent");
        }
        if let Some(team) = e.team {
            let own = |p: usize| p < PLAYER_COUNT && side_of(p) == team;
            let other = |p: usize| p < PLAYER_COUNT && side_of(p) != team;
            let player_ok = e
                .player
                .is_none_or(|p| own(p) || (e.kind == EngineEventKind::Goal && p < PLAYER_COUNT));
            if team > 1 || !player_ok || !e.secondary.is_none_or(other) {
                bad(PLAYER_SIDE, "player_side");
            }
        }
        match (e.kind, e.added_time_s) {
            (_, None) => {}
            (EngineEventKind::HalfTime | EngineEventKind::FullTime, Some(s)) => {
                let ok = match self.rules.added_s {
                    Some((lo, hi)) => (lo..=hi).contains(&s),
                    None => s == 0,
                };
                if !ok {
                    bad(ADDED_TIME, "added_time");
                }
            }
            (_, Some(_)) => bad(ADDED_TIME, "added_time"),
        }
        if e.kind == EngineEventKind::Card {
            match (e.card, e.player) {
                (Some(card), Some(p)) if p < PLAYER_COUNT => {
                    if card == Card::SecondYellow && !self.booked[p] {
                        bad(CARD_KIND, "card_kind");
                    }
                    if card == Card::Yellow {
                        self.booked[p] = true;
                    } else {
                        self.sent_off[p] = true;
                    }
                }
                _ => bad(CARD_KIND, "card_kind"),
            }
        }
        if e.kind == EngineEventKind::Substitution {
            match (e.detail, e.team, e.player) {
                (Some(EventDetail::Substitution { off, on }), Some(team), Some(p))
                    if team < 2 && p < PLAYER_COUNT && side_of(p) == team =>
                {
                    let slot = p % PLAYERS_PER_TEAM;
                    if self.lineup[team][slot] != off || self.played[team].contains(&on) {
                        bad(SUBSTITUTION_SQUAD, "substitution_squad");
                    }
                    self.lineup[team][slot] = on;
                    self.played[team].push(on);
                    self.booked[p] = false;
                    self.used[team] += 1;
                    // The window rule needs every half-time tick of the match: judged at
                    // finish.
                    self.subs.push(Sub {
                        at,
                        team,
                        used: self.used[team],
                    });
                }
                _ => bad(SUBSTITUTION_SQUAD, "substitution_squad"),
            }
        }
        let prev = self.prev_event.replace((e.tick, e.scores));
        let Some((prev_tick, prev_scores)) = prev else {
            return;
        };
        if e.tick < prev_tick {
            bad(EVENT_ORDER, "event_order");
        }
        if e.kind == EngineEventKind::Goal && !self.shootout {
            let mut expected = prev_scores;
            match e.team {
                Some(team) if team < 2 => expected[team] += 1,
                _ => {
                    bad(GOAL_SCORE, "goal_score");
                    return;
                }
            }
            if e.scores != expected {
                bad(GOAL_SCORE, "goal_score");
            }
        } else if e.scores != prev_scores {
            bad(SCORE_KEPT, "score_kept");
        }
    }

    /// The next record. Every team shape and event of its tick must be fed before it.
    pub fn tick(&mut self, r: &TickRecord) {
        while let Some((from, _)) = self.pending_shapes.front()
            && *from <= r.tick
        {
            let (_, teams) = self.pending_shapes.pop_front().expect("a front entry");
            self.shapes = teams;
        }
        let idx = self.records;
        self.records += 1;
        let teams = &self.shapes;
        let pitch = teams[0].pitch();
        let ball = DVec2::new(f64::from(r.ball[0]), f64::from(r.ball[1]));
        let pos: [DVec2; PLAYER_COUNT] = std::array::from_fn(|i| {
            DVec2::new(f64::from(r.players[i][0]), f64::from(r.players[i][1]))
        });
        let parked: [bool; PLAYER_COUNT] = std::array::from_fn(|i| pitch.is_parking_spot(pos[i]));
        let found = &mut self.tick_found;
        let mut bad = |rank: u8, rule: &'static str, player: Option<usize>, value: f64| {
            found.push((
                (idx, rank),
                Violation {
                    tick: r.tick,
                    rule,
                    player,
                    value,
                },
            ));
        };

        for (i, p) in pos.iter().enumerate() {
            if !parked[i] && !pitch.contains(*p) {
                bad(IN_BOUNDS, "in_bounds", Some(i), p.x.abs().max(p.y.abs()));
            }
        }
        for i in 0..PLAYER_COUNT {
            if parked[i] {
                continue;
            }
            for j in (i + 1)..PLAYER_COUNT {
                if parked[j] {
                    continue;
                }
                // `length()` is the square root of `length_squared()` (source: glam 0.33.8
                // `src/f64/dvec2.rs`), so a squared distance at or above the bound has a
                // length above `MIN_SEPARATION`: only a pair below it takes the square
                // root, through the same `length()` call, so its value and decision are
                // the old validator's.
                let gap = pos[i] - pos[j];
                if gap.length_squared() >= SEPARATION_BOUND_SQ {
                    continue;
                }
                let d = gap.length();
                if d < MIN_SEPARATION {
                    bad(SEPARATION, "separation", Some(i), d);
                }
            }
        }
        if let Some(prev) = self.prev_ball
            && !r.restart
        {
            let speed = (ball - prev).length() / self.tuning.dt;
            if speed > MAX_BALL_SPEED + 0.5 {
                bad(BALL_SPEED, "ball_speed", None, speed);
            }
        }
        self.prev_ball = Some(ball);
        if r.restart {
            // An event naming this tick's spot can come after the record: judged at finish.
            self.restarts.push((idx, r.tick, ball));
        }

        let shootout = self.shootout_from.is_some_and(|from| r.tick >= from);
        for (i, p) in pos.iter().enumerate() {
            if parked[i] || shootout {
                self.far_ticks[i] = 0;
                continue;
            }
            let team = i / PLAYERS_PER_TEAM;
            let slot = i % PLAYERS_PER_TEAM;
            let anchor = teams[team].anchor(slot, ball, &self.tuning);
            let d_ball = (*p - ball).length();
            let d_anchor = (*p - anchor).length();
            let prev_dist = self.prev_dist[i];
            let closing = d_ball < prev_dist[0] - 1e-3 || d_anchor < prev_dist[1] - 1e-3;
            self.prev_dist[i] = [d_ball, d_anchor];
            let drifting = d_ball > self.tuning.anchor_ball_distance
                && d_anchor > self.tuning.anchor_tolerance
                && !closing;
            self.far_ticks[i] = if drifting { self.far_ticks[i] + 1 } else { 0 };
            if self.far_ticks[i] >= self.tuning.anchor_grace_ticks {
                bad(ANCHOR_TOLERANCE, "anchor_tolerance", Some(i), d_anchor);
            }
        }
    }

    /// Judges the rules that need the whole match and returns every violation: the tick
    /// rules' in record order, then the event rules' in stream order, as
    /// [`super::Validator::check`] and [`super::Validator::check_events`] return them.
    pub fn finish(mut self) -> Vec<Violation> {
        // The same stable sort and lookup as `Validator::for_match` and `check`, so a tick
        // with two spots resolves to the same one.
        self.spots.sort_by_key(|(tick, _)| *tick);
        for &(idx, tick, ball) in &self.restarts {
            if let Ok(at) = self.spots.binary_search_by_key(&tick, |(t, _)| *t) {
                let off = (ball - self.spots[at].1).length();
                if off > RESTART_SPOT_TOLERANCE {
                    self.tick_found.push((
                        (idx, RESTART_SPOT),
                        Violation {
                            tick,
                            rule: "restart_spot",
                            player: None,
                            value: off,
                        },
                    ));
                }
            }
        }

        if let Some((last, last_is_full_time)) = self.last_event {
            if !(self.full_times == 1 && last_is_full_time) {
                let at = self.first_full_time.unwrap_or(last);
                self.event_found
                    .push(((at.index, FULL_TIME_LAST), at.violation("full_time_last")));
            }
            let mut windows: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
            for sub in &self.subs {
                let tick = sub.at.tick;
                let exempt = self.rules.half_time_exempt && self.half_times.contains(&tick);
                if !exempt && !windows[sub.team].contains(&tick) {
                    windows[sub.team].push(tick);
                }
                if sub.used > self.rules.substitutions
                    || windows[sub.team].len() as u32 > self.rules.windows
                {
                    self.event_found.push((
                        (sub.at.index, SUBSTITUTION_LIMITS),
                        sub.at.violation("substitution_limits"),
                    ));
                }
            }
        }

        self.tick_found.sort_by_key(|(key, _)| *key);
        self.event_found.sort_by_key(|(key, _)| *key);
        for (_, v) in &self.tick_found {
            tracing::warn!(signal = "validate.violation", tick = v.tick, rule = v.rule, player = ?v.player, value = v.value);
        }
        self.tick_found
            .into_iter()
            .chain(self.event_found)
            .map(|(_, v)| v)
            .collect()
    }

    /// Feeds every team shape and event `sim` added since the last call.
    fn catch_up(&mut self, sim: &Simulation) {
        let timeline = sim.team_timeline();
        debug_assert!(timeline.len() >= self.timeline_seen, "the timeline shrank");
        for (tick, teams) in timeline.get(self.timeline_seen..).unwrap_or_default() {
            self.shapes(*tick, teams.clone());
        }
        self.timeline_seen = self.timeline_seen.max(timeline.len());
        let events = sim.events();
        debug_assert!(
            events.len() >= self.events_seen,
            "the events were taken while the match played"
        );
        for e in events.get(self.events_seen..).unwrap_or_default() {
            self.event(e);
        }
        self.events_seen = self.events_seen.max(events.len());
    }

    /// Feeds what `sim` added after its last record (full time) and finishes. Call it
    /// before the match's events are taken.
    pub fn finish_match(mut self, sim: &Simulation) -> Vec<Violation> {
        self.catch_up(sim);
        self.finish()
    }
}

impl TickSink for RunningCheck {
    fn on_step(&mut self, sim: &Simulation) -> Result<(), EngineError> {
        self.catch_up(sim);
        Ok(())
    }

    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.tick(record);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::Validator;
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

    fn rules() -> StreamRules {
        StreamRules {
            substitutions: 5,
            windows: 3,
            half_time_exempt: true,
            added_s: None,
            lineups: [std::array::from_fn(|s| s), std::array::from_fn(|s| s)],
        }
    }

    /// The running checker fed in tick order: before each record, every shape and event
    /// whose tick is not after the record's, in their own order; the rest after the last.
    fn running(
        timeline: &[(u32, [Team; 2])],
        events: &[EngineEvent],
        records: &[TickRecord],
    ) -> Vec<Violation> {
        let mut check = RunningCheck::new(Tuning::default(), timeline[0].1.clone(), rules());
        let (mut shape, mut event) = (1, 0);
        for r in records {
            while shape < timeline.len() && timeline[shape].0 <= r.tick {
                check.shapes(timeline[shape].0, timeline[shape].1.clone());
                shape += 1;
            }
            while event < events.len() && events[event].tick <= r.tick {
                check.event(&events[event]);
                event += 1;
            }
            check.tick(r);
        }
        for (tick, teams) in &timeline[shape..] {
            check.shapes(*tick, teams.clone());
        }
        for e in &events[event..] {
            check.event(e);
        }
        check.finish()
    }

    /// The old validator's report on the same input: tick rules, then event rules.
    fn old(
        timeline: &[(u32, [Team; 2])],
        events: &[EngineEvent],
        records: &[TickRecord],
    ) -> Vec<Violation> {
        let mut out = Validator::for_match(Tuning::default(), timeline, events).check(records);
        out.extend(Validator::check_events(events, &rules()));
        out
    }

    fn same(timeline: &[(u32, [Team; 2])], events: &[EngineEvent], records: &[TickRecord]) {
        assert_eq!(
            running(timeline, events, records),
            old(timeline, events, records)
        );
    }

    fn teams() -> Vec<(u32, [Team; 2])> {
        vec![(0, [bare(0), bare(1)])]
    }

    #[test]
    fn todays_five_validator_cases_give_the_same_report() {
        let mut overlap = record(10);
        overlap.players[1] = overlap.players[0];
        let out = running(&teams(), &[], &[record(9), overlap]);
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].rule, out[0].tick), ("separation", 10));
        same(&teams(), &[], &[record(9), overlap]);

        let mut fast = record(2);
        fast.ball = [0.9, 0.0, 0.0];
        assert_eq!(
            running(&teams(), &[], &[record(1), fast])[0].rule,
            "ball_speed"
        );
        same(&teams(), &[], &[record(1), fast]);

        let mut jump = record(2);
        jump.ball = [20.0, 0.0, 0.0];
        let mut restart = jump;
        restart.restart = true;
        assert!(running(&teams(), &[], &[record(1), restart]).is_empty());
        same(&teams(), &[], &[record(1), jump]);

        let mut parked = record(2);
        let spot = crate::pitch::Pitch::DEFAULT.parking_spot(0, 3);
        parked.players[3] = [spot.x as f32, spot.y as f32];
        assert!(running(&teams(), &[], &[record(1), parked]).is_empty());

        let config = crate::data::test_support::shipped_config(1, 90).unwrap();
        let sim = Simulation::new(config).unwrap();
        let mut throw_in = sim.event(EngineEventKind::ThrowIn, Some(0));
        throw_in.tick = 2;
        throw_in.spot = Some(DVec2::new(10.0, 34.0));
        let mut on_spot = record(2);
        on_spot.restart = true;
        on_spot.ball = [10.0, 34.0, 0.0];
        let mut off_spot = on_spot;
        off_spot.ball = [11.0, 34.0, 0.0];
        let tick_rules = |out: Vec<Violation>| -> Vec<&'static str> {
            out.iter()
                .map(|v| v.rule)
                .filter(|r| *r == "restart_spot")
                .collect()
        };
        assert!(tick_rules(running(&teams(), &[throw_in], &[record(1), on_spot])).is_empty());
        assert_eq!(
            tick_rules(running(&teams(), &[throw_in], &[record(1), off_spot])),
            ["restart_spot"]
        );
        same(&teams(), &[throw_in], &[record(1), off_spot]);
    }

    #[test]
    fn two_shapes_on_one_tick_end_on_the_later_one() {
        let mut moved = bare(0);
        moved.attack_x = -moved.attack_x;
        let timeline = [
            (0, [bare(0), bare(1)]),
            (5, [moved.clone(), bare(1)]),
            (5, [bare(0), bare(1)]),
        ];
        let mut check = RunningCheck::new(Tuning::default(), timeline[0].1.clone(), rules());
        for (tick, teams) in &timeline[1..] {
            check.shapes(*tick, teams.clone());
        }
        check.tick(&record(5));
        assert_eq!(check.shapes[0].attack_x, bare(0).attack_x);
        assert!(check.pending_shapes.is_empty());
    }

    #[test]
    fn a_spot_event_after_its_restart_record_still_judges_the_restart() {
        let config = crate::data::test_support::shipped_config(1, 90).unwrap();
        let sim = Simulation::new(config).unwrap();
        let mut throw_in = sim.event(EngineEventKind::ThrowIn, Some(0));
        throw_in.tick = 2;
        throw_in.spot = Some(DVec2::new(10.0, 34.0));
        let mut off_spot = record(2);
        off_spot.restart = true;
        off_spot.ball = [11.0, 34.0, 0.0];
        let mut check = RunningCheck::new(Tuning::default(), [bare(0), bare(1)], rules());
        check.tick(&record(1));
        check.tick(&off_spot);
        check.event(&throw_in);
        let out = check.finish();
        assert!(
            out.iter()
                .any(|v| v.rule == "restart_spot" && v.tick == 2 && (v.value - 1.0).abs() < 1e-6),
            "{out:?}"
        );
    }

    #[test]
    fn the_shoot_out_exempts_the_anchor_rule_from_its_first_kick() {
        let tuning = Tuning::default();
        let grace = tuning.anchor_grace_ticks;
        // Player 5 stands still far from its anchor and from the ball.
        let far = |tick: u32| {
            let mut r = record(tick);
            r.players[5] = [-50.0, -30.0];
            r.ball = [40.0, 30.0, 0.0];
            r
        };
        let records: Vec<TickRecord> = (1..=grace + 20).map(far).collect();
        let anchors = |events: &[EngineEvent]| {
            running(&teams(), events, &records)
                .iter()
                .filter(|v| v.rule == "anchor_tolerance")
                .count()
        };
        assert!(anchors(&[]) > 0, "the plant drifts without a shoot-out");
        let config = crate::data::test_support::shipped_config(1, 90).unwrap();
        let sim = Simulation::new(config).unwrap();
        let mut kick = sim.event(EngineEventKind::Penalty, Some(0));
        kick.tick = 1;
        kick.shootout_round = Some(1);
        assert_eq!(anchors(&[kick]), 0);
        same(&teams(), &[kick], &records);
        same(&teams(), &[], &records);
    }
}
