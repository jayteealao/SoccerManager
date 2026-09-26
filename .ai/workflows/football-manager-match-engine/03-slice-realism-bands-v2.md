---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: realism-bands-v2
status: complete
stage-number: 3
created-at: "2026-09-23T22:03:35Z"
updated-at: "2026-09-24T02:30:58Z"
complexity: m
depends-on: [calibration, probe-engine-core]
source: extension
source-ref: "user description"
extension-round: 1
tags: [calibration, realism, observability]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  source: ""
  plan: 04-plan-realism-bands-v2.md
  implement: 05-implement-realism-bands-v2.md
---

# Slice: Realism bands, version 2

## Goal

Make calibration measure what a real match looks like, not only its averages. Add eleven bands from real-match data, a formation suite, and the missing record fields. The engine does not change in this slice, so several of the new bands fail on purpose. Their failures are the baseline that the next three slices work down.

## Why This Slice Exists

A fresh run of 10,000 matches at `5a235a4` passed every existing band on five seeds. The same matches were not realistic:
- **Goals:** 10.5% had ten or more goals, against none in 1,941 real matches.
- **Cards:** 45% had a sending-off, against a real 15.7%.
- **Shots:** 75% of shots were on target, against a real 35%.
- **Passes and set pieces:** passes ran at about three times the real volume, and teams earned almost no corners.

The goals band passes because blowouts and extra goalless draws average out to about three goals. The product owner asked for bands first, so that every later slice shows its progress against them (`po-answers.md`, extend round 1).

## Scope

- **In:**
  - Add these bands to `content/realism-bands.json`. The values are sourced from Wyscout and StatsBomb data, as recorded in `docs/design/realism/01-engine-realism.md` and its local sources.

    | Band | Low | High | Real value |
    |---|---|---|---|
    | Share of matches with 10 or more goals | 0 | 0.5% | 0 of 1,941 |
    | Share of matches with a sending-off | 8% | 22% | 15.7% |
    | Yellow cards per team | 1.2 | 2.6 | about 1.4–2.5 |
    | Shots on target, share of shots | 30% | 42% | 35% |
    | Goals per unit of xG | 0.85 | 1.15 | 0.99 |
    | Passes per team | 350 | 550 | 429 |
    | Pass accuracy | 75% | 88% | 82–83% |
    | Corners per team | 3.5 | 6.5 | 4.98 |
    | Throw-ins per match | 35 | 55 | 42–46 |
    | Goal kicks per match | 12 | 22 | 16–17 |
    | Share of goalless draws | 4% | 12% | 7.1% |

  - Compute each new figure in `crates/engine-cli/src/report/` and check it in the equal suite.
  - Give the `Bands` struct, its loader tests and the pinned band lists the new fields.
  - Add `stats.throw_ins` and `stats.goal_kicks` to the `match-stats` record. `Summary` already counts both, but `LawStats::new` does not copy them.
  - Update `schemas/observability/*.schema.json` and `.ai/observability.md`, so the new figures become contract keys.
  - Add a `formations` suite to `calibrate`. Each shipped formation (4-4-2, 4-3-3, 4-2-3-1, 3-5-2) plays against 4-4-2 with equal clubs. Each pairing reports goals per match and the share of matches with ten or more goals.
  - Record the failing bands as the baseline for the later slices.
- **Out:**
  - Any change to engine behaviour or tuning. The following slices own those.
  - Widening a band to make it pass. Bands change only by a recorded product-owner answer.

## Acceptance Criteria

- **Every band is reported.** Given the shipped bands, when `engine-cli calibrate --seed 42` runs 1,000 matches per suite, then `report.json` holds every one of the eleven new bands with its value, `lo`, `hi` and `pass`, and a failed band sets exit code 2 as today.
- **The baseline is recorded.** Given the engine at this slice, when the run completes, then stderr names each failing band. The run is recorded as the baseline in this slice's implement record. At least these bands fail: the 10-or-more-goals share, the sending-off share, shots on target, passes per team and corners per team.
- **Formations are reported.** Given every shipped formation, when the `formations` suite runs, then the report holds goals per match and the share of 10 or more goals for each pairing against 4-4-2.
- **New keys pass the schema.** Given a finished match, when `match-stats` is written, then it holds `stats.throw_ins` and `stats.goal_kicks` for both teams, and the record passes its schema.

## Dependencies on Other Slices

- `calibration`: the harness, the band loader and the report this slice extends.
- `probe-engine-core`: the `error` outcome word and the schema shape it settled.

## Risks

- **Expected failures.** The ignored slow test `a_thousand_matches_hold_the_realism_bands` and the calibration exit code fail until `realism-tuning`. The product owner accepted this: slices before the final tuning slice are judged on their own criteria, and their band misses are recorded rather than treated as failures.
- **Paired-run verdict.** A band that starts at 0 adds distance-to-centre in `report/compare.rs` `verdict()`, even when it passes. The paired-run verdict must stay meaningful.
- **Reference values.** The values come from another programme's staged document and its local sources. This slice cites them at their line numbers and does not edit that document.
