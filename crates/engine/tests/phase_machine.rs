//! The phase machine (IFAB Laws 7, 8 and 10 to 17 as the engine plays them). Every change of
//! phase in a batch of full matches, a shoot-out, two other grounds, an abandonment and a
//! renamed taker is a transition the machine's table declares; the named phase always equals
//! the stored phase's name at a tick boundary; and only the machine writes the phase. Each
//! check runs once against a planted fault, so it is proven able to fail.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::{calm_match, default_teams, index};
use engine::data::TeamFile;
use engine::data::team::Ground;
use engine::rules::clock::TICKS_PER_MINUTE;
use engine::rules::phases::{self, Cause, PhaseName, Step, TRANSITIONS};
use engine::rules::shootout::SAFETY_ROUNDS;
use engine::scenario::Scene;
use engine::{MatchConfig, Simulation};

/// `file` with its club's ground set to `length` by `width` metres.
fn on_ground(file: &TeamFile, length: f64, width: f64) -> TeamFile {
    let mut out = file.clone();
    out.club.ground = Ground { length, width };
    out
}

/// A 90-minute match of `seed` on the shipped content, the home team on a ground of `length`
/// by `width`.
fn full_match(seed: u64, length: f64, width: f64) -> MatchConfig {
    let content = common::content();
    let [a, b] = default_teams(&content);
    let home = on_ground(&a, length, width);
    MatchConfig::new(seed, 90, &content, [&home, &b]).unwrap()
}

/// Steps `sim` to full time, checking after every tick that the named phase equals the stored
/// phase's name, then finishes it.
fn play(mut sim: Simulation) -> Simulation {
    for _ in 0..1_000_000 {
        if sim.is_over() {
            break;
        }
        sim.step();
        assert_eq!(
            sim.phase_name(),
            sim.derived_phase_name(),
            "the named phase drifted from the stored phase"
        );
    }
    assert!(sim.is_over(), "the match never ended");
    sim.finish();
    sim
}

/// [`play`], returning the phase log.
fn play_logged(sim: Simulation) -> Vec<Step> {
    play(sim).phase_log().to_vec()
}

/// The batch: five seeds at 90 minutes on the default ground, seed 42 on a 100 by 64 and a
/// 120 by 90 ground, a level knockout match that goes to the shoot-out, a match abandoned
/// below the minimum, and a dead ball whose taker is injured.
fn batch() -> Vec<(String, Simulation)> {
    let mut out = Vec::new();
    for seed in [42, 1, 7, 99, 2026] {
        let sim = Simulation::new(full_match(seed, 105.0, 68.0)).unwrap();
        out.push((format!("seed {seed}"), play(sim)));
    }
    for (length, width) in [(100.0, 64.0), (120.0, 90.0)] {
        let sim = Simulation::new(full_match(42, length, width)).unwrap();
        out.push((format!("seed 42 on {length} by {width}"), play(sim)));
    }
    let shootout = Scene::new(calm_match(5).with_knockout())
        .manager(0, engine::Manager::Human)
        .manager(1, engine::Manager::Human)
        .build();
    out.push(("shoot-out".into(), play(shootout)));
    out.push(("abandoned".into(), play(abandoned())));
    out.push(("renamed taker".into(), play(renamed_taker())));
    let half = TICKS_PER_MINUTE * 5;
    let at_half_time = Scene::new(calm_match(10))
        .tick(half - 2)
        .injure(index(0, 2))
        .build();
    out.push(("dead at half-time".into(), play(at_half_time)));
    let at_full_time = Scene::new(calm_match(10))
        .at_minute(9)
        .tick(2 * half - 2)
        .injure(index(0, 2))
        .build();
    out.push(("dead at full time".into(), play(at_full_time)));
    let at_shootout = Scene::new(calm_match(5).with_knockout())
        .manager(0, engine::Manager::Human)
        .manager(1, engine::Manager::Human)
        .at_minute(4)
        .tick(half - 2)
        .injure(index(0, 2))
        .build();
    out.push(("dead at the shoot-out".into(), play(at_shootout)));
    let round_limit = Scene::new(calm_match(5).with_knockout())
        .manager(0, engine::Manager::Human)
        .manager(1, engine::Manager::Human)
        .shootout_kicks(&[true; 2 * SAFETY_ROUNDS as usize])
        .build();
    out.push(("shoot-out round limit".into(), play(round_limit)));
    out
}

/// Four home players sent off before kick-off leave seven; an injury in open play leaves six,
/// below the rule pack's minimum of seven, so the match is abandoned at the dropped ball.
fn abandoned() -> Simulation {
    Scene::new(calm_match(5))
        .sent_off(index(0, 7))
        .sent_off(index(0, 8))
        .sent_off(index(0, 9))
        .sent_off(index(0, 10))
        .injure(index(0, 2))
        .build()
}

/// An injury in open play stops it for a dropped ball; the dropped ball's taker is then
/// injured too, so the restart gets a new taker.
fn renamed_taker() -> Simulation {
    let first = Scene::new(calm_match(5)).injure(index(0, 2)).build();
    let taker = first.dead_ball().expect("the injury stops play").taker;
    Scene::new(calm_match(5))
        .injure(index(0, 2))
        .injure(taker)
        .build()
}

/// Rows the batch does not reach, each with the reason no test match reaches it.
const UNREACHED: &[(usize, &str)] = &[
    (
        10,
        "a red card, or a card held for advantage, shown in open play that leaves a team below the minimum",
    ),
    (
        12,
        "a card held for advantage, shown at the break, that leaves a team below the minimum",
    ),
];

#[test]
fn every_change_of_phase_in_the_batch_is_declared_and_every_row_is_reached() {
    let mut reached = BTreeSet::new();
    let mut faults = Vec::new();
    for (name, sim) in batch() {
        let log = sim.phase_log().to_vec();
        assert!(!log.is_empty(), "{name}: no change of phase recorded");
        assert_eq!(
            log[0].cause,
            Cause::KickOffTaken,
            "{name}: the match starts with its kick-off"
        );
        assert_eq!(log[0].from, PhaseName::PreMatch, "{name}");
        assert_eq!(log.last().unwrap().to, PhaseName::FullTime, "{name}");
        for fault in phases::undeclared(&log) {
            faults.push(format!("{name}: {fault}"));
        }
        for step in &log {
            if let Some(row) = phases::declared(step.from, step.to, step.cause) {
                reached.insert(row);
            }
        }
    }
    assert!(faults.is_empty(), "{}", faults.join("\n"));
    let unreached: Vec<String> = (0..TRANSITIONS.len())
        .filter(|r| !reached.contains(r) && !UNREACHED.iter().any(|(u, _)| u == r))
        .map(|r| format!("row {r}: {:?}", TRANSITIONS[r]))
        .collect();
    assert!(
        unreached.is_empty(),
        "rows the batch never reached:\n{}",
        unreached.join("\n")
    );
    for (row, why) in UNREACHED {
        assert!(!reached.contains(row), "row {row} is reached now: {why}");
    }
}

/// The validator finds a planted step that no row declares.
#[test]
fn a_planted_undeclared_step_fails_the_validator() {
    let mut log = play_logged(Simulation::new(calm_match(5)).unwrap());
    assert!(phases::undeclared(&log).is_empty());
    let at = log.len() / 2;
    let planted = Step {
        tick: log[at].tick,
        from: PhaseName::FullTime,
        to: PhaseName::OpenPlay,
        cause: Cause::RestartTaken,
    };
    log.insert(at, planted);
    let faults = phases::undeclared(&log);
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(
        faults[0].contains("full time -> open play (restart taken)"),
        "{}",
        faults[0]
    );
}

/// Every write of the stored phase in the engine's source, as `path: line`, outside the
/// files allowed to write it: the machine's one writer in the referee, the snapshot reader
/// that restores it, the scene builder, and the scene builder's one seat in the referee
/// (`seat_phase`, compiled only for test scenes).
fn phase_writes(files: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    let mut in_machine = 0;
    let mut seats = 0;
    for (path, text) in files {
        for line in text.lines() {
            let code = line.trim_start();
            if code.starts_with("//") || !code.contains(".phase = ") {
                continue;
            }
            match path.as_str() {
                "snapshot.rs" | "scenario.rs" => {}
                "rules/mod.rs" if code == "self.referee.phase = next;" => in_machine += 1,
                "rules/mod.rs" if code == "self.referee.phase = phase;" => seats += 1,
                _ => found.push(format!("{path}: {code}")),
            }
        }
    }
    if seats > 1 {
        found.push(format!("rules/mod.rs: {seats} scene seats, not 1"));
    }
    if in_machine != 1 {
        found.push(format!(
            "rules/mod.rs: {in_machine} writes in the machine, not 1"
        ));
    }
    found
}

/// The engine's sources as (path relative to `src` with `/`, text).
fn engine_sources() -> Vec<(String, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    common::sources(&src)
        .into_iter()
        .map(|p| {
            let rel = p
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (rel, std::fs::read_to_string(&p).unwrap())
        })
        .collect()
}

#[test]
fn only_the_phase_machine_writes_the_phase() {
    let files = engine_sources();
    assert!(files.iter().any(|(p, _)| p == "rules/mod.rs"));
    let writes = phase_writes(&files);
    assert!(writes.is_empty(), "{}", writes.join("\n"));
    // The control: a planted write outside the machine is found.
    let mut planted = files.clone();
    planted.push((
        "sim/fatigue.rs".into(),
        "        self.referee.phase = Phase::FullTime;\n".into(),
    ));
    assert_eq!(
        phase_writes(&planted),
        ["sim/fatigue.rs: self.referee.phase = Phase::FullTime;"]
    );
}

/// One played match: its summary, its events, and its last tick record.
fn played(
    config: MatchConfig,
) -> (
    engine::Summary,
    Vec<engine::EngineEvent>,
    engine::TickRecord,
) {
    let mut sim = Simulation::new(config).unwrap();
    let mut events = Vec::new();
    while !sim.is_over() {
        sim.step();
        events.extend(sim.take_events());
    }
    sim.finish();
    events.extend(sim.take_events());
    (sim.summary(), events, sim.record())
}

/// The shipped rule pack with video reviews priced at 90 s, read from a copy of the rule file
/// that writes the field.
fn priced_content() -> engine::Content {
    let shipped = std::fs::read_to_string(common::content_dir().path("rules/default.json"))
        .expect("the shipped rule pack reads");
    assert!(
        !shipped.contains("video_review_s"),
        "the shipped pack names no price"
    );
    let priced = shipped.replacen("\"card_s\":", "\"video_review_s\": 90,\n    \"card_s\":", 1);
    assert_ne!(priced, shipped);
    let rules = engine::data::load_json_bytes::<engine::data::RulePack>(
        "rules",
        priced.as_bytes(),
        "rules/priced.json",
        engine::data::RULES_VERSION,
        &(),
    )
    .expect("a rule pack that prices video reviews loads")
    .value;
    assert_eq!(rules.added_time.video_review_s, 90);
    let mut content = common::content();
    assert_eq!(
        content.rules.added_time.video_review_s, 60,
        "the default price"
    );
    content.rules = rules;
    content
}

/// Pricing video reviews changes no added time and no event, because no event counts a
/// review: seed 42, seed 7, and a level knockout match with extra time and a shoot-out.
#[test]
fn pricing_video_reviews_changes_no_added_time() {
    let shipped = common::content();
    let priced = priced_content();
    for (name, seed, knockout) in [
        ("seed-42", 42, false),
        ("seed-7", 7, false),
        ("knockout", 42, true),
    ] {
        let config = |content: &engine::Content| {
            let [a, b] = default_teams(content);
            let mut config = MatchConfig::new(seed, 90, content, [&a, &b]).unwrap();
            if knockout {
                // Nobody decides, so the match stays level into extra time and the shoot-out.
                config.tuning.decision_interval_ticks = u32::MAX;
                config = config.with_knockout();
            }
            config
        };
        let (s0, e0, r0) = played(config(&shipped));
        let (s1, e1, r1) = played(config(&priced));
        assert!(
            s0.added_s.iter().all(|&s| s > 0),
            "{name}: added time is played"
        );
        assert_eq!(s0.added_s, s1.added_s, "{name}");
        assert_eq!(s0.extra_added_s, s1.extra_added_s, "{name}");
        assert_eq!(s0, s1, "{name}");
        assert_eq!(e0, e1, "{name}");
        assert_eq!(r0, r1, "{name}");
        if knockout {
            assert!(
                s0.extra_time && s0.shootout.is_some(),
                "{name}: extra time and a shoot-out"
            );
        }
    }
}

/// The control: two reviews counted in the first half add twice their price to it, so the
/// comparison above can fail.
#[test]
fn counted_video_reviews_add_their_price() {
    let first_half_added = |reviews: u32| {
        let mut sim = Scene::new(priced_content_config())
            .tick(45 * TICKS_PER_MINUTE - 10)
            // Three goals' worth of stoppages keep the half clear of the minimum.
            .tally(engine::StoppageKind::Goal, 3)
            .reviews(reviews)
            .build();
        while sim.summary().added_s[0] == 0 {
            sim.step();
        }
        sim.summary().added_s[0]
    };
    assert_eq!(first_half_added(2), first_half_added(0) + 2 * 90);
}

/// Seed 42 over 90 minutes with the priced rule pack and nobody deciding.
fn priced_content_config() -> MatchConfig {
    let content = priced_content();
    let [a, b] = default_teams(&content);
    let mut config = MatchConfig::new(42, 90, &content, [&a, &b]).unwrap();
    config.tuning.decision_interval_ticks = u32::MAX;
    config
}

/// The restart position check over the batch, reported and not enforced: every restart
/// whose positions break a Law, one line each. Run with `-- --ignored --nocapture`.
#[test]
#[ignore = "a report: prints every restart position fault in the batch"]
fn restart_position_report() {
    let mut total = 0;
    for (name, sim) in batch() {
        for fault in sim.position_faults() {
            println!("{name}: {fault}");
            total += 1;
        }
    }
    println!("{total} restart position faults");
}

/// [`restart_position_report`] over forty more seeds at 90 minutes on the default ground,
/// with the knockout switch on so level matches play extra time and a shoot-out. Run with
/// `--release -- --ignored --nocapture`.
#[test]
#[ignore = "a report: prints every restart position fault in forty knockout matches"]
fn restart_position_report_wide() {
    let mut total = 0;
    for seed in 100..140 {
        let sim = play(Simulation::new(full_match(seed, 105.0, 68.0).with_knockout()).unwrap());
        for fault in sim.position_faults() {
            println!("seed {seed} knockout: {fault}");
            total += 1;
        }
    }
    println!("{total} restart position faults");
}

/// The phase steps that take a restart: a kick-off that starts a period, a dead ball's
/// restart, or a shoot-out kick.
fn restarts_taken(log: &[Step]) -> u32 {
    log.iter()
        .filter(|s| {
            matches!(
                s.cause,
                Cause::KickOffTaken | Cause::RestartTaken | Cause::ShootoutKickTaken
            )
        })
        .count() as u32
}

/// In debug mode every restart of the batch is judged and every position stands
/// where the Laws ask. The count of judged restarts equals the restarts the phase machine
/// recorded, and the restart events of a match equal the judged restarts plus the dead balls
/// a period end closed before their restart.
#[test]
fn in_debug_mode_every_restart_passes_the_position_check() {
    let mut matches: Vec<(String, Simulation)> = Vec::new();
    for seed in [42, 1, 7, 99, 2026] {
        let sim = Simulation::new_traced(full_match(seed, 105.0, 68.0)).unwrap();
        assert!(sim.debug_trace_on());
        matches.push((format!("seed {seed}"), sim));
    }
    for (length, width) in [(100.0, 64.0), (120.0, 90.0)] {
        let sim = Simulation::new_traced(full_match(42, length, width)).unwrap();
        matches.push((format!("seed 42 on {length} by {width}"), sim));
    }
    for (name, sim) in matches {
        let mut sim = sim;
        let mut events = Vec::new();
        while !sim.is_over() {
            sim.step();
            let _ = sim.take_trace();
            events.extend(sim.take_events());
        }
        sim.finish();
        events.extend(sim.take_events());
        assert!(
            sim.position_faults().is_empty(),
            "{name}: {:?}",
            sim.position_faults()
        );
        let log = sim.phase_log();
        let checked = sim.restarts_checked();
        assert_eq!(checked, restarts_taken(log), "{name}");
        assert!(checked > 40, "{name}: {checked} restarts");
        let restart_events = events
            .iter()
            .filter(|e| {
                use engine::EngineEventKind as K;
                matches!(
                    e.kind,
                    K::KickOff | K::ThrowIn | K::Corner | K::GoalKick | K::FreeKick | K::Penalty
                ) || (e.kind == K::Injury && e.spot.is_some())
            })
            .count() as u32;
        let closed = log
            .iter()
            .filter(|s| {
                matches!(s.from, PhaseName::DeadBall(_))
                    && matches!(s.to, PhaseName::HalfTime | PhaseName::FullTime)
            })
            .count() as u32;
        assert_eq!(restart_events, checked + closed, "{name}");
    }
}

/// The induced fault: a penalty with a defender planted inside the penalty area is
/// taken through the engine's own restart path, and the check fails loudly with the phase,
/// the player, his position, and the rule.
#[test]
fn an_illegal_position_at_a_restart_fails_naming_the_phase_the_player_and_the_position() {
    let config = calm_match(5);
    let attack_x = config.teams[0].attack_x;
    let half_length = config.pitch().half_length();
    let defender = index(1, 4);
    // The legal scene first: it takes the penalty without a fault.
    let legal = Scene::new(config.clone()).penalty(0, index(0, 10)).build();
    let legal = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let mut sim = legal;
        let before = sim.restarts_checked();
        sim.step();
        sim.restarts_checked() - before
    }));
    assert_eq!(
        legal.ok(),
        Some(1),
        "the legal penalty is judged and passes"
    );
    let at = engine::math::DVec2::new(attack_x * (half_length - 8.0), 6.0);
    let mut sim = Scene::new(config)
        .penalty(0, index(0, 10))
        .place(defender, at)
        .build();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || sim.step()))
        .expect_err("the check fails loudly");
    let message = caught.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(
        message.starts_with("restart position fault at tick 1: dead ball (penalty) (penalty): "),
        "{message}"
    );
    assert!(
        message.contains(&format!("player {defender} (team 1, slot 4)")),
        "{message}"
    );
    assert!(
        message.contains(&format!("at ({:.2}, 6.00)", at.x)),
        "{message}"
    );
    assert!(
        message.contains("Law 14: inside the penalty area"),
        "{message}"
    );
}

/// A sweep of the enforced check over many 90-minute knockout matches on the default
/// ground: each match that stops on a restart position fault prints the fault. The seeds
/// run from 1000 to 1000 plus `SWEEP` (default 200). Run with `--release -- --ignored
/// --nocapture`.
#[test]
#[ignore = "a sweep: prints every match the restart position check stops"]
fn restart_position_sweep() {
    let n: u64 = std::env::var("SWEEP").map_or(200, |v| v.parse().unwrap());
    let stopped = common::run_many(1000..=1000 + n - 1, |seed| {
        let config = full_match(seed, 105.0, 68.0).with_knockout();
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            play(Simulation::new(config).unwrap()).restarts_checked()
        }))
        .err()
        .map(|e| {
            let message = e.downcast_ref::<String>().cloned().unwrap_or_default();
            format!("seed {seed}: {message}")
        })
    });
    let stopped: Vec<String> = stopped.into_iter().flatten().collect();
    for line in &stopped {
        println!("{line}");
    }
    println!("{} of {n} matches stopped by the check", stopped.len());
}
