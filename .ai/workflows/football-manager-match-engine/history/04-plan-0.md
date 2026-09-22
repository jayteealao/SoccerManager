---
schema: sdlc/v1
type: plan-index
slug: football-manager-match-engine
status: complete
stage-number: 4
created-at: "2026-09-21T21:57:49Z"
updated-at: "2026-09-21T21:57:49Z"
planning-mode: single
slices-planned: 1
slices-total: 16
implementation-order: [engine-core, data-schemas-generator, stream-protocol, viewer-pitch, match-rules, tactics-and-ai, commentary, calibration, viewer-match-day, viewer-lineup-tactics, viewer-reports-recovery, integration]
conflicts-found: 0
tags: [engine, rust, 2d-viewer]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  plans: [04-plan-engine-core.md]
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine engine-core"
---

# Plan Index

## Slice Plan Summaries

- **engine-core** (`04-plan-engine-core.md`): 34 new files across the workspace root, `crates/engine`, and `crates/engine-cli`; strategy: build the library bottom-up with determinism as a design constraint, measure the benchmark at step 15 before the tests are finalized; key risk: the 2-second budget at 50 decisions per second (a miss reopens the rate with the product owner under NFR-1 yields-to C2).

Eleven buildable slices and four deferred slices are not yet planned.

## Cross-Cutting Concerns

- Conventions fixed by the first plan bind every later slice: edition 2024, clippy and rustfmt as blocking gates, dual MIT OR Apache-2.0 license, thiserror in libraries and anyhow in binaries, `glam` for vectors, no `HashMap` on any simulation path.
- Observability: every record kind follows `.ai/observability.md` plan-version 1; the `MatchStats` and `RunReport` structs in `crates/engine/src/observe/mod.rs` are the single source of the JSON keys.
- Benchmark tripwire: every engine slice after this one reruns `engine-cli bench --seed 42 --matches 5 --json` and the criterion bench; verify compares against `05c-benchmark.md`.
- Design: the visual contract `02c-craft.md` is authored by the `viewer-pitch` plan after a product name replaces the `[TODO]` marker in PRODUCT.md.
- Experiment augmentation: deferred to the `experiment-flags` slice per the shape's Augmentation Plan.

## Integration Points Between Slices

- `TickSink` in `crates/engine/src/sim.rs` is the seam `stream-protocol` wraps for the socket feed and the recorded fixture.
- `Tuning::default()` and the built-in attribute set in `player.rs` are the seams `data-schemas-generator` replaces with validated files.
- The `.ticks` file layout (header, 192-byte records, trailer) is provisional; `stream-protocol` decides the wire encoding (U-1) and may keep or replace it.
- `EngineError` is the public error contract later crates match on.

## Recommended Implementation Order

1. `engine-core` — the root; retires the performance risk.
2. `data-schemas-generator`, `stream-protocol` — both depend only on the root and may run in parallel.
3. `viewer-pitch` — the visible milestone.
4. `match-rules`, `tactics-and-ai`, `commentary`, `calibration` — the engine track in dependency order.
5. `viewer-match-day`, `viewer-lineup-tactics`, `viewer-reports-recovery` — the viewer track.
6. `integration` — the full charter scenario.

## Conflicts Found

None. One plan exists.

## Freshness Research

See `04-plan-engine-core.md` § Freshness Research; the sources on `rand` 0.10, `HashMap` ordering, platform transcendental functions, and workspace inheritance apply to every later Rust slice.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine engine-core`.
- **Option B:** `/wf slice football-manager-match-engine` — only if cohesion issues appear once more plans exist; none exist now.
