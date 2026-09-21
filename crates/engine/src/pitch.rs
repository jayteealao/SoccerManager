//! The pitch: 105 m by 68 m, centred at the origin. `x` runs along the length.

use crate::math::DVec2;

/// Pitch length in metres.
pub const LENGTH: f64 = 105.0;
/// Pitch width in metres.
pub const WIDTH: f64 = 68.0;
/// Goal width in metres.
pub const GOAL_WIDTH: f64 = 7.32;

/// Half the pitch length.
pub const HALF_LENGTH: f64 = LENGTH / 2.0;
/// Half the pitch width.
pub const HALF_WIDTH: f64 = WIDTH / 2.0;

/// `true` when `p` lies inside the pitch (touchlines and goal lines included).
pub fn contains(p: DVec2) -> bool {
    p.x.abs() <= HALF_LENGTH && p.y.abs() <= HALF_WIDTH
}

/// Clamps `p` inside the pitch, `margin` metres away from every line.
pub fn clamp(p: DVec2, margin: f64) -> DVec2 {
    DVec2::new(
        p.x.clamp(-HALF_LENGTH + margin, HALF_LENGTH - margin),
        p.y.clamp(-HALF_WIDTH + margin, HALF_WIDTH - margin),
    )
}

/// The goal centre for a team that attacks in direction `attack_x` (+1 or -1).
pub fn goal_centre(attack_x: f64) -> DVec2 {
    DVec2::new(HALF_LENGTH * attack_x, 0.0)
}

/// `true` when a ball at `p` has crossed the goal line between the posts on the `attack_x` side.
pub fn in_goal(p: DVec2, attack_x: f64) -> bool {
    p.x * attack_x > HALF_LENGTH && p.y.abs() < GOAL_WIDTH / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_and_clamp() {
        assert!(contains(DVec2::new(52.5, 34.0)));
        assert!(!contains(DVec2::new(52.6, 0.0)));
        assert_eq!(
            clamp(DVec2::new(100.0, -100.0), 0.5),
            DVec2::new(52.0, -33.5)
        );
    }

    #[test]
    fn goal_detection_uses_the_posts() {
        assert!(in_goal(DVec2::new(52.6, 3.0), 1.0));
        assert!(!in_goal(DVec2::new(52.6, 4.0), 1.0));
        assert!(!in_goal(DVec2::new(-52.6, 0.0), 1.0));
    }
}
