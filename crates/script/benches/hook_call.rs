//! The cost of one hook call of the shipped sample pack: the decision hook, which runs once
//! per refresh, and the rule hook, which runs once per foul.

use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main};
use engine::Card;
use engine::plugin::{DecisionContext, FoulContext};
use script::LoadedPack;

fn sample() -> LoadedPack {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/scripts/sample");
    LoadedPack::load(&dir).expect("the sample pack loads")
}

fn hook_call(c: &mut Criterion) {
    let pack = sample();
    let mut plugins = pack.plugins();
    // A trailing carrier 20 m from goal under close pressure: both branches of the sample
    // `decide` run.
    let decision = DecisionContext {
        tick: 210_000,
        minute: 70,
        team: 0,
        slot: 9,
        goals_for: 0,
        goals_against: 1,
        goal_distance: 20.0,
        nearest_opponent: 2.0,
        progress: 0.6,
    };
    let foul = FoulContext {
        tick: 12_000,
        minute: 4,
        team: 1,
        slot: 4,
        yellows: 0,
        aggression: 0.6,
        advantage: false,
        penalty: false,
    };
    let hook = plugins
        .decision
        .as_mut()
        .expect("the sample has a decision hook");
    c.bench_function("sample_decide", |b| {
        b.iter(|| std::hint::black_box(hook.adjust(std::hint::black_box(&decision))))
    });
    let rule = plugins.rule.as_mut().expect("the sample has a rule hook");
    c.bench_function("sample_card", |b| {
        b.iter(|| std::hint::black_box(rule.card(std::hint::black_box(&foul), Some(Card::Yellow))))
    });
}

criterion_group!(benches, hook_call);
criterion_main!(benches);
