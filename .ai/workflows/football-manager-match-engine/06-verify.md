---
schema: sdlc/v1
type: verify-index
slug: football-manager-match-engine
status: in-progress
stage-number: 6
created-at: "2026-09-21T22:40:58Z"
updated-at: "2026-09-25T16:40:00Z"
slices-verified: 23
slices-total: 24
slices:
  - {slug: engine-core, result: pass, convergence: not-needed, artifact: 06-verify-engine-core.md, verified-at: "2026-09-21T22:40:58Z"}
  - {slug: data-schemas-generator, result: pass, convergence: converged, fix-commit: "ae31329", artifact: 06-verify-data-schemas-generator.md, verified-at: "2026-09-22T10:01:47Z"}
  - {slug: stream-protocol, result: pass, convergence: converged, fix-commit: "c7f7357", artifact: 06-verify-stream-protocol.md, verified-at: "2026-09-22T13:35:00Z"}
  - {slug: viewer-pitch, result: pass, convergence: converged, fix-commit: "c7c54bd", artifact: 06-verify-viewer-pitch.md, verified-at: "2026-09-22T18:49:11Z"}
  - {slug: match-rules, result: pass, convergence: converged, fix-commit: "a4893c8", artifact: 06-verify-match-rules.md, verified-at: "2026-09-22T22:02:26Z"}
  - {slug: tactics-and-ai, result: pass, convergence: converged, fix-commit: "e79ad61", artifact: 06-verify-tactics-and-ai.md, verified-at: "2026-09-22T23:38:05Z"}
  - {slug: commentary, result: pass, convergence: not-needed, artifact: 06-verify-commentary.md, verified-at: "2026-09-23T00:07:54Z"}
  - {slug: calibration, result: pass, convergence: converged, fix-commit: "3ec2198", artifact: 06-verify-calibration.md, verified-at: "2026-09-23T07:28:00Z"}
  - {slug: viewer-match-day, result: partial, convergence: not-needed, interactive-verification: deferred, artifact: 06-verify-viewer-match-day.md, verified-at: "2026-09-23T09:49:45Z"}
  - {slug: viewer-lineup-tactics, result: pass, convergence: converged, fix-commit: "8a5a182", artifact: 06-verify-viewer-lineup-tactics.md, verified-at: "2026-09-23T11:36:00Z"}
  - {slug: viewer-reports-recovery, result: pass, convergence: not-needed, artifact: 06-verify-viewer-reports-recovery.md, verified-at: "2026-09-23T13:12:00Z"}
  - {slug: integration, result: pass, convergence: converged, fix-commit: "f6f36b6", artifact: 06-verify-integration.md, verified-at: "2026-09-23T16:07:18Z"}
  - {slug: extra-time-penalties, result: pass, convergence: not-needed, artifact: 06-verify-extra-time-penalties.md, verified-at: "2026-09-23T16:50:36Z"}
  - {slug: experiment-flags, result: pass, convergence: not-needed, artifact: 06-verify-experiment-flags.md, verified-at: "2026-09-23T17:17:16Z"}
  - {slug: scripting-runtime, result: pass, convergence: not-needed, artifact: 06-verify-scripting-runtime.md, verified-at: "2026-09-23T17:59:05Z"}
  - {slug: distribution, result: partial, convergence: converged, fix-commit: "89fe959", interactive-verification: deferred, artifact: 06-verify-distribution.md, verified-at: "2026-09-23T18:35:00Z"}
  - {slug: probe-engine-core, result: pass, convergence: not-needed, artifact: 06-verify-probe-engine-core.md, verified-at: "2026-09-23T19:17:29Z"}
  - {slug: realism-bands-v2, result: pass, convergence: converged, artifact: 06-verify-realism-bands-v2.md, verified-at: "2026-09-24T02:27:50Z"}
  - {slug: defending-and-discipline, result: pass, convergence: not-needed, artifact: 06-verify-defending-and-discipline.md, verified-at: "2026-09-24T07:01:29Z"}
  - {slug: tuning-loop, result: pass, convergence: converged, fix-commit: "a447ff1", artifact: 06-verify-tuning-loop.md, verified-at: "2026-09-24T10:55:00Z"}
  - {slug: lone-forward, result: pass, convergence: not-needed, artifact: 06-verify-lone-forward.md, verified-at: "2026-09-24T16:23:17Z"}
  - {slug: keeper-and-shots, result: pass, convergence: not-needed, artifact: 06-verify-keeper-and-shots.md, verified-at: "2026-09-25T02:57:00Z"}
  - {slug: tempo-and-restarts, result: pass, convergence: not-needed, artifact: 06-verify-tempo-and-restarts.md, verified-at: "2026-09-25T16:40:00Z"}
tags: [engine, rust, viewer, laws, snapshot, lineup, tactics]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine tempo-and-restarts"
---

# Verify Index

| Slice | Result | Convergence | Checks | Acceptance | Interactive | Artifact |
|---|---|---|---|---|---|---|
| `engine-core` | pass | not-needed | 12 / 12 | 6 / 6 (1 user-observable, live) | 3 / 3 | [06-verify-engine-core.md](06-verify-engine-core.md) |
| `data-schemas-generator` | pass | converged (1 round, fix `ae31329`) | 13 / 13 | 6 / 6 (0 user-observable) | not applicable | [06-verify-data-schemas-generator.md](06-verify-data-schemas-generator.md) |
| `stream-protocol` | pass | converged (1 round, fix `c7f7357`) | 15 / 15 | 6 / 6 (0 user-observable) | not applicable | [06-verify-stream-protocol.md](06-verify-stream-protocol.md) |
| `viewer-pitch` | pass | converged (2 verify runs; fixes `75a4ba6` and `c7c54bd`) | 12 / 13 | 8 / 8 (5 user-observable, headless) | 5 / 5 | [06-verify-viewer-pitch.md](06-verify-viewer-pitch.md) |
| `match-rules` | pass | converged (1 round, fix `a4893c8`) | 16 / 16 | 8 / 8 (1 user-observable, live) | 1 / 1 | [06-verify-match-rules.md](06-verify-match-rules.md) |
| `tactics-and-ai` | pass | converged (1 round, fix `e79ad61`) | 16 / 16 | 9 / 9 (1 user-observable, live) | 1 / 1 | [06-verify-tactics-and-ai.md](06-verify-tactics-and-ai.md) |
| `commentary` | pass | not-needed | 13 / 13 | 4 / 4 (0 user-observable) | not applicable | [06-verify-commentary.md](06-verify-commentary.md) |
| `calibration` | pass | converged (2 verify runs; fixes `aedf0e3` and `3ec2198`) | 14 / 14 | 5 / 5 (1 user-observable, live) | 1 / 1 | [06-verify-calibration.md](06-verify-calibration.md) |
| `viewer-match-day` | partial (AC-7 human reading deferred) | not-needed | 12 / 12 | 6 / 7 (5 user-observable, headless) | 4 / 5 | [06-verify-viewer-match-day.md](06-verify-viewer-match-day.md) |
| `viewer-lineup-tactics` | pass | converged (1 round; fixes `d47e7d3` and `8a5a182`) | 10 / 10 | 6 / 6 (5 user-observable, headless) | 6 / 6 | [06-verify-viewer-lineup-tactics.md](06-verify-viewer-lineup-tactics.md) |
| `viewer-reports-recovery` | pass | not-needed | 10 / 10 | 6 / 6 (5 user-observable, headless) | 8 / 8 | [06-verify-viewer-reports-recovery.md](06-verify-viewer-reports-recovery.md) |
| `integration` | pass | converged (1 round, fix `f6f36b6`) | 12 / 12 | 6 / 6 (4 user-observable: 1 live, 3 headless) | 4 / 4 | [06-verify-integration.md](06-verify-integration.md) |
| `extra-time-penalties` | pass | not-needed | 12 / 12 | 3 / 3 (0 user-observable) | not applicable | [06-verify-extra-time-penalties.md](06-verify-extra-time-penalties.md) |
| `experiment-flags` | pass | not-needed | 13 / 13 | 2 / 2 (0 user-observable) | not applicable | [06-verify-experiment-flags.md](06-verify-experiment-flags.md) |
| `scripting-runtime` | pass | not-needed | 12 / 12 | 4 / 4 (1 user-observable, live) | 1 / 1 | [06-verify-scripting-runtime.md](06-verify-scripting-runtime.md) |
| `distribution` | partial (macOS build deferred) | converged (1 round, fix `89fe959`) | 14 / 14 | 3 / 4 (3 user-observable: 2 emulator-or-container, 1 deferred) | 4 / 4 | [06-verify-distribution.md](06-verify-distribution.md) |
| `probe-engine-core` | pass | not-needed | 7 / 7 | 6 / 6 (6 user-observable, live) | 6 / 6 | [06-verify-probe-engine-core.md](06-verify-probe-engine-core.md) |
| `realism-bands-v2` | pass | converged (1 round; 2 check readings cleared by re-measurement, no code change) | 11 / 11 | 4 / 4 (4 user-observable, live) | 5 / 5 | [06-verify-realism-bands-v2.md](06-verify-realism-bands-v2.md) |
| `defending-and-discipline` | pass | not-needed | 10 / 10 | 3 / 3 kept (3 user-observable, headless); 2 moved to `lone-forward`, not judged | 3 / 3 | [06-verify-defending-and-discipline.md](06-verify-defending-and-discipline.md) |
| `tuning-loop` | pass | converged (1 round, fix `a447ff1`) | 6 / 6 | 7 / 7 (7 user-observable, headless) | 7 / 7 | [06-verify-tuning-loop.md](06-verify-tuning-loop.md) |
| `lone-forward` | pass | not-needed | 7 / 7 | 3 / 3 kept (3 user-observable, headless); red-card criterion moved to `keeper-and-shots`, measured, not judged | 3 / 3 | [06-verify-lone-forward.md](06-verify-lone-forward.md) |
| `keeper-and-shots` | pass | not-needed | 10 / 10 | 4 / 4 kept (4 user-observable, headless); corner floor 1.2 per team (Q-KS2); red-card criterion moved to `realism-tuning`, not judged | 4 / 4 | [06-verify-keeper-and-shots.md](06-verify-keeper-and-shots.md) |
| `tempo-and-restarts` | pass | not-needed | 11 / 11 | 2 / 2 kept (2 user-observable, headless); passes, ball in play and throw-ins moved to `realism-tuning` (Q-TR3), not judged | 2 / 2 | [06-verify-tempo-and-restarts.md](06-verify-tempo-and-restarts.md) |

Evidence quality across verified slices: live 16 / headless 42 / emulator-or-container 2 / n-a 61. Runtime-evidence deferrals: 2 open (`viewer-match-day` AC-7, the product owner's legibility reading on the reference laptop; `distribution` macOS build, an operator with a Mac runs the build and clean-home scripts). Ship blocks until both clear.

## Recommended Next Stage

- `/wf review football-manager-match-engine tempo-and-restarts` (verify `result: pass`, convergence not-needed; a clearance and a restart kick are counted under their own keys and not as passes; the full server gate gives every earlier figure as at `keeper-and-shots`, and the eleven calibrate reports differ only in pass counts, pass accuracy, the new ball-in-play figure and wall time; the red-card and three moved criteria fail as owned by `realism-tuning`; blocked by the steer.md veto until the product owner lifts it)
- `/wf review football-manager-match-engine keeper-and-shots` (verify `result: pass`, convergence not-needed; corners 1.205 per team and goal kicks 11.205 per match over 200 matches with every corner after a defending touch; on target 0.376 to 0.387 and goals per xG 1.084 to 1.111 on five seeds of 1,000 matches; the wide and over scenes give a goal kick with no save; 398 of 500 penalties convert; heavy runs ran on the server; processor time per tick is 3.0% under baseline; goals per match stay under their band until `realism-tuning`)
- `/wf review football-manager-match-engine lone-forward` (verify `result: pass`, convergence not-needed; 4-4-1-1 and 3-4-3 score 3.22 and 2.91 against 4-4-2 under the 4.0 limit; the eight kept pairings hold; discipline gives 0.045 second yellows per match, 11.0% of matches with a sending-off and no same-tick pair; the pressed-forward scene passes 40 of 40 seeds; 489 workspace tests pass; processor time per tick is +4.6% to +5.3% against a +10% tripwire; the red-card slow test still fails and belongs to `keeper-and-shots`)
- `/wf review football-manager-match-engine tuning-loop` (verify `result: pass`, converged in 1 round with fix `a447ff1`, which makes the server log a viewer that leaves during a message send; one targeted pairing at 1,000 matches with its diff takes 88.6 s, the equal suite 91.0 s, the red-card experiment 48.1 s; a fresh seed-7 full run matches the targeted pairing, equal and strength suites exactly; four wrong baselines exit 1 before any match; the red-card suite equals the slow test on 9 of 9 figures; neither faster build candidate is faster beyond run-to-run spread; 479 workspace tests pass)
- `/wf review football-manager-match-engine defending-and-discipline` (verify `result: pass`, convergence not-needed; discipline over 200 matches gives 0.085 second yellows per match, 16.5% of matches with a sending-off and no same-tick pair; the acting keeper passes 6 of 6 scenes; the eight kept formation pairings are all at or under 4.0 goals per side; 458 workspace tests pass; processor time per tick is +4.5% against a +10% tripwire; the red-card criterion and the 4-4-1-1 and 3-4-3 pairings are recorded as measured for `lone-forward`)
- `/wf review football-manager-match-engine realism-bands-v2` (verify `result: pass`, converged in 1 round with no code change; a fresh 57,000-match seed-42 run reports all eleven new bands, names each of its 160 failed checks on stderr, exits 2, holds 55 formation pairings of 1,000 matches, and matches the recorded baseline exactly; the seed-7 record carries `stats.throw_ins` and `stats.goal_kicks` and passes its schema)
- `/wf review football-manager-match-engine probe-engine-core` (verify `result: pass`, convergence not-needed; 414 workspace tests pass, three failed `simulate`/`bench` drives each print one schema-valid `outcome: error` record, both injected calibration failures write `error` and no `failure`, and all 20 help checks fit 80 columns)
- `/wf review football-manager-match-engine distribution` (verify `result: partial` only for the deferred macOS build; converged in 1 round with fix `89fe959`, which lets the documented WSL build command find cargo; a clean Windows Sandbox passes 11 of 11 checks, the Linux archive passes 7 of 7 from a fresh home, and `launch --open` opened the page in the default browser on the reference laptop; review may proceed, ship blocks until the deferral clears)
- `/wf review football-manager-match-engine scripting-runtime` (verify `result: pass`, convergence not-needed; 398 workspace tests pass, the loop, import, and network fixtures are aborted or denied and recorded, the sample pack costs 1.038 times a freshly rebuilt parent per match, and the no-pack match is byte-identical to the parent)
- `/wf review football-manager-match-engine experiment-flags` (verify `result: pass`, convergence not-needed; 364 workspace tests pass, a paired run shows both arms side by side on the same seeds with a schema-valid report, a flag without an owner is refused by name, and the default match is byte-identical to its parent)
- `/wf review football-manager-match-engine extra-time-penalties` (verify `result: pass`, convergence not-needed; 341 workspace tests pass, a live seed-6 knockout match ends 0-0 then 3-4 on kicks, the default match is byte-identical to its parent build, and IFAB Laws 3, 7, and 10 were re-read)
- `/wf review football-manager-match-engine integration` (verify converged with `result: pass`; the whole match, the tutorial from a new clone, the benchmark, and the records all have evidence; the fix `f6f36b6` makes a held-back caution for a booked player a second yellow)
- `/wf review football-manager-match-engine viewer-reports-recovery` (verify `result: pass`, convergence not-needed; a real engine kill restarted at the snapshot tick with the same clock and a 1–0 score across a goal)
- `/wf review football-manager-match-engine viewer-lineup-tactics` (verify converged with `result: pass`; the fix added the `seen` command, which bounds a live engine to 500 ticks past the drawn tick)
- `/wf review football-manager-match-engine viewer-match-day` (verify `result: partial` only for the deferred AC-7 human reading; convergence not-needed; review may proceed, ship blocks until the deferral clears)
- `/wf review football-manager-match-engine calibration` (verify converged with `result: pass`; the product owner's dark-path definition is in `3ec2198`)
- `/wf review football-manager-match-engine commentary` (slug-wide ledger; the tactics-and-ai review is also pending; the match-rules review is also pending; the engine-core, data-schemas-generator, stream-protocol, and viewer-pitch reviews are pending on the same ledger)
