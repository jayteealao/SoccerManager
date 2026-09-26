---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: tuning-loop
status: complete
stage-number: 6
created-at: "2026-09-24T10:55:00Z"
updated-at: "2026-09-24T10:55:00Z"
result: pass
metric-checks-run: 6
metric-checks-passed: 6
metric-acceptance-met: 7
metric-acceptance-total: 7
metric-acceptance-user-observable: 7
metric-acceptance-code-only: 0
metric-acceptance-mock-rung: 0
metric-interactive-checks-run: 7
metric-interactive-checks-passed: 7
metric-issues-found: 0
metric-issues-found-initial: 1
metric-issues-found-final: 0
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: "a447ff1"
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/tuning-loop/"
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
prior-deferrals-rechallenged:
  - {slice: viewer-match-day, touches-this-slice: false, probe: "grep -ciE \"legibility|read the match screen\" po-answers.md -> 1 (line 662, Q-E5: the product owner leaves the reading open; not a recorded reading)", wall: stands}
  - {slice: distribution, touches-this-slice: false, probe: "test -f verify-evidence/distribution/macos/results.json -> absent", wall: stands}
steering-honored:
  - "Moved criteria and slice order (2026-09-24): the red-card suite's verdict (fail) is reported and not judged here; the criterion, its 1.6 limit and the card values belong to lone-forward and are unchanged."
  - "A targeted calibrate run is an inner loop (Q-I1): --suite all still plays equal, strength and formations only (55 pairings in the seed-7 full run), and red-card runs only when named."
  - "Output boundary: the one verify-time commit (a447ff1) uses product language and was leak-checked (no match)."
tags: [calibration, tooling, realism, performance]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-tuning-loop.md
  plan: 04-plan-tuning-loop.md
  implement: 05-implement-tuning-loop.md
  review: 07-review-tuning-loop.md
  adapters: runtime-adapters.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  experiment: 04c-experiment.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine tuning-loop"
---

# Verify: Tuning loop

## The Verification

Implement left the tuning loop at commit `a6dc5d2`: targeted `calibrate` runs, a baseline diff with a sampling error on every band, the red-card experiment as a suite, and a measured build profile that was not kept. No code changed after that commit until this run. This run drove every criterion again with the release binary on the 8-core reference machine. The results were not copied from the implement record.

All 7 criteria are met from headless runs made in this run. One targeted pairing at 1,000 matches took 88.6 s with its diff, the equal suite 91.0 s, and the red-card experiment 48.1 s. The targeted pairing played 1,000 matches of that pairing only. A fresh `--suite all` run on seed 7 matched the targeted pairing, equal suite and strength suite exactly. Four wrong baselines were refused with exit 1 in 0.0 s, with no run folder and a message that names the difference. A real change (a flag that shortens the shot range) marked 6 of 15 rows as change. A rerun marked all 15 as noise. The red-card suite's 9 figures equal the slow test's 9 figures, which were printed in this run. Both faster build candidates gave identical figures and were not faster beyond the spread between two release runs.

The first workspace test run failed once in a stream test from an earlier slice: 379 passed and 1 failed before cargo stopped. The failure showed a real gap. The server did not log `socket.client_gone` when a viewer left while a match message, not a tick, was being sent. The one fix round added that log (commit `a447ff1`). After the fix, 479 tests passed and 0 failed, and the stream test file passed 10 of 10 reruns. The slice can go to review. The top open risk is still noise: a share band with few events has a wide error, for example 0.0067 on the targeted pairing's goalless share.

## Verification Summary

- Criteria: 7 of 7 met. All 7 are user-observable (command output, exit codes, run folders), and all 7 have headless evidence from this run.
- Checks: 6 of 6 pass after the fix round. Before the fix round, the workspace tests failed once.
- Fix round: 1 issue found, 1 fixed, 0 left. Commit `a447ff1`.
- Deferrals: none. The slice needs no device, credential or external service.

## Automated Checks Run

- `cargo fmt --check`: pass (`fmt.txt`, and `fmt-after-fix.txt` after the fix).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass (`clippy.txt`, `clippy-after-fix.txt`).
- `cargo test --workspace --all-features`: FAILED on the first run. 44 test binaries ran, with 379 passed, 1 failed and 7 ignored, before cargo stopped at `stream_cli` (`a_viewer_that_closes_in_mid_match_ends_the_run_without_an_error`: "the run must name the viewer that went away"; `workspace-tests.txt`). After the fix: `--no-fail-fast`, 63 binaries, 479 passed, 0 failed, 7 ignored (the slow tests) (`workspace-tests-after-fix.txt`).
- `cargo build --release -p engine-cli`: pass (`build-release.txt`, `build-release-after-fix.txt`).
- `gitleaks git --log-opts=2b62cd3..a6dc5d2`: pass, no leaks found (`gitleaks.txt`).
- Benchmark compare (`engine-cli bench --seed 42 --matches 5 --json`, 3 drives): pass. The per-tick time and peak memory are under their tripwires (`bench-1.json` to `bench-3.json`).
- Extra evidence run (not counted as a check): the slow test `a_sending_off_gives_no_advantage` (`--release --ignored`), run for its printed figures (`slow-test.txt`). Its assertion fails, as it did before this slice. That criterion belongs to `lone-forward` (steer.md).

## Interactive Verification Results

The adapter is `cli`, which runs the release binary headless. The drive script is `verify-evidence/tuning-loop/drive.sh`. The figures come from `analyse.py`, and its output is in `analysis.json`. Each run's line is in `drives.txt`.

- **Criterion:** A targeted run is fast.
  - **Platform and tool:** Windows 11, 8 logical cores (`nproc` = 8), release `engine-cli.exe`, shell clock.
  - **Steps performed:** `drive.sh speed`. This made three runs, each with `--baseline`: (1) `--suite formations --pairing "4-4-1-1 v 4-4-2" --seed 42 --matches 1000`, (2) `--suite equal --seed 42 --matches 1000`, (3) `--suite red-card --seed 1 --matches 120`.
  - **Evidence:** `verify-evidence/tuning-loop/drives.txt`, `pair-v.*`, `equal-v.*`, `red-v.*`.
  - **Observation:** The shell wall times, including the diff, were 88.6 s, 91.0 s and 48.1 s. Later release runs took 84.5 s for the pairing and 40.6 s for the red-card experiment.
  - **Result:** pass. Every run is under 120 s.
- **Criterion:** A targeted run plays only its selection.
  - **Platform and tool:** as above.
  - **Steps performed:** run (1) above.
  - **Evidence:** `pair-v.report.json`, `analysis.json` → `pair-v`.
  - **Observation:** The run folder holds 1000 `stats/` files. `calib.formations` has 1 entry. `calib.suites` names only `formations`, with `matches` 1000. `calib.selection` is `{suites: [formations], pairings: ["4-4-2 v 4-4-1-1"], bands: []}`.
  - **Result:** pass.
- **Criterion:** A targeted run agrees with the full run.
  - **Platform and tool:** as above.
  - **Steps performed:** `drive.sh agree`. This ran a fresh `--suite all --seed 7 --matches 40` (2280 matches, 191.9 s), then the targeted pairing, the equal suite and the strength suite on the same seed and match count. At 1,000 matches, runs (1) and (2) were also compared with the seed-42 `--suite all` report that the same code made at implement (`implement-evidence/tuning-loop/full-42.report.json`). No code under `crates/`, `schemas/` or `Cargo.*` changed between `a6dc5d2` and the start of this run (`code-diff-since-impl.txt` is empty).
  - **Evidence:** `agree-*.report.json`, `analysis.json` → `agree`, `pair-v`, `equal-v`.
  - **Observation:** In the fresh seed-7 run, the pairing entry, the pairing's band rows, the equal and strength figures (without the wall-time `outliers` count) and their band rows all equal the full run's. All 4 reports have `fixtures.hash` `c4c0a247c3c9`. At seed 42 and 1,000 matches, `equals_full_entry`, `rows_equal_full` and `figures_equal_full` are all true.
  - **Result:** pass.
- **Criterion:** The diff separates change from noise.
  - **Platform and tool:** as above.
  - **Steps performed:** `drive.sh change`, run on a copied content folder with one declared flag: `change-base` (equal suite, seed 9, 300 matches), then `change-rerun` against it, then `change-on` with `--flag probe_short_range=on` against it. The diffs from runs (1) to (3) were read as well.
  - **Evidence:** `change-*.stderr.txt`, `pair-v.stderr.txt`, `red-v.stderr.txt`, `analysis.json`.
  - **Observation:** Every row prints baseline, new, change, error, mark and verdict. The rerun gave 15 of 15 rows as noise, with every change at 0. The flag run gave 6 rows as change: goal kicks -5.27 ± 0.217, goals per xG -0.7233 ± 0.053, pass accuracy -0.314 ± 0.105, passes -12.04 ± 4.04, shots on target share +0.179 ± 0.0056, and shots -2.52 ± 0.45. It gave 9 rows as noise, with `content changed` in the header. The rule "noise when |change| ≤ 2 × error" holds on every row (`noise_rule_holds: true`).
  - **Result:** pass.
- **Criterion:** A wrong baseline is refused. The product owner restated this in Q-P1 as "a baseline with different fixtures is refused".
  - **Platform and tool:** as above.
  - **Steps performed:** `drive.sh refuse`. This ran seed 43 against a seed-42 baseline, 999 matches against a 1000-match baseline, an edited tactics formation (so the fixtures differ), and a file that is not a report.
  - **Evidence:** `refuse-*.stderr.txt`, `drives.txt`.
  - **Observation:** Each run exits with 1 in 0.0 s and leaves no run folder. The messages are: "seed 43 differs from the baseline's 42", "match count 999 differs from the baseline's 1000", "fixtures hash 75c8b55d190e differs from the baseline's c4c0a247c3c9", and "it is not the run report of a calibrate run". The `change-on` run shows that a different content hash on the same fixtures is accepted.
  - **Result:** pass.
- **Criterion:** The red-card experiment is a suite.
  - **Platform and tool:** as above, plus `cargo test --release -p engine --all-features --test defending a_sending_off_gives_no_advantage -- --ignored --nocapture`.
  - **Steps performed:** run (3) above, and the slow test run in this run.
  - **Evidence:** `red-v.report.json`, `slow-test.txt`.
  - **Observation:** `calib.red_card` has control 0.775 / 2.2333, keeper 2.0583 / 5.8167, centre-back 0.7083 / 2.3917, striker 0.95 / 3.8417, and limit 1.24. The slow test printed the same 9 values to 4 decimals. The suite reports each arm, the control and `pass: false`, which is the same verdict as the slow test's failed assertion. The run has 6 band rows (2 per arm) and 0 validator violations.
  - **Result:** pass. The criterion asks for the figures and the verdict to be reported, and they are. The red-card criterion itself failing is the product state, and it belongs to `lone-forward`.
- **Criterion:** A faster profile changes no result.
  - **Platform and tool:** as above. Two candidates were built into separate scratch target folders through environment variables, so no repository file was touched: (b) `CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1` (96 s), and (c) (b) plus `RUSTFLAGS="-C target-cpu=native"` (101 s).
  - **Steps performed:** `drive.sh profile-a2 profile-b profile-c profile-red-a2`. The pairing and the red-card suite were run with each build, each against the release run's report.
  - **Evidence:** `pair-a2|b|c.*`, `red-a2|b|c.*`, `build-b.txt`, `build-c.txt`, `analysis.json`.
  - **Observation:** Both candidates give a diff of exactly 0 on every row. Their `calib.suites`, `calib.formations` and `calib.red_card` objects are equal to the release run's. Pairing rates in matches per second: release 11.87 and 12.44, (b) 12.61, (c) 12.33. Red-card rates: release 10.93 and 13.14, (b) 13.27, (c) 13.28. The best candidate gain over the faster release run is +1.4% on the pairing and +1.1% on the red-card suite. The spread between the two release runs is 4.8% and 20%. `Cargo.toml` still has only `[profile.release] debug = 1`. The how-to records the measurement (`docs/how-to/calibration.md:104-108`).
  - **Result:** pass. No profile is faster, the measurement is recorded, and the current profile stays.

## Acceptance Criteria Status

| Criterion | kind | status | verification method | evidence | evidence-rung |
|---|---|---|---|---|---|
| A targeted run is fast | user-observable | met | interactive (headless CLI) | `drives.txt`: 88.6 s / 91.0 s / 48.1 s | headless |
| A targeted run plays only its selection | user-observable | met | interactive (headless CLI) | `pair-v.report.json`: 1000 stats, 1 pairing | headless |
| A targeted run agrees with the full run | user-observable | met | interactive (headless CLI) | `analysis.json` → `agree` (fresh seed 7), `pair-v`/`equal-v` (seed 42) | headless |
| The diff separates change from noise | user-observable | met | interactive (headless CLI) | `change-rerun`/`change-on` stderr and reports | headless |
| A wrong baseline is refused | user-observable | met | interactive (headless CLI) | `refuse-*.stderr.txt`, exit 1, no run folder | headless |
| The red-card experiment is a suite | user-observable | met | interactive (headless CLI) + slow test | `red-v.report.json` = `slow-test.txt`, 9 of 9 | headless |
| A faster profile changes no result | user-observable | met | interactive (headless CLI) | `pair-*`/`red-*` reports, diff 0, rates | headless |

Evidence: headless 7.

## Issues Found

- (resolved) `severity: medium` — `crates/engine-cli/src/serve.rs:414`: when a viewer left while a match message was being sent, `serve` swallowed `StreamError::ClientGone` with no `socket.client_gone` signal. The `stream_cli` test caught this once in 11 runs. It was fixed in `a447ff1`.

No open issues.

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| TEST-1 (`stream_cli::a_viewer_that_closes_in_mid_match_ends_the_run_without_an_error`) | check-failure | Fix (autonomous) | Patched | exempt: a timing race between the viewer's drop and a message send that no test can force without a fault-injection seam; the existing test already covers the signal, and it caught the gap | Pass (workspace 479/0; `stream_cli` file 10 of 10) |

Commit: `a447ff1`

Regression tests added: 0

Root cause: the failed run's log showed `socket.session_closed ticks=0` with no `socket.client_gone` line and exit code 2. The only path that ends a started session with no log line is the `Err(StreamError::ClientGone) => None` arm in `Serving::play`. It is reached when the message route (an event or a statistics message), rather than the tick sink, finds the socket closed. The patch logs the signal in that arm with `written = 0` and the reason "the viewer disconnected while a message was sent". This slice's diff does not touch `serve.rs`, `stream/` or `stream_cli.rs` (`git diff --stat 2b62cd3 a6dc5d2` on those paths is empty). The gap was already in the code, and the fix does not change this slice's figures.

## Augmentation Verification

- `benchmark` (baseline on `2089ed7`, in `05c-benchmark.md`), compared over three drives:

  | Target | Baseline | This run (median) | Delta | Tripwire | Result |
  |---|---|---|---|---|---|
  | processor time per tick (gate) | 1.4753 µs | 1.5422 µs (1.5524, 1.4777, 1.5422) | +4.5% | 1.6228 µs | pass |
  | peak memory (gate) | 6.617 MB | 6.672 MB (6.672, 6.773, 6.625) | +0.8% | 8.27 MB | pass |
  | processor time per match | 422.0 ms | 450.0 ms | +6.6% | reported | 291,800 ticks per match, the same as the defending re-check; under the 2000 ms budget |
  | full match wall time | 424 ms | 452 ms | +6.6% | reported | — |

  The delta equals the defending-and-discipline re-check, which is where the tick count moved. The only engine change in this slice, `send_off_before_kickoff`, runs before kick-off.
- `instrument` (the match-rules signal set): every drive reports `validate.violations` 0. The run reports carry the new additive keys, and the schema tests validate them (`calibrate_targeted.rs` loads `RecordSchemas`). `calibrate.band_failed` fires once for each band that misses. The server fix adds a `socket.client_gone` line on the path that had none. No designed signal is missing.
- `experiment` (experiment-flags): the `change-on` drive turned a declared flag on with `--flag probe_short_range=on`. The content hash changed while the fixtures hash stayed the same, and the diff shows the flag's effect. The flag path works with the baseline guard.

## Security Scan

- CVE scan: `cargo audit` is not installed (`error: no such command: audit`). The only dependency change is `sha2 = { workspace = true }` in `crates/engine-cli/Cargo.toml`. It adds one `Cargo.lock` dependency line and no new crate or version.
- Secret detection: `gitleaks` over `2b62cd3..a6dc5d2` found no leaks.
- SAST: none is installed. The slice adds no network, file-permission or process-spawn surface. The hidden `--pairing-numbers` worker flag is internal, and the help test keeps it hidden.

## Accessibility Gate

Not applicable. The slice changes no page. Every output is a report, standard error, an exit code or a file count.

## Performance Gate

Bundle size: skipped, because no page changed. The engine's processor time per tick is +4.5% against a +10% tripwire. The calibrate throughput is 11.9 to 13.3 matches per second. Build time was not compared.

## Cross-Slice Regression

Siblings checked: every workspace test (63 binaries, 479 passed after the fix), including the calibration, experiment-flags, probe-engine-core, realism-bands-v2 and defending-and-discipline suites. No regressions were found that this slice caused. The one failure came from `stream-protocol`/`integration` code that this slice does not touch, and it is fixed.

## Longitudinal Delta

- Targeted runs against the implement-time reports (`full-42`, `red-base`): change 0 on every row, 3 + 15 + 6 rows. This is expected, because the code is the same.
- Throughput: the implement run measured 12.58 matches per second on the pairing, and this run measured 11.87 and 12.44. That is within the run-to-run spread. This is expected.

## Friction Notes

- A calibrate run with a band miss exits with code 2 even when the diff is all noise. That is the documented behaviour ("The diff does not change the exit code"), but a script that tests the diff must read `calib.diff`, not the exit code.

## Free Exploration Notes

- The first release red-card run (10.93 matches per second) was 17% slower than the next one (13.14). The first run after a build seems to pay a warm-up cost. Anyone who compares profiles should discard the first run. Informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Baseline on another seed | pass | exit 1, no run folder |
| Baseline with another match count | pass | exit 1, no run folder |
| Baseline on other fixtures (edited tactics) | pass | exit 1, names both hashes |
| A file that is not a calibrate report | pass | exit 1, "not the run report of a calibrate run" |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Slow response / concurrent session / session expiry | n-a | Offline command-line tool with no session. |

## Gaps / Unverified Areas

- `--minutes` is not part of the baseline identity (implement caveat, Q-P1 lists the inputs). A baseline at other minutes is accepted. This is not tested as a refusal, because the product owner's answer does not require it.
- `cargo audit` is not installed, so no CVE database was consulted. No new crate was added.

## Freshness Research

- AC staleness: checked. No criterion names an external API or service, and the plan is from 2026-09-24. 0 criteria are stale.
- Cargo profile keys (`lto`, `codegen-units` through `CARGO_PROFILE_RELEASE_*`) worked on cargo 1.92.0 in this run. Both candidate builds succeeded.

## Assumptions

Autonomous run: no product owner was present. Every decision is class-stamped.

- V1 (class: implementation-detail; ac: "A targeted run is fast"; classification: runtime-evidence): headless release runs on this machine, which is the reference machine (8 cores). The shell clock includes the diff.
- V2 (class: implementation-detail; ac: "A targeted run agrees with the full run"; classification: runtime-evidence): a fresh full run at 40 matches on seed 7 is the fresh agreement evidence. The 1,000-match comparison uses the implement-time full seed-42 report, which the same code made (the code diff since then is empty). A fresh 81-minute full run adds no information that the fresh seed-7 run does not already give.
- V3 (class: implementation-detail; ac: "The red-card experiment is a suite"; classification: runtime-evidence): the slow test's assertion fails, and that is the moved criterion (steer.md, 2026-09-24). This AC is judged on figure parity and a reported verdict, not on the verdict passing.
- V4 (class: implementation-detail; ac: "A faster profile changes no result"; classification: runtime-evidence): "faster" means a gain beyond the spread between two release runs. The candidates were built through environment variables into scratch folders, so no repository file changed.
- V5 (class: implementation-detail): the failing `stream_cli` test was triaged Fix under the autonomous policy. The root cause is a missing log line in `serve.rs`. This is a one-arm patch with no behaviour change beyond the log, so it is minimal and in reach.
- V6 (class: implementation-detail): the fix was applied inline, not by a write-isolated sub-agent, because this run has no sub-agent tool. The patch was then checked with fmt, clippy, the full workspace tests and 10 reruns of the test file.
- V7 (class: implementation-detail): no regression test was added. The race cannot be forced without a fault-injection seam. The existing test covers the signal and caught the gap.
- V8 (class: implementation-detail): the fix commit `a447ff1` was made by path, so the staged `docs/design/realism/*` files, which belong to another session, stayed out of it.
- V9 (class: implementation-detail): the prompt said a prior fix round had already run, but no `06-verify-tuning-loop.md` or `verify-evidence/tuning-loop/` existed on disk. This is treated as the first verify run (`evidence-run-count: 1`, `fix-rounds-run: 1` for this run's round).
- V10 (class: implementation-detail): the prior dead driver had already set `tuning-loop` to `status: complete` in `00-index.md` and in `03-slice.md` before verify. This run's `result: pass` now supports that status. It is kept, not changed.
- V11 (class: implementation-detail): the prior deferrals (viewer-match-day legibility, distribution macOS) were re-probed in this run. Neither touches a command-line calibrate slice on Windows. Both walls still stand, and neither is carried into this slice.

## Triage Decisions

- TEST-1, the `stream_cli` intermittent failure: Fix (autonomous policy: every fixable issue is fixed). Patched, and the re-check passes. class: implementation-detail.

## Fix Status

1 issue was triaged Fix, and 1 was patched. The re-check passed: 479 workspace tests passed and 0 failed, and the stream test file passed 10 of 10 reruns. Nothing was escalated or skipped.

## Recommendation

Proceed to review. Every criterion has fresh headless evidence, the one check failure was fixed and committed, and no deferral is open for this slice.

## Recommended Next Stage

- **Option A (default): Review** → `/wf review football-manager-match-engine tuning-loop`. `result: pass`, `convergence: converged`. Consider compacting first, because verify was long.
- **Option D: Skip review** → `/wf handoff football-manager-match-engine tuning-loop`. The slice is tooling, but it adds an engine function and a server fix, so review is still worthwhile.
- **Option G: Slug-wide runtime probe** → `/wf probe football-manager-match-engine`. Use this if a cross-slice sweep is wanted before `lone-forward` starts.
