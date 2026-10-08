//! A lone forward. A carrier with no active outfield team-mate ahead of him weighs a lay-off
//! to a team-mate behind him, and, when he is pressed, holding the ball against a dribble;
//! off the ball, a team's single forward holds the onside line. A defender in contact wins
//! the ball cleanly below the win chance and fouls in the band above it.

mod common;

use common::{content, index, quiet_match, spread};
use engine::math::{DVec2, DVec3};
use engine::rules::fouls::{foul_chance, win_chance};
use engine::scenario::Scene;
use engine::{Content, EngineEventKind, MatchConfig, Simulation, Tactics};

/// The home 4-4-1-1's striker and a central midfielder.
const STRIKER: usize = 10;
const MIDFIELDER: usize = 7;

/// A home side playing `formation` from the tactics file.
fn tactics(content: &Content, formation: &str) -> Tactics {
    let schema = &content.tactics;
    let f = schema.formation_index(formation).unwrap();
    let mut t = Tactics::defaults(schema);
    t.set_formation(f as u8, schema);
    t
}

/// A 90-minute match on seed `seed` with decisions every tick, no injuries, and the home
/// side in 4-4-1-1.
fn live(content: &Content, seed: u64) -> MatchConfig {
    let mut config = common::short_match(90);
    config.seed = seed;
    config.tuning.injury_per_minute = 0.0;
    config.tuning.injury_per_tackle = 0.0;
    config.with_tactics(0, tactics(content, "4-4-1-1"))
}

/// `live` with the shipped lone-carrier weights, which the scenes need switched on.
fn weighted(content: &Content, seed: u64) -> MatchConfig {
    let config = live(content, seed);
    let w = &config.tuning.decision;
    assert!(
        w.lone_hold > 0.0 || w.lone_dribble < 0.0,
        "the shipped weights favour holding over dribbling"
    );
    config
}

/// The home striker holds the ball at (30, 0) with an away presser 1.5 m in front of him and
/// a home midfielder open 12 m behind him. Every other player stands far away. With
/// `partner`, a second home forward stands 5 m ahead of the striker.
fn pressed_striker(config: MatchConfig, partner: bool) -> Simulation {
    let striker = index(0, STRIKER);
    let at = DVec2::new(30.0, 0.0);
    let mut scene = spread(Scene::new(config), -30.0, 45.0)
        .place(striker, at)
        .place(index(0, MIDFIELDER), DVec2::new(18.0, 0.0))
        .place(index(1, 9), at + DVec2::new(1.5, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(striker))
        .tick(1_001);
    if partner {
        scene = scene.place(index(0, 9), DVec2::new(35.0, -6.0));
    }
    scene.build()
}

#[test]
fn a_pressed_lone_forward_passes_or_holds_instead_of_dribbling() {
    let content = content();
    let striker = index(0, STRIKER);
    for seed in 1..=40 {
        let mut sim = pressed_striker(weighted(&content, seed), false);
        let before = sim.players()[striker].pos;
        sim.step();
        let ball = sim.record().ball;
        let passed = sim.carrier().is_none() && f64::from(ball[0]) < before.x;
        let target = sim.players()[striker].target;
        let held = sim.carrier() == Some(striker) && (target - before).length() <= 0.5;
        assert!(
            passed || held,
            "seed {seed}: carrier {:?}, ball {ball:?}, target {target}",
            sim.carrier()
        );
    }
}

#[test]
fn the_lone_terms_need_a_lone_carrier() {
    let content = content();
    for seed in 1..=40 {
        let tuned = weighted(&content, seed);
        let mut neutral = tuned.clone();
        neutral.tuning.decision.lone_layoff = 0.0;
        neutral.tuning.decision.lone_hold = 0.0;
        neutral.tuning.decision.lone_dribble = 0.0;
        let [a, b] = [tuned, neutral].map(|config| {
            let mut sim = pressed_striker(config, true);
            sim.step();
            (
                sim.record(),
                sim.carrier(),
                sim.players()[index(0, STRIKER)].target,
            )
        });
        assert_eq!(a, b, "seed {seed}");
    }
}

#[test]
fn the_lone_forward_holds_the_onside_line_while_his_side_has_the_ball() {
    let content = content();
    let striker = index(0, STRIKER);
    let carrier = index(0, MIDFIELDER);
    // The away back line stands at x = 5; its keeper is on the goal line.
    let line = 5.0;
    for share in [content.tuning.engine.lone_line_hold, 1.0] {
        let mut config = live(&content, 7);
        config.tuning.lone_line_hold = share;
        let mut scene = spread(Scene::new(config), -30.0, line)
            .place(carrier, DVec2::new(-20.0, 0.0))
            .ball(DVec3::new(-20.0, 0.0, 0.0))
            .carrier(Some(carrier))
            .tick(1_001);
        // The striker starts well past the line, where his anchor also is.
        scene = scene.place(striker, DVec2::new(15.0, 0.0));
        let mut sim = scene.build();
        let anchor = sim.teams()[0].anchor(STRIKER, DVec2::new(-20.0, 0.0), false, sim.tuning());
        assert!(anchor.x > line, "the anchor {anchor} is past the line");
        sim.step();
        let target = sim.players()[striker].target;
        if share > 0.0 {
            assert!(
                target.x <= line - 0.5 + 1e-9,
                "share {share}: target {target}"
            );
            assert_eq!(
                target.y, anchor.y,
                "share {share}: he keeps his place across"
            );
        } else {
            assert_eq!(target, anchor, "at share 0 he holds his anchor");
        }
    }
}

/// The home carrier holds the ball at `at` and an away defender stands 0.3 m from it. The
/// first step judges the contact with `draw`.
fn contact(draw: impl Fn(f64, f64) -> f64) -> (Simulation, Vec<engine::EngineEvent>) {
    const CARRIER: usize = 5;
    const TACKLER: usize = 16;
    let config = quiet_match(90);
    let plain = Simulation::new(config.clone()).unwrap();
    let tackler = plain.skills(TACKLER);
    let carrier = plain.skills(CARRIER);
    let p_win = win_chance(tackler, carrier, &config.tuning);
    let p_foul = foul_chance(tackler, 0, &config.tuning);
    let at = DVec2::new(0.0, 10.0);
    let mut sim = spread(Scene::new(config), -30.0, 30.0)
        .place(CARRIER, at)
        .place(TACKLER, at + DVec2::new(0.3, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001)
        .rolls(&[draw(p_win, p_foul), 0.99])
        .build();
    sim.step();
    let events = sim.take_events();
    (sim, events)
}

#[test]
fn a_contact_below_the_win_chance_wins_the_ball_and_above_it_is_a_foul() {
    let (sim, events) = contact(|w, _| w * 0.999);
    assert!(events.iter().all(|e| e.kind != EngineEventKind::Foul));
    let c = sim.carrier().expect("the tackler carries");
    assert_eq!(sim.players()[c].team, 1);

    let (_, events) = contact(|w, f| w + 0.5 * f);
    assert!(
        events.iter().any(|e| e.kind == EngineEventKind::Foul),
        "{events:?}"
    );
}
