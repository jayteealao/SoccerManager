//! PoC 7: FLAME 2023 Open on its own (no MakeHuman geometry).
//!
//! `flame7_sheet identity`  clay: FLAME mean, then six genomes at 24; and the
//!                          full look for the same six.
//! `flame7_sheet ages`      six genomes x eight ages, plus photo-finish inputs.
//! `flame7_sheet hair`      four hair styles x three skin tones.
//!
//! Needs `flame/flame_prep.py` to have run (downloads/flame_asset/).

use regen_faces_poc::age::{AgeState, SHEET_AGES};
use regen_faces_poc::cards::Style;
use regen_faces_poc::flamehead::FlameAsset;
use regen_faces_poc::genome::Genome;
use regen_faces_poc::img::{contact_sheet, Rgba};
use regen_faces_poc::pools::nation;
use regen_faces_poc::render::{Camera, Draw, Renderer, SIZE};
use std::path::Path;

const SHEET: [(&str, u64); 6] = [("NHV", 6), ("CVD", 4), ("CVD", 1), ("KSI", 2), ("NHV", 13), ("NHV", 20)];
const DIR: &str = "out/tmp/flame7";

fn clay(mut draws: Vec<Draw>) -> Vec<Draw> {
    draws[0].params.colour = [0.32, 0.32, 0.32, 1.0];
    draws[0].params.p0 = [0.0; 4];
    for v in &mut draws[0].vertices {
        v.aux2 = [0.0; 4];
        v.aux[0] = 0.0;
    }
    draws
}

fn identity(a: &FlameAsset, r: &Renderer) {
    let cam = Camera::portrait_flame(0.0);
    let mut clay_row = Vec::new();
    let mut full_row = Vec::new();
    let mut labels = Vec::new();
    for (code, seed) in SHEET {
        let g = Genome::generate(seed, nation(code).unwrap(), 2004);
        let age = AgeState::new(&g, 24.0);
        clay_row.push(Rgba::from_raw(SIZE, SIZE, r.render(&clay(a.portrait(&g, &age, None)), &cam).colour));
        full_row.push(Rgba::from_raw(SIZE, SIZE, r.render(&a.portrait(&g, &age, Some(Style::for_genome(&g))), &cam).colour));
        labels.push(format!("{code} {seed}"));
    }
    contact_sheet(&[clay_row, full_row], 260, &labels, &["CLAY".into(), "FULL".into()])
        .save_jpeg(Path::new("out/poc7_flame_identity.jpg"), 82);
}

fn ages(a: &FlameAsset, r: &Renderer) {
    let cam = Camera::portrait_flame(12.0);
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    let mut manifest = String::new();
    for (code, seed) in SHEET {
        let g = Genome::generate(seed, nation(code).unwrap(), 2004);
        let style = Style::for_genome(&g);
        labels.push(format!("{code} SEED {seed}\n{}", style.label()));
        let mut row = Vec::new();
        for years in SHEET_AGES {
            let age = AgeState::new(&g, years);
            let f = r.render(&a.portrait(&g, &age, Some(style)), &cam);
            let img = Rgba::from_raw(SIZE, SIZE, f.colour);
            let id = format!("{code}_{seed}_age{}", years as u32);
            img.save_png(Path::new(&format!("{DIR}/renders/{id}.png")));
            if [19.0, 30.0, 60.0].contains(&years) {
                img.save_png(Path::new(&format!("{DIR}/finish_inputs/{id}.png")));
                Rgba::from_raw(SIZE, SIZE, f.depth).save_png(Path::new(&format!("{DIR}/finish_inputs/{id}_depth.png")));
                let eyes = format!("{:?}", g.eye_colour).to_lowercase();
                manifest += &format!(
                    "{DIR}/finish_inputs/{id}.png,{DIR}/finish_inputs/{id}_depth.png,{DIR}/finished/{id}.png,embeds_age{}_{eyes}\n",
                    years as u32
                );
            }
            row.push(img);
        }
        rows.push(row);
    }
    std::fs::write(format!("{DIR}/pairs.csv"), manifest).unwrap();
    let cols: Vec<String> = SHEET_AGES.iter().map(|a| format!("AGE {a}")).collect();
    contact_sheet(&rows, 240, &cols, &labels).save_jpeg(Path::new("out/poc7_flame_age_sheet.jpg"), 80);
}

fn hair(a: &FlameAsset, r: &Renderer) {
    let styles = [Style::ShortStraight, Style::MediumWavy, Style::TightFade, Style::Twists];
    let base = Genome::generate(4, nation("CVD").unwrap(), 2004);
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    for (tone, name) in [(0.12, "LIGHT"), (0.45, "MEDIUM"), (0.85, "DARK")] {
        let mut g = base.clone();
        g.skin_tone = tone;
        g.hair_colour.dark = 0.9;
        let age = AgeState::new(&g, 26.0);
        rows.push(
            styles
                .iter()
                .map(|s| Rgba::from_raw(SIZE, SIZE, r.render(&a.portrait(&g, &age, Some(*s)), &Camera::hair_closeup(35.0)).colour))
                .collect(),
        );
        labels.push(format!("SKIN {name}"));
    }
    let cols: Vec<String> = styles.iter().map(|s| s.label().to_uppercase()).collect();
    contact_sheet(&rows, 320, &cols, &labels).save_jpeg(Path::new("out/poc7_flame_hair_sheet.jpg"), 84);
}

/// Render and finish pairs for 6 faces at 19, 30 and 60.
fn compare() {
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    for (code, seed) in SHEET {
        let mut row = Vec::new();
        for years in [19, 30, 60] {
            let id = format!("{code}_{seed}_age{years}");
            row.push(Rgba::load_png(Path::new(&format!("{DIR}/finish_inputs/{id}.png"))));
            row.push(Rgba::load_png(Path::new(&format!("{DIR}/finished/{id}.png"))));
        }
        rows.push(row);
        labels.push(format!("{code} SEED {seed}"));
    }
    let cols: Vec<String> = [19, 30, 60].iter().flat_map(|a| [format!("{a} RENDER"), format!("{a} FINISH")]).collect();
    contact_sheet(&rows, 240, &cols, &labels).save_jpeg(Path::new("out/poc7_finish_pairs.jpg"), 80);
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("compare") {
        return compare();
    }
    let a = FlameAsset::load();
    let r = Renderer::new();
    match std::env::args().nth(1).as_deref() {
        Some("identity") => identity(&a, &r),
        Some("ages") => ages(&a, &r),
        Some("hair") => hair(&a, &r),
        _ => eprintln!("usage: flame7_sheet identity | ages | hair | compare"),
    }
}
