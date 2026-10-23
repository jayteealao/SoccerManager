//! A band's word against the old engine.
//!
//! A change run pairs each match of the changed engine with the same match of the old engine,
//! so every band has a paired change `d` with a studentized error `se`. Two tests cross:
//!
//! | difference test `|d| > c * se` | equivalence test `|d| + z(1 - alpha) * se < e` | word |
//! |---|---|---|
//! | moved | yes | equivalent: a move smaller than anyone would notice |
//! | moved | no | better or worse, by direction |
//! | not moved | yes | equivalent |
//! | not moved | no | not sure, never equivalent |
//!
//! `c` is the joint threshold over every band of the run, so a sound change flags a move on
//! any band in at most the family false-alarm rate of runs; `e` is the band's noticeable
//! error. Better means the changed engine sits closer to the band's target than the old one.
//! A result that is not significant is never equivalent unless the equivalence test itself
//! passes, and a zero error never passes it. Only the real-football word decides a run; this
//! word says what the change moved.

use serde::{Deserialize, Serialize};

use crate::special::norm_quantile;

/// A band's word against the old engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OldWord {
    #[serde(rename = "better")]
    Better,
    #[serde(rename = "worse")]
    Worse,
    #[serde(rename = "equivalent")]
    Equivalent,
    #[serde(rename = "not sure")]
    NotSure,
}

impl OldWord {
    pub fn as_str(self) -> &'static str {
        match self {
            OldWord::Better => "better",
            OldWord::Worse => "worse",
            OldWord::Equivalent => "equivalent",
            OldWord::NotSure => "not sure",
        }
    }
}

/// The direction of a significant move of the changed engine against the old one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Moved {
    #[serde(rename = "up")]
    Up,
    #[serde(rename = "down")]
    Down,
}

/// The word of a band from its paired change `diff` (changed minus old), the change's error
/// `se`, the joint threshold `critical`, the band's noticeable error `noticeable` and the
/// pass rule's `alpha`. `toward_target` says whether the changed engine sits closer to the
/// band's target than the old one (`None` when the band has no target here, or both sit at
/// the same distance). Returns the word and, when the difference test is significant, the
/// direction of the move.
pub fn old_word(
    diff: f64,
    se: f64,
    critical: f64,
    noticeable: f64,
    alpha: f64,
    toward_target: Option<bool>,
) -> (OldWord, Option<Moved>) {
    if !diff.is_finite() {
        return (OldWord::NotSure, None);
    }
    let error = if se.is_finite() {
        se.max(0.0)
    } else {
        f64::INFINITY
    };
    let moved = diff.abs() > critical * error;
    let direction = moved.then_some(if diff > 0.0 { Moved::Up } else { Moved::Down });
    // A zero error proves nothing: the equivalence test needs a positive error.
    let equivalent = error > 0.0
        && error.is_finite()
        && noticeable > 0.0
        && diff.abs() + norm_quantile(1.0 - alpha) * error < noticeable;
    let word = match (moved, equivalent) {
        (_, true) => OldWord::Equivalent,
        (true, false) => match toward_target {
            Some(true) => OldWord::Better,
            Some(false) => OldWord::Worse,
            None => OldWord::NotSure,
        },
        (false, false) => OldWord::NotSure,
    };
    (word, direction)
}

/// Whether the changed engine sits closer to `target` than the old one: `Some(true)` closer,
/// `Some(false)` farther, `None` when there is no target or both sit at the same distance.
pub fn toward(changed: f64, old: f64, target: Option<f64>) -> Option<bool> {
    let t = target?;
    let (a, b) = ((changed - t).abs(), (old - t).abs());
    if !(a.is_finite() && b.is_finite()) || a == b {
        None
    } else {
        Some(a < b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: f64 = 3.0;
    const ALPHA: f64 = 0.05;

    struct Row {
        name: &'static str,
        diff: f64,
        se: f64,
        noticeable: f64,
        toward: Option<bool>,
        word: OldWord,
        moved: Option<Moved>,
    }

    #[test]
    fn each_row_of_the_truth_table_gets_its_word() {
        let z = norm_quantile(1.0 - ALPHA);
        let rows = [
            // Moved (|d| > 3 se) and inside the noticeable error: a move nobody would notice.
            Row {
                name: "moved, equivalent",
                diff: 0.4,
                se: 0.1,
                noticeable: 0.4 + z * 0.1 + 0.01,
                toward: Some(false),
                word: OldWord::Equivalent,
                moved: Some(Moved::Up),
            },
            Row {
                name: "moved, not equivalent, closer",
                diff: -0.5,
                se: 0.1,
                noticeable: 0.3,
                toward: Some(true),
                word: OldWord::Better,
                moved: Some(Moved::Down),
            },
            Row {
                name: "moved, not equivalent, farther",
                diff: 0.5,
                se: 0.1,
                noticeable: 0.3,
                toward: Some(false),
                word: OldWord::Worse,
                moved: Some(Moved::Up),
            },
            Row {
                name: "not moved, equivalent",
                diff: 0.1,
                se: 0.1,
                noticeable: 0.1 + z * 0.1 + 0.01,
                toward: Some(true),
                word: OldWord::Equivalent,
                moved: None,
            },
            Row {
                name: "not moved, not equivalent",
                diff: 0.1,
                se: 0.1,
                noticeable: 0.2,
                toward: Some(true),
                word: OldWord::NotSure,
                moved: None,
            },
        ];
        for r in rows {
            let got = old_word(r.diff, r.se, C, r.noticeable, ALPHA, r.toward);
            assert_eq!(got, (r.word, r.moved), "{}", r.name);
        }
    }

    #[test]
    fn a_result_that_is_not_significant_is_never_equivalent_unless_it_proves_it() {
        // A zero change with a wide error: not moved, and the interval reaches past the
        // noticeable error, so the word is not sure.
        let (word, moved) = old_word(0.0, 1.0, C, 0.5, ALPHA, Some(true));
        assert_eq!((word, moved), (OldWord::NotSure, None));
        for diff in [-0.2, -0.05, 0.0, 0.05, 0.2] {
            let (w, _) = old_word(diff, 0.3, C, 0.4, ALPHA, None);
            assert_ne!(w, OldWord::Equivalent, "diff {diff}");
        }
    }

    #[test]
    fn a_zero_error_is_never_equivalent() {
        assert_eq!(
            old_word(0.0, 0.0, C, 1.0, ALPHA, Some(true)).0,
            OldWord::NotSure
        );
        let (w, m) = old_word(0.01, 0.0, C, 1.0, ALPHA, Some(true));
        assert_eq!((w, m), (OldWord::Better, Some(Moved::Up)));
        assert_eq!(
            old_word(0.0, f64::NAN, C, 1.0, ALPHA, None).0,
            OldWord::NotSure
        );
    }

    #[test]
    fn a_moved_band_with_no_target_is_not_sure_and_keeps_its_direction() {
        assert_eq!(
            old_word(-0.9, 0.1, C, 0.3, ALPHA, None),
            (OldWord::NotSure, Some(Moved::Down))
        );
    }

    #[test]
    fn closer_to_the_target_is_toward_it() {
        assert_eq!(toward(1.1, 1.3, Some(1.0)), Some(true));
        assert_eq!(toward(1.3, 1.1, Some(1.0)), Some(false));
        assert_eq!(toward(0.5, 1.5, Some(1.0)), None);
        assert_eq!(toward(0.5, 1.5, None), None);
    }
}
