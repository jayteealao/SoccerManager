---
schema: sdlc/v1
type: augmentation
augmentation-type: instrument
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: stream-protocol
instrumentation-framework: "serde_json JSON Lines per .ai/observability.md plan-version 1; tracing to stderr for diagnostics"
dark-paths-found: 5
signals-designed: 8
pii-warnings: false
status: ready
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T11:28:51Z"
revision-count: 2
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
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-stream-protocol.md
  contract: ../../observability.md
  prior: history/04b-instrument-1.md
---

# Instrumentation: Stream Protocol and Fixture Harness

## The Instrumentation

Two slices have filled the two record kinds the engine can emit: `match-stats` from `simulate` and `run-report` from `bench`, with eleven signals between them, and both prior records are byte-copied under `history/`. The contract has been waiting for this slice: it names the local socket eight times, reserves `viewer.socket_drops`, and defines a `match-event` record the engine has never written, because until now nothing structured happened inside a match.

Eight signals are designed. Three carry the connection itself (`socket.listening`, `socket.client`, `socket.backpressure`), one carries every refused handshake (`socket.refused`), one opens the `match-event` record kind with the three event types the engine can produce today, one records each queued change and its verdict, and two cover the fixture path (`fixture.recorded`, `fixture.replayed`). Five dark paths are covered: a client that connects and reads nothing, a client refused by the Origin check with no trace, a match-event stream that stops without a full-time record, a queued change that is accepted and never applied, and a replay that diverges from its fixture. No signal carries a person, an absolute path, or free text; the port and the client address are loopback values the contract treats as machine-local.

Implement folds these into the server, the codec, and the two new commands; verify exercises each against a real loopback client. The top open risk is `change.queued_tick` against a queue identifier: the contract reserves `change.queued_tick`, `change.applied_tick`, and `change.rejected_reason`, but no key for the identifier the acknowledgement must return, so this slice emits `change.queue_id` as an additive extra and the observability audit settles it.

## 1. Current state

| File | Quality | Existing signals | Dark paths |
|------|---------|-----------------|------------|
| `crates/stream/src/server.rs` (planned) | dark | none | a client connects, completes the handshake, and never reads; a refused Origin leaves no trace |
| `crates/stream/src/session.rs` (planned) | dark | none | the producer pauses at the bound and nothing states how long or how often |
| `crates/stream/src/record.rs` (planned) | dark | none | a fixture is written or replayed with no statement of what it holds |
| `crates/protocol/src/event.rs` (planned) | dark | none | a match ends without a full-time event and a consumer cannot tell a finished match from a dropped one |
| `crates/protocol/src/command.rs` (planned) | dark | none | a change is queued, acknowledged, and never applied (the contract names this dark path) |
| `crates/engine/src/observe/mod.rs` | good | `match-stats`, `run-report` with the contract keys | none new |
| `crates/engine/src/record.rs` | good | `tickfile.header`, `tickfile.trailer` | none new |
| `crates/engine/src/data/mod.rs` | good | `content.loaded`, `content.refused` | none new |

Summary: 5 dark paths found across 8 files (5 planned). Framework: serde_json JSON Lines per the contract; tracing to stderr for diagnostics.

## 2. Instrumentation plan

| File | Function/path | Signal type | Signal name | Key fields | Rationale |
|------|--------------|-------------|-------------|------------|-----------|
| `crates/stream/src/server.rs` | `Server::bind()` | event (tracing info) | `socket.listening` | `port`, `protocol_version`, `match.id` | The page and the operator learn the port the run chose |
| `crates/stream/src/server.rs` | `Server::accept()` | event (tracing info) | `socket.client` | `origin`, `protocol_version`, `match.id` | A connection is visible with the page that opened it |
| `crates/stream/src/server.rs` | `Server::accept()` | log (tracing warn) | `socket.refused` | `origin`, `reason` | A refused handshake states which page and which rule |
| `crates/stream/src/session.rs` | `Session::push()` | event (tracing info) | `socket.backpressure` | `tick`, `bound`, `paused_ms`, `resumes` | The pause path is visible instead of looking like a slow engine |
| `crates/protocol/src/event.rs` | `MatchEvent::emit()` | event (record: `match-event`) | `match-event.engine` | `record.kind`, `schema.version`, `match.id`, `owner.id`, `tick`, `minute`, `event.type`, `team.id`, `home.score`, `away.score` | Opens the fourth record kind with kick-off, goal, and full-time |
| `crates/protocol/src/command.rs` | `Queue::accept()` / `reject()` | event (record: `match-event`) | `match-event.change` | `event.type` `tactics-change`, `change.kind`, `change.queued_tick`, `change.queue_id`, `change.rejected_reason` | Every queued change and every rejection is on the record |
| `crates/stream/src/record.rs` | `Recorder::finish()` | event (tracing info) | `fixture.recorded` | `path` (relative), `frames`, `ticks`, `bytes`, `hash` | A fixture states what it holds and hashes to prove it |
| `crates/stream/src/replay.rs` | `Replayer::run()` | event (tracing info) | `fixture.replayed` | `path` (relative), `frames`, `speed`, `hash` | A replay states the fixture and the speed it ran |

## 3. Signal designs

```rust
// crates/stream/src/server.rs — bind and accept
tracing::info!(signal = "socket.listening", port, protocol_version = PROTOCOL_VERSION, match_id = %match_id);
tracing::info!(signal = "socket.client", origin = %origin, protocol_version = PROTOCOL_VERSION, match_id = %match_id);
tracing::warn!(signal = "socket.refused", origin = %origin, reason = %reason);

// crates/stream/src/session.rs — the bounded producer buffer
tracing::info!(signal = "socket.backpressure", tick, bound = BUFFER_TICKS, paused_ms, resumes);

// crates/protocol/src/event.rs — the match-event record, one JSON Lines row per event
#[serde(rename = "event.type")] pub event_type: EventType,   // kick-off | goal | full-time in this slice
#[serde(rename = "team.id")]    pub team_id: Option<String>,
#[serde(rename = "home.score")] pub home_score: u32,
#[serde(rename = "away.score")] pub away_score: u32,
// tick and minute are plain keys per the contract; the envelope adds record.kind, schema.version, owner.id.

// crates/protocol/src/command.rs — the queue verdict, also a match-event row
#[serde(rename = "change.kind")]            pub change_kind: ChangeKind,      // tactics | substitution
#[serde(rename = "change.queued_tick")]     pub queued_tick: u32,
#[serde(rename = "change.queue_id")]        pub queue_id: String,             // additive extra, see the risk
#[serde(rename = "change.rejected_reason")] pub rejected_reason: Option<String>,

// crates/stream/src/record.rs and replay.rs — the fixture path
tracing::info!(signal = "fixture.recorded", path = %relative, frames, ticks, bytes, hash = %hash12);
tracing::info!(signal = "fixture.replayed", path = %relative, frames, speed, hash = %hash12);
```

Types: `port` is an integer; `origin` is the header string or the literal `none`; `paused_ms` and `resumes` are integers; `speed` is a float with one decimal; `hash` is 12 lower-case hex characters from the same helper the content hash uses. Every `match-event` row is one JSON Lines row written to `SM_DATA_DIR/matches/<match.id>/events.jsonl` and also sent over the socket, exactly as the contract prescribes.

## 4. PII & security notes

No PII concerns identified. `origin` is a page origin on the local machine, never a remote address; the server binds `127.0.0.1` only, so no external address can appear. `owner.id` is opaque per the contract and enters records only, never the stderr lines. Fixture paths are relative to the data folder. The port file `SM_DATA_DIR/engine.port` holds one integer and no secret, because the Origin allowlist, not a token, is the guard (PO Q9).

## 5. Implementation notes

- `crates/protocol` holds the record structs and carries no tracing calls: the codec must stay free of input and output so the document-enumeration test and a later WebAssembly build can use it.
- `match-event` rows append to `events.jsonl` through one writer opened at kick-off, so a crash keeps every event up to the last write; the contract requires this file beside `stats.json`.
- `socket.backpressure` is emitted once per pause, not once per blocked tick, so a slow client cannot flood stderr.
- The restart event retires the `sdlc-debt:` marker at `crates/engine/src/validate.rs:77`: once the event stream names a kick-off, the validator reads the flag instead of guessing from a ball jump above two metres.
- `change.queue_id` is outside the Block A vocabulary. Emit it as an additive extra, exactly as `content.hash` and `teams` were, and let `/wf observability audit` settle whether the identifier is a key of its own or `change.queued_tick` in another form.
