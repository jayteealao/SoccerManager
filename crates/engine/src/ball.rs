//! The ball: position and velocity in three dimensions, ground friction, air drag, gravity,
//! and bounce. Integration is semi-implicit Euler at the fixed timestep.

use crate::math::{DVec2, DVec3};
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
