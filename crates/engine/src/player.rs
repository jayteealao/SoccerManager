//! Player state and the built-in attribute set.
//!
//! The six attributes are a stopgap; the data-schemas slice replaces them with the
//! data-driven schema of 30 to 50 attributes on the 1 to 100 scale.

use crate::math::DVec2;
use crate::tuning::Tuning;

/// Attribute values on the 1 to 100 scale.
#[derive(Debug, Clone, Copy)]
pub struct Attributes {
    pub pace: u8,
    pub acceleration: u8,
    pub passing: u8,
    pub dribbling: u8,
    pub tackling: u8,
    pub positioning: u8,
}

impl Attributes {
    /// A uniform attribute set.
    pub const fn uniform(v: u8) -> Self {
        Self {
            pace: v,
            acceleration: v,
            passing: v,
            dribbling: v,
            tackling: v,
            positioning: v,
        }
    }
}

/// One player on the pitch.
#[derive(Debug, Clone, Copy)]
pub struct Player {
    /// Roster index, 0 to 21.
    pub id: usize,
    /// Team index, 0 (home) or 1 (away).
    pub team: usize,
    /// Slot index inside the formation, 0 to 10.
    pub slot: usize,
    pub shirt: u8,
    pub attributes: Attributes,
    pub pos: DVec2,
    pub vel: DVec2,
    /// Where steering drives the player this tick.
    pub target: DVec2,
    /// Unit vector of the last non-zero velocity.
    pub facing: DVec2,
}

impl Player {
    /// Maximum speed in metres per second.
    pub fn max_speed(&self, t: &Tuning) -> f64 {
        t.base_speed + t.pace_speed * f64::from(self.attributes.pace) / 100.0
    }

    /// Maximum acceleration in metres per second squared.
    pub fn max_accel(&self, t: &Tuning) -> f64 {
        t.base_accel + t.accel_bonus * f64::from(self.attributes.acceleration) / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_scale_speed() {
        let t = Tuning::default();
        let mut p = Player {
            id: 0,
            team: 0,
            slot: 0,
            shirt: 1,
            attributes: Attributes::uniform(0),
            pos: DVec2::ZERO,
            vel: DVec2::ZERO,
            target: DVec2::ZERO,
            facing: DVec2::X,
        };
        assert_eq!(p.max_speed(&t), t.base_speed);
        p.attributes.pace = 100;
        assert_eq!(p.max_speed(&t), t.base_speed + t.pace_speed);
    }
}
