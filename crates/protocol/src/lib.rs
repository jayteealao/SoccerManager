//! The wire protocol between the engine and a viewer page.
//!
//! This crate holds the message set, the tick codec, and the change queue, and it performs
//! no input and no output. The viewer decoder and a later WebAssembly build mirror it; the
//! `stream` crate adds the socket, the recorder, and the replayer.

pub mod codec;
pub mod command;
pub mod event;
pub mod frame;
pub mod message;

use thiserror::Error;

pub use codec::{DEFAULT_KEYFRAME_INTERVAL, DELTA_BYTES, KEYFRAME_BYTES, Quantised};
pub use command::{Ack, ChangeKind, ChangeState, Pending, Queue, Reject, Verdict};
pub use event::{CardKind, ChangeOutcome, EventType, MatchEvent};
pub use frame::{Frame, TickFrame};
pub use message::{
    Advice, AdvicePick, CancelChange, ChangeDetail, ChangeStateNote, ClientCommand, Condition,
    DEFAULT_GROUND_LENGTH, DEFAULT_GROUND_WIDTH, GroundEvent, GroundKind, GroundProgress, Hello,
    Jump, Matchday, MatchdayFixture, PatchWire, QueueChange, RoleWire, RosterEntry, Seen,
    ServerMessage, SetLineup, SetSpeed, Side, SlotRole, SquadEntry, Stats, SubstitutionRules,
    TeamRef, TeamSetup,
};

/// The protocol version a client must ask for. A client that asks for another version is
/// refused at the handshake, with both versions named.
///
/// Version 1 survived the two kit-colour fields added to `TeamRef`. The judgement was made,
/// not missed: no message was removed, no field changed meaning, and both producers in
/// existence were updated in the same commit. A JavaScript client ignores a field it does
/// not know, and `deny_unknown_fields` reaches only a Rust client built from this same
/// commit.
///
/// Version 2: `ticks_expected` changed meaning from the exact tick count to the most ticks
/// the match can last, because added time makes the real length known only at full time.
/// A field that changes meaning takes a new version. The event message also gained the law
/// event types and six optional fields.
///
/// Version 2 also survived three event types (`injury`, `substitution`, `ai-decision`) and two
/// optional fields (`change.applied_tick`, `ai.decision`) that the observability contract
/// already lists, and `team.id` on change events. No field changed meaning, and a client
/// ignores what it does not know.
///
/// Version 2 also survived the optional `commentary` line on play events, under the same
/// judgement: no field was removed, none changed meaning (`player.id` now also names the
/// scorer and the restart taker, values the field already allowed), and both producers in
/// existence changed in the same commit.
///
/// Version 3: `stats` changed meaning from one closing total to a running total sent every
/// simulated second (and again at full time), and gained nine panel fields. A new message,
/// `condition`, carries every player's energy on the same cadence, and each hello team gained
/// a `roster`. A viewer built for version 2 would read the first `stats` message as the final
/// score sheet, so the change takes a new version.
///
/// Version 3 also carries the pre-match lineup and live changes, under the same single raise:
/// a `serve` session now sends only the hello until the page sends `start`; a new command,
/// `set-lineup`, picks the home lineup, bench, and pre-match tactics in that hold; the hello
/// gained the home team's `squad` and `setup`, the loaded `tactics` file, and the
/// `substitutions` limits; and `queue-change` changed meaning, because its `detail` is now
/// read and the change reaches the engine.
///
/// Version 3 also survived knockout matches: five optional event fields (`period`,
/// `shootout.round`, `shootout.scored`, `shootout.scores`, `result.decided_by`) and no new
/// event type. For a knockout match `ticks_expected` also covers extra time and the rule
/// pack's allowance of shoot-out rounds, and a sudden death past the allowance runs longer;
/// clients already grow their history past `ticks_expected` for added time. A match that is
/// not a knockout match sends exactly what it sent before, so no field changed meaning for it.
///
/// Version 3 also survived script packs: one event type (`script`) and four optional event
/// fields (`script.pack`, `script.hook`, `script.outcome`, `script.detail`). A match without
/// a pack sends exactly what it sent before, no field changed meaning, the page ignores an
/// event type it does not list, and both producers changed in the same commit.
///
/// Version 3 also survived withdrawing a queued change: one command (`cancel-change`), one
/// message (`change-state`, the "applies now" word for a change the opening stoppage takes),
/// and optional fields (`player.injury_resistance` on a squad entry, the extra-time and
/// window-exempt substitution rules on the hello, and each team's substitutions and windows
/// used on `condition`). No field was removed or changed meaning, a client ignores a message
/// type or field it does not know, and a client that never sends `cancel-change` gets exactly
/// the answers it got before.
///
/// Version 3 also survived the assistant's advice: one message (`advice`, the computer
/// manager's picks for the page's team, sent only to a page that manages a team), and two
/// optional hello fields (`formation` on each team and `knockout`). No field was removed or
/// changed meaning, a client ignores a message type or field it does not know, and the match
/// itself is unchanged: the advice reads the match and writes nothing.
///
/// Version 3 also survived skipping to the result: one command (`skip`, no fields), answered
/// with the existing `ack` or `reject`. The match it plays on is the same match, so every
/// tick and message after it is what the match played through would send, and a client that
/// never sends it gets exactly the answers it got before.
///
/// Version 3 also survived the test-only jump: one command (`jump`, one `tick` field),
/// answered with the existing `ack` or `reject` and refused unless the engine was started
/// with `--test-jump`. Like a skip it changes only when ticks are sent, never the match, and
/// a client that never sends it gets exactly the answers it got before.
pub const PROTOCOL_VERSION: u16 = 3;

/// Errors this crate returns.
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// A binary frame carried no bytes.
    #[error("empty binary frame")]
    EmptyFrame,
    /// The first byte of a binary frame named no known frame kind.
    #[error("unknown frame kind {0:#04x}")]
    UnknownFrameKind(u8),
    /// A binary frame was the wrong length for its kind.
    #[error("frame kind {kind:#04x}: expected {expected} bytes, found {found}")]
    FrameLength {
        kind: u8,
        expected: usize,
        found: usize,
    },
    /// A delta frame arrived before any keyframe.
    #[error("delta frame before the first keyframe")]
    DeltaWithoutKeyframe,
    /// A decoded delta left the pitch coordinate range.
    #[error("delta at tick {tick} leaves the coordinate range")]
    DeltaOutOfRange { tick: u32 },
    /// A JSON text frame could not be read as a message.
    #[error("cannot read the message")]
    Json {
        #[source]
        source: serde_json::Error,
    },
    /// The client asked for a protocol version this build does not speak.
    #[error("protocol version {found}; this build speaks {PROTOCOL_VERSION}")]
    UnsupportedVersion { found: u16 },
}

/// Which way a message travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    ServerToClient,
    ClientToServer,
}

/// How a message is framed on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// One JSON object in a text frame, tagged by `type`.
    JsonText,
    /// A binary frame: a one-byte kind tag and a fixed payload.
    Binary,
}

/// One message of the protocol, as the reference document must describe it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageSpec {
    pub name: &'static str,
    pub direction: Direction,
    pub encoding: Encoding,
    pub fields: &'static [&'static str],
}

/// Every message of the protocol. `tests/document.rs` holds the reference document to this
/// list, and the completeness test below holds this list to the two message enumerations.
pub const MESSAGES: &[MessageSpec] = &[
    MessageSpec {
        name: "hello",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &[
            "protocol.version",
            "engine.version",
            "build.hash",
            "owner.id",
            "match.id",
            "seed",
            "dt_ms",
            "ticks_expected",
            "keyframe_interval",
            "teams",
            "team.id",
            "team.name",
            "team.kit.primary",
            "team.kit.secondary",
            "roster",
            "player.id",
            "player.name",
            "player.shirt",
            "player.position",
            "player.squad_index",
            "squad",
            "player.natural_fitness",
            "role_fit",
            "setup",
            "lineup",
            "bench",
            "formation",
            "mentality",
            "instructions",
            "roles",
            "role",
            "duty",
            "tactics",
            "substitutions",
            "limit",
            "windows",
            "extra_substitutions",
            "extra_windows",
            "windows_exempt",
            "player.injury_resistance",
            "knockout",
            "ground.length",
            "ground.width",
        ],
    },
    MessageSpec {
        name: "tick",
        direction: Direction::ServerToClient,
        encoding: Encoding::Binary,
        fields: &["kind", "tick", "ball", "players"],
    },
    MessageSpec {
        name: "event",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &[
            "owner.id",
            "match.id",
            "tick",
            "minute",
            "event.type",
            "team.id",
            "home.score",
            "away.score",
            "change.kind",
            "change.queued_tick",
            "change.queue_id",
            "change.rejected_reason",
            "change.state",
            "change.applied_tick",
            "ai.decision",
            "player.id",
            "player.secondary_id",
            "card.kind",
            "foul.advantage",
            "minute.added",
            "added_time.s",
            "commentary",
            "period",
            "shootout.round",
            "shootout.scored",
            "shootout.scores",
            "result.decided_by",
            "script.pack",
            "script.hook",
            "script.outcome",
            "script.detail",
        ],
    },
    MessageSpec {
        name: "stats",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &[
            "tick",
            "minute",
            "home.score",
            "away.score",
            "possession.changes",
            "ball.max_speed",
            "ball.idle_ticks",
            "stats.possession_pct",
            "stats.shots",
            "stats.shots_on_target",
            "stats.xg",
            "stats.passes",
            "stats.pass_accuracy_pct",
            "stats.fouls",
            "stats.corners",
            "stats.offsides",
        ],
    },
    MessageSpec {
        name: "condition",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &["tick", "energy", "subs_used", "windows_used"],
    },
    MessageSpec {
        name: "change-state",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &["change.queue_id", "state", "tick"],
    },
    MessageSpec {
        name: "advice",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &[
            "tick", "minute", "picks", "code", "kind", "off", "on", "patch",
        ],
    },
    MessageSpec {
        name: "matchday",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &[
            "round",
            "fixtures",
            "fixture",
            "home",
            "away",
            "team.id",
            "team.name",
            "team.kit.primary",
            "team.kit.secondary",
        ],
    },
    MessageSpec {
        name: "ground-event",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &[
            "fixture", "tick", "kind", "minute", "added", "side", "scorer", "score", "late",
        ],
    },
    MessageSpec {
        name: "ground-progress",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &["tick", "reached"],
    },
    MessageSpec {
        name: "ack",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &[
            "command",
            "change.queue_id",
            "change.queued_tick",
            "state",
            "speed",
        ],
    },
    MessageSpec {
        name: "reject",
        direction: Direction::ServerToClient,
        encoding: Encoding::JsonText,
        fields: &["command", "reason"],
    },
    MessageSpec {
        name: "start",
        direction: Direction::ClientToServer,
        encoding: Encoding::JsonText,
        fields: &[],
    },
    MessageSpec {
        name: "pause",
        direction: Direction::ClientToServer,
        encoding: Encoding::JsonText,
        fields: &[],
    },
    MessageSpec {
        name: "set-speed",
        direction: Direction::ClientToServer,
        encoding: Encoding::JsonText,
        fields: &["speed"],
    },
    MessageSpec {
        name: "queue-change",
        direction: Direction::ClientToServer,
        encoding: Encoding::JsonText,
        fields: &[
            "change.kind",
            "detail",
            "patch",
            "off",
            "on",
            "formation",
            "mentality",
            "instructions",
            "roles",
            "squad",
            "role",
            "duty",
        ],
    },
    MessageSpec {
        name: "set-lineup",
        direction: Direction::ClientToServer,
        encoding: Encoding::JsonText,
        fields: &["lineup", "bench", "patch"],
    },
    MessageSpec {
        name: "seen",
        direction: Direction::ClientToServer,
        encoding: Encoding::JsonText,
        fields: &["tick"],
    },
    MessageSpec {
        name: "cancel-change",
        direction: Direction::ClientToServer,
        encoding: Encoding::JsonText,
        fields: &["change.queue_id"],
    },
    MessageSpec {
        name: "skip",
        direction: Direction::ClientToServer,
        encoding: Encoding::JsonText,
        fields: &[],
    },
    MessageSpec {
        name: "jump",
        direction: Direction::ClientToServer,
        encoding: Encoding::JsonText,
        fields: &["tick"],
    },
];

/// The specification of `name`, or `None`.
pub fn message_spec(name: &str) -> Option<&'static MessageSpec> {
    MESSAGES.iter().find(|m| m.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventType;

    /// The name `MESSAGES` must carry for each variant. The exhaustive match means a new
    /// variant fails to compile until this test and `MESSAGES` both name it.
    fn server_name(m: &ServerMessage) -> &'static str {
        match m {
            ServerMessage::Hello(_) => "hello",
            ServerMessage::Event(_) => "event",
            ServerMessage::Stats(_) => "stats",
            ServerMessage::Condition(_) => "condition",
            ServerMessage::Ack(_) => "ack",
            ServerMessage::Reject(_) => "reject",
            ServerMessage::ChangeState(_) => "change-state",
            ServerMessage::Advice(_) => "advice",
            ServerMessage::Matchday(_) => "matchday",
            ServerMessage::GroundEvent(_) => "ground-event",
            ServerMessage::GroundProgress(_) => "ground-progress",
        }
    }

    fn client_name(c: &ClientCommand) -> &'static str {
        match c {
            ClientCommand::Start => "start",
            ClientCommand::Pause => "pause",
            ClientCommand::SetSpeed(_) => "set-speed",
            ClientCommand::QueueChange(_) => "queue-change",
            ClientCommand::SetLineup(_) => "set-lineup",
            ClientCommand::Seen(_) => "seen",
            ClientCommand::CancelChange(_) => "cancel-change",
            ClientCommand::Skip => "skip",
            ClientCommand::Jump(_) => "jump",
        }
    }

    fn blank_team() -> TeamRef {
        TeamRef {
            id: String::new(),
            name: String::new(),
            kit_primary: String::new(),
            kit_secondary: String::new(),
            roster: Vec::new(),
            squad: Vec::new(),
            setup: None,
            formation: String::new(),
        }
    }

    fn every_server_message() -> Vec<ServerMessage> {
        vec![
            ServerMessage::Hello(Box::new(Hello {
                protocol_version: PROTOCOL_VERSION,
                engine_version: String::new(),
                build_hash: String::new(),
                owner_id: String::new(),
                match_id: String::new(),
                seed: 0,
                dt_ms: 20.0,
                ticks_expected: 0,
                keyframe_interval: DEFAULT_KEYFRAME_INTERVAL,
                teams: [blank_team(), blank_team()],
                tactics: serde_json::Value::Null,
                substitutions: SubstitutionRules::default(),
                knockout: false,
                ground_length: DEFAULT_GROUND_LENGTH,
                ground_width: DEFAULT_GROUND_WIDTH,
            })),
            ServerMessage::Event(Box::new(MatchEvent::play(
                "",
                "",
                0,
                EventType::KickOff,
                None,
                [0, 0],
            ))),
            ServerMessage::Stats(Stats {
                tick: 0,
                minute: 0,
                home_score: 0,
                away_score: 0,
                possession_changes: 0,
                ball_max_speed: 0.0,
                ball_idle_ticks: 0,
                possession_pct: [0.0; 2],
                shots: [0; 2],
                shots_on_target: [0; 2],
                xg: [0.0; 2],
                passes: [0; 2],
                pass_accuracy_pct: [0.0; 2],
                fouls: [0; 2],
                corners: [0; 2],
                offsides: [0; 2],
            }),
            ServerMessage::Condition(Condition {
                tick: 0,
                energy: Vec::new(),
                subs_used: [0; 2],
                windows_used: [0; 2],
            }),
            ServerMessage::Ack(Ack {
                command: String::new(),
                queue_id: None,
                queued_tick: 0,
                state: None,
                speed: None,
            }),
            ServerMessage::Reject(Reject {
                command: String::new(),
                reason: String::new(),
            }),
            ServerMessage::ChangeState(ChangeStateNote {
                queue_id: String::new(),
                state: ChangeState::AppliesNow,
                tick: 0,
            }),
            ServerMessage::Advice(Advice {
                tick: 0,
                minute: 0,
                picks: Vec::new(),
            }),
            ServerMessage::Matchday(Matchday {
                round: 1,
                fixtures: Vec::new(),
            }),
            ServerMessage::GroundEvent(GroundEvent {
                fixture: 0,
                tick: 0,
                kind: GroundKind::FullTime,
                minute: 0,
                added: None,
                side: None,
                scorer: None,
                score: [0, 0],
                late: false,
            }),
            ServerMessage::GroundProgress(GroundProgress {
                tick: 0,
                reached: Vec::new(),
            }),
        ]
    }

    fn every_client_command() -> Vec<ClientCommand> {
        vec![
            ClientCommand::Start,
            ClientCommand::Pause,
            ClientCommand::SetSpeed(SetSpeed { speed: 1.0 }),
            ClientCommand::QueueChange(QueueChange {
                kind: String::new(),
                detail: serde_json::Value::Null,
            }),
            ClientCommand::SetLineup(SetLineup {
                lineup: Vec::new(),
                bench: Vec::new(),
                patch: None,
            }),
            ClientCommand::Seen(Seen { tick: 0 }),
            ClientCommand::CancelChange(CancelChange {
                queue_id: String::new(),
            }),
            ClientCommand::Skip,
            ClientCommand::Jump(Jump { tick: 0 }),
        ]
    }

    #[test]
    fn messages_names_every_variant_and_nothing_else() {
        let mut expected: Vec<&'static str> = every_server_message()
            .iter()
            .map(server_name)
            .chain(every_client_command().iter().map(client_name))
            .collect();
        // The tick frame is binary, so it has no enumeration variant, but it is a message.
        expected.push("tick");
        expected.sort_unstable();
        let mut listed: Vec<&'static str> = MESSAGES.iter().map(|m| m.name).collect();
        listed.sort_unstable();
        assert_eq!(listed, expected);
    }

    #[test]
    fn every_client_command_name_matches_its_specification() {
        for c in every_client_command() {
            assert_eq!(c.name(), client_name(&c));
            assert!(message_spec(c.name()).is_some(), "{}", c.name());
        }
    }

    #[test]
    fn every_specified_field_is_a_non_empty_key() {
        for m in MESSAGES {
            for f in m.fields {
                assert!(!f.is_empty(), "{} carries an empty field name", m.name);
            }
        }
    }
}
