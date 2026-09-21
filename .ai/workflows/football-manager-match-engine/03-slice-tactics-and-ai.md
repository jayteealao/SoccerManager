---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: tactics-and-ai
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: l
depends-on: [match-rules, data-schemas-generator]
tags: [engine, tactics, ai-manager, fatigue, substitutions]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-match-rules.md, 03-slice-data-schemas-generator.md, 03-slice-calibration.md, 03-slice-viewer-lineup-tactics.md, 03-slice-scripting-runtime.md]
  plan: 04-plan-tactics-and-ai.md
  implement: 05-implement-tactics-and-ai.md
---

# Slice: Tactics, Fatigue, and the AI Manager

## The Slice

Match rules deliver a lawful game with stoppages and a rule pack that names which stoppages admit which change. The shape's Tier 2 tactics model, the stoppage-gated change queue, fatigue, injuries, substitution limits, and the reactive AI manager are still missing, and they are the substance of commitments C1, C3, and C5.

This slice adds the tactics model (formation, mentality, 6 to 8 team instructions, per-player roles and duties), a decision layer that reads attributes and roles, fatigue curves and injuries from the tuning file, the change queue driven by the rule pack, the 5-in-3-windows substitution limit, and the AI manager module that manages either team.

Calibration measures the result next. The top risk is that decisions become a tangle of special cases; the decision layer is a scored-options mechanism, named below, so every choice is a weighted comparison the tuning file can move.

## Goal

Attributes and tactics change what happens; managers change tactics and substitute at stoppages; the computer manages its own team.

## Why This Slice Exists

Without this slice the engine is a physics toy. C1, C3, and C5 are proven here, and the AI manager is what makes headless calibration meaningful.

## Scope

In:
- Tactics model: formation, mentality, team instructions (pressing, width, tempo, line height, passing directness, time wasting), roles and duties per player; data model leaves room for Tier 3.
- Decision layer: each agent scores candidate actions (pass to N teammates, dribble, shoot, clear, hold) by attributes, role, and instructions, and picks the highest with RNG noise (named mechanism: **scored-options decision layer**, replaces hand-written branching so tuning moves behavior).
- Fatigue curves and their effect on attributes; injuries with a rate from the tuning file.
- Stoppage-gated change queue driven by the rule pack (named mechanism per the shape): tactics at any dead ball, substitutions at any dead ball within the limit; rejections with reasons; conflict rule (substitution wins over a role change for the same player).
- Substitution limit: 5 changes in 3 windows plus half-time.
- AI manager module (named mechanism per the shape): pre-match setup, mentality change when trailing late, substitutions for fatigue and injury.
- Benchmark rerun with the tripwire.

Out:
- Calibration of realism bands: `calibration`.
- The tactics panel UI: `viewer-lineup-tactics`.
- Scripted decisions: `scripting-runtime` (deferred).

## Acceptance Criteria

- Given a queued tactics change, When the next dead ball occurs, Then the change applies at that tick and an event records it; and Given the same queued change with no dead ball, Then it has not applied.
  <!-- observable: false — cargo test injects a change at a known tick -->
- Given five substitutions used across three windows, When a sixth is queued, Then it is rejected with a reason event; and Given four used, When a fifth is queued, Then it applies at the next dead ball.
  <!-- observable: false — cargo test -->
- Given a substitution and a role change queued for the same player, Then the substitution applies and the role change is rejected with a reason.
  <!-- observable: false — cargo test -->
- Given a player's fatigue passes the tuning threshold, Then that player's pace and decision attributes degrade per the curve.
  <!-- observable: false — cargo test reads effective attributes -->
- Given an injury event, Then the injured player leaves play, and the AI manager queues a substitution that applies within the next window when one is available.
  <!-- observable: false — cargo test -->
- Given team A with attributes 15 percent higher than team B, When 200 seeded matches run headless, Then team A wins more than 50 percent (a coarse pre-calibration check; the 1000-match band is in `calibration`).
  <!-- observable: false — cargo test, marked slow -->
- Given the AI manager's team trails after minute 70, Then a tactical-change event appears before full time in at least 90 percent of 100 headless runs.
  <!-- observable: false — cargo test, marked slow -->
- Given a mentality set to defensive versus attacking with identical squads, When 200 matches run each, Then shots per match differ in the expected direction.
  <!-- observable: false — cargo test, marked slow -->
- Given the benchmark reruns after this slice, Then CPU time per match is within 10 percent and memory within 25 percent of the match-rules baseline.
  <!-- observable: true — developer-visible benchmark report -->
  verify: { method: cargo bench harness compare, env: reference laptop, fixture: seed 42, rung: cli-direct }

## Dependencies on Other Slices

- `match-rules`: stoppages, events, snapshots.
- `data-schemas-generator`: attribute schema, tuning file, rule pack, generated teams.

## Risks

- Decision scoring produces passive or chaotic play: calibration tunes weights; the experiment-flags slice later compares models.
- Benchmark regression from scoring many candidates per agent per tick: limit candidate sets and decision frequency (decide every N ticks, steer every tick).
