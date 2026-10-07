//! The plugin interface at its call sites, with hooks written in Rust: the decision hook's
//! offsets change the carrier's choice, a failed hook keeps the engine's choice and records a
//! `script` event, the rule hook replaces the referee's card, and a hook whose slot is
//! switched off is never called.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use common::{quiet_match, short_match, spread};
use engine::math::{DVec2, DVec3};
use engine::modules::registry::{HOOK_COMMENTARY, HOOK_DECISION, HOOK_RULE};
use engine::modules::{REGISTRY, ResolvedModules, SlotEntry, SlotFile, resolve};
use engine::plugin::{
    CommentaryHook, DecisionContext, DecisionHook, FoulContext, HookOutcome, HookPoint,
    LineContext, OptionOffsets, Plugins, RuleHook, ScriptOutcome,
};
use engine::rules::fouls::foul_chance;
use engine::scenario::Scene;
use engine::{Card, EngineEventKind, EventDetail, MatchConfig, NullSink, Simulation};

/// The home carrier: a midfielder with the ball in midfield, too far out to shoot, with an
/// open team-mate ahead.
const CARRIER: usize = 9;
const MATE: usize = 10;

struct Fixed(HookOutcome<OptionOffsets>);
impl DecisionHook for Fixed {
    fn adjust(&mut self, _: &DecisionContext) -> HookOutcome<OptionOffsets> {
        self.0.clone()
    }
}

fn with_decision(outcome: HookOutcome<OptionOffsets>) -> Plugins {
    let mut plugins = Plugins::new("test@1.0.0+000000000000");
    plugins.decision = Some(Box::new(Fixed(outcome)));
    plugins
}

/// The carrier stands 18 m from goal with a team-mate open 10 m to its side; the away
/// outfield players are far away. Decision noise is off, so the scene is exact.
fn scene(plugins: Option<Plugins>) -> Simulation {
    scene_in(short_match(90), plugins)
}

fn scene_in(mut config: MatchConfig, plugins: Option<Plugins>) -> Simulation {
    config.tuning.decision.noise = 0.0;
    let mut scene = spread(Scene::new(config), -30.0, -40.0)
        .place(CARRIER, DVec2::new(34.0, 0.0))
        .place(MATE, DVec2::new(34.0, 10.0))
        .ball(DVec3::new(34.0, 0.0, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001);
    if let Some(plugins) = plugins {
        scene = scene.plugins(plugins);
    }
    scene.build()
}

fn shoot_only() -> OptionOffsets {
    OptionOffsets {
        pass: -10.0,
        dribble: -10.0,
        shoot: 10.0,
        clear: -10.0,
        hold: -10.0,
    }
}

fn pass_only() -> OptionOffsets {
    OptionOffsets {
        pass: 10.0,
        dribble: -10.0,
        shoot: -10.0,
        clear: -10.0,
        hold: -10.0,
    }
}

#[test]
fn the_decision_hook_turns_the_carriers_choice_into_the_option_it_favours() {
    let mut shot = scene(Some(with_decision(HookOutcome::Value(shoot_only()))));
    shot.step();
    assert_eq!(shot.summary().shots[0], 1, "the favoured shot was taken");
    assert_eq!(shot.summary().passes[0], 0);

    let mut pass = scene(Some(with_decision(HookOutcome::Value(pass_only()))));
    pass.step();
    assert_eq!(pass.summary().passes[0], 1, "the favoured pass was played");
    assert_eq!(pass.summary().shots[0], 0);
    assert_eq!(pass.plugins().stats.calls, 1);
}

#[test]
fn a_hook_that_keeps_or_fails_leaves_the_native_choice_and_a_failure_is_recorded() {
    let mut native = scene(None);
    native.step();
    let native_record = native.record();

    let mut kept = scene(Some(with_decision(HookOutcome::Keep)));
    kept.step();
    assert_eq!(kept.record(), native_record);
    assert!(
        kept.take_events()
            .iter()
            .all(|e| e.kind != EngineEventKind::Script)
    );

    let mut failed = scene(Some(with_decision(HookOutcome::Aborted(
        "operation budget".into(),
    ))));
    failed.step();
    assert_eq!(failed.record(), native_record, "the native choice stands");
    let events = failed.take_events();
    let script: Vec<_> = events
        .iter()
        .filter(|e| e.kind == EngineEventKind::Script)
        .collect();
    assert_eq!(script.len(), 1);
    let Some(EventDetail::Script(note)) = script[0].detail else {
        panic!("a script event carries its note");
    };
    assert_eq!(note.hook, HookPoint::Decision);
    assert_eq!(note.outcome, ScriptOutcome::Aborted);
    assert_eq!(failed.plugins().detail(&note), "operation budget");
    assert_eq!(failed.plugins().stats.aborts, 1);
}

#[test]
fn the_offsets_are_cached_for_the_carrier_until_the_refresh_interval_passes() {
    let mut plugins = with_decision(HookOutcome::Value(OptionOffsets {
        pass: -10.0,
        dribble: -10.0,
        shoot: -10.0,
        clear: -10.0,
        hold: 10.0,
    }));
    plugins.refresh_ticks = 10;
    let mut sim = scene(Some(plugins));
    for _ in 0..25 {
        sim.step();
    }
    assert_eq!(sim.carrier(), Some(CARRIER), "the carrier held the ball");
    assert_eq!(
        sim.plugins().stats.calls,
        3,
        "asked at ticks 1001, 1011, 1021"
    );
}

struct Referee(HookOutcome<Option<Card>>);
impl RuleHook for Referee {
    fn card(&mut self, _: &FoulContext, _: Option<Card>) -> HookOutcome<Option<Card>> {
        self.0.clone()
    }
}

/// A foul with the ball lost and no card from the referee's draw, then the step's events.
fn foul(outcome: HookOutcome<Option<Card>>) -> Vec<engine::EngineEvent> {
    let mut plugins = Plugins::new("test@1.0.0+000000000000");
    plugins.rule = Some(Box::new(Referee(outcome)));
    let mut sim = foul_scene(quiet_match(90), plugins);
    sim.step();
    sim.take_events()
}

/// The scene of [`foul`] on `config`, with `plugins` attached.
fn foul_scene(config: MatchConfig, plugins: Plugins) -> Simulation {
    const CARRIER: usize = 5;
    const TACKLER: usize = 16;
    let sim = Simulation::new(config.clone()).unwrap();
    let tackler = sim.skills(TACKLER);
    let carrier = sim.skills(CARRIER);
    let p_win = engine::rules::fouls::win_chance(tackler, carrier, &config.tuning);
    let p_foul = foul_chance(tackler, 0, &config.tuning);
    let at = DVec2::new(0.0, 10.0);
    spread(Scene::new(config), -30.0, 30.0)
        .place(CARRIER, at)
        .place(TACKLER, at + DVec2::new(0.3, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001)
        .rolls(&[p_win + 0.1 * p_foul, 0.99])
        .plugins(plugins)
        .build()
}

#[test]
fn the_rule_hook_replaces_the_referees_card_and_keep_leaves_it() {
    let kinds = |events: &[engine::EngineEvent]| events.iter().map(|e| e.kind).collect::<Vec<_>>();
    let native = foul(HookOutcome::Keep);
    assert_eq!(
        kinds(&native),
        [EngineEventKind::Foul, EngineEventKind::FreeKick]
    );
    let red = foul(HookOutcome::Value(Some(Card::Red)));
    assert_eq!(
        kinds(&red),
        [
            EngineEventKind::Foul,
            EngineEventKind::Card,
            EngineEventKind::FreeKick
        ]
    );
    assert_eq!(red[1].card, Some(Card::Red));
    let denied = foul(HookOutcome::Denied("function `http_get`".into()));
    assert_eq!(
        kinds(&denied),
        [
            EngineEventKind::Foul,
            EngineEventKind::Script,
            EngineEventKind::FreeKick
        ]
    );
}

/// A hook of every point that counts its calls and keeps the engine's choice.
#[derive(Clone, Default)]
struct Counting(Arc<AtomicU32>);

impl Counting {
    fn calls(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }

    fn count(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

impl DecisionHook for Counting {
    fn adjust(&mut self, _: &DecisionContext) -> HookOutcome<OptionOffsets> {
        self.count();
        HookOutcome::Keep
    }
}

impl RuleHook for Counting {
    fn card(&mut self, _: &FoulContext, _: Option<Card>) -> HookOutcome<Option<Card>> {
        self.count();
        HookOutcome::Keep
    }
}

impl CommentaryHook for Counting {
    fn line(&mut self, _: &LineContext, _: &str) -> HookOutcome<String> {
        self.count();
        HookOutcome::Keep
    }
}

/// The default modules, with `slot` switched off when `off`.
fn modules(slot: &str, off: bool) -> ResolvedModules {
    let mut file = SlotFile::builtin_default();
    if off {
        file.slots.insert(
            slot.to_string(),
            SlotEntry {
                module: "off".to_string(),
                version: None,
            },
        );
    }
    let modules = resolve(&file, REGISTRY).unwrap();
    let picked = modules.picked_for(slot).unwrap().module;
    assert_eq!(picked == "off", off, "{slot}: {picked}");
    modules
}

/// With a hook slot switched off, the loop never calls that point's hook, and the plugin
/// counts no call; with the slot on, the same scene calls the hook.
#[test]
fn a_switched_off_hook_slot_never_calls_its_hook() {
    for off in [false, true] {
        // The decision hook: the carrier's scene, one step.
        let hook = Counting::default();
        let mut plugins = Plugins::new("test@1.0.0+000000000000");
        plugins.decision = Some(Box::new(hook.clone()));
        let mut config = short_match(90);
        config.modules = modules(HOOK_DECISION.id, off);
        let mut sim = scene_in(config, Some(plugins));
        sim.step();
        let calls = sim.plugins().stats.calls;
        if off {
            assert_eq!((hook.calls(), calls), (0, 0), "decision slot off");
        } else {
            assert!(hook.calls() >= 1 && calls >= 1, "decision slot on");
        }

        // The rule hook: the foul scene, one step.
        let hook = Counting::default();
        let mut plugins = Plugins::new("test@1.0.0+000000000000");
        plugins.rule = Some(Box::new(hook.clone()));
        let mut config = quiet_match(90);
        config.modules = modules(HOOK_RULE.id, off);
        let mut sim = foul_scene(config, plugins);
        sim.step();
        assert!(
            sim.take_events()
                .iter()
                .any(|e| e.kind == EngineEventKind::Foul),
            "the scene has a foul"
        );
        let calls = sim.plugins().stats.calls;
        if off {
            assert_eq!((hook.calls(), calls), (0, 0), "rule slot off");
        } else {
            assert!(hook.calls() >= 1 && calls >= 1, "rule slot on");
        }

        // The commentary hook: a line offered for each event of a short match.
        let hook = Counting::default();
        let mut plugins = Plugins::new("test@1.0.0+000000000000");
        plugins.commentary = Some(Box::new(hook.clone()));
        let mut config = short_match(2);
        config.modules = modules(HOOK_COMMENTARY.id, off);
        let mut sim = Simulation::new(config).unwrap();
        sim.set_plugins(plugins);
        sim.run(&mut NullSink).unwrap();
        let events = sim.take_events();
        assert!(!events.is_empty(), "the match has events");
        for event in &events {
            let (line, script) = sim.offer_line(event, Some("native".to_string()));
            assert_eq!(line.as_deref(), Some("native"));
            assert!(script.is_empty());
        }
        let calls = sim.plugins().stats.calls;
        if off {
            assert_eq!((hook.calls(), calls), (0, 0), "commentary slot off");
        } else {
            assert_eq!(hook.calls(), events.len() as u32, "commentary slot on");
            assert_eq!(calls, events.len() as u32);
        }
    }
}
