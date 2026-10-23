//! The running rule checker may not get slower. It is timed against the old whole-match
//! validator on the same recorded match, as a ratio: the old validator is not edited, so
//! the ratio moves only with the running checker, and load from other work on the machine
//! slows both sides alike. A checker whose ratio is more than 10 percent above the kept
//! ratio fails.

mod common;

use std::hint::black_box;
use std::time::{Duration, Instant};

use engine::record::{TickRecord, VecSink};
use engine::team::Team;
use engine::{EngineEvent, RunningCheck, Simulation, StreamRules, Tuning, Validator};

/// The median ratio (running checker over old validator) measured on the checker before
/// it was made faster, 2026-10-22, release build.
const BEFORE_RATIO: f64 = 0.8827;

/// The ratio a checker must not exceed by more than [`TOLERANCE`]: the median of three
/// release runs on the faster checker (reused shape anchors, job targets without a copy),
/// 2026-10-23: 0.7055, 0.7062, 0.7049. The old validator again calls play's direct
/// anchor, as it did for [`BEFORE_RATIO`]; while play built its anchor from the split frame
/// (2026-10-22) the old validator was slower and the ratio read 0.6226.
const KEPT_RATIO: f64 = 0.7055;

/// How far above the kept ratio a measured ratio may lie before it fails: timing noise.
const TOLERANCE: f64 = 0.10;

/// Rounds of the alternated timing; the median of the per-round ratios is judged.
const ROUNDS: usize = 15;

/// Runs of each side per round; the fastest counts.
const RUNS: usize = 5;

/// One recorded match: what both checkers read.
struct Input {
    tuning: Tuning,
    timeline: Vec<(u32, [Team; 2])>,
    events: Vec<EngineEvent>,
    records: Vec<TickRecord>,
    rules: StreamRules,
}

/// The seed-42 10-minute match, recorded once.
fn recorded() -> Input {
    let config = common::short_match(10);
    let rules = StreamRules::for_config(&config);
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut sink = VecSink::default();
    sim.run(&mut sink).unwrap();
    Input {
        tuning: config.tuning.clone(),
        timeline: sim.team_timeline().to_vec(),
        events: sim.take_events(),
        records: sink.records,
        rules,
    }
}

/// The old validator's whole report: its count.
fn reference(input: &Input) -> usize {
    let mut found = Validator::for_match(input.tuning.clone(), &input.timeline, &input.events)
        .check(&input.records);
    found.extend(Validator::check_events(&input.events, &input.rules));
    found.len()
}

/// The running checker fed in tick order, as a match feeds it: its whole count.
fn running(input: &Input) -> usize {
    let (timeline, events) = (&input.timeline, &input.events);
    let mut check = RunningCheck::new(
        input.tuning.clone(),
        timeline[0].1.clone(),
        input.rules.clone(),
    );
    let (mut shape, mut event) = (1, 0);
    for r in &input.records {
        while shape < timeline.len() && timeline[shape].0 <= r.tick {
            check.shapes(timeline[shape].0, timeline[shape].1.clone());
            shape += 1;
        }
        while event < events.len() && events[event].tick <= r.tick {
            check.event(&events[event]);
            event += 1;
        }
        check.tick(r);
    }
    for (tick, teams) in &timeline[shape..] {
        check.shapes(*tick, teams.clone());
    }
    for e in &events[event..] {
        check.event(e);
    }
    check.finish().len()
}

/// The fastest of [`RUNS`] runs of `f`.
fn fastest(f: &dyn Fn() -> usize) -> Duration {
    (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            black_box(f());
            start.elapsed()
        })
        .min()
        .expect("at least one run")
}

/// The median over [`ROUNDS`] rounds of the running checker's time over the old
/// validator's; the side that runs first alternates by round.
fn measure(input: &Input) -> f64 {
    let mut ratios: Vec<f64> = (0..ROUNDS)
        .map(|round| {
            let (old, new) = if round % 2 == 0 {
                let old = fastest(&|| reference(input));
                (old, fastest(&|| running(input)))
            } else {
                let new = fastest(&|| running(input));
                (fastest(&|| reference(input)), new)
            };
            new.as_secs_f64() / old.as_secs_f64()
        })
        .collect();
    ratios.sort_by(f64::total_cmp);
    ratios[ROUNDS / 2]
}

/// Fails a measured ratio more than [`TOLERANCE`] above `kept`.
fn judge(kept: f64, measured: f64) -> Result<(), String> {
    if measured > kept * (1.0 + TOLERANCE) {
        Err(format!(
            "the rule checker is slower: ratio {measured:.4} against the kept {kept:.4} \
             (at most {:.4})",
            kept * (1.0 + TOLERANCE)
        ))
    } else {
        Ok(())
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "timing: run with --release")]
fn the_running_checker_is_no_slower_than_its_kept_ratio() {
    let input = recorded();
    assert_eq!(running(&input), reference(&input), "both checkers agree");
    let measured = measure(&input);
    println!(
        "checker ratio {measured:.4} (kept {KEPT_RATIO:.4}, before {BEFORE_RATIO:.4}, {} records)",
        input.records.len()
    );
    judge(KEPT_RATIO, measured).unwrap();
}

#[test]
fn a_slower_ratio_fails_the_judge() {
    assert!(judge(0.40, 0.45).is_err());
    assert!(judge(0.40, 0.43).is_ok());
    assert!(judge(0.40, 0.30).is_ok());
}
