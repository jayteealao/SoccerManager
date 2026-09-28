//! The libm maths backend: pure-Rust functions that give the same bits on every machine.

/// Sine, from `libm`.
#[inline]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

/// Cosine, from `libm`.
#[inline]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

/// Sine and cosine in one call, from `libm`.
#[inline]
pub fn sin_cos(x: f64) -> (f64, f64) {
    libm::sincos(x)
}

/// `e` to the power `x`, from `libm`.
#[inline]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// The four-quadrant arctangent of `y / x`, from `libm`.
#[inline]
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

/// `x` to the integer power `n`, from `libm`. libm has no integer power function, so this is
/// `pow` with the integer as its exponent (source: libm-0.2.16/src/math/mod.rs lists no `powi`).
#[inline]
pub fn powi(x: f64, n: i32) -> f64 {
    libm::pow(x, f64::from(n))
}
