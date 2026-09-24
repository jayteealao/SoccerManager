//! Facial anthropometry on the morphed head, for checking proportions
//! against published adult male norms.
//!
//! Landmarks are picked once on the unmorphed base mesh (by geometry) and
//! then read on any morphed head, so they follow the morph.

use crate::head::{BaseHead, HeadMesh, Part};
use glam::Vec3;

/// Adult male norms, mm. Sources: Farkas norms for North American Whites
/// (NAW) as tabulated in Wamalwa et al. 2019 (PMC6384287), which also gives
/// Kenyan and African American means; zy-zy 137 and n-gn 121.3 from Farkas.
/// SDs are the NAW SDs from the same table, except zy-zy, n-gn and go-go,
/// whose SDs (5, 6, 6) are estimates; `african` n-gn and go-go are estimates
/// too (no male value in the source) and are not used by the fit.
pub struct Norm {
    pub name: &'static str,
    pub naw: f32,
    pub african: f32,
    /// Population standard deviation (NAW), mm; weights the fit and sets
    /// how far shape genes may spread.
    pub sd: f32,
}

pub const NORMS: [Norm; 13] = [
    Norm { name: "face width zy-zy", naw: 137.0, african: 137.0, sd: 5.0 },
    Norm { name: "face height n-gn", naw: 121.3, african: 128.0, sd: 6.0 },
    Norm { name: "forehead height tr-n", naw: 67.1, african: 72.0, sd: 7.5 },
    Norm { name: "nose height n-sn", naw: 54.8, african: 51.5, sd: 3.3 },
    Norm { name: "lower face sn-me", naw: 72.6, african: 77.5, sd: 4.5 },
    Norm { name: "upper lip sn-sto", naw: 22.3, african: 25.8, sd: 2.1 },
    Norm { name: "lower lip+chin sto-me", naw: 50.3, african: 51.7, sd: 4.0 },
    Norm { name: "intercanthal en-en", naw: 33.3, african: 34.0, sd: 2.7 },
    Norm { name: "biocular ex-ex", naw: 91.2, african: 97.5, sd: 3.0 },
    Norm { name: "eye width ex-en", naw: 31.3, african: 33.5, sd: 1.3 },
    Norm { name: "nose width al-al", naw: 34.9, african: 43.6, sd: 2.1 },
    Norm { name: "mouth width ch-ch", naw: 54.5, african: 55.2, sd: 3.0 },
    Norm { name: "jaw width go-go", naw: 97.0, african: 100.0, sd: 6.0 },
];

/// Global vertex indices of the landmarks (right side = negative x).
pub struct Landmarks {
    zy: [u32; 2],
    n: u32,
    gn: u32,
    tr: u32,
    sn: u32,
    sto: u32,
    en: [u32; 2],
    ex: [u32; 2],
    al: [u32; 2],
    ch: [u32; 2],
    go: [u32; 2],
}

fn pick(base: &[Vec3], cand: &[u32], filter: impl Fn(Vec3) -> bool, key: impl Fn(Vec3) -> f32) -> u32 {
    *cand
        .iter()
        .filter(|&&i| filter(base[i as usize]))
        .max_by(|&&a, &&b| key(base[a as usize]).total_cmp(&key(base[b as usize])))
        .expect("landmark candidate")
}

impl Landmarks {
    pub fn find(bh: &mut BaseHead) -> Self {
        // Regions a MakeHuman target moves strongly, as vertex sets.
        let region = |bh: &mut BaseHead, name: &str, frac: f32| -> Vec<u32> {
            let t = bh.target(name).clone();
            let max = t.iter().map(|(_, d)| d.length()).fold(0.0, f32::max);
            t.iter().filter(|(_, d)| d.length() > frac * max).map(|(i, _)| *i).collect()
        };
        let nose_wing = region(bh, "nose/nose-flaring-incr", 0.4);
        let mut lips = region(bh, "mouth/mouth-upperlip-volume-incr", 0.15);
        lips.extend(region(bh, "mouth/mouth-lowerlip-volume-incr", 0.15));
        let b = &bh.base;
        let skin: Vec<u32> = {
            let mut v: Vec<u32> = bh.tris.iter().filter(|(_, p)| *p == Part::Skin).flat_map(|(t, _)| *t).collect();
            v.sort();
            v.dedup();
            v
        };
        let eye_verts = |part: Part| -> Vec<Vec3> {
            bh.tris.iter().filter(|(_, p)| *p == part).flat_map(|(t, _)| t.map(|i| b[i as usize])).collect()
        };
        let side = |s: f32| {
            let part = if s > 0.0 { Part::EyeL } else { Part::EyeR };
            let ev = eye_verts(part);
            let c = ev.iter().copied().sum::<Vec3>() / ev.len() as f32;
            let r = ev.iter().map(|v| v.distance(c)).fold(0.0, f32::max);
            // Lid-margin vertices: skin just in front of the eyeball.
            let lid = move |p: Vec3| {
                p.x * s > 0.0 && (p.distance(c) - r).abs() < 0.035 && p.z > c.z && (p.y - c.y).abs() < 0.03
            };
            let en = pick(b, &skin, lid, |p| -p.x.abs());
            let ex = pick(b, &skin, lid, |p| p.x.abs());
            let al = pick(b, &nose_wing, |p| p.x * s > 0.0, |p| p.x.abs());
            let ch = pick(b, &lips, |p| p.x * s > 0.0, |p| p.x.abs());
            let zy = pick(b, &skin, |p| p.x * s > 0.0 && p.y > 7.0 && p.y < 7.3 && p.z > 0.85, |p| p.x.abs());
            let go = pick(b, &skin, |p| p.x * s > 0.0 && p.y > 6.3 && p.y < 6.5 && p.z > 0.62 && p.z < 1.0, |p| p.x.abs());
            (en, ex, al, ch, zy, go)
        };
        let (l, r) = (side(1.0), side(-1.0));
        let _ = &skin;
        let mid = |p: Vec3| p.x.abs() < 0.012;
        let n = pick(b, &skin, |p| mid(p) && p.y > 7.25 && p.y < 7.5 && p.z > 1.3, |p| -p.z);
        let gn = pick(b, &skin, |p| mid(p) && p.z > 0.9 && p.y > 6.0, |p| -p.y);
        let sn = pick(b, &skin, |p| mid(p) && p.y > 6.72 && p.y < 6.86 && p.z > 1.3, |p| -p.z - (p.y - 6.8).abs());
        let sto = pick(b, &skin, |p| mid(p) && p.y > 6.55 && p.y < 6.66 && p.z > 1.3, |p| -p.z);
        let tr = pick(b, &skin, |p| mid(p) && p.z > 0.9, |p| -(p.y - 7.96).abs());
        Self {
            zy: [l.4, r.4],
            n,
            gn,
            tr,
            sn,
            sto,
            en: [l.0, r.0],
            ex: [l.1, r.1],
            al: [l.2, r.2],
            ch: [l.3, r.3],
            go: [l.5, r.5],
        }
    }

    /// All landmark vertex indices, for drawing markers.
    pub fn all(&self) -> Vec<u32> {
        let mut v = vec![self.n, self.gn, self.tr, self.sn, self.sto];
        for a in [self.zy, self.en, self.ex, self.al, self.ch, self.go] {
            v.extend(a);
        }
        v
    }

    /// Measurements in mm, in `NORMS` order. Heights are vertical (y) only,
    /// widths are x only, as calipers on a frontal face would read.
    pub fn measure(&self, head: &HeadMesh) -> [f32; 13] {
        let p = |g: u32| head.positions[head.remap[&g] as usize];
        let w = |a: [u32; 2]| (p(a[0]).x - p(a[1]).x).abs() * 100.0;
        let h = |a: u32, b: u32| (p(a).y - p(b).y).abs() * 100.0;
        [
            w(self.zy),
            h(self.n, self.gn),
            h(self.tr, self.n),
            h(self.n, self.sn),
            h(self.sn, self.gn),
            h(self.sn, self.sto),
            h(self.sto, self.gn),
            (p(self.en[0]).x - p(self.en[1]).x).abs() * 100.0,
            w(self.ex),
            (p(self.ex[0]).x - p(self.en[0]).x).abs() * 100.0,
            w(self.al),
            w(self.ch),
            w(self.go),
        ]
    }
}
