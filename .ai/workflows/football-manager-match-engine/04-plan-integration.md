---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: integration
status: complete
stage-number: 4
created-at: "2026-09-22T22:21:00Z"
updated-at: "2026-09-22T22:21:00Z"
metric-files-to-touch: 21
metric-step-count: 16
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [integration, charter-scenario, playwright, docs, license-audit, benchmark, observability]
stack-source: confirmed
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-integration.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-viewer-lineup-tactics.md, 04-plan-viewer-match-day.md]
  implement: 05-implement-integration.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine integration"
---

# Plan: Integration and Charter Scenario

## The Plan

Every slice before this one verifies alone, and most viewer criteria verify against a recorded fixture, not the live engine. The repository already has most of what this slice needs. The engine serves the page itself (`engine-cli serve --web web`). It writes `stats.json` and `events.jsonl` to `SM_DATA_DIR/matches/<match.id>/` (`crates/engine/src/observe/mod.rs:257`, `crates/stream/src/events.rs:16`). The `bench` command reports the 2-second budget. Both license files and the dual SPDX expression are present (`Cargo.toml:10`). In this session, 192 Rust tests and 46 page tests pass, with 0 failures.

This plan adds 21 files in 16 steps, and it adds almost no product code. The browser suite lives in its own `e2e/` package with one pinned dependency, `@playwright/test` 1.63.0 (Apache-2.0). This keeps the page's own `node --test` suite free of dependencies. The suite starts a real engine per test with a temporary data folder, so one spec drives the page and reads the observability records. The license audit is a Rust test over `cargo metadata`, not a new tool: `cargo-deny` is not installed, and the check needs only SPDX expression parsing. The benchmark runs before and after the slice's work, so the tripwires compare like with like. Five documents are written to the targets the shape names, and two tests keep the command-line reference and the document set honest.

When this slice is complete, the first release has evidence for every criterion, and the deferred slices can start. The top open risk is a late performance miss after the tactics AI and commentary slices add work to every tick. NFR-1 yields to C2, so a miss stops this slice and goes back to the product owner. The tick model is never lowered silently.

## Current State

- **Dependencies.** `calibration`, `commentary`, `viewer-lineup-tactics`, and `viewer-reports-recovery` have status `defined` (`00-index.md` `slices:`). `viewer-lineup-tactics` and `viewer-match-day` have plans with status `awaiting-input` on contract questions: the squad, the lineup, and the tactics schema. This slice cannot start implementation until all four dependencies are verified.
- **Engine and page.** `engine-cli` has seven subcommands: `simulate`, `bench`, `generate`, `serve`, `record`, `replay`, and `resume` (`crates/engine-cli/src/cli.rs:30-46`). `serve --web <DIR>` prints the socket port and then the page address. The port is dynamic (`README.md`, Watch a match). `replay --sustain <speed>` caps the delivered rate for the lag-notice check. The page exposes the read-only hook `globalThis.__touchline` (`web/main.mjs:225`).
- **Observability sink.** The contract in `.ai/observability.md` is file-based: `SM_DATA_DIR/matches/<match.id>/events.jsonl`, `stats.json`, and `session.json`. Records are written at every stoppage snapshot and at match end (Block E). The engine writes the first two today. The slice risk "sink not built" does not apply.
- **Benchmark.** The last measured build ran a median of 388 to 390 ms per 90-minute match, on one thread, on the AMD Ryzen 7 9800X3D (`bench-baseline/match-rules/`). `bench` reports the median and the budget pass flag. It does not report the total run wall time (`crates/engine-cli/src/bench.rs:45,73`).
- **Documentation.** `README.md` and `docs/reference/protocol.md` exist. `docs/reference/protocol.md` is kept honest by `crates/protocol/tests/document.rs:10`. `content/README.md` documents every data-file field. `docs/tutorials/`, `docs/how-to/`, and `docs/explanation/` do not exist.
- **Licenses.** `cargo metadata` lists 139 packages. Every package carries a permissive expression. One package offers LGPL-2.1-or-later only as an OR alternative. No package is GPL-only or AGPL. The tally was run in this session.
- **Tooling.** Node 22.15.0 and npm 10.9.2 are present. `npm view @playwright/test` gives 1.63.0, Apache-2.0, Node 20 or later. Playwright browser builds are already cached in `%LOCALAPPDATA%\ms-playwright` (chromium-1200 to chromium-1243). Microsoft Edge is installed. `cargo deny` is not installed.
- **Tests run this session.** `cargo test --workspace --release`: 192 passed, 0 failed, 0 ignored. `node --test "web/tests/*.test.mjs"`: 46 passed, 0 failed.

## Simplicity Ladder

- Browser drive of the interactive criteria → rung 3 reuse plus a PO-authorised install. Playwright is the driver the product owner named for this point (shape Q29, "Playwright added to the repository once the viewer has stable markup"). The slice risk authorises the install. The page readers come from `window.__touchline` (rung 3, exact match).
- Engine launch per test → rung 1 stdlib: `node:child_process.spawn` of `target/release/engine-cli`. This is the same pattern that `crates/engine-cli/tests/cli_args.rs:9-14` uses in Rust (rung 3 pattern reuse). Playwright's `webServer` option is not used, because the port is dynamic and each test needs its own engine for one viewer per match.
- Replay download in the save-replay check → rung 2 native: Playwright `page.waitForEvent('download')`.
- Reduced-motion check → rung 2 native: `page.emulateMedia({ reducedMotion: 'reduce' })`.
- License audit → rung 3 plus rung 4. `cargo metadata` (in the toolchain) and `serde_json` (a workspace dependency) supply the data. The SPDX evaluation is about 60 lines of new code, because rung 1 has no SPDX parser and `cargo-deny` is a new tool outside `stack:`.
- 1000-match timing → rung 3 reuse: `engine-cli bench --matches 1000`, timed from outside with PowerShell `Measure-Command`. This avoids a new field in the run report.
- Command-line reference drift test → rung 3 reuse: the `--help` output of the binary, following `crates/protocol/tests/document.rs`.
- Documentation → no new capability. The `sdlc-workflow:diataxis` skill (`stack.available-skills` lists the workflow's skills) guides the type of each document.

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist in this repository (checked in this session).

Repeat-deferral tripwire: `00-index.md` has `runtime-evidence-deferrals: []`, so the tripwire does not fire.

## Likely Files / Areas to Touch

- `e2e/package.json` (new): the private package, `@playwright/test` pinned at 1.63.0.
- `e2e/package-lock.json` (new, generated): a fixed tree for the license audit.
- `e2e/playwright.config.mjs` (new): headless, 1280 by 800, one worker, Chromium with the `msedge` fallback.
- `e2e/support/engine.mjs` (new): engine launcher, league generator, and record reader.
- `e2e/tests/charter-scenario.spec.mjs` (new): the twelve scenario steps against the live engine.
- `e2e/tests/pitch.spec.mjs`, `match-day.spec.mjs`, `lineup-tactics.spec.mjs`, `reports-recovery.spec.mjs` (new): the interactive viewer criteria.
- `e2e/tests/observability.spec.mjs` (new): the records reach the data folder in time.
- `e2e/README.md` (new): how to run the suite, the map from each behaviour to its test, and the three human checks.
- `crates/engine-cli/tests/licenses.rs` (new): the license audit.
- `crates/engine-cli/tests/docs.rs` (new): checks that the document set exists and that the command-line reference covers every flag.
- `docs/tutorials/first-match.md`, `docs/how-to/modding.md`, `docs/reference/cli.md`, `docs/reference/data-files.md`, `docs/explanation/engine.md` (new).
- `content/README.md` (modified): now a pointer to the data-file reference.
- `README.md` (modified): documentation links, the browser suite, and the 1000-match benchmark.
- `.gitignore` (modified): the suite's output folders.
- Fixes found by the scenario run: unknown now. The files depend on what the scenario finds (Step 14).

## Proposed Change Strategy

Measure first, then add tests and documents, then fix what the tests find. The benchmark baseline is captured before any other change. After that, every later number compares against the complete engine as the dependencies left it.

Mechanism choice under NFR-1. The shape ranks NFR-1 as "`yields-to: C2` — a miss reopens the tick rate with the product owner; it never lowers the per-tick positional model silently." This plan does not use NFR-1 to justify any mechanism. It uses NFR-1 only as a stop condition: if the budget is missed, the slice stops and the tick-rate question goes to the product owner.

The browser suite binds to behaviour, not to structure. A spec finds a control by its accessible role and name, and it reads state through `window.__touchline`. It never uses a CSS class. The only exceptions are a `data-testid` that a viewer slice already added, and the steer.md rule that a state word must be present as text. This protects the suite against markup that three viewer plans have not finalised yet. For each viewer behaviour, the spec is written after the slice that owns that behaviour is verified. Every spec starts its own engine with a temporary `SM_DATA_DIR`. As a result, a spec can drive the page and then read the records the engine wrote, and no test depends on the developer's real data folder.

## Step-by-Step Plan

1. **Capture the benchmark baseline before any change.** Build with `cargo build --release`. Run `target/release/engine-cli bench --seed 42 --matches 5 --json` three times. Store the three run reports under the workflow's bench-baseline folder for this slice. These numbers are the complete engine as the four dependencies left it. The tripwires (10 percent processor, 25 percent memory) compare against them in Step 15.
2. **Create the browser-suite package.** Add `e2e/package.json` (`"private": true`, `"type": "module"`, devDependency `@playwright/test` pinned to `1.63.0`, and the scripts `test`, `test:scenario`, and `test:headed`). Run `npm install` in `e2e/` to write `e2e/package-lock.json`. Run `npx playwright install chromium` only if the cached build does not match 1.63.0. This is the install that the product owner authorised (shape Q29; slice risk "the plan authorizes the install"). Add `e2e/node_modules/`, `e2e/test-results/`, and `e2e/playwright-report/` to `.gitignore`.
3. **Write the configuration.** In `e2e/playwright.config.mjs`, set: `headless: true`; `viewport: { width: 1280, height: 800 }` (the design canvas size in steer.md); `workers: 1`; `fullyParallel: false`; and `use.channel` read from `PW_CHANNEL` (unset means the bundled Chromium, and `msedge` means the installed Edge). Set `screenshot: 'only-on-failure'` and `trace: 'retain-on-failure'`. Use two projects: `scenario` (the charter spec, `timeout` 45 minutes) and `viewer` (every other spec, `timeout` 5 minutes).
4. **Write the engine launcher.** In `e2e/support/engine.mjs`, write these functions:
   - `startEngine({ command, args })`. It spawns `target/release/engine-cli`. It sets `SM_DATA_DIR` to a new `fs.mkdtempSync` folder and `SM_CONTENT_DIR` to the repository `content/`. It reads the page address from the second stdout line (the format is in `README.md`, Watch a match), waiting up to 10 seconds. It returns `{ url, dataDir, matchId(), kill(), exited }`. If the binary is missing, it fails with the build command in the message.
   - `generateLeague(seed)`. It runs `engine-cli generate --seed <seed> --clubs 2 --out <tmp>` and returns the two team-file paths.
   - `readRecords(dataDir, matchId)`. It returns the parsed `stats.json` and the parsed lines of `events.jsonl`.
   The `matchId` is the page's own match identifier, read through `window.__touchline`.
5. **Write the charter-scenario spec.** In `e2e/tests/charter-scenario.spec.mjs`, write one `test.step` per scenario step, with one full-page screenshot per step attached to the report. The engine runs `serve --seed 42 --web web --team-a <A> --team-b <B>` with the league from `generateLeague(2026)`. Assert each step's checkpoint through the page:
   1. The header reads "Engine connected" with the version.
   2. An illegal lineup keeps kick-off disabled and shows the reason. A legal lineup enables kick-off.
   3. The tactics summary shows the chosen values.
   4. The clock advances, 22 markers and the ball move, and no marker is outside the pitch, read from the hook.
   5. At 4x the clock advances four times faster, or the notice names the sustained speed.
   6. The mentality chip reads Queued and then clears at the next dead ball, and the feed shows "Tactical change applied".
   7. The substitution applies at a dead ball, the lineup swaps, and the counter decrements.
   8. A goal updates the score, the banner, the feed, and the commentary line naming the scorer, minute, and score state.
   9. A rewind to the goal tick draws the stored positions.
   10. The half-time report statistics equal the event counts.
   11. The full-time report shows, "Save replay" downloads a file, and `stats.json` and `events.jsonl` exist in the data folder.
   12. Headless `engine-cli simulate` runs over seeds 1 to 50 until a run where the AI team trails after minute 70. In that run's `events.jsonl`, a `tactics-change` event from the AI manager appears before full time.
   After step 5, the spec raises the speed to 8x, so the rest of the match fits the timeout.
6. **Write the pitch spec.** In `e2e/tests/pitch.spec.mjs`:
   - At 1x, over five match minutes, the page's frame-time counter reports 60 frames per second and `darkpath.viewer_dropped_ticks` is 0.
   - At 4x, the clock advances four times faster than wall time, within 2 percent.
   - Against `replay --sustain 3 --speed 8`, a notice names 3x. Against `--sustain 8`, no notice shows. The fixture is recorded fresh by `engine-cli record --seed 7`.
   - After a rewind to a stored tick, the hook's rendered positions equal the stored positions.
7. **Write the match-day spec.** In `e2e/tests/match-day.spec.mjs`:
   - The goal moment updates within one frame of the goal tick, read from the hook's frame counter.
   - At kick-off with no events, the feed shows the empty-state text and the header reads 0 to 0.
   - A fatigue marker shows its state word.
   - With `page.emulateMedia({ reducedMotion: 'reduce' })`, the banner has no running animation (`element.getAnimations()` is empty) and the score-bug pulse is off.
8. **Write the lineup-and-tactics spec.** In `e2e/tests/lineup-tactics.spec.mjs`:
   - Each illegal lineup (ten starters, no goalkeeper, a player twice) keeps kick-off disabled and shows its reason.
   - The mentality chip goes from Queued to Applied, and the feed line appears.
   - A substitution applies, the players swap, and the count decrements.
   - A sixth substitution shows the engine's rejection reason verbatim.
   - A role change made while paused has not applied at resume, and it applies at the next dead ball. The spec compares the applied tick with the resume tick.
   - Using `Tab` and `Enter` only, every slot, control, and picker is reached, and each focused element has a non-`none` computed outline.
9. **Write the reports-and-recovery spec.** In `e2e/tests/reports-recovery.spec.mjs`:
   - `kill()` the engine in mid-match. The failure panel shows and offers a restart. After the restart, the score and the clock equal those at the last stoppage.
   - Close the socket from the page side without killing the engine. The page reconnects, and no restart prompt shows.
   - The half-time report equals the feed's event counts.
   - At full time, "Save replay" downloads a file. After loading the file with the engine killed, play and rewind work.
   - A wrong engine path shows the first-run state with the path and the instructions. The exact mechanism follows the reports-and-recovery plan.
10. **Write the observability spec.** In `e2e/tests/observability.spec.mjs`, drive a 10-minute match (`serve --seed 42 --minutes 10 --web web`) at 8x in the browser. Assert these checks:
    - `events.jsonl` exists and has grown by the first stoppage, which meets the contract's "written at every stoppage snapshot".
    - Within 2 seconds of the page receiving full time, `stats.json` parses with `record.kind` `match-stats`.
    - Every `events.jsonl` line parses with `record.kind` `match-event`.
    - `schema.version` equals the build's version.
    - `owner.id` and `match.id` equal the values the page holds.
    Read everything through `readRecords`. No transformation is applied (AC-17 wording).
11. **Write the license audit.** In `crates/engine-cli/tests/licenses.rs`, run `cargo metadata --format-version 1` (the `CARGO` environment variable names the binary) and read `e2e/package-lock.json` with `serde_json`. Parse each package's license expression. Accept `OR`, `AND`, parentheses, `WITH`, and the legacy `/` as OR. A package passes when at least one OR alternative contains no `GPL-*` or `AGPL-*` term. `LGPL-*` is allowed under NFR-5. A package with no license field fails. On failure, the assertion names every failing package and its expression. Also add unit cases for `GPL-3.0-only` (fails), `MIT OR GPL-2.0` (passes), `AGPL-3.0` (fails), `LGPL-2.1-or-later` (passes), and `MIT/Apache-2.0` (passes).
12. **Write the five documents.** Load the `sdlc-workflow:diataxis` skill first. Write only product language: no work-item names, stage names, or criterion numbers.
    - `docs/tutorials/first-match.md`: build, generate a league with seed 2026, serve two generated clubs with `--web web`, pick a lineup, kick off, make a substitution, and save the replay. Each step states what the reader sees.
    - `docs/how-to/modding.md`: change a tuning value, add a rule pack, edit team data and the attribute schema, and read a validation error.
    - `docs/reference/cli.md`: every subcommand, flag, default, environment variable, exit code, and output file.
    - `docs/reference/data-files.md`: move the full content of `content/README.md` here, and add the record, snapshot, and replay files. Reduce `content/README.md` to a pointer.
    - `docs/explanation/engine.md`: the agent model, the tick loop, the referee, the stoppage-gated queue, the snapshot, the seedable generator, and the best-effort determinism stance.
    - `README.md`: what the project is, how to run it, links to the four document types, how to run the browser suite, and the license. Engine internals move to the explanation.
13. **Write the documentation checks.** In `crates/engine-cli/tests/docs.rs`, check that each of the six documents in the set exists and is non-empty. Run `engine-cli <sub> --help` for each of the seven subcommands (`CARGO_BIN_EXE_engine-cli`) and parse every `--flag` token. Check that each flag appears in `docs/reference/cli.md` under that subcommand's heading. Check that every `engine-cli <sub>` in the tutorial and the how-to names a real subcommand.
14. **Run the whole scenario, then fix what it finds.** Run the charter scenario in the in-app browser first, with one screenshot per step. Then run `npm test` in `e2e/`. Fix any failure in the owning code as a normal change with a regression test beside it. A fix that changes a public contract (a protocol message, a record field, a file format, or user-visible behaviour a verified slice defined) is not made here: it stops the slice and goes to the product owner. Map each interactive viewer behaviour to its test title, or to one of the three human checks, in `e2e/README.md`.
15. **Rerun the benchmark on the complete engine.** Run `engine-cli bench --seed 42 --matches 5 --json` three times, and compare against Step 1 with the 10 percent processor and 25 percent memory tripwires. Run `Measure-Command { target/release/engine-cli bench --seed 42 --matches 1000 --json }` once and record the total wall time against 30 minutes. If the per-match median is 2000 ms or more, or the 1000-match total is 30 minutes or more, stop and route the tick-rate question to the product owner (NFR-1 yields-to C2).
16. **Final gates.** Run `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `node --test "web/tests/*.test.mjs"`, and `npm test` in `e2e/`. Search `e2e/`, `docs/`, `README.md`, and `crates/` for workflow vocabulary (slice names, stage names, and criterion numbers) before the commit.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| Charter scenario: all 12 steps with their checkpoints against the live engine | Claude_Browser drive with a screenshot per step (web-1), then the Playwright scenario spec (web-1, headless) | Windows 11 reference laptop, release build, Node 22; Playwright 1.63.0 through the authorised install — yes | `e2e/support/engine.mjs`; the league from seed 2026; `window.__touchline` readers added by the viewer slices; step 12's headless seed scan | Playwright headed project → Claude_Browser drive alone with screenshots → pre-registered deferral whose clearing event is the verification of the named dependency slice |
| Benchmark: one match under 2 s on one thread; 1000 matches under 30 min | `engine-cli bench` (cli-direct) plus `Measure-Command` | reference laptop — yes | Step 1 baseline; no code | none needed. A miss is a STOP to the product owner, not a deferral |
| Records reach the observability sink within the stated latency | Playwright observability spec reading `SM_DATA_DIR` (infra-1) | the engine's file sink — built (`crates/engine/src/observe/mod.rs:257`, `crates/stream/src/events.rs:16`) — yes | `readRecords`; a temporary `SM_DATA_DIR` per test | Claude_Browser drive plus a manual read of the match folder → none further needed |
| Documentation set exists and the tutorial runs as written | `docs.rs` (automated proxy: presence and command reference) plus the charter spec, which performs the tutorial's steps; the operator walkthrough (web-5, human) | a human at the reference laptop with a fresh clone — not automatable | `docs.rs`; the tutorial written against the same commands the charter spec runs | proxy (`docs.rs` plus the charter spec) → pre-registered human check |
| Playwright suite: every viewer interactive criterion has a passing test or a pre-registered human check (observable: false) | `npm test` in `e2e/` plus the map in `e2e/README.md` | as the charter row | the four viewer specs and the map | none needed |
| No dependency carries a GPL or AGPL license (observable: false) | `cargo test --test licenses` | yes | `licenses.rs` | none needed |

Constraint resolutions:
- Charter scenario: `constraint-resolution: prerequisite-slice: calibration, commentary, viewer-lineup-tactics, viewer-reports-recovery`. These are this slice's declared dependencies (`03-slice-integration.md` `depends-on`). Each is ordered before this slice in the implementation order, and this slice does not start implementation until all four are verified. The Playwright install is scoped into Step 2 under the product owner's authorisation (shape Q29). `wall-ownership: code-owned`.
- Benchmark: no environment wall.
- Observability sink: no environment wall. The sink exists in the repository.
- Tutorial walkthrough: `constraint-resolution: proxy+deferral: an operator clones the repository into a new folder on the reference laptop, follows docs/tutorials/first-match.md from the top, and records the result against the human check in e2e/README.md`. The proxy that verify can hold now is `docs.rs` plus the charter spec, which runs the same commands and steps. `wall-ownership: external` (the criterion is a person following the document; the slice pre-registers it as a human check at rung web-5).

Pre-registered human checks (the complete list the suite map refers to):
1. Legibility and focus at 1280 by 800 in a lit room. A reviewer reads the clock, the score, and every statistics value without zooming, and sees a focus ring on every control.
2. The goal-moment treatment matches the design contract (`02c-craft.md` Motion and the north-star mock).
3. The first-match tutorial is followed from a fresh clone.

## Test / Verification Plan

### Automated checks

- `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace`. These include `licenses.rs` and `docs.rs`, and the baseline in this session is 192 passing.
- `node --test "web/tests/*.test.mjs"`. The baseline in this session is 46 passing. Use the glob form, because the directory form fails on this machine (viewer-lineup-tactics plan, A6).
- `npm test` in `e2e/`, headless: the `viewer` project and the `scenario` project.
- The benchmark pair (Steps 1 and 15), with the report stored beside the workflow's bench baselines.
- The workflow-vocabulary search before the commit.

### Interactive verification (human-in-the-loop)

Platform: web (`stack.platforms: [web, cli]`). Tools: the Claude_Browser in-app browser pane (`stack.available-mcp`, the product owner's Q29 driver), then Playwright 1.63.0 in `e2e/` (authorised by Q29 for this point). Companion skills: `frontend-design` and `dataviz` are not needed, and `sdlc-workflow:diataxis` is used for the documents.

Run commands:
1. `cargo build --release`
2. `target/release/engine-cli generate --seed 2026 --clubs 2 --out %TEMP%\league-2026`
3. `target/release/engine-cli serve --seed 42 --web web --team-a <first file> --team-b <second file>`
4. Open the printed page address in the in-app browser, sized to 1280 by 800.

Drive the twelve steps in order, with one screenshot per step and a `window.__touchline` read at each checkpoint. After full time, list `SM_DATA_DIR/matches/<match.id>/` and read the first and last lines of `events.jsonl` and the whole `stats.json`. Evidence goes under `verify-evidence/integration/`: step screenshots named `step-NN.png`, hook reads, the records listing, the Playwright HTML report, and the benchmark reports. Pass criteria: every step's checkpoint holds as written in the shape's charter scenario, and the records parse as the observability contract's `match-event` and `match-stats`.

## Risks / Watchouts

- **Late performance miss.** The tactics AI and commentary add work per tick after the last measurement of 390 ms. A miss stops the slice and goes to the product owner (NFR-1 yields-to C2).
- **Markup not final.** Three viewer plans are awaiting contract answers. Specs locate controls by role and name and read state through the hook. Each spec is written after its slice is verified.
- **Headless frame pacing.** Headless Chromium may pace `requestAnimationFrame` differently from a real panel. The page counter is the assertion. The headed project and the in-app browser drive are the fallbacks.
- **Scenario duration.** A 90-minute match at 8x still lasts more than 11 minutes of wall time, plus the 4x stretch. The scenario project has a 45-minute timeout and runs alone.
- **Step 12 seed scan.** If none of seeds 1 to 50 has the AI team trailing after minute 70, the step fails and names the range. The scan is then widened. The spec does not fake a score.
- **Viewer session record.** `.ai/observability.md` (Block B, viewer unit) plans a `viewer-session` record from the page, but no slice definition owns it. It is outside this slice's criteria. It is recorded here so it is not lost.
- **Downloads during the suite.** Step 2 fetches npm packages and possibly one browser build. This is the install the product owner authorised. It does not run at test time.

## Dependencies on Other Slices

- `calibration`: the tuned engine and the run records. The benchmark measures the tuned engine.
- `commentary`: the commentary lines that steps 8 and the goal-moment spec read.
- `viewer-lineup-tactics`: scenario steps 2, 3, 6, and 7, and the lineup-and-tactics spec. Its plan has open contract questions. This slice binds to its behaviour, not to its contract.
- `viewer-reports-recovery`: scenario steps 10 and 11, and the reports-and-recovery spec.
- Indirect dependencies: `viewer-match-day` (score, banner, feed, lineups) and `tactics-and-ai` (the change queue drain, step 12's AI change).

## Assumptions

- **A1** (class: implementation-detail): The parallel plan-mode rule applies. This agent writes only its own three plan files. The master `04-plan.md`, `00-index.md`, `INDEX.md`, and the shared augmentation artifacts `04b-instrument.md` and `05c-benchmark.md` are left to the single writer. Why: several plan agents run at once (driver journal), and a second writer on a shared file risks a lost update.
- **A2** (class: implementation-detail): The benchmark augmentation for this slice is Steps 1 and 15, with evidence stored under the workflow's bench-baseline folder. `05c-benchmark.md` is not re-authored now. Why: the baseline must be the complete engine after the four dependencies land, and those dependencies are not implemented yet. A baseline taken today would measure the wrong engine.
- **A3** (class: implementation-detail): The instrument augmentation adds no new signal in this slice. Why: this slice checks that existing records arrive, and it emits nothing new.
- **A4** (class: implementation-detail): The browser suite lives in `e2e/` with its own `package.json`, not at the repository root. Why: the page's `node --test` suite stays free of dependencies, as `README.md` promises ("no package.json, no install"), and Playwright stays in one folder.
- **A5** (class: implementation-detail): `@playwright/test` is pinned to 1.63.0, the latest release (`npm view` in this session). The browser is the bundled Chromium, with `PW_CHANNEL=msedge` as the fallback to the installed Edge. Why: the bundled build is the one Playwright tests against, and browser builds are already cached on this machine.
- **A6** (class: implementation-detail): Each spec starts its own engine through `child_process` with a temporary `SM_DATA_DIR`, and Playwright's `webServer` option is not used. Why: the port is dynamic, the engine serves one viewer per match, and the observability check needs a known data folder.
- **A7** (class: implementation-detail, ac: "no dependency carries a GPL or AGPL license", classification: build-capability): The audit is a Rust test over `cargo metadata` and `e2e/package-lock.json`, not `cargo-deny`. LGPL is allowed, and an OR alternative that avoids GPL passes. Why: `cargo-deny` is not installed and is not in `stack:`, and NFR-5 allows LGPL ("dependencies permissive or LGPL").
- **A8** (class: implementation-detail, ac: "benchmark under 2 s and 1000 matches under 30 min", classification: runtime-evidence): The 1000-match wall time is measured with `Measure-Command` around `bench --matches 1000`. No total-time field is added to the run report. Why: this adds no change to the record shape of the observability contract.
- **A9** (class: implementation-detail, ac: "records appear in the sink within the stated latency", classification: runtime-evidence): The stated latency is read from `.ai/observability.md` Block E ("at every stoppage snapshot and at match end"). The check is two-part: `events.jsonl` has grown by the first stoppage, and `stats.json` exists within 2 seconds of full time. Why: the contract states no number, and 2 seconds is the tolerance of this check, not a new requirement.
- **A10** (class: implementation-detail): The data-file reference moves from `content/README.md` to `docs/reference/data-files.md`, and `content/README.md` becomes a pointer. Why: the shape targets `docs/reference/` for every schema, and one copy cannot drift.
- **A11** (class: implementation-detail): The command-line reference is kept honest by a test that parses `--help` output, as `crates/protocol/tests/document.rs` does for the protocol. Why: the shape requires "every flag". The test turns a missing flag into a failure.
- **A12** (class: implementation-detail, ac: "the tutorial's steps run as written", classification: runtime-evidence): The tutorial walkthrough stays the pre-registered human check that the slice names (rung web-5). Its automated proxy is `docs.rs` plus the charter spec. Why: the slice itself pre-registers it as a human check.
- **A13** (class: implementation-detail): Scenario step 12 scans seeds 1 to 50 for a run where the AI team trails after minute 70. Why: the step needs a real trailing state, and the engine's randomness makes a single seed unreliable.
- **A14** (class: implementation-detail): The license files and the SPDX expression already exist (`LICENSE-MIT`, `LICENSE-APACHE`, `Cargo.toml:10`, from plan Q10 "Dual MIT OR Apache-2.0"). This slice only checks them. Why: that decision is already recorded.
- **A15** (class: implementation-detail): The consult second opinion is not fired. The trigger `appetite-medium-or-larger` holds, but the product owner excluded `consult` at intake (`stack.excluded-by-po`).
- **A16** (class: implementation-detail): A fix found by the scenario that would change a public contract or a behaviour that a verified slice defined is not made in this slice. It is a stop condition for the product owner. Why: those are product decisions, and this slice's scope is tests, documents, and fixes.

## Blockers

None. The four dependency slices must be verified before implementation starts. That is ordering, not a blocker.

## Freshness Research

- `@playwright/test` latest is 1.63.0, license Apache-2.0, `engines.node >=20` (`npm view @playwright/test version license engines`, run in this session). `playwright-core` is Apache-2.0. Node 22.15.0 meets the engine range.
- Playwright supports the branded Edge channel and the reduced-motion emulation. Source read in this session (study-sources rung: installed package): the `msedge` channel is registered in `%LOCALAPPDATA%\npm-cache\_npx\9833c18b2d85bc59\node_modules\playwright-core\lib\server\registry\index.js`, and `reducedMotion?: null|"reduce"|"no-preference"` is declared in that package's `types/types.d.ts:2682`. That cached copy is 1.58.0-alpha. The implement step re-reads the same two points in the installed 1.63.0 package after Step 2. Edge is installed at `C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe` (checked in this session).
- `cargo metadata --format-version 1` gives each package's `license` string (observed in this session). Some older crates still use the legacy `/` separator: "MIT/Apache-2.0" (5 packages) and "Unlicense/MIT" (2 packages) in this session's tally. The parser treats `/` as OR, which is how those crates mean it (each lists two permissive licenses).
- `cargo-deny` would need `cargo install` and is outside `stack:` (`cargo deny --version` failed in this session). No built-in (rung 1 or 2) evaluates SPDX expressions.
- No other dependency changes. The workspace dependencies stay as `Cargo.toml` pins them.

## Recommended Next Stage

- **Option A (default, after the four dependency slices are verified):** `/wf implement football-manager-match-engine integration`. The plan is complete, and the first step captures the benchmark baseline on the complete engine.
- **Option B:** `/wf plan football-manager-match-engine viewer-lineup-tactics`. Complete the dependency plans that are awaiting product-owner answers first. This slice's scenario steps 2, 3, 6, and 7 depend on them.
- **Option C:** `/wf slice football-manager-match-engine`. Use it if the driver decides the unowned viewer-session record needs a slice.
