//! The message set. Every control message is one JSON text frame tagged by `type`; tick
//! positions travel as binary frames instead (see `frame.rs`). Every payload refuses an
//! unknown field, so a viewer built against a later protocol cannot be misread as this one.

use serde::{Deserialize, Serialize};

use crate::command::{Ack, Reject};
use crate::event::MatchEvent;

/// One club, as the hello names it. The two kit colours let a viewer draw the markers in the
/// club's own colours; both are lower-case `#rrggbb`, exactly as a team file writes them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamRef {
    #[serde(rename = "team.id")]
    pub id: String,
    #[serde(rename = "team.name")]
    pub name: String,
    #[serde(rename = "team.kit.primary")]
    pub kit_primary: String,
    #[serde(rename = "team.kit.secondary")]
    pub kit_secondary: String,
    /// The 11 starters in wire-slot order, then the named bench in bench order. A hello from
    /// an earlier build carries no roster and reads as an empty one.
    #[serde(default)]
    pub roster: Vec<RosterEntry>,
    /// The whole squad in file order, so an entry's place in the list is its squad index.
    /// Sent only by a session that waits for the page's lineup before kick-off; empty
    /// otherwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub squad: Vec<SquadEntry>,
    /// The lineup, bench, and tactics the computer manager picked before kick-off. Present
    /// exactly when `squad` is: the page starts its lineup editor from it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup: Option<TeamSetup>,
}

/// One squad player, as the lineup editor shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SquadEntry {
    #[serde(rename = "player.id")]
    pub id: String,
    #[serde(rename = "player.name")]
    pub name: String,
    #[serde(rename = "player.shirt")]
    pub shirt: u8,
    /// The position code, such as `GK` or `CB`.
    #[serde(rename = "player.position")]
    pub position: String,
    /// The natural-fitness attribute, 0 to 100. Before kick-off every player is fresh, so
    /// this is the one fitness figure the engine holds.
    #[serde(rename = "player.natural_fitness")]
    pub natural_fitness: u8,
    /// How well the player fits each role, 0 to 100, one value per role in the order of the
    /// hello's `tactics.roles`.
    pub role_fit: Vec<u8>,
}

/// One slot's role and duty, as indices into the hello's `tactics.roles` and
/// `tactics.duties`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotRole {
    pub role: u8,
    pub duty: u8,
}

/// A team's pre-match setup. Every number is an index: squad indices for players, and
/// indices into the hello's `tactics` for the rest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamSetup {
    /// Eleven squad indices in slot order; slot 0 is the goalkeeper.
    pub lineup: Vec<u16>,
    pub bench: Vec<u16>,
    pub formation: u8,
    pub mentality: u8,
    /// One level per team instruction, in the order `tactics.instructions` names them:
    /// pressing, width, tempo, line height, passing directness, time wasting.
    pub instructions: Vec<u8>,
    /// One role and duty per slot, in slot order.
    pub roles: Vec<SlotRole>,
}

/// The substitution limits of the loaded rule pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubstitutionRules {
    /// Substitutions each team may make.
    pub limit: u8,
    /// Stoppages at which each team may make them; half-time uses none.
    pub windows: u8,
}

/// One player a hello names. `player.squad_index` is the player's place in the team file's
/// squad, the handle a lineup change uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterEntry {
    #[serde(rename = "player.id")]
    pub id: String,
    #[serde(rename = "player.name")]
    pub name: String,
    #[serde(rename = "player.shirt")]
    pub shirt: u8,
    /// The position code, such as `GK` or `CB`.
    #[serde(rename = "player.position")]
    pub position: String,
    #[serde(rename = "player.squad_index")]
    pub squad_index: u32,
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
    /// The most ticks the match can last: regulation time plus the cap on added time in every
    /// half. The match ends earlier when it earns less added time; the full-time event marks
    /// the real last tick.
    pub ticks_expected: u32,
    pub keyframe_interval: u32,
    /// The two clubs, home first.
    pub teams: [TeamRef; 2],
    /// The tactics file the engine loaded, as JSON: formations, mentalities, instructions
    /// and their levels, roles, and duties. Every tactics index on the wire points into it.
    #[serde(default)]
    pub tactics: serde_json::Value,
    #[serde(default)]
    pub substitutions: SubstitutionRules,
}

/// The running totals of a match, sent once every simulated second and again at full time.
/// Every pair is home first; the shares and expected goals are rounded exactly as the
/// `match-stats` record rounds them.
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
    /// Each team's share of the open-play ticks, one decimal.
    #[serde(rename = "stats.possession_pct")]
    pub possession_pct: [f64; 2],
    #[serde(rename = "stats.shots")]
    pub shots: [u32; 2],
    #[serde(rename = "stats.shots_on_target")]
    pub shots_on_target: [u32; 2],
    /// Expected goals, two decimals.
    #[serde(rename = "stats.xg")]
    pub xg: [f64; 2],
    #[serde(rename = "stats.passes")]
    pub passes: [u32; 2],
    /// Completed passes over passes played, one decimal; 0 for a team that played none.
    #[serde(rename = "stats.pass_accuracy_pct")]
    pub pass_accuracy_pct: [f64; 2],
    #[serde(rename = "stats.fouls")]
    pub fouls: [u32; 2],
    #[serde(rename = "stats.corners")]
    pub corners: [u32; 2],
    #[serde(rename = "stats.offsides")]
    pub offsides: [u32; 2],
}

/// Every player's energy, sent once every simulated second beside the statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub tick: u32,
    /// One value per wire slot, home first, from 0.0 (spent) to 1.0 (fresh), three decimals.
    pub energy: Vec<f64>,
}

/// Everything the server sends as a JSON text frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerMessage {
    Hello(Box<Hello>),
    Event(MatchEvent),
    Stats(Stats),
    Condition(Condition),
    Ack(Ack),
    Reject(Reject),
}

/// The playback speed a client asks for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetSpeed {
    pub speed: f32,
}

/// The newest tick a viewer has drawn. Once a client reports it, a live engine produces no
/// more than the stream's buffer bound of ticks beyond it, so a change the manager queues
/// reaches the engine close to the tick on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seen {
    pub tick: u32,
}

/// A change a client queues for a later stoppage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueueChange {
    /// `tactics` or `substitution`. Any other value is refused by name.
    #[serde(rename = "change.kind")]
    pub kind: String,
    /// `{ "patch": … }` for a tactics change, `{ "off": …, "on": … }` for a substitution.
    /// [`QueueChange::detail_typed`] reads it.
    #[serde(default)]
    pub detail: serde_json::Value,
}

/// A tactics change on the wire. Every field is optional; every number is an index into the
/// hello's `tactics`, and a role names its player by squad index.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mentality: Option<u8>,
    /// Six levels in the order of `tactics.instructions`; `null` leaves one unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<[Option<u8>; 6]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<RoleWire>,
}

/// One player's new role and duty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleWire {
    /// The player's squad index.
    pub squad: u16,
    pub role: u8,
    pub duty: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchDetail {
    patch: PatchWire,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SwapDetail {
    off: u16,
    on: u16,
}

/// A queued change's detail, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeDetail {
    Patch(PatchWire),
    /// Squad index `on` replaces squad index `off`.
    Swap {
        off: u16,
        on: u16,
    },
}

impl QueueChange {
    /// The detail read for the change's kind, or the fault in words a viewer can show. A kind
    /// this protocol does not name answers `None`, and the queue refuses it by name.
    pub fn detail_typed(&self) -> Option<Result<ChangeDetail, String>> {
        let read = match self.kind.as_str() {
            "tactics" => serde_json::from_value::<PatchDetail>(self.detail.clone())
                .map(|d| ChangeDetail::Patch(d.patch)),
            "substitution" => serde_json::from_value::<SwapDetail>(self.detail.clone()).map(|d| {
                ChangeDetail::Swap {
                    off: d.off,
                    on: d.on,
                }
            }),
            _ => return None,
        };
        Some(read.map_err(|e| format!("cannot read the {} change: {e}", self.kind)))
    }
}

/// The lineup the page picked, sent before `start`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetLineup {
    /// Eleven squad indices in slot order; slot 0 is the goalkeeper.
    pub lineup: Vec<u16>,
    pub bench: Vec<u16>,
    /// Pre-match tactics (formation, mentality, instructions, and roles), applied before
    /// kick-off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<PatchWire>,
}

/// Everything a client sends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ClientCommand {
    Start,
    Pause,
    SetSpeed(SetSpeed),
    QueueChange(QueueChange),
    SetLineup(SetLineup),
    Seen(Seen),
}

impl ClientCommand {
    /// The command name as the acknowledgement echoes it.
    pub fn name(&self) -> &'static str {
        match self {
            ClientCommand::Start => "start",
            ClientCommand::Pause => "pause",
            ClientCommand::SetSpeed(_) => "set-speed",
            ClientCommand::QueueChange(_) => "queue-change",
            ClientCommand::SetLineup(_) => "set-lineup",
            ClientCommand::Seen(_) => "seen",
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
                    kit_primary: "#c8102e".into(),
                    kit_secondary: "#000000".into(),
                    roster: vec![RosterEntry {
                        id: "a-1".into(),
                        name: "Keeper One".into(),
                        shirt: 1,
                        position: "GK".into(),
                        squad_index: 0,
                    }],
                    squad: vec![SquadEntry {
                        id: "a-1".into(),
                        name: "Keeper One".into(),
                        shirt: 1,
                        position: "GK".into(),
                        natural_fitness: 71,
                        role_fit: vec![80, 12],
                    }],
                    setup: Some(TeamSetup {
                        lineup: (0..11).collect(),
                        bench: vec![11, 12],
                        formation: 0,
                        mentality: 2,
                        instructions: vec![1, 1, 1, 1, 1, 0],
                        roles: vec![SlotRole { role: 0, duty: 1 }; 11],
                    }),
                },
                TeamRef {
                    id: "club-b".into(),
                    name: "B".into(),
                    kit_primary: "#6a0dad".into(),
                    kit_secondary: "#ff6a13".into(),
                    roster: Vec::new(),
                    squad: Vec::new(),
                    setup: None,
                },
            ],
            tactics: serde_json::json!({"roles": [{"name": "goalkeeper"}]}),
            substitutions: SubstitutionRules {
                limit: 5,
                windows: 3,
            },
        }
    }

    fn queue_change(kind: &str, detail: serde_json::Value) -> QueueChange {
        QueueChange {
            kind: kind.into(),
            detail,
        }
    }

    #[test]
    fn each_detail_shape_is_read_for_its_kind() {
        let patch = queue_change(
            "tactics",
            serde_json::json!({"patch": {"mentality": 4, "instructions": [null, 2, null, null, null, null],
                "roles": [{"squad": 7, "role": 3, "duty": 2}]}}),
        );
        let Some(Ok(ChangeDetail::Patch(p))) = patch.detail_typed() else {
            panic!("a patch detail must be read");
        };
        assert_eq!(p.mentality, Some(4));
        assert_eq!(
            p.instructions,
            Some([None, Some(2), None, None, None, None])
        );
        assert_eq!(
            p.roles,
            vec![RoleWire {
                squad: 7,
                role: 3,
                duty: 2
            }]
        );

        let swap = queue_change("substitution", serde_json::json!({"off": 9, "on": 14}));
        assert_eq!(
            swap.detail_typed(),
            Some(Ok(ChangeDetail::Swap { off: 9, on: 14 }))
        );
        assert_eq!(
            queue_change("formation", serde_json::Value::Null).detail_typed(),
            None
        );
    }

    #[test]
    fn a_detail_that_does_not_match_its_kind_is_refused_naming_the_fault() {
        let swapped = queue_change("tactics", serde_json::json!({"off": 9, "on": 14}));
        let Some(Err(reason)) = swapped.detail_typed() else {
            panic!("a swap under tactics must be refused");
        };
        assert!(
            reason.starts_with("cannot read the tactics change"),
            "{reason}"
        );
        let patched = queue_change("substitution", serde_json::json!({"patch": {}}));
        assert!(matches!(patched.detail_typed(), Some(Err(_))));
        let missing = queue_change("substitution", serde_json::Value::Null);
        assert!(matches!(missing.detail_typed(), Some(Err(_))));
    }

    #[test]
    fn an_unknown_field_in_a_detail_is_refused() {
        let extra = queue_change(
            "substitution",
            serde_json::json!({"off": 9, "on": 14, "captain": true}),
        );
        let Some(Err(reason)) = extra.detail_typed() else {
            panic!("an unknown field must be refused");
        };
        assert!(reason.contains("captain"), "{reason}");
        let short = queue_change(
            "tactics",
            serde_json::json!({"patch": {"instructions": [1, 2]}}),
        );
        assert!(matches!(short.detail_typed(), Some(Err(_))));
    }

    #[test]
    fn set_lineup_round_trips_and_a_hello_without_a_squad_omits_it() {
        let command = ClientCommand::SetLineup(SetLineup {
            lineup: (0..11).collect(),
            bench: vec![11, 12],
            patch: Some(PatchWire {
                mentality: Some(3),
                ..PatchWire::default()
            }),
        });
        let json = serde_json::to_string(&command).unwrap();
        assert!(json.starts_with("{\"type\":\"set-lineup\""), "{json}");
        assert_eq!(
            serde_json::from_str::<ClientCommand>(&json).unwrap(),
            command
        );
        let hello = serde_json::to_string(&ServerMessage::Hello(Box::new(hello()))).unwrap();
        assert_eq!(hello.matches("\"squad\":").count(), 1, "{hello}");
        assert_eq!(hello.matches("\"setup\":").count(), 1, "{hello}");
        assert!(hello.contains("\"player.natural_fitness\":71"), "{hello}");
    }

    #[test]
    fn every_server_message_round_trips_through_json() {
        let messages = [
            ServerMessage::Hello(Box::new(hello())),
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
                possession_pct: [52.4, 47.6],
                shots: [12, 9],
                shots_on_target: [5, 3],
                xg: [1.42, 0.87],
                passes: [480, 410],
                pass_accuracy_pct: [84.2, 79.5],
                fouls: [11, 13],
                corners: [6, 4],
                offsides: [2, 3],
            }),
            ServerMessage::Condition(Condition {
                tick: 50,
                energy: vec![0.998; 22],
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
            ClientCommand::SetLineup(SetLineup {
                lineup: vec![0; 11],
                bench: Vec::new(),
                patch: None,
            }),
            ClientCommand::Seen(Seen { tick: 1_200 }),
        ];
        for c in commands {
            let json = serde_json::to_string(&c).unwrap();
            assert_eq!(serde_json::from_str::<ClientCommand>(&json).unwrap(), c);
        }
        assert_eq!(
            serde_json::to_string(&ClientCommand::Start).unwrap(),
            "{\"type\":\"start\"}"
        );
        assert_eq!(
            serde_json::to_string(&ClientCommand::Seen(Seen { tick: 7 })).unwrap(),
            "{\"type\":\"seen\",\"tick\":7}"
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
    fn a_hello_names_both_kit_colours_under_dotted_keys() {
        let json = serde_json::to_string(&ServerMessage::Hello(Box::new(hello()))).unwrap();
        assert!(json.contains("\"team.kit.primary\":\"#c8102e\""), "{json}");
        assert!(
            json.contains("\"team.kit.secondary\":\"#000000\""),
            "{json}"
        );
        assert!(json.contains("\"team.kit.primary\":\"#6a0dad\""), "{json}");
        assert!(
            json.contains("\"team.kit.secondary\":\"#ff6a13\""),
            "{json}"
        );
        let back = serde_json::from_str::<ServerMessage>(&json).unwrap();
        assert_eq!(back, ServerMessage::Hello(Box::new(hello())));
    }

    #[test]
    fn a_hello_without_a_roster_reads_as_an_empty_one() {
        let json = serde_json::to_string(&ServerMessage::Hello(Box::new(hello()))).unwrap();
        assert!(json.contains("\"player.squad_index\":0"), "{json}");
        let bare = json.replace(",\"roster\":[]", "");
        assert_ne!(bare, json);
        let ServerMessage::Hello(back) = serde_json::from_str::<ServerMessage>(&bare).unwrap()
        else {
            panic!("not a hello");
        };
        assert!(back.teams[1].roster.is_empty());
    }

    #[test]
    fn a_condition_message_is_tagged_condition() {
        let json = serde_json::to_string(&ServerMessage::Condition(Condition {
            tick: 50,
            energy: vec![1.0, 0.5],
        }))
        .unwrap();
        assert_eq!(
            json,
            "{\"type\":\"condition\",\"tick\":50,\"energy\":[1.0,0.5]}"
        );
    }

    #[test]
    fn a_hello_without_kit_colours_is_refused() {
        let json = serde_json::to_string(&ServerMessage::Hello(Box::new(hello())))
            .unwrap()
            .replace(",\"team.kit.primary\":\"#c8102e\"", "");
        let err = serde_json::from_str::<ServerMessage>(&json).unwrap_err();
        assert!(err.to_string().contains("team.kit.primary"), "{err}");
    }

    #[test]
    fn an_unknown_command_type_is_refused() {
        let err = serde_json::from_str::<ClientCommand>("{\"type\":\"teleport\"}").unwrap_err();
        assert!(err.to_string().contains("teleport"), "{err}");
    }
}
