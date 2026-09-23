---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: scripting-runtime
status: complete
stage-number: 5
created-at: "2026-09-23T17:51:42Z"
updated-at: "2026-09-23T17:51:42Z"
metric-files-changed: 58
metric-lines-added: 3165
metric-lines-removed: 76
metric-deviations-from-plan: 9
metric-review-fixes-applied: 0
commit-sha: "7fd5197169521baa4b1fbdbea427acd9bc4b6cc1"
commits:
  - "7fd5197169521baa4b1fbdbea427acd9bc4b6cc1"
steering-honored:
  - "steer.md holds design direction for the viewer only; this slice changes no page. The page ignores the new script event type, so no design constraint applies."
  - "Dark-path counter definition: untouched. The computer manager's queuing behaviour did not change."
  - "Output boundary: the commit message, code comments, docs, fixtures, and test titles use product language; a search of every added line found no workflow vocabulary."
tags: [engine, modding, scripting, sandbox, rhai, plugin-interface, protocol-v3, schemas, docs, deferred, rim-6]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-scripting-runtime.md
  plan: 04-plan-scripting-runtime.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/scripting-runtime/
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-commentary.md, 05-implement-calibration.md, 05-implement-viewer-match-day.md, 05-implement-viewer-lineup-tactics.md, 05-implement-viewer-reports-recovery.md, 05-implement-integration.md, 05-implement-extra-time-penalties.md, 05-implement-experiment-flags.md]
  verify: 06-verify-scripting-runtime.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine scripting-runtime"
---

# Implement: Scripting Runtime for Modding

## The Implementation

The build started from a tree where modders could change data files but no code of theirs could run. The source had no plugin trait and no hook. The seams the plan drew from sibling plans had all landed: the five-option scoring in `decision.rs`, the `card_outcome` call in `rules/mod.rs:226-235`, the commentator in `stream_run.rs`, `bench::measure`, and the three schema files. The baseline on commit `3066066` was 422.0 ms of processor time per match (drives 422.0, 418.8, 422.0), 1.4933 µs per tick, and 6.79 MB peak memory. The seed-42 tick records hashed to `8120abc5…adc0`. The step 2 gate measured a draft decide call at 1.149 µs, under the 3 µs stop line, and it built with no C compiler present.

The engine now owns a runtime-free plugin interface (`crates/engine/src/plugin.rs`, `PLUGIN_API_VERSION = 1`) with a decision hook, a rule hook, and a commentary hook. A new `script` crate implements it with Rhai 1.26.1. The decision hook adds offsets to the five option scores. The engine caches the offsets per carrier and refreshes them on a new carrier, after a stoppage, or every `refresh_ticks` (default 25), so the sample pack makes 3,704 calls in a 90-minute match. A failed hook keeps the native choice and records a `script` event. After three failures in a row, the hook is switched off. The pack hash joins the content hash, so a scripted snapshot resumes only with the same pack. The build made 9 minor, in-scope deviations from the plan. The largest is that `ServerMessage::Event` is now boxed, because the four new event fields pushed the enum over clippy's size-difference lint. All 24 autonomous decisions are `class: implementation-detail`.

After the change, 398 Rust tests pass with 0 failed and 4 ignored (34 new), 127 page tests pass, and fmt and clippy (`-D warnings`) are clean. Without a pack, the seed-42 tick records are byte-identical to the baseline. With the sample pack, the median is 431.4 ms per match, 1.022 times the baseline, inside the 1.10 limit. Verify can prove AC-1 to AC-3 with `cargo test` and re-run the AC-4 compare. The top open risk is peak memory with a pack: 8.29 MB, 1.22 times the baseline, against the 1.25 limit.

## Summary of Changes

- A versioned plugin interface in the engine: three hook traits (`DecisionHook`, `RuleHook`, `CommentaryHook`, all `Send`), `Copy` context structs, `HookOutcome` (`Value`, `Keep`, `Aborted`, `Denied`), `Plugins` with the pack identity, `refresh_ticks`, `ScriptStats`, and the three-in-a-row breaker. The engine has no runtime dependency.
- Three call sites: cached decision offsets in `decide_carrier`, the rule hook after the referee's draw in `foul`, and `Simulation::offer_line` for the commentator's line. No hook draws from the random stream.
- `EngineEventKind::Script` with `EventDetail::Script(ScriptNote)`. The note points into `Plugins::details` for the detail text, so `EngineEvent` stays `Copy` and gains no field.
- A new `script` crate: the `pack.json` loader (schema 1, `deny_unknown_fields`, garde bounds, the `id@version+sha12` identity), the sandbox, and the three script-backed hooks.
- The sandbox: an operation budget (default 10,000), a 2 ms wall-clock backstop checked every 256 operations, 16 call levels, expression depths of 64 and 32, a 1,024-character string cap, a 256-item array cap, a 64-entry map cap, Rhai's refusing module resolver, `eval` disabled, and `print`/`debug` routed to `tracing`. An unknown function is a denial that names it.
- `--script-pack <DIR>` on `simulate`, `bench`, `serve`, `record`, and `resume`, through one shared loader (`content::Loaded::fold` and `attach`).
- Contract: the `script` event type and the `script.pack`, `script.hook`, `script.outcome`, and `script.detail` fields. `match-stats` gains `script.pack` plus four counters, and `run-report` gains `script.pack`. Protocol version 3 stays, with the reason recorded beside `PROTOCOL_VERSION`.
- The sample pack `content/scripts/sample/`, the modder reference `content/scripts/README.md`, and a pointer in `content/README.md`. `docs/reference/cli.md` and `docs/reference/protocol.md` document the flag and the fields.

## Files Changed

- `Cargo.toml`: the `crates/script` member and `rhai = { version = "1.26", features = ["sync"] }`.
- `Cargo.lock`: rhai 1.26.1, rhai_codegen 3.2.0, and their pure-Rust dependencies.
- `crates/engine/src/plugin.rs` (new): the plugin interface, `Plugins::settle` (counting, warning signals, the breaker), and a unit test for the breaker.
- `crates/engine/src/lib.rs`: `pub mod plugin`; re-exports `PLUGIN_API_VERSION` and `Plugins`.
- `crates/engine/src/sim.rs`: `EngineEventKind::Script` (18 kinds), `EventDetail::Script`, `ScriptCache`, the `plugins` and `script_cache` fields, `set_plugins`, `plugins`, `offer_line`, `push_script_notes`, `MatchConfig::fold_pack_hash`, and the cache cleared at every stoppage and every gain of possession.
- `crates/engine/src/decision.rs`: `script_offsets` (cache and refresh) and `decision_context`; the offsets are added to the option scores in `decide_carrier`.
- `crates/engine/src/rules/mod.rs`: the rule hook after the referee's card draw in `foul`.
- `crates/engine/src/scenario.rs`: the `Scene::plugins` builder method.
- `crates/engine/src/observe/mod.rs`: `ScriptFigures` flattened into `MatchStats`; `RunReport::script_pack`.
- `crates/engine/tests/plugin_hooks.rs` (new): fake-hook tests for the decision and rule call sites and the cache.
- `crates/engine/tests/identity.rs`: the new `MatchStats` field.
- `crates/script/` (new): `Cargo.toml`, `src/lib.rs` (`LoadedPack`, `ScriptError`), `src/pack.rs`, `src/sandbox.rs`, `src/hooks.rs`, `benches/hook_call.rs`, `tests/common/mod.rs`, `tests/decision.rs`, `tests/sandbox.rs`, `tests/sample.rs`, and five fixture packs of two files each.
- `crates/protocol/src/event.rs`: `EventType::Script` (17 types), the four optional fields, and the `script_pack` and `script` builders.
- `crates/protocol/src/lib.rs`: the version-3 reason and the four fields in `MESSAGES`.
- `crates/protocol/src/message.rs`: `ServerMessage::Event(Box<MatchEvent>)`.
- `crates/stream/src/session.rs`, `crates/stream/tests/common/mod.rs`: box the event; the helper's mapping gains the `Script` arm.
- `crates/engine-cli/Cargo.toml`: depends on `script`.
- `crates/engine-cli/src/cli.rs`: `--script-pack` on five commands.
- `crates/engine-cli/src/content.rs`: loads the pack; `Loaded::fold` and `Loaded::attach`.
- `crates/engine-cli/src/stream_run.rs`: maps `Script`; `rows()` offers each line to the commentary hook, names the pack on the first kick-off, and fills the `script.*` fields; the driver's two event loops use it.
- `crates/engine-cli/src/simulate.rs`, `resume.rs`, `record.rs`, `serve.rs`: fold the pack hash and attach the hooks at every match build and resume.
- `crates/engine-cli/src/bench.rs`: `measure` takes the pack and gives each match fresh hooks; the run report names the pack.
- `crates/engine-cli/src/calibrate/mod.rs`, `calibrate/worker.rs`, `report/mod.rs`: the new `load` and `measure` arguments and the `MatchStats` field (no pack).
- `crates/engine-cli/tests/script_cli.rs` (new): the command-line criterion tests.
- `schemas/observability/match-event.schema.json`, `match-stats.schema.json`, `run-report.schema.json`: the enum value and the optional keys.
- `docs/reference/protocol.md`, `docs/reference/cli.md`: the fields and the flag.
- `content/scripts/sample/pack.json`, `content/scripts/sample/main.rhai`, `content/scripts/README.md` (new), and `content/README.md`: the sample and the modder reference.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs`, `decision.rs`, `rules/mod.rs`: every engine slice. The no-pack path is unchanged, and the seed-42 records are byte-identical.
- `crates/engine/src/observe/mod.rs` and every `MatchStats` literal: a later slice that adds a `MatchStats` literal needs `script: ScriptFigures::default()` or `ScriptFigures::new(sim.plugins())`.
- `crates/protocol/src/message.rs`: `ServerMessage::Event` is boxed. A constructor now writes `ServerMessage::Event(Box::new(row))`, and a filter that collects events dereferences them.
- `crates/engine-cli/src/content.rs`: `content::load` takes a fourth argument, the script pack.
- `crates/engine-cli/src/bench.rs`: `measure(config, script, matches)`; `calibrate` passes `None`.
- `schemas/observability/*.json`, `docs/reference/*.md`: the shared contract and references.

## Notes on Design Choices

- The breaker lives in `Plugins::settle` in the engine, not in the script crate. One place counts, logs, records, and switches off for all three hooks, and any future runtime inherits it.
- `ScriptNote` carries an index into `Plugins::details` rather than a string, so `EngineEvent` stays `Copy`. The detail text is resolved when the row is built.
- `offer_line` returns the failure events instead of pushing them. The caller routes them right after the row they belong to, so the final drain after full time loses no event.
- The decision cache is cleared in `gain()`, which covers every change of carrier, and after `apply_changes` at every stoppage. No cache state reaches the snapshot.
- The pack is read with `data::load_json`, so a refusal reads the same as for any content file (`content refused: script pack <path>: <field>: <reason>`).
- `Sandbox` is shared by `Arc` across the three hooks and across matches. `LoadedPack::plugins()` gives each match fresh counters.

## Verification Seams Built

- AC-1 (scripted decisions take effect; the event stream records the pack) → the `shoot-bias` fixture pack at `crates/script/tests/fixtures/shoot-bias/main.rhai:2-4`; `Scene::plugins` at `crates/engine/src/scenario.rs:255`; the pack named on the first kick-off at `crates/engine-cli/src/stream_run.rs:462`; `ScriptFigures` at `crates/engine/src/observe/mod.rs:109` (enables `cargo test -p script --test decision` and `cargo test -p engine-cli --test script_cli` to observe it).
- AC-2 (a budget overrun is aborted and logged, and the default decision stands) → the `looping` fixture at `crates/script/tests/fixtures/looping/main.rhai:2-4`; the budget classification at `crates/script/src/sandbox.rs:138`; the `script.aborted` warning in `Plugins::settle` at `crates/engine/src/plugin.rs:234`; a log capture through `tracing_subscriber` in `crates/script/tests/sandbox.rs` (enables `cargo test -p script --test sandbox`).
- AC-3 (file or network access is denied and recorded) → the `import-escape` and `network-call` fixtures; the refusing resolver and the disabled `eval` at `crates/script/src/sandbox.rs:67-68` (enables `cargo test -p script --test sandbox` and `cargo test -p engine-cli --test script_cli`).
- AC-4 (the benchmark with the sample pack is within 10 percent of the calibration baseline) → `--script-pack` on `bench` at `crates/engine-cli/src/cli.rs:297`; `RunReport::script_pack` at `crates/engine/src/observe/mod.rs:416`; the sample pack; the step 1 baseline capture in `implement-evidence/scripting-runtime/baseline-bench-{1,2,3}.stdout.txt` (enables `engine-cli bench --json` on the reference machine, cli-direct).

## Deviations from Plan

1. **`ServerMessage::Event` is boxed** (`crates/protocol/src/message.rs`). The four new optional fields made the `Event` variant 424 bytes, and `cargo clippy -- -D warnings` failed with `large_enum_variant`. This follows the precedent of `Hello(Box<Hello>)`, and serde serializes a `Box` transparently, so the wire format is unchanged. Six sites changed.
2. **Protocol version is 3, not 2.** The plan named version 2, but version 3 had landed by implement time (`crates/protocol/src/lib.rs`). The additive rule is the same, and the reason is recorded beside `PROTOCOL_VERSION`.
3. **The breaker is in the engine** (`Plugins::settle`), not in `hooks.rs`, and `HookOutcome` has no `Disabled` variant. The engine produces `disabled` itself. The behaviour matches the plan: three consecutive failures switch the hook off and are recorded once as `disabled`.
4. **The resolver is Rhai's `DummyModuleResolver`**, not a hand-written `DenyResolver`. The plan's own resolver needed `SharedModule`, which Rhai 1.26.1 keeps private (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rhai-1.26.1/src/lib.rs:276`, `type SharedModule = Shared<Module>;` without `pub`, read this run). The installed `src/module/resolvers/dummy.rs:42-50` refuses every path with `ErrorModuleNotFound`, which is exactly the plan's behaviour (rung 3 reuse).
5. **The script event carries its detail through an index** (`ScriptNote::detail` into `Plugins::details`), and the note rides in `EventDetail::Script`, not in a new `EngineEvent` field. This keeps `EngineEvent` `Copy` and leaves its three struct literals unchanged.
6. **The no-pack check in `tests/decision.rs` compares a hookless `Plugins` with no plugin**, instead of pinning the pre-slice SHA-256 in a test. A hard-coded hash would break on every later engine change. The byte-identity against the pre-slice build was checked once, with the release binary, at implement time (`implement-evidence/scripting-runtime/baseline-ticks-sha256.txt` and `after-ticks-sha256.txt`, both `8120abc554836ec9a4691e0afe25386a0ccc137417a6b9522ec5d73f2144adc0`).
7. **`DecisionContext` has `progress`, `goal_diff`, and no `x`/`y`**. It gives the carrier's position as a signed progress from -1 to 1 up the pitch, not as raw coordinates, so a script does not need to know which way a team attacks. `goal_diff` is a getter over `goals_for` and `goals_against`.
8. **One extra test file, `crates/script/tests/sample.rs`**, checks each hook of the shipped sample pack. The seeded 90-minute match ended 0-0, so the goal-line rewrite was not exercised by a match.
9. **The `--script-pack` long help was shortened** to keep every help line under 80 columns (`cli_args.rs`). **`docs/reference/cli.md` gained the flag** on five commands, which `docs.rs` requires. Neither file was in the plan's list.

## Anything Deferred

- A scripted manager that queues tactical changes (the `queue_change` entry point the tactics plan named) is out of scope (plan Assumption A-4). Only decision scoring, cards, and commentary have hooks.
- `.ai/observability.md` does not yet list the `script` event type or the `script.*` keys. The schema files carry them, and the contract list is updated at the next observability audit (plan, Dependencies).
- `calibrate` does not take `--script-pack`. `measure` accepts a pack, so adding it later is a one-flag change.
- Script counters are not in the snapshot, so a resumed match counts from the resume tick (plan R6). `content/scripts/README.md` states this.

## Known Risks / Caveats

- Peak memory with the sample pack is 8.29, 8.26, and 8.41 MB (median 8.29) against a baseline median of 6.79 MB: 1.22 times, inside the plan's 1.25 limit, with a thin margin. The Rhai engine and its standard packages account for the rise. `Engine::new_raw()` with only the needed packages is the upgrade path if a later pack pushes it over.
- The 2 ms wall-clock backstop is not deterministic. A call that a slow machine stops by time would run on a fast one. The operation budget stops every loop the tests use first. The backstop only guards a slow built-in call.
- Rhai's internal hash uses `compile-time-rng`. It affects only Rhai's function lookup, and the same-seed, same-pack byte-for-byte test covers the claim (`the_same_seed_and_pack_twice_give_the_same_ticks`).
- The pack identity hashes the bytes on disk. A checkout that converts line endings (`core.autocrlf=true` on Windows) gives the sample pack a different `+hash` than an LF checkout. A snapshot still resumes only with the pack it was written with, on the same machine. A `.gitattributes` rule that pins `content/scripts/**` to LF is the upgrade path.
- `Pack::read` reads `pack.json` twice, once through `load_json` and once for the pack hash. A file changed between the two reads would hash a different manifest than the one validated. This matters only for a file edited during the load.

## Freshness Research

- Installed Rhai source, `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rhai-1.26.1/`, read this run:
  - `src/types/error.rs:30-117` lists the variants the classifier uses: `ErrorTooManyOperations`, `ErrorTerminated`, `ErrorModuleNotFound`, `ErrorFunctionNotFound`, `ErrorInFunctionCall`, and `ErrorInModule`.
  - `src/api/events.rs:242` `on_progress(Fn(u64) -> Option<Dynamic>)`; `:274` `on_print`; `:321` `on_debug`.
  - `src/api/call_fn.rs:16-80` `CallFnOptions::eval_ast` and `rewind_scope`.
  - `src/api/limits.rs` holds the setters used; `src/api/mod.rs:97` `set_module_resolver`; `:142` `disable_symbol`.
  - `src/module/resolvers/dummy.rs` is the refusing resolver.
  - `src/ast/ast.rs:681` `iter_functions`, used to check that a listed hook defines its function.
- Rhai 1.26.1 resolved from crates.io (`Cargo.lock` checksum `0334639972c0ea5a3fd366aa36116754a11431b619fec3ed559b3f73bcbcebf5`). The workspace's `licenses.rs` test passes with it. It built with no C compiler (`where cl clang` reported "Could not find files").
- Measured, not guessed: a draft decide call cost 1.149 µs at step 2. The final sample decide call costs 1.316 µs and the sample card call 869 ns (`recheck-hook-call.stdout.txt`).

## Assumptions

Each entry is an autonomous decision taken in place of a human gate. All are `class: implementation-detail`. None touches an open or carried intent risk (all six are `adjudicated` in `00-index.md`), alters a product-owner directive, assigns control authority, changes the core loop, or drops a committed capability.

1. `class: implementation-detail`. The step 1 seam check found all five seams in the tree, so the plan stands and no re-plan was needed.
2. `class: implementation-detail`. The step 2 gate passed (1.149 µs against 3 µs), so no stop.
3. `class: implementation-detail`. Box `ServerMessage::Event` to satisfy clippy (deviation 1).
4. `class: implementation-detail`. Keep protocol version 3 under the additive rule (deviation 2).
5. `class: implementation-detail`. The breaker lives in the engine (deviation 3).
6. `class: implementation-detail`. Use Rhai's `DummyModuleResolver` (deviation 4).
7. `class: implementation-detail`. Carry the detail by index in `EventDetail::Script` (deviation 5).
8. `class: implementation-detail`. Test the no-pack path by comparison and check the pre-slice hash once at implement (deviation 6).
9. `class: implementation-detail`. The context field set (deviation 7).
10. `class: implementation-detail`. Add `tests/sample.rs` (deviation 8).
11. `class: implementation-detail`. Shorten the help text and document the flag in `cli.md` (deviation 9).
12. `class: implementation-detail`. Hook function names are `decide(ctx)`, `card(ctx, native)`, and `line(ctx, native)`. A pack that lists a hook without its function is refused at load.
13. `class: implementation-detail`. `card` receives the native card as `"none"`, `"yellow"`, `"second-yellow"`, or `"red"`, and can return only `"none"`, `"yellow"`, or `"red"`. A yellow for a booked player becomes a second yellow through the existing `show_card` rule.
14. `class: implementation-detail`. An unknown property on a context is `aborted`, not `denied`. Only an unknown function is a denial.
15. `class: implementation-detail`. `script.pack` rides on the first `kick-off` event of the match, tracked by `Ids::pack_named`, so a resumed match that emits no kick-off carries it in `match-stats` only.
16. `class: implementation-detail`. A script's top-level statements run once at load under the operation budget, and a failure there refuses the pack.
17. `class: implementation-detail`. `serve` attaches fresh hooks at kick-off, at each resume from a stoppage snapshot, and at a restart from kick-off; the preview simulation for the hello gets none.
18. `class: implementation-detail`. `calibrate` passes no pack (`None`); a scripted calibration is out of this slice's criteria.
19. `class: implementation-detail`. Files written by an ad-hoc edit script got CRLF endings on this Windows checkout, and `docs.rs` failed on `cli.md`. Every touched file was put back to LF; `core.autocrlf=true` stores LF in the index either way.
20. `class: implementation-detail`. The code is committed by explicit paths with `git commit -- <paths>`. Files another session had staged (`crates/engine/tests/zz_stall_probe.rs`, `docs/design/realism/*`) were left out and left staged as found. The workflow artifacts go in a second commit, as in the sibling records.
21. `class: implementation-detail`. The baseline is re-measured at step 1 on `3066066` (422.0 ms) rather than taken from `05c-benchmark.md` (418.8 ms on `837cb5c`), as plan step 1 says. `05c-benchmark.md` is not rewritten (plan Assumption A-14).
22. `class: implementation-detail`. The AC-4 memory reading (1.22 times) passes the plan's 1.25 limit and is recorded as a risk, not as a failure.
23. `class: implementation-detail`. The sample pack's offsets (shoot +0.4 when trailing inside 25 m, dribble -0.3 under 2.5 m pressure) and its card and line rules follow the plan YAML word for word.
24. `class: implementation-detail`. The observability contract list in `.ai/observability.md` is not edited (plan Dependencies); it is recorded under Anything Deferred.

Acceptance criteria classification (canonical for this run): AC-1 is `build-capability` (`cargo test -p script --test decision` and `cargo test -p engine-cli --test script_cli`). AC-2 is `build-capability` (`cargo test -p script --test sandbox`). AC-3 is `build-capability` (`cargo test -p script --test sandbox` and `script_cli`). AC-4 is `runtime-evidence` through cli-direct on the reference machine. It was produced at implement and re-runs at verify.

## Checks Run This Stage

- Baseline, commit `3066066`, release build: `engine-cli bench --seed 42 --matches 5 --json` three times gave `bench.cpu_ms` 2110, 2094, and 2110, so 422.0, 418.8, and 422.0 ms per match (median 422.0). Per tick 1.4933, 1.482, and 1.4933 µs. Peak memory 6.801, 6.645, and 6.793 MB. 282,600 ticks per match. Exit 0 each time.
- Seed-42 tick records (`simulate --seed 42 --minutes 90 --json`): `8120abc554836ec9a4691e0afe25386a0ccc137417a6b9522ec5d73f2144adc0` before (twice) and after the change.
- Step 2 gate: `spike_decide` 1.1488 µs median (exit 0).
- Step 15 recheck: `sample_decide` 1.3158 µs, `sample_card` 869.31 ns.
- AC-4, sample pack: 428.2, 437.6, and 431.4 ms per match (median 431.4, 1.022 times the baseline, limit 1.10). Per tick 1.5425, 1.5764, and 1.554 µs. 277,600 ticks per match. Peak memory 8.293, 8.262, and 8.406 MB (median 1.22 times, limit 1.25). `script.pack` is `sample@1.0.0+898636c32a99`. `build.hash` is `3066066-dirty`, the parent build with this change uncommitted. Exit 0 each time.
- Without a pack, same build: 425.0, 421.8, and 425.0 ms per match (median 425.0, 1.007 times, limit 1.02).
- `script.calls` per match with the sample pack: 3,704 (seed 42, 90 minutes), with 0 aborts, 0 denials, and 0 disabled hooks. The score was 0-0, with 4 shots each.
- `cargo fmt --all -- --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings` exit 0; `cargo test --workspace --no-fail-fast` exit 0 with 398 passed, 0 failed, and 4 ignored; `node --test web/tests/*.test.mjs` 127 passed and 0 failed.
- Evidence: `implement-evidence/scripting-runtime/` (bench stdout, stderr, and exit codes; tick hashes; criterion output; test, fmt, and clippy logs).

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine scripting-runtime`. AC-1 to AC-3 are cargo tests. AC-4 re-runs the six benchmark drives on the reference machine against the 422.0 ms baseline recorded here. Compact the session first; the workflow state lives in the artifact files, and the SessionStart hook re-reads it after compaction.
- **Option B:** `/wf review football-manager-match-engine scripting-runtime`. Do this only if verify is judged redundant with the checks above. It is not recommended, because AC-4 is runtime evidence.
- **Option C:** `/wf plan football-manager-match-engine scripting-runtime`. Do this only if the product owner wants the scripted-manager hook (queued changes from a script) inside this slice.
