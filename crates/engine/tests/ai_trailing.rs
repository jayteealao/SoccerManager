//! AC-7: an AI-managed team that trails after minute 70 makes an attacking change. Over 100
//! seeded scenes that start at minute 70 with the home side one goal down, an applied
//! tactics change for the home side appears in at least 90.
//!
//! Slow: `cargo test --release -p engine -- --ignored`.

mod common;

use common::{content, default_teams, run_many};
use engine::record::NullSink;
use engine::scenario::Scene;
use engine::{ChangeKind, EngineEventKind, EventDetail, MatchConfig};

#[test]
#[ignore = "slow: cargo test --release -p engine -- --ignored"]
fn a_trailing_ai_team_changes_its_tactics() {
    let content = content();
    let [a, b] = default_teams(&content);
    let changed = run_many(1..=100, |seed| {
        let config = MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
        let mut sim = Scene::new(config).at_minute(70).score([0, 1]).build();
        sim.run(&mut NullSink).unwrap();
        sim.take_events().iter().any(|e| {
            e.kind == EngineEventKind::ChangeApplied
                && e.team == Some(0)
                && matches!(
                    e.detail,
                    Some(EventDetail::Change {
                        kind: ChangeKind::Tactics,
                        ..
                    })
                )
        })
    });
    let n = changed.iter().filter(|&&c| c).count();
    eprintln!("an applied tactics change in {n} of 100 scenes");
    assert!(
        n >= 90,
        "an applied tactics change in only {n} of 100 scenes"
    );
}
