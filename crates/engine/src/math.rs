//! Vector types, small helpers, and the engine maths functions. All simulation maths is
//! `f64`. Every sine, cosine, exponent, logarithm, arctangent, and integer power in the engine goes
//! through the functions below, which call the pure-Rust `libm` crate: the same bits on every
//! machine. A clippy rule (`crates/engine/clippy.toml`) refuses the platform maths methods in
//! the engine crate.

pub use glam::{DVec2, DVec3};

/// Sine of `x` radians.
#[inline]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

/// Cosine of `x` radians.
#[inline]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

/// Sine and cosine of `x` radians, in one call.
#[inline]
pub fn sin_cos(x: f64) -> (f64, f64) {
    libm::sincos(x)
}

/// `e` to the power `x`.
#[inline]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// The natural logarithm of `x`.
#[inline]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

/// The four-quadrant arctangent of `y / x`.
#[inline]
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

/// `x` to the integer power `n`. libm has no integer power function, so this is `pow` with
/// the integer as its exponent (source: libm-0.2.16/src/math/mod.rs lists no `powi`).
#[inline]
pub fn powi(x: f64, n: i32) -> f64 {
    libm::pow(x, f64::from(n))
}

/// Clamps `v` to at most `max` in length. A zero vector stays zero. A vector whose squared
/// length is at or below `sq_keep_limit(max)` is returned before the square root; every
/// other vector, and every negative or NaN `max`, runs the square-root form unchanged.
pub fn clamp_len(v: DVec2, max: f64) -> DVec2 {
    if max >= 0.0 && v.length_squared() <= sq_keep_limit(max) {
        return v;
    }
    let len = v.length();
    if len > max && len > 0.0 {
        v * (max / len)
    } else {
        v
    }
}

/// The squared-length skip limit for a radius `r` >= 0: every squared length at or above
/// this value has a rounded square root of at least `r`, so `d.length() < r` is false for it.
/// `r * r` is correctly rounded, so the exact square lies below the next number up; a squared
/// length at or above that number has an exact root above `r`, and a representable `r` is
/// its own rounding. For a negative, zero, or NaN `r`, `dist < r` is false for every root, or
/// every compare with NaN is false, so a skip on this limit is never wrong. An underflowing
/// square gives the smallest subnormal and an overflowing one gives infinity; both stay
/// conservative.
pub(crate) fn sq_skip_limit(r: f64) -> f64 {
    (r * r).next_up()
}

/// The squared-length keep limit for a limit `m` >= 0: every squared length at or below
/// this value has a rounded square root of at most `m`, so `v.length() > m` is false for it.
/// This is the mirror of `sq_skip_limit`. An underflowing square gives a negative limit, so
/// no vector is kept early; an overflowing one gives `f64::MAX`, and every finite squared
/// length has a root below such an `m`. Callers check `m >= 0.0` first.
pub(crate) fn sq_keep_limit(m: f64) -> f64 {
    (m * m).next_down()
}

/// Unit vector from `from` toward `to`, or zero when the points coincide.
pub fn toward(from: DVec2, to: DVec2) -> DVec2 {
    let d = to - from;
    let len = d.length();
    if len > 1e-9 { d / len } else { DVec2::ZERO }
}

/// Distance from point `p` to the segment `a`–`b`.
pub fn segment_distance(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-12 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    (a + ab * t - p).length()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::EngineRng;

    /// The radii every threshold test sweeps: signed zeros, a negative, NaN, infinity, the
    /// smallest subnormal, a radius whose square underflows, the shipped and nearby radii,
    /// and radii whose square overflows.
    fn edge_radii() -> Vec<f64> {
        vec![
            0.0,
            -0.0,
            -1.0,
            f64::NAN,
            f64::INFINITY,
            f64::from_bits(1),
            1e-160,
            0.08,
            0.3,
            0.4,
            1.0,
            1.000_000_000_000_000_2,
            1.5,
            2.9,
            std::f64::consts::SQRT_2,
            15.0,
            1e154,
            1e155,
            1e200,
        ]
    }

    /// The edge radii and 10,000 seeded radii in (0, 15].
    fn radii() -> Vec<f64> {
        let mut rng = EngineRng::from_seed(15);
        let mut out = edge_radii();
        out.extend((0..10_000).map(|_| 15.0 * (1.0 - rng.next_f64())));
        out
    }

    /// `x` and the 64 representable values on each side of it.
    fn around(x: f64) -> Vec<f64> {
        let mut out = vec![x];
        let (mut up, mut down) = (x, x);
        for _ in 0..64 {
            up = up.next_up();
            down = down.next_down();
            out.push(up);
            out.push(down);
        }
        out
    }

    /// The squared lengths swept for radius `r`: `r * r` +- 64 ulps and the special values.
    fn squared_lengths(r: f64) -> Vec<f64> {
        let mut out = around(r * r);
        out.extend([0.0, f64::from_bits(1), f64::MAX, f64::INFINITY, f64::NAN]);
        out
    }

    #[test]
    fn skip_limit_matches_the_square_root_form() {
        let mut cases = 0u64;
        for r in radii() {
            let skip = sq_skip_limit(r);
            for d2 in squared_lengths(r) {
                let old = d2.sqrt() < r;
                // The code's form: skip at or above the limit, else the square root.
                let new = if d2 >= skip { false } else { d2.sqrt() < r };
                assert_eq!(new, old, "r = {r:e}, d2 = {d2:e}");
                cases += 1;
            }
        }
        assert!(cases > 1_000_000, "{cases} cases");
    }

    /// Today's `clamp_len` before the squared-length keep, for the threshold tests.
    fn clamp_len_before(v: DVec2, max: f64) -> DVec2 {
        let len = v.length();
        if len > max && len > 0.0 {
            v * (max / len)
        } else {
            v
        }
    }

    fn bits(v: DVec2) -> (u64, u64) {
        (v.x.to_bits(), v.y.to_bits())
    }

    #[test]
    fn keep_limit_matches_the_square_root_form() {
        let mut limits = radii();
        limits.extend([f64::MAX]);
        let (sin, cos) = (sin(30f64.to_radians()), cos(30f64.to_radians()));
        let (mut cases, mut at_square, mut kept, mut scaled) = (0u64, 0u64, 0u64, 0u64);
        for m in limits {
            let keep = sq_keep_limit(m);
            // The branch alone: a squared length kept early must not be clamped by the
            // square-root form.
            for d2 in squared_lengths(m) {
                let early = m >= 0.0 && d2 <= keep;
                if early {
                    let len = d2.sqrt();
                    assert!(!(len > m && len > 0.0), "m = {m:e}, d2 = {d2:e}");
                }
            }
            // Whole vectors along an axis and along 30 degrees, lengths m +- 64 ulps; the
            // squared length each case tests is its own `length_squared()`.
            let mut vectors = vec![
                DVec2::ZERO,
                DVec2::new(-0.0, 0.0),
                DVec2::new(f64::INFINITY, 0.0),
                DVec2::new(f64::NAN, 0.0),
                DVec2::new(1e200, 1e200),
                DVec2::new(1e-170, 0.0),
            ];
            for len in around(m) {
                vectors.push(DVec2::new(len, 0.0));
                vectors.push(DVec2::new(len * cos, len * sin));
            }
            for v in vectors {
                let d2 = v.length_squared();
                let new = clamp_len(v, m);
                assert_eq!(
                    bits(new),
                    bits(clamp_len_before(v, m)),
                    "m = {m:e}, v = {v:?}, d2 = {d2:e}"
                );
                if (m * m) > 0.0 && d2.to_bits() == (m * m).to_bits() {
                    at_square += 1;
                }
                if m >= 0.0 && d2 <= keep {
                    kept += 1;
                } else {
                    scaled += 1;
                }
                cases += 1;
            }
        }
        // The sweep reaches the square itself (the one-ulp band above the keep limit) and
        // both sides of the limit.
        assert!(at_square > 10_000, "{at_square} cases at the square");
        assert!(
            kept > 100_000 && scaled > 100_000,
            "{kept} kept, {scaled} scaled"
        );
        assert!(cases > 2_000_000, "{cases} cases");
    }

    #[test]
    fn a_plain_squared_compare_is_caught() {
        // Control: the plain form picks another branch than the square-root form one ulp
        // below the shipped overlap radius, so a sweep that compares branches is not blind.
        let r: f64 = 0.4;
        let d2 = (r * r).next_down();
        assert_ne!(d2 < r * r, d2.sqrt() < r);
        // The one-sided form keeps the square-root branch for that value.
        let skip = sq_skip_limit(r);
        let new = if d2 >= skip { false } else { d2.sqrt() < r };
        assert_eq!(new, d2.sqrt() < r);
    }

    #[test]
    fn clamp_len_limits_long_vectors_only() {
        assert_eq!(clamp_len(DVec2::new(3.0, 4.0), 10.0), DVec2::new(3.0, 4.0));
        assert!((clamp_len(DVec2::new(3.0, 4.0), 1.0).length() - 1.0).abs() < 1e-12);
        assert_eq!(clamp_len(DVec2::ZERO, 1.0), DVec2::ZERO);
    }

    #[test]
    fn toward_is_unit_or_zero() {
        assert_eq!(
            toward(DVec2::ZERO, DVec2::new(0.0, 2.0)),
            DVec2::new(0.0, 1.0)
        );
        assert_eq!(toward(DVec2::ONE, DVec2::ONE), DVec2::ZERO);
    }

    /// The fixed maths input set: signed zeros, the smallest subnormal and normal, a tiny
    /// value, the quarter, half, and full turns and the neighbours of the quarter turn, the 22
    /// centre-circle angles, large arguments, the edges of `exp`, the largest finite values,
    /// the infinities, NaN, and 10,000 seeded values in [-10, 10).
    pub(super) fn maths_inputs() -> Vec<f64> {
        use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};
        let mut out = vec![
            0.0,
            -0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            1e-300,
            f64::NAN,
        ];
        for x in [
            FRAC_PI_4,
            FRAC_PI_4.next_down(),
            FRAC_PI_4.next_up(),
            FRAC_PI_2,
            PI,
            TAU,
            1e6,
            1e22,
            709.78,
            745.2,
            f64::MAX,
            f64::INFINITY,
        ] {
            out.push(x);
            out.push(-x);
        }
        out.extend((0..22).map(|i| TAU * f64::from(i) / 22.0));
        let mut rng = EngineRng::from_seed(29);
        out.extend((0..10_000).map(|_| 20.0 * rng.next_f64() - 10.0));
        out
    }

    /// The `atan2` operand pairs: every pair of the first 64 inputs, which holds all four
    /// signed-zero quadrants.
    fn atan2_pairs() -> Vec<(f64, f64)> {
        let inputs: Vec<f64> = maths_inputs().into_iter().take(64).collect();
        let mut out = Vec::new();
        for &y in &inputs {
            for &x in &inputs {
                out.push((y, x));
            }
        }
        out
    }

    /// The `powi` operands: the bases with every power from -8 to 22.
    fn powi_pairs() -> Vec<(f64, i32)> {
        let mut out = Vec::new();
        for x in [10.0, 2.0, -1.5, 0.0, -0.0, f64::INFINITY, f64::NAN] {
            for n in -8..=22 {
                out.push((x, n));
            }
        }
        out
    }

    /// The six maths functions of one backend: sin, cos, sin_cos, exp, atan2, and powi.
    pub(super) type Backends = (
        fn(f64) -> f64,
        fn(f64) -> f64,
        fn(f64) -> (f64, f64),
        fn(f64) -> f64,
        fn(f64, f64) -> f64,
        fn(f64, i32) -> f64,
    );

    /// Checks one maths backend, given as its six functions, against six reference functions
    /// bit for bit over the fixed input set, and returns the number of compares.
    pub(super) fn same_bits(got: Backends, want: Backends) -> usize {
        let mut cases = 0;
        for x in maths_inputs() {
            assert_eq!(got.0(x).to_bits(), want.0(x).to_bits(), "sin({x:e})");
            assert_eq!(got.1(x).to_bits(), want.1(x).to_bits(), "cos({x:e})");
            let (s, c) = got.2(x);
            let (ws, wc) = want.2(x);
            assert_eq!(
                (s.to_bits(), c.to_bits()),
                (ws.to_bits(), wc.to_bits()),
                "sin_cos({x:e})"
            );
            assert_eq!(got.3(x).to_bits(), want.3(x).to_bits(), "exp({x:e})");
            cases += 4;
        }
        for (y, x) in atan2_pairs() {
            assert_eq!(
                got.4(y, x).to_bits(),
                want.4(y, x).to_bits(),
                "atan2({y:e}, {x:e})"
            );
            cases += 1;
        }
        for (x, n) in powi_pairs() {
            assert_eq!(
                got.5(x, n).to_bits(),
                want.5(x, n).to_bits(),
                "powi({x:e}, {n})"
            );
            cases += 1;
        }
        cases
    }

    #[test]
    fn the_maths_functions_equal_the_direct_libm_calls() {
        let cases = same_bits(
            (sin, cos, sin_cos, exp, atan2, powi),
            (
                libm::sin,
                libm::cos,
                libm::sincos,
                libm::exp,
                libm::atan2,
                |x, n| libm::pow(x, f64::from(n)),
            ),
        );
        assert!(cases > 40_000, "{cases} cases");
    }

    #[test]
    fn segment_distance_projects_inside_and_clamps_outside() {
        let a = DVec2::ZERO;
        let b = DVec2::new(10.0, 0.0);
        assert!((segment_distance(DVec2::new(5.0, 3.0), a, b) - 3.0).abs() < 1e-12);
        assert!((segment_distance(DVec2::new(-4.0, 0.0), a, b) - 4.0).abs() < 1e-12);
    }
}
