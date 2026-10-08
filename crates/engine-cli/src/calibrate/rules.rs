//! The rules stage of a change run: every sensitivity rule the change touches, checked at
//! every level of its setting. No sensitivity rule exists yet, so the list is empty and the
//! stage reports zero rules; the rules arrive with their own format later.

pub use crate::report::RulesReport;

/// A sensitivity rule: a setting, its levels, and the check at one level.
pub struct Rule {
    pub levels: &'static [&'static str],
    /// `true` when the rule holds at the level.
    pub check: fn(level: &str) -> bool,
}

/// Every sensitivity rule. Empty until the first rule is written.
pub const RULES: &[Rule] = &[];

/// Checks every rule of `rules` that `touched` names, at every level.
pub fn run(rules: &[Rule], touched: &dyn Fn(&Rule) -> bool) -> RulesReport {
    let mut out = RulesReport::default();
    for rule in rules.iter().filter(|r| touched(r)) {
        out.checked += 1;
        for level in rule.levels {
            out.levels += 1;
            out.failed += u32::from(!(rule.check)(level));
        }
    }
    out
}

/// The console line of the stage.
pub fn line(r: &RulesReport) -> String {
    format!(
        "rules stage: {} touched sensitivity rules checked at {} levels, {} failed",
        r.checked, r.levels, r.failed
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stage_checks_every_touched_rule_at_every_level_and_none_exist_yet() {
        assert_eq!(run(RULES, &|_| true), RulesReport::default());
        let rules = [
            Rule {
                levels: &["low", "high"],
                check: |level| level == "low",
            },
            Rule {
                levels: &["on"],
                check: |_| true,
            },
        ];
        let r = run(&rules, &|r| r.levels.len() == 2);
        assert_eq!(
            r,
            RulesReport {
                checked: 1,
                levels: 2,
                failed: 1
            }
        );
        assert_eq!(
            line(&RulesReport::default()),
            "rules stage: 0 touched sensitivity rules checked at 0 levels, 0 failed"
        );
    }
}
