//! The attribute contract: every action has its stage tables and every attribute a job, in
//! the shipped content and in the frozen copy the converters read; play reads ratings only
//! through the contract; every contest stays inside its action's limits; and the skill gate
//! decides who tries a chip and who pulls off a take-on.

mod common;

use std::path::{Path, PathBuf};

use common::{calm_match, index, spread};
use engine::contract::params::uses;
use engine::contract::{ActionKind, CHIP_M, Skills, curve};
use engine::data::attributes::JobStage;
use engine::math::DVec2;
use engine::player::Derived;
use engine::scenario::Scene;
use engine::tuning::Tuning;
use engine::{MatchConfig, Rating};

// --- Tables and jobs.

#[test]
fn every_action_has_a_table_for_each_of_its_stages_in_the_shipped_content() {
    let content = common::content();
    let actions = &content.attributes.actions;
    for action in ActionKind::ALL {
        let tables = actions
            .get(&action)
            .unwrap_or_else(|| panic!("{action}: no tables"));
        for stage in action.stages() {
            assert!(tables.contains_key(&stage), "{action}: no {stage:?} table");
        }
        assert_eq!(tables.len(), action.stages().count(), "{action}");
    }
}

#[test]
fn the_frozen_copies_have_a_table_for_each_stage_of_every_action() {
    let frozen: serde_json::Value =
        serde_json::from_str(engine::data::convert::FROZEN_V1).expect("the frozen copy parses");
    let frozen_v4: serde_json::Value =
        serde_json::from_str(engine::data::convert::FROZEN_V4).expect("the fourth copy parses");
    let actions = frozen["attributes"]["actions"]
        .as_object()
        .expect("the frozen copy has action tables");
    for action in ActionKind::ALL {
        for stage in action.stages() {
            let in_v1 = actions
                .get(action.name())
                .is_some_and(|t| t.get(stage.name()).is_some());
            let in_v4 = frozen_v4["tables"]
                .get(action.name())
                .is_some_and(|t| t.get(stage.name()).is_some());
            assert!(in_v1 || in_v4, "{action}: no frozen {stage:?} table");
        }
    }
    let jobs = frozen["attributes"]["jobs"]
        .as_object()
        .expect("the frozen copy has jobs");
    let content = common::content();
    let frozen_v3: serde_json::Value =
        serde_json::from_str(engine::data::convert::FROZEN_V3).expect("the third copy parses");
    for def in &content.attributes.attributes {
        // Version 3 renames injury resistance and adds consistency from the third copy.
        let first = match def.name.as_str() {
            "injury_proneness" => "injury_resistance",
            name => name,
        };
        assert!(
            jobs.contains_key(first)
                || frozen_v3["attributes"].get(&def.name).is_some()
                || frozen_v4["attributes"].get(&def.name).is_some(),
            "{}: no frozen job",
            def.name
        );
    }
}

#[test]
fn every_attribute_feeds_a_stage_and_has_a_job_with_a_statistic() {
    let content = common::content();
    let schema = &content.attributes;
    for def in &schema.attributes {
        assert!(!def.job.statistic.trim().is_empty(), "{}", def.name);
        match def.job.stage.stage() {
            None if def.job.stage == JobStage::Spread => {
                assert!(def.hidden, "{}: only a hidden value spreads", def.name);
                assert_eq!(def.name, "consistency");
            }
            None if matches!(def.job.stage, JobStage::Occasion | JobStage::Inert) => {
                assert!(def.is_personality(), "{}: only personality", def.name);
                assert!(def.job.action.is_none(), "{}", def.name);
            }
            None => {
                assert_eq!(def.job.stage, JobStage::TopSpeed);
                assert_eq!(def.name, "pace", "only pace feeds the speed map");
            }
            Some(stage) => {
                let action = def.job.action.expect("a stage job names its action");
                assert!(
                    schema.in_stage(&def.name, action, stage),
                    "{}: its job names {action} {stage:?}, which it does not feed",
                    def.name,
                )
            }
        }
    }
}

// --- Personality and the small weights.

/// Shape AC-4 of the record: no state family moves a personality value, and nothing panics,
/// with deltas on every state group and a form offset on the blend.
#[test]
fn no_state_moves_a_personality_value_and_nothing_panics() {
    use engine::contract::stages::Blend;
    use engine::contract::states::effective;
    let content = common::content();
    let schema = &content.attributes;
    let mut base = engine::player::Attributes {
        values: [Rating::from_tenths(100); engine::data::MAX_ATTRIBUTES],
        len: schema.len() as u8,
    };
    let personality: Vec<usize> = schema
        .attributes
        .iter()
        .enumerate()
        .filter(|(_, d)| d.is_personality())
        .map(|(i, _)| i)
        .collect();
    assert_eq!(personality.len(), 11);
    for &i in &personality {
        base.values[i] = Rating::from_tenths(30);
    }
    let moved = effective(&base, schema, [-20, -10, -5, 5]);
    for &i in &personality {
        assert_eq!(moved.get(i).tenths(), 30, "{}", schema.attributes[i].name);
    }
    let passing = schema.index("passing").unwrap();
    assert_ne!(
        moved.get(passing).tenths(),
        100,
        "a visible value still moves"
    );
    let blend = Blend::of(schema);
    let tuning = Tuning::default();
    for form in [-7, 6] {
        let stages = blend.values_with_form(&moved, &tuning.contract, form);
        assert!(stages.f.iter().all(|v| v.is_finite()));
    }
}

/// Shape AC-18: with a consistency degree per action, a form offset moves each action by its
/// own degree. Degrees planted at 0.5 on the pass and 1.5 on the shot, a player at 10.0
/// throughout and an offset of +0.5: `ln(F / 8) * width / 0.5` of every stage equals its
/// action's degree, and a degree of 0 leaves a stage at 8.
#[test]
fn each_action_moves_by_its_own_consistency_degree() {
    use engine::contract::stages::Blend;
    use engine::contract::{STAGES, Stage, StageKind};
    let content = common::content();
    let mut schema = content.attributes.clone();
    schema.consistency_degrees.insert(ActionKind::Pass, 0.5);
    schema.consistency_degrees.insert(ActionKind::Shot, 1.5);
    schema.consistency_degrees.insert(ActionKind::Cross, 0.0);
    let tuning = Tuning::default();
    let width = tuning.contract.curve.width;
    let flat = engine::player::Attributes {
        values: [Rating::from_tenths(100); engine::data::MAX_ATTRIBUTES],
        len: schema.len() as u8,
    };
    let stages = Blend::of(&schema).values_with_form(&flat, &tuning.contract, 5);
    for (i, &(action, stage)) in STAGES.iter().enumerate() {
        let degree = schema.consistency_degrees[&action];
        let read = engine::math::ln(stages.f[i] / 8.0) * width / 0.5;
        assert!(
            (read - degree).abs() < 1e-9,
            "{action}.{}: moved by degree {read}, its degree is {degree}",
            stage.name()
        );
    }
    let cross = Stage::find(ActionKind::Cross, StageKind::Execute).unwrap();
    assert_eq!(stages.f(cross), 8.0);
    let pass = Stage::find(ActionKind::Pass, StageKind::Execute).unwrap();
    let shot = Stage::find(ActionKind::Shot, StageKind::Execute).unwrap();
    assert!(stages.f(shot) > stages.f(pass) && stages.f(pass) > 8.0);
}

/// Shape AC-5 of the record: in every stage with small weights, each small weight is lighter
/// than the lightest shown support, the main is the heaviest, and the blend reads them; a
/// small weight as heavy as a shown support is refused.
#[test]
fn small_weights_are_lighter_than_every_shown_support_and_blend_into_the_stage() {
    use engine::contract::{Stage, StageKind, stage_values};
    let content = common::content();
    let schema = &content.attributes;
    let mut with_minor = 0;
    for stages in schema.actions.values() {
        for t in stages.values() {
            if t.minor.is_empty() {
                continue;
            }
            with_minor += 1;
            let lightest = t
                .supports
                .iter()
                .map(|s| s.weight)
                .fold(t.main.weight, f64::min);
            for m in &t.minor {
                assert!(m.weight < lightest, "{} {}", m.attribute, m.weight);
            }
            for w in t.supports.iter().chain(&t.minor) {
                assert!(w.weight < t.main.weight);
            }
        }
    }
    assert_eq!(with_minor, 5, "penalty, dribble, receive, tackle, block");
    // The blend reads a small weight: flair moves the receive stage, a little.
    let tuning = Tuning::default();
    let mut ratings = engine::player::Attributes {
        values: [Rating::from_tenths(100); engine::data::MAX_ATTRIBUTES],
        len: schema.len() as u8,
    };
    let receive = Stage::find(ActionKind::Receive, StageKind::Execute).unwrap();
    let before = stage_values(&ratings, schema, &tuning.contract).f(receive);
    ratings.values[schema.index("flair").unwrap()] = Rating::from_tenths(200);
    let flair = stage_values(&ratings, schema, &tuning.contract).f(receive) - before;
    ratings.values[schema.index("flair").unwrap()] = Rating::from_tenths(100);
    ratings.values[schema.index("balance").unwrap()] = Rating::from_tenths(200);
    let support = stage_values(&ratings, schema, &tuning.contract).f(receive) - before;
    assert!(
        flair > 0.0 && flair < support,
        "flair {flair}, support {support}"
    );
    // A planted small weight as heavy as a shown support is refused.
    let mut planted = schema.clone();
    let t = planted
        .actions
        .get_mut(&ActionKind::Receive)
        .unwrap()
        .get_mut(&StageKind::Execute)
        .unwrap();
    t.minor[0].weight = t.supports.iter().map(|s| s.weight).fold(f64::MAX, f64::min);
    use garde::Validate;
    let err = planted.validate().unwrap_err().to_string();
    assert!(err.contains("receive.execute: small weight flair"), "{err}");
}

// --- One door to the ratings.

/// Every Rust file under `dir`, recursively.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Play reads ratings only through the contract: outside the contract, the data layer, the
/// rating type, and the named allow-list (lineup role fit, the fitted fast model, the plugin
/// hook's share, and the sensitivity rules, which set levels on player copies before a match),
/// no source reads a rating. A file's own unit tests are not play.
#[test]
fn only_the_contract_and_the_allow_list_read_ratings() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let allowed = |rel: &str| {
        rel.starts_with("contract/")
            || rel.starts_with("data/")
            || rel == "rating.rs"
            || rel == "ai.rs"
            || rel == "modules/fast_model.rs"
            || rel == "hook_slots.rs"
            || rel.starts_with("sensitivity/")
    };
    let reads = [
        "attributes.get(",
        ".attributes.iter()",
        ".tenths()",
        ".decimal()",
    ];
    let mut found = Vec::new();
    for path in &files {
        let rel = path
            .strip_prefix(&src)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(path).unwrap();
        assert!(!text.contains("old_scale"), "{rel} names old_scale");
        if allowed(&rel) {
            continue;
        }
        let play = text.split("#[cfg(test)]\nmod tests").next().unwrap();
        for (n, line) in play.lines().enumerate() {
            if reads.iter().any(|r| line.contains(r)) {
                found.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "ratings read outside the contract:\n{}",
        found.join("\n")
    );
}

// --- The curve and the contests.

#[test]
fn the_curve_passes_through_its_hand_values() {
    let c = Tuning::default().contract.curve;
    for (r, expected) in [(4.0, 3.778_9), (10.0, 8.0), (18.0, 21.746_3)] {
        assert!(
            (curve::f(r, &c) - expected).abs() < 1e-4,
            "F({r}) = {}",
            curve::f(r, &c)
        );
    }
    assert_eq!(curve::f(10.0, &c), 8.0);
}

#[test]
fn every_contest_at_the_extremes_stays_inside_its_limits() {
    let t = Tuning::default();
    let c = &t.contract;
    let (top, bottom) = (curve::f(20.0, &c.curve), curve::f(1.0, &c.curve));
    for action in ActionKind::ALL {
        let [k, base, limits, _] = uses(action);
        if !k || !limits {
            continue;
        }
        let p = c.actions.of(action);
        let base = if base { p.base() } else { 0.5 };
        for (a, b) in [(top, bottom), (bottom, top), (top, top), (bottom, bottom)] {
            let chance = curve::contest(base, p.k(), a, b, p.floor(), p.ceiling());
            assert!(
                (p.floor()..=p.ceiling()).contains(&chance),
                "{action}: {chance} outside {} to {}",
                p.floor(),
                p.ceiling()
            );
        }
        assert_eq!(
            curve::contest(base, p.k(), 8.0, 8.0, p.floor(), p.ceiling()),
            base.clamp(p.floor(), p.ceiling()),
            "{action}: two equal players leave the base"
        );
    }
}

// --- The skill gate.

/// A player of `config` whose every rating is 10.0 except `name`, at `tenths`.
fn set_rating(config: &mut MatchConfig, i: usize, name: &str, tenths: u8) {
    let schema = &config.attributes;
    let p = &mut config.players[i];
    for k in 0..usize::from(p.attributes.len) {
        p.attributes.values[k] = Rating::from_tenths(100);
    }
    let at = schema.index(name).unwrap();
    p.attributes.values[at] = Rating::from_tenths(tenths);
    let (derived, stages) = Derived::from_attributes(&p.attributes, schema, &config.tuning);
    p.derived = derived;
    let entry = &mut config.teams[p.team].squad[p.squad];
    entry.attributes = p.attributes;
    entry.derived = derived;
    entry.stages = stages;
}

/// Player `i`'s stage values in `config`, as play reads them.
fn skills(config: &MatchConfig, i: usize) -> Skills<'_> {
    let p = &config.players[i];
    Skills::new(&config.teams[p.team].squad[p.squad].stages, &p.derived)
}

/// A small seeded generator, so the situations are the same on every run.
struct Draws(u64);

impl Draws {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) as f64 / u64::MAX as f64
    }
}

/// Over 2,000 seeded carrier situations with an open team-mate more than 25 m away, a
/// carrier with technique 7.9 never drafts a chip, and one with 8.0 drafts the open one
/// every time.
#[test]
fn technique_under_the_threshold_never_tries_a_chip() {
    let carrier = index(0, 6);
    let mate = index(0, 9);
    for (tenths, tries) in [(79u8, false), (80, true)] {
        let mut config = calm_match(90);
        set_rating(&mut config, carrier, "technique", tenths);
        let mut draws = Draws(7);
        for n in 0..2_000 {
            let d = CHIP_M + 1.0 + 18.0 * draws.next();
            // Toward his own goal side, far from the opponents' line.
            let angle = std::f64::consts::PI * (2.0 / 3.0 + 2.0 / 3.0 * draws.next());
            let (sin, cos) = engine::math::sin_cos(angle);
            let at = DVec2::new(d * cos, d * sin);
            let sim = spread(Scene::new(config.clone()), -60.0, 50.0)
                .place(carrier, DVec2::ZERO)
                .place(mate, at)
                .carrier(Some(carrier))
                .build();
            let view = sim.view();
            let draft = sim.config().modules.decision.options(&view, carrier);
            let chips: Vec<usize> = draft.passes[..draft.pass_count]
                .iter()
                .map(|&(j, _)| j)
                .filter(|&j| (view.player(j).pos - view.player(carrier).pos).length() > CHIP_M)
                .collect();
            if tries {
                assert!(
                    chips.contains(&mate),
                    "situation {n}: the open chip at {d:.1} m"
                );
            } else {
                assert!(
                    chips.is_empty(),
                    "situation {n}: technique 7.9 drafted {chips:?}"
                );
            }
        }
    }
}

/// On the same 2,000 seeded draws, a running carrier with agility 6.0 loses the ball to an
/// average tackler more often than one with agility 9.0, by more than three standard errors.
#[test]
fn agility_under_the_threshold_loses_more_take_ons() {
    let mut config = calm_match(90);
    let carrier = index(0, 6);
    let tackler = index(1, 6);
    set_rating(&mut config, tackler, "agility", 100);
    let t = config.tuning.clone();
    let mut lost = [0u32; 2];
    for (k, tenths) in [60u8, 90].into_iter().enumerate() {
        set_rating(&mut config, carrier, "agility", tenths);
        let p_win = engine::rules::fouls::running_win_chance(
            skills(&config, tackler),
            skills(&config, carrier),
            &t,
        );
        let mut draws = Draws(11);
        for _ in 0..2_000 {
            if draws.next() < p_win {
                lost[k] += 1;
            }
        }
    }
    let n = 2_000.0;
    let (a, b) = (f64::from(lost[0]) / n, f64::from(lost[1]) / n);
    let se = (a * (1.0 - a) / n + b * (1.0 - b) / n).sqrt();
    assert!(
        a - b > 3.0 * se,
        "agility 6.0 lost {a:.3}, 9.0 lost {b:.3}, se {se:.4}"
    );
}

// --- Familiarity and the shared floor.

/// The technical and mental groups' indices in a player's deltas.
const TECHNICAL: usize = 0;
const MENTAL: usize = 1;

/// A position other than `played`, for his natural one.
fn elsewhere(played: engine::data::team::Position) -> engine::data::team::Position {
    use engine::data::team::Position;
    if played == Position::ST {
        Position::CB
    } else {
        Position::ST
    }
}

/// Home player 5, natural 16, plays a position he is rated `at` in, rated `versatility` and
/// `adaptability`: his technical and mental deltas at kick-off, in tenths.
fn familiarity_deltas(at: f64, versatility: f64, adaptability: f64) -> [i8; 2] {
    let me = index(0, 5);
    let scene = Scene::new(calm_match(90));
    let played = scene.played_position(me);
    // The position ratings last: setting one refreshes his deltas.
    let sim = scene
        .rating(me, "versatility", versatility)
        .rating(me, "adaptability", adaptability)
        .position_rating(me, elsewhere(played), 16.0)
        .position_rating(me, played, at)
        .build();
    let d = sim.players()[me].deltas;
    [d[TECHNICAL], d[MENTAL]]
}

/// A player rated 8 at the position he plays, 16 at his natural one, loses some
/// of his technical and mental values at kick-off, inside the familiarity cap; less with a
/// high versatility or adaptability than a low one; nothing at his natural position.
#[test]
fn familiarity_lowers_technical_and_mental_inside_its_cap() {
    let config = calm_match(90);
    let cap = (config.tuning.contract.states.caps.familiarity[0] * 10.0).round() as i8;
    let base = familiarity_deltas(8.0, 10.0, 10.0);
    println!("rated 8, natural 16: {base:?} tenths (cap {cap})");
    for d in base {
        assert!(d < 0 && d >= cap, "{d}");
    }
    let (low_v, high_v) = (
        familiarity_deltas(8.0, 5.0, 10.0),
        familiarity_deltas(8.0, 18.0, 10.0),
    );
    let (low_a, high_a) = (
        familiarity_deltas(8.0, 10.0, 5.0),
        familiarity_deltas(8.0, 10.0, 18.0),
    );
    for g in 0..2 {
        assert!(high_v[g] > low_v[g], "versatility {low_v:?} {high_v:?}");
        assert!(high_a[g] > low_a[g], "adaptability {low_a:?} {high_a:?}");
        assert!(low_v[g] >= cap && low_a[g] >= cap);
    }
    assert_eq!(familiarity_deltas(16.0, 10.0, 10.0), [0, 0]);
}

/// The worst case for home player 5: every attribute but the personality
/// values at 18, played at a position he is rated 1 in (natural 18), his whole side at a full
/// low mood, no energy, no sharpness and his form at the shipped bound below. Returns the
/// match, the player and a flat-11 player's stage values. `plant` edits the configuration
/// first.
fn worst_sim(
    plant: impl Fn(&mut MatchConfig),
) -> (engine::Simulation, usize, engine::contract::StageValues) {
    let me = index(0, 5);
    let mut config = calm_match(90);
    let squad = config.players[me].squad;
    config.teams[0].squad[squad].condition.sharpness = Some(0);
    plant(&mut config);
    let bound = common::content()
        .tuning
        .engine
        .contract
        .consistency
        .bound_tenths();
    let schema = config.attributes.clone();
    let tuning = config.tuning.clone();
    let mut scene = Scene::new(config);
    for def in schema.attributes.iter().filter(|d| !d.is_personality()) {
        scene = scene.rating(me, &def.name, 18.0);
    }
    let played = scene.played_position(me);
    scene = scene
        .position_rating(me, elsewhere(played), 18.0)
        .position_rating(me, played, 1.0);
    for slot in 0..11 {
        scene = scene.mood(index(0, slot), -100.0);
    }
    let sim = scene.energy(me, 0.0).form(me, -bound).build();
    let flat = engine::player::Attributes {
        values: [Rating::from_tenths(110); engine::data::MAX_ATTRIBUTES],
        len: schema.len() as u8,
    };
    let (_, eleven) = Derived::from_attributes(&flat, &schema, &tuning);
    (sim, me, eleven)
}

/// The worst case ([`worst_sim`]): the ratio of each of the player's stage values to a
/// flat-11 player's, the difference of each share, and his lowest visible effective rating.
fn worst_case(plant: impl Fn(&mut MatchConfig)) -> (Vec<f64>, Vec<f64>, f64) {
    use engine::contract::STAGE_COUNT;
    let (sim, me, eleven) = worst_sim(plant);
    let schema = sim.config().attributes.clone();
    let skills = sim.skills(me);
    let f: Vec<f64> = (0..STAGE_COUNT)
        .map(|s| skills.stages.f[s] / eleven.f[s])
        .collect();
    let share: Vec<f64> = (0..STAGE_COUNT)
        .map(|s| skills.stages.share[s] - eleven.share[s])
        .collect();
    let ratings = sim.effective_ratings(me);
    let lowest = schema
        .attributes
        .iter()
        .enumerate()
        .filter(|(_, d)| !d.hidden && !d.is_personality())
        .map(|(k, _)| f64::from(ratings.get(k).tenths()) / 10.0)
        .fold(f64::INFINITY, f64::min);
    (f, share, lowest)
}

/// The lowest stage of `ratios` on the rating scale: `11 + width · ln(ratio)`.
fn lowest_rating(ratios: &[f64], width: f64) -> f64 {
    ratios
        .iter()
        .map(|r| 11.0 + width * engine::math::ln(*r))
        .fold(f64::INFINITY, f64::min)
}

/// An 18 in the worst case, before any kick: with the shipped caps, the worst case keeps
/// every stage value, every share (so every pressed execution share) and every visible
/// effective rating at least a flat 11's. The weaker foot on top of it is the next test.
#[test]
fn an_eighteen_in_the_worst_case_plays_no_action_below_eleven() {
    let width = calm_match(90).tuning.contract.curve.width;
    let (f, share, lowest) = worst_case(|_| {});
    println!(
        "shipped caps: lowest stage {:.2}, lowest visible rating {lowest:.1}",
        lowest_rating(&f, width)
    );
    assert!(f.iter().all(|r| *r >= 1.0 - 1e-9), "{f:?}");
    assert!(share.iter().all(|d| *d >= -1e-9), "{share:?}");
    assert!(lowest >= 11.0, "{lowest}");
}

/// The worst case on the weaker foot: the player's weaker foot at 1, every kick (pass, chip,
/// cross, shot, long shot) to his weaker side, pressed by an opponent 0.5 m away, at 40 m.
/// Returns, per kick, the ratio of each of its two stages after the shrunk cost to a flat
/// 11's, and its pressed execution share less a flat 11's.
fn weaker_foot_kicks(plant: impl Fn(&mut MatchConfig)) -> Vec<(&'static str, [f64; 2], f64)> {
    use engine::contract::Stage;
    use engine::contract::foot::{KickSide, cost_points, kick_share, shrunk};
    use engine::data::team::{Foot, Side};
    let (sim, me, eleven) = worst_sim(plant);
    let c = &sim.config().tuning.contract;
    let floor = c.person.map(|p| p.floor_drop);
    let foot = Foot {
        strong: Side::Right,
        weaker: 1,
    };
    let cost = cost_points(foot, KickSide::Weaker, false, 0.5, 40.0, &c.foot());
    assert!(cost > 4.5 && cost <= 5.0, "{cost}");
    let (stages, base) = (sim.skills(me).stages, sim.base_skills(me).stages);
    let width = c.curve.width;
    [
        ("pass", Stage::PASS_EXECUTE, Stage::PASS_PRESSURE),
        ("chip", Stage::CHIP_EXECUTE, Stage::CHIP_PRESSURE),
        ("cross", Stage::CROSS_EXECUTE, Stage::CROSS_PRESSURE),
        ("shot", Stage::SHOT_EXECUTE, Stage::SHOT_PRESSURE),
        (
            "long shot",
            Stage::LONG_SHOT_EXECUTE,
            Stage::LONG_SHOT_PRESSURE,
        ),
    ]
    .into_iter()
    .map(|(name, e, p)| {
        let after = |s: Stage| {
            let f = stages.f(s);
            let d = shrunk(cost, f, base.f(s), floor, width);
            f * engine::math::exp(-d / width) / eleven.f(s)
        };
        let share = kick_share(stages, base, e, Some(p), cost, floor, c);
        (
            name,
            [after(e), after(p)],
            share - eleven.pressed_share(e, p),
        )
    })
    .collect()
}

/// AC-20, full case: an 18 out of position, in low mood, tired, out of sharpness, at his
/// worst form, pressed and on his weakest foot at range plays no kick below a flat 11: the
/// weaker foot's cost shrinks to the points the states left above the floor.
#[test]
fn an_eighteen_on_his_weaker_foot_in_the_worst_case_plays_no_action_below_eleven() {
    let width = calm_match(90).tuning.contract.curve.width;
    let kicks = weaker_foot_kicks(|_| {});
    let lowest = kicks
        .iter()
        .flat_map(|(_, r, _)| r.iter().copied())
        .collect::<Vec<_>>();
    println!(
        "weaker foot in the worst case: lowest kick stage {:.2}",
        lowest_rating(&lowest, width)
    );
    for (name, ratios, share) in &kicks {
        assert!(
            ratios.iter().all(|r| *r >= 1.0 - 1e-9),
            "{name}: {ratios:?}"
        );
        assert!(*share >= -1e-9, "{name}: {share}");
    }
}

/// AC-20's guarded failure for the weaker foot: with the floor out of reach (12.7 points) the
/// same kicks fall below a flat 11, so the shrink, not the 5-point cap, holds the line.
#[test]
fn the_shrink_holds_the_weaker_foot_inside_the_floor() {
    let width = calm_match(90).tuning.contract.curve.width;
    let open = weaker_foot_kicks(|config| {
        if let Some(person) = config.tuning.contract.person.as_mut() {
            person.floor_drop = 12.7;
        }
    });
    let lowest = open
        .iter()
        .flat_map(|(_, r, _)| r.iter().copied())
        .collect::<Vec<_>>();
    println!(
        "weaker foot, floor out of reach: lowest kick stage {:.2}",
        lowest_rating(&lowest, width)
    );
    assert!(
        open.iter().any(|(_, _, share)| *share < -1e-6),
        "without the shrink the weaker foot must take a kick below 11: {open:?}"
    );
}

/// The floor, guarded failure: a planted stack past the shipped caps (the body and total
/// caps widened to -8 points and the fatigue curve ending at 0.3) stays at or above a flat 11
/// with the floor at 7 points, and falls below it with the floor out of reach (12.7 points),
/// so the floor, not the caps, holds the line.
#[test]
fn the_floor_holds_a_planted_stack_the_caps_do_not() {
    let width = calm_match(90).tuning.contract.curve.width;
    let plant = |drop: f64| {
        move |config: &mut MatchConfig| {
            let caps = &mut config.tuning.contract.states.caps;
            caps.body = [-8.0, 0.5];
            caps.total = [-8.0, 2.25];
            if let Some(last) = config.fatigue.curve.last_mut() {
                last[1] = 0.3;
            }
            if let Some(person) = config.tuning.contract.person.as_mut() {
                person.floor_drop = drop;
            }
        }
    };
    let (held, held_share, _) = worst_case(plant(7.0));
    let (open, _, _) = worst_case(plant(12.7));
    println!(
        "planted stack: lowest stage {:.2} with the floor, {:.2} without",
        lowest_rating(&held, width),
        lowest_rating(&open, width)
    );
    assert!(held.iter().all(|r| *r >= 1.0 - 1e-9), "{held:?}");
    assert!(held_share.iter().all(|d| *d >= -1e-9));
    assert!(
        open.iter().any(|r| *r < 1.0 - 1e-6),
        "without the floor the planted stack must fall below 11"
    );
}
