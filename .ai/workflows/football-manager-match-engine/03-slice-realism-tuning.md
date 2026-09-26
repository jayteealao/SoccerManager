---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: realism-tuning
status: skipped
skipped: true
skip-record: skip-slice-realism-tuning.md
stage-number: 3
created-at: "2026-09-23T22:03:35Z"
updated-at: "2026-09-25T22:45:37Z"
complexity: m
depends-on: [tempo-and-restarts]
source: extension
source-ref: "user description"
extension-round: 1
tags: [calibration, tuning, realism, e2e]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  source: ""
  plan: 04-plan-realism-tuning.md
  implement: 05-implement-realism-tuning.md
---

# Slice: Realism tuning

## Goal

Bring every band into range on every seed, and prove that the whole product still works end to end. This is the final tuning slice the product owner asked for. It is the only extension slice that must pass every band.

## Why This Slice Exists

The three mechanism slices before it each change match results, and each is judged on its own criteria. The product owner chose one final tuning slice to retune once over the combined behaviour, instead of retuning after every slice (`po-answers.md`, extend round 1). The browser suite was not run again after review fix T1 (`07-review.md`). The product owner asked for it before handoff.

## Scope

- **In:**
  - **Retuning:** retune `content/tuning.json` and `Tuning::default()` together, so that every band passes in the equal, strength and formations suites. The bands include the original four and the eleven from `realism-bands-v2`.
  - **Seeds:** tune on seeds 42, 1, 7, 99 and 2026. Record the tuning log.
  - **Tests:** re-derive every pinned expectation the retune moves, and record each one.
  - **Formation bands:** tighten the formation-pairing check from "at most 4.0 goals per side" to the goals-per-match band for every pairing.
  - **Benchmark:** run it against the tripwires (+10% CPU per tick, +25% peak memory). A breach is recorded and re-baselined only by a product-owner answer.
  - **Browser suite:** run the 21-test Playwright suite on a fresh release build.
  - **Run analysis:** regenerate the run analysis so the build book can be updated.
- **Out:**
  - **The two ship-blocking deferrals:** the legibility reading and the macOS build. The product owner chose to leave both open (`po-answers.md`, extend round 1).
  - **Handoff:** handoff is the next command after this slice, not part of it.
  - **CI:** review item T6 stays deferred.

## Acceptance Criteria

- **Every band passes on every seed.** `engine-cli calibrate --seed <s>` runs 1,000 matches per suite for each of the seeds 42, 1, 7, 99 and 2026. Every band passes in every suite, including the formations suite, and each run exits 0.
- **No advantage from a red card.** The controlled sending-off experiment from `defending-and-discipline` still passes on the tuned build.
- **The browser suite passes.** On a fresh `cargo build --release`, `npm test` in `e2e/` passes 21 of 21.
- **The benchmark holds.** `bench --seed 42 --matches 5` stays within the tripwires, or a recorded product-owner answer re-baselines it.

## Dependencies on Other Slices

- `tempo-and-restarts`, `keeper-and-shots` and `defending-and-discipline`: the mechanisms being tuned.
- `realism-bands-v2`: the band set and the formations suite.

## Risks

- **Bands that conflict.** Some bands pull against each other. For example, fewer passes and more restart time lower shots and goals. If no tuning passes every band on every seed, the slice stops and brings the product owner the closest result and the conflicting pair. It never widens a band silently.
- **Seed sensitivity.** A result tuned on one seed can miss on another. The review found this at IF-1. All five seeds are part of the criterion.
- **Browser suite runtime.** The suite takes about 25 minutes, and the whole-match scenario is about 15 of those. Engine changes that lengthen stoppages lengthen the scenario, so its timeout may need to change. Any timeout change is recorded.
