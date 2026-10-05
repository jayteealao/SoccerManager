//! The plugin hook calls of the central loop: the decision hook's offsets for the carrier,
//! the rule hook's review of a card, and the commentary hook's line. The loop consults each
//! hook only through its hook slot (`engine.hook.decision`, `engine.hook.rule`,
//! `engine.hook.commentary`): the slot's module builds what the hook sees, and a slot
//! switched off returns nothing, so its hook is never called. Every presence check comes
//! first, so a match without a hook asks no module. Each call settles its outcome through
//! [`crate::plugin::Plugins::settle`] and records any `script` notes; no hook call draws
//! from the match's random stream.

use serde_json::json;

use crate::decision::offsets_json;
use crate::plugin::{HookPoint, OptionOffsets, ScriptNote};
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
        let module = self.config.modules.decision_hook;
        let ctx = module.context(&self.view(), c)?;
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

    /// Offers `card`, the card the referee chose for a foul by player `offender`, to the rule
    /// hook, and returns the card to show. Without a rule hook, or with its slot switched
    /// off, the card comes back as it went in.
    pub(crate) fn rule_card(
        &mut self,
        offender: usize,
        advantage: bool,
        penalty: bool,
        mut card: Option<Card>,
    ) -> Option<Card> {
        if self.plugins.rule.is_none() {
            return card;
        }
        let module = self.config.modules.rule_hook;
        let context = module.context(&self.view(), offender, advantage, penalty);
        if let (Some(ctx), Some(hook)) = (context, self.plugins.rule.as_mut()) {
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
    /// the event's tick. Without a commentary hook, or with its slot switched off, the line
    /// comes back as it went in.
    pub fn offer_line(
        &mut self,
        event: &EngineEvent,
        native: Option<String>,
    ) -> (Option<String>, Vec<EngineEvent>) {
        let (Some(text), true) = (native.as_deref(), self.plugins.commentary.is_some()) else {
            return (native, Vec::new());
        };
        let module = self.config.modules.commentary_hook;
        let context = module.context(&self.view(), event);
        let (Some(ctx), Some(hook)) = (context, self.plugins.commentary.as_mut()) else {
            return (native, Vec::new());
        };
        let outcome = hook.line(&ctx, text);
        let (line, notes) =
            self.plugins
                .settle(crate::plugin::HookPoint::Commentary, outcome, event.tick);
        let events = notes
            .into_iter()
            .map(|note| self.script_event_at(event.tick, note))
            .collect();
        (Some(line.unwrap_or_else(|| text.to_string())), events)
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
