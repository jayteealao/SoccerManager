//! The kinds of a forward's off-ball run, by the definitions the engine's runs measure and the
//! real-team table (`engine-cli signatures`, from tracking data) share.
//!
//! A run is a spell of [`RUN_MIN_S`] or more at [`RUN_SPEED`] or faster by a player of the side
//! with the ball in play who is not the carrier. Its kind reads where it began and where it
//! ended ([`RunEnds`]), as SkillCorner's run families read them:
//!
//! - **in behind**: the run gains depth and ends within [`BEHIND_M`] of the second-last
//!   defender or beyond him;
//! - **coming short**: the run begins ahead of the ball, gives up [`COME_SHORT_M`] or more of
//!   depth, and ends nearer the ball than it began.
//!
//! Depth is metres toward the goal the runner's side attacks.

/// The speed of a run, in metres per second.
pub const RUN_SPEED: f64 = 4.0;
/// The shortest run, in seconds.
pub const RUN_MIN_S: f64 = 1.0;
/// A run that ends within this many metres of the second-last defender, or beyond him, is in
/// behind.
pub const BEHIND_M: f64 = 2.0;
/// A run coming short gives up at least this much depth.
pub const COME_SHORT_M: f64 = 2.0;

/// Where a run began (`[0]`) and ended (`[1]`): the runner's depth, the second-last defender's
/// depth, the ball's depth, and the runner's distance to the ball.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RunEnds {
    pub depth: [f64; 2],
    pub line: [f64; 2],
    pub ball: [f64; 2],
    pub ball_m: [f64; 2],
}

impl RunEnds {
    /// `true` for a run in behind.
    pub fn in_behind(&self) -> bool {
        self.depth[1] > self.depth[0] && self.depth[1] >= self.line[1] - BEHIND_M
    }

    /// `true` for a run coming short.
    pub fn coming_short(&self) -> bool {
        self.depth[0] > self.ball[0]
            && self.depth[0] - self.depth[1] >= COME_SHORT_M
            && self.ball_m[1] < self.ball_m[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A run that gains depth and ends at or past the line is in behind; one that ends short of
    /// the line, or goes back, is not. A run that begins ahead of the ball and comes back
    /// toward it is coming short; one behind the ball, or one that drifts away, is not.
    #[test]
    fn the_run_kinds_read_the_shared_definitions() {
        let run = |d0, d1, l1, b0, m0, m1| RunEnds {
            depth: [d0, d1],
            line: [l1, l1],
            ball: [b0, b0],
            ball_m: [m0, m1],
        };
        assert!(run(10.0, 19.0, 20.0, 0.0, 10.0, 19.0).in_behind());
        assert!(
            run(10.0, 25.0, 20.0, 0.0, 10.0, 25.0).in_behind(),
            "past the line"
        );
        assert!(
            !run(10.0, 17.0, 20.0, 0.0, 10.0, 17.0).in_behind(),
            "short of the line"
        );
        assert!(
            !run(22.0, 19.0, 20.0, 0.0, 22.0, 19.0).in_behind(),
            "going back"
        );
        assert!(run(20.0, 14.0, 22.0, 0.0, 20.0, 14.0).coming_short());
        assert!(
            !run(20.0, 19.0, 22.0, 0.0, 20.0, 19.0).coming_short(),
            "too little"
        );
        assert!(
            !run(-5.0, -9.0, 22.0, 0.0, 5.0, 9.0).coming_short(),
            "behind the ball"
        );
        assert!(
            !run(20.0, 14.0, 22.0, 0.0, 20.0, 24.0).coming_short(),
            "away from the ball"
        );
    }
}
