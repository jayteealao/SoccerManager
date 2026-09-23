---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: viewer-lineup-tactics
status: complete
stage-number: 5
created-at: "2026-09-23T10:43:56Z"
updated-at: "2026-09-23T10:43:56Z"
metric-files-changed: 37
metric-lines-added: 3843
metric-lines-removed: 162
metric-deviations-from-plan: 14
metric-review-fixes-applied: 0
commit-sha: "b46f0832ef0b17e18ea5ab289da231bda417bc81"
commits:
  - "b46f0832ef0b17e18ea5ab289da231bda417bc81"
steering-honored:
  - "Tokens: every colour in web/components/chip.css and web/components/editor.css is a --tl- token (--tl-success, --tl-warning, --tl-danger, --tl-fg-muted and the existing surface tokens). No new colour value, no second prefix."
  - "State words: every chip renders its word as text beside the state colour: Queued, Applies now, Applied, Rejected (web/pending.mjs:13 STATE_WORDS). Fit and fitness show a number and a word band (web/lineup.mjs fitWord, fitnessWord)."
  - "No spinners: kick-off shows a disabled control reading 'Kicking off' while it waits (web/lineup-editor.mjs:361-362); no spinner element was added."
  - "Layout 1280 x 800 with no page scroll: measured document.documentElement.scrollHeight = 800 and scrollWidth = 1280 pre-match and live on seed 11 in the in-app browser."
  - "Product-owner answers OQ-1 = 1 and OQ-2 = 1: protocol version 3 carries the squad, the setup, the tactics file, and the substitution limits; serve holds before kick-off; set-lineup is a new command."
  - "Output boundary: the commit message, code comments, and docs/reference/protocol.md use product language; a scan of every added line for workflow vocabulary found only the code method `slice`."
tags: [viewer, lineup, tactics, substitutions, protocol-v3, flow-control]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-viewer-lineup-tactics.md
  plan: 04-plan-viewer-lineup-tactics.md
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-commentary.md, 05-implement-calibration.md, 05-implement-viewer-match-day.md]
  verify: 06-verify-viewer-lineup-tactics.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine viewer-lineup-tactics"
---

# Implement: Lineup Editor, Tactics Panel, and Substitutions

## The Implementation

The page could watch a match but not manage one. The engine already applied queued changes at stoppages, but a change sent over the socket stopped at the stream's queue, and `serve` simulated a whole match in about 13 seconds before any person could act. The work now bridges the socket into the engine. `serve` holds before kick-off (`Gate::held`), takes the page's lineup and pre-match patch through a new `set-lineup` command, and builds the match from them (`MatchConfig::with_setup`). During play, an admitted `queue-change` is parsed into a typed detail, placed in an inbox, and drained into `Simulation::queue_change` only while the match runs. The verdict event carries the page's own change identifier.

Two decisions carry the weight. First, the page paces the engine with the existing `pause` and `start` commands. It pauses once it holds more than 5 seconds of undrawn match and restarts it below 2 seconds (`web/lead.mjs`). It paces on every tick arrival as well as on every drawn frame, because a throttled tab draws few frames. The first drive paced from frames only: the engine ran to full time at tick 284950 while the page had drawn 790 ticks. The second drive, pacing on arrival too, held the engine to within 1238 ticks at 8x speed with 19 pauses. Second, the hello grew past the socket's 8 KB write bound, so tungstenite refused it and every client saw a reset. `send_whole` lifts the bound for that one message only. The run ends with 290 Rust tests and 103 page tests passing, clippy clean, and one live drive on seed 11 in which a mentality change went from Queued to Applied at tick 1992, with the feed row "Tactical change applied".

Verify now drives AC-a to AC-f against the live engine. The top open risk is AC-e timing under page flow control. The engine test proves a change made while paused applies at the next stoppage. The live drive has not yet read `applied_tick` against a stoppage event tick with the page's own pause in play.

## Summary of Changes

- Protocol version 3 additions: `TeamRef.squad` (`SquadEntry` with role fit and natural fitness) and `TeamRef.setup` (lineup, bench, formation, mentality, instructions, roles). The hello gains `tactics` (the tactics file plus `instruction_order`) and `substitutions` (`limit`, `windows`). New types: `ClientCommand::SetLineup`, `PatchWire`, `ChangeDetail`, and `QueueChange::detail_typed()`.
- The stream crate gains a held gate with a `started` flag, `PreMatch`, `LineupRules`, `check_lineup`, `Inbox`/`Admitted`, `engine_change`, `tactics_patch`, and the `set-lineup` and pre-start `queue-change` verdicts.
- The engine gains `MatchConfig::with_setup` and `TacticsPatch::applied_to`. The latter is shared by the pre-match patch and the in-play change.
- `serve` holds, applies the page setup, and bridges the inbox. `record` and `bench` keep an open gate and no inbox.
- New page parts: the lineup rules, the lineup editor with kick-off, the tactics panel, the substitution picker, the pending-change list with chips, and page-side lead control. They are wired in `web/main.mjs` with the `pending()`, `lineup()`, and `dugout()` test readers.
- `docs/reference/protocol.md` documents version 3: the hold, `set-lineup`, the new hello fields, and the queue-change detail shapes.

## Files Changed

- `crates/protocol/src/message.rs`: version 3 squad, setup, tactics, and substitution fields; `set-lineup`; the typed change detail parsed by kind; `Hello` boxed in `ServerMessage`.
- `crates/protocol/src/lib.rs`: re-exports, the version 3 paragraph, and the `MESSAGES` field lists for the document test.
- `crates/protocol/src/command.rs`: the module doc says socket changes now reach the engine.
- `crates/stream/src/control.rs`: the held gate, pre-match lineup rules and verdicts, the inbox, and the change mapping, with tests.
- `crates/stream/src/lib.rs`: re-exports and `send_whole` for a hello longer than the write bound.
- `crates/stream/src/session.rs`: hello via `send_whole`; the gate stops when the socket pump ends, so a held producer never hangs.
- `crates/stream/src/replay.rs`: frames via `send_whole`; hello unboxed.
- `crates/stream/tests/common/mod.rs`, `crates/stream/tests/fixture.rs`: the new `TeamRef`, `Hello`, and `CommandContext` fields.
- `crates/stream/tests/commands.rs`: the substitution detail now uses `{off, on}`.
- `crates/engine/src/sim.rs`: `MatchConfig::with_setup`, with a slot-order test.
- `crates/engine/src/tactics/mod.rs`: `TacticsPatch::applied_to`.
- `crates/engine/src/tactics/change.rs`: `retactic` uses `applied_to`.
- `crates/engine-cli/src/stream_run.rs`: inbox drain, engine-to-page id map, hello squad, setup, tactics, and substitutions builders, with three new tests.
- `crates/engine-cli/src/serve.rs`: held gate, pre-match rules, page setup applied before the simulation is built, client-gone exit during the hold.
- `crates/engine-cli/src/record.rs`, `crates/engine-cli/src/bench.rs`: the new hello builders and an absent inbox.
- `crates/engine-cli/tests/stream_cli.rs`: the serve tests send `start` after connecting.
- `docs/reference/protocol.md`: version 3.
- `web/lineup.mjs` (new): lineup legality and reasons; fit and fitness words.
- `web/pending.mjs` (new): the pending-change list and state words.
- `web/lead.mjs` (new): page-side lead control.
- `web/lineup-editor.mjs` (new): formation picker, slots, bench, squad list, click and drag placement, kick-off.
- `web/tactics-panel.mjs` (new): mentality, instructions, roles and duties from the hello tactics; `detailFor`, `labelFor`, `applyPatch`, `prematchPatch`.
- `web/substitution-picker.mjs` (new): off and on lists, the remaining count, queue.
- `web/components/chip.css`, `web/components/editor.css` (new): chip, editor, tactics, and picker styles.
- `web/main.mjs`: the `Dugout` controller, message routing, pacing, and test readers.
- `web/index.html`: editor, board, and squad regions; the tactics column under the feed.
- `web/feed.mjs`: change rows in words.
- `web/match-state.mjs`: tactics-change rows without a team are left out of the feed.
- `web/socket.mjs`: `send(command)`.
- `web/tests/lineup.test.mjs`, `web/tests/pending.test.mjs`, `web/tests/lead.test.mjs`, `web/tests/tactics-panel.test.mjs` (new); `web/tests/feed.test.mjs` (change-row test).

## Shared Files (also touched by sibling slices)

- `crates/protocol/src/message.rs`, `crates/protocol/src/lib.rs`, `docs/reference/protocol.md`: also changed by `stream-protocol` and `viewer-match-day`. Version 3 now carries both slices' fields.
- `crates/engine-cli/src/stream_run.rs`: `hello_teams` came from `viewer-match-day` and is extended, not duplicated.
- `crates/engine/src/sim.rs`, `crates/engine/src/tactics/*`: owned by `engine-core` and `tactics-and-ai`.
- `web/main.mjs`, `web/index.html`, `web/feed.mjs`, `web/match-state.mjs`: `viewer-pitch` and `viewer-match-day`. `viewer-reports-recovery` will inherit the `Dugout` phases.

## Notes on Design Choices

- The chip never predicts. Its state comes only from `ack`, `reject`, and the engine's `tactics-change` events. It shows a verdict once the drawn tick reaches the verdict tick, so the pitch and the chip agree.
- The engine and the computer manager share one change counter. The driver therefore keeps `engine id → page id` and writes the page id into the event (`crates/engine-cli/src/stream_run.rs:335`).
- `ChangeDetail` is parsed by `change.kind` rather than as an untagged enum. A detail that does not match its kind is refused with a reason that names the kind.
- The pre-match patch and the in-play change share `TacticsPatch::applied_to`, so a formation change resets unsuitable roles the same way before kick-off and during play.
- The page's Pause button stays a local playback pause. Engine pause and start are sent only by lead control.

## Verification Seams Built

- AC-a (illegal lineup disables kick-off with a reason) → `data-testid` `slot-n`, `bench-n`, `squad-i`, `formation`, `lineup-reason`, `kick-off` at `web/lineup-editor.mjs:26`, `:76`, `:123`, plus `__touchline.lineup()` at `web/main.mjs:716`. This lets a browser drive read `legal` against the control's `disabled` state. The rules are unit-tested in `web/tests/lineup.test.mjs`.
- AC-b (mentality change: chip, then applied) → `data-testid` `mentality` at `web/tactics-panel.mjs:190`, chip `chip-<queue_id>` at `web/main.mjs:569`, and `__touchline.pending()` at `web/main.mjs:715` with `queued_tick` and `applied_tick`. The bridge test is at `crates/engine-cli/src/stream_run.rs:659`.
- AC-c and AC-d (substitution applies; the sixth is rejected) → `sub-off`, `sub-on`, `sub-queue`, `subs-left` at `web/substitution-picker.mjs:41-53`; `__touchline.dugout().subs_left` at `web/main.mjs:717`.
- AC-e (a change while paused applies at the next stoppage) → the inbox is drained after the gate wait at `crates/engine-cli/src/stream_run.rs:76`; the engine test is at `crates/engine-cli/src/stream_run.rs:713`; `pending()` exposes both ticks; `__touchline.dugout().lead_holding` and `lead_pauses` separate the page's pacing pauses from the manager's pause.
- AC-f (keyboard only) → native `select` and `button` controls throughout; `:focus-visible` rings at `web/components/editor.css:41-43`.
- Hold and pre-match verdicts → `Gate::held` at `crates/stream/src/control.rs:51` and `check_lineup` at `crates/stream/src/control.rs:238`, with unit tests in the same file.

## Visual Contract Honored

- Item 1, the 1280 × 800 grid → honored. Measured scrollHeight 800 and scrollWidth 1280 pre-match and live. Deviation: before kick-off the editor borrows the left column, the pitch tile (`#board` over the pitch), and the statistics region (`#squad`), because the squad list and eleven slots do not fit a 296-pixel column. After kick-off they hand back to the match-day panels.
- Item 6, type and tabular numbers → honored. Fit, fitness, and the remaining count use the existing `.tl-num` treatment. No new font.
- Item 7, `--tl-` tokens → honored in `web/components/chip.css:39-70` and `web/components/editor.css`.
- Item 9, state words beside every state colour → honored at `web/pending.mjs:13` and in chip rendering at `web/main.mjs:569`.
- Item 10, no spinner → honored. No spinner was added.

## Deviations from Plan

1. No `crates/engine-cli/src/hello.rs`. The shared builder `hello_teams` from the match-day work was extended, as plan step 10 allows.
2. `TeamSetup` also carries formation, mentality, instructions, and roles, so the editor starts from the computer manager's tactics.
3. `squad` and `setup` are sent for the home team only, and only by `serve`. `record` and `bench` send neither, which keeps fixtures small.
4. The plan had no pacing. Page-side lead control was added (`web/lead.mjs`, `web/tests/lead.test.mjs`) using the existing `pause` and `start` commands, because `serve` otherwise finishes the match in about 13 s. Product-owner answers Q4 (client pull, bounded lead) and Q10 (a 500-tick lead) cover it. It paces on tick arrival as well as on drawn frames; see The Implementation.
5. New file `web/components/editor.css`, beyond the plan's `chip.css`.
6. Files changed beyond the plan: `web/socket.mjs`, `web/feed.mjs`, `web/match-state.mjs`, `crates/engine-cli/tests/stream_cli.rs`, `crates/stream/src/session.rs`, `crates/stream/src/replay.rs`, `crates/engine/src/tactics/mod.rs`, `crates/engine/src/tactics/change.rs`, `crates/stream/tests/commands.rs`.
7. `ServerMessage::Hello` is boxed, because clippy `large_enum_variant` refused the grown hello under `-D warnings`.
8. `send_whole`: the hello exceeded the 8 KB write bound. Source: the local cargo registry copy of `tungstenite-0.30.0/src/protocol/frame/mod.rs:254` (`buffer_frame`) refuses a single frame longer than `max_write_buffer_size`. The observed failure was `ResetWithoutClosingHandshake` in both serve tests.
9. The session stops the gate when the socket pump ends, because a held `serve` whose client left otherwise hung. Orphaned test processes showed this.
10. `instruction_order` in the hello tactics. `serde_json::Value` sorts object keys, so the page listed instructions alphabetically and mapped levels to the wrong index. The first drive showed this.
11. `ChangeDetail` is dispatched by kind rather than untagged; see Design Choices.
12. The editor borrows the pitch tile and the statistics region before kick-off; see Visual Contract item 1.
13. The page Pause button stays local; engine pause belongs to lead control.
14. No contract-check or research helper was run. The installed tungstenite source was read directly for deviation 8.

## Anything Deferred

- The live drives for AC-a (four illegal cases with screenshots), AC-c, AC-d, AC-e, and AC-f were not run in this stage. They are verify's browser drives. This stage ran one live drive for AC-b on seed 11 rather than 42.
- The away team's squad is not sent. The page manages the home team only, as the slice scopes it.

## Known Risks / Caveats

- `sdlc-debt:` The lead thresholds are fixed at 5 s and 2 s of match at playback speed (`web/lead.mjs`). At 8x the engine holds up to about 2000 ticks ahead, so a change can take effect up to about 5 s of drawn match after the manager expected. The ceiling is acceptable under the Q10 500-tick guidance only at 1x. Upgrade path: scale the pause threshold to a tick bound rather than seconds.
- AC-e under page flow control: the engine test proves the rule, but the live drive has not compared `applied_tick` with a stoppage event tick while the page's own pauses are in play.
- The first `set-lineup` wins. A second one before `start` replaces it; the page sends one per kick-off press.

## Freshness Research

- tungstenite 0.30.0, the local registry source `src/protocol/frame/mod.rs:254`. Relevance: the socket write bound. Takeaway: a single frame larger than `max_write_buffer_size` is refused outright, not split, so a large hello must lift the bound for that message.
- serde_json `Value` object key order (observed in the first drive). Relevance: the hello tactics. Takeaway: without `preserve_order`, keys are sorted, so order-bearing lists must be sent as arrays.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine viewer-lineup-tactics`. Six browser-observable ACs need live drives against `serve`, and every seam they read is in place.
- **Option B:** `/wf review football-manager-match-engine viewer-lineup-tactics`. Only if verify is deferred; not recommended, because AC-e timing under page pacing is unproven live.
