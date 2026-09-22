---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: stream-protocol
status: complete
stage-number: 5
created-at: "2026-09-22T12:35:00Z"
updated-at: "2026-09-22T12:35:00Z"
metric-files-changed: 47
metric-lines-added: 5167
metric-lines-removed: 35
metric-deviations-from-plan: 10
metric-review-fixes-applied: 0
commit-sha: "f361e22"
steering-honored:
  - "used the four change-state words Queued, Applies now, Applied, Rejected"
  - "left every layout, typography, palette, and mark rule to the viewer slices"
tags: [engine, protocol, socket, fixture, websocket, rust]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-stream-protocol.md
  plan: 04-plan-stream-protocol.md
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md]
  verify: 06-verify-stream-protocol.md
  instrument: 04b-instrument.md
  benchmark: 05c-benchmark.md
  reference: ../../../docs/reference/protocol.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine stream-protocol"
---

# Implement: Stream Protocol and Fixture Harness

## The Implementation

Two slices handed over an engine that wrote 694,087 ticks per second into a file behind one
consumer seam, with no thread, no socket, and no clock anywhere in the workspace. This stage
built two crates and opened the socket the observability contract had named eight times and
nothing had ever served. `crates/protocol` holds the messages, the tick codec, and the change
queue, and performs no input and no output, so the viewer decoder and a later WebAssembly
build can mirror it. `crates/stream` holds the server, the bounded producer buffer, the
recorder, and the replayer.

Ten decisions departed from the plan and each one is recorded below. The largest is the tick
file: the plan put the restart flag only in the wire frame, but `engine-cli simulate`
validates a stream it re-reads from the file, so a frame-only flag would have turned every
kick-off into a false `ball_speed` violation. The flag now rides in bit 31 of the stored tick
number, the record stays 192 bytes, and the file schema goes 2 to 3 under the fail-closed
pattern the header already used. Two other decisions came from reading the installed
transport: `tungstenite` panics unless `max_write_buffer_size` is above `write_buffer_size`,
so the bound is 8 KiB over 4 KiB rather than four keyframes, and one socket thread on a
non-blocking socket replaces the plan's reader and writer pair. Two defects surfaced during
the build and both are fixed: the socket thread outlived its match because a sink still held
a sending half, and the buffer gauge underflowed when the consumer took a frame before the
producer had counted it.

Verify runs next against six criteria that are all automated. Tests went from 66 to 121, the
benchmark holds at 387.4 milliseconds of processor time against a 422-millisecond tripwire
and 6.01 megabytes against 7.03, and the socket delivered 640,057 ticks per second to an
unthrottled client with no pause. The top risk left is the change queue: this build queues
and answers but applies nothing to play, so `match-rules` must take the queue before
"customizable" means anything to a player.

## Summary of Changes

- Two new crates: `crates/protocol` (messages, codec, change queue, no input or output) and
  `crates/stream` (server, session, control channel, event writer, recorder, replayer,
  loopback client).
- A versioned WebSocket protocol on `127.0.0.1`: a 98-byte keyframe every 50 ticks and a
  47-byte delta between, with JSON text frames for control. About 13 megabytes per match
  against 51.8 raw.
- Handshake guards: an `Origin` allowlist for local pages and a protocol-version check that
  names both versions. A refused handshake answers 403 or 400 and the server keeps listening.
- Backpressure with two bounds: a 500-tick `sync_channel` that blocks the simulation thread,
  and an 8-kilobyte socket write buffer whose overflow is treated as the same pause.
- The `match-event` record kind, written for the first time: kick-off, goal, full time, and
  every queued or refused change, appended to `matches/<match.id>/events.jsonl` and sent over
  the socket.
- The `.smfx` fixture: every frame of one match exactly as it travels on the wire, with a
  replayer that writes those bytes back with no re-encoding.
- Three command-line subcommands (`serve`, `record`, `replay`) and `bench --stream`.
- The restart flag on the tick record, which retires the `sdlc-debt:` marker in the validator.
- `docs/reference/protocol.md`, held to the code by a test.

## Files Changed

- `Cargo.toml`: two new workspace members and `tungstenite` 0.30 with `default-features = false, features = ["handshake"]`. The fetched tree is `bytes`, `data-encoding`, `http`, `httparse`, `sha1` on top of `log`, `rand`, and `thiserror`, all MIT or Apache-2.0 (NFR-5).
- `crates/protocol/Cargo.toml`: serde, serde_json, and thiserror only, so the crate stays free of input and output.
- `crates/protocol/src/lib.rs`: `PROTOCOL_VERSION`, `ProtocolError` with explicit `#[source]`, and `MESSAGES` at line 85, the list the reference document is held to.
- `crates/protocol/src/codec.rs`: `Quantised` in signed centimetres, keyframe and delta encoders that write into a caller-owned buffer, and the matching decoders.
- `crates/protocol/src/frame.rs`: the one place that decides ticks are binary frames and control messages are JSON text frames. `TickFrame` is a fixed-size buffer, never a heap allocation.
- `crates/protocol/src/message.rs`: `ServerMessage` and `ClientCommand`, every payload refusing an unknown field.
- `crates/protocol/src/event.rs`: `MatchEvent`, `EventType`, and `ChangeOutcome` with the contract keys.
- `crates/protocol/src/command.rs`: `ChangeKind`, `ChangeState` with the four viewer words, and `Queue`, which assigns `q-{tick}-{n}` and refuses an unknown type by name.
- `crates/protocol/tests/document.rs`: AC-f. Every message in `MESSAGES` must appear in the reference document with every field.
- `crates/stream/src/lib.rs`: `StreamError` and the conversion into `EngineError::Sink`.
- `crates/stream/src/server.rs`: the loopback listener, the port file, the `Origin` allowlist, and the version check. The write-buffer bounds are at lines 20 and 23.
- `crates/stream/src/session.rs`: the bounded producer buffer, `Gauge` at line 64, `FrameSink`, and the socket thread.
- `crates/stream/src/control.rs`: `Gate` for start and pause, and `CommandContext::handle`, which answers every command and writes the change verdict to the record.
- `crates/stream/src/events.rs`: `EventRecord` at line 21 wearing the engine's observability envelope, and the append-only `EventWriter`.
- `crates/stream/src/record.rs`: the `.smfx` recorder and reader, with a header, length-prefixed entries, and a hashed trailer.
- `crates/stream/src/replay.rs`: the mock server, paced by tick index through `Instant`.
- `crates/stream/src/client.rs`: the loopback client the benchmark and the tests read with.
- `crates/stream/tests/`: `common/mod.rs` (a real seeded match over a real socket), `hello.rs`, `throughput.rs`, `backpressure.rs`, `fixture.rs`, `commands.rs`.
- `crates/engine/src/record.rs`: `SCHEMA_VERSION` 3 with the restart flag in bit 31 of the stored tick (lines 26 and 29), `FanoutSink` at line 175, and a discarding `Option<S>` sink at line 201.
- `crates/engine/src/sim.rs`: `EngineEvent`, the restart flag on `record()`, `finish()` at line 181, and `take_events()` at line 191.
- `crates/engine/src/validate.rs`: the jump rule reads the restart flag at line 74; the `sdlc-debt:` marker and its two-metre guess are gone.
- `crates/engine/src/error.rs`: `EngineError::Sink` at line 26, for a consumer that goes away.
- `crates/engine/src/data/tuning.rs`: `StreamTuning` at line 36 with `buffer_ticks` 10 to 5000 and `keyframe_interval` 1 to 500.
- `crates/engine/src/observe/mod.rs`: two optional fields on `RunReport` for the socket measurement.
- `crates/engine-cli/src/serve.rs`, `record.rs`, `replay.rs`, `stream_run.rs`: the three subcommands and the one match driver they share.
- `crates/engine-cli/src/bench.rs`: `--stream` runs one match to an in-process client and reports the delivered rate and the pause count.
- `crates/engine-cli/tests/stream_cli.rs`: the command-line tests, including a real `serve` process streaming to a real client.
- `content/tuning.json`, `content/README.md`: the `stream` block, every field with its unit, default, and bound.
- `docs/reference/protocol.md`: one table per message, plus framing, backpressure, and the fixture layout.
- `README.md`: the streaming, recording, and replay commands.
- `.gitignore`: `*.smfx`, like `*.ticks`.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/record.rs` — `engine-core` owns the tick file; this slice bumps its schema and adds the fan-out.
- `crates/engine/src/sim.rs` — `engine-core` owns the loop; this slice adds the event buffer and the restart flag.
- `crates/engine/src/observe/mod.rs` — `data-schemas-generator` owns the records; this slice adds two optional benchmark fields.
- `crates/engine/src/data/tuning.rs` and `content/tuning.json` — `data-schemas-generator` owns the loader; this slice adds a fourth block.
- `crates/engine-cli/src/cli.rs` and `main.rs` — both prior slices own subcommands; this slice adds three.

## Notes on Design Choices

- The codec quantises to signed centimetres, so a delta is one signed byte per component. One byte covers 1.27 m of movement per tick against a 0.80 m ball cap, and a restart or any wider step forces a keyframe. The decoder reconstructs absolute positions and invents nothing (RIM-5, C2).
- `TickFrame` is a fixed 99-byte buffer, so the hot path copies and never allocates. 270,000 frames per match at 694,087 ticks per second left no room for a `Vec` per tick.
- The engine names facts and the protocol decides how they travel: `EngineEvent` is an engine type and no protocol type enters `crates/engine`.
- `crates/protocol` depends on serde alone. The envelope a `match-event` row needs comes from a local newtype in `crates/stream`, so the pure crate stays pure.
- The change queue holds and answers; it applies nothing to play. `ChangeState` already carries all four words, so `match-rules` adds behaviour and not vocabulary.
- The server binds `127.0.0.1` only, never `0.0.0.0`, which also avoids a Windows Firewall prompt.

## Verification Seams Built

Every criterion in `03-slice-stream-protocol.md` is annotated `observable: false` and names a
Rust test, so the seams are test seams rather than user-surface seams. Each line below cites a
file re-opened after editing.

- AC-a (hello before the first tick) → a loopback client at `crates/stream/src/client.rs:32`, plus the `Served` harness in `crates/stream/tests/common/mod.rs` (enables `cargo test -p stream --test hello` to read the message order).
- AC-b (270,000 ticks in order, buffer within its bound) → `Gauge` at `crates/stream/src/session.rs:64` with `high_water()` at line 86 (enables the throughput test to read the buffer depth without a debugger).
- AC-c (a slow client pauses the producer) → `Gauge::pauses()` at `crates/stream/src/session.rs:91`, plus `buffer_ticks` on `SessionConfig` at `crates/stream/src/session.rs:229` (enables the test to choose a 64-tick bound instead of waiting for 500).
- AC-d (a replayed fixture is byte-identical) → `read_fixture` in `crates/stream/src/record.rs` and `Client::read_raw` at `crates/stream/src/client.rs:92` (enables a byte comparison with no re-decoding).
- AC-e (a change is acknowledged or refused by name) → `EventWriter::path()` at `crates/stream/src/events.rs:89` (enables the test to read the `match-event` rows the verdict wrote).
- AC-f (every message is in the document) → `MESSAGES` at `crates/protocol/src/lib.rs:85` and `message_spec` at line 188 (enables `crates/protocol/tests/document.rs` to enumerate the implementation against the document).
- Command line (`serve` is reachable from a test) → the port on standard output at `crates/engine-cli/src/serve.rs:53` and `Server::address()` at `crates/stream/src/server.rs:78` (enables `crates/engine-cli/tests/stream_cli.rs` to connect to a real child process without guessing a port).

## Deviations from Plan

1. **The restart flag rides in the tick file, and the file schema is 3.** The plan put it in the frame only, calling the 192-byte record byte-identical. But `engine-cli simulate` validates a stream re-read from the file (`crates/engine-cli/src/simulate.rs`), where a frame-only flag is lost, so every kick-off would have become a false `ball_speed` violation. The flag now occupies bit 31 of the stored tick number at `crates/engine/src/record.rs:29`; the record is still 192 bytes and the layout is unchanged. `SCHEMA_VERSION` goes 2 to 3, which is the header's own fail-closed pattern.
2. **`max_write_buffer_size` is 8 KiB over a 4 KiB write buffer, not four keyframes.** `WebSocketConfig::assert_valid` panics unless `max_write_buffer_size > write_buffer_size` (source: `.scratch/sources/rust/tungstenite-0.30.0/src/protocol/mod.rs`, `assert_valid`), and a 392-byte buffer would force one write syscall per 48-byte frame.
3. **Two threads, not three.** One socket thread on a non-blocking socket reads commands and writes frames. Splitting the socket into a reader half and a writer half would have needed the handshake's buffered bytes, which `tungstenite` does not expose after `accept_hdr_with_config` (source: `.scratch/sources/rust/tungstenite-0.30.0/src/server.rs`, which returns a `WebSocket` and no partial buffer).
4. **The `Record` implementation lives in `crates/stream/src/events.rs:29`, not `crates/protocol/src/event.rs`.** The orphan rule forbids implementing the engine's trait for a protocol type from a third crate, and `crates/protocol` must not depend on `crates/engine`. A local newtype, `EventRecord`, carries the envelope and the output is identical.
5. **`crates/stream/src/client.rs` is a file the plan did not list.** `bench --stream` and all five integration tests need a loopback client, and a `tests/common` helper cannot be linked into the binary.
6. **`Frame::Tick(TickFrame)` carries a fixed-size buffer instead of `Frame::Binary(Vec<u8>)`.** Risk R1 forbids an allocation per tick on the hot path.
7. **`ServerMessage` has five JSON variants and the binary tick frame is the sixth entry of `MESSAGES` with no enumeration variant.** A serde variant for a 99-byte array needs a hand-written codec and buys nothing; the completeness test still matches exhaustively on both enumerations, so a new variant cannot be added without a compile error.
8. **`EngineError` gains a `Sink(String)` variant** at `crates/engine/src/error.rs:26`. A `TickSink` can now fail for a reason that is neither a file nor a format: the viewer went away.
9. **`Server::accept` keeps listening past a refused handshake.** The plan implied one accept. One page with the wrong origin must not end the match for the right one (R5), and the refusal is still on the record and in the client's 403 or 400.
10. **`serve --ticks-out` is the `FanoutSink` call site.** The plan asked for the fan-out and named no user; streaming a match while keeping its tick file is the natural one.

## Anything Deferred

- **Applying a queued change to play.** This build queues and answers only (PO Q8). `ChangeState::AppliesNow` and `ChangeState::Applied` exist and are never produced here. Upgrade path: `match-rules` takes `Queue::pending()` and applies each change at a qualifying stoppage.
- **`change.queue_id` stays an additive extra** (R6). The contract reserves no key for the identifier an acknowledgement returns. Upgrade path: `/wf observability audit` settles whether it is a key of its own.
- **One viewer per match.** The thread-per-client shape is there, but `serve` accepts exactly one client. Upgrade path: a second accept for the debug dashboard.
- **`QueueChange.detail` is opaque.** It is carried and recorded and never read. Upgrade path: `match-rules` gives it a schema.
- **The touchline and goal-line `sdlc-debt:` marker in `crates/engine/src/sim.rs`** is untouched; it names `match-rules`, not this slice.

## Known Risks / Caveats

- **The buffer gauge can read one above its bound during hand-off.** `sync_channel(bound)` bounds the buffer structurally; the gauge counts a frame from before the producer's send until after the consumer's receive, so one frame in flight can show as depth `bound + 1`. Both throughput and backpressure tests assert `<= bound + 1` and say why.
- **`MatchState` packs both scores into one `AtomicU64`.** A reader can see a tick from one moment and a score from another. The only reader is the change-event writer, at a rate of a few messages per match.
- **The socket thread sleeps 100 microseconds when neither direction has work.** At 640,057 delivered ticks per second the loop is never idle during play, but a paused match wakes the thread 10,000 times a second. Upgrade path: block on a condition variable when the gate is closed.
- **Peak memory rose from 5.62 to 6.01 megabytes** on the non-streaming benchmark, which links but never runs the new crates. That is 7 percent against a 25 percent tripwire.

## Freshness Research

- Source: `.scratch/sources/rust/tungstenite-0.30.0/src/protocol/mod.rs` (read during the build). Why it matters: the write path. Takeaway: `assert_valid` panics unless `max_write_buffer_size > write_buffer_size`; `buffer_frame` answers `WriteBufferFull(Message::Frame(frame))` and hands the frame back, so nothing is lost on a pause; a non-blocking write returns `Io(WouldBlock)` with the frame already buffered.
- Source: `.scratch/sources/rust/tungstenite-0.30.0/src/protocol/frame/mod.rs`. Why it matters: protocol correctness and the non-blocking read. Takeaway: the library refuses an unmasked client frame by default and never masks a server frame; `read_frame` propagates `WouldBlock` from the stream, so a non-blocking read returns rather than spinning. `accept_unmasked_frames` is not set.
- Source: `.scratch/sources/rust/tungstenite-0.30.0/src/server.rs`. Why it matters: deviation 3. Takeaway: `accept_hdr_with_config` returns a `WebSocket` and exposes no partially-read buffer, so the stream cannot be safely split into two halves after the handshake.
- Source: `crates/engine/src/record.rs` and `crates/engine-cli/src/simulate.rs` in this repository. Why it matters: deviation 1. Takeaway: `simulate` validates records re-read from the file, so a flag that exists only on the wire cannot reach the validator.

## Measurements Taken

Release build, `SM_DATA_DIR` on a scratch folder, three drives each.

| Target | Baseline (`05c-benchmark.md`) | Measured | Tripwire | Verdict |
|---|---|---|---|---|
| full match wall time | 389 ms | 390 ms (386, 390, 390) | 2000 ms budget | pass |
| processor time per match | 384 ms | 387.4 ms (1937 ms over 5) | 422 ms | pass |
| peak memory | 5.62 MB | 6.01 MB (5.98, 6.01, 6.08) | 7.03 MB | pass |
| ticks per second | 694,087 | 692,308 | none | -0.3 percent |
| stream throughput | not measured | 640,057 ticks/s (639,260, 640,057, 641,982) | none; first value | new baseline |
| stream pauses | not measured | 0 (0, 0, 3) | none | unthrottled client |

`/wf verify` re-runs the compare and records the fifth target formally.

## Instrumentation Applied

All eight signals of `04b-instrument.md` section 2 are in place. `fixture.recorded` was
confirmed live: `signal="fixture.recorded" path=m.smfx frames=3003 ticks=3000 bytes=174675
hash=7aa4908d59c8`. `socket.listening` and `socket.client` were confirmed live during the
benchmark run. The remaining four are at `crates/stream/src/server.rs:137`
(`socket.refused`), `crates/stream/src/session.rs:145` (`socket.backpressure`),
`crates/stream/src/replay.rs:98` (`fixture.replayed`), and the `match-event` rows written by
`crates/stream/src/events.rs`. `crates/protocol` carries no tracing call, as the artifact
requires.

## Recommended Next Stage

- **Option A (recommended): `/wf verify football-manager-match-engine stream-protocol`** — the slice ships testable behaviour, all six criteria name Rust tests, and both augmentation re-checks (benchmark compare, instrument key check) are ready. Consider compacting the session first; workflow state lives in artifact files on disk and the SessionStart hook re-reads it after compaction.
- **Option B: `/wf review football-manager-match-engine stream-protocol`** — only if the 121 passing tests, the clean `cargo clippy --workspace --all-targets -- -D warnings`, and the measurements above are judged sufficient. Not recommended: the benchmark compare and the instrument key check belong to verify.
