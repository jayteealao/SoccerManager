//! The rules stage of a change run: the sensitivity rules of the content folder's
//! `sensitivity.json` (one per attribute job and per body job) and the five-match rule, each
//! judged pass, fail or not sure from its low and its high level. Every rule counts as
//! touched. The rules and the job probe they count with build only with the `sensitivity`
//! feature, so a normal build's matches pay nothing for them; such a build checks no rule and
//! says so.

use engine::data::{Content, ContentDir};

pub use crate::report::{RuleInterval, RuleRow, RulesReport};

/// The rules a change run checks, loaded before any match plays, so a refused rules file
/// stops the run with nothing done.
pub struct RuleSet {
    #[cfg(feature = "sensitivity")]
    file: engine::sensitivity::rules::SensitivityFile,
    #[cfg(feature = "sensitivity")]
    clubs: engine::sensitivity::design::Clubs,
}

/// Loads the rules of `dir`, checked against `content`.
#[cfg(feature = "sensitivity")]
pub fn load(content: &Content, dir: &ContentDir) -> anyhow::Result<RuleSet> {
    Ok(RuleSet {
        file: engine::sensitivity::load_rules(content, dir)?,
        clubs: engine::sensitivity::default_clubs(content, dir)?,
    })
}

/// A build without the `sensitivity` feature has no rules to load.
#[cfg(not(feature = "sensitivity"))]
pub fn load(_content: &Content, _dir: &ContentDir) -> anyhow::Result<RuleSet> {
    Ok(RuleSet {})
}

/// Plays and judges every rule of `set` at the size its file sets.
#[cfg(feature = "sensitivity")]
pub fn run(content: &Content, set: &RuleSet) -> RulesReport {
    use engine::sensitivity::{self, Size, five_match};

    let started = std::time::Instant::now();
    let file = &set.file;
    let (_, results) = sensitivity::run_rules(content, &set.clubs, file, None, Size::of(file));
    let five = five_match::run(
        content,
        &set.clubs,
        &file.five_match,
        file.design.seed,
        file.design.resamples,
    );
    let mut rows: Vec<RuleRow> = file
        .rules
        .iter()
        .zip(&results)
        .map(|(def, r)| RuleRow {
            job: r.job.clone(),
            statistic: r.statistic.clone(),
            levels: def.levels,
            low: Some(r.low),
            high: Some(r.high),
            mv: interval(r.mv),
            share: r.share.and_then(interval),
            roles: None,
            word: word(r.word),
        })
        .collect();
    rows.push(RuleRow {
        job: "five-match".to_string(),
        statistic: "runs of five matches in which a top player out-rates an average one"
            .to_string(),
        levels: [file.five_match.average, file.five_match.top],
        low: None,
        high: None,
        mv: None,
        share: interval(five.pooled),
        roles: Some(
            five.roles
                .iter()
                .map(|r| (format!("{:?}", r.role), r.share))
                .collect::<std::collections::BTreeMap<_, _>>(),
        ),
        word: word(five.word),
    });
    report(rows, true, started.elapsed())
}

/// A build without the `sensitivity` feature checks no rule.
#[cfg(not(feature = "sensitivity"))]
pub fn run(_content: &Content, _set: &RuleSet) -> RulesReport {
    report(Vec::new(), false, std::time::Duration::ZERO)
}

#[cfg(feature = "sensitivity")]
/// The interval of `i`, or none when a bound is not finite: the arms' outcome did not move,
/// so a share of it has no value.
fn interval(i: engine::sensitivity::words::Interval) -> Option<RuleInterval> {
    [i.est, i.lo, i.hi]
        .iter()
        .all(|v| v.is_finite())
        .then_some(RuleInterval {
            est: i.est,
            lo: i.lo,
            hi: i.hi,
        })
}

#[cfg(feature = "sensitivity")]
fn word(w: engine::sensitivity::words::Word) -> &'static str {
    use engine::sensitivity::words::Word;
    match w {
        Word::Pass => "pass",
        Word::Fail => "fail",
        Word::NotSure => "not_sure",
    }
}

/// The stage's counts over `rows`, each rule checked at its two levels.
fn report(rows: Vec<RuleRow>, built: bool, took: std::time::Duration) -> RulesReport {
    let count = |w: &str| rows.iter().filter(|r| r.word == w).count() as u32;
    RulesReport {
        checked: rows.len() as u32,
        levels: 2 * rows.len() as u32,
        failed: count("fail"),
        not_sure: count("not_sure"),
        built,
        ms: u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
        rows,
    }
}

/// The console line of the stage.
pub fn line(r: &RulesReport) -> String {
    if !r.built {
        return "rules stage: this build has no sensitivity rules; build engine-cli with \
                --features sensitivity to check them"
            .to_string();
    }
    format!(
        "rules stage: {} touched sensitivity rules checked at {} levels, {} failed, {} not sure \
         ({:.1} s)",
        r.checked,
        r.levels,
        r.failed,
        r.not_sure,
        r.ms as f64 / 1000.0
    )
}

/// The stage's table: one line per rule.
pub fn render_table(r: &RulesReport) -> String {
    let iv = |i: Option<RuleInterval>| {
        i.map_or("-".to_string(), |i| {
            format!("{:+.3} [{:+.3}, {:+.3}]", i.est, i.lo, i.hi)
        })
    };
    let fig = |v: Option<f64>| v.map_or("-".to_string(), |v| format!("{v:.4}"));
    let mut out = format!(
        "{:<17} {:>13} {:>9} {:>9} {:>26} {:>26} word\n",
        "rule", "levels", "low", "high", "move [95%]", "share [95%]"
    );
    for row in &r.rows {
        out.push_str(&format!(
            "{:<17} {:>13} {:>9} {:>9} {:>26} {:>26} {}\n",
            row.job,
            format!("{}-{}", row.levels[0], row.levels[1]),
            fig(row.low),
            fig(row.high),
            iv(row.mv),
            iv(row.share),
            row.word.replace('_', " ")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(job: &str, word: &'static str) -> RuleRow {
        RuleRow {
            job: job.to_string(),
            statistic: "pass completion share".to_string(),
            levels: [8.0, 16.0],
            low: Some(0.8),
            high: Some(0.86),
            mv: Some(RuleInterval {
                est: 0.075,
                lo: 0.06,
                hi: 0.09,
            }),
            share: None,
            roles: None,
            word,
        }
    }

    #[test]
    fn the_stage_counts_each_rule_at_two_levels_and_its_words() {
        let r = report(
            vec![
                row("passing", "pass"),
                row("vision", "fail"),
                row("flair", "not_sure"),
            ],
            true,
            std::time::Duration::from_millis(1_500),
        );
        assert_eq!((r.checked, r.levels, r.failed, r.not_sure), (3, 6, 1, 1));
        assert_eq!(
            line(&r),
            "rules stage: 3 touched sensitivity rules checked at 6 levels, 1 failed, 1 not sure \
             (1.5 s)"
        );
        let table = render_table(&r);
        assert!(table.contains("flair"), "{table}");
        assert!(table.contains("not sure"), "{table}");
        assert!(table.contains("+0.075 [+0.060, +0.090]"), "{table}");
    }

    #[test]
    fn a_build_without_the_rules_says_so() {
        let r = report(Vec::new(), false, std::time::Duration::ZERO);
        assert_eq!((r.checked, r.levels, r.failed, r.not_sure), (0, 0, 0, 0));
        assert!(line(&r).contains("--features sensitivity"), "{}", line(&r));
    }
}
