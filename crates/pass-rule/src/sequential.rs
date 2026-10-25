//! Early stopping by error spending, for tactics-array cells only.
//!
//! Change, milestone and release runs play their full count. A tactics-array cell may stop
//! early, but only at one of a fixed number of planned looks, and only with boundaries that
//! spend the error rate over the looks so that stopping never raises it. The spending
//! function is Lan and DeMets' O'Brien-Fleming type,
//! `alpha(t) = 2 - 2 * Phi(z(1 - alpha / 2) / sqrt(t))`, which spends almost nothing early,
//! so a cell rarely stops on noise. The boundaries come from the recursive numerical
//! integration of Armitage, McPherson and Rowe over the continuation region of each look.

use crate::special::{norm_cdf, norm_quantile};

/// The grid of the recursive integration: points over `[-GRID_EDGE, GRID_EDGE]` on the
/// score scale (the score at information fraction `t` has variance `t <= 1`).
pub const GRID_POINTS: usize = 6001;
pub const GRID_EDGE: f64 = 10.0;

/// A plan of looks: the information fraction of each look (increasing, the last 1) and the
/// two-sided error rate the looks spend in all.
#[derive(Debug, Clone, PartialEq)]
pub struct SpendingPlan {
    pub fractions: Vec<f64>,
    pub alpha: f64,
}

impl SpendingPlan {
    /// `looks` equally spaced looks.
    pub fn equal(looks: usize, alpha: f64) -> Self {
        let k = looks.max(1);
        SpendingPlan {
            fractions: (1..=k).map(|i| i as f64 / k as f64).collect(),
            alpha,
        }
    }

    /// The error spent by information fraction `t`.
    pub fn spent(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return 0.0;
        }
        2.0 - 2.0 * norm_cdf(norm_quantile(1.0 - self.alpha / 2.0) / t.sqrt())
    }

    /// The two-sided boundary of each look on the z scale: the cell may stop at look `k`
    /// when the band's statistic is beyond `b_k` standard errors, and the chance under no
    /// effect of first crossing at look `k` equals the error spent between looks `k - 1` and
    /// `k`.
    pub fn boundaries(&self) -> Vec<f64> {
        let h = 2.0 * GRID_EDGE / (GRID_POINTS - 1) as f64;
        let grid: Vec<f64> = (0..GRID_POINTS)
            .map(|i| -GRID_EDGE + i as f64 * h)
            .collect();
        let mut out = Vec::with_capacity(self.fractions.len());
        // The score's density at the previous look over its continuation region, as nodes
        // with trapezoid weights that end exactly at the boundary.
        let mut nodes: Option<Vec<(f64, f64)>> = None;
        let mut prev_t = 0.0;
        let mut prev_spent = 0.0;
        for &t in &self.fractions {
            let dt = t - prev_t;
            let f: Vec<f64> = match &nodes {
                None => grid.iter().map(|s| normal_pdf(*s, t.sqrt())).collect(),
                Some(prev) => {
                    let sd = dt.sqrt();
                    grid.iter()
                        .map(|s| prev.iter().map(|(u, w)| w * normal_pdf(s - u, sd)).sum())
                        .collect()
                }
            };
            // The density is symmetric: the upper tail beyond `c`, twice, is the crossing
            // chance. `upper[i]` is the trapezoid mass from `grid[i]` to the grid's edge.
            let mut upper = vec![0.0; GRID_POINTS];
            for i in (0..GRID_POINTS - 1).rev() {
                upper[i] = upper[i + 1] + h * (f[i] + f[i + 1]) / 2.0;
            }
            let at = |c: f64| -> (usize, f64) {
                let i = (((c + GRID_EDGE) / h).floor() as usize).min(GRID_POINTS - 2);
                let frac = (c - grid[i]) / h;
                (i, f[i] + frac * (f[i + 1] - f[i]))
            };
            let cross = |c: f64| -> f64 {
                let (i, fc) = at(c);
                2.0 * (upper[i + 1] + (grid[i + 1] - c) * (fc + f[i + 1]) / 2.0)
            };
            let spend = (self.spent(t) - prev_spent).max(0.0);
            let c = if nodes.is_none() {
                // The first look has a closed form.
                norm_quantile(1.0 - spend / 2.0) * t.sqrt()
            } else {
                // The crossing chance falls as `c` grows: bisect it.
                let (mut lo, mut hi) = (0.0, GRID_EDGE - h);
                for _ in 0..80 {
                    let mid = (lo + hi) / 2.0;
                    if cross(mid) > spend {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                (lo + hi) / 2.0
            };
            out.push(c / t.sqrt());
            let fc = at(c).1;
            let mut points: Vec<(f64, f64)> = vec![(-c, fc)];
            points.extend(
                grid.iter()
                    .zip(&f)
                    .filter(|(s, _)| s.abs() < c)
                    .map(|(s, v)| (*s, *v)),
            );
            points.push((c, fc));
            let weights: Vec<f64> = (0..points.len())
                .map(|j| {
                    let left = if j > 0 {
                        points[j].0 - points[j - 1].0
                    } else {
                        0.0
                    };
                    let right = if j + 1 < points.len() {
                        points[j + 1].0 - points[j].0
                    } else {
                        0.0
                    };
                    (left + right) / 2.0
                })
                .collect();
            nodes = Some(
                points
                    .iter()
                    .zip(&weights)
                    .map(|((s, v), w)| (*s, v * w))
                    .collect(),
            );
            prev_t = t;
            prev_spent = self.spent(t);
        }
        out
    }
}

fn normal_pdf(x: f64, sd: f64) -> f64 {
    (-(x / sd).powi(2) / 2.0).exp() / (sd * (2.0 * std::f64::consts::PI).sqrt())
}

/// One band at a look: its estimate, its error and its margin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandLook {
    pub estimate: f64,
    pub se: f64,
    pub margin: f64,
}

/// A band's state at a look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decided {
    Pass,
    Fail,
    Open,
}

/// What a cell does after a look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellStep {
    /// Every band is decided at a planned look before the last: the cell stops here.
    Stop(Vec<Decided>),
    /// Some band is open: play on to the next planned look.
    Continue,
    /// The last look: the cell has played its full count and is judged there.
    Full(Vec<Decided>),
}

/// The looks of one tactics-array cell.
#[derive(Debug, Clone, PartialEq)]
pub struct CellLook {
    pub boundaries: Vec<f64>,
}

impl CellLook {
    pub fn new(plan: &SpendingPlan) -> Self {
        CellLook {
            boundaries: plan.boundaries(),
        }
    }

    /// The step after planned look `look` (0-based). A look that is not planned never stops
    /// the cell; a cell stops early only when every band is decided at the look's boundary:
    /// pass when `estimate +- b * se` sits inside the margin, fail when wholly outside.
    pub fn decide(&self, look: usize, bands: &[BandLook]) -> CellStep {
        let Some(&b) = self.boundaries.get(look) else {
            return CellStep::Continue;
        };
        let words: Vec<Decided> = bands
            .iter()
            .map(|x| {
                let (lo, hi) = (x.estimate - b * x.se, x.estimate + b * x.se);
                if !(x.se.is_finite() && x.se > 0.0) {
                    Decided::Open
                } else if lo >= -x.margin && hi <= x.margin {
                    Decided::Pass
                } else if lo > x.margin || hi < -x.margin {
                    Decided::Fail
                } else {
                    Decided::Open
                }
            })
            .collect();
        if look + 1 == self.boundaries.len() {
            CellStep::Full(words)
        } else if words.iter().all(|w| *w != Decided::Open) {
            CellStep::Stop(words)
        } else {
            CellStep::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The boundaries of five equal looks at alpha 0.05, pinned once the path simulation
    /// below agreed with the integration (a grid of 16,001 points gives the same to 1e-5).
    const GOLDEN_FIVE: [f64; 5] = [4.382613, 3.099735, 2.553359, 2.253852, 2.063504];

    #[test]
    fn one_look_is_the_fixed_test() {
        let b = SpendingPlan::equal(1, 0.05).boundaries();
        assert_eq!(b.len(), 1);
        assert!((b[0] - 1.959964).abs() < 1e-6, "{b:?}");
    }

    #[test]
    fn the_first_boundary_is_the_closed_form() {
        let plan = SpendingPlan::equal(5, 0.05);
        assert!((plan.spent(0.2) - 1.1726e-5).abs() < 1e-8);
        let b = plan.boundaries();
        assert!((b[0] - 4.382613).abs() < 1e-6, "{b:?}");
        // The grid's own crossing chance agrees with the closed form.
        let tail = 2.0 * (1.0 - norm_cdf(b[0]));
        assert!((tail - plan.spent(0.2)).abs() < 1e-9);
    }

    #[test]
    fn five_equal_looks_match_the_golden_boundaries() {
        let b = SpendingPlan::equal(5, 0.05).boundaries();
        for (k, (got, want)) in b.iter().zip(GOLDEN_FIVE).enumerate() {
            assert!((got - want).abs() < 1e-4, "look {k}: {got} against {want}");
        }
    }

    #[test]
    fn a_path_simulation_crosses_as_the_plan_spends() {
        let plan = SpendingPlan::equal(5, 0.05);
        let b = plan.boundaries();
        let paths = 1_000_000_u32;
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut uniform = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            ((state >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        };
        let mut crossed = [0_u32; 5];
        for _ in 0..paths {
            let mut s = 0.0;
            for k in 0..5 {
                // One normal step of variance 0.2 (Box-Muller, one of the pair).
                let (u1, u2) = (uniform(), uniform());
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                s += z * 0.2_f64.sqrt();
                let t = (k + 1) as f64 * 0.2;
                if (s / t.sqrt()).abs() >= b[k] {
                    crossed[k] += 1;
                    break;
                }
            }
        }
        let mut by = 0_u32;
        for (k, c) in crossed.iter().enumerate() {
            by += c;
            let p = f64::from(by) / f64::from(paths);
            let want = plan.spent((k + 1) as f64 * 0.2);
            let se = (want * (1.0 - want) / f64::from(paths)).sqrt().max(1e-6);
            assert!(
                (p - want).abs() < 3.0 * se,
                "look {k}: crossed {p} against spent {want} (se {se})"
            );
        }
    }

    #[test]
    fn a_cell_stops_only_at_a_planned_look_when_every_band_is_decided() {
        let cell = CellLook::new(&SpendingPlan::equal(5, 0.05));
        let tight = BandLook {
            estimate: 0.0,
            se: 0.01,
            margin: 1.0,
        };
        let open = BandLook {
            estimate: 0.0,
            se: 1.0,
            margin: 1.0,
        };
        assert_eq!(
            cell.decide(1, &[tight, tight]),
            CellStep::Stop(vec![Decided::Pass, Decided::Pass])
        );
        assert_eq!(cell.decide(1, &[tight, open]), CellStep::Continue);
        // A look past the plan never stops the cell.
        assert_eq!(cell.decide(9, &[tight]), CellStep::Continue);
        // The last look judges the full count, decided or not.
        assert_eq!(
            cell.decide(4, &[tight, open]),
            CellStep::Full(vec![Decided::Pass, Decided::Open])
        );
        let far = BandLook {
            estimate: 9.0,
            se: 0.1,
            margin: 1.0,
        };
        assert_eq!(cell.decide(0, &[far]), CellStep::Stop(vec![Decided::Fail]));
    }
}
