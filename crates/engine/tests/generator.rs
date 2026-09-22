//! AC-d: a seeded league of 20 clubs gives every club 22 players with legal positions, and
//! per-position attribute means fall inside the bounds the tuning file sets. Plus: the same
//! seed gives the same league.

mod common;

use std::collections::BTreeMap;

use engine::data::{Group, Position, generate_league};

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
        // A generated file passes the same validation a hand-written one must pass.
        garde::Validate::validate_with(team, &content.attributes)
            .unwrap_or_else(|r| panic!("club {} invalid: {r}", team.club.id));
    }
}

#[test]
fn per_position_means_fall_inside_the_tuning_bounds() {
    let content = common::content();
    let league = generate_league(7, 20, &content);
    let mut sums: BTreeMap<(Position, Group), (f64, u32)> = BTreeMap::new();
    for team in &league {
        for p in &team.players {
            for def in &content.attributes.attributes {
                let v = f64::from(p.attributes[&def.name]);
                let e = sums.entry((p.position, def.group)).or_insert((0.0, 0));
                e.0 += v;
                e.1 += 1;
            }
        }
    }
    for ((position, group), (sum, n)) in sums {
        let dist = content.tuning.generator.per_position[position.code()].get(group);
        let mean = sum / f64::from(n);
        let lo = dist.mean - dist.spread;
        let hi = dist.mean + dist.spread;
        assert!(
            (lo..=hi).contains(&mean),
            "{} {group:?}: sample mean {mean:.2} over {n} outside {lo} to {hi}",
            position.code()
        );
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
