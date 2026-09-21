---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: calibration
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: m
depends-on: [tactics-and-ai, data-schemas-generator]
tags: [engine, calibration, statistics, observability]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-tactics-and-ai.md, 03-slice-data-schemas-generator.md, 03-slice-experiment-flags.md, 03-slice-integration.md]
  plan: 04-plan-calibration.md
  implement: 05-implement-calibration.md
---

# Slice: Calibration Harness and Statistics Record

## The Slice

Tactics and the AI manager make the engine play a game worth measuring. The shape's realism bands (goals, shots, possession, strength advantage) and the aggregate statistics record for the observability pipeline are untested until many matches run.

This slice builds the 1000-match calibration harness, the per-match statistics record and the event stream file output (the instrument augmentation's signals), the aggregate report, and one tuning pass that lands the realism bands. The record schema is agreed with the observability plan (U-2).

Integration consumes the tuned engine. The top risk is the tuning loop taking longer than the slice: the acceptance criteria accept the bands, and a miss records the residual as an open tuning item rather than widening the bands.

## Goal

Realism statistics inside the shape's bands over 1000 matches, and every match emitting the records the pipeline needs.

## Why This Slice Exists

C3 and C4 are measured here; the product owner named the observability pipeline as the measure of success (intake Q3).

## Scope

In:
- `engine-cli calibrate --matches 1000 --seed N`: runs matches with the AI manager on both sides over generated leagues, in parallel processes, and writes an aggregate report.
- Per-match statistics record and event stream file output, schema shared with `.ai/observability.md` (named mechanism per the shape: **event stream and statistics record**).
- Tuning pass on the tuning file until the bands hold.
- Dark-path counters: queued change that never applied, match without a statistics record.

Out:
- Feature flags for competing models: `experiment-flags` (deferred).
- The pipeline itself: `/wf observability init` and build.

## Acceptance Criteria

- Given 1000 matches between generated teams of equal strength, Then goals per match lie in 2.4 to 3.2, shots per team in 8 to 16, and possession for either side in 35 to 65 percent.
  <!-- observable: false — the harness asserts the bands; marked slow -->
- Given team A with attributes 15 percent higher than team B, When 1000 matches run, Then team A wins more than 50 percent.
  <!-- observable: false — harness assertion; marked slow -->
- Given any match, Then the engine writes one statistics record and one event stream file that validate against the agreed schema without transformation.
  <!-- observable: false — cargo test validates against the schema file -->
- Given 1000 matches, Then the dark-path counters read zero.
  <!-- observable: false — harness assertion -->
- Given 1000 matches on the reference laptop, Then wall time is under 30 minutes, and the report also records the single-thread per-match time from the benchmark harness.
  <!-- observable: true — the developer reads the report -->
  verify: { method: harness report, env: reference laptop, fixture: seed 2026 generated league, rung: cli-direct }

## Dependencies on Other Slices

- `tactics-and-ai`: decisions, fatigue, AI manager.
- `data-schemas-generator`: generated teams and the tuning file.

## Risks

- Bands unreachable with the current decision model: record the residual and route to `experiment-flags`; never widen the bands silently (they are PO-accepted criteria).
- Schema disagreement with the observability plan: run `/wf observability init` before this slice's plan.
