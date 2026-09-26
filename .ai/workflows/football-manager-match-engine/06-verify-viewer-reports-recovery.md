---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: viewer-reports-recovery
status: complete
stage-number: 6
created-at: "2026-09-23T13:12:00Z"
updated-at: "2026-09-23T13:12:00Z"
result: pass
metric-checks-run: 10
metric-checks-passed: 10
metric-acceptance-met: 6
metric-acceptance-total: 6
metric-acceptance-user-observable: 5
metric-acceptance-code-only: 1
metric-interactive-checks-run: 8
metric-interactive-checks-passed: 8
metric-issues-found: 0
metric-issues-found-initial: 0
metric-issues-found-final: 0
fix-rounds-run: 0
convergence: not-needed
verify-owned-fix-commit: null
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [web, cli]
bootstrap-failures:
  - {adapter: web, step: "in-app browser pane drive", remediation: "Not used. The pane throttles animation frames while hidden (viewer-pitch and viewer-match-day runs), and a kill drive needs the page drawing at 8x. Every drive ran in headless Microsoft Edge over the DevTools protocol with the harness at verify-evidence/viewer-reports-recovery/cdp.mjs and drive.mjs, against the real release engine-cli."}
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/viewer-reports-recovery/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: pass
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped — the page is served unbundled; no bundler exists"
ac-staleness-checked: true
ac-stale-count: 0
longitudinal-baseline-compared: "skipped — first verify run of this slice; every surface it checks (step list, first-run and error panels, report dialog, replay open) is new"
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
tags: [viewer, reports, replay, recovery, launcher, reconnect, snapshots, headless-edge]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-viewer-reports-recovery.md
  plan: 04-plan-viewer-reports-recovery.md
  implement: 05-implement-viewer-reports-recovery.md
  review: 07-review-viewer-reports-recovery.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine viewer-reports-recovery"
---

# Verify: Reports, Replay Files, and Recovery States

## The Verification

Implement left the launcher, the reconnect path, the two reports, and the replay file on commits `b03163d` and `aa10882`, with 305 Rust tests and 126 page tests green. It named one open risk: no browser drive had yet read the header score against the event list after a restart that crosses a goal. This run drove all five browser criteria in headless Microsoft Edge at 1280 by 800 against the release `engine-cli`, with a fresh data folder per drive. The kill drive ran twice on two live seed-42 matches. The first killed the worker at 29:33 with the score 0–0. The second waited for a goal (1–0 at tick 135542), then killed the worker after the stoppage at 47:04. Both times the panel read "The engine stopped (exit code 1)" with "Restart from mm:ss" and "Abandon". Restart resumed at exactly the snapshot tick, with the clock at that minute, the score equal to the last event at or before it, no duplicate events, and play moving on.

The other drives agree with the criteria. An injected drop at tick 3006 reconnected in 270 ms and resumed at 2895. Across 91 samples at 100 ms, no panel and no Restart button appeared. The half-time report of the fixture matched a separate per-club count of the event list at tick 148650 in all 10 rows, and the row totals matched the feed rows. The full-time save wrote 20,460,598 bytes. They are byte-identical to `fixture.smfx` (same SHA-256). The engine's own `replay` reader played the file with exit 0, and it played and rewound exactly (`exact: true` at ticks 2000, 100000, and 50) in a page whose launcher had no engine. The first-run panel showed `C:/missing/engine-cli.exe` and the build instruction. A corrupted snapshot on restart gave "The saved match could not be read: checksum mismatch: the file is corrupt" with Abandon only. No issue reached the fix loop: 10 of 10 checks and 6 of 6 criteria pass, so convergence is not-needed.

Review can now read a recovery path proven against a real process kill, including a score that crosses a goal. The top open risk is one the build names: a change the manager queued before a crash may keep its "Queued" chip after the restart. No drive queued a change before a kill, so this path has no evidence either way.

## Verification Summary

- Slice: `viewer-reports-recovery`, standard mode, branch `feat/football-manager-match-engine` (dedicated; confirmed with `git branch --show-current`), head `db33cd3`.
- Stack: `user-confirmed: true` in `00-index.md`; the plan carries `stack-source: confirmed`.
- Constraint resolution: every user-observable criterion in the plan's Verification Strategy carries `constraint-resolution: prerequisite-slice: viewer-reports-recovery` with `wall-ownership: code-owned`. The sixth criterion is `observable: false` with no environment need. `constraint-resolution-missing: []`.
- Re-run status: no earlier `06-verify-viewer-reports-recovery.md` existed, so this is the slice's first verify run (`evidence-run-count: 1`). The run was dispatched as autonomous fix round 1; no fix was needed.
- Checks: 10 run, 10 pass.
- Acceptance: 6 of 6 met. 5 user-observable at `evidence-rung: headless`, 1 code-only met by unit and launcher tests plus a live drive.
- Fix loop: 0 issues at the gate, 0 rounds, no commit.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (`fmt.txt`, empty output, exit 0).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass (`clippy.txt`, exit 0).
- `cargo build --release -p engine-cli`: pass (`build-release.txt`).
- `cargo test --workspace`: pass, 305 passed, 0 failed, 4 ignored (`cargo-test.txt`; summed over every `test result` line).
- `node --test "web/tests/*.test.mjs"`: pass, 126 of 126 (`node-test.txt`).
- `gitleaks detect --log-opts="db33cd3~3..db33cd3"`: pass, 3 commits scanned, no leaks (`gitleaks.txt`).
- Launcher action origin probe (curl against a live `launch`): pass. A foreign origin gets 403, no origin gets 403, GET on the action gets 404, and a same-origin abandon gets 202 (`security-probe.txt`).
- Processor-time benchmark, `engine-cli bench --seed 42 --matches 5 --json`, three drives: pass. 418.8, 415.8, and 409.4 ms per match (median 415.8 against the 460.7 ms gate); 6.45 MB peak (against 6.82 MB); 1.4713 µs per tick median (`bench-cpu-{1,2,3}.json`).
- Stream benchmark, `engine-cli bench --seed 42 --matches 1 --stream --json`, three drives: pass. 611,115, 612,668, and 609,930 ticks/s (median 611,115; baseline 609,220); 7.96, 7.99, and 7.88 MB (under the 8.55 MB stream tripwire) (`bench-stream-{1,2,3}.json`).
- Saved replay through the engine's reader, `engine-cli replay --fixture saved.smfx --speed 1000` with one client: pass. 11,856 text frames plus 292,900 tick frames (304,756, the page's count), first message `hello`, engine exit 0, `fixture.replayed ... hash=b3a7575432cf` (`replay-saved-file.txt`, `replay-client.mjs`).

## Interactive Verification Results

All drives use `verify-evidence/viewer-reports-recovery/drive.mjs <scenario>` over `cdp.mjs`: headless Edge, 1280 by 800, downloads denied (the saved bytes come through the read-only hook, per plan A-10), and every page signal captured from the console. Each drive writes `drive-<scenario>.json`, its stdout, and the engine log `server-<scenario>.log`.

**AC-a: engine killed mid-match, then restart**
- **Platform & tool:** web, headless Edge over the DevTools protocol; `taskkill /F /PID <engine.pid>` from `/engine.json`.
- **Steps performed:** `launch --web web --seed 42`; Kick off; 8x. Waited until `snapshot.tick` was on disk and the page had drawn 250 ticks past it. For the second drive (`kill-restart-goal`) the wait also required a goal before the snapshot tick, and the drive clicked Continue on the half-time report. Recorded the score, the clock, and `__touchline.events()`. Killed the worker. Screenshots at t0, t250, and the panel. Clicked Restart. Read the `viewer.resumed` signal, the clock, the score, and `/engine.json`, then read again 3 s later.
- **Evidence:** `ac-a-1-t0.png` to `ac-a-5-playing-on.png`, `drive-kill-restart.json`; `ac-a-goal-1-t0.png` to `ac-a-goal-5-playing-on.png`, `drive-kill-restart-goal.json`.
- **Observation:** Drive 1: the kill came at 29:33 with the score 0–0. The panel showed the "Error" word, the title "The engine stopped (exit code 1)", "The match was saved at the stoppage at 29:33.", and the buttons "Restart from 29:33" and "Abandon". After Restart, `viewer.resumed` read `to_tick` 88660, equal to `snapshot.tick`. The clock read 29:33, the score 0–0, and the new process id was 38440 (the old one was 40192). The page drew 88693 → 89940 in 3 s, with 0 duplicate events. Drive 2: the goal came at tick 135542 (1–0), and the kill came after the stoppage at 141200 (47:04). The panel showed "Restart from 47:04". After Restart, `to_tick` was 141200, the clock 47:04, and the score 1–0, which equals the last event at or before the tick. The page drew 141235 → 142478 in 3 s. The event list up to the drawn tick was the same before and after the kill, with 0 duplicates.
- **Result:** pass (two independent drives, same outcome).

**AC-b: socket drop without an engine crash**
- **Platform & tool:** web, headless Edge; the launcher's hidden `--drop-client-at 3000`.
- **Steps performed:** `launch --web web --seed 42 --drop-client-at 3000`; Kick off; 8x. Read the page state every 100 ms from kick-off to 3 s after `viewer.resumed`: panel kind, reconnecting flag, Restart visibility, and the drawn tick.
- **Evidence:** `ac-b-1-reconnecting.png`, `ac-b-2-resumed.png`, `drive-socket-drop.json` (`samples`), `server-socket-drop.log`.
- **Observation:** The engine logged `socket.drop_injected tick=3006`, `socket.dropped tick=3016 sent_tick=3006`, `events.resumed tick=2895 kept=3 dropped=1`, and `socket.reconnected resume_tick=2895`. The page logged `viewer.reconnecting {attempt 1, delay_ms 250, dropped: true}` and `viewer.resumed {from_tick 3006, to_tick 2895, gap 0}` 270 ms after the drop. The notice read "Reconnecting — The connection to the engine dropped. Reconnecting." In 91 samples the panel kind was always null and Restart never appeared. After the resume the page drew 2696 → 3883, and the engine stayed `running`.
- **Result:** pass.

**AC-c: half-time report statistics equal the counts of events in the feed**
- **Platform & tool:** web, headless Edge; `engine-cli replay --fixture fixture.smfx --speed 8 --web web`.
- **Steps performed:** Played the fixture at 8x until the half-time report opened on its own. Read every `data-count` cell and its text. Counted the events of each type (and card kind) per club at or before the report tick, separately from the page's own model. Counted the feed rows by `data-kind` at or before the tick.
- **Evidence:** `ac-c-half-time-report.png`, `drive-report-replay.json` (step `half-time`).
- **Observation:** The report opened at tick 148650, the tick of the half-time event, and read "Oakmere Rangers 2 – 0 Eldstead City". The table rows were goals 2–0, yellow cards 2–0, red 0–0, fouls 5–2, corners 0–0, offsides 0–7, throw-ins 9–5, goal kicks 1–0, free kicks 8–3, and penalties 0–0. Every cell equals the separate per-club count of the event list, and every text equals its `data-count`. Each row total equals the feed rows of that kind, and the card rows total 2, equal to the feed's 2 card rows.
- **Result:** pass.

**AC-d: full time, save the replay; load it with no engine; play and rewind**
- **Platform & tool:** web, headless Edge; the save through the full-time report's Save replay; the load through `DOM.setFileInputFiles` on the page's own `<input type="file">` (the file-picker path); the engine's `replay` reader as the byte check.
- **Steps performed:** Continued the fixture drive past half-time to the full-time report and clicked Save replay. Read the bytes from `__touchline.lastSavedBytes()` in 1 MB chunks and wrote `saved.smfx`. Played `saved.smfx` through `engine-cli replay`. Started `launch --web web --seed 42 --engine C:/missing/engine-cli.exe`. Listed every `engine-cli.exe` process with its parent id: this launcher had no child. Opened the page, chose `saved.smfx` in the file input, selected 8x, read the drawn tick over 3 s, then dragged the scrub bar to ticks 2000, 100000, and 50.
- **Evidence:** `ac-d-1-full-time-report.png`, `ac-d-2-saved.png`, `ac-d-3-no-engine.png`, `ac-d-4-playing-loaded.png`, `ac-d-5-rewound.png`, `drive-report-replay.json` (steps `full-time`, `saved`, `saved-file`), `drive-load.json`, `replay-saved-file.txt`.
- **Observation:** The full-time report opened at tick 292900 (97:38, 2–1). `viewer.replay_saved` read 20,460,598 bytes, 304,756 frames, and 292,900 ticks. The file on disk is the same size and SHA-256 (`322184d8…`) as `fixture.smfx`, so it is byte-identical. The engine's reader played it with exit 0. In the page with no engine (`engine.state: not-found`, no child process of launcher 36680), `viewer.replay_loaded` read 304,756 frames. The drawn tick went 3 → 1203 in 3 s, and each scrub gave `lastRewind().exact === true` at 2000 (clock 00:42 after 300 ms of play), 100000 (33:22), and 50 (00:03).
- **Result:** pass.

**AC-e: no engine at the configured path shows the first-run state**
- **Platform & tool:** web, headless Edge; `launch --web web --seed 42 --engine C:/missing/engine-cli.exe`.
- **Steps performed:** Read `/engine.json`, opened the page, read the panel DOM and `recovery()`, took a screenshot.
- **Evidence:** `ac-e-first-run.png`, `drive-first-run.json`, `server-first-run.log` (`launch.not_found`).
- **Observation:** `engine.state` was `not-found`. The panel showed the "Setup" word, the title "Touchline could not find the match engine", "The launcher looked for the engine program here:", a visible `<code>` that reads `C:/missing/engine-cli.exe`, and the instruction 'Build it with "cargo build --release -p engine-cli", then start the launcher again, or point it at the program with --engine.' The only action was "Open a replay". `recovery().kind` was `first-run`.
- **Result:** pass.

**AC-f (code-only, driven as well): corrupt snapshot on restart**
- **Platform & tool:** web, headless Edge; the corruption was 48 bytes flipped in `snapshot.smsn` after the kill and before Restart.
- **Evidence:** `corrupt-1-t0.png` to `corrupt-4-refused.png`, `drive-corrupt.json`, `server-corrupt.log` (`snapshot.refused`, `launch.refused`).
- **Observation:** The panel read "The saved match could not be read: checksum mismatch: the file is corrupt" with "Abandon" as the only button. `engine.reason` carried the engine's full `snapshot refused: <path>: checksum mismatch: the file is corrupt`.
- **Result:** pass.

Harness runs superseded (not product failures; kept for audit): `drive-kill-restart-goal-run1-held.*` waited forever because the drive did not click Continue on the half-time report. The report holds playback by design, and the lead bound then holds the engine. The drive was fixed to click Continue and re-run. `drive-load-run1-confounded.*` returned FAIL only because its "no engine" check matched a `serve` process that belonged to the concurrent goal drive's own launcher. All the replay criteria in that run held. The check was scoped to the launcher's own children and re-run.

## Acceptance Criteria Status

| AC | Criterion | kind | status | method | evidence | evidence-rung |
|---|---|---|---|---|---|---|
| AC-a | Engine killed mid-match → failure shown with restart from the last stoppage; restart → play resumes at that stoppage with the same score and clock | user-observable | met | interactive (2 drives) | `ac-a-*.png`, `ac-a-goal-*.png`, `drive-kill-restart*.json` | headless |
| AC-b | Socket drop without a crash → reconnects and resumes with no restart prompt | user-observable | met | interactive | `ac-b-*.png`, `drive-socket-drop.json`, `server-socket-drop.log` | headless |
| AC-c | Half-time report statistics equal the counts of events in the feed | user-observable | met | interactive | `ac-c-half-time-report.png`, `drive-report-replay.json` | headless |
| AC-d | Full time: save writes a file; the page loads it and plays and rewinds with no engine running | user-observable | met | interactive + engine reader | `ac-d-*.png`, `drive-report-replay.json`, `drive-load.json`, `replay-saved-file.txt`, `saved.smfx` | headless |
| AC-e | No engine binary at the configured path → first-run state shows the path and instructions | user-observable | met | interactive | `ac-e-first-run.png`, `drive-first-run.json` | headless |
| AC-f | Corrupt snapshot on restart → error panel names the corruption and offers abandon only (`observable: false`) | code-only | met | automated (`web/tests/recovery.test.mjs`, `crates/engine-cli/tests/launch.rs` corrupt case) + live drive | `node-test.txt`, `cargo-test.txt`, `drive-corrupt.json` | n-a |

Evidence: headless 5 / n-a 1. `metric-acceptance-mock-rung: 0`. No mock or fixture emulates an external interface: `fixture.smfx` is the engine's own recording of seed 42, and every drive ran the real engine.

## Issues Found

None.

## Augmentation Verification

- Mock fidelity (`02c-craft.md`), the items this slice owns or touches:
  - Item 6, Barlow Condensed for titles and tabular figures for numbers: honored. `web/components/surfaces.css:76` and `:137` use `var(--tl-font-display)`, and `web/report.mjs:121` and `:139` set `tl-num`. The screenshots `ac-c-half-time-report.png` and `ac-a-3-panel.png` show the condensed display face on the report and panel titles.
  - Item 7, `--tl-` tokens with no pure black or white: honored. A search of `web/components/surfaces.css` for hex, `rgb(`, `black`, and `white` found none. The one `font-family` (line 99) is the system monospace stack for the engine path. It is a face, not a colour, and loads no font service.
  - Item 9, a state word beside every state colour: honored. "Error" on the crash and refusal panels, "Setup" on first run, and "Reconnecting" beside the warning notice (`ac-b-1-reconnecting.png`).
  - Item 10, a skeleton and step list, never a spinner: honored. `surfaces.css` has no animation, and `web/index.html:59` records the rule. The step list reached step 3 (all Done) in every live drive (`recovery().step`).
- Instrument (`04b-instrument.md` and the build's signal list): fired in the drives were `socket.drop_injected`, `socket.dropped`, `socket.reconnected`, `events.resumed`, `match.resumed`, `snapshot.refused`, `launch.worker_started`, `launch.worker_exited`, `launch.restart`, `launch.refused`, `launch.not_found`, `launch.abandoned`, and `web.action_refused` (engine logs and `.data/secprobe/out.txt`), plus `viewer.reconnecting`, `viewer.resumed`, `viewer.recovery_panel`, `viewer.report_shown`, `viewer.replay_saved`, `viewer.replay_loaded`, and `viewer.socket_drops` (page console). Not exercised: `viewer.resume_gap` (no drive had a gap), `viewer.replay_refused` (no bad file was opened), and `snapshot.written` (not in the engine logs at the default level; the snapshot tick advanced in `/engine.json`).
- Benchmark (`05c-benchmark.md` compare): 415.8 ms per match against the 460.7 ms gate and the 418.8 ms baseline; 6.45 MB against 6.82 MB. The stream ran 611,115 ticks/s against the 609,220 baseline, at 7.96 MB against the 8.55 MB tripwire. No tripwire fired.
- Experiment (`04c-experiment.md`): `deferred-to-experiment-flags`; nothing to check.

## Security Scan

- CVE scan: skipped. `cargo audit` is not installed (the sibling slice's probe recorded "no such command: `audit`"). This slice adds no dependency: neither `Cargo.lock` nor any `Cargo.toml` is in the diff of `b03163d~1..db33cd3`.
- Secret detection: gitleaks, 3 commits (`b03163d`, `aa10882`, `db33cd3`), no leaks.
- SAST: none installed. A live probe of the new side-effect routes: `POST /engine/restart` from a foreign origin and with no origin both return 403 and log `web.action_refused`. GET on an action returns 404. A same-origin POST abandon returns 202. The page never sends the match state: restart carries no body.

## Accessibility Gate

- Tool: DevTools-protocol drive (`drive.mjs load`). It computes the WCAG contrast of every visible text node, resolving the OKLCH tokens through a canvas, and it reads `document.activeElement`, `:focus-visible`, and the computed outline after each real Tab key.
- First-run panel: 6 text nodes, lowest contrast 6.39:1, none under its threshold. Tab reached "Open a replay" first, with `:focus-visible` and a 2 px solid ring.
- Half-time report dialog: 48 text nodes, lowest contrast 4.61:1, none under 4.5:1. Focus opened on "Continue" with a 2 px solid ring. Tab left the modal to the document once (Chromium's modal-dialog cycle through the browser frame), then returned inside the dialog. Escape closed the report and resumed playback (`playingAfterEscape: true`).
- New WCAG AA violations: 0.

## Performance Gate

- Bundle size: skipped; the page is served unbundled.
- Build time: the release build finished in about 10 s incremental.
- Engine cold start: not timed separately. The per-match benchmark is unchanged (415.8 against 418.8 ms per match).

## Cross-Slice Regression

- Siblings checked: every slice that shares files with this one (`viewer-pitch`, `viewer-match-day`, and `viewer-lineup-tactics` for `web/main.mjs`, `index.html`, `socket.mjs`, `history.mjs`, `stoppages.mjs`, and `match-state.mjs`; `stream-protocol`, `match-rules`, and `viewer-lineup-tactics` for `session.rs`, `serve.rs`, and `web.rs`). The full workspace suite (305) and the full page suite (126) cover their tests, and both pass.
- In the live drives the lineup hold, Kick off, the 8x speed, the tactics panel, and the lead bound behaved as their slices left them.
- Regressions found: 0.

## Longitudinal Delta

- Skipped. This is the first verify run of the slice, and every surface it checks is new. The unchanged surfaces (pitch, feed, statistics, lineups) match the viewer-lineup-tactics evidence by eye in `ac-a-4-resumed.png`.

## Friction Notes

- After a crash the page keeps playing its stored ticks behind the error panel. The header clock read 32:06 while the panel offered "Restart from 29:33" (`ac-a-3-panel.png`). Restart then rewinds to 29:33. This is informational: the panel says where play resumes, but a paused pitch might read more clearly.
- The half-time report shows the second-half kick-off row ("45' Eldstead City kick off behind") in the feed behind the dialog, below the "45+5'" rows. Informational.

## Free Exploration Notes

- A loaded replay fires `viewer.change_unknown` for each of the computer manager's tactics changes (9 in the fixture), because this page never queued them. The panel is correct ("Changes need a live match"). The signal is noise on a replay — informational.
- `viewer.tick_skipped` fires about once a second at 8x. That is the expected cost of drawing 400 ticks/s at 60 fps — informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Corrupt snapshot, then Restart | pass | Refused by name, Abandon only (`drive-corrupt.json`) |
| Cross-site POST to restart (foreign origin; no origin) | pass | 403 both, `web.action_refused` logged (`security-probe.txt`) |
| GET on a side-effect route | pass | 404 (`security-probe.txt`) |
| Mid-flow interruption: the socket dropped mid-play | pass | Reconnected in 270 ms with no prompt (`drive-socket-drop.json`) |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Hard process kill (`taskkill /F`) of the engine mid-match, twice | pass | Panel within one poll; restart restores the tick, clock, and score |
| Network failure: an abnormal close (1006) injected by the engine | pass | Automatic reconnect, resume at the newest fully received stoppage |
| Slow response or session expiry | n-a | Loopback only, and no sessions or authentication |

## Cross-Browser Delta

- Primary: Microsoft Edge (Chromium), headless. Secondary: not run. Firefox and WebKit are not installed, and the stack names none. Divergences found: none observed.

## Web Vitals

- Not measured (null). The page is a local tool served on loopback. The frame budget gauge read 60 fps at 8x in every drive.

## Gaps / Unverified Areas

- The build's own risk: a change queued before a crash may keep its "Queued" chip after the restart. No drive queued a change before a kill.
- Re-drives: AC-a was driven twice on two independent matches. AC-b to AC-e were driven once each, and a single live run of AC-f was added. No differing outcome was seen.
- The in-app browser pane was not the driver. Headless Edge ran the same page against the same engine (see `bootstrap-failures`).
- Teardown: the drive profiles (`verify-evidence/viewer-reports-recovery/.profiles/`, about 1.2 GB) and data folders (`.data/`) are still on disk. The shell refused the removal in this run. They are not git-ignored and must stay out of any commit, as with the sibling slice's `.profiles/`. `saved.smfx` is ignored by the repository's `*.smfx` rule.

## Freshness Research

- No new dependency. The native behaviour the criteria depend on was observed directly this run: `taskkill /F` gives exit code 1; an injected abnormal close reaches the page as a non-clean close; `DOM.setFileInputFiles` fires the input's `change`; `crypto.subtle` works on loopback. The plan is 1 day old, and no criterion names an external API, so `ac-stale-count: 0`.

## Recommendation

Pass. All six criteria are met with direct evidence, and no check failed. Review can proceed. Two things are worth a look in review: the queued-change chip across a restart, and the playback that continues behind the error panel.

## Recommended Next Stage

- **Option A (default):** `/wf review football-manager-match-engine viewer-reports-recovery`. Verify passed with `convergence: not-needed`. Consider compacting first: the drive output is long and is noise for review.
- **Option D:** `/wf handoff football-manager-match-engine viewer-reports-recovery`. Valid with `result: pass`, but not recommended: the launcher adds side-effect routes and process supervision that deserve review.
- **Option G:** `/wf probe football-manager-match-engine`. A slug-wide runtime sweep, if a cross-slice pass is wanted before integration.
