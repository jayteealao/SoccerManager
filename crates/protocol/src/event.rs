//! Match events: the `match-event` record kind the observability contract reserves. The
//! engine produces kick-off, goal, half-time, full-time, the law events (offside, foul, card,
//! and every restart), and the verdict on a queued change.

use serde::{Deserialize, Serialize};

use crate::command::{ChangeKind, ChangeState};

/// Simulated ticks in one minute of play (50 ticks per second).
pub const TICKS_PER_MINUTE: u32 = 50 * 60;

/// The event types this build emits. The contract's enumeration is wider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventType {
    KickOff,
    Goal,
    HalfTime,
    FullTime,
    TacticsChange,
    Offside,
    Foul,
    Card,
    ThrowIn,
    Corner,
    GoalKick,
    FreeKick,
    Penalty,
}

impl EventType {
    /// Every event type, in declaration order.
    pub const ALL: [EventType; 13] = [
        EventType::KickOff,
        EventType::Goal,
        EventType::HalfTime,
        EventType::FullTime,
        EventType::TacticsChange,
        EventType::Offside,
        EventType::Foul,
        EventType::Card,
        EventType::ThrowIn,
        EventType::Corner,
        EventType::GoalKick,
        EventType::FreeKick,
        EventType::Penalty,
    ];

    /// The value as the contract writes it.
    pub fn code(&self) -> &'static str {
        match self {
            EventType::KickOff => "kick-off",
            EventType::Goal => "goal",
            EventType::HalfTime => "half-time",
            EventType::FullTime => "full-time",
            EventType::TacticsChange => "tactics-change",
            EventType::Offside => "offside",
            EventType::Foul => "foul",
            EventType::Card => "card",
            EventType::ThrowIn => "throw-in",
            EventType::Corner => "corner",
            EventType::GoalKick => "goal-kick",
            EventType::FreeKick => "free-kick",
            EventType::Penalty => "penalty",
        }
    }
}

/// The card a `card` event shows. A second yellow is a yellow card followed by a red one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CardKind {
    Yellow,
    SecondYellow,
    Red,
}

/// What the server decided about one queued change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeOutcome {
    pub kind: Option<ChangeKind>,
    pub queue_id: Option<String>,
    pub state: ChangeState,
    pub rejected_reason: Option<String>,
}

/// One `match-event` row. The envelope keys (`record.kind`, `schema.version`, `service`,
/// `build.hash`) are added by the writer, which owns the observability envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchEvent {
    #[serde(rename = "owner.id")]
    pub owner_id: String,
    #[serde(rename = "match.id")]
    pub match_id: String,
    pub tick: u32,
    pub minute: u32,
    #[serde(rename = "event.type")]
    pub event_type: EventType,
    #[serde(rename = "team.id", skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(rename = "home.score")]
    pub home_score: u32,
    #[serde(rename = "away.score")]
    pub away_score: u32,
    #[serde(rename = "change.kind", skip_serializing_if = "Option::is_none")]
    pub change_kind: Option<ChangeKind>,
    #[serde(rename = "change.queued_tick", skip_serializing_if = "Option::is_none")]
    pub change_queued_tick: Option<u32>,
    /// Additive extra: the contract reserves no key for the queue identifier the
    /// acknowledgement returns. The observability audit settles it.
    #[serde(rename = "change.queue_id", skip_serializing_if = "Option::is_none")]
    pub change_queue_id: Option<String>,
    #[serde(
        rename = "change.rejected_reason",
        skip_serializing_if = "Option::is_none"
    )]
    pub change_rejected_reason: Option<String>,
    #[serde(rename = "change.state", skip_serializing_if = "Option::is_none")]
    pub change_state: Option<ChangeState>,
    /// The player the event names: the offender, the booked player.
    #[serde(rename = "player.id", skip_serializing_if = "Option::is_none")]
    pub player_id: Option<String>,
    /// The second player: the fouled player.
    #[serde(
        rename = "player.secondary_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub player_secondary_id: Option<String>,
    /// Additive extra, on a `card` event.
    #[serde(rename = "card.kind", skip_serializing_if = "Option::is_none")]
    pub card_kind: Option<CardKind>,
    /// Additive extra, on a `foul` event: `true` when play continued with advantage.
    #[serde(rename = "foul.advantage", skip_serializing_if = "Option::is_none")]
    pub foul_advantage: Option<bool>,
    /// Additive extra: the added minute in added time (2 at 45+2), absent otherwise.
    #[serde(rename = "minute.added", skip_serializing_if = "Option::is_none")]
    pub minute_added: Option<u32>,
    /// Additive extra, on `half-time` and `full-time`: the seconds added to the half.
    #[serde(rename = "added_time.s", skip_serializing_if = "Option::is_none")]
    pub added_time_s: Option<u32>,
}

impl MatchEvent {
    /// A play event. The minute is counted from the tick; an event from the engine carries
    /// the engine clock's minute instead, through `at_minute`.
    pub fn play(
        owner_id: &str,
        match_id: &str,
        tick: u32,
        event_type: EventType,
        team_id: Option<String>,
        scores: [u32; 2],
    ) -> Self {
        Self {
            owner_id: owner_id.to_string(),
            match_id: match_id.to_string(),
            tick,
            minute: tick / TICKS_PER_MINUTE,
            event_type,
            team_id,
            home_score: scores[0],
            away_score: scores[1],
            change_kind: None,
            change_queued_tick: None,
            change_queue_id: None,
            change_rejected_reason: None,
            change_state: None,
            player_id: None,
            player_secondary_id: None,
            card_kind: None,
            foul_advantage: None,
            minute_added: None,
            added_time_s: None,
        }
    }

    /// The minute of play and the added minute, as the engine clock shows them.
    pub fn at_minute(mut self, minute: u32, added: Option<u32>) -> Self {
        self.minute = minute;
        self.minute_added = added;
        self
    }

    /// The player the event names.
    pub fn player(mut self, id: Option<String>) -> Self {
        self.player_id = id;
        self
    }

    /// The second player the event names.
    pub fn secondary(mut self, id: Option<String>) -> Self {
        self.player_secondary_id = id;
        self
    }

    pub fn card(mut self, kind: Option<CardKind>) -> Self {
        self.card_kind = kind;
        self
    }

    pub fn advantage(mut self, advantage: Option<bool>) -> Self {
        self.foul_advantage = advantage;
        self
    }

    pub fn added_time(mut self, seconds: Option<u32>) -> Self {
        self.added_time_s = seconds;
        self
    }

    /// The verdict on a queued change, as a `tactics-change` event.
    pub fn change(
        owner_id: &str,
        match_id: &str,
        tick: u32,
        scores: [u32; 2],
        outcome: ChangeOutcome,
    ) -> Self {
        Self {
            change_kind: outcome.kind,
            change_queued_tick: Some(tick),
            change_queue_id: outcome.queue_id,
            change_rejected_reason: outcome.rejected_reason,
            change_state: Some(outcome.state),
            ..Self::play(
                owner_id,
                match_id,
                tick,
                EventType::TacticsChange,
                None,
                scores,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_goal_carries_both_scores_and_the_minute() {
        let e = MatchEvent::play(
            "0123456789abcdef0123456789abcdef",
            "000000000000002a-1",
            9_000,
            EventType::Goal,
            Some("club-a".into()),
            [1, 0],
        );
        assert_eq!(e.minute, 3);
        let json = serde_json::to_string(&e).unwrap();
        for key in [
            "\"event.type\":\"goal\"",
            "\"home.score\":1",
            "\"away.score\":0",
            "\"team.id\":\"club-a\"",
            "\"tick\":9000",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        assert!(!json.contains("change."), "{json}");
    }

    #[test]
    fn every_event_type_writes_the_contract_spelling() {
        for t in EventType::ALL {
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, format!("\"{}\"", t.code()));
        }
    }

    #[test]
    fn a_second_yellow_names_the_player_the_card_and_the_added_minute() {
        let e = MatchEvent::play(
            "0123456789abcdef0123456789abcdef",
            "000000000000002a-1",
            139_000,
            EventType::Card,
            Some("club-b".into()),
            [0, 1],
        )
        .at_minute(45, Some(2))
        .player(Some("p-club-b-04".into()))
        .card(Some(CardKind::SecondYellow));
        let json = serde_json::to_string(&e).unwrap();
        for key in [
            "\"event.type\":\"card\"",
            "\"minute\":45",
            "\"minute.added\":2",
            "\"player.id\":\"p-club-b-04\"",
            "\"card.kind\":\"second-yellow\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        assert!(!json.contains("foul.advantage"), "{json}");
        let back: MatchEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }
}
