# Browser suite

These tests drive the match page in a real browser against the release engine. Each test starts its own `engine-cli` process with a new, temporary data folder, so a test can drive the page and then read the files the engine wrote. No test reads or changes your own data folder.

## Run the suite

1. Build the engine from the repository root:

   ```bash
   cargo build --release
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

To run the tests in the installed Microsoft Edge instead of the bundled Chromium, set `PW_CHANNEL=msedge`.

The HTML report is in `playwright-report/`. A failed test keeps its screenshot and trace in `test-results/`.

## How the tests find things

A test finds a control by its role and its accessible name, as a screen reader does, and reads the state of the page through the read-only hook `window.__touchline`. A test uses a `data-testid` only where a control has no unique accessible name. A test never uses a CSS class to find a control.

## Which test covers each behaviour

| Behaviour of the page | Test |
|---|---|
| The whole first match, lineup to full-time report, in twelve steps | `first-match.spec.mjs` › a manager plays a whole match, lineup to full-time report |
| At 1x, 60 frames per second for five match minutes with no skipped tick | `pitch.spec.mjs` › at 1x the page holds 60 frames per second for five match minutes and skips no tick |
| Each drawn frame lies between two stored ticks, never past the newest | `pitch.spec.mjs` › every drawn frame lies between two stored ticks, never past the newest; page test `interpolate.test.mjs` |
| At 8x the page skips ticks and never extrapolates | `pitch.spec.mjs` › every drawn frame lies between two stored ticks, never past the newest; page test `schedule.test.mjs` › at eight times speed ticks are skipped and counted |
| At 4x the clock runs four times faster, within 2 percent | `pitch.spec.mjs` › at 4x the clock runs four times faster than the wall clock |
| A stream that sustains 3x shows a notice naming 3x; one that sustains 8x shows none | `pitch.spec.mjs` › a stream sustained at 3x shows lag notice at 8x; a stream sustained at 8x shows no lag notice at 8x |
| A rewind draws the stored positions | `pitch.spec.mjs` › a rewind draws the stored positions of the tick |
| The stored history of a whole match stays under 300 MB | page test `history.test.mjs` › a whole match holds exactly 25,380,000 bytes |
| The goal moment shows in the frame that draws the goal | `match-day.spec.mjs` › the goal moment shows in the frame that draws the goal |
| Before play, the feed shows its empty text and the score is 0–0 | `match-day.spec.mjs` › before play the feed shows its empty text and the score is 0–0 |
| Each player's condition carries a word, not only a colour | `match-day.spec.mjs` › every player on the pitch carries a condition word |
| The statistics panel shows every field with the stream's value | page test `stats.test.mjs` › every field of a stats line captured from a recorded match appears with an equal value |
| At 8x the feed shows every event, none dropped | page test `feed.test.mjs` › a burst of 40 events inside one simulated second at 8x all appear, in order, none twice |
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

## Human checks

Three behaviours need a person. Record each result with the date, the machine, and the name of the person.

1. **Legibility and focus.** On the reference laptop at 1280 by 800, in a lit room, read the clock, the score, and every statistics value without zooming. Tab through the page and see a focus ring on every control.
2. **The goal moment.** Watch a goal at 1x. The banner shows over the pitch, holds, and leaves; the score bug pulses once; the goal row in the feed is highlighted. The motion is smooth and does not hide the pitch for longer than the banner holds.
3. **The first-match tutorial.** Clone the repository into a new folder, and follow `docs/tutorials/first-match.md` from the top. Each step does what the tutorial says.
