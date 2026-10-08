//! Hidden values in words: what a manager may learn of a player's consistency and injury
//! proneness.
//!
//! A hidden value never leaves the engine as a number. It leaves as a word key from the
//! content bands (`rarely_off`, `injury_prone`, …) and a confidence from the matches the
//! player has seen at the club: none gives "not yet known" and no word, a few give
//! "tentative", many give "firm". The screens own each key's display text; the staff piece
//! later replaces the confidence source.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

/// How sure the club is of a hidden value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// No match seen at the club: no word is given.
    NotYetKnown,
    Tentative,
    Firm,
}

impl Confidence {
    /// The confidence as the wire writes it.
    pub fn code(self) -> &'static str {
        match self {
            Confidence::NotYetKnown => "not_yet_known",
            Confidence::Tentative => "tentative",
            Confidence::Firm => "firm",
        }
    }
}

/// The hidden block of the tuning file (`hidden`): the confidence thresholds and each hidden
/// value's word bands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct HiddenTuning {
    #[garde(dive)]
    pub confidence: ConfidenceTuning,
    /// Per hidden attribute, its bands from the top down: the first band whose `from` the
    /// rating reaches gives the word, and the last band starts at 1.0.
    #[garde(custom(check_words))]
    pub words: BTreeMap<String, Vec<WordBand>>,
}

/// Matches at the club from which a word is tentative, and from which it is firm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ConfidenceTuning {
    #[garde(range(min = 1, max = 1000))]
    pub tentative_from: u16,
    #[garde(range(min = self.tentative_from, max = 1000))]
    pub firm_from: u16,
}

/// One word band: ratings from `from` up to the band above take `word`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordBand {
    pub from: f64,
    pub word: String,
}

fn check_words(words: &BTreeMap<String, Vec<WordBand>>, _ctx: &()) -> garde::Result {
    for (name, bands) in words {
        let Some(last) = bands.last() else {
            return Err(garde::Error::new(format!("{name}: no band")));
        };
        for (i, b) in bands.iter().enumerate() {
            if !(1.0..=20.0).contains(&b.from) {
                return Err(garde::Error::new(format!(
                    "{name}: band {i} starts at {}; allowed 1.0 to 20.0",
                    b.from
                )));
            }
            if i > 0 && b.from >= bands[i - 1].from {
                return Err(garde::Error::new(format!(
                    "{name}: band {i} starts at {}, not below the band above it; list the \
                     bands from the top down",
                    b.from
                )));
            }
            let key_ok = !b.word.is_empty()
                && b.word.len() <= 32
                && b.word.bytes().all(|c| c.is_ascii_lowercase() || c == b'_');
            if !key_ok {
                return Err(garde::Error::new(format!(
                    "{name}: word {:?} is not a lower-case key",
                    b.word
                )));
            }
        }
        if last.from != 1.0 {
            return Err(garde::Error::new(format!(
                "{name}: the last band starts at {}; it must start at 1.0",
                last.from
            )));
        }
    }
    Ok(())
}

/// The confidence for `matches` seen at the club: none or 0 is not yet known, from
/// `firm_from` firm, otherwise tentative.
pub fn confidence(matches: Option<u16>, t: &ConfidenceTuning) -> Confidence {
    match matches {
        None | Some(0) => Confidence::NotYetKnown,
        Some(m) if m >= t.firm_from => Confidence::Firm,
        Some(m) if m >= t.tentative_from => Confidence::Tentative,
        Some(_) => Confidence::NotYetKnown,
    }
}

/// The word of `rating` (1.0 to 20.0): the first band whose `from` it reaches, or the last.
pub fn word(rating: f64, bands: &[WordBand]) -> Option<&str> {
    bands
        .iter()
        .find(|b| rating >= b.from)
        .or(bands.last())
        .map(|b| b.word.as_str())
}

/// A hidden value as the club knows it: a word key, given only once the club knows something,
/// and how sure it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenWord {
    pub word: Option<String>,
    pub confidence: Confidence,
}

/// The word and confidence of hidden attribute `name` at `rating`, for a player who has seen
/// `matches` at the club.
pub fn hidden_word(name: &str, rating: f64, matches: Option<u16>, t: &HiddenTuning) -> HiddenWord {
    let confidence = confidence(matches, &t.confidence);
    let word = match confidence {
        Confidence::NotYetKnown => None,
        _ => t
            .words
            .get(name)
            .and_then(|bands| word(rating, bands))
            .map(str::to_string),
    };
    HiddenWord { word, confidence }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> HiddenTuning {
        crate::data::test_support::shipped_content().tuning.hidden
    }

    #[test]
    fn matches_at_the_club_set_the_confidence() {
        let t = tuning().confidence;
        let cases = [
            (None, Confidence::NotYetKnown),
            (Some(0), Confidence::NotYetKnown),
            (Some(5), Confidence::Tentative),
            (Some(19), Confidence::Tentative),
            (Some(20), Confidence::Firm),
            (Some(40), Confidence::Firm),
        ];
        for (m, want) in cases {
            assert_eq!(confidence(m, &t), want, "{m:?}");
        }
    }

    #[test]
    fn a_word_appears_exactly_when_the_club_knows_something() {
        let t = tuning();
        for m in [None, Some(0), Some(1), Some(19), Some(20)] {
            let w = hidden_word("consistency", 12.0, m, &t);
            assert_eq!(
                w.word.is_some(),
                w.confidence != Confidence::NotYetKnown,
                "{m:?}"
            );
        }
        let firm = hidden_word("injury_proneness", 16.0, Some(31), &t);
        assert_eq!(firm.word.as_deref(), Some("injury_prone"));
        assert_eq!(firm.confidence, Confidence::Firm);
    }

    #[test]
    fn each_band_edge_picks_its_word() {
        let t = tuning();
        let bands = &t.words["consistency"];
        let cases = [
            (20.0, "rarely_off"),
            (15.0, "rarely_off"),
            (14.9, "steady"),
            (11.0, "steady"),
            (10.9, "has_off_days"),
            (7.0, "has_off_days"),
            (6.9, "erratic"),
            (1.0, "erratic"),
        ];
        for (r, want) in cases {
            assert_eq!(word(r, bands), Some(want), "{r}");
        }
        let risk = &t.words["injury_proneness"];
        assert_eq!(word(15.0, risk), Some("injury_prone"));
        assert_eq!(word(10.9, risk), Some("rarely_injured"));
        assert_eq!(word(6.9, risk), Some("hardly_ever_injured"));
    }

    #[test]
    fn bands_out_of_order_or_not_ending_at_one_are_refused_by_name() {
        let mut t = tuning();
        t.words.get_mut("consistency").unwrap().swap(0, 1);
        let text = t.validate().unwrap_err().to_string();
        assert!(text.contains("consistency: band 1 starts at 15"), "{text}");
        let mut t = tuning();
        t.words.get_mut("injury_proneness").unwrap().pop();
        let text = t.validate().unwrap_err().to_string();
        assert!(
            text.contains("injury_proneness: the last band starts at 7; it must start at 1.0"),
            "{text}"
        );
    }
}
