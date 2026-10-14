//! The transition counts of one side: what it does in the seconds after it loses the ball
//! and after it wins it, on the possession count of the primitives (the module header of
//! [`crate::measures`]). The signature command reads the real data by the same definitions:
//!
//! - an open-play loss is the end of a side's possession by the other side's touch, with no
//!   dead ball between (a short regain is no loss); its time is the other side's first touch;
//! - a regain is the start of the side's next possession in open play, at its first touch;
//! - a pressure is an opponent's first press of the carrier in that carrier's spell on the
//!   ball: his own share of the carrier's pressure intensity passes the pressure model's
//!   threshold ([`crate::pressure::presses`]);
//! - a fast break is a possession that starts with a regain at or behind
//!   [`crate::decision::COUNTER_START_M`] and reaches the attacked final third within
//!   [`crate::decision::COUNTER_WINDOW_S`];
//! - the counter-press figures read the losing side's pressures within
//!   [`crate::decision::COUNTERPRESS_S`] of a loss, whether it regains the ball within that
//!   time, and its last pressure within [`crate::decision::COUNTERPRESS_LAST_S`].

use crate::TICKS_PER_SECOND;

/// `s` seconds in ticks.
pub(crate) fn ticks(s: f64) -> u32 {
    // The constants are a few seconds: the cast is exact.
    (s * f64::from(TICKS_PER_SECOND)).round() as u32
}

/// `t` ticks in seconds.
pub(crate) fn seconds(t: u32) -> f64 {
    f64::from(t) / f64::from(TICKS_PER_SECOND)
}

/// The transition counts of one side: every field is a count, or a sum of seconds for an
/// exact mean.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Transitions {
    /// Open-play losses, and by the third the ball was in when the other side touched it,
    /// counted from the losing side's own goal line.
    pub losses: u32,
    pub losses_by_third: [u32; 3],
    /// Possessions that started with an open-play regain.
    pub regains: u32,
    /// Those that were fast breaks.
    pub counter_attacks: u32,
    /// Regain possessions with a shot, and the sum of the seconds from each one's start to its
    /// first shot.
    pub regain_shots: u32,
    pub regain_shot_s: f64,
    /// Pressures of the side within [`crate::decision::COUNTERPRESS_S`] of its losses.
    pub counterpress_pressures: u32,
    /// Losses after which the side won the ball back within that time.
    pub counterpress_regains: u32,
    /// Losses with a pressure of the side within [`crate::decision::COUNTERPRESS_LAST_S`],
    /// and the sum of the seconds from each loss to the last such pressure.
    pub last_counterpress: u32,
    pub last_counterpress_s: f64,
    /// Every pressure of the side.
    pub pressures: u32,
}

/// An open-play loss in progress: tentative until the other side's touch becomes its
/// possession, then confirmed; counted when the side wins the ball back or the ball goes
/// dead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Loss {
    pub at: u32,
    pub third: usize,
    pub confirmed: bool,
    pub pressures: u32,
    pub last_pressure: Option<u32>,
    pub regained_at: Option<u32>,
}

impl Loss {
    pub(crate) fn new(at: u32, third: usize) -> Self {
        Self {
            at,
            third,
            confirmed: false,
            pressures: 0,
            last_pressure: None,
            regained_at: None,
        }
    }

    /// `n` pressures of the losing side at `tick`.
    pub(crate) fn pressed(&mut self, n: u32, tick: u32) {
        let since = tick.saturating_sub(self.at);
        if since <= ticks(crate::decision::COUNTERPRESS_S) {
            self.pressures += n;
        }
        if since <= ticks(crate::decision::COUNTERPRESS_LAST_S) {
            self.last_pressure = Some(tick);
        }
    }

    /// Adds a confirmed loss to the side's counts.
    pub(crate) fn count(self, t: &mut Transitions) {
        if !self.confirmed {
            return;
        }
        t.losses += 1;
        t.losses_by_third[self.third.min(2)] += 1;
        t.counterpress_pressures += self.pressures;
        let window = ticks(crate::decision::COUNTERPRESS_S);
        if self
            .regained_at
            .is_some_and(|r| r.saturating_sub(self.at) <= window)
        {
            t.counterpress_regains += 1;
        }
        if let Some(last) = self.last_pressure {
            t.last_counterpress += 1;
            t.last_counterpress_s += seconds(last.saturating_sub(self.at));
        }
    }
}

/// What a possession of the primitives' count adds to the transition counts when it ends:
/// `start` its first touch, `regain` whether it started with an open-play regain, `start_m`
/// the ball's depth past halfway toward the attacked goal then, `final_at` the tick it first
/// reached the attacked final third, `shot_at` its first shot.
pub(crate) fn possession_ended(
    t: &mut Transitions,
    start: u32,
    regain: bool,
    start_m: f64,
    final_at: Option<u32>,
    shot_at: Option<u32>,
) {
    if !regain {
        return;
    }
    t.regains += 1;
    let window = ticks(crate::decision::COUNTER_WINDOW_S);
    if start_m <= crate::decision::COUNTER_START_M
        && final_at.is_some_and(|f| f.saturating_sub(start) <= window)
    {
        t.counter_attacks += 1;
    }
    if let Some(s) = shot_at {
        t.regain_shots += 1;
        t.regain_shot_s += seconds(s.saturating_sub(start));
    }
}
