//! PoC 8: Google's GNM Head v3.0 (Apache-2.0) as the head model.
//!
//! The asset is prepared by `gnm/gnm_prep.py` from the model data shipped in
//! https://github.com/google/GNM (no GNM code is run). This module:
//! - samples identity with GNM's gender- and ethnicity-conditioned decoder
//!   (a 5-layer MLP, reimplemented here), conditioned on the genome's
//!   ancestry shares through `genes::Comp` -> GNM class weights, plus a
//!   residual that restores published within-group spread (`gnm_norms.py`);
//! - ages the head with rules on GNM's named regions (as PoC 7 did on
//!   FLAME's masks);
//! - builds portrait draws for the PoC 8 shaders: two-chromophore skin with
//!   pre-integrated scattering, an eye interior with refracted iris lookup,
//!   a cornea, an eye-occlusion shell, eyelashes and Marschner hair cards.

use crate::age::{smoothstep, AgeState};
use crate::cards::{self, Scalp, Style};
use crate::face::{beard_mask, crow_mask, curvature_ao, Colours};
use crate::genes::{chromophores, Genome2};
use crate::head::vertex_normals;
use crate::render::{kind, Draw, DrawParams, Vertex};
use glam::{Vec2, Vec3};
use std::path::PathBuf;

pub const IDENTITY: usize = 253;

/// Extra between-class contrast and residual scale, chosen in gnm_norms.py
/// so class contrasts move towards Farkas 2005 and within-class CVs match
/// Fang 2011 (REPORT.md, PoC 8).
pub const CLASS_GAIN: f32 = 1.0;
pub const RESID_SCALE: f32 = 0.6;

struct Layer {
    k: Vec<f32>,
    b: Vec<f32>,
    n_in: usize,
    n_out: usize,
    relu: bool,
}

pub struct GnmAsset {
    pub verts: Vec<Vec3>,
    pub faces: Vec<[u32; 3]>,
    basis: Vec<f32>,
    eyes_t: [Vec3; 2],
    eyes_b: Vec<[Vec3; 2]>,
    groups: Vec<u64>,
    names: Vec<String>,
    layers: Vec<Layer>,
    class_means: Vec<Vec<f32>>,
    resid: Vec<f32>,
    lm68: Vec<[(usize, f32); 3]>,
    pub picks: serde_json::Map<String, serde_json::Value>,
    regions: Regions,
    lut: (u32, u32, Vec<u8>),
}

pub struct GnmHead {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub eyes: [Vec3; 2],
}

fn read<T: bytemuck::Pod + Clone>(path: &PathBuf) -> Vec<T> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e} (run gnm/gnm_prep.py and gnm/gnm_norms.py)", path.display()));
    bytemuck::pod_collect_to_vec(&bytes)
}

fn v3(f: &[f32]) -> Vec<Vec3> {
    f.chunks(3).map(|c| Vec3::new(c[0], c[1], c[2])).collect()
}

/// Per-vertex weights computed once on the template.
struct Regions {
    skin_n: usize,
    jowl: Vec<f32>,
    neck: Vec<f32>,
    eyebag: Vec<f32>,
    nasolabial: Vec<f32>,
    cheek: Vec<f32>,
    lips: Vec<f32>,
    lip_soft: Vec<f32>,
    nose: Vec<f32>,
    ears: Vec<f32>,
    forehead: Vec<f32>,
    hb: Vec<f32>,
    oil: Vec<f32>,
    sto_y: f32,
    nose_root: Vec3,
    ear_c: [Vec3; 2],
    chin_y: f32,
    bottom_y: f32,
    /// Lid-opening polygon per eye (x, y relative to the eye centre),
    /// from iBUG-68 points 36-41 / 42-47, upper arc first.
    lids: [Vec<Vec2>; 2],
    /// Per eye: inner and outer corner x, and cubic coefficients in
    /// t = (x - inner) / (outer - inner) for the upper and lower margins.
    lid_fit: [(f32, f32, [f32; 4], [f32; 4]); 2],
}

impl GnmAsset {
    pub fn load() -> Self {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("downloads/gnm_asset");
        let meta: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("meta.json")).unwrap()).unwrap();
        let mut verts = v3(&read::<f32>(&dir.join("verts.f32")));
        let n = verts.len();
        // Open the relaxed scan lids by 1.8 mm (gnm_prep.py): the template
        // everything else is measured on includes it.
        for (v, d) in verts.iter_mut().zip(v3(&read::<f32>(&dir.join("lid_open.f32")))) {
            *v += d;
        }
        let faces = read::<u32>(&dir.join("faces.u32")).chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
        let basis = read::<f32>(&dir.join("identity.f32"));
        assert_eq!(basis.len(), IDENTITY * n * 3);
        let e = read::<f32>(&dir.join("eyes.f32"));
        let eyes_t = [Vec3::new(e[0], e[1], e[2]), Vec3::new(e[3], e[4], e[5])];
        let eyes_b = (0..IDENTITY)
            .map(|i| {
                let o = 6 + i * 6;
                [Vec3::new(e[o], e[o + 1], e[o + 2]), Vec3::new(e[o + 3], e[o + 4], e[o + 5])]
            })
            .collect();
        let groups = read::<u64>(&dir.join("groups.u64"));
        let names = meta["groups"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect();
        let dec = read::<f32>(&dir.join("decoder.f32"));
        let mut off = 0;
        let layers = meta["decoder"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| {
                let (ni, no) = (l["in"].as_u64().unwrap() as usize, l["out"].as_u64().unwrap() as usize);
                let k = dec[off..off + ni * no].to_vec();
                let b = dec[off + ni * no..off + ni * no + no].to_vec();
                off += ni * no + no;
                Layer { k, b, n_in: ni, n_out: no, relu: l["activation"] == "relu" }
            })
            .collect();
        let cm = read::<f32>(&dir.join("class_means.f32"));
        let class_means = cm.chunks(IDENTITY).map(|c| c.to_vec()).collect();
        let resid = read::<f32>(&dir.join("resid.f32"));
        let lm: Vec<Vec<(usize, f32)>> = serde_json::from_str(&std::fs::read_to_string(dir.join("lm68.json")).unwrap()).unwrap();
        let lm68 = lm.iter().map(|t| [t[0], t[1], t[2]]).collect();
        let picks: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("landmarks.json")).unwrap()).unwrap();
        let mut a = Self {
            verts,
            faces,
            basis,
            eyes_t,
            eyes_b,
            groups,
            names,
            layers,
            class_means,
            resid,
            lm68,
            picks: picks.as_object().unwrap().clone(),
            regions: Regions::empty(),
            lut: preintegrated_lut(),
        };
        a.regions = Regions::new(&a);
        a
    }

    pub fn has(&self, v: usize, group: &str) -> bool {
        let bit = self.names.iter().position(|n| n == group).unwrap_or_else(|| panic!("group {group}"));
        self.groups[v] & (1 << bit) != 0
    }

    fn set(&self, group: &str) -> Vec<usize> {
        (0..self.verts.len()).filter(|&i| self.has(i, group)).collect()
    }

    pub fn landmark(&self, pos: &[Vec3], i: usize) -> Vec3 {
        self.lm68[i].iter().map(|(v, w)| pos[*v] * *w).sum()
    }

    /// GNM's identity decoder: MLP([z, condition]) -> 253 coefficients.
    pub fn decode(&self, z: &[f32], cond: &[f32; 6]) -> Vec<f32> {
        let mut x: Vec<f32> = z.iter().chain(cond.iter()).copied().collect();
        for l in &self.layers {
            let mut y = l.b.clone();
            for i in 0..l.n_in {
                let xi = x[i];
                if xi == 0.0 {
                    continue;
                }
                let row = &l.k[i * l.n_out..(i + 1) * l.n_out];
                for (yo, k) in y.iter_mut().zip(row) {
                    *yo += xi * k;
                }
            }
            if l.relu {
                y.iter_mut().for_each(|v| *v = v.max(0.0));
            }
            x = y;
        }
        x
    }

    /// Identity coefficients for a genome: decoder sample at the genome's
    /// ancestry condition, extra class contrast, and the residual.
    pub fn identity(&self, g: &Genome2) -> Vec<f32> {
        self.identity_for(&g.gnm_condition(), &g.face_z, &g.face_r)
    }

    pub fn identity_for(&self, cond: &[f32; 6], z: &[f32], r: &[f32]) -> Vec<f32> {
        let mut id = self.decode(z, cond);
        for k in 0..IDENTITY {
            let gm: f32 = self.class_means.iter().map(|m| m[k]).sum::<f32>() / 4.0;
            let cm: f32 = (0..4).map(|e| cond[2 + e] * self.class_means[e][k]).sum::<f32>();
            id[k] += CLASS_GAIN * (cm - gm) + RESID_SCALE * self.resid[k] * r.get(k).copied().unwrap_or(0.0);
        }
        id
    }

    /// Head positions for identity coefficients, before age and anchoring.
    pub fn shape(&self, id: &[f32]) -> (Vec<Vec3>, [Vec3; 2]) {
        let n = self.verts.len();
        let mut pos = self.verts.clone();
        let mut eyes = self.eyes_t;
        for (k, c) in id.iter().enumerate() {
            if *c == 0.0 {
                continue;
            }
            let b = &self.basis[k * n * 3..(k + 1) * n * 3];
            for (p, d) in pos.iter_mut().zip(b.chunks(3)) {
                *p += Vec3::new(d[0], d[1], d[2]) * *c;
            }
            eyes[0] += self.eyes_b[k][0] * *c;
            eyes[1] += self.eyes_b[k][1] * *c;
        }
        (pos, eyes)
    }

    pub fn build(&self, g: &Genome2, age: &AgeState) -> GnmHead {
        self.build_id(&self.identity(g), g.fat_tendency, age)
    }

    pub fn build_id(&self, id: &[f32], fat_tendency: f32, age: &AgeState) -> GnmHead {
        let (mut pos, mut eyes) = self.shape(id);
        self.age(&mut pos, fat_tendency, age);
        let shift = crate::head::EYE_ANCHOR - (eyes[0] + eyes[1]) * 0.5;
        pos.iter_mut().for_each(|p| *p += shift);
        eyes.iter_mut().for_each(|p| *p += shift);
        let idx: Vec<u32> = self.faces.iter().flatten().copied().collect();
        let normals = vertex_normals(&pos, &idx);
        GnmHead { positions: pos, normals, eyes }
    }

    /// Rule-based ageing on GNM's regions (decimetres), as PoC 7 on FLAME.
    fn age(&self, pos: &mut [Vec3], fat_tendency: f32, a: &AgeState) {
        let r = &self.regions;
        let late = smoothstep(35.0, 70.0, a.years);
        let fat = fat_tendency * smoothstep(33.0, 58.0, a.years);
        let youth = (a.mh_child / 0.45).clamp(0.0, 1.0);
        for v in 0..r.skin_n {
            let p = self.verts[v];
            let side = p.x.signum();
            let mut d = Vec3::ZERO;
            d += r.jowl[v] * (Vec3::new(0.04 * side, -0.07, 0.015) * (0.45 * late + 0.7 * fat));
            d += r.neck[v] * Vec3::new(0.0, -0.04, 0.12) * (0.3 * late + fat);
            d += r.eyebag[v] * Vec3::new(0.0, -0.012, 0.06) * late;
            d += r.nasolabial[v] * Vec3::new(0.015 * side, -0.015, 0.035) * late;
            d += r.cheek[v] * Vec3::new(0.025 * side, 0.0, 0.03) * youth;
            if r.lips[v] > 0.0 {
                d.y += (r.sto_y - p.y) * 0.18 * late * r.lips[v];
            }
            if r.nose[v] > 0.0 {
                d += (p - r.nose_root) * (0.05 * late - 0.04 * youth) * r.nose[v];
            }
            if r.ears[v] > 0.0 {
                let c = if side > 0.0 { r.ear_c[0] } else { r.ear_c[1] };
                d += (p - c) * 0.07 * late * r.ears[v];
            }
            // Upper lid skin descends a little late in life.
            pos[v] += d;
        }
    }

    /// Portrait draws for PoC 8. `years` sets AgeState from the genome's
    /// sourced greying and baldness curves.
    pub fn portrait(&self, g: &Genome2, years: f32, style: Option<Style>) -> Vec<Draw> {
        let legacy = g.legacy();
        let mut age = AgeState::new(&legacy, years);
        age.grey = g.grey_at(years);
        age.recession = g.recession_at(years);
        let head = self.build(g, &age);
        self.draws(g, &legacy, &age, &head, style)
    }

    pub fn age_state(&self, g: &Genome2, years: f32) -> AgeState {
        let mut age = AgeState::new(&g.legacy(), years);
        age.grey = g.grey_at(years);
        age.recession = g.recession_at(years);
        age
    }

    pub fn draws(&self, g: &Genome2, legacy: &crate::genome::Genome, age: &AgeState, head: &GnmHead, style: Option<Style>) -> Vec<Draw> {
        let r = &self.regions;
        let sn = r.skin_n;
        let colours = Colours::new(legacy, age);
        let skin_tris: Vec<[u32; 3]> = self
            .faces
            .iter()
            .filter(|t| t.iter().all(|&i| (i as usize) < sn && !self.has(i as usize, "mouth_sock")))
            .copied()
            .collect();
        let skin_idx: Vec<u32> = skin_tris.iter().flatten().copied().collect();
        let hp = &head.positions[..sn];
        let hn = &head.normals[..sn];
        let ao = curvature_ao(hp, hn, &skin_idx);
        let curv = curvature(hp, hn, &skin_tris);
        let eyes_t = self.eyes_t;
        let mut dens = Vec::with_capacity(sn);
        let mut skin = Vec::with_capacity(sn);
        let (cm, ch) = chromophores::invert(g.skin_lab);
        for v in 0..sn {
            let b = self.verts[v];
            let e = if b.x > 0.0 { 0 } else { 1 };
            let face = self.has(v, "hockey_mask");
            let ear = self.has(v, "ears");
            let neck = b.y < r.chin_y + 0.05 && !face;
            let scalp = if face || ear || neck { 0.0 } else { self.scalp_density(b, g, age) };
            dens.push(scalp);
            let rel = b - eyes_t[e];
            let (lash, wet, caruncle) = lid_masks(&r.lids[e], Vec2::new(rel.x, rel.y), rel.z, e);
            let jersey = if b.y < r.bottom_y + 0.3 { 1.0 } else { 0.0 };
            let cav = (1.0 + curv[v].min(0.0) * 0.06).clamp(0.3, 1.0);
            skin.push(Vertex {
                pos: hp[v].to_array(),
                nrm: hn[v].to_array(),
                uv: [ao[v], r.forehead[v]],
                aux: [
                    crate::flamehead::brow(b, eyes_t[e]),
                    r.lip_soft[v],
                    beard_mask(b) * (1.0 - r.lip_soft[v]) * if face || neck { 1.0 } else { 0.0 },
                    scalp,
                ],
                aux2: [jersey, crow_mask(b), lash, curv[v].abs()],
                aux3: [r.hb[v] + 0.6 * caruncle, r.oil[v], wet.max(caruncle), cav],
            });
        }
        let stubble = g.beard * age.beard_growth * 0.75;
        let mut draws = vec![Draw {
            vertices: skin,
            indices: skin_idx.clone(),
            params: DrawParams {
                kind: kind::SKIN2,
                colour: [cm, ch, age.skin_age, 0.0],
                colour2: colours.brow.extend(1.0).to_array(),
                p0: [age.skin_age, stubble, 0.9, 0.55],
                p1: [0.0, 0.05, 0.0, 0.0],
                ..Default::default()
            },
            texture: Some(self.lut.clone()),
        }];
        draws.extend(self.eye_draws(head, &colours, age));
        draws.extend(lashes(&r.lids, head, g));
        if let Some(style) = style {
            let scalp = Scalp::new(hp.to_vec(), hn.to_vec(), self.verts[..sn].to_vec(), dens, skin_tris.clone());
            let (hair, tint) = cards::build(style, legacy, &scalp, &colours);
            draws[0].params.p1[0] = tint;
            let sa = hair_sigma_a(g.hair_dark, g.red);
            for mut h in hair {
                if h.params.kind == kind::HAIR_CARD {
                    h.params.kind = kind::HAIR_CARD2;
                    h.params.p1 = [sa.x, sa.y, sa.z, 0.0];
                }
                draws.push(h);
            }
        }
        draws
    }

    fn eye_draws(&self, head: &GnmHead, colours: &Colours, age: &AgeState) -> Vec<Draw> {
        let mut out = Vec::new();
        let r = &self.regions;
        for (e, side) in ["left_eye", "right_eye"].iter().enumerate() {
            let c = head.eyes[e];
            let ids: Vec<usize> = (0..self.verts.len()).filter(|&v| self.has(v, side)).collect();
            let interior: Vec<usize> = ids.iter().copied().filter(|&v| self.has(v, "eye_interiors")).collect();
            let exterior: Vec<usize> = ids.iter().copied().filter(|&v| self.has(v, "eye_exteriors")).collect();
            let irises: Vec<usize> = interior.iter().copied().filter(|&v| self.has(v, "irises")).collect();
            let iris_c = irises.iter().map(|&v| head.positions[v]).sum::<Vec3>() / irises.len() as f32;
            let fwd = (iris_c - c).normalize();
            let iris_r = irises
                .iter()
                .map(|&v| {
                    let d = head.positions[v] - c;
                    (d - fwd * d.dot(fwd)).length()
                })
                .fold(0.0, f32::max);
            let sub = |set: &[usize], f: &dyn Fn(usize) -> Vertex| -> (Vec<Vertex>, Vec<u32>) {
                let mut map = std::collections::HashMap::new();
                let verts: Vec<Vertex> = set.iter().enumerate().map(|(k, &v)| {
                    map.insert(v as u32, k as u32);
                    f(v)
                }).collect();
                let idx = self
                    .faces
                    .iter()
                    .filter(|t| t.iter().all(|i| map.contains_key(i)))
                    .flat_map(|t| t.map(|i| map[&i]))
                    .collect();
                (verts, idx)
            };
            let (vi, ii) = sub(&interior, &|v| Vertex {
                pos: head.positions[v].to_array(),
                nrm: head.normals[v].to_array(),
                aux: [
                    self.has(v, "scleras") as u8 as f32,
                    self.has(v, "irises") as u8 as f32,
                    self.has(v, "pupils") as u8 as f32,
                    0.0,
                ],
                ..Default::default()
            });
            let limbal = 0.15 + 0.5 * (1.0 - smoothstep(20.0, 60.0, age.years));
            out.push(Draw {
                vertices: vi,
                indices: ii,
                params: DrawParams {
                    kind: kind::EYE2,
                    colour: [limbal, smoothstep(35.0, 75.0, age.years), 0.36, 0.0],
                    colour2: colours.iris.extend(1.0).to_array(),
                    p0: [fwd.x, fwd.y, fwd.z, 0.028],
                    p1: [c.x, c.y, c.z, iris_r],
                    ..Default::default()
                },
                texture: None,
            });
            let (vc, ic) = sub(&exterior, &|v| Vertex {
                pos: head.positions[v].to_array(),
                nrm: head.normals[v].to_array(),
                ..Default::default()
            });
            out.push(Draw { vertices: vc, indices: ic, params: DrawParams { kind: kind::CORNEA, ..Default::default() }, texture: None });
            // Occlusion shell: the cornea pushed out 0.3 mm. The shader
            // computes the lid shadow per fragment from eye-local x, y and
            // cubic fits of the upper and lower lid margins.
            let base_c = self.eyes_t[e];
            let (vo, io) = sub(&exterior, &|v| {
                let b = self.verts[v] - base_c;
                let n = Vec3::from(head.normals[v]);
                Vertex {
                    pos: (head.positions[v] + n * 0.003).to_array(),
                    nrm: n.to_array(),
                    aux: [b.x, b.y, b.z, 0.0],
                    ..Default::default()
                }
            });
            let (x0, x1, up, lo) = &r.lid_fit[e];
            out.push(Draw {
                vertices: vo,
                indices: io,
                params: DrawParams {
                    kind: kind::OCCLUSION,
                    colour: [*x0, *x1, 0.0, 0.0],
                    colour2: *up,
                    p0: [1.0, 0.0, 0.0, 0.0],
                    p1: *lo,
                    ..Default::default()
                },
                texture: None,
            });
        }
        out
    }

    /// Hair density at a template point, from a hairline profile around the
    /// head (height above the eyes by azimuth), the genome's hairline height
    /// and the age's recession.
    fn scalp_density(&self, b: Vec3, g: &Genome2, age: &AgeState) -> f32 {
        let ey = crate::head::EYE_ANCHOR.y;
        let az = b.x.atan2(b.z - 0.35).abs();
        // (azimuth, hairline height over the eye line) around the head.
        const PROFILE: [(f32, f32); 8] = [(0.0, 0.72), (0.55, 0.66), (0.95, 0.42), (1.3, 0.08), (1.6, 0.14), (2.1, -0.2), (2.6, -0.62), (3.15, -0.78)];
        let mut line = PROFILE[PROFILE.len() - 1].1;
        for w in PROFILE.windows(2) {
            if az >= w[0].0 && az <= w[1].0 {
                let t = (az - w[0].0) / (w[1].0 - w[0].0);
                line = w[0].1 + (w[1].1 - w[0].1) * t;
                break;
            }
        }
        let front = 1.0 - smoothstep(0.5, 1.2, az);
        let temple = smoothstep(0.3, 0.6, az) * (1.0 - smoothstep(0.8, 1.1, az));
        line += ey + front * (g.hairline_height * 0.05 + 0.3 * age.recession) + temple * 0.2 * age.recession;
        let mut d = smoothstep(line - 0.03, line + 0.05, b.y);
        let crown_y = self.verts[..self.regions.skin_n].iter().map(|p| p.y).fold(f32::MIN, f32::max) - 0.1;
        let crown = Vec3::new(0.0, crown_y, 0.15);
        let rec = age.recession;
        let rr = (b - crown).length();
        d *= 1.0 - smoothstep(0.55, 1.0, rec) * (1.0 - smoothstep(0.15 + 0.35 * rec, 0.35 + 0.4 * rec, rr));
        d
    }

    /// The 13 linear measures of src/measure.rs where GNM landmarks exist,
    /// in mm, on posed positions.
    pub fn measures(&self, pos: &[Vec3]) -> Vec<(&'static str, f32)> {
        let p = |k: &str| pos[self.picks[k].as_u64().unwrap() as usize];
        let dx = |a: &str, b: &str| (p(a).x - p(b).x).abs() * 100.0;
        let dy = |a: &str, b: &str| (p(a).y - p(b).y).abs() * 100.0;
        vec![
            ("en-en", dx("en_l", "en_r")),
            ("ex-ex", dx("ex_l", "ex_r")),
            ("al-al", dx("al_l", "al_r")),
            ("ch-ch", dx("ch_l", "ch_r")),
            ("zy-zy", dx("zy_l", "zy_r")),
            ("n-gn", dy("n", "gn")),
            ("n-sn", dy("n", "sn")),
            ("sn-gn", dy("sn", "gn")),
        ]
    }
}

/// Lash line, tear meniscus and caruncle weights for a skin point near an
/// eye (x, y relative to the eye centre; z depth relative to it).
fn lid_masks(lid: &[Vec2], q: Vec2, z: f32, e: usize) -> (f32, f32, f32) {
    if z < 0.05 || q.length() > 0.3 {
        return (0.0, 0.0, 0.0);
    }
    let d = signed_dist(lid, q).abs();
    let near = 1.0 - smoothstep(0.004, 0.014, d);
    let upper = smoothstep(-0.02, 0.01, q.y);
    // Inner corner: iBUG 39 (right eye) / 42 (left eye) is the medial point.
    let inner = if e == 0 { lid[0] } else { lid[3] };
    let inner = if inner.x.abs() < lid[3].x.abs().min(lid[0].x.abs()) + 1e-4 { inner } else { lid[3] };
    let car = 1.0 - smoothstep(0.008, 0.025, (q - inner).length());
    (near * upper, near * (1.0 - upper) * 0.8, car)
}

/// Signed distance to a closed polygon (negative inside).
fn signed_dist(poly: &[Vec2], q: Vec2) -> f32 {
    let mut d = f32::MAX;
    let mut inside = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let ab = b - a;
        let t = ((q - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
        d = d.min((q - a - ab * t).length());
        if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    if inside { -d } else { d }
}

/// Coefficients (a, b, c, d) of the cubic a + bt + ct^2 + dt^3 through four
/// points (Lagrange, expanded).
fn cubic_through(p: [(f32, f32); 4]) -> [f32; 4] {
    let mut c = [0.0f32; 4];
    for i in 0..4 {
        let mut poly = [1.0f32, 0.0, 0.0, 0.0];
        let mut den = 1.0;
        for j in 0..4 {
            if j == i {
                continue;
            }
            let mut next = [0.0f32; 4];
            for k in 0..3 {
                next[k + 1] += poly[k];
                next[k] -= poly[k] * p[j].0;
            }
            poly = next;
            den *= p[i].0 - p[j].0;
        }
        for k in 0..4 {
            c[k] += p[i].1 * poly[k] / den;
        }
    }
    c
}

/// Smooth closed curve through the six lid landmarks (Catmull-Rom).
fn lid_curve(pts: &[Vec2; 6], per: usize) -> Vec<Vec2> {
    let mut out = Vec::new();
    for i in 0..6 {
        let (p0, p1, p2, p3) = (pts[(i + 5) % 6], pts[i], pts[(i + 1) % 6], pts[(i + 2) % 6]);
        for s in 0..per {
            let t = s as f32 / per as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            out.push(0.5 * (2.0 * p1 + (p2 - p0) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3));
        }
    }
    out
}

/// Eyelashes as thin tapered ribbons along the lid margins, curling out
/// and up (upper) or down (lower). Drawn as Marschner hair cards.
fn lashes(lids: &[Vec<Vec2>; 2], head: &GnmHead, g: &Genome2) -> Vec<Draw> {
    let mut vs = Vec::new();
    let mut is = Vec::new();
    let mut rng = crate::rng::Rng::fork(g.seed, "lashes");
    for e in 0..2 {
        let c = head.eyes[e];
        let lid = &lids[e];
        let n = lid.len();
        // Upper arc: points with y above the corners' mean.
        for k in 0..n {
            let a = lid[k];
            let b = lid[(k + 1) % n];
            let upper = a.y > -0.02;
            let count = if upper { 5 } else { 2 };
            for j in 0..count {
                let t = (j as f32 + rng.f32()) / count as f32;
                let q = a.lerp(b, t);
                // Root on the eyeball-front surface at the lid margin.
                let zr = (0.1535f32.powi(2) - q.length_squared()).max(0.0).sqrt();
                let root = c + Vec3::new(q.x, q.y, zr);
                let across = ((q.x.abs() - 0.02) / 0.12).clamp(0.0, 1.0);
                let len = if upper { 0.065 + 0.03 * (std::f32::consts::PI * (across * 0.8 + 0.1)).sin() } else { 0.03 } * (0.75 + 0.5 * rng.f32());
                let up = if upper { 1.0 } else { -1.0 };
                let out_dir = Vec3::new(q.x * 1.5, up * 0.3, 1.0).normalize();
                let curl = Vec3::new(0.0, up * 0.9, -0.3);
                let tang = Vec3::new(b.x - a.x, b.y - a.y, 0.0).normalize_or_zero();
                let segs = 4;
                let first = vs.len() as u32;
                let mut p = root;
                for s in 0..=segs {
                    let u = s as f32 / segs as f32;
                    let dir = (out_dir + curl * u * 1.1).normalize();
                    let w = 0.0011 * (1.0 - 0.7 * u);
                    let nrm = dir.cross(tang).normalize_or_zero();
                    for sg in [-1.0f32, 1.0] {
                        vs.push(Vertex {
                            pos: (p + tang * w * sg).to_array(),
                            nrm: nrm.to_array(),
                            uv: [0.5, 0.2 + 0.8 * u],
                            aux: dir.extend(0.0).to_array(),
                            ..Default::default()
                        });
                    }
                    p += dir * len / segs as f32;
                }
                for s in 0..segs as u32 {
                    let a0 = first + s * 2;
                    is.extend_from_slice(&[a0, a0 + 1, a0 + 2, a0 + 1, a0 + 3, a0 + 2]);
                }
            }
        }
    }
    let sa = hair_sigma_a(g.hair_dark.max(0.88), g.red * 0.3);
    vec![Draw {
        vertices: vs,
        indices: is,
        params: DrawParams { kind: kind::HAIR_CARD2, p0: [0.3, 0.0, 0.0, 0.0], p1: [sa.x, sa.y, sa.z, 0.0], ..Default::default() },
        texture: None,
    }]
}

/// Hair absorption from melanin (pbrt-v3 conversion, BSD-2):
/// sigma_a = ce * (0.419, 0.697, 1.37) + cp * (0.187, 0.4, 1.05).
pub fn hair_sigma_a(dark: f32, red: f32) -> Vec3 {
    let ce = (0.45 + 8.0 * dark.powf(2.5)) * (1.0 - 0.7 * red);
    let cp = red * 2.5 + ce * 0.05;
    Vec3::new(0.419, 0.697, 1.37) * ce + Vec3::new(0.187, 0.4, 1.05) * cp
}

/// Signed per-vertex curvature (1/dm): mean of dot(dn, dp) / |dp|^2 over
/// edges. Positive = convex.
pub fn curvature(pos: &[Vec3], nrm: &[Vec3], tris: &[[u32; 3]]) -> Vec<f32> {
    let n = pos.len();
    let mut acc = vec![0.0f32; n];
    let mut cnt = vec![0.0f32; n];
    for t in tris {
        for k in 0..3 {
            let (a, b) = (t[k] as usize, t[(k + 1) % 3] as usize);
            let dp = pos[b] - pos[a];
            let l2 = dp.length_squared().max(1e-10);
            let kk = (nrm[b] - nrm[a]).dot(dp) / l2;
            acc[a] += kk;
            acc[b] += kk;
            cnt[a] += 1.0;
            cnt[b] += 1.0;
        }
    }
    (0..n).map(|i| if cnt[i] > 0.0 { acc[i] / cnt[i] } else { 0.0 }).collect()
}

/// Pre-integrated skin diffuse (Penner 2011): for N.L (u) and curvature
/// (v = 5 mm / radius), the ring integral of the clamped cosine weighted by
/// d'Eon and Luebke's six-Gaussian skin diffusion profile.
pub fn preintegrated_lut() -> (u32, u32, Vec<u8>) {
    const VARS: [f32; 6] = [0.0064, 0.0484, 0.187, 0.567, 1.99, 7.41];
    const W: [[f32; 3]; 6] = [
        [0.233, 0.455, 0.649],
        [0.100, 0.336, 0.344],
        [0.118, 0.198, 0.0],
        [0.113, 0.007, 0.007],
        [0.358, 0.004, 0.0],
        [0.078, 0.0, 0.0],
    ];
    let (w, h) = (128u32, 64u32);
    let mut px = vec![255u8; (w * h * 4) as usize];
    for y in 0..h {
        let v = (y as f32 + 0.5) / h as f32;
        let radius = 5.0 / v.max(0.02); // mm
        for x in 0..w {
            let ndl = (x as f32 + 0.5) / w as f32 * 2.0 - 1.0;
            let theta = ndl.clamp(-1.0, 1.0).acos();
            let mut num = [0.0f32; 3];
            let mut den = [0.0f32; 3];
            let steps = 200;
            for s in 0..steps {
                let a = -std::f32::consts::PI + (s as f32 + 0.5) / steps as f32 * std::f32::consts::TAU;
                let dist = 2.0 * radius * (a * 0.5).sin().abs();
                let cosv = (theta + a).cos().max(0.0);
                for c in 0..3 {
                    let r: f32 = (0..6).map(|k| W[k][c] * (-dist * dist / (2.0 * VARS[k])).exp() / (std::f32::consts::TAU * VARS[k])).sum();
                    num[c] += cosv * r;
                    den[c] += r;
                }
            }
            let o = ((y * w + x) * 4) as usize;
            for c in 0..3 {
                px[o + c] = ((num[c] / den[c].max(1e-9)).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }
    }
    (w, h, px)
}

impl Regions {
    fn empty() -> Self {
        Self {
            skin_n: 0,
            jowl: vec![],
            neck: vec![],
            eyebag: vec![],
            nasolabial: vec![],
            cheek: vec![],
            lips: vec![],
            lip_soft: vec![],
            nose: vec![],
            ears: vec![],
            forehead: vec![],
            hb: vec![],
            oil: vec![],
            sto_y: 0.0,
            nose_root: Vec3::ZERO,
            ear_c: [Vec3::ZERO; 2],
            chin_y: 0.0,
            bottom_y: 0.0,
            lids: [vec![], vec![]],
            lid_fit: [(0.0, 0.0, [0.0; 4], [0.0; 4]); 2],
        }
    }

    fn new(a: &GnmAsset) -> Self {
        let v = &a.verts;
        let n = a.set("skin").len();
        let mean = |ids: &[usize]| ids.iter().map(|&i| v[i]).sum::<Vec3>() / ids.len().max(1) as f32;
        let tris: Vec<[u32; 3]> = a.faces.iter().filter(|t| t.iter().all(|&i| (i as usize) < n)).copied().collect();
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
        let mask = |g: &[&str]| -> Vec<f32> { (0..n).map(|i| if g.iter().any(|m| a.has(i, m)) { 1.0 } else { 0.0 }).collect() };
        let lips_set: Vec<usize> = (0..n).filter(|&i| a.has(i, "upper_lip") || a.has(i, "lower_lip")).collect();
        let lipr: Vec<usize> = (0..n).filter(|&i| a.has(i, "upper_lip_region") || a.has(i, "lower_lip_region")).collect();
        let nose: Vec<usize> = (0..n).filter(|&i| a.has(i, "nose_region")).collect();
        let sto = a.landmark(v, 62).lerp(a.landmark(v, 66), 0.5);
        let lip_w = a.landmark(v, 54).x.abs();
        let nose_top = nose.iter().map(|&i| v[i].y).fold(f32::MIN, f32::max);
        let nose_root = Vec3::new(0.0, nose_top, mean(&nose).z);
        let chin_y = a.landmark(v, 8).y;
        let bottom_y = (0..n).map(|i| v[i].y).fold(f32::MAX, f32::min);
        let ears_l: Vec<usize> = (0..n).filter(|&i| a.has(i, "ears") && v[i].x > 0.0).collect();
        let ears_r: Vec<usize> = (0..n).filter(|&i| a.has(i, "ears") && v[i].x < 0.0).collect();
        let al_x = a.landmark(v, 35).x.abs();
        let nose_base = a.landmark(v, 33).y;
        let eye = a.eyes_t;
        let mut r = Self::empty();
        r.skin_n = n;
        r.sto_y = sto.y;
        r.nose_root = nose_root;
        r.ear_c = [mean(&ears_l), mean(&ears_r)];
        r.chin_y = chin_y;
        r.bottom_y = bottom_y;
        for (e, pts) in [(0usize, [42, 43, 44, 45, 46, 47]), (1, [39, 38, 37, 36, 41, 40])] {
            let ps: Vec<Vec2> = pts.iter().map(|&k| {
                let p = a.landmark(v, k) - eye[e];
                Vec2::new(p.x, p.y)
            }).collect();
            r.lids[e] = lid_curve(&[ps[0], ps[1], ps[2], ps[3], ps[4], ps[5]], 8);
            // Corners are ps[0] (inner) and ps[3] (outer); upper ps[1], ps[2];
            // lower ps[4], ps[5].
            let (x0, x1) = (ps[0].x, ps[3].x);
            let t = |p: Vec2| (p.x - x0) / (x1 - x0);
            let up = cubic_through([(0.0, ps[0].y), (t(ps[1]), ps[1].y), (t(ps[2]), ps[2].y), (1.0, ps[3].y)]);
            let lo = cubic_through([(0.0, ps[0].y), (t(ps[5]), ps[5].y), (t(ps[4]), ps[4].y), (1.0, ps[3].y)]);
            r.lid_fit[e] = (x0, x1, up, lo);
        }
        r.lips = (0..n).map(|i| if lipr.contains(&i) { 1.0 } else { 0.0 }).collect();
        let lip_core: Vec<f32> = (0..n).map(|i| if lips_set.contains(&i) { 1.0 } else { 0.0 }).collect();
        r.lip_soft = feather(lip_core, 2).iter().map(|x| smoothstep(0.3, 0.8, *x)).collect();
        r.nose = mask(&["nose_region"]);
        r.ears = mask(&["ears"]);
        r.forehead = feather(mask(&["forehead_region", "middle_brow_region"]), 4);
        r.hb = feather(
            (0..n)
                .map(|i| {
                    let cheek = a.has(i, "left_cheek_region") || a.has(i, "right_cheek_region") || a.has(i, "left_zygomatic_region") || a.has(i, "right_zygomatic_region");
                    let tip = a.has(i, "nose_region") && v[i].y < nose_base + 0.12;
                    0.8 * cheek as u8 as f32 + 0.7 * tip as u8 as f32 + 0.8 * a.has(i, "ears") as u8 as f32 + 0.3 * a.has(i, "chin_region") as u8 as f32
                })
                .collect(),
            8,
        );
        r.oil = feather(mask(&["forehead_region", "nose_region", "chin_region", "middle_brow_region"]), 6);
        for i in 0..n {
            let p = v[i];
            let face = a.has(i, "hockey_mask");
            let ax = p.x.abs();
            let jowl = if face {
                smoothstep(lip_w * 0.8, lip_w * 1.3, ax) * (1.0 - smoothstep(sto.y - 0.05, sto.y + 0.1, p.y)) * smoothstep(chin_y - 0.1, chin_y + 0.15, p.y)
            } else {
                0.0
            };
            let neck = if !face && p.y < chin_y && p.z > 0.3 { 1.0 - smoothstep(0.2, 0.5, ax) } else { 0.0 };
            let eyebag = (a.has(i, "left_infraorbital_region") || a.has(i, "right_infraorbital_region")) as u8 as f32;
            let _ = eye;
            let t = ((nose_base - p.y) / (nose_base - sto.y)).clamp(0.0, 1.0);
            let line_x = al_x + (lip_w - al_x) * t;
            let nasolabial = if face && p.y < nose_base + 0.03 && p.y > sto.y - 0.08 && p.z > 0.9 {
                smoothstep(line_x, line_x + 0.05, ax) * (1.0 - smoothstep(line_x + 0.1, line_x + 0.2, ax))
            } else {
                0.0
            };
            let cheek = (a.has(i, "left_cheek_region") || a.has(i, "right_cheek_region") || a.has(i, "left_zygomatic_region") || a.has(i, "right_zygomatic_region")) as u8 as f32;
            r.jowl.push(jowl);
            r.neck.push(neck);
            r.eyebag.push(eyebag);
            r.nasolabial.push(nasolabial);
            r.cheek.push(cheek);
        }
        r.jowl = feather(std::mem::take(&mut r.jowl), 12);
        r.neck = feather(std::mem::take(&mut r.neck), 8);
        r.eyebag = feather(std::mem::take(&mut r.eyebag), 6);
        r.nasolabial = feather(std::mem::take(&mut r.nasolabial), 8);
        r.cheek = feather(std::mem::take(&mut r.cheek), 10);
        r
    }
}
