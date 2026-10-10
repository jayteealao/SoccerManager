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
//! moves. The person fields of a version 3 file (position ratings, the feet, the traits of
//! play) come from a third keyed stream drawn after the bodies; the personality values are
//! hidden values after the last visible attribute, so the hidden stream draws them. The whole
//! league is still made in one generator run.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::contract::level::overall_level;
use crate::data::attributes::AttributeSchema;
use crate::data::team::{
    AGE_YEARS, Club, Foot, Ground, HEIGHT_CM, Kit, PlayerEntry, Position, Side, TEAM_VERSION,
    TeamFile,
};
use crate::data::traits::{TraitCatalogue, TraitStrength};
use crate::data::tuning::{BodyTuning, HiddenDist, PersonTuning};
use crate::data::{Content, hex12, names};
use crate::player::Attributes;
use crate::rating::{MAX_TENTHS, MIN_TENTHS, Rating};
use crate::rng::EngineRng;

/// The key the body-field stream's seed is mixed with.
const BODY_KEY: u64 = 0x626f_6479_6669_656c; // "bodyfiel"
/// The key the person-field stream's seed is mixed with.
const PERSON_KEY: u64 = 0x7065_7273_6f6e_6669; // "personfi"
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
                positions: None,
                foot: None,
                traits: BTreeMap::new(),
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
    if let Some(person) = &spec.person {
        draw_person(&mut out, person, &content.traits, &content.attributes, seed);
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

/// A rating of `value` on the 1 to 20 scale, rounded to a tenth and kept to 1.0 to `top`.
fn kept(value: f64, top: Rating) -> Rating {
    let tenths = (value * 10.0)
        .round()
        .clamp(f64::from(MIN_TENTHS), f64::from(top.tenths()));
    // The clamp keeps the value inside 10..=200, so the cast cannot truncate.
    Rating::from_tenths(tenths as u8)
}

/// Draws every player's position ratings, feet, and traits of play from the person stream, in
/// league order. His natural position is rated at his level; each position the tuning relates
/// to it sits its gap below, give or take the spread; every other position (and, for a keeper,
/// every outfield position, and for an outfield player the keeper position) is drawn around
/// the far mean. No position is above the natural one. A keeper carries only the keeper
/// traits, an outfield player never one of them; no player carries two opposites.
fn draw_person(
    league: &mut [TeamFile],
    person: &PersonTuning,
    catalogue: &TraitCatalogue,
    schema: &AttributeSchema,
    seed: u64,
) {
    let mut rng = EngineRng::from_seed(seed ^ PERSON_KEY);
    let draw = &person.positions;
    let traits = &person.traits;
    let total: f64 = traits.count_weights.iter().sum();
    for team in league {
        for p in &mut team.players {
            let level = overall_level(
                &Attributes::from_entry(&p.attributes, schema),
                schema,
                p.position,
            )
            .clamp(MIN_TENTHS, MAX_TENTHS);
            let natural = Rating::from_tenths(level);
            let keeper = p.position == Position::GK;
            let related = draw.related.get(p.position.code());
            let positions = Position::ALL
                .into_iter()
                .map(|code| {
                    if code == p.position {
                        return (code, natural);
                    }
                    let gap = related
                        .and_then(|r| r.get(code.code()))
                        .filter(|_| keeper == (code == Position::GK));
                    let value = match gap {
                        Some(gap) => natural.decimal() - gap + rng.bell(0.0, draw.spread),
                        None => rng.bell(draw.far.mean, draw.far.spread),
                    };
                    (code, kept(value, natural))
                })
                .collect();
            p.positions = Some(positions);
            let foot = &person.foot;
            let strong = if rng.chance(foot.left_share) {
                Side::Left
            } else {
                Side::Right
            };
            let weaker = rng
                .bell(foot.weaker.mean, foot.weaker.spread)
                .round()
                .clamp(f64::from(foot.weaker.min), f64::from(foot.weaker.max));
            // The clamp keeps the draw inside 1..=100, so the cast cannot truncate.
            p.foot = Some(Foot {
                strong,
                weaker: weaker as u8,
            });
            let mut pick = rng.next_f64() * total;
            let mut count = traits.count_weights.len() - 1;
            for (n, w) in traits.count_weights.iter().enumerate() {
                if pick < *w {
                    count = n;
                    break;
                }
                pick -= w;
            }
            p.traits = BTreeMap::new();
            while p.traits.len() < count {
                let open: Vec<&str> = catalogue
                    .traits
                    .iter()
                    .map(|t| t.name.as_str())
                    .filter(|n| traits.keeper_only.iter().any(|k| k == n) == keeper)
                    .filter(|n| !p.traits.contains_key(*n))
                    .filter(|n| {
                        catalogue
                            .opposite(n)
                            .is_none_or(|o| !p.traits.contains_key(o))
                    })
                    .collect();
                if open.is_empty() {
                    break;
                }
                let name = open[rng.range_usize(open.len())].to_string();
                let strength = if rng.chance(traits.strong_share) {
                    TraitStrength::Strong
                } else {
                    TraitStrength::Moderate
                };
                p.traits.insert(name, strength);
            }
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
