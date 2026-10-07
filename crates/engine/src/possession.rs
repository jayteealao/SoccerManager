//! Possession: who reaches a loose ball, who may block a shot or clear a cross, whether the
//! keeper reaches a shot and holds it, which way he parries, and who may tackle the carrier.
//! The central loop draws every roll and writes every outcome; this module only says who
//! may try and with what chance.

use crate::contract::{ActionKind, HEAD_HEIGHT_M, Stage, curve};
use crate::math::{DVec2, toward};
use crate::modules::{
    Contest, CrossContest, LooseBall, MatchView, ModuleCard, ParrySide, PossessionModule,
};
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
}

/// The team whose shot is in flight and fast: slower than `control_speed`, the ordinary
/// contest for a loose ball applies instead.
fn shot_contest(view: &MatchView<'_>) -> Option<usize> {
    let shooter = view.shot_in_flight()?;
    (view.ball().speed() > view.tuning().control_speed).then_some(shooter)
}

/// The chance keeper `k` holds a save: the tuned hold threshold as the base of a contest of
/// his hold stage (handling) against the average player.
pub fn hold_chance(view: &MatchView<'_>, k: usize) -> f64 {
    let t = view.tuning();
    let c = &t.contract;
    let p = c.actions.of(ActionKind::Hold);
    curve::contest(
        t.shots.save_hold,
        p.k(),
        view.skills(k).f(Stage::HOLD_EXECUTE),
        curve::f(10.0, &c.curve),
        p.floor(),
        p.ceiling(),
    )
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

/// How far player `p` reaches a ball at `ball_xy` and height `z`, or `None` when he cannot:
/// an outfield player within `reach_radius` times his intercept factor, a keeper within
/// `keeper_reach` times his claim factor (and his claim range for a ball above head height
/// in his own penalty area). The ball must be under `reach_height` times his aerial factor,
/// and only a keeper reaches a ball faster than `control_speed` times his receive factor.
fn reach(
    view: &MatchView<'_>,
    p: &crate::player::Player,
    keeper: bool,
    reach_radius: f64,
) -> Option<f64> {
    let t = view.tuning();
    let ball = view.ball();
    let k = &p.derived.knobs;
    if !p.active() || ball.pos.z > t.reach_height * k.aerial_reach {
        return None;
    }
    if !keeper && ball.speed() > t.control_speed * k.receive {
        return None;
    }
    let ball_xy = ball.xy();
    let reach = if keeper {
        let lofted = ball.pos.z > HEAD_HEIGHT_M
            && view
                .pitch()
                .in_penalty_area(ball_xy, -view.attack_x(p.team));
        let range = if lofted { k.claim_range } else { 1.0 };
        t.keeper_reach * k.claim_reach * range
    } else {
        reach_radius * k.intercept
    };
    let d = (p.pos - ball_xy).length();
    (d < reach).then_some(d)
}

/// The nearest active player within reach of a loose ball (see [`reach`]). A ball faster
/// than `control_speed` is fast, and only a keeper takes it, holding it with the catch
/// chance times his claim factor. An outfield player who meets the ball above head height
/// heads it: he controls it with a contest of his header stage against the best header of
/// an opponent within reach (the average player when none is).
fn loose_ball(view: &MatchView<'_>, reach_radius: f64) -> Option<LooseBall> {
    let t = view.tuning();
    let ball = view.ball();
    let fast = ball.speed() > t.control_speed;
    let keepers = [view.keeper(0), view.keeper(1)];
    let mut best: Option<(f64, usize)> = None;
    let mut any_height = false;
    for (i, p) in view.players().iter().enumerate() {
        let keeper = i == keepers[p.team];
        if ball.pos.z <= t.reach_height * p.derived.knobs.aerial_reach {
            any_height = true;
        }
        if fast && !keeper {
            continue;
        }
        let Some(d) = reach(view, p, keeper, reach_radius) else {
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
        Some((_, i)) if ball.pos.z > HEAD_HEIGHT_M && i != keepers[view.player(i).team] => {
            let me = view.player(i);
            let rival = view
                .players()
                .iter()
                .enumerate()
                .filter(|&(j, q)| {
                    q.team != me.team
                        && reach(view, q, j == keepers[q.team], reach_radius).is_some()
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
    inputs: "Every player's position, team, activity, and foul hold-back; the ball; the carrier and when he gained the ball; the shot or pass in flight and who already tried to stop it; whether the keeper was beaten; the keepers; each team's attack direction; the tick; and the engine tuning.",
    outputs: "The loose-ball winner and catch chance, the blockers, clearers, and tacklers as roster masks with their chances, the keeper who can save, the save-hold threshold, the parry side, and the clearance line.",
    tuning: &[
        "reach_radius",
        "keeper_reach",
        "reach_height",
        "control_speed",
        "control_cooldown_ticks",
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
