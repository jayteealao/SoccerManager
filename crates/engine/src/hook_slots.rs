//! The hook slots (`engine.hook.decision`, `engine.hook.rule`, `engine.hook.commentary`):
//! the adapters between the central loop and the plugin hooks of [`crate::plugin`].
//!
//! A hook is per-match state: it takes `&mut self` and holds the match's watchdog mark, so
//! it cannot be a shared, stateless module. The loop keeps the hooks, their failure counts,
//! and the watchdog mark in [`crate::plugin::Plugins`], and consults a hook only through its
//! slot: the slot's module builds what the hook sees, and the off version returns `None`, so
//! the loop consults no hook for that point and the engine's own choice stands. A module
//! here reads the view only and draws nothing, so its card owns no action key.

use crate::modules::{
    CommentaryHookModule, DecisionHookModule, MatchView, ModuleCard, RuleHookModule,
};
use crate::plugin::{DecisionContext, FoulContext, LineContext};
use crate::sim::EngineEvent;

/// Version 1 of the decision hook's slot: the carrier and the match around it.
pub struct DecisionHookV1;

impl DecisionHookModule for DecisionHookV1 {
    fn context(&self, view: &MatchView<'_>, c: usize) -> Option<DecisionContext> {
        let players = view.players();
        let carrier = players[c];
        let team = carrier.team;
        let side = &view.teams()[team];
        let tick = view.tick();
        let goals = view.summary().goals;
        let nearest_opponent = players
            .iter()
            .filter(|p| p.team != team && p.active())
            .map(|p| (p.pos - carrier.pos).length())
            .fold(f64::INFINITY, f64::min);
        Some(DecisionContext {
            tick,
            minute: view.referee().clock.minute(tick).0,
            team,
            slot: carrier.slot,
            goals_for: goals[team],
            goals_against: goals[1 - team],
            goal_distance: (side.target_goal() - carrier.pos).length(),
            nearest_opponent: if nearest_opponent.is_finite() {
                nearest_opponent
            } else {
                view.pitch().half_length() * 2.0
            },
            progress: (carrier.pos.x * side.attack_x / view.pitch().half_length()).clamp(-1.0, 1.0),
        })
    }
}

pub const DECISION_HOOK_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Builds what an attached decision hook sees about the ball carrier, so the loop can ask the hook for offsets to the carrier's option scores.",
    inputs: "The carrier's team, slot, and position; the positions and status of the opponents; the team's attack direction and target goal; the score; the tick and the match clock.",
    outputs: "The decision context for the carrier, or none.",
    tuning: &["none"],
    calibration: "none: plugin hook adapter, no realism band",
    keys: &[],
};

/// The decision hook's off version: the loop consults no decision hook.
pub struct DecisionHookOff;

impl DecisionHookModule for DecisionHookOff {
    fn context(&self, _: &MatchView<'_>, _: usize) -> Option<DecisionContext> {
        None
    }
}

pub const DECISION_HOOK_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "Consults no decision hook: the decision maker's own option scores stand.",
    inputs: "Nothing.",
    outputs: "None.",
    tuning: &["none"],
    calibration: "none: off version, the hook is not consulted",
    keys: &[],
};

/// Version 1 of the rule hook's slot: the offender and the referee's call.
pub struct RuleHookV1;

impl RuleHookModule for RuleHookV1 {
    fn context(
        &self,
        view: &MatchView<'_>,
        offender: usize,
        advantage: bool,
        penalty: bool,
    ) -> Option<FoulContext> {
        let offender = view.player(offender);
        let tick = view.tick();
        // The hook's aggression stays the rating as a share, as hooks saw it before the
        // contract: tenths / 200.
        let aggression = view.attributes().index("aggression").map_or(0.5, |i| {
            crate::contract::share_of(offender.attributes.get(i))
        });
        Some(FoulContext {
            tick,
            minute: view.referee().clock.minute(tick).0,
            team: offender.team,
            slot: offender.slot,
            yellows: offender.yellow,
            aggression,
            advantage,
            penalty,
        })
    }
}

pub const RULE_HOOK_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Builds what an attached rule hook sees when the referee has judged a foul, so the loop can ask the hook to review the card.",
    inputs: "The offender's team, slot, yellow cards, and aggression; the tick and the match clock; whether the referee plays advantage; whether the foul gives a penalty.",
    outputs: "The foul context for the offender, or none.",
    tuning: &["none"],
    calibration: "none: plugin hook adapter, no realism band",
    keys: &[],
};

/// The rule hook's off version: the loop consults no rule hook.
pub struct RuleHookOff;

impl RuleHookModule for RuleHookOff {
    fn context(&self, _: &MatchView<'_>, _: usize, _: bool, _: bool) -> Option<FoulContext> {
        None
    }
}

pub const RULE_HOOK_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "Consults no rule hook: the referee's own card stands.",
    inputs: "Nothing.",
    outputs: "None.",
    tuning: &["none"],
    calibration: "none: off version, the hook is not consulted",
    keys: &[],
};

/// Version 1 of the commentary hook's slot: the event a line is for.
pub struct CommentaryHookV1;

impl CommentaryHookModule for CommentaryHookV1 {
    fn context(&self, _: &MatchView<'_>, event: &EngineEvent) -> Option<LineContext> {
        Some(LineContext {
            tick: event.tick,
            minute: event.minute,
            kind: event.kind.code(),
            team: event.team,
            scores: event.scores,
        })
    }
}

pub const COMMENTARY_HOOK_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Builds what an attached commentary hook sees for an event that has a line, so the loop can ask the hook to rewrite the line.",
    inputs: "The event: its tick, minute, kind, team, and the score after it.",
    outputs: "The line context for the event, or none.",
    tuning: &["none"],
    calibration: "none: plugin hook adapter, no realism band",
    keys: &[],
};

/// The commentary hook's off version: the loop consults no commentary hook.
pub struct CommentaryHookOff;

impl CommentaryHookModule for CommentaryHookOff {
    fn context(&self, _: &MatchView<'_>, _: &EngineEvent) -> Option<LineContext> {
        None
    }
}

pub const COMMENTARY_HOOK_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "Consults no commentary hook: the commentator's own line stands.",
    inputs: "Nothing.",
    outputs: "None.",
    tuning: &["none"],
    calibration: "none: off version, the hook is not consulted",
    keys: &[],
};
