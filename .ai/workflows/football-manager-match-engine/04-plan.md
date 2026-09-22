---
schema: sdlc/v1
type: plan-index
slug: football-manager-match-engine
status: complete
stage-number: 4
created-at: "2026-09-21T21:57:49Z"
updated-at: "2026-09-22T11:28:51Z"
planning-mode: single
slices-planned: 3
slices-total: 16
implementation-order: [engine-core, data-schemas-generator, stream-protocol, viewer-pitch, match-rules, tactics-and-ai, commentary, calibration, viewer-match-day, viewer-lineup-tactics, viewer-reports-recovery, integration]
conflicts-found: 0
revision-count: 2
revisions:
  - rev: 1
    at: "2026-09-22T06:40:12Z"
    trigger: new-slice
    because: "data-schemas-generator planned"
    changed: "slices-planned 2; summary, cross-cutting concerns, integration points, and order updated"
  - rev: 2
    at: "2026-09-22T11:28:51Z"
    trigger: new-slice
    because: "stream-protocol planned; U-1 closed"
    changed: "slices-planned 3; summary, cross-cutting concerns, integration points, order, and conflicts updated"
tags: [engine, rust, 2d-viewer]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  plans: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md]
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine stream-protocol"
---

# Plan Index

## Slice Plan Summaries

- **engine-core** (`04-plan-engine-core.md`, implemented and verified): 34 new files across the workspace root, `crates/engine`, and `crates/engine-cli`; strategy: build the library bottom-up with determinism as a design constraint, measure the benchmark at step 15; key risk: the 2-second budget at 50 decisions per second, retired at 393 ms.
- **data-schemas-generator** (`04-plan-data-schemas-generator.md`): 44 files (21 new, 23 modified) across `content/`, `crates/engine/src/data`, the engine seams, `observe`, tests, and the command line; strategy: one generic fail-closed loader with `garde` rules, attributes as a fixed array with derived values computed at load, the generator as the single source of the shipped default teams, identity in `stats.json` and the tick header; key risk: determinism once attributes vary per player.

- **stream-protocol** (`04-plan-stream-protocol.md`): 38 files (26 new, 12 modified) across two new crates, the engine seams, the content folder, the command line, and `docs/reference/`; strategy: a blocking WebSocket server with one thread per client, a codec crate free of input and output, a bounded channel of 500 ticks as the backpressure, and a fixture that stores the wire bytes so a replay is byte-identical by construction; key risk: the encoder on the hot path at 270,000 frames per match.

Nine buildable slices and four deferred slices are not yet planned.

## Cross-Cutting Concerns

- Conventions fixed by the first plan bind every later slice: edition 2024, clippy and rustfmt as blocking gates, dual MIT OR Apache-2.0 license, thiserror in libraries and anyhow in binaries, `glam` for vectors, no `HashMap` on any simulation path. The second plan adds: rust-version 1.87, `garde` for every content struct, `#[serde(deny_unknown_fields)]` without `#[serde(flatten)]`, JSON for every content file.
- Content: every data file carries an integer `schema_version` checked before deserialization; the constants live in `crates/engine/src/data/*.rs`; `content/README.md` is the modder reference and must change with any field.
- Identity: `owner.id` is created once under `SM_DATA_DIR`; `match.id` is `{seed:016x}-{millis}`; both ride in `stats.json` and the tick-file header (schema 2). Later slices reuse `observe::identity`, never a second source.
- Observability: every record kind follows `.ai/observability.md` plan-version 1; `MatchStats` and `RunReport` in `crates/engine/src/observe/mod.rs` stay the single source of the JSON keys; `content.hash` and `teams` are additive extras until the observability audit settles them.
- Benchmark tripwire: every engine slice after engine-core reruns `engine-cli bench --seed 42 --matches 5 --json` and the criterion bench; verify compares against `05c-benchmark.md`, re-baselined per slice with the prior record under `history/`.
- Design: the visual contract `02c-craft.md` is authored by the `viewer-pitch` plan after a product name replaces the `[TODO]` marker in PRODUCT.md.
- Experiment augmentation: deferred to the `experiment-flags` slice per the shape's Augmentation Plan.
- Probe carry-overs: findings 1, 3, 4, and 5 of `03-slice-probe-engine-core.md` landed in the data-schemas-generator command-line step; finding 2 waits for `/wf observability init`.
- Transport, fixed by the third plan: a browser page can open a WebSocket or an HTTP connection and nothing else, so every engine-to-page path is WebSocket over loopback. `tungstenite` 0.30 blocking, one thread per client, no async runtime anywhere in the workspace. The bind address is `127.0.0.1`, never `0.0.0.0`.
- Wire encoding (closes U-1): binary keyframe every 50 ticks plus signed-byte centimetre deltas between, about 13 MB per match. `crates/protocol` is the single source of the frame layout; the viewer decoder mirrors it, and no second encoder exists.
- Crate boundaries per NFR-8: `crates/engine` stays network-free so a later WebAssembly build stays open (RIM-3); `crates/protocol` carries no input or output; `crates/stream` owns every socket, thread, and file the stream needs.
- Threads: the workspace is single-threaded until this slice. Every thread it starts ends on a channel disconnect, and every test joins with a timeout.

## Integration Points Between Slices

- `TickSink` in `crates/engine/src/sim.rs` is the seam `stream-protocol` wraps for the socket feed and the recorded fixture.
- `MatchConfig::new(seed, minutes, &Content, [&TeamFile; 2])` replaces the built-in teams; `stream-protocol`, `match-rules`, and `calibration` build configs through it.
- `Content` (`crates/engine/src/data/mod.rs`) is the one loader: `RulePack` feeds `match-rules`; `FatigueTuning` and `Position` feed `tactics-and-ai`; `generate_league` feeds `calibration`.
- The `.ticks` header (schema 2) carries identity in the reserved bytes; the 192-byte record layout is provisional and `stream-protocol` decides the wire encoding (U-1).
- `EngineError` is the public error contract later crates match on; `Data` and `Version` are its content variants.
- `observe::identity` is the one source of `owner.id` and `MatchId` for the viewer slices; the hello message reuses it rather than minting an identifier.
- `crates/protocol` is the contract the viewer slices read: `MESSAGES` enumerates every message, and `docs/reference/protocol.md` is tested against it, so a viewer built on the document cannot drift from the code.
- The `.smfx` fixture and `engine-cli replay` are the prerequisite harness the shape's force-scope rule named: `viewer-pitch`, `viewer-match-day`, and `viewer-reports-recovery` verify against them before the engine is complete.
- `protocol::Queue` holds pending changes with their identifiers; `match-rules` takes the queue and decides when a change applies.
- The `match-event` record kind opens here with kick-off, goal, and full-time; `match-rules` and `commentary` add event types without touching the protocol.

## Recommended Implementation Order

1. `engine-core` — done.
2. `data-schemas-generator` — done.
3. `stream-protocol` — planned; it reads the schema-2 tick header and the loaded rule pack, so it follows data-schemas-generator rather than running beside it.
4. `viewer-pitch` — the visible milestone; it consumes the fixture and the replayer this slice ships.
5. `match-rules`, `tactics-and-ai`, `commentary`, `calibration` — the engine track in dependency order; `match-rules` takes over the held change queue.
6. `viewer-match-day`, `viewer-lineup-tactics`, `viewer-reports-recovery` — the viewer track.
7. `integration` — the full charter scenario.

## Conflicts Found

None. The three plans touch `crates/engine/src/record.rs` in sequence, never in parallel: engine-core wrote it, data-schemas-generator extended the header to schema 2, and stream-protocol adds a restart flag to the frame and a fan-out sink while leaving the 192-byte file record untouched, so the determinism and validator tests keep their meaning. Two shared-file notes for implement: `content/tuning.json` gains a fourth block and its pinning test must be updated in the same step, and `crates/engine-cli/src/cli.rs` gains three subcommands under the same 80-column help rule the second plan introduced.

## Freshness Research

See `04-plan-engine-core.md` § Freshness Research for `rand` 0.10, `HashMap` ordering, platform transcendental functions, and workspace inheritance; `04-plan-data-schemas-generator.md` § Freshness Research for `garde` 0.23, `serde` error behaviour, `deny_unknown_fields` with `flatten`, RUSTSEC-2026-0097, and the schema-version pattern; and `04-plan-stream-protocol.md` § Freshness Research for browser reachability, `tungstenite` 0.30 and its unbounded default write buffer, RFC 6455 masking and the Origin header, `sync_channel` semantics, and `DataView` decode cost.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine stream-protocol`.
- **Option B:** `/wf review football-manager-match-engine data-schemas-generator` — two verified slices await the slug-wide review ledger.
- **Option C:** `/wf slice football-manager-match-engine` — only if cohesion issues appear once more plans exist; none exist now.
