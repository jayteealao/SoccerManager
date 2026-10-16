//! The AI manager's in-match decisions follow the situation, not the clock.
//!
//! - A1: no AI decision reads a fixed minute; the tactics file holds no fixed-minute field and
//!   refuses each old one.
//! - A2: every style's rules load in the person's rule form, and play the same under the AI
//!   manager and a person, with substitutions off.
//! - A3: the AI manager makes no tactical change of its own.
//! - A4: a booked heavy tackler is replaced earlier on average (slow; paired seeds).
//! - A5: a planted need above the bar is substituted at the stoppage, with the code of its
//!   largest term; at or below the bar, not.
//! - A6: no need-driven change with the limit used, with no window left, or between stoppages.
//! - A7: the same planted need brings the same decision at every minute from 1 to 89.

mod common;

use common::{content, default_teams, run_many};
use engine::ai::{AiCode, AiManagerV1, need_of};
use engine::data::rules::StoppageKind;
use engine::modules::ManagerModule;
use engine::record::NullSink;
use engine::scenario::Scene;
use engine::{Change, EngineEventKind, EventDetail, Manager, MatchConfig, Simulation};

/// The home slot the planted scenes load: a midfielder in every shipped formation.
const SLOT: usize = 6;

/// The engine sources the AI's decisions live in.
const AI_SOURCE: &str = include_str!("../src/ai.rs");
const TICK_SOURCE: &str = include_str!("../src/sim/manager.rs");

#[test]
fn a1_no_ai_decision_reads_a_fixed_minute() {
    for (name, source) in [("ai.rs", AI_SOURCE), ("sim/manager.rs ai_tick", ai_tick())] {
        for banned in [
            ".minute(",
            "_minute",
            "check_interval",
            "is_multiple_of",
            "fatigue_energy",
        ] {
            assert!(!source.contains(banned), "{name} holds {banned}");
        }
    }
    let path = common::content_dir().path(engine::data::TACTICS_FILE);
    let shipped: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for field in engine::data::convert::TACTICS_V8_DROPPED_AI {
        assert!(
            shipped["ai"].get(field).is_none(),
            "the shipped file holds {field}"
        );
        let mut file = shipped.clone();
        file["ai"][field] = 70.into();
        let err =
            engine::data::load_tactics_bytes(&serde_json::to_vec(&file).unwrap(), "tactics.json")
                .unwrap_err()
                .to_string();
        assert!(err.contains(field), "{field}: {err}");
    }
}

/// The text of `ai_tick` in the manager pass, up to the next function.
fn ai_tick() -> &'static str {
    let start = TICK_SOURCE.find("fn ai_tick").expect("ai_tick");
    let end = TICK_SOURCE[start..]
        .find("fn ai_check")
        .map_or(TICK_SOURCE.len(), |e| start + e);
    &TICK_SOURCE[start..end]
}

#[test]
fn a2_a_styles_rules_play_the_same_under_the_ai_and_a_person() {
    let content = content();
    let [a, b] = default_teams(&content);
    let styles = content.styles.as_ref().expect("the shipped styles");
    let mut fired = 0;
    for (k, style) in styles.styles.iter().enumerate() {
        // Every style carries its chase and hold rules, which load in the person's rule form.
        assert_eq!(style.game_state.rules.len(), 2, "{}", style.name);
        let tactics = style.tactics(style.formations[0], &content.tactics);
        let play = |manager: Manager| {
            let mut config = MatchConfig::new(k as u64 + 1, 90, &content, [&a, &b]).unwrap();
            // Substitutions off on both sides: the AI manager's own check makes no change.
            config.modules.manager = &engine::ai::ManagerOff;
            let config = config
                .with_ai_tactics(0, tactics)
                .with_manager(0, manager)
                .with_game_state(0, style.game_state.clone());
            let mut sim = Simulation::new(config).unwrap();
            let mut records = Vec::new();
            let mut sink = engine::VecSink::default();
            sim.run(&mut sink).unwrap();
            records.extend(sink.records);
            let rules = sim
                .applied_changes()
                .iter()
                .filter(|c| c.team == 0 && matches!(c.change, Change::Rule { .. }))
                .count();
            (
                records,
                sim.applied_changes().to_vec(),
                sim.take_events(),
                rules,
            )
        };
        let (ai, person) = (play(Manager::Ai), play(Manager::Human));
        assert!(ai.0 == person.0, "{}: the records differ", style.name);
        assert_eq!(ai.1, person.1, "{}", style.name);
        assert_eq!(ai.2, person.2, "{}", style.name);
        fired += ai.3;
    }
    assert!(fired > 0, "no style's rule fired");
}

#[test]
fn a3_the_ai_manager_makes_no_tactical_change() {
    for banned in ["Change::Tactics", "TacticsPatch", "Change::Rule"] {
        assert!(!AI_SOURCE.contains(banned), "ai.rs builds {banned}");
    }
}

/// A seed-1 match at `minute` with every home outfield player fresh and unbooked, the home
/// side's need-driven bar unaffected by bench quality, and a throw-in opening on the tick;
/// `plant` adds the planted state and `edit` changes the configuration.
fn scene(
    minute: u32,
    edit: impl FnOnce(&mut MatchConfig),
    plant: impl FnOnce(Scene) -> Scene,
) -> Simulation {
    let content = content();
    let [a, b] = default_teams(&content);
    let mut config = MatchConfig::new(1, 90, &content, [&a, &b]).unwrap();
    // The quality of the bench option is weighed apart: these scenes test the need alone.
    config.tactics.ai.substitutions.w_quality = 0.0;
    edit(&mut config);
    let mut s = Scene::new(config).at_minute(minute);
    for slot in 0..11 {
        s = s.energy(common::index(0, slot), 1.0);
    }
    plant(s).stoppage_opened(StoppageKind::ThrowIn).build()
}

/// The bar of the home side with nothing used: the threshold.
fn bar(sim: &Simulation) -> f64 {
    sim.config().tactics.ai.substitutions.threshold
}

/// The home side's check on the scene's stoppage.
fn decide(sim: &Simulation) -> Vec<(Change, AiCode)> {
    assert!(
        sim.view().substitutions_admitted(),
        "the throw-in admits a substitution"
    );
    AiManagerV1.check(&sim.view(), 0, true).changes
}

/// A planted state of a scene.
type Plant = Box<dyn Fn(Scene) -> Scene>;

/// The four planted inputs: each loads one need term of the slot's player.
fn plants() -> Vec<(&'static str, AiCode, Plant)> {
    let i = common::index(0, SLOT);
    vec![
        (
            "energy",
            AiCode::SubFatigue,
            Box::new(move |s: Scene| s.energy(i, 0.3)),
        ),
        (
            "a booked heavy tackler",
            AiCode::SubBooked,
            Box::new(move |s: Scene| {
                s.yellow(i, 1).player_tally(i, |t| {
                    t.tackles_won = 4;
                    t.fouls = 3;
                })
            }),
        ),
        (
            "a low rating",
            AiCode::SubForm,
            Box::new(move |s: Scene| {
                s.player_tally(i, |t| {
                    t.passes = 60;
                    t.passes_completed = 10;
                    t.fouls = 6;
                })
            }),
        ),
        (
            "a knock",
            AiCode::SubKnock,
            Box::new(move |s: Scene| s.knock(i)),
        ),
    ]
}

#[test]
fn a5_a_planted_need_above_the_bar_is_substituted_with_its_code_and_one_below_is_not() {
    let i = common::index(0, SLOT);
    for (name, code, plant) in plants() {
        // The planted need, read the way the check reads it.
        let probe = scene(60, |_| {}, &plant);
        let need = need_of(&probe.view(), i);
        assert_eq!(need.code(), code, "{name}: largest term");
        let off = probe.teams()[0].lineup[SLOT];
        for (above, threshold) in [(true, need.total() - 0.01), (false, need.total() + 0.01)] {
            let sim = scene(
                60,
                |c| c.tactics.ai.substitutions.threshold = threshold,
                &plant,
            );
            assert!((bar(&sim) - threshold).abs() < 1e-12);
            let changes = decide(&sim);
            if above {
                assert_eq!(changes.len(), 1, "{name}: {changes:?}");
                let (change, got) = &changes[0];
                assert_eq!(*got, code, "{name}");
                assert!(
                    matches!(change, Change::Substitution { off: o, .. } if *o == off),
                    "{name}: {change:?}"
                );
            } else {
                assert!(changes.is_empty(), "{name} below the bar: {changes:?}");
            }
        }
    }
}

#[test]
fn a6_no_change_past_the_limit_without_a_window_or_between_stoppages() {
    let i = common::index(0, SLOT);
    let plant = |s: Scene| s.knock(i).energy(i, 0.2);
    let threshold = |c: &mut MatchConfig| c.tactics.ai.substitutions.threshold = 0.1;
    // The control: the need is far above the bar.
    assert_eq!(decide(&scene(60, threshold, plant)).len(), 1);
    let (limit, windows) = scene(60, threshold, plant).view().substitution_limits();
    // The limit used.
    let used = scene(60, threshold, |s| plant(s).subs_used(0, limit, 1));
    assert!(decide(&used).is_empty());
    // Every window used, at a stoppage that uses one.
    let closed = scene(60, threshold, |s| plant(s).subs_used(0, 1, windows));
    assert!(decide(&closed).is_empty());
    // Between stoppages: three minutes of play in which no stoppage the AI checks at
    // opens on a tick it decides on.
    let content = content();
    let [a, b] = default_teams(&content);
    let mut config = MatchConfig::new(1, 90, &content, [&a, &b]).unwrap();
    config.tactics.ai.substitutions.threshold = 0.1;
    let mut sim = plant(Scene::new(config).at_minute(60)).build();
    for _ in 0..3 * engine::rules::clock::TICKS_PER_MINUTE {
        sim.step();
        let events = sim.take_events();
        let decided = events
            .iter()
            .any(|e| e.kind == EngineEventKind::AiDecision && e.team == Some(0));
        let asked = events.iter().any(|e| {
            matches!(
                e.kind,
                EngineEventKind::Goal | EngineEventKind::Injury | EngineEventKind::Card
            )
        });
        if decided {
            assert!(
                sim.view().substitutions_admitted() || asked,
                "a decision at tick {} between stoppages",
                sim.tick()
            );
        }
    }
}

#[test]
fn a7_the_same_need_brings_the_same_decision_at_every_minute() {
    for (name, _, plant) in plants() {
        let mut seen: Option<Vec<(Change, AiCode)>> = None;
        let mut below: Option<Vec<(Change, AiCode)>> = None;
        let probe = scene(1, |_| {}, &plant);
        let total = need_of(&probe.view(), common::index(0, SLOT)).total();
        for minute in 1..=89 {
            let above = decide(&scene(
                minute,
                |c| c.tactics.ai.substitutions.threshold = total - 0.01,
                &plant,
            ));
            let under = decide(&scene(
                minute,
                |c| c.tactics.ai.substitutions.threshold = total + 0.01,
                &plant,
            ));
            assert_eq!(above.len(), 1, "{name} at minute {minute}");
            assert!(under.is_empty(), "{name} at minute {minute}");
            if let Some(first) = &seen {
                assert_eq!(&above, first, "{name}: minute {minute} differs");
            }
            seen.get_or_insert(above);
            below.get_or_insert(under);
        }
    }
}

/// The minute the home slot's starter left the pitch in a match played from `minute` with
/// `plant`, or 95 when he played on.
fn off_minute(seed: u64, plant: &dyn Fn(Scene) -> Scene) -> f64 {
    let content = content();
    let [a, b] = default_teams(&content);
    let config = MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
    let mut sim = plant(Scene::new(config).at_minute(30)).build();
    let off = sim.teams()[0].lineup[SLOT];
    sim.run(&mut NullSink).unwrap();
    sim.take_events()
        .iter()
        .find_map(|e| match (&e.kind, &e.detail) {
            (EngineEventKind::Substitution, Some(EventDetail::Substitution { off: o, .. }))
                if e.team == Some(0) && *o == off =>
            {
                Some(f64::from(e.minute))
            }
            _ => None,
        })
        .unwrap_or(95.0)
}

#[test]
#[ignore = "slow: 600 matches from minute 30; cargo test --release -p engine --test ai_in_match -- --ignored"]
fn a4_a_booked_heavy_tackler_is_replaced_earlier_on_average() {
    let i = common::index(0, SLOT);
    let none = |s: Scene| s;
    let booked = move |s: Scene| {
        s.yellow(i, 1).player_tally(i, |t| {
            t.tackles_won = 3;
            t.fouls = 2;
        })
    };
    // The control first: two unbooked arms are the same match.
    let control: Vec<f64> = run_many(1..=200, |seed| off_minute(seed, &none));
    let again: Vec<f64> = run_many(1..=200, |seed| off_minute(seed, &none));
    assert_eq!(control, again, "the unbooked arms differ");
    let arm: Vec<f64> = run_many(1..=200, |seed| off_minute(seed, &booked));
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    eprintln!(
        "booked arm: mean minute {:.2}; control: {:.2}",
        mean(&arm),
        mean(&control)
    );
    assert!(mean(&arm) < mean(&control));
}
