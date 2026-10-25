//! The sensitivity rules: proof that every attribute job and every body job moves its own
//! statistic, and that the curve makes a top player look clearly better than an average one
//! over five matches.
//!
//! One balanced design measures every job from the same matches ([`design`]); a read-only
//! probe counts each job's statistic as the match plays ([`probe`]); each rule is judged pass,
//! fail or not sure from bootstrap intervals of its move and of its share of the outcome
//! ([`words`]); and the five-match rule swaps one player for a top and an average copy
//! ([`five_match`]). The thresholds, the levels and the size of a run live in
//! `content/sensitivity.json` ([`rules`]). A rule with the report-only flag runs and reports
//! its word; only the rules without it can fail the full run ([`blocking`]).

pub mod design;
pub mod five_match;
pub mod probe;
pub mod rules;
pub mod words;

use crate::data::{Content, ContentDir, TEAM_A_FILE, TEAM_B_FILE};
use crate::error::EngineError;
use design::{Clubs, Factor};
use rules::{SENSITIVITY_FILE, SensitivityFile};
use words::{Ask, RuleResult};

/// The two default clubs from the content folder `dir`: club A is measured, club B plays it.
pub fn default_clubs(content: &Content, dir: &ContentDir) -> Result<Clubs, EngineError> {
    let a = content.load_team(dir, &dir.path(TEAM_A_FILE))?.value;
    let b = content.load_team(dir, &dir.path(TEAM_B_FILE))?.value;
    Ok(Clubs {
        measured: a,
        opponent: b,
    })
}

/// Loads `content/sensitivity.json` from `dir`, checked against `content`.
pub fn load_rules(content: &Content, dir: &ContentDir) -> Result<SensitivityFile, EngineError> {
    let bytes = crate::data::read_bytes(&dir.path(SENSITIVITY_FILE), SENSITIVITY_FILE)?;
    SensitivityFile::load(
        &bytes,
        SENSITIVITY_FILE,
        &content.attributes,
        &content.tuning.engine.contract.body,
    )
}

/// The size of one run of the rules.
#[derive(Debug, Clone, Copy)]
pub struct Size {
    /// Matches of the balanced design.
    pub matches: usize,
    /// Matches of each arm; `None` plays no arm, and then no rule can pass.
    pub arm_matches: Option<usize>,
    pub seed: u64,
    pub resamples: u32,
}

impl Size {
    /// The size the rules file sets.
    pub fn of(file: &SensitivityFile) -> Self {
        Self {
            matches: file.design.matches as usize,
            arm_matches: Some(file.design.arm_matches as usize),
            seed: file.design.seed,
            resamples: file.design.resamples,
        }
    }
}

/// The rows a run played: the design, and the two arms when it played them.
#[derive(Debug, Clone)]
pub struct Played {
    pub design: Vec<design::MatchRow>,
    pub arms: Option<(Vec<design::MatchRow>, Vec<design::MatchRow>)>,
}

/// The factors of a run of `file`'s rules whose jobs `only` names (every rule when `None`):
/// one factor per distinct target, in rule order, so a stage rule rides its main
/// attribute's factor; the factors the design varies (indices into the first list), in the
/// order of the first selected rule that needs each; and, per selected rule, its index in
/// the file and the place of its factor among the varied ones.
pub fn factors(
    content: &Content,
    file: &SensitivityFile,
    only: Option<&[&str]>,
) -> (Vec<Factor>, Vec<usize>, Vec<(usize, usize)>) {
    let mut all: Vec<Factor> = Vec::new();
    let of_rule: Vec<usize> = file
        .rules
        .iter()
        .map(|r| {
            let f = Factor::of(r, content);
            all.iter()
                .position(|g| g.target == f.target)
                .unwrap_or_else(|| {
                    all.push(f);
                    all.len() - 1
                })
        })
        .collect();
    let mut varied: Vec<usize> = Vec::new();
    let mut asks = Vec::new();
    for (i, r) in file.rules.iter().enumerate() {
        if !only.is_none_or(|o| o.contains(&r.job.as_str())) {
            continue;
        }
        let k = varied
            .iter()
            .position(|&f| f == of_rule[i])
            .unwrap_or_else(|| {
                varied.push(of_rule[i]);
                varied.len() - 1
            });
        asks.push((i, k));
    }
    (all, varied, asks)
}

/// Plays the rules of `file` whose jobs `only` names (every rule when `None`): the balanced
/// design over those jobs' factors, every other factor held at its middle level, and the two
/// arms over every factor. Returns the rows and the judged rules.
pub fn run_rules(
    content: &Content,
    clubs: &Clubs,
    file: &SensitivityFile,
    only: Option<&[&str]>,
    size: Size,
) -> (Played, Vec<RuleResult>) {
    let (all, varied, selected) = factors(content, file, only);
    let rows = design::play_design(content, clubs, &all, &varied, size.matches, size.seed);
    let arms = size.arm_matches.map(|n| {
        (
            design::play_arm(content, clubs, &all, false, n, size.seed.wrapping_add(1)),
            design::play_arm(content, clubs, &all, true, n, size.seed.wrapping_add(2)),
        )
    });
    let asks = asks(content, file, &selected);
    let results = words::judge(
        &asks,
        &rows,
        arms.as_ref().map(|(lo, hi)| (lo.as_slice(), hi.as_slice())),
        size.resamples,
        size.seed,
    );
    (Played { design: rows, arms }, results)
}

/// The rows a three-level run played: the design with each match's levels, and the two
/// arms when it played them.
#[derive(Debug, Clone)]
pub struct LevelPlayed {
    pub design: Vec<design::MatchRow>,
    pub levels: Vec<Vec<u8>>,
    pub arms: Option<(Vec<design::MatchRow>, Vec<design::MatchRow>)>,
}

/// Plays the rules of `file` whose jobs `only` names (every rule when `None`) at three levels:
/// the three-level design over those jobs' factors, every other factor held at its middle
/// level, and the two arms over every factor. Returns the rows and the judged rules, each with
/// its middle level and its two steps.
pub fn run_levels(
    content: &Content,
    clubs: &Clubs,
    file: &SensitivityFile,
    only: Option<&[&str]>,
    size: Size,
) -> (LevelPlayed, Vec<words::LevelResult>) {
    let (all, varied, selected) = factors(content, file, only);
    let (levels, rows) =
        design::play_levels(content, clubs, &all, &varied, size.matches, size.seed);
    let arms = size.arm_matches.map(|n| {
        (
            design::play_arm(content, clubs, &all, false, n, size.seed.wrapping_add(1)),
            design::play_arm(content, clubs, &all, true, n, size.seed.wrapping_add(2)),
        )
    });
    let asks = asks(content, file, &selected);
    let results = words::judge_levels(
        &asks,
        &rows,
        &levels,
        arms.as_ref().map(|(lo, hi)| (lo.as_slice(), hi.as_slice())),
        size.resamples,
        size.seed,
    );
    (
        LevelPlayed {
            design: rows,
            levels,
            arms,
        },
        results,
    )
}

/// What each selected rule asks, from its index in the file and its factor's place among the
/// varied ones.
fn asks(content: &Content, file: &SensitivityFile, selected: &[(usize, usize)]) -> Vec<Ask> {
    let body = &content.tuning.engine.contract.body;
    selected
        .iter()
        .map(|&(i, k)| {
            let r = &file.rules[i];
            Ask {
                job: r.job.clone(),
                statistic: r.statistic.clone(),
                measure: r.measure,
                factor: k,
                direction: file.direction(r, &content.attributes, body),
                thresholds: r.thresholds(file.defaults),
                expect: file.expect(r, &content.attributes, body),
                report_only: r.report_only,
            }
        })
        .collect()
}

/// The rules that fail a full run: those whose word is not pass and that lack the
/// report-only flag.
pub fn blocking(results: &[RuleResult]) -> Vec<&RuleResult> {
    results
        .iter()
        .filter(|r| r.word != words::Word::Pass && !r.report_only)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensitivity::rules::{Expect, Measure};
    use words::{Interval, Word};

    fn result(job: &str, word: Word, report_only: bool) -> RuleResult {
        let i = Interval {
            est: 0.0,
            lo: 0.0,
            hi: 0.0,
        };
        RuleResult {
            job: job.into(),
            statistic: "s".into(),
            measure: Measure::PassCompletion,
            low: 0.0,
            high: 0.0,
            mv: i,
            share: None,
            min_move: 0.05,
            ceiling: 0.2,
            expect: Expect::Move,
            word,
            report_only,
        }
    }

    #[test]
    fn a_failing_flagged_rule_is_not_blocking_and_a_failing_unflagged_rule_is() {
        let results = [
            result("marking", Word::Fail, true),
            result("flair", Word::NotSure, true),
            result("passing", Word::Fail, false),
            result("vision", Word::Pass, false),
            result("corners", Word::Pass, true),
        ];
        let jobs: Vec<&str> = blocking(&results).iter().map(|r| r.job.as_str()).collect();
        assert_eq!(jobs, ["passing"]);
    }

    /// A stage rule rides its main attribute's factor: the shipped rules give one factor per
    /// distinct target, every target once; the press and cover stage asks read positioning's
    /// factor, alone or with positioning's own rule.
    #[test]
    fn a_stage_rule_shares_its_main_attributes_factor() {
        let content = crate::data::test_support::shipped_content();
        let dir = crate::data::test_support::shipped_dir();
        let file = load_rules(&content, &dir).unwrap();
        let (all, varied, asks) = factors(&content, &file, None);
        assert_eq!(all.len(), file.rules.len() - 2);
        for (a, f) in all.iter().enumerate() {
            assert!(all[..a].iter().all(|g| g.target != f.target), "{}", f.job);
        }
        assert_eq!(varied.len(), all.len());
        assert_eq!(asks.len(), file.rules.len());
        let factor_of = |job: &str| {
            let (i, k) = asks
                .iter()
                .copied()
                .find(|&(i, _)| file.rules[i].job == job)
                .unwrap();
            assert_eq!(file.rules[i].job, job);
            all[varied[k]].job.clone()
        };
        assert_eq!(factor_of("press.choose"), "positioning");
        assert_eq!(factor_of("cover.execute"), "positioning");
        let (all_only, varied, asks) = factors(&content, &file, Some(&["press.choose"]));
        assert_eq!(all_only.len(), all.len());
        assert_eq!(varied.len(), 1);
        assert_eq!(all_only[varied[0]].job, "positioning");
        assert_eq!(asks.len(), 1);
        let (_, varied, asks) = factors(
            &content,
            &file,
            Some(&["positioning", "press.choose", "teamwork"]),
        );
        assert_eq!(varied.len(), 2);
        let k = |job: &str| {
            asks.iter()
                .find(|&&(i, _)| file.rules[i].job == job)
                .unwrap()
                .1
        };
        assert_eq!(k("positioning"), k("press.choose"));
        assert_ne!(k("positioning"), k("teamwork"));
    }
}
