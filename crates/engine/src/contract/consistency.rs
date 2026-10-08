//! Consistency: the hidden everyday spread of a player's play around his ratings.
//!
//! Each player carries a form offset in tenths of a rating point. It has two parts: a match
//! part drawn at kick-off (or when he comes on), and a period part drawn again every
//! `period_minutes` of the match. Each part is a bounded zero-mean draw whose size shrinks as
//! his consistency rises: `spread = max · (20 − c) / 19`, so a player at 20.0 has no off day
//! and a player at 1.0 swings by the whole maximum. Each part is centred over the players of
//! his side who drew it, so a side's mean offset is 0: consistency spreads a player's matches
//! and periods without adding team-level variance.
//!
//! The offset moves every rating a stage table reads, through the same rebuild the states
//! use ([`super::states::effective_with_form`]); pace and the hidden values take none.

use garde::Validate;
use serde::{Deserialize, Serialize};

/// The consistency block of the contract (`engine.contract.consistency`), in rating points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ConsistencyTuning {
    /// The largest match part, for a player at consistency 1.0.
    #[garde(range(min = 0.0, max = 3.0))]
    pub match_max: f64,
    /// The largest period part, for a player at consistency 1.0.
    #[garde(range(min = 0.0, max = 3.0))]
    pub period_max: f64,
    /// Minutes of the match between two draws of the period part.
    #[garde(range(min = 5, max = 45))]
    pub period_minutes: u32,
}

impl ConsistencyTuning {
    /// The largest offset either way, in tenths: both maxima together.
    pub fn bound_tenths(&self) -> i8 {
        // Both maxima are at most 3.0, so the bound is at most 60 tenths.
        ((self.match_max + self.period_max) * 10.0).round() as i8
    }
}

/// The size of a part for a player of consistency `c` (1.0 to 20.0): `max · (20 − c) / 19`,
/// `max` at 1.0 and 0 at 20.0.
pub fn spread(c: f64, max: f64) -> f64 {
    max * ((20.0 - c) / 19.0).clamp(0.0, 1.0)
}

/// The size of a part for a player whose attributes are `attributes`, `consistency` being
/// the schema index of his consistency: [`spread`] on that rating.
pub fn spread_of(attributes: &crate::player::Attributes, consistency: usize, max: f64) -> f64 {
    spread(attributes.get(consistency).decimal(), max)
}

/// A bounded zero-mean draw from two uniform draws in [0, 1): their sum less 1, a triangular
/// draw on −1 to 1.
pub fn draw(u1: f64, u2: f64) -> f64 {
    u1 + u2 - 1.0
}

/// `offsets` less their mean, so they sum to 0. Empty input stays empty.
pub fn centre(offsets: &mut [f64]) {
    if offsets.is_empty() {
        return;
    }
    let mean = offsets.iter().sum::<f64>() / offsets.len() as f64;
    for o in offsets {
        *o -= mean;
    }
}

/// An offset in rating points as tenths, rounded and kept within `±bound` tenths.
pub fn to_tenths(x: f64, bound: i8) -> i8 {
    let b = f64::from(bound);
    // Inside ±bound, at most 60, after the clamp.
    (x * 10.0).round().clamp(-b, b) as i8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_player_at_twenty_never_moves_and_the_spread_falls_with_consistency() {
        assert_eq!(spread(20.0, 1.0), 0.0);
        assert_eq!(spread(1.0, 1.0), 1.0);
        assert_eq!(spread(10.0, 0.75), 0.75 * 10.0 / 19.0);
        for u in [0.0, 0.3, 0.999] {
            assert_eq!(spread(20.0, 3.0) * draw(u, u), 0.0);
        }
        assert!(spread(18.0, 1.0) < spread(4.0, 1.0));
    }

    #[test]
    fn a_draw_stays_inside_minus_one_to_one_and_is_zero_mean() {
        assert_eq!(draw(0.5, 0.5), 0.0);
        assert_eq!(draw(0.0, 0.0), -1.0);
        assert!(draw(0.999_999, 0.999_999) < 1.0);
        assert_eq!(draw(0.25, 0.75), 0.0);
    }

    #[test]
    fn centred_offsets_sum_to_zero() {
        let mut o = [0.4, -0.1, 0.9, 0.3, -0.7, 0.2];
        centre(&mut o);
        assert!(o.iter().sum::<f64>().abs() < 1e-12, "{o:?}");
        let mut none: [f64; 0] = [];
        centre(&mut none);
    }

    #[test]
    fn tenths_round_and_keep_inside_the_bound() {
        assert_eq!(to_tenths(0.26, 17), 3);
        assert_eq!(to_tenths(-0.24, 17), -2);
        assert_eq!(to_tenths(5.0, 17), 17);
        assert_eq!(to_tenths(-5.0, 17), -17);
        let t = ConsistencyTuning {
            match_max: 1.0,
            period_max: 0.75,
            period_minutes: 15,
        };
        assert_eq!(t.bound_tenths(), 18);
    }
}
