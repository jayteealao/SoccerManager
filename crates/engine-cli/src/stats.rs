//! The small statistics the tables and reports share, kept in one place so a rule (how an
//! empty sample reads, how a share between two ranks is taken) changes once for all of them.
//!
//! Two quantiles live here and their names say which: [`quantile_linear`] interpolates
//! between the two nearest ranks, as the signature table and the restart waits take it;
//! [`quantile_rank`] picks one value of the sample, as the bootstrap bands take it.

/// The value at share `p` (0 to 1) of the sorted values `v`, linear between the two nearest
/// ranks. An empty sample has no value and reads NaN.
pub fn quantile_linear(v: &[f64], p: f64) -> f64 {
    let Some(last) = v.len().checked_sub(1) else {
        return f64::NAN;
    };
    let at = p * last as f64;
    let i = at.floor() as usize;
    let j = (i + 1).min(last);
    v[i] + (v[j] - v[i]) * (at - i as f64)
}

/// The `q` quantile of the sorted values by the bootstrap's rule: the `ceil((B + 1) q)`-th
/// smallest of `B`. An empty sample reads 0. The pass rule owns it, so the bootstrap bands
/// and the pass rule pick their bounds with one rule.
pub use pass_rule::interval::quantile_rank;

/// `x` rounded to `places` decimal places.
pub fn round_to(x: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (x * scale).round() / scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_linear_quantile_interpolates_and_an_empty_sample_reads_nan() {
        let v = [1.0, 2.0, 3.0, 5.0];
        assert_eq!(quantile_linear(&v, 0.0), 1.0);
        assert_eq!(quantile_linear(&v, 1.0), 5.0);
        assert_eq!(quantile_linear(&v, 0.5), 2.5);
        assert_eq!(quantile_linear(&[7.0], 0.3), 7.0);
        assert!(quantile_linear(&[], 0.5).is_nan());
    }

    #[test]
    fn the_rank_quantile_picks_a_value_of_the_sample() {
        let v = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(quantile_rank(&v, 0.5), 3.0);
        assert_eq!(quantile_rank(&v, 0.0), 1.0);
        assert_eq!(quantile_rank(&v, 1.0), 4.0);
        assert_eq!(quantile_rank(&[], 0.5), 0.0);
    }

    #[test]
    fn rounding_keeps_the_asked_places() {
        assert_eq!(round_to(0.123_456_789, 6), 0.123_457);
        assert_eq!(round_to(0.123_456_789, 4), 0.1235);
        assert_eq!(round_to(-2.5, 0), -3.0);
    }
}
