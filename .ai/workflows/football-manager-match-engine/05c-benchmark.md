---
schema: sdlc/v1
type: augmentation
augmentation-type: benchmark
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: match-rules
mode: baseline
language: "rust"
benchmark-framework: "timing-fallback (engine-cli bench) plus criterion 0.8"
targets-measured: 6
targets-failed: 0
baseline-branch: feat/football-manager-match-engine
baseline-commit: "d86996a"
measured-at: "2026-09-22T19:11:21Z"
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T19:29:33Z"
revision-count: 4
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
  - rev: 3
    at: "2026-09-22T14:41:32Z"
    trigger: new-slice
    because: "viewer-pitch plan re-baselines the five engine targets on 3570eb9 and adds four browser targets; the product owner chose option 2 alongside option 3 at plan Round 3 Q12"
    changed: "slice-slug, language and framework now name two sides, nine targets, re-measured engine values and new tripwires, four declared browser targets with budgets and no baseline value; the stream-protocol record with its comparison is kept at history/05c-benchmark-2.md"
  - rev: 4
    at: "2026-09-22T19:29:33Z"
    trigger: new-slice
    because: "match-rules plan re-baselines the engine on d86996a and adds processor time per tick as the gate (PO plan Round 4 Q13)"
    changed: "slice-slug, mode back to baseline, six engine targets with the per-tick target as the gate, page targets carried unmeasured to viewer-match-day; the viewer-pitch record with its comparison is kept at history/05c-benchmark-3.md"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-match-rules.md
  prior: history/05c-benchmark-3.md
---

# Benchmark: Match Rules and Stoppage Snapshots (baseline)

## The Benchmark

The viewer slice closed with no tripwire fired: 386 milliseconds per match, 384.4 milliseconds of processor time, 5.33 megabytes of peak memory, and 1.4434 microseconds per tick step. That record, with its comparison table and its four page targets, is byte-copied at `history/05c-benchmark-3.md`.

This baseline re-measures the engine on commit `d86996a`, before `match-rules` changes any code, and the engine has not moved: 388 milliseconds wall, 384.4 milliseconds of processor time, 5.36 megabytes peak, 695,876 ticks per second, 1.4336 microseconds per tick step, and 639,935 ticks per second over the socket with no pause. Three drives gave 390, 388, and 388 milliseconds. One target is new and it is the gate: processor time per tick, 1.4237 microseconds at baseline. The product owner chose it (plan Round 4 Q13) because added time makes a match about 9 percent longer, so a per-match gate would fire from match length alone.

Compare mode reads `bench.cpu_us_per_tick` from the new run report against 1.566 microseconds, and peak memory against 6.69 megabytes. Per-match wall time and ticks per match are reported beside the gate, not judged by it; the 2-second per-match budget still holds. The top risk is the referee's per-tick work: exit detection runs every tick, and the offside set runs at every pass, on a tick that costs 1.42 microseconds today.

## Benchmark Targets

| Target | Type | File:line | Framework | Command |
|--------|------|-----------|-----------|---------|
| `processor time per tick` (new, the gate) | cpu | `crates/engine-cli/src/bench.rs` (field planned in step 19) | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 5 --json`; field `bench.cpu_us_per_tick` (baseline derived from `bench.cpu_ms` / matches / 270,000) |
| `full match wall time` | latency | `crates/engine-cli/src/bench.rs:measure` | engine-cli bench | same command; fields `bench.match_wall_ms` and `bench.ticks_per_match` (planned) |
| `ticks per second` | throughput | `crates/engine/src/sim.rs:run` | engine-cli bench | same command; field `engine.ticks_per_s` |
| `cpu time and peak memory` | cpu/memory | `crates/engine/src/observe/process.rs` | engine-cli bench | same command; fields `bench.cpu_ms`, `bench.peak_mem_mb` |
| `tick step` | cpu | `crates/engine/benches/tick_step.rs` | criterion 0.8 | `cargo bench -p engine --bench tick_step` |
| `stream throughput` | throughput | `crates/stream/src/session.rs` | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json`; field `bench.stream_ticks_per_s` |

## Baseline Results

| Target | Median | P95 | P99 | Allocs/op | Bytes/op | Runs | Notes |
|--------|--------|-----|-----|-----------|----------|------|-------|
| `processor time per tick` | 1.4237 µs | N/A | N/A | N/A | N/A | 3 drives | 384.4 ms / 270,000 ticks; tripwire 1.566 µs (+10%) |
| `full match wall time` | 388 ms | N/A | N/A | N/A | N/A | 3 drives x (5 + 1 warm-up) | drives 390, 388, 388 ms; 270,000 ticks per match; budget 2000 ms |
| `ticks per second` | 695,876 | N/A | N/A | N/A | N/A | 3 drives | 692,308 to 695,876 |
| `cpu time and peak memory` | 384.4 ms CPU per match; 5.36 MB peak | N/A | N/A | N/A | N/A | 3 drives | per-match CPU 387.6, 384.4, 384.4 ms; peak 5.355, 5.328, 5.363 MB; memory tripwire 6.69 MB (+25%) |
| `tick step` | 1.4336 µs | N/A | N/A | N/A | N/A | criterion auto | interval 1.4298 to 1.4379 µs; `steering_pass_22` 872.40 ns |
| `stream throughput` | 639,935 ticks/s delivered | N/A | N/A | N/A | N/A | 1 drive | 0 pauses; peak memory 6.65 MB on this workload; `engine.ticks_per_s` 699,482 with the socket on; memory tripwire 8.31 MB |

Evidence: `bench-baseline/match-rules/bench-1.stdout.txt`, `bench-2.stdout.txt`, `bench-3.stdout.txt`, `bench-stream-1.stdout.txt`, `criterion.stdout.txt`, each with its `.stderr.txt` and `.exit-code` (all 0), plus `commit.txt` and `measured-at.txt`. Build hash `d86996a-dirty`; the `-dirty` suffix comes from the hook-appended cost ledger and the untracked workflow files, not from code. Content hash `02d33ad91de5`, unchanged. Machine hash `74ca12fc08a4`, AMD Ryzen 7 9800X3D, Ultimate Performance plan.

The three drives agree: processor-to-wall ratios 0.994, 0.993, and 0.987, and peak memory within 35 kilobytes. No reading is discarded.

## Measurement Commands

Exact commands to reproduce these results, in order:

```bash
# 1. processor time per tick (the gate), full match wall time, ticks per second, cpu time and peak memory
#    (one thread; warm-up run discarded; 5 timed matches per drive; median reported; three drives, median of medians)
cargo build --release -p engine-cli
target/release/engine-cli.exe bench --seed 42 --matches 5 --json

# 2. tick step (criterion, harness = false)
cargo bench -p engine --bench tick_step

# 3. stream throughput (socket on, in-process client reading as fast as it can)
target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json
```

Run every command from Git Bash or PowerShell with no other heavy process running, and with `SM_DATA_DIR` set to a scratch folder so the run does not touch the real runtime folder. Command 1 loads the content folder and the two shipped default team files. Command 3 must never run while command 1 is running. Compare mode reads `bench.cpu_us_per_tick` from the run report; on the baseline commit that field does not exist, so the baseline value is derived as `bench.cpu_ms / bench.matches / 270000`, in microseconds.

## Targets That Could Not Be Measured

None of the six engine targets failed. The four page targets from the viewer slice (`viewer frame rate`, `viewer frame time p95`, `viewer decode time`, `viewer history bytes`) are not in this baseline: this slice changes no drawing, decoding, or history code, and `viewer-match-day` takes their readings from `history/05c-benchmark-3.md` as its baseline. The page's history reservation grows from 25.4 MB to 33.8 MB because the announced maximum rises to 360,000 ticks; that is recorded in the plan and stays far under the 300 MB budget.

## Compare rules for this baseline

- The gate is `bench.cpu_us_per_tick` against 1.566 µs (+10 percent) and `bench.peak_mem_mb` against 6.69 MB (+25 percent). A reading over either limit is a tripwire.
- `bench.match_wall_ms`, `bench.cpu_ms` per match, and `bench.ticks_per_match` are reported beside the gate with their deltas and are not judged by the 10 percent tripwire (plan Round 4 Q13). The 2000 ms per-match budget still applies.
- The stream workload keeps its own memory tripwire of 8.31 MB.
- **Update the sibling `05c-benchmark.yaml` in the same pass:** set `mode: compare`, fill `compare_commit`, move each baseline value to `before`, and write the new reading to `after`.
