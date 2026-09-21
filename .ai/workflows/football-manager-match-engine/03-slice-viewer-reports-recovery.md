---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: viewer-reports-recovery
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: m
depends-on: [viewer-match-day, match-rules]
tags: [viewer, reports, replay, recovery, error-states]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-viewer-match-day.md, 03-slice-match-rules.md, 03-slice-viewer-lineup-tactics.md, 03-slice-integration.md]
  plan: 04-plan-viewer-reports-recovery.md
  implement: 05-implement-viewer-reports-recovery.md
---

# Slice: Reports, Replay Files, and Recovery States

## The Slice

The match-day screen covers play. The product owner selected half-time and full-time reports, replay saving, and error and recovery as first-version states, and the design brief weights loading, error, and recovery. Match rules already write a snapshot at every stoppage.

This slice adds the two reports, saving and loading a replay file from the whole-match history, and the non-play surfaces: loading skeleton while the engine starts, first-run state when no engine is found, an error panel on crash or socket drop with restart from the last stoppage, and abandon. The two groups share one slice because both are the surfaces around play that consume the snapshot and replay files.

Integration proves everything end to end next. The top risk is the restart path: the page must relaunch or reconnect to the engine and resume from the snapshot without a second copy of the match state in the browser.

## Goal

The manager gets a report at each break, keeps a replay, and never loses a match to a crash.

## Why This Slice Exists

Round 3 Q11 selected the states; Round 4 Q17 chose restart from the last stoppage; Round 4 Q20 chose saved replays; the design brief weights the recovery state.

## Scope

In:
- Half-time and full-time reports: statistics and event summary; sketch quality acceptable per the design brief.
- Save replay at full time (the compact history to a file); load a replay into the viewer.
- Loading skeleton pitch while the engine starts; first-run state when the engine binary is not found, with the path shown.
- Error panel on engine crash or socket drop: names the failure, offers restart from the last stoppage or abandon.
- Restart path: relaunch or reconnect, send `resume` with the snapshot, continue from the stoppage (named mechanism: **restart-from-snapshot flow**, replaces abandon-on-error).

Out:
- Extra-time reports: `extra-time-penalties` (deferred).
- Installer and engine discovery beyond a configured path: `distribution` (deferred).

## Acceptance Criteria

- Given the engine process is killed mid-match, Then the page shows the failure and offers restart from the last stoppage; and Given restart, Then play resumes at that stoppage with the same score and clock.
  <!-- observable: true — the induced failure and the recovery are visible -->
  verify: { method: in-app browser drive with a scripted process kill and a recording (Playwright later), env: Windows 11 with the engine built with match-rules, fixture: seed 42, rung: web-2 }
- Given a socket drop without an engine crash, Then the page reconnects and resumes without a restart prompt.
  <!-- observable: true — visible continuity -->
  verify: { method: in-app browser drive with a scripted socket close, env: as above, fixture: seed 42, rung: web-2 }
- Given half-time, Then the report's statistics equal the counts of events in the feed.
  <!-- observable: true — visible report -->
  verify: { method: in-app browser drive with screenshot and DOM read, env: as above, fixture: fixture.bin, rung: web-2 }
- Given full time, When the manager saves the replay, Then a file is written; and Given the page loads that file, Then playback and rewind work with no engine running.
  <!-- observable: true — visible save and reload -->
  verify: { method: in-app browser drive with file download and reload, env: as above, fixture: fixture.bin, rung: web-2 }
- Given no engine binary at the configured path, Then the first-run state shows the path and instructions.
  <!-- observable: true — visible first-run surface -->
  verify: { method: in-app browser drive with the path misconfigured, env: as above, fixture: none, rung: web-2 }
- Given a corrupt snapshot on restart, Then the error panel names the corruption and offers abandon only.
  <!-- observable: false — a unit test over the error-panel model with the engine's corruption message -->

## Dependencies on Other Slices

- `viewer-match-day`: the page and panels the reports summarize.
- `match-rules`: snapshots and `resume`.

## Risks

- Restart duplicates state: the page discards its buffer beyond the snapshot tick and trusts the resumed stream.
- Large replay files: compact encoding from `viewer-pitch`; the file is the same encoding.
