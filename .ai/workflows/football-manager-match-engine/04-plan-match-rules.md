---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: match-rules
status: complete
stage-number: 4
created-at: "2026-09-22T19:29:33Z"
updated-at: "2026-09-22T19:29:33Z"
metric-files-to-touch: 54
metric-step-count: 23
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, rules, set-pieces, snapshot, protocol]
stack-source: confirmed
steering-honored:
  - "no visual change to the page; the one page change is stoppage-index logic, so the design direction in steer.md is untouched"
  - "a card reaches the event stream as the word in card.kind, so colour never carries the card state alone"
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-match-rules.md
  shape: 02-shape.md
  instrument: 04b-instrument.md
  benchmark: 05c-benchmark.md
  observability: ../../observability.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md]
  implement: 05-implement-match-rules.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine match-rules"
---

# Plan: Match Rules and Stoppage Snapshots

## The Plan

The engine plays a kick-about with no referee. The ball bounces off the touchlines and goal lines like walls (`crates/engine/src/sim.rs:272-281`), a tackle only decides possession, the rule pack loads and is never read, and no match state can be saved. Four research passes found every seam this slice needs: a restart flag the codec already turns into a restart keyframe, a stoppage-kind enumeration with nine kinds and no consumer, an `aggression` attribute in the shipped schema that nothing reads, and a seeded generator whose exact position can be saved through `get_word_pos` and `set_word_pos`. The tree is green at 132 Rust tests and 43 page tests, and the benchmark baseline on `d86996a` is 388 milliseconds and 384.4 milliseconds of processor time per match.

Thirteen product-owner answers fixed the build. This slice announces stoppages through one hook and applies no queued change. The engine announces a maximum match length, because added time now makes a half longer than 45 minutes. Players walk to restart positions during a dead ball, and the referee plays advantage when the fouled team keeps the ball. Every tackle rolls three ways from one draw, with `aggression` now required. Added time is seconds per stoppage kind plus a seeded variance, clamped between a minimum and a cap. A card is one `card` event with a `card.kind` word. The snapshot is a binary file with a SHA-256 trailer, and a resumed match continues tick for tick. A scenario builder behind a test-only feature builds the criterion scenes. The page filters its stoppage index to events that stop play. Because added time makes a match about 9 percent longer, the benchmark gate is judged per tick.

Implement touches 54 files, 16 new, in 23 steps: content first, then the pure law modules, then the referee inside the loop, then the snapshot, then the command line, the protocol, and the page. Tactics, commentary, and crash recovery build on the stoppage hook and the new events. The top risk is exact continuation: one field missing from the snapshot makes a resumed match drift, and only the byte-for-byte test catches it.

## Current State

- Branch `feat/football-manager-match-engine` at `d86996a`; four slices implemented and verified. `cargo test --workspace` passes 132 tests; `node --test web/tests/*.test.mjs` passes 43. Clippy with `-D warnings` and `cargo fmt --check` are clean. On Node 24 the bare `node --test web/tests/` form fails with `MODULE_NOT_FOUND`; the glob form works.
- `Simulation` (`sim.rs:106-120`) holds config, teams, players, ball, generator, tick, carrier, `control_since`, `last_touch`, summary, a scratch buffer, the restart flag, and an event list. No half, clock, card, or referee state exists. `run(ticks, sink)` (`sim.rs:171-178`) runs a fixed tick count.
- The only restart is `kick_off` (`sim.rs:295-319`), which resets every player to the formation base and sets `restart` for one tick. The only tackle is the possession roll `p_win = 0.05 * tackle / (tackle + dribble)` at `sim.rs:362-365`.
- `RulePack` (`data/rules.rs:78-91`) carries `halves`, `half_minutes`, `substitutions`, and nine stoppage kinds with admission flags. It has no added-time formula and no minimum team size. `content/README.md:125` states that the engine does not enforce the pack.
- The engine requires six attributes (`data/attributes.rs:14-21`). `aggression` is in the shipped mental group (`content/README.md:29`) and nothing reads it.
- The restart flag rides in the high bit of the tick field (`record.rs:29`). `FrameSink::push` (`crates/stream/src/session.rs:196-219`) turns a restart tick into a `KIND_RESTART` keyframe, and the validator exempts restart ticks from the jump check (`validate.rs:73-85`). Every new restart kind needs only the flag.
- `EventType` (`crates/protocol/src/event.rs:16-21`) has kick-off, goal, full-time, and tactics-change. `MatchEvent` has no `player.id`. The observability contract reserves `player.id`, `player.secondary_id`, `snapshot.tick`, `stats.fouls`, `stats.corners`, `stats.offsides`, and `rules.pack_version`, and names no card colour key.
- `hello.ticks_expected` sizes the page's rewind history (`web/main.mjs:66`) and the scrubber (`web/main.mjs:73`). The page adds every event message to the stoppage index (`web/main.mjs:122-126`), including `tactics-change`.
- No test outside the engine crate can place a player: `Simulation::new` always calls `kick_off(0)` and the fields are crate-private.
- `rand_chacha` 0.10 is built without its `serde` feature (`Cargo.lock:744-750`), and it exposes `get_seed`, `get_stream`, `get_word_pos`, `set_stream`, and `set_word_pos` (`rand_chacha-0.10.0/src/chacha.rs:134-198` in the local cargo registry).
- Seven code comments name workflow slices: `sim.rs:273`, `data/rules.rs:2`, `command.rs:1-2`, `command.rs:55`, `event.rs:3`, `event.rs:12`, `message.rs:90`.
- Benchmark baseline on `d86996a` (three drives): wall 390, 388, 388 ms; processor time per match 387.6, 384.4, 384.4 ms; peak memory 5.355, 5.328, 5.363 MB; tick step 1.4336 µs. Evidence under `bench-baseline/match-rules/`.

## Simplicity Ladder

| Capability | Rung | Choice |
|---|---|---|
| Offside detection | rung 4 new code | Coordinate comparisons along `attack_x` at a pass, a 22-bit set cached until the next touch; no in-repo symbol compares against the second-last defender. Geometry uses `DVec2` and `pitch.rs`. |
| Three-way tackle roll | rung 3 reuse with modification | `sim.rs:362-365` `resolve_possession` and `EngineRng::next_f64`: one draw split into win, foul, and miss bands; extracted into `rules/fouls.rs` as a pure function. |
| Card logic | rung 4 new code | No card type exists. Copies the enum-plus-`ALL`-plus-`code()` idiom of `StoppageKind` (`data/rules.rs:11-53`) and `ChangeKind` (`command.rs:8-53`). |
| Restart placement and dead ball | rung 3 reuse with modification | `sim.rs:295-319` `kick_off` generalised into a restart with a kind, a spot, and a taker; steering (`steering::step_all`) moves players to restart targets. Spot geometry is rung 4 in `pitch.rs`. |
| Stoppage-time accumulation | rung 4 new code | A per-kind tally and a clamp; `EngineRng::next_f64` gives the variance. No reusable accumulator exists. |
| Half structure and side switch | rung 4 new code | `Team::attack_x` (`team.rs:50-60`) is fixed at construction; `switch_ends()` flips it. |
| Snapshot layout | rung 3 reuse with modification | The magic, version, header, body, and trailer pattern of `record.rs:108-145` and `record.rs:280-307`, with the SHA-256 digest idea of `crates/stream/src/record.rs` (`SMFX` and `SMFE`). |
| Snapshot checksum | rung 3 reuse | `sha2` 0.10, already a workspace dependency. The standard library has no stable data-integrity hash (`DefaultHasher` is not stable across builds). |
| Atomic snapshot replace | rung 1 stdlib | `std::fs::rename` over an existing file replaces it on Windows 10 1607 and later ([std::fs::rename](https://doc.rust-lang.org/std/fs/fn.rename.html)). |
| Generator state | rung 3 reuse | `ChaCha8Rng::get_seed`, `get_stream`, `get_word_pos`, `set_stream`, `set_word_pos`; no `serde` feature needed. |
| Scenario builder | rung 4 new code | A Cargo feature and a self dev-dependency; no test-support crate is needed for one builder. |
| Resume command | rung 3 reuse | `crates/engine-cli/src/simulate.rs` structure and the clap derive pattern in `cli.rs`. |
| Page filter | rung 3 reuse with modification | `web/stoppages.mjs` gains one exported predicate. |

## Applied Learnings

No applicable learnings found: `.ai/solutions/INDEX.md` does not exist and `.ai/sdlc-config.json` sets no global folder. `runtime-evidence-deferrals` is empty, so the repeat-deferral tripwire does not fire.

## Likely Files / Areas to Touch

- `content/rules/default.json`, `content/tuning.json`, `content/README.md`: rule pack version 2, the new tuning fields, the modder reference.
- `crates/engine/Cargo.toml`, `src/lib.rs`, `src/error.rs`: the `scenario` feature, the new modules, `EngineError::Snapshot`.
- `crates/engine/src/data/attributes.rs`, `data/rules.rs`: `aggression` required; `AddedTime` and `min_players`.
- `crates/engine/src/player.rs`, `team.rs`, `pitch.rs`, `rng.rs`, `tuning.rs`: status and cards, reshape and side switch, law geometry, generator state, foul and dead-ball tuning.
- `crates/engine/src/rules/mod.rs`, `clock.rs`, `offside.rs`, `fouls.rs`, `restart.rs`, `discipline.rs` (new): the referee.
- `crates/engine/src/sim.rs`, `decision.rs`, `validate.rs`, `record.rs`: the loop, the dead-ball decisions, the validator, tick schema 4.
- `crates/engine/src/snapshot.rs`, `scenario.rs` (new): the snapshot file and the test-only builder.
- `crates/engine/src/observe/mod.rs`: statistics and benchmark fields.
- `crates/engine/tests/`: six new criterion files; `common/mod.rs`, `content.rs`, `full_match.rs`, `validator.rs` updated.
- `crates/protocol/src/event.rs`, `lib.rs`, `command.rs`, `message.rs`, and `docs/reference/protocol.md`: event types, fields, version 2, comments.
- `crates/engine-cli/src/cli.rs`, `main.rs`, `resume.rs` (new), `simulate.rs`, `serve.rs`, `stream_run.rs`, `record.rs`, `bench.rs`, `tests/resume.rs` (new): the command line.
- `web/stoppages.mjs`, `web/main.mjs`, `web/tests/stoppages.test.mjs`: the stoppage filter and the scrubber clamp.
- `README.md`: laws, snapshots, and `resume`.

Full topology with line estimates: `04-plan-match-rules.yaml`.

## Proposed Change Strategy

Build the laws as pure functions first and prove each one with a unit test, then put one referee state machine inside the loop. The **referee** (named mechanism) owns the phase: live play, a dead ball with its kind, team, spot, and ready time, or full time. The loop asks the referee after the ball moves and after possession resolves. When the whole ball crosses a line, or a foul or offside is called, the referee opens a dead ball, sets the restart flag on the tick the ball is placed, and calls the **stoppage hook** (named mechanism): `TickSink::on_stoppage(&Stoppage, &Simulation)`, a default no-op. The command line writes a snapshot through that hook, and the next slice applies queued changes through it. This slice applies no queued change (Round 1 Q1).

A dead ball suspends decisions. Every agent steers to its restart target; the taker kicks when the tuned delay has passed and the taker is within the ready radius. Opening and second-half kick-offs place players at once, because a break comes before them. Offside is computed only when a team-mate plays the ball and is judged at the next first touch, which keeps the per-tick cost at two comparisons. Fouls come from one draw split into three bands, so the number of draws per tackle stays one.

Added time is a per-half tally of seconds by stoppage kind plus a seeded variance, clamped by the rule pack (Round 2 Q7). A full-length match ends at regulation plus added time; a shortened test match plays none. The announced length is the **maximum** (Round 1 Q2): regulation plus both caps, so `ticks_expected` and `expected_ticks` change meaning. That change raises `PROTOCOL_VERSION` to 2 and the tick-file schema to 4 under the rule at `crates/protocol/src/lib.rs:21-28`.

The **snapshot** (named mechanism, shape "Snapshot at every stoppage") is a binary file with a SHA-256 trailer (Round 3 Q10), written atomically as `SM_DATA_DIR/matches/<match.id>/snapshot.smsn` at every stoppage. It holds every field of `Simulation` except the scratch buffer and the drained event list, including the generator's seed, stream, and word position, so a resumed match continues tick for tick (Round 3 Q9). A snapshot from another build or other content is refused with a named reason (NFR-4).

Performance: NFR-1 governs the mechanism choices and `yields-to: C2` (PO-ratified). The benchmark criterion is judged per tick (Round 4 Q13); the 2-second per-match budget stays per match.

## Step-by-Step Plan

1. **Content schema.** In `data/attributes.rs`, add `aggression` to `REQUIRED`. In `player.rs`, add `Derived::aggression` scaled to 0..1, `Player::status` (`OnPitch`, `SentOff`), and `Player::yellow`. In `data/rules.rs`, set `RULES_VERSION` to 2 and add `AddedTime { per_kind, card_s, variance_s, min_s, max_s }` and `min_players` with garde ranges; keep `every_kind_once`; rewrite the module comment in product language. Update `content/rules/default.json` with the values in `04-plan-match-rules.yaml`. Update `crates/engine/tests/content.rs` (version 2 in the refusal text, the new pins, a schema without `aggression` refused naming it). Run `cargo test -p engine --test content`. All tests must pass.
2. **Tuning.** In `tuning.rs`, add `foul_base`, `foul_aggression_weight`, `foul_tackling_weight`, `foul_ball_loss`, `yellow_base`, `yellow_aggression_weight`, `red_base`, `restart_delay_s` (throw-in 3, corner 8, goal kick 6, free kick 8, penalty 15, kick-off 5), and `restart_ready_radius` 1.0, each with a garde range. Set `foul_base` so a default match gives about 10 fouls per team. Add the same values to `content/tuning.json`. The pinning test `content.tuning.engine == Tuning::default()` must pass.
3. **Generator state.** In `rng.rs`, add `RngState { seed, stream, word_pos }`, `EngineRng::state()`, and `EngineRng::from_state()`. Under the `scenario` feature, add a scripted queue of draws that `next_f64` consumes before the stream. Unit test: 500 draws, save the state, 500 more; a restored generator gives the same second 500.
4. **Law geometry.** In `pitch.rs`, add the law constants, `exit(prev, ball) -> Option<Exit>` using the ball radius, the restart spot functions (throw-in point on the line, goal-area point on the exit side, corner-arc point, penalty mark), `in_penalty_area`, and `parking_spot(team, slot)` beside the pitch. Unit tests for each spot and for a ball on the line (not out) and fully over it (out).
5. **Clock.** Write `rules/clock.rs`: `MatchClock`, `added_seconds(tally, rules, draw)` equal to the clamp of the tally plus the variance term, the announced minute rounded up, `minute()` and `minute_added()`, and `max_ticks(config, rules)`. A match shorter than the rule pack's regulation length plays no added time. Unit tests: the formula, the clamp at both ends, and `max_ticks` for 90 minutes (regulation 270,000 plus 2 x 900 s = 360,000 ticks).
6. **Fouls and cards.** Write `rules/fouls.rs`: `tackle_outcome(p_win, p_foul, draw)`, `foul_chance(tackler, tuning)`, and `card_outcome(aggression, yellows, draw)`. Unit tests over fixed draws at each band edge.
7. **Offside.** Write `rules/offside.rs`: `offside_set(passer, ball, players, teams)` and `is_offence(set, toucher)`. The set is empty for a throw-in, a goal kick, and a corner. Level with the second-last opponent is onside, and the own half is onside. Unit tests for each case.
8. **Restarts.** Write `rules/restart.rs`: `spot`, `targets`, `taker`, and `is_ready`. For a free kick and a corner, an opponent inside 9.15 m targets the nearest point on the 9.15 m circle. For a goal kick, opponents target outside the penalty area. For a penalty, all players except the taker and the defending goalkeeper target outside the area and behind the mark. Unit tests for each kind.
9. **Discipline and team shape.** Write `rules/discipline.rs`: `book`, `send_off`, `on_pitch_count`, and `abandoned`. In `team.rs`, add `active`, `reshape(slot)` (the line's remaining slots spread evenly across the width), and `switch_ends()`; `anchor()` reads active slots only. Unit test: a 4-4-2 back four with one slot removed gives three evenly spread defenders inside the pitch.
10. **Referee.** Write `rules/mod.rs`: `Referee`, `Phase`, `DeadBall`, `Stoppage`, the pending-card list for advantage, and the added-time tally. The referee decides out of play, offside, foul, advantage (outside the penalty area only), card, restart, half-time, full time, and abandonment.
11. **Loop.** In `sim.rs`: replace the wall bounce at `sim.rs:272-281` with exit detection; call the referee after `move_ball` and `resolve_possession`; replace the possession roll with the three-way roll; track `last_touch` for every touch; set `restart` on the tick the ball is placed; call `TickSink::on_stoppage`; change `run(sink)` to play to full time or `max_ticks`; add the law events to `EngineEventKind` with player ids, `card.kind`, advantage, and the engine's minute; generalise `kick_off`. Keep the tick order decide, steer, integrate players, integrate ball, possession, referee, overlaps.
12. **Decisions.** In `decision.rs`, during a dead ball every agent targets its restart target and no pass or tackle is decided. The taker kicks when ready: a throw-in is a short lofted throw, an indirect free kick is always a pass. Every loop skips sent-off players.
13. **Validator.** In `validate.rs`, exempt parking spots from `in_bounds` and `separation`, read reshaped anchors, and add `restart_spot`: on a restart tick the ball lies within 0.5 m of the spot the event names.
14. **Tick file.** In `record.rs`, set `SCHEMA_VERSION` to 4 and document `expected_ticks` as the maximum. The reader accepts a written count at or below it and refuses schema 3 naming both versions.
15. **Snapshot.** Write `snapshot.rs` with the layout in `04-plan-match-rules.yaml`: `Snapshot::capture(&Simulation)`, `write_atomic(path)` (temporary file in the same folder, then `std::fs::rename`), and `read(path)`. The reader checks, in order: magic, version, length, SHA-256, build hash, content hash. Each refusal is `EngineError::Snapshot` naming the reason. Add `Simulation::from_snapshot(config, &Snapshot)`, which verifies the two team-file digests. Unit test: capture, write, read, and capture again give identical bytes.
16. **Scenario builder.** Write `scenario.rs` behind the `scenario` feature: `Scene` with `place`, `ball`, `carrier`, `tick`, `rolls`, and `build`, with no kick-off reset. Add the feature and the self dev-dependency to `crates/engine/Cargo.toml`. Add the scene helpers to `tests/common/mod.rs`.
17. **Records.** In `observe/mod.rs`, add the statistics fields and the two benchmark fields listed in `04-plan-match-rules.yaml`, and fold in the signals from `04b-instrument.md` section 2. Unit test: every contract key is present in both records.
18. **Protocol.** In `crates/protocol`, add the nine event types and six optional fields, set `PROTOCOL_VERSION` to 2 with the reason in the comment beside it, extend the `MESSAGES` event field list, and rewrite the comments at `event.rs:3`, `event.rs:12`, `command.rs:1-2`, `command.rs:55`, and `message.rs:90` without slice names. Update `docs/reference/protocol.md`. Run `cargo test -p protocol`. The document test must pass.
19. **Command line.** In `stream_run.rs`, map every new engine event to `MatchEvent` and forward `on_stoppage`. In `simulate.rs` and `serve.rs`, announce `max_ticks` and attach the snapshot sink; `--no-snapshot` turns the sink off for `simulate`. In `record.rs`, announce `max_ticks`. In `bench.rs`, report `bench.ticks_per_match` and `bench.cpu_us_per_tick` and attach no snapshot sink. Add `resume` to `cli.rs` and `main.rs`, and write `resume.rs`.
20. **Page.** In `web/stoppages.mjs`, export `STOPS_PLAY` and `stopsPlay(message)`. In `web/main.mjs`, add an event tick only when `stopsPlay` is true, and on full time set the scrubber maximum to the last received tick. Extend `web/tests/stoppages.test.mjs`. Run `node --test web/tests/*.test.mjs`. All tests must pass.
21. **Criterion tests.** Write `rules_offside.rs`, `rules_fouls.rs`, `rules_cards.rs`, `rules_restarts.rs`, `rules_clock.rs`, and `snapshot.rs` in `crates/engine/tests`, and `resume.rs` in `crates/engine-cli/tests`, per `04-plan-match-rules.yaml`. Update `full_match.rs` and `validator.rs`. Run `cargo test --workspace`. All tests must pass.
22. **First measurement.** Run `cargo build --release -p engine-cli`, then `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` three times with `SM_DATA_DIR` set to a scratch folder. Record `bench.cpu_us_per_tick`, `bench.ticks_per_match`, and `bench.peak_mem_mb` in the implement artifact. The processor time per tick must be at most 1.566 µs and the peak memory at most 6.69 MB. If either limit is exceeded, stop and report to the product owner with the levers (offside set cost, exit detection, dead-ball targets). Do not remove a law to meet the limit.
23. **Fixture, documents, and gates.** Regenerate the local fixture with `engine-cli record --seed 7 --out fixture.smfx`. Update `README.md` and `content/README.md`. Search `crates/`, `web/`, `docs/`, and `README.md` for workflow vocabulary (slice names, stage names, `.ai/`); the search must return nothing. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `node --test web/tests/*.test.mjs`. Commit with a message in product language that ends with the attribution line the session requires.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-h: benchmark rerun; processor time per tick within 10 percent and peak memory within 25 percent of the baseline, per-match time reported beside it (Round 4 Q13) | `engine-cli bench --seed 42 --matches 5 --json`, three drives (`cli-direct`, cli adapter) | Windows 11 reference machine, rustc 1.92 — yes (this machine; desktop on mains power, Ultimate Performance plan) | `bench.ticks_per_match` and `bench.cpu_us_per_tick` in the run report (step 19); the baseline on `d86996a` in `05c-benchmark.md`; evidence capture of stdout, stderr, and exit code | rerun with `--matches 10` when `bench.cpu_wall_ratio` is below 0.9 → run from a fresh PowerShell with no other heavy process → pre-registered deferral only if the machine is unavailable, clearing event: the developer runs the same command on the reference machine |

- `constraint-resolution: prerequisite-slice: match-rules` — the harness is this slice's own step 19 and step 22; nothing outside the slice is needed. `wall-ownership: code-owned`.

Every other criterion is automated (`observable: false` in the slice) and runs under `cargo test --workspace`. The acceptance criteria keep the slice's wording with three recorded readings: AC-b gains the advantage branch (Round 1 Q4), AC-e is judged against the formula with its seeded variance term (Round 2 Q7), and AC-h is judged per tick (Round 4 Q13).

## Test / Verification Plan

### Automated checks

- Format: `cargo fmt --all -- --check`.
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`.
- Unit and integration: `cargo test --workspace` (about 40 seconds; the full-match tests run by default).
- Page: `node --test web/tests/*.test.mjs` (the glob form; the bare folder form fails on Node 24).
- Criteria map: AC-a `rules_offside.rs`; AC-b `rules_fouls.rs`; AC-c `rules_cards.rs`; AC-d `rules_restarts.rs`; AC-e `rules_clock.rs`; AC-f `snapshot.rs` and `crates/engine-cli/tests/resume.rs`; AC-g the same two files; AC-h the benchmark drive below.
- Regression: `determinism.rs` keeps its byte-identical assertion; `validator.rs` asserts zero violations with the laws on.
- Contract check: the record test in step 17 asserts every contract key the slice emits; `crates/protocol/tests/document.rs` asserts every event field is documented.
- Benchmark compare: `/wf verify` loads `augment/benchmark.md` in compare mode against `05c-benchmark.md` and judges per tick.

### Interactive verification (human-in-the-loop)

- **AC-h (benchmark report).** Platform `cli` from `stack.platforms`; driver: the binary itself (`cli-direct`); no companion skill needed.
  1. Run `cargo build --release -p engine-cli`.
  2. Set `SM_DATA_DIR` to a scratch folder.
  3. From Git Bash, run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json > <evidence-dir>/ac-h-<n>.stdout.txt 2> <evidence-dir>/ac-h-<n>.stderr.txt` for `n` = 1, 2, 3, then write `$?` to `<evidence-dir>/ac-h-<n>.exit-code`.
  4. Read each stdout file: one JSON line with `record.kind` `run-report`. Pass criteria: the median `bench.cpu_us_per_tick` is at most 1.566, the median `bench.peak_mem_mb` is at most 6.69, `bench.match_wall_ms` is at most 2000, `budget.pass` is true, `bench.cpu_wall_ratio` is at least 0.9, `build.hash` is not `unknown`, and the exit code is 0. Report `bench.match_wall_ms` and `bench.ticks_per_match` beside the per-tick figure.
  5. Evidence layout per the cli adapter: `<evidence-dir>/ac-h-<n>.stdout.txt`, `ac-h-<n>.stderr.txt`, `ac-h-<n>.exit-code`.

## Risks / Watchouts

- R1 (high): processor time per tick exceeds the tripwire. Offside runs only at a pass; exit detection is two comparisons; step 22 measures before the tests are finalised. A miss goes to the product owner; no law is removed silently.
- R2 (high): a resumed match drifts. The snapshot covers every field except the scratch buffer and the drained events; the continuation test compares bytes from the snapshot tick onward, and the write-read-write test catches a field that does not round-trip.
- R3 (medium): restart positioning fights the steering agents (slice risk). Decisions are suspended during a dead ball; encroaching opponents target the 9.15 m circle; a restart is taken at three times the delay at the latest; `rules.dead_ball_ticks` shows a stall.
- R4 (medium): PROTOCOL_VERSION 2 and tick schema 4 refuse earlier files. No `.ticks` or `.smfx` file is in the repository; step 23 regenerates the local fixture; both refusals name the found and expected version.
- R5 (medium): the three-way roll changes every seeded match. `determinism.rs` still compares two runs of the new engine; `full_match.rs` and the benchmark re-pin against the new match.
- R6 (low): the viewer draws a sent-off player at the parking spot beside the pitch until `viewer-match-day` hides parked markers.
- R7 (low): foul, card, and added-time rates are estimates. `full_match.rs` holds a sanity band only; `calibration` tunes the rates.
- The `serde_json` `float_roundtrip` concern does not apply: the snapshot is binary and stores every float as its little-endian bytes.

## Dependencies on Other Slices

- Consumes `engine-core` (the loop, the validator, the benchmark), `data-schemas-generator` (the rule pack loader, the attribute schema), and `stream-protocol` (the restart keyframe, the event writer, the protocol document test).
- Touches three `viewer-pitch` files (`web/stoppages.mjs`, `web/main.mjs`, `web/tests/stoppages.test.mjs`) after that slice is verified, never in parallel.
- Consumers: `tactics-and-ai` applies queued changes through `TickSink::on_stoppage`; `commentary` reads the new event types and `player.id`; `viewer-match-day` shows cards and added time and hides parked markers; `viewer-reports-recovery` builds crash recovery on the snapshot and `resume`; `extra-time-penalties` extends `MatchClock`; `calibration` tunes the foul, card, and added-time defaults.

## Assumptions

- The advantage branch applies outside the penalty area only; inside it, a foul is a penalty (Round 1 Q4 scope line).
- The shipped added-time cap is 900 seconds per half, so the announced maximum for a full match is 360,000 ticks and the page reserves about 33.8 MB of history instead of 25.4 MB, under the 300 MB budget.
- Snapshots are written by `simulate` and `serve` only; `bench` and `record` write none, so the timed path and the fixture stay free of disk writes.
- The design contract `02c-craft.md` binds no step: this slice changes page logic in one module and no visual element.
- The `experiment` augmentation stays deferred to `experiment-flags`; `04c-experiment.md` is not authored.
- Consult triggers `appetite-medium-or-larger` and `touches-migration` hold; the product owner excluded `consult` at intake, so no consult ran (`consult-runs: []`).
- The open brainstorm `brainstorm-realism-additions-20260922` routes no thread to this slice; its claim C-01 confirms the wall bounce this slice removes.

## Blockers

None.

## Freshness Research

- Source: [IFAB Laws of the Game, Law 11 Offside](https://www.theifab.com/laws/latest/offside/)
  Why it matters: defines the offside position, the moment of judgment, involvement, the exceptions, and the restart.
  Takeaway: judge at the team-mate's pass; no offside from a goal kick, a throw-in, or a corner; indirect free kick where the player became involved.
- Source: [IFAB Law 12 Fouls and Misconduct](https://www.theifab.com/laws/latest/fouls-and-misconduct/) and [Law 3 The Players](https://www.theifab.com/laws/latest/the-players/)
  Why it matters: a second caution is a sending-off; a team below seven players cannot continue.
  Takeaway: `second-yellow` as a `card.kind`; `min_players: 7` in the rule pack.
- Source: [IFAB Law 13 Free Kicks](https://www.theifab.com/laws/latest/free-kicks/) and [Law 14 The Penalty Kick](https://www.theifab.com/laws/latest/the-penalty-kick/)
  Why it matters: 9.15 m distance; direct versus indirect; the penalty mark and player positions.
  Takeaway: restart targets keep opponents 9.15 m away; an indirect free kick is always a pass.
- Source: [IFAB Laws 15 to 17](https://www.theifab.com/laws/latest/the-throw-in/)
  Why it matters: throw-in at the exit point, goal kick anywhere in the goal area with opponents outside the penalty area, corner in the arc nearest the exit.
  Takeaway: the spot functions in `pitch.rs`.
- Source: [IFAB Law 7 Duration](https://www.theifab.com/laws/latest/the-duration-of-the-match/), [Premier League on added time](https://www.premierleague.com/en/news/3860720)
  Why it matters: allowance for time lost per half; 2024-25 added time fell by about 1 minute 51 seconds against 2023-24.
  Takeaway: per-kind seconds and a cap in the rule pack; defaults aim at about 3 to 4 minutes and 5 to 6 minutes.
- Source: [StatMuse, Premier League fouls 2024-25](https://www.statmuse.com/fc/ask/premier-league-fouls-team-stats-2024-2025), [The Analyst, corners 2024-25](https://theanalyst.com/articles/premier-league-corners-2024-25)
  Why it matters: about 10.5 fouls per team per match and about 5 corners per team per match.
  Takeaway: the default foul rate and the sanity band in `full_match.rs`; yellow, red, offside, throw-in, and goal-kick rates are unsourced estimates for `calibration` to refine.
- Source: local registry `rand_chacha-0.10.0/Cargo.toml` and `src/chacha.rs:134-198`
  Why it matters: the snapshot must restore the generator exactly.
  Takeaway: save seed, stream, and word position; the `serde` feature is not needed.
- Source: [std::fs::rename](https://doc.rust-lang.org/std/fs/fn.rename.html)
  Why it matters: an atomic snapshot replace on Windows.
  Takeaway: an existing file is replaced; write a temporary file in the same folder, then rename.
- Source: [serde_json issue 707](https://github.com/serde-rs/json/issues/707)
  Why it matters: the default float parser is not always correctly rounded.
  Takeaway: not applicable after Round 3 Q10 chose a binary snapshot; recorded so a later JSON export enables `float_roundtrip`.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine match-rules` — the plan is complete and both augmentation artifacts are re-authored for this slice; compact the session first so the SessionStart hook re-reads the artifacts.
- **Option B:** `/wf review football-manager-match-engine viewer-pitch` — four verified slices wait for the slug-wide review; this slice changes three viewer-pitch files, so a review first keeps the two diffs apart.
- **Option C:** `/wf slice football-manager-match-engine` — not needed; the change-queue boundary held (Round 1 Q1).
- **Option D:** `/wf shape football-manager-match-engine` — not needed; the spec held, and the three criterion readings are recorded as product-owner decisions.
