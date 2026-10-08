//! The sensitivity rules: proof that every attribute job and every body job moves its own
//! statistic, and that the curve makes a top player look clearly better than an average one
//! over five matches.
//!
//! One balanced design measures every job from the same matches ([`design`]); a read-only
//! probe counts each job's statistic as the match plays ([`probe`]); each rule is judged pass,
//! fail or not sure from bootstrap intervals of its move and of its share of the outcome
//! ([`words`]); and the five-match rule swaps one player for a top and an average copy
//! ([`five_match`]). The thresholds, the levels and the size of a run live in
//! `content/sensitivity.json` ([`rules`]).

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

/// Plays the rules of `file` whose jobs `only` names (every rule when `None`): the balanced
/// design over those jobs, every other job held at its middle level, and the two arms over
/// every job. Returns the rows and the judged rules.
pub fn run_rules(
    content: &Content,
    clubs: &Clubs,
    file: &SensitivityFile,
    only: Option<&[&str]>,
    size: Size,
) -> (Played, Vec<RuleResult>) {
    let all: Vec<Factor> = file.rules.iter().map(|r| Factor::of(r, content)).collect();
    let varied: Vec<usize> = file
        .rules
        .iter()
        .enumerate()
        .filter(|(_, r)| only.is_none_or(|o| o.contains(&r.job.as_str())))
        .map(|(i, _)| i)
        .collect();
    let rows = design::play_design(content, clubs, &all, &varied, size.matches, size.seed);
    let arms = size.arm_matches.map(|n| {
        (
            design::play_arm(content, clubs, &all, false, n, size.seed.wrapping_add(1)),
            design::play_arm(content, clubs, &all, true, n, size.seed.wrapping_add(2)),
        )
    });
    let body = &content.tuning.engine.contract.body;
    let asks: Vec<Ask> = varied
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            let r = &file.rules[i];
            Ask {
                job: r.job.clone(),
                statistic: r.statistic.clone(),
                measure: r.measure,
                factor: k,
                direction: file.direction(r, &content.attributes, body),
                thresholds: r.thresholds(file.defaults),
            }
        })
        .collect();
    let results = words::judge(
        &asks,
        &rows,
        arms.as_ref().map(|(lo, hi)| (lo.as_slice(), hi.as_slice())),
        size.resamples,
        size.seed,
    );
    (Played { design: rows, arms }, results)
}
