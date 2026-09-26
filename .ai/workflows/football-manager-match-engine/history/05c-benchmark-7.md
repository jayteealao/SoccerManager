---
schema: sdlc/v1
type: augmentation
augmentation-type: benchmark
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: lone-forward
mode: baseline
language: "rust"
benchmark-framework: "timing-fallback (engine-cli bench) plus criterion 0.8"
targets-measured: 7
targets-failed: 0
baseline-branch: feat/football-manager-match-engine
baseline-commit: "a447ff1"
measured-at: "2026-09-24T11:01:30Z"
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-24T11:05:00Z"
revision-count: 7
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
  - rev: 6
    at: "2026-09-24T02:42:52Z"
    trigger: new-slice
    because: "defending-and-discipline plan re-baselines the engine on 2089ed7; the slice names a +10 percent processor-time-per-tick tripwire for the new marking search"
    changed: "slice-slug, mode back to baseline, gate moves to processor time per tick, seven targets re-measured with new tripwires; the tactics-and-ai record with its comparison is kept at history/05c-benchmark-5.md"
  - rev: 7
    at: "2026-09-24T11:05:00Z"
    trigger: new-slice
    because: "lone-forward plan re-baselines the engine on a447ff1; the slice changes the carrier's per-tick decision and the tackle draw"
    changed: "slice-slug, baseline commit and seven targets re-measured; gate stays processor time per tick (+10 percent) and peak memory (+25 percent); the defending-and-discipline record is kept at history/05c-benchmark-6.md"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-lone-forward.md
  prior: history/05c-benchmark-6.md
---

# Benchmark: Lone Forward (baseline)

## The Benchmark

The defending-and-discipline comparison closed with no tripwire fired: 1.5312 µs of processor time per tick against a 1.6228 µs limit, and 6.797 MB peak memory (`05-implement-defending-and-discipline.md`, Checks Run). That baseline record is byte-copied at `history/05c-benchmark-6.md`. Three commits landed after it: `f7fe35b` (defending and discipline), `a6dc5d2` (the public send-off before kick-off, which runs once at kick-off and not per tick) and `a447ff1` (a server log line). So this plan measures again before the slice changes code.

This baseline measures the engine on `a447ff1`. `git status` shows no change under `crates`, `content`, `web`, `Cargo.toml` or `Cargo.lock`. Three drives gave 1.5319, 1.5422 and 1.5422 microseconds of processor time per tick, and 447.0, 450.0 and 450.0 milliseconds per match. Wall time was 449, 449 and 450 ms over 291,800 ticks per match. Peak memory was 6.836, 6.656 and 6.660 MB.

The slice changes two things that run on every tick with the ball in play: the carrier's option scores (a scan of his team-mates for one ahead of him) and the tackle draw (a tuned win chance in place of a constant). The slice names no tripwire of its own, so the gate stays per tick at the default rule: 1.6964 µs (+10 percent) and 8.33 MB peak memory (+25 percent). Processor time per match (495.0 ms at +10 percent) and ticks per match are reported beside the gate. New decision weights and tackle odds change play, so a match may run longer or shorter; a per-match change that match length alone explains is not a regression. The top risk is the lone-forward check in the option scores, which the plan keeps to one pass over the carrier's team-mates.

## Benchmark Targets

| Target | Type | File:line | Framework | Command |
|--------|------|-----------|-----------|---------|
| `processor time per tick` (the gate) | cpu | `crates/engine-cli/src/bench.rs` (`bench.cpu_us_per_tick`) | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` |
| `peak memory` (gate) | memory | `crates/engine/src/observe/process.rs` | engine-cli bench | same command; field `bench.peak_mem_mb` |
| `processor time per match` | cpu | `crates/engine-cli/src/bench.rs` | engine-cli bench | same command; `bench.cpu_ms / bench.matches` |
| `full match wall time` | latency | `crates/engine-cli/src/bench.rs:run_one` | engine-cli bench | same command; fields `bench.match_wall_ms` and `bench.ticks_per_match` |
| `ticks per second` | throughput | `crates/engine/src/sim.rs:run` | engine-cli bench | same command; field `engine.ticks_per_s` |
| `tick step` | cpu | `crates/engine/benches/tick_step.rs` | criterion 0.8 | `cargo bench -p engine --bench tick_step` |
| `stream throughput` | throughput | `crates/stream/src/session.rs` | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json`; field `bench.stream_ticks_per_s` |

## Baseline Results

| Target | Median | P95 | P99 | Allocs/op | Bytes/op | Runs | Notes |
|--------|--------|-----|-----|-----------|----------|------|-------|
| `processor time per tick` (gate) | 1.5422 µs | N/A | N/A | N/A | N/A | 3 drives x 5 matches | drives 1.5319, 1.5422, 1.5422 µs; tripwire 1.6964 µs (+10%) |
| `peak memory` (gate) | 6.660 MB | N/A | N/A | N/A | N/A | 3 drives | drives 6.836, 6.656, 6.660 MB; tripwire 8.33 MB (+25%) |
| `processor time per match` | 450.0 ms | N/A | N/A | N/A | N/A | 3 drives | drives 447.0, 450.0, 450.0 ms; reported, +10% would be 495.0 ms |
| `full match wall time` | 449 ms | N/A | N/A | N/A | N/A | 3 drives | drives 449, 449, 450 ms; 291,800 ticks per match; budget 2000 ms |
| `ticks per second` | 649,889 | N/A | N/A | N/A | N/A | 3 drives | 649,889, 649,889, 648,444 |
| `tick step` | 1.5408 µs | N/A | N/A | N/A | N/A | criterion auto | interval 1.5231 to 1.5581 µs; criterion reports +3.3% against its saved state (p = 0.00); that state is older than this run and is not the gate; `steering_pass_22` 869.58 ns (interval 867.38 to 871.91 ns, within noise) |
| `stream throughput` | 446,862 ticks/s delivered | N/A | N/A | N/A | N/A | 1 drive | 0 pauses; peak memory 8.207 MB on this workload; stream memory tripwire 10.26 MB |

Evidence: `bench-baseline/lone-forward/bench-1.stdout.txt`, `bench-2.stdout.txt`, `bench-3.stdout.txt`, `bench-stream-1.stdout.txt` and `criterion.stdout.txt`, each with its `.stderr.txt` and `.exit-code` (all 0), plus `build.stderr.txt`, `commit.txt` (`a447ff1`) and `measured-at.txt` (`2026-09-24T11:01:30Z`). The build hash is `a447ff1-dirty`; the suffix comes from workflow files and staged design documents, not from engine code. The content hash is `3b3ddb56d4ec`. The machine hash is `74ca12fc08a4` (the 8-core reference machine). The processor-to-wall ratios are 0.993, 1.000 and 0.998. No reading is discarded.

The per-tick figure (1.5422 µs) is 4.5% above the defending-and-discipline baseline (1.4753 µs) and 0.7% above that slice's own compare reading (1.5312 µs). The defending mechanisms explain the rise, and that slice's gate passed. It is a new baseline, not a regression of this slice.

## Measurement Commands

Exact commands to reproduce these results, in order:

```bash
# 1. processor time per tick (the gate), per match, wall time, ticks per second, peak memory
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

None of the engine targets failed. The four page targets from the viewer slice are not re-measured, because this slice changes no drawing, decoding or history code.

## Compare rules for this baseline

- The gate is `bench.cpu_us_per_tick` against 1.6964 µs (+10 percent) and `bench.peak_mem_mb` against 8.33 MB (+25 percent), each the median of three drives. A reading over either limit is a tripwire.
- `bench.cpu_ms / bench.matches` (450.0 ms at baseline) and `bench.ticks_per_match` (291,800) are reported beside the gate with their deltas. The 2000 ms per-match budget still applies (NFR-1).
- The stream workload keeps its own memory tripwire of 10.26 MB.
- **Update the sibling `05c-benchmark.yaml` in the same pass:** set `mode: compare`, fill `compare_commit`, move each baseline value from `after` to `before`, and write the new reading to `after`.
