//! AC-6: a clearly stronger team wins more than half its matches. Team A is default-b with
//! every attribute times 1.15 under a new club id; it plays default-b over seeds 1 to 200, at
//! home for the first 100 and away for the rest, with both sides managed by the AI.
//!
//! Slow: `cargo test --release -p engine -- --ignored`.

mod common;

use common::{content, default_teams, judge_band, limits, run_many, stronger, units};
use engine::record::NullSink;
use engine::{MatchConfig, Simulation};

#[test]
#[ignore = "slow: cargo test --release -p engine -- --ignored"]
fn a_stronger_team_wins_more_than_half_its_matches() {
    let content = content();
    let [_, b] = default_teams(&content);
    let strong = stronger(&b);
    let results = run_many(1..=200, |seed| {
        let home = seed <= 100;
        let teams = if home { [&strong, &b] } else { [&b, &strong] };
        let mut sim =
            Simulation::new(MatchConfig::new(seed, 90, &content, teams).unwrap()).unwrap();
        sim.run(&mut NullSink).unwrap();
        let goals = sim.summary().goals;
        let (a, other) = if home { (0, 1) } else { (1, 0) };
        (goals[a], goals[other])
    });
    let wins = results.iter().filter(|(a, b)| a > b).count();
    let draws = results.iter().filter(|(a, b)| a == b).count();
    let losses = results.len() - wins - draws;
    eprintln!("stronger team: won {wins}, drew {draws}, lost {losses} of 200");
    let won = units(results.iter().map(|(a, b)| f64::from(u8::from(a > b))));
    if let Err(e) = judge_band(&limits(), "stronger_team_win_rate", &won) {
        panic!("{e}");
    }
}
