//! PoC 1: prints 20 sample genomes for each of three sample nations as JSON.
//!
//! Usage: cargo run --bin genomes [-- --count N]

use regen_faces_poc::genome::Genome;
use regen_faces_poc::pools::NATIONS;
use regen_faces_poc::rng::Rng;

fn main() {
    let count: usize = std::env::args()
        .skip_while(|a| a != "--count")
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(20);

    let mut out = serde_json::Map::new();
    for (n, nation) in NATIONS.iter().enumerate() {
        let genomes: Vec<Genome> = (0..count)
            .map(|i| {
                // Seeds are arbitrary u64s; derive them from a fixed root so the
                // sample file is reproducible.
                let seed = Rng::fork(2026, &format!("{}-{}", nation.code, i)).next_u64();
                let birth_year = 1990 + ((seed >> 7) % 20) as u16;
                Genome::generate(seed, nation, birth_year)
            })
            .collect();
        let _ = n;
        out.insert(
            nation.code.to_string(),
            serde_json::json!({ "name": nation.name, "genomes": genomes }),
        );
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
