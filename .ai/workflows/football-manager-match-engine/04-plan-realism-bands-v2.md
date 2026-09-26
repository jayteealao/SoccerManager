---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: realism-bands-v2
status: complete
stage-number: 4
created-at: "2026-09-23T22:29:35Z"
updated-at: "2026-09-23T22:29:35Z"
metric-files-to-touch: 22
metric-step-count: 14
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [calibration, realism, observability, formations]
stack-source: confirmed
open-questions: []
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-realism-bands-v2.md
  siblings: [04-plan-calibration.md, 04-plan-probe-engine-core.md, 04-plan-tactics-and-ai.md, 04-plan-viewer-lineup-tactics.md]
  contract: ../../observability.md
  implement: 05-implement-realism-bands-v2.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine realism-bands-v2"
---

# Plan: Realism Bands, Version 2

## The Plan

A 10,000-match run at `5a235a4` passed every existing band. Yet 10.5% of matches had ten or more goals, and 45% had a sending-off. The four bands measure averages only, so floods and goalless draws cancel out. This slice adds eleven sourced bands to the equal suite, adds `stats.throw_ins` and `stats.goal_kicks` to the match record, and adds a formations suite. It changes no engine behaviour. The research confirmed that nine of the eleven figures already reach the report builder. Only throw-ins and goal kicks are missing, and the engine already counts both (`sim.rs:394-395`).

The product owner widened the formations suite in planning. It now pairs every formation against every other formation, and each pairing plays 1,000 matches and is checked against the three goal bands (Q-P1, Q-P2). Six formations join the four shipped ones, so 10 formations give 55 pairings (Q-P5). `--suite all` always includes the suite (Q-P6), and every pairing plays all its matches (Q-P8). The bands file moves to version 2 and refuses an older file (Q-P7). The paired-run distance counts only the excess above the high end for a band whose low end is 0 (Q-P4, delegated). The ignored slow test asserts every band and stays red until `realism-tuning` (Q-P3).

The work is 14 steps over 22 files. When it lands, the next three slices get a five-seed baseline of every suite (Q-P9). The top risk is run time: a full run plays 57,000 matches, about 74 minutes per seed and about 6 hours for the baseline. The plan saves time without cutting matches. The formations suite runs as one suite on one worker pool, and the worker writes a replay file only for an outlier when step 11 measures a gain.

## Current State

- Code: HEAD `5a235a4`. The four research reads in this run cite that tree.
- `content/realism-bands.json` has 9 lines: `schema_version` 1, `sample_size` 1000, `goals_per_match` 2.4–3.2, `shots_per_team` 8–16, `possession_pct` 35–65, `stronger_team` {1.15, 0.5}, `wall_minutes_per_sample` 30 (lines 2–8).
- `report/bands.rs`: `Band` (:15–22) and `Bands` (:52–71) both use `deny_unknown_fields`. `BANDS_VERSION = 1` (:12). The loader is `Bands::load` (:76–79). The tests `the_shipped_bands_are_the_accepted_criteria` (:99–108) and `an_inverted_band_is_refused_naming_the_field` (:122–132) pin the values and the goals line.
- `report/mod.rs`:
  - `Suite` has `Equal` and `Strength` (:19–44). `number()` feeds `match_seed` (`fixtures.rs:74-76`).
  - `RunBuilder::add` (:128) keeps each whole `MatchStats`. `suite_figures` (:168–241) uses only `outcome == "success"` records and the closures `per_match` (:175) and `per_team` (:179).
  - `SuiteFigures` (:57–79) already computes `shots_on_target_per_team_mean`, `xg_per_match_mean`, `passes_per_team_mean`, `pass_accuracy_pct_mean` and `fouls_per_team_mean` without a band, and one share, `possession_in_band_share` (:188–192).
  - `checks()` (:244–323) emits the five checks and `wall_ms` per suite. The wall budget scales with the match count (`wall_budget_ms`, :311).
  - `outlier()` (:144–158) is a per-match predicate over the bands only.
- Per-match data for the new figures, all in `MatchStats`: goals `r.goals`; red `r.laws.red` (:280), which counts a straight red and a second yellow; yellow `r.laws.yellow` (:278), which also counts a second yellow (`rules/mod.rs:322,332`); shots `r.tactics.shots`; on target, xG (2 decimals per match, `observe/mod.rs:187`), passes and pass accuracy in `r.figures`; corners `r.laws.corners`. Throw-ins and goal kicks are missing: `LawStats` (`observe/mod.rs:268-330`) has no field for them, and `LawStats::new` (:335–355) does not copy `Summary.throw_ins` and `Summary.goal_kicks`.
- `report/compare.rs`: `distance()` (:253–258) adds `|v − centre| / half-width` for every row except `STRONGER` and `hi <= lo`, so a passing 0 in a 0–0.005 band adds 1.0. `compare()` drops only `wall_ms` (:206). Rows are keyed by suite and band.
- `calibrate/mod.rs`: suites run one after another, each on `jobs` worker processes (:326–358). `jobs` defaults to the logical core count (:98; 16 on the reference machine). The parent reads every `stats/<id>.json`, adds it to the builder, and deletes the replay file of a non-outlier (:369–385). Exit code 2 comes from :219–225. An unpaired run prints no band result on stderr. Only `calibrate.darkpath` (:397) and `calibrate.worker_failed` (:340) warn.
- Formations: four in `content/tactics.json` (:5, :21, :37, :53), found by name through `TacticsSchema::formation_index` (`data/tactics.rs:382`). A test pins 4 (`data/tactics.rs:436`). The computer manager starts in `ai.formation` "4-4-2" (`tactics.json:211`) and its in-match patches change only mentality and instructions (`ai.rs:266-291`). So a formation set through `MatchConfig::with_tactics` (`sim.rs:159-166`) after `Tactics::set_formation` (`tactics/mod.rs:58`) holds for the whole match. `tests/mentality.rs:21-26` shows this pattern. The lineup editor and the tactics panel list formations from the schema in a `<select>` (`web/lineup-editor.mjs:75-81`).
- Schemas (`boon` 0.6.1, draft 2020-12): `match-stats.schema.json` requires the `stats.*` keys in the `else` branch (:308–331) and allows extra keys (:332). `run-report.schema.json` lists the suite names `equal` and `strength` as an enum in three places (:227–230, :272–275, :430–433). It requires seven suite figures (:234–242).
- Tests at this tree: `cargo test -p engine-cli --bin engine-cli report` gave 14 passed. `--test schemas` gave 4, `--test calibrate` 2 passed and 1 ignored, `--test docs` 3 passed. `calibrate_pair.rs:164-173` pins five compare rows. The ignored slow test (`calibrate.rs:146-183`) pins the band names and exit 0.
- Timing: 1,000 matches take about 78 s per suite with 16 workers (`06-verify-calibration.md:124`).
- The slice definition names `docs/design/realism/01-engine-realism.md` as the source of the values. Four values are in that document: goalless share (:120), pass accuracy (:124), throw-ins and goal kicks (:128). The other seven come from its local sources: `.scratch/out/FINDINGS.md:140-153`, `.scratch/out/ACTIONS.md:345-347, 413, 562`, and a count over the Wyscout match files `.scratch/out/wy2_*.csv` (0 of 1,941 matches with 10 or more goals; 15.7% with a sending-off; 428.6 passes and 4.98 corners per team). The values are the product owner's answer Q-E1 and do not change.

## Simplicity Ladder

- Eleven band fields → rung 3 reuse — `report/bands.rs` → `Band` with `#[garde(dive)]`, exact match; reuse as-is.
- Shares (10 or more goals, sending-off, goalless) → rung 3 reuse — `report/mod.rs` → the `per_match` closure with a 0/1 indicator and `mean()` (:175, :347), the same pattern as `possession_in_band_share`; reuse as-is.
- Per-team means (yellows, corners) and per-match means (throw-ins, goal kicks) → rung 3 reuse — `per_team` and `per_match`; reuse as-is. `passes_per_team_mean` and `pass_accuracy_pct_mean` already exist; reuse as-is.
- Pooled ratios (shots on target over shots, goals over xG) → rung 1 stdlib — `f64` division with an explicit guard. Rust has no safe-ratio built-in, and `serde_json` writes a non-finite `f64` as `null` (`serde_json-1.0.151/src/ser.rs:169-180`), so a small `ratio(num, den)` helper returns 0.0 when `den <= 0`.
- Throw-ins and goal kicks in the record → rung 3 reuse — `sim.rs` → `Summary.throw_ins` and `Summary.goal_kicks` exist; add the two fields to `LawStats` and copy them in `LawStats::new`.
- Formations suite → rung 3 reuse with modification — `calibrate/fixtures.rs` → `fixtures()` and `double_round_robin()` supply the clubs; `worker.rs` sharding by `index % shards` (:58–61) and the parent read-back are reused; `MatchConfig::with_tactics` and `Tactics::set_formation` apply the formation. New code: the pairing list and the pairing figures (rung 4, because nothing enumerates formation pairs today).
- Six new formations → rung 3 reuse — `content/tactics.json` slot objects over the ten existing position codes (`data/team.rs:17-28`); data only.
- Failing-band stderr lines → rung 3 reuse — `tracing::warn!` with a `signal` field, as in `calibrate.darkpath` (:397).
- Floor-band distance → rung 4 new code — one branch in `distance()`; no existing helper covers a one-sided band.
- Outlier-only replay files → rung 3 reuse with modification — `RunBuilder::outlier()` becomes a free function `report::is_outlier(&Bands, &MatchStats)` that the worker and the parent share.

## Applied Learnings

No applicable learnings found (`.ai/solutions/INDEX.md` does not exist, and `solutions.globalDir` is not set).

Repeat-deferral tripwire: the two open deferrals (the legibility reading and the macOS build) name a human reader and Apple hardware. This slice's verification needs neither, so the tripwire does not fire.

## Likely Files / Areas to Touch

- `content/realism-bands.json`: version 2 and eleven bands.
- `content/tactics.json`: six formations appended.
- `content/README.md`: the bands with units, values and sources; the new formations.
- `schemas/observability/match-stats.schema.json`: two new required statistics.
- `schemas/observability/run-report.schema.json`: the formations suite, the new figures, `pairing`, `calib.formations`.
- `crates/engine/src/observe/mod.rs`: `LawStats` fields and copy; the literal at :598; the contract-key test at :622–650.
- `crates/engine/src/data/tactics.rs`: the formation count test (:436).
- `crates/engine/tests/resume.rs`: the contract-key list (:70–83).
- `crates/engine-cli/src/report/bands.rs`, `report/mod.rs`, `report/compare.rs`: bands, figures, checks, pairings, distance.
- `crates/engine-cli/src/calibrate/fixtures.rs`, `worker.rs`, `mod.rs`: the formations suite, stderr lines, replay files.
- `crates/engine-cli/src/cli.rs`: `SuiteArg::Formations` and its help (:416–424, :494–499).
- `crates/engine-cli/tests/schemas.rs`, `calibrate.rs`, `calibrate_pair.rs`: the tests.
- `.ai/observability.md`: the contract keys (:33–44, :139–142, :172–173, :225–226).
- `docs/reference/cli.md` (:186–210), `docs/reference/data-files.md` (:25), `docs/how-to/modding.md` (:43).
- Not touched: `dist/stage*/content/` (build output, regenerated by packaging), `.kilo/worktrees/` (another tool's worktree), `docs/design/realism/*` (another programme's staged document).

## Proposed Change Strategy

The engine does not change. Every new number comes from counters the engine already keeps, and the six formations are data that the computer manager never picks by default. So equal-suite and strength-suite matches for a given seed stay the same, and the baseline measures the engine exactly as `5a235a4` plays.

Charter C7 (at-risk, `00-index.md`) commits to realistic statistics "for every shipped formation and after a sending-off". This slice gives C7 its measurement. The fixes belong to the next three slices. Charter C1 (a match in under 2,000 ms on one thread) is not affected, because the per-tick code does not change.

Suite design:
- The equal suite checks the eleven new bands.
- The strength suite does not change.
- The formations suite plays `pairings × matches` fixtures in one worker pool. Fixture `i` belongs to pairing `i / matches` at local index `i % matches`. It takes the clubs of equal-suite fixture `local`. The first formation of the pairing is at home when `local` is even. The parent derives the pairing from the index, as it derives `boosted_side` today, so the record needs no new field. Each pairing gets three checks: `goals_per_match`, `ten_plus_goals_share` and `goalless_share`, each with a `pairing` field such as `"4-3-3 v 4-4-2"`.

Figure definitions, chosen to match how the reference values were measured:
- Shares are per match: a sending-off share counts a match with `laws.red[0] + laws.red[1] > 0` (the source counts a straight red and a second yellow, 15.7%).
- Shots on target share and goals per xG are pooled ratios of totals, because the sources derive them that way (35.0% of all shots; 2.93 goals over 2.96 xG).
- Pass accuracy keeps the mean of per-team percentages (the source is 81.6% per team-match).
- Yellow cards per team include a second yellow, as the engine counts it. The source's 3.83 yellows per match does not count the 0.085 second yellows per match, so the difference is under 0.05 per team.

Time savings, all without fewer matches:
- One pool for all 55 pairings. Workers wait only at the three suite boundaries.
- Step 11 measures the replay-file cost. If writing and deleting non-outlier replay files costs 5% or more of a suite's wall time, the worker decides the outlier itself and writes only outlier files. `events.files_written` then counts the files written, which equals `events.files_kept` on a run with outlier retention.
- The 4-4-2 mirror pairing is not taken from the equal suite. It would save 2% and would couple two suites.

## Step-by-Step Plan

1. **Bands file version 2.**
   1. Set `schema_version` to 2 in `content/realism-bands.json`.
   2. Add `ten_plus_goals_share` {0, 0.005}, `sending_off_share` {0.08, 0.22}, `yellow_cards_per_team` {1.2, 2.6}, `shots_on_target_share` {0.30, 0.42}, `goals_per_xg` {0.85, 1.15}, `passes_per_team` {350, 550}, `pass_accuracy_pct` {75, 88}, `corners_per_team` {3.5, 6.5}, `throw_ins_per_match` {35, 55}, `goal_kicks_per_match` {12, 22} and `goalless_share` {0.04, 0.12}. Shares are fractions from 0 to 1, as `min_win_rate` is. Percentages stay as `possession_pct` is.
   3. In `report/bands.rs`, set `BANDS_VERSION` to 2 and add the eleven `Band` fields with `#[garde(dive)]`. Add `Band::is_floor()` (`lo == 0.0`).
   4. Update the pinned-values test. Add a test that a version-1 file is refused with a message that names the file and the version. Add a test that a file without `goals_per_xg` is refused and names that field.
2. **Record fields.** Add `#[serde(rename = "stats.throw_ins")] throw_ins: [u32; 2]` and `stats.goal_kicks` to `LawStats`. Copy both in `LawStats::new`. Update the literals at `worker.rs:207` and `observe/mod.rs:598`, the contract-key test (`observe/mod.rs:622-650`) and `tests/resume.rs:70-83`.
3. **Match-stats schema.** Declare both keys as two-element arrays of integers of 0 or more, as `stats.corners` is (:173–181). Add both to the success `required` list. In `tests/schemas.rs`, assert that a real `simulate` record carries both keys, and that a record without `stats.throw_ins` is refused.
4. **Figures.** Add these fields to `SuiteFigures`, each from `outcome == "success"` records: `ten_plus_goals_share`, `goalless_share`, `sending_off_share`, `yellow_cards_per_team_mean`, `shots_on_target_share`, `goals_per_xg`, `corners_per_team_mean`, `throw_ins_per_match_mean` and `goal_kicks_per_match_mean`. Add `ratio(num, den)`, which returns 0.0 when `den <= 0`. A 0.0 ratio fails its band, because every ratio band has `lo > 0`.
5. **Equal-suite checks.** Add the eleven checks to `checks()` after the existing four, in the band-file order. Use one local helper for a `Band` check so each check is one line.
6. **Formation data.** Append six formations to `content/tactics.json` after 3-5-2, modelled on the shipped slot style (x from the own goal line, y across the pitch):
   - `4-1-4-1`: back four as 4-4-2; DM (36, 0); LW (48, −24), CM (48, −8), CM (48, 8), RW (48, 24); ST (66, 0).
   - `4-4-1-1`: back four and midfield four as 4-4-2; AM (57, 0); ST (66, 0).
   - `4-1-2-1-2`: back four; DM (36, 0); CM (45, −14), CM (45, 14); AM (55, 0); ST (65, −8), ST (65, 8).
   - `3-4-3`: CB (24, −14), CB (22, 0), CB (24, 14); LB (44, −26), CM (44, −8), CM (44, 8), RB (44, 26); LW (62, −22), ST (66, 0), RW (62, 22).
   - `5-3-2`: LB (30, −26), CB (23, −14), CB (21, 0), CB (23, 14), RB (30, 26); CM (43, −15), DM (38, 0), CM (43, 15); ST (63, −8), ST (63, 8).
   - `5-4-1`: back five as 5-3-2; LW (46, −24), CM (44, −8), CM (44, 8), RW (46, 24); ST (64, 0).
   Update the count test in `data/tactics.rs:436` to 10 and pin the names in order. The content validator must accept every new formation.
7. **Formation pairings.** In `calibrate/fixtures.rs`, add `pairings(n) -> Vec<[usize; 2]>`, which lists every `[a, b]` with `a <= b` (55 for 10 formations). Add `formation_fixture(matches, pairings, i)`, which gives the pairing, the local index, the clubs of equal fixture `local` and the home side. Add tests: 55 pairings for 10; the formation at home alternates; the clubs at local index `k` equal the equal-suite clubs at `k`.
8. **Formations suite.** Add `Suite::Formations` (code `formations`, `number()` 2) and `SuiteArg::Formations`. Include it in `Suite::ALL`, so `--suite all` runs it. Plan `pairings × matches` fixtures for it. In the worker, give each side `Tactics::defaults(schema).set_formation(idx, schema)` through `MatchConfig::with_tactics`. In the parent, pass the pairing to the builder with each record. Update the `--suite` help in `cli.rs` to list `formations`. Every help line must stay under 80 columns.
9. **Pairing figures and checks.** Add `PairingFigures { pairing: [String; 2], matches, goals_per_match_mean, goals_for_mean: [f64; 2], ten_plus_goals_share, goalless_share }` and write the list as `calib.formations`. Add `pairing: Option<String>` to `BandCheck`, skipped when absent. Emit the three goal-band checks for each pairing. Sort the checks by suite, band and pairing.
10. **Failing bands on stderr.** After the checks are computed, emit one `tracing::warn!` for each failing check: `signal = "calibrate.band_failed"`, with `run.id`, `suite`, `band`, `pairing` when present, `value`, `lo` and `hi`. The line is visible at the default `SM_LOG=warn` level. Paired runs emit the lines too, before the table.
11. **Replay-file cost (measured gate).**
    1. On the release build, run `engine-cli calibrate --seed 42 --suite equal --matches 1000 --keep-events outliers` and then the same with `--keep-events all`. Record both `calib.wall_ms` figures.
    2. If writing non-outlier replay files costs 5% or more, move the predicate to a free function `report::is_outlier(&Bands, &MatchStats)`. Make the worker buffer a match's events and write the file only for an outlier when retention is `outliers`. Keep the parent's delete for the retention `all` case.
    3. If the cost is under 5%, record the figures and make no change.
12. **Paired-run distance.** In `compare.rs`, key rows by suite, band and pairing. In `distance()`, for a floor band (`lo == 0` and `hi > 0`), add `max(0, v − hi) / hi`. Other rows keep today's formula. Add tests: a floor band at 0 adds 0.0; at twice `hi` it adds 1.0; the goals row is unchanged.
13. **Run-report schema, contract and documents.**
    1. Add `formations` to the three suite enums. Add the new suite figures and require them. Add an optional `pairing` string to the `calib.bands` and `calib.compare` items. Add a `calib.formations` array of pairing figures, optional because `--suite equal` omits it.
    2. Update `.ai/observability.md`: the two record keys, the version-2 bands, the formations suite, `calib.formations`, `pairing`, and the signal `calibrate.band_failed`.
    3. Update `content/README.md` with every band's unit, range, real value and source (file and line, as in Current State), and the six formations. Update `docs/reference/cli.md` (the `--suite` values, the pairing count, about 74 minutes per seed on the reference machine, the stderr lines), `docs/reference/data-files.md:25` and `docs/how-to/modding.md:43`.
14. **Tests and the baseline.**
    1. Update the report unit tests. Extend the `record()` helper (`report/mod.rs:531-570`) with the new figures, and give it realistic defaults so the existing "all pass" test stays meaningful. Re-pin the equal-suite band list and the failed list.
    2. Update the pinned rows in `calibrate_pair.rs:164-173`. Keep that test on `--suite equal` so it stays fast.
    3. In `calibrate.rs`, add a formations smoke test: `calibrate --seed 1 --suite formations --matches 1 --minutes 5 --jobs 2` writes a valid report with 55 `calib.formations` rows, each with `matches` 1. Add a test that stderr names a failing band. Make the slow test assert every band and exit 0 (Q-P3).
    4. Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`. Everything must pass except the ignored slow test.
    5. Check every pinned content hash and snapshot fixture that `tactics.json` moves. Re-derive each one and record it in the implement record.
    6. Build the release binary. For each seed in 42, 1, 7, 99 and 2026, run `engine-cli calibrate --seed <s> --matches 1000 --out <scratch>/baseline/s<s>`. Each run must exit 2. Record every band's value, range and verdict per seed, and every failing pairing, in `05-implement-realism-bands-v2.md` as the baseline (Q-P9).
    7. Run `engine-cli bench --seed 42 --matches 5 --json`. The figures must stay within the tripwires in `05c-benchmark.md` (+10% CPU per tick, +25% peak memory).

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| Every band is reported | `engine-cli calibrate --seed 42` on the release binary, read `report.json` and the exit code (cli-direct, live) | Windows reference machine, release build — yes | nothing beyond steps 1–5 | the ignored slow test → the smoke test with `--minutes 5` (proxy for shape only) |
| The baseline is recorded | the same run, read stderr for `calibrate.band_failed` lines (cli-direct, live) | same — yes | step 10 | none needed |
| Formations are reported | `engine-cli calibrate --seed 42 --suite formations`, read `calib.formations` and the pairing checks (cli-direct, live) | same — yes; about 72 minutes | steps 6–9 | the smoke test with 1 match per pairing (shape only) |
| New keys pass the schema | `engine-cli simulate --seed 7`, validate `stats.json` with the schema test (cli-direct, headless validator) | same — yes | step 3 | none needed |

No acceptance criterion depends on an environment outside the reference machine, so no `constraint-resolution:` line is needed.

## Test / Verification Plan

### Automated checks

- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo test -p engine-cli --bin engine-cli report` and `calibrate`: bands, figures, checks, pairings, distance.
- `cargo test -p engine-cli --test schemas --test calibrate --test calibrate_pair --test cli_args --test docs`.
- `cargo test -p engine --test resume` and the `data::tactics` and `observe` unit tests.
- `cargo test --workspace`: every test passes except the ignored slow test.
- Web unit tests (`node --test web/tests`) still pass with 10 formations.
- The ignored slow test is run once on the release build and is expected to fail on the new bands. Its output goes in the implement record.

### Interactive verification (human-in-the-loop)

- **The baseline (CLI).** Drive the release `engine-cli.exe` directly, with `SM_DATA_DIR` in a scratch folder, for the five seeds in step 14.6. Capture `report.json`, stdout and stderr for each seed under `implement-evidence/realism-bands-v2/` and `verify-evidence/realism-bands-v2/`. Pass: every run exits 2, every report validates, and stderr names each failing band.
- **The new formations in the viewer (web).** The formation list is user-visible. Start `engine-cli launch`. Open the page in the in-app browser (`Claude_Browser`, the chosen driver, Q29). Open the lineup editor and read the `formation` select (`data-testid="formation"`). Pass: 10 options, the new six after the shipped four. Pick 5-4-1, start the match, and take a screenshot of the pitch. Pass: 11 home markers in the 5-4-1 shape. Save the screenshots in `verify-evidence/realism-bands-v2/`.

## Risks / Watchouts

- **Run time (high).** `--suite all` plays 57,000 matches: about 74 minutes per seed, and about 6 hours for the baseline. Run the baseline in the background, one seed at a time, and record the wall time of each. A paired run (`--pair`) plays both arms, so it takes about 2.5 hours.
- **Expected red runs (medium).** Calibration exits 2 and the slow test fails until `realism-tuning`. The implement record states this at its top.
- **Content hash (medium).** New formations change the content hash. A snapshot saved before this slice is refused on resume (`snapshot.rs:239`). This is designed behaviour. Pinned hashes and replay fixtures (for example `web/tests/data/one-minute.smfx`) are checked in step 14.5.
- **Compare rows grow (medium).** A paired run now has 11 + 165 more rows. The pass count comes first in the verdict, so the formations rows weigh most. This is intended: the formations are where the regression shows.
- **Formation slot positions (low).** The coordinates in step 6 are a starting point. The defending slice may move them. Because indices 0 to 3 do not move, saved lineups keep their formation.
- **Next slices' criteria.** `defending-and-discipline` measures each formation against 4-4-2 at no more than 4.0 goals per side, and `realism-tuning` tightens this to the goals band. With the all-against-all suite (Q-P1), their plans read those criteria over all 55 pairings.

## Dependencies on Other Slices

- `calibration` (complete): the harness, the bands loader and the report that this slice extends.
- `probe-engine-core` (complete): the `error` outcome and the schema shape. Records with `outcome: error` stay out of every figure.
- `tactics-and-ai` (complete): `Tactics::set_formation` and the computer manager, which keeps the formation in play.
- Later: `defending-and-discipline`, `keeper-and-shots`, `tempo-and-restarts` and `realism-tuning` read this slice's baseline and bands.

## Assumptions

- A1 (class: implementation-detail): shares are stored as fractions and percentages as 0–100, matching the existing file.
- A2 (class: implementation-detail): the formations suite checks the three goal bands, reading "check goal bands" in Q-P1 as goals per match, the 10 or more goals share and the goalless share.
- A3 (class: implementation-detail): pairings are unordered with mirrors, and home and away alternate inside each pairing, so 10 formations give 55 pairings.
- A4 (class: implementation-detail): the 4-4-2 mirror pairing is played in the formations suite and not reused from the equal suite. The saving is 2%.
- A5 (class: implementation-detail): no augmentation is re-authored. The engine's per-tick code does not change, so the benchmark baseline in `05c-benchmark.md` stays valid and step 14.7 re-checks it. The new keys join the contract in step 13 and add no dark path, so `04b-instrument.md` does not change. `04c-experiment.md` is not involved.
- A6 (class: implementation-detail): no consult ran. The trigger `appetite-medium-or-larger` holds, but the product owner excluded `consult` at intake (`stack.excluded-by-po`), so `consult-runs: []`.

## Blockers

None.

## Freshness Research

- `serde` 1.0.229, `serde_derive` 1.0.229, `serde_json` 1.0.151 and `boon` 0.6.1 are the newest stable releases on crates.io (crates.io API, read in this run). No upgrade is needed.
- `deny_unknown_fields` refuses extra keys only. A missing field is refused unless it has `#[serde(default)]` or is an `Option` (`serde_derive-1.0.229/src/de.rs:766-806`; `serde-1.0.229/src/private/de.rs:24-60`). So required new fields give Q-P7 its refusal with no extra code.
- `serde_json` writes a NaN or an infinite `f64` as `null` (`serde_json-1.0.151/src/ser.rs:169-180`). The `ratio` guard in step 4 prevents a `null` figure.
- `boon` uses draft 2020-12 by default (`boon-0.6.1/src/compiler.rs:66-71`), and a `required` inside `then` or `else` applies only on that branch (`validator.rs:736-744`). So the new record keys are required on success records only.

## Recommended Next Stage

- **Option A (default): Implement** → `/wf implement football-manager-match-engine realism-bands-v2`. The plan is complete and has no blocker. Compact the session first. The baseline in step 14.6 runs for about 6 hours, so start it in the background.
- **Option C: Revisit slice** → `/wf slice football-manager-match-engine`. Choose this only if the all-against-all formations suite should move to its own slice.
