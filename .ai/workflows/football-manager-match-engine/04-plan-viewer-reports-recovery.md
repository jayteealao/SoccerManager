---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: viewer-reports-recovery
status: complete
stage-number: 4
created-at: "2026-09-22T22:24:14Z"
updated-at: "2026-09-22T22:24:14Z"
metric-files-to-touch: 35
metric-step-count: 22
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [viewer, reports, replay, recovery, launcher, error-states]
stack-source: confirmed
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-viewer-reports-recovery.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-viewer-match-day.md, 04-plan-viewer-lineup-tactics.md]
  design: 02b-design.md
  contract: 02c-craft.md
  steer: steer.md
  implement: 05-implement-viewer-reports-recovery.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine viewer-reports-recovery"
---

# Plan: Reports, Replay Files, and Recovery States

## The Plan

The match-rules slice left a snapshot file that is replaced at every stoppage and a `resume` command that continues a match from it. The viewer slice left a page that the engine process serves itself, and that names a closed socket "Stream ended" with no recovery action. Research found the fact that decides the whole slice: a browser page cannot start a process, and the page's own server is the engine process. When the engine dies, the page loses its server, its socket, and every way to ask for a restart. The product owner's answer to Round 4 Q17 says that the viewer restarts the engine. So something that survives the engine must hold the page and start the engine again. This plan adds that part as `engine-cli launch`: a launcher that serves the page and runs the engine as a child worker. It reports the worker's state in the existing `engine.json` response and accepts two same-origin POST requests, restart and abandon. The launcher is also where the first-run state comes from, because only a process separate from the engine can report that the engine binary is missing at its configured path.

Four decisions keep the blast radius small. First, the protocol does not change: a reconnect repeats the same `hello`, and the first tick frame after it is a keyframe one tick past the resume stoppage. That keyframe tells the page where to truncate, so no new message is needed. Second, the replay file is the existing `.smfx` fixture layout. The page keeps the wire bytes it received and writes them unchanged, so there is no new file format and no second position encoder. The engine's own `replay` command accepts the saved file. Third, `serve` keeps today's behaviour unless it gets `--reconnect-wait`, so the existing test for a closed page still passes. Fourth, the snapshot file keeps its name, layout, and location. It is written only when the socket has already written the frames past that stoppage, so after a crash the page always holds every tick and event up to the resume point.

Implement changes 35 files, 14 of them new, in 22 steps, with the engine side first and the browser drives last. This slice is planned before the two sibling viewer slices it depends on, and both of those plans are waiting for product-owner input. Steps 17 to 19 therefore bind to the event list and the `main.mjs` that those slices land, and an auto-review of this plan must run before implementation. The top open risk is a resumed match whose score differs from the one the manager saw. The gated snapshot write is the mitigation, and the launcher test kills a real worker and compares scores.

## Current State

- `crates/engine-cli/src/serve.rs:67-68` calls `server.accept` once. After the session ends, the run ends. `crates/engine-cli/tests/stream_cli.rs:139-168` asserts this: a page that drops its socket "sends no close frame" and the run ends without an error. `crates/stream/src/session.rs:315-326` sees both a close frame and an abnormal drop, and stops the gate either way.
- `serve.rs:21-24` creates a new match identity with `MatchId::now(opts.seed)`. The launcher cannot know the match identifier, and so cannot know the snapshot path, unless it passes the timestamp in.
- `crates/engine-cli/src/resume.rs:18-60` shows the full resume build: `Snapshot::read`, a config from the snapshot's seed and minutes, `MatchId { seed, millis: snapshot.match_millis }`, the owner from the snapshot, and `Simulation::from_snapshot`. It does not stream. It plays to full time and prints statistics.
- `crates/engine/src/snapshot.rs:241-298` `SnapshotSink` writes `SM_DATA_DIR/matches/<match.id>/snapshot.smsn` atomically at every stoppage. `Snapshot::capture` (`:65`), `tick()` (`:88`), `read` (`:192`), and `write_atomic` (`:202`) are all `pub`. The snapshot holds every field "except the scratch buffer and the events already handed out" (`:9-11`). A resumed match therefore does not emit the events of the stoppage tick again.
- `crates/engine-cli/src/stream_run.rs:47-67` sends each tick, then calls `on_stoppage`, then routes that tick's events. So the tick-T frame and the tick-T snapshot come before the tick-T events on the wire. The producer buffer holds up to 500 ticks (`docs/reference/protocol.md` Backpressure), so the snapshot on disk can be up to 10 seconds of play ahead of the page.
- `crates/stream/src/events.rs:52-60` `EventWriter::open` creates `events.jsonl`. If a restarted worker opened it the same way, it would erase every event from before the crash.
- `crates/engine-cli/src/web.rs:39,94-106` serves one generated path, `/engine.json`, which carries `socket.port` and `protocol.version`. It answers GET and HEAD only (`:87-93`). `docs/reference/protocol.md:205-207` still shows `"protocol.version": 1`, which is stale because the build speaks version 2.
- `web/main.mjs:258-267` fetches `engine.json` once and, on any close, shows "The match is no longer live" in the lag notice with the `error` kind. `02c-craft.md` state coverage stubs the canvas and page error states and gives this slice the job of completing them. Mock fidelity item 10, a skeleton with a step list and never a spinner, is owned by this slice (`02c-craft.md:77`).
- `web/history.mjs` stores absolute `Int16Array` ticks indexed by `tick - firstTick` and has no truncation. `web/stoppages.mjs` has no truncation.
- The `.smfx` layout (`crates/stream/src/record.rs:15-160`) is a 32-byte header (magic `SMFX`, `u16` protocol version, `u64` match milliseconds, `u32` frames, `u32` ticks, `u64` seed), then 9-byte entries (kind, `u32` tick, `u32` length) followed by the payloads, then a 16-byte trailer (magic `SMFE`, `u32` frames, and the first 6 bytes of the SHA-256 over all payloads). `read_fixture` fails closed on every mismatch. The local `fixture.smfx` is 16,529,205 bytes for 90 minutes. `.gitignore` ignores `*.smfx`.
- `crates/stream/src/replay.rs:30-40` refuses a fixture whose first entry is not the recorded `hello`. A page-written file must therefore start with the first `hello`.
- The `stats` message is documented as sent once at full time, and the events carry both scores (`crates/protocol/src/lib.rs:125-160`). The page can rebuild the score at any tick from the events.
- `04-plan-viewer-match-day.md` (awaiting input) adds `web/match-state.mjs`, a pure reducer from events to state at a tick, and `web/feed.mjs`. Its A-1 releases events by the rendered tick, and its A-10 reads a close after full time as "Full time". `04-plan-viewer-lineup-tactics.md` (awaiting input) also changes `web/main.mjs`.
- Test baseline this run: `node --test web/tests/*.test.mjs` passes 46 of 46. `cargo test -p stream -p engine-cli` passes 46 of 46: `engine-cli` 3 + 6 + 2 + 5 + 2, and `stream` 16 + 1 + 2 + 5 + 3 + 1. The directory form `node --test web/tests/` fails on this machine, and the glob form is the one to use.

## Simplicity Ladder

| Capability | Rung | Evidence / recommendation |
|---|---|---|
| Start, watch, and kill the engine worker | rung 1 stdlib | `std::process::Command`, `Child::wait`, and `Child::kill`. No process-supervision crate. |
| Launcher HTTP page server | rung 3 reuse | `crates/engine-cli/src/web.rs` `start()` / `answer()`. Reuse with modification: take a status source in place of the fixed port. Backward compatible for `serve --web`. |
| Resume a match from a snapshot | rung 3 reuse | `Snapshot::read` and `Simulation::from_snapshot`, following `resume.rs:18-60`. Reuse as-is. |
| Snapshot writing | rung 3 reuse | `Snapshot::capture` and `Snapshot::write_atomic`. Reuse as-is inside a new gating sink. `SnapshotSink` stays unchanged for `simulate` and `resume`. |
| Reconnect on the same port | rung 3 reuse | `Server::accept` (`server.rs:85-93`) already loops. Reuse as-is. |
| Replay file format | rung 3 reuse | `.smfx` from `crates/stream/src/record.rs`. Reuse the layout. The page gets a byte-level writer and reader, not a second position encoder. |
| SHA-256 in the page | rung 2 native-platform | `crypto.subtle.digest`. It is available in a secure context, and a loopback origin is one (MDN Secure Contexts). Node 22 has the same global. |
| Save a file from the page | rung 2 native-platform | `URL.createObjectURL(blob)` plus `<a download>`. `download` works for same-origin and `blob:` URLs (MDN `<a>`). |
| Open a file in the page | rung 2 native-platform | `<input type="file" accept=".smfx">` and `File.arrayBuffer()`. |
| Report surface | rung 2 native-platform | `<dialog>` with `showModal()` for focus trapping and Escape handling. No dialog library. |
| Report counts | rung 3 reuse | The event list and reducer that `viewer-match-day` adds (`web/match-state.mjs`), so the report counts the same events the feed shows. If that module lands under another name, bind to it. Never keep a second event store. |
| Launcher state machine, recovery model, frame store | rung 4 new-code | Nothing in the repository supervises a process or models recovery states. The frame store is about 60 lines over a growing `Uint8Array`, like `history.mjs`. |

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist, and there is no `.ai/sdlc-config.json`, so no global learnings directory is configured.

Repeat-deferral tripwire: `00-index.md` `runtime-evidence-deferrals` is `[]`, so it does not fire.

## Likely Files / Areas to Touch

- `crates/engine-cli/src/launch.rs` (new): the launcher and its worker state machine.
- `crates/engine-cli/src/cli.rs`, `main.rs`: `launch` and the new `serve` options.
- `crates/engine-cli/src/serve.rs`: resume build, reconnect loop, and gated snapshots.
- `crates/engine-cli/src/web.rs`: status source, the new `engine.json` keys, and the two POST routes.
- `crates/engine-cli/src/stream_run.rs`: `Driven.client_gone`.
- `crates/stream/src/session.rs`, `lib.rs`: `sent_tick`, `SessionEnd`, and the drop seam.
- `crates/stream/src/snapshots.rs` (new): `GatedSnapshots`.
- `crates/stream/src/events.rs`: `EventWriter::resume`.
- `crates/stream/tests/reconnect.rs` (new). `crates/engine-cli/tests/launch.rs` (new). `stream_cli.rs`, `web_cli.rs`, `cli_args.rs` (modified).
- `web/replay-file.mjs`, `recovery.mjs`, `recovery-panel.mjs`, `launcher.mjs`, `report.mjs`, `components/surfaces.css` (new).
- `web/main.mjs`, `socket.mjs`, `history.mjs`, `stoppages.mjs`, `index.html` (modified).
- `web/tests/replay-file.test.mjs`, `recovery.test.mjs`, `report.test.mjs`, `data/one-minute.smfx` (new). `history.test.mjs`, `stoppages.test.mjs` (modified).
- `docs/reference/protocol.md`, `README.md`, `.gitignore` (modified).

## Proposed Change Strategy

Three processes take part, and each has one job. The launcher (`engine-cli launch`) owns the page server and the lifetime of the worker. The worker (`engine-cli serve`) owns the match and the socket. The page owns what the manager has seen. The page never holds match state that the engine needs. On a restart it sends nothing but an HTTP POST with no body, which honours the slice's "without a second copy of the match state in the browser".

**Socket drop, engine alive.** When the connection drops without a close frame (WebSocket code 1006, "closed abnormally … with no close frame", MDN CloseEvent), the worker started with `--reconnect-wait` keeps its listener. It rebuilds the simulation from the newest retained snapshot at or below the tick that its socket has written, and waits for the next client. The page sees an abnormal closure. It polls `engine.json`, finds `running`, and reconnects with backoff. It shows a "Reconnecting" notice with its state word, never a prompt. It gets the same `hello`, and it truncates its stores to T when the first keyframe arrives at T+1. A close frame (1000, or 1001 "navigating away") ends the run exactly as today.

**Engine crash.** The launcher's watcher sees a nonzero exit while a match is running and reports `crashed` with the exit code and the snapshot tick. The page shows the error panel: the failure named in words, "Restart from mm:ss", and "Abandon". Restart POSTs to the launcher. The launcher reads the snapshot with `Snapshot::read`. If it is valid, the launcher starts `serve --resume <snapshot>` and reports `running` with the new port. If it is corrupt, the launcher reports `refused` with the engine's own reason, and the page offers abandon only.

**Why the snapshot write is gated.** Events are not stored in a snapshot, and the tick-T events travel after the tick-T frame. If a snapshot is written at T and the page crashes before it has the tick-T events, the page would resume with a lost goal and the wrong score. `GatedSnapshots` captures at every stoppage, exactly as before, and persists a snapshot only once the socket thread has written tick T+1. Channel order puts the tick-T events before that frame, so the page holds them. NFR ranking: none is invoked. The mechanism serves AC-26 and the slice criterion "same score and clock", and it narrows no charter commitment.

**Reports** read the page's single event list (the one `viewer-match-day` adds). They open when the rendered tick, not the arrival tick, reaches the half-time or full-time event, which is the same release rule as viewer-match-day A-1. **Replay save** writes the stored frames as `.smfx`. **Replay open** feeds the stored frames through the same `onHello`, `onTick`, and `onMessage` path with no socket, so playback and rewind are the same code as live play.

## Step-by-Step Plan

1. **Session end and sent tick.** In `crates/stream/src/session.rs`, add `sent_tick: AtomicU32` to `MatchState` and set it after each tick frame is written successfully. Carry `(Option<u32>, Message)` in the outbox. Make `pump` return `SessionEnd::{Done, Closed, Dropped}`: a close frame is `Closed`, and a `peer_gone` read or write error is `Dropped`. Add `drop_at: Option<u32>` to `SessionConfig`. Once a tick at or past it is written, call `socket.get_ref().shutdown(Shutdown::Both)` and return `Dropped`, only once per worker. Keep `Session::finish` returning the end kind. Re-export from `lib.rs`.
2. **Gated snapshots.** Add `crates/stream/src/snapshots.rs` with `GatedSnapshots::new(data_dir, match_id, owner, millis, state)`, implementing `TickSink`. `on_stoppage` captures with `Snapshot::capture` into a ring of 16. `on_tick` writes the newest captured snapshot whose tick is below `state.sent_tick()` and that is not yet written, using `write_atomic` to `matches/<id>/snapshot.smsn`. Emit `snapshot.written` exactly as `SnapshotSink` does, plus `lag_ticks`. `newest_at_or_below(tick) -> Option<&Snapshot>` serves the reconnect path. Keep a `writes` counter for the statistics record.
3. **Event file resume.** In `crates/stream/src/events.rs`, add `EventWriter::resume(data_dir, match_id, tick)`. It reads the existing rows, keeps those whose `tick` is at or below the resume tick, writes them to a temporary file, renames it over `events.jsonl`, and opens it for append. Emit `events.resumed { kept, dropped }`.
4. **Stream tests.** Add `crates/stream/tests/reconnect.rs`. Drop the TCP stream without a close frame and assert `Dropped`. Send a close frame and assert `Closed`. Assert that `sent_tick` follows written frames. Assert that `GatedSnapshots` never persists beyond `sent_tick` and keeps at most 16. Assert that `EventWriter::resume` keeps exactly the rows at or below the tick. Use the existing `stream::client::Client` and `tests/common`.
5. **Command line.** In `crates/engine-cli/src/cli.rs`, add `Launch(LaunchOpts)` with `--seed`, `--minutes`, `--team-a`, `--team-b`, `--web DIR` (required), `--engine FILE` (default `SM_ENGINE_PATH`, else `std::env::current_exe()`), and hidden `--drop-client-at TICK`. Add to `ServeOpts`: `--resume FILE`, `--reconnect-wait SECONDS` (default 0), hidden `--match-millis MS`, and hidden `--drop-client-at TICK`. Keep help lines under 80 columns. Wire `mod launch` in `main.rs`.
6. **Serve builds fresh or resumed.** In `serve.rs`, factor `open_match(opts) -> Opened { sim, identity, hello, event_writer }`. For a fresh match, use `MatchId { seed, millis: opts.match_millis.unwrap_or(now) }`. For `--resume`, follow `resume.rs:18-60`: take the seed, minutes, owner, and millis from the snapshot. Log a refusal as `snapshot.refused` and exit with code 1 before printing a port, so the launcher can tell a refusal from a crash. Open the event writer with `EventWriter::resume` at `snapshot.tick()`. The `hello` is built the same way in both cases, so the page sees the same `match.id`.
7. **Serve reconnect loop.** Replace the single `accept` with a loop. Start the session with `drop_at` on the first session only. Drive with `FanoutSink(session.sink(), ticks_file) + GatedSnapshots`. On full time, or on `SessionEnd::Closed`, end as today (exit codes 0 and 2 unchanged). On `Dropped` with `--reconnect-wait` above 0, emit `socket.dropped { tick, sent_tick }`. Pick `gated.newest_at_or_below(sent_tick)`. If there is none, the drop came before the first stoppage: rebuild a fresh simulation with the same seed and identity. Otherwise call `Simulation::from_snapshot`, then `EventWriter::resume`. Set the `accept` deadline to the wait with a non-blocking poll on the listener, emit `socket.reconnected { resume_tick }`, and continue. Add `client_gone` to `Driven` in `stream_run.rs`.
8. **Page server status and routes.** In `web.rs`, change `start(dir, socket_port)` to `start(dir, status: Arc<dyn StatusSource>)`. `serve --web` passes a fixed `running` source, so behaviour is unchanged. `/engine.json` keeps `socket.port` (present only when running) and `protocol.version`, and adds `engine.state`, `engine.path`, `engine.reason`, `engine.pid`, `snapshot.tick`, and `match.id`. Add `POST /engine/restart` and `POST /engine/abandon`. They require an `Origin` header equal to `http://127.0.0.1:<page port>`, else 403. They ignore the body and answer 202 with the new status. Any other POST answers 405. The same-origin guard reads the header block that `read_request_line` currently drains, so keep `Origin` while draining. Keep the 8 KiB request cap.
9. **Launcher.** Add `crates/engine-cli/src/launch.rs`. Resolve the engine path. If it is missing, set the state to `not-found { path }` and serve the page anyway. Otherwise start `serve --seed --minutes [--team-a --team-b] [--content-dir] --reconnect-wait 120 --match-millis <now>` with stdout and stderr piped. Its first stdout line is the port, which sets `running { port, pid }`. A stderr reader forwards every line to the launcher's stderr and keeps the last `error:` line. A watcher thread handles the exit: code 0 means `finished`; exit before a port means `refused { reason: last error line }`; any other exit means `crashed { code }`, with `snapshot.tick` read from `matches/<id>/snapshot.smsn` when that file exists. Restart runs `Snapshot::read`. On an error it sets `refused { reason }`. On success it starts `serve --resume <path> --reconnect-wait 120` and sets `starting`. Abandon kills a live worker and sets `abandoned`. Print the page address, and nothing else, on stdout. Emit `launch.worker_started { pid, resume, snapshot.tick }`, `launch.worker_exited { code, state }`, `launch.restart`, `launch.refused { reason }`, `launch.abandoned`, and `launch.not_found { path }`.
10. **Command-line tests.** Add `crates/engine-cli/tests/launch.rs` with five cases: (a) `--engine` pointing at a missing path gives `engine.json` state `not-found` and the path; (b) kill the worker after `snapshot.tick` becomes non-zero, using `taskkill /F /PID` on Windows and `kill -9` elsewhere; the state goes to `crashed`, a POST restart gives `running` on a new port with the same `match.id`, the first tick is `snapshot.tick + 1`, and the scores in the last event at or before the snapshot tick equal those the client saw before the kill; (c) overwrite one byte of the snapshot and POST restart; the state is `refused` and the reason names the checksum; (d) abandon gives `abandoned` and no live pid; (e) a POST with `Origin: http://example.com` gives 403. In `stream_cli.rs`, add a case with `serve --reconnect-wait 30 --drop-client-at 3000` in which the client reconnects on the same port and gets the same `hello`. In `web_cli.rs`, add the new keys and 405 on POST under `serve --web`. In `cli_args.rs`, check the width of `launch --help` and that the hidden flags do not appear.
11. **Documentation.** In `docs/reference/protocol.md`, extend the `engine.json` section with the keys, states, both routes, and the origin rule, and correct the version example to 2. Add a "Reconnection" section: the same `hello`, and the first frame is a keyframe at the resume stoppage plus one. State that `PROTOCOL_VERSION` stays 2 because no message or field changes. Keep `crates/protocol/tests/document.rs` green, because no `MESSAGES` entry changes. In `README.md`, cover `launch`, crash and drop behaviour, replay files, `--engine`, and `SM_ENGINE_PATH`.
12. **Golden replay file.** Run `engine-cli record --seed 42 --minutes 1 --out web/tests/data/one-minute.smfx` and add `!web/tests/data/*.smfx` to `.gitignore`. Record the size in the implement record. It is expected to be near 185 KB, from 16.5 MB for 90 minutes. Regenerate it whenever `PROTOCOL_VERSION` changes.
13. **Replay file module.** Add `web/replay-file.mjs`. `FrameStore` appends every text and binary payload's bytes, with `kind`, `tick`, and `offset` in typed index arrays that grow by doubling. `truncate(tick)` drops entries past the tick. `writeReplay(store) -> Promise<Uint8Array>` writes the exact layout from `record.rs`. Take the seed from the first 16 hex characters of `match.id` as a `BigInt` and the milliseconds from its suffix, because a JSON number loses a 64-bit seed. `readReplay(bytes)` fails closed on magic, version, header and trailer counts, truncation, and the hash, each with a named reason. Store only the first `hello`, all ticks, events, and `stats`, and never `ack` or `reject`. Tests in `web/tests/replay-file.test.mjs`: the golden file reads and writes back byte for byte, and the four corruptions are refused by name.
14. **Truncation.** `History.truncate(tick)` ends the history at the tick. When the resume tick lies past `newestTick`, record the gap. `tickAt` reports the gap ticks as absent, and the scheduler skips them. It never draws them. Add `Stoppages.truncate(tick)`. Extend `history.test.mjs` and `stoppages.test.mjs`.
15. **Socket close kinds.** In `web/socket.mjs`, pass `{ clean: event.wasClean && event.code !== 1006, code }` to `onClose`, and add `onRaw(data)` so the frame store sees every frame before decoding. The keys of `viewer.socket_drops` are unchanged.
16. **Recovery models and launcher client.** Add `web/recovery.mjs` with the pure functions `loadingSteps`, `panelModel`, and `backoff`, as in the file table. The error title names the failure: "The engine stopped (exit code N)", "The saved match could not be read: <reason>", "Touchline could not find the match engine". Add `web/launcher.mjs` for status, restart, abandon, and poll. When `engine.json` cannot be fetched after an abnormal close, there is no launcher. The panel then says the engine stopped and offers abandon only, with the text "Start with engine-cli launch to restart from the last stoppage." Tests in `web/tests/recovery.test.mjs` cover the corrupt-snapshot criterion with the engine's exact message shape, `snapshot refused: <path>: <reason>` (`crates/engine/src/error.rs:38`), and every other state.
17. **Surfaces.** In `web/index.html` and the new `web/components/surfaces.css`: put the step list inside the existing skeleton, with three steps ("Starting the engine", "Connecting to the match", "Waiting for kick-off"), each with the state word Done, In progress, or Waiting (inventory item 10, and no spinner). Put the first-run panel in the pitch panel, with the path in `<code>`, the build instruction, and "Open a replay". Put the error panel in the same place, with `role="alert"`, the failure named in words, and Restart and Abandon as `match-control` buttons. Add the report `<dialog>`. Use `--tl-` tokens only. Report titles use Barlow Condensed 700 and counts use tabular figures. `--tl-danger` always sits next to the word "Error", and `--tl-warning` next to "Reconnecting". Follow `design/typeset.md`, `design/harden.md`, `design/animate.md`, and `design/polish.md`, per the `02c-craft.md` references.
18. **Reports.** Add `web/report.mjs`. `reportModel(events, uptoTick, teams)` counts per team: goals, yellow cards, red cards (including second yellows), fouls (with or without advantage), corners, offsides, throw-ins, goal kicks, free kicks, and penalties. It also lists goals and cards with their minutes. Build it on the event list and reducer from `viewer-match-day`. `showReport('half-time' | 'full-time')` fills the dialog. At half-time it pauses the page's playback, and "Continue" resumes it. At full time it offers "Save replay" and "Open a replay". Each report opens once per match, when the rendered tick first reaches the event tick. Tests in `web/tests/report.test.mjs` compare each count with a direct filter over the same events.
19. **Page wiring.** Changes to `web/main.mjs`:
    - Start by polling the launcher until `running`, `not-found`, or a terminal state, and drive the step list.
    - On an abnormal close, poll the launcher. If it reports `running`, reconnect with `backoff`, showing the "Reconnecting" notice. If it reports `crashed` or `refused`, or cannot be fetched, show the panel.
    - On a `hello` whose `match.id` equals the current one, keep every store. When the first tick T+1 arrives, truncate the history, stoppages, event list, and frame store to T. If the playhead is past T, seek to T. Emit `viewer.resumed { from_tick, to_tick, gap }`.
    - Run the reports by the rendered tick.
    - Save: `writeReplay` → `Blob` → `<a download="touchline-<match.id>.smfx">`, then `viewer.replay_saved { bytes, frames, ticks, hash }`.
    - Open: file input → `readReplay`. Replace the stores and feed the frames synchronously, with no socket and without `playback.noteArrival`, then `viewer.replay_loaded`.
    - Extend the test hook with read-only readers: `recovery()` (panel kind, title, actions, engine state), `report()` (kind, counts, open), `replay()` (last saved name, size, and hash, plus `lastSavedBytes()` returning a copy), and `events()`. Keep it free of setters.
20. **Signals.** Page signals: `viewer.reconnecting { attempt, delay_ms }`, `viewer.resumed`, `viewer.resume_gap { ticks }`, `viewer.recovery_panel { kind, reason }`, `viewer.report_shown { kind, tick }`, `viewer.replay_saved`, `viewer.replay_loaded { frames, ticks }`, and `viewer.replay_refused { reason }`, all through `signal()`. Engine signals are those named in steps 2, 3, 7, and 9, emitted with `tracing` in the existing key style. Record them in the implement record for the observability audit. The shared instrumentation artifact is not rewritten by this slice (Assumption A-12).
21. **Full check.** Search the changed source for workflow vocabulary, such as slice names and stage words, and rewrite any hit in product language. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `node --test "web/tests/*.test.mjs"`. Run the stream benchmark `engine-cli bench --seed 42 --matches 5 --stream --json` in release mode and compare it with the numbers in `05c-benchmark.md` current at implement time. This slice adds one atomic store per written frame on the socket thread, and a regression beyond the tripwire is reported, not waived.
22. **Browser drives.** Run the five drives in `## Test / Verification Plan` with the in-app browser against `engine-cli launch`, and save the evidence under `verify-evidence/viewer-reports-recovery/`.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| Engine killed mid-match → failure shown, restart offered; restart → play resumes at that stoppage with the same score and clock | Claude_Browser drive plus `taskkill /F /PID` (web-2) | Windows 11, engine built with match-rules, seed 42 — yes (this machine; `taskkill` is in the OS) | `engine-cli launch` (step 9); `engine.pid` and `snapshot.tick` in `engine.json` (step 8); `window.__touchline.recovery()` and `events()` (step 19) | `crates/engine-cli/tests/launch.rs` case (b) (automated, same kill) → pre-registered deferral only if the in-app browser is unavailable; clearing event: the operator runs step 22's drive in any Chromium browser against `launch` |
| Socket drop without an engine crash → reconnects and resumes with no restart prompt | Claude_Browser drive with the hidden `--drop-client-at 3000` (web-2) | as above — yes | the drop seam (steps 1, 5); `--reconnect-wait` (step 7); `recovery()` plus the `viewer.socket_drops` and `viewer.resumed` signals | `stream_cli.rs` reconnect case → deferral with the same clearing event |
| Half-time report statistics equal the counts of events in the feed | Claude_Browser drive, screenshot plus DOM read (web-2) | `fixture.smfx` served by `engine-cli replay --web web` — yes (regenerate with `engine-cli record --seed 42 --out fixture.smfx` if it is refused by version) | `report()` and `events()` hook readers; the report DOM carries `data-count` per cell | `report.test.mjs` (unit) → deferral, same clearing event |
| Full time → save writes a file; loading it plays back and rewinds with no engine running | Claude_Browser drive (web-2); file bytes through the hook; `engine-cli replay --fixture <saved>` as the byte-level check | Browser pane; no worker alive — yes, using `launch --engine <missing path>` for the load half | `replay()` / `lastSavedBytes()`; the file input takes a `DataTransfer` file in the drive; `readReplay` | `replay-file.test.mjs` golden round trip → deferral, same clearing event |
| No engine binary at the configured path → first-run state shows the path and instructions | Claude_Browser drive, DOM read plus screenshot (web-2) | `launch --engine C:/missing/engine-cli.exe` — yes | `not-found` state (step 9); first-run panel (step 17) | `launch.rs` case (a) plus `recovery.test.mjs` → deferral, same clearing event |
| Corrupt snapshot on restart → error panel names the corruption, abandon only | `observable: false`: unit test (`recovery.test.mjs`) plus `launch.rs` case (c) | none | the refused state carries the engine's reason | none needed |

- AC kill-and-restart: `constraint-resolution: prerequisite-slice: viewer-reports-recovery` (the launcher, the `engine.json` process identifier, and the hook readers are steps 8, 9, and 19 of this slice). `wall-ownership: code-owned`.
- AC socket drop: `constraint-resolution: prerequisite-slice: viewer-reports-recovery` (the drop seam and reconnect wait, steps 1, 5, and 7). `wall-ownership: code-owned`.
- AC half-time report: `constraint-resolution: prerequisite-slice: viewer-reports-recovery` (hook readers, step 19; the fixture harness exists from the stream slice). `wall-ownership: code-owned`.
- AC save and load: `constraint-resolution: prerequisite-slice: viewer-reports-recovery` (hook readers and `launch --engine`, steps 9 and 19). `wall-ownership: code-owned`. The browser's own download to disk is not required evidence. The file written from the hook bytes and accepted by the engine's `replay` reader proves that the file is a real replay, and a browser download is an agent action that needs permission.
- AC first run: `constraint-resolution: prerequisite-slice: viewer-reports-recovery` (step 9). `wall-ownership: code-owned`.

Every environment dependency is on this machine: Windows 11, the built engine, the in-app browser named in `stack.available-mcp`, and Node 22. No tool outside `stack:` is needed. The slice's stub mentions "a recording (Playwright later)". Playwright is in `toolchains-absent` and not in `stack.testing`, so the kill drive's evidence is the three-point screenshot sequence plus the signal ring buffer. That fully evidences the criterion. A recording joins only when Playwright enters the stack through shape.

## Test / Verification Plan

### Automated checks

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo test --workspace`: the 46 existing tests in `stream` and `engine-cli` plus `reconnect.rs`, `launch.rs`, and the new cases in `stream_cli.rs`, `web_cli.rs`, and `cli_args.rs`. `crates/protocol/tests/document.rs` must stay green with no `MESSAGES` change.
- `node --test "web/tests/*.test.mjs"`: the 46 existing tests plus `replay-file`, `recovery`, `report`, and the truncation cases.
- The stream benchmark compare from step 21.

### Interactive verification (human-in-the-loop)

The platform is web (`stack.platforms`). The driver is the in-app browser (Claude_Browser MCP, PO Q29). Evidence follows `runtime-adapters/web.md` → Evidence layout, with `-t0`, `-t250`, and `-final` screenshots and a `.console.log` per criterion. Every run starts with `cargo build --release -p engine-cli` and uses a fresh `SM_DATA_DIR` under the evidence folder.

1. **kill-restart.** Run `target/release/engine-cli launch --web web --seed 42` and open the printed address. Wait until `engine.json` `snapshot.tick` is above 0 and the rendered tick has passed it. Record the score and clock from the DOM and `__touchline.events()`, then `taskkill /F /PID <engine.pid>`. Screenshot t0, t250, and final. Pass: the panel reads "The engine stopped" with Restart and Abandon, and `recovery().kind` is `crashed`. Click Restart. Pass: `viewer.resumed` shows `to_tick` equal to `snapshot.tick`, the clock reads the snapshot minute, and the score equals the score of the last event at or before that tick. Play then continues, with the rendered tick increasing over 3 seconds.
2. **socket-drop.** Run `launch --web web --seed 42 --drop-client-at 3000`. Pass: `viewer.socket_drops` shows code 1006, then `viewer.resumed` follows. `recovery().kind` is never `crashed` or `refused`: sample it every 100 ms from the drop to the resume. No Restart button appears. The rendered tick keeps increasing afterwards.
3. **half-time-report.** Run `engine-cli replay --fixture fixture.smfx --speed 8 --web web` and play to half-time. Pass: the dialog is open, and for every team and type the `data-count` equals the count in `__touchline.events()` of that type at or before the half-time tick. Screenshot the dialog.
4. **replay-save-load.** Replay the fixture at `--speed 1000` to full time, then click Save replay. Pass: `viewer.replay_saved` shows bytes greater than 0. Write the bytes from `__touchline.replay().lastSavedBytes()` to `saved.smfx` through the shell, and `engine-cli replay --fixture saved.smfx` accepts it with exit 0 against a test client. Then run `launch --web web --engine C:/missing/engine-cli.exe`, confirm that no `engine-cli serve` process exists with `tasklist`, and load the same bytes through the file input with a `DataTransfer` file. Pass: play advances the rendered tick, scrubbing to a tick gives `lastRewind().exact === true`, and `viewer.replay_loaded` shows the saved frame count.
5. **first-run.** Run `launch --web web --engine C:/missing/engine-cli.exe`. Pass: `recovery().kind` is `first-run`, and the panel's `<code>` text equals the configured path. The build instruction is visible. Screenshot.

## Risks / Watchouts

- R1, a resumed score differs: see the yaml sibling. The mitigation is the gated write plus launcher test (b).
- R2, restart duplicates state: truncate before append (steps 14 and 19).
- R3, launcher routes reachable cross-site: same-origin POST only (step 8, test (e)).
- R4, a clean exit misread as a crash: exit-code mapping (step 9, test (b)).
- R5, the order of sibling slices: see Dependencies.
- Windows file locking: `write_atomic` renames over a file that a reader may have open. The launcher reads the snapshot only on restart, when no worker is writing, and `std::fs::rename` replace semantics on Windows were already researched in `04-plan-match-rules.md` Freshness Research.
- Two replay-file writers must agree byte for byte. The golden round trip in step 13 is the tripwire.

## Dependencies on Other Slices

- `match-rules` (complete, verified): `Snapshot`, `Simulation::from_snapshot`, and `snapshot.smsn` are used as-is.
- `viewer-match-day` (plan awaiting input, not built): the event list and reducer that the reports count, and the header score the drives read. Steps 17 to 19 bind to its landed module names.
- `viewer-lineup-tactics` (plan awaiting input, not built): also changes `web/main.mjs`. Changes are made in sequence, never in parallel.
- `integration` consumes `launch` as the way the charter scenario starts.
- Before `/wf implement`, run `/wf plan football-manager-match-engine viewer-reports-recovery` (auto-review) after both sibling viewer slices land.

## Assumptions

Each entry is an autonomous decision, stamped `class: implementation-detail` per `_decision-classes.md`. None touches a carried intent-risk: all six RIMs are adjudicated.

- **A-1** A launcher process (`engine-cli launch`) serves the page and supervises the engine worker. Why: a page cannot start a process, and the page's server dies with the engine, so Q17's "the viewer restarts the engine" needs a survivor. The page still starts the restart. The slice's "configured path" first-run state already presumes a launcher. `class: implementation-detail`.
- **A-2** No protocol change: the reconnect repeats the same `hello`, and the first keyframe marks the resume tick. Why: smallest contract surface; the sibling viewer plans stopped on protocol additions, and this slice needs none. `class: implementation-detail`.
- **A-3** The replay file is the existing `.smfx` layout, written by the page from the wire bytes it kept. Why: no new persisted format, no second position encoder, and the engine's `replay` reads it. `class: implementation-detail`.
- **A-4** `serve` keeps today's end-on-drop behaviour unless it gets `--reconnect-wait`, which the launcher passes as 120 seconds. Why: the existing tested behaviour (`stream_cli.rs:139-168`) is unchanged for every other caller. `class: implementation-detail`.
- **A-5** The snapshot is captured at every stoppage and written once the socket has written the next tick, in the serving path only. The file name, layout, and location are unchanged, and `simulate` and `resume` keep `SnapshotSink`. Why: the page must hold every event at or before the resume stoppage for "same score"; events are not in a snapshot. `class: implementation-detail`.
- **A-6** The launcher passes the match timestamp with a hidden `--match-millis`, so it knows `match.id` and the snapshot path without parsing logs. `class: implementation-detail`.
- **A-7** The engine path comes from `--engine`, then `SM_ENGINE_PATH`, then the launcher's own executable. Why: the slice defers discovery beyond a configured path to `distribution`. `class: implementation-detail`.
- **A-8** Restart and abandon are same-origin POST routes on the launcher's page server with no body. Why: side effects do not belong on GET, and the origin check stops cross-site requests. `class: implementation-detail`.
- **A-9** The half-time report pauses the page's playback until Continue. The engine is not paused: it stays inside its bounded lead. Reports open by rendered tick. Why: the manager reads the report during the break, and the release rule matches viewer-match-day A-1. `class: implementation-detail`.
- **A-10** Save uses a browser download of a `blob:` URL. The drives verify the saved bytes through a read-only hook and the engine's own reader, not a download to disk. Why: the native platform rung; an agent-initiated download needs explicit permission. `class: implementation-detail`.
- **A-11** A one-minute golden `.smfx` is tracked under `web/tests/data/` through a `.gitignore` negation. Why: the only way to prove byte parity with the engine's writer in `node --test` without making `cargo test` depend on Node. `class: implementation-detail`.
- **A-12** This plan does not rewrite `04b-instrument.md` or `05c-benchmark.md`, and does not update the master `04-plan.md` or `po-answers.md` beyond appending its own entry. Why: sibling plan agents are writing those shared files in this same run. `history/04b-instrument-4.*` and `history/05c-benchmark-4.*` already exist uncommitted, and a second writer would collide. The signals are listed in step 20 and the benchmark compare in step 21, so implement and verify have them. The master cohesion pass belongs to the orchestrator. `class: implementation-detail`.
- **A-13** Loading copy is "Starting the engine", "Connecting to the match", and "Waiting for kick-off", with the state words Done, In progress, and Waiting. First-run copy: "Touchline could not find the match engine". Why: wording is implementation-detail. `02b-design.md` fixes the states but gives no copy. `class: implementation-detail`.
- **A-14** Without a launcher (plain `serve --web`), a crash leaves the page offering abandon only, with a line naming `engine-cli launch`. Why: nothing is left to restart the engine; `launch` becomes the documented way to start a match. `class: implementation-detail`.
- Consult: the `appetite-medium-or-larger` and `touches-concurrency` triggers hold. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`), so it is not fired and `consult-runs` stays empty.
- Discovery interview: not asked, by the autonomous run's policy. Every question it would have asked is answered in A-1 to A-14 and appended to `po-answers.md` as an autonomous record.

## Blockers

None. Each acceptance criterion has a verification path on this machine with tooling already in `stack:`.

## Freshness Research

- A loopback origin is a secure context: "An origin is potentially trustworthy if has: A host value of `127.0.0.0/8`… `localhost`" (MDN, Secure Contexts, https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts). So `crypto.subtle.digest` is available to the page on `http://127.0.0.1`.
- WebSocket close codes: 1006 means the connection "was closed abnormally … with no close frame", and 1001 means the endpoint is going away or the browser is navigating away (MDN, CloseEvent.code, https://developer.mozilla.org/en-US/docs/Web/API/CloseEvent/code). The page treats only 1006, and an unclean close, as a drop.
- `<a download>` "only works for same-origin URLs, or the `blob:` and `data:` schemes" (MDN, `<a>`, https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/a). A `blob:` URL from the page qualifies.
- `std::process::Child::kill` is documented as SIGKILL-equivalent on Unix. The Windows behaviour and the killed-process exit code are not stated in the std page (https://doc.rust-lang.org/std/process/struct.Child.html). The launcher therefore does not interpret specific codes: 0 is finished and everything else is crashed. Launcher test (b) observes the real code on Windows.
- `tungstenite` 0.30 blocking server, `sync_channel`, and Origin handling: see `04-plan-stream-protocol.md` § Freshness Research. Windows `rename` replace semantics: see `04-plan-match-rules.md` § Freshness Research. Node 22 `--test` and `.mjs` resolution: see `04-plan-viewer-pitch.md` § Freshness Research.
- No new dependency is added.

## Recommended Next Stage

- **Option A (default): `/wf implement football-manager-match-engine viewer-reports-recovery`**: the plan is complete with no blocker. Run it only after `viewer-match-day` and `viewer-lineup-tactics` land, and after an auto-review re-run of this plan binds steps 17 to 19 to their module names. Compact the session first.
- **Option B: `/wf plan football-manager-match-engine viewer-reports-recovery`** (auto-review): once the sibling viewer plans have their answers and are built.
- **Option C: `/wf slice football-manager-match-engine`**: only if the product owner wants the launcher as its own slice ahead of the viewer track, because `integration` also depends on it.
