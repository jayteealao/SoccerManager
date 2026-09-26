---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: lone-forward
status: complete
stage-number: 6
created-at: "2026-09-24T16:23:17Z"
updated-at: "2026-09-24T16:23:17Z"
result: pass
metric-checks-run: 7
metric-checks-passed: 7
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
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/lone-forward/"
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
  - {criterion: "No advantage from a red card", to: keeper-and-shots, by: "po-answers.md Q-LF2; steer.md 2026-09-24", measured-this-run: "control home 1.0083 away 1.0375; keeper full 1.6375 reduced 2.6667; centre-back full 0.8792 reduced 1.0792; striker full 1.2083 reduced 1.7875; limit 1.6133 (slow test exit 101)"}
prior-deferrals-rechallenged:
  - {slice: viewer-match-day, touches-this-slice: false, probe: "grep -ciE \"legibility|read the match screen\" po-answers.md -> 1 (line 662, Q-E5: the product owner leaves the reading open; not a recorded reading); git diff --stat a447ff1 HEAD -- web -> empty", wall: stands}
  - {slice: distribution, touches-this-slice: false, probe: "test -f verify-evidence/distribution/macos/results.json -> absent; rustup target list --installed -> aarch64-linux-android armv7-linux-androideabi x86_64-linux-android x86_64-pc-windows-msvc (no apple-darwin)", wall: stands}
steering-honored:
  - "Red-card criterion moves to keeper-and-shots (Q-LF2): the slice is judged on three criteria. The red-card slow test was run for its figures and recorded; it fails as the implement record says, and it is not judged here."
  - "Moved criteria and slice order (2026-09-24): the 4-4-1-1 and 3-4-3 pairings were judged here at the unchanged 4.0 limit."
  - "A slice gate runs the full suites (Q-I1): every_formation_holds ran at 120 seeds per pairing and discipline_is_realistic at 200 matches, in release. The equal suite ran at 1,000 matches on seed 42 against the pre-slice baseline."
  - "Dark-path counter definition: the seed-42 equal suite reads darkpath.change_never_applied 0, change.expired_at_full_time 134, validate.violations 0."
  - "Output boundary: this stage made no commit; evidence stays under the workflow folder."
tags: [engine, tactics, rules, realism]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-lone-forward.md
  plan: 04-plan-lone-forward.md
  implement: 05-implement-lone-forward.md
  review: 07-review-lone-forward.md
  adapters: runtime-adapters.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine lone-forward"
---

# Verify: Lone forward

## The Verification

Implement left the slice at commit `07d87c2`: `tackle_win_base` 0.5, `lone_hold` 0.5 and `lone_dribble` −0.5, with the red-card criterion moved to `keeper-and-shots` by the product owner (Q-LF2). The later commit `a30313b` changed no file outside the workflow folder, so this run verified the code at `07d87c2`. Every criterion was driven again in this run with release builds. No figure was copied from the implement record.

All 3 kept criteria are met from headless runs made in this run. In `every_formation_holds`, 4-4-1-1 scores 3.22–0.56 and 3-4-3 scores 2.91–0.78 against 4-4-2, under the 4.0 limit. The eight kept pairings are all at or under 2.31 per side. `discipline_is_realistic` gives 0.045 second yellows per match, 11.0% of matches with a sending-off and 0 same-tick pairs. The pressed-forward scene passes on all 40 seeds with the shipped weights. The workspace suite passed 489 of 489 tests (7 slow tests ignored), and all 7 checks passed, so the fix loop had nothing to do. Processor time per tick is 1.613 to 1.6245 µs, 4.6% to 5.3% above the 1.5422 µs baseline and under the 1.6964 µs tripwire.

The slice can go to review. The top open risk is realism outside this slice's criteria. A fresh seed-42 equal suite reproduces the implement figures exactly: yellow cards fall from 1.821 to 0.912 per team, under the 1.2 band floor, and 8 of 16 bands miss. The red-card slow test still fails (keeper arm: reduced side 2.67 against 1.64), and `keeper-and-shots` now owns it.

## Verification Summary

- Criteria: 3 of 3 kept criteria met. All 3 are user-observable (match outcomes printed by the release slow tests and the scene test), and all 3 have headless evidence from this run.
- Moved criterion: "No advantage from a red card" moved to `keeper-and-shots` (Q-LF2). It was measured and recorded, not judged.
- Checks: 7 of 7 pass.
- Fix round: none needed (0 issues).
- Deferrals: none. The slice needs no device, credential or external service.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (`fmt.txt`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass (`clippy.txt`).
- `cargo test --workspace --all-features --no-fail-fast`: pass. 64 binaries, 489 passed, 0 failed, 7 ignored (the slow tests) (`workspace-tests.txt`).
- `cargo build --release -p engine-cli`: pass (`build-release.txt`).
- `gitleaks git --log-opts=a447ff1..HEAD`: pass. 6 commits scanned, no leaks found (`gitleaks.txt`).
- `sdlc-debt:` marker scan over `git diff a447ff1 HEAD -- crates content docs`: pass, 0 markers (`debt-markers.txt`).
- Benchmark compare (`target/release/engine-cli.exe bench --seed 42 --matches 5 --json`, 3 drives): pass. See Augmentation Verification (`bench-1.json` to `bench-3.json`).
- Extra evidence, not counted as a check: the seed-42 equal suite against the pre-slice baseline (exit 2, band misses only; see Longitudinal Delta), and the moved red-card slow test (exit 101; see Acceptance Criteria Status).

## Interactive Verification Results

The adapter is `cli`: release builds of the real engine, run headless. The drive script is `verify-evidence/lone-forward/drive.sh`, and each step's exit code and time is in `drives.txt`.

- **Criterion:** The lone-forward formations hold.
  - **Platform and tool:** Windows 11, 8 logical cores, `cargo test --release`.
  - **Steps performed:** `drive.sh criteria`, which runs `cargo test --release -p engine --all-features --test defending --test discipline -- --include-ignored --nocapture every_formation_holds discipline_is_realistic`.
  - **Evidence:** `verify-evidence/lone-forward/criteria-formations-discipline.txt`.
  - **Observation:** 4-4-1-1 v 4-4-2: 3.22 - 0.56. 3-4-3 v 4-4-2: 2.91 - 0.78. `every_formation_holds ... ok` in 72.68 s.
  - **Result:** pass. No side averages more than 4.0 goals per match in either pairing.
- **Criterion:** Nothing that passes now regresses.
  - **Platform and tool:** as above.
  - **Steps performed:** the same run.
  - **Evidence:** `criteria-formations-discipline.txt`, `workspace-tests.txt`.
  - **Observation:** 4-4-2 1.03–0.95, 4-3-3 2.31–0.32, 4-2-3-1 2.21–1.83, 3-5-2 0.97–2.17, 4-1-4-1 0.84–0.78, 4-1-2-1-2 0.83–0.72, 5-3-2 0.43–0.83, 5-4-1 1.15–1.09. `discipline_is_realistic`: second yellows per match 0.045, matches with a sending-off 11.0%, same-tick pairs 0 (200 matches, 11.71 s). The workspace suite, which holds every earlier slice's tests, passed 489 of 489.
  - **Result:** pass. All eight pairings are at or under 4.0 per side; 0.045 ≤ 0.10; 11.0% ≤ 25%; 0 same-tick pairs.
- **Criterion:** A lone forward uses his team-mates.
  - **Platform and tool:** as above; scripted scenes on the real engine with the shipped `content/tuning.json` (`lone_hold` 0.5, `lone_dribble` −0.5, read back in this run at lines 85 and 86).
  - **Steps performed:** `cargo test --release -p engine --all-features --test lone_forward -- --nocapture`.
  - **Evidence:** `criteria-lone-forward.txt`.
  - **Observation:** `a_pressed_lone_forward_passes_or_holds_instead_of_dribbling ... ok` (seeds 1–40: a pressed lone striker with an open team-mate 12 m behind him passes back or holds the ball). `the_lone_terms_need_a_lone_carrier ... ok` (with a forward team-mate ahead the terms change nothing), `the_lone_forward_holds_the_onside_line_while_his_side_has_the_ball ... ok`, `a_contact_below_the_win_chance_wins_the_ball_and_above_it_is_a_foul ... ok`. 4 passed, 0 failed, 0 ignored.
  - **Result:** pass.

## Acceptance Criteria Status

| Criterion | Kind | Status | Method | Evidence | Evidence rung |
|---|---|---|---|---|---|
| The lone-forward formations hold | user-observable | met | interactive (headless release engine runs, 120 seeds per pairing) | `criteria-formations-discipline.txt` | headless |
| Nothing that passes now regresses | user-observable | met | interactive (headless release engine runs) + workspace suite | `criteria-formations-discipline.txt`, `workspace-tests.txt` | headless |
| A lone forward uses his team-mates | user-observable | met | interactive (scripted scenes on the real engine, 40 seeds) | `criteria-lone-forward.txt` | headless |
| No advantage from a red card | moved to `keeper-and-shots` (Q-LF2) | not judged | measured | `redcard-slow-test.txt` | n-a |

Evidence: headless 3.

The partition follows Step B: each kept criterion names an observable post-condition (goals per match, card rates, the carrier's choice) printed by a run of the engine. The moved criterion was measured in this run: control home 1.0083, away 1.0375; keeper arm full 1.6375, reduced 2.6667; centre-back arm full 0.8792, reduced 1.0792; striker arm full 1.2083, reduced 1.7875; limit 1.6133. The test fails on four assertions, as the implement record says.

## Issues Found

None.

## Augmentation Verification

- **Benchmark (`05c-benchmark.md`, baseline on `a447ff1`, gate per tick +10% and peak memory +25%):**

  | Drive | Processor time per tick | Processor time per match | Wall time per match | Ticks per match | Peak memory |
  |---|---|---|---|---|---|
  | 1 | 1.613 µs | 446.8 ms | 450 ms | 277,000 | 6.648 MB |
  | 2 | 1.6245 µs | 450.0 ms | 448 ms | 277,000 | 6.746 MB |
  | 3 | 1.6245 µs | 450.0 ms | 451 ms | 277,000 | 6.652 MB |

  Baseline 1.5422 µs per tick (median of three), tripwire 1.6964 µs: +4.6% to +5.3%, pass. Peak memory tripwire 8.33 MB: pass. Processor time per match is under the 495.0 ms line reported beside the gate. Ticks per match fell from 291,800 to 277,000, which the changed play explains (fewer fouls and restarts).
- **Instrumentation (`04b-instrument.md`):** the seed-42 equal suite report reads `darkpath.change_never_applied` 0, `darkpath.match_without_stats` 0, `validate.violations` 0 and `change.expired_at_full_time` 134. The `calibrate.band_failed` and `calibrate.diff` signals fired on stderr (`equal-42.stderr.txt`).
- **Craft (`02c-craft.md`) and experiment (`04c-experiment.md`):** not applicable. The slice changes no page and no flag; `git diff --stat a447ff1 HEAD -- web packaging` is empty.

## Security Scan

- CVE scan: skipped; no dependency was added or upgraded (`Cargo.lock` unchanged by the slice).
- Secret detection: gitleaks, 6 commits, no leaks found.
- SAST: clippy with `-D warnings`, clean. No new unsafe code.

## Accessibility Gate

Not applicable: no user interface changed. `a11y-result: not-automatable` records that no page is in scope.

## Performance Gate

- Bundle size: skipped; no web bundle changed.
- Per-tick processor time: +4.6% to +5.3% against a +10% tripwire (see Augmentation Verification).
- Cold start: not measured; the command-line start path did not change.

## Cross-Slice Regression

Every sibling slice's test suite is in the workspace run: 489 passed, 0 failed. The four seeded tests that implement moved to another match (`every_play_event_names_a_player`, the full-match commentary test, the snapshot resume and round-trip tests, the viewer reconnect test) pass. Regressions found: 0.

## Longitudinal Delta

- Baseline: `implement-evidence/lone-forward/baseline/equal-base/report.json` (seed 42, 1,000 matches, before the slice).
- This run: `target/release/engine-cli.exe calibrate --suite equal --seed 42 --matches 1000 --baseline <that report>`, exit 2 (band misses), 83 s (`equal-42.report.json`, `equal-42.stderr.txt`).
- Delta: 15 rows, 9 change and 6 noise. Goals per match 4.092 → 2.841 (into band), shots per team 17.165 → 12.924 (into band), sending-off share 0.326 → 0.113 (into band), ten-plus-goal share 0.043 → 0.008, goalless share 0.047 → 0.099, goal kicks 7.926 → 5.875, yellow cards per team 1.821 → 0.912 (leaves the 1.2–2.6 band). Fouls per team 7.016.
- Interpretation: expected change. The figures equal the implement record's seed-42 gate figures exactly, so the shipped build reproduces. Band misses are 8 of 16 (corners, goal kicks, goals per xG, passes, shots-on-target share, ten-plus-goal share, throw-ins and yellow cards). Band misses are recorded, not failed (Q-E4); band tuning belongs to `realism-tuning`.

## Friction Notes

- An even tackle now wins the ball ten times as often as before (clean-win chance 0.025 → 0.25). Fouls per team are 7.0, and yellow cards fall below the band floor. The slice risk named this.

## Free Exploration Notes

- The default-club seed-42 match now ends 0-0 (implement record). The equal suite's goals per match (2.841) is inside its band, so this is the clubs' strength gap, not a league-wide drop — informational.
- The red-card slow test fails when slow tests are included. A default `cargo test` run stays green because the test is ignored by default — informational; `keeper-and-shots` owns it.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Empty submission | n-a | The slice adds no input surface. |
| Max-length input | n-a | As above. |
| Double-click / rapid repeat | n-a | As above. |
| Offline / network failure | n-a | The engine runs locally with no network. |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Slow response | n-a | No network path in the slice. |
| Concurrent session | n-a | As above. |
| Session expiry | n-a | As above. |

## Gaps / Unverified Areas

- The pressed-forward criterion is judged on the scripted scene, the plan's named rung. No traced full match counting the lone forward's passes and holds under pressure was made; the formations and equal-suite figures show the effect on whole matches.
- `tests/lone_forward.rs` builds its scenes from the shipped content file. A later change to the shipped weights would move the scene's result, and the guard in `weighted()` only checks the direction of the weights.

## Freshness Research

Not needed. No test failed, the plan is from 2026-09-24, and the slice touches no external API or schema. No dependency was added or upgraded.

## Recommendation

Proceed to review. All three kept criteria pass on shipped play in this run, every check passes and the benchmark is inside its tripwire.

## Recommended Next Stage

- **Option A (default): Review.** `/wf review football-manager-match-engine lone-forward`. `result: pass`, convergence not-needed.
- **Option D: Skip review.** `/wf handoff football-manager-match-engine lone-forward`. Valid because `result: pass`, but not advised: the slice moves match outcomes and tuning values for every later slice.
