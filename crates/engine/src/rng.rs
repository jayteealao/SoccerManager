//! The engine-owned, seedable random-number generator.
//!
//! `ChaCha8Rng` is the generator the Rand Book names as reproducible across versions
//! (source: https://rust-random.github.io/book/). The engine never uses a thread-local
//! generator. Match play draws every random number through the keyed stream registry
//! ([`crate::streams`]); `EngineRng` serves offline generation (club and player data).

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// The exact position of a generator: enough to continue its stream draw for draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RngState {
    pub seed: [u8; 32],
    pub stream: u64,
    pub word_pos: u128,
}

// The stream registry owns these; the paths through `rng` stay for other crates.
pub use crate::streams::{STREAM_SCHEME, StreamState};

/// A seeded generator with a fixed algorithm.
pub struct EngineRng {
    inner: ChaCha8Rng,
}

impl EngineRng {
    /// Creates a generator from a 64-bit seed.
    pub fn from_seed(seed: u64) -> Self {
        Self::wrap(ChaCha8Rng::seed_from_u64(seed))
    }

    fn wrap(inner: ChaCha8Rng) -> Self {
        Self { inner }
    }

    /// The generator's position (source: `rand_chacha-0.10.0/src/chacha.rs:145-198` in the
    /// local cargo registry, `get_seed`, `get_stream`, and `get_word_pos`).
    pub fn state(&self) -> RngState {
        RngState {
            seed: self.inner.get_seed(),
            stream: self.inner.get_stream(),
            word_pos: self.inner.get_word_pos(),
        }
    }

    /// A generator that continues from `state`.
    pub fn from_state(state: RngState) -> Self {
        let mut inner = ChaCha8Rng::from_seed(state.seed);
        inner.set_stream(state.stream);
        inner.set_word_pos(state.word_pos);
        Self::wrap(inner)
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

    /// A bell-curve draw on the 1 to 20 scale, rounded to a tenth and kept to 1.0 to 20.0.
    pub fn rating(&mut self, mean: f64, spread: f64) -> crate::rating::Rating {
        let tenths = (self.bell(mean, spread) * 10.0).round().clamp(
            f64::from(crate::rating::MIN_TENTHS),
            f64::from(crate::rating::MAX_TENTHS),
        );
        // The clamp keeps the value inside 10..=200, so the cast cannot truncate.
        crate::rating::Rating::from_tenths(tenths as u8)
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
    fn a_restored_state_continues_the_stream() {
        let mut a = EngineRng::from_seed(42);
        for _ in 0..500 {
            a.next_f64();
        }
        let mut b = EngineRng::from_state(a.state());
        for _ in 0..500 {
            assert_eq!(a.next_f64().to_bits(), b.next_f64().to_bits());
        }
        assert_eq!(a.state(), b.state());
    }

    #[test]
    fn different_seed_different_stream() {
        let mut a = EngineRng::from_seed(1);
        let mut b = EngineRng::from_seed(2);
        assert_ne!(a.next_f64().to_bits(), b.next_f64().to_bits());
    }

    #[test]
    fn rating_draws_centre_on_the_mean_and_stay_in_range() {
        let mut rng = EngineRng::from_seed(7);
        let mut sum = 0.0;
        for _ in 0..10_000 {
            let v = rng.rating(10.0, 2.0);
            assert!((10..=200).contains(&v.tenths()));
            sum += v.decimal();
        }
        let mean = sum / 10_000.0;
        assert!((9.9..=10.1).contains(&mean), "sample mean {mean}");
    }
}
