//! The message set. Every control message is one JSON text frame tagged by `type`; tick
//! positions travel as binary frames instead (see `frame.rs`). Every payload refuses an
//! unknown field, so a viewer built against a later protocol cannot be misread as this one.

use serde::{Deserialize, Serialize};

use crate::command::{Ack, Reject};
use crate::event::MatchEvent;

/// One club, as the hello names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamRef {
    #[serde(rename = "team.id")]
    pub id: String,
    #[serde(rename = "team.name")]
    pub name: String,
}

/// The first message on every connection, before any tick frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    #[serde(rename = "protocol.version")]
    pub protocol_version: u16,
    #[serde(rename = "engine.version")]
    pub engine_version: String,
    #[serde(rename = "build.hash")]
    pub build_hash: String,
    #[serde(rename = "owner.id")]
    pub owner_id: String,
    #[serde(rename = "match.id")]
    pub match_id: String,
    pub seed: u64,
    pub dt_ms: f64,
    pub ticks_expected: u32,
    pub keyframe_interval: u32,
    /// The two clubs, home first.
    pub teams: [TeamRef; 2],
}

/// The running totals of a match, sent at full time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub tick: u32,
    pub minute: u32,
    #[serde(rename = "home.score")]
    pub home_score: u32,
    #[serde(rename = "away.score")]
    pub away_score: u32,
    #[serde(rename = "possession.changes")]
    pub possession_changes: u32,
    #[serde(rename = "ball.max_speed")]
    pub ball_max_speed: f64,
    #[serde(rename = "ball.idle_ticks")]
    pub ball_idle_ticks: u32,
}

/// Everything the server sends as a JSON text frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerMessage {
    Hello(Hello),
    Event(MatchEvent),
    Stats(Stats),
    Ack(Ack),
    Reject(Reject),
}

/// The playback speed a client asks for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetSpeed {
    pub speed: f32,
}

/// A change a client queues for a later stoppage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueueChange {
    /// `tactics` or `substitution`. Any other value is refused by name.
    #[serde(rename = "change.kind")]
    pub kind: String,
    /// Opaque here; the match-rules slice reads it when it applies the change.
    #[serde(default)]
    pub detail: serde_json::Value,
}

/// Everything a client sends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ClientCommand {
    Start,
    Pause,
    SetSpeed(SetSpeed),
    QueueChange(QueueChange),
}

impl ClientCommand {
    /// The command name as the acknowledgement echoes it.
    pub fn name(&self) -> &'static str {
        match self {
            ClientCommand::Start => "start",
            ClientCommand::Pause => "pause",
            ClientCommand::SetSpeed(_) => "set-speed",
            ClientCommand::QueueChange(_) => "queue-change",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::ChangeState;
    use crate::event::EventType;

    fn hello() -> Hello {
        Hello {
            protocol_version: crate::PROTOCOL_VERSION,
            engine_version: "0.1.0".into(),
            build_hash: "abc1234".into(),
            owner_id: "0123456789abcdef0123456789abcdef".into(),
            match_id: "000000000000002a-1700000000000".into(),
            seed: 42,
            dt_ms: 20.0,
            ticks_expected: 270_000,
            keyframe_interval: 50,
            teams: [
                TeamRef {
                    id: "club-a".into(),
                    name: "A".into(),
                },
                TeamRef {
                    id: "club-b".into(),
                    name: "B".into(),
                },
            ],
        }
    }

    #[test]
    fn every_server_message_round_trips_through_json() {
        let messages = [
            ServerMessage::Hello(hello()),
            ServerMessage::Event(MatchEvent::play(
                "owner",
                "match",
                1,
                EventType::KickOff,
                None,
                [0, 0],
            )),
            ServerMessage::Stats(Stats {
                tick: 270_000,
                minute: 90,
                home_score: 2,
                away_score: 1,
                possession_changes: 40,
                ball_max_speed: 27.5,
                ball_idle_ticks: 100,
            }),
            ServerMessage::Ack(Ack {
                command: "queue-change".into(),
                queue_id: Some("q-1-0".into()),
                queued_tick: 1,
                state: Some(ChangeState::Queued),
                speed: None,
            }),
            ServerMessage::Reject(Reject {
                command: "queue-change".into(),
                reason: "unknown change type formation".into(),
            }),
        ];
        for m in messages {
            let json = serde_json::to_string(&m).unwrap();
            assert_eq!(serde_json::from_str::<ServerMessage>(&json).unwrap(), m);
        }
    }

    #[test]
    fn every_client_command_round_trips_through_json() {
        let commands = [
            ClientCommand::Start,
            ClientCommand::Pause,
            ClientCommand::SetSpeed(SetSpeed { speed: 8.0 }),
            ClientCommand::QueueChange(QueueChange {
                kind: "tactics".into(),
                detail: serde_json::Value::Null,
            }),
        ];
        for c in commands {
            let json = serde_json::to_string(&c).unwrap();
            assert_eq!(serde_json::from_str::<ClientCommand>(&json).unwrap(), c);
        }
        assert_eq!(
            serde_json::to_string(&ClientCommand::Start).unwrap(),
            "{\"type\":\"start\"}"
        );
    }

    #[test]
    fn an_unknown_field_is_refused() {
        let err = serde_json::from_str::<ClientCommand>(
            "{\"type\":\"set-speed\",\"speed\":2.0,\"turbo\":true}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("turbo"), "{err}");
        let err = serde_json::from_str::<ServerMessage>(
            "{\"type\":\"reject\",\"command\":\"start\",\"reason\":\"no\",\"extra\":1}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("extra"), "{err}");
    }

    #[test]
    fn an_unknown_command_type_is_refused() {
        let err = serde_json::from_str::<ClientCommand>("{\"type\":\"teleport\"}").unwrap_err();
        assert!(err.to_string().contains("teleport"), "{err}");
    }
}
