//! How a rule is judged. Per rule: the statistic at the low and the high level, the move (the
//! relative change from low to high in the job's direction), and the share (the job's effect
//! on the measured side's goal difference over the move of all jobs together between the two
//! arms). A percentile bootstrap over matches gives a 95 percent interval for each, and the
//! word follows the intervals: pass only when both are sure, fail when either is surely
//! wrong, not sure otherwise. Not sure is never a pass. A job inert in a match expects no
//! move: it passes when the move's interval lies inside the least move either way, and fails
//! when the interval of the absolute move clears it.

use serde::Serialize;

use crate::data::attributes::Direction;
use crate::rng::EngineRng;
use crate::sensitivity::design::MatchRow;
use crate::sensitivity::rules::{Expect, Measure, Thresholds};

/// The verdict on one rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Word {
    Pass,
    Fail,
    NotSure,
}

impl Word {
    pub fn name(self) -> &'static str {
        match self {
            Word::Pass => "pass",
            Word::Fail => "fail",
            Word::NotSure => "not sure",
        }
    }
}

/// An estimate and its 95 percent interval.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Interval {
    pub est: f64,
    pub lo: f64,
    pub hi: f64,
}

impl Interval {
    /// The interval of the absolute value.
    pub fn abs(self) -> Interval {
        let (lo, hi) = if self.lo <= 0.0 && self.hi >= 0.0 {
            (0.0, self.hi.max(-self.lo))
        } else {
            (
                self.lo.abs().min(self.hi.abs()),
                self.lo.abs().max(self.hi.abs()),
            )
        };
        Interval {
            est: self.est.abs(),
            lo,
            hi,
        }
    }
}

/// The weight of each row in one resample: how many times it was drawn.
type Weights = Vec<f64>;

/// The statistic of `measure` over the rows with weight, keeping those `keep` lets through.
pub fn statistic(
    measure: Measure,
    rows: &[MatchRow],
    w: &[f64],
    keep: impl Fn(&MatchRow) -> bool,
) -> f64 {
    if measure == Measure::RatingSpread {
        return spread(rows, w, keep);
    }
    let k = measure.index();
    let (mut num, mut den) = (0.0, 0.0);
    for (row, &wt) in rows.iter().zip(w) {
        if wt == 0.0 || !keep(row) {
            continue;
        }
        num += wt * row.parts[k][0];
        den += wt * row.parts[k][1];
    }
    if den == 0.0 { 0.0 } else { num / den }
}

/// The spread of a player's match rating across matches: each rating less the mean of his
/// side's ratings in that match (which takes out what every player of the match shares), its
/// standard deviation per player across the matches, averaged over the players weighted by
/// their matches.
fn spread(rows: &[MatchRow], w: &[f64], keep: impl Fn(&MatchRow) -> bool) -> f64 {
    let mut by_player: std::collections::BTreeMap<usize, (f64, f64, f64)> = Default::default();
    for (row, &wt) in rows.iter().zip(w) {
        if wt == 0.0 || !keep(row) || row.ratings.len() < 2 {
            continue;
        }
        let mean = row.ratings.iter().map(|r| r.1).sum::<f64>() / row.ratings.len() as f64;
        for &(squad, r) in &row.ratings {
            let e = r - mean;
            let s = by_player.entry(squad).or_default();
            s.0 += wt;
            s.1 += wt * e;
            s.2 += wt * e * e;
        }
    }
    let (mut total, mut n) = (0.0, 0.0);
    for (count, sum, sq) in by_player.into_values() {
        if count < 2.0 {
            continue;
        }
        let mean = sum / count;
        let var = (sq / count - mean * mean).max(0.0) * count / (count - 1.0);
        total += count * var.sqrt();
        n += count;
    }
    if n == 0.0 { 0.0 } else { total / n }
}

/// The relative move from `low` to `high` in `direction`.
pub fn relative_move(low: f64, high: f64, direction: Direction) -> f64 {
    let sign = match direction {
        Direction::Up => 1.0,
        Direction::Down => -1.0,
    };
    let base = if low.abs() > 1e-12 {
        low.abs()
    } else {
        high.abs().max(1e-12)
    };
    sign * (high - low) / base
}

/// The mean goal difference of the rows with weight that `keep` lets through.
fn mean_gd(rows: &[MatchRow], w: &[f64], keep: impl Fn(&MatchRow) -> bool) -> f64 {
    let (mut s, mut n) = (0.0, 0.0);
    for (row, &wt) in rows.iter().zip(w) {
        if wt > 0.0 && keep(row) {
            s += wt * row.gd;
            n += wt;
        }
    }
    if n == 0.0 { 0.0 } else { s / n }
}

/// A resample of `n` rows with replacement.
fn resample(n: usize, rng: &mut EngineRng) -> Weights {
    let mut w = vec![0.0; n];
    for _ in 0..n {
        w[rng.range_usize(n)] += 1.0;
    }
    w
}

/// The 2.5 and 97.5 percentiles of `values`.
fn percentiles(mut values: Vec<f64>) -> (f64, f64) {
    values.sort_by(f64::total_cmp);
    let at = |q: f64| {
        let i = ((values.len() - 1) as f64 * q).round() as usize;
        values[i]
    };
    (at(0.025), at(0.975))
}

/// What one rule asks: which factor of the run it reads, the measure, the direction and the
/// thresholds.
#[derive(Debug, Clone)]
pub struct Ask {
    pub job: String,
    pub statistic: String,
    pub measure: Measure,
    pub factor: usize,
    pub direction: Direction,
    pub thresholds: Thresholds,
    pub expect: Expect,
    /// The rule reports its word, and the full run does not fail on it.
    pub report_only: bool,
}

/// The result of one rule.
#[derive(Debug, Clone, Serialize)]
pub struct RuleResult {
    pub job: String,
    pub statistic: String,
    pub measure: Measure,
    pub low: f64,
    pub high: f64,
    #[serde(rename = "move")]
    pub mv: Interval,
    pub share: Option<Interval>,
    pub min_move: f64,
    pub ceiling: f64,
    pub expect: Expect,
    pub word: Word,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub report_only: bool,
}

/// The word of a job expected not to move: pass when the move's interval lies inside
/// `min_move` either way, fail when the absolute move's interval lies wholly at or beyond it,
/// not sure otherwise. The share is not read: an inert job carries none of the outcome.
pub fn no_move_word(mv: Interval, t: Thresholds) -> Word {
    if mv.lo > -t.min_move && mv.hi < t.min_move {
        Word::Pass
    } else if mv.abs().lo >= t.min_move {
        Word::Fail
    } else {
        Word::NotSure
    }
}

/// The word of a move and a share interval against `t`.
pub fn word(mv: Interval, share: Option<Interval>, t: Thresholds) -> Word {
    let share = share.map(Interval::abs);
    if mv.hi < t.min_move || share.is_some_and(|s| s.lo > t.ceiling) {
        return Word::Fail;
    }
    if mv.lo >= t.min_move && share.is_some_and(|s| s.hi <= t.ceiling) {
        return Word::Pass;
    }
    Word::NotSure
}

/// Judges every rule of `asks` on the design rows and, when given, the two arms (all low,
/// all high), with `resamples` bootstrap resamples from `seed`.
pub fn judge(
    asks: &[Ask],
    design: &[MatchRow],
    arms: Option<(&[MatchRow], &[MatchRow])>,
    resamples: u32,
    seed: u64,
) -> Vec<RuleResult> {
    let one = vec![1.0; design.len()];
    let estimate =
        |w: &[f64], arm_w: Option<(&[f64], &[f64])>| -> Vec<(f64, f64, f64, Option<f64>)> {
            let outcome = arms
                .zip(arm_w)
                .map(|((lo, hi), (wl, wh))| mean_gd(hi, wh, |_| true) - mean_gd(lo, wl, |_| true));
            asks.iter()
                .map(|a| {
                    let low = statistic(a.measure, design, w, |r| !r.high[a.factor]);
                    let high = statistic(a.measure, design, w, |r| r.high[a.factor]);
                    let mv = relative_move(low, high, a.direction);
                    let share = outcome.map(|all| {
                        let effect = mean_gd(design, w, |r| r.high[a.factor])
                            - mean_gd(design, w, |r| !r.high[a.factor]);
                        if all.abs() > 1e-12 {
                            effect / all
                        } else {
                            f64::INFINITY
                        }
                    });
                    (low, high, mv, share)
                })
                .collect()
        };
    let arm_one = arms.map(|(lo, hi)| (vec![1.0; lo.len()], vec![1.0; hi.len()]));
    let point = estimate(
        &one,
        arm_one.as_ref().map(|(a, b)| (a.as_slice(), b.as_slice())),
    );
    let mut rng = EngineRng::from_seed(seed);
    let mut moves = vec![Vec::with_capacity(resamples as usize); asks.len()];
    let mut shares = vec![Vec::with_capacity(resamples as usize); asks.len()];
    for _ in 0..resamples {
        let w = resample(design.len(), &mut rng);
        let aw = arms.map(|(lo, hi)| (resample(lo.len(), &mut rng), resample(hi.len(), &mut rng)));
        let est = estimate(&w, aw.as_ref().map(|(a, b)| (a.as_slice(), b.as_slice())));
        for (k, (_, _, mv, share)) in est.into_iter().enumerate() {
            moves[k].push(mv);
            if let Some(s) = share {
                shares[k].push(s);
            }
        }
    }
    asks.iter()
        .enumerate()
        .map(|(k, a)| {
            let (low, high, mv, share) = point[k];
            let (mlo, mhi) = percentiles(std::mem::take(&mut moves[k]));
            let mv = Interval {
                est: mv,
                lo: mlo,
                hi: mhi,
            };
            let share = share.map(|s| {
                let (lo, hi) = percentiles(std::mem::take(&mut shares[k]));
                Interval { est: s, lo, hi }
            });
            RuleResult {
                job: a.job.clone(),
                statistic: a.statistic.clone(),
                measure: a.measure,
                low,
                high,
                mv,
                share,
                min_move: a.thresholds.min_move,
                ceiling: a.thresholds.ceiling,
                expect: a.expect,
                word: match a.expect {
                    Expect::Move => word(mv, share, a.thresholds),
                    Expect::NoMove => no_move_word(mv, a.thresholds),
                },
                report_only: a.report_only,
            }
        })
        .collect()
}

/// The result of one rule judged at three levels: the low-to-high result, as at two levels,
/// with the statistic at the middle level and the two steps, low to middle and middle to
/// high, each a relative move in the job's direction with its interval.
#[derive(Debug, Clone, Serialize)]
pub struct LevelResult {
    #[serde(flatten)]
    pub result: RuleResult,
    pub mid: f64,
    pub steps: [Interval; 2],
}

/// The word of a rule at three levels: the low-to-high word, then a pass becomes fail when a
/// step's interval lies wholly against the job's direction, and not sure when a step's interval
/// crosses zero against it. A job expected not to move keeps its no-move word.
pub fn level_word(base: Word, expect: Expect, steps: [Interval; 2]) -> Word {
    if expect == Expect::NoMove || base != Word::Pass {
        return base;
    }
    if steps.iter().any(|s| s.hi < 0.0) {
        Word::Fail
    } else if steps.iter().any(|s| s.lo < 0.0) {
        Word::NotSure
    } else {
        Word::Pass
    }
}

/// `w` with every row whose level of `factor` is not `level` set to zero weight.
fn at_level(w: &[f64], levels: &[Vec<u8>], factor: usize, level: u8) -> Vec<f64> {
    w.iter()
        .zip(levels)
        .map(|(&wt, l)| if l[factor] == level { wt } else { 0.0 })
        .collect()
}

/// One rule's point figures in one resample: the statistic at each level, the low-to-high
/// move, the two steps, and the share.
type LevelPoint = ([f64; 3], f64, [f64; 2], Option<f64>);

/// Judges every rule of `asks` on the three-level design rows, whose levels `levels` holds
/// per row, and, when given, the two arms (all low, all high), with `resamples` bootstrap
/// resamples from `seed`. The move and the share read the low and the high level, as
/// [`judge`] does; the steps read the middle level too.
pub fn judge_levels(
    asks: &[Ask],
    design: &[MatchRow],
    levels: &[Vec<u8>],
    arms: Option<(&[MatchRow], &[MatchRow])>,
    resamples: u32,
    seed: u64,
) -> Vec<LevelResult> {
    let estimate = |w: &[f64], arm_w: Option<(&[f64], &[f64])>| -> Vec<LevelPoint> {
        let outcome = arms
            .zip(arm_w)
            .map(|((lo, hi), (wl, wh))| mean_gd(hi, wh, |_| true) - mean_gd(lo, wl, |_| true));
        asks.iter()
            .map(|a| {
                let w_at: Vec<Vec<f64>> =
                    (0..3u8).map(|l| at_level(w, levels, a.factor, l)).collect();
                let s = |l: usize| statistic(a.measure, design, &w_at[l], |_| true);
                let stat = [s(0), s(1), s(2)];
                let mv = relative_move(stat[0], stat[2], a.direction);
                let steps = [
                    relative_move(stat[0], stat[1], a.direction),
                    relative_move(stat[1], stat[2], a.direction),
                ];
                let share = outcome.map(|all| {
                    let effect =
                        mean_gd(design, &w_at[2], |_| true) - mean_gd(design, &w_at[0], |_| true);
                    if all.abs() > 1e-12 {
                        effect / all
                    } else {
                        f64::INFINITY
                    }
                });
                (stat, mv, steps, share)
            })
            .collect()
    };
    let one = vec![1.0; design.len()];
    let arm_one = arms.map(|(lo, hi)| (vec![1.0; lo.len()], vec![1.0; hi.len()]));
    let point = estimate(
        &one,
        arm_one.as_ref().map(|(a, b)| (a.as_slice(), b.as_slice())),
    );
    let mut rng = EngineRng::from_seed(seed);
    let n = asks.len();
    let mut moves = vec![Vec::with_capacity(resamples as usize); n];
    let mut steps = vec![[Vec::new(), Vec::new()]; n];
    let mut shares = vec![Vec::with_capacity(resamples as usize); n];
    for _ in 0..resamples {
        let w = resample(design.len(), &mut rng);
        let aw = arms.map(|(lo, hi)| (resample(lo.len(), &mut rng), resample(hi.len(), &mut rng)));
        let est = estimate(&w, aw.as_ref().map(|(a, b)| (a.as_slice(), b.as_slice())));
        for (k, (_, mv, st, share)) in est.into_iter().enumerate() {
            moves[k].push(mv);
            steps[k][0].push(st[0]);
            steps[k][1].push(st[1]);
            if let Some(s) = share {
                shares[k].push(s);
            }
        }
    }
    asks.iter()
        .enumerate()
        .map(|(k, a)| {
            let (stat, mv, st, share) = point[k];
            let interval = |est: f64, values: Vec<f64>| {
                let (lo, hi) = percentiles(values);
                Interval { est, lo, hi }
            };
            let mv = interval(mv, std::mem::take(&mut moves[k]));
            let step_intervals = [
                interval(st[0], std::mem::take(&mut steps[k][0])),
                interval(st[1], std::mem::take(&mut steps[k][1])),
            ];
            let share = share.map(|s| interval(s, std::mem::take(&mut shares[k])));
            let base = match a.expect {
                Expect::Move => word(mv, share, a.thresholds),
                Expect::NoMove => no_move_word(mv, a.thresholds),
            };
            LevelResult {
                result: RuleResult {
                    job: a.job.clone(),
                    statistic: a.statistic.clone(),
                    measure: a.measure,
                    low: stat[0],
                    high: stat[2],
                    mv,
                    share,
                    min_move: a.thresholds.min_move,
                    ceiling: a.thresholds.ceiling,
                    expect: a.expect,
                    word: level_word(base, a.expect, step_intervals),
                    report_only: a.report_only,
                },
                mid: stat[1],
                steps: step_intervals,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: Thresholds = Thresholds {
        min_move: 0.05,
        ceiling: 0.20,
    };

    /// A design of `n` rows with one factor alternating, the pass completion parts of each
    /// level drawn around `low` and `high` (out of 40 passes), and a goal difference of `gd_low`
    /// and `gd_high` plus noise.
    fn rows(n: usize, low: f64, high: f64, gd: [f64; 2], noise: f64, seed: u64) -> Vec<MatchRow> {
        let mut rng = EngineRng::from_seed(seed);
        (0..n)
            .map(|m| {
                let hi = m % 2 == 1;
                let p = if hi { high } else { low };
                let completed = (0..40).filter(|_| rng.next_f64() < p).count() as f64;
                let mut parts = vec![[0.0, 0.0]; Measure::ALL.len()];
                parts[Measure::PassCompletion.index()] = [completed, 40.0];
                parts[Measure::Lapses.index()] = [
                    if hi { 2.0 } else { 4.0 } + rng.range_f64(-noise, noise),
                    1.0,
                ];
                MatchRow {
                    high: vec![hi],
                    parts,
                    gd: gd[usize::from(hi)] + rng.range_f64(-noise, noise),
                    xgd: 0.0,
                    ratings: Vec::new(),
                }
            })
            .collect()
    }

    fn arm(n: usize, gd: f64, seed: u64) -> Vec<MatchRow> {
        let mut rng = EngineRng::from_seed(seed);
        (0..n)
            .map(|_| MatchRow {
                high: Vec::new(),
                parts: vec![[0.0, 0.0]; Measure::ALL.len()],
                gd: gd + rng.range_f64(-1.0, 1.0),
                xgd: 0.0,
                ratings: Vec::new(),
            })
            .collect()
    }

    fn ask(measure: Measure, direction: Direction) -> Ask {
        Ask {
            job: "passing".into(),
            statistic: "pass completion share".into(),
            measure,
            factor: 0,
            direction,
            thresholds: T,
            expect: Expect::Move,
            report_only: false,
        }
    }

    fn one(asks: &[Ask], design: &[MatchRow], lo: &[MatchRow], hi: &[MatchRow]) -> RuleResult {
        judge(asks, design, Some((lo, hi)), 499, 7).remove(0)
    }

    #[test]
    fn a_clear_move_with_a_small_share_passes() {
        let d = rows(400, 0.6, 0.75, [0.0, 0.2], 1.0, 1);
        let r = one(
            &[ask(Measure::PassCompletion, Direction::Up)],
            &d,
            &arm(200, -2.0, 2),
            &arm(200, 2.0, 3),
        );
        assert!(r.mv.lo > 0.05, "{r:?}");
        assert_eq!(r.word, Word::Pass, "{r:?}");
    }

    #[test]
    fn no_move_fails() {
        let d = rows(400, 0.7, 0.7, [0.0, 0.0], 1.0, 4);
        let r = one(
            &[ask(Measure::PassCompletion, Direction::Up)],
            &d,
            &arm(200, -2.0, 2),
            &arm(200, 2.0, 3),
        );
        assert_eq!(r.word, Word::Fail, "{r:?}");
    }

    #[test]
    fn a_wide_interval_is_not_sure() {
        // A 5 percent move from 12 matches: the interval spans the least move.
        let d = rows(12, 0.6, 0.63, [0.0, 0.0], 0.1, 5);
        let r = one(
            &[ask(Measure::PassCompletion, Direction::Up)],
            &d,
            &arm(200, -2.0, 2),
            &arm(200, 2.0, 3),
        );
        assert_eq!(r.word, Word::NotSure, "{r:?}");
    }

    #[test]
    fn a_share_above_the_ceiling_fails() {
        let d = rows(400, 0.6, 0.75, [-1.5, 1.5], 0.5, 6);
        let r = one(
            &[ask(Measure::PassCompletion, Direction::Up)],
            &d,
            &arm(200, -2.0, 2),
            &arm(200, 2.0, 3),
        );
        assert!(r.share.unwrap().lo > 0.2, "{r:?}");
        assert_eq!(r.word, Word::Fail, "{r:?}");
    }

    #[test]
    fn a_down_job_reads_its_sign() {
        let d = rows(400, 0.6, 0.6, [0.0, 0.0], 0.5, 8);
        let lo = arm(200, -2.0, 2);
        let hi = arm(200, 2.0, 3);
        let down = one(&[ask(Measure::Lapses, Direction::Down)], &d, &lo, &hi);
        assert!(down.mv.est > 0.4, "{down:?}");
        assert_eq!(down.word, Word::Pass);
        let up = one(&[ask(Measure::Lapses, Direction::Up)], &d, &lo, &hi);
        assert_eq!(up.word, Word::Fail);
    }

    #[test]
    fn an_inert_job_passes_with_no_move_and_fails_with_a_sure_one() {
        let still = rows(400, 0.7, 0.7, [0.0, 0.0], 1.0, 4);
        let lo = arm(200, -2.0, 2);
        let hi = arm(200, 2.0, 3);
        let mut a = ask(Measure::PassCompletion, Direction::Up);
        a.expect = Expect::NoMove;
        a.report_only = true;
        let r = one(std::slice::from_ref(&a), &still, &lo, &hi);
        assert_eq!(r.word, Word::Pass, "{r:?}");
        assert!(r.report_only);
        let moved = rows(400, 0.6, 0.75, [0.0, 0.0], 1.0, 1);
        let r = one(std::slice::from_ref(&a), &moved, &lo, &hi);
        assert_eq!(r.word, Word::Fail, "{r:?}");
        let i = |lo: f64, hi: f64| Interval { est: 0.0, lo, hi };
        assert_eq!(no_move_word(i(-0.02, 0.07), T), Word::NotSure);
        assert_eq!(no_move_word(i(-0.2, -0.06), T), Word::Fail);
    }

    #[test]
    fn without_arms_a_rule_never_passes() {
        let d = rows(400, 0.6, 0.75, [0.0, 0.0], 1.0, 9);
        let r = judge(
            &[ask(Measure::PassCompletion, Direction::Up)],
            &d,
            None,
            199,
            7,
        )
        .remove(0);
        assert_eq!(r.word, Word::NotSure);
    }

    /// A three-level design of `n` rows with one factor cycling through its levels, the pass
    /// completion of each level drawn around `p[level]` out of 40 passes.
    fn level_rows(n: usize, p: [f64; 3], seed: u64) -> (Vec<MatchRow>, Vec<Vec<u8>>) {
        let mut rng = EngineRng::from_seed(seed);
        let levels: Vec<Vec<u8>> = (0..n).map(|m| vec![(m % 3) as u8]).collect();
        let rows = levels
            .iter()
            .map(|l| {
                let p = p[usize::from(l[0])];
                let completed = (0..40).filter(|_| rng.next_f64() < p).count();
                let mut parts = vec![[0.0, 0.0]; Measure::ALL.len()];
                parts[Measure::PassCompletion.index()] = [completed as f64, 40.0];
                MatchRow {
                    high: vec![l[0] == 2],
                    parts,
                    gd: if l[0] == 2 { 0.1 } else { 0.0 } + rng.range_f64(-1.0, 1.0),
                    xgd: 0.0,
                    ratings: Vec::new(),
                }
            })
            .collect();
        (rows, levels)
    }

    fn three(p: [f64; 3], n: usize, expect: Expect) -> LevelResult {
        let (d, levels) = level_rows(n, p, 11);
        let mut a = ask(Measure::PassCompletion, Direction::Up);
        a.expect = expect;
        judge_levels(
            &[a],
            &d,
            &levels,
            Some((&arm(200, -2.0, 2), &arm(200, 2.0, 3))),
            499,
            7,
        )
        .remove(0)
    }

    #[test]
    fn a_steady_rise_passes_at_three_levels() {
        let r = three([0.55, 0.65, 0.75], 1200, Expect::Move);
        assert!(r.steps.iter().all(|s| s.lo > 0.0), "{r:?}");
        assert!((r.mid - 0.65).abs() < 0.02, "{r:?}");
        assert_eq!(r.result.word, Word::Pass, "{r:?}");
    }

    #[test]
    fn a_rise_that_turns_back_at_the_top_fails() {
        let r = three([0.55, 0.8, 0.7], 1200, Expect::Move);
        assert!(r.result.mv.lo > 0.05, "{r:?}");
        assert!(r.steps[1].hi < 0.0, "{r:?}");
        assert_eq!(r.result.word, Word::Fail, "{r:?}");
    }

    #[test]
    fn a_wide_middle_reads_not_sure() {
        // The whole rise sits in the first step; the second step is flat, so its interval
        // crosses zero.
        let r = three([0.55, 0.72, 0.72], 300, Expect::Move);
        assert!(r.result.mv.lo > 0.05, "{r:?}");
        assert!(r.steps[1].lo < 0.0 && r.steps[1].hi > 0.0, "{r:?}");
        assert_eq!(r.result.word, Word::NotSure, "{r:?}");
    }

    #[test]
    fn an_inert_job_passes_with_no_move_at_any_level() {
        let r = three([0.7, 0.7, 0.7], 1200, Expect::NoMove);
        assert_eq!(r.result.word, Word::Pass, "{r:?}");
    }

    #[test]
    fn the_level_word_only_lowers_a_pass() {
        let i = |lo: f64, hi: f64| Interval { est: 0.0, lo, hi };
        let up = [i(0.01, 0.1), i(0.02, 0.2)];
        assert_eq!(level_word(Word::Pass, Expect::Move, up), Word::Pass);
        assert_eq!(
            level_word(Word::Pass, Expect::Move, [i(-0.2, -0.01), up[1]]),
            Word::Fail
        );
        assert_eq!(
            level_word(Word::Pass, Expect::Move, [i(-0.01, 0.2), up[1]]),
            Word::NotSure
        );
        assert_eq!(level_word(Word::Fail, Expect::Move, up), Word::Fail);
        assert_eq!(
            level_word(Word::Pass, Expect::NoMove, [i(-0.2, -0.1), up[1]]),
            Word::Pass
        );
    }
}
