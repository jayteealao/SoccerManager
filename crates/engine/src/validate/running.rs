//! The running rule checker: the tick rules of [`Validator::check`] and the event rules of
//! [`Validator::check_events`], judged while the match plays. It keeps no tick list: each
//! record is judged when it arrives, and only small state stays (the team shapes in force,
//! the previous ball and positions, each player's drift count, the restart ticks and spots,
//! the substitutions, the half-time ticks and the violations found). Three rules need the whole
//! match and are judged at [`RunningCheck::finish`]: `restart_spot` (an event naming a
//! restart's spot can come after the record in the stream), `full_time_last` (full time
//! comes after the last record) and the half-time exemption of `substitution_limits`.
//!
//! Its report equals the old validator's on the same match: the tick-rule violations of
//! [`Validator::check`], then the event-rule violations of [`Validator::check_events`],
//! each list in the same order with the same values. It keeps at most
//! [`RunningCheck::KEPT_PER_RULE`] violations of each rule, the first ones, and only counts
//! the rest: a broken match can break a rule hundreds of times per tick, and a list of all
//! of them would fill memory and the log. Under that cap the kept list is the old report.
//!
//! [`Validator::check`]: super::Validator::check
//! [`Validator::check_events`]: super::Validator::check_events

use std::collections::VecDeque;

use super::{
    BallRest, JOB_NEAR_M, MAX_BALL_REST_S, MAX_BALL_SPEED, MIN_SEPARATION, RESTART_SPOT_TOLERANCE,
    RunSpell, StreamRules, Violation, job_closing, job_distance,
};
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
const PLAYER_SPEED: u8 = 3;
const RESTART_SPOT: u8 = 4;
const ANCHOR_TOLERANCE: u8 = 5;
const BALL_REST: u8 = 6;
/// The tick rules' count: the first places of [`RunningCheck::RULES`].
const TICK_RULES: usize = 7;

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
/// The event rules' count: the places of [`RunningCheck::RULES`] after the tick rules.
const EVENT_RULES: usize = 12;

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

/// The violations of one family of rules (tick or event) found so far: the first
/// [`RunningCheck::KEPT_PER_RULE`] of each rule with its sort key, and each rule's count.
/// `N` is the family's rule count; a key's rank is the rule's place in it.
struct Found<const N: usize> {
    kept: Vec<(Key, Violation)>,
    counts: [usize; N],
}

impl<const N: usize> Found<N> {
    fn new() -> Self {
        Self {
            kept: Vec::new(),
            counts: [0; N],
        }
    }

    /// Counts `v` against its rule and keeps it while the rule is under the cap. Each
    /// rule's violations arrive in key order, so the kept ones are the rule's first.
    fn push(&mut self, key: Key, v: Violation) {
        let count = &mut self.counts[usize::from(key.1)];
        *count += 1;
        if *count <= RunningCheck::KEPT_PER_RULE {
            self.kept.push((key, v));
        }
    }
}

/// A match's violations as the running checker reports them: the kept ones in the old
/// validator's order, and every rule's total count, kept or not.
#[derive(Debug, Clone, PartialEq)]
pub struct Violations {
    kept: Vec<Violation>,
    totals: [usize; RunningCheck::RULES.len()],
}

impl Violations {
    /// Every violation found, kept or not.
    pub fn len(&self) -> usize {
        self.totals.iter().sum()
    }

    /// True when no rule was broken.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The kept violations: at most [`RunningCheck::KEPT_PER_RULE`] of each rule, the
    /// first ones; the tick rules' in record order, then the event rules' in stream order.
    pub fn kept(&self) -> &[Violation] {
        &self.kept
    }

    /// Each rule of [`RunningCheck::RULES`] with its total count, kept or not.
    pub fn totals(&self) -> impl Iterator<Item = (&'static str, usize)> + '_ {
        RunningCheck::RULES.into_iter().zip(self.totals)
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
    /// The previous record's player positions and which of them were parked.
    prev_players: Option<([DVec2; PLAYER_COUNT], [bool; PLAYER_COUNT])>,
    /// The most a player may move between two records ([`max_step`]).
    max_step: f64,
    /// The roster slot each substitution names, with its tick: the substitute enters there.
    entries: Vec<(u32, usize)>,
    /// The spot of the restart whose ball has not yet appeared at it: from a restart event
    /// until a record with the restart flag or with the ball at the spot. The anchor rule
    /// reads it as the ball.
    fetch_spot: Option<DVec2>,
    /// The last record judged: its tick, its restart flag and the ball. An event of a tick
    /// already judged (the stream sends a tick's events after its record) names a spot that
    /// record has already read, as [`super::Validator::check`] reads it: a restart taken on
    /// that record, or a ball already at the spot, leaves nothing to fetch.
    last_record: Option<(u32, bool, DVec2)>,
    far_ticks: [u32; PLAYER_COUNT],
    /// Each player's run state for the anchor rule ([RunSpell]).
    runs: [RunSpell; PLAYER_COUNT],
    prev_dist: [[f64; 3]; PLAYER_COUNT],
    /// The ball the anchor rule read on the previous record and every player's position
    /// then: a player who moves toward where the ball was is closing on it, however fast the
    /// ball runs away from him.
    prev_chase: Option<(DVec2, [DVec2; PLAYER_COUNT])>,
    shootout_from: Option<u32>,
    /// Each restart record: its index, tick and ball.
    restarts: Vec<(usize, u32, DVec2)>,
    /// The restart spots the events name, in stream order.
    spots: Vec<(u32, DVec2)>,
    /// The ball-rest rule's dead spell and still count.
    rest: BallRest,
    rest_limit: u32,
    tick_found: Found<TICK_RULES>,

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
    event_found: Found<EVENT_RULES>,

    // The live face's place in the match's timeline and event list.
    timeline_seen: usize,
    events_seen: usize,
}

impl RunningCheck {
    /// Every rule it judges: the seven tick rules, then the twelve event rules.
    pub const RULES: [&'static str; 19] = [
        "in_bounds",
        "separation",
        "ball_speed",
        "player_speed",
        "restart_spot",
        "anchor_tolerance",
        "ball_rest",
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

    /// The most violations of one rule a checker keeps. Past it, only the rule's count
    /// grows.
    pub const KEPT_PER_RULE: usize = 64;

    /// A checker for a match judged by `tuning` and `rules`, whose teams start in `teams`.
    pub fn new(tuning: Tuning, teams: [Team; 2], rules: StreamRules) -> Self {
        let rest_limit = BallRest::limit_ticks(&tuning);
        Self {
            max_step: max_step(&tuning),
            tuning,
            shapes: teams,
            pending_shapes: VecDeque::new(),
            records: 0,
            prev_ball: None,
            prev_players: None,
            entries: Vec::new(),
            fetch_spot: None,
            last_record: None,
            far_ticks: [0; PLAYER_COUNT],
            runs: [RunSpell::default(); PLAYER_COUNT],
            prev_dist: [[f64::INFINITY; 3]; PLAYER_COUNT],
            prev_chase: None,
            shootout_from: None,
            restarts: Vec::new(),
            spots: Vec::new(),
            rest: BallRest::default(),
            rest_limit,
            tick_found: Found::new(),
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
            event_found: Found::new(),
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
            let fetched = self.last_record.is_some_and(|(tick, restart, ball)| {
                tick >= e.tick
                    && (restart
                        || tick == super::OPENING_RECORD_TICK
                        || (ball - spot).length() <= RESTART_SPOT_TOLERANCE)
            });
            if !fetched {
                self.fetch_spot = Some(spot);
            }
            self.rest.stopped();
        }
        if e.kind == EngineEventKind::Substitution
            && let Some(p) = e.player
        {
            self.entries.push((e.tick, p));
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
        let mut bad = |rank: u8, rule: &'static str| found.push((i, rank), at.violation(rule));
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
            found.push(
                (idx, rank),
                Violation {
                    tick: r.tick,
                    rule,
                    player,
                    value,
                },
            );
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
        let shootout = self.shootout_from.is_some_and(|from| r.tick >= from);
        if self
            .rest
            .record(self.prev_ball, ball, r.restart, shootout, self.rest_limit)
        {
            bad(BALL_REST, "ball_rest", None, MAX_BALL_REST_S);
        }
        self.prev_ball = Some(ball);
        // The first record of a half places the kick-off: no jump is judged on it.
        let half_start = self.half_times.last() == Some(&r.tick);
        if let Some((prev, prev_parked)) = &self.prev_players
            && !half_start
        {
            for i in 0..PLAYER_COUNT {
                let entered = self.entries.contains(&(r.tick, i));
                if parked[i] || prev_parked[i] || entered {
                    continue;
                }
                let step = (pos[i] - prev[i]).length();
                if step > self.max_step {
                    bad(PLAYER_SPEED, "player_speed", Some(i), step / self.tuning.dt);
                }
            }
        }
        self.prev_players = Some((pos, parked));
        if r.restart {
            // An event naming this tick's spot can come after the record: judged at finish.
            self.restarts.push((idx, r.tick, ball));
        }
        // While the ball waits where it left play, the players set up around the restart
        // spot: the anchor rule reads the spot as the ball until the ball appears there.
        let reference = self.fetch_spot.unwrap_or(ball);
        if r.restart
            || r.tick == super::OPENING_RECORD_TICK
            || (ball - reference).length() <= RESTART_SPOT_TOLERANCE
        {
            self.fetch_spot = None;
        }
        self.last_record = Some((r.tick, r.restart, ball));

        for (i, p) in pos.iter().enumerate() {
            if parked[i] || shootout {
                self.far_ticks[i] = 0;
                self.runs[i] = RunSpell::default();
                continue;
            }
            let team = i / PLAYERS_PER_TEAM;
            let slot = i % PLAYERS_PER_TEAM;
            // The record does not say which team has the ball, so the nearer of the slot's
            // two anchors counts.
            let anchors =
                [true, false].map(|has| teams[team].anchor(slot, reference, has, &self.tuning));
            let nearer = nearer_anchor(anchors, *p);
            let d_anchor = (*p - nearer).length().min(kick_off_place_distance(
                &teams[team],
                slot,
                *p,
                reference,
            ));
            let d_ball = (*p - reference).length();
            let d_job = job_distance(&pos, &parked, i, teams, nearer, &self.tuning);
            let prev_dist = self.prev_dist[i];
            let toward_ball = self
                .prev_chase
                .is_some_and(|(was, at)| (*p - was).length() < (at[i] - was).length() - 1e-3);
            let far = d_ball > self.tuning.anchor_ball_distance
                && d_anchor > self.tuning.anchor_tolerance
                && d_job > JOB_NEAR_M;
            let on_run = self.runs[i].closing(
                self.prev_chase.map(|(_, at)| at[i]),
                *p,
                teams[team].attack_x,
                far.then_some(nearer),
                self.max_step,
                &self.tuning,
            );
            let toward_job = self.prev_chase.is_some_and(|(_, at)| {
                job_closing(
                    &at,
                    *p,
                    prev_dist[2],
                    &parked,
                    i,
                    teams,
                    nearer,
                    &self.tuning,
                )
            });
            let closing = d_ball < prev_dist[0] - 1e-3
                || d_anchor < prev_dist[1] - 1e-3
                || toward_job
                || toward_ball
                || on_run;
            self.prev_dist[i] = [d_ball, d_anchor, d_job];
            let drifting = far && !closing;
            self.far_ticks[i] = if drifting { self.far_ticks[i] + 1 } else { 0 };
            if self.far_ticks[i] >= self.tuning.anchor_grace_ticks {
                bad(ANCHOR_TOLERANCE, "anchor_tolerance", Some(i), d_anchor);
            }
        }
        self.prev_chase = Some((reference, pos));
    }

    /// Judges the rules that need the whole match and returns the violations: the kept
    /// ones, the tick rules' in record order, then the event rules' in stream order, as
    /// [`super::Validator::check`] and [`super::Validator::check_events`] return them; and
    /// every rule's total. It logs each kept tick-rule violation, and one line with the
    /// total for each tick rule past the cap.
    pub fn finish(mut self) -> Violations {
        // The same stable sort and lookup as `Validator::for_match` and `check`, so a tick
        // with two spots resolves to the same one: each restart record is judged against the
        // latest restart spot named at or before its tick, since the ball may appear at the
        // spot some ticks after the stoppage named it.
        self.spots.sort_by_key(|(tick, _)| *tick);
        for &(idx, tick, ball) in &self.restarts {
            if let Some(spot) = latest_spot(&self.spots, tick) {
                let off = (ball - spot).length();
                if off > RESTART_SPOT_TOLERANCE {
                    self.tick_found.push(
                        (idx, RESTART_SPOT),
                        Violation {
                            tick,
                            rule: "restart_spot",
                            player: None,
                            value: off,
                        },
                    );
                }
            }
        }

        if let Some((last, last_is_full_time)) = self.last_event {
            if !(self.full_times == 1 && last_is_full_time) {
                let at = self.first_full_time.unwrap_or(last);
                self.event_found
                    .push((at.index, FULL_TIME_LAST), at.violation("full_time_last"));
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
                    self.event_found.push(
                        (sub.at.index, SUBSTITUTION_LIMITS),
                        sub.at.violation("substitution_limits"),
                    );
                }
            }
        }

        self.tick_found.kept.sort_by_key(|(key, _)| *key);
        self.event_found.kept.sort_by_key(|(key, _)| *key);
        for (_, v) in &self.tick_found.kept {
            tracing::warn!(signal = "validate.violation", tick = v.tick, rule = v.rule, player = ?v.player, value = v.value);
        }
        for (rule, &total) in Self::RULES.iter().zip(&self.tick_found.counts) {
            if total > Self::KEPT_PER_RULE {
                tracing::warn!(
                    signal = "validate.violations_capped",
                    rule,
                    total,
                    kept = Self::KEPT_PER_RULE
                );
            }
        }
        let mut totals = [0; Self::RULES.len()];
        let (tick, event) = totals.split_at_mut(TICK_RULES);
        tick.copy_from_slice(&self.tick_found.counts);
        event.copy_from_slice(&self.event_found.counts);
        Violations {
            kept: self
                .tick_found
                .kept
                .into_iter()
                .chain(self.event_found.kept)
                .map(|(_, v)| v)
                .collect(),
            totals,
        }
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
    pub fn finish_match(mut self, sim: &Simulation) -> Violations {
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

/// The most a player may move between two consecutive records: the top speed of the tuning's
/// speed map (pace 1 to 20) for one tick, plus the minimum player distance, the most one
/// separation push moves him.
pub(crate) fn max_step(t: &Tuning) -> f64 {
    let top = (10..=200)
        .map(|tenths| t.contract.speed.top_speed(f64::from(tenths) / 10.0))
        .fold(0.0, f64::max);
    top * t.dt + t.min_player_distance
}

/// The spot of the latest restart named at or before `tick` in `spots` (sorted by tick; of two
/// on one tick, the later in the list).
pub(crate) fn latest_spot(spots: &[(u32, DVec2)], tick: u32) -> Option<DVec2> {
    let after = spots.partition_point(|(t, _)| *t <= tick);
    after.checked_sub(1).map(|at| spots[at].1)
}

/// While the ball waits on the centre spot for a kick-off, the referee stands each player on
/// his kick-off place: the formation slot in his own half (Law 8). It counts as a third
/// reference for the anchor rule, so a player standing there is not drifting even when his
/// stand for the phase lies far from his slot. In play the ball is never exactly on the
/// centre spot for long, and the reference is infinitely far.
pub(crate) fn kick_off_place_distance(team: &Team, slot: usize, at: DVec2, ball: DVec2) -> f64 {
    if ball == DVec2::ZERO {
        (at - crate::rules::restart::kick_off_position(team, slot)).length()
    } else {
        f64::INFINITY
    }
}

/// The nearer to `at` of a slot's two anchors (with the ball, then without): the record does
/// not say which team has the ball. Of two at the same distance, the first.
pub(crate) fn nearer_anchor(anchors: [DVec2; 2], at: DVec2) -> DVec2 {
    if (at - anchors[1]).length() < (at - anchors[0]).length() {
        anchors[1]
    } else {
        anchors[0]
    }
}

#[cfg(test)]
mod tests {
    use super::super::{MAX_RUN_S, Validator};
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
    ) -> Violations {
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

    /// The old report as the running checker keeps it: the first `KEPT_PER_RULE` of each
    /// rule in the report's order, and each rule's total.
    fn capped(old: &[Violation]) -> (Vec<Violation>, Vec<(&'static str, usize)>) {
        let mut seen = [0usize; RunningCheck::RULES.len()];
        let place = |rule| RunningCheck::RULES.iter().position(|r| *r == rule).unwrap();
        let kept = old
            .iter()
            .filter(|v| {
                let n = &mut seen[place(v.rule)];
                *n += 1;
                *n <= RunningCheck::KEPT_PER_RULE
            })
            .cloned()
            .collect();
        let totals = RunningCheck::RULES
            .map(|rule| (rule, old.iter().filter(|v| v.rule == rule).count()))
            .to_vec();
        (kept, totals)
    }

    fn same(timeline: &[(u32, [Team; 2])], events: &[EngineEvent], records: &[TickRecord]) {
        let new = running(timeline, events, records);
        let old = old(timeline, events, records);
        assert_eq!(new.len(), old.len());
        assert_eq!((new.kept().to_vec(), new.totals().collect()), capped(&old));
    }

    fn teams() -> Vec<(u32, [Team; 2])> {
        vec![(0, [bare(0), bare(1)])]
    }

    #[test]
    fn todays_five_validator_cases_give_the_same_report() {
        let mut overlap = record(10);
        overlap.players[1] = overlap.players[0];
        // The overlap is the first record, so player 1 has no step to judge: a jump onto
        // player 0 from the record before would also break the player speed rule.
        let out = running(&teams(), &[], &[overlap]);
        assert_eq!(out.len(), 1);
        assert_eq!((out.kept()[0].rule, out.kept()[0].tick), ("separation", 10));
        same(&teams(), &[], &[overlap]);

        let mut fast = record(2);
        fast.ball = [0.9, 0.0, 0.0];
        assert_eq!(
            running(&teams(), &[], &[record(1), fast]).kept()[0].rule,
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
        let tick_rules = |out: Violations| -> Vec<&'static str> {
            out.kept()
                .iter()
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

    /// The ball appears at its spot seconds after the event that named it, and the
    /// restart record is judged against that spot: on it, it passes; 2 m off, `restart_spot`
    /// reports it. The control: a lookup of a spot named on the record's own tick, as before
    /// the fetch, finds none and would judge nothing.
    #[test]
    fn a_restart_record_seconds_after_its_event_is_judged_against_that_spot() {
        let config = crate::data::test_support::shipped_config(1, 90).unwrap();
        let sim = Simulation::new(config).unwrap();
        let spot = DVec2::new(10.0, 34.0);
        let mut throw_in = sim.event(EngineEventKind::ThrowIn, Some(0));
        throw_in.tick = 2;
        throw_in.spot = Some(spot);
        let fetched = 2 + 600;
        let mut on_spot = record(fetched);
        on_spot.restart = true;
        on_spot.ball = [10.0, 34.0, 0.0];
        let mut off_spot = on_spot;
        off_spot.ball = [12.0, 34.0, 0.0];
        let spot_rule = |out: Violations| -> Vec<(u32, f64)> {
            out.kept()
                .iter()
                .filter(|v| v.rule == "restart_spot")
                .map(|v| (v.tick, v.value))
                .collect()
        };
        let records = |last: TickRecord| [record(1), record(2), last];
        assert!(spot_rule(running(&teams(), &[throw_in], &records(on_spot))).is_empty());
        let found = spot_rule(running(&teams(), &[throw_in], &records(off_spot)));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0, fetched);
        assert!((found[0].1 - 2.0).abs() < 1e-6);
        same(&teams(), &[throw_in], &records(off_spot));
        // The control: the spot of an event on the record's own tick.
        let spots = [(2u32, spot)];
        let exact = spots.iter().find(|(t, _)| *t == fetched).map(|(_, s)| *s);
        assert_eq!(exact, None, "an exact-tick lookup would judge nothing");
        assert_eq!(latest_spot(&spots, fetched), Some(spot));
    }

    /// A player who moves 5 m in one tick breaks `player_speed`; the first record of a
    /// half and a substitute's slot on the tick he comes on are not judged.
    #[test]
    fn a_one_tick_jump_breaks_the_player_speed_rule_except_where_play_places_players() {
        let config = crate::data::test_support::shipped_config(1, 90).unwrap();
        let sim = Simulation::new(config).unwrap();
        let mut jump = record(2);
        jump.players[4][0] += 5.0;
        let speed = |out: Violations| -> Vec<(u32, Option<usize>)> {
            out.kept()
                .iter()
                .filter(|v| v.rule == "player_speed")
                .map(|v| (v.tick, v.player))
                .collect()
        };
        assert_eq!(
            speed(running(&teams(), &[], &[record(1), jump])),
            [(2, Some(4))]
        );
        same(&teams(), &[], &[record(1), jump]);
        let mut half_time = sim.event(EngineEventKind::HalfTime, None);
        half_time.tick = 2;
        assert!(speed(running(&teams(), &[half_time], &[record(1), jump])).is_empty());
        let mut sub = sim.event(EngineEventKind::Substitution, Some(0));
        sub.tick = 2;
        sub.player = Some(4);
        assert!(speed(running(&teams(), &[sub], &[record(1), jump])).is_empty());
        sub.player = Some(5);
        assert_eq!(
            speed(running(&teams(), &[sub], &[record(1), jump])),
            [(2, Some(4))],
            "another slot's substitution does not cover the jump"
        );
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
            out.kept()
                .iter()
                .any(|v| v.rule == "restart_spot" && v.tick == 2 && (v.value - 1.0).abs() < 1e-6),
            "{out:?}"
        );
    }

    /// A player far from his anchor runs after a ball that rolls away from him faster than he
    /// runs: he is closing on it (he moves toward where it was) and breaks no anchor rule. The
    /// control: the same player standing still breaks it once the grace has passed.
    #[test]
    fn a_player_running_after_a_faster_ball_is_closing_on_it() {
        let tuning = Tuning::default();
        let grace = tuning.anchor_grace_ticks;
        let chase = |run: f64| {
            move |tick: u32| {
                let mut r = record(tick);
                let t = f64::from(tick);
                r.players[5] = [(-50.0 + run * t) as f32, -30.0];
                r.ball = [(0.0 + 0.2 * t) as f32, 30.0, 0.0];
                r
            }
        };
        let anchors = |records: &[TickRecord]| {
            running(&teams(), &[], records)
                .totals()
                .find(|(rule, _)| *rule == "anchor_tolerance")
                .unwrap()
                .1
        };
        let running_after: Vec<TickRecord> = (1..=grace + 20).map(chase(0.1)).collect();
        assert_eq!(anchors(&running_after), 0, "a chaser is closing");
        same(&teams(), &[], &running_after);
        let standing: Vec<TickRecord> = (1..=grace + 20).map(chase(0.0)).collect();
        assert!(anchors(&standing) > 0, "a still player drifts");
        same(&teams(), &[], &standing);
    }

    /// The opening kick-off is taken on the step that writes the match's first record, so no
    /// record shows the ball waiting at the centre spot: the anchor rule reads the ball from
    /// that record on, and every player standing on his anchor breaks no rule, however long
    /// the ball stays away from the centre. The control: the same kick-off spot named later in
    /// the match, with the ball never at it, still stands for the ball.
    #[test]
    fn the_opening_kick_off_spot_does_not_stand_for_the_ball_once_play_starts() {
        let tuning = Tuning::default();
        let ax = bare(0).attack_x;
        let b = DVec2::new(ax * 52.0, 20.0);
        let on_anchor = bare(0).anchor(5, b, true, &tuning);
        let records = run_scene(1..=tuning.anchor_grace_ticks + 20, |_| b, |_| on_anchor);
        let config = crate::data::test_support::shipped_config(1, 90).unwrap();
        let sim = Simulation::new(config).unwrap();
        let kick_off = |tick: u32| {
            let mut e = sim.event(EngineEventKind::KickOff, Some(0));
            e.tick = tick;
            e.spot = Some(DVec2::ZERO);
            e
        };
        let anchors = |events: &[EngineEvent]| {
            same(&teams(), events, &records);
            running(&teams(), events, &records)
                .totals()
                .find(|(rule, _)| *rule == "anchor_tolerance")
                .unwrap()
                .1
        };
        assert_eq!(anchors(&[kick_off(super::super::OPENING_RECORD_TICK)]), 0);
        assert!(anchors(&[kick_off(5)]) > 0);
    }

    /// Records of a scene for the run rule: the ball at `ball(k)` on record `k`, every player
    /// but player 5 standing on his anchor, and player 5 at `at(k)`.
    fn run_scene(
        ticks: std::ops::RangeInclusive<u32>,
        ball: impl Fn(u32) -> DVec2,
        at: impl Fn(u32) -> DVec2,
    ) -> Vec<TickRecord> {
        let tuning = Tuning::default();
        let teams = [bare(0), bare(1)];
        ticks
            .map(|k| {
                let b = ball(k);
                let mut r = record(k);
                r.ball = [b.x as f32, b.y as f32, 0.0];
                for (i, p) in r.players.iter_mut().enumerate() {
                    let a =
                        teams[i / PLAYERS_PER_TEAM].anchor(i % PLAYERS_PER_TEAM, b, true, &tuning);
                    *p = [a.x as f32, a.y as f32];
                }
                let p = at(k);
                r.players[5] = [p.x as f32, p.y as f32];
                r
            })
            .collect()
    }

    /// Player 5's anchor-rule violations in `records`, after a check that the two checkers
    /// agree; no other player breaks the rule.
    fn run_scene_anchors(records: &[TickRecord]) -> usize {
        same(&teams(), &[], records);
        let out = running(&teams(), &[], records);
        let all = out
            .totals()
            .find(|(rule, _)| *rule == "anchor_tolerance")
            .unwrap()
            .1;
        let his = out
            .kept()
            .iter()
            .filter(|v| v.rule == "anchor_tolerance" && v.player == Some(5))
            .count();
        assert_eq!(all, his, "only player 5 drifts");
        his
    }

    /// A run in behind takes a forward far from his anchor and from the ball, and he is closing
    /// while it lasts: at 8 m/s toward the goal his side attacks for the grace period and 20
    /// ticks more, from 30 m into that half with the ball 25 m behind him and 60 m across, no
    /// violation. The control: the same path at a jog (3 m/s, below a run's speed) breaks the
    /// rule, and so does standing still at the run's end.
    #[test]
    fn a_player_on_a_run_in_behind_is_closing() {
        let tuning = Tuning::default();
        let (grace, dt) = (tuning.anchor_grace_ticks, tuning.dt);
        let ax = bare(0).attack_x;
        let ball = move |_| DVec2::new(ax * 5.0, 30.0);
        let path =
            move |v: f64| move |k: u32| DVec2::new(ax * (30.0 + v * dt * f64::from(k)), -30.0);
        let n = grace + 20;
        let far = run_scene(1..=n, ball, path(3.0));
        for r in &far {
            let p = DVec2::new(f64::from(r.players[5][0]), f64::from(r.players[5][1]));
            let b = DVec2::new(f64::from(r.ball[0]), f64::from(r.ball[1]));
            let a = [true, false].map(|has| bare(0).anchor(5, b, has, &tuning));
            assert!((p - b).length() > tuning.anchor_ball_distance);
            assert!((p - nearer_anchor(a, p)).length() > tuning.anchor_tolerance);
        }
        assert!(run_scene_anchors(&far) > 0, "a jog away from both drifts");
        assert_eq!(
            run_scene_anchors(&run_scene(1..=n, ball, path(8.0))),
            0,
            "a run is closing"
        );
        let end = path(8.0)(n);
        assert!(
            run_scene_anchors(&run_scene(1..=n, ball, move |_| end)) > 0,
            "a still player far from both drifts"
        );
    }

    /// The rule judges a player by his job: far from his anchor and from the ball, a marker
    /// standing 1.5 m from a still opponent is on his job and breaks no rule, and the opponent
    /// he stands by, far from his own anchor, is on his job too. The control: the same marker
    /// on the same spot with nobody near him, stuck far from his place and from every job
    /// target, breaks it.
    #[test]
    fn a_far_marker_on_a_still_runner_is_on_his_job_and_a_stuck_player_is_not() {
        let tuning = Tuning::default();
        let n = tuning.anchor_grace_ticks + 20;
        let ax = bare(0).attack_x;
        let b = DVec2::new(ax * 5.0, 30.0);
        let spot = DVec2::new(ax * 38.0, -30.0);
        let stuck = run_scene(1..=n, move |_| b, move |_| spot);
        for r in &stuck {
            let a = [true, false].map(|has| bare(0).anchor(5, b, has, &tuning));
            assert!((spot - b).length() > tuning.anchor_ball_distance);
            assert!((spot - nearer_anchor(a, spot)).length() > tuning.anchor_tolerance);
            let nearest = (PLAYERS_PER_TEAM..PLAYER_COUNT)
                .map(|j| {
                    (DVec2::new(f64::from(r.players[j][0]), f64::from(r.players[j][1])) - spot)
                        .length()
                })
                .fold(f64::INFINITY, f64::min);
            assert!(nearest > super::super::JOB_NEAR_M, "nobody near him");
        }
        assert!(run_scene_anchors(&stuck) > 0, "a stuck player drifts");
        let runner = PLAYERS_PER_TEAM + 9;
        let mut marking = stuck.clone();
        for r in &mut marking {
            r.players[runner] = [(spot.x + 1.5) as f32, spot.y as f32];
        }
        assert_eq!(
            run_scene_anchors(&marking),
            0,
            "a marker on a still runner is on his job"
        );
    }

    /// The end of a run: a forward runs at `run` m/s for 1 s toward the goal his side attacks,
    /// brakes to a stop over 1.5 s, still going forward, then walks back toward his anchor at
    /// 1.5 m/s for the grace period, with the ball still, 60 m across. Each step of the walk
    /// aims at his nearer anchor.
    fn run_brake_and_back(run: f64) -> Vec<TickRecord> {
        let tuning = Tuning::default();
        let (grace, dt) = (tuning.anchor_grace_ticks, tuning.dt);
        let ax = bare(0).attack_x;
        let b = DVec2::new(ax * 5.0, 30.0);
        let (run_ticks, brake_ticks) = ((1.0 / dt) as u32, (1.5 / dt) as u32);
        let mut at = vec![DVec2::new(ax * 30.0, -30.0)];
        for k in 1..=run_ticks + brake_ticks + grace {
            let p = *at.last().unwrap();
            let step = if k <= run_ticks {
                DVec2::new(ax * run * dt, 0.0)
            } else if k <= run_ticks + brake_ticks {
                let left = f64::from(run_ticks + brake_ticks - k) / f64::from(brake_ticks);
                DVec2::new(ax * run * left * dt, 0.0)
            } else {
                let a = [true, false].map(|has| bare(0).anchor(5, b, has, &tuning));
                (nearer_anchor(a, p) - p).normalize() * 1.5 * dt
            };
            at.push(p + step);
        }
        let n = at.len() as u32 - 1;
        run_scene(1..=n, move |_| b, move |k| at[k as usize])
    }

    /// A run in behind ends with the forward braking far from his anchor and from the ball,
    /// then walking back: no violation, because the run is closing while it lasts. The
    /// controls: the same path at a jog (3 m/s, no run) breaks the rule while he jogs and
    /// brakes away from both, and standing still at the end of the run breaks it too.
    #[test]
    fn a_run_in_behind_its_braking_and_the_way_back_break_no_rule() {
        let tuning = Tuning::default();
        let run = run_brake_and_back(8.0);
        let end = DVec2::new(
            f64::from(run[90].players[5][0]),
            f64::from(run[90].players[5][1]),
        );
        for r in &run[..120] {
            let p = DVec2::new(f64::from(r.players[5][0]), f64::from(r.players[5][1]));
            let b = DVec2::new(f64::from(r.ball[0]), f64::from(r.ball[1]));
            let a = [true, false].map(|has| bare(0).anchor(5, b, has, &tuning));
            assert!((p - b).length() > tuning.anchor_ball_distance);
            assert!((p - nearer_anchor(a, p)).length() > tuning.anchor_tolerance);
        }
        assert_eq!(
            run_scene_anchors(&run),
            0,
            "a run, its braking and the way back"
        );
        assert!(
            run_scene_anchors(&run_brake_and_back(3.0)) > 0,
            "a jog away from both drifts"
        );
        let b = DVec2::new(f64::from(run[0].ball[0]), f64::from(run[0].ball[1]));
        let n = tuning.anchor_grace_ticks + 20;
        assert!(
            run_scene_anchors(&run_scene(1..=n, move |_| b, move |_| end)) > 0,
            "a still player far from both drifts"
        );
    }

    /// The way back after a run: a step toward the anchor is closing even when the anchor
    /// slides away faster than he walks; a still step is not; the same step with no run
    /// before it is not; and the way back ends once he is near his anchor or the ball.
    #[test]
    fn a_runner_on_his_way_back_is_closing_while_he_moves_toward_his_anchor() {
        let tuning = Tuning::default();
        let step_max = max_step(&tuning);
        let dt = tuning.dt;
        let run = DVec2::new(8.0 * dt, 0.0);
        let walk = DVec2::new(-1.5 * dt, 0.0);
        let anchor = DVec2::new(-60.0, -30.0);
        let ran = || {
            let mut spell = RunSpell::default();
            let mut at = DVec2::new(-40.0, -30.0);
            spell.closing(None, at, 1.0, Some(anchor), step_max, &tuning);
            for _ in 0..50 {
                assert!(spell.closing(Some(at), at + run, 1.0, Some(anchor), step_max, &tuning));
                at += run;
            }
            (spell, at)
        };
        let (mut spell, at) = ran();
        assert!(
            spell.closing(Some(at), at + walk, 1.0, Some(anchor), step_max, &tuning),
            "on his way back"
        );
        let (mut spell, at) = ran();
        assert!(
            !spell.closing(Some(at), at, 1.0, Some(anchor), step_max, &tuning),
            "standing still"
        );
        let mut fresh = RunSpell::default();
        let at = DVec2::new(-30.0, -30.0);
        fresh.closing(None, at, 1.0, Some(anchor), step_max, &tuning);
        assert!(
            !fresh.closing(Some(at), at + walk, 1.0, Some(anchor), step_max, &tuning),
            "no run before"
        );
        let (mut spell, at) = ran();
        assert!(
            !spell.closing(Some(at), at + walk, 1.0, None, step_max, &tuning),
            "near again"
        );
        assert!(!spell.closing(
            Some(at + walk),
            at + walk * 2.0,
            1.0,
            Some(anchor),
            step_max,
            &tuning
        ));
    }

    /// A run counts as closing for as long as real runs last ([`MAX_RUN_S`]) and no longer.
    #[test]
    fn a_run_is_closing_for_at_most_the_longest_real_runs() {
        let tuning = Tuning::default();
        let step_max = max_step(&tuning);
        let mut spell = RunSpell::default();
        let step = DVec2::new(8.0 * tuning.dt, 0.0);
        let limit = (MAX_RUN_S / tuning.dt + 1e-9).floor() as u32;
        let mut at = DVec2::new(-40.0, -30.0);
        let anchor = DVec2::new(-60.0, -30.0);
        assert!(!spell.closing(None, at, 1.0, Some(anchor), step_max, &tuning));
        for k in 1..=limit + 50 {
            let next = at + step;
            let closing = spell.closing(Some(at), next, 1.0, Some(anchor), step_max, &tuning);
            assert_eq!(closing, k <= limit, "record {k}");
            at = next;
        }
        // Turning back toward the anchor behind him ends the run; the way back is closing.
        assert!(spell.closing(Some(at), at - step, 1.0, Some(anchor), step_max, &tuning));
    }

    /// A step faster than any player's top speed is a jump, which the speed rule reports; it is
    /// no run, so it starts neither a run nor a way back.
    #[test]
    fn a_jump_is_no_run() {
        let tuning = Tuning::default();
        let step_max = max_step(&tuning);
        let anchor = DVec2::new(-60.0, -30.0);
        let mut spell = RunSpell::default();
        let at = DVec2::new(-40.0, -30.0);
        spell.closing(None, at, 1.0, Some(anchor), step_max, &tuning);
        let jump = at + DVec2::new(step_max * 2.0, 0.0);
        assert!(!spell.closing(Some(at), jump, 1.0, Some(anchor), step_max, &tuning));
        let walk = DVec2::new(-1.5 * tuning.dt, 0.0);
        assert!(
            !spell.closing(
                Some(jump),
                jump + walk,
                1.0,
                Some(anchor),
                step_max,
                &tuning
            ),
            "no way back after a jump"
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
                .totals()
                .find(|(rule, _)| *rule == "anchor_tolerance")
                .unwrap()
                .1
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

    /// Every player on one spot for three ticks: 231 pairs too close per tick. The checker
    /// keeps the first 64 separation violations and counts all 693, as the old validator
    /// does.
    #[test]
    fn a_rule_past_the_cap_keeps_its_first_violations_and_counts_them_all() {
        let crowd = |tick: u32| {
            let mut r = record(tick);
            r.players = [[1.0, 1.0]; PLAYER_COUNT];
            r
        };
        let records = [crowd(1), crowd(2), crowd(3)];
        let pairs = PLAYER_COUNT * (PLAYER_COUNT - 1) / 2;
        let out = running(&teams(), &[], &records);
        let separation = out.totals().find(|(rule, _)| *rule == "separation");
        assert_eq!(separation, Some(("separation", 3 * pairs)));
        assert!(3 * pairs > RunningCheck::KEPT_PER_RULE);
        let kept: Vec<&Violation> = out
            .kept()
            .iter()
            .filter(|v| v.rule == "separation")
            .collect();
        assert_eq!(kept.len(), RunningCheck::KEPT_PER_RULE);
        assert!(kept.iter().all(|v| v.tick == 1), "the first ones are kept");
        assert_eq!(out.len(), old(&teams(), &[], &records).len());
        same(&teams(), &[], &records);
    }
}
