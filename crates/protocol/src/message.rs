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
    /// The name of the team's formation at kick-off, as the tactics file names it (`4-4-2`).
    /// Sent by a session a page manages, for both teams; empty otherwise, and a hello from an
    /// earlier build carries none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub formation: String,
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
    /// The natural-fitness attribute in tenths of the 1 to 20 scale (10 to 200; a team
    /// converted from an old file may hold 2 to 8). Before kick-off every player is fresh,
    /// so this is the one fitness figure the engine holds. Protocol 3 sent 0 to 100.
    #[serde(rename = "player.natural_fitness")]
    pub natural_fitness: u8,
    /// The player's consistency, a hidden value: never a number, only a word and how sure
    /// the club is of it.
    #[serde(rename = "player.consistency")]
    pub consistency: HiddenWord,
    /// The player's injury proneness, a hidden value: never a number, only a word and how
    /// sure the club is of it. The prematch Risk word reads it.
    #[serde(rename = "player.injury_proneness")]
    pub injury_proneness: HiddenWord,
    /// How well the player fits each role in tenths of the 1 to 20 scale (0 to 200), one value
    /// per role in the order of the hello's `tactics.roles`.
    pub role_fit: Vec<u8>,
}

/// A hidden value as the club knows it. `word` is a lower-case key (for example
/// `rarely_off` or `injury_prone`) whose display text the page owns; it is absent while the
/// confidence is `not_yet_known`. No number for a hidden value is ever sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HiddenWord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word: Option<String>,
    pub confidence: Confidence,
}

/// How sure the club is of a hidden value: from the matches the player has seen at the club,
/// none is `not_yet_known`, a few `tentative`, many `firm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    NotYetKnown,
    Tentative,
    Firm,
}

/// Every player's match rating, sent once at full time after the closing statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ratings {
    pub tick: u32,
    /// Every player who played, home first and in squad order.
    pub ratings: Vec<RatingWire>,
}

/// One player's match rating: 1.0 to 10.0 with one decimal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RatingWire {
    #[serde(rename = "player.id")]
    pub id: String,
    pub rating: f64,
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
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubstitutionRules {
    /// Substitutions each team may make.
    pub limit: u8,
    /// Stoppages at which each team may make them; half-time uses none.
    pub windows: u8,
    /// Substitutions each team gains once extra time starts, on top of `limit`.
    #[serde(default)]
    pub extra_substitutions: u8,
    /// Windows each team gains once extra time starts, on top of `windows`.
    #[serde(default)]
    pub extra_windows: u8,
    /// The stoppage kinds whose substitutions use no window, as the rule pack writes them
    /// (`half_time` in the shipped pack).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows_exempt: Vec<String>,
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
    /// `true` for a knockout match: level after regulation time, it plays extra time and then
    /// a penalty shoot-out. Written only when `true`; a hello from an earlier build reads as
    /// `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub knockout: bool,
    /// The length of the home team's ground, the pitch of the match, in metres. Written
    /// only when it is not 105; a hello without it, as from an earlier build, reads as 105.
    #[serde(
        rename = "ground.length",
        default = "default_ground_length",
        skip_serializing_if = "is_default_ground_length"
    )]
    pub ground_length: f64,
    /// The width of the home team's ground in metres. Written only when it is not 68; a
    /// hello without it reads as 68.
    #[serde(
        rename = "ground.width",
        default = "default_ground_width",
        skip_serializing_if = "is_default_ground_width"
    )]
    pub ground_width: f64,
}

/// The length of a ground that a hello does not name, in metres.
pub const DEFAULT_GROUND_LENGTH: f64 = 105.0;
/// The width of a ground that a hello does not name, in metres.
pub const DEFAULT_GROUND_WIDTH: f64 = 68.0;

fn default_ground_length() -> f64 {
    DEFAULT_GROUND_LENGTH
}

fn default_ground_width() -> f64 {
    DEFAULT_GROUND_WIDTH
}

fn is_default_ground_length(v: &f64) -> bool {
    *v == DEFAULT_GROUND_LENGTH
}

fn is_default_ground_width(v: &f64) -> bool {
    *v == DEFAULT_GROUND_WIDTH
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
    /// Substitutions each team has made, home first.
    #[serde(default)]
    pub subs_used: [u8; 2],
    /// Substitution windows each team has used, home first.
    #[serde(default)]
    pub windows_used: [u8; 2],
}

/// A queued change that the stoppage now opening takes. It is not a match event: it is sent
/// on the tick the stoppage opens, before the change's verdict event, and no record or
/// replay keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeStateNote {
    #[serde(rename = "change.queue_id")]
    pub queue_id: String,
    pub state: crate::command::ChangeState,
    pub tick: u32,
}

/// One change the assistant would make for the page's team, as the computer manager's check
/// finds it. The page may queue it with `queue-change`; nothing is queued until it does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvicePick {
    /// The reason, as the `ai.decision` codes name it: `sub-injury`, `sub-keeper`,
    /// `sub-fatigue`, `mentality-up-trailing` or `mentality-down-leading`.
    pub code: String,
    /// `substitution` or `tactics`, as `change.kind` names it.
    pub kind: String,
    /// The squad index of the player coming off, for a substitution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off: Option<u16>,
    /// The squad index of the player coming on, for a substitution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<u16>,
    /// The tactics change, for a tactics pick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<PatchWire>,
}

/// The assistant's picks for the page's team after a check. It is not a match event: no
/// record or replay keeps it, and it changes nothing in the match. Each message replaces the
/// picks before it; an empty list means the assistant has no pick open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Advice {
    pub tick: u32,
    pub minute: u32,
    pub picks: Vec<AdvicePick>,
}

/// One other match of the player's matchday.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchdayFixture {
    /// The fixture's place in the round, from 0; every ground message names it.
    pub fixture: u32,
    /// The two clubs, with no roster.
    pub home: TeamRef,
    pub away: TeamRef,
}

/// The other matches of the player's matchday. It is not a match event: no record or replay
/// keeps it. Sent after the hello on every connection, every fixture at 0-0 until its
/// ground events say otherwise; an empty list means the match has no other grounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Matchday {
    /// The matchday's number; 1 while the generated round is the only matchday.
    pub round: u32,
    pub fixtures: Vec<MatchdayFixture>,
}

/// What happened at another ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GroundKind {
    Goal,
    HalfTime,
    SecondHalf,
    /// A period of extra time starts.
    ExtraTime,
    FullTime,
    /// A fault stopped the match; it has no result.
    Unavailable,
}

/// The side of a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Side {
    Home,
    Away,
}

/// One event at another ground, revealed on the player's match clock: the engine sends it
/// once its own tick of the player's match reaches `tick`, and a new connection receives
/// every one up to that tick again. Every ground kicks off with the player's match, so
/// `tick` is a moment on the player's clock too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundEvent {
    pub fixture: u32,
    pub tick: u32,
    pub kind: GroundKind,
    /// The minute of play, counted from 0, and the added minute in added time.
    pub minute: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added: Option<u32>,
    /// The scoring side, on a goal only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
    /// The scorer's name, on a goal only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scorer: Option<String>,
    /// The score after the event, home first.
    pub score: [u32; 2],
    /// `true` when the event was computed more than a simulated second after the player's
    /// match passed its tick, so it reaches the page late.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub late: bool,
}

/// How far each other ground has played, once every simulated second: the tick each fixture
/// has reached, in fixture order. A fixture behind the player's clock shows its events late.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundProgress {
    /// The player's tick when the progress was sent.
    pub tick: u32,
    pub reached: Vec<u32>,
}

/// Everything the server sends as a JSON text frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerMessage {
    Hello(Box<Hello>),
    /// Boxed, like the hello, because the event is by far the largest message.
    Event(Box<MatchEvent>),
    Stats(Stats),
    Condition(Condition),
    Ack(Ack),
    Reject(Reject),
    ChangeState(ChangeStateNote),
    Advice(Advice),
    Matchday(Matchday),
    GroundEvent(GroundEvent),
    GroundProgress(GroundProgress),
    Ratings(Ratings),
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

/// A test seam: asks a started match to produce every tick up to `tick` without waiting for
/// `start`, `pause` or the `seen` lead bound. The match is unchanged; only when its ticks are
/// sent changes. Refused unless the engine was started with `--test-jump`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Jump {
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

/// Withdraws a queued change before a stoppage takes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelChange {
    /// The identifier the change's `queue-change` acknowledgement gave.
    #[serde(rename = "change.queue_id")]
    pub queue_id: String,
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
    CancelChange(CancelChange),
    /// After kick-off, plays the rest of the match from its exact state at full speed: the
    /// engine stops waiting for `start`, `pause` and the `seen` lead bound and streams every
    /// remaining tick and message as usual. Refused before kick-off.
    Skip,
    /// A test seam: after kick-off, produces every tick up to `tick` flat out. Refused unless
    /// the engine was started with `--test-jump`, and before kick-off.
    Jump(Jump),
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
            ClientCommand::CancelChange(_) => "cancel-change",
            ClientCommand::Skip => "skip",
            ClientCommand::Jump(_) => "jump",
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
                        natural_fitness: 142,
                        consistency: HiddenWord {
                            word: None,
                            confidence: Confidence::NotYetKnown,
                        },
                        injury_proneness: HiddenWord {
                            word: Some("rarely_injured".into()),
                            confidence: Confidence::Tentative,
                        },
                        role_fit: vec![160, 24],
                    }],
                    setup: Some(TeamSetup {
                        lineup: (0..11).collect(),
                        bench: vec![11, 12],
                        formation: 0,
                        mentality: 2,
                        instructions: vec![1, 1, 1, 1, 1, 0],
                        roles: vec![SlotRole { role: 0, duty: 1 }; 11],
                    }),
                    formation: "4-4-2".into(),
                },
                TeamRef {
                    id: "club-b".into(),
                    name: "B".into(),
                    kit_primary: "#6a0dad".into(),
                    kit_secondary: "#ff6a13".into(),
                    roster: Vec::new(),
                    squad: Vec::new(),
                    setup: None,
                    formation: "4-3-3".into(),
                },
            ],
            tactics: serde_json::json!({"roles": [{"name": "goalkeeper"}]}),
            substitutions: SubstitutionRules {
                limit: 5,
                windows: 3,
                extra_substitutions: 1,
                extra_windows: 1,
                windows_exempt: vec!["half_time".into()],
            },
            knockout: true,
            ground_length: DEFAULT_GROUND_LENGTH,
            ground_width: DEFAULT_GROUND_WIDTH,
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
        assert!(hello.contains("\"player.natural_fitness\":142"), "{hello}");
    }

    #[test]
    fn every_server_message_round_trips_through_json() {
        let messages = [
            ServerMessage::Hello(Box::new(hello())),
            ServerMessage::Event(Box::new(MatchEvent::play(
                "owner",
                "match",
                1,
                EventType::KickOff,
                None,
                [0, 0],
            ))),
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
                subs_used: [2, 0],
                windows_used: [1, 0],
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
            ServerMessage::ChangeState(ChangeStateNote {
                queue_id: "q-1-0".into(),
                state: ChangeState::AppliesNow,
                tick: 3_000,
            }),
            ServerMessage::Advice(advice()),
            ServerMessage::Matchday(matchday()),
            ServerMessage::GroundEvent(goal()),
            ServerMessage::GroundProgress(GroundProgress {
                tick: 6_000,
                reached: vec![6_150, 6_100, 6_200, 5_900],
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
            ClientCommand::CancelChange(CancelChange {
                queue_id: "q-1200-0".into(),
            }),
            ClientCommand::Skip,
            ClientCommand::Jump(Jump { tick: 20_000 }),
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
            subs_used: [1, 0],
            windows_used: [1, 0],
        }))
        .unwrap();
        assert_eq!(
            json,
            "{\"type\":\"condition\",\"tick\":50,\"energy\":[1.0,0.5],\
             \"subs_used\":[1,0],\"windows_used\":[1,0]}"
        );
        // A condition message from an earlier build carries no counts and reads as none.
        let ServerMessage::Condition(bare) = serde_json::from_str::<ServerMessage>(
            "{\"type\":\"condition\",\"tick\":50,\"energy\":[1.0]}",
        )
        .unwrap() else {
            panic!("not a condition message");
        };
        assert_eq!((bare.subs_used, bare.windows_used), ([0, 0], [0, 0]));
    }

    #[test]
    fn a_change_state_note_and_a_cancel_travel_under_their_own_tags() {
        let note = serde_json::to_string(&ServerMessage::ChangeState(ChangeStateNote {
            queue_id: "q-7-0".into(),
            state: ChangeState::AppliesNow,
            tick: 9,
        }))
        .unwrap();
        assert_eq!(
            note,
            "{\"type\":\"change-state\",\"change.queue_id\":\"q-7-0\",\
             \"state\":\"applies-now\",\"tick\":9}"
        );
        let cancel = serde_json::to_string(&ClientCommand::CancelChange(CancelChange {
            queue_id: "q-7-0".into(),
        }))
        .unwrap();
        assert_eq!(
            cancel,
            "{\"type\":\"cancel-change\",\"change.queue_id\":\"q-7-0\"}"
        );
    }

    #[test]
    fn a_skip_carries_no_fields_and_its_ack_names_it() {
        let text = serde_json::to_string(&ClientCommand::Skip).unwrap();
        assert_eq!(text, "{\"type\":\"skip\"}");
        assert_eq!(
            serde_json::from_str::<ClientCommand>(&text).unwrap(),
            ClientCommand::Skip
        );
        assert_eq!(ClientCommand::Skip.name(), "skip");
        let ack = serde_json::to_string(&ServerMessage::Ack(crate::Ack {
            command: ClientCommand::Skip.name().into(),
            queue_id: None,
            queued_tick: 90_000,
            state: None,
            speed: None,
        }))
        .unwrap();
        assert_eq!(
            ack,
            "{\"type\":\"ack\",\"command\":\"skip\",\"change.queued_tick\":90000}"
        );
    }

    #[test]
    fn a_jump_carries_its_tick_and_its_ack_names_it() {
        let jump = ClientCommand::Jump(Jump { tick: 20_000 });
        let text = serde_json::to_string(&jump).unwrap();
        assert_eq!(text, "{\"type\":\"jump\",\"tick\":20000}");
        assert_eq!(serde_json::from_str::<ClientCommand>(&text).unwrap(), jump);
        assert_eq!(jump.name(), "jump");
        let ack = serde_json::to_string(&ServerMessage::Ack(crate::Ack {
            command: jump.name().into(),
            queue_id: None,
            queued_tick: 3_000,
            state: None,
            speed: None,
        }))
        .unwrap();
        assert_eq!(
            ack,
            "{\"type\":\"ack\",\"command\":\"jump\",\"change.queued_tick\":3000}"
        );
    }

    #[test]
    fn a_hello_from_an_earlier_build_reads_without_the_newer_fields() {
        let json = serde_json::to_string(&ServerMessage::Hello(Box::new(hello())))
            .unwrap()
            .replace(",\"extra_substitutions\":1,\"extra_windows\":1", "")
            .replace(",\"windows_exempt\":[\"half_time\"]", "");
        assert!(!json.contains("extra_windows"), "{json}");
        let ServerMessage::Hello(back) = serde_json::from_str::<ServerMessage>(&json).unwrap()
        else {
            panic!("not a hello");
        };
        assert_eq!(back.substitutions.extra_substitutions, 0);
        assert!(back.substitutions.windows_exempt.is_empty());
    }

    /// A hidden value travels as a word key and a confidence; with no match seen there is no
    /// word, and a squad entry still carrying the old resistance figure is refused.
    #[test]
    fn hidden_values_travel_as_words_and_the_old_figure_is_refused() {
        let json = serde_json::to_string(&ServerMessage::Hello(Box::new(hello()))).unwrap();
        assert!(
            json.contains(
                "\"player.consistency\":{\"confidence\":\"not_yet_known\"},\
                 \"player.injury_proneness\":{\"word\":\"rarely_injured\",\
                 \"confidence\":\"tentative\"}"
            ),
            "{json}"
        );
        let old = json.replace(
            "\"player.natural_fitness\":142,",
            "\"player.natural_fitness\":142,\"player.injury_resistance\":128,",
        );
        assert!(serde_json::from_str::<ServerMessage>(&old).is_err());
        let ratings = ServerMessage::Ratings(Ratings {
            tick: 270_000,
            ratings: vec![RatingWire {
                id: "a-1".into(),
                rating: 7.3,
            }],
        });
        assert_eq!(
            serde_json::to_string(&ratings).unwrap(),
            "{\"type\":\"ratings\",\"tick\":270000,\"ratings\":[{\"player.id\":\"a-1\",\"rating\":7.3}]}"
        );
    }

    fn bare_team(id: &str, name: &str) -> TeamRef {
        TeamRef {
            id: id.into(),
            name: name.into(),
            kit_primary: "#c8102e".into(),
            kit_secondary: "#000000".into(),
            roster: Vec::new(),
            squad: Vec::new(),
            setup: None,
            formation: String::new(),
        }
    }

    fn matchday() -> Matchday {
        Matchday {
            round: 1,
            fixtures: vec![MatchdayFixture {
                fixture: 0,
                home: bare_team("club-000007ea-00", "Belfield Athletic"),
                away: bare_team("club-000007ea-03", "Harrow Vale"),
            }],
        }
    }

    fn goal() -> GroundEvent {
        GroundEvent {
            fixture: 2,
            tick: 201_000,
            kind: GroundKind::Goal,
            minute: 66,
            added: None,
            side: Some(Side::Home),
            scorer: Some("Ade Okafor".into()),
            score: [1, 0],
            late: false,
        }
    }

    #[test]
    fn the_matchday_messages_travel_under_their_own_tags() {
        let json = serde_json::to_string(&ServerMessage::Matchday(matchday())).unwrap();
        assert!(
            json.starts_with(r#"{"type":"matchday","round":1,"fixtures":[{"fixture":0,"#),
            "{json}"
        );
        assert!(json.contains(r#""team.id":"club-000007ea-00""#), "{json}");
        let text = serde_json::to_string(&ServerMessage::GroundEvent(goal())).unwrap();
        assert_eq!(
            text,
            r#"{"type":"ground-event","fixture":2,"tick":201000,"kind":"goal","minute":66,"side":"home","scorer":"Ade Okafor","score":[1,0]}"#
        );
        // A late event says so; a half-time names no side or scorer.
        let late = GroundEvent {
            kind: GroundKind::HalfTime,
            minute: 45,
            added: Some(2),
            side: None,
            scorer: None,
            late: true,
            ..goal()
        };
        let json = serde_json::to_string(&ServerMessage::GroundEvent(late.clone())).unwrap();
        assert!(
            json.contains(r#""kind":"half-time","minute":45,"added":2,"#),
            "{json}"
        );
        assert!(json.ends_with(r#""score":[1,0],"late":true}"#), "{json}");
        assert_eq!(
            serde_json::from_str::<ServerMessage>(&json).unwrap(),
            ServerMessage::GroundEvent(late)
        );
        assert_eq!(
            serde_json::to_string(&GroundKind::Unavailable).unwrap(),
            r#""unavailable""#
        );
        let progress = serde_json::to_string(&ServerMessage::GroundProgress(GroundProgress {
            tick: 50,
            reached: vec![50, 0],
        }))
        .unwrap();
        assert_eq!(
            progress,
            r#"{"type":"ground-progress","tick":50,"reached":[50,0]}"#
        );
        let err = serde_json::from_str::<ServerMessage>(
            r#"{"type":"ground-progress","tick":1,"reached":[],"fast":true}"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("fast"), "{err}");
    }

    fn advice() -> Advice {
        Advice {
            tick: 3_000,
            minute: 58,
            picks: vec![
                AdvicePick {
                    code: "sub-fatigue".into(),
                    kind: "substitution".into(),
                    off: Some(9),
                    on: Some(14),
                    patch: None,
                },
                AdvicePick {
                    code: "mentality-up-trailing".into(),
                    kind: "tactics".into(),
                    off: None,
                    on: None,
                    patch: Some(PatchWire {
                        mentality: Some(3),
                        instructions: Some([Some(2), None, None, None, None, None]),
                        ..PatchWire::default()
                    }),
                },
            ],
        }
    }

    #[test]
    fn advice_travels_under_its_own_tag_and_a_pick_omits_what_its_kind_does_not_use() {
        let json = serde_json::to_string(&ServerMessage::Advice(advice())).unwrap();
        assert!(
            json.starts_with("{\"type\":\"advice\",\"tick\":3000,\"minute\":58,"),
            "{json}"
        );
        assert!(
            json.contains(
                "{\"code\":\"sub-fatigue\",\"kind\":\"substitution\",\"off\":9,\"on\":14}"
            ),
            "{json}"
        );
        assert!(
            json.contains("\"kind\":\"tactics\",\"patch\":{\"mentality\":3"),
            "{json}"
        );
        let err = serde_json::from_str::<ServerMessage>(
            "{\"type\":\"advice\",\"tick\":1,\"minute\":0,\"picks\":[],\"gain\":2}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("gain"), "{err}");
    }

    #[test]
    fn a_hello_names_both_formations_and_the_knockout_flag_and_reads_without_them() {
        let json = serde_json::to_string(&ServerMessage::Hello(Box::new(hello()))).unwrap();
        assert!(json.contains("\"formation\":\"4-4-2\""), "{json}");
        assert!(json.contains("\"formation\":\"4-3-3\""), "{json}");
        assert!(json.ends_with("\"knockout\":true}"), "{json}");
        let bare = json
            .replace(",\"formation\":\"4-4-2\"", "")
            .replace(",\"formation\":\"4-3-3\"", "")
            .replace(",\"knockout\":true", "");
        let ServerMessage::Hello(back) = serde_json::from_str::<ServerMessage>(&bare).unwrap()
        else {
            panic!("not a hello");
        };
        assert!(back.teams.iter().all(|t| t.formation.is_empty()));
        assert!(!back.knockout);
        // A hello that is not a knockout match and names no formation writes neither field.
        let mut plain = hello();
        plain.knockout = false;
        plain.teams[1].formation.clear();
        let json = serde_json::to_string(&ServerMessage::Hello(Box::new(plain))).unwrap();
        assert!(!json.contains("knockout"), "{json}");
        assert_eq!(json.matches("\"formation\":\"").count(), 1, "{json}");
    }

    #[test]
    fn a_hello_names_a_ground_only_off_105_by_68_and_reads_without_it() {
        let json = serde_json::to_string(&ServerMessage::Hello(Box::new(hello()))).unwrap();
        assert!(!json.contains("ground."), "{json}");
        let ServerMessage::Hello(back) = serde_json::from_str::<ServerMessage>(&json).unwrap()
        else {
            panic!("not a hello");
        };
        assert_eq!((back.ground_length, back.ground_width), (105.0, 68.0));
        let mut smaller = hello();
        smaller.ground_length = 100.0;
        smaller.ground_width = 64.0;
        let json = serde_json::to_string(&ServerMessage::Hello(Box::new(smaller.clone()))).unwrap();
        assert!(
            json.ends_with("\"knockout\":true,\"ground.length\":100.0,\"ground.width\":64.0}"),
            "{json}"
        );
        let ServerMessage::Hello(back) = serde_json::from_str::<ServerMessage>(&json).unwrap()
        else {
            panic!("not a hello");
        };
        assert_eq!(*back, smaller);
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
