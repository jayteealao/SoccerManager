---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: scripting-runtime
status: complete
stage-number: 6
created-at: "2026-09-23T17:59:05Z"
updated-at: "2026-09-23T17:59:05Z"
result: pass
metric-checks-run: 12
metric-checks-passed: 12
metric-acceptance-met: 4
metric-acceptance-total: 4
metric-acceptance-user-observable: 1
metric-acceptance-code-only: 3
metric-acceptance-mock-rung: 0
metric-interactive-checks-run: 1
metric-interactive-checks-passed: 1
metric-issues-found: 0
metric-issues-found-initial: 0
metric-issues-found-final: 0
fix-rounds-run: 0
convergence: not-needed
verify-owned-fix-commit: null
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/scripting-runtime/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped"
ac-staleness-checked: true
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 0
adversarial-tests-run: 7
adversarial-tests-failed: 0
failure-mode-probes-run: 0
cross-browser-delta: "none"
web-vitals-lcp-ms: null
web-vitals-cls: null
web-vitals-inp-ms: null
stack-source: confirmed
debt-markers-found: 0
debt-markers-malformed: 0
debt-markers-unrecorded: 0
skipped-gating-specs: []
consult-runs: []
steering-honored:
  - "Design direction: this slice changes no page, style, token, or layout file; the page suite passes 127 of 127 unchanged."
  - "Dark-path counter definition: untouched; without a pack the seed-42 match dump is byte-identical to the parent build 3066066 (8120abc5...adc0), so the computer manager's queuing is unchanged."
  - "Output boundary: a vocabulary scan over the build and record commits' added lines outside the workflow folder, and over both commit messages, found 0 workflow terms."
tags: [engine, modding, scripting, sandbox, rhai, benchmark, rim-6]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-scripting-runtime.md
  plan: 04-plan-scripting-runtime.md
  implement: 05-implement-scripting-runtime.md
  benchmark: 05c-benchmark.md
  review: 07-review-scripting-runtime.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine scripting-runtime"
---

# Verify: Scripting Runtime for Modding

## The Verification

Implement handed over commit `7fd5197` and its record commit `e74ee1b`. The engine now has a plugin interface with a decision hook, a rule hook, and a commentary hook, and a new `script` crate runs Rhai packs behind it in a sandbox. AC-1 to AC-3 carry `observable: false`, so they are verified by named `cargo test` cases. AC-4 is the one user-observable criterion: a benchmark report on the reference machine. Cargo serialises on its target folder, so this run did every check in sequence, with no sub-agents.

Every check passed on the first run, so the fix loop had nothing to do. The workspace suite passed 398 tests with 0 failed and 4 ignored over 56 result lines, and the 21 hook, sandbox, sample, and command-line tests passed by name. For AC-4 this run did not reuse the recorded baseline. It rebuilt the parent commit `3066066` from a `git archive` and ran it three times, interleaved with three plain runs and three sample-pack runs of the new build. The sample pack's median is 431.4 ms per match against a fresh baseline of 415.8 ms (1.038 times, limit 1.10) and against the recorded baseline of 422.0 ms (1.022 times). Without a pack the new build reads 415.8 ms (1.000 times), and the seed-42 match dump hashes to `8120abc5…adc0`, the same as the parent.

Review is next. The top open risk is peak memory with a pack. The median is 8.391 MB, which is 1.235 times the plan's recorded baseline of 6.793 MB (limit 1.25). Against this run's fresh parent reading of 6.648 MB it is 1.262 times. The parent's own memory varies from 6.645 to 6.801 MB between sessions, so the limit sits inside the measurement noise.

## Verification Summary

- Slice: `scripting-runtime`, standard mode, on the dedicated branch `feat/football-manager-match-engine` at HEAD `e74ee1b`.
- Stack: `stack.user-confirmed: true`, and the plan has `stack-source: confirmed`.
- Constraint-resolution gate: AC-4 names the reference machine and carries `constraint-resolution: prerequisite-slice: calibration`. Calibration is verified (`06-verify.md`), so nothing is missing (`constraint-resolution-missing: []`).
- No prior `06-verify-scripting-runtime.md` existed, and the evidence folder was empty. This is evidence run 1, so nothing was archived.
- Code under test equals `7fd5197`. `git diff --stat HEAD` over `crates`, `content`, `schemas`, `web`, and `docs` shows only the staged `docs/design/realism/*` files from another session. The staged-then-deleted `crates/engine/tests/zz_stall_probe.rs` is not in the working tree, so no check compiled it.
- Issues found: 0 initial and 0 final. The fix loop did not run (`convergence: not-needed`).

## Automated Checks Run

1. `cargo fmt --all -- --check`: pass, exit 0 (`fmt.txt`).
2. `cargo clippy --workspace --all-targets -- -D warnings`: pass, exit 0, no warning (`clippy.txt`).
3. `cargo test --workspace --no-fail-fast`: pass. 398 passed, 0 failed, 4 ignored over 56 result lines, exit 0 (`test-workspace.txt`). This is the same count as the implement record.
4. AC tests by name: 21 of 21 passed (`test-ac.txt`). The set is `script --test decision` (4), `--test sandbox` (5), `--test sample` (3), `engine --test plugin_hooks` (4), and `engine-cli --test script_cli` (5).
5. `node --test web/tests/*.test.mjs`: pass. 127 tests, 0 fail, 0 skipped (`web-tests.txt`).
6. `cargo build --release -p engine-cli`: pass, exit 0 (`release-build.txt`).
7. Parent baseline: `3066066` was extracted with `git archive` into the scratchpad and built with its own target folder. `engine-cli bench --seed 42 --matches 5 --json` ×3 read 415.6, 415.8, and 418.8 ms per match (median 415.8) and 6.648, 6.648, and 6.645 MB (median 6.648). All three exits were 0 (`baseline-bench-*.stdout.txt`).
8. New build without a pack, same command ×3: 415.8, 421.8, and 412.6 ms (median 415.8, 1.000 times) and a median of 6.680 MB (1.005 times). All three exits were 0 (`plain-bench-*`).
9. New build with `--script-pack content/scripts/sample` ×3 (AC-4): 428.0, 437.6, and 431.4 ms (median 431.4) and 8.266, 8.391, and 8.391 MB (median 8.391). Each run had `script.pack` `sample@1.0.0+898636c32a99`, `build.hash` `e74ee1b-dirty`, `budget.pass: true`, and exit 0 (`scripted-bench-*`, `bench-summary.txt`). The three configurations ran interleaved, one of each per round.
10. Default-match identity: `simulate --seed 42 --minutes 90 --json --no-snapshot` with the parent binary and with the new binary gives the same JSON Lines dump SHA-256, `8120abc554836ec9a4691e0afe25386a0ccc137417a6b9522ec5d73f2144adc0`. This is also the value in the implement record (`seed42-identity.txt`).
11. Sample-pack match drive: the same command with `--script-pack content/scripts/sample` exits 0. The statistics record carries `script.pack` `sample@1.0.0+898636c32a99`, `script.calls` 3704, and 0 aborts, 0 denials, and 0 disabled hooks. stderr carries one `script.loaded` signal. The dump hash differs from the no-pack match (`ddbde43b…`), so the offsets changed the play (`seed42-identity.txt`).
12. Security, dependency, and boundary scan (`security-boundary.txt`): the new packages in `Cargo.lock` are rhai, rhai_codegen, and their pure-Rust dependencies (bitflags, const-random, const-random-macro, getrandom, no-std-compat, portable-atomic, smartstring, spin, thin-vec, tiny-keccak, wasi, web-time), plus the workspace's own `script`. Other scans found 0 added `unsafe` blocks, 0 secret-pattern hits, 0 workflow-vocabulary hits, and 0 `sdlc-debt` markers. The only file access in `crates/script/src` is the pack loader in `pack.rs`. That loader refuses an entry name with a separator or `..`, and refuses an entry that canonicalises outside the pack folder.

## Interactive Verification Results

- **Criterion:** AC-4, "Given the benchmark reruns with the sample script loaded, Then CPU time per match is within 10 percent of the calibration baseline."
- **Platform & tool:** the `cli` adapter (cli-direct) on the reference machine (AMD Ryzen 7 9800X3D, the Ultimate Performance power plan). This is the machine on which the implement baseline ran.
- **Steps performed:** (1) build the new release binary. (2) Build the parent `3066066` release binary from a `git archive`. (3) Do three rounds. Each round runs the parent bench, then the new bench with no pack, then the new bench with the sample pack, all with `--seed 42 --matches 5 --json`. (4) Read each stdout JSON line and compute the median of `bench.cpu_ms / bench.matches`.
- **Evidence:** `verify-evidence/scripting-runtime/baseline-bench-{1,2,3}.*`, `plain-bench-{1,2,3}.*`, `scripted-bench-{1,2,3}.*`, and `bench-summary.txt`.
- **Observation:** the scripted median was 431.4 ms against 415.8 ms for the fresh parent (1.038 times) and 422.0 ms for the implement baseline (1.022 times). Every other plan pass criterion held: `script.pack` starts with `sample@1.0.0+`, all exits were 0, and `build.hash` is `e74ee1b-dirty`, not `unknown`. Peak memory was 8.391 MB, 1.235 times the recorded step 1 baseline of 6.793 MB (the plan's limit is 1.25). Processor time per tick was 1.554 µs against 1.4713 µs (1.056 times), over 277,600 ticks against 282,600.
- **Result:** pass.

## Acceptance Criteria Status

| # | Criterion | kind | status | method | evidence | evidence-rung |
|---|-----------|------|--------|--------|----------|---------------|
| AC-1 | "Given a script pack that overrides the decision scoring hook, When a seeded match runs, Then the scripted decisions take effect and the event stream records the pack identifier." | code-only (explicit `observable: false`) | met | automated | `the_shoot_bias_pack_takes_more_shots_in_the_seeded_match`, `a_carrier_in_range_shoots_instead_of_passing_with_the_pack`, `the_decision_hook_turns_the_carriers_choice_into_the_option_it_favours`, and `the_pack_identity_reaches_the_first_kick_off_and_the_match_statistics` pass (`test-ac.txt`). The CLI drive gives `script.pack` `sample@1.0.0+898636c32a99` and 3704 calls, and its dump differs from the no-pack dump (`seed42-identity.txt`). | n-a |
| AC-2 | "Given a script that exceeds its time budget on a tick, Then the engine aborts that hook, logs it, and continues with the default decision." | code-only (explicit `observable: false`) | met | automated | `a_looping_decision_hook_is_aborted_logged_and_the_native_decision_stands` passes. It asserts 3 `aborted` notes ("operation budget of 10000 exhausted") and then `disabled`, a `script.aborted` log line, and tick records equal to the unscripted match (`crates/script/tests/sandbox.rs:75`, `test-ac.txt`). | n-a |
| AC-3 | "Given a script that attempts file or network access, Then the sandbox denies it and the engine records the denial." | code-only (explicit `observable: false`) | met | automated | `an_import_is_denied_and_recorded_and_the_match_goes_on` (`import ../../../Cargo is not allowed`, a `script.denied` log, 3 denials) passes. So do `a_network_call_is_denied_naming_the_function` (`function http_get is not available`) and `a_denied_hook_is_a_script_event_in_the_stream` (`test-ac.txt`). | n-a |
| AC-4 | "Given the benchmark reruns with the sample script loaded, Then CPU time per match is within 10 percent of the calibration baseline." | user-observable (explicit `observable: true`) | met | interactive | Scripted median 431.4 ms against the fresh parent's 415.8 ms (1.038 times) and the recorded 422.0 ms (1.022 times), limit 1.10 (`bench-summary.txt`). | live |

Rollup: evidence live 1, n-a 3. User-observable 1, mock-rung 0.

## Issues Found

None.

## Augmentation Verification

- **Benchmark (`05c-benchmark.md`, which gates the default match against the tactics-and-ai baseline):**
  - Processor time per match with no pack: median 415.8 ms against the 460.7 ms tripwire. Pass.
  - Peak memory with no pack: median 6.680 MB against the 6.82 MB tripwire. Pass.
  - Per tick with no pack: 1.4713 µs, reported beside the gate.
  - This slice's own AC-4 run adds the scripted readings above. A scripted match is outside the scope of the 05c default-match gate, and no tripwire fired.
- **Instrumentation (`04b-instrument.md`):** the new signals fired at run time. `script.loaded` appeared on the pack drive. `script.aborted`, `script.denied`, and `script.disabled` are asserted in log captures by the sandbox tests. The `match-stats` record carries `script.pack` and the four counters. Validating that record against the schema is covered by the workspace suite's schema tests, which pass.
- **Experiment (`04c-experiment.md`):** no flag is involved in this slice.
- **Craft (`02c-craft.md`):** no visual surface changed, so no mock-fidelity item applies.

## Security Scan

- CVE: `cargo audit` is not installed, as recorded for earlier slices. The new dependencies are rhai 1.26.1 and its pure-Rust dependency tree. The workspace `licenses` test passes with them (it is inside check 3). New critical/high issues known to this run: 0.
- Secrets: the pattern scan over the added lines found 0 hits.
- SAST: clippy `-D warnings` is clean, and the change adds 0 `unsafe` blocks. The sandbox has an operation budget, a 2 ms wall-clock backstop, and call and expression depth limits. It also has string, array, and map caps, Rhai's refusing module resolver, and `eval` disabled. A script cannot reach files or the network: an import is refused, an unknown function is denied, and the three adversarial fixtures prove this at run time. New HIGH+ issues: 0.

## Accessibility Gate

Not automatable for this slice. No UI element, style, or text surface changed. New WCAG AA violations: 0.

## Performance Gate

- Bundle size delta: skipped, because no page file changed.
- Processor time per match: the no-pack build equals the parent (1.000 times), and the sample pack costs 1.038 times the parent. Peak memory is covered under Gaps.

## Cross-Slice Regression

The workspace suite covers every sibling slice's Rust tests, and all 398 pass. Boxing `ServerMessage::Event` touched the stream crate, and its tests pass. The page suite covers the four viewer slices, and all 127 pass. The seed-42 default match is byte-identical to the parent build. Regressions found: 0.

## Longitudinal Delta

- Default match: compared with the parent `3066066`, rebuilt this run. The dump is byte-identical, which is the expected result.
- Benchmark: compared with the implement record's readings. The scripted median is 431.4 ms in both runs. The plain median is 415.8 ms in this run and 425.0 ms in the implement run, which is session noise. The memory medians are 8.391 MB now and 8.29 MB in the implement run.

## Friction Notes

- The parent binary built from a `git archive` reports `build.hash: unknown` because it has no git folder. The plan's `build.hash` rule applies to the scripted run, which carries `e74ee1b-dirty`. Informational.

## Free Exploration Notes

- The sample-pack seeded match ended 0-0, and the no-pack match ended 2-1. The decision offsets changed the course of the match, as AC-1 intends. The commentary hook's goal-line rewrite is covered by `a_goal_line_gains_its_time_of_the_match`, not by this match. Informational.
- A `git worktree add` of the parent failed on this Windows checkout with "Filename too long", caused by a long evidence path under the workflow folder. It left no worktree registered (`git worktree list` shows only the main tree). `git archive` of the code paths was used instead. Informational.

## Adversarial Tests

| Test | Result | Finding |
|------|--------|---------|
| Endless loop in the decision hook | pass | `a_looping_decision_hook_is_aborted_logged_and_the_native_decision_stands` |
| Import that climbs out of the pack folder | pass | `an_import_is_denied_and_recorded_and_the_match_goes_on` |
| Network function call | pass | `a_network_call_is_denied_naming_the_function` |
| Bad return value | pass | `a_bad_return_value_is_aborted` |
| `eval`, or a listed hook with no function | pass | `a_pack_that_uses_eval_or_lacks_its_hook_function_is_refused_at_load` |
| Malformed pack manifest | pass | `a_bad_pack_is_refused_naming_the_field` |
| Resume a scripted snapshot with another pack | pass | `a_scripted_snapshot_resumes_only_with_the_same_pack` |

## Failure Mode Probes

| Probe | Result | Finding |
|-------|--------|---------|
| Slow response / concurrent session / session expiry | n-a | engine-only slice, with no session or network surface |

## Gaps / Unverified Areas

- Peak memory with the sample pack is close to the plan's limit. The median is 8.391 MB. Against the plan's step 1 baseline (6.793 MB) that is 1.235 times, which passes the 1.25 limit. Against this run's fresh parent reading (6.648 MB) it is 1.262 times. The parent's memory ranged from 6.645 to 6.801 MB across sessions at the same commit, so the limit sits inside the noise. AC-4 names CPU time only, and CPU passes both ways. The implement record already names `Engine::new_raw()` with only the needed packages as the way to cut memory. That change would narrow the functions available to modders, so it is a scope decision and not a verify patch.
- The 2 ms wall-clock backstop is not deterministic across machines. The tests are stopped by the operation budget first, so they do not exercise the backstop.
- `cargo audit` is not installed.
- The observability contract list in `.ai/observability.md` does not yet name the `script` event type or the `script.*` keys. The implement record defers this. The schema files do carry them.

## Freshness Research

No dependency version changed after implement. The implement record read the installed rhai 1.26.1 source for the error variants, `on_progress`, the refusing resolver, and `disable_symbol`. This run observed those behaviours at run time: the budget abort text, the import refusal, and the unknown-function denial. No further source read was needed.

## Assumptions

Each entry is an autonomous decision of this run, with its class per `_decision-classes.md`.

- **V-1** AC-1 to AC-3 are code-only because of their explicit `observable: false` annotations (partition Step A, which is final). Each is classified `build-capability` and verified by `cargo test`. `class: implementation-detail`.
- **V-2** AC-4 is user-observable because of its explicit `observable: true` annotation. It is classified `runtime-evidence` and was produced live on the reference machine through the cli adapter, so no deferral applies. `class: implementation-detail`.
- **V-3** The AC-4 baseline was re-measured this run by rebuilding the parent `3066066`, and the three configurations ran interleaved. The recorded 422.0 ms was not the only comparison. Both comparisons are reported. `class: implementation-detail`.
- **V-4** Peak memory is judged against the plan's own wording, "1.25 times the step 1 baseline" (6.793 MB), which gives 1.235 and a pass. The fresh-parent ratio (1.262) is recorded under Gaps and not as an issue: the criterion names CPU only, and the plan binds the memory check to the step 1 figure. `class: implementation-detail`.
- **V-5** No sub-agents were dispatched, because cargo serialises on one target folder. `class: implementation-detail`.
- **V-6** The driver said a prior round had applied one fix pass, but nothing on disk shows a verify artifact or verify evidence for this slice. This run is evidence run 1 with `fix-rounds-run: 0`. `class: implementation-detail`.
- **V-7** No consult ran. No trigger holds: every criterion is met by direct evidence, none is deferred, and the product owner excluded consult at intake. `class: implementation-detail`.
- **V-8** `security-scan-result: pass` rests on the secret scan, clippy, the `unsafe` count, the licence test, and the three sandbox fixtures. `cargo audit` is absent and was not installed. `class: implementation-detail`.

## Triage Decisions

None. With 0 issues, no Fix, Skip, or Escalate decision was needed.

## Recommendation

Ready for review. The three code criteria are met by named, passing tests. The benchmark criterion is met live against a parent build rebuilt this run. Without a pack, the default match is byte-identical to the parent.

## Recommended Next Stage

- **Option A (default): Review** → `/wf review football-manager-match-engine scripting-runtime`. Convergence is `not-needed` and the result is `pass`. Review should look closely at the sandbox and the memory headroom.
- **Option D: Skip review** → `/wf handoff football-manager-match-engine scripting-runtime`. It is valid because `result: pass`, but it is not recommended. The slice adds a sandbox that runs third-party code, a new dependency tree, and 58 changed files.
- **Option G: Slug-wide runtime probe** → `/wf probe football-manager-match-engine`. Use it for a cross-slice runtime sweep.
