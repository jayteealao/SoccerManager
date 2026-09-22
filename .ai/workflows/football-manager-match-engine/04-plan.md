---
schema: sdlc/v1
type: plan-index
slug: football-manager-match-engine
status: complete
stage-number: 4
created-at: "2026-09-21T21:57:49Z"
updated-at: "2026-09-22T14:41:32Z"
planning-mode: single
slices-planned: 4
slices-total: 16
implementation-order: [engine-core, data-schemas-generator, stream-protocol, viewer-pitch, match-rules, tactics-and-ai, commentary, calibration, viewer-match-day, viewer-lineup-tactics, viewer-reports-recovery, integration]
conflicts-found: 0
revision-count: 3
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
  - rev: 3
    at: "2026-09-22T14:41:32Z"
    trigger: new-slice
    because: "viewer-pitch planned; the visual contract 02c-craft.md is authored and the frontend enters the repository"
    changed: "slices-planned 4; summary, four new cross-cutting concerns for the page, integration points, order, and conflicts updated"
tags: [engine, rust, 2d-viewer]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  plans: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md]
  contract: 02c-craft.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine viewer-pitch"
---

# Plan Index

## Slice Plan Summaries

- **engine-core** (`04-plan-engine-core.md`, implemented and verified): 34 new files across the workspace root, `crates/engine`, and `crates/engine-cli`; strategy: build the library bottom-up with determinism as a design constraint, measure the benchmark at step 15; key risk: the 2-second budget at 50 decisions per second, retired at 393 ms.
- **data-schemas-generator** (`04-plan-data-schemas-generator.md`): 44 files (21 new, 23 modified) across `content/`, `crates/engine/src/data`, the engine seams, `observe`, tests, and the command line; strategy: one generic fail-closed loader with `garde` rules, attributes as a fixed array with derived values computed at load, the generator as the single source of the shipped default teams, identity in `stats.json` and the tick header; key risk: determinism once attributes vary per player.

- **stream-protocol** (`04-plan-stream-protocol.md`): 38 files (26 new, 12 modified) across two new crates, the engine seams, the content folder, the command line, and `docs/reference/`; strategy: a blocking WebSocket server with one thread per client, a codec crate free of input and output, a bounded channel of 500 ticks as the backpressure, and a fixture that stores the wire bytes so a replay is byte-identical by construction; key risk: the encoder on the hot path at 270,000 frames per match.

- **viewer-pitch** (`04-plan-viewer-pitch.md`): 46 files (30 new, 16 modified) across a new `web/` folder, the protocol and stream crates, the command line, and the project documentation; strategy: the engine binary serves the page over HTTP because a browser blocks module scripts over `file://`, the page measures its own lag because no wire message announces it and the browser exposes no inbound-queue depth, the whole match decodes into absolute `Int16Array` arrays at 24.2 MB for an O(1) rewind, and rendering stays on the main thread because the draw is sub-millisecond against a 16.6-millisecond budget; key risk: a high-refresh display making a frame count report 144 and pass for the wrong reason.

Eight buildable slices and four deferred slices are not yet planned.

## Cross-Cutting Concerns

- Conventions fixed by the first plan bind every later slice: edition 2024, clippy and rustfmt as blocking gates, dual MIT OR Apache-2.0 license, thiserror in libraries and anyhow in binaries, `glam` for vectors, no `HashMap` on any simulation path. The second plan adds: rust-version 1.87, `garde` for every content struct, `#[serde(deny_unknown_fields)]` without `#[serde(flatten)]`, JSON for every content file.
- Content: every data file carries an integer `schema_version` checked before deserialization; the constants live in `crates/engine/src/data/*.rs`; `content/README.md` is the modder reference and must change with any field.
- Identity: `owner.id` is created once under `SM_DATA_DIR`; `match.id` is `{seed:016x}-{millis}`; both ride in `stats.json` and the tick-file header (schema 2). Later slices reuse `observe::identity`, never a second source.
- Observability: every record kind follows `.ai/observability.md` plan-version 1; `MatchStats` and `RunReport` in `crates/engine/src/observe/mod.rs` stay the single source of the JSON keys; `content.hash` and `teams` are additive extras until the observability audit settles them.
- Benchmark tripwire: every engine slice after engine-core reruns `engine-cli bench --seed 42 --matches 5 --json` and the criterion bench; verify compares against `05c-benchmark.md`, re-baselined per slice with the prior record under `history/`.
- Design, fixed by the fourth plan: `02c-craft.md` is authored and both design gates are closed — `image-gate: pass` on `steer.md`, and the brief-confirm gate on a user-confirmed PRODUCT.md. The product is named Touchline, so the token prefix is `--tl-` and the `--mv-` prefix in `02b-design.md` and `02b-design.yaml` is stale, not in conflict. `web/tokens.css` is the single source of every colour, radius, spacing, font and easing value, and DESIGN.md carries the same rows.
- Frontend conventions, fixed by the fourth plan: the page lives in `web/` at the repository root; modules carry the `.mjs` extension and are run by `node --test` with no `package.json`, so the repository stays a pure Cargo workspace; relative imports always name their extension, which is the one spelling Node and the browser both accept; plain HTML, CSS, and JavaScript with no framework, no component library, and no build step.
- Page delivery, fixed by the fourth plan: the engine binary serves the page. A browser blocks module scripts over `file://` absolutely, so `--web <dir>` on `serve` and `replay` starts a static server beside the socket, on an origin the WebSocket allowlist already accepts. Every response carries `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`, which the page's memory gauge requires and which no later slice may remove without breaking it.
- Page-side observability has no transport. The contract's file sink and socket feed both reach only the engine. Every page signal is a JSON Lines row on `console.info` plus a ring buffer on the page test hook, and `record.kind: viewer-event` is a fifth kind emitted as an additive extra. `/wf observability` owns closing the gap.
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
- `hello.teams[]` is the one place the page learns a club's kit colours, from the fourth plan onward. `viewer-match-day` reuses the same two fields for the score bug and the club crests rather than reading team files.
- `web/tokens.css`, `web/components/match-control.css`, and `web/mark.mjs` are the three files every later viewer slice builds on. The fourth plan defines all four control sizes, five states, and both themes even though it uses two sizes, so no later slice adds geometry.
- `window.__touchline` is the page's observability seam: `lastRendered()`, `signals`, `history`, and `frame`. Three of this slice's acceptance criteria read it, and every later viewer slice extends it rather than adding a second hook.
- `web/interpolate.mjs` and `web/schedule.mjs` are where commitment C2 lives in the frontend. Every later viewer slice renders through the same scheduler; none may draw a position the engine did not compute.
- `engine-cli replay --sustain <x>` is a test harness for the lag notice and sits on `replay` alone. No later slice promotes it to `serve`.

## Recommended Implementation Order

1. `engine-core` — done.
2. `data-schemas-generator` — done.
3. `stream-protocol` — done.
4. `viewer-pitch` — planned; the visible milestone. It consumes the fixture and the replayer the third slice shipped, and changes three of that slice's files rather than adding a crate.
5. `match-rules`, `tactics-and-ai`, `commentary`, `calibration` — the engine track in dependency order; `match-rules` takes over the held change queue.
6. `viewer-match-day`, `viewer-lineup-tactics`, `viewer-reports-recovery` — the viewer track.
7. `integration` — the full charter scenario.

## Conflicts Found

None. The fourth plan touches three files the third plan created — `crates/protocol/src/message.rs`, `crates/stream/src/server.rs`, and `crates/stream/src/replay.rs` — in sequence, after that slice is complete and verified, never in parallel. Two notes for implement. First, the two kit fields added to `hello` leave `PROTOCOL_VERSION` at 1, because no field changes meaning and both producers move in the same commit; the judgement is recorded in a comment beside the constant so a later reviewer sees it was made rather than missed. Second, the recorded fixture must be rebuilt once the kit fields exist, because the replayer now forwards the stored opening message verbatim; a stale fixture would draw grey markers with no error anywhere, and the byte-identity test is what turns that into a loud failure.

The three engine plans touch `crates/engine/src/record.rs` in sequence, never in parallel: engine-core wrote it, data-schemas-generator extended the header to schema 2, and stream-protocol adds a restart flag to the frame and a fan-out sink while leaving the 192-byte file record untouched, so the determinism and validator tests keep their meaning. Two shared-file notes for implement: `content/tuning.json` gains a fourth block and its pinning test must be updated in the same step, and `crates/engine-cli/src/cli.rs` gains three subcommands under the same 80-column help rule the second plan introduced.

## Freshness Research

See `04-plan-engine-core.md` § Freshness Research for `rand` 0.10, `HashMap` ordering, platform transcendental functions, and workspace inheritance; `04-plan-data-schemas-generator.md` § Freshness Research for `garde` 0.23, `serde` error behaviour, `deny_unknown_fields` with `flatten`, RUSTSEC-2026-0097, and the schema-version pattern; and `04-plan-stream-protocol.md` § Freshness Research for browser reachability, `tungstenite` 0.30 and its unbounded default write buffer, RFC 6455 masking and the Origin header, `sync_channel` semantics, and `DataView` decode cost. `04-plan-viewer-pitch.md` § Freshness Research carries the browser side: module scripts blocked over `file://`, loopback WebSocket exempt from Local Network Access, `bufferedAmount` measuring only the outgoing queue, `DataView` parity since V8 6.9, `OffscreenCanvas` available and not needed, `requestAnimationFrame` following the panel refresh rate, the cross-origin-isolation requirement on the memory measurement, Node 22's built-in test runner and `.mjs` resolution, and the SIL Open Font License on both typefaces.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine viewer-pitch` — the visible-milestone slice is planned, its visual contract is written, and both augmentation artifacts are re-authored and ready.
- **Option B:** `/wf review football-manager-match-engine stream-protocol` — three verified slices await the slug-wide review ledger.
- **Option C:** `/wf slice football-manager-match-engine` — only if the static file server and the kit-colour protocol change are judged to belong to `stream-protocol` rather than to `viewer-pitch`; no other cohesion issue exists.
