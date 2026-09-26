---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: viewer-lineup-tactics
status: complete
stage-number: 6
created-at: "2026-09-23T11:36:00Z"
updated-at: "2026-09-23T11:36:00Z"
result: pass
metric-checks-run: 10
metric-checks-passed: 10
metric-acceptance-met: 6
metric-acceptance-total: 6
metric-acceptance-user-observable: 5
metric-acceptance-code-only: 1
metric-interactive-checks-run: 6
metric-interactive-checks-passed: 6
metric-issues-found: 0
metric-issues-found-initial: 2
metric-issues-found-final: 0
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: "8a5a182"
regression-tests-added: 5
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [web, cli]
bootstrap-failures:
  - {adapter: web, step: "in-app browser pane drive", remediation: "Not used. The pane throttles animation frames while hidden (viewer-pitch and viewer-match-day runs). Every drive ran in headless Microsoft Edge 153 over the DevTools protocol, with the harness at verify-evidence/viewer-lineup-tactics/cdp.mjs."}
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/viewer-lineup-tactics/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: pass
cross-slice-regressions-found: 1
metric-bundle-size-delta-pct: "skipped — the page is served unbundled; no bundler exists"
ac-staleness-checked: true
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 0
adversarial-tests-run: 4
adversarial-tests-failed: 0
failure-mode-probes-run: 2
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
tags: [viewer, lineup, tactics, substitutions, pending-chips, flow-control, headless-edge, verify-fix]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-viewer-lineup-tactics.md
  plan: 04-plan-viewer-lineup-tactics.md
  implement: 05-implement-viewer-lineup-tactics.md
  review: 07-review-viewer-lineup-tactics.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine viewer-lineup-tactics"
---

# Verify: Lineup Editor, Tactics Panel, and Substitutions

## The Verification

Implement left the lineup editor, the tactics panel, the substitution picker, and the pending-change chips on commit `b46f083`, with 290 Rust tests and 103 page tests green. It had driven one criterion live (AC-b on seed 11). It named one risk: at 8x the engine could run far ahead of the pitch, so a change could apply later than the manager expects. This run drove all six criteria in headless Microsoft Edge 153 at 1280 by 800 against the live `serve` engine: seed 11 for the editor, seed 42 for play. The first drive confirmed the risk as a real fault. At 8x the engine was 1,848 to 4,627 ticks ahead when the manager acted. In 3 of 5 samples the change skipped the next dead ball on screen; one waited 18,566 ticks (about 6 minutes of match). A 1x sample was 1,281 to 1,977 ticks ahead, against the product owner's 500-tick bound (shape Round 3 Q10).

The single fix round landed two commits. `d47e7d3` adds a `seen` command. The page reports the tick it draws at most every 100 ms and once before kick-off. `serve` then produces no tick more than `buffer_ticks` (500) past that report. After it, the lead at a change was 460 to 500 ticks at 8x. All 6 of 6 changes at 8x and 3 of 3 at 1x applied at the next dead ball on screen. The re-check drives found two more faults, and `8a5a182` fixes both. First, the feed stopped following the newest row once a chip appeared or the roles list opened. That fault was already there before the fix. The follow check read the feed shrinking as the manager scrolling up. Second, the lag notice fired on a healthy match and never cleared, because a held engine delivers ticks only as fast as playback uses them. The first fix caused this fault. Final state: 292 Rust tests, 106 page tests, fmt, clippy, gitleaks, and both benchmark gates pass. All six criteria are met with headless evidence.

Review can now read a slice whose timing claim is measured, not assumed. The top open risk is `seen` itself. It is a new client command on protocol version 3 with no version bump. It is additive and ignored by `record`, `replay`, and `bench`, but review should confirm that choice. The throttled-tab case needs a check too: a hidden tab stops drawing, so a live engine now waits for it.

## Verification Summary

- Slice: `viewer-lineup-tactics`, standard mode, branch `feat/football-manager-match-engine` (dedicated; confirmed with `git branch --show-current`).
- Stack: `user-confirmed: true` in `00-index.md`; the plan carries no `stack-source: unconfirmed-auto-detect`. `stack-source: confirmed`.
- Constraint resolution: every user-observable AC in the plan's Verification Strategy carries a `constraint-resolution:` line (prerequisite-slice for AC-a to AC-e; AC-f has no environment wall). `constraint-resolution-missing: []`.
- Checks: 10 run, 10 pass in the final state. One check (the page tests) first "failed" because of the verify run's own wrong invocation (`node --test web/tests/` as a directory); re-run with the file glob, it passed 103 of 103. That was a tooling error, not a product failure.
- Acceptance: 6 of 6 met. 5 user-observable, all at `evidence-rung: headless` (real engine, real page, headless browser); 1 code-only (AC-f, explicitly annotated `observable: false`), met by a keyboard-only browser drive.
- Fix loop: 2 issues at the gate, 1 round, 3 patches in 2 commits (one patch fixed a regression the first patch caused), 5 regression tests, 0 issues left.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (before and after the fixes; `fmt.txt`, `fmt-after-fix.txt`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass (`clippy.txt`, `clippy-after-fix.txt`).
- `cargo test --workspace`: pass. Before: 290 passed, 0 failed, 4 ignored (`cargo-test.txt`). After: 292 passed, 0 failed, 4 ignored (`cargo-test-after-fix.txt`).
- `node --test web/tests/*.test.mjs`: pass. Before: 103 of 103 (`node-test.txt`). After: 106 of 106 (`node-test-after-fix.txt`).
- `cargo build --release -p engine-cli`: pass (`build-release.txt`), rebuilt after each Rust change.
- Secret scan `gitleaks 8.30.1` over commits `53dae38..HEAD` and over `crates/protocol/src`, `crates/stream/src`, `crates/engine-cli/src`, `web`, `docs/reference`: no leaks (`gitleaks.txt`).
- Benchmark gate `engine-cli bench --seed 42 --matches 5 --json`: pass. 2079 ms over 5 matches is 415.8 ms per match (gate 460.7 ms); 1.4713 µs per tick; peak 6.56 MB (gate 6.82 MB) (`bench-cpu.stdout.txt`).
- Stream benchmark `engine-cli bench --seed 42 --matches 1 --stream --json`: pass. 613,223 ticks/s delivered (baseline 609,220); peak 7.88 MB (tripwire 8.55 MB) (`bench-stream.stdout.txt`).
- Lag-notice regression probe (the viewer-pitch lag criterion, re-driven after `8a5a182`): pass. `replay --sustain 3` at 8x shows "Playing at 3x — the engine sustains three times real time" in 3 of 3 reads. Uncapped, it plays at 8x with no notice in 3 of 3 reads (`probe-lag-lag.json`, `probe-lag-nolag.json`).
- Feed-follow probe (substitution chip late in the match, then roles list open, then wheel up and down): pass after `8a5a182` (`probe-feed-sub-late-roles-after.json`); the same probe failed before (`probe-feed-sub-late-before.json`).

## Interactive Verification Results

All drives: headless Microsoft Edge 153, 1280 by 800, over the DevTools protocol (`cdp.mjs`, extended here with real key events), against `target/release/engine-cli.exe serve --seed <n> --web web`. Mouse clicks and key presses are real input events; the only scripted DOM writes are `focus()` before a key press, and opening the roles `details` in the AC-e drive.

**AC-a — illegal lineups disable kick-off with an inline reason; a legal one enables it**
- Platform & tool: web, headless Edge; `drive-lineup.mjs`, seed 11.
- Steps: read the initial lineup; pick slot 4 and "Empty the picked slot" (mouse); restore it; swap slot 1 and slot 2 (goalkeeper out of slot 1); swap back; pick the slot-2 player from the squad and place him in slot 3; click kick-off while illegal; restore; read the legal state.
- Evidence: `verify-evidence/viewer-lineup-tactics/ac-a-1-fewer-than-eleven.png`, `ac-a-2-no-goalkeeper.png`, `ac-a-3-placed-twice.png`, `ac-a-4-legal.png`, `drive-lineup.json`.
- Observation: fewer than eleven → "Ten starters; a match needs eleven.", kick-off `disabled`. No goalkeeper → "The goalkeeper's slot holds Sador Ostez, who is not a goalkeeper.", disabled. Placed twice → "Sador Ostez is placed twice.", disabled, and a click on kick-off leaves the phase `pre-match`. Legal → "The lineup is ready.", enabled. The reason is shown inline beside the control (`aria-describedby="lineup-reason"`) with the word "Not ready:" as well as the colour.
- Result: pass.

**AC-b — a mentality change shows a chip, and at the next dead ball the chip clears and the feed says "Tactical change applied"**
- Platform & tool: web, headless Edge; `drive-live.mjs`, seed 42, 8x; the samples are from `drive-timing.mjs`.
- Steps: kick off; select 8x; at rendered tick 1539 focus Mentality and press ArrowDown (Balanced to Positive); watch the chip, the feed, and the pending hook.
- Evidence: `ac-b-1-queued.png`, `ac-b-2-applied.png`, `ac-b-3-chip-cleared.png`, `drive-live.json`; the before and after timing samples are in `drive-timing-8x-before.json`, `drive-timing-1x-before.json`, `drive-timing-8x-after.json`, and `timing-superseded/drive-timing-{8x,1x}-after.json`.
- Observation (final): the chip `chip-q-2006-0` shows "Queued" (engine tick 2006, 467 ahead of the pitch). It turns "Applied" at tick 2895, the first stoppage after the manager's action. The feed's newest row reads "0' Tactical change applied — Oakmere Rangers". The chip is gone by rendered tick 3072 (after the 150-tick hold). Before the fix, the same drive queued at engine tick 3807 while the pitch showed 1538. The timing samples at 8x skipped a visible dead ball in 3 of 5 cases (for example, action at 3125, next dead ball on screen 5949, applied 24515). After the fix: 6 of 6 at 8x and 3 of 3 at 1x applied at the next dead ball on screen, with 0 skipped.
- Result: pass (after fix; before fix: fail, issue VER-1).

**AC-c — a substitution applies at the next dead ball, the lineup panel swaps, the count decrements**
- Platform & tool: web, headless Edge; `drive-live.mjs`, seed 42, 8x.
- Steps: choose "9 Ninis Palova off" and "12 Peric Stoini on" with arrow keys; click "Queue substitution"; read the lineup panel and the count just before and after the verdict tick.
- Evidence: `ac-c-1-queued.png`, `ac-c-2-applied.png`, `drive-live.json` (`ac-c-applied.preVerdict`).
- Observation: queued at engine tick 3592. At rendered tick 5479 (before the verdict) the panel still lists Ninis Palova and reads "5 of 5 substitutions left". The change applies at 5949, the first stoppage after the queue tick. Slot 6 of the home panel then lists Peric Stoini, the count reads "4 of 5 substitutions left", and the feed reads "1' SUBSTITUTION Peric Stoini comes on for Oakmere Rangers in place of Ninis Palova." and "1' Substitution applied — Oakmere Rangers".
- Result: pass.

**AC-d — with the limit used, another substitution shows the engine's rejection reason**
- Platform & tool: web, headless Edge; `drive-live.mjs`, seed 42, 8x.
- Steps: pause playback (the engine holds at rendered 6039 + 500 = 6539); queue four substitutions; resume; wait for all four; queue a sixth (Mador Erker off, Nisef Lomton on).
- Evidence: `ac-d-1-four-queued.png`, `ac-d-2-five-used.png`, `ac-d-3-sixth-rejected.png`, `drive-live.json`, `server-live.log` (`signal="change.rejected" ... reason=substitution limit reached (5 of 5)`).
- Observation: all four were queued at tick 6539 and applied together at 17797 (one window). The count reads "0 of 5 substitutions left". The sixth was queued at 18319 and refused by the engine at the stoppage at 19767. The chip reads "Rejected — substitution limit reached (5 of 5)" with a Dismiss button, and the feed reads "6' Change rejected: substitution limit reached (5 of 5) — Oakmere Rangers".
- Result: pass.

**AC-e — a role changed while paused applies at the next dead ball, not at resume**
- Platform & tool: web, headless Edge; `drive-live.mjs`, seed 42, 8x.
- Steps: pause at rendered tick 19913; open "Roles and duties"; on slot 1 (Faric Morberg) press ArrowDown (Full back to Wing back); wait 4 s while paused; resume; read `queued_tick`, `applied_tick`, and the stoppage index.
- Evidence: `ac-e-1-queued-while-paused.png`, `ac-e-2-applied.png`, `drive-live.json`.
- Observation: while paused the engine held at 20413 (500 past the pitch) and did not move for 4 s. The chip stayed "Queued" with no verdict. The change was queued at 20413, when the engine resumed. It applied at 44545. That is the first stoppage after the pause frame and the first after the queue tick: this seed has no stoppage between minute 7 and minute 14. It is not the resume tick. The feed's newest visible row reads "14' Tactical change applied — Oakmere Rangers", and the role now reads Wing back (role 4, duty 1). Playback stayed at 8x with no lag notice.
- Result: pass.

**AC-f — keyboard-only operation reaches and operates every slot, control, and picker with visible focus** (annotated `observable: false`)
- Platform & tool: web, headless Edge; `drive-lineup.mjs` with Tab, Shift+Tab, Enter, Space, Delete, and arrow keys only.
- Evidence: `ac-f-1-focus-slot-4.png` to `ac-f-6-live-keyboard.png`, `drive-lineup.json` (`tab-coverage-pre-match`, `tab-coverage-live`).
- Observation: before kick-off the Tab cycle reached 59 targets: all 11 slots, 7 bench places, 22 squad players, formation, kick-off, playback controls, the feed, the mentality and 6 instruction selects, and the roles summary. Each showed `:focus-visible` with a 2 px solid ring and 0 missed rings. Left out: the reason paragraph and the count span, which are not controls, and the 22 role and duty selects inside the closed roles list. Enter opened that list, and the next Tab reached `role-0`. Delete emptied slot 4. Enter on the squad player and then Enter on the slot put him back. Space swapped two slots and swapped them back. The arrow keys changed the formation and back. Enter on kick-off started the match. Live, the Tab cycle reached 42 of 42 enabled controls, including `sub-off`, `sub-on`, and `sub-queue`, with 0 missed rings. Enter on "Queue substitution" and ArrowDown on Mentality each queued a change.
- Result: pass.

## Acceptance Criteria Status

| Criterion | Kind | Status | Method | Evidence | Evidence-rung |
|---|---|---|---|---|---|
| AC-a: fewer than eleven, no goalkeeper, or a player placed twice disables kick-off with the reason inline; a legal lineup enables it | user-observable | met | interactive | `ac-a-1..4-*.png`, `drive-lineup.json` | headless |
| AC-b: a mentality change shows a pending chip; at the next dead ball the chip clears and the feed shows "Tactical change applied" | user-observable | met (after fix VER-1) | interactive | `ac-b-*.png`, `drive-live.json`, `drive-timing-8x-after.json`, `timing-superseded/` | headless |
| AC-c: a substitution applies at the next dead ball, the lineup panel swaps the players, the remaining count decrements | user-observable | met | interactive | `ac-c-*.png`, `drive-live.json` | headless |
| AC-d: with the limit used, another substitution shows the engine's rejection reason | user-observable | met | interactive | `ac-d-*.png`, `drive-live.json`, `server-live.log` | headless |
| AC-e: a role changed while paused applies at the next dead ball, not at resume | user-observable | met | interactive | `ac-e-*.png`, `drive-live.json` | headless |
| AC-f: keyboard-only operation reaches and operates every slot, control, and picker with visible focus | code-only (annotated `observable: false`) | met | interactive (keyboard drive) | `ac-f-*.png`, `drive-lineup.json` | n-a |

Evidence: headless 5 / n-a 1. No user-observable AC rests on a mock or static rung (`metric-acceptance-mock-rung` contribution 0). No deferral.

## Issues Found

None open. Two issues were found at the gate and fixed in this round (see Verify-Owned Fixes):
- high: VER-1 — at 8x a change made during play applied one or more dead balls after the next one on screen (engine 1,848 to 4,627 ticks ahead; PO bound 500), failing AC-b's "at the next dead ball". Fixed in `d47e7d3`.
- medium: VER-2 — the match feed stopped following the newest row after a pending chip appeared or the roles list opened, so a "Tactical change applied" row could be out of view. This is a cross-slice regression of the match-day feed, caused by this slice's tactics column. Fixed in `8a5a182`.

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| VER-1 | unmet-ac (AC-b; AC-e timing under page pacing) | Fix | Patched (`d47e7d3`) | `crates/stream/src/control.rs` `a_reported_drawn_tick_bounds_the_lead_and_a_silent_client_is_not_held`; `crates/engine-cli/src/stream_run.rs` `a_reported_drawn_tick_holds_the_engine_within_the_lead_bound`; `web/tests/lead.test.mjs` "the drawn tick is reported at most once per interval, and only when it moved" | Pass (8x 6/6 and 1x 3/3 at the next visible dead ball; lead ≤ 500) |
| VER-2 | check-failure (incidental defect, cross-slice feed follow) | Fix | Patched (`8a5a182`) | `web/tests/feed.test.mjs` "the feed keeps following the newest row when a panel beside it shrinks it" | Pass (`probe-feed-sub-late-after.json`, `probe-feed-sub-late-roles-after.json`, `ac-e-2-applied.png`) |
| VER-1a | check-failure (regression caused by the VER-1 patch: the lag notice locked playback at 3x) | Fix | Patched (`8a5a182`) | `web/tests/playback.test.mjs` "a slow arrival rate with a healthy lead is an engine held near the pitch, not lag" | Pass (live drive at 8x with no notice; `probe-lag-lag.json` still shows the notice at 3x) |

Commit: `d47e7d3` (engine lead bound and `seen`), `8a5a182` (feed follow and lag gate). The fix-loop default message names the workflow, and the output boundary forbids that. Both messages are in product language instead, and both passed a leak check. Each commit names only its own files, so another session's staged entries (`crates/engine/tests/zz_stall_probe.rs`, `docs/design/realism/*`) stayed staged and out of these commits.

Regression tests added: 5.

Method note: the fix loop normally dispatches one write-isolated fix sub-agent per issue. This run had no sub-agent dispatch tool, so the orchestrator applied each minimal patch itself. It then ran the same sanity check on each diff: the patch against the issue, a test first where re-runnable, and no existing test weakened.

## Triage Decisions

- VER-1 → Fix (autonomous policy: every fixable issue is Fix). A page-side retune of `web/lead.mjs` thresholds was rejected. The engine simulates about 670,000 ticks a second, so the overshoot comes from how long a `pause` takes to arrive, and no page threshold can bound it. The chosen method honours the product owner's standing decision, "client-pull, bounded … 500 ticks" (shape Round 3 Q4 and Q10). The client reports its drawn tick, and the producer waits at `buffer_ticks` past it. The report is opt-in, so every existing client and test is unchanged.
- VER-2 → Fix. Minimal: the follow state now changes only on the manager's own scroll. The old rule re-derived it from geometry at each flush.
- VER-1a → Fix. The regression came from this round's own patch, so it was fixed in the same round rather than left to the next.

## Augmentation Verification

- Mock fidelity (`02c-craft.md` section 3), the items this slice touches:
  - Item 1, the 1280 by 800 grid: honored. `scrollWidth` 1280 and `scrollHeight` 800 both before kick-off and live (`drive-lineup.json` `layout-pre-match`; `drive-live.json` `layout-live`). The implement record's deviation still stands: before kick-off the editor borrows the pitch tile and the statistics region.
  - Item 6, type and tabular numbers: honored. Fit, fitness, and the count use `.tl-num`, and `web/components/chip.css` and `editor.css` declare no `font-family`.
  - Item 7, `--tl-` tokens with no pure black or white: honored. A search of both new stylesheets for hex, `rgb(`, `black`, and `white` found only `white-space`.
  - Item 9, state words beside state colours: honored. The chips read "Queued", "Applied", and "Rejected" in the DOM (`drive-live.json` `chipsDom`), and the lineup reason carries "Not ready:".
  - Item 10, no spinner: honored. Kick-off reads "Kicking off" while waiting, and there is no spinner element.
- Instrument (`04b-instrument.md`): the `tactics-change` event carries `change.applied_tick`, which the pending hook reads (`drive-live.json`). `change.rejected` fires in the engine log for the sixth substitution (`server-live.log`).
- Benchmark (`05c-benchmark.md`, compare against the match-rules baseline): 415.8 ms per match against the 460.7 ms gate and 418.8 ms baseline; 6.56 MB against the 6.82 MB gate. The stream delivered 613,223 ticks/s against the 609,220 baseline, with 7.88 MB against the 8.55 MB tripwire. No regression.
- Experiment (`04c-experiment.md`): `deferred-to-experiment-flags`; nothing to check.

## Security Scan

- CVE scan: skipped. `cargo audit` is not installed (`security-probe.txt`: "no such command: `audit`"). The fixes add no dependency.
- Secret detection: gitleaks 8.30.1, no leaks over the slice commits and the changed trees.
- SAST: none installed (no semgrep). A manual read of the new command path: `seen` takes a `u32` with `deny_unknown_fields`, and `saturating_add` guards the bound arithmetic, so an unknown field or an overflow is refused or held safely.

## Accessibility Gate

- Tool: a keyboard-only drive over the DevTools protocol, reading `document.activeElement`, `:focus-visible`, and the computed outline at each step.
- New WCAG AA violations: 0 found by that drive. 59 pre-match and 42 live focus targets, each with a 2 px solid ring; all actions operable by keyboard; chip state and lineup legality carried in words.
- Not run: no automated colour-contrast pass over the new chip and editor styles (viewer-match-day's custom contrast reader was not re-run). Recorded under Gaps.

## Performance Gate

- Bundle size: skipped; the page is served unbundled.
- Build time: release build about 7 s incremental; no material change.
- Engine cold start: unchanged. The new per-tick gate check takes the same lock `wait_until_running` already took, and the benchmark shows no change (415.8 against 418.8 ms per match).

## Cross-Slice Regression

- Siblings checked: `viewer-pitch` (page tests, and the lag criterion re-driven with `replay --sustain 3`), `viewer-match-day` (page tests, feed follow), and `stream-protocol`, `tactics-and-ai`, `engine-core`, `match-rules`, `commentary`, `calibration`, and `data-schemas-generator` (the full workspace test suite).
- Regressions found: 1, fixed.
  - `viewer-match-day` — feed follow: the feed stopped following once this slice's tactics column grew under it (VER-2); fixed in `8a5a182`.
- The VER-1 patch briefly regressed the `viewer-pitch` lag estimator on a held live engine (VER-1a). It was fixed in the same round, and the capped-replay lag notice still works.

## Longitudinal Delta

- Match screen live at 8x: compared against the viewer-match-day evidence run (`verify-evidence/viewer-match-day/ac3-lineups.png`). The right column now carries chips, the tactics panel, and the picker under the feed. The feed is shorter as a result. That is an expected change, and the follow fault it exposed is fixed.
- Pre-match: no baseline. This slice creates the screen.
- Pre-fix and post-fix live runs of this slice are kept side by side (`before-fix-*.png`, `run-after-seen-fix/`, `run-after-feed-fix-1/`, `run-after-feed-fix-2/`, then the final files at the top of the folder).

## Friction Notes

- With the roles list open, the feed shrinks to about two rows at 1280 by 800 (`ac-e-2-applied.png`). It still follows the newest row, but little history is visible. Informational; a layout question for review.
- The substitution picker stays enabled at "0 of 5 substitutions left", and the engine then refuses the change. This is by design (the page never predicts a verdict, and AC-d depends on it), but a manager may expect the button to disable. Informational.
- Before kick-off the 4-4-2 board places a player's name in a truncated tile ("Sador …"). The full name is in the tile's accessible name and in the squad list. Informational.

## Free Exploration Notes

- Seed 42 has no stoppage between minute 7 and minute 14 (ticks about 21,600 to 44,545), so a change made in that stretch waits 8 minutes of match. That is engine realism, not this slice — informational.
- A hidden or throttled tab now holds a live engine within 500 ticks of its last drawn tick, because the page reports only what it draws. This is intended (the manager is not watching), but a tab left in the background pauses the match — informational, escalate only if review disagrees.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Empty submission (kick-off clicked with an illegal lineup) | pass | The phase stays `pre-match`; the control is disabled and the reason stays. |
| Max-length input (the full bench of 7 named, the full squad of 22 in the list) | pass | The layout holds at 1280 by 800 with no page scroll. |
| Double-click / rapid repeat (four substitutions queued in about 1 s while held) | pass | Four distinct chips, all applied at one stoppage (one window). |
| Mid-flow interruption (a change queued while paused, a 4 s wait, then resume) | pass | The engine held; the change applied at the next stoppage, not on resume. |
| Offline / network failure | n-a | The socket-close path belongs to `viewer-reports-recovery`. A queue while disconnected shows a Rejected chip, "The engine is not connected." (`web/main.mjs` `queue`); not driven. |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Slow response (`replay --sustain 3` at 8x) | pass | The lag notice shows 3x in words and digits; no false notice on a healthy live match after `8a5a182`. |
| Concurrent session | n-a | `serve` accepts one client per match by design (single-player v1). |
| Session expiry mid-flow | n-a | No sessions or accounts in v1. |
| Paused playback holds the engine (new with `seen`) | pass | Held at exactly pitch + 500 for 4 s (`drive-live.json` `ac-e-4s-later-still-paused`). |

## Cross-Browser Delta

- Primary: Microsoft Edge 153 (Chromium), headless.
- Secondary: none available. Probe: `ls "C:/Program Files/Mozilla Firefox/firefox.exe"` → "No such file or directory". Playwright is on the PO's `toolchains-absent` list, so no Firefox or WebKit engine exists here. Google Chrome is present but is the same engine.
- Divergences found: none (only one engine could be driven).

## Web Vitals

Not measured: the page is a local single-page viewer with no navigation after load, and the slice's timing criteria were measured in engine ticks. LCP, CLS, and INP are `null`.

## Gaps / Unverified Areas

- No automated colour-contrast pass over `web/components/chip.css` and `editor.css`; only the keyboard and focus half of the accessibility gate ran.
- No second browser engine.
- The disconnected-queue Rejected chip was read in code, not driven.
- The throttled or hidden tab now holding the engine is reasoned from the design and the unit tests, not driven with a hidden tab.

## Freshness Research

Not required beyond the implement record. Every check passed after the fix, the plan is dated 2026-09-22 (one day old), and the fixes add no external API. The one external fact the fix relies on is that the engine simulates far faster than a round trip. That was measured here, not assumed: 676,077 ticks/s in `bench-cpu.stdout.txt`.

## Recommendation

Proceed to review. All six criteria are met on headless evidence against the real engine. The fix round converged, and the three patches carry five regression tests. Review should look at the new `seen` command on protocol version 3 and the hidden-tab hold it implies.

## Recommended Next Stage

- **Option A (default):** `/wf review football-manager-match-engine viewer-lineup-tactics`. Convergence `converged`, `result: pass`. Compacting first is recommended, because this run carried long drive output.
- **Option D:** `/wf handoff football-manager-match-engine viewer-lineup-tactics`. Only with a clear reason to skip review. Not recommended here, because the fix added a protocol command.
- **Option G:** `/wf probe football-manager-match-engine`. A slug-wide runtime sweep, useful once `viewer-reports-recovery` lands, since it inherits the `Dugout` phases and the new hold.
