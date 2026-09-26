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
