//! Loads the MakeHuman CC0 base mesh, cuts out the head, and composes
//! MakeHuman targets into a morphed head from a genome and an age.

use crate::age::AgeState;
use crate::genome::Genome;
use glam::Vec3;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Faces whose vertices all sit above this height (decimetres, base mesh
/// space) belong to the head-and-neck cut.
const NECK_CUT_Y: f32 = 4.75;

pub fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("downloads/makehuman")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Skin,
    EyeL,
    EyeR,
}

/// A sparse MakeHuman target: vertex index and offset.
pub type Target = Vec<(u32, Vec3)>;

/// Midpoint between the eyes on the base mesh; every head is moved here.
pub const EYE_ANCHOR: Vec3 = Vec3::new(0.0, 7.284, 1.245);

pub struct BaseHead {
    /// All base mesh vertices (targets index into this array).
    pub base: Vec<Vec3>,
    /// Triangles of the head cut, as global vertex indices, with their part.
    pub tris: Vec<([u32; 3], Part)>,
    targets: HashMap<String, Target>,
    dir: PathBuf,
}

/// A morphed head, compacted to only the vertices the cut uses.
pub struct HeadMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub parts: Vec<Part>,
    /// Map from global base-mesh index to compact index.
    pub remap: HashMap<u32, u32>,
}

impl BaseHead {
    pub fn load() -> std::io::Result<Self> {
        let dir = assets_dir();
        let obj = std::fs::read_to_string(dir.join("3dobjs/base.obj")).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("{e}: run scripts/fetch_makehuman.sh first ({})", dir.display()),
            )
        })?;
        let mut base = Vec::new();
        let mut tris = Vec::new();
        let mut part: Option<Part> = None;
        for line in obj.lines() {
            let mut it = line.split_whitespace();
            match it.next() {
                Some("v") => {
                    let v: Vec<f32> = it.take(3).map(|t| t.parse().unwrap()).collect();
                    base.push(Vec3::new(v[0], v[1], v[2]));
                }
                Some("g") => {
                    part = match it.next() {
                        Some("body") => Some(Part::Skin),
                        Some("helper-l-eye") => Some(Part::EyeL),
                        Some("helper-r-eye") => Some(Part::EyeR),
                        _ => None,
                    }
                }
                Some("f") => {
                    let Some(p) = part else { continue };
                    let idx: Vec<u32> = it
                        .map(|t| t.split('/').next().unwrap().parse::<u32>().unwrap() - 1)
                        .collect();
                    for k in 1..idx.len() - 1 {
                        tris.push(([idx[0], idx[k], idx[k + 1]], p));
                    }
                }
                _ => {}
            }
        }
        tris.retain(|(t, p)| *p != Part::Skin || t.iter().all(|&i| base[i as usize].y > NECK_CUT_Y));
        Ok(Self { base, tris, targets: HashMap::new(), dir })
    }

    /// Loads (and caches) a target by its path under `targets/`, without the
    /// `.target` suffix, for example `nose/nose-hump-incr`.
    pub fn target(&mut self, name: &str) -> &Target {
        if !self.targets.contains_key(name) {
            let t = load_target(&self.dir.join("targets").join(format!("{name}.target")))
                .unwrap_or_else(|e| panic!("target {name}: {e}"));
            self.targets.insert(name.to_string(), t);
        }
        &self.targets[name]
    }

    /// Applies weighted targets to the base mesh and returns the head cut.
    pub fn build(&mut self, weights: &[(String, f32)]) -> HeadMesh {
        let mut pos = self.base.clone();
        for (name, w) in weights {
            if w.abs() < 1e-4 {
                continue;
            }
            let w = *w;
            for (i, d) in self.target(name).iter() {
                pos[*i as usize] += *d * w;
            }
        }
        let mut remap = HashMap::new();
        let mut positions = Vec::new();
        let mut parts = Vec::new();
        let mut indices = Vec::with_capacity(self.tris.len() * 3);
        for (tri, part) in &self.tris {
            for &g in tri {
                let c = *remap.entry(g).or_insert_with(|| {
                    positions.push(pos[g as usize]);
                    parts.push(*part);
                    positions.len() as u32 - 1
                });
                indices.push(c);
            }
        }
        // Anchor at the eyes: macro targets change body height and would
        // otherwise move the head up and down between ages.
        let eyes: Vec<Vec3> = positions
            .iter()
            .zip(&parts)
            .filter(|(_, p)| **p != Part::Skin)
            .map(|(v, _)| *v)
            .collect();
        let centre = eyes.iter().copied().sum::<Vec3>() / eyes.len() as f32;
        let shift = EYE_ANCHOR - centre;
        for p in &mut positions {
            *p += shift;
        }
        let normals = vertex_normals(&positions, &indices);
        HeadMesh { positions, normals, indices, parts, remap }
    }

    /// The weights for a genome at an age.
    pub fn weights_for(&self, g: &Genome, age: &AgeState) -> Vec<(String, f32)> {
        let mut w: Vec<(String, f32)> = Vec::new();

        // Macro targets: ancestry mix x age class, all male.
        let races = ["african", "asian", "caucasian"];
        let ages = [("child", age.mh_child), ("young", age.mh_young), ("old", age.mh_old)];
        for (r, race) in races.iter().enumerate() {
            for (a, aw) in ages {
                w.push((format!("macrodetails/{race}-male-{a}"), g.mh_mix[r] * aw));
            }
        }
        let (mmin, mavg, mmax) = split3(age.muscle);
        let (wmin, wavg, wmax) = split3(age.weight);
        for (a, aw) in ages {
            for (m, mw) in [("minmuscle", mmin), ("averagemuscle", mavg), ("maxmuscle", mmax)] {
                for (f, fw) in [("minweight", wmin), ("averageweight", wavg), ("maxweight", wmax)] {
                    w.push((
                        format!("macrodetails/universal-male-{a}-{m}-{f}"),
                        aw * mw * fw,
                    ));
                }
            }
        }

        // Identity: signed shape axes.
        for (axis, decr, incr) in SHAPE_TARGETS {
            // Gain so identity differences read at portrait size.
            let x = (g.shape_axis(axis) * 1.35).clamp(-1.0, 1.0);
            push_signed(&mut w, decr, incr, x);
        }

        // Age rules that are not in the macro targets.
        for (name, x) in &age.extra {
            push_signed_named(&mut w, name, *x);
        }
        w
    }
}

/// Axis name -> (negative target(s), positive target(s)). `{s}` expands to
/// `l` and `r` for paired features.
const SHAPE_TARGETS: [(&str, &str, &str); 24] = [
    ("head-width", "head/head-scale-horiz-decr", "head/head-scale-horiz-incr"),
    ("head-height", "head/head-scale-vert-decr", "head/head-scale-vert-incr"),
    ("head-depth", "head/head-scale-depth-decr", "head/head-scale-depth-incr"),
    ("head-square-oval", "head/head-oval", "head/head-square"),
    ("face-fullness", "head/head-fat-decr", "head/head-fat-incr"),
    ("jaw-width", "chin/chin-width-decr", "chin/chin-width-incr"),
    ("chin-height", "chin/chin-height-decr", "chin/chin-height-incr"),
    ("chin-prominence", "chin/chin-prominent-decr", "chin/chin-prominent-incr"),
    ("jaw-bones", "chin/chin-bones-decr", "chin/chin-bones-incr"),
    ("nose-width", "nose/nose-scale-horiz-decr", "nose/nose-scale-horiz-incr"),
    ("nose-length", "nose/nose-scale-vert-decr", "nose/nose-scale-vert-incr"),
    ("nose-hump", "nose/nose-hump-decr", "nose/nose-hump-incr"),
    ("nose-tip-width", "nose/nose-point-width-decr", "nose/nose-point-width-incr"),
    ("nostril-flare", "nose/nose-flaring-decr", "nose/nose-flaring-incr"),
    ("mouth-width", "mouth/mouth-scale-horiz-decr", "mouth/mouth-scale-horiz-incr"),
    ("lower-lip", "mouth/mouth-lowerlip-volume-decr", "mouth/mouth-lowerlip-volume-incr"),
    ("upper-lip", "mouth/mouth-upperlip-volume-decr", "mouth/mouth-upperlip-volume-incr"),
    ("cheekbones", "cheek/{s}-cheek-bones-decr", "cheek/{s}-cheek-bones-incr"),
    ("eye-size", "eyes/{s}-eye-scale-decr", "eyes/{s}-eye-scale-incr"),
    ("epicanthus", "eyes/{s}-eye-epicanthus-in", "eyes/{s}-eye-epicanthus-out"),
    ("brow-height", "eyebrows/eyebrows-trans-down", "eyebrows/eyebrows-trans-up"),
    ("forehead-height", "forehead/forehead-scale-vert-decr", "forehead/forehead-scale-vert-incr"),
    ("ear-size", "ears/{s}-ear-scale-decr", "ears/{s}-ear-scale-incr"),
    ("neck-width", "neck/neck-scale-horiz-decr", "neck/neck-scale-horiz-incr"),
];

fn expand(name: &str) -> Vec<String> {
    if name.contains("{s}") {
        vec![name.replace("{s}", "l"), name.replace("{s}", "r")]
    } else {
        vec![name.to_string()]
    }
}

fn push_signed(w: &mut Vec<(String, f32)>, decr: &str, incr: &str, x: f32) {
    let (name, amount) = if x < 0.0 { (decr, -x) } else { (incr, x) };
    for n in expand(name) {
        w.push((n, amount));
    }
}

/// `name` is a target with `{s}` expansion; a negative amount is clamped to 0
/// because age rules name the side they want explicitly.
fn push_signed_named(w: &mut Vec<(String, f32)>, name: &str, x: f32) {
    for n in expand(name) {
        w.push((n, x.max(0.0)));
    }
}

/// MakeHuman's three-way split of a 0..1 macro slider around 0.5.
fn split3(x: f32) -> (f32, f32, f32) {
    if x < 0.5 {
        (1.0 - 2.0 * x, 2.0 * x, 0.0)
    } else {
        (0.0, 2.0 - 2.0 * x, 2.0 * x - 1.0)
    }
}

fn load_target(path: &Path) -> std::io::Result<Target> {
    let text = std::fs::read_to_string(path)?;
    Ok(text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let p: Vec<&str> = l.split_whitespace().collect();
            let f = |s: &str| s.parse::<f32>().unwrap();
            (p[0].parse().unwrap(), Vec3::new(f(p[1]), f(p[2]), f(p[3])))
        })
        .collect())
}

pub fn vertex_normals(pos: &[Vec3], idx: &[u32]) -> Vec<Vec3> {
    let mut n = vec![Vec3::ZERO; pos.len()];
    for t in idx.chunks(3) {
        let (a, b, c) = (pos[t[0] as usize], pos[t[1] as usize], pos[t[2] as usize]);
        let fnrm = (b - a).cross(c - a);
        for &i in t {
            n[i as usize] += fnrm;
        }
    }
    n.iter().map(|v| v.normalize_or_zero()).collect()
}
