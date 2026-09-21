---
schema: sdlc/v1
type: augmentation
augmentation-type: benchmark
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: engine-core
mode: baseline
language: rust
benchmark-framework: "timing-fallback (engine-cli bench) plus criterion 0.8"
targets-measured: 0
targets-failed: 4
baseline-branch: master
baseline-commit: "none"
measured-at: "2026-09-21T21:57:49Z"
created-at: "2026-09-21T21:57:49Z"
updated-at: "2026-09-21T21:57:49Z"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-engine-core.md
---

# Benchmark: Engine Core (baseline)

## The Benchmark

The shape set the performance budget before any code existed: one 90-minute match in under 2 seconds on one thread, 1000 matches in under 30 minutes, and tripwires of 10 percent CPU and 25 percent memory regression between slices. The repository has no code and no commit, so no measurement can run at baseline.

The baseline therefore records the budget as the comparison line and the reference machine as it is today: hostname hashed per the contract, an AMD Ryzen 7 9800X3D desktop with 61.6 GB of memory, no battery, and the Ultimate Performance power plan. Four targets are named; all four wait for the code. The product owner chose both a `bench` command in the binary and criterion benches for the tick step, so compare mode runs two commands.

Verify runs compare mode after implement, and the first measured numbers land against the budget. The top risk is the decision rate the product owner set at 50 per second; if the median match wall time exceeds 2000 ms, the tripwire reads as a budget miss and the decision rate and the tick rate reopen with the product owner under NFR-1 yields-to C2.

## Benchmark Targets

| Target | Type | File:line | Framework | Command |
|--------|------|-----------|-----------|---------|
| `full match wall time` | latency | `crates/engine-cli/src/bench.rs` (planned) | engine-cli bench | `cargo run --release -p engine-cli -- bench --seed 42 --matches 5 --json` |
| `ticks per second` | throughput | `crates/engine/src/sim.rs:run` (planned) | engine-cli bench | same command; field `engine.ticks_per_s` |
| `cpu time and peak memory` | cpu/memory | `crates/engine/src/observe/process_win.rs` (planned) | engine-cli bench | same command; fields `bench.cpu_ms`, `bench.peak_mem_mb` |
| `tick step` | cpu | `crates/engine/benches/tick_step.rs` (planned) | criterion 0.8 | `cargo bench -p engine --bench tick_step` |

## Baseline Results

| Target | Median | P95 | P99 | Allocs/op | Bytes/op | Runs | Notes |
|--------|--------|-----|-----|-----------|----------|------|-------|
| `full match wall time` | N/A | N/A | N/A | N/A | N/A | 0 | budget 2000 ms |
| `ticks per second` | N/A | N/A | N/A | N/A | N/A | 0 | budget 135,000 ticks/s (270,000 in 2 s) |
| `cpu time and peak memory` | N/A | N/A | N/A | N/A | N/A | 0 | tripwires 10 percent CPU, 25 percent memory |
| `tick step` | N/A | N/A | N/A | N/A | N/A | 0 | budget 7.4 µs per tick (2 s / 270,000) |

## Measurement Commands

Exact commands to reproduce these results, in order:

```bash
# 1. full match wall time, ticks per second, cpu time and peak memory (one thread; warm-up run discarded; 5 timed runs; median reported)
cargo build --release -p engine-cli
target/release/engine-cli.exe bench --seed 42 --matches 5 --json

# 2. tick step (criterion, harness = false)
cargo bench -p engine --bench tick_step
```

Run both from PowerShell or Git Bash with no other heavy process running; the report records `bench.cpu_wall_ratio`, and a value below 0.9 marks the run as contended.

## Targets That Could Not Be Measured

| Target | Reason | Manual measurement approach |
|--------|--------|-----------------------------|
| `full match wall time` | no code exists at baseline | run command 1 after implement |
| `ticks per second` | no code exists at baseline | run command 1 after implement |
| `cpu time and peak memory` | no code exists at baseline | run command 1 after implement |
| `tick step` | no code exists at baseline | run command 2 after implement |
