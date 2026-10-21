//! Exact count bounds and the error of a mean.
//!
//! A rare band (red cards, penalties, own goals) can see zero events in a run. A normal
//! approximation then gives an error of zero and a band that passes by luck; the exact
//! bounds below never do. A share uses Clopper-Pearson, a rate per match uses Garwood.

use crate::special::{beta_quantile, chi2_quantile};

/// The Clopper-Pearson bounds of a share with `k` events in `n` trials, each bound one-sided
/// at `confidence` (so the pair covers at least `2 * confidence - 1`).
pub fn clopper_pearson(k: u64, n: u64, confidence: f64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let (k, n) = (k as f64, n as f64);
    let lo = if k == 0.0 {
        0.0
    } else {
        beta_quantile(1.0 - confidence, k, n - k + 1.0)
    };
    let hi = if k >= n {
        1.0
    } else {
        beta_quantile(confidence, k + 1.0, n - k)
    };
    (lo, hi)
}

/// The Garwood bounds of a rate with `k` events over `exposure` units (matches), each bound
/// one-sided at `confidence`.
pub fn garwood(k: u64, exposure: f64, confidence: f64) -> (f64, f64) {
    if exposure <= 0.0 {
        return (0.0, f64::INFINITY);
    }
    let k = k as f64;
    let lo = if k == 0.0 {
        0.0
    } else {
        chi2_quantile(1.0 - confidence, 2.0 * k) / 2.0 / exposure
    };
    let hi = chi2_quantile(confidence, 2.0 * k + 2.0) / 2.0 / exposure;
    (lo, hi)
}

/// The standard error of a mean, never zero.
///
/// With spread it is the sample standard deviation over the square root of `n`. A sample
/// with no spread still has an error: a mean of counts takes the Poisson error
/// `sqrt(max(mean, -ln(alpha) / n) / n)`, where `-ln(alpha) / n` is the exact upper bound of a
/// rate with zero events; any other mean takes its rounding `step` over `sqrt(12 n)`, the
/// error of a value known only to that step.
pub fn mean_error(values: &[f64], counts: bool, step: f64, alpha: f64) -> f64 {
    let n = values.len();
    if n == 0 {
        return f64::INFINITY;
    }
    let nf = n as f64;
    let mean = values.iter().sum::<f64>() / nf;
    if n > 1 {
        let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (nf - 1.0);
        if var > 0.0 {
            return (var / nf).sqrt();
        }
    }
    if counts {
        (mean.max(-alpha.ln() / nf) / nf).sqrt()
    } else {
        step / (12.0 * nf).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: f64, want: f64, tol: f64) {
        assert!((got - want).abs() < tol, "got {got}, want {want}");
    }

    #[test]
    fn zero_events_in_a_thousand_matches_give_the_exact_upper_bound() {
        // One-sided 95 percent: Garwood gives -ln(0.05) / 1000, about 3 / n.
        let (lo, hi) = garwood(0, 1000.0, 0.95);
        assert_eq!(lo, 0.0);
        close(hi, 0.002_995_7, 1e-7);
        close(hi, -(0.05_f64).ln() / 1000.0, 1e-9);
        // Clopper-Pearson for a share: 1 - 0.05^(1/1000).
        let (lo, hi) = clopper_pearson(0, 1000, 0.95);
        assert_eq!(lo, 0.0);
        close(hi, 0.002_991_3, 1e-7);
        close(hi, 1.0 - 0.05_f64.powf(1.0 / 1000.0), 1e-9);
    }

    #[test]
    fn three_in_a_hundred_two_sided() {
        // Clopper-Pearson 95 percent two-sided for 3 of 100: 0.00623 to 0.08518 (R binom.test).
        let (lo, hi) = clopper_pearson(3, 100, 0.975);
        close(lo, 0.006_230, 1e-5);
        close(hi, 0.085_176, 1e-5);
        // Garwood 95 percent two-sided for 3 events: 0.6187 to 8.7673 per unit exposure.
        let (lo, hi) = garwood(3, 1.0, 0.975);
        close(lo, 0.618_7, 1e-4);
        close(hi, 8.767_3, 1e-4);
    }

    #[test]
    fn a_mean_with_no_spread_still_has_an_error() {
        let twos = [2.0; 10];
        let counts = mean_error(&twos, true, 1.0, 0.05);
        assert!(counts > 0.0);
        close(counts, (2.0_f64 / 10.0).sqrt(), 1e-12);
        let measured = mean_error(&twos, false, 0.01, 0.05);
        assert!(measured > 0.0);
        close(measured, 0.01 / 120.0_f64.sqrt(), 1e-12);
        let zeros = [0.0; 10];
        close(
            mean_error(&zeros, true, 1.0, 0.05),
            (-(0.05_f64).ln() / 10.0 / 10.0).sqrt(),
            1e-12,
        );
        let spread = [1.0, 3.0];
        close(mean_error(&spread, true, 1.0, 0.05), 1.0, 1e-12);
    }
}
