---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: calibration
status: complete
stage-number: 6
created-at: "2026-09-23T01:14:00Z"
updated-at: "2026-09-23T01:14:00Z"
result: fail
metric-checks-run: 13
metric-checks-passed: 11
metric-acceptance-met: 4
metric-acceptance-total: 5
metric-acceptance-user-observable: 1
metric-acceptance-code-only: 4
metric-interactive-checks-run: 1
metric-interactive-checks-passed: 1
metric-issues-found: 1
metric-issues-found-initial: 2
metric-issues-found-final: 1
fix-rounds-run: 1
convergence: escalated
verify-owned-fix-commit: "aedf0e3"
regression-tests-added: 1
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/calibration/"
evidence-run-count: 1
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
  - "No visual change: the slice and the fix commit touch no file under web/, so no layout, typography, palette, or token rule of the standing design direction applies."
  - "Output boundary: a vocabulary scan of every added line in crates/, content/, schemas/, and README.md since 846c48f found no workflow term; the fix commit message uses product language only."
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
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine calibration"
---

# Verify: Calibration Harness and Statistics Record

## The Verification

Implement handed over commits `80f976d` and `c00eaac`: the `calibrate` command, the match figures, three record schemas, and a tuning pass. Implement reported that every realism band holds and that one criterion fails: 288 changes were still waiting at full time over 2000 matches. This run repeated the 1000-match drive on seed 2026 on the reference machine (machine hash `74ca12fc08a4`). The drive reproduced every figure: 2.831 goals per match, 12.675 shots per team, 49.997 / 50.003 possession, a 0.65 win rate for the stronger club, 77.3 s and 78.7 s per suite, and 430 ms per match single-threaded. It also reproduced 288 changes that never applied. Four of the five criteria are met on direct evidence.

The fix round split the dark-path failure into two issues. The first is a code bug. When a team had used its limit or its last window, the computer manager asked again for a refused injury substitution every 30 seconds, so the last request was still waiting when the match ended. The fix (`aedf0e3`, with one regression test) makes the manager ask once, on the injury's own stoppage, and ask again only while a substitution can still apply. Match play is unchanged: after the fix every band value is identical, and the count falls from 288 to 105. The second issue is the remaining 105 changes. The computer manager queued them after the last stoppage of the match (75 fatigue substitutions, 29 mentality changes, and 1 fatigue substitution that waited through a penalty). No stoppage came to apply them. Whether such a change counts as a change that never applied is a question about the product owner's contract, and it is already open in the index. This run does not decide it, and it did not edit the counter or the bands.

The slice therefore stays at `result: fail` with `convergence: escalated`. The harness, the records, the schemas, and the tuned engine are ready for integration. The criterion that the dark paths read zero stays blocked until the product owner answers the definition question. Then either the plan redefines the counter, or the computer manager stops queuing changes it cannot get applied. The top open risk after that is the goals distribution: the mean is in band, but the standard deviation is 3.36.

## Verification Summary

- Branch `feat/football-manager-match-engine`, verified at `e7d1232` (implement head). The fix commit is `aedf0e3`.
- Stack confirmed (`stack.user-confirmed: true`), plan `stack-source` agrees. Standard mode.
- Checks: 13 run, 11 pass. The 2 failures are the same criterion (AC-d) seen by the calibration drive and by the slow test.
- Criteria: 4 of 5 met. AC-d is not met (105 after the fix, 288 before it).
- Runtime gate: the one user-observable criterion (AC-e) has live `cli-direct` evidence. There is no deferral.
- The cargo target folder serialises builds, so every check ran in sequence in this agent. No sub-agent was dispatched (see Assumptions).

## Automated Checks Run

1. `cargo fmt --all -- --check`: pass (exit 0; `verify-evidence/calibration/fmt.txt`; re-run after the fix: `recheck-fmt.txt`, exit 0).
2. `cargo clippy --workspace --all-targets -- -D warnings`: pass (exit 0; `clippy.txt`; after the fix: `recheck-clippy.txt`, exit 0).
3. `cargo test --workspace`: pass. Before the fix: 271 passed, 0 failed, 4 ignored (`tests-default.txt`). After the fix: 272 passed, 0 failed, 4 ignored (`recheck-tests-default.txt`). The 4 ignored tests are the slow criteria tests.
4. `node --test web/tests/*.test.mjs`: pass, 47 of 47 (`node-tests.txt`).
5. `cargo build --release -p engine-cli`: pass (`release-build.txt`).
6. `engine-cli calibrate --seed 2026 --matches 1000` (release, 8 workers): **FAILED**, exit 2. Every band and both time budgets pass. `darkpath.change_never_applied` = 288 (`ac-e.stdout.txt`, `ac-e.stderr.txt`, `ac-e.report.json`, `ac-e.exit-code`). After the fix, with `--keep-events all`: exit 2, `darkpath.change_never_applied` = 105, and every band value is unchanged (`recheck-ac-e.*`).
7. `cargo test --release -p engine-cli --test calibrate -- --ignored` (after the fix): **FAILED**, exit 101. Every band assertion passed. The test stopped at `calibrate.rs:171` with left 105, right 0 (`recheck-slow-calibrate.stdout.txt`).
8. `cargo test -p engine --test injury` (regression test for the fix): fails on the pre-fix engine (asked 2 times, expected 1; `regression-test-before-fix.txt`), and passes 3 of 3 after the fix (`regression-test-after-fix.txt`).
9. Benchmark compare, `engine-cli bench --seed 42 --matches 5 --json` three times: pass. Processor time per match 418.8, 418.8, and 412.6 ms (median 418.8 against the 460.7 ms gate). Peak memory 6.449, 6.477, and 6.469 MB (median 6.469 against 6.82 MB). `bench-{1,2,3}.*`.
10. Instrument signals: pass. `calibrate.suite` fired for both suites, and `calibrate.darkpath` fired with the counter and its value (`ac-e.stderr.txt`).
11. Output-boundary scan of the added lines since `846c48f` in `crates/`, `content/`, `schemas/`, and `README.md`: pass, 0 hits.
12. Secret scan (key, secret, password, and token assignments in the added lines): pass, 0 hits.
13. `sdlc-debt` marker scan of the slice diff: pass, 0 markers.

## Interactive Verification Results

- **Criterion:** AC-e, "Given 1000 matches on the reference laptop, Then wall time is under 30 minutes, and the report also records the single-thread per-match time from the benchmark harness."
- **Platform & tool:** Windows 11, the release `engine-cli` binary run directly (`cli-direct`), on machine hash `74ca12fc08a4` (AMD Ryzen 7 9800X3D). This is the machine behind every benchmark baseline in `05c-benchmark.md`.
- **Steps performed:** built release, then ran `engine-cli calibrate --seed 2026 --matches 1000 --out <scratch>/calib-pre` from 2026-09-23T00:59:27Z to 01:02:08Z. Read the stdout line and `report.json`.
- **Evidence:** `verify-evidence/calibration/ac-e.stdout.txt` (one JSON line), `ac-e.report.json`, `ac-e.stderr.txt`, `ac-e.exit-code`, `ac-e.started-at`, and `ac-e.ended-at`. The post-fix repeat is in `recheck-ac-e.*`.
- **Observation:** `calib.wall_ms` equal 77,319 and strength 78,661 (total 155,980 ms, about 1.3 minutes per 1000-match suite against 30). Both `wall_ms` band checks pass. `bench.match_wall_ms` 430, `bench.cpu_us_per_tick` 1.5126, `bench.ticks_per_match` 278,850, and `machine.cpu_model` are all present in the report. The post-fix repeat gave 77,233 and 78,814 ms and 426 ms per match.
- **Result:** pass. The exit code 2 comes from AC-d, not from this criterion.

## Acceptance Criteria Status

| # | Criterion | kind | status | method | evidence | evidence-rung |
|---|---|---|---|---|---|---|
| AC-a | "goals per match lie in 2.4 to 3.2, shots per team in 8 to 16, and possession for either side in 35 to 65 percent" over 1000 equal matches | code-only (annotated `observable: false`) | met | automated | `ac-e.report.json` `calib.bands`: goals 2.831, shots 12.675, possession 49.997 / 50.003, all `pass: true`. Slow test band assertions passed (`recheck-slow-calibrate.stdout.txt`) | n-a |
| AC-b | "team A with attributes 15 percent higher ... wins more than 50 percent" | code-only (annotated) | met | automated | `stronger_team_win_rate` 0.65 (650 W, 205 D, 145 L), `pass: true`, before and after the fix | n-a |
| AC-c | "one statistics record and one event stream file that validate against the agreed schema without transformation" | code-only (annotated) | met | automated | `crates/engine-cli/tests/schemas.rs` and the calibration smoke test (validates every calibration file and `report.json` with `boon`) pass in `tests-default.txt` and `recheck-tests-default.txt`. The drive wrote 2000 statistics records and 2000 event files, and `darkpath.match_without_stats` = 0 | n-a |
| AC-d | "Given 1000 matches, Then the dark-path counters read zero" | code-only (annotated) | **not met** | automated | `darkpath.match_without_stats` = 0. `darkpath.change_never_applied` = 288 before the fix and 105 after it (`recheck-slow-calibrate.stdout.txt`, `recheck-darkpath-breakdown.txt`) | n-a |
| AC-e | "wall time is under 30 minutes, and the report also records the single-thread per-match time" | user-observable (annotated `observable: true`) | met | interactive | `ac-e.report.json`, `ac-e.stdout.txt` | live |

Evidence: live 1 / n-a 4. User-observable criteria at a mock rung: 0.

## Issues Found

- `severity: high` AC-d not met: `darkpath.change_never_applied` = 105 over 2000 matches after the fix round (DARKPATH-2). All 105 are changes the computer manager queued with no admitting stoppage left before full time: 75 fatigue substitutions and 29 mentality changes queued after the last stoppage, plus 1 fatigue substitution queued before a penalty stoppage that did not apply it. The median time from the last stoppage to full time in these 47 matches is 830 s, and the longest is 1809 s. Triage: **Escalate** (intent-bearing: the definition of a product-owner-locked dark-path counter).
- `severity: info` Resolved in this run: DARKPATH-1, the re-request of refused injury substitutions (183 of the original 288), fixed in `aedf0e3`.
- `severity: info` CVE scan not run: `cargo audit`, `cargo deny`, and `osv-scanner` are not installed. The slice added 39 crates to `Cargo.lock`, all through the test-only `boon` 0.6.1. The release binary does not link them.

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| DARKPATH-1 | unmet-ac (code bug: the computer manager re-requests an injury substitution that the queue must refuse) | Fix | Patched (`crates/engine/src/ai.rs`: `ai_check` takes `at_stoppage`, and a new `substitution_possible` gates later requests) | `crates/engine/tests/injury.rs` `a_refused_injury_substitution_is_not_asked_for_again` (fails before, passes after) | Pass: the regression test passes, 0 injury substitutions remain among the late changes (`recheck-darkpath-breakdown.txt`), the count falls from 288 to 105, and every band value is unchanged |
| DARKPATH-2 | unmet-ac (counter definition: changes queued with no admitting stoppage left) | Escalate | N/A | n-a | Not re-run |

Commit: `aedf0e3` (`fix(engine): stop the computer manager re-asking for a refused injury substitution`, which touches only `crates/engine/src/ai.rs` and `crates/engine/tests/injury.rs`).
Regression tests added: 1.

The commit rule allows this commit: every re-check of a `Fix`-triaged issue passed. The escalated issue keeps the slice at `convergence: escalated`.

## Augmentation Verification

- **Mock fidelity (`02c-craft.md`):** not applicable. The slice and the fix change no page, component, or style.
- **Instrumentation (`04b-instrument.md` and plan Assumption 12):** the `match-stats` figures, the calibrate `run-report`, and the `match-event` rows validate against the schemas (AC-c). `calibrate.suite` and `calibrate.darkpath` fire (check 10).
- **Experiment (`04c-experiment.md`):** `deferred-to-experiment-flags`, so there is no wiring to check.
- **Benchmark (`05c-benchmark.md`, compare mode):** 418.8 ms per match against the 460.7 ms gate, and 6.469 MB against 6.82 MB. `bench.cpu_us_per_tick` 1.482 against the 1.4965 µs baseline (−1.0 percent). `bench.ticks_per_match` is 282,600 against 279,850 (+1.0 percent), because the tuning pass changes play. No tripwire fired. The fix does not move a tick: a refused change never touches a player, and every band value is identical before and after it.

## Security Scan

- CVE scan: skipped. `cargo audit`, `cargo deny`, and `osv-scanner` are absent (`error: no such command: audit`, `error: no such command: deny`, `osv-scanner: command not found`). The 39 new crates enter through the test-only `boon` dependency.
- Secret detection: pass, 0 findings in the added lines.
- SAST: none available for Rust on this machine. `cargo clippy -D warnings` is clean.

## Accessibility Gate

Not automatable: the slice has no page surface. New WCAG AA violations: 0.

## Performance Gate

Bundle size: skipped (no web bundle change). Calibration wall time: 77 to 79 s per 1000-match suite against a budget of 30 minutes. Single-thread per match: 426 to 430 ms in the report, and 418.8 ms processor time in the benchmark gate.

## Cross-Slice Regression

Siblings checked through the whole workspace suite and the web tests: engine-core, data-schemas-generator, stream-protocol, viewer-pitch, match-rules, tactics-and-ai, and commentary. Regressions found: 0. The fix touches the tactics slice's computer manager. Both of its injury tests (`an_injured_player_leaves_play_and_the_ai_replaces_him_at_that_stoppage` and `with_the_limit_used_the_team_plays_on_with_ten`) pass unchanged.

## Longitudinal Delta

- Calibration report: the baseline is the implement run on `c00eaac` (`implement-evidence/calibration/ac-e.report.json`). Goals, shots, possession, the win rate, and the dark-path count (288) are identical, and the wall times are within 1 percent. Expected.
- After the fix: the dark-path count is 105 and every other figure is identical. Expected: refused changes never moved play.

## Friction Notes

- A run that fails one criterion prints the same single JSON line as a passing run and exits 2. A developer must read `calib.pass` or the stderr warning to learn why. The `calibrate.darkpath` warning names the counter, which is enough.

## Free Exploration Notes

- 885 of 1000 equal-suite matches are outliers, so outlier-only retention kept 1799 of 2000 event files. Informational; implement recorded this already.
- The strength suite averages 22.4 shots per team, above the equal-suite band. The bands judge only the equal suite. Informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| empty submission | n-a | command-line harness; argument errors are covered by `cli_args.rs` |
| max-length input | n-a | no free-text input |
| double-click / rapid repeat | n-a | no interactive surface |
| mid-flow interruption | n-a | a crashed worker surfaces as `darkpath.match_without_stats`, which the smoke test's record count covers |
| offline / network failure | n-a | no network use |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| slow response | n-a | local command-line harness |
| concurrent session | n-a | each run writes its own run folder |
| session expiry mid-flow | n-a | no session |

## Gaps / Unverified Areas

- AC-d stays unmet (DARKPATH-2) pending the product owner's answer on the counter definition.
- The event-file breakdown relies on a heuristic: the last restart event marks the last stoppage (`darkpath-breakdown.py`). It explains 104 of 105 changes directly. The one match it does not cover holds a fatigue substitution that waited through a penalty stoppage.
- No CVE scan (tooling absent).

## Freshness Research

Not required. No test failed for a dependency reason, the plan is two days old, and the fix touches no external API. The only new dependency (`boon` 0.6.1) was read from its installed source at implement time.

## Assumptions

1. `class: implementation-detail`. **No fix sub-agent was dispatched.** This agent has no sub-agent tool. The cargo target folder also serialises builds. The one `Fix` patch was applied in place, and the prescribed method was followed: a minimal patch, a regression test written and proven failing first, and only the affected checks re-run.
2. `class: implementation-detail`. **AC-d was split into two issues at triage.** The implement record had already separated two causes (183 re-requested injury substitutions and 105 late changes). The first is a code defect with a code fix that leaves the counter definition untouched. The second cannot be closed without deciding what the counter means.
3. `class: implementation-detail`. **DARKPATH-1 triaged Fix (autonomous policy).** The slice criterion "the AI manager queues a substitution that applies within the next window when one is available" (tactics slice) is silent on the unavailable case. The tactics test that pins the first refusal and its reason (`with_the_limit_used_the_team_plays_on_with_ten`) is kept: the manager still asks once on the injury's own stoppage.
4. `class: intent-bearing`. **DARKPATH-2 triaged Escalate, not decided.** Counting a change queued with no stoppage left as "applied", "expired", or "never applied", or stopping the computer manager from queuing late, changes either a product-owner-locked contract (`.ai/observability.md` dark-paths, "must read zero") or the tactics slice's designed behaviour. The question is already in `00-index.md` `open-questions`, and its figure is updated there to 105.
5. `class: implementation-detail`. **Committing the fix while the slice stays escalated.** The commit rule gates on the `Fix`-triaged re-checks, and all of them passed. The commit path list was limited to the two fix files, so staged files from other work (`docs/design/realism/*`, `crates/engine/tests/zz_stall_probe.rs`) stayed out of it.
6. `class: implementation-detail`. **`security-scan-result: skipped`.** Secret detection ran. No dependency CVE scanner is installed, and 39 new test-only crates entered this slice. `skipped` reports that gap more honestly than `pass`.
7. `class: implementation-detail`. **AC-e evidence rung `live`.** A `cli-direct` run of the real release binary on the reference machine, with its report read back, is the live rung for a command-line criterion.
8. `class: implementation-detail`. **No consult.** The triggers `ac-met-by-inference` and `ac-deferred` do not hold, and the product owner excluded `consult` at intake.

## Recommendation

Do not review this slice as passing. The harness, records, schemas, and tuning are sound, and 4 of 5 criteria are met on direct evidence. AC-d needs a product-owner decision before any code change can close it.

## Recommended Next Stage

- **Option E (recommended, after the product owner answers the AC-d question in `00-index.md`):** `/wf plan football-manager-match-engine calibration`. Either redefine `darkpath.change_never_applied` (for example, count only changes that an admitting stoppage refused to apply or skipped), or scope a computer-manager change that stops late queuing. Both change a locked contract or a designed behaviour, so they belong in a plan.
- **Option C:** `/wf implement football-manager-match-engine calibration`. Use this if the product owner answers in a way that needs no plan change, for example by keeping the counter and asking the computer manager to stop queuing changes in the closing minutes.
- **Option B:** `/wf verify football-manager-match-engine calibration`. A second round is useful only after the answer. Without it, the round would escalate the same issue again.
