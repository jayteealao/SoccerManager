---
schema: sdlc/v1
type: implement-index
slug: football-manager-match-engine
status: in-progress
stage-number: 5
created-at: "2026-09-21T22:35:04Z"
updated-at: "2026-09-21T22:35:04Z"
slices-implemented: 1
slices-total: 16
metric-total-files-changed: 36
metric-total-lines-added: 4022
metric-total-lines-removed: 0
tags: [engine, rust]
refs:
  index: 00-index.md
  plan-index: 04-plan.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine engine-core"
---

# Implement Index

## Cross-Slice Integration Notes

- `engine-core` is the root slice; it is implemented and committed on `feat/football-manager-match-engine`.
- `data-schemas-generator` replaces `Tuning::default()` in `crates/engine/src/tuning.rs` and the built-in `Attributes::uniform(60)` in `crates/engine/src/sim.rs`.
- `stream-protocol` wraps the `TickSink` trait in `crates/engine/src/record.rs` and adds the restart event the validator's heuristic waits for.
- `match-rules` replaces the wall bounce in `crates/engine/src/sim.rs` with throw-ins, corners, goal kicks, and offside.
- `calibration` tunes goals, pass completion, and possession share; the constants live in `Tuning`.
- The benchmark baseline in `05c-benchmark.md` now has a first measurement: 398 milliseconds per 90-minute match.

## Recommended Next Stage

- `/wf verify football-manager-match-engine engine-core`
