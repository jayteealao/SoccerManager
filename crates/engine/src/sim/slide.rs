//! The slide pass of the central loop: before the decisions of each open-play tick, every
//! outfield player's followed ball point eases toward the ball, later the farther he is from
//! it ([`crate::team::follow`]); his anchor's ball shift reads that point, so the block slides
//! with the ball and its far players follow late. Every dead ball and each kick-off placement
//! set every point to the ball, so a restart starts settled.

use crate::sim::Simulation;

impl Simulation {
    /// One open-play tick of every followed ball point. A keeper's point and an inactive
    /// player's point are the ball.
    pub(crate) fn slide_tick(&mut self) {
        let ball = self.ball.xy();
        let t = &self.config.tuning;
        let (k, dt) = (t.far_delay_s_per_m, t.dt);
        let keepers = [self.keeper(0), self.keeper(1)];
        for (i, p) in self.players.iter().enumerate() {
            self.slide_ball[i] = if p.active() && i != keepers[p.team] {
                crate::team::follow(self.slide_ball[i], ball, (p.pos - ball).length(), k, dt)
            } else {
                ball
            };
        }
    }

    /// Every followed ball point is the ball: at a dead ball and at a kick-off placement.
    pub(crate) fn slide_reset(&mut self) {
        let ball = self.ball.xy();
        self.slide_ball = [ball; crate::modules::ROSTER];
    }
}
