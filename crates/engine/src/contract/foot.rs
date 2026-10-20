//! The weaker foot in play. A kick goes to one side of the kicker's body: the side of its
//! direction against his facing, since the ball he carries sits straight ahead of him. A kick
//! to the side of his weaker foot, without time to move the ball onto his strong one, loses
//! rating points on the stages that set its error: more the weaker the foot, the closer the
//! nearest opponent and the longer the kick, never more than the block's cap (at most 5). With
//! the shared floor in the tuning, the cost on each stage shrinks to the points left above
//! that stage's floor, so the foot never takes a player below it ([`super::states::headroom`]).

use crate::data::team::{Foot, Side};
use crate::math::{self, DVec2};

use super::params::{ContractTuning, FootTuning};
use super::{Stage, StageValues, curve};

/// The side of the body a kick goes to, by the foot that plays it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KickSide {
    Strong,
    Weaker,
}

/// The side of a kick in direction `dir` by a player facing `facing` with feet `foot`. Within
/// `either_deg` of straight ahead either foot plays it, so it is his strong one; to his left
/// (`facing.perp_dot(dir) > 0`) a left-footer kicks with his strong foot, and so on. A player
/// with no facing kicks with his strong foot.
pub fn kick_side(facing: DVec2, dir: DVec2, foot: Foot, either_deg: f64) -> KickSide {
    let cross = facing.perp_dot(dir);
    let angle = math::atan2(cross, facing.dot(dir)).abs();
    if angle <= either_deg.to_radians() {
        return KickSide::Strong;
    }
    match (foot.strong, cross > 0.0) {
        (Side::Left, true) | (Side::Right, false) => KickSide::Strong,
        _ => KickSide::Weaker,
    }
}

/// The weaker foot's gap: 0 for a two-footed player (100), 0.99 for the most one-footed (1).
#[inline]
pub fn gap(foot: Foot) -> f64 {
    1.0 - f64::from(foot.weaker.min(100)) / 100.0
}

/// The cost of a kick, in rating points: 0 on the strong side or with time to shift the ball;
/// on the weaker side `cost_cap · gap · min(1, base + pressure_weight · pressure +
/// range_weight · range)`, where pressure rises from 0 with the nearest opponent at
/// `pressure_m` to 1 at 0 m, and range from 0 to 1 at `range_m`.
pub fn cost_points(
    foot: Foot,
    side: KickSide,
    has_time: bool,
    nearest_m: f64,
    distance_m: f64,
    t: &FootTuning,
) -> f64 {
    if side == KickSide::Strong || has_time || t.cost_cap <= 0.0 {
        return 0.0;
    }
    let pressure = (1.0 - nearest_m / t.pressure_m).clamp(0.0, 1.0);
    let range = (distance_m / t.range_m).clamp(0.0, 1.0);
    let share = (t.base + t.pressure_weight * pressure + t.range_weight * range).min(1.0);
    t.cost_cap * gap(foot) * share
}

/// `cost` shrunk to the rating points between a stage's value `f` and its floor, with `base_f`
/// its base value and `floor` the shared floor's drop; with no floor, `cost` unchanged.
#[inline]
pub fn shrunk(cost: f64, f: f64, base_f: f64, floor: Option<f64>, width: f64) -> f64 {
    match floor {
        Some(drop) => cost.min(super::states::headroom(f, base_f, drop, width)),
        None => cost,
    }
}

/// The execution share of a kick that costs `cost` rating points: the execute stage's share,
/// averaged with the pressure stage's when the kicker is pressed (`pressed` names that
/// stage), each read from its stage value lowered by the cost shrunk to its own floor. With
/// no cost, exactly the plain share or the pressed share.
pub fn kick_share(
    stages: &StageValues,
    base: &StageValues,
    execute: Stage,
    pressed: Option<Stage>,
    cost: f64,
    floor: Option<f64>,
    c: &ContractTuning,
) -> f64 {
    if cost <= 0.0 {
        return match pressed {
            Some(p) => stages.pressed_share(execute, p),
            None => stages.share(execute),
        };
    }
    let width = c.curve.width;
    let one = |s: Stage| {
        let f = stages.f(s);
        let d = shrunk(cost, f, base.f(s), floor, width);
        if d <= 0.0 {
            stages.share(s)
        } else {
            curve::share(
                c.actions.of(s.kind().0).k(),
                f * math::exp(-d / width),
                &c.curve,
            )
        }
    };
    match pressed {
        Some(p) => (one(execute) + one(p)) / 2.0,
        None => one(execute),
    }
}

/// The metres a closing presser moves toward the strong side of a carrier facing `facing`
/// with feet `foot`: `show_m · gap` along his strong side, 0 for a two-footed carrier.
pub fn show_offset(facing: DVec2, foot: Foot, show_m: f64) -> DVec2 {
    let left = facing.perp();
    let strong = match foot.strong {
        Side::Left => left,
        Side::Right => -left,
    };
    strong * (show_m * gap(foot))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RIGHT_20: Foot = Foot {
        strong: Side::Right,
        weaker: 20,
    };

    fn t() -> FootTuning {
        FootTuning::shipped()
    }

    #[test]
    fn the_side_of_a_kick_follows_its_direction_against_the_facing() {
        let ahead = DVec2::X;
        let left = DVec2::new(0.2, 1.0).normalize();
        let right = DVec2::new(0.2, -1.0).normalize();
        assert_eq!(kick_side(ahead, ahead, RIGHT_20, 20.0), KickSide::Strong);
        assert_eq!(kick_side(ahead, right, RIGHT_20, 20.0), KickSide::Strong);
        assert_eq!(kick_side(ahead, left, RIGHT_20, 20.0), KickSide::Weaker);
        let lefty = Foot {
            strong: Side::Left,
            weaker: 20,
        };
        assert_eq!(kick_side(ahead, left, lefty, 20.0), KickSide::Strong);
        assert_eq!(kick_side(ahead, right, lefty, 20.0), KickSide::Weaker);
        // Within the either-foot angle the strong foot plays.
        let slight = DVec2::new(1.0, 0.3).normalize();
        assert_eq!(kick_side(ahead, slight, RIGHT_20, 20.0), KickSide::Strong);
        assert_eq!(
            kick_side(DVec2::ZERO, left, RIGHT_20, 20.0),
            KickSide::Strong
        );
    }

    #[test]
    fn the_cost_is_zero_on_the_strong_side_with_time_or_two_footed_and_at_most_five() {
        let w = KickSide::Weaker;
        assert_eq!(
            cost_points(RIGHT_20, KickSide::Strong, false, 0.5, 40.0, &t()),
            0.0
        );
        assert_eq!(cost_points(RIGHT_20, w, true, 0.5, 40.0, &t()), 0.0);
        let two = Foot {
            strong: Side::Right,
            weaker: 100,
        };
        assert_eq!(cost_points(two, w, false, 0.5, 40.0, &t()), 0.0);
        let worst = Foot {
            strong: Side::Right,
            weaker: 1,
        };
        let top = cost_points(worst, w, false, 0.0, 80.0, &t());
        assert!(top > 4.9 && top <= 5.0, "{top}");
        let at_range = cost_points(RIGHT_20, w, false, 1.0, 35.0, &t());
        assert!(at_range > 0.0 && at_range <= 5.0, "{at_range}");
        assert_eq!(
            cost_points(RIGHT_20, w, false, 1.0, 35.0, &FootTuning::NONE),
            0.0
        );
    }

    #[test]
    fn the_cost_rises_with_pressure_and_with_range() {
        let w = KickSide::Weaker;
        assert!(
            cost_points(RIGHT_20, w, false, 1.0, 20.0, &t())
                > cost_points(RIGHT_20, w, false, 4.0, 20.0, &t())
        );
        assert!(
            cost_points(RIGHT_20, w, false, 3.0, 35.0, &t())
                > cost_points(RIGHT_20, w, false, 3.0, 10.0, &t())
        );
    }

    #[test]
    fn the_shrink_keeps_the_stage_at_or_above_its_floor() {
        let width = 8.0;
        let base_f = 21.75;
        for f in [21.75, 18.0, 14.0, 12.5, 10.0] {
            let d = shrunk(5.0, f, base_f, Some(7.0), width);
            let after = f * math::exp(-d / width);
            assert!(
                after >= base_f * math::exp(-7.0 / width) - 1e-9,
                "{f}: {after}"
            );
            assert!(d <= 5.0);
        }
        assert_eq!(shrunk(5.0, 12.0, base_f, None, width), 5.0);
    }

    #[test]
    fn with_no_cost_the_share_is_the_plain_or_pressed_share_and_a_cost_lowers_it() {
        let c = ContractTuning::default();
        let k = c.actions.of(super::super::ActionKind::Pass).k();
        let base = StageValues::flat(14.0, curve::share(k, 14.0, &c.curve));
        let mut stages = base;
        stages.share[Stage::PASS_PRESSURE.index()] = 0.6;
        let (e, p) = (Stage::PASS_EXECUTE, Some(Stage::PASS_PRESSURE));
        assert_eq!(
            kick_share(&stages, &base, e, None, 0.0, Some(7.0), &c),
            stages.share(e)
        );
        assert_eq!(
            kick_share(&stages, &base, e, p, 0.0, Some(7.0), &c),
            stages.pressed_share(e, Stage::PASS_PRESSURE)
        );
        let free = kick_share(&base, &base, e, p, 0.0, Some(7.0), &c);
        let paid = kick_share(&base, &base, e, p, 3.0, Some(7.0), &c);
        assert!(paid < free, "{paid} {free}");
        // 3 points on a stage at 14: the share of F(r - 3).
        let r = curve::f_inv(14.0, &c.curve);
        let want = curve::share(k, curve::f(r - 3.0, &c.curve), &c.curve);
        assert!((paid - want).abs() < 1e-12, "{paid} {want}");
    }

    #[test]
    fn a_presser_shows_toward_the_strong_side_in_proportion_to_the_gap() {
        let o = show_offset(DVec2::X, RIGHT_20, 1.5);
        assert!((o - DVec2::new(0.0, -1.2)).length() < 1e-12, "{o}");
        let lefty = Foot {
            strong: Side::Left,
            weaker: 20,
        };
        assert!(show_offset(DVec2::X, lefty, 1.5).y > 1.19);
        let two = Foot {
            strong: Side::Right,
            weaker: 100,
        };
        assert_eq!(show_offset(DVec2::X, two, 1.5).length(), 0.0);
    }
}
