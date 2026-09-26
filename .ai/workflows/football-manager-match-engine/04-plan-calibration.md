---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: calibration
status: complete
stage-number: 4
created-at: "2026-09-22T22:16:48Z"
updated-at: "2026-09-22T22:16:48Z"
metric-files-to-touch: 32
metric-step-count: 22
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, calibration, statistics, observability]
stack-source: confirmed
steering-honored:
  - "steer.md holds design direction for the viewer only; this slice changes no page, so no design constraint applies"
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-calibration.md
  shape: 02-shape.md
  observability: ../../observability.md
  instrument: 04b-instrument.md
  benchmark: 05c-benchmark.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md]
  implement: 05-implement-calibration.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine calibration"
---

# Plan: Calibration Harness and Statistics Record

## The Plan

The engine plays a lawful match and writes one statistics record per `simulate` run, but it counts no shots, no passes, and no possession (`crates/engine/src/sim.rs:164-187`). Headless runs write no event stream file: only `serve` opens `events.jsonl` (`crates/engine-cli/src/serve.rs:70`). The observability contract names three schema files at `schemas/observability/` and no such folder exists. The benchmark times one match on one thread, and no command runs many matches. The tree builds on `837cb5c`. In a release test run of the workspace this session, 191 tests pass and 1 fails: `stream_cli.rs` `a_viewer_that_closes_in_mid_match_ends_the_run_without_an_error` at line 174. The failure is in a stream test, outside this slice. The tactics slice is defined and not built, so this plan names the seams it needs from that slice and re-checks them in step 1.

Eighteen decisions were settled without a product owner in the loop. Every one is an implementation detail, and each is recorded under Assumptions. The bands are judged as aggregates over 1000 matches, as shape AC-3 words it ("When statistics aggregate"). One `calibrate` run holds two suites of 1000 matches each: equal strength, and a 15 percent attribute boost. Parallel work runs in worker processes of the same binary, as the slice scope says. Each match writes its statistics record and its event file. At run end, the event files of in-band matches are pruned, per the product owner's retention answer. The records are validated in tests against JSON Schema files by `boon` 0.6, a pure-Rust validator added for tests only. The bands live in `content/realism-bands.json` as data and are never tuned.

Implement touches 32 files, 12 of them new, in 22 steps. The order is counters, then records and schemas, then the harness, then the tuning pass, then the full run. Integration consumes the tuned engine, and the feature-flags slice reuses the harness. The top risk is the tuning loop: the bands may not be reachable with the decision model the tactics slice ships. A miss is recorded as an open tuning item with its measured distance, and the bands are never widened.

## Current State

- Branch `feat/football-manager-match-engine` at `837cb5c`. Five slices are implemented: engine-core, data-schemas-generator, stream-protocol, viewer-pitch, and match-rules. `tactics-and-ai` is `status: defined` in `00-index.md` and has no plan. `cargo test --workspace --release --no-fail-fast` ran this session: 191 passed, 1 failed (`crates/engine-cli/tests/stream_cli.rs:174`, "the viewer disconnected"). The failure is outside this slice's files.
- `Summary` (`crates/engine/src/sim.rs:164-187`) counts goals, fouls, offsides, corners, throw-ins, goal kicks, free kicks, penalties, cards, added time, stoppages, dead-ball ticks, and offside checks. It has no counter for shots, passes, completed passes, possession time, or xG. The kick is resolved at `sim.rs:446` (`Kick::Pass` and `Kick::Shot`), and possession changes at `sim.rs:499-577`.
- `MatchStats` (`crates/engine/src/observe/mod.rs:64-93`) carries `goals: [u32; 2]` and the law counts. It lacks the contract keys `stats.goals`, `stats.shots`, `stats.shots_on_target`, `stats.xg`, `stats.passes`, `stats.pass_accuracy_pct`, `stats.possession_pct`, and `darkpath.change_never_applied` (`.ai/observability.md:33-59`). `RunReport` (`observe/mod.rs:163-205`) is the benchmark record only.
- `read_stats` (`observe/mod.rs:268-291`) fails closed on record kind and schema version, and `to_json` puts the envelope keys first (`observe/mod.rs:225-241`).
- `EventWriter` (`crates/stream/src/events.rs:48-81`) opens only `matches/<match.id>/events.jsonl`. `match_event` (`crates/engine-cli/src/stream_run.rs:127-157`) maps engine events to `MatchEvent` and is private to the stream driver.
- `simulate` (`crates/engine-cli/src/simulate.rs:17-91`) runs the validator over every tick record and writes `stats.json`. It writes no events file.
- `bench::run_one` (`crates/engine-cli/src/bench.rs:98-106`) times one match into a `NullSink`, and `process::cpu_time_ms` and `process::peak_memory_mb` give the processor time and memory.
- `generate_league(seed, clubs, &Content)` (`crates/engine/src/data/generator.rs:13`) makes seeded clubs. Attributes range 1 to 100 (`po-answers.md`, data-schemas-generator plan Q5).
- The observability contract (`.ai/observability.md`) is plan-version 1 and locked (`po-answers.md`, observability init Q5). It gives the run folder layout `SM_DATA_DIR/runs/<run.id>/report.json` and `stats/<match.id>.json` (line 106) and the builder location `crates/engine-cli/src/report/` (line 78). The schema files are planned at `schemas/observability/*.schema.json` (line 152). The retention answer keeps the report, every statistics record, and the event streams of outlier and failed matches only (`po-answers.md`, observability init Q4).
- No test is marked `#[ignore]` in the workspace (searched this session). The reference machine is an AMD Ryzen 7 9800X3D with 8 cores and 61.6 GB of memory (read this session).

## Simplicity Ladder

| Capability | Rung | Choice |
|---|---|---|
| Parallel match execution | rung 1 stdlib | `std::process::Command` on `std::env::current_exe()` plus `std::thread::available_parallelism()`. No `rayon`; the slice asks for processes. |
| League and fixture generation | rung 3 reuse | `crates/engine/src/data/generator.rs` → `generate_league()`; exact match; reuse as-is. The round-robin fixture list is rung 4 (about 20 lines; no in-repo scheduler). |
| Attribute boost for the stronger team | rung 4 new code | A map over `TeamFile.players[].attributes`. No in-repo transform exists. |
| Single-thread per-match time | rung 3 reuse with modification | `crates/engine-cli/src/bench.rs` → `run_one()` and the median logic in `run()`; extract `measure()` and keep `bench` output byte-compatible. |
| Per-match statistics record | rung 3 reuse with modification | `observe/mod.rs` → `MatchStats`, `to_json()`, `read_stats()`; add a flattened figures block. |
| Event stream file | rung 3 reuse with modification | `crates/stream/src/events.rs` → `EventWriter`; add `create_at(path)`. `stream_run.rs` → `match_event()`; widen visibility. |
| Shot, pass, and possession counters | rung 4 new code | Counters in `Summary` at the two existing resolution points. None exists (`sim.rs:164-187`). Reused if the tactics slice adds one. |
| xG per shot | rung 4 new code | One logistic function of distance and angle with coefficients in the tuning file. No model exists. |
| Record validation in tests | rung 3 new dev-dependency | `boon` 0.6.1: JSON Schema draft 2020-12, MIT OR Apache-2.0. Its dependency list has no C code and no network (read from `~/.cargo/registry/cache/*/boon-0.6.1.crate`, `Cargo.toml`). The API is `Compiler::add_resource`, `Compiler::compile`, and `Schemas::validate(&Value, SchemaIndex)` (`boon-0.6.1/src/compiler.rs:182-205`, `src/lib.rs:186-196`). The standard library has no JSON Schema validator. `jsonschema` 0.57 was rejected: its default features include `tls-aws-lc-rs` (read with `cargo info jsonschema@0.57.0`), and this machine has no C compiler (`00-index.md` `toolchains-absent: [clang, cl]`). |
| Band data loading | rung 3 reuse | `serde` with `deny_unknown_fields` and the `schema_version`-first pattern of `crates/engine/src/data/mod.rs`; `ContentDir::resolve` for the path. |
| Aggregation and band checks | rung 4 new code | Mean, standard deviation, and share in band over at most 1000 values. Pure functions; no statistics crate needed. |

## Applied Learnings

No applicable learnings found: `.ai/solutions/INDEX.md` does not exist, and `.ai/sdlc-config.json` does not exist, so no global folder is set. `runtime-evidence-deferrals` in `00-index.md` is empty, so the repeat-deferral tripwire does not fire.

## Likely Files / Areas to Touch

- `schemas/observability/match-event.schema.json`, `match-stats.schema.json`, `run-report.schema.json` (new): the agreed record schemas that the contract plans at this path.
- `content/realism-bands.json` (new): the accepted bands as data (contract `additional-contracts: realism-bands`).
- `content/tuning.json`, `content/README.md`: the xG coefficients and the tuning-pass changes.
- `crates/engine/src/sim.rs`, `tuning.rs`, `snapshot.rs`: the counters, the xG function, and the snapshot fields for the new counters.
- `crates/engine/src/observe/mod.rs`: statistics figures, the calibration report record, and a writer for the run folder.
- `crates/engine/tests/match_stats.rs` (new), `content.rs`, `full_match.rs`, `identity.rs`: counter tests, pinned values, the sanity band, and the literal that builds `MatchStats`.
- `crates/stream/src/events.rs`: an event writer at any path.
- `crates/engine-cli/src/calibrate/{mod,fixtures,worker}.rs` (new), `report/{mod,bands}.rs` (new): the harness.
- `crates/engine-cli/src/cli.rs`, `main.rs`, `simulate.rs`, `resume.rs`, `bench.rs`, `stream_run.rs`: the command, event files from `simulate`, the new figures, and the extracted benchmark.
- `crates/engine-cli/tests/schemas.rs`, `calibrate.rs` (new): the schema criterion, the smoke run, and the slow runs.
- `Cargo.toml`, `crates/engine-cli/Cargo.toml`, `Cargo.lock`: `boon` for tests.
- `README.md`: the calibrate command.

## Proposed Change Strategy

Counters come first, because every band reads them. Shots and passes are counted where a kick is resolved (`sim.rs:446`). Completed passes and possession time are counted where control changes (`sim.rs:499-577`). Possession is the share of open-play ticks after the last controlling team's touch. xG is a logistic function of shot distance and angle. Any counter the tactics slice has already added is reused, never duplicated. Each new `Summary` field goes into the snapshot in the same step, per the match-rules cross-cutting rule, and the snapshot format version rises by one.

The records follow the contract. `MatchStats` gains a flattened figures block with the contract keys as home-first arrays. The existing `goals` field stays as an additive extra, so older `stats.json` files still load. New fields carry `#[serde(default)]`. The schema version stays `"1"`, because every change adds a field, as the law fields of the previous slice did. The three schema files are written from the contract's key list, with `additionalProperties: true` so additive extras pass. The tests validate real output from the binary, not the structs, so a drift between code and schema fails loudly.

The harness has two layers. The parent process plans the fixtures, starts `--jobs` workers, and waits. It then runs the single-thread benchmark on an idle machine, aggregates the records, prunes the event files, and writes one `run-report`. A worker is the same binary with hidden flags. It runs its share of the fixtures and writes one statistics file and one event file per match, into the run folder. A crashed worker leaves its statistics files missing, and `darkpath.match_without_stats` counts them. That dark path is named by the contract, and it is measured instead of assumed. NFR-1 (`yields-to: C2`) is the budget: the run's wall time is reported against 30 minutes per 1000 matches, and the per-tick model is not changed to meet it.

Tuning moves only values in `content/tuning.json`, within their `garde` ranges. It follows a fixed protocol (step 18), and every iteration is logged. The bands in `content/realism-bands.json` are accepted criteria and are never edited by the tuning pass.

## Step-by-Step Plan

1. **Pre-flight: read the tactics seams.** Confirm that `05-implement-tactics-and-ai.md` and `06-verify-tactics-and-ai.md` exist. Then read from the landed code: (a) how a `MatchConfig` puts the computer manager on both sides; (b) the change-queue state at full time (queued, applied, and rejected counts); (c) whether `Summary` already counts shots, passes, or possession. If (a) or (b) differs from the Assumptions below, stop and run `/wf plan football-manager-match-engine calibration` so the plan is re-reviewed. Change no file in this step.
2. **Counters.** In `crates/engine/src/sim.rs`, add `shots`, `shots_on_target`, `passes`, `passes_completed`, `possession_ticks` (all `[u32; 2]`) and `xg: [f64; 2]` to `Summary`, skipping any counter that step 1 found. Count a shot and a pass at the kick branch. A shot is on target when it produces a goal or a goalkeeper save or catch. A pass is completed when the next player to take control is on the passer's team. `possession_ticks` increments on open-play ticks for the team of the last controlling touch.
3. **xG.** In `crates/engine/src/tuning.rs`, add `XgTuning { intercept, distance_coef, angle_coef }` with `garde` ranges. Add the same block to `content/tuning.json`, then add rows to `content/README.md` with unit, default, and bound. xG of a shot is `1 / (1 + exp(-(intercept + distance_coef·d + angle_coef·θ)))`, where d is the distance to the goal centre and θ is the angle the goal mouth subtends. The start values are intercept −1.0, distance coefficient −0.1, and angle coefficient 1.0. Add each shot's xG to `Summary.xg` at the kick branch.
4. **Snapshot fields.** In `crates/engine/src/snapshot.rs`, extend `encode_summary` and `decode_summary` (`snapshot.rs:532`, `snapshot.rs:651`) with the new fields, and raise the snapshot format version by one. Run the existing continuation test in `crates/engine/tests/snapshot.rs` unchanged.
5. **Counter tests.** Add `crates/engine/tests/match_stats.rs` and use the `scenario` feature scenes: a scripted shot increments `shots` and adds an xG strictly between 0 and 1; a pass received by a team-mate increments `passes_completed`; the two teams' possession percentages over the seed-42 full match sum to 100 within 0.1.
6. **Statistics figures.** In `crates/engine/src/observe/mod.rs`, add a `MatchFigures` block, flattened into `MatchStats` with `#[serde(default)]`. Its keys are `stats.goals`, `stats.shots`, `stats.shots_on_target`, `stats.xg` (two decimals), `stats.passes`, `stats.pass_accuracy_pct` (one decimal), `stats.possession_pct` (one decimal), `darkpath.change_never_applied`, and `manager.kind` (`["human"|"ai", …]`). Add `MatchFigures::new(&Summary, change_never_applied, managers)`. Update the constructors in `simulate.rs`, `resume.rs`, and `crates/engine/tests/identity.rs`. Extend `both_records_carry_every_contract_key` with the new keys.
7. **Run-folder writer.** In `observe/mod.rs`, add `write_stats_at(dir, &MatchStats)`, which writes `<dir>/<match.id>.json`. Make `write_stats` call it with `matches/<match.id>/stats.json` kept as today. Add `EventWriter::create_at(path)` to `crates/stream/src/events.rs`, and make `open` call it.
8. **Shared event mapping.** In `crates/engine-cli/src/stream_run.rs`, change `match_event` to `pub(crate) fn match_event(event, owner_id, match_id, club_ids, player_ids)`, and update both call sites. The wire output stays byte-identical, and `crates/stream/tests/fixture.rs` must stay green.
9. **Event file from `simulate`.** In `crates/engine-cli/src/simulate.rs`, open `EventWriter::open(&data, &match_id)` and write every `sim.take_events()` event through `match_event` before the validator runs. `stats.json` and `events.jsonl` then sit side by side, as `serve` already places them.
10. **Schema files.** Write `schemas/observability/match-event.schema.json`, `match-stats.schema.json`, and `run-report.schema.json` (draft 2020-12). Required keys: every record requires the envelope keys (`record.kind` as a `const`, `schema.version` as `const "1"`, `owner.id`, `service`, `version`, `build.hash`, `env`, `operation`). The match-event and match-stats files also require the correlation keys and every contract key that the slice emits. `run-report` uses an `if`/`then` on `operation`: `benchmark` requires the current `bench.*` keys, and `calibrate` requires the keys of step 13. Every file sets `additionalProperties: true`. Each file carries a `$id` of the form `https://touchline.local/schemas/observability/<kind>.schema.json`, so `boon` resolves the schema without a network fetch.
11. **Band data.** Write `content/realism-bands.json` with `schema_version: 1`. Its values: goals per match 2.4 to 3.2; shots per team 8 to 16; possession 35 to 65 percent; stronger-team win rate above 0.50 at a 1.15 attribute boost; sample size 1000; run wall time under 30 minutes. Add `crates/engine-cli/src/report/bands.rs`: it loads the file through `ContentDir::resolve`, checks `schema_version` first, and refuses with the file and field named. Add the file to `content/README.md` and state that the bands are accepted criteria, never tuned values.
12. **Benchmark extraction.** In `crates/engine-cli/src/bench.rs`, extract `pub(crate) fn measure(config, matches) -> BenchFigures`. It returns the median wall time, the ticks per match, the processor time per tick, and peak memory, with the warm-up included. `bench::run` calls it. Its output must stay key-for-key identical: check by diffing the key set of one `bench --seed 42 --matches 1 --json` line before and after the change.
13. **Report builder.** Add `crates/engine-cli/src/report/mod.rs` with `RunBuilder`. It folds `MatchStats` records per suite. Equal suite: mean and standard deviation of goals per match, mean shots per team, mean possession for home and for away, and the share of matches with both possessions in 35 to 65. Strength suite: the stronger team's wins, draws, and losses, and its win rate. It computes the band checks `[{ band, value, lo, hi, pass }]` and sums the dark paths. `outlier(&MatchStats)` is true on failure, on validator violations, on a match slower than 2000 ms, on a dark-path hit, on either possession outside 35 to 65, or on either team's shots outside 8 to 16. Add `CalibrationReport` to `observe/mod.rs` as `record.kind` `run-report` with `operation` `calibrate`. Its keys: `run.id`, `seed`, `calib.matches`, `calib.jobs`, `calib.suites`, `calib.wall_ms` per suite and in total, `bench.match_wall_ms`, `bench.cpu_us_per_tick`, `bench.peak_mem_mb`, `darkpath.change_never_applied`, `darkpath.match_without_stats`, `validate.violations`, `events.files_written`, `events.files_kept`, `machine.hash`, `machine.cpu_model`, `outcome`, and `calib.pass`. Unit-test the aggregates with hand-built records, including one missing record.
14. **Fixtures.** Add `crates/engine-cli/src/calibrate/fixtures.rs`. League k is `generate_league(seed + k, 20, &content)`. A league has a double round-robin of 380 fixtures, and leagues are generated until `matches` fixtures exist. For the strength suite, the home club of fixture i has every attribute multiplied by 1.15, rounded, and clamped to 1 to 100. The boosted club plays at home on even i and away on odd i. The match seed is `splitmix64(seed ^ (suite << 32) ^ i)`, and a shard owns `i % shards == shard`. Unit-test that the same seed yields the same fixture list, and that the boost clamps at 100.
15. **Worker.** Add `crates/engine-cli/src/calibrate/worker.rs`. For each owned fixture: build `MatchConfig` with the computer manager on both sides (the seam from step 1), run into a `VecSink`, run the `Validator`, and build `MatchStats` with the figures. Then write `stats/<match.id>.json` with `write_stats_at`, and write `events/<match.id>.jsonl` with `EventWriter::create_at`, both under the run folder. An engine error writes a `MatchStats` record with `outcome: "failure"` and `error.type`, and the worker continues with the next fixture.
16. **Command.** In `crates/engine-cli/src/cli.rs`, add `Calibrate` with these arguments: `--seed` (required); `--matches` (default 1000, applied to each suite); `--minutes` (default 90); `--jobs` (default `available_parallelism`); `--suite all|equal|strength` (default `all`); `--out` (default `SM_DATA_DIR/runs/<run.id>`); and `--keep-events outliers|all` (default `outliers`). Add the hidden `--worker`, `--shard`, `--shards`, and `--run-dir` flags with `#[arg(hide = true)]`. Keep help lines under 80 columns. Dispatch the command in `main.rs`.
17. **Parent.** Add `crates/engine-cli/src/calibrate/mod.rs`. For each suite, it starts `jobs` children of `current_exe()` with the worker flags, stdout to null and stderr inherited, then waits and times the suite. It then runs `bench::measure` for 5 matches on the default teams, loads every `stats/*.json` with `read_stats`, and counts missing records as `darkpath.match_without_stats`. It deletes the `events/*.jsonl` of non-outlier matches unless `--keep-events all` is set. It writes `report.json` with `to_json`, prints the same line on stdout, and emits `tracing::warn!(signal = "calibrate.darkpath", …)` for any non-zero dark path. Exit 0 when every band, both dark paths, and the time budget pass; otherwise exit 2. Record `run.id` as `calib-<seed:016x>-<millis>`.
18. **Tuning pass.** Build release. Each iteration runs `engine-cli calibrate --matches 200 --seed 2026 --jobs 8` and changes one knob group of `content/tuning.json`. The groups, in order:
    1. shot volume: `shot_range`, `press_distance`;
    2. conversion: `shot_noise`, `aim_noise`, `keeper_catch_chance`, `keeper_reach`;
    3. possession balance: `control_cooldown_ticks`, `foul_ball_loss`;
    4. the strength effect: the attribute-weighted terms of tackles, passes, and shots that the tuning file exposes;
    5. the generator's per-position means, only if the equal suite's aggregates drift from the band centre.

    Log every iteration as one row (iteration, knob, old value, new value, the four band values) in `implement-evidence/calibration/tuning-log.md`. The cap is 12 iterations. After the loop, update the pinned values in `crates/engine/tests/content.rs` and the sanity band in `full_match.rs`. If a band still misses, record it under the implement artifact's residuals as an open tuning item with its measured distance, routed to the feature-flags work, and leave `content/realism-bands.json` unchanged.
19. **Tests.** Add `crates/engine-cli/tests/schemas.rs` for AC-c. It runs `simulate --seed 7 --minutes 5` with `SM_DATA_DIR` in a temporary folder. It then validates `stats.json` and every `events.jsonl` line against the schema files with `boon`, unchanged. It also asserts that a stats record without `match.id` is rejected, and so is a record with `schema.version` `"9"`. Add `crates/engine-cli/tests/calibrate.rs`:
    - Smoke test, in the default run: `calibrate --seed 1 --matches 4 --minutes 5 --jobs 2` writes 8 statistics files. Every file and `report.json` validate against their schemas. Both dark paths read 0, `events.files_written` is 8, and the exit code is 0 or 2, because short matches are not judged against the bands.
    - Slow test, `#[ignore = "slow: 2000 full matches; run in release with --ignored"]`: `calibrate --seed 2026 --matches 1000` asserts AC-a, AC-b, AC-d, and AC-e from `report.json`, with exit code 0.
20. **Full run and benchmark.** In release, run `engine-cli calibrate --seed 2026 --matches 1000` (the slice fixture) and keep its stdout, stderr, exit code, and `report.json` as evidence. Then rerun `engine-cli bench --seed 42 --matches 5 --json` three times, and compare the figures against the tactics slice's recorded baseline under the per-tick gate (match-rules plan Round 4 Q13), because the tuning pass changes engine behaviour.
21. **Docs.** Add a calibrate section to `README.md`: the command, the run folder layout, the bands file, exit codes 0 and 2, and the single-thread figure.
22. **Gates and boundary.** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `node --test web/tests/*.test.mjs`. Search the changed source, comments, and docs for workflow vocabulary (stage names, slice slugs, `.ai/`) and rewrite any hit in product language before the commit.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-e: 1000 matches on the reference machine run in under 30 minutes, and the report records the single-thread per-match time from the benchmark harness | `engine-cli calibrate --seed 2026 --matches 1000`, release build, read `report.json` (`cli-direct`, cli adapter) | Windows 11 reference machine, Ryzen 7 9800X3D, 8 cores, rustc 1.92: yes, this machine | `calib.wall_ms` per suite and in total; `bench.match_wall_ms` and `bench.cpu_us_per_tick` from `bench::measure` (steps 12, 13, 17); evidence capture of stdout, stderr, exit code, and `report.json` | rerun with `--jobs 4` if another heavy process skewed the first run → run from a fresh PowerShell with no other heavy process → pre-registered deferral only if the machine is unavailable; clearing event: the developer runs the same command on the reference machine |

- `constraint-resolution: prerequisite-slice: calibration` — the harness is this slice's own steps 12 to 17. Nothing outside the slice is needed except the tactics slice, which is already a declared dependency. `wall-ownership: code-owned`.

AC-a, AC-b, AC-c, and AC-d are `observable: false` in the slice. They run as tests: AC-c in the default `cargo test --workspace`, and AC-a, AC-b, and AC-d in the slow test (step 19). The slow test reads the same report as AC-e, so one release run evidences four criteria.

## Test / Verification Plan

### Automated checks

- Format: `cargo fmt --all -- --check`.
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`.
- Unit and integration: `cargo test --workspace`. The default run includes the schema tests, the calibrate smoke test, the report builder unit tests, the fixture unit tests, and `match_stats.rs`.
- Slow criteria: `cargo test --release -p engine-cli --test calibrate -- --ignored`. It runs 2000 full matches in parallel. At the match-rules median of 390 ms per match on 8 workers, the estimate is under 3 minutes, before the tactics slice's cost is added.
- Page: `node --test web/tests/*.test.mjs` (unchanged; run as a regression gate).
- Criteria map:
  - AC-a (bands): `report.json` `calib.suites.equal` band checks, from the slow test.
  - AC-b (the stronger team wins more than 50 percent): `calib.suites.strength` win rate, from the slow test.
  - AC-c (the records validate against the schema without transformation): `crates/engine-cli/tests/schemas.rs`, plus the smoke test's validation of every calibration file.
  - AC-d (dark paths read 0): `darkpath.change_never_applied` and `darkpath.match_without_stats` in the slow test.
  - AC-e (wall time and the single-thread figure): the interactive drive below.
- Regression: `determinism.rs` and `snapshot.rs` stay byte-identical with the new counters. `crates/stream/tests/fixture.rs` stays green after the `match_event` change. The `bench` key set is unchanged (step 12).
- Pre-existing failure: `stream_cli.rs` `a_viewer_that_closes_in_mid_match_ends_the_run_without_an_error` failed in the release run of this session. Implement records whether it still fails before step 2, so that this slice is not blamed for it or credited with it.

### Interactive verification (human-in-the-loop)

- **AC-e (calibration report).** Platform `cli` from `stack.platforms`. The driver is the binary itself (`cli-direct`). No companion skill is needed.
  1. Run `cargo build --release -p engine-cli`.
  2. Set `SM_DATA_DIR` to a scratch folder.
  3. From Git Bash, run `target/release/engine-cli.exe calibrate --seed 2026 --matches 1000 > <evidence-dir>/ac-e.stdout.txt 2> <evidence-dir>/ac-e.stderr.txt`, then write `$?` to `<evidence-dir>/ac-e.exit-code`. Copy `$SM_DATA_DIR/runs/<run.id>/report.json` to `<evidence-dir>/ac-e.report.json`.
  4. Pass criteria:
     - The stdout file holds one JSON line with `record.kind` `run-report` and `operation` `calibrate`.
     - `calib.wall_ms.equal` and `calib.wall_ms.strength` are each below 1,800,000.
     - `bench.match_wall_ms` and `bench.cpu_us_per_tick` are present and non-null.
     - `build.hash` is not `unknown`.
     - The exit code is 0.
     - Report the four band values beside the times.
  5. Evidence layout per the cli adapter: `<evidence-dir>/ac-e.stdout.txt`, `ac-e.stderr.txt`, `ac-e.exit-code`, `ac-e.report.json`.

## Risks / Watchouts

- **The bands are not reachable with the tactics slice's decision model.** Tuning moves only the tuning-file knobs. The protocol has a cap of 12 iterations, and a miss becomes an open tuning item with its distance. The bands are never widened (slice Risks).
- **The plan precedes the tactics slice's plan and code.** Step 1 stops the build and calls for a plan re-review if the seams differ. The realism programme design (`docs/design/realism/01-engine-realism.md`, W-01) replaces these bands later and depends on this slice being complete. It does not change this slice's criteria.
- **The stronger team's win rate.** A win excludes draws. The report shows wins, draws, and losses, so a near miss is visible.
- **Schema drift.** The default test run validates the binary's real output with an independent validator.
- **Worker memory.** A worker holds about 60 MB of tick records for the validator (22 players, 196 bytes per record, about 300,000 ticks). With 8 workers that is about 0.5 GB, against 61.6 GB on the reference machine.
- **Disk.** Each event file is written and then pruned unless the match is an outlier. The report records `events.files_written` and `events.files_kept`, so the retention rule can be audited.
- **Benchmark drift from tuning.** Step 20 compares against the tactics slice's baseline under the per-tick gate.

## Dependencies on Other Slices

- `tactics-and-ai` (not built): the computer manager on both sides, the change queue's final state for `darkpath.change_never_applied`, and fatigue. Step 1 checks these seams.
- `data-schemas-generator` (complete): `generate_league`, `Content`, and `tuning.json` with `garde` validation.
- `match-rules` (complete): the law counts in `MatchStats`, the scenario test feature, and the snapshot that must carry the new counters.
- `stream-protocol` (complete): `EventWriter` and `MatchEvent`.
- Downstream: `integration` consumes the tuned engine; `experiment-flags` adds `--flag` to `calibrate` and reads `report.json`; `scripting-runtime` reuses the harness.

## Assumptions

Every entry is `class: implementation-detail`, settled autonomously in place of the discovery interview. None of them touches an open intent risk, a product-owner directive, control authority, the core loop, or a committed capability.

1. **Band reading.** Goals, shots, and possession are judged as means over 1000 matches. Possession is judged per side, home and away. Why: shape AC-3 says "When statistics aggregate" (`02-shape.md:127`). The share of single matches in band is reported but not judged.
2. **Strength transform.** "Attributes 15 percent higher" means every attribute of the stronger club × 1.15, rounded and clamped to 1 to 100, with the stronger club alternating home and away. Why: this is the literal reading, and alternating cancels any side effect. Clamping is rare at the shipped means of 40 to 62.
3. **Two suites per run.** `--matches` applies to each suite, and AC-e is judged per 1000-match suite. Why: each AC names 1000 matches, and one command gives one report.
4. **Worker processes.** Workers are child processes of the same binary with hidden flags, and `--jobs` defaults to the core count. Why: the slice scope says "parallel processes", and a crashed match then shows up as a missing record, which makes the contract's dark path measurable.
5. **Event files.** Every match writes an event file, and non-outlier files are deleted at run end. Why: this satisfies AC-c ("any match … one event stream file") and the product owner's retention answer (observability init Q4) together. `simulate` also writes `events.jsonl`, as `serve` already does.
6. **Run-folder layout.** The layout is `runs/<run.id>/report.json`, `stats/<match.id>.json`, and `events/<match.id>.jsonl`. Why: the contract fixes the first two (`.ai/observability.md:106`). The third keeps a calibration run self-contained.
7. **Validator crate.** `boon` 0.6, as a dev-dependency only. Why: it is pure Rust, permissively licensed (NFR-5), and needs no network. `jsonschema`'s default features need a C toolchain that this machine lacks. The release binary is unchanged.
8. **Band file location.** `content/realism-bands.json`. Why: the contract asks for the bands as data read by one source. The content folder resolution already exists and fails closed.
9. **xG.** A simple logistic function of distance and angle, with coefficients in the tuning file. Why: `stats.xg` is a contract key of match-stats. The simplest model fills it, and the realism programme may replace it.
10. **Additive record change.** The new keys are added at `schema.version` `"1"`, and `goals` is kept as an extra. Why: the previous slice set this precedent, and older `stats.json` files keep loading.
11. **Snapshot version bump.** Why: the summary block grows. A snapshot from another build is already refused, and the bump makes the refusal name the version.
12. **Augmentation artifacts not re-authored in this run.** The signals of this slice are listed in the table below, and the benchmark baseline is the tactics slice's figures. Why: the tactics slice plans and builds before this slice implements, and it re-authors both shared augmentation files. A revision written now would be replaced before this slice uses it. The index, where augmentations are recorded, is also withheld from this run. Implement Step 0.7 reads this table instead. The signals:
    - `match-stats` figures (step 6): `stats.shots`, `stats.shots_on_target`, `stats.xg`, `stats.passes`, `stats.pass_accuracy_pct`, `stats.possession_pct`, `stats.goals`, `darkpath.change_never_applied`, `manager.kind`.
    - The calibrate `run-report` (step 13).
    - `match-event` rows from headless runs (step 9).
    - `tracing::warn!` `calibrate.darkpath` with `run.id`, `counter`, and `value` (step 17).
    - `tracing::info!` `calibrate.suite` with `suite`, `matches`, `wall_ms`, and `jobs` at each suite's end (step 17).
13. **Tuning protocol.** Iterations of 200 matches, one knob group at a time, with a cap of 12. Why: this bounds the slice's top risk, and the slice's own Risks forbid widening the bands.
14. **Slow tests.** The slow tests use `#[ignore]` and are run in release. Why: the slice marks AC-a, AC-b, and AC-d slow. A default debug `cargo test` stays fast.
15. **Validator per match.** The positional validator runs on every calibration match. Why: the shape's Observation Model observes AC-5 and AC-6 "by a tick-stream validator run over every calibration match" (`02-shape.md:283`).
16. **Tactics seams assumed from the slice definition.** `MatchConfig` can put the computer manager on both sides, and the change queue exposes the changes still pending at full time. Why: the tactics slice scope names "AI manager module that manages either team" and the stoppage-gated queue. Step 1 verifies both.
17. **No consult.** The trigger `appetite-medium-or-larger` holds (appetite: large). The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`), so no consult ran (`consult-runs: []`).
18. **Autonomous answers are not written to `po-answers.md`.** Why: they are not product-owner answers. They are recorded here only, so the answer file keeps its meaning.

## Blockers

None. The tactics slice is a declared sequencing dependency (`depends-on`), not a planning blocker. Step 1 gates implementation on it.

## Freshness Research

- `boon` 0.6.1 (crates.io, via `cargo search boon` this session): JSON Schema drafts 2020-12, 2019-09, 7, 6, and 4; MIT OR Apache-2.0. The API was read from the installed crate source (`boon-0.6.1/src/compiler.rs:182-205`, `src/lib.rs:186-196`). Its dependencies (`ahash`, `appendlist`, `base64`, `fluent-uri`, `idna`, `once_cell`, `percent-encoding`, `regex`, `regex-syntax`, `serde`, `serde_json`, `url`) are pure Rust. It uses edition 2021 and builds under the workspace's edition 2024.
- `jsonschema` 0.57.0 (`cargo info jsonschema@0.57.0` this session): the default features are `resolve-http`, `resolve-file`, `tls-aws-lc-rs`, and `idna`. `aws-lc-rs` builds C code, and `00-index.md` lists `clang` and `cl` as absent. Rejected for that reason.
- `std::thread::available_parallelism` returns the logical core count and may be lower under process affinity ([std docs](https://doc.rust-lang.org/std/thread/fn.available_parallelism.html)). `--jobs` overrides it.
- `std::process::Command` with `Stdio::null()` for the worker stdout avoids a full pipe blocking a worker ([std docs](https://doc.rust-lang.org/std/process/struct.Stdio.html)).
- `std::env::current_exe` re-invokes the running binary. On Windows it returns the path the process was started from ([std docs](https://doc.rust-lang.org/std/env/fn.current_exe.html)).
- The realism figures behind the bands were settled at shape (`02-shape.md:325`: "the AC-3 bands hold; shots per team band is 8 to 16"). The realism programme's later targets (`docs/design/realism/01-engine-realism.md`, W-01) are out of scope here.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine calibration`. The plan is complete. Run it only after the tactics slice is implemented and verified, because step 1 reads that slice's seams. Compact the session first.
- **Option C:** `/wf slice football-manager-match-engine`. Use it only if the tactics slice's landed design makes these slice boundaries wrong. Nothing found in planning points to this.
