//! Tests of the team shape pass ([`crate::shape`]): they arrange match state directly, so
//! they live with the central loop.

use crate::ball::Ball;
use crate::data::attributes::Group;
use crate::data::tactics::StandOption;
use crate::data::tactics::TacticsSchema;
use crate::data::team::Position;
use crate::data::test_support::{
    shipped_config, shipped_config_with_stands, shipped_content, shipped_content_with_stands,
};
use crate::decision::Kick;
use crate::math::DVec2;
use crate::modules::modifier::{Effect, Family, Modifier};
use crate::modules::{CoverTrace, TargetsTrace};
use crate::modules::{MatchView, ROSTER};
use crate::shape::{ShapeOutcome, anchors, apply};
use crate::sim::{MatchConfig, Simulation};
use crate::tactics::Tactics;
use crate::team::PLAYERS_PER_TEAM;

/// The registry's weather slot (surroundings family, a stand-in with no effect): the slot a
/// test puts its familiarity modifier in. Registry order: fatigue, sharpness, pressure,
/// momentum, adaptation, weather.
const WEATHER_SLOT: usize = 5;

/// A familiarity modifier for tests only: `value` rating points on the mental group for
/// every home player. The registry has no familiarity modifier until the player record
/// feeds the family.
struct Familiar(f64);

impl Modifier for Familiar {
    fn family(&self) -> Family {
        Family::Familiarity
    }

    fn effect(&self, _: &MatchView<'_>, i: usize) -> Effect {
        let mut e = Effect::NEUTRAL;
        if i < PLAYERS_PER_TEAM {
            e.groups[Group::Mental as usize] = self.0;
        }
        e
    }
}

static FAMILIAR: Familiar = Familiar(0.25);
static UNFAMILIAR: Familiar = Familiar(-1.0);

/// A match on the shipped content whose home tactics `edit` sets from the defaults, with
/// the content's tactics file changed first by `schema`.
fn match_with(
    schema: impl FnOnce(&mut TacticsSchema),
    edit: impl FnOnce(&mut Tactics, &TacticsSchema),
) -> Simulation {
    match_on(shipped_config(7, 90).unwrap(), schema, edit)
}

/// [`match_with`] with the team stands on: the guard and cover scenes are built on the
/// players the stands move up the pitch.
fn match_with_stands(
    schema: impl FnOnce(&mut TacticsSchema),
    edit: impl FnOnce(&mut Tactics, &TacticsSchema),
) -> Simulation {
    match_on(shipped_config_with_stands(7, 90).unwrap(), schema, edit)
}

fn match_on(
    mut config: MatchConfig,
    schema: impl FnOnce(&mut TacticsSchema),
    edit: impl FnOnce(&mut Tactics, &TacticsSchema),
) -> Simulation {
    schema(&mut config.tactics);
    let mut tactics = Tactics::defaults(&config.tactics);
    edit(&mut tactics, &config.tactics);
    let config = config.with_tactics(0, tactics);
    Simulation::new(config).unwrap()
}

fn home(edit: impl FnOnce(&mut Tactics, &TacticsSchema)) -> Simulation {
    match_with(|_| {}, edit)
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
    sim.slide_reset();
}

/// The shape outcome of both teams for the match as it stands.
fn outcome(sim: &Simulation) -> ([ShapeOutcome; 2], [DVec2; ROSTER]) {
    let view = sim.view();
    let phase = view.possession_phase();
    let a = anchors(&view, phase);
    let mut targets = a;
    (apply(&view, phase, &a, &mut targets), targets)
}

/// One tick of the decision's targets and steering, with the carrier holding the ball
/// where he stands. Returns the shape outcome and the decision's trace of the tick.
fn settle_tick(sim: &mut Simulation) -> ([ShapeOutcome; 2], TargetsTrace) {
    let (shape, _) = outcome(sim);
    let decided = sim.config.modules.decision.targets(&sim.view());
    for (p, &t) in sim.players.iter_mut().zip(decided.targets.iter()) {
        p.target = t;
    }
    let held = sim.carrier.map(|c| (c, sim.players[c].pos));
    if let Some((c, at)) = held {
        sim.players[c].target = at;
    }
    let t = std::sync::Arc::clone(&sim.tuning);
    sim.steer_players(&t);
    if let Some((c, at)) = held {
        sim.players[c].pos = at;
        sim.players[c].vel = DVec2::ZERO;
    }
    sim.tick += 1;
    (shape, decided.trace)
}

fn ticks(sim: &Simulation, seconds: f64) -> u32 {
    (seconds / sim.tuning.dt).round() as u32
}

/// Every player of the team in possession and of the other team at his anchor.
fn at_anchors(sim: &mut Simulation) {
    let a = anchors(&sim.view(), sim.possession_phase);
    for (i, p) in sim.players.iter_mut().enumerate() {
        if Some(i) != sim.carrier && p.active() {
            p.pos = a[i];
            p.target = a[i];
        }
    }
}

/// Where player `i` stands by the content numbers alone: his formation slot, moved by the
/// line height out of possession, his duty, and his position's stand plus his role's offset
/// for the phase, widened, scaled to the ground and mirrored for his team, plus the ball
/// shift. Worked here from the tactics file, not through the anchor.
fn by_content(sim: &Simulation, schema: &TacticsSchema, i: usize, phase: bool) -> DVec2 {
    let p = &sim.players[i];
    let team = &sim.teams[p.team];
    let tactics = &team.tactics;
    let (fx, fy) = team.formation[p.slot];
    let position = schema.formations[usize::from(tactics.formation)].slots[p.slot].position;
    let rd = tactics.roles[p.slot];
    let role = &schema.roles[usize::from(rd.role)];
    let duty = &schema.duties[usize::from(rd.duty)];
    let ins = &schema.instructions;
    let line = ins.line_height.levels[usize::from(tactics.instructions[3])].value;
    let width = ins.width.levels[usize::from(tactics.instructions[1])].value;
    let stand = schema.stands.of(position);
    let (sx, sy, rx, ry) = if phase {
        (
            stand.in_possession.x,
            stand.in_possession.y,
            role.in_possession.x,
            role.in_possession.y,
        )
    } else {
        (
            stand.out_of_possession.x,
            stand.out_of_possession.y,
            role.out_of_possession.x,
            role.out_of_possession.y,
        )
    };
    let pitch = sim.config.pitch();
    let (gx, gy) = pitch.scale();
    // The line height moves the block out of possession only.
    let block = if phase { 0.0 } else { line };
    let depth = (fx + block + duty.depth + sx + rx) * gx;
    let side = if fy.abs() < 0.5 { 0.0 } else { fy.signum() };
    let ax = team.attack_x;
    let base = DVec2::new(
        (depth - pitch.half_length()) * ax,
        (fy * width + side * (sy + ry)) * gy * ax,
    );
    let ball = sim.ball.xy();
    let t = &sim.config.tuning;
    pitch.clamp(
        base + DVec2::new(ball.x * t.compactness_x, ball.y * t.compactness_y),
        0.5,
    )
}

/// Shape AC-1: with the ball held at the centre spot by each team in turn, every player
/// the shape pass, pressing and cover leave alone stands at his stand from his formation
/// slot, within 1 m once the shapes settle.
#[test]
fn each_position_settles_at_its_stand_from_its_slot_with_each_team_in_possession() {
    let content = shipped_content();
    for team in 0..2 {
        let mut sim = home(|_, _| {});
        let c = team * PLAYERS_PER_TEAM + 6;
        hold(&mut sim, c, DVec2::ZERO);
        let mut last = None;
        for _ in 0..ticks(&sim, 15.0) {
            last = Some(settle_tick(&mut sim));
        }
        let (shape, trace) = last.unwrap();
        let mut busy = [false; ROSTER];
        if let TargetsTrace::Carrier {
            count,
            pressers,
            cover,
            ..
        } = trace
        {
            for &(_, i) in &pressers[..count] {
                if i != usize::MAX {
                    busy[i] = true;
                }
            }
            if let CoverTrace::Covered { defender, .. } = cover {
                busy[defender] = true;
            }
        }
        let mut checked = 0;
        for (i, p) in sim.players.iter().enumerate() {
            if i == c || i == sim.keeper(p.team) || busy[i] || shape[p.team].touched(p.slot) {
                continue;
            }
            let want = by_content(&sim, &content.tactics, i, p.team == team);
            // In both phases a player who keeps his place less well than average trails his
            // place by the shortfall of his shape stage; out of possession a back-line player
            // who covers less well than average also steps with his line late, by the
            // shortfall of his cover stage, on the lagged target's depth.
            let shape_lag = (1.0 - p.derived.knobs.shape).max(0.0);
            let step = if p.team != team && sim.teams[p.team].back_line().contains(&p.slot) {
                (1.0 - p.derived.knobs.cover).max(0.0)
            } else {
                0.0
            };
            let lag = shape_lag + step * (1.0 + shape_lag);
            // In possession a player with off-ball movement above the average takes space at
            // his stand: his timed run moves his target across by up to that share of
            // `hooks.anchor_space_m` ([`crate::decision`]).
            let run = if p.team == team {
                let s = sim
                    .skills(i)
                    .skill(crate::contract::Stage::OFF_BALL_RUN_EXECUTE)
                    .max(0.0);
                sim.config.tuning.contract.hooks().anchor_space_m * s
            } else {
                0.0
            };
            assert!(
                (p.target - want).length() <= lag * (p.pos - want).length() + run + 1e-9,
                "team {team} in possession: player {i} target {} against {want}",
                p.target
            );
            assert!(
                (p.pos - want).length() < 1.0 + run,
                "team {team} in possession: player {i} stands at {} against {want}",
                p.pos
            );
            checked += 1;
        }
        assert!(checked >= 8, "team {team}: only {checked} players checked");
    }
}

/// Shape AC-2: a pass in flight, a deflection and the loose ball after it keep the
/// passing team in possession; the phase changes on the tick a player of the other team
/// controls the ball, not before.
#[test]
fn a_team_keeps_its_phase_until_the_other_team_controls_the_ball() {
    let mut sim = home(|_, _| {});
    let c = 6;
    hold(&mut sim, c, DVec2::new(-10.0, -5.0));
    // Every other player far from the pass's path but one away midfielder ahead of it.
    let taker = PLAYERS_PER_TEAM + 6;
    for i in 0..ROSTER {
        if i != c && i != taker && i != sim.keeper(0) && i != sim.keeper(1) {
            let side = if i < PLAYERS_PER_TEAM { -30.0 } else { 30.0 };
            sim.players[i].pos = DVec2::new(side, if i % 2 == 0 { 30.0 } else { -30.0 });
        }
    }
    sim.players[taker].pos = DVec2::new(8.0, -5.0);
    let t = std::sync::Arc::clone(&sim.tuning);
    // The home centre-back whose target the check reads stands at his place, behind the
    // pass: a player stranded far from play and from both his places heads for the nearer.
    sim.players[2].pos = sim.teams[0].anchor(2, sim.ball.xy(), true, &t);
    sim.apply_kick(
        Kick::Pass {
            dir: DVec2::X,
            speed: crate::decision::kick_speed(18.0, 0.0, &t),
            loft: 0.0,
        },
        &t,
        true,
    );
    assert_eq!(sim.carrier, None);
    assert_eq!(sim.pass_in_flight, Some(0));
    assert_eq!(
        sim.possession_phase,
        Some(0),
        "a pass in flight keeps the phase"
    );
    // During the flight both teams' anchors take the home team in possession.
    let view = sim.view();
    let a = anchors(&view, view.possession_phase());
    for i in [2, 3, 13, 14] {
        let p = view.player(i);
        let want = sim.teams[p.team].anchor(p.slot, view.ball().xy(), p.team == 0, &t);
        assert_eq!(a[i], want, "player {i}");
    }
    let home_cb = sim.config.modules.decision.targets(&view).targets[2];
    assert_eq!(home_cb, a[2], "a home centre-back stands in possession");
    // An away player's deflection keeps the home team in possession.
    sim.deflected_by(PLAYERS_PER_TEAM + 9);
    assert_eq!(sim.last_touch, Some(1));
    assert_eq!(
        sim.possession_phase,
        Some(0),
        "a deflection keeps the phase"
    );
    // The ball runs loose to the away midfielder: the phase holds until he controls it.
    let mut changed = None;
    for _ in 0..ticks(&sim, 6.0) {
        let before = sim.possession_phase;
        sim.step();
        if sim.carrier.is_some() {
            changed = Some((before, sim.carrier, sim.possession_phase));
            break;
        }
        assert_eq!(
            sim.possession_phase,
            Some(0),
            "loose ball at tick {}",
            sim.tick
        );
    }
    let (before, carrier, after) = changed.expect("somebody controls the ball");
    assert_eq!(before, Some(0));
    let team = sim.players[carrier.unwrap()].team;
    assert_eq!(after, Some(team), "the phase is the controlling team's");
    assert_eq!(
        team, 1,
        "the away midfielder in the ball's path controls it"
    );
}

/// Shape AC-4: duties that would send every outfield player forward, a carrier walking
/// forward from the centre spot, and at every sampled tick after a 2 s grace at least the
/// mentality's minimum stands goal-side of the ball. With the minimum edited to 0 the same
/// fixture falls below it, so the check can fail.
#[test]
fn rest_defence_keeps_the_minimum_goal_side_under_every_mentality() {
    let content = shipped_content_with_stands();
    let mut control_fell = false;
    for m in 0..content.tactics.mentalities.len() {
        let minimum = usize::from(content.tactics.mentalities[m].rest_defence);
        for guard in [true, false] {
            let mut sim = match_with_stands(
                |schema| {
                    if !guard {
                        for mentality in &mut schema.mentalities {
                            mentality.rest_defence = 0;
                        }
                    }
                },
                |tactics, schema| {
                    tactics.mentality = m as u8;
                    let attack = schema.duty_index("attack").unwrap() as u8;
                    for rd in &mut tactics.roles {
                        rd.duty = attack;
                    }
                },
            );
            let c = 6;
            hold(&mut sim, c, DVec2::ZERO);
            let grace = ticks(&sim, 2.0);
            let dt = sim.tuning.dt;
            let mut lowest = usize::MAX;
            for k in 0..ticks(&sim, 8.0) {
                // The carrier walks forward at 1 m/s.
                let at = DVec2::new(f64::from(k) * dt, 0.0);
                sim.players[c].pos = at;
                sim.ball = Ball::at(at);
                sim.slide_reset();
                settle_tick(&mut sim);
                if k < grace || k % 10 != 0 {
                    continue;
                }
                let behind = (1..PLAYERS_PER_TEAM)
                    .filter(|&i| i != c && sim.players[i].active())
                    .filter(|&i| sim.players[i].pos.x < sim.ball.pos.x)
                    .count();
                lowest = lowest.min(behind);
            }
            if guard {
                assert!(
                    lowest >= minimum,
                    "mentality {m}: {lowest} goal-side against the minimum {minimum}"
                );
            } else if lowest < minimum {
                control_fell = true;
            }
        }
    }
    assert!(
        control_fell,
        "without the minimum the fixture never fell below it: the check could not fail"
    );
}

/// Shape AC-5: with the minimum met an attacking full-back goes forward, and a familiar
/// team-mate stays to cover him while an unfamiliar one goes.
#[test]
fn an_attacking_duty_goes_and_a_familiar_team_mate_covers_the_full_back() {
    let scene = |familiar: Option<&'static Familiar>| {
        let mut sim = match_with_stands(
            |_| {},
            |tactics, schema| {
                tactics.mentality = schema.mentality_index("positive").unwrap() as u8;
                tactics.roles[1].duty = schema.duty_index("attack").unwrap() as u8;
            },
        );
        if let Some(m) = familiar {
            sim.config.modules.modifiers = sim.config.modules.modifiers.with(WEATHER_SLOT, m);
        }
        hold(&mut sim, 7, DVec2::ZERO);
        at_anchors(&mut sim);
        let (shape, targets) = outcome(&sim);
        (sim, shape[0], targets)
    };
    for familiar in [None, Some(&FAMILIAR)] {
        let (sim, shape, targets) = scene(familiar);
        assert!(sim.view().familiarity(6) >= 0.0);
        // The left-back's attacking duty takes him ahead of the ball.
        assert_eq!(shape.held & (1 << 1), 0, "the left-back is not held");
        assert!(targets[1].x > sim.ball.pos.x, "the left-back goes forward");
        let held_or_behind = (1..PLAYERS_PER_TEAM)
            .filter(|&i| i != 7 && targets[i].x < sim.ball.pos.x)
            .count();
        assert!(held_or_behind >= 3, "the positive minimum of 3 is met");
        assert_ne!(
            shape.covering, 0,
            "a familiar team-mate covers the left-back"
        );
        let j = (0..PLAYERS_PER_TEAM)
            .find(|&s| shape.covering & (1 << s) != 0)
            .unwrap();
        assert!(targets[j].x < 0.0, "the cover stands goal-side");
    }
    let (sim, shape, _) = scene(Some(&UNFAMILIAR));
    assert_eq!(sim.view().familiarity(6), -1.0);
    assert_eq!(shape.covering, 0, "an unfamiliar team-mate goes forward");
}

/// Lays out the home team for a menu scene: every home outfield player but `subject` and
/// the carrier far from him on the other flank and far up the pitch.
fn clear_around(sim: &mut Simulation, subject: usize) {
    for i in 1..PLAYERS_PER_TEAM {
        if i != subject && Some(i) != sim.carrier {
            sim.players[i].pos = DVec2::new(45.0, 30.0);
        }
    }
}

/// Where the carrier holds the ball so that he stands at `offset` from `subject`'s
/// anchor, which itself moves with the ball: solved by repeating the step.
fn carrier_at(sim: &mut Simulation, c: usize, subject: usize, offset: impl Fn(DVec2) -> DVec2) {
    let mut at = DVec2::ZERO;
    for _ in 0..40 {
        hold(sim, c, at);
        let a = anchors(&sim.view(), sim.possession_phase)[subject];
        at = offset(a);
    }
    hold(sim, c, at);
}

/// The option home slot `slot` takes as role `role` once `arrange` has set the scene. The
/// rest-defence guard is off, so the scene shows the menu alone.
fn pick(role: &str, slot: usize, arrange: impl FnOnce(&mut Simulation)) -> Option<StandOption> {
    let mut sim = match_with(
        |schema| schema.rest_defence = None,
        |tactics, schema| {
            tactics.roles[slot].role = schema.role_index(role).unwrap() as u8;
        },
    );
    arrange(&mut sim);
    outcome(&sim).0[0].option[slot]
}

/// Slot 6 of the home 4-4-2: a left-sided central slot.
const MID: usize = 6;

fn drop_scene(sim: &mut Simulation) {
    // The carrier 20 m behind the player, on his line: nothing else holds.
    carrier_at(sim, 2, MID, |a| a - DVec2::new(20.0, 0.0));
    clear_around(sim, MID);
}

fn half_space_scene(sim: &mut Simulation) {
    // The carrier far ahead on the centre line; a team-mate wider on the player's side.
    carrier_at(sim, 10, MID, |a| DVec2::new(a.x + 30.0, 0.0));
    clear_around(sim, MID);
    let a = anchors(&sim.view(), sim.possession_phase)[MID];
    sim.players[5].pos = a + DVec2::new(3.0, -8.0);
}

fn widen_scene(sim: &mut Simulation) {
    // The carrier far ahead on the centre line; a team-mate just inside the player.
    carrier_at(sim, 10, MID, |a| DVec2::new(a.x + 30.0, 0.0));
    clear_around(sim, MID);
    let a = anchors(&sim.view(), sim.possession_phase)[MID];
    sim.players[7].pos = a + DVec2::new(2.0, 4.0);
}

fn overlap_scene(sim: &mut Simulation, across: f64) {
    // The carrier 10 m ahead of the player on his side, `across` from the centre line.
    carrier_at(sim, 5, MID, |a| DVec2::new(a.x + 10.0, -across));
    clear_around(sim, MID);
}

/// Shape AC-6: every role with a menu takes each option of its menu when the ball and
/// its team-mates meet that option's condition, the first in menu order when two hold.
#[test]
fn each_role_takes_each_option_of_its_menu_when_its_condition_holds() {
    let content = shipped_content();
    let mut seen = 0;
    for role in &content.tactics.roles {
        let menu = &role.options.in_possession;
        for (k, &option) in menu.iter().enumerate() {
            // A back-line player widens in build-up: a full-back or a centre-back slot.
            let back = role
                .positions
                .iter()
                .any(|p| matches!(p, Position::CB | Position::LB | Position::RB));
            let got = match option {
                StandOption::Drop => pick(&role.name, MID, drop_scene),
                StandOption::HalfSpace => pick(&role.name, MID, half_space_scene),
                StandOption::Widen if back => pick(&role.name, 2, |sim| {
                    hold(sim, 0, DVec2::new(-48.0, 0.0));
                    clear_around(sim, 2);
                }),
                StandOption::Widen => pick(&role.name, MID, widen_scene),
                StandOption::Overlap => pick(&role.name, MID, |s| overlap_scene(s, 18.0)),
                StandOption::Underlap => pick(&role.name, MID, |s| overlap_scene(s, 30.0)),
            };
            assert_eq!(
                got,
                Some(option),
                "role {} option {} (menu place {k})",
                role.name,
                option.name()
            );
            seen += 1;
        }
    }
    assert!(seen >= 20, "{seen} role options checked");
    // Two conditions at once: the drop and the half-space; the menu's order decides.
    let both = pick("central_midfielder", MID, |sim| {
        drop_scene(sim);
        let a = anchors(&sim.view(), sim.possession_phase)[MID];
        sim.players[5].pos = a + DVec2::new(3.0, -8.0);
    });
    assert_eq!(both, Some(StandOption::Drop));
}

/// Shape AC-6, out of possession: a defensive midfielder drops into a gap in the back
/// line.
#[test]
fn a_defensive_midfielder_drops_into_a_gap_in_the_back_line() {
    let gap = |wide: bool| {
        pick("defensive_midfielder", MID, |sim| {
            hold(sim, PLAYERS_PER_TEAM + 9, DVec2::new(-10.0, 0.0));
            let ys = if wide {
                [-20.0, -15.0, 10.0, 20.0]
            } else {
                [-18.0, -6.0, 6.0, 18.0]
            };
            for (slot, y) in (1..5).zip(ys) {
                sim.players[slot].pos = DVec2::new(-30.0, y);
            }
        })
    };
    assert_eq!(gap(true), Some(StandOption::Drop));
    assert_eq!(gap(false), None, "a closed back line draws nobody");
}

/// A settled pick holds: in build-up with the keeper on the ball, once the shapes settle
/// every option the home team takes stays the same for 5 s of steering.
#[test]
fn a_settled_pick_holds_for_five_seconds() {
    let mut sim = home(|_, _| {});
    hold(&mut sim, 0, DVec2::new(-48.0, 0.0));
    for _ in 0..ticks(&sim, 4.0) {
        settle_tick(&mut sim);
    }
    let (first, _) = settle_tick(&mut sim);
    assert!(first[0].option.iter().any(Option::is_some), "{first:?}");
    for _ in 0..ticks(&sim, 5.0) {
        let (now, _) = settle_tick(&mut sim);
        assert_eq!(now[0].option, first[0].option, "tick {}", sim.tick);
    }
}

/// The neutral plan and a tactics file with no stand options or guard leave every target
/// at its anchor.
#[test]
fn a_file_without_options_or_guard_moves_nobody() {
    let mut sim = match_with(
        |schema| {
            schema.stand_options = None;
            schema.rest_defence = None;
        },
        |_, _| {},
    );
    hold(&mut sim, 6, DVec2::ZERO);
    let view = sim.view();
    let a = anchors(&view, Some(0));
    let mut targets = a;
    let out = apply(&view, Some(0), &a, &mut targets);
    assert_eq!(out, [ShapeOutcome::default(); 2]);
    assert_eq!(targets, a);
}
