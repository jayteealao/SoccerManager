---
schema: sdlc/v1
type: augmentation
augmentation-type: instrument
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: viewer-pitch
instrumentation-framework: "engine side: serde_json JSON Lines per .ai/observability.md plan-version 1, tracing to stderr. Page side: console JSON Lines with the same envelope, plus a ring buffer on the page test hook. No browser transport exists yet."
dark-paths-found: 5
signals-designed: 9
pii-warnings: false
status: ready
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T14:41:32Z"
revision-count: 3
revisions:
  - rev: 1
    at: "2026-09-22T06:37:07Z"
    trigger: new-slice
    because: "data-schemas-generator plan designs the loader, generator, and identity signals (PO plan Q11)"
    changed: "slice-slug, signal table, dark paths; engine-core record kept at history/04b-instrument-0.md"
  - rev: 2
    at: "2026-09-22T11:28:51Z"
    trigger: new-slice
    because: "stream-protocol plan designs the socket, event, and command signals; the contract already reserves the socket keys"
    changed: "slice-slug, signal table, dark paths, implementation notes; the data-schemas-generator record is kept at history/04b-instrument-1.md"
  - rev: 3
    at: "2026-09-22T14:41:32Z"
    trigger: new-slice
    because: "viewer-pitch plan designs page-side signals; the product owner chose option 3 alongside option 2 at plan Round 3 Q12"
    changed: "slice-slug, framework now names two sides, eight browser signals plus one new engine signal, five browser dark paths, a new transport-gap note; the stream-protocol record is kept at history/04b-instrument-2.md"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-viewer-pitch.md
  contract: ../../observability.md
  prior: history/04b-instrument-2.md
---

# Instrumentation: Viewer Pitch and Playback

## The Instrumentation

Three slices have filled four record kinds and eighteen signals, every one of them written by the Rust process to stderr or to a JSON Lines file. The contract has been half-waiting for this slice since its first version: it reserves `viewer.socket_drops` at line 65 and puts the viewer in scope over the socket, and the shape's Augmentation Plan names "a viewer that drops ticks silently" as a dark path. Nothing in a browser has ever written a signal, because nothing in a browser has ever existed.

Nine signals are designed. Eight of them run in the page: one for the connection, one for a socket that drops mid-match, one for skipped ticks, one for a playback speed the engine cannot sustain, one rolling frame-budget summary, one history and memory reading, one rewind, and one refused frame. The ninth runs in Rust, because this slice makes the engine binary serve the page: `web.serving` names the directory, the file count, and whether the two cross-origin-isolation headers are on, which is the precondition for the memory gauge the product owner chose. One existing signal gains a field: `fixture.replayed` carries the new `sustain` cap beside its `speed`.

The product owner chose this work knowing its cost (plan Round 3 Q12, options 2 and 3 together). The cost is real and is recorded here rather than hidden: **the page-side signals have no transport.** The observability contract closed U-2 for the engine with a file sink plus the socket feed, and neither reaches a browser. Until a browser path exists, every page-side signal is one JSON Lines row on `console.info` plus a ring buffer on the page test hook, which a browser drive and a Node test can both read. That is enough for verify and not enough for a dashboard, and `/wf observability` owns closing it.

## 1. Current state

| File | Quality | Existing signals | Dark paths |
|------|---------|-----------------|------------|
| `web/socket.mjs` (planned) | dark | none | the socket closes mid-match and the page keeps drawing the last frame with no statement that the stream ended |
| `web/schedule.mjs` (planned) | dark | none | the scheduler skips ticks at high speed and nothing counts them; the shape names this dark path directly |
| `web/playback.mjs` (planned) | dark | none | playback runs slower than the selected speed and the manager is never told |
| `web/history.mjs` (planned) | dark | none | the history grows past the 300 MB budget and the page keeps allocating |
| `web/decode.mjs` (planned) | dark | none | a frame the decoder cannot read is discarded and the match simply stutters |
| `crates/engine-cli/src/web.rs` (planned) | dark | none | the static server serves a directory with no statement of which one, or with the isolation headers off, which silently disables the memory gauge |
| `crates/stream/src/replay.rs` | good | `fixture.replayed` | none new; the signal gains the `sustain` field |
| `crates/stream/src/server.rs`, `session.rs`, `record.rs` | good | `socket.listening`, `socket.client`, `socket.refused`, `socket.backpressure`, `fixture.recorded` | none new |
| `crates/protocol/src/event.rs`, `command.rs` | good | `match-event.engine`, `match-event.change` | none new |
| `crates/engine/src/observe/mod.rs`, `record.rs`, `data/mod.rs` | good | `match-stats`, `run-report`, `tickfile.*`, `content.*` | none new |

Summary: 5 dark paths found, all of them in the page, across 10 files (6 planned). Framework: two sides now. The Rust side is unchanged. The page side writes the same envelope to `console.info` and to a ring buffer, with no transport.

## 2. Instrumentation plan

| File | Function/path | Signal type | Signal name | Key fields | Rationale |
|------|--------------|-------------|-------------|------------|-----------|
| `web/socket.mjs` | `open()` after the hello decodes | event (console JSON) | `viewer.connected` | `protocol.version`, `engine.version`, `match.id`, `owner.id`, `origin`, `ticks_expected` | The page states which engine and which match it is showing, so a screenshot is attributable |
| `web/socket.mjs` | the `close` and `error` handlers | event (console JSON) | `viewer.socket_drops` | `match.id`, `tick`, `code`, `wasClean`, `reason` | The contract reserves this key at line 65; a dropped socket stops looking like a frozen match |
| `web/schedule.mjs` | the frame scheduler, once per second | event (console JSON) | `viewer.tick_skipped` | `tick`, `skipped`, `speed`, `window_s` | Closes the shape's named dark path: a viewer that drops ticks silently |
| `web/playback.mjs` | the sustained-rate estimator | event (console JSON) | `viewer.lag` | `requested_speed`, `sustained_speed`, `tick`, `notice_shown` | The page states why it slowed down; this is the evidence for AC-5 |
| `web/schedule.mjs` | rolling window, once per five seconds | metric (console JSON) | `viewer.frame_budget` | `frames`, `fps_median`, `frame_ms_p95`, `dropped_frames`, `refresh_hz`, `window_s` | The frame-time counter is a signal, not only a number on screen; `refresh_hz` stops a 144 Hz panel reading as a pass |
| `web/history.mjs` | after each keyframe, and on demand | metric (console JSON) | `viewer.history` | `ticks_stored`, `history_bytes`, `page_bytes`, `budget_bytes` | Both numbers the product owner chose at Round 3 Q9: the exact byte count and the browser's whole-page figure |
| `web/playback.mjs` | the scrubber commit | event (console JSON) | `viewer.rewind` | `from_tick`, `to_tick`, `exact` | A rewind states where it landed and whether the drawn frame equalled the stored tick |
| `web/decode.mjs` | the frame decoder's refusal path | log (console JSON, warn) | `viewer.decode_refused` | `reason`, `tick`, `bytes`, `kind` | A frame the page cannot read is named, not swallowed |
| `crates/engine-cli/src/web.rs` | the static server's bind | event (tracing info) | `web.serving` | `dir` (relative), `files`, `isolated`, `port` | States the directory, the file count, and whether the two isolation headers are on, which the memory gauge requires |

One existing signal changes: `fixture.replayed` in `crates/stream/src/replay.rs` gains `sustain`, the new cap value, beside its `speed`. A replay run without `--sustain` reports `sustain = null`.

## 3. Signal designs

```js
// web/signal.mjs — one writer, the same envelope the Rust records use.
export function signal(name, fields) {
  const row = {
    "record.kind": "viewer-event",
    "schema.version": "1",
    service: "touchline-viewer",
    operation: "view",
    signal: name,
    ts: new Date().toISOString(),
    ...fields,
  };
  console.info(JSON.stringify(row));   // the only transport that exists today
  ring.push(row);                      // last 256 rows, readable at window.__touchline.signals
  return row;
}

// web/socket.mjs
signal("viewer.connected", { "protocol.version": hello["protocol.version"], "engine.version": hello["engine.version"],
                             "match.id": hello["match.id"], "owner.id": hello["owner.id"],
                             origin: location.origin, ticks_expected: hello.ticks_expected });
signal("viewer.socket_drops", { "match.id": matchId, tick: lastTick, code: ev.code, wasClean: ev.wasClean, reason: ev.reason });

// web/schedule.mjs — once per second, and only when skipped > 0
signal("viewer.tick_skipped", { tick, skipped, speed, window_s: 1 });
// rolling five-second window, always
signal("viewer.frame_budget", { frames, fps_median, frame_ms_p95, dropped_frames, refresh_hz, window_s: 5 });

// web/playback.mjs
signal("viewer.lag", { requested_speed, sustained_speed, tick, notice_shown });
signal("viewer.rewind", { from_tick, to_tick, exact });

// web/history.mjs — page_bytes is null until the browser resolves its measurement
signal("viewer.history", { ticks_stored, history_bytes, page_bytes, budget_bytes: 314572800 });

// web/decode.mjs
signal("viewer.decode_refused", { reason, tick, bytes, kind });
```

```rust
// crates/engine-cli/src/web.rs — the static server this slice adds
tracing::info!(signal = "web.serving", dir = %relative, files, isolated, port);

// crates/stream/src/replay.rs — the existing signal gains one field
tracing::info!(signal = "fixture.replayed", path = %relative, frames, speed, sustain = ?sustain, hash = %hash12);
```

Types: every tick is an integer; `speed`, `requested_speed`, and `sustained_speed` are floats with one decimal; `fps_median` and `refresh_hz` are integers; `frame_ms_p95` is a float with two decimals; `history_bytes`, `page_bytes`, and `budget_bytes` are integers in bytes, and `page_bytes` is `null` until the browser resolves its asynchronous measurement; `isolated` is a boolean; `exact` is a boolean that states whether the drawn frame equalled the stored tick exactly. `record.kind` is `viewer-event`, a fifth kind that the observability audit must accept or rename.

## 4. PII & security notes

No PII concerns identified. `owner.id` is opaque per the contract and is echoed from the hello, never minted in the page. `origin` is `http://127.0.0.1:<port>`, a loopback value. No file path from the user's machine enters a page-side signal, because the page has no file access. `web.serving` reports the served directory relative to the working directory, never an absolute path.

One security note that is not a PII note: this slice sets `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` on every response from the static server, because the memory gauge the product owner chose requires them. That is a tightening, not a loosening: it isolates the page from every other browsing context and refuses any cross-origin subresource. Every asset this page loads is same-origin, so nothing is lost today, and `web.serving` reports `isolated` so a run with the headers off is visible rather than silently ungauged.

## 5. Implementation notes

- **The transport gap is the headline.** `.ai/observability.md` closed U-2 with a file sink plus the socket feed. Neither reaches a browser: the page cannot write a file, and the socket carries engine-to-page messages, not page-to-engine records. Until a browser path exists, `console.info` plus the ring buffer is the whole transport. Do not invent a sink in this slice; record the gap and let `/wf observability` decide between a page-to-engine record message, a beacon to a new endpoint on the static server, or nothing.
- **`record.kind: viewer-event` is a fifth kind** and is outside the four the contract's Block A names. Emit it as the additive extra `content.hash`, `teams`, and `change.queue_id` already are, and let `/wf observability audit` settle whether it is a kind of its own or a variant of `match-event`.
- **`viewer.socket_drops` is the one reserved key.** Spell it exactly as the contract does, including the underscore, so the reservation is honoured rather than shadowed by a near-miss name.
- **Sampling.** `viewer.frame_budget` is a five-second rolling summary, and `viewer.tick_skipped` is a one-second summary emitted only when the count is above zero. Neither writes per frame, because 60 rows per second over 90 minutes would be 324,000 rows for one match. `viewer.connected`, `viewer.socket_drops`, `viewer.lag`, and `viewer.rewind` are per-occurrence.
- **`page_bytes` is asynchronous.** `performance.measureUserAgentSpecificMemory()` returns a promise the browser resolves on its own schedule and may coalesce. Write `null` when no measurement has resolved yet and never block a frame on it. `history_bytes` is exact and synchronous, which is why Round 3 Q9 kept both.
- **`refresh_hz` exists to stop a false pass.** `requestAnimationFrame` fires at the display's refresh rate, so counting callbacks on a 144 Hz panel would report 144 and a naive 60-frames-per-second assertion would pass for the wrong reason. Derive the frame rate from consecutive timestamp deltas and report the panel rate beside it.
- **The page never logs a tick payload.** Positions are the product, not a signal; a signal carries counts and identifiers only.
