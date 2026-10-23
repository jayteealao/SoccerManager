//! The rules stage of a change run: the sensitivity rules of the content folder's
//! `sensitivity.json` (one per attribute job and per body job) and the five-match rule. Only
//! the rules of the settings the change touched play ([`super::touched`]); a change to engine
//! code, or a run with no revision to compare with, plays every rule. Each job rule plays at
//! three levels, low, middle and high, and is judged pass, fail or not sure from its
//! low-to-high move and its two steps; the five-match rule keeps its two copies. The job rules
//! and the probe they count with build only with the `sensitivity` feature, so a normal build's
//! matches pay nothing for them; such a build checks no job rule and says so, and still
//! reports what the change touched. The confirmed setting rules play in any build
//! ([`super::setting_rules`]).

use engine::data::{Content, ContentDir};

use super::touched::{FIVE_MATCH, Touched};
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

/// Plays and judges the rules of `set` that `touched` selects, each job rule at three levels,
/// at the size its file sets.
#[cfg(feature = "sensitivity")]
pub fn run(content: &Content, set: &RuleSet, touched: &Touched, kind: &str) -> RulesReport {
    use engine::sensitivity::{self, Size, five_match};

    let started = std::time::Instant::now();
    let file = &set.file;
    let jobs: Vec<&str> = file
        .rules
        .iter()
        .map(|r| r.job.as_str())
        .filter(|j| touched.selects(j))
        .collect();
    let mut rows: Vec<RuleRow> = Vec::new();
    if !jobs.is_empty() {
        let only = (!touched.all).then_some(jobs.as_slice());
        let (_, results) = sensitivity::run_levels(content, &set.clubs, file, only, Size::of(file));
        for r in &results {
            let def = file
                .rules
                .iter()
                .find(|d| d.job == r.result.job)
                .expect("a result names a rule of the file");
            let mid = engine::sensitivity::design::Factor::of(def, content).middle();
            let steps = match (interval(r.steps[0]), interval(r.steps[1])) {
                (Some(a), Some(b)) => Some([a, b]),
                _ => None,
            };
            rows.push(RuleRow {
                job: r.result.job.clone(),
                statistic: r.result.statistic.clone(),
                levels: vec![def.levels[0], mid, def.levels[1]],
                low: Some(r.result.low),
                mid: Some(r.mid),
                high: Some(r.result.high),
                steps,
                mv: interval(r.result.mv),
                share: r.result.share.and_then(interval),
                roles: None,
                word: word(r.result.word).to_string(),
                report_only: r.result.report_only,
            });
        }
    }
    if touched.selects(FIVE_MATCH) {
        let five = five_match::run(
            content,
            &set.clubs,
            &file.five_match,
            file.design.seed,
            file.design.resamples,
        );
        rows.push(RuleRow {
            job: FIVE_MATCH.to_string(),
            statistic: "runs of five matches in which a top player out-rates an average one"
                .to_string(),
            levels: vec![file.five_match.average, file.five_match.top],
            low: None,
            mid: None,
            high: None,
            steps: None,
            mv: None,
            share: interval(five.pooled),
            roles: Some(
                five.roles
                    .iter()
                    .map(|r| (format!("{:?}", r.role), r.share))
                    .collect::<std::collections::BTreeMap<_, _>>(),
            ),
            word: word(five.word).to_string(),
            report_only: false,
        });
    }
    report(rows, true, started.elapsed(), touched, kind)
}

/// A build without the `sensitivity` feature checks no job rule; it still reports what the
/// change touched.
#[cfg(not(feature = "sensitivity"))]
pub fn run(_content: &Content, _set: &RuleSet, touched: &Touched, kind: &str) -> RulesReport {
    report(Vec::new(), false, std::time::Duration::ZERO, touched, kind)
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

/// The stage's counts over `rows`, each rule at its levels.
fn report(
    rows: Vec<RuleRow>,
    built: bool,
    took: std::time::Duration,
    touched: &Touched,
    kind: &str,
) -> RulesReport {
    let count = |w: &str| rows.iter().filter(|r| r.word == w).count() as u32;
    RulesReport {
        kind: kind.to_string(),
        touched: Some(touched.clone()),
        checked: rows.len() as u32,
        levels: rows.iter().map(|r| r.levels.len() as u32).sum(),
        failed: count("fail"),
        not_sure: count("not_sure"),
        built,
        ms: u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
        rows,
    }
}

/// The console line of the stage.
pub fn line(r: &RulesReport) -> String {
    let why = r
        .touched
        .as_ref()
        .map_or_else(|| "every rule".to_string(), Touched::why_line);
    if !r.built {
        return format!(
            "rules stage (kind {}): {why}; this build has no sensitivity rules; build \
             engine-cli with --features sensitivity to check them",
            r.kind
        );
    }
    format!(
        "rules stage (kind {}): {} touched rules checked at {} levels ({why}), {} failed, {} not \
         sure ({:.1} s)",
        r.kind,
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
        "{:<17} {:>16} {:>9} {:>9} {:>9} {:>26} {:>26} word\n",
        "rule", "levels", "low", "mid", "high", "move [95%]", "share [95%]"
    );
    for row in &r.rows {
        let levels = row
            .levels
            .iter()
            .map(|l| format!("{l}"))
            .collect::<Vec<_>>()
            .join("-");
        let levels = if row.job == FIVE_MATCH {
            format!("{levels} two copies")
        } else {
            levels
        };
        out.push_str(&format!(
            "{:<17} {:>16} {:>9} {:>9} {:>9} {:>26} {:>26} {}\n",
            row.job,
            levels,
            fig(row.low),
            fig(row.mid),
            fig(row.high),
            iv(row.mv),
            iv(row.share),
            if row.report_only {
                format!("{} report", row.word.replace('_', " "))
            } else {
                row.word.replace('_', " ")
            }
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibrate::touched::Reason;

    fn row(job: &str, word: &str) -> RuleRow {
        RuleRow {
            job: job.to_string(),
            statistic: "pass completion share".to_string(),
            levels: vec![8.0, 12.0, 16.0],
            low: Some(0.8),
            mid: Some(0.83),
            high: Some(0.86),
            steps: None,
            mv: Some(RuleInterval {
                est: 0.075,
                lo: 0.06,
                hi: 0.09,
            }),
            share: None,
            roles: None,
            word: word.to_string(),
            report_only: false,
        }
    }

    fn content_change() -> Touched {
        let mut t = Touched::every(Reason::Content, Some("abc".into()), None, Vec::new(), &[]);
        t.all = false;
        t.settings = vec!["attributes.json#/attributes[passing]/group".into()];
        t.rules = vec!["passing".into()];
        t
    }

    #[test]
    fn the_stage_counts_each_rule_at_three_levels_and_its_words() {
        let r = report(
            vec![
                row("passing", "pass"),
                row("vision", "fail"),
                RuleRow {
                    report_only: true,
                    ..row("flair", "not_sure")
                },
                RuleRow {
                    levels: vec![10.0, 16.0],
                    ..row(FIVE_MATCH, "pass")
                },
            ],
            true,
            std::time::Duration::from_millis(1_500),
            &content_change(),
            "change",
        );
        assert_eq!((r.checked, r.levels, r.failed, r.not_sure), (4, 11, 1, 1));
        assert_eq!(
            line(&r),
            "rules stage (kind change): 4 touched rules checked at 11 levels (1 changed \
             settings, 0 with no rule), 1 failed, 1 not sure (1.5 s)"
        );
        let table = render_table(&r);
        assert!(table.contains("flair"), "{table}");
        assert!(table.contains("not sure report"), "{table}");
        assert!(table.contains("8-12-16"), "{table}");
        assert!(table.contains("10-16 two copies"), "{table}");
        let json = serde_json::to_value(&r.rows[2]).unwrap();
        assert_eq!(json["report_only"], true);
        assert_eq!(json["levels"], serde_json::json!([8.0, 12.0, 16.0]));
        assert_eq!(json["mid"], 0.83);
        assert!(
            serde_json::to_value(&r.rows[0])
                .unwrap()
                .get("report_only")
                .is_none()
        );
        assert!(table.contains("+0.075 [+0.060, +0.090]"), "{table}");
        let back: RulesReport = serde_json::from_value(serde_json::to_value(&r).unwrap()).unwrap();
        assert_eq!(back.rows, r.rows);
        assert_eq!(back.touched.unwrap().rules, ["passing"]);
    }

    #[test]
    fn a_build_without_the_rules_says_so_and_names_the_touched_settings() {
        let r = report(
            Vec::new(),
            false,
            std::time::Duration::ZERO,
            &content_change(),
            "milestone",
        );
        assert_eq!((r.checked, r.levels, r.failed, r.not_sure), (0, 0, 0, 0));
        let l = line(&r);
        assert!(l.contains("--features sensitivity"), "{l}");
        assert!(
            l.starts_with("rules stage (kind milestone): 1 changed settings"),
            "{l}"
        );
        let every = Touched::every(Reason::Code, None, None, vec!["crates/x.rs".into()], &[]);
        let r = report(
            Vec::new(),
            false,
            std::time::Duration::ZERO,
            &every,
            "change",
        );
        assert!(line(&r).contains("engine code changed: 1"), "{}", line(&r));
    }
}
