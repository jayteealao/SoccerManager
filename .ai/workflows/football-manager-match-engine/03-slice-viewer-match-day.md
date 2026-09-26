---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: viewer-match-day
status: complete
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: m
depends-on: [viewer-pitch, stream-protocol, commentary]
tags: [viewer, panels, feed, statistics, goal-moment]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-viewer-pitch.md, 03-slice-stream-protocol.md, 03-slice-commentary.md, 03-slice-viewer-lineup-tactics.md, 03-slice-viewer-reports-recovery.md]
  plan: 04-plan-viewer-match-day.md
  implement: 05-implement-viewer-match-day.md
---

# Slice: Match-Day Panels and Goal Moments

## The Slice

Viewer pitch shows motion and controls. The product owner chose the full match-day screen: header with clock and score, event and commentary feed, both lineups with fatigue, and a live statistics panel; the design brief weights the goal moment.

This slice adds every panel around the pitch, the empty state at kick-off, and the goal-moment treatment: score-bug pulse, banner, feed highlight. It consumes events, commentary lines, and statistics from the stream.

Lineup and tactics panels come next. The top risk is legibility: broadcast energy on a light scene against 14px tabular numbers; the visual contract must prove the statistics panel before banner motion is added.

## Goal

The manager reads the match state at a glance: clock, score, who is tired, what just happened, and the numbers.

## Why This Slice Exists

Round 3 Q10 chose the full screen; the design brief's live-play and goal-moment state carries design weight.

## Scope

In:
- Header: clock (tabular numerals), score, team names, kit chips, speed indicator.
- Event feed with commentary lines; goals, cards, substitutions, injuries highlighted; batched inserts per frame at 8x.
- Lineups: two columns with position, name, fatigue bar, condition marker, cards; color never the only indicator.
- Statistics panel: possession, shots, on target, expected goals, passes, pass accuracy, fouls, corners, offsides.
- Empty state at kick-off (no events, 0 to 0) per the design brief.
- Goal moment: score-bug pulse, banner in and out within 1.5 seconds, feed highlight; reduced-motion respected.

Out:
- Tactics and substitution panel, pending chips: `viewer-lineup-tactics`.
- Half-time and full-time reports: `viewer-reports-recovery`.

## Acceptance Criteria

- Given a goal event in the stream, Then the score, the banner, the feed line, and the commentary line update within one frame of the goal tick.
  <!-- observable: true — a person sees the goal moment -->
  verify: { method: in-app browser drive with screenshot at the goal tick and the test hook's tick counter (Playwright later), env: Windows 11, Node 22, mock server, fixture: fixture.bin with a known goal tick, rung: web-2 }
- Given kick-off with no events, Then the feed shows the empty-state text from the design brief and the header reads 0 to 0.
  <!-- observable: true — visible empty state -->
  verify: { method: in-app browser drive with screenshot, env: as above, fixture: fixture.bin paused at tick 0, rung: web-2 }
- Given a player's fatigue field changes in the stream, Then that player's fatigue bar and condition marker update, and the marker carries a text label in addition to color.
  <!-- observable: true — visible lineup panel -->
  verify: { method: in-app browser drive with DOM read of the label, env: as above, fixture: fixture.bin, rung: web-2 }
- Given the statistics record fields in the stream, Then the panel shows every field and the values equal the stream values.
  <!-- observable: false — a JavaScript unit test maps a statistics message to the panel model -->
- Given playback at 8x with more than 10 events per second, Then the feed inserts all events with none dropped, batched per frame.
  <!-- observable: false — unit test over the feed batcher with a synthetic burst -->
- Given `prefers-reduced-motion: reduce`, Then the goal banner appears without motion and the score-bug pulse is disabled.
  <!-- observable: true — visible behavior under the media query -->
  verify: { method: in-app browser drive with the reduced-motion emulation if the browser tool exposes it, otherwise Playwright with `reducedMotion: reduce`, env: as above, fixture: fixture.bin goal tick, rung: web-2 }
- Given the screen at 1280 by 800, Then a reviewer reads the clock, score, and every statistics value without zooming, and focus rings are visible on every control.
  <!-- observable: true — human legibility judgment -->
  verify: { method: operator session with screenshot, env: reference laptop, fixture: fixture.bin, rung: web-5 (residual, pre-registered as a human check) }

## Dependencies on Other Slices

- `viewer-pitch`: the page, the socket client, the history.
- `stream-protocol`: event, statistics, and fatigue messages.
- `commentary`: commentary lines on events (until it ships, the feed shows the event kind).

## Risks

- Panels steal frame budget from the canvas: panels update on their own cadence (per event, per second), never per tick.
- Contrast failure on the light scene: the visual contract at plan proves it with token values before implementation.
