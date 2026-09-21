---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: commentary
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: s
depends-on: [match-rules]
tags: [engine, commentary, text]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-match-rules.md, 03-slice-viewer-match-day.md]
  plan: 04-plan-commentary.md
  implement: 05-implement-commentary.md
---

# Slice: Context-Aware Commentary

## The Slice

Match rules emit typed events with actors, minute, and score state. Commitment C6 requires text commentary, and the product owner chose context-aware lines over plain templates.

This slice adds a commentary generator: per event kind, several templates, selected by minute band, score state, in-match form, and repeat count, with player and team names filled in. Lines ride on the event stream as a field, so the viewer's feed needs no extra channel.

The match-day panels show these lines next. The top risk is repetition; the acceptance criteria measure distinct lines over a match.

## Goal

Every event produces an English commentary line that reads as aware of the match situation.

## Why This Slice Exists

C6 names commentary; the product owner chose context awareness (Q22) and made commentary its own slice (slice Q6).

## Scope

In:
- Template files per event kind (about 30 kinds) with variables; context selectors (minute band, score state, form, repeat count).
- Generator inside the engine; the line is a field on the event record.
- English only.

Out:
- Rendering in the feed: `viewer-match-day`.
- Other languages: out of scope.

## Acceptance Criteria

- Given any event kind in the rule set, Then a commentary line is produced that names the player and team involved.
  <!-- observable: false — cargo test enumerates event kinds -->
- Given the same event kind five times in ten minutes, Then at least three distinct lines appear.
  <!-- observable: false — cargo test -->
- Given a goal that levels the score in minute 88, Then the line reflects a late equalizer; and Given a goal that makes it 4 to 0 in minute 20, Then the line reflects a rout.
  <!-- observable: false — cargo test with scripted events -->
- Given a full seeded match, Then no template variable is left unfilled in any line.
  <!-- observable: false — cargo test scans output -->

## Dependencies on Other Slices

- `match-rules`: the event kinds and score state.

## Risks

- Template authoring volume: start with 3 lines per kind and grow through data files (modding surface).
