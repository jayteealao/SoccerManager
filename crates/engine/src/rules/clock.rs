//! The match clock: the half structure from the rule pack, the time added at the end of each
//! half, and the minute a match event shows.
//!
//! Added time is the rule pack's seconds per stoppage of each kind and per card, plus a
//! variance of up to `variance_s` either way from the engine's seeded generator, clamped
//! between `min_s` and `max_s`. A match shorter than the rule pack's regulation length plays
//! no added time, so a short test match keeps a fixed tick count.

use crate::data::rules::{AddedTime, RulePack, StoppageKind};
use crate::{TICKS_PER_SECOND, ticks_for_minutes};

/// Ticks in one minute of play.
pub const TICKS_PER_MINUTE: u32 = 60 * TICKS_PER_SECOND;

/// Stoppages counted in the current half, by kind, and the cards shown in it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub kinds: [u32; StoppageKind::ALL.len()],
    pub cards: u32,
}

impl Tally {
    pub fn add(&mut self, kind: StoppageKind) {
        self.kinds[kind.index()] += 1;
    }

    /// The seconds the rule pack allows for the counted stoppages and cards.
    pub fn seconds(&self, added: &AddedTime) -> u32 {
        let stoppages: u32 = StoppageKind::ALL
            .iter()
            .map(|k| self.kinds[k.index()] * added.seconds(*k))
            .sum();
        stoppages + self.cards * added.card_s
    }
}

/// Seconds added to a half: the tally plus `(2 * draw - 1) * variance_s`, rounded to the
/// second and clamped between `min_s` and `max_s`. `draw` lies in `[0, 1)`.
pub fn added_seconds(tally: &Tally, added: &AddedTime, draw: f64) -> u32 {
    let base = f64::from(tally.seconds(added));
    let variance = (2.0 * draw - 1.0) * f64::from(added.variance_s);
    // The clamp keeps the value inside 0..=max_s, so the cast cannot truncate.
    (base + variance)
        .round()
        .clamp(f64::from(added.min_s), f64::from(added.max_s)) as u32
}

/// The added time the fourth official announces: whole minutes, rounded up.
pub fn announced_minutes(added_s: u32) -> u32 {
    added_s.div_ceil(60)
}

/// The most ticks a match of `minutes` can last: regulation plus the cap on added time in
/// every half, when the match plays added time.
pub fn max_ticks(minutes: u32, rules: &RulePack) -> u32 {
    let regulation = ticks_for_minutes(minutes);
    if plays_added_time(minutes, rules) {
        regulation + u32::from(rules.halves) * rules.added_time.max_s * TICKS_PER_SECOND
    } else {
        regulation
    }
}

/// `true` when a match of `minutes` is at least the rule pack's regulation length.
pub fn plays_added_time(minutes: u32, rules: &RulePack) -> bool {
    minutes >= rules.regulation_minutes()
}

/// Where a match is in its halves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchClock {
    pub halves: u32,
    /// The current half, from 0.
    pub half: u32,
    /// The tick the current half started after.
    pub half_start: u32,
    /// Regulation ticks in each half.
    pub half_ticks: u32,
    pub plays_added: bool,
    /// Added ticks per half, known once regulation time in that half ends.
    pub added_ticks: [Option<u32>; 2],
}

impl MatchClock {
    pub fn new(minutes: u32, rules: &RulePack) -> Self {
        let halves = u32::from(rules.halves);
        Self {
            halves,
            half: 0,
            half_start: 0,
            half_ticks: ticks_for_minutes(minutes) / halves,
            plays_added: plays_added_time(minutes, rules),
            added_ticks: [None; 2],
        }
    }

    /// `true` when regulation time in the current half is over at `tick`.
    pub fn regulation_over(&self, tick: u32) -> bool {
        tick.saturating_sub(self.half_start) >= self.half_ticks
    }

    /// The tick the current half ends on, once its added time is known.
    pub fn half_end(&self) -> Option<u32> {
        self.added_ticks[self.half as usize].map(|added| self.half_start + self.half_ticks + added)
    }

    /// `true` when the current half is the last one.
    pub fn last_half(&self) -> bool {
        self.half + 1 >= self.halves
    }

    /// Starts the next half after `tick`.
    pub fn next_half(&mut self, tick: u32) {
        self.half += 1;
        self.half_start = tick;
    }

    /// The minute an event at `tick` shows, counted from 0 like the tick, and the added
    /// minute when the tick lies in added time (1 for the first added minute).
    pub fn minute(&self, tick: u32) -> (u32, Option<u32>) {
        let elapsed = tick.saturating_sub(self.half_start);
        let before = self.half * self.half_ticks;
        if elapsed < self.half_ticks {
            ((before + elapsed) / TICKS_PER_MINUTE, None)
        } else {
            let over = elapsed - self.half_ticks;
            (
                (before + self.half_ticks) / TICKS_PER_MINUTE,
                Some(over / TICKS_PER_MINUTE + 1),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;

    #[test]
    fn added_time_is_the_tally_plus_the_variance_clamped() {
        let rules = shipped_content().rules;
        let added = &rules.added_time;
        let mut tally = Tally::default();
        for _ in 0..3 {
            tally.add(StoppageKind::Goal);
        }
        for _ in 0..10 {
            tally.add(StoppageKind::FreeKick);
        }
        tally.cards = 2;
        // 3 x 40 + 10 x 4 + 2 x 15 = 190 seconds.
        assert_eq!(tally.seconds(added), 190);
        assert_eq!(added_seconds(&tally, added, 0.5), 190);
        assert_eq!(added_seconds(&tally, added, 0.0), 160);
        assert_eq!(added_seconds(&tally, added, 0.75), 205);
        assert_eq!(added_seconds(&Tally::default(), added, 0.0), added.min_s);
        let mut busy = Tally::default();
        busy.kinds[StoppageKind::Injury.index()] = 40;
        assert_eq!(added_seconds(&busy, added, 0.9), added.max_s);
        assert_eq!(announced_minutes(190), 4);
        assert_eq!(announced_minutes(180), 3);
    }

    #[test]
    fn a_full_match_announces_regulation_plus_both_caps() {
        let rules = shipped_content().rules;
        assert_eq!(max_ticks(90, &rules), 270_000 + 2 * 900 * 50);
        assert_eq!(max_ticks(90, &rules), 360_000);
        assert_eq!(max_ticks(5, &rules), 15_000);
        assert!(!MatchClock::new(5, &rules).plays_added);
    }

    #[test]
    fn the_minute_counts_regulation_then_added_time() {
        let rules = shipped_content().rules;
        let mut clock = MatchClock::new(90, &rules);
        assert_eq!(clock.minute(9_000), (3, None));
        assert_eq!(clock.minute(135_000 + 4_000), (45, Some(2)));
        clock.added_ticks[0] = Some(9_000);
        clock.next_half(144_000);
        assert_eq!(clock.minute(144_000 + 3_000), (46, None));
        assert_eq!(clock.minute(144_000 + 135_000 + 1), (90, Some(1)));
    }
}
