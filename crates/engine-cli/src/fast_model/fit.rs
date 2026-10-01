//! The maximum-likelihood fit of the fast model's eight parameters and its minute shares
//! from the full-engine rows of the fit batch.
//!
//! Newton steps on the Poisson likelihood of each side's goals start the five mean
//! parameters (the means of a negative binomial are those of the Poisson); coordinate ascent
//! on the likelihood of the final scores under the whole normalised table then sets the
//! dispersion, the low-score factor and the draw weight and refines the means, one
//! golden-section search at a time. No statistics crate is in the workspace, and eight
//! parameters do not need one.

use engine::modules::fast_model::{
    FastFit, FastParams, MAX_GOALS, MINUTES, covariates, score_table,
};

use super::batch::Row;

/// Fits the model to `rows`.
pub fn fit(rows: &[Row]) -> FastFit {
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
                let x = covariates(&r.kick_off, side);
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

#[cfg(test)]
mod tests {
    use super::*;
    use engine::modules::fast_model::{FastModel, FittedScoresV1, KickOff};

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
        };
        let total: f64 = truth.minute_shares.iter().sum();
        let truth = FastFit {
            minute_shares: truth.minute_shares.iter().map(|s| s / total).collect(),
            ..truth
        };
        let rows: Vec<Row> = (0..6_000u64)
            .map(|i| {
                let level = |n: u64| 40.0 + (n % 21) as f64;
                let kick_off = KickOff {
                    strength: [50.0; 2],
                    attack: [level(i), level(i / 21)],
                    defence: [level(i / 441), level(i / 11)],
                };
                let m = FittedScoresV1.play(&truth, &kick_off, i).unwrap();
                Row {
                    pairing: 0,
                    kick_off,
                    goals: m.scores,
                    goal_minutes: m
                        .events
                        .iter()
                        .filter(|e| e.kind == engine::EngineEventKind::Goal)
                        .map(|e| e.minute)
                        .collect(),
                }
            })
            .collect();
        let got = fit(&rows);
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
