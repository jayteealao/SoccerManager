//! The out-of-possession counts of one side: its defensive actions and their height, the
//! passes it plays in its own build-up zone (the other side's PPDA numerator), the press
//! triggers of the other side and the pressures inside their windows, and its offside traps.
//!
//! The definitions follow the real-team table (`signatures.json`), which reads the Wyscout
//! events of the four fit leagues: a defensive action is a tackle (the first attempt of a
//! defender on one carrier's spell, the engine trying again every tick), an interception or a
//! foul; its height is the ball's distance from the side's own goal line in metres; a PPDA
//! action lies beyond
//! [`PPDA_ACTION_SHARE`] of the length from the own goal line, and a build-up pass starts
//! within [`PPDA_BUILD_SHARE`] of it. A defender's action is one by a player whose slot is a
//! centre-back or a full-back.

use crate::TICKS_PER_SECOND;

/// The share of the pitch length from the own goal line beyond which a defensive action
/// counts for PPDA (Wyscout x above 40).
pub const PPDA_ACTION_SHARE: f64 = 0.4;
/// The share within which a pass starts in its side's build-up zone (Wyscout x below 60).
pub const PPDA_BUILD_SHARE: f64 = 0.6;

/// One side's out-of-possession counts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Defending {
    /// Defensive actions, and the sum of their heights in metres.
    pub actions: u32,
    pub action_m: f64,
    /// Those by defenders, and the sum of their heights.
    pub defender_actions: u32,
    pub defender_action_m: f64,
    /// Defensive actions beyond [`PPDA_ACTION_SHARE`].
    pub ppda_actions: u32,
    /// The side's own open-play passes that start within [`PPDA_BUILD_SHARE`].
    pub build_passes: u32,
    /// Press triggers of the other side, by kind in [`crate::decision::TRIGGERS`] order, and
    /// the side's pressures inside each kind's window.
    pub trigger_events: [u32; 4],
    pub trigger_pressures: [u32; 4],
    /// Offside traps started.
    pub traps: u32,
}

impl Defending {
    /// A defensive action at `share` of the pitch length, `m` metres, from the side's own goal
    /// line, by a defender when `defender`.
    pub fn action(&mut self, share: f64, m: f64, defender: bool) {
        self.actions += 1;
        self.action_m += m;
        if defender {
            self.defender_actions += 1;
            self.defender_action_m += m;
        }
        self.ppda_actions += u32::from(share > PPDA_ACTION_SHARE);
    }
}

/// [`crate::decision::TRIGGER_WINDOW_S`] in ticks.
pub fn trigger_window_ticks() -> u32 {
    // The window is a few seconds: the cast is exact.
    (crate::decision::TRIGGER_WINDOW_S * f64::from(TICKS_PER_SECOND)).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_action_counts_its_height_and_the_ppda_zone() {
        let mut d = Defending::default();
        d.action(0.3, 30.0, true);
        d.action(0.5, 50.0, false);
        assert_eq!((d.actions, d.defender_actions, d.ppda_actions), (2, 1, 1));
        assert!((d.action_m - 80.0).abs() < 1e-9);
        assert!((d.defender_action_m - 30.0).abs() < 1e-9);
    }
}
