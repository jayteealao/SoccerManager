//! The comparison of a paired calibration run: the off arm and the on arm of one flag,
//! band by band, and the verdict of the decision rule.
//!
//! The rule, in order: an on arm that breaks a guardrail is rejected; otherwise the arm that
//! passes more bands wins; on equal passes, the arm closer to the band centres wins when the
//! summed normalized distance differs by more than [`MARGIN`]; otherwise there is no
//! difference, and the candidate is removed. A band that starts at 0 has no centre to aim
//! for: only a value above its top adds distance.

use serde::Serialize;

use super::BandCheck;
use super::bands::Band;

/// The smallest difference in summed normalized distance that decides a tie on passes.
pub const MARGIN: f64 = 0.05;

/// The on arm's single-thread match time may be at most this multiple of the off arm's.
pub const TIME_GUARDRAIL: f64 = 1.10;

/// The band whose lower bound is a guardrail as well as a band.
const STRONGER: &str = "stronger_team_win_rate";

/// One band of one suite, in both arms.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompareRow {
    pub suite: String,
    pub band: String,
    /// The formation pairing, for a row of the formations suite.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairing: Option<String>,
    pub lo: f64,
    pub hi: f64,
    pub off: f64,
    pub on: f64,
    /// `on` minus `off`.
    pub delta: f64,
    pub off_pass: bool,
    pub on_pass: bool,
}

/// What the rule concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    /// The on arm broke a guardrail.
    OnRejected,
    OnBetter,
    OffBetter,
    NoDifference,
}

impl Verdict {
    pub fn code(self) -> &'static str {
        match self {
            Verdict::OnRejected => "on-rejected",
            Verdict::OnBetter => "on-better",
            Verdict::OffBetter => "off-better",
            Verdict::NoDifference => "no-difference",
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

/// One row per suite, realism band, and formation pairing that both arms judged. The time
/// budget is not a realism band and is left out; each arm reports its own.
pub fn compare(off: &[BandCheck], on: &[BandCheck]) -> Vec<CompareRow> {
    off.iter()
        .filter(|c| c.band != "wall_ms")
        .filter_map(|o| {
            let n = on
                .iter()
                .find(|c| c.suite == o.suite && c.band == o.band && c.pairing == o.pairing)?;
            Some(CompareRow {
                suite: o.suite.clone(),
                band: o.band.clone(),
                pairing: o.pairing.clone(),
                lo: o.lo,
                hi: o.hi,
                off: o.value,
                on: n.value,
                delta: engine::observe::round_to(n.value - o.value, 4),
                off_pass: o.pass,
                on_pass: n.pass,
            })
        })
        .collect()
}

/// The verdict of the decision rule.
pub fn verdict(rows: &[CompareRow], off: &Guard, on: &Guard) -> Verdict {
    let slower = on.match_wall_ms as f64 > off.match_wall_ms as f64 * TIME_GUARDRAIL;
    let weaker = rows.iter().any(|r| r.band == STRONGER && !r.on_pass);
    if on.dark_paths > 0 || on.violations > 0 || slower || weaker {
        return Verdict::OnRejected;
    }
    let passes = |arm: fn(&CompareRow) -> bool| rows.iter().filter(|r| arm(r)).count();
    let (off_passes, on_passes) = (passes(|r| r.off_pass), passes(|r| r.on_pass));
    if on_passes != off_passes {
        return if on_passes > off_passes {
            Verdict::OnBetter
        } else {
            Verdict::OffBetter
        };
    }
    let (off_d, on_d) = (distance(rows, |r| r.off), distance(rows, |r| r.on));
    if off_d - on_d > MARGIN {
        Verdict::OnBetter
    } else if on_d - off_d > MARGIN {
        Verdict::OffBetter
    } else {
        Verdict::NoDifference
    }
}

/// The summed distance of one arm to the band centres, each in half-widths of its band.
/// The stronger team's win rate has a lower bound only, so it has no centre and counts as a
/// pass and a guardrail, not as a distance. A floor band (0 to `hi`) adds only the excess
/// above `hi`, in widths of the band, so a passing 0 adds nothing.
fn distance(rows: &[CompareRow], value: fn(&CompareRow) -> f64) -> f64 {
    rows.iter()
        .filter(|r| r.band != STRONGER && r.hi > r.lo)
        .map(|r| {
            let v = value(r);
            let band = Band { lo: r.lo, hi: r.hi };
            if band.is_floor() {
                (v - r.hi).max(0.0) / r.hi
            } else {
                (v - (r.lo + r.hi) / 2.0).abs() / ((r.hi - r.lo) / 2.0)
            }
        })
        .sum()
}

/// A fixed-width table of the rows and the verdict, for a person reading the terminal.
pub fn render_table(flag: &str, rows: &[CompareRow], verdict: Verdict) -> String {
    let mut out = format!("paired run of flag {flag}\n");
    out.push_str(&format!(
        "{:<10} {:<23} {:>19} {:>9} {:>9} {:>9}  {}\n",
        "suite", "band", "band range", "off", "on", "delta", "pairing"
    ));
    for r in rows {
        let mark = |pass: bool| if pass { ' ' } else { '*' };
        out.push_str(&format!(
            "{:<10} {:<23} {:>19} {:>8.3}{} {:>8.3}{} {:>+9.3}  {}\n",
            r.suite,
            r.band,
            format!("{:.3} to {:.3}", r.lo, r.hi),
            r.off,
            mark(r.off_pass),
            r.on,
            mark(r.on_pass),
            r.delta,
            r.pairing.as_deref().unwrap_or("")
        ));
    }
    out.push_str("* outside the band\n");
    out.push_str(&format!("verdict: {}\n", verdict.code()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(band: &str, value: f64, lo: f64, hi: f64) -> BandCheck {
        BandCheck {
            band: band.into(),
            suite: if band == STRONGER {
                "strength"
            } else {
                "equal"
            }
            .into(),
            pairing: None,
            value,
            lo,
            hi,
            pass: if band == STRONGER {
                value > lo
            } else {
                (lo..=hi).contains(&value)
            },
            se: 0.0,
            informational: None,
        }
    }

    /// Goals and shots per arm, with possession and the stronger team in band in both.
    fn rows(off: (f64, f64), on: (f64, f64)) -> Vec<CompareRow> {
        let arm = |(goals, shots): (f64, f64)| {
            vec![
                check("goals_per_match", goals, 2.4, 3.2),
                check("shots_per_team", shots, 8.0, 16.0),
                check("possession_home_pct", 50.0, 35.0, 65.0),
                check("possession_away_pct", 50.0, 35.0, 65.0),
                check(STRONGER, 0.6, 0.5, 1.0),
                check("wall_ms", 10.0, 0.0, 100.0),
            ]
        };
        compare(&arm(off), &arm(on))
    }

    const CLEAN: Guard = Guard {
        dark_paths: 0,
        violations: 0,
        match_wall_ms: 100,
    };

    #[test]
    fn the_rows_pair_every_realism_band_and_leave_out_the_time_budget() {
        let r = rows((2.8, 12.0), (3.0, 12.0));
        let names: Vec<&str> = r.iter().map(|r| r.band.as_str()).collect();
        assert_eq!(
            names,
            [
                "goals_per_match",
                "shots_per_team",
                "possession_home_pct",
                "possession_away_pct",
                STRONGER
            ]
        );
        assert_eq!(r[0].off, 2.8);
        assert_eq!(r[0].on, 3.0);
        assert_eq!(r[0].delta, 0.2);
    }

    #[test]
    fn a_guardrail_breach_rejects_the_on_arm() {
        let r = rows((2.8, 12.0), (2.8, 12.0));
        let dark = Guard {
            dark_paths: 1,
            ..CLEAN
        };
        assert_eq!(verdict(&r, &CLEAN, &dark), Verdict::OnRejected);
        let slow = Guard {
            match_wall_ms: 111,
            ..CLEAN
        };
        assert_eq!(verdict(&r, &CLEAN, &slow), Verdict::OnRejected);
        let mut weak = r.clone();
        weak[4].on_pass = false;
        assert_eq!(verdict(&weak, &CLEAN, &CLEAN), Verdict::OnRejected);
    }

    #[test]
    fn more_bands_passed_wins() {
        // The off arm misses the goals band; the on arm passes it.
        let r = rows((3.5, 12.0), (3.0, 12.0));
        assert_eq!(verdict(&r, &CLEAN, &CLEAN), Verdict::OnBetter);
        let r = rows((3.0, 12.0), (3.5, 12.0));
        assert_eq!(verdict(&r, &CLEAN, &CLEAN), Verdict::OffBetter);
    }

    #[test]
    fn equal_passes_fall_to_the_distance_and_its_margin() {
        // Goals centre 2.8, half-width 0.4. 0.1 goals is 0.25 of a half-width.
        let r = rows((3.0, 12.0), (2.9, 12.0));
        assert_eq!(verdict(&r, &CLEAN, &CLEAN), Verdict::OnBetter);
        // 0.01 goals is 0.025 of a half-width: inside the margin.
        let r = rows((2.81, 12.0), (2.82, 12.0));
        assert_eq!(verdict(&r, &CLEAN, &CLEAN), Verdict::NoDifference);
    }

    #[test]
    fn the_margin_boundary_is_not_a_difference() {
        // Shots centre 12, half-width 4: 0.2 shots is exactly 0.05 of a half-width.
        let r = rows((2.8, 12.2), (2.8, 12.0));
        assert_eq!(verdict(&r, &CLEAN, &CLEAN), Verdict::NoDifference);
        // 0.24 shots is 0.06: over the margin, and the on arm is closer.
        let r = rows((2.8, 12.24), (2.8, 12.0));
        assert_eq!(verdict(&r, &CLEAN, &CLEAN), Verdict::OnBetter);
    }

    #[test]
    fn a_floor_band_adds_only_the_excess_above_its_top() {
        let floor = |v: f64| {
            let arm = vec![check("ten_plus_goals_share", v, 0.0, 0.005)];
            distance(&compare(&arm, &arm), |r| r.off)
        };
        assert_eq!(floor(0.0), 0.0);
        assert_eq!(floor(0.005), 0.0);
        assert!((floor(0.01) - 1.0).abs() < 1e-12, "{}", floor(0.01));
        // The goals band keeps the distance to its centre.
        let goals = rows((3.0, 12.0), (3.0, 12.0));
        assert!((distance(&goals, |r| r.off) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn rows_of_the_formations_suite_pair_by_pairing() {
        let arm = |v: f64| {
            ["4-4-2 v 4-4-2", "4-4-2 v 4-3-3"]
                .map(|p| BandCheck {
                    suite: "formations".into(),
                    pairing: Some(p.into()),
                    ..check("goals_per_match", v, 2.4, 3.2)
                })
                .to_vec()
        };
        let mut on = arm(3.0);
        on.reverse();
        let r = compare(&arm(2.8), &on);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].pairing.as_deref(), Some("4-4-2 v 4-4-2"));
        assert_eq!(r[1].pairing.as_deref(), Some("4-4-2 v 4-3-3"));
        assert!(r.iter().all(|r| r.on == 3.0 && r.off == 2.8));
    }

    #[test]
    fn the_table_names_every_row_and_the_verdict() {
        let r = rows((3.5, 12.0), (3.0, 12.0));
        let text = render_table("probe", &r, Verdict::OnBetter);
        assert!(text.contains("goals_per_match"), "{text}");
        assert!(text.contains("2.400 to 3.200"), "{text}");
        assert!(text.contains("3.500*"), "{text}");
        assert!(text.ends_with("verdict: on-better\n"), "{text}");
    }
}
