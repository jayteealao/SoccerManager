//! A tiny seeded generator. It lives here instead of coming from the `rand`
//! crate so that the stream for a seed can never change under a dependency
//! upgrade: a genome seed must mean the same face forever.

/// SplitMix64 (Steele, Lea, Flood 2014). Public-domain algorithm.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Derives an independent stream, so adding a gene later does not shift
    /// the values of the genes that already exist.
    pub fn fork(seed: u64, stream: &str) -> Self {
        let mut h = seed ^ 0x9e37_79b9_7f4a_7c15;
        for b in stream.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
        let mut r = Self::new(h);
        r.next_u64();
        r
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    /// Standard normal by Box-Muller.
    pub fn normal(&mut self) -> f32 {
        let u1 = self.f32().max(1e-7);
        let u2 = self.f32();
        (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
    }

    /// Index drawn from unnormalised weights.
    pub fn pick(&mut self, weights: &[f32]) -> usize {
        let total: f32 = weights.iter().sum();
        let mut x = self.f32() * total;
        for (i, w) in weights.iter().enumerate() {
            if x < *w {
                return i;
            }
            x -= w;
        }
        weights.len() - 1
    }
}
