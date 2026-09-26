---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: defending-and-discipline
status: complete
stage-number: 6
created-at: "2026-09-24T07:01:29Z"
updated-at: "2026-09-24T07:01:29Z"
result: pass
metric-checks-run: 10
metric-checks-passed: 10
metric-acceptance-met: 3
metric-acceptance-total: 3
metric-acceptance-user-observable: 3
metric-acceptance-code-only: 0
metric-acceptance-mock-rung: 0
metric-interactive-checks-run: 3
metric-interactive-checks-passed: 3
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
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/defending-and-discipline/"
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
moved-criteria:
  - {criterion: "No advantage from a red card", to: lone-forward, by: "po-answers.md Q-I2", measured-this-run: "keeper full 2.06 reduced 5.82; centre-back full 0.71 reduced 2.39; striker full 0.95 reduced 3.84"}
  - {criterion: "Every formation holds: 4-4-1-1 and 3-4-3 against 4-4-2", to: lone-forward, by: "po-answers.md Q-I2", measured-this-run: "4-4-1-1 5.18-1.00; 3-4-3 4.17-1.26 (limit 4.0)"}
prior-deferrals-rechallenged:
  - {slice: viewer-match-day, touches-this-slice: false, probe: "grep -ciE \"legibility|read the match screen\" po-answers.md -> 1 (line 662, Q-E5: the product owner leaves the reading open; not a recorded reading)", wall: stands}
  - {slice: distribution, touches-this-slice: false, probe: "test -f verify-evidence/distribution/macos/results.json -> absent", wall: stands}
steering-honored:
  - "Moved criteria and slice order (2026-09-24): the slice is judged on discipline, the acting keeper and the eight formation pairings that hold. The red-card criterion and the 4-4-1-1 and 3-4-3 pairings were measured and recorded, not judged; their limits and tests are unchanged."
  - "A slice gate runs the full suites (Q-I1): the criterion tests ran at their full size (200 matches for discipline, 120 seeds for each formation pairing), not a targeted subset."
  - "Dark-path counter definition: the seed-42 equal suite reads darkpath.change_never_applied 0, change.expired_at_full_time 51, validate.violations 0."
  - "Output boundary: this stage made no commit; evidence stays under the workflow folder."
tags: [engine, tactics, rules, realism, defending, discipline, keeper]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-defending-and-discipline.md
  plan: 04-plan-defending-and-discipline.md
  implement: 05-implement-defending-and-discipline.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-defending-and-discipline.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine defending-and-discipline"
---

# Verify: Defending and Discipline

## The Verification

The implement record left commit `f7fe35b` with goal-side cover, intercepting pressers, a capped back line, an acting keeper, a foul cooldown, one held card per player, and a booked-player factor. It claimed three kept criteria pass. The product owner moved two criteria to `lone-forward` (Q-I2). This stage re-ran every claim from a fresh build on HEAD `2b62cd3`. The source tree under `crates` and `content` is clean.

All three kept criteria are met by headless runs of the real engine. Discipline over 200 matches: 0.085 second yellows per match (limit 0.10), 16.5% of matches with a sending-off (limit 25%), and 0 same-tick card pairs. The acting keeper: 6 of 6 scripted scenes pass. They cover the bench keeper at a stoppage, no substitution left, the catch chance, the penalty, the shoot-out, and a goal kick after half-time. Formations over 120 seeds for each pairing: the eight kept pairings are all at or under 4.0 goals per side, with the highest at 3.15. The workspace suite passes, 458 of 458. fmt and clippy are clean. The processor-time gate passes at 1.5422 µs per tick, against a limit of 1.6228 (+4.5% on the baseline). Every figure matches the implement record exactly. No issue was found, so no fix round ran.

The slice can go to review. The two moved criteria still fail as measured (4-4-1-1 5.18, 3-4-3 4.17, and the reduced side outscores the full side in all three red-card arms). `lone-forward` owns them. The `defending` test file exits 101 until that slice lands. The top open risk is realism: the equal suite at seed 42 scores 4.092 goals per match against a band of 2.4 to 3.2. The sending-off share is 32.6% against a band of 8% to 22%. `realism-tuning` owns those bands (Q-E4).

## Verification Summary

- Kept criteria: 3 of 3 met, each at the headless rung on the real engine library.
- Moved criteria: 2, measured and recorded, not judged (steer.md, Q-I2).
- Checks: 10 run, 10 passed. Issues: 0 initial, 0 final. Fix rounds: 0.
- Earlier open deferrals: both re-probed this run. Neither touches this slice, and both walls stand.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (exit 0; `fmt.txt`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass (exit 0; `clippy.txt`).
- `cargo test --workspace --all-features --no-fail-fast`: pass. 458 passed, 0 failed, 7 ignored (exit 0; `workspace-tests.txt`).
- `cargo test --release -p engine --all-features --test acting_keeper -- --include-ignored`: pass, 6 of 6 (`criteria.txt`).
- `cargo test --release -p engine --all-features --test discipline -- --include-ignored`: pass, 4 of 4, including `discipline_is_realistic` (`criteria.txt`).
- `cargo test --release -p engine --all-features --test defending -- --include-ignored`: 5 scenes pass. `every_formation_holds` prints all ten pairings, and the eight kept pairings are within the limit. The command exits 101 only on the moved criteria: `a_sending_off_gives_no_advantage`, and `every_formation_holds` on 4-4-1-1 and 3-4-3. Recorded as pass on this slice's scope (`criteria.txt`). See Assumption A1.
- Sibling slow tests, release (`a_trailing_ai_team_changes_its_tactics`, `an_attacking_team_shoots_more_than_a_defensive_one`, `a_stronger_team_wins_more_than_half_its_matches`): pass, 3 of 3 (exit 0; `sibling-slow.txt`).
- Benchmark compare, `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` × 3: pass. Median 1.5422 µs per tick (limit 1.6228), median peak memory 6.75 MB (limit 8.27) (`bench-1..3.json`).
- `engine-cli calibrate --seed 42 --suite equal`: signals pass. `darkpath.change_never_applied` 0, `darkpath.match_without_stats` 0, `validate.violations` 0, `change.expired_at_full_time` 51. The run exits 2 on 10 band misses, which are recorded, not failed (Q-E4) (`calibrate-equal-s42.*`).
- `gitleaks git` over `2089ed7..f7fe35b`: pass, no leaks found (`gitleaks.txt`).

## Interactive Verification Results

The engine has no screen for these criteria. The plan names the rung: headless runs of the real engine library over many seeded matches. Adapter: `cli` (cargo test in release on the reference machine).

- **Criterion:** Every formation holds (the eight kept pairings). **Platform & tool:** Windows 11, `cargo test --release`, `every_formation_holds`, seeds 1–120 for each pairing, with the formation at home on odd seeds. **Steps performed:** ran the full test and read the printed pairings. **Evidence:** `verify-evidence/defending-and-discipline/criteria.txt`. **Observation:** 4-4-2 1.43–1.59, 4-3-3 3.15–0.68, 4-2-3-1 3.09–2.64, 3-5-2 1.33–2.76, 4-1-4-1 2.07–0.87, 4-1-2-1-2 1.10–1.19, 5-3-2 0.72–1.41, 5-4-1 3.12–1.63. **Result:** pass. No side averages more than 4.0.
- **Criterion:** Discipline is realistic. **Platform & tool:** same, `discipline_is_realistic`, 200 matches with the card events read per tick. **Evidence:** `criteria.txt`. **Observation:** second yellows per match 0.085, matches with a sending-off 16.5%, same-tick pairs 0. **Result:** pass.
- **Criterion:** The acting keeper covers a red card. **Platform & tool:** same, `tests/acting_keeper.rs` (six scripted scenes on the real engine). **Evidence:** `criteria.txt`. **Observation:** the bench keeper replaces the outfield stand-in through one `sub-keeper` decision and a substitution. With no substitution left, an outfield player keeps goal. The catch chance, the penalty, the shoot-out kick, and a goal kick after half-time all use the acting keeper. **Result:** pass.

## Acceptance Criteria Status

| Criterion | Kind | Status | Method | Evidence | Evidence rung |
|---|---|---|---|---|---|
| Every formation holds (eight kept pairings; 4-4-1-1 and 3-4-3 moved) | user-observable | met | interactive (headless engine runs) | `criteria.txt` pairing lines | headless |
| Discipline is realistic | user-observable | met | interactive (headless engine runs) | `criteria.txt` `discipline_is_realistic` | headless |
| The acting keeper covers a red card | user-observable | met | interactive (scripted scenes on the real engine) | `criteria.txt` `acting_keeper` 6/6 | headless |
| No advantage from a red card | moved to `lone-forward` (Q-I2) | not judged | measured | `criteria.txt` | n-a |

Evidence: headless 3.

## Issues Found

None for this slice's criteria. Two moved criteria fail as measured and belong to `lone-forward` (see Gaps).

## Augmentation Verification

- `benchmark` (baseline on `2089ed7`), compare mode, three drives:

  | Target | Baseline | This run (median) | Delta | Tripwire | Result |
  |---|---|---|---|---|---|
  | processor time per tick (gate) | 1.4753 µs | 1.5422 µs (drives 1.5312, 1.5422, 1.5422) | +4.5% | 1.6228 µs | pass |
  | peak memory (gate) | 6.617 MB | 6.75 MB (drives 6.750, 6.754, 6.621) | +2.0% | 8.27 MB | pass |
  | processor time per match | 422.0 ms | 450.0 ms | +6.6% | reported | ticks per match 291,800 against 286,050 (+2.0%); under the 2000 ms budget |
  | full match wall time | 424 ms | 449 ms | +5.9% | reported | — |
- `instrument` (match-rules signal set): the seed-42 equal suite reports the `match-stats` dark-path counters at 0 and `validate.violations` at 0. The new `sub-keeper` value goes through the shared `ai.decision` path (`crates/engine/src/ai.rs:59`, `:411-417`). `a_keeper_sent_off_is_replaced_by_the_bench_keeper_at_the_next_stoppage` asserts exactly one `sub-keeper` decision event. No signal is missing.
- `experiment` (experiment-flags): not touched by this slice. No re-check was needed beyond the workspace suite, which passes.

## Security Scan

- CVE scan: no dependency changed in `f7fe35b` (`Cargo.toml`, `Cargo.lock` and the crate manifests are untouched). `cargo audit` and `cargo deny` are not installed.
- Secret detection: `gitleaks` over `2089ed7..f7fe35b`, no leaks found.
- SAST: none installed. A pattern scan of the slice diff for credentials found nothing.

## Accessibility Gate

Not applicable. The slice changes no page (`git diff --stat 2089ed7 f7fe35b -- web packaging` is empty). The new decision is an `ai-decision` event, which the page hides.

## Performance Gate

Bundle size: skipped, because no page changed. Engine processor time per tick is +4.5%, under the +10% tripwire. Build time is not measured.

## Cross-Slice Regression

Siblings checked: every workspace test (458), the tactics-and-ai and calibration slow tests (strength, mentality, trailing computer manager), and the experiment-flags and scripting suites inside the workspace run. Regressions found: 0. The 57,000-match `a_thousand_matches_hold_the_realism_bands` test was not run. Its bands are expected to miss until `realism-tuning`, and the equal suite at seed 42 stands in for it.

## Longitudinal Delta

- Criterion figures: compared against the implement record's two runs (first pass and the re-run on `cd9669c`). Identical to every printed decimal.
- Equal suite at seed 42: identical to the implement record (goals 4.092, sending-off share 0.326, yellow cards per team 1.821, throw-ins 17.976). Against the realism-bands-v2 baseline, goals rose from 3.035 to 4.092. This is an expected change and recorded, not flagged (Q-E4).

## Friction Notes

- The criterion test name promises "at the next stoppage", but the scene asserts only that a substitution happened within 60 minutes. That the change applies at the first admitting stoppage comes from the change queue, which tactics-and-ai verified. Informational.

## Free Exploration Notes

- The `defending` test file exits 101 until `lone-forward` lands, because its two slow tests still assert the moved limits. A gate that runs `--include-ignored` over the whole engine will see red until then. Informational; the implement record kept this on purpose (R2).

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| empty submission | n-a | no input surface |
| max-length input | n-a | no input surface |
| double-click / rapid repeat | n-a | repeat fouls on consecutive ticks are covered by `a_tackler_who_fouled_with_advantage_makes_no_attempt_on_the_next_tick` (pass) |
| mid-flow interruption | n-a | resume through a foul cooldown is covered by the snapshot test in the workspace suite (pass) |
| offline / network failure | n-a | engine-only change |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| slow response | n-a | engine-only change |
| concurrent session | n-a | engine-only change |
| session expiry mid-flow | n-a | engine-only change |

## Cross-Browser Delta

Not applicable: no page changed.

## Web Vitals

Not applicable: no page changed.

## Gaps / Unverified Areas

- Moved to `lone-forward` (Q-I2), measured this run, not judged: "No advantage from a red card". Keeper arm: full side 2.06, reduced side 5.82. Centre-back arm: 0.71 and 2.39. Striker arm: 0.95 and 3.84. Also "4-4-1-1 and 3-4-3 against 4-4-2": 5.18 and 4.17, limit 4.0.
- The realism bands are out of band at seed 42. `realism-tuning` owns them.
- Earlier deferrals (viewer legibility, macOS build) were re-probed. They do not touch this slice, and both stay open (`prior-deferral-probes.txt`).

## Freshness Research

No dependency was added or changed. The criteria name no external API or schema. The plan is from 2026-09-23, under 14 days old. AC staleness was checked: 0 stale.

## Recommendation

Proceed to review. The kept criteria are met on direct evidence and reproduce the implement figures exactly.

## Recommended Next Stage

- **Option A (default): Review** → `/wf review football-manager-match-engine defending-and-discipline`. `convergence: not-needed`, `result: pass`. Compact first; the verify output is noise for review.
- **Option D: Skip review** → `/wf handoff football-manager-match-engine defending-and-discipline`. Valid with `result: pass`, but not recommended: the slice changes shared engine files and records nine plan deviations that a reviewer should see.
- **Option G: Slug-wide runtime probe** → `/wf probe football-manager-match-engine`, if a sweep across slices is wanted.

## Assumptions

- A1 (class: implementation-detail): the `defending` test command exits 101, and every failing assertion in it is a criterion the product owner moved to `lone-forward` (Q-I2, steer.md "Moved criteria and slice order"). The check is recorded as passing on this slice's scope, and the two moved criteria are recorded as measured figures, not issues. No test, limit or tolerance was changed.
- A2 (class: implementation-detail): the three kept criteria are partitioned as user-observable (observable match outcomes), following the precedent of the sibling engine slices. Their rung is `headless` (runs of the real engine library, as the plan's Verification Strategy names). No mock is involved.
- A3 (class: implementation-detail): the 57,000-match realism-band test was not run. Its bands are outside this slice (Q-E4, `realism-tuning`), and the seed-42 equal suite gives the same signals.
- A4 (class: implementation-detail): the two open deferrals from earlier slices were re-probed fresh. Neither applies to this slice, which changes no page and no packaging. The legibility probe now counts 1, but the matching line is Q-E5, where the product owner leaves the reading open. That is not a reading, so the wall stands and no ledger entry changes.
- A5 (class: implementation-detail): no consult ran. Every kept criterion is met by direct observed evidence and none is deferred, so no consult trigger holds (consult is also excluded by the product owner).

## Triage Decisions

None. No issue was found, so no triage decision was needed.

## Fix Status

No fix round ran (`metric-issues-found-initial: 0`).
