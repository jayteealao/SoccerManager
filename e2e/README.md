# Browser suite

These tests drive the match viewer in a real browser against the release engine. Each test starts its own `engine-cli` process with a new, temporary data folder and serves the built viewer (`viewer/dist`) with `--web`, so a test can drive the page and then read the files the engine wrote. No test reads or changes your own data folder.

## Run the suite

1. Build the engine and the viewer from the repository root:

   ```bash
   cargo build --release
   npm --prefix viewer ci
   npm --prefix viewer run build
   ```

2. Install the suite and its browser, once:

   ```bash
   cd e2e
   npm install
   npx playwright install chromium
   ```

3. Run the tests:

   ```bash
   npm test                 # every test, headless
   npm run test:viewer      # the viewer tests only
   npm run test:scenario    # the whole first match only, about 15 minutes
   npm run test:headed      # every test, in a visible window
   ```

The suites run two tests at a time; each test has its own engine, port and data folder. Set `SM_E2E_WORKERS` to change the count; CI and the two single-project scripts run one. A test tagged `@timing` measures frame rate or playback speed, so it runs alone, after the others. A run narrowed to some files or tests also runs every timing test when the count is above one, so set `SM_E2E_WORKERS=1` for a narrowed run. The shell suite's preview server uses port 4180; set `SM_SHELL_PORT` to move it.

To run the tests in the installed Microsoft Edge instead of the bundled Chromium, set `PW_CHANNEL=msedge`.

To test an installed game instead of the repository build, set `SM_E2E_INSTALL` to its folder: every test then runs that folder's `engine-cli` and serves its `web/`, the page the release carries.

The HTML report is in `playwright-report/`. A failed test keeps its screenshot and trace in `test-results/`.

## How the tests find things

A test finds a control by its role and its accessible name, as a screen reader does, and reads the state of the page through the read-only hook `window.__touchline`. The viewer shows one screen at a time (Tactics, the Pre-match line-ups, the match, the Touchline, the report, the replay) and keeps the match screen mounted behind the others, so a control is looked for on the screen on show; `support/page.mjs` opens the right one first. Where a control has no unique accessible name, a test finds it by the data attribute or the region that holds it.

## Which test covers each behaviour

| Behaviour of the page | Test |
|---|---|
| The whole first match, lineup to full-time report, in twelve steps | `first-match.spec.mjs` › a manager plays a whole match, lineup to full-time report |
| At 1x, 60 frames per second for five match minutes with no skipped tick | `pitch.spec.mjs` › at 1x the page holds 60 frames per second for five match minutes and skips no tick |
| Each drawn frame lies between two stored ticks, never past the newest | `pitch.spec.mjs` › every drawn frame lies between two stored ticks, never past the newest; viewer test `interpolate.test.js` |
| At 8x the page skips ticks and never extrapolates | `pitch.spec.mjs` › every drawn frame lies between two stored ticks, never past the newest; viewer test `schedule.test.js` › at eight times speed ticks are skipped and counted |
| At 4x the clock runs four times faster, within 2 percent | `pitch.spec.mjs` › at 4x the clock runs four times faster than the wall clock |
| A stream that sustains 3x shows a notice naming 3x; one that sustains 8x shows none | `pitch.spec.mjs` › a stream sustained at 3x shows lag notice at 8x; a stream sustained at 8x shows no lag notice at 8x |
| A rewind draws the stored positions | `pitch.spec.mjs` › a rewind draws the stored positions of the tick |
| The stored history of a whole match stays under 300 MB | viewer test `history.test.js` › a whole match holds exactly 25,380,000 bytes |
| The goal moment shows in the frame that draws the goal | `match-day.spec.mjs` › the goal moment shows in the frame that draws the goal |
| Before play, the feed shows its empty text and the score is 0–0 | `match-day.spec.mjs` › before play the feed shows its empty text and the score is 0–0 |
| Each player's condition carries a word, not only a colour | `match-day.spec.mjs` › every player on the pitch carries a condition word |
| The statistics panel shows every field with the stream's value | viewer test `stats.test.js` › every field of a stats line captured from a recorded match appears with an equal value |
| At 8x the feed shows every event, none dropped | viewer test `feed.test.js` › a burst of 40 events inside one simulated second at 8x all appear, in order, none twice |
| With reduced motion, the banner does not animate and the score does not pulse | `match-day.spec.mjs` › with reduced motion the goal banner does not animate and the score does not pulse |
| Each illegal lineup keeps kick-off disabled and names its reason | `lineup-tactics.spec.mjs` › each illegal lineup keeps kick-off disabled and names its reason |
| A mentality change is queued, applies at the next dead ball, and the feed says so | `lineup-tactics.spec.mjs` › changes during play wait for a stoppage, and a sixth substitution is refused |
| A substitution applies at a dead ball, the players swap, and the count drops | `lineup-tactics.spec.mjs` › changes during play wait for a stoppage, and a sixth substitution is refused |
| A substitution past the limit shows the engine's reason | `lineup-tactics.spec.mjs` › changes during play wait for a stoppage, and a sixth substitution is refused |
| A role change made while paused applies at the next stoppage, not at resume | `lineup-tactics.spec.mjs` › a role change made while paused applies at the next stoppage, not at resume |
| Every slot, control, and picker works from the keyboard, with a focus ring | `lineup-tactics.spec.mjs` › every slot, control, and picker is reached with Tab and shows a focus ring |
| A crashed engine shows the failure, and a restart resumes with the same score and clock | `reports-recovery.spec.mjs` › a crashed engine shows the failure and restarts from the last stoppage |
| A damaged snapshot is named, and only abandon is offered | `reports-recovery.spec.mjs` › a damaged snapshot is named on restart, and only abandon is offered |
| A dropped connection reconnects with no restart prompt | `reports-recovery.spec.mjs` › a dropped connection reconnects by itself, with no restart prompt |
| The half-time report equals the feed's counts | `reports-recovery.spec.mjs` › the half-time report counts equal the feed, and a saved replay plays with no engine |
| A saved replay plays and rewinds with no engine | `reports-recovery.spec.mjs` › the half-time report counts equal the feed, and a saved replay plays with no engine |
| A missing engine shows the path and how to build it | `reports-recovery.spec.mjs` › a missing engine shows the path it looked for and how to build it |
| The records of a match reach the data folder in time | `observability.spec.mjs` › the records of a browser-driven match reach the data folder in time |
| The splash waits for the engine and at least 1.5 s; a key or a click opens the start screen; reduced motion stops the reveal | `match/front-door.spec.mjs` › the splash tests (`npx playwright test -c match.config.mjs front-door`) |
| Start screen, match setup, settings across a relaunch, Return to start and Resume, Quit, and the licences against the notices file, on the real launcher | `match/front-door.spec.mjs` › the launcher drives |
| Every front-door screen matches its baseline in both skins | `match/front-door.spec.mjs` › the screenshot tests |
| The charter scenario, steps 1 to 5, from the start screen through match setup: the other grounds, a skip at minute 30, the report and the replay equal to the match played through | `match/charter.spec.mjs` › the charter scenario to the replay of a skipped match (`npx playwright test -c match.config.mjs charter`) |
| The charter scenario, step 6: a previous-release save finishes on the previous engine with that build's result | `match/charter.spec.mjs` › charter step 6: a previous-release save finishes on the previous engine (needs `SM_PREVIOUS_ENGINE_PATH`, or `SM_E2E_INSTALL` with `previous/`) |

## Human checks

Three behaviours need a person. Record each result with the date, the machine, and the name of the person.

1. **Legibility and focus.** On the reference laptop at 1280 by 800, in a lit room, read the clock, the score, and every statistics value without zooming. Tab through the page and see a focus ring on every control.
2. **The goal moment.** Watch a goal at 1x. The banner shows over the pitch, holds, and leaves; the score changes in the same frame; the goal row in the commentary is highlighted. The motion is smooth and does not hide the pitch for longer than the banner holds.
3. **The first-match tutorial.** Clone the repository into a new folder, and follow `docs/tutorials/first-match.md` from the top. Each step does what the tutorial says.
