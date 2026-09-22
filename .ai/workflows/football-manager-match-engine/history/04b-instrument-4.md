---
schema: sdlc/v1
type: augmentation
augmentation-type: instrument
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: match-rules
instrumentation-framework: "serde_json JSON Lines per .ai/observability.md plan-version 1 (match-event, match-stats, run-report) plus tracing to stderr. The page side from viewer-pitch is unchanged."
dark-paths-found: 5
signals-designed: 10
pii-warnings: false
status: ready
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T19:29:33Z"
revision-count: 4
revisions:
  - rev: 1
    at: "2026-09-22T06:37:07Z"
    trigger: new-slice
    because: "data-schemas-generator plan designs the loader, generator, and identity signals (PO plan Q11)"
    changed: "slice-slug, signal table, dark paths; engine-core record kept at history/04b-instrument-0.md"
  - rev: 2
    at: "2026-09-22T11:28:51Z"
    trigger: new-slice
    because: "stream-protocol plan designs the socket, event, and command signals; the contract already reserves the socket keys"
    changed: "slice-slug, signal table, dark paths, implementation notes; the data-schemas-generator record is kept at history/04b-instrument-1.md"
  - rev: 3
    at: "2026-09-22T14:41:32Z"
    trigger: new-slice
    because: "viewer-pitch plan designs page-side signals; the product owner chose option 3 alongside option 2 at plan Round 3 Q12"
    changed: "slice-slug, framework now names two sides, eight browser signals plus one new engine signal, five browser dark paths, a new transport-gap note; the stream-protocol record is kept at history/04b-instrument-2.md"
  - rev: 4
    at: "2026-09-22T19:29:33Z"
    trigger: new-slice
    because: "match-rules plan designs the law, snapshot, and per-tick benchmark signals; the contract already reserves most of the keys"
    changed: "slice-slug, framework back to the engine side, ten signals, five dark paths; the viewer-pitch record is kept at history/04b-instrument-3.md"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-match-rules.md
  contract: ../../observability.md
  prior: history/04b-instrument-3.md
---

# Instrumentation: Match Rules and Stoppage Snapshots

## The Instrumentation

The viewer slice left the engine with four record kinds and the page with nine signals on the console. The observability contract has waited for this slice longer than for any other: its `event.type` list already names offside, foul, card, corner, throw-in, goal-kick, free-kick, penalty, half-time, and snapshot, and it reserves `player.id`, `player.secondary_id`, `snapshot.tick`, `stats.fouls`, `stats.corners`, `stats.offsides`, and `rules.pack_version`. The engine emits none of them yet.

Ten signals are designed, and seven of them fill keys the contract already holds. Law events ride the existing `match-event` record kind with the two player keys and four additive extras: `card.kind`, `foul.advantage`, `minute.added`, and `added_time.s`. Four tracing lines cover the snapshot's life: written, failed, refused, and resumed. Two warnings cover the referee's failure modes: a dead ball forced after a stall and a match abandoned below seven players. The statistics record gains the law counts, and the benchmark report gains processor time per tick, because the product owner chose to judge the gate per tick (plan Round 4 Q13).

Five dark paths are closed, and one of them is inherited from the shape: a queued change that never applies. This slice opens stoppages and applies no change, so `rules.stoppages` counts every hook call and the next slice can prove that each queued change met a stoppage. The top risk is volume: about 100 snapshots per match would flood an info-level log, so `snapshot.written` is debug level and `snapshot.writes` is the statistic to watch.

## 1. Current state

| File | Quality | Existing signals | Dark paths |
|------|---------|-----------------|------------|
| `crates/engine/src/sim.rs` | partial | kick-off, goal, and full-time engine events; the summary counters | a stoppage with no consumer (new with this slice) |
| `crates/engine/src/rules/*.rs` (planned) | dark | none | a dead-ball stall; a silent abandonment; the unseen offside cost |
| `crates/engine/src/snapshot.rs` (planned) | dark | none | none of its own; the write path is in the command line |
| `crates/engine-cli/src/simulate.rs`, `serve.rs` | good | `match-stats` on stdout and in `stats.json`, `tickfile.*` | a lost snapshot |
| `crates/engine-cli/src/resume.rs` (planned) | dark | none | a refused snapshot with no named reason (closed by design in the plan) |
| `crates/engine-cli/src/stream_run.rs` | good | `match-event` for kick-off, goal, full-time | none; the law events extend the mapping |
| `crates/engine-cli/src/bench.rs` | good | `run-report` | none; the report lacks the per-tick unit the gate now reads |
| `crates/engine/src/observe/mod.rs` | good | `MatchStats`, `RunReport` | none; the contract's law keys are absent |
| `crates/protocol/src/event.rs` | good | `match-event.engine`, `match-event.change` | none |
| `web/*.mjs` | good | nine `viewer.*` signals | none new |

Summary: 5 dark paths found across 10 files (4 planned). Framework: the engine side's JSON Lines records and tracing; the page side is unchanged.

## 2. Instrumentation plan

| File | Function/path | Signal type | Signal name | Key fields | Rationale |
|------|--------------|-------------|-------------|------------|-----------|
| `crates/engine-cli/src/stream_run.rs` | event mapping | event (JSON record) | `match-event` law types | `event.type`, `tick`, `minute`, `team.id`, `player.id`, `player.secondary_id`, `card.kind`, `foul.advantage`, `minute.added`, `added_time.s` | Fills the contract's reserved event types and player keys; commentary and the viewer read them |
| `crates/engine/src/rules/clock.rs` | end of regulation per half | event (tracing info) | `rules.added_time` | `half`, `tally_s`, `variance_s`, `added_s`, `announced_min` | The added-time formula is visible in a real match |
| `crates/engine/src/rules/mod.rs` | dead-ball hard limit | log (tracing warn) | `rules.dead_ball_stalled` | `tick`, `kind`, `team.id`, `waited_ticks`, `taker_distance_m` | A forced restart is counted, not hidden |
| `crates/engine/src/rules/discipline.rs` | `abandoned` | log (tracing warn) | `rules.abandoned` | `tick`, `team.id`, `on_pitch` | A match below seven players states why it ended |
| `crates/engine/src/snapshot.rs` | `write_atomic` | event (tracing debug) | `snapshot.written` | `match.id`, `snapshot.tick`, `kind`, `bytes` | Each snapshot is traceable at debug level |
| `crates/engine-cli/src/simulate.rs`, `serve.rs` | snapshot sink | log (tracing error) | `snapshot.write_failed` | `match.id`, `snapshot.tick`, `path`, `reason` | A lost snapshot is named at the stoppage |
| `crates/engine-cli/src/resume.rs` | read | log (tracing error) | `snapshot.refused` | `path`, `reason` | The refusal reason reaches the log as well as stderr |
| `crates/engine-cli/src/resume.rs` | after `from_snapshot` | event (tracing info) | `match.resumed` | `match.id`, `snapshot.tick`, `half`, `home.score`, `away.score`, `build.hash` | A resumed match states where it picked up |
| `crates/engine/src/observe/mod.rs` | `MatchStats` | metric (JSON record) | `match-stats` law fields | `stats.fouls`, `stats.offsides`, `stats.corners`, `rules.pack_version`, `cards.yellow`, `cards.red`, `added_time.s`, `ticks.played`, `rules.stoppages`, `rules.dead_ball_ticks`, `rules.offside_checks`, `snapshot.writes` | The law counts per match, for calibration and for the dark paths |
| `crates/engine-cli/src/bench.rs` | report | metric (JSON record) | `run-report` per-tick fields | `bench.ticks_per_match`, `bench.cpu_us_per_tick` | The benchmark gate reads processor time per tick |

## 3. Signal designs

```rust
// crates/engine-cli/src/stream_run.rs — the law events extend the existing mapping.
// Optional fields are skipped when None, as team.id and change.* already are.
MatchEvent::play(match_id, owner_id, EventType::Card, tick, minute, score)
    .team(team_id)
    .player(player_id)              // player.id — the booked player
    .card(CardKind::SecondYellow)   // card.kind — "yellow" | "second-yellow" | "red"
    .minute_added(minute_added);    // minute.added — Some(2) at 45+2, None otherwise

MatchEvent::play(match_id, owner_id, EventType::Foul, tick, minute, score)
    .team(offending_team_id)
    .player(tackler_id)             // player.id — the offender
    .secondary(fouled_id)           // player.secondary_id — the fouled player
    .advantage(true);               // foul.advantage — play continued

// crates/engine/src/rules/clock.rs — once per half, when regulation time ends
tracing::info!(signal = "rules.added_time", half, tally_s, variance_s, added_s, announced_min);

// crates/engine/src/rules/mod.rs — the forced restart
tracing::warn!(signal = "rules.dead_ball_stalled", tick, kind = kind.code(), team.id = %team_id,
               waited_ticks, taker_distance_m);

// crates/engine/src/rules/discipline.rs
tracing::warn!(signal = "rules.abandoned", tick, team.id = %team_id, on_pitch);

// crates/engine/src/snapshot.rs — debug level: about 100 rows per match
tracing::debug!(signal = "snapshot.written", match.id = %match_id, snapshot.tick = tick,
                kind = kind.code(), bytes);

// crates/engine-cli/src/simulate.rs and serve.rs — the match continues after this line
tracing::error!(signal = "snapshot.write_failed", match.id = %match_id, snapshot.tick = tick,
                path = %relative, reason = %err);

// crates/engine-cli/src/resume.rs
tracing::error!(signal = "snapshot.refused", path = %relative, reason = %reason);
tracing::info!(signal = "match.resumed", match.id = %match_id, snapshot.tick = tick, half,
               home.score, away.score, build.hash = %build);
```

Types: every tick is a `u32`; `minute` and `minute.added` are integers, and `minute.added` is absent outside added time; `added_time.s` is an integer number of seconds and appears on `half-time` and `full-time`; `card.kind` is one of three strings; `foul.advantage` is a boolean and appears on `foul` only; `waited_ticks` is an integer; `taker_distance_m` is a float with two decimals; `bytes` is an integer; `bench.cpu_us_per_tick` is a float with four decimals; `bench.ticks_per_match` is an integer.

## 4. PII & security notes

No PII concerns identified in the planned signals. `player.id` and `player.secondary_id` identify fictional generated players (`p-<club>-<nn>`), never a person. `owner.id` stays opaque per the contract. Every path in a signal is relative to `SM_DATA_DIR` or the working directory, never absolute, so a user name in a home folder does not reach a log.

One integrity note that is not a PII note: `snapshot.refused` must name the reason and never print the file's bytes. A corrupt file is untrusted input, and the reason string is built from the reader's own checks, not from the file's content.

## 5. Implementation notes

- **Touch order.** `observe/mod.rs` first (step 17 of the plan), so the statistics fields exist before the referee fills them; then `rules/*.rs` with their tracing lines as each module lands (steps 5 to 10); then `snapshot.rs` (step 15); then the command line (step 19). No file is touched twice for signals alone.
- **No new dependency.** `tracing`, `serde`, and `serde_json` are already in the workspace.
- **No new environment variable.** `SM_LOG` already controls the tracing level; `SM_DATA_DIR` already locates the snapshot folder.
- **Additive extras.** `card.kind`, `foul.advantage`, `minute.added`, `added_time.s`, and the non-contract statistics fields are additive extras, like `content.hash`, `teams`, and `change.queue_id` before them. `/wf observability audit` settles whether they join Block A.
- **The contract's `snapshot` event type is not emitted as a `match-event`.** About 100 rows per match on the socket would crowd the event stream the page and commentary read. The snapshot is a tracing line plus `snapshot.writes`; the audit can promote it if a consumer needs it.
- **`minute` moves to the engine.** `MatchEvent::play` computes the minute from the tick today (`crates/protocol/src/event.rs:94`), which is wrong after first-half added time. The engine's clock supplies `minute` and `minute.added` from this slice on.
- **The benchmark path writes nothing.** `bench` attaches no snapshot sink, so `snapshot.written` never appears in a benchmark run and the per-tick figure measures the referee alone.
