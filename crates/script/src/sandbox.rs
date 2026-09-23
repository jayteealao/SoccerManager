//! The sandbox a pack's script runs in (named mechanism: sandbox limits). It is a Rhai engine
//! with these limits:
//!
//! - an operation budget per call (the pack's `limits.max_operations`), which is the
//!   deterministic budget, and a 2 ms wall-clock backstop checked every 256 operations, which
//!   only catches a slow built-in call;
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
//! under the same budget. Every hook call starts from an empty scope, so a script keeps no
//! state from one call to the next.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use engine::plugin::{DecisionContext, FoulContext, HookOutcome, LineContext};
use rhai::module_resolvers::DummyModuleResolver;
use rhai::{AST, CallFnOptions, Dynamic, Engine, EvalAltResult, FuncArgs, Scope};

/// Wall-clock time one call may take before it is stopped.
pub const CALL_BACKSTOP: Duration = Duration::from_millis(2);
/// Operations between two checks of the wall clock.
const CLOCK_EVERY: u64 = 256;
pub const MAX_CALL_LEVELS: usize = 16;
pub const MAX_EXPR_DEPTH: usize = 64;
pub const MAX_FN_EXPR_DEPTH: usize = 32;
pub const MAX_STRING: usize = 1_024;
pub const MAX_ARRAY: usize = 256;
pub const MAX_MAP: usize = 64;

/// A compiled script inside its sandbox.
pub struct Sandbox {
    engine: Engine,
    ast: AST,
    max_operations: u64,
    /// When the running call must stop, in nanoseconds after `origin`.
    deadline: Arc<AtomicU64>,
    origin: Instant,
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
        let origin = Instant::now();
        let deadline = Arc::new(AtomicU64::new(u64::MAX));
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
        let clock = Arc::clone(&deadline);
        engine.on_progress(move |ops| {
            if ops.is_multiple_of(CLOCK_EVERY) {
                let now = u64::try_from(origin.elapsed().as_nanos()).unwrap_or(u64::MAX);
                if now > clock.load(Ordering::Relaxed) {
                    return Some(Dynamic::from("time"));
                }
            }
            None
        });
        register_contexts(&mut engine);
        let ast = engine.compile(source).map_err(|e| e.to_string())?;
        let sandbox = Self {
            engine,
            ast,
            max_operations,
            deadline,
            origin,
        };
        sandbox.arm();
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
    /// A failure comes back as `Aborted` or `Denied` with the reason.
    pub fn call(&self, name: &str, args: impl FuncArgs) -> Result<Dynamic, HookOutcome<()>> {
        self.arm();
        let mut scope = Scope::new();
        let options = CallFnOptions::new().eval_ast(false).rewind_scope(true);
        self.engine
            .call_fn_with_options::<Dynamic>(options, &mut scope, &self.ast, name, args)
            .map_err(|err| self.classify(*err))
    }

    /// Sets the wall-clock deadline for the call about to start.
    fn arm(&self) {
        let limit = self.origin.elapsed() + CALL_BACKSTOP;
        let nanos = u64::try_from(limit.as_nanos()).unwrap_or(u64::MAX);
        self.deadline.store(nanos, Ordering::Relaxed);
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
            EvalAltResult::ErrorTerminated(..) => HookOutcome::Aborted(format!(
                "time budget of {} ms exceeded",
                CALL_BACKSTOP.as_millis()
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
        sandbox(body).call("run", ()).unwrap_err()
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
            .call("run", ())
            .unwrap();
        assert_eq!(out.as_int().unwrap(), 7);
    }

    #[test]
    fn top_level_statements_run_once_under_the_budget() {
        let err = Sandbox::new("loop {}", 10_000, "test").unwrap_err();
        assert!(err.contains("operation budget"), "{err}");
    }
}
