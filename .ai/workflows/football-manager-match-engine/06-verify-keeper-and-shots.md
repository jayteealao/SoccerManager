---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: keeper-and-shots
status: complete
stage-number: 6
created-at: "2026-09-24T23:33:26Z"
updated-at: "2026-09-25T02:55:52Z"
result: pass
metric-checks-run: 10
metric-checks-passed: 10
metric-acceptance-met: 4
metric-acceptance-total: 4
metric-acceptance-user-observable: 4
metric-acceptance-code-only: 0
metric-acceptance-mock-rung: 0
metric-interactive-checks-run: 4
metric-interactive-checks-passed: 4
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
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/keeper-and-shots/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped"
ac-staleness-checked: true
ac-stale-count: 1
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
  - {criterion: "No advantage from a red card", to: realism-tuning, by: "po-answers.md Q-KS1; steer.md 2026-09-24", measured-this-run: "not run; its slow test and the red-card suite are unchanged by this slice"}
amended-criteria:
  - {criterion: "Set pieces arise naturally", from: "corners per team at least 3.0", to: "corners per team at least 1.2 over 200 matches; goal kicks at least 10 per match and the defending-touch origin unchanged", by: "po-answers.md Q-KS2; steer.md 2026-09-24"}
prior-deferrals-rechallenged:
  - {slice: viewer-match-day, touches-this-slice: false, probe: "grep -ciE \"legibility|read the match screen\" po-answers.md -> 1 (line 662, Q-E5: the product owner leaves the reading open; not a recorded reading); git diff --stat a30313b HEAD -- web packaging -> empty", wall: stands}
  - {slice: distribution, touches-this-slice: false, probe: "test -f verify-evidence/distribution/macos/results.json -> absent; rustup target list --installed -> aarch64-linux-android armv7-linux-androideabi x86_64-linux-android x86_64-pc-windows-msvc (no apple-darwin)", wall: stands}
steering-honored:
  - "keeper-and-shots criteria changes (Q-KS1, Q-KS2): the slice is judged on four criteria, with the corner floor at 1.2 per team over 200 matches. The red-card criterion belongs to realism-tuning and was not judged or run."
  - "Heavy runs go to the server (2026-09-25): fmt, clippy, the workspace suite, every release slow test and every calibrate run ran on the server through the sync script. The benchmark and the two Windows-only tests ran on this PC."
  - "A slice gate runs the full suites (Q-I1): equal and strength on seeds 1, 7, 42, 99 and 2026, and formations on seed 42, 1,000 matches each."
  - "Dark-path counter definition: every equal-suite report reads darkpath.change_never_applied 0 and reports change.expired_at_full_time (74 to 105)."
  - "Output boundary: this stage made no commit; evidence stays under the workflow folder."
tags: [engine, shots, set-pieces, realism]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-keeper-and-shots.md
  plan: 04-plan-keeper-and-shots.md
  implement: 05-implement-keeper-and-shots.md
  review: 07-review-keeper-and-shots.md
  adapters: runtime-adapters.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  prior-unfinished-evidence: verify-evidence/keeper-and-shots-run-0/
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine keeper-and-shots"
---

# Verify: Keeper and shots

## The Verification

Implement left the slice at commit `bce0144`. A shot flies on a trajectory and counts as on target only when that flight crosses between the posts under the bar. The keeper saves on the sourced curve and holds one save in three. A defender within reach can block a shot. The product owner then moved the red-card criterion to `realism-tuning` (Q-KS1) and set this slice's corner floor to 1.2 per team (Q-KS2). An earlier verify run on 2026-09-24 stopped without writing a record: its formations suite ran on this PC and never finished, and this PC crashed during long all-core runs. Its evidence is kept in `verify-evidence/keeper-and-shots-run-0/`, and this run did not use any figure from it. No file outside the workflow folder changed after `bce0144`, so this run verified the code at `bce0144`.

All 4 kept criteria are met from headless runs made in this run. Every heavy run went to the server. Over 200 matches, corners are 1.205 per team and goal kicks 11.205 per match, and the test found no corner without a defending last touch at the goal line. On the five gate seeds of 1,000 matches, on target is 0.376 to 0.387 of shots and goals per xG is 1.084 to 1.111. The wide and over scenes give a goal kick with no save and no on-target count. 398 of 500 penalties convert (0.796). The workspace suite passed 501 tests on the server and failed only the two tests that fail on Linux by design, and both of those passed on this PC. All 10 of 10 checks passed, so the fix loop had nothing to do. Processor time per tick is 1.5638 µs (median of three drives), 3.0% under the 1.613 µs baseline and under the 1.7743 µs tripwire.

The slice can go to review. The top open risk is realism outside this slice's criteria. Goals per match are 1.70 to 2.14 on the gate seeds, under the 2.4 to 3.2 band, and 6 of 16 equal-suite bands miss on every seed (corners, goalless share, goals, passes, throw-ins and yellow cards). The product owner accepted goals out of band until `realism-tuning`. The corner floor has 0.005 of headroom, so a later change that lowers parries or blocks will trip it.

## Verification Summary

- Criteria: 4 of 4 kept criteria met. All 4 are user-observable (match outcomes printed by release engine runs and scripted scenes), and all 4 have headless evidence from this run.
- Amended criterion: "Set pieces arise naturally" is judged at 1.2 corners per team (Q-KS2), not 3.0. The goal-kick floor and the corner-origin rule are unchanged.
- Moved criterion: "No advantage from a red card" moved to `realism-tuning` (Q-KS1). It was not run and not judged.
- Checks: 10 of 10 pass.
- Fix round: none needed (0 issues).
- Deferrals: none. The slice needs no device, credential or external service. The two open deferrals of other slices were probed again and do not touch this slice.
- Where each run ran: the server (Linux, 6 cores, through `.scratch/remote/vps.sh`) ran fmt, clippy, the workspace suite, the release slow tests and every calibrate suite. This PC ran the benchmark, the two Windows-only tests, the secret scan and the marker scan.

## Automated Checks Run

- `cargo fmt --all -- --check` (server): pass, exit 0 (`vps/ks-verify/fmt.txt`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` (server): pass, exit 0 (`vps/ks-verify/clippy.txt`).
- `cargo test --workspace --all-features --no-fail-fast`: pass. On the server, 65 result lines, 501 passed, 2 failed, 10 ignored (the slow tests) (`vps/ks-verify/workspace-tests.txt`, exit 101). The 2 failures are `data::tests::relative_paths_never_leak_the_root` and `a_small_run_writes_a_record_per_match_and_a_report_that_validate`, the two tests that steering names as failing on Linux by design. Both passed on this PC: `cargo test -j 6 -p engine --lib relative_paths` (1 passed, `windows-only-relative-paths.txt`) and `cargo test -j 6 -p engine-cli --test calibrate a_small_run_writes` (1 passed, `windows-only-small-run.txt`). No other test failed.
- `cargo build --release -p engine-cli`: pass on the server (`vps/ks-verify/build-release.txt`) and on this PC (used by the benchmark).
- Cross-slice slow tests (server, release): `cargo test --release -p engine --all-features --test defending --test discipline --test strength -- --include-ignored --nocapture every_formation_holds discipline_is_realistic stronger`: pass, 3 of 3 (`vps/ks-verify/regress-slow.txt`).
- Strength suite gate (server): `engine-cli --content-dir content calibrate --suite strength --seed <s> --matches 1000` for seeds 42, 1, 7, 99 and 2026: pass, exit 0 on each (`strength-table.txt`).
- Formations suite gate (server): `engine-cli --content-dir content calibrate --suite formations --seed 42 --matches 1000`: pass: exit 2 (band misses only, the recorded-not-failed exit, Q-E4), 9,337 s, 55 pairings of 1,000 matches (55,000 matches), 0 failed matches, `validate.violations` 0, `darkpath.change_never_applied` 0 (`vps/ks-verify/formations-42/report.json`, `formations-delta.txt`).
- `gitleaks git --log-opts=a30313b..HEAD`: pass. 3 commits scanned, no leaks found (`gitleaks.txt`).
- `sdlc-debt:` marker scan over `git diff a30313b HEAD -- crates content docs`: pass, 0 markers (`debt-markers.txt`).
- Benchmark compare (`target/release/engine-cli.exe bench --seed 42 --matches 5 --json`, 3 drives, and 1 stream drive, on this PC): pass. See Augmentation Verification (`bench-1.json` to `bench-3.json`, `bench-stream.json`).
- Extra evidence, not counted as a check: the equal suite on the five gate seeds (exit 2 on each, band misses only), which is the evidence for "Shots are counted honestly" (`equal-table.txt`).

## Interactive Verification Results

The adapter is `cli`: release builds of the real engine, run headless. The server script is recorded in `vps/ks-verify/status.txt` with each step's exit code and time. Local steps are in `drives.txt`.

- **Criterion:** Set pieces arise naturally (floor 1.2 corners per team, Q-KS2).
  - **Platform and tool:** the server (Linux x86_64, 6 cores), `cargo test --release`.
  - **Steps performed:** `cargo test --release -p engine --all-features --test keeper_and_shots -- --include-ignored --nocapture`.
  - **Evidence:** `verify-evidence/keeper-and-shots/vps/ks-verify/criteria-keeper-and-shots.txt`.
  - **Observation:** `corners per team 1.205, goal kicks per match 11.205`, then `test set_pieces_arise_from_play ... ok`. The test fails if any corner lacks a defending last touch within one tick of the goal line. The parry scene printed `parries that gave a corner: 14 of 40`. Census over 400 matches: wide or over 0.514, blocked 0.121, held 0.090, parried 0.188, scored 0.068, other 0.019.
  - **Result:** pass. 1.205 ≥ 1.2 per team; 11.205 ≥ 10 per match; no corner without a defending touch over the line.
- **Criterion:** Shots are counted honestly.
  - **Platform and tool:** the server, the release `engine-cli`.
  - **Steps performed:** `./target/release/engine-cli --content-dir content calibrate --suite equal --seed <s> --matches 1000 --out ../out/ks-verify/equal-<s>` for seeds 42, 1, 7, 99 and 2026.
  - **Evidence:** `verify-evidence/keeper-and-shots/vps/ks-verify/equal-<s>/report.json`, `equal-table.txt`.
  - **Observation:** on target 0.387, 0.3827, 0.3844, 0.3827 and 0.3759; goals per xG 1.0976, 1.0838, 1.0908, 1.1114 and 1.0851. Each run is 1,000 matches, so the five seeds together are 5,000 matches.
  - **Result:** pass. Every seed is inside 0.30–0.42 and 0.85–1.15.
- **Criterion:** A wide shot stays wide.
  - **Platform and tool:** as for the first criterion.
  - **Steps performed:** the same run.
  - **Evidence:** `criteria-keeper-and-shots.txt`.
  - **Observation:** `a_wide_shot_is_not_on_target_is_not_saved_and_gives_a_goal_kick ... ok` and `a_shot_over_the_bar_is_not_on_target_and_gives_a_goal_kick ... ok`. Each scene scripts a successful save roll with the keeper in reach, so a save attempt would have held the ball.
  - **Result:** pass. Not on target, no save attempt, and a goal kick to the defending side.
- **Criterion:** Penalties convert realistically.
  - **Platform and tool:** as for the first criterion.
  - **Steps performed:** the same run (`penalties_convert_seventy_to_eighty_five_percent`, 500 scripted penalty scenes).
  - **Evidence:** `criteria-keeper-and-shots.txt`.
  - **Observation:** `penalties: 398 of 500 scored, 0.796`, then `... ok`.
  - **Result:** pass. 0.796 is inside 0.70–0.85.

The server results equal the implement record's Windows figures on every printed number (1.205, 11.205, 0.387, 1.0976, 398 of 500, the census and the refit coefficients). This agrees with the steering note that server and Windows results match.

## Acceptance Criteria Status

| Criterion | Kind | Status | Method | Evidence | Evidence rung |
|---|---|---|---|---|---|
| Set pieces arise naturally (1.2 floor, Q-KS2) | user-observable | met | interactive (headless release engine runs, 200 matches) | `vps/ks-verify/criteria-keeper-and-shots.txt` | headless |
| Shots are counted honestly | user-observable | met | interactive (headless release calibrate runs, 5 seeds × 1,000 matches) | `vps/ks-verify/equal-*/report.json`, `equal-table.txt` | headless |
| A wide shot stays wide | user-observable | met | interactive (scripted scenes on the real engine) | `vps/ks-verify/criteria-keeper-and-shots.txt` | headless |
| Penalties convert realistically | user-observable | met | interactive (500 scripted penalty scenes on the real engine) | `vps/ks-verify/criteria-keeper-and-shots.txt` | headless |
| No advantage from a red card | moved to `realism-tuning` (Q-KS1) | not judged | not run | none | n-a |

Evidence: headless 4.

The partition follows Step B. Each kept criterion names an observable result of a match (corners, goal kicks, the on-target share, goals per xG, the restart, the penalty result) that a run of the engine prints. The implement record calls "A wide shot stays wide" a build-capability criterion. This run judges it user-observable, because it names a restart the viewer shows, and it is met on the headless rung either way.

AC staleness: 1 criterion is stale in the slice definition. `03-slice-keeper-and-shots.md` still reads "at least 3.0" corners per team. The product owner's answer Q-KS2 and `steer.md` set 1.2, and this run judged 1.2. The slice definition is left as the implement record's assumption A11 describes.

## Issues Found

None.

## Augmentation Verification

- **Benchmark (`05c-benchmark.md`, baseline on `a30313b`, gate per tick +10% and peak memory +25%), this PC, `bce0144`:**

  | Drive | Processor time per tick | Processor time per match | Wall time per match | Ticks per match | Peak memory |
  |---|---|---|---|---|---|
  | 1 | 1.5638 µs | 447.0 ms | 449 ms | 285,850 | 6.867 MB |
  | 2 | 1.5631 µs | 446.8 ms | 455 ms | 285,850 | 6.848 MB |
  | 3 | 1.5743 µs | 450.0 ms | 454 ms | 285,850 | 6.848 MB |

  Median 1.5638 µs per tick against the 1.613 µs baseline and the 1.7743 µs tripwire: −3.0%, pass. Peak memory median 6.848 MB against the 8.39 MB tripwire: pass. Processor time per match 447.0 ms against 446.8 ms at baseline (+0.04%), under the 491.5 ms line. Ticks per match rose from 277,000 to 285,850 (+3.2%), because more corners and goal kicks add dead-ball time. The stream drive delivered 438,125 ticks per second with 0 pauses and 8.348 MB peak memory, under its 10.23 MB tripwire. The machine hash is `74ca12fc08a4`, the same reference machine as the baseline. `05c-benchmark.md` stays in baseline mode, as for the earlier slices; the compare is recorded here.
- **Instrumentation (`04b-instrument.md`):** every equal-suite report reads `darkpath.change_never_applied` 0, `darkpath.match_without_stats` 0 and `validate.violations` 0. `change.expired_at_full_time` is 74, 105, 93, 76 and 89 on seeds 42, 1, 7, 99 and 2026. Every strength report reads `validate.violations` 0 and `darkpath.change_never_applied` 0.
- **Craft (`02c-craft.md`) and experiment (`04c-experiment.md`):** not applicable. The slice changes no page and no flag. `git diff --stat a30313b HEAD -- web packaging Cargo.lock` is empty (`untouched-surfaces.txt`).

## Security Scan

- CVE scan: skipped; no dependency was added or upgraded (`Cargo.lock` unchanged by the slice).
- Secret detection: gitleaks, 3 commits, no leaks found.
- SAST: clippy with `-D warnings`, clean. No new unsafe code.

## Accessibility Gate

Not applicable: no user interface changed. `a11y-result: not-automatable` records that no page is in scope.

## Performance Gate

- Bundle size: skipped; no web bundle changed.
- Per-tick processor time: −3.0% against a +10% tripwire (see Augmentation Verification).
- Cold start: not measured; the command-line start path did not change.

## Cross-Slice Regression

Every sibling slice's test suite is in the workspace run: 501 passed on the server, and the 2 Linux-only failures passed on this PC. The earlier slices' slow criterion tests pass in release on shipped play: `every_formation_holds` (4-4-1-1 2.87–0.65 and 3-4-3 2.88–0.88 against 4-4-2; every side at or under 2.88), `discipline_is_realistic` (0.025 second yellows per match, 11.5% of matches with a sending-off, 0 same-tick pairs) and the strength test (won 140, drew 33, lost 27 of 200). The strength suite passes on all five gate seeds; the stronger club's win rate is 0.642, 0.671, 0.677, 0.673 and 0.648. The formations suite on seed 42 (55,000 matches) against the lone-forward gate run (`implement-evidence/lone-forward/shipped/gate/formations-42/report.json`): on target 0.7814 → 0.3853, goals per xG 1.1477 → 0.9401, corners per team 0.012 → 2.467, goal kicks 8.718 → 18.825 per match, goals per match 3.867 → 3.331, ten-plus-goal share 0.084 → 0.0689, and the highest pairing mean falls from 17.04 to 9.67 goals per match. Band misses move from 119 to 122 of 166, all in three bands (goals per match 45, goalless share 40, ten-plus-goal share 37). Per pairing the movement is mixed: 6 pairings newly miss the ten-plus-goal band and 3 newly meet it, 8 newly miss the goals band and 7 newly meet it, 7 newly miss the goalless band and 8 newly meet it. No criterion of an earlier slice reads these bands; `every_formation_holds` is the judged formations criterion and it passes. The band misses are recorded under Free Exploration Notes. Regressions found: 0.

## Longitudinal Delta

- Baseline: `implement-evidence/keeper-and-shots/baseline/equal-base/report.json` (seed 42, 1,000 matches, on `a30313b`, before the slice).
- This run: the seed-42 equal suite on the server, exit 2 (band misses), 174 s.
- Delta on seed 42: on target 0.7728 → 0.387 and goals per xG 1.4955 → 1.0976 (both into band). Corners per team 0.005 → 1.59. Goal kicks 5.875 → 13.087 per match (into band). Throw-ins 18.096 → 22.813. Goals per match 2.841 → 1.884 (leaves the 2.4–3.2 band). Goalless share 0.099 → 0.22 (leaves its band). Ten-plus-goal share 0.008 → 0.0. Shots per team 12.924 → 13.195. Sending-off share 0.113 → 0.098. Yellow cards per team 0.912 → 0.935.
- Interpretation: expected change. The seed-42 figures equal the implement record's gate figures exactly. On every gate seed, 6 of 16 bands miss: corners, goalless share, goals, passes, throw-ins and yellow cards. Band misses are recorded, not failed (Q-E4). Band tuning belongs to `realism-tuning`.

## Friction Notes

- Goals per match fell by a third (2.841 → 1.884 on seed 42), and 22% of matches end goalless. A player will notice fewer goals until `realism-tuning`. The slice risk named this, and the product owner accepted it.

## Free Exploration Notes

- The earlier unfinished verify run tried the formations suite on this PC twice. The first try ran for 4,067 s and exited 127, and the second exited 127 after 840 s. Its memory log shows commit charge near 260 GB. That run is the reason the product owner moved heavy runs to the server. This run did not use its figures — informational.
- The formations suite shows open-play scoring far above real football in some pairings: 48.1% of 4-3-3 v 4-3-3 matches and 43.3% of 4-3-3 v 5-3-2 matches reach ten goals, and the highest pairing mean is 9.67 goals per match. This was already so before the slice (the lone-forward gate run had 87.9% and 17.04), and no criterion of this slice covers it. `realism-tuning` owns the bands, and another task in progress investigates the collapse of a side with ten men — informational (for `realism-tuning`; not a finding against this slice).
- `rules.dead_ball_stalled` warnings for injury stoppages appear in that earlier formations log. Another task in progress investigates the AI queueing refused injury substitutions. The criteria of this slice do not cover it — informational.

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

- The corner floor has 0.005 of headroom on a seeded run. The run is deterministic, so it passes on this tree. A later change that lowers parries or blocks will trip it.
- "A wide shot stays wide" is judged on the scripted scenes, the plan's named rung. No traced full match was examined for a wide shot followed by a goal kick. The equal suite's goal-kick rate (12.2 to 14.4 per match) shows the effect over whole matches.
- The server's workspace run reports one test fewer than the implement record's Windows run (503 against 504, with the ignored tests equal at 10). The two Linux failures were run on this PC. The Windows-only tests are compiled only on Windows. No test failed on either machine for a reason other than its platform.

## Freshness Research

Not needed. No test failed for a code reason, the plan is from 2026-09-24, and the slice touches no external API or schema. No dependency was added or upgraded.

## Recommendation

Proceed to review. All four kept criteria pass on shipped play in this run, every check passes and the benchmark is inside its tripwire.

## Recommended Next Stage

- **Option A (default): Review.** `/wf review football-manager-match-engine keeper-and-shots`. `result: pass`, convergence not-needed. Compact the session first: this run held long server logs.
- **Option D: Skip review.** `/wf handoff football-manager-match-engine keeper-and-shots`. Valid because `result: pass`, but not advised: the slice moves match outcomes and the save model for every later slice.
- **Option G: Slug-wide runtime probe.** `/wf probe football-manager-match-engine`, when a slug-wide sweep is wanted after the realism slices.

## Assumptions

- A1 (class: implementation-detail): the earlier verify run's evidence folder had no verify record, so it is archived as `keeper-and-shots-run-0/`, and this run counts as the first run (`evidence-run-count: 1`).
- A2 (class: implementation-detail): the two Linux-only failures in the server workspace run are platform failures, not issues, because steering names both and both passed on this PC in this run.
- A3 (class: implementation-detail): the equal suite was run without `--baseline` on the server. The seed-42 delta was computed from the two reports (`equal-table.txt`), as the baseline file stays on this PC.
- A4 (class: implementation-detail): `05c-benchmark.md` stays in baseline mode. The earlier slices' verify runs recorded the compare in the verify record and did not rewrite the benchmark record.
- A5 (class: implementation-detail; ac: "Set pieces arise naturally"; classification: runtime-evidence): judged at the product owner's 1.2 floor from Q-KS2.
- A6 (class: implementation-detail; ac: "A wide shot stays wide"; classification: runtime-evidence): judged user-observable on the headless rung.

## Triage Decisions

- No issue reached triage (0 issues). No fix sub-agent was dispatched.
- Intent-bearing decisions: 0. Q-KS1 and Q-KS2 were answered by the product owner and applied as written.
- The formations band misses (see Free Exploration Notes) were not triaged as issues (class: implementation-detail). No criterion of this slice or of an earlier slice reads those bands, band misses are recorded and not failed (Q-E4), and `realism-tuning` owns them.
- Recovered error: the first write of this record carried placeholder values and the frontmatter check rejected it. The values were filled and the record passed on the next write.
