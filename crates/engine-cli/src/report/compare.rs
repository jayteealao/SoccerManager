//! The comparison of a paired calibration run: the off arm and the on arm of one flag,
//! judged as a change run judges two engines, with the off arm in the old engine's place.
//!
//! Every band gets the on arm's word against real football (pass, fail or not sure) and its
//! word against the off arm (better, worse, equivalent or not sure), from the paired change
//! over the fixtures both arms played. The verdict, in order: an on arm that breaks a
//! guardrail is rejected; otherwise the verdict is the on arm's joint word against real
//! football, with the counts of the words against the off arm beside it.

use serde::Serialize;

use super::verdict::{OldWord, VerdictRow, Word};

/// The on arm's single-thread match time may be at most this multiple of the off arm's.
pub const TIME_GUARDRAIL: f64 = 1.10;

/// The band whose word is a guardrail as well as a word.
const STRONGER: &str = "stronger_team_win_rate";

/// What the rule concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    /// The on arm broke a guardrail.
    OnRejected,
    /// The on arm's joint word against real football.
    Pass,
    Fail,
    NotSure,
}

impl Verdict {
    pub fn code(self) -> &'static str {
        match self {
            Verdict::OnRejected => "on-rejected",
            Verdict::Pass => "pass",
            Verdict::Fail => "fail",
            Verdict::NotSure => "not-sure",
        }
    }
}

/// The guardrail figures of one arm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Guard {
    /// Missing statistics records, changes never applied, and failed workers.
    pub dark_paths: u32,
    pub violations: usize,
    /// The single-thread median match time, in milliseconds.
    pub match_wall_ms: u64,
}

/// How many bands the on arm moved which way against the off arm.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct WordCounts {
    pub better: u32,
    pub worse: u32,
    pub equivalent: u32,
    pub not_sure: u32,
}

impl WordCounts {
    /// The counts of the old-engine words over the rows that gate.
    pub fn of(rows: &[VerdictRow]) -> WordCounts {
        let mut c = WordCounts::default();
        for r in rows.iter().filter(|r| r.report_only.is_none()) {
            match r.old_word.unwrap_or(OldWord::NotSure) {
                OldWord::Better => c.better += 1,
                OldWord::Worse => c.worse += 1,
                OldWord::Equivalent => c.equivalent += 1,
                OldWord::NotSure => c.not_sure += 1,
            }
        }
        c
    }
}

/// The verdict of a paired run with its reasons and counts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PairVerdict {
    pub verdict: Verdict,
    /// The guardrails the on arm broke; empty unless it is rejected.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rejected: Vec<String>,
    /// The words against the off arm over the bands that gate.
    pub words: WordCounts,
}

/// The verdict of the decision rule over the on arm's rows, judged against the off arm, and
/// the on arm's joint word against real football.
pub fn verdict(rows: &[VerdictRow], joint: Word, off: &Guard, on: &Guard) -> PairVerdict {
    let mut rejected = Vec::new();
    if on.dark_paths > 0 {
        rejected.push(format!("{} dark paths in the on arm", on.dark_paths));
    }
    if on.violations > 0 {
        rejected.push(format!("{} rule violations in the on arm", on.violations));
    }
    if on.match_wall_ms as f64 > off.match_wall_ms as f64 * TIME_GUARDRAIL {
        rejected.push(format!(
            "the on arm's match takes {} ms against {} ms, more than {} percent slower",
            on.match_wall_ms,
            off.match_wall_ms,
            ((TIME_GUARDRAIL - 1.0) * 100.0).round()
        ));
    }
    if rows
        .iter()
        .any(|r| r.band == STRONGER && r.word == Word::Fail)
    {
        rejected.push(format!("{STRONGER} fails in the on arm"));
    }
    let verdict = if !rejected.is_empty() {
        Verdict::OnRejected
    } else {
        match joint {
            Word::Pass => Verdict::Pass,
            Word::Fail => Verdict::Fail,
            Word::NotSure => Verdict::NotSure,
        }
    };
    PairVerdict {
        verdict,
        rejected,
        words: WordCounts::of(rows),
    }
}

/// The verdict lines under the paired run's table, for a person reading the terminal.
pub fn render_verdict(flag: &str, v: &PairVerdict) -> String {
    let w = &v.words;
    let mut out = format!(
        "paired run of flag {flag}: on against off: {} better, {} worse, {} equivalent, {} not \
         sure\n",
        w.better, w.worse, w.equivalent, w.not_sure
    );
    for r in &v.rejected {
        out.push_str(&format!("  rejected: {r}\n"));
    }
    out.push_str(&format!("verdict: {}\n", v.verdict.code()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::BandCheck;
    use crate::report::verdict::range_rows;

    fn row(band: &str, word: Word, old: OldWord) -> VerdictRow {
        let check = BandCheck {
            band: band.into(),
            suite: "equal".into(),
            pairing: None,
            value: 1.0,
            lo: 0.0,
            hi: 2.0,
            pass: true,
            se: 0.0,
            informational: None,
            why_none: None,
            unjudged: false,
            whole: false,
            dist: None,
            rule: None,
        };
        let mut r = range_rows(&[check], None).remove(0);
        r.word = word;
        r.old_word = Some(old);
        r
    }

    const CLEAN: Guard = Guard {
        dark_paths: 0,
        violations: 0,
        match_wall_ms: 100,
    };

    fn rows() -> Vec<VerdictRow> {
        vec![
            row("goals_per_match", Word::Pass, OldWord::Better),
            row("shots_per_team", Word::Pass, OldWord::Equivalent),
            row("corners_per_team", Word::NotSure, OldWord::NotSure),
            row(STRONGER, Word::Pass, OldWord::Worse),
        ]
    }

    #[test]
    fn the_verdict_is_the_joint_word_with_the_counts_of_the_old_words() {
        let v = verdict(&rows(), Word::NotSure, &CLEAN, &CLEAN);
        assert_eq!(v.verdict, Verdict::NotSure);
        assert!(v.rejected.is_empty());
        assert_eq!(
            v.words,
            WordCounts {
                better: 1,
                worse: 1,
                equivalent: 1,
                not_sure: 1
            }
        );
        assert_eq!(
            verdict(&rows(), Word::Pass, &CLEAN, &CLEAN).verdict,
            Verdict::Pass
        );
        // A report-only row counts no word.
        let mut r = rows();
        r[0].report_only = Some("reported".into());
        assert_eq!(verdict(&r, Word::Fail, &CLEAN, &CLEAN).words.better, 0);
    }

    #[test]
    fn a_guardrail_breach_rejects_the_on_arm_whatever_its_words() {
        let dark = Guard {
            dark_paths: 1,
            ..CLEAN
        };
        let v = verdict(&rows(), Word::Pass, &CLEAN, &dark);
        assert_eq!(v.verdict, Verdict::OnRejected);
        assert!(v.rejected[0].contains("dark paths"), "{:?}", v.rejected);
        let slow = Guard {
            match_wall_ms: 111,
            ..CLEAN
        };
        assert_eq!(
            verdict(&rows(), Word::Pass, &CLEAN, &slow).verdict,
            Verdict::OnRejected
        );
        // Exactly 10 percent slower is inside the guardrail.
        let edge = Guard {
            match_wall_ms: 110,
            ..CLEAN
        };
        assert_eq!(
            verdict(&rows(), Word::Pass, &CLEAN, &edge).verdict,
            Verdict::Pass
        );
        let mut weak = rows();
        weak[3].word = Word::Fail;
        let v = verdict(&weak, Word::Fail, &CLEAN, &CLEAN);
        assert_eq!(v.verdict, Verdict::OnRejected);
        assert!(v.rejected[0].contains(STRONGER));
    }

    #[test]
    fn the_verdict_lines_name_the_counts_and_the_verdict() {
        let v = verdict(&rows(), Word::NotSure, &CLEAN, &CLEAN);
        let text = render_verdict("probe", &v);
        assert!(
            text.contains("1 better, 1 worse, 1 equivalent, 1 not sure"),
            "{text}"
        );
        assert!(text.ends_with("verdict: not-sure\n"), "{text}");
    }

    /// The band-centre rule is gone: no source file of this crate names its parts.
    #[test]
    fn no_source_file_keeps_the_band_centre_rule() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let names = [
            ["On", "Better"].concat(),
            ["Off", "Better"].concat(),
            ["No", "Difference"].concat(),
            ["pub const ", "MARGIN:"].concat(),
            ["off", "_pass"].concat(),
        ];
        let mut stack = vec![src];
        let mut seen = 0;
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs") {
                    continue;
                }
                seen += 1;
                let text = std::fs::read_to_string(&path).unwrap();
                for n in &names {
                    assert!(!text.contains(n.as_str()), "{} names {n}", path.display());
                }
                if path.ends_with("report/compare.rs") {
                    let f = ["fn ", "distance("].concat();
                    assert!(!text.contains(&f), "compare.rs keeps {f}");
                }
            }
        }
        assert!(seen > 20, "only {seen} source files read");
    }
}
