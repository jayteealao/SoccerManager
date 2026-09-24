//! PoC 6: FLAME 2023 Open on the MakeHuman topology.
//!
//! `flame_sheet ladder`  clay renders: MakeHuman (hand shape genes), +FLAME
//!                       mean form, +FLAME identity, and the full look.
//! `flame_sheet ages`    the six-genome age sheet with FLAME identity, plus
//!                       photo-finish inputs and a manifest.
//!
//! Needs `flame/flame_transfer.py` to have run (downloads/flame_mh/).

use regen_faces_poc::age::{AgeState, SHEET_AGES};
use regen_faces_poc::cards::Style;
use regen_faces_poc::face::FaceBuilder;
use regen_faces_poc::flame::Flame;
use regen_faces_poc::genome::Genome;
use regen_faces_poc::img::{contact_sheet, Rgba};
use regen_faces_poc::measure::{Landmarks, NORMS};
use regen_faces_poc::pools::nation;
use regen_faces_poc::render::{Camera, Renderer, SIZE};
use std::path::Path;

const SHEET: [(&str, u64); 6] = [("NHV", 6), ("CVD", 4), ("CVD", 1), ("KSI", 2), ("NHV", 13), ("NHV", 20)];
const DIR: &str = "out/tmp/flame";

fn clay(fb: &mut FaceBuilder, r: &Renderer, g: &Genome, age: &AgeState, w: &[(String, f32)]) -> Rgba {
    let head = fb.base.build(w);
    let mut draws = fb.portrait_from(g, age, &head, None);
    draws[0].params.colour = [0.32, 0.32, 0.32, 1.0];
    draws[0].params.p0 = [0.0; 4];
    for v in &mut draws[0].vertices {
        v.aux2[2] = 0.0;
    }
    Rgba::from_raw(SIZE, SIZE, r.render(&draws, &Camera::portrait(0.0)).colour)
}

fn ladder(fb: &mut FaceBuilder) {
    let r = Renderer::new();
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    for (code, seed) in [("NHV", 6u64), ("CVD", 4), ("KSI", 2), ("NHV", 20)] {
        let g = Genome::generate(seed, nation(code).unwrap(), 2004);
        let age = AgeState::new(&g, 24.0);
        let flame = fb.base.flame.take();
        let mh = fb.base.weights_for(&g, &age);
        let a = clay(fb, &r, &g, &age, &mh);
        fb.base.flame = flame;
        let full = fb.base.weights_for(&g, &age);
        let mean_only: Vec<_> = full.iter().filter(|(n, _)| !n.starts_with("flame/beta")).cloned().collect();
        let b = clay(fb, &r, &g, &age, &mean_only);
        let c = clay(fb, &r, &g, &age, &full);
        let head = fb.base.build(&full);
        let draws = fb.portrait_from(&g, &age, &head, Some(Style::for_genome(&g)));
        let d = Rgba::from_raw(SIZE, SIZE, r.render(&draws, &Camera::portrait(0.0)).colour);
        rows.push(vec![a, b, c, d]);
        labels.push(format!("{code} SEED {seed}"));
    }
    let cols = ["MAKEHUMAN", "+FLAME MEAN", "+FLAME IDENTITY", "FULL LOOK"].map(String::from);
    contact_sheet(&rows, 300, &cols, &labels).save_jpeg(Path::new("out/poc6_flame_ladder.jpg"), 82);
}

fn ages(fb: &mut FaceBuilder) {
    let r = Renderer::new();
    let cam = Camera::portrait(12.0);
    let lm = Landmarks::find(&mut fb.base);
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    let mut manifest = String::new();
    let mut table = format!("{:24} {:>6} {:>6}", "measure at 24 (mm)", "NAW", "AFR");
    let mut meas = Vec::new();
    for (code, seed) in SHEET {
        let g = Genome::generate(seed, nation(code).unwrap(), 2004);
        let style = Style::for_genome(&g);
        labels.push(format!("{code} SEED {seed}\n{}", style.label()));
        let mut row = Vec::new();
        for years in SHEET_AGES {
            let age = AgeState::new(&g, years);
            let head = fb.head(&g, &age);
            if years == 24.0 {
                meas.push((format!("{code}{seed}"), lm.measure(&head)));
            }
            let draws = fb.portrait_from(&g, &age, &head, Some(style));
            let f = r.render(&draws, &cam);
            let img = Rgba::from_raw(SIZE, SIZE, f.colour);
            let id = format!("{code}_{seed}_age{}", years as u32);
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
    contact_sheet(&rows, 240, &cols, &labels).save_jpeg(Path::new("out/poc6_flame_age_sheet.jpg"), 80);
    for (n, _) in &meas {
        table += &format!(" {:>7}", n);
    }
    table += "\n";
    for (k, norm) in NORMS.iter().enumerate() {
        table += &format!("{:24} {:6.1} {:6.1}", norm.name, norm.naw, norm.african);
        for (_, m) in &meas {
            table += &format!(" {:7.1}", m[k]);
        }
        table += "\n";
    }
    std::fs::write("out/poc6_measurements.txt", &table).unwrap();
    print!("{table}");
}

fn main() {
    let mut fb = FaceBuilder::new().expect("MakeHuman assets (run scripts/fetch_makehuman.sh)");
    fb.base.flame = Some(Flame::load().expect("FLAME transfer data"));
    std::fs::create_dir_all(DIR).unwrap();
    match std::env::args().nth(1).as_deref() {
        Some("ladder") => ladder(&mut fb),
        Some("closeup") => closeup(&mut fb),
        Some("check") => check(&mut fb),
        Some("compare") => compare(),
        Some("ages") => ages(&mut fb),
        _ => eprintln!("usage: flame_sheet ladder | flame_sheet ages"),
    }
}

/// Close-ups of the eye and nose region, MakeHuman vs FLAME, clay.
fn closeup(fb: &mut FaceBuilder) {
    use glam::Vec3;
    use regen_faces_poc::render::Camera;
    let r = Renderer::new();
    let g = Genome::generate(6, nation("NHV").unwrap(), 2004);
    let age = AgeState::new(&g, 24.0);
    let cams = [
        Camera { eye: Vec3::new(0.0, 7.3, 14.0), target: Vec3::new(0.0, 7.25, 1.2), fov_y_deg: 4.5 },
        Camera { eye: Vec3::new(8.0, 7.0, 11.0), target: Vec3::new(0.0, 7.0, 1.2), fov_y_deg: 6.0 },
    ];
    let mut rows = Vec::new();
    for use_flame in [false, true] {
        let flame = if use_flame { None } else { fb.base.flame.take() };
        let w = fb.base.weights_for(&g, &age);
        let head = fb.base.build(&w);
        if !use_flame { fb.base.flame = flame; }
        let mut draws = fb.portrait_from(&g, &age, &head, None);
        draws[0].params.colour = [0.32, 0.32, 0.32, 1.0];
        draws[0].params.p0 = [0.0; 4];
        if std::env::var("NOMASK").is_ok() {
            for v in &mut draws[0].vertices {
                v.aux2 = [0.0; 4];
            }
        }
        rows.push(cams.iter().map(|c| Rgba::from_raw(SIZE, SIZE, r.render(&draws, c).colour)).collect::<Vec<_>>());
    }
    contact_sheet(&rows, 400, &[], &["MAKEHUMAN".into(), "FLAME".into()]).save_jpeg(Path::new("out/tmp/flame/closeup.jpg"), 85);
}

/// Reports triangles whose orientation changes a lot when FLAME is added to
/// a fully built head (macros, calibration, age), for the sheet genomes.
fn check(fb: &mut FaceBuilder) {
    for (code, seed) in SHEET {
        let g = Genome::generate(seed, nation(code).unwrap(), 2004);
        for years in [24.0, 60.0] {
            let age = AgeState::new(&g, years);
            let mut w = fb.base.weights_for(&g, &age);
            let plain: Vec<_> = w.iter().filter(|(n, _)| !n.starts_with("flame/")).cloned().collect();
            let a = fb.base.build(&plain);
            if std::env::var("MEAN_ONLY").is_ok() {
                w.retain(|(n, _)| !n.starts_with("flame/beta"));
            }
            let b = fb.base.build(&w);
            let nrm = |p: &[glam::Vec3], t: &[u32]| {
                let (x, y, z) = (p[t[0] as usize], p[t[1] as usize], p[t[2] as usize]);
                (y - x).cross(z - x).normalize_or_zero()
            };
            let mut bad = Vec::new();
            for t in a.indices.chunks(3) {
                if a.parts[t[0] as usize] != regen_faces_poc::head::Part::Skin { continue; }
                let d = nrm(&a.positions, t).dot(nrm(&b.positions, t));
                if d < 0.5 {
                    let c = (b.positions[t[0] as usize] + b.positions[t[1] as usize] + b.positions[t[2] as usize]) / 3.0;
                    bad.push((d, c));
                }
            }
            bad.sort_by(|x, y| x.0.total_cmp(&y.0));
            println!("{code}{seed} age {years}: {} triangles turned >60 deg; worst: {:?}", bad.len(),
                bad.iter().take(4).map(|(d, c)| format!("{d:.2}@({:.2},{:.2},{:.2})", c.x, c.y, c.z)).collect::<Vec<_>>());
        }
    }
}

/// MakeHuman v2 finish vs FLAME render vs FLAME finish, ages 30 and 60.
fn compare() {
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    for (code, seed) in SHEET {
        let mut row = Vec::new();
        for years in [30, 60] {
            let id = format!("{code}_{seed}_age{years}");
            row.push(Rgba::load_png(Path::new(&format!("out/tmp/finished/{id}.png"))));
            row.push(Rgba::load_png(Path::new(&format!("{DIR}/finish_inputs/{id}.png"))));
            row.push(Rgba::load_png(Path::new(&format!("{DIR}/finished/{id}.png"))));
        }
        rows.push(row);
        labels.push(format!("{code} SEED {seed}"));
    }
    let cols = ["30 MH FINISH", "30 FLAME", "30 FLAME FIN", "60 MH FINISH", "60 FLAME", "60 FLAME FIN"].map(String::from);
    contact_sheet(&rows, 240, &cols, &labels).save_jpeg(Path::new("out/poc6_finish_compare.jpg"), 80);
}
