//! PoC 7: FLAME 2023 Open on its own (no MakeHuman geometry). The asset is
//! prepared by `flame/flame_prep.py` from the FLAME files the person provided.
//!
//! - Identity: FLAME shape betas, drawn per genome around a per-ancestry mean
//!   fitted to published facial norms.
//! - Age: FLAME has no age space and no permissive ageing data is available,
//!   so ageing is rule-based on FLAME's own mask regions (jowls, neck, eye
//!   bags, nasolabial fullness, lip thinning, nose and ear growth, youthful
//!   cheeks), plus skin, greying and the photo finish.
//! - Masks: FLAME_masks.pkl regions drive lips, forehead, scalp, ears, the
//!   shoulder band (jersey) and where the face ends.
//!
//! FLAME: Li, Bolkart, Black, Li, Romero, "Learning a model of facial shape
//! and expression from 4D scans", ACM ToG 2017. CC-BY-4.0.

use crate::age::{smoothstep, AgeState};
use crate::cards::{self, Scalp, Style};
use crate::face::{beard_mask, crow_mask, curvature_ao, forehead_mask, Colours};
use crate::genome::Genome;
use crate::head::vertex_normals;
use crate::render::{kind, Draw, DrawParams, Vertex};
use crate::rng::Rng;
use glam::Vec3;
use std::path::PathBuf;

const HEAD_VERTS: usize = 3931;

pub struct FlameAsset {
    pub verts: Vec<Vec3>,
    pub faces: Vec<[u32; 3]>,
    pub shape: Vec<Vec<Vec3>>,
    masks: Vec<u16>,
    names: Vec<String>,
    ancestry: [Vec<f32>; 3],
    regions: Regions,
}

/// Per-vertex weights for the age rules, computed once on the mean head.
struct Regions {
    jowl: Vec<f32>,
    neck: Vec<f32>,
    eyebag: Vec<f32>,
    nasolabial: Vec<f32>,
    cheek: Vec<f32>,
    lips: Vec<f32>,
    nose: Vec<f32>,
    ears: Vec<f32>,
    sto_y: f32,
    nose_root: Vec3,
    ear_c: [Vec3; 2],
    /// Lowest scalp-mask height per azimuth bin (0 = front, PI = back).
    hairline: Vec<f32>,
    crown_y: f32,
    /// Lip mask feathered over two rings.
    lip_soft: Vec<f32>,
}

const AZ_BINS: usize = 24;

fn azimuth(p: Vec3) -> f32 {
    p.x.atan2(p.z - 0.35).abs()
}

pub struct FlameHead {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
}

fn read<T: bytemuck::Pod + Clone>(path: &PathBuf) -> Vec<T> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e} (run flame/flame_prep.py)", path.display()));
    bytemuck::pod_collect_to_vec(&bytes)
}

fn v3(f: &[f32]) -> Vec<Vec3> {
    f.chunks(3).map(|c| Vec3::new(c[0], c[1], c[2])).collect()
}

impl FlameAsset {
    pub fn load() -> Self {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("downloads/flame_asset");
        let meta: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("meta.json")).unwrap()).unwrap();
        let k = meta["components"].as_u64().unwrap() as usize;
        let verts = v3(&read::<f32>(&dir.join("verts.f32")));
        let n = verts.len();
        let faces = read::<u32>(&dir.join("faces.u32")).chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
        let shape_flat = read::<f32>(&dir.join("shape.f32"));
        let shape = shape_flat.chunks(n * 3).map(v3).collect::<Vec<_>>();
        assert_eq!(shape.len(), k);
        let masks = read::<u16>(&dir.join("masks.u16"));
        let names = meta["masks"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect();
        let anc = read::<f32>(&dir.join("ancestry.f32"));
        let ancestry = [anc[0..k].to_vec(), anc[k..2 * k].to_vec(), anc[2 * k..3 * k].to_vec()];
        let mut a = Self { verts, faces, shape, masks, names, ancestry, regions: Regions::empty() };
        a.regions = Regions::new(&a);
        a
    }

    pub fn has(&self, v: usize, mask: &str) -> bool {
        let bit = self.names.iter().position(|n| n == mask).expect("mask name");
        self.masks[v] & (1 << bit) != 0
    }

    pub fn eye_ids(&self, left: bool) -> std::ops::Range<usize> {
        if left { 3931..4477 } else { 4477..5023 }
    }

    pub fn eye_centre(&self, left: bool) -> Vec3 {
        let r = self.eye_ids(left);
        let n = r.len() as f32;
        self.verts[r].iter().copied().sum::<Vec3>() / n
    }

    /// Identity betas: the ancestry mix's mean shift plus a seeded draw.
    pub fn betas(&self, g: &Genome) -> Vec<f32> {
        let mut r = Rng::fork(g.seed, "flame-beta");
        (0..self.shape.len())
            .map(|k| {
                let mean: f32 = (0..3).map(|a| g.mh_mix[a] * self.ancestry[a][k]).sum();
                mean + 0.85 * r.normal().clamp(-2.5, 2.5)
            })
            .collect()
    }

    pub fn build(&self, g: &Genome, age: &AgeState) -> FlameHead {
        let beta = self.betas(g);
        let mut pos = self.verts.clone();
        for (b, dir) in beta.iter().zip(&self.shape) {
            for (p, d) in pos.iter_mut().zip(dir) {
                *p += *d * *b;
            }
        }
        self.age(&mut pos, g, age);
        // Keep the eyes at the shared anchor, as the other heads do.
        let c = (self.eye_centre_of(&pos, true) + self.eye_centre_of(&pos, false)) * 0.5;
        let shift = crate::head::EYE_ANCHOR - c;
        for p in &mut pos {
            *p += shift;
        }
        let idx: Vec<u32> = self.faces.iter().flatten().copied().collect();
        let normals = vertex_normals(&pos, &idx);
        FlameHead { positions: pos, normals }
    }

    fn eye_centre_of(&self, pos: &[Vec3], left: bool) -> Vec3 {
        let r = self.eye_ids(left);
        let n = r.len() as f32;
        pos[r].iter().copied().sum::<Vec3>() / n
    }

    /// Rule-based ageing on FLAME's regions (decimetres).
    fn age(&self, pos: &mut [Vec3], g: &Genome, a: &AgeState) {
        let r = &self.regions;
        let late = smoothstep(35.0, 70.0, a.years);
        let fat = g.fat_tendency * smoothstep(33.0, 58.0, a.years);
        let youth = (a.mh_child / 0.45).clamp(0.0, 1.0);
        for v in 0..HEAD_VERTS {
            let p = self.verts[v];
            let side = p.x.signum();
            let mut d = Vec3::ZERO;
            // Fat moves lower: jowls drop and spread, the front of the neck fills.
            // Magnitudes in decimetres: real sag is of the order of 5-10 mm.
            d += r.jowl[v] * (Vec3::new(0.05 * side, -0.13, 0.015) * (0.5 * late + 0.8 * fat));
            d += r.neck[v] * Vec3::new(0.0, -0.05, 0.14) * (0.3 * late + fat);
            // Eye bags and a fuller cheek beside the nasolabial fold.
            d += r.eyebag[v] * Vec3::new(0.0, -0.015, 0.07) * late;
            d += r.nasolabial[v] * Vec3::new(0.02 * side, -0.03, 0.055) * late;
            // Youth: fuller, rounder cheeks that thin out by the early twenties.
            d += r.cheek[v] * Vec3::new(0.025 * side, 0.0, 0.03) * youth;
            // Upper lids sit lower with age (dermatochalasis), handled below.
            // Lips thin with age.
            if r.lips[v] > 0.0 {
                d.y += (r.sto_y - p.y) * 0.18 * late * r.lips[v];
            }
            // Nose and ears keep growing slowly.
            if r.nose[v] > 0.0 {
                d += (p - r.nose_root) * (0.05 * late - 0.04 * youth) * r.nose[v];
            }
            if r.ears[v] > 0.0 {
                let c = if side > 0.0 { r.ear_c[0] } else { r.ear_c[1] };
                d += (p - c) * 0.07 * late * r.ears[v];
            }
            pos[v] += d;
        }
        // Relaxed upper lids: FLAME's mean face has wide-open eyes. Rotate the
        // upper-lid band down about each eyeball centre so the lid covers the
        // top of the iris; a little more droop late in life.
        let angle = (11.0 + 5.0 * late).to_radians();
        for left in [true, false] {
            let c = self.eye_centre(left);
            let rad = self.verts[self.eye_ids(left)].iter().map(|p| p.distance(c)).fold(0.0, f32::max);
            for v in 0..HEAD_VERTS {
                let p = self.verts[v];
                if (p.x > 0.0) != left || p.y < c.y || p.z < c.z {
                    continue;
                }
                let off = p.distance(c) - rad;
                let w = (1.0 - smoothstep(0.0, 0.06, off)) * (1.0 - smoothstep(0.1, 0.2, p.y - c.y)) * (1.0 - smoothstep(0.18, 0.3, (p.x - c.x).abs()));
                if w <= 0.0 {
                    continue;
                }
                let rel = pos[v] - c;
                let a = angle * w;
                let (sn, cs) = a.sin_cos();
                // Rotate about the x axis: top of the lid moves forward-down.
                let ry = rel.y * cs - rel.z * sn;
                let rz = rel.y * sn + rel.z * cs;
                pos[v] = c + Vec3::new(rel.x, ry, rz);
            }
        }
    }

    /// Skin and eye draws plus the scalp for hair, like `FaceBuilder::draws`.
    pub fn portrait(&self, g: &Genome, age: &AgeState, style: Option<Style>) -> Vec<Draw> {
        let head = self.build(g, age);
        let colours = Colours::new(g, age);
        let skin_idx: Vec<u32> = self
            .faces
            .iter()
            .filter(|t| t.iter().all(|&i| (i as usize) < HEAD_VERTS))
            .flatten()
            .copied()
            .collect();
        let hp = &head.positions[..HEAD_VERTS];
        let ao = curvature_ao(hp, &head.normals[..HEAD_VERTS], &skin_idx);
        let eyes = [self.eye_centre(true), self.eye_centre(false)];
        let eye_r = self.verts[self.eye_ids(true)].iter().map(|p| p.distance(eyes[0])).fold(0.0, f32::max);
        let mut skin = Vec::with_capacity(HEAD_VERTS);
        let mut dens = Vec::with_capacity(HEAD_VERTS);
        for v in 0..HEAD_VERTS {
            let b = self.verts[v];
            let face = self.has(v, "face");
            let ear = self.has(v, "left_ear") || self.has(v, "right_ear");
            let neck = self.has(v, "neck") || self.has(v, "boundary");
            let lip = self.regions.lip_soft[v];
            let forehead = if self.has(v, "forehead") { forehead_mask(b).max(0.5) } else { 0.0 };
            let hair_ok = if face || ear || neck { 0.0 } else { 1.0 };
            let scalp = self.scalp_density(b, g, age) * hair_ok;
            dens.push(scalp);
            // Lash line: head vertices on the eyeball surface, in front.
            let (c, _) = if b.x > 0.0 { (eyes[0], 0) } else { (eyes[1], 1) };
            let off = b.distance(c) - eye_r;
            let lash = if b.z > c.z && (b.y - c.y).abs() < 0.09 {
                (1.0 - smoothstep(0.004, 0.03, off.abs())) * if b.y > c.y { 1.0 } else { 0.45 }
            } else {
                0.0
            };
            let jersey = if self.has(v, "boundary") { 1.0 } else { 0.0 };
            skin.push(Vertex {
                pos: head.positions[v].to_array(),
                nrm: head.normals[v].to_array(),
                uv: [ao[v], forehead],
                aux: [brow(b, c), lip, beard_mask(b) * (1.0 - lip) * if face || neck { 1.0 } else { 0.0 }, scalp],
                aux2: [jersey, crow_mask(b), lash, 0.0],
                aux3: [0.0; 4],
            });
        }
        let stubble = g.beard * age.beard_growth * 0.75;
        let mut draws = vec![Draw {
            vertices: skin,
            indices: skin_idx.clone(),
            params: DrawParams {
                kind: kind::SKIN,
                colour: colours.skin.extend(1.0).to_array(),
                colour2: colours.brow.extend(1.0).to_array(),
                p0: [age.skin_age, stubble, 0.9, 0.55],
                ..Default::default()
            },
            texture: None,
        }];
        for left in [true, false] {
            let r = self.eye_ids(left);
            let c = self.eye_centre_of(&head.positions, left);
            let verts: Vec<Vertex> = r
                .clone()
                .map(|v| Vertex { pos: head.positions[v].to_array(), nrm: head.normals[v].to_array(), ..Default::default() })
                .collect();
            let idx: Vec<u32> = self
                .faces
                .iter()
                .filter(|t| t.iter().all(|&i| r.contains(&(i as usize))))
                .flat_map(|t| t.map(|i| i - r.start as u32))
                .collect();
            draws.push(Draw {
                vertices: verts,
                indices: idx,
                params: DrawParams {
                    kind: kind::EYE,
                    colour2: colours.iris.extend(1.0).to_array(),
                    // FLAME's eyeball (14.3 mm radius) needs a smaller iris angle
                    // than MakeHuman's for a real ~11.7 mm iris.
                    p1: c.extend(0.91).to_array(),
                    ..Default::default()
                },
                texture: None,
            });
        }
        if let Some(style) = style {
            let scalp = Scalp::new(
                hp.to_vec(),
                head.normals[..HEAD_VERTS].to_vec(),
                self.verts[..HEAD_VERTS].to_vec(),
                dens,
                skin_idx.chunks(3).map(|t| [t[0], t[1], t[2]]).collect(),
            );
            let (hair, tint) = cards::build(style, g, &scalp, &colours);
            draws[0].params.p1[0] = tint;
            draws.extend(hair);
        }
        draws
    }
}

impl FlameAsset {
    /// Hair density at a mean-head point: FLAME's scalp region, with the
    /// genome's hairline height and the age's recession on the front.
    fn scalp_density(&self, b: Vec3, g: &Genome, age: &AgeState) -> f32 {
        let az = azimuth(b);
        let t = az / std::f32::consts::PI * (AZ_BINS as f32 - 1.0);
        let i = (t.floor() as usize).min(AZ_BINS - 2);
        let f = t - i as f32;
        let mut line = self.regions.hairline[i] * (1.0 - f) + self.regions.hairline[i + 1] * f;
        let front = 1.0 - smoothstep(0.5, 1.2, az);
        let temple = smoothstep(0.3, 0.6, az) * (1.0 - smoothstep(0.8, 1.1, az));
        line += front * (g.hairline.height * 0.05 + 0.3 * age.recession) + temple * 0.2 * age.recession;
        let mut d = smoothstep(line - 0.03, line + 0.05, b.y);
        let crown = Vec3::new(0.0, self.regions.crown_y, 0.15);
        let rec = age.recession;
        let r = (b - crown).length();
        d *= 1.0 - smoothstep(0.55, 1.0, rec) * (1.0 - smoothstep(0.15 + 0.35 * rec, 0.35 + 0.4 * rec, r));
        d
    }
}

/// Eyebrow band above each eye, relative to that eye's centre.
pub fn brow(b: Vec3, eye: Vec3) -> f32 {
    if b.z < eye.z - 0.1 {
        return 0.0;
    }
    let u = (b.x.abs() - 0.06) / 0.50;
    if !(0.0..=1.0).contains(&u) {
        return 0.0;
    }
    let centre = eye.y + 0.15 + 0.05 * (std::f32::consts::PI * u * 0.85).sin() - 0.025 * u;
    let half = 0.052 * (1.0 - 0.5 * u) + 0.014;
    let d = (b.y - centre).abs();
    (1.0 - smoothstep(half * 0.55, half, d)) * smoothstep(0.0, 0.08, u) * (1.0 - smoothstep(0.85, 1.0, u))
}

impl Regions {
    fn empty() -> Self {
        Self {
            jowl: vec![],
            neck: vec![],
            eyebag: vec![],
            nasolabial: vec![],
            cheek: vec![],
            lips: vec![],
            nose: vec![],
            ears: vec![],
            sto_y: 0.0,
            nose_root: Vec3::ZERO,
            ear_c: [Vec3::ZERO; 2],
            hairline: vec![],
            crown_y: 0.0,
            lip_soft: vec![],
        }
    }

    fn new(a: &FlameAsset) -> Self {
        let v = &a.verts;
        let n = HEAD_VERTS;
        let set = |m: &str| -> Vec<usize> { (0..n).filter(|&i| a.has(i, m)).collect() };
        let mean = |ids: &[usize]| ids.iter().map(|&i| v[i]).sum::<Vec3>() / ids.len().max(1) as f32;
        let lips = set("lips");
        let nose = set("nose");
        let sto_y = mean(&lips).y;
        let lip_w = lips.iter().map(|&i| v[i].x.abs()).fold(0.0, f32::max);
        let nose_root = {
            let top = nose.iter().map(|&i| v[i].y).fold(f32::MIN, f32::max);
            Vec3::new(0.0, top, mean(&nose).z)
        };
        let chin_y = (0..n).filter(|&i| a.has(i, "face") && v[i].x.abs() < 0.03).map(|i| v[i].y).fold(f32::MAX, f32::min);
        let eye = [a.eye_centre(true), a.eye_centre(false)];
        let ear_c = [mean(&set("left_ear")), mean(&set("right_ear"))];
        let al_x = nose.iter().map(|&i| v[i].x.abs()).fold(0.0, f32::max);
        let nose_base = nose.iter().map(|&i| v[i].y).fold(f32::MAX, f32::min);
        let mut r = Self::empty();
        r.sto_y = sto_y;
        r.nose_root = nose_root;
        r.ear_c = ear_c;
        // Hairline: FLAME's scalp mask gives the resting hair region.
        let mut line = vec![f32::MAX; AZ_BINS];
        for i in set("scalp") {
            let b = ((azimuth(v[i]) / std::f32::consts::PI) * (AZ_BINS as f32 - 1.0)).round() as usize;
            line[b] = line[b].min(v[i].y);
        }
        for b in 0..AZ_BINS {
            if line[b] == f32::MAX {
                line[b] = if b > 0 { line[b - 1] } else { 8.0 };
            }
        }
        r.hairline = line;
        r.crown_y = (0..n).map(|i| v[i].y).fold(f32::MIN, f32::max) - 0.1;
        // Feather the lip mask: average over neighbours twice.
        let mut soft: Vec<f32> = (0..n).map(|i| if a.has(i, "lips") { 1.0 } else { 0.0 }).collect();
        let tris: Vec<[u32; 3]> = a.faces.iter().filter(|t| t.iter().all(|&i| (i as usize) < n)).copied().collect();
        for _ in 0..2 {
            let mut acc = vec![0.0f32; n];
            let mut cnt = vec![0.0f32; n];
            for t in &tris {
                for k in 0..3 {
                    let (x, y) = (t[k] as usize, t[(k + 1) % 3] as usize);
                    acc[x] += soft[y];
                    acc[y] += soft[x];
                    cnt[x] += 1.0;
                    cnt[y] += 1.0;
                }
            }
            soft = (0..n).map(|i| if cnt[i] > 0.0 { 0.5 * soft[i] + 0.5 * acc[i] / cnt[i] } else { soft[i] }).collect();
        }
        r.lip_soft = soft.iter().map(|x| smoothstep(0.45, 0.9, *x)).collect();
        for i in 0..n {
            let p = v[i];
            let face = a.has(i, "face");
            let ax = p.x.abs();
            // Jowls: lateral lower face, below the mouth corners.
            let jowl = if face {
                smoothstep(lip_w * 0.8, lip_w * 1.3, ax)
                    * (1.0 - smoothstep(sto_y - 0.05, sto_y + 0.1, p.y))
                    * smoothstep(chin_y - 0.1, chin_y + 0.15, p.y)
            } else {
                0.0
            };
            let neck = if a.has(i, "neck") { smoothstep(0.3, 0.8, p.z) * (1.0 - smoothstep(0.2, 0.5, ax)) } else { 0.0 };
            let e = if p.x > 0.0 { eye[0] } else { eye[1] };
            let dy = p.y - e.y;
            let eyebag = if face && p.z > e.z - 0.1 {
                smoothstep(-0.28, -0.14, dy) * (1.0 - smoothstep(-0.12, -0.05, dy)) * (1.0 - smoothstep(0.12, 0.22, (p.x - e.x).abs()))
            } else {
                0.0
            };
            // Beside the line from nose wing to mouth corner, on the cheek side.
            let t = ((nose_base - p.y) / (nose_base - sto_y)).clamp(0.0, 1.0);
            let line_x = al_x + (lip_w - al_x) * t;
            let nasolabial = if face && p.y < nose_base + 0.03 && p.y > sto_y - 0.08 && p.z > 0.9 {
                smoothstep(line_x, line_x + 0.05, ax) * (1.0 - smoothstep(line_x + 0.1, line_x + 0.2, ax))
            } else {
                0.0
            };
            let cheek = if face && p.y < e.y - 0.12 && p.y > sto_y && p.z > 0.8 {
                smoothstep(0.2, 0.35, ax) * (1.0 - smoothstep(0.55, 0.7, ax))
            } else {
                0.0
            };
            r.jowl.push(jowl);
            r.neck.push(neck);
            r.eyebag.push(eyebag);
            r.nasolabial.push(nasolabial);
            r.cheek.push(cheek);
            r.lips.push(if a.has(i, "lips") { 1.0 } else { 0.0 });
            r.nose.push(if a.has(i, "nose") { 1.0 } else { 0.0 });
            r.ears.push(if a.has(i, "left_ear") || a.has(i, "right_ear") { 1.0 } else { 0.0 });
        }
        // Feather the ageing regions so their displacements have no edges.
        let feather = |mut f: Vec<f32>, passes: usize| -> Vec<f32> {
            for _ in 0..passes {
                let mut acc = vec![0.0f32; n];
                let mut cnt = vec![0.0f32; n];
                for t in &tris {
                    for k in 0..3 {
                        let (x, y) = (t[k] as usize, t[(k + 1) % 3] as usize);
                        acc[x] += f[y];
                        acc[y] += f[x];
                        cnt[x] += 1.0;
                        cnt[y] += 1.0;
                    }
                }
                f = (0..n).map(|i| if cnt[i] > 0.0 { 0.5 * f[i] + 0.5 * acc[i] / cnt[i] } else { f[i] }).collect();
            }
            f
        };
        r.jowl = feather(std::mem::take(&mut r.jowl), 12);
        r.neck = feather(std::mem::take(&mut r.neck), 8);
        r.eyebag = feather(std::mem::take(&mut r.eyebag), 6);
        r.nasolabial = feather(std::mem::take(&mut r.nasolabial), 8);
        r.cheek = feather(std::mem::take(&mut r.cheek), 10);
        r
    }
}
