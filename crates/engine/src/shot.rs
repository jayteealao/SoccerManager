//! Shots: whether a shot's flight is on target, how good a chance it was, how likely a keeper
//! is to save it, and where a parried or blocked ball goes. Every function is pure: the
//! caller takes the random draws and passes them in.

use crate::ball::Ball;
use crate::math::{DVec2, DVec3};
use crate::pitch;
use crate::sim::shot_xg;
use crate::tuning::Tuning;

/// The most ticks the flight of a shot is followed: five seconds.
const FLIGHT_TICKS: u32 = 250;

/// Shot quality at which the save chance starts to fall, and at which it stops falling.
const QUALITY_LOW: f64 = 0.05;
const QUALITY_HIGH: f64 = 0.40;

/// `true` when `ball`, left alone, crosses the goal line of a team attacking `attack_x`
/// between the posts and under the bar. A copy of the ball is moved with the same physics
/// as the match, so the prediction and the flight agree; it stops at rest, when the ball
/// leaves play anywhere else, or after `FLIGHT_TICKS`.
pub fn on_target(mut ball: Ball, attack_x: f64, t: &Tuning) -> bool {
    for _ in 0..FLIGHT_TICKS {
        let prev = ball.xy();
        ball.integrate(t);
        let xy = ball.xy();
        if pitch::in_goal(prev, xy, attack_x) {
            return ball.pos.z < t.crossbar_height;
        }
        if pitch::exit(prev, xy).is_some() || ball.speed() == 0.0 {
            return false;
        }
    }
    false
}

/// The quality of a shot from `from` at the goal a team attacking `attack_x` aims at: the
/// expected goals of the fixed quality model.
pub fn quality(from: DVec2, attack_x: f64, t: &Tuning) -> f64 {
    shot_xg(from, attack_x, &t.shots.quality)
}

/// The chance that a keeper saves a shot of `quality` heading on target: `save_high` up to
/// quality 0.05, `save_low` from 0.40, and a straight line between.
pub fn save_chance(quality: f64, t: &Tuning) -> f64 {
    let s = &t.shots;
    let k = ((quality - QUALITY_LOW) / (QUALITY_HIGH - QUALITY_LOW)).clamp(0.0, 1.0);
    s.save_high + (s.save_low - s.save_high) * k
}

/// The velocity of a ball moving at `vel` after it is deflected toward `away`: turned by up
/// to `spread` radians either side (`draw_angle` in `[0, 1)` picks the angle), with
/// `speed_share` of its speed, and a vertical speed of up to `loft` (`draw_loft` in `[0, 1)`).
pub fn deflect(
    vel: DVec3,
    away: DVec2,
    speed_share: f64,
    spread: f64,
    loft: f64,
    draw_angle: f64,
    draw_loft: f64,
) -> DVec3 {
    let speed = speed_share * vel.length();
    let angle = (2.0 * draw_angle - 1.0) * spread;
    let (sin, cos) = angle.sin_cos();
    let dir = away.normalize_or_zero();
    let dir = DVec2::new(dir.x * cos - dir.y * sin, dir.x * sin + dir.y * cos);
    let up = (draw_loft * loft).min(speed);
    let ground = (speed * speed - up * up).max(0.0).sqrt();
    DVec3::new(dir.x * ground, dir.y * ground, up)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ball kicked from `from` at the goal at `x = 52.5` toward `aim` at 27 m/s.
    fn shot(from: DVec2, aim: DVec2, loft: f64, t: &Tuning) -> Ball {
        let mut ball = Ball::at(from);
        ball.kick((aim - from).normalize(), 27.0, loft, t);
        ball
    }

    #[test]
    fn a_shot_at_the_centre_from_sixteen_metres_is_on_target() {
        let t = Tuning::default();
        let ball = shot(DVec2::new(36.5, 0.0), DVec2::new(52.5, 0.0), 0.0, &t);
        assert!(on_target(ball, 1.0, &t));
    }

    #[test]
    fn a_shot_aimed_a_metre_outside_a_post_is_not_on_target() {
        let t = Tuning::default();
        let post = pitch::GOAL_WIDTH / 2.0;
        let ball = shot(DVec2::new(36.5, 0.0), DVec2::new(52.5, post + 1.0), 0.0, &t);
        assert!(!on_target(ball, 1.0, &t));
    }

    #[test]
    fn a_shot_lofted_at_nine_metres_per_second_from_eighteen_metres_goes_over_the_bar() {
        let t = Tuning::default();
        let ball = shot(DVec2::new(34.5, 0.0), DVec2::new(52.5, 0.0), 9.0, &t);
        assert!(!on_target(ball, 1.0, &t));
    }

    #[test]
    fn a_shot_at_the_other_goal_is_not_on_target() {
        let t = Tuning::default();
        let ball = shot(DVec2::new(36.5, 0.0), DVec2::new(52.5, 0.0), 0.0, &t);
        assert!(!on_target(ball, -1.0, &t));
    }

    #[test]
    fn the_save_chance_falls_from_the_poorest_chances_to_the_best() {
        let t = Tuning::default();
        assert!((save_chance(0.02, &t) - 0.891).abs() < 1e-12);
        assert!((save_chance(0.6, &t) - 0.272).abs() < 1e-12);
        let mut last = save_chance(0.0, &t);
        for k in 1..=100 {
            let now = save_chance(f64::from(k) / 100.0, &t);
            assert!(now <= last, "quality {k}: {now} > {last}");
            last = now;
        }
    }

    #[test]
    fn a_deflection_keeps_its_share_of_the_speed() {
        let vel = DVec3::new(20.0, 5.0, 1.0);
        for (a, l) in [(0.0, 0.0), (0.3, 0.9), (0.99, 0.5)] {
            let out = deflect(vel, DVec2::new(-1.0, 0.0), 0.4, 1.2, 4.0, a, l);
            assert!((out.length() - 0.4 * vel.length()).abs() < 1e-9, "{out:?}");
            assert!(out.z >= 0.0 && out.z <= 4.0);
        }
        // With no spread the ball goes straight toward `away`.
        let out = deflect(vel, DVec2::new(0.0, 1.0), 0.5, 0.0, 0.0, 0.7, 0.0);
        assert!(out.x.abs() < 1e-9 && out.y > 0.0, "{out:?}");
    }
}
