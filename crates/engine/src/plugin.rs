//! The plugin interface (named mechanism: plugin boundary). A plugin offers up to three hooks:
//! one adjusts the ball carrier's option scores, one reviews the referee's card for a foul,
//! and one rewrites a commentary line. The engine owns this interface and depends on no
//! scripting runtime; a runtime implements the traits in its own package.
//!
//! A hook never decides alone and never stops a match. When a hook fails, the engine keeps
//! its own choice, records a `script` event, logs a warning, and counts the failure. After
//! [`MAX_CONSECUTIVE_FAILURES`] failures in a row, the hook is switched off for the rest of
//! the match. No hook call draws from the match's random stream, so a hook that keeps every
//! native choice leaves the match exactly as it is without a plugin.
//!
//! A hook call that runs past the runtime's wall-clock limit is not a failure (named
//! mechanism: watchdog mark). Wall-clock time differs between machines, so a slow call keeps
//! its result, and the runtime only records the hit on the match's [`WatchdogMark`]. A match
//! with a hit is marked invalid ([`INVALID_SLOW_SCRIPT`]) and logs one `match.invalid`
//! signal; play, the failure counts, the events, and the snapshot stay as they would be
//! without the hit.
//!
//! The loop consults each hook through its hook slot (`engine.hook.decision`,
//! `engine.hook.rule`, `engine.hook.commentary`; see [`crate::hook_slots`]): the slot's module
//! builds what the hook sees, and a slot switched off never calls its hook. The hooks, their
//! failure counts, and the watchdog mark stay per-match state in [`Plugins`].

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::rules::fouls::Card;

/// The version of this interface. A pack declares the version it was written for, and a
/// loader refuses any other.
pub const PLUGIN_API_VERSION: u32 = 1;

/// Failures in a row that switch a hook off for the rest of the match.
pub const MAX_CONSECUTIVE_FAILURES: u32 = 3;

/// Hook calls of one match that ran past the wall-clock limit (named mechanism: watchdog
/// mark). Wall-clock time differs between machines, so the mark never changes play: it is
/// not a failure, not in the snapshot, and not in the gate's state inventory.
#[derive(Debug, Clone, Default)]
pub struct WatchdogMark(Arc<AtomicU32>);

impl WatchdogMark {
    /// Records one call past the limit.
    pub fn hit(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// Calls past the limit so far.
    pub fn hits(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Why a marked match is invalid.
pub const INVALID_SLOW_SCRIPT: &str = "slow script";

/// Where a hook is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPoint {
    Decision,
    Rule,
    Commentary,
}

impl HookPoint {
    /// Every hook point, in declaration order.
    pub const ALL: [HookPoint; 3] = [HookPoint::Decision, HookPoint::Rule, HookPoint::Commentary];

    /// The hook as a pack and the event contract name it.
    pub fn code(&self) -> &'static str {
        match self {
            HookPoint::Decision => "decision",
            HookPoint::Rule => "rule",
            HookPoint::Commentary => "commentary",
        }
    }
}

/// What one hook call returned.
#[derive(Debug, Clone, PartialEq)]
pub enum HookOutcome<T> {
    /// The hook's value replaces the engine's.
    Value(T),
    /// The hook keeps the engine's choice.
    Keep,
    /// The call ran out of budget or failed; the text says why.
    Aborted(String),
    /// The call tried something the sandbox does not allow; the text names it.
    Denied(String),
}

/// How a failed hook call ended, as a `script` event reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptOutcome {
    Aborted,
    Denied,
    /// The hook failed too many times in a row and is off for the rest of the match.
    Disabled,
}

impl ScriptOutcome {
    /// The outcome as the event contract spells it.
    pub fn code(&self) -> &'static str {
        match self {
            ScriptOutcome::Aborted => "aborted",
            ScriptOutcome::Denied => "denied",
            ScriptOutcome::Disabled => "disabled",
        }
    }
}

/// What a `script` event carries: the hook, how the call ended, and an index into
/// [`Plugins::details`] for the text that explains it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptNote {
    pub hook: HookPoint,
    pub outcome: ScriptOutcome,
    pub detail: u32,
}

/// What the decision hook sees: the ball carrier and the match around it. Distances are in
/// metres; `team` is 0 for home and 1 for away; `slot` is the carrier's formation slot, 0
/// for the goalkeeper.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecisionContext {
    pub tick: u32,
    pub minute: u32,
    pub team: usize,
    pub slot: usize,
    pub goals_for: u32,
    pub goals_against: u32,
    /// Distance to the centre of the goal the carrier attacks.
    pub goal_distance: f64,
    /// Distance to the nearest opponent on the pitch.
    pub nearest_opponent: f64,
    /// How far up the pitch the carrier is, from -1 at its own goal line to 1 at the
    /// opponent's.
    pub progress: f64,
}

/// Offsets the decision hook adds to the carrier's option scores. An option the carrier does
/// not have stays unavailable whatever its offset.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OptionOffsets {
    pub pass: f64,
    pub dribble: f64,
    pub shoot: f64,
    pub clear: f64,
    pub hold: f64,
}

/// What the rule hook sees when the referee has judged a foul.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoulContext {
    pub tick: u32,
    pub minute: u32,
    /// The offender's team and formation slot.
    pub team: usize,
    pub slot: usize,
    /// Yellow cards the offender was shown before this foul.
    pub yellows: u8,
    /// The offender's aggression, 0 to 1.
    pub aggression: f64,
    /// `true` when the referee plays advantage and holds any card for the next stoppage.
    pub advantage: bool,
    /// `true` when the foul gives a penalty.
    pub penalty: bool,
}

/// What the commentary hook sees for one event that has a line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineContext {
    pub tick: u32,
    pub minute: u32,
    /// The event type as the event contract spells it, such as `goal`.
    pub kind: &'static str,
    /// The team the event belongs to, if any.
    pub team: Option<usize>,
    /// The score after the event, home first.
    pub scores: [u32; 2],
}

/// Adjusts the carrier's option scores.
pub trait DecisionHook: Send {
    fn adjust(&mut self, ctx: &DecisionContext) -> HookOutcome<OptionOffsets>;
}

/// Reviews the card the referee chose for a foul: `Value(None)` shows no card.
pub trait RuleHook: Send {
    fn card(&mut self, ctx: &FoulContext, native: Option<Card>) -> HookOutcome<Option<Card>>;
}

/// Rewrites a commentary line.
pub trait CommentaryHook: Send {
    fn line(&mut self, ctx: &LineContext, native: &str) -> HookOutcome<String>;
}

/// Counters kept for a match with a plugin. They are not in the snapshot, so a resumed
/// match counts from the resume tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScriptStats {
    /// Hook calls made, every hook.
    pub calls: u32,
    pub aborts: u32,
    pub denials: u32,
    /// Hooks switched off for failing too many times in a row.
    pub disabled: u32,
}

/// The plugin a match runs with. [`Plugins::default`] has no hook, and a match without
/// hooks runs exactly as it would without this type.
#[derive(Default)]
pub struct Plugins {
    /// The pack identity, such as `sample@1.0.0+0123456789ab`.
    pub pack: Option<String>,
    pub decision: Option<Box<dyn DecisionHook>>,
    pub rule: Option<Box<dyn RuleHook>>,
    pub commentary: Option<Box<dyn CommentaryHook>>,
    /// Ticks a decision hook's offsets stay in force before the hook is asked again. A new
    /// carrier or a stoppage asks sooner.
    pub refresh_ticks: u32,
    pub stats: ScriptStats,
    /// The text each `script` event's note points to.
    pub details: Vec<String>,
    /// Failures in a row, per hook point.
    failures: [u32; 3],
    /// Hook calls past the wall-clock limit. The hooks hold clones of it.
    watchdog: WatchdogMark,
    /// `true` once the `match.invalid` signal is logged.
    invalid_logged: bool,
}

impl std::fmt::Debug for Plugins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plugins")
            .field("pack", &self.pack)
            .field("decision", &self.decision.is_some())
            .field("rule", &self.rule.is_some())
            .field("commentary", &self.commentary.is_some())
            .field("refresh_ticks", &self.refresh_ticks)
            .field("stats", &self.stats)
            .field("slow_calls", &self.slow_calls())
            .finish()
    }
}

/// The refresh interval a pack gets when it names none.
pub const DEFAULT_REFRESH_TICKS: u32 = 25;

impl Plugins {
    /// A plugin identified as `pack` with no hook yet.
    pub fn new(pack: impl Into<String>) -> Self {
        Self {
            pack: Some(pack.into()),
            refresh_ticks: DEFAULT_REFRESH_TICKS,
            ..Self::default()
        }
    }

    /// `true` when any hook is attached.
    pub fn any(&self) -> bool {
        self.decision.is_some() || self.rule.is_some() || self.commentary.is_some()
    }

    /// Which hooks are attached now: decision, rule, commentary. A hook switched off for
    /// failing reads `false`.
    pub fn hooks_present(&self) -> [bool; 3] {
        [
            self.decision.is_some(),
            self.rule.is_some(),
            self.commentary.is_some(),
        ]
    }

    /// Failures in a row per hook point, in [`HookPoint::ALL`] order.
    pub fn failures(&self) -> [u32; 3] {
        self.failures
    }

    /// The watchdog mark of this match, for the hooks to record slow calls on. Every clone
    /// shares one count.
    pub fn watchdog(&self) -> WatchdogMark {
        self.watchdog.clone()
    }

    /// Hook calls of this match that ran past the wall-clock limit.
    pub fn slow_calls(&self) -> u32 {
        self.watchdog.hits()
    }

    /// Why the match is invalid, or `None` when no hook call ran past the wall-clock limit.
    pub fn invalid(&self) -> Option<&'static str> {
        (self.slow_calls() > 0).then_some(INVALID_SLOW_SCRIPT)
    }

    /// The text a note points to.
    pub fn detail(&self, note: &ScriptNote) -> &str {
        self.details
            .get(note.detail as usize)
            .map_or("", String::as_str)
    }

    /// Counts one call to `hook` and sorts its outcome. Returns the hook's value, if it gave
    /// one, and the notes to record: none on success, one for a failure, and a second when
    /// the failure switches the hook off, which the caller must then drop. The first slow
    /// call of the match logs `match.invalid` at `tick`; a slow call changes nothing else.
    pub(crate) fn settle<T>(
        &mut self,
        hook: HookPoint,
        outcome: HookOutcome<T>,
        tick: u32,
    ) -> (Option<T>, Vec<ScriptNote>) {
        if !self.invalid_logged && self.watchdog.hits() > 0 {
            self.invalid_logged = true;
            tracing::warn!(
                signal = "match.invalid",
                reason = INVALID_SLOW_SCRIPT,
                hook = hook.code(),
                pack = self.pack.as_deref().unwrap_or(""),
                tick,
                slow_calls = self.watchdog.hits()
            );
        }
        self.stats.calls += 1;
        let index = hook as usize;
        let (outcome, text) = match outcome {
            HookOutcome::Value(v) => {
                self.failures[index] = 0;
                return (Some(v), Vec::new());
            }
            HookOutcome::Keep => {
                self.failures[index] = 0;
                return (None, Vec::new());
            }
            HookOutcome::Aborted(text) => {
                self.stats.aborts += 1;
                (ScriptOutcome::Aborted, text)
            }
            HookOutcome::Denied(text) => {
                self.stats.denials += 1;
                (ScriptOutcome::Denied, text)
            }
        };
        let signal = match outcome {
            ScriptOutcome::Aborted => "script.aborted",
            _ => "script.denied",
        };
        tracing::warn!(
            signal,
            hook = hook.code(),
            pack = self.pack.as_deref().unwrap_or(""),
            detail = %text
        );
        let mut notes = vec![self.note(hook, outcome, text)];
        self.failures[index] += 1;
        if self.failures[index] >= MAX_CONSECUTIVE_FAILURES {
            self.stats.disabled += 1;
            let text = format!(
                "{MAX_CONSECUTIVE_FAILURES} failures in a row; the hook is off for the rest of the match"
            );
            tracing::warn!(
                signal = "script.disabled",
                hook = hook.code(),
                pack = self.pack.as_deref().unwrap_or(""),
                detail = %text
            );
            notes.push(self.note(hook, ScriptOutcome::Disabled, text));
            match hook {
                HookPoint::Decision => self.decision = None,
                HookPoint::Rule => self.rule = None,
                HookPoint::Commentary => self.commentary = None,
            }
        }
        (None, notes)
    }

    fn note(&mut self, hook: HookPoint, outcome: ScriptOutcome, text: String) -> ScriptNote {
        let detail = u32::try_from(self.details.len()).unwrap_or(u32::MAX);
        self.details.push(text);
        ScriptNote {
            hook,
            outcome,
            detail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Failing;
    impl RuleHook for Failing {
        fn card(&mut self, _: &FoulContext, _: Option<Card>) -> HookOutcome<Option<Card>> {
            HookOutcome::Aborted("boom".into())
        }
    }

    #[test]
    fn three_failures_in_a_row_switch_the_hook_off_and_a_success_resets_the_count() {
        let mut p = Plugins::new("x@1.0.0+000000000000");
        p.rule = Some(Box::new(Failing));
        let (_, notes) = p.settle::<u8>(HookPoint::Rule, HookOutcome::Aborted("a".into()), 0);
        assert_eq!(notes.len(), 1);
        p.settle::<u8>(HookPoint::Rule, HookOutcome::Denied("d".into()), 0);
        p.settle(HookPoint::Rule, HookOutcome::Value(1u8), 0);
        p.settle::<u8>(HookPoint::Rule, HookOutcome::Aborted("a".into()), 0);
        p.settle::<u8>(HookPoint::Rule, HookOutcome::Aborted("a".into()), 0);
        assert!(p.rule.is_some(), "two failures in a row keep the hook");
        let (_, notes) = p.settle::<u8>(HookPoint::Rule, HookOutcome::Aborted("a".into()), 0);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[1].outcome, ScriptOutcome::Disabled);
        assert!(p.rule.is_none());
        assert_eq!(
            p.stats,
            ScriptStats {
                calls: 6,
                aborts: 4,
                denials: 1,
                disabled: 1
            }
        );
        assert_eq!(p.detail(&notes[0]), "a");
    }

    /// Plays the same `settle` calls on two plugins, one with `hits` watchdog hits before
    /// each call, and returns both.
    fn twin_settle(outcomes: &[HookOutcome<u8>], hits: u32) -> (Plugins, Plugins) {
        let mut plain = Plugins::new("x@1.0.0+000000000000");
        plain.rule = Some(Box::new(Failing));
        let mut marked = Plugins::new("x@1.0.0+000000000000");
        marked.rule = Some(Box::new(Failing));
        for (tick, outcome) in outcomes.iter().enumerate() {
            for _ in 0..hits {
                marked.watchdog().hit();
            }
            let a = plain.settle(HookPoint::Rule, outcome.clone(), tick as u32);
            let b = marked.settle(HookPoint::Rule, outcome.clone(), tick as u32);
            assert_eq!(a, b, "a hit changes no outcome of settle ({outcome:?})");
        }
        (plain, marked)
    }

    #[test]
    fn a_watchdog_hit_is_not_a_failure() {
        let mut p = Plugins::new("x@1.0.0+000000000000");
        p.rule = Some(Box::new(Failing));
        for tick in 0..3 {
            p.watchdog().hit();
            let (value, notes) = p.settle(HookPoint::Rule, HookOutcome::Value(7u8), tick);
            assert_eq!(value, Some(7));
            assert!(notes.is_empty());
        }
        assert_eq!(p.failures(), [0, 0, 0]);
        assert_eq!(p.stats.aborts, 0);
        assert!(p.rule.is_some(), "the hook stays attached");
        assert_eq!(p.slow_calls(), 3);
        assert_eq!(p.invalid(), Some("slow script"));
        assert_eq!(
            Plugins::new("y").invalid(),
            None,
            "a fresh match starts unmarked"
        );
        assert_eq!(Plugins::default().slow_calls(), 0);
    }

    #[test]
    fn a_hit_changes_no_outcome_of_settle() {
        let each = [
            HookOutcome::Value(1u8),
            HookOutcome::Keep,
            HookOutcome::Aborted("a".into()),
            HookOutcome::Denied("d".into()),
        ];
        for outcome in &each {
            let (plain, marked) = twin_settle(std::slice::from_ref(outcome), 1);
            assert_eq!(plain.failures(), marked.failures());
            assert_eq!(plain.stats, marked.stats);
            assert_eq!(plain.details, marked.details);
            assert_eq!(plain.hooks_present(), marked.hooks_present());
            assert_eq!((plain.slow_calls(), marked.slow_calls()), (0, 1));
        }
        let aborts = vec![HookOutcome::Aborted("a".into()); 3];
        let (plain, marked) = twin_settle(&aborts, 2);
        assert!(
            plain.rule.is_none() && marked.rule.is_none(),
            "three aborts switch off"
        );
        assert_eq!(plain.stats, marked.stats);
        assert_eq!(marked.stats.disabled, 1);
        assert_eq!(plain.failures(), marked.failures());
    }

    /// A log writer the test can read back.
    #[derive(Clone, Default)]
    struct Log(Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for Log {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn match_invalid_is_logged_once() {
        let log = Log::default();
        let writer = log.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            let mut p = Plugins::new("x@1.0.0+000000000000");
            p.rule = Some(Box::new(Failing));
            p.settle::<u8>(HookPoint::Rule, HookOutcome::Keep, 1);
            // The first hit comes with `Keep`.
            p.watchdog().hit();
            p.settle::<u8>(HookPoint::Rule, HookOutcome::Keep, 2);
            p.watchdog().hit();
            p.settle(HookPoint::Rule, HookOutcome::Value(1u8), 3);
            p.watchdog().hit();
            p.settle::<u8>(HookPoint::Rule, HookOutcome::Aborted("a".into()), 4);
            p.watchdog().hit();
            p.settle::<u8>(HookPoint::Rule, HookOutcome::Denied("d".into()), 5);
        });
        let text = String::from_utf8_lossy(&log.0.lock().unwrap()).into_owned();
        let lines: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("match.invalid"))
            .collect();
        assert_eq!(lines.len(), 1, "{text}");
        assert!(lines[0].contains("slow script"), "{text}");
        assert!(lines[0].contains("tick=2"), "{text}");
    }
}
