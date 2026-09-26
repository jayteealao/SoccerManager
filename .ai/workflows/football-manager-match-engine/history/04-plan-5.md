---
schema: sdlc/v1
type: plan-index
slug: football-manager-match-engine
status: complete
stage-number: 4
created-at: "2026-09-21T21:57:49Z"
updated-at: "2026-09-22T22:16:48Z"
planning-mode: single
slices-planned: 6
slices-total: 16
implementation-order: [engine-core, data-schemas-generator, stream-protocol, viewer-pitch, match-rules, tactics-and-ai, commentary, calibration, viewer-match-day, viewer-lineup-tactics, viewer-reports-recovery, integration]
conflicts-found: 0
revision-count: 5
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
  - rev: 4
    at: "2026-09-22T19:29:33Z"
    trigger: new-slice
    because: "match-rules planned; the referee, the stoppage hook, and the snapshot enter the engine, and the announced match length becomes a maximum"
    changed: "slices-planned 5; summary, five new cross-cutting concerns, integration points, order, and conflicts updated"
  - rev: 5
    at: "2026-09-22T22:16:48Z"
    trigger: new-slice
    because: "calibration planned ahead of tactics-and-ai; the record schema files, the run report, and the calibrate command enter the plan"
    changed: "calibration added to slices-planned, refs.plans, and the slice summaries"
tags: [engine, rust, 2d-viewer]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  plans: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-calibration.md]
  contract: 02c-craft.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine match-rules"
---

# Plan Index

## Slice Plan Summaries

- **engine-core** (`04-plan-engine-core.md`, implemented and verified): 34 new files across the workspace root, `crates/engine`, and `crates/engine-cli`; strategy: build the library bottom-up with determinism as a design constraint, measure the benchmark at step 15; key risk: the 2-second budget at 50 decisions per second, retired at 393 ms.
- **data-schemas-generator** (`04-plan-data-schemas-generator.md`): 44 files (21 new, 23 modified) across `content/`, `crates/engine/src/data`, the engine seams, `observe`, tests, and the command line; strategy: one generic fail-closed loader with `garde` rules, attributes as a fixed array with derived values computed at load, the generator as the single source of the shipped default teams, identity in `stats.json` and the tick header; key risk: determinism once attributes vary per player.

- **stream-protocol** (`04-plan-stream-protocol.md`): 38 files (26 new, 12 modified) across two new crates, the engine seams, the content folder, the command line, and `docs/reference/`; strategy: a blocking WebSocket server with one thread per client, a codec crate free of input and output, a bounded channel of 500 ticks as the backpressure, and a fixture that stores the wire bytes so a replay is byte-identical by construction; key risk: the encoder on the hot path at 270,000 frames per match.

- **viewer-pitch** (`04-plan-viewer-pitch.md`): 46 files (30 new, 16 modified) across a new `web/` folder, the protocol and stream crates, the command line, and the project documentation; strategy: the engine binary serves the page over HTTP because a browser blocks module scripts over `file://`, the page measures its own lag because no wire message announces it and the browser exposes no inbound-queue depth, the whole match decodes into absolute `Int16Array` arrays at 24.2 MB for an O(1) rewind, and rendering stays on the main thread because the draw is sub-millisecond against a 16.6-millisecond budget; key risk: a high-refresh display making a frame count report 144 and pass for the wrong reason.

- **match-rules** (`04-plan-match-rules.md`): 54 files (16 new, 38 modified) across a new `crates/engine/src/rules` module, the loop, the snapshot, the protocol, the command line, three page files, and the content folder; strategy: pure law functions first, then one referee state machine inside the loop, a stoppage hook (`TickSink::on_stoppage`) that applies no queued change yet, a binary snapshot with a SHA-256 trailer that resumes tick for tick, and an announced maximum match length; key risk: a snapshot that misses one field and lets a resumed match drift.

- **calibration** (`04-plan-calibration.md`, planned before `tactics-and-ai` is planned or built): 32 files (12 new, 20 modified) across `schemas/observability/`, `content/`, the engine counters and snapshot, `observe`, a new `crates/engine-cli/src/calibrate` and `report` pair, and the tests. Strategy: shot, pass, possession, and xG counters at the two existing resolution points; the contract's statistics keys added to `match-stats` at schema version 1; three JSON Schema files that the default tests check real output against with `boon` 0.6 (test-only); `engine-cli calibrate`, which runs an equal-strength suite and a 15 percent strength suite of 1000 matches each in worker processes and writes one `run-report` with the band checks, the dark paths, and the single-thread figure; and a capped tuning pass on `content/tuning.json` against the bands in `content/realism-bands.json`. Key risk: the bands may be unreachable with the tactics slice's decision model. A miss becomes an open tuning item, and the bands are never widened. Step 1 re-reads the tactics seams and stops for a plan re-review if they differ. Shared-file notes: `observe/mod.rs`, `sim.rs`, `snapshot.rs`, `stream_run.rs`, and `content/tuning.json` are also touched by earlier plans, always in sequence. Every new `Summary` field enters the snapshot in the same step.

Seven buildable slices and four deferred slices are not yet planned.

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
- Match length, fixed by the fifth plan: `ticks_expected` in the opening message and `expected_ticks` in the tick-file header mean "at most" (regulation plus the rule pack's added-time cap for both halves, 360,000 ticks for a full match). `PROTOCOL_VERSION` is 2 and the tick-file schema is 4 because of that change of meaning. The real count rides in the trailer and the statistics record. A shortened match plays no added time.
- Stoppages, fixed by the fifth plan: `TickSink::on_stoppage(&Stoppage, &Simulation)` is the one hook where a stoppage becomes visible outside the engine. Snapshots are written through it, and `tactics-and-ai` applies queued changes through it. No later slice adds a second stoppage path.
- Snapshots, fixed by the fifth plan: one binary `.smsn` file per match at `SM_DATA_DIR/matches/<match.id>/snapshot.smsn`, replaced atomically at every stoppage, with a SHA-256 trailer. A snapshot resumes tick for tick on the build and content that wrote it and is refused with a named reason anywhere else. Any field added to `Simulation` by a later slice must be added to the snapshot in the same step, or the continuation test fails.
- Law events, fixed by the fifth plan: the engine supplies `minute` and `minute.added`; `MatchEvent` carries `player.id`, `player.secondary_id`, `card.kind`, `foul.advantage`, and `added_time.s` as optional fields. `card.kind` and `foul.advantage` are additive extras until the observability audit settles them.
- Test scenes, fixed by the fifth plan: the `scenario` Cargo feature on `crates/engine` builds a match already in progress with scripted draws. Only tests enable it; release builds never contain it. Later engine slices reuse it rather than adding a second test seam.
- Output boundary: seven code comments that named workflow slices are rewritten in product language by the fifth plan's steps, and its last step searches the source for workflow vocabulary before the commit. Later plans keep that search.

## Integration Points Between Slices

- `TickSink` in `crates/engine/src/sim.rs` is the seam `stream-protocol` wraps for the socket feed and the recorded fixture.
- `MatchConfig::new(seed, minutes, &Content, [&TeamFile; 2])` replaces the built-in teams; `stream-protocol`, `match-rules`, and `calibration` build configs through it.
- `Content` (`crates/engine/src/data/mod.rs`) is the one loader: `RulePack` feeds `match-rules`; `FatigueTuning` and `Position` feed `tactics-and-ai`; `generate_league` feeds `calibration`.
- The `.ticks` header (schema 2) carries identity in the reserved bytes; the 192-byte record layout is provisional and `stream-protocol` decides the wire encoding (U-1).
- `EngineError` is the public error contract later crates match on; `Data` and `Version` are its content variants.
- `observe::identity` is the one source of `owner.id` and `MatchId` for the viewer slices; the hello message reuses it rather than minting an identifier.
- `crates/protocol` is the contract the viewer slices read: `MESSAGES` enumerates every message, and `docs/reference/protocol.md` is tested against it, so a viewer built on the document cannot drift from the code.
- The `.smfx` fixture and `engine-cli replay` are the prerequisite harness the shape's force-scope rule named: `viewer-pitch`, `viewer-match-day`, and `viewer-reports-recovery` verify against them before the engine is complete.
- `protocol::Queue` holds pending changes with their identifiers; `tactics-and-ai` decides when a change applies, through the stoppage hook `match-rules` opens (corrected by the fifth plan, Round 1 Q1).
- The `match-event` record kind opens here with kick-off, goal, and full-time; `match-rules` adds nine event types and six optional fields, and `commentary` reads them.
- `hello.teams[]` is the one place the page learns a club's kit colours, from the fourth plan onward. `viewer-match-day` reuses the same two fields for the score bug and the club crests rather than reading team files.
- `web/tokens.css`, `web/components/match-control.css`, and `web/mark.mjs` are the three files every later viewer slice builds on. The fourth plan defines all four control sizes, five states, and both themes even though it uses two sizes, so no later slice adds geometry.
- `window.__touchline` is the page's observability seam: `lastRendered()`, `signals`, `history`, and `frame`. Three of this slice's acceptance criteria read it, and every later viewer slice extends it rather than adding a second hook.
- `web/interpolate.mjs` and `web/schedule.mjs` are where commitment C2 lives in the frontend. Every later viewer slice renders through the same scheduler; none may draw a position the engine did not compute.
- `engine-cli replay --sustain <x>` is a test harness for the lag notice and sits on `replay` alone. No later slice promotes it to `serve`.
- `TickSink::on_stoppage` and `Stoppage { tick, kind, team, spot }` are where `tactics-and-ai` drains `protocol::Queue`. The fifth plan emits `ChangeState::Queued` and `Rejected` only, as before; `AppliesNow` and `Applied` wait for that slice.
- The law event types and `player.id` are what `commentary` reads. `viewer-match-day` reads `card.kind`, `added_time.s`, and `minute.added`, and hides the parked markers of sent-off players, which the tick record keeps at fixed spots beside the pitch.
- `snapshot.smsn` and `engine-cli resume` are what `viewer-reports-recovery` builds crash recovery on. `MatchClock` in `crates/engine/src/rules/clock.rs` is what `extra-time-penalties` extends.
- `web/stoppages.mjs` exports `STOPS_PLAY`, the list of event types that stop play. Any slice that adds an event type decides whether it joins the list.

## Recommended Implementation Order

1. `engine-core` — done.
2. `data-schemas-generator` — done.
3. `stream-protocol` — done.
4. `viewer-pitch` — done; the visible milestone.
5. `match-rules` — planned; the referee and the snapshot. It changes three `viewer-pitch` files after that slice is verified.
6. `tactics-and-ai`, `commentary`, `calibration` — the rest of the engine track in dependency order; `tactics-and-ai` applies the held change queue through the stoppage hook.
7. `viewer-match-day`, `viewer-lineup-tactics`, `viewer-reports-recovery` — the viewer track.
8. `integration` — the full charter scenario.

## Conflicts Found

None. The fourth plan touches three files the third plan created — `crates/protocol/src/message.rs`, `crates/stream/src/server.rs`, and `crates/stream/src/replay.rs` — in sequence, after that slice is complete and verified, never in parallel. Two notes for implement. First, the two kit fields added to `hello` leave `PROTOCOL_VERSION` at 1, because no field changes meaning and both producers move in the same commit; the judgement is recorded in a comment beside the constant so a later reviewer sees it was made rather than missed. Second, the recorded fixture must be rebuilt once the kit fields exist, because the replayer now forwards the stored opening message verbatim; a stale fixture would draw grey markers with no error anywhere, and the byte-identity test is what turns that into a loud failure.

The fifth plan touches files four earlier plans created, always in sequence and after each slice is verified: `sim.rs`, `record.rs`, `validate.rs`, and `observe/mod.rs` from the engine slices; `crates/protocol/src/event.rs`, `lib.rs`, `command.rs`, and `message.rs` from the stream slice; and `web/stoppages.mjs`, `web/main.mjs`, and `web/tests/stoppages.test.mjs` from the viewer slice. One stale statement is corrected rather than followed: the comment at `crates/protocol/src/command.rs:1-3` and the earlier integration note said this slice applies the change queue, while `03-slice-match-rules.md` gives substitution windows to `tactics-and-ai`. The product owner kept the slice boundary (plan Round 1 Q1), so this slice opens the stoppage hook and applies nothing. Three shared-file notes for implement: `content/tuning.json` and `content/rules/default.json` both gain fields and their pinning tests in `crates/engine/tests/content.rs` change in the same step; the `.smfx` fixture and every `.ticks` file from before this slice are refused by version, so the local fixture is regenerated; and the page's history reservation rises to about 33.8 MB because the announced maximum is 360,000 ticks.

The three engine plans touch `crates/engine/src/record.rs` in sequence, never in parallel: engine-core wrote it, data-schemas-generator extended the header to schema 2, and stream-protocol adds a restart flag to the frame and a fan-out sink while leaving the 192-byte file record untouched, so the determinism and validator tests keep their meaning. Two shared-file notes for implement: `content/tuning.json` gains a fourth block and its pinning test must be updated in the same step, and `crates/engine-cli/src/cli.rs` gains three subcommands under the same 80-column help rule the second plan introduced.

## Freshness Research

See `04-plan-engine-core.md` § Freshness Research for `rand` 0.10, `HashMap` ordering, platform transcendental functions, and workspace inheritance; `04-plan-data-schemas-generator.md` § Freshness Research for `garde` 0.23, `serde` error behaviour, `deny_unknown_fields` with `flatten`, RUSTSEC-2026-0097, and the schema-version pattern; and `04-plan-stream-protocol.md` § Freshness Research for browser reachability, `tungstenite` 0.30 and its unbounded default write buffer, RFC 6455 masking and the Origin header, `sync_channel` semantics, and `DataView` decode cost. `04-plan-viewer-pitch.md` § Freshness Research carries the browser side: module scripts blocked over `file://`, loopback WebSocket exempt from Local Network Access, `bufferedAmount` measuring only the outgoing queue, `DataView` parity since V8 6.9, `OffscreenCanvas` available and not needed, `requestAnimationFrame` following the panel refresh rate, the cross-origin-isolation requirement on the memory measurement, Node 22's built-in test runner and `.mjs` resolution, and the SIL Open Font License on both typefaces. `04-plan-match-rules.md` § Freshness Research carries the laws: IFAB Laws 3, 7, and 11 to 17, 2024-25 foul and corner rates, the `rand_chacha` 0.10 word-position methods read from the installed source, `std::fs::rename` replace semantics on Windows, and the `serde_json` float-parsing caveat that the binary snapshot avoids.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine match-rules` — the referee slice is planned and both augmentation artifacts are re-authored for it.
- **Option B:** `/wf review football-manager-match-engine viewer-pitch` — four verified slices wait for the slug-wide review ledger; reviewing first keeps the viewer diff apart from the three viewer files this slice changes.
- **Option C:** `/wf plan football-manager-match-engine tactics-and-ai` — plan the next engine slice before implementing; not recommended, because its plan depends on the stoppage hook this slice lands.
