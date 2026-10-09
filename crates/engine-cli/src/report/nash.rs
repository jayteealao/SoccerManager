//! The formation field rating of the formations suite: each formation's win share against
//! the field as its maximum-entropy Nash equilibrium weighs it, beside its plain win rate
//! (the field with equal weights). It only reports: no band, verdict, count or exit code
//! reads it.
//!
//! Nash averaging is defined by Balduzzi, Tuyls, Perolat and Graepel, "Re-evaluating
//! Evaluation", NeurIPS 2018 (arXiv:1806.02643): for an antisymmetric payoff table `A`, the
//! rating is `A p*`, with `p*` the unique maximum-entropy Nash equilibrium of the symmetric
//! zero-sum game `A`; it splits its weight evenly over copies of one strategy, so a cloned
//! formation cannot inflate the field.
//!
//! The payoff here is wins minus losses, `A = W - Wᵀ`, with `W[i][j]` the share of the
//! matches between formations `i` and `j` that `i` won (a draw is not a win). The field
//! rating is the same win share as the win rate, weighted by `p*`, so the two compare on
//! one scale; the Nash average `A p*` is reported beside them.

use engine::observe::round_to;
use serde::Serialize;

/// A formation whose field rating exceeds its win rate by more than this is flagged.
pub const FLAG_MARGIN: f64 = 0.10;

/// The solve stops when no strategy gains more than this against the equilibrium and the
/// duality gap is within it.
const TOLERANCE: f64 = 1e-9;

/// The solve stops here when it has not converged; the result says so.
const ITERATION_CAP: u32 = 1_000_000;

/// The maximum-entropy Nash equilibrium of an antisymmetric table, and how the solve ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Equilibrium {
    pub p: Vec<f64>,
    pub converged: bool,
    pub iterations: u32,
}

/// The maximum-entropy Nash equilibrium of the symmetric zero-sum game `a`: the point of
/// largest entropy in `{p in the simplex : a p <= 0}`, in two steps.
///
/// 1. The support: the strategies some equilibrium plays, by one small linear programme
///    per strategy (the largest weight on it over the equilibrium set). Every equilibrium
///    of a zero-sum game earns 0 against each strategy of this support, and the
///    maximum-entropy point plays all of it, so its weights there are positive and the
///    rest are 0.
/// 2. The weights: through the dual over that support. With multipliers `mu >= 0`, `p(mu)`
///    is proportional to `exp(a mu)` on the support, and `log sum exp(a mu)` is convex in
///    `mu` with gradient `-a p(mu)` and a finite minimiser; accelerated projected gradient
///    descent with restarts minimises it. The duality gap is `-mu · a p`, so the stop rule
///    (every `(a p)[i] <= 1e-9` and the gap within 1e-9) bounds how far `p` is from the
///    optimum.
///
/// Panics when `a` is not square or not antisymmetric within 1e-12.
pub fn maxent_equilibrium(a: &[Vec<f64>]) -> Equilibrium {
    let n = a.len();
    for (i, row) in a.iter().enumerate() {
        assert_eq!(row.len(), n, "the payoff table must be square");
        for (j, x) in row.iter().enumerate() {
            assert!(
                (x + a[j][i]).abs() <= 1e-12,
                "the payoff table must be antisymmetric: [{i}][{j}] = {x}, [{j}][{i}] = {}",
                a[j][i]
            );
        }
    }
    let norm: f64 = a.iter().flatten().map(|x| x * x).sum();
    if n == 0 || norm == 0.0 {
        // Every strategy ties: the uniform field is the maximum-entropy equilibrium.
        return Equilibrium {
            p: vec![1.0 / n.max(1) as f64; n],
            converged: true,
            iterations: 0,
        };
    }
    let support = support(a);
    // The Hessian of the dual is aᵀ (diag(p) - p pᵀ) a, whose norm is at most the squared
    // Frobenius norm of `a`: a safe fixed step.
    let step = 1.0 / norm;
    let mut mu = vec![0.0; n];
    let mut y = mu.clone();
    let mut t = 1.0_f64;
    for iteration in 0..ITERATION_CAP {
        let p = weights(a, &mu, &support);
        let ap = mat_vec(a, &p);
        let gap: f64 = -mu.iter().zip(&ap).map(|(m, x)| m * x).sum::<f64>();
        if ap.iter().all(|&x| x <= TOLERANCE) && gap.abs() <= TOLERANCE {
            return Equilibrium {
                p,
                converged: true,
                iterations: iteration,
            };
        }
        let apy = mat_vec(a, &weights(a, &y, &support));
        let next: Vec<f64> = y
            .iter()
            .zip(&apy)
            .map(|(yk, g)| (yk + step * g).max(0.0))
            .collect();
        // Restart the momentum when it points against the step (O'Donoghue and Candès).
        let against: f64 = (0..n).map(|k| (y[k] - next[k]) * (next[k] - mu[k])).sum();
        if against > 0.0 {
            t = 1.0;
            y.clone_from(&next);
        } else {
            let t_next = (1.0 + (1.0 + 4.0 * t * t).sqrt()) / 2.0;
            let beta = (t - 1.0) / t_next;
            y = next
                .iter()
                .zip(&mu)
                .map(|(x, m)| x + beta * (x - m))
                .collect();
            t = t_next;
        }
        mu = next;
    }
    Equilibrium {
        p: weights(a, &mu, &support),
        converged: false,
        iterations: ITERATION_CAP,
    }
}

/// `p(mu)`: the softmax of `a mu` over the support, 0 elsewhere.
fn weights(a: &[Vec<f64>], mu: &[f64], support: &[bool]) -> Vec<f64> {
    let z = mat_vec(a, mu);
    let top = z
        .iter()
        .zip(support)
        .filter(|(_, s)| **s)
        .map(|(x, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let e: Vec<f64> = z
        .iter()
        .zip(support)
        .map(|(x, s)| if *s { (x - top).exp() } else { 0.0 })
        .collect();
    let sum: f64 = e.iter().sum();
    e.iter().map(|x| x / sum).collect()
}

/// A weight above this in a linear programme's solution puts the strategy in the support;
/// a reduced cost or a pivot entry must exceed it to count.
const LP_EPS: f64 = 1e-9;

/// The strategies some equilibrium of `a` plays. Never empty: a symmetric zero-sum game
/// always has an equilibrium.
fn support(a: &[Vec<f64>]) -> Vec<bool> {
    let n = a.len();
    let mut support = vec![false; n];
    for k in 0..n {
        if support[k] {
            continue;
        }
        for (j, x) in max_weight(a, k).iter().enumerate() {
            if *x > LP_EPS {
                support[j] = true;
            }
        }
    }
    support
}

/// An equilibrium of `a` with the largest weight on strategy `k`: maximise `p[k]` subject
/// to `a p + s = 0`, `sum p = 1`, `p, s >= 0`, by the two-phase simplex method with
/// Bland's rule, which cannot cycle. A dense tableau: at most 17 rows and 34 columns here.
fn max_weight(a: &[Vec<f64>], k: usize) -> Vec<f64> {
    let n = a.len();
    // Columns: p (0..n), s (n..2n), the artificial variable (2n), the right-hand side.
    let art = 2 * n;
    let rhs = 2 * n + 1;
    let mut tab = vec![vec![0.0; rhs + 1]; n + 1];
    for (row, a_row) in tab.iter_mut().zip(a) {
        row[..n].copy_from_slice(a_row);
    }
    for (i, row) in tab.iter_mut().enumerate().take(n) {
        row[n + i] = 1.0;
    }
    for x in &mut tab[n][..n] {
        *x = 1.0;
    }
    tab[n][art] = 1.0;
    tab[n][rhs] = 1.0;
    let mut basis: Vec<usize> = (n..2 * n).chain([art]).collect();
    // Phase 1: drive the artificial variable to 0.
    let mut cost = vec![0.0; rhs];
    cost[art] = -1.0;
    simplex(&mut tab, &mut basis, &cost, rhs);
    // Pivot it out of the basis where its row allows; a row that does not is redundant.
    if let Some(row) = basis.iter().position(|&b| b == art) {
        if let Some(col) = (0..art).find(|&c| tab[row][c].abs() > LP_EPS) {
            pivot(&mut tab, &mut basis, row, col);
        }
    }
    // Phase 2: the artificial variable may not enter again.
    let mut cost = vec![0.0; art];
    cost[k] = 1.0;
    simplex(&mut tab, &mut basis, &cost, art);
    let mut x = vec![0.0; n];
    for (row, &b) in basis.iter().enumerate() {
        if b < n {
            x[b] = tab[row][rhs];
        }
    }
    x
}

/// Maximises `cost · x` over the tableau's columns `0..cols`, from a feasible basis.
fn simplex(tab: &mut [Vec<f64>], basis: &mut [usize], cost: &[f64], cols: usize) {
    let rhs = tab[0].len() - 1;
    loop {
        let reduced = |c: usize| {
            cost[c]
                - basis
                    .iter()
                    .zip(tab.iter())
                    .map(|(&b, row)| cost.get(b).copied().unwrap_or(0.0) * row[c])
                    .sum::<f64>()
        };
        // Bland's rule: the first column with a positive reduced cost enters.
        let Some(col) = (0..cols).find(|&c| !basis.contains(&c) && reduced(c) > LP_EPS) else {
            return;
        };
        // The ratio test; a tie goes to the smallest basic variable.
        let leave = (0..tab.len())
            .filter(|&r| tab[r][col] > LP_EPS)
            .min_by(|&r, &q| {
                let (x, y) = (tab[r][rhs] / tab[r][col], tab[q][rhs] / tab[q][col]);
                x.total_cmp(&y).then(basis[r].cmp(&basis[q]))
            });
        // The equilibrium set lies in the simplex, so the programme is never unbounded.
        let Some(row) = leave else { return };
        pivot(tab, basis, row, col);
    }
}

fn pivot(tab: &mut [Vec<f64>], basis: &mut [usize], row: usize, col: usize) {
    let lead = tab[row][col];
    for x in &mut tab[row] {
        *x /= lead;
    }
    let pivot_row = tab[row].clone();
    for (r, other) in tab.iter_mut().enumerate() {
        let f = other[col];
        if r != row && f != 0.0 {
            for (x, p) in other.iter_mut().zip(&pivot_row) {
                *x -= f * p;
            }
        }
    }
    basis[row] = col;
}

fn mat_vec(a: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    a.iter()
        .map(|row| row.iter().zip(v).map(|(x, y)| x * y).sum())
        .collect()
}

/// One formation's rating, each number rounded to 4 decimals.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FormationRating {
    pub formation: String,
    /// Its weight in the maximum-entropy equilibrium.
    pub weight: f64,
    /// Its mean win share over every formation of the field, itself included.
    pub win_rate: f64,
    /// Its win share against the field as the equilibrium weighs it.
    pub field_rating: f64,
    /// `(A p*)[i]`, as Nash averaging defines it: 0 inside the equilibrium's support, at
    /// most 0 outside it.
    pub nash_average: f64,
    /// The field rating exceeds the win rate by more than [`FLAG_MARGIN`].
    pub flagged: bool,
}

/// The formation field rating of one run, as `calib.nash` holds it. Report only.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NashReport {
    pub flag_margin: f64,
    pub converged: bool,
    pub iterations: u32,
    /// The fewest finished matches of a pairing.
    pub matches_per_pairing: u32,
    /// One row per formation, in the tactics file's order.
    pub formations: Vec<FormationRating>,
}

/// Rates every formation of `names` from the win-share table `wins` (`wins[i][j]`: the
/// share of the matches between `i` and `j` that `i` won).
pub fn rate(names: &[String], wins: &[Vec<f64>], matches: u32) -> NashReport {
    let n = names.len();
    let a: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| wins[i][j] - wins[j][i]).collect())
        .collect();
    let eq = maxent_equilibrium(&a);
    let ap = mat_vec(&a, &eq.p);
    // `+ 0.0` turns a rounded -0 into 0.
    let round = |x: f64| round_to(x, 4) + 0.0;
    let formations = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let win_rate = round(wins[i].iter().sum::<f64>() / n as f64);
            let field_rating = round(wins[i].iter().zip(&eq.p).map(|(w, p)| w * p).sum());
            FormationRating {
                formation: name.clone(),
                weight: round(eq.p[i]),
                win_rate,
                field_rating,
                nash_average: round(ap[i]),
                // Judged on the printed numbers, so a reader can check it.
                flagged: round(field_rating - win_rate) > FLAG_MARGIN,
            }
        })
        .collect();
    NashReport {
        flag_margin: FLAG_MARGIN,
        converged: eq.converged,
        iterations: eq.iterations,
        matches_per_pairing: matches,
        formations,
    }
}

/// The console table, after the reported bands: one row per formation.
pub fn render(r: &NashReport) -> String {
    let state = if r.converged {
        String::new()
    } else {
        format!(", not converged after {} iterations", r.iterations)
    };
    let mut out = format!(
        "formation field rating (report only{state}; it never changes the verdict), {} \
         matches per pairing:\n",
        r.matches_per_pairing
    );
    out.push_str(&format!(
        "{:<14} {:>9} {:>13} {:>8} {:>13}  {}\n",
        "formation", "win rate", "field rating", "weight", "nash average", "flag"
    ));
    for f in &r.formations {
        out.push_str(&format!(
            "{:<14} {:>9.4} {:>13.4} {:>8.4} {:>13.4}  {}\n",
            f.formation,
            f.win_rate,
            f.field_rating,
            f.weight,
            f.nash_average,
            if f.flagged { "flagged" } else { "-" }
        ));
    }
    let flagged = r.formations.iter().filter(|f| f.flagged).count();
    out.push_str(&format!(
        "field rating: {flagged} of {} formations flagged (field rating above win rate by more \
         than {:.2}); it never changes the verdict\n",
        r.formations.len(),
        r.flag_margin
    ));
    out
}

/// The console line of a run that did not finish every pairing of the formations suite.
pub fn incomplete_line(played: usize, total: usize) -> String {
    format!("formation field rating: needs every pairing ({played} of {total} played)\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feasible(a: &[Vec<f64>], p: &[f64]) {
        for (i, x) in mat_vec(a, p).iter().enumerate() {
            assert!(*x <= 1e-9, "strategy {i} gains {x} against {p:?}");
        }
        assert!((p.iter().sum::<f64>() - 1.0).abs() < 1e-12, "{p:?}");
    }

    fn close(p: &[f64], want: &[f64], tol: f64) {
        assert_eq!(p.len(), want.len());
        for (x, w) in p.iter().zip(want) {
            assert!((x - w).abs() <= tol, "{p:?} is not {want:?}");
        }
    }

    fn rps() -> Vec<Vec<f64>> {
        vec![
            vec![0.0, -1.0, 1.0],
            vec![1.0, 0.0, -1.0],
            vec![-1.0, 1.0, 0.0],
        ]
    }

    #[test]
    fn rock_paper_scissors_is_uniform() {
        let eq = maxent_equilibrium(&rps());
        assert!(eq.converged, "{eq:?}");
        close(&eq.p, &[1.0 / 3.0; 3], 1e-6);
        feasible(&rps(), &eq.p);
    }

    #[test]
    fn a_cloned_rock_splits_its_weight() {
        // Rock, rock again, paper, scissors.
        let a = vec![
            vec![0.0, 0.0, -1.0, 1.0],
            vec![0.0, 0.0, -1.0, 1.0],
            vec![1.0, 1.0, 0.0, -1.0],
            vec![-1.0, -1.0, 1.0, 0.0],
        ];
        let eq = maxent_equilibrium(&a);
        assert!(eq.converged, "{eq:?}");
        close(&eq.p, &[1.0 / 6.0, 1.0 / 6.0, 1.0 / 3.0, 1.0 / 3.0], 1e-6);
        feasible(&a, &eq.p);
    }

    #[test]
    fn a_strictly_dominant_strategy_takes_all_the_weight() {
        let a = vec![
            vec![0.0, 1.0, 0.5],
            vec![-1.0, 0.0, 0.2],
            vec![-0.5, -0.2, 0.0],
        ];
        let eq = maxent_equilibrium(&a);
        assert!(eq.converged, "{eq:?}");
        close(&eq.p, &[1.0, 0.0, 0.0], 1e-6);
        feasible(&a, &eq.p);
    }

    #[test]
    fn a_tie_is_uniform() {
        let eq = maxent_equilibrium(&[vec![0.0, 0.0], vec![0.0, 0.0]]);
        assert!(eq.converged);
        assert_eq!(eq.p, [0.5, 0.5]);
    }

    #[test]
    #[should_panic(expected = "antisymmetric")]
    fn a_table_that_is_not_antisymmetric_is_refused() {
        maxent_equilibrium(&[vec![0.0, 1.0], vec![1.0, 0.0]]);
    }

    /// Tables of ten formations as one or five matches per pairing give them: shares of 0,
    /// 0.5 and 1 with many ties, the hardest case. Each solve converges to an equilibrium,
    /// and no equilibrium the linear programme finds has more entropy in its direction.
    #[test]
    fn noisy_tables_of_ten_formations_converge_to_the_maximum_entropy_equilibrium() {
        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % 3
        };
        for trial in 0..40 {
            let (n, m) = (10, if trial % 2 == 0 { 1 } else { 5 });
            let mut w = vec![vec![0.0; n]; n];
            for (i, j) in (0..n).flat_map(|i| (i..n).map(move |j| (i, j))) {
                let (mut first, mut second) = (0.0, 0.0);
                for _ in 0..m {
                    match next() {
                        0 => first += 1.0,
                        1 => second += 1.0,
                        _ => {}
                    }
                }
                let m = f64::from(m);
                if i == j {
                    w[i][i] = (first + second) / (2.0 * m);
                } else {
                    w[i][j] = first / m;
                    w[j][i] = second / m;
                }
            }
            let a: Vec<Vec<f64>> = (0..n)
                .map(|i| (0..n).map(|j| w[i][j] - w[j][i]).collect())
                .collect();
            let eq = maxent_equilibrium(&a);
            assert!(eq.converged, "trial {trial}: {eq:?}");
            feasible(&a, &eq.p);
            for k in 0..n {
                let q = max_weight(&a, k);
                feasible(&a, &q);
                // The entropy's slope from p towards q: never upward at the maximum. A
                // weight of q under the programme's tolerance is rounding, not play.
                let slope: f64 = q
                    .iter()
                    .zip(&eq.p)
                    .filter(|(qi, pi)| **qi > LP_EPS || **pi > 0.0)
                    .map(|(qi, pi)| (qi - pi) * -(pi.ln() + 1.0))
                    .sum();
                assert!(slope <= 1e-6, "trial {trial}, k {k}: slope {slope}, {eq:?}");
            }
        }
    }

    fn names(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_cycle_with_equal_win_rates_flags_nothing() {
        // Each formation beats the next 0.6 to 0.4; a mirror is won by either side 0.4 of
        // the time.
        let w = vec![
            vec![0.4, 0.6, 0.4],
            vec![0.4, 0.4, 0.6],
            vec![0.6, 0.4, 0.4],
        ];
        let r = rate(&names(&["a", "b", "c"]), &w, 10);
        assert!(r.converged);
        for f in &r.formations {
            assert_eq!(f.weight, 0.3333, "{f:?}");
            assert_eq!(f.win_rate, 0.4667, "{f:?}");
            assert_eq!(f.field_rating, 0.4667, "{f:?}");
            assert_eq!(f.nash_average, 0.0, "{f:?}");
            assert!(!f.flagged);
        }
        assert_eq!(r.matches_per_pairing, 10);
        assert_eq!(r.flag_margin, 0.10);
    }

    /// Rock, paper and scissors; `x`, a copy of rock that loses to the two traps; and two
    /// traps that lose to rock, paper and scissors and beat `x`. The traps pull `x`'s win
    /// rate down, but the equilibrium never plays them, so `x`'s field rating is far above
    /// its win rate.
    fn trapped() -> (Vec<String>, Vec<Vec<f64>>) {
        let w = vec![
            //   r    p    s    x    t1   t2
            vec![0.5, 0.0, 1.0, 0.5, 1.0, 1.0], // r
            vec![1.0, 0.5, 0.0, 1.0, 1.0, 1.0], // p
            vec![0.0, 1.0, 0.5, 0.0, 1.0, 1.0], // s
            vec![0.5, 0.0, 1.0, 0.5, 0.0, 0.0], // x
            vec![0.0, 0.0, 0.0, 1.0, 0.5, 0.5], // t1
            vec![0.0, 0.0, 0.0, 1.0, 0.5, 0.5], // t2
        ];
        (names(&["r", "p", "s", "x", "t1", "t2"]), w)
    }

    #[test]
    fn a_formation_that_only_loses_to_formations_the_field_never_plays_is_flagged() {
        let (n, w) = trapped();
        let r = rate(&n, &w, 4);
        assert!(r.converged, "{r:?}");
        let by = |name: &str| r.formations.iter().find(|f| f.formation == name).unwrap();
        // Rock's weight splits between rock and its copy.
        assert_eq!(by("r").weight, 0.1667);
        assert_eq!(by("x").weight, 0.1667);
        assert_eq!(by("p").weight, 0.3333);
        assert_eq!(by("s").weight, 0.3333);
        assert_eq!(by("t1").weight, 0.0);
        let x = by("x");
        assert_eq!(x.win_rate, 0.3333);
        assert_eq!(x.field_rating, 0.5);
        assert!(x.flagged, "{x:?}");
        let flagged: Vec<&str> = r
            .formations
            .iter()
            .filter(|f| f.flagged)
            .map(|f| f.formation.as_str())
            .collect();
        assert_eq!(flagged, ["x"]);
        // A trap is outside the support: its Nash average is negative.
        assert!(by("t1").nash_average < 0.0);
        assert_eq!(by("p").nash_average, 0.0);
    }

    #[test]
    fn a_table_of_draws_rates_every_formation_zero() {
        let w = vec![vec![0.0; 3]; 3];
        let r = rate(&names(&["a", "b", "c"]), &w, 1);
        for f in &r.formations {
            assert_eq!((f.win_rate, f.field_rating, f.flagged), (0.0, 0.0, false));
        }
    }

    #[test]
    fn the_table_names_each_formation_and_says_it_only_reports() {
        let (n, w) = trapped();
        let text = render(&rate(&n, &w, 4));
        assert!(
            text.starts_with(
                "formation field rating (report only; it never changes the verdict), 4 matches \
                 per pairing:\n"
            ),
            "{text}"
        );
        assert!(
            text.contains(
                "x                 0.3333        0.5000   0.1667        0.0000  flagged\n"
            ),
            "{text}"
        );
        assert!(
            text.contains("field rating: 1 of 6 formations flagged"),
            "{text}"
        );
        let capped = NashReport {
            converged: false,
            iterations: ITERATION_CAP,
            ..rate(&n, &w, 4)
        };
        assert!(render(&capped).contains("not converged after 1000000 iterations"));
        assert_eq!(
            incomplete_line(1, 55),
            "formation field rating: needs every pairing (1 of 55 played)\n"
        );
    }
}
