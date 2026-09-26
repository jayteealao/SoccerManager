---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: scripting-runtime
status: complete
stage-number: 4
created-at: "2026-09-22T22:30:09Z"
updated-at: "2026-09-22T22:30:09Z"
metric-files-to-touch: 45
metric-step-count: 17
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, modding, scripting, sandbox, rhai, deferred, rim-6]
stack-source: confirmed
steering-honored:
  - "no visual change to the page: the new script event does not stop play and the page ignores it, so the design direction in steer.md is untouched"
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-scripting-runtime.md
  shape: 02-shape.md
  benchmark: 05c-benchmark.md
  observability: ../../observability.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md, 04-plan-commentary.md, 04-plan-calibration.md]
  implement: 05-implement-scripting-runtime.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine scripting-runtime"
---

# Plan: Scripting Runtime for Modding

## The Plan

Modders can change the data files today, but no code of theirs can run. The shape promised a plugin boundary, and that boundary exists only as a sentence: the source has no plugin trait, no hook, and no script (`grep -i plugin crates/` returns nothing, run this session). The carrier decides every tick, a product-owner choice (`crates/engine/src/decision.rs:3`, `tuning.rs:155`). A match runs about 280,000 ticks, and the latest recorded baseline is 418.8 ms of processor time per match (`05c-benchmark.md`, commit `837cb5c`). This slice is built after `tactics-and-ai` and `calibration`, so its budget is the calibration baseline, which implement measures at step 1.

Four decisions set the build. First, the runtime is Rhai 1.26 (MIT OR Apache-2.0, released 2026-09-10). It is pure Rust. No C compiler is on this machine (`where cl` and `where clang` find nothing), and that rules out the Lua and QuickJS bindings, which compile C. Rhai also has a deterministic operation budget. Second, the engine owns a runtime-free plugin interface, `crates/engine/src/plugin.rs`. A new `script` crate implements it, so the engine and a match without a pack do not change. Third, a script does not run every tick. It returns offsets to the native option scores. The engine caches them and asks again when the carrier changes, after a stoppage, or every 25 ticks. Decisions still run every tick, and one call per refresh fits the budget. Fourth, the pack's hash is folded into the existing content hash. The snapshot layout stays the same, and a resume with a different pack is refused by the check that already exists.

Implement touches 45 files (23 new, 22 modified) in 17 steps. A modder can then write a script that changes decisions, cards, and commentary, and the slice clears RIM-6. The top risk is the tick budget. Step 2 measures the cost of one call before anything else is built, and stops for the product owner if one call costs more than 3 microseconds.

## Current State

- **No plugin interface in code.** The shape says "the engine defines the interface a scripting runtime will call" (`02-shape.md:58`). The data slice built the data half only (`03-slice-data-schemas-generator.md:27`). A search for `plugin`, `script`, and `hook` in `crates/**/*.rs` finds only the stoppage hook (`record.rs:154-164`) and the referee's test draws (`rng.rs:21-71`).
- **Decision path.** `Simulation::decide` sets every target and calls `decide_carrier` (`decision.rs:22-94`). `decide_carrier` scores passes (`:171`), a dribble (`:178`), and a shot threshold (`:132`) today. The tactics plan turns it into five scored options (pass, dribble, shoot, clear, hold) with tuned weights (`04-plan-tactics-and-ai.md` step 8). `decision_interval_ticks` is 1 (`tuning.rs:155`, used at `sim.rs:377`).
- **Rule path.** Cards come from `fouls::card_outcome(aggression, yellows, tuning, draw)` (`rules/fouls.rs:73`) and are applied by `discipline::book` (`rules/discipline.rs:14`).
- **Commentary path.** The commentary plan puts the commentator outside the tick loop. The shared match driver `stream_run.rs` attaches one line to each mapped event (`04-plan-commentary.md:86`).
- **Events.** `EngineEventKind` has twelve kinds (`sim.rs:95-115`). `EngineEvent` is `Copy` (`sim.rs:133-156`). `MatchEvent` uses `deny_unknown_fields`, and every field must be documented (`protocol/src/event.rs:91-146`, `crates/protocol/tests/document.rs`). The rule beside `PROTOCOL_VERSION` (`protocol/src/lib.rs:21-34`) keeps version 2 for additive fields.
- **Snapshot.** The header holds a 12-byte content hash (`snapshot.rs:6-7`), and resume refuses a mismatch (`snapshot.rs:224-227`). `MatchConfig::new` computes the hash from the content (`sim.rs:38`, `:76`).
- **Threads.** The stream session runs the match on a spawned thread (`stream/src/session.rs:264`), so a hook object must be `Send`.
- **Toolchain.** The target is `x86_64-pc-windows-msvc` (`rustup show`). `where cl` and `where clang` report "Could not find", and `00-index.md` `stack.toolchains-absent` lists `clang`, `cl`, and `zig`.
- **Siblings not built yet.** `tactics-and-ai`, `commentary`, and `calibration` are planned and not implemented (`00-index.md` `slices:`). This plan relies on their planned seams, and step 1 re-checks them.

## Simplicity Ladder

| Capability | Rung | Detail |
|---|---|---|
| Embedded, sandboxed script interpreter | rung 4 new dependency | Rungs 1 to 3 do not hold: Rust has no built-in interpreter, and `Cargo.lock` has none. Rhai 1.26.1 is chosen from five candidates (Assumption A-1). Evidence: crates.io API (`https://crates.io/api/v1/crates/rhai`), license MIT OR Apache-2.0, meets NFR-5. Upstream `Cargo.toml` (`https://raw.githubusercontent.com/rhaiscript/rhai/main/Cargo.toml`) has no `cc` build dependency. |
| Operation and time budget | rung 3 reuse (of the new dependency) | `Engine::set_max_operations` plus `Engine::on_progress(Fn(u64) -> Option<Dynamic>)`. Returning `Some` ends the call with `ErrorTerminated` (`https://rhai.rs/book/safety/progress.html`, read this run). |
| No file or network access | rung 3 reuse with modification | Rhai has no file or network functions. `Engine::new()` installs a `FileModuleResolver` (upstream `src/engine.rs`, read this run), so the sandbox must replace it with a refusing resolver. |
| Pack manifest loading and validation | rung 3 reuse | `data::load_json` and the garde idiom (`crates/engine/src/data/mod.rs`), `sha2` (workspace). |
| Hook call sites | rung 3 reuse with modification | The scored-options layer from the tactics slice, `card_outcome` (`rules/fouls.rs:73`), and the commentator attachment point in `stream_run.rs`. |
| Pack identity in resume | rung 3 reuse | The snapshot's content-hash check (`snapshot.rs:224`). The pack hash is folded into `content_hash`, so no new field is added. |
| Benchmark compare | rung 3 reuse | `engine-cli bench --json` (`crates/engine-cli/src/bench.rs`), extracted by the calibration plan into `measure()`. |
| Plugin interface traits | rung 4 new code | No trait exists (search above). About 180 lines of plain traits and `Copy` context structs. |

## Applied Learnings

`.ai/solutions/INDEX.md` does not exist (`ls .ai/solutions` returned nothing). `.ai/sdlc-config.json` does not exist, so no global learnings directory applies. No applicable learnings found.

Repeat-deferral tripwire: `00-index.md` `runtime-evidence-deferrals: []`. It does not fire.

## Likely Files / Areas to Touch

- `Cargo.toml`, `Cargo.lock`: the new member and the `rhai` workspace dependency.
- `crates/script/` (new: `Cargo.toml`, `src/lib.rs`, `pack.rs`, `sandbox.rs`, `hooks.rs`, `tests/sandbox.rs`, `tests/decision.rs`, `tests/fixtures/` with five packs of two files each, `benches/hook_call.rs`): the runtime, the pack loader, the sandbox, and the criterion tests.
- `crates/engine/src/plugin.rs` (new), `lib.rs`, `sim.rs`, `decision.rs`, `rules/mod.rs`, `scenario.rs`, `observe/mod.rs`: the interface and its three call sites.
- `crates/protocol/src/event.rs`, `lib.rs`, `docs/reference/protocol.md`: the `script` event type and the `script.*` fields.
- `schemas/observability/match-event.schema.json`, `match-stats.schema.json` (from the calibration slice): the new enum value and keys.
- `crates/engine-cli/Cargo.toml`, `src/cli.rs`, `content.rs`, `stream_run.rs`, `simulate.rs`, `bench.rs`, `resume.rs`, `tests/script_cli.rs` (new): the flag, the loading, the mapping, and the command-line test.
- `crates/stream/tests/common/mod.rs`: the helper's own event mapping.
- `content/scripts/sample/pack.json`, `main.rhai`, `content/scripts/README.md` (new), `content/README.md`: the sample pack and the modder reference.

## Proposed Change Strategy

**The plugin interface (named mechanism: shape "Plugin boundary").** `crates/engine/src/plugin.rs` defines `PLUGIN_API_VERSION = 1` and three traits. `DecisionHook::adjust(&mut self, &DecisionContext) -> HookOutcome<OptionOffsets>`. `RuleHook::card(&mut self, &FoulContext, native: Option<Card>) -> HookOutcome<Option<Card>>`. `CommentaryHook::line(&mut self, &LineContext, native: &str) -> HookOutcome<String>`. `HookOutcome` is `Value`, `Keep`, `Aborted(reason)`, `Denied(kind)`, or `Disabled`. `Plugins` holds the pack identity, the three optional boxed hooks (`Send`), and `ScriptStats`. The engine does not depend on any runtime. NFR-8 ("the plugin boundary is a versioned interface", `02-shape.md:167`) is met by the constant and the `plugin_api` field in every pack.

**Decision hook cadence.** Native scoring runs every tick, as the product owner chose. The script adds offsets to the five option scores. The engine keeps a cache `{ carrier, until_tick, offsets }` and refreshes it through the hook in three cases: the carrier changes, a stoppage has occurred, or `refresh_ticks` have passed (pack field, 5 to 250, default 25). Estimate: about 168,000 ticks with a carrier divided by 25, plus about 1,500 carrier changes, gives about 8,200 calls per match. At 2 µs per call that is about 16 ms, or 4 percent of 418.8 ms. Step 2 measures the real figure. NFR-1 yields to C2 (`02-shape.md:160`). This mechanism leaves the per-tick model and the decision interval unchanged, so it does not narrow any charter commitment.

**The sandbox (named mechanism: slice Scope "sandbox limits").** Start with `rhai::Engine::new()`. Replace the module resolver with `DenyResolver`, which refuses every `import` and records a denial. Disable the `eval` symbol, so a pack that uses it is refused at load. Route `print` and `debug` to `tracing`. Set these limits: operations per call (pack field, 1,000 to 50,000, default 10,000), call levels 16, expression depths 64 and 32, string size 1,024, array size 256, and map size 64. `on_progress` checks a 2 ms wall-clock backstop every 256 operations. The operation limit is the deterministic budget, and the backstop only catches a slow native call. The AST is compiled and its top-level statements run once at load, under the same budget. Each call uses `CallFnOptions::eval_ast(false)` and a fresh scope, so a script keeps no state between calls. Errors are classified as follows. Too many operations, or a termination, is `Aborted(budget)`. A refused import, or a call to a function the sandbox does not expose, is `Denied(name)`; Rhai has no file or network functions, so any such attempt lands here. Any other error is `Aborted(error)`. After three consecutive failures, a hook is `Disabled` for the rest of the match.

**Failure is never fatal.** On any failure the engine keeps the native choice: zero offsets, the referee's card, or the native line. It also pushes `EngineEventKind::Script` with a `ScriptNote { hook, outcome }`, logs a `tracing` warning, and increments `ScriptStats`. No random draw happens on the hook path, so the random stream stays aligned with the no-pack match.

**The pack format.** A folder holds `pack.json` (schema version 1) and one `.rhai` entry file. Fields: `id`, `version`, `name`, `plugin_api`, `entry`, `hooks`, `decision.refresh_ticks`, and `limits.max_operations`. They use `deny_unknown_fields`, garde ranges, and refusals that name the file, the field, and the reason (NFR-7). The identity is `id@version+sha12`, where the hash is SHA-256 over `pack.json` followed by the entry bytes. `--script-pack <DIR>` selects a pack on every command that builds a match. The pack hash is folded into `MatchConfig::content_hash`.

**The contract.** An additive event type `script` and four optional fields: `script.pack` (on the tick-0 kick-off event), `script.hook`, `script.outcome` (`aborted`, `denied`, or `disabled`), and `script.detail`. The match-stats record gains `script.pack` and the counters. Protocol version 2 stays, under the rule at `protocol/src/lib.rs:21-34`. The observability schema files gain the enum value and the keys.

## Step-by-Step Plan

1. **Re-read the landed seams and capture the baseline.** Confirm these seams in the tree: the five-option scoring in `decision.rs`, the `card_outcome` call site in `rules/mod.rs`, the commentator attachment point in `stream_run.rs`, `bench::measure`, and the observability schema files. If any seam differs from this plan, stop and re-run the plan for this slice. Run `cargo build --release -p engine-cli`, then run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` three times. Record the median `bench.cpu_ms / bench.matches`, `bench.cpu_us_per_tick`, and `bench.peak_mem_mb` in the implement artifact as the calibration baseline. Also record the SHA-256 of `simulate --seed 42 --minutes 90` tick output.
2. **Spike the call cost (gate).** Create `crates/script` with `rhai` only, and write `benches/hook_call.rs` with the draft sample `decide` and a representative context. Run `cargo bench -p script --bench hook_call`. The build must succeed with no C compiler; this is the evidence for rung 4. If the median decide call is more than 3 µs, stop and report to the product owner with the figure and the lever (the default `refresh_ticks`). Do not change `decision_interval_ticks`.
3. **Plugin interface.** Write `crates/engine/src/plugin.rs` as in the change strategy, and export it from `lib.rs`. Add `Plugins` and the script cache to `Simulation` with `set_plugins`. Add `EngineEventKind::Script` and `ScriptNote`. Add `MatchConfig::fold_pack_hash`. Add `plugins(...)` to the scenario builder. Run `cargo test -p engine`; all tests must pass. Re-run the step 1 `simulate`; its SHA-256 must be equal.
4. **Decision call site.** In `decision.rs`, add the cached offsets to the option scores before the best option is chosen. Refresh the cache under the three triggers, and clear it in the stoppage path and on a carrier change in `sim.rs`. Unit test: a scene with a fake `DecisionHook` that returns a large shoot offset turns a pass into a shot. A fake hook that returns `Aborted` leaves the native choice and pushes one `Script` event.
5. **Rule call site.** In `rules/mod.rs`, offer the referee's card to the `RuleHook` after the draw. `Value` replaces it. `Keep` and any failure keep it. Unit test with a fake hook.
6. **Counters.** Add the optional `script.*` keys to `MatchStats` in `observe/mod.rs`, filled from `ScriptStats`. Run `cargo test -p engine`; all tests must pass.
7. **Pack loader.** Write `pack.rs` as described in the YAML. Unit tests: an unknown field, `plugin_api` 2, an entry with `..` or a separator, an empty `hooks` list, and `refresh_ticks` 4 are each refused with the field named. A valid pack gives the same identity twice.
8. **Sandbox.** Write `sandbox.rs` as in the change strategy. Unit tests: `loop {}` aborts with a budget reason; `import "x" as y;` is denied; `http_get("u")` is denied naming the function; `eval("1")` is refused at load; `print("x")` writes nothing to stdout; a 2,000-character string is refused.
9. **Hooks.** Write `hooks.rs`. Register `DecisionContext`, `FoulContext`, and `LineContext` as custom types with getters. Parse the return shapes, reject non-finite or out-of-range offsets, cap lines at 280 characters, and add the circuit breaker. Unit tests for each return shape and for the breaker.
10. **Command line.** Add `--script-pack` in `cli.rs`. Add one shared loader in `content.rs`: load the pack, fold the hash, and return the `Plugins`. In `stream_run.rs`: attach the plugins, map `Script`, put `script.pack` on the tick-0 kick-off event, and offer each commentary line to the commentary hook. Wire the same loader into `simulate.rs`, `bench.rs` (with `script.pack` in the run report), and `resume.rs` (load the pack before the snapshot check).
11. **Protocol.** In `event.rs`, add `EventType::Script` and the four fields. In `lib.rs`, add the fields to `MESSAGES` and the version-2 reason. Add the rows and examples to `docs/reference/protocol.md`. Add the arm to `crates/stream/tests/common/mod.rs`. Update both schema files. Run `cargo test -p protocol -p stream`; all tests must pass.
12. **Sample pack and reference.** Write `content/scripts/sample/pack.json` and `main.rhai` as described in the YAML. Write `content/scripts/README.md` and the pointer in `content/README.md`.
13. **Fixtures.** Write the five fixture packs in `crates/script/tests/fixtures/`: `shoot-bias`, `looping`, `import-escape`, `network-call`, and `bad-return`.
14. **Criterion tests.** Write `crates/script/tests/decision.rs` (AC-1, engine side, with the determinism checks), `crates/script/tests/sandbox.rs` (AC-2, AC-3, and the breaker), and `crates/engine-cli/tests/script_cli.rs` (AC-1, stream side, the bad-pack refusal, and the resume refusal). Run `cargo test --workspace`; all tests must pass.
15. **Call-cost recheck.** Re-run `cargo bench -p script --bench hook_call` with the final sample pack, and record the median decide and card call times.
16. **Benchmark compare (AC-4).** Run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json --script-pack content/scripts/sample` three times, then run the same command three times without the flag. Pass: the scripted median processor time per match is at most 1.10 times the step 1 baseline. Also, the unscripted median must be within the run-to-run noise of the baseline (at most 1.02 times). Record `script.calls` per match. If the scripted run fails, stop and report to the product owner with the figures. Do not lower `refresh_ticks` below the default without the product owner.
17. **Gates.** Search `crates/`, `content/`, `docs/`, `schemas/`, and `README.md` for workflow vocabulary (slice names, stage names, `.ai/`); the search must return nothing. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `node --test web/tests/*.test.mjs`. Commit with a product-language message that ends with the attribution line the session requires.

## Verification Strategy

The slice definition marks AC-1 to AC-3 `observable: false`, and they are verified by `cargo test`. AC-4 is developer-visible.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-4 benchmark with the sample pack is within 10 percent of the calibration baseline | `engine-cli bench --json` with and without `--script-pack` (cli-direct) | The reference laptop (this Windows machine; the earlier baselines ran here) — yes. The calibration baseline figure — produced at step 1 once `calibration` lands | The `--script-pack` flag on `bench`, `script.pack` in the run report, the sample pack, and the step 1 baseline capture | The criterion call-cost bench (`hook_call`) × `script.calls` per match as a proxy → pre-registered deferral cleared when the implementer runs step 16 on the reference laptop |

- AC-4: `constraint-resolution: prerequisite-slice: calibration` — `wall-ownership: code-owned`. The baseline exists only after the calibration slice lands, and this slice's `depends-on` already orders it. Step 1 captures the figure, so no external wall remains.

## Test / Verification Plan

### Automated checks

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo test -p script`: pack refusals, sandbox classification, return-shape parsing, breaker, `tests/decision.rs` (AC-1 and determinism), `tests/sandbox.rs` (AC-2, AC-3).
- `cargo test -p engine`: fake-hook unit tests for the decision and rule call sites; the no-pack tick output equals the step 1 SHA-256.
- `cargo test -p engine-cli --test script_cli`: `script.pack` in `events.jsonl` and `stats.json`, the bad-pack exit, and the resume refusal.
- `cargo test -p protocol`: the document test covers the new fields.
- `cargo bench -p script --bench hook_call` (steps 2 and 15).
- `node --test web/tests/*.test.mjs`: unchanged and green.

AC classification: AC-1, AC-2, and AC-3 are `build-capability` (cargo tests). AC-4 is `runtime-evidence` on the reference laptop through `cli-direct`.

### Interactive verification (human-in-the-loop)

AC-4 on the command line (runtime adapter `cli`, `stack.platforms: [web, cli]`, `stack.testing: [cargo-test]`, `stack.observability: []`, `user-confirmed: true`):
1. `cargo build --release -p engine-cli`.
2. Run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json --script-pack content/scripts/sample` three times, keeping stdout, stderr, and the exit code of each run under the verify evidence folder.
3. Run the same command three times without `--script-pack`.
4. Read each stdout file. Each holds one `run-report` JSON line. Pass criteria: the scripted median of `bench.cpu_ms / bench.matches` is at most 1.10 times the step 1 baseline; `script.pack` starts with `sample@1.0.0+`; the exit code is 0; `build.hash` is not `unknown`; `bench.peak_mem_mb` is at most 1.25 times the baseline.

No page is involved. The page ignores the `script` event type, which `web/stoppages.mjs` does not list.

## Risks / Watchouts

- **R1 (high) Tick budget.** A per-tick interpreted call would cost several times the budget. Mitigations: cached offsets with a refresh interval, the step 2 gate, and the stop rule at step 16. `decision_interval_ticks` is never changed.
- **R2 (high) Sibling seams drawn from plans.** The scored options, the commentator, `bench::measure`, and the schema files come from sibling plans that are not built yet. Step 1 stops for a plan re-review on a mismatch.
- **R3 (medium) Default import resolver reads files.** The upstream `Engine::new()` installs a `FileModuleResolver`. The sandbox replaces it, and the `import-escape` fixture proves the replacement.
- **R4 (medium) Determinism.** No random function is exposed, scripts are stateless, the cache clears at stoppages, and a byte-for-byte test covers two seeded runs. Rhai's internal `ahash` uses `compile-time-rng` (upstream `Cargo.toml`, read this run). It affects only function lookup inside Rhai, not what a script can observe, and the byte-for-byte test covers that claim.
- **R5 (medium) Stdout corruption.** Rhai `print` writes to stdout by default, so it is routed to `tracing`, and a test checks this.
- **R6 (low) Resumed counters.** `ScriptStats` is not in the snapshot, so a resumed match counts script calls from the resume tick. `content/scripts/README.md` states this.

## Dependencies on Other Slices

- `data-schemas-generator` (complete): `data::load_json`, the garde idiom, and `ContentDir`.
- `tactics-and-ai` (planned): the five-option scoring in `decision.rs`, which is the decision call site. The tactics plan names `queue_change` as this slice's entry point for scripted decisions (`04-plan-tactics-and-ai.md:186`). This plan chooses score offsets instead, and a scripted manager that queues changes is not in this slice's scope (Assumption A-4).
- `commentary` (planned): the attachment point in `stream_run.rs`, which is the commentary call site.
- `calibration` (planned): `bench::measure`, the schema files, and the baseline figure.
- `experiment-flags` (deferred): no interaction.
- `.ai/observability.md` lists the contract keys. The new `script` event type and `script.*` keys are written into the schema files by this slice. The contract list needs the same keys at the next observability audit. That update is outside this plan's writes.

## Assumptions

Each entry records a question the plan's discovery round would have asked. It was resolved without the product owner under the autonomous policy, and each one is stamped `class: implementation-detail`. Each entry picks among ways to meet criteria that the shape and the slice already fixed, inside scope that the slice delegates to this plan ("runtime choice (plan decides among embeddable options with a permissive license), sandbox limits, hook points, a script pack format", `03-slice-scripting-runtime.md` Scope). No `carried` intent-risk exists (`00-index.md` marks all six adjudicated), so no decision touches one. No product-owner question was asked. A summary line is appended to `po-answers.md` with `stage: plan`.

1. **A-1 Runtime** (`class: implementation-detail`). Rhai 1.26. Rejected: mlua with vendored Lua, because `lua-src` depends on `cc` (`https://raw.githubusercontent.com/mlua-rs/lua-src-rs/main/Cargo.toml`, read this run) and no C compiler is present. rquickjs, because it binds the QuickJS C library; that crate's build was not read this run, so this rejection rests on the same missing-compiler evidence and must be re-checked if a compiler is added. wasmtime, because it is a large dependency and modders would need a separate WebAssembly toolchain. boa, because it is heavier and has no general operation budget in the pages read. piccolo, because it is pre-1.0. Why Rhai: pure Rust, MIT OR Apache-2.0 (NFR-5), a release on 2026-09-10 (maintenance risk in the slice), and a deterministic operation budget.
2. **A-2 Separate crate** (`class: implementation-detail`). The interface lives in the engine and the runtime in `crates/script`. Why: NFR-8 asks for separate packages and a versioned boundary, and a match without a pack keeps a byte-identical path.
3. **A-3 Hook cadence** (`class: implementation-detail`). Cached offsets refresh on a carrier change, after a stoppage, or every `refresh_ticks` (default 25, 5 to 250). Why: the tick budget (slice risk "hooks run at decision frequency, not steering frequency"). The product owner's per-tick decision interval is kept.
4. **A-4 Hook shape** (`class: implementation-detail`). The decision hook returns offsets from -10 to 10 for the five options. An offset in that range can force any option, so a script can override the choice (AC-1). Scripted manager changes through `queue_change` are not planned; the slice's hook list names decision scoring, rule handlers, and commentary only.
5. **A-5 Time budget unit** (`class: implementation-detail`). The budget is an operation count (deterministic), with a 2 ms wall-clock backstop. Why: best-effort determinism (Q6, Q32) holds only with a count, and the backstop keeps the literal "time budget" of AC-2.
6. **A-6 Denial model** (`class: implementation-detail`). The only file path in Rhai is `import`, and it is refused and recorded. Rhai exposes no network function, so a call to any function that the sandbox does not expose is recorded as a denial that names the function. `eval` is refused at load.
7. **A-7 Stateless scripts** (`class: implementation-detail`). There is no state between calls and no random function. Why: resume from a stoppage snapshot and determinism need no script state in the snapshot.
8. **A-8 Circuit breaker** (`class: implementation-detail`). Three consecutive failures disable a hook for the match, recorded as `disabled`. Why: a pack that fails on every call would otherwise pay the full budget on every refresh.
9. **A-9 Pack format and selection** (`class: implementation-detail`). A folder with `pack.json` schema version 1 and one `.rhai` entry, selected by `--script-pack <DIR>`. No pack is loaded by default. Why: the slice puts the pack format in this plan's scope, and an explicit flag keeps every existing run unchanged.
10. **A-10 Pack identity and resume** (`class: implementation-detail`). The identity is `id@version+sha12`. Its hash is folded into `content_hash`, so the snapshot layout and version do not change, and a resume with a different pack is refused by the existing check.
11. **A-11 Event contract** (`class: implementation-detail`). An additive `script` event type and optional `script.pack`, `script.hook`, `script.outcome`, and `script.detail` fields. Protocol version 2 stays, under the recorded additive rule, following the commentary plan's precedent. Why: AC-1 requires the stream to record the pack, and AC-2 and AC-3 require the engine to record aborts and denials.
12. **A-12 Counters outside the snapshot** (`class: implementation-detail`). `ScriptStats` lives in `Plugins`, not in `Summary`. Why: this keeps the snapshot version. The cost is that a resumed match counts from the resume tick (R6).
13. **A-13 Rule and commentary hook scope** (`class: implementation-detail`). The rule hook can change the card that the referee chose for a foul. The commentary hook can replace a line at the shared driver. Why: these are the lowest-frequency sites that meet "rule handlers" and "commentary" in the slice scope. Neither criterion tests them, so they carry unit tests only.
14. **A-14 Augmentations** (`class: implementation-detail`). `05c-benchmark.md` and `04b-instrument.md` are not rewritten in this run. Step 1 captures the benchmark baseline at implement time, when the calibration baseline exists. Why: this follows the calibration plan's precedent (its Assumption 12), and those shared files will be re-authored by the slices planned before this one. The experiment augmentation is deferred to `experiment-flags` (`00-index.md` `augmentations:`). Signals of this slice: `event.type` `script` with `script.outcome`, `tracing` warnings `script.aborted`, `script.denied`, and `script.disabled`, and match-stats `script.calls`, `script.aborts`, `script.denials`, and `script.disabled`. The dark path is a hook that silently never runs; `script.calls` of zero with a pack loaded exposes it.
15. **A-15 Second opinion** (`class: implementation-detail`). The consult trigger `appetite-medium-or-larger` holds (`00-index.md` `appetite: large`), but the product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`). No consult is run, and `consult-runs` stays empty.
16. **A-16 Visual contract** (`class: implementation-detail`). The slice has no page surface, so no visual contract step applies. The image gate in `steer.md` covers viewer slices only.

## Blockers

None.

## Freshness Research

- Source: crates.io API for `rhai`, read this run. The newest stable version is 1.26.1 (2026-09-10), and earlier releases came out on 2026-08-25, 2026-05-29, 2026-05-24, and 2026-01-19. License MIT OR Apache-2.0. Takeaway: actively maintained, and the license is permissive (NFR-5).
- Source: `https://raw.githubusercontent.com/rhaiscript/rhai/main/Cargo.toml`, read this run. No `cc` or C build step. The minimum Rust version is 1.66, below the workspace's 1.87. `ahash` uses `compile-time-rng`. The `sync` feature makes the engine `Send + Sync`. Takeaway: it builds on this machine's toolchain, and the `sync` feature meets the thread requirement.
- Source: `https://raw.githubusercontent.com/rhaiscript/rhai/main/src/engine.rs`, read this run. `Engine::new()` sets `module_resolver = FileModuleResolver::new()` when `no_module`, `no_std`, and `no_ast` are off; `new_raw()` sets `None`. Takeaway: the sandbox must replace the resolver (R3).
- Source: `https://rhai.rs/book/safety/progress.html`, read this run. `on_progress` takes `Fn(u64) -> Option<Dynamic>`, and `Some` terminates with `ErrorTerminated`. Takeaway: this is the time backstop.
- Source: `https://rhai.rs/book/engine/call-fn.html`, read this run. `call_fn` evaluates the whole AST before each call by default, and `CallFnOptions::eval_ast(false)` skips that. Takeaway: every call uses `eval_ast(false)`, otherwise the cost of each call grows with the size of the script.
- Source: `https://raw.githubusercontent.com/mlua-rs/lua-src-rs/main/Cargo.toml`, read this run. `lua-src` depends on `cc` 1.2. Takeaway: vendored Lua cannot build here (A-1).
- Not verified this run: the exact error variant names of Rhai 1.26 (`ErrorTooManyOperations`, `ErrorModuleNotFound`, `ErrorFunctionNotFound`), and whether `set_max_modules` treats 0 as unlimited. Step 2 reads the installed source under `~/.cargo/registry` before step 8 relies on them, and the plan does not use `set_max_modules`.
- The Rhai per-call cost of about 2 µs used in the estimate is a planning guess. Step 2 measures the real figure.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine scripting-runtime`, after `tactics-and-ai`, `commentary`, and `calibration` are implemented. The plan is complete. Its step 1 re-checks their seams, and step 2 gates the call cost. Compact the session first so the SessionStart hook re-reads the artifacts.
- **Option B:** `/wf implement football-manager-match-engine tactics-and-ai`, to continue the buildable order that this deferred slice depends on.
- **Option C:** `/wf slice football-manager-match-engine`, only if the scripted-manager capability (queued changes from a script) is wanted in this slice; A-4 leaves it out.
