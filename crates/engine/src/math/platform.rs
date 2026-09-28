//! The platform maths backend: the only file in the engine that calls a platform
//! transcendental function. Each function is the original operation unchanged, so this
//! backend gives today's results. The one recorded result change removes it.

/// Sine, from the platform maths library.
#[inline]
pub fn sin(x: f64) -> f64 {
    x.sin()
}

/// Cosine, from the platform maths library.
#[inline]
pub fn cos(x: f64) -> f64 {
    x.cos()
}

/// Sine and cosine in one call, from the platform maths library.
#[inline]
pub fn sin_cos(x: f64) -> (f64, f64) {
    x.sin_cos()
}

/// `e` to the power `x`, from the platform maths library.
#[inline]
pub fn exp(x: f64) -> f64 {
    x.exp()
}

/// The four-quadrant arctangent of `y / x`, from the platform maths library.
#[inline]
pub fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

/// `x` to the integer power `n`, from the platform maths library.
#[inline]
pub fn powi(x: f64, n: i32) -> f64 {
    x.powi(n)
}

#[cfg(test)]
mod tests {
    use super::super::tests::same_bits;

    /// Each platform function is the original method call it replaced, bit for bit, so the
    /// platform backend cannot move a result. The original calls live here, in the one file
    /// the platform-maths search allows.
    #[test]
    fn platform_backend_equals_the_original_calls() {
        let cases = same_bits(
            (
                super::sin,
                super::cos,
                super::sin_cos,
                super::exp,
                super::atan2,
                super::powi,
            ),
            (
                |x| x.sin(),
                |x| x.cos(),
                |x| x.sin_cos(),
                |x| x.exp(),
                |y, x| y.atan2(x),
                |x, n| x.powi(n),
            ),
        );
        assert!(cases > 40_000, "{cases} cases");
    }
}
