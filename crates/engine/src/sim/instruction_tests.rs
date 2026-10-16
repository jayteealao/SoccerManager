//! Player instructions at the hooks: each override of the list sets only its own inputs in
//! the instructed slot's plan, and the slot's decision moves at the same hook that reads the
//! side's value while a team-mate's stays the side's.

use crate::data::primitives::level_value;
use crate::data::test_support::shipped_config;
use crate::decision::{COUNTER_WINDOW_S, tackle_decision};
use crate::math::DVec2;
use crate::modules::TackleChances;
use crate::pace::Job;
use crate::sim::Simulation;
use crate::tactics::TeamPlan;
use crate::tactics::rules::{GameState, PlayerInstruction};
use crate::team::PLAYERS_PER_TEAM;

/// A seed-7 one-minute match on the shipped content with the options' noise off.
fn calm() -> Simulation {
    let mut config = shipped_config(7, 1).unwrap();
    config.tuning.decision.noise = 0.0;
    Simulation::new(config).unwrap()
}

/// The index of primitive `name` and of its level `level`.
fn level_of(sim: &Simulation, name: &str, level: &str) -> (u8, u8) {
    let schema = &sim.config.tactics;
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
    (p as u8, l as u8)
}

/// Gives the player in `slot` of `team` the instruction `primitive` at `level`.
fn instruct(sim: &mut Simulation, team: usize, slot: usize, primitive: &str, level: &str) {
    let (primitive, level) = level_of(sim, primitive, level);
    let state = GameState {
        rules: Vec::new(),
        instructions: vec![PlayerInstruction {
            slot: slot as u8,
            primitive,
            level,
        }],
    };
    let schema = sim.config.tactics.clone();
    let tuning = sim.config.tuning.clone();
    sim.teams[team].set_game_state(state, &schema, &tuning);
}

/// Every override of the shipped list, one level above its default (the first choice that is
/// not the default for a choice): the instructed slot's plan holds the level's value of each
/// of the override's inputs and the side's value of everything else, and every other slot
/// reads the side's plan.
#[test]
fn each_override_sets_only_its_inputs_in_its_slots_plan() {
    let probe = calm();
    let schema = probe.config.tactics.clone();
    let t = probe.config.tuning.clone();
    let block = schema.game_state.clone().unwrap();
    assert_eq!(block.player_overrides.len(), 10);
    for entry in &block.player_overrides {
        let mut sim = calm();
        let p = schema
            .primitives
            .iter()
            .position(|p| p.name == entry.primitive)
            .unwrap();
        let prim = &schema.primitives[p];
        let default = prim.default_index(&schema).unwrap();
        let names = prim.level_names(&schema);
        let level = if prim.kind == crate::data::primitives::PrimitiveKind::Choice {
            (0..names.len()).find(|&l| l != default).unwrap()
        } else {
            default + 1
        };
        // An attack-line slot, which every override reaches.
        let slot = (1..PLAYERS_PER_TEAM)
            .find(|&s| crate::decision::attack_line(sim.teams[0].plan.slots[s].position))
            .unwrap();
        instruct(&mut sim, 0, slot, &entry.primitive, names[level]);
        let side = sim.teams[0].plan;
        let own = *sim.teams[0].plan_for(slot);
        for input in &entry.inputs {
            let value = if prim.owns_levels()
                || prim.kind == crate::data::primitives::PrimitiveKind::Choice
            {
                prim.own_value(input, level)
            } else {
                level_value(&schema, input, level)
            };
            let want = match (input.as_str(), value) {
                ("press_distance", Some(v)) => t.press_distance * v,
                ("pace_lift", Some(v)) => f64::from(u8::from(v > 0.0)),
                (_, Some(v)) => v,
                (_, None) => TeamPlan::neutral(&t).player_input(input).unwrap(),
            };
            assert_eq!(
                own.player_input(input),
                Some(want),
                "{} sets {input}",
                entry.primitive
            );
        }
        assert_eq!(
            own.with_override(p, &entry.inputs, default, &schema, &t),
            side,
            "{}: nothing but its inputs moves",
            entry.primitive
        );
        for other in (0..PLAYERS_PER_TEAM).filter(|&s| s != slot) {
            assert_eq!(*sim.teams[0].plan_for(other), side, "{}", entry.primitive);
        }
        assert_eq!(*sim.teams[1].plan_for(slot), sim.teams[1].plan);
    }
}

/// A home carrier `c` alone at `at` facing the goal he attacks: every outfield opponent and
/// team-mate far off, the ball at his feet.
fn alone(sim: &mut Simulation, c: usize, at: DVec2) {
    let ax = sim.teams[0].attack_x;
    let (own_keeper, their_keeper) = (sim.keeper(0), sim.keeper(1));
    for (i, p) in sim.players.iter_mut().enumerate() {
        if p.team == 1 && p.id != their_keeper {
            p.pos = DVec2::new(-ax * 50.0, p.pos.y);
        } else if p.team == 0 && i != c && p.id != own_keeper {
            p.pos = DVec2::new(-ax * 50.0, 30.0);
        }
        p.vel = DVec2::ZERO;
    }
    sim.players[c].pos = at;
    sim.players[c].facing = DVec2::new(ax, 0.0);
    sim.ball.pos = at.extend(0.0);
    sim.carrier = Some(c);
    sim.possession_phase = Some(0);
    sim.tick = 2_000;
    sim.control_since = 2_000;
}

/// Dribbling (hook `dribble.weight`): the instructed carrier's dribble score rises by the
/// level's weight; a team-mate's instruction leaves his score as it was.
#[test]
fn a_dribbling_instruction_moves_only_its_players_dribble_score() {
    let at = DVec2::new(0.0, 2.0);
    let dribble = |slot: Option<usize>| {
        let mut sim = calm();
        if let Some(s) = slot {
            instruct(&mut sim, 0, s, "dribbling", "dribble_often");
        }
        alone(&mut sim, 6, at);
        sim.options(6).dribble.expect("a dribble")
    };
    let base = dribble(None);
    assert!((dribble(Some(6)) - base - 0.0625).abs() < 1e-12);
    assert_eq!(dribble(Some(7)), base);
}

/// Final third (hook `shot.range`): a carrier past the side's shot range has no shot, and the
/// instruction that reaches farther gives the instructed carrier one, not a team-mate.
#[test]
fn a_final_third_instruction_moves_only_its_players_shot_range() {
    let shot = |slot: Option<usize>| {
        let mut sim = calm();
        if let Some(s) = slot {
            instruct(&mut sim, 0, s, "final_third", "shoot_on_sight");
        }
        let ax = sim.teams[0].attack_x;
        let goal = sim.teams[0].target_goal();
        let at = goal - DVec2::new(ax * (sim.config.tuning.shot_range + 4.0), 0.0);
        alone(&mut sim, 9, at);
        sim.options(9).shot
    };
    assert_eq!(shot(None), None);
    assert!(shot(Some(9)).is_some());
    assert_eq!(shot(Some(10)), None);
}

/// On winning the ball (hook `regain.counter`): inside the regain window the instructed
/// carrier's forward pass scores more; a team-mate's instruction moves nothing.
#[test]
fn an_on_winning_instruction_moves_only_its_players_counter() {
    let pass = |slot: Option<usize>| {
        let mut sim = calm();
        if let Some(s) = slot {
            instruct(&mut sim, 0, s, "on_winning_the_ball", "always_counter");
        }
        let ax = sim.teams[0].attack_x;
        alone(&mut sim, 9, DVec2::new(0.0, 2.0));
        sim.players[10].pos = DVec2::new(ax * 30.0, 2.0);
        let window = (COUNTER_WINDOW_S / 2.0 / sim.config.tuning.dt) as u32;
        sim.regained_at[0] = Some(2_000 - window);
        sim.options(9).pass.expect("an open team-mate").0
    };
    let base = pass(None);
    assert!((pass(Some(9)) - base - 2.0 * 30.0 / 45.0).abs() < 1e-9);
    assert_eq!(pass(Some(10)), base);
}

/// Width (hook `shape.width`): the instructed player's anchor with the ball moves across by
/// his own width; a team-mate's anchor stays the side's.
#[test]
fn a_width_instruction_moves_only_its_players_anchor() {
    let mut sim = calm();
    let t = sim.config.tuning.clone();
    let wide_slot = (1..PLAYERS_PER_TEAM)
        .find(|&s| sim.teams[0].slot_side(s) != 0.0 && !sim.teams[0].in_back_line(s))
        .unwrap();
    let mate = (1..PLAYERS_PER_TEAM)
        .find(|&s| s != wide_slot && sim.teams[0].slot_side(s) != 0.0)
        .unwrap();
    let before = [wide_slot, mate].map(|s| sim.teams[0].anchor(s, DVec2::ZERO, true, &t));
    instruct(&mut sim, 0, wide_slot, "width", "very_wide");
    let after = [wide_slot, mate].map(|s| sim.teams[0].anchor(s, DVec2::ZERO, true, &t));
    assert!(after[0].y.abs() > before[0].y.abs(), "{before:?} {after:?}");
    assert_eq!(after[0].x, before[0].x);
    assert_eq!(after[1], before[1]);
    // Out of possession the anchor reads the defensive width, which no instruction sets.
    assert_eq!(
        sim.teams[0].anchor(wide_slot, DVec2::ZERO, false, &t),
        calm().teams[0].anchor(wide_slot, DVec2::ZERO, false, &t)
    );
}

/// The farthest an away player `d` stands from the home carrier at the centre and still
/// presses him, in whole metres from 1 to 60, every other away outfield player far off; the
/// away side lost the ball `since_s` seconds ago (`None`: no loss), and `instruct` gives the
/// away side its instructions first.
fn press_reach(d: usize, since_s: Option<f64>, instruct: impl Fn(&mut Simulation)) -> u32 {
    let mut farthest = 0;
    for m in 1..=60u32 {
        let mut sim = calm();
        instruct(&mut sim);
        let ax = sim.teams[0].attack_x;
        let their_keeper = sim.keeper(1);
        for i in PLAYERS_PER_TEAM..2 * PLAYERS_PER_TEAM {
            if i != their_keeper {
                sim.players[i].pos = DVec2::new(ax * 50.0, 30.0);
            }
            sim.players[i].vel = DVec2::ZERO;
        }
        sim.players[6].pos = DVec2::ZERO;
        sim.players[6].vel = DVec2::ZERO;
        sim.players[d].pos = DVec2::new(ax * f64::from(m), 0.0);
        sim.ball.pos = DVec2::ZERO.extend(0.0);
        sim.carrier = Some(6);
        sim.possession_phase = Some(0);
        sim.tick = 2_000;
        if let Some(s) = since_s {
            sim.lost_at[1] = Some(2_000 - (s / sim.config.tuning.dt) as u32);
        }
        let targets = sim.config.modules.decision.targets(&sim.view());
        if targets.jobs[d] == Job::Press {
            farthest = m;
        }
    }
    farthest
}

/// Pressing (hook `press.distance`): the instructed player's press reach is his own level's;
/// a team-mate's instruction leaves his reach as it was.
#[test]
fn a_pressing_instruction_moves_only_its_players_reach() {
    let d = PLAYERS_PER_TEAM + 6;
    let base = press_reach(d, None, |_| {});
    // The low level's press distance scale is three times the default's.
    let own = press_reach(d, None, |sim| instruct(sim, 1, 6, "pressing", "low"));
    let mate = press_reach(d, None, |sim| instruct(sim, 1, 7, "pressing", "low"));
    assert!(own > base, "{own} against {base}");
    assert_eq!(mate, base);
}

/// On losing the ball (hook `loss.counter_press`): inside the side's counter-press window the
/// instructed player presses from farther; a team-mate's instruction moves nothing.
#[test]
fn an_on_losing_instruction_moves_only_its_players_counter_press() {
    let d = PLAYERS_PER_TEAM + 6;
    let since = Some(1.0);
    let base = press_reach(d, since, |_| {});
    let own = press_reach(d, since, |sim| {
        instruct(sim, 1, 6, "on_losing_the_ball", "always_counter_press");
        // A counter-press window long enough to hold the loss.
        sim.teams[1].plan.counter_press_s = 5.0;
    });
    let mate = press_reach(d, since, |sim| {
        instruct(sim, 1, 7, "on_losing_the_ball", "always_counter_press");
        sim.teams[1].plan.counter_press_s = 5.0;
    });
    let window = press_reach(d, since, |sim| sim.teams[1].plan.counter_press_s = 5.0);
    assert_eq!(
        window, base,
        "the window alone moves nothing at the default"
    );
    assert!(own > base, "{own} against {base}");
    assert_eq!(mate, base);
}

/// Tackling (hooks `tackle.decide` and `tackle.foul_risk`): the instructed defender's tackle
/// fouls more often in the tackles he makes; a team-mate's tackle stays the side's.
#[test]
fn a_tackling_instruction_moves_only_its_players_tackle() {
    let fair = TackleChances {
        p_win: 0.25,
        p_foul: 0.1,
        ball_loss: 0.5,
    };
    let decide = |slot: Option<usize>, tackler: usize| {
        let mut sim = calm();
        if let Some(s) = slot {
            instruct(&mut sim, 1, s, "tackling", "get_stuck_in");
        }
        tackle_decision(&sim.view(), PLAYERS_PER_TEAM + tackler, 6, fair)
    };
    assert_eq!(decide(None, 6), Some(fair));
    let risky = decide(Some(6), 6).unwrap();
    assert!(risky.p_foul > fair.p_foul, "{risky:?}");
    assert_eq!(risky.p_win, fair.p_win);
    assert_eq!(decide(Some(7), 6), Some(fair));
}

/// Forward runs (hook `run.start`): with the side at the default nobody runs; the instructed
/// striker onside within his own start distance of the line runs in behind, and a striker
/// whose team-mate holds the instruction does not.
#[test]
fn a_forward_runs_instruction_moves_only_its_players_runs() {
    let job = |slot: Option<usize>| {
        let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
        if let Some(s) = slot {
            instruct(&mut sim, 0, s, "forward_runs", "run_in_behind");
        }
        let carrier = 6;
        sim.players[carrier].pos = DVec2::new(0.0, 0.0);
        let their_keeper = sim.keeper(1);
        for (k, p) in sim
            .players
            .iter_mut()
            .filter(|p| p.team == 1 && p.id != their_keeper)
            .enumerate()
        {
            p.pos = DVec2::new(30.0, -27.0 + 6.0 * k as f64);
        }
        // 1.5 m short of the onside line at 29.5 m: inside the instruction's 1.875 m.
        let near = DVec2::new(28.0, -6.0);
        sim.players[9].pos = near;
        sim.players[9].target = near;
        sim.carrier = Some(carrier);
        sim.tick = 100;
        let view = sim.view();
        sim.config.modules.decision.targets(&view).jobs[9]
    };
    assert_ne!(job(None), Job::TimedRun);
    assert_eq!(job(Some(9)), Job::TimedRun);
    assert_ne!(job(Some(10)), Job::TimedRun);
}

/// Crossing, cross type and on winning the ball's pace lift reach their hooks through the
/// carrier's and the runner's own plans: the instructed slot's plan holds the level's cross
/// weight, early weight, loft and lift, and the plans of the side and a team-mate do not.
#[test]
fn crossing_cross_type_and_the_lift_are_read_off_the_players_own_plan() {
    let mut sim = calm();
    instruct(&mut sim, 0, 5, "crossing", "cross_often");
    assert_eq!(sim.teams[0].plan_for(5).crossing, 0.3);
    assert_eq!(
        sim.teams[0].plan_for(8).crossing,
        sim.teams[0].plan.crossing
    );
    let mut sim = calm();
    instruct(&mut sim, 0, 8, "cross_type", "floated");
    assert_eq!(sim.teams[0].plan_for(8).cross_loft, 1.0);
    assert_eq!(sim.teams[0].plan_for(8).early_cross, 0.0);
    assert_eq!(
        sim.teams[0].plan_for(5).cross_loft,
        sim.teams[0].plan.cross_loft
    );
    let mut sim = calm();
    instruct(&mut sim, 0, 9, "on_winning_the_ball", "counter");
    assert!(sim.teams[0].plan_for(9).pace_lift);
    assert!(!sim.teams[0].plan_for(10).pace_lift);
}
