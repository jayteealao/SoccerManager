//! PoC 4: lays out before/after pairs of the photo finish.
//!
//! Reads `out/tmp/finish_inputs/<stem>.png` (render) and
//! `out/tmp/finished/<stem>.png` (finished) and writes
//! `out/poc4_finish_pairs.jpg`, plus two full-size pairs in `out/poc4/`.

use regen_faces_poc::img::{contact_sheet, Rgba};
use std::path::Path;

const FACES: [(&str, u64); 6] = [("NHV", 6), ("CVD", 4), ("CVD", 1), ("KSI", 2), ("NHV", 13), ("NHV", 20)];
const AGES: [u32; 3] = [19, 30, 60];

fn main() {
    strength_sweep();
    compare_versions();
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    for (code, seed) in FACES {
        let mut row = Vec::new();
        for age in AGES {
            let stem = format!("{code}_{seed}_age{age}");
            let before = Rgba::load_png(Path::new(&format!("out/tmp/finish_inputs/{stem}.png")));
            let after = Rgba::load_png(Path::new(&format!("out/tmp/finished/{stem}.png")));
            if (code, seed) == ("CVD", 4) || (code, seed) == ("NHV", 6) {
                let pair = contact_sheet(&[vec![before.clone(), after.clone()]], 512, &[], &[]);
                pair.save_jpeg(Path::new(&format!("out/poc4/{stem}_pair.jpg")), 88);
            }
            row.push(before);
            row.push(after);
        }
        rows.push(row);
        labels.push(format!("{code} SEED {seed}"));
    }
    let cols: Vec<String> = AGES.iter().flat_map(|a| [format!("{a} RENDER"), format!("{a} FINISH")]).collect();
    contact_sheet(&rows, 240, &cols, &labels).save_jpeg(Path::new("out/poc4_finish_pairs.jpg"), 80);
}

/// Render vs strength 0.3 / 0.2 / 0.15 on the cases that drifted most.
fn strength_sweep() {
    let cases = ["KSI_2_age30", "CVD_1_age60", "NHV_13_age60", "CVD_4_age60"];
    let dirs = ["out/tmp/finish_inputs", "out/tmp/finished", "out/tmp/finished_s20", "out/tmp/finished_s15"];
    if !Path::new("out/tmp/finished_s15").exists() {
        return;
    }
    let rows: Vec<Vec<Rgba>> = cases
        .iter()
        .map(|c| dirs.iter().map(|d| Rgba::load_png(Path::new(&format!("{d}/{c}.png")))).collect())
        .collect();
    let cols = ["RENDER", "STRENGTH 0.3", "STRENGTH 0.2", "STRENGTH 0.15"].map(String::from);
    let labels: Vec<String> = cases.iter().map(|c| c.replace('_', " ")).collect();
    contact_sheet(&rows, 300, &cols, &labels).save_jpeg(Path::new("out/poc4_strength_sweep.jpg"), 82);
}

/// v1 (earlier render, fixed prompt, strength 0.3) against v2 (tuned render,
/// age and eye-colour prompt, strength 0.2) at ages 30 and 60.
fn compare_versions() {
    if !Path::new("out/tmp/finished_v1").exists() {
        return;
    }
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    for (code, seed) in FACES {
        let mut row = Vec::new();
        for age in [30, 60] {
            let stem = format!("{code}_{seed}_age{age}");
            for d in ["out/tmp/finish_inputs", "out/tmp/finished_v1", "out/tmp/finished"] {
                row.push(Rgba::load_png(Path::new(&format!("{d}/{stem}.png"))));
            }
        }
        rows.push(row);
        labels.push(format!("{code} SEED {seed}"));
    }
    let cols = ["30 RENDER", "30 V1", "30 V2", "60 RENDER", "60 V1", "60 V2"].map(String::from);
    contact_sheet(&rows, 240, &cols, &labels).save_jpeg(Path::new("out/poc4_v1_vs_v2.jpg"), 80);
}
