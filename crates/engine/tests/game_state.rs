//! A side's game-state rules and player instructions through a whole match: the rules fire
//! and their changes are logged; a match saved while the players are still taking a rule
//! change up resumes tick for tick, the rules, the instructions, the change being taken up
//! and the run of play all coming back from the snapshot.

mod common;

use common::{content, default_teams};
use engine::data::game_state;
use engine::record::RECORD_BYTES;
use engine::tactics::rules::GameState;
use engine::{
    EngineError, MatchConfig, Simulation, Snapshot, Stoppage, TickRecord, TickSink, VecSink,
};

const OWNER: [u8; 16] = [9; 16];

/// Rules that fire through the second half, one on the run of play, and a striker told to
/// run in behind.
const TACTIC: &str = r#"{
  "rules": [
    { "when": [{ "kind": "time_left", "minutes": 40 }], "set": [{ "primitive": "tempo", "level": "fast" }, { "primitive": "pressing", "level": "high" }] },
    { "when": [{ "kind": "time_left", "minutes": 25 }], "set": [{ "formation": "4-3-3" }] },
    { "when": [{ "kind": "run_of_play", "above": 0.2 }], "set": [{ "primitive": "mentality", "level": "attacking" }] }
  ],
  "instructions": [{ "slot": 10, "primitive": "forward_runs", "level": "run_in_behind" }]
}"#;

fn state(config: &MatchConfig) -> GameState {
    let file = game_state::parse(TACTIC.as_bytes()).unwrap();
    let formation = config.teams[0].tactics.formation;
    game_state::resolve(&file, &config.tactics, &[formation]).unwrap()
}

fn config() -> MatchConfig {
    let content = content();
    let [a, b] = default_teams(&content);
    // A person's side plays the default tactics, which every rule here moves.
    let mut config = MatchConfig::new(3, 90, &content, [&a, &b])
        .unwrap()
        .with_manager(0, engine::Manager::Human);
    // A slow take-up, so that a stoppage comes while the players are taking a change up.
    config.tactics.game_state.as_mut().unwrap().take_up_s = 600.0;
    let state = state(&config);
    config.with_game_state(0, state)
}

/// Keeps every record and captures the first snapshot taken while the home side is taking a
/// rule change up.
#[derive(Default)]
struct Capture {
    records: Vec<TickRecord>,
    snapshot: Option<Snapshot>,
}

impl TickSink for Capture {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.records.push(*record);
        Ok(())
    }

    fn on_stoppage(&mut self, _stoppage: &Stoppage, sim: &Simulation) -> Result<(), EngineError> {
        if self.snapshot.is_none() && sim.teams()[0].take_up.is_some() {
            self.snapshot = Some(Snapshot::capture(sim, OWNER, 1_700_000_000_000));
        }
        Ok(())
    }
}

fn bytes(records: &[TickRecord]) -> Vec<u8> {
    let mut out = Vec::with_capacity(records.len() * RECORD_BYTES);
    let mut buf = [0u8; RECORD_BYTES];
    for record in records {
        record.write_to(&mut buf);
        out.extend_from_slice(&buf);
    }
    out
}

/// G11: the match saved during a take-up resumes tick for tick, and the rules fired in it.
#[test]
fn a_match_saved_during_a_take_up_resumes_tick_for_tick() {
    let mut capture = Capture::default();
    let mut whole = Simulation::new(config()).unwrap();
    whole.run(&mut capture).unwrap();
    let rule_changes = whole
        .applied_changes()
        .iter()
        .filter(|c| matches!(c.change, engine::Change::Rule { .. }) && c.team == 0)
        .count();
    assert!(rule_changes >= 2, "{rule_changes} rule changes applied");
    let snapshot = capture
        .snapshot
        .expect("a stoppage while a rule change is taken up");
    let from = snapshot.tick();

    let read = Snapshot::from_bytes(&snapshot.to_bytes(), "snapshot.smsn").unwrap();
    let mut resumed = Simulation::from_snapshot(config(), &read).unwrap();
    assert_eq!(resumed.tick(), from);
    let at = resumed.teams();
    assert!(at[0].take_up.is_some());
    assert!(at[0].rules_active != 0);
    assert_eq!(
        Snapshot::capture(&resumed, OWNER, 1_700_000_000_000).to_bytes(),
        snapshot.to_bytes(),
        "the snapshot written again differs"
    );
    let mut sink = VecSink::default();
    resumed.run(&mut sink).unwrap();
    let tail: Vec<TickRecord> = capture
        .records
        .iter()
        .copied()
        .filter(|r| r.tick > from)
        .collect();
    assert!(!tail.is_empty());
    assert_eq!(sink.records.len(), tail.len());
    assert!(
        bytes(&sink.records) == bytes(&tail),
        "the resumed records differ from the uninterrupted match"
    );
    assert_eq!(resumed.summary(), whole.summary());
    // The log of applied changes starts again at the resume; the changes after it match.
    let key = |c: &engine::AppliedChange| (c.team, c.id, c.tick, c.stoppage, c.change.clone());
    let after: Vec<_> = whole
        .applied_changes()
        .iter()
        .filter(|c| c.tick > from)
        .map(key)
        .collect();
    assert!(!after.is_empty());
    assert_eq!(
        resumed
            .applied_changes()
            .iter()
            .map(key)
            .collect::<Vec<_>>(),
        after
    );
    assert_eq!(resumed.teams()[0].tactics, whole.teams()[0].tactics);
}
