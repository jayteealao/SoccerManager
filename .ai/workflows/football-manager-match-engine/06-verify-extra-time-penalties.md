---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: extra-time-penalties
status: complete
stage-number: 6
created-at: "2026-09-23T16:50:36Z"
updated-at: "2026-09-23T16:50:36Z"
result: pass
metric-checks-run: 12
metric-checks-passed: 12
metric-acceptance-met: 3
metric-acceptance-total: 3
metric-acceptance-user-observable: 0
metric-acceptance-code-only: 3
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
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/extra-time-penalties/"
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
adversarial-tests-run: 3
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
  - "Product-owner answer extra-time-penalties Q-1 = B: a live seed-6 knockout run played 12 shoot-out kicks on the pitch (tick records advance through every kick; test level_after_extra_time_a_shootout_on_the_pitch_produces_one_winner asserts the ball and players move on each kick)."
  - "Dark-path counter definition: the live knockout run reports change.expired_at_full_time 0 and darkpath.change_never_applied 0, and a_change_queued_during_the_shootout_is_not_applied passes; the computer manager's queuing is unchanged."
  - "Image gate, tokens, layout, no spinner: the only page change is the scrubber limit (web/history.mjs, web/main.mjs); no style, token, or layout file changed."
  - "Output boundary: a vocabulary scan of every added line in the build commit and of its message found no workflow term (the only hits were Rust copy_from_slice calls)."
tags: [engine, rules, extra-time, shoot-out, fatigue, knockout]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-extra-time-penalties.md
  plan: 04-plan-extra-time-penalties.md
  implement: 05-implement-extra-time-penalties.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-extra-time-penalties.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine extra-time-penalties"
---

# Verify: Extra Time and Penalty Shoot-outs

## The Verification

Implement handed over commit `90d4ee1` and its record commit `ee2ce12`: a knockout switch, a period clock that runs to minute 120, one extra substitution in one extra window, extra-time breaks that give no energy back, and a shoot-out played kick by kick on the pitch. The slice has three criteria. Each carries an explicit `observable: false` annotation, so the runtime gate does not apply and no deferral is needed. Cargo serialises on its target folder, so this run did every check in sequence, with no sub-agents.

Every check passed on the first run, so the fix loop had nothing to do. The workspace suite passed 341 tests with 0 failed and 4 ignored (47 result lines). The 6 extra-time tests, the 8 shoot-out tests, the 4 fatigue tests, and the 6 snapshot tests passed by name. A live `simulate --seed 6 --knockout` run went to extra time at 0-0, used six substitutions per side, and ended 3-4 on a 12-kick shoot-out with 0 validator violations. Its statistics record is valid against the match-stats schema. The default match does not change: this run rebuilt the parent commit `f6f36b6` from a clean archive, and the seed-42 JSON Lines dumps hash to the same value (`8120abc5…`). The tick records after the 64-byte header also hash the same (`b126cd59…`). The benchmark median is 418.8 ms per match against the 460.7 ms gate, and 6.55 MB against 6.82 MB. This run also re-read IFAB Laws 3, 7, and 10 at the source (plan risk R7), and the engine matches them.

Review is next. The top open risk is the realism of the shoot-out. Its hand-set keeper and aim constants have no calibration band yet, and this run checked conversion on one seed only.

## Verification Summary

- Slice: `extra-time-penalties`, standard mode, branch `feat/football-manager-match-engine` (dedicated; confirmed with `git branch --show-current`).
- Stack: `stack.user-confirmed: true`; plan `stack-source: confirmed`.
- The plan's `## Verification Strategy` names no environment dependency, so the constraint-resolution gate does not apply (`constraint-resolution-missing: []`).
- No prior `06-verify-extra-time-penalties.md` or evidence directory existed. This is evidence run 1, and no archive move was needed.
- Code under test equals `90d4ee1`: `git status` shows no modified path under `crates/`, `web/`, `content/`, or `schemas/`. The staged `docs/design/realism/*` files and the staged-then-deleted `crates/engine/tests/zz_stall_probe.rs` belong to other sessions. The deleted test file is not in the working tree, so it did not compile into any check.
- Issues found: 0 initial, 0 final. Fix loop: not run (`convergence: not-needed`).

## Automated Checks Run

1. `cargo fmt --all -- --check`: pass (exit 0; `fmt.txt`).
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass (exit 0, no warning; `clippy.txt`).
3. `cargo test --workspace --all-features`: pass, 341 passed, 0 failed, 4 ignored over 47 result lines (`test-workspace.txt`).
4. `cargo test -p engine --all-features --test rules_extra_time --test rules_shootout --test fatigue --test snapshot --test content --test determinism --test substitutions --test commentary`: pass. rules_extra_time 6/6, rules_shootout 8/8, fatigue 4/4, snapshot 6/6, content 7/7, determinism 1/1, substitutions 5/5, commentary 10/10 (`test-ac.txt`).
5. `node --test web/tests/*.test.mjs`: pass, 127 tests, 0 fail, 0 skipped (`web-tests.txt`).
6. `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` ×3: pass. 418.8 / 418.8 / 422.0 ms per match (median 418.8, gate 460.7). Peak memory 6.484 / 6.551 / 6.551 MB (median 6.551, gate 6.82). 1.482 / 1.482 / 1.4933 µs per tick, 282,600 ticks per match, `budget.pass: true` on all three (`bench-*.json`, `bench-summary.txt`).
7. Live knockout run `engine-cli simulate --seed 6 --knockout --json --no-snapshot`: pass, exit 0. `extra_time.played: true`, `extra_time.added_s: [60,60]`, `goals: [0,0]`, `result.decided_by: "shoot-out"`, `shootout.scores: [3,4]`, `shootout.kicks: 12`, `substitutions: [6,6]`, `validate.violations: 0`, `ticks.played: 384570` (`ko-seed6.stdout.txt`). Signals on stderr: `rules.extra_time` ×2, `rules.shootout_start` ×1, `rules.shootout_kick` ×12, `rules.shootout_result` ×1 (`ko-seed6.stderr.txt`).
8. Schema check: the knockout match-stats record validates against `schemas/observability/match-stats.schema.json` with jsonschema 4.26.0 (`schema-check.txt`).
9. Default-match identity: pass. The parent commit `f6f36b6` was built from a `git archive` copy. Seed-42 JSONL SHA-256 is `8120abc5…` for both builds. The tick bytes after the 64-byte header hash to `b126cd59…` for both. `cmp` finds 2 differing bytes, at offsets 45 and 46, inside the header's wall-clock match stamp (`seed42-identity.txt`).
10. IFAB Laws 3, 7, and 10 re-read from theifab.com: the engine matches them (`ifab-laws-reread.txt`).
11. Security: the secret-pattern scan over the build commit's added lines found 0 hits. `Cargo.lock` and every `Cargo.toml` are unchanged (0 diff lines), so the change adds no dependency. `cargo audit` is not installed.
12. Output-boundary scan over the build commit's added lines and its message: 0 workflow terms (3 `copy_from_slice` false positives only).

## Interactive Verification Results

Automated only. All three criteria carry `observable: false` in `03-slice-extra-time-penalties.md`, and the slice's Scope Out excludes shoot-out display in the viewer. The live CLI run in check 7 is extra runtime evidence, not a gate requirement.

## Acceptance Criteria Status

| # | Criterion | kind | status | method | evidence | evidence-rung |
|---|-----------|------|--------|--------|----------|---------------|
| AC-1 | "Given a knockout fixture level at full time, Then two extra-time periods run, and if still level a shoot-out runs and produces a winner." | code-only (explicit `observable: false`) | met | automated | `a_level_knockout_match_plays_two_periods_of_extra_time`: breaks with period 2 and 3 at minutes 90 and 105, and `decided_by: Shootout` at minute 120. `level_after_extra_time_a_shootout_on_the_pitch_produces_one_winner`: shoot-out scores unequal, kicks alternate, and ball and players move on every kick. `level_after_five_kicks_each_sudden_death_decides_it` and `a_team_that_cannot_catch_up_loses_early` also pass. The live seed-6 run ended 0-0, then 3-4 on kicks (`test-ac.txt`, `ko-seed6.stdout.txt`). | n-a |
| AC-2 | "Given extra time, Then the extra-time substitution allowance from the rule pack is available and enforced." | code-only (explicit `observable: false`) | met | automated | `extra_time_adds_one_substitution_in_one_window`: with 5 substitutions in 3 windows used, a sixth applies at the break before extra time and uses no window. A seventh is rejected with `LimitReached { limit: 6 }`. The live seed-6 run shows 6 substitutions per side (`test-ac.txt`, `ko-seed6.stdout.txt`). | n-a |
| AC-3 | "Given a player at minute 105, Then fatigue follows the extended curve with no discontinuity at minute 90." | code-only (explicit `observable: false`) | met | automated | `fatigue_runs_on_through_extra_time_without_a_step_at_ninety_minutes`: energy never rises from minute 89 to minute 106. The step at minute 90 is no larger than the largest step elsewhere. At minute 105 the derived speed and decisions equal base × `multiplier(energy)` (`test-ac.txt`). | n-a |

Rollup: evidence: n-a 3 (code-only). User-observable 0; mock-rung 0.

## Issues Found

None.

## Augmentation Verification

- **Instrumentation (`04b-instrument.md`, match-rules):** the shared plan catalogs the `rules.` signal family and does not list signal names one by one. The new rule signals fire on a live knockout run: `rules.extra_time` (2), `rules.shootout_start` (1), `rules.shootout_kick` (12), `rules.shootout_result` (1). `rules.shootout_round_limit` did not fire. By design it fires only when a shoot-out passes 100 rounds, which is a defect guard (plan A-20). The additive match-stats properties validate against the schema (check 8). The dark-path counters read 0 (`darkpath.change_never_applied`, `change.expired_at_full_time`).
- **Benchmark (`05c-benchmark.md`, compare against the tactics-and-ai baseline):** processor time per match has a median of 418.8 ms against a baseline of 418.8 ms (0.0 percent; gate 460.7 ms): pass. Peak memory has a median of 6.551 MB against 5.45 MB (+20.2 percent; gate 6.82 MB): pass. The memory figure matches the level that later slices recorded before this one (implement recorded 6.54 to 6.58 MB before and after its change). Processor time per tick is 1.482 µs against 1.4965 µs (−1.0 percent): reported. No tripwire fired.
- **Experiment (`04c-experiment.md`):** deferred to the experiment-flags slice. There is nothing to wire here.
- **Craft (`02c-craft.md`):** this slice changes no visual surface. The scrubber-limit change adds no element, style, or token, so no mock-fidelity item applies.

## Security Scan

- CVE: `cargo audit` is not installed (`error: no such command: audit`). The build commit changes neither `Cargo.lock` nor any `Cargo.toml`, so no new dependency or version entered. New critical/high: 0.
- Secrets: a pattern scan over the added lines (keys, tokens, passwords, private-key headers, AWS key ids) found 0 hits.
- SAST: clippy `-D warnings` is clean, and the change adds no `unsafe` block. New HIGH+: 0.

## Accessibility Gate

Not automatable for this slice. No UI element, style, or text surface changed. The only page edit moves the scrubber's maximum to the newest tick (`web/history.mjs` `scrubLimit`), which `web/tests/history.test.mjs` covers. New WCAG AA violations: 0.

## Performance Gate

- Bundle size delta: skipped. The page ships unbundled modules, and the change is one getter and one assignment.
- Per-match processor time and memory: within the gates (see Augmentation Verification).
- The default match plays 282,600 ticks, which equals the implement record's figure. A knockout match plays more ticks only when `--knockout` is set (384,570 on seed 6).

## Cross-Slice Regression

The workspace suite covers every completed sibling slice's tests (engine-core, data-schemas-generator, stream-protocol, match-rules, tactics-and-ai, commentary, calibration, integration drivers). All 341 pass. The page suite covers viewer-pitch, viewer-match-day, viewer-lineup-tactics, and viewer-reports-recovery, and all 127 pass. Regressions found: 0.

## Longitudinal Delta

- Default match: compared against the parent build `f6f36b6`. The tick records and JSONL are byte-identical apart from the header wall-clock stamp. This is the expected result (plan R3).
- Benchmark: compared against `05c-benchmark.md`. The delta is within noise.

## Friction Notes

- During the shoot-out, every event carries the last minute of play (120). A consumer that sorts by minute alone sees every kick at 120. This is recorded in implement's Known Risks and is informational.

## Free Exploration Notes

- Seed 6 knockout: the away side had 14 shots with 13 on target and 0 goals over 120 minutes. This is informational and not in this slice's scope. Scoring realism belongs to calibration.
- The knockout run's shoot-out went to a sixth round (12 kicks, 3-4). This exercised sudden death live. Informational.

## Adversarial Tests

| Test | Result | Finding |
|------|--------|---------|
| A level knockout match that was shortened (5 minutes) goes straight to the shoot-out | pass | `a_shortened_level_knockout_match_goes_straight_to_the_shootout` |
| A change queued during the shoot-out is not applied | pass | `a_change_queued_during_the_shootout_is_not_applied` |
| A seventh substitution in extra time is rejected at the raised limit | pass | `extra_time_adds_one_substitution_in_one_window` |
| Empty submission / max-length input / double-click / offline | n-a | engine-only slice, no input surface |

## Failure Mode Probes

| Probe | Result | Finding |
|-------|--------|---------|
| Slow response / concurrent session / session expiry | n-a | engine-only slice, no session or network surface |

## Gaps / Unverified Areas

- Shoot-out conversion realism was checked on one live seed (3 of 6 and 4 of 6 kicks scored). The implement record's 80-shoot-out sample (67.7 percent) was not re-run here. No calibration band covers it yet (a deferred item in the build record).
- Law 10 allows a goalkeeper who cannot continue to be replaced during the shoot-out. The engine has no such path, and the path cannot be reached because no injury roll runs during the shoot-out. This is recorded as deferred in the build record.
- `cargo audit` is not installed. The change adds no dependency, so the gap does not bear on this slice.

## Freshness Research

- IFAB Laws of the Game, fetched from theifab.com this run: "Determining the outcome of a match" and Law 3 "The players". These close plan risk R7. The engine's periods (2 × 15 minutes), extra substitution and window, exempt breaks, five kicks with alternation and sudden death, eligibility, equalising, and kick completion all agree with the source. Summary with per-rule mapping: `verify-evidence/extra-time-penalties/ifab-laws-reread.txt`.
- No dependency was added or upgraded, so no library behaviour needed a source read.

## Assumptions

Each entry is an autonomous decision of this run, with its class per `_decision-classes.md`.

- **V-1** AC-1, AC-2, and AC-3 are code-only, from the explicit `observable: false` annotations (partition Step A, final). Each is classified `build-capability` and verified by `cargo test`. `class: implementation-detail`.
- **V-2** No sub-agents were dispatched. Cargo serialises on one target folder, and every check ran in sequence in this session. `class: implementation-detail`.
- **V-3** No prior verify artifact existed for this slice. The driver said a prior round had applied one fix pass, but no evidence of one exists on disk, so this run is recorded as evidence run 1 with `fix-rounds-run: 0`. `class: implementation-detail`.
- **V-4** `security-scan-result: pass` rests on the secret scan, clippy, and a zero dependency diff. `cargo audit` is absent and was not installed. `class: implementation-detail`.
- **V-5** The seed-42 identity check rebuilt the parent commit from a `git archive` copy in the session scratchpad, with a separate target folder. Nothing was written to the repository. `class: implementation-detail`.
- **V-6** No consult ran. No trigger holds (every criterion is met by direct test evidence, and none is deferred), and the product owner excluded consult at intake. `class: implementation-detail`.

## Triage Decisions

None. There were 0 issues, so no Fix, Skip, or Escalate decision was needed.

## Recommendation

Ready for review. Every criterion is met by named, passing tests and a live knockout run, and the default match is byte-identical.

## Recommended Next Stage

- **Option A (default): Review** → `/wf review football-manager-match-engine extra-time-penalties`. Convergence is `not-needed` and the result is `pass`.
- **Option D: Skip review** → `/wf handoff football-manager-match-engine extra-time-penalties`. This option is valid because `result: pass`. It is not recommended: the slice changes engine behaviour on 43 files.
- **Option G: Slug-wide runtime probe** → `/wf probe football-manager-match-engine`. Use this for a cross-slice runtime sweep that includes knockout matches served to the page.
