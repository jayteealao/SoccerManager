---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: viewer-pitch
status: complete
stage-number: 6
created-at: "2026-09-22T18:24:27Z"
updated-at: "2026-09-22T18:49:11Z"
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-22T18:49:11Z"
    trigger: manual
    because: "second verify round after the first round escalated on AC-A"
    changed: "AC-A fixed and committed at c7c54bd; all five user-observable criteria re-driven on the final code; result partial to pass, convergence escalated to converged; the first-round fixes are recorded at 75a4ba6"
result: pass
metric-checks-run: 13
metric-checks-passed: 12
metric-acceptance-met: 8
metric-acceptance-total: 8
metric-acceptance-user-observable: 5
metric-acceptance-code-only: 3
metric-interactive-checks-run: 5
metric-interactive-checks-passed: 5
metric-issues-found: 1
metric-issues-found-initial: 2
metric-issues-found-final: 1
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: "c7c54bd"
regression-tests-added: 1
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [web, cli]
bootstrap-failures:
  - {adapter: web, step: "in-app browser pane drive", remediation: "The pane throttles requestAnimationFrame while hidden (32 Hz, then 0 frames); every drive runs in headless Microsoft Edge 153 over the DevTools protocol at 60 Hz."}
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/viewer-pitch/"
evidence-run-count: 2
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped — the base branch has no page; the page is 30 files served unbundled"
ac-staleness-checked: false
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 0
adversarial-tests-run: 3
adversarial-tests-failed: 1
failure-mode-probes-run: 2
cross-browser-delta: "none"
web-vitals-lcp-ms: 87
web-vitals-cls: 0.0004
web-vitals-inp-ms: null
stack-source: confirmed
debt-markers-found: 0
debt-markers-malformed: 0
debt-markers-unrecorded: 0
skipped-gating-specs: []
consult-runs: []
tags: [viewer, canvas, playback, milestone, headless-edge]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-viewer-pitch.md
  plan: 04-plan-viewer-pitch.md
  implement: 05-implement-viewer-pitch.md
  review: 07-review-viewer-pitch.md
  adapters: runtime-adapters.md
  prior: history/06-verify-viewer-pitch-0.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine viewer-pitch"
---

# Verify: Viewer Pitch and Playback

## The Verification

The implement stage left a page that draws the match and eight criteria with seams for each. It also left one open verdict: AC-a reached only 32 frames per second in the browser pane. That shortfall was the pane, which throttles animation frames while it is hidden. Every drive therefore runs in headless Microsoft Edge at 1280 by 800 and 60 Hz, with trusted pointer clicks over the DevTools protocol. Headless Edge also resolves the whole-page memory figure that the pane refuses: 31.9 MB for a full stored match, against a 300 MB budget.

Verification took two rounds. The first round found three defects that the implement drive could not see. Against the live engine, a click on 4x after the stream ended left playback at 1x. The play button announced "Pause, pressed" to a screen reader. Page loads intermittently skipped 1 to 6 ticks at 1x in their first second. The speed fix and the accessibility fix landed at `75a4ba6`. The first tick-skip patch cut normal fractional steps as well, so it was discarded. The second round applied the corrected cap: a speed the panel can show at one tick per frame never skips, and a stalled frame lands on the next tick instead. That fix landed at `c7c54bd`, with a regression test that fails on the old code.

On the final code, all eight criteria are met. All five user-observable criteria have headless runtime evidence, and each drive was repeated. The 5-minute run at 1x held 60 frames per second with no dropped frame and no skipped tick, and three more loads skipped nothing. Review can start now. The top risk moves to `viewer-reports-recovery`: a page reload ends the engine process with exit code 1, and the reloaded page never recovers.

## Verification Summary

- Branch `feat/football-manager-match-engine`, head `c7c54bd`. Stack confirmed: `web`, `cli`.
- 13 automated checks run, 12 pass. The failure is the frame-time p95 budget, which no 60 Hz panel can meet (issue BENCH-P95, Skip).
- 8 criteria: 5 user-observable, 3 code-only. 8 met.
- 5 user-observable criteria driven in headless Edge on the final code, 5 pass, 0 flaky.
- This round: 2 issues entered the fix loop (AC-A, BENCH-P95). AC-A is fixed and committed. BENCH-P95 stays Skip.
- Verify-owned commits: `75a4ba6` (round 1: AC-H5 and A11Y-PLAY) and `c7c54bd` (round 2: AC-A).

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (`verify-evidence/viewer-pitch/fmt.txt`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass, no warnings (`clippy.txt`).
- `cargo test --workspace`: pass, 132 passed, 0 failed (`cargo-test.txt`).
- `node --test "web/tests/*.test.mjs"`: pass, 42 of 42 before the round-2 fix (`node-test.txt`), 43 of 43 after it (`node-test-after-fix.txt`).
- `cargo build --release -p engine-cli`: pass (`build-release.txt`).
- Secret detection: `gitleaks` over `3570eb9..62a5dab` (1 false positive on prose at `04-plan-viewer-pitch.md:184`) and over `36139b4..75a4ba6` (no leaks); a grep of added lines found nothing.
- Dependency diff over the slice: pass, no crate or npm package added. `cargo audit` is not installed.
- SAST: semgrep is not installed. A manual read of `crates/engine-cli/src/web.rs` found no HIGH finding (see `## Security Scan`).
- `sdlc-debt:` markers in the slice and fix diffs: pass, 0.
- Benchmark compare, engine targets: pass, no tripwire (round 1 evidence in `verify-evidence/viewer-pitch-run-2/`).
- Benchmark compare, page targets: fail on `viewer frame time p95` only; 16.70 to 16.84 ms against a 16.6 ms budget. The frame rate, decode, and memory budgets pass.
- Instrument key check, `04b-instrument.md` section 2: pass, 8 of 8 page signals and `web.serving` observed; `fixture.replayed` with `sustain` not observed at runtime.
- Mock fidelity inventory, `02c-craft.md` section 3: pass, 10 of 10 honored, one wording deviation.

## Interactive Verification Results

Platform `web`, tool headless Microsoft Edge 153 driven by `verify-evidence/viewer-pitch/drive.mjs` and `cdp.mjs`. Clicks are `Input.dispatchMouseEvent`, which the page records as `isTrusted: true`. Servers: `target/release/engine-cli.exe replay --fixture fixture.smfx --speed 8 --web web` (seed 7 fixture) and `serve --seed 42 --web web` (live engine). Every result below is on commit `c7c54bd`.

**AC-a — 60 frames per second with zero dropped ticks at 1x for 5 minutes**
- Platform & tool: web, headless Edge at 60 Hz.
- Steps performed: opened the page against the fixture replay; sampled `window.__touchline.frame()`, the clock, the pressed speed, and a running sum of `viewer.tick_skipped` every 30 seconds for 300 seconds; screenshots at 0, 150, and 300 seconds. Then three separate page loads, each read after 6 seconds.
- Evidence: `drive-main.json` (step `ac-a`), `ac-a-t0.png`, `ac-a-t150.png`, `ac-a-final.png`, `drive-probe-after-fix-1.json` to `-3.json`. Before the fix: `drive-probe-before-fix*.json` (1 skipped tick in 4 loads) and `viewer-pitch-run-2/drive-main.json` (4 skipped ticks).
- Observation: the clock ran from 00:00 to 05:00, 15,010 ticks in 300 wall seconds. All 60 five-second windows read `fps_median` 60, `refresh_hz` 60, `dropped_frames` 0. No `viewer.tick_skipped` row fired. The three extra loads also read 0 skipped ticks and 0 dropped frames.
- Result: pass.

**AC-d — at 4x the clock advances four times faster than wall time within 2 percent**
- Steps performed: a click on `4x`; three windows of 20 wall seconds, each reading `lastRenderedTick()` and `performance.now()` at both ends.
- Evidence: `drive-main.json` (step `ac-d`), `ac-d-t0.png`, `ac-d-final.png`.
- Observation: 4.0018, 3.9985, and 3.9997 times wall time; the largest error is 0.05 percent.
- Result: pass, stable over 3 windows.

**AC-e — a sustained 3x drops playback to 3x with a notice; a sustained 8x shows no notice**
- Steps performed: `replay --speed 8 --sustain 3`, a click on `8x`, three reads 8 seconds apart, then a 10-second rate window. Then `replay --speed 8`, the same steps.
- Evidence: `drive-lag.json`, `ac-e-lag-t0.png`, `ac-e-lag-final.png`, `drive-nolag.json`, `ac-e-nolag-t0.png`, `ac-e-nolag-final.png`.
- Observation: capped run: the notice reads "Lag Playing at 3x — the engine sustains three times real time" in all three reads, the live region reads "Speed 8 times asked for. Playing at 3 times.", and `viewer.lag` carries `requested_speed` 8 and `sustained_speed` 3. Uncapped run: no notice in all three reads, no `viewer.lag` row, playback at 8x.
- Result: pass, stable over 3 reads per run.

**AC-f — a rewind to a stored tick draws exactly the stored positions**
- Steps performed: trusted clicks on the scrubber track, three in the fixture session and three on the fully stored live match (paused) at 20, 50, and 80 percent.
- Evidence: `drive-main.json` (step `ac-f`), `ac-f-25.png`; `drive-live.json` (step `ac-f-live`), `ac-f-live-20.png`, `ac-f-live-50.png`, `ac-f-live-80.png`.
- Observation: every rewind that landed on a stored tick reported `exact: true` over 47 components, and `tickAt(tick)` equalled `lastRendered()`: ticks 55,694; 41,435; 134,980; 228,565. The same ticks matched in both rounds. Two clicks drew no exact rewind, for stated reasons: one landed on the thumb and changed no value; one landed past the newest received tick, which the page reports as `stored: false` while it holds the newest tick.
- Result: pass, stable over 2 sessions and 2 rounds.

**AC-h — charter scenario steps 1, 4, and 5 against the live engine**
- Steps performed: step 1 read the header; step 4 read 40 frames 250 ms apart and checked every player against the pitch bounds; step 5 clicked `4x` after the stream had ended and measured two 15-second windows. Steps 2 and 3 use the default lineup, as the slice states.
- Evidence: `drive-live.json`, `ac-h-step1.png`, `ac-h-step4.png`, `ac-h-step5.png`. The round-1 failure is in `viewer-pitch-run-2/drive-live-prefix.json`.
- Observation: step 1 shows "Oakmere Rangers v Eldstead City" and "engine 0.1.0". Step 4: the clock advanced from 00:00 to 00:10, all 22 players and the ball moved, and no player left the pitch (largest |x| 49.65 m, largest |y| 27.00 m). Step 5: 4.004 times and about 4.00 times, with the effective readout at 4x.
- Result: pass.

## Acceptance Criteria Status

| Criterion | Kind | Status | Method | Evidence | Evidence-rung |
|---|---|---|---|---|---|
| AC-a: 5 minutes at 1x, 60 fps, zero dropped ticks | user-observable | met | interactive | `drive-main.json`, `ac-a-*.png`, `drive-probe-after-fix-*.json` | headless |
| AC-b: a marker lies on the segment between two ticks | code-only | met | automated | `node-test-after-fix.txt`: 4 tests in `interpolate.test.mjs` | n-a |
| AC-c: at 8x the viewer skips ticks and never passes the newest tick | code-only | met | automated | `node-test-after-fix.txt`: 6 tests in `schedule.test.mjs` | n-a |
| AC-d: 4x clock within 2 percent | user-observable | met | interactive | `drive-main.json` step `ac-d` | headless |
| AC-e: sustained 3x drops to 3x with a notice; 8x shows none | user-observable | met | interactive | `drive-lag.json`, `drive-nolag.json` | headless |
| AC-f: rewind draws exactly the stored tick | user-observable | met | interactive | `drive-main.json`, `drive-live.json` | headless |
| AC-g: a full match keeps history under 300 MB | code-only | met | automated | `history.test.mjs`: 25,380,000 bytes; page figure 31,927,291 bytes in `drive-live.json` | n-a |
| AC-h: charter scenario steps 1, 4, 5 against the live engine | user-observable | met | interactive | `drive-live.json`, `ac-h-step*.png` | headless |

Evidence: headless 5 / n-a 3. No user-observable criterion rests on a mock or on static reasoning. The fixture is a recording of the real engine, not an emulation of an external interface, so `mock-provenance` does not apply.

## Issues Found

- MED: BENCH-P95 — the `viewer frame time p95` budget of 16.6 ms is below one 60 Hz frame interval, so it cannot be met; measured 16.70 to 16.84 ms. Triage: Skip (round-1 decision, unchanged). `viewer-match-day` re-states the budget relative to `refresh_hz`.

Carried from round 1 and not re-examined this round (the round-2 checks did not touch them):
- MED: ADV-RELOAD — a page reload or tab close ends `replay` and `serve` with exit code 1 and the raw text "An established connection was aborted by the software in your host machine. (os error 10053)"; the reloaded page stays unstyled at "Waiting for the engine". At `crates/stream/src/replay.rs:95`. Triage: Escalate, routed to `viewer-reports-recovery`.
- LOW: SIGNAL-RING — `viewer.history` fires on every keyframe, so the 256-row ring on the test hook drops rarer signals within seconds. At `web/main.mjs:94`. Triage: Skip; `/wf observability audit` owns page sampling.
- LOW: CRAFT-TRAIL — the ball trail is stroked once per segment, while `02c-craft.md` callout 8 says "one path, never N separate strokes". At `web/pitch.mjs:234`. Triage: Skip; a falling alpha needs one stroke per segment.
- LOW: WEB-HARDEN — the page server has no socket read timeout, no Host header check, and no `nosniff` header. At `crates/engine-cli/src/web.rs:74`. Triage: Skip; loopback only.

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| AC-A | unmet-ac | Fix | Patched (round 2, corrected cap `position >= before + 2`) | `web/tests/schedule.test.mjs` "at one times speed a stalled frame delays playback instead of skipping" (fails before: `4 !== 0`) | Pass: 5 minutes at 1x with 0 skipped ticks and 0 dropped frames; 3 loads with 0 skipped ticks |
| BENCH-P95 | augmentation-regression | Skip | N/A | n-a | Not re-run |

Round 1 of this verify (snapshot at `history/06-verify-viewer-pitch-0.md`) fixed AC-H5 (a selected speed now applies at once, regression test `web/tests/playback.test.mjs`) and A11Y-PLAY (the play button dropped `aria-pressed`, test exempt because `main.mjs` has no DOM-free seam). The product owner asked for those two to be committed at `75a4ba6` while AC-A still failed. Round 1 discarded a wrong AC-A patch (`viewer-pitch-run-2/fix-ac-a-discarded.diff`). In round 2, the AC-A decision carried forward from round 1, because the method was already chosen and only the condition changed. The fix agent edited the working tree directly, because worktree isolation lost shell access in round 1.

Commit: `c7c54bd` (round 2). Round 1: `75a4ba6`.

Regression tests added: 1 this round (2 across both rounds).

## Augmentation Verification

**Mock fidelity inventory (`02c-craft.md` section 3).** All 10 items honored, confirmed at the code:
1. Grid: `web/tokens.css:62-67` and `web/layout.css:64`; measured 336 / 616 / 296 at 1280 by 800.
2. Pitch 616 by 411 with one uniform scale: `web/index.html:36-37`, `web/pitch.mjs:79-84`.
3. Control strip 40 px: `web/layout.css:144`.
4. Marker fill, conditional ring, contrast-chosen number: `web/colour.mjs:128-141`, drawn at `web/pitch.mjs:213-224`.
5. Ball trail, ball only, falling alpha: `web/pitch.mjs:228-241`. Deviation: one stroke per segment (issue CRAFT-TRAIL).
6. Local fonts and tabular numbers: `web/fonts/fonts.css:18,29`, `web/layout.css:87-89`.
7. No hex, `rgb()`, or `hsl()` outside the conversion module and tests; lightest token `oklch(0.995 …)` at `web/tokens.css:13`.
8. Mark drawn in code, no image file in `web/`: `web/mark.mjs:79`, favicon as a generated SVG data URI at `web/mark.mjs:136`.
9. The notice carries its state word: `web/main.mjs:49` beside the text from `web/playback.mjs:61-62`.
10. Skeleton, no spinner: `web/index.html:42-45`, `web/layout.css:256-262`.
Some implement citations point a few lines off; the items are correct. The layout spot-check used the `steer.md` text, because the canvas link is private.

**Anti-patterns (`02c-craft.md` section 5).** No marker shadow or glow, no gradient, no bounce or elastic easing, no font service, no image file. At kick-off the ball is drawn over the centre-forward's marker (`ac-a-t0.png`). Informational.

**Instrument (`04b-instrument.md` section 2).** `viewer.connected`, `viewer.socket_drops`, `viewer.tick_skipped`, `viewer.lag`, `viewer.frame_budget`, `viewer.history`, and `viewer.rewind` fire at runtime with their designed fields (`viewer-pitch-run-2/drive-instr.json` and this round's drives). `viewer.decode_refused` is covered by the test "a truncated frame is refused and named". `web.serving` fires on every server start with `isolated=true` and `files=30`. `fixture.replayed` with `sustain` fires only at the clean end of a full replay, which no drive reached; the field is present at `crates/stream/src/replay.rs:104`.

**Experiment (`04c-experiment.md`).** Status `deferred-to-experiment-flags`; nothing to check.

**Benchmark (`05c-benchmark.md`, compare mode).** No engine regression: wall 386 ms (−0.8 percent), 384.4 ms of processor time per match (−0.9 percent), 5.33 MB peak (−10.7 percent), stream 640,584 ticks per second (+0.9 percent). Page first values: 60 fps at 60 Hz with 0 dropped frames, decode 0.083 µs per frame, `page_bytes` 31,927,291, `history_bytes` 25,380,000. One budget miss by construction (BENCH-P95).

## Security Scan

- CVE scan: `cargo audit` is not installed; no crate or npm package was added, so no new dependency can carry a CVE. New critical or high: 0.
- Secret detection: `gitleaks` over the slice and over `75a4ba6`, no real finding.
- SAST: semgrep is not installed. Manual read of `crates/engine-cli/src/web.rs`: binds `127.0.0.1` only (`:51`); `GET` and `HEAD` only (`:94-102`); the path is percent-decoded, then `\`, `:`, NUL, `.` and `..` segments are refused, then the canonical path must stay under the root (`:124-167`); both isolation headers go out on every response (`:241-242`). New HIGH or above: 0.

## Accessibility Gate

- Tool: no axe-core is installed and a download needs approval, so no automated WCAG scan ran. A static read and computed-style checks replace it.
- New WCAG AA violations found: 0.
- Names and roles: `lang="en"`; both canvases `role="img"` with an `aria-label`; every button has a text name; the scrubber has `aria-label="Rewind to a tick"`; `#live` has `aria-live="polite"`.
- Focus: the focused `2x` button computes a 2 px solid outline plus a 4 px ring (`drive-lag.json`, step `focus-lag`).
- The play button carries no `aria-pressed` and toggles its label and `data-playing` (`drive-live.json`, step `play-toggle`).

## Performance Gate

- Bundle size delta: skipped. The base branch has no page, and the page ships 30 unbundled files.
- Build time delta: not measured against the base branch; the incremental release build takes about 7 seconds.
- Cold start: the page server prints its address in under 0.1 second in every drive; the base branch has no page server to compare.

## Cross-Slice Regression

- Siblings checked: `engine-core`, `data-schemas-generator`, `stream-protocol` (all `pass`).
- Method: the full `cargo test --workspace` (132 tests), which contains every sibling's recorded suite.
- Regressions found: 0.

## Longitudinal Delta

- Every surface: baseline source is the round-1 evidence at `verify-evidence/viewer-pitch-run-2/`. The screenshots show no layout, element, colour, or typography delta. The only change is the play button's accessible markup, which is not visible. Interpretation: expected.

## Friction Notes

- The implement record's 32 Hz reading came from pane throttling; the pane gives 60 Hz while visible.
- After the live stream ends, the notice reads "Stream ended — The match is no longer live" while the whole match is stored and still plays (class `ambiguous-copy`). `viewer-match-day` owns the live-mode decision.
- The scrubber tracks the newest received tick, not the drawn tick; after the live load the thumb sits at the far right while the clock reads 00:43.
- The grey effective-speed readout beside the buttons carries no label.
- `stack.toolchains-present` records Node v22.15.0; this machine runs Node v24.14.0. The page tests pass on it.

## Free Exploration Notes

- A second page opened against the same engine receives no ticks and shows the skeleton with no message — informational; `viewer-reports-recovery` owns it.
- A click on the scrubber beyond the newest received tick holds the newest tick and emits `viewer.rewind` with `stored: false` — informational, handled.
- `Next stop` with no stoppage announces "No later stoppage." — informational; `match-rules` fills the index.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Empty submission | n-a | The surface has no form. |
| Max-length input | n-a | The surface has no text input. |
| Double-click or rapid repeat | n-a | Not run; speed buttons set a value and are idempotent. |
| Mid-flow interruption (reload, tab close) | fail | ADV-RELOAD (round 1): the engine process exits with code 1; the page does not recover. |
| Network failure (stream closes mid-playback) | pass | "Stream ended" shows with its state word; the stored match still plays and rewinds. |
| Scrub beyond the received data | pass | The page holds the newest tick and reports `stored: false`. |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Slow response (`--sustain 3`) | pass | Playback settles at 3x with the lag notice (AC-e). |
| Concurrent session (second page to one engine) | pass with finding | No crash; the second page waits with no message. |
| Session expiry | n-a | The page has no authentication. |

## Cross-Browser Delta

- Primary browser: headless Microsoft Edge 153 (Chromium).
- Secondary browser: the in-app pane, Chromium 152. No Firefox or WebKit is installed, so this is not a cross-engine sweep.
- Divergences found: the pane refuses `performance.measureUserAgentSpecificMemory()` with a `SecurityError` while `crossOriginIsolated` is true; Edge resolves it. The page names the refusal in `page_bytes_reason`.

## Web Vitals

- Source: the page's Performance API, buffered observers, through the DevTools protocol.
- LCP: 87 ms and 78 ms (good).
- CLS: 0.0004 (good).
- INP: null. No interaction produced an event-timing entry over 16 ms.

## Gaps / Unverified Areas

- `fixture.replayed` with `sustain` was not observed at runtime.
- No automated WCAG scan and no Firefox or WebKit drive.
- The human-in-the-loop design check from the plan (contrast in a lit room, focus rings by eye) is not done; it completes at `integration`.
- ADV-RELOAD and the three LOW issues were not re-examined in round 2.

## Freshness Research

- The plan is dated 2026-09-22, so the AC staleness pass did not run.
- Chromium throttles `requestAnimationFrame` in a hidden surface: observed directly (32 Hz, then 0 frames while hidden; 60 Hz while visible).
- `performance.measureUserAgentSpecificMemory()` resolves in headless Edge 153 with cross-origin isolation and is refused in the pane's embedded Chromium 152.

## Recommendation

Advance to review. Every criterion is met on committed code with headless runtime evidence, and the one remaining failed check is a budget-definition defect already routed to `viewer-match-day`.

## Recommended Next Stage

- **Option A (recommended): review** → `/wf review football-manager-match-engine viewer-pitch`. Convergence is `converged` and the result is `pass`. Compact first: two verify rounds of drive output are noise for review.
- **Option D: skip review** → `/wf handoff football-manager-match-engine viewer-pitch`. Not recommended: review is slug-wide in this workflow, and three earlier slices still wait on the same ledger.
- **Option G: slug-wide runtime probe** → `/wf probe football-manager-match-engine`. Choose this to sweep the page and the engine together before review.
