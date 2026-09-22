---
schema: sdlc/v1
type: augmentation
augmentation-type: benchmark
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: data-schemas-generator
mode: complete
compare-branch: feat/football-manager-match-engine
compare-commit: "4a806c8"
compared-at: "2026-09-22T10:01:47Z"
regressions-found: 0
improvements-found: 0
language: rust
benchmark-framework: "timing-fallback (engine-cli bench) plus criterion 0.8"
targets-measured: 4
targets-failed: 0
baseline-branch: feat/football-manager-match-engine
baseline-commit: "27d21f5"
measured-at: "2026-09-22T06:37:07Z"
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T10:01:47Z"
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-22T06:37:07Z"
    trigger: new-slice
    because: "data-schemas-generator plan re-baselines per the shape's per-engine-slice rule (PO plan Q11)"
    changed: "slice-slug, mode back to baseline, measured values replace the budget line; engine-core record kept at history/05c-benchmark-0.md"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-data-schemas-generator.md
  prior: history/05c-benchmark-0.md
---

# Benchmark: Data Schemas and Team Generator (baseline)

## The Benchmark

Engine-core closed with four targets measured under budget on commit `27d21f5`, and the shape asks for a benchmark artifact per engine slice with tripwires of 10 percent CPU and 25 percent memory between slices. The engine-core record, with its budget-line baseline and its first measured values, is byte-copied at `history/05c-benchmark-0.md`.

This baseline re-measures the same two commands on the same commit before the data-schemas-generator slice changes any code, so the compare run after implement lands against real numbers instead of the budget line: a 90-minute match in 393 milliseconds, 687,023 ticks per second, 387 milliseconds of CPU per match, a 5.06 megabyte peak working set, 1.4611 microseconds per tick step, and 880.97 nanoseconds per steering pass. The product owner chose to re-baseline rather than keep the engine-core file (plan Round 3, Q11).

Verify ran compare mode on commit `4a806c8`, after implement replaced the uniform-60 built-in teams with loaded team files whose attributes differ per player. Every target stayed inside the no-change band: 389 ms wall against 393, 694,087 ticks per second against 687,023, 387 ms CPU per match against 387, 1.4418 µs per tick step against 1.4611. Peak memory rose from 5.06 to 5.62 MB (+11.1 percent), which is the parsed content and the two team files held for the run, inside the 25 percent tripwire. The loader runs once before the warm-up match, so no file read sits on the timed path. The next engine slice binds its tripwires against 387 ms CPU and 5.62 MB.

## Benchmark Targets

| Target | Type | File:line | Framework | Command |
|--------|------|-----------|-----------|---------|
| `full match wall time` | latency | `crates/engine-cli/src/bench.rs:measure` | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` |
| `ticks per second` | throughput | `crates/engine/src/sim.rs:run` | engine-cli bench | same command; field `engine.ticks_per_s` |
| `cpu time and peak memory` | cpu/memory | `crates/engine/src/observe/process.rs` | engine-cli bench | same command; fields `bench.cpu_ms`, `bench.peak_mem_mb` |
| `tick step` | cpu | `crates/engine/benches/tick_step.rs` | criterion 0.8 | `cargo bench -p engine --bench tick_step` |

## Baseline Results

| Target | Median | P95 | P99 | Allocs/op | Bytes/op | Runs | Notes |
|--------|--------|-----|-----|-----------|----------|------|-------|
| `full match wall time` | 393 ms | N/A | N/A | N/A | N/A | 5 (+1 warm-up) | `bench.cpu_wall_ratio` 0.985; budget 2000 ms |
| `ticks per second` | 687,023 | N/A | N/A | N/A | N/A | 5 | budget 135,000 |
| `cpu time and peak memory` | 387 ms CPU per match; 5.06 MB peak | N/A | N/A | N/A | N/A | 5 | `bench.cpu_ms` 1937 over 5 matches; tripwires +10% CPU (426 ms), +25% memory (6.33 MB) |
| `tick step` | 1.4611 µs | N/A | N/A | N/A | N/A | criterion auto | interval 1.4525 to 1.4728 µs; `steering_pass_22` 880.97 ns |

Evidence: `bench-baseline/data-schemas-generator/bench.stdout.txt`, `criterion.stdout.txt`, both exit codes 0. Build hash `27d21f5-dirty`; the `-dirty` suffix comes from the hook-appended cost ledger, not from code.

## Measurement Commands

Exact commands to reproduce these results, in order:

```bash
# 1. full match wall time, ticks per second, cpu time and peak memory (one thread; warm-up run discarded; 5 timed runs; median reported)
cargo build --release -p engine-cli
target/release/engine-cli.exe bench --seed 42 --matches 5 --json

# 2. tick step (criterion, harness = false)
cargo bench -p engine --bench tick_step
```

Run both from PowerShell or Git Bash with no other heavy process running. After the slice lands, command 1 loads the content folder and the two default team files; the compare run must use the shipped defaults, not generated teams, so the workload matches.

## Targets That Could Not Be Measured

None. All four targets measured.

## Comparison Results

Baseline medians are the measured values on commit `27d21f5`. Compare medians are measured on commit `4a806c8`, `feat/football-manager-match-engine`, 2026-09-22T10:01:47Z, one thread, Ultimate Performance plan, the shipped default clubs. Evidence: `verify-evidence/data-schemas-generator/bench-drive-1.stdout.txt`, `bench-drive-2.stdout.txt`, `bench-drive-3.stdout.txt`, `check-criterion.stdout.txt`.

| Target | Baseline median | Compare median | Delta | Delta% | Alloc delta% | Verdict |
|--------|----------------|---------------|-------|--------|--------------|---------|
| `full match wall time` | 393 ms | 389 ms | -4 ms | -1.0% | N/A | ✓ no change |
| `ticks per second` | 687,023 ticks/s | 694,087 ticks/s | +7,064 ticks/s | +1.0% | N/A | ✓ no change |
| `cpu time and peak memory` | 387 ms CPU per match; 5.06 MB peak | 387 ms CPU per match; 5.62 MB peak | 0 ms; +0.56 MB | 0.0% CPU; +11.1% memory | N/A | ✓ no change (memory inside the 25% tripwire) |
| `tick step` | 1.4611 µs | 1.4418 µs | -0.0193 µs | -1.3% | N/A | ✓ no change |

Three drives of command 1 without a reset gave medians of 389, 389, and 389 milliseconds, CPU of 372, 387, and 391 milliseconds per match (the reported 387 is the median drive), peak memory of 5.68, 5.62, and 5.62 MB, and CPU-to-wall ratios of 0.952, 0.980, and 0.995; no drive was contended. Command 2 reported `tick_step` at 1.4373 to 1.4464 µs and `steering_pass_22` at 883.43 ns (baseline 880.97 ns, +0.3 percent).

**Regression summary:** none.

**Tripwires:** none fired. CPU per match is unchanged at 387 ms against the 426 ms tripwire; peak memory is 5.62 MB against the 6.33 MB tripwire. The verify-time fix commit `ae31329` changes the content folder resolver only, which runs once before the warm-up match and is outside the timed path; the compare numbers stand for it.
