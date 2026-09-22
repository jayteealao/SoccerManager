---
schema: sdlc/v1
type: augmentation
augmentation-type: instrument
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: tactics-and-ai
instrumentation-framework: "serde_json JSON Lines per .ai/observability.md plan-version 1 (match-event, match-stats, run-report) plus tracing to stderr. The page side is unchanged."
dark-paths-found: 4
signals-designed: 9
pii-warnings: false
status: ready
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T22:18:43Z"
revision-count: 5
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
  - rev: 5
    at: "2026-09-22T22:18:43Z"
    trigger: new-slice
    because: "tactics-and-ai plan designs the change-verdict, substitution, injury, AI, and fatigue signals; the contract already reserves the event types and most keys"
    changed: "slice-slug, nine signals, four dark paths including the shape's change-never-applied path, which this slice closes; the match-rules record is kept at history/04b-instrument-4.md"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-tactics-and-ai.md
  contract: ../../observability.md
  prior: history/04b-instrument-4.md
---

# Instrumentation: Tactics, Fatigue, and the AI Manager

## The Instrumentation

The match-rules slice opened stoppages and left one dark path from the shape open on purpose: a queued change that never applies. It counted the stoppages so this slice could close that path. The contract already lists `substitution`, `injury`, and `ai-decision` as event types, and it reserves `change.applied_tick`, `ai.decision`, `fatigue.mean_pct`, `injury.count`, `stats.shots`, and `darkpath.change_never_applied` (`.ai/observability.md:27-59`). The engine emits none of them yet.

This slice designs nine signals, and most of them fill keys the contract already holds. Every change in the engine's queue ends as an applied or rejected `tactics-change` event with its tick and reason. `darkpath.change_never_applied` counts changes still pending at full time. A change queued over the socket still ends at `queued`, as today, because the socket bridge waits for the product owner's answer on the payload format; its `match-event` rows already show that it was never applied. Every AI choice emits an `ai-decision` event with a short code. Fatigue and injuries reach the statistics record.

The top risk is noise. A plan recompute and an applied change are debug lines, because a match with many stoppages would flood the info level. Rejections, AI choices, and injuries are info lines, because each one is rare and a person acts on it.

## 1. Current state

| File | Quality | Existing signals | Dark paths |
|------|---------|-----------------|------------|
| `crates/stream/src/control.rs` | partial | `match-event` for queued and refused changes | none new; socket changes are not bridged in this slice |
| `crates/engine/src/tactics/change.rs` (planned) | dark | none | a change never applied; a rejection without a reason |
| `crates/engine/src/ai.rs` (planned) | dark | none | a silent AI |
| `crates/engine/src/fatigue.rs` (planned) | dark | none | invisible fatigue |
| `crates/engine/src/rules/mod.rs` | good | law events, `rules.*` tracing | none new; injuries extend it |
| `crates/engine-cli/src/stream_run.rs` | good | `match-event` for every engine event | none; the new events extend the mapping |
| `crates/engine/src/observe/mod.rs` | good | `MatchStats` with law fields | none; shots, fatigue, injuries, and change counts are absent |
| `crates/protocol/src/event.rs` | good | `match-event.engine`, `match-event.change` | `team.id` is missing on change events |
| `crates/engine-cli/src/bench.rs` | good | `run-report` with per-match and per-tick fields | none |

Summary: 4 dark paths found across 9 files (3 planned). Framework: the engine side's JSON Lines records and tracing.

## 2. Instrumentation plan

| File | Function/path | Signal type | Signal name | Key fields | Rationale |
|------|--------------|-------------|-------------|------------|-----------|
| `crates/engine-cli/src/stream_run.rs` | event mapping | event (JSON record) | `match-event` change verdicts | `event.type` `tactics-change`, `change.kind`, `change.queue_id`, `change.queued_tick`, `change.applied_tick`, `change.state`, `change.rejected_reason`, `team.id` | The page clears its chips from these, and the timeline dashboard reads queued against applied ticks |
| `crates/engine-cli/src/stream_run.rs` | event mapping | event (JSON record) | `match-event` substitution, injury, ai-decision | `player.id`, `player.secondary_id`, `team.id`, `ai.decision` | Fills three reserved event types; commentary and the AI-decisions dashboard read them |
| `crates/engine/src/tactics/change.rs` | `apply_at_stoppage` | event (tracing debug) | `change.applied` | `tick`, `team`, `kind`, `queue_id`, `waited_ticks`, `stoppage` | How long a change waited, at debug level |
| `crates/engine/src/tactics/change.rs` | `apply_at_stoppage` | log (tracing info) | `change.rejected` | `tick`, `team`, `kind`, `queue_id`, `reason` | Every rejection names its reason |
| `crates/engine/src/ai.rs` | `in_match` | event (tracing info) | `ai.decision` | `tick`, `minute`, `team`, `code`, `score_state` | The AI's reasoning is readable on stderr |
| `crates/engine/src/rules/mod.rs` | `injure` | event (tracing info) | `injury.occurred` | `tick`, `team`, `player`, `source`, `energy` | Injuries and their cause, for calibration |
| `crates/engine/src/observe/mod.rs` | `MatchStats` | metric (JSON record) | `match-stats` tactics fields | `stats.shots`, `injury.count`, `fatigue.mean_pct`, `darkpath.change_never_applied`, `changes.queued`, `changes.applied`, `changes.rejected`, `substitutions`, `ai.decisions` | Closes the shape's dark path and feeds the realism bands |
| `crates/engine/src/tactics/mod.rs` | `TeamPlan::from` | log (tracing debug) | `tactics.plan` | `tick`, `team`, `formation`, `mentality`, `press_count`, `block_depth` | What a change did to the numbers play reads |
| `crates/engine-cli/src/bench.rs` | report | metric (JSON record) | `run-report` (unchanged) | `bench.cpu_ms`, `bench.matches`, `bench.cpu_us_per_tick`, `bench.ticks_per_match` | The gate reads per-match processor time; no new field |

## 3. Signal designs

```rust
// crates/engine-cli/src/stream_run.rs — an applied change at a stoppage
MatchEvent::change(owner_id, match_id, tick, scores, outcome)   // outcome.state = Applied
    .team(Some(club_id))            // team.id — now present on change events
    .applied_tick(Some(tick));      // change.applied_tick — contract key

// a substitution names both players
MatchEvent::play(owner_id, match_id, tick, EventType::Substitution, Some(club_id), scores)
    .player(off_id)                 // player.id — the player leaving
    .secondary(on_id);              // player.secondary_id — the player entering

// an AI choice
MatchEvent::play(owner_id, match_id, tick, EventType::AiDecision, Some(club_id), scores)
    .ai_decision(Some("mentality-up-trailing".into()));        // ai.decision — contract key

// crates/engine/src/tactics/change.rs
tracing::debug!(signal = "change.applied", tick, team, kind = kind.code(), queue_id = %id,
                waited_ticks, stoppage = stoppage.code());
tracing::info!(signal = "change.rejected", tick, team, kind = kind.code(), queue_id = %id,
               reason = %reason);

// crates/engine/src/ai.rs
tracing::info!(signal = "ai.decision", tick, minute, team, code, score_state = %state);

// crates/engine/src/rules/mod.rs
tracing::info!(signal = "injury.occurred", tick, team, player, source, energy);

// crates/engine/src/tactics/mod.rs
tracing::debug!(signal = "tactics.plan", tick, team, formation = %name, mentality = %m,
                press_count, block_depth);
```

Types: ticks are `u32`; `waited_ticks` is an integer; `energy` and `fatigue.mean_pct` are floats with two decimals (`fatigue.mean_pct` is 0 to 100); `code` is one of four short strings; `source` is `tackle` or `background`; `darkpath.change_never_applied` is an integer that must be 0 in a healthy match.

## 4. PII & security notes

No PII concerns identified in the planned signals. `player.id` and `player.secondary_id` identify fictional generated players, never a person, and `owner.id` stays opaque per the contract. `change.rejected_reason` is built from the engine's own checks and names player ids from the team file. It never echoes the client's raw detail text, so a malformed payload cannot inject text into a log line. The protocol's refusal of a malformed detail names the field, not the value.

## 5. Implementation notes

- **Touch order.** `observe/mod.rs` first (plan step 12), then `tactics/change.rs` and `ai.rs` as they land (steps 10 and 11), then `rules/mod.rs` for injuries (step 7), then the command line mapping (step 17). No file is touched twice for signals alone.
- **No new dependency and no new environment variable.** `tracing`, `serde`, and `serde_json` are in the workspace; `SM_LOG` controls the level.
- **Contract keys first.** `change.applied_tick`, `ai.decision`, `fatigue.mean_pct`, `injury.count`, `stats.shots`, and `darkpath.change_never_applied` are contract keys; `stats.shots_on_target` stays with the calibration work that defines it. `changes.queued`, `changes.applied`, `changes.rejected`, `substitutions`, and `ai.decisions` are additive extras until `/wf observability audit` settles them.
- **`team.id` on change events.** `MatchEvent::change` set it to `None` (`crates/protocol/src/event.rs:220-229`). A change belongs to one team, so it is filled from this slice on.
- **The benchmark path stays quiet.** `bench` runs with tracing at the default level, where the debug lines are off. The AI and injury info lines are rare (a few per match), so they do not load the timed path.
