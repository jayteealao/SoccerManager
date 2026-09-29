//! Shots: whether a shot's flight is on target, how good a chance it was, how likely a keeper
//! is to save it, and where a parried or blocked ball goes. Every function is pure: the
//! caller takes the random draws and passes them in.

use crate::ball::Ball;
use crate::math::{self, DVec2, DVec3};
use crate::modules::{MatchView, ModuleCard, ShotModule};
use crate::pitch;
use crate::sim::shot_xg;
use crate::streams::Action;
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
    let (sin, cos) = math::sin_cos(angle);
    let dir = away.normalize_or_zero();
    let dir = DVec2::new(dir.x * cos - dir.y * sin, dir.x * sin + dir.y * cos);
    let up = (draw_loft * loft).min(speed);
    let ground = (speed * speed - up * up).max(0.0).sqrt();
    DVec3::new(dir.x * ground, dir.y * ground, up)
}

/// Shot model version 1: the logistic expected-goals and quality models, the flight check,
/// and the linear save chance above.
pub struct ShotV1;

impl ShotModule for ShotV1 {
    fn xg(&self, view: &MatchView<'_>, from: DVec2, attack_x: f64) -> f64 {
        shot_xg(from, attack_x, &view.tuning().xg)
    }

    fn quality(&self, view: &MatchView<'_>, from: DVec2, attack_x: f64) -> f64 {
        quality(from, attack_x, view.tuning())
    }

    fn on_target(&self, view: &MatchView<'_>, ball: Ball, attack_x: f64) -> bool {
        on_target(ball, attack_x, view.tuning())
    }

    fn save_chance(&self, view: &MatchView<'_>, quality: f64) -> f64 {
        save_chance(quality, view.tuning())
    }
}

pub const SHOT_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Rates each shot: its expected goals, its quality, whether its flight goes in under the bar, and the keeper's save chance.",
    inputs: "The shot's position and attack direction, a copy of the kicked ball, and the engine tuning.",
    outputs: "The expected goals, the quality, the on-target result, and the save chance.",
    tuning: &[
        "xg.intercept",
        "xg.distance_coef",
        "xg.angle_coef",
        "shots.quality",
        "shots.save_high",
        "shots.save_low",
        "crossbar_height",
    ],
    calibration: "goals_per_xg",
    keys: &[Action::Save, Action::ShootoutSave],
};

/// The shot model switched off: every shot is worth nothing, never on target, and never
/// saved. The loop still takes every draw on its key.
pub struct ShotOff;

impl ShotModule for ShotOff {
    fn xg(&self, _: &MatchView<'_>, _: DVec2, _: f64) -> f64 {
        0.0
    }

    fn quality(&self, _: &MatchView<'_>, _: DVec2, _: f64) -> f64 {
        0.0
    }

    fn on_target(&self, _: &MatchView<'_>, _: Ball, _: f64) -> bool {
        false
    }

    fn save_chance(&self, _: &MatchView<'_>, _: f64) -> f64 {
        0.0
    }
}

pub const SHOT_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "The shot model switched off: shots carry no expected goals or quality, none is on target, and none is saved.",
    inputs: "Nothing.",
    outputs: "Expected goals 0, quality 0, never on target, and a save chance of 0.",
    tuning: &["none"],
    calibration: "none: off version, no shot model",
    keys: &[Action::Save, Action::ShootoutSave],
};

/// Test only: version 1 with every shot's quality raised by 0.125 from tick 30,000. The
/// gate tests select it to prove that one changed output fails the gate.
#[cfg(feature = "scenario")]
pub struct ShotFaulty;

#[cfg(feature = "scenario")]
impl ShotModule for ShotFaulty {
    fn xg(&self, view: &MatchView<'_>, from: DVec2, attack_x: f64) -> f64 {
        ShotV1.xg(view, from, attack_x)
    }

    fn quality(&self, view: &MatchView<'_>, from: DVec2, attack_x: f64) -> f64 {
        let q = ShotV1.quality(view, from, attack_x);
        if view.tick() >= 30_000 { q + 0.125 } else { q }
    }

    fn on_target(&self, view: &MatchView<'_>, ball: Ball, attack_x: f64) -> bool {
        ShotV1.on_target(view, ball, attack_x)
    }

    fn save_chance(&self, view: &MatchView<'_>, quality: f64) -> f64 {
        ShotV1.save_chance(view, quality)
    }
}

#[cfg(feature = "scenario")]
pub const SHOT_FAULTY_CARD: ModuleCard = ModuleCard {
    purpose: "Test only: shot model version 1 with every shot's quality raised by 0.125 from tick 30,000.",
    inputs: "As shot model version 1, plus the tick.",
    outputs: "As shot model version 1.",
    tuning: &["none"],
    calibration: "none: test module for the replay gate",
    keys: &[Action::Save, Action::ShootoutSave],
};

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
