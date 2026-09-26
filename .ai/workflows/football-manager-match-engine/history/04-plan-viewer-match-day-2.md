---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: viewer-match-day
status: awaiting-input
stage-number: 4
created-at: "2026-09-22T22:14:50Z"
updated-at: "2026-09-23T07:30:49Z"
metric-files-to-touch: 26
metric-step-count: 16
has-blockers: true
revision-count: 2
revisions:
  - rev: 1
    at: "2026-09-23T07:03:28Z"
    trigger: manual
    because: "auto-review — 7 issues found"
    changed: "Producers shipped (commentary, tactics-and-ai, most of calibration): current state, dependencies, feed and lineup steps, and file list re-grounded; Q-1 narrowed from 'which source and when' to the wire shape only, with new options; the old re-sequence option removed as satisfied."
    snapshot: history/04-plan-viewer-match-day-0.md
  - rev: 2
    at: "2026-09-23T07:30:49Z"
    trigger: manual
    because: "auto-review — 2 issues found"
    changed: "Calibration now complete (status drift in Current State and Dependencies); new risk R8 and assumption A-15 for the planned event-contract redesign that names this slice as a migration consumer (event kinds held in one table). Q-1 unchanged and still open."
    snapshot: history/04-plan-viewer-match-day-1.md
consult-runs: []
tags: [viewer, panels, feed, statistics, goal-moment, awaiting-input]
stack-source: confirmed
augmentations:
  instrument: 04b-instrument.md
  benchmark: 05c-benchmark.md
  design-contract: 02c-craft.md
open-questions:
  - id: Q-1
    class: intent-bearing
    question: "The engine now keeps every value the panels need, but no message carries the roster, the running statistics, or player energy. What wire shape carries them?"
    options: [A-additive-same-version, B-new-messages-version-3, C-roster-from-team-files]
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-viewer-match-day.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md, 04-plan-commentary.md, 04-plan-calibration.md, 04-plan-viewer-lineup-tactics.md, 04-plan-viewer-reports-recovery.md]
  design: 02b-design.md
  contract: 02c-craft.md
  steer: steer.md
  implement: 05-implement-viewer-match-day.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine viewer-match-day"
---

# Plan: Match-Day Panels and Goal Moments

## The Plan

The first version of this plan stopped because nothing produced the data three panels need. That has changed. The tactics, commentary, and calibration work has shipped into the engine. The engine now keeps all nine statistics as running counters (`crates/engine/src/sim.rs:289-328`: shots, shots on target, expected goals, passes, completed passes, possession ticks, fouls, corners, offsides). It keeps an energy value for every player (`crates/engine/src/player.rs:137`, drained by `crates/engine/src/fatigue.rs`). It emits `injury` and `substitution` events, and every play event carries a `commentary` line (`crates/protocol/src/event.rs:16-33`, `:163-165`). The one gap left is the wire. The hello still names clubs only (`crates/protocol/src/message.rs:28-48`). The `stats` message is still sent once, at full time, with three diagnostic fields (`message.rs:53`, `crates/engine-cli/src/stream_run.rs:103-111`). No message carries a player's energy.

So Q-1 is now narrower. The question is no longer which slice produces the data, or when. The question is the wire shape that carries the roster, the running statistics, and energy to the page. That shape is still a product decision. It changes the versioned protocol (C4's data contract). Every payload refuses unknown fields (`message.rs:1-3`), so the version rule is part of the decision. It also changes the bytes that recorded match files store (`crates/stream/src/record.rs:1-3`). And it overlaps the squad-delivery question already open in the lineup and tactics plan (OQ-1). The plan stays at **awaiting-input** on this one question, with three options. Everything else is planned in full: the event reducer, the per-frame feed with commentary lines, the header, the goal moment, reduced motion, the empty state, hidden sent-off markers, and the verification seams. Eleven implementation-detail decisions are recorded under `## Assumptions`.

When Q-1 is answered, a re-run of this stage fills in steps 13 to 15 and clears the blocker. The top risk after Q-1 is timing. The server leads playback by up to 500 ticks, so an event shown when it arrives would announce a goal before the pitch draws it. Events are released by the rendered tick, never by arrival.

## Current State

- `web/` holds the same modules as when the first plan was written. The last change was product wording in the lineup placeholder (`git log -- web/`: `e79ad61`). `node --test web/tests/*.test.mjs` passes 47 of 47 tests this run. The directory form `node --test web/tests/` fails on this machine. Use the file glob.
- `web/index.html` holds the header placeholder, the "Lineups" left column, the statistics region, and the "Match feed" right column as `region--empty` placeholders. `web/main.mjs` `onMessage` handles only the stoppage index and the full-time scrubber maximum. Every socket close reads "Stream ended", including the expected close after full time (`06-verify-viewer-pitch.md:240` routes that copy here).
- `MatchEvent` (`crates/protocol/src/event.rs:98-166`) carries `tick`, `minute`, `minute.added`, `event.type` (**16 kinds**, now including `injury`, `substitution`, and `ai-decision`), `team.id`, both scores, `player.id`, `player.secondary_id` (the player coming on, on `substitution`), `card.kind`, `foul.advantage`, the change fields, and `commentary`. `docs/reference/protocol.md` documents `commentary` as present "on every event except `tactics-change`".
- **All nine statistics exist in the engine, live.** `Summary` (`sim.rs:289-328`) counts fouls, offsides, corners, shots, shots on target, expected goals, passes, completed passes, and possession ticks per team. `MatchFigures::new` (`crates/engine/src/observe/mod.rs:104-150`) derives pass accuracy and possession share from them. **None of them reaches the wire.** `stream_run.rs:103-111` sends `possession.changes`, `ball.max_speed`, and `ball.idle_ticks` once, after the last event.
- **Energy exists per player** (`player.rs:137`, `energy: f64`, starting at 1.0), drained every tick and refreshed every 50 ticks (`fatigue.rs:1-9`, `REFRESH_TICKS`). No message carries it.
- The hello is built in three places: `crates/engine-cli/src/serve.rs:35`, `record.rs:34`, and `bench.rs:149`. Player names, shirts, and positions are in `content/teams/*.json` and never reach the page. The 22 wire slots are "in roster order, home team first" (`docs/reference/protocol.md`).
- A recorded match file stores the wire bytes exactly as sent (`crates/stream/src/record.rs:1-3`). Any new message or field therefore appears in recorded files and replays.
- `PROTOCOL_VERSION` is 2 (`crates/protocol/src/lib.rs:44`).
- A sent-off player parks at `parking_spot()` beside the touchline (`crates/engine/src/pitch.rs:155-162`). `grep -rn parking web/*.mjs` returns nothing this run, so the page still draws parked markers. `04-plan-match-rules.md` R5 hands this job to this slice.
- `docs/design/realism/02-event-contract-redesign.md` (added to the repository after the first plan, not yet committed) is a planned later programme. It names this slice as a consumer to migrate (lines 320 and 346): event kinds move to provider names such as `KickOff` and `CornerKick`, the protocol version rises, and a match may carry about 1,400 events. Nothing in it is scheduled before this slice, and it does not answer Q-1.
- `06-verify-viewer-pitch.md:163` routes BENCH-P95 here: the 16.6 ms p95 frame-time budget sits below one 60 Hz frame interval and cannot be met. This slice restates it relative to `refresh_hz`.
- Slice status in `00-index.md` this run: `tactics-and-ai` complete, `commentary` complete, `calibration` complete. `viewer-lineup-tactics` depends on this slice and has its own open question (OQ-1) on how squads cross to the page.

## Simplicity Ladder

| Capability | Rung | Evidence |
|---|---|---|
| Event reducer (score and feed at tick N) | rung 4 new-code | No client-side event state exists. `web/stoppages.mjs` indexes ticks only. About 150 lines of pure code over the existing message shape |
| Per-frame feed batching | rung 2 native-platform | `requestAnimationFrame`, already driving `main.mjs`, plus one `DocumentFragment` insert per frame. No virtual-list library, because 40 to 120 rows per match is small |
| Feed line text | rung 3 reuse | the `commentary` field on every play event (`event.rs:163-165`); no page-side phrasing |
| Club crests and kit chips | rung 3 reuse | `web/mark.mjs` `drawMark()`, which `02c-craft.md` item 8 mandates for crests |
| Tabular numerals and fonts | rung 3 reuse | `web/fonts/fonts.css` and the `.tl-num` class already in `layout.css` |
| Reduced motion | rung 2 native-platform | the CSS `@media (prefers-reduced-motion: reduce)` block and `matchMedia` for the banner timer |
| Goal banner and pulse | rung 2 native-platform | CSS transitions on `opacity` and `transform`, per `02c-craft.md` Motion. No animation library |
| Parked-marker test | rung 3 reuse | a port of `is_parking_spot()` from `crates/engine/src/pitch.rs:161` |
| Statistics counters | rung 3 reuse | `Summary` (`sim.rs:289-328`) and `MatchFigures::new` (`observe/mod.rs:132`) for the two derived shares. No new counter |
| Player energy | rung 3 reuse | `Player::energy` (`player.rs:137`). No new model |
| Statistics message and roster on the wire | **pending Q-1** | See `## Blockers` |
| JavaScript test runner | rung 2 native-platform | `node:test`, 47 tests passing this run |

## Applied Learnings

- `.ai/solutions/INDEX.md` does not exist (`ls .ai/solutions` returned "No such file or directory" this run). No applicable learnings found.
- Carry-over from `04-plan-match-rules.md` R5: hide parked markers (step 12).
- Carry-over from `06-verify-viewer-pitch.md:163` BENCH-P95: restate the budget relative to `refresh_hz` (step 16).
- Carry-over from `06-verify-viewer-pitch.md:240`: name full time distinctly from a dropped stream (step 10).
- **Repeat-deferral tripwire.** `00-index.md` `runtime-evidence-deferrals` holds no entry for reduced-motion emulation or human legibility, so no wall is inherited. This slice names one new wall, reduced-motion emulation, which is classified in `## Verification Strategy`.

## Likely Files / Areas to Touch

- `web/match-state.mjs` (new): a pure reducer from events to state at a tick.
- `web/feed.mjs` (new): the per-frame batcher and the feed renderer.
- `web/scoreboard.mjs` (new): the header score bug, clock, kit chips, and speed.
- `web/lineups.mjs` (new): the two lineup columns, the fatigue bar, and the condition word.
- `web/stats.mjs` (new): the statistics panel model and renderer.
- `web/goal-moment.mjs` (new): the pulse, the banner, the highlight, and reduced motion.
- `web/components/panels.css` (new).
- `web/index.html`, `web/main.mjs`, `web/layout.css`, `web/tokens.css`, `web/pitch.mjs`, `web/tests/helpers.mjs` (modified).
- `web/tests/match-state.test.mjs`, `feed.test.mjs`, `stats.test.mjs`, `lineups.test.mjs` (new).
- **Pending Q-1:** `crates/protocol/src/message.rs`, `crates/protocol/src/lib.rs`, `crates/protocol/tests/document.rs`, `docs/reference/protocol.md`, `crates/engine-cli/src/serve.rs`, `crates/engine-cli/src/record.rs`, `crates/engine-cli/src/bench.rs`, `crates/engine-cli/src/stream_run.rs`, `crates/engine/src/sim.rs` (a read-only energy accessor only).

## Proposed Change Strategy

Build from the inside out, as the two earlier viewer and stream slices did. Pure modules come first under `node --test`, then renderers, then wiring, then the browser drive.

NFR-2 (`yields-to: C2`) governs timing. A panel never shows what the pitch has not drawn. Every event is stored by tick in `match-state`, and each animation frame asks "what is true at the rendered tick?" The first frame whose rendered tick is at or past the goal tick applies the score, the banner, the feed line, and the commentary line together. That is "within one frame of the goal tick" by construction. It also makes rewind correct: scrubbing back before a goal restores the earlier score and removes the later feed lines. Running statistics and energy, once Q-1 settles their message, are stored by tick the same way and released by the rendered tick.

The slice's own risk rule governs cadence: panels update per event and per statistics message, never per tick. One flush per frame writes only the nodes whose values changed.

NFR-6 governs every state colour. Cards carry "Yellow" or "Red", fatigue bands carry a word, injury rows carry "Injured", and highlighted feed rows carry the event word. Colour is never the only signal (`02c-craft.md` item 9, `steer.md`).

## Step-by-Step Plan

1. **Match state.** `web/match-state.mjs`: `MatchState` stores events sorted by tick and exposes `at(tick)`, which returns `{ home, away, entries, cards, injuries, substitutions, goals }` for every event with `tick <= tick`. It is incremental going forward and recomputes from the start on a backward seek (at most about 150 events). Every event-kind string that the new modules read (the feed highlight kinds, the hidden `ai-decision` kind, `goal`, `full-time`) sits in one exported table in this module (A-15). It is pure, with no DOM. Tests: kick-off returns 0 to 0 with no entries; a goal at T shows 1 to 0 at T and 0 to 0 at T-1; a goal that arrives 400 ticks before the rendered tick is not visible until the rendered tick reaches it; a seek backwards removes later entries.
2. **Feed batcher.** In `web/feed.mjs`, `FeedBatcher` takes the released entries each frame and returns one batch per frame. The renderer inserts the batch through one `DocumentFragment`. Tests with a synthetic burst: 40 events inside one simulated second at 8x all appear, in tick order, with none dropped, and no frame inserts twice.
3. **Feed renderer.** Each row shows the minute stamp (`45+2'` from `minute` and `minute.added`) and the event's `commentary` line. A `tactics-change` row, which has no commentary, shows the event kind in words ("Tactics change — Oakmere Rangers"). `ai-decision` rows are not shown in the feed; they are the computer manager's reasoning, and the resulting `tactics-change` or `substitution` row carries the visible outcome. Goal, card, substitution, and injury rows carry a highlight class plus the event word. The feed scrolls to the newest row unless the manager has scrolled up. The region carries `aria-live="polite"` for goals and cards only, so a screen reader is not flooded at 8x.
4. **Empty state.** With no released entries, the feed shows the text from Assumption A-9 and the header reads 0 to 0. This state returns after a rewind to tick 0.
5. **Header.** `web/scoreboard.mjs` fills the 56-pixel band with two crests from `drawMark()` in kit colours, team names, the score bug in `--tl-font-display` at 30 pixels or larger, and the clock and speed indicator in `--tl-font-num` with `tabular-nums`. The clock and speed move from the control strip's readout to the header, and the control strip keeps its own readout for the effective speed.
6. **Statistics model.** `web/stats.mjs` `toPanel(message)` returns nine rows in the order the slice lists: possession, shots, on target, expected goals, passes, pass accuracy, fouls, corners, offsides. Each row holds home and away values and an accessible label. Values are shown exactly as the message carries them (expected goals to two decimals, shares to one decimal, matching `MatchFigures`). **The message name and key names come from Q-1.** Test: every field in a statistics message appears in the model with an equal value.
7. **Statistics renderer.** A two-column tabular grid under the control strip, using `--tl-fg` on `--tl-panel` in the 14-pixel numeric face. It updates when a released statistics message changes and on seek. Before writing any banner motion, measure the numeric row's contrast in the drive and require at least 4.5:1, as `02c-craft.md` Colour application and the slice's top risk require.
8. **Lineups.** `web/lineups.mjs` renders two columns of rows with position, shirt, name, fatigue bar, condition word, and card marker. Energy maps to three bands: Fresh (0.70 to 1.00), Tiring (0.40 to 0.69), and Exhausted (below 0.40). Each band carries its word, and the bar uses `--tl-success`, `--tl-warning`, or `--tl-danger`. A card shows "Yellow" or "Red" beside the `--tl-card-yellow` or `--tl-danger` chip. An injured player's row reads "Injured" from the released `injury` event. A substitution swaps the row named by `player.id` for the player named by `player.secondary_id`. A sent-off player's row reads "Sent off". **The roster and energy source comes from Q-1.** Test: every band returns a word, and no band returns a colour alone.
9. **Goal moment.** `web/goal-moment.mjs` `play(goal)` runs in the frame that releases the goal. It sets the score, starts the score-bug pulse, shows the banner "GOAL — <team> <home>–<away> <minute>'", and highlights the feed row with its commentary line. The banner enters over 250 ms, holds, and leaves over 200 ms. Total visible time is 1,500 ms, set by the tokens in step 11. Only one banner shows at a time, and a second goal replaces the first. At 8x playback, the banner timer uses wall time, not match time. `matchMedia('(prefers-reduced-motion: reduce)')` drops the motion but keeps the banner: it appears and disappears without transition, and the pulse is off. The CSS rule for this sits in the same block as the `[data-motion="reduce"]` proxy attribute (see Verification Strategy).
10. **Wire the page.** In `web/main.mjs`, `onMessage` routes `event` messages to `MatchState` and statistics messages to `stats.mjs`. The existing `frame()` calls `panels.flush(renderedTick)` after `pitch.draw`, and one call updates the header, the feed, lineups, and the goal moment. Extend the read-only hook with `matchDay()`, which returns `{ renderedTick, score, feedCount, lastFeed, goalShownAtTick, bannerVisible, stats, lineupLabels }` and has no setter. Separate full time from a dropped stream: after a `full-time` event, a socket close shows "Full time. The whole match is stored and plays back." with state word "Full time". Any other close keeps "Stream ended". Emit `viewer.goal_moment` with `goal_tick`, `rendered_tick`, and `frame_delta` so the one-frame criterion has a signal as well as a screenshot.
11. **Styles and motion tokens.** `web/components/panels.css` covers the score bug, banner, feed rows, lineup rows, and statistics grid. `web/tokens.css` gains `--tl-goal-in: 250ms`, `--tl-goal-hold: 1050ms`, `--tl-goal-out: 200ms`, and `--tl-pulse: 600ms`. Transitions use `opacity` and `transform` only, with `bounce: 0`. The reduced-motion block sets them to `0.01ms`. `web/layout.css` removes the `region--empty` placeholders for the three regions this slice fills. The left column keeps its label for the lineups.
12. **Hide parked markers.** In `web/pitch.mjs`, skip any marker whose position is within 1 centimetre of a parking spot. This is a port of `is_parking_spot()`, `crates/engine/src/pitch.rs:161`; read the exact coordinates from that function at implement time. Test it in a new `pitch.test.mjs` if the function is pure.
13. **PENDING Q-1: roster on the wire.** The wire shape is decided by the product owner. The data comes from the loaded team files; the hello is built at `serve.rs:35`, `record.rs:34`, and `bench.rs:149`.
14. **PENDING Q-1: running statistics on the wire.** The message, the key names, the cadence, and the version rule are decided by the product owner. The values come from `sim.summary()` with the two shares derived as `MatchFigures::new` derives them.
15. **PENDING Q-1: energy on the wire.** The message and the cadence are decided by the product owner. The value is `Player::energy`, which the engine refreshes every 50 ticks.
16. **The full check.** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `node --test web/tests/*.test.mjs` (the file glob). Then run the browser drives in `## Test / Verification Plan`. Restate BENCH-P95 in `05c-benchmark.md` as "p95 frame time at most one frame interval (1000 / `refresh_hz`) plus 2 ms" and record the four page targets against the `history/05c-benchmark-3.md` baseline.

## Verification Strategy

The slice defines seven acceptance criteria. Five are user-observable and two are unit tests.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| **AC-1** a goal updates score, banner, feed line, and commentary line within one frame of the goal tick | `Claude_Browser` drive: screenshot at the goal tick plus `__touchline.matchDay()` and `viewer.goal_moment` (`web-2`) | Windows 11, built engine, `fixture.smfx` with a noted goal tick — **yes**. The commentary line now travels on the goal event | `matchDay()` hook (step 10); `viewer.goal_moment` signal; noted goal tick from a fixture recorded after this slice | `node --test` over `match-state` (goal visible at T, not at T-1) → pre-registered deferral |
| **AC-2** kick-off with no events shows the empty-state text and 0 to 0 | `Claude_Browser` drive paused at tick 0 plus DOM read (`web-2`) | as AC-1 — **yes** | Empty-state copy (A-9); pause before the first event | `match-state` unit test at tick 0 → deferral |
| **AC-3** a fatigue change updates the bar and the labelled condition marker | `Claude_Browser` drive plus DOM read of the label (`web-2`) | **No: no message carries energy** | **Blocked on Q-1** | the live producer once Q-1 fixes the message |
| **AC-4** the statistics panel shows every field equal to the stream values | `node --test` over `stats.mjs` (`web-1`) | Node 22 — **yes**, once the message exists | **Blocked on Q-1** (the message shape) | none beyond Q-1 |
| **AC-5** at 8x with more than 10 events per second, the feed inserts all of them, batched per frame | `node --test` over `FeedBatcher` with a synthetic burst (`web-1`) | Node 22 — **yes** | `FeedBatcher` as a pure module (step 2) | none needed |
| **AC-6** reduced motion: banner without motion, pulse disabled | Proxy: `Claude_Browser` drive with `?motion=reduce` setting `data-motion="reduce"`, which the same CSS block serves as the media query; computed `transition-duration` read through the script tool (`web-2`) | **Real media-query emulation is not available.** The `resize_window` tool offers only `colorScheme` light or dark, and Playwright is in `toolchains-absent` | The `data-motion` proxy seam sharing one rule block with the media query (step 9) | proxy → real media query when the operator turns off Windows animation effects → pre-registered deferral |
| **AC-7** at 1280 by 800 a reviewer reads the clock, score, and statistics, with focus rings visible | Proxy: `Claude_Browser` screenshot at 1280 by 800 plus computed contrast ratios for every numeric node (`web-2`); then the human check (`web-5`) | a human reviewer on the reference laptop — **environment-negotiable** | Contrast read through the script tool; `:focus-visible` screenshots | proxy → human check at `integration` → deferral |

**Constraint resolution, one line per environment dependency on a user-observable AC:**

- AC-3 (and AC-4, which is not user-observable but shares the wall): `constraint-resolution: pending Q-1` — `wall-ownership: code-owned`. The engine already keeps the values; a message in this repository dissolves the wall, so `proxy+deferral` is unavailable. `wall-cost: retire ≈ one or two messages plus three hello call sites, about 120 lines of Rust | carry = 2 AC (AC-3, AC-4) across 1 slice`. **This is the hard gate; the plan is not complete until Q-1 is answered.** Once answered, the resolution becomes `prerequisite-slice: viewer-match-day` (the producer tasks move into steps 13 to 15 of this plan).
- AC-6: `constraint-resolution: proxy+deferral: the operator turns off Windows Settings > Accessibility > Visual effects > Animation effects, reloads the page, and runs the AC-6 drive; Chromium then reports prefers-reduced-motion: reduce` — `wall-ownership: environment-negotiable`. The agent may not change system settings, and a person can perform the act on demand. The proxy is `data-motion="reduce"`, served by the same CSS rule block, so the two cannot drift apart.
- AC-7: `constraint-resolution: proxy+deferral: the product owner reads the match screen on the reference laptop at 1280 by 800 in a lit room during the integration slice's charter-scenario run` — `wall-ownership: external` (human judgement). The slice definition pre-registers this as a human check at rung web-5. The proxy is computed contrast of at least 4.5:1 on every numeric node, plus focus-ring screenshots.
- AC-1 and AC-2 need no credential, device, or external service.

**Tooling resolution.** Every tool above is inside `stack:`. `Claude_Browser` is the chosen interactive driver (shape Q29), and `node:test` needs no install. Playwright remains absent and unused.

## Test / Verification Plan

### Automated checks

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. The Rust changes are pending Q-1. With no Rust change, these runs guard only against regressions.
- `node --test web/tests/*.test.mjs`: the 47 existing tests plus `match-state.test.mjs` (AC-1 and AC-2 logic), `feed.test.mjs` (AC-5), `stats.test.mjs` (AC-4), `lineups.test.mjs` (the AC-3 label rule), and `pitch.test.mjs` (parked markers).
- Benchmark compare against `history/05c-benchmark-3.md` for the four page targets, with p95 restated (step 16).
- Instrument check: `viewer.goal_moment` in the console and in `__touchline.signals`.

### Interactive verification (human-in-the-loop)

Platform `web`; driver `Claude_Browser` (shape Q29). Evidence goes under `verify-evidence/viewer-match-day/` per the web adapter layout. Bootstrap:

```bash
cargo build --release -p engine-cli
target/release/engine-cli.exe record --seed 7 --out fixture.smfx --minutes 90
target/release/engine-cli.exe replay --fixture fixture.smfx --speed 1.0 --web web
```

| Criterion | Drive | Pass criteria |
|---|---|---|
| AC-1 | Play to the noted goal tick at 1x. Screenshot at the frame `viewer.goal_moment` fires, then read `matchDay()` | `frame_delta` ≤ 1; score, banner, and the newest feed row with its commentary line all present in the same screenshot; `goalShownAtTick` ≥ goal tick and < goal tick + ticks per frame |
| AC-2 | Load, pause before tick 1, screenshot, read `matchDay()` | `feedCount` 0, the empty-state text present, score reads 0–0 |
| AC-3 | Pending Q-1. Play until a player's band changes, then read the row | the fatigue bar width changes and the condition node's text is one of Fresh, Tiring, or Exhausted |
| AC-6 | Load `?motion=reduce`, play to the goal tick, read the computed `transition-duration` on the banner and the pulse | banner visible; durations ≤ 0.01 ms; no pulse class applied. Record that the real media query is deferred |
| AC-7 | Resize to 1280 by 800, screenshot, compute the contrast of every `.tl-num` node, tab through every control and screenshot focus | every ratio ≥ 4.5:1; a focus ring is visible on each control. The human reading is deferred to integration |

## Risks / Watchouts

- **R1 (high) Q-1 is open.** Two criteria (AC-3, AC-4) and half of the lineup panel read fields no message carries. See `## Blockers`.
- **R2 (high) Events arrive ahead of playback.** The 500-tick server lead (`po-answers.md` stream-protocol Q10) would let the feed announce a goal before the pitch draws it. Release by rendered tick (step 1), with a test that drives a goal arriving 400 ticks early.
- **R3 (medium) Panel work steals frame budget.** Flush once per frame, write changed nodes only, and never update statistics per tick. The p95 reading after this slice is compared against the viewer-pitch baseline.
- **R4 (medium) The banner timer at 8x.** Match time runs eight times faster, so a match-time timer would flash the banner for under 200 ms. The timer uses wall time (step 9).
- **R5 (medium) Rewind across a goal.** A seek back must remove the goal from the score and the feed without replaying the banner. `at(tick)` recomputes the state, and the goal moment fires only on a forward crossing during play.
- **R6 (medium) Recorded match files.** Whatever Q-1 adds appears in every recorded file (`record.rs:1-3`). Fixtures recorded before the change do not carry it; verify must record a fresh fixture.
- **R8 (medium) A later event-contract redesign.** `docs/design/realism/02-event-contract-redesign.md` renames event kinds, raises the protocol version, and may raise the event count to about 1,400 per match. Mitigation: one event-kind table in `match-state.mjs` beside `STOPS_PLAY` in `web/stoppages.mjs` (A-15), and a feed batcher that does not depend on the event count. A-2 (no virtual list) is re-checked if that programme lands.
- **R7 (low) Reduced-motion proxy drift.** The proxy attribute and the media query share one CSS block, so they cannot diverge.
- Watch: dotted JSON keys (`event.type`, `home.score`, `minute.added`, `player.secondary_id`), exactly as viewer-pitch warned.
- Watch: `node --test web/tests/` (directory form) fails on this machine. Use the file glob.

## Dependencies on Other Slices

- `viewer-pitch` (complete): the page, the socket, history, the scheduler, the mark function, the tokens.
- `stream-protocol` (complete): the event messages. The roster, statistics, and energy messages it was expected to supply do not exist (Q-1).
- `match-rules` (complete): event types, cards, added minutes, and parked markers.
- `commentary` (complete): the `commentary` line on every play event. The feed uses it directly.
- `tactics-and-ai` (complete): energy, injury, and substitution. The values exist in the engine.
- `calibration` (complete): the statistics counters and `MatchFigures` it relies on are already in the engine. A live statistics message should use the same key names as the `match-stats` record where possible, which is part of Q-1.
- `viewer-lineup-tactics` (defined; its plan is awaiting input on OQ-1, how squads cross to the page): the roster decision here and there should be one decision.

## Assumptions

Each entry is an autonomous decision, stamped `class: implementation-detail` per `_decision-classes.md`.

- **A-1** Events are released by the rendered tick, not by arrival. Why: C2 and the one-frame criterion; the stream leads playback by up to 500 ticks. `class: implementation-detail`.
- **A-2** The feed batches with one `DocumentFragment` insert per animation frame, and no virtual list is used. Why: 40 to 150 rows per match (`02b-design.md:51`); ladder rung 2. `class: implementation-detail`.
- **A-3** Seeking backwards recomputes the match state from the start, rather than keeping per-tick snapshots. Why: at most about 150 events, O(n) per seek, and no memory cost. `class: implementation-detail`.
- **A-4** The banner timer runs on wall time, not match time. Why: the 1.5-second rule is a human-perception rule. `class: implementation-detail`.
- **A-5** Only one banner shows at a time, and a later goal replaces it. The banner does not replay on rewind. Why: the least surprising behaviour, and the same rule the notice uses. `class: implementation-detail`.
- **A-6** Energy bands are Fresh 0.70 to 1.00, Tiring 0.40 to 0.69, and Exhausted below 0.40, each with its word. Why: NFR-6 requires words; the thresholds are presentational and do not change engine behaviour. `class: implementation-detail`.
- **A-7** Reduced-motion verification uses a `data-motion="reduce"` proxy that shares the media query's CSS block, and the real query is deferred to an operator act. Why: the browser tool exposes only colour-scheme emulation, and changing OS settings is forbidden to the agent. `class: implementation-detail`.
- **A-8** Parked markers are hidden by a client-side port of `is_parking_spot()`, not by a new wire bit. Why: no protocol change, and match-rules R5 assigned it here. `class: implementation-detail`.
- **A-9** The empty-state feed copy is "No events yet. The feed fills as the match plays." with the header at "0 – 0". Why: `02b-design.md:55` defines the state but gives no literal copy, and wording is implementation-detail per the decision classes. `class: implementation-detail`.
- **A-10** After a `full-time` event, a socket close reads "Full time" rather than "Stream ended". Why: `06-verify-viewer-pitch.md:240` ambiguous copy; the expected close is not a fault. `class: implementation-detail`.
- **A-11** BENCH-P95 is restated as at most one frame interval (1000 / `refresh_hz`) plus 2 ms. Why: routed here by `06-verify-viewer-pitch.md:163`; NFR-2's 60 frames per second is unchanged, and only the instrument's threshold is corrected. `class: implementation-detail`.
- **A-12** The feed uses the event's `commentary` line and falls back to the event kind in words only for `tactics-change`, and it hides `ai-decision` rows. Why: `commentary` now ships on every play event (`event.rs:163-165`, `docs/reference/protocol.md`); `ai-decision` is reasoning, and its visible outcome arrives as its own row. `class: implementation-detail`.
- **A-13** This re-run is auto-review of the existing plan, not a new plan. The prior revision is snapshotted to `history/04-plan-viewer-match-day-0.md` (with its `.yaml` and `.html.fragment`) and the change is ledgered in `revisions:`. Why: `plan.md` Step 0 item 8(d) and review-and-fix mode. `class: implementation-detail`.
- **A-14** This third run is again auto-review (`plan.md` Step 0 item 8(d)): the plan exists and no feedback text was given. No product-owner answer to Q-1 exists in `po-answers.md` or `steer.md` since the 07:03 revision, so Q-1 stays open and the plan stays at awaiting input. The review found two issues: calibration status drift (low) and the unplanned event-contract redesign risk (medium). The prior revision is snapshotted to `history/04-plan-viewer-match-day-1.md` with its `.yaml` and `.html.fragment`. Why: the review-and-fix contract. `class: implementation-detail`.
- **A-15** The new page modules keep every event-kind string they read in one exported table in `match-state.mjs`, and do not change `web/stoppages.mjs`. Why: a later programme renames event kinds; one table limits that migration to two known places without widening this slice. `class: implementation-detail`.
- The master `04-plan.md` is not updated in this run. Its `slices-planned` counts complete plans, and this plan is still awaiting input. The update lands when Q-1 is answered and this plan completes. `class: implementation-detail`.
- Consult: triggers `appetite-medium-or-larger` and `unknowns-present` hold. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`). Not fired. `class: implementation-detail`.

## Blockers

**Q-1 (intent-bearing — the versioned wire contract and recorded match files). The engine now keeps every value the panels need, but no message carries the roster, the running statistics, or player energy. What wire shape carries them?**

Why this is not decided autonomously: each answer changes the documented, versioned protocol (C4's data contract) and the bytes every recorded match file stores. Every payload refuses unknown fields (`message.rs:1-3`), so whether the version rises is part of the answer. The roster part is the same decision as `viewer-lineup-tactics` OQ-1 (squads in the opening message or elsewhere). Criterion 2 of `_decision-classes.md` (a contract surface) applies, and every earlier protocol addition was asked of the product owner.

What changed since the first version: the old option B ("wait for the tactics, commentary, and calibration work") is satisfied, because that work has shipped its producers. The old option A no longer needs new counters or dashes for expected goals and fatigue. The old option C (a proposed contract with a synthetic fixture) is no longer needed, because a live producer exists.

- **Option A (recommended): additive fields and one periodic message, same version rule as the earlier additive extras.** Each hello team gains a `roster` list (player id, name, shirt, position, in wire-slot order) from the team files. The `stats` message keeps its name and full-time send, gains the nine panel fields per team (key names aligned with the `match-stats` record: `stats.shots`, `stats.shots_on_target`, `stats.xg`, `stats.passes`, `stats.pass_accuracy_pct`, `stats.possession_pct`, and fouls, corners, offsides), and is also sent once per simulated second. A new `condition` message carries the 22 energy values in wire-slot order at the same cadence. Whether `PROTOCOL_VERSION` stays 2 or rises to 3 follows the rule at `crates/protocol/src/lib.rs:21-28`. *Consequence:* about 120 lines of Rust across 8 files. Viewers and recorded files from before the change do not carry the new data. The lineup and tactics plan's OQ-1 gets its squad source from the same hello.
- **Option B: new message types, protocol version 3.** `roster` (sent once after the hello), `live-stats`, and `condition` are three new messages. The full-time `stats` message is unchanged. *Consequence:* the cleanest separation, and the version rises, so a viewer built for version 2 refuses the stream by name.
- **Option C: roster from the team files, only numbers on the wire.** The page server serves `content/teams/<team.id>.json` and the page reads names and positions from it. Only statistics and energy go on the wire, as in option A. *Consequence:* the smallest protocol change, but the page depends on the content folder layout, and a substitution or a lineup chosen before kick-off must still reach the page by event.

Recorded in `po-answers.md` as awaiting input. On an answer, re-run `/wf plan football-manager-match-engine viewer-match-day` with the answer as feedback. That run fills steps 13 to 15 and the AC-3 and AC-4 rows and sets `status: complete`.

## Freshness Research

- Source: MDN `prefers-reduced-motion` and `Window.matchMedia`. Why it matters: AC-6. Takeaway: the media query is the sole signal, and the page reads it at load and on `change`. Chromium derives it from the OS animation setting on Windows, which is the deferral's clearing act.
- Source: MDN `DocumentFragment` and `requestAnimationFrame`. Why it matters: AC-5 and R3. Takeaway: one fragment append per frame is one layout invalidation, and inserting inside the existing `rAF` callback avoids a second callback.
- Source: MDN `aria-live`. Why it matters: the feed at 8x. Takeaway: `polite` regions queue announcements, so a live region on every row floods a screen reader. Only goals and cards are announced.
- Source: the `Claude_Browser` `resize_window` tool schema in this session. Why it matters: AC-6. Takeaway: it offers `colorScheme` light or dark and no reduced-motion emulation, which is the wall recorded above.
- No dependency is added. The page stays plain HTML, CSS, and ES modules, with `node:test` already present.

## Recommended Next Stage

- **Awaiting input (this plan):** answer Q-1, ideally together with `viewer-lineup-tactics` OQ-1, then re-run `/wf plan football-manager-match-engine viewer-match-day` with the answer.
- **Option C (revisit slice):** `/wf slice football-manager-match-engine` if the product owner prefers to move the wire additions into a separate protocol slice that both viewer slices depend on.
