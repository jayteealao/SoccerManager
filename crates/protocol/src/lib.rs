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
pub use message::{ClientCommand, Hello, QueueChange, ServerMessage, SetSpeed, Stats, TeamRef};

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
pub const PROTOCOL_VERSION: u16 = 2;

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
        ],
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
        fields: &["change.kind", "detail"],
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
            ServerMessage::Ack(_) => "ack",
            ServerMessage::Reject(_) => "reject",
        }
    }

    fn client_name(c: &ClientCommand) -> &'static str {
        match c {
            ClientCommand::Start => "start",
            ClientCommand::Pause => "pause",
            ClientCommand::SetSpeed(_) => "set-speed",
            ClientCommand::QueueChange(_) => "queue-change",
        }
    }

    fn blank_team() -> TeamRef {
        TeamRef {
            id: String::new(),
            name: String::new(),
            kit_primary: String::new(),
            kit_secondary: String::new(),
        }
    }

    fn every_server_message() -> Vec<ServerMessage> {
        vec![
            ServerMessage::Hello(Hello {
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
            }),
            ServerMessage::Event(MatchEvent::play(
                "",
                "",
                0,
                EventType::KickOff,
                None,
                [0, 0],
            )),
            ServerMessage::Stats(Stats {
                tick: 0,
                minute: 0,
                home_score: 0,
                away_score: 0,
                possession_changes: 0,
                ball_max_speed: 0.0,
                ball_idle_ticks: 0,
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
