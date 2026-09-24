use regen_faces_poc::genome::{Genome, SHAPE_AXES};
use regen_faces_poc::pools::{nation, NATIONS};

#[test]
fn same_seed_same_genome() {
    for n in &NATIONS {
        for seed in [0u64, 1, 42, u64::MAX, 0xdead_beef_cafe_f00d] {
            let a = Genome::generate(seed, n, 2004);
            let b = Genome::generate(seed, n, 2004);
            assert_eq!(a, b);
            assert_eq!(a.to_json(), b.to_json());
        }
    }
}

#[test]
fn seed_stream_is_pinned() {
    // Guards against accidental changes to the generator: this seed must keep
    // producing this exact genome. Update deliberately with GENOME_VERSION.
    let g = Genome::generate(7, nation("CVD").unwrap(), 2005);
    let again: Genome = serde_json::from_str(&g.to_json()).unwrap();
    assert_eq!(g, again, "JSON round trip is exact");
    insta_like_pin(&g.to_json());
}

fn insta_like_pin(json: &str) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/pinned_seed7_cvd.json");
    match std::fs::read_to_string(path) {
        Ok(expected) => assert_eq!(json, expected.trim_end(), "seed 7 genome changed"),
        Err(_) => std::fs::write(path, json).unwrap(),
    }
}

#[test]
fn different_seeds_differ() {
    let n = nation("NHV").unwrap();
    let a = Genome::generate(1, n, 2004);
    let b = Genome::generate(2, n, 2004);
    assert_ne!(a.shape, b.shape);
}

#[test]
fn serialised_size_under_1kb() {
    let mut max = 0;
    for n in &NATIONS {
        for seed in 0..2000u64 {
            let len = Genome::generate(seed, n, 2004).to_json().len();
            max = max.max(len);
        }
    }
    println!("largest genome JSON: {max} bytes");
    assert!(max < 1024, "largest genome was {max} bytes");
}

#[test]
fn genes_are_in_range_and_mixes_sum_to_one() {
    for n in &NATIONS {
        for seed in 0..500u64 {
            let g = Genome::generate(seed, n, 2004);
            assert_eq!(g.shape.len(), SHAPE_AXES.len());
            assert!(g.shape.iter().all(|x| (-1.0..=1.0).contains(x)));
            assert!((0.0..=1.0).contains(&g.skin_tone));
            let s: f32 = g.ancestry.iter().map(|a| a.share).sum();
            assert!((s - 1.0).abs() < 0.01, "ancestry sums to {s}");
            let m: f32 = g.mh_mix.iter().sum();
            assert!((m - 1.0).abs() < 0.01, "mh mix sums to {m}");
        }
    }
}

#[test]
fn nations_produce_mixed_ancestry() {
    for n in &NATIONS {
        let mixed = (0..1000u64)
            .filter(|s| Genome::generate(*s, n, 2004).ancestry.len() > 1)
            .count();
        assert!(mixed > 100, "{} had only {mixed} mixed of 1000", n.code);
    }
}
