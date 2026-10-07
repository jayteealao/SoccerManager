//! The maximum-likelihood fit of the fast model's eight parameters and its minute shares
//! from the full-engine rows of the fit batch.
//!
//! Newton steps on the Poisson likelihood of each side's goals start the five mean
//! parameters (the means of a negative binomial are those of the Poisson); coordinate ascent
//! on the likelihood of the final scores under the whole normalised table then sets the
//! dispersion, the low-score factor and the draw weight and refines the means, one
//! golden-section search at a time. No statistics crate is in the workspace, and eight
//! parameters do not need one.

use engine::modules::fast_events::{
    BINS, COUNT_TERMS, CountFit, EventFit, FitRules, SHARE_TERMS, ShareFit, SubstitutionFit,
    count_terms, share_terms, strengths,
};
use engine::modules::fast_model::{self, FastFit, FastParams, MAX_GOALS, MINUTES, score_table};

use super::batch::{Counts, Row, Timed};

/// Fits the model to `rows`, played under `rules`.
pub fn fit(rows: &[Row], rules: FitRules) -> FastFit {
    let [base, home, attack, curve, defence] = mean_params(rows);
    let params = FastParams {
        base,
        home,
        attack,
        curve,
        defence,
        dispersion: 5.0,
        rho: 0.0,
        draw: 0.0,
    };
    let params = joint(rows, params);
    FastFit {
        params,
        minute_shares: minute_shares(rows),
        events: event_fit(rows, rules),
    }
}

/// The mean parameters: intercept, home, attack, attack squared, the other side's defence.
const BETAS: usize = 5;

/// Newton steps on the Poisson log-likelihood of every side's goals.
fn mean_params(rows: &[Row]) -> [f64; BETAS] {
    let total: f64 = rows
        .iter()
        .map(|r| f64::from(r.goals[0] + r.goals[1]))
        .sum();
    let mean = (total / (2.0 * rows.len().max(1) as f64)).max(0.05);
    let mut beta = [0.0; BETAS];
    beta[0] = mean.ln();
    for _ in 0..100 {
        let mut grad = [0.0; BETAS];
        let mut info = [[0.0; BETAS]; BETAS];
        for r in rows {
            for (side, &y) in r.goals.iter().enumerate() {
                let x = fast_model::covariates(&r.kick_off, side);
                let mu = dot(&beta, &x).exp();
                for i in 0..BETAS {
                    grad[i] += (f64::from(y) - mu) * x[i];
                    for j in 0..BETAS {
                        info[i][j] += mu * x[i] * x[j];
                    }
                }
            }
        }
        let Some(step) = solve(info, grad) else {
            break;
        };
        for i in 0..BETAS {
            beta[i] += step[i];
        }
        if step.iter().all(|s| s.abs() < 1e-12) {
            break;
        }
    }
    beta
}

fn dot<const N: usize>(a: &[f64; N], b: &[f64; N]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Solves `a x = b` by Gaussian elimination with partial pivoting; `None` when singular.
fn solve<const N: usize>(mut a: [[f64; N]; N], mut b: [f64; N]) -> Option<[f64; N]> {
    for col in 0..N {
        let pivot = (col..N).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in col + 1..N {
            let f = a[row][col] / a[col][col];
            let pivot_row = a[col];
            for (cell, p) in a[row].iter_mut().zip(pivot_row).skip(col) {
                *cell -= f * p;
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = [0.0; N];
    for row in (0..N).rev() {
        let s: f64 = (row + 1..N).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - s) / a[row][row];
    }
    Some(x)
}

/// The maximum of `f` on `[lo, hi]` by golden-section search.
fn golden_max(f: impl Fn(f64) -> f64, mut lo: f64, mut hi: f64) -> f64 {
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let mut a = hi - g * (hi - lo);
    let mut b = lo + g * (hi - lo);
    let (mut fa, mut fb) = (f(a), f(b));
    for _ in 0..40 {
        if fa < fb {
            lo = a;
            a = b;
            fa = fb;
            b = lo + g * (hi - lo);
            fb = f(b);
        } else {
            hi = b;
            b = a;
            fb = fa;
            a = hi - g * (hi - lo);
            fa = f(a);
        }
        if hi - lo < 1e-9 {
            break;
        }
    }
    (lo + hi) / 2.0
}

/// The log-likelihood of every row's final score under the whole normalised table.
fn log_likelihood(rows: &[Row], params: &FastParams) -> f64 {
    rows.iter()
        .map(|r| {
            let table = score_table(params, &r.kick_off);
            let [h, a] = r.goals.map(|g| (g as usize).min(MAX_GOALS - 1));
            table[h][a].max(1e-300).ln()
        })
        .sum()
}

/// Sweeps of coordinate ascent on the whole likelihood.
const SWEEPS: usize = 4;

/// Coordinate ascent on the likelihood of the final scores: each sweep searches the
/// dispersion (on a log scale, 0.3 to 200), the low-score factor (-0.3 to 0.3), the draw
/// weight (-0.5 to 1.5) and then each mean parameter within 0.3 of its value, one at a time.
fn joint(rows: &[Row], mut params: FastParams) -> FastParams {
    type Field = fn(&mut FastParams) -> &mut f64;
    let means: [Field; 5] = [
        |p| &mut p.base,
        |p| &mut p.home,
        |p| &mut p.attack,
        |p| &mut p.curve,
        |p| &mut p.defence,
    ];
    let with = |params: &FastParams, field: Field, v: f64| {
        let mut p = *params;
        *field(&mut p) = v;
        p
    };
    for _ in 0..SWEEPS {
        let dispersion: Field = |p| &mut p.dispersion;
        let t = golden_max(
            |t| log_likelihood(rows, &with(&params, dispersion, t.exp())),
            0.3f64.ln(),
            200f64.ln(),
        );
        params.dispersion = t.exp();
        let rho: Field = |p| &mut p.rho;
        params.rho = golden_max(|v| log_likelihood(rows, &with(&params, rho, v)), -0.3, 0.3);
        let draw: Field = |p| &mut p.draw;
        params.draw = golden_max(|v| log_likelihood(rows, &with(&params, draw, v)), -0.5, 1.5);
        for field in means {
            let mut p = params;
            let now = *field(&mut p);
            let v = golden_max(
                |v| log_likelihood(rows, &with(&params, field, v)),
                now - 0.3,
                now + 0.3,
            );
            *field(&mut params) = v;
        }
    }
    params
}

/// The share of goals in each minute of regulation time.
fn minute_shares(rows: &[Row]) -> Vec<f64> {
    let mut counts = vec![0u64; MINUTES];
    for r in rows {
        for &m in &r.goal_minutes {
            counts[(m as usize).min(MINUTES - 1)] += 1;
        }
    }
    let total: u64 = counts.iter().sum();
    if total == 0 {
        return vec![1.0 / MINUTES as f64; MINUTES];
    }
    let mut shares: Vec<f64> = counts.iter().map(|&c| c as f64 / total as f64).collect();
    // Rounding can leave the sum a hair off 1; the last non-empty minute takes it.
    let sum: f64 = shares.iter().sum();
    if let Some(last) = shares.iter_mut().rev().find(|s| **s > 0.0) {
        *last += 1.0 - sum;
    }
    shares
}

/// The event fit from the full-engine rows: each count's log-linear mean by Newton steps on
/// its Poisson likelihood and its dispersion by the method of moments, each foul share and
/// share table by its frequency in the batch, and each kind's minute shares from its
/// minutes.
pub fn event_fit(rows: &[Row], rules: FitRules) -> EventFit {
    let limit = usize::from(rules.substitutions.limit);
    let fouls = |c: &[Counts; 2], side: usize| c[side].fouls;
    let played = |c: &[Counts; 2], side: usize| c[side].fouls - c[side].advantage;
    EventFit {
        fouls: count_fit(rows, &|c| c.fouls, Timed::Foul, true),
        offsides: count_fit(rows, &|c| c.offsides, Timed::Offside, true),
        corners: count_fit(rows, &|c| c.corners, Timed::Corner, true),
        throw_ins: count_fit(rows, &|c| c.throw_ins, Timed::ThrowIn, true),
        goal_kicks: count_fit(rows, &|c| c.goal_kicks, Timed::GoalKick, true),
        injuries: count_fit(rows, &|c| c.injuries, Timed::Injury, false),
        advantage: share_fit(rows, &|c, side| c[side].advantage, &fouls),
        // A penalty counts for the side that takes it, the other side from the foul.
        penalty: share_fit(rows, &|c, side| c[1 - side].penalties, &played),
        yellow: share_fit(rows, &|c, side| c[side].yellow, &fouls),
        red: share_fit(rows, &|c, side| c[side].red, &fouls),
        second_yellow: share_fit(rows, &|c, side| c[side].second_yellow, &|c, side| {
            c[side].booked_cards
        }),
        injury_stoppage: share_fit(rows, &|c, side| c[side].stopping_injuries, &|c, side| {
            c[side].injuries
        }),
        substitutions: substitution_fit(rows, limit),
        substitution_minute_shares: bin_shares(rows, Timed::Substitution),
        added_goals: [0, 1].map(|half| added_goal_share(rows, half)),
        rules,
    }
}

/// The share of the goals in the last minute of `half` (minute 44 or 89 of the score's
/// minutes, which hold the half's added time) that came in added time; 0 with no such goal.
fn added_goal_share(rows: &[Row], half: usize) -> f64 {
    let last = [44, 89][half];
    let in_last: u32 = rows
        .iter()
        .map(|r| r.goal_minutes.iter().filter(|m| **m == last).count() as u32)
        .sum();
    let added: u32 = rows.iter().map(|r| r.tally.added_goals[half]).sum();
    if in_last == 0 {
        0.0
    } else {
        (f64::from(added) / f64::from(in_last)).min(1.0)
    }
}

/// `values` scaled to sum to 1; equal shares when they sum to 0. The last non-empty value
/// takes the rounding.
fn normalised(mut values: Vec<f64>) -> Vec<f64> {
    let total: f64 = values.iter().sum();
    if total <= 0.0 {
        let n = values.len() as f64;
        return values.iter().map(|_| 1.0 / n).collect();
    }
    for v in values.iter_mut() {
        *v /= total;
    }
    let sum: f64 = values.iter().sum();
    if let Some(last) = values.iter_mut().rev().find(|v| **v > 0.0) {
        *last += 1.0 - sum;
    }
    values
}

/// The share of a timed kind's events in each minute bin.
fn bin_shares(rows: &[Row], kind: Timed) -> Vec<f64> {
    let mut counts = vec![0.0; BINS];
    for r in rows {
        for &(t, b) in &r.tally.bins {
            if t == kind {
                counts[usize::from(b).min(BINS - 1)] += 1.0;
            }
        }
    }
    normalised(counts)
}

/// One count's fit: Newton steps on the Poisson likelihood of every side's count for the
/// coefficients of the count terms, then the negative binomial's shape by the method of
/// moments (none, a Poisson count, when `dispersed` is false or the counts are not
/// overdispersed).
fn count_fit(
    rows: &[Row],
    count: &dyn Fn(&Counts) -> u32,
    kind: Timed,
    dispersed: bool,
) -> CountFit {
    let samples: Vec<([f64; COUNT_TERMS], f64, f64)> = rows
        .iter()
        .flat_map(|r| {
            (0..2).map(move |side| {
                (
                    count_terms(&r.kick_off, side),
                    f64::from(count(&r.tally.counts[side])),
                    1.0,
                )
            })
        })
        .collect();
    let beta = poisson(&samples);
    let dispersion = dispersed
        .then(|| {
            let (mut num, mut den) = (0.0, 0.0);
            for (x, y, _) in &samples {
                let mu = dot(&beta, x).exp();
                num += mu * mu;
                den += (y - mu).powi(2) - mu;
            }
            (den > 0.0 && num > 0.0).then(|| num / den)
        })
        .flatten();
    CountFit {
        coefficients: beta,
        dispersion,
        minute_shares: bin_shares(rows, kind),
    }
}

/// One foul share's fit: the Poisson likelihood of the outcome's count with the fouls it is a
/// share of as exposure, on the share terms of the side that fouls. `outcome` and `exposure`
/// take the match's counts and the fouling side.
fn share_fit(
    rows: &[Row],
    outcome: &dyn Fn(&[Counts; 2], usize) -> u32,
    exposure: &dyn Fn(&[Counts; 2], usize) -> u32,
) -> ShareFit {
    let samples: Vec<([f64; SHARE_TERMS], f64, f64)> = rows
        .iter()
        .flat_map(|r| {
            (0..2).map(move |side| {
                (
                    share_terms(&r.kick_off, side),
                    f64::from(outcome(&r.tally.counts, side)),
                    f64::from(exposure(&r.tally.counts, side)),
                )
            })
        })
        .collect();
    ShareFit {
        coefficients: poisson(&samples),
    }
}

/// The ridge on every coefficient but the intercept: it keeps a fit on a few matches finite
/// and moves a fit on thousands by far less than its error.
const RIDGE: f64 = 1e-3;

/// The coefficients of a log-linear Poisson model of `(terms, count, exposure)` samples, by
/// Newton steps on the likelihood with a small ridge; the first term is the intercept.
/// Samples with no exposure carry no information and are skipped.
fn poisson<const N: usize>(samples: &[([f64; N], f64, f64)]) -> [f64; N] {
    let samples: Vec<&([f64; N], f64, f64)> = samples.iter().filter(|s| s.2 > 0.0).collect();
    let total: f64 = samples.iter().map(|s| s.1).sum();
    let exposure: f64 = samples.iter().map(|s| s.2).sum();
    let mut beta = [0.0; N];
    beta[0] = (total / exposure.max(1.0)).max(1e-6).ln();
    if total <= 0.0 {
        return beta;
    }
    for _ in 0..100 {
        let mut grad = [0.0; N];
        let mut info = [[0.0; N]; N];
        for (x, y, e) in &samples {
            let mu = e * dot(&beta, x).exp();
            for i in 0..N {
                grad[i] += (y - mu) * x[i];
                for j in 0..N {
                    info[i][j] += mu * x[i] * x[j];
                }
            }
        }
        for i in 1..N {
            grad[i] -= RIDGE * beta[i];
            info[i][i] += RIDGE;
        }
        let Some(step) = solve(info, grad) else {
            break;
        };
        let next: [f64; N] = std::array::from_fn(|i| beta[i] + step[i]);
        if next.iter().any(|v| !v.is_finite()) {
            break;
        }
        beta = next;
        if step.iter().all(|s| s.abs() < 1e-12) {
            break;
        }
    }
    beta
}

/// The substitution fit: Newton steps on the likelihood of every side's count of
/// substitutions, those an injury forced among them, under the tilted table of
/// [`SubstitutionFit`]. The
/// parameters are the weights of counts 1 to `limit`, then `own` and `other`.
fn substitution_fit(rows: &[Row], limit: usize) -> SubstitutionFit {
    let samples: Vec<([f64; 2], usize)> = rows
        .iter()
        .flat_map(|r| {
            (0..2).map(move |side| {
                let n = r.tally.counts[side].substitutions as usize;
                (strengths(&r.kick_off, side), n.min(limit))
            })
        })
        .collect();
    let size = limit + 2;
    let mut theta = vec![0.0; size];
    let fit_of = |theta: &[f64]| SubstitutionFit {
        weights: std::iter::once(0.0)
            .chain(theta[..limit].iter().copied())
            .collect(),
        own: theta[limit],
        other: theta[limit + 1],
    };
    // The statistic of count `k` for a side at strengths `x`: count k's indicator (none for
    // count 0), then `k × x`.
    let stat = |k: usize, x: &[f64; 2]| -> Vec<f64> {
        let mut u = vec![0.0; size];
        if k > 0 {
            u[k - 1] = 1.0;
        }
        u[limit] = k as f64 * x[0];
        u[limit + 1] = k as f64 * x[1];
        u
    };
    for _ in 0..100 {
        let current = fit_of(&theta);
        let mut grad = vec![0.0; size];
        let mut info = vec![vec![0.0; size]; size];
        for (x, n) in &samples {
            let p = current.shares_at(*x);
            let stats: Vec<Vec<f64>> = (0..=limit).map(|k| stat(k, x)).collect();
            let mean: Vec<f64> = (0..size)
                .map(|i| (0..=limit).map(|k| p[k] * stats[k][i]).sum())
                .collect();
            for i in 0..size {
                grad[i] += stats[*n][i] - mean[i];
                for j in 0..size {
                    let second: f64 = (0..=limit).map(|k| p[k] * stats[k][i] * stats[k][j]).sum();
                    info[i][j] += second - mean[i] * mean[j];
                }
            }
        }
        for i in 0..size {
            grad[i] -= RIDGE * theta[i];
            info[i][i] += RIDGE;
        }
        let Some(step) = solve_dyn(info, grad) else {
            break;
        };
        if step.iter().any(|v| !v.is_finite()) {
            break;
        }
        for (t, s) in theta.iter_mut().zip(&step) {
            *t += s;
        }
        if step.iter().all(|s| s.abs() < 1e-10) {
            break;
        }
    }
    fit_of(&theta)
}

/// Solves `a x = b` for a system whose size is known only at run time.
fn solve_dyn(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in col + 1..n {
            let f = a[row][col] / a[col][col];
            let pivot_row = a[col].clone();
            for (cell, p) in a[row].iter_mut().zip(pivot_row).skip(col) {
                *cell -= f * p;
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let s: f64 = (row + 1..n).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - s) / a[row][row];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::modules::fast_model::{FastModel, FittedScoresV1, KickOff};

    use crate::fast_model::batch::tally;

    /// 6 000 rows drawn from a known model across strengths: the fit recovers it.
    #[test]
    fn the_fit_recovers_known_parameters() {
        let truth = FastFit {
            params: FastParams {
                base: -0.29,
                home: 0.04,
                attack: 1.25,
                curve: 0.15,
                defence: -0.2,
                dispersion: 5.0,
                rho: -0.1,
                draw: 0.3,
            },
            minute_shares: (0..MINUTES)
                .map(|m| (1.0 + m as f64 / 90.0) / 135.5)
                .collect(),
            events: EventFit::plain(FitRules::standard()),
        };
        let total: f64 = truth.minute_shares.iter().sum();
        let truth = FastFit {
            minute_shares: truth.minute_shares.iter().map(|s| s / total).collect(),
            ..truth
        };
        let rows: Vec<Row> = (0..6_000u64)
            .map(|i| {
                let level = |n: u64| 8.0 + (n % 21) as f64 / 5.0;
                let kick_off = KickOff {
                    attack: [level(i), level(i / 21)],
                    defence: [level(i / 441), level(i / 11)],
                    ..KickOff::even([10.0; 2])
                };
                let m = FittedScoresV1.play(&truth, &kick_off, i).unwrap();
                Row {
                    pairing: 0,
                    kick_off,
                    goals: m.scores,
                    tally: tally(&m.events),
                    goal_minutes: m
                        .events
                        .iter()
                        .filter(|e| e.kind == engine::EngineEventKind::Goal)
                        .map(|e| e.minute)
                        .collect(),
                }
            })
            .collect();
        let got = fit(&rows, FitRules::standard());
        let (t, g) = (truth.params, got.params);
        for (name, a, b) in [
            ("base", t.base, g.base),
            ("home", t.home, g.home),
            ("attack", t.attack, g.attack),
            ("curve", t.curve, g.curve),
            ("defence", t.defence, g.defence),
            ("rho", t.rho, g.rho),
            ("draw", t.draw, g.draw),
            ("1/dispersion", 1.0 / t.dispersion, 1.0 / g.dispersion),
        ] {
            assert!((a - b).abs() < 0.05, "{name}: truth {a}, fit {b}");
        }
        let sum: f64 = got.minute_shares.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
        assert!(got.minute_shares[89] > got.minute_shares[0]);
    }

    /// 20 000 fast matches drawn from known event rates, each side's strength from 35 to 65
    /// on its own: the event fit recovers every count's seven coefficients within 0.06, the
    /// foul shares' coefficients within 0.06 (0.12 for the rare penalty and red, 0.3 for the
    /// rarer second yellow), and the substitution tilt within 0.12.
    #[test]
    fn the_event_fit_recovers_known_rates() {
        let mut truth = FastFit {
            params: FastParams {
                base: 0.1,
                home: 0.0,
                attack: 0.3,
                curve: 0.0,
                defence: 0.0,
                dispersion: 5.0,
                rho: 0.0,
                draw: 0.0,
            },
            minute_shares: vec![1.0 / MINUTES as f64; MINUTES],
            events: EventFit::plain(FitRules::standard()),
        };
        let e = &mut truth.events;
        // The terms: 1, home, own, other, own², other², own × other.
        e.fouls.coefficients = [2.0, -0.08, 0.07, 0.7, 0.0, -0.2, 0.0];
        e.corners.coefficients = [0.8, 0.0, 1.1, 0.2, -0.4, 0.0, -0.3];
        e.goal_kicks.coefficients = [2.2, 0.0, 0.0, 1.2, 0.0, -0.5, -0.2];
        e.throw_ins.coefficients = [2.3, 0.05, 0.38, -0.8, -0.1, 0.4, 0.4];
        e.offsides.coefficients = [-0.5, 0.1, 1.0, -0.7, 0.3, -0.5, -0.5];
        e.injuries.coefficients = [0.1f64.ln(), 0.0, 0.2, -0.1, 0.0, 0.0, 0.0];
        // The share terms of the side that fouls: 1, home, own, other.
        e.advantage.coefficients = [0.35f64.ln(), 0.0, 0.1, -0.2];
        e.penalty.coefficients = [0.02f64.ln(), 0.0, 0.0, 0.7];
        e.yellow.coefficients = [0.13f64.ln(), 0.0, 0.15, 0.0];
        e.red.coefficients = [0.015f64.ln(), 0.0, 0.1, 0.3];
        e.substitutions = SubstitutionFit {
            weights: vec![0.0, 1.0, 2.0, 3.0, 3.5, 3.0],
            own: -0.9,
            other: 0.05,
        };
        let rows: Vec<Row> = (0..20_000u64)
            .map(|i| {
                let level = |n: u64| 7.0 + (n % 31) as f64 / 5.0;
                let kick_off = KickOff::even([level(i), level(i / 31)]);
                let m = FittedScoresV1.play(&truth, &kick_off, i).unwrap();
                Row {
                    pairing: 0,
                    kick_off,
                    goals: m.scores,
                    goal_minutes: Vec::new(),
                    tally: tally(&m.events),
                }
            })
            .collect();
        let got = event_fit(&rows, FitRules::standard());
        let t = &truth.events;
        for ((name, a), (_, b)) in t.counts().into_iter().zip(got.counts()) {
            for (term, (x, y)) in a.coefficients.iter().zip(&b.coefficients).enumerate() {
                assert!(
                    (x - y).abs() < 0.06,
                    "{name} term {term}: truth {x}, fit {y}"
                );
            }
        }
        for (name, a, b, within) in [
            ("advantage", &t.advantage, &got.advantage, 0.06),
            ("penalty", &t.penalty, &got.penalty, 0.12),
            ("yellow", &t.yellow, &got.yellow, 0.06),
            ("red", &t.red, &got.red, 0.12),
            ("second_yellow", &t.second_yellow, &got.second_yellow, 0.3),
        ] {
            for (term, (x, y)) in a.coefficients.iter().zip(&b.coefficients).enumerate() {
                assert!(
                    (x - y).abs() < within,
                    "{name} term {term}: truth {x}, fit {y}"
                );
            }
        }
        let (a, b) = (&t.substitutions, &got.substitutions);
        for (term, x, y) in [("own", a.own, b.own), ("other", a.other, b.other)] {
            assert!(
                (x - y).abs() < 0.12,
                "substitutions {term}: truth {x}, fit {y}"
            );
        }
        assert!(got.check().is_ok());
    }

    #[test]
    fn the_solver_solves_and_refuses_a_singular_system() {
        let a = [
            [4.0, 1.0, 0.0, 0.0],
            [1.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 2.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let x = solve(a, [1.0, 2.0, 4.0, 3.0]).unwrap();
        assert!((4.0 * x[0] + x[1] - 1.0).abs() < 1e-12);
        assert!((x[2] - 2.0).abs() < 1e-12);
        assert!(solve([[0.0; 4]; 4], [1.0; 4]).is_none());
    }
}
