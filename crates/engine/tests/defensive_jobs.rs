//! The defensive jobs: the press, shape, cover and recovery-run stages carry the decided main
//! attribute, shown supports and small weight, and each stage's decision moves
//! when a scripted scene changes its main attribute on one player from 5.0 to 18.0:
//! the press angle, the holding distance, the cover step and lead, and who takes a
//! recovery run and how likely it is a sprint. Each scene carries a control that must not
//! move. The rules that measure the stages in a match run and report in `sensitivity.rs`
//! (`the_defensive_job_rules_run_and_report`).

mod common;

use common::{calm_match, index, spread};
use engine::contract::{ActionKind, Stage, StageKind, StageTable};
use engine::data::attributes::{AttributeSchema, Group};
use engine::math::{DVec2, DVec3};
use engine::modules::{CoverTrace, TargetsTrace};
use engine::pace::Job;
use engine::scenario::Scene;

/// Both sides placed far from the scene: home outfield on `x = -45`, away on `x = 45`.
fn far() -> Scene {
    spread(Scene::new(calm_match(90)), -45.0, 45.0)
}

fn table(s: &AttributeSchema, action: ActionKind, stage: StageKind) -> &StageTable {
    &s.actions[&action][&stage]
}

fn names(list: &[engine::contract::Weighted]) -> Vec<&str> {
    list.iter().map(|w| w.attribute.as_str()).collect()
}

/// The decided tables of the four stages in `s`.
fn assert_decided(s: &AttributeSchema, familiarity: &[Group]) {
    let press = table(s, ActionKind::Press, StageKind::Choose);
    assert_eq!(press.main.attribute, "positioning");
    assert_eq!(names(&press.supports), ["anticipation", "decisions"]);
    assert_eq!(names(&press.minor), ["vision"]);

    let shape = table(s, ActionKind::Shape, StageKind::Execute);
    assert_eq!(shape.main.attribute, "teamwork");
    assert_eq!(names(&shape.supports), ["concentration", "consistency"]);
    assert!(shape.minor.is_empty());
    // The player's grasp of the tactic is his familiarity: its family moves the mental
    // group, and every attribute the shape stage blends is mental.
    assert!(familiarity.contains(&Group::Mental));
    for name in
        std::iter::once(&shape.main.attribute).chain(shape.supports.iter().map(|w| &w.attribute))
    {
        let def = &s.attributes[s.index(name).unwrap()];
        assert_eq!(def.group, Group::Mental, "{name}");
    }

    let cover = table(s, ActionKind::Cover, StageKind::Execute);
    assert_eq!(cover.main.attribute, "positioning");
    assert_eq!(names(&cover.supports), ["anticipation", "concentration"]);
    assert!(cover.minor.is_empty());

    let run = table(s, ActionKind::RecoveryRun, StageKind::Execute);
    assert_eq!(run.main.attribute, "work_rate");
    assert_eq!(names(&run.supports), ["pace", "acceleration"]);
    assert!(run.minor.is_empty());

    let work = &s.attributes[s.index("work_rate").unwrap()].job;
    assert_eq!(
        (work.action, work.stage.stage()),
        (Some(ActionKind::RecoveryRun), Some(StageKind::Execute))
    );
}

/// The shipped catalogue, and the version 4 catalogue converted on load, carry
/// the decided main attribute, shown supports and small weight of each stage.
#[test]
fn the_four_stages_carry_the_decided_weights_shipped_and_converted() {
    let content = common::content();
    let familiarity = &content
        .tuning
        .engine
        .contract
        .states
        .family_groups
        .familiarity;
    assert_decided(&content.attributes, familiarity);
    let bytes = std::fs::read(common::fixture_path("attributes-v4.json")).unwrap();
    let converted = engine::data::load_attributes_bytes(&bytes, "attributes-v4.json").unwrap();
    assert_eq!(converted.converted_from, Some(4));
    assert_decided(&converted.value, familiarity);
}

/// The relative move from `low` to `high`, up.
fn rose(low: f64, high: f64) -> f64 {
    (high - low) / low.abs().max(1e-9)
}

// --- Press angle.

/// The home striker stands with the ball at (0, 0); away midfielder 6, rated `v` in
/// positioning, stands at (6, 4) and presses him, with the lean hook at `lean_m`. How far his
/// target lies toward his own goal from the carrier, in metres.
fn press_angle(v: f64, lean_m: f64) -> f64 {
    let carrier = index(0, 9);
    let presser = index(1, 6);
    let mut config = calm_match(90);
    if let Some(h) = config.tuning.contract.hooks.as_mut() {
        h.press_goal_side_m = lean_m;
    }
    let sim = spread(Scene::new(config), -45.0, 45.0)
        .place(carrier, DVec2::ZERO)
        .place(presser, DVec2::new(6.0, 4.0))
        .ball(DVec3::ZERO)
        .carrier(Some(carrier))
        .tick(1_000)
        .rating(presser, "positioning", v)
        .build();
    let t = sim.config().modules.decision.targets(&sim.view());
    match t.trace {
        TargetsTrace::Carrier {
            pressers, count, ..
        } => assert!(
            pressers[..count].iter().any(|&(_, i)| i == presser),
            "he presses"
        ),
        other => panic!("no carrier: {other:?}"),
    }
    assert_eq!(t.jobs[presser], Job::Press);
    let goal = sim.teams()[1].own_goal();
    let to_goal = (goal - DVec2::ZERO).normalize();
    t.targets[presser].dot(to_goal)
}

/// The press stage: a good presser (positioning 18) aims goal-side of the
/// carrier, a poor one (5) from the open side; with the lean hook at 0 both aim alike.
#[test]
fn positioning_leans_the_press_goal_side() {
    let (low, high) = (press_angle(5.0, 2.0), press_angle(18.0, 2.0));
    println!("press angle: 5: {low:+.3} m  18: {high:+.3} m");
    assert!(low < 0.0 && high > 0.0, "{low} {high}");
    assert!(rose(low, high) >= 0.10);
    let (low, high) = (press_angle(5.0, 0.0), press_angle(18.0, 0.0));
    assert!((high - low).abs() < 1e-9, "control: {low} {high}");
}

// --- Holding distance.

/// Home player `slot`, rated `v` in teamwork, stands 10 m across from his anchor; the ball is
/// with away striker 9 at (30, 20) (`home_has` false) or with home striker 9 at (-30, -20)
/// (`home_has` true). `adjust` changes the scene before it is built. How far his target lies
/// from his anchor.
fn holding(slot: usize, v: f64, home_has: bool, adjust: impl Fn(Scene, usize) -> Scene) -> f64 {
    let me = index(0, slot);
    let (carrier, at) = if home_has {
        (index(0, 9), DVec2::new(-30.0, -20.0))
    } else {
        (index(1, 9), DVec2::new(30.0, 20.0))
    };
    let phase = Some(usize::from(!home_has));
    let scene = |pos: DVec2| {
        let s = far()
            .place(carrier, at)
            .place(me, pos)
            .ball(DVec3::new(at.x, at.y, 0.0))
            .carrier(Some(carrier))
            .phase(phase)
            .tick(1_000)
            .rating(me, "teamwork", v);
        adjust(s, me).build()
    };
    let first = scene(DVec2::ZERO);
    let anchor = engine::shape::anchors(&first.view(), phase)[me];
    let sim = scene(anchor + DVec2::new(0.0, 10.0));
    let again = engine::shape::anchors(&sim.view(), phase)[me];
    assert!((again - anchor).length() < 1e-9, "the anchor is his own");
    let t = sim.config().modules.decision.targets(&sim.view());
    assert!(
        !matches!(t.jobs[me], Job::Press | Job::Carry),
        "{:?}",
        t.jobs[me]
    );
    (t.targets[me] - anchor).length()
}

/// The shape stage: in both phases, a player who keeps his place well
/// (teamwork 18) takes his anchor and a poor one (5) trails it; a familiarity drop (playing a
/// position he is rated 1 in, natural 16) lengthens the holding distance against the same
/// player at his natural position.
#[test]
fn teamwork_and_familiarity_hold_the_place_in_both_phases() {
    // Midfielder 5: no stand option of his menus fires in either scene, so his target is his
    // anchor and the lag alone.
    let none = |s: Scene, _| s;
    for home_has in [false, true] {
        let (low, high) = (
            holding(5, 5.0, home_has, none),
            holding(5, 18.0, home_has, none),
        );
        println!("holding (in possession {home_has}): 5: {low:.3} m  18: {high:.3} m");
        assert!(
            high < 1e-9,
            "control: a good holder takes his anchor, {high}"
        );
        // The distance falls by all of itself (a move of 100 percent, at least 10).
        assert!(low > 0.05, "a poor holder trails it, {low}");
    }
    let natural = |s: Scene, me: usize| s.familiar(me);
    let away = |s: Scene, me: usize| {
        let played = s.played_position(me);
        let other = engine::data::team::Position::ALL
            .into_iter()
            .find(|&p| p != played)
            .unwrap();
        s.position_rating(me, other, 16.0)
            .position_rating(me, played, 1.0)
    };
    for home_has in [false, true] {
        let (at_home, out) = (
            holding(5, 8.0, home_has, natural),
            holding(5, 8.0, home_has, away),
        );
        println!("familiarity (in possession {home_has}): natural {at_home:.3} m  away {out:.3} m");
        assert!(
            out > at_home * 1.10 && out > at_home + 0.05,
            "{at_home} {out}"
        );
    }
}

// --- Cover step and lead.

/// The home back line out of possession: away striker 9 carries 20 m into the home half at
/// 2 m across, running at `run` toward the home goal (x by the attack direction), with velocity
/// `run`; the home back-line player farthest from him, rated `v` in positioning and 18 in
/// teamwork, stands 6 m behind his place in depth. Returns his depth gap from his place,
/// and the cover defender's distance from the spot he would take with no lead.
fn cover_scene(v: f64, run: DVec2) -> (f64, f64) {
    let carrier = index(1, 9);
    let ax = far().build().teams()[0].attack_x;
    let at = DVec2::new(-20.0 * ax, 2.0);
    let run = DVec2::new(run.x * ax, run.y);
    let build = |me: Option<(usize, DVec2)>, run: DVec2, rated: Option<usize>| {
        let mut s = far()
            .place(carrier, at)
            .ball(DVec3::new(at.x, at.y, 0.0))
            .carrier(Some(carrier))
            .phase(Some(1))
            .velocity(carrier, run)
            .tick(1_000);
        if let Some((i, pos)) = me {
            s = s.place(i, pos);
        }
        if let Some(i) = rated {
            s = s.rating(i, "positioning", v).rating(i, "teamwork", 18.0);
        }
        s.build()
    };
    let probe = build(None, run, None);
    let teams = probe.teams();
    let line: Vec<usize> = teams[0]
        .back_line()
        .into_iter()
        .map(|s| index(0, s))
        .collect();
    let t = probe.config().modules.decision.targets(&probe.view());
    let defender = match t.trace {
        TargetsTrace::Carrier {
            cover: CoverTrace::Covered { defender, .. },
            ..
        } => defender,
        other => panic!("no cover: {other:?}"),
    };
    let me = line
        .iter()
        .copied()
        .filter(|&i| i != defender)
        .max_by(|&a, &b| {
            let d = |i: usize| (probe.players()[i].pos - at).length();
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    // The step: his anchor is his place; he stands 6 m behind it toward his goal.
    let anchor = engine::shape::anchors(&probe.view(), Some(1))[me];
    let behind = anchor + DVec2::new(-6.0 * teams[0].attack_x, 0.0);
    let sim = build(Some((me, behind)), run, Some(me));
    let t = sim.config().modules.decision.targets(&sim.view());
    assert!(!matches!(t.jobs[me], Job::Press), "he does not press");
    let gap = (t.targets[me].x - anchor.x).abs();
    // The lead: the cover defender rated `v`, with the carrier running and standing.
    let lead = |run: DVec2| {
        let sim = build(None, run, Some(defender));
        sim.config().modules.decision.targets(&sim.view()).targets[defender]
    };
    (gap, (lead(run) - lead(DVec2::ZERO)).length())
}

/// The cover stage: a back-line player who covers well (positioning 18) steps
/// with his line and one who covers poorly (5) is left behind it; the cover defender's lead
/// on a running carrier rises with positioning; a rating-10 pair reads alike.
#[test]
fn positioning_steps_the_line_and_leads_the_cover() {
    let run = DVec2::new(-4.0, 1.0);
    let (gap_low, lead_low) = cover_scene(5.0, run);
    let (gap_high, lead_high) = cover_scene(18.0, run);
    println!("line gap: 5: {gap_low:.3} m  18: {gap_high:.3} m");
    println!("cover lead: 5: {lead_low:.3} m  18: {lead_high:.3} m");
    assert!(
        gap_high < 1e-9,
        "a good coverer steps with the line, {gap_high}"
    );
    // The gap falls by all of itself (a move of 100 percent, at least 10).
    assert!(gap_low > 0.05, "a poor coverer is left behind, {gap_low}");
    assert!(rose(lead_low, lead_high) >= 0.10, "{lead_low} {lead_high}");
    let (a, b) = (cover_scene(10.0, run), cover_scene(10.0, run));
    assert_eq!(a, b, "control: a rating-10 pair reads alike");
}

// --- Recovery runs.

/// The home side defends with every outfield player ahead of the ball but three: two
/// candidates 8 m ahead of the ball at the same depth, rated `a` and `b` in work rate, and the
/// rest farther ahead. Away striker 9 carries 20 m into the home half; with the shipped mentality the
/// side misses one or more players goal-side. Returns which of the two candidates is marked
/// `Recover` (both may be), and each one's recovery-run share.
fn recovery(a: f64, b: f64) -> ([bool; 2], [f64; 2]) {
    let carrier = index(1, 9);
    let probe = far().build();
    let teams = probe.teams();
    let ax = teams[0].attack_x;
    let at = DVec2::new(-20.0 * ax, 0.0);
    let keeper = probe.view().keeper(0);
    let outfield: Vec<usize> = (0..11)
        .map(|s| index(0, s))
        .filter(|&i| i != keeper)
        .collect();
    let minimum = teams[0].plan.rest_defence;
    // Goal-side: minimum − 1 players behind the ball, far from it across the pitch.
    let mut s = far()
        .place(carrier, at)
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(carrier))
        .phase(Some(1))
        .tick(1_000);
    let behind = minimum.saturating_sub(1);
    let mut k = 0;
    for (n, &i) in outfield.iter().enumerate() {
        let y = -30.0 + 6.0 * n as f64;
        let pos = if n < behind {
            DVec2::new(at.x - 15.0 * ax, y)
        } else if k < 2 {
            k += 1;
            // The two candidates, the same depth ahead, far apart across.
            DVec2::new(at.x + 8.0 * ax, if k == 1 { -28.0 } else { 28.0 })
        } else {
            DVec2::new(at.x + 25.0 * ax, y)
        };
        s = s.place(i, pos);
    }
    let pair = [outfield[behind], outfield[behind + 1]];
    let sim = s
        .rating(pair[0], "work_rate", a)
        .rating(pair[1], "work_rate", b)
        .build();
    let t = sim.config().modules.decision.targets(&sim.view());
    let marked = pair.map(|i| t.jobs[i] == Job::Recover);
    let share = pair.map(|i| sim.skills(i).share(Stage::RECOVERY_RUN_EXECUTE));
    (marked, share)
}

/// The recovery-run stage: of two candidates at the same distance ahead of the
/// ball, the one with the higher work rate takes the run first, whichever index he has, and
/// his run is likelier a sprint; with equal work rate the index order decides.
#[test]
fn work_rate_takes_the_recovery_run_first_and_sprints_it_more() {
    let content = common::content();
    let t = &content.tuning.engine.pace;
    let (marked, share) = recovery(5.0, 18.0);
    println!("recovery 5 vs 18: {marked:?} shares {share:?}");
    assert_eq!(marked, [false, true], "the willing runner goes first");
    let (marked, _) = recovery(18.0, 5.0);
    assert_eq!(marked, [true, false]);
    let (low, high) = (
        engine::pace::sprint_chance(share[0], t),
        engine::pace::sprint_chance(share[1], t),
    );
    println!("sprint chance: 5: {low:.3}  18: {high:.3}");
    assert!(rose(low, high) >= 0.10, "{low} {high}");
    let (marked, _) = recovery(10.0, 10.0);
    assert_eq!(
        marked,
        [true, false],
        "control: equal work rate keeps the index order"
    );
}
