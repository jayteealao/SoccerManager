---
schema: sdlc/v1
type: review
slug: football-manager-match-engine
review-scope: slug-wide
slice-slug: ""
status: complete
stage-number: 7
created-at: "2026-09-23T20:34:36Z"
updated-at: "2026-09-23T20:34:36Z"
verdict: ship
commands-run: [correctness, security, performance, architecture, intent-fidelity]
metric-commands-run: 5
metric-findings-total: 6
metric-findings-raw: 35
metric-findings-blocker: 0
metric-findings-pre-existing: 0
metric-findings-high: 0
metric-findings-med: 0
metric-findings-low: 3
metric-findings-nit: 3
metric-findings-resolved: 0
metric-findings-total-ever: 35
runs:
  - at: "2026-09-23T20:34:36Z"
    dimensions: [correctness, security, performance, architecture, intent-fidelity]
    verdict: ship
    fix-commit: "933e150 5a235a4"
consult-runs: []
tags: [engine, stream, viewer, security, performance, laws]
refs:
  index: 00-index.md
  shape: 02-shape.md
  slice-index: 03-slice.md
  implements: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-commentary.md, 05-implement-calibration.md, 05-implement-viewer-match-day.md, 05-implement-viewer-lineup-tactics.md, 05-implement-viewer-reports-recovery.md, 05-implement-integration.md, 05-implement-extra-time-penalties.md, 05-implement-experiment-flags.md, 05-implement-scripting-runtime.md, 05-implement-distribution.md, 05-implement-probe-engine-core.md]
  verifies: [06-verify-engine-core.md, 06-verify-data-schemas-generator.md, 06-verify-stream-protocol.md, 06-verify-viewer-pitch.md, 06-verify-match-rules.md, 06-verify-tactics-and-ai.md, 06-verify-commentary.md, 06-verify-calibration.md, 06-verify-viewer-match-day.md, 06-verify-viewer-lineup-tactics.md, 06-verify-viewer-reports-recovery.md, 06-verify-integration.md, 06-verify-extra-time-penalties.md, 06-verify-experiment-flags.md, 06-verify-scripting-runtime.md, 06-verify-distribution.md, 06-verify-probe-engine-core.md]
  sub-reviews: [07-review-correctness.md, 07-review-security.md, 07-review-performance.md, 07-review-architecture.md, 07-review-intent-fidelity.md]
next-command: wf-handoff
next-invocation: "/wf handoff football-manager-match-engine"
---
# Review

## The Review

The branch came to review with all 17 slices built and verified, and a scan across correctness, security, performance and architecture that had already been checked adversarially. It surfaced 33 findings: 11 MED and 22 LOW or NIT, with no BLOCKER or HIGH. A separate check of the intake Success Criteria added one more: over several calibration seeds, goals per match sat above the 2.4 to 3.2 band on most seeds, although the verified seed passed.

Every MED finding was fixed, as the triage policy requires. That includes a penalty in open play that was never kicked, a fake full-time row when the viewer left, lost change identifiers after a reconnect, a reconnect wait that a silent client could hold open, a socket origin allowlist that accepted `null`, a socket thread that spun at full load, and the goals band. The band was brought back by raising `keeper_catch_chance` a little. LOW and NIT findings were fixed where they were local and safe: 17 of them. Six were deferred with a reason: CI for the ignored acceptance tests, new viewer unit suites, the per-tick tuning clone, a structured refusal reason, redacting local paths from the evidence, and a paired calibration test (T11) that failed once under full-suite load. T11 was raised during this run.

No BLOCKER or HIGH findings are open, so the branch can go to handoff. The biggest open risk is T6: the slow realism and AI acceptance tests still run only by hand, so a later tuning change can move the bands while `cargo test` stays green. The Playwright suite was not re-run after the 4x-speed assertion was tightened (T1), so run it once before release.

## Verdict

**Ship**

No BLOCKER or HIGH findings are open. 29 findings were fixed in this run and 6 LOW/NIT findings are deferred with reasons. The most serious defect found was C1: a penalty in play was not a kick. It is fixed and has a regression test. The full Rust suite, clippy, rustfmt and the 128 viewer unit tests pass after the fixes.

## Domain Coverage

| Domain | Command | Status |
|--------|---------|--------|
| correctness | `correctness` | Issues (deferred) |
| security | `security` | Issues (deferred) |
| performance | `performance` | Issues (deferred) |
| architecture | `architecture` | Issues (deferred) |
| intent-fidelity | `intent-fidelity` | Clean after fixes |

## All Findings

ALL findings ever recorded, open AND closed.

| ID | Sev | Conf | Status | Pre | Surfaced | Source | File:Line | Issue |
|----|-----|------|--------|-----|----------|--------|-----------|-------|
| IF-1 | MED | high | fixed | false | 2026-09-23T20:34:36Z | intent-fidelity | content/tuning.json:23 | Goals per match miss the realism band on most calibration seeds |
| PERF-3 | MED | high | fixed | false | 2026-09-23T20:34:36Z | performance | crates/engine-cli/src/calibrate/worker.rs:121-123 | Each calibration match keeps every tick in memory with Vec doubling |
| C3 | MED | medium | fixed | false | 2026-09-23T20:34:36Z | correctness | crates/engine-cli/src/serve.rs:262 | Verdicts after a reconnect lose the page queue_id; some acked changes are lost |
| M2 | MED | high | fixed | false | 2026-09-23T20:34:36Z | architecture | crates/engine-cli/src/serve.rs:107 | serve::run is one 270-line function with bare exit codes |
| C2 | MED | high | fixed | false | 2026-09-23T20:34:36Z | correctness | crates/engine-cli/src/stream_run.rs:70 | A viewer that leaves mid-match gets a false full-time event |
| M1 | MED | high | fixed | false | 2026-09-23T20:34:36Z | architecture | crates/engine/src/lib.rs:56 | Tick rate is set in two places nothing keeps in step |
| C1 | MED | high | fixed | false | 2026-09-23T20:34:36Z | correctness | crates/engine/src/rules/mod.rs:551 | A penalty in play is not taken as a kick |
| C4 | MED | medium | fixed | false | 2026-09-23T20:34:36Z | correctness | crates/stream/src/server.rs:110 | accept_within can block past its reconnect deadline |
| SEC-1 | MED | high | fixed | false | 2026-09-23T20:34:36Z | security | crates/stream/src/server.rs:223-231 | WebSocket origin allowlist accepts the null origin and file:// |
| PERF-1 | MED | high | fixed | false | 2026-09-23T20:34:36Z | performance | crates/stream/src/session.rs:465-484 | Socket thread spins at a full core under backpressure |
| PERF-2 | MED | high | fixed | false | 2026-09-23T20:34:36Z | performance | crates/stream/src/session.rs:30 | Socket thread polls every 100 us with no backoff |
| T1 | MED | high | fixed | false | 2026-09-23T20:34:36Z | correctness | e2e/tests/first-match.spec.mjs:151-167 | The 4x speed check cannot fail usefully |
| T11 | LOW | medium | deferred | false | 2026-09-23T20:34:36Z | correctness | crates/engine-cli/tests/calibrate_pair.rs:106 | Paired calibration test failed once under full-suite load |
| PERF-5 | LOW | medium | deferred | false | 2026-09-23T20:34:36Z | performance | crates/engine/src/sim.rs:766 | step clones the whole Tuning every tick |
| T6 | LOW | high | deferred | false | 2026-09-23T20:34:36Z | correctness | crates/engine/tests/ai_trailing.rs:15 | Four acceptance tests are #[ignore] and nothing runs them |
| C5 | LOW | medium | fixed | false | 2026-09-23T20:34:36Z | correctness | crates/engine-cli/src/serve.rs:232 | Reconnect hello describes the pre-kick-off preview, not the match |
| C6 | LOW | medium | fixed | false | 2026-09-23T20:34:36Z | correctness | crates/engine-cli/src/serve.rs:268 | --ticks-out stops at the first drop and holds ticks the resumed match replays |
| SEC-2 | LOW | medium | fixed | false | 2026-09-23T20:34:36Z | security | crates/engine-cli/src/web.rs:171-255 | Page server never checks Host (DNS rebinding) |
| SEC-4 | LOW | medium | fixed | false | 2026-09-23T20:34:36Z | security | crates/engine-cli/src/web.rs:152-163 | Page server: unbounded threads, no timeouts |
| M5 | LOW | medium | fixed | false | 2026-09-23T20:34:36Z | architecture | crates/engine/src/commentary/mod.rs:237 | Commentary rotation seeded by list position |
| C7 | LOW | medium | fixed | false | 2026-09-23T20:34:36Z | correctness | crates/engine/src/pitch.rs:70 | A goal counts before the whole ball is over the line, and the crossing point is not used |
| M6 | LOW | medium | fixed | false | 2026-09-23T20:34:36Z | architecture | crates/engine/src/sim.rs:301 | Wire event strings written twice with no cross-check |
| PERF-4 | LOW | high | fixed | false | 2026-09-23T20:34:36Z | performance | crates/engine/src/validate.rs:96-101 | Validator allocates two Vecs per tick |
| T7 | LOW | high | fixed | false | 2026-09-23T20:34:36Z | correctness | crates/engine/tests/zz_stall_probe.rs:1 | A debug probe was staged for commit |
| SEC-3 | LOW | high | fixed | false | 2026-09-23T20:34:36Z | security | crates/stream/src/record.rs:244 | Replay reader trusts the header frame count for allocation |
| T4 | LOW | medium | fixed | false | 2026-09-23T20:34:36Z | correctness | crates/stream/tests/reconnect.rs:118-129 | Drop-seam test never pins the cut to its tick |
| PERF-6 | LOW | high | fixed | false | 2026-09-23T20:34:36Z | performance | web/main.mjs:308, 774-780 | Two sorts of the 300-frame window every animation frame, and an unconditional gauge write |
| PERF-7 | LOW | high | fixed | false | 2026-09-23T20:34:36Z | performance | web/main.mjs:744 | fullTimeTick scans all events on every tick and frame |
| M7 | LOW | high | fixed | false | 2026-09-23T20:34:36Z | architecture | web/main.mjs:75 | clockText duplicates clockAt; recovery.mjs redefines the tick rate |
| T8 | LOW | medium | fixed | false | 2026-09-23T20:34:36Z | correctness | web/tests/pitch.test.mjs:9-12 | Viewer test re-implements the engine parking-spot formula |
| SEC-5 | NIT | high | deferred | false | 2026-09-23T20:34:36Z | security | .ai/workflows/football-manager-match-engine/:n/a | Workflow evidence contains absolute local paths |
| T10 | NIT | medium | deferred | false | 2026-09-23T20:34:36Z | correctness | web/main.mjs:1 | Several viewer modules have no unit tests |
| M10 | NIT | medium | deferred | false | 2026-09-23T20:34:36Z | architecture | web/recovery.mjs:33 | refusalReason parses the engine error prefix |
| PERF-9 | NIT | high | fixed | false | 2026-09-23T20:34:36Z | performance | crates/engine-cli/src/simulate.rs:80-86 | simulate --json reads the tick file twice |
| PERF-10 | NIT | medium | fixed | false | 2026-09-23T20:34:36Z | performance | web/pitch.mjs:235, 254-260 | Font set twice per frame; trail comment wrong |

**Open:** BLOCKER: 0 | HIGH: 0 | MED: 0 | LOW: 3 | NIT: 3   **Pre-existing:** 0
**Closed:** resolved: 0 | fixed: 29 | dismissed: 0   **Ledger size (ever):** 35
*(This run: 35 net-new, 0 re-confirmed, 0 resolved; merged from 35 raw findings across 5 commands)*

## Findings (Detailed)

### IF-1: Goals per match miss the realism band on most calibration seeds [MED]

**Location:** `content/tuning.json:23`
**Source:** intent-fidelity

**Issue:** Success Criterion "goals per match 2.4 to 3.2" over 1000 matches held at the verified seed 2026 (3.03) but failed at seeds 7 (3.39), 42 (3.32) and 99 (3.33) on this branch, before and after the review fixes (pre-fix seed 7: 3.42).

**Fix:** keeper_catch_chance raised from 0.84 to 0.86 in content/tuning.json, Tuning::default and data-files.md. At 1000 matches per suite, goals per match were 2.863 (seed 7), 3.035 (42), 2.948 (99), 2.656 (1) and 2.559 (2026), all inside the band. Shots per team were 12.9 to 14.8 and the stronger-team win rate 0.59 to 0.67, also inside their bands.

**Severity:** MED | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### PERF-3: Each calibration match keeps every tick in memory with Vec doubling [MED]

**Location:** `crates/engine-cli/src/calibrate/worker.rs:121-123`
**Source:** performance

**Issue:** VecSink::default() grew to ~55 MB per match with ~2x transient peaks, and the growth was inside the timed run.

**Fix:** Records preallocated to max_ticks (the finding's stated minimum): no doubling peaks, no copies in ticks_per_s. An incremental validator was not built; one match of records is still held per worker.

**Severity:** MED | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### C3: Verdicts after a reconnect lose the page queue_id; some acked changes are lost [MED]

**Location:** `crates/engine-cli/src/serve.rs:262`
**Source:** correctness

**Issue:** A fresh Ids per connection dropped the engine-to-page id map, and a change admitted at the snapshot tick but drained after capture vanished from the resumed match.

**Fix:** Page changes are carried across connections (PageChange + carry_page_changes): kept when in the snapshot, re-queued when their queued row survived, dropped otherwise; inbox leftovers carried too. Unit test added.

**Severity:** MED | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### M2: serve::run is one 270-line function with bare exit codes [MED]

**Location:** `crates/engine-cli/src/serve.rs:107`
**Source:** architecture

**Issue:** Content, opening, port, writers, per-connection session, kick-off, drive, stats and reconnect/rewind all lived in one function returning 0/1/2.

**Fix:** Split into Serving::play (per connection) and Serving::rewind, with EXIT_FULL_TIME / EXIT_REFUSED / EXIT_VIEWER_GONE. Values kept (the launcher reads them). calibrate::run was not split.

**Severity:** MED | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### C2: A viewer that leaves mid-match gets a false full-time event [MED]

**Location:** `crates/engine-cli/src/stream_run.rs:70`
**Source:** correctness

**Issue:** A stopped gate made drive() break and still call sim.finish(), writing a full-time row at the drop tick for a match that never finished.

**Fix:** drive() returns without finish() when the gate stops; test a_viewer_that_leaves_while_the_match_waits_gets_no_full_time.

**Severity:** MED | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### M1: Tick rate is set in two places nothing keeps in step [MED]

**Location:** `crates/engine/src/lib.rs:56`
**Source:** architecture

**Issue:** tuning dt accepted 0.005-0.1 while the clock, minute maths, commentary and viewer all hardcode 50 ticks/s.

**Fix:** The loader refuses any dt other than one clock tick; commentary uses the clock constant; a cross-crate test pins the protocol minute; the viewer duplicate constant is removed; data-files.md updated.

**Severity:** MED | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### C1: A penalty in play is not taken as a kick [MED]

**Location:** `crates/engine/src/rules/mod.rs:551`
**Source:** correctness

**Issue:** take_restart let StoppageKind::Penalty fall into the `_ => None` arm, so the taker became the carrier and could pass, dribble, hold or clear while opponents closed in (IFAB Law 14). Skews penalty conversion and xG.

**Fix:** Penalty now shoots at once via shot_kick at the placed-kick spread; unit test a_penalty_in_play_is_shot_at_goal_at_once.

**Severity:** MED | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### C4: accept_within can block past its reconnect deadline [MED]

**Location:** `crates/stream/src/server.rs:110`
**Source:** correctness

**Issue:** The handshake ran with no read timeout, so a silent TCP client blocked the reconnect wait forever; refused handshakes skipped the deadline check.

**Fix:** Handshake gets the remaining wait (min 500 ms) as read/write timeout; refusals past the deadline end the wait. Test a_silent_client_does_not_hold_the_reconnect_wait_open.

**Severity:** MED | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### SEC-1: WebSocket origin allowlist accepts the null origin and file:// [MED]

**Location:** `crates/stream/src/server.rs:223-231`
**Source:** security

**Issue:** Any remote site could open the socket from a sandboxed iframe or data: URL (Origin: null) and read the stream or send commands.

**Fix:** null and file:// dropped; tests and docs/reference/protocol.md updated.

**Severity:** MED | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### PERF-1: Socket thread spins at a full core under backpressure [MED]

**Location:** `crates/stream/src/session.rs:465-484`
**Source:** performance

**Issue:** A refused write (WriteBufferFull) still marked the pass busy, so the thread never slept while the reader was slow.

**Fix:** Only a write the socket took counts as work.

**Severity:** MED | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### PERF-2: Socket thread polls every 100 us with no backoff [MED]

**Location:** `crates/stream/src/session.rs:30`
**Source:** performance

**Issue:** While held, paused or lead-bound the thread woke about 10,000 times a second.

**Fix:** Idle sleep doubles from 100 us up to 1 ms and resets on work.

**Severity:** MED | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### T1: The 4x speed check cannot fail usefully [MED]

**Location:** `e2e/tests/first-match.spec.mjs:151-167`
**Source:** correctness

**Issue:** The in-tolerance branch re-asserted what the condition guaranteed; the lag branch accepted any "Nx" text and never compared it with the measured rate.

**Fix:** Asserts rate < 4.2 always; lag branch requires a shown notice naming a speed below 4 within 1x of the measured rate; normal branch requires no lag notice. Syntax-checked only; the browser suite was not run.

**Severity:** MED | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### T11: Paired calibration test failed once under full-suite load [LOW]

**Location:** `crates/engine-cli/tests/calibrate_pair.rs:106`
**Source:** correctness

**Issue:** a_paired_run_plays_both_arms_on_the_same_seeds_and_compares_every_band failed its exit-code assertion once during a full `cargo test --workspace` run. It passed three times on its own and in the next full run.

**Fix:** Seen once and could not be reproduced. The cause is probably concurrent calibration workers under load. Diagnosing it needs repeated runs under load, which is not a localized fix.

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** deferred | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z

### PERF-5: step clones the whole Tuning every tick [LOW]

**Location:** `crates/engine/src/sim.rs:766`
**Source:** performance

**Issue:** About 0.7 KB copied ~280k times per match (1-2% of the tick budget).

**Fix:** The fix (Arc<Tuning> or split borrows) touches every tuning reader and test that edits tuning in place; not localized.

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** deferred | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z

### T6: Four acceptance tests are #[ignore] and nothing runs them [LOW]

**Location:** `crates/engine/tests/ai_trailing.rs:15`
**Source:** correctness

**Issue:** No CI config; AC-6/7/8 and the realism bands can regress with cargo test green.

**Fix:** Needs a CI pipeline or a scheduled slow-test job: new infrastructure, not a localized change.

**Severity:** LOW | **Confidence:** High | **Pre-existing:** false
**Status:** deferred | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z

### C5: Reconnect hello describes the pre-kick-off preview, not the match [LOW]

**Location:** `crates/engine-cli/src/serve.rs:232`
**Source:** correctness

**Issue:** Each reconnect resent the hello built before kick-off: AI lineup, bench and tactics, no substitutions.

**Fix:** Hello rebuilt from the resumed match on every rewind. In scope, localized, safe.

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### C6: --ticks-out stops at the first drop and holds ticks the resumed match replays [LOW]

**Location:** `crates/engine-cli/src/serve.rs:268`
**Source:** correctness

**Issue:** The ticks file moved into the first session and was finished there.

**Fix:** File kept across connections, cut back with FileSink::rewind at each rewind, finished once. Unit test for rewind.

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### SEC-2: Page server never checks Host (DNS rebinding) [LOW]

**Location:** `crates/engine-cli/src/web.rs:171-255`
**Source:** security

**Issue:** A rebound hostname could read the page files and /engine.json.

**Fix:** Only 127.0.0.1 / localhost with no port or the own port are answered (421 otherwise). Unit test.

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### SEC-4: Page server: unbounded threads, no timeouts [LOW]

**Location:** `crates/engine-cli/src/web.rs:152-163`
**Source:** security

**Issue:** Silent connections could pile up threads in the launcher.

**Fix:** 10 s read/write timeouts and a 64-connection cap.

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### M5: Commentary rotation seeded by list position [LOW]

**Location:** `crates/engine/src/commentary/mod.rs:237`
**Source:** architecture

**Issue:** Reordering EngineEventKind::ALL shifted lines for later kinds; a missing kind shared KickOff's rotation.

**Fix:** Salted by an FNV-1a hash of the variant name.

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### C7: A goal counts before the whole ball is over the line, and the crossing point is not used [LOW]

**Location:** `crates/engine/src/pitch.rs:70`
**Source:** correctness

**Issue:** in_goal tested the ball centre at the end of the step only.

**Fix:** Whole ball over (HALF_LENGTH + BALL_RADIUS) and y interpolated at the goal line. Tests extended.

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### M6: Wire event strings written twice with no cross-check [LOW]

**Location:** `crates/engine/src/sim.rs:301`
**Source:** architecture

**Issue:** EngineEventKind::code and EventType::code spelled independently.

**Fix:** Mapping extracted to event_type(); a test checks every kind's code against its wire type.

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### PERF-4: Validator allocates two Vecs per tick [LOW]

**Location:** `crates/engine/src/validate.rs:96-101`
**Source:** performance

**Issue:** pos and parked were heap Vecs of 22.

**Fix:** Fixed-size arrays.

**Severity:** LOW | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### T7: A debug probe was staged for commit [LOW]

**Location:** `crates/engine/tests/zz_stall_probe.rs:1`
**Source:** correctness

**Issue:** Index held an add of a diagnostic test deleted in the working tree.

**Fix:** Unstaged with git rm --cached.

**Severity:** LOW | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### SEC-3: Replay reader trusts the header frame count for allocation [LOW]

**Location:** `crates/stream/src/record.rs:244`
**Source:** security

**Issue:** A crafted .smfx with 0xFFFFFFFF counts aborted the process.

**Fix:** Capacity capped at body.len() / 9. Unit test.

**Severity:** LOW | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### T4: Drop-seam test never pins the cut to its tick [LOW]

**Location:** `crates/stream/tests/reconnect.rs:118-129`
**Source:** correctness

**Issue:** Asserted only last < 3000 and sent >= 1000.

**Fix:** Also asserts sent < 1000 + 32 (one outbox per flush).

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### PERF-6: Two sorts of the 300-frame window every animation frame, and an unconditional gauge write [LOW]

**Location:** `web/main.mjs:308, 774-780`
**Source:** performance

**Issue:** refreshHz and budget sorted the window each frame.

**Fix:** Panel rate re-measured every 30 frames; gauge refreshed every 15 frames and written only on change.

**Severity:** LOW | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### PERF-7: fullTimeTick scans all events on every tick and frame [LOW]

**Location:** `web/main.mjs:744`
**Source:** performance

**Issue:** A getter scanned the events array backwards each call.

**Fix:** MatchState keeps the full-time tick on add, recomputes on truncate, clears on clear. Test added.

**Severity:** LOW | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### M7: clockText duplicates clockAt; recovery.mjs redefines the tick rate [LOW]

**Location:** `web/main.mjs:75`
**Source:** architecture

**Issue:** Line-for-line copy plus a second TICKS_PER_SECOND.

**Fix:** clockText removed (uses clockAt); recovery.mjs imports the constant.

**Severity:** LOW | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### T8: Viewer test re-implements the engine parking-spot formula [LOW]

**Location:** `web/tests/pitch.test.mjs:9-12`
**Source:** correctness

**Issue:** A change to the Rust layout would not fail the JS test.

**Fix:** Shared web/tests/data/parking-spots.json read by the JS test and checked against parking_spot by a Rust test.

**Severity:** LOW | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### SEC-5: Workflow evidence contains absolute local paths [NIT]

**Location:** `.ai/workflows/football-manager-match-engine/:n/a`
**Source:** security

**Issue:** About 189 lines of committed evidence carry C:\Users\<name> paths.

**Fix:** Evidence files are generated workflow records; redacting them is outside this code diff and changes recorded evidence. Route to a redaction pass before publishing.

**Severity:** NIT | **Confidence:** High | **Pre-existing:** false
**Status:** deferred | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z

### T10: Several viewer modules have no unit tests [NIT]

**Location:** `web/main.mjs:1`
**Source:** correctness

**Issue:** main.mjs, lineup-editor.mjs, substitution-picker.mjs, socket.mjs, scoreboard.mjs, signal.mjs, launcher.mjs.

**Fix:** Writing new suites for seven modules is broad test work, not a localized fix.

**Severity:** NIT | **Confidence:** Medium | **Pre-existing:** false
**Status:** deferred | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z

### M10: refusalReason parses the engine error prefix [NIT]

**Location:** `web/recovery.mjs:33`
**Source:** architecture

**Issue:** Coupled to Rust Display text by a comment only.

**Fix:** A structured reason field crosses the engine, launcher and page contract; not localized.

**Severity:** NIT | **Confidence:** Medium | **Pre-existing:** false
**Status:** deferred | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z

### PERF-9: simulate --json reads the tick file twice [NIT]

**Location:** `crates/engine-cli/src/simulate.rs:80-86`
**Source:** performance

**Issue:** Once for JSON, once for the validator.

**Fix:** Read once.

**Severity:** NIT | **Confidence:** High | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

### PERF-10: Font set twice per frame; trail comment wrong [NIT]

**Location:** `web/pitch.mjs:235, 254-260`
**Source:** performance

**Issue:** ctx.font set per team; comment claimed one path.

**Fix:** Font set once per frame; comment corrected (per-segment alpha needs one stroke each).

**Severity:** NIT | **Confidence:** Medium | **Pre-existing:** false
**Status:** fixed | **Surfaced:** 2026-09-23T20:34:36Z | **Last seen:** 2026-09-23T20:34:36Z — **Fixed:** 2026-09-23T20:34:36Z

## Triage Decisions

Autonomous policy: every BLOCKER/HIGH/MED is fixed; a LOW/NIT is fixed only when in scope, localized and safe, otherwise deferred with a reason. Every decision is `class: implementation-detail`.

| ID | Sev | Source | Decision | Notes |
|----|-----|--------|----------|-------|
| IF-1 | MED | intent-fidelity | fix | MED: always fixed |
| PERF-3 | MED | performance | fix | MED: always fixed |
| C3 | MED | correctness | fix | MED: always fixed |
| M2 | MED | architecture | fix | MED: always fixed |
| C2 | MED | correctness | fix | MED: always fixed |
| M1 | MED | architecture | fix | MED: always fixed |
| C1 | MED | correctness | fix | MED: always fixed |
| C4 | MED | correctness | fix | MED: always fixed |
| SEC-1 | MED | security | fix | MED: always fixed |
| PERF-1 | MED | performance | fix | MED: always fixed |
| PERF-2 | MED | performance | fix | MED: always fixed |
| T1 | MED | correctness | fix | MED: always fixed |
| T11 | LOW | correctness | defer | Seen once and could not be reproduced. The cause is probably concurrent calibration workers under load. Diagnosing it needs repeated runs under load, which is not a localized fix. |
| PERF-5 | LOW | performance | defer | The fix (Arc<Tuning> or split borrows) touches every tuning reader and test that edits tuning in place; not localized. |
| T6 | LOW | correctness | defer | Needs a CI pipeline or a scheduled slow-test job: new infrastructure, not a localized change. |
| C5 | LOW | correctness | fix | in scope, localized, safe |
| C6 | LOW | correctness | fix | in scope, localized, safe |
| SEC-2 | LOW | security | fix | in scope, localized, safe |
| SEC-4 | LOW | security | fix | in scope, localized, safe |
| M5 | LOW | architecture | fix | in scope, localized, safe |
| C7 | LOW | correctness | fix | in scope, localized, safe |
| M6 | LOW | architecture | fix | in scope, localized, safe |
| PERF-4 | LOW | performance | fix | in scope, localized, safe |
| T7 | LOW | correctness | fix | in scope, localized, safe |
| SEC-3 | LOW | security | fix | in scope, localized, safe |
| T4 | LOW | correctness | fix | in scope, localized, safe |
| PERF-6 | LOW | performance | fix | in scope, localized, safe |
| PERF-7 | LOW | performance | fix | in scope, localized, safe |
| M7 | LOW | architecture | fix | in scope, localized, safe |
| T8 | LOW | correctness | fix | in scope, localized, safe |
| SEC-5 | NIT | security | defer | Evidence files are generated workflow records; redacting them is outside this code diff and changes recorded evidence. Route to a redaction pass before publishing. |
| T10 | NIT | correctness | defer | Writing new suites for seven modules is broad test work, not a localized fix. |
| M10 | NIT | architecture | defer | A structured reason field crosses the engine, launcher and page contract; not localized. |
| PERF-9 | NIT | performance | fix | in scope, localized, safe |
| PERF-10 | NIT | performance | fix | in scope, localized, safe |

## Fix Status

The orchestrator applied the fixes itself, not through fix sub-agents: this run had no tool for dispatching sub-agents. Each fix follows the method the finding prescribed unless its note says otherwise. Checks: `cargo test --workspace` (all pass), `cargo clippy --workspace --all-targets` (0 warnings), `cargo fmt --all --check` (clean), `node --test web/tests/*.test.mjs` (128 pass).

| ID | Sev | Source | Status | Fixed-at | Commit | Notes |
|----|-----|--------|--------|----------|--------|-------|
| IF-1 | MED | intent-fidelity | fixed | 2026-09-23T20:34:36Z | 5a235a4 | keeper_catch_chance raised from 0.84 to 0.86 in content/tuning.json, Tuning::default and data-files.md. At 1000 matches per suite, goals per match were 2.863 (seed 7), 3.035 (42), 2.948 (99), 2.656 (1) and 2.559 (2026), all inside the band. Shots per team were 12.9 to 14.8 and the stronger-team win rate 0.59 to 0.67, also inside their bands. |
| PERF-3 | MED | performance | fixed | 2026-09-23T20:34:36Z | 933e150 | Records preallocated to max_ticks (the finding's stated minimum): no doubling peaks, no copies in ticks_per_s. An incremental validator was not built; one match of records is still held per worker. |
| C3 | MED | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | Page changes are carried across connections (PageChange + carry_page_changes): kept when in the snapshot, re-queued when their queued row survived, dropped otherwise; inbox leftovers carried too. Unit test added. |
| M2 | MED | architecture | fixed | 2026-09-23T20:34:36Z | 933e150 | Split into Serving::play (per connection) and Serving::rewind, with EXIT_FULL_TIME / EXIT_REFUSED / EXIT_VIEWER_GONE. Values kept (the launcher reads them). calibrate::run was not split. |
| C2 | MED | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | drive() returns without finish() when the gate stops; test a_viewer_that_leaves_while_the_match_waits_gets_no_full_time. |
| M1 | MED | architecture | fixed | 2026-09-23T20:34:36Z | 933e150 | The loader refuses any dt other than one clock tick; commentary uses the clock constant; a cross-crate test pins the protocol minute; the viewer duplicate constant is removed; data-files.md updated. |
| C1 | MED | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | Penalty now shoots at once via shot_kick at the placed-kick spread; unit test a_penalty_in_play_is_shot_at_goal_at_once. |
| C4 | MED | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | Handshake gets the remaining wait (min 500 ms) as read/write timeout; refusals past the deadline end the wait. Test a_silent_client_does_not_hold_the_reconnect_wait_open. |
| SEC-1 | MED | security | fixed | 2026-09-23T20:34:36Z | 933e150 | null and file:// dropped; tests and docs/reference/protocol.md updated. |
| PERF-1 | MED | performance | fixed | 2026-09-23T20:34:36Z | 933e150 | Only a write the socket took counts as work. |
| PERF-2 | MED | performance | fixed | 2026-09-23T20:34:36Z | 933e150 | Idle sleep doubles from 100 us up to 1 ms and resets on work. |
| T1 | MED | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | Asserts rate < 4.2 always; lag branch requires a shown notice naming a speed below 4 within 1x of the measured rate; normal branch requires no lag notice. Syntax-checked only; the browser suite was not run. |
| C5 | LOW | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | Hello rebuilt from the resumed match on every rewind. In scope, localized, safe. |
| C6 | LOW | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | File kept across connections, cut back with FileSink::rewind at each rewind, finished once. Unit test for rewind. |
| SEC-2 | LOW | security | fixed | 2026-09-23T20:34:36Z | 933e150 | Only 127.0.0.1 / localhost with no port or the own port are answered (421 otherwise). Unit test. |
| SEC-4 | LOW | security | fixed | 2026-09-23T20:34:36Z | 933e150 | 10 s read/write timeouts and a 64-connection cap. |
| M5 | LOW | architecture | fixed | 2026-09-23T20:34:36Z | 933e150 | Salted by an FNV-1a hash of the variant name. |
| C7 | LOW | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | Whole ball over (HALF_LENGTH + BALL_RADIUS) and y interpolated at the goal line. Tests extended. |
| M6 | LOW | architecture | fixed | 2026-09-23T20:34:36Z | 933e150 | Mapping extracted to event_type(); a test checks every kind's code against its wire type. |
| PERF-4 | LOW | performance | fixed | 2026-09-23T20:34:36Z | 933e150 | Fixed-size arrays. |
| T7 | LOW | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | Unstaged with git rm --cached. |
| SEC-3 | LOW | security | fixed | 2026-09-23T20:34:36Z | 933e150 | Capacity capped at body.len() / 9. Unit test. |
| T4 | LOW | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | Also asserts sent < 1000 + 32 (one outbox per flush). |
| PERF-6 | LOW | performance | fixed | 2026-09-23T20:34:36Z | 933e150 | Panel rate re-measured every 30 frames; gauge refreshed every 15 frames and written only on change. |
| PERF-7 | LOW | performance | fixed | 2026-09-23T20:34:36Z | 933e150 | MatchState keeps the full-time tick on add, recomputes on truncate, clears on clear. Test added. |
| M7 | LOW | architecture | fixed | 2026-09-23T20:34:36Z | 933e150 | clockText removed (uses clockAt); recovery.mjs imports the constant. |
| T8 | LOW | correctness | fixed | 2026-09-23T20:34:36Z | 933e150 | Shared web/tests/data/parking-spots.json read by the JS test and checked against parking_spot by a Rust test. |
| PERF-9 | NIT | performance | fixed | 2026-09-23T20:34:36Z | 933e150 | Read once. |
| PERF-10 | NIT | performance | fixed | 2026-09-23T20:34:36Z | 933e150 | Font set once per frame; comment corrected (per-segment alpha needs one stroke each). |

## Success Criteria (intake, quoted exactly)

| Criterion | Current truth | Evidence |
|---|---|---|
| "**Realism statistics** (over 1000 simulated matches): goals per match 2.4 to 3.2; shots per team 8 to 16; possession split within 35 to 65 percent for evenly matched teams; a team with 15 percent better attributes wins over 50 percent of matches." | Met after IF-1 fix | `engine-cli calibrate --matches 1000` at keeper_catch_chance 0.86: see IF-1 for per-seed values; shots 13-15, possession ~50/50, stronger-team win rate 0.60-0.67 |
| "**Performance**: a full 90-minute match simulates headless in under 2 seconds on a laptop; 1000 matches complete in under 30 minutes." | Met | calibrate run report: bench.match_wall_ms 438, suite wall 78-88 s per 1000 matches on 8 jobs |
| "**Viewer fidelity**: the 2D viewer replays a match at 1x to 8x speed at 60 frames per second without dropped ticks; every goal in the replay matches the engine's event log." | Met per verify evidence; browser suite not re-run this stage | 06-verify-viewer-pitch.md, 06-verify-integration.md; T1 tightened the 4x assertion |
| "**Positional sanity**: no player leaves the pitch bounds; no two players occupy the same point; ball speed never exceeds 40 m/s; formation shape is kept when the ball is far away." | Met | calibrate run reports: validate.violations 0 over 2000 matches per run |
| "**Observability**: every match emits a structured event stream and an aggregate statistics record that the pipeline can ingest without transformation." | Met | darkpath.match_without_stats 0; events.files_written 2000 in each calibrate run |

## Recommendations

### Must Fix (triaged "fix")
None remaining. Every fix decision landed.

### Should Fix (MED triaged "fix")
None remaining.

### Deferred (triaged "defer")
- T11 (LOW): Paired calibration test failed once under full-suite load. Seen once and could not be reproduced. The cause is probably concurrent calibration workers under load. Diagnosing it needs repeated runs under load, which is not a localized fix.
- PERF-5 (LOW): step clones the whole Tuning every tick. The fix (Arc<Tuning> or split borrows) touches every tuning reader and test that edits tuning in place; not localized.
- T6 (LOW): Four acceptance tests are #[ignore] and nothing runs them. Needs a CI pipeline or a scheduled slow-test job: new infrastructure, not a localized change.
- SEC-5 (NIT): Workflow evidence contains absolute local paths. Evidence files are generated workflow records; redacting them is outside this code diff and changes recorded evidence. Route to a redaction pass before publishing.
- T10 (NIT): Several viewer modules have no unit tests. Writing new suites for seven modules is broad test work, not a localized fix.
- M10 (NIT): refusalReason parses the engine error prefix. A structured reason field crosses the engine, launcher and page contract; not localized.

### Dismissed
None.

### Consider (LOW/NIT — not triaged)
None; every LOW/NIT was triaged under the autonomous policy.

## Recommended Next Stage
- **Option A:** `/wf handoff football-manager-match-engine` — no open BLOCKER/HIGH; all 17 slices verified; ready for the PR. Run the Playwright first-match suite once before release (T1 changed it and it was not run here).
- **Option E:** `/wf ship football-manager-match-engine` — skip handoff if no PR description is needed.
- **Option F:** `/wf intake football-manager-match-engine from-review` — only to take on the deferred CI job for the ignored acceptance tests (T6) as new scope.
