---
schema: sdlc/v1
type: augmentation
augmentation-type: benchmark
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: stream-protocol
mode: complete
language: rust
benchmark-framework: "timing-fallback (engine-cli bench) plus criterion 0.8"
targets-measured: 5
targets-failed: 0
targets-planned: 5
baseline-branch: feat/football-manager-match-engine
baseline-commit: "d58fce3"
measured-at: "2026-09-22T11:28:51Z"
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T13:35:00Z"
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

## Comparison Results

Baseline medians are the measured values on commit `d58fce3`. Compare medians are measured on commit `65a3414`, `feat/football-manager-match-engine`, 2026-09-22T13:35:00Z, one thread, Ultimate Performance plan, the shipped default clubs, `SM_DATA_DIR` on a scratch folder. Evidence: `verify-evidence/stream-protocol/bench-drive-{1,2,3}.stdout.txt`, `bench-stream-{1,2,3}.stdout.txt`, `check-criterion.stdout.txt`, `benchmark-compare.txt`.

| Target | Baseline median | Compare median | Delta | Delta% | Alloc delta% | Verdict |
|--------|----------------|---------------|-------|--------|--------------|---------|
| `full match wall time` | 389 ms | 389 ms | 0 ms | 0.0% | N/A | ✓ no change |
| `ticks per second` | 694,087 ticks/s | 694,087 ticks/s | 0 ticks/s | 0.0% | N/A | ✓ no change |
| `cpu time and peak memory` | 384 ms CPU per match; 5.62 MB peak | 381 ms CPU per match; 5.97 MB peak | -3 ms; +0.35 MB | -0.8% CPU; +6.3% memory | N/A | ✓ no change (both inside the tripwires) |
| `tick step` | 1.4342 µs | 1.4376 µs | +0.0034 µs | +0.24% | N/A | ✓ no change |
| `stream throughput` | not measured | 640,213 ticks/s | first value | N/A | N/A | ✓ recorded |

Three drives of command 1 without a reset gave wall times of 389, 390, and 389 milliseconds, processor time of 390.6, 378.0, and 381.2 milliseconds per match (the reported 381 is the median drive), peak memory of 6.04, 5.97, and 5.97 MB, and processor-to-wall ratios of 1.002, 0.968, and 0.978; no drive was contended. Command 2 reported `tick_step` at 1.4339 to 1.4415 µs, which overlaps the baseline interval of 1.4309 to 1.4377 µs, and `steering_pass_22` at 888.96 ns against 887.68 (+0.14 percent).

Three drives of command 3 gave 640,213, 634,857, and 642,476 ticks per second delivered, with 0, 1, and 1 pause. The socket costs 0.8 percent of engine throughput: `engine.ticks_per_s` falls from 694,087 without the socket to 688,775 with it, on the same seed and the same content hash `02d33ad91de5`. Delivered throughput is 4.7 times the 135,000 budget.

Peak memory on command 3 is 7.31 MB. It is not compared to the 7.03 MB tripwire, because that tripwire belongs to command 1. Command 3 is a different workload: it adds a socket, a producer buffer of 500 ticks, a client thread, and the tungstenite write buffer. 7.31 MB is the first value of that workload and `viewer-pitch` inherits it.

**Regression summary:** none.

**Tripwires:** none fired. Processor time per match is 381 ms against the 422 ms tripwire; peak memory is 5.97 MB against the 7.03 MB tripwire.

**The top risk did not land.** The plan warned that a copy on the hot path would absorb 270,000 allocations. The encoder writes into a reused 99-byte array inside `TickFrame`, and a full streamed match costs 1.69 MB of extra peak memory against the same match without a socket, which is the 500-frame buffer, the client, and the write buffer, not a per-tick allocation.

The verify-time fix commit `c7f7357` changes two test assertions and the handling of a client that goes away. Neither runs on the timed path; the compare numbers stand for it.
