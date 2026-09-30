//! The plugin hook calls of the central loop: the decision hook's offsets for the carrier,
//! the rule hook's review of a card, and the commentary hook's line. Each call settles its
//! outcome through [`crate::plugin::Plugins::settle`] and records any `script` notes; no
//! hook call draws from the match's random stream.

use serde_json::json;

use crate::decision::offsets_json;
use crate::pitch;
use crate::player::Player;
use crate::plugin::{DecisionContext, HookPoint, OptionOffsets, ScriptNote};
use crate::rules::fouls::Card;
use crate::sim::{EngineEvent, EngineEventKind, EventDetail, ScriptCache, Simulation};
use crate::trace::Point;

impl Simulation {
    /// The decision hook's offsets for carrier `c`, or `None` without a decision hook. The
    /// hook is asked again for a new carrier, after a stoppage, and when the pack's refresh
    /// interval has passed; in between, the cached offsets apply. A failed call gives zero
    /// offsets until the next refresh. Nothing here draws from the random stream.
    pub(crate) fn script_offsets(&mut self, c: usize) -> Option<OptionOffsets> {
        self.plugins.decision.as_ref()?;
        if let Some(cache) = self.script_cache
            && cache.carrier == c
            && self.tick < cache.until
        {
            if self.trace_on() {
                self.trace_point(
                    Point::ScriptDecision,
                    json!({"carrier": c, "cached": true, "offsets": offsets_json(&cache.offsets)}),
                );
            }
            return Some(cache.offsets);
        }
        let ctx = self.decision_context(c);
        let outcome = self.plugins.decision.as_mut()?.adjust(&ctx);
        let (value, notes) = self.plugins.settle(HookPoint::Decision, outcome, self.tick);
        let failed = value.is_none();
        let note_count = notes.len();
        let switched_off = self.plugins.decision.is_none();
        self.push_script_notes(notes);
        let offsets = value.unwrap_or_default();
        if self.trace_on() {
            let result = if !failed {
                "value"
            } else if switched_off {
                "switched_off"
            } else {
                "failed"
            };
            self.trace_point(
                Point::ScriptDecision,
                json!({
                    "carrier": c,
                    "cached": false,
                    "offsets": offsets_json(&offsets),
                    "result": result,
                    "notes": note_count,
                }),
            );
        }
        self.script_cache = Some(ScriptCache {
            carrier: c,
            until: self.tick.saturating_add(self.plugins.refresh_ticks.max(1)),
            offsets,
        });
        Some(offsets)
    }

    /// What the decision hook sees about carrier `c`.
    pub(crate) fn decision_context(&self, c: usize) -> DecisionContext {
        let carrier = self.players[c];
        let team = carrier.team;
        let side = &self.teams[team];
        let nearest_opponent = self
            .players
            .iter()
            .filter(|p| p.team != team && p.active())
            .map(|p| (p.pos - carrier.pos).length())
            .fold(f64::INFINITY, f64::min);
        DecisionContext {
            tick: self.tick,
            minute: self.referee.clock.minute(self.tick).0,
            team,
            slot: carrier.slot,
            goals_for: self.summary.goals[team],
            goals_against: self.summary.goals[1 - team],
            goal_distance: (side.target_goal() - carrier.pos).length(),
            nearest_opponent: if nearest_opponent.is_finite() {
                nearest_opponent
            } else {
                pitch::HALF_LENGTH * 2.0
            },
            progress: (carrier.pos.x * side.attack_x / pitch::HALF_LENGTH).clamp(-1.0, 1.0),
        }
    }

    /// Offers `card`, the card the referee chose for a foul by `offender`, to the rule hook,
    /// and returns the card to show. Without a rule hook the card comes back as it went in.
    pub(crate) fn rule_card(
        &mut self,
        offender: Player,
        advantage: bool,
        penalty: bool,
        mut card: Option<Card>,
    ) -> Option<Card> {
        if let Some(hook) = self.plugins.rule.as_mut() {
            let ctx = crate::plugin::FoulContext {
                tick: self.tick,
                minute: self.referee.clock.minute(self.tick).0,
                team: offender.team,
                slot: offender.slot,
                yellows: offender.yellow,
                aggression: offender.derived.aggression,
                advantage,
                penalty,
            };
            let outcome = hook.card(&ctx, card);
            let (value, notes) =
                self.plugins
                    .settle(crate::plugin::HookPoint::Rule, outcome, self.tick);
            self.push_script_notes(notes);
            if let Some(scripted) = value {
                card = scripted;
            }
        }
        card
    }

    /// Offers `native`, the line the commentator chose for `event`, to the commentary hook.
    /// Returns the line to use and, when the hook failed, the `script` events to record on
    /// the event's tick. Without a commentary hook the line comes back as it went in.
    pub fn offer_line(
        &mut self,
        event: &EngineEvent,
        native: Option<String>,
    ) -> (Option<String>, Vec<EngineEvent>) {
        let (Some(native), Some(hook)) = (native.as_deref(), self.plugins.commentary.as_mut())
        else {
            return (native, Vec::new());
        };
        let ctx = crate::plugin::LineContext {
            tick: event.tick,
            minute: event.minute,
            kind: event.kind.code(),
            team: event.team,
            scores: event.scores,
        };
        let outcome = hook.line(&ctx, native);
        let (line, notes) =
            self.plugins
                .settle(crate::plugin::HookPoint::Commentary, outcome, event.tick);
        let events = notes
            .into_iter()
            .map(|note| self.script_event_at(event.tick, note))
            .collect();
        (Some(line.unwrap_or_else(|| native.to_string())), events)
    }

    /// Records the notes a hook call produced as `script` events on the tick this step
    /// produces.
    pub(crate) fn push_script_notes(&mut self, notes: Vec<ScriptNote>) {
        for note in notes {
            let event = self.script_event_at(self.tick + 1, note);
            self.events.push(event);
        }
    }

    fn script_event_at(&self, tick: u32, note: ScriptNote) -> EngineEvent {
        let mut event = self.event_at(tick, EngineEventKind::Script, None);
        event.detail = Some(EventDetail::Script(note));
        event
    }
}
