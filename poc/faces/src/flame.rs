//! PoC 6: FLAME 2023 Open (CC-BY-4.0, MPI-IS) carried onto the MakeHuman
//! topology by `flame/flame_transfer.py`. Two layers, both expressed as
//! pseudo-targets so they compose with MakeHuman targets and the fits:
//! - `flame/conform`: moves the base head onto FLAME's mean head;
//! - `flame/beta{i}`: FLAME shape direction i per unit beta.
//!
//! FLAME: Li, Bolkart, Black, Li, Romero, "Learning a model of facial shape
//! and expression from 4D scans", ACM ToG 2017.

use crate::genome::Genome;
use crate::rng::Rng;
use glam::Vec3;
use std::path::PathBuf;

pub struct Flame {
    pub conform: Vec<Vec3>,
    /// `dirs[k][v]`: displacement of base-mesh vertex v per unit beta k.
    pub dirs: Vec<Vec<Vec3>>,
}

fn read_vec3(path: &PathBuf) -> std::io::Result<Vec<Vec3>> {
    let bytes = std::fs::read(path)?;
    let f: &[f32] = bytemuck::cast_slice(&bytes);
    Ok(f.chunks(3).map(|c| Vec3::new(c[0], c[1], c[2])).collect())
}

impl Flame {
    pub fn load() -> std::io::Result<Self> {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("downloads/flame_mh");
        let conform = read_vec3(&dir.join("conform.f32")).map_err(|e| {
            std::io::Error::new(e.kind(), format!("{e}: run flame/flame_transfer.py first"))
        })?;
        let all = read_vec3(&dir.join("dirs.f32"))?;
        let n = conform.len();
        let dirs = all.chunks(n).map(|c| c.to_vec()).collect();
        Ok(Self { conform, dirs })
    }

    /// Identity betas for a genome: standard normal per component, clamped,
    /// from the genome seed (its own stream, so other genes are unaffected).
    pub fn betas(&self, g: &Genome) -> Vec<f32> {
        let mut r = Rng::fork(g.seed, "flame-beta");
        self.dirs.iter().map(|_| r.normal().clamp(-2.5, 2.5)).collect()
    }
}
