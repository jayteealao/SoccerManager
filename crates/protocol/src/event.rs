//! Match events: the `match-event` record kind the observability contract reserves. The
//! engine produces kick-off, goal, half-time, full-time, the law events (offside, foul, card,
//! and every restart), injuries, substitutions, the AI manager's choices, the verdict on a
//! queued change, and a script hook that failed.

use serde::{Deserialize, Serialize};

use crate::command::{ChangeKind, ChangeState};

/// Simulated ticks in one minute of play (50 ticks per second). This crate does not depend on
/// the engine; a test in the command-line crate pins this to the engine's clock.
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
    Injury,
    Substitution,
    AiDecision,
    /// A script pack's hook failed or was switched off; play went on with the engine's own
    /// choice.
    Script,
}

impl EventType {
    /// Every event type, in declaration order.
    pub const ALL: [EventType; 17] = [
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
        EventType::Injury,
        EventType::Substitution,
        EventType::AiDecision,
        EventType::Script,
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
            EventType::Injury => "injury",
            EventType::Substitution => "substitution",
            EventType::AiDecision => "ai-decision",
            EventType::Script => "script",
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
    /// The tick an applied change took effect on.
    #[serde(
        rename = "change.applied_tick",
        skip_serializing_if = "Option::is_none"
    )]
    pub change_applied_tick: Option<u32>,
    /// On `ai-decision`: the AI manager's choice, as a short code.
    #[serde(rename = "ai.decision", skip_serializing_if = "Option::is_none")]
    pub ai_decision: Option<String>,
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
    /// One English commentary line, on every play event; absent on `tactics-change`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commentary: Option<String>,
    /// Additive extra, on the `half-time` break before an extra-time period and on its
    /// `kick-off`: the period that starts, counted from 0 (2 and 3 are extra time).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub period: Option<u32>,
    /// Additive extra, on a shoot-out `penalty` event: the kicking team's round, from 1.
    #[serde(
        rename = "shootout.round",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub shootout_round: Option<u32>,
    /// Additive extra, on the outcome event of a shoot-out kick: `true` when it scored.
    #[serde(
        rename = "shootout.scored",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub shootout_scored: Option<bool>,
    /// Additive extra, on the outcome event of a shoot-out kick and on `full-time` after a
    /// shoot-out: the shoot-out score, home first.
    #[serde(
        rename = "shootout.scores",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub shootout_scores: Option<[u32; 2]>,
    /// Additive extra, on `full-time` of a knockout match: `regulation`, `extra-time`, or
    /// `shoot-out`.
    #[serde(
        rename = "result.decided_by",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub decided_by: Option<String>,
    /// Additive extra, on the first `kick-off` of a match that runs a script pack: the pack
    /// identity, `id@version+hash`.
    #[serde(
        rename = "script.pack",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub script_pack: Option<String>,
    /// Additive extra, on a `script` event: the hook that failed, `decision`, `rule`, or
    /// `commentary`.
    #[serde(
        rename = "script.hook",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub script_hook: Option<String>,
    /// Additive extra, on a `script` event: `aborted`, `denied`, or `disabled`.
    #[serde(
        rename = "script.outcome",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub script_outcome: Option<String>,
    /// Additive extra, on a `script` event: why the call failed, such as the budget it ran
    /// out of or the import or function the sandbox denied.
    #[serde(
        rename = "script.detail",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub script_detail: Option<String>,
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
            change_applied_tick: None,
            ai_decision: None,
            player_id: None,
            player_secondary_id: None,
            card_kind: None,
            foul_advantage: None,
            minute_added: None,
            added_time_s: None,
            commentary: None,
            period: None,
            shootout_round: None,
            shootout_scored: None,
            shootout_scores: None,
            decided_by: None,
            script_pack: None,
            script_hook: None,
            script_outcome: None,
            script_detail: None,
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

    /// The club the event belongs to.
    pub fn team(mut self, id: Option<String>) -> Self {
        self.team_id = id;
        self
    }

    /// The tick a queued change was queued on, when it differs from the event's tick.
    pub fn queued_at(mut self, tick: u32) -> Self {
        self.change_queued_tick = Some(tick);
        self
    }

    /// The tick an applied change took effect on.
    pub fn applied_tick(mut self, tick: Option<u32>) -> Self {
        self.change_applied_tick = tick;
        self
    }

    /// The AI manager's choice.
    pub fn ai_decision(mut self, code: Option<String>) -> Self {
        self.ai_decision = code;
        self
    }

    /// The commentary line.
    pub fn commentary(mut self, line: Option<String>) -> Self {
        self.commentary = line;
        self
    }

    /// The period an extra-time break or kick-off starts.
    pub fn period(mut self, period: Option<u32>) -> Self {
        self.period = period;
        self
    }

    /// A shoot-out kick: the round, and on its outcome whether it scored and the shoot-out
    /// score; the score alone at full time after a shoot-out.
    pub fn shootout(
        mut self,
        round: Option<u32>,
        scored: Option<bool>,
        scores: Option<[u32; 2]>,
    ) -> Self {
        self.shootout_round = round;
        self.shootout_scored = scored;
        self.shootout_scores = scores;
        self
    }

    /// How a knockout match was decided, at full time.
    pub fn decided_by(mut self, decided_by: Option<String>) -> Self {
        self.decided_by = decided_by;
        self
    }

    /// The script pack the match runs, on its first kick-off.
    pub fn script_pack(mut self, pack: Option<String>) -> Self {
        self.script_pack = pack;
        self
    }

    /// A failed script hook: the hook, the outcome, and why.
    pub fn script(mut self, hook: &str, outcome: &str, detail: &str) -> Self {
        self.script_hook = Some(hook.to_string());
        self.script_outcome = Some(outcome.to_string());
        self.script_detail = Some(detail.to_string());
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
        .card(Some(CardKind::SecondYellow))
        .commentary(Some("Second yellow card! Sent off.".into()));
        let json = serde_json::to_string(&e).unwrap();
        for key in [
            "\"event.type\":\"card\"",
            "\"minute\":45",
            "\"minute.added\":2",
            "\"player.id\":\"p-club-b-04\"",
            "\"card.kind\":\"second-yellow\"",
            "\"commentary\":\"Second yellow card! Sent off.\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        assert!(!json.contains("foul.advantage"), "{json}");
        let back: MatchEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn an_applied_change_names_its_club_and_both_ticks() {
        let e = MatchEvent::change(
            "0123456789abcdef0123456789abcdef",
            "000000000000002a-1",
            9_100,
            [0, 1],
            ChangeOutcome {
                kind: Some(ChangeKind::Tactics),
                queue_id: Some("q-9000-0".into()),
                state: ChangeState::Applied,
                rejected_reason: None,
            },
        )
        .team(Some("club-a".into()))
        .queued_at(9_000)
        .applied_tick(Some(9_100));
        let json = serde_json::to_string(&e).unwrap();
        for key in [
            "\"event.type\":\"tactics-change\"",
            "\"team.id\":\"club-a\"",
            "\"change.queued_tick\":9000",
            "\"change.applied_tick\":9100",
            "\"change.state\":\"applied\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        let back: MatchEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn an_ai_decision_carries_its_code() {
        let e = MatchEvent::play(
            "0123456789abcdef0123456789abcdef",
            "000000000000002a-1",
            216_000,
            EventType::AiDecision,
            Some("club-b".into()),
            [1, 0],
        )
        .ai_decision(Some("mentality-up-trailing".into()));
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"event.type\":\"ai-decision\""), "{json}");
        assert!(
            json.contains("\"ai.decision\":\"mentality-up-trailing\""),
            "{json}"
        );
    }
}
