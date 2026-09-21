---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: engine-core
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: l
depends-on: []
tags: [engine, rust, benchmark]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-data-schemas-generator.md, 03-slice-stream-protocol.md, 03-slice-viewer-pitch.md, 03-slice-match-rules.md, 03-slice-tactics-and-ai.md, 03-slice-commentary.md, 03-slice-calibration.md, 03-slice-viewer-match-day.md, 03-slice-viewer-lineup-tactics.md, 03-slice-viewer-reports-recovery.md, 03-slice-integration.md]
  plan: 04-plan-engine-core.md
  implement: 05-implement-engine-core.md
---

# Slice: Engine Core

## The Slice

The shape fixed a native Rust engine at 50 ticks per second and left one risk open above all others: whether an agent-based engine with ball physics simulates a 90-minute match in under 2 seconds on one thread. The product owner chose to retire that risk first.

This slice builds the Rust workspace, the fixed-timestep tick loop, the pitch and ball model, 22 agents with steering and a minimal possession decision, formation shape, the seedable random-number generator, a headless command line, a tick-stream validator, and the benchmark harness. It ships no laws of the game, no tactics model, and no socket; a built-in default attribute set stands in until the data-schemas slice replaces it.

Every later engine slice builds on this loop, and the stream-protocol slice wraps it. The top risk is the benchmark: a miss reopens the tick rate with the product owner under NFR-1's `yields-to: C2` rule, and never lowers the positional model silently.

## Goal

A headless engine that simulates a plausible kick-about at 50 ticks per second, emits every tick, and measures its own speed.

## Why This Slice Exists

Performance at 50 ticks per second is the largest unproven assumption in the shape (NFR-1). Every other slice consumes the tick loop, so its shape must exist before anything else can be planned in detail.

## Scope

In:
- Cargo workspace with `engine` library and `engine-cli` binary crates.
- Fixed-timestep loop at 20 ms per tick (named mechanism: **fixed-timestep loop**, replaces a variable-step loop so tick count is a function of match time alone).
- Pitch model (105 m by 68 m), ball with drag, friction, and restitution; no spin yet.
- 22 agents with steering behaviors (seek, arrive, separation) and a minimal possession decision: pass to the best-placed teammate or dribble.
- Formation shape as target positions relative to the ball (named mechanism: **formation anchor**, replaces free-roaming agents so AC-6 has something to measure).
- Seedable engine-owned RNG (named mechanism per the shape).
- Headless command line: `engine-cli simulate --seed N --ticks-out file`.
- Tick-stream validator: bounds, overlap, ball speed, formation tolerance.
- Benchmark harness: ticks per second and 90-minute wall time, single thread, with machine name and build hash.
- Built-in default attribute set (pace, acceleration, passing, dribbling, tackling, positioning) as a stopgap.

Out:
- Laws of the game: `match-rules`.
- Attribute schema files, team data, generator: `data-schemas-generator`.
- Tactics, fatigue, injuries, AI manager: `tactics-and-ai`.
- Socket server and wire format: `stream-protocol`.

## Acceptance Criteria

- Given a seed and two built-in teams, When `engine-cli simulate` runs 90 minutes, Then the output holds 270,000 ticks, each with one ball position and 22 player positions.
  <!-- observable: false — a cargo integration test counts ticks and fields in the output file -->
- Given the same seed and build on the same machine, When two simulations run, Then the two tick outputs are byte-identical.
  <!-- observable: false — a cargo test compares the two files -->
- Given any tick, Then no player is outside the pitch bounds, no two players share a point within 0.1 m, and ball speed is at most 40 m/s.
  <!-- observable: false — the validator runs inside a cargo test over a seeded match -->
- Given open play with the ball more than 30 m from a player, Then that player's distance to the formation anchor stays within the tolerance the tuning constants set.
  <!-- observable: false — validator assertion in a cargo test -->
- Given the benchmark harness on the reference laptop with one thread, When it simulates one 90-minute match, Then wall time is under 2 seconds and the report records machine name and build hash.
  <!-- observable: true — the benchmark output is a user-visible report the developer reads; the number is the deliverable -->
  verify: { method: cargo bench harness, env: Windows 11 reference laptop with rustc 1.92 (installed), fixture: seed 42 with built-in teams, rung: cli-direct }
- Given an invalid seed argument, When `engine-cli simulate` runs, Then it exits non-zero with a message naming the argument; and Given a valid seed, Then it exits zero.
  <!-- observable: false — cargo test drives the binary -->

## Dependencies on Other Slices

None. This slice is the root.

## Risks

- The benchmark misses 2 seconds: reopen the tick rate with the product owner (NFR-1 yields to C2); candidate levers are spatial partitioning and fewer perception checks per tick.
- Steering agents oscillate around anchors: tune arrive radius; the validator's formation tolerance catches it.
- Byte-identical output across two runs fails because of hash-map iteration order: use ordered collections in the simulation path.
