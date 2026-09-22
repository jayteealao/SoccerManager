//! AC-8: with identical squads, a team set to attack takes more shots than the same team set
//! to defend. Team A (default-a, at home) plays default-b 200 times defensive and 200 times
//! attacking; its in-match AI changes are off, so the mentality stays as set.
//!
//! Slow: `cargo test --release -p engine -- --ignored`.

mod common;

use common::{content, default_teams, run_many};
use engine::record::NullSink;
use engine::{Manager, MatchConfig, Simulation, Tactics};

fn mean_shots(mentality: &str) -> f64 {
    let content = content();
    let [a, b] = default_teams(&content);
    let schema = &content.tactics;
    let m = schema
        .mentality_index(mentality)
        .expect("the tactics file names the mentality") as u8;
    let shots = run_many(1..=200, |seed| {
        let mut tactics = Tactics::defaults(schema);
        tactics.mentality = m;
        let config = MatchConfig::new(seed, 90, &content, [&a, &b])
            .unwrap()
            .with_manager(0, Manager::Human)
            .with_tactics(0, tactics);
        let mut sim = Simulation::new(config).unwrap();
        sim.run(&mut NullSink).unwrap();
        f64::from(sim.summary().shots[0])
    });
    shots.iter().sum::<f64>() / shots.len() as f64
}

#[test]
#[ignore = "slow: cargo test --release -p engine -- --ignored"]
fn an_attacking_team_shoots_more_than_a_defensive_one() {
    let defensive = mean_shots("defensive");
    let attacking = mean_shots("attacking");
    eprintln!("mean shots per match: defensive {defensive:.2}, attacking {attacking:.2}");
    assert!(
        attacking > defensive,
        "attacking {attacking:.2} against defensive {defensive:.2}"
    );
}
