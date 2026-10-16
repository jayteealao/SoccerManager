//! The possession decision. The ball carrier scores its options (shoot, pass to each open
//! team-mate, dribble, clear, hold) and takes the highest; the team's pressing instruction
//! sets how many opponents press and from how far; every other player holds its formation
//! anchor, which the team's plan moves. Every agent decides every tick (product-owner choice;
//! `Tuning::decision_interval_ticks`).
//! A sent-off player takes no part in any decision. While the ball is dead, the referee sets
//! every target instead, and a restart kick comes from `restart_pass`.
//!
//! Every attribute read goes through the attribute contract ([`crate::contract`]): each
//! option's skill term reads the carrier's choose stage, the noise his choose and pressure
//! stages, each kick its execute stage (with the pressure stage when an opponent is close),
//! and each player's press reach, run reading, keeper range, and place keeping their own
//! per-player factors. The skill gate keeps a carrier from trying a chip or a take-on his
//! technique does not allow.
//!
//! The nearest hooks of the newer attributes act here too: marking sets how tight goal-side
//! cover tracks a carrier, off-ball movement brings an attacker beyond the offside line back
//! toward it and gives him space at his anchor, flair raises a forward pass's score,
//! determination widens a presser's reach while his side trails, eccentricity widens a
//! keeper's chase range, and the set-piece skills set a restart's delivery, a direct free
//! kick's strike and a throw-in's range.
//!
//! The defensive jobs split how well a player does a job from how much he does it. Work rate
//! alone sets how far he presses from; the press stage (positioning leading) leans his aim
//! goal-side of the carrier by `hooks.press_goal_side_m`. The shape stage (teamwork leading)
//! holds his place in both phases. The cover stage (positioning leading) sets the cover
//! defender's lead on a carrier's run and how late a back-line player steps with his line.
//! The recovery-run stage (work rate leading) sets who takes a recovery run first; the pace
//! pass reads its share for how often the run is a sprint.
//!
//! Each trait of play a player carries weighs on its choice here ([`crate::traits`]): the
//! scores of shots, passes, dribbles and holds, a carry's direction from a wide channel, a
//! shot's aim and power, a restart's target, the anchors of a side with the ball, the press
//! reach and a keeper's chase range. An override in force plays the trait's action instead.

use crate::contract::{self, Stage};
use crate::data::rules::StoppageKind;
use crate::math::{self, DVec2, segment_distance, toward};
use crate::modules::{
    CarrierPlan, CoverTrace, DecisionModule, MATES, MatchView, ModuleCard, OptionDraft,
    OptionDraws, ROSTER, RestartPass, Scored, ShotDraws, ShotKind, TackleChances, Targets,
    TargetsTrace,
};
use crate::pace::Job;
use crate::pitch;
use crate::plugin::OptionOffsets;
use crate::rules::offside;
use crate::shape;
use crate::sim::{WIDE_SPREAD, wide_of_goal};
use crate::streams::{Action, map_range};
use crate::team::PLAYERS_PER_TEAM;
use crate::traits::{
    CHANNEL_Y_M, EARLY_CROSS_M, LATE_RUN_DEPTH_M, LATE_RUN_WIDTH_M, SWITCH_M, TraitKind,
};
use crate::tuning::Tuning;
use serde_json::json;

/// A pass of this length or less that gains less than [`FORWARD_M`] toward goal recycles the
/// ball: tempo's patience is added to it (hook `pass.recycle`).
pub const RECYCLE_M: f64 = 20.0;
/// A pass is forward when it gains this many metres or more toward the attacked goal.
pub const FORWARD_M: f64 = 5.0;
/// A short regain: when the side that had the ball acts on it again within this many seconds
/// of the other side's first touch, with no dead ball between, and the other side completed
/// no pass and took no shot, the other side never had the ball. The primitives' possession
/// count and the real-team table both use it.
pub const REGAIN_S: f64 = 2.0;
/// A pass is long when it covers this many metres or more: the build-up term weighs such a
/// pass from the side's own third (hook `pass.build_up`), and the long-ball statistics count
/// it.
pub const LONG_BALL_M: f64 = 32.0;
/// The metres of a goal kick's score one unit of build-up adds per 45 m of the kick, and the
/// metres one unit of build-up toward going long adds to the goal kick's reach (hook
/// `pass.build_up`).
pub const BUILD_UP_M: f64 = 45.0;
pub const GOAL_KICK_REACH_M: f64 = 25.0;

/// The regain window and the fast break's time: for this many seconds after a side wins the
/// ball in open play, while it keeps it, the side's on-winning terms act (hook
/// `regain.counter`); and a possession that starts with a regain at or behind
/// [`COUNTER_START_M`] and reaches the attacked final third within it is a fast break. The
/// signature command and the measures share it. Set from the counter-attacks Wyscout's
/// analysts tagged in the four fit leagues: 80 percent of the tagged possessions that reach
/// the final third reach it within 12.6 s.
pub const COUNTER_WINDOW_S: f64 = 13.0;
/// The most advanced start of a fast break, in metres past halfway toward the attacked goal
/// (below 0 inside the side's own half): 80 percent of the tagged counter-attacks start at or
/// behind it.
pub const COUNTER_START_M: f64 = -7.0;
/// The counter-press figures read the losing side's pressures within this many seconds of an
/// open-play loss, and its regains within the same time.
pub const COUNTERPRESS_S: f64 = 5.0;
/// The last counter-press pressure is read up to this many seconds after the loss.
pub const COUNTERPRESS_LAST_S: f64 = 10.0;
/// Metres from the defensive line, per unit of the side's on-winning weight, within which an
/// attack-line player may start a run in behind while the regain window is open.
pub const COUNTER_RUN_M: f64 = 10.0;

/// The press triggers, in the order of [`crate::sim::Simulation`]'s trigger ticks: a pass
/// toward the passer's own goal, a gain after the ball came too fast to control, a receiver
/// facing his own goal, and a pass to a full-back or into a wide channel.
pub const TRIGGERS: [&str; 4] = ["back_pass", "poor_touch", "facing_back", "ball_wide"];
pub const TRIGGER_BACK_PASS: usize = 0;
pub const TRIGGER_POOR_TOUCH: usize = 1;
pub const TRIGGER_FACING_BACK: usize = 2;
pub const TRIGGER_WIDE: usize = 3;
/// A pass is a back pass when its direction's cosine with the passer's attack direction is
/// below minus this (more than 60 degrees from square, backward).
pub const BACK_PASS_COS: f64 = 0.5;
/// A receiver faces his own goal when his facing's cosine with his attack direction is below
/// minus this.
pub const FACING_BACK_COS: f64 = 0.5;
/// The press-trigger window (hooks `press.trigger.*`): for this many seconds after a trigger
/// the pressing side switches on fires, while the other side keeps the ball, the side's press
/// reach takes [`TRIGGER_REACH`] and [`TRIGGER_PRESSERS`] more players press, and its nearest
/// outfield player presses whatever his reach, at a sprint (an urgent press job).
pub const TRIGGER_WINDOW_S: f64 = 3.0;
pub const TRIGGER_REACH: f64 = 1.5;
pub const TRIGGER_PRESSERS: usize = 1;
/// Force the opponent (hook `press.force`): metres per unit of force the presser's aim moves
/// across, toward the centre to show the carrier wide (force above 0) or toward the near
/// touchline to show him inside (below 0).
pub const FORCE_M: f64 = 2.0;
/// The offside trap (hook `line.offside_trap`): the back line steps up this many metres
/// together for this many seconds once the trap starts.
pub const TRAP_STEP_M: f64 = 5.0;
pub const TRAP_WINDOW_S: f64 = 2.0;
/// Stop short goal kicks (hook `press.screen_short`) and the goal-kick pick: a team-mate of
/// the kicker with an opponent within this many metres of him is screened.
pub const SCREEN_M: f64 = 4.0;
/// The metres of a goal kick's score a screened team-mate loses.
pub const SCREENED_M: f64 = 20.0;
/// The score a pass from the passer's own penalty area to a screened team-mate loses in the
/// carrier's open-play choice (the keeper's goal kick included): the same read as the
/// restart pick's [`SCREENED_M`], in the option scores' units, where it marks the pass as
/// one he expects to lose, as [`OFFSIDE_READ_COST`] does.
pub const SCREENED_COST: f64 = 1.0;
/// The tackle decision (hook `tackle.decide`): the cost of a foul's chance, in units of the
/// win chance, at no danger; the danger of a foul doubles it near the own goal. Set from the
/// probe of the default sides: at 0.1 a defender declines only a tackle he is far likelier to
/// foul in than to win, and the equal suite's goals stay within 0.05 a match of the automatic
/// tackle's (at 0.25 seed 9 rises 0.29, at 0.5 both seeds about 0.35, over 200 matches).
pub const TACKLE_FOUL_COST: f64 = 0.1;

/// Press triggers (hooks `press.trigger.*`): the factor on pressing side `team`'s press reach,
/// [`TRIGGER_REACH`] while the window of a trigger its plan switches on is open, else 1.
pub(crate) fn trigger_factor(view: &MatchView<'_>, team: usize) -> f64 {
    if trigger_open(view, team) {
        TRIGGER_REACH
    } else {
        1.0
    }
}

/// Press triggers: the extra pressers of side `team`, [`TRIGGER_PRESSERS`] while the window of
/// a trigger its plan switches on is open, else 0.
pub(crate) fn trigger_pressers(view: &MatchView<'_>, team: usize) -> usize {
    if trigger_open(view, team) {
        TRIGGER_PRESSERS
    } else {
        0
    }
}

/// The active outfield player of side `team` (not its keeper `keeper`) nearest `at`, with his
/// distance.
fn nearest_outfield(
    view: &MatchView<'_>,
    team: usize,
    keeper: usize,
    at: DVec2,
) -> Option<(f64, usize)> {
    view.players()
        .iter()
        .enumerate()
        .filter(|&(i, p)| p.team == team && p.active() && i != keeper)
        .map(|(i, p)| ((p.pos - at).length(), i))
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

/// Whether the window of a press trigger side `team`'s plan switches on is open.
pub(crate) fn trigger_open(view: &MatchView<'_>, team: usize) -> bool {
    let plan = &view.teams()[team].plan;
    let on = [
        plan.trigger_back_pass,
        plan.trigger_poor_touch,
        plan.trigger_facing_back,
        plan.trigger_wide,
    ];
    on.iter().enumerate().any(|(k, &on)| {
        on && view
            .since_trigger_s(team, k)
            .is_some_and(|s| s < TRIGGER_WINDOW_S)
    })
}

/// The line of engagement's press start (hook `press.engage`): `true` while the ball lies
/// within side `team`'s engage line, a share of the pitch length from its own goal line; the
/// default share 1 never limits.
pub(crate) fn engaged(view: &MatchView<'_>, team: usize, ball: DVec2) -> bool {
    let share = view.teams()[team].plan.engage_line;
    if share >= 1.0 {
        return true;
    }
    let pitch = view.pitch();
    let from_own_line = ball.x * view.teams()[team].attack_x + pitch.half_length();
    from_own_line < share * pitch.length()
}

/// On winning the ball (hook `regain.counter`): side `team`'s on-winning weight while its
/// regain window is open, [`COUNTER_WINDOW_S`] after it won the ball in open play and still
/// on it; 0 otherwise, and 0 at the default level.
pub(crate) fn counter_weight(view: &MatchView<'_>, team: usize) -> f64 {
    match view.since_regain_s(team) {
        Some(s) if s < COUNTER_WINDOW_S => view.teams()[team].plan.counter,
        _ => 0.0,
    }
}

/// On losing the ball and counter-press length (hooks `loss.counter_press` and
/// `loss.counter_press_window`): the factor on how far from the carrier side `team`'s nearest
/// players start to press; how many press stays the pressing level's. While its counter-press
/// window is open, `counter_press_s` seconds after it lost the ball in open play with the
/// other side still on it, the factor is 1 plus its counter-press, never below 0; otherwise 1.
/// The default level's counter-press is 0, so a side presses as its pressing level says.
pub(crate) fn counter_press_factor(view: &MatchView<'_>, team: usize) -> f64 {
    let plan = &view.teams()[team].plan;
    match view.since_loss_s(team) {
        Some(s) if s < plan.counter_press_s => (1.0 + plan.counter_press).max(0.0),
        _ => 1.0,
    }
}

/// The channel of a point `y` across the pitch for a side attacking `attack_x`: 0 the side's
/// left (where its left-back stands), 1 the centre, 2 its right. A point more than
/// [`contract::CROSS_WIDE_M`] from the centre line is in a wide channel.
pub fn channel(y: f64, attack_x: f64) -> usize {
    let across = y * attack_x;
    if across < -contract::CROSS_WIDE_M {
        0
    } else if across > contract::CROSS_WIDE_M {
        2
    } else {
        1
    }
}

/// A kick the carrier decided on this tick. A clearance is kicked away from danger to no
/// team-mate, and is counted apart from a pass.
#[derive(Debug, Clone, Copy)]
pub enum Kick {
    Pass { dir: DVec2, speed: f64, loft: f64 },
    Shot { dir: DVec2, speed: f64, loft: f64 },
    Clear { dir: DVec2, speed: f64, loft: f64 },
}

impl Kick {
    /// The direction, ground speed and vertical speed of the kick.
    pub fn flight(self) -> (DVec2, f64, f64) {
        match self {
            Kick::Pass { dir, speed, loft }
            | Kick::Shot { dir, speed, loft }
            | Kick::Clear { dir, speed, loft } => (dir, speed, loft),
        }
    }
}

/// The carrier's scored options on one tick. `None` is an option the carrier does not have:
/// no shot beyond the shooting range, no pass without an open team-mate, and no dribble or
/// hold for a goalkeeper.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub shot: Option<f64>,
    /// The best pass and the team-mate's roster index.
    pub pass: Option<(f64, usize)>,
    pub dribble: Option<f64>,
    pub clear: f64,
    pub hold: Option<f64>,
    pub nearest_opp_pos: DVec2,
}

/// The option a carrier takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Shot,
    Pass,
    Dribble,
    Clear,
    Hold,
}

impl Choice {
    pub fn name(self) -> &'static str {
        match self {
            Choice::Shot => "shot",
            Choice::Pass => "pass",
            Choice::Dribble => "dribble",
            Choice::Clear => "clear",
            Choice::Hold => "hold",
        }
    }
}

/// Metres short of the second-last defender at which a lone forward holds the line, so he
/// stays onside.
pub(crate) const ONSIDE_MARGIN: f64 = 0.5;
/// The offside lag: an attacker reads the offside line as it stood up to this many seconds
/// ago, the whole lag at the lowest off-ball skill and none at the highest (his off-ball run
/// stage share), for every side, so a line that steps up can catch him beyond it. A modest
/// size, not fitted to the real offsides a match: the runner's late read is one of the three
/// causes of an offside (with the passer's misread, [`OFFSIDE_MISREAD_M`], and a line that
/// steps up), and with almost no runs in behind at the default no size of it alone reaches
/// the real rate.
pub const OFFSIDE_LAG_S: f64 = 3.0;
/// The passer's misread of the offside line: the spread, in metres, of a passer's read of
/// where a team-mate stands against the second-last defender, at the lowest read (vision and
/// decisions under the press he feels, both at share 0); none at the highest. A modest size,
/// not fitted to the real offsides a match, for the same reason as [`OFFSIDE_LAG_S`].
pub const OFFSIDE_MISREAD_M: f64 = 4.0;
/// The score a pass loses when its passer reads the receiver as offside.
pub const OFFSIDE_READ_COST: f64 = 1.0;
/// The spread of the sharpest read, in metres: even a perfect reader cannot split a level
/// receiver.
const OFFSIDE_READ_FLOOR_M: f64 = 0.1;

/// The chance a passer whose read is `read` (0 to 1: the mean of his vision share and his
/// decisions under the press he feels) reads a team-mate standing `beyond` metres beyond
/// the second-last defender (below 0 when short of him) as offside: a logistic step at the
/// line, sharp for a good read and spread over [`OFFSIDE_MISREAD_M`] for the worst, so a weak
/// reader plays some passes to a receiver beyond the line and passes up some onside ones.
pub(crate) fn offside_read(beyond: f64, read: f64) -> f64 {
    let spread = OFFSIDE_MISREAD_M * (1.0 - read.clamp(0.0, 1.0)) + OFFSIDE_READ_FLOOR_M;
    1.0 / (1.0 + math::exp(-1.7 * beyond / spread))
}

/// How far player `i`'s read of his side's offside line trails the live one: the line's
/// speed toward the goal his side attacks (`speed`, below 0 while the line steps up) times
/// his lag ([`OFFSIDE_LAG_S`]). Added to the live line, it keeps him beyond a line that has
/// just stepped up.
pub(crate) fn line_lag(view: &MatchView<'_>, i: usize, speed: f64) -> f64 {
    let share = view
        .skills(i)
        .share(Stage::OFF_BALL_RUN_EXECUTE)
        .clamp(0.0, 1.0);
    -speed * OFFSIDE_LAG_S * (1.0 - share)
}

/// The most opponents a pressing instruction can send at the carrier.
pub const MAX_PRESSERS: usize = 4;
/// The furthest ahead, in seconds, an average presser aims along the carrier's run: beyond
/// it the carrier's run is too uncertain to chase. A presser's intercept stage (anticipation)
/// scales it.
const INTERCEPT_HORIZON_S: f64 = 1.0;

/// The distance in metres from the ball within which an average keeper comes off his line
/// for a loose ball; his rush stage scales it.
const KEEPER_CHASE_M: f64 = 16.0;

/// Where a player at `p` with top speed `s` meets a carrier at `c` running at `v`: `c + v t`,
/// where `t` is the smallest positive time with `|c + v t - p| = s t`, capped at `horizon`
/// seconds. With no such time (a carrier running away faster than the player) the point is
/// the carrier.
pub(crate) fn intercept(p: DVec2, s: f64, c: DVec2, v: DVec2, horizon: f64) -> DVec2 {
    let d = c - p;
    let a = v.dot(v) - s * s;
    let b = 2.0 * d.dot(v);
    let k = d.dot(d);
    let t = if a.abs() < 1e-9 {
        (b < 0.0).then(|| -k / b)
    } else {
        let disc = b * b - 4.0 * a * k;
        if disc < 0.0 {
            None
        } else {
            let r = disc.sqrt();
            let (t1, t2) = ((-b - r) / (2.0 * a), (-b + r) / (2.0 * a));
            let (lo, hi) = (t1.min(t2), t1.max(t2));
            if lo > 0.0 {
                Some(lo)
            } else if hi > 0.0 {
                Some(hi)
            } else {
                None
            }
        }
    };
    match t {
        Some(t) => c + v * t.min(horizon),
        None => c,
    }
}

/// What kind of pass a kick is, for the stage it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassKind {
    /// A ground pass up to 25 m.
    Pass,
    /// A lofted pass over 25 m: the gated chip.
    Chip,
    /// A pass from a wide channel into the penalty area the carrier attacks.
    Cross,
    /// A keeper's throw, up to 30 m.
    Throw,
    /// A keeper's kick, over 30 m.
    Kick,
}

impl PassKind {
    /// The kind of a pass `d` metres long from `from` to `to` by a carrier attacking
    /// `attack_x`; `keeper` when the carrier keeps goal.
    pub fn of(
        from: DVec2,
        to: DVec2,
        d: f64,
        attack_x: f64,
        keeper: bool,
        pitch: &crate::pitch::Pitch,
    ) -> Self {
        if keeper {
            if d <= contract::THROW_M {
                PassKind::Throw
            } else {
                PassKind::Kick
            }
        } else if from.y.abs() > contract::CROSS_WIDE_M && pitch.in_penalty_area(to, attack_x) {
            PassKind::Cross
        } else if d > contract::CHIP_M {
            PassKind::Chip
        } else {
            PassKind::Pass
        }
    }

    /// The choose stage of an outfield pass of this kind.
    fn choose(self) -> Stage {
        match self {
            PassKind::Chip => Stage::CHIP_CHOOSE,
            PassKind::Cross => Stage::CROSS_CHOOSE,
            _ => Stage::PASS_CHOOSE,
        }
    }

    /// The execute and pressure stages of a pass of this kind; a keeper's distribution has
    /// no pressure stage.
    fn execute(self) -> (Stage, Option<Stage>) {
        match self {
            PassKind::Pass => (Stage::PASS_EXECUTE, Some(Stage::PASS_PRESSURE)),
            PassKind::Chip => (Stage::CHIP_EXECUTE, Some(Stage::CHIP_PRESSURE)),
            PassKind::Cross => (Stage::CROSS_EXECUTE, Some(Stage::CROSS_PRESSURE)),
            PassKind::Throw => (Stage::KEEPER_THROW_EXECUTE, None),
            PassKind::Kick => (Stage::KEEPER_KICK_EXECUTE, None),
        }
    }
}

/// `true` when player `c` is pressed: his pressure intensity passes the one threshold
/// ([`crate::pressure::pressed`]).
pub(crate) fn pressed(view: &MatchView<'_>, c: usize) -> bool {
    let p = view.player(c);
    crate::pressure::pressed(view.players(), p, view.attack_x(p.team))
}

/// The distance in metres from player `c` to the nearest active opponent; infinite with none.
pub(crate) fn nearest_opponent_m(view: &MatchView<'_>, c: usize) -> f64 {
    let me = view.player(c);
    view.players()
        .iter()
        .filter(|p| p.team != me.team && p.active())
        .map(|p| (p.pos - me.pos).length())
        .fold(f64::INFINITY, f64::min)
}

/// The weaker-foot cost, in rating points before the floor's shrink, of a kick by player `c`
/// in direction `dir` over `distance` metres ([`contract::foot`]): 0 on his strong side, and
/// 0 with time to shift the ball, which he has at a dead ball (`dead_ball`) or when he is
/// not pressed and has held the ball `contract.foot.shift_s` seconds. 0 with no foot block.
pub fn foot_cost(
    view: &MatchView<'_>,
    c: usize,
    dir: DVec2,
    distance: f64,
    dead_ball: bool,
) -> f64 {
    let t = view.tuning();
    let ft = t.contract.foot();
    if ft.cost_cap <= 0.0 {
        return 0.0;
    }
    let foot = view.foot(c);
    let side = contract::foot::kick_side(view.player(c).facing, dir, foot, ft.either_deg);
    if side == contract::foot::KickSide::Strong {
        return 0.0;
    }
    let held = f64::from(view.tick().saturating_sub(view.control_since())) * t.dt;
    let has_time = dead_ball || (held >= ft.shift_s && !pressed(view, c));
    let nearest = nearest_opponent_m(view, c);
    contract::foot::cost_points(foot, side, has_time, nearest, distance, &ft)
}

/// The execution share of player `c`'s kick: the execute stage's share, blended with the
/// pressure stage's when an opponent is close, each lowered by the weaker-foot cost `foot`
/// (rating points, shrunk to each stage's floor when the person block is present), then moved
/// down by `penalty` in log-odds (a gated skill the body cannot pull off).
fn execution(
    view: &MatchView<'_>,
    c: usize,
    execute: Stage,
    pressure: Option<Stage>,
    pressed: bool,
    penalty: f64,
    foot: f64,
) -> f64 {
    let ct = &view.tuning().contract;
    // The shared floor binds only with a person block, as in the state rebuild.
    let floor = ct.person.map(|p| p.floor_drop);
    let share = contract::foot::kick_share(
        view.skills(c).stages,
        view.base_skills(c).stages,
        execute,
        pressure.filter(|_| pressed),
        foot,
        floor,
        ct,
    );
    if penalty > 0.0 && share > 0.0 && share < 1.0 {
        contract::logistic(contract::logit(share) - penalty)
    } else {
        share
    }
}

/// The farthest team-mate a throw-in looks for, in metres, and the throw's vertical speed.
pub(crate) const THROW_RANGE: f64 = 15.0;
pub(crate) const THROW_LOFT: f64 = 2.0;
/// How far a goalkeeper's clearance travels before it stops, in metres, and its vertical
/// speed.
pub(crate) const CLEARANCE_DISTANCE: f64 = 55.0;
pub(crate) const CLEARANCE_LOFT: f64 = 6.0;

/// The ground speed of a kick that reaches a team-mate `d` metres away at the pass arrival
/// speed. A ground pass decelerates by friction the whole way. A lofted pass flies for
/// `2 * loft / g` seconds with no friction, so it needs less speed: the flight covers
/// `speed * T` and the roll `speed^2 / (2 f)`, which puts the ball at the team-mate with the
/// arrival speed left.
pub(crate) fn kick_speed(d: f64, loft: f64, t: &Tuning) -> f64 {
    let f = t.ground_friction;
    let a = t.pass_arrival_speed;
    let speed = if loft <= 0.0 {
        (2.0 * f * d).sqrt() + a
    } else {
        let flight = 2.0 * loft / t.gravity;
        let reach = d + a * a / (2.0 * f);
        f * (-flight + (flight * flight + 2.0 * reach / f).sqrt())
    };
    speed.min(t.pass_max_speed)
}

/// How long a pass along the ground to a team-mate `d` metres away takes to reach him, in
/// seconds: the distance over the mean of its kick speed ([`kick_speed`]) and its arrival
/// speed.
pub(crate) fn pass_time(d: f64, t: &Tuning) -> f64 {
    2.0 * d / (kick_speed(d, 0.0, t) + t.pass_arrival_speed)
}

/// The decision hook's offsets as trace detail.
pub(crate) fn offsets_json(f: &OptionOffsets) -> serde_json::Value {
    json!({"shoot": f.shoot, "pass": f.pass, "dribble": f.dribble, "clear": f.clear, "hold": f.hold})
}

/// Rotates a unit vector by `angle` radians.
pub(crate) fn rotate(v: DVec2, angle: f64) -> DVec2 {
    let (s, c) = math::sin_cos(angle);
    DVec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Decision maker version 1: formation anchors, the lone forward's line hold, pressing,
/// goal-side cover and chasing for every player, and the scored options, the choice, and
/// the kicks for the carrier.
pub struct DecisionV1;

impl DecisionModule for DecisionV1 {
    fn lapse_chance(&self, view: &MatchView<'_>, i: usize) -> f64 {
        lapse_chance(view, i)
    }

    fn tackle(
        &self,
        view: &MatchView<'_>,
        tackler: usize,
        carrier: usize,
        chances: TackleChances,
    ) -> Option<TackleChances> {
        tackle_decision(view, tackler, carrier, chances)
    }

    fn targets(&self, view: &MatchView<'_>) -> Targets {
        let players = view.players();
        let teams = view.teams();
        let t = view.tuning();
        let ball = view.ball();
        let ball_xy = ball.xy();
        let n = players.len();
        let keepers = [view.keeper(0), view.keeper(1)];
        let mut targets = [DVec2::ZERO; ROSTER];
        for (slot, p) in targets.iter_mut().zip(players) {
            *slot = p.target;
        }
        match view.carrier() {
            Some(c) => {
                let team = players[c].team;
                let def = 1 - team;
                // Teammates hold their anchors; the nearest opponents press, as many and from
                // as far as the defending team's pressing instruction says, less one for each
                // player the defending team has lost.
                // Inside its counter-press window the side's players press from farther, or
                // from nearer, or not at all.
                // Beyond the side's line of engagement nobody presses; inside a trigger's
                // window the press reaches farther with more players.
                let count = if engaged(view, def, ball_xy) {
                    (teams[def].pressers() + trigger_pressers(view, def)).min(MAX_PRESSERS)
                } else {
                    0
                };
                let reach = teams[def].plan.press_distance
                    * counter_press_factor(view, def)
                    * trigger_factor(view, def);
                // While the defending side trails, each presser's determination widens his
                // reach.
                let goals = view.goals();
                let behind = goals[def] < goals[team];
                let behind_effort = t.contract.hooks().behind_effort;
                let mut pressers = [(f64::INFINITY, usize::MAX); MAX_PRESSERS];
                // The carrier's team is in possession.
                let phase = Some(team);
                let anchors = shape::anchors(view, phase);
                for i in 0..n {
                    let p = &players[i];
                    if !p.active() {
                        continue;
                    }
                    // With the ball, a player's traits move his anchor.
                    let anchor = if p.team == team && !view.traits(i).is_empty() {
                        trait_anchor(view, i, anchors[i])
                    } else {
                        anchors[i]
                    };
                    // In both phases, a player who keeps his place less well than the
                    // average (shape stage under 1) trails his anchor by the shortfall; the
                    // carrier and the keepers take theirs.
                    targets[i] = if i == c || i == keepers[p.team] {
                        anchor
                    } else {
                        lagged(p, anchor)
                    };
                    if p.team != team && i != keepers[def] && count > 0 {
                        let d = (p.pos - ball_xy).length();
                        // Each presser's own reach: his press effort (work rate alone) scales
                        // the team's pressing distance.
                        let effort = if behind {
                            1.0 + behind_effort * p.derived.occasion.determination
                        } else {
                            1.0
                        };
                        // A player who presses relentlessly presses from farther.
                        let relentless = 1.0
                            + view
                                .traits(i)
                                .weight(TraitKind::PressesRelentlessly, t.contract.traits());
                        if d < reach * p.derived.knobs.press * effort * relentless
                            && d < pressers[count - 1].0
                        {
                            // Insert in distance order among the first `count` places.
                            let mut k = count - 1;
                            while k > 0 && pressers[k - 1].0 > d {
                                pressers[k] = pressers[k - 1];
                                k -= 1;
                            }
                            pressers[k] = (d, i);
                        }
                    }
                }
                // Inside a press trigger's window the side's nearest outfield player presses
                // whatever his reach; the pace pass sends him at a sprint.
                if count > 0
                    && trigger_open(view, def)
                    && let Some((d, i)) = nearest_outfield(view, def, keepers[def], ball_xy)
                    && !pressers[..count].iter().any(|&(_, k)| k == i)
                {
                    for k in (1..count).rev() {
                        pressers[k] = pressers[k - 1];
                    }
                    pressers[0] = (d, i);
                }
                // Stand options from the role menus and the rest-defence guard; pressing,
                // the line hold and cover then take their players from there.
                let shaped = shape::apply(view, phase, &anchors, &mut targets);
                hold_the_line(view, &mut targets, team, c);
                let sent_back = back_to_place(view, &mut targets);
                time_runs(view, &mut targets, team, c);
                let in_behind = forward_runs(view, &mut targets, team, c);
                let box_runners = box_runs(view, &mut targets, team, c);
                // Pressers run at the point where they can meet the carrier's run, and go for
                // the ball once they are close. Short of that, each leans his aim goal-side of
                // the carrier by his press stage (positioning): a good presser closes the way
                // to goal, a poor one comes from the open side.
                let run = players[c].vel;
                let engage = t.press_engage;
                let lean_m = t.contract.hooks().press_goal_side_m;
                let to_goal = toward(players[c].pos, teams[def].own_goal());
                // Outside engaging range a presser stands toward the carrier's strong side,
                // showing a one-footed carrier onto his weaker foot.
                let show = contract::foot::show_offset(
                    players[c].facing,
                    view.foot(c),
                    t.contract.foot().show_m,
                );
                // A player whose relentless pressing overrode his side's low press presses
                // this carrier too.
                let relentless = (0..n).filter(|&i| {
                    players[i].team == def
                        && view
                            .trait_act(i)
                            .is_some_and(|a| a.kind == TraitKind::PressesRelentlessly)
                });
                let force = teams[def].plan.force;
                let chosen = pressers[..count].iter().map(|&(_, i)| i);
                for i in chosen.chain(relentless) {
                    if i != usize::MAX {
                        let p = &players[i];
                        // Within engaging range the presser goes for the ball itself.
                        targets[i] = if (p.pos - ball_xy).length() < engage {
                            ball_xy
                        } else {
                            let horizon = INTERCEPT_HORIZON_S * p.derived.knobs.intercept;
                            // The press stage's goal-side lean and the strong-side show both
                            // move the point he meets the carrier at.
                            let meet = intercept(p.pos, p.max_speed(), ball_xy, run, horizon);
                            let lean = lean_m * view.skills(i).skill(Stage::PRESS_CHOOSE);
                            let aim = meet + to_goal * lean + show;
                            // Force the opponent moves the aim across, beside the show.
                            let aim = if force != 0.0 {
                                aim + DVec2::new(0.0, -players[c].pos.y.signum()) * FORCE_M * force
                            } else {
                                aim
                            };
                            view.pitch().clamp(aim, 0.5)
                        };
                    }
                }
                let cover = cover(view, &mut targets, def, c, &pressers[..count], keepers[def]);
                let cover_defender = match cover {
                    CoverTrace::Covered { defender, .. } => Some(defender),
                    _ => None,
                };
                let skip = |i: usize| {
                    sent_back & (1 << i) != 0
                        || Some(i) == cover_defender
                        || pressers[..count].iter().any(|&(_, k)| k == i)
                };
                step_with_line(view, &mut targets, def, skip);
                trap_step(view, &mut targets, def, skip);
                hold_lapses(view, &mut targets);
                let mut jobs = shape_jobs(view, phase, &targets, &shaped);
                // A run in behind, a run coming short and a run into the box are timed runs
                // (urgent), unless a lapse holds the runner.
                for (i, job) in jobs.iter_mut().enumerate() {
                    if (in_behind | box_runners) & (1 << i) != 0
                        && (players[i].pos - targets[i]).length() > 1.0
                    {
                        *job = Job::TimedRun;
                    }
                }
                // The side's chosen full-back run (hook `shape.fullback_run`) is a timed run:
                // a full-back who takes it goes at urgent pace. Staying back leaves every
                // full-back's option at its own pace.
                let chosen_run = match teams[team].plan.fullback_run {
                    r if r > 0.0 => Some(crate::data::tactics::StandOption::Overlap),
                    r if r < 0.0 => Some(crate::data::tactics::StandOption::Underlap),
                    _ => None,
                };
                if chosen_run.is_some() {
                    for (i, p) in players.iter().enumerate() {
                        let fullback = matches!(
                            teams[team].plan.slots[p.slot].position,
                            Some(crate::data::team::Position::LB | crate::data::team::Position::RB)
                        );
                        if p.team == team
                            && fullback
                            && shaped[team].option[p.slot] == chosen_run
                            && (p.pos - targets[i]).length() > 1.0
                        {
                            jobs[i] = Job::TimedRun;
                        }
                    }
                }
                jobs[c] = Job::Carry;
                for &(_, i) in &pressers[..count] {
                    if i != usize::MAX {
                        jobs[i] = Job::Press;
                    }
                }
                recovery_jobs(view, &mut jobs, def, keepers[def]);
                if let CoverTrace::Covered { defender, .. } = cover {
                    // The cover defender runs urgently only while the carrier is goal-side
                    // of him: between him and his own goal.
                    let ax = teams[def].attack_x;
                    if (players[c].pos.x - players[defender].pos.x) * ax < 0.0 {
                        jobs[defender] = Job::Recover;
                    }
                }
                lapse_jobs(view, &mut jobs);
                Targets {
                    targets,
                    jobs,
                    trace: TargetsTrace::Carrier {
                        def,
                        count,
                        reach,
                        pressers,
                        cover,
                    },
                }
            }
            None => {
                // The nearest player of each team chases; a goalkeeper chases a ball near its
                // goal.
                let predicted = view
                    .pitch()
                    .clamp(ball_xy + DVec2::new(ball.vel.x, ball.vel.y) * 0.3, 0.3);
                let mut nearest = [usize::MAX; 2];
                let mut nearest_dist = [f64::INFINITY; 2];
                // A loose ball or a pass in flight: each team keeps its phase until a player
                // of the other team controls the ball.
                let phase = view.possession_phase();
                let anchors = shape::anchors(view, phase);
                for i in 0..n {
                    let p = &players[i];
                    if !p.active() {
                        continue;
                    }
                    // Holding position acts while the ball is loose too; the chasers take
                    // their own targets below.
                    targets[i] = if i == keepers[p.team] {
                        anchors[i]
                    } else {
                        lagged(p, anchors[i])
                    };
                    let d = (p.pos - ball_xy).length();
                    // The kicker does not chase his own kick while it leaves him: his side's
                    // next nearest player goes for it.
                    let leaving = crate::possession::leaving_kicker(
                        view,
                        p,
                        ball_xy,
                        DVec2::new(ball.vel.x, ball.vel.y),
                    );
                    if i != keepers[p.team] && !leaving && d < nearest_dist[p.team] {
                        nearest_dist[p.team] = d;
                        nearest[p.team] = i;
                    }
                }
                let shaped = shape::apply(view, phase, &anchors, &mut targets);
                let sent_back = back_to_place(view, &mut targets);
                for team in 0..2 {
                    if phase != Some(team) {
                        step_with_line(view, &mut targets, team, |i| sent_back & (1 << i) != 0);
                    }
                }
                let mut chasing_keepers = [None; 2];
                for (team, &i) in nearest.iter().enumerate() {
                    if i != usize::MAX {
                        targets[i] = predicted;
                    }
                    let gk = keepers[team];
                    let chase = keeper_chase_m(view, gk);
                    if players[gk].active() && (players[gk].pos - ball_xy).length() < chase {
                        targets[gk] = predicted;
                        chasing_keepers[team] = Some(gk);
                    }
                }
                // A forward's timed run (hook `run.start`) goes on while his side's open-play
                // pass travels forward: the runner keeps his target, unless he chases the ball.
                // A pass back or across resets the attack, and the runners with it. At the
                // default (no forward runs) every run stops at a pass, as before.
                let mut runners = 0u32;
                if let Some(team) = view.pass_in_flight().filter(|&team| {
                    run_start(view, team) != 0.0 && ball.vel.x * teams[team].attack_x > 0.0
                }) {
                    for (i, p) in players.iter().enumerate() {
                        if p.team == team
                            && p.active()
                            && attack_line(teams[team].plan.slots[p.slot].position)
                            && p.pace.job == Job::TimedRun
                            && !nearest.contains(&i)
                            && (p.pos - p.target).length() > 1.0
                        {
                            targets[i] = p.target;
                            runners |= 1 << i;
                        }
                    }
                }
                hold_lapses(view, &mut targets);
                let mut jobs = shape_jobs(view, phase, &targets, &shaped);
                for (i, job) in jobs.iter_mut().enumerate() {
                    if runners & (1 << i) != 0 {
                        *job = Job::TimedRun;
                    }
                }
                // The chaser who reaches the ball first, at his top speed, goes for a loose
                // ball; the other team's chaser chases.
                let time =
                    |i: usize| (players[i].pos - predicted).length() / players[i].max_speed();
                let first = match nearest {
                    [a, b] if a != usize::MAX && b != usize::MAX => {
                        Some(if time(b) < time(a) { b } else { a })
                    }
                    [a, b] => (a != usize::MAX)
                        .then_some(a)
                        .or((b != usize::MAX).then_some(b)),
                };
                for &i in &nearest {
                    if i != usize::MAX {
                        jobs[i] = if Some(i) == first {
                            Job::LooseBall
                        } else {
                            Job::Chase
                        };
                    }
                }
                for gk in chasing_keepers.into_iter().flatten() {
                    jobs[gk] = Job::LooseBall;
                }
                // A pass in flight or a loose ball: the team out of possession (both, with no
                // phase) recovers as it does against a carrier. A chaser keeps his chase.
                for (def, &keeper) in keepers.iter().enumerate() {
                    if phase != Some(def) {
                        recovery_jobs(view, &mut jobs, def, keeper);
                    }
                }
                lapse_jobs(view, &mut jobs);
                Targets {
                    targets,
                    jobs,
                    trace: TargetsTrace::Loose {
                        nearest,
                        nearest_dist,
                        keepers: chasing_keepers,
                    },
                }
            }
        }
    }

    /// Scores every option the carrier has (named mechanism: scored-options decision layer).
    /// Each score is a weighted sum of the option's features with weights from the tuning
    /// file, plus the offsets of the team's plan (mentality, instructions, the carrier's role
    /// and duty); the noise that shrinks as the carrier's choose stage (decisions) rises, and
    /// with an opponent close his pressure stage (composure), is the last term, which
    /// [`DecisionModule::scored`] adds.
    fn options(&self, view: &MatchView<'_>, c: usize) -> OptionDraft {
        let t = view.tuning();
        let w = &t.decision;
        let players = view.players();
        let carrier = &players[c];
        let stages = view.skills(c);
        let gates = carrier.derived.gates;
        let team = carrier.team;
        let side = &view.teams()[team];
        let plan = side.plan;
        let role = plan.slots[carrier.slot];
        let goal = side.target_goal();
        let attack = DVec2::new(side.attack_x, 0.0);
        let goal_dist = (goal - carrier.pos).length();
        let keeper = c == view.keeper(team);
        let their_keeper = view.keeper(1 - team);
        // An option's skill term: `(2 · share − 1) · skill`, 0 at rating 10.
        let skill = |s: Stage| stages.skill(s) * w.skill;
        let flair_risk = t.contract.hooks().flair_risk;
        // Mood: a carrier in good mood shoots and plays forward more and holds less; in low
        // mood the reverse. 0 at a neutral mood and with no person block.
        let mood_risk = t.contract.person().mood.risk * view.mood(c) / crate::contract::mood::FULL;
        // Traits of play: each trait he carries adds its weight to its choice; 0 without a
        // traits block or a trait.
        let traits = view.traits(c);
        let tw = t.contract.traits();
        let has_traits = !traits.is_empty();
        let tr = |k: TraitKind| {
            if has_traits {
                traits.weight(k, tw)
            } else {
                0.0
            }
        };

        // Risk appetite (hook `option.loss_danger`): a pass, a dribble or a hold loses
        // `loss_weight x failure chance x danger`. Losing the ball at `at` hands the opponent
        // the threat of that place near the side's own goal and gives up the side's own threat
        // near the goal it attacks; each grows with the square of the nearness to its goal, so
        // the danger is 1 at either goal line and 0.5 at halfway. A shot has no loss term: a
        // missed shot gives the ball back at the opponent's own goal line, where it threatens
        // nothing, and the shot spends the side's threat instead of losing it. (Counted as a
        // loss at the shooter's feet, it held back every shot but the closest, so the shots
        // left were twice as good and goals rose by half.)
        // On winning the ball: inside the side's regain window a counter plays the ball
        // forward and holds it less; 0 outside it and at the default.
        let counter = counter_weight(view, team);
        let half = view.pitch().half_length();
        let danger = |at: DVec2| {
            let s = ((at.x * side.attack_x + half) / (2.0 * half)).clamp(0.0, 1.0);
            (1.0 - s) * (1.0 - s) + s * s
        };
        let loss = |fail: f64, at: DVec2| plan.loss_weight * fail.clamp(0.0, 1.0) * danger(at);

        // Pressure and space around the carrier.
        let mut nearest_opp = f64::INFINITY;
        let mut nearest_opp_pos = carrier.pos;
        let mut space_ahead: f64 = 10.0;
        for p in players {
            if p.team == team || !p.active() {
                continue;
            }
            let d = p.pos - carrier.pos;
            let dist = d.length();
            if dist < nearest_opp {
                nearest_opp = dist;
                nearest_opp_pos = p.pos;
            }
            if dist < 10.0 && dist > 1e-6 && d.dot(attack) / dist > 0.5 {
                space_ahead = space_ahead.min(dist);
            }
        }
        // The pressed carrier's choice reads the pressure model ([`crate::pressure`]): the
        // intensity he feels, his read of the press included. `pressed` is whether it passes
        // the one threshold (the carry window, the skill gate and the pressed traits read it);
        // `near` grades the pressed terms of the scores (the judgement, the take-on, the loss
        // terms, the clear in his own third) around the threshold ([`crate::pressure::grade`]).
        // The lone forward's hold-up reads the intensity the observer sees (`seen_pressed`):
        // a press he feels more than it is, from his blind side, does not make him hold the
        // ball up.
        let felt = crate::pressure::felt(
            players,
            carrier,
            crate::pressure::read(stages),
            side.attack_x,
        );
        let seen_pressed = crate::pressure::pressed(players, carrier, side.attack_x);
        let pressed = felt > crate::pressure::THRESHOLD;
        let near = crate::pressure::grade(felt);
        let calm = stages.share(Stage::PASS_CHOOSE);
        let judgement =
            calm + near * (stages.pressed_share(Stage::PASS_CHOOSE, Stage::PASS_PRESSURE) - calm);
        // Each option that keeps the ball (a pass, a dribble, a hold) is scored by his own
        // chance to carry it out under the pressure he feels: it loses
        // `decision.pressure_choice` x his penalty for it (intensity x the option's difficulty
        // x (1 - his resistance for it)) x the danger of losing the ball where it would be
        // lost, the risk appetite's danger shape. A clearance keeps no ball and pays nothing,
        // so as the press rises a player who resists it less (composure, decisions and the
        // action's own attributes) plays long or clears more, and one who resists it more
        // keeps the ball.
        let keep_cost = |difficulty: f64, resistance: f64, at: DVec2| {
            w.pressure_choice * crate::pressure::penalty(felt, difficulty, resistance) * danger(at)
        };
        // Creative freedom (hook `option.noise`) scales the noise: a side given more freedom
        // strays further from the top-scored option.
        let noise = w.noise * (1.5 - judgement) * plan.freedom;
        // A lone carrier: an outfield carrier with no active outfield team-mate ahead of him.
        let depth = carrier.pos.x * attack.x;
        // The passer's read of the offside line (named mechanism): a team-mate in the
        // opponents' half and beyond the ball is read as offside with the chance
        // [`offside_read`] gives for how far he stands beyond the second-last defender and
        // how well the passer reads it (his vision and his decisions under the press he
        // feels); a pass read as offside loses [`OFFSIDE_READ_COST`]. A goal kick, the one
        // restart this choice decides, creates no offside, so its taker reads nothing.
        let offside_line = offside::second_last_depth(team, players, side.attack_x);
        let reads_offside = offside_line.is_finite() && view.restart_taker() != Some(c);
        let line_read = 0.5 * (stages.share(Stage::PASS_SEE) + judgement);
        // His read of the press each receiver will be under ([`crate::pressure::passer_read`]).
        let press_read =
            crate::pressure::passer_read(stages.share(Stage::PASS_SEE), judgement, felt);
        // A pass from his own penalty area (the keeper's goal kick and every open-play pass
        // from there) reads a screened team-mate, one with an opponent within [`SCREEN_M`] of
        // him, the way the restart pick does: it loses [`SCREENED_COST`], so he looks past
        // him.
        let from_own_area = view.pitch().in_penalty_area(carrier.pos, -side.attack_x);
        let lone = !keeper
            && !players.iter().enumerate().any(|(j, p)| {
                p.team == team
                    && j != c
                    && j != view.keeper(team)
                    && p.active()
                    && p.pos.x * attack.x > depth
            });

        // Shot. The final third's reach (hook `shot.range`) lets a side look at shots from
        // farther out.
        let shot = if !keeper && goal_dist < t.shot_range + plan.shot_reach_m {
            let lane = players
                .iter()
                .filter(|p| p.team != team && p.id != their_keeper && p.active())
                .map(|p| segment_distance(p.pos, carrier.pos, goal))
                .fold(f64::INFINITY, f64::min);
            let under_pressure = if nearest_opp < 2.0 && lane > 1.0 {
                w.shot_under_pressure
            } else {
                0.0
            };
            let choose = if view.pitch().in_penalty_area(carrier.pos, side.attack_x) {
                Stage::SHOT_CHOOSE
            } else {
                Stage::LONG_SHOT_CHOOSE
            };
            // Shooting from distance: outside the area, more the farther out; the side's
            // long-shot weight (hook `shot.outside_box`) acts on every shot from there.
            let distance = if choose == Stage::LONG_SHOT_CHOOSE {
                tr(TraitKind::ShootsFromDistance) * goal_dist / t.shot_range + plan.long_shot
            } else {
                0.0
            };
            Some(
                w.shot_base + w.shot_lane * ((lane.min(5.0) - 2.5) / 2.5)
                    - w.shot_distance * goal_dist / t.shot_range
                    + under_pressure
                    + skill(choose)
                    + role.shoot
                    + mood_risk
                    + distance,
            )
        } else {
            None
        };

        // Passes.
        let mut passes = [(0usize, 0.0f64); MATES];
        let mut pass_pick = [0.0f64; MATES];
        let mut pass_count = 0;
        for j in 0..players.len() {
            let mate = &players[j];
            if mate.team != team || j == c || !mate.active() {
                continue;
            }
            let d = (mate.pos - carrier.pos).length();
            if !(4.0..=45.0).contains(&d) {
                continue;
            }
            let kind = PassKind::of(
                carrier.pos,
                mate.pos,
                d,
                side.attack_x,
                keeper,
                view.pitch(),
            );
            // The skill gate: a carrier whose technique is under the chip's threshold does
            // not try one.
            if kind == PassKind::Chip && !gates.chip_try {
                continue;
            }
            let mut lane: f64 = 6.0;
            let mut receiver_space: f64 = 8.0;
            for opp in players {
                if opp.team == team || !opp.active() {
                    continue;
                }
                lane = lane.min(segment_distance(opp.pos, carrier.pos, mate.pos));
                receiver_space = receiver_space.min((opp.pos - mate.pos).length());
            }
            if lane < w.min_lane {
                continue;
            }
            let progress = ((mate.pos - carrier.pos).dot(attack) / 40.0).clamp(-1.0, 1.0);
            let forward = progress.max(0.0);
            // Flair: a carrier with it rates a forward pass higher, one without it lower.
            let flair = flair_risk * carrier.derived.occasion.flair * forward;
            let layoff = if lone && progress <= 0.0 {
                w.lone_layoff
            } else {
                0.0
            };
            // Patience weighs keeping the ball in the side's circulation against playing
            // forward when the carrier picks his pass: a short pass that does not go forward
            // gains it, a forward pass loses it, so a negative patience plays forward early.
            // It moves which pass he picks, never whether he passes, holds or dribbles.
            let gain = (mate.pos - carrier.pos).dot(attack);
            let recycle = if gain >= FORWARD_M {
                -plan.patience
            } else if d <= RECYCLE_M {
                plan.patience
            } else {
                0.0
            };
            // The side's in-possession plan weighs the pass: build-up a long pass out of its
            // own third (hook `pass.build_up`, the keeper's included), the attacking channel
            // a pass into a channel of the middle or final third (hooks `pass.channel.*`),
            // crossing and early crossing a cross (hooks `pass.cross`, `pass.cross_early`).
            // Each term is 0 at its default.
            let build_up = if d >= LONG_BALL_M && carrier.pos.x * attack.x < -half / 3.0 {
                plan.build_up * (d / 45.0)
            } else {
                0.0
            };
            // Width's second input (hook `shape.width`) weighs a pass to a wide team-mate of
            // the middle or final third the same way.
            let channel_bias = if mate.pos.x * attack.x >= -half / 3.0 {
                match channel(mate.pos.y, side.attack_x) {
                    0 => plan.channel_left + plan.wide_target,
                    2 => plan.channel_right + plan.wide_target,
                    _ => plan.channel_centre,
                }
            } else {
                0.0
            };
            let cross = if kind == PassKind::Cross {
                let early = half - carrier.pos.x * side.attack_x > EARLY_CROSS_M;
                plan.crossing + if early { plan.early_cross } else { 0.0 }
            } else {
                0.0
            };
            // The receiver's press on arrival (named mechanism): the pressure he will be under
            // once the ball is his, each opponent's reaction and run against the ball's flight
            // and his control time, as the passer reads it ([`crate::pressure::arrival_read`]:
            // a weak or pressed reader pictures a moving opponent late). It costs the pass the
            // receiver's own chance to keep the ball under that pressure (his reception's
            // difficulty and resistance, as a pressed touch reads them) at the danger of
            // losing it there, and it is the receiver's part of the pass's failure: a team-mate
            // a presser reaches before the ball is controlled is as good as marked, however far
            // the presser stood when the pass was chosen.
            let arrives = crate::pressure::arrival_read(
                players,
                carrier,
                press_read,
                mate,
                mate.pos,
                pass_time(d, t),
                side.attack_x,
            );
            let received_cost = if arrives > 0.0 {
                let mate_stages = view.skills(j);
                let facing_back = (1.0 - mate.facing.x * side.attack_x) / 2.0;
                let speed_share =
                    t.pass_arrival_speed / (t.control_speed * mate.derived.knobs.receive);
                w.pressure_choice
                    * crate::pressure::penalty(
                        arrives,
                        crate::pressure::reception_difficulty(
                            speed_share,
                            facing_back,
                            crate::pressure::read(mate_stages),
                        ),
                        crate::pressure::reception_resistance(mate_stages),
                    )
                    * danger(mate.pos)
            } else {
                0.0
            };
            // The pass fails as its lane closes and as its receiver will be pressed when the
            // ball reaches him; the ball is lost about halfway.
            let fail = 0.5 * ((1.0 - lane / 6.0) + (arrives / crate::pressure::THRESHOLD).min(1.0));
            // His own chance to play it under pressure: an outfield pass kind with a pressure
            // stage pays its keep cost.
            let pressed_cost = match kind.execute() {
                (execute, Some(ps)) if felt > 0.0 => keep_cost(
                    crate::pressure::pass_difficulty(d, kind != PassKind::Pass),
                    crate::pressure::pass_resistance(stages, execute, ps),
                    (carrier.pos + mate.pos) * 0.5,
                ),
                _ => 0.0,
            };
            let mate_depth = mate.pos.x * attack.x;
            let offside_cost = if reads_offside && mate_depth > depth.max(0.0) {
                OFFSIDE_READ_COST * offside_read(mate_depth - offside_line, line_read)
            } else {
                0.0
            };
            let screened = if from_own_area && receiver_space < SCREEN_M {
                SCREENED_COST
            } else {
                0.0
            };
            let score = (w.progress + role.progress) * progress
                + skill(Stage::PASS_SEE) * forward
                + w.teamwork_bonus * stages.skill(kind.choose())
                + w.lane * (lane / 6.0)
                + w.space * (receiver_space / 8.0)
                - w.distance * (d / 45.0)
                + plan.directness * (gain.max(0.0) / 45.0)
                + counter * (gain.max(0.0) / 45.0)
                + recycle
                - loss(fail, (carrier.pos + mate.pos) * 0.5)
                + layoff
                + plan.slots[mate.slot].long_ball_target * (d / 45.0)
                + flair
                + mood_risk * forward
                + build_up
                + channel_bias
                + cross
                - pressed_cost
                - received_cost
                - offside_cost
                - screened;
            let score = if has_traits {
                let switch = d > SWITCH_M && (mate.pos.y - carrier.pos.y).abs() >= SWITCH_M;
                let early = kind == PassKind::Cross
                    && view.pitch().half_length() - carrier.pos.x * side.attack_x > EARLY_CROSS_M;
                let back = view.last_passer(c) == Some(j);
                score
                    + tr(TraitKind::TriesKillerBalls) * forward * (1.0 - lane / 6.0)
                    + tr(TraitKind::PlaysShortSimplePasses) * (1.0 - d / 45.0 - forward)
                    + if kind == PassKind::Chip {
                        tr(TraitKind::TriesLobsAndChips)
                    } else {
                        0.0
                    }
                    + if switch {
                        tr(TraitKind::SwitchesPlay)
                    } else {
                        0.0
                    }
                    + if early {
                        tr(TraitKind::CrossesEarly)
                    } else {
                        0.0
                    }
                    + if back {
                        tr(TraitKind::PlaysOneTwos)
                    } else {
                        0.0
                    }
                    + match kind {
                        PassKind::Kick => tr(TraitKind::KicksLong),
                        PassKind::Throw => tr(TraitKind::ThrowsShort),
                        _ => 0.0,
                    }
            } else {
                score
            };
            passes[pass_count] = (j, score);
            pass_pick[pass_count] = recycle;
            pass_count += 1;
        }

        let held = view.tick().saturating_sub(view.control_since());
        let held_s = f64::from(held) * t.dt;
        // The carry window: an unpressed carrier who has just gained the ball keeps it
        // rather than releasing it at once. A shot is never charged.
        let carry = if !pressed && held_s < w.carry_s {
            // A player who releases the ball early pays less for leaving the window.
            if has_traits {
                (w.carry_cost - tr(TraitKind::ReleasesBallEarly)).max(0.0)
            } else {
                w.carry_cost
            }
        } else {
            0.0
        };
        let dribble_hold = if keeper {
            None
        } else {
            let (lone_dribble, lone_hold) = if lone && seen_pressed {
                (w.lone_dribble, w.lone_hold)
            } else {
                (0.0, 0.0)
            };
            let dribble = w.dribble_base + w.dribble_space * (space_ahead / 10.0)
                - w.takeon_under_pressure * near
                + if held < 10 { w.first_touch } else { 0.0 }
                + skill(Stage::DRIBBLE_CHOOSE)
                + role.dribble
                + lone_dribble
                - loss(0.5 * (near + (1.0 - space_ahead / 10.0)), carrier.pos)
                // The side's dribbling weight (hook `dribble.weight`), 0 at its default.
                + plan.dribble
                - keep_cost(
                    crate::pressure::DRIBBLE_DIFFICULTY,
                    crate::pressure::choice_resistance(
                        crate::pressure::dribble_resistance(stages),
                        stages.share(Stage::DRIBBLE_CHOOSE),
                    ),
                    carrier.pos,
                );
            let hold = w.hold - w.hold_per_s * held_s + lone_hold
                - mood_risk
                - counter
                - loss(0.5 * near, carrier.pos)
                - keep_cost(
                    crate::pressure::HOLD_DIFFICULTY,
                    crate::pressure::choice_resistance(
                        crate::pressure::hold_resistance(stages),
                        stages.share(Stage::PASS_CHOOSE),
                    ),
                    carrier.pos,
                );
            let (dribble, hold, tricks) = if has_traits {
                let pressed_only = |k: TraitKind| if pressed { tr(k) } else { 0.0 };
                // Holding the ball up, like the lone forward's hold-up, reads the observer's
                // intensity.
                let holds_up = if seen_pressed {
                    tr(TraitKind::HoldsUpBall)
                } else {
                    0.0
                };
                let dwell = tr(TraitKind::DwellsOnBall);
                // A player who dwells on the ball loses less of the hold's score with time.
                let slower = w.hold_per_s * held_s * dwell / (1.0 + dwell);
                (
                    dribble + tr(TraitKind::RunsWithBallOften) - tr(TraitKind::RarelyDribbles)
                        + pressed_only(TraitKind::TriesTricks),
                    hold + dwell + slower - tr(TraitKind::ReleasesBallEarly) + holds_up,
                    gates.take_on_short
                        <= traits.share(TraitKind::TriesTricks, tw) * tw.weights.tries_tricks_gate,
                )
            } else {
                (dribble, hold, false)
            };
            // The skill gate: a pressed carrier whose technique is under the take-on's
            // threshold does not try to dribble past; he may still hold. A player who tries
            // tricks needs less technique.
            let dribble = (!pressed || gates.take_on_try || tricks).then_some(dribble);
            Some((dribble, hold))
        };
        let own_third = carrier.pos.x * attack.x < -view.pitch().half_length() / 3.0;
        let clear = if keeper {
            w.keeper_clear
        } else {
            w.clear
                + if own_third {
                    w.clear_pressure * near
                } else {
                    0.0
                }
        } - carry;
        OptionDraft {
            noise,
            shot,
            passes,
            pass_count,
            pass_pick,
            dribble_hold,
            clear,
            carry,
            nearest_opp_pos,
        }
    }

    fn scored(&self, draft: &OptionDraft, draws: &OptionDraws) -> Scored {
        let noise = draft.noise;
        let jitter = |u: f64| map_range(-noise, noise, u);
        let shot = draft.shot.map(|s| s + jitter(draws.shot));
        let mut pass: Option<(f64, usize, f64)> = None;
        let mut candidates = [(0usize, 0.0f64); MATES];
        let drafted = draft.passes[..draft.pass_count]
            .iter()
            .zip(&draws.passes)
            .zip(&draft.pass_pick);
        for (candidate, ((&(j, s), &u), &pick)) in candidates.iter_mut().zip(drafted) {
            let score = s + jitter(u);
            *candidate = (j, score);
            if pass.is_none_or(|(best, ..)| score > best) {
                pass = Some((score, j, pick));
            }
        }
        let pass = pass.map(|(s, j, pick)| (s - pick - draft.carry, j));
        let (dribble, hold) = match draft.dribble_hold {
            Some((dribble, hold)) => (
                dribble.map(|d| d + jitter(draws.dribble_hold.0)),
                Some(hold + jitter(draws.dribble_hold.1)),
            ),
            None => (None, None),
        };
        let clear = draft.clear + jitter(draws.clear);
        Scored {
            options: Options {
                shot,
                pass,
                dribble,
                clear,
                hold,
                nearest_opp_pos: draft.nearest_opp_pos,
            },
            candidates,
            count: draft.pass_count,
        }
    }

    fn choose(
        &self,
        _: &MatchView<'_>,
        _: usize,
        options: &Options,
        offsets: Option<OptionOffsets>,
    ) -> (Options, Choice) {
        let mut o = *options;
        if let Some(off) = offsets {
            o.shot = o.shot.map(|s| s + off.shoot);
            o.pass = o.pass.map(|(s, j)| (s + off.pass, j));
            o.dribble = o.dribble.map(|s| s + off.dribble);
            o.clear += off.clear;
            o.hold = o.hold.map(|s| s + off.hold);
        }
        let mut choice = (o.clear, Choice::Clear);
        for (score, option) in [
            (o.shot, Choice::Shot),
            (o.pass.map(|(s, _)| s), Choice::Pass),
            (o.dribble, Choice::Dribble),
            (o.hold, Choice::Hold),
        ] {
            if let Some(score) = score
                && score > choice.0
            {
                choice = (score, option);
            }
        }
        (o, choice.1)
    }

    fn plan(
        &self,
        view: &MatchView<'_>,
        c: usize,
        options: &Options,
        choice: Choice,
    ) -> CarrierPlan {
        let t = view.tuning();
        let carrier = view.player(c);
        let team = carrier.team;
        let side = &view.teams()[team];
        let goal = side.target_goal();
        match choice {
            Choice::Shot => CarrierPlan::Shot {
                goal,
                keeper: view.keeper(1 - team),
            },
            Choice::Pass => CarrierPlan::Pass {
                j: options.pass.map_or(c, |(_, j)| j),
            },
            Choice::Clear => {
                // Clear long toward the far half; near his own goal line a clearance may go
                // wide toward that line instead.
                let own_goal_x = -side.attack_x;
                let c = &t.clearances;
                let wide = c.wide_chance > 0.0
                    && carrier.pos.x * own_goal_x.signum()
                        >= view.pitch().half_length() - c.wide_depth;
                CarrierPlan::Clear {
                    wide_chance: wide.then_some(c.wide_chance),
                }
            }
            Choice::Hold => CarrierPlan::Move {
                target: carrier.pos,
            },
            Choice::Dribble => {
                // Dribble toward goal, drifting away from the nearest opponent.
                let (dir_goal, perp, side) = dribble_line(view, c, options.nearest_opp_pos);
                // From a wide channel, cutting inside turns the drift toward the centre and
                // hugging the touchline toward the line; an override turns it all the way.
                let traits = view.traits(c);
                let side = if traits.is_empty() || carrier.pos.y.abs() <= contract::CROSS_WIDE_M {
                    side
                } else {
                    let tw = t.contract.traits();
                    let inside = inside_side(carrier.pos, perp);
                    let (cut, hug) = match view.trait_act(c).map(|a| a.kind) {
                        Some(TraitKind::CutsInside) => (1.0, 0.0),
                        Some(TraitKind::HugsTouchline) => (0.0, 1.0),
                        _ => (
                            traits.weight(TraitKind::CutsInside, tw),
                            traits.weight(TraitKind::HugsTouchline, tw),
                        ),
                    };
                    side + (inside - side) * cut + (-inside - side) * hug
                };
                let target = carrier.pos + dir_goal * 8.0 + perp * (4.0 * side);
                CarrierPlan::Move {
                    target: view.pitch().clamp(target, 1.0),
                }
            }
        }
    }

    /// A pass's aim noise falls as its execution share rises: the execute stage of its kind
    /// (passing, technique for a chip, crossing, a keeper's throwing or kicking), with the
    /// pressure stage when an opponent is close, and a chip the body cannot pull off loses
    /// its gate penalty. The pressure model ([`crate::pressure`]): an outfield passer's aim
    /// noise grows by `decision.pass_under_pressure` times his penalty, the pressure intensity
    /// he feels (his read of the press) x the pass's difficulty (length, loft) x (1 - his pass
    /// resistance: passing, composure, vision and decisions).
    fn pass_kick(&self, view: &MatchView<'_>, c: usize, j: usize, aim: f64, weight: f64) -> Kick {
        let t = view.tuning();
        let carrier = view.player(c);
        let mate_pos = view.player(j).pos;
        let d = (mate_pos - carrier.pos).length();
        let attack_x = view.teams()[carrier.team].attack_x;
        let keeper = c == view.keeper(carrier.team);
        let kind = PassKind::of(carrier.pos, mate_pos, d, attack_x, keeper, view.pitch());
        let (execute, pressure) = kind.execute();
        let penalty = if kind == PassKind::Chip {
            carrier.derived.gates.chip_penalty
        } else {
            0.0
        };
        let under = pressure.is_some() && pressed(view, c);
        let line = toward(carrier.pos, mate_pos);
        // A keeper's distribution pays no weaker-foot cost.
        let foot = if keeper {
            0.0
        } else {
            foot_cost(view, c, line, d, false)
        };
        let skill = execution(view, c, execute, pressure, under, penalty, foot);
        let noise = t.aim_noise * (1.5 - skill);
        // The pressure model: an outfield passer's penalty widens his aim noise and spoils the
        // weight of his pass.
        let penalty = match pressure {
            Some(ps) => {
                let stages = view.skills(c);
                let read = crate::pressure::read(stages);
                let intensity = crate::pressure::felt(
                    view.players(),
                    carrier,
                    read,
                    view.attack_x(carrier.team),
                );
                let resistance = crate::pressure::pass_resistance(stages, execute, ps);
                let difficulty = crate::pressure::pass_difficulty(d, kind != PassKind::Pass);
                crate::pressure::penalty(intensity, difficulty, resistance)
            }
            None => 0.0,
        };
        let noise = noise * (1.0 + t.decision.pass_under_pressure * penalty);
        // Too hard or too soft by up to the weight scale times the penalty.
        let weight = 1.0 + t.decision.pass_weight_under_pressure * penalty * (2.0 * weight - 1.0);
        let dir = rotate(line, map_range(-noise, noise, aim));
        let lofted = if d > 25.0 { 3.0 + d * 0.08 } else { 0.0 };
        // The cross type (hook `cross.trajectory`): a driven cross stays on the ground, a
        // floated one is lofted at every length, and the default keeps the length rule.
        let cross_loft = view.teams()[carrier.team].plan.cross_loft;
        let loft = if kind != PassKind::Cross || cross_loft == 0.0 {
            lofted
        } else if cross_loft < 0.0 {
            0.0
        } else {
            3.0 + d * 0.08
        };
        let speed = (kick_speed(d, loft, t) * weight.max(0.0)).min(t.pass_max_speed);
        Kick::Pass { dir, speed, loft }
    }

    fn clear_kick(&self, view: &MatchView<'_>, c: usize, wide: bool, aim: f64) -> Kick {
        let t = view.tuning();
        let carrier = view.player(c);
        let attack_x = view.teams()[carrier.team].attack_x;
        let own_goal_x = -attack_x;
        let (line, spread) = if wide {
            (wide_of_goal(carrier.pos, own_goal_x), WIDE_SPREAD)
        } else {
            (DVec2::new(attack_x, 0.0), t.clearances.aim_spread)
        };
        // A keeper's clearance is a kick: his kicking stage narrows its spread.
        let spread = if c == view.keeper(carrier.team) {
            spread * (1.5 - view.skills(c).share(Stage::KEEPER_KICK_EXECUTE))
        } else {
            spread
        };
        let dir = rotate(line, map_range(-spread, spread, aim));
        Kick::Clear {
            dir,
            speed: kick_speed(CLEARANCE_DISTANCE, CLEARANCE_LOFT, t),
            loft: CLEARANCE_LOFT,
        }
    }

    fn shot_draws_side(&self, view: &MatchView<'_>, keeper: usize) -> bool {
        !view.player(keeper).active()
    }

    /// Player `c` shoots at the goal centred on `goal`, which `keeper` defends. The shot aims
    /// for the side of the goal away from the goalkeeper; a better finisher places it nearer
    /// the post and strikes it truer. A placed kick from the penalty mark has a smaller aim
    /// noise (`shots.penalty_spread`). The execution share is the shot execute stage
    /// (finishing) from inside the penalty area and the long-shot stage from outside it, each
    /// with its pressure stage when an opponent is close; the penalty stage for a penalty,
    /// with its pressure stage; and the free-kick stage, with nobody close, for a direct free
    /// kick.
    fn shot_kick(
        &self,
        view: &MatchView<'_>,
        c: usize,
        goal: DVec2,
        keeper: usize,
        kind: ShotKind,
        draws: &ShotDraws,
    ) -> Kick {
        let t = view.tuning();
        let carrier = view.player(c);
        let attack_x = view.teams()[carrier.team].attack_x;
        let (execute, pressure) = match kind {
            ShotKind::Penalty => (Stage::PENALTY_EXECUTE, Some(Stage::PENALTY_PRESSURE)),
            ShotKind::FreeKick => (Stage::FREE_KICK_EXECUTE, None),
            ShotKind::Open if view.pitch().in_penalty_area(carrier.pos, attack_x) => {
                (Stage::SHOT_EXECUTE, Some(Stage::SHOT_PRESSURE))
            }
            ShotKind::Open => (Stage::LONG_SHOT_EXECUTE, Some(Stage::LONG_SHOT_PRESSURE)),
        };
        // Pressure handling narrows a penalty's spread, a shoot-out kick's included.
        let spread_scale = match kind {
            ShotKind::Penalty => {
                t.shots.penalty_spread
                    * (1.0
                        - t.contract.person().occasions.penalty_pressure
                            * view.personality(c).pressure_handling)
            }
            _ => 1.0,
        };
        let under = pressure.is_some() && pressed(view, c);
        // The weaker foot: the side and range of the strike at the goal, before its aim and
        // noise. A penalty and a direct free kick are placed dead balls, so the taker always
        // has time to strike with his strong foot.
        let dead_ball = matches!(kind, ShotKind::Penalty | ShotKind::FreeKick);
        let foot = foot_cost(
            view,
            c,
            toward(carrier.pos, goal),
            (goal - carrier.pos).length(),
            dead_ball,
        );
        let finishing = execution(view, c, execute, pressure, under, 0.0, foot).clamp(0.0, 1.0);
        let keeper = view.player(keeper);
        let side = if !keeper.active() {
            if draws.side == Some(true) { 1.0 } else { -1.0 }
        } else if keeper.pos.y > goal.y {
            -1.0
        } else {
            1.0
        };
        // A player who places his shots aims nearer the post and strikes softer; one who
        // powers them strikes harder and wider.
        let traits = view.traits(c);
        let (place, power) = if traits.is_empty() {
            (0.0, 0.0)
        } else {
            let tw = t.contract.traits();
            (
                traits.weight(TraitKind::PlacesShots, tw),
                traits.weight(TraitKind::PowersShots, tw),
            )
        };
        let reach = pitch::GOAL_WIDTH / 2.0 * (0.4 + 0.5 * finishing);
        let reach = if place > 0.0 {
            (reach * (1.0 + place)).min(pitch::GOAL_WIDTH / 2.0 * 0.95)
        } else {
            reach
        };
        let aim = DVec2::new(goal.x, side * map_range(0.5 * reach, reach, draws.aim));
        let spread = t.shot_noise * (1.5 - finishing) * spread_scale * (1.0 + power);
        let dir = rotate(
            toward(carrier.pos, aim),
            map_range(-spread, spread, draws.spread),
        );
        Kick::Shot {
            dir,
            speed: t.shot_speed * (1.0 - 0.5 * place) * (1.0 + power),
            loft: map_range(0.0, t.shots.loft_max, draws.loft),
        }
    }

    /// The kick that takes a restart: a short lofted throw to the nearest team-mate, a cross
    /// toward the penalty mark from a corner or a direct free kick, or a pass to an open
    /// team-mate. With no team-mate in range, the ball goes forward along the pitch. A
    /// throw-in looks for team-mates as far as his long-throw factor carries `THROW_RANGE`; a
    /// corner's or free kick's delivery turns from its aim by the aim draw, up to
    /// `hooks.restart_noise` for an average taker, less for a better one.
    fn restart_pass(
        &self,
        view: &MatchView<'_>,
        taker: usize,
        kind: StoppageKind,
        direct: bool,
        aim: f64,
    ) -> RestartPass {
        let t = view.tuning();
        let c = &t.contract;
        let players = view.players();
        let me = players[taker];
        let skills = view.skills(taker);
        let attack = DVec2::new(view.teams()[me.team].attack_x, 0.0);
        let delivered = kind == StoppageKind::Corner || (kind == StoppageKind::FreeKick && direct);
        let cross_at = delivered.then(|| view.pitch().penalty_spot(attack.x));
        // Build-up (hook `pass.build_up`) weighs a goal kick by its length and lets it reach
        // farther toward going long; 0 at its default.
        let build_up = if kind == StoppageKind::GoalKick {
            view.teams()[me.team].plan.build_up
        } else {
            0.0
        };
        let range = if kind == StoppageKind::ThrowIn {
            let long = contract::factor(
                c.actions.of(contract::ActionKind::LongThrow).spread(),
                skills.share(Stage::LONG_THROW_EXECUTE),
            );
            3.0..=(THROW_RANGE * long)
        } else {
            4.0..=(45.0 + build_up.max(0.0) * GOAL_KICK_REACH_M)
        };
        let mut candidates = [(0usize, 0.0f64); MATES];
        let mut count = 0;
        let mut best: Option<(f64, DVec2)> = None;
        // The taker's traits move his pick, in metres of the score: a killer ball's gain, a
        // short pass's length, and a keeper's long kick or short throw at a goal kick.
        let traits = view.traits(taker);
        let keeper = taker == view.keeper(me.team);
        for (j, mate) in players.iter().enumerate() {
            if j == taker || mate.team != me.team || !mate.active() {
                continue;
            }
            let d = (mate.pos - me.pos).length();
            if !range.contains(&d) {
                continue;
            }
            let score = match cross_at {
                Some(mark) => -(mate.pos - mark).length(),
                None => {
                    -d + 0.2 * (mate.pos - me.pos).dot(attack) + build_up * BUILD_UP_M * d / 45.0
                }
            };
            // At a goal kick a screened team-mate, an opponent within [`SCREEN_M`] of him,
            // scores less: the keeper looks past him.
            let score = if kind == StoppageKind::GoalKick
                && players.iter().any(|q| {
                    q.team != me.team && q.active() && (q.pos - mate.pos).length() < SCREEN_M
                }) {
                score - SCREENED_M
            } else {
                score
            };
            let score = if traits.is_empty() || cross_at.is_some() {
                score
            } else {
                let tw = c.traits();
                let tr = |k: TraitKind| traits.weight(k, tw);
                let gain = (mate.pos - me.pos).dot(attack).max(0.0);
                let distribution = match (keeper, d > contract::THROW_M) {
                    (true, true) => tr(TraitKind::KicksLong) * RESTART_TRAIT_M,
                    (true, false) => tr(TraitKind::ThrowsShort) * RESTART_TRAIT_M,
                    _ => 0.0,
                };
                score + tr(TraitKind::TriesKillerBalls) * 0.5 * gain
                    - tr(TraitKind::PlaysShortSimplePasses) * 0.5 * d
                    + distribution
            };
            candidates[count] = (j, score);
            count += 1;
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, mate.pos));
            }
        }
        let to = best.map_or_else(
            || view.pitch().clamp(me.pos + attack * 10.0, 4.0),
            |(_, at)| at,
        );
        let d = (to - me.pos).length();
        let dir = match toward(me.pos, to) {
            v if v == DVec2::ZERO => attack,
            v => v,
        };
        // The delivery error of a corner or a free kick: its stage's share narrows the turn.
        let delivery = match kind {
            StoppageKind::Corner => Some(Stage::CORNER_EXECUTE),
            StoppageKind::FreeKick => Some(Stage::FREE_KICK_EXECUTE),
            _ => None,
        };
        let noise = c.hooks().restart_noise;
        let dir = match delivery {
            Some(stage) if noise > 0.0 => {
                let turn = noise * (1.5 - skills.share(stage));
                rotate(dir, map_range(-turn, turn, aim))
            }
            _ => dir,
        };
        let loft = match kind {
            StoppageKind::ThrowIn => THROW_LOFT,
            StoppageKind::Corner => 6.0 + 0.1 * d,
            StoppageKind::FreeKick if delivered => 6.0 + 0.1 * d,
            _ if d > 25.0 => 3.0 + d * 0.08,
            _ => 0.0,
        };
        RestartPass {
            kick: Kick::Pass {
                dir,
                speed: kick_speed(d, loft, t),
                loft,
            },
            target: to,
            fallback: best.is_none(),
            candidates,
            count,
        }
    }
}

/// The metres of a restart's score a keeper's full long-kick or short-throw weight is worth.
const RESTART_TRAIT_M: f64 = 20.0;

/// The line of player `c`'s dribble: toward the goal he attacks, and the side across it,
/// `perp` times `side`, he drifts to, away from the opponent nearest him at
/// `nearest_opp_pos`.
pub(crate) fn dribble_line(
    view: &MatchView<'_>,
    c: usize,
    nearest_opp_pos: DVec2,
) -> (DVec2, DVec2, f64) {
    let carrier = view.player(c);
    let side = &view.teams()[carrier.team];
    let to_goal = side.target_goal() - carrier.pos;
    let goal_dist = to_goal.length();
    let dir_goal = if goal_dist > 1e-6 {
        to_goal / goal_dist
    } else {
        DVec2::new(side.attack_x, 0.0)
    };
    let perp = DVec2::new(-dir_goal.y, dir_goal.x);
    let drift = if (nearest_opp_pos - carrier.pos).dot(perp) > 0.0 {
        -1.0
    } else {
        1.0
    };
    (dir_goal, perp, drift)
}

/// The side across `perp` that points from `pos` toward the centre line: 1 or −1.
pub(crate) fn inside_side(pos: DVec2, perp: DVec2) -> f64 {
    if perp.y * pos.y > 0.0 { -1.0 } else { 1.0 }
}

/// A player's anchor while his side has the ball, moved by his traits: up or back the pitch
/// (getting forward, staying back), toward the channel between full-back and centre-back,
/// and toward goal while the ball is in the final third (arriving late in the box). An
/// override in force plays its action instead: the attack or defend duty's depth, or the late
/// run into the penalty area.
fn trait_anchor(view: &MatchView<'_>, i: usize, anchor: DVec2) -> DVec2 {
    let p = view.player(i);
    if i == view.keeper(p.team) {
        return anchor;
    }
    let traits = view.traits(i);
    let tw = view.tuning().contract.traits();
    let side = &view.teams()[p.team];
    let attack_x = side.attack_x;
    let pitch = view.pitch();
    let (sx, sy) = pitch.scale();
    let act = view.trait_act(i).map(|a| a.kind);
    let mut at = anchor;
    let duty = side.plan.slots[p.slot].depth;
    at.x += match act {
        Some(TraitKind::GetsForward) => (side.plan.attack_depth - duty) * sx,
        Some(TraitKind::StaysBack) => (side.plan.defend_depth - duty) * sx,
        _ => traits.weight(TraitKind::GetsForward, tw) - traits.weight(TraitKind::StaysBack, tw),
    } * attack_x;
    let channel = traits.weight(TraitKind::MovesIntoChannels, tw);
    if channel > 0.0 {
        let to = CHANNEL_Y_M * sy * if at.y < 0.0 { -1.0 } else { 1.0 };
        at.y += (to - at.y).clamp(-channel, channel);
    }
    if view.ball().pos.x * attack_x > pitch.half_length() / 3.0 {
        if act == Some(TraitKind::ArrivesLateInBox) {
            at = DVec2::new(
                (pitch.half_length() - LATE_RUN_DEPTH_M) * attack_x,
                at.y.clamp(-LATE_RUN_WIDTH_M, LATE_RUN_WIDTH_M),
            );
        } else {
            at.x += traits.weight(TraitKind::ArrivesLateInBox, tw) * attack_x;
        }
    }
    pitch.clamp(at, 0.5)
}

/// The tackle decision of `tackler`, in reach of `carrier` (hooks `tackle.decide` and
/// `tackle.foul_risk`). He weighs the clean win against the foul, whose cost
/// [`TACKLE_FOUL_COST`] grows with the danger of a free kick near his own goal: `(1 - s)^2`
/// for the ball `s` of the pitch length from his own goal line, up to double at the line. He
/// tackles when the log of the win chance over that cost, plus his side's willingness, is
/// above 0; otherwise he stays on his feet and nothing is drawn. A tackle he makes takes the
/// fouls module's chances with the foul chance's log-odds shifted by his side's tackle risk
/// (taken from the misses, never from the wins).
pub fn tackle_decision(
    view: &MatchView<'_>,
    tackler: usize,
    _carrier: usize,
    chances: TackleChances,
) -> Option<TackleChances> {
    let team = view.player(tackler).team;
    let side = &view.teams()[team];
    let plan = &side.plan;
    let pitch = view.pitch();
    let from_own = view.ball().xy().x * side.attack_x + pitch.half_length();
    let s = (from_own / pitch.length()).clamp(0.0, 1.0);
    let danger = (1.0 - s) * (1.0 - s);
    let cost = TACKLE_FOUL_COST * chances.p_foul * (1.0 + danger);
    // A presser (a player on a press job) adds his pressing level's willingness to his side's
    // tackling willingness (hook `tackle.decide`, its second input; 0 at the default level).
    let willingness = plan.tackle_willingness
        + if view.player(tackler).pace.job == crate::pace::Job::Press {
            plan.press_willingness
        } else {
            0.0
        };
    // The log of the win chance over the cost, plus the willingness, is above 0 when the win
    // chance is above the cost scaled by e to the minus willingness (1 at the default).
    if chances.p_win <= 0.0 || (cost > 0.0 && chances.p_win <= cost * math::exp(-willingness)) {
        return None;
    }
    if plan.tackle_risk == 0.0 || chances.p_foul <= 0.0 || chances.p_foul >= 1.0 {
        return Some(chances);
    }
    let shifted = contract::logistic(contract::logit(chances.p_foul) + plan.tackle_risk);
    Some(TackleChances {
        p_foul: shifted.min(1.0 - chances.p_win).max(0.0),
        ..chances
    })
}

/// The chance per minute of play that player `i` lapses: the tuned chance, growing past
/// the late minute by `late_growth` per 30 minutes, times `2 · (1 − share)` of his shape
/// choose stage (concentration), which is 1 at rating 10.
pub fn lapse_chance(view: &MatchView<'_>, i: usize) -> f64 {
    let l = &view.tuning().contract.lapse;
    let minute = view.tick() / crate::rules::clock::TICKS_PER_MINUTE;
    let late = f64::from(minute.saturating_sub(l.late_from_minute)) / 30.0;
    let share = view.skills(i).share(Stage::SHAPE_CHOOSE);
    // Professionalism keeps a player's concentration.
    let professional = 1.0
        - view.tuning().contract.person().occasions.professionalism
            * view.personality(i).professionalism;
    (l.per_minute * (1.0 + l.late_growth * late) * 2.0 * (1.0 - share) * professional)
        .clamp(0.0, 1.0)
}

/// The job of every active player from his shape: `Recover` when the rest-defence guard
/// held him back to keep the mentality minimum goal-side and his spot is farther than
/// `return_far_m` (goal-side cover is missing until he gets there), `Offer` when his team is
/// in possession and his stand option fired, `Return` farther than `return_far_m` from his
/// target, `Follow` within `calm_ball_m` of the ball, otherwise `Hold`. An inactive player's
/// job is `Hold`. Out of possession a stand option offers no pass, so its player's job comes
/// from the distances.
fn shape_jobs(
    view: &MatchView<'_>,
    phase: Option<usize>,
    targets: &[DVec2; ROSTER],
    shaped: &[shape::ShapeOutcome; 2],
) -> [Job; ROSTER] {
    let t = &view.tuning().pace;
    let ball = view.ball().xy();
    let mut jobs = [Job::Hold; ROSTER];
    for ((job, p), &target) in jobs.iter_mut().zip(view.players()).zip(targets) {
        if !p.active() {
            continue;
        }
        let far = (target - p.pos).length() > t.return_far_m;
        *job = if far && shaped[p.team].held & (1 << p.slot) != 0 {
            Job::Recover
        } else if phase == Some(p.team) && shaped[p.team].option[p.slot].is_some() {
            Job::Offer
        } else if far {
            Job::Return
        } else if (p.pos - ball).length() <= t.calm_ball_m {
            Job::Follow
        } else {
            Job::Hold
        };
    }
    jobs
}

/// Recovery runs while goal-side cover is missing: while team `def` has fewer active
/// outfield players goal-side of the ball than its mentality minimum (never more than its
/// active outfield players less one), the players ahead of the ball nearest to getting
/// goal-side recover, one per missing player. Nearness is his distance ahead of the ball
/// divided by his recovery-run stage (work rate leading), so a willing runner takes the run
/// before a reluctant one as near. A presser keeps his press, and a chaser his chase.
fn recovery_jobs(view: &MatchView<'_>, jobs: &mut [Job; ROSTER], def: usize, keeper: usize) {
    let players = view.players();
    let side = &view.teams()[def];
    let ball = view.ball().xy();
    let first = def * PLAYERS_PER_TEAM;
    // Metres ahead of the ball toward the other goal: below zero is goal-side.
    let ahead = |i: usize| (players[i].pos.x - ball.x) * side.attack_x;
    let outfield =
        || (first..first + PLAYERS_PER_TEAM).filter(|&i| i != keeper && players[i].active());
    let active = outfield().count();
    let minimum = side.plan.rest_defence.min(active.saturating_sub(1));
    let behind = outfield().filter(|&i| ahead(i) < 0.0).count();
    let mut missing = minimum.saturating_sub(behind);
    if missing == 0 {
        return;
    }
    let mut candidates: Vec<(f64, f64, usize)> = outfield()
        .filter(|&i| {
            ahead(i) >= 0.0 && !matches!(jobs[i], Job::Press | Job::LooseBall | Job::Chase)
        })
        .map(|i| {
            (
                ahead(i) / players[i].derived.knobs.recovery_run,
                ahead(i),
                i,
            )
        })
        .collect();
    candidates.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    for (_, _, i) in candidates {
        if missing == 0 {
            break;
        }
        jobs[i] = Job::Recover;
        missing -= 1;
    }
}

/// A player in a concentration lapse holds: his job is `Hold` until the lapse ends.
fn lapse_jobs(view: &MatchView<'_>, jobs: &mut [Job; ROSTER]) {
    let tick = view.tick();
    for (job, p) in jobs.iter_mut().zip(view.players()) {
        if tick < p.lapse_until && p.active() {
            *job = Job::Hold;
        }
    }
}

/// A player left far from play and from his place, as after a restart he took near a
/// corner flag, gets back into the team's shape by the shortest way: while he is farther from
/// the ball than `anchor_ball_distance` and farther than `anchor_tolerance` from both his
/// places (his anchors in and out of possession, read with the ball itself), his target is
/// the nearer place, whatever option or held spot the shape pass gave him. Pressers, chasers
/// and the cover defender take their targets after this, and a lapse still holds him; the
/// line step leaves him. Returns the roster bits of the players it sent back.
fn back_to_place(view: &MatchView<'_>, targets: &mut [DVec2; ROSTER]) -> u32 {
    let mut sent = 0u32;
    let t = view.tuning();
    let ball = view.ball().xy();
    let teams = view.teams();
    for (i, p) in view.players().iter().enumerate() {
        if !p.active()
            || i == view.keeper(p.team)
            || (p.pos - ball).length() <= t.anchor_ball_distance
        {
            continue;
        }
        let [a_in, a_out] = [true, false].map(|has| teams[p.team].anchor(p.slot, ball, has, t));
        let nearer = if (p.pos - a_in).length() <= (p.pos - a_out).length() {
            a_in
        } else {
            a_out
        };
        if (p.pos - nearer).length() > t.anchor_tolerance {
            targets[i] = nearer;
            sent |= 1 << i;
        }
    }
    sent
}

/// A player in a concentration lapse holds where he stands: his target is his position until
/// the lapse ends.
fn hold_lapses(view: &MatchView<'_>, targets: &mut [DVec2; ROSTER]) {
    let tick = view.tick();
    for (slot, p) in targets.iter_mut().zip(view.players()) {
        if tick < p.lapse_until && p.active() {
            *slot = p.pos;
        }
    }
}

/// The distance from the ball within which keeper `gk` comes off his line for a loose ball:
/// the average keeper's chase distance times his rush factor, widened by his eccentricity
/// (`hooks.rash_chase`).
pub(crate) fn keeper_chase_m(view: &MatchView<'_>, gk: usize) -> f64 {
    if view.traits(gk).is_empty() {
        return chase_base(view, gk);
    }
    // An override holds his action: the trait's range, or his line.
    match view.trait_act(gk).map(|a| a.kind) {
        Some(TraitKind::RushesOut) => keeper_ranges(view, gk).1,
        Some(TraitKind::StaysOnLine) => 0.0,
        _ => keeper_ranges(view, gk).0,
    }
}

/// Keeper `gk`'s chase range before his traits.
fn chase_base(view: &MatchView<'_>, gk: usize) -> f64 {
    let d = &view.player(gk).derived;
    let rash = view.tuning().contract.hooks().rash_chase;
    KEEPER_CHASE_M * d.knobs.rush * (1.0 + rash * d.occasion.eccentricity)
}

/// Keeper `gk`'s chase ranges with his traits: the range he plays (rushing out widens it and
/// staying on his line narrows it, unless his side's line height asks the opposite, which then
/// holds him to the base range), and his rushing-out trait's own range.
pub(crate) fn keeper_ranges(view: &MatchView<'_>, gk: usize) -> (f64, f64) {
    let base = chase_base(view, gk);
    let traits = view.traits(gk);
    let tw = view.tuning().contract.traits();
    let p = view.player(gk);
    let plan = &view.teams()[p.team].plan;
    let rush = traits.weight(TraitKind::RushesOut, tw);
    let stay = traits.weight(TraitKind::StaysOnLine, tw);
    let rush_played = if plan.keeper_rush_off[p.slot] {
        0.0
    } else {
        rush
    };
    let stay_played = if plan.keeper_stay_off[p.slot] {
        0.0
    } else {
        stay
    };
    (
        base * (1.0 + rush_played - stay_played),
        base * (1.0 + rush),
    )
}

/// `true` when keeper `gk` chasing a ball `distance` metres away is a rash chase: one an
/// average keeper would not start.
#[cfg(feature = "sensitivity")]
pub(crate) fn rash_chase(distance: f64) -> bool {
    distance > KEEPER_CHASE_M
}

/// Timed runs (the off-ball movement hook) of team `team` while it has the ball and carrier
/// `c` is someone else: every active outfield attacker with off-ball movement above the
/// average (skill `s` of his off-ball run stage, 0 to 1) whose target lies beyond the onside
/// line, 0.5 m short of the second-last defender, is brought back toward that line by the
/// share `s`; and his target moves `hooks.anchor_space_m · s` across the pitch away from the
/// nearest defender, kept on the pitch.
fn time_runs(view: &MatchView<'_>, targets: &mut [DVec2; ROSTER], team: usize, c: usize) {
    let space = view.tuning().contract.hooks().anchor_space_m;
    let side = &view.teams()[team];
    let attack_x = side.attack_x;
    let line = offside::second_last_depth(team, view.players(), attack_x) - ONSIDE_MARGIN;
    let speed = offside::second_last_speed(team, view.players(), attack_x);
    let keeper = view.keeper(team);
    for (i, p) in view.players().iter().enumerate() {
        if p.team != team || i == c || i == keeper || !p.active() {
            continue;
        }
        let s = view.skills(i).skill(Stage::OFF_BALL_RUN_EXECUTE).max(0.0);
        if s <= 0.0 {
            continue;
        }
        let line = line + line_lag(view, i, speed);
        let mut at = targets[i];
        let depth = at.x * attack_x;
        if line.is_finite() && depth > line {
            at.x = (depth + s * (line - depth)) * attack_x;
        }
        if space > 0.0 {
            let nearest = view
                .players()
                .iter()
                .filter(|q| q.team != team && q.active())
                .map(|q| q.pos)
                .min_by(|a, b| (*a - at).length().total_cmp(&(*b - at).length()));
            if let Some(q) = nearest {
                let away = if at.y >= q.y { 1.0 } else { -1.0 };
                at.y += away * space * s;
            }
        }
        targets[i] = view.pitch().clamp(at, 0.5);
    }
}

/// `true` for a slot of the attack line: a winger or a striker by the formation's position.
pub(crate) fn attack_line(position: Option<crate::data::team::Position>) -> bool {
    use crate::data::team::Position;
    matches!(position, Some(Position::LW | Position::RW | Position::ST))
}

/// The farthest, in metres, a carrier looks for a runner in behind: the longest pass he
/// considers.
const RUN_PASS_M: f64 = 45.0;

/// How far beyond the onside line a run in behind aims, in metres.
const RUN_BEYOND_M: f64 = 6.0;
/// A run coming short aims this far back from where the forward stands, in metres, and lasts
/// at most [`COME_SHORT_S`]. Real forwards' coming-short runs take a median of 7.6 m in 1.6 s
/// (SkillCorner's open data) at speed. An engine forward starts a check from standing in his
/// place and brakes into its end, so a check of 8 m keeps him at a run's speed (4 m/s) for
/// under the 1 s a run needs: the engine's run measure read almost none of them. At 12 m the
/// check is a run by the shared definition.
const COME_SHORT_M: f64 = 12.0;
const COME_SHORT_S: f64 = 4.0;
/// The nearest a run coming short comes to the carrier's depth, in metres.
const COME_SHORT_TO_M: f64 = 5.0;
/// A forward within this many metres of his place stands in it.
const SETTLED_M: f64 = 2.0;

/// How far from the defensive line side `team`'s attack-line players may start a timed run:
/// its forward runs (hook `run.start`); inside its regain window a positive on-winning weight
/// lets them start a run in behind from up to `counter x COUNTER_RUN_M` metres.
fn run_start(view: &MatchView<'_>, team: usize) -> f64 {
    let start = view.teams()[team].plan.run_start_m;
    let counter = counter_weight(view, team);
    if counter > 0.0 {
        start.max(counter * COUNTER_RUN_M)
    } else {
        start
    }
}

/// The side's forward runs (hook `run.start`) while team `team` has the ball and carrier `c`
/// is someone else. Nothing moves at the default of 0. The level sets how far from the
/// defensive line a forward may start a timed run: while a pass is on (the carrier is not
/// pressed and the runner stands ahead of him within a pass's reach), an active attack-line
/// player within `|run_start_m|` of the onside line (0.5 m short of the second-last defender)
/// starts one. Above 0 it is a run in behind: he aims [`RUN_BEYOND_M`] past the onside line,
/// so a pass played once he is past it is offside. Below 0 he checks short from his place
/// (within [`SETTLED_M`] of it), pass on or not: he aims [`COME_SHORT_M`] back toward the ball,
/// never nearer than [`COME_SHORT_TO_M`] ahead of the carrier, keeps that point, and is free
/// after [`COME_SHORT_S`]; back in his place he may check again. A run under way (the
/// runner's job is a timed run) goes on until he arrives, and through a pass that travels
/// forward. The farther from the line a run may start, the more forwards start one, so the
/// frequency rises with the level. Returns the roster bits of the runners.
fn forward_runs(view: &MatchView<'_>, targets: &mut [DVec2; ROSTER], team: usize, c: usize) -> u32 {
    let side = &view.teams()[team];
    let start = run_start(view, team);
    let mut running = 0u32;
    if start == 0.0 {
        return running;
    }
    let attack_x = side.attack_x;
    let line = offside::second_last_depth(team, view.players(), attack_x) - ONSIDE_MARGIN;
    if !line.is_finite() {
        return running;
    }
    let speed = offside::second_last_speed(team, view.players(), attack_x);
    let carrier = view.player(c);
    let carrier_depth = carrier.pos.x * attack_x;
    let pass_on = !pressed(view, c);
    let behind = start > 0.0;
    for (i, p) in view.players().iter().enumerate() {
        if p.team != team || i == c || !p.active() || !attack_line(side.plan.slots[p.slot].position)
        {
            continue;
        }
        let line = line + line_lag(view, i, speed);
        let depth = p.pos.x * attack_x;
        let gap = line - depth;
        let from_carrier = p.pos - carrier.pos;
        // In behind he aims past the line, keeping his place across; coming short he aims at
        // the carrier's feet, `COME_SHORT_TO_M` from him on the line between them.
        let (aim, to_go) = if behind {
            let mut at = targets[i];
            at.x = (line + RUN_BEYOND_M) * attack_x;
            (at, line + RUN_BEYOND_M - depth)
        } else {
            let to = (depth - COME_SHORT_M).max(carrier_depth + COME_SHORT_TO_M);
            let mut at = targets[i];
            at.x = to * attack_x;
            (at, depth - to)
        };
        if to_go <= 1.0 {
            continue;
        }
        let ahead = depth > carrier_depth && from_carrier.length() <= RUN_PASS_M;
        // An onside forward starts a run in behind; any forward may come back short.
        let near = gap <= start.abs() && (!behind || gap >= 0.0);
        // A forward checks short from his place: one away from it (coming back from a check
        // or still moving to it) starts no new check.
        let settled = behind || (p.pos - targets[i]).length() <= SETTLED_M;
        let starts = (pass_on || !behind) && ahead && near && settled;
        // A run coming short lasts [`COME_SHORT_S`]: a check toward the ball, then he is free.
        let short_left = behind
            || f64::from(view.tick().saturating_sub(p.pace.since))
                < COME_SHORT_S * f64::from(crate::TICKS_PER_SECOND);
        let under_way = p.pace.job == Job::TimedRun && short_left;
        if !(starts || under_way) {
            continue;
        }
        // A check under way keeps the point it set out for; a run in behind follows the line.
        let aim = if under_way && !starts && !behind {
            p.target
        } else {
            aim
        };
        targets[i] = view.pitch().clamp(aim, 0.5);
        running |= 1 << i;
    }
    running
}

/// Box runs (named mechanism) while team `team` has the ball in a wide channel of its final
/// third, carried by `c`: every other active attack-line player runs into the penalty area,
/// to the depth of the penalty mark and no wider than the posts' channel
/// ([`LATE_RUN_DEPTH_M`], [`LATE_RUN_WIDTH_M`]), each keeping his side of the pitch, never past
/// the onside line. A cross then has a team-mate to find. Returns the roster bits of the
/// runners.
fn box_runs(view: &MatchView<'_>, targets: &mut [DVec2; ROSTER], team: usize, c: usize) -> u32 {
    let side = &view.teams()[team];
    let attack_x = side.attack_x;
    let pitch = view.pitch();
    let half = pitch.half_length();
    let ball = view.player(c).pos;
    let mut runners = 0u32;
    if ball.y.abs() <= contract::CROSS_WIDE_M || ball.x * attack_x <= half / 3.0 {
        return runners;
    }
    let line = offside::second_last_depth(team, view.players(), attack_x) - ONSIDE_MARGIN;
    let speed = offside::second_last_speed(team, view.players(), attack_x);
    for (i, p) in view.players().iter().enumerate() {
        if p.team != team || i == c || !p.active() || !attack_line(side.plan.slots[p.slot].position)
        {
            continue;
        }
        let depth = if line.is_finite() {
            (half - LATE_RUN_DEPTH_M).min(line + line_lag(view, i, speed))
        } else {
            half - LATE_RUN_DEPTH_M
        };
        let at = DVec2::new(
            depth * attack_x,
            targets[i].y.clamp(-LATE_RUN_WIDTH_M, LATE_RUN_WIDTH_M),
        );
        targets[i] = pitch.clamp(at, 0.5);
        runners |= 1 << i;
    }
    runners
}

/// The line hold (named mechanism) of team `team`'s lone forward while his team has the
/// ball and carrier `c` is someone else: his target moves from his anchor toward the
/// onside line, 0.5 m short of the second-last defender, by the share `lone_line_hold`,
/// and never past that line. His place across the pitch stays the anchor's.
fn hold_the_line(view: &MatchView<'_>, targets: &mut [DVec2; ROSTER], team: usize, c: usize) {
    let share = view.tuning().lone_line_hold;
    if share == 0.0 {
        return;
    }
    let side = &view.teams()[team];
    let Some(slot) = side.lone_forward() else {
        return;
    };
    let i = team * PLAYERS_PER_TEAM + slot;
    if i == c || !view.player(i).active() {
        return;
    }
    let attack_x = side.attack_x;
    let line = offside::second_last_depth(team, view.players(), attack_x) - ONSIDE_MARGIN;
    if !line.is_finite() {
        return;
    }
    let line = line
        + line_lag(
            view,
            i,
            offside::second_last_speed(team, view.players(), attack_x),
        );
    let anchor = targets[i];
    let depth = anchor.x * attack_x;
    let held = (depth + share * (line - depth)).min(line);
    targets[i] = view
        .pitch()
        .clamp(DVec2::new(held * attack_x, anchor.y), 0.5);
}

/// Goal-side cover (named mechanism) while team `def` defends against carrier `c`. The
/// covered attacker is the carrier when he is in `def`'s half inside the central channel,
/// otherwise the most advanced attacker there. The nearest active back-line player of
/// `def` who is neither pressing nor keeping goal covers him, on the line from the
/// attacker to the centre of its own goal:
/// - a carrier is tracked `cover_distance` goal-side of him, divided by the covering
///   player's marking factor (a better marker stays tighter), at his pace, so the covering
///   player stays between him and the goal as he runs;
/// - any other attacker is covered at the depth of the defender's own place in the line,
///   so the cover never plays an attacker onside.
fn cover(
    view: &MatchView<'_>,
    targets: &mut [DVec2; ROSTER],
    def: usize,
    c: usize,
    pressers: &[(f64, usize)],
    keeper: usize,
) -> CoverTrace {
    let t = view.tuning();
    let players = view.players();
    let side = &view.teams()[def];
    let first = def * PLAYERS_PER_TEAM;
    let goal = side.own_goal();
    // Distance from `def`'s goal line toward the halfway line.
    let depth = |at: DVec2| at.x * side.attack_x + view.pitch().half_length();
    let covered = |p: &crate::player::Player| {
        p.active() && p.pos.y.abs() <= t.cover_channel && depth(p.pos) < view.pitch().half_length()
    };
    let attacker = if covered(&players[c]) {
        Some((depth(players[c].pos), c))
    } else {
        let mut most: Option<(f64, usize)> = None;
        for (j, p) in players.iter().enumerate() {
            if p.team == def || !covered(p) {
                continue;
            }
            let d = depth(p.pos);
            if most.is_none_or(|(best, _)| d < best) {
                most = Some((d, j));
            }
        }
        most
    };
    let Some((reach, j)) = attacker else {
        return CoverTrace::NoAttacker;
    };
    let at = players[j].pos;
    let mut best: Option<(f64, usize)> = None;
    for slot in side.back_line() {
        let i = first + slot;
        let p = &players[i];
        if !p.active() || i == keeper || pressers.iter().any(|&(_, k)| k == i) {
            continue;
        }
        let d = (p.pos - at).length();
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    let Some((distance, i)) = best else {
        return CoverTrace::NoDefender { attacker: j };
    };
    let me = &players[i];
    let hold = if j == c {
        let mark = contract::factor(
            t.contract.actions.of(contract::ActionKind::Mark).spread(),
            view.skills(i).share(Stage::MARK_EXECUTE),
        );
        reach - t.cover_distance / mark
    } else {
        // The defender's own place in the line: its anchor, set earlier this tick.
        depth(targets[i])
    }
    .max(0.5);
    let share = if reach > 1e-6 {
        (hold / reach).min(1.0)
    } else {
        0.0
    };
    let mut spot = DVec2::new(
        goal.x + side.attack_x * hold,
        goal.y + (at.y - goal.y) * share,
    );
    if j == c {
        // Lead the spot by the carrier's run over the covering player's braking time, so
        // steering's slow-down near the spot leaves it running at the carrier's pace.
        let braking =
            (me.max_speed() * me.max_speed() / (2.0 * me.max_decel())).max(t.arrive_radius);
        // His cover stage (positioning) scales the lead: a good coverer reads the run.
        spot += players[c].vel * (braking / me.max_speed()) * me.derived.knobs.cover;
    }
    targets[i] = view.pitch().clamp(spot, 0.5);
    CoverTrace::Covered {
        attacker: j,
        defender: i,
        distance,
    }
}

/// Holding position (named mechanism), in both phases: a player whose shape stage
/// (teamwork leading) is under 1 trails `anchor` by the shortfall, `anchor + (pos − anchor) ·
/// (1 − knob)`; an average or better one takes the anchor.
fn lagged(p: &crate::player::Player, anchor: DVec2) -> DVec2 {
    let lag = 1.0 - p.derived.knobs.shape;
    if lag > 0.0 {
        anchor + (p.pos - anchor) * lag
    } else {
        anchor
    }
}

/// The line step (named mechanism) of team `def` out of possession: each active back-line
/// player other than the keeper and those `skip` names (the pressers, the cover defender and
/// a player sent back to his place) whose cover stage (positioning leading) is under 1 steps
/// up or drops with his line late: his target depth moves toward his own depth by the
/// shortfall, `1 − knob`; his place across is unchanged. An average or better coverer moves
/// with the line.
fn step_with_line(
    view: &MatchView<'_>,
    targets: &mut [DVec2; ROSTER],
    def: usize,
    skip: impl Fn(usize) -> bool,
) {
    let side = &view.teams()[def];
    let first = def * PLAYERS_PER_TEAM;
    let keeper = view.keeper(def);
    for slot in side.back_line() {
        let i = first + slot;
        let p = view.player(i);
        let late = 1.0 - p.derived.knobs.cover;
        if !p.active() || late <= 0.0 || i == keeper || skip(i) {
            continue;
        }
        targets[i].x += (p.pos.x - targets[i].x) * late;
    }
}

/// The offside trap (hook `line.offside_trap`): while side `def`'s trap window is open, each
/// active back-line player other than the keeper and those `skip` names steps up
/// [`TRAP_STEP_M`] toward the halfway line, together.
fn trap_step(
    view: &MatchView<'_>,
    targets: &mut [DVec2; ROSTER],
    def: usize,
    skip: impl Fn(usize) -> bool,
) {
    if !view.since_trap_s(def).is_some_and(|s| s < TRAP_WINDOW_S) {
        return;
    }
    let side = &view.teams()[def];
    let first = def * PLAYERS_PER_TEAM;
    let keeper = view.keeper(def);
    for slot in side.back_line() {
        let i = first + slot;
        if !view.player(i).active() || i == keeper || skip(i) {
            continue;
        }
        targets[i].x += side.attack_x * TRAP_STEP_M;
        targets[i] = view.pitch().clamp(targets[i], 0.5);
    }
}

pub const DECISION_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Sets every player's target (anchors at each position's stand for the possession phase, held in both phases by each player's shape stage, stand options from the role menus, the rest-defence guard, the lone forward's line hold, timed runs against the offside line, pressing with a goal-side lean by the press stage, goal-side cover led by the cover stage, the back line's step with its line, chasing) and the job each target is for (carry, press, recovery run, loose ball, chase, offer, return, follow, hold; recovery runs taken first by the recovery-run stage), and decides the carrier's option, pass, clearance, shot, and restart kick.",
    inputs: "Every player's position, velocity, target, activity, slot, lapse, and effective values through the attribute contract (the carrier's see, choose, execute and pressure stages of passes, chips, crosses, shots, long shots, penalties, dribbles and keeper distribution, his skill gates and flair, the restart taker's corner, free-kick and long-throw stages, and every player's press effort (work rate alone), press choose, intercept, shape, cover, recovery-run, rush, marking and off-ball run values, determination and eccentricity); the carrier's mood, pressure handling and professionalism; the kicker's feet, facing and nearest opponent for the weaker-foot cost, and the carrier's feet and facing for the closing pressers' offset; every player's traits of play and the override he plays, and the carrier's last passer; both teams' shapes, plans and the instructions they hold (the stand table, the role menus, the stand-option sizes, the mentality's rest-defence minimum and the guard); the possession phase; each player's familiarity (the familiarity family); the score; the ball; the carrier and how long he has held the ball; the tick; and the engine tuning.",
    outputs: "Every player's target and its job, the carrier's option scores and choice, and the kick of a pass, clearance, shot, or restart. The loop grades each job, draws an urgent job's sprint on its key, and picks the pace level steering arrives at.",
    tuning: &[
        "decision",
        "pace",
        "contract.curve",
        "contract.actions",
        "contract.lapse",
        "contract.hooks",
        "contract.person.mood.risk",
        "contract.person.occasions.professionalism",
        "contract.person.occasions.penalty_pressure",
        "contract.traits",
        "contract.foot",
        "contract.person.floor_drop",
        "shots.penalty_spread",
        "shot_range",
        "aim_noise",
        "shot_noise",
        "shot_speed",
        "shots.loft_max",
        "press_engage",
        "cover_channel",
        "cover_distance",
        "arrive_radius",
        "lone_line_hold",
        "clearances.aim_spread",
        "clearances.wide_chance",
        "clearances.wide_depth",
        "pass_arrival_speed",
        "pass_max_speed",
        "ground_friction",
        "gravity",
        "dt",
    ],
    calibration: "pass_accuracy_pct",
    keys: &[
        Action::ShotScore,
        Action::PassScore,
        Action::DribbleScore,
        Action::HoldScore,
        Action::ClearScore,
        Action::PassAim,
        Action::ClearWide,
        Action::ClearAim,
        Action::ShotSide,
        Action::ShotAim,
        Action::ShotSpread,
        Action::ShotLoft,
        Action::Lapse,
        Action::PaceSprint,
        Action::TraitOverride,
        Action::PassWeight,
    ],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;
    use crate::sim::Simulation;
    use crate::streams::{Action, Key};

    #[test]
    fn a_lofted_pass_reaches_its_target_slow_enough_to_control() {
        let t = crate::tuning::Tuning::default();
        for d in [30.0, 40.0, 45.0] {
            let loft = 3.0 + d * 0.08;
            let mut ball = crate::ball::Ball::at(DVec2::ZERO);
            ball.kick(DVec2::X, kick_speed(d, loft, &t), loft, &t);
            let mut at_target = None;
            for _ in 0..1000 {
                ball.integrate(&t);
                if at_target.is_none() && ball.pos.x >= d {
                    at_target = Some(ball.speed());
                }
            }
            let speed = at_target.unwrap_or_else(|| panic!("a {d} m pass fell short"));
            assert!(
                speed <= t.control_speed,
                "a {d} m pass arrived at {speed} m/s"
            );
            // The bounce after landing carries an uncontrolled ball on by up to a third.
            assert!(
                ball.pos.x < d * 1.34,
                "a {d} m pass rolled on to {}",
                ball.pos.x
            );
        }
    }

    #[test]
    fn an_open_teammate_draws_a_pass_or_a_shot() {
        let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
        // Give the ball to the home striker (slot 9) deep in the away half with a teammate open.
        let striker = 9;
        sim.players[striker].pos = DVec2::new(20.0, 0.0);
        sim.players[10].pos = DVec2::new(30.0, 6.0);
        // The away side stands deep behind him, so the open team-mate is onside (with the
        // away side far up the pitch he would be offside, and the striker reads it).
        for p in sim.players.iter_mut().filter(|p| p.team == 1) {
            p.pos = DVec2::new(48.0, p.pos.y);
        }
        sim.carrier = Some(striker);
        sim.control_since = 0;
        sim.tick = 100;
        let mut kicked = false;
        for _ in 0..50 {
            if sim.decide().is_some() {
                kicked = true;
                break;
            }
        }
        assert!(kicked, "the carrier never passed or shot");
    }

    /// The options of the home striker at `(x, 2)` under `tactics` and the tactics file
    /// `schema`, noise off, with the away outfield players far back and the home team-mates
    /// placed by `mates` (slot, position); every other home outfield player stands out of
    /// passing range.
    fn scene(
        schema: &crate::data::tactics::TacticsSchema,
        tactics: crate::tactics::Tactics,
        x: f64,
        mates: &[(usize, DVec2)],
        opps: &[(usize, DVec2)],
    ) -> Options {
        let mut config = shipped_config(7, 1).unwrap();
        config.tuning.decision.noise = 0.0;
        config.tactics = schema.clone();
        let mut sim = Simulation::new(config.with_tactics(0, tactics)).unwrap();
        let striker = 9;
        sim.players[striker].pos = DVec2::new(x, 2.0);
        let their_keeper = sim.keeper(1);
        for p in sim
            .players
            .iter_mut()
            .filter(|p| p.team == 1 && p.id != their_keeper)
        {
            p.pos = DVec2::new(-50.0, p.pos.y);
        }
        if !mates.is_empty() {
            let own_keeper = sim.keeper(0);
            for (i, p) in sim.players.iter_mut().enumerate() {
                if p.team == 0 && i != striker && p.id != own_keeper {
                    p.pos = DVec2::new(x - 50.0, 30.0);
                }
            }
            sim.players[own_keeper].pos = DVec2::new(-52.0, 0.0);
        }
        for &(slot, pos) in mates {
            sim.players[slot].pos = pos;
        }
        for &(i, pos) in opps {
            sim.players[i].pos = pos;
        }
        sim.carrier = Some(striker);
        sim.tick = 100;
        sim.options(striker)
    }

    /// Tempo is patience: no tempo level moves the hold or the dribble score of a scene.
    #[test]
    fn no_tempo_level_moves_the_hold_or_the_dribble_score() {
        use crate::data::tactics::TEMPO;
        let content = crate::data::test_support::shipped_content();
        let schema = &content.tactics;
        let opp = [(13, DVec2::new(12.0, 3.0))];
        let base = |level: u8| {
            let mut tactics = crate::tactics::Tactics::defaults(schema);
            tactics.instructions[TEMPO] = level;
            let o = scene(schema, tactics, 10.0, &[], &opp);
            (o.hold, o.dribble)
        };
        let levels = schema.instructions.tempo.levels.len() as u8;
        assert_eq!(levels, 5);
        for level in 0..levels {
            assert_eq!(base(level), base(2), "tempo level {level}");
        }
    }

    /// Mentality is a risk appetite with no flat term: with every loss weight at 0, the shot
    /// and the pass scores of a scene are the same under every mentality.
    #[test]
    fn with_no_loss_weight_no_mentality_moves_the_shot_or_the_pass_score() {
        let content = crate::data::test_support::shipped_content();
        let mut schema = content.tactics.clone();
        for m in &mut schema.mentalities {
            m.loss_weight = 0.0;
        }
        let mates = [(10, DVec2::new(30.0, 12.0))];
        let score = |mentality: usize| {
            let mut tactics = crate::tactics::Tactics::defaults(&schema);
            tactics.mentality = mentality as u8;
            let o = scene(&schema, tactics, 40.0, &mates, &[]);
            (o.shot, o.pass.map(|(s, _)| s))
        };
        for m in 0..schema.mentalities.len() {
            assert_eq!(score(m), score(0), "mentality {m}");
        }
    }

    /// A shot has no loss term: no loss weight, risk-averse or risk-seeking, moves the shot
    /// score of a scene.
    #[test]
    fn no_loss_weight_moves_the_shot_score() {
        let content = crate::data::test_support::shipped_content();
        let shot = |weight: f64| {
            let mut schema = content.tactics.clone();
            for m in &mut schema.mentalities {
                m.loss_weight = weight;
            }
            let tactics = crate::tactics::Tactics::defaults(&schema);
            scene(&schema, tactics, 40.0, &[], &[])
                .shot
                .expect("inside the shooting range")
        };
        for weight in [-2.0, 0.0, 0.5, 3.0] {
            assert_eq!(shot(weight), shot(0.0), "loss weight {weight}");
        }
    }

    /// A higher loss weight lowers a risky forward pass (a narrow lane to a marked receiver)
    /// more than a safe square pass (an open lane to a free team-mate).
    #[test]
    fn a_higher_loss_weight_lowers_a_risky_forward_pass_more_than_a_safe_square_pass() {
        let content = crate::data::test_support::shipped_content();
        let pass_score = |weight: f64, risky: bool| {
            let mut schema = content.tactics.clone();
            for m in &mut schema.mentalities {
                m.loss_weight = weight;
            }
            let tactics = crate::tactics::Tactics::defaults(&schema);
            let (mate, opps): (DVec2, Vec<(usize, DVec2)>) = if risky {
                (
                    DVec2::new(-10.0, 4.0),
                    vec![(13, DVec2::new(-12.0, 2.2)), (14, DVec2::new(-10.0, 5.5))],
                )
            } else {
                (DVec2::new(-30.0, 14.0), Vec::new())
            };
            let o = scene(&schema, tactics, -30.0, &[(10, mate)], &opps);
            o.pass.expect("a team-mate in range").0
        };
        let risky_drop = pass_score(0.0, true) - pass_score(2.0, true);
        let safe_drop = pass_score(0.0, false) - pass_score(2.0, false);
        assert!(safe_drop >= 0.0, "{safe_drop}");
        assert!(
            risky_drop > safe_drop,
            "risky {risky_drop}, safe {safe_drop}"
        );
    }

    /// Passing directness weighs a pass by its gain toward goal: the most direct level raises
    /// a 30 m forward pass above the default and leaves a 30 m back pass where it was.
    #[test]
    fn directness_raises_a_forward_pass_and_leaves_a_back_pass() {
        use crate::data::tactics::DIRECTNESS;
        let content = crate::data::test_support::shipped_content();
        let schema = &content.tactics;
        let pass = |level: u8, mate: DVec2| {
            let mut tactics = crate::tactics::Tactics::defaults(schema);
            tactics.instructions[DIRECTNESS] = level;
            scene(schema, tactics, 0.0, &[(10, mate)], &[])
                .pass
                .expect("a team-mate in range")
                .0
        };
        let ahead = DVec2::new(30.0, 2.0);
        let behind = DVec2::new(-30.0, 2.0);
        assert!(pass(4, ahead) > pass(2, ahead));
        assert!((pass(4, behind) - pass(2, behind)).abs() < 1e-12);
    }

    /// Width's second input weighs a pass to a team-mate in a wide channel of the final third
    /// by the level's wide-target value, and leaves a pass to a central team-mate alone.
    #[test]
    fn width_weighs_a_pass_to_a_wide_team_mate_only() {
        use crate::data::tactics::WIDTH;
        let content = crate::data::test_support::shipped_content();
        let schema = &content.tactics;
        let pass = |level: u8, mate: DVec2| {
            let mut tactics = crate::tactics::Tactics::defaults(schema);
            tactics.instructions[WIDTH] = level;
            scene(schema, tactics, 25.0, &[(10, mate)], &[])
                .pass
                .expect("a team-mate in range")
                .0
        };
        let wide = DVec2::new(40.0, 26.0);
        let centre = DVec2::new(40.0, 0.0);
        let levels = &schema.instructions.width.levels;
        for level in 0..levels.len() as u8 {
            let target = levels[usize::from(level)].wide_target;
            assert!((pass(level, wide) - pass(2, wide) - target).abs() < 1e-9);
            assert!((pass(level, centre) - pass(2, centre)).abs() < 1e-12);
        }
    }

    /// The dribble score reads the take-on pressure weight when the carrier is pressed; the
    /// shot score reads its own weight and does not move.
    #[test]
    fn a_pressed_dribble_reads_the_take_on_weight_and_the_shot_its_own() {
        let options = |takeon: f64| {
            let mut config = shipped_config(7, 1).unwrap();
            config.tuning.decision.noise = 0.0;
            config.tuning.decision.takeon_under_pressure = takeon;
            let mut sim = Simulation::new(config).unwrap();
            let striker = 9;
            sim.players[striker].pos = DVec2::new(35.0, 2.0);
            let their_keeper = sim.keeper(1);
            for p in sim
                .players
                .iter_mut()
                .filter(|p| p.team == 1 && p.id != their_keeper)
            {
                p.pos = DVec2::new(-40.0, p.pos.y);
            }
            sim.players[13].pos = DVec2::new(36.5, 2.0);
            sim.carrier = Some(striker);
            sim.tick = 100;
            sim.options(striker)
        };
        let (a, b) = (options(-0.8), options(-1.6));
        assert_ne!(a.dribble, b.dribble);
        assert_eq!(a.shot, b.shot);
    }

    /// Box runs: while a team-mate carries the ball wide in the final third, both home strikers
    /// aim into the penalty area, at the penalty mark's depth and inside the posts' channel, on
    /// a timed run; with the carrier central they do not.
    #[test]
    fn strikers_run_into_the_box_when_the_ball_is_wide_in_the_final_third() {
        let targets = |carrier_y: f64| {
            let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
            let carrier = 6;
            sim.players[carrier].pos = DVec2::new(38.0, carrier_y);
            sim.players[9].pos = DVec2::new(25.0, -8.0);
            sim.players[10].pos = DVec2::new(25.0, 8.0);
            let their_keeper = sim.keeper(1);
            for (k, p) in sim
                .players
                .iter_mut()
                .filter(|p| p.team == 1 && p.id != their_keeper)
                .enumerate()
            {
                p.pos = DVec2::new(49.0, -25.0 + 5.0 * k as f64);
            }
            sim.carrier = Some(carrier);
            sim.tick = 100;
            let view = sim.view();
            sim.config.modules.decision.targets(&view)
        };
        let half = 52.5;
        let wide = targets(28.0);
        for s in [9, 10] {
            let at = wide.targets[s];
            assert!(
                (at.x - (half - LATE_RUN_DEPTH_M)).abs() < 1.0,
                "striker {s} at {at}"
            );
            assert!(at.y.abs() <= LATE_RUN_WIDTH_M + 1e-9, "striker {s} at {at}");
            assert_eq!(wide.jobs[s], Job::TimedRun, "striker {s}");
        }
        let central = targets(0.0);
        for s in [9, 10] {
            assert!(
                (central.targets[s].x - (half - LATE_RUN_DEPTH_M)).abs() >= 1.0
                    || central.jobs[s] != Job::TimedRun,
                "striker {s}"
            );
        }
    }

    /// Forward runs (hook `run.start`): at the default no striker runs; above 0 an onside
    /// striker within the start distance of the line aims past it on a timed run; below 0 a
    /// striker in his place within the start distance checks short toward the carrier, and one
    /// out of his place does not start a check.
    #[test]
    fn forward_runs_start_by_the_distance_from_the_line() {
        // The home side carries at the halfway line; every outfield opponent stands on a line
        // 30 m up, so the onside line is 29.5 m.
        let plan = |start: f64, striker: Option<DVec2>| {
            let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
            let carrier = 6;
            sim.players[carrier].pos = DVec2::new(0.0, 0.0);
            let their_keeper = sim.keeper(1);
            for (k, p) in sim
                .players
                .iter_mut()
                .filter(|p| p.team == 1 && p.id != their_keeper)
                .enumerate()
            {
                p.pos = DVec2::new(30.0, -27.0 + 6.0 * k as f64);
            }
            if let Some(at) = striker {
                sim.players[9].pos = at;
                sim.players[9].target = at;
            }
            sim.teams[0].plan.run_start_m = start;
            sim.carrier = Some(carrier);
            sim.tick = 100;
            let view = sim.view();
            sim.config.modules.decision.targets(&view)
        };
        let line = 29.5;
        // On the line's near side, 1.5 m short of it.
        let near = DVec2::new(28.0, -6.0);
        assert_ne!(plan(0.0, Some(near)).jobs[9], Job::TimedRun, "the default");
        let behind = plan(3.0, Some(near));
        assert_eq!(behind.jobs[9], Job::TimedRun);
        assert!((behind.targets[9].x - (line + RUN_BEYOND_M)).abs() < 1e-9);
        assert_ne!(
            plan(1.0, Some(near)).jobs[9],
            Job::TimedRun,
            "beyond the start"
        );
        // Coming short: the striker stands in his place (his target at the default) within
        // 25 m of the line.
        let place = plan(0.0, None).targets[9];
        assert!(line - place.x <= 25.0 && place.x > 5.0, "place {place}");
        let check = plan(-25.0, Some(place));
        assert_eq!(check.jobs[9], Job::TimedRun);
        let to = (place.x - COME_SHORT_M).max(COME_SHORT_TO_M);
        assert!(
            (check.targets[9].x - to).abs() < 1e-9,
            "{}",
            check.targets[9]
        );
        let away = place + DVec2::new(-2.0 * SETTLED_M, 0.0);
        assert_ne!(
            plan(-25.0, Some(away)).jobs[9],
            Job::TimedRun,
            "out of place"
        );
    }

    /// A forward's timed run goes on while his side's pass travels forward: he keeps his
    /// target and his job. A pass back ends it, and at the default (no forward runs) every run
    /// ends at a pass, as before.
    #[test]
    fn a_forward_run_goes_on_while_a_pass_travels_forward() {
        let goal = DVec2::new(40.0, -5.0);
        let run = |start: f64, ball_vx: f64| {
            let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
            sim.players[9].pos = DVec2::new(30.0, -5.0);
            sim.players[9].target = goal;
            sim.players[9].pace.job = Job::TimedRun;
            sim.teams[0].plan.run_start_m = start;
            sim.carrier = None;
            sim.pass_in_flight = Some(0);
            sim.possession_phase = Some(0);
            sim.ball.pos = crate::math::DVec3::new(-10.0, 20.0, 0.0);
            sim.ball.vel = crate::math::DVec3::new(ball_vx, 0.0, 0.0);
            sim.tick = 100;
            let view = sim.view();
            let t = sim.config.modules.decision.targets(&view);
            (t.targets[9], t.jobs[9])
        };
        assert_eq!(run(-10.0, 12.0), (goal, Job::TimedRun));
        assert_ne!(run(-10.0, -12.0).1, Job::TimedRun, "a pass back");
        assert_ne!(run(0.0, 12.0).1, Job::TimedRun, "the default");
    }

    /// The options of the home striker at `(40, 2)` with an open team-mate, noise off, the
    /// carry cost `cost`, having held the ball `held` ticks, with an opponent `opp` metres
    /// away or none; and the next draw of the stream after scoring them.
    fn carry_scene(cost: f64, held: u32, opp: Option<f64>) -> (Options, f64) {
        let mut config = shipped_config(7, 1).unwrap();
        config.tuning.decision.noise = 0.0;
        config.tuning.decision.carry_s = 1.5;
        config.tuning.decision.carry_cost = cost;
        let mut sim = Simulation::new(config).unwrap();
        let striker = 9;
        sim.players[striker].pos = DVec2::new(40.0, 2.0);
        sim.players[10].pos = DVec2::new(30.0, 12.0);
        let their_keeper = sim.keeper(1);
        for p in sim
            .players
            .iter_mut()
            .filter(|p| p.team == 1 && p.id != their_keeper)
        {
            p.pos = DVec2::new(-40.0, p.pos.y);
        }
        if let Some(d) = opp {
            sim.players[13].pos = DVec2::new(40.0 + d, 2.0);
        }
        sim.carrier = Some(striker);
        sim.tick = 1_000;
        sim.control_since = 1_000 - held;
        let o = sim.options(striker);
        (o, sim.streams.draw(Key::of_match(Action::AddedTime)))
    }

    #[test]
    fn an_unpressed_carrier_pays_the_carry_cost_on_a_pass_and_a_clearance_only() {
        let (free, next_free) = carry_scene(0.0, 0, None);
        let (charged, next_charged) = carry_scene(1.0, 0, None);
        let pass = |o: &Options| o.pass.expect("an open team-mate").0;
        assert!((pass(&free) - pass(&charged) - 1.0).abs() < 1e-12);
        assert!((free.clear - charged.clear - 1.0).abs() < 1e-12);
        assert_eq!(free.shot, charged.shot, "a shot is never charged");
        assert_eq!(free.dribble, charged.dribble);
        assert_eq!(free.hold, charged.hold);
        assert_eq!(next_free, next_charged, "the window draws nothing");
    }

    #[test]
    fn a_pressed_carrier_and_one_past_the_window_pay_no_carry_cost() {
        let pass = |o: &Options| o.pass.expect("an open team-mate").0;
        let (free, _) = carry_scene(0.0, 0, Some(2.0));
        let (charged, _) = carry_scene(1.0, 0, Some(2.0));
        assert_eq!(pass(&free), pass(&charged), "pressed");
        assert_eq!(free.clear, charged.clear, "pressed");
        let (free, _) = carry_scene(0.0, 80, None);
        let (charged, _) = carry_scene(1.0, 80, None);
        assert_eq!(pass(&free), pass(&charged), "past the window");
        assert_eq!(free.clear, charged.clear, "past the window");
    }

    /// `s` seconds in ticks.
    fn ticks(s: f64) -> u32 {
        (s * f64::from(crate::TICKS_PER_SECOND)).round() as u32
    }

    /// The options of the home striker at `(0, 2)` with one open team-mate at `mate`, noise
    /// off, the home side's on-winning weight `counter`, having won the ball `since_s` seconds
    /// ago in open play (`None`: no regain).
    fn counter_scene(counter: f64, since_s: Option<f64>, mate: DVec2) -> Options {
        let mut config = shipped_config(7, 1).unwrap();
        config.tuning.decision.noise = 0.0;
        let mut sim = Simulation::new(config).unwrap();
        let striker = 9;
        sim.players[striker].pos = DVec2::new(0.0, 2.0);
        let (own_keeper, their_keeper) = (sim.keeper(0), sim.keeper(1));
        for (i, p) in sim.players.iter_mut().enumerate() {
            if p.team == 1 && p.id != their_keeper {
                p.pos = DVec2::new(-50.0, p.pos.y);
            } else if p.team == 0 && i != striker && p.id != own_keeper {
                p.pos = DVec2::new(-50.0, 30.0);
            }
        }
        sim.players[10].pos = mate;
        sim.teams[0].plan.counter = counter;
        sim.carrier = Some(striker);
        sim.possession_phase = Some(0);
        sim.tick = 2_000;
        sim.control_since = 2_000;
        sim.regained_at[0] = since_s.map(|s| 2_000 - ticks(s));
        sim.options(striker)
    }

    /// On winning the ball (hook `regain.counter`): inside the regain window the on-winning
    /// weight adds `counter x gain / 45` to a forward pass, nothing to a back pass, and takes
    /// `counter` from the hold; at the default weight, past the window or with no regain it
    /// moves nothing.
    #[test]
    fn the_counter_term_acts_only_inside_the_regain_window() {
        let ahead = DVec2::new(30.0, 2.0);
        let behind = DVec2::new(-30.0, 2.0);
        let pass = |o: &Options| o.pass.expect("an open team-mate").0;
        let hold = |o: &Options| o.hold.expect("an outfield carrier");
        let inside = Some(COUNTER_WINDOW_S / 2.0);
        let base = counter_scene(0.0, inside, ahead);
        let counter = counter_scene(1.0, inside, ahead);
        assert!((pass(&counter) - pass(&base) - 30.0 / 45.0).abs() < 1e-9);
        assert!((hold(&base) - hold(&counter) - 1.0).abs() < 1e-9);
        let back = counter_scene(1.0, inside, behind);
        assert_eq!(pass(&back), pass(&counter_scene(0.0, inside, behind)));
        for since in [Some(COUNTER_WINDOW_S + 0.5), None] {
            let o = counter_scene(1.0, since, ahead);
            let b = counter_scene(0.0, since, ahead);
            assert_eq!((pass(&o), hold(&o)), (pass(&b), hold(&b)), "{since:?}");
        }
    }

    /// The jobs of a home carrier's opponents at `(0, 0)` with one away player at `(d, 0)` and
    /// the rest 45 m away; the away side's counter-press `press` and window `window_s`, having
    /// lost the ball `since_s` seconds ago (`None`: no loss).
    fn press_scene(d: f64, press: f64, window_s: f64, since_s: Option<f64>) -> Targets {
        let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
        let carrier = 6;
        sim.players[carrier].pos = DVec2::ZERO;
        let their_keeper = sim.keeper(1);
        for (k, p) in sim
            .players
            .iter_mut()
            .filter(|p| p.team == 1 && p.id != their_keeper)
            .enumerate()
        {
            p.pos = DVec2::new(-45.0, -27.0 + 6.0 * k as f64);
        }
        let presser = PLAYERS_PER_TEAM + 5;
        sim.players[presser].pos = DVec2::new(d, 0.0);
        sim.teams[1].plan.counter_press = press;
        sim.teams[1].plan.counter_press_s = window_s;
        sim.carrier = Some(carrier);
        sim.possession_phase = Some(0);
        sim.tick = 2_000;
        sim.lost_at[1] = since_s.map(|s| 2_000 - ticks(s));
        let view = sim.view();
        sim.config.modules.decision.targets(&view)
    }

    /// On losing the ball and counter-press length (hooks `loss.counter_press` and
    /// `loss.counter_press_window`): inside the window a factor of 0 leaves a close opponent
    /// unpressing and a high factor sends one from beyond the press distance; the window ends
    /// at its length, and with no loss the press is the pressing level's.
    #[test]
    fn the_counter_press_factor_acts_only_inside_its_window() {
        let presser = PLAYERS_PER_TEAM + 5;
        let presses = |t: Targets| t.jobs[presser] == Job::Press;
        assert!(
            presses(press_scene(3.0, 0.0, 5.0, None)),
            "the press at 3 m"
        );
        assert!(!presses(press_scene(3.0, -1.0, 5.0, Some(1.0))), "all drop");
        assert!(
            presses(press_scene(3.0, -1.0, 5.0, Some(5.5))),
            "window over"
        );
        assert!(!presses(press_scene(35.0, 0.0, 5.0, None)), "out of reach");
        assert!(
            presses(press_scene(35.0, 2.0, 5.0, Some(4.5))),
            "from farther"
        );
        assert!(
            !presses(press_scene(35.0, 2.0, 5.0, Some(5.5))),
            "window over"
        );
        assert!(
            !presses(press_scene(35.0, 2.0, 0.0, Some(0.5))),
            "no window"
        );
        // The default level's counter-press of 0 leaves the press as the pressing level's.
        assert!(presses(press_scene(3.0, 0.0, 5.0, Some(1.0))), "default");
        assert!(!presses(press_scene(35.0, 0.0, 5.0, Some(1.0))), "default");
    }

    /// On winning the ball, the forward-run start: inside the regain window a positive
    /// on-winning weight lets an onside striker start a run in behind from up to
    /// `counter x COUNTER_RUN_M` metres short of the line; past the window, or with a weight of
    /// 0 or below, he does not run.
    #[test]
    fn a_counter_sends_a_striker_in_behind_inside_the_window() {
        let job = |counter: f64, since_s: Option<f64>| {
            let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
            let carrier = 6;
            sim.players[carrier].pos = DVec2::ZERO;
            let their_keeper = sim.keeper(1);
            for (k, p) in sim
                .players
                .iter_mut()
                .filter(|p| p.team == 1 && p.id != their_keeper)
                .enumerate()
            {
                p.pos = DVec2::new(30.0, -27.0 + 6.0 * k as f64);
            }
            // 5 m short of the onside line (29.5 m).
            let at = DVec2::new(24.5, -6.0);
            sim.players[9].pos = at;
            sim.players[9].target = at;
            sim.teams[0].plan.counter = counter;
            sim.carrier = Some(carrier);
            sim.possession_phase = Some(0);
            sim.tick = 2_000;
            sim.regained_at[0] = since_s.map(|s| 2_000 - ticks(s));
            let view = sim.view();
            sim.config.modules.decision.targets(&view).jobs[9]
        };
        assert_eq!(job(1.0, Some(2.0)), Job::TimedRun);
        assert_ne!(job(0.4, Some(2.0)), Job::TimedRun, "4 m reach");
        assert_ne!(job(1.0, Some(COUNTER_WINDOW_S + 1.0)), Job::TimedRun);
        assert_ne!(job(0.0, Some(2.0)), Job::TimedRun);
        assert_ne!(job(-1.0, Some(2.0)), Job::TimedRun);
    }
}
