---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: engine-core
status: complete
stage-number: 5
created-at: "2026-09-21T22:35:04Z"
updated-at: "2026-09-21T22:35:04Z"
metric-files-changed: 36
metric-lines-added: 4022
metric-lines-removed: 0
metric-deviations-from-plan: 8
metric-review-fixes-applied: 0
commit-sha: "3dd2a82e7fa99eb4465eb5b1df1a445cc1523f12"
tags: [engine, rust, benchmark, determinism]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-engine-core.md
  plan: 04-plan-engine-core.md
  siblings: []
  verify: 06-verify-engine-core.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine engine-core"
---

# Implement: Engine Core

## The Implementation

Plan handed over an empty repository, an 18-step build order, and one risk above all others: whether 22 steering agents with ball physics and a decision every tick simulate 90 minutes in under 2 seconds on one thread. The plan fixed the crate layout, the binary tick file, `ChaCha8Rng`, `glam`, `clap`, edition 2024 with clippy and rustfmt as blocking gates, and the dual license.

The build wrote 36 files in two crates and 4,022 lines, in the plan's order. The first full run exposed three defects the plan could not foresee, and each one changed a rule. A loose kick-off ball went to the home team every time by index tie-break and left the ball idle for 12 percent of the match; the conceding team's centre-forward now restarts with the ball in hand, and idle ticks fell from 32,362 to 767. No goalkeeper stopped a shot, so seed 42 ended 267 to 28; the keeper now stands on the line from the goal centre toward the ball and catches within 2.6 metres, which brought six seeds to between 0 and 2 goals. The carried ball snapped to the carrier's feet at a computed 65 metres per second; it now follows at 0.4 metres per tick. The validator's anchor rule flagged every legitimate loose-ball chaser, so the rule now names idle drift: far from the ball, far from the anchor, and closing on neither for 100 ticks. Every rule change kept the plan's contract; none lowered the positional model.

The benchmark retired the top risk with room to spare: the median 90-minute match takes 398 milliseconds against the 2,000 millisecond budget, at 678,000 ticks per second, with a CPU-to-wall ratio of 0.94 and a peak working set of 5 megabytes. Six seeds produce zero validator violations and about 780 possession changes each. Verify next runs the gates and captures the benchmark evidence. The top open risk is realism: 0 to 2 goals per match is plausible but untuned, and the `calibration` slice owns the realism bands.

## Summary of Changes

- Cargo workspace with `crates/engine` (library) and `crates/engine-cli` (binary), edition 2024, dual MIT OR Apache-2.0 license, rustfmt and clippy clean.
- Fixed-timestep loop at 50 ticks per second: decisions, steering, ball integration, possession, overlap resolution, in that order every tick.
- Pitch 105 by 68 metres, ball with height, gravity, bounce, ground friction, and air drag; speed capped at 40 metres per second.
- 22 players in 4-4-2 with seek, arrive, and separation steering; the arrive curve uses the braking distance `v²/(2a)`.
- Possession decision every tick: pass scoring by lane, forward progress, receiver space, and distance; dribble; shot; two pressers; goalkeeper positioning, catch, and clearance.
- Seedable `ChaCha8Rng` behind `EngineRng`; no `HashMap` on the simulation path; every loop runs in index order.
- Binary `.ticks` file: 64-byte header, 192-byte records, 16-byte trailer; the reader fails closed; optional JSON Lines dump.
- Validator with four rules: in bounds, 0.1 metre separation, 40 metres per second ball speed, anchor idle drift.
- Observability records `match-stats` and `run-report` with the contract's dotted keys, hashed machine identity, and build hash.
- Command line `simulate` and `bench`; Win32 CPU time and peak working set; exit code 2 when the median misses the budget.
- 26 unit tests, 4 engine integration tests over the full 270,000-tick match, 2 command-line tests, 2 criterion benches.

## Files Changed

- `Cargo.toml`: workspace members, shared package fields, shared dependency versions, release profile with debug info.
- `Cargo.lock`: resolved dependency graph; committed so the build hash and the benchmark bind to one set of versions.
- `.gitignore`: `target/`, `.scratch/`, `*.ticks`.
- `rustfmt.toml`: edition 2024 formatting.
- `LICENSE-MIT`, `LICENSE-APACHE`: the dual license.
- `README.md`: build, simulate, bench, test, and license sections in product language.
- `crates/engine/Cargo.toml`: library manifest, criterion bench target, windows-sys behind `cfg(windows)`.
- `crates/engine/build.rs`: embeds `git rev-parse --short HEAD` with a `-dirty` suffix as `ENGINE_BUILD_HASH`.
- `crates/engine/src/lib.rs`: module tree, re-exports, `TICKS_PER_SECOND`, `ticks_for_minutes`, `build_hash`, `version`.
- `crates/engine/src/error.rs`: `EngineError` with `InvalidConfig`, `Io`, `Format`.
- `crates/engine/src/math.rs`: `DVec2`, `DVec3` re-exports; `clamp_len`, `toward`, `segment_distance`.
- `crates/engine/src/rng.rs`: `EngineRng` over `ChaCha8Rng::seed_from_u64`; `next_f64`, `range_f64`, `range_usize`, `chance`.
- `crates/engine/src/tuning.rs`: `Tuning` defaults for physics, steering, decisions, keeper, and validator thresholds.
- `crates/engine/src/pitch.rs`: dimensions, `contains`, `clamp`, `goal_centre`, `in_goal`.
- `crates/engine/src/ball.rs`: `Ball` with `kick` and `integrate`.
- `crates/engine/src/player.rs`: `Attributes`, `Player`, derived max speed and acceleration.
- `crates/engine/src/team.rs`: 4-4-2 formation, `slot_base`, `anchor` with the keeper's ball-line position, `players`.
- `crates/engine/src/steering.rs`: `arrive`, `seek`, `separation`, `next_velocity`, `step_all`, `resolve_overlaps`.
- `crates/engine/src/decision.rs`: `decide` (targets, pressers, chasers) and `decide_carrier` (shot, pass, keeper clearance, dribble).
- `crates/engine/src/sim.rs`: `MatchConfig`, `Simulation`, `Summary`, the tick order, kick-off, goals, possession, tackles.
- `crates/engine/src/record.rs`: `TickRecord`, `TickSink`, `NullSink`, `VecSink`, `FileSink`, `read_ticks`, `write_jsonl`.
- `crates/engine/src/validate.rs`: `Validator`, `Violation`, the four rules.
- `crates/engine/src/observe/mod.rs`: `MatchStats`, `RunReport`, envelope keys, `machine_hash`, `match_id`, `emit_line`.
- `crates/engine/src/observe/process.rs`: `cpu_time_ms`, `peak_memory_mb` through `GetProcessTimes` and `K32GetProcessMemoryInfo`.
- `crates/engine/tests/common/mod.rs`: seed-42 configuration and temp paths.
- `crates/engine/tests/full_match.rs`: AC-a, 270,000 records with one ball and 22 player positions.
- `crates/engine/tests/determinism.rs`: AC-b, two runs byte-identical.
- `crates/engine/tests/validator.rs`: AC-c zero violations over the seeded match; AC-d one violation from a corrupted stream.
- `crates/engine/benches/tick_step.rs`: criterion benches `tick_step` and `steering_pass_22`.
- `crates/engine-cli/Cargo.toml`: binary manifest.
- `crates/engine-cli/src/main.rs`: tracing to stderr with `SM_LOG`, dispatch, exit codes.
- `crates/engine-cli/src/cli.rs`: clap derive for `simulate` and `bench`.
- `crates/engine-cli/src/simulate.rs`: run, write, validate, and print one `match-stats` record.
- `crates/engine-cli/src/bench.rs`: warm-up, timed runs, median, CPU and memory, machine fields, one `run-report` record.
- `crates/engine-cli/tests/cli_args.rs`: AC-f, `--seed abc` rejected with a message that names `--seed`; a valid seed exits 0.

## Shared Files (also touched by sibling slices)

- None. This slice is the root; no sibling implementation exists.

## Notes on Design Choices

- The conceding team's centre-forward restarts with the ball. A loose ball at the centre went to the lower index every time and idled for 12 percent of the match.
- The goalkeeper is the only player who may gain a ball faster than `control_speed`, with `keeper_catch_chance` 0.7 and `keeper_reach` 2.6 metres.
- A goalkeeper in possession never dribbles: it passes to the best option or clears long at 28 metres per second with 6 metres per second of loft.
- The carried ball moves at most `carry_step` 0.4 metres per tick toward the carry point. A snap on gain read as 65 metres per second in the validator.
- The `arrive` behaviour slows inside the braking distance `v²/(2a)` or the arrive radius, whichever is larger. A fixed 3 metre radius overshot by 1.45 metres at 7 metres per second.
- The anchor rule counts a player as drifting only when the player is more than 30 metres from the ball, more than 15 metres from its anchor, and closing on neither. A player chasing a loose ball 35 metres away is not drifting.
- Every random draw goes through one `EngineRng` by mutable reference; loops run by index; no `HashMap` exists on the simulation path.
- A goal requires the ball under the 2.44 metre crossbar; a ball over the bar bounces off the goal line like a wall until the laws arrive.
- The engine records run on stdout; `SM_DATA_DIR` file layout arrives with `calibration`, as the plan assumed.

## Verification Seams Built

- AC-e (bench on the reference machine, one thread, one 90-minute match under 2 seconds; the report records machine identity and build hash) → the release binary with the `bench` subcommand at `crates/engine-cli/src/bench.rs:12` (budget) and `crates/engine-cli/src/bench.rs:58` (exit code 2 on a miss); the seed-42 built-in teams at `crates/engine/src/team.rs:15` and `crates/engine/src/sim.rs:63`; the JSON `run-report` on stdout at `crates/engine/src/observe/mod.rs:146` with `machine.hash` at `crates/engine/src/observe/mod.rs:35` and `build.hash` at `crates/engine/src/lib.rs:37` (enables `cli-direct`: run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` and capture stdout, stderr, and the exit code).
- AC-a, AC-b, AC-c, AC-d, AC-f (automated) → the seed-42 fixture at `crates/engine/tests/common/mod.rs:9`; `VecSink` at `crates/engine/src/record.rs:87` for in-memory validation; `read_ticks` at `crates/engine/src/record.rs:175` for byte comparison; `CARGO_BIN_EXE_engine-cli` in `crates/engine-cli/tests/cli_args.rs` (enables `cargo test --workspace`).
- Measurement recorded at step 15 (`bench --seed 42 --matches 5 --json`, release build after the bootstrap commit): `bench.match_wall_ms` 398, `engine.ticks_per_s` 678,392, `bench.cpu_ms` 1,875, `bench.cpu_wall_ratio` 0.942, `bench.peak_mem_mb` 5.05, `budget.pass` true, exit code 0, `machine.hash` `74ca12fc08a4`, `machine.cpu_model` "AMD Ryzen 7 9800X3D 8-Core Processor", `machine.power_plan` "Ultimate Performance", `build.hash` `7b0681c-dirty`. Criterion: `tick_step` 1.467 µs, `steering_pass_22` 877 ns.

## Deviations from Plan

- Step 1: `rust-version` is 1.86, not 1.85. `cargo fetch` reported that criterion 0.8.2 requires rustc 1.86.
- Step 3: `EngineError` has no `InvalidSeed` variant. Every `u64` is a valid seed; there is nothing to reject.
- Step 5: restitution is 0.55; seven tuning fields the plan did not name were added: `keeper_reach`, `keeper_depth`, `keeper_catch_chance`, `carry_step`, `crossbar_height`, `shot_noise`, and `anchor_grace_ticks`.
- Step 8: `arrive` takes the acceleration and uses the braking distance; there is no `combine` function, `next_velocity` clamps the change directly.
- Step 9 and step 10: the plan named no goalkeeper model. Without one, seed 42 ended 267 to 28. The keeper anchor, catch, and clearance were added; kick-off gives the ball to the conceding team's centre-forward instead of leaving it loose.
- Step 12: the process module is `observe/process.rs` with `cfg(windows)` inside, not `process_win.rs`; `build_info()` is split into `build_hash()` and `version()` in `lib.rs`.
- Step 13: the anchor rule is idle drift (far from the ball, far from the anchor, closing on neither for 100 ticks). The plan's rule, applied to the seed-42 match, produced 1,988 violations, all on loose-ball chasers moving toward the ball.
- Step 18: the commit SHA is written into this record by a second small commit on the slice branch, because a SHA cannot be embedded in the commit it names.

## Anything Deferred

- Laws of the game: `sdlc-debt:` at `crates/engine/src/sim.rs:187`. Touchlines and goal lines bounce the ball like walls. Ceiling: no throw-ins, corners, goal kicks, or offside. Upgrade path: the `match-rules` slice.
- Restart flag in the tick record: `sdlc-debt:` at `crates/engine/src/validate.rs:77`. A ball jump above 2 metres per tick is read as a kick-off. Ceiling: a teleport bug that jumps more than 2 metres passes as a restart. Upgrade path: the `stream-protocol` slice adds an explicit restart event.
- Realism tuning: goals per match, pass completion, and possession share are untuned constants in `Tuning`. Ceiling: plausible, not calibrated. Upgrade path: the `calibration` slice and the realism-bands contract.
- Spatial queries: brute force over 22 agents and 231 pairs per tick, as the plan pre-filled. Ceiling: fine at 22 agents. Upgrade path: a grid if a later slice adds agents.
- `#![allow(dead_code)]` in `crates/engine/tests/common/mod.rs:1`: each integration test binary compiles the shared module and uses a subset of it. No `sdlc-debt:` ceiling; the allowance is the standard pattern for shared test modules.

## Known Risks / Caveats

- The wall bounce at the goal line is live in shipped code: a ball over the crossbar rebounds into play instead of leaving it. Marked at `crates/engine/src/sim.rs:187`.
- The restart heuristic in the validator is live: see `crates/engine/src/validate.rs:77`.
- Determinism holds on the same machine, the same build, and the same C runtime. A different platform may differ in transcendental results.
- The build hash reads `unknown` when the binary is built outside a git checkout, and `<sha>-dirty` with uncommitted changes.

## Freshness Research

- Source: installed `rand-0.10.3/src/rng.rs` (cargo registry)
  Why it matters: the plan named `Rng::random_range`; the compiler reported the method missing.
  Takeaway: in rand 0.10 the methods `random` and `random_range` live on the `RngExt` trait; `use rand::RngExt`.
- Source: installed `windows-sys-0.61.2` `Win32/System/Threading` and `Win32/System/ProcessStatus`
  Why it matters: exact signatures for `GetProcessTimes`, `K32GetProcessMemoryInfo`, `PROCESS_MEMORY_COUNTERS`.
  Takeaway: `K32GetProcessMemoryInfo` is the exported name; the struct needs every field initialised and `cb` set.
- Source: installed `criterion-0.8.2`
  Why it matters: the bench target and the minimum compiler.
  Takeaway: criterion 0.8.2 requires rustc 1.86; `criterion::black_box` is superseded by `std::hint::black_box`.
- Source: [Reynolds, Steering Behaviors for Autonomous Characters (1999)](https://www.red3d.com/cwr/papers/1999/gdc99steer.pdf)
  Why it matters: arrive is defined by a slowing distance, not a fixed radius.
  Takeaway: the braking distance `v²/(2a)` removes the overshoot the fixed radius produced.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine engine-core` — the slice has testable behaviour and one interactive criterion (AC-e). Consider compacting the session before `/wf verify`; workflow state lives in artifact files on disk and the SessionStart hook re-reads it after compaction.
- **Option B:** `/wf review football-manager-match-engine engine-core` — not recommended; the slice is behaviour, not declaration, and verify captures the benchmark evidence review needs.
- **Option C:** `/wf plan football-manager-match-engine engine-core` — not needed; the eight deviations are recorded and none changed the crate layout or the contract.
- **Option D:** Blocked — not applicable; every gate passes.
