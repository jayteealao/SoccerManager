//! Quantiles of the normal, t, chi-square and beta distributions.
//!
//! The standard library has no special functions and `statrs` needs a newer compiler than
//! the workspace allows, so these are built here on `libm`'s gamma and error functions. Each
//! quantile is pinned to published values in the tests below.

use std::f64::consts::{PI, SQRT_2};

/// The standard normal distribution function.
pub fn norm_cdf(x: f64) -> f64 {
    0.5 * libm::erfc(-x / SQRT_2)
}

/// The standard normal quantile: Acklam's rational start, then one Halley step on `erfc`,
/// which brings it to full double precision.
pub fn norm_quantile(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    let low = 0.024_25;
    let x = if p < low {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - low {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    let e = norm_cdf(x) - p;
    let u = e * (2.0 * PI).sqrt() * (x * x / 2.0).exp();
    x - u / (1.0 + x * u / 2.0)
}

/// The natural log of the beta function.
pub fn ln_beta(a: f64, b: f64) -> f64 {
    libm::lgamma(a) + libm::lgamma(b) - libm::lgamma(a + b)
}

/// The regularized incomplete beta function `I_x(a, b)`, by Lentz's continued fraction.
pub fn inc_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front = (a * x.ln() + b * libm::log1p(-x) - ln_beta(a, b)).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_fraction(a, b, x) / a
    } else {
        1.0 - front * beta_fraction(b, a, 1.0 - x) / b
    }
}

fn beta_fraction(a: f64, b: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    let mut c = 1.0;
    let mut d = 1.0 - (a + b) * x / (a + 1.0);
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=10_000 {
        let m = f64::from(m);
        let m2 = 2.0 * m;
        // The even and the odd term of the fraction, each one Lentz step.
        let even = m * (b - m) * x / ((a + m2 - 1.0) * (a + m2));
        let odd = -(a + m) * (a + b + m) * x / ((a + m2) * (a + m2 + 1.0));
        let mut last = 1.0;
        for coef in [even, odd] {
            d = 1.0 + coef * d;
            if d.abs() < TINY {
                d = TINY;
            }
            c = 1.0 + coef / c;
            if c.abs() < TINY {
                c = TINY;
            }
            d = 1.0 / d;
            last = d * c;
            h *= last;
        }
        if (last - 1.0).abs() < 1e-15 {
            break;
        }
    }
    h
}

/// The regularized lower incomplete gamma function `P(a, x)`: the series below `a + 1`,
/// the continued fraction above it.
pub fn inc_gamma(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let ln_front = a * x.ln() - x - libm::lgamma(a);
    if x < a + 1.0 {
        let mut term = 1.0 / a;
        let mut sum = term;
        let mut ap = a;
        for _ in 0..10_000 {
            ap += 1.0;
            term *= x / ap;
            sum += term;
            if term.abs() < sum.abs() * 1e-16 {
                break;
            }
        }
        sum * ln_front.exp()
    } else {
        const TINY: f64 = 1e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / TINY;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..=10_000 {
            let i = f64::from(i);
            let an = -i * (i - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < TINY {
                d = TINY;
            }
            c = b + an / c;
            if c.abs() < TINY {
                c = TINY;
            }
            d = 1.0 / d;
            let delta = d * c;
            h *= delta;
            if (delta - 1.0).abs() < 1e-16 {
                break;
            }
        }
        1.0 - ln_front.exp() * h
    }
}

/// Student's t distribution function with `df` degrees of freedom.
pub fn t_cdf(t: f64, df: f64) -> f64 {
    let tail = 0.5 * inc_beta(df / 2.0, 0.5, df / (df + t * t));
    if t > 0.0 { 1.0 - tail } else { tail }
}

/// The chi-square distribution function with `k` degrees of freedom.
pub fn chi2_cdf(x: f64, k: f64) -> f64 {
    inc_gamma(k / 2.0, x / 2.0)
}

/// Inverts a rising distribution function by bisection on a bracket that grows until it
/// holds the answer; the result is exact to the last few bits, which is all the judging
/// needs, and is computed once per band rather than per resample.
fn invert(cdf: impl Fn(f64) -> f64, p: f64, mut lo: f64, mut hi: f64, grow_lo: bool) -> f64 {
    while cdf(hi) < p {
        lo = hi;
        hi *= 2.0;
    }
    if grow_lo {
        while cdf(lo) > p {
            hi = lo;
            lo *= 2.0;
        }
    }
    for _ in 0..400 {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if cdf(mid) < p { lo = mid } else { hi = mid }
    }
    0.5 * (lo + hi)
}

/// The `p` quantile of Student's t with `df` degrees of freedom.
pub fn t_quantile(p: f64, df: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    if p == 0.5 {
        return 0.0;
    }
    if p < 0.5 {
        return -t_quantile(1.0 - p, df);
    }
    invert(|t| t_cdf(t, df), p, 0.0, 1.0, false)
}

/// The `p` quantile of the chi-square distribution with `k` degrees of freedom.
pub fn chi2_quantile(p: f64, k: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    invert(|x| chi2_cdf(x, k), p, 0.0, k.max(1.0), false)
}

/// The `p` quantile of the beta distribution with shapes `a` and `b`.
pub fn beta_quantile(p: f64, a: f64, b: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    for _ in 0..400 {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if inc_beta(a, b, mid) < p {
            lo = mid
        } else {
            hi = mid
        }
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: f64, want: f64) {
        assert!((got - want).abs() < 1e-6, "got {got}, want {want}");
    }

    #[test]
    fn normal_quantiles_match_published_values() {
        close(norm_quantile(0.95), 1.644_853_6);
        close(norm_quantile(0.975), 1.959_964_0);
        close(norm_quantile(0.025), -1.959_964_0);
        close(norm_quantile(0.5), 0.0);
        close(norm_cdf(norm_quantile(1e-6)), 1e-6);
        close(norm_cdf(1.959_964), 0.975);
    }

    #[test]
    fn t_quantiles_match_published_values() {
        close(t_quantile(0.975, 6.0), 2.446_911_9);
        close(t_quantile(0.95, 2.0), 2.919_985_6);
        close(t_quantile(0.025, 6.0), -2.446_911_9);
        close(t_quantile(0.975, 1.0), 12.706_204_7);
    }

    #[test]
    fn chi_square_quantiles_match_published_values() {
        close(chi2_quantile(0.95, 2.0), 5.991_464_5);
        close(chi2_quantile(0.025, 10.0), 3.246_972_7);
        close(chi2_quantile(0.975, 2.0), 7.377_758_9);
    }

    #[test]
    fn beta_quantiles_match_published_values() {
        close(beta_quantile(0.025, 1.0, 20.0), 0.001_265_1);
        close(beta_quantile(0.5, 2.0, 2.0), 0.5);
        close(inc_beta(2.0, 3.0, 0.4), 0.524_8);
    }
}
