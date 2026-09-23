---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: integration
status: complete
stage-number: 5
created-at: "2026-09-23T14:42:29Z"
updated-at: "2026-09-23T14:42:29Z"
metric-files-changed: 25
metric-lines-added: 3004
metric-lines-removed: 445
metric-deviations-from-plan: 9
metric-review-fixes-applied: 0
commit-sha: "e71ebfdcf5b353e9d197a035a3780eac22768c75"
commits:
  - "e71ebfdcf5b353e9d197a035a3780eac22768c75"
steering-honored:
  - "Image gate and layout: no layout, token, or typography change. The one page change is text in the existing header span (web/main.mjs:113-126), styled by the existing .header__teams rule."
  - "State words: the header names the engine state in words (Engine connected, Engine reconnecting, Engine stopped, Engine finished, Replay), never by colour alone (web/main.mjs:126,195,958,970,989)."
  - "No spinner: no loading element was added."
  - "Dark-path counter definition: not touched; the computer manager's queuing behaviour is unchanged."
  - "Product-owner answers: probe-engine-core Q-1 and the protocol version 3 answers are untouched; no protocol message, record field, or file format changed."
  - "Output boundary: the commit message, code comments, docs, e2e/README.md, and test titles use product language. The whole-match spec was renamed from the plan's name to e2e/tests/first-match.spec.mjs so that no workflow word ships."
tags: [integration, e2e, playwright, docs, license-audit, benchmark, observability, served-stats]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-integration.md
  plan: 04-plan-integration.md
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-commentary.md, 05-implement-calibration.md, 05-implement-viewer-match-day.md, 05-implement-viewer-lineup-tactics.md, 05-implement-viewer-reports-recovery.md]
  verify: 06-verify-integration.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine integration"
---

# Implement: Integration and Charter Scenario

## The Implementation

All four dependency slices were verified when this run started, and the engine served the page, wrote match records, and passed its benchmark. The browser suite, the document set, and the license audit did not exist. The first run of the whole-match spec found a real gap: `serve` wrote the event rows but never wrote `stats.json`, so a match played in the browser left no statistics record.

The run adds a Playwright 1.63.0 suite in `e2e/` with 21 tests in 6 files, five new documents and a new README, and two Rust test files. It makes two product changes. First, `serve` writes the `match-stats` record at full time, the same way `simulate` and `resume` already do. The record shape does not change. A regression assertion failed before the fix. Second, the header shows the engine state in words, because the ratified scenario checkpoint names "Engine connected". The full suite passes: 21 of 21 in 24.4 minutes. The Rust tests pass (310, with 4 ignored), and the page tests pass (126). The benchmark median moves from 418 ms to 420 ms, and 1000 matches take 423.5 seconds.

Verify can now observe every criterion of this slice. The top open risk is that three behaviours stay human checks: legibility and focus, the goal-moment motion, and the tutorial followed from a new clone. No human took part in this run, so all three are still open. The earlier legibility deferral for match day names this slice as the place to clear it.

## Summary of Changes

- Added the browser suite in `e2e/`: its own private package with one pinned dependency, a headless configuration at 1280 by 800, an engine launcher that uses a temporary data folder for each test, and page helpers that find controls by role and name.
- Added six spec files: the whole first match in twelve steps, the pitch, match day, the lineup and tactics, reports and recovery, and the observability records.
- Fixed `engine-cli serve`: it now writes `matches/<match.id>/stats.json` at full time. `launch` runs `serve`, so the launcher is fixed as well.
- Changed the header: the engine-version span now reads "Engine connected · v0.1.0" and follows the socket state.
- Added `crates/engine-cli/tests/licenses.rs`: an SPDX expression audit over `cargo metadata` and `e2e/package-lock.json`.
- Added `crates/engine-cli/tests/docs.rs`: checks that the document set exists, that `cli.md` names every flag of every subcommand, and that every `engine-cli <sub>` in the guides and the README is a real subcommand.
- Added the documents: tutorial, how-to, command-line reference, data-file reference (moved from `content/README.md` and extended with the runtime files), and the engine explanation. Rewrote `README.md` as the front page.
- Recorded the benchmark before and after the changes, and timed one 1000-match run.

## Files Changed

- `e2e/package.json`: new private package. `@playwright/test` is pinned to 1.63.0. Scripts: `test`, `test:viewer`, `test:scenario`, `test:headed`.
- `e2e/package-lock.json`: generated. Four entries, all Apache-2.0 or the dual MIT/Apache expression.
- `e2e/playwright.config.mjs`: headless, 1280 by 800, one worker, `PW_CHANNEL` selects the channel, and two projects (`scenario` 45 min, `viewer` 5 min).
- `e2e/support/engine.mjs`: `startEngine`, `runEngine`, `generateLeague`, `readRecords`, `matchIds`, `killTree`, and `tempDir`.
- `e2e/support/page.mjs`: `openMatch` (captures the page's own `hello` off its socket), `kickOff`, `setSpeed`, `playUntil` (presses Continue at half-time), `queued`, `queueSubstitution`, `changeReaches`, `scrubTo`, and `feedRows`.
- `e2e/tests/first-match.spec.mjs`: the twelve scenario steps against the live engine, with one screenshot attached for each step.
- `e2e/tests/pitch.spec.mjs`: 60 fps at 1x for five minutes, 4x within 2 percent, the lag notice (sustain 3 and sustain 8), rewind exactness, and the interpolation bounds.
- `e2e/tests/match-day.spec.mjs`: the goal moment in its frame, the empty feed at 0–0, the condition words, and reduced motion.
- `e2e/tests/lineup-tactics.spec.mjs`: the three illegal lineups, the mentality chip, one plus four substitutions, the refused sixth substitution, a role change made while paused, and keyboard-only operation.
- `e2e/tests/reports-recovery.spec.mjs`: kill and restart, damaged snapshot, socket drop, half-time report against the feed, saved replay played with no engine, and the first-run panel.
- `e2e/tests/observability.spec.mjs`: event rows by the first stoppage, `stats.json` within 2 s of full time, record kinds, required keys, schema version, and identity.
- `e2e/README.md`: how to run the suite, how the tests find controls, the map from behaviour to test, and the three human checks.
- `crates/engine-cli/src/serve.rs`: new `match_stats()` (line 50). `write_stats` is called at full time (lines 299-303).
- `crates/engine-cli/tests/stream_cli.rs`: the served match must leave `events.jsonl` and a `match-stats` `stats.json` (line 130).
- `crates/engine-cli/tests/licenses.rs`: new. The SPDX parser, 11 unit cases, and the tree audit.
- `crates/engine-cli/tests/docs.rs`: new. The document set, the flag coverage, and the guide commands.
- `web/main.mjs`: `setEngineWord()` (line 113) and its five call sites.
- `docs/tutorials/first-match.md`, `docs/how-to/modding.md`, `docs/reference/cli.md`, `docs/reference/data-files.md`, `docs/explanation/engine.md`: new.
- `README.md`: rewritten as the front page, with links to the four document types, the browser suite, and the benchmark.
- `content/README.md`: now a pointer to the data-file reference and the how-to.
- `.gitignore`: `e2e/node_modules/`, `e2e/test-results/`, and `e2e/playwright-report/`.

## Shared Files (also touched by sibling slices)

- `web/main.mjs` (viewer-pitch, viewer-match-day, viewer-lineup-tactics, viewer-reports-recovery): one helper and five one-line calls. No behaviour other than the header text changed.
- `crates/engine-cli/src/serve.rs` (stream-protocol, viewer-lineup-tactics, viewer-reports-recovery): an addition on the full-time path only.
- `crates/engine-cli/tests/stream_cli.rs` (stream-protocol): assertions added to the existing serve test.
- `README.md` and `content/README.md` (every earlier slice): restructured. The content moved to `docs/`.

## Notes on Design Choices

- The suite binds to behaviour. It finds controls by role and accessible name, and reads state through `window.__touchline`. A `data-testid` is used only for controls with no unique name: `lineup-reason`, `squad-N`, `mentality`, `role-N`, `subs-left`, and `sub-queue`. `#chips`, `#feed`, and `#surface-*` are read by id only to read text.
- The spec captures the `hello` from the page's own WebSocket frames (`page.on('websocket')`), so the identity comparison uses what the page received, and the read-only hook stays unchanged.
- Viewer tests use short matches (`--minutes` 10 to 45) and recorded fixtures (seed 3 has a goal in the first minutes, and seed 7 is used for the pitch). The viewer project finishes in about 13 minutes. Only the whole-match spec plays 90 minutes.
- The 1x frame-rate test reads every signal from the page console, because the in-page ring keeps only 256 rows.
- The license parser treats `WITH` as bound to its license and never as permissive, and treats `/` as OR. A missing license field fails.
- The engine explanation states the best-effort determinism stance and the reason for it: floating-point rounding differs across machines. It points to the replay file as the portable artefact.

## Verification Seams Built

- Whole-match scenario, 12 steps against the live engine → `e2e/tests/first-match.spec.mjs:64` with one `test.step` per step (lines 74-313), each attaching `step-NN.png`. The launcher is `e2e/support/engine.mjs:49` with a temporary `SM_DATA_DIR`. The league comes from `generateLeague(2026)` at `e2e/support/engine.mjs:113`. This enables Playwright headless (web-1) to observe every checkpoint. The last full run passed all 12 steps (`implement-evidence/integration/e2e-full-run1.txt`).
- Playwright covers every interactive viewer criterion → the specs at the lines listed in `e2e/README.md`, plus the page unit tests for memory, statistics fields, and feed batching. This enables `npm test` to report a pass or a failure for each criterion.
- Benchmark under 2 s for one match and under 30 min for 1000 matches → `bench-baseline/integration/before-{1,2,3}.json`, `after-{1,2,3}.json`, `thousand.json`, and `thousand.wall.txt`. This enables a cli-direct comparison against the tripwires.
- Records reach the sink in time → `e2e/tests/observability.spec.mjs:33` reads `readRecords` at `e2e/support/engine.mjs:137`. The fix in `crates/engine-cli/src/serve.rs:299-303` makes the statistics record exist. This enables infra-1 to observe the sink.
- Documents exist and the tutorial runs → `crates/engine-cli/tests/docs.rs:62,77,95` as the automated proxy. The human walkthrough is pre-registered as human check 3 in `e2e/README.md`.
- No GPL or AGPL dependency → `crates/engine-cli/tests/licenses.rs:144,207`.
- Header connection word → `web/main.mjs:113` (`setEngineWord`), called at lines 126, 195, 958, 970, and 989. This enables step 1 of the scenario and the recovery test (`e2e/tests/reports-recovery.spec.mjs:27`) to read the state in words.

## Visual Contract Honored

- 02c-craft.md is present. This run built no new visual surface. The header change puts text in the existing `#engine-version` span, which the `.header__teams` rule styles (tabular numbers, paper on brand at 4.61:1). No token, layout track, or component was added. The mock-fidelity inventory items are unchanged, and the viewer slices that own them verified them. The contract-check pass was not dispatched, because no inventory item was touched.

## Deviations from Plan

1. **Header copy aligned to the ratified scenario** (plan Step 5.1). The page showed `engine 0.1.0`, and the scenario checkpoint names "Engine connected" with the version. The span now reads `Engine connected · v0.1.0` and follows the socket: reconnecting, stopped, finished, or replay. This is a minimal copy change in an existing element. It adds nothing to scope beyond what the product owner ratified in the shape's scenario. Class: implementation-detail.
2. **`serve` writes `stats.json` at full time** (plan Current State said "the engine writes the first two today"). Only `simulate`, `resume`, and the calibration worker called `write_stats`: `grep -rn write_stats crates` showed no call in `serve.rs`. The first whole-match run failed at step 11 on the missing file (`e2e-scenario-run2.txt`). The fix implements the observability contract's existing `match-stats` record, emitted once at full time, with no change to the record shape. It is a normal fix in the owning code with a regression assertion (`stream_cli.rs:130` failed before the fix with `NotFound`). Class: implementation-detail.
3. **Step 8 "score state"** read as the commentary slice's definition. The shape's checkpoint says the feed and commentary lines name "the scorer, the minute, and the score state". The shipped commentary names the score state as the match situation, for example "The first goal goes to …", and not always as digits. The spec asserts the scorer, the minute, that the row carries the engine's commentary line, and that the score bug changed to the goal's score in the goal frame. The first run had asserted the literal `1-0` and failed on a correct line. Class: implementation-detail. This matches the commentary slice's definition, as the selector of the line (AC-13).
4. **The spec file name.** The plan named it `charter-scenario.spec.mjs`. The output boundary forbids workflow words in shipped files, so the file is `e2e/tests/first-match.spec.mjs`. Class: implementation-detail.
5. **Nine subcommands, not seven.** `launch` and `calibrate` came after the plan's count. `cli.md` and `docs.rs` cover all nine (`engine-cli --help`). Class: implementation-detail.
6. **Browser install not needed.** `npx playwright install chromium` was skipped, because `playwright-core/browsers.json` in 1.63.0 names chromium-1243, and that build was already in `%LOCALAPPDATA%\ms-playwright`. Class: implementation-detail.
7. **In-app browser drive not run in this run** (plan Step 14 first half). The Playwright whole-match spec attaches one full-page screenshot for each step. The in-app pane drive with `step-NN.png` under `verify-evidence/integration/` is the plan's verify-time method (Verification Strategy row 1). It is left to verify, and was not duplicated here. Class: implementation-detail.
8. **Timing tool.** The 1000-match wall time was measured with the Git Bash clock around `bench --matches 1000` (`thousand.wall.txt`), not with PowerShell `Measure-Command`. It is the same measurement from outside the process. Class: implementation-detail.
9. **Tests added beyond the plan list.** The additions are an interpolation-bounds test, a damaged-snapshot restart test, and README-command coverage in `docs.rs`, because three viewer criteria were not in the plan's spec list. The memory budget, the statistics-panel fields, and feed batching are mapped to existing page unit tests in `e2e/README.md`. Class: implementation-detail.

## Assumptions

- **A-I1** (class: implementation-detail, ac: "the Playwright suite covers every viewer interactive criterion", classification: build-capability): A criterion that a page unit test already asserts exactly (memory bytes, statistics fields equal to the stream, feed burst at 8x) counts as "a passing test". Each is named in `e2e/README.md`.
- **A-I2** (class: implementation-detail, ac: "records appear in the sink within the stated latency", classification: runtime-evidence): The tolerance is 2 s from the page receiving full time, as in plan A9. The run observed it on the release build (`observability.spec.mjs`, passed in `e2e-full-run1.txt`).
- **A-I3** (class: implementation-detail, ac: "benchmark under 2 s and 1000 matches under 30 min", classification: runtime-evidence): `bench --matches 1000` includes one warm-up match. The measured 423.5 s therefore covers 1001 matches, which is conservative.
- **A-I4** (class: implementation-detail): `serve` writes the statistics record at full time only. A served match that the viewer abandons writes no statistics record. This is recorded under Anything Deferred and not built, because the product owner defined the abandonment outcome value nowhere, and the schema admits only `success` and `failure`.
- **A-I5** (class: implementation-detail): Scenario step 12 treats the trailing club as the AI manager's club in `simulate`, where the AI manages both clubs, and requires an applied `tactics-change` at minute 70 or later while that club trails. Seed 1 qualifies: the home club trails 1-2 at minute 70 and applies a tactics change (checked again outside the suite in this run).

## Triage Decisions

- The header copy fix and the `serve` statistics fix are in this slice's scope ("tests, documentation, and fixes"). Neither changes a protocol message, a record field, or a file format, so plan A16's stop condition does not apply.
- The three failures of the first viewer run were test defects, not product defects, and were fixed in the specs:
  - The focus test counted the non-focusable `lineup-reason` and the hidden role selects. It now uses `checkVisibility()` on focusable elements.
  - The 1x test ran into the half-time pause of a 10-minute fixture. The fixture is now 14 minutes.
  - The crash test read the clock one frame after the resume at 8x. It now pauses before the restart.

## Anything Deferred

- **Statistics record for an abandoned served match.** Ceiling: a served match that the viewer leaves before full time writes event rows but no `stats.json`. The observability contract's `darkpath.match_without_stats` would count it. Upgrade path: write the record with `outcome: "failure"` on the viewer-gone path of `serve.rs`, after the product owner says whether an abandoned match is a failure.
- **Three human checks** (e2e/README.md): legibility and focus at 1280 by 800, the goal-moment motion, and the tutorial followed from a new clone. No human took part in this run. The viewer-match-day legibility deferral (`needed-by: integration`) stays open until the product owner records the reading in po-answers.md.
- **Viewer-session record.** This is unchanged from plan Risks: no slice owns it.

## Known Risks / Caveats

- The whole-match spec depends on a goal in a 90-minute match with the seed-2026 league and seed 42. Every run of this spec so far reached a goal. The first run's failure output shows a goal by Raton Garberg at minute 22. The manager's changes alter the match, so a future engine change could produce a goalless match. In that case, step 8 fails with a clear message and does not pass silently.
- The frame-rate test needs a 60 Hz `requestAnimationFrame` in headless Chromium. It passed on this machine, and a slower CI host could fail it.
- I started one probe early in this run and stopped it with `taskkill /IM engine-cli.exe`, which ends every `engine-cli` process on the machine. After that, every stop used the process tree of the test's own engine (`killTree`).

## Freshness Research

- `npm view @playwright/test version license` → `1.63.0`, `Apache-2.0` (this run). Installed tree: `@playwright/test`, `playwright`, and `playwright-core` are all 1.63.0 and Apache-2.0 (`e2e/node_modules/*/package.json`).
- The `msedge` channel is registered in the installed `e2e/node_modules/playwright-core/lib/coreBundle.js:6937`. `reducedMotion?: null|"reduce"|"no-preference"` is at `e2e/node_modules/playwright-core/types/types.d.ts:2812`. Both were read again in the installed 1.63.0, as the plan required.
- `e2e/node_modules/playwright-core/browsers.json` names chromium-1243, which was already cached.
- `cargo metadata --offline` supplies every package's `license`. The audit read more than 100 packages and found none that fails.

## Benchmark

| Run | Median wall ms | Processor ms | µs per tick | Peak MB | Budget |
|---|---|---|---|---|---|
| before-1 / 2 / 3 | 418 / 418 / 420 | 2094 / 2079 / 2079 | 1.482 / 1.4713 / 1.4713 | 6.44 / 6.51 / 6.45 | pass |
| after-1 / 2 / 3 | 420 / 421 / 420 | 2094 / 2079 / 2110 | 1.482 / 1.4713 / 1.4933 | 6.45 / 6.46 / 6.54 | pass |

The processor time rises by at most 1.5 percent (tripwire 10 percent), and peak memory by at most 1.6 percent (tripwire 25 percent). 1000 matches plus one warm-up take 423.5 s against 30 minutes. The median is 421 ms, and the budget passes.

## Checks Run

- `cargo fmt --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: exit 0. `cargo test --workspace`: 310 passed, 0 failed, 4 ignored. `node --test "web/tests/*.test.mjs"`: 126 passed, 0 failed.
- Browser suite, full run: 21 passed in 24.4 minutes (`implement-evidence/integration/e2e-full-run1.txt`, HTML report in `implement-evidence/integration/playwright-report/`).
- The workflow-vocabulary search found no match in `e2e/`, `docs/`, `README.md`, `content/README.md`, the new tests, `serve.rs`, or `web/main.mjs`. The only hits are the JavaScript method `slice`.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine integration`. The slice has runtime criteria: the whole match in the in-app browser with screenshots for each step, the suite, the benchmark, and the sink. Verify also holds the three human checks. Consider compacting the session before verify. The workflow state lives in the artifact files on disk.
- **Option B:** `/wf review football-manager-match-engine integration`. Use it only if verify's evidence is accepted from this run's suite output. It is not recommended, because the in-app drive and the human checks belong to verify.
