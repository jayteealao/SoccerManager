//! Match events: the `match-event` record kind the observability contract reserves. This
//! slice produces kick-off, goal, full-time, and the verdict on a queued change; the
//! match-rules and commentary slices add the rest.

use serde::{Deserialize, Serialize};

use crate::command::{ChangeKind, ChangeState};

/// Simulated ticks in one minute of play (50 ticks per second).
pub const TICKS_PER_MINUTE: u32 = 50 * 60;

/// The event types this build emits. The contract's enumeration is wider; a later slice
/// fills it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventType {
    KickOff,
    Goal,
    FullTime,
    TacticsChange,
}

impl EventType {
    /// The value as the contract writes it.
    pub fn code(&self) -> &'static str {
        match self {
            EventType::KickOff => "kick-off",
            EventType::Goal => "goal",
            EventType::FullTime => "full-time",
            EventType::TacticsChange => "tactics-change",
        }
    }
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
}

impl MatchEvent {
    /// A play event: kick-off, goal, or full-time.
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
        }
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
        for t in [
            EventType::KickOff,
            EventType::Goal,
            EventType::FullTime,
            EventType::TacticsChange,
        ] {
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, format!("\"{}\"", t.code()));
        }
    }
}
