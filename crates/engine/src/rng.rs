//! The engine-owned, seedable random-number generator.
//!
//! `ChaCha8Rng` is the generator the Rand Book names as reproducible across versions
//! (source: https://rust-random.github.io/book/). The engine never uses a thread-local
//! generator; every random draw goes through one `EngineRng` passed by mutable reference.

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// A seeded generator with a fixed algorithm.
pub struct EngineRng {
    inner: ChaCha8Rng,
}

impl EngineRng {
    /// Creates a generator from a 64-bit seed.
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// A value in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        self.inner.random::<f64>()
    }

    /// A value in `[lo, hi)`.
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    /// An index in `[0, n)`; `n` must be positive.
    pub fn range_usize(&mut self, n: usize) -> usize {
        self.inner.random_range(0..n)
    }

    /// `true` with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// A bell-curve draw: `mean` plus `spread` times a unit normal approximated by the sum of
    /// twelve uniform draws minus six (Irwin-Hall; product-owner choice, plan Q5).
    pub fn bell(&mut self, mean: f64, spread: f64) -> f64 {
        let sum: f64 = (0..12).map(|_| self.next_f64()).sum();
        mean + spread * (sum - 6.0)
    }

    /// A bell-curve draw rounded and clamped to the 1 to 100 attribute scale.
    pub fn attribute(&mut self, mean: f64, spread: f64) -> u8 {
        // The clamp keeps the value inside 1..=100, so the cast cannot truncate.
        self.bell(mean, spread).round().clamp(1.0, 100.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a = EngineRng::from_seed(42);
        let mut b = EngineRng::from_seed(42);
        for _ in 0..1000 {
            assert_eq!(a.next_f64().to_bits(), b.next_f64().to_bits());
        }
    }

    #[test]
    fn different_seed_different_stream() {
        let mut a = EngineRng::from_seed(1);
        let mut b = EngineRng::from_seed(2);
        assert_ne!(a.next_f64().to_bits(), b.next_f64().to_bits());
    }

    #[test]
    fn attribute_draws_centre_on_the_mean_and_stay_in_range() {
        let mut rng = EngineRng::from_seed(7);
        let mut sum = 0.0;
        for _ in 0..10_000 {
            let v = rng.attribute(50.0, 10.0);
            assert!((1..=100).contains(&v));
            sum += f64::from(v);
        }
        let mean = sum / 10_000.0;
        assert!((49.0..=51.0).contains(&mean), "sample mean {mean}");
    }
}
