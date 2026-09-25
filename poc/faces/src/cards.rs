//! Hair geometry generated in code.
//!
//! - `ShortStraight`, `MediumWavy`: classic hair cards (ribbons with an alpha
//!   strand texture) walked over the scalp along a growth field.
//! - `ShortCurly`, `TightFade`: stacked shells whose shader draws the ring of
//!   each hair's helix at the shell height, so the hair reads as coils.
//! - `Twists`: a low shell layer for the scalp plus twisted tubes.

use crate::face::Colours;
use crate::genome::{Genome, HairType};
use crate::render::{kind, Draw, DrawParams, Vertex};
use crate::rng::Rng;
use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    ShortStraight,
    MediumWavy,
    ShortCurly,
    TightFade,
    Twists,
}

impl Style {
    pub fn for_genome(g: &Genome) -> Style {
        match g.hair_type {
            HairType::Straight => Style::ShortStraight,
            HairType::Wavy => Style::MediumWavy,
            HairType::Curly => Style::ShortCurly,
            HairType::Coily => {
                if g.seed % 3 == 0 { Style::Twists } else { Style::TightFade }
            }
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Style::ShortStraight => "short straight",
            Style::MediumWavy => "medium wavy",
            Style::ShortCurly => "short curly",
            Style::TightFade => "tight fade",
            Style::Twists => "short twists",
        }
    }
}

/// The skin of the head as seen by the hair: compact arrays over skin
/// vertices plus each vertex's hair density and base-mesh position.
pub struct Scalp {
    pub pos: Vec<Vec3>,
    pub nrm: Vec<Vec3>,
    pub base: Vec<Vec3>,
    pub dens: Vec<f32>,
    pub tris: Vec<[u32; 3]>,
    near: Vec<u32>,
}

impl Scalp {
    pub fn new(pos: Vec<Vec3>, nrm: Vec<Vec3>, base: Vec<Vec3>, dens: Vec<f32>, tris: Vec<[u32; 3]>) -> Self {
        // Vertices that can matter for collision: the head above the jaw.
        let near = (0..pos.len() as u32).filter(|&i| base[i as usize].y > 6.6).collect();
        Self { pos, nrm, base, dens, tris, near }
    }

    fn nearest(&self, p: Vec3) -> usize {
        let mut best = (f32::MAX, 0usize);
        for &i in &self.near {
            let d = self.pos[i as usize].distance_squared(p);
            if d < best.0 {
                best = (d, i as usize);
            }
        }
        best.1
    }

    /// Height of `p` over the surface near it, and that surface's normal.
    fn height(&self, p: Vec3) -> (f32, Vec3) {
        let i = self.nearest(p);
        let n = self.nrm[i];
        ((p - self.pos[i]).dot(n), n)
    }

    /// Area-weighted random points on haired triangles, thinned by density.
    fn sample_roots(&self, rng: &mut Rng, count: usize, min_dist: f32) -> Vec<(Vec3, Vec3, Vec3)> {
        let tris: Vec<(usize, f32)> = self
            .tris
            .iter()
            .enumerate()
            .filter(|(_, t)| t.iter().any(|&i| self.dens[i as usize] > 0.05))
            .map(|(k, t)| {
                let [a, b, c] = t.map(|i| self.pos[i as usize]);
                (k, (b - a).cross(c - a).length() * 0.5)
            })
            .collect();
        let weights: Vec<f32> = tris.iter().map(|t| t.1).collect();
        let mut out: Vec<(Vec3, Vec3, Vec3)> = Vec::new();
        let mut tries = 0;
        while out.len() < count && tries < count * 30 {
            tries += 1;
            let t = self.tris[tris[rng.pick(&weights)].0];
            let (mut u, mut v) = (rng.f32(), rng.f32());
            if u + v > 1.0 {
                u = 1.0 - u;
                v = 1.0 - v;
            }
            let w = 1.0 - u - v;
            let [a, b, c] = t.map(|i| i as usize);
            let dens = self.dens[a] * w + self.dens[b] * u + self.dens[c] * v;
            if rng.f32() > dens {
                continue;
            }
            let p = self.pos[a] * w + self.pos[b] * u + self.pos[c] * v;
            if min_dist > 0.0 && out.iter().any(|o| o.0.distance(p) < min_dist) {
                continue;
            }
            let n = (self.nrm[a] * w + self.nrm[b] * u + self.nrm[c] * v).normalize();
            let base = self.base[a] * w + self.base[b] * u + self.base[c] * v;
            out.push((p, n, base));
        }
        out
    }
}

/// Growth direction: away from the crown whorl, in the tangent plane.
fn flow(base: Vec3, n: Vec3, gravity: f32) -> Vec3 {
    let crown = Vec3::new(0.06, 8.38, 0.1);
    let d = (base - crown).normalize_or_zero() + Vec3::new(0.0, -gravity, 0.0);
    (d - n * d.dot(n)).normalize_or_zero()
}

fn hair_params(kind: u32, c: &Colours) -> DrawParams {
    DrawParams {
        kind,
        colour: c.hair_root.extend(1.0).to_array(),
        colour2: c.hair_tip.extend(1.0).to_array(),
        p0: [0.35, 1.0, 0.0, c.grey],
        ..Default::default()
    }
}

/// Builds the hair draws and the scalp tint the skin should use under them.
pub fn build(style: Style, g: &Genome, scalp: &Scalp, c: &Colours) -> (Vec<Draw>, f32) {
    let mut rng = Rng::fork(g.seed, "hair-geometry");
    match style {
        Style::ShortStraight => (vec![cards(scalp, &mut rng, c, 4200, 0.22, 0.36, 0.11, 0.0, 0.25)], 0.55),
        Style::MediumWavy => (vec![cards(scalp, &mut rng, c, 3200, 0.5, 0.8, 0.2, 1.0, 0.9)], 0.5),
        Style::ShortCurly => (vec![shells(scalp, c, 24, |_| 0.2, 0.05, 0.016, 0.02, 1.0)], 0.75),
        Style::TightFade => (vec![shells(scalp, c, 22, fade_profile, 0.036, 0.011, 0.015, 1.3)], 0.8),
        Style::Twists => {
            let base = shells(scalp, c, 6, |_| 0.035, 0.03, 0.009, 0.013, 1.2);
            (vec![base, tubes(scalp, &mut rng, c)], 0.7)
        }
    }
}

/// A fade: longer on top, tapering to skin on the sides and back.
fn fade_profile(b: Vec3) -> f32 {
    let side = crate::age::smoothstep(7.45, 8.05, b.y);
    0.005 + 0.12 * side
}

#[allow(clippy::too_many_arguments)]
fn cards(
    scalp: &Scalp,
    rng: &mut Rng,
    c: &Colours,
    count: usize,
    len_min: f32,
    len_max: f32,
    width: f32,
    wave: f32,
    gravity: f32,
) -> Draw {
    let roots = scalp.sample_roots(rng, count, 0.0);
    let segs = 8;
    let mut vs = Vec::new();
    let mut is = Vec::new();
    for (root, n0, base) in roots {
        let len = rng.range(len_min, len_max);
        let lift = rng.range(0.004, 0.03);
        let phase = rng.range(0.0, std::f32::consts::TAU);
        let wl = rng.range(0.22, 0.3);
        let mut dir = flow(base, n0, 0.0);
        let mut p = root + n0 * 0.003;
        let mut n = n0;
        let ds = len / segs as f32;
        let first = vs.len() as u32;
        for k in 0..=segs {
            let s = k as f32 / segs as f32;
            if k > 0 {
                let target_h = 0.004 + lift * s.sqrt() + 0.02 * s * wave;
                let fall = flow(base, n, 0.0) * (1.0 - gravity * s) + Vec3::new(0.0, -gravity * s, 0.0);
                dir = (dir * 0.6 + fall.normalize_or_zero() * 0.4).normalize_or_zero();
                let mut q = p + dir * ds;
                let (h, hn) = scalp.height(q);
                if h < target_h {
                    q += hn * (target_h - h);
                }
                n = hn;
                dir = (q - p).normalize_or_zero();
                p = q;
            }
            let side = n.cross(dir).normalize_or_zero();
            let wig = side * (wave * 0.035 * (s * len / wl * std::f32::consts::TAU + phase).sin());
            let half = width * 0.5 * (1.0 - 0.45 * s);
            for (u, sgn) in [(0.0, -1.0f32), (1.0, 1.0)] {
                vs.push(Vertex {
                    pos: (p + wig + side * half * sgn).to_array(),
                    nrm: n.to_array(),
                    uv: [u, s],
                    aux: dir.extend(0.0).to_array(),
                    aux2: [0.0; 4],
                    aux3: [0.0; 4],
                });
            }
        }
        for k in 0..segs as u32 {
            let a = first + k * 2;
            is.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
        }
    }
    let mut params = hair_params(kind::HAIR_CARD, c);
    params.p0[0] = 0.45;
    Draw { vertices: vs, indices: is, params, texture: Some(strand_texture(wave > 0.0)) }
}

/// Strand texture: R = strand id (for greying), G = shade, A = coverage.
pub fn strand_texture(wavy: bool) -> (u32, u32, Vec<u8>) {
    let (w, h) = (128u32, 256u32);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let mut rng = Rng::new(if wavy { 11 } else { 7 });
    for _ in 0..110 {
        let x0 = rng.range(4.0, w as f32 - 4.0);
        let tip = rng.range(0.55, 1.0);
        let id = rng.f32();
        let shade = rng.f32();
        let amp = if wavy { rng.range(3.0, 6.0) } else { rng.range(0.0, 1.5) };
        let ph = rng.range(0.0, 6.28);
        let thick = rng.range(0.9, 1.6);
        for y in 0..h {
            let v = y as f32 / h as f32;
            if v > tip {
                break;
            }
            let x = x0 + amp * (v * if wavy { 14.0 } else { 3.0 } + ph).sin();
            let taper = 1.0 - (v / tip).powf(4.0);
            for dx in -2i32..=2 {
                let xi = x as i32 + dx;
                if xi < 0 || xi >= w as i32 {
                    continue;
                }
                let cov = (1.0 - ((xi as f32 + 0.5 - x).abs() / thick)).clamp(0.0, 1.0) * taper;
                let i = ((y * w + xi as u32) * 4) as usize;
                if cov * 255.0 > px[i + 3] as f32 {
                    px[i] = (id * 255.0) as u8;
                    px[i + 1] = (shade * 255.0) as u8;
                    px[i + 3] = (cov * 255.0) as u8;
                }
            }
        }
    }
    (w, h, px)
}

#[allow(clippy::too_many_arguments)]
fn shells(
    scalp: &Scalp,
    c: &Colours,
    layers: usize,
    length: impl Fn(Vec3) -> f32,
    cell: f32,
    radius: f32,
    thick: f32,
    turns_per_cell: f32,
) -> Draw {
    let keep: Vec<bool> = scalp.tris.iter().map(|t| t.iter().any(|&i| scalp.dens[i as usize] > 0.03)).collect();
    let max_len: Vec<f32> = scalp.base.iter().map(|b| length(*b)).collect();
    let top = max_len.iter().cloned().fold(0.0, f32::max);
    let mut vs = Vec::new();
    let mut is = Vec::new();
    for l in 0..layers {
        let h = top * (l as f32 + 0.5) / layers as f32;
        let first = vs.len() as u32;
        for i in 0..scalp.pos.len() {
            let lift = h.min(max_len[i] * 1.02);
            vs.push(Vertex {
                pos: (scalp.pos[i] + scalp.nrm[i] * lift).to_array(),
                nrm: scalp.nrm[i].to_array(),
                uv: [max_len[i], scalp.dens[i]],
                aux: [scalp.pos[i].x, scalp.pos[i].y, scalp.pos[i].z, h],
                aux2: [0.0; 4],
                aux3: [0.0; 4],
            });
        }
        for (t, k) in scalp.tris.iter().zip(&keep) {
            if *k && t.iter().any(|&i| max_len[i as usize] * 1.1 >= h) {
                is.extend(t.iter().map(|&i| first + i));
            }
        }
    }
    let mut params = hair_params(kind::HAIR_SHELL, c);
    params.p0[0] = 0.4;
    // Coiled hair is matte: little of the sharp strand highlight survives.
    params.p0[1] = 0.3;
    params.p1 = [std::f32::consts::TAU * turns_per_cell / cell, cell, thick, radius];
    Draw { vertices: vs, indices: is, params, texture: None }
}

fn tubes(scalp: &Scalp, rng: &mut Rng, c: &Colours) -> Draw {
    let roots = scalp.sample_roots(rng, 320, 0.062);
    let ring = 8;
    let segs = 10;
    let mut vs = Vec::new();
    let mut is = Vec::new();
    for (root, n0, base) in roots {
        let len = rng.range(0.28, 0.48);
        let r0 = rng.range(0.028, 0.036);
        let id = rng.f32();
        let mut dir = (n0 * 0.5 + flow(base, n0, 0.3) * 0.9).normalize();
        let mut p = root - n0 * 0.01;
        let ds = len / segs as f32;
        let first = vs.len() as u32;
        let mut prev_side = n0.cross(dir).normalize_or_zero();
        for k in 0..=segs {
            let s = k as f32 / segs as f32;
            if k > 0 {
                dir = (dir + Vec3::new(0.0, -4.5, 0.0) * ds + flow(base, n0, 0.2) * 1.5 * ds).normalize();
                let mut q = p + dir * ds;
                let (h, hn) = scalp.height(q);
                let min_h = 0.035 + 0.03 * s;
                if h < min_h {
                    q += hn * (min_h - h);
                }
                dir = (q - p).normalize();
                p = q;
            }
            // Parallel transport keeps the ring from twisting.
            let side = (prev_side - dir * prev_side.dot(dir)).normalize_or_zero();
            prev_side = side;
            let up = dir.cross(side);
            let r = r0 * (1.0 - 0.35 * s) * if k == segs { 0.5 } else { 1.0 };
            for j in 0..=ring {
                let a = j as f32 / ring as f32 * std::f32::consts::TAU;
                let nr = side * a.cos() + up * a.sin();
                vs.push(Vertex {
                    pos: (p + nr * r).to_array(),
                    nrm: nr.to_array(),
                    uv: [j as f32 / ring as f32, s],
                    aux: [dir.x, dir.y, dir.z, id],
                    aux2: [0.0; 4],
                    aux3: [0.0; 4],
                });
            }
        }
        let stride = ring as u32 + 1;
        for k in 0..segs as u32 {
            for j in 0..ring as u32 {
                let a = first + k * stride + j;
                let b = a + stride;
                is.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
            }
        }
    }
    let mut params = hair_params(kind::HAIR_TUBE, c);
    params.p0[1] = 0.5;
    params.p1 = [7.0, 0.0, 0.0, 0.0];
    Draw { vertices: vs, indices: is, params, texture: None }
}
