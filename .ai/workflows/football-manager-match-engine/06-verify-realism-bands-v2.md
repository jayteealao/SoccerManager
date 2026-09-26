---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: realism-bands-v2
status: complete
stage-number: 6
created-at: "2026-09-24T02:27:50Z"
updated-at: "2026-09-24T02:27:50Z"
result: pass
metric-checks-run: 11
metric-checks-passed: 11
metric-acceptance-met: 4
metric-acceptance-total: 4
metric-acceptance-user-observable: 4
metric-acceptance-code-only: 0
metric-acceptance-mock-rung: 0
metric-interactive-checks-run: 5
metric-interactive-checks-passed: 5
metric-issues-found: 0
metric-issues-found-initial: 2
metric-issues-found-final: 0
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: null
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [cli, web-headless, web-in-app-browser]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/realism-bands-v2/"
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
steering-honored:
  - "Dark-path counter definition: the seed-42 run reads darkpath.change_never_applied 0, darkpath.match_without_stats 0 and validate.violations 0; this stage changed no engine or computer-manager code."
  - "Design direction: this slice changes no page layout; the lineup editor lists the ten formations from the tactics schema it already reads (screenshots formations-1..3)."
  - "Output boundary: no commit was made by this stage; evidence and scripts stay under the workflow folder."
tags: [calibration, realism, observability, formations, bands]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-realism-bands-v2.md
  plan: 04-plan-realism-bands-v2.md
  implement: 05-implement-realism-bands-v2.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-realism-bands-v2.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine realism-bands-v2"
---

# Verify: Realism Bands, Version 2

## The Verification

The implement record left a release build that claims eleven sourced bands, two new record keys, and a formations suite of 55 pairings, with a seed-42 baseline in which 160 of 184 checks fail by design. This stage re-ran that claim from scratch on the release binary: one full `engine-cli calibrate --seed 42` run of all three suites at 1,000 matches per suite and per pairing (57,000 matches, 75 minutes on the 8-core reference machine), one `engine-cli simulate --seed 7`, and a headless and an in-app browser drive of the lineup editor.

All four criteria are met by live evidence. The report holds the eleven new bands with `value`, `lo`, `hi` and `pass`, and the run exits 2. Stderr carries 160 `calibrate.band_failed` lines for exactly the 160 failed checks, and the five bands the slice names all fail. `calib.formations` holds 55 rows of 1,000 matches, each with goals per match and the 10-or-more-goals share, including the four shipped formations against 4-4-2. The `simulate` record carries `stats.throw_ins` [20, 17] and `stats.goal_kicks` [1, 7] and passes its schema, which refuses the record when either key is removed. The new run is identical to the implement baseline on all 181 non-timing checks and all 55 pairing rows, so the recorded baseline is reproducible.

Two check readings failed first and cleared in the single fix round with no code change. A sibling-slice sandbox test lost a 2 ms time race while this stage's calibration run held all eight cores (434 of 434 on a quiet re-run). The first three benchmark drives put median peak memory at 6.848 MB over the 6.82 MB tripwire. Five alternating drives then read 6.719 MB against 6.711 MB for a fresh build of the parent commit, so the slice adds no memory. Review can proceed. The top open risk is the thin memory margin, which the parent commit shares.

## Verification Summary

- Slice: `realism-bands-v2`, standard mode, branch `feat/football-manager-match-engine` at `2089ed7` (code as of `0835298`; `2089ed7` adds only workflow records).
- Stack: confirmed (`stack.user-confirmed: true`, plan `stack-source: confirmed`).
- Constraint-resolution gate: the plan names no environment dependency beyond the reference machine, so no criterion needs a `constraint-resolution:` line.
- Acceptance: 4 of 4 met, all user-observable, all at `live`.
- Checks: 11 of 11 pass after the fix round (2 initial failures, both cleared by re-measurement with no code change).
- Deferrals: none for this slice.

## Automated Checks Run

1. `cargo fmt --all --check`: pass, exit 0 (`fmt.txt`).
2. `cargo clippy --workspace --all-targets -- -D warnings`: pass, exit 0 (`clippy.txt`).
3. `cargo test --workspace --no-fail-fast`: first run under load FAILED (433 passed, 1 failed, 4 ignored; `sandbox::tests::a_loop_aborts_on_the_operation_budget` read `time budget of 2 ms exceeded`, `test.txt`). Quiet re-run: pass, 434 passed, 0 failed, 4 ignored over 59 test binaries (`test-quiet.txt`).
4. `node --test web/tests/*.test.mjs`: pass, 128 of 128 (`node-test.txt`).
5. `cargo build --release -p engine-cli`: pass.
6. `gitleaks git --log-opts=5a235a4..0835298`: pass, no leaks found (`gitleaks.txt`).
7. `sdlc-debt` marker scan of `0835298` over crates, content, schemas, docs and web: 0 markers (`security-and-markers.txt`).
8. Run-report schema (`schemas/observability/run-report.schema.json`, Draft 2020-12) over the seed-42 `report.json`: pass, 0 errors (`check-s42.txt`).
9. Match-stats schema over the seed-7 `stats.json`: pass, 0 errors; negative controls refused (`simulate-s7.schema-check.txt`).
10. Benchmark gate (`bench --seed 42 --matches 5 --json`): first 3 drives FAILED on memory (median 6.848 MB against the 6.82 MB tripwire; CPU median 422.0 ms against 460.7 ms, `bench.txt`). Re-check with 5 alternating drives against the parent build: pass, slice CPU median 421.8 ms and peak memory median 6.719 MB (`bench-compare.txt`).
11. Determinism against the implement baseline (same seed and inputs): pass, 0 differences over 181 non-timing checks, and the 55 `calib.formations` rows are identical (`check-s42.txt`).

## Interactive Verification Results

- **Criterion:** Every band is reported.
  - **Platform & tool:** Windows reference machine, release `engine-cli.exe`, cli-direct.
  - **Steps performed:** `SM_DATA_DIR=<scratch> engine-cli calibrate --seed 42 --out <scratch>/runs/s42` (default 1,000 matches, all suites, 8 jobs), then `check-s42.py`.
  - **Evidence:** `verify-evidence/realism-bands-v2/s42-all.report.json`, `s42-all.exit-code`, `check-s42.txt`.
  - **Observation:** Exit code 2, `calib.pass` false. All eleven new bands are present in the equal suite with `value`, `lo`, `hi` and `pass`: `ten_plus_goals_share` 0.063 (0–0.005, fail), `sending_off_share` 0.421 (0.08–0.22, fail), `yellow_cards_per_team` 1.6 (1.2–2.6, pass), `shots_on_target_share` 0.7501 (0.30–0.42, fail), `goals_per_xg` 1.5255 (0.85–1.15, fail), `passes_per_team` 1304.772 (350–550, fail), `pass_accuracy_pct` 84.771 (75–88, pass), `corners_per_team` 0.004 (3.5–6.5, fail), `throw_ins_per_match` 25.958 (35–55, fail), `goal_kicks_per_match` 7.402 (12–22, fail), `goalless_share` 0.192 (0.04–0.12, fail). 160 of 184 checks fail.
  - **Result:** pass.
- **Criterion:** The baseline is recorded.
  - **Platform & tool:** same run, stderr.
  - **Steps performed:** counted `calibrate.band_failed` lines in `s42-all.stderr.txt` and matched each line's suite, band and pairing to the failed checks.
  - **Evidence:** `s42-all.stderr.txt`, `check-s42.txt`; baseline table in `05-implement-realism-bands-v2.md` `## Baseline`.
  - **Observation:** 160 lines for 160 failed checks; every failed check is named, with no extra names. The five named bands (10-or-more-goals share, sending-off share, shots on target, passes per team, corners per team) fail and are named. The implement record holds the run as the baseline, and this run reproduces it with 0 differences.
  - **Result:** pass.
- **Criterion:** Formations are reported.
  - **Platform & tool:** same run, `calib.formations` and the pairing checks.
  - **Steps performed:** read the 55 rows and the three checks per pairing.
  - **Evidence:** `s42-all.report.json`, `check-s42.txt`.
  - **Observation:** 55 rows, each `matches` 1000, each with `goals_per_match_mean` and `ten_plus_goals_share` (plus `goalless_share` and `goals_for_mean`). Against 4-4-2: 4-4-2 2.979 goals and 0.057 share; 4-3-3 8.587 and 0.366; 4-2-3-1 7.705 and 0.313; 3-5-2 6.35 and 0.205. 165 pairing checks, 14 pass. Suite time 4,272,114 ms.
  - **Result:** pass.
- **Criterion:** New keys pass the schema.
  - **Platform & tool:** release `engine-cli.exe simulate`, Python `jsonschema` 4.26.0 Draft 2020-12 validator.
  - **Steps performed:** `SM_DATA_DIR=<scratch> engine-cli simulate --seed 7 --ticks-out <scratch>`; validated `stats.json`; removed each new key, then gave a one-element and a negative array.
  - **Evidence:** `simulate-s7.stats.json`, `simulate-s7.schema-check.txt`.
  - **Observation:** Exit 0, `outcome` success, `stats.throw_ins` [20, 17], `stats.goal_kicks` [1, 7], 0 schema errors. Without either key: 1 error, "is a required property". A one-element or a negative array: 1 error each.
  - **Result:** pass.
- **Criterion (plan check, not a slice criterion):** the ten formations in the viewer.
  - **Platform & tool:** `engine-cli launch --seed 42`; the in-app browser (`Claude_Browser`, the chosen driver), then headless Microsoft Edge over the DevTools protocol for saved screenshots (`drive-formations.mjs`).
  - **Steps performed:** read the `data-testid="formation"` select, chose 5-4-1, read the formation slots, pressed Kick off, captured the pitch.
  - **Evidence:** `formations-1-list-default.png`, `formations-2-lineup-5-4-1.png`, `formations-3-match-5-4-1.png`, `drive-formations.json`.
  - **Observation:** 10 options in order: 4-4-2, 4-3-3, 4-2-3-1, 3-5-2, 4-1-4-1, 4-4-1-1, 4-1-2-1-2, 3-4-3, 5-3-2, 5-4-1. With 5-4-1 the slots are GK, LB, CB, CB, CB, RB, LW, CM, CM, RW, ST. After kick-off the pitch shows 11 home markers: a keeper, a back line of five, a midfield line of four, and one forward.
  - **Result:** pass.

## Acceptance Criteria Status

| Criterion | Kind | Status | Method | Evidence | Evidence rung |
|---|---|---|---|---|---|
| Every band is reported | user-observable | met | interactive (cli-direct) | `s42-all.report.json`, `s42-all.exit-code`, `check-s42.txt` | live |
| The baseline is recorded | user-observable | met | interactive (cli-direct) | `s42-all.stderr.txt`, `check-s42.txt`, implement `## Baseline` | live |
| Formations are reported | user-observable | met | interactive (cli-direct) | `s42-all.report.json` `calib.formations`, `check-s42.txt` | live |
| New keys pass the schema | user-observable | met | interactive (cli-direct) + schema validator | `simulate-s7.stats.json`, `simulate-s7.schema-check.txt` | live |

Evidence: live 4.

## Issues Found

None open. The two initial readings are closed in `## Verify-Owned Fixes`.

- informational: the benchmark memory margin is thin. The first 3-drive median (6.848 MB) exceeded the 6.82 MB tripwire; 5 alternating drives read 6.719 MB for this slice and 6.711 MB for the parent. The tripwire itself belongs to `05c-benchmark.md`, and changing it is a product-owner decision.
- informational: `sandbox::tests::a_loop_aborts_on_the_operation_budget` (scripting runtime, not touched by this slice) is sensitive to CPU load. A background task was suggested to make it deterministic.

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| TEST-LOAD-1 | check-failure (`cargo test --workspace`, 1 of 434 under full CPU load) | Fix | N/A (re-measured on a quiet machine; no code change; the test is in a sibling slice, and weakening it is forbidden) | exempt: not a code bug of this slice; a timing race caused by this stage's own concurrent calibration run | Pass (434 of 434; the test alone 5 of 5 under load) |
| BENCH-MEM | augmentation-regression (memory tripwire on the first 3-drive median) | Fix | N/A (re-measured with 5 alternating drives against a fresh build of the parent `5a235a4`; no code change) | exempt: measurement, not a code bug | Pass (median 6.719 MB against 6.82 MB; the parent reads 6.711 MB) |

Commit: (no files changed)

Regression tests added: 0

## Augmentation Verification

- **Mock fidelity (`02c-craft.md`):** not applicable to this slice's criteria; the slice changes no page. The only visible change, the formation list, reads from the tactics schema, as the lineup-editor inventory item already requires (`formations-1..2`).
- **Instrumentation (`04b-instrument.md`, `.ai/observability.md`):** the new signal `calibrate.band_failed` fires once per failed check (160 of 160). The new record keys `stats.throw_ins` and `stats.goal_kicks` are emitted and schema-checked. The dark-path counters read 0.
- **Experiment (`04c-experiment.md`):** not involved.
- **Benchmark (`05c-benchmark.md`), compare re-run:**

  | Build | Processor time per match (5 drives) | Median | Peak memory (5 drives) | Median |
  |---|---|---|---|---|
  | This slice | 409.4, 421.8, 425.0, 418.8, 428.2 ms | 421.8 ms | 6.801, 6.719, 6.715, 6.797, 6.715 MB | 6.719 MB |
  | Parent `5a235a4` | 425.0, 421.8, 415.8, 412.6, 421.8 ms | 421.8 ms | 7.227, 6.707, 6.711, 6.711, 6.707 MB | 6.711 MB |

  Tripwires 460.7 ms and 6.82 MB. The earlier 3-drive set on this slice read 412.6, 425.0, 422.0 ms and 6.848, 6.848, 6.738 MB.

## Security Scan

- CVE scan: no dependency changed in `0835298` (`Cargo.toml`, `Cargo.lock` and the crate manifests are untouched); `cargo audit` is not installed.
- Secret detection: `gitleaks` 8.30.1 over `5a235a4..0835298`, no leaks found.
- SAST: none installed. A pattern scan of the slice diff for credentials found nothing.

## Accessibility Gate

Not automatable for this slice: it adds only six `<option>` entries to an existing labelled select, which the viewer-lineup-tactics gate already covers. No new component. New WCAG AA violations: 0 observed.

## Performance Gate

- Bundle size: skipped; no web asset changed except the tactics data file the page reads.
- Processor time per match: 421.8 ms median, equal to the parent.
- Calibration run time: `calib.wall_ms` total 4,458.6 s (equal 90.2 s, strength 96.3 s, formations 4,272.1 s) and `duration_ms` 4,510.7 s for seed 42 of all suites, against the implement record's 4,590 s wall.

## Cross-Slice Regression

Siblings checked through the whole workspace suite (434 tests) and the web suite (128 tests): calibration, probe-engine-core, tactics-and-ai, viewer-lineup-tactics, experiment-flags and the rest. Regressions found: 0. The one first-run failure is the load race in TEST-LOAD-1, not a regression.

## Longitudinal Delta

- Calibration report: baseline source is the implement run `implement-evidence/realism-bands-v2/baseline-s42.report.json`. Delta: 0 over 181 non-timing checks and 55 pairing rows. Interpretation: expected; every match is deterministic by seed.
- Viewer: no layout baseline compared; the only visible change is the option list.

## Friction Notes

- The team sheet on the match screen lists each player's own position, not the slot. With 5-4-1 it still shows two ST entries. This behaviour existed before this slice. Informational.

## Free Exploration Notes

- The first engine-side formation proof is the pitch shape; no event carries the formation. — informational
- The strength suite exits 0 on its own, but `--suite all` exits 2 while any band fails, as designed. — informational

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Empty submission | n-a | no form input in this slice |
| Max-length input | n-a | no text input |
| Double-click / rapid repeat | n-a | no new control |
| Mid-flow interruption | n-a | calibrate interruption is covered by the calibration slice |
| Offline / network failure | n-a | local-only CLI |

Schema negative controls were run instead: a record missing each new key, a one-element array and a negative value are all refused.

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Slow response (Fast 3G) | n-a | no network path |
| Concurrent session | n-a | the calibration run and two viewer sessions ran at the same time without interference |
| Session expiry mid-flow | n-a | no session |

## Cross-Browser Delta

The in-app browser and headless Microsoft Edge showed the same 10 options and slots. Divergences: none.

## Web Vitals

Not measured; this slice changes no page code.

## Gaps / Unverified Areas

- The ignored slow test `a_thousand_matches_hold_the_realism_bands` was not run; it fails by design until the tuning slice. The full seed-42 run exercises the same code paths.
- Seeds 1, 7, 99 and 2026 were not re-run by this stage. The seed-42 run is identical to its baseline, so the other seeds' baselines are taken as recorded.
- Prior deferrals re-challenged (they do not touch this slice): `grep -ciE "legibility|read the match screen" po-answers.md` -> `1`. The one hit is the product owner's answer Q-E5, "leave both for later", not a recorded reading, so that wall still stands. `rustup target list --installed` -> `aarch64-linux-android armv7-linux-androideabi x86_64-linux-android x86_64-pc-windows-msvc` (no apple-darwin target), and `test -f .../distribution/macos/results.json` -> exit 1, so the macOS wall still stands. Neither deferral was changed.

## Freshness Research

Not required: no test failed for a code reason, the plan is 1 day old, and no external API changed. No dependency version moved since the plan (`Cargo.lock` untouched in `0835298`).

## Recommendation

Proceed to review. The slice does what it promises: calibration now reports the eleven sourced bands, the formations suite, and the two new record keys. Its failing bands are the intended, reproducible baseline for the next slices.

## Recommended Next Stage

- **Option A (default): Review** → `/wf review football-manager-match-engine realism-bands-v2`. `result: pass`, `convergence: converged` (1 round, no code change). Compact first: the 75-minute run and the benchmark drives are noise for review.
- **Option D: Skip review** → `/wf handoff football-manager-match-engine realism-bands-v2`. Only if the slug-wide review ledger already covers this change. Not recommended, because the slice touches the paired-run verdict and the run-report contract.
- **Option G: Slug-wide runtime probe** → `/wf probe football-manager-match-engine`. Use it for a cross-slice sweep after the tuning slices land.

## Assumptions

- A-V1 (class: implementation-detail): all four criteria are user-observable (command output, a written report, stderr, a written record). They are driven live on the release binary; none is partitioned code-only.
- A-V2 (class: implementation-detail): the checks ran inline, without verify sub-agents. One calibration run covers three criteria and one `simulate` covers the fourth, so parallel children would add no coverage.
- A-V3 (class: implementation-detail): the full 1,000-match seed-42 run of all suites was re-run (75 minutes), not reused from the implement record, so the evidence is this stage's own.
- A-V4 (class: implementation-detail): the plan's viewer check of the formations is recorded as an extra interactive check, not as a slice criterion.

## Triage Decisions

- TEST-LOAD-1 → Fix (autonomous). Method: re-run on a quiet machine. The failure is a 2 ms time race in a sibling slice's test, caused by this stage's own 8-worker run. A code change to that test is outside this slice's scope, so a background task was suggested instead. (class: implementation-detail)
- BENCH-MEM → Fix (autonomous). Method: controlled re-measure, 5 alternating drives of this slice and a fresh parent build. The slice and the parent read the same memory, so this slice caused no regression. Moving the tripwire is not this stage's call. (class: implementation-detail)

## Fix Status

Round 1 of 1 ran. Both issues were re-checked and pass. No file changed, so there is no commit.
