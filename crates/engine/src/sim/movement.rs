//! The movement pass of the central loop.

use crate::sim::Simulation;
use crate::steering::apply_velocities;
use crate::tuning::Tuning;

impl Simulation {
    /// The movement pass: the steering module sets every player's new velocity from the old
    /// state, then every player moves.
    pub(crate) fn steer_players(&mut self, t: &Tuning) {
        let steering = self.config.modules.steering;
        let mut velocities = std::mem::take(&mut self.scratch);
        steering.next_velocities(&self.view(), &mut velocities);
        apply_velocities(&mut self.players, &velocities, t, self.config.pitch());
        self.scratch = velocities;
    }
}
