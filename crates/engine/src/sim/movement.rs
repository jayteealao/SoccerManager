//! The movement pass of the central loop.

use crate::math::DVec2;
use crate::sim::Simulation;
use crate::steering::apply_velocities;
use crate::tuning::Tuning;

impl Simulation {
    /// The movement pass: the steering module sets every player's new velocity from the old
    /// state, then every player moves. The same two halves as [`step_all`].
    pub(crate) fn steer_players(&mut self, t: &Tuning) {
        let steering = self.config.modules.steering;
        let mut velocities = std::mem::take(&mut self.scratch);
        velocities.clear();
        let view = self.view();
        for i in 0..self.players.len() {
            velocities.push(if self.players[i].active() {
                steering.next_velocity(&view, i)
            } else {
                DVec2::ZERO
            });
        }
        apply_velocities(&mut self.players, &velocities, t);
        self.scratch = velocities;
    }
}
