//! The team generator: fictional clubs and players with per-position attribute
//! distributions, seedable, from one `EngineRng` stream.
//!
//! Attributes are drawn on the old 1 to 100 scale and stored as `2v` tenths, so a league
//! is the same league it was before ratings moved to tenths. The body fields come from a
//! second stream keyed from the seed, drawn after the league, so no attribute draw moves.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::data::team::{
    AGE_YEARS, Club, Ground, HEIGHT_CM, Kit, PlayerEntry, TEAM_VERSION, TeamFile,
};
use crate::data::tuning::BodyTuning;
use crate::data::{Content, hex12, names};
use crate::rating::Rating;
use crate::rng::EngineRng;

/// The key the body-field stream's seed is mixed with.
const BODY_KEY: u64 = 0x626f_6479_6669_656c; // "bodyfiel"

/// Generates `clubs` team files from `seed`. The same seed and content give the same output.
/// Without a `generator.body` block in the tuning, players have no body fields; such a
/// league plays, but `engine-cli generate` refuses to write it.
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
                attributes.insert(def.name.clone(), Rating::from_tenths(2 * value));
            }
            players.push(PlayerEntry {
                id: format!("p-{club_id}-{:02}", i + 1),
                name: names::player_name(&mut rng),
                // Squad size is at most 40, so the shirt number fits.
                shirt: (i + 1) as u8,
                position,
                attributes,
                height: None,
                age: None,
                nationality: None,
            });
        }
        out.push(TeamFile {
            schema_version: TEAM_VERSION,
            club: Club {
                id: club_id,
                name,
                short_name,
                kit: Kit { primary, secondary },
                ground: Ground::default(),
            },
            players,
        });
    }
    if let Some(body) = &spec.body {
        draw_bodies(&mut out, body, seed);
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

/// Draws every player's height, age, and nationality from the body stream, in league order.
fn draw_bodies(league: &mut [TeamFile], body: &BodyTuning, seed: u64) {
    let mut rng = EngineRng::from_seed(seed ^ BODY_KEY);
    let (lo, hi) = (f64::from(*HEIGHT_CM.start()), f64::from(*HEIGHT_CM.end()));
    let age = &body.age;
    let (age_lo, age_hi) = (
        f64::from(age.min.max(*AGE_YEARS.start())),
        f64::from(age.max.min(*AGE_YEARS.end())),
    );
    let nation = &body.nationality;
    for team in league {
        for p in &mut team.players {
            let h = &body.height[p.position.code()];
            // Each clamp keeps the draw inside a u8 range, so the casts cannot truncate.
            p.height = Some(rng.bell(h.mean, h.spread).round().clamp(lo, hi) as u8);
            p.age = Some(rng.bell(age.mean, age.spread).round().clamp(age_lo, age_hi) as u8);
            let code = if rng.chance(nation.foreign_share) {
                &nation.foreign[rng.range_usize(nation.foreign.len())]
            } else {
                &nation.home
            };
            p.nationality = Some(code.clone());
        }
    }
}

/// Lifts every rating under 1.0 in `team` to 1.0 and returns how many were lifted. The
/// generator draws on the old scale, so a few values (goalkeeping on outfield players)
/// convert under 1.0; a version 2 file may not hold them, so the written copy is lifted.
pub fn lift_to_floor(team: &mut TeamFile) -> usize {
    let floor = Rating::from_tenths(crate::rating::MIN_TENTHS);
    let mut lifted = 0;
    for p in &mut team.players {
        for v in p.attributes.values_mut() {
            if *v < floor {
                *v = floor;
                lifted += 1;
            }
        }
    }
    lifted
}
