//! The tactics board's messages at the socket: a level change and a game-state change are
//! read by name and passed on as the engine's changes, and the pre-match lineup carries the
//! other primitives' levels and the rules; every bad value is refused by name before it takes
//! a queue identifier.

mod common;

use std::sync::{Arc, Mutex};

use engine::Change;
use engine::data::tactics::TacticsSchema;
use protocol::{
    ChangeKind, ClientCommand, LevelWire, Queue, QueueChange, ServerMessage, SetLineup,
};
use stream::EventWriter;
use stream::control::{CommandContext, Gate, Inbox, LineupRules, PreMatch};
use stream::session::MatchState;

const MATCH_ID: &str = "000000000000002a-9";

fn tactics() -> TacticsSchema {
    engine::Content::load(&common::content_dir())
        .expect("the shipped content loads")
        .tactics
}

fn rules() -> LineupRules {
    LineupRules {
        // Squad 0 and 11 are the goalkeepers, as in the shipped team files.
        keepers: (0..22).map(|i| i == 0 || i == 11).collect(),
        bench_size: 7,
        tactics: tactics(),
        formation: 0,
    }
}

/// A started session for the home side, standing in formation 0 (4-4-2).
fn session(dir: &std::path::Path, gate: Gate, pre_match: PreMatch) -> CommandContext {
    let state = MatchState::default();
    state.set_home_formation(0);
    CommandContext {
        owner_id: "0".repeat(32),
        match_id: MATCH_ID.into(),
        gate: Arc::new(gate),
        state: Arc::new(state),
        events: Arc::new(Mutex::new(EventWriter::open(dir, MATCH_ID).unwrap())),
        // A rule pack that admits no stoppage change: the board's kinds wait for none.
        queue: Queue::new(Vec::new()),
        pre_match: Arc::new(pre_match),
        inbox: Arc::new(Inbox::default()),
    }
}

fn queue(kind: &str, detail: serde_json::Value) -> String {
    serde_json::to_string(&ClientCommand::QueueChange(QueueChange {
        kind: kind.into(),
        detail,
    }))
    .unwrap()
}

fn index(schema: &TacticsSchema, name: &str) -> u8 {
    schema
        .primitives
        .iter()
        .position(|p| p.name == name)
        .unwrap() as u8
}

/// Sends `text` and returns the refusal's reason; panics on an acknowledgement.
fn refused(ctx: &mut CommandContext, text: &str) -> String {
    match ctx.handle(text).unwrap().0 {
        ServerMessage::Reject(r) => r.reason,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_level_change_and_a_game_state_change_reach_the_engine_by_name() {
    let dir = common::temp_dir("board-messages-ok");
    let schema = tactics();
    let mut ctx = session(&dir, Gate::new(), PreMatch::new(rules()));
    let (answer, row) = ctx
        .handle(&queue(
            "instructions",
            serde_json::json!({"levels": [
                {"primitive": "tempo", "level": "very_fast"},
                {"primitive": "cross_type", "level": "floated"},
                {"primitive": "offside_trap", "level": "on"}
            ]}),
        ))
        .unwrap();
    let ServerMessage::Ack(ack) = answer else {
        panic!("a level change is admitted: {answer:?}");
    };
    assert!(ack.queue_id.is_some());
    assert_eq!(row.unwrap().change_kind, Some(ChangeKind::Instructions));
    let state = serde_json::json!({
        "rules": [{
            "when": [{"kind": "score", "state": "trailing", "by": 1}, {"kind": "time_left", "minutes": 15}],
            "set": [{"primitive": "mentality", "level": "attacking"}]
        }],
        "instructions": [{"slot": 9, "primitive": "forward_runs", "level": "run_in_behind"}]
    });
    let (answer, _) = ctx
        .handle(&queue(
            "game-state",
            serde_json::json!({"game_state": state}),
        ))
        .unwrap();
    assert!(matches!(answer, ServerMessage::Ack(_)), "{answer:?}");
    let admitted = ctx.inbox.drain();
    assert_eq!(admitted.len(), 2);
    assert_eq!(
        admitted[0].change,
        Change::Instructions(vec![
            (index(&schema, "tempo"), 0),
            (index(&schema, "cross_type"), 2),
            (index(&schema, "offside_trap"), 1),
        ])
    );
    let Change::GameState(read) = &admitted[1].change else {
        panic!("a game-state change: {:?}", admitted[1].change);
    };
    assert_eq!(read.rules.len(), 1);
    assert_eq!(read.rules[0].when.len(), 2);
    assert_eq!(read.instructions.len(), 1);
    assert_eq!(read.instructions[0].slot, 9);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_bad_value_is_refused_by_name_and_takes_no_identifier() {
    let dir = common::temp_dir("board-messages-bad");
    let mut ctx = session(&dir, Gate::new(), PreMatch::new(rules()));
    let level = |primitive: &str, level: &str| {
        queue(
            "instructions",
            serde_json::json!({"levels": [{"primitive": primitive, "level": level}]}),
        )
    };
    let state =
        |state: serde_json::Value| queue("game-state", serde_json::json!({"game_state": state}));
    let rule = serde_json::json!({
        "when": [{"kind": "time_left", "minutes": 10}],
        "set": [{"primitive": "tempo", "level": "fast"}]
    });
    let cases: Vec<(String, &str)> = vec![
        (level("flair", "high"), "flair is not a primitive"),
        (level("tempo", "warp"), "warp is not a level of tempo"),
        (
            level("cross_type", "lobbed"),
            "lobbed is not a level of cross_type",
        ),
        (
            queue("instructions", serde_json::json!({"levels": []})),
            "a level change names at least one primitive",
        ),
        (
            state(
                serde_json::json!({"instructions": [{"slot": 11, "primitive": "dribbling", "level": "dribble_often"}]}),
            ),
            "slot 11 is not a slot",
        ),
        (
            state(
                serde_json::json!({"rules": [{"when": [{"kind": "weather", "below": 1}], "set": [{"primitive": "tempo", "level": "fast"}]}]}),
            ),
            "weather is not a condition kind",
        ),
        (
            state(serde_json::json!({"rules": vec![rule.clone(); 7]})),
            "7 rules; a tactic holds at most 6",
        ),
        (
            state(
                serde_json::json!({"instructions": [{"slot": 2, "primitive": "offside_trap", "level": "on"}]}),
            ),
            "offside_trap is not on the player override list",
        ),
        // Slot 2 is a centre-back in 4-4-2: forward runs reach only wingers and strikers.
        (
            state(
                serde_json::json!({"instructions": [{"slot": 2, "primitive": "forward_runs", "level": "push_up"}]}),
            ),
            "forward_runs reaches only LW, RW, ST; slot 2 is CB in 4-4-2",
        ),
        (
            state(
                serde_json::json!({"rules": [{"when": [{"kind": "time_left", "minutes": 90}], "set": [{"primitive": "tempo", "level": "fast"}]}]}),
            ),
            "90 minutes; allowed 1 to 45",
        ),
        (
            queue(
                "game-state",
                serde_json::json!({"game_state": {"plans": []}}),
            ),
            "cannot read the game state",
        ),
    ];
    for (text, want) in &cases {
        let reason = refused(&mut ctx, text);
        assert!(reason.contains(want), "{want:?} not in {reason:?}");
    }
    assert!(
        ctx.inbox.drain().is_empty(),
        "no refused change reaches the engine"
    );
    // A session with no tactics file to read names by refuses both kinds.
    let mut bare = session(&dir, Gate::new(), PreMatch::none());
    let reason = refused(&mut bare, &level("tempo", "fast"));
    assert_eq!(reason, "this session reads no level or rule by name");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_pre_match_lineup_carries_levels_and_rules() {
    let dir = common::temp_dir("board-messages-lineup");
    let schema = tactics();
    let mut ctx = session(&dir, Gate::held(), PreMatch::new(rules()));
    let lineup = |levels: Vec<LevelWire>, game_state: Option<serde_json::Value>| {
        serde_json::to_string(&ClientCommand::SetLineup(SetLineup {
            lineup: (0..11).collect(),
            bench: vec![11, 12],
            patch: None,
            levels,
            game_state,
        }))
        .unwrap()
    };
    let crossing = vec![LevelWire {
        primitive: "crossing".into(),
        level: "cross_often".into(),
    }];
    let reason = refused(
        &mut ctx,
        &lineup(
            vec![LevelWire {
                primitive: "crossing".into(),
                level: "always".into(),
            }],
            None,
        ),
    );
    assert!(
        reason.contains("always is not a level of crossing"),
        "{reason}"
    );
    let reason = refused(
        &mut ctx,
        &lineup(
            Vec::new(),
            Some(
                serde_json::json!({"instructions": [{"slot": 1, "primitive": "forward_runs", "level": "push_up"}]}),
            ),
        ),
    );
    assert!(reason.contains("slot 1 is LB in 4-4-2"), "{reason}");
    let (answer, _) = ctx
        .handle(&lineup(
            crossing,
            Some(serde_json::json!({"instructions": [{"slot": 10, "primitive": "forward_runs", "level": "push_up"}]})),
        ))
        .unwrap();
    assert!(matches!(answer, ServerMessage::Ack(_)), "{answer:?}");
    let setup = ctx.pre_match.take().expect("the lineup is stored");
    assert_eq!(setup.levels, vec![(index(&schema, "crossing"), 4)]);
    let state = setup.game_state.expect("the rules are stored");
    assert_eq!(state.instructions.len(), 1);
    assert_eq!(state.instructions[0].slot, 10);
    let _ = std::fs::remove_dir_all(&dir);
}
