//! A band's word from its interval and its margin.
//!
//! Pass when the whole interval sits inside `[-margin, +margin]`; fail when the whole
//! interval sits outside it; not sure otherwise. A point inside the margin with an interval
//! that crosses an edge is not sure, never pass, so a small or noisy run cannot pass by luck.

use serde::{Deserialize, Serialize};

use crate::interval::Interval;

/// A band's word against real football (or its design target).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Word {
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "not sure")]
    NotSure,
    #[serde(rename = "fail")]
    Fail,
}

impl Word {
    pub fn as_str(self) -> &'static str {
        match self {
            Word::Pass => "pass",
            Word::NotSure => "not sure",
            Word::Fail => "fail",
        }
    }
}

/// The word of an interval against a symmetric margin.
pub fn judge(iv: &Interval, margin: f64) -> Word {
    if iv.lo >= -margin && iv.hi <= margin {
        Word::Pass
    } else if iv.lo > margin || iv.hi < -margin {
        Word::Fail
    } else {
        Word::NotSure
    }
}

/// The word of a band with a design target and a cap: the interval of engine minus design
/// target against the margin, and the interval of engine minus real against the cap. Pass
/// when both pass, fail when either fails, not sure otherwise. `None` means the band has no
/// cap; a caller whose band has a cap passes its real interval, and when that interval is
/// missing the caller turns a pass into not sure itself.
pub fn judge_design(to_target: &Interval, margin: f64, to_real: Option<(&Interval, f64)>) -> Word {
    let target = judge(to_target, margin);
    let real = to_real.map(|(iv, cap)| judge(iv, cap));
    match (target, real) {
        (Word::Fail, _) | (_, Some(Word::Fail)) => Word::Fail,
        (Word::Pass, None) | (Word::Pass, Some(Word::Pass)) => Word::Pass,
        _ => Word::NotSure,
    }
}

/// The word of a distribution band from the bounds of its distance to the real distribution:
/// pass when the upper bound is below the margin, fail when the lower bound is above it.
pub fn judge_distance(lower: f64, upper: f64, margin: f64) -> Word {
    if upper < margin {
        Word::Pass
    } else if lower > margin {
        Word::Fail
    } else {
        Word::NotSure
    }
}

/// A pass below the band's minimum exposure becomes not sure, with the reason.
pub fn gate_exposure(word: Word, exposure: f64, min_exposure: f64) -> (Word, Option<String>) {
    if word == Word::Pass && exposure < min_exposure {
        (
            Word::NotSure,
            Some(format!(
                "exposure {exposure} is under the minimum {min_exposure}; a pass waits for it"
            )),
        )
    } else {
        (word, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iv(lo: f64, hi: f64) -> Interval {
        Interval {
            point: (lo + hi) / 2.0,
            lo,
            hi,
            se: (hi - lo) / 3.3,
            dropped: 0,
        }
    }

    #[test]
    fn a_whole_interval_inside_the_margin_passes() {
        assert_eq!(judge(&iv(-0.1, 0.2), 0.3), Word::Pass);
    }

    #[test]
    fn a_point_inside_with_an_edge_crossed_is_not_sure() {
        let crossing = Interval {
            point: 0.2,
            lo: 0.05,
            hi: 0.4,
            se: 0.1,
            dropped: 0,
        };
        assert!(crossing.point.abs() < 0.3);
        assert_eq!(judge(&crossing, 0.3), Word::NotSure);
    }

    #[test]
    fn a_whole_interval_outside_fails() {
        assert_eq!(judge(&iv(0.35, 0.6), 0.3), Word::Fail);
        assert_eq!(judge(&iv(-0.6, -0.31), 0.3), Word::Fail);
    }

    #[test]
    fn near_the_design_target_but_beyond_the_real_cap_fails() {
        let to_target = iv(-0.05, 0.05);
        let to_real = iv(0.6, 0.8);
        assert_eq!(
            judge_design(&to_target, 0.1, Some((&to_real, 0.5))),
            Word::Fail
        );
        assert_eq!(
            judge_design(&to_target, 0.1, Some((&iv(0.1, 0.3), 0.5))),
            Word::Pass
        );
        assert_eq!(judge_design(&to_target, 0.1, None), Word::Pass);
        assert_eq!(
            judge_design(&to_target, 0.1, Some((&iv(0.4, 0.6), 0.5))),
            Word::NotSure
        );
    }

    #[test]
    fn a_pass_under_the_minimum_exposure_is_not_sure() {
        let (word, why) = gate_exposure(Word::Pass, 400.0, 1000.0);
        assert_eq!(word, Word::NotSure);
        assert!(why.unwrap().contains("minimum 1000"));
        assert_eq!(gate_exposure(Word::Fail, 400.0, 1000.0).0, Word::Fail);
        assert_eq!(gate_exposure(Word::Pass, 1000.0, 1000.0).0, Word::Pass);
    }

    #[test]
    fn a_distance_passes_below_the_margin_and_fails_above_it() {
        assert_eq!(judge_distance(0.01, 0.04, 0.05), Word::Pass);
        assert_eq!(judge_distance(0.06, 0.09, 0.05), Word::Fail);
        assert_eq!(judge_distance(0.03, 0.07, 0.05), Word::NotSure);
    }
}
