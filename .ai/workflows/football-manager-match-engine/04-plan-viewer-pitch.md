---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: viewer-pitch
status: complete
stage-number: 4
created-at: "2026-09-22T14:41:32Z"
updated-at: "2026-09-22T14:41:32Z"
metric-files-to-touch: 46
metric-step-count: 22
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [viewer, canvas, playback, websocket, javascript, milestone]
stack-source: confirmed
augmentations:
  instrument: 04b-instrument.md
  benchmark: 05c-benchmark.md
  design-contract: 02c-craft.md
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-viewer-pitch.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md]
  design: 02b-design.md
  contract: 02c-craft.md
  steer: steer.md
  implement: 05-implement-viewer-pitch.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine viewer-pitch"
---

# Plan: Viewer Pitch and Playback

## The Plan

Three slices handed over an engine that delivers 634,859 ticks per second over a WebSocket, a codec crate with no input or output, a byte-exact fixture, and a replayer. They also handed over an empty frontend: the repository holds no HTML, no CSS, no JavaScript, and no `package.json`. This is the visible-milestone slice, where commitment C2 is proven or broken, and research settled the first fact before any design question: a browser blocks module scripts outright over `file://`, so the page cannot be a double-clicked file and something must serve it.

Twelve product-owner answers fixed the build, and three of them closed gaps the slice definition assumed were already closed. The engine binary gains a `--web` flag and serves the page itself, which gives one program to start, an origin already on the server's allowlist, and the two cross-origin-isolation headers the chosen memory gauge requires. The lag notice is measured in the page, not announced by the server, because `ReplayOpts` has no cap flag, `Replayer::serve` never reads the socket, no `ServerMessage` announces lag, and the browser exposes no inbound-queue depth — `WebSocket.bufferedAmount` counts outgoing bytes only; a one-flag `--sustain` cap on `replay` is the harness that makes the criterion drivable. Kit colours join `TeamRef`, because the opening message carries an identifier and a name and nothing else, while the standing steering file needs the club's secondary colour for every marker ring.

Inside the page, the numbers made the decisions easy. Drawing 22 discs, 22 shirt numbers, a ball and a trail at 616 by 411 is sub-millisecond against a 16.6-millisecond budget, so rendering stays on the main thread and the test hook stays directly readable; a worker and `transferControlToOffscreen()` would be a one-way trade for headroom nothing needs. The whole-match history decodes to absolute `Int16Array` arrays at 24.2 megabytes, twelvefold under the 300-megabyte budget, which buys an O(1) rewind instead of a 49-step keyframe walk on every scrub. The modules carry the `.mjs` extension so `node --test` runs the interpolation and scheduler proofs with nothing installed and the repository stays a pure Cargo workspace.

Implement builds 30 new files and changes 16 across 22 steps, the pure modules first and the browser drive last. The top risk is not drawing and not decoding: it is that `requestAnimationFrame` fires at the display's refresh rate, so on a 144 hertz panel a naive callback count reports 144 and the 60-frames-per-second criterion passes for the wrong reason. Every frame-rate reading carries `refresh_hz` beside it and is derived from timestamp deltas, never from a count.

## Current State

- The repository holds **no frontend of any kind**: no HTML, no CSS, no JavaScript, no `package.json`, no `node_modules`. `.gitignore` carries `target/`, `.scratch/`, `*.ticks`, `*.smfx` only. No `web/` or `viewer/` convention is declared in `README.md`, `PRODUCT.md`, `DESIGN.md`, or `AGENTS.md`.
- The wire format is fixed and documented. `KIND_KEYFRAME = 0x01`, `KIND_DELTA = 0x02`, `KIND_RESTART = 0x03` (`crates/protocol/src/frame.rs:12-16`). A keyframe is 1 tag byte plus 98 payload bytes: `tick` as `u32` little-endian at 0..4, then ball x, y, height as three `i16` centimetres at 4..10, then 22 players as `[i16; 2]` centimetres at 10..98, home team first (`crates/protocol/src/codec.rs:10,80-87`). A delta is 1 tag byte plus 47: no tick field, then 3 `i8` centimetre steps for the ball and 44 for the players (`codec.rs:12,105-117`). A restart frame is keyframe-shaped with tag `0x03` (`frame.rs:29-38`).
- A step wider than `i8` cannot encode, and the producer falls back to a keyframe rather than failing (`codec.rs:114`, `crates/stream/src/session.rs:196-219`). The decoder therefore never sees an out-of-range delta.
- `PROTOCOL_VERSION = 1` (`crates/protocol/src/lib.rs:23`). `DEFAULT_KEYFRAME_INTERVAL = 50` (`codec.rs:14`), but the live value rides in `hello.keyframe_interval` and comes from `content/tuning.json` `stream.keyframe_interval`.
- JSON keys are **dotted, not camel case**: `protocol.version`, `engine.version`, `build.hash`, `owner.id`, `match.id`, `home.score`, `change.queue_id` (`crates/protocol/src/message.rs`, `event.rs`, `command.rs`). Enum tag values are kebab-case. A decoder must reproduce both literally.
- **`TeamRef` carries `team.id` and `team.name` and nothing else** (`crates/protocol/src/message.rs:11-18`). No kit colour reaches the page.
- **`ReplayOpts` carries `--fixture` and `--speed` only** (`crates/engine-cli/src/cli.rs:106-114`), and `Replayer::serve` writes stored frames on an `Instant` schedule and never calls `socket.read()` (`crates/stream/src/replay.rs:64-105`), so it honours no `Start`, `Pause`, or `SetSpeed` command.
- **`Replayer::serve` synthesises its own hello** rather than forwarding the recorded one, and hard-codes `keyframe_interval` to `DEFAULT_KEYFRAME_INTERVAL` instead of reading the fixture. A fixture recorded under a tuned interval would therefore announce the wrong interval to the page.
- **Backpressure is a server-side log only.** `ChannelOut::send` blocks the simulation thread and emits one `socket.backpressure` line (`session.rs:127-159`); nothing reaches the client, and `docs/reference/protocol.md:144-149` states this. No `ServerMessage` variant announces lag (`crates/protocol/src/lib.rs:85-185`).
- The Origin allowlist accepts `none`, `null`, any `file://`, and `http://localhost` or `http://127.0.0.1` with any port; `https://` loopback is refused (`crates/stream/src/server.rs:180-188`). The version guard reads `?v=` and names both versions on refusal (`server.rs:191-197`). The server keeps listening past a refusal (`server.rs:85-93`).
- The port lands at `<data_dir>/engine.port` and the address is `ws://127.0.0.1:<port>/?v=1` (`server.rs:18,56-58,78-80`). `data_dir()` is `SM_DATA_DIR`, else `%LOCALAPPDATA%\SoccerManager` on Windows (`crates/engine/src/observe/identity.rs:20-30`).
- The pitch is 105 by 68 metres **centred on the origin**, so x spans -52.5 to 52.5 and y spans -34 to 34, with `GOAL_WIDTH = 7.32` centred on y = 0 (`crates/engine/src/pitch.rs:5-15`). This is the space the tick stream quantises.
- Kit colours exist as hex strings at `club.kit.primary` and `club.kit.secondary` in `content/teams/*.json`. `default-a.json` sets the secondary to `#000000`.
- 122 tests pass on commit `3570eb9` in about 30 seconds across 4 crates and 16 binaries. Node is v22.15.0 with `node:test`, `node:assert`, and a global `WebSocket`; there is no DOM, no `jsdom`, no `puppeteer`, and Playwright is in `toolchains-absent`.
- `06-verify-stream-protocol.md:241` states the gap this slice closes: "The protocol is verified against one client, the scratch client built on the `stream` crate. No browser has opened the socket yet; `viewer-pitch` is the first."
- One `sdlc-debt:` marker exists in the tree, at `crates/engine/src/sim.rs:272`, and it belongs to `match-rules`, not to this slice.

## Simplicity Ladder

| Capability | Rung | Evidence |
|---|---|---|
| WebSocket client | rung 2 native-platform | The browser `WebSocket` API. `binaryType = 'arraybuffer'` avoids the asynchronous `Blob` step the default would force at 400 frames per second (MDN, `WebSocket.binaryType`) |
| Binary frame decode | rung 1 stdlib | `DataView.getInt16(offset, true)` over the delivered `ArrayBuffer`. V8 closed the DataView penalty at 6.9, so explicit little-endian reads at fixed offsets match typed-array speed (v8.dev/blog/dataview). No parser library |
| Compact history | rung 1 stdlib | `Int16Array` structure-of-arrays. 270,000 ticks x 47 components x 2 bytes = 25,380,000 bytes, about a thousandth of any `ArrayBuffer` ceiling |
| Frame scheduling | rung 2 native-platform | `requestAnimationFrame` with a fixed-timestep accumulator. The pattern is Fiedler's, already cited in `02-shape.md` Freshness Research |
| Tick-bounded interpolation | rung 4 new-code | No interpolation code exists anywhere in the repository, in Rust or otherwise. The function is about twenty lines and is the direct expression of RIM-5 |
| Canvas pitch rendering | rung 2 native-platform | `CanvasRenderingContext2D`. MDN's optimisation guidance applies: `alpha: false`, integer coordinates, batched paths, minimal state changes |
| Off-main-thread rendering | rung 2 native-platform, **not taken** | `OffscreenCanvas` with `transferControlToOffscreen()` is Baseline since March 2023, and `WebSocket` plus a canvas-bound `requestAnimationFrame` both work in a dedicated worker. Not taken: the draw is sub-millisecond against 16.6 ms, and the transfer is one-way, which would move the test hook across a message boundary (PO Q5) |
| Pitch geometry | rung 3 reuse | `crates/engine/src/pitch.rs:5-15` — `LENGTH`, `WIDTH`, `GOAL_WIDTH`, and the centred-origin convention, ported as constants |
| Wire layout constants | rung 3 reuse | `crates/protocol/src/codec.rs:8-14` and `frame.rs:12-18`, ported as constants. `docs/reference/protocol.md` is the written source and is held to the code by `crates/protocol/tests/document.rs` |
| Stoppage index | rung 3 reuse | The restart tag already marks every restart tick (`frame.rs:83-85`); the event messages already carry their ticks. Merging two existing sources beats adding a third |
| Static file server | rung 4 new-code | Nothing in the workspace serves a file over HTTP. `python -m http.server` is rung 2 but mis-types `.mjs` and needs a second runtime and a second process; Node has no built-in static server without a `package.json`. About a hundred lines in `engine-cli` buys one process, one origin, a correct MIME table, and the two isolation headers (PO Q1) |
| JavaScript test runner | rung 2 native-platform | Node 22's `node:test` and `node:assert`, confirmed present with `node -e "require('node:test')"`. No install, no `package.json`, no third-party framework |
| Colour conversion | rung 4 new-code | No colour utility exists. Hex to OKLCH with a lightness clamp and a contrast ratio is about forty lines; a library would be a dependency for one file (PO Q11) |
| Memory gauge | rung 2 native-platform | `performance.measureUserAgentSpecificMemory()`, unlocked by the two headers the Q1 server sends. `performance.memory` is deprecated and non-standard; `navigator.deviceMemory` reports the device's RAM tier, not the page's use |

## Applied Learnings

- `.ai/solutions/INDEX.md` does not exist and `.ai/sdlc-config.json` does not exist. No applicable learnings found.
- Carry-over from `04-plan-stream-protocol.md` deviation 6: the encoder writes into a fixed-size reused buffer rather than a `Vec` per tick. The decoder mirrors the discipline — one `DataView` over the delivered buffer, positions written straight into the pre-allocated history arrays, no intermediate object per tick.
- Carry-over from `06-verify-stream-protocol.md:241`: no browser has opened the socket. Step 12 is the first, and it is deliberately the smallest possible page — connect, read the hello, log it — so a handshake fault is diagnosed before any rendering code exists to blame.
- Carry-over from `06-verify-stream-protocol.md`: a rejected `queue-change` writes no `change.kind` on its event row. Informational only here; this slice queues no change.
- Carry-over from `03-slice-probe-engine-core.md` finding 3: `thiserror` with an explicit `#[source]`, never `#[from]` on a type whose display repeats its source. The new `web.rs` error variants follow it.
- **Repeat-deferral tripwire.** `00-index.md` `runtime-evidence-deferrals` is empty, so no wall is inherited. One new wall is named by this slice and is classified below rather than deferred: see `## Verification Strategy`.

## Likely Files / Areas to Touch

- `web/` (new, 28 files): `index.html`; `tokens.css`, `layout.css`, `fonts/fonts.css`, `components/match-control.css`; four `woff2` faces under `fonts/`; the modules `signal.mjs`, `colour.mjs`, `decode.mjs`, `interpolate.mjs`, `history.mjs`, `schedule.mjs`, `stoppages.mjs`, `playback.mjs`, `socket.mjs`, `pitch.mjs`, `mark.mjs`, `main.mjs`; and `tests/` with `helpers.mjs` plus six `*.test.mjs` files.
- `crates/engine-cli/src/web.rs` (new): the static server, its MIME table, and the two isolation headers.
- `crates/engine-cli/tests/web_cli.rs` (new): the server's own tests.
- `crates/protocol/src/message.rs`: `TeamRef` gains the two kit fields.
- `crates/protocol/src/lib.rs`: the `MESSAGES` field list for `hello`.
- `crates/protocol/tests/document.rs`: the document test reads the new fields.
- `crates/stream/src/server.rs`: the hello builder carries kit colours.
- `crates/stream/src/replay.rs`: forward the stored hello instead of synthesising one; honour `--sustain`; `fixture.replayed` gains `sustain`.
- `crates/stream/src/lib.rs`: one new error variant for a fixture with no stored hello.
- `crates/engine-cli/src/cli.rs`, `main.rs`, `serve.rs`, `replay.rs`: the `--web` and `--sustain` options and their wiring.
- `crates/engine-cli/tests/cli_args.rs`, `tests/stream_cli.rs`: flag and help coverage.
- `crates/engine-cli/Cargo.toml`: no new dependency; the server uses `std::net` and the existing `tungstenite` handshake path is untouched.
- `docs/reference/protocol.md`: the hello table gains two fields; a new section documents the static server and its headers.
- `README.md`: how to run the engine and open the page (the shape's Documentation Plan).
- `DESIGN.md`: the ten new `--tl-` token rows, so it never disagrees with `web/tokens.css`.

## Proposed Change Strategy

Build from the inside out, exactly as the stream slice did, and prove each layer before the next uses it. The pure modules — `colour`, `decode`, `interpolate`, `history`, `schedule`, `stoppages` — have no DOM, no canvas, and no socket, so `node --test` proves all six before a browser is ever opened. Then the renderer, then the socket, then the page, then the drive.

NFR-2 governs the rendering decisions and `yields-to: C2` (PO-ratified): frame pacing never invents a position. The interpolation function takes two consecutive received ticks and a fraction in `[0, 1]` and cannot express a value outside that segment; the scheduler advances the tick cursor and skips, and no code path extrapolates. That is RIM-5 expressed as a type constraint rather than as a comment.

NFR-3 governs the history: 24.2 megabytes against 300, which is why the decoded form wins over the wire form. NFR-6 governs the marker ring, the focus rings, and the lag notice: colour is never the only signal. NFR-9 keeps the target at Windows 11, and the static server uses `std::net` only, so no platform branch appears.

The one genuinely new architectural fact is that the engine binary now speaks two protocols on two ports: the existing WebSocket on its assigned port, and plain HTTP for the page. They stay separate listeners in separate modules, because the WebSocket server's Origin allowlist and version guard have nothing to do with serving a stylesheet, and folding them together would put the page's MIME table inside `crates/stream`, which `NFR-8` keeps for the socket alone.

## Step-by-Step Plan

1. **Kit colours on the wire.** `crates/protocol/src/message.rs`: `TeamRef` gains `#[serde(rename = "team.kit.primary")] pub kit_primary: String` and `team.kit.secondary`, both lower-case hex with a leading `#`. Update the `MESSAGES` field list for `hello` in `lib.rs` and the round-trip unit test. `PROTOCOL_VERSION` stays 1: no message is removed and no field changes meaning, and the only two producers in existence are updated in the same commit. Record that judgement in a comment beside the constant.
2. **Hello builders carry the colours.** `crates/stream/src/server.rs`: read `club.kit.primary` and `club.kit.secondary` from the two loaded `TeamFile` values and fill the new fields. Unit test: a hello built from the shipped defaults carries `#c8102e` for team A.
3. **Replay forwards the stored hello.** `crates/stream/src/replay.rs`: read the first stored text entry, confirm it is a `hello`, and write those exact bytes instead of synthesising a new message. This retires a latent fault — the current code hard-codes `keyframe_interval` to `DEFAULT_KEYFRAME_INTERVAL` rather than reading the fixture, so a fixture recorded under a tuned interval announces the wrong interval — and it carries kit colours through replay for free. Add `StreamError::FixtureNoHello` naming the path when the first entry is not a hello. Test: a replay's hello is byte-identical to the recorded one.
4. **The sustain cap.** `crates/engine-cli/src/cli.rs`: `ReplayOpts` gains `--sustain <f32>`, optional, help text "Cap the delivered rate at this speed regardless of --speed; for testing the viewer's lag notice." `crates/stream/src/replay.rs`: `serve()` takes `Option<f32>` and paces at `min(speed, sustain)` while the hello still announces the requested speed, so the page sees a stream slower than it asked for. `fixture.replayed` gains `sustain`, `null` when absent. Test: `--speed 8 --sustain 3` delivers at three times real time within 5 percent over 10 seconds of match time.
5. **The static server.** `crates/engine-cli/src/web.rs` (new): a `std::net::TcpListener` on `127.0.0.1:0`, one thread, a blocking read of the request line, and a file response from the directory given by `--web`. Serve `GET` and `HEAD` only; answer anything else with 405. Refuse any path containing `..` or a leading drive letter with 403. MIME table: `.html` `text/html`, `.css` `text/css`, `.mjs` and `.js` `text/javascript`, `.woff2` `font/woff2`, `.json` `application/json`, `.png` `image/png`, `.svg` `image/svg+xml`; anything else `application/octet-stream`. Every response carries `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`, which the memory gauge requires (PO Q8), plus `Cache-Control: no-store` so a reload always shows the current file. Emit `web.serving` with `dir`, `files`, `isolated`, `port` per `04b-instrument.md`.
6. **Wire the flag.** `crates/engine-cli/src/cli.rs`: `--web <DIR>` on both `ServeOpts` and `ReplayOpts`, optional. `serve.rs` and `replay.rs` start the static server before the socket and print both addresses, the page address last so it is the line a reader copies. Keep every help line inside 80 columns, the rule the second plan introduced. Tests in `tests/web_cli.rs`: the page address is printed; a missing directory is refused naming the path; `..` in a request path is refused; a `.mjs` response carries `text/javascript`; every response carries both isolation headers.
7. **Re-record the fixture.** Run `engine-cli record --seed 7 --out fixture.smfx --minutes 90`, then confirm the stored hello carries the kit colours and note the tick of the first goal and of the first restart, which steps 19 and 20 assert against. Fixtures stay gitignored; record the seed, the tick counts, and the two noted ticks in `05-implement-viewer-pitch.md` so any machine can rebuild the same file.
8. **Signal writer.** `web/signal.mjs`: one exported `signal(name, fields)` that stamps the `viewer-event` envelope, writes one JSON Lines row to `console.info`, and pushes into a 256-row ring exposed later on the test hook. Pure; no DOM. Test: the envelope carries `record.kind`, `schema.version`, `service`, `operation`, `signal`, and `ts`, and the ring never exceeds 256 rows.
9. **Colour.** `web/colour.mjs`: `hexToOklch`, `clampLightness(l)` into `0.20 … 0.88`, `contrast(a, b)`, and `safeKit({primary, secondary})` returning the clamped fill, the ring colour, and the shirt-number colour. The ring is the clamped secondary when it measures at least 3:1 against `--tl-pitch`, otherwise `--tl-pitch-line`; the number is white or ink, whichever contrasts more with the fill (contract section 3, item 4). Tests: `#000000` clamps to lightness 0.20 and never returns pure black; `#c8102e` survives unchanged; a secondary that fails 3:1 returns the pitch-line fallback; the chosen number colour is always the higher-contrast of the two.
10. **Decoder.** `web/decode.mjs`: the constants ported from `codec.rs` and `frame.rs` — `KIND_KEYFRAME` 1, `KIND_DELTA` 2, `KIND_RESTART` 3, `KEYFRAME_BYTES` 98, `DELTA_BYTES` 47, `PLAYER_COUNT` 22 — and `decodeInto(buffer, prev, out)` reading a `DataView` at fixed little-endian offsets and writing centimetres straight into the caller's arrays. A keyframe or restart writes absolutes; a delta adds signed-byte steps to `prev`. Return the frame kind and the tick. Refuse a short frame or an unknown tag through `viewer.decode_refused` and return `null`. Tests over recorded bytes from step 7: a keyframe round-trips; a 50-frame cycle reconstructs every absolute position exactly; a restart frame is reported as a restart; a truncated frame is refused and named.
11. **History.** `web/history.mjs`: three `Int16Array` buffers sized from `hello.ticks_expected` — ball x/y/height, and players — plus `tickAt(i)`, `append(...)`, and `bytes()` returning the exact sum of `byteLength`. Grow by doubling if a match runs past `ticks_expected`, which stoppage time makes possible. Emit `viewer.history` with `ticks_stored`, `history_bytes`, `page_bytes`, and `budget_bytes` after each keyframe; `page_bytes` is `null` until `performance.measureUserAgentSpecificMemory()` resolves and never blocks a frame. Tests: 270,000 ticks report exactly 25,380,000 history bytes; a read at tick N returns exactly what was written at tick N; growth past `ticks_expected` preserves every earlier tick.
12. **First browser contact — the smallest page.** `web/index.html` with nothing but a script that opens `ws://127.0.0.1:<port>/?v=1`, reads the hello, and logs `viewer.connected`. Run `engine-cli replay --fixture fixture.smfx --web web` and open the printed address in the in-app browser. Capture the console and a screenshot. This is the first time any browser has opened this socket; prove the handshake, the Origin, the version query, and the two isolation headers before a single line of rendering code exists.
13. **Interpolation.** `web/interpolate.mjs`: `between(a, b, t)` with `t` clamped to `[0, 1]`, writing into a caller-supplied output array. The function cannot express a position outside the segment, which is RIM-5 as a constraint rather than a comment. Tests, both `observable: false` acceptance criteria in part: `t = 0` returns `a` exactly; `t = 1` returns `b` exactly; every `t` in `[0, 1]` lies on the segment within one centimetre; `t = 1.5` and `t = -0.5` clamp and never exceed `b` or precede `a`.
14. **Scheduler.** `web/schedule.mjs`: a fixed-timestep accumulator driven by `requestAnimationFrame` timestamps. Each frame advances simulated time by `elapsed x speed`, consumes whole ticks from the buffer, and renders at the fraction between the last two consumed ticks. When ticks owed exceed ticks available the cursor jumps and the skipped count rises; it never renders beyond the newest received tick. Derive the frame rate from consecutive timestamp deltas and report `refresh_hz` beside it. Emit `viewer.tick_skipped` once per second when the count is above zero, and `viewer.frame_budget` every five seconds. Tests over a synthetic stream with no DOM: at 8x with 50 ticks per second arriving, the scheduler consumes 400 ticks per second and the rendered cursor never passes the newest received tick; at 1x no tick is skipped; a 16-millisecond stall does not cause an extrapolation.
15. **Stoppage index.** `web/stoppages.mjs`: one sorted index merged from restart-flagged ticks and event-message ticks, with two entries inside 25 ticks of each other collapsed to the earlier, so a goal and its restart are one stoppage (PO Q4). `next(fromTick)` and `prev(fromTick)`. Tests: a goal at tick T and a restart at T + 12 collapse to one entry at T; a restart with no event stands alone; `next` past the last entry returns `null`.
16. **The mark.** `web/mark.mjs`: one exported function drawing the three zones into a canvas or an SVG from a tile size, a corner radius of size times 0.21, and the three pitch tokens, with the four variants the contract names (contract section 3, item 8). Generate the favicon from it at load; load no image file. Test: the function is deterministic for a given size and colour set.
17. **Pitch renderer.** `web/pitch.mjs`: the coordinate map from the engine's centred metre space to canvas pixels — x from -52.5..52.5 and y from -34..34 onto 616 by 411 — the markings, the 22 markers with their rings and shirt numbers from `colour.mjs`, and the ball with its falling-alpha trail drawn as one path. Size the backing store by `devicePixelRatio` and scale the context. Use `getContext('2d', { alpha: false })`, integer coordinates, and one `fillStyle` change per team rather than per marker. Pre-render the static markings to an offscreen canvas once and blit them each frame. Covers contract inventory items 2, 4, and 5.
18. **Page shell, tokens, fonts, and the control primitive.** `web/tokens.css` as the single `:root` block with all 36 tokens from contract section 4; `web/layout.css` with the 1280 by 800 grid — 56-pixel header, columns 336 / 616 / 296, 8-pixel gutter — and the header and both side columns present and empty (PO Q10); `web/fonts/fonts.css` with `@font-face` at `font-display: swap` for Barlow Condensed 700 and IBM Plex Sans 400/500/600, all four `woff2` under `web/fonts/`, never a font service; `web/components/match-control.css` with all four heights, five states, and both themes, of which this slice uses `sm` and `md`. Add the same ten new rows to `DESIGN.md`. Covers contract inventory items 1, 6, and 7. Verify the numeric row measures at least 4.5:1 before any transition is written — the brief's named top risk.
19. **Socket client and playback.** `web/socket.mjs`: `binaryType = 'arraybuffer'`, the hello decoded once, binary frames to `decode.mjs` and text frames to the message handler, and `viewer.connected` and `viewer.socket_drops` at the two ends. `web/playback.mjs`: play and pause, the 1x to 8x speed selector, skip to next stoppage, the rewind scrubber over the history, and the sustained-rate estimator that compares tick arrival against the requested speed, drops playback to the measured rate, and emits `viewer.lag`. The notice names the rate in words and digits and carries `--tl-warning`, never colour alone.
20. **Wire the page and the test hook.** `web/main.mjs`: build the shell, start the socket, drive the scheduler, render through `pitch.mjs`, and expose `window.__touchline` with `lastRendered()` (the exact positions of the last drawn frame), `signals` (the ring), `history` (`ticks_stored`, `history_bytes`, `page_bytes`), and `frame` (`fps_median`, `frame_ms_p95`, `dropped_frames`, `refresh_hz`). The hook is the observability seam three criteria read; it is a read-only view and exposes no setter. Add the canvas accessibility treatment: `role="img"` with a label, plus a visually hidden `aria-live="polite"` region mirroring clock, speed, and lag in text.
21. **Documentation.** `docs/reference/protocol.md`: the hello table gains `team.kit.primary` and `team.kit.secondary`, and a new section documents the static server, its MIME table, and the two isolation headers. `README.md` gains the run instructions the shape's Documentation Plan requires: build the engine, run `engine-cli serve --seed 42 --web web`, open the printed address. Confirm `crates/protocol/tests/document.rs` still passes, since it parses these tables.
22. **The full check.** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (122 existing plus the new Rust tests), and `node --test web/tests/`. Then the browser drives of the five user-observable criteria against the served page, per `## Test / Verification Plan`. Record the four browser benchmark targets against their budgets in `05c-benchmark.md`.

## Verification Strategy

Eight acceptance criteria from `03-slice-viewer-pitch.md`. Six are user-observable after plan Round 3 Q9 moved AC-g's gauge into the browser; two are pure-function unit tests.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| **AC-a** 5 minutes at 1x holds 60 fps with zero dropped ticks | in-app browser drive, console read of `viewer.frame_budget` (`web-2`) | Windows 11, Node not needed, built engine, served page, `fixture.smfx` from seed 7 — **yes** | The `--web` server (step 5); the `viewer.frame_budget` signal with `refresh_hz` (step 14); the `window.__touchline.frame` hook (step 20) | timestamp-delta read from the hook → `node --test` over the scheduler with a synthetic 5-minute stream → pre-registered deferral |
| **AC-b** a rendered marker lies on the segment between two ticks | `node --test` over `interpolate.mjs` (`web-1`) | Node 22 — **yes** | `interpolate.mjs` as a DOM-free module (step 13) | none needed; no environment dependency |
| **AC-c** at 8x the viewer skips ticks and never passes the newest tick | `node --test` over `schedule.mjs` with a synthetic stream (`web-1`) | Node 22 — **yes** | `schedule.mjs` as a DOM-free module with an injectable clock (step 14) | none needed; no environment dependency |
| **AC-d** at 4x the clock advances four times wall time within 2 percent | in-app browser drive reading the clock at two wall times (`web-2`) | as AC-a — **yes** | The clock readout in the shell (step 18); the speed selector (step 19) | read `tick` from the hook at two wall times instead of the rendered clock → pre-registered deferral |
| **AC-e** at a sustained 3x, playback drops to 3x and a notice names 3x | in-app browser drive with `replay --speed 8 --sustain 3` (`web-2`) | as AC-a **plus a server that can sustain less than it is asked for** — **needs the flag built** | `--sustain` on `ReplayOpts` and the paced `min(speed, sustain)` in `Replayer::serve` (step 4); the sustained-rate estimator and the notice (step 19); `viewer.lag` (step 14) | read `viewer.lag` from the hook instead of the rendered notice → `node --test` over the estimator with a synthetic slow stream → pre-registered deferral |
| **AC-f** rewind to a stored tick draws exactly the stored positions | in-app browser drive plus `window.__touchline.lastRendered()` (`web-2`) | as AC-a — **yes** | The scrubber (step 19); the `lastRendered()` hook (step 20); a fixture with a noted goal tick (step 7) | compare `history.tickAt(N)` to `lastRendered()` in the hook → pre-registered deferral |
| **AC-g** a full match keeps the stored history under 300 MB | two-part: `node --test` for `history_bytes`, in-app browser drive for `page_bytes` (`web-1` and `web-2`) | Node 22 for part one; for part two a page served with both isolation headers — **yes, once step 5 sends them** | The `bytes()` accounting (step 11); the two isolation headers on every response (step 5); `viewer.history` (step 11) | if the browser never resolves its measurement, `history_bytes` alone is reported and the shortfall is named — never silently passed |
| **AC-h** the charter scenario steps 1, 4, 5 against the live engine | in-app browser drive against `engine-cli serve --seed 42 --web web` (`web-2`) | Windows 11, built engine, default teams — **yes** | `--web` on `ServeOpts` (step 6); the whole page | drive against the replayer instead of the live engine, and name the residual → pre-registered deferral |

**Force-scope rule applied.** One environment dependency sits on a user-observable criterion's critical path: AC-e needs a server that sustains a lower rate than the manager asks for, and none exists.

- `constraint-resolution: prerequisite-slice: viewer-pitch` — `wall-ownership: code-owned`.
- **Classification first, per the ladder's triage question.** Would a change to code in this repository dissolve the wall? Yes: `ReplayOpts` is in `crates/engine-cli/src/cli.rs:106-114` and `Replayer::serve` is in `crates/stream/src/replay.rs:64-105`, both in this workspace. The wall is therefore `code-owned`, and option 2, `proxy+deferral`, is unavailable to it. It is dissolved by scoping the harness or refused on the record; it is never parked behind a clearing event.
- **Cost of the wall.** `wall-cost: retire ≈ one CLI option, one `min()` in the pacing loop, and one test — about 30 lines | carry = 1 deferred AC across 4 slices (`viewer-pitch`, `viewer-match-day`, `viewer-reports-recovery`, `integration`) riding "the live engine happens to fall behind", which is not an act anyone can perform on demand`. Retiring it is cheaper than carrying it and the harness is in the plan as step 4.
- No other criterion names a credential, a device, an external service, an inbound callback, or a deploy target. No `runtime-evidence-deferrals` entry is inherited and none is created.

**Tooling resolution.** Two capabilities the slice's `verify:` stubs assume were checked against `stack:`:

1. The stubs name "the mock server's rate cap flag". It did not exist. Resolved above by scoping the harness into step 4, not by a verify-time bootstrap.
2. The stubs assume a JavaScript test runner. `stack.testing` lists `cargo-test` only, and `stack.package-managers` lists `cargo` and `npm`. Node 22.15.0 ships `node:test` and `node:assert` with **no install**, confirmed by `node -e "require('node:test')"`. Using a runtime already in `toolchains-present`, with nothing fetched and nothing added to the repository, is not a new tool, so no route back through shape is needed. The product owner confirmed the shape at plan Round 2 Q7. Playwright stays absent and unused; the interactive driver is `Claude_Browser`, the tool shape Q29 chose.

**AC-a's measurement circularity is named, not hidden.** The frame-rate criterion is evidenced by a counter the page itself computes, and no independent frame-rate instrument exists in `stack:`. Two guards make the self-report trustworthy rather than tautological: the counter is derived from consecutive `requestAnimationFrame` timestamp deltas, not from a callback count, and `refresh_hz` is reported beside it, so a 144 hertz panel reading 144 frames per second is visibly a panel fact and not a pass. A screenshot of a visibly smooth pitch accompanies every reading.

## Test / Verification Plan

### Automated checks

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`: blocking, as in all three prior slices.
- `cargo test --workspace`: the 122 existing tests plus the new ones. `crates/protocol`: the `TeamRef` round trip with the two kit fields, the `MESSAGES` completeness test, and `tests/document.rs` against the updated hello table. `crates/stream`: a hello built from the shipped defaults carries `#c8102e`; a replay's hello is byte-identical to the recorded one; a fixture whose first entry is not a hello is refused naming the path; `--speed 8 --sustain 3` delivers at three times real time within 5 percent. `crates/engine-cli`: the page address is printed, a missing `--web` directory is refused naming the path, a request path containing `..` is refused with 403, a `.mjs` response carries `text/javascript`, every response carries both isolation headers, and no help line exceeds 80 columns.
- `node --test web/tests/`: six files, no install, no `package.json`. `interpolate.test.mjs` (AC-b), `schedule.test.mjs` (AC-c), `history.test.mjs` (AC-g part one), `decode.test.mjs`, `colour.test.mjs`, `stoppages.test.mjs`.
- Benchmark compare: `/wf verify` loads `augment/benchmark.md` in compare mode against `05c-benchmark.md` — five engine targets with tripwires at 427 milliseconds of processor time and 7.46 megabytes, and four browser targets measured for the first time against their budgets.
- Instrument key check: `04b-instrument.md` section 2 against the live records — `web.serving` and the extended `fixture.replayed` on stderr; `viewer.connected`, `viewer.socket_drops`, `viewer.tick_skipped`, `viewer.lag`, `viewer.frame_budget`, `viewer.history`, `viewer.rewind`, and `viewer.decode_refused` in the browser console and in `window.__touchline.signals`.

### Interactive verification (human-in-the-loop)

Platform `web` from `stack.platforms`; driver `Claude_Browser`, the in-app browser the product owner chose at shape Q29 and `00-index.md` records as "chosen interactive driver". Playwright is in `toolchains-absent` and is not used. Evidence follows the `web` runtime adapter's layout under `verify-evidence/viewer-pitch/`: `<criterion>-t0.png`, `-t250.png`, `-final.png`, `<criterion>.console.log`, `<criterion>.network.json`, and `video/<criterion>.webm`. Web Vitals come from the page's own Performance API through the browser's script tool, because no Chrome DevTools Protocol path exists through this driver; the cross-browser sweep is not available from a single Chromium-based pane and its absence is reported under `## Cross-Browser Delta` rather than left unexplained.

Bootstrap, replacing the adapter's `npm run dev` step, which does not exist here:

```bash
cargo build --release -p engine-cli
target/release/engine-cli.exe record --seed 7 --out fixture.smfx --minutes 90
target/release/engine-cli.exe replay --fixture fixture.smfx --speed 1.0 --web web
# then open the printed page address; for AC-h use:
target/release/engine-cli.exe serve --seed 42 --web web
```

| Criterion | Drive | Pass criteria |
|---|---|---|
| AC-a | Open the page at 1x against the seed-7 fixture. Sample `window.__touchline.frame` every 30 seconds for 5 minutes of match time. Screenshot at t0, mid-run, and final. | `fps_median` at least 60, `dropped_frames` 0, `viewer.tick_skipped` never emitted, `refresh_hz` recorded beside the reading |
| AC-d | Set 4x. Read the on-screen clock and `performance.now()` at two points about 60 wall seconds apart. | Match time advances 4.00 times wall time within 2 percent |
| AC-e | Restart the replayer with `--speed 8 --sustain 3`. Select 8x in the page. Then restart with `--speed 8` alone and select 8x again. | First run: playback settles at 3x, the notice reads "3x" in digits and words, `viewer.lag` carries `requested_speed` 8 and `sustained_speed` 3. Second run: no notice, no `viewer.lag` row |
| AC-f | Play past the fixture's noted goal tick, drag the scrubber back to it, and read `window.__touchline.lastRendered()`. | Every one of the 23 returned positions equals `history.tickAt(goalTick)` exactly; `viewer.rewind` reports `exact: true` |
| AC-g | Load the full 90-minute fixture, then read the gauge. | `history_bytes` equals 25,380,000; `page_bytes` resolves and is under 314,572,800. A `page_bytes` of `null` is a failed drive, not a pass |
| AC-h | Run `engine-cli serve --seed 42 --web web`. Execute charter-scenario steps 1, 4, and 5. | Step 1: the page reports the engine version from the hello. Step 4: the clock advances, 22 markers and the ball move every tick, no marker leaves the pitch bounds. Step 5: 4x advances the clock four times faster. Steps 2 and 3 use a default lineup until `viewer-lineup-tactics` ships, as the slice states |

Human-in-the-loop, not automatable: the design-contract check. A reviewer reads the numeric row at 1280 by 800 in a lit room and confirms at least 4.5:1, confirms focus rings are visible on every playback control, and confirms no anti-pattern from `02c-craft.md` section 5 appears. This is AC-29 of the shape, partially discharged here and completed at `integration`.

## Risks / Watchouts

- **R1 (high) the refresh-rate false pass.** `requestAnimationFrame` fires at the display rate, so a callback count on a 120 or 144 hertz panel reports 120 or 144 and a naive assertion passes for the wrong reason. Derive frame rate from consecutive timestamp deltas and report `refresh_hz` beside every reading (step 14). Verify records both numbers or the drive does not count.
- **R2 (high) the browser has never opened this socket.** Three slices verified the protocol against a Rust client only (`06-verify-stream-protocol.md:241`). Step 12 is a deliberate ten-line page that proves the handshake, the Origin, the `?v=1` query, and the two isolation headers before any rendering code exists to blame. Watch in particular that `Cross-Origin-Embedder-Policy: require-corp` does not refuse the four `woff2` files; they are same-origin, so it should not, and step 12 is where that is confirmed.
- **R3 (medium) interpolation that quietly extrapolates.** This is RIM-5 and commitment C2. The mitigation is structural, not a review note: `between(a, b, t)` clamps `t` into `[0, 1]` and cannot express a position outside the segment, and the scheduler moves the cursor rather than the fraction. Two unit tests drive `t = 1.5` and `t = -0.5` directly.
- **R4 (medium) the `--sustain` cap is only a test harness.** It exists to make AC-e drivable and must never be reachable from a real match. It sits on `replay` alone, never on `serve`, and the help text says "for testing the viewer's lag notice". Watch that a later slice does not promote it.
- **R5 (medium) the isolation headers restrict what the page may load later.** `require-corp` refuses any cross-origin subresource. Everything this page loads is same-origin, so nothing is lost today, but a later slice that wants an external resource will hit it. `web.serving` reports `isolated` so a run with the headers off is visible, and the constraint is recorded in `docs/reference/protocol.md`.
- **R6 (medium) the fixture must be re-recorded.** The stored hello is forwarded verbatim from step 3 onward, so a fixture recorded before step 1 carries no kit colours and the page would draw grey markers. Step 7 re-records, and step 3's test asserts byte-identity so a stale fixture fails loudly rather than rendering wrongly.
- **R7 (medium) history growth past `ticks_expected`.** Stoppage time makes a match run longer than the hello predicts. The buffers double rather than refusing, and a test drives growth and asserts every earlier tick survives.
- **R8 (low) a path-traversal request to the static server.** The server serves a directory the operator names, on loopback, to a browser the operator opened. Refuse `..` and any absolute path with 403 anyway, and serve `GET` and `HEAD` only.
- **R9 (low) `page_bytes` never resolves.** `performance.measureUserAgentSpecificMemory()` is asynchronous and the browser may coalesce or defer it. Write `null`, never block a frame, and treat a `null` at verify as a failed drive rather than a pass — the exact byte count is reported beside it and is always available.
- **R10 (low) the `viewer-event` record kind is outside the contract's four.** Emit it as an additive extra, exactly as `content.hash` and `change.queue_id` were, and let `/wf observability audit` settle it.
- Watch: JSON keys are dotted, not camel case. `protocol.version`, not `protocolVersion`. A decoder that camel-cases them reads `undefined` and draws nothing.
- Watch: home team is first in `hello.teams` and first in the 22 player slots. Reversing them swaps the kits on every marker with no error anywhere.

## Dependencies on Other Slices

- `engine-core` (complete): the tick loop and the pitch geometry the renderer maps from.
- `data-schemas-generator` (complete): the team files whose kit colours now ride in the hello.
- `stream-protocol` (complete): the socket, the codec this slice mirrors in JavaScript, the fixture, and the replayer. This slice changes three of its files — `message.rs`, `replay.rs`, `server.rs` — and adds no new crate.
- `viewer-match-day` (next viewer slice): fills the header, both side columns, and the statistics region this slice leaves empty, and inherits the tokens, the fonts, the `match-control` primitive in all four sizes, and the mark function.
- `viewer-lineup-tactics` (later): the pending chips whose four state words `stream-protocol` already fixed.
- `viewer-reports-recovery` (later): completes the error and recovery states this slice stubs, and owns the saved replay file format.
- `match-rules` (later): adds the event types that make the stoppage index meaningful beyond kick-off, goal, and full time.
- `/wf observability` (open): owns the browser transport the page-side signals do not have.

## Assumptions

- `PROTOCOL_VERSION` stays 1. Two fields are added to a message, no field changes meaning, no message is removed, and both producers are updated in the same commit. A JavaScript client ignores an unknown field, and the Rust decoder's `deny_unknown_fields` only matters for a Rust client built from the same commit. Step 1 records the judgement beside the constant so a later reviewer sees it was made, not missed.
- The `web/` folder name is this plan's choice; no convention existed to follow. It sits at the repository root beside `crates/` and `content/`, so `--web web` is the natural invocation from the workspace root.
- The four `woff2` files enter the repository as binary assets. They are fonts under the SIL Open Font License, not brand images, so the product owner's "generate assets in JavaScript" rule does not reach them; `steer.md` requires a local bundle explicitly.
- One client per match in version 1, unchanged from the stream slice. The static server is a separate listener and serves any number of page loads.
- Fixtures stay gitignored. Step 7 records the seed and the noted ticks in the implement artifact so any machine rebuilds the same file.
- The `experiment` augmentation stays deferred to the `experiment-flags` slice, per the shape.
- Consult: triggers `appetite-medium-or-larger` and `unknowns-present` hold. The product owner excluded `consult` at intake. Not fired.

## Blockers

None.

## Freshness Research

- Source: MDN Secure Contexts, and the WICG Local Network Access draft dated 2026-08-07. Why it matters: whether the page can reach the engine at all, and whether a permission prompt appears. Takeaway: `http://localhost`, `http://127.0.0.1`, and `file://` are potentially trustworthy origins; the LNA draft exempts requests originating from a loopback address entirely, and WebSocket is not yet gated by LNA (crbug.com/421156866, still open). No prompt to design around; watch that bug before distribution.
- Source: WHATWG HTML issue tracker on module scripts over `file://`, reproduced in a browser. Why it matters: it decided how the page is delivered. Takeaway: a `file://` page loading a module script is blocked by CORS with origin `null`, absolutely and with no partial case. This is why the engine serves the page (PO Q1).
- Source: MDN `WebSocket.binaryType` and `WebSocket.bufferedAmount`. Why it matters: the receive path and the lag notice. Takeaway: `'arraybuffer'` delivers bytes synchronously in the message event while `'blob'` forces an asynchronous step per frame; `bufferedAmount` counts bytes queued by the page's own `send()` and says nothing about inbound backlog. MDN states plainly that the API offers no receive-side backpressure, which is why the lag notice is self-measured.
- Source: v8.dev/blog/dataview. Why it matters: the decoder's cost. Takeaway: V8 closed the `DataView` penalty at 6.9; explicit little-endian reads at fixed offsets now match typed-array views, so no manual overlay and no parser library is needed.
- Source: MDN Optimizing Canvas, and MDN `devicePixelRatio`. Why it matters: the frame budget. Takeaway: `alpha: false`, integer coordinates, batched paths, minimal state changes, and pre-rendered static content; text is the most expensive primitive, and 22 short labels is still a small fraction of 16.6 milliseconds. Backing store must be scaled by `devicePixelRatio` for crisp markings.
- Source: MDN `OffscreenCanvas`, `transferControlToOffscreen`, and `DedicatedWorkerGlobalScope.requestAnimationFrame`. Why it matters: the rendering topology decision. Takeaway: Baseline since March 2023; `WebSocket` works in a dedicated worker and `requestAnimationFrame` exists there only when the worker owns an `OffscreenCanvas`; the transfer is one-way and the main thread can never draw to that canvas again. Available and not needed (PO Q5).
- Source: MDN `requestAnimationFrame`, and Fiedler's "Fix Your Timestep!". Why it matters: the scheduler. Takeaway: the callback receives a `DOMHighResTimeStamp` on the `performance.now()` origin; the fixed-timestep accumulator with an interpolation fraction is the canonical shape, and clamping the accumulator rather than catching up is the standard guard. `rAF` follows the display rate and can exceed 60 hertz, which is R1.
- Source: MDN `performance.memory`, the WICG `measureUserAgentSpecificMemory` explainer, web.dev on COOP and COEP, MDN `navigator.deviceMemory`. Why it matters: the memory gauge. Takeaway: `performance.memory` is deprecated and non-standard; `measureUserAgentSpecificMemory()` needs `Cross-Origin-Opener-Policy: same-origin` plus `Cross-Origin-Embedder-Policy: require-corp` to make the page cross-origin isolated, which a `file://` page and a bare static server cannot supply; `navigator.deviceMemory` reports the device's RAM tier, not the page's use. This is why the engine's server sends both headers (PO Q8).
- Source: Node 22 documentation for `node:test` and ESM resolution; confirmed on this machine at v22.15.0. Why it matters: the test seam. Takeaway: `node --test` runs ESM with no install; with no `package.json`, the `.mjs` extension is the only way to mark a file as a module; relative specifiers need explicit extensions in both Node and the browser, so one spelling works in both. Node also ships a global `WebSocket`, which is not needed here but is available.
- Source: Python `mimetypes` issue bpo-31715 on `.mjs`. Why it matters: it removed `python -m http.server` as a candidate. Takeaway: a static server that mis-types `.mjs` makes strict module-MIME checking refuse the script outright. The Rust server owns its own MIME table and the problem disappears.
- Source: SIL Open Font License; github.com/IBM/plex; the Google Fonts specimen for Barlow Condensed. Why it matters: NFR-5 and the local bundle. Takeaway: both families are OFL, so bundling and redistributing the files is permitted; `woff2` is the only format a 2026 Chrome or Edge target needs; IBM Plex Sans carries real OpenType tabular figures, so `font-variant-numeric: tabular-nums` does what the contract expects.
- Source: MDN `prefers-reduced-motion`, MDN `:focus-visible`, current guidance on accessible data-bearing canvas. Why it matters: NFR-6. Takeaway: `:focus-visible` is the right hook for the control strip; a canvas carrying data gets `role="img"` with a label plus a visually hidden `aria-live="polite"` text mirror of the changing state, rather than an attempt to expose pixels; reduced motion disables decoration, never the simulation redraw.

## Recommended Next Stage

- **Option A (recommended): `/wf implement football-manager-match-engine viewer-pitch`** — the plan is complete, no blocker exists, the visual contract is written, and both augmentation artifacts are re-authored and ready. Compact the session first; the SessionStart hook re-reads the artifacts after compaction.
- **Option C: `/wf slice football-manager-match-engine`** — only if the static file server and the kit-colour protocol change are judged to belong to `stream-protocol` rather than here. This plan keeps them because neither has a consumer until a page exists, and both would otherwise ship unverified.
