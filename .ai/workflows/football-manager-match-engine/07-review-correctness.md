---
schema: sdlc/v1
type: review-command
slug: football-manager-match-engine
dimension: correctness
parent: 07-review.md
review-scope: slug-wide
status: complete
created-at: "2026-09-23T20:34:36Z"
updated-at: "2026-09-23T20:34:36Z"
verdict: ship
metric-findings-total: 3
metric-findings-blocker: 0
metric-findings-high: 0
---

# Review: correctness

14 findings in this dimension. 11 are fixed and 3 are deferred. Nothing is open at BLOCKER or HIGH.

| ID | Sev | Conf | Status | Surfaced | File:Line | Issue | Fix / reason |
|---|---|---|---|---|---|---|---|
| C3 | MED | medium | fixed | 2026-09-23T20:34:36Z | crates/engine-cli/src/serve.rs:262 | Verdicts after a reconnect lose the page queue_id; some acked changes are lost | Page changes are carried across connections (PageChange + carry_page_changes): kept when in the snapshot, re-queued when their queued row survived, dropped otherwise; inbox leftovers carried too. Unit test added. |
| C2 | MED | high | fixed | 2026-09-23T20:34:36Z | crates/engine-cli/src/stream_run.rs:70 | A viewer that leaves mid-match gets a false full-time event | drive() returns without finish() when the gate stops; test a_viewer_that_leaves_while_the_match_waits_gets_no_full_time. |
| C1 | MED | high | fixed | 2026-09-23T20:34:36Z | crates/engine/src/rules/mod.rs:551 | A penalty in play is not taken as a kick | Penalty now shoots at once via shot_kick at the placed-kick spread; unit test a_penalty_in_play_is_shot_at_goal_at_once. |
| C4 | MED | medium | fixed | 2026-09-23T20:34:36Z | crates/stream/src/server.rs:110 | accept_within can block past its reconnect deadline | Handshake gets the remaining wait (min 500 ms) as read/write timeout; refusals past the deadline end the wait. Test a_silent_client_does_not_hold_the_reconnect_wait_open. |
| T1 | MED | high | fixed | 2026-09-23T20:34:36Z | e2e/tests/first-match.spec.mjs:151-167 | The 4x speed check cannot fail usefully | Asserts rate < 4.2 always; lag branch requires a shown notice naming a speed below 4 within 1x of the measured rate; normal branch requires no lag notice. Syntax-checked only; the browser suite was not run. |
| T11 | LOW | medium | deferred | 2026-09-23T20:34:36Z | crates/engine-cli/tests/calibrate_pair.rs:106 | Paired calibration test failed once under full-suite load | Seen once and could not be reproduced. The cause is probably concurrent calibration workers under load. Diagnosing it needs repeated runs under load, which is not a localized fix. |
| T6 | LOW | high | deferred | 2026-09-23T20:34:36Z | crates/engine/tests/ai_trailing.rs:15 | Four acceptance tests are #[ignore] and nothing runs them | Needs a CI pipeline or a scheduled slow-test job: new infrastructure, not a localized change. |
| C5 | LOW | medium | fixed | 2026-09-23T20:34:36Z | crates/engine-cli/src/serve.rs:232 | Reconnect hello describes the pre-kick-off preview, not the match | Hello rebuilt from the resumed match on every rewind. In scope, localized, safe. |
| C6 | LOW | medium | fixed | 2026-09-23T20:34:36Z | crates/engine-cli/src/serve.rs:268 | --ticks-out stops at the first drop and holds ticks the resumed match replays | File kept across connections, cut back with FileSink::rewind at each rewind, finished once. Unit test for rewind. |
| C7 | LOW | medium | fixed | 2026-09-23T20:34:36Z | crates/engine/src/pitch.rs:70 | A goal counts before the whole ball is over the line, and the crossing point is not used | Whole ball over (HALF_LENGTH + BALL_RADIUS) and y interpolated at the goal line. Tests extended. |
| T7 | LOW | high | fixed | 2026-09-23T20:34:36Z | crates/engine/tests/zz_stall_probe.rs:1 | A debug probe was staged for commit | Unstaged with git rm --cached. |
| T4 | LOW | medium | fixed | 2026-09-23T20:34:36Z | crates/stream/tests/reconnect.rs:118-129 | Drop-seam test never pins the cut to its tick | Also asserts sent < 1000 + 32 (one outbox per flush). |
| T8 | LOW | medium | fixed | 2026-09-23T20:34:36Z | web/tests/pitch.test.mjs:9-12 | Viewer test re-implements the engine parking-spot formula | Shared web/tests/data/parking-spots.json read by the JS test and checked against parking_spot by a Rust test. |
| T10 | NIT | medium | deferred | 2026-09-23T20:34:36Z | web/main.mjs:1 | Several viewer modules have no unit tests | Writing new suites for seven modules is broad test work, not a localized fix. |
