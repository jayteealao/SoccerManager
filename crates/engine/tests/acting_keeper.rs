//! The acting keeper. When the keeper is sent off, the computer manager brings the bench
//! keeper on at the next stoppage while a substitution is left; otherwise the most advanced
//! outfield player keeps goal and the forward line plays one short. Saves, penalties, the
//! shoot-out, and restarts all use the acting keeper.

mod common;

use common::{calm_match, index};
use engine::data::Position;
use engine::math::{DVec2, DVec3};
use engine::rules::clock::TICKS_PER_MINUTE;
use engine::scenario::Scene;
use engine::{
    AiCode, EngineEvent, EngineEventKind, EventDetail, Manager, MatchConfig, Simulation,
    StoppageKind,
};

const AWAY_KEEPER: usize = 11;

/// A 90-minute match in which every player decides and nobody is injured.
fn live() -> MatchConfig {
    let mut config = common::short_match(90);
    config.tuning.injury_per_minute = 0.0;
    config.tuning.injury_per_tackle = 0.0;
    config
}

fn play_until(
    sim: &mut Simulation,
    ticks: u32,
    stop: impl Fn(&[EngineEvent]) -> bool,
) -> Vec<EngineEvent> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        if sim.is_over() {
            break;
        }
        sim.step();
        events.extend(sim.take_events());
        if stop(&events) {
            break;
        }
    }
    events
}

fn is_keeper_by_position(sim: &Simulation, team: usize, i: usize) -> bool {
    let side = &sim.teams()[team];
    side.squad[side.lineup[i % 11]].position == Position::GK
}

#[test]
fn a_keeper_sent_off_is_replaced_by_the_bench_keeper_at_the_next_stoppage() {
    let mut sim = Scene::new(live()).sent_off(AWAY_KEEPER).build();
    // Until the substitute arrives, an outfield player keeps goal.
    let stand_in = sim.keeper(1);
    assert_ne!(stand_in, AWAY_KEEPER);
    assert!(!is_keeper_by_position(&sim, 1, stand_in));
    let events = play_until(&mut sim, 60 * TICKS_PER_MINUTE, |events| {
        events
            .iter()
            .any(|e| e.kind == EngineEventKind::Substitution && e.team == Some(1))
    });
    let decisions: Vec<&EngineEvent> = events
        .iter()
        .filter(|e| {
            e.kind == EngineEventKind::AiDecision
                && e.detail
                    == Some(EventDetail::Ai {
                        code: AiCode::SubKeeper,
                    })
        })
        .collect();
    assert_eq!(decisions.len(), 1, "one keeper decision: {events:?}");
    let sub = events
        .iter()
        .find(|e| e.kind == EngineEventKind::Substitution && e.team == Some(1))
        .expect("the bench keeper came on");
    assert_eq!(sub.player, Some(stand_in), "the stand-in made way");
    let keeper = sim.keeper(1);
    assert_eq!(keeper, stand_in, "the keeper took the stand-in's place");
    assert!(is_keeper_by_position(&sim, 1, keeper));
}

#[test]
fn a_keeper_sent_off_with_no_substitution_left_leaves_an_outfield_player_in_goal() {
    let sim = Scene::new(live())
        .subs_used(1, 5, 3)
        .sent_off(AWAY_KEEPER)
        .build();
    let team = &sim.teams()[1];
    let slot = team.keeper_slot();
    assert_eq!(
        slot, 9,
        "the more central of the level strikers, then the lower slot"
    );
    assert_eq!(team.formation[slot], team.base_formation[0]);
    // The other striker stays where he was: the forward line is one short.
    assert_eq!(team.formation[10], team.base_formation[10]);
    assert!(team.active[10]);
}

/// The away side with its keeper sent off and no substitution left, the stand-in (the away
/// striker in slot 9) on his line, and the home striker shooting straight at him from 12 m.
fn shot_at_the_stand_in(catch: f64) -> Simulation {
    let mut config = calm_match(90);
    config.tuning.keeper_catch_chance = catch;
    let shooter = index(0, 9);
    let from = DVec2::new(40.0, 0.0);
    let mut sim = common::spread(
        Scene::new(config).subs_used(1, 5, 3).sent_off(AWAY_KEEPER),
        -30.0,
        -40.0,
    )
    .place(index(1, 9), DVec2::new(51.0, 0.0))
    .place(shooter, from)
    .ball(DVec3::new(from.x + 0.3, 0.0, 0.0))
    .carrier(Some(shooter))
    .tick(1_000)
    .shoot(DVec2::new(1.0, 0.0), 25.0, 0.0)
    .build();
    assert_eq!(sim.keeper(1), index(1, 9));
    for _ in 0..40 {
        sim.step();
    }
    sim
}

#[test]
fn a_fast_shot_at_the_stand_in_is_held_with_the_catch_chance() {
    let held = shot_at_the_stand_in(1.0);
    assert_eq!(
        held.carrier(),
        Some(held.keeper(1)),
        "the stand-in holds the ball"
    );
    assert_eq!(held.summary().shots_on_target[0], 1);
    let beaten = shot_at_the_stand_in(0.0);
    assert_ne!(beaten.carrier(), Some(beaten.keeper(1)));
}

#[test]
fn a_penalty_faces_the_stand_in_on_his_goal_line() {
    let config = calm_match(90);
    let carrier = index(0, 9);
    let tackler = index(1, 2);
    let at = DVec2::new(45.0, 3.0);
    let sim = Simulation::new(config.clone()).unwrap();
    let t = sim.players()[tackler].derived;
    let c = sim.players()[carrier].derived;
    let p_win = 0.05 * t.tackling / (t.tackling + c.dribbling);
    let p_foul = engine::rules::fouls::foul_chance(&t, 0, &config.tuning);
    let mut sim = common::spread(
        Scene::new(config).subs_used(1, 5, 3).sent_off(AWAY_KEEPER),
        -30.0,
        -30.0,
    )
    .place(carrier, at)
    .place(tackler, at + DVec2::new(0.3, 0.0))
    .ball(DVec3::new(at.x, at.y, 0.0))
    .carrier(Some(carrier))
    .tick(1_001)
    .rolls(&[p_win + 0.1 * p_foul, 0.99])
    .build();
    sim.step();
    let dead = sim.dead_ball().expect("play stopped");
    assert_eq!(dead.kind, StoppageKind::Penalty);
    let keeper = sim.keeper(1);
    sim.step();
    let target = sim.players()[keeper].target;
    assert_eq!(
        target,
        DVec2::new(52.0, 0.0),
        "the stand-in is sent to his goal line (kept 0.5 m inside the pitch)"
    );
}

#[test]
fn a_shoot_out_kick_faces_the_stand_in() {
    let mut sim = Scene::new(calm_match(5).with_knockout())
        .manager(0, Manager::Human)
        .manager(1, Manager::Human)
        .subs_used(1, 5, 3)
        .sent_off(AWAY_KEEPER)
        .build();
    let keeper = sim.keeper(1);
    let mut seen = 0;
    for _ in 0..400_000 {
        if sim.is_over() {
            break;
        }
        sim.step();
        sim.take_events();
        if sim.in_shootout()
            && let Some(dead) = sim.dead_ball()
            && dead.team == 0
            && sim.tick() > dead.since
        {
            let target = sim.players()[keeper].target;
            let end = dead.spot.x.signum();
            assert_eq!(
                target,
                DVec2::new(end * 52.2, 0.0),
                "the stand-in keeps goal"
            );
            seen += 1;
        }
    }
    assert!(seen > 0, "no home kick was set up");
}

#[test]
fn a_goal_kick_after_half_time_with_the_keeper_parked_is_taken_at_the_defending_end() {
    // Second half: the away side defends the goal at x = -52.5, and its parked keeper stands
    // beside the pitch on the side it had in the first half.
    let mut sim = Scene::new(calm_match(90))
        .at_minute(50)
        .subs_used(1, 5, 3)
        .sent_off(AWAY_KEEPER)
        .ball(DVec3::new(-52.0, 10.0, 0.0))
        .ball_velocity(DVec3::new(-10.0, 0.0, 0.0))
        .carrier(None)
        .last_touch(0)
        .build();
    let own_end = -sim.teams()[1].attack_x;
    assert_eq!(own_end, -1.0);
    for _ in 0..20 {
        sim.step();
        if sim.dead_ball().is_some() {
            break;
        }
    }
    let dead = sim.dead_ball().expect("the ball went out");
    assert_eq!(dead.kind, StoppageKind::GoalKick);
    assert_eq!(dead.team, 1);
    assert!(
        dead.spot.x * own_end > 0.0,
        "the goal kick is at {}",
        dead.spot
    );
    assert_eq!(dead.taker, sim.keeper(1), "the stand-in takes it");
}
