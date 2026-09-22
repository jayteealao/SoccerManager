---
schema: sdlc/v1
type: implement-index
slug: football-manager-match-engine
status: in-progress
stage-number: 5
created-at: "2026-09-21T22:35:04Z"
updated-at: "2026-09-22T07:18:04Z"
slices-implemented: 2
slices-total: 16
metric-total-files-changed: 83
metric-total-lines-added: 9917
metric-total-lines-removed: 210
tags: [engine, rust, data]
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  records: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md]
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine data-schemas-generator"
---

# Implement Index

## Cross-Slice Integration Notes

- `engine-core` is the root slice; it is implemented, verified, and committed on `feat/football-manager-match-engine`.
- `data-schemas-generator` is implemented and awaits verify: `MatchConfig::new(seed, minutes, &Content, [&TeamFile; 2])` replaced the built-in teams and `Tuning::default()` on the binary path; the tick-file header is schema 2; `owner.id` and `match.id` ride in `stats.json` and the header.
- `stream-protocol` wraps the `TickSink` trait in `crates/engine/src/record.rs`, reads header schema 2, and adds the restart event the validator's heuristic waits for.
- `match-rules` consumes `RulePack` from `crates/engine/src/data/rules.rs` and replaces the wall bounce in `crates/engine/src/sim.rs`.
- `tactics-and-ai` maps roles onto the ten `Position` codes and reads `FatigueTuning`.
- `calibration` calls `generate_league` for 20-club leagues and tunes `per_position` and the constants in `Tuning`.
- Benchmark: engine-core 398 ms at implement and 393 ms at verify (the baseline); data-schemas-generator measured 390 ms at implement with 5.69 MB peak memory, inside both tripwires.

## Recommended Next Stage

- `/wf verify football-manager-match-engine data-schemas-generator`
