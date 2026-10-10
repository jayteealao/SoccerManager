//! The reader table: every new attribute, and agility and communication, names the engine
//! function that reads it, and a scripted scene shows that changing it on one player from 5.0
//! to 18.0 moves the decision or the outcome that function sets, by at least the row's least
//! move. The three personality values inert in a match change nothing at all. A value with
//! neither a row nor a place in the pending list fails the table, and so does a pending value
//! that gains a row.
//!
//! A row reads one quantity of its hook for the player at a rating: a distance, an angle, a
//! chance, a score or a range. Each scene is deterministic: it reads the decision maker's
//! functions with fixed draws, so no row depends on a seed.

mod common;

use common::{calm_match, index, spread};
use engine::contract::stages::Blend;
use engine::contract::{ActionKind, Stage, StageKind};
use engine::data::rules::StoppageKind;
use engine::math::{DVec2, DVec3};
use engine::modules::{CoverTrace, ShotDraws, ShotKind, TargetsTrace};
use engine::player::Derived;
use engine::scenario::Scene;
use engine::{Rating, Simulation};

/// The direction a row's quantity moves as the rating rises.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Dir {
    Up,
    Down,
}

/// One row of the table.
struct Row {
    /// The attribute.
    field: &'static str,
    /// The engine function that reads it, as a path.
    reader: &'static str,
    /// What the scene measures.
    quantity: &'static str,
    dir: Dir,
    /// The least relative move from 5.0 to 18.0, in the row's direction.
    least: f64,
    /// Why a least move is below 10 percent; `None` at 10 percent.
    reason: Option<&'static str>,
    scene: fn(f64) -> f64,
}

/// The values whose occasions the mood model, the familiarity input and the season goal read;
/// no match reader exists for them yet.
///
/// sdlc-debt: the mood model, the familiarity input and the season goal give these values
/// their readers and delete this list; the test fails when a listed value gains a row, so the
/// list can only shrink.
const AWAITS_READER: [&str; 8] = [
    "professionalism",
    "temperament",
    "pressure_handling",
    "sportsmanship",
    "leadership",
    "adaptability",
    "versatility",
    "ambition",
];

/// The personality values inert in a match.
const INERT: [&str; 3] = ["loyalty", "money_motivation", "controversy"];

/// Both sides placed far from the scene: home outfield on `x = -45`, away on `x = 45`.
fn far() -> Scene {
    spread(Scene::new(calm_match(90)), -45.0, 45.0)
}

/// The relative move from `low` to `high` in `dir`, as the sensitivity rules read it.
fn relative(low: f64, high: f64, dir: Dir) -> f64 {
    let sign = if dir == Dir::Up { 1.0 } else { -1.0 };
    let base = if low.abs() > 1e-12 {
        low.abs()
    } else {
        high.abs().max(1e-12)
    };
    sign * (high - low) / base
}

/// The angle in radians between two directions.
fn angle(a: DVec2, b: DVec2) -> f64 {
    engine::math::atan2(a.perp_dot(b), a.dot(b)).abs()
}

// --- Scenes.

/// Marking: the home striker carries at (20, 2) in the away half; the away back-line player
/// who covers him is rated `v`. The distance from his target to the carrier.
fn marking(v: f64) -> f64 {
    let carrier = index(0, 9);
    let at = DVec2::new(20.0, 2.0);
    let scene = |rated: Option<(usize, f64)>| {
        let mut s = far()
            .place(carrier, at)
            .ball(DVec3::new(at.x, at.y, 0.0))
            .carrier(Some(carrier))
            .tick(1_000);
        if let Some((i, v)) = rated {
            s = s.rating(i, "marking", v);
        }
        s.build()
    };
    let covering = |sim: &Simulation| {
        let t = sim.config().modules.decision.targets(&sim.view());
        match t.trace {
            TargetsTrace::Carrier {
                cover: CoverTrace::Covered { defender, .. },
                ..
            } => (defender, t.targets[defender]),
            other => panic!("no cover: {other:?}"),
        }
    };
    let (defender, _) = covering(&scene(None));
    let sim = scene(Some((defender, v)));
    let (again, target) = covering(&sim);
    assert_eq!(again, defender, "the same defender covers");
    (target - at).length()
}

/// Off-ball movement: the home side has the ball at (-5, 0), the away outfield stands on
/// x = 0, and the home striker (rated `v`) has his target beyond the onside line. How far his
/// target lies beyond that line.
fn off_ball_movement(v: f64) -> f64 {
    let carrier = index(0, 6);
    let runner = index(0, 9);
    let at = DVec2::new(-5.0, 0.0);
    let sim = spread(Scene::new(calm_match(90)), -45.0, 0.0)
        .place(carrier, at)
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(carrier))
        .tick(1_000)
        .rating(runner, "off_ball_movement", v)
        .build();
    let t = sim.config().modules.decision.targets(&sim.view());
    let line = engine::rules::offside::second_last_depth(0, sim.players(), 1.0) - 0.5;
    (t.targets[runner].x - line).max(0.0)
}

/// A restart of `kind` (`direct` for a direct free kick) taken by the home player 7, rated
/// `v` in `attribute`, at `spot`, with home players 9 and 10 in the away penalty area: how far
/// the delivery turns from its aim at the widest aim draw.
fn delivery(attribute: &str, v: f64, kind: StoppageKind, direct: bool, spot: DVec2) -> f64 {
    let taker = index(0, 7);
    let sim = far()
        .place(taker, spot)
        .place(index(0, 9), DVec2::new(45.0, 2.0))
        .place(index(0, 10), DVec2::new(44.0, -4.0))
        .rating(taker, attribute, v)
        .build();
    let view = sim.view();
    let d = sim.config().modules.decision;
    let aimed = d
        .restart_pass(&view, taker, kind, direct, 0.5)
        .kick
        .flight()
        .0;
    let wide = d
        .restart_pass(&view, taker, kind, direct, 1.0)
        .kick
        .flight()
        .0;
    angle(aimed, wide)
}

fn corners(v: f64) -> f64 {
    delivery(
        "corners",
        v,
        StoppageKind::Corner,
        false,
        DVec2::new(52.0, 33.5),
    )
}

fn free_kick_delivery(v: f64) -> f64 {
    delivery(
        "free_kicks",
        v,
        StoppageKind::FreeKick,
        true,
        DVec2::new(10.0, 20.0),
    )
}

/// A shot of `kind` by the home striker, rated `v` in `attribute`, from `from` at the away
/// goal: how far the widest spread draw turns it from the middle one.
fn strike(attribute: &str, v: f64, kind: ShotKind, from: DVec2) -> f64 {
    let taker = index(0, 9);
    let sim = far().place(taker, from).rating(taker, attribute, v).build();
    let view = sim.view();
    let d = sim.config().modules.decision;
    let goal = sim.teams()[0].target_goal();
    let keeper = sim.keeper(1);
    let kick = |spread: f64| {
        let draws = ShotDraws {
            side: None,
            aim: 0.5,
            spread,
            loft: 0.0,
        };
        d.shot_kick(&view, taker, goal, keeper, kind, &draws)
            .flight()
            .0
    };
    angle(kick(0.5), kick(1.0))
}

fn free_kick_shot(v: f64) -> f64 {
    strike("free_kicks", v, ShotKind::FreeKick, DVec2::new(30.0, 4.0))
}

fn penalties(v: f64) -> f64 {
    let spot = engine::pitch::Pitch::DEFAULT.penalty_spot(1.0);
    strike("penalties", v, ShotKind::Penalty, spot)
}

/// A throw-in by home player 2 at the touchline, with one team-mate 16.5 m away and nobody
/// else in range: how far the throw goes (to him when he is in range, else 10 m forward).
fn long_throws(v: f64) -> f64 {
    let taker = index(0, 2);
    let at = DVec2::new(0.0, -33.9);
    let sim = far()
        .place(taker, at)
        .place(index(0, 4), DVec2::new(0.0, -17.4))
        .rating(taker, "long_throws", v)
        .build();
    let pass = sim.config().modules.decision.restart_pass(
        &sim.view(),
        taker,
        StoppageKind::ThrowIn,
        false,
        0.5,
    );
    (pass.target - at).length()
}

/// The home midfielder carries at (0, 0), rated `v` in flair, with a team-mate 30 m ahead
/// and one behind him: his draft scores.
fn flair_draft(v: f64) -> engine::modules::OptionDraft {
    let carrier = index(0, 6);
    let sim = far()
        .place(carrier, DVec2::ZERO)
        .place(index(0, 9), DVec2::new(30.0, 5.0))
        .place(index(0, 4), DVec2::new(-8.0, 10.0))
        .ball(DVec3::ZERO)
        .carrier(Some(carrier))
        .tick(1_000)
        .rating(carrier, "flair", v)
        .build();
    sim.config().modules.decision.options(&sim.view(), carrier)
}

/// The score of the forward pass less the score of the backward one.
fn flair_pass(v: f64) -> f64 {
    let draft = flair_draft(v);
    let score = |j: usize| {
        draft.passes[..draft.pass_count]
            .iter()
            .find(|&&(m, _)| m == j)
            .map(|&(_, s)| s)
            .unwrap_or_else(|| panic!("no pass to {j}"))
    };
    score(index(0, 9)) - score(index(0, 4))
}

/// The dribble score.
fn flair_dribble(v: f64) -> f64 {
    flair_draft(v)
        .dribble_hold
        .and_then(|(d, _)| d)
        .expect("an unpressed carrier may dribble")
}

/// A carrier with technique 6.5, under the take-on's 7.0, rated `v` in flair: 1 when he may
/// try a take-on, else 0.
fn flair_gate(v: f64) -> f64 {
    let carrier = index(0, 6);
    let sim = far()
        .rating(carrier, "technique", 6.5)
        .rating(carrier, "flair", v)
        .build();
    f64::from(u8::from(sim.players()[carrier].derived.gates.take_on_try))
}

/// The away keeper on his line at (50, 0) and a still loose ball in front of him, rated `v`
/// in eccentricity: the farthest ball, in steps of 5 cm, he comes off his line for.
fn eccentricity(v: f64) -> f64 {
    let keeper = index(1, 0);
    let mut farthest: f64 = 0.0;
    for k in 0..500 {
        let d = 5.0 + 0.05 * f64::from(k);
        let sim = far()
            .place(keeper, DVec2::new(50.0, 0.0))
            .ball(DVec3::new(50.0 - d, 0.0, 0.0))
            .carrier(None)
            .tick(1_000)
            .rating(keeper, "eccentricity", v)
            .build();
        let t = sim.config().modules.decision.targets(&sim.view());
        if let TargetsTrace::Loose { keepers, .. } = t.trace
            && keepers[1] == Some(keeper)
        {
            farthest = d;
        }
    }
    farthest
}

/// The home striker carries at (0, 0); away midfielder 6, rated `v` in determination, stands
/// in front of him with the score `score`: the farthest distance, in steps of 10 cm, from
/// which he presses.
fn press_reach(v: f64, score: [u32; 2]) -> f64 {
    let carrier = index(0, 9);
    let presser = index(1, 6);
    let mut farthest: f64 = 0.0;
    for k in 0..400 {
        let d = 1.0 + 0.1 * f64::from(k);
        let sim = far()
            .place(carrier, DVec2::ZERO)
            .place(presser, DVec2::new(d, 0.0))
            .ball(DVec3::ZERO)
            .carrier(Some(carrier))
            .score(score)
            .tick(1_000)
            .rating(presser, "determination", v)
            .build();
        let t = sim.config().modules.decision.targets(&sim.view());
        if let TargetsTrace::Carrier {
            pressers, count, ..
        } = t.trace
            && pressers[..count].iter().any(|&(_, i)| i == presser)
        {
            farthest = d;
        }
    }
    farthest
}

/// While his side trails 0-1.
fn determination(v: f64) -> f64 {
    press_reach(v, [1, 0])
}

/// A still ball at head height at (10, 0) and two players under it: home striker 9 on it,
/// rated `v` in bravery, and away player 5 beside him. The striker's chance to win the header.
fn bravery_header(v: f64) -> f64 {
    let me = index(0, 9);
    let sim = far()
        .place(me, DVec2::new(10.0, 0.0))
        .place(index(1, 5), DVec2::new(10.4, 0.0))
        .ball(DVec3::new(10.0, 0.0, 2.0))
        .carrier(None)
        .rating(me, "bravery", v)
        .build();
    let loose = sim
        .config()
        .modules
        .possession
        .loose_ball(&sim.view())
        .expect("a ball within reach");
    assert_eq!(loose.best.map(|(_, i)| i), Some(me));
    loose.header.expect("a header")
}

/// Away player 5's reach to block a shot, rated `v` in bravery, as a factor on the average.
fn bravery_block(v: f64) -> f64 {
    let me = index(1, 5);
    far().rating(me, "bravery", v).build().players()[me]
        .derived
        .knobs
        .block
}

/// Away player 5's chance to win a standing tackle on an average carrier, rated `v` in
/// bravery.
fn bravery_tackle(v: f64) -> f64 {
    let me = index(1, 5);
    let sim = far().rating(me, "bravery", v).build();
    engine::rules::fouls::win_chance(sim.skills(me), sim.skills(index(0, 9)), sim.tuning())
}

/// The away side with one back-line player sent off, so its line is too wide and closes up;
/// another back-line player rated `v` in communication. The width of the back line.
fn communication(v: f64) -> f64 {
    let line = far().build().teams()[1].back_line();
    line_width(index(1, line[0]), index(1, line[1]), v)
}

/// As [`communication`], with the away keeper rated `v`.
fn keeper_communication(v: f64) -> f64 {
    let line = far().build().teams()[1].back_line();
    line_width(index(1, line[0]), index(1, 0), v)
}

/// The width of the away back line with player `off` sent off and player `rated` rated `v`
/// in communication.
fn line_width(off: usize, rated: usize, v: f64) -> f64 {
    let sim = far()
        .sent_off(off)
        .rating(rated, "communication", v)
        .build();
    let team = &sim.teams()[1];
    let ys: Vec<f64> = team
        .back_line()
        .into_iter()
        .filter(|&s| team.active[s])
        .map(|s| team.formation[s].1)
        .collect();
    let lo = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    hi - lo
}

/// Home player 5, rated `v` in agility, runs at 7 m/s and stops: the distance he covers.
fn agility(v: f64) -> f64 {
    let me = index(0, 5);
    let sim = far()
        .rating(me, "agility", v)
        .velocity(me, DVec2::new(7.0, 0.0))
        .build();
    let t = sim.tuning().clone();
    let mut players = sim.players().to_vec();
    let start = players[me].pos;
    for _ in 0..2_000 {
        players[me].target = players[me].pos;
        let vel = engine::steering::next_velocity(&players, me, &t);
        players[me].vel = vel;
        players[me].pos += vel * t.dt;
        if vel.length() < 0.05 {
            break;
        }
    }
    (players[me].pos - start).length()
}

fn rows() -> Vec<Row> {
    vec![
        Row {
            field: "marking",
            reader: "decision::cover",
            quantity: "distance from the covering defender's target to the carrier",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: marking,
        },
        Row {
            field: "off_ball_movement",
            reader: "decision::time_runs",
            quantity: "metres his target lies beyond the onside line",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: off_ball_movement,
        },
        Row {
            field: "corners",
            reader: "decision::DecisionV1::restart_pass",
            quantity: "turn of a corner delivery at the widest aim draw",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: corners,
        },
        Row {
            field: "free_kicks",
            reader: "decision::DecisionV1::shot_kick",
            quantity: "turn of a direct free kick's strike at the widest spread draw",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: free_kick_shot,
        },
        Row {
            field: "free_kicks",
            reader: "decision::DecisionV1::restart_pass",
            quantity: "turn of a direct free kick's delivery at the widest aim draw",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: free_kick_delivery,
        },
        Row {
            field: "penalties",
            reader: "decision::DecisionV1::shot_kick",
            quantity: "turn of a penalty at the widest spread draw",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: penalties,
        },
        Row {
            field: "long_throws",
            reader: "decision::DecisionV1::restart_pass",
            quantity: "length of a throw-in with a team-mate 16.5 m away",
            dir: Dir::Up,
            least: 0.10,
            reason: None,
            scene: long_throws,
        },
        Row {
            field: "flair",
            reader: "decision::DecisionV1::options",
            quantity: "score of a forward pass over a backward one",
            dir: Dir::Up,
            least: 0.10,
            reason: None,
            scene: flair_pass,
        },
        Row {
            field: "flair",
            reader: "decision::DecisionV1::options",
            quantity: "dribble score",
            dir: Dir::Up,
            least: 0.0,
            reason: Some(
                "flair is one of three shown supports of the dribble choice; the take-on gate \
                 and the pass score carry its larger effect",
            ),
            scene: flair_dribble,
        },
        Row {
            field: "flair",
            reader: "player::Derived::from_blend",
            quantity: "a 6.5-technique carrier may try a take-on",
            dir: Dir::Up,
            least: 0.10,
            reason: None,
            scene: flair_gate,
        },
        Row {
            field: "eccentricity",
            reader: "decision::keeper_chase_m",
            quantity: "farthest loose ball the keeper comes off his line for",
            dir: Dir::Up,
            least: 0.10,
            reason: None,
            scene: eccentricity,
        },
        Row {
            field: "determination",
            reader: "decision::DecisionV1::targets",
            quantity: "farthest distance he presses from while his side trails",
            dir: Dir::Up,
            least: 0.10,
            reason: None,
            scene: determination,
        },
        Row {
            field: "bravery",
            reader: "possession::loose_ball",
            quantity: "chance to win a header against an opponent",
            dir: Dir::Up,
            least: 0.10,
            reason: None,
            scene: bravery_header,
        },
        Row {
            field: "bravery",
            reader: "possession::blockers",
            quantity: "reach to block a shot",
            dir: Dir::Up,
            least: 0.005,
            reason: Some(
                "a small weight of the block stage, held below every shown support so the \
                 attribute hover stays as confirmed, on a factor of spread 0.2",
            ),
            scene: bravery_block,
        },
        Row {
            field: "bravery",
            reader: "rules::fouls::win_chance",
            quantity: "chance to win a standing tackle",
            dir: Dir::Up,
            least: 0.05,
            reason: Some(
                "a small weight of the tackle stage, held below every shown support so the \
                 attribute hover stays as confirmed",
            ),
            scene: bravery_tackle,
        },
        Row {
            field: "communication",
            reader: "team::Team::relayout",
            quantity: "width of a back line that lost a player",
            dir: Dir::Down,
            least: 0.01,
            reason: Some(
                "the line's organisation is shared by the keeper and the whole back line; one \
                 player of the line moves it by his share, on a factor of spread 0.2",
            ),
            scene: communication,
        },
        Row {
            field: "communication",
            reader: "team::Team::relayout",
            quantity: "width of a back line that lost a player, the keeper rated",
            dir: Dir::Down,
            least: 0.03,
            reason: Some(
                "the keeper carries half the line's organisation, on a factor of spread 0.2",
            ),
            scene: keeper_communication,
        },
        Row {
            field: "agility",
            reader: "steering::steer",
            quantity: "distance to stop from 7 m/s",
            dir: Dir::Down,
            least: 0.05,
            reason: Some(
                "braking follows the turn factor, whose spread every body knob shares (0.2)",
            ),
            scene: agility,
        },
    ]
}

/// Shape AC-10 (the attribute part) and AC-23: every row's hook moves by at least its least
/// move from 5.0 to 18.0.
#[test]
fn every_new_attribute_moves_its_hook() {
    let mut failed = Vec::new();
    for row in rows() {
        assert!(row.least >= 0.10 || row.reason.is_some(), "{}", row.field);
        let (low, high) = ((row.scene)(5.0), (row.scene)(18.0));
        let mv = relative(low, high, row.dir);
        println!(
            "{:<18} {:<36} {:<62} 5: {:>9.4}  18: {:>9.4}  move {:+.3} (least {:.3})",
            row.field, row.reader, row.quantity, low, high, mv, row.least
        );
        let moved = if row.least > 0.0 {
            mv >= row.least
        } else {
            mv > 0.0
        };
        if !moved {
            failed.push(format!("{} ({}): {mv:+.4}", row.field, row.quantity));
        }
    }
    assert!(failed.is_empty(), "hooks that move too little: {failed:?}");
}

/// Determination acts while his side trails: his reach moves more behind than level, where
/// only his small support of the press stage acts.
#[test]
fn determination_widens_the_press_only_while_behind() {
    let behind = relative(press_reach(5.0, [1, 0]), press_reach(18.0, [1, 0]), Dir::Up);
    let level = relative(press_reach(5.0, [0, 0]), press_reach(18.0, [0, 0]), Dir::Up);
    assert!(
        behind > level + 0.05,
        "behind {behind:+.3}, level {level:+.3}"
    );
}

/// Shape AC-18 in play: with form at +0.5, the shot (degree 1.3) rises more than the pass
/// (degree 0.9), each by its own degree, read through the blend a player's state rebuild uses.
#[test]
fn consistency_moves_each_action_by_its_degree() {
    let config = calm_match(90);
    let schema = &config.attributes;
    let flat = engine::player::Attributes {
        values: [Rating::from_tenths(100); engine::data::MAX_ATTRIBUTES],
        len: schema.len() as u8,
    };
    let blend = Blend::of(schema);
    let body = engine::contract::body::Body::default();
    let (_, still) = Derived::from_blend(&flat, &blend, schema, &config.tuning, body, 0);
    let (_, formed) = Derived::from_blend(&flat, &blend, schema, &config.tuning, body, 5);
    let width = config.tuning.contract.curve.width;
    for action in [ActionKind::Shot, ActionKind::Pass] {
        let s = Stage::find(action, StageKind::Execute).unwrap();
        let read = engine::math::ln(formed.f(s) / still.f(s)) * width / 0.5;
        let degree = schema.consistency_degrees[&action];
        assert!(
            (read - degree).abs() < 1e-9,
            "{action}: {read} against {degree}"
        );
    }
    let (shot, pass) = (
        Stage::find(ActionKind::Shot, StageKind::Execute).unwrap(),
        Stage::find(ActionKind::Pass, StageKind::Execute).unwrap(),
    );
    assert!(formed.f(shot) / still.f(shot) > formed.f(pass) / still.f(pass));
}

/// The inert values: a ten-minute match with home player 5 at 1.0 and at 20.0 in each of
/// loyalty, money motivation and controversy plays the same events and puts every player in
/// the same place on every tick.
#[test]
fn the_inert_values_change_nothing_in_a_match() {
    let me = index(0, 5);
    let play = |field: &str, v: f64| {
        let mut config = common::short_match(10);
        config.seed = 17;
        let mut sim = Scene::new(config).rating(me, field, v).build();
        let mut places = Vec::new();
        while !sim.is_over() {
            sim.step();
            places.extend(sim.players().iter().map(|p| (p.pos.x, p.pos.y)));
        }
        sim.finish();
        (sim.take_events(), places)
    };
    for field in INERT {
        let (low, high) = (play(field, 1.0), play(field, 20.0));
        assert_eq!(low.0, high.0, "{field}: the events differ");
        assert!(low.1 == high.1, "{field}: a player's place differs");
    }
}

/// Completeness: every attribute the version 4 catalogue added, communication and agility
/// has a row, is inert, or waits in the pending list; a pending value has no row.
#[test]
fn every_new_value_has_a_reader_or_waits_for_one() {
    let rows = rows();
    let content = common::content();
    let has_row = |name: &str| rows.iter().any(|r| r.field == name);
    let mut fields: Vec<&str> = engine::data::convert::V4_ADDED.to_vec();
    fields.extend(["communication", "agility"]);
    for name in fields {
        assert!(
            content.attributes.index(name).is_some(),
            "{name} is not in the catalogue"
        );
        let counted = [
            has_row(name),
            INERT.contains(&name),
            AWAITS_READER.contains(&name),
        ]
        .iter()
        .filter(|&&b| b)
        .count();
        assert_eq!(counted, 1, "{name}: a row, inert, or pending, exactly one");
    }
    for name in AWAITS_READER {
        assert!(
            !has_row(name),
            "{name} has a reader now; remove it from AWAITS_READER"
        );
    }
    for row in &rows {
        assert!(!row.reader.is_empty() && !row.quantity.is_empty());
    }
}
