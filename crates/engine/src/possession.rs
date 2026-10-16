//! Possession: who reaches a loose ball, who may block a shot or clear a cross, whether the
//! keeper reaches a shot and holds it, which way he parries, and who may tackle the carrier.
//! The central loop draws every roll and writes every outcome; this module only says who
//! may try and with what chance.

use crate::contract::{ActionKind, HEAD_HEIGHT_M, Stage, curve};
use crate::math::{DVec2, toward};
use crate::modules::{
    Contest, CrossContest, LooseBall, MatchView, ModuleCard, ParrySide, PassCut, PossessionModule,
};
use crate::rules::fouls::{RUNNING_SPEED, running_win_chance};
use crate::sim::{WIDE_SPREAD, wide_of_goal};
use crate::streams::Action;

/// Possession version 1.
pub struct PossessionV1;

impl PossessionModule for PossessionV1 {
    fn shot_contest(&self, view: &MatchView<'_>) -> Option<usize> {
        shot_contest(view)
    }

    fn blockers(&self, view: &MatchView<'_>, shooter: usize) -> Contest {
        blockers(view, shooter)
    }

    fn save_reach(&self, view: &MatchView<'_>, shooter: usize) -> Option<usize> {
        save_reach(view, shooter)
    }

    fn save_hold(&self, view: &MatchView<'_>, keeper: usize) -> f64 {
        hold_chance(view, keeper)
    }

    fn parry_side(&self, view: &MatchView<'_>, k: usize) -> ParrySide {
        parry_side(view, k)
    }

    fn parry_side_from_draw(&self, draw: f64, threshold: f64) -> f64 {
        side_from_draw(draw, threshold)
    }

    fn cross_clearers(&self, view: &MatchView<'_>) -> Option<CrossContest> {
        cross_clearers(view)
    }

    fn clearance_line(&self, view: &MatchView<'_>, i: usize, wide: bool) -> (DVec2, f64) {
        clearance_line(view, i, wide)
    }

    fn loose_ball(&self, view: &MatchView<'_>) -> Option<LooseBall> {
        loose_ball(view, view.tuning().reach_radius)
    }

    fn tacklers(&self, view: &MatchView<'_>, c: usize) -> Option<u32> {
        tacklers(view, c)
    }

    fn pass_cut(&self, view: &MatchView<'_>, i: usize) -> Option<PassCut> {
        pass_cut(view, i)
    }

    fn take_on(&self, view: &MatchView<'_>, c: usize, i: usize) -> Option<f64> {
        take_on(view, c, i)
    }
}

/// The team whose shot is in flight and fast: slower than `control_speed`, the ordinary
/// contest for a loose ball applies instead.
fn shot_contest(view: &MatchView<'_>) -> Option<usize> {
    let shooter = view.shot_in_flight()?;
    (view.ball().speed() > view.tuning().control_speed).then_some(shooter)
}

/// The chance keeper `k` holds a save: the tuned hold threshold as the base of a contest of
/// his hold stage (handling) against the average player, less his punching trait's weight (he
/// punches it away instead).
pub fn hold_chance(view: &MatchView<'_>, k: usize) -> f64 {
    let t = view.tuning();
    let c = &t.contract;
    let p = c.actions.of(ActionKind::Hold);
    let hold = curve::contest(
        t.shots.save_hold,
        p.k(),
        view.skills(k).f(Stage::HOLD_EXECUTE),
        curve::f(10.0, &c.curve),
        p.floor(),
        p.ceiling(),
    );
    let traits = view.traits(k);
    if traits.is_empty() {
        hold
    } else {
        (hold
            - traits.weight(
                crate::traits::TraitKind::PunchesRatherThanCatches,
                c.traits(),
            ))
        .max(0.0)
    }
}

/// Each outfield defender of the side that did not shoot within his block reach
/// (`block_reach` times his block factor) of a shot under `reach_height` may try once per
/// shot to block it.
fn blockers(view: &MatchView<'_>, shooter: usize) -> Contest {
    let t = view.tuning();
    let chance = t.shots.block_chance;
    let ball = view.ball();
    if ball.pos.z > t.reach_height {
        return Contest { mask: 0, chance };
    }
    let ball_xy = ball.xy();
    let keeper = view.keeper(1 - shooter);
    let tried = view.blockers_tried();
    let mut mask = 0u32;
    for (i, p) in view.players().iter().enumerate() {
        let bit = 1u32 << i;
        if p.team == shooter
            || i == keeper
            || !p.active()
            || tried & bit != 0
            || (p.pos - ball_xy).length() >= t.shots.block_reach * p.derived.knobs.block
        {
            continue;
        }
        mask |= bit;
    }
    Contest { mask, chance }
}

/// The acting keeper of the side that did not shoot, when he is within `keeper_reach` of a
/// shot on target under the bar and has not been beaten in this flight.
fn save_reach(view: &MatchView<'_>, shooter: usize) -> Option<usize> {
    let t = view.tuning();
    let k = view.keeper(1 - shooter);
    let ball = view.ball();
    let keeper = view.player(k);
    if view.keeper_beaten()
        || ball.pos.z >= t.crossbar_height
        || !keeper.active()
        || (keeper.pos - ball.xy()).length() >= t.keeper_reach
    {
        return None;
    }
    Some(k)
}

/// A parry goes along the goal line away from the goal centre; a ball dead centre goes to
/// either side with even chances.
fn parry_side(view: &MatchView<'_>, _k: usize) -> ParrySide {
    let y = view.ball().pos.y;
    if y == 0.0 {
        ParrySide::Draw(0.5)
    } else {
        ParrySide::Fixed(y.signum())
    }
}

fn side_from_draw(draw: f64, threshold: f64) -> f64 {
    if draw < threshold { 1.0 } else { -1.0 }
}

/// While an open-play pass is in flight, fast, under `reach_height` and inside the penalty
/// area of the side that did not play it, each active defending outfield player within
/// `cross_reach` may try once per flight to clear it.
fn cross_clearers(view: &MatchView<'_>) -> Option<CrossContest> {
    let passer = view.pass_in_flight()?;
    let t = view.tuning();
    let c = &t.clearances;
    let ball = view.ball();
    if c.cross_chance <= 0.0 || ball.speed() <= t.control_speed || ball.pos.z > t.reach_height {
        return None;
    }
    let def = 1 - passer;
    let own_goal_x = -view.attack_x(def);
    let ball_xy = ball.xy();
    if !view.pitch().in_penalty_area(ball_xy, own_goal_x) {
        return None;
    }
    let keeper = view.keeper(def);
    let tried = view.clearers_tried();
    let mut mask = 0u32;
    for (i, p) in view.players().iter().enumerate() {
        let bit = 1u32 << i;
        if p.team != def
            || i == keeper
            || !p.active()
            || tried & bit != 0
            || (p.pos - ball_xy).length() >= c.cross_reach
        {
            continue;
        }
        mask |= bit;
    }
    Some(CrossContest {
        team: def,
        contest: Contest {
            mask,
            chance: c.cross_chance,
        },
        wide_chance: (c.wide_chance > 0.0).then_some(c.wide_chance),
    })
}

/// A clearance by player `i` goes away from his own goal centre, or wide toward his own goal
/// line.
fn clearance_line(view: &MatchView<'_>, i: usize, wide: bool) -> (DVec2, f64) {
    let own_goal_x = -view.attack_x(view.player(i).team);
    let ball_xy = view.ball().xy();
    if wide {
        (wide_of_goal(ball_xy, own_goal_x), WIDE_SPREAD)
    } else {
        let goal = view.pitch().goal_centre(own_goal_x);
        let away = match toward(goal, ball_xy) {
            v if v == DVec2::ZERO => DVec2::new(-own_goal_x.signum(), 0.0),
            v => v,
        };
        (away, view.tuning().clearances.cross_spread)
    }
}

/// `true` while player `p` of the side without the ball wins a pass in flight only by cutting
/// it out ([`pass_cut`]): the cut is on (`decision.pass_cut` above 0), the pass of the other
/// side is in flight and the ball still runs faster than a pass reaches its receiver
/// (`pass_arrival_speed`): past that the pass has run out, the ball is loose for both sides and
/// the nearest player within reach takes it. A keeper claims as before.
#[inline]
fn cut_only(view: &MatchView<'_>, p: &crate::player::Player, keeper: bool, speed: f64) -> bool {
    !keeper
        && view.tuning().decision.pass_cut > 0.0
        && view.pass_in_flight().is_some_and(|team| team != p.team)
        && speed > view.tuning().pass_arrival_speed
}

/// How far a defender stretches to block a pass, as a multiple of his reach for a ball he
/// controls (`reach_radius` times his intercept factor): a leg out, not a touch he keeps.
pub const BLOCK_STRETCH: f64 = 2.0;

/// Defender `i`'s try to cut out the pass in flight, as it passes him. He tries once per
/// flight, on the first tick the ball is within his stretch ([`BLOCK_STRETCH`] times his reach
/// for a ball he controls: `reach_radius` times his intercept factor), under his reach height,
/// still running faster than a pass's arrival speed and no longer coming closer to him. His chance is
/// `decision.pass_cut` x twice his stage share (1 for a player rated 10) x how close it passes
/// him (1 at his feet, 0 at the edge of his stretch) x its pace against what he can control
/// (1 up to `control_speed` times his receive factor, then the square of that speed over the
/// ball's). A ball within his reach that he can control he reads and takes (the intercept see
/// stage: anticipation leading; above head height, the header stage), an interception; any
/// other he can only block (the block execute stage: positioning leading), a deflection.
fn pass_cut(view: &MatchView<'_>, i: usize) -> Option<PassCut> {
    let t = view.tuning();
    let scale = t.decision.pass_cut;
    let passing = view.pass_in_flight()?;
    let p = view.player(i);
    if scale <= 0.0
        || p.team == passing
        || !p.active()
        || i == view.keeper(p.team)
        || view.cutters_tried() & (1u32 << i) != 0
    {
        return None;
    }
    let ball = view.ball();
    let speed = ball.speed();
    if ball.pos.z > p.derived.reach_m || speed <= t.pass_arrival_speed {
        return None;
    }
    let xy = ball.xy();
    let k = &p.derived.knobs;
    let reach = t.reach_radius * k.intercept;
    let stretch = reach * BLOCK_STRETCH;
    let d = (p.pos - xy).length();
    if d >= stretch || DVec2::new(ball.vel.x, ball.vel.y).dot(p.pos - xy) > 0.0 {
        return None;
    }
    let controls = t.control_speed * k.receive;
    let control = d < reach && speed <= controls;
    let s = view.skills(i);
    let stage = if !control {
        Stage::BLOCK_EXECUTE
    } else if ball.pos.z > HEAD_HEIGHT_M {
        Stage::HEADER_EXECUTE
    } else {
        Stage::INTERCEPT_SEE
    };
    let share = s.share(stage);
    let ratio = (controls / speed).min(1.0);
    let pace = ratio * ratio;
    let close = 1.0 - d / stretch;
    Some(PassCut {
        chance: (scale * 2.0 * share * close * pace).clamp(0.0, 1.0),
        control,
    })
}

/// How far, as a multiple of `tackle_reach`, a defender goal-side of a running carrier meets
/// him in a take-on: the pressed distance at the default reach.
pub const TAKE_ON_STRETCH: f64 = 2.5;

/// Defender `i`'s contest of a take-on by carrier `c`, who runs with the ball. He meets him
/// once, on the first tick that he stands goal-side of him within [`TAKE_ON_STRETCH`] times
/// `tackle_reach`, and not again until he has left that distance; the keeper does not, nor a
/// defender a foul still holds back. These are the opponents a take-on beats when it passes
/// them. His chance is `decision.take_on_contest` x twice the running tackle contest of the
/// two (his tackle stage against the carrier's dribble side, anchored on the even tackle: 0.5
/// between two players rated 10 at the default `tackle_win_base`) x the angle the carrier
/// comes at him (`(1 + cos) / 2` of the angle between his run and the line to the defender: 1
/// running straight at him, 0.5 across him, 0 straight away) x how square he stands (`(1 -
/// cos) / 2` of his facing against that line: 1 facing the carrier, 0 with his back to him) x
/// the pace (1 when the carrier does not close on him, falling to 0 as he closes at twice the
/// defender's top speed). The defender's read of the take-on ([`take_on_read`]) moves the
/// last two: a defender who reads it sets himself square before the carrier arrives and meets
/// him early, so less of his turned stance and of the carrier's closing speed counts against
/// him; one who reads it badly is caught more turned and later. A defender rated 10 reads it
/// as the stance and the pace stand.
fn take_on(view: &MatchView<'_>, c: usize, i: usize) -> Option<f64> {
    let t = view.tuning();
    let scale = t.decision.take_on_contest;
    if scale <= 0.0 {
        return None;
    }
    let carrier = view.player(c);
    let speed = carrier.vel.length();
    let p = view.player(i);
    if speed <= RUNNING_SPEED
        || !p.active()
        || p.team == carrier.team
        || i == view.keeper(p.team)
        || view.tick() < p.foul_ready
        || view.duelled() & (1u32 << i) != 0
    {
        return None;
    }
    let to = p.pos - carrier.pos;
    let d = to.length();
    if d <= 0.0
        || d >= t.tackle_reach * TAKE_ON_STRETCH
        || to.x * view.attack_x(carrier.team) <= 0.0
    {
        return None;
    }
    let line = to / d;
    let angle = (1.0 + (carrier.vel / speed).dot(line)) / 2.0;
    // How much of a bad stance and of the closing speed counts: 1 at the neutral read, none
    // at a full read, twice at none.
    let unread = 2.0 * (1.0 - take_on_read(view.skills(i)));
    let turned = (1.0 + p.facing.dot(line)) / 2.0;
    let square = (1.0 - turned * unread).clamp(0.0, 1.0);
    let closing = (carrier.vel - p.vel).dot(line).max(0.0);
    let pace = (1.0 - unread * closing / (2.0 * p.max_speed())).clamp(0.0, 1.0);
    let contest = 2.0 * running_win_chance(view.skills(i), view.skills(c), t);
    Some((scale * contest * angle * square * pace).clamp(0.0, 1.0))
}

/// A defender's read of a take-on, 0 to 1 (0.5 for a player rated 10): how early he sees the
/// carrier come and sets himself to meet him. It reads the attribute contract's two stages of
/// stepping to a carrier, in equal parts: press choose (positioning leading, anticipation and
/// decisions supporting) and cover execute (positioning leading, anticipation and
/// concentration supporting).
#[inline]
pub fn take_on_read(s: crate::contract::Skills<'_>) -> f64 {
    (0.5 * s.share(crate::contract::Stage::PRESS_CHOOSE)
        + 0.5 * s.share(crate::contract::Stage::COVER_EXECUTE))
    .clamp(0.0, 1.0)
}

/// The loose ball as every reach test of one tick reads it: its place on the ground, its
/// height and its speed, each worked out once.
#[derive(Clone, Copy)]
struct BallAt {
    xy: DVec2,
    z: f64,
    speed: f64,
    /// The ball's velocity along the ground.
    vel: DVec2,
}

/// `true` while the ball moving at `ball_vel` along the ground is still on its way from player
/// `p`, who kicked it last: it is faster than a pass reaches its receiver
/// (`pass_arrival_speed`) and still runs along the line he sent it ([`MatchView::last_kick_dir`]).
/// A kicker neither plays nor chases his own kick on its way, wherever he stands against the
/// ball (a keeper holds it up to his reach from his body, so a kick can start behind him);
/// once it has slowed to a pass's arrival speed, or a rebound turns it back, he may.
#[inline]
pub(crate) fn leaving_kicker(
    view: &MatchView<'_>,
    p: &crate::player::Player,
    _ball_xy: DVec2,
    ball_vel: DVec2,
) -> bool {
    view.last_kicker() == Some(p.id)
        && ball_vel.length() > view.tuning().pass_arrival_speed
        && ball_vel.dot(view.last_kick_dir()) > 0.0
}

/// How far player `p` reaches a ball at `ball.xy` and height `ball.z`, or `None` when he
/// cannot: an outfield player within `reach_radius` times his intercept factor, a keeper
/// within `keeper_reach` times his claim factor (and his claim range for a ball above head
/// height in his own penalty area). The ball must be under `reach_height` times his aerial
/// factor, and only a keeper reaches a ball faster than `control_speed` times his receive
/// factor. An outfield player receiving his side's pass in flight reaches it only within
/// `1 - decision.touch_under_pressure` times his pressure penalty of his reach
/// ([`reception_penalty`]).
#[inline]
fn reach(
    view: &MatchView<'_>,
    p: &crate::player::Player,
    keeper: bool,
    reach_radius: f64,
    ball: BallAt,
) -> Option<f64> {
    let t = view.tuning();
    let k = &p.derived.knobs;
    if !p.active() || ball.z > p.derived.reach_m {
        return None;
    }
    if !keeper && ball.speed > t.control_speed * k.receive {
        return None;
    }
    if cut_only(view, p, keeper, ball.speed) {
        return None;
    }
    let reach = if keeper {
        let lofted = ball.z > HEAD_HEIGHT_M
            && view
                .pitch()
                .in_penalty_area(ball.xy, -view.attack_x(p.team));
        let range = if lofted { k.claim_range } else { 1.0 };
        t.keeper_reach * k.claim_reach * range
    } else {
        reach_radius * k.intercept
    };
    let d = (p.pos - ball.xy).length();
    if d >= reach || leaving_kicker(view, p, ball.xy, ball.vel) {
        return None;
    }
    // The pressure model: an outfield receiver of his side's pass reaches the ball only within
    // `1 - touch_under_pressure x penalty` of his reach, so a pressed first touch more often
    // lets the ball run on.
    let touch = t.decision.touch_under_pressure;
    if !keeper
        && touch > 0.0
        && view.pass_in_flight() == Some(p.team)
        && d >= reach * (1.0 - touch).max(0.0)
    {
        let cut = touch * reception_penalty(view, p, ball.speed / (t.control_speed * k.receive));
        if d >= reach * (1.0 - cut) {
            return None;
        }
    }
    Some(d)
}

/// The pressure model's penalty of player `p`'s reception of a ball at `speed_share` of the
/// fastest he controls: the intensity he feels (his read of the press, anticipation leading)
/// x the reception's difficulty (the ball's speed, facing his own goal, eased by his read) x
/// (1 - his reception resistance: first touch, composure, strength and balance).
pub fn reception_penalty(view: &MatchView<'_>, p: &crate::player::Player, speed_share: f64) -> f64 {
    let stages = view.skills(p.id);
    let read = crate::pressure::read(stages);
    let intensity = crate::pressure::felt(view.players(), p, read, view.attack_x(p.team));
    if intensity <= 0.0 {
        return 0.0;
    }
    let facing_back = (1.0 - p.facing.x * view.attack_x(p.team)) / 2.0;
    let difficulty = crate::pressure::reception_difficulty(speed_share, facing_back, read);
    let resistance = crate::pressure::reception_resistance(stages);
    crate::pressure::penalty(intensity, difficulty, resistance)
}

/// The nearest active player within reach of a loose ball (see [`reach`]). A ball faster
/// than `control_speed` is fast, and only a keeper takes it, holding it with the catch
/// chance times his claim factor. An outfield player who meets the ball above head height
/// heads it: he controls it with a contest of his header stage against the best header of
/// an opponent within reach (the average player when none is).
fn loose_ball(view: &MatchView<'_>, reach_radius: f64) -> Option<LooseBall> {
    let t = view.tuning();
    let ball = view.ball();
    let at = BallAt {
        xy: ball.xy(),
        z: ball.pos.z,
        speed: ball.speed(),
        vel: DVec2::new(ball.vel.x, ball.vel.y),
    };
    let fast = at.speed > t.control_speed;
    let keepers = [view.keeper(0), view.keeper(1)];
    let mut best: Option<(f64, usize)> = None;
    let mut any_height = false;
    for (i, p) in view.players().iter().enumerate() {
        let keeper = i == keepers[p.team];
        if at.z <= p.derived.reach_m {
            any_height = true;
        }
        if fast && !keeper {
            continue;
        }
        let Some(d) = reach(view, p, keeper, reach_radius, at) else {
            continue;
        };
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    if !any_height {
        return None;
    }
    let c = &t.contract;
    let claim = c.actions.of(ActionKind::Claim);
    let catch_chance = match best {
        Some((_, i)) if i == keepers[view.player(i).team] => {
            let knob = view.player(i).derived.knobs.claim_reach;
            if knob == 1.0 {
                t.keeper_catch_chance
            } else {
                (t.keeper_catch_chance * knob).clamp(claim.floor(), claim.ceiling())
            }
        }
        _ => t.keeper_catch_chance,
    };
    let header = match best {
        Some((_, i)) if at.z > HEAD_HEIGHT_M && i != keepers[view.player(i).team] => {
            let me = view.player(i);
            let rival = view
                .players()
                .iter()
                .enumerate()
                .filter(|&(j, q)| {
                    q.team != me.team
                        && reach(view, q, j == keepers[q.team], reach_radius, at).is_some()
                })
                .map(|(j, _)| view.skills(j).f(Stage::HEADER_EXECUTE))
                .fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))))
                .unwrap_or_else(|| curve::f(10.0, &c.curve));
            let h = c.actions.of(ActionKind::Header);
            Some(curve::contest(
                h.base(),
                h.k(),
                view.skills(i).f(Stage::HEADER_EXECUTE),
                rival,
                h.floor(),
                h.ceiling(),
            ))
        }
        _ => None,
    };
    Some(LooseBall {
        best,
        fast,
        catch_chance,
        header,
    })
}

/// After the control cooldown, every active opponent within `tackle_reach` of the ball whose
/// last foul no longer holds him back may try to tackle carrier `c`.
fn tacklers(view: &MatchView<'_>, c: usize) -> Option<u32> {
    let t = view.tuning();
    let tick = view.tick();
    if tick.saturating_sub(view.control_since()) < t.control_cooldown_ticks {
        return None;
    }
    let team = view.player(c).team;
    let ball_xy = view.ball().xy();
    let mut mask = 0u32;
    for (i, p) in view.players().iter().enumerate() {
        if !p.active()
            || p.team == team
            || tick < p.foul_ready
            || (p.pos - ball_xy).length() > t.tackle_reach
        {
            continue;
        }
        mask |= 1u32 << i;
    }
    Some(mask)
}

pub const POSSESSION_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Says who may contest the ball: the nearest player to a loose ball and whether a fast ball can be caught, the defenders who may block a shot or clear a cross and along which line, whether the keeper reaches a shot and holds it, the side of a parry, and the opponents who may tackle the carrier.",
    inputs: "Every player's position, team, activity, and foul hold-back; the ball; the carrier and when he gained the ball; the shot or pass in flight and who already tried to stop it; whether the keeper was beaten; the keepers and their traits of play; each team's attack direction; the tick; and the engine tuning.",
    outputs: "The loose-ball winner and catch chance, the blockers, clearers, and tacklers as roster masks with their chances, the keeper who can save, the save-hold threshold, the parry side, and the clearance line.",
    tuning: &[
        "reach_radius",
        "keeper_reach",
        "reach_height",
        "control_speed",
        "control_cooldown_ticks",
        "decision.touch_under_pressure",
        "decision.pass_cut",
        "decision.take_on_contest",
        "tackle_reach",
        "keeper_catch_chance",
        "crossbar_height",
        "shots.block_chance",
        "shots.block_reach",
        "shots.save_hold",
        "clearances.cross_chance",
        "clearances.wide_chance",
        "clearances.cross_reach",
        "clearances.cross_spread",
        "contract.traits",
    ],
    calibration: "possession_pct",
    keys: POSSESSION_KEYS,
};

const POSSESSION_KEYS: &[Action] = &[
    Action::Block,
    Action::SaveHold,
    Action::ParrySide,
    Action::CrossClear,
    Action::CrossWide,
    Action::KeeperCatch,
    Action::Header,
    Action::PassCut,
    Action::TakeOn,
];

/// A faulty possession module for the gate tests: an outfield player reaches a loose ball
/// from 1 cm further than `reach_radius`. Test builds only.
#[cfg(feature = "scenario")]
pub struct PossessionFaulty;

#[cfg(feature = "scenario")]
impl PossessionModule for PossessionFaulty {
    fn shot_contest(&self, view: &MatchView<'_>) -> Option<usize> {
        shot_contest(view)
    }

    fn blockers(&self, view: &MatchView<'_>, shooter: usize) -> Contest {
        blockers(view, shooter)
    }

    fn save_reach(&self, view: &MatchView<'_>, shooter: usize) -> Option<usize> {
        save_reach(view, shooter)
    }

    fn save_hold(&self, view: &MatchView<'_>, keeper: usize) -> f64 {
        hold_chance(view, keeper)
    }

    fn parry_side(&self, view: &MatchView<'_>, k: usize) -> ParrySide {
        parry_side(view, k)
    }

    fn parry_side_from_draw(&self, draw: f64, threshold: f64) -> f64 {
        side_from_draw(draw, threshold)
    }

    fn cross_clearers(&self, view: &MatchView<'_>) -> Option<CrossContest> {
        cross_clearers(view)
    }

    fn clearance_line(&self, view: &MatchView<'_>, i: usize, wide: bool) -> (DVec2, f64) {
        clearance_line(view, i, wide)
    }

    fn loose_ball(&self, view: &MatchView<'_>) -> Option<LooseBall> {
        loose_ball(view, view.tuning().reach_radius + 0.01)
    }

    fn tacklers(&self, view: &MatchView<'_>, c: usize) -> Option<u32> {
        tacklers(view, c)
    }

    fn pass_cut(&self, view: &MatchView<'_>, i: usize) -> Option<PassCut> {
        pass_cut(view, i)
    }

    fn take_on(&self, view: &MatchView<'_>, c: usize, i: usize) -> Option<f64> {
        take_on(view, c, i)
    }
}

#[cfg(feature = "scenario")]
pub const POSSESSION_FAULTY_CARD: ModuleCard = ModuleCard {
    purpose: "Test-only: possession version 1 with the outfield loose-ball reach 1 cm longer, to prove the gate catches one changed output.",
    inputs: "As possession version 1.",
    outputs: "As possession version 1.",
    tuning: &["reach_radius"],
    calibration: "none: test-only faulty module",
    keys: POSSESSION_KEYS,
};
