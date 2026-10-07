//! One rating on the 1 to 20 scale, stored in tenths.
//!
//! Every rating the engine holds is a [`Rating`]: a whole number of tenths, 10 to 200 for
//! 1.0 to 20.0. It fits in a `u8`, so a player record stays `Copy`. A team file writes a
//! rating as a number with one decimal (`12.5`); the screens show the whole number (`13`).
//!
//! A rating converted from a version 1 team file is `2 × v` tenths, at least 1.0: a version
//! 1 value of 1 to 4 becomes 1.0. Play reads ratings only through the attribute contract
//! ([`crate::contract`]).

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A rating in tenths of the 1 to 20 scale.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Rating(u8);

/// The tenths of 1.0, the lowest rating.
pub const MIN_TENTHS: u8 = 10;
/// The tenths of 20.0, the highest rating.
pub const MAX_TENTHS: u8 = 200;

/// Why a decimal is not a rating.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum RatingError {
    #[error("is {0}; not on a tenth")]
    OffGrid(f64),
    #[error("is {0}; allowed 1.0 to 20.0")]
    OutOfRange(f64),
}

impl Rating {
    /// The rating of `tenths` tenths. Any `u8` is accepted; the loaders keep a rating inside
    /// 1.0 to 20.0.
    pub const fn from_tenths(tenths: u8) -> Self {
        Self(tenths)
    }

    /// The rating in tenths.
    pub const fn tenths(self) -> u8 {
        self.0
    }

    /// The whole number a screen shows: `T / 10` rounded, at least 1.
    pub fn whole(self) -> u8 {
        ((u16::from(self.0) + 5) / 10).max(1) as u8
    }

    /// The rating of a decimal from a version 2 file: on the tenth grid and from 1.0 to
    /// 20.0, or refused with the reason.
    pub fn from_decimal(value: f64) -> Result<Self, RatingError> {
        let tenths = value * 10.0;
        if !tenths.is_finite() || (tenths - tenths.round()).abs() > 1e-6 {
            return Err(RatingError::OffGrid(value));
        }
        let tenths = tenths.round();
        if !(f64::from(MIN_TENTHS)..=f64::from(MAX_TENTHS)).contains(&tenths) {
            return Err(RatingError::OutOfRange(value));
        }
        Ok(Self(tenths as u8))
    }

    /// The rating as a decimal, `T / 10`.
    pub fn decimal(self) -> f64 {
        f64::from(self.0) / 10.0
    }
}

impl fmt::Display for Rating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.0 / 10, self.0 % 10)
    }
}

impl Serialize for Rating {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f64(self.decimal())
    }
}

impl<'de> Deserialize<'de> for Rating {
    /// Reads any number on the tenth grid that fits in tenths (0.0 to 25.5); the range 1.0
    /// to 20.0 is the team file's check, which names the player and the attribute.
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(d)?;
        let tenths = value * 10.0;
        if !tenths.is_finite() || (tenths - tenths.round()).abs() > 1e-6 {
            return Err(serde::de::Error::custom(RatingError::OffGrid(value)));
        }
        let tenths = tenths.round();
        if !(0.0..=255.0).contains(&tenths) {
            return Err(serde::de::Error::custom(RatingError::OutOfRange(value)));
        }
        Ok(Self(tenths as u8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tenth_round_trips_through_json() {
        for t in MIN_TENTHS..=MAX_TENTHS {
            let r = Rating::from_tenths(t);
            let text = serde_json::to_string(&r).unwrap();
            assert_eq!(serde_json::from_str::<Rating>(&text).unwrap(), r, "{text}");
            assert_eq!(Rating::from_decimal(r.decimal()).unwrap(), r);
            assert_eq!(text, r.to_string(), "one decimal in the file");
        }
    }

    #[test]
    fn a_decimal_off_the_grid_or_out_of_range_is_refused() {
        assert_eq!(
            Rating::from_decimal(12.34).unwrap_err().to_string(),
            "is 12.34; not on a tenth"
        );
        assert_eq!(
            Rating::from_decimal(20.5).unwrap_err().to_string(),
            "is 20.5; allowed 1.0 to 20.0"
        );
        assert!(Rating::from_decimal(0.9).is_err());
        assert!(Rating::from_decimal(f64::NAN).is_err());
        assert_eq!(Rating::from_decimal(1.0).unwrap().tenths(), 10);
        assert_eq!(Rating::from_decimal(20.0).unwrap().tenths(), 200);
        assert!(serde_json::from_str::<Rating>("12.34").is_err());
        assert!(serde_json::from_str::<Rating>("25.6").is_err());
    }

    #[test]
    fn the_whole_number_rounds_and_is_at_least_one() {
        assert_eq!(Rating::from_tenths(2).whole(), 1);
        assert_eq!(Rating::from_tenths(14).whole(), 1);
        assert_eq!(Rating::from_tenths(15).whole(), 2);
        assert_eq!(Rating::from_tenths(125).whole(), 13);
        assert_eq!(Rating::from_tenths(200).whole(), 20);
        assert_eq!(Rating::from_tenths(125).to_string(), "12.5");
        assert_eq!(Rating::from_tenths(2).to_string(), "0.2");
    }
}
