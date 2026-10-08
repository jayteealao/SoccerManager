//! criterion benches: one tick step on a warmed simulation, the same step checked by the
//! running rule checker, the checker's phases apart (the tick record, the event half and
//! the tick rules), the separation pass, and one steering pass.

use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main};
use engine::data::{TEAM_A_FILE, TEAM_B_FILE};
use engine::math::DVec2;
use engine::modules::SteeringModule;
use engine::record::TickSink;
use engine::{Content, ContentDir, MatchConfig, RunningCheck, Simulation, StreamRules};
use std::hint::black_box;

fn config() -> MatchConfig {
    let dir = ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
    let content = Content::load(&dir).unwrap();
    let a = content
        .load_team(&dir, &dir.path(TEAM_A_FILE))
        .unwrap()
        .value;
    let b = content
        .load_team(&dir, &dir.path(TEAM_B_FILE))
        .unwrap()
        .value;
    MatchConfig::new(42, 90, &content, [&a, &b]).unwrap()
}

fn tick_step(c: &mut Criterion) {
    // Names the build each run timed, for the recorder's cost compare.
    eprintln!("debug-trace compiled in: {}", engine::trace::COMPILED);
    let config = config();
    let warmed = || {
        let mut sim = Simulation::new(config.clone()).unwrap();
        for _ in 0..500 {
            sim.step();
        }
        sim
    };
    let mut sim = warmed();
    c.bench_function("tick_step", |b| {
        b.iter(|| {
            // A match ends at full time; a new one starts so every sample times a real tick.
            if sim.is_over() {
                sim = warmed();
            }
            sim.step();
            black_box(sim.tick())
        })
    });
}

fn running_check_tick(c: &mut Criterion) {
    // One step fed to the running rule checker as `Simulation::run` feeds it; the checker's
    // share of a tick is this bench's time less `tick_step`'s.
    let config = config();
    let warmed = || {
        let mut sim = Simulation::new(config.clone()).unwrap();
        let mut check = RunningCheck::for_match(&sim, StreamRules::for_config(&config));
        for _ in 0..500 {
            sim.step();
            check.on_step(&sim).unwrap();
            check.on_tick(&sim.record()).unwrap();
        }
        (sim, check)
    };
    let (mut sim, mut check) = warmed();
    c.bench_function("running_check_tick", |b| {
        b.iter(|| {
            if sim.is_over() {
                (sim, check) = warmed();
            }
            sim.step();
            check.on_step(&sim).unwrap();
            check.on_tick(&sim.record()).unwrap();
            black_box(check.ticks())
        })
    });
}

/// The records of ticks 500 to 2,500 of one match, and the match at tick 500 for a fresh
/// checker: the running checker's two halves are timed apart on real records.
fn recorded() -> (MatchConfig, Vec<engine::record::TickRecord>) {
    let config = config();
    let mut sim = Simulation::new(config.clone()).unwrap();
    for _ in 0..500 {
        sim.step();
    }
    let records = (0..2000)
        .map(|_| {
            sim.step();
            sim.record()
        })
        .collect();
    (config, records)
}

fn running_check_phases(c: &mut Criterion) {
    // Building one tick record from a warmed match.
    let mut sim = Simulation::new(config()).unwrap();
    for _ in 0..500 {
        sim.step();
    }
    c.bench_function("tick_record", |b| b.iter(|| black_box(sim.record())));

    // The event half: what one step added (shapes and events), fed to the checker.
    let config = config();
    let warmed = || {
        let mut sim = Simulation::new(config.clone()).unwrap();
        let mut check = RunningCheck::for_match(&sim, StreamRules::for_config(&config));
        for _ in 0..500 {
            sim.step();
            check.on_step(&sim).unwrap();
        }
        (sim, check)
    };
    let (mut sim, mut check) = warmed();
    c.bench_function("running_check_step", |b| {
        b.iter_custom(|iters| {
            let mut spent = std::time::Duration::ZERO;
            for _ in 0..iters {
                if sim.is_over() {
                    (sim, check) = warmed();
                }
                sim.step();
                let start = std::time::Instant::now();
                check.on_step(&sim).unwrap();
                spent += start.elapsed();
            }
            spent
        })
    });

    // The tick half: the tick rules on one prebuilt record, in record order.
    let (config, records) = recorded();
    let fresh = || {
        let sim = Simulation::new(config.clone()).unwrap();
        RunningCheck::for_match(&sim, StreamRules::for_config(&config))
    };
    let mut check = fresh();
    let mut next = 0;
    c.bench_function("running_check_on_tick", |b| {
        b.iter(|| {
            if next == records.len() {
                check = fresh();
                next = 0;
            }
            check.on_tick(&records[next]).unwrap();
            next += 1;
            black_box(check.ticks())
        })
    });
}

fn steering_separate(c: &mut Criterion) {
    // The end-of-step separation pass on a warmed match.
    let mut sim = Simulation::new(config()).unwrap();
    for _ in 0..500 {
        sim.step();
    }
    let steering: &dyn SteeringModule = &engine::steering::SteeringV1;
    c.bench_function("steering_separate_22", |b| {
        b.iter(|| black_box(steering.separate(&sim.view())))
    });
}

fn steering_pass(c: &mut Criterion) {
    // The steering module the loop resolves, through the read-only view, as the movement pass
    // calls it: every active player's next velocity.
    let mut sim = Simulation::new(config()).unwrap();
    for _ in 0..500 {
        sim.step();
    }
    let steering: &dyn SteeringModule = &engine::steering::SteeringV1;
    c.bench_function("steering_pass_22", |b| {
        b.iter(|| {
            let view = sim.view();
            let mut sum = DVec2::ZERO;
            for i in 0..view.players().len() {
                sum += steering.next_velocity(&view, i);
            }
            black_box(sum)
        })
    });
}

criterion_group!(
    benches,
    tick_step,
    running_check_tick,
    running_check_phases,
    steering_separate,
    steering_pass
);
criterion_main!(benches);
