---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: calibration
status: complete
stage-number: 5
created-at: "2026-09-23T00:51:38Z"
updated-at: "2026-09-23T00:51:38Z"
metric-files-changed: 36
metric-lines-added: 3360
metric-lines-removed: 94
metric-deviations-from-plan: 14
metric-review-fixes-applied: 0
commit-sha: "c00eaaca13c581f015b966044762db19561e7353"
commits:
  - "80f976d85d6d3e3132327e52e29c15d71439eb58"
  - "c00eaaca13c581f015b966044762db19561e7353"
steering-honored:
  - "No visual change: no file under web/ changed, so the design direction in steer.md (tokens, layout, no-spinner and colour rules) is untouched."
  - "Output boundary: both commit messages, every code comment, README.md, and content/README.md use product language; a scan of the added lines and the messages for workflow vocabulary found none. No debt marker naming the workflow tool was placed in code; shortcuts are recorded here instead."
tags: [engine, calibration, statistics, observability, schemas, tuning]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-calibration.md
  plan: 04-plan-calibration.md
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-commentary.md]
  verify: 06-verify-calibration.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine calibration"
---

# Implement: Calibration Harness and Statistics Record

## The Implementation

The engine that this slice inherited counted shots and nothing else a realism band needs: no passes, no possession, no shots on target, no expected goals. `simulate` wrote no event file, the three schema files the observability contract names did not exist, and no command played many matches. The tactics slice had already added the computer manager on both sides and the change-queue state at full time, so the plan's step-1 seams held without a plan re-review. The pre-existing stream test failure named in the plan did not reproduce: the release baseline ran 251 passed, 0 failed, 3 ignored.

Four decisions carry the work. First, the counters sit at the two points where the engine already resolves play: a kick counts a pass or a shot and scores the shot's expected goals, and the next controlling touch completes a pass or makes a shot on target. Possession is credited per open-play tick to the team that touched the ball last. Second, `engine-cli calibrate` runs two suites (equal strength, and one club with every attribute times 1.15) in worker processes of the same binary, and every match writes its own statistics record and event file. A crashed worker therefore shows up as a missing record, which is the contract's second dark path. Third, the three record schemas are validated in tests against the binary's real output by `boon`, a validator used in tests only. Fourth, the tuning pass took 6 knob iterations out of the 12 allowed, plus one AI change: `shot_noise` 0.25, `keeper_catch_chance` 0.84, and `decision.skill` 0.8, and the AI manager now reviews the score on each goal's stoppage. Over 1000 matches per suite on seed 2026, goals per match are 2.831, shots per team 12.675, possession 49.997/50.003, and the stronger club wins 0.65. Each suite runs in 78 seconds on 8 workers, against a 30-minute budget. The single-thread figure is 432 ms per match at 1.5464 µs per tick.

The dark path for changes that never applied reads 288 over 2000 matches, so AC-d fails and the run exits 2. Every one of the 288 changes was queued after the last stoppage of its match, and 183 of them are refused injury substitutions that the AI manager queues again every 30 seconds. The counter was left as the contract defines it. Whether a change that expires at full time counts as "never applied" is a product-owner question, and it is recorded as a residual. Integration can consume the tuned engine now. The top open risk is the goals distribution: the mean is in band, but the standard deviation is 3.36 and one match ended with 28 goals.

## Summary of Changes

- The engine counts shots on target, expected goals, passes, completed passes, and open-play possession ticks. The counters are in the snapshot (layout version 3).
- Expected goals: a logistic function of the distance to the goal centre and of the angle the goal mouth subtends. The coefficients are in the new `xg` block of `tuning.json`.
- `match-stats` gains `stats.goals`, `stats.shots_on_target`, `stats.xg`, `stats.passes`, `stats.pass_accuracy_pct`, `stats.possession_pct`, `manager.kind`, and the extra `passes.completed`, plus `error.type` on a failed match. All of them are home-first arrays, at `schema.version` "1".
- `simulate` writes `events.jsonl` beside `stats.json`, and each row carries its commentary line.
- `engine-cli calibrate`: generated 20-club leagues, a double round-robin, per-match seeds, a strength suite, worker processes, a run folder (`report.json`, `stats/`, `events/`), outlier-only event retention, band checks, dark-path counters, a single-thread benchmark figure, and exit code 0 or 2.
- `content/realism-bands.json`: the accepted bands as data, loaded fail-closed.
- `schemas/observability/`: `match-event`, `match-stats`, and `run-report` in JSON Schema 2020-12.
- Tuning pass: three values moved, and the AI manager now checks the score on the stoppage of every goal.

## Files Changed

- `Cargo.toml`, `Cargo.lock`: `boon` 0.6.1 in the workspace dependencies. Only `engine-cli`'s dev-dependencies use it.
- `crates/engine-cli/Cargo.toml`: `serde` and `garde` for the bands file, and `boon` for tests.
- `crates/engine/src/sim.rs`: new `Summary` counters, `pass_in_flight` and `shot_in_flight`, `shot_xg`, possession ticks, and pass and shot resolution in `gain`.
- `crates/engine/src/rules/mod.rs`: a goal counts a shot on target and makes both AI checks due. Every dead ball and kick-off clears the in-flight markers.
- `crates/engine/src/tuning.rs`: `XgTuning` with `garde` ranges, and the new defaults.
- `crates/engine/src/snapshot.rs`: the summary encodes the new counters, and `VERSION` is 3.
- `crates/engine/src/observe/mod.rs`: `MatchFigures`, `round_to`, `write_stats_at`, and `write_record_at`.
- `crates/engine/src/scenario.rs`: `Scene::shoot` for scripted shots in tests.
- `crates/engine/tests/match_stats.rs` (new): counter and xG tests.
- `crates/engine/tests/identity.rs`, `crates/engine/tests/snapshot.rs`: the literal gains `figures`, and the version text reads 3.
- `crates/stream/src/events.rs`: `EventWriter::create_at(path)`. `open` delegates to it.
- `crates/engine-cli/src/stream_run.rs`: `Ids::new` and `match_event` are shared crate-wide.
- `crates/engine-cli/src/simulate.rs`, `resume.rs`: `MatchFigures` in the record. `simulate` writes the events file with commentary.
- `crates/engine-cli/src/bench.rs`: `measure` extracted, and `cpu_model` shared. The `bench` output key set is unchanged.
- `crates/engine-cli/src/cli.rs`, `main.rs`: the `calibrate` command and its hidden worker flags.
- `crates/engine-cli/src/calibrate/{mod,fixtures,worker}.rs` (new): the parent, the fixture list, and the worker.
- `crates/engine-cli/src/report/{mod,bands}.rs` (new): `RunBuilder`, `CalibrationReport`, and the bands loader.
- `crates/engine-cli/tests/{schemas,calibrate}.rs`, `tests/common/mod.rs` (new): the schema criterion, the smoke and retention runs, and the slow criteria.
- `crates/engine-cli/tests/cli_args.rs`: the help-width test covers `calibrate --help`.
- `crates/engine-cli/tests/stream_cli.rs`: the seed-7 one-minute fixture now holds one throw-in (3007 frames).
- `content/tuning.json`, `content/README.md`, `content/realism-bands.json`, `README.md`, `schemas/observability/*.schema.json`: data, docs, and schemas.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs`, `rules/mod.rs`, `snapshot.rs`: engine-core, match-rules, and tactics-and-ai all edit these. The snapshot is version 3, so a version-2 file is refused by name.
- `crates/engine/src/observe/mod.rs`: `MatchStats` gained a third flattened block. Older `stats.json` files still load because every new field defaults.
- `crates/engine-cli/src/stream_run.rs`: owned by the stream and commentary slices. The wire output did not change, and `crates/stream/tests/fixture.rs` passed.
- `content/tuning.json`: the tactics slice's decision block. Only `skill` changed.

## Notes on Design Choices

- The in-flight markers are cleared at every dead ball, like `last_kicker`, so the snapshot never needs them.
- The bands judge suite means, per plan Assumption 1. The share of single matches in the possession band is reported (`possession_in_band_share`) and not judged.
- The worker's match identifier is `MatchId { seed: match_seed, millis: run_millis }`, so the parent knows every expected record name and counts a missing one exactly.
- Workers log at `warn` unless `SM_LOG` is set, and their stdout is discarded. The parent passes the resolved content folder explicitly.
- `outcome` in the run report is `failure` only when a worker process fails. A band miss is a failed check, not a failed run.

## Verification Seams Built

- AC-a (goals, shots, possession bands over 1000 equal matches) → `report.json` `calib.bands` from `RunBuilder::checks` at `crates/engine-cli/src/report/mod.rs:243`, read by `a_thousand_matches_hold_the_realism_bands` at `crates/engine-cli/tests/calibrate.rs:145` (enables `cargo test --release -p engine-cli --test calibrate -- --ignored`).
- AC-b (the stronger club wins more than half) → the strength suite, with the boost at `crates/engine-cli/src/calibrate/fixtures.rs:88` and the side alternating by fixture. `stronger_team_win_rate` is checked at `report/mod.rs:243` (the same slow test).
- AC-c (one statistics record and one event file per match validate without transformation) → `RecordSchemas` at `crates/engine-cli/tests/common/mod.rs:30`, compiled from `schemas/observability/`. The tests are `crates/engine-cli/tests/schemas.rs:11`, `:54`, `:82`, and `calibrate.rs:33`, which validate every calibration file (enables the default `cargo test --workspace`).
- AC-d (dark paths read zero) → `darkpath.change_never_applied` summed by `RunBuilder::change_never_applied`, and `darkpath.match_without_stats` counted in the parent at `crates/engine-cli/src/calibrate/mod.rs:28` (the read loop). The `calibrate.darkpath` warning is emitted in the same function (enables the slow test and the report).
- AC-e (wall time and the single-thread figure) → `calib.wall_ms` per suite and `bench.*` from `measure` at `crates/engine-cli/src/bench.rs:90`, called after the workers by `calibrate/mod.rs:28`, and the stdout line from `emit_line` (enables the cli-direct drive; evidence in `implement-evidence/calibration/ac-e.*`).
- Counters → `Scene::shoot` at `crates/engine/src/scenario.rs:121`, `shot_xg` at `crates/engine/src/sim.rs:334`, and tests at `crates/engine/tests/match_stats.rs:15`, `:41`, `:54`, `:78`.

## Visual Contract Honored (only if `02c-craft.md` was present)

Not applicable. `02c-craft.md` exists for the viewer, but this slice changes no page, component, or style.

## Deviations from Plan

1. **Step 1: shots were already counted.** The tactics slice added `Summary.shots` and `TacticsStats` with `stats.shots` and `darkpath.change_never_applied`. `MatchFigures` leaves those two keys out, because a duplicate flattened key breaks serde. The plan's key list is met across the two blocks.
2. **`shot_xg` lives in `sim.rs:334`, not `tuning.rs`.** `XgTuning` is in `tuning.rs:150` as planned. The function needs the pitch geometry, and `sim.rs` already imports it.
3. **`CalibrationReport` lives in `crates/engine-cli/src/report/mod.rs:358`, not `observe/mod.rs`.** It implements the engine's public `Record` trait. The suite and band types belong to the harness, so the engine crate carries no harness vocabulary.
4. **`write_record_at` is added (`observe/mod.rs:386`).** Both `write_stats` and `write_stats_at` call it, and the calibration report is written through it too.
5. **The AI manager checks on every goal's stoppage (`rules/mod.rs:136`).** The plan's tuning protocol names only tuning-file knobs. This change cut dark-path hits from 141 to 52 at 200 matches per suite, with the bands unchanged in direction. It reuses the AI's existing `due` flag, as the injury path already does.
6. **The tuning log is `implement-evidence/calibration/tuning-log.txt`, not `.md`.** The workflow's file guard refuses a Markdown file there without an `NN-` name and frontmatter.
7. **`rules.pack_version` is typed as an integer in `match-stats.schema.json`.** The contract's Block A lists it as a string. The engine has written an integer since the laws slice, and AC-c requires validation "without transformation", so the schema follows the emitted type.
8. **`Scene::shoot` is added** to the test scene builder. The plan's counter test needs a scripted shot, and the scene could only pass.
9. **Commentary lines are added to the saved event rows** (`simulate.rs`, `worker.rs`; commit `c00eaac`). The commentary slice's integration note asked the slice that makes `match_event` shared to thread the commentator through it.
10. **A hidden `--run-millis` flag** carries the run start to the workers, so that their match identifiers equal the ones the parent expects. The plan listed four hidden flags.
11. **The bands file has a `lo`/`hi` shape and adds `wall_minutes_per_sample`.** The plan listed the values only. The budget of a smaller run scales with its match count.
12. **Tests beyond the plan:** retention (`calibrate.rs:106`), the benchmark report against the run-report schema (`schemas.rs:82`), and `calibrate --help` in the help-width test.
13. **Pinned values updated:** the snapshot version refusal text (2 to 3, `snapshot.rs` unit test and `tests/snapshot.rs`), and the record fixture's frame count (3006 to 3007: tuning adds a throw-in to the seed-7 minute). `content.rs` and `full_match.rs` needed no change: the pinned-default test passes, and the sanity band holds.
14. **The slow test ran on `80f976d`, and the AC-e evidence run on `c00eaac`.** The commentary commit writes lines only and does not change play. Both runs reported the same figures: 2.831 goals, 12.675 shots, 0.65 win rate, and 288 changes never applied.

## Anything Deferred

- **Feature flags for competing models**: out of scope, owned by the later flags work (slice Scope, Out).
- **The pipeline itself** (dashboard, collector): out of scope. The records and schemas are its input.
- **The AI re-queue of refused injury substitutions** (183 of the 288 dark-path hits): found here and not fixed, because it is the tactics slice's designed behaviour. It was handed to a separate task for follow-up.

## Known Risks / Caveats

- **AC-d fails: `darkpath.change_never_applied` = 288 over 2000 matches** (`implement-evidence/calibration/darkpath-breakdown.txt`). All 288 changes were queued after the last stoppage of their match: `sub-injury` 183, `sub-fatigue` 76, `mentality-down-leading` 26, `mentality-up-trailing` 3. In the hit matches, the median time from the last stoppage to full time is 116 s, and the longest is 1809 s. All 183 injury substitutions follow an earlier rejection of the same team's substitution (142 no window left, 41 limit reached). The run therefore exits 2 even though every band passes.
- **Heavy goals tail.** The equal suite's goals have a standard deviation of 3.364 (mean 2.831), and single matches reach 28 goals. The worst blow-out in the tuning runs was repeated goals by one striker straight after the kick-off. The bands judge means only.
- **Mentality effect.** The strength suite averages 22.4 shots per team, and matches where one side trails and attacks late produce 30-plus shots. The mentality offsets in `tactics.json` were not tuned, because the plan's knob groups name `tuning.json` only.
- **Almost every match is an outlier.** 885 of 1000 equal-suite matches are outliers, mostly because a single team's shots fall outside 8 to 16. So 1799 of 2000 event files are kept, and outlier-only retention prunes little.
- **Expected goals are not calibrated.** The mean xG is 1.70 per match against 2.83 goals, because the coefficients are the plan's start values.
- **Shortcuts with a ceiling (no in-code debt marker, per the output boundary):** the event writer flushes every row (about 300 per match; this costs little next to the 400 ms simulation). Workers hold every tick record for the validator (about 60 MB each, 8 workers). Aggregation keeps every record in memory (at most 2000 records).

## Freshness Research

- `boon` 0.6.1: API read from the installed crate at `~/.cargo/registry/src/*/boon-0.6.1/src/compiler.rs:182-205` (`add_resource`, `compile`) and `src/lib.rs:186-196` (`Schemas::validate`). `cargo fetch` pulled its pure-Rust dependencies. No C toolchain was needed.
- `std::process::Command` with `Stdio::null()` stdout for workers, and `std::env::current_exe()`: as planned. Both behaved on this Windows machine: 8 workers, 2000 matches, 0 missing records.
- `garde` 0.23 `custom(fn(self.field))`: the same pattern as `crates/engine/src/data/tuning.rs:58`.

## Evidence

All under `implement-evidence/calibration/`:
- `baseline-tests.txt`: release workspace tests before any change. 251 passed, 0 failed, 3 ignored. The stream test did not fail.
- `tuning-log.txt`: every tuning iteration, with its band values.
- `tests-after-tuning.txt`: the release run after tuning showed 3 pinned-value failures, which were then fixed (Deviation 13).
- `tests-default.txt`: `cargo test --workspace`. 271 passed, 0 failed, 4 ignored, exit 0.
- `engine-slow-tests.txt`: the engine's slow criteria after tuning, all 3 pass (trailing AI, mentality shots, stronger team).
- `slow-calibrate.stdout.txt`, `.stderr.txt`, `.exit-code` (101): the 2000-match slow test. Every band assertion passed, and it failed at `darkpath.change_never_applied` (left 288, right 0).
- `ac-e.stdout.txt`, `ac-e.stderr.txt`, `ac-e.exit-code` (2), `ac-e.report.json`: the cli-direct run on `c00eaac`. Wall time 78,053 ms (equal) and 78,760 ms (strength), `bench.match_wall_ms` 432, `bench.cpu_us_per_tick` 1.5464, `build.hash` `c00eaac-dirty`, one JSON line on stdout.
- `darkpath-breakdown.txt`: the analysis of the 288 changes.
- `bench-1..3.*`: three drives of `bench --seed 42 --matches 5 --json` gave 418.8, 412.4, and 415.8 ms of processor time per match (median 415.8 against the 460.7 ms gate) and 6.332, 6.320, and 6.352 MB peak (median 6.332 against 6.82 MB). No tripwire fired.
- `bench-keys-before.txt`, `bench-keys-after.txt`: identical `bench` key sets (step 12).
- `clippy.txt` (exit 0) and `node-tests.txt` (47 pass). `cargo fmt --all -- --check` exited 0.

## Assumptions and Decisions

Every entry is `class: implementation-detail`. None resolves an open or carried intent risk. The one intent-bearing question found is recorded under Residuals and was not settled.

1. **Seams from step 1 accepted without a plan re-review** (`class: implementation-detail`). `MatchConfig::new` defaults both managers to `Manager::Ai` (`sim.rs:99`), and `pending_changes()` exposes the queue at full time. Both match plan Assumption 16.
2. **Reuse the tactics slice's shot counter and dark-path key** (`class: implementation-detail`). See Deviation 1.
3. **Shots on target = goal or opposing keeper takes control while the shot is in flight** (`class: implementation-detail`). This is the plan's definition. A keeper who fails the catch roll does not count.
4. **Every `Kick::Pass` counts as a pass, restarts included** (`class: implementation-detail`). `apply_kick` does not tell open play from a restart kick. A pass ends at the next controlling touch or at a dead ball.
5. **The AI manager reacts on the goal's stoppage** (`class: implementation-detail`). It reuses the committed `due` mechanism, and what the AI decides is unchanged. It changes only when the AI notices a new score.
6. **Tuning values** (`class: implementation-detail`): `shot_noise` 0.25, `keeper_catch_chance` 0.84, and `decision.skill` 0.8, all within their `garde` ranges. `realism-bands.json` is untouched.
7. **Schema typing follows the emitted record** (`class: implementation-detail`). See Deviation 7.
8. **The report lives in the harness crate** (`class: implementation-detail`). See Deviation 3.
9. **Commentary in the saved event rows** (`class: implementation-detail`): honours the commentary slice's integration note (Deviation 9).
10. **No suppression attributes** (`class: implementation-detail`). The worker command takes `(shard, shards)` as one argument instead of `#[allow(clippy::too_many_arguments)]`.
11. **The AI re-queue loop is handed off, not fixed here** (`class: implementation-detail`). It is the tactics slice's designed behaviour, and fixing it would not bring AC-d to zero on its own.

## Residuals

- **AC-d (dark paths read zero): FAILING.** `darkpath.change_never_applied` = 288, and `darkpath.match_without_stats` = 0. This is a code reason and a definition question:
  - Code reason: the AI manager queues a refused injury substitution again every 30 seconds (183 of 288). It was handed off to a follow-up task.
  - Open product-owner question (intent-bearing, not settled here): does a change queued after the last stoppage of a match, which the queue could never apply, count as "a change that never applied" for the zero-tolerance dark path? The remaining 105 are of this kind. Redefining the counter would change a product-owner-locked contract, and the run leaves it unchanged.
- **Open tuning items (bands pass; not failures):** the goals tail (sd 3.36), the untuned mentality offsets in `tactics.json`, and uncalibrated xG coefficients.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine calibration`. The slice touches testable behaviour, and verify judges AC-d against the residual above. Consider compacting the session before `/wf verify`: workflow state lives in the artifact files on disk, and the SessionStart hook re-reads it after compaction.
- **Option C:** `/wf plan football-manager-match-engine calibration`, if the product owner answers the dark-path question by changing the counter's definition, or asks for the AI re-queue fix and mentality tuning in this slice.
- **Option D (blocked on the product owner, for AC-d only):** the dark-path definition question above.
