---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: experiment-flags
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: s
depends-on: [calibration]
tags: [engine, calibration, feature-flags, deferred]
deferred: true
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-calibration.md, 03-slice-tactics-and-ai.md]
  plan: 04-plan-experiment-flags.md
  implement: 05-implement-experiment-flags.md
---

# Slice: Feature Flags for Calibration Experiments

## The Slice

Shape Q24 selected the experiment augmentation; slice Q4 deferred it until a second competing model exists. Calibration ships one decision model and one shot model.

This slice adds feature flags in the tuning file that switch between competing models, and paired calibration runs that compare realism bands between flag states. It realizes the shape's experiment augmentation (`04c-experiment.md`).

It ships after integration, when a second model is proposed. The top risk is flag sprawl; every flag carries an owner, a hypothesis, and a removal condition.

## Goal

Two models can be compared on the same seeds with the bands as the metric, and a losing model is removed.

## Why This Slice Exists

The product owner selected flags (Q24) and deferred them (slice Q4).

## Scope

In: flag schema in the tuning file, paired `engine-cli calibrate --flag name=on|off` runs, a comparison report, a removal checklist.
Out: runtime flag changes mid-match.

## Acceptance Criteria

- Given a flag defined in the tuning file, When calibration runs paired with the flag on and off on the same seeds, Then the report shows both bands side by side.
  <!-- observable: false — harness test -->
- Given a flag without an owner, hypothesis, or removal condition, When the engine loads the tuning file, Then it refuses naming the flag.
  <!-- observable: false — cargo test -->

## Dependencies on Other Slices

- `calibration`: the harness and the bands.

## Risks

- Flags outlive their experiment: the removal condition is a required field.
