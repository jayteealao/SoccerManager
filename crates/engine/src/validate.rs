//! The tick-stream validator: pitch bounds, player separation, ball speed, player speed,
//! formation anchor tolerance in open play, and the ball's place at a restart. The anchor rule
//! judges a player by his job: it flags a player that is far from the ball, far from its
//! anchor, far from every job target the records show (`job_distance`: an opponent he may
//! track, mark or press, and for a back-line player the goal-side cover spot of a central
//! opponent in his half), and closing on none of them for the grace period (idle drift); his
//! own step toward where a job target stood counts as closing on it, not the target coming to
//! him (`job_closing`); a player who moves toward where the ball was on the previous
//! record is closing on it, however fast the ball runs away from him; a player on a run in
//! behind is closing for as long as real runs last, and on his way back after it while he moves
//! toward his anchor ([`RunSpell`]); while the ball waits
//! where it left play before it appears at a restart spot, the spot stands for the ball. A
//! sent-off or injured player at its parking spot beside the pitch is exempt from every player
//! rule. The player-speed rule limits each
//! player's move between two records to the top of the speed map for one tick plus one
//! separation push; the first record of a half (the kick-off placed after the walk-on) and the
//! slot a substitution names on its tick (the substitute enters at the halfway line on the
//! touchline) are exempt. The anchors follow the team shapes in force at each tick: the teams
//! change ends at half-time, a line closes up after a sending-off or an injury and reopens when
//! a substitute replaces the injured player, and a tactics change moves every anchor. From the
//! first kick of a penalty shoot-out on, the players wait in the centre circle and the anchor
//! rule no longer applies. The ball-rest rule flags a ball in play that lies still for
//! [`MAX_BALL_REST_S`] (a stall: nobody reaches it, and play never stops); a dead spell, from an
//! event naming a restart spot until the ball first moves after it is placed, and a shoot-out
//! are exempt.
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
/// The tick of a match's first record. The opening kick-off is taken on the step that
/// records it, before any record shows the ball at the spot, so the anchor rule reads that
/// record as the kick-off's restart: the ball no longer waits at the centre spot.
pub const OPENING_RECORD_TICK: u32 = 1;
/// The longest a ball in play may lie still, in seconds: the median length of the 696 real
/// dead spells of the IDSSE Bundesliga and 2. Bundesliga matches (`idsse_dead.csv`, the spells
/// the restart waits are drawn from; shortest 1.64 s, longest 206.48 s). Half of all real
/// stoppages are over sooner, so a ball that lies still this long while play has not stopped
/// is a stall, not football.
pub const MAX_BALL_REST_S: f64 = 19.92;
/// A ball that moves less than this between two records is at rest, in metres.
pub const BALL_REST_STEP: f64 = 0.001;

/// The `ball_rest` rule's state, shared by the two checkers: a dead spell runs from an event
/// that names a restart spot until the ball first moves on a record after the restart record
/// that placed it; outside one, the records the ball lies still on are counted, and the rule
/// fires once, on the record that reaches [`MAX_BALL_REST_S`].
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BallRest {
    dead: bool,
    placed: bool,
    still: u32,
}

impl BallRest {
    /// The records of rest at which the rule fires, at the tuning's tick length.
    pub(crate) fn limit_ticks(t: &Tuning) -> u32 {
        (MAX_BALL_REST_S / t.dt).round() as u32
    }

    /// A spell that starts inside a stoppage: placed when its restart record came before the
    /// first record.
    pub(crate) fn dead(placed: bool) -> Self {
        Self {
            dead: true,
            placed,
            still: 0,
        }
    }

    /// An event naming a restart spot: play has stopped.
    pub(crate) fn stopped(&mut self) {
        self.dead = true;
        self.placed = false;
    }

    /// The next record's ball after `prev` (none on the first record); `true` when this record
    /// reaches the limit. A shoot-out's records count no rest.
    pub(crate) fn record(
        &mut self,
        prev: Option<DVec2>,
        ball: DVec2,
        restart: bool,
        shootout: bool,
        limit: u32,
    ) -> bool {
        let still = prev.is_some_and(|p| (ball - p).length() < BALL_REST_STEP);
        if self.dead {
            if restart {
                self.placed = true;
            } else if self.placed && !still && prev.is_some() {
                self.dead = false;
            }
        }
        if still && !self.dead && !shootout {
            self.still += 1;
        } else {
            self.still = 0;
        }
        self.still == limit
    }
}

/// The longest a run in behind counts as closing, in seconds: the 99th percentile of the
/// durations of 5,850 tracked runs in behind (DFL Bundesliga, Metrica, PFF FIFA World Cup 2022
/// and SkillCorner A-League tracking, by the shared run definitions of [`crate::run_kind`];
/// median 3.08 s, longest 15.42 s).
pub const MAX_RUN_S: f64 = 9.81;

/// The anchor rule's run state of one player, shared by the two checkers. A run is a spell at
/// [`crate::run_kind::RUN_SPEED`] or faster that gains depth toward the goal his side attacks.
/// While it lasts, for at most [`MAX_RUN_S`], the player is closing: real runs in behind take
/// forwards far from their place and from the ball. After a run he is on his way back until he
/// is near his anchor or the ball again, and while he moves toward his anchor he is closing on
/// it, however fast the anchor slides away. A player who stands still far from both is not.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RunSpell {
    ticks: u32,
    returning: bool,
}

impl RunSpell {
    /// The player's step from `was` (the previous record; none on the first) to `now`, on a
    /// side that attacks `attack_x`; `far_from` is his nearer anchor while he is farther than the
    /// rule's distances from both his anchor and the ball, and `None` otherwise. A step longer
    /// than `max_step`, the most any player moves between two records, is a jump that the speed
    /// rule reports, not a run. `true` when the step counts as closing.
    pub(crate) fn closing(
        &mut self,
        was: Option<DVec2>,
        now: DVec2,
        attack_x: f64,
        far_from: Option<DVec2>,
        max_step: f64,
        t: &Tuning,
    ) -> bool {
        let Some(was) = was else {
            *self = Self::default();
            return false;
        };
        let step = now - was;
        let length = step.length();
        let on_run = length >= crate::run_kind::RUN_SPEED * t.dt
            && length <= max_step
            && step.x * attack_x > 0.0;
        if on_run {
            self.ticks += 1;
        } else {
            self.returning |= self.ticks > 0;
            self.ticks = 0;
        }
        if far_from.is_none() {
            self.returning = false;
        }
        let live = on_run && f64::from(self.ticks) * t.dt <= MAX_RUN_S;
        let back = self.returning
            && far_from.is_some_and(|anchor| step.dot((anchor - now).normalize_or_zero()) > 1e-3);
        live || back
    }
}

/// How near, in metres, a player stands to a job's target when he is doing that job: the
/// distance from which an opponent adds no pressure ([`crate::pressure::FAR_M`]), so a player
/// that close to a man is on him.
pub const JOB_NEAR_M: f64 = crate::pressure::FAR_M;

/// The anchor rule's job targets, shared by the two checkers: the distance from player `i`
/// (at `pos[i]`, his nearer anchor at `anchor`) to the nearest target
/// of a job the records show he may be doing. Each opponent on the pitch is one (the runner he
/// tracks, the man he marks or presses). For a back-line player, so is the cover spot of each
/// opponent in his side's half inside the central channel (`cover_channel`): the point on the
/// line from that opponent to his own goal's centre at the depth of his anchor, where the
/// engine's goal-side cover places him. Parked players are no target. The ball, his other job
/// target, the rule reads apart.
pub(crate) fn job_distance(
    pos: &[DVec2; PLAYER_COUNT],
    parked: &[bool; PLAYER_COUNT],
    i: usize,
    teams: &[Team; 2],
    anchor: DVec2,
    t: &Tuning,
) -> f64 {
    let team = i / PLAYERS_PER_TEAM;
    let side = JobSide::of(&teams[team], i % PLAYERS_PER_TEAM);
    job_distance_from(pos, parked, team, pos[i], &side, anchor, t)
}

/// What the job-target rule reads of the mover's side and slot: whether he covers (a
/// back-line player), his own goal's centre, the half length and the attack direction. It
/// changes only with the team's shape.
#[derive(Debug, Clone, Copy)]
pub(crate) struct JobSide {
    covers: bool,
    goal: DVec2,
    half: f64,
    attack_x: f64,
}

impl JobSide {
    /// The job side of `slot` on `side`'s shape.
    pub(crate) fn of(side: &Team, slot: usize) -> Self {
        Self {
            covers: side.in_back_line(slot),
            goal: side.own_goal(),
            half: side.pitch().half_length(),
            attack_x: side.attack_x,
        }
    }
}

/// [`job_distance`] for a mover of `team` standing at `me` (his nearer anchor at `anchor`),
/// the targets read from `targets`: the mover's own team is never a target, so his own
/// entry in `targets` is not read.
pub(crate) fn job_distance_from(
    targets: &[DVec2; PLAYER_COUNT],
    parked: &[bool; PLAYER_COUNT],
    team: usize,
    me: DVec2,
    side: &JobSide,
    anchor: DVec2,
    t: &Tuning,
) -> f64 {
    let half = side.half;
    let depth = |at: DVec2| at.x * side.attack_x + half;
    let covers = side.covers;
    let goal = side.goal;
    let hold = depth(anchor).max(0.5);
    let mut nearest = f64::INFINITY;
    for (j, &at) in targets.iter().enumerate() {
        if j / PLAYERS_PER_TEAM == team || parked[j] {
            continue;
        }
        nearest = nearest.min((at - me).length());
        let reach = depth(at);
        if covers && at.y.abs() <= t.cover_channel && reach < half && reach > 1e-6 {
            let share = (hold / reach).min(1.0);
            let spot = DVec2::new(
                goal.x + side.attack_x * hold,
                goal.y + (at.y - goal.y) * share,
            );
            nearest = nearest.min((spot - me).length());
        }
    }
    nearest
}

/// `true` when player `i`'s step to `now` closes on a job target where it stood on the record
/// before (`before`, the previous positions, where he stood `was_m` from his nearest target):
/// his own move counts, not a target coming to him.
#[allow(clippy::too_many_arguments)]
pub(crate) fn job_closing(
    before: &[DVec2; PLAYER_COUNT],
    now: DVec2,
    was_m: f64,
    parked: &[bool; PLAYER_COUNT],
    i: usize,
    teams: &[Team; 2],
    anchor: DVec2,
    t: &Tuning,
) -> bool {
    let team = i / PLAYERS_PER_TEAM;
    let side = JobSide::of(&teams[team], i % PLAYERS_PER_TEAM);
    job_distance_from(before, parked, team, now, &side, anchor, t) < was_m - 1e-3
}

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
    /// The ticks of half-time and each extra-time break: the next half's first record.
    half_times: Vec<u32>,
    /// The roster slot each substitution names, with its tick.
    entries: Vec<(u32, usize)>,
    /// The ball-rest state the first record starts in: a match resumed while play was
    /// stopped starts inside that stoppage, whose event came before its records.
    rest_at_start: BallRest,
}

impl Validator {
    /// A validator for teams whose shape never changes.
    pub fn new(tuning: Tuning, teams: [Team; 2]) -> Self {
        Self {
            tuning,
            timeline: vec![(0, teams)],
            spots: Vec::new(),
            shootout_from: None,
            half_times: Vec::new(),
            entries: Vec::new(),
            rest_at_start: BallRest::default(),
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
        let half_times = events
            .iter()
            .filter(|e| e.kind == EngineEventKind::HalfTime)
            .map(|e| e.tick)
            .collect();
        let entries = events
            .iter()
            .filter(|e| e.kind == EngineEventKind::Substitution)
            .filter_map(|e| e.player.map(|p| (e.tick, p)))
            .collect();
        Self {
            tuning,
            timeline: timeline.to_vec(),
            spots,
            shootout_from,
            half_times,
            entries,
            rest_at_start: BallRest::default(),
        }
    }

    /// The validator of records that start while play is stopped, as a match resumed from a
    /// snapshot taken at a stoppage does: the ball-rest rule counts no rest until the ball
    /// first moves after its restart record. `placed` is `true` when that record came before
    /// the first record.
    pub fn starting_in_stoppage(mut self, placed: bool) -> Self {
        self.rest_at_start = BallRest::dead(placed);
        self
    }

    /// Returns every violation in `records`, in tick order.
    pub fn check(&self, records: &[TickRecord]) -> Vec<Violation> {
        let mut out = Vec::new();
        let mut far_ticks = [0u32; PLAYER_COUNT];
        let mut runs = [RunSpell::default(); PLAYER_COUNT];
        let mut prev_dist = [[f64::INFINITY; 3]; PLAYER_COUNT];
        let mut prev_chase: Option<(DVec2, [DVec2; PLAYER_COUNT])> = None;
        let mut prev_ball: Option<DVec2> = None;
        let mut prev_players: Option<([DVec2; PLAYER_COUNT], [bool; PLAYER_COUNT])> = None;
        let max_step = running::max_step(&self.tuning);
        // The restart spot whose ball has not yet appeared there, and the next spot to read.
        let mut fetch_spot: Option<DVec2> = None;
        let mut next_spot = 0;
        let mut shape = 0;
        let mut rest = self.rest_at_start;
        let rest_limit = BallRest::limit_ticks(&self.tuning);
        let mut next_stop = 0;
        for r in records {
            while shape + 1 < self.timeline.len() && self.timeline[shape + 1].0 <= r.tick {
                shape += 1;
            }
            let teams = &self.timeline[shape].1;
            let pitch = teams[0].pitch();
            let ball = DVec2::new(f64::from(r.ball[0]), f64::from(r.ball[1]));
            while let Some(&(tick, _)) = self.spots.get(next_stop)
                && tick <= r.tick
            {
                rest.stopped();
                next_stop += 1;
            }
            let shootout = self.shootout_from.is_some_and(|from| r.tick >= from);
            let rested = rest.record(prev_ball, ball, r.restart, shootout, rest_limit);
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
            if let Some((prev, prev_parked)) = &prev_players
                && !self.half_times.contains(&r.tick)
            {
                for i in 0..PLAYER_COUNT {
                    if parked[i] || prev_parked[i] || self.entries.contains(&(r.tick, i)) {
                        continue;
                    }
                    let step = (pos[i] - prev[i]).length();
                    if step > max_step {
                        out.push(Violation {
                            tick: r.tick,
                            rule: "player_speed",
                            player: Some(i),
                            value: step / self.tuning.dt,
                        });
                    }
                }
            }
            prev_players = Some((pos, parked));
            if r.restart
                && let Some(spot) = running::latest_spot(&self.spots, r.tick)
            {
                let off = (ball - spot).length();
                if off > RESTART_SPOT_TOLERANCE {
                    out.push(Violation {
                        tick: r.tick,
                        rule: "restart_spot",
                        player: None,
                        value: off,
                    });
                }
            }

            while let Some(&(tick, spot)) = self.spots.get(next_spot)
                && tick <= r.tick
            {
                fetch_spot = Some(spot);
                next_spot += 1;
            }
            // While the ball waits where it left play, the spot stands for the ball.
            let reference = fetch_spot.unwrap_or(ball);
            if r.restart
                || r.tick == OPENING_RECORD_TICK
                || (ball - reference).length() <= RESTART_SPOT_TOLERANCE
            {
                fetch_spot = None;
            }

            for (i, p) in pos.iter().enumerate() {
                if parked[i] || shootout {
                    far_ticks[i] = 0;
                    runs[i] = RunSpell::default();
                    continue;
                }
                let team = i / PLAYERS_PER_TEAM;
                let slot = i % PLAYERS_PER_TEAM;
                // The record does not say which team has the ball, so the nearer of the
                // slot's two anchors counts.
                let anchors =
                    [true, false].map(|has| teams[team].anchor(slot, reference, has, &self.tuning));
                let nearer = running::nearer_anchor(anchors, *p);
                let d_anchor = (*p - nearer).length().min(running::kick_off_place_distance(
                    &teams[team],
                    slot,
                    *p,
                    reference,
                ));
                let d_ball = (*p - reference).length();
                let d_job = job_distance(&pos, &parked, i, teams, nearer, &self.tuning);
                let toward_ball = prev_chase
                    .is_some_and(|(was, at)| (*p - was).length() < (at[i] - was).length() - 1e-3);
                let far = d_ball > self.tuning.anchor_ball_distance
                    && d_anchor > self.tuning.anchor_tolerance
                    && d_job > JOB_NEAR_M;
                let on_run = runs[i].closing(
                    prev_chase.map(|(_, at)| at[i]),
                    *p,
                    teams[team].attack_x,
                    far.then_some(nearer),
                    max_step,
                    &self.tuning,
                );
                let toward_job = prev_chase.is_some_and(|(_, at)| {
                    job_closing(
                        &at,
                        *p,
                        prev_dist[i][2],
                        &parked,
                        i,
                        teams,
                        nearer,
                        &self.tuning,
                    )
                });
                let closing = d_ball < prev_dist[i][0] - 1e-3
                    || d_anchor < prev_dist[i][1] - 1e-3
                    || toward_job
                    || toward_ball
                    || on_run;
                prev_dist[i] = [d_ball, d_anchor, d_job];
                let drifting = far && !closing;
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
            prev_chase = Some((reference, pos));
            if rested {
                out.push(Violation {
                    tick: r.tick,
                    rule: "ball_rest",
                    player: None,
                    value: MAX_BALL_REST_S,
                });
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
        // One record only: from a record before it, player 1's move onto player 0 would also
        // break the player speed rule.
        let out = v.check(&[r]);
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

    /// A ball in play that lies still for the rest limit is one `ball_rest` violation, on the
    /// record that reaches it. The same rest while play is stopped (from a stoppage until the
    /// ball first moves after its restart record) is no violation; once that ball has moved,
    /// a new rest counts from there.
    #[test]
    fn a_ball_still_in_play_for_the_rest_limit_is_one_violation_and_a_dead_ball_none() {
        let t = Tuning::default();
        let limit = BallRest::limit_ticks(&t);
        assert_eq!(limit, 996, "19.92 s at 50 ticks a second");
        let records: Vec<TickRecord> = (1..=limit + 200).map(record).collect();
        let v = Validator::new(t.clone(), [bare(0), bare(1)]);
        let out = v.check(&records);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].rule, "ball_rest");
        assert_eq!(out[0].tick, limit + 1, "the first record rests nothing");
        assert_eq!(out[0].value, MAX_BALL_REST_S);

        // A throw-in at tick 2: the ball is placed there and waits to the end.
        let config = crate::data::test_support::shipped_config(1, 90).unwrap();
        let sim = crate::sim::Simulation::new(config).unwrap();
        let mut throw = sim.event(crate::sim::EngineEventKind::ThrowIn, Some(0));
        throw.tick = 2;
        throw.spot = Some(DVec2::ZERO);
        let v = Validator::for_match(t.clone(), &[(0, [bare(0), bare(1)])], &[throw]);
        let mut waiting = records.clone();
        waiting[1].restart = true;
        assert!(v.check(&waiting).is_empty(), "{:?}", v.check(&waiting));

        // The throw is taken at tick 100 (the ball moves 0.5 m); from there it lies still again.
        let mut taken = waiting;
        for r in &mut taken[99..] {
            r.ball = [0.5, 0.0, 0.0];
        }
        let out = v.check(&taken);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].rule, "ball_rest");
        assert_eq!(out[0].tick, 100 + limit);
    }

    /// Records that start inside a stoppage, as a match resumed from a snapshot taken while
    /// play is stopped: the still ball rests nothing until it moves after its restart record,
    /// whether that record comes in the records or came before them. The same records read as
    /// a match in play break the rule.
    #[test]
    fn records_that_start_in_a_stoppage_rest_nothing_until_the_restart_is_taken() {
        let t = Tuning::default();
        let limit = BallRest::limit_ticks(&t);
        let still: Vec<TickRecord> = (1..=limit + 200).map(record).collect();
        let in_play = Validator::new(t.clone(), [bare(0), bare(1)]);
        assert_eq!(in_play.check(&still).len(), 1);

        // The restart record came before the first record.
        let placed = Validator::new(t.clone(), [bare(0), bare(1)]).starting_in_stoppage(true);
        assert!(placed.check(&still).is_empty());
        // Taken at tick 100: a new rest counts from there.
        let mut taken = still.clone();
        for r in &mut taken[99..] {
            r.ball = [0.5, 0.0, 0.0];
        }
        let out = placed.check(&taken);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].tick, 100 + limit);

        // The restart record is in the records, at tick 50.
        let waiting = Validator::new(t.clone(), [bare(0), bare(1)]).starting_in_stoppage(false);
        let mut appears = still;
        appears[49].restart = true;
        assert!(waiting.check(&appears).is_empty());
    }
}
