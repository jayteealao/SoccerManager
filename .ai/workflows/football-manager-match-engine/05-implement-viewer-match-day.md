---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: viewer-match-day
status: complete
stage-number: 5
created-at: "2026-09-23T09:18:45Z"
updated-at: "2026-09-23T09:18:45Z"
metric-files-changed: 32
metric-lines-added: 2521
metric-lines-removed: 118
metric-deviations-from-plan: 9
metric-review-fixes-applied: 0
commit-sha: "4d210411b725f015a6291efad0df2a2d358fd179"
commits:
  - "9e684e901d6649de80c702127ccbfd65c588909d"
  - "4d210411b725f015a6291efad0df2a2d358fd179"
steering-honored:
  - "Design direction: header 56 px carries the score bug; the 336 / 616 / 296 grid, the pitch tile, and the control strip are unchanged; statistics sit under the controls and the feed in the right column (web/index.html, web/layout.css)."
  - "Typography: the score bug and the goal banner use --tl-font-display (Barlow Condensed 700); every changing number uses .tl-num or font-variant-numeric: tabular-nums; both faces load from web/fonts/ only."
  - "Palette: no new colour value; every colour in web/components/panels.css is a --tl- token or a color-mix of two tokens. Token prefix --tl-."
  - "The mark: club crests call drawMark() with the club's safe kit colours in place of the turf (web/scoreboard.mjs:36). No image file added."
  - "State words: cards read Yellow or Red, fatigue bands Fresh, Tiring, or Exhausted, injured rows Injured, sent-off rows Sent off, highlighted feed rows carry the event word. Colour never carries a state alone."
  - "Loading: no spinner added; the skeleton pitch is unchanged."
  - "Product-owner answer Q-1 = A: protocol version 3 with the hello roster, per-second stats with nine fields, and a condition message."
  - "Output boundary: both commit messages, every code comment, and docs/reference/protocol.md use product language; a scan of the changed source for workflow vocabulary found none."
tags: [viewer, panels, feed, statistics, goal-moment, protocol-v3, lineups]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-viewer-match-day.md
  plan: 04-plan-viewer-match-day.md
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-commentary.md, 05-implement-calibration.md]
  verify: 06-verify-viewer-match-day.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine viewer-match-day"
---

# Implement: Match-Day Panels and Goal Moments

## The Implementation

The page this slice inherited drew the pitch and the controls and left four labelled placeholders: the header band, the lineups, the statistics, and the feed. The engine already counted every figure the screen needs, but the wire carried no roster, sent statistics once at full time with seven fields, and sent no energy at all. The product owner chose the wire shape before this run: a roster in the hello, statistics every simulated second with nine panel fields, and a condition message, under protocol version 3.

Three decisions carry the work. First, the wire additions landed first and reuse the engine's own figures: `stats_message` reads `MatchFigures::new` and `Summary`, so the panel and the saved match record round the same way, and `drive` sends statistics and energy every 50 ticks. A 90-minute recording grew from 16.3 MB to 20.5 MB, and the stream still delivers 611,209 ticks per second at 7.61 MB peak memory, under the 8.55 MB tripwire. Second, every panel reads a pure `MatchState` at the tick the pitch is drawing, never at arrival. A goal therefore updates the score, the banner, the feed line, and the commentary line in the frame that reaches its tick, and a rewind restores the earlier score, feed, statistics, and energy without replaying the banner. Third, reduced motion has one CSS rule block: the page mirrors the operating-system preference onto `<html data-motion>`, and `?motion=reduce` sets the same attribute for a browser tool that cannot emulate the query. The build added 13 new files and changed 19, in two commits. It passes 278 Rust tests and 80 JavaScript tests, clippy with warnings denied, and the format check.

Verify can now drive all five user-observable criteria against a fresh version-3 fixture with known goal ticks (103516, 119176, 200722) through `__touchline.matchDay()` and the `viewer.goal_moment` signal. The top open risk is runtime evidence: the browser pane was hidden in this run, so animation frames were throttled. The goal moment during live play, the frame-time readings, and the restated p95 budget are not verified yet. Verify must run them with the pane shown.

## Summary of Changes

- Protocol version 3 (`crates/protocol`): `TeamRef.roster` (`RosterEntry` with `player.id`, `player.name`, `player.shirt`, `player.position`, `player.squad_index`), nine `stats.*` pairs on `Stats`, and a new `Condition { tick, energy }` message (`type: "condition"`). `MESSAGES`, the exhaustive-name test, and the reference document all name the additions.
- One driver change (`crates/engine-cli/src/stream_run.rs`): `hello_teams(&Simulation)`, `stats_message(&Simulation)`, and `condition_message(&Simulation)`. `drive` sends statistics and then condition after each tick's events whenever the tick is a multiple of one simulated second, and sends statistics once more at full time.
- `serve`, `record`, and the streaming `bench` build the simulation before the hello and share `hello_teams`.
- Page: a pure match-state reducer released by rendered tick, a per-frame feed batcher and renderer, the header scoreboard with crests, the lineups with fatigue bars, condition words, card chips, and a bench list, the statistics panel, the goal moment, reduced motion, hidden parked markers, a read-only `matchDay()` hook, a `viewer.goal_moment` signal, and a "Full time" close notice.
- Tests: 4 new Rust tests (roster order; per-second cadence, values, and condition shape; hello without roster; condition tag) plus 3 existing Rust tests extended, and 33 new JavaScript tests (47 before, 80 after) across six new files.

## Files Changed

- `crates/protocol/src/message.rs`: `RosterEntry`, `TeamRef.roster` with `#[serde(default)]`, nine stats fields, `Condition`, `ServerMessage::Condition`; two new tests and the round-trip test extended.
- `crates/protocol/src/lib.rs`: `PROTOCOL_VERSION` 3 with its reason; `MESSAGES` lists the roster fields, the nine stats fields, and `condition`; the exhaustive test names the new variant.
- `crates/engine-cli/src/stream_run.rs`: the three helpers, the per-second send, the shared closing stats; two new tests and a shared test driver.
- `crates/engine-cli/src/serve.rs`, `record.rs`, `bench.rs`: simulation before the hello; `teams: hello_teams(&sim)`.
- `crates/engine-cli/tests/stream_cli.rs`: a one-minute recording now holds 3,127 frames (60 statistics and 60 condition messages added).
- `crates/engine-cli/tests/web_cli.rs`, `crates/stream/src/server.rs`: version assertions read `PROTOCOL_VERSION` instead of a literal 2.
- `crates/stream/tests/common/mod.rs`, `crates/stream/tests/fixture.rs`: test literals fill the new fields.
- `docs/reference/protocol.md`: version 3, the roster table, the new stats cadence and fields, the `condition` section; two stale "version 1" mentions corrected to 3.
- `web/match-state.mjs` (new): `MatchState`, the event-kind table `KIND`, `HIGHLIGHT_WORDS`, `HIDDEN_KINDS`.
- `web/feed.mjs` (new): `FeedBatcher`, `feedRow`, `minuteStamp`, `Feed` (one `DocumentFragment` per frame, polite announcements for goals and cards only).
- `web/stats.mjs` (new): `STAT_ROWS`, `toPanel`, `formatStat`, `StatsPanel` (two tables of five and four rows).
- `web/lineups.mjs` (new): `energyBand`, `cardWord`, `lineupModel`, `benchModel`, `Lineups`.
- `web/scoreboard.mjs` (new): crests, names, score bug, clock, speed.
- `web/goal-moment.mjs` (new): timings, `bannerText`, `watchMotion`, `reducedMotion`, `GoalMoment`.
- `web/components/panels.css` (new): header, banner, feed, lineups, bench, statistics, and the one reduced-motion block.
- `web/index.html`: the header scoreboard, the banner over the pitch, the three filled regions, a feed live region; the clock moved from the control strip to the header.
- `web/main.mjs`: `Panels` (one flush per frame, lineups rebuilt only on a released change), the `matchDay()` hook, the goal signal, the full-time close notice.
- `web/layout.css`: comment updated, the unused control-strip clock rule removed, the `end` notice kind added.
- `web/pitch.mjs`: `isParkingSpot()` port; parked markers skipped in both drawing passes.
- `web/tokens.css`: `--tl-goal-in`, `--tl-goal-hold`, `--tl-goal-out`, `--tl-pulse`.
- `web/tests/helpers.mjs`: `CAPTURED_STATS` (a stats line from the recorded match), `statsMessage`, `eventMessage`, `roster`.
- `web/tests/decode.test.mjs`: the fixture version check reads 3.
- `web/tests/match-state.test.mjs`, `feed.test.mjs`, `stats.test.mjs`, `lineups.test.mjs`, `pitch.test.mjs`, `goal-moment.test.mjs` (new).

## Shared Files (also touched by sibling slices)

- `crates/protocol/src/message.rs`, `crates/protocol/src/lib.rs`, `docs/reference/protocol.md`: `viewer-lineup-tactics` adds its squad, lineup hold, `set-lineup`, and tactics schema under the same version 3.
- `crates/engine-cli/src/stream_run.rs`: shared with `calibration` (`match_event`, `Ids`) and `commentary`.
- `web/main.mjs`, `web/index.html`, `web/layout.css`, `web/tokens.css`: `viewer-lineup-tactics` fills the tactics panel in the right column, and `viewer-reports-recovery` completes the error state and the reports.
- `web/pitch.mjs`: owned by `viewer-pitch`; this slice adds only the parked-marker skip.

## Notes on Design Choices

- The roster is the 11 starters in wire-slot order, then the named bench; `player.squad_index` is the handle the lineup editor uses. A hello without a roster reads as an empty list, but `deny_unknown_fields` still refuses a field a later build adds.
- A tick that is both a per-second tick and the full-time tick carries two stats messages with the same values; the page keeps the newest at or before the rendered tick, so the duplicate is harmless.
- `MatchState.add` resets its view when an event lands at or before the tick already shown, so a late message can never be skipped.
- The feed is oldest first with auto-scroll to the newest row unless the manager scrolled up. The live region announces only goals and cards.
- The lineup column shows two lines per player (position, shirt, name, card; then the bar and the condition word) so a name is readable in a 156-pixel column.
- The statistics panel uses two real tables so each value has a row header and a column header for a screen reader.
- `frame_delta` in `viewer.goal_moment` is 0 when the frame that shows the goal is the first whose rendered tick reached it, and 1 when the goal event arrived after the pitch had passed its tick.

## Verification Seams Built

- AC-1 (goal within one frame) → `__touchline.matchDay()` at `web/main.mjs:358` returning `renderedTick`, `score`, `feedCount`, `lastFeed`, `goalShownAtTick`, `bannerVisible`, `bannerText` (built by `Panels.snapshot()` at `web/main.mjs:254`); `viewer.goal_moment` with `goal_tick`, `rendered_tick`, `prev_rendered_tick`, `frame_delta` at `web/main.mjs:239-246`; a version-3 fixture `fixture.smfx` (seed 7, 90 minutes, goals at 103516, 119176, 200722; gitignored, recorded this run) (enables the in-app browser drive plus a screenshot at the goal tick).
- AC-2 (empty state) → the empty-state text at `web/feed.mjs:11` and `web/index.html:122`, `emptyStateShown` in `matchDay()`, zero model from `toPanel(null)` at `web/stats.mjs:32` (enables a DOM read paused before tick 1).
- AC-3 (fatigue marker with a label) → the `condition` message (`crates/protocol/src/message.rs:116`, sent at `crates/engine-cli/src/stream_run.rs:89-91`); `lineupLabels` in `matchDay()` from `Lineups.labels()` at `web/lineups.mjs:210`; each row's `data-condition` and `.lineup__condition` text (enables the DOM read of the label).
- AC-4 (statistics equal the stream) → `stats_message` at `crates/engine-cli/src/stream_run.rs:118`, `toPanel` at `web/stats.mjs:32`, `CAPTURED_STATS` at `web/tests/helpers.mjs:108` (enables `node --test` and `cargo test`).
- AC-5 (no dropped events at 8x) → `FeedBatcher` at `web/feed.mjs:17` as a pure module (enables `node --test` with a synthetic burst).
- AC-6 (reduced motion) → `watchMotion` at `web/goal-moment.mjs:34` reading `?motion=reduce` and the media query into `<html data-motion>`, served by the one block at `web/components/panels.css:406`; `motion` in `matchDay()` (enables the proxy drive and a computed `transition-duration` read).
- AC-7 (legibility, focus rings) → no new seam: computed contrast is read through the script tool (implement-time reading: minimum 4.61:1, see `implement-evidence/viewer-match-day/drive.txt`); the feed list is focusable with a `:focus-visible` ring (`web/index.html:123`, `web/components/panels.css` `.feed__list:focus-visible`).

## Visual Contract Honored (only if `02c-craft.md` was present)

1. The 1280 by 800 grid — honored, unchanged at `web/layout.css:64`; the filled panels fit without page scroll (`scrollHeight` 800 at 1280 by 800, drive.txt).
2. Pitch tile 616 by 411 — honored, unchanged; the banner overlays it absolutely (`web/index.html:56`) and does not change its size.
3. Control strip 40 px under the pitch — honored; the clock moved to the header, as the plan's header step directs, and the strip keeps the effective-speed readout.
4. Marker treatment — honored, unchanged; parked markers are now skipped (`web/pitch.mjs:213`).
5. Ball trail — honored, unchanged.
6. Barlow Condensed for display, IBM Plex Sans with tabular figures — honored: score bug `web/components/panels.css:38-47`, banner `.goal-banner` font, `.tl-num` on every changing number.
7. `--tl-` OKLCH tokens, no pure black or white — honored: four motion tokens added at `web/tokens.css:63-66`; no colour literal in `panels.css`.
8. Mark drawn in code, crests from the same function — honored at `web/scoreboard.mjs:36`.
9. State words beside every state colour — honored: fatigue bands `web/lineups.mjs:7-11`, cards `web/lineups.mjs:23`, Injured and Sent off `web/lineups.mjs:60`, feed highlight words `web/match-state.mjs:23` and `web/feed.mjs:58`.
10. Skeletons, never a spinner — honored, no spinner added; the skeleton is unchanged (owner `viewer-reports-recovery`).
- Contract-check pass: deviation. No fresh-context check agent could be dispatched in this run (no agent-dispatch tool). A self-check against the ten items above, the anti-goals (no clutter over markers beyond the banner, no gradient, no nested cards, no side stripe, no bounce, no spinner, no image file), and the 4.5:1 numeric-contrast rule found no material departure. Verify or review should run the blind check.

## Deviations from Plan

1. **Benchmark record not rewritten here.** The plan's full-check step restates BENCH-P95 in `05c-benchmark.md` and records the four page targets. The browser pane was hidden, so frame readings are throttled and meaningless, and `05c-benchmark.md` carries uncommitted edits from another writer. The stream and processor-time targets were measured and are recorded below; the p95 restatement and the page targets go to verify's benchmark compare. `class: implementation-detail`.
2. **One extra test file**, `web/tests/goal-moment.test.mjs`, holds the motion tokens equal to the module's timings and tests the banner timer, replacement, and reduced motion. `class: implementation-detail`.
3. **Bench list added under each lineup.** The plan's lineup step renders the 11 starters; the design brief's content inventory lists "two columns of 11 plus bench". A bench list of shirt and name, which drops each player brought on, was added in the second commit. It adds content the brief committed to and narrows nothing. `class: implementation-detail`.
4. **Reduced motion through one attribute.** The plan put the media query and the proxy attribute in one CSS block; CSS cannot join a media query and an attribute selector in one rule, so `watchMotion` mirrors the query onto the attribute and one block serves both. `class: implementation-detail`.
5. **Version-3 fallout in tests the plan did not list:** `crates/stream/src/server.rs` and `crates/engine-cli/tests/web_cli.rs` asserted a literal 2, `crates/engine-cli/tests/stream_cli.rs` counted 3,007 frames (now 3,127), and `web/tests/decode.test.mjs` pinned the fixture version to 2. Each now reads the current version or count. `class: implementation-detail`.
6. **Statistics layout.** Two tables of five and four rows instead of one nine-row grid, so all nine rows fit under the controls without page scroll at 1280 by 800. `class: implementation-detail`.
7. **`bench.rs` also builds the simulation before the hello.** The plan named `serve` and `record`; the shared helper needs the simulation in all three. The simulation is created before timing starts, so the measurement is unchanged. `class: implementation-detail`.
8. **No research sub-agents and no contract-check agent.** No agent-dispatch tool was available. The code was re-checked directly (every file the plan names was read before editing), and the contract self-check is recorded above. `class: implementation-detail`.
9. **Two commits instead of one.** The bench list followed the smoke drive as a second commit (`4d21041`). `class: implementation-detail`.

## Anything Deferred

- The goal moment during forward play, the AC-1 screenshot, the AC-3 band change during play, and the AC-6 drive: verify's runtime evidence, with the browser pane shown.
- BENCH-P95 restated as one frame interval (1000 / `refresh_hz`) plus 2 ms, and the four page targets against `history/05c-benchmark-3.md`: verify's benchmark compare (deviation 1).
- The real `prefers-reduced-motion` media query: the operator act the plan pre-registered (turn off Windows animation effects and reload).
- The human legibility reading at 1280 by 800: the integration slice's charter run, as the slice pre-registers.

## Known Risks / Caveats

- R9 periodic message volume: a 90-minute recording is 20.46 MB (was 16.31 MB), and a full match adds 5,858 statistics and 5,858 condition messages. The stream memory reading is 7.61 MB median against the 8.55 MB tripwire, about 11 percent margin. If a later version-3 addition breaches it, the per-second cadence is the lever.
- Version 3 refuses every earlier recording by name. The one local fixture was re-recorded; any other local recording must be recorded again.
- The lineup editor slice changes the roster before kick-off under the same version; the roster built here reflects the computer manager's pre-match lineup.
- A later event-kind rename touches the `KIND` table in `web/match-state.mjs` and `STOPS_PLAY` in `web/stoppages.mjs`.

## Freshness Research

- Source: the installed `serde` usage in `crates/protocol/src/message.rs`. Relevance: the new fields. Takeaway: `#[serde(default)]` on `roster` keeps an older hello readable, and `deny_unknown_fields` still refuses unknown fields; the new tests confirm both.
- Source: the in-app browser tool schema (`resize_window` offers only `colorScheme`). Relevance: AC-6. Takeaway: no reduced-motion emulation, so the `?motion=reduce` proxy is the drive path.
- Source: clippy 1.92 (`manual_is_multiple_of`). Relevance: the per-second send. Takeaway: `u32::is_multiple_of` replaces `% == 0`.
- No dependency was added on either side.

## Evidence

- Commits: `9e684e9` (panels, protocol version 3; 32 files) and `4d21041` (bench list; 3 files). Combined `git diff --shortstat 9e684e9^ 4d21041`: 32 files changed, 2521 insertions, 118 deletions.
- `cargo test --workspace --no-fail-fast`: 278 passed, 0 failed, 4 ignored (`implement-evidence/viewer-match-day/cargo-test.txt`).
- `node --test web/tests/*.test.mjs`: 80 tests, 80 pass, 0 fail (`node-test.txt`, after the second commit).
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0 (`clippy.txt`). `cargo fmt --all -- --check`: exit 0 (`fmt.txt`).
- Stream benchmark, three drives (`bench-stream-{1,2,3}.json`): 611,209, 609,885, and 611,588 ticks per second (baseline 609,220); peak memory 7.605, 7.613, and 7.656 MB (tripwire 8.55); pauses 0, 0, 2.
- Processor-time guard, 3 matches (`bench-cpu.json`): 1,250 ms, 416.7 ms per match (gate 460.7); peak memory 6.28 MB (gate 6.82).
- Smoke drive (`drive.txt`): panels filled at tick 16475, contrast minimum 4.61:1 over 101 nodes, reduced-motion durations 1e-05s, no page scroll.

## Assumptions and Decisions

Plan assumptions A-1 to A-12 and A-15 to A-21 were applied as written. Decisions made in this run:

- D-1 Deviations 1 to 9 above, each `class: implementation-detail`.
- D-2 The slice roster entry is set to `in-progress`, not `complete`, because the runtime criteria are not yet driven. `class: implementation-detail`.
- D-3 `00-index.md` and `03-slice.md` carried uncommitted edits from another writer before this run, so they are updated on disk but not staged; only this run's own files are committed. `class: implementation-detail`.
- D-4 The earlier version-2 `fixture.smfx` was moved to the scratchpad rather than deleted, and a version-3 fixture was recorded in its place. `class: implementation-detail`.
- D-5 The implement-time drive note is stored as `drive.txt`; the workflow write guard refuses a Markdown file without frontmatter in that folder. `class: implementation-detail`.
- Intent-bearing escapes: 0. No carried intent risk exists (`00-index.md` lists every RIM as adjudicated).

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine viewer-match-day`. Five user-observable criteria need browser drives with the pane shown, and the benchmark compare owes the p95 restatement and the page targets. Compact the session first; the workflow state is in the artifact files.
- **Option B:** `/wf review football-manager-match-engine viewer-match-day`. Not recommended: the change has testable runtime behaviour.
