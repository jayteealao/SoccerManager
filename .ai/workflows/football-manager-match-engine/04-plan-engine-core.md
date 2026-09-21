---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: engine-core
status: complete
stage-number: 4
created-at: "2026-09-21T21:57:49Z"
updated-at: "2026-09-21T21:57:49Z"
metric-files-to-touch: 34
metric-step-count: 18
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, rust, benchmark, determinism]
stack-source: confirmed
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-engine-core.md
  shape: 02-shape.md
  instrument: 04b-instrument.md
  benchmark: 05c-benchmark.md
  observability: ../../observability.md
  siblings: []
  implement: 05-implement-engine-core.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine engine-core"
---

# Plan: Engine Core

## The Plan

Slice handed over the root slice of a 16-slice roster with one open risk above all others: whether an agent-based engine simulates 90 minutes in under 2 seconds on one thread. Three research passes found an empty repository with no commit, a cargo cache that already holds `rand`, `rand_chacha`, `serde`, `clap`, `indexmap`, `thiserror`, `anyhow`, `tracing`, and `windows-sys`, a reference machine that is a desktop AMD Ryzen 7 9800X3D on mains power with the Ultimate Performance plan, and a determinism boundary of same machine, same build, same C runtime.

Twelve product-owner answers fixed the build. The tick file is binary with 32-bit floats (about 51 MB per match), the ball carries height from the start, the simulation runs in 64-bit floats, and the command line uses clap. The product owner chose 50 decisions per second over the recommended 10, both a `bench` command and criterion benches, full-match integration tests, edition 2024 with clippy and rustfmt as blocking gates, a dual MIT OR Apache-2.0 license, `glam` for vectors, and thiserror in the library with anyhow in the binary. The benchmark report identifies the machine by a hashed hostname plus the CPU model, which reconciles the slice criterion with the observability contract. Two augmentations are authored: an instrumentation plan with 8 signals and 4 dark paths against the contract's record kinds, and a benchmark baseline that records the budget because no code exists to measure.

Implement builds 34 new files in 18 steps, the engine library first and the command line last. The top risk is the benchmark at 50 decisions per second: step 15 measures it before the criterion tests are finalized, and a miss reopens the decision rate and the tick rate with the product owner under NFR-1 yields-to C2, never a silent reduction of the positional model.

## Current State

- Repository: no source, no manifest, no `.gitignore`, no license file, no commit; branch `master` is unborn; five untracked paths (the workflow folder, `AGENTS.md`, `CLAUDE.md`, `DESIGN.md`, `PRODUCT.md`).
- Toolchain: rustc 1.92.0, cargo 1.92.0, clippy 0.1.92, rustfmt 1.8.0, git 2.49.0, host `x86_64-pc-windows-msvc`. Not installed: cargo-nextest, cargo-criterion, cargo-llvm-cov.
- Cargo cache (offline-available): rand 0.8.5, rand_chacha 0.3.1, serde 1.0.228, clap 4.5.60, indexmap 2.13.0, thiserror 2.0.18, anyhow 1.0.102, tracing, tracing-subscriber, windows-sys 0.61.2. Not cached: serde_json, glam, criterion, sha2.
- Latest stable on crates.io (2026-09-21): rand 0.10.3, rand_chacha 0.10.0, serde 1.0.229, serde_json 1.0.151, clap 4.6.7, criterion 0.8.2, glam 0.33.8, windows-sys 0.61.2. All MIT OR Apache-2.0. RUSTSEC-2026-0097 affects rand 0.10.0 only; 0.10.3 is clear.
- Reference machine: hostname hashed in reports; AMD Ryzen 7 9800X3D, 61.6 GB, no battery, Ultimate Performance plan. The shape calls it a laptop; it is a desktop.
- Network: the sandboxed shell got HTTP 403 from crates.io; cargo's own fetch is untested until the first build.
- Observability contract `.ai/observability.md` plan-version 1 exists; design brief exists without a visual contract (see Assumptions).

## Simplicity Ladder

| Capability | Rung | Choice |
|---|---|---|
| Fixed-timestep loop | rung 1 stdlib | a counted `for` loop; a headless engine needs no accumulator (Fiedler) |
| Random numbers | rung 3 reuse | `rand_chacha::ChaCha8Rng::seed_from_u64` through `rand` traits; the Rand Book names ChaCha as the reproducible generator; `StdRng` is not version-pinned |
| Vector math | rung 3 reuse | `glam` 0.33 `DVec2`/`DVec3` (PO Q11); scalar f64 types, no SIMD path |
| Deterministic iteration | rung 1 stdlib | `Vec` indexed by roster order; `BTreeMap` where a map is needed; no `HashMap` on the simulation path |
| Tick file | rung 1 stdlib | `BufWriter` plus `to_le_bytes`; no byteorder or bytemuck |
| JSON records | rung 3 reuse | `serde` + `serde_json`; float formatting by ryu, keys renamed to the contract |
| Argument parsing | rung 3 reuse | `clap` derive (PO Q4) |
| Wall time | rung 1 stdlib | `std::time::Instant` |
| CPU time, peak memory | rung 2 native platform | `GetProcessTimes`, `GetProcessMemoryInfo` via `windows-sys` (std has no CPU-time API) |
| Machine hash | rung 3 reuse | `sha2` over `COMPUTERNAME` (contract Block C) |
| Build hash | rung 1 stdlib | `build.rs` runs `git rev-parse --short HEAD`; no vergen |
| Micro-benchmarks | rung 3 reuse | `criterion` 0.8 with `harness = false` (PO Q7) |
| Steering behaviors | rung 4 new code | seek, arrive, separation from Reynolds 1999; no crate needed for three behaviors |
| Errors | rung 3 reuse | `thiserror` (library), `anyhow` (binary) (PO Q12) |
| Binary-driving tests | rung 1 stdlib | `std::process::Command` on `CARGO_BIN_EXE_engine-cli`; no assert_cmd |

## Applied Learnings

No applicable learnings found. No `runtime-evidence-deferrals` exist; the repeat-deferral tripwire does not fire.

## Likely Files / Areas to Touch

- `Cargo.toml`, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`, `.gitignore`, `rustfmt.toml`: workspace root; conventions and license.
- `crates/engine/Cargo.toml`, `crates/engine/build.rs`: library manifest; build hash.
- `crates/engine/src/lib.rs`, `error.rs`, `math.rs`, `rng.rs`, `tuning.rs`: public API, errors, vectors, RNG, constants.
- `crates/engine/src/pitch.rs`, `ball.rs`, `player.rs`, `team.rs`: the world model and the formation anchor mechanism.
- `crates/engine/src/steering.rs`, `decision.rs`, `sim.rs`: behaviors, decisions, the fixed-timestep loop.
- `crates/engine/src/record.rs`, `validate.rs`: the tick file and the validator.
- `crates/engine/src/observe/mod.rs`, `observe/process_win.rs`: records per the contract; Win32 measurements.
- `crates/engine/tests/full_match.rs`, `determinism.rs`, `validator.rs`, `common/mod.rs`; `crates/engine/benches/tick_step.rs`: the criterion tests and benches.
- `crates/engine-cli/Cargo.toml`, `src/main.rs`, `cli.rs`, `simulate.rs`, `bench.rs`; `tests/cli_args.rs`: the command line.

Full topology with line estimates: `04-plan-engine-core.yaml`.

## Proposed Change Strategy

Build the library bottom-up and prove each layer with a unit test before the next layer uses it: math and RNG, then the world model, then behaviors and decisions, then the loop, then the file and the validator, then observability, then the command line. The fixed-timestep loop replaces a variable-step loop so the tick count is a function of match time alone (named mechanism). The formation anchor gives every agent a target when it is not involved in play (named mechanism).

Determinism is a design constraint, not a test: the simulation path uses `Vec` and index order only, one `EngineRng` passed by mutable reference, and no transcendental call in a place where two builds could diverge on the same machine. The reproducibility boundary is same machine, same build, same C runtime (NFR-4).

Performance: NFR-1 governs the mechanism choices and `yields-to: C2` (PO-ratified). The product owner set decisions every tick (Q5), so the first measurement happens at step 15, before the tests are finalized. A miss is reported, not absorbed: the decision interval, a spatial grid, and fewer pass candidates are the levers, and the product owner decides.

## Step-by-Step Plan

1. **Workspace root.** Write `Cargo.toml` with members `crates/engine` and `crates/engine-cli`; `[workspace.package]` edition 2024, license `MIT OR Apache-2.0`, rust-version 1.85; `[workspace.dependencies]` glam 0.33, rand 0.10, rand_chacha 0.10, serde 1 (derive), serde_json 1, clap 4.6 (derive), thiserror 2, anyhow 1, tracing 0.1, tracing-subscriber 0.3, sha2 0.10, windows-sys 0.61 (features `Win32_System_Threading`, `Win32_System_ProcessStatus`, `Win32_Foundation`), criterion 0.8. Write `LICENSE-MIT`, `LICENSE-APACHE`, `.gitignore` (`target/`, `.scratch/`, `*.ticks`), `rustfmt.toml` (`edition = "2024"`), and a `README.md` with the build, simulate, and bench commands. Run `cargo metadata` to confirm the workspace parses.
2. **Library manifest and build hash.** Write `crates/engine/Cargo.toml` (lib; `[[bench]] name = "tick_step", harness = false`) and `build.rs` that sets `ENGINE_BUILD_HASH` from `git rev-parse --short HEAD` with `-dirty` when `git status --porcelain` is non-empty, and `unknown` when git fails. Rerun-if-changed on `.git/HEAD`.
3. **Errors and math.** Write `error.rs` (`EngineError`: `InvalidSeed`, `InvalidConfig`, `Io`, `Format`) and `math.rs` re-exporting `DVec2`, `DVec3` with `clamp_len` and `toward` helpers. Unit tests for the helpers.
4. **RNG.** Write `rng.rs`: `EngineRng` wraps `ChaCha8Rng::seed_from_u64(seed)`; expose `next_f64`, `range_usize`. Unit test: two instances with seed 42 produce the same first 1000 values.
5. **Tuning.** Write `tuning.rs`: a `Tuning` struct with `Default` holding `dt = 0.02`, `decision_interval_ticks = 1` (PO Q5), speeds and accelerations per attribute, ball drag, ground friction, gravity 9.81, restitution 0.6, ball max speed 40 m/s, arrive radius, separation radius, anchor tolerance, ball reach height 2.0 m, horizontal and vertical compactness. The data-schemas slice replaces the defaults with a file.
6. **Pitch and ball.** Write `pitch.rs` (105 m by 68 m centred at the origin, `contains`, `clamp`) and `ball.rs` (position `DVec3`, velocity `DVec3`, semi-implicit Euler, gravity when `z > 0`, bounce with restitution at `z = 0`, ground friction on the ground, air drag in flight, speed capped, `kick(direction, speed, loft)`). Unit tests: a kicked ball stops; a lofted ball lands; speed never exceeds the cap.
7. **Players and teams.** Write `player.rs` (id, team index, shirt number, position, velocity, target, the six built-in attributes 1 to 100, derived max speed and acceleration) and `team.rs` (attack direction, 4-4-2 slots, `anchor(slot, ball)` = slot offset plus ball-relative shift scaled by compactness, clamped to the pitch). Unit test: anchors stay inside the pitch for a ball at every corner.
8. **Steering.** Write `steering.rs`: `seek`, `arrive` (slow inside the radius), `separation` (inverse-distance push inside the radius) and `combine` clamped to the player's acceleration; velocity clamped to max speed. Unit tests: arrive converges without overshoot within N ticks; separation pushes two overlapping players apart.
9. **Decisions.** Write `decision.rs`: possession = nearest player within reach radius and ball height under the reach height; the carrier scores each teammate (open lane by minimum distance from a defender to the pass line, forward progress, distance penalty) and passes to the best or dribbles toward goal; defenders on the ball side chase when within a pressing distance; every other player targets its anchor. Every agent decides every tick (PO Q5); the interval constant stays in `Tuning`. All loops run by index; the RNG breaks ties. Unit test: with the ball at a striker's feet and an open teammate, a pass is chosen.
10. **Simulation loop.** Write `sim.rs`: `MatchConfig { seed, minutes, tuning }`, `Simulation::new` (validates the seed and the config), `step()` in the order decisions, steering, integrate players, integrate ball, resolve possession and kicks, then `run(ticks, &mut impl TickSink)` with `TickSink::on_tick(&TickRecord)` and a `NullSink`. Kick-off places the ball at the centre and both teams in formation; no laws yet (out of scope). Unit test: 15,000 ticks with a counting sink.
11. **Tick file.** Write `record.rs`: `TickRecord { tick: u32, ball: [f32; 3], players: [[f32; 2]; 22] }` (192 bytes); the `.ticks` layout: 64-byte header (magic `SMTK`, schema version 1, seed, dt in ms, expected ticks, float width), records, 16-byte trailer (magic `SMTE`, written count). `FileSink` writes through `BufWriter`; `read_ticks` fails closed with `EngineError::Format` on a bad magic, an unknown version, a missing trailer, or a count mismatch. Optional JSON Lines dump behind a flag with positions as arrays in roster order. Unit tests: round-trip 1000 records; a truncated file is refused.
12. **Observability records.** Write `observe/mod.rs` (`build_info()`, `machine_hash()`, `MatchStats`, `RunReport` with `#[serde(rename)]` to the contract keys, `emit_line`) and `observe/process_win.rs` (`cpu_time_ms`, `peak_memory_mb` via `GetProcessTimes` and `GetProcessMemoryInfo`, `cfg(windows)`; `None` elsewhere). Fold the signals from `04b-instrument.md` §2 into the structs. Unit test: both records serialize with every contract key present.
13. **Validator.** Write `validate.rs`: rules `in_bounds`, `separation >= 0.1 m`, `ball_speed <= 40 m/s`, `anchor_tolerance` (only when the ball is more than 30 m from the player) over a tick iterator; returns `Vec<Violation>`; emits one tracing line per violation. Unit test: a hand-built stream with one overlap returns one violation.
14. **Command line: simulate.** Write `crates/engine-cli/Cargo.toml`, `main.rs` (tracing-subscriber to stderr, `EnvFilter` from `SM_LOG`, exit 1 with one message on error), `cli.rs` (clap derive: `simulate --seed <u64> --ticks-out <path> [--minutes 90] [--json]`, `bench --seed <u64> [--matches 5] [--json]`), `simulate.rs` (run with `FileSink`, run the validator over the file, print one `match-stats` record). An invalid `--seed` value is rejected by clap with a message that names `--seed`.
15. **Command line: bench and the first measurement.** Write `bench.rs`: one warm-up run, then `matches` timed runs on the current thread with `NullSink`; wall time per run, `GetProcessTimes` delta, peak working set, ticks per second, `cpu_wall_ratio`, `machine.hash`, CPU model from `PROCESSOR_IDENTIFIER` or WMI, power plan from `powercfg /getactivescheme` through PowerShell, `build.hash`; print one `run-report` record; exit 2 when the median exceeds 2000 ms. Run `cargo run --release -p engine-cli -- bench --seed 42 --matches 5 --json` and record the number in the implement artifact. If the median exceeds 2000 ms, stop and report to the product owner with the three levers; do not lower the positional model.
16. **Integration tests.** Write `tests/common/mod.rs` (seed-42 config, temp path), `tests/full_match.rs` (AC-a: 270,000 records with 1 ball and 22 player positions), `tests/determinism.rs` (AC-b: two runs, byte-identical files), `tests/validator.rs` (AC-c, AC-d: zero violations over the seeded match; a corrupted stream returns the expected violation), and `crates/engine-cli/tests/cli_args.rs` (AC-f: `--seed abc` exits non-zero and the stderr names `--seed`; `--seed 42` exits zero).
17. **Criterion benches.** Write `benches/tick_step.rs`: one `step()` on a warmed simulation; one steering pass for 22 players. Run `cargo bench -p engine --bench tick_step` once and keep the report under `target/criterion`.
18. **Gates and first commit.** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Create the dedicated branch per the index (`branch-strategy: dedicated`, base `main`; the current unborn branch is `master`, so create `main` at the first commit and the slice branch from it) and commit the workspace. The commit message describes the engine core in product language and ends with the attribution line the session requires.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-e: bench on the reference machine, one thread, one 90-minute match under 2 s; report records machine identity and build hash | `engine-cli bench --seed 42 --matches 5 --json` (`cli-direct`, cli adapter) | Windows 11 reference machine with rustc 1.92 — yes (this machine; desktop, mains power, Ultimate Performance plan) | the release binary (step 15), the seed-42 built-in teams (steps 7 and 10), the JSON run-report on stdout (step 12), evidence capture of stdout, stderr, exit code | rerun with `--matches 10` when `bench.cpu_wall_ratio` < 0.9 → run from a fresh PowerShell with no other process → pre-registered deferral only if the machine is unavailable, clearing event: run the same command on the reference machine |

- `constraint-resolution: prerequisite-slice: engine-core` — the harness is this slice's own step 15; nothing outside the slice is needed. `wall-ownership: code-owned`.

Every other criterion is automated (`observable: false` in the slice) and runs under `cargo test --workspace`.

## Test / Verification Plan

### Automated checks

- Format: `cargo fmt --all -- --check`.
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`.
- Unit and integration: `cargo test --workspace` (about 10 seconds; full-match tests run by default per PO Q8).
- Contract check: the observability record test (step 12) asserts every Block A key the slice emits.
- Benchmark compare: `/wf verify` loads `augment/benchmark.md` in compare mode against `05c-benchmark.md` and runs the two measurement commands; the tripwires are 10 percent CPU and 25 percent memory against the budget line.

### Interactive verification (human-in-the-loop)

- **AC-e (benchmark report).** Platform `cli` from `stack.platforms`; driver: the binary itself (`cli-direct`); no companion skill needed.
  1. Run `cargo build --release -p engine-cli`.
  2. From PowerShell, run `& "target/release/engine-cli.exe" bench --seed 42 --matches 5 --json 1> "<evidence-dir>/ac-e.stdout.txt" 2> "<evidence-dir>/ac-e.stderr.txt"`, then write `$LASTEXITCODE` to `<evidence-dir>/ac-e.exit-code`. From Git Bash use `>`/`2>` and `echo $? >`.
  3. Read `ac-e.stdout.txt`: one JSON line with `record.kind` `run-report`. Pass criteria: `bench.match_wall_ms` ≤ 2000, `budget.pass` true, `machine.hash` is 12 hex characters, `machine.cpu_model` non-empty, `build.hash` non-empty and not `unknown`, `bench.cpu_wall_ratio` ≥ 0.9, exit code 0.
  4. Evidence layout per the cli adapter: `<evidence-dir>/ac-e.stdout.txt`, `ac-e.stderr.txt`, `ac-e.exit-code`.

## Risks / Watchouts

- R1 (high): the benchmark misses 2 s at 50 decisions per second. Measure at step 15; report a miss to the product owner with the levers; never lower the positional model silently (NFR-1 yields to C2).
- R2 (medium): byte-identical output breaks through `HashMap` iteration, unordered pair loops, or a transcendental call. Design rules above; the determinism test runs on every verify.
- R3 (medium): steering oscillation around anchors. Arrive radius and damping in `Tuning`; the validator test tunes them.
- R4 (low): crates.io unreachable for cargo at the first build. Fall back to cached versions (rand 0.8.5 uses `gen_range`, clap 4.5.60 is API-compatible) and record the wall.
- R5 (low): partial tick file after an abort. The reader fails closed; signal `tickfile.trailer`.
- Rand 0.10 renamed `gen` to `random` and `thread_rng` to `rng`; the code uses `random_range` and never a thread-local generator.
- Edition 2024 lint defaults: run `cargo fix --edition` is not needed on new code; address `rust-2024-compatibility` lints as they appear.

## Dependencies on Other Slices

None. This slice is the root. Consumers: `data-schemas-generator` replaces `Tuning::default()` and the built-in attribute set; `stream-protocol` wraps `TickSink`; `match-rules` adds laws on top of `sim.rs`; `calibration` runs `bench` and `simulate` in batch.

## Assumptions

- The reference machine is the desktop host used in this session, not a laptop; the shape's "reference laptop" wording is read as "reference machine".
- The visual contract `02c-craft.md` is not authored in this plan: the slice has no user-interface surface, and the contract build gate requires PRODUCT.md without a `[TODO]` marker, which the product owner chose to keep at design setup. The `viewer-pitch` plan authors the contract after a product name is written. Design context was loaded; no design reference applies to this slice.
- The `experiment` augmentation from the shape is deferred to the `experiment-flags` slice per the shape's Augmentation Plan ("arrives with the second calibration slice"); no `04c-experiment.md` is authored here.
- Consult trigger `appetite-medium-or-larger` holds; the product owner excluded `consult` at intake, so no consult ran (`consult-runs: []`).
- Records go to stdout in this slice; `SM_DATA_DIR` file layout arrives with `calibration`.

## Blockers

None.

## Freshness Research

- Source: [The Rust Rand Book — Portability](https://rust-random.github.io/book/) and [Updating to 0.10](https://rust-random.github.io/book/update-0.10.html)
  Why it matters: the seedable RNG must reproduce across runs; `StdRng` is not version-pinned and 0.10 renamed the API.
  Takeaway: seed `ChaCha8Rng` directly; use `random_range`; never a thread-local generator.
- Source: [std::collections::hash_map::RandomState](https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html)
  Why it matters: iteration order is randomized per process.
  Takeaway: no `HashMap` on the simulation path.
- Source: [f64 primitive docs](https://doc.rust-lang.org/std/primitive.f64.html)
  Why it matters: transcendental functions come from the platform C runtime and are not bit-stable across platforms.
  Takeaway: the reproducibility boundary is same machine, same build, same C runtime; document it.
- Source: [glam-rs README](https://github.com/bitshifter/glam-rs)
  Why it matters: f64 vector types are scalar, so no SIMD path affects determinism.
  Takeaway: `DVec2`/`DVec3` are safe for the engine.
- Source: [Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/)
  Why it matters: the accumulator exists to decouple rendering from physics; a headless loop needs none.
  Takeaway: a counted loop is the fixed-timestep mechanism here.
- Source: [Reynolds, Steering Behaviors for Autonomous Characters (1999)](https://www.red3d.com/cwr/papers/1999/gdc99steer.pdf)
  Why it matters: seek, arrive, and separation definitions.
  Takeaway: implement the three behaviors from the paper; `FootballEngine` (GPLv3) is read-only reference, never copied (NFR-5).
- Source: [Criterion FAQ](https://bheisler.github.io/criterion.rs/book/faq.html)
  Why it matters: `cargo bench` on stable needs `harness = false`.
  Takeaway: declare the bench target in `Cargo.toml`.
- Source: [GetProcessMemoryInfo (windows-docs-rs)](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/ProcessStatus/fn.GetProcessMemoryInfo.html)
  Why it matters: std exposes no CPU-time or peak-memory API.
  Takeaway: two Win32 calls behind a 30-line wrapper.
- Source: [RUSTSEC-2026-0097](https://rustsec.org/advisories/RUSTSEC-2026-0097.html)
  Why it matters: affects rand 0.9.0 to 0.9.2 and 0.10.0.
  Takeaway: pin rand 0.10.3 or later.
- Source: [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
  Why it matters: `[workspace.package]` and `[workspace.dependencies]` inheritance.
  Takeaway: one place for edition, license, and versions.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine engine-core` — the plan is complete; compact the session first so the SessionStart hook re-reads the artifacts.
- **Option B:** `/wf plan football-manager-match-engine data-schemas-generator` — plan the next engine slice before implementing; not recommended, because its plan depends on the crate layout and the `Tuning` struct this slice lands.
- **Option C:** `/wf slice football-manager-match-engine` — not needed; planning revealed no boundary problem.
- **Option D:** `/wf shape football-manager-match-engine` — not needed; the spec held.
