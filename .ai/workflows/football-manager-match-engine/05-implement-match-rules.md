---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: match-rules
status: complete
stage-number: 5
created-at: "2026-09-22T20:41:00Z"
updated-at: "2026-09-22T20:41:00Z"
metric-files-changed: 70
metric-lines-added: 5048
metric-lines-removed: 319
metric-deviations-from-plan: 10
metric-review-fixes-applied: 0
commit-sha: "0611856b858dd9c975ae4d897423b2a1d521f2a7"
steering-honored:
  - "No visual change: the page gains only an event filter and a scrubber bound, so the --tl- tokens, the no-spinner rule, and the colour rule are untouched."
  - "Colour never carries a state alone: a card travels as the word in card.kind (yellow, second-yellow, red), never as a colour."
tags: [engine, laws, referee, snapshot, resume, protocol, milestone]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-match-rules.md
  plan: 04-plan-match-rules.md
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md]
  verify: 06-verify-match-rules.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine match-rules"
---

# Implement: Match Rules, Restarts, and Snapshots

## The Implementation

The engine that this slice inherited moved 22 players and a ball, but no law applied. The ball bounced off invisible walls, a tackle always succeeded or failed cleanly, and a match ended at exactly 270,000 ticks. This slice adds a referee inside the loop. The referee owns the phase of play (live, dead ball, full time) and enforces the ball out of play, offside, fouls with advantage, three kinds of card, six restarts, half-time, added time, and abandonment. It announces every stoppage through one hook, and a snapshot of the whole match is written at every stoppage.

Four decisions carry the work. First, one referee draw decides each tackle as a win, a foul with the ball lost, a foul with the ball kept, or a miss, so a test scripts one number to get one outcome. Second, the snapshot holds every field of the simulation, the generator's word position included, so a resumed match continues tick for tick: the continuation test compares every tick record after the 30th stoppage, byte for byte, and they match. Third, `ticks_expected` now means the most ticks a match can last, which is a change of meaning, so the protocol goes to version 2. Fourth, the removal of the wall bounce exposed a defect that the bounce had hidden: a lofted pass used the ground-roll speed and overshot its target. The first full match had 350 throw-ins and 22 minutes of dead ball. A new `kick_speed` gives a lofted pass the speed that lands it and rolls it to its target, and throw-ins fell to about 2 per match.

This slice unblocks tactics, commentary, and crash recovery: tactics applies queued changes through the stoppage hook, commentary reads the new events and `player.id`, and recovery builds on `resume`. The top open risk is the processor budget. The engine now costs 1.47 to 1.51 µs per tick against a limit of 1.566 µs, a margin of about 4 percent, and the next slice adds decisions on top.

## Summary of Changes

- The rule pack is version 2. It prices each stoppage kind in seconds of added time, and it sets `min_players` 7. The attribute schema requires `aggression`.
- Six law modules under `crates/engine/src/rules/`: five pure modules (clock, fouls, offside, restart, discipline) and the referee in `mod.rs`.
- The simulation loop runs the referee after the ball moves and possession resolves. The wall bounce is gone. The ball leaves the pitch.
- A sent-off player parks beside the pitch at a fixed spot. The team reshapes the player's line across the gap.
- Players walk to restart positions during a dead ball. The taker restarts play when every opponent stands back, or at three times the delay.
- The snapshot is a binary file with a SHA-256 trailer, written atomically at every stoppage to `SM_DATA_DIR/matches/<match.id>/snapshot.smsn`. `engine-cli resume` continues a match from it.
- A test-only scene builder (the `scenario` feature) arranges players, the ball, cards, the tally, and the referee's next draws.
- The protocol is version 2: 13 event types, a `card.kind` word, and six optional event fields. The page counts only events that stop play as stoppage marks, and it stops the scrubber at the last tick.
- `match-stats` carries the law counts. `run-report` carries `bench.ticks_per_match` and `bench.cpu_us_per_tick`.

## Files Changed

- `content/rules/default.json`: schema 2, `added_time` per kind with card, variance, minimum, and cap, and `min_players` 7.
- `content/tuning.json`: foul, card, restart-delay, and ready-radius constants.
- `content/README.md`: the new rule and tuning fields with units, defaults, and bounds, the aggression requirement, and the added-time formula.
- `README.md`: the laws, the snapshot location, `resume`, `?v=2`, and the variable match length.
- `Cargo.lock`: the engine's self dev-dependency.
- `crates/engine/Cargo.toml`: the `scenario` feature and the self dev-dependency that turns it on for tests.
- `crates/engine/src/data/attributes.rs`: `aggression` required; seven required names.
- `crates/engine/src/data/rules.rs`: `RULES_VERSION` 2, `AddedTime` with its checks, `min_players`, `regulation_minutes`, ordered `StoppageKind`.
- `crates/engine/src/data/mod.rs`: re-exports `AddedTime`; loader tests at version 2.
- `crates/engine/src/data/tuning.rs`: comments no longer name a later work item.
- `crates/engine/src/player.rs`: `Derived.aggression`, `Status`, `yellow`, `active()`.
- `crates/engine/src/team.rs`: `active` slots, `reshape`, `switch_ends`.
- `crates/engine/src/tuning.rs`: the new constants and `RestartDelays`.
- `crates/engine/src/rng.rs`: `RngState` save and restore, `referee_draw`, scripted draws behind `scenario`.
- `crates/engine/src/pitch.rs`: law constants, line exits, every restart spot, the penalty area, parking spots.
- `crates/engine/src/rules/clock.rs` (new): the match clock, the added-time formula, `max_ticks`.
- `crates/engine/src/rules/fouls.rs` (new): the three-way tackle, the foul chance, the card outcome.
- `crates/engine/src/rules/offside.rs` (new): the offside set at a kick and the offence at a touch.
- `crates/engine/src/rules/restart.rs` (new): the dead ball, delays, takers, restart targets, readiness.
- `crates/engine/src/rules/discipline.rs` (new): booking, sending off, on-pitch counts, abandonment.
- `crates/engine/src/rules/mod.rs` (new): the referee state machine and its `Simulation` methods.
- `crates/engine/src/sim.rs`: the referee in the loop, the 12 event kinds with player, card, minute, and spot, the law counters, `run` to full time, `minute()`.
- `crates/engine/src/decision.rs`: inactive players skipped, `restart_pass`, `kick_speed` for lofted passes and the keeper's clearance.
- `crates/engine/src/steering.rs`: inactive players stand still and are skipped in separation and overlaps.
- `crates/engine/src/record.rs`: tick-file schema 4, `ticks_expected` as a maximum, the `on_stoppage` hook forwarded by `FanoutSink` and `Option`.
- `crates/engine/src/validate.rs`: the team timeline across half-time and send-offs, parking exemptions, the `restart_spot` rule.
- `crates/engine/src/error.rs`: `EngineError::Snapshot` naming the path and the reason.
- `crates/engine/src/snapshot.rs` (new): the file format, capture, restore, refusals, `SnapshotSink`.
- `crates/engine/src/scenario.rs` (new): the test scene builder.
- `crates/engine/src/lib.rs`: the new modules and re-exports.
- `crates/engine/src/observe/mod.rs`: `LawStats` in `match-stats`; two benchmark keys in `run-report`.
- `crates/engine/benches/tick_step.rs`: rebuilds a warmed match when the match ends.
- `crates/engine/tests/common/mod.rs`: `short_match`, `quiet_match`, `index`, `spread`.
- `crates/engine/tests/content.rs`, `determinism.rs`, `identity.rs`, `validator.rs`, `full_match.rs`: the new rule pack, `run` to full time, the timeline validator, and the full-match law bands.
- `crates/engine/tests/rules_offside.rs`, `rules_fouls.rs`, `rules_cards.rs`, `rules_restarts.rs`, `rules_clock.rs`, `snapshot.rs` (new): the criterion tests.
- `crates/protocol/src/event.rs`: 13 event types, `CardKind`, six optional fields and their builders.
- `crates/protocol/src/lib.rs`: `PROTOCOL_VERSION` 2 with its reason; the event field list.
- `crates/protocol/src/command.rs`, `message.rs`: comments; `ticks_expected` documented as a maximum.
- `docs/reference/protocol.md`: version 2, the new event types and fields, `ticks_expected` as a maximum.
- `crates/stream/src/server.rs`: the version-refusal test at version 2.
- `crates/stream/src/record.rs`: a comment no longer names a later work item.
- `crates/stream/tests/common/mod.rs`, `fixture.rs`, `throughput.rs`: every event kind mapped, the loop ends at full time, the tick count is regulation plus added time.
- `crates/engine-cli/src/stream_run.rs`: every event mapped with player ids, card, advantage, minute, and added time; the stoppage hook forwarded; `Driven { written, full_time }`.
- `crates/engine-cli/src/simulate.rs`: `max_ticks` in the header, the snapshot sink, `--no-snapshot`, the timeline validator, the law counts.
- `crates/engine-cli/src/serve.rs`, `record.rs`: `max_ticks`, the snapshot sink on `serve`, exit 0 at full time.
- `crates/engine-cli/src/bench.rs`: ticks per match and processor time per tick; no snapshot sink.
- `crates/engine-cli/src/cli.rs`, `main.rs`: `resume` and `--no-snapshot`.
- `crates/engine-cli/src/resume.rs` (new): the resume command.
- `crates/engine-cli/tests/resume.rs` (new): resume reaches the same full time; damaged files are refused.
- `crates/engine-cli/tests/cli_args.rs`, `stream_cli.rs`, `web_cli.rs`: tick count with added time, 3,006 frames for a two-half minute, protocol version 2.
- `web/stoppages.mjs`: `STOPS_PLAY` and `stopsPlay`.
- `web/main.mjs`: only events that stop play become marks; the scrubber stops at the last tick at full time.
- `web/tests/stoppages.test.mjs`, `web/tests/decode.test.mjs`: the filter, and the fixture at version 2.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs`, `record.rs`, `validate.rs`, `decision.rs`, `steering.rs`: built by the engine core; the referee changes the loop order and the tick-file schema.
- `crates/engine/src/data/*`, `content/*`: built by the data slice; the rule pack and the attribute schema change version or content.
- `crates/protocol/*`, `crates/stream/*`, `docs/reference/protocol.md`: built by the stream slice; the protocol goes to version 2.
- `crates/engine-cli/src/*`: shared by every slice.
- `web/main.mjs`, `web/stoppages.mjs`: built by the viewer slice; the stoppage index now filters events.

## Notes on Design Choices

- The laws are pure functions with unit tests. The referee in `rules/mod.rs` is the only code that changes match state for a law.
- One draw per tackle, split into bands, keeps a scene reproducible with one scripted number. The card draw is a second number.
- Scripted draws replace only `referee_draw`, not every draw. Decision noise draws every tick and would empty the scripted queue before the tackle.
- Advantage applies only outside the offender's penalty area. A card held for advantage is shown at the next stoppage, before the restart.
- The offside set is a bitmask fixed at a kick in open play and judged at the next first touch. A throw-in, a corner, and a goal kick clear the bitmask.
- A sent-off player parks at (±(10 + slot), −37), outside the pitch, so the tick record keeps 22 positions and the validator exempts parking spots.
- The snapshot header holds the build hash, the content hash, the owner, and the match time. The body holds every simulation field except the scratch buffer and the drained events. A refusal names its reason from the reader's own checks and never prints file bytes.
- `SnapshotSink` logs a failed write and lets the match continue, because a lost snapshot must not end a match.
- `resume` keeps the original match id, so a resumed match overwrites the same `stats.json` and snapshot.

## Verification Seams Built

- AC-a to AC-e (every law) → the scene builder `Scene` at `crates/engine/src/scenario.rs:13`, with scripted referee draws at `crates/engine/src/scenario.rs:70` and `crates/engine/src/rng.rs:70`, consumed by `referee_draw` at `crates/engine/src/rng.rs:60` (enables `cargo test` to arrange one law situation and get one outcome).
- AC-a to AC-e → `quiet_match` at `crates/engine/tests/common/mod.rs:72` turns off player decisions, and `spread` at `crates/engine/tests/common/mod.rs:86` places both sides far from the scene (enables a scene that moves only as the test and the referee move it).
- AC-d → the validator rule `restart_spot` with `RESTART_SPOT_TOLERANCE` 0.5 m at `crates/engine/src/validate.rs:20`, fed by the event spots through `Validator::for_match` at `crates/engine/src/validate.rs:52` (enables `simulate` to report every misplaced restart in `validate.violations`).
- AC-f → `Snapshot::capture` at `crates/engine/src/snapshot.rs:65` and `Simulation::from_snapshot` at `crates/engine/src/snapshot.rs:213`, with the generator state at `crates/engine/src/rng.rs:42` and `crates/engine/src/rng.rs:51`, called through the stoppage hook at `crates/engine/src/sim.rs:336` (enables a byte-for-byte comparison of resumed and uninterrupted tick records).
- AC-f → the `match.resumed` log line at `crates/engine-cli/src/resume.rs:79`, with score, half, on-pitch counts, and cards (enables the command-line test and a person to see where a match picked up).
- AC-g → the ordered refusals in `Snapshot::from_bytes` at `crates/engine/src/snapshot.rs:120` and the exit code 1 with `snapshot.refused` at `crates/engine-cli/src/resume.rs:65` (enables a test to damage a file and read the reason on stderr).
- AC-h → `bench.cpu_us_per_tick` at `crates/engine/src/observe/mod.rs:180` (enables the benchmark gate to be judged per tick, because added time makes the match length vary).
- Page → `stopsPlay` at `web/stoppages.mjs:30` and the full-time scrubber bound at `web/main.mjs:128` (enables `node --test` to check the stoppage filter with no browser).
- Operations → `rules.added_time` at `crates/engine/src/rules/mod.rs:407`, `rules.abandoned` at `crates/engine/src/rules/mod.rs:260`, and `rules.dead_ball_stalled` at `crates/engine/src/rules/mod.rs:349`.

## Deviations from Plan

1. `crates/engine/src/decision.rs` gained `kick_speed` for lofted passes and a longer, lofted goalkeeper clearance. The plan did not name this change. The wall bounce had hidden an overshoot: a lofted pass used the ground-roll speed formula. With the bounce removed, the first full match had 350 throw-ins and 22 minutes of dead ball. After the fix, a full match has about 2 throw-ins, and the test `a_lofted_pass_reaches_its_target_slow_enough_to_control` holds the fix.
2. The full-match band in `crates/engine/tests/full_match.rs` asserts fouls (4 to 20 per team), free kicks, out-of-play restarts, half-time, full time, and the stoppage count. Offsides and corners are counted and not banded. Open play has no forward runs and no deflections, so seed 42 gives 0 offsides and 0 corners. The criterion tests prove both laws with scenes.
3. The continuation test resumes at the 30th stoppage, not the 40th. The seed-42 match has 34 stoppages.
4. Scripted draws apply to `referee_draw` only, not to every draw (see the design notes).
5. `SnapshotSink` lives in `crates/engine/src/snapshot.rs`, not in the command-line crate, so `simulate`, `serve`, and `resume` share one sink.
6. The `rules.abandoned` warning is emitted in `crates/engine/src/rules/mod.rs`, where the referee ends the match, not in the command line.
7. `resume` also accepts `--team-a` and `--team-b`, so a match started with other clubs resumes with the same files; the digests refuse any other file.
8. The match driver returns `Driven { written, full_time }` and `Simulation` gained `minute()`. `serve` and `record` exit 0 when the referee ended the match, because the tick count no longer fixes the end.
9. Files beyond the plan: `crates/engine/benches/tick_step.rs`, `crates/engine/tests/determinism.rs`, `identity.rs`, `common/mod.rs`, `crates/stream/src/server.rs` (test), `crates/stream/tests/common/mod.rs`, `fixture.rs`, `throughput.rs`, `crates/engine-cli/tests/cli_args.rs`, `stream_cli.rs`, `web_cli.rs`, and `web/tests/decode.test.mjs`. Each asserted a fixed length of 270,000 ticks, protocol version 1, or three event kinds.
10. Three older comments named later work items (`crates/protocol/src/lib.rs`, `crates/engine/src/data/tuning.rs`, `crates/stream/src/record.rs`). They are rewritten without workflow vocabulary.

## Anything Deferred

- Queued tactical changes and substitutions are not applied. The stoppage hook announces every stoppage; the tactics slice applies the queue through it. `admits_tactics`, `admits_substitution`, and `substitutions` are validated only.
- The `injury` stoppage kind is priced in the rule pack and never produced. Injuries have no model yet.
- Forward runs and deflections do not exist, so offsides and corners are rare in open play. Calibration tunes the rates when tactics adds forward runs.
- Extra time and penalty shoot-outs extend `MatchClock` in their own slice.
- A paced live mode for `serve` is still absent; the viewer slice owns that question.

## Known Risks / Caveats

- The processor budget margin is small. Five benchmark runs gave 1.4719 to 1.5053 µs per tick against the 1.566 µs limit, and the median match is 276,100 ticks. Peak memory is 5.45 MB against 6.69 MB.
- A snapshot resumes only on the build that wrote it. A development build with uncommitted changes has a `-dirty` build hash, so any rebuild refuses every earlier snapshot. This is the intended guard; it is noted because it surprises during development.
- `foul_base` 0.1 was tuned on seeds 42, 1, 2, and 3 (5 to 15 fouls per team). Calibration owns the final value.
- A restart that stalls is taken at three times its delay whatever the players are doing, with a `rules.dead_ball_stalled` warning. The free-kick test shows no stall in the crowded scene.

## Freshness Research

- `rand_chacha` 0.10, installed source `rand_chacha-0.10.0/src/chacha.rs:145-198`: `get_seed`, `get_stream`, `get_word_pos`, and `set_word_pos` restore a generator at an exact word, so the snapshot stores three values instead of a draw count.
- Rust standard library, `std::fs::rename` (https://doc.rust-lang.org/std/fs/fn.rename.html): on Windows 10 1607 and later, a rename replaces an existing file, so the snapshot write is atomic with no remove step.
- `garde`, installed source: a validation report lists errors in field-path order, so a range test must hold a complete `per_kind` map to reach the field under test.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine match-rules`. The slice changes testable behavior on every surface: the engine, the protocol, the command line, and the page. Consider compacting the session before `/wf verify`; workflow state lives in artifact files on disk and the SessionStart hook re-reads it after compaction.
- **Option C:** `/wf plan football-manager-match-engine match-rules` only if verify shows the relaxed full-match band hides a law defect.
