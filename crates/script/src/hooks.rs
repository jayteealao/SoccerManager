//! The three engine hooks, each backed by one script function:
//!
//! - `decide(ctx)` returns a map of offsets to the carrier's option scores, with any of the
//!   keys `pass`, `dribble`, `shoot`, `clear`, and `hold`, each a number from -10 to 10; a
//!   missing key is 0, and `()` keeps the engine's scores;
//! - `card(ctx, native)` gets the referee's card (`"none"`, `"yellow"`, `"second-yellow"`, or
//!   `"red"`) and returns `"none"`, `"yellow"`, or `"red"`, or `()` to keep it;
//! - `line(ctx, native)` gets the commentator's line and returns a line of at most 280
//!   characters, or `()` to keep it.
//!
//! Any other return is a script error: the call is aborted and the engine keeps its choice.
//!
//! Each hook holds its match's [`WatchdogMark`]; a call past the wall-clock limit adds a hit
//! to it and keeps its result.
//!
//! The match loop reaches these hooks only through the engine's hook slots
//! (`engine.hook.decision`, `engine.hook.rule`, `engine.hook.commentary`): with a slot switched
//! off, the pack's hook for that point is never called. [`RHAI_ADAPTER_CARD`] describes this
//! runtime with the engine's module card; the engine's registry cannot list it, because this
//! crate depends on the engine.

use std::sync::Arc;

use engine::Card;
use engine::modules::ModuleCard;
use engine::plugin::{
    CommentaryHook, DecisionContext, DecisionHook, FoulContext, HookOutcome, LineContext,
    OptionOffsets, RuleHook, WatchdogMark,
};
use rhai::Dynamic;

use crate::sandbox::Sandbox;

/// The card of the Rhai adapter: the script runtime behind the three hook slots.
pub const RHAI_ADAPTER_CARD: ModuleCard = ModuleCard {
    purpose: "Runs a script pack's decide, card, and line functions in the Rhai sandbox as the decision, rule, and commentary hooks, under the operation budget and the wall-clock backstop.",
    inputs: "The decision context, the foul context with the referee's card, and the line context with the commentator's line, as the hook slots build them.",
    outputs: "Offsets to the carrier's option scores, a replacement card, or a replacement line; or keep, aborted, or denied; and a hit on the match's watchdog mark for a call past the wall-clock limit.",
    tuning: &["none"],
    calibration: "none: script runtime, no realism band",
    keys: &[],
};

/// The largest offset, either way, a decision hook may return.
pub const MAX_OFFSET: f64 = 10.0;
/// The longest line, in characters, a commentary hook may return.
pub const MAX_LINE: usize = 280;

/// The script function behind each hook, and its parameter count.
pub const DECIDE_FN: (&str, usize) = ("decide", 1);
pub const CARD_FN: (&str, usize) = ("card", 2);
pub const LINE_FN: (&str, usize) = ("line", 2);

/// Turns a failed call into the matching outcome of another type.
fn failed<T>(err: HookOutcome<()>) -> HookOutcome<T> {
    match err {
        HookOutcome::Denied(why) => HookOutcome::Denied(why),
        HookOutcome::Aborted(why) => HookOutcome::Aborted(why),
        HookOutcome::Value(()) | HookOutcome::Keep => HookOutcome::Keep,
    }
}

pub struct ScriptDecisionHook {
    pub sandbox: Arc<Sandbox>,
    /// The watchdog mark of the match this hook plays in.
    pub mark: WatchdogMark,
}

impl DecisionHook for ScriptDecisionHook {
    fn adjust(&mut self, ctx: &DecisionContext) -> HookOutcome<OptionOffsets> {
        match self.sandbox.call(DECIDE_FN.0, (*ctx,), &self.mark) {
            Ok(value) => offsets(value),
            Err(err) => failed(err),
        }
    }
}

/// Reads a `decide` return value.
pub fn offsets(value: Dynamic) -> HookOutcome<OptionOffsets> {
    if value.is_unit() {
        return HookOutcome::Keep;
    }
    let Some(map) = value.try_cast::<rhai::Map>() else {
        return HookOutcome::Aborted("decide must return a map of offsets or ()".into());
    };
    let mut out = OptionOffsets::default();
    for (key, v) in map {
        let slot = match key.as_str() {
            "pass" => &mut out.pass,
            "dribble" => &mut out.dribble,
            "shoot" => &mut out.shoot,
            "clear" => &mut out.clear,
            "hold" => &mut out.hold,
            other => {
                return HookOutcome::Aborted(format!(
                    "decide returned an unknown option '{other}'; use pass, dribble, shoot, clear, or hold"
                ));
            }
        };
        let number = v
            .as_float()
            .ok()
            .or_else(|| v.as_int().ok().map(|i| i as f64));
        match number {
            Some(x) if x.is_finite() && x.abs() <= MAX_OFFSET => *slot = x,
            _ => {
                return HookOutcome::Aborted(format!(
                    "the offset for {key} must be a number from -{MAX_OFFSET} to {MAX_OFFSET}"
                ));
            }
        }
    }
    HookOutcome::Value(out)
}

pub struct ScriptRuleHook {
    pub sandbox: Arc<Sandbox>,
    /// The watchdog mark of the match this hook plays in.
    pub mark: WatchdogMark,
}

impl RuleHook for ScriptRuleHook {
    fn card(&mut self, ctx: &FoulContext, native: Option<Card>) -> HookOutcome<Option<Card>> {
        let native = match native {
            None => "none",
            Some(Card::Yellow) => "yellow",
            Some(Card::SecondYellow) => "second-yellow",
            Some(Card::Red) => "red",
        };
        match self
            .sandbox
            .call(CARD_FN.0, (*ctx, native.to_string()), &self.mark)
        {
            Ok(value) => card(value),
            Err(err) => failed(err),
        }
    }
}

/// Reads a `card` return value.
pub fn card(value: Dynamic) -> HookOutcome<Option<Card>> {
    if value.is_unit() {
        return HookOutcome::Keep;
    }
    match value.into_string().as_deref() {
        Ok("none") => HookOutcome::Value(None),
        Ok("yellow") => HookOutcome::Value(Some(Card::Yellow)),
        Ok("red") => HookOutcome::Value(Some(Card::Red)),
        _ => HookOutcome::Aborted("card must return \"none\", \"yellow\", \"red\", or ()".into()),
    }
}

pub struct ScriptCommentaryHook {
    pub sandbox: Arc<Sandbox>,
    /// The watchdog mark of the match this hook plays in.
    pub mark: WatchdogMark,
}

impl CommentaryHook for ScriptCommentaryHook {
    fn line(&mut self, ctx: &LineContext, native: &str) -> HookOutcome<String> {
        match self
            .sandbox
            .call(LINE_FN.0, (*ctx, native.to_string()), &self.mark)
        {
            Ok(value) => line(value),
            Err(err) => failed(err),
        }
    }
}

/// Reads a `line` return value.
pub fn line(value: Dynamic) -> HookOutcome<String> {
    if value.is_unit() {
        return HookOutcome::Keep;
    }
    match value.into_string() {
        Ok(text) if !text.trim().is_empty() && text.chars().count() <= MAX_LINE => {
            HookOutcome::Value(text)
        }
        Ok(_) => HookOutcome::Aborted(format!("line must return 1 to {MAX_LINE} characters")),
        Err(_) => HookOutcome::Aborted("line must return a string or ()".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Map;

    fn map(pairs: &[(&str, Dynamic)]) -> Dynamic {
        let mut m = Map::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Dynamic::from_map(m)
    }

    #[test]
    fn decide_reads_offsets_and_refuses_bad_ones() {
        assert_eq!(offsets(Dynamic::UNIT), HookOutcome::Keep);
        let HookOutcome::Value(o) = offsets(map(&[
            ("shoot", Dynamic::from_float(0.4)),
            ("dribble", Dynamic::from_int(-1)),
        ])) else {
            panic!("a valid map");
        };
        assert_eq!((o.shoot, o.dribble, o.pass), (0.4, -1.0, 0.0));
        assert!(matches!(
            offsets(map(&[("shoot", Dynamic::from_float(f64::NAN))])),
            HookOutcome::Aborted(_)
        ));
        assert!(matches!(
            offsets(map(&[("shoot", Dynamic::from_float(10.5))])),
            HookOutcome::Aborted(_)
        ));
        assert!(matches!(
            offsets(map(&[("volley", Dynamic::from_float(1.0))])),
            HookOutcome::Aborted(_)
        ));
        assert!(matches!(
            offsets(Dynamic::from_int(3)),
            HookOutcome::Aborted(_)
        ));
    }

    #[test]
    fn card_reads_the_three_calls_and_keep() {
        assert_eq!(card(Dynamic::UNIT), HookOutcome::Keep);
        assert_eq!(card("none".into()), HookOutcome::Value(None));
        assert_eq!(
            card("yellow".into()),
            HookOutcome::Value(Some(Card::Yellow))
        );
        assert_eq!(card("red".into()), HookOutcome::Value(Some(Card::Red)));
        assert!(matches!(card("green".into()), HookOutcome::Aborted(_)));
    }

    #[test]
    fn line_caps_the_length() {
        assert_eq!(line(Dynamic::UNIT), HookOutcome::Keep);
        assert_eq!(line("Goal!".into()), HookOutcome::Value("Goal!".into()));
        assert!(matches!(
            line("x".repeat(281).into()),
            HookOutcome::Aborted(_)
        ));
        assert!(matches!(line("  ".into()), HookOutcome::Aborted(_)));
        assert!(matches!(
            line(Dynamic::from_int(1)),
            HookOutcome::Aborted(_)
        ));
    }
}
