//! The sandbox a pack's script runs in (named mechanism: sandbox limits). It is a Rhai engine
//! with these limits:
//!
//! - an operation budget per call (the pack's `limits.max_operations`), which is the
//!   deterministic budget and the only limit that stops a call;
//! - a 2 ms wall-clock limit per call (named mechanism: watchdog mark), checked once when
//!   the call returns. A call past it is not stopped and keeps its result: wall-clock time
//!   differs between machines, and a stopped call would make a slow machine play another
//!   match. The call is recorded on the match's [`WatchdogMark`], which marks the match
//!   invalid. A caller may skip the clock ([`Backstop::Never`]);
//! - 16 call levels, expression depths of 64 and 32, strings of 1,024 characters, arrays of
//!   256 items, and maps of 64 entries;
//! - no import: a resolver that refuses every module replaces the default one, which reads
//!   script files from disk;
//! - no `eval`: a pack that uses it is refused when it loads;
//! - no file or network function: Rhai has none, so a call to any function the sandbox does
//!   not expose is reported as a denial that names the function;
//! - `print` and `debug` go to the log, never to standard output, which carries records.
//!
//! The script is compiled once, and its top-level statements run once when the pack loads,
//! under the same operation budget and with no wall-clock check (there is no match to mark). Every hook call starts from an empty scope, so a script keeps no
//! state from one call to the next.

#[cfg(feature = "test-clock")]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use engine::plugin::{DecisionContext, FoulContext, HookOutcome, LineContext, WatchdogMark};
use rhai::module_resolvers::DummyModuleResolver;
use rhai::{AST, CallFnOptions, Dynamic, Engine, EvalAltResult, FuncArgs, Scope};

/// Wall-clock time one call may take before the match is marked invalid. The call is not
/// stopped.
pub const CALL_BACKSTOP: Duration = Duration::from_millis(2);
pub const MAX_CALL_LEVELS: usize = 16;
pub const MAX_EXPR_DEPTH: usize = 64;
pub const MAX_FN_EXPR_DEPTH: usize = 32;
pub const MAX_STRING: usize = 1_024;
pub const MAX_ARRAY: usize = 256;
pub const MAX_MAP: usize = 64;

/// The wall-clock limit on one call. A call past it marks the match and is not stopped;
/// the operation budget alone stops a long call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backstop {
    /// A call that runs longer than this marks the match. The default is
    /// [`CALL_BACKSTOP`].
    Wall(Duration),
    /// No clock is read and no call marks the match.
    Never,
    /// Test clock: the check at return adds 10 ms to the real elapsed time, so a timed call
    /// runs "long". `limit` is the wall limit on that clock, or `None` for no limit, as
    /// [`Backstop::Never`]. `only_call` times only the call with that number (the first call
    /// after the load is 1); `None` times every call.
    #[cfg(feature = "test-clock")]
    Skewed {
        limit: Option<Duration>,
        only_call: Option<u64>,
    },
}

impl Default for Backstop {
    fn default() -> Self {
        Backstop::Wall(CALL_BACKSTOP)
    }
}

/// A compiled script inside its sandbox.
pub struct Sandbox {
    engine: Engine,
    ast: AST,
    max_operations: u64,
    backstop: Backstop,
    /// Calls made since the load, for [`Backstop::Skewed`]'s `only_call`.
    #[cfg(feature = "test-clock")]
    calls: AtomicU64,
}

impl std::fmt::Debug for Sandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sandbox")
            .field("max_operations", &self.max_operations)
            .finish()
    }
}

impl Sandbox {
    /// Compiles `source` and runs its top-level statements once. `shown` names the pack in
    /// messages. A script that does not compile, uses `eval`, or fails at the top level is
    /// refused with the reason.
    pub fn new(source: &str, max_operations: u64, shown: &str) -> Result<Self, String> {
        Self::with_backstop(source, max_operations, shown, Backstop::default())
    }

    /// [`Sandbox::new`] with the wall-clock limit `backstop`. The limit marks a slow call
    /// ([`Sandbox::call`]); it never stops one, and the top-level statements are not timed.
    pub fn with_backstop(
        source: &str,
        max_operations: u64,
        shown: &str,
        backstop: Backstop,
    ) -> Result<Self, String> {
        let mut engine = Engine::new();
        // Refuses every import with `ErrorModuleNotFound` (rhai-1.26.1
        // src/module/resolvers/dummy.rs, read from the installed crate).
        engine.set_module_resolver(DummyModuleResolver::new());
        engine.disable_symbol("eval");
        let pack = shown.to_string();
        engine.on_print(move |text| {
            tracing::info!(signal = "script.print", pack = %pack, text);
        });
        let pack = shown.to_string();
        engine.on_debug(move |text, _, pos| {
            tracing::debug!(signal = "script.debug", pack = %pack, text, position = %pos);
        });
        engine.set_max_operations(max_operations);
        engine.set_max_call_levels(MAX_CALL_LEVELS);
        engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_FN_EXPR_DEPTH);
        engine.set_max_string_size(MAX_STRING);
        engine.set_max_array_size(MAX_ARRAY);
        engine.set_max_map_size(MAX_MAP);
        register_contexts(&mut engine);
        let ast = engine.compile(source).map_err(|e| e.to_string())?;
        let sandbox = Self {
            engine,
            ast,
            max_operations,
            backstop,
            #[cfg(feature = "test-clock")]
            calls: AtomicU64::new(0),
        };
        if let Err(err) = sandbox.engine.run_ast(&sandbox.ast) {
            return Err(match sandbox.classify(*err) {
                HookOutcome::Aborted(why) | HookOutcome::Denied(why) => why,
                _ => "the top-level statements failed".into(),
            });
        }
        Ok(sandbox)
    }

    /// `true` when the script defines `name` with `params` parameters.
    pub fn defines(&self, name: &str, params: usize) -> bool {
        self.ast
            .iter_functions()
            .any(|f| f.name == name && f.params.len() == params)
    }

    /// Calls the script function `name` with `args` from an empty scope, under the budget.
    /// A failure comes back as `Aborted` or `Denied` with the reason. When the call returns
    /// (with a value or a failure) past the wall-clock limit, it adds one hit to `mark`; the
    /// result is the same either way. `Instant` is monotonic, so a call that passed the
    /// limit at any point has passed it at return. A call that never returns is stopped by
    /// the operation budget alone.
    pub fn call(
        &self,
        name: &str,
        args: impl FuncArgs,
        mark: &WatchdogMark,
    ) -> Result<Dynamic, HookOutcome<()>> {
        let timing = self.clock_for_this_call();
        let started = timing.map(|_| Instant::now());
        let mut scope = Scope::new();
        let options = CallFnOptions::new().eval_ast(false).rewind_scope(true);
        let result = self
            .engine
            .call_fn_with_options::<Dynamic>(options, &mut scope, &self.ast, name, args);
        if let (Some((limit, skew)), Some(started)) = (timing, started)
            && started.elapsed() + skew > limit
        {
            mark.hit();
        }
        result.map_err(|err| self.classify(*err))
    }

    /// The wall limit and the clock skew for the call about to start, or `None` when this
    /// call reads no clock.
    fn clock_for_this_call(&self) -> Option<(Duration, Duration)> {
        match self.backstop {
            Backstop::Wall(limit) => Some((limit, Duration::ZERO)),
            Backstop::Never => None,
            #[cfg(feature = "test-clock")]
            Backstop::Skewed { limit, only_call } => {
                let number = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
                let timed = only_call.is_none_or(|n| n == number);
                limit
                    .filter(|_| timed)
                    .map(|l| (l, Duration::from_millis(10)))
            }
        }
    }

    /// Sorts a Rhai error into an abort or a denial with a reason a modder can act on.
    fn classify(&self, err: EvalAltResult) -> HookOutcome<()> {
        match err {
            EvalAltResult::ErrorInFunctionCall(_, _, inner, _)
            | EvalAltResult::ErrorInModule(_, inner, _) => self.classify(*inner),
            EvalAltResult::ErrorTooManyOperations(_) => HookOutcome::Aborted(format!(
                "operation budget of {} exhausted",
                self.max_operations
            )),
            EvalAltResult::ErrorModuleNotFound(path, _) => {
                HookOutcome::Denied(format!("import {path} is not allowed"))
            }
            EvalAltResult::ErrorFunctionNotFound(signature, pos) => {
                let name = signature
                    .split([' ', '('])
                    .next()
                    .unwrap_or(&signature)
                    .to_string();
                // A property the context does not have is a script error, not a denial.
                if name.starts_with("get$") || name.starts_with("set$") {
                    HookOutcome::Aborted(format!(
                        "no property {} ({pos})",
                        name.trim_start_matches("get$").trim_start_matches("set$")
                    ))
                } else {
                    HookOutcome::Denied(format!("function {name} is not available"))
                }
            }
            other => HookOutcome::Aborted(other.to_string()),
        }
    }
}

/// The hook contexts as script types, read through getters so a call builds no map.
fn register_contexts(engine: &mut Engine) {
    engine
        .register_type_with_name::<DecisionContext>("DecisionContext")
        .register_get("tick", |c: &mut DecisionContext| i64::from(c.tick))
        .register_get("minute", |c: &mut DecisionContext| i64::from(c.minute))
        .register_get("team", |c: &mut DecisionContext| c.team as i64)
        .register_get("slot", |c: &mut DecisionContext| c.slot as i64)
        .register_get("goals_for", |c: &mut DecisionContext| {
            i64::from(c.goals_for)
        })
        .register_get("goals_against", |c: &mut DecisionContext| {
            i64::from(c.goals_against)
        })
        .register_get("goal_diff", |c: &mut DecisionContext| {
            i64::from(c.goals_for) - i64::from(c.goals_against)
        })
        .register_get("goal_distance", |c: &mut DecisionContext| c.goal_distance)
        .register_get("nearest_opponent", |c: &mut DecisionContext| {
            c.nearest_opponent
        })
        .register_get("progress", |c: &mut DecisionContext| c.progress);
    engine
        .register_type_with_name::<FoulContext>("FoulContext")
        .register_get("tick", |c: &mut FoulContext| i64::from(c.tick))
        .register_get("minute", |c: &mut FoulContext| i64::from(c.minute))
        .register_get("team", |c: &mut FoulContext| c.team as i64)
        .register_get("slot", |c: &mut FoulContext| c.slot as i64)
        .register_get("yellows", |c: &mut FoulContext| i64::from(c.yellows))
        .register_get("aggression", |c: &mut FoulContext| c.aggression)
        .register_get("advantage", |c: &mut FoulContext| c.advantage)
        .register_get("penalty", |c: &mut FoulContext| c.penalty);
    engine
        .register_type_with_name::<LineContext>("LineContext")
        .register_get("tick", |c: &mut LineContext| i64::from(c.tick))
        .register_get("minute", |c: &mut LineContext| i64::from(c.minute))
        .register_get("kind", |c: &mut LineContext| c.kind.to_string())
        .register_get("team", |c: &mut LineContext| {
            c.team.map_or(-1, |t| t as i64)
        })
        .register_get("home_score", |c: &mut LineContext| i64::from(c.scores[0]))
        .register_get("away_score", |c: &mut LineContext| i64::from(c.scores[1]));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sandbox(body: &str) -> Sandbox {
        Sandbox::new(&format!("fn run() {{ {body} }}"), 10_000, "test").unwrap()
    }

    fn fails(body: &str) -> HookOutcome<()> {
        sandbox(body)
            .call("run", (), &WatchdogMark::default())
            .unwrap_err()
    }

    #[test]
    fn a_loop_aborts_on_the_operation_budget() {
        assert_eq!(
            fails("loop {}"),
            HookOutcome::Aborted("operation budget of 10000 exhausted".into())
        );
    }

    #[test]
    fn an_import_is_denied() {
        assert_eq!(
            fails(r#"import "x" as y; 1"#),
            HookOutcome::Denied("import x is not allowed".into())
        );
    }

    #[test]
    fn an_unknown_function_is_denied_naming_it() {
        assert_eq!(
            fails(r#"http_get("u")"#),
            HookOutcome::Denied("function http_get is not available".into())
        );
    }

    #[test]
    fn eval_is_refused_at_load() {
        let err = Sandbox::new(r#"fn run() { eval("1") }"#, 10_000, "test").unwrap_err();
        assert!(err.contains("eval"), "{err}");
    }

    #[test]
    fn a_long_string_is_refused() {
        let HookOutcome::Aborted(why) = fails(r#"let s = "x"; for i in 0..11 { s += s; } s"#)
        else {
            panic!("a long string must abort");
        };
        assert!(why.contains("Length of string"), "{why}");
    }

    #[test]
    fn print_returns_normally_and_writes_nothing_to_stdout() {
        // Standard output is checked end to end in the command-line test; here the call
        // succeeds with print routed away.
        let out = sandbox(r#"print("x"); debug("y"); 7"#)
            .call("run", (), &WatchdogMark::default())
            .unwrap();
        assert_eq!(out.as_int().unwrap(), 7);
    }

    #[test]
    fn top_level_statements_run_once_under_the_budget() {
        let err = Sandbox::new("loop {}", 10_000, "test").unwrap_err();
        assert!(err.contains("operation budget"), "{err}");
    }

    fn skewed(body: &str, only_call: Option<u64>) -> Sandbox {
        Sandbox::with_backstop(
            &format!("fn run() {{ {body} }}"),
            10_000,
            "test",
            Backstop::Skewed {
                limit: Some(CALL_BACKSTOP),
                only_call,
            },
        )
        .unwrap()
    }

    #[test]
    fn a_slow_call_marks_once_whatever_its_operation_count() {
        // Bodies around the old in-call check interval of 256 operations: the mark comes
        // from the one check at return, so it cannot depend on where a count stops.
        for n in [1_i64, 255, 256, 257] {
            let body = if n == 1 {
                "1".to_string()
            } else {
                format!("let x = 0; for i in 0..{n} {{ x += 1; }} x")
            };
            let sandbox = skewed(&body, None);
            let mark = WatchdogMark::default();
            let out = sandbox.call("run", (), &mark).unwrap();
            assert_eq!(out.as_int().unwrap(), n, "the call keeps its value");
            assert_eq!(mark.hits(), 1, "exactly one hit for a body of {n}");
        }
    }

    #[test]
    fn an_operation_budget_abort_still_aborts_and_marks() {
        let mark = WatchdogMark::default();
        let err = skewed("loop {}", None).call("run", (), &mark).unwrap_err();
        assert_eq!(
            err,
            HookOutcome::Aborted("operation budget of 10000 exhausted".into())
        );
        assert_eq!(mark.hits(), 1);
    }

    #[test]
    fn only_the_named_call_is_slow() {
        let sandbox = skewed("7", Some(2));
        let mark = WatchdogMark::default();
        let _ = sandbox.call("run", (), &mark).unwrap();
        assert_eq!(mark.hits(), 0, "call 1 is untimed");
        let _ = sandbox.call("run", (), &mark).unwrap();
        assert_eq!(mark.hits(), 1, "call 2 is slow");
        let _ = sandbox.call("run", (), &mark).unwrap();
        assert_eq!(mark.hits(), 1, "call 3 is untimed");
    }

    #[test]
    fn no_mark_without_a_hit() {
        for backstop in [Backstop::Wall(Duration::from_secs(60)), Backstop::Never] {
            let sandbox =
                Sandbox::with_backstop("fn run() { 7 }", 10_000, "test", backstop).unwrap();
            let mark = WatchdogMark::default();
            for _ in 0..10 {
                let _ = sandbox.call("run", (), &mark).unwrap();
            }
            assert_eq!(mark.hits(), 0, "{backstop:?}");
        }
    }
}
