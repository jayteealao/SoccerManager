//! criterion benches: one tick step on a warmed simulation, and one steering pass.

use criterion::{Criterion, criterion_group, criterion_main};
use engine::{MatchConfig, Simulation};
use std::hint::black_box;

fn tick_step(c: &mut Criterion) {
    let mut sim = Simulation::new(MatchConfig::new(42, 90).unwrap()).unwrap();
    for _ in 0..500 {
        sim.step();
    }
    c.bench_function("tick_step", |b| {
        b.iter(|| {
            sim.step();
            black_box(sim.tick())
        })
    });
}

fn steering_pass(c: &mut Criterion) {
    let sim = Simulation::new(MatchConfig::new(42, 90).unwrap()).unwrap();
    let tuning = sim.tuning().clone();
    let teams = sim.teams();
    let mut players = teams[0].players(0, engine::player::Attributes::uniform(60));
    players.extend(teams[1].players(11, engine::player::Attributes::uniform(60)));
    let mut scratch = Vec::new();
    c.bench_function("steering_pass_22", |b| {
        b.iter(|| {
            engine::steering::step_all(&mut players, &mut scratch, &tuning);
            black_box(players[0].pos)
        })
    });
}

criterion_group!(benches, tick_step, steering_pass);
criterion_main!(benches);
