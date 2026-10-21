//! The drift check of a band's history.
//!
//! Each run adds one point per band. The first [`REFERENCE_POINTS`] points of a definition
//! version set the band's reference, their precision-weighted mean; each later point is
//! standardized by its own error and the reference's, and a two-sided tabular cumulative sum
//! with allowance [`K`] flags the band when either sum passes [`H`]. A flag names the run
//! where the sum began to rise, so a person can trace the drift to the changes made since.
//! No flag blocks a change.

/// Points that set a band's reference before the check starts.
pub const REFERENCE_POINTS: usize = 5;

/// The allowance: half a standard error, which tunes the check to a shift of one error.
pub const K: f64 = 0.5;

/// The decision threshold. Set by the seeded simulation in the tests so that a band with no
/// drift, its reference taken from its first five points, runs at least 500 points on
/// average before a false flag.
pub const H: f64 = 5.5;

/// One history point: a band's value and its standard error in one run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub value: f64,
    pub se: f64,
}

/// A drift the check found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flag {
    /// The index of the first point of the rise.
    pub start: usize,
    /// The index of the point where the sum passed the threshold.
    pub at: usize,
    /// True for an upward drift.
    pub up: bool,
}

/// Runs the two-sided cumulative sum over `points` and returns the first flag, if any.
/// Fewer than `REFERENCE_POINTS + 1` points, or a reference with no finite error, gives none.
pub fn cusum(points: &[Point], k: f64, h: f64) -> Option<Flag> {
    if points.len() <= REFERENCE_POINTS {
        return None;
    }
    let reference = &points[..REFERENCE_POINTS];
    let weights: Vec<f64> = reference.iter().map(|p| 1.0 / p.se.powi(2)).collect();
    let total: f64 = weights.iter().sum();
    if !(total.is_finite() && total > 0.0) {
        return None;
    }
    let mean = reference
        .iter()
        .zip(&weights)
        .map(|(p, w)| p.value * w)
        .sum::<f64>()
        / total;
    let ref_var = 1.0 / total;
    let (mut up, mut down) = (0.0_f64, 0.0_f64);
    let (mut up_start, mut down_start) = (REFERENCE_POINTS, REFERENCE_POINTS);
    for (i, p) in points.iter().enumerate().skip(REFERENCE_POINTS) {
        let sd = (p.se.powi(2) + ref_var).sqrt();
        if !(sd.is_finite() && sd > 0.0) {
            continue;
        }
        let z = (p.value - mean) / sd;
        if up == 0.0 {
            up_start = i;
        }
        if down == 0.0 {
            down_start = i;
        }
        up = (up + z - k).max(0.0);
        down = (down - z - k).max(0.0);
        if up > h {
            return Some(Flag {
                start: up_start,
                at: i,
                up: true,
            });
        }
        if down > h {
            return Some(Flag {
                start: down_start,
                at: i,
                up: false,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interval::tests::stream;
    use crate::special::norm_quantile;

    fn normal(draw: &mut impl FnMut() -> f64) -> f64 {
        norm_quantile(draw().clamp(1e-12, 1.0 - 1e-12))
    }

    /// The average run length to a false flag of a band with no drift, over `series` seeded
    /// series of at most `cap` monitored points each.
    fn false_flag_run_length(h: f64, series: usize, cap: usize) -> f64 {
        let mut draw = stream(500);
        let mut total = 0usize;
        for _ in 0..series {
            let points: Vec<Point> = (0..REFERENCE_POINTS + cap)
                .map(|_| Point {
                    value: 10.0 + 0.3 * normal(&mut draw),
                    se: 0.3,
                })
                .collect();
            total += cusum(&points, K, h).map_or(cap, |f| f.at + 1 - REFERENCE_POINTS);
        }
        total as f64 / series as f64
    }

    #[test]
    fn a_band_with_no_drift_flags_less_than_once_in_500_points() {
        let run = false_flag_run_length(H, 400, 5_000);
        assert!(run >= 500.0, "average run to a false flag {run}");
    }

    #[test]
    fn a_step_of_two_errors_at_point_twenty_flags_by_point_thirty() {
        let mut draw = stream(77);
        let mut flagged = 0;
        for _ in 0..200 {
            let points: Vec<Point> = (0..40)
                .map(|i| {
                    let shift = if i >= 20 { 2.0 * 0.3 } else { 0.0 };
                    Point {
                        value: 10.0 + shift + 0.3 * normal(&mut draw),
                        se: 0.3,
                    }
                })
                .collect();
            if let Some(f) = cusum(&points, K, H) {
                if f.up && f.at <= 30 && f.at >= 20 {
                    flagged += 1;
                }
            }
        }
        // Four in five series with a two-error step flag within ten points of it; the rest
        // flag a little later, or once in a while too early on the reference's own noise.
        assert!(flagged >= 160, "flagged {flagged} of 200 by point 30");
    }

    #[test]
    fn the_flag_names_where_the_rise_began() {
        let mut points: Vec<Point> = (0..20)
            .map(|_| Point {
                value: 1.0,
                se: 0.1,
            })
            .collect();
        points.extend((0..10).map(|_| Point {
            value: 1.5,
            se: 0.1,
        }));
        let flag = cusum(&points, K, H).unwrap();
        assert!(flag.up);
        assert_eq!(flag.start, 20);
        assert!(flag.at >= 20 && flag.at < 25);
    }

    #[test]
    fn too_few_points_give_no_flag() {
        let points = [Point {
            value: 1.0,
            se: 0.1,
        }; REFERENCE_POINTS];
        assert_eq!(cusum(&points, K, H), None);
    }

    #[test]
    #[ignore = "prints the run length for each threshold; used to set H"]
    fn print_run_lengths() {
        for h in [4.5, 5.0, 5.2, 5.4, 5.6, 5.8, 6.0, 6.5] {
            println!("h {h}: {}", false_flag_run_length(h, 400, 5_000));
        }
    }
}
