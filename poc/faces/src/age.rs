//! Age rules from 15 to 70. They change only age: bones and identity come
//! from the genome and stay fixed.
//!
//! The rules, as the brief states them:
//! - growth finishes at about 21 (youth -> adult macro targets);
//! - fat moves lower from the mid-30s (jowls, neck, eye bags; cheeks thin);
//! - greying runs from the mid-30s to mid-40s, onset set by ancestry;
//! - plus a genome-driven receding hairline and skin ageing for the shader.

use crate::genome::Genome;

/// Career-stage ages used for the contact sheets.
pub const SHEET_AGES: [f32; 8] = [16.0, 19.0, 24.0, 30.0, 36.0, 45.0, 60.0, 70.0];

pub struct AgeState {
    pub years: f32,
    /// Macro age weights for MakeHuman child (11 y), young (25 y), old (90 y).
    pub mh_child: f32,
    pub mh_young: f32,
    pub mh_old: f32,
    pub muscle: f32,
    pub weight: f32,
    /// Extra targets and weights (paired names use `{s}`).
    pub extra: Vec<(&'static str, f32)>,
    /// Fraction of grey hair, 0..1.
    pub grey: f32,
    /// How far the hairline has receded, 0..1.
    pub recession: f32,
    /// Skin ageing for the shader (creases, texture, tone), 0..1.
    pub skin_age: f32,
    /// Beard growth potential (a 16-year-old has little), 0..1.
    pub beard_growth: f32,
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl AgeState {
    pub fn new(g: &Genome, years: f32) -> Self {
        // Growth: MakeHuman's child target is an 11-year-old, which is far too
        // young for 15; blend at most 45% of it and finish growth by 21.
        let mh_child = if years < 21.0 { 0.45 * ((21.0 - years) / 6.0).min(1.0) } else { 0.0 };
        // Ageing: MakeHuman's old target is a 90-year-old.
        let mh_old = ((years - 25.0) / 65.0).clamp(0.0, 1.0);
        let mh_young = 1.0 - mh_child - mh_old;

        // Athletes carry more muscle through the career, less afterwards.
        let muscle = 0.62 - 0.12 * smoothstep(33.0, 55.0, years);
        let fat = g.fat_tendency * smoothstep(33.0, 58.0, years);
        let weight = 0.5 + 0.3 * fat - 0.05 * (1.0 - smoothstep(15.0, 21.0, years));

        // Fat moves lower: fuller neck and lower face, eye bags, thinner upper
        // cheeks, deeper laugh lines.
        let late = smoothstep(35.0, 70.0, years);
        let extra = vec![
            ("neck/neck-double-incr", 0.9 * fat),
            ("head/head-fat-incr", 0.35 * fat),
            ("cheek/{s}-cheek-volume-decr", 0.5 * late),
            ("eyes/{s}-eye-bag-incr", 0.8 * late),
            ("mouth/mouth-laugh-lines-in", 0.6 * late),
            ("head/head-age-incr", 0.6 * smoothstep(40.0, 70.0, years)),
            ("head/head-age-decr", 0.5 * (1.0 - smoothstep(15.0, 21.0, years))),
        ];

        let grey = smoothstep(g.greying.onset, g.greying.onset + g.greying.span, years);
        let recession =
            g.hairline.recession * smoothstep(g.hairline.onset, g.hairline.onset + 30.0, years);
        let skin_age = smoothstep(28.0, 72.0, years);
        let beard_growth = smoothstep(15.0, 23.0, years);

        Self {
            years,
            mh_child,
            mh_young,
            mh_old,
            muscle,
            weight,
            extra,
            grey,
            recession,
            skin_age,
            beard_growth,
        }
    }
}
