//! Vector types and small helpers. All simulation math is `f64`.

pub use glam::{DVec2, DVec3};

/// Clamps `v` to at most `max` in length. A zero vector stays zero.
pub fn clamp_len(v: DVec2, max: f64) -> DVec2 {
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

    #[test]
    fn segment_distance_projects_inside_and_clamps_outside() {
        let a = DVec2::ZERO;
        let b = DVec2::new(10.0, 0.0);
        assert!((segment_distance(DVec2::new(5.0, 3.0), a, b) - 3.0).abs() < 1e-12);
        assert!((segment_distance(DVec2::new(-4.0, 0.0), a, b) - 4.0).abs() < 1e-12);
    }
}
