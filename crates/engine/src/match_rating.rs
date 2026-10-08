//! The match rating: one number per player per match, on 1.0 to 10.0 with one decimal.
//!
//! A rating is `base` plus a weighted sum of what the player did ([`PlayerTally`]): goals,
//! shots, expected goals, passes, tackles, interceptions, blocks, clearances, saves, fouls and
//! cards; goals conceded and a clean sheet by his position class (a keeper or a listed
//! defender); and the result, weighted by the share of the match he played. It is clamped to
//! `min` to `max` and rounded to a tenth. Every weight is in the tuning file's `match_rating`
//! block, so the five-match rule can tune them without a build.

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::team::Position;
use crate::rules::clock::TICKS_PER_MINUTE;
use crate::sim::tally::PlayerTally;

/// The match-rating block of the tuning file (`match_rating`). Weights are rating points per
/// count; `xg` is per expected goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct MatchRatingTuning {
    #[garde(range(min = 1.0, max = 10.0))]
    pub base: f64,
    #[garde(range(min = 1.0, max = 10.0))]
    pub min: f64,
    #[garde(range(min = self.min, max = 10.0))]
    pub max: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub goal: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub shot_on_target: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub shot_off_target: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub xg: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub pass_completed: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub pass_failed: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub tackle_won: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub interception: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub block: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub clearance: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub save: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub foul: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub yellow: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub red: f64,
    /// Per goal his side conceded while he played.
    #[garde(dive)]
    pub conceded: ByClass,
    #[garde(dive)]
    pub clean_sheet: CleanSheet,
    #[garde(dive)]
    pub result: ResultWeights,
    /// The positions that count as defenders for the conceded and clean-sheet weights.
    #[garde(skip)]
    pub defenders: Vec<Position>,
}

/// A weight for a keeper and one for a defender; other players take none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ByClass {
    #[garde(range(min = -5.0, max = 5.0))]
    pub keeper: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub defender: f64,
}

/// The clean-sheet bonus, given only after `min_minutes` on the pitch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CleanSheet {
    #[garde(range(min = -5.0, max = 5.0))]
    pub keeper: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub defender: f64,
    #[garde(range(min = 0, max = 120))]
    pub min_minutes: u32,
}

/// The result weights, times the share of the match played.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ResultWeights {
    #[garde(range(min = -5.0, max = 5.0))]
    pub win: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub loss: f64,
}

/// How the match ended for the player's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Win,
    Draw,
    Loss,
}

/// The rating of a player of `position` with tally `t`, whose side's match ended `outcome`,
/// in a match of `match_ticks` ticks: tenths of a point, 10 to 100.
pub fn rating(
    t: &PlayerTally,
    position: Position,
    outcome: Outcome,
    match_ticks: u32,
    m: &MatchRatingTuning,
) -> u8 {
    let n = |c: u32| f64::from(c);
    let off_target = t.shots.saturating_sub(t.shots_on_target);
    let failed = t.passes.saturating_sub(t.passes_completed);
    let mut r = m.base
        + m.goal * n(t.goals)
        + m.shot_on_target * n(t.shots_on_target)
        + m.shot_off_target * n(off_target)
        + m.xg * t.xg
        + m.pass_completed * n(t.passes_completed)
        + m.pass_failed * n(failed)
        + m.tackle_won * n(t.tackles_won)
        + m.interception * n(t.interceptions)
        + m.block * n(t.blocks)
        + m.clearance * n(t.clearances)
        + m.save * n(t.saves)
        + m.foul * n(t.fouls)
        + m.yellow * n(t.yellow)
        + m.red * n(t.red);
    let class = if position == Position::GK {
        Some((m.conceded.keeper, m.clean_sheet.keeper))
    } else if m.defenders.contains(&position) {
        Some((m.conceded.defender, m.clean_sheet.defender))
    } else {
        None
    };
    if let Some((conceded, clean)) = class {
        r += conceded * n(t.conceded);
        let minutes = t.ticks_played / TICKS_PER_MINUTE;
        if t.conceded == 0 && minutes >= m.clean_sheet.min_minutes {
            r += clean;
        }
    }
    let share = if match_ticks == 0 {
        0.0
    } else {
        (f64::from(t.ticks_played) / f64::from(match_ticks)).min(1.0)
    };
    r += share
        * match outcome {
            Outcome::Win => m.result.win,
            Outcome::Draw => 0.0,
            Outcome::Loss => m.result.loss,
        };
    // Inside 10..=100 after the clamp.
    (r.clamp(m.min, m.max) * 10.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> MatchRatingTuning {
        crate::data::test_support::shipped_content()
            .tuning
            .match_rating
    }

    const FULL: u32 = 90 * TICKS_PER_MINUTE;

    fn played() -> PlayerTally {
        PlayerTally {
            ticks_played: FULL,
            ..PlayerTally::default()
        }
    }

    #[test]
    fn a_quiet_full_draw_rates_the_base() {
        let m = tuning();
        assert_eq!(rating(&played(), Position::CM, Outcome::Draw, FULL, &m), 60);
    }

    #[test]
    fn two_goals_from_four_shots_on_target_in_a_draw() {
        let m = tuning();
        let t = PlayerTally {
            goals: 2,
            shots: 4,
            shots_on_target: 4,
            ..played()
        };
        // 6.0 + 2 · 1.0 + 4 · 0.15 = 8.6
        assert_eq!(rating(&t, Position::ST, Outcome::Draw, FULL, &m), 86);
    }

    #[test]
    fn a_keeper_with_five_saves_and_a_clean_sheet_in_a_win() {
        let m = tuning();
        let t = PlayerTally {
            saves: 5,
            ..played()
        };
        // 6.0 + 5 · 0.3 + 0.5 + 0.3 = 8.3
        assert_eq!(rating(&t, Position::GK, Outcome::Win, FULL, &m), 83);
    }

    #[test]
    fn a_red_card_and_forty_failed_passes_clamp_at_the_floor() {
        let m = tuning();
        let t = PlayerTally {
            red: 1,
            passes: 40,
            ..played()
        };
        // 6.0 − 1.5 − 40 · 0.04 − 0.3 = 2.6; add more failures to cross the floor.
        assert_eq!(rating(&t, Position::CM, Outcome::Loss, FULL, &m), 26);
        let worse = PlayerTally { passes: 200, ..t };
        assert_eq!(rating(&worse, Position::CM, Outcome::Loss, FULL, &m), 10);
    }

    #[test]
    fn a_clean_sheet_needs_the_minutes_and_a_defender_concedes() {
        let m = tuning();
        let short = PlayerTally {
            ticks_played: 30 * TICKS_PER_MINUTE,
            ..PlayerTally::default()
        };
        // A third of a draw: no clean sheet, no result weight.
        assert_eq!(rating(&short, Position::CB, Outcome::Draw, FULL, &m), 60);
        let beaten = PlayerTally {
            conceded: 2,
            ..played()
        };
        // 6.0 − 2 · 0.15 − 0.3 = 5.4
        assert_eq!(rating(&beaten, Position::CB, Outcome::Loss, FULL, &m), 54);
        // A midfielder takes no conceded weight.
        assert_eq!(rating(&beaten, Position::CM, Outcome::Loss, FULL, &m), 57);
    }
}
