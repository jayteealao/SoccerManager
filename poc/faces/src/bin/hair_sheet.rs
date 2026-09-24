//! PoC 3: four hair styles across three skin tones, on the PoC 2 head.
//!
//! Writes `out/poc3_hair_sheet.jpg` (three-quarter close-ups) and
//! `out/poc3/` full-size close-ups of the two coiled styles.

use regen_faces_poc::age::AgeState;
use regen_faces_poc::cards::Style;
use regen_faces_poc::face::FaceBuilder;
use regen_faces_poc::genome::Genome;
use regen_faces_poc::img::{contact_sheet, Rgba};
use regen_faces_poc::pools::nation;
use regen_faces_poc::render::{Camera, Renderer, SIZE};
use std::path::Path;

const STYLES: [Style; 4] = [Style::ShortStraight, Style::MediumWavy, Style::TightFade, Style::Twists];

fn main() {
    let r = Renderer::new();
    let mut fb = FaceBuilder::new().expect("MakeHuman assets (run scripts/fetch_makehuman.sh)");
    // One identity; only skin tone changes per row, so the hair is the variable.
    let base = Genome::generate(4, nation("CVD").unwrap(), 2004);
    let tones = [(0.12, "LIGHT"), (0.45, "MEDIUM"), (0.85, "DARK")];
    let age = 26.0;
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    for (tone, name) in tones {
        let mut g = base.clone();
        g.skin_tone = tone;
        g.hair_colour.dark = 0.9;
        labels.push(format!("SKIN {name}\nTONE {tone}"));
        let st = AgeState::new(&g, age);
        let mut row = Vec::new();
        for style in STYLES {
            let draws = fb.portrait(&g, &st, Some(style));
            let img = Rgba::from_raw(SIZE, SIZE, r.render(&draws, &Camera::hair_closeup(35.0)).colour);
            if name == "DARK" && matches!(style, Style::TightFade | Style::Twists) {
                let stem = style.label().replace(' ', "_");
                img.save_png(Path::new(&format!("out/poc3/{stem}_closeup.png")));
            }
            row.push(img);
        }
        rows.push(row);
    }
    let cols: Vec<String> = STYLES.iter().map(|s| s.label().to_uppercase()).collect();
    contact_sheet(&rows, 320, &cols, &labels).save_jpeg(Path::new("out/poc3_hair_sheet.jpg"), 86);
}
