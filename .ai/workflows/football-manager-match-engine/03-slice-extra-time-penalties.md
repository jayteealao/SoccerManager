---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: extra-time-penalties
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: s
depends-on: [match-rules, tactics-and-ai]
tags: [engine, rules, deferred]
deferred: true
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-match-rules.md, 03-slice-tactics-and-ai.md]
  plan: 04-plan-extra-time-penalties.md
  implement: 05-implement-extra-time-penalties.md
---

# Slice: Extra Time and Penalty Shoot-outs

## The Slice

Shape Q21 selected extra time and penalties; slice Q4 deferred them past the first release. The rules slice ships 90 minutes plus stoppage time.

This slice adds extra time for knockout fixtures, the extra-time substitution allowance from the rule pack, fatigue curves extended to 120 minutes, and the penalty shoot-out. It restores AC-12 from the shape.

It ships after integration. The top risk is fatigue behavior beyond 90 minutes, which the curves must cover without a discontinuity.

## Goal

A level knockout fixture continues into extra time and, if still level, a shoot-out decides it.

## Why This Slice Exists

The product owner selected the rule (Q21) and chose to defer it (slice Q4); it stays a named slice so the narrowing is visible.

## Scope

In: extra-time periods, extra-time substitution allowance, fatigue to 120 minutes, penalty shoot-out with kicker order, extra-time report fields.
Out: replay of shoot-outs in the viewer beyond the normal event feed.

## Acceptance Criteria

- Given a knockout fixture level at full time, Then two extra-time periods run, and if still level a shoot-out runs and produces a winner.
  <!-- observable: false — cargo test with a forced draw -->
- Given extra time, Then the extra-time substitution allowance from the rule pack is available and enforced.
  <!-- observable: false — cargo test -->
- Given a player at minute 105, Then fatigue follows the extended curve with no discontinuity at minute 90.
  <!-- observable: false — cargo test over the curve -->

## Dependencies on Other Slices

- `match-rules`: periods and restarts.
- `tactics-and-ai`: fatigue and substitution windows.

## Risks

- Shoot-out attributes (composure, goalkeeping) are thin in the default set: extend the schema through its version.
