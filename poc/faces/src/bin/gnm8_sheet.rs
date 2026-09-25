//! PoC 8: GNM Head (Apache-2.0) with the sourced genome and the PoC 8
//! shaders.
//!
//! `gnm8_sheet list`       genomes of the three nations (pick sheet seeds)
//! `gnm8_sheet identity`   clay and full renders of the sheet genomes at 24
//! `gnm8_sheet classes`    GNM classes at the same latent (ancestry only)
//! `gnm8_sheet ages`       sheet genomes x eight ages, plus finish inputs
//! `gnm8_sheet hair`       hair styles x skin tones, old vs new hair path
//! `gnm8_sheet eyes`       eye close-ups: PoC 7 eye vs the PoC 8 eye model
//! `gnm8_sheet mst`        Monk Skin Tone 1-10 on one face
//! `gnm8_sheet family`     a father and three sons
//! `gnm8_sheet compare`    render vs finish pairs (after finish.py)
//!
//! Needs gnm/gnm_prep.py and gnm/gnm_norms.py to have run.

use regen_faces_poc::age::SHEET_AGES;
use regen_faces_poc::cards::Style;
use regen_faces_poc::genes::{chromophores, nation2, Genome2};
use regen_faces_poc::gnm::GnmAsset;
use regen_faces_poc::img::{contact_sheet, Rgba};
use regen_faces_poc::render::{Camera, Draw, Frame, RenderOpts, Renderer, SIZE};
use std::path::Path;

const DIR: &str = "out/tmp/gnm8";

pub const SHEET: [(&str, u64); 6] = [("NHV", 6), ("CVD", 4), ("CVD", 1), ("KSI", 2), ("NHV", 13), ("KSI", 5)];

fn opts() -> RenderOpts {
    RenderOpts { stochastic_hair: true, passes: 8 }
}

fn clay(mut draws: Vec<Draw>) -> Vec<Draw> {
    draws.truncate(1);
    draws[0].params.colour = [0.9, 0.2, 0.0, 0.0];
    draws[0].params.p0 = [0.0; 4];
    for v in &mut draws[0].vertices {
        v.aux = [0.0; 4];
        v.aux2[0] = 0.0;
        v.aux2[1] = 0.0;
        v.aux2[2] = 0.0;
        v.aux3 = [0.0, 0.0, 0.0, 1.0];
    }
    draws
}

fn img(f: Frame) -> Rgba {
    Rgba::from_raw(SIZE, SIZE, f.colour)
}

fn describe(g: &Genome2) -> String {
    let anc: Vec<String> = g.ancestry.iter().map(|(c, w)| format!("{c:?} {:.0}", w * 100.0)).collect();
    format!(
        "{} {} | {} | L*{:.0} MST{} | {:?} {:?} | grey {:.0}",
        g.nation,
        g.seed,
        anc.join(" "),
        g.skin_lab[0],
        chromophores::mst_bucket(g.skin_lab),
        g.hair_type(),
        g.eye_colour,
        g.grey_onset
    )
}

fn list() {
    for code in ["NHV", "CVD", "KSI"] {
        for seed in 0..16 {
            println!("{}", describe(&Genome2::generate(seed, nation2(code).unwrap(), 2004)));
        }
    }
}

fn genome(code: &str, seed: u64) -> Genome2 {
    Genome2::generate(seed, nation2(code).unwrap(), 2004)
}

fn identity(a: &GnmAsset, r: &Renderer) {
    let cam = Camera::portrait_flame(0.0);
    let mut clay_row = Vec::new();
    let mut full_row = Vec::new();
    let mut labels = Vec::new();
    for (code, seed) in SHEET {
        let g = genome(code, seed);
        eprintln!("{}", describe(&g));
        clay_row.push(img(r.render(&clay(a.portrait(&g, 24.0, None)), &cam)));
        let style = Style::for_genome(&g.legacy());
        full_row.push(img(r.render_opts(&a.portrait(&g, 24.0, Some(style)), &cam, opts())));
        labels.push(format!("{code} {seed}"));
    }
    contact_sheet(&[clay_row, full_row], 260, &labels, &["CLAY".into(), "FULL".into()]).save_jpeg(Path::new("out/poc8_gnm_identity.jpg"), 84);
}

fn classes(a: &GnmAsset, r: &Renderer) {
    // Same latent and residual, only GNM's ethnicity condition changes.
    let cam = Camera::portrait_flame(20.0);
    let names = ["MIDDLE EASTERN", "ASIAN", "WHITE", "BLACK"];
    let mut rows = Vec::new();
    let mut row_labels = Vec::new();
    for seed in [3u64, 11, 27] {
        let base = genome("NHV", seed);
        let mut row = Vec::new();
        for e in 0..4 {
            let mut cond = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
            cond[2 + e] = 1.0;
            let id = a.identity_for(&cond, &base.face_z, &base.face_r);
            let age = a.age_state(&base, 24.0);
            let head = a.build_id(&id, base.fat_tendency, &age);
            let draws = a.draws(&base, &base.legacy(), &age, &head, None);
            row.push(img(r.render(&clay(draws), &cam)));
        }
        rows.push(row);
        row_labels.push(format!("LATENT {seed}"));
    }
    let cols: Vec<String> = names.iter().map(|s| s.to_string()).collect();
    contact_sheet(&rows, 240, &cols, &row_labels).save_jpeg(Path::new("out/poc8_gnm_classes.jpg"), 84);
}

fn ages(a: &GnmAsset, r: &Renderer) {
    let cam = Camera::portrait_flame(12.0);
    std::fs::create_dir_all(format!("{DIR}/renders")).unwrap();
    std::fs::create_dir_all(format!("{DIR}/finish_inputs")).unwrap();
    std::fs::create_dir_all(format!("{DIR}/finished")).unwrap();
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    let mut manifest = String::new();
    let mut genomes = serde_json::Map::new();
    for (code, seed) in SHEET {
        let g = genome(code, seed);
        genomes.insert(format!("{code}_{seed}"), serde_json::from_str(&g.to_json()).unwrap());
        let style = Style::for_genome(&g.legacy());
        labels.push(format!("{code} SEED {seed}\n{}", style.label()));
        let mut row = Vec::new();
        for years in SHEET_AGES {
            let f = r.render_opts(&a.portrait(&g, years, Some(style)), &cam, opts());
            let id = format!("{code}_{seed}_age{}", years as u32);
            let depth = Rgba::from_raw(SIZE, SIZE, f.depth.clone());
            let mask = Rgba::from_raw(SIZE, SIZE, f.mask.clone());
            let im = img(f);
            im.save_png(Path::new(&format!("{DIR}/renders/{id}.png")));
            if [19.0, 30.0, 60.0].contains(&years) {
                im.save_png(Path::new(&format!("{DIR}/finish_inputs/{id}.png")));
                depth.save_png(Path::new(&format!("{DIR}/finish_inputs/{id}_depth.png")));
                mask.save_png(Path::new(&format!("{DIR}/finish_inputs/{id}_mask.png")));
                let eyes = format!("{:?}", g.eye_colour).to_lowercase();
                // Columns: input, depth, output, prompt set, mask, seed, target skin CIELAB.
                manifest += &format!(
                    "{DIR}/finish_inputs/{id}.png,{DIR}/finish_inputs/{id}_depth.png,{DIR}/finished/{id}.png,embeds_age{}_{eyes},{DIR}/finish_inputs/{id}_mask.png,{},{:.1} {:.1} {:.1}\n",
                    years as u32,
                    g.seed ^ (years as u64) << 32 ^ code.bytes().fold(0u64, |h, b| h * 131 + b as u64),
                    g.skin_lab[0],
                    g.skin_lab[1],
                    g.skin_lab[2]
                );
            }
            row.push(im);
        }
        rows.push(row);
    }
    std::fs::write(format!("{DIR}/pairs.csv"), manifest).unwrap();
    std::fs::write(format!("{DIR}/genomes.json"), serde_json::to_string_pretty(&genomes).unwrap()).unwrap();
    let cols: Vec<String> = SHEET_AGES.iter().map(|a| format!("AGE {a}")).collect();
    contact_sheet(&rows, 240, &cols, &labels).save_jpeg(Path::new("out/poc8_gnm_age_sheet.jpg"), 82);
}

fn hair(a: &GnmAsset, r: &Renderer) {
    let styles = [Style::ShortStraight, Style::MediumWavy, Style::ShortCurly, Style::TightFade];
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    for (dark, red, name) in [(0.2, 0.0, "BLOND"), (0.55, 0.0, "BROWN"), (0.95, 0.0, "BLACK"), (0.3, 0.9, "RED")] {
        let mut g = genome("NHV", 6);
        g.hair_dark = dark;
        g.red = red;
        for (new, tag) in [(false, "HAIR V1 A2C"), (true, "HAIR V2")] {
            let row = styles
                .iter()
                .map(|s| {
                    let mut draws = a.portrait(&g, 26.0, Some(*s));
                    if !new {
                        for d in &mut draws {
                            if d.params.kind == regen_faces_poc::render::kind::HAIR_CARD2 && d.texture.is_some() {
                                d.params.kind = regen_faces_poc::render::kind::HAIR_CARD;
                            }
                        }
                    }
                    let o = if new { opts() } else { RenderOpts::default() };
                    img(r.render_opts(&draws, &Camera::hair_closeup(35.0), o))
                })
                .collect();
            rows.push(row);
            labels.push(format!("{name}\n{tag}"));
        }
    }
    let cols: Vec<String> = styles.iter().map(|s| s.label().to_uppercase()).collect();
    contact_sheet(&rows, 260, &cols, &labels).save_jpeg(Path::new("out/poc8_gnm_hair_sheet.jpg"), 84);
}

fn eyes(a: &GnmAsset, r: &Renderer) {
    let cam = Camera { eye: glam::Vec3::new(0.9, 7.35, 5.2), target: glam::Vec3::new(0.0, 7.2, 1.25), fov_y_deg: 17.0 };
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    for (code, seed) in [("NHV", 6), ("KSI", 2), ("CVD", 1)] {
        let g = genome(code, seed);
        let full = a.portrait(&g, 30.0, None);
        // Without the PoC 8 eye layers: interior only, no cornea, no
        // occlusion, no lashes, no wet margins.
        let mut bare: Vec<Draw> = Vec::new();
        for d in a.portrait(&g, 30.0, None) {
            use regen_faces_poc::render::kind;
            if matches!(d.params.kind, kind::CORNEA | kind::OCCLUSION | kind::HAIR_CARD2) {
                continue;
            }
            bare.push(d);
        }
        for v in &mut bare[0].vertices {
            v.aux2[2] = 0.0;
            v.aux3[2] = 0.0;
        }
        rows.push(vec![img(r.render_opts(&bare, &cam, opts())), img(r.render_opts(&full, &cam, opts()))]);
        labels.push(format!("{code} {seed}"));
    }
    contact_sheet(&rows, 380, &["NO EYE LAYERS".into(), "POC 8 EYE MODEL".into()], &labels).save_jpeg(Path::new("out/poc8_gnm_eyes.jpg"), 88);
}

fn mst(a: &GnmAsset, r: &Renderer) {
    let cam = Camera::portrait_flame(10.0);
    let mut row = Vec::new();
    let mut cols = Vec::new();
    let mut report = String::from("MST swatch -> chromophore fit -> albedo dE76\n");
    for (i, h) in chromophores::MST.iter().enumerate() {
        let mut g = genome("CVD", 4);
        g.skin_lab = chromophores::lab(chromophores::srgb_hex(h));
        let (cm, ch) = chromophores::invert(g.skin_lab);
        let e = chromophores::de(chromophores::lab(chromophores::albedo(cm, ch)), g.skin_lab);
        report += &format!("MST {:2} #{h}  Cm {cm:.2} Ch {ch:.2}  dE {e:.2}\n", i + 1);
        let f = r.render_opts(&a.portrait(&g, 26.0, Some(Style::TightFade)), &cam, opts());
        let im = img(f);
        im.save_png(Path::new(&format!("{DIR}/mst/mst{:02}.png", i + 1)));
        row.push(im);
        cols.push(format!("MST {}", i + 1));
    }
    std::fs::write("out/poc8_mst.txt", report).unwrap();
    contact_sheet(&[row], 200, &cols, &["".into()]).save_jpeg(Path::new("out/poc8_gnm_mst.jpg"), 84);
}

fn family(a: &GnmAsset, r: &Renderer) {
    let cam = Camera::portrait_flame(12.0);
    let nat = nation2("CVD").unwrap();
    let mut rows = Vec::new();
    let mut labels = Vec::new();
    for fseed in [4u64, 9] {
        let father = genome("CVD", fseed);
        let mut row = vec![img(r.render_opts(&a.portrait(&father, 45.0, Some(Style::for_genome(&father.legacy()))), &cam, opts()))];
        for k in 0..3u64 {
            let son = Genome2::child_of(&father, nat, fseed * 1000 + k, 2004);
            row.push(img(r.render_opts(&a.portrait(&son, 22.0, Some(Style::for_genome(&son.legacy()))), &cam, opts())));
        }
        rows.push(row);
        labels.push(format!("CVD {fseed}"));
    }
    let cols = vec!["FATHER AT 45".into(), "SON 1 AT 22".into(), "SON 2 AT 22".into(), "SON 3 AT 22".into()];
    contact_sheet(&rows, 240, &cols, &labels).save_jpeg(Path::new("out/poc8_gnm_family.jpg"), 84);
}

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
    contact_sheet(&rows, 240, &cols, &labels).save_jpeg(Path::new("out/poc8_finish_pairs.jpg"), 82);
}

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    if cmd == "list" {
        return list();
    }
    if cmd == "compare" {
        return compare();
    }
    std::fs::create_dir_all(format!("{DIR}/mst")).unwrap();
    let a = GnmAsset::load();
    let r = Renderer::new();
    match cmd.as_str() {
        "identity" => identity(&a, &r),
        "classes" => classes(&a, &r),
        "ages" => ages(&a, &r),
        "hair" => hair(&a, &r),
        "eyes" => eyes(&a, &r),
        "mst" => mst(&a, &r),
        "family" => family(&a, &r),
        _ => eprintln!("usage: gnm8_sheet list|identity|classes|ages|hair|eyes|mst|family|compare"),
    }
}
