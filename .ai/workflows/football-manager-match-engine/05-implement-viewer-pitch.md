---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: viewer-pitch
status: complete
stage-number: 5
created-at: "2026-09-22T16:30:00Z"
updated-at: "2026-09-22T16:30:00Z"
metric-files-changed: 50
metric-lines-added: 4098
metric-lines-removed: 68
metric-deviations-from-plan: 7
metric-review-fixes-applied: 0
commit-sha: "62a5dab8acd9a83ce2cff6010c4263d7e3fb8763"
tags: [viewer, canvas, playback, milestone]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-viewer-pitch.md
  plan: 04-plan-viewer-pitch.md
  siblings: [05-implement-data-schemas-generator.md, 05-implement-engine-core.md, 05-implement-stream-protocol.md]
  verify: 06-verify-viewer-pitch.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine viewer-pitch"
---

# Implement: Viewer Pitch and Playback

## The Implementation

Three slices before this one built an engine that computes a match, a protocol that
describes it, and a socket that streams it. Nothing in the repository could show a match
to a person. This slice adds the page: `web/`, thirty files, and a static server inside
`engine-cli` that sends it with the two isolation headers the page's memory gauge needs.

Four decisions carry the work. The page is twelve DOM-free modules plus one wiring file,
so six of the eight acceptance criteria are provable by `node --test` with no install, no
`package.json`, and no browser. The renderer maps the engine's own metre space uniformly
onto the canvas, because the 616 by 411 tile is squarer than a 105 by 68 pitch and
stretching one axis would draw a marker where the engine never put one. The history keeps
decoded `Int16Array` components rather than wire frames: a full 90-minute match measures
24.2 megabytes against a 300-megabyte budget. The lag notice is measured in the page and
never announced by the server, because no server message carries it and
`WebSocket.bufferedAmount` counts outgoing bytes only.

Two facts the plan did not have now stand on the record. A recorded fixture stored no
hello, so team names and kit colours could not survive a replay; the recorder now writes
the hello as the fixture's first entry. The live `serve` command sends a whole match as
fast as the socket accepts it — 270,000 ticks in about 13 seconds — then closes, so a
live watch is in fact a fast load followed by local playback. That is the top open
question for `viewer-match-day`. The page absorbs it: it stores the match and the
scrubber reaches every tick of it.

## Summary of Changes

- `web/` ships the page: 13 ES modules, 4 stylesheets, 2 fonts with their licences, the
  shell, a diagnostic page, and 7 test files with 40 tests.
- `crates/engine-cli/src/web.rs` serves that folder over plain HTTP with
  `Cross-Origin-Opener-Policy` and `Cross-Origin-Embedder-Policy` on every response.
- A generated `GET /engine.json` tells the page the socket port and the protocol version.
- `--web <DIR>` is added to `serve` and to `replay`; `--sustain <SPEED>` is added to
  `replay` as the harness AC-e needs.
- `TeamRef` gains `team.kit.primary` and `team.kit.secondary`; the recorder stores the
  hello; the replayer forwards the stored hello instead of inventing one.
- `StreamError::FixtureNoHello` refuses a fixture that does not open with a hello.
- `README.md`, `DESIGN.md`, and `docs/reference/protocol.md` follow the code.

## Files Changed

- `web/index.html`: the 1280 by 800 shell, the header mark canvas, three columns, the
  pitch panel with its skeleton, the control strip, the notice, and the live region.
- `web/tokens.css`: the 36 `--tl-` tokens in one `:root` block, all OKLCH.
- `web/layout.css`: the grid, the panels, the notice, the scrubber, and the skeleton.
- `web/components/match-control.css`: the button primitive, four sizes, six states.
- `web/fonts/fonts.css`, `fonts/barlow-condensed-700.woff2`,
  `fonts/ibm-plex-sans-var.woff2`, and the two OFL licence files: the local font bundle.
- `web/signal.mjs`: the `viewer-event` envelope, `console.info`, and a 256-row ring.
- `web/colour.mjs`: OKLCH conversion, lightness clamping, contrast, and `safeKit`.
- `web/decode.mjs`: the wire decoder, ported constants, `viewer.decode_refused`.
- `web/interpolate.mjs`: one clamped segment function, which cannot extrapolate.
- `web/history.mjs`: two `Int16Array` buffers, byte accounting, `viewer.history`.
- `web/schedule.mjs`: the tick cursor, skipping, and the frame-budget window.
- `web/stoppages.mjs`: the stoppage index behind Prev stop and Next stop.
- `web/mark.mjs`: the touchline mark, drawn for the favicon and the header.
- `web/pitch.mjs`: the canvas renderer and the metre-to-pixel mapping.
- `web/socket.mjs`: the client, the address builder, and the drop signals.
- `web/playback.mjs`: the speed selector, the sustained-rate estimator, the notice text.
- `web/main.mjs`: the wiring, the clock, the scrubber, and the `__touchline` hook.
- `web/handshake.html`: the diagnostic page that opens the socket and prints the hello.
- `web/tests/*`: `helpers.mjs` and seven test files.
- `crates/engine-cli/src/web.rs`: the static server, the MIME table, `/engine.json`.
- `crates/engine-cli/tests/web_cli.rs`: the server's own tests, with a child-process guard.
- `crates/engine-cli/src/cli.rs`: `--web` on both commands, `--sustain` on `replay`.
- `crates/engine-cli/src/main.rs`: the `web` module declaration.
- `crates/engine-cli/src/replay.rs`: the fallible `Replayer::new`, both printed addresses.
- `crates/engine-cli/src/record.rs`: the hello is written as the fixture's first entry.
- `crates/engine-cli/src/serve.rs`, `src/bench.rs`: kit colours in the hello builders.
- `crates/engine-cli/tests/stream_cli.rs`: the frame count rises to 3004 for the hello.
- `crates/protocol/src/message.rs`: the two kit fields and their two tests.
- `crates/protocol/src/lib.rs`: the `MESSAGES` field list and a note on the version.
- `crates/stream/src/replay.rs`: the stored hello, the `sustain` cap, the wider signal.
- `crates/stream/src/lib.rs`: `FixtureNoHello`.
- `crates/stream/tests/fixture.rs`, `tests/hello.rs`, `tests/common/mod.rs`: coverage.
- `docs/reference/protocol.md`: the nested teams table and a page-server section.
- `README.md`: Watch a match, and Test the page.
- `DESIGN.md`: eight colour rows, the two faces, the 36 tokens, the mark's file.
- `PRODUCT.md`: the product name, Touchline.

## Shared Files (also touched by sibling slices)

- `crates/protocol/src/message.rs` and `src/lib.rs`: `stream-protocol` owns the hello.
  The two kit fields are additive and `PROTOCOL_VERSION` stays 1, which the doc comment
  records with its reason.
- `crates/stream/src/replay.rs` and `src/lib.rs`: `stream-protocol` owns the replayer.
- `crates/engine-cli/src/cli.rs`, `main.rs`, `serve.rs`, `record.rs`, `bench.rs`: shared by
  every prior slice.
- `docs/reference/protocol.md`, `README.md`, `DESIGN.md`: the documentation plan.

## Notes on Design Choices

- **Rung 1 wherever the standard library reaches.** The static server is `std::net` and a
  match over file extensions. No HTTP crate and no bundler entered the workspace.
- **The page has no build step.** ES modules load directly, which is why a static server
  exists at all: the `file://` scheme blocks module loading.
- **The isolation headers sit on every response**, not on the page alone, because
  `crossOriginIsolated` is a document property and one stylesheet without them poisons it.
- **`safeKit` clamps lightness and chooses the ring by measured contrast.** A kit that is
  too dark or too light on turf is lifted into range; a trim colour under 3:1 against the
  turf is replaced by the pitch line. Colour is never the only separation between teams.
- **The shirt-number face is read back from the page** rather than written a second time
  in JavaScript, so the canvas cannot drift from the stylesheet.
- **The mark is drawn from the live token values.** No image file exists in the product.

## Verification Seams Built

- **AC-a** 60 frames per second with zero dropped ticks → `viewer.frame_budget`, derived
  from consecutive `requestAnimationFrame` deltas with `refresh_hz` beside it, at
  [web/schedule.mjs:130](web/schedule.mjs:130), plus `window.__touchline.frame()` at
  [web/main.mjs:231](web/main.mjs:231) (enables the browser drive to read the rate and the
  panel's own limit in one row).
- **AC-b** a marker lies on the segment between two ticks → `between()` as a DOM-free
  module at [web/interpolate.mjs:1](web/interpolate.mjs:1) and
  `window.__touchline.lastRendered()` at [web/main.mjs:221](web/main.mjs:221) (enables
  `node --test` and the drive to read the same numbers).
- **AC-c** at 8x the viewer skips and never passes the newest tick → `Scheduler.advance`
  with an injected timestamp at [web/schedule.mjs:1](web/schedule.mjs:1) and
  `viewer.tick_skipped` at [web/schedule.mjs:119](web/schedule.mjs:119) (enables
  `node --test` with a synthetic stream).
- **AC-d** the 4x clock within 2 percent → the clock readout and the speed selector, with
  `lastRenderedTick()` at [web/main.mjs:222](web/main.mjs:222) (enables the drive to take
  two readings and divide).
- **AC-e** a sustained 3x drops playback to 3x with a notice → `--sustain` on `ReplayOpts`
  at [crates/engine-cli/src/cli.rs:119](crates/engine-cli/src/cli.rs:119) and the paced
  minimum at [crates/stream/src/replay.rs:72](crates/stream/src/replay.rs:72), with
  `viewer.lag` at [web/playback.mjs:102](web/playback.mjs:102) (enables a server that
  sustains less than it is asked for, which is what makes this criterion drivable at all).
- **AC-f** rewind draws exactly the stored positions → `window.__touchline.lastRewind()`
  at [web/main.mjs:232](web/main.mjs:232), which compares the drawn array to the stored
  array component by component and reports `exact`, and `viewer.rewind` at
  [web/main.mjs:174](web/main.mjs:174) (enables the drive to prove equality, not presence).
- **AC-g** a full match under 300 MB → `History.bytes()` and `viewer.history` at
  [web/history.mjs:90](web/history.mjs:90), the two isolation headers at
  [crates/engine-cli/src/web.rs:241](crates/engine-cli/src/web.rs:241), and
  `page_bytes_reason`, which names a refused measurement instead of reporting a bare null
  (enables `node --test` for the accounting and the drive for the page gauge).
- **AC-h** the charter scenario against the live engine → `--web` on `ServeOpts` and the
  generated `/engine.json` at
  [crates/engine-cli/src/web.rs:39](crates/engine-cli/src/web.rs:39) (enables one command
  to start the engine and serve the page, with the page finding the socket by itself).

## Visual Contract Honored

1. **The 1280 by 800 grid** — honored at [web/layout.css:65](web/layout.css:65), with the
   tokens at [web/tokens.css:63](web/tokens.css:63). Measured in the browser at 1280 by
   800 with columns 336, 616, and 296.
2. **Pitch tile at exactly 616 by 411 with markings from the 105 by 68 metre space** —
   honored at [web/index.html:36](web/index.html:36) and
   [web/pitch.mjs:72](web/pitch.mjs:72). | deviation: the panel carries no padding and its
   hairline is an outset shadow ring, because 8 pixels of padding pushed the tile past its
   616-pixel track and moved the right column off the screen.
3. **Playback control strip, 40 pixels, under the pitch** — honored at
   [web/layout.css:143](web/layout.css:143). Measured at exactly 616 by 40.
4. **Marker treatment: safe fill, conditional ring, contrast-chosen number** — honored at
   [web/colour.mjs:128](web/colour.mjs:128) and [web/pitch.mjs:196](web/pitch.mjs:196).
5. **Ball with a short falling-alpha trail** — honored at
   [web/pitch.mjs:194](web/pitch.mjs:194): one path, twelve ticks, the ball alone.
6. **Barlow Condensed 700 and IBM Plex Sans from a local bundle, tabular numbers** —
   honored at [web/fonts/fonts.css:14](web/fonts/fonts.css:14) and
   [web/layout.css:87](web/layout.css:87). | deviation: two font files ship, not four. One
   variable IBM Plex Sans file covers 400, 500, and 600 in fewer bytes than three static
   files.
7. **The `--tl-` OKLCH token set, no pure black and no pure white** — honored at
   [web/tokens.css:13](web/tokens.css:13). No hex, `rgb()`, or `hsl()` value appears
   anywhere in `web/` outside the conversion module and its tests.
8. **The touchline mark drawn programmatically** — honored at
   [web/mark.mjs:80](web/mark.mjs:80), used for the header canvas and for the favicon as a
   data URI. No image file exists in `web/`.
9. **State words beside every state colour** — honored at
   [web/index.html:92](web/index.html:92) and [web/playback.mjs:61](web/playback.mjs:61):
   the notice carries the word Lag and names the rate in digits and in words.
10. **Skeletons, never a spinner in a content area** — honored at
    [web/index.html:42](web/index.html:42) and [web/layout.css:257](web/layout.css:257),
    with a cross-fade when the first tick arrives.

## Deviations from Plan

1. **Step 3 assumed a stored hello that did not exist.** `engine-cli record` wrote tick
   entries only, so "replay forwards the stored hello" had nothing to forward, and team
   names were unrecoverable from any fixture. The recorder now writes the hello as the
   fixture's first entry, and `stream_cli.rs` expects 3004 frames instead of 3003.
2. **The plan did not say how the page learns the socket port.** The page and the socket
   sit on two ports, both chosen by the operating system. The server generates
   `GET /engine.json`, which answers with the socket port and the protocol version.
3. **Seven test files, not six.** Step 16 names a determinism test for the mark, which has
   no file in the plan's list of six. `web/tests/mark.test.mjs` carries it.
4. **Step 7's goal tick and restart tick do not exist.** The fixture scores 0 to 0 and
   contains no restart frame, because `match-rules` has not shipped. AC-f was driven at
   plain mid-match ticks instead.
5. **The hello builder is not in `crates/stream/src/server.rs`.** The builders live in
   `engine-cli`'s `serve.rs`, `record.rs`, and `bench.rs`, plus the stream tests' helper.
   All four now carry the kit colours.
6. **Two font files, not four.** See the visual contract, item 6.
7. **`crates/engine-cli/tests/cli_args.rs` needed no change.** Its help-width assertions
   already cover the new options, which the passing suite confirms.

## Anything Deferred

- **A worker with `OffscreenCanvas`.** The slice's own risk list names it. The main thread
  holds every frame the panel offers at 8x with zero dropped frames, so the work is not
  needed yet. Ceiling: a panel above 60 hertz with a heavier overlay. Upgrade path: the
  renderer takes a canvas in its constructor and transfers cleanly.
- **`performance.measureUserAgentSpecificMemory()` is refused by this browser pane**, with
  `crossOriginIsolated` true, both headers present, and the function itself defined. The
  page records `page_bytes_reason` with the thrown message instead of a silent null.
  Ceiling: `page_bytes` is unmeasured in this driver. Upgrade path: any Chromium build
  that resolves the promise, or a headless run at verify.
- **Stoppage navigation ships without stoppages.** `stoppages.mjs` and its two buttons are
  built and tested, and the fixture contains no restart frame to fill the index.
  `match-rules` fills it.

## Known Risks / Caveats

- **AC-a cannot pass in this driver.** The browser pane refreshes at 32 hertz, so the page
  reports 32 frames per second with `refresh_hz` 32, `frame_ms_p95` 31.62, and zero
  dropped frames. The page draws every frame the pane offers. The 60 in the criterion
  describes a 60-hertz panel, and R1's guard is what makes this visible instead of a false
  pass.
- **`requestAnimationFrame` is throttled while the pane does not paint.** Any measurement
  at verify must keep the tab in front for its whole window.
- **`engine-cli serve` does not pace to real time.** It sends 270,000 ticks in about 13
  seconds and closes the session. The page absorbs it, and the manager then watches a
  stored match. A paced live mode is a decision for `viewer-match-day`.
- No new `sdlc-debt:` marker was written by this slice.

## Freshness Research

- **Node 22 `node:test`** — the runner accepts a glob, and a bare directory argument fails
  with `MODULE_NOT_FOUND`. The documented command is
  `node --test "web/tests/*.test.mjs"`, which `README.md` records.
- **`WebSocket.bufferedAmount`** — it counts outgoing bytes only. A client cannot read its
  own inbound queue, which is why the lag estimator measures arrival times instead.
- **`performance.measureUserAgentSpecificMemory()`** — it needs cross-origin isolation,
  which the server provides; the pane still refuses it. The refusal is recorded, not
  hidden.
- **ES modules over the `file://` scheme** — the browser blocks them, which is the reason
  the static server exists.
- **OKLCH to sRGB and WCAG contrast** — the conversion and the 3:1 ring threshold were
  measured, not assumed: a clamped black reaches 5.23:1 on turf and stays the ring, while
  the orange trim reaches 1.21:1 and falls back to the pitch line.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine viewer-pitch`. Six of
  the eight criteria are user-observable, two augmentations are due to re-check, and
  AC-a's refresh-rate shortfall needs a verdict from the stage that owns the gate.
- **Option B:** `/wf review football-manager-match-engine viewer-pitch`. Choose this only
  to review the page's structure before the criteria are exercised again. It skips the
  benchmark compare and the instrument key check, and both are due.
