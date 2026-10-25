//! The surprises of a change run against its intent. Every verdict row, gating or report
//! only, is read against the old engine's word:
//!
//! - `unintended`: a band the intent does not name whose word is better or worse, or not
//!   sure with a direction of move;
//! - `missed`: a band the intent names whose word is equivalent, or not sure with no
//!   direction;
//! - `wrong-way`: a band the intent names whose move goes the other way. A distribution band
//!   has no up or down, so it is never wrong-way: either direction in the intent means "should
//!   move".
//!
//! A row with no old-engine word, or a rare band not judged in this run's kind, is listed as
//! not checked against the intent and is never a surprise. A confirmed setting rule whose
//! setting the change touched adds its band and direction as an expected move: a move that way
//! is no surprise, a move the other way is wrong-way, and no move is not flagged. Each surprise
//! takes the intent's note on its band; the run is finished once every surprise has one.

use serde::{Deserialize, Serialize};

use super::intent::{Direction, Intent, Note, NoteKind};
use crate::report::bands::Registry;
use crate::report::verdict::{Moved, OldWord, VerdictRow, Word};

/// What kind of surprise a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurpriseKind {
    Unintended,
    Missed,
    WrongWay,
}

impl SurpriseKind {
    pub fn code(self) -> &'static str {
        match self {
            SurpriseKind::Unintended => "unintended",
            SurpriseKind::Missed => "missed",
            SurpriseKind::WrongWay => "wrong-way",
        }
    }
}

/// A move a confirmed setting rule expects: its band, the direction, and the rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expected {
    pub band: String,
    pub direction: Direction,
    pub rule: String,
}

/// One surprise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Surprise {
    pub band: String,
    pub suite: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pairing: Option<String>,
    pub kind: SurpriseKind,
    /// The band's word against the old engine.
    pub old_word: String,
    /// The direction the band moved, when the row has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moved: Option<Direction>,
    /// The direction the intent (or a confirmed rule) gave it, and the size the intent named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intended: Option<Direction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    /// The confirmed setting rule that expected the move, when one did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_by: Option<String>,
    /// The observed change, changed engine minus old.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<f64>,
    /// The band is a whole distribution: it moves by distance, with no up or down.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub distribution: bool,
    /// The developer's note on it, from the intent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<Note>,
}

/// A row that is not checked against the intent, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotChecked {
    pub band: String,
    pub suite: String,
    pub why: String,
}

/// `calib.surprises`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Surprises {
    /// Surprises without a note.
    pub open: u32,
    /// Notes that call their surprise a bug.
    pub bug_notes: u32,
    pub rows: Vec<Surprise>,
    /// The intended or rule-expected bands that moved as expected.
    pub as_expected: Vec<String>,
    /// The confirmed setting rules' expected moves.
    pub expected: Vec<Expected>,
    pub not_checked: Vec<NotChecked>,
}

/// Why a rare band in a change run is not checked.
pub const NOT_JUDGED_HERE: &str = "not judged in a change run; not checked against the intent";
/// Why a row without an old-engine word is not checked.
pub const NO_OLD_WORD: &str = "no old-engine word; not checked against the intent";

fn direction(m: Moved) -> Direction {
    match m {
        Moved::Up => Direction::Up,
        Moved::Down => Direction::Down,
    }
}

/// The surprises of `rows` against `intent` and the confirmed rules' `expected` moves.
pub fn flag(
    rows: &[&VerdictRow],
    intent: &Intent,
    expected: &[Expected],
    registry: &Registry,
) -> Surprises {
    let mut out = Surprises {
        expected: expected.to_vec(),
        ..Surprises::default()
    };
    for r in rows {
        if r.word == Word::NotJudged {
            out.not_checked.push(NotChecked {
                band: r.band.clone(),
                suite: r.suite.clone(),
                why: NOT_JUDGED_HERE.to_string(),
            });
            continue;
        }
        let Some(old) = r.old_word else {
            out.not_checked.push(NotChecked {
                band: r.band.clone(),
                suite: r.suite.clone(),
                why: r.old_reason.as_ref().map_or_else(
                    || NO_OLD_WORD.to_string(),
                    |why| format!("{NO_OLD_WORD} ({why})"),
                ),
            });
            continue;
        };
        let distribution = registry.band(&r.band).is_some_and(|b| b.is_distribution());
        let sign = r.diff.filter(|d| *d != 0.0).map(|d| {
            if d > 0.0 {
                Direction::Up
            } else {
                Direction::Down
            }
        });
        let (moved_any, dir) = match old {
            OldWord::Better | OldWord::Worse => (true, r.moved.map(direction).or(sign)),
            OldWord::NotSure => (r.moved.is_some(), r.moved.map(direction)),
            OldWord::Equivalent => (false, None),
        };
        let intended = intent.direction_of(&r.band);
        let rule = expected.iter().find(|e| e.band == r.band);
        let target = intended.map(|i| i.direction).or(rule.map(|e| e.direction));
        let kind = match target {
            None if moved_any => Some(SurpriseKind::Unintended),
            None => None,
            Some(_) if !moved_any => intended.map(|_| SurpriseKind::Missed),
            Some(t) if !distribution && dir.is_some_and(|d| d != t) => Some(SurpriseKind::WrongWay),
            Some(_) => {
                out.as_expected.push(r.band.clone());
                None
            }
        };
        let Some(kind) = kind else { continue };
        out.rows.push(Surprise {
            band: r.band.clone(),
            suite: r.suite.clone(),
            pairing: r.pairing.clone(),
            kind,
            old_word: old.as_str().to_string(),
            moved: dir,
            intended: target,
            size: intended.and_then(|i| i.size),
            expected_by: if intended.is_none() {
                rule.map(|e| e.rule.clone())
            } else {
                None
            },
            change: r.diff,
            distribution,
            note: intent.note_on(&r.band).cloned(),
        });
    }
    out.as_expected.sort();
    out.as_expected.dedup();
    out.open = out.rows.iter().filter(|s| s.note.is_none()).count() as u32;
    out.bug_notes = out
        .rows
        .iter()
        .filter(|s| s.note.as_ref().is_some_and(|n| n.kind == NoteKind::Bug))
        .count() as u32;
    out
}

impl Surprises {
    /// Whether every surprise has its note.
    pub fn finished(&self) -> bool {
        self.open == 0
    }

    /// The part the run's headline gains: the open surprises and "run unfinished", or "run
    /// finished" with the bug notes.
    pub fn headline_part(&self) -> String {
        if !self.finished() {
            format!("; open surprises: {}; run unfinished", self.open)
        } else if self.bug_notes > 0 {
            format!("; bug notes: {}; run finished", self.bug_notes)
        } else {
            "; run finished".to_string()
        }
    }

    /// The console table: one line per surprise, then the rows not checked.
    pub fn render(&self) -> String {
        if self.rows.is_empty() && self.not_checked.is_empty() {
            return "surprises against the intent: none\n".to_string();
        }
        let mut out = format!(
            "surprises against the intent: {} ({} without a note)\n",
            self.rows.len(),
            self.open
        );
        for s in &self.rows {
            let dir = |d: Option<Direction>| d.map_or("-", Direction::code);
            out.push_str(&format!(
                "  {:<28} {:<10} {:<10} {:<10} moved {:<4} intended {:<4}{} {}\n",
                s.band,
                s.suite,
                s.kind.code(),
                s.old_word,
                dir(s.moved),
                dir(s.intended),
                if s.distribution {
                    " (a distribution: it moves by distance)"
                } else {
                    ""
                },
                s.note.as_ref().map_or_else(
                    || "no note yet".to_string(),
                    |n| format!("{}: {}", n.kind.code(), n.note)
                )
            ));
        }
        for n in &self.not_checked {
            out.push_str(&format!("  {:<28} {:<10} {}\n", n.band, n.suite, n.why));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibrate::intent::IntentBand;
    use std::path::Path;

    fn registry() -> Registry {
        Registry::load(&engine::ContentDir::at(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        ))
        .unwrap()
    }

    fn row(band: &str, old: Option<OldWord>, moved: Option<Moved>, diff: f64) -> VerdictRow {
        VerdictRow {
            band: band.to_string(),
            suite: "equal".to_string(),
            pairing: None,
            word: Word::Pass,
            value: 1.0,
            lo: 0.0,
            hi: 2.0,
            base_value: Some(1.0 - diff),
            diff: Some(diff),
            se: Some(0.01),
            smallest_shift: None,
            outside_by: None,
            real: None,
            distance: None,
            upper: None,
            noise_floor: None,
            report_only: None,
            real_note: None,
            judged: Some("paired"),
            note: None,
            accepted: None,
            pass_rule: None,
            old_word: old,
            old_reason: None,
            moved,
        }
    }

    fn intent(bands: &[(&str, Direction)]) -> Intent {
        Intent {
            bands: bands
                .iter()
                .map(|(b, d)| IntentBand {
                    band: b.to_string(),
                    direction: *d,
                    size: None,
                })
                .collect(),
            ..Intent::empty()
        }
    }

    fn kinds(s: &Surprises) -> Vec<(&str, SurpriseKind)> {
        s.rows.iter().map(|r| (r.band.as_str(), r.kind)).collect()
    }

    #[test]
    fn each_kind_of_surprise() {
        let rows = [
            row(
                "goals_per_match",
                Some(OldWord::Worse),
                Some(Moved::Up),
                0.4,
            ),
            row("fouls_per_team", Some(OldWord::Equivalent), None, 0.0),
            row(
                "corners_per_team",
                Some(OldWord::Better),
                Some(Moved::Down),
                -0.5,
            ),
            row(
                "yellow_cards_per_team",
                Some(OldWord::Equivalent),
                None,
                0.01,
            ),
        ];
        let refs: Vec<&VerdictRow> = rows.iter().collect();
        let s = flag(
            &refs,
            &intent(&[
                ("fouls_per_team", Direction::Up),
                ("corners_per_team", Direction::Up),
            ]),
            &[],
            &registry(),
        );
        assert_eq!(
            kinds(&s),
            [
                ("goals_per_match", SurpriseKind::Unintended),
                ("fouls_per_team", SurpriseKind::Missed),
                ("corners_per_team", SurpriseKind::WrongWay),
            ]
        );
        assert_eq!(s.open, 3);
        assert!(!s.finished());
        assert_eq!(s.headline_part(), "; open surprises: 3; run unfinished");
    }

    #[test]
    fn not_sure_with_a_direction_is_unintended_and_without_one_missed() {
        let rows = [
            row(
                "goals_per_match",
                Some(OldWord::NotSure),
                Some(Moved::Up),
                0.2,
            ),
            row("fouls_per_team", Some(OldWord::NotSure), None, 0.1),
            row("corners_per_team", Some(OldWord::NotSure), None, 0.1),
        ];
        let refs: Vec<&VerdictRow> = rows.iter().collect();
        let s = flag(
            &refs,
            &intent(&[("fouls_per_team", Direction::Down)]),
            &[],
            &registry(),
        );
        assert_eq!(
            kinds(&s),
            [
                ("goals_per_match", SurpriseKind::Unintended),
                ("fouls_per_team", SurpriseKind::Missed),
            ]
        );
    }

    #[test]
    fn a_better_row_without_moved_takes_the_sign_of_its_change() {
        let rows = [row("goals_per_match", Some(OldWord::Better), None, -0.3)];
        let refs: Vec<&VerdictRow> = rows.iter().collect();
        let s = flag(
            &refs,
            &intent(&[("goals_per_match", Direction::Up)]),
            &[],
            &registry(),
        );
        assert_eq!(kinds(&s), [("goals_per_match", SurpriseKind::WrongWay)]);
        assert_eq!(s.rows[0].moved, Some(Direction::Down));
        let s = flag(
            &refs,
            &intent(&[("goals_per_match", Direction::Down)]),
            &[],
            &registry(),
        );
        assert!(s.rows.is_empty());
        assert_eq!(s.as_expected, ["goals_per_match"]);
    }

    #[test]
    fn a_distribution_band_is_never_wrong_way() {
        let r = registry();
        let dist = r
            .names()
            .into_iter()
            .find(|n| r.band(n).unwrap().is_distribution())
            .unwrap()
            .to_string();
        let rows = [row(&dist, Some(OldWord::Worse), Some(Moved::Down), 0.1)];
        let refs: Vec<&VerdictRow> = rows.iter().collect();
        let s = flag(&refs, &intent(&[(&dist, Direction::Up)]), &[], &r);
        assert!(s.rows.is_empty(), "{s:?}");
        let s = flag(&refs, &Intent::empty(), &[], &r);
        assert_eq!(s.rows.len(), 1);
        assert!(s.rows[0].distribution);
    }

    #[test]
    fn no_old_word_and_a_not_judged_row_are_not_checked() {
        let mut rare = row(
            "ten_plus_goals_share",
            Some(OldWord::Worse),
            Some(Moved::Up),
            0.01,
        );
        rare.word = Word::NotJudged;
        let mut bare = row("goals_per_match", None, None, 0.2);
        bare.old_reason = Some("the old engine has no measures".into());
        let rows = [rare.clone(), bare];
        let refs: Vec<&VerdictRow> = rows.iter().collect();
        let s = flag(
            &refs,
            &intent(&[("ten_plus_goals_share", Direction::Down)]),
            &[],
            &registry(),
        );
        assert!(s.rows.is_empty(), "{s:?}");
        assert_eq!(s.not_checked.len(), 2);
        assert_eq!(s.not_checked[0].why, NOT_JUDGED_HERE);
        assert!(s.not_checked[1].why.starts_with(NO_OLD_WORD));
        assert!(s.finished());
        // The same band judged, in a milestone run, is flagged like any band.
        rare.word = Word::Pass;
        let s = flag(&[&rare], &Intent::empty(), &[], &registry());
        assert_eq!(
            kinds(&s),
            [("ten_plus_goals_share", SurpriseKind::Unintended)]
        );
    }

    #[test]
    fn a_rule_expected_move_clears_a_surprise_and_the_other_way_is_wrong_way() {
        let rules = [Expected {
            band: "goals_per_match".into(),
            direction: Direction::Up,
            rule: "sr-1".into(),
        }];
        let up = [row(
            "goals_per_match",
            Some(OldWord::Worse),
            Some(Moved::Up),
            0.4,
        )];
        let s = flag(&[&up[0]], &Intent::empty(), &rules, &registry());
        assert!(s.rows.is_empty());
        assert_eq!(s.as_expected, ["goals_per_match"]);
        let down = row(
            "goals_per_match",
            Some(OldWord::Better),
            Some(Moved::Down),
            -0.4,
        );
        let s = flag(&[&down], &Intent::empty(), &rules, &registry());
        assert_eq!(kinds(&s), [("goals_per_match", SurpriseKind::WrongWay)]);
        assert_eq!(s.rows[0].expected_by.as_deref(), Some("sr-1"));
        let still = row("goals_per_match", Some(OldWord::Equivalent), None, 0.0);
        let s = flag(&[&still], &Intent::empty(), &rules, &registry());
        assert!(s.rows.is_empty());
    }

    #[test]
    fn notes_finish_the_run_and_bug_notes_are_counted() {
        let rows = [
            row(
                "goals_per_match",
                Some(OldWord::Worse),
                Some(Moved::Up),
                0.4,
            ),
            row(
                "fouls_per_team",
                Some(OldWord::Better),
                Some(Moved::Down),
                -0.4,
            ),
        ];
        let refs: Vec<&VerdictRow> = rows.iter().collect();
        let mut i = Intent::empty();
        i.notes = vec![Note {
            band: "goals_per_match".into(),
            kind: NoteKind::SideEffect,
            note: "keepers save less".into(),
            setting: None,
        }];
        let s = flag(&refs, &i, &[], &registry());
        assert_eq!(s.headline_part(), "; open surprises: 1; run unfinished");
        i.notes.push(Note {
            band: "fouls_per_team".into(),
            kind: NoteKind::Bug,
            note: "a bug".into(),
            setting: None,
        });
        let s = flag(&refs, &i, &[], &registry());
        assert!(s.finished());
        assert_eq!(s.headline_part(), "; bug notes: 1; run finished");
        let s = flag(&[], &Intent::empty(), &[], &registry());
        assert_eq!(s.headline_part(), "; run finished");
        assert!(s.render().contains("none"));
    }
}
