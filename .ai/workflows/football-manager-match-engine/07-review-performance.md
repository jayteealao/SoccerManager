---
schema: sdlc/v1
type: review-command
slug: football-manager-match-engine
dimension: performance
parent: 07-review.md
review-scope: slug-wide
status: complete
created-at: "2026-09-23T20:34:36Z"
updated-at: "2026-09-23T20:34:36Z"
verdict: ship
metric-findings-total: 1
metric-findings-blocker: 0
metric-findings-high: 0
---

# Review: performance

9 findings in this dimension. 8 are fixed and 1 are deferred. Nothing is open at BLOCKER or HIGH.

| ID | Sev | Conf | Status | Surfaced | File:Line | Issue | Fix / reason |
|---|---|---|---|---|---|---|---|
| PERF-3 | MED | high | fixed | 2026-09-23T20:34:36Z | crates/engine-cli/src/calibrate/worker.rs:121-123 | Each calibration match keeps every tick in memory with Vec doubling | Records preallocated to max_ticks (the finding's stated minimum): no doubling peaks, no copies in ticks_per_s. An incremental validator was not built; one match of records is still held per worker. |
| PERF-1 | MED | high | fixed | 2026-09-23T20:34:36Z | crates/stream/src/session.rs:465-484 | Socket thread spins at a full core under backpressure | Only a write the socket took counts as work. |
| PERF-2 | MED | high | fixed | 2026-09-23T20:34:36Z | crates/stream/src/session.rs:30 | Socket thread polls every 100 us with no backoff | Idle sleep doubles from 100 us up to 1 ms and resets on work. |
| PERF-5 | LOW | medium | deferred | 2026-09-23T20:34:36Z | crates/engine/src/sim.rs:766 | step clones the whole Tuning every tick | The fix (Arc<Tuning> or split borrows) touches every tuning reader and test that edits tuning in place; not localized. |
| PERF-4 | LOW | high | fixed | 2026-09-23T20:34:36Z | crates/engine/src/validate.rs:96-101 | Validator allocates two Vecs per tick | Fixed-size arrays. |
| PERF-6 | LOW | high | fixed | 2026-09-23T20:34:36Z | web/main.mjs:308, 774-780 | Two sorts of the 300-frame window every animation frame, and an unconditional gauge write | Panel rate re-measured every 30 frames; gauge refreshed every 15 frames and written only on change. |
| PERF-7 | LOW | high | fixed | 2026-09-23T20:34:36Z | web/main.mjs:744 | fullTimeTick scans all events on every tick and frame | MatchState keeps the full-time tick on add, recomputes on truncate, clears on clear. Test added. |
| PERF-9 | NIT | high | fixed | 2026-09-23T20:34:36Z | crates/engine-cli/src/simulate.rs:80-86 | simulate --json reads the tick file twice | Read once. |
| PERF-10 | NIT | medium | fixed | 2026-09-23T20:34:36Z | web/pitch.mjs:235, 254-260 | Font set twice per frame; trail comment wrong | Font set once per frame; comment corrected (per-segment alpha needs one stroke each). |
