//! The team generator: fictional clubs and players drawn around the world spread, seedable,
//! from one `EngineRng` stream.
//!
//! A club's level is drawn around its tier's mean, each player's level around his club's,
//! and each attribute around the player's level plus his position's offset for the
//! attribute's group, all on the 1 to 20 scale and rounded to a tenth. A hidden value is drawn
//! around its own mean, independent of the player's level, and the league's attribute mean
//! leaves it out. A hidden value in the schema among the visible ones is drawn in its place;
//! one after the last visible attribute (added at the schema's end) comes from a stream of its
//! own, drawn after the league, so adding it moves no earlier draw. The body fields come
//! from a second stream keyed from the seed, drawn after the league, so no attribute draw
//! moves.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::data::team::{
    AGE_YEARS, Club, Ground, HEIGHT_CM, Kit, PlayerEntry, TEAM_VERSION, TeamFile,
};
use crate::data::tuning::{BodyTuning, HiddenDist};
use crate::data::{Content, hex12, names};
use crate::rating::Rating;
use crate::rng::EngineRng;

/// The key the body-field stream's seed is mixed with.
const BODY_KEY: u64 = 0x626f_6479_6669_656c; // "bodyfiel"
/// Keys the stream of the hidden values added at the schema's end.
const HIDDEN_KEY: u64 = 0x6869_6464_656e_7661; // "hiddenva"

/// Generates `clubs` team files of the top tier from `seed`. The same seed and content give
/// the same output. Without a `generator.body` block in the tuning, players have no body
/// fields; such a league plays, but `engine-cli generate` refuses to write it.
pub fn generate_league(seed: u64, clubs: u32, content: &Content) -> Vec<TeamFile> {
    generate_league_in_tier(seed, clubs, 0, content)
}

/// Generates `clubs` team files of tier `tier` (0 is the top flight; a tier past the last
/// reads the last) from `seed`.
pub fn generate_league_in_tier(
    seed: u64,
    clubs: u32,
    tier: usize,
    content: &Content,
) -> Vec<TeamFile> {
    let mut rng = EngineRng::from_seed(seed);
    let spec = &content.tuning.generator;
    let world = &spec.world;
    // The schema places up to the last visible attribute are drawn in the league stream; the
    // hidden values after it in their own stream.
    let schema = &content.attributes.attributes;
    let appended = schema.iter().rposition(|d| !d.hidden).map_or(0, |i| i + 1);
    let level = &world.tiers[tier.min(world.tiers.len() - 1)];
    let mut out = Vec::with_capacity(clubs as usize);
    let mut count = 0u64;
    let mut sum = 0u64;
    let mut min = u8::MAX;
    let mut max = u8::MIN;
    for n in 0..clubs {
        let club_id = format!("club-{seed:08x}-{n:02}");
        let (name, short_name) = names::club_name(&mut rng);
        let (primary, secondary) = names::kit(&mut rng);
        let club_level = rng.bell(level.mean, level.club_spread);
        let positions = spec
            .slot_positions
            .iter()
            .chain(spec.bench_positions.iter())
            .copied();
        let mut players = Vec::with_capacity(spec.squad_size);
        for (i, position) in positions.enumerate() {
            let offsets = &world.offsets[position.code()];
            let player_level = rng.bell(club_level, world.player_spread);
            let mut attributes = BTreeMap::new();
            for def in &content.attributes.attributes[..appended] {
                // A hidden value is drawn in its schema place, around its own mean,
                // independent of the player's level and position.
                if def.hidden {
                    let value = rng.rating(world.hidden.mean, world.hidden.spread);
                    attributes.insert(def.name.clone(), value);
                    continue;
                }
                let value = rng.rating(
                    player_level + offsets.get(def.group),
                    world.attribute_spread,
                );
                let tenths = value.tenths();
                count += 1;
                sum += u64::from(tenths);
                min = min.min(tenths);
                max = max.max(tenths);
                attributes.insert(def.name.clone(), value);
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
                condition: None,
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
    draw_appended_hidden(&mut out, &schema[appended..], world.hidden, seed);
    if let Some(body) = &spec.body {
        draw_bodies(&mut out, body, seed);
    }
    let json = serde_json::to_vec(&out).unwrap_or_default();
    let hash = hex12(&Sha256::digest(&json));
    // The mean on the 1 to 20 scale, to two decimals; the least and greatest as decimals.
    let mean = if count == 0 {
        0.0
    } else {
        (sum as f64 / count as f64 * 10.0).round() / 100.0
    };
    tracing::info!(
        signal = "generator.league",
        seed,
        clubs,
        tier,
        players = out.iter().map(|t| t.players.len()).sum::<usize>(),
        attr.mean = mean,
        attr.min = Rating::from_tenths(min).decimal(),
        attr.max = Rating::from_tenths(max).decimal(),
        hash = %hash
    );
    out
}

/// Draws the hidden values in `defs` (those after the schema's last visible attribute) for
/// every player from the hidden stream, in league order, around the world's hidden mean.
fn draw_appended_hidden(
    league: &mut [TeamFile],
    defs: &[crate::data::attributes::AttributeDef],
    dist: HiddenDist,
    seed: u64,
) {
    if defs.is_empty() {
        return;
    }
    let mut rng = EngineRng::from_seed(seed ^ HIDDEN_KEY);
    for team in league {
        for p in &mut team.players {
            for def in defs {
                p.attributes
                    .insert(def.name.clone(), rng.rating(dist.mean, dist.spread));
            }
        }
    }
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
/// generator draws every rating inside 1.0 to 20.0, so it lifts nothing; it stays as the
/// guard before a version 2 file is written.
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
