---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: viewer-match-day
status: awaiting-input
stage-number: 4
created-at: "2026-09-22T22:14:50Z"
updated-at: "2026-09-22T22:14:50Z"
metric-files-to-touch: 24
metric-step-count: 16
has-blockers: true
revision-count: 0
revisions: []
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
    question: "The stream carries no player roster, no fatigue or condition, and no live statistics. Which source feeds the lineup and statistics panels?"
    options: [A-scope-wire-here, B-resequence-after-producers, C-viewer-only-against-proposed-contract]
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-viewer-match-day.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md]
  design: 02b-design.md
  contract: 02c-craft.md
  steer: steer.md
  implement: 05-implement-viewer-match-day.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine viewer-match-day"
---

# Plan: Match-Day Panels and Goal Moments

## The Plan

The viewer slice handed over a page that draws 22 markers and a ball at the shipping 1280 by 800 layout, with the header, the right column, and the statistics region built and left empty. The match-rules slice added nine event types, cards, and added time to the stream. This slice fills those regions. Research found the load-bearing fact before any layout question: the stream carries **no player roster, no fatigue or condition field, and no live statistics**. The `stats` message is "Sent once, at full time" (`docs/reference/protocol.md:116`) and carries `possession.changes`, `ball.max_speed`, and `ball.idle_ticks` (`crates/protocol/src/message.rs:51-66`). None of these is one of the nine panel fields. The hello names each club and nothing about its players (`message.rs:11-23`). The engine has no fatigue model (`crates/engine/src/data/tuning.rs:100`, "The engine does not read them yet") and no expected-goals model.

Three of the seven acceptance criteria read fields that no message carries: the fatigue bar, the statistics panel, and the lineup names under the goal moment. Deciding the wire shape for these fields is a change to the documented, versioned protocol. The protocol is the engine's headless data contract (C4), and every earlier protocol addition was asked of the product owner (`po-answers.md`, viewer-pitch Round 1 Q3, kit colours). This plan therefore stops at **awaiting-input** on one question, Q-1, with three options recorded below. Everything that does not depend on Q-1 is planned in full: the event reducer, the per-frame feed batcher, the header, the goal moment, reduced motion, the empty state, and the verification seams. Eleven autonomous implementation-detail decisions are recorded under `## Assumptions`.

When Q-1 is answered, a re-run of this stage fills in steps 13 to 15 and clears the blocker. The top risk after Q-1 is timing. The server leads playback by up to 500 ticks, so an event shown when it arrives would announce a goal before the pitch draws it. Events are released by the rendered tick, never by arrival.

## Current State

- `web/` holds 16 modules and pages, and `node --test web/tests/*.test.mjs` passes 46 of 46 tests this run. The directory form `node --test web/tests/` fails on this machine with "test failed" at `web\tests:1:1`. The file glob is the working invocation, and the viewer-pitch plan's command must not be copied.
- `web/index.html:17-23` holds a header with the mark, a title, a team line, and the engine version. The left column (`:26-29`, labelled "Lineups"), the statistics region (`:96-99`), and the right column "Match feed" (`:102-105`) are `region--empty` placeholders.
- `web/main.mjs:122-131` `onMessage` handles only the stoppage index and the full-time scrubber maximum. No event reaches any panel. `main.mjs:262-267` names every socket close "Stream ended", including the expected close after full time. `06-verify-viewer-pitch.md:240` routes that ambiguous copy to this slice.
- `MatchEvent` (`crates/protocol/src/event.rs:91-142`) carries `tick`, `minute`, `minute.added`, `event.type` (13 kinds: kick-off, goal, half-time, full-time, tactics-change, offside, foul, card, throw-in, corner, goal-kick, free-kick, penalty), `team.id`, both scores, `player.id`, `player.secondary_id`, `card.kind`, and `foul.advantage`. It has **no substitution or injury event type**. Those belong to `tactics-and-ai`, which is `defined`, not built.
- **Fouls, corners, and offsides can be counted from events. Shots, shots on target, passes, pass accuracy, possession share, and expected goals cannot.** `crates/engine/src/decision.rs:17,139` produces `Kick::Shot`, but no counter reaches the wire. `crates/engine/src/sim.rs:165` counts `possession_changes`, which is not a share.
- Player names, shirts, and positions exist in `content/teams/*.json` (`players[].name`, `shirt`, `position`) and never reach the page. The 22 wire slots are "in roster order, home team first" (`docs/reference/protocol.md:81`).
- A sent-off player parks at `parking_spot()` beside the touchline (`crates/engine/src/pitch.rs:155-162`). `04-plan-match-rules.md` R5 hands this slice the job of hiding parked markers.
- `commentary` is `defined`, not built. The slice definition allows the feed to show the event kind until it ships.
- `06-verify-viewer-pitch.md:163` routes BENCH-P95 here: the 16.6 ms p95 frame-time budget sits below one 60 Hz frame interval and cannot be met. This slice restates it relative to `refresh_hz`.
- `05c-benchmark.md:106`: this slice takes the page baselines from `history/05c-benchmark-3.md`.
- `02c-craft.md` pins cards and fatigue to item 9, state words beside every state colour, and club crests to item 8, the mark function. Its motion table (`02c-craft.md` section 4, Motion) has **no row for the goal banner or the score-bug pulse**. `02b-design.md:81` gives the only rule: the banner "enters from the pitch edge and leaves within 1.5 seconds". This plan adds those rows in step 11 within the committed direction.
- `02b-design.md:55` describes the empty state as "kick-off with no events and 0 to 0 in the header" and gives no literal copy. The slice's criterion refers to "the empty-state text from the design brief", which does not exist as text. Assumption A-9 supplies it.
- The slice's `verify:` stubs name `fixture.bin`. The replayer takes `.smfx` fixtures (`crates/engine-cli/src/cli.rs:146`). The plan uses `fixture.smfx`.

## Simplicity Ladder

| Capability | Rung | Evidence |
|---|---|---|
| Event reducer (score and feed at tick N) | rung 4 new-code | No client-side event state exists. `web/stoppages.mjs` indexes ticks only. It is about 150 lines of pure code over the existing message shape |
| Per-frame feed batching | rung 2 native-platform | `requestAnimationFrame`, already driving `main.mjs:133`, plus one `DocumentFragment` insert per frame. No virtual-list library, because 40 to 120 rows per match is small |
| Club crests and kit chips | rung 3 reuse | `web/mark.mjs` `drawMark()`, which `02c-craft.md` item 8 mandates for crests |
| Tabular numerals and fonts | rung 3 reuse | `web/fonts/fonts.css` and the `.tl-num` class already in `layout.css` |
| Reduced motion | rung 2 native-platform | the CSS `@media (prefers-reduced-motion: reduce)` block and `matchMedia` for the banner timer |
| Goal banner and pulse | rung 2 native-platform | CSS transitions on `opacity` and `transform`, per `02c-craft.md` Motion. No animation library |
| Parked-marker test | rung 3 reuse | a port of `is_parking_spot()` from `crates/engine/src/pitch.rs:161` |
| Statistics panel model | rung 4 new-code, **pending Q-1** | The message it maps from does not exist yet |
| Roster and fatigue source | **pending Q-1** | See `## Blockers` |
| JavaScript test runner | rung 2 native-platform | `node:test`, 46 tests passing this run |

## Applied Learnings

- `.ai/solutions/INDEX.md` does not exist (`ls .ai/solutions` returned "No such file or directory" this run). No applicable learnings found.
- Carry-over from `04-plan-match-rules.md` R5: hide parked markers (step 12).
- Carry-over from `06-verify-viewer-pitch.md:163` BENCH-P95: restate the budget relative to `refresh_hz` (step 16).
- Carry-over from `06-verify-viewer-pitch.md:240`: name full time distinctly from a dropped stream (step 10).
- **Repeat-deferral tripwire.** `00-index.md` `runtime-evidence-deferrals` is `[]`, so no wall is inherited. This slice names one new wall, reduced-motion emulation, which is classified in `## Verification Strategy`.

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
- **Pending Q-1:** `crates/protocol/src/message.rs`, `crates/protocol/src/lib.rs`, `crates/protocol/tests/document.rs`, `docs/reference/protocol.md`, `crates/stream/src/server.rs`, `crates/engine-cli/src/stream_run.rs`, `crates/engine/src/sim.rs`.

## Proposed Change Strategy

Build from the inside out, as the two earlier viewer and stream slices did. Pure modules come first under `node --test`, then renderers, then wiring, then the browser drive.

NFR-2 (`yields-to: C2`) governs timing. A panel never shows what the pitch has not drawn. Every event is stored by tick in `match-state`, and each animation frame asks "what is true at the rendered tick?" The first frame whose rendered tick is at or past the goal tick applies the score, the banner, the feed line, and the commentary line together. That is "within one frame of the goal tick" by construction. It also makes rewind correct for free: scrubbing back before a goal restores the earlier score and removes the later feed lines.

The slice's own risk rule governs cadence: panels update per event and per statistics message, never per tick. One flush per frame writes only the nodes whose values changed.

NFR-6 governs every state colour. Cards carry "Yellow" or "Red", fatigue bands carry a word, and highlighted feed rows carry the event word. Colour is never the only signal (`02c-craft.md` item 9).

## Step-by-Step Plan

1. **Match state.** `web/match-state.mjs`: `MatchState` stores events sorted by tick and exposes `at(tick)`, which returns `{ home, away, entries, cards, goals }` for every event with `tick <= tick`. It is incremental going forward and recomputes from the start on a backward seek (at most about 120 events). It is pure, with no DOM. Tests: kick-off returns 0 to 0 with no entries; a goal at T shows 1 to 0 at T and 0 to 0 at T-1; a goal that arrives 400 ticks before the rendered tick is not visible until the rendered tick reaches it; a seek backwards removes later entries.
2. **Feed batcher.** In `web/feed.mjs`, `FeedBatcher` takes the released entries each frame and returns one batch per frame. The renderer inserts the batch through one `DocumentFragment`. Tests with a synthetic burst: 40 events inside one simulated second at 8x all appear, in tick order, with none dropped, and no frame inserts twice.
3. **Feed renderer.** Each row shows the minute stamp (`45+2'` from `minute` and `minute.added`) and a line. The line is the commentary text when a commentary field exists, and until then the event kind in words ("Goal — Oakmere Rangers", "Corner", "Yellow card"). Goal, card, substitution, and injury rows carry a highlight class plus the event word. The feed scrolls to the newest row unless the manager has scrolled up. The region carries `aria-live="polite"` for goals and cards only, so a screen reader is not flooded at 8x.
4. **Empty state.** With no released entries, the feed shows the text from Assumption A-9 and the header reads 0 to 0. This state returns after a rewind to tick 0.
5. **Header.** `web/scoreboard.mjs` fills the 56-pixel band with two crests from `drawMark()` in kit colours, team names, the score bug in `--tl-font-display` at 30 pixels or larger, and the clock and speed indicator in `--tl-font-num` with `tabular-nums`. The clock and speed move from the control strip's readout to the header, and the control strip keeps its own readout for the effective speed.
6. **Statistics model.** `web/stats.mjs` `toPanel(message)` returns nine rows in the order the slice lists: possession, shots, on target, expected goals, passes, pass accuracy, fouls, corners, offsides. Each row holds home and away values and an accessible label. **The field names and the message come from Q-1.** Test: every field in a statistics message appears in the model with an equal value.
7. **Statistics renderer.** A two-column tabular grid under the control strip, using `--tl-fg` on `--tl-panel` in the 14-pixel numeric face. It updates on message arrival and on seek. Before writing any banner motion, measure the numeric row's contrast in the drive and require at least 4.5:1, as `02c-craft.md` Colour application and the slice's top risk require.
8. **Lineups.** `web/lineups.mjs` renders two columns of rows with position, shirt, name, fatigue bar, condition word, and card marker. Fatigue bands are Fresh (70 to 100), Tiring (40 to 69), and Exhausted (0 to 39). Each band carries its word, and the bar uses `--tl-success`, `--tl-warning`, or `--tl-danger`. A card shows "Yellow" or "Red" beside the `--tl-card-yellow` or `--tl-danger` chip. A sent-off player's row reads "Sent off". **The roster and fatigue source comes from Q-1.** Test: every band returns a word, and no band returns a colour alone.
9. **Goal moment.** `web/goal-moment.mjs` `play(goal)` runs in the frame that releases the goal. It sets the score, starts the score-bug pulse, shows the banner "GOAL — <team> <home>–<away> <minute>'", and highlights the feed row. The banner enters over 250 ms, holds, and leaves over 200 ms. Total visible time is 1,500 ms, set by the tokens in step 11. Only one banner shows at a time, and a second goal replaces the first. At 8x playback, the banner timer uses wall time, not match time. `matchMedia('(prefers-reduced-motion: reduce)')` drops the motion but keeps the banner: it appears and disappears without transition, and the pulse is off. The CSS rule for this sits in the same block as the `[data-motion="reduce"]` proxy attribute (see Verification Strategy).
10. **Wire the page.** In `web/main.mjs`, `onMessage` routes `event` messages to `MatchState` and statistics messages to `stats.mjs`. The existing `frame()` calls `panels.flush(renderedTick)` after `pitch.draw`, and one call updates the header, the feed, lineups, and the goal moment. Extend the read-only hook with `matchDay()`, which returns `{ renderedTick, score, feedCount, lastFeed, goalShownAtTick, bannerVisible, stats, lineupLabels }` and has no setter. Separate full time from a dropped stream: after a `full-time` event, a socket close shows "Full time. The whole match is stored and plays back." with state word "Full time". Any other close keeps "Stream ended". Emit `viewer.goal_moment` with `goal_tick`, `rendered_tick`, and `frame_delta` so the one-frame criterion has a signal as well as a screenshot.
11. **Styles and motion tokens.** `web/components/panels.css` covers the score bug, banner, feed rows, lineup rows, and statistics grid. `web/tokens.css` gains `--tl-goal-in: 250ms`, `--tl-goal-hold: 1050ms`, `--tl-goal-out: 200ms`, and `--tl-pulse: 600ms`. Transitions use `opacity` and `transform` only, with `bounce: 0`. The reduced-motion block sets them to `0.01ms`. `web/layout.css` removes the `region--empty` placeholders for the three regions this slice fills. The left column keeps its label for the lineups.
12. **Hide parked markers.** In `web/pitch.mjs`, skip any marker whose position is within 1 centimetre of a parking spot (y = -(34 + 3) m, |x| from 10 to 20 m). This is a port of `is_parking_spot()`, `crates/engine/src/pitch.rs:161`. Test it in the existing pitch-geometry tests or in a new `pitch.test.mjs` if the function is pure.
13. **PENDING Q-1: roster on the wire.** The wire shape is decided by the product owner.
14. **PENDING Q-1: live statistics on the wire.** The cadence, the fields, and the counters are decided by the product owner.
15. **PENDING Q-1: fatigue and condition on the wire.** The source and the cadence are decided by the product owner.
16. **The full check.** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `node --test web/tests/*.test.mjs` (the file glob; the directory form fails on this machine). Then run the browser drives in `## Test / Verification Plan`. Restate BENCH-P95 in `05c-benchmark.md` as "p95 frame time at most one frame interval (1000 / `refresh_hz`) plus 2 ms" and record the four page targets against the `history/05c-benchmark-3.md` baseline.

## Verification Strategy

The slice defines seven acceptance criteria. Five are user-observable and two are unit tests.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| **AC-1** a goal updates score, banner, feed line, and commentary line within one frame of the goal tick | `Claude_Browser` drive: screenshot at the goal tick plus `__touchline.matchDay()` and `viewer.goal_moment` (`web-2`) | Windows 11, built engine, `fixture.smfx` with a noted goal tick — **yes** for score, banner, and feed. The commentary line shows the event kind until `commentary` ships | `matchDay()` hook (step 10); `viewer.goal_moment` signal; noted goal tick from the re-recorded fixture | `node --test` over `match-state` (goal visible at T, not at T-1) → pre-registered deferral |
| **AC-2** kick-off with no events shows the empty-state text and 0 to 0 | `Claude_Browser` drive paused at tick 0 plus DOM read (`web-2`) | as AC-1 — **yes** | Empty-state copy (A-9); pause before the first event | `match-state` unit test at tick 0 → deferral |
| **AC-3** fatigue change updates the bar and the labelled condition marker | `Claude_Browser` drive plus DOM read of the label (`web-2`) | **No: no message carries fatigue** | **Blocked on Q-1** | Under option C, a synthetic fixture carrying the proposed field. Under options A and B, the live producer |
| **AC-4** the statistics panel shows every field equal to the stream values | `node --test` over `stats.mjs` (`web-1`) | Node 22 — **yes**, once the message exists | **Blocked on Q-1** (the message shape) | none beyond Q-1 |
| **AC-5** at 8x with more than 10 events per second, the feed inserts all of them, batched per frame | `node --test` over `FeedBatcher` with a synthetic burst (`web-1`) | Node 22 — **yes** | `FeedBatcher` as a pure module (step 2) | none needed |
| **AC-6** reduced motion: banner without motion, pulse disabled | Proxy: `Claude_Browser` drive with `?motion=reduce` setting `data-motion="reduce"`, which the same CSS block serves as the media query; computed `transition-duration` read through the script tool (`web-2`) | **Real media-query emulation is not available.** The `resize_window` tool this run offers only `colorScheme` light or dark, and Playwright is in `toolchains-absent` | The `data-motion` proxy seam sharing one rule block with the media query (step 9) | proxy → real media query when the operator turns off Windows animation effects → pre-registered deferral |
| **AC-7** at 1280 by 800 a reviewer reads the clock, score, and statistics, with focus rings visible | Proxy: `Claude_Browser` screenshot at 1280 by 800 plus computed contrast ratios for every numeric node (`web-2`); then the human check (`web-5`) | a human reviewer on the reference laptop — **environment-negotiable** | Contrast read through the script tool; `:focus-visible` screenshots | proxy → human check at `integration` → deferral |

**Constraint resolution, one line per environment dependency on a user-observable AC:**

- AC-3: `constraint-resolution: pending Q-1` — `wall-ownership: code-owned`. A change to code in this repository dissolves the wall, because the protocol and the engine are both in this workspace. So `proxy+deferral` is unavailable. The wall is retired by scoping the producer (options A or B) or by a synthetic fixture under a contract the product owner approves (option C). `wall-cost: retire ≈ one message plus counters, about 150 lines of Rust (A), or zero here with a dependency on three slices (B) | carry = 2 AC (AC-3, AC-4) across 1 slice`. **This is the hard gate; the plan is not complete until Q-1 is answered.**
- AC-6: `constraint-resolution: proxy+deferral: the operator turns off Windows Settings > Accessibility > Visual effects > Animation effects, reloads the page, and runs the AC-6 drive; Chromium then reports prefers-reduced-motion: reduce` — `wall-ownership: environment-negotiable`. The agent is forbidden to change system settings, and a person can perform the act on demand. The proxy is `data-motion="reduce"`, served by the same CSS rule block, so the two cannot drift apart.
- AC-7: `constraint-resolution: proxy+deferral: the product owner reads the match screen on the reference laptop at 1280 by 800 in a lit room during the integration slice's charter-scenario run` — `wall-ownership: external` (human judgement). The slice definition pre-registers this as a human check at rung web-5. The proxy is computed contrast of at least 4.5:1 on every numeric node, plus focus-ring screenshots.
- AC-1 and AC-2 need no credential, device, or external service.

**Tooling resolution.** Every tool above is inside `stack:`. `Claude_Browser` is the chosen interactive driver (Q29), and `node:test` needs no install, as in viewer-pitch. Playwright remains absent and unused.

## Test / Verification Plan

### Automated checks

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. The Rust changes are pending Q-1. With no Rust change, these runs guard only against regressions.
- `node --test web/tests/*.test.mjs`: the 46 existing tests plus `match-state.test.mjs` (AC-1 and AC-2 logic), `feed.test.mjs` (AC-5), `stats.test.mjs` (AC-4), and `lineups.test.mjs` (the AC-3 label rule).
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
| AC-1 | Play to the noted goal tick at 1x. Screenshot at the frame `viewer.goal_moment` fires, then read `matchDay()` | `frame_delta` ≤ 1; score, banner, and newest feed row all present in the same screenshot; `goalShownAtTick` ≥ goal tick and < goal tick + ticks per frame |
| AC-2 | Load, pause before tick 1, screenshot, read `matchDay()` | `feedCount` 0, the empty-state text present, score reads 0–0 |
| AC-3 | Pending Q-1 | the fatigue bar width changes and the condition node's text is one of Fresh, Tiring, or Exhausted |
| AC-6 | Load `?motion=reduce`, play to the goal tick, read the computed `transition-duration` on the banner and the pulse | banner visible; durations ≤ 0.01 ms; no pulse class applied. Record that the real media query is deferred |
| AC-7 | Resize to 1280 by 800, screenshot, compute the contrast of every `.tl-num` node, tab through every control and screenshot focus | every ratio ≥ 4.5:1; a focus ring is visible on each control. The human reading is deferred to integration |

## Risks / Watchouts

- **R1 (high) Q-1 is open.** Three criteria read fields no message carries. See `## Blockers`.
- **R2 (high) Events arrive ahead of playback.** The 500-tick server lead (`po-answers.md` stream-protocol Q10) would let the feed announce a goal before the pitch draws it. Release by rendered tick (step 1), with a test that drives a goal arriving 400 ticks early.
- **R3 (medium) Panel work steals frame budget.** Flush once per frame, write changed nodes only, and never update statistics per tick. The p95 reading after this slice is compared against the viewer-pitch baseline.
- **R4 (medium) The banner timer at 8x.** Match time runs eight times faster, so a match-time timer would flash the banner for under 200 ms. The timer uses wall time (step 9).
- **R5 (medium) Rewind across a goal.** A seek back must remove the goal from the score and the feed without replaying the banner. `at(tick)` recomputes the state, and the goal moment fires only on a forward crossing during play.
- **R6 (low) Reduced-motion proxy drift.** The proxy attribute and the media query share one CSS block, so they cannot diverge.
- Watch: dotted JSON keys (`event.type`, `home.score`, `minute.added`), exactly as viewer-pitch warned.
- Watch: `node --test web/tests/` (directory form) fails on this machine. Use the file glob.

## Dependencies on Other Slices

- `viewer-pitch` (complete): the page, the socket, history, the scheduler, the mark function, the tokens.
- `stream-protocol` (complete): the event messages. The roster, statistics, and fatigue messages it was expected to supply do not exist (Q-1).
- `match-rules` (complete): nine event types, cards, added minutes, and parked markers.
- `commentary` (defined, not built): the feed shows the event kind until it ships. The slice definition allows this.
- `tactics-and-ai` (defined, not built): the only slice that produces fatigue, substitution, and injury. **This slice does not declare it as a dependency**, which is part of Q-1.
- `calibration` (defined, not built): the per-match statistics record whose schema waits on U-2. A live statistics message defined here would fix part of that schema first, which is also part of Q-1.
- `04-plan.md` `implementation-order` already places `tactics-and-ai`, `commentary`, and `calibration` before `viewer-match-day`.

## Assumptions

Each entry is an autonomous decision, stamped `class: implementation-detail` per `_decision-classes.md`.

- **A-1** Events are released by the rendered tick, not by arrival. Why: C2 and the one-frame criterion; the stream leads playback by up to 500 ticks. `class: implementation-detail`.
- **A-2** The feed batches with one `DocumentFragment` insert per animation frame, and no virtual list is used. Why: 40 to 120 rows per match (`02b-design.md:51`); ladder rung 2. `class: implementation-detail`.
- **A-3** Seeking backwards recomputes the match state from the start, rather than keeping per-tick snapshots. Why: at most about 120 events, O(n) per seek, and no memory cost. `class: implementation-detail`.
- **A-4** The banner timer runs on wall time, not match time. Why: the 1.5-second rule is a human-perception rule. `class: implementation-detail`.
- **A-5** Only one banner shows at a time, and a later goal replaces it. The banner does not replay on rewind. Why: the least surprising behaviour, and the same rule the notice uses. `class: implementation-detail`.
- **A-6** Fatigue bands are Fresh 70 to 100, Tiring 40 to 69, and Exhausted 0 to 39, each with its word. Why: NFR-6 requires words; the thresholds are presentational and are revisited when `tactics-and-ai` defines the curve. `class: implementation-detail`.
- **A-7** Reduced-motion verification uses a `data-motion="reduce"` proxy that shares the media query's CSS block, and the real query is deferred to an operator act. Why: the browser tool exposes only colour-scheme emulation, and changing OS settings is forbidden to the agent. `class: implementation-detail`.
- **A-8** Parked markers are hidden by a client-side port of `is_parking_spot()`, not by a new wire bit. Why: no protocol change, and match-rules R5 assigned it here. `class: implementation-detail`.
- **A-9** The empty-state feed copy is "No events yet. The feed fills as the match plays." with the header at "0 – 0". Why: `02b-design.md:55` defines the state but gives no literal copy, and wording is implementation-detail per the decision classes. `class: implementation-detail`.
- **A-10** After a `full-time` event, a socket close reads "Full time" rather than "Stream ended". Why: `06-verify-viewer-pitch.md:240` ambiguous copy; the expected close is not a fault. `class: implementation-detail`.
- **A-11** BENCH-P95 is restated as at most one frame interval (1000 / `refresh_hz`) plus 2 ms. Why: routed here by `06-verify-viewer-pitch.md:163`; NFR-2's 60 frames per second is unchanged, and only the instrument's threshold is corrected. `class: implementation-detail`.
- The master `04-plan.md` is not updated in this run. Its `slices-planned` counts complete plans, and this plan is awaiting input. The update lands when Q-1 is answered and this plan completes. `class: implementation-detail`.
- Consult: triggers `appetite-medium-or-larger` and `unknowns-present` hold. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`). Not fired.

## Blockers

**Q-1 (intent-bearing — contract and sequencing). The stream carries no player roster, no fatigue or condition, and no live statistics. Which source feeds the lineup and statistics panels?**

Why this is not decided autonomously: each answer changes the documented, versioned protocol (C4's data contract), or the slice's dependency list, or which panels ship populated. Criterion 2 (a PO directive is narrowed) and criterion 5 (a committed capability is stubbed) of `_decision-classes.md` both apply, and every earlier protocol addition was asked of the product owner.

- **Option A: scope the wire here.** Protocol version 2 stays. The additions are: a `roster` on each hello team (player id, name, shirt, position, from the team files); a live `stats` message once per simulated second and at every event, carrying the nine panel fields per team; and a per-player `condition` value. The engine gains counters for shots, shots on target, passes, completed passes, and possession share now. Expected goals and fatigue ride as `null` until `tactics-and-ai` and `calibration` model them, and the panel shows "—" with the words "not modelled yet". *Consequence:* the screen ships this slice, but two fields show a dash in live play. Part of the statistics schema is fixed before U-2's observability agreement. About 7 Rust files and 150 lines are added to this slice.
- **Option B: re-sequence (recommended).** Plan and build this slice after `tactics-and-ai`, `commentary`, and `calibration`, the order `04-plan.md` already records. Those slices define fatigue, substitution, injury, commentary, and the statistics record, and this slice consumes them. Add `tactics-and-ai` and `calibration` to `depends-on`. *Consequence:* the match-day screen waits for three engine slices, but no field ships as a dash and no schema is defined twice.
- **Option C: viewer-only against a proposed contract.** This plan writes the three message shapes into `docs/reference/protocol.md` as "proposed". A synthetic fixture carries them, and the panels are verified against that fixture. The engine producers land later. *Consequence:* the page is verified against data no engine yet emits, and a later producer slice must match a contract the viewer defined.

Recorded in `po-answers.md` as awaiting input. On an answer, re-run `/wf plan football-manager-match-engine viewer-match-day` with the answer as feedback. That run fills steps 13 to 15 and the AC-3 and AC-4 rows and sets `status: complete`.

## Freshness Research

- Source: MDN `prefers-reduced-motion` and `Window.matchMedia`. Why it matters: AC-6. Takeaway: the media query is the sole signal, and the page reads it at load and on `change`. Chromium derives it from the OS animation setting on Windows, which is the deferral's clearing act.
- Source: MDN `DocumentFragment` and `requestAnimationFrame`. Why it matters: AC-5 and R3. Takeaway: one fragment append per frame is one layout invalidation, and inserting inside the existing `rAF` callback avoids a second callback.
- Source: MDN `aria-live`. Why it matters: the feed at 8x. Takeaway: `polite` regions queue announcements, so a live region on every row floods a screen reader. Only goals and cards are announced.
- Source: this run's `Claude_Browser` `resize_window` tool schema. Why it matters: AC-6. Takeaway: it offers `colorScheme` light or dark and no reduced-motion emulation, which is the wall recorded above.
- No dependency is added. The page stays plain HTML, CSS, and ES modules, with `node:test` at Node v22.15.0, already present.

## Recommended Next Stage

- **Awaiting input (this plan):** answer Q-1, then re-run `/wf plan football-manager-match-engine viewer-match-day` with the answer.
- **Option B path:** if the product owner picks re-sequencing, plan `tactics-and-ai` next (`/wf plan football-manager-match-engine tactics-and-ai`) and return to this slice after `commentary` and `calibration`.
- **Option C (revisit slice):** `/wf slice football-manager-match-engine`. Planning showed that this slice's `depends-on` omits `tactics-and-ai` and `calibration`, the only producers of fatigue and statistics.
