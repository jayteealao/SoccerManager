//! Tests of the pace pass ([`crate::pace`]) through the real decision pass: each scene sets
//! match state directly, so the tests live with the central loop.

use crate::ball::Ball;
use crate::data::test_support::shipped_config;
use crate::math::DVec2;
use crate::pace::{Grade, Job, Level, base_grade};
use crate::sim::Simulation;
use crate::team::PLAYERS_PER_TEAM;

/// A seed-7 match on the shipped content, a minute in, before any late-match change.
fn scene() -> Simulation {
    let mut sim = Simulation::new(shipped_config(7, 90).unwrap()).unwrap();
    sim.tick = 3_000;
    sim
}

/// Player `c` holds the ball at `at`; his team is in possession.
fn hold(sim: &mut Simulation, c: usize, at: DVec2) {
    let p = &mut sim.players[c];
    p.pos = at;
    p.vel = DVec2::ZERO;
    p.target = at;
    sim.ball = Ball::at(at);
    sim.carrier = Some(c);
    sim.control_since = sim.tick;
    sim.last_touch = Some(sim.players[c].team);
    sim.possession_phase = Some(sim.players[c].team);
}

/// The decision pass and the pace pass of one live tick.
fn decide_and_pace(sim: &mut Simulation) {
    let _ = sim.decide();
    sim.pace_tick();
}

/// Every player stands at the target the decision pass gives him now.
fn settle(sim: &mut Simulation) {
    let targets = sim.config.modules.decision.targets(&sim.view());
    for (i, p) in sim.players.iter_mut().enumerate() {
        if Some(i) != sim.carrier && p.active() {
            p.pos = targets.targets[i];
            p.vel = DVec2::ZERO;
        }
    }
}

fn keeper(sim: &Simulation, team: usize) -> usize {
    team * PLAYERS_PER_TEAM + sim.teams[team].keeper_slot()
}

/// AC-10: with no lift and early in the match, each job carries its decided grade on the
/// player, and the pace pass writes it.
fn assert_grades(sim: &Simulation) {
    for (i, p) in sim.players.iter().enumerate() {
        if p.active() {
            assert_eq!(
                p.pace.grade,
                base_grade(p.pace.job),
                "player {i}: {:?}",
                p.pace
            );
        }
    }
}

#[test]
fn the_carrier_carries_and_the_nearest_opponent_presses_urgently() {
    let mut sim = scene();
    let c = 9;
    let at = DVec2::new(10.0, 0.0);
    hold(&mut sim, c, at);
    settle(&mut sim);
    decide_and_pace(&mut sim);
    assert_eq!(sim.players[c].pace.job, Job::Carry);
    assert_eq!(sim.players[c].pace.grade, Grade::Urgent);
    let away = PLAYERS_PER_TEAM..2 * PLAYERS_PER_TEAM;
    let nearest = away
        .clone()
        .filter(|&i| i != keeper(&sim, 1))
        .min_by(|&a, &b| {
            let d = |i: usize| (sim.players[i].pos - at).length();
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    assert_eq!(sim.players[nearest].pace.job, Job::Press);
    for i in away {
        if sim.players[i].pace.job == Job::Press {
            assert_eq!(sim.players[i].pace.grade, Grade::Urgent);
        }
    }
    assert_grades(&sim);
}

#[test]
fn a_team_short_of_goal_side_cover_sends_its_nearest_players_back_urgently() {
    let mut sim = scene();
    // The home carrier deep in the away half; every away outfield player upfield of him.
    let c = 9;
    let ball = DVec2::new(35.0, 0.0);
    hold(&mut sim, c, ball);
    let away_keeper = keeper(&sim, 1);
    let mut k = 0.0;
    for i in PLAYERS_PER_TEAM..2 * PLAYERS_PER_TEAM {
        if i != away_keeper {
            // Away defends the goal at x = 52.5 in the first half: these stand ahead of the
            // ball, from 5 m to 32 m.
            sim.players[i].pos = DVec2::new(30.0 - 3.0 * k, -20.0 + 4.0 * k);
            k += 1.0;
        }
    }
    decide_and_pace(&mut sim);
    let side = &sim.teams[1];
    let minimum = side.plan.rest_defence.min(PLAYERS_PER_TEAM - 2);
    assert!(minimum > 0, "the mentality asks for cover");
    let recovering: Vec<usize> = (PLAYERS_PER_TEAM..2 * PLAYERS_PER_TEAM)
        .filter(|&i| sim.players[i].pace.job == Job::Recover)
        .collect();
    assert_eq!(recovering.len(), minimum, "{recovering:?}");
    for &i in &recovering {
        assert_eq!(sim.players[i].pace.grade, Grade::Urgent);
    }
    // The nearest to getting goal-side go: the deepest players not pressing.
    let deepest_free = (PLAYERS_PER_TEAM..2 * PLAYERS_PER_TEAM)
        .filter(|&i| i != away_keeper && sim.players[i].pace.job != Job::Press)
        .max_by(|&a, &b| sim.players[a].pos.x.total_cmp(&sim.players[b].pos.x))
        .unwrap();
    assert!(recovering.contains(&deepest_free));
    assert_grades(&sim);
}

#[test]
fn the_first_chaser_goes_for_a_loose_ball_and_the_other_chases() {
    let mut sim = scene();
    hold(&mut sim, 9, DVec2::ZERO);
    settle(&mut sim);
    sim.carrier = None;
    sim.ball = Ball::at(DVec2::new(0.0, 20.0));
    let (home, away) = (6, PLAYERS_PER_TEAM + 6);
    sim.players[home].pos = DVec2::new(0.0, 17.0);
    sim.players[away].pos = DVec2::new(0.0, 25.0);
    for i in 0..2 * PLAYERS_PER_TEAM {
        if i != home && i != away && (sim.players[i].pos - DVec2::new(0.0, 20.0)).length() < 9.0 {
            sim.players[i].pos.x += 12.0;
        }
    }
    decide_and_pace(&mut sim);
    assert_eq!(sim.players[home].pace.job, Job::LooseBall);
    assert_eq!(sim.players[home].pace.grade, Grade::Urgent);
    assert_eq!(sim.players[away].pace.job, Job::Chase);
    assert_eq!(sim.players[away].pace.grade, Grade::Normal);
    assert_grades(&sim);
}

#[test]
fn players_in_shape_follow_near_the_ball_hold_far_from_it_and_return_when_far_from_their_place() {
    let mut sim = scene();
    hold(&mut sim, 9, DVec2::new(-20.0, 0.0));
    settle(&mut sim);
    decide_and_pace(&mut sim);
    let ball = sim.ball.xy();
    let t = sim.tuning.pace.clone();
    let mut seen = [false; 3];
    for p in sim.players.iter().filter(|p| p.active()) {
        let near = (p.pos - ball).length() <= t.calm_ball_m;
        let at_place = (p.target - p.pos).length() <= t.return_far_m;
        match p.pace.job {
            Job::Follow => {
                assert!(near && at_place, "{p:?}");
                seen[0] = true;
            }
            Job::Hold => {
                assert!(!near && at_place, "{p:?}");
                assert_eq!(p.pace.grade, Grade::Calm);
                assert!(p.pace.level <= Level::Walk);
                seen[1] = true;
            }
            _ => {}
        }
    }
    // One player 12 m from his place returns to it.
    let i = PLAYERS_PER_TEAM + 2;
    sim.players[i].pos += DVec2::new(0.0, 12.0);
    decide_and_pace(&mut sim);
    if sim.players[i].pace.job == Job::Return {
        assert_eq!(sim.players[i].pace.grade, Grade::Normal);
        assert_eq!(sim.players[i].pace.level, Level::Jog);
        seen[2] = true;
    }
    assert_eq!(seen, [true; 3], "follow, hold and return each seen");
    assert_grades(&sim);
}

#[test]
fn a_player_whose_stand_option_fired_in_possession_offers_and_a_lapse_holds() {
    let mut sim = scene();
    // Build-up: the home keeper's pass reaches a centre-back deep in his half.
    let c = 2;
    hold(&mut sim, c, DVec2::new(-40.0, -8.0));
    settle(&mut sim);
    decide_and_pace(&mut sim);
    let shaped = crate::shape::apply(
        &sim.view(),
        sim.possession_phase,
        &crate::shape::anchors(&sim.view(), sim.possession_phase),
        &mut [DVec2::ZERO; crate::modules::ROSTER],
    );
    let mut offers = 0;
    for i in 0..PLAYERS_PER_TEAM {
        let p = &sim.players[i];
        if shaped[0].option[p.slot].is_some() && p.pace.job == Job::Offer {
            assert_eq!(p.pace.grade, Grade::Normal);
            offers += 1;
        }
    }
    assert!(offers > 0, "a home player offers in build-up");
    // Out of possession, no player offers.
    for p in &sim.players[PLAYERS_PER_TEAM..] {
        assert_ne!(p.pace.job, Job::Offer, "{p:?}");
    }
    let lapsing = 7;
    sim.players[lapsing].lapse_until = sim.tick + 100;
    decide_and_pace(&mut sim);
    assert_eq!(sim.players[lapsing].pace.job, Job::Hold);
    assert_eq!(sim.players[lapsing].pace.grade, Grade::Calm);
}

/// Charter scenario step 4 (proxy): the home team holds the ball in the away half and loses
/// it; over the next second the nearest home presser sprints (the sprint chance is 1 here),
/// while the home players behind the ball go to their places at a jog or a run, and at least
/// the mentality minimum stays goal-side.
#[test]
fn losing_the_ball_sends_the_nearest_presser_at_a_sprint_and_the_others_jog_back() {
    let mut config = shipped_config(7, 90).unwrap();
    config.tuning.pace.sprint_mid = 1.0;
    config.tuning.pace.sprint_slope = 0.0;
    let mut sim = Simulation::new(config).unwrap();
    sim.tick = 3_000;
    assert_eq!(crate::pace::sprint_chance(0.1, &sim.tuning.pace), 1.0);
    let ball = DVec2::new(25.0, 5.0);
    hold(&mut sim, 9, ball);
    settle(&mut sim);
    // An away midfielder wins the ball where it is; the home striker is 6 m from it.
    let winner = PLAYERS_PER_TEAM + 6;
    sim.players[winner].pos = ball;
    sim.players[9].pos = ball + DVec2::new(-6.0, 0.0);
    hold(&mut sim, winner, ball);
    let minimum = sim.teams[0].plan.rest_defence;
    let home_keeper = keeper(&sim, 0);
    let mut presser_sprinted = false;
    for _ in 0..50 {
        if sim.carrier != Some(winner) {
            break;
        }
        decide_and_pace(&mut sim);
        let bx = sim.ball.xy().x;
        let pressers: Vec<usize> = (0..PLAYERS_PER_TEAM)
            .filter(|&i| sim.players[i].pace.job == Job::Press)
            .collect();
        assert!(!pressers.is_empty(), "a home player presses");
        let nearest = *pressers
            .iter()
            .min_by(|&&a, &&b| {
                let d = |i: usize| (sim.players[i].pos - sim.ball.xy()).length();
                d(a).total_cmp(&d(b))
            })
            .unwrap();
        let p = &sim.players[nearest].pace;
        if p.level == Level::Sprint {
            presser_sprinted = true;
        }
        let mut goal_side = 0;
        for i in 0..PLAYERS_PER_TEAM {
            let q = &sim.players[i];
            if i == home_keeper || !q.active() || pressers.contains(&i) {
                continue;
            }
            // Home attacks x = 52.5 in the first half: behind the ball is a smaller x.
            if q.pos.x < bx {
                goal_side += 1;
                if q.pace.job != Job::Recover {
                    assert!(
                        q.pace.level <= Level::Run,
                        "player {i} behind the ball: {:?}",
                        q.pace
                    );
                }
                if matches!(q.pace.job, Job::Return) {
                    assert!(
                        matches!(q.pace.level, Level::Jog | Level::Run),
                        "{:?}",
                        q.pace
                    );
                }
            }
        }
        assert!(
            goal_side >= minimum,
            "{goal_side} goal-side, {minimum} asked"
        );
        let t = std::sync::Arc::clone(&sim.tuning);
        sim.steer_players(&t);
        // The winner keeps the ball where he stands.
        sim.players[winner].pos = ball;
        sim.players[winner].vel = DVec2::ZERO;
        sim.tick += 1;
    }
    assert!(presser_sprinted, "the nearest presser sprinted");
}

/// AC-14 with the person's narrowing: a team leading late with time wasting drops its NORMAL
/// jobs one step while it has the ball, and keeps every grade while the opponent has it. The
/// late window covers the whole match here, so a scene a minute in counts as late.
#[test]
fn a_leading_team_wastes_time_with_the_ball_and_keeps_its_grades_without_it() {
    let mut config = shipped_config(7, 90).unwrap();
    config.tuning.pace.late_minutes = 90;
    let mut sim = Simulation::new(config).unwrap();
    sim.tick = 3_000;
    sim.summary.goals = [1, 0];
    sim.teams[0].plan.time_wasting = 2.0;
    let normal_home = |sim: &Simulation| -> Vec<(Job, Grade)> {
        sim.players[..PLAYERS_PER_TEAM]
            .iter()
            .filter(|p| p.active() && base_grade(p.pace.job) == Grade::Normal)
            .map(|p| (p.pace.job, p.pace.grade))
            .collect()
    };

    // The home team has the ball: its NORMAL jobs walk.
    hold(&mut sim, 6, DVec2::new(-10.0, 0.0));
    settle(&mut sim);
    sim.players[2].pos += DVec2::new(0.0, 12.0);
    decide_and_pace(&mut sim);
    let with_ball = normal_home(&sim);
    assert!(!with_ball.is_empty(), "a home player has a NORMAL job");
    for (job, grade) in &with_ball {
        assert_eq!(*grade, Grade::Calm, "{job:?} with the ball");
    }

    // The away team has the ball: the home team keeps its grades, presses stay URGENT.
    let winner = PLAYERS_PER_TEAM + 6;
    hold(&mut sim, winner, DVec2::new(-10.0, 0.0));
    sim.players[2].pos += DVec2::new(0.0, 12.0);
    decide_and_pace(&mut sim);
    let without_ball = normal_home(&sim);
    assert!(!without_ball.is_empty(), "a home player has a NORMAL job");
    for (job, grade) in &without_ball {
        assert_eq!(*grade, Grade::Normal, "{job:?} without the ball");
    }
    for p in sim.players[..PLAYERS_PER_TEAM]
        .iter()
        .filter(|p| p.active())
    {
        if matches!(p.pace.job, Job::Press | Job::Recover) {
            assert_eq!(p.pace.grade, Grade::Urgent, "{p:?}");
        }
    }

    // Control: the same scene with time wasting off grades the home team as usual.
    sim.teams[0].plan.time_wasting = 1.0;
    hold(&mut sim, 6, DVec2::new(-10.0, 0.0));
    decide_and_pace(&mut sim);
    for (job, grade) in normal_home(&sim) {
        assert_eq!(
            grade,
            Grade::Normal,
            "{job:?} with the ball, no time wasting"
        );
    }
}
