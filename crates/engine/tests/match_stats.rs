//! The match figures the realism bands read: shots and their expected goals, passes and
//! completed passes, and each team's share of open play.

mod common;

use common::{calm_match, index, spread};
use engine::math::{DVec2, DVec3};
use engine::observe::MatchFigures;
use engine::record::NullSink;
use engine::scenario::Scene;
use engine::sim::shot_xg;
use engine::{Simulation, Tuning};

#[test]
fn a_scripted_shot_counts_once_with_an_expected_goal_between_zero_and_one() {
    let probe = Scene::new(calm_match(90)).build();
    let attack_x = probe.teams()[0].attack_x;
    let shooter = index(0, 9);
    let from = DVec2::new(30.0 * attack_x, 4.0);
    let mut sim = spread(Scene::new(calm_match(90)), -30.0 * attack_x, 0.0)
        .place(shooter, from)
        .carrier(Some(shooter))
        .ball(DVec3::new(from.x, from.y, 0.0))
        .tick(1_001)
        .shoot(DVec2::new(attack_x, 0.0), 25.0, 0.0)
        .build();
    let s = sim.summary();
    assert_eq!(s.shots, [1, 0]);
    assert_eq!(s.passes, [0, 0]);
    assert!(s.xg[0] > 0.0 && s.xg[0] < 1.0, "xG {}", s.xg[0]);
    assert_eq!(s.xg[1], 0.0);
    // The shot is resolved as a goal, a save, or a miss; none of them counts a second shot.
    for _ in 0..200 {
        sim.step();
    }
    assert_eq!(sim.summary().shots, [1, 0]);
    assert!(sim.summary().shots_on_target[0] <= 1);
}

#[test]
fn expected_goals_fall_with_distance_and_rise_with_the_angle() {
    let t = Tuning::default().xg;
    let near = shot_xg(DVec2::new(40.0, 0.0), 1.0, &t);
    let far = shot_xg(DVec2::new(10.0, 0.0), 1.0, &t);
    let wide = shot_xg(DVec2::new(40.0, 25.0), 1.0, &t);
    assert!(near > far, "{near} <= {far}");
    assert!(near > wide, "{near} <= {wide}");
    // Mirrored ends give the same figure.
    let mirrored = shot_xg(DVec2::new(-40.0, 0.0), -1.0, &t);
    assert!((near - mirrored).abs() < 1e-12);
}

#[test]
fn a_pass_received_by_a_team_mate_counts_as_completed() {
    let passer = index(0, 9);
    let receiver = index(0, 10);
    let mut sim = spread(Scene::new(calm_match(90)), -30.0, 30.0)
        .place(passer, DVec2::new(0.0, 10.0))
        .place(receiver, DVec2::new(0.0, 15.0))
        .carrier(Some(passer))
        .ball(DVec3::new(0.0, 11.2, 0.0))
        .tick(1_001)
        .kick(DVec2::Y, 7.0, 0.0)
        .build();
    assert_eq!(sim.summary().passes, [1, 0]);
    assert_eq!(sim.summary().passes_completed, [0, 0]);
    for _ in 0..200 {
        sim.step();
        if sim.carrier().is_some() {
            break;
        }
    }
    assert_eq!(sim.carrier(), Some(receiver));
    assert_eq!(sim.summary().passes_completed, [1, 0]);
}

#[test]
fn the_two_possession_shares_of_a_full_match_sum_to_one_hundred() {
    let mut sim = Simulation::new(common::full_match()).unwrap();
    sim.run(&mut NullSink).unwrap();
    let s = sim.summary();
    let figures = MatchFigures::new(&s, sim.managers());
    let total = figures.possession_pct[0] + figures.possession_pct[1];
    assert!((total - 100.0).abs() <= 0.1, "{:?}", figures.possession_pct);
    for team in 0..2 {
        assert!(
            s.possession_ticks[team] > 0,
            "team {team} never had the ball"
        );
        assert!(s.passes_completed[team] <= s.passes[team]);
        assert!(s.shots_on_target[team] <= s.shots[team]);
    }
    assert_eq!(figures.manager_kind, ["ai".to_string(), "ai".to_string()]);
}
