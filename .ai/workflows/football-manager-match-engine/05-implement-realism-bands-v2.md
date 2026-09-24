---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: realism-bands-v2
status: complete
stage-number: 5
created-at: "2026-09-24T00:51:46Z"
updated-at: "2026-09-24T00:51:46Z"
metric-files-changed: 22
metric-lines-added: 1191
metric-lines-removed: 112
metric-deviations-from-plan: 8
metric-review-fixes-applied: 0
commit-sha: "0835298c86afeb90184f3d719b666fafd90158ea"
commits:
  - "0835298c86afeb90184f3d719b666fafd90158ea"
steering-honored:
  - "Dark-path counter definition: change.expired_at_full_time and darkpath.change_never_applied are unchanged; the computer manager's queuing behaviour did not change. The uncommitted contract hunk that records the definition in .ai/observability.md is committed with this slice."
  - "Design direction: not applicable; this slice changes no page. The six new formations reach the lineup editor and the tactics panel through the tactics schema they already read."
  - "Output boundary: code comments, help text, schema descriptions, the content README and the docs use product language."
tags: [calibration, realism, observability, formations, bands]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-realism-bands-v2.md
  plan: 04-plan-realism-bands-v2.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/realism-bands-v2/
  siblings: [05-implement-calibration.md, 05-implement-probe-engine-core.md, 05-implement-tactics-and-ai.md, 05-implement-experiment-flags.md]
  verify: 06-verify-realism-bands-v2.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine realism-bands-v2"
---

# Implement: Realism Bands, Version 2

## The Implementation

The engine at `5a235a4` passed its four average-based bands while its matches flooded goals and cards. Calibration now measures what a real match looks like. The bands file is at version 2 with eleven sourced bands, the match record carries throw-ins and goal kicks, and a formations suite plays all 55 pairings of ten formations. The engine's behaviour did not change. So the baseline shows the engine as it plays today: 9 of the 15 equal-suite bands fail on all five seeds, and 151 of 165 pairing checks fail on seed 42. The worst pairing, 4-2-3-1 against itself, averages 38.5 goals per match.

Two measurements changed the plan. The plan's replay-file test compared two runs that both write every file. So the write cost was measured directly against a temporary build that writes none: 0.44%, under the 5% gate, so the worker still writes every file. The first seed took 76.5 minutes on this 8-core machine. After that seed, the product owner decided that the formations suite runs on one seed (Q-I1), because its misses are far larger than sampling noise. Seeds 1, 7, 99 and 2026 ran the equal and strength suites only.

Verify can now drive the four criteria on the release binary. The next three slices work this baseline down. The top risk is iteration speed: a full run takes about 76 minutes, and the product owner asked for a new slice that gives tuning a faster loop. The benchmark passes on the median of three drives: 422.0 ms per match and 6.73 MB peak memory. The memory margin is thin, but the parent commit reads 6.82 MB on the same drives, so this slice did not cause it.

**Expected red runs:** `engine-cli calibrate` exits 2, and the ignored slow test fails, until `realism-tuning`. The product owner accepted this (Q-E4, Q-P3).

## Summary of Changes

- **Bands file version 2.** Eleven bands are added: 10 or more goals share, sending-off share, yellow cards per team, shots on target share, goals per xG, passes per team, pass accuracy, corners per team, throw-ins per match, goal kicks per match and goalless share. A version-1 file is refused, and so is a file without a band.
- **New record fields.** `stats.throw_ins` and `stats.goal_kicks` are copied from counters the engine already kept. The schema requires both on a success record.
- **New suite figures.** The report gains the nine figures behind the new bands. Shots on target and goals per xG are pooled ratios, guarded so that they are never `NaN`.
- **Six new formations.** 4-1-4-1, 4-4-1-1, 4-1-2-1-2, 3-4-3, 5-3-2 and 5-4-1 are appended, so indices 0 to 3 do not move.
- **The formations suite (`--suite formations`, included in `all`).**
  - It plays 55 pairings, a formation against itself included, each with the equal-suite clubs.
  - The first formation of a pairing is at home in every other match.
  - Each pairing gets three goal-band checks with a `pairing` label, and the pairing figures are written to `calib.formations`.
- **Failing bands on stderr.** Each failing band is one `warn` line with `signal` `calibrate.band_failed`.
- **The paired-run comparison.** Rows are keyed by suite, band and pairing. A floor band (low end 0) adds only the excess above its top to the distance.
- **Documents.** The run-report schema, the observability contract, the content README (every band with its source), the CLI reference, the data-file reference and the modding how-to are updated.

## Files Changed

- `content/realism-bands.json`: version 2; eleven bands (plan step 1).
- `content/tactics.json`: six formations after 3-5-2 (step 6).
- `content/README.md`: realism bands section with unit, range, real value and source per band; formations section (step 13).
- `crates/engine-cli/src/report/bands.rs`: `BANDS_VERSION` 2, eleven `Band` fields, `Band::is_floor`, refusal tests (step 1).
- `crates/engine/src/observe/mod.rs`: `LawStats` `throw_ins` and `goal_kicks`, copied in `LawStats::new`; contract-key test (step 2).
- `schemas/observability/match-stats.schema.json`: the two keys, required on success (step 3).
- `crates/engine-cli/tests/schemas.rs`: a real record carries both keys; a record without `stats.throw_ins` is refused (step 3).
- `crates/engine-cli/tests/resume.rs`: both keys carry over a resume (step 2).
- `crates/engine-cli/src/report/mod.rs`: `Suite::Formations`, new `SuiteFigures`, `ratio`, `PairingFigures`, `BandCheck.pairing`, eleven equal checks, three checks per pairing, `calib.formations`, rebuilt unit tests (steps 4, 5, 8, 9, 14.1).
- `crates/engine/src/data/tactics.rs`: the count test pins ten names in order (step 6).
- `crates/engine-cli/src/calibrate/fixtures.rs`: `pairings`, `FormationFixture`, `formation_fixtures`, tests (step 7).
- `crates/engine-cli/src/calibrate/worker.rs`: formation fixtures and each side's formation through `with_tactics` (step 8).
- `crates/engine-cli/src/calibrate/mod.rs`: formations suite planning and read-back, the jobs clamp over the largest suite, sorted checks, `calibrate.band_failed` lines (steps 8, 10).
- `crates/engine-cli/src/cli.rs`: `SuiteArg::Formations`; help text within 80 columns (step 8).
- `crates/engine-cli/src/report/compare.rs`: rows keyed by pairing, floor-band distance, pairing column, tests (step 12).
- `schemas/observability/run-report.schema.json`: `formations` in three enums, the nine figures, `pairing`, `calib.formations` (step 13).
- `crates/engine-cli/tests/calibrate.rs`: smoke test over all three suites, formations test, slow test on every band (step 14.3).
- `crates/engine-cli/tests/calibrate_pair.rs`: runs on `--suite equal`; fifteen pinned rows; band lines before the table (step 14.2).
- `.ai/observability.md`: two record keys, version-2 bands, formations suite, `calib.formations`, `pairing`, `calibrate.band_failed` (step 13).
- `docs/reference/cli.md`, `docs/reference/data-files.md`, `docs/how-to/modding.md`: suites, run time, stderr lines, version 2 (step 13).

## Shared Files (also touched by sibling slices)

- `crates/engine-cli/src/report/*` and `crates/engine-cli/src/calibrate/*` (`calibration`, `experiment-flags`, `probe-engine-core`).
- `crates/engine/src/observe/mod.rs` and `schemas/observability/*` (`calibration`, `probe-engine-core`, `match-rules`).
- `content/tactics.json` (`tactics-and-ai`, `viewer-lineup-tactics`).
- `.ai/observability.md` (every slice that adds a contract key).

## Notes on Design Choices

- **Pairing on the record, not in it.** The parent derives each record's pairing and first side from the fixture index, as it derives the boosted side of the strength suite. `match-stats` gains no field.
- **A second collection for pairing checks.** In `checks()`, the pairing checks go into their own list, because the shared closure already holds the output list mutably.
- **The jobs clamp.** It now uses the largest suite's fixture count. `--suite formations --matches 1 --jobs 2` then keeps 2 workers for its 55 fixtures.
- **The ratio guard.** It returns 0.0 for a zero denominator. Every ratio band has a low end above 0, so a 0.0 value fails its band instead of writing `null`.
- **Floor-band distance.** A floor band is judged with `Band::is_floor`, so the loader and the comparison share one definition.

## Verification Seams Built

- **Every band is reported** → the eleven equal-suite checks at `crates/engine-cli/src/report/mod.rs:446`, with the figures behind them. The loader is at `crates/engine-cli/src/report/bands.rs:12` and `:102`. This lets `engine-cli calibrate --seed 42` on the release binary, and `report.json`, show each band with `value`, `lo`, `hi` and `pass`.
- **The baseline is recorded** → one `warn` line per failing check at `crates/engine-cli/src/calibrate/mod.rs:445`. Verify can count the lines in stderr against the failed checks. Seed 42 gave 160 lines for 160 failed checks.
- **Formations are reported** → these seams let the formations run and the smoke test with 1 match per pairing read `calib.formations` and the pairing checks:
  - the pairings at `crates/engine-cli/src/calibrate/fixtures.rs:50` and `:68`
  - each side's formation at `crates/engine-cli/src/calibrate/worker.rs:144-145`
  - the pairing figures at `crates/engine-cli/src/report/mod.rs:344`
  - `calib.formations` at `:631`
  - `BandCheck.pairing` at `:129`
- **New keys pass the schema** → `crates/engine/src/observe/mod.rs:276`, `:279` and `:346`, and `schemas/observability/match-stats.schema.json:182`, `:191` and `:339-340`. This lets `engine-cli simulate --seed 7`, with the schema test, check `stats.json`.

## Deviations from Plan

1. **Step 11 measurement method.** The plan compared `--keep-events outliers` with `--keep-events all`. Both runs write every replay file, so that pair measures only the delete cost. The measurement instead compared the release build with a temporary build that skips the replay-file write. The worker source was restored and the release binary rebuilt, and `SM_MEASURE` appears nowhere in the tree. Result on seed 42, equal suite: 75,250 ms against 74,917 ms, so the write costs 0.44% (`implement-evidence/realism-bands-v2/measure.txt`). This is under 5%, so step 11.3 applied: no change, and `outlier()` stays a method.
2. **The baseline covers formations on one seed (Q-I1).** Seed 42 ran every suite. Seeds 1, 7, 99 and 2026 ran the equal and strength suites only, by the product owner's answer after seed 42. The partial seed 1 and seed 7 runs of every suite were stopped and their files removed.
3. **Reference machine.** It has 8 logical cores (Ryzen 7 9800X3D), not 16. Throughput is 12.5 matches per second, so one seed of every suite takes about 76 minutes. The docs state about 74 minutes on a 16-core machine; see Known Risks.
4. **The existing smoke test runs 2 matches, not 4.** It now also plays 55 pairings. `calibrate_pair.rs` moved to `--suite equal` as the plan said, so its strength row is gone. The unit tests in `compare.rs` still cover the stronger-team guardrail.
5. **The jobs clamp.** It uses the largest suite, not `--matches`. Without that change, `--matches 1` forces one worker.
6. **`calib.formations` also appears in each arm of a paired run (`ArmReport`).** The plan named only the top level.
7. **The comparison table.** It shows the band range with 3 decimals, in a column 19 wide, and has a pairing column. At 2 decimals, the band of 0 to 0.005 prints as "0.00 to 0.01".
8. **Help text.** The `--matches` short help reads "Matches per suite or pairing." The 80-column test refused the longer wording. The detail is in the long help.

## Anything Deferred

- **The ignored slow test was not run here.** It plays 57,000 matches on seed 2026, about 76 minutes, and fails by design until `realism-tuning`. The seed 42 baseline ran the same command shape, and its report shows the failures the test would assert. Status: skip, not pass.
- **Interactive checks of the new formations in the viewer.** These are the lineup-editor list of 10 and a 5-4-1 match screenshot. The plan assigns them to verify.

## Known Risks / Caveats

- **Run time.** Each seed of every suite takes about 76 minutes on the reference machine. The formations suite is 97% of that time (4,421 s of 4,579 s on seed 42). The product owner asked for a new slice that gives tuning a fast loop.
- **The `cli.md` time figure.** `docs/reference/cli.md` says "about 74 minutes on a 16-core machine". The reference machine is 8 cores and 76 minutes. The docs figure is not wrong for a 16-core machine, but it has not been measured on one.
- **Timing during the four-seed runs.** The four equal and strength runs ran beside the stopped seed 7 run for about 15 minutes, so their `calib.wall_ms` is about twice the unshared 80 s. Band values do not depend on timing, because every match is deterministic by seed.
- **Memory margin.** The median peak memory is 6.73 MB against the 6.82 MB tripwire. The parent commit reads 6.82 MB on the same drives.
- **Disk while a run is live.** A run writes every replay file before the parent deletes the non-outliers: 58 MB per 1,000 matches, so about 3.3 GB for a full run.
- **Content hash.** It moves from `64f5f49c7994` to `b64cecf856ec`, because `tactics.json` changed. A snapshot saved before this slice is refused on resume, by design. No test pins a real content hash. The replay fixture `web/tests/data/one-minute.smfx` holds its own recorded schema and is not compared with the current content.

## Checks Run

- `cargo fmt --all --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo test --workspace --no-fail-fast`: 434 passed, 0 failed, 4 ignored, over 59 test binaries (`implement-evidence/realism-bands-v2/gate.txt`).
- `node --test web/tests/*.test.mjs`: 128 passed. The directory form, `node --test web/tests`, fails on this Node version, because it runs the directory as one test file.
- Benchmark, `bench --seed 42 --matches 5`, three drives of each build (`bench.txt`):

  | Build | Processor time per match | Peak memory |
  |---|---|---|
  | This slice | 422.0, 412.6, 422.0 ms; median 422.0 (tripwire 460.7) | 6.734, 6.758, 6.734 MB; median 6.734 (tripwire 6.82) |
  | Parent `5a235a4` | 425.0, 422.0, 418.8 ms; median 422.0 | 7.137, 6.820, 6.813 MB; median 6.820 |

  No tripwire fired.

## Baseline

All runs used the release binary with 1,000 matches per suite and per pairing. Every run exited 2, and every strength run exited 0. Every run read zero on `darkpath.change_never_applied`, `darkpath.match_without_stats` and `validate.violations`. Stderr carries one `calibrate.band_failed` line per failed check: 160 on seed 42, and 9 on each of the other equal runs. The evidence is in `implement-evidence/realism-bands-v2/`: `baseline-*.report.json`, `.stderr.txt`, `.exit-code`, `baseline-timing.txt` and `baseline-table.md`.

**Equal and strength suites, five seeds** (x marks a miss):

| Suite, band | Range | seed 42 | seed 1 | seed 7 | seed 99 | seed 2026 | Seeds passed |
|---|---|---|---|---|---|---|---|
| equal `corners_per_team` | 3.5 to 6.5 | 0.004 (x) | 0.007 (x) | 0.007 (x) | 0.005 (x) | 0.004 (x) | 0 of 5 |
| equal `goal_kicks_per_match` | 12.0 to 22.0 | 7.402 (x) | 6.339 (x) | 7.132 (x) | 7.0 (x) | 6.57 (x) | 0 of 5 |
| equal `goalless_share` | 0.04 to 0.12 | 0.192 (x) | 0.224 (x) | 0.218 (x) | 0.186 (x) | 0.231 (x) | 0 of 5 |
| equal `goals_per_match` | 2.4 to 3.2 | 3.035 | 2.656 | 2.863 | 2.948 | 2.559 | 5 of 5 |
| equal `goals_per_xg` | 0.85 to 1.15 | 1.5255 (x) | 1.5469 (x) | 1.5115 (x) | 1.5425 (x) | 1.4924 (x) | 0 of 5 |
| equal `pass_accuracy_pct` | 75.0 to 88.0 | 84.771 | 84.89 | 84.857 | 84.36 | 84.822 | 5 of 5 |
| equal `passes_per_team` | 350.0 to 550.0 | 1304.772 (x) | 1328.293 (x) | 1325.249 (x) | 1298.319 (x) | 1310.293 (x) | 0 of 5 |
| equal `possession_away_pct` | 35.0 to 65.0 | 50.07 | 50.289 | 50.15 | 50.227 | 49.816 | 5 of 5 |
| equal `possession_home_pct` | 35.0 to 65.0 | 49.93 | 49.711 | 49.85 | 49.773 | 50.184 | 5 of 5 |
| equal `sending_off_share` | 0.08 to 0.22 | 0.421 (x) | 0.398 (x) | 0.4 (x) | 0.363 (x) | 0.39 (x) | 0 of 5 |
| equal `shots_on_target_share` | 0.3 to 0.42 | 0.7501 (x) | 0.7558 (x) | 0.7493 (x) | 0.7558 (x) | 0.7466 (x) | 0 of 5 |
| equal `shots_per_team` | 8.0 to 16.0 | 14.753 | 12.897 | 14.122 | 14.261 | 12.899 | 5 of 5 |
| equal `ten_plus_goals_share` | 0.0 to 0.005 | 0.063 (x) | 0.047 (x) | 0.059 (x) | 0.051 (x) | 0.046 (x) | 0 of 5 |
| equal `throw_ins_per_match` | 35.0 to 55.0 | 25.958 (x) | 28.552 (x) | 25.851 (x) | 26.94 (x) | 26.665 (x) | 0 of 5 |
| equal `yellow_cards_per_team` | 1.2 to 2.6 | 1.6 | 1.555 | 1.483 | 1.512 | 1.623 | 5 of 5 |
| strength `stronger_team_win_rate` | 0.5 to 1.0 | 0.6 | 0.596 | 0.645 | 0.666 | 0.592 | 5 of 5 |

The five bands the slice named as failing all fail on every seed: the 10 or more goals share, the sending-off share, shots on target, passes per team and corners per team.

**Formations suite, seed 42.** All 55 rows are in `formations-s42-table.md`.

| Measure | Result |
|---|---|
| Pairing checks passed | 14 of 165 |
| Goals per match inside 2.4 to 3.2 | 7 of 55 pairings: 4-4-2 v 4-4-2, 4-2-3-1 v 3-5-2, 4-3-3 v 4-1-4-1, 4-3-3 v 4-2-3-1, 4-1-4-1 v 5-3-2, 4-1-2-1-2 v 5-4-1, 4-1-2-1-2 v 3-4-3 |
| 10 or more goals share at most 0.005 | 1 of 55 pairings |
| Goalless share inside 0.04 to 0.12 | 6 of 55 pairings |
| Pairings with all three bands | 0 |
| Highest goals per match | 4-2-3-1 v 4-2-3-1: 38.461 (every match 10 or more goals); 4-4-1-1 v 4-4-1-1: 20.662; 4-3-3 v 4-3-3: 19.842 |
| Lowest goals per match | 3-5-2 v 4-1-4-1: 1.454 (goalless 0.369); 4-4-2 v 4-1-2-1-2: 1.605 |
| Suite time | 4,421 s of the seed's 4,579 s |

## Freshness Research

- No dependency changed since the plan's freshness pass: `serde` 1.0.229, `serde_json` 1.0.151 and `boon` 0.6.1. The plan's reads still hold. A missing band is refused without extra code (the new refusal test confirms it). A non-finite `f64` would be written as `null`, which the ratio guard prevents (the new unit test asserts no `null`).

## Recommended Next Stage

- **Option A (default): Verify** → `/wf verify football-manager-match-engine realism-bands-v2`. It drives the four criteria on the release binary, and it runs the viewer check of the ten formations. When the build is unchanged since this record, the seed 42 formations run is the same evidence a new 76-minute run would give. Verify decides whether to reuse it. Compact the session before verify. Workflow state lives in the artifact files, and the SessionStart hook re-reads it after compaction.
- **Option B: Skip to Review** → `/wf review football-manager-match-engine realism-bands-v2`. Not recommended, because the slice has user-observable criteria that verify drives.
- **Also queued:** the new slice for a fast tuning loop, which the product owner asked for (`/wf intake football-manager-match-engine <description>`). The problem statement from this session is its input.
