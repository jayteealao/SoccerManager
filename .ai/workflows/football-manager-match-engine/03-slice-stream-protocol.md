---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: stream-protocol
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: m
depends-on: [engine-core]
tags: [engine, protocol, socket, fixture]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-engine-core.md, 03-slice-viewer-pitch.md, 03-slice-viewer-match-day.md, 03-slice-integration.md]
  plan: 04-plan-stream-protocol.md
  implement: 05-implement-stream-protocol.md
---

# Slice: Stream Protocol and Fixture Harness

## The Slice

Engine core produces ticks into a file. The shape needs a live tick stream over a local socket, an event stream, a statistics record, a command channel, and, by the force-scope rule, a recorded fixture with a replayer so viewer slices verify before the engine is complete.

This slice decides the wire encoding (open question U-1), builds the local socket server inside the engine process, defines the message set, records one full match as a fixture file, and ships a mock server that replays it. It also carries owner and match identifiers on every stream header.

Viewer-pitch consumes this slice next and reaches the visible milestone. The top risk is choosing an encoding that cannot carry 50 ticks per second at 8x playback; the acceptance criteria measure throughput, not just correctness.

## Goal

A documented, versioned protocol that streams ticks, events, and statistics from the engine to a page and carries commands back, plus a replayable fixture.

## Why This Slice Exists

The viewer cannot be verified without a stream, and the engine will not be complete for several slices. The fixture harness is the shape's named prerequisite for every viewer acceptance criterion.

## Scope

In:
- Wire encoding decision and reference document (U-1): a compact binary tick frame with delta encoding and a JSON control channel is the candidate; plan decides with a measurement.
- Local socket server in the engine process (named mechanism per the shape: **local socket server**); one client per match in v1.
- Messages: hello with engine version and identifiers, tick frames, event records, statistics record, snapshot notice, commands (start, pause, set speed, queue change), acknowledgements with reasons.
- Backpressure: the server produces ahead of the client up to a bounded buffer (named mechanism: **bounded producer buffer**, replaces unbounded queuing so memory stays flat).
- Fixture recorder: `engine-cli record` writes a full match stream to a file.
- Mock server: replays a fixture at a chosen speed over the same socket protocol.
- Protocol reference document (Documentation Plan: reference).

Out:
- Rendering: `viewer-pitch`.
- Observability sink beyond the file and socket: `/wf observability init` (U-2).

## Acceptance Criteria

- Given the engine simulates with the socket server on, When a client connects, Then it receives a hello with engine version, owner identifier, and match identifier before the first tick frame.
  <!-- observable: false — a Rust client test asserts message order -->
- Given a client reads 8 times faster than real time, When the engine streams a full match, Then every one of the 270,000 ticks arrives in order with no gap, and the server's buffer never exceeds its bound.
  <!-- observable: false — client test counts ticks and reads the buffer gauge -->
- Given a client reads slower than the engine produces, Then the server pauses production at the buffer bound and resumes when the client drains; no tick is dropped.
  <!-- observable: false — throttled client test -->
- Given `engine-cli record --seed 7 --out fixture.bin`, Then the fixture holds the full stream, and Given the mock server replays it, Then a client receives a byte-identical sequence.
  <!-- observable: false — cargo test compares streams -->
- Given a `queue change` command with an unknown change type, Then the server answers with a rejection naming the type; and Given a valid command, Then it answers with an acknowledgement carrying a queue identifier.
  <!-- observable: false — client test -->
- Given the protocol reference document, Then every message in the implementation appears in the document with its fields (a test enumerates message types against the document's list).
  <!-- observable: false — cargo test over the document -->

## Dependencies on Other Slices

- `engine-core`: the tick loop and the command line.

## Risks

- Encoding choice (U-1) made without measurement: plan runs a throughput spike before fixing it.
- Windows socket behavior differs from POSIX: tests run on the reference laptop; the choice of transport (TCP on loopback versus named pipe) is plan's, with TCP on loopback as the default because the browser can reach it.
