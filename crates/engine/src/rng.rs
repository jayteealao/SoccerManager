//! The engine-owned, seedable random-number generator.
//!
//! `ChaCha8Rng` is the generator the Rand Book names as reproducible across versions
//! (source: https://rust-random.github.io/book/). The engine never uses a thread-local
//! generator. Match play draws every random number through the keyed stream registry
//! ([`crate::streams`]); `EngineRng` serves offline generation (club and player data) and is
//! the reference the registry's scheme 0 is tested against.

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// The exact position of a generator: enough to continue its stream draw for draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RngState {
    pub seed: [u8; 32],
    pub stream: u64,
    pub word_pos: u128,
}

/// The random-stream scheme this build plays ([`crate::streams::Scheme`]): scheme 0, one
/// shared stream, or scheme 1, one keyed stream per key, under the `keyed-streams` feature.
/// The golden file's ledger records it with every entry.
pub const STREAM_SCHEME: u8 = if cfg!(feature = "keyed-streams") {
    crate::streams::KEYED_SCHEME
} else {
    0
};

/// The position of every random stream a match draws from, as the replay gate hashes it:
/// the stream scheme, then each stream's id and word position in stream order. Today's
/// engine has one stream, so it reports scheme 0 with one entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamState {
    pub scheme: u8,
    /// `(stream id, word position)` per stream, in stream order.
    pub entries: Vec<(u64, u128)>,
}

impl StreamState {
    /// The canonical bytes: the scheme id (u8), the entry count (u32), then per entry the
    /// stream id (u64) and the word position (u128), all little-endian.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(5 + 24 * self.entries.len());
        out.push(self.scheme);
        // A match has a handful of streams, far below 4 billion.
        out.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for (stream, word_pos) in &self.entries {
            out.extend_from_slice(&stream.to_le_bytes());
            out.extend_from_slice(&word_pos.to_le_bytes());
        }
        out
    }
}

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

    /// The stream state the replay gate hashes: scheme 0, one entry.
    pub fn stream_state(&self) -> StreamState {
        StreamState {
            scheme: STREAM_SCHEME,
            entries: vec![(self.inner.get_stream(), self.inner.get_word_pos())],
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
    #[cfg(not(feature = "keyed-streams"))]
    fn todays_stream_state_is_scheme_0_with_one_entry_in_the_documented_bytes() {
        let mut rng = EngineRng::from_seed(42);
        for _ in 0..3 {
            rng.next_f64();
        }
        let state = rng.stream_state();
        assert_eq!(state.scheme, 0);
        assert_eq!(state.entries.len(), 1);
        let (stream, word_pos) = state.entries[0];
        assert_eq!(stream, rng.state().stream);
        assert_eq!(word_pos, rng.state().word_pos);
        // Three f64 draws read six 32-bit words.
        assert_eq!(word_pos, 6);
        let mut expected = vec![0u8];
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(&stream.to_le_bytes());
        expected.extend_from_slice(&6u128.to_le_bytes());
        assert_eq!(state.to_bytes(), expected);
        assert_eq!(state.to_bytes().len(), 1 + 4 + 8 + 16);
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
