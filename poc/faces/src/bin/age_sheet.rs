//! PoC 2: six genomes aged 16 to 70, rendered off-screen with wgpu.
//!
//! Writes `out/poc2_age_sheet.jpg` (the contact sheet), a few full-size PNGs
//! in `out/poc2/`, and every render plus its depth map in `out/tmp/poc2/`.
//! With `--finish-inputs`, also writes colour/depth pairs for PoC 4 into
//! `out/tmp/finish_inputs/`.

use regen_faces_poc::age::{AgeState, SHEET_AGES};
use regen_faces_poc::cards::Style;
use regen_faces_poc::face::FaceBuilder;
use regen_faces_poc::genome::Genome;
use regen_faces_poc::img::{contact_sheet, Rgba};
use regen_faces_poc::pools::nation;
use regen_faces_poc::render::{Camera, Renderer, SIZE};
use std::path::Path;
use std::time::Instant;

/// Six sample genomes chosen for spread: ancestry, hair type, fat tendency.
pub const SHEET: [(&str, u64); 6] = [("NHV", 6), ("CVD", 4), ("CVD", 1), ("KSI", 2), ("NHV", 13), ("NHV", 20)];

fn main() {
    let finish = std::env::args().any(|a| a == "--finish-inputs");
    let r = Renderer::new();
    eprintln!("adapter: {}", r.adapter_info);
    let mut fb = FaceBuilder::new().expect("MakeHuman assets (run scripts/fetch_makehuman.sh)");
    let cam = Camera::portrait(12.0);
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    let mut times = Vec::new();
    // PoC 4 manifest: render, depth, output, prompt set (age band + eye colour).
    let mut manifest = String::new();
    for (code, seed) in SHEET {
        let g = Genome::generate(seed, nation(code).unwrap(), 2004);
        let style = Style::for_genome(&g);
        let anc: Vec<String> = g.ancestry.iter().map(|a| format!("{:?}{:.2}", a.pool, a.share)).collect();
        labels.push(format!("{code} SEED {seed}\n{}\n{}", anc.join("+"), style.label()));
        let mut row = Vec::new();
        for years in SHEET_AGES {
            let age = AgeState::new(&g, years);
            let t = Instant::now();
            let draws = fb.portrait(&g, &age, Some(style));
            let f = r.render(&draws, &cam);
            times.push(t.elapsed().as_secs_f32());
            let img = Rgba::from_raw(SIZE, SIZE, f.colour);
            let depth = Rgba::from_raw(SIZE, SIZE, f.depth);
            let stem = format!("{code}_{seed}_age{}", years as u32);
            img.save_png(Path::new(&format!("out/tmp/poc2/{stem}.png")));
            depth.save_png(Path::new(&format!("out/tmp/poc2/{stem}_depth.png")));
            if finish && [19.0, 30.0, 60.0].contains(&years) {
                img.save_png(Path::new(&format!("out/tmp/finish_inputs/{stem}.png")));
                depth.save_png(Path::new(&format!("out/tmp/finish_inputs/{stem}_depth.png")));
                let eyes = format!("{:?}", g.eye_colour).to_lowercase();
                manifest += &format!(
                    "out/tmp/finish_inputs/{stem}.png,out/tmp/finish_inputs/{stem}_depth.png,out/tmp/finished/{stem}.png,embeds_age{}_{eyes}\n",
                    years as u32
                );
            }
            row.push(img);
        }
        rows.push(row);
    }
    if finish {
        std::fs::write("out/tmp/pairs.csv", manifest).unwrap();
    }
    let cols: Vec<String> = SHEET_AGES.iter().map(|a| format!("AGE {a}")).collect();
    let sheet = contact_sheet(&rows, 240, &cols, &labels);
    sheet.save_jpeg(Path::new("out/poc2_age_sheet.jpg"), 80);
    let n = times.len() as f32;
    eprintln!(
        "{} renders (build + draw + readback), mean {:.3}s, max {:.3}s",
        times.len(),
        times.iter().sum::<f32>() / n,
        times.iter().cloned().fold(0.0, f32::max)
    );
}
