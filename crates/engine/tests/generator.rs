//! AC-d: a seeded league of 20 clubs gives every club 22 players with legal positions, and
//! a world sample of one league per tier meets the world spread targets. Plus: the same seed
//! gives the same league.

mod common;

use engine::data::{Group, Position, generate_league, generate_league_in_tier};

#[test]
fn twenty_clubs_have_twenty_two_players_with_legal_positions() {
    let content = common::content();
    let league = generate_league(7, 20, &content);
    assert_eq!(league.len(), 20);
    for team in &league {
        assert_eq!(team.players.len(), 22, "club {}", team.club.id);
        assert!(
            team.players.iter().any(|p| p.position == Position::GK),
            "club {} has no goalkeeper",
            team.club.id
        );
        // A generated file, as `engine-cli generate` writes it (values under 1.0 lifted to
        // 1.0), passes the same validation a hand-written one must pass.
        let mut written = team.clone();
        engine::data::generator::lift_to_floor(&mut written);
        garde::Validate::validate_with(&written, &content.attributes)
            .unwrap_or_else(|r| panic!("club {} invalid: {r}", team.club.id));
        for p in &team.players {
            let (height, age) = (p.height.unwrap(), p.age.unwrap());
            assert!((150..=215).contains(&height), "{} height {height}", p.id);
            assert!((17..=38).contains(&age), "{} age {age}", p.id);
            assert_eq!(p.nationality.as_deref().map(str::len), Some(3), "{}", p.id);
        }
    }
}

/// One player's measured level and his attributes' deviations from it: the mean of his
/// outfield attributes (his goalkeeping ones for a keeper) after his position's offset for
/// each attribute's group. The hidden values are drawn apart from his level and are left out.
fn measure(p: &engine::data::PlayerEntry, content: &engine::data::Content) -> (f64, Vec<f64>) {
    let world = &content.tuning.generator.world;
    let offsets = world.offsets[p.position.code()].clone();
    let keeper = p.position == Position::GK;
    let values: Vec<f64> = content
        .attributes
        .attributes
        .iter()
        .filter(|def| !def.hidden && (def.group == Group::Goalkeeping) == keeper)
        .map(|def| p.attributes[&def.name].decimal() - offsets.get(def.group))
        .collect();
    let level = values.iter().sum::<f64>() / values.len() as f64;
    let deviations = values.iter().map(|v| v - level).collect();
    (level, deviations)
}

/// Clubs per tier in the world sample: enough that a tier's sample mean, drawn around its
/// target with the club spread, sits well inside the tolerance.
const CLUBS_PER_TIER: u32 = 200;

fn mean_and_spread(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let var = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n;
    (mean, var.sqrt())
}

/// The world spread: over a world sample of one league of [`CLUBS_PER_TIER`] clubs
/// per tier, the overall level centres on 10 with a spread of about 3.5, attributes spread
/// about 2 around their player's level, each tier's mean sits on its target, and the largest drop between tiers
/// is from the first to the second.
#[test]
fn the_world_sample_meets_the_spread_targets() {
    let content = common::content();
    let tiers = &content.tuning.generator.world.tiers;
    let mut levels = Vec::new();
    let mut deviations = Vec::new();
    let mut tier_means = Vec::new();
    for (tier, dist) in tiers.iter().enumerate() {
        let league = generate_league_in_tier(100 + tier as u64, CLUBS_PER_TIER, tier, &content);
        let mut tier_levels = Vec::new();
        for team in &league {
            for p in &team.players {
                let (level, dev) = measure(p, &content);
                tier_levels.push(level);
                deviations.extend(dev);
            }
        }
        let (mean, _) = mean_and_spread(&tier_levels);
        assert!(
            (mean - dist.mean).abs() <= 0.3,
            "tier {tier}: mean level {mean:.2} against {}",
            dist.mean
        );
        tier_means.push(mean);
        levels.extend(tier_levels);
    }
    let (mean, spread) = mean_and_spread(&levels);
    assert!((mean - 10.0).abs() <= 0.3, "overall level {mean:.2}");
    assert!((spread - 3.5).abs() <= 0.3, "overall spread {spread:.2}");
    let (_, attribute_spread) = mean_and_spread(&deviations);
    assert!(
        (attribute_spread - 2.0).abs() <= 0.25,
        "attribute spread {attribute_spread:.2}"
    );
    let gaps: Vec<f64> = tier_means.windows(2).map(|w| w[0] - w[1]).collect();
    let largest = gaps.iter().copied().fold(f64::MIN, f64::max);
    assert_eq!(gaps[0], largest, "tier gaps {gaps:?}");
}

/// The hidden values are drawn around the world's hidden mean, whatever a player's level:
/// over a world sample each centres on 10.0 within 0.3, and the strongest tier's mean is no
/// higher than the weakest's by more than the same margin.
#[test]
fn hidden_values_centre_on_ten_whatever_the_level() {
    let content = common::content();
    let hidden: Vec<String> = content
        .attributes
        .hidden_names()
        .map(str::to_string)
        .collect();
    assert_eq!(hidden, ["injury_proneness", "consistency"]);
    let tiers = content.tuning.generator.world.tiers.len();
    let mut by_tier = Vec::new();
    for tier in 0..tiers {
        let league = generate_league_in_tier(300 + tier as u64, 100, tier, &content);
        let values: Vec<Vec<f64>> = hidden
            .iter()
            .map(|name| {
                league
                    .iter()
                    .flat_map(|team| team.players.iter())
                    .map(|p| p.attributes[name].decimal())
                    .collect()
            })
            .collect();
        for (name, v) in hidden.iter().zip(&values) {
            let (mean, _) = mean_and_spread(v);
            assert!(
                (mean - 10.0).abs() <= 0.3,
                "tier {tier} {name}: mean {mean:.2}"
            );
        }
        by_tier.push(mean_and_spread(&values.concat()).0);
    }
    let gap = by_tier[0] - by_tier[tiers - 1];
    assert!(gap.abs() <= 0.3, "hidden means by tier {by_tier:?}");
}

/// Ratings are drawn inside 1.0 to 20.0, so a generated file needs no lift.
#[test]
fn a_generated_league_draws_every_rating_inside_the_range() {
    let content = common::content();
    for tier in 0..content.tuning.generator.world.tiers.len() {
        for mut team in generate_league_in_tier(9, 4, tier, &content) {
            assert_eq!(
                engine::data::generator::lift_to_floor(&mut team),
                0,
                "tier {tier}"
            );
        }
    }
}

#[test]
fn the_same_seed_gives_the_same_league() {
    let content = common::content();
    let a = generate_league(3, 4, &content);
    let b = generate_league(3, 4, &content);
    assert_eq!(a, b);
    let c = generate_league(4, 4, &content);
    assert_ne!(a, c);
}

/// The body half of the world targets: over a generated top-flight league, the mean height
/// of each position is within 2 cm of its content mean, the age mean and spread within a
/// year of theirs, and the share of foreign players within 0.05 of its content share.
#[test]
fn generated_heights_ages_and_nationalities_meet_the_content_targets() {
    let content = common::content();
    let body = content
        .tuning
        .generator
        .body
        .as_ref()
        .expect("the shipped generator body");
    let league = generate_league_in_tier(11, 60, 0, &content);
    let players: Vec<&engine::data::PlayerEntry> = league.iter().flat_map(|t| &t.players).collect();
    for position in Position::ALL {
        let heights: Vec<f64> = players
            .iter()
            .filter(|p| p.position == position)
            .map(|p| f64::from(p.height.unwrap()))
            .collect();
        if heights.is_empty() {
            continue;
        }
        let mean = heights.iter().sum::<f64>() / heights.len() as f64;
        let target = body.height[position.code()].mean;
        assert!(
            (mean - target).abs() <= 2.0,
            "{} mean height {mean:.1} against {target} over {}",
            position.code(),
            heights.len()
        );
    }
    let ages: Vec<f64> = players.iter().map(|p| f64::from(p.age.unwrap())).collect();
    let n = ages.len() as f64;
    let mean = ages.iter().sum::<f64>() / n;
    let spread = (ages.iter().map(|a| (a - mean) * (a - mean)).sum::<f64>() / n).sqrt();
    assert!((mean - body.age.mean).abs() <= 1.0, "age mean {mean:.2}");
    assert!(
        (spread - body.age.spread).abs() <= 1.0,
        "age spread {spread:.2}"
    );
    let nation = &body.nationality;
    let foreign = players
        .iter()
        .filter(|p| p.nationality.as_deref() != Some(nation.home.as_str()))
        .count() as f64
        / n;
    assert!(
        (foreign - nation.foreign_share).abs() <= 0.05,
        "foreign share {foreign:.3} against {}",
        nation.foreign_share
    );
}
