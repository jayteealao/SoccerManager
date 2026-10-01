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
/// phase's name, then finishes it and returns its phase log.
fn play_logged(mut sim: Simulation) -> Vec<Step> {
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
    sim.phase_log().to_vec()
}

/// The batch: five seeds at 90 minutes on the default ground, seed 42 on a 100 by 64 and a
/// 120 by 90 ground, a level knockout match that goes to the shoot-out, a match abandoned
/// below the minimum, and a dead ball whose taker is injured.
fn batch() -> Vec<(String, Vec<Step>)> {
    let mut out = Vec::new();
    for seed in [42, 1, 7, 99, 2026] {
        let sim = Simulation::new(full_match(seed, 105.0, 68.0)).unwrap();
        out.push((format!("seed {seed}"), play_logged(sim)));
    }
    for (length, width) in [(100.0, 64.0), (120.0, 90.0)] {
        let sim = Simulation::new(full_match(42, length, width)).unwrap();
        out.push((format!("seed 42 on {length} by {width}"), play_logged(sim)));
    }
    let shootout = Scene::new(calm_match(5).with_knockout())
        .manager(0, engine::Manager::Human)
        .manager(1, engine::Manager::Human)
        .build();
    out.push(("shoot-out".into(), play_logged(shootout)));
    out.push(("abandoned".into(), play_logged(abandoned())));
    out.push(("renamed taker".into(), play_logged(renamed_taker())));
    let half = TICKS_PER_MINUTE * 5;
    let at_half_time = Scene::new(calm_match(10))
        .tick(half - 2)
        .injure(index(0, 2))
        .build();
    out.push(("dead at half-time".into(), play_logged(at_half_time)));
    let at_full_time = Scene::new(calm_match(10))
        .at_minute(9)
        .tick(2 * half - 2)
        .injure(index(0, 2))
        .build();
    out.push(("dead at full time".into(), play_logged(at_full_time)));
    let at_shootout = Scene::new(calm_match(5).with_knockout())
        .manager(0, engine::Manager::Human)
        .manager(1, engine::Manager::Human)
        .at_minute(4)
        .tick(half - 2)
        .injure(index(0, 2))
        .build();
    out.push(("dead at the shoot-out".into(), play_logged(at_shootout)));
    let round_limit = Scene::new(calm_match(5).with_knockout())
        .manager(0, engine::Manager::Human)
        .manager(1, engine::Manager::Human)
        .shootout_kicks(&[true; 2 * SAFETY_ROUNDS as usize])
        .build();
    out.push(("shoot-out round limit".into(), play_logged(round_limit)));
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
    for (name, log) in batch() {
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
/// that restores it, and the scene builder.
fn phase_writes(files: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    let mut in_machine = 0;
    for (path, text) in files {
        for line in text.lines() {
            let code = line.trim_start();
            if code.starts_with("//") || !code.contains(".phase = ") {
                continue;
            }
            match path.as_str() {
                "snapshot.rs" | "scenario.rs" => {}
                "rules/mod.rs" if code == "self.referee.phase = next;" => in_machine += 1,
                _ => found.push(format!("{path}: {code}")),
            }
        }
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
