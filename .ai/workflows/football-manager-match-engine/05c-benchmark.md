---
schema: sdlc/v1
type: augmentation
augmentation-type: benchmark
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: stream-protocol
mode: baseline
language: rust
benchmark-framework: "timing-fallback (engine-cli bench) plus criterion 0.8"
targets-measured: 4
targets-failed: 0
targets-planned: 5
baseline-branch: feat/football-manager-match-engine
baseline-commit: "d58fce3"
measured-at: "2026-09-22T11:28:51Z"
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T11:28:51Z"
revision-count: 2
revisions:
  - rev: 1
    at: "2026-09-22T06:37:07Z"
    trigger: new-slice
    because: "data-schemas-generator plan re-baselines per the shape's per-engine-slice rule (PO plan Q11)"
    changed: "slice-slug, mode back to baseline, measured values replace the budget line; engine-core record kept at history/05c-benchmark-0.md"
  - rev: 2
    at: "2026-09-22T11:28:51Z"
    trigger: new-slice
    because: "stream-protocol plan re-baselines on commit d58fce3 and adds a fifth target for the socket path (PO plan Q12)"
    changed: "slice-slug, mode back to baseline, five targets, new measured values; the data-schemas-generator record with its comparison is kept at history/05c-benchmark-1.md"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-stream-protocol.md
  prior: history/05c-benchmark-1.md
---

# Benchmark: Stream Protocol and Fixture Harness (baseline)

## The Benchmark

The data-schemas-generator slice closed with four targets compared and no tripwire fired: 389 milliseconds per match, 387 milliseconds of processor time, 5.62 megabytes of peak memory, and 1.4418 microseconds per tick step. That record, with its comparison table, is byte-copied at `history/05c-benchmark-1.md`.

This baseline re-measures the same four targets on commit `d58fce3`, before the stream slice changes any code, and confirms the engine is where the last verify left it: 389 milliseconds, 384 milliseconds of processor time, 5.62 megabytes, 694,087 ticks per second, 1.4342 microseconds per tick step, and 887.68 nanoseconds per steering pass. Three drives of the first command gave 388, 389, and 390 milliseconds. The product owner chose to re-baseline and to add a fifth target (plan Round 3, Q12).

The fifth target has no baseline value, because no socket exists yet: ticks per second delivered over the socket to an unthrottled client. Verify measures it for the first time and it becomes the baseline for `viewer-pitch`. The top risk is a copy on the hot path: the encoder must write into a reused buffer, never allocate per tick, or the wall time absorbs 270,000 allocations; the tripwires against 384 milliseconds of processor time and 5.62 megabytes are the gate.

## Benchmark Targets

| Target | Type | File:line | Framework | Command |
|--------|------|-----------|-----------|---------|
| `full match wall time` | latency | `crates/engine-cli/src/bench.rs:measure` | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` |
| `ticks per second` | throughput | `crates/engine/src/sim.rs:run` | engine-cli bench | same command; field `engine.ticks_per_s` |
| `cpu time and peak memory` | cpu/memory | `crates/engine/src/observe/process.rs` | engine-cli bench | same command; fields `bench.cpu_ms`, `bench.peak_mem_mb` |
| `tick step` | cpu | `crates/engine/benches/tick_step.rs` | criterion 0.8 | `cargo bench -p engine --bench tick_step` |
| `stream throughput` (new) | throughput | `crates/stream/src/session.rs` (planned) | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json`; field `bench.stream_ticks_per_s` |

## Baseline Results

| Target | Median | P95 | P99 | Allocs/op | Bytes/op | Runs | Notes |
|--------|--------|-----|-----|-----------|----------|------|-------|
| `full match wall time` | 389 ms | N/A | N/A | N/A | N/A | 3 drives x (5 + 1 warm-up) | drives 388, 389, 390 ms; budget 2000 ms |
| `ticks per second` | 694,087 | N/A | N/A | N/A | N/A | 3 drives | 692,308 to 695,876; budget 135,000 |
| `cpu time and peak memory` | 384 ms CPU per match; 5.62 MB peak | N/A | N/A | N/A | N/A | 3 drives | tripwires +10% CPU (422 ms), +25% memory (7.03 MB) |
| `tick step` | 1.4342 µs | N/A | N/A | N/A | N/A | criterion auto | interval 1.4309 to 1.4377 µs; `steering_pass_22` 887.68 ns |
| `stream throughput` | not measured | N/A | N/A | N/A | N/A | 0 | No socket exists at baseline; verify records the first value |

Evidence: `bench-baseline/stream-protocol/bench.stdout.txt`, `bench-2.stdout.txt`, `bench-3.stdout.txt`, `criterion.stdout.txt`, exit code 0. Build hash `d58fce3-dirty`; the `-dirty` suffix comes from the hook-appended cost ledger, not from code.

One reading is discarded from the memory median by the three-drive rule: the first drive reported 6.86 MB against 5.62 MB on the other two. The two later drives agree with the 5.62 MB the last verify measured, and nothing in the binary changed between the drives, so the first reading is a first-touch working-set artefact of the process start, not a code fact.

## Measurement Commands

Exact commands to reproduce these results, in order:

```bash
# 1. full match wall time, ticks per second, cpu time and peak memory (one thread; warm-up run discarded; 5 timed runs; median reported; three drives, median of medians)
cargo build --release -p engine-cli
target/release/engine-cli.exe bench --seed 42 --matches 5 --json

# 2. tick step (criterion, harness = false)
cargo bench -p engine --bench tick_step

# 3. stream throughput (after implement; no baseline value exists)
target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json
```

Run every command from PowerShell or Git Bash with no other heavy process running, and with `SM_DATA_DIR` set to a scratch folder so the run does not touch the real runtime folder. Command 1 loads the content folder and the two default team files; the compare run must use the shipped defaults, not generated teams, so the workload matches. Command 3 runs one match with the socket server on and an in-process client that reads as fast as it can; it reports ticks delivered per second and the pause count, and it must never be run while command 1 is running.

## Targets That Could Not Be Measured

`stream throughput`: no socket server exists on commit `d58fce3`. The target is declared here so verify measures it once and `viewer-pitch` inherits the number. It is not a regression when verify records it for the first time.
