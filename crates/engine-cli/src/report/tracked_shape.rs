//! The shared shape report: one code that measures team shape, running, keeper position and
//! ball-in-play time for an engine match and for a real tracked match converted into the
//! engine's tick format, so a definition difference cannot pose as an engine gap.
//!
//! The report takes frames at 25 a second: every second engine tick, counted from the half's
//! first tick, which is where a converted match holds its frames. A frame at an odd step from
//! its half's start is refused. A frame holds the ball and the 22 record places as the tick
//! record holds them (f32 metres), each place's player and position group or `None` off the
//! pitch, whether the ball is in play and who has it when the source says, the restarts
//! counted on it, and each team's attack direction.
//!
//! The definitions are the engine shape measures' ([`engine::measures::shape`]) at 25 frames
//! a second:
//! - A sample is a ball-in-play frame with a team in possession; positions are read every
//!   frame.
//! - Depth is the ball's x minus the player's x in his team's attacking frame; width is |y|;
//!   the ball is on a player's own flank when more than 5 m to his side of the centre line
//!   (his side the sign of his mean y over the half), on the far flank when more than 5 m to
//!   the other side.
//! - A player's speed is his displacement over the last 0.32 s (8 frames), over fewer frames
//!   in his first 0.32 s on the pitch, 0 on his first frame. A step of more than 1 m from one
//!   frame to the next (25 m/s, beyond any run) breaks his track, as a gap does: a tracking
//!   system's identity swap or jump, or the engine placing him for a kick-off, reads as no
//!   movement, and his speed and far-player windows start again from that frame. Pace levels: under 0.2, 2, 4,
//!   5.5 and 7 m/s, and 7 and over. A sprint is a run above 7 m/s for at least 1 s of play
//!   (25 frames): forward when x gains more than 3 m in the attacking frame, back when it
//!   loses more than 3 m, else lateral.
//! - The slide is the slope of the team's outfield centroid on the ball, up and across, in
//!   and out of possession, from frames with 9 or more outfield players.
//! - The far-player gain is Σ(p·q) / Σ(q·q) over windows of 1 s and 10 s of unbroken play,
//!   each starting every 0.2 s (5 frames), for windows that start with the player 30 to 45
//!   m or 45 m and more from the ball.
//! - Calm is the share of dead-ball outfield frames under 2 m/s. Stoppages count by the kind
//!   of restart and the side that takes it.
//! - Running, per side over every player on the pitch: metres (speed times 0.04 s each
//!   frame), frames on the pitch, ball-in-play frames in the speed zones of the real player
//!   file (2, 4, 5.5, 7 m/s), the sum of each player's peak speed and the players used.
//! - The keeper's distance from his own goal line, in and out of possession.
//! - Three figures that need no ball, possession or ball status, over every frame with 2 or
//!   more outfield players: the team's length (deepest to highest outfield player along x),
//!   width (across y) and spread (the mean distance of the outfield players from their
//!   centroid). A ball-free report (SoccerTrack) computes only these and the frame count,
//!   and no speed.
//!
//! [`TrackedShape::finish`] gives every figure as sums per side in [`field_names`] order;
//! the bands read them as `trk.<name>.<side>`.

use std::collections::VecDeque;

use crate::tracking::Group;

/// The outfield groups, in the report's order.
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
    "live_frames",
];
pub const GROUP_FIELD_COUNT: usize = 26;
/// The figures kept per side beyond the groups, in order, each with its family.
pub const SIDE_FIELDS: [&str; SIDE_FIELD_COUNT] = [
    "team.slide_x_in_sxy",
    "team.slide_x_in_sxx",
    "team.slide_x_out_sxy",
    "team.slide_x_out_sxx",
    "team.slide_y_in_sxy",
    "team.slide_y_in_sxx",
    "team.slide_y_out_sxy",
    "team.slide_y_out_sxx",
    "team.far_1s_30_45_pq",
    "team.far_1s_30_45_qq",
    "team.far_1s_gt45_pq",
    "team.far_1s_gt45_qq",
    "team.far_10s_30_45_pq",
    "team.far_10s_30_45_qq",
    "team.far_10s_gt45_pq",
    "team.far_10s_gt45_qq",
    "team.calm_frames",
    "team.dead_frames",
    "team.stops_throw_in",
    "team.stops_free_kick",
    "team.stops_goal_kick",
    "team.stops_corner",
    "team.stops_kick_off",
    "team.stops_penalty",
    "team.live_frames",
    "team.frames",
    "run.distance_m",
    "run.frames_on",
    "run.zone_walk",
    "run.zone_jog",
    "run.zone_run",
    "run.zone_hi",
    "run.zone_sprint",
    "run.peak_sum",
    "run.players_used",
    "keeper.in_sum",
    "keeper.in_n",
    "keeper.out_sum",
    "keeper.out_n",
    "free.length_sum",
    "free.width_sum",
    "free.spread_sum",
    "free.n",
];
pub const SIDE_FIELD_COUNT: usize = 43;
/// Figures per side: every group's, then the side's.
pub const TRACKED_FIELDS: usize = GROUPS.len() * GROUP_FIELD_COUNT + SIDE_FIELD_COUNT;

/// Every figure of a match, per side, home first, in [`field_names`] order.
pub type TrackedValues = [[f64; TRACKED_FIELDS]; 2];

/// Every figure's name, per side, in value order: `<group>.<field>`, then the side's.
pub fn field_names() -> Vec<String> {
    let mut out = Vec::with_capacity(TRACKED_FIELDS);
    for g in GROUPS {
        for f in GROUP_FIELDS {
            out.push(format!("{g}.{f}"));
        }
    }
    out.extend(SIDE_FIELDS.iter().map(|f| (*f).to_string()));
    out
}

/// The place of the figure `name` among [`field_names`].
pub fn place(name: &str) -> Option<usize> {
    if let Some(at) = SIDE_FIELDS.iter().position(|f| *f == name) {
        return Some(GROUPS.len() * GROUP_FIELD_COUNT + at);
    }
    let (g, f) = name.split_once('.')?;
    let g = GROUPS.iter().position(|x| *x == g)?;
    Some(g * GROUP_FIELD_COUNT + GROUP_FIELDS.iter().position(|x| *x == f)?)
}

/// Frames a second.
pub const FRAMES_PER_SECOND: f64 = 25.0;
/// Seconds per frame.
const DT: f64 = 1.0 / FRAMES_PER_SECOND;
/// The upper bounds of the pace levels below sprint, in m/s.
const PACE_TOPS: [f64; 5] = [0.2, 2.0, 4.0, 5.5, 7.0];
/// The upper bounds of the running zones below sprint, in m/s.
const ZONE_TOPS: [f64; 4] = [2.0, 4.0, 5.5, 7.0];
const SPRINT_MPS: f64 = 7.0;
/// A sprint lasts at least this many frames: 1 s.
const SPRINT_FRAMES: u32 = 25;
const SPRINT_RUN_M: f64 = 3.0;
const FLANK_M: f64 = 5.0;
/// The speed window: 0.32 s.
const SMOOTH_FRAMES: usize = 8;
/// A step longer than this from one frame to the next breaks a player's track, in metres.
const BREAK_M: f64 = 1.0;
/// Far-player windows start every this many frames: 0.2 s.
const FAR_EVERY: u64 = 5;
/// The far-player windows in starts: 1 s and 10 s.
const WINDOWS: [usize; 2] = [5, 50];
const FAR_BINS: [(f64, f64); 2] = [(30.0, 45.0), (45.0, f64::INFINITY)];
const CALM_MPS: f64 = 2.0;
const SLIDE_MIN_PLAYERS: usize = 9;
const ROSTER: usize = 22;
const PER_TEAM: usize = 11;

const fn gi(g: usize, f: usize) -> usize {
    g * GROUP_FIELD_COUNT + f
}

const fn si(f: usize) -> usize {
    GROUPS.len() * GROUP_FIELD_COUNT + f
}

// Group field places.
const DEPTH: usize = 0;
const WIDTH: usize = 4;
const PACE: usize = 12;
const SPEED: usize = 18;
const SPRINTS: usize = 22;
const LIVE: usize = 25;
// Side field places.
const SLIDE: usize = 0;
const FAR: usize = 8;
const CALM: usize = 16;
const DEAD: usize = 17;
const STOPS: usize = 18;
const MATCH_LIVE: usize = 24;
const FRAMES: usize = 25;
const RUN: usize = 26; // distance, frames_on, 5 zones, peak_sum, players_used
const KEEPER: usize = 35;
const FREE: usize = 39;

/// A restart, by the stoppage field it counts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restart {
    ThrowIn,
    FreeKick,
    GoalKick,
    Corner,
    KickOff,
    Penalty,
}

impl Restart {
    fn place(self) -> usize {
        match self {
            Restart::ThrowIn => 0,
            Restart::FreeKick => 1,
            Restart::GoalKick => 2,
            Restart::Corner => 3,
            Restart::KickOff => 4,
            Restart::Penalty => 5,
        }
    }

    /// The restart of a side file's event word, when the word is one.
    pub fn of_word(word: &str) -> Option<Self> {
        Some(match word {
            "throw_in" => Restart::ThrowIn,
            "free_kick" => Restart::FreeKick,
            "goal_kick" => Restart::GoalKick,
            "corner" => Restart::Corner,
            "kick_off" => Restart::KickOff,
            "penalty" => Restart::Penalty,
            _ => return None,
        })
    }

    /// The side file's word.
    pub fn word(self) -> &'static str {
        match self {
            Restart::ThrowIn => "throw_in",
            Restart::FreeKick => "free_kick",
            Restart::GoalKick => "goal_kick",
            Restart::Corner => "corner",
            Restart::KickOff => "kick_off",
            Restart::Penalty => "penalty",
        }
    }
}

/// A record place's player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Holder {
    /// A key unique to the player within the match.
    pub person: u32,
    pub group: Group,
}

/// One frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub tick: u32,
    /// The half, from 0.
    pub half: usize,
    pub ball: Option<[f32; 3]>,
    /// Each record place's position: team 0's eleven, then team 1's.
    pub players: [[f32; 2]; ROSTER],
    /// Each record place's player, `None` when the place's player is not on the pitch.
    pub on: [Option<Holder>; ROSTER],
    pub live: Option<bool>,
    pub possession: Option<usize>,
    /// The restarts counted on this frame, with the side that takes each.
    pub restarts: Vec<(Restart, usize)>,
    /// Each team's attack direction this half: +1 towards positive x.
    pub attack_x: [f64; 2],
}

/// What the report measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Everything.
    Full,
    /// Only the figures that need no ball, possession or speed.
    BallFree,
}

/// One player's samples kept by the ball's side until his own side is known.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Flanked {
    width: [[(f64, f64); 2]; 2],
    speed: [(f64, f64); 2],
    y: (f64, f64),
}

/// A record place's state as the report follows it.
#[derive(Debug, Clone, Default)]
struct Follow {
    key: Option<Holder>,
    flanked: Flanked,
    /// The sprint in progress: first x, last x (attacking frame), frames.
    sprint: Option<(f64, f64, u32)>,
    trail: VecDeque<[f64; 2]>,
    /// Far-player samples: position, ball, dead frames so far.
    ring: VecDeque<([f64; 2], [f64; 2], u64)>,
}

/// One player's running.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Runner {
    team: usize,
    peak: f64,
}

type SlideSums = [[[f64; 5]; 2]; 2];

/// The shared shape report of one match.
#[derive(Debug, Clone)]
pub struct TrackedShape {
    mode: Mode,
    pitch_half_length: f64,
    values: TrackedValues,
    follow: Vec<Follow>,
    slide: [SlideSums; 2],
    /// The running of each player by person key.
    runners: std::collections::BTreeMap<u32, Runner>,
    /// The half followed, its first tick and the last tick seen.
    half: Option<(usize, u32, u32)>,
    /// Frames since the half started, and dead frames so far.
    half_frames: u64,
    dead_frames: u64,
}

impl TrackedShape {
    /// A report for a match on a pitch `length` metres long.
    pub fn new(mode: Mode, length: f64) -> Self {
        Self {
            mode,
            pitch_half_length: length / 2.0,
            values: [[0.0; TRACKED_FIELDS]; 2],
            follow: vec![Follow::default(); ROSTER],
            slide: [[[[0.0; 5]; 2]; 2]; 2],
            runners: std::collections::BTreeMap::new(),
            half: None,
            half_frames: 0,
            dead_frames: 0,
        }
    }

    /// Every figure, with every open sample closed.
    pub fn finish(mut self) -> TrackedValues {
        for i in 0..ROSTER {
            self.flush(i);
            self.end_sprint(i);
        }
        for team in 0..2 {
            for dim in 0..2 {
                for phase in 0..2 {
                    let [n, b, c, bb, bc] = self.slide[team][dim][phase];
                    let at = si(SLIDE + dim * 4 + phase * 2);
                    if n > 0.0 {
                        self.values[team][at] = bc - b * c / n;
                        self.values[team][at + 1] = bb - b * b / n;
                    }
                }
            }
        }
        for r in self.runners.values() {
            self.values[r.team][si(RUN + 7)] += r.peak;
            self.values[r.team][si(RUN + 8)] += 1.0;
        }
        self.values
    }

    /// One frame. A frame at an odd step from its half's first tick, or not after the frame
    /// before, is refused.
    pub fn frame(&mut self, f: &Frame) -> Result<(), String> {
        match self.half {
            Some((h, start, last)) if h == f.half => {
                if (f.tick - start) % 2 != 0 || f.tick <= last {
                    return Err(format!(
                        "tick {} is not an even step after tick {last} from its half's first \
                         tick {start}: the report reads every second engine tick",
                        f.tick
                    ));
                }
                if f.tick != last + 2 {
                    // A gap: no speed or window reaches across it.
                    for p in &mut self.follow {
                        p.trail.clear();
                        p.ring.clear();
                    }
                }
                self.half = Some((h, start, f.tick));
            }
            Some((h, ..)) if f.half < h => {
                return Err(format!("half {} after half {h}", f.half));
            }
            _ => {
                for i in 0..ROSTER {
                    self.flush(i);
                    self.end_sprint(i);
                    self.follow[i].trail.clear();
                    self.follow[i].ring.clear();
                }
                self.half = Some((f.half, f.tick, f.tick));
                self.half_frames = 0;
            }
        }
        for team in 0..2 {
            self.values[team][si(FRAMES)] += 1.0;
        }
        self.free(f);
        if self.mode == Mode::Full {
            self.full(f);
        } else {
            for (i, holder) in f.on.iter().enumerate() {
                self.follow[i].key = *holder;
            }
        }
        self.half_frames += 1;
        Ok(())
    }

    /// The ball-free figures of a frame: length, width and spread of each side's outfield.
    fn free(&mut self, f: &Frame) {
        for team in 0..2 {
            let pts: Vec<[f64; 2]> = (team * PER_TEAM..(team + 1) * PER_TEAM)
                .filter(|&i| f.on[i].is_some_and(|h| h.group != Group::Keeper))
                .map(|i| [f64::from(f.players[i][0]), f64::from(f.players[i][1])])
                .collect();
            if pts.len() < 2 {
                continue;
            }
            let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
            let mut c = [0.0; 2];
            for p in &pts {
                for d in 0..2 {
                    lo[d] = lo[d].min(p[d]);
                    hi[d] = hi[d].max(p[d]);
                    c[d] += p[d];
                }
            }
            let n = pts.len() as f64;
            let c = [c[0] / n, c[1] / n];
            let spread = pts
                .iter()
                .map(|p| (p[0] - c[0]).hypot(p[1] - c[1]))
                .sum::<f64>()
                / n;
            let v = &mut self.values[team];
            v[si(FREE)] += hi[0] - lo[0];
            v[si(FREE + 1)] += hi[1] - lo[1];
            v[si(FREE + 2)] += spread;
            v[si(FREE + 3)] += 1.0;
        }
    }

    /// The figures that need the ball, its status, possession or speed.
    fn full(&mut self, f: &Frame) {
        let live = f.live == Some(true);
        if live {
            for team in 0..2 {
                self.values[team][si(MATCH_LIVE)] += 1.0;
            }
        }
        if f.live == Some(false) {
            self.dead_frames += 1;
        }
        for &(r, team) in &f.restarts {
            if team < 2 {
                self.values[team][si(STOPS + r.place())] += 1.0;
            }
        }
        let ball = f.ball.map(|b| [f64::from(b[0]), f64::from(b[1])]);
        let phase = f.possession.filter(|_| live);
        let sample_far = self.half_frames.is_multiple_of(FAR_EVERY);
        let mut centroid = [([0.0f64; 2], 0usize); 2];
        for i in 0..ROSTER {
            self.player(f, i, live, phase, ball, sample_far, &mut centroid);
        }
        if let (Some(owner), Some(ball)) = (phase, ball) {
            for (team, (sum, n)) in centroid.into_iter().enumerate() {
                if n < SLIDE_MIN_PLAYERS {
                    continue;
                }
                let ax = f.attack_x[team];
                let c = [sum[0] / n as f64, sum[1] / n as f64];
                let ph = usize::from(owner != team);
                for (dim, (b, c)) in [(ball[0] * ax, c[0] * ax), (ball[1], c[1])]
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
    }

    /// One record place's frame.
    #[allow(clippy::too_many_arguments)] // sdlc-debt: one call site in `full`; a struct of the frame's state would only rename these.
    fn player(
        &mut self,
        f: &Frame,
        i: usize,
        live: bool,
        phase: Option<usize>,
        ball: Option<[f64; 2]>,
        sample_far: bool,
        centroid: &mut [([f64; 2], usize); 2],
    ) {
        let team = i / PER_TEAM;
        let holder = f.on[i];
        if self.follow[i].key != holder {
            self.flush(i);
            self.end_sprint(i);
            self.follow[i].trail.clear();
            self.follow[i].ring.clear();
            self.follow[i].key = holder;
        }
        let Some(h) = holder else {
            return;
        };
        let pos = [f64::from(f.players[i][0]), f64::from(f.players[i][1])];
        let speed = self.speed(i, pos);
        // Running: every player on the pitch.
        {
            let v = &mut self.values[team];
            v[si(RUN)] += speed * DT;
            v[si(RUN + 1)] += 1.0;
            if live {
                let z = ZONE_TOPS.iter().take_while(|&&t| speed >= t).count();
                v[si(RUN + 2 + z)] += 1.0;
            }
            let r = self
                .runners
                .entry(h.person)
                .or_insert(Runner { team, peak: 0.0 });
            r.peak = r.peak.max(speed);
        }
        let ax = f.attack_x[team];
        if h.group == Group::Keeper {
            if let Some(owner) = phase {
                let ph = usize::from(owner != team);
                let v = &mut self.values[team];
                v[si(KEEPER + ph * 2)] += pos[0] * ax + self.pitch_half_length;
                v[si(KEEPER + ph * 2 + 1)] += 1.0;
            }
            return;
        }
        let Some(g) = h.group.outfield() else {
            return;
        };
        if f.live == Some(false) {
            let v = &mut self.values[team];
            v[si(DEAD)] += 1.0;
            if speed < CALM_MPS {
                v[si(CALM)] += 1.0;
            }
        }
        let x = pos[0] * ax;
        let (Some(owner), Some(ball)) = (phase, ball) else {
            self.end_sprint(i);
            return;
        };
        let ph = usize::from(owner != team);
        {
            let v = &mut self.values[team];
            v[gi(g, LIVE)] += 1.0;
            let level = PACE_TOPS.iter().take_while(|&&t| speed >= t).count();
            v[gi(g, PACE + level)] += 1.0;
            v[gi(g, DEPTH + ph * 2)] += ball[0] * ax - x;
            v[gi(g, DEPTH + ph * 2 + 1)] += 1.0;
        }
        let side = if ball[1] > FLANK_M {
            Some(1)
        } else if ball[1] < -FLANK_M {
            Some(0)
        } else {
            None
        };
        let fo = &mut self.follow[i];
        fo.flanked.y.0 += pos[1];
        fo.flanked.y.1 += 1.0;
        if let Some(s) = side {
            fo.flanked.speed[s].0 += speed;
            fo.flanked.speed[s].1 += 1.0;
            fo.flanked.width[s][ph].0 += pos[1].abs();
            fo.flanked.width[s][ph].1 += 1.0;
        }
        if speed > SPRINT_MPS {
            let run = fo.sprint.get_or_insert((x, x, 0));
            run.1 = x;
            run.2 += 1;
        } else {
            self.end_sprint(i);
        }
        centroid[team].0[0] += pos[0];
        centroid[team].0[1] += pos[1];
        centroid[team].1 += 1;
        if sample_far {
            self.far(i, team, pos, ball);
        }
    }

    /// Place `i`'s speed: his displacement over the last 8 frames, over fewer at first.
    fn speed(&mut self, i: usize, pos: [f64; 2]) -> f64 {
        let fo = &mut self.follow[i];
        if fo
            .trail
            .back()
            .is_some_and(|last| (pos[0] - last[0]).hypot(pos[1] - last[1]) > BREAK_M)
        {
            fo.trail.clear();
            fo.ring.clear();
        }
        let trail = &mut fo.trail;
        trail.push_back(pos);
        if trail.len() > SMOOTH_FRAMES + 1 {
            trail.pop_front();
        }
        let n = trail.len() - 1;
        if n == 0 {
            return 0.0;
        }
        let first = trail[0];
        (pos[0] - first[0]).hypot(pos[1] - first[1]) / (n as f64 * DT)
    }

    /// Place `i`'s far-player sample.
    fn far(&mut self, i: usize, team: usize, pos: [f64; 2], ball: [f64; 2]) {
        let dead = self.dead_frames;
        let ring = &mut self.follow[i].ring;
        ring.push_back((pos, ball, dead));
        if ring.len() > WINDOWS[1] + 1 {
            ring.pop_front();
        }
        let len = ring.len();
        for (w, &back) in WINDOWS.iter().enumerate() {
            if len <= back {
                continue;
            }
            let (p0, b0, d0) = ring[len - 1 - back];
            if d0 != dead {
                continue;
            }
            let d = (p0[0] - b0[0]).hypot(p0[1] - b0[1]);
            let Some(bin) = FAR_BINS.iter().position(|&(lo, hi)| d >= lo && d < hi) else {
                continue;
            };
            let dp = [pos[0] - p0[0], pos[1] - p0[1]];
            let db = [ball[0] - b0[0], ball[1] - b0[1]];
            let at = si(FAR + w * 4 + bin * 2);
            self.values[team][at] += dp[0] * db[0] + dp[1] * db[1];
            self.values[team][at + 1] += db[0] * db[0] + db[1] * db[1];
        }
    }

    /// Moves place `i`'s flank samples into his group's figures.
    fn flush(&mut self, i: usize) {
        let team = i / PER_TEAM;
        let fo = &mut self.follow[i];
        let Some(g) = fo.key.and_then(|h| h.group.outfield()) else {
            fo.flanked = Flanked::default();
            return;
        };
        let fl = std::mem::take(&mut fo.flanked);
        if fl.y.1 == 0.0 {
            return;
        }
        let mean = fl.y.0 / fl.y.1;
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

    /// Ends place `i`'s sprint in progress: one sprint of his group when it lasted 1 s.
    fn end_sprint(&mut self, i: usize) {
        let team = i / PER_TEAM;
        let fo = &mut self.follow[i];
        let (Some((x0, x1, frames)), Some(g)) =
            (fo.sprint.take(), fo.key.and_then(|h| h.group.outfield()))
        else {
            return;
        };
        if frames < SPRINT_FRAMES {
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
        self.values[team][gi(g, SPRINTS + way)] += 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LENGTH: f64 = 105.0;

    /// A frame of `half` at `tick` with every player on, standing at `players`, the ball at
    /// `ball`, live, home in possession, team 0 attacking positive x.
    fn frame(tick: u32, ball: [f32; 2], players: [[f32; 2]; ROSTER]) -> Frame {
        let mut on = [None; ROSTER];
        for (i, h) in on.iter_mut().enumerate() {
            let slot = i % PER_TEAM;
            let group = match slot {
                0 => Group::Keeper,
                1..=4 => Group::Cb,
                5 | 6 => Group::Fb,
                7 | 8 => Group::Cm,
                9 => Group::W,
                _ => Group::St,
            };
            *h = Some(Holder {
                person: i as u32,
                group,
            });
        }
        Frame {
            tick,
            half: 0,
            ball: Some([ball[0], ball[1], 0.0]),
            players,
            on,
            live: Some(true),
            possession: Some(0),
            restarts: Vec::new(),
            attack_x: [1.0, -1.0],
        }
    }

    fn still(x: f32, y: f32) -> [[f32; 2]; ROSTER] {
        [[x, y]; ROSTER]
    }

    fn at(name: &str) -> usize {
        place(name).unwrap_or_else(|| panic!("no field {name}"))
    }

    fn ratio(v: &TrackedValues, side: usize, num: &str, den: &str) -> f64 {
        v[side][at(num)] / v[side][at(den)]
    }

    fn run(frames: impl IntoIterator<Item = Frame>, mode: Mode) -> TrackedValues {
        let mut r = TrackedShape::new(mode, LENGTH);
        for f in frames {
            r.frame(&f).unwrap();
        }
        r.finish()
    }

    #[test]
    fn the_names_match_the_sizes_and_places() {
        let names = field_names();
        assert_eq!(names.len(), TRACKED_FIELDS);
        for (i, n) in names.iter().enumerate() {
            assert_eq!(place(n), Some(i), "{n}");
        }
        assert_eq!(SIDE_FIELDS[FAR + 4], "team.far_10s_30_45_pq");
        assert_eq!(SIDE_FIELDS[MATCH_LIVE], "team.live_frames");
        assert_eq!(SIDE_FIELDS[RUN + 8], "run.players_used");
        assert_eq!(SIDE_FIELDS[KEEPER + 2], "keeper.out_sum");
        assert_eq!(SIDE_FIELDS[FREE + 3], "free.n");
        assert_eq!(GROUP_FIELDS[LIVE], "live_frames");
    }

    /// A frame at an odd step from its half's first tick is refused; a new half restarts the
    /// count.
    #[test]
    fn a_frame_at_an_odd_step_is_refused() {
        let mut r = TrackedShape::new(Mode::Full, LENGTH);
        r.frame(&frame(7, [0.0, 0.0], still(0.0, 0.0))).unwrap();
        r.frame(&frame(9, [0.0, 0.0], still(0.0, 0.0))).unwrap();
        let err = r
            .frame(&frame(12, [0.0, 0.0], still(0.0, 0.0)))
            .unwrap_err();
        assert!(err.contains("not an even step"), "{err}");
        let mut f = frame(100, [0.0, 0.0], still(0.0, 0.0));
        f.half = 1;
        r.frame(&f).unwrap();
        f.tick = 103;
        assert!(r.frame(&f).is_err());
    }

    /// Every home outfield player 15 m behind a still ball and 10 m from the centre line, the
    /// ball 20 m to his side.
    #[test]
    fn depth_and_width_of_a_group_against_a_still_ball() {
        let mut p = still(0.0, 10.0);
        for (i, xy) in p.iter_mut().enumerate().take(PER_TEAM) {
            *xy = if i == 0 { [-50.0, 0.0] } else { [-15.0, 10.0] };
        }
        let v = run((0..40).map(|k| frame(2 * k, [0.0, 20.0], p)), Mode::Full);
        for g in ["cb", "fb", "cm", "w", "st"] {
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
            v[0][at("cb.depth_in_n")],
            4.0 * 40.0,
            "every frame is a sample"
        );
        assert_eq!(v[0][at("cb.width_far_in_n")], 0.0);
        assert_eq!(v[0][at("cb.depth_out_n")], 0.0);
        assert!(v[1][at("cb.depth_out_n")] > 0.0);
        assert_eq!(v[0][at("cb.pace_stand")], v[0][at("cb.live_frames")]);
    }

    /// The speed is the displacement over the last 8 frames: a player who steps 6 cm out and
    /// back each frame stands; one who moves 0.1 m a frame runs at 2.5 m/s.
    #[test]
    fn the_speed_is_the_displacement_over_eight_frames() {
        let frames = (0..60).map(|k| {
            let mut p = still(0.0, 0.0);
            p[1] = [if k % 2 == 0 { 0.06 } else { 0.0 }, 0.0];
            p[10] = [0.1 * k as f32, 0.0];
            frame(2 * k, [0.0, 0.0], p)
        });
        let v = run(frames, Mode::Full);
        // Place 1 (cb): after his first 8 frames the displacement over 8 frames is 0.
        assert!(
            v[0][at("cb.pace_stand")] >= 52.0,
            "{}",
            v[0][at("cb.pace_stand")]
        );
        // Place 10 (st): 0.1 m per 0.04 s is 2.5 m/s, a jog, from his second frame.
        assert_eq!(v[0][at("st.pace_jog")], 59.0);
        assert_eq!(v[0][at("st.pace_stand")], 1.0, "his first frame reads 0");
    }

    /// A step of more than 1 m in one frame breaks the track: the player who jumps 20 m reads
    /// 0 on that frame, then his run again, and the jump adds no metres.
    #[test]
    fn a_jump_breaks_the_track() {
        let frames = (0..30).map(|k| {
            let mut p = still(0.0, 0.0);
            let jump = if k >= 15 { 20.0 } else { 0.0 };
            p[3] = [0.1 * k as f32 + jump, 0.0];
            frame(2 * k, [0.0, 0.0], p)
        });
        let v = run(frames, Mode::Full);
        // 2.5 m/s each frame but the first and the jump's.
        assert_eq!(v[0][at("cb.pace_jog")], 28.0);
        assert_eq!(v[0][at("cb.pace_stand")], 2.0 + 30.0 * 3.0);
        let d = v[0][at("run.distance_m")];
        assert!((d - 28.0 * 0.1).abs() < 1e-4, "{d}");
        assert!((v[0][at("run.peak_sum")] - 2.5).abs() < 1e-4);
    }

    /// A run that reads over 7 m/s for 25 frames (32 frames at 7.5 m/s, smoothed over 8) is
    /// one sprint; for 24 frames, none. Forward follows the attacking frame.
    #[test]
    fn a_sprint_lasts_a_second() {
        for (moving, sprints) in [(32u32, 1.0), (31, 0.0)] {
            let mut frames = Vec::new();
            let mut x = 0.0f32;
            for k in 0..(moving + 20) {
                if (5..5 + moving).contains(&k) {
                    x += 0.3;
                }
                let mut p = still(0.0, 0.0);
                p[7] = [x, 0.0];
                frames.push(frame(2 * k, [0.0, 0.0], p));
            }
            let v = run(frames, Mode::Full);
            assert_eq!(v[0][at("cm.sprints_fwd")], sprints, "{moving} frames");
            assert_eq!(v[0][at("cm.pace_sprint")], f64::from(moving - 7));
        }
    }

    /// A block whose centroid follows the ball one to one slides with slope 1.0, up and
    /// across, in and out of possession.
    #[test]
    fn a_block_that_follows_the_ball_slides_one_to_one() {
        let mut frames = Vec::new();
        let mut k = 0;
        for b in [-30.0f32, -10.0, 0.0, 15.0, 30.0] {
            for _ in 0..5 {
                let p = still(b, b / 2.0);
                frames.push(frame(2 * k, [b, b / 2.0], p));
                k += 1;
            }
        }
        let v = run(frames, Mode::Full);
        for (side, ph) in [(0, "in"), (1, "out")] {
            for dim in ["x", "y"] {
                let s = ratio(
                    &v,
                    side,
                    &format!("team.slide_{dim}_{ph}_sxy"),
                    &format!("team.slide_{dim}_{ph}_sxx"),
                );
                assert!((s - 1.0).abs() < 1e-9, "side {side} {dim} {ph}: {s}");
            }
        }
    }

    /// A player 40 m from the ball who moves a quarter as far as it, the same way, gains a
    /// quarter over 1 s and over 10 s.
    #[test]
    fn a_far_player_who_moves_a_quarter_as_far_as_the_ball_gains_a_quarter() {
        let frames = (0..300).map(|k| {
            let q = 0.02 * k as f32;
            let mut p = still(40.0, q);
            p[1] = [0.0, q * 0.25];
            frame(2 * k, [40.0, q], p)
        });
        let v = run(frames, Mode::Full);
        for w in ["1s", "10s"] {
            let g = ratio(
                &v,
                0,
                &format!("team.far_{w}_30_45_pq"),
                &format!("team.far_{w}_30_45_qq"),
            );
            assert!((g - 0.25).abs() < 1e-6, "{w}: {g}");
        }
    }

    /// Dead-ball outfield frames under 2 m/s are calm; restarts count by kind and side; the
    /// ball-in-play frames count once per side.
    #[test]
    fn calm_stoppages_and_live_frames() {
        let mut frames = Vec::new();
        for k in 0..10u32 {
            let mut p = still(0.0, 0.0);
            // Away runs at 3 m/s on dead balls.
            for (i, xy) in p.iter_mut().enumerate().skip(PER_TEAM) {
                *xy = [0.12 * k as f32, i as f32];
            }
            let mut f = frame(2 * k, [0.0, 0.0], p);
            f.live = Some(k < 4);
            if k == 4 {
                f.restarts = vec![(Restart::ThrowIn, 1), (Restart::KickOff, 0)];
            }
            frames.push(f);
        }
        let v = run(frames, Mode::Full);
        assert_eq!(v[0][at("team.dead_frames")], 10.0 * 6.0);
        assert_eq!(v[0][at("team.calm_frames")], 10.0 * 6.0);
        // Away's first dead frame reads the run over 5 frames: 3 m/s, not calm.
        assert_eq!(v[1][at("team.calm_frames")], 0.0);
        assert_eq!(v[1][at("team.stops_throw_in")], 1.0);
        assert_eq!(v[0][at("team.stops_kick_off")], 1.0);
        assert_eq!(v[0][at("team.live_frames")], 4.0);
        assert_eq!(v[1][at("team.live_frames")], 4.0);
        assert_eq!(v[0][at("team.frames")], 10.0);
    }

    /// Running: metres from the smoothed speed, frames on, live frames by zone, each player's
    /// peak summed and the players used.
    #[test]
    fn running_zones_and_peak_speed() {
        let frames = (0..50).map(|k| {
            let mut p = still(0.0, 0.0);
            // Place 3 jogs at 3 m/s; place 4 runs at 6 m/s.
            p[3] = [0.12 * k as f32, 0.0];
            p[4] = [0.24 * k as f32, 0.0];
            let mut f = frame(2 * k, [0.0, 0.0], p);
            f.live = Some(k >= 10);
            f
        });
        let v = run(frames, Mode::Full);
        let distance = v[0][at("run.distance_m")];
        // 49 frames of movement each: 0.12 and 0.24 m per frame.
        assert!((distance - 49.0 * 0.36).abs() < 1e-4, "{distance}");
        assert_eq!(v[0][at("run.frames_on")], 50.0 * 11.0);
        assert_eq!(v[0][at("run.zone_jog")], 40.0);
        assert_eq!(v[0][at("run.zone_hi")], 40.0);
        assert_eq!(v[0][at("run.zone_walk")], 40.0 * 9.0);
        assert!((v[0][at("run.peak_sum")] - 9.0).abs() < 1e-4);
        assert_eq!(v[0][at("run.players_used")], 11.0);
    }

    /// The keeper's distance from his own goal line, in and out of possession.
    #[test]
    fn the_keepers_distance_from_his_goal_line() {
        let frames = (0..10).map(|k| {
            let mut p = still(0.0, 0.0);
            p[0] = [-42.5, 0.0];
            p[PER_TEAM] = [40.5, 0.0];
            frame(2 * k, [0.0, 0.0], p)
        });
        let v = run(frames, Mode::Full);
        assert!((ratio(&v, 0, "keeper.in_sum", "keeper.in_n") - 10.0).abs() < 1e-9);
        assert!((ratio(&v, 1, "keeper.out_sum", "keeper.out_n") - 12.0).abs() < 1e-9);
        assert_eq!(v[0][at("keeper.out_n")], 0.0);
    }

    /// Length, width and spread of the outfield, over every frame, the keeper left out.
    #[test]
    fn team_length_width_and_spread() {
        let mut p = still(0.0, 0.0);
        p[0] = [-50.0, 0.0];
        for (j, xy) in p.iter_mut().enumerate().take(PER_TEAM).skip(1) {
            // Ten outfield players on a 30 by 40 m rectangle's corners and centre line.
            *xy = match j % 4 {
                0 => [-15.0, -20.0],
                1 => [15.0, -20.0],
                2 => [-15.0, 20.0],
                _ => [15.0, 20.0],
            };
        }
        let mut f = frame(0, [0.0, 0.0], p);
        f.live = None;
        f.possession = None;
        let v = run([f], Mode::Full);
        assert!((v[0][at("free.length_sum")] - 30.0).abs() < 1e-9);
        assert!((v[0][at("free.width_sum")] - 40.0).abs() < 1e-9);
        assert!((v[0][at("free.spread_sum")] - 25.0).abs() < 1e-9);
        assert_eq!(v[0][at("free.n")], 1.0);
    }

    /// A ball-free report computes no speed and no figure of the ball: only the shape that
    /// needs neither, and the frame count.
    #[test]
    fn a_ball_free_report_computes_no_speed_and_no_ball_figure() {
        let frames = (0..60).map(|k| {
            let mut p = still(0.0, 0.0);
            p[3] = [0.3 * k as f32, 10.0];
            frame(2 * k, [5.0, 20.0], p)
        });
        let v = run(frames, Mode::BallFree);
        for (i, name) in field_names().iter().enumerate() {
            let kept = name.starts_with("free.") || name == "team.frames";
            if !kept {
                assert_eq!(v[0][i], 0.0, "{name}");
                assert_eq!(v[1][i], 0.0, "{name}");
            }
        }
        assert_eq!(v[0][at("free.n")], 60.0);
        assert_eq!(v[0][at("team.frames")], 60.0);
    }
}
