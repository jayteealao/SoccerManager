---
schema: sdlc/v1
type: augmentation
augmentation-type: benchmark
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: viewer-pitch
mode: complete
language: "rust, javascript"
benchmark-framework: "timing-fallback (engine-cli bench) plus criterion 0.8 for the engine; driven-browser signal reads plus node --test for the page"
targets-measured: 5
targets-failed: 0
targets-planned: 9
baseline-branch: feat/football-manager-match-engine
baseline-commit: "3570eb9"
measured-at: "2026-09-22T14:47:49Z"
compare-branch: feat/football-manager-match-engine
compare-commit: "36139b4"
compared-at: "2026-09-22T18:24:00Z"
regressions-found: 0
improvements-found: 1
budget-misses: 1
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T18:24:00Z"
revision-count: 3
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
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-viewer-pitch.md
  prior: history/05c-benchmark-2.md
---

# Benchmark: Viewer Pitch and Playback (baseline)

## The Benchmark

The stream slice closed with five targets compared and no tripwire fired: 389 milliseconds per match, 381 milliseconds of processor time, 5.97 megabytes of peak memory, 1.4376 microseconds per tick step, and the first socket reading at 640,213 ticks per second delivered. That record, with its comparison table, is byte-copied at `history/05c-benchmark-2.md`.

This baseline re-measures all five on commit `3570eb9`, before `viewer-pitch` changes any code, and the engine has not moved: 389 milliseconds wall, 694,087 ticks per second, 388 milliseconds of processor time, 5.97 megabytes peak, 1.4378 microseconds per tick step, and 634,859 ticks per second delivered over the socket with one pause. Three drives of the first command gave 387, 389, and 390 milliseconds. The re-baseline matters this time because `viewer-pitch` does touch Rust: a static file server, a replay speed cap, and two new fields in the opening message.

Four browser targets are declared and none is measured, because no page exists on this commit. They are budgets, not baselines: 60 frames per second median with zero dropped ticks, a 95th-percentile frame time under 16.6 milliseconds, a mean decode under 25 microseconds per frame, and a whole-page memory figure under 300 megabytes against an expected 24.2 megabytes of history. Verify measures each for the first time, and those first readings become the baseline for `viewer-match-day`, which is the slice that will actually put the frame budget under pressure by adding panels beside the pitch. The top risk is not drawing — research measured that as sub-millisecond — it is that a 144 hertz display makes a naive frame count report 144 and pass for the wrong reason, which is why `refresh_hz` rides beside every frame-rate reading.

## Benchmark Targets

| Target | Type | File:line | Framework | Command |
|--------|------|-----------|-----------|---------|
| `full match wall time` | latency | `crates/engine-cli/src/bench.rs:measure` | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` |
| `ticks per second` | throughput | `crates/engine/src/sim.rs:run` | engine-cli bench | same command; field `engine.ticks_per_s` |
| `cpu time and peak memory` | cpu/memory | `crates/engine/src/observe/process.rs` | engine-cli bench | same command; fields `bench.cpu_ms`, `bench.peak_mem_mb` |
| `tick step` | cpu | `crates/engine/benches/tick_step.rs` | criterion 0.8 | `cargo bench -p engine --bench tick_step` |
| `stream throughput` | throughput | `crates/stream/src/session.rs` | engine-cli bench | `target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json`; field `bench.stream_ticks_per_s` |
| `viewer frame rate` (new) | throughput | `web/schedule.mjs` (planned) | driven browser, signal read | drive the page at 1x for 5 minutes; read `viewer.frame_budget` fields `fps_median`, `dropped_frames`, `refresh_hz` |
| `viewer frame time p95` (new) | latency | `web/schedule.mjs` (planned) | driven browser, signal read | same drive; field `frame_ms_p95` |
| `viewer decode time` (new) | cpu | `web/decode.mjs` (planned) | node --test, timed loop | `node --test web/tests/` — decode 10,000 recorded frames, report mean microseconds per frame |
| `viewer history bytes` (new) | memory | `web/history.mjs` (planned) | node --test plus driven browser | `node --test web/tests/` for `history_bytes` over a full-match history; driven browser for `page_bytes` from the gauge |

## Baseline Results

| Target | Median | P95 | P99 | Allocs/op | Bytes/op | Runs | Notes |
|--------|--------|-----|-----|-----------|----------|------|-------|
| `full match wall time` | 389 ms | N/A | N/A | N/A | N/A | 3 drives x (5 + 1 warm-up) | drives 387, 389, 390 ms; budget 2000 ms |
| `ticks per second` | 694,087 | N/A | N/A | N/A | N/A | 3 drives | 692,308 to 697,674; budget 135,000 |
| `cpu time and peak memory` | 388 ms CPU per match; 5.97 MB peak | N/A | N/A | N/A | N/A | 3 drives | per-match CPU 387.4, 387.6, 390.6 ms; peak 5.973, 5.965, 5.969 MB; tripwires +10% CPU (427 ms), +25% memory (7.46 MB) |
| `tick step` | 1.4378 µs | N/A | N/A | N/A | N/A | criterion auto | interval 1.4342 to 1.4414 µs; `steering_pass_22` 898.22 ns, +1.49% against 888.96 (p = 0.00), far inside the tripwire |
| `stream throughput` | 634,859 ticks/s delivered | N/A | N/A | N/A | N/A | 1 drive | 1 pause; peak memory 7.30 MB on this workload; `engine.ticks_per_s` 690,537 with the socket on; tripwires +10% CPU, +25% memory (9.13 MB) against this workload's own numbers |
| `viewer frame rate` | not measured | N/A | N/A | N/A | N/A | 0 | No page exists at baseline. Budget: `fps_median` at least 60 with `dropped_frames` 0 over a 5-minute window at 1x (AC-1, NFR-2) |
| `viewer frame time p95` | not measured | N/A | N/A | N/A | N/A | 0 | No page exists at baseline. Budget: under 16.6 ms, the frame budget at 60 frames per second |
| `viewer decode time` | not measured | N/A | N/A | N/A | N/A | 0 | No page exists at baseline. Budget: mean under 25 µs per frame, so 400 frames per second at 8x costs under 10 ms of wall time per second |
| `viewer history bytes` | not measured | N/A | N/A | N/A | N/A | 0 | No page exists at baseline. Budget: `page_bytes` under 314,572,800 (300 MB, NFR-3). Expected `history_bytes` 25,380,000 for 270,000 ticks at 47 int16 components |

Evidence: `bench-baseline/viewer-pitch/bench-1.stdout.txt`, `bench-2.stdout.txt`, `bench-3.stdout.txt`, `bench-stream-1.stdout.txt`, `criterion.stdout.txt`, exit code 0. Build hash `3570eb9-dirty`; the `-dirty` suffix comes from the hook-appended cost ledger, not from code. Content hash `02d33ad91de5`, unchanged since the last two baselines. Machine hash `74ca12fc08a4`, AMD Ryzen 7 9800X3D, Ultimate Performance plan.

The three drives agree closely this time, so no reading is discarded: wall times 387, 389, 390 milliseconds with processor-to-wall ratios of 0.997, 0.996, and 1.003, and peak memory within 8 kilobytes across all three. Processor time per match rose from 381 to 388 milliseconds against the last compare run, which is 1.8 percent and well inside the tripwire; nothing in the two intervening commits touched the tick loop.

## Measurement Commands

Exact commands to reproduce these results, in order:

```bash
# 1. full match wall time, ticks per second, cpu time and peak memory
#    (one thread; warm-up run discarded; 5 timed matches per drive; median reported; three drives, median of medians)
cargo build --release -p engine-cli
target/release/engine-cli.exe bench --seed 42 --matches 5 --json

# 2. tick step (criterion, harness = false)
cargo bench -p engine --bench tick_step

# 3. stream throughput (socket on, in-process client reading as fast as it can)
target/release/engine-cli.exe bench --seed 42 --matches 1 --stream --json

# 4. viewer decode time and history bytes (after implement; no baseline value exists)
node --test web/tests/

# 5. viewer frame rate and frame time (after implement; no baseline value exists)
#    Start the engine serving the page and the fixture, then drive the page in the in-app browser
#    for 5 minutes of match time at 1x and read the viewer.frame_budget signal from the console.
target/release/engine-cli.exe replay --fixture <fixture>.smfx --speed 1.0 --web web
```

Run every command from PowerShell or Git Bash with no other heavy process running, and with `SM_DATA_DIR` set to a scratch folder so the run does not touch the real runtime folder. Command 1 loads the content folder and the two shipped default team files; the compare run must use the shipped defaults, not generated teams, so the workload matches. Command 3 must never run while command 1 is running. Command 5 needs the browser window at 1280 by 800 and reports `refresh_hz`, which must be recorded beside `fps_median` — a 144 hertz panel makes a raw callback count meaningless.

## Targets That Could Not Be Measured

All four browser targets — `viewer frame rate`, `viewer frame time p95`, `viewer decode time`, `viewer history bytes`. No page, no JavaScript module, and no browser process exists on commit `3570eb9`; the repository holds no HTML, CSS, or JavaScript file at all. Each target is declared here with its budget so verify measures it once, against the budget rather than against a prior reading, and those first readings become the baseline for `viewer-match-day`. Recording a first value is never a regression.

## Comparison Results

Run by `/wf verify football-manager-match-engine viewer-pitch` on commit `36139b4` (the slice at `62a5dab` plus a docs commit), with no other heavy process running. The engine commands ran exactly as recorded; the page targets ran in headless Microsoft Edge 153 at 1280 by 800 and 60 Hz, driven over the DevTools protocol, and in `node --test`-style timed loops. Evidence: `verify-evidence/viewer-pitch/bench-1.stdout.txt` to `bench-3.stdout.txt`, `bench-stream-1.stdout.txt`, `criterion.stdout.txt`, `decode-timing.txt`, `drive-main.json`, `drive-live.json`.

| Target | Baseline median | Compare median | Delta | Delta% | Alloc delta% | Verdict |
|--------|----------------|---------------|-------|--------|--------------|---------|
| `full match wall time` | 389 ms | 386 ms | −3 ms | −0.8% | N/A | ✓ no change |
| `ticks per second` | 694,087 | 699,482 | +5,395 | +0.8% | N/A | ✓ no change |
| `cpu time` | 388 ms per match | 384.4 ms per match | −3.6 ms | −0.9% | N/A | ✓ no change |
| `peak memory` | 5.97 MB | 5.33 MB | −0.64 MB | −10.7% | N/A | ✓ improvement |
| `tick step` | 1.4378 µs | 1.4434 µs | +0.0056 µs | +0.4% | N/A | ✓ no change (criterion p = 0.37) |
| `steering_pass_22` | 898.22 ns | 873.52 ns | −24.7 ns | −2.7% | N/A | ✓ no change (informational) |
| `stream throughput` | 634,859 ticks/s | 640,584 ticks/s | +5,725 | +0.9% | N/A | ✓ no change; 2 pauses |
| `stream peak memory` | 7.30 MB | 6.66 MB | −0.64 MB | −8.8% | N/A | ✓ no change (inside −10%) |
| `viewer frame rate` | budget ≥ 60 fps, 0 dropped | 60 fps at `refresh_hz` 60 over 60 five-second windows | first value | N/A | N/A | ✓ budget met: after the verify-owned scheduler fix `c7c54bd`, 0 dropped frames and 0 skipped ticks over 5 minutes at 1x (the first round saw 0 to 2 dropped frames per window and a startup tick skip) |
| `viewer frame time p95` | budget < 16.6 ms | 16.69 to 16.85 ms (second round 16.70 to 16.84 ms) | first value | N/A | N/A | ⚠ budget miss by construction: the p95 of 60 Hz frame intervals cannot fall under 16.67 ms |
| `viewer decode time` | budget < 25 µs | 0.083 µs per frame (ArrayBuffer input), 0.232 µs (Uint8Array input) | first value | N/A | N/A | ✓ budget met, 300 times under |
| `viewer history bytes` | budget 25,380,000 exact; `page_bytes` < 314,572,800 | `history_bytes` 25,380,000; `page_bytes` 31,924,655 at 270,000 ticks | first value | N/A | N/A | ✓ budget met; `page_bytes` resolved in headless Edge (the in-app pane refuses the measurement) |

Median of three drives of command 1: wall 386, 386, 385 ms; per-match processor time 384.4, 384.4, 381.2 ms; peak memory 5.348, 5.332, 5.332 MB. The peak-memory drop is the second consecutive improvement reading and no engine code changed in this slice; the likely cause is allocator placement, not a code effect, so it is recorded and not credited.

**Tripwires:**

- `[budget-definition]: viewer frame time p95 16.6 ms is below one 60 Hz frame interval (16.67 ms)` — the reading shows vsync, not work time. Verify triaged it Skip; `viewer-match-day` re-states the budget relative to `refresh_hz`.

No performance or memory tripwire fired.

Compare rules for this baseline:

- The five engine targets compare against the medians above, with tripwires at +10 percent processor time (427 milliseconds per match) and +25 percent memory (7.46 megabytes). The stream workload keeps its own memory tripwire of 9.13 megabytes, because it carries a socket, a 500-tick producer buffer, a client thread, and the tungstenite write buffer that command 1 does not.
- The four browser targets have no prior median, so each is checked against its budget and recorded as a first value. A budget miss is a blocker, not a regression.
- `viewer history bytes` carries two numbers, per plan Round 3 Q9: `history_bytes` is exact and comes from `node --test`; `page_bytes` is the browser's whole-page figure and comes from the driven browser. Report both. A `page_bytes` of `null` means the browser never resolved its measurement, which is a failed drive, not a pass.
- **Update the sibling `05c-benchmark.yaml` in the same pass.** The sibling schema requires an `after` value on every metric, so `before` holds the previous slice's reading and `after` holds the current one; set `mode: compare` and fill `compare_commit`. The stream-protocol run updated this document and left its sibling at `mode: baseline` with an empty `compare_commit`, so the rendered page showed baseline-only numbers while the document showed a full comparison. That drift is visible in `history/05c-benchmark-2.yaml` and must not repeat.
