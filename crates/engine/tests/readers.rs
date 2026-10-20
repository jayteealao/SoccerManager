//! The reader table: every new attribute and personality value, and agility and
//! communication, names the engine function that reads it, and a scripted scene shows that
//! changing it on one player from 5.0 to 18.0 moves the decision or the outcome that function
//! sets, by at least the row's least move. The three personality values inert in a match
//! change nothing at all. A value with neither a row nor a place in the inert list fails the
//! table.
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
/// and one behind him: his draft scores. Every mentality's loss weight is 0, so the scene reads
/// flair alone and not the team's appetite for risk.
fn flair_draft(v: f64) -> engine::modules::OptionDraft {
    let carrier = index(0, 6);
    let mut config = calm_match(90);
    for m in &mut config.tactics.mentalities {
        m.loss_weight = 0.0;
    }
    let tactics = engine::Tactics::defaults(&config.tactics);
    let config = config.with_tactics(0, tactics);
    let sim = spread(Scene::new(config), -45.0, 45.0)
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

// --- The personality occasions.

/// The home side has the ball; away defender 5, rated `v` in professionalism, at minute 10:
/// his chance of a concentration lapse this minute.
fn professionalism(v: f64) -> f64 {
    let me = index(1, 5);
    let carrier = index(0, 6);
    let sim = far()
        .carrier(Some(carrier))
        .rating(me, "professionalism", v)
        .tick(10 * engine::rules::clock::TICKS_PER_MINUTE)
        .build();
    engine::decision::lapse_chance(&sim.view(), me)
}

/// Home player 5, rated `v` in temperament: how far a mood event moves him (his swing).
fn temperament(v: f64) -> f64 {
    let me = index(0, 5);
    let sim = far().rating(me, "temperament", v).build();
    let m = sim.tuning().contract.person().mood;
    engine::sim::mood::swing(&sim.personality(me), &m)
}

fn pressure_penalty(v: f64) -> f64 {
    let spot = engine::pitch::Pitch::DEFAULT.penalty_spot(1.0);
    strike("pressure_handling", v, ShotKind::Penalty, spot)
}

/// Away player 5, rated `v` in sportsmanship, tackles the home striker running at 5 m/s: the
/// chance the tackle is a foul.
fn sportsmanship(v: f64) -> f64 {
    let tackler = index(1, 5);
    let carrier = index(0, 9);
    let at = DVec2::new(10.0, 2.0);
    let sim = far()
        .place(carrier, at)
        .velocity(carrier, DVec2::new(5.0, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(carrier))
        .place(tackler, DVec2::new(11.0, 2.0))
        .rating(tackler, "sportsmanship", v)
        .build();
    let fouls = sim.config().modules.fouls;
    fouls.tackle_chances(&sim.view(), tackler, carrier).p_foul
}

/// Home player 5, rated `v` in leadership, at a full mood, his team-mates at 0: his side's
/// mood.
/// Home player 5, rated `v` in ambition, in form at the neutral rating and ahead of his
/// season goal at 150 percent of the pace: his kick-off mood.
fn ambition(v: f64) -> f64 {
    let me = index(0, 5);
    let neutral = far().build().tuning().contract.season().form_neutral;
    let form = (neutral * 10.0).round() as u8;
    let sim = far()
        .rating(me, "ambition", v)
        .kick_off(me, Some(form), Some(150))
        .build();
    sim.mood(me)
}

fn leadership(v: f64) -> f64 {
    let me = index(0, 5);
    let sim = far()
        .rating(me, "leadership", v)
        .mood(me, engine::sim::mood::FULL)
        .build();
    sim.team_mood(0)
}

/// Home player 5, natural 16, plays a position he is rated 8 in, rated `v` in `attribute`:
/// the familiarity modifier's drop on his technical group, in rating points (a size).
fn familiarity(attribute: &str, v: f64) -> f64 {
    use engine::data::team::Position;
    use engine::modules::modifier::Modifier;
    let me = index(0, 5);
    let scene = far();
    let played = scene.played_position(me);
    let natural = if played == Position::ST {
        Position::CB
    } else {
        Position::ST
    };
    let sim = scene
        .position_rating(me, natural, 16.0)
        .position_rating(me, played, 8.0)
        .rating(me, attribute, v)
        .build();
    let effect = engine::modules::modifier::familiarity::FamiliarityV1.effect(&sim.view(), me);
    -effect.groups[0]
}

fn adaptability(v: f64) -> f64 {
    familiarity("adaptability", v)
}

fn versatility(v: f64) -> f64 {
    familiarity("versatility", v)
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
        Row {
            field: "professionalism",
            reader: "decision::lapse_chance",
            quantity: "a defender's chance of a concentration lapse in a minute",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: professionalism,
        },
        Row {
            field: "temperament",
            reader: "sim::mood::swing",
            quantity: "how far a mood event moves him",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: temperament,
        },
        Row {
            field: "pressure_handling",
            reader: "decision::DecisionV1::shot_kick",
            quantity: "turn of a penalty at the widest spread draw",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: pressure_penalty,
        },
        Row {
            field: "sportsmanship",
            reader: "rules::fouls::foul_occasion",
            quantity: "chance a tackle on a running carrier is a foul",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: sportsmanship,
        },
        Row {
            field: "leadership",
            reader: "sim::mood::pull",
            quantity: "his side's mood with him alone at a full mood",
            dir: Dir::Up,
            least: 0.10,
            reason: None,
            scene: leadership,
        },
        Row {
            field: "adaptability",
            reader: "modules::modifier::familiarity::FamiliarityV1",
            quantity: "technical drop away from his natural position",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: adaptability,
        },
        Row {
            field: "versatility",
            reader: "modules::modifier::familiarity::FamiliarityV1",
            quantity: "technical drop away from his natural position",
            dir: Dir::Down,
            least: 0.10,
            reason: None,
            scene: versatility,
        },
        Row {
            field: "ambition",
            reader: "sim::mood::kick_off_mood",
            quantity: "kick-off mood ahead of his season goal",
            dir: Dir::Up,
            least: 0.10,
            reason: None,
            scene: ambition,
        },
    ]
}

/// Every row's hook moves by at least its least
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
/// nothing of his determination acts (his job is an occasion, in no stage of the press).
#[test]
fn determination_widens_the_press_only_while_behind() {
    let behind = relative(press_reach(5.0, [1, 0]), press_reach(18.0, [1, 0]), Dir::Up);
    let level = relative(press_reach(5.0, [0, 0]), press_reach(18.0, [0, 0]), Dir::Up);
    assert!(
        behind > level + 0.05,
        "behind {behind:+.3}, level {level:+.3}"
    );
}

/// In play: with form at +0.5, the shot (degree 1.3) rises more than the pass
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
/// has a row or is inert, never both.
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
        assert!(
            has_row(name) != INERT.contains(&name),
            "{name}: a row or inert, exactly one"
        );
    }
    for row in &rows {
        assert!(!row.reader.is_empty() && !row.quantity.is_empty());
    }
}

// --- The traits of play.
//
// Every trait of the shipped catalogue names the engine function that reads it, and a
// scripted scene shows that carrying it moves the choice that function sets: strong by at
// least the row's least move over the same scene without the trait, and moderate strictly
// between the two.

use engine::data::traits::TraitStrength;
use engine::decision::{Choice, Options};
use engine::modules::CarrierPlan;

/// One trait row: the scene reads its quantity with the trait absent, moderate or strong.
struct TraitRow {
    name: &'static str,
    reader: &'static str,
    quantity: &'static str,
    dir: Dir,
    least: f64,
    /// Why moderate does not lie strictly between: a yes-or-no quantity.
    binary: Option<&'static str>,
    scene: fn(Option<TraitStrength>) -> f64,
}

/// `scene` with `who` carrying trait `name` at `s`.
fn traited(scene: Scene, who: usize, name: &str, s: Option<TraitStrength>) -> Scene {
    scene.with_trait(who, name, s)
}

/// The home midfielder carries at (0, 0) with every other player far away and the team-mates
/// `mates` placed; `name` carried at `s`; his draft.
fn carrier_draft(
    name: &str,
    s: Option<TraitStrength>,
    at: DVec2,
    mates: &[(usize, DVec2)],
    extra: impl FnOnce(Scene) -> Scene,
) -> engine::modules::OptionDraft {
    let carrier = index(0, 6);
    let mut scene = far()
        .place(carrier, at)
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(carrier))
        .tick(1_000);
    for &(i, p) in mates {
        scene = scene.place(i, p);
    }
    let sim = traited(extra(scene), carrier, name, s).build();
    sim.config().modules.decision.options(&sim.view(), carrier)
}

/// The draft's score of the pass to `j`.
fn pass_score(draft: &engine::modules::OptionDraft, j: usize) -> f64 {
    draft.passes[..draft.pass_count]
        .iter()
        .find(|&&(m, _)| m == j)
        .map(|&(_, s)| s)
        .unwrap_or_else(|| panic!("no pass to {j}"))
}

/// A wide carrier, home player 5 at (10, 25), dribbles with the nearest opponent at `opp`:
/// how far his carry's target lies from the centre line.
fn wide_carry(name: &str, s: Option<TraitStrength>, opp: DVec2) -> f64 {
    let carrier = index(0, 5);
    let at = DVec2::new(10.0, 25.0);
    let sim = traited(
        far()
            .place(carrier, at)
            .ball(DVec3::new(at.x, at.y, 0.0))
            .carrier(Some(carrier))
            .tick(1_000),
        carrier,
        name,
        s,
    )
    .build();
    let options = Options {
        shot: None,
        pass: None,
        dribble: Some(0.0),
        clear: 0.0,
        hold: None,
        nearest_opp_pos: opp,
    };
    match sim
        .config()
        .modules
        .decision
        .plan(&sim.view(), carrier, &options, Choice::Dribble)
    {
        CarrierPlan::Move { target } => target.y.abs(),
        other => panic!("a dribble moves: {other:?}"),
    }
}

/// With the nearest opponent inside him, he drifts toward the touchline.
fn cuts_inside(s: Option<TraitStrength>) -> f64 {
    wide_carry("cuts_inside", s, DVec2::new(12.0, 20.0))
}

/// With the nearest opponent outside him, he drifts toward the centre.
fn hugs_touchline(s: Option<TraitStrength>) -> f64 {
    wide_carry("hugs_touchline", s, DVec2::new(12.0, 30.0))
}

/// The home striker 20 m out, outside the penalty area: his shot score.
fn shoots_from_distance(s: Option<TraitStrength>) -> f64 {
    let draft = carrier_draft(
        "shoots_from_distance",
        s,
        DVec2::new(32.5, 0.0),
        &[],
        |sc| sc,
    );
    draft.shot.expect("inside the shooting range")
}

/// The home striker shoots from (40, 5) at the widest aim draw: the kick.
fn shot(name: &str, s: Option<TraitStrength>) -> engine::decision::Kick {
    let taker = index(0, 9);
    let sim = traited(far().place(taker, DVec2::new(40.0, 5.0)), taker, name, s).build();
    let draws = ShotDraws {
        side: None,
        aim: 1.0,
        spread: 0.5,
        loft: 0.0,
    };
    let goal = sim.teams()[0].target_goal();
    sim.config().modules.decision.shot_kick(
        &sim.view(),
        taker,
        goal,
        sim.keeper(1),
        ShotKind::Open,
        &draws,
    )
}

/// How far from the post the shot aims: its line crossed with the goal line.
fn places_shots(s: Option<TraitStrength>) -> f64 {
    let (dir, _, _) = shot("places_shots", s).flight();
    let from = DVec2::new(40.0, 5.0);
    let t = (52.5 - from.x) / dir.x;
    engine::pitch::GOAL_WIDTH / 2.0 - (from.y + dir.y * t).abs()
}

fn powers_shots(s: Option<TraitStrength>) -> f64 {
    shot("powers_shots", s).flight().1
}

/// A chip to home striker 9, 30 m ahead.
fn tries_lobs_and_chips(s: Option<TraitStrength>) -> f64 {
    let mate = index(0, 9);
    let d = carrier_draft(
        "tries_lobs_and_chips",
        s,
        DVec2::ZERO,
        &[(mate, DVec2::new(30.0, 5.0))],
        |sc| sc,
    );
    pass_score(&d, mate)
}

/// A forward pass through a tight lane to home striker 9 at (25, 3), with away player 6 2.4 m
/// off its line, against a short pass to home player 4 at (-6, 6): the score of the first
/// less the second.
fn risky_over_safe(name: &str, s: Option<TraitStrength>) -> f64 {
    let (forward, short) = (index(0, 9), index(0, 4));
    let d = carrier_draft(
        name,
        s,
        DVec2::ZERO,
        &[
            (forward, DVec2::new(25.0, 3.0)),
            (short, DVec2::new(-6.0, 6.0)),
            (index(1, 6), DVec2::new(12.0, -1.0)),
        ],
        |sc| sc,
    );
    pass_score(&d, forward) - pass_score(&d, short)
}

fn tries_killer_balls(s: Option<TraitStrength>) -> f64 {
    risky_over_safe("tries_killer_balls", s)
}

fn plays_short_simple_passes(s: Option<TraitStrength>) -> f64 {
    risky_over_safe("plays_short_simple_passes", s)
}

/// From (0, −25), a switch to home player 8 at (5, 15).
fn switches_play(s: Option<TraitStrength>) -> f64 {
    let mate = index(0, 8);
    let d = carrier_draft(
        "switches_play",
        s,
        DVec2::new(0.0, -25.0),
        &[(mate, DVec2::new(5.0, 15.0))],
        |sc| sc,
    );
    pass_score(&d, mate)
}

/// From (20, −25), 32.5 m from the goal line, a cross to home striker 9 at (42, 0).
fn crosses_early(s: Option<TraitStrength>) -> f64 {
    let mate = index(0, 9);
    let d = carrier_draft(
        "crosses_early",
        s,
        DVec2::new(20.0, -25.0),
        &[(mate, DVec2::new(42.0, 0.0))],
        |sc| sc,
    );
    pass_score(&d, mate)
}

/// The unpressed carrier's dribble score.
fn dribble(name: &str, s: Option<TraitStrength>) -> f64 {
    carrier_draft(name, s, DVec2::ZERO, &[], |sc| sc)
        .dribble_hold
        .and_then(|(d, _)| d)
        .expect("an unpressed carrier may dribble")
}

fn runs_with_ball_often(s: Option<TraitStrength>) -> f64 {
    dribble("runs_with_ball_often", s)
}

fn rarely_dribbles(s: Option<TraitStrength>) -> f64 {
    dribble("rarely_dribbles", s)
}

/// The carrier pressed by away player 6 at 1.5 m, 0.2 s after he gained the ball: his draft.
fn pressed_draft(name: &str, s: Option<TraitStrength>) -> engine::modules::OptionDraft {
    carrier_draft(
        name,
        s,
        DVec2::ZERO,
        &[(index(1, 6), DVec2::new(1.5, 0.0))],
        |sc| sc.control_since(990),
    )
}

fn tries_tricks(s: Option<TraitStrength>) -> f64 {
    pressed_draft("tries_tricks", s)
        .dribble_hold
        .and_then(|(d, _)| d)
        .expect("an average carrier tries a take-on")
}

/// A pressed carrier with technique 6.5, under the take-on's 7.0: 1 when he may try one.
fn tries_tricks_gate(s: Option<TraitStrength>) -> f64 {
    let d = carrier_draft(
        "tries_tricks",
        s,
        DVec2::ZERO,
        &[(index(1, 6), DVec2::new(1.5, 0.0))],
        |sc| sc.rating(index(0, 6), "technique", 6.5),
    );
    f64::from(u8::from(d.dribble_hold.and_then(|(d, _)| d).is_some()))
}

/// A pass back to home player 4 at (−8, 10), whose pass reached him.
fn plays_one_twos(s: Option<TraitStrength>) -> f64 {
    let mate = index(0, 4);
    let d = carrier_draft(
        "plays_one_twos",
        s,
        DVec2::ZERO,
        &[(mate, DVec2::new(-8.0, 10.0))],
        |sc| sc.last_pass(mate, index(0, 6)),
    );
    pass_score(&d, mate)
}

fn holds_up_ball(s: Option<TraitStrength>) -> f64 {
    pressed_draft("holds_up_ball", s)
        .dribble_hold
        .expect("an outfield carrier may hold")
        .1
}

/// The hold score after 1 s on the ball.
fn hold_after_a_second(name: &str, s: Option<TraitStrength>) -> f64 {
    carrier_draft(name, s, DVec2::ZERO, &[], |sc| sc.control_since(950))
        .dribble_hold
        .expect("an outfield carrier may hold")
        .1
}

fn releases_ball_early(s: Option<TraitStrength>) -> f64 {
    hold_after_a_second("releases_ball_early", s)
}

fn dwells_on_ball(s: Option<TraitStrength>) -> f64 {
    hold_after_a_second("dwells_on_ball", s)
}

/// The home side has the ball with player 4 at `ball`; home midfielder 6 carries `name`: his
/// target.
fn off_ball_target(name: &str, s: Option<TraitStrength>, ball: DVec2) -> DVec2 {
    off_ball_target_from(name, s, ball, |scene| scene)
}

/// [`off_ball_target`] with `place` applied to the scene first.
fn off_ball_target_from(
    name: &str,
    s: Option<TraitStrength>,
    ball: DVec2,
    place: impl FnOnce(Scene) -> Scene,
) -> DVec2 {
    let carrier = index(0, 4);
    let me = index(0, 6);
    let sim = traited(
        place(far())
            .place(carrier, ball)
            .ball(DVec3::new(ball.x, ball.y, 0.0))
            .carrier(Some(carrier))
            .tick(1_000),
        me,
        name,
        s,
    )
    .build();
    sim.config().modules.decision.targets(&sim.view()).targets[me]
}

/// His target's distance from his own goal line.
fn gets_forward(s: Option<TraitStrength>) -> f64 {
    off_ball_target("gets_forward", s, DVec2::new(-20.0, 0.0)).x + 52.5
}

fn stays_back(s: Option<TraitStrength>) -> f64 {
    off_ball_target("stays_back", s, DVec2::new(-20.0, 0.0)).x + 52.5
}

/// With the ball in the final third: his target's distance from his own goal line. He stands
/// at (5, -5), within `anchor_ball_distance` of the ball, so he is in play and is not sent
/// back to his place.
fn arrives_late_in_box(s: Option<TraitStrength>) -> f64 {
    let ball = DVec2::new(25.0, 10.0);
    let at = |scene: Scene| scene.place(index(0, 6), DVec2::new(5.0, -5.0));
    off_ball_target_from("arrives_late_in_box", s, ball, at).x + 52.5
}

fn moves_into_channels(s: Option<TraitStrength>) -> f64 {
    off_ball_target("moves_into_channels", s, DVec2::new(-20.0, 0.0))
        .y
        .abs()
}

/// The home striker carries at (0, 0); away midfielder 6 carries the trait: the farthest
/// distance, in steps of 10 cm, from which he presses.
fn presses_relentlessly(s: Option<TraitStrength>) -> f64 {
    let carrier = index(0, 9);
    let presser = index(1, 6);
    let mut farthest: f64 = 0.0;
    for k in 0..400 {
        let d = 1.0 + 0.1 * f64::from(k);
        let sim = traited(
            far()
                .place(carrier, DVec2::ZERO)
                .place(presser, DVec2::new(d, 0.0))
                .ball(DVec3::ZERO)
                .carrier(Some(carrier))
                .tick(1_000),
            presser,
            "presses_relentlessly",
            s,
        )
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

/// Away player 5 tackles the home striker running at 5 m/s; `who` (0 the tackler, 1 the
/// carrier) carries the trait: the chances.
fn tackle(
    name: &str,
    s: Option<TraitStrength>,
    carrier_has: bool,
) -> engine::modules::TackleChances {
    let tackler = index(1, 5);
    let carrier = index(0, 9);
    let at = DVec2::new(10.0, 2.0);
    let who = if carrier_has { carrier } else { tackler };
    let sim = traited(
        far()
            .place(carrier, at)
            .velocity(carrier, DVec2::new(5.0, 0.0))
            .ball(DVec3::new(at.x, at.y, 0.0))
            .carrier(Some(carrier))
            .place(tackler, DVec2::new(11.0, 2.0)),
        who,
        name,
        s,
    )
    .build();
    sim.config()
        .modules
        .fouls
        .tackle_chances(&sim.view(), tackler, carrier)
}

fn dives_into_tackles(s: Option<TraitStrength>) -> f64 {
    tackle("dives_into_tackles", s, false).p_foul
}

fn dives_into_tackles_win(s: Option<TraitStrength>) -> f64 {
    tackle("dives_into_tackles", s, false).p_win
}

fn stays_on_feet(s: Option<TraitStrength>) -> f64 {
    tackle("stays_on_feet", s, false).p_foul
}

fn goes_down_easily(s: Option<TraitStrength>) -> f64 {
    tackle("goes_down_easily", s, true).p_foul
}

/// Away player 5's chance of a card for a foul: the upper card threshold.
fn argues_with_officials(s: Option<TraitStrength>) -> f64 {
    let me = index(1, 5);
    let sim = traited(far(), me, "argues_with_officials", s).build();
    sim.config().modules.fouls.card_thresholds(&sim.view(), me)[1]
}

/// The away keeper on his line at (50, 0): the farthest still loose ball, in steps of 5 cm,
/// he comes off his line for.
fn chase(name: &str, s: Option<TraitStrength>) -> f64 {
    let keeper = index(1, 0);
    let mut farthest: f64 = 0.0;
    for k in 0..600 {
        let d = 5.0 + 0.05 * f64::from(k);
        let sim = traited(
            far()
                .place(keeper, DVec2::new(50.0, 0.0))
                .ball(DVec3::new(50.0 - d, 0.0, 0.0))
                .carrier(None)
                .tick(1_000),
            keeper,
            name,
            s,
        )
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

fn rushes_out(s: Option<TraitStrength>) -> f64 {
    chase("rushes_out", s)
}

fn stays_on_line(s: Option<TraitStrength>) -> f64 {
    chase("stays_on_line", s)
}

/// The away keeper's chance to hold a save.
fn punches_rather_than_catches(s: Option<TraitStrength>) -> f64 {
    let keeper = index(1, 0);
    let sim = traited(far(), keeper, "punches_rather_than_catches", s).build();
    sim.config()
        .modules
        .possession
        .save_hold(&sim.view(), keeper)
}

/// The home keeper has the ball at (−48, 0): a kick to home player 9 40 m out less a throw to
/// home player 4 20 m out.
fn keeper_kick_over_throw(name: &str, s: Option<TraitStrength>) -> f64 {
    let keeper = index(0, 0);
    let (long, short) = (index(0, 9), index(0, 4));
    let at = DVec2::new(-48.0, 0.0);
    let sim = traited(
        far()
            .place(keeper, at)
            .place(long, DVec2::new(-8.0, 3.0))
            .place(short, DVec2::new(-28.0, 3.0))
            .ball(DVec3::new(at.x, at.y, 0.0))
            .carrier(Some(keeper))
            .tick(1_000),
        keeper,
        name,
        s,
    )
    .build();
    let d = sim.config().modules.decision.options(&sim.view(), keeper);
    pass_score(&d, long) - pass_score(&d, short)
}

fn kicks_long(s: Option<TraitStrength>) -> f64 {
    keeper_kick_over_throw("kicks_long", s)
}

fn throws_short(s: Option<TraitStrength>) -> f64 {
    keeper_kick_over_throw("throws_short", s)
}

fn trait_rows() -> Vec<TraitRow> {
    let row = |name, reader, quantity, dir, scene| TraitRow {
        name,
        reader,
        quantity,
        dir,
        least: 0.10,
        binary: None,
        scene,
    };
    use Dir::{Down, Up};
    vec![
        row(
            "cuts_inside",
            "decision::DecisionV1::plan",
            "carry target's distance from the centre line",
            Down,
            cuts_inside,
        ),
        row(
            "hugs_touchline",
            "decision::DecisionV1::plan",
            "carry target's distance from the centre line",
            Up,
            hugs_touchline,
        ),
        row(
            "shoots_from_distance",
            "decision::DecisionV1::options",
            "shot score 20 m out, outside the area",
            Up,
            shoots_from_distance,
        ),
        row(
            "places_shots",
            "decision::DecisionV1::shot_kick",
            "aim's distance from the post",
            Down,
            places_shots,
        ),
        row(
            "powers_shots",
            "decision::DecisionV1::shot_kick",
            "shot speed",
            Up,
            powers_shots,
        ),
        row(
            "tries_lobs_and_chips",
            "decision::DecisionV1::options",
            "a chip's score",
            Up,
            tries_lobs_and_chips,
        ),
        row(
            "tries_killer_balls",
            "decision::DecisionV1::options",
            "risky forward pass over a short safe one",
            Up,
            tries_killer_balls,
        ),
        row(
            "plays_short_simple_passes",
            "decision::DecisionV1::options",
            "risky forward pass over a short safe one",
            Down,
            plays_short_simple_passes,
        ),
        row(
            "switches_play",
            "decision::DecisionV1::options",
            "a switch's score",
            Up,
            switches_play,
        ),
        row(
            "crosses_early",
            "decision::DecisionV1::options",
            "an early cross's score",
            Up,
            crosses_early,
        ),
        row(
            "runs_with_ball_often",
            "decision::DecisionV1::options",
            "dribble score",
            Up,
            runs_with_ball_often,
        ),
        row(
            "rarely_dribbles",
            "decision::DecisionV1::options",
            "dribble score",
            Down,
            rarely_dribbles,
        ),
        row(
            "tries_tricks",
            "decision::DecisionV1::options",
            "pressed dribble score",
            Up,
            tries_tricks,
        ),
        TraitRow {
            binary: Some("the take-on gate opens or stays shut"),
            ..row(
                "tries_tricks",
                "decision::DecisionV1::options",
                "a 6.5-technique pressed carrier may try a take-on",
                Up,
                tries_tricks_gate,
            )
        },
        row(
            "plays_one_twos",
            "decision::DecisionV1::options",
            "the return pass's score",
            Up,
            plays_one_twos,
        ),
        row(
            "holds_up_ball",
            "decision::DecisionV1::options",
            "pressed hold score",
            Up,
            holds_up_ball,
        ),
        row(
            "releases_ball_early",
            "decision::DecisionV1::options",
            "hold score after 1 s on the ball",
            Down,
            releases_ball_early,
        ),
        row(
            "dwells_on_ball",
            "decision::DecisionV1::options",
            "hold score after 1 s on the ball",
            Up,
            dwells_on_ball,
        ),
        row(
            "gets_forward",
            "decision::DecisionV1::targets",
            "anchor depth in possession",
            Up,
            gets_forward,
        ),
        row(
            "stays_back",
            "decision::DecisionV1::targets",
            "anchor depth in possession",
            Down,
            stays_back,
        ),
        row(
            "arrives_late_in_box",
            "decision::DecisionV1::targets",
            "target depth with the ball in the final third",
            Up,
            arrives_late_in_box,
        ),
        row(
            "moves_into_channels",
            "decision::DecisionV1::targets",
            "lateral offset of his anchor",
            Up,
            moves_into_channels,
        ),
        row(
            "presses_relentlessly",
            "decision::DecisionV1::targets",
            "press reach",
            Up,
            presses_relentlessly,
        ),
        row(
            "dives_into_tackles",
            "rules::fouls::FoulsV1::tackle_chances",
            "foul chance of his tackle",
            Up,
            dives_into_tackles,
        ),
        row(
            "dives_into_tackles",
            "rules::fouls::FoulsV1::tackle_chances",
            "win chance of his tackle",
            Up,
            dives_into_tackles_win,
        ),
        row(
            "stays_on_feet",
            "rules::fouls::FoulsV1::tackle_chances",
            "foul chance of his tackle",
            Down,
            stays_on_feet,
        ),
        row(
            "goes_down_easily",
            "rules::fouls::FoulsV1::tackle_chances",
            "foul chance of a tackle on him",
            Up,
            goes_down_easily,
        ),
        row(
            "argues_with_officials",
            "rules::fouls::FoulsV1::card_thresholds",
            "card chance on a foul",
            Up,
            argues_with_officials,
        ),
        row(
            "rushes_out",
            "decision::keeper_chase_m",
            "chase radius",
            Up,
            rushes_out,
        ),
        row(
            "stays_on_line",
            "decision::keeper_chase_m",
            "chase radius",
            Down,
            stays_on_line,
        ),
        row(
            "punches_rather_than_catches",
            "possession::hold_chance",
            "hold chance of a save",
            Down,
            punches_rather_than_catches,
        ),
        row(
            "kicks_long",
            "decision::DecisionV1::options",
            "long kick's score over a throw",
            Up,
            kicks_long,
        ),
        row(
            "throws_short",
            "decision::DecisionV1::options",
            "long kick's score over a throw",
            Down,
            throws_short,
        ),
    ]
}

/// Every trait row: strong moves each row's quantity by at least its least move, and
/// moderate lies strictly between none and strong unless the row is yes-or-no.
#[test]
fn every_trait_moves_its_choice() {
    let mut failed = Vec::new();
    for row in trait_rows() {
        let none = (row.scene)(None);
        let moderate = (row.scene)(Some(TraitStrength::Moderate));
        let strong = (row.scene)(Some(TraitStrength::Strong));
        let mv = relative(none, strong, row.dir);
        println!(
            "{:<28} {:<40} {:<50} none {:>9.4}  moderate {:>9.4}  strong {:>9.4}  move {:+.3}",
            row.name, row.reader, row.quantity, none, moderate, strong, mv
        );
        if mv < row.least {
            failed.push(format!("{} ({}): {mv:+.4}", row.name, row.quantity));
        }
        let between =
            relative(none, moderate, row.dir) > 0.0 && relative(moderate, strong, row.dir) > 0.0;
        if row.binary.is_none() && !between {
            failed.push(format!(
                "{} ({}): moderate {moderate} not between {none} and {strong}",
                row.name, row.quantity
            ));
        }
    }
    assert!(failed.is_empty(), "traits that move too little: {failed:?}");
}

/// Completeness: every trait of the shipped catalogue has a row, and every row names a trait
/// of the catalogue, its reader and its quantity.
#[test]
fn every_trait_has_a_reader() {
    let content = common::content();
    let rows = trait_rows();
    for t in &content.traits.traits {
        assert!(
            rows.iter().any(|r| r.name == t.name),
            "trait {} has no reader row",
            t.name
        );
    }
    for r in &rows {
        assert!(
            content.traits.get(r.name).is_some(),
            "{} is not a trait",
            r.name
        );
        assert!(!r.reader.is_empty() && !r.quantity.is_empty());
    }
}

/// Without a traits block in the tuning, a strong trait moves nothing.
#[test]
fn a_tuning_without_the_traits_block_plays_no_trait() {
    let carrier = index(0, 6);
    let mut config = calm_match(90);
    config.tuning.contract.traits = None;
    let draft = |s| {
        let sim = Scene::new(config.clone())
            .place(carrier, DVec2::ZERO)
            .ball(DVec3::ZERO)
            .carrier(Some(carrier))
            .tick(1_000)
            .with_trait(carrier, "runs_with_ball_often", s)
            .build();
        sim.config().modules.decision.options(&sim.view(), carrier)
    };
    let (a, b) = (draft(None), draft(Some(TraitStrength::Strong)));
    assert_eq!(a.dribble_hold, b.dribble_hold);
}
