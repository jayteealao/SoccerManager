//! The jobs of the body fields and the match condition inputs: height, age, the days of rest
//! before the match, sharpness, and adaptation; and the build word.
//!
//! Height adds standing reach to the jump and costs a little turning. Age speeds the drain
//! late in a match, slows the recovery from the days of rest, and raises the injury chance
//! when rest is short. Sharpness and adaptation lower ratings through the sharpness and
//! adaptation modifiers ([`crate::modules::modifier::condition`]); nationality acts through
//! adaptation, so the engine never compares countries. Build is a word derived from strength
//! and balance, with no job in play.
//!
//! A missing field or input reads the reference, which has no effect at all: a converted
//! version 1 player, who has no body, plays as an average body.

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::attributes::Direction;

/// The body block of the contract (`engine.contract.body`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct BodyJobs {
    #[garde(dive)]
    pub height: HeightJob,
    #[garde(dive)]
    pub age: AgeJob,
    #[garde(dive)]
    pub rest: RestJob,
    #[garde(dive)]
    pub sharpness: DropJob,
    #[garde(dive)]
    pub adaptation: DropJob,
    #[garde(dive)]
    pub build: BuildWord,
    /// The statistic each body job moves, for the sensitivity rules.
    #[garde(dive)]
    pub jobs: BodyJobRows,
}

/// Height: standing reach in the air, and a small turning cost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct HeightJob {
    /// The height with no effect, in centimetres.
    #[garde(range(min = 150.0, max = 215.0))]
    pub reference_cm: f64,
    /// Metres of reach per centimetre above the reference.
    #[garde(range(min = 0.0, max = 0.05))]
    pub reach_per_cm: f64,
    /// The share of turning lost per centimetre above the reference.
    #[garde(range(min = 0.0, max = 0.01))]
    pub turn_cost_per_cm: f64,
}

/// Age: the late fade, the recovery from rest, and the injury risk under congestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AgeJob {
    /// The age with no effect, in years; younger players are treated as this age.
    #[garde(range(min = 15.0, max = 45.0))]
    pub reference: f64,
    /// The minute from which the late fade grows, over the next 30 minutes.
    #[garde(range(min = 0, max = 120))]
    pub late_from_minute: u32,
    /// The extra share of drain per year above the reference, at full fade.
    #[garde(range(min = 0.0, max = 0.5))]
    pub fade_per_year: f64,
    /// The share of the recovery from rest lost per year above the reference.
    #[garde(range(min = 0.0, max = 0.1))]
    pub recovery_loss_per_year: f64,
    /// The extra share of congestion injury risk per year above the reference.
    #[garde(range(min = 0.0, max = 0.5))]
    pub injury_per_year: f64,
}

/// The days of rest before the match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RestJob {
    /// Energy a player has the day after a match, before any rest day.
    #[garde(range(min = 0.0, max = 1.0))]
    pub post_match_energy: f64,
    /// Fewer rest days than this raise the injury chance.
    #[garde(range(min = 1, max = 14))]
    pub congestion_days: u8,
    /// The extra share of injury chance with no rest day at all.
    #[garde(range(min = 0.0, max = 5.0))]
    pub congestion_risk: f64,
}

/// A condition input from 0 to 100 percent: at 0 it lowers its group by `max_drop` rating
/// points, at 100 not at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct DropJob {
    #[garde(range(min = 0.0, max = 5.0))]
    pub max_drop: f64,
}

/// The build word: `frame = strength − frame_balance_weight · (balance − 10)`; slight below
/// `slight_below`, powerful from `powerful_from`, athletic between.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct BuildWord {
    #[garde(range(min = 0.0, max = 2.0))]
    pub frame_balance_weight: f64,
    #[garde(range(min = 1.0, max = 20.0))]
    pub slight_below: f64,
    #[garde(range(min = self.slight_below, max = 20.0))]
    pub powerful_from: f64,
}

/// The statistic each body job moves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct BodyJobRows {
    #[garde(dive)]
    pub height: JobRow,
    #[garde(dive)]
    pub age: JobRow,
    #[garde(dive)]
    pub nationality: JobRow,
}

/// One body job's statistic and the way it moves as the field rises.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct JobRow {
    #[garde(length(min = 3, max = 120))]
    pub statistic: String,
    #[garde(skip)]
    pub direction: Direction,
}

impl Default for BodyJobs {
    /// The shipped body block.
    fn default() -> Self {
        Self {
            height: HeightJob {
                reference_cm: 181.0,
                reach_per_cm: 0.0133,
                turn_cost_per_cm: 0.002,
            },
            age: AgeJob {
                reference: 28.0,
                late_from_minute: 60,
                fade_per_year: 0.04,
                recovery_loss_per_year: 0.03,
                injury_per_year: 0.03,
            },
            rest: RestJob {
                post_match_energy: 0.4,
                congestion_days: 4,
                congestion_risk: 1.0,
            },
            sharpness: DropJob { max_drop: 1.5 },
            adaptation: DropJob { max_drop: 1.0 },
            build: BuildWord {
                frame_balance_weight: 0.5,
                slight_below: 9.0,
                powerful_from: 14.0,
            },
            jobs: BodyJobRows {
                height: JobRow {
                    statistic: "aerial balls reached per match".into(),
                    direction: Direction::Up,
                },
                age: JobRow {
                    statistic: "energy lost after minute 60".into(),
                    direction: Direction::Up,
                },
                nationality: JobRow {
                    statistic: "mental effective ratings, through adaptation".into(),
                    direction: Direction::Up,
                },
            },
        }
    }
}

/// A player's body as play reads it: height in centimetres and age in years, each absent for
/// a player whose file gives none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Body {
    pub height_cm: Option<u8>,
    pub age: Option<u8>,
}

impl BodyJobs {
    /// Centimetres above the reference height; 0 for no height.
    fn above_cm(&self, body: Body) -> f64 {
        body.height_cm
            .map_or(0.0, |h| f64::from(h) - self.height.reference_cm)
    }

    /// Years above the reference age, never below 0; 0 for no age.
    fn years_over(&self, body: Body) -> f64 {
        body.age
            .map_or(0.0, |a| (f64::from(a) - self.age.reference).max(0.0))
    }

    /// How high the player reaches a ball in the air, in metres: the average player's reach
    /// times his jump (the aerial reach knob), plus his height above the reference.
    pub fn reach_m(&self, reach_height: f64, jump_knob: f64, body: Body) -> f64 {
        reach_height * jump_knob + self.height.reach_per_cm * self.above_cm(body)
    }

    /// The turning factor: the turn knob less the height cost, per centimetre above the
    /// reference (a shorter player turns a little better).
    pub fn turn_factor(&self, turn_knob: f64, body: Body) -> f64 {
        turn_knob * (1.0 - self.height.turn_cost_per_cm * self.above_cm(body))
    }

    /// The factor on the drain at `minute`: 1 until the late minute, then growing over 30
    /// minutes to `1 + fade_per_year` per year above the reference.
    pub fn late_fade(&self, body: Body, minute: f64) -> f64 {
        let late = ((minute - f64::from(self.age.late_from_minute)) / 30.0).clamp(0.0, 1.0);
        if late == 0.0 {
            return 1.0;
        }
        let years = self.years_over(body);
        if years == 0.0 {
            return 1.0;
        }
        1.0 + self.age.fade_per_year * years * late
    }

    /// Energy at kick-off after `rest_days` days of rest: the energy after a match plus each
    /// day's recovery (`recovery_per_day` stamina points, out of 100), less the share age
    /// takes off it; full energy when no rest days are given.
    pub fn start_energy(&self, rest_days: Option<u8>, body: Body, recovery_per_day: f64) -> f64 {
        let Some(days) = rest_days else {
            return 1.0;
        };
        let age = (1.0 - self.age.recovery_loss_per_year * self.years_over(body)).max(0.0);
        (self.rest.post_match_energy + f64::from(days) * recovery_per_day / 100.0 * age).min(1.0)
    }

    /// The factor on the injury chance from short rest: 1 with `congestion_days` days or
    /// more, or none given; up to `1 + congestion_risk` with no rest day, more for an older
    /// player.
    pub fn congestion(&self, rest_days: Option<u8>, body: Body) -> f64 {
        let Some(days) = rest_days else {
            return 1.0;
        };
        let short = self.rest.congestion_days.saturating_sub(days);
        if short == 0 {
            return 1.0;
        }
        let share = f64::from(short) / f64::from(self.rest.congestion_days);
        1.0 + self.rest.congestion_risk
            * share
            * (1.0 + self.age.injury_per_year * self.years_over(body))
    }

    /// The build word of a player with `strength` and `balance` on the 1 to 20 scale.
    pub fn build_word(&self, strength: f64, balance: f64) -> &'static str {
        let b = &self.build;
        let frame = strength - b.frame_balance_weight * (balance - 10.0);
        if frame < b.slight_below {
            "slight"
        } else if frame >= b.powerful_from {
            "powerful"
        } else {
            "athletic"
        }
    }

    /// The drop in rating points of a condition input of `percent` (0 to 100).
    pub fn drop(job: &DropJob, percent: Option<u8>) -> f64 {
        percent.map_or(0.0, |p| {
            -(1.0 - f64::from(p.min(100)) / 100.0) * job.max_drop
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(height_cm: Option<u8>, age: Option<u8>) -> Body {
        Body { height_cm, age }
    }

    #[test]
    fn every_job_is_exactly_neutral_at_the_reference_and_with_no_body() {
        let j = BodyJobs::default();
        for body in [
            b(None, None),
            b(Some(181), Some(28)),
            b(Some(181), Some(20)),
        ] {
            assert_eq!(
                j.reach_m(2.0, 1.07, body).to_bits(),
                (2.0f64 * 1.07).to_bits()
            );
            assert_eq!(j.turn_factor(0.93, body).to_bits(), 0.93f64.to_bits());
            assert_eq!(j.late_fade(body, 89.0), 1.0);
            assert_eq!(j.start_energy(None, body, 20.0), 1.0);
            assert_eq!(j.congestion(None, body), 1.0);
            assert_eq!(j.congestion(Some(7), body), 1.0);
        }
        assert_eq!(BodyJobs::drop(&j.sharpness, None), 0.0);
        assert_eq!(BodyJobs::drop(&j.sharpness, Some(100)), 0.0);
    }

    #[test]
    fn height_adds_reach_and_costs_turning() {
        let j = BodyJobs::default();
        let (short, tall) = (b(Some(170), None), b(Some(195), None));
        let gap = j.reach_m(2.0, 1.0, tall) - j.reach_m(2.0, 1.0, short);
        assert!((gap - 25.0 * 0.0133).abs() < 1e-12, "{gap}");
        assert!(j.turn_factor(1.0, tall) < 1.0 && j.turn_factor(1.0, short) > 1.0);
    }

    #[test]
    fn age_fades_late_recovers_slower_and_risks_more_under_congestion() {
        let j = BodyJobs::default();
        let (young, old) = (b(None, Some(22)), b(None, Some(34)));
        assert_eq!(j.late_fade(old, 55.0), 1.0, "nothing before minute 60");
        assert!(j.late_fade(old, 75.0) > 1.0 && j.late_fade(old, 90.0) > j.late_fade(old, 75.0));
        assert!((j.late_fade(old, 95.0) - (1.0 + 0.04 * 6.0)).abs() < 1e-12);
        assert_eq!(j.late_fade(young, 90.0), 1.0);
        assert!(j.start_energy(Some(2), old, 20.0) < j.start_energy(Some(2), young, 20.0));
        assert!(j.start_energy(Some(2), young, 20.0) < j.start_energy(Some(7), young, 20.0));
        assert_eq!(j.start_energy(Some(7), young, 20.0), 1.0);
        assert!(j.congestion(Some(2), young) > 1.0);
        assert!(j.congestion(Some(2), old) > j.congestion(Some(2), young));
        assert!((j.congestion(Some(0), young) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_build_word_derives_from_strength_and_balance() {
        let j = BodyJobs::default();
        assert_eq!(j.build_word(6.0, 12.0), "slight");
        assert_eq!(j.build_word(12.0, 10.0), "athletic");
        assert_eq!(j.build_word(16.0, 8.0), "powerful");
    }

    #[test]
    fn a_condition_input_drops_its_group_in_proportion() {
        let j = BodyJobs::default();
        assert_eq!(BodyJobs::drop(&j.sharpness, Some(0)), -1.5);
        assert!((BodyJobs::drop(&j.adaptation, Some(30)) + 0.7).abs() < 1e-12);
    }
}
