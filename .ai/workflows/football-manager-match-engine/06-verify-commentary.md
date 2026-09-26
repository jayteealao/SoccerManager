---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: commentary
status: complete
stage-number: 6
created-at: "2026-09-23T00:07:54Z"
updated-at: "2026-09-23T00:07:54Z"
result: pass
metric-checks-run: 13
metric-checks-passed: 13
metric-acceptance-met: 4
metric-acceptance-total: 4
metric-acceptance-user-observable: 0
metric-acceptance-code-only: 4
metric-interactive-checks-run: 0
metric-interactive-checks-passed: 0
metric-issues-found: 0
metric-issues-found-initial: 0
metric-issues-found-final: 0
fix-rounds-run: 0
convergence: not-needed
verify-owned-fix-commit: null
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: not-applicable
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/commentary/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped"
ac-staleness-checked: false
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 0
adversarial-tests-run: 4
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
  - "No visual change: the diff of the build commit against its base touches no file under web/, so no layout, typography, palette, or token rule of the standing design direction applies."
  - "Output boundary: a vocabulary scan of every added line in crates/, content/, docs/, web/, and README.md, and of the build commit message, found no workflow term."
tags: [engine, commentary, text, content, protocol]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-commentary.md
  plan: 04-plan-commentary.md
  implement: 05-implement-commentary.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-commentary.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine commentary"
---

# Verify: Context-Aware Commentary

## The Verification

Implement handed over commit `a414df5` and its record commit `846c48f`: a per-match commentator in the engine crate, 168 English lines in 54 template sets, a loader that refuses a thin or broken file, and the scorer and taker named on goal, kick-off, and restart events. The slice has four criteria, and each carries an explicit `observable: false` annotation, so the runtime gate does not apply. Cargo serialises on its target folder, so this run did every check in sequence, with no sub-agents.

Every check passed on the first run, so the fix loop had nothing to do. The workspace suite passed 251 tests with 0 failed and 3 ignored (38 result lines). The nine criterion and loader tests in `crates/engine/tests/commentary.rs` passed. To confirm that the engine change moves no tick, this run rebuilt the base commit `e79ad61` from a clean archive and simulated seed 42 with both binaries: the tick records after the 64-byte header hash to the same value (`05250c2d…`), and `cmp` finds only two differing bytes, at offsets 45 and 46, inside the header timestamp. Interleaved benchmark runs gave a median of 1.4453 µs per tick against 1.456 at the base (ratio 0.993, limit 1.10) and 6.309 MB peak memory against 6.199 MB (ratio 1.018, limit 1.25). The three slow release tests from the tactics slice also pass.

Review is next. The match-day screen can now render the `commentary` field. The top open risk is repetition over a whole match: about 40 throw-ins draw on 7 throw-in lines, so a viewer will see lines recur. The fix is more lines in the content file, with no code change.

## Verification Summary

- Slice: `commentary`; standard mode; branch `feat/football-manager-match-engine` at `846c48f` (confirmed with `git branch --show-current`).
- Stack: `user-confirmed: true`; the plan's stack source agrees.
- Constraint-resolution gate: the plan's `## Verification Strategy` names no environment dependency, so no criterion needs a `constraint-resolution:` line.
- Checks: 13 run, 13 passed. Criteria: 4 of 4 met, all `code-only`.
- Issues found: 0 initial, 0 final. Fix loop: not run (`convergence: not-needed`).

## Automated Checks Run

1. `cargo fmt --all -- --check`: pass (exit 0; `verify-evidence/commentary/fmt.txt`).
2. `cargo clippy --workspace --all-targets -- -D warnings`: pass (exit 0, no warning; `clippy.txt`).
3. `cargo test --workspace`: pass. 38 result lines, 251 passed, 0 failed, 3 ignored (`test-workspace.txt`).
4. `cargo test -p engine --test commentary`: pass, 9 of 9 (`test-commentary.txt`).
5. `cargo test -p engine-cli stream_run`: pass. `stream_run::tests::every_play_event_carries_a_commentary_line` passed (`test-stream-run.txt`).
6. `cargo test -p protocol`: pass. 25 + 2 passed, 0 failed (`test-protocol.txt`); the event round-trip carries the `commentary` key.
7. `node --test web/tests/*.test.mjs`: pass. 47 passed, 0 failed (`node-test.txt`).
8. Tick-record identity: pass. `engine-cli simulate --seed 42` from HEAD and from a clean build of `e79ad61`; `tail -c +65 | sha256sum` gives `05250c2d87f7…cd42a4` for both (`ticks-body-head.sha256`, `ticks-body-base-e79ad61.sha256`).
9. Benchmark compare, interleaved, 3 runs each of `engine-cli bench --seed 42 --matches 5 --json`: pass. `budget.pass` true in all six (`bench-base-*.json`, `bench-head-*.json`).
10. Stream-path benchmark, `bench --matches 1 --stream --json`, once each: pass. 618,239 ticks per second at HEAD against 620,632 at the base (0.4 percent, within run noise); `budget.pass` true (`bench-stream-*.json`).
11. Cross-slice slow tests, `cargo test --release -p engine -- --ignored`: pass. `a_trailing_ai_team_changes_its_tactics`, `an_attacking_team_shoots_more_than_a_defensive_one`, `a_stronger_team_wins_more_than_half_its_matches` (`test-slow-ignored.txt`).
12. Secret scan, `gitleaks git --log-opts=e79ad61..a414df5`: pass, no leaks (`gitleaks.txt`, `gitleaks.json`).
13. Vocabulary and marker scan over the added lines of `a414df5` and its message: pass. 0 workflow-term matches (`vocab-scan.txt` is empty); 0 `sdlc-debt` markers.

## Interactive Verification Results

Automated only. Each of the four criteria carries `observable: false` in `03-slice-commentary.md`, and the plan names no interactive step. The shape's user-observable commentary criterion (the feed updates within one frame of the goal tick) belongs to `viewer-match-day`. The `cli` adapter was used only to run `simulate` and `bench` for the regression and benchmark checks.

## Acceptance Criteria Status

| # | Criterion | Kind | Status | Method | Evidence | Evidence rung |
|---|---|---|---|---|---|---|
| AC-1 | "Given any event kind in the rule set, Then a commentary line is produced that names the player and team involved." | code-only | met | automated | `every_event_kind_gets_a_line_naming_its_player_and_club`, `every_shipped_line_names_the_player_and_the_club`, `a_full_seeded_match_fills_every_placeholder_and_names_every_player` pass (`test-commentary.txt`) | n-a |
| AC-2 | "Given the same event kind five times in ten minutes, Then at least three distinct lines appear." | code-only | met | automated | `five_events_of_one_kind_in_ten_minutes_give_three_distinct_lines` passes, looped over all 15 commented kinds | n-a |
| AC-3 | "Given a goal that levels the score in minute 88 … late equalizer; … 4 to 0 in minute 20 … rout." | code-only | met | automated | `a_late_equaliser_and_a_rout_read_as_such` passes; it compares the line against the filled lines of the sets whose conditions are late-equaliser and rout, not against keywords | n-a |
| AC-4 | "Given a full seeded match, Then no template variable is left unfilled in any line." | code-only | met | automated | the full seed-42 match test asserts no `{` or `}` in any line, more than 100 lines, and 0 fallbacks | n-a |

Evidence: n-a 4 (all code-only). User-observable criteria with a mock or static rung: 0.

Notes on AC-1. The engine has 17 event kinds. The 15 commented kinds each get a line. The two change verdicts (`ChangeApplied`, `ChangeRejected`, sent as `tactics-change`) get none. The plan decided this in its contract paragraph and assumption 5: a verdict acknowledges the manager's own request and carries no player. A substitution the manager makes still gets a line through its own `substitution` event. The `ai-decision` event carries no player in the engine contract, so its lines name the club. The test checks the player wherever the event carries one.

## Issues Found

None.

## Augmentation Verification

- Mock fidelity (`02c-craft.md`): not applicable. `git diff --stat e79ad61 a414df5 -- web` is empty, so no mock-fidelity item binds this slice.
- Instrumentation (`04b-instrument.md`): the one new signal, `commentary.fallback` (`crates/engine/src/commentary/mod.rs:165`), is plan-sanctioned (plan step 12). Its path runs in the unit test that expects the built-in line `corner: Alpha.` (`mod.rs:367`). The full-match test asserts that the seed-42 match emits no fallback. The catalog was not re-authored, as the plan records.
- Experiment (`04c-experiment.md`): deferred to `experiment-flags`; no re-check.
- Benchmark compare (`05c-benchmark.md`): pass. Base medians 1.456 µs per tick and 6.199 MB; HEAD medians 1.4453 µs per tick (ratio 0.993, limit 1.10) and 6.309 MB (ratio 1.018, limit 1.25). This matches the implement record (0.993 and 1.026).

## Security Scan

- CVE scan: `cargo audit` is not installed (`error: no such command: audit`). The commit changes no `Cargo.toml` and no `Cargo.lock` line (`git diff --stat e79ad61 a414df5 -- Cargo.lock '**/Cargo.toml'` is empty), so no dependency entered with this slice. New critical or high: 0.
- Secret detection: gitleaks 8.30.1, 1 commit scanned, no leaks.
- SAST by inspection: the new module has no `unsafe`. Every `unwrap()` in `crates/engine/src/commentary/` is inside a test. The loader reads one fixed content path and refuses unknown placeholders.

## Accessibility Gate

Not automatable for this slice: no page file changed. New violations: 0.

## Performance Gate

- Bundle size: skipped; no web change.
- Processor time per tick: ratio 0.993 against the base, measured this run.
- Peak memory: ratio 1.018 against the base, measured this run.
- Stream path (the path that runs the commentator): 0.4 percent lower throughput, within run noise.

## Cross-Slice Regression

Siblings checked: `engine-core`, `data-schemas-generator`, `stream-protocol`, `viewer-pitch`, `match-rules`, `tactics-and-ai`. Each sibling's tests are in the workspace suite (251 passed) and the page suite (47 passed). The three slow release tests from `tactics-and-ai` pass. The seed-42 tick records are identical to the base. Regressions found: 0.

## Longitudinal Delta

- Tick file: baseline source is a clean build of `e79ad61`. Delta: none in the records; two header bytes (the match timestamp). Expected.
- Benchmark: baseline source is the same base build, run interleaved. Delta: within limits. Expected.

## Friction Notes

- The throw-in has 4 unconditional lines and a 3-line streak set. A match has about 40 throw-ins, so lines recur after the ten-minute window. This is informational; more lines in `content/commentary/en.json` fix it without code.

## Free Exploration Notes

- An applied mentality change by the human manager produces a `ChangeApplied` verdict and no line. A substitution produces a line through its `substitution` event. Informational: this follows the plan's decision, and the match-day screen shows the verdict as a state chip.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| A kind with only two unconditional lines | pass | refused with `event corner has 2 lines` |
| An unknown placeholder (`{referee}`) | pass | refused with the kind named |
| A placeholder the kind cannot fill (`{other_player}` on a goal) | pass | refused |
| Another schema version | pass | refused with `schema_version 2; this build reads 1` |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Slow response | n-a | no network surface in this slice |
| Concurrent session | n-a | one commentator per driven match |
| Session expiry | n-a | no session |

## Gaps / Unverified Areas

- No dependency CVE scanner is installed. No dependency changed, so the gap does not affect this slice.
- Commentary for the `simulate` and `resume` commands is not wired, because those commands route no events today. The implement record defers this to the slice that makes the event mapping shared.

## Freshness Research

Not required. No test failed, the plan is less than 14 days old (2026-09-22), and the slice touches no external API. The garde 0.23 validation signature was confirmed by the passing build and the refusal tests.

## Recommendation

Proceed to review. All four criteria are met by tests that assert the property directly. The engine change moves no tick, and the cost is within both benchmark limits.

## Assumptions and Decisions

1. `class: implementation-detail`. The four criteria are `code-only` because each carries an explicit `observable: false` annotation, which the partition rule treats as final. Classification for AC-1 to AC-4: build-capability.
2. `class: implementation-detail`. AC-1 is met with the two change verdicts excluded. The plan settled this before implement (contract paragraph and assumption 5); verify checks the build against the plan and does not reopen it.
3. `class: implementation-detail`. The tick-identity check was re-derived from a clean `git archive` build of `e79ad61` in the session scratch folder. It was not taken from the implement record's hashes. A `git worktree add` attempt failed with exit 128 and was replaced by the archive build; the repository was not changed.
4. `class: implementation-detail`. The benchmark was run interleaved against the rebuilt base binary, so both sides ran under the same machine state.
5. `class: implementation-detail`. `security-scan-result: pass` rests on the secret scan and on no dependency change, because `cargo audit` is not installed.
6. `class: implementation-detail`. The run found no failing check and no unmet criterion, so the fix loop did not run and no question was needed.

## Recommended Next Stage

- **Option A (default):** `/wf review football-manager-match-engine commentary`. `result: pass`, `convergence: not-needed`. Review is slug-wide, so the pending reviews of the earlier slices share the same ledger.
- **Option D:** `/wf handoff football-manager-match-engine commentary`. Valid because `result: pass`, but not recommended: the slice changes engine events and the wire contract.
- **Option G:** `/wf probe football-manager-match-engine`. Optional slug-wide runtime sweep.
