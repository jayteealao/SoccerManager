//! PoC 8 / roadmap 8: the sourced genome (version 2).
//!
//! Replaces the invented pools of `pools.rs` with neutral ancestry components
//! whose numbers carry a provenance tag. Every number is either `Sourced`
//! (with its citation) or `Assumed` (a placeholder with the reason, to be
//! replaced when data exists). Component IDs are internal and neutral; the
//! reference labels explain where the numbers come from and must never be
//! shown in a game UI.
//!
//! Model:
//! - Ancestry: nation templates give expected shares; a player's shares are a
//!   Dirichlet draw around the template.
//! - Two-allele loci (HERC2 rs12913832, MC1R R alleles, SLC24A5 rs1426654,
//!   EDAR rs3827760): each of the two chromosomes takes its local ancestry
//!   from the player's shares, then its allele from that component's
//!   frequency. This is why an F1 child of a blue-eyed and a brown-eyed
//!   population is rarely blue-eyed.
//! - Continuous traits (skin CIELAB, hair darkness, beard, face identity):
//!   share-weighted component means plus locus effects plus within-group
//!   residuals (skin: the ISSA within-group covariance).
//! - Ordinal traits (curl, baldness): liability thresholds.
//! - Age curves: greying onset (Tobin and Paus 2001) and span fitted to
//!   Panhard 2012; Norwood III+ prevalence by decade.
//! - Inheritance: `child_of` averages ancestry, passes loci on Mendelian-style
//!   and inherits the face latent with parent-offspring correlation h2/2.

use crate::genome::{EyeColour, Genome, Greying, HairColour, HairType, Hairline, Share};
use crate::pools::PoolId;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

pub const GENOME2_VERSION: u8 = 2;
/// Length of the face latent that drives GNM Head's identity sampler.
pub const FACE_LATENT: usize = 64;
/// Length of the residual that restores within-group spread (src/gnm.rs).
pub const FACE_RESID: usize = 253;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Comp {
    K1,
    K2,
    K3,
    K4,
    K5,
    K6,
}

pub const COMPS: [Comp; 6] = [Comp::K1, Comp::K2, Comp::K3, Comp::K4, Comp::K5, Comp::K6];

#[derive(Clone, Copy, Debug)]
pub enum Src {
    Sourced(&'static str),
    Assumed(&'static str),
}

#[derive(Clone, Copy, Debug)]
pub struct V {
    pub x: f32,
    pub src: Src,
}

const fn s(x: f32, c: &'static str) -> V {
    V { x, src: Src::Sourced(c) }
}
const fn a(x: f32, c: &'static str) -> V {
    V { x, src: Src::Assumed(c) }
}

const G1K: &str = "1000 Genomes phase 3 via Ensembl (RESEARCH.md table)";
const TOBIN: &str = "Tobin and Paus 2001, greying onset";
const NORWOOD_EU: &str = "Norwood 1975 prevalence of type III+ by decade; approximate, UNVERIFIED digits";
const CN_BALD: &str = "Chinese men, Norwood III+ by decade (RESEARCH.md)";

pub struct CompData {
    pub id: Comp,
    /// Where the numbers come from. Internal only.
    pub reference: &'static str,
    /// GNM Head ethnicity condition: [middle_eastern, asian, white, black].
    pub gnm: ([f32; 4], Src),
    /// ISSA group code for male face skin (data/issa_face_skin.json).
    pub issa: &'static str,
    pub herc2_g: V,
    /// MC1R loss-of-function ("R") alleles combined.
    pub mc1r_r: V,
    pub slc24a5_a: V,
    pub edar_g: V,
    /// Greying onset (first grey), mean and SD in years.
    pub grey_onset: (V, f32),
    /// Norwood III+ prevalence for the 20s, 30s, 40s, 50s, 60s and 70s.
    pub bald: ([f32; 6], Src),
    /// Mean hair-curl liability (thresholds 0, 1, 2 split straight, wavy,
    /// curly, coily; residual N(0, 1)).
    pub curl: V,
    /// Mean hair darkness 0 (pale blond) .. 1 (black), before locus effects.
    pub hair_dark: V,
    pub beard: V,
}

const MC1R_NOTE: &str = "rs1805007 frequency x 2.2 for the other R alleles (R151C, D294H); multiplier UNVERIFIED";

pub const COMP_DATA: [CompData; 6] = [
    CompData {
        id: Comp::K1,
        reference: "West/Central African reference (1000G AFR; ISSA African; GNM black)",
        gnm: ([0.0, 0.0, 0.0, 1.0], Src::Sourced("GNM class")),
        issa: "AF",
        herc2_g: s(0.028, G1K),
        mc1r_r: a(0.003 * 2.2, MC1R_NOTE),
        slc24a5_a: s(0.074, G1K),
        edar_g: s(0.003, G1K),
        grey_onset: (s(43.9, TOBIN), 10.3),
        bald: ([0.02, 0.10, 0.18, 0.28, 0.34, 0.40], Src::Assumed("data gap: no African curve; East Asian curve lowered")),
        curl: a(2.85, "data gap (Loussouarn 2007 paywalled): about 80% coily"),
        hair_dark: a(0.96, "carried over from PoC 1"),
        beard: a(0.45, "data gap: beard density"),
    },
    CompData {
        id: Comp::K2,
        reference: "Northern European reference (1000G EUR, HERC2 Finland; ISSA Caucasian; GNM white)",
        gnm: ([0.0, 0.0, 1.0, 0.0], Src::Sourced("GNM class")),
        issa: "CA",
        herc2_g: s(0.91, "HERC2 Finland .91 (RESEARCH.md, clinal within Europe)"),
        mc1r_r: a(0.072 * 2.2, MC1R_NOTE),
        slc24a5_a: s(0.997, G1K),
        edar_g: s(0.011, G1K),
        grey_onset: (s(34.0, TOBIN), 9.6),
        bald: ([0.16, 0.25, 0.35, 0.45, 0.50, 0.55], Src::Sourced(NORWOOD_EU)),
        curl: a(-0.2, "data gap: about 58% straight"),
        hair_dark: a(0.45, "carried over from PoC 1"),
        beard: a(0.70, "data gap: beard density"),
    },
    CompData {
        id: Comp::K3,
        reference: "Southern European reference (1000G EUR, HERC2 Spain; ISSA Caucasian; GNM white)",
        gnm: ([0.15, 0.0, 0.85, 0.0], Src::Assumed("southern Europe between GNM white and middle eastern")),
        issa: "CA",
        herc2_g: s(0.32, "HERC2 Spain .32 (RESEARCH.md, clinal within Europe)"),
        mc1r_r: a(0.03 * 2.2, "southern-European rs1805007 lower than the EUR mean; UNVERIFIED"),
        slc24a5_a: s(0.997, G1K),
        edar_g: s(0.011, G1K),
        grey_onset: (s(34.0, TOBIN), 9.6),
        bald: ([0.16, 0.25, 0.35, 0.45, 0.50, 0.55], Src::Sourced(NORWOOD_EU)),
        curl: a(0.25, "data gap: about 40% straight"),
        hair_dark: a(0.82, "carried over from PoC 1"),
        beard: a(0.80, "data gap: beard density"),
    },
    CompData {
        id: Comp::K4,
        reference: "Middle East / North Africa reference (ISSA Iraqi; GNM middle eastern; no 1000G panel)",
        gnm: ([1.0, 0.0, 0.0, 0.0], Src::Sourced("GNM class")),
        issa: "IQ",
        herc2_g: a(0.10, "no 1000G panel for the region"),
        mc1r_r: a(0.01, "no 1000G panel for the region"),
        slc24a5_a: a(0.90, "no 1000G panel for the region"),
        edar_g: a(0.01, "no 1000G panel for the region"),
        grey_onset: (a(36.0, "between Tobin and Paus groups"), 10.0),
        bald: ([0.16, 0.25, 0.35, 0.45, 0.50, 0.55], Src::Assumed("European curve reused")),
        curl: a(0.55, "data gap"),
        hair_dark: a(0.9, "data gap"),
        beard: a(0.85, "data gap: beard density"),
    },
    CompData {
        id: Comp::K5,
        reference: "South Asian reference (1000G SAS; ISSA Pakistani; GNM mix)",
        gnm: ([0.6, 0.1, 0.3, 0.0], Src::Assumed("GNM has no South Asian class; mix of its classes")),
        issa: "SA",
        herc2_g: s(0.071, G1K),
        mc1r_r: a(0.005 * 2.2, MC1R_NOTE),
        slc24a5_a: s(0.685, G1K),
        edar_g: s(0.013, G1K),
        grey_onset: (a(36.0, "between Tobin and Paus groups"), 10.0),
        bald: ([0.10, 0.20, 0.30, 0.40, 0.45, 0.50], Src::Assumed("data gap")),
        curl: a(0.35, "data gap"),
        hair_dark: a(0.93, "data gap"),
        beard: a(0.75, "data gap: beard density"),
    },
    CompData {
        id: Comp::K6,
        reference: "East Asian reference (1000G EAS; ISSA Chinese; GNM asian)",
        gnm: ([0.0, 1.0, 0.0, 0.0], Src::Sourced("GNM class")),
        issa: "CN",
        herc2_g: s(0.002, G1K),
        mc1r_r: a(0.001 * 2.2, MC1R_NOTE),
        slc24a5_a: s(0.012, G1K),
        edar_g: s(0.873, G1K),
        grey_onset: (a(37.0, "Tobin and Paus: Asians in their late 30s"), 9.6),
        bald: ([0.028, 0.133, 0.214, 0.319, 0.362, 0.414], Src::Sourced(CN_BALD)),
        curl: a(-0.6, "EDAR G carries most of the straightness"),
        hair_dark: a(0.93, "carried over from PoC 1"),
        beard: a(0.30, "data gap: beard density"),
    },
];

pub fn comp(c: Comp) -> &'static CompData {
    &COMP_DATA[c as usize]
}

/// Fictional nations as ancestry templates: (weight, concentration, shares).
/// Concentration sets how tightly a player's shares cluster round the
/// template (Dirichlet); low values make varied mixes.
pub struct Nation2 {
    pub code: &'static str,
    pub templates: &'static [(f32, f32, &'static [(Comp, f32)])],
}

use Comp::*;

pub const NATIONS2: [Nation2; 3] = [
    Nation2 {
        code: "NHV",
        templates: &[
            (0.66, 40.0, &[(K2, 0.95), (K3, 0.05)]),
            (0.08, 6.0, &[(K2, 0.5), (K1, 0.5)]),
            (0.08, 40.0, &[(K1, 1.0)]),
            (0.07, 20.0, &[(K3, 0.7), (K4, 0.3)]),
            (0.06, 8.0, &[(K2, 0.5), (K3, 0.5)]),
            (0.03, 40.0, &[(K6, 1.0)]),
            (0.02, 6.0, &[(K2, 0.5), (K6, 0.5)]),
        ],
    },
    Nation2 {
        code: "CVD",
        templates: &[
            (0.35, 5.0, &[(K3, 0.55), (K1, 0.4), (K4, 0.05)]),
            (0.25, 25.0, &[(K3, 0.9), (K4, 0.1)]),
            (0.18, 8.0, &[(K1, 0.8), (K3, 0.2)]),
            (0.10, 40.0, &[(K1, 1.0)]),
            (0.06, 6.0, &[(K3, 0.5), (K2, 0.5)]),
            (0.06, 6.0, &[(K6, 0.5), (K3, 0.5)]),
        ],
    },
    Nation2 {
        code: "KSI",
        templates: &[
            (0.74, 40.0, &[(K6, 1.0)]),
            (0.10, 6.0, &[(K6, 0.5), (K2, 0.5)]),
            (0.09, 6.0, &[(K6, 0.5), (K1, 0.5)]),
            (0.07, 10.0, &[(K6, 0.7), (K5, 0.3)]),
        ],
    },
];

pub fn nation2(code: &str) -> Option<&'static Nation2> {
    NATIONS2.iter().find(|n| n.code == code)
}

/// ISSA male face skin: CIELAB mean and covariance per group.
pub struct Skin {
    pub mean: [f32; 3],
    pub cov: [[f32; 3]; 3],
}

pub fn issa_skin(code: &str) -> Skin {
    let j: serde_json::Value = serde_json::from_str(include_str!("../data/issa_face_skin.json")).unwrap();
    let m = &j["groups"][code]["M"];
    let v = |x: &serde_json::Value| x.as_f64().unwrap() as f32;
    let mean = [v(&m["lab_mean"][0]), v(&m["lab_mean"][1]), v(&m["lab_mean"][2])];
    let mut cov = [[0.0; 3]; 3];
    for i in 0..3 {
        for k in 0..3 {
            cov[i][k] = v(&m["lab_cov"][i][k]);
        }
    }
    Skin { mean, cov }
}

/// Genotype: number of effect alleles (0, 1, 2) at the four loci.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Loci {
    pub herc2_g: [bool; 2],
    pub mc1r_r: [bool; 2],
    pub slc24a5_a: [bool; 2],
    pub edar_g: [bool; 2],
}

fn n(x: [bool; 2]) -> f32 {
    x[0] as u8 as f32 + x[1] as u8 as f32
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Genome2 {
    pub v: u8,
    pub seed: u64,
    pub nation: String,
    pub birth_year: u16,
    pub ancestry: Vec<(Comp, f32)>,
    pub loci: Loci,
    /// Face skin CIELAB (D65), before age.
    pub skin_lab: [f32; 3],
    pub hair_dark: f32,
    pub red: f32,
    pub curl_liability: f32,
    pub eye_colour: EyeColour,
    pub eye_shade: f32,
    pub grey_onset: f32,
    pub grey_span: f32,
    /// Baldness liability as a percentile, 0..1 (higher = balder sooner).
    pub bald_u: f32,
    pub hairline_height: f32,
    pub beard: f32,
    pub fat_tendency: f32,
    /// GNM identity sampler latent (N(0, 1)) and residual (N(0, 1)).
    pub face_z: Vec<f32>,
    pub face_r: Vec<f32>,
}

fn q(x: f32) -> f32 {
    (x * 1000.0).round() / 1000.0
}

/// Marsaglia and Tsang gamma sampler (shape k > 0, scale 1).
fn gamma(r: &mut Rng, k: f32) -> f32 {
    if k < 1.0 {
        let u = r.f32().max(1e-7);
        return gamma(r, k + 1.0) * u.powf(1.0 / k);
    }
    let d = k - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        let x = r.normal();
        let v = (1.0 + c * x).powi(3);
        if v <= 0.0 {
            continue;
        }
        let u = r.f32().max(1e-7);
        if u.ln() < 0.5 * x * x + d - d * v + d * v.ln() {
            return d * v;
        }
    }
}

/// Standard normal CDF (Abramowitz and Stegun 7.1.26).
pub fn phi(x: f32) -> f32 {
    let t = 1.0 / (1.0 + 0.3275911 * x.abs() / std::f32::consts::SQRT_2);
    let y = 1.0 - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t * (-(x * x) / 2.0).exp();
    if x >= 0.0 { 0.5 + 0.5 * y } else { 0.5 - 0.5 * y }
}

impl Genome2 {
    pub fn generate(seed: u64, nation: &Nation2, birth_year: u16) -> Self {
        let mut r = Rng::fork(seed, "g2-ancestry");
        let w: Vec<f32> = nation.templates.iter().map(|t| t.0).collect();
        let (_, conc, tpl) = nation.templates[r.pick(&w)];
        let draws: Vec<f32> = tpl.iter().map(|(_, sh)| gamma(&mut r, conc * sh)).collect();
        let tot: f32 = draws.iter().sum::<f32>().max(1e-6);
        let mut ancestry: Vec<(Comp, f32)> = tpl.iter().zip(&draws).map(|((c, _), d)| (*c, q(d / tot))).filter(|x| x.1 > 0.005).collect();
        ancestry.sort_by(|a, b| b.1.total_cmp(&a.1));
        let loci = draw_loci(&mut Rng::fork(seed, "g2-loci"), &ancestry);
        let mut r = Rng::fork(seed, "g2-face");
        let face_z = (0..FACE_LATENT).map(|_| q(r.normal())).collect();
        let face_r = (0..FACE_RESID).map(|_| q(r.normal())).collect();
        Self::express(seed, nation.code, birth_year, ancestry, loci, face_z, face_r)
    }

    /// A son of `father`, with a mother drawn from `nation`.
    pub fn child_of(father: &Genome2, nation: &Nation2, seed: u64, birth_year: u16) -> Self {
        let mother = Self::generate(seed ^ 0x6d6f_7468_6572, nation, birth_year - 28);
        let mut shares: Vec<(Comp, f32)> = Vec::new();
        for (c, w) in father.ancestry.iter().chain(&mother.ancestry) {
            match shares.iter_mut().find(|x| x.0 == *c) {
                Some(x) => x.1 += w * 0.5,
                None => shares.push((*c, w * 0.5)),
            }
        }
        shares.iter_mut().for_each(|x| x.1 = q(x.1));
        shares.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut r = Rng::fork(seed, "g2-meiosis");
        let mut pass = |f: [bool; 2], m: [bool; 2]| [f[(r.f32() < 0.5) as usize], m[(r.f32() < 0.5) as usize]];
        let loci = Loci {
            herc2_g: pass(father.loci.herc2_g, mother.loci.herc2_g),
            mc1r_r: pass(father.loci.mc1r_r, mother.loci.mc1r_r),
            slc24a5_a: pass(father.loci.slc24a5_a, mother.loci.slc24a5_a),
            edar_g: pass(father.loci.edar_g, mother.loci.edar_g),
        };
        // Face: parent-offspring correlation h2/2 with unit variance kept.
        let h2 = 0.5;
        let mut r = Rng::fork(seed, "g2-face-child");
        let keep = (1.0 - h2 * h2 / 2.0f32).sqrt();
        let mix = |f: &[f32], m: &[f32], r: &mut Rng| -> Vec<f32> {
            f.iter().zip(m).map(|(a, b)| q(h2 * (a + b) / 2.0 + keep * r.normal())).collect()
        };
        let face_z = mix(&father.face_z, &mother.face_z, &mut r);
        let face_r = mix(&father.face_r, &mother.face_r, &mut r);
        Self::express(seed, &father.nation, birth_year, shares, loci, face_z, face_r)
    }

    /// Continuous and ordinal traits from ancestry and genotype.
    fn express(seed: u64, nation: &str, birth_year: u16, ancestry: Vec<(Comp, f32)>, loci: Loci, face_z: Vec<f32>, face_r: Vec<f32>) -> Self {
        let blend = |f: &dyn Fn(&CompData) -> f32| -> f32 { ancestry.iter().map(|(c, w)| f(comp(*c)) * w).sum() };
        // Expected allele counts, for centring the locus effects.
        let e_slc = 2.0 * blend(&|d| d.slc24a5_a.x);
        let e_herc = 2.0 * blend(&|d| d.herc2_g.x);
        let e_edar = 2.0 * blend(&|d| d.edar_g.x);

        // Skin: ISSA means blended by share, the SLC24A5 dose relative to its
        // expectation (+2.5 L* per A allele, assumed), and the ISSA
        // within-group covariance (Cholesky of the share-weighted covariance).
        let mut r = Rng::fork(seed, "g2-skin");
        let mut mean = [0.0f32; 3];
        let mut cov = [[0.0f32; 3]; 3];
        for (c, w) in &ancestry {
            let sk = issa_skin(comp(*c).issa);
            for i in 0..3 {
                mean[i] += w * sk.mean[i];
                for k in 0..3 {
                    cov[i][k] += w * sk.cov[i][k];
                }
            }
        }
        mean[0] += 2.5 * (n(loci.slc24a5_a) - e_slc);
        let l = cholesky3(cov);
        let z = [r.normal(), r.normal(), r.normal()];
        let mut skin_lab = [0.0; 3];
        for i in 0..3 {
            skin_lab[i] = q(mean[i] + (0..3).map(|k| l[i][k] * z[k]).sum::<f32>());
        }

        let mut r = Rng::fork(seed, "g2-hair");
        let hair_dark = q((blend(&|d| d.hair_dark.x) - 0.12 * (n(loci.herc2_g) - e_herc) + 0.12 * r.normal()).clamp(0.0, 1.0));
        let red = match n(loci.mc1r_r) as u8 {
            2 => q(r.range(0.6, 1.0)),
            1 if r.f32() < 0.12 => q(r.range(0.2, 0.45)),
            _ => 0.0,
        };
        let curl_liability = q(blend(&|d| d.curl.x) - 0.6 * (n(loci.edar_g) - e_edar) + 0.8 * r.normal());

        // Eye colour from the HERC2 genotype (Sturm et al. 2008, approximate
        // class shares; UNVERIFIED digits).
        let mut r = Rng::fork(seed, "g2-eyes");
        let probs = match n(loci.herc2_g) as u8 {
            2 => [0.03, 0.07, 0.2, 0.7],
            1 => [0.6, 0.25, 0.1, 0.05],
            _ => [0.93, 0.06, 0.01, 0.0],
        };
        let eye_colour = [EyeColour::Brown, EyeColour::Hazel, EyeColour::Green, EyeColour::Blue][r.pick(&probs)];
        let eye_shade = q(r.f32());

        let mut r = Rng::fork(seed, "g2-age");
        let grey_mean = blend(&|d| d.grey_onset.0.x);
        let grey_sd = blend(&|d| d.grey_onset.1);
        let grey_onset = q((grey_mean + grey_sd * r.normal()).max(18.0));
        // Span from first grey to half grey, fitted so that 6-23% of people
        // are at least half grey at 50 (Panhard 2012).
        let grey_span = q((25.0 + 6.0 * r.normal()).max(8.0));
        let bald_u = q(r.f32());
        let hairline_height = q((0.4 * r.normal()).clamp(-1.0, 1.0));
        let beard = q((blend(&|d| d.beard.x) + 0.2 * r.normal()).clamp(0.0, 1.0));
        let fat_tendency = q((0.45 + 0.25 * r.normal()).clamp(0.0, 1.0));

        Self {
            v: GENOME2_VERSION,
            seed,
            nation: nation.to_string(),
            birth_year,
            ancestry,
            loci,
            skin_lab,
            hair_dark,
            red,
            curl_liability,
            eye_colour,
            eye_shade,
            grey_onset,
            grey_span,
            bald_u,
            hairline_height,
            beard,
            fat_tendency,
            face_z,
            face_r,
        }
    }

    pub fn hair_type(&self) -> HairType {
        match self.curl_liability {
            x if x < 0.0 => HairType::Straight,
            x if x < 1.0 => HairType::Wavy,
            x if x < 2.0 => HairType::Curly,
            _ => HairType::Coily,
        }
    }

    /// GNM Head condition: [female, male, middle_eastern, asian, white, black].
    pub fn gnm_condition(&self) -> [f32; 6] {
        let mut c = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        for (k, w) in &self.ancestry {
            for e in 0..4 {
                c[2 + e] += w * comp(*k).gnm.0[e];
            }
        }
        c
    }

    /// Share-weighted Norwood III+ prevalence at an age.
    pub fn bald_prevalence(&self, years: f32) -> f32 {
        self.ancestry.iter().map(|(c, w)| w * bald_curve(&comp(*c).bald.0, years)).sum()
    }

    /// Fraction grey at an age: 0 before onset, 0.5 at onset + span.
    pub fn grey_at(&self, years: f32) -> f32 {
        let t = ((years - self.grey_onset) / (2.0 * self.grey_span)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Hairline recession at an age, 0..1: the player is Norwood III+ once
    /// their liability percentile passes the population's non-bald share.
    pub fn recession_at(&self, years: f32) -> f32 {
        let p = self.bald_prevalence(years);
        let over = self.bald_u - (1.0 - p);
        crate::age::smoothstep(-0.15, 0.15, over)
    }

    /// A version-1 genome view so the existing hair cards, colours and age
    /// rules can run on a version-2 genome.
    pub fn legacy(&self) -> Genome {
        let gc = self.gnm_condition();
        Genome {
            v: 1,
            seed: self.seed,
            nation: self.nation.clone(),
            birth_year: self.birth_year,
            ancestry: self.ancestry.iter().map(|(c, w)| Share { pool: legacy_pool(*c), share: *w }).collect(),
            mh_mix: [gc[5], gc[3], gc[2] + gc[4]],
            skin_tone: ((66.0 - self.skin_lab[0]) / 36.0).clamp(0.0, 1.0),
            undertone: ((self.skin_lab[2] - 15.5) / 4.0).clamp(-1.0, 1.0),
            hair_type: self.hair_type(),
            hair_colour: HairColour { dark: self.hair_dark, red: self.red },
            hairline: Hairline { height: self.hairline_height, recession: 1.0, onset: 20.0 },
            greying: Greying { onset: self.grey_onset, span: 2.0 * self.grey_span },
            eye_colour: self.eye_colour,
            eye_shade: self.eye_shade,
            beard: self.beard,
            fat_tendency: self.fat_tendency,
            shape: vec![0.0; crate::genome::SHAPE_AXES.len()],
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

fn legacy_pool(c: Comp) -> PoolId {
    match c {
        K1 => PoolId::A,
        K2 => PoolId::B,
        K3 | K4 | K5 => PoolId::C,
        K6 => PoolId::D,
    }
}

fn bald_curve(c: &[f32; 6], years: f32) -> f32 {
    // Decade midpoints 25, 35, ..., 75; zero at 17.
    if years <= 25.0 {
        return c[0] * ((years - 17.0) / 8.0).clamp(0.0, 1.0);
    }
    let t = ((years - 25.0) / 10.0).min(5.0);
    let i = (t.floor() as usize).min(4);
    c[i] + (c[i + 1] - c[i]) * (t - i as f32)
}

fn draw_loci(r: &mut Rng, ancestry: &[(Comp, f32)]) -> Loci {
    let w: Vec<f32> = ancestry.iter().map(|x| x.1).collect();
    let mut locus = |f: &dyn Fn(&CompData) -> f32| -> [bool; 2] {
        let mut out = [false; 2];
        for o in &mut out {
            let c = comp(ancestry[r.pick(&w)].0);
            *o = r.f32() < f(c);
        }
        out
    };
    Loci {
        herc2_g: locus(&|d| d.herc2_g.x),
        mc1r_r: locus(&|d| d.mc1r_r.x),
        slc24a5_a: locus(&|d| d.slc24a5_a.x),
        edar_g: locus(&|d| d.edar_g.x),
    }
}

fn cholesky3(a: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut l = [[0.0f32; 3]; 3];
    for i in 0..3 {
        for k in 0..=i {
            let s: f32 = (0..k).map(|j| l[i][j] * l[k][j]).sum();
            l[i][k] = if i == k { (a[i][i] - s).max(1e-6).sqrt() } else { (a[i][k] - s) / l[k][k] };
        }
    }
    l
}

// ------------------------------------------------------------- skin model ---

/// Two-chromophore skin albedo (roadmap 5): linear RGB reflectance from
/// melanin (eumelanin with a pheomelanin share that falls as melanin rises)
/// and haemoglobin concentrations. Parameters were fitted so the model
/// reaches all ten Monk Skin Tone swatches (Google, CC BY 4.0) within
/// dE76 4.3 (tests below; the fit is in REPORT.md).
pub mod chromophores {
    use glam::Vec3;

    const BASE: [f32; 3] = [0.936, 1.0, 0.822];
    const EU: [f32; 3] = [1.0, 0.756, 1.0];
    const PHEO: [f32; 3] = [0.0, 0.656, 1.0];
    const HB: [f32; 3] = [0.482, 1.0, 0.0];
    const PHEO_MAX: f32 = 1.0;
    const PHEO_FALL: f32 = 3.456;

    pub fn albedo(cm: f32, ch: f32) -> Vec3 {
        let f = PHEO_MAX * (-cm / PHEO_FALL).exp();
        let c = |i: usize| BASE[i] * (-(cm * ((1.0 - f) * EU[i] + f * PHEO[i]) + ch * HB[i])).exp();
        Vec3::new(c(0), c(1), c(2))
    }

    pub fn lab(rgb: Vec3) -> [f32; 3] {
        let x = 0.4124 * rgb.x + 0.3576 * rgb.y + 0.1805 * rgb.z;
        let y = 0.2126 * rgb.x + 0.7152 * rgb.y + 0.0722 * rgb.z;
        let z = 0.0193 * rgb.x + 0.1192 * rgb.y + 0.9505 * rgb.z;
        let f = |t: f32| if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
        let (fx, fy, fz) = (f(x / 0.9505), f(y), f(z / 1.089));
        [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
    }

    pub fn de(a: [f32; 3], b: [f32; 3]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    /// (melanin, haemoglobin) whose albedo is closest to a CIELAB target,
    /// with haemoglobin held near a physiological 0.35 unless the target
    /// needs otherwise.
    pub fn invert(target: [f32; 3]) -> (f32, f32) {
        let cost = |cm: f32, ch: f32| de(lab(albedo(cm, ch)), target) + 0.5 * (ch - 0.35).abs();
        let (mut bm, mut bh, mut bc) = (0.0, 0.0, f32::MAX);
        for i in 0..=90 {
            for j in 0..=20 {
                let (cm, ch) = (i as f32 * 0.06, j as f32 * 0.05);
                let c = cost(cm, ch);
                if c < bc {
                    (bm, bh, bc) = (cm, ch, c);
                }
            }
        }
        let mut step = 0.03;
        for _ in 0..30 {
            for (dm, dh) in [(step, 0.0), (-step, 0.0), (0.0, step), (0.0, -step)] {
                let (cm, ch) = ((bm + dm).max(0.0), (bh + dh).clamp(0.0, 1.0));
                let c = cost(cm, ch);
                if c < bc {
                    (bm, bh, bc) = (cm, ch, c);
                }
            }
            step *= 0.8;
        }
        (bm, bh)
    }

    pub fn srgb_hex(h: &str) -> Vec3 {
        let v = |i: usize| {
            let c = u8::from_str_radix(&h[i..i + 2], 16).unwrap() as f32 / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        Vec3::new(v(0), v(2), v(4))
    }

    /// Monk Skin Tone scale swatches 1-10 (Google, CC BY 4.0).
    pub const MST: [&str; 10] = ["f6ede4", "f3e7db", "f7ead0", "eadaba", "d7bd96", "a07e56", "825c43", "604134", "3a312a", "292420"];

    pub fn ita(lab: [f32; 3]) -> f32 {
        ((lab[0] - 50.0) / lab[2]).atan().to_degrees()
    }

    /// Nearest MST swatch (1-10) by dE76.
    pub fn mst_bucket(l: [f32; 3]) -> usize {
        (0..10).min_by(|&a, &b| de(lab(srgb_hex(MST[a])), l).total_cmp(&de(lab(srgb_hex(MST[b])), l))).unwrap() + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pure(c: Comp, seed: u64) -> Genome2 {
        let nat = Nation2 { code: "T", templates: &[] };
        let _ = nat;
        let loci = draw_loci(&mut Rng::fork(seed, "g2-loci"), &[(c, 1.0)]);
        Genome2::express(seed, "T", 2000, vec![(c, 1.0)], loci, vec![0.0; FACE_LATENT], vec![0.0; FACE_RESID])
    }

    #[test]
    fn allele_frequencies_reproduce_sources() {
        let n = 20000;
        for c in COMPS {
            let d = comp(c);
            let mut herc = 0.0;
            let mut edar = 0.0;
            for s in 0..n {
                let g = pure(c, s);
                herc += n_(g.loci.herc2_g);
                edar += n_(g.loci.edar_g);
            }
            let (fh, fe) = (herc / (2.0 * n as f32), edar / (2.0 * n as f32));
            assert!((fh - d.herc2_g.x).abs() < 0.01, "{c:?} HERC2 {fh} vs {}", d.herc2_g.x);
            assert!((fe - d.edar_g.x).abs() < 0.01, "{c:?} EDAR {fe} vs {}", d.edar_g.x);
        }
    }

    fn n_(x: [bool; 2]) -> f32 {
        n(x)
    }

    #[test]
    fn skin_reproduces_issa() {
        for c in COMPS {
            let sk = issa_skin(comp(c).issa);
            let n = 6000;
            let ls: Vec<f32> = (0..n).map(|s| pure(c, s).skin_lab[0]).collect();
            let m = ls.iter().sum::<f32>() / n as f32;
            let sd = (ls.iter().map(|x| (x - m).powi(2)).sum::<f32>() / n as f32).sqrt();
            assert!((m - sk.mean[0]).abs() < 0.6, "{c:?} L* mean {m} vs {}", sk.mean[0]);
            // SLC24A5 adds a little spread in mixed components; allow 25%.
            assert!((sd / sk.cov[0][0].sqrt() - 1.0).abs() < 0.25, "{c:?} L* sd {sd} vs {}", sk.cov[0][0].sqrt());
        }
    }

    #[test]
    fn mixed_ancestry_rarely_blue_eyed() {
        // A 50/50 K1/K2 mix: the locus model gives P(GG) ~ ((0.028+0.91)/2)^2
        // = 0.22 and blue ~ 0.16; picking one pool's eye-colour table (PoC 1)
        // gave 0.25. Pure K2 stays mostly light-eyed.
        let n = 20000;
        let mut blue_mix = 0;
        let mut blue_k2 = 0;
        for s in 0..n {
            let loci = draw_loci(&mut Rng::fork(s, "g2-loci"), &[(K1, 0.5), (K2, 0.5)]);
            let g = Genome2::express(s, "T", 2000, vec![(K1, 0.5), (K2, 0.5)], loci, vec![], vec![]);
            blue_mix += (g.eye_colour == EyeColour::Blue) as u32;
            blue_k2 += (pure(K2, s).eye_colour == EyeColour::Blue) as u32;
        }
        let (bm, bk) = (blue_mix as f32 / n as f32, blue_k2 as f32 / n as f32);
        assert!(bm > 0.12 && bm < 0.2, "mix blue {bm}");
        assert!(bk > 0.5 && bk < 0.65, "K2 blue {bk}");
    }

    #[test]
    fn f1_children_are_rarely_blue_eyed() {
        // Son of a K1 father and a K2 mother: one HERC2 allele comes from the
        // K1 side, so blue needs a rare K1 G allele.
        let nat = Nation2 { code: "T", templates: &[(1.0, 1000.0, &[(K2, 1.0)])] };
        let mut blue = 0;
        let n = 5000;
        for s in 0..n {
            let father = Genome2::generate(s, &Nation2 { code: "T", templates: &[(1.0, 1000.0, &[(K1, 1.0)])] }, 1980);
            let son = Genome2::child_of(&father, &nat, s + 1_000_000, 2008);
            blue += (son.eye_colour == EyeColour::Blue) as u32;
        }
        assert!((blue as f32 / n as f32) < 0.06);
    }

    #[test]
    fn greying_matches_panhard() {
        // 6-23% of people are at least half grey at 50 (Panhard 2012).
        for c in [K1, K2, K6] {
            let n = 8000;
            let half = (0..n).filter(|s| pure(c, *s).grey_at(50.0) >= 0.5).count() as f32 / n as f32;
            assert!((0.03..=0.25).contains(&half), "{c:?} half grey at 50: {half}");
        }
    }

    #[test]
    fn baldness_matches_curve() {
        let n = 8000;
        for (years, want) in [(35.0, 0.133), (55.0, 0.319)] {
            let got = (0..n).filter(|s| pure(K6, *s).recession_at(years) > 0.5).count() as f32 / n as f32;
            assert!((got - want).abs() < 0.05, "K6 at {years}: {got} vs {want}");
        }
    }

    #[test]
    fn chromophores_cover_monk_scale() {
        use chromophores::*;
        let mut last = -1.0;
        for (i, h) in MST.iter().enumerate() {
            let t = lab(srgb_hex(h));
            let (cm, ch) = invert(t);
            let e = de(lab(albedo(cm, ch)), t);
            assert!(e < 5.0, "MST {} dE {e}", i + 1);
            assert!(cm >= last - 0.05, "melanin not monotonic at MST {}", i + 1);
            last = cm;
            assert_eq!(mst_bucket(lab(albedo(cm, ch))), i + 1);
        }
    }

    #[test]
    fn ancestry_explains_skin_more_than_face() {
        // Relethford 2002 / Ruiz-Linares 2014: ancestry explains most of the
        // skin colour variance between groups but little of face shape. Here
        // the face latent is independent of ancestry by construction; the
        // ancestry-driven part of face shape comes from GNM's condition
        // (tested in gnm.rs). Skin: R^2 of L* on ancestry across a mixed
        // nation must be high.
        let nat = nation2("CVD").unwrap();
        let gs: Vec<Genome2> = (0..4000).map(|s| Genome2::generate(s, nat, 2000)).collect();
        let x: Vec<f32> = gs.iter().map(|g| g.ancestry.iter().filter(|a| a.0 == K1).map(|a| a.1).sum()).collect();
        let y: Vec<f32> = gs.iter().map(|g| g.skin_lab[0]).collect();
        let r2 = corr(&x, &y).powi(2);
        assert!(r2 > 0.6, "skin R2 on K1 share {r2}");
    }

    fn corr(x: &[f32], y: &[f32]) -> f32 {
        let n = x.len() as f32;
        let (mx, my) = (x.iter().sum::<f32>() / n, y.iter().sum::<f32>() / n);
        let c: f32 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
        let vx: f32 = x.iter().map(|a| (a - mx).powi(2)).sum();
        let vy: f32 = y.iter().map(|b| (b - my).powi(2)).sum();
        c / (vx * vy).sqrt()
    }

    #[test]
    fn deterministic_and_round_trips() {
        let g = Genome2::generate(42, nation2("KSI").unwrap(), 2004);
        assert_eq!(g, Genome2::generate(42, nation2("KSI").unwrap(), 2004));
        let back: Genome2 = serde_json::from_str(&g.to_json()).unwrap();
        assert_eq!(g, back);
    }
}
