---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: viewer-match-day
status: complete
stage-number: 6
created-at: "2026-09-23T09:49:45Z"
updated-at: "2026-09-23T09:49:45Z"
result: partial
metric-checks-run: 12
metric-checks-passed: 12
metric-acceptance-met: 6
metric-acceptance-total: 7
metric-acceptance-user-observable: 5
metric-acceptance-code-only: 2
metric-interactive-checks-run: 5
metric-interactive-checks-passed: 4
metric-issues-found: 0
metric-issues-found-initial: 0
metric-issues-found-final: 0
fix-rounds-run: 0
convergence: not-needed
verify-owned-fix-commit: null
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: deferred
interactive-verification-defer-reason: "AC-7 human legibility reading only. Rungs tried: headless Microsoft Edge 153 at 1280 by 800 and 60 Hz over the DevTools protocol read the computed contrast of 201 visible text and number nodes (minimum 4.61:1, none under 4.5:1), confirmed no page scroll (scrollWidth 1280, scrollHeight 800), and tabbed through every control (8 controls plus the feed list, each :focus-visible with a 2px solid ring; screenshots ac7-focus-*.png). The pre-registered rung web-5 is a person reading the screen on the reference laptop; this run has no human operator. Probe: `grep -ciE \"legibility|read the match screen\" po-answers.md` -> `0` (no recorded reading), and the integration slice that hosts the reading is `status: defined`. Residual: the human judgement that the clock, score, and every statistics value read without zooming on the reference laptop."
interactive-verification-wall-ownership: external
adapters-used: [web, cli]
bootstrap-failures:
  - {adapter: web, step: "in-app browser pane drive", remediation: "Not used. The pane throttles animation frames while hidden (viewer-pitch verify and this slice's implement run). Every drive ran in headless Microsoft Edge 153 over the DevTools protocol, the harness at verify-evidence/viewer-match-day/cdp.mjs."}
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/viewer-match-day/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: pass
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped — the page is served unbundled; no bundler exists"
ac-staleness-checked: true
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 0
adversarial-tests-run: 3
adversarial-tests-failed: 0
failure-mode-probes-run: 1
cross-browser-delta: "none"
web-vitals-lcp-ms: null
web-vitals-cls: null
web-vitals-inp-ms: null
stack-source: confirmed
debt-markers-found: 0
debt-markers-malformed: 0
debt-markers-unrecorded: 0
skipped-gating-specs: []
consult-runs: []
tags: [viewer, panels, feed, statistics, goal-moment, headless-edge, deferral]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-viewer-match-day.md
  plan: 04-plan-viewer-match-day.md
  implement: 05-implement-viewer-match-day.md
  review: 07-review-viewer-match-day.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine viewer-match-day"
---

# Verify: Match-Day Panels and Goal Moments

## The Verification

Implement left the match-day screen built on commits `9e684e9` and `4d21041`, with 278 Rust tests and 80 page tests green. It also left one open gap: the browser pane was hidden in that run, so the goal moment during play, the fatigue change, reduced motion, and the frame budget had no runtime evidence. This run drove all five user-observable criteria in headless Microsoft Edge at 1280 by 800 and 60 Hz, against the version-3 fixture (seed 7, goals at ticks 103516, 119176, 200722).

Both goals reached the screen in the frame that first rendered their tick (`frame_delta` 0, rendered ticks 103519 and 119180). The score, banner, highlighted feed row, and commentary line were all in place in that same task. Reduced motion was driven twice, once with the browser's own `prefers-reduced-motion: reduce` emulation and once with `?motion=reduce`. Both runs showed the banner with 1e-05 s transitions and no score-bug pulse. So the plan's pre-registered reduced-motion deferral was not needed. Nine fatigue-band changes (Fresh to Tiring) moved the bar colour and the word together, and bar widths fell between condition messages. All 12 checks pass. Five minutes at 1x with every panel live held 60 fps with 0 dropped frames, and p95 was 16.69 to 16.84 ms against the restated 18.67 ms budget.

Six of seven criteria are met. AC-7 has its measurable half met: every text node is at or above 4.61:1, the page does not scroll, and all controls show focus rings. The human reading on the reference laptop is deferred, so the result is `partial`. Review and handoff can proceed. Ship stays blocked until the product owner's reading during the integration charter run clears the deferral. The top open risk is small: the kick-off empty state is on screen only until the first tick. In replay the kick-off event is at tick 1, the first stored tick, so a viewer sees the empty feed for less than one frame. The planned pre-match lineup hold is what makes that state last.

## Verification Summary

- Slice `viewer-match-day`, standard mode, branch `feat/football-manager-match-engine` at `53dae38` (the slice code is `9e684e9` and `4d21041`; `53dae38` is its build notes).
- Stack confirmed (`stack.user-confirmed: true`, plan `stack-source: confirmed`). Every user-observable criterion with an environment dependency carries a `constraint-resolution:` line in the plan, so `constraint-resolution-missing` is empty.
- Checks: 12 run, 12 pass. Criteria: 7, of which 5 are user-observable and 2 code-only. 6 met, 1 partially met with a lawful deferral (AC-7 human reading).
- No failing check and no unmet criterion entered the fix loop: `metric-issues-found-initial` 0, `fix-rounds-run` 0, `convergence: not-needed`.
- Result `partial` because of the AC-7 deferral, and for no code reason.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass, exit 0 (`fmt.txt`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass, exit 0 (`clippy.txt`).
- `cargo test --workspace --no-fail-fast`: pass, 278 passed, 0 failed, 4 ignored (`cargo-test.txt`). This includes `the_hello_roster_lists_the_starters_in_slot_order_then_the_bench`, `statistics_and_energy_are_sent_every_simulated_second_and_statistics_at_full_time`, `a_condition_message_is_tagged_condition`, and `a_hello_without_a_roster_reads_as_an_empty_one`.
- `node --test web/tests/*.test.mjs`: pass, 80 tests, 80 pass, 0 fail, 0 skipped (`node-test.txt`).
- `cargo build --release -p engine-cli`: pass (`build-release.txt`).
- Security pattern scan over the slice diff (`git diff 9e684e9^ 53dae38 -- crates web docs`): pass. No secret patterns. The one `innerHTML` (`web/lineups.mjs:129`) is a static template with no data; every player value is written through `textContent`.
- Benchmark, processor time (`engine-cli bench --seed 42 --matches 5 --json`): pass. 412.6 ms per match (2,063 ms / 5) against the 460.7 ms gate; peak memory 6.45 MB against 6.82 MB; 1.46 µs per tick (`bench-cpu.stdout.txt`).
- Benchmark, stream (`engine-cli bench --seed 42 --matches 1 --stream --json`): pass. 605,475 ticks per second delivered (baseline 609,220, -0.6 percent); peak 7.62 MB against the 8.55 MB stream tripwire; 1 pause (`bench-stream.stdout.txt`).
- Page frame budget, 5 minutes at 1x with every panel live (`bench1x.mjs`, 60 five-second windows): pass. `fps_median` 60 in every window at `refresh_hz` 60; 0 dropped frames; p95 16.69 to 16.84 ms (median 16.71) against the restated budget of 1000 / `refresh_hz` + 2 = 18.67 ms; no `viewer.tick_skipped` at 1x; `page_bytes` 37,498,128 against 314,572,800 (`bench1x.json`).
- Instrument check, `viewer.goal_moment`: pass. It fired for both goals with `goal_tick`, `rendered_tick`, `prev_rendered_tick`, `frame_delta`, `home.score`, and `away.score` (`drive-main.json` step `signals`).
- Mock fidelity re-check (`02c-craft.md` items 6 to 9 owned or reused here): pass. See Augmentation Verification.
- Contrast and focus gate at 1280 by 800: pass. 201 nodes, minimum 4.61:1, none under 4.5:1; every control has a `:focus-visible` 2px solid ring (`drive-main.json` steps `ac7-contrast`, `ac7-focus`).

## Interactive Verification Results

Platform `web`. Tool: headless Microsoft Edge 153 (`HeadlessChrome/153.0.0.0 ... Edg/153.0.0.0`) driven over the DevTools protocol by `verify-evidence/viewer-match-day/drive.mjs`, `ac2.mjs`, and `bench1x.mjs` on the harness `cdp.mjs`. Clicks and key presses are `Input.dispatch*` events, which the page receives as trusted input. Server: `target/release/engine-cli.exe replay --fixture fixture.smfx --speed 8 --web web` (1x for AC-2 and the frame budget). Fixture: protocol version 3, seed 7, 292,900 ticks, goals at 103516, 119176, 200722, kick-off event at tick 1 (`fixture-facts.txt`).

**AC-1 goal moment within one frame**
- Platform & tool: web, headless Edge.
- Steps performed: played at 8x from kick-off. A page-side hook took a DOM snapshot inside the same task that emits `viewer.goal_moment`, so in the same frame. The drive then paused and took a screenshot. This was done for both first-half goals.
- Evidence: `verify-evidence/viewer-match-day/ac-goal-1-main.png`, `ac-goal-2-main.png`, `drive-main.json` steps `goal-1` and `goal-2`.
- Observation: goal 1, `goal_tick` 103516, `rendered_tick` 103519, `prev_rendered_tick` 103513, `frame_delta` 0. In the same task the score read 1-0, the score bug's label read "Score 1 – 0", the banner was shown with "GOAL — Oakmere Rangers 1–0 34'", and the pulse was on. The feed held the GOAL row with its commentary line, "The first goal goes to Oakmere Rangers, and Idador Holton is the scorer!", highlighted. The restart row at the same tick followed it. Goal 2: 119176 rendered at 119180 (previous 119173), `frame_delta` 0, 2-0, banner "GOAL — Oakmere Rangers 2–0 39'". At 8x one frame is 6.67 ticks, so both goals show within one frame of their tick. The banner was gone 1.6 s after it appeared (`after-banner-1`: shown false, visible false, pulse off).
- Result: pass.

**AC-2 kick-off with no events**
- Platform & tool: web, headless Edge, with a DevTools breakpoint.
- Steps performed: the fixture's kick-off event is at tick 1, the first stored tick. No human-speed pause can land before it: the first read, 0.5 s after load, was already at tick 16 with one feed row (`drive-main.json` step `ac2-first-read`). Pausing at the first stored tick shows the kick-off row (`drive-ac2-first-stored-tick.json`). To read the state the criterion names, `ac2.mjs hold` held the page on a breakpoint at `web/main.mjs:151`. That is the first animation frame that holds a tick, before it is drawn and after the hello was applied. The drive read the DOM there with `Debugger.evaluateOnCallFrame`.
- Evidence: `drive-ac2-hold.json` step `ac2-held-after-hello-before-first-tick`; `ac2-first-stored-tick.png` for the next state.
- Observation: `renderedTick` 0, 0 feed rows, the empty-state paragraph visible (`hidden` false) with "No events yet. The feed fills as the match plays.". The header read 0-0 and 00:00 with both team names. Every statistics cell read 0, 0.0%, or 0.00, and all 22 lineup rows read Fresh. The screenshot could not be taken while the page's script was held; the capture timed out after 5 s. The DOM read is the evidence.
- Result: pass. The state lasts until the first tick; see Friction Notes.

**AC-3 fatigue bar and labelled condition marker**
- Platform & tool: web, headless Edge.
- Steps performed: during the 8x drive, read every lineup row's band, condition word, bar width, bar colour token, and accessible label every 100 ms from kick-off to the second goal.
- Evidence: `drive-main.json` steps `ac3-band-changes`, `ac3-width-changes`, `ac3-lineup-final`; `ac3-lineups.png`; `ac-goal-1-main.png`.
- Observation: 9 band changes. The first was Peren Palton at rendered tick 89372, next to the fixture's first energy below 0.70 at 89350. Each change moved `data-band`, the visible word (Fresh to Tiring), the bar token (`--tl-success` to `--tl-warning`), and the accessible label together. Bar widths changed between released condition messages (for example 100% to 99%), with 40 width changes recorded. The word is always present, so colour never carries the state alone.
- Result: pass.

**AC-6 reduced motion**
- Platform & tool: web, headless Edge. Two drives: `reduce-media` with `Emulation.setEmulatedMedia` `prefers-reduced-motion: reduce`, which is the browser's own media-query path, and `reduce-param` with `?motion=reduce`.
- Steps performed: played at 8x to the first goal and read the banner, pulse, and computed transition durations in the goal task and after a pause.
- Evidence: `ac-goal-1-reduce-media.png`, `ac-goal-1-reduce-param.png`, `drive-reduce-media.json`, `drive-reduce-param.json`. Comparison: `ac-goal-1-main.png` at full motion.
- Observation: media drive: `matchMedia('(prefers-reduced-motion: reduce)').matches` true, `data-motion` reduce, banner shown with its text, banner and score-bug `transition-duration` 1e-05s, pulse off. Parameter drive: the same readings, with the media query false. Full-motion comparison: banner 0.25s, score bug 0.3s, pulse on.
- Result: pass. The operating-system toggle itself was not flipped. The emulation drives the same media feature that the Windows setting feeds.

**AC-7 legibility at 1280 by 800 and focus rings**
- Platform & tool: web, headless Edge at 1280 by 800 (the proxy). Human reading on the reference laptop (deferred).
- Steps performed: after the first goal, with every panel filled, computed WCAG contrast for 201 visible nodes (every `.tl-num`, statistics headers and cells, the score bug, clock, condition words, names, feed text, controls). Measured the page scroll size. Pressed Tab 14 times and read the focused element's `:focus-visible` state and outline.
- Evidence: `ac7-1280x800.png`, `ac7-focus-play.png`, `ac7-focus-speed-8.png`, `ac7-focus-speed-8-zoom.png`, `ac7-focus-scrub.png`, `ac7-focus-feed.png`, `ac7-focus-feed-zoom.png`, `drive-main.json`.
- Observation: minimum 4.61:1 (header clock, speed, team names, and the speed buttons: paper on brand at 14px); statistics values 16:1 class. None under 4.5:1. `scrollWidth` 1280 and `scrollHeight` 800. Play, Next stop, 1x, 2x, 4x, 8x, the scrubber, and the feed list each showed a 2px solid `oklch(0.55 0.17 250)` ring at a 2px offset, visible in the zoomed captures.
- Result: partial. The measurable half is met. The human reading is deferred, with the probe recorded in the frontmatter.

## Acceptance Criteria Status

| # | Criterion | Kind | Status | Method | Evidence | Evidence-rung |
|---|---|---|---|---|---|---|
| AC-1 | "Given a goal event in the stream, Then the score, the banner, the feed line, and the commentary line update within one frame of the goal tick." | user-observable | met | interactive | `ac-goal-1-main.png`, `ac-goal-2-main.png`, `drive-main.json` goal snapshots (`frame_delta` 0) | headless |
| AC-2 | "Given kick-off with no events, Then the feed shows the empty-state text from the design brief and the header reads 0 to 0." | user-observable | met | interactive | `drive-ac2-hold.json` (DOM at rendered tick 0 after the hello) | headless |
| AC-3 | "Given a player's fatigue field changes in the stream, Then that player's fatigue bar and condition marker update, and the marker carries a text label in addition to color." | user-observable | met | interactive | `drive-main.json` 9 band changes, 40 width changes; `ac3-lineups.png` | headless |
| AC-4 | "Given the statistics record fields in the stream, Then the panel shows every field and the values equal the stream values." | code-only | met | automated | `node-test.txt` (factory and captured-line tests), `cargo-test.txt` (per-second stats test) | n-a |
| AC-5 | "Given playback at 8x with more than 10 events per second, Then the feed inserts all events with none dropped, batched per frame." | code-only | met | automated | `node-test.txt` "a burst of 40 events inside one simulated second at 8x all appear, in order, none twice" | n-a |
| AC-6 | "Given `prefers-reduced-motion: reduce`, Then the goal banner appears without motion and the score-bug pulse is disabled." | user-observable | met | interactive | `drive-reduce-media.json`, `drive-reduce-param.json`, both screenshots | headless |
| AC-7 | "Given the screen at 1280 by 800, Then a reviewer reads the clock, score, and every statistics value without zooming, and focus rings are visible on every control." | user-observable | partially met (human reading deferred) | interactive (proxy) + deferred human check | `ac7-*.png`, `drive-main.json` contrast and focus | headless |

Evidence: headless 5 / n-a 2. `metric-acceptance-mock-rung`: 0.

AC-7 deferral: `interactive-verification: deferred`, `wall-ownership: external` (a person's judgement). Rungs tried: the headless contrast, scroll, and focus drive. Probe this run: `grep -ciE "legibility|read the match screen" po-answers.md` returned `0`; the integration slice is `status: defined`. Clearing event: the product owner reads the match screen on the reference laptop at 1280 by 800 during the integration slice's charter run and records the reading in `po-answers.md`. Registered in `00-index.md` `runtime-evidence-deferrals`.

## Issues Found

None. No check failed and no criterion was unmet for a code reason. The AC-7 deferral is recorded above and in the frontmatter, not as an issue.

## Augmentation Verification

**Mock fidelity (`02c-craft.md` section 3), items this slice owns or reuses:**
- 1 to 3, grid, pitch tile, control strip: honored. At 1280 by 800 the filled page does not scroll (`scrollWidth` 1280, `scrollHeight` 800). The pitch tile and the strip are unchanged in the screenshots.
- 6, typography: honored. `web/fonts/fonts.css` declares Barlow Condensed 700 and IBM Plex Sans from local `woff2` files, with no font service. The score bug uses `font: var(--tl-font-display)` with `tabular-nums` (`web/components/panels.css:46-47`). Every changing number carries `tabular-nums` in the contrast readout.
- 7, tokens: honored. `web/components/panels.css` holds no colour literal (hex, rgb, hsl, oklch, black, or white). The computed colours in the readout are all `--tl-` token values.
- 8, crests drawn by the mark function: honored. The crests are `<canvas>` (`crest-home`, `crest-away`) drawn in code, and no image file is referenced.
- 9, state words: honored at runtime. Fatigue shows Fresh or Tiring, cards show Yellow, and feed rows show GOAL, CARD, SUBSTITUTION, and INJURY beside the tinted rows (`ac-goal-1-main.png`).

**Instrument (`04b-instrument.md`):** `viewer.goal_moment` is a page signal this slice added. It fired at runtime with every designed field. `viewer.frame_budget` and `viewer.tick_skipped` still fire.

**Benchmark (`05c-benchmark.md` compare, page targets against `history/05c-benchmark-3.md`):**

| Target | Before (viewer-pitch) | After (this slice) | Verdict |
|---|---|---|---|
| viewer frame rate | 60 fps, 0 dropped, `refresh_hz` 60 | 60 fps in 60 of 60 windows, 0 dropped | pass |
| viewer frame time p95 | 16.69 to 16.85 ms | 16.69 to 16.84 ms (median 16.71) | pass against the restated budget 1000 / `refresh_hz` + 2 = 18.67 ms |
| viewer history bytes (`page_bytes`) | 31,924,655 at 270,000 ticks | 37,498,128 at 15,143 ticks (history is preallocated from `ticks_expected`; `history_bytes` 33,840,000) | pass, under 314,572,800 |
| viewer decode time | 0.083 µs per frame | not re-measured; this slice changes no decode code | n-a |
| processor time per match | gate 460.7 ms | 412.6 ms | pass |
| stream throughput / stream peak | 609,220 t/s / tripwire 8.55 MB | 605,475 t/s / 7.62 MB | pass |

The restated p95 budget and these readings are recorded here. `05c-benchmark.md` itself is not rewritten, because it carries uncommitted edits from another writer and belongs to the match-rules baseline.

**Experiment (`04c-experiment.md`):** deferred to `experiment-flags`, with no wiring in this slice.

## Security Scan

- CVE scan: `cargo audit` is not installed (`cargo audit --version` gives "no such command"). The slice adds no dependency: `Cargo.toml` and `Cargo.lock` are unchanged across `9e684e9^..53dae38`, and there is no `package.json`. No new critical or high findings are possible from dependencies.
- Secret detection: a pattern scan of the added lines found none.
- SAST: one `innerHTML`, a static template (`web/lineups.mjs:129`); data goes through `textContent`. No new HIGH finding.

## Accessibility Gate

- Tool: a computed-style contrast pass in headless Edge (WCAG relative luminance, colours resolved to sRGB through a canvas), plus a keyboard Tab walk.
- New WCAG AA violations: 0. The minimum text contrast is 4.61:1. All 8 controls and the focusable feed list show a visible focus ring. State words accompany every state colour.
- Not automated: no axe-core in the stack. The human legibility reading is deferred (AC-7).

## Performance Gate

- Bundle size: skipped. The page is served as unbundled modules, and there is no bundler.
- Frame budget with panels live: 60 fps, 0 dropped, p95 16.71 ms median (above).
- Engine: 412.6 ms per match (gate 460.7), stream 605,475 t/s, and stream peak 7.62 MB (tripwire 8.55).

## Cross-Slice Regression

Siblings checked: the whole workspace suite (278 Rust tests over engine-core, data-schemas-generator, stream-protocol, match-rules, tactics-and-ai, commentary, and calibration) and all 80 page tests, including viewer-pitch's playback, decode, and history tests. Regressions found: 0.

## Longitudinal Delta

- Page frame budget: baseline is the viewer-pitch verify run 2 (`history/05c-benchmark-3.md`). The p95 moved from 16.69-16.85 to 16.69-16.84 ms, with the same 0 dropped frames. Interpretation: expected. The panels flush once per frame and did not move the frame time.
- Screen: the viewer-pitch screenshots showed four placeholders. This run shows them filled (header score bug, lineups with bench, statistics tables, feed). Interpretation: the expected change.

## Friction Notes

- The kick-off empty state is shown only until the first tick. The fixture's kick-off event is at tick 1, the first stored tick, so a viewer sees "No events yet" only behind the loading skeleton and for less than one frame after the hello. Plan step 4 says the state returns after a rewind to tick 0, but no tick 0 is stored. A scrub to the left edge lands on tick 1 with the kick-off row. Informational. The pre-match lineup hold planned in `viewer-lineup-tactics` is what makes the state last on screen.
- Statistics trail the score by up to one simulated second, by design (per-second cadence). At the first goal the panel showed Oakmere Rangers with 1 shot and 0 on target next to a 1-0 score, until the next statistics message. Informational.
- A lineup name is cut with an ellipsis when a card chip sits beside it ("Visef…", "Idado…" in `ac-goal-1-main.png`). The accessible label carries the full name. Informational, and outside the AC-7 set (clock, score, statistics).

## Free Exploration Notes

- The goal and its restart share one tick (103516, 119176). The feed lists the GOAL row, then the restart row, in the same frame. This is correct ordering — informational.
- At 8x the page reports `viewer.tick_skipped` by design (6.67 ticks per frame). At 1x none was seen in 5 minutes — informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Empty submission | n-a | The page has no form. |
| Max-length input | n-a | No text input. The longest commentary lines wrap inside the 296 px feed without horizontal scroll. |
| Double-click / rapid repeat | pass | Pause and play were pressed around each goal in three drives; the banner and pulse timers cleared and restarted cleanly (`after-banner-1`). |
| Mid-flow interruption | pass | Pausing inside the goal frame kept the banner on its wall-time timer, and it left within 1.6 s. |
| Offline / network failure | pass | Unchanged from viewer-pitch: the socket close shows the error notice. The reload fault stays with `viewer-reports-recovery`. |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Slow response | n-a | This is a local socket. The `--sustain` lag path is unchanged from viewer-pitch. |
| Concurrent session | pass | Four headless pages ran against four separate replay processes at once, and each reached its goals at 60 fps. |
| Session expiry mid-flow | n-a | There are no sessions. |

## Cross-Browser Delta

Primary: headless Microsoft Edge 153 (Chromium). Secondary: none available. Playwright, Firefox, and WebKit are in `toolchains-absent`, so no divergence was measured. The field reads "none" in the sense of no findings, not a clean secondary run.

## Web Vitals

Not measured this run (the frontmatter holds null). The viewer-pitch run measured LCP 87 ms and CLS 0.0004 on the same shell. This slice adds no layout shift after load, because the panels fill fixed regions.

## Gaps / Unverified Areas

- AC-7 human reading on the reference laptop: deferred (see Acceptance Criteria Status).
- The Windows "Animation effects" toggle itself was not flipped. AC-6 was driven through the browser's own media-feature emulation and through the `?motion=reduce` parameter. The OS-to-browser hop belongs to Chromium, not this repository.
- The third goal (tick 200722) was not driven. The first two cover the criterion at 8x.
- No secondary browser was available.

## Freshness Research

- Chromium DevTools protocol, `Emulation.setEmulatedMedia` with `features: [{name: 'prefers-reduced-motion', value: 'reduce'}]`: supported in Edge 153. The drive read `matchMedia(...).matches === true` in the page, confirming the feature took effect.
- No dependency or schema changed outside this repository. The plan is from 2026-09-23 (under 14 days old), and no test failed, so no further freshness pass was needed.

## Recommendation

The slice meets six of seven criteria on runtime and test evidence, and the seventh on every measurable part. Proceed to review. Ship waits on the product owner's legibility reading.

## Recommended Next Stage

- **Option A: Review** — `/wf review football-manager-match-engine viewer-match-day`. Convergence is `not-needed`. The only residual is the deferred AC-7 human reading, which does not block review. Compact first; the drives produced long logs.
- **Option D: Skip review** — `/wf handoff football-manager-match-engine viewer-match-day`. Not recommended: the slice changes the wire protocol to version 3, and review should see that.
- **Option F: Clear the deferral** — the product owner reads the screen on the reference laptop during the integration charter run, then `/wf probe football-manager-match-engine` or a re-verify records it. Ship blocks until then.
- **Option G: Slug-wide runtime probe** — `/wf probe football-manager-match-engine` once the sibling viewer slices land.

## Assumptions and Decisions

- D-1 Drives ran in headless Edge over the DevTools protocol, not in the in-app browser pane. The pane throttles animation frames while hidden (viewer-pitch verify; implement run of this slice). The harness `cdp.mjs` is reused from viewer-pitch and kept in this slice's evidence folder, so the capability persists. `class: implementation-detail`.
- D-2 AC-6 is ruled met on the browser's own `prefers-reduced-motion` emulation plus the parameter proxy, instead of taking the plan's pre-registered deferral. The emulation exercises the real media query in the page, which the plan's proxy-only path could not. `class: implementation-detail`.
- D-3 AC-2 is ruled met on a DOM read taken while the page was held on a breakpoint after the hello and before the first tick was drawn. The fixture's first stored tick already carries the kick-off event, so no screenshot can show the state with the script running. `class: implementation-detail`.
- D-4 AC-7 is split. The measurable proxy (contrast, scroll, and focus) was driven and met. The pre-registered human reading is deferred with a probe, `wall-ownership: external`. `class: implementation-detail`.
- D-5 The restated p95 budget (1000 / `refresh_hz` + 2 ms) and the page readings are recorded in this artifact, not written into `05c-benchmark.md`, which carries another writer's uncommitted edits. `class: implementation-detail`.
- D-6 No automatic second opinion was run, although the `ac-deferred` trigger holds, because the product owner excluded `consult` (`stack.excluded-by-po`). `class: implementation-detail`.
- D-7 No commit: no verify-owned fix was needed. The evidence and artifacts stay on disk for the next stage. `class: implementation-detail`.
- D-8 The friction items (the brief kick-off empty state, statistics trailing the score by up to one second, and names cut beside card chips) are recorded as informational and not triaged as issues. None contradicts an acceptance criterion as written. `class: implementation-detail`.
- Intent-bearing escapes: 0.

## Verify-Owned Fixes

None. `fix-rounds-run` is 0: no check failed and no criterion was unmet.

Commit: (no files changed)
Regression tests added: 0
