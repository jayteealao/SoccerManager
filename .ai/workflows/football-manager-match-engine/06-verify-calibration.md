---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: calibration
status: complete
stage-number: 6
created-at: "2026-09-23T01:14:00Z"
updated-at: "2026-09-23T07:28:00Z"
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-23T07:28:00Z"
    trigger: answers-returned
    because: "the product owner answered the AC-d counter definition (2026-09-23T07:00:32Z): changes left waiting at full time are expired, not never applied"
    changed: "second verify run; DARKPATH-2 fixed and committed at 3ec2198 with two regression tests; the 1000-match drive re-run on the final code; result fail to pass, convergence escalated to converged; the first run's fix aedf0e3 and evidence kept in calibration-run-2/"
result: pass
metric-checks-run: 14
metric-checks-passed: 14
metric-acceptance-met: 5
metric-acceptance-total: 5
metric-acceptance-user-observable: 1
metric-acceptance-code-only: 4
metric-acceptance-mock-rung: 0
metric-interactive-checks-run: 1
metric-interactive-checks-passed: 1
metric-issues-found: 0
metric-issues-found-initial: 1
metric-issues-found-final: 0
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: "3ec2198"
regression-tests-added: 2
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/calibration/"
evidence-run-count: 2
security-scan-result: skipped
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped"
ac-staleness-checked: true
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 0
adversarial-tests-run: 0
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
  - "Dark-path counter definition (product owner, 2026-09-23): a change still queued at full time with no admitting stoppage after it is counted in change.expired_at_full_time (reported in match-stats and the calibration run report, no zero rule); darkpath.change_never_applied keeps its zero rule; .ai/observability.md key list, dark-paths block, and match-stats record amended; the computer manager's queuing behaviour is unchanged (ai.rs not touched by 3ec2198)."
  - "No visual change: neither fix commit touches a file under web/, so no layout, typography, palette, or token rule of the standing design direction applies."
  - "Output boundary: a vocabulary scan of the 177 added lines of 3ec2198 found no workflow term (one hit, the product key darkpath.change_never_applied, is a contract key name); the commit message uses product language only."
tags: [engine, calibration, statistics, observability, dark-path]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-calibration.md
  plan: 04-plan-calibration.md
  implement: 05-implement-calibration.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-calibration.md
  adapters: runtime-adapters.md
  prior-revision: history/06-verify-calibration-0.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine calibration"
---

# Verify: Calibration Harness and Statistics Record

## The Verification

The first verify run left this slice at `fail`. The harness, the records, the schemas, and the tuning held every band. One criterion did not hold: the dark-path counter `darkpath.change_never_applied` read 288 over 2000 matches. That run fixed the larger cause in `aedf0e3`: the computer manager no longer re-requests a refused injury substitution, and the count fell to 105. The remaining 105 were changes the computer manager queued after the last stoppage of a match. Whether those count as "never applied" was a product-owner question, so the run escalated it. The product owner has now answered: such a change is *expired at full time*, not never applied, and it goes in its own counter with no zero rule.

This run first reproduced the starting point on the current head. The 1000-match drive on seed 2026 gave the same band values and 105 unapplied changes (exit 2). The fix (`3ec2198`) makes the change queue record the tick of the latest stoppage that admitted each change kind, and the snapshot keeps that tick. At full time, a waiting change is *never applied* only when an admitting stoppage opened after it was queued. Otherwise it is expired and counted in `change.expired_at_full_time`, which the match figures, the calibration report, and both schemas now carry. Two regression tests cover this. An end-to-end test fails with the old counting (left 1, right 0) and passes with the fix. A unit test covers both branches. After the fix, the same drive exits 0 with `calib.pass: true`, `darkpath.change_never_applied` 0, and `change.expired_at_full_time` 105. Every band value is the same as before the fix, and so are the ticks per benchmark match. All 5 criteria are met on direct evidence from this run.

The slice can go to review. The computer manager still queues changes late in a match that never reach a stoppage (105 over 2000 matches). They are now reported in the open, not hidden, but the tactics behaviour behind them is unchanged, as the product owner directed. The top open risk is still the goals distribution: the mean is in band, and the spread is wide.

## Verification Summary

- Branch `feat/football-manager-match-engine`. Verified at `aedf0e3` (the starting point for this run) and at `3ec2198` (the fix). The index held staged files from other work (`docs/design/realism/*`, `crates/engine/tests/zz_stall_probe.rs`), and none of them went into the fix commit.
- Stack confirmed (`stack.user-confirmed: true`). The plan's `stack-source` agrees. Standard mode.
- The earlier evidence moved to `verify-evidence/calibration-run-2/`. This run's evidence is in `verify-evidence/calibration/`.
- Checks: 14 run, 14 pass on the final code. Before the fix, the calibration drive failed at exit 2 (`pre-ac-e.*`). That failure is the one issue this round took in.
- Criteria: 5 of 5 met. The one user-observable criterion (AC-e) has live `cli-direct` evidence. There is no deferral.
- The cargo target folder serialises builds, so every check ran in sequence in this agent. No sub-agent was dispatched (see Assumptions).

## Automated Checks Run

1. Pre-fix baseline: `engine-cli calibrate --seed 2026 --matches 1000` (release) at `aedf0e3` exited 2. Every band passed. `darkpath.change_never_applied` was 105 and the `calibrate.darkpath` warning fired (`pre-ac-e.stdout.txt`, `pre-ac-e.stderr.txt`, `pre-ac-e.report.json`, `pre-ac-e.exit-code`). This is the issue the round took in, not a check on the final code.
2. `cargo fmt --all -- --check`: pass, exit 0 (`fmt.txt`, `fmt.exit-code`).
3. `cargo clippy --workspace --all-targets -- -D warnings`: pass, exit 0 (`clippy.txt`, `clippy.exit-code`).
4. `cargo test --workspace`: pass. 274 passed, 0 failed, 4 ignored (`tests-default.txt`). The two new tests make 274, against 272 in the first run. The 4 ignored tests are the slow criteria tests.
5. `node --test web/tests/*.test.mjs`: pass, 47 of 47 (`node-tests.txt`).
6. `cargo build --release -p engine-cli`: pass (`release-build.txt`).
7. `engine-cli calibrate --seed 2026 --matches 1000` (release, final code): pass, exit 0. `calib.pass: true`, `darkpath.change_never_applied` 0, `change.expired_at_full_time` 105, `darkpath.match_without_stats` 0, `validate.violations` 0. No `calibrate.darkpath` warning was emitted: 0 matching lines in `ac-e.stderr.txt` (`ac-e.*`).
8. `cargo test --release -p engine-cli --test calibrate -- --ignored`: pass, exit 0. `a_thousand_matches_hold_the_realism_bands` passed in 160.37 s. It asserts both dark-path counters at 0 (`slow-calibrate.stdout.txt`).
9. Regression tests: with the fix, both pass (`regression-test-after-fix.txt`: `a_change_queued_after_the_last_admitting_stoppage_expires_at_full_time` and `a_change_an_admitting_stoppage_passed_is_never_applied_and_a_later_one_expired`). With the counting in `TacticsStats::new` reverted to the old line, the end-to-end test fails at `tactics_queue.rs:178`, left 1 and right 0 (`regression-test-before-fix.txt`).
10. The calibration smoke test (`crates/engine-cli/tests/calibrate.rs`) validates every statistics record and `report.json` against the amended schemas with `boon`. It now also checks that the report's `change.expired_at_full_time` equals the sum over the records. It passes in `tests-default.txt`.
11. Benchmark compare, `engine-cli bench --seed 42 --matches 5 --json` run three times: pass. Processor time per match was 409.4, 418.8, and 415.6 ms (median 415.6, against the 460.7 ms gate). Peak memory was 6.473, 6.438, and 6.402 MB (median 6.438, against 6.82 MB). Ticks per match were 282,600, the same as the first run (`bench-{1,2,3}.*`).
12. Instrument signals: pass. `calibrate.suite` fired for both suites. `calibrate.darkpath` stays silent when both counters read 0: it fired with value 105 before the fix and did not fire after it.
13. Output-boundary scan of the 177 added lines of the fix diff (`added-lines.txt`): pass. There is one match, the product key `darkpath.change_never_applied`, which is a contract key and not a workflow term.
14. Secret scan (key, secret, password, and token assignments in the added lines): pass, 0 hits. `sdlc-debt` marker scan: pass, 0 markers.

## Interactive Verification Results

- **Criterion:** AC-e, "Given 1000 matches on the reference laptop, Then wall time is under 30 minutes, and the report also records the single-thread per-match time from the benchmark harness."
- **Platform & tool:** Windows 11, with the release `engine-cli` binary run directly (`cli-direct`) on machine hash `74ca12fc08a4` (AMD Ryzen 7 9800X3D). All the benchmark baselines come from this machine.
- **Steps performed:** Built the release binary on the final code. Ran `engine-cli calibrate --seed 2026 --matches 1000 --out <scratch>/calib-post-r3` from 2026-09-23T07:20:33Z to 07:23:13Z. Read the stdout line and `report.json`.
- **Evidence:** `verify-evidence/calibration/ac-e.stdout.txt` (one JSON line, 2433 bytes), `ac-e.report.json`, `ac-e.stderr.txt`, `ac-e.exit-code` (0), `ac-e.started-at`, and `ac-e.ended-at`. The pre-fix drive is in `pre-ac-e.*`.
- **Observation:** `calib.wall_ms` was 77,115 ms for the equal suite and 78,331 ms for the strength suite, 155,447 ms in total. That is about 1.3 minutes per 1000-match suite, against 30 minutes. Both `wall_ms` band checks pass. The report holds `bench.match_wall_ms` 426, `bench.cpu_us_per_tick` 1.5012, `bench.ticks_per_match` 278,850, and `machine.cpu_model`. The report also carries the new `change.expired_at_full_time` (105).
- **Result:** pass.

## Acceptance Criteria Status

| # | Criterion | kind | status | method | evidence | evidence-rung |
|---|---|---|---|---|---|---|
| AC-a | "goals per match lie in 2.4 to 3.2, shots per team in 8 to 16, and possession for either side in 35 to 65 percent" over 1000 equal matches | code-only (annotated `observable: false`) | met | automated | `ac-e.report.json` `calib.bands`: goals 2.831, shots 12.675, possession 49.997 / 50.003, all `pass: true`. The slow test passed (`slow-calibrate.stdout.txt`) | n-a |
| AC-b | "team A with attributes 15 percent higher ... wins more than 50 percent" | code-only (annotated) | met | automated | `stronger_team_win_rate` 0.65, `pass: true`, before and after the fix | n-a |
| AC-c | "one statistics record and one event stream file that validate against the agreed schema without transformation" | code-only (annotated) | met | automated | The schema tests and the calibration smoke test validate every file against the amended schemas with `boon` (`tests-default.txt`). The drive wrote 2000 event files, `darkpath.match_without_stats` was 0, and `validate.violations` was 0 | n-a |
| AC-d | "Given 1000 matches, Then the dark-path counters read zero" | code-only (annotated) | met | automated | `darkpath.change_never_applied` 0 and `darkpath.match_without_stats` 0 in `ac-e.report.json`. The slow test asserts both at 0 and passed. It was 105 before the fix (`pre-ac-e.report.json`) | n-a |
| AC-e | "wall time is under 30 minutes, and the report also records the single-thread per-match time" | user-observable (annotated `observable: true`) | met | interactive | `ac-e.report.json`, `ac-e.stdout.txt` | live |

Evidence: live 1 / n-a 4. User-observable criteria at a mock rung: 0.

## Issues Found

- `severity: info` Resolved in this run: DARKPATH-2, where changes queued with no admitting stoppage left were counted as never applied (105 over 2000 matches). Fixed in `3ec2198` by the product owner's definition.
- `severity: info` Resolved in the first run: DARKPATH-1, where a refused injury substitution was requested again (183 of the original 288). Fixed in `aedf0e3`.
- `severity: info` The CVE scan did not run, because `cargo audit`, `cargo deny`, and `osv-scanner` are not installed (recorded in the first run, `calibration-run-2/`). The fix adds no dependency.

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| DARKPATH-2 | unmet-ac (AC-d: changes left waiting at full time counted as never applied) | Fix (autonomous policy, carrying out the product owner's answer) | Patched. `crates/engine/src/tactics/change.rs`: `ChangeQueue.admitted`, `Unapplied`, `ChangeQueue::unapplied`, and `apply_changes` records admitting stoppages. `snapshot.rs` persists `admitted`. `observe/mod.rs` adds `change.expired_at_full_time`. The report, calibrate, and both schemas carry the new key | `crates/engine/tests/tactics_queue.rs` `a_change_queued_after_the_last_admitting_stoppage_expires_at_full_time` (fails before, passes after). `crates/engine/src/tactics/change.rs` `tests::a_change_an_admitting_stoppage_passed_is_never_applied_and_a_later_one_expired` | Pass. The drive exits 0 with `darkpath.change_never_applied` 0. The slow test passes. Every band value and the benchmark ticks per match are unchanged |

Commit: `3ec2198` (`fix(engine): count changes left waiting at full time separately`, 10 files under `crates/` and `schemas/`). The first run's fix `aedf0e3` (DARKPATH-1) is recorded in `history/06-verify-calibration-0.md`.
Regression tests added: 2.

The observability contract amendment (`.ai/observability.md`: key list, `dark-paths` purpose, and `match-stats` record) is in the working tree, and it is not in the fix commit. It is a workflow file, so it goes in with the next workflow-record commit.

## Augmentation Verification

- **Mock fidelity (`02c-craft.md`):** not applicable. The fix changes no page, component, or style.
- **Instrumentation (`04b-instrument.md`):** `match-stats` and the calibrate `run-report` validate against the amended schemas (AC-c). `calibrate.suite` fires. `calibrate.darkpath` fires only on a non-zero dark path: it fired before the fix and was silent after it.
- **Experiment (`04c-experiment.md`):** `deferred-to-experiment-flags`, so there is no wiring to check.
- **Benchmark (`05c-benchmark.md`, compare mode):** processor time per match was 415.6 ms against the 460.7 ms gate. Peak memory was 6.438 MB against 6.82 MB. Ticks per match were 282,600, the same as the first run. No tripwire fired. The fix adds one comparison per stoppage and one pass over the queue at full time.

## Security Scan

- CVE scan: skipped, because no scanner is installed. The fix adds no dependency.
- Secret detection: pass, with 0 findings in the 177 added lines.
- SAST: no tool is available for Rust on this machine. `cargo clippy -D warnings` is clean.

## Accessibility Gate

Not automatable: the slice has no page surface. New WCAG AA violations: 0.

## Performance Gate

Bundle size: skipped, because the web bundle did not change. Calibration wall time was 77 to 79 s per 1000-match suite, against a budget of 30 minutes. The single-thread time per match was 426 ms in the report and 415.6 ms of processor time in the benchmark gate.

## Cross-Slice Regression

The whole workspace suite and the web tests covered the sibling slices: engine-core, data-schemas-generator, stream-protocol, viewer-pitch, match-rules, tactics-and-ai, and commentary. Regressions found: 0. The fix touches the tactics slice's change queue, and its tests (`tactics_queue.rs`, `substitutions.rs`, `injury.rs`) pass. The fix also touches the match-rules slice's snapshot, and the continuation and determinism tests (`snapshot.rs`, `determinism.rs`) pass.

## Longitudinal Delta

- Calibration report: the baseline is this run's pre-fix drive on `aedf0e3` (`pre-ac-e.report.json`). Goals, shots, possession, and the win rate are identical. `darkpath.change_never_applied` went from 105 to 0, and the new `change.expired_at_full_time` is 105. That matches: the same 105 changes are now classified as expired. Wall times are within 1 percent. This change was expected.
- First run to this run: 288, then 105 (after `aedf0e3`), then 0 plus 105 expired (after `3ec2198`).

## Friction Notes

- A developer who reads `report.json` now sees 105 expired changes next to a passing run. The key name and the schema description explain the count.

## Free Exploration Notes

- Outlier-only retention kept 1774 of 2000 event files. A match that only has expired changes no longer counts as a dark-path outlier. Informational.
- The strength suite averages more shots per team than the equal-suite band allows. The bands judge only the equal suite. Informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| empty submission | n-a | command-line harness; argument errors are covered by `cli_args.rs` |
| max-length input | n-a | no free-text input |
| double-click / rapid repeat | n-a | no interactive surface |
| mid-flow interruption | n-a | a snapshot taken mid-match now carries the admitting-stoppage ticks. The snapshot continuation test passes |
| offline / network failure | n-a | no network use |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| slow response | n-a | local command-line harness |
| concurrent session | n-a | each run writes its own run folder |
| session expiry mid-flow | n-a | no session |

## Gaps / Unverified Areas

- `darkpath.change_never_applied` counts only waiting changes that an admitting stoppage passed without a verdict. `apply_changes` gives a verdict to every admitted change, so the counter guards against a regression in that function. It cannot see a stoppage that never reaches `apply_changes`. The same limit applied to the old counter.
- No CVE scan (the tooling is absent).

## Freshness Research

Not required. No test failed, the plan is two days old, and the fix touches no external API or dependency.

## Assumptions

1. `class: implementation-detail`. **The product owner's answer is authoritative input.** `po-answers.md` (2026-09-23T07:00:32Z) and `steer.md` § "Dark-path counter definition" record the answer to the question the first run escalated. This run carries out that answer and does not reopen it.
2. `class: implementation-detail`. **DARKPATH-2 triaged Fix under the autonomous policy.** Once the product owner had defined the counter, the remaining work was the minimal code to match that definition: a patch with a regression test that fails first, and a re-run of only the affected checks.
3. `class: implementation-detail`. **Classify at full time by the latest admitting stoppage for each kind.** A waiting change is never applied when a stoppage that admits its kind opened on a tick after the tick it was queued, and expired otherwise. This follows the product owner's words ("no stoppage that admits it came after it was queued") without inferring from the queue's current behaviour that the counter is always zero.
4. `class: implementation-detail`. **The snapshot keeps the admitting-stoppage ticks inside format version 3.** This slice introduced version 3 (`80f976d`) and has not shipped. A snapshot from another build is already refused on its build hash. Adding the field without a second version bump keeps the plan's rule of one bump per slice.
5. `class: implementation-detail`. **No fix sub-agent was dispatched.** This agent has no sub-agent tool, and the cargo target folder serialises builds. The patch was applied in place by the prescribed method.
6. `class: implementation-detail`. **The fix commit holds only its own 10 files.** `.ai/observability.md` and the staged files from other work stayed out of it.
7. `class: implementation-detail`. **`security-scan-result: skipped`.** Secret detection ran, and no dependency CVE scanner is installed. `skipped` reports that gap honestly.
8. `class: implementation-detail`. **AC-e evidence rung `live`.** A `cli-direct` run of the real release binary on the reference machine, with its report read back, is the live rung for a command-line criterion.
9. `class: implementation-detail`. **No consult.** The triggers `ac-met-by-inference` and `ac-deferred` do not hold, and the product owner excluded `consult` at intake.

## Recommendation

Send the slice to review. All 5 criteria are met on direct evidence from the final code. The dark-path counter now means what the product owner defined, and every realism band and the time budget hold.

## Recommended Next Stage

- **Option A (recommended):** `/wf review football-manager-match-engine calibration`. Verify converged with `result: pass`. Compact first, because this run's test and drive output is noise for review.
- **Option D:** `/wf handoff football-manager-match-engine calibration`. Skip review only if the change has an external reviewer. The slug's other slices are waiting on the same review ledger, so review is the better choice.
- **Option G:** `/wf probe football-manager-match-engine`. Use this for a slug-wide runtime sweep after review, because the fix touched the shared change queue and snapshot.
