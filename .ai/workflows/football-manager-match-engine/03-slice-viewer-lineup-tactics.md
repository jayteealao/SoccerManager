---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: viewer-lineup-tactics
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: l
depends-on: [viewer-match-day, tactics-and-ai]
tags: [viewer, lineup, tactics, substitutions]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-viewer-match-day.md, 03-slice-tactics-and-ai.md, 03-slice-viewer-reports-recovery.md, 03-slice-integration.md]
  plan: 04-plan-viewer-lineup-tactics.md
  implement: 05-implement-viewer-lineup-tactics.md
---

# Slice: Lineup Editor and Tactics Panel

## The Slice

The match-day screen shows the match; the engine accepts queued changes. The manager still has no way to pick a lineup or change tactics from the page, which is the first and fifth step of the core loop and the substance of C1 and C5.

This slice adds the pre-match lineup editor (formation slots, eleven plus bench from a squad, validation), the Tier 2 tactics panel (mentality, team instructions, roles and duties), the substitution picker, and the pending-change chips that clear when a change applies. The panel opens at any time; changes queue and apply at the next qualifying stoppage.

Reports and recovery come next; integration proves the whole loop after that. The top risk is the editor's complexity on a canvas-heavy page; the editor is ordinary DOM, not canvas, so it stays accessible.

## Goal

The manager sets the team before kick-off and changes it during the match from the page, with clear feedback on what is pending, applied, or rejected.

## Why This Slice Exists

Round 1 Q2 chose eleven plus bench; Q3 chose Tier 2; Round 3 Q11 required the panel at any time with stoppage application; the design brief weights the pre-match screen and the stoppage intervention.

## Scope

In:
- Pre-match lineup editor: formation picker, eleven slots plus up to seven bench slots, click or drag from the squad list, role fit and fitness per slot, inline validation, kick-off disabled until legal.
- Tactics panel: mentality, 6 to 8 team instructions, roles and duties per slot; available during play and while paused.
- Substitution picker with the remaining-count display; rejected changes show the engine's reason.
- Pending-change chips (named mechanism: **pending-change list**, mirrors the engine's queue so the manager sees what is queued, applied, or rejected).
- DOM-based editor; keyboard operable; focus rings; color never the only indicator.

Out:
- Set-piece routines and per-player instructions (Tier 3): out of scope.
- Reports, recovery: `viewer-reports-recovery`.

## Acceptance Criteria

- Given fewer than eleven starters, no goalkeeper, or a player placed twice, Then kick-off is disabled and the reason shows inline; and Given a legal lineup, Then kick-off is enabled.
  <!-- observable: true — visible validation -->
  verify: { method: in-app browser drive with screenshots for each illegal case and the legal case (Playwright later), env: Windows 11, Node 22, engine built with tactics-and-ai, fixture: generated squad seed 11, rung: web-2 }
- Given the manager queues a mentality change during play, Then a pending chip shows, and at the next dead ball the chip clears and the feed shows "Tactical change applied".
  <!-- observable: true — visible chip lifecycle -->
  verify: { method: in-app browser drive against the live engine with the socket on, env: as above, fixture: seed 42 default teams, rung: web-2 }
- Given the manager queues a substitution, Then it applies at the next dead ball, the lineup panel swaps the players, and the remaining count decrements.
  <!-- observable: true — visible lineup change -->
  verify: { method: in-app browser drive against the live engine, env: as above, fixture: seed 42, rung: web-2 }
- Given the substitution limit is used, When the manager queues another, Then the page shows the engine's rejection reason.
  <!-- observable: true — visible rejection -->
  verify: { method: in-app browser drive against the live engine after five substitutions, env: as above, fixture: seed 42, rung: web-2 }
- Given the tactics panel is open while paused, When the manager changes a role and resumes, Then the change applies at the next dead ball, not at resume.
  <!-- observable: true — visible timing -->
  verify: { method: in-app browser drive reading the applied-event minute, env: as above, fixture: seed 42, rung: web-2 }
- Given keyboard-only operation, Then every slot, control, and picker is reachable and operable with visible focus.
  <!-- observable: false — a Testing Library test over roles and focus order -->

## Dependencies on Other Slices

- `viewer-match-day`: the page and the lineup panel it updates.
- `tactics-and-ai`: the change queue, rejection reasons, and the tactics model.

## Risks

- Drag and drop on the editor fights accessibility: click-to-place is primary; drag is an enhancement.
- Tier 2 instruction count grows: the panel renders from the tactics schema, so new instructions need no UI change.
