//! The weaker foot in play. A kick to the side of a one-footed player's weaker foot, pressed
//! and at range, costs at most 5 rating points and makes the kick less accurate; on his strong
//! side, or with time to move the ball onto his strong foot, it costs nothing, and a placed
//! penalty never pays. A defender closing a one-footed carrier stands toward his strong side,
//! showing him onto his weaker foot. With the foot block out of the tuning, nothing pays and
//! nobody shows.
//!
//! Every scene is deterministic: it reads the decision maker's functions with fixed draws.

mod common;

use common::{calm_match, index, spread};
use engine::data::team::Side;
use engine::math::{DVec2, DVec3};
use engine::modules::{ShotDraws, ShotKind, TargetsTrace};
use engine::scenario::Scene;
use engine::{MatchConfig, Simulation};

/// The carrier: home slot 6, at the centre spot, running along +x (facing +x, so +y is his
/// left).
const AT: DVec2 = DVec2::new(0.0, 0.0);
const TICK: u32 = 1_000;

fn carrier() -> usize {
    index(0, 6)
}

/// The angle in radians between two directions.
fn angle(a: DVec2, b: DVec2) -> f64 {
    engine::math::atan2(a.perp_dot(b), a.dot(b)).abs()
}

/// A scene with both sides far away, the carrier on the ball at the centre running along +x
/// with feet `strong` / `weaker`, a team-mate (home slot 7) at `mate`, and the nearest away
/// player (away slot 6) at `opponent`. He gained the ball `held` ticks ago.
fn pass_scene(
    config: MatchConfig,
    strong: Side,
    weaker: u8,
    mate: DVec2,
    opponent: DVec2,
    held: u32,
) -> Simulation {
    let c = carrier();
    spread(Scene::new(config), -45.0, 45.0)
        .place(c, AT)
        .velocity(c, DVec2::new(3.0, 0.0))
        .place(index(0, 7), mate)
        .place(index(1, 6), opponent)
        .ball(DVec3::new(AT.x, AT.y, 0.0))
        .carrier(Some(c))
        .tick(TICK)
        .control_since(TICK - held)
        .foot(c, strong, weaker)
        .build()
}

/// The weaker-foot cost of a pass from the carrier to the team-mate, in rating points.
fn pass_cost(sim: &Simulation) -> f64 {
    let (c, j) = (carrier(), index(0, 7));
    let view = sim.view();
    let (from, to) = (view.player(c).pos, view.player(j).pos);
    engine::decision::foot_cost(
        &view,
        c,
        engine::math::toward(from, to),
        (to - from).length(),
        false,
    )
}

/// How far the pass leaves its line at the widest aim draw: its aim noise, which falls as the
/// kick's execution share rises.
fn pass_miss(sim: &Simulation) -> f64 {
    let (c, j) = (carrier(), index(0, 7));
    let view = sim.view();
    let kick = sim.config().modules.decision.pass_kick(&view, c, j, 1.0);
    let line = engine::math::toward(view.player(c).pos, view.player(j).pos);
    angle(kick.flight().0, line)
}

/// 35 m to the carrier's left (his weaker side as a right-footer) or right.
const LEFT_35: DVec2 = DVec2::new(0.0, 35.0);
const RIGHT_35: DVec2 = DVec2::new(0.0, -35.0);
/// An opponent 1 m behind the carrier; one 10 m away does not press him.
const CLOSE: DVec2 = DVec2::new(-1.0, 0.0);
const FAR: DVec2 = DVec2::new(-10.0, 0.0);

/// Ticks of holding the ball longer than the shipped shift time by 0.2 s.
fn time_to_shift(config: &MatchConfig) -> u32 {
    let foot = config.tuning.contract.foot();
    ((foot.shift_s + 0.2) / config.tuning.dt).ceil() as u32
}

/// A one-footed player (weaker foot 20) passing 35 m on his weaker side, pressed at
/// 1 m, pays more than 0 and at most 5 points, and his pass is less accurate than the same
/// pass on his strong side, which pays 0; the same weaker-side pass unpressed after holding
/// the ball long enough to shift it pays 0.
#[test]
fn a_one_footed_pass_on_his_weaker_side_pressed_at_range_pays_at_most_five_points() {
    let config = calm_match(90);
    let weak = pass_scene(config.clone(), Side::Right, 20, LEFT_35, CLOSE, 0);
    let strong = pass_scene(config.clone(), Side::Right, 20, RIGHT_35, CLOSE, 0);
    let cost = pass_cost(&weak);
    println!("weaker side, pressed at 1 m, 35 m: {cost:.3} points");
    assert!(cost > 0.0 && cost <= 5.0, "{cost}");
    assert_eq!(pass_cost(&strong), 0.0);
    // The field changes the kick's outcome: the weaker-side pass misses its line by more.
    assert!(
        pass_miss(&weak) > pass_miss(&strong) + 1e-6,
        "{} {}",
        pass_miss(&weak),
        pass_miss(&strong)
    );
    // A two-footed player's identical pass misses by exactly the strong side's.
    let two = pass_scene(config.clone(), Side::Right, 100, LEFT_35, CLOSE, 0);
    assert_eq!(pass_cost(&two), 0.0);
    assert_eq!(pass_miss(&two).to_bits(), pass_miss(&strong).to_bits());

    // With time to shift: unpressed, held 0.2 s past the shift time.
    let held = time_to_shift(&config);
    let calm = pass_scene(config.clone(), Side::Right, 20, LEFT_35, FAR, held);
    assert_eq!(pass_cost(&calm), 0.0);
    // Unpressed but a first-time kick: no time, so a cost (from the base share and range).
    let first_time = pass_scene(config.clone(), Side::Right, 20, LEFT_35, FAR, 0);
    assert!(pass_cost(&first_time) > 0.0);
    // A left-footer mirrors him.
    let lefty = pass_scene(config, Side::Left, 20, RIGHT_35, CLOSE, 0);
    assert_eq!(pass_cost(&lefty).to_bits(), cost.to_bits());
}

/// The gap grows with pressure and with distance.
#[test]
fn the_weaker_foot_costs_more_under_closer_pressure_and_over_longer_range() {
    let config = calm_match(90);
    let at = |mate: DVec2, opp: DVec2| {
        pass_cost(&pass_scene(config.clone(), Side::Right, 20, mate, opp, 0))
    };
    let (near, loose) = (at(LEFT_35, CLOSE), at(LEFT_35, DVec2::new(-4.0, 0.0)));
    assert!(near > loose, "{near} {loose}");
    let (long, short) = (at(LEFT_35, CLOSE), at(DVec2::new(0.0, 10.0), CLOSE));
    assert!(long > short, "{long} {short}");
}

/// A shot by a left-footer facing up the pitch's width, with the goal on his weaker (right)
/// side and an opponent 1 m away: an open shot pays and strikes wider; a penalty from the
/// same spot is a placed dead ball and pays nothing, so it strikes exactly as a two-footed
/// taker's.
#[test]
fn a_penalty_never_pays_the_weaker_foot_cost() {
    let taker = index(0, 9);
    let spot = DVec2::new(41.5, 0.0);
    let shot = |weaker: u8, kind: ShotKind| {
        let sim = spread(Scene::new(calm_match(90)), -45.0, 45.0)
            .place(taker, spot)
            .velocity(taker, DVec2::new(0.0, 3.0))
            .place(index(1, 6), spot + DVec2::new(0.0, -1.0))
            .ball(DVec3::new(spot.x, spot.y, 0.0))
            .carrier(Some(taker))
            .tick(TICK)
            .control_since(TICK)
            .foot(taker, Side::Left, weaker)
            .build();
        let draws = ShotDraws {
            side: None,
            aim: 0.5,
            spread: 1.0,
            loft: 0.0,
        };
        let goal = sim.teams()[0].target_goal();
        let view = sim.view();
        let cost = engine::decision::foot_cost(
            &view,
            taker,
            engine::math::toward(spot, goal),
            (goal - spot).length(),
            kind == ShotKind::Penalty,
        );
        let kick = sim.config().modules.decision.shot_kick(
            &view,
            taker,
            goal,
            sim.keeper(1),
            kind,
            &draws,
        );
        (cost, kick.flight().0)
    };
    let (open_cost, open_one) = shot(20, ShotKind::Open);
    let (_, open_two) = shot(100, ShotKind::Open);
    assert!(open_cost > 0.0 && open_cost <= 5.0, "{open_cost}");
    assert_ne!(open_one, open_two, "the open shot pays");
    let (pen_cost, pen_one) = shot(20, ShotKind::Penalty);
    let (_, pen_two) = shot(100, ShotKind::Penalty);
    assert_eq!(pen_cost, 0.0);
    assert_eq!(pen_one, pen_two);
}

/// The target of the away player (slot 6) closing the carrier from 8 m ahead.
fn presser_target(config: MatchConfig, strong: Side, weaker: u8) -> DVec2 {
    let c = carrier();
    let presser = index(1, 6);
    let sim = spread(Scene::new(config), -45.0, 45.0)
        .place(c, AT)
        .velocity(c, DVec2::new(4.0, 0.0))
        .place(presser, DVec2::new(8.0, 0.0))
        .ball(DVec3::new(AT.x, AT.y, 0.0))
        .carrier(Some(c))
        .tick(TICK)
        .control_since(TICK)
        .foot(c, strong, weaker)
        .build();
    let t = sim.config().modules.decision.targets(&sim.view());
    match t.trace {
        TargetsTrace::Carrier { pressers, .. } => assert!(
            pressers.iter().any(|&(_, i)| i == presser),
            "slot 6 presses: {pressers:?}"
        ),
        other => panic!("no carrier: {other:?}"),
    }
    t.targets[presser]
}

/// A defender closing a one-footed dribbler (right strong, weaker 20) stands toward
/// his strong side (his right, −y) by at least half of `show_m · 0.8`; a left-footer
/// mirrors it; a two-footed dribbler gives no offset; with the foot block out of the tuning
/// no carrier does.
#[test]
fn a_closing_defender_shows_a_one_footed_dribbler_onto_his_weaker_foot() {
    let config = calm_match(90);
    let show = config.tuning.contract.foot().show_m;
    assert!(show > 0.0);
    let two = presser_target(config.clone(), Side::Right, 100);
    let right = presser_target(config.clone(), Side::Right, 20);
    let left = presser_target(config.clone(), Side::Left, 20);
    println!("presser target y: two-footed {two}, right-footer {right}, left-footer {left}");
    assert!(right.y - two.y <= -0.5 * show * 0.8, "{right} {two}");
    assert!(left.y - two.y >= 0.5 * show * 0.8, "{left} {two}");
    assert!((right.x - two.x).abs() < 1e-9 && (left.x - two.x).abs() < 1e-9);

    let mut off = config;
    off.tuning.contract.foot = None;
    let plain = presser_target(off.clone(), Side::Right, 100);
    assert_eq!(presser_target(off.clone(), Side::Right, 20), plain);
    assert_eq!(plain, two, "a two-footed carrier is shown nowhere");
    // And no kick pays.
    let weak = pass_scene(off, Side::Right, 20, LEFT_35, CLOSE, 0);
    assert_eq!(pass_cost(&weak), 0.0);
}
