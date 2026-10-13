//! The team-shape measures of one match: where each outfield position stands against the
//! ball, how wide, how fast and how often it sprints, and per team how the block slides with
//! the ball, how far players answer the ball, how calm dead balls are, the stoppages by kind
//! and the completed links between team-mates.
//!
//! [`ShapeSink`] is a tick sink: it reads the match after each step and writes nothing, so a
//! seed plays the same match with it or without it. [`ShapeSink::finish`] gives every figure
//! as sums, per side, in [`field_names`] order; the report turns the sums into bands as
//! ratios, so the sums of many matches pool exactly.
//!
//! The definitions follow the movement scripts the real figures come from:
//! - A sample is a ball-in-play tick with a team in possession; positions are read at 5 Hz
//!   (every tenth tick), speeds every tick.
//! - Depth is the ball's x minus the player's x in his team's attacking frame: positive behind
//!   the ball. Width is |y|.
//! - The ball is on a player's own flank when it is more than 5 m to his side of the centre
//!   line, on the far flank when more than 5 m to the other side. His side is the sign of his
//!   mean y over the half, so his samples are kept by the ball's side until the half ends.
//! - A player's speed is his displacement over the last 0.32 s, as the scripts smooth the
//!   tracked positions (a 0.32 s moving average, then the gradient); in his first 0.32 s on
//!   the pitch it is his velocity. Pace, mean speed, sprints and calm read it.
//! - Pace levels: under 0.2, 2, 4, 5.5 and 7 m/s, and 7 and over.
//! - A sprint is a run above 7 m/s of at least 1 s of play; it is forward when x gains more
//!   than 3 m in the attacking frame, back when it loses more than 3 m, else lateral.
//! - The slide is the slope of the team's outfield centroid on the ball, up the pitch (x, in
//!   the attacking frame) and across (y), in and out of possession, from 5 Hz samples with 9
//!   or more outfield players: kept as the match's centred sums `sxy` and `sxx`.
//! - The far-player gain is Σ(p·q) / Σ(q·q) over windows of 1 s and 10 s of unbroken play,
//!   each starting every 0.2 s, of the player's displacement p and the ball's q, for windows
//!   that start with the player 30 to 45 m or more than 45 m from the ball.
//! - Calm is the share of dead-ball outfield ticks under 2 m/s. The walk-on is not a tick.
//! - Stoppages count by the kind of restart and the side that takes it; a kick-off after a
//!   goal counts as a kick-off.
//! - A completed link is a change of controlling player within one side in play, with no dead
//!   ball between and under 5 s from the last touch of the passer. At full time the side's
//!   two players who received most links give the top-two share: the links between those two
//!   over all links. The back-line share is the links a full-back's or centre-back's slot
//!   gave or received over all links.

use crate::data::rules::StoppageKind;
use crate::data::team::Position;
use crate::math::DVec2;
use crate::player::Player;
use crate::record::{TickRecord, TickSink};
use crate::rules::Stoppage;
use crate::sim::Simulation;
use crate::{EngineError, TICKS_PER_SECOND};

/// The outfield groups, by the position of the player's formation slot.
pub const GROUPS: [&str; 5] = ["cb", "fb", "cm", "w", "st"];
/// The figures kept per group, in order.
pub const GROUP_FIELDS: [&str; GROUP_FIELD_COUNT] = [
    "depth_in_sum",
    "depth_in_n",
    "depth_out_sum",
    "depth_out_n",
    "width_near_in_sum",
    "width_near_in_n",
    "width_near_out_sum",
    "width_near_out_n",
    "width_far_in_sum",
    "width_far_in_n",
    "width_far_out_sum",
    "width_far_out_n",
    "pace_stand",
    "pace_walk",
    "pace_jog",
    "pace_run",
    "pace_hs",
    "pace_sprint",
    "speed_near_sum",
    "speed_near_n",
    "speed_far_sum",
    "speed_far_n",
    "sprints_fwd",
    "sprints_back",
    "sprints_lat",
    "live_ticks",
];
pub const GROUP_FIELD_COUNT: usize = 26;
/// The figures kept per team, in order.
pub const TEAM_FIELDS: [&str; TEAM_FIELD_COUNT] = [
    "slide_x_in_sxy",
    "slide_x_in_sxx",
    "slide_x_out_sxy",
    "slide_x_out_sxx",
    "slide_y_in_sxy",
    "slide_y_in_sxx",
    "slide_y_out_sxy",
    "slide_y_out_sxx",
    "far_1s_30_45_pq",
    "far_1s_30_45_qq",
    "far_1s_gt45_pq",
    "far_1s_gt45_qq",
    "far_10s_30_45_pq",
    "far_10s_30_45_qq",
    "far_10s_gt45_pq",
    "far_10s_gt45_qq",
    "calm_ticks",
    "dead_ticks",
    "stops_throw_in",
    "stops_free_kick",
    "stops_goal_kick",
    "stops_corner",
    "stops_kick_off",
    "stops_penalty",
    "links_top2",
    "links_back",
    "links_total",
];
pub const TEAM_FIELD_COUNT: usize = 27;
/// Figures per side: every group's, then the team's.
pub const SHAPE_FIELDS: usize = GROUPS.len() * GROUP_FIELD_COUNT + TEAM_FIELD_COUNT;

/// Every figure's name, per side, in value order: `<group>.<field>`, then the team fields.
pub fn field_names() -> Vec<String> {
    let mut out = Vec::with_capacity(SHAPE_FIELDS);
    for g in GROUPS {
        for f in GROUP_FIELDS {
            out.push(format!("{g}.{f}"));
        }
    }
    out.extend(TEAM_FIELDS.iter().map(|f| (*f).to_string()));
    out
}

/// Every figure of a match, per side, home first, in [`field_names`] order.
pub type ShapeValues = [[f64; SHAPE_FIELDS]; 2];

/// The upper bounds of the pace levels below sprint, in m/s.
const PACE_TOPS: [f64; 5] = [0.2, 2.0, 4.0, 5.5, 7.0];
/// A sprint: above this speed, in m/s.
const SPRINT_MPS: f64 = 7.0;
/// A sprint lasts at least this many ticks.
const SPRINT_TICKS: u32 = TICKS_PER_SECOND;
/// A sprint's run along x that makes it forward or back, in m.
const SPRINT_RUN_M: f64 = 3.0;
/// The ball is on a flank this far from the centre line, in m.
const FLANK_M: f64 = 5.0;
/// Positions are read every this many ticks: 5 Hz.
const SAMPLE_TICKS: u32 = TICKS_PER_SECOND / 5;
/// The far-player windows, in samples: 1 s and 10 s.
const WINDOWS: [usize; 2] = [5, 50];
/// The far-player distance classes from the ball, in m: 30 to 45, and 45 and over.
const FAR_BINS: [(f64, f64); 2] = [(30.0, 45.0), (45.0, f64::INFINITY)];
/// Calm on a dead ball: under this speed, in m/s.
const CALM_MPS: f64 = 2.0;
/// The speed window: 0.32 s.
const SMOOTH_TICKS: usize = 16;
/// A completed link: at most this many ticks from the passer's last touch to the receiver's
/// first.
const LINK_TICKS: u32 = 5 * TICKS_PER_SECOND;
/// The fewest outfield players a slide sample needs.
const SLIDE_MIN_PLAYERS: usize = 9;
/// Players a match can have on the pitch.
const ROSTER: usize = 22;

/// The value index of group `g`'s field `f`.
const fn gi(g: usize, f: usize) -> usize {
    g * GROUP_FIELD_COUNT + f
}

/// The value index of team field `f`.
const fn ti(f: usize) -> usize {
    GROUPS.len() * GROUP_FIELD_COUNT + f
}

// Group field places.
const DEPTH: usize = 0; // depth_in_sum; + 2 for out
const WIDTH: usize = 4; // width_near_in_sum; near/far × in/out, sum then n
const PACE: usize = 12;
const SPEED: usize = 18; // speed_near_sum, speed_near_n, speed_far_sum, speed_far_n
const SPRINTS: usize = 22;
const LIVE: usize = 25;
// Team field places.
const SLIDE: usize = 0;
const FAR: usize = 8;
const CALM: usize = 16;
const DEAD: usize = 17;
const STOPS: usize = 18;
const LINKS: usize = 24;

/// The group of a slot's position; `None` for the keeper.
fn group_of(p: Position) -> Option<usize> {
    match p {
        Position::GK => None,
        Position::CB => Some(0),
        Position::LB | Position::RB => Some(1),
        Position::DM | Position::CM | Position::AM => Some(2),
        Position::LW | Position::RW => Some(3),
        Position::ST => Some(4),
    }
}

/// The pace level of a speed, 0 (stand) to 5 (sprint).
fn pace_level(speed: f64) -> usize {
    PACE_TOPS.iter().take_while(|&&top| speed >= top).count()
}

/// The stoppage kind's place in the `stops_*` fields.
fn stop_place(kind: StoppageKind) -> Option<usize> {
    match kind {
        StoppageKind::ThrowIn => Some(0),
        StoppageKind::FreeKick => Some(1),
        StoppageKind::GoalKick => Some(2),
        StoppageKind::Corner => Some(3),
        StoppageKind::Goal | StoppageKind::KickOff => Some(4),
        StoppageKind::Penalty => Some(5),
        StoppageKind::HalfTime | StoppageKind::Injury => None,
    }
}

/// One player's samples in one group and half, kept by the ball's side until his own side is
/// known at the end of the half. Index 0 holds the samples with the ball more than 5 m to the
/// left (negative y), 1 to the right.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Flanked {
    /// Per ball side, per phase (in, out): (width sum, samples).
    width: [[(f64, f64); 2]; 2],
    /// Per ball side: (speed sum, ticks).
    speed: [(f64, f64); 2],
    /// The player's y sum and samples over the half, for his side.
    y: (f64, f64),
}

/// A player's state as the sink follows him.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Follow {
    /// The squad index and group he is kept under; a change flushes his flank samples.
    key: Option<(usize, usize)>,
    team: usize,
    flanked: Flanked,
    /// The sprint in progress: its first tick, its first x in the attacking frame, its last
    /// x, and its length in ticks.
    sprint: Option<(f64, f64, u32)>,
}

/// One 5 Hz sample of a player for the far-player windows.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sample {
    pos: DVec2,
    ball: DVec2,
    /// Dead-ball ticks so far, the half, and the squad index: a window is play only when all
    /// three are the same at both ends.
    dead: u64,
    half: u32,
    squad: usize,
}

/// The slide sums of one side, dimension (x, y) and phase (in, out): n, Σb, Σc, Σb², Σbc.
type SlideSums = [[[f64; 5]; 2]; 2];

/// The team-shape measures of one match, as a tick sink.
#[derive(Debug, Clone)]
pub struct ShapeSink {
    values: ShapeValues,
    players: [Follow; ROSTER],
    /// Each player's 5 Hz samples, the newest last, at most the longest window plus one.
    rings: Vec<std::collections::VecDeque<Sample>>,
    /// Each player's positions over the last 0.32 s, the newest last, for his speed.
    trail: Vec<std::collections::VecDeque<DVec2>>,
    slide: [SlideSums; 2],
    /// The match's ball-in-play ticks after the last step.
    last_live: u32,
    /// Dead-ball steps so far.
    dead_steps: u64,
    /// The half of the last step.
    half: u32,
    /// The last controlling player, his side, and the tick of his last touch.
    last_carrier: Option<(usize, usize, u32)>,
    /// A dead ball since the last control.
    dead_since_control: bool,
    /// Completed links, by passer and receiver roster place.
    links: Box<[[u32; ROSTER]; ROSTER]>,
    /// The back-line links per side.
    back_links: [u32; 2],
    steps: u32,
}

impl Default for ShapeSink {
    fn default() -> Self {
        Self {
            values: [[0.0; SHAPE_FIELDS]; 2],
            players: [Follow::default(); ROSTER],
            rings: vec![std::collections::VecDeque::with_capacity(WINDOWS[1] + 1); ROSTER],
            trail: vec![std::collections::VecDeque::with_capacity(SMOOTH_TICKS + 1); ROSTER],
            slide: [[[[0.0; 5]; 2]; 2]; 2],
            last_live: 0,
            dead_steps: 0,
            half: 0,
            last_carrier: None,
            dead_since_control: false,
            links: Box::new([[0; ROSTER]; ROSTER]),
            back_links: [0; 2],
            steps: 0,
        }
    }
}

impl ShapeSink {
    /// Every figure of the match so far, with every open sample closed: flank samples are
    /// kept by each player's side over his half so far, sprints in progress end, and the
    /// links give their shares.
    pub fn finish(mut self) -> ShapeValues {
        for i in 0..ROSTER {
            self.flush(i);
            self.end_sprint(i);
        }
        for team in 0..2 {
            for dim in 0..2 {
                for phase in 0..2 {
                    let [n, b, c, bb, bc] = self.slide[team][dim][phase];
                    let at = ti(SLIDE + dim * 4 + phase * 2);
                    if n > 0.0 {
                        self.values[team][at] = bc - b * c / n;
                        self.values[team][at + 1] = bb - b * b / n;
                    }
                }
            }
            // The side's two players who received most links, by roster place; ties go to
            // the lower place.
            let ours: Vec<usize> = (0..ROSTER)
                .filter(|&a| self.players[a].team == team)
                .collect();
            let mut received = [0u32; ROSTER];
            let mut total = 0u32;
            for &a in &ours {
                for (b, r) in received.iter_mut().enumerate() {
                    *r += self.links[a][b];
                    total += self.links[a][b];
                }
            }
            let mut order = ours;
            order.sort_by_key(|&b| (std::cmp::Reverse(received[b]), b));
            let top2 = match order[..] {
                [x, y, ..] => self.links[x][y] + self.links[y][x],
                _ => 0,
            };
            self.values[team][ti(LINKS)] = f64::from(top2);
            self.values[team][ti(LINKS + 1)] = f64::from(self.back_links[team]);
            self.values[team][ti(LINKS + 2)] = f64::from(total);
        }
        self.values
    }

    /// Moves player `i`'s flank samples into his group's figures: the ball on his own side
    /// is near, on the other side far. A player whose mean y is exactly 0 has no side, and
    /// his flank samples count nowhere.
    fn flush(&mut self, i: usize) {
        let f = &mut self.players[i];
        let Some((_, g)) = f.key else {
            return;
        };
        let team = f.team;
        let fl = std::mem::take(&mut f.flanked);
        if fl.y.1 == 0.0 {
            return;
        }
        let mean = fl.y.0 / fl.y.1;
        // His side: 1 (right, positive y) or 0 (left).
        let own = if mean > 0.0 {
            1
        } else if mean < 0.0 {
            0
        } else {
            return;
        };
        let v = &mut self.values[team];
        for (zone, ball_side) in [(0, own), (1, 1 - own)] {
            for phase in 0..2 {
                let (s, n) = fl.width[ball_side][phase];
                let at = gi(g, WIDTH + zone * 4 + phase * 2);
                v[at] += s;
                v[at + 1] += n;
            }
            let (s, n) = fl.speed[ball_side];
            let at = gi(g, SPEED + zone * 2);
            v[at] += s;
            v[at + 1] += n;
        }
    }

    /// Ends player `i`'s sprint in progress: one sprint of his group when it lasted 1 s.
    fn end_sprint(&mut self, i: usize) {
        let f = &mut self.players[i];
        let (Some((x0, x1, ticks)), Some((_, g))) = (f.sprint.take(), f.key) else {
            return;
        };
        if ticks < SPRINT_TICKS {
            return;
        }
        let run = x1 - x0;
        let way = if run > SPRINT_RUN_M {
            0
        } else if run < -SPRINT_RUN_M {
            1
        } else {
            2
        };
        self.values[f.team][gi(g, SPRINTS + way)] += 1.0;
    }

    /// One step of the match: `live` when the ball was in play this step.
    fn step(&mut self, sim: &Simulation, live: bool) {
        self.steps += 1;
        let half = sim.half();
        if half != self.half {
            for i in 0..ROSTER {
                self.flush(i);
                self.end_sprint(i);
            }
            self.half = half;
        }
        if !live {
            self.dead_steps += 1;
            self.dead_since_control = true;
        }
        let phase = sim.possession_phase();
        let ball = sim.ball.pos.truncate();
        let sample = live && phase.is_some() && self.steps.is_multiple_of(SAMPLE_TICKS);
        let mut centroid = [(DVec2::ZERO, 0usize); 2];
        for p in sim.players().iter() {
            self.player(sim, p, live, phase, ball, sample, &mut centroid);
        }
        if let (true, Some(owner)) = (sample, phase) {
            for (team, (sum, n)) in centroid.into_iter().enumerate() {
                if n < SLIDE_MIN_PLAYERS {
                    continue;
                }
                let ax = sim.teams[team].attack_x;
                let c = sum / n as f64;
                let ph = usize::from(owner != team);
                for (dim, (b, c)) in [(ball.x * ax, c.x * ax), (ball.y, c.y)]
                    .into_iter()
                    .enumerate()
                {
                    let s = &mut self.slide[team][dim][ph];
                    s[0] += 1.0;
                    s[1] += b;
                    s[2] += c;
                    s[3] += b * b;
                    s[4] += b * c;
                }
            }
        }
        if live {
            self.link(sim);
        }
    }

    /// One player's step.
    #[allow(clippy::too_many_arguments)] // sdlc-debt: one call site in `step`; a struct of the step's state would only rename these.
    fn player(
        &mut self,
        sim: &Simulation,
        p: &Player,
        live: bool,
        phase: Option<usize>,
        ball: DVec2,
        sample: bool,
        centroid: &mut [(DVec2, usize); 2],
    ) {
        let i = p.id;
        let team = p.team;
        let outfield = p.active() && p.slot != sim.teams[team].keeper_slot();
        let group = if outfield {
            group_of(sim.slot_position(team, p.slot))
        } else {
            None
        };
        let key = group.map(|g| (p.squad, g));
        if self.players[i].key != key {
            self.flush(i);
            self.end_sprint(i);
            self.rings[i].clear();
            self.trail[i].clear();
            self.players[i].key = key;
        }
        self.players[i].team = team;
        let Some(g) = group else {
            return;
        };
        let speed = self.speed(i, p);
        if !live {
            let v = &mut self.values[team];
            v[ti(DEAD)] += 1.0;
            if speed < CALM_MPS {
                v[ti(CALM)] += 1.0;
            }
        }
        let ax = sim.teams[team].attack_x;
        let x = p.pos.x * ax;
        let Some(owner) = phase.filter(|_| live) else {
            self.end_sprint(i);
            return;
        };
        let ph = usize::from(owner != team);
        let v = &mut self.values[team];
        v[gi(g, LIVE)] += 1.0;
        v[gi(g, PACE + pace_level(speed))] += 1.0;
        let f = &mut self.players[i];
        let ball_side = if ball.y > FLANK_M {
            Some(1)
        } else if ball.y < -FLANK_M {
            Some(0)
        } else {
            None
        };
        if let Some(s) = ball_side {
            f.flanked.speed[s].0 += speed;
            f.flanked.speed[s].1 += 1.0;
        }
        if speed > SPRINT_MPS {
            let run = f.sprint.get_or_insert((x, x, 0));
            run.1 = x;
            run.2 += 1;
        } else {
            self.end_sprint(i);
        }
        if !sample {
            return;
        }
        let f = &mut self.players[i];
        f.flanked.y.0 += p.pos.y;
        f.flanked.y.1 += 1.0;
        if let Some(s) = ball_side {
            f.flanked.width[s][ph].0 += p.pos.y.abs();
            f.flanked.width[s][ph].1 += 1.0;
        }
        let v = &mut self.values[team];
        v[gi(g, DEPTH + ph * 2)] += ball.x * ax - x;
        v[gi(g, DEPTH + ph * 2 + 1)] += 1.0;
        centroid[team].0 += p.pos;
        centroid[team].1 += 1;
        self.far(i, team, p, ball, sim.half());
    }

    /// Player `i`'s speed: his displacement over the last 0.32 s, or his velocity until he
    /// has been followed that long.
    fn speed(&mut self, i: usize, p: &Player) -> f64 {
        let trail = &mut self.trail[i];
        trail.push_back(p.pos);
        if trail.len() > SMOOTH_TICKS + 1 {
            trail.pop_front();
        }
        match (trail.front(), trail.len()) {
            (Some(first), n) if n == SMOOTH_TICKS + 1 => {
                p.pos.distance(*first) * f64::from(TICKS_PER_SECOND) / SMOOTH_TICKS as f64
            }
            _ => p.vel.length(),
        }
    }

    /// Player `i`'s 5 Hz sample for the far-player windows.
    fn far(&mut self, i: usize, team: usize, p: &Player, ball: DVec2, half: u32) {
        let now = Sample {
            pos: p.pos,
            ball,
            dead: self.dead_steps,
            half,
            squad: p.squad,
        };
        let ring = &mut self.rings[i];
        ring.push_back(now);
        if ring.len() > WINDOWS[1] + 1 {
            ring.pop_front();
        }
        let len = ring.len();
        for (w, &back) in WINDOWS.iter().enumerate() {
            if len <= back {
                continue;
            }
            let start = ring[len - 1 - back];
            if start.dead != now.dead || start.half != now.half || start.squad != now.squad {
                continue;
            }
            let d = start.pos.distance(start.ball);
            let Some(bin) = FAR_BINS.iter().position(|&(lo, hi)| d >= lo && d < hi) else {
                continue;
            };
            let dp = now.pos - start.pos;
            let db = now.ball - start.ball;
            let at = ti(FAR + w * 4 + bin * 2);
            self.values[team][at] += dp.dot(db);
            self.values[team][at + 1] += db.dot(db);
        }
    }

    /// A live step's control: a new controlling player of the side that last controlled the
    /// ball, with no dead ball between and within 5 s, completes a link.
    fn link(&mut self, sim: &Simulation) {
        let Some(c) = sim.carrier() else {
            return;
        };
        let tick = sim.tick();
        let team = sim.players()[c].team;
        if let Some((a, ta, last)) = self.last_carrier
            && a != c
            && ta == team
            && !self.dead_since_control
            && tick.saturating_sub(last) < LINK_TICKS
        {
            self.links[a][c] += 1;
            let back = |i: usize| {
                matches!(
                    sim.played_position(i),
                    Position::LB | Position::CB | Position::RB
                )
            };
            if back(a) || back(c) {
                self.back_links[team] += 1;
            }
        }
        self.last_carrier = Some((c, team, tick));
        self.dead_since_control = false;
    }
}

impl TickSink for ShapeSink {
    fn on_tick(&mut self, _record: &TickRecord) -> Result<(), EngineError> {
        Ok(())
    }

    fn on_step(&mut self, sim: &Simulation) -> Result<(), EngineError> {
        let live_ticks = sim.summary().live_ticks;
        let live = live_ticks > self.last_live;
        self.last_live = live_ticks;
        self.step(sim, live);
        Ok(())
    }

    fn on_stoppage(&mut self, stoppage: &Stoppage, _sim: &Simulation) -> Result<(), EngineError> {
        if let (Some(place), Some(team)) = (stop_place(stoppage.kind), stoppage.team) {
            self.values[team][ti(STOPS + place)] += 1.0;
        }
        Ok(())
    }
}

/// The height of each side's block out of possession: the distance of its outfield centroid
/// from its own goal line, at the same 5 Hz ball-in-play samples as the slide (a side in
/// possession, 9 or more outfield players). Per side, home first: the sum of the heights in
/// metres and the samples, so many matches pool exactly. The mentality's signature reads it.
#[derive(Debug, Clone, Default)]
pub struct BlockHeightSink {
    /// Per side: (sum of heights in metres, samples).
    pub height: [(f64, f64); 2],
    last_live: u32,
    steps: u32,
}

impl BlockHeightSink {
    /// The mean block height of `team`, in metres; `None` with no sample.
    pub fn mean(&self, team: usize) -> Option<f64> {
        let (sum, n) = self.height[team];
        (n > 0.0).then(|| sum / n)
    }
}

impl TickSink for BlockHeightSink {
    fn on_tick(&mut self, _record: &TickRecord) -> Result<(), EngineError> {
        Ok(())
    }

    fn on_step(&mut self, sim: &Simulation) -> Result<(), EngineError> {
        self.steps += 1;
        let live_ticks = sim.summary().live_ticks;
        let live = live_ticks > self.last_live;
        self.last_live = live_ticks;
        let Some(owner) = sim.possession_phase() else {
            return Ok(());
        };
        if !live || !self.steps.is_multiple_of(SAMPLE_TICKS) {
            return Ok(());
        }
        let team = 1 - owner;
        let keeper = sim.keeper(team);
        let (sum, n) = sim
            .players()
            .iter()
            .enumerate()
            .filter(|(i, p)| p.team == team && *i != keeper && p.active())
            .fold((0.0, 0usize), |(s, n), (_, p)| (s + p.pos.x, n + 1));
        if n < SLIDE_MIN_PLAYERS {
            return Ok(());
        }
        let half = sim.config().pitch().half_length();
        let x = sum / n as f64 * sim.teams[team].attack_x;
        self.height[team].0 += x + half;
        self.height[team].1 += 1.0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;

    /// A match at kick-off with every player still, home in possession, the ball at
    /// `ball`, in the first half.
    fn still_match(ball: DVec2) -> Simulation {
        let mut sim = Simulation::new(shipped_config(7, 90).unwrap()).unwrap();
        for p in &mut sim.players {
            p.vel = DVec2::ZERO;
        }
        sim.ball.pos = ball.extend(0.0);
        sim.possession_phase = Some(0);
        sim.carrier = None;
        sim
    }

    /// One step, live or dead. A macro, not a function: only the central loop has functions
    /// that write the match (the structure rule), and this test seam writes the tick count.
    macro_rules! step {
        ($sink:expr, $sim:expr, $live:expr) => {{
            let sim: &mut Simulation = $sim;
            sim.tick += 1;
            if $live {
                sim.summary.live_ticks += 1;
            }
            $sink.on_step(sim).unwrap();
        }};
    }

    /// The home outfield players of group `g`, by roster place.
    fn of_group(sim: &Simulation, g: usize) -> Vec<usize> {
        sim.players
            .iter()
            .filter(|p| {
                p.team == 0
                    && p.slot != sim.teams[0].keeper_slot()
                    && group_of(sim.slot_position(0, p.slot)) == Some(g)
            })
            .map(|p| p.id)
            .collect()
    }

    fn at(name: &str) -> usize {
        field_names().iter().position(|x| x == name).unwrap()
    }

    fn ratio(v: &ShapeValues, side: usize, name: &str, den: &str) -> f64 {
        v[side][at(name)] / v[side][at(den)]
    }

    #[test]
    fn the_names_match_the_sizes() {
        assert_eq!(field_names().len(), SHAPE_FIELDS);
        assert_eq!(GROUP_FIELDS[LIVE], "live_ticks");
        assert_eq!(GROUP_FIELDS[SPRINTS], "sprints_fwd");
        assert_eq!(GROUP_FIELDS[SPEED], "speed_near_sum");
        assert_eq!(GROUP_FIELDS[PACE], "pace_stand");
        assert_eq!(GROUP_FIELDS[WIDTH], "width_near_in_sum");
        assert_eq!(TEAM_FIELDS[STOPS], "stops_throw_in");
        assert_eq!(TEAM_FIELDS[LINKS + 2], "links_total");
        assert_eq!(TEAM_FIELDS[CALM], "calm_ticks");
        assert_eq!(TEAM_FIELDS[DEAD], "dead_ticks");
        assert_eq!(TEAM_FIELDS[FAR + 4], "far_10s_30_45_pq");
    }

    /// Known positions give known depth and width: every home outfield player at y = +10
    /// and 15 m behind the ball in his attacking frame, the ball 20 m to his side.
    #[test]
    fn known_positions_give_known_depth_and_width() {
        let mut sim = still_match(DVec2::new(0.0, 20.0));
        let ax = sim.teams[0].attack_x;
        for p in sim.players.iter_mut().filter(|p| p.team == 0) {
            p.pos = DVec2::new(-15.0 * ax, 10.0);
        }
        let mut sink = ShapeSink::default();
        for _ in 0..40 {
            step!(sink, &mut sim, true);
        }
        let v = sink.finish();
        for g in ["cb", "fb", "cm", "st"] {
            let depth = ratio(
                &v,
                0,
                &format!("{g}.depth_in_sum"),
                &format!("{g}.depth_in_n"),
            );
            assert!((depth - 15.0).abs() < 1e-9, "{g} depth {depth}");
            let width = ratio(
                &v,
                0,
                &format!("{g}.width_near_in_sum"),
                &format!("{g}.width_near_in_n"),
            );
            assert!((width - 10.0).abs() < 1e-9, "{g} width {width}");
        }
        assert_eq!(
            v[0][at("cb.width_far_in_n")],
            0.0,
            "the ball is on his side"
        );
        assert_eq!(v[0][at("cb.depth_out_n")], 0.0, "home has the ball");
        assert!(v[1][at("cb.depth_out_n")] > 0.0, "away defends");
        assert_eq!(v[0][at("cb.pace_stand")], v[0][at("cb.live_ticks")]);
    }

    /// The centroid moving half as far as the ball gives a slope of 0.5, up and across.
    #[test]
    fn a_block_that_moves_half_as_far_as_the_ball_slides_one_half() {
        let mut sim = still_match(DVec2::ZERO);
        let mut sink = ShapeSink::default();
        let base: Vec<DVec2> = sim.players.iter().map(|p| p.pos).collect();
        for b in [-30.0, -10.0, 0.0, 15.0, 30.0] {
            let ball = DVec2::new(b, b / 2.0);
            sim.ball.pos = ball.extend(0.0);
            for (p, start) in sim.players.iter_mut().zip(&base) {
                p.pos = *start + ball * 0.5;
            }
            for _ in 0..SAMPLE_TICKS {
                step!(sink, &mut sim, true);
            }
        }
        let v = sink.finish();
        for (side, ph) in [(0, "in"), (1, "out")] {
            for dim in ["x", "y"] {
                let s = ratio(
                    &v,
                    side,
                    &format!("slide_{dim}_{ph}_sxy"),
                    &format!("slide_{dim}_{ph}_sxx"),
                );
                assert!((s - 0.5).abs() < 1e-9, "side {side} {dim} {ph}: {s}");
            }
        }
    }

    /// A run of 1.4 s at 8 m/s is one sprint and a run of 0.9 s none; forward and back follow
    /// the attacking frame. The speed is the displacement over 0.32 s, so a run reads 7 m/s
    /// or more (the sprint pace level) from its 14th tick to two ticks after it stops: 59
    /// ticks of a 70-tick run and 33 of a 45-tick run.
    #[test]
    fn a_sprint_lasts_a_second_and_takes_its_direction_from_the_frame() {
        let mut sim = still_match(DVec2::ZERO);
        let ax = sim.teams[0].attack_x;
        let i = of_group(&sim, 2)[0];
        let mut sink = ShapeSink::default();
        for _ in 0..20 {
            step!(sink, &mut sim, true);
        }
        for (ticks, way) in [(70, 1.0), (45, 1.0), (70, -1.0), (70, 0.0)] {
            for _ in 0..ticks {
                let v = if way == 0.0 {
                    DVec2::new(0.0, 8.0)
                } else {
                    DVec2::new(8.0 * ax * way, 0.0)
                };
                sim.players[i].vel = v;
                sim.players[i].pos += v / f64::from(TICKS_PER_SECOND);
                step!(sink, &mut sim, true);
            }
            sim.players[i].vel = DVec2::ZERO;
            for _ in 0..20 {
                step!(sink, &mut sim, true);
            }
        }
        let v = sink.finish();
        assert_eq!(v[0][at("cm.sprints_fwd")], 1.0);
        assert_eq!(v[0][at("cm.sprints_back")], 1.0);
        assert_eq!(v[0][at("cm.sprints_lat")], 1.0);
        assert_eq!(v[0][at("cm.pace_sprint")], 3.0 * 59.0 + 33.0);
    }

    /// The speed is the displacement over the last 0.32 s: a player who jitters on the spot
    /// at 3 m/s for single ticks stands, and one who moves steadily keeps his speed.
    #[test]
    fn the_speed_is_the_displacement_over_a_third_of_a_second() {
        let mut sim = still_match(DVec2::ZERO);
        let i = of_group(&sim, 0)[0];
        let j = of_group(&sim, 4)[0];
        let start = sim.players[i].pos;
        let mut sink = ShapeSink::default();
        for k in 0..100 {
            // Player i steps 6 cm out and back each tick; player j walks at 1.5 m/s.
            let out = if k % 2 == 0 { 0.06 } else { 0.0 };
            sim.players[i].pos = start + DVec2::new(out, 0.0);
            sim.players[i].vel = DVec2::new(3.0, 0.0);
            sim.players[j].vel = DVec2::new(1.5, 0.0);
            sim.players[j].pos += DVec2::new(1.5 / f64::from(TICKS_PER_SECOND), 0.0);
            step!(sink, &mut sim, true);
        }
        let v = sink.finish();
        // The first 16 ticks read the velocity: jog for i, walk for j.
        assert_eq!(v[0][at("cb.pace_jog")], 16.0);
        assert!(
            v[0][at("cb.pace_stand")] >= 84.0,
            "{}",
            v[0][at("cb.pace_stand")]
        );
        assert!(
            v[0][at("st.pace_walk")] >= 100.0,
            "{}",
            v[0][at("st.pace_walk")]
        );
    }

    /// A player 40 m from the ball who moves a quarter as far as it, the same way, has a
    /// far-player gain of 0.25 over 1 s and over 10 s.
    #[test]
    fn a_far_player_who_moves_a_quarter_as_far_as_the_ball_gains_a_quarter() {
        let mut sim = still_match(DVec2::ZERO);
        let i = of_group(&sim, 0)[0];
        let start = DVec2::new(sim.players[i].pos.x, 0.0);
        let ball0 = start + DVec2::new(40.0, 0.0);
        let mut sink = ShapeSink::default();
        for k in 0..60 {
            let q = DVec2::new(0.0, 0.1 * f64::from(k));
            sim.ball.pos = (ball0 + q).extend(0.0);
            // Everyone else stands on the ball, inside every far-player class.
            for p in &mut sim.players {
                p.pos = ball0 + q;
            }
            sim.players[i].pos = start + q * 0.25;
            for _ in 0..SAMPLE_TICKS {
                step!(sink, &mut sim, true);
            }
        }
        let v = sink.finish();
        for w in ["1s", "10s"] {
            let g = ratio(
                &v,
                0,
                &format!("far_{w}_30_45_pq"),
                &format!("far_{w}_30_45_qq"),
            );
            assert!((g - 0.25).abs() < 1e-9, "{w}: {g}");
        }
    }

    /// A pass from one home player to another is a link; an interception is none; a dead
    /// ball between two controls breaks the link.
    #[test]
    fn a_link_needs_the_same_side_and_unbroken_play() {
        let mut sim = still_match(DVec2::ZERO);
        let home: Vec<usize> = (0..ROSTER).filter(|&i| sim.players[i].team == 0).collect();
        let away: Vec<usize> = (0..ROSTER).filter(|&i| sim.players[i].team == 1).collect();
        let mut sink = ShapeSink::default();
        for (c, live) in [
            (Some(home[1]), true),
            (None, true),
            (Some(home[2]), true),
            (Some(away[3]), true),
            (Some(home[2]), true),
            (None, false),
            (Some(home[4]), true),
        ] {
            sim.carrier = c;
            step!(sink, &mut sim, live);
        }
        let v = sink.finish();
        assert_eq!(v[0][at("links_total")], 1.0);
        assert_eq!(v[1][at("links_total")], 0.0);
        assert_eq!(
            v[0][at("links_top2")],
            0.0,
            "one player received; the second place received nothing from him"
        );
    }

    /// The top-two share counts only the links between the two players who received most.
    #[test]
    fn the_top_two_share_counts_the_links_between_the_two_main_receivers() {
        let mut sim = still_match(DVec2::ZERO);
        let h: Vec<usize> = (0..ROSTER).filter(|&i| sim.players[i].team == 0).collect();
        let mut sink = ShapeSink::default();
        step!(sink, &mut sim, true);
        // 9 and 10 pass to each other 3 times each way; 4 passes to 9 twice.
        for (a, b, n) in [(h[9], h[10], 3), (h[10], h[9], 3), (h[4], h[9], 2)] {
            sink.links[a][b] += n;
        }
        let v = sink.finish();
        assert_eq!(v[0][at("links_top2")], 6.0);
        assert_eq!(v[0][at("links_total")], 8.0);
    }

    /// Dead-ball ticks under 2 m/s are calm; a stoppage counts by kind and restarting side.
    #[test]
    fn dead_balls_count_calm_ticks_and_stoppages_by_kind() {
        let mut sim = still_match(DVec2::ZERO);
        let mut sink = ShapeSink::default();
        step!(sink, &mut sim, false);
        for p in sim.players.iter_mut().filter(|p| p.team == 1) {
            p.vel = DVec2::new(3.0, 0.0);
        }
        step!(sink, &mut sim, false);
        let stop = Stoppage {
            tick: 2,
            kind: StoppageKind::ThrowIn,
            team: Some(1),
            spot: DVec2::ZERO,
        };
        sink.on_stoppage(&stop, &sim).unwrap();
        let goal = Stoppage {
            kind: StoppageKind::Goal,
            team: Some(0),
            ..stop
        };
        sink.on_stoppage(&goal, &sim).unwrap();
        let v = sink.finish();
        assert_eq!(v[0][at("dead_ticks")], 20.0);
        assert_eq!(v[0][at("calm_ticks")], 20.0);
        assert_eq!(v[1][at("calm_ticks")], 10.0);
        assert_eq!(v[1][at("stops_throw_in")], 1.0);
        assert_eq!(v[0][at("stops_kick_off")], 1.0);
    }
}
