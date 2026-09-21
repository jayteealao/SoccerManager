---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: viewer-pitch
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: m
depends-on: [stream-protocol]
tags: [viewer, canvas, playback, milestone]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-stream-protocol.md, 03-slice-viewer-match-day.md, 03-slice-viewer-lineup-tactics.md, 03-slice-viewer-reports-recovery.md, 03-slice-integration.md]
  plan: 04-plan-viewer-pitch.md
  implement: 05-implement-viewer-pitch.md
---

# Slice: Viewer Pitch and Playback

## The Slice

The protocol slice delivers a live stream and a recorded fixture. The design brief weights live play as the core of the experience, and the product owner wants crude play on screen as early as slice 3 or 4.

This slice draws the pitch, 22 markers, and the ball on a canvas at 60 frames per second with tick-bounded interpolation, and adds the playback controls: speed 1x to 8x, pause, skip to the next stoppage, and rewind over a whole-match compact history. It also carries the lag notice. It verifies first against the fixture through the mock server, then against the live engine.

This is the visible-milestone slice: the charter scenario executes through step 5 here. The top risk is frame pacing at 8x, where 400 ticks per second arrive for 60 frames; the viewer must skip ticks, never extrapolate.

## Goal

A page that shows the match moving, driven by the stream, with every playback control from the shape.

## Why This Slice Exists

Commitment C2 is proven or broken here: the viewer must show what the engine computed and nothing else. Early feedback on feel and performance shapes every later viewer slice.

## Scope

In:
- Static HTML, CSS, and JavaScript page; canvas pitch with markings; markers with shirt numbers; ball with a short trail.
- Socket client with a small playback buffer (named mechanism per the shape: **live tick stream with a viewer buffer**).
- Tick-bounded interpolation (named mechanism per the shape; RIM-5).
- Playback controls: speed 1x to 8x, pause and resume, skip to next stoppage, rewind scrubber.
- Whole-match compact history in memory (named mechanism per the shape).
- Lag notice: drop to the sustained speed and show it.
- A frame-time counter and a test hook that exposes the last rendered positions.
- Design-brief live-play state; the score bug and goal banner come in `viewer-match-day`.

Out:
- Header, feeds, lineups, statistics: `viewer-match-day`.
- Pre-match and tactics panels: `viewer-lineup-tactics`.
- Reports, loading, error, recovery: `viewer-reports-recovery`.

## Acceptance Criteria

- Given the mock server replays the fixture at 1x, When the page plays for 5 minutes of match time, Then the frame-time counter reports 60 frames per second with zero dropped ticks.
  <!-- observable: true — a person sees smooth motion; the counter makes it measurable -->
  verify: { method: in-app browser drive with screenshot and console read of the counter (Playwright later), env: Windows 11 with Node 22 (installed) and the mock server built by stream-protocol, fixture: fixture.bin from seed 7, rung: web-2 }
- Given two consecutive ticks, When the test hook reads the rendered positions, Then every marker lies on the straight segment between the two tick positions.
  <!-- observable: false — a JavaScript unit test over the interpolation function and the hook -->
- Given playback at 8x, Then the viewer skips ticks and never renders a position beyond the next received tick.
  <!-- observable: false — unit test over the frame scheduler with a synthetic stream -->
- Given the manager sets 4x, When play continues, Then the on-screen clock advances four times faster than wall time within 2 percent.
  <!-- observable: true — visible clock behavior -->
  verify: { method: in-app browser drive reading the clock at two wall times, env: as above, fixture: fixture.bin, rung: web-2 }
- Given the mock server sustains only 3x, When the manager selects 8x, Then playback drops to 3x and a notice names 3x; and Given the mock server sustains 8x, Then no notice shows.
  <!-- observable: true — the notice is a user-visible surface -->
  verify: { method: in-app browser drive with the mock server's rate cap flag, env: as above, fixture: fixture.bin, rung: web-2 }
- Given the manager rewinds to a stored tick, Then the frame drawn equals the stored positions for that tick.
  <!-- observable: true — visible scrubber and frame -->
  verify: { method: in-app browser drive plus the test hook, env: as above, fixture: fixture.bin, rung: web-2 }
- Given a full match at 50 ticks per second, Then the stored history stays under 300 MB as reported by the page's memory gauge.
  <!-- observable: false — a headless JavaScript test loads the fixture and reads the gauge -->
- Standing charter-scenario AC: Given the live engine (not the mock), When the charter scenario runs, Then steps 1, 4, and 5 execute with their checkpoints; steps 2 and 3 use a default lineup until `viewer-lineup-tactics` ships.
  <!-- observable: true — the visible milestone -->
  verify: { method: in-app browser drive against engine-cli with the socket on, env: Windows 11 with the engine built, fixture: seed 42 default teams, rung: web-2 }

## Dependencies on Other Slices

- `stream-protocol`: the socket client contract, the fixture, and the mock server.

## Risks

- Canvas on the main thread stutters at 8x with the feed rendering later: keep rendering in a worker with OffscreenCanvas where available (design brief `optimize.md`).
- Memory gauge over 300 MB: tighten the compact encoding; the shape's edge case keeps the last 45 minutes as a fallback with a warning.
