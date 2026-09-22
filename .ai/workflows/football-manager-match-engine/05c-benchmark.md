---
schema: sdlc/v1
type: augmentation
augmentation-type: benchmark
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: tactics-and-ai
mode: baseline
language: "rust"
benchmark-framework: "timing-fallback (engine-cli bench) plus criterion 0.8"
targets-measured: 6
targets-failed: 0
baseline-branch: feat/football-manager-match-engine
baseline-commit: "837cb5c"
measured-at: "2026-09-22T22:13:37Z"
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T22:18:43Z"
revision-count: 5
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
  - rev: 5
    at: "2026-09-22T22:18:43Z"
    trigger: new-slice
    because: "tactics-and-ai plan re-baselines the engine on 837cb5c; its criterion names processor time per match, so the gate returns to per match with the per-tick figure reported beside it"
    changed: "slice-slug, mode back to baseline, six engine targets re-measured, gate per match (460.7 ms) and memory (6.82 MB); the match-rules record with its comparison is kept at history/05c-benchmark-4.md"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-tactics-and-ai.md
  prior: history/05c-benchmark-4.md
---

# Benchmark: Tactics, Fatigue, and the AI Manager (baseline)

## The Benchmark

The match-rules slice closed with no tripwire fired: 1.5080 microseconds per tick against a 1.566 limit, and 5.45 MB peak memory. That record and its comparison table are byte-copied at `history/05c-benchmark-4.md`.

This baseline re-measures the engine on `837cb5c`, before the slice changes any code. The code on `837cb5c` equals the verified match-rules state `a4893c8`: `git diff --stat a4893c8 837cb5c` over `crates`, `web`, `content`, `docs`, and `README.md` is empty. Three drives gave 418.8, 418.8, and 415.8 milliseconds of processor time per match, and 1.4965, 1.4965, and 1.4858 microseconds per tick. Wall time was 421, 421, and 419 ms over 279,850 ticks per match. Peak memory was 5.469, 5.453, and 5.445 MB.

This slice's criterion names processor time per match, so the gate is per match: 460.7 ms (+10 percent) and 6.82 MB (+25 percent). The product owner's per-tick reading (plan Round 4 Q13) was scoped to the match-rules criterion. Processor time per tick and ticks per match are reported beside the gate. If the per-match gate fails only because the match is longer, the product owner decides. The top risk is the decision layer: every tick the carrier scores more options on a tick that already costs about 1.5 microseconds.

## Benchmark Targets

| Target | Type | File:line | Framework | Command |
|--------|------|-----------|-----------|---------|
| `processor time per match` (the gate) | cpu | `crates/engine-cli/src/bench.rs` (`bench.cpu_ms`, `bench.matches`) | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 5 --json`; `bench.cpu_ms / bench.matches` |
| `processor time per tick` | cpu | `crates/engine-cli/src/bench.rs` | engine-cli bench | same command; field `bench.cpu_us_per_tick` |
| `full match wall time` | latency | `crates/engine-cli/src/bench.rs:run_one` | engine-cli bench | same command; fields `bench.match_wall_ms` and `bench.ticks_per_match` |
| `ticks per second` | throughput | `crates/engine/src/sim.rs:run` | engine-cli bench | same command; field `engine.ticks_per_s` |
| `peak memory` | memory | `crates/engine/src/observe/process.rs` | engine-cli bench | same command; field `bench.peak_mem_mb` |
| `tick step` | cpu | `crates/engine/benches/tick_step.rs` | criterion 0.8 | `cargo bench -p engine --bench tick_step` |
| `stream throughput` | throughput | `crates/stream/src/session.rs` | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json`; field `bench.stream_ticks_per_s` |

## Baseline Results

| Target | Median | P95 | P99 | Allocs/op | Bytes/op | Runs | Notes |
|--------|--------|-----|-----|-----------|----------|------|-------|
| `processor time per match` | 418.8 ms | N/A | N/A | N/A | N/A | 3 drives x 5 matches | drives 418.8, 418.8, 415.8 ms; tripwire 460.7 ms (+10%) |
| `peak memory` | 5.45 MB | N/A | N/A | N/A | N/A | 3 drives | drives 5.469, 5.453, 5.445 MB; tripwire 6.82 MB (+25%) |
| `processor time per tick` | 1.4965 µs | N/A | N/A | N/A | N/A | 3 drives | drives 1.4965, 1.4965, 1.4858 µs; reported, +10% would be 1.646 µs |
| `full match wall time` | 421 ms | N/A | N/A | N/A | N/A | 3 drives | drives 421, 421, 419 ms; 279,850 ticks per match; budget 2000 ms |
| `ticks per second` | 664,727 | N/A | N/A | N/A | N/A | 3 drives | 664,727 to 667,900 |
| `tick step` | 1.5264 µs | N/A | N/A | N/A | N/A | criterion auto | interval 1.5118 to 1.5457 µs; `steering_pass_22` 1.0271 µs (interval 998.86 ns to 1.0582 µs) |
| `stream throughput` | 609,220 ticks/s delivered | N/A | N/A | N/A | N/A | 1 drive | 0 pauses; peak memory 6.84 MB on this workload; stream memory tripwire 8.55 MB |

Evidence: `bench-baseline/tactics-and-ai/bench-1.stdout.txt`, `bench-2.stdout.txt`, `bench-3.stdout.txt`, `bench-stream-1.stdout.txt`, and `criterion.stdout.txt`, each with its `.stderr.txt` and `.exit-code` (all 0), plus `build.stderr.txt`, `commit.txt` (`837cb5c`), and `measured-at.txt`. The build hash is `837cb5c-dirty`. The `-dirty` suffix comes from untracked workflow files and the staged documents of other work, not from engine code. The content hash is `30e098265c77`. The machine hash is `74ca12fc08a4` (AMD Ryzen 7 9800X3D, Ultimate Performance plan). The processor-to-wall ratios are 0.993, 0.994, and 0.991. No reading is discarded.

Criterion reported `steering_pass_22` 23 percent slower than its saved state, although no steering code changed since the verify reading of 875.22 ns on `a4893c8`. The whole-tick figure from the same run showed no change (+0.4 percent, p = 0.24). The steering reading is recorded as machine noise and is not a gate.

## Measurement Commands

Exact commands to reproduce these results, in order:

```bash
# 1. processor time per match (the gate), per tick, wall time, ticks per second, peak memory
#    (one thread; warm-up run discarded; 5 timed matches per drive; three drives, median of drives)
cargo build --release -p engine-cli
target/release/engine-cli.exe bench --seed 42 --matches 5 --json

# 2. tick step (criterion, harness = false)
cargo bench -p engine --bench tick_step

# 3. stream throughput (socket on, in-process client reading as fast as it can)
target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json
```

Run every command from Git Bash or PowerShell with no other heavy process running, and with `SM_DATA_DIR` set to a scratch folder. Command 3 must never run while command 1 is running.

## Targets That Could Not Be Measured

None of the engine targets failed. The four page targets from the viewer slice are not re-measured, because this slice changes no drawing, decoding, or history code. `viewer-match-day` still takes their readings from `history/05c-benchmark-3.md`.

## Compare rules for this baseline

- The gate is `bench.cpu_ms / bench.matches` against 460.7 ms (+10 percent) and `bench.peak_mem_mb` against 6.82 MB (+25 percent), each the median of three drives. A reading over either limit is a tripwire.
- `bench.cpu_us_per_tick` (1.4965 µs at baseline) and `bench.ticks_per_match` (279,850) are reported beside the gate with their deltas. If the per-match gate fires while the per-tick figure is within 10 percent, the product owner decides. The comparison does not pass that case by itself.
- The 2000 ms per-match budget still applies (NFR-1). The stream workload keeps its own memory tripwire of 8.55 MB.
- **Update the sibling `05c-benchmark.yaml` in the same pass:** set `mode: compare`, fill `compare_commit`, move each baseline value from `after` to `before`, and write the new reading to `after`.
