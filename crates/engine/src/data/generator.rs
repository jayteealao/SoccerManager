//! The team generator: fictional clubs and players with per-position attribute
//! distributions, seedable, from one `EngineRng` stream.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::data::team::{Club, Kit, PlayerEntry, TEAM_VERSION, TeamFile};
use crate::data::{Content, hex12, names};
use crate::rng::EngineRng;

/// Generates `clubs` team files from `seed`. The same seed and content give the same output.
pub fn generate_league(seed: u64, clubs: u32, content: &Content) -> Vec<TeamFile> {
    let mut rng = EngineRng::from_seed(seed);
    let spec = &content.tuning.generator;
    let mut out = Vec::with_capacity(clubs as usize);
    let mut count = 0u64;
    let mut sum = 0u64;
    let mut min = u8::MAX;
    let mut max = u8::MIN;
    for n in 0..clubs {
        let club_id = format!("club-{seed:08x}-{n:02}");
        let (name, short_name) = names::club_name(&mut rng);
        let (primary, secondary) = names::kit(&mut rng);
        let positions = spec
            .slot_positions
            .iter()
            .chain(spec.bench_positions.iter())
            .copied();
        let mut players = Vec::with_capacity(spec.squad_size);
        for (i, position) in positions.enumerate() {
            let dist = &spec.per_position[position.code()];
            let mut attributes = BTreeMap::new();
            for def in &content.attributes.attributes {
                let d = dist.get(def.group);
                let value = rng.attribute(d.mean, d.spread);
                count += 1;
                sum += u64::from(value);
                min = min.min(value);
                max = max.max(value);
                attributes.insert(def.name.clone(), value);
            }
            players.push(PlayerEntry {
                id: format!("p-{club_id}-{:02}", i + 1),
                name: names::player_name(&mut rng),
                // Squad size is at most 40, so the shirt number fits.
                shirt: (i + 1) as u8,
                position,
                attributes,
            });
        }
        out.push(TeamFile {
            schema_version: TEAM_VERSION,
            club: Club {
                id: club_id,
                name,
                short_name,
                kit: Kit { primary, secondary },
            },
            players,
        });
    }
    let json = serde_json::to_vec(&out).unwrap_or_default();
    let hash = hex12(&Sha256::digest(&json));
    let mean = if count == 0 {
        0.0
    } else {
        (sum as f64 / count as f64 * 100.0).round() / 100.0
    };
    tracing::info!(
        signal = "generator.league",
        seed,
        clubs,
        players = out.iter().map(|t| t.players.len()).sum::<usize>(),
        attr.mean = mean,
        attr.min = min,
        attr.max = max,
        hash = %hash
    );
    out
}
