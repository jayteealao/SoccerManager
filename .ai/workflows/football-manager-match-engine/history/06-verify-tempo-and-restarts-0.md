---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: tempo-and-restarts
status: complete
stage-number: 6
created-at: "2026-09-25T12:45:00Z"
updated-at: "2026-09-25T13:10:00Z"
result: blocked-runtime-evidence-missing
metric-checks-run: 8
metric-checks-passed: 8
metric-acceptance-met: 1
metric-acceptance-total: 2
metric-acceptance-user-observable: 2
metric-acceptance-code-only: 0
metric-acceptance-mock-rung: 0
metric-interactive-checks-run: 1
metric-interactive-checks-passed: 1
metric-issues-found: 1
metric-issues-found-initial: 1
metric-issues-found-final: 1
fix-rounds-run: 0
convergence: escalated
verify-owned-fix-commit: null
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/tempo-and-restarts/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped"
ac-staleness-checked: true
ac-stale-count: 3
longitudinal-baseline-compared: "skipped — server gate unfinished when this run stopped"
stability-check-flaky-count: 0
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
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine tempo-and-restarts"
---

# Verify: Tempo and restarts

## The Verification

Implement left the slice at `d6ccbb7`. HEAD is `b9c9212`, and no engine code changed after `d6ccbb7`. The product owner's answer Q-TR3 judges this slice on two criteria: "Only real passes count", and no regression of the earlier criteria. Passes, ball in play and throw-ins moved to `realism-tuning`.

This run met "Only real passes count" on the real engine. The scenes passed on this PC (10 passed, 4 ignored). A full release match (seed 42) printed a match-stats record with `stats.passes` [1432, 1067], `stats.clearances` [3, 2] and `stats.restart_kicks` [27, 26], and the record passed its schema with 0 errors. The benchmark passed: the median of three drives is 1.5854 µs per tick, against a 1.7071 µs tripwire, and peak memory is 6.848 MB, against an 8.55 MB tripwire. Both Windows-only tests passed. The secret scan found no leaks, and the marker scan found 0 markers.

This run stopped before the no-regression evidence was complete. The server gate started at 12:46:19Z. It passed fmt and clippy (exit 0 each). The workspace suite, the release slow criteria and the calibrate suites (about 3.4 hours) had not finished. So no-regression is `blocked-runtime-evidence-missing`. It is not a deferral, because the environment can produce the evidence. It is not a failure, because no criterion was seen to fail. The next step is to run verify again and let the server gate finish (`.scratch/remote/tr-verify.sh`).

## Automated Checks Run

- `cargo fmt --all -- --check` (server): pass, exit 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` (server): pass, exit 0.
- `cargo test -p engine --all-features --test tempo_and_restarts` (this PC): pass, 10 passed, 4 ignored (`local-counting-scenes.txt`).
- `cargo test -p engine --lib relative_paths` (this PC): pass, 1 passed.
- `cargo test -p engine-cli --test calibrate a_small_run_writes` (this PC): pass, 1 passed.
- Benchmark, 3 drives plus 1 stream drive (this PC): pass. Per tick: 1.5854, 1.5743 and 1.6071 µs. Peak memory: 6.848, 6.855 and 6.707 MB. The stream drive has 0 pauses and 8.352 MB.
- `gitleaks git --log-opts=bce0144..HEAD`: pass, 7 commits, no leaks found.
- `sdlc-debt` marker scan: pass, 0 markers.
- Not finished: the server workspace suite, the release slow criteria, and the equal, strength and formations calibrate gate.

## Interactive Verification Results

- **Criterion:** Only real passes count. **Tool:** the release `engine-cli simulate --seed 42 --json` on this PC. **Evidence:** `cli-simulate-42.stdout.txt`, `cli-simulate-42-schema.txt`. **Observation:** the record has separate keys for clearances and restart kicks. The schema validates with 0 errors. **Result:** pass.
- **Criterion:** No regression of the earlier criteria. **Result:** no evidence yet. The server gate did not finish before this run stopped.

## Acceptance Criteria Status

| Criterion | Kind | Status | Method | Evidence | Evidence rung |
|---|---|---|---|---|---|
| Only real passes count | user-observable | met | interactive (the real engine's scenes and a CLI match record) | `local-counting-scenes.txt`, `cli-simulate-42.stdout.txt` | headless |
| No regression of the earlier criteria (Q-TR3) | user-observable | runtime-evidence-missing | none: the server gate did not finish | (none — runtime evidence missing) | n-a |
| Passes, ball in play, throw-ins | moved to `realism-tuning` (Q-TR3) | not judged | not run | none | n-a |

Evidence: headless 1. AC staleness: `03-slice-tempo-and-restarts.md` still lists the 3 moved criteria. Q-TR3 in `steer.md` governs.

## Issues Found

- blocked: RUNTIME-MISSING-1 — the no-regression evidence (the server workspace suite, the slow criteria and the calibrate gate) was not produced before this run stopped.

## Triage Decisions

- RUNTIME-MISSING-1 (class: implementation-detail): no fix patch applies, because this is missing evidence and not a code defect. The fix is to finish the gate run, so the fix loop did not run.

## Assumptions

- A1 (class: implementation-detail): verify judges the two criteria that Q-TR3 names, and records the 3 moved criteria as not judged.
- A2 (class: implementation-detail): the two open deferrals of other slices do not touch this slice. This slice changes no files under `web` or `packaging`. Their walls still stand, as the fresh probes show.

## Recommended Next Stage

- **Option B:** `/wf verify football-manager-match-engine tempo-and-restarts`. Run it again, and let the server gate finish.
- **Option C:** `/wf implement football-manager-match-engine tempo-and-restarts`. Use this only if the gate shows a regression.
