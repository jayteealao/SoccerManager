---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: tempo-and-restarts
status: complete
stage-number: 6
created-at: "2026-09-25T12:45:00Z"
updated-at: "2026-09-25T16:40:00Z"
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-25T16:40:00Z"
    trigger: resume
    because: "the server gate finished (12:46:19Z to 16:12:08Z), so the no-regression evidence exists"
    changed: "no regression is met on the full gate; result moves from blocked-runtime-evidence-missing to pass; the prior evidence moves to verify-evidence/tempo-and-restarts-run-1/"
result: pass
metric-checks-run: 11
metric-checks-passed: 11
metric-acceptance-met: 2
metric-acceptance-total: 2
metric-acceptance-user-observable: 2
metric-acceptance-code-only: 0
metric-acceptance-mock-rung: 0
metric-interactive-checks-run: 2
metric-interactive-checks-passed: 2
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
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/tempo-and-restarts/"
evidence-run-count: 2
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped"
ac-staleness-checked: true
ac-stale-count: 3
longitudinal-baseline-compared: "verify-evidence/keeper-and-shots/vps/ks-verify/ (11 reports, same server, same seeds)"
stability-check-flaky-count: 1
adversarial-tests-run: 0
adversarial-tests-failed: 0
failure-mode-probes-run: 0
cross-browser-delta: "none"
web-vitals-lcp-ms: null
web-vitals-cls: null
web-vitals-inp-ms: null
stack-source: confirmed
skipped-gating-specs: []
consult-runs: []
moved-criteria:
  - {criterion: "Passes are realistic", to: realism-tuning, by: "steer.md Q-TR3"}
  - {criterion: "The ball is in play for about an hour", to: realism-tuning, by: "steer.md Q-TR3"}
  - {criterion: "Throw-ins stay in band", to: realism-tuning, by: "steer.md Q-TR3"}
prior-deferrals-rechallenged:
  - {slice: viewer-match-day, touches-this-slice: false, probe: "grep -ciE \"legibility|read the match screen\" po-answers.md -> 1 (line 662, Q-E5: reading left open, not a recorded reading); git diff --stat bce0144 HEAD -- web packaging -> empty", wall: stands}
  - {slice: distribution, touches-this-slice: false, probe: "test -f verify-evidence/distribution/macos/results.json -> absent; rustup target list --installed -> aarch64-linux-android armv7-linux-androideabi x86_64-linux-android x86_64-pc-windows-msvc", wall: stands}
tags: [engine, passing, restarts]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-tempo-and-restarts.md
  plan: 04-plan-tempo-and-restarts.md
  implement: 05-implement-tempo-and-restarts.md
  review: 07-review-tempo-and-restarts.md
  adapters: runtime-adapters.md
  benchmark: 05c-benchmark.md
  prior-revision: history/06-verify-tempo-and-restarts-0.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine tempo-and-restarts"
---

# Verify: Tempo and restarts

## The Verification

Implement left the slice at `d6ccbb7`, and HEAD is `b9c9212` with no engine change after `d6ccbb7`. The product owner's answer Q-TR3 judges this slice on two criteria: "Only real passes count", and no regression of the earlier criteria. The first verify run met the counting criterion on this PC. That run stopped before the server gate finished, so no regression had no evidence.

The server gate then ran to its end (12:46:19Z to 16:12:08Z, 12,349 s). Every figure of the earlier criteria is the same as at `keeper-and-shots`. The eleven calibrate reports match the `keeper-and-shots` gate reports on the same server and seeds, key for key. Only the pass counts (about 35 fewer per team), pass accuracy (at most 0.22 points), the new ball-in-play figure and wall time differ. The slow tests give the recorded figures exactly: 0.025 second yellows, 140 wins in 200, 1.205 corners per team. The red-card test fails with the figures it had at `keeper-and-shots` (keeper arm 2.10 against 1.18). The three moved tests fail as planned targets. `realism-tuning` owns all four.

One check was unstable. The stream backpressure test failed once in the loaded server run, passed on this PC, and passed 3 of 3 on the idle server. The slice does not touch the stream crate, so this is a timing effect and not a regression. So the slice passes. The next stage is review, but the product owner's veto in `steer.md` stops the driver here.

## Automated Checks Run

- `cargo fmt --all -- --check` (server): pass, exit 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` (server): pass, exit 0.
- `cargo test --workspace --all-features --no-fail-fast` (server): exit 101 from three tests, each cleared on its own environment:
  - `data::tests::relative_paths_never_leak_the_root` and `a_small_run_writes_a_record_per_match_and_a_report_that_validate` fail on Linux by design (Windows-only measures). Both passed on this PC in run 1. The `keeper-and-shots` gate had the same two failures.
  - `a_slow_client_pauses_the_producer_and_loses_no_tick` failed once under server load. It passed on this PC (0.37 s) and 3 of 3 on the idle server (`backpressure-rerun-server.txt`).
- Slow criteria, release, `--include-ignored` (server, `criteria.txt`): 39 passed, 4 failed. The 4 failures are the moved criteria and the red-card criterion, all owned by `realism-tuning`.
- `engine-cli calibrate`, equal and strength suites on seeds 1, 7, 42, 99 and 2026, formations on seed 42, 1,000 matches each (server): the strength suites exit 0. The equal and formations suites exit 2, as at `keeper-and-shots`, on band misses that `realism-tuning` owns.
- Report compare against `keeper-and-shots` (`compare-vs-keeper-and-shots.txt`): no band differs except the pass counts, pass accuracy and wall time.
- From run 1 (this PC): the counting scenes (10 passed, 4 ignored), the benchmark (1.5854 µs per tick against a 1.7071 µs tripwire, 6.848 MB against 8.55 MB), `gitleaks` (no leaks) and the marker scan (0 markers). No engine code changed after run 1.

## Interactive Verification Results

- **Criterion:** Only real passes count. **Tool:** the release `engine-cli simulate --seed 42 --json` on this PC (run 1). **Evidence:** `../tempo-and-restarts-run-1/cli-simulate-42.stdout.txt`, `../tempo-and-restarts-run-1/cli-simulate-42-schema.txt`. **Observation:** `stats.passes` [1432, 1067], `stats.clearances` [3, 2], `stats.restart_kicks` [27, 26]. The schema validates with 0 errors. **Result:** pass.
- **Criterion:** No regression of the earlier criteria. **Tool:** the release `engine-cli calibrate` and the release slow tests on the server. **Evidence:** `status.txt`, `criteria.txt`, 11 `*/report.json`, `compare-vs-keeper-and-shots.txt`. **Observation:** every earlier figure equals the `keeper-and-shots` figure. **Result:** pass.

## Acceptance Criteria Status

| Criterion | Kind | Status | Method | Evidence | Evidence rung |
|---|---|---|---|---|---|
| Only real passes count | user-observable | met | interactive (the real engine's scenes and a CLI match record) | `../tempo-and-restarts-run-1/local-counting-scenes.txt`, `../tempo-and-restarts-run-1/cli-simulate-42.stdout.txt` | headless |
| No regression of the earlier criteria (Q-TR3) | user-observable | met | interactive (the release CLI and slow tests on the server, compared key for key with the `keeper-and-shots` gate) | `criteria.txt`, `compare-vs-keeper-and-shots.txt` | headless |
| Passes, ball in play, throw-ins | moved to `realism-tuning` (Q-TR3) | not judged | measured only | seed 42 equal: 1,190.986 passes per team, 86.533%, 89.479 minutes in play, 22.813 throw-ins | n-a |

Evidence: headless 2. AC staleness: `03-slice-tempo-and-restarts.md` still lists the 3 moved criteria. Q-TR3 in `steer.md` governs.

## Issues Found

- None blocking.
- Unstable test (not this slice): `a_slow_client_pauses_the_producer_and_loses_no_tick` can fail on a loaded slow host, because the client keeps up and the producer never pauses. It passed 3 of 3 when run again.

## Triage Decisions

- No issue entered the fix loop, so `convergence: not-needed`. The unstable test passed when run again, and it is outside this slice.

## Assumptions

- A1 (class: implementation-detail): verify judges the two criteria that Q-TR3 names, and records the 3 moved criteria as not judged.
- A2 (class: implementation-detail): the two open deferrals of other slices do not touch this slice. This slice changes no files under `web` or `packaging`. Their walls still stand.
- A3 (class: implementation-detail): a failed earlier criterion counts as a regression only if its figures moved. The red-card figures did not move.

## Recommended Next Stage

- **Option A:** `/wf review football-manager-match-engine tempo-and-restarts`. The review scope is slug-wide, and the veto in `steer.md` blocks the slug-wide review. Run it after the product owner lifts the veto.
- **Option G:** `/wf probe football-manager-match-engine`. Use this for a slug-wide runtime check.
