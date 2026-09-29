//! The match clock: the periods from the rule pack (the regulation halves, then the
//! extra-time periods of a knockout match), the time added at the end of each period, and
//! the minute a match event shows.
//!
//! Added time is the rule pack's seconds per stoppage of each kind and per card, plus a
//! variance of up to `variance_s` either way from the engine's seeded generator, clamped
//! between `min_s` and `max_s` (`extra_time.added_max_s` in extra time). A match shorter than
//! the rule pack's regulation length plays no added time and no extra time, so a short test
//! match keeps a fixed tick count.

use crate::data::rules::{AddedTime, RulePack, StoppageKind};
use crate::modules::{ClockModule, MatchView, ModuleCard, PeriodEnd, ShootoutLineup};
use crate::rules::{discipline, shootout};
use crate::sim::DecidedBy;
use crate::streams::Action;
use crate::{TICKS_PER_SECOND, ticks_for_minutes};

/// Ticks in one minute of play.
pub const TICKS_PER_MINUTE: u32 = 60 * TICKS_PER_SECOND;

/// Ticks a shoot-out kick may stay live before it counts as missed (5 s).
pub const KICK_LIVE_TICKS: u32 = 5 * TICKS_PER_SECOND;

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

/// The added-time rules of an extra-time period: the regulation pricing with the cap
/// lowered to `extra_time.added_max_s`, and the floor no higher than that cap.
pub fn extra_time_allowance(rules: &RulePack) -> AddedTime {
    let mut added = rules.added_time.clone();
    added.max_s = rules.extra_time.added_max_s;
    added.min_s = added.min_s.min(added.max_s);
    added
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

/// The most ticks extra time can add to a knockout match of `minutes`: every period at its
/// full length plus its cap on added time. A match shorter than regulation plays none.
pub fn knockout_extra_ticks(minutes: u32, rules: &RulePack) -> u32 {
    if !plays_added_time(minutes, rules) {
        return 0;
    }
    let extra = &rules.extra_time;
    u32::from(extra.periods)
        * (u32::from(extra.period_minutes) * 60 + extra.added_max_s)
        * TICKS_PER_SECOND
}

/// The ticks the announced maximum allows for the shoot-out: the rule pack's allowance of
/// rounds, two kicks a round, and each kick at most its dead-ball hard limit (three times
/// `penalty_delay_ticks`) plus `KICK_LIVE_TICKS` of live ball.
pub fn shootout_allowance_ticks(rules: &RulePack, penalty_delay_ticks: u32) -> u32 {
    u32::from(rules.shootout.allowance_rounds) * 2 * (3 * penalty_delay_ticks + KICK_LIVE_TICKS)
}

/// Where a match is in its periods: the regulation halves, then the extra-time periods of a
/// knockout match. The fields keep the half names of a regulation match; `half` counts every
/// period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchClock {
    /// Regulation halves.
    pub halves: u32,
    /// The current period, from 0; the extra-time periods follow the halves.
    pub half: u32,
    /// The tick the current period started after.
    pub half_start: u32,
    /// Regulation ticks in each half.
    pub half_ticks: u32,
    /// Extra-time periods a level match plays: 0 unless the match is a knockout match that
    /// plays added time.
    pub extra_periods: u32,
    /// Ticks in each extra-time period.
    pub extra_ticks: u32,
    pub plays_added: bool,
    /// Added ticks per period, known once the period's own time ends.
    pub added_ticks: [Option<u32>; 4],
}

impl MatchClock {
    /// The clock of a match of `minutes`; a `knockout` match that plays added time also has
    /// the rule pack's extra-time periods.
    pub fn new(minutes: u32, rules: &RulePack, knockout: bool) -> Self {
        let halves = u32::from(rules.halves);
        let plays_added = plays_added_time(minutes, rules);
        let extra = &rules.extra_time;
        Self {
            halves,
            half: 0,
            half_start: 0,
            half_ticks: ticks_for_minutes(minutes) / halves,
            extra_periods: if knockout && plays_added {
                u32::from(extra.periods)
            } else {
                0
            },
            extra_ticks: ticks_for_minutes(u32::from(extra.period_minutes)),
            plays_added,
            added_ticks: [None; 4],
        }
    }

    /// Ticks of play in period `period`, added time excluded.
    pub fn period_ticks(&self, period: u32) -> u32 {
        if period < self.halves {
            self.half_ticks
        } else {
            self.extra_ticks
        }
    }

    /// `true` when the current period is an extra-time period.
    pub fn in_extra_time(&self) -> bool {
        self.half >= self.halves
    }

    /// `true` when no period follows the current one, extra time included.
    pub fn last_period(&self) -> bool {
        self.half + 1 >= self.halves + self.extra_periods
    }

    /// `true` when the current period's own time is over at `tick`.
    pub fn regulation_over(&self, tick: u32) -> bool {
        tick.saturating_sub(self.half_start) >= self.period_ticks(self.half)
    }

    /// The tick the current period ends on, once its added time is known.
    pub fn half_end(&self) -> Option<u32> {
        self.added_ticks[self.half as usize]
            .map(|added| self.half_start + self.period_ticks(self.half) + added)
    }

    /// `true` when the current period is the last regulation half or a later period.
    pub fn last_half(&self) -> bool {
        self.half + 1 >= self.halves
    }

    /// Starts the next period after `tick`.
    pub fn next_half(&mut self, tick: u32) {
        self.half += 1;
        self.half_start = tick;
    }

    /// Ticks of play in every period before the current one, added time excluded.
    fn before(&self) -> u32 {
        (0..self.half).map(|p| self.period_ticks(p)).sum()
    }

    /// The minute the clock stops at when the current period ends: 90 after regulation, 120
    /// after two periods of 15 minutes of extra time.
    pub fn end_minute(&self) -> u32 {
        (self.before() + self.period_ticks(self.half)) / TICKS_PER_MINUTE
    }

    /// The minute an event at `tick` shows, counted from 0 like the tick, and the added
    /// minute when the tick lies in added time (1 for the first added minute).
    pub fn minute(&self, tick: u32) -> (u32, Option<u32>) {
        let elapsed = tick.saturating_sub(self.half_start);
        let before = self.before();
        let length = self.period_ticks(self.half);
        if elapsed < length {
            ((before + elapsed) / TICKS_PER_MINUTE, None)
        } else {
            let over = elapsed - length;
            (
                (before + length) / TICKS_PER_MINUTE,
                Some(over / TICKS_PER_MINUTE + 1),
            )
        }
    }
}

/// The clock and match end, version 1: the rule pack's added time, the period sequence of
/// the Laws (half-time, extra time for a level knockout match, then the shoot-out), the
/// rule pack's minimum players, and the shoot-out law in `shootout`.
pub struct ClockV1;

impl ClockModule for ClockV1 {
    fn tally_seconds(&self, view: &MatchView<'_>, extra: bool) -> u32 {
        let tally = &view.referee().tally;
        if extra {
            tally.seconds(&extra_time_allowance(view.rules()))
        } else {
            tally.seconds(&view.rules().added_time)
        }
    }

    fn added_seconds(&self, view: &MatchView<'_>, extra: bool, draw: f64) -> u32 {
        let tally = &view.referee().tally;
        if extra {
            added_seconds(tally, &extra_time_allowance(view.rules()), draw)
        } else {
            added_seconds(tally, &view.rules().added_time, draw)
        }
    }

    fn period_end(&self, view: &MatchView<'_>) -> PeriodEnd {
        let clock = view.referee().clock;
        let goals = view.goals();
        let level = goals[0] == goals[1];
        if !clock.last_half() {
            PeriodEnd::Break { recover: true }
        } else if !clock.last_period() && (clock.in_extra_time() || level) {
            // Extra-time periods exist only in a knockout match. Its breaks give no energy
            // back, so fatigue runs on past 90 minutes without a step.
            PeriodEnd::Break { recover: false }
        } else if view.knockout() && level {
            PeriodEnd::Shootout
        } else {
            PeriodEnd::FullTime {
                decided_by: view.knockout().then_some(if clock.in_extra_time() {
                    DecidedBy::ExtraTime
                } else {
                    DecidedBy::Regulation
                }),
            }
        }
    }

    fn extra_kick_off(&self, draw: f64) -> usize {
        usize::from(draw >= 0.5)
    }

    fn abandoned(&self, view: &MatchView<'_>) -> Option<usize> {
        discipline::abandoned(view.players(), view.rules().min_players)
    }

    fn shootout_lineup(&self, view: &MatchView<'_>) -> ShootoutLineup {
        let attributes = view.attributes();
        let keeping = [
            attributes.index("reflexes"),
            attributes.index("one_on_ones"),
        ];
        let acting = [view.keeper(0), view.keeper(1)];
        let candidates = [0, 1].map(|team| {
            view.players()
                .iter()
                .enumerate()
                .filter(|(_, p)| p.team == team && p.active())
                .map(|(i, p)| shootout::Candidate {
                    index: i,
                    goalkeeper: i == acting[team],
                    kicking: p.derived.finishing + p.derived.composure,
                    keeping: keeping
                        .iter()
                        .flatten()
                        .map(|&k| f64::from(p.attributes.get(k)))
                        .sum(),
                })
                .collect::<Vec<_>>()
        });
        let eligible = shootout::equalise(candidates);
        let keepers = [0, 1].map(|team| shootout::keeper(&eligible[team]).unwrap_or(acting[team]));
        let order = [0, 1].map(|team| shootout::order(&eligible[team], Some(keepers[team])));
        ShootoutLineup { order, keepers }
    }

    fn shootout_first(&self, draw: f64) -> usize {
        usize::from(draw >= 0.5)
    }

    fn shootout_end(&self, draw: f64) -> f64 {
        if draw < 0.5 { -1.0 } else { 1.0 }
    }

    fn keeper_dive(&self, view: &MatchView<'_>, draw: f64) -> f64 {
        if draw.abs() < view.tuning().shots.keeper_stays {
            0.0
        } else {
            draw.signum()
        }
    }

    fn shootout_save_hold(&self, view: &MatchView<'_>) -> f64 {
        view.tuning().shots.save_hold
    }

    fn shootout_decided(&self, view: &MatchView<'_>, scores: [u32; 2], taken: [u32; 2]) -> bool {
        shootout::decided(scores, taken, u32::from(view.rules().shootout.kicks))
    }
}

/// The action keys the clock and match end owns.
const CLOCK_KEYS: &[Action] = &[
    Action::AddedTime,
    Action::ExtraTimeAdded,
    Action::ExtraKickOff,
    Action::ShootoutFirstTeam,
    Action::ShootoutEnd,
    Action::KeeperDive,
    Action::ShootoutSaveHold,
];

pub const CLOCK_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Runs the clock and the end of the match: added time, half-time, extra time, the shoot-out, full time, and abandonment below the minimum players.",
    inputs: "The referee's clock and stoppage tally, the score, whether the match is a knockout, the players on the pitch and their finishing, composure, and keeping attributes, the rule pack, and the engine tuning.",
    outputs: "The added seconds, what follows each period, the extra-time kick-off team, the abandoned team, the shoot-out line-up, first team, end, keeper dive, save-hold threshold, and result; the loop applies them.",
    tuning: &[
        "added_time",
        "extra_time",
        "shootout",
        "min_players",
        "shots.keeper_stays",
        "shots.save_hold",
    ],
    calibration: "none: no added-time or shoot-out band in realism-bands.json",
    keys: CLOCK_KEYS,
};

/// Test only: version 1 with one second more added time in the first half. The gate tests
/// select it to prove that one changed output fails the gate.
#[cfg(feature = "scenario")]
pub struct ClockFaulty;

#[cfg(feature = "scenario")]
impl ClockModule for ClockFaulty {
    fn tally_seconds(&self, view: &MatchView<'_>, extra: bool) -> u32 {
        ClockV1.tally_seconds(view, extra)
    }

    fn added_seconds(&self, view: &MatchView<'_>, extra: bool, draw: f64) -> u32 {
        let seconds = ClockV1.added_seconds(view, extra, draw);
        if view.referee().clock.half == 0 {
            seconds + 1
        } else {
            seconds
        }
    }

    fn period_end(&self, view: &MatchView<'_>) -> PeriodEnd {
        ClockV1.period_end(view)
    }

    fn extra_kick_off(&self, draw: f64) -> usize {
        ClockV1.extra_kick_off(draw)
    }

    fn abandoned(&self, view: &MatchView<'_>) -> Option<usize> {
        ClockV1.abandoned(view)
    }

    fn shootout_lineup(&self, view: &MatchView<'_>) -> ShootoutLineup {
        ClockV1.shootout_lineup(view)
    }

    fn shootout_first(&self, draw: f64) -> usize {
        ClockV1.shootout_first(draw)
    }

    fn shootout_end(&self, draw: f64) -> f64 {
        ClockV1.shootout_end(draw)
    }

    fn keeper_dive(&self, view: &MatchView<'_>, draw: f64) -> f64 {
        ClockV1.keeper_dive(view, draw)
    }

    fn shootout_save_hold(&self, view: &MatchView<'_>) -> f64 {
        ClockV1.shootout_save_hold(view)
    }

    fn shootout_decided(&self, view: &MatchView<'_>, scores: [u32; 2], taken: [u32; 2]) -> bool {
        ClockV1.shootout_decided(view, scores, taken)
    }
}

#[cfg(feature = "scenario")]
pub const CLOCK_FAULTY_CARD: ModuleCard = ModuleCard {
    purpose: "Test only: clock version 1 with one second more added time in the first half.",
    inputs: "As clock version 1.",
    outputs: "As clock version 1.",
    tuning: &["none"],
    calibration: "none: test module for the replay gate",
    keys: CLOCK_KEYS,
};

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
        // Extra time keeps the pricing and caps the added time lower.
        let extra = extra_time_allowance(&rules);
        assert_eq!(added_seconds(&busy, &extra, 0.9), 300);
        assert_eq!(added_seconds(&tally, &extra, 0.5), 190);
    }

    #[test]
    fn a_full_match_announces_regulation_plus_both_caps() {
        let rules = shipped_content().rules;
        assert_eq!(max_ticks(90, &rules), 270_000 + 2 * 900 * 50);
        assert_eq!(max_ticks(90, &rules), 360_000);
        assert_eq!(max_ticks(5, &rules), 15_000);
        assert!(!MatchClock::new(5, &rules, false).plays_added);
    }

    #[test]
    fn a_knockout_match_announces_extra_time_and_the_shootout_allowance() {
        let rules = shipped_content().rules;
        // Two periods of 15 minutes, each with up to 300 s added.
        assert_eq!(knockout_extra_ticks(90, &rules), 2 * (900 + 300) * 50);
        assert_eq!(knockout_extra_ticks(90, &rules), 120_000);
        assert_eq!(knockout_extra_ticks(5, &rules), 0);
        // 10 rounds of two kicks, each 3 x 750 ticks of dead ball and 250 of live ball.
        assert_eq!(shootout_allowance_ticks(&rules, 750), 10 * 2 * 2_500);
        assert_eq!(shootout_allowance_ticks(&rules, 750), 50_000);
        assert_eq!(MatchClock::new(90, &rules, true).extra_periods, 2);
        assert_eq!(MatchClock::new(90, &rules, false).extra_periods, 0);
        assert_eq!(MatchClock::new(5, &rules, true).extra_periods, 0);
    }

    #[test]
    fn the_minute_counts_regulation_then_added_time() {
        let rules = shipped_content().rules;
        let mut clock = MatchClock::new(90, &rules, false);
        assert_eq!(clock.minute(9_000), (3, None));
        assert_eq!(clock.minute(135_000 + 4_000), (45, Some(2)));
        clock.added_ticks[0] = Some(9_000);
        clock.next_half(144_000);
        assert_eq!(clock.minute(144_000 + 3_000), (46, None));
        assert_eq!(clock.minute(144_000 + 135_000 + 1), (90, Some(1)));
        assert!(clock.last_period());
    }

    #[test]
    fn the_minute_runs_on_past_ninety_in_extra_time() {
        let rules = shipped_content().rules;
        let mut clock = MatchClock::new(90, &rules, true);
        clock.added_ticks[0] = Some(0);
        clock.next_half(135_000);
        assert!(!clock.last_period());
        assert_eq!(clock.minute(270_000 + 5 * 3_000), (90, Some(6)));
        clock.added_ticks[1] = Some(15_000);
        // Extra time starts after 90 minutes plus 5 added.
        let et = 270_000 + 15_000;
        clock.next_half(et);
        assert!(clock.in_extra_time());
        assert_eq!(clock.minute(et + 5 * 3_000), (95, None));
        assert_eq!(clock.end_minute(), 105);
        clock.added_ticks[2] = Some(3_000);
        let second = et + 45_000 + 3_000;
        clock.next_half(second);
        assert!(clock.last_period());
        assert_eq!(clock.minute(second), (105, None));
        assert_eq!(clock.minute(second + 45_000 - 1), (119, None));
        assert_eq!(clock.minute(second + 45_000 + 1), (120, Some(1)));
        assert_eq!(clock.end_minute(), 120);
    }
}
