---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: tuning-loop
status: defined
stage-number: 3
created-at: "2026-09-24T06:26:59Z"
updated-at: "2026-09-24T06:26:59Z"
complexity: m
depends-on: [realism-bands-v2]
source: extension
source-ref: "user description"
extension-round: 2
tags: [calibration, tooling, realism, performance]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  source: ""
  plan: 04-plan-tuning-loop.md
  implement: 05-implement-tuning-loop.md
---

# Slice: Tuning loop

## Goal

A tuning change can be measured in under 2 minutes. `calibrate` runs only the suites, pairings or bands a change targets, on one seed, and prints each band's change against a stored baseline with its sampling error. The red-card experiment runs through the same loop.

## Why This Slice Exists

Every remaining realism slice is a tuning loop, and the loop is too slow:
- **A full run is slow.** One seed of every suite is 57,000 matches and takes about 76 minutes at 12.5 matches per second on the 8-core reference machine. Five seeds take more than 6 hours.
- **Most of that time is not needed.** A change that targets one pairing needs that pairing's 1,000 matches, about 80 seconds.
- **Two measurements live outside calibrate.** The red-card experiment and the formations check are 120-seed cargo slow tests, so they have no baseline and no band diff.
- **Noise reads as progress.** Two runs are compared by eye, without a sampling error.

`defending-and-discipline` stopped after a sweep of 7 settings on 60 seeds. `lone-forward` and `realism-tuning` need many more settings. The product owner agreed the slice in the `realism-bands-v2` implement run and defined it in extension round 2 (`po-answers.md`, Q-X1, Q-X5, Q-X6).

## Scope

- **In:**
  - **Targeted runs:** `calibrate` selects named suites, named pairings and named bands on one seed. A run of a selection plays only the fixtures that selection needs.
  - **Stored baseline and diff:**
    - `calibrate --baseline <report.json>` compares the run to an earlier report on the same seed and the same matches.
    - The diff prints, for each band, the baseline value, the new value, the change, its sampling error and the band verdict.
    - A band change inside two sampling errors is marked as noise.
    - A baseline on a different seed, match count or content hash is refused with a message that names the difference.
  - **The red-card experiment as a suite:** the controlled sending-off experiment (keeper, centre-back and striker arms, cards otherwise off, and the control) becomes a calibrate suite. Its figures and its criterion are in the run report, so a baseline and a diff apply to it.
  - **Build profile:** measure LTO, one codegen unit and a native CPU target for the calibrate binary. Keep a profile only if matches per second rise and every result is identical per seed.
  - **Docs:** the command reference and the calibration how-to.
- **Out:**
  - Splitting a run across machines.
  - Early stopping and sequential tests.
  - Any change to a band, a criterion limit or a tuning value.
  - The five-seed rule for the equal and strength suites and the one-seed rule for formations (Q-I1). A targeted run is an inner loop; the slice gates still run the full suites.

## Acceptance Criteria

- **A targeted run is fast.** On the 8-core reference machine, one targeted suite or pairing at 1,000 matches, or the full red-card experiment, finishes with its diff against the baseline in under 2 minutes.
- **A targeted run plays only its selection.** Given `calibrate --suite formations --pairing "4-4-1-1 v 4-4-2" --matches 1000`, when the run finishes, then the report holds that pairing only, and 1,000 matches were played.
- **A targeted run agrees with the full run.** For the same seed, the figures of a targeted pairing or suite equal the figures of that pairing or suite in a full `--suite all` run.
- **The diff separates change from noise.** Given a baseline and a run on the same seed, when the diff prints, then every band shows the baseline value, the new value, the change and its sampling error, and a change inside two sampling errors is marked as noise.
- **A wrong baseline is refused.** Given a baseline on a different seed, match count or content hash, when the run starts, then it exits non-zero before any match is played, and the message names the difference.
- **The red-card experiment is a suite.** `calibrate --suite red-card` reports each arm's goals for the full and the reduced side, the control, and the criterion verdict. Its figures equal the ones the slow test `a_sending_off_gives_no_advantage` computes on the same seeds.
- **A faster profile changes no result.** A kept build profile gives the same report figures per seed as the current profile, and its matches per second are recorded. When no profile is faster, the measurement is recorded and the current profile stays.

## Dependencies on Other Slices

- `realism-bands-v2`: the bands file, the formations suite, pairing figures and `compare`.
- `defending-and-discipline`: the red-card experiment and its test code, which this slice moves into calibrate. The experiment is committed at `f7fe35b`.

## Risks

- **Targeted and full runs drift apart.** A fixture's seed must not depend on which other fixtures run. The agreement criterion checks this.
- **A faster profile changes floating-point results.** A native CPU target can change float operations and so move seeded results. The identical-results criterion guards this, and a profile that fails it is dropped.
- **Noise is still misread.** The sampling error for a share band with few events is wide. The diff prints the error, and the slices that tune must confirm a gain on the full gate.
