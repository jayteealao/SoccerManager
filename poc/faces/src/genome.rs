//! The face genome: a small seeded record that fully decides a regen's face.
//!
//! Generation reads only the seed, the nation, and the birth year. Each gene
//! draws from its own forked random stream, so adding a gene later does not
//! change existing genes for old seeds.

use crate::pools::{self, Nation, PoolId, POOLS};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// Bump when the meaning of any field changes.
pub const GENOME_VERSION: u8 = 1;

/// Shape axes. Each is a signed coefficient in [-1, 1] that `head` maps to a
/// pair of MakeHuman targets (negative -> "decr" side, positive -> "incr").
pub const SHAPE_AXES: [&str; 24] = [
    "head-width",
    "head-height",
    "head-depth",
    "head-square-oval",
    "face-fullness",
    "jaw-width",
    "chin-height",
    "chin-prominence",
    "jaw-bones",
    "nose-width",
    "nose-length",
    "nose-hump",
    "nose-tip-width",
    "nostril-flare",
    "mouth-width",
    "lower-lip",
    "upper-lip",
    "cheekbones",
    "eye-size",
    "epicanthus",
    "brow-height",
    "forehead-height",
    "ear-size",
    "neck-width",
];

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HairType {
    Straight,
    Wavy,
    Curly,
    Coily,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyeColour {
    Brown,
    Hazel,
    Green,
    Blue,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Share {
    pub pool: PoolId,
    pub share: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HairColour {
    /// 0 = pale blond, 1 = black.
    pub dark: f32,
    /// 0 = none, 1 = strong red.
    pub red: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Hairline {
    /// Resting hairline height, -1 = low, 1 = high.
    pub height: f32,
    /// How far the hairline recedes by 70, 0..1.
    pub recession: f32,
    /// Age when recession starts.
    pub onset: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Greying {
    /// Age at which the first grey shows.
    pub onset: f32,
    /// Years from first grey to mostly grey.
    pub span: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Genome {
    pub v: u8,
    pub seed: u64,
    pub nation: String,
    pub birth_year: u16,
    pub ancestry: Vec<Share>,
    /// MakeHuman macro mix [african, asian, caucasian], sums to 1.
    pub mh_mix: [f32; 3],
    /// Continuous skin tone, 0 = lightest, 1 = darkest.
    pub skin_tone: f32,
    /// Undertone, -1 = cool/pink, 1 = warm/olive.
    pub undertone: f32,
    pub hair_type: HairType,
    pub hair_colour: HairColour,
    pub hairline: Hairline,
    pub greying: Greying,
    pub eye_colour: EyeColour,
    /// Shade within the eye class, 0 = dark, 1 = light.
    pub eye_shade: f32,
    pub beard: f32,
    /// Tendency to carry facial fat later in life, 0..1.
    pub fat_tendency: f32,
    /// One coefficient per `SHAPE_AXES` entry, in [-1, 1].
    pub shape: Vec<f32>,
}

/// Rounds so JSON stays short and round-trips exactly.
fn q(x: f32) -> f32 {
    (x * 1000.0).round() / 1000.0
}

fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

impl Genome {
    pub fn generate(seed: u64, nation: &Nation, birth_year: u16) -> Genome {
        let ancestry = draw_ancestry(seed, nation);
        let pools: Vec<(&pools::Pool, f32)> =
            ancestry.iter().map(|s| (pools::pool(s.pool), s.share)).collect();
        let blend = |f: &dyn Fn(&pools::Pool) -> f32| -> f32 {
            pools.iter().map(|(p, w)| f(p) * w).sum()
        };
        // Discrete genes pick one ancestral pool, weighted by share.
        let pick_pool = |r: &mut Rng| -> &pools::Pool {
            let w: Vec<f32> = pools.iter().map(|(_, w)| *w).collect();
            pools[r.pick(&w)].0
        };

        let mut r = Rng::fork(seed, "mh_mix");
        let mut mh = [0.0f32; 3];
        for (i, m) in mh.iter_mut().enumerate() {
            *m = (blend(&|p| p.mh_mix[i]) + 0.04 * r.normal()).max(0.0);
        }
        let total: f32 = mh.iter().sum::<f32>().max(1e-6);
        let mh_mix = mh.map(|m| q(m / total));

        let mut r = Rng::fork(seed, "skin");
        let sd = blend(&|p| p.skin.1);
        let skin_tone = q(clamp01(blend(&|p| p.skin.0) + sd * r.normal()));
        let undertone = q((0.5 * r.normal()).clamp(-1.0, 1.0));

        // Curl is polygenic: each pool contributes a draw, blended by share.
        let mut r = Rng::fork(seed, "curl");
        let curl: f32 = pools
            .iter()
            .map(|(p, w)| r.pick(&p.curl) as f32 * w)
            .sum::<f32>()
            + 0.25 * r.normal();
        let hair_type = match curl.round().clamp(0.0, 3.0) as u8 {
            0 => HairType::Straight,
            1 => HairType::Wavy,
            2 => HairType::Curly,
            _ => HairType::Coily,
        };

        let mut r = Rng::fork(seed, "hair_colour");
        let dark_sd = blend(&|p| p.hair_dark.1);
        let dark = q(clamp01(blend(&|p| p.hair_dark.0) + dark_sd * r.normal()));
        let red_pool = pick_pool(&mut r);
        let red = if r.f32() < red_pool.red_hair { q(r.range(0.5, 1.0)) } else { 0.0 };

        let mut r = Rng::fork(seed, "hairline");
        let hairline = Hairline {
            height: q((0.4 * r.normal()).clamp(-1.0, 1.0)),
            recession: q(clamp01(blend(&|p| p.recession) + 0.25 * r.normal())),
            onset: q(r.range(20.0, 38.0)),
        };

        let mut r = Rng::fork(seed, "greying");
        let greying = Greying {
            onset: q((blend(&|p| p.grey_onset) + 4.0 * r.normal()).max(26.0)),
            span: q((12.0 + 4.0 * r.normal()).max(4.0)),
        };

        let mut r = Rng::fork(seed, "eyes");
        let eye_pool = pick_pool(&mut r);
        let eye_colour = match r.pick(&eye_pool.eyes) {
            0 => EyeColour::Brown,
            1 => EyeColour::Hazel,
            2 => EyeColour::Green,
            _ => EyeColour::Blue,
        };
        let eye_shade = q(r.f32());

        let mut r = Rng::fork(seed, "beard");
        let beard = q(clamp01(blend(&|p| p.beard) + 0.2 * r.normal()));
        let mut r = Rng::fork(seed, "fat");
        let fat_tendency = q(clamp01(0.45 + 0.25 * r.normal()));

        let mut r = Rng::fork(seed, "shape");
        let shape = SHAPE_AXES
            .iter()
            .map(|_| q((0.45 * r.normal()).clamp(-1.0, 1.0)))
            .collect();

        Genome {
            v: GENOME_VERSION,
            seed,
            nation: nation.code.to_string(),
            birth_year,
            ancestry,
            mh_mix,
            skin_tone,
            undertone,
            hair_type,
            hair_colour: HairColour { dark, red },
            hairline,
            greying,
            eye_colour,
            eye_shade,
            beard,
            fat_tendency,
            shape,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("genome serialises")
    }

    pub fn shape_axis(&self, name: &str) -> f32 {
        SHAPE_AXES
            .iter()
            .position(|a| *a == name)
            .map(|i| self.shape[i])
            .unwrap_or(0.0)
    }
}

/// Picks an ancestry template for the nation and jitters its shares, so two
/// players from the same template still differ in their mix.
fn draw_ancestry(seed: u64, nation: &Nation) -> Vec<Share> {
    let mut r = Rng::fork(seed, "ancestry");
    let weights: Vec<f32> = nation.templates.iter().map(|t| t.0).collect();
    let template = nation.templates[r.pick(&weights)].1;
    let mut shares: Vec<Share> = template
        .iter()
        .map(|(pool, w)| Share { pool: *pool, share: w * (0.3 * r.normal()).exp() })
        .collect();
    // A small chance of a trace third lineage keeps mixes from being too tidy.
    if r.f32() < 0.15 {
        let extra = POOLS[r.pick(&[1.0; 4])].id;
        if !shares.iter().any(|s| s.pool == extra) {
            shares.push(Share { pool: extra, share: r.range(0.05, 0.2) });
        }
    }
    let total: f32 = shares.iter().map(|s| s.share).sum();
    for s in &mut shares {
        s.share = q(s.share / total);
    }
    shares.sort_by(|a, b| b.share.total_cmp(&a.share));
    shares
}
