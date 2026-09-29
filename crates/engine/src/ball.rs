//! The ball: position and velocity in three dimensions, ground friction, air drag, gravity,
//! and bounce. Integration is semi-implicit Euler at the fixed timestep.

use crate::math::{DVec2, DVec3, clamp_len};
use crate::modules::{BallModule, Crossing, Deflection, MatchView, ModuleCard};
use crate::pitch;
use crate::shot;
use crate::streams::Action;
use crate::tuning::Tuning;

/// Ball state.
#[derive(Debug, Clone, Copy)]
pub struct Ball {
    pub pos: DVec3,
    pub vel: DVec3,
}

impl Ball {
    /// A ball at rest at `p` on the ground.
    pub fn at(p: DVec2) -> Self {
        Self {
            pos: DVec3::new(p.x, p.y, 0.0),
            vel: DVec3::ZERO,
        }
    }

    /// Ground position.
    pub fn xy(&self) -> DVec2 {
        DVec2::new(self.pos.x, self.pos.y)
    }

    /// Ground speed plus vertical speed.
    pub fn speed(&self) -> f64 {
        self.vel.length()
    }

    /// `true` when the ball rests on or rolls along the ground.
    pub fn on_ground(&self) -> bool {
        self.pos.z <= 0.0 && self.vel.z <= 0.0
    }

    /// Kicks the ball: `dir` is a ground unit vector, `speed` the ground speed, `loft` the
    /// vertical speed. The total speed is capped at the tuning limit.
    pub fn kick(&mut self, dir: DVec2, speed: f64, loft: f64, t: &Tuning) {
        let mut v = DVec3::new(dir.x * speed, dir.y * speed, loft);
        let len = v.length();
        if len > t.ball_max_speed {
            v *= t.ball_max_speed / len;
        }
        self.vel = v;
        if loft > 0.0 && self.pos.z <= 0.0 {
            self.pos.z = 0.001;
        }
    }

    /// Advances the ball by one tick.
    pub fn integrate(&mut self, t: &Tuning) {
        if self.on_ground() {
            self.pos.z = 0.0;
            self.vel.z = 0.0;
            let ground = DVec2::new(self.vel.x, self.vel.y);
            let speed = ground.length();
            if speed > 0.0 {
                let new_speed = (speed - t.ground_friction * t.dt).max(0.0);
                let scaled = if new_speed < 0.05 {
                    DVec2::ZERO
                } else {
                    ground * (new_speed / speed)
                };
                self.vel.x = scaled.x;
                self.vel.y = scaled.y;
            }
        } else {
            self.vel.z -= t.gravity * t.dt;
            self.vel *= 1.0 - t.air_drag * t.dt;
        }
        self.pos += self.vel * t.dt;
        if self.pos.z < 0.0 {
            self.pos.z = 0.0;
            self.vel.z = -self.vel.z * t.restitution;
            if self.vel.z < 0.5 {
                self.vel.z = 0.0;
            }
        }
    }
}

/// Ball physics version 1: the ball carried at the carrier's feet, [`Ball::integrate`],
/// [`Ball::kick`], the goal-line and touchline crossings, and [`shot::deflect`].
pub struct BallV1;

impl BallModule for BallV1 {
    fn carry(&self, view: &MatchView<'_>, c: usize) -> Ball {
        let p = view.player(c);
        let ball = view.ball();
        let at = pitch::clamp(p.pos + p.facing * 0.5, 0.1);
        let step = clamp_len(at - ball.xy(), view.tuning().carry_step);
        let next = ball.xy() + step;
        Ball {
            pos: DVec3::new(next.x, next.y, 0.0),
            vel: DVec3::new(p.vel.x, p.vel.y, 0.0),
        }
    }

    fn integrate(&self, view: &MatchView<'_>, mut ball: Ball) -> Ball {
        ball.integrate(view.tuning());
        ball
    }

    fn kick(
        &self,
        view: &MatchView<'_>,
        mut ball: Ball,
        dir: DVec2,
        speed: f64,
        loft: f64,
    ) -> Ball {
        ball.kick(dir, speed, loft, view.tuning());
        ball
    }

    fn crossing(&self, view: &MatchView<'_>, prev: DVec2, ball: &Ball) -> Crossing {
        let xy = ball.xy();
        for team in 0..2 {
            if pitch::in_goal(prev, xy, view.attack_x(team))
                && ball.pos.z < view.tuning().crossbar_height
            {
                return Crossing::Goal(team);
            }
        }
        match pitch::exit(prev, xy) {
            Some(exit) => Crossing::Out(exit),
            None => Crossing::None,
        }
    }

    fn deflect(&self, view: &MatchView<'_>, how: Deflection, angle: f64, loft: f64) -> DVec3 {
        let t = view.tuning();
        let vel = view.ball().vel;
        match how {
            Deflection::Block => {
                let s = &t.shots;
                let back = -DVec2::new(vel.x, vel.y);
                shot::deflect(vel, back, s.block_speed, s.block_spread, 0.0, angle, 0.0)
            }
            Deflection::Parry { side } => {
                let s = &t.shots;
                shot::deflect(
                    vel,
                    DVec2::new(0.0, side),
                    s.parry_speed,
                    s.parry_spread,
                    s.parry_loft,
                    angle,
                    loft,
                )
            }
            Deflection::Clear { away, spread } => {
                let c = &t.clearances;
                shot::deflect(vel, away, c.cross_speed, spread, c.cross_loft, angle, loft)
            }
        }
    }
}

pub const BALL_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Moves the ball: carried at the carrier's feet, in flight and rolling with gravity, drag, friction, and bounce; kicks it; finds the goal-line and touchline crossings; and turns a blocked, parried, or cleared ball.",
    inputs: "The ball, the carrier's position, facing, and velocity, each team's attack direction, and the engine tuning.",
    outputs: "The next ball, a kicked ball, the line crossed, and a deflected ball's velocity.",
    tuning: &[
        "gravity",
        "air_drag",
        "ground_friction",
        "restitution",
        "ball_max_speed",
        "carry_step",
        "crossbar_height",
        "dt",
        "shots.block_speed",
        "shots.block_spread",
        "shots.parry_speed",
        "shots.parry_spread",
        "shots.parry_loft",
        "clearances.cross_speed",
        "clearances.cross_loft",
    ],
    calibration: "none: ball physics, no realism band",
    keys: &[
        Action::BlockDeflect,
        Action::ParryAngle,
        Action::ParryLoft,
        Action::CrossAngle,
        Action::CrossLoft,
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kicked_ball_stops() {
        let t = Tuning::default();
        let mut b = Ball::at(DVec2::ZERO);
        b.kick(DVec2::X, 10.0, 0.0, &t);
        for _ in 0..500 {
            b.integrate(&t);
        }
        assert_eq!(b.speed(), 0.0);
        assert!(b.pos.x > 10.0 && b.pos.x < 13.0, "rolled {}", b.pos.x);
    }

    #[test]
    fn a_lofted_ball_lands() {
        let t = Tuning::default();
        let mut b = Ball::at(DVec2::ZERO);
        b.kick(DVec2::X, 15.0, 8.0, &t);
        let mut max_z: f64 = 0.0;
        for _ in 0..500 {
            b.integrate(&t);
            max_z = max_z.max(b.pos.z);
        }
        assert!(max_z > 2.0);
        assert_eq!(b.pos.z, 0.0);
        assert!(b.on_ground());
    }

    #[test]
    fn speed_never_exceeds_the_cap() {
        let t = Tuning::default();
        let mut b = Ball::at(DVec2::ZERO);
        b.kick(DVec2::X, 100.0, 50.0, &t);
        assert!(b.speed() <= t.ball_max_speed + 1e-9);
    }
}
