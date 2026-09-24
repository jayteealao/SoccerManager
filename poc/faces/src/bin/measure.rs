//! Compares head proportions with published adult male norms (mm), and
//! with `--fit` refits `src/calibration.rs` by least squares.
//!
//! Usage: cargo run --release --bin measure [-- --fit]

use regen_faces_poc::age::AgeState;
use regen_faces_poc::face::FaceBuilder;
use regen_faces_poc::genome::Genome;
use regen_faces_poc::head::{signed_pair, BaseHead, SHAPE_TARGETS};
use regen_faces_poc::measure::{Landmarks, NORMS};
use regen_faces_poc::pools::nation;

const M: usize = 13;
const RACES: [&str; 3] = ["african", "asian", "caucasian"];

/// Targets the fit may use. Each names a signed pair (see `signed_pair`).
/// With the largest weight the fit may give each: whole-head scales and eye
/// size stay small because large values read as unhuman.
const CAL: [(&str, f32); 13] = [
    ("neck/neck-scale-horiz", 0.5),
    ("head/head-fat", 0.4),
    ("mouth/mouth-scale-horiz", 0.8),
    ("mouth/mouth-trans", 0.5),
    ("chin/chin-height", 0.6),
    ("chin/chin-width", 0.6),
    ("chin/chin-bones", 0.6),
    ("head/head-scale-vert", 0.25),
    ("head/head-scale-horiz", 0.25),
    ("nose/nose-scale-horiz", 0.5),
    ("nose/nose-scale-vert", 0.5),
    ("eyes/{s}-eye-trans", 0.5),
    ("eyes/{s}-eye-scale", 0.25),
];

/// Shape genes whose spread is set from a population SD: axis -> measure.
const SPREAD: [(&str, usize); 7] = [
    ("head-width", 0),
    ("nose-length", 3),
    ("chin-height", 6),
    ("eye-size", 9),
    ("nose-width", 10),
    ("mouth-width", 11),
    ("jaw-width", 12),
];

fn neutral(race: usize) -> (Genome, AgeState) {
    let mut g = Genome::generate(0, nation("NHV").unwrap(), 2004);
    g.mh_mix = [0.0; 3];
    g.mh_mix[race] = 1.0;
    g.shape = vec![0.0; g.shape.len()];
    let age = AgeState::new(&g, 24.0);
    (g, age)
}

/// Weights for a neutral head: FLAME identity betas off (mean face only).
fn neutral_weights(bh: &BaseHead, g: &Genome, age: &AgeState) -> Vec<(String, f32)> {
    bh.weights_for(g, age).into_iter().filter(|(n, _)| !n.starts_with("flame/beta")).collect()
}

fn measure_with(bh: &mut BaseHead, lm: &Landmarks, base: &[(String, f32)], extra: &[(String, f32)]) -> [f32; M] {
    let mut w = base.to_vec();
    w.extend_from_slice(extra);
    lm.measure(&bh.build(&w))
}

fn signed(name: &str, x: f32) -> Vec<(String, f32)> {
    let (d, i) = signed_pair(name);
    let n = if x < 0.0 { d } else { i };
    if n.contains("{s}") {
        vec![(n.replace("{s}", "l"), x.abs()), (n.replace("{s}", "r"), x.abs())]
    } else {
        vec![(n, x.abs())]
    }
}

/// Target mm per measure and the weight (1/SD, 0 = unconstrained).
fn goal(race: usize) -> [(f32, f32); M] {
    let mut out = [(0.0, 0.0); M];
    for (k, n) in NORMS.iter().enumerate() {
        let w = 1.0 / n.sd;
        out[k] = match (race, k) {
            (_, 2) => (0.0, 0.0), // forehead: set by the hairline, not by targets
            // Inner canthus: the 3D pick disagrees with a 2D detector (MediaPipe
            // puts it wider), so let the outer span drive eye placement.
            (_, 7) => (0.0, 0.0),
            (0, 1) | (0, 12) => (0.0, 0.0), // no sourced African male value
            (0, _) => (n.african, w),
            // No sourced East Asian norms: constrain only vertical proportions
            // and mouth width (reported similar across groups).
            (1, 1) | (1, 3) | (1, 4) | (1, 5) | (1, 6) | (1, 11) => (n.naw, w),
            (1, _) => (0.0, 0.0),
            _ => (n.naw, w),
        };
    }
    out
}

fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for c in 0..n {
        let p = (c..n).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs())).unwrap();
        a.swap(c, p);
        b.swap(c, p);
        for r in c + 1..n {
            let f = a[r][c] / a[c][c];
            for k in c..n {
                a[r][k] -= f * a[c][k];
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = vec![0.0; n];
    for c in (0..n).rev() {
        x[c] = (b[c] - (c + 1..n).map(|k| a[c][k] * x[k]).sum::<f64>()) / a[c][c];
    }
    x
}

fn fit(fb: &mut FaceBuilder, lm: &Landmarks) {
    fb.base.calibrate = false;
    let mut cal_out = Vec::new();
    for (race, name) in RACES.iter().enumerate() {
        let (g, age) = neutral(race);
        let base = neutral_weights(&fb.base, &g, &age);
        let goal = goal(race);
        let mut x = vec![0.0f32; CAL.len()];
        for _round in 0..3 {
            let cur: Vec<(String, f32)> = CAL.iter().zip(&x).flat_map(|((n, _), v)| signed(n, *v)).collect();
            let m0 = measure_with(&mut fb.base, lm, &base, &cur);
            // Jacobian by central differences around the current weights.
            let mut jac = vec![[0.0f32; M]; CAL.len()];
            for (j, n) in CAL.iter().enumerate() {
                let eval = |fb: &mut FaceBuilder, d: f32| {
                    let mut xs = x.clone();
                    xs[j] += d;
                    let w: Vec<(String, f32)> = CAL.iter().zip(&xs).flat_map(|((n, _), v)| signed(n, *v)).collect();
                    measure_with(&mut fb.base, lm, &base, &w)
                };
                let (p, q) = (eval(fb, 0.2), eval(fb, -0.2));
                for k in 0..M {
                    jac[j][k] = (p[k] - q[k]) / 0.4;
                }
                let _ = n;
            }
            // Ridge least squares on the step, with bounds by clipping.
            let lambda = 0.3;
            let nv = CAL.len();
            let mut ata = vec![vec![0.0f64; nv]; nv];
            let mut atb = vec![0.0f64; nv];
            for k in 0..M {
                let (target, wt) = goal[k];
                if wt == 0.0 {
                    continue;
                }
                let r = (target - m0[k]) as f64 * wt as f64;
                for i in 0..nv {
                    atb[i] += jac[i][k] as f64 * wt as f64 * r;
                    for j2 in 0..nv {
                        ata[i][j2] += (jac[i][k] * jac[j2][k]) as f64 * (wt * wt) as f64;
                    }
                }
            }
            for i in 0..nv {
                ata[i][i] += lambda;
                atb[i] -= lambda * x[i] as f64;
            }
            let step = solve(ata, atb);
            for i in 0..nv {
                x[i] = (x[i] + step[i] as f32).clamp(-CAL[i].1, CAL[i].1);
            }
        }
        eprintln!("{name}: {:?}", CAL.iter().zip(&x).map(|((n, _), v)| format!("{n}={v:.2}")).collect::<Vec<_>>());
        cal_out.push(CAL.iter().zip(&x).filter(|(_, v)| v.abs() > 0.01).map(|((n, _), v)| (n.to_string(), *v)).collect::<Vec<_>>());
    }

    // Shape gene spread: a gene at 1 SD (0.45) should move its measure by
    // about one population SD.
    let (g, age) = neutral(2);
    let base = neutral_weights(&fb.base, &g, &age);
    let mut spread = Vec::new();
    for (axis, k) in SPREAD {
        let (_, decr, incr) = SHAPE_TARGETS.iter().find(|t| t.0 == axis).unwrap();
        let one = |n: &str| -> Vec<(String, f32)> {
            if n.contains("{s}") {
                vec![(n.replace("{s}", "l"), 0.5), (n.replace("{s}", "r"), 0.5)]
            } else {
                vec![(n.to_string(), 0.5)]
            }
        };
        let p = measure_with(&mut fb.base, lm, &base, &one(incr))[k];
        let q = measure_with(&mut fb.base, lm, &base, &one(decr))[k];
        let per_weight = (p - q).abs();
        if per_weight < 1.0 {
            eprintln!("spread {axis}: target barely moves {} ({per_weight:.1} mm); keep hand-set range", NORMS[k].name);
            continue;
        }
        let range = (NORMS[k].sd / (0.45 * per_weight)).clamp(0.08, 0.6);
        eprintln!("spread {axis}: {per_weight:.1} mm per unit weight -> range {range:.2}");
        spread.push((axis.to_string(), range));
    }

    let mut src = String::from("//! GENERATED by `cargo run --release --bin measure -- --fit`. Do not edit.\n//! Per-ancestry target weights that bring neutral heads to adult male norms,\n//! and shape gene ranges sized from population SDs. See `src/measure.rs`.\n\npub const CALIBRATION: [&[(&str, f32)]; 3] = [\n");
    for (race, c) in RACES.iter().zip(&cal_out) {
        src += &format!("    // {race}\n    &[\n");
        for (n, v) in c {
            src += &format!("        (\"{n}\", {v:.3}),\n");
        }
        src += "    ],\n";
    }
    src += "];\n\npub const SHAPE_RANGE: &[(&str, f32)] = &[\n";
    for (a, r) in spread {
        src += &format!("    (\"{a}\", {r:.3}),\n");
    }
    src += "];\n";
    let (file, cmd) = if fb.base.flame.is_some() {
        ("/src/calibration_flame.rs", "--fit --flame")
    } else {
        ("/src/calibration.rs", "--fit")
    };
    let src = src.replace("-- --fit`", &format!("-- {cmd}`"));
    std::fs::write(format!("{}{file}", env!("CARGO_MANIFEST_DIR")), src).unwrap();
    eprintln!("wrote {file}; rebuild and rerun without --fit to check");
}

fn main() {
    let mut fb = FaceBuilder::new().expect("MakeHuman assets");
    let flame = std::env::args().any(|a| a == "--flame");
    if flame {
        fb.base.flame = Some(regen_faces_poc::flame::Flame::load().expect("FLAME transfer data"));
    }
    let lm = Landmarks::find(&mut fb.base);
    if std::env::args().any(|a| a == "--fit") {
        fit(&mut fb, &lm);
        return;
    }
    let mut cols: Vec<(String, [f32; M])> = Vec::new();
    for (i, race) in RACES.iter().enumerate() {
        let (g, age) = neutral(i);
        let w = neutral_weights(&fb.base, &g, &age);
        cols.push((race.to_string(), lm.measure(&fb.base.build(&w))));
    }
    for (code, seed) in [("NHV", 6u64), ("CVD", 4), ("CVD", 1), ("KSI", 2), ("NHV", 13), ("NHV", 20)] {
        let g = Genome::generate(seed, nation(code).unwrap(), 2004);
        let age = AgeState::new(&g, 24.0);
        cols.push((format!("{code}{seed}"), lm.measure(&fb.head(&g, &age))));
    }
    print!("{:24} {:>6} {:>6} {:>5}", "measure (mm)", "NAW", "AFR", "SD");
    for (n, _) in &cols {
        print!(" {:>9}", n);
    }
    println!();
    for (k, norm) in NORMS.iter().enumerate() {
        print!("{:24} {:6.1} {:6.1} {:5.1}", norm.name, norm.naw, norm.african, norm.sd);
        for (_, m) in &cols {
            print!(" {:9.1}", m[k]);
        }
        println!();
    }
}
