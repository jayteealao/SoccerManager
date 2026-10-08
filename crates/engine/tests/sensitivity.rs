//! The sensitivity rules on small batches: every job has one rule, the job probe changes no
//! tick, a working job passes its rule, a job planted at zero weight fails it, amplified pace
//! does not pass its ceiling, and the five-match rule returns a share and a word. The full
//! run of every rule is `sensitivity_full.rs`. The working, planted and amplified-pace check
//! plays about 2,600 matches, so it is ignored and runs with the full run:
//!
//! ```text
//! cargo test --release -p engine --features sensitivity --test sensitivity --test sensitivity_full -- --ignored --nocapture
//! ```

mod common;

use engine::contract::{ActionKind, StageKind};
use engine::gate::StateWriter;
use engine::sensitivity::words::{RuleResult, Word};
use engine::sensitivity::{self, Size};
use engine::{Content, Simulation};
use sha2::{Digest, Sha256};

/// The shipped content with the test flag `pace_amplified` on: pace's speed amplification at
/// 10, the largest value the tuning file allows.
fn pace_amplified() -> Content {
    let dir = common::content_dir();
    let mut files = engine::data::ContentFiles::read(&dir).unwrap();
    let mut tuning: serde_json::Value = serde_json::from_slice(&files.tuning).unwrap();
    tuning["flags"]["pace_amplified"] = serde_json::json!({
        "owner": "sensitivity rules test",
        "hypothesis": "pace amplified to the largest speed difference the file allows carries more than its share of the outcome",
        "removal_condition": "test only; never shipped",
        "state": "on",
        "overrides": {"engine.contract.speed.amplification": 10.0}
    });
    files.tuning = serde_json::to_vec(&tuning).unwrap();
    let slots = std::fs::read(dir.path("slots.json")).unwrap();
    Content::from_files(&files)
        .unwrap()
        .with_slots(&slots)
        .unwrap()
}

/// Runs the rules of `jobs` on `content` with `matches` design matches and, when given, arms
/// of `arms` matches.
fn run(content: &Content, jobs: &[&str], matches: usize, arms: Option<usize>) -> Vec<RuleResult> {
    let dir = common::content_dir();
    let clubs = sensitivity::default_clubs(content, &dir).unwrap();
    let file = sensitivity::load_rules(content, &dir).unwrap();
    let size = Size {
        matches,
        arm_matches: arms,
        seed: file.design.seed,
        resamples: 499,
    };
    sensitivity::run_rules(content, &clubs, &file, Some(jobs), size).1
}

fn rule<'a>(results: &'a [RuleResult], job: &str) -> &'a RuleResult {
    results.iter().find(|r| r.job == job).unwrap()
}

/// Coverage: every attribute job and the three body jobs have exactly one rule, each
/// with its job's own statistic, and build has none.
#[test]
fn every_job_has_exactly_one_rule() {
    let content = common::content();
    let file = sensitivity::load_rules(&content, &common::content_dir()).unwrap();
    let body = ["height", "age", "nationality"];
    let mut jobs: Vec<&str> = content
        .attributes
        .attributes
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    jobs.extend(body);
    for job in &jobs {
        let n = file.rules.iter().filter(|r| r.job == *job).count();
        assert_eq!(n, 1, "{job} has {n} rules");
    }
    assert_eq!(file.rules.len(), jobs.len());
    for def in &content.attributes.attributes {
        let r = file.rules.iter().find(|r| r.job == def.name).unwrap();
        assert_eq!(r.statistic, def.job.statistic, "{}", def.name);
    }
    assert!(file.rules.iter().all(|r| r.job != "build"));
    assert_eq!(file.derived, vec!["build".to_string()]);
}

/// The SHA-256 of every tick's state bytes, and the events, of the seed's default match.
fn digests(seed: u64, probe: bool) -> (Vec<[u8; 32]>, Vec<engine::EngineEvent>) {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let config = engine::MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
    let mut sim = Simulation::new(config).unwrap();
    if probe {
        sim.with_probe();
    }
    let mut writer = StateWriter::new(false);
    let mut out = Vec::new();
    let mut all = Vec::new();
    while !sim.is_over() {
        sim.step();
        if sim.is_over() {
            sim.finish();
        }
        let events = sim.take_events();
        out.push(Sha256::digest(writer.tick(&sim, &events)).into());
        all.extend(events);
    }
    (out, all)
}

/// The probe reads the match and writes nothing: with it on, every tick's state digest and
/// every event equal those with it off.
#[test]
fn the_job_probe_leaves_every_tick_the_same() {
    for seed in 1..=3 {
        let (off, off_events) = digests(seed, false);
        let (on, on_events) = digests(seed, true);
        assert_eq!(off.len(), on.len(), "seed {seed}: tick count");
        if let Some(t) = (0..off.len()).find(|&t| off[t] != on[t]) {
            panic!("seed {seed}: tick {} differs with the probe on", t + 1);
        }
        assert_eq!(off_events, on_events, "seed {seed}: events");
    }
}

/// The one-rule check and the planted job: a design over tackling and pace at 400 matches
/// with arms of 250 clears tackling's least move surely and reports pace's share; a design of
/// 800 matches with tackling's main weight planted at zero (in memory: the loader refuses a
/// weight of 0) fails tackling's rule. With the pace flag on, pace's rule does not pass and
/// its share of the outcome is larger than with the flag off and above the ceiling.
#[test]
#[ignore = "about 2,600 matches; run it with the full sensitivity run"]
fn a_working_job_passes_a_planted_job_fails_and_amplified_pace_does_not_pass() {
    let content = common::content();
    let shipped = run(&content, &["tackling", "pace"], 400, Some(250));
    let tackling = rule(&shipped, "tackling");
    // A batch this small measures the move surely; the share's interval is too wide for a
    // sure pass, which the release-size run judges.
    assert!(tackling.mv.lo >= tackling.min_move, "{tackling:?}");
    assert_ne!(tackling.word, Word::Fail, "{tackling:?}");
    let pace_off = rule(&shipped, "pace").clone();

    let mut planted = content.clone();
    planted
        .attributes
        .actions
        .get_mut(&ActionKind::Tackle)
        .unwrap()
        .get_mut(&StageKind::Execute)
        .unwrap()
        .main
        .weight = 0.0;
    let zero = run(&planted, &["tackling"], 800, None);
    let tackling = rule(&zero, "tackling");
    assert_eq!(tackling.word, Word::Fail, "{tackling:?}");
    assert!(tackling.mv.hi < tackling.min_move, "{tackling:?}");

    let flagged = run(&pace_amplified(), &["pace"], 400, Some(250));
    let pace_on = rule(&flagged, "pace");
    assert_ne!(pace_on.word, Word::Pass, "{pace_on:?}");
    let (on, off) = (
        pace_on.share.unwrap().est.abs(),
        pace_off.share.unwrap().est.abs(),
    );
    assert!(
        on > off,
        "amplified pace's share {on:.3} is not above the shipped {off:.3}"
    );
    assert!(
        on > pace_on.ceiling,
        "amplified pace's share {on:.3} is not above the ceiling"
    );
}

/// The five-match runner at five runs per role returns a share between 0 and 1 for each
/// role and a word for the pooled share.
#[test]
fn the_five_match_rule_returns_a_share_and_a_word() {
    let content = common::content();
    let dir = common::content_dir();
    let clubs = sensitivity::default_clubs(&content, &dir).unwrap();
    let mut file = sensitivity::load_rules(&content, &dir).unwrap();
    file.five_match.runs = 5;
    let result =
        sensitivity::five_match::run(&content, &clubs, &file.five_match, file.design.seed, 199);
    assert_eq!(result.roles.len(), file.five_match.roles.len());
    for r in &result.roles {
        assert!((0.0..=1.0).contains(&r.share), "{r:?}");
        assert_eq!(r.runs, 5);
    }
    let p = result.pooled;
    assert!(p.lo <= p.est && p.est <= p.hi, "{p:?}");
    assert_eq!(
        result.word,
        sensitivity::five_match::word(p, file.five_match.target, file.five_match.tolerance)
    );
}
