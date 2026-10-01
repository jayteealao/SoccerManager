//! criterion benches: one tick step on a warmed simulation, and one steering pass.

use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main};
use engine::data::{TEAM_A_FILE, TEAM_B_FILE};
use engine::{Content, ContentDir, MatchConfig, Simulation};
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

fn steering_pass(c: &mut Criterion) {
    let config = config();
    let tuning = config.tuning.clone();
    let mut players = config.players.clone();
    let mut scratch = Vec::new();
    c.bench_function("steering_pass_22", |b| {
        b.iter(|| {
            engine::steering::step_all(
                &mut players,
                &mut scratch,
                &tuning,
                &engine::pitch::Pitch::DEFAULT,
            );
            black_box(players[0].pos)
        })
    });
}

criterion_group!(benches, tick_step, steering_pass);
criterion_main!(benches);
