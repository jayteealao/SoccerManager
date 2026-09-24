//! Defending as a team. A central attacker in the defending half draws a back-line defender
//! onto the line between him and the goal; a presser runs at the point where it meets a
//! running carrier; the back line stays compact at 11 against 11 and narrows as players
//! leave it; a team with a player out presses with one fewer.
//!
//! The criterion tests are slow: `cargo test --release -p engine --all-features -- --ignored`.
//! - A sending-off gives no advantage: 120 seeds, cards otherwise off, the away keeper,
//!   centre-back, or striker sent off at kick-off. The reduced side never outscores the full
//!   side on average, and the full side scores at most 1.6 times its no-send-off mean.
//! - Every shipped formation holds against 4-4-2 at 11 against 11: neither side averages more
//!   than 4.0 goals per match over 120 seeds.

mod common;

use common::{content, default_teams, index, run_many};
use engine::math::{DVec2, DVec3, segment_distance};
use engine::record::NullSink;
use engine::scenario::Scene;
use engine::{Content, MatchConfig, Simulation, Tactics, Validator, VecSink};

/// A 90-minute match on the shipped content with decisions every tick and no injuries.
fn live(seed: u64) -> MatchConfig {
    let mut config = common::short_match(90);
    config.seed = seed;
    config.tuning.injury_per_minute = 0.0;
    config.tuning.injury_per_tackle = 0.0;
    config
}

/// The home side's `formation` from the tactics file.
fn tactics(content: &Content, formation: &str) -> Tactics {
    let schema = &content.tactics;
    let f = schema
        .formation_index(formation)
        .unwrap_or_else(|| panic!("the tactics file has {formation}"));
    let mut t = Tactics::defaults(schema);
    t.set_formation(f as u8, schema);
    t
}

#[test]
fn a_central_attacker_in_the_defending_half_draws_a_back_line_defender_goal_side() {
    // The home side attacks positive x; the away side defends the goal at x = 52.5.
    let carrier = index(0, 5);
    let striker = index(0, 9);
    let at = DVec2::new(20.0, 2.0);
    let mut sim = Scene::new(live(3))
        .place(carrier, DVec2::new(0.0, -28.0))
        .ball(DVec3::new(0.0, -28.0, 0.0))
        .carrier(Some(carrier))
        .place(striker, at)
        .tick(1_000)
        .build();
    sim.step();
    let goal = DVec2::new(52.5, 0.0);
    let back_line = sim.teams()[1].back_line();
    let covering: Vec<usize> = back_line
        .iter()
        .map(|&s| index(1, s))
        .filter(|&i| {
            let target = sim.players()[i].target;
            segment_distance(target, at, goal) < 0.5 && target.x > at.x
        })
        .collect();
    assert_eq!(
        covering.len(),
        1,
        "one back-line defender between the striker and the goal: {:?}",
        back_line
            .iter()
            .map(|&s| sim.players()[index(1, s)].target)
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_running_carrier_draws_a_presser_to_a_point_ahead_of_him() {
    let carrier = index(0, 9);
    let from = DVec2::new(0.0, 0.0);
    let presser = index(1, 6);
    // Everyone else far from the ball, the presser 10 m ahead and to the side.
    let mut sim = common::spread(Scene::new(live(3)), -40.0, 40.0)
        .place(carrier, from)
        .velocity(carrier, DVec2::new(6.0, 0.0))
        .ball(DVec3::new(0.3, 0.0, 0.0))
        .carrier(Some(carrier))
        .place(presser, DVec2::new(10.0, 8.0))
        .tick(1_000)
        .build();
    sim.step();
    let target = sim.players()[presser].target;
    assert!(
        target.x > from.x + 1.0,
        "the presser aims ahead of the carrier, at {target}"
    );
}

#[test]
fn a_back_line_with_a_centre_back_sent_off_is_at_most_two_gaps_wide() {
    let sim = Scene::new(live(3)).sent_off(index(1, 2)).build();
    let team = &sim.teams()[1];
    let gap = sim.tuning().back_line_gap;
    let ys: Vec<f64> = team
        .back_line()
        .into_iter()
        .filter(|&s| team.active[s])
        .map(|s| team.formation[s].1)
        .collect();
    assert_eq!(ys.len(), 3);
    let lo = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(hi - lo <= 2.0 * gap + 1e-9, "the back line spans {ys:?}");
}

#[test]
fn a_team_with_one_player_out_presses_with_one_fewer() {
    let full = Simulation::new(live(3)).unwrap();
    let count = full.teams()[1].plan.press_count;
    let sim = Scene::new(live(3)).sent_off(index(1, 7)).build();
    assert_eq!(sim.teams()[1].pressers(), count.saturating_sub(1).max(1));
}

#[test]
fn a_back_four_behind_a_4_3_3_leaves_no_gap_for_a_central_striker() {
    let content = content();
    let config = live(3).with_tactics(1, tactics(&content, "4-3-3"));
    let striker = index(0, 9);
    let mut sim = Scene::new(config)
        .place(striker, DVec2::new(15.0, 0.0))
        .tick(1_000)
        .build();
    sim.step();
    let team = &sim.teams()[1];
    let gap = sim.tuning().back_line_gap;
    let mut ys: Vec<f64> = team
        .back_line()
        .into_iter()
        .map(|s| team.formation[s].1)
        .collect();
    ys.sort_by(f64::total_cmp);
    for w in ys.windows(2) {
        assert!(
            w[1] - w[0] <= gap + 1e-9,
            "a {} m gap in {ys:?}",
            w[1] - w[0]
        );
    }
}

/// Mean goals per match, home then away, over `seeds`, with `arrange` applied to each match.
fn mean_goals(
    seeds: std::ops::RangeInclusive<u64>,
    config: impl Fn(u64) -> MatchConfig + Sync,
    arrange: impl Fn(Scene) -> Scene + Sync,
) -> [f64; 2] {
    let goals = run_many(seeds, |seed| {
        let mut sim = arrange(Scene::new(config(seed))).build();
        sim.run(&mut NullSink).unwrap();
        sim.summary().goals
    });
    let n = goals.len() as f64;
    [0, 1].map(|t| goals.iter().map(|g| f64::from(g[t])).sum::<f64>() / n)
}

#[test]
fn the_public_send_off_before_kick_off_plays_as_the_scene_one_and_validates() {
    // A 15-minute match with cards otherwise off, as the sending-off experiment plays it.
    let content = content();
    let [a, b] = default_teams(&content);
    let cards_off = |seed: u64| {
        let mut config = MatchConfig::new(seed, 15, &content, [&a, &b]).unwrap();
        config.tuning.red_base = 0.0;
        config.tuning.yellow_base = 0.0;
        config.tuning.yellow_aggression_weight = 0.0;
        config
    };
    for seed in 1..=3 {
        for i in [11, 13, 21] {
            let mut scene = Scene::new(cards_off(seed)).sent_off(i).build();
            scene.run(&mut NullSink).unwrap();
            let mut sim = Simulation::new(cards_off(seed)).unwrap();
            sim.send_off_before_kickoff(i);
            let mut sink = VecSink::default();
            sim.run(&mut sink).unwrap();
            assert_eq!(
                scene.summary().goals,
                sim.summary().goals,
                "seed {seed}, player {i}"
            );
            let events = sim.take_events();
            let validator =
                Validator::for_match(sim.tuning().clone(), sim.team_timeline(), &events);
            let violations = validator.check(&sink.records);
            assert!(
                violations.is_empty(),
                "seed {seed}, player {i}: {} violations; first: {:?}",
                violations.len(),
                violations.first()
            );
        }
    }
}

#[test]
#[ignore = "slow: cargo test --release -p engine --all-features -- --ignored"]
fn a_sending_off_gives_no_advantage() {
    let content = content();
    let [a, b] = default_teams(&content);
    // Seeds 1 to 120, each in both home and away orders of the default clubs, so each club
    // is the reduced side in half the matches and club strength cancels out. Match `k`
    // plays seed `(k + 1) / 2`, with the clubs swapped on even `k`.
    let cards_off = |k: u64| {
        let teams = if k.is_multiple_of(2) {
            [&b, &a]
        } else {
            [&a, &b]
        };
        let mut config = MatchConfig::new(k.div_ceil(2), 90, &content, teams).unwrap();
        config.tuning.red_base = 0.0;
        config.tuning.yellow_base = 0.0;
        config.tuning.yellow_aggression_weight = 0.0;
        config
    };
    let control = mean_goals(1..=240, cards_off, |s| s);
    eprintln!("control: home {:.4} away {:.4}", control[0], control[1]);
    let mut failures = Vec::new();
    for (arm, i) in [("keeper", 11), ("centre-back", 13), ("striker", 21)] {
        let [full, reduced] = mean_goals(1..=240, cards_off, |s| s.sent_off(i));
        eprintln!(
            "{arm} sent off: full side {full:.4}, reduced side {reduced:.4}, limit {:.4}",
            1.6 * control[0]
        );
        if reduced > full {
            failures.push(format!(
                "{arm}: the reduced side scored {reduced:.2} against {full:.2}"
            ));
        }
        if full > 1.6 * control[0] {
            failures.push(format!(
                "{arm}: the full side scored {full:.2}, above 1.6 x {:.2}",
                control[0]
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
#[ignore = "slow: cargo test --release -p engine --all-features -- --ignored"]
fn every_formation_holds() {
    let content = content();
    let [a, b] = default_teams(&content);
    let mut failures = Vec::new();
    for formation in &content.tactics.formations {
        let name = formation.name.as_str();
        // [formation, 4-4-2] goals per match; the formation is at home on odd seeds.
        let goals = run_many(1..=120, |seed| {
            let side = usize::from(seed % 2 == 0);
            let config = MatchConfig::new(seed, 90, &content, [&a, &b])
                .unwrap()
                .with_tactics(side, tactics(&content, name));
            let mut sim = Simulation::new(config).unwrap();
            sim.run(&mut NullSink).unwrap();
            let g = sim.summary().goals;
            [g[side], g[1 - side]]
        });
        let n = goals.len() as f64;
        let [mine, theirs] = [0, 1].map(|k| goals.iter().map(|g| f64::from(g[k])).sum::<f64>() / n);
        eprintln!("{name} v 4-4-2: {mine:.2} - {theirs:.2}");
        if mine > 4.0 || theirs > 4.0 {
            failures.push(format!("{name} v 4-4-2: {mine:.2} - {theirs:.2}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
