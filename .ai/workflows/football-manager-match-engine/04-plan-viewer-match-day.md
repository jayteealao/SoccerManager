---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: viewer-match-day
status: complete
stage-number: 4
created-at: "2026-09-22T22:14:50Z"
updated-at: "2026-09-23T08:43:22Z"
metric-files-to-touch: 28
metric-step-count: 16
has-blockers: false
revision-count: 3
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
  - rev: 3
    at: "2026-09-23T08:43:22Z"
    trigger: answers-returned
    because: "product owner answered Q-1 with option A (roster in the hello, stats every simulated second with nine fields, a condition message, version 3)"
    changed: "Steps 13 to 15 written in full; AC-3 and AC-4 rows resolved in-slice; blocker cleared; file list re-grounded (no engine sim.rs change, two stream test files added); new risks for the version-3 fixture refusal, periodic message volume, and the lineup editor's roster; status complete."
    snapshot: history/04-plan-viewer-match-day-2.md
consult-runs: []
tags: [viewer, panels, feed, statistics, goal-moment, protocol-v3]
stack-source: confirmed
augmentations:
  instrument: 04b-instrument.md
  benchmark: 05c-benchmark.md
  design-contract: 02c-craft.md
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-viewer-match-day.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md, 04-plan-commentary.md, 04-plan-calibration.md, 04-plan-viewer-lineup-tactics.md, 04-plan-viewer-reports-recovery.md]
  design: 02b-design.md
  contract: 02c-craft.md
  steer: steer.md
  implement: 05-implement-viewer-match-day.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine viewer-match-day"
---

# Plan: Match-Day Panels and Goal Moments

## The Plan

The engine already keeps every value the match-day screen needs. It keeps all nine statistics as running counters (`crates/engine/src/sim.rs:289-328`), an energy value for every player (`crates/engine/src/player.rs:137`), injury and substitution events, and a commentary line on every play event. Until now the plan stopped at one question: what wire shape carries the roster, the running statistics, and energy to the page. The product owner answered it with option A. Each hello team gains a `roster`. The `stats` message gains the nine panel fields and is sent once per simulated second as well as at full time. A new `condition` message carries the 22 energy values. The protocol becomes version 3, which it shares with the lineup editor's additions.

So this plan is now complete in 16 steps over 28 files. Steps 1 to 12 build the page: a pure event reducer, the per-frame feed with commentary lines, the header, the lineups, the statistics panel, the goal moment, reduced motion, the empty state, and hidden sent-off markers. Steps 13 to 15 build the three wire additions in about 150 lines of Rust across 10 files. They reuse `MatchFigures::new` for the rounding the `match-stats` record already uses, and `Simulation::players()` for energy. No new engine counter and no engine crate change is needed. Sixteen implementation-detail decisions are recorded under `## Assumptions`.

This slice lands first among the viewer slices that raise the version. It therefore changes `PROTOCOL_VERSION` to 3, and the lineup editor adds its fields under the same number. The top risk is still timing. The server leads playback by up to 500 ticks, so a goal, a statistics line, or an energy value shown when it arrives would appear before the pitch draws it. Every message is released by the rendered tick, never by arrival.

## Current State

- `web/` holds the same modules as when the first plan was written (`git log -- web/`: last change `e79ad61`, product wording). `node --test web/tests/*.test.mjs` passes 47 of 47 tests this run. The directory form `node --test web/tests/` fails on this machine. Use the file glob.
- `web/index.html` holds the header placeholder, the "Lineups" left column, the statistics region, and the "Match feed" right column as `region--empty` placeholders. `web/main.mjs` `onMessage` (`main.mjs:122-130`) handles only the stoppage index and the full-time scrubber maximum. `start(hello)` (`main.mjs:65-73`) reads the kit colours and team names from `hello.teams`. Every socket close reads "Stream ended", including the expected close after full time (`06-verify-viewer-pitch.md:240` routes that copy here).
- `web/socket.mjs:28-40` hands the hello to `onHello` and every other JSON message to `onMessage`. The page reads the protocol version from the engine (`main.mjs:257`), so the version rise needs no page logic.
- `MatchEvent` (`crates/protocol/src/event.rs:98-166`) carries `tick`, `minute`, `minute.added`, `event.type` (16 kinds, including `injury`, `substitution`, and `ai-decision`), `team.id`, both scores, `player.id`, `player.secondary_id` (the player coming on, on `substitution`), `card.kind`, `foul.advantage`, the change fields, and `commentary`.
- **Statistics.** `Summary` (`sim.rs:289-328`) counts fouls, offsides, corners, shots, shots on target, expected goals, passes, completed passes, and possession ticks per team. `MatchFigures::new` (`crates/engine/src/observe/mod.rs:132-160`) rounds expected goals to two decimals and derives pass accuracy and possession share to one decimal. The contract keys already exist: `stats.possession_pct`, `stats.shots`, `stats.shots_on_target`, `stats.xg`, `stats.passes`, `stats.pass_accuracy_pct`, `stats.fouls`, `stats.corners`, and `stats.offsides` (`.ai/observability.md:33-41`, `observe/mod.rs:106-122`, `:224-228`). The wire `Stats` (`crates/protocol/src/message.rs:50-66`) holds `tick`, `minute`, both scores, `possession.changes`, `ball.max_speed`, and `ball.idle_ticks`. `stream_run.rs:102-113` sends it once, after the last event.
- **Energy.** `Player::energy` is public (`player.rs:137`), and `Simulation::players()` (`sim.rs:450`) returns the players in wire-slot order, home first. The engine refreshes energy every 50 ticks (`crates/engine/src/fatigue.rs`, `REFRESH_TICKS`). No sim change is needed to read it.
- **Roster data.** `Simulation::teams()` (`sim.rs:441`) returns each `Team` with `player_ids`, `player_names`, `squad` (shirt and `Position`, `crates/engine/src/team.rs:41-47`), `lineup` (squad index per slot), `bench`, and `kit`. `Position` serialises as its code (`"GK"`, `"CB"`, …; `crates/engine/src/data/team.rs:16-28`). The lineup exists only after `Simulation::new`, because the computer manager's pre-match setup runs there.
- **Hello construction.** Three sites build a protocol `TeamRef`: `serve.rs:35-56` and `record.rs:34-58` build the hello **before** `Simulation::new` (`serve.rs:111`, `record.rs:64`); `bench.rs:149` builds it after (`bench.rs:122`). Four more literals sit in tests (`crates/protocol/src/lib.rs:252`, `message.rs` tests, `crates/stream/tests/common/mod.rs`, `crates/stream/tests/fixture.rs`). The `TeamRef` in `simulate.rs:43` and `resume.rs:53` is the observability record's type, not the protocol one.
- **One driver.** `serve`, `record`, and the streaming `bench` all run `stream_run::drive` (`serve.rs:112`, `record.rs:67`, `bench.rs:209`). A periodic message added in `drive` reaches the socket, the recorded file, and the stream benchmark alike.
- **Version.** `PROTOCOL_VERSION` is 2 (`crates/protocol/src/lib.rs:44`), with the version rule in its doc comment (`lib.rs:21-43`). A recorded file's header carries the version, and replay refuses a mismatch by name (`crates/stream/src/record.rs:65`, `:218-221`). No recorded `.smfx` file is tracked in the repository (`git ls-files` this run). `cargo test -p protocol` passes 27 tests this run (25 unit, 2 document).
- A sent-off player parks at `parking_spot()` beside the touchline (`crates/engine/src/pitch.rs:155-163`). `grep -rn parking web/*.mjs` returns nothing, so the page still draws parked markers. `04-plan-match-rules.md` R5 hands this job to this slice.
- `docs/design/realism/02-event-contract-redesign.md` (not yet committed) is a planned later programme that names this slice as a consumer to migrate. Nothing in it is scheduled before this slice.
- `06-verify-viewer-pitch.md:163` routes BENCH-P95 here: the 16.6 ms p95 frame-time budget sits below one 60 Hz frame interval and cannot be met. This slice restates it relative to `refresh_hz`.
- Slice status in `00-index.md` this run: `tactics-and-ai`, `commentary`, and `calibration` complete. `viewer-lineup-tactics` depends on this slice. Its OQ-1 and OQ-2 were answered in the same product-owner round (`po-answers.md` 2026-09-23T08:37:28Z).

## Simplicity Ladder

| Capability | Rung | Evidence |
|---|---|---|
| Event reducer (score, feed, statistics, and energy at tick N) | rung 4 new-code | No client-side event state exists. `web/stoppages.mjs` indexes ticks only. About 200 lines of pure code over the message shapes |
| Per-frame feed batching | rung 2 native-platform | `requestAnimationFrame`, already driving `main.mjs`, plus one `DocumentFragment` insert per frame. No virtual-list library, because 40 to 150 rows per match is small |
| Feed line text | rung 3 reuse | the `commentary` field on every play event (`event.rs:163-165`); no page-side phrasing |
| Club crests and kit chips | rung 3 reuse | `web/mark.mjs` `drawMark()`, which `02c-craft.md` item 8 mandates for crests |
| Tabular numerals and fonts | rung 3 reuse | `web/fonts/fonts.css` and the `.tl-num` class already in `layout.css` |
| Reduced motion | rung 2 native-platform | the CSS `@media (prefers-reduced-motion: reduce)` block and `matchMedia` for the banner timer |
| Goal banner and pulse | rung 2 native-platform | CSS transitions on `opacity` and `transform`, per `02c-craft.md` Motion. No animation library |
| Parked-marker test | rung 3 reuse | a port of `is_parking_spot()` from `crates/engine/src/pitch.rs:161` |
| Statistics values on the wire | rung 3 reuse | `crates/engine/src/observe/mod.rs` → `MatchFigures::new()` for the rounded and derived values, and `Summary` for fouls, corners, offsides, and shots. Exact match with the `match-stats` record. Reuse as-is |
| Statistics key names | rung 3 reuse | the contract keys in `.ai/observability.md:33-41`. Reuse as-is |
| Player energy on the wire | rung 3 reuse | `crates/engine/src/sim.rs` → `Simulation::players()` and the public `Player::energy`. Reuse as-is; no accessor added |
| Roster on the wire | rung 3 reuse | `crates/engine/src/sim.rs` → `Simulation::teams()` (`lineup`, `bench`, `squad`, `player_ids`, `player_names`). Reuse as-is |
| One hello builder for three commands | rung 3 reuse (extract) | the three `Hello` literals in `serve.rs`, `record.rs`, and `bench.rs` repeat the same `TeamRef` construction. Extract one `hello_teams(&Simulation)` into `stream_run.rs`, which all three already import |
| JSON serialisation of the new messages | rung 3 reuse | `serde` with `deny_unknown_fields` and dotted `rename`s, as every existing message does (`message.rs:1-3`) |
| JavaScript test runner | rung 2 native-platform | `node:test`, 47 tests passing this run |

## Applied Learnings

- `.ai/solutions/INDEX.md` does not exist (checked on the first run of this plan; no solutions directory was added since, per `git status`). No applicable learnings found.
- Carry-over from `04-plan-match-rules.md` R5: hide parked markers (step 12).
- Carry-over from `06-verify-viewer-pitch.md:163` BENCH-P95: restate the budget relative to `refresh_hz` (step 16).
- Carry-over from `06-verify-viewer-pitch.md:240`: name full time distinctly from a dropped stream (step 10).
- **Repeat-deferral tripwire.** `00-index.md` `runtime-evidence-deferrals` is `[]` this run, so no wall is inherited. This slice names one new wall, reduced-motion emulation, which is classified in `## Verification Strategy`.

## Likely Files / Areas to Touch

- `web/match-state.mjs` (new): a pure reducer from event, stats, and condition messages to the state at a tick.
- `web/feed.mjs` (new): the per-frame batcher and the feed renderer.
- `web/scoreboard.mjs` (new): the header score bug, clock, kit chips, and speed.
- `web/lineups.mjs` (new): the two lineup columns from the hello roster, the fatigue bar, and the condition word.
- `web/stats.mjs` (new): the statistics panel model and renderer.
- `web/goal-moment.mjs` (new): the pulse, the banner, the highlight, and reduced motion.
- `web/components/panels.css` (new).
- `web/index.html`, `web/main.mjs`, `web/layout.css`, `web/tokens.css`, `web/pitch.mjs`, `web/tests/helpers.mjs` (modified).
- `web/tests/match-state.test.mjs`, `feed.test.mjs`, `stats.test.mjs`, `lineups.test.mjs`, `pitch.test.mjs` (new).
- `crates/protocol/src/message.rs`: `RosterEntry`, `TeamRef.roster`, nine fields on `Stats`, and a new `Condition` message.
- `crates/protocol/src/lib.rs`: `PROTOCOL_VERSION` 3 with its reason, `MESSAGES`, and the exhaustive name test.
- `crates/protocol/tests/document.rs` and `docs/reference/protocol.md`: the document and its test.
- `crates/engine-cli/src/stream_run.rs`: the shared hello helper and the two periodic messages.
- `crates/engine-cli/src/serve.rs`, `record.rs`, `bench.rs`: build the simulation before the hello and call the helper.
- `crates/stream/tests/common/mod.rs`, `crates/stream/tests/fixture.rs`: test literals fill the new fields.

## Proposed Change Strategy

Build from the inside out, as the two earlier viewer and stream slices did. The wire additions come first (steps 13 to 15), because the page modules read their shapes. Then pure page modules under `node --test`, then renderers, then wiring, then the browser drive. The step numbers keep the order the earlier revisions used, so the implement order is 13, 14, 15, then 1 to 12, then 16.

NFR-2 (`yields-to: C2`) governs timing. A panel never shows what the pitch has not drawn. Every event, stats message, and condition message is stored by tick in `match-state`. Each animation frame asks "what is true at the rendered tick?" The first frame whose rendered tick is at or past the goal tick applies the score, the banner, the feed line, and the commentary line together. That is "within one frame of the goal tick" by construction. It also makes rewind correct: scrubbing back before a goal restores the earlier score, removes the later feed lines, and shows the statistics and energy of that earlier second.

The slice's own risk rule governs cadence: panels update per released message, never per tick. One flush per frame writes only the nodes whose values changed. The engine sends statistics and energy once per simulated second (every 50 ticks), which matches the energy refresh interval, so no value on the wire is staler than the engine's own.

NFR-6 governs every state colour. Cards carry "Yellow" or "Red", fatigue bands carry a word, injury rows carry "Injured", and highlighted feed rows carry the event word. Colour is never the only signal (`02c-craft.md` item 9, `steer.md`).

The wire shape is the product owner's (Q-1 option A, `po-answers.md` 2026-09-23T08:37:28Z). This plan settles only the details inside it: field names, rounding, the order of roster entries, the send tick, and which commit raises the version (A-16 to A-20).

## Step-by-Step Plan

1. **Match state.** `web/match-state.mjs`: `MatchState` stores events, stats messages, and condition messages sorted by tick. `at(tick)` returns `{ home, away, entries, cards, injuries, substitutions, sentOff, goals, stats, energy }` from every message with `tick <= tick`. `stats` and `energy` are the newest released message of each kind, or `null` before the first. It is incremental going forward and recomputes from the start on a backward seek (about 150 events; stats and condition use a binary search by tick). Every event-kind string that the new modules read (the feed highlight kinds, the hidden `ai-decision` kind, `goal`, `full-time`) sits in one exported table in this module (A-15). It is pure, with no DOM. Tests: kick-off returns 0 to 0 with no entries and `stats` null; a goal at T shows 1 to 0 at T and 0 to 0 at T-1; a goal that arrives 400 ticks before the rendered tick is not visible until the rendered tick reaches it; a seek backwards removes later entries and returns the earlier stats message.
2. **Feed batcher.** In `web/feed.mjs`, `FeedBatcher` takes the released entries each frame and returns one batch per frame. The renderer inserts the batch through one `DocumentFragment`. Tests with a synthetic burst: 40 events inside one simulated second at 8x all appear, in tick order, with none dropped, and no frame inserts twice.
3. **Feed renderer.** Each row shows the minute stamp (`45+2'` from `minute` and `minute.added`) and the event's `commentary` line. A `tactics-change` row, which has no commentary, shows the event kind in words ("Tactics change — Oakmere Rangers"). `ai-decision` rows are not shown in the feed; the resulting `tactics-change` or `substitution` row carries the visible outcome. Goal, card, substitution, and injury rows carry a highlight class plus the event word. The feed scrolls to the newest row unless the manager has scrolled up. The region carries `aria-live="polite"` for goals and cards only, so a screen reader is not flooded at 8x.
4. **Empty state.** With no released entries, the feed shows the text from Assumption A-9 and the header reads 0 to 0. The statistics panel shows every row at 0 (0.0 for shares and expected goals) until the first stats message is released. The lineups show every player as Fresh. This state returns after a rewind to tick 0.
5. **Header.** `web/scoreboard.mjs` fills the 56-pixel band with two crests from `drawMark()` in kit colours, team names, the score bug in `--tl-font-display` at 30 pixels or larger, and the clock and speed indicator in `--tl-font-num` with `tabular-nums`. The clock and speed move from the control strip's readout to the header, and the control strip keeps its own readout for the effective speed.
6. **Statistics model.** `web/stats.mjs` `toPanel(message)` returns nine rows in the order the slice lists: possession (`stats.possession_pct`), shots (`stats.shots`), on target (`stats.shots_on_target`), expected goals (`stats.xg`), passes (`stats.passes`), pass accuracy (`stats.pass_accuracy_pct`), fouls (`stats.fouls`), corners (`stats.corners`), and offsides (`stats.offsides`). Each row holds home and away values read from index 0 and 1 of the pair, and an accessible label. Values are shown exactly as the message carries them (expected goals to two decimals, shares to one decimal with a `%` sign). `toPanel(null)` returns the zero model for step 4. Test: every one of the nine keys in a factory message and in one stats line captured from a recorded match appears in the model with an equal value.
7. **Statistics renderer.** A two-column tabular grid under the control strip, using `--tl-fg` on `--tl-panel` in the 14-pixel numeric face. It updates when the released stats message changes and on seek. Before writing any banner motion, measure the numeric row's contrast in the drive and require at least 4.5:1, as `02c-craft.md` Colour application and the slice's top risk require.
8. **Lineups.** `web/lineups.mjs` renders two columns from `hello.teams[i].roster`: the first 11 entries in wire-slot order are the starters, and the rest are the bench. Each starter row shows position, shirt, name, fatigue bar, condition word, and card marker. Energy for wire slot `s` is `energy[s]` from the released condition message (home slots 0 to 10, away 11 to 21). Energy maps to three bands: Fresh (0.70 to 1.00), Tiring (0.40 to 0.69), and Exhausted (below 0.40). Each band carries its word, and the bar uses `--tl-success`, `--tl-warning`, or `--tl-danger`. A card shows "Yellow" or "Red" beside the `--tl-card-yellow` or `--tl-danger` chip. An injured player's row reads "Injured" from the released `injury` event. A substitution replaces the row whose `player.id` left with the roster entry whose `player.id` equals `player.secondary_id`, and that row reads energy from the same wire slot. A sent-off player's row reads "Sent off". Tests: every band returns a word, no band returns a colour alone, and a substitution swaps the row to the named bench entry.
9. **Goal moment.** `web/goal-moment.mjs` `play(goal)` runs in the frame that releases the goal. It sets the score, starts the score-bug pulse, shows the banner "GOAL — <team> <home>–<away> <minute>'", and highlights the feed row with its commentary line. The banner enters over 250 ms, holds, and leaves over 200 ms. Total visible time is 1,500 ms, set by the tokens in step 11. Only one banner shows at a time, and a second goal replaces the first. At 8x playback, the banner timer uses wall time, not match time. `matchMedia('(prefers-reduced-motion: reduce)')` drops the motion but keeps the banner: it appears and disappears without transition, and the pulse is off. The CSS rule for this sits in the same block as the `[data-motion="reduce"]` proxy attribute (see Verification Strategy).
10. **Wire the page.** In `web/main.mjs`, `start(hello)` passes both rosters to `lineups.mjs`. `onMessage` routes `event`, `stats`, and `condition` messages to `MatchState`. The existing `frame()` calls `panels.flush(renderedTick)` after `pitch.draw`, and one call updates the header, the feed, the lineups, the statistics, and the goal moment. Extend the read-only hook with `matchDay()`, which returns `{ renderedTick, score, feedCount, lastFeed, goalShownAtTick, bannerVisible, stats, lineupLabels }` and has no setter. Separate full time from a dropped stream: after a `full-time` event, a socket close shows "Full time. The whole match is stored and plays back." with state word "Full time". Any other close keeps "Stream ended". Emit `viewer.goal_moment` with `goal_tick`, `rendered_tick`, and `frame_delta` so the one-frame criterion has a signal as well as a screenshot.
11. **Styles and motion tokens.** `web/components/panels.css` covers the score bug, banner, feed rows, lineup rows, and statistics grid. `web/tokens.css` gains `--tl-goal-in: 250ms`, `--tl-goal-hold: 1050ms`, `--tl-goal-out: 200ms`, and `--tl-pulse: 600ms`. Transitions use `opacity` and `transform` only, with `bounce: 0`. The reduced-motion block sets them to `0.01ms`. `web/layout.css` removes the `region--empty` placeholders for the three regions this slice fills. The left column keeps its label for the lineups.
12. **Hide parked markers.** In `web/pitch.mjs`, skip any marker whose position satisfies a port of `is_parking_spot()` (`crates/engine/src/pitch.rs:161-163`: `y` within 0.01 m of `-(HALF_WIDTH + PARKING_OFFSET)` and `|x|` between 9.99 and 20.01 m). Read the two constants from `pitch.rs` at implement time. Test it in a new `web/tests/pitch.test.mjs`: all 22 spots from `parking_spot(team, slot)` are hidden, and points on the pitch are not.
13. **Roster on the wire and version 3.** In `crates/protocol/src/message.rs`, add `RosterEntry { player.id: String, player.name: String, player.shirt: u8, player.position: String, player.squad_index: u32 }` with `deny_unknown_fields`, and `roster: Vec<RosterEntry>` on `TeamRef` with `#[serde(default)]`. In `crates/engine-cli/src/stream_run.rs`, add `pub(crate) fn hello_teams(sim: &Simulation) -> [TeamRef; 2]`. It builds each team's club fields as today, then the roster: the 11 starters in wire-slot order (`team.lineup[slot]` gives the squad index), followed by the named bench in `team.bench` order. Shirt and position come from `team.squad[i]`, and the position is the `Position` code (A-17). In `serve.rs` and `record.rs`, move `Simulation::new` before the hello, and build `teams` with the helper in all three commands. In `crates/protocol/src/lib.rs`, set `PROTOCOL_VERSION` to 3. Its doc comment states why: `stats` changes cadence from once at full time to every simulated second, and a new message type joins (A-16). Tests: a unit test that `hello_teams` returns 11 starters then the bench for both teams, with every `player.id` present in the team file; the `MESSAGES` round-trip test with a filled roster; the recorder and replayer tests pass with the new version.
14. **Running statistics on the wire.** In `message.rs`, add nine fields to `Stats`, each a home-first pair: `stats.possession_pct: [f64; 2]`, `stats.shots: [u32; 2]`, `stats.shots_on_target: [u32; 2]`, `stats.xg: [f64; 2]`, `stats.passes: [u32; 2]`, `stats.pass_accuracy_pct: [f64; 2]`, `stats.fouls: [u32; 2]`, `stats.corners: [u32; 2]`, `stats.offsides: [u32; 2]`. The existing seven fields stay. In `stream_run.rs`, extract `fn stats_message(sim: &Simulation) -> Stats`, which takes the rounded and derived values from `MatchFigures::new(&sim.summary(), sim.managers())` and the counts from `sim.summary()`. `drive` sends it after the tick's events whenever `record.tick % ticks_per_second == 0`, where `ticks_per_second` is `(1.0 / dt).round()` (50 at the shipped `dt`), and once more at full time as today (A-18). Tests: over a 2-minute seeded match driven into a `Vec`, the stats messages number `ticks / 50 + 1`, their ticks rise, and the last one's nine fields equal `MatchFigures::new` and `Summary` for the same seed.
15. **Energy on the wire.** In `message.rs`, add `Condition { tick: u32, energy: Vec<f64> }` with `deny_unknown_fields` as `ServerMessage::Condition` (`type: "condition"`). `drive` sends it right after each periodic stats message: `energy` holds `sim.players()[i].energy` for the 22 wire slots, home first, rounded to three decimals (A-19). No condition message is sent at full time. Tests: every condition message holds 22 values in 0.0 to 1.0, and at least one value falls below 1.0 by the end of a 2-minute match. Update `docs/reference/protocol.md` (version 3, the roster table, the new stats cadence and fields, the `condition` section), `crates/protocol/tests/document.rs` if it lists fields by name, and the test literals in `crates/protocol/src/lib.rs`, `crates/stream/tests/common/mod.rs`, and `crates/stream/tests/fixture.rs`.
16. **The full check.** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `node --test web/tests/*.test.mjs` (the file glob). Search the changed source for workflow vocabulary before the commit, as the match-rules plan requires. Re-measure the stream throughput and stream peak memory targets with `target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json` against `05c-benchmark.md` (tripwire 8.55 MB). Restate BENCH-P95 in `05c-benchmark.md` as "p95 frame time at most one frame interval (1000 / `refresh_hz`) plus 2 ms", and record the four page targets against the `history/05c-benchmark-3.md` baseline. Then run the browser drives in `## Test / Verification Plan`.

## Verification Strategy

The slice defines seven acceptance criteria. Five are user-observable and two are unit tests.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| **AC-1** a goal updates score, banner, feed line, and commentary line within one frame of the goal tick | `Claude_Browser` drive: screenshot at the goal tick plus `__touchline.matchDay()` and `viewer.goal_moment` (`web-2`) | Windows 11, built engine, a fixture recorded after step 15 with a noted goal tick — **yes** | `matchDay()` hook (step 10); `viewer.goal_moment` signal; a fresh fixture (version 3) | `node --test` over `match-state` (goal visible at T, not at T-1) → pre-registered deferral |
| **AC-2** kick-off with no events shows the empty-state text and 0 to 0 | `Claude_Browser` drive paused at tick 0 plus DOM read (`web-2`) | as AC-1 — **yes** | Empty-state copy (A-9); pause before the first event | `match-state` unit test at tick 0 → deferral |
| **AC-3** a fatigue change updates the bar and the labelled condition marker | `Claude_Browser` drive plus DOM read of the label (`web-2`), fatigue driven by the live `condition` message | as AC-1 — **yes, once step 15 lands** | `condition` message (step 15); `lineupLabels` in `matchDay()` (step 10); `lineups.test.mjs` band rule | `lineups.test.mjs` with a factory condition message → deferral |
| **AC-4** the statistics panel shows every field equal to the stream values | `node --test` over `stats.mjs` with a factory message and a line captured from a recorded match (`web-1`), plus the Rust test that the stats message equals `MatchFigures` (`step 14`) | Node 22 and cargo — **yes** | Stats fields (step 14); captured line in `web/tests/helpers.mjs` | none needed |
| **AC-5** at 8x with more than 10 events per second, the feed inserts all of them, batched per frame | `node --test` over `FeedBatcher` with a synthetic burst (`web-1`) | Node 22 — **yes** | `FeedBatcher` as a pure module (step 2) | none needed |
| **AC-6** reduced motion: banner without motion, pulse disabled | Proxy: `Claude_Browser` drive with `?motion=reduce` setting `data-motion="reduce"`, which the same CSS block serves as the media query; computed `transition-duration` read through the script tool (`web-2`) | **Real media-query emulation is not available.** The `resize_window` tool offers only `colorScheme` light or dark, and Playwright is in `toolchains-absent` | The `data-motion` proxy seam sharing one rule block with the media query (step 9) | proxy → real media query when the operator turns off Windows animation effects → pre-registered deferral |
| **AC-7** at 1280 by 800 a reviewer reads the clock, score, and statistics, with focus rings visible | Proxy: `Claude_Browser` screenshot at 1280 by 800 plus computed contrast ratios for every numeric node (`web-2`); then the human check (`web-5`) | a human reviewer on the reference laptop — **environment-negotiable** | Contrast read through the script tool; `:focus-visible` screenshots | proxy → human check at `integration` → deferral |

**Constraint resolution, one line per environment dependency on a user-observable AC:**

- AC-3 (and AC-4, which is not user-observable but shared the same wall): `constraint-resolution: prerequisite-slice: viewer-match-day` — `wall-ownership: code-owned`. The wall was a missing message in this repository. Steps 13 to 15 build the roster, the stats fields, and the `condition` message inside this slice, so the wall is dissolved by in-slice work, not parked.
- AC-6: `constraint-resolution: proxy+deferral: the operator turns off Windows Settings > Accessibility > Visual effects > Animation effects, reloads the page, and runs the AC-6 drive; Chromium then reports prefers-reduced-motion: reduce` — `wall-ownership: environment-negotiable`. The agent may not change system settings, and a person can perform the act on demand. The proxy is `data-motion="reduce"`, served by the same CSS rule block, so the two cannot drift apart.
- AC-7: `constraint-resolution: proxy+deferral: the product owner reads the match screen on the reference laptop at 1280 by 800 in a lit room during the integration slice's charter-scenario run` — `wall-ownership: external` (human judgement). The slice definition pre-registers this as a human check at rung web-5. The proxy is computed contrast of at least 4.5:1 on every numeric node, plus focus-ring screenshots.
- AC-1 and AC-2 need no credential, device, or external service.

**Tooling resolution.** Every tool above is inside `stack:`. `Claude_Browser` is the chosen interactive driver (shape Q29), and `node:test` and `cargo test` need no install. Playwright remains absent and unused.

## Test / Verification Plan

### Automated checks

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. New Rust tests: the `hello_teams` roster order (step 13), the stats message count and values over a seeded 2-minute match (step 14), the condition message shape (step 15), and the `MESSAGES` round-trip and document tests with the new message.
- `node --test web/tests/*.test.mjs`: the 47 existing tests plus `match-state.test.mjs` (AC-1 and AC-2 logic, release by tick), `feed.test.mjs` (AC-5), `stats.test.mjs` (AC-4), `lineups.test.mjs` (the AC-3 label rule and the substitution swap), and `pitch.test.mjs` (parked markers).
- Benchmark: the stream throughput and stream peak memory targets re-measured against `05c-benchmark.md` (step 16); the four page targets compared against `history/05c-benchmark-3.md`, with p95 restated. The processor-time gate runs `bench` without the stream and does not pass through `drive`, so it is not expected to move; it is re-run as a regression guard.
- Instrument check: `viewer.goal_moment` in the console and in `__touchline.signals`.

### Interactive verification (human-in-the-loop)

Platform `web`; driver `Claude_Browser` (shape Q29). Evidence goes under `verify-evidence/viewer-match-day/` per the web adapter layout. Bootstrap (a fresh fixture is required, because version 3 refuses earlier recorded files):

```bash
cargo build --release -p engine-cli
target/release/engine-cli.exe record --seed 7 --out fixture.smfx --minutes 90
target/release/engine-cli.exe replay --fixture fixture.smfx --speed 1.0 --web web
```

| Criterion | Drive | Pass criteria |
|---|---|---|
| AC-1 | Play to the noted goal tick at 1x. Screenshot at the frame `viewer.goal_moment` fires, then read `matchDay()` | `frame_delta` ≤ 1; score, banner, and the newest feed row with its commentary line all present in the same screenshot; `goalShownAtTick` ≥ goal tick and < goal tick + ticks per frame |
| AC-2 | Load, pause before tick 1, screenshot, read `matchDay()` | `feedCount` 0, the empty-state text present, score reads 0–0, every statistics row reads 0 |
| AC-3 | Play at 8x until a player's band changes (energy below 0.70), then read `matchDay().lineupLabels` and the row DOM | the fatigue bar width changes between two released condition messages and the condition node's text is one of Fresh, Tiring, or Exhausted |
| AC-6 | Load `?motion=reduce`, play to the goal tick, read the computed `transition-duration` on the banner and the pulse | banner visible; durations ≤ 0.01 ms; no pulse class applied. Record that the real media query is deferred |
| AC-7 | Resize to 1280 by 800, screenshot, compute the contrast of every `.tl-num` node, tab through every control and screenshot focus | every ratio ≥ 4.5:1; a focus ring is visible on each control. The human reading is deferred to integration |

## Risks / Watchouts

- **R2 (high) Messages arrive ahead of playback.** The 500-tick server lead (`po-answers.md` stream-protocol Q10) would let the feed announce a goal, or the panels show a number, before the pitch draws it. Release every message by rendered tick (step 1), with a test that drives a goal arriving 400 ticks early.
- **R3 (medium) Panel work steals frame budget.** Flush once per frame, write changed nodes only, and never update statistics or energy per tick. The p95 reading after this slice is compared against the viewer-pitch baseline.
- **R4 (medium) The banner timer at 8x.** Match time runs eight times faster, so a match-time timer would flash the banner for under 200 ms. The timer uses wall time (step 9).
- **R5 (medium) Rewind across a goal.** A seek back must remove the goal from the score and the feed without replaying the banner. `at(tick)` recomputes the state, and the goal moment fires only on a forward crossing during play.
- **R6 (medium) Version 3 refuses earlier recorded files.** The recorded-file header carries the protocol version, and replay refuses a mismatch by name (`crates/stream/src/record.rs:219`). No recorded file is tracked, but local fixtures from an earlier build stop replaying. Verify records a fresh fixture (bootstrap above).
- **R9 (medium) Periodic message volume.** A full match adds about 5,400 stats and 5,400 condition messages (roughly 3 to 4 MB of JSON) beside the tick frames, on the socket and in recorded files. Energy is rounded to three decimals. Step 16 re-measures stream throughput and stream peak memory against the 8.55 MB tripwire. A breach routes back to this plan, where the cadence (A-18) is the lever.
- **R10 (medium) The lineup editor changes the roster before kick-off.** `viewer-lineup-tactics` adds a pre-match hold and a `set-lineup` command under the same version. The roster built here reflects the computer manager's pre-match lineup. Each entry carries `player.squad_index`, so that slice can re-order the roster or extend it to the whole squad without renaming fields. Telling the page about a changed lineup is that slice's work.
- **R8 (medium) A later event-contract redesign.** `docs/design/realism/02-event-contract-redesign.md` renames event kinds, raises the protocol version again, and may raise the event count to about 1,400 per match. Mitigation: one event-kind table in `match-state.mjs` beside `STOPS_PLAY` in `web/stoppages.mjs` (A-15), and a feed batcher that does not depend on the event count. A-2 (no virtual list) is re-checked if that programme lands.
- **R7 (low) Reduced-motion proxy drift.** The proxy attribute and the media query share one CSS block, so they cannot diverge.
- Watch: dotted JSON keys (`event.type`, `home.score`, `minute.added`, `player.secondary_id`, `stats.xg`, `player.squad_index`), exactly as viewer-pitch warned. A page reads `message['stats.xg']`, never `message.stats.xg`.
- Watch: `node --test web/tests/` (directory form) fails on this machine. Use the file glob.
- Watch: moving `Simulation::new` before the hello in `serve.rs` must keep the port line printed before the page address (`serve.rs:58-60`), because the launcher and the drives read it.

## Dependencies on Other Slices

- `viewer-pitch` (complete): the page, the socket, history, the scheduler, the mark function, the tokens.
- `stream-protocol` (complete): the event messages, the recorder, and the replayer. This slice extends its message set.
- `match-rules` (complete): event types, cards, added minutes, and parked markers.
- `commentary` (complete): the `commentary` line on every play event. The feed uses it directly.
- `tactics-and-ai` (complete): energy, injury, and substitution. The values exist in the engine.
- `calibration` (complete): `Summary`, `MatchFigures`, and the contract key names the stats message reuses.
- `viewer-lineup-tactics` (defined; its OQ-1 and OQ-2 were answered in the same product-owner round): it depends on this slice. It reuses the hello roster added here and adds its squad, lineup hold, `set-lineup`, and tactics schema under protocol version 3, which this slice introduces.

## Assumptions

Each entry is an autonomous decision, stamped `class: implementation-detail` per `_decision-classes.md`. The wire shape itself is the product owner's answer (Q-1 option A), not an assumption.

- **A-1** Events, stats messages, and condition messages are released by the rendered tick, not by arrival. Why: C2 and the one-frame criterion; the stream leads playback by up to 500 ticks. `class: implementation-detail`.
- **A-2** The feed batches with one `DocumentFragment` insert per animation frame, and no virtual list is used. Why: 40 to 150 rows per match (`02b-design.md:51`); ladder rung 2. `class: implementation-detail`.
- **A-3** Seeking backwards recomputes the match state from the start, rather than keeping per-tick snapshots. Why: about 150 events, O(n) per seek, and no memory cost; stats and condition are found by binary search. `class: implementation-detail`.
- **A-4** The banner timer runs on wall time, not match time. Why: the 1.5-second rule is a human-perception rule. `class: implementation-detail`.
- **A-5** Only one banner shows at a time, and a later goal replaces it. The banner does not replay on rewind. Why: the least surprising behaviour, and the same rule the notice uses. `class: implementation-detail`.
- **A-6** Energy bands are Fresh 0.70 to 1.00, Tiring 0.40 to 0.69, and Exhausted below 0.40, each with its word. Why: NFR-6 requires words; the thresholds are presentational and do not change engine behaviour. `class: implementation-detail`.
- **A-7** Reduced-motion verification uses a `data-motion="reduce"` proxy that shares the media query's CSS block, and the real query is deferred to an operator act. Why: the browser tool exposes only colour-scheme emulation, and changing OS settings is forbidden to the agent. `class: implementation-detail`.
- **A-8** Parked markers are hidden by a client-side port of `is_parking_spot()`, not by a new wire bit. Why: no further protocol change, and match-rules R5 assigned it here. `class: implementation-detail`.
- **A-9** The empty-state feed copy is "No events yet. The feed fills as the match plays." with the header at "0 – 0". Why: `02b-design.md:55` defines the state but gives no literal copy, and wording is implementation-detail per the decision classes. `class: implementation-detail`.
- **A-10** After a `full-time` event, a socket close reads "Full time" rather than "Stream ended". Why: `06-verify-viewer-pitch.md:240` ambiguous copy; the expected close is not a fault. `class: implementation-detail`.
- **A-11** BENCH-P95 is restated as at most one frame interval (1000 / `refresh_hz`) plus 2 ms. Why: routed here by `06-verify-viewer-pitch.md:163`; NFR-2's 60 frames per second is unchanged, and only the instrument's threshold is corrected. `class: implementation-detail`.
- **A-12** The feed uses the event's `commentary` line and falls back to the event kind in words only for `tactics-change`, and it hides `ai-decision` rows. Why: `commentary` ships on every play event (`event.rs:163-165`); `ai-decision` is reasoning, and its visible outcome arrives as its own row. `class: implementation-detail`.
- **A-15** The new page modules keep every event-kind string they read in one exported table in `match-state.mjs`, and do not change `web/stoppages.mjs`. Why: a later programme renames event kinds; one table limits that migration to two known places without widening this slice. `class: implementation-detail`.
- **A-16** This slice raises `PROTOCOL_VERSION` to 3, and the lineup editor adds its fields under the same number. Why: the product owner fixed the number ("these additions ship in version 3", `po-answers.md` 2026-09-23T08:37:28Z); this slice is ordered before `viewer-lineup-tactics` (`04-plan.md` implementation order), both land on the same unreleased branch, and the rule at `lib.rs:21-43` asks for a new version when a message changes meaning, which the `stats` cadence does. Only which commit raises the number is decided here. `class: implementation-detail`.
- **A-17** A roster entry holds `player.id`, `player.name`, `player.shirt`, `player.position` (the `Position` code), and `player.squad_index`. The list is the 11 starters in wire-slot order, then the named bench in bench order. Why: the answer names "player id, name, shirt, position, in wire-slot order"; the bench entries are needed to name the player a `substitution` brings on (`player.secondary_id` carries only an id), and the squad index is the handle the lineup editor's `{ "off", "on" }` change detail uses (`po-answers.md` OQ-2). Players outside the named bench are left to the lineup editor. `class: implementation-detail`.
- **A-18** Periodic stats and condition messages are sent after the tick's events whenever `tick % 50 == 0` (one simulated second at the shipped `dt`, computed from `dt`), and a final stats message is still sent at full time. The stats values come from `MatchFigures::new` and `Summary`, so they equal the `match-stats` record for the same match. Why: "once per simulated second" in the answer; the energy refresh interval is also 50 ticks (`fatigue.rs`); one rounding source prevents the panel and the record from disagreeing. `class: implementation-detail`.
- **A-19** Energy on the wire is rounded to three decimals, and no condition message is sent at full time. Why: the page shows three bands and a bar, so three decimals lose nothing visible, and the rounding keeps the added volume near 1.4 MB per match. `class: implementation-detail`.
- **A-20** The three hello sites share one `hello_teams(&Simulation)` helper in `stream_run.rs`, and `serve` and `record` create the simulation before the hello. Why: the roster's slot order exists only after the pre-match setup inside `Simulation::new`; one helper stops the three copies drifting (ladder rung 3, extract). `class: implementation-detail`.
- **A-21** No engine crate change: energy is read through `Simulation::players()` and the public `Player::energy`. Why: the earlier revision planned an accessor in `sim.rs`, but `sim.rs:450` already exposes the players in wire-slot order. `class: implementation-detail`.
- **A-22** This run is a re-plan on returned answers (review-and-fix, trigger `answers-returned`). The prior revision is snapshotted byte for byte to `history/04-plan-viewer-match-day-2.md` with its `.yaml` and `.html.fragment`, and the change is ledgered as rev 3. Why: `plan.md` review-and-fix mode and `_additive-write.md`. `class: implementation-detail`.
- **A-23** No discovery interview is held. Every implementation question left after the product owner's answer is settled above (A-16 to A-21). Why: autonomous run; none of them changes user-observable scope or a contract beyond the answered wire shape. `class: implementation-detail`.
- Consult: triggers `appetite-medium-or-larger` and `touches-migration` (protocol version and recorded files) hold. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`). Not fired. `class: implementation-detail`.
- A-13 and A-14 (earlier review-run bookkeeping) are superseded by A-22 and live in the history snapshots.

## Blockers

None. Q-1 (the wire shape for the roster, the running statistics, and energy) was answered by the product owner with option A on 2026-09-23 (`po-answers.md` 2026-09-23T08:37:28Z; `steer.md` "Product-owner answers to plan questions"). Its steps are 13 to 15.

## Freshness Research

- Source: MDN `prefers-reduced-motion` and `Window.matchMedia`. Why it matters: AC-6. Takeaway: the media query is the sole signal, and the page reads it at load and on `change`. Chromium derives it from the OS animation setting on Windows, which is the deferral's clearing act.
- Source: MDN `DocumentFragment` and `requestAnimationFrame`. Why it matters: AC-5 and R3. Takeaway: one fragment append per frame is one layout invalidation, and inserting inside the existing `rAF` callback avoids a second callback.
- Source: MDN `aria-live`. Why it matters: the feed at 8x. Takeaway: `polite` regions queue announcements, so a live region on every row floods a screen reader. Only goals and cards are announced.
- Source: the `Claude_Browser` `resize_window` tool schema. Why it matters: AC-6. Takeaway: it offers `colorScheme` light or dark and no reduced-motion emulation, which is the wall recorded above.
- Source: the in-repo `serde` usage (`crates/protocol/src/message.rs`). Why it matters: steps 13 to 15. Takeaway: `#[serde(default)]` on the new `roster` field keeps a hello without it readable; `deny_unknown_fields` still refuses fields a later build adds, which is why the version rises.
- No dependency is added. The page stays plain HTML, CSS, and ES modules with `node:test`, and the Rust side uses the existing `serde` and `serde_json`.

## Recommended Next Stage

- **Option A (default): Implement** → `/wf implement football-manager-match-engine viewer-match-day`. The plan is complete and unblocked. Implement steps 13 to 15 first, then 1 to 12, then 16. Compact the session first; the SessionStart hook re-reads the artifacts.
- **Option C (revisit slice):** `/wf slice football-manager-match-engine`, only if the product owner prefers to move the wire additions into a separate protocol slice that both viewer slices depend on.
