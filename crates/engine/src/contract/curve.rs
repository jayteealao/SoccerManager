//! The curve and the chance forms. Every exponent and logarithm goes through
//! [`crate::math`] (libm), so each value has the same bits on every machine.

use crate::math;

use super::params::CurveTuning;

/// `F(r) = scale · e^((r − center) / width)`: a rating on the 1 to 20 scale mapped onto the
/// curve that makes the top of the scale count for more. `F(10) = 8` with the shipped curve.
#[inline]
pub fn f(r: f64, c: &CurveTuning) -> f64 {
    c.scale * math::exp((r - c.center) / c.width)
}

/// The rating whose curve value is `v`: the inverse of [`f`].
#[inline]
pub fn f_inv(v: f64, c: &CurveTuning) -> f64 {
    c.center + c.width * math::ln(v / c.scale)
}

/// The log-odds of the chance `p`.
#[inline]
pub fn logit(p: f64) -> f64 {
    math::ln(p / (1.0 - p))
}

/// The chance whose log-odds are `x`.
#[inline]
pub fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + math::exp(-x))
}

/// A contest between curve values `fa` and `fb` with strength `k`, anchored on `base`, the
/// chance when the two are equal: `σ(logit(base) + k · (fa − fb))`, kept inside `floor` and
/// `ceiling`. Equal values return `base` (inside the floor and ceiling) bit for bit.
#[inline]
pub fn contest(base: f64, k: f64, fa: f64, fb: f64, floor: f64, ceiling: f64) -> f64 {
    shifted(base, k * (fa - fb), floor, ceiling)
}

/// `base` moved by `shift` in log-odds, kept inside `floor` and `ceiling`. A zero shift
/// returns `base` (inside the floor and ceiling) bit for bit. A base of 0 or 1 is returned
/// as it is, outside the limits too: no shift in log-odds moves it, and a chance tuned to 0
/// stays switched off.
#[inline]
pub fn shifted(base: f64, shift: f64, floor: f64, ceiling: f64) -> f64 {
    if base <= 0.0 || base >= 1.0 {
        return base;
    }
    let p = if shift == 0.0 {
        base
    } else {
        logistic(logit(base) + shift)
    };
    p.clamp(floor, ceiling)
}

/// The skill share of a curve value `v` with strength `k`: `σ(k · (v − F(10)))`, exactly 0.5
/// at rating 10.
#[inline]
pub fn share(k: f64, v: f64, c: &CurveTuning) -> f64 {
    logistic(k * (v - f(10.0, c)))
}

/// A per-player factor on an average player's value: `1 + spread · (2 · share − 1)`, exactly
/// 1 at rating 10, from `1 − spread` to `1 + spread`.
#[inline]
pub fn factor(spread: f64, share: f64) -> f64 {
    1.0 + spread * (2.0 * share - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> CurveTuning {
        CurveTuning {
            scale: 8.0,
            center: 10.0,
            width: 8.0,
        }
    }

    #[test]
    fn the_curve_matches_the_hand_values() {
        let c = shipped();
        // 8 e^(-6/8), 8 e^0, 8 e^(8/8).
        assert!((f(4.0, &c) - 3.7789).abs() < 1e-4, "{}", f(4.0, &c));
        assert_eq!(f(10.0, &c), 8.0);
        assert!((f(18.0, &c) - 21.7463).abs() < 1e-4, "{}", f(18.0, &c));
        for t in 10..=200u8 {
            let r = f64::from(t) / 10.0;
            assert!((f_inv(f(r, &c), &c) - r).abs() < 1e-12, "{r}");
        }
    }

    #[test]
    fn a_contest_stays_inside_its_floor_and_ceiling_and_equal_values_give_the_base() {
        let c = shipped();
        let (top, bottom) = (f(20.0, &c), f(1.0, &c));
        assert_eq!(contest(0.25, 0.35, top, bottom, 0.02, 0.97), 0.97);
        assert_eq!(contest(0.25, 0.35, bottom, top, 0.02, 0.97), 0.02);
        for base in [0.1, 0.25, 0.333, 0.5, 0.86] {
            let v = f(13.7, &c);
            assert_eq!(
                contest(base, 0.35, v, v, 0.02, 0.97).to_bits(),
                base.to_bits()
            );
        }
        assert_eq!(contest(0.0, 0.35, top, bottom, 0.0, 1.0), 0.0);
    }

    #[test]
    fn the_share_is_a_half_at_rating_ten_and_the_factor_is_one() {
        let c = shipped();
        assert_eq!(share(0.35, f(10.0, &c), &c), 0.5);
        assert_eq!(factor(0.2, 0.5), 1.0);
        assert!(share(0.35, f(16.0, &c), &c) > share(0.35, f(12.0, &c), &c));
        assert!((factor(0.2, 1.0) - 1.2).abs() < 1e-15);
    }
}
