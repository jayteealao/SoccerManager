//! Turns a genome and an age into draw calls for the skin and the eyes:
//! morphs the head, computes masks (brows, lips, beard, forehead, scalp) on
//! the unmorphed base mesh so they follow the morph, and picks colours.

use crate::age::{smoothstep, AgeState};
use crate::cards::{self, Scalp, Style};
use crate::genome::{EyeColour, Genome};
use crate::hair;
use crate::head::{BaseHead, HeadMesh, Part};
use crate::render::{kind, Draw, DrawParams, Vertex};
use glam::Vec3;
use std::collections::HashMap;

pub struct FaceBuilder {
    pub base: BaseHead,
    lip: HashMap<u32, f32>,
    /// Base-mesh eyeball centres and radii (left, right).
    eyes: [(Vec3, f32); 2],
}

impl FaceBuilder {
    pub fn new() -> std::io::Result<Self> {
        let mut base = BaseHead::load()?;
        let mut lip = HashMap::new();
        for name in ["mouth/mouth-lowerlip-volume-incr", "mouth/mouth-upperlip-volume-incr"] {
            let t = base.target(name).clone();
            let max = t.iter().map(|(_, d)| d.length()).fold(0.0, f32::max);
            for (i, d) in t {
                let m = smoothstep(0.12, 0.45, d.length() / max);
                let e = lip.entry(i).or_insert(0.0f32);
                *e = e.max(m);
            }
        }
        let eye = |part: Part| {
            let v: Vec<Vec3> = base
                .tris
                .iter()
                .filter(|(_, p)| *p == part)
                .flat_map(|(t, _)| t.map(|i| base.base[i as usize]))
                .collect();
            let c = v.iter().copied().sum::<Vec3>() / v.len() as f32;
            (c, v.iter().map(|p| p.distance(c)).fold(0.0, f32::max))
        };
        let eyes = [eye(Part::EyeL), eye(Part::EyeR)];
        Ok(Self { base, lip, eyes })
    }

    pub fn head(&mut self, g: &Genome, age: &AgeState) -> HeadMesh {
        let w = self.base.weights_for(g, age);
        self.base.build(&w)
    }

    /// A full portrait: skin, eyes and (optionally) hair.
    pub fn portrait(&mut self, g: &Genome, age: &AgeState, style: Option<Style>) -> Vec<Draw> {
        let head = self.head(g, age);
        let (mut draws, scalp) = self.draws(g, age, &head);
        if let Some(style) = style {
            let colours = Colours::new(g, age);
            let (hair, tint) = cards::build(style, g, &scalp, &colours);
            draws[0].params.p1[0] = tint;
            draws.extend(hair);
        }
        draws
    }

    /// Skin and eye draws, plus the scalp for hair placement.
    pub fn draws(&self, g: &Genome, age: &AgeState, head: &HeadMesh) -> (Vec<Draw>, Scalp) {
        let colours = Colours::new(g, age);
        let mut inv = vec![0u32; head.positions.len()];
        for (gi, ci) in &head.remap {
            inv[*ci as usize] = *gi;
        }
        let ao = curvature_ao(&head.positions, &head.normals, &head.indices);

        let mut skin_v = Vec::new();
        let mut skin_base = Vec::new();
        let mut skin_map = vec![u32::MAX; head.positions.len()];
        let mut eyes: [(Vec<Vertex>, Vec<u32>, Vec<u32>); 2] = Default::default();
        for (ci, p) in head.positions.iter().enumerate() {
            let b = self.base.base[inv[ci] as usize];
            let part = head.parts[ci];
            let mut v = Vertex {
                pos: p.to_array(),
                nrm: head.normals[ci].to_array(),
                ..Default::default()
            };
            match part {
                Part::Skin => {
                    let lip = self.lip.get(&inv[ci]).copied().unwrap_or(0.0);
                    v.uv = [ao[ci], forehead_mask(b)];
                    v.aux = [
                        brow_mask(b),
                        lip,
                        beard_mask(b) * (1.0 - lip),
                        hair::scalp_density(b, g, age),
                    ];
                    v.aux2 = [jersey_mask(b), crow_mask(b), self.lash_mask(b), 0.0];
                    skin_map[ci] = skin_v.len() as u32;
                    skin_base.push(b);
                    skin_v.push(v);
                }
                Part::EyeL | Part::EyeR => {
                    let e = &mut eyes[(part == Part::EyeR) as usize];
                    e.2.push(ci as u32);
                    e.0.push(v);
                }
            }
        }
        let mut skin_i = Vec::new();
        for t in head.indices.chunks(3) {
            match head.parts[t[0] as usize] {
                Part::Skin => skin_i.extend(t.iter().map(|&i| skin_map[i as usize])),
                p => {
                    let e = &mut eyes[(p == Part::EyeR) as usize];
                    for &i in t {
                        let local = e.2.iter().position(|&c| c == i).unwrap() as u32;
                        e.1.push(local);
                    }
                }
            }
        }

        let brow_density = 0.9;
        let stubble = g.beard * age.beard_growth * 0.75;
        let mut draws = vec![Draw {
            vertices: skin_v,
            indices: skin_i,
            params: DrawParams {
                kind: kind::SKIN,
                colour: colours.skin.extend(1.0).to_array(),
                colour2: colours.brow.extend(1.0).to_array(),
                p0: [age.skin_age, stubble, brow_density, 1.0],
                p1: [0.0, 0.0, 0.0, 0.0],
                ..Default::default()
            },
            texture: None,
        }];
        for (verts, idx, _) in eyes {
            let c = verts.iter().map(|v| Vec3::from(v.pos)).sum::<Vec3>() / verts.len() as f32;
            draws.push(Draw {
                vertices: verts,
                indices: idx,
                params: DrawParams {
                    kind: kind::EYE,
                    colour2: colours.iris.extend(1.0).to_array(),
                    p1: c.extend(0.0).to_array(),
                    ..Default::default()
                },
                texture: None,
            });
        }
        let sv = &draws[0].vertices;
        let scalp = Scalp::new(
            sv.iter().map(|v| Vec3::from(v.pos)).collect(),
            sv.iter().map(|v| Vec3::from(v.nrm)).collect(),
            skin_base,
            sv.iter().map(|v| v.aux[3]).collect(),
            draws[0].indices.chunks(3).map(|t| [t[0], t[1], t[2]]).collect(),
        );
        (draws, scalp)
    }
}

impl FaceBuilder {
    /// Lash line: skin on the lid margin, just in front of the eyeball; the
    /// upper lid carries more (lashes and lid shadow).
    fn lash_mask(&self, b: Vec3) -> f32 {
        let (c, r) = if b.x > 0.0 { self.eyes[0] } else { self.eyes[1] };
        let off = b.distance(c) - r;
        if b.z < c.z || (b.y - c.y).abs() > 0.09 {
            return 0.0;
        }
        let margin = 1.0 - smoothstep(0.004, 0.028, off.abs());
        let upper = if b.y > c.y { 1.0 } else { 0.45 };
        margin * upper
    }
}

pub fn srgb(r: u8, g: u8, b: u8) -> Vec3 {
    let f = |c: u8| {
        let c = c as f32 / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    Vec3::new(f(r), f(g), f(b))
}

fn ramp(stops: &[Vec3], t: f32) -> Vec3 {
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (t.floor() as usize).min(stops.len() - 2);
    stops[i].lerp(stops[i + 1], t - i as f32)
}

pub struct Colours {
    pub skin: Vec3,
    pub hair_root: Vec3,
    pub hair_tip: Vec3,
    pub brow: Vec3,
    pub iris: Vec3,
    pub grey: f32,
}

impl Colours {
    pub fn new(g: &Genome, age: &AgeState) -> Self {
        let skin_stops = [
            srgb(214, 180, 160),
            srgb(204, 160, 132),
            srgb(186, 142, 112),
            srgb(142, 100, 74),
            srgb(98, 68, 50),
            srgb(58, 40, 31),
        ];
        let mut skin = ramp(&skin_stops, g.skin_tone);
        // Undertone: warm/olive pushes green-yellow, cool pushes pink.
        let u = g.undertone * 0.08;
        skin *= Vec3::new(1.0 - u * 0.3, 1.0 + u * 0.4, 1.0 - u);

        let hair_stops = [srgb(214, 184, 130), srgb(150, 110, 66), srgb(78, 54, 36), srgb(22, 17, 14)];
        let mut hair = ramp(&hair_stops, g.hair_colour.dark);
        if g.hair_colour.red > 0.0 {
            hair = hair.lerp(srgb(150, 58, 24) * (1.2 - 0.6 * g.hair_colour.dark), g.hair_colour.red);
        }
        let hair_tip = hair * 1.25;
        let grey_col = srgb(150, 146, 140);
        let brow = hair.lerp(grey_col, age.grey * 0.35) * 0.9;

        let s = g.eye_shade;
        let iris = match g.eye_colour {
            EyeColour::Brown => srgb(58, 32, 16).lerp(srgb(118, 72, 34), s),
            EyeColour::Hazel => srgb(100, 76, 40).lerp(srgb(140, 110, 60), s),
            EyeColour::Green => srgb(74, 100, 62).lerp(srgb(110, 140, 90), s),
            EyeColour::Blue => srgb(60, 96, 130).lerp(srgb(120, 160, 196), s),
        };
        Self { skin, hair_root: hair, hair_tip, brow, iris, grey: age.grey }
    }
}

/// Eyebrow band, in base-mesh coordinates (decimetres).
fn brow_mask(b: Vec3) -> f32 {
    if b.z < 1.0 {
        return 0.0;
    }
    let ax = b.x.abs();
    let u = (ax - 0.06) / 0.50;
    if !(0.0..=1.0).contains(&u) {
        return 0.0;
    }
    let centre = 7.445 + 0.05 * (std::f32::consts::PI * u * 0.85).sin() - 0.025 * u;
    let half = 0.052 * (1.0 - 0.5 * u) + 0.014;
    let d = (b.y - centre).abs();
    (1.0 - smoothstep(half * 0.55, half, d)) * smoothstep(0.0, 0.08, u) * (1.0 - smoothstep(0.85, 1.0, u))
}

fn forehead_mask(b: Vec3) -> f32 {
    smoothstep(7.62, 7.72, b.y) * (1.0 - smoothstep(7.88, 8.0, b.y)) * smoothstep(1.05, 1.25, b.z)
        * (1.0 - smoothstep(0.35, 0.55, b.x.abs()))
}

fn beard_mask(b: Vec3) -> f32 {
    let ax = b.x.abs();
    // Upper edge: from the mouth corner up the cheek to the sideburn.
    let top = if ax < 0.26 { 6.80 } else { 6.80 + (ax - 0.26) / 0.52 * 0.55 };
    let upper = 1.0 - smoothstep(top - 0.08, top + 0.02, b.y);
    // Keep off the nose, in front of the ears, and fade down the neck.
    let nose = smoothstep(0.16, 0.24, ax).max(1.0 - smoothstep(6.74, 6.8, b.y));
    let front = smoothstep(0.35, 0.55, b.z + 0.25 * (1.0 - ax));
    let neck = smoothstep(5.95, 6.25, b.y);
    upper * nose * front * neck
}

/// Crew-neck collar line: everything below it is the shirt.
fn jersey_mask(b: Vec3) -> f32 {
    let az = b.x.atan2(b.z - 0.2).abs() / std::f32::consts::PI; // 0 front, 1 back
    let collar = 5.78 + 0.28 * az * az;
    1.0 - smoothstep(collar - 0.01, collar + 0.01, b.y)
}

fn crow_mask(b: Vec3) -> f32 {
    let d = Vec3::new(b.x.abs() - 0.52, b.y - 7.27, 0.0).length();
    (1.0 - smoothstep(0.05, 0.2, d)) * smoothstep(0.43, 0.5, b.x.abs()) * smoothstep(0.7, 0.95, b.z)
}

/// Cheap ambient occlusion from local concavity.
fn curvature_ao(pos: &[Vec3], nrm: &[Vec3], idx: &[u32]) -> Vec<f32> {
    let n = pos.len();
    let mut sum = vec![Vec3::ZERO; n];
    let mut cnt = vec![0.0f32; n];
    let mut edge = vec![0.0f32; n];
    for t in idx.chunks(3) {
        for k in 0..3 {
            let (a, b) = (t[k] as usize, t[(k + 1) % 3] as usize);
            sum[a] += pos[b];
            sum[b] += pos[a];
            cnt[a] += 1.0;
            cnt[b] += 1.0;
            let l = (pos[a] - pos[b]).length();
            edge[a] += l;
            edge[b] += l;
        }
    }
    let mut ao: Vec<f32> = (0..n)
        .map(|i| {
            if cnt[i] == 0.0 {
                return 1.0;
            }
            let lap = sum[i] / cnt[i] - pos[i];
            let e = edge[i] / cnt[i];
            let c = lap.dot(nrm[i]) / e.max(1e-4);
            (1.0 - 1.4 * c.max(0.0)).clamp(0.45, 1.0)
        })
        .collect();
    // Two smoothing passes over the one-ring.
    for _ in 0..2 {
        let mut s = vec![0.0f32; n];
        let mut c = vec![0.0f32; n];
        for t in idx.chunks(3) {
            for k in 0..3 {
                let (a, b) = (t[k] as usize, t[(k + 1) % 3] as usize);
                s[a] += ao[b];
                s[b] += ao[a];
                c[a] += 1.0;
                c[b] += 1.0;
            }
        }
        ao = (0..n).map(|i| if c[i] > 0.0 { 0.5 * ao[i] + 0.5 * s[i] / c[i] } else { ao[i] }).collect();
    }
    ao
}
