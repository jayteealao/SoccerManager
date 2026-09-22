---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: viewer-pitch
status: complete
stage-number: 6
created-at: "2026-09-22T18:24:27Z"
updated-at: "2026-09-22T18:24:27Z"
result: partial
metric-checks-run: 13
metric-checks-passed: 12
metric-acceptance-met: 7
metric-acceptance-total: 8
metric-acceptance-user-observable: 5
metric-acceptance-code-only: 3
metric-interactive-checks-run: 5
metric-interactive-checks-passed: 4
metric-issues-found: 6
metric-issues-found-initial: 8
metric-issues-found-final: 6
fix-rounds-run: 1
convergence: escalated
verify-owned-fix-commit: "75a4ba6"
regression-tests-added: 1
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [web, cli]
bootstrap-failures:
  - {adapter: web, step: "in-app browser pane drive", remediation: "The pane throttles requestAnimationFrame while hidden (32 Hz, then 0 frames); the drive moved to headless Microsoft Edge 153 over the DevTools protocol, which ran at 60 Hz."}
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/viewer-pitch/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped — the base branch has no page; the page is 30 files served unbundled"
ac-staleness-checked: false
ac-stale-count: 0
longitudinal-baseline-compared: "skipped — no prior evidence run and no page on the base branch"
stability-check-flaky-count: 0
adversarial-tests-run: 3
adversarial-tests-failed: 1
failure-mode-probes-run: 2
cross-browser-delta: "none"
web-vitals-lcp-ms: 156
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
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine viewer-pitch"
---

# Verify: Viewer Pitch and Playback

## The Verification

The implement stage left a page that draws the match, eight criteria with seams built for each, and one open verdict: AC-a had only reached 32 frames per second in the browser pane. That shortfall was not the page. The pane throttles animation frames while it is hidden, and it gave 60 Hz while it was visible. The pane then stayed hidden, so every criterion was driven in headless Microsoft Edge at 1280 by 800 and 60 Hz, over its DevTools protocol, with trusted pointer clicks. Headless Edge also resolved the whole-page memory figure that the pane refuses: 31.9 MB for a full stored match, against a budget of 300 MB.

Seven of eight criteria are met. The drives found two defects that the implement drive could not see. First, against the live engine, a click on 4x after the stream ends left playback at 1x while 4x showed as pressed. Second, every page load skipped 2 to 6 ticks at 1x in its first second, so AC-a holds 60 frames per second for 5 minutes but not with zero dropped ticks. The fix loop ran one round on three Fix decisions. The speed fix and the play-button accessibility fix re-checked clean. The tick-skip patch was wrong: its cap also cut normal fractional steps, so 1x ran at 0.59 times real time. That patch was discarded. The two good fixes were committed at `75a4ba6` on the product owner's instruction.

The result is `partial` with convergence `escalated`. The next step is a second verify round with the corrected cap condition, `position >= before + 2`, in place of `position > before + 1`. The top open risk is carried to `viewer-reports-recovery`: a page reload ends the engine process with exit code 1, and the reloaded page never recovers.

## Verification Summary

- Branch `feat/football-manager-match-engine`, head `36139b4` (the slice commit `62a5dab` plus a docs commit). Stack confirmed: `web`, `cli`.
- 13 automated checks run, 12 pass. The failure is the page frame-time p95 budget, which is a budget-definition defect.
- 8 criteria: 5 user-observable, 3 code-only. 7 met, 1 partially met (AC-a).
- 5 user-observable criteria driven in headless Edge, 4 pass. Every drive used trusted pointer events and was repeated at least twice more.
- 8 issues found, 3 triaged Fix, 2 fixed and re-checked, 1 could not be fixed. 6 issues remain.
- Verify-owned commit `75a4ba6` (product-owner instruction) holds the two accepted fixes: `web/playback.mjs`, `web/tests/playback.test.mjs`, `web/index.html`, `web/main.mjs`, `web/components/match-control.css`.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (`verify-evidence/viewer-pitch/fmt.txt`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass, no warnings (`clippy.txt`).
- `cargo test --workspace`: pass, 132 passed, 0 failed, 0 ignored (`cargo-test.txt`).
- `node --test "web/tests/*.test.mjs"`: pass, 40 of 40 before the fix loop (`node-test.txt`); 42 of 42 after it (`node-test-after-fix.txt`).
- `cargo build --release -p engine-cli`: pass (`build-release.txt`).
- Secret detection, `gitleaks git --log-opts="3570eb9..62a5dab"` plus a grep of added lines: pass. One hit is a false positive on prose at `04-plan-viewer-pitch.md:184`.
- Dependency diff, `git diff 3570eb9..62a5dab -- Cargo.lock **/Cargo.toml`: pass, empty; no `package.json` exists. `cargo audit` is not installed.
- SAST: semgrep is not installed; a manual read of `crates/engine-cli/src/web.rs` found no HIGH finding (see `## Security Scan`).
- `sdlc-debt:` markers in the slice diff: pass, 0 new markers.
- Benchmark compare, engine targets (3 drives of `bench --matches 5`, `bench --stream`, `cargo bench -p engine --bench tick_step`): pass, no tripwire (`bench-*.stdout.txt`, `criterion.stdout.txt`).
- Benchmark compare, page targets: fail. The frame-time p95 reads 16.69 to 16.85 ms against a 16.6 ms budget that no 60 Hz panel can meet. The other three page budgets pass.
- Instrument key check, `04b-instrument.md` section 2: pass, 8 of 8 page signals and `web.serving` observed; `fixture.replayed` with `sustain` not observed (see `## Augmentation Verification`).
- Mock fidelity inventory, `02c-craft.md` section 3: pass, 10 of 10 honored, one wording deviation.

## Interactive Verification Results

Platform `web`, primary tool headless Microsoft Edge 153 (`HeadlessChrome/153 Edg/153`) driven by `verify-evidence/viewer-pitch/drive.mjs` and `cdp.mjs` over the DevTools protocol. Clicks are `Input.dispatchMouseEvent`, which the page records as `isTrusted: true`. Servers: `target/release/engine-cli.exe replay --fixture fixture.smfx --speed 8 --web web` (seed 7 fixture) and `serve --seed 42 --web web` (live engine).

**AC-a — 60 frames per second with zero dropped ticks at 1x for 5 minutes**
- Platform & tool: web, headless Edge at 60 Hz.
- Steps performed: opened the page against the fixture replay; sampled `window.__touchline.frame()`, the clock, the pressed speed, and a running sum of `viewer.tick_skipped` every 30 seconds for 300 seconds; screenshots at 0, 150, and 300 seconds.
- Evidence: `drive-main.json` (step `ac-a`), `ac-a-t0.png`, `ac-a-t150.png`, `ac-a-final.png`; startup skips also in `drive-probe.json` and `drive-instr.json`.
- Observation: the clock ran from 00:00 to 05:00 at 1x with `fps_median` 60 and `refresh_hz` 60 in all 60 five-second windows. One `viewer.tick_skipped` row fired at tick 45 with `skipped: 4`, in the first second of playback; no tick was skipped in the remaining 299 seconds. `dropped_frames` was 0 in most windows and at most 2. The startup skip reproduced on every load: 6, 4, and 5 ticks (2 + 3) in three separate sessions.
- Result: partial. The frame rate is met. Zero dropped ticks is not met, because the first frames stall while the stream burst decodes and the scheduler passes over ticks.

**AC-d — at 4x the clock advances four times faster than wall time within 2 percent**
- Platform & tool: web, headless Edge.
- Steps performed: a click on `4x`; three windows of 20 wall seconds, each reading `lastRenderedTick()` and `performance.now()` at both ends.
- Evidence: `drive-main.json` (step `ac-d`), `ac-d-t0.png`, `ac-d-final.png`.
- Observation: 4.0007, 3.9996, and 4.0009 times wall time; the largest error is 0.02 percent. The clock read 05:09 to 09:09 over 60 wall seconds.
- Result: pass, stable over 3 windows.

**AC-e — a sustained 3x drops playback to 3x with a notice; a sustained 8x shows no notice**
- Platform & tool: web, headless Edge, server flag `--sustain 3`, then no cap.
- Steps performed: run 1 with `replay --speed 8 --sustain 3`, a click on `8x`, three reads 8 seconds apart, then a 10-second rate window. Run 2 with `replay --speed 8`, the same steps.
- Evidence: `drive-lag.json`, `ac-e-lag-t0.png`, `ac-e-lag-final.png`, `drive-nolag.json`, `ac-e-nolag-t0.png`, `ac-e-nolag-final.png`.
- Observation: run 1 notice "Lag Playing at 3x — the engine sustains three times real time" in all three reads; live region "Speed 8 times asked for. Playing at 3 times."; `viewer.lag` with `requested_speed` 8, `sustained_speed` 3, `measured_speed` 2.99, `notice_shown` true; measured playback 3.002 times. Run 2: notice empty in all three reads, 0 `viewer.lag` rows, measured playback 7.992 times.
- Result: pass, stable over 3 reads per run.

**AC-f — a rewind to a stored tick draws exactly the stored positions**
- Platform & tool: web, headless Edge.
- Steps performed: trusted clicks on the scrubber track. Session 1 (fixture replay, still arriving): three clicks. Session 2 (live engine, match fully stored, paused): clicks at 20, 50, and 80 percent of the track, repeated in a second session after the fix loop.
- Evidence: `drive-main.json` (step `ac-f`), `ac-f-25.png`; `drive-live.json` (step `ac-f-live`), `ac-f-live-20.png`, `ac-f-live-50.png`, `ac-f-live-80.png`.
- Observation: every rewind that landed on a stored tick reported `exact: true` over 47 components, and the hook's own comparison of `tickAt(tick)` with `lastRendered()` agreed: ticks 55,694; 41,435; 134,980; 228,565, twice each for the last three. Two clicks produced no exact rewind, for stated reasons: one click landed on the thumb and changed no value; one click landed past the newest received tick, and the page emitted `viewer.rewind` with `stored: false` and held the newest tick.
- Result: pass, stable over two sessions.

**AC-h — charter scenario steps 1, 4, and 5 against the live engine**
- Platform & tool: web, headless Edge, `engine-cli serve --seed 42 --web web`.
- Steps performed: step 1 read the header; step 4 read 40 frames 250 ms apart and checked every player position against the pitch bounds; step 5 clicked `4x` and measured two 15-second windows. Steps 2 and 3 use the default lineup, as the slice states.
- Evidence: `drive-live.json`, `ac-h-step1.png`, `ac-h-step4.png`, `ac-h-step5.png`; the pre-fix run is `drive-live-prefix.json`.
- Observation: step 1 shows "Oakmere Rangers v Eldstead City" and "engine 0.1.0", and `viewer.connected` carries `engine.version` 0.1.0. Step 4: the clock advanced from 00:00 to 00:11, all 22 players and the ball moved, and no player left the pitch (largest |x| 49.65 m, largest |y| 27.00 m). Step 5 before the fix: the whole match arrived in about 13 seconds and the socket closed; after that, a click on `4x` showed 4x as pressed and the page played at 1.000 times. Step 5 after the fix: 3.999 and 3.998 times, with the effective readout at 4x.
- Result: pass after the verify-owned fix `75a4ba6`.

## Acceptance Criteria Status

| Criterion | Kind | Status | Method | Evidence | Evidence-rung |
|---|---|---|---|---|---|
| AC-a: 5 minutes at 1x, 60 fps, zero dropped ticks | user-observable | partially met | interactive | `drive-main.json`, `ac-a-*.png`; 4 ticks skipped at load | headless |
| AC-b: a marker lies on the segment between two ticks | code-only | met | automated | `node-test.txt`: 4 tests in `interpolate.test.mjs` | n-a |
| AC-c: at 8x the viewer skips ticks and never passes the newest tick | code-only | met | automated | `node-test.txt`: 5 tests in `schedule.test.mjs` | n-a |
| AC-d: 4x clock within 2 percent | user-observable | met | interactive | `drive-main.json` step `ac-d`: error at most 0.02 percent | headless |
| AC-e: sustained 3x drops to 3x with a notice; 8x shows none | user-observable | met | interactive | `drive-lag.json`, `drive-nolag.json` | headless |
| AC-f: rewind draws exactly the stored tick | user-observable | met | interactive | `drive-main.json`, `drive-live.json`: 7 exact rewinds | headless |
| AC-g: a full match keeps history under 300 MB | code-only | met | automated | `history.test.mjs` 25,380,000 bytes; page figure 31,924,655 bytes in `drive-live.json` | n-a |
| AC-h: charter scenario steps 1, 4, 5 against the live engine | user-observable | met (after verify-owned fix `75a4ba6`) | interactive | `drive-live.json`, `ac-h-step*.png` | headless |

Evidence: headless 5 / n-a 3. No user-observable criterion rests on a mock or on static reasoning (`metric-acceptance-mock-rung: 0`). The fixture is a recording of the real engine, not an emulation of an external interface, so `mock-provenance` does not apply.

## Issues Found

- HIGH: AC-A — every page load skips 2 to 6 ticks at 1x in the first second, at `web/schedule.mjs:72`. Triage: Fix. Could not fix: the patch capped with `position > before + 1`, which truncates normal fractional steps and ran 1x at 0.59 times real time (existing test "at one times speed no tick is skipped" failed with `consumed 29.67`). Discarded; the patch is kept at `verify-evidence/viewer-pitch/fix-ac-a-discarded.diff`. Suggested fix for the next round: the same method with the condition `this.position >= before + 2`.
- MED: BENCH-P95 — the `viewer frame time p95` budget of 16.6 ms is below one 60 Hz frame interval, so it cannot be met; measured 16.69 to 16.85 ms. Triage: Skip. Recorded in `05c-benchmark.md`; `viewer-match-day` re-states the budget relative to `refresh_hz`.
- MED: ADV-RELOAD — a page reload or tab close ends `replay` and `serve` with exit code 1 and the raw text "An established connection was aborted by the software in your host machine. (os error 10053)"; the reloaded page shows unstyled HTML at "Waiting for the engine" with no error state. At `crates/stream/src/replay.rs:95`. Triage: Escalate to `viewer-reports-recovery`, with the replayer's abort handling owned by `stream-protocol` code.
- LOW: SIGNAL-RING — `viewer.history` fires on every keyframe, so the 256-row ring on the test hook drops rarer signals (`viewer.connected`, `viewer.lag`) within seconds at fast arrival; a live load writes 5,400 history rows. At `web/main.mjs:94`. Triage: Skip; `/wf observability audit` owns page sampling.
- LOW: CRAFT-TRAIL — the ball trail is stroked once per segment, while `02c-craft.md` callout 8 says "one path, never N separate strokes" and the implement record calls it one path. At `web/pitch.mjs:234`. Triage: Skip; a falling alpha needs one stroke per segment on a 2D canvas.
- LOW: WEB-HARDEN — the page server has no socket read timeout, no Host header check (DNS rebinding can read the static files and `/engine.json`), and no `X-Content-Type-Options: nosniff`. At `crates/engine-cli/src/web.rs:74`. Triage: Skip; loopback only, no secret is served, and the socket checks Origin.

Resolved in this run (not counted above): AC-H5 (the stale effective speed) and A11Y-PLAY (`aria-pressed` on the play button).

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| AC-H5 | unmet-ac | Fix | Patched | `web/tests/playback.test.mjs` (2 tests; both fail on the old `playback.mjs`, both pass on the patch) | Pass: 3.999 and 3.998 times after the stream ended, effective 4x |
| AC-A | unmet-ac | Fix | Could not fix (patch discarded) | not added; the new test passed but an existing test failed | Still failing |
| A11Y-PLAY | check-failure | Fix | Patched | exempt: `main.mjs` imports the DOM at load and has no DOM-free test seam | Pass: label Pause/Play, `aria-pressed` null, `data-playing` toggles, live region "Paused.", same fill as before |
| BENCH-P95 | augmentation-regression | Skip | N/A | n-a | Not re-run |
| ADV-RELOAD | check-failure | Escalate | N/A | n-a | Not re-run |
| SIGNAL-RING | check-failure | Skip | N/A | n-a | Not re-run |
| CRAFT-TRAIL | augmentation-regression | Skip | N/A | n-a | Not re-run |
| WEB-HARDEN | check-failure | Skip | N/A | n-a | Not re-run |

The three fix agents ran with worktree isolation, and all three lost shell access inside the worktree. Two wrote their patches without running them; the orchestrator tested those patches in the worktrees. The A11Y-PLAY agent changed nothing and was re-dispatched without isolation, because its files did not overlap the other two. Triage ran through two gate rounds of four questions each; the product owner took the recommended option on all eight.

Commit: `75a4ba6`, made on the product owner's explicit instruction after this run, although the AC-A re-check still fails. It holds only the AC-H5 and A11Y-PLAY patches, which each passed their own re-check. The AC-A fix is not in it; `verify-evidence/viewer-pitch/fix-accepted-working-tree.diff` records the committed patches.

Regression tests added: 1.

## Augmentation Verification

**Mock fidelity inventory (`02c-craft.md` section 3).** All 10 items honored, confirmed at the code, not from the implement record:
1. Grid: `web/tokens.css:62-67` and `web/layout.css:64`; measured 336 / 616 / 296 at 1280 by 800 in the screenshots.
2. Pitch 616 by 411 with one uniform scale: `web/index.html:36-37`, `web/pitch.mjs:79-84`.
3. Control strip 40 px: `web/layout.css:144`.
4. Marker fill, conditional ring, contrast-chosen number: `web/colour.mjs:128-141`, drawn at `web/pitch.mjs:213-224`.
5. Ball trail, ball only, falling alpha: `web/pitch.mjs:228-241`. Deviation: one stroke per segment, not one path (issue CRAFT-TRAIL).
6. Local fonts and tabular numbers: `web/fonts/fonts.css:18,29`, `web/layout.css:87-89`.
7. No hex, `rgb()`, or `hsl()` outside the conversion module and tests; lightest token `oklch(0.995 …)`: `web/tokens.css:13`.
8. Mark drawn in code, no image file in `web/`: `web/mark.mjs:79`, favicon as a generated SVG data URI at `web/mark.mjs:136`.
9. The notice carries its state word: `web/main.mjs:49` ("Lag" or "Stream ended") beside the text from `web/playback.mjs:61-62`.
10. Skeleton, no spinner: `web/index.html:42-45`, `web/layout.css:256-262`.
Several implement citations point one to thirty lines off; the items themselves are correct. The visual spot-check against the north-star layout found no composition or hierarchy regression; the canvas link is private, so the check used the `steer.md` text.

**Anti-patterns (`02c-craft.md` section 5).** No marker shadow or glow, no gradient, no bounce or elastic easing (the one easing is `cubic-bezier(0.32, 0.72, 0, 1)`), no font service, no image file. One observation: at kick-off the ball is drawn over the centre-forward's marker (`ac-a-t0.png`), which touches "nothing is drawn over a player marker". Informational.

**Instrument (`04b-instrument.md` section 2).**
- `viewer.connected`: fires once, with `protocol.version`, `engine.version`, `match.id`, `owner.id`, `origin`, `ticks_expected` (`drive-instr.json`).
- `viewer.socket_drops`: fires when the live stream closes, with `tick` 30000, `code` 1005, `wasClean` false.
- `viewer.tick_skipped`: fires once per second while skipping, with `speed`.
- `viewer.lag`: fires on the lag edge, with all four designed fields plus `measured_speed`.
- `viewer.frame_budget`: every 5 seconds, with `refresh_hz`.
- `viewer.history`: every keyframe, with `page_bytes` resolved in headless Edge.
- `viewer.rewind`: per rewind, with `exact` and `stored`.
- `viewer.decode_refused`: covered by the test "a truncated frame is refused and named".
- `web.serving`: on every server start, with `isolated=true` and `files=30`.
- `fixture.replayed` with `sustain`: not observed, because it fires only at the clean end of a full replay (about 11 minutes at 8x) and every drive ended earlier. The field is present at `crates/stream/src/replay.rs:104`. Gap, not a failure.

**Experiment (`04c-experiment.md`).** Status `deferred-to-experiment-flags`; nothing to check.

**Benchmark (`05c-benchmark.md`, compare mode).** No engine regression: wall 386 ms (−0.8 percent), 384.4 ms of processor time per match (−0.9 percent), 5.33 MB peak (−10.7 percent), tick step 1.4434 µs (+0.4 percent), stream 640,584 ticks per second (+0.9 percent). Page first values: 60 fps at 60 Hz, decode 0.083 µs per frame, `page_bytes` 31,924,655, `history_bytes` 25,380,000. One budget miss by construction (BENCH-P95). The sibling `05c-benchmark.yaml` and fragment now carry `mode: compare` and `compare_commit: "36139b4"`; the verifier passes them. The verifier still fails the pre-existing `history/05c-benchmark-2.yaml`, which has no `after` values; that file predates this run.

## Security Scan

- CVE scan: `cargo audit` is not installed; the slice adds no crate and no npm package, so no new dependency can carry a CVE. New critical or high: 0.
- Secret detection: `gitleaks` over `3570eb9..62a5dab`, 1 false positive on prose; manual grep of added lines, 0 findings.
- SAST: semgrep is not installed. Manual read of `crates/engine-cli/src/web.rs`: binds `127.0.0.1` only (`:51`); `GET` and `HEAD` only, else 405 (`:94-102`); the path is percent-decoded before it is checked (`:158`), then `\`, `:`, NUL, `.` and `..` segments are refused (`:159-167`), then the canonical path must stay under the root (`:124-128`); both isolation headers go out on every response including errors (`:241-242`). New HIGH or above: 0. Three LOW items are issue WEB-HARDEN.

## Accessibility Gate

- Tool: no axe-core is installed and a download needs approval, so no automated WCAG scan ran (`a11y-result: not-automatable`). A static read and a computed-style check replace it.
- New WCAG AA violations found: 0.
- `lang="en"` is on the root element; both canvases have `role="img"` and an `aria-label`; every button has a text name; the scrubber has `aria-label="Rewind to a tick"`; the control group has `aria-label="Playback"`; `#live` has `aria-live="polite"`.
- Focus: the focused `2x` button computes `outline: solid 2px oklch(0.55 0.17 250)` plus a 4 px ring (`drive-lag.json`, step `focus-lag`).
- Fixed in this run: the play button no longer carries `aria-pressed` beside a changing label.
- Remaining note: `#notice` has `role="status"` but toggles `aria-hidden`, so its own announcement may be unreliable; `#live` repeats the text, so no information is lost.

## Performance Gate

- Bundle size delta: skipped. The base branch has no page, and the page ships 30 unbundled files. The Rust release binary rebuilt in 7.4 seconds incrementally.
- Build time delta: not measured against the base branch; the incremental release build took 7.36 seconds.
- Cold start: the page server prints its address in under 0.1 second after launch in every drive (`server-*.log`); no base-branch comparison exists, because the base branch has no page server.

## Cross-Slice Regression

- Siblings checked: `engine-core`, `data-schemas-generator`, `stream-protocol` (all `pass`). The slice touches `crates/protocol`, `crates/stream`, and `crates/engine-cli`, which the siblings own.
- Method: the full `cargo test --workspace` (132 tests), which contains every sibling's recorded suite.
- Regressions found: 0.

## Longitudinal Delta

- Every surface: baseline source skipped. No prior evidence run exists for this slice, and the base branch has no page to screenshot. The screenshots in this run become the baseline for the next verify run.

## Friction Notes

- The implement record's 32 Hz reading came from pane throttling, not from the display: the same pane gave 60 Hz while visible. The record's claim that AC-a "cannot pass in this driver" is superseded.
- After the live stream ends, the notice reads "Stream ended — The match is no longer live" while the whole match is stored and still plays. The text is accurate for the engine and misleading for the manager (class `ambiguous-copy`). `viewer-match-day` owns the live-mode decision.
- The scrubber tracks the newest received tick, not the drawn tick. After the live load, the thumb sits at the far right while the clock reads 00:43 (`ac-h-step5.png`). A manager reads the thumb as the playback position.
- The grey effective-speed readout (for example "1x" beside the buttons) carries no label; its meaning shows only when it differs from the pressed button.
- Headless Edge and the pane both report Node v24.14.0 on this machine, while `stack.toolchains-present` records Node v22.15.0. The page tests pass on both.

## Free Exploration Notes

- A second page opened against the same engine connects to the page server but receives no ticks and shows the skeleton with no message, because the engine serves one session — informational; `viewer-reports-recovery` owns it.
- A click on the scrubber beyond the newest received tick holds the newest tick and emits `viewer.rewind` with `stored: false` — informational, handled.
- A click on the scrubber thumb changes no value and draws no rewind — informational, standard range-input behaviour.
- `Next stop` with no stoppage announces "No later stoppage." in the live region — informational; `match-rules` fills the index.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Empty submission | n-a | The surface has no form. |
| Max-length input | n-a | The surface has no text input. |
| Double-click or rapid repeat | n-a | Not run; speed buttons are idempotent by construction (`select` sets a value). |
| Mid-flow interruption (reload, tab close) | fail | ADV-RELOAD: the engine process exits with code 1, the reloaded page stays unstyled at "Waiting for the engine". |
| Network failure (stream closes mid-playback) | pass | The "Stream ended" notice shows with its state word; the stored match still plays and rewinds. |
| Scrub beyond the received data | pass | The page holds the newest tick and reports `stored: false`. |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Slow response (`--sustain 3`) | pass | Playback settles at 3x with the lag notice; covered by AC-e. |
| Concurrent session (second page to one engine) | pass with finding | No crash; the second page waits with no message (Free Exploration Notes). |
| Session expiry | n-a | The page has no authentication. |

## Cross-Browser Delta

- Primary browser: headless Microsoft Edge 153 (Chromium).
- Secondary browser: the in-app browser pane, Chromium 152. No Firefox or WebKit is installed, so this is not a cross-engine sweep.
- Divergences found: one. The pane refuses `performance.measureUserAgentSpecificMemory()` with a `SecurityError` while `crossOriginIsolated` is true; Edge resolves it. The page names the refusal in `page_bytes_reason`, so the divergence is visible, not silent.

## Web Vitals

- Source: the page's own Performance API through the DevTools protocol, buffered observers.
- LCP: 124 ms and 156 ms in two loads (good, under 2500 ms).
- CLS: 0.0004 (good, under 0.1).
- INP: null. No interaction produced an event-timing entry over the 16 ms threshold, so every measured click finished in under 16 ms.

## Gaps / Unverified Areas

- AC-a zero dropped ticks: not met until the corrected scheduler cap lands and a 5-minute drive shows no `viewer.tick_skipped` row at 1x.
- `fixture.replayed` with `sustain` was not observed at runtime.
- No automated WCAG scan and no Firefox or WebKit drive.
- The human-in-the-loop design check (contrast in a lit room, focus rings by eye) from the plan's verification strategy is not done; it completes at `integration`.
- The accepted fixes are committed at `75a4ba6`; the AC-A fix still needs its own commit.

## Freshness Research

- The plan is dated 2026-09-22, so it is not older than 14 days; the AC staleness pass did not run.
- `requestAnimationFrame` throttling in a hidden Chromium surface: observed directly (32 Hz, then 0 frames while the pane was hidden, 60 Hz while visible). A frame-rate drive must keep its surface visible or run headless.
- `performance.measureUserAgentSpecificMemory()`: resolves in headless Edge 153 with cross-origin isolation (28.4 MB to 31.9 MB); refused in the pane's embedded Chromium 152 with the same headers.

## Recommendation

Do not advance to review on this run. Run one more verify round: apply the AC-A method with the condition `this.position >= before + 2`, re-run the page tests, and re-drive AC-a for 5 minutes. When that round converges, it commits the AC-A fix.

## Recommended Next Stage

- **Option B (recommended): second verify round** → `/wf verify football-manager-match-engine viewer-pitch`. One issue is still broken (AC-A), its cause is known, and the corrected condition is one expression. The two accepted fixes are already committed at `75a4ba6`.
- **Option C: manual implement** → `/wf implement football-manager-match-engine viewer-pitch`. Choose this to rework the stall policy by hand, for example to hold playback until the first burst is decoded instead of capping the step.
- **Option F: probe-only re-check** is not needed: every user-observable criterion has headless runtime evidence, and no deferral applies.
