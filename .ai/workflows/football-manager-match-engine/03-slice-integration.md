---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: integration
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: m
depends-on: [calibration, commentary, viewer-lineup-tactics, viewer-reports-recovery]
tags: [integration, charter-scenario, playwright, docs, final]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-calibration.md, 03-slice-commentary.md, 03-slice-viewer-lineup-tactics.md, 03-slice-viewer-reports-recovery.md]
  plan: 04-plan-integration.md
  implement: 05-implement-integration.md
---

# Slice: Integration and Charter Scenario

## The Slice

Every buildable slice verifies on its own, most viewer slices against the fixture. The shape's residual for the viewer, play against the live tuned engine, clears only when the whole charter scenario runs end to end.

This slice runs the full charter scenario against the live engine, adds the Playwright suite the product owner asked for once the viewer markup is stable, reruns the benchmark on the complete engine, ships the documentation from the plan, and confirms the observability records flow into the pipeline. It writes little new product code; it writes tests, documentation, and fixes.

This is the final slice of the first release; the deferred slices follow. The top risk is a late performance miss on the complete engine, which reopens the tick-rate question under NFR-1.

## Goal

A manager plays a complete match from lineup to full-time report on the reference machine, and every acceptance criterion in the shape has evidence.

## Why This Slice Exists

The charter scenario's final AC belongs to a final slice; the Playwright suite needs stable markup; the benchmark on the complete engine is the shape's pre-deploy proxy residual.

## Scope

In:
- Charter scenario, all 12 steps, against the live engine.
- Playwright suite covering the interactive acceptance criteria of the viewer slices; runs headless.
- Full-engine benchmark against NFR-1 with the tripwire.
- Documentation per the shape's Documentation Plan: README update, first-match tutorial, modding how-to, protocol and schema reference, engine explanation.
- Observability check: the event stream and statistics record from a browser-driven match reach the pipeline sink defined by `.ai/observability.md`.
- License files (MIT or Apache-2.0) and dependency license audit.

Out:
- Installer and other operating systems: `distribution` (deferred).

## Acceptance Criteria

- Standing charter-scenario AC: Given the live engine and the page, When the charter scenario runs, Then all 12 steps execute with their checkpoints.
  <!-- observable: true — the end-to-end proof -->
  verify: { method: in-app browser drive with screenshots per step, then the Playwright scenario test, env: Windows 11 reference laptop with the complete engine, fixture: generated league seed 2026, rung: web-1 }
- Given the Playwright suite, When it runs headless, Then every viewer interactive acceptance criterion has a passing test or a pre-registered human check.
  <!-- observable: false — the suite result is the assertion -->
- Given the complete engine, When the benchmark runs one match on one thread, Then wall time is under 2 seconds; and Given 1000 matches, Then under 30 minutes.
  <!-- observable: true — developer-visible report -->
  verify: { method: cargo bench harness, env: reference laptop, fixture: seed 42, rung: cli-direct }
- Given a browser-driven match, Then the statistics record and event stream appear in the observability sink within the pipeline's stated latency.
  <!-- observable: true — visible in the pipeline's own surface -->
  verify: { method: read the sink defined by .ai/observability.md, env: reference laptop with the pipeline built, fixture: seed 42, rung: infra-1 }
- Given the documentation set, Then every document in the plan exists at its target location and the tutorial's steps run as written.
  <!-- observable: true — a person follows the tutorial -->
  verify: { method: operator follows the tutorial from a clean clone, env: reference laptop, fixture: none, rung: web-5 (residual, pre-registered as a human check) }
- Given the dependency tree, Then no dependency carries a GPL or AGPL license.
  <!-- observable: false — cargo-deny or equivalent in a test -->

## Dependencies on Other Slices

- `calibration`: the tuned engine and the records.
- `commentary`: lines in the feed.
- `viewer-lineup-tactics`: steps 2, 3, 6, 7 of the scenario.
- `viewer-reports-recovery`: steps 10 and 11.

## Risks

- Playwright install on the reference laptop: Node 22 is present; the plan authorizes the install (rung 0).
- Observability sink not built yet: `/wf observability build` precedes this slice, or the AC defers with the sink's slice named.
