//! Sample regional gene pools and sample nations.
//!
//! EVERY NUMBER IN THIS FILE IS INVENTED FOR THE PROOF OF CONCEPT. None of it
//! is measured population data. The pools are labelled with broad sample
//! regions only so a reader can see that mixing works; a real version needs
//! numbers chosen with care and reviewed. The nations are fictional.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PoolId {
    A,
    B,
    C,
    D,
}

pub struct Pool {
    pub id: PoolId,
    /// Human-readable label. The region is a sample label, not a claim.
    pub label: &'static str,
    /// Mean MakeHuman macro mix: [african, asian, caucasian] target weights.
    pub mh_mix: [f32; 3],
    /// Skin tone, 0 = lightest, 1 = darkest. Mean and spread.
    pub skin: (f32, f32),
    /// Hair curl class probabilities: straight, wavy, curly, coily.
    pub curl: [f32; 4],
    /// Eumelanin in hair, 0 = pale blond, 1 = black. Mean and spread.
    pub hair_dark: (f32, f32),
    /// Chance that the red-hair gene is expressed.
    pub red_hair: f32,
    /// Eye colour class probabilities: brown, hazel, green, blue.
    pub eyes: [f32; 4],
    /// Age at which greying starts, mean, in years.
    pub grey_onset: f32,
    /// Propensity for a receding hairline, 0..1.
    pub recession: f32,
    /// Beard density, 0..1.
    pub beard: f32,
}

pub const POOLS: [Pool; 4] = [
    Pool {
        id: PoolId::A,
        label: "Pool A (INVENTED sample, West-Africa-like)",
        mh_mix: [0.85, 0.02, 0.13],
        skin: (0.82, 0.08),
        curl: [0.01, 0.03, 0.16, 0.80],
        hair_dark: (0.96, 0.03),
        red_hair: 0.0,
        eyes: [0.97, 0.03, 0.0, 0.0],
        grey_onset: 43.0,
        recession: 0.35,
        beard: 0.45,
    },
    Pool {
        id: PoolId::B,
        label: "Pool B (INVENTED sample, Northern-Europe-like)",
        mh_mix: [0.02, 0.03, 0.95],
        skin: (0.10, 0.05),
        curl: [0.55, 0.35, 0.10, 0.0],
        hair_dark: (0.45, 0.20),
        red_hair: 0.06,
        eyes: [0.20, 0.15, 0.15, 0.50],
        grey_onset: 35.0,
        recession: 0.55,
        beard: 0.70,
    },
    Pool {
        id: PoolId::C,
        label: "Pool C (INVENTED sample, Mediterranean-like)",
        mh_mix: [0.10, 0.05, 0.85],
        skin: (0.34, 0.08),
        curl: [0.35, 0.40, 0.23, 0.02],
        hair_dark: (0.82, 0.10),
        red_hair: 0.01,
        eyes: [0.70, 0.18, 0.07, 0.05],
        grey_onset: 37.0,
        recession: 0.45,
        beard: 0.80,
    },
    Pool {
        id: PoolId::D,
        label: "Pool D (INVENTED sample, East-Asia-like)",
        mh_mix: [0.02, 0.90, 0.08],
        skin: (0.28, 0.07),
        curl: [0.85, 0.12, 0.03, 0.0],
        hair_dark: (0.93, 0.04),
        red_hair: 0.0,
        eyes: [0.97, 0.03, 0.0, 0.0],
        grey_onset: 40.0,
        recession: 0.30,
        beard: 0.30,
    },
];

pub fn pool(id: PoolId) -> &'static Pool {
    POOLS.iter().find(|p| p.id == id).expect("every PoolId has a pool")
}

/// A fictional nation: a weighted list of ancestry templates.
pub struct Nation {
    pub code: &'static str,
    pub name: &'static str,
    pub templates: &'static [(f32, &'static [(PoolId, f32)])],
}

use PoolId::*;

pub const NATIONS: [Nation; 3] = [
    Nation {
        code: "NHV",
        name: "Nordhavn (fictional)",
        templates: &[
            (0.66, &[(B, 1.0)]),
            (0.08, &[(B, 0.5), (A, 0.5)]),
            (0.08, &[(A, 1.0)]),
            (0.07, &[(C, 1.0)]),
            (0.06, &[(B, 0.5), (C, 0.5)]),
            (0.03, &[(D, 1.0)]),
            (0.02, &[(B, 0.5), (D, 0.5)]),
        ],
    },
    Nation {
        code: "CVD",
        name: "Costa Verde (fictional)",
        templates: &[
            (0.35, &[(C, 0.6), (A, 0.4)]),
            (0.25, &[(C, 1.0)]),
            (0.18, &[(A, 0.8), (C, 0.2)]),
            (0.10, &[(A, 1.0)]),
            (0.06, &[(C, 0.5), (B, 0.5)]),
            (0.06, &[(D, 0.5), (C, 0.5)]),
        ],
    },
    Nation {
        code: "KSI",
        name: "Kaisei Isles (fictional)",
        templates: &[
            (0.74, &[(D, 1.0)]),
            (0.10, &[(D, 0.5), (B, 0.5)]),
            (0.09, &[(D, 0.5), (A, 0.5)]),
            (0.07, &[(D, 0.7), (C, 0.3)]),
        ],
    },
];

pub fn nation(code: &str) -> Option<&'static Nation> {
    NATIONS.iter().find(|n| n.code == code)
}
