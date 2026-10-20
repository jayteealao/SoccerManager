//! The open-play windows of both sides: when each side last won and lost the ball, when each
//! press trigger last fired, and when each side's offside trap last started. They are kept
//! together so the one rule they share holds in one place: a window reads open only while the
//! right side has the ball, and every dead ball closes them all, so a restart opens no window
//! and the snapshot never needs them.

/// The ticks the open-play windows start from, per side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Windows {
    /// Per side, the tick it last won the ball from the other side in open play (a gain that
    /// changed the team in possession). The regain window reads it while the side keeps the
    /// ball.
    pub(crate) regained_at: [Option<u32>; 2],
    /// Per side, the tick it last lost the ball to the other side in open play. The
    /// counter-press window reads it while the other side has the ball.
    pub(crate) lost_at: [Option<u32>; 2],
    /// Per pressing side, the tick each press trigger of the other side last fired in open
    /// play, in [`crate::decision::TRIGGERS`] order: a back pass, a poor touch, a receiver
    /// facing his own goal, a pass to a full-back or into a wide channel. Each fires whatever
    /// the plan; a side's press reads only the triggers its plan switches on.
    pub(crate) trigger_at: [[Option<u32>; 4]; 2],
    /// Per side, the tick its offside trap last started: an opposing carrier unpressed and
    /// facing forward while the side's plan sets the trap.
    pub(crate) trap_at: [Option<u32>; 2],
}

impl Windows {
    /// Side `team` won the ball from the other side at `tick`: its regain window and the other
    /// side's counter-press window open.
    pub(crate) fn won(&mut self, team: usize, tick: u32) {
        self.regained_at[team] = Some(tick);
        self.lost_at[1 - team] = Some(tick);
    }

    /// Press trigger `kind` fired for pressing side `def` at `tick`.
    pub(crate) fn fire_trigger(&mut self, def: usize, kind: usize, tick: u32) {
        self.trigger_at[def][kind] = Some(tick);
    }

    /// Side `def`'s offside trap started at `tick`.
    pub(crate) fn start_trap(&mut self, def: usize, tick: u32) {
        self.trap_at[def] = Some(tick);
    }

    /// Whether side `def`'s offside trap started at or after `tick`.
    pub(crate) fn trap_since(&self, def: usize, tick: u32) -> bool {
        self.trap_at[def].is_some_and(|at| at >= tick)
    }

    /// Seconds since side `team` won the ball, while `possession` (the side with the ball) is
    /// `team`.
    pub(crate) fn since_regain_s(
        &self,
        team: usize,
        possession: Option<usize>,
        tick: u32,
        dt: f64,
    ) -> Option<f64> {
        since(self.regained_at[team], possession == Some(team), tick, dt)
    }

    /// Seconds since side `team` lost the ball, while the other side has it.
    pub(crate) fn since_loss_s(
        &self,
        team: usize,
        possession: Option<usize>,
        tick: u32,
        dt: f64,
    ) -> Option<f64> {
        since(self.lost_at[team], possession == Some(1 - team), tick, dt)
    }

    /// Seconds since press trigger `kind` fired for pressing side `team`, while the other side
    /// has the ball.
    pub(crate) fn since_trigger_s(
        &self,
        team: usize,
        kind: usize,
        possession: Option<usize>,
        tick: u32,
        dt: f64,
    ) -> Option<f64> {
        since(
            self.trigger_at[team][kind],
            possession == Some(1 - team),
            tick,
            dt,
        )
    }

    /// Seconds since side `team`'s offside trap started, while the other side has the ball.
    pub(crate) fn since_trap_s(
        &self,
        team: usize,
        possession: Option<usize>,
        tick: u32,
        dt: f64,
    ) -> Option<f64> {
        since(self.trap_at[team], possession == Some(1 - team), tick, dt)
    }

    /// Closes every window: a dead ball ends them all.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Seconds from tick `at` to `tick` while the window is `open`; `None` when it is closed or
/// never started.
fn since(at: Option<u32>, open: bool, tick: u32, dt: f64) -> Option<f64> {
    let at = at.filter(|_| open)?;
    Some(f64::from(tick.saturating_sub(at)) * dt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_reads_only_while_the_right_side_has_the_ball_and_a_dead_ball_closes_all() {
        let mut w = Windows::default();
        w.won(0, 100);
        w.fire_trigger(0, 2, 120);
        w.start_trap(0, 130);
        assert_eq!(w.since_regain_s(0, Some(0), 150, 0.02), Some(1.0));
        assert_eq!(w.since_regain_s(0, Some(1), 150, 0.02), None);
        assert_eq!(w.since_loss_s(1, Some(0), 150, 0.02), Some(1.0));
        assert_eq!(w.since_loss_s(1, None, 150, 0.02), None);
        assert_eq!(w.since_trigger_s(0, 2, Some(1), 170, 0.02), Some(1.0));
        assert_eq!(w.since_trigger_s(0, 2, Some(0), 170, 0.02), None);
        assert_eq!(w.since_trap_s(0, Some(1), 180, 0.02), Some(1.0));
        assert!(w.trap_since(0, 130));
        assert!(!w.trap_since(0, 131));
        w.clear();
        assert_eq!(w, Windows::default());
    }
}
