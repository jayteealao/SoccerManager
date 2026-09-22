---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: stream-protocol
status: complete
stage-number: 4
created-at: "2026-09-22T11:28:51Z"
updated-at: "2026-09-22T11:28:51Z"
metric-files-to-touch: 38
metric-step-count: 20
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, protocol, socket, fixture, websocket, rust]
stack-source: confirmed
augmentations:
  instrument: 04b-instrument.md
  benchmark: 05c-benchmark.md
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-stream-protocol.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md]
  probe: 03-slice-probe-engine-core.md
  contract: ../../observability.md
  implement: 05-implement-stream-protocol.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine stream-protocol"
---

# Plan: Stream Protocol and Fixture Harness

## The Plan

Two slices handed over an engine that produces 694,087 ticks per second into a file and reads every data file from `content/`, with one seam for a consumer: the `TickSink` trait, called once per tick. Nothing in the workspace speaks to a network, no thread is spawned anywhere, and no test uses a socket, a channel, or a clock. The slice asks for a live stream to a browser page, and research settled the first fact: a page can open a WebSocket or an HTTP connection to a local program and nothing else, so the "TCP on loopback" default named in the slice needs WebSocket framing above it.

Twelve product-owner answers fixed the build. The transport is `tungstenite` 0.30, blocking, one thread per client, because version 1 serves one viewer and `tokio` would add an executor for concurrency the engine does not have. The tick encoding closes unknown U-1: a 98-byte keyframe every 50 ticks and 47-byte delta frames between, about 13 megabytes per match against 51.8 raw and a 300-megabyte budget. Two new crates keep the split NFR-8 asks for: `crates/protocol` holds the messages and the codec with no input or output, so the viewer decoder and a later WebAssembly build can mirror it, and `crates/stream` holds the server, the recorder, and the replayer. The engine simulates as fast as the client reads, up to 500 ticks of lead held in `content/tuning.json`; a full `sync_channel` blocks the simulation thread, which is the backpressure. The fixture records the wire bytes exactly as sent, so the replay is byte-identical by construction.

Implement builds 26 new files across 20 steps, the codec first and the command line last, and retires one debt marker on the way: the event stream names a kick-off, so the validator stops reading every two-metre ball jump as a restart. The top risk is the encoder on the hot path — 270,000 frames per match with no allocation allowed — and step 19 measures it against the 384-millisecond and 5.62-megabyte tripwires before any test is called done.

## Current State

- `crates/engine` is network-free and single-threaded. `Simulation::run` (`crates/engine/src/sim.rs:144-150`) loops `step()` then `sink.on_tick(&self.record())`; `step()` is public, so a serve loop can drive the simulation tick by tick instead of calling `run`. `Simulation` holds only plain data and is `Send` by inference.
- `TickSink` is the one consumer seam (`crates/engine/src/record.rs:131-133`): `fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError>`. Three implementations exist (`NullSink`, `VecSink`, `FileSink`) and no combinator, so a run feeds exactly one sink today.
- `TickRecord` is 192 bytes (`crates/engine/src/record.rs:34-39`): `tick: u32`, `ball: [f32; 3]`, `players: [[f32; 2]; 22]`, packed with `to_le_bytes` into a reused buffer.
- Identity is settled and reusable: `owner.id` and `MatchId` (`crates/engine/src/observe/identity.rs:34-126`), `engine::version()` and `engine::build_hash()` (`crates/engine/src/lib.rs:39-46`), `content_hash` (`crates/engine/src/sim.rs:51-56`). The hello message reuses all five rather than inventing identifiers.
- The tick file header is schema 2 and fails closed on an unknown version, naming both (`crates/engine/src/record.rs:107-112`). This is the pattern the protocol version copies.
- `.ai/observability.md` already assumes this socket exists: it names it at lines 81, 101, 134, 162, 181, and 201, reserves `viewer.socket_drops` at line 65, and defines a `match-event` record kind with `change.queued_tick`, `change.applied_tick`, and `change.rejected_reason` (lines 21-52) that no code has ever written.
- `crates/engine/src/validate.rs:77` carries an `sdlc-debt:` marker that names this slice as its upgrade path: the record has no restart flag, so a ball jump above two metres is read as a kick-off.
- No async runtime exists (`tokio` is absent from `Cargo.lock`); no `std::net`, `std::thread`, `std::sync::mpsc`, `Instant`-pacing, or `sleep` appears in any test. 66 tests pass on commit `d58fce3`.
- The registry holds no WebSocket crate, no SHA-1, and no base64, so the transport is the one dependency this slice fetches.

## Simplicity Ladder

| Capability | Rung | Evidence |
|---|---|---|
| Accept a loopback connection | rung 1 stdlib | `std::net::TcpListener::bind("127.0.0.1:0")`; `local_addr()` returns the assigned port |
| WebSocket handshake and framing | rung 4 new-code | No `tungstenite`, `sha1`, `base64`, or `httparse` in `~/.cargo/registry`; the handshake needs SHA-1, which `sha2` does not provide. `tungstenite` 0.30 is MIT OR Apache-2.0, MSRV 1.85, needs no runtime, and refuses an unmasked client frame by default (`.scratch/sources/rust/tungstenite-0.30.0/src/protocol/frame/mod.rs:214-224`) |
| Binary tick framing | rung 3 reuse | `TickRecord::write_to` (`crates/engine/src/record.rs:43-56`) is the little-endian pattern; the codec quantises to `i16` centimetres and adds the delta |
| Bounded buffer and backpressure | rung 1 stdlib | `std::sync::mpsc::sync_channel(bound)`: `send` blocks when the buffer is full and unblocks as the consumer drains, which is the criterion verbatim |
| Two sinks at once | rung 4 new-code | No fan-out exists; a `FanoutSink<A, B>` forwarding `on_tick` to both is about fifteen lines, cheaper than duplicating the `run` loop |
| Fixture recorder | rung 3 reuse | `FileSink` (`crates/engine/src/record.rs:158-212`) is the header-records-trailer pattern; the fixture stores frames instead of records |
| Replay pacing | rung 1 stdlib | `std::time::Instant` plus `sleep`; no timer crate exists or is needed |
| Control-channel encoding | rung 3 reuse | `serde_json` is installed and already carries every record; MessagePack and CBOR would add a dependency and a second format for a low-rate channel |
| Event records | rung 3 reuse | `observe::to_json` and the `Record` trait (`crates/engine/src/observe/mod.rs:45-49,148-165`) already build the envelope every record kind shares |

## Applied Learnings

- `.ai/solutions/INDEX.md` does not exist. No applicable learnings found.
- Carry-over from `05-implement-engine-core.md`: the `sdlc-debt:` marker at `crates/engine/src/validate.rs:77` names this slice. Step 12 removes the shortcut and the marker together.
- Carry-over from `03-slice-probe-engine-core.md` finding 2: a failed run leaves no structured record. This slice adds failure paths of the same shape (a refused handshake, a dropped client), and `socket.refused` plus the `full-time` event cover them on the socket side; the `outcome: error` record stays with `/wf observability init` (U-2).
- Carry-over from `06-verify-data-schemas-generator.md`: two runs of one seed now differ in header bytes 44 to 51 because the header carries `match.id`. The fixture header repeats the pattern, so the byte-identity test compares frame bytes, never the whole file.
- Repeat-deferral tripwire: `00-index.md` `runtime-evidence-deferrals` is empty. No wall is inherited and none is named by this slice; every criterion is `observable: false`.

## Likely Files / Areas to Touch

- `crates/protocol/` (new): `lib.rs`, `message.rs`, `codec.rs`, `event.rs`, `command.rs`, `frame.rs`, `tests/document.rs`. Messages, the codec, the version, and the enumeration the document test reads. No input or output.
- `crates/stream/` (new): `lib.rs`, `server.rs`, `session.rs`, `control.rs`, `record.rs`, `replay.rs`, `events.rs`, and five test files plus `tests/common/mod.rs`. The server, the bounded buffer, the recorder, and the replayer.
- `crates/engine/src/sim.rs`: `take_events()` for the three events the engine can produce.
- `crates/engine/src/record.rs`: the restart flag on the frame and `FanoutSink`.
- `crates/engine/src/validate.rs`: read the restart flag; delete the debt marker.
- `crates/engine/src/data/tuning.rs` and `content/tuning.json`: the `stream` block with `buffer_ticks` and `keyframe_interval`.
- `crates/engine-cli/`: `cli.rs`, `main.rs`, `serve.rs` (new), `record.rs` (new), `replay.rs` (new), `bench.rs`, `tests/cli_args.rs`.
- `docs/reference/protocol.md` (new), `content/README.md`, `README.md`, workspace `Cargo.toml`.

## Proposed Change Strategy

Build from the inside out and prove each layer before the next uses it: the codec with no network, then the messages, then the server, then the recorder and the replayer, then the command line. `crates/protocol` never depends on `crates/stream`, on `tungstenite`, or on any input or output, which is what lets the document test and a later WebAssembly build use it.

NFR-2 governs the encoding choice and `yields-to: C2` (PO-ratified): the delta scheme exists to hold 60 frames per second at 8x playback with a whole-match history under 300 megabytes, and it never invents a position — a delta is an exact centimetre step between two computed ticks, and the decoder reconstructs absolute positions. NFR-8 governs the crate split. NFR-9 keeps the target at Windows 11; the code uses `std::net` only, so no platform branch is needed.

The backpressure has two guards because the socket has two buffers: the `sync_channel` bound (500 ticks, from the tuning file) and `tungstenite`'s `max_write_buffer_size`, which defaults to unbounded and must be set. Both must say "pause", or the criterion passes in one buffer and fails in the other.

## Step-by-Step Plan

1. **Workspace and dependency.** Add `crates/protocol` and `crates/stream` to `members`; add `tungstenite = { version = "0.30", default-features = false, features = ["handshake"] }` to `workspace.dependencies`. Confirm the fetched tree is `bytes`, `log`, `rand`, `thiserror`, `data-encoding`, `http`, `httparse`, `sha1`, each MIT or Apache-2.0 (NFR-5). No transport-layer-security feature.
2. **Protocol crate skeleton.** `crates/protocol/Cargo.toml` (serde, serde_json, thiserror only) and `lib.rs` with `PROTOCOL_VERSION: u16 = 1`, `KEYFRAME_INTERVAL` re-exported from the codec, and `ProtocolError` (thiserror; explicit `#[source]`, never `#[from]` on a type whose display repeats its source, per the probe finding 3 fix).
3. **Tick codec.** `codec.rs`: `Quantised { tick: u32, ball: [i16; 3], players: [[i16; 2]; 22] }` in centimetres; `encode_keyframe_into(&Quantised, &mut [u8; 98])`, `encode_delta_into(&Quantised, &Quantised, &mut [u8; 47]) -> Option<()>` returning `None` when any step exceeds `i8`, and the matching decoders. Unit tests: a round trip; a delta at the 40 m/s ball cap (80 cm per tick) encodes; a two-metre jump forces a keyframe; a full 50-tick cycle reconstructs every absolute position within one centimetre.
4. **Frames.** `frame.rs`: `Frame::Binary(Vec<u8>)` and `Frame::Text(String)` with a one-byte kind tag, the single place that decides ticks are binary frames and control messages are JSON text frames.
5. **Messages.** `message.rs`: `ServerMessage { Hello, Tick, Event, Stats, Ack, Reject }` and `ClientCommand { Start, Pause, SetSpeed, QueueChange }` as serde enums with `deny_unknown_fields` and dotted rename keys. `Hello` carries `protocol.version`, `engine.version`, `build.hash`, `owner.id`, `match.id`, `seed`, `dt_ms`, `ticks_expected`, `keyframe_interval`, and both `team.id` and `team.name` values. Unit test: every message round-trips through JSON and an unknown field is refused.
6. **Message enumeration.** `MESSAGES: &[MessageSpec { name, direction, fields }]` in `lib.rs`, filled from the two enums by hand and asserted complete by a unit test that matches on every variant, so a new message cannot be added without a compile error in the test.
7. **Events.** `event.rs`: `MatchEvent` with the contract keys (`tick`, `minute`, `event.type`, `team.id`, `home.score`, `away.score`) and `EventType { KickOff, Goal, FullTime }`; a `Record` implementation so the envelope matches `match-stats`. Unit test: a goal event serialises with `record.kind` `match-event` and both scores.
8. **Commands and the queue.** `command.rs`: `ChangeKind { Tactics, Substitution }` checked against the loaded `RulePack`; `Queue` holds pending changes in order, assigns `q-{tick}-{n}`, and answers `Ack { queue_id, queued_tick }` or `Reject { reason }`. The rejection wording is fixed here and asserted verbatim later: `unknown change type <name>`. Nothing is applied to play (PO Q8).
9. **Engine events.** `crates/engine/src/sim.rs`: `EngineEvent { tick, kind, team, scores }` pushed during `step()` at the three places the engine already knows — kick-off or restart after a goal, the goal itself (the `match.goal` trace site), and the final tick — plus `take_events()` draining the buffer. No protocol type enters `crates/engine`.
10. **Restart flag.** `crates/engine/src/record.rs`: `TickRecord` gains `restart: bool` carried in the frame, not in the 192-byte file record, which stays byte-identical for the validator and the benchmark. `FanoutSink<A: TickSink, B: TickSink>` forwards `on_tick` to A then B and returns the first error.
11. **Tuning block.** `crates/engine/src/data/tuning.rs`: `StreamTuning { buffer_ticks: 500, keyframe_interval: 50 }` with garde ranges `10..=5000` and `1..=500`; a fourth block in `TuningFile`. Update `content/tuning.json` and `content/README.md` (every field, unit, default, bound). The existing test that pins the file to the defaults must keep passing.
12. **Validator.** `crates/engine/src/validate.rs`: the jump rule reads the restart flag instead of the two-metre guess; delete the `sdlc-debt:` marker at line 77 with its shortcut. Test: a hand-built stream with a restart flag reports no violation, and the same stream without the flag reports one.
13. **Stream crate skeleton.** `crates/stream/Cargo.toml` (engine, protocol, tungstenite, serde_json, tracing) and `lib.rs` with `StreamError`.
14. **Server.** `server.rs`: `Server::bind()` on `127.0.0.1:0`, read the assigned port, write `SM_DATA_DIR/engine.port`, emit `socket.listening`. `accept()` runs the `tungstenite` handshake with an `Origin` allowlist (`null`, `file://`, any `http://localhost` or `http://127.0.0.1` port) and a protocol-version check that refuses with both versions named; emit `socket.client` or `socket.refused`; `set_nodelay(true)`; set `max_write_buffer_size` to four keyframes; one thread per client.
15. **Session and backpressure.** `session.rs`: `SocketSink` implements `TickSink`, encodes into one reused buffer, and pushes into `sync_channel(buffer_ticks)`; the writer thread drains it to the socket. A full channel blocks the simulation thread. A high-water gauge (`Arc<AtomicUsize>`) is exposed for the test. Emit `socket.backpressure` once per pause with `paused_ms` and the resume count. Treat `WriteBufferFull` as the same pause.
16. **Control channel.** `control.rs`: the reader thread decodes client commands; `Start` and `Pause` gate production through a condition variable, `SetSpeed` is stored and echoed in the acknowledgement, `QueueChange` goes to `protocol::Queue`; every verdict also becomes a `match-event` row.
17. **Event writer.** `events.rs`: drain `take_events()` each tick, write each as a JSON Lines row into `matches/<match.id>/events.jsonl` through one writer opened at kick-off, and send the same message over the socket.
18. **Recorder and replayer.** `record.rs`: a `.smfx` fixture — 32-byte header (magic `SMFX`, protocol version, match millis, frame count), length-prefixed frames with their kind tag and tick index, then a trailer with the count and the hash; emit `fixture.recorded`. `replay.rs`: serve the same protocol from the file, writing the stored bytes with no re-encoding, paced at `speed x 50` frames per second through `Instant`; emit `fixture.replayed`.
19. **Command line and the first measurement.** `cli.rs`, `main.rs`, `serve.rs`, `record.rs`, `replay.rs`, and `bench.rs --stream`. Run `cargo build --release` then `engine-cli bench --seed 42 --matches 1 --stream --json` and record `bench.stream_ticks_per_s` and `bench.stream_pauses`. Compare `bench --seed 42 --matches 5 --json` against 384 milliseconds and 5.62 megabytes before writing any further test. If the processor time exceeds the tripwire, the encoder allocates; fix it here, not later.
20. **Documentation and the tests.** `docs/reference/protocol.md` with one table per message; the six integration tests (`hello.rs`, `throughput.rs`, `backpressure.rs`, `fixture.rs`, `commands.rs`, `protocol/tests/document.rs`) and `crates/stream/tests/common/mod.rs`; the CLI tests; `README.md`. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.

## Verification Strategy

No user-observable AC — automated only. Every one of the six criteria in `03-slice-stream-protocol.md` is annotated `observable: false` and names a Rust test. No environment wall exists: every test binds `127.0.0.1:0` in the test process, sets `SM_DATA_DIR` to a temp folder, and needs no device, credential, browser, or external service. No `constraint-resolution:` line is needed.

One verification-relevant constraint is recorded rather than resolved: a browser page cannot reach a raw TCP socket, which is why the transport is WebSocket (PO Q1). A browser smoke test of the socket is possible from the in-app browser once a page exists; no page exists in this slice, so the first browser-driven evidence belongs to `viewer-pitch`, which the fixture and the replayer exist to serve.

## Test / Verification Plan

### Automated checks

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`: blocking, as in both prior slices.
- `cargo test --workspace`: the 66 existing tests plus the new files. `crates/protocol`: codec round trip, delta overflow forces a keyframe, message JSON round trip, unknown field refused, the `MESSAGES` completeness test, and `tests/document.rs` (AC-f). `crates/stream`: `hello.rs` (AC-a), `throughput.rs` (AC-b), `backpressure.rs` (AC-c), `fixture.rs` (AC-d), `commands.rs` (AC-e). `crates/engine`: the codec-adjacent restart test in `validate.rs` and the unchanged determinism, full-match, and validator tests. `crates/engine-cli`: `record` writes a fixture and prints its counts, `replay` refuses a missing fixture naming the path, `serve` prints a port and writes `engine.port`, and no help line exceeds 80 columns.
- Benchmark compare: `/wf verify` loads `augment/benchmark.md` in compare mode against `05c-benchmark.md` (389 ms, 694,087 ticks/s, 384 ms CPU, 5.62 MB, 1.4342 µs) and records the fifth target for the first time; tripwires 10 percent processor time (422 ms) and 25 percent memory (7.03 MB).
- Instrument key check: `04b-instrument.md` section 2 against the live records and stderr — `socket.listening`, `socket.client`, `socket.refused`, `socket.backpressure`, `fixture.recorded`, `fixture.replayed` on stderr; `match-event` rows in `events.jsonl` and over the socket.

### Interactive verification (human-in-the-loop)

Automated only — every criterion is annotated `observable: false`; the slice ships no user interface, and its one user-visible surface is the command line, whose output the automated tests parse. `stack.platforms: [web, cli]`, `stack.testing: [cargo-test]`, `user-confirmed: true`. The `web` adapter has no detection signal in the repository yet (no `package.json`, no HTML), so no web adapter runs.

## Risks / Watchouts

- R1 (high) the encoder on the hot path: 270,000 frames per match at 694,087 ticks per second. `encode_into` writes into one reused buffer owned by the sink; the channel carries a fixed-size frame, never a `Vec` per tick; step 19 measures before the tests are declared done.
- R2 (high) determinism through a second sink: `FanoutSink` calls A then B in order and returns the first error; the determinism test keeps running on the file path; the fixture test compares recorded bytes, not re-encoded ones.
- R3 (medium) `tungstenite` buffers without a bound by default (`max_write_buffer_size` is `usize::MAX`, `.scratch/sources/rust/tungstenite-0.30.0/src/protocol/mod.rs:100`). Set it to four keyframes; treat `WriteBufferFull` as a pause; the throttled test asserts the high-water mark.
- R4 (medium) a delta step wider than one signed byte: `i8` covers 1.27 m per tick and the ball cap is 0.80 m, but a restart teleport is unbounded. The encoder forces a keyframe on overflow and the codec test drives a restart.
- R5 (medium) the Origin allowlist blocking the real viewer: accept any `http://localhost` or `http://127.0.0.1` origin regardless of port, and name the origin in the refusal line.
- R6 (medium) `change.queue_id` is outside the Block A vocabulary; emitted as an additive extra, settled by the observability audit.
- R7 (low) three threads in one binary must all end: the channel disconnect is the shutdown signal both ways; every test joins with a timeout; the CLI test asserts the process exits.
- R8 (low) a stale `engine.port` after a crash: `serve` removes the file on a clean exit, and the hello carries `match.id` so a page that reaches the wrong engine can tell.
- Watch: `tungstenite` must never mask a server frame and must refuse an unmasked client frame; both are its default behaviour (`.scratch/sources/rust/tungstenite-0.30.0/src/protocol/frame/mod.rs:214-224`). Do not set `accept_unmasked_frames`.
- Watch: bind `127.0.0.1`, never `0.0.0.0`, which also avoids a Windows Firewall prompt.

## Dependencies on Other Slices

- `engine-core` (complete): the tick loop, `TickSink`, and the command line.
- `data-schemas-generator` (complete): `Content`, the `RulePack` the command validator reads, and `observe::identity`.
- `viewer-pitch` (later): consumes the fixture, the replayer, and the decoder mirror of `crates/protocol`.
- `match-rules` (later): takes the held queue and applies changes at qualifying stoppages; adds the event types this slice does not emit.
- `commentary` (later): reads the event stream this slice opens.
- `/wf observability init` (done, plan-version 1): this slice writes the first `match-event` records against its vocabulary; the audit settles `change.queue_id`.

## Assumptions

- The visual contract `02c-craft.md` is not authored here: the slice ships no user-interface surface; the `viewer-pitch` plan authors it.
- The `experiment` augmentation stays deferred to the `experiment-flags` slice per the shape.
- One client per match in version 1; the debug dashboard is a second connection later, which the thread-per-client server already serves.
- The fixture file extension is `.smfx` and is gitignored like `.ticks`; the viewer slices generate their own.
- Consult: triggers `appetite-medium-or-larger` and `unknowns-present` hold; the PO excluded `consult` at intake; not fired.

## Blockers

None.

## Freshness Research

- Source: MDN WebSocket API and the WICG Local Network Access specification. Why it matters: whether the page can reach the engine at all. Takeaway: a page can open WebSocket or HTTP only; loopback is a secure context, so a `file://` or `http://localhost` page reaching `ws://127.0.0.1` is neither mixed-content blocked nor permission-prompted; `WebSocket.bufferedAmount` exists but the API has no built-in backpressure, so the client paces itself.
- Source: `.scratch/sources/rust/tungstenite-0.30.0/src/protocol/mod.rs` (fetched 2026-09-22). Why it matters: the chosen transport. Takeaway: `read`, `write`, `flush` on a blocking stream with no runtime; `WebSocketConfig::max_write_buffer_size` defaults to `usize::MAX` and must be set for bounded backpressure; `max_message_size` 64 MiB and `max_frame_size` 16 MiB are far above any frame here; `Error::WriteBufferFull` is the pause signal.
- Source: `.scratch/sources/rust/tungstenite-0.30.0/src/protocol/frame/mod.rs:214-224`. Why it matters: protocol correctness. Takeaway: the library unmasks client frames, refuses an unmasked one by default, and never masks a server frame, as RFC 6455 section 5.1 requires.
- Source: crates.io and lib.rs for `tungstenite` 0.30.0, `tokio-tungstenite`, `fastwebsockets`, `soketto`. Why it matters: the dependency decision. Takeaway: `tungstenite` is the only candidate that needs no async runtime; MIT OR Apache-2.0, MSRV 1.85, about eight permissive transitive crates; `fastwebsockets` and `soketto` both need `tokio` or a futures executor. RUSTSEC-2023-0065 affects versions before 0.20.1 and not 0.30.0.
- Source: RFC 6455 section 5.1. Why it matters: who may connect. Takeaway: the `Origin` header exists to stop cross-origin use from a browser, cannot be forged by page code, and the server may refuse on it, which is the guard chosen over a token.
- Source: Rust standard library documentation for `std::sync::mpsc::sync_channel` (1.92). Why it matters: the bounded buffer. Takeaway: `send` blocks when the buffer is full and unblocks as the receiver drains; a disconnected receiver returns `SendError`, which is the shutdown signal.
- Source: V8 blog on `DataView` performance. Why it matters: the decoder on the page. Takeaway: `DataView` matches typed-array speed for fixed-offset little-endian reads, so the viewer can decode 400 frames per second without a typed-array overlay; a structure-of-arrays target avoids per-tick allocation.
- Source: Clippy lint-group documentation. Why it matters: the blocking gate. Takeaway: `cast_possible_truncation` and `cast_sign_loss` sit in `pedantic`, which is not enabled here, so the quantisation casts do not need allowances; `clippy::all` stays clean.

## Recommended Next Stage

- **Option A (recommended): `/wf implement football-manager-match-engine stream-protocol`** — the plan is complete, no blocker exists, and both augmentation artifacts are ready. Compact the session first; the SessionStart hook re-reads the artifacts.
- **Option C: `/wf slice football-manager-match-engine`** — only if the two new crates are judged to be two slices rather than one. The plan holds them together because the codec is useless without a server and the fixture is the viewer's prerequisite.
