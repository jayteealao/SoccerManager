//! Game-state rules in the match loop: a rule fires when its conditions come to hold and
//! lapses when they stop holding; a change the rules make is taken up player by player until
//! the whole side has it; a new formation waits for a stoppage; the situation the rules read
//! is the match's; and a side's rules play the same under a person as under the computer
//! manager.

use crate::ai::Manager;
use crate::data::test_support::shipped_config;
use crate::sim::{MatchConfig, Simulation};
use crate::tactics::change::Change;
use crate::tactics::rules::{self, Action, Condition, GameState, Rule, Score};
use crate::tactics::{Tactics, TeamPlan};
use crate::ticks_for_minutes;

/// The index of primitive `name` and of its level `level`.
fn level_of(config: &MatchConfig, name: &str, level: &str) -> Action {
    let schema = &config.tactics;
    let p = schema
        .primitives
        .iter()
        .position(|p| p.name == name)
        .unwrap();
    let l = schema.primitives[p]
        .level_names(schema)
        .iter()
        .position(|n| *n == level)
        .unwrap();
    Action::Level {
        primitive: p as u8,
        level: l as u8,
    }
}

/// One rule: leading by a goal with 15 minutes or less left, sit deep.
fn leading_late(config: &MatchConfig) -> GameState {
    GameState {
        rules: vec![Rule {
            when: vec![
                Condition::Score(Score::Leading(1)),
                Condition::TimeLeft { minutes: 15.0 },
            ],
            set: vec![
                level_of(config, "mentality", "defensive"),
                level_of(config, "line_of_engagement", "low"),
            ],
        }],
        instructions: Vec::new(),
    }
}

/// A seed-5 90-minute match with both sides run by people, home playing `state`.
fn with_state(state: impl FnOnce(&MatchConfig) -> GameState) -> Simulation {
    let config = shipped_config(5, 90)
        .unwrap()
        .with_manager(0, Manager::Human)
        .with_manager(1, Manager::Human);
    let state = state(&config);
    Simulation::new(config.with_game_state(0, state)).unwrap()
}

/// Puts the clock in the second half with `left` minutes of regulation time to play.
fn minutes_left(sim: &mut Simulation, left: u32) {
    let clock = &mut sim.referee.clock;
    clock.half = 1;
    clock.half_start = 0;
    let length = clock.period_ticks(1);
    sim.tick = length - ticks_for_minutes(left) - 1;
}

/// The rules' tactics in force with `active` applied over the side's base, in the formation
/// and roles in force.
fn expected(sim: &Simulation, active: u8) -> Tactics {
    let side = &sim.teams[0];
    let target = rules::effective(
        &side.base,
        &side.game_state.rules,
        active,
        &sim.config.tactics,
    );
    rules::at_once(&target, &side.tactics)
}

/// G1: leading by one with ten minutes left, the rule fires; level again, it lapses. Each
/// time the change applies whole on its own tick, logged with no stoppage.
#[test]
fn a_leading_late_rule_fires_and_lapses() {
    let mut sim = with_state(leading_late);
    let base = sim.teams[0].tactics;
    minutes_left(&mut sim, 20);
    sim.summary.goals = [1, 0];
    sim.rules_tick();
    assert_eq!(sim.teams[0].rules_active, 0, "twenty minutes left");
    minutes_left(&mut sim, 10);
    sim.rules_tick();
    assert_eq!(sim.teams[0].rules_active, 1);
    let fired = expected(&sim, 1);
    assert_ne!(fired, base);
    assert_eq!(sim.teams[0].tactics, fired);
    assert_eq!(sim.teams[0].base, base, "the base stays");
    assert!(sim.queue.pending.is_empty());
    let log = &sim.applied;
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].change, Change::Rule { active: 1 });
    assert_eq!(log[0].stoppage, None);

    sim.summary.goals = [1, 1];
    sim.rules_tick();
    assert_eq!(sim.teams[0].rules_active, 0);
    assert_eq!(sim.teams[0].tactics, base);
    assert_eq!(sim.applied.len(), 2);
    assert_eq!(sim.applied[1].change, Change::Rule { active: 0 });
}

/// G5: the players take a rule change up one by one; the share that has it only rises, the
/// side reads it as a whole from half, and after the take-up time every player has it.
#[test]
fn a_rule_change_is_taken_up_until_every_player_has_it() {
    let mut sim = with_state(leading_late);
    minutes_left(&mut sim, 10);
    sim.summary.goals = [1, 0];
    let before = sim.teams[0].plan;
    sim.rules_tick();
    let (mut last, on) = sim.teams[0]
        .take_up_share()
        .expect("a shape stage below its top delays someone");
    assert!(last < on, "{last} of {on} at once");
    let schema = sim.config.tactics.clone();
    let after = TeamPlan::from(&sim.teams[0].tactics, &schema, &sim.tuning);
    assert_ne!(before, after);
    let take_up_s = schema.game_state.as_ref().unwrap().take_up_s;
    let limit = (take_up_s * f64::from(crate::TICKS_PER_SECOND)) as u32 + 1;
    let mut ticks = 0;
    while let Some((taken, on)) = sim.teams[0].take_up_share() {
        assert!(taken >= last, "the share fell from {last} to {taken}");
        let switched = sim.teams[0].take_up.as_ref().unwrap().switched;
        assert_eq!(switched, 2 * taken >= on, "{taken} of {on}");
        assert_eq!(sim.teams[0].plan, if switched { after } else { before });
        last = taken;
        sim.tick += 1;
        sim.rules_tick();
        ticks += 1;
        assert!(ticks <= limit, "still taking up after {ticks} ticks");
    }
    assert_eq!(sim.teams[0].plan, after);
    for slot in 0..crate::team::PLAYERS_PER_TEAM {
        assert_eq!(*sim.teams[0].plan_for(slot), after, "slot {slot}");
    }
}

/// G6: a rule that sets a formation changes the levels at once; the formation waits in the
/// queue and applies at the next stoppage that admits tactics.
#[test]
fn a_rule_formation_waits_for_a_stoppage() {
    let mut sim = with_state(|config| {
        let f433 = config.tactics.formation_index("4-3-3").unwrap() as u8;
        GameState {
            // Holds all match.
            rules: vec![Rule {
                when: vec![Condition::TimeLeft { minutes: 200.0 }],
                set: vec![
                    Action::Formation(f433),
                    level_of(config, "mentality", "attacking"),
                ],
            }],
            instructions: Vec::new(),
        }
    });
    let f433 = sim.config.tactics.formation_index("4-3-3").unwrap() as u8;
    let formation = sim.teams[0].tactics.formation;
    assert_ne!(formation, f433);
    sim.rules_tick();
    assert_eq!(sim.teams[0].rules_active, 1);
    assert_eq!(
        sim.teams[0].tactics.formation, formation,
        "the formation waits"
    );
    assert_eq!(sim.teams[0].tactics, expected(&sim, 1));
    assert!(
        sim.queue
            .pending
            .iter()
            .any(|q| q.team == 0 && q.change == Change::Rule { active: 1 })
    );
    assert!(sim.applied.is_empty());
    while !sim.is_over() && sim.applied.is_empty() {
        sim.step();
    }
    let log = &sim.applied;
    assert_eq!(log.len(), 1, "{log:?}");
    assert_eq!(log[0].change, Change::Rule { active: 1 });
    assert!(log[0].stoppage.is_some());
    assert_eq!(sim.teams[0].tactics.formation, f433);
    assert!(sim.teams[0].take_up.is_none());
}

/// A rule change replaces the rule change of the side still waiting in the queue.
#[test]
fn a_waiting_rule_change_is_replaced() {
    let mut sim = with_state(|config| {
        let f433 = config.tactics.formation_index("4-3-3").unwrap() as u8;
        let f4411 = config.tactics.formation_index("4-4-1-1").unwrap() as u8;
        GameState {
            rules: vec![
                Rule {
                    when: vec![Condition::Score(Score::Level)],
                    set: vec![Action::Formation(f433)],
                },
                Rule {
                    when: vec![Condition::Score(Score::Leading(1))],
                    set: vec![Action::Formation(f4411)],
                },
            ],
            instructions: Vec::new(),
        }
    });
    sim.rules_tick();
    sim.summary.goals = [1, 0];
    sim.rules_tick();
    let rules: Vec<_> = sim
        .queue
        .pending
        .iter()
        .filter(|q| matches!(q.change, Change::Rule { .. }))
        .collect();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].change, Change::Rule { active: 2 });
    assert_eq!(sim.summary.changes_queued, 1);
    assert!(sim.applied.is_empty());
    // Trailing, no rule holds: the base needs no stoppage, so the change applies whole at
    // once and the waiting one is gone.
    sim.summary.goals = [1, 2];
    sim.rules_tick();
    assert!(sim.queue.pending.is_empty());
    assert_eq!(sim.applied.len(), 1);
    assert_eq!(sim.applied[0].change, Change::Rule { active: 0 });
    assert_eq!(sim.applied[0].stoppage, None);
}

/// The situation the rules read: the score, the minutes left, the players sent off, the run
/// of play over its window, and the mean outfield energy.
#[test]
fn the_situation_is_the_matchs() {
    let mut sim = with_state(leading_late);
    minutes_left(&mut sim, 30);
    sim.summary.goals = [0, 2];
    sim.players[3].status = crate::player::Status::SentOff;
    sim.players[14].status = crate::player::Status::SentOff;
    sim.players[15].status = crate::player::Status::SentOff;
    let now = sim.tick + 1;
    sim.chances = vec![(now - 10, 0, 0.4), (now - 5, 1, 0.1), (now + 3, 1, 0.9)];
    let s = sim.situation(0, now);
    assert_eq!(s.goal_difference, -2);
    assert!((s.minutes_left - 30.0).abs() < 1e-9, "{}", s.minutes_left);
    assert_eq!(s.sent_off, [1, 2]);
    assert!((s.run_of_play - 0.3).abs() < 1e-12, "{}", s.run_of_play);
    let away = sim.situation(1, now);
    assert_eq!(away.goal_difference, 2);
    assert_eq!(away.sent_off, [2, 1]);
    // The run-of-play record drops a shot once it is older than the window.
    let window = ticks_for_minutes(15);
    sim.chances = vec![(now - window, 0, 0.4), (now - window + 1, 0, 0.2)];
    sim.rules_tick();
    assert_eq!(sim.chances, vec![(now - window + 1, 0, 0.2)]);
}

/// G7: the same rules play the same match under a person as under the computer manager, its
/// own changes held off.
#[test]
fn a_sides_rules_play_the_same_under_a_person_and_the_computer() {
    let play = |manager: Manager| {
        let mut config = shipped_config(5, 30).unwrap();
        config.modules.manager = &crate::ai::ManagerOff;
        let tactics = config.teams[0].tactics;
        let config = config.with_tactics(0, tactics).with_manager(0, manager);
        let state = GameState {
            rules: vec![
                Rule {
                    when: vec![Condition::TimeLeft { minutes: 20.0 }],
                    set: vec![level_of(&config, "tempo", "fast")],
                },
                Rule {
                    when: vec![Condition::TeamEnergy { below: 0.95 }],
                    set: vec![level_of(&config, "pressing", "low")],
                },
            ],
            instructions: Vec::new(),
        };
        let mut sim = Simulation::new(config.with_game_state(0, state)).unwrap();
        while !sim.is_over() {
            sim.step();
        }
        (sim.summary(), sim.applied.clone(), sim.take_events())
    };
    let (person, computer) = (play(Manager::Human), play(Manager::Ai));
    assert!(
        person
            .1
            .iter()
            .any(|c| matches!(c.change, Change::Rule { .. })),
        "no rule fired"
    );
    assert_eq!(person.1, computer.1);
    assert_eq!(person.0, computer.0);
    assert_eq!(person.2, computer.2);
}
