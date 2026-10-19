//! The match rating: one number per player per match, on 1.0 to 10.0 with one decimal.
//!
//! A rating is `base` plus, for an outfield player, the threat his actions added
//! ([`PlayerTally::value_added`]: the real threat of a place, [`crate::sim::place_threat`],
//! that his passes and carries gained or his lost balls gave up, his shots' expected goals and
//! his goals, and the threat his tackles, interceptions, blocks and clearances took from the
//! other side), or, for a keeper, the goals he prevented (the post-shot expected goals he
//! faced less the goals from shots he conceded); then his fouls and cards; goals conceded and
//! a clean sheet by his position class (a keeper or a listed defender); and the result,
//! weighted by the share of the match he played. No term counts how many passes, shots or
//! tackles he made. It is clamped to `min` to `max` and rounded to a tenth. Every weight is in
//! the tuning file's `match_rating` block, so the five-match rule can tune them without a
//! build.

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::team::Position;
use crate::rules::clock::TICKS_PER_MINUTE;
use crate::sim::tally::PlayerTally;

/// The match-rating block of the tuning file (`match_rating`). Weights are rating points per
/// count; `value_added` and `goals_prevented` are per expected goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct MatchRatingTuning {
    #[garde(range(min = 1.0, max = 10.0))]
    pub base: f64,
    #[garde(range(min = 1.0, max = 10.0))]
    pub min: f64,
    #[garde(range(min = self.min, max = 10.0))]
    pub max: f64,
    /// Per expected goal of threat an outfield player's actions added
    /// ([`PlayerTally::value_added`]).
    #[garde(range(min = 0.0, max = 20.0))]
    pub value_added: f64,
    /// Per goal a keeper prevented: the post-shot expected goals he faced less the goals from
    /// shots he conceded.
    #[garde(range(min = -5.0, max = 5.0))]
    pub goals_prevented: f64,
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
    let keeper = position == Position::GK;
    // A keeper is rated by the goals he prevented; an outfield player by the threat his
    // actions added. Neither takes points for how many passes, shots or tackles he made.
    let value = if keeper {
        m.goals_prevented * (t.psxg_faced - n(t.shot_goals_conceded))
    } else {
        m.value_added * t.value_added
    };
    let mut r = m.base + value + m.foul * n(t.fouls) + m.yellow * n(t.yellow) + m.red * n(t.red);
    let class = if keeper {
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
        // 6.66, rounded to a tenth.
        assert_eq!(rating(&played(), Position::CM, Outcome::Draw, FULL, &m), 67);
    }

    #[test]
    fn the_threat_a_player_added_sets_his_rating() {
        let m = tuning();
        let t = PlayerTally {
            value_added: 0.5,
            ..played()
        };
        // 6.66 + 0.5 · 5.1 = 9.21
        assert_eq!(rating(&t, Position::ST, Outcome::Draw, FULL, &m), 92);
        let lost = PlayerTally {
            value_added: -0.2,
            ..played()
        };
        // 6.66 − 0.2 · 5.1 = 5.64
        assert_eq!(rating(&lost, Position::CM, Outcome::Draw, FULL, &m), 56);
    }

    #[test]
    fn a_keeper_who_prevents_goals_and_keeps_a_clean_sheet_in_a_win() {
        let m = tuning();
        let t = PlayerTally {
            saves: 5,
            psxg_faced: 1.5,
            ..played()
        };
        // 6.66 + 1.5 · 1.0 + 0.5 + 0.3 = 8.96; the saves add nothing of their own.
        assert_eq!(rating(&t, Position::GK, Outcome::Win, FULL, &m), 90);
        let beaten = PlayerTally {
            psxg_faced: 1.0,
            shot_goals_conceded: 2,
            conceded: 2,
            ..played()
        };
        // 6.66 + (1.0 − 2) · 1.0 − 2 · 0.3 − 0.3 = 4.76
        assert_eq!(rating(&beaten, Position::GK, Outcome::Loss, FULL, &m), 48);
    }

    #[test]
    fn no_player_takes_points_for_how_many_passes_shots_or_tackles_he_made() {
        let m = tuning();
        let busy = PlayerTally {
            passes: 60,
            passes_completed: 50,
            shots: 6,
            shots_on_target: 2,
            goals: 1,
            xg: 0.4,
            tackles_won: 5,
            interceptions: 4,
            blocks: 2,
            clearances: 10,
            ..played()
        };
        // Only the clean sheet adds: 6.66 + 0.5 for a keeper, + 0.3 for a defender; the counts
        // add nothing, and a midfielder rates the base.
        assert_eq!(rating(&busy, Position::GK, Outcome::Draw, FULL, &m), 72);
        assert_eq!(rating(&busy, Position::CB, Outcome::Draw, FULL, &m), 70);
        assert_eq!(rating(&busy, Position::CM, Outcome::Draw, FULL, &m), 67);
    }

    #[test]
    fn a_red_card_and_a_heavy_loss_of_threat_clamp_at_the_floor() {
        let m = tuning();
        let t = PlayerTally { red: 1, ..played() };
        // 6.66 − 1.5 − 0.3 = 4.86; lose more threat to cross the floor.
        assert_eq!(rating(&t, Position::CM, Outcome::Loss, FULL, &m), 49);
        let worse = PlayerTally {
            value_added: -2.0,
            ..t
        };
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
        assert_eq!(rating(&short, Position::CB, Outcome::Draw, FULL, &m), 67);
        let beaten = PlayerTally {
            conceded: 2,
            ..played()
        };
        // 6.66 − 2 · 0.15 − 0.3 = 6.06
        assert_eq!(rating(&beaten, Position::CB, Outcome::Loss, FULL, &m), 61);
        // A midfielder takes no conceded weight: 6.66 − 0.3 = 6.36.
        assert_eq!(rating(&beaten, Position::CM, Outcome::Loss, FULL, &m), 64);
    }
}
