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
fn the_frozen_copy_has_a_table_for_each_stage_of_every_action() {
    let frozen: serde_json::Value =
        serde_json::from_str(engine::data::convert::FROZEN_V1).expect("the frozen copy parses");
    let actions = frozen["attributes"]["actions"]
        .as_object()
        .expect("the frozen copy has action tables");
    for action in ActionKind::ALL {
        let tables = actions[action.name()]
            .as_object()
            .unwrap_or_else(|| panic!("{action}: no frozen tables"));
        for stage in action.stages() {
            assert!(
                tables.contains_key(stage.name()),
                "{action}: no frozen {stage:?} table"
            );
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
            jobs.contains_key(first) || frozen_v3["attributes"].get(&def.name).is_some(),
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
/// rating type, and the named allow-list (lineup role fit, the fitted fast model, and the
/// plugin hook's share), no source reads a rating. A file's own unit tests are not play.
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
