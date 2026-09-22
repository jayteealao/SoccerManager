---
schema: sdlc/v1
type: implement-index
slug: football-manager-match-engine
status: in-progress
stage-number: 5
created-at: "2026-09-21T22:35:04Z"
updated-at: "2026-09-22T23:27:52Z"
slices-implemented: 6
slices-total: 16
metric-total-files-changed: 301
metric-total-lines-added: 28953
metric-total-lines-removed: 887
tags: [engine, rust, data, protocol, socket, viewer, canvas, web, laws, snapshot, tactics, ai-manager, fatigue]
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  records:
    - 05-implement-engine-core.md
    - 05-implement-data-schemas-generator.md
    - 05-implement-stream-protocol.md
    - 05-implement-viewer-pitch.md
    - 05-implement-match-rules.md
    - 05-implement-tactics-and-ai.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine tactics-and-ai"
---

# Implement Index

## Cross-Slice Integration Notes

- `engine-core` is the root slice; it is implemented, verified, and committed on `feat/football-manager-match-engine`.
- `data-schemas-generator` is implemented and awaits verify: `MatchConfig::new(seed, minutes, &Content, [&TeamFile; 2])` replaced the built-in teams and `Tuning::default()` on the binary path; the tick-file header is schema 2; `owner.id` and `match.id` ride in `stats.json` and the header.
- `stream-protocol` is implemented, verified, and committed. It wraps the `TickSink` trait in `crates/engine/src/record.rs` with `FrameSink`, adds `FanoutSink` beside it, and raises the tick-file header to schema 3: the restart flag now rides in bit 31 of the stored tick number, so the validator reads kick-offs instead of guessing from a two-metre ball jump. Two new crates: `crates/protocol` (messages, codec, change queue, no input or output) and `crates/stream` (server, session, recorder, replayer).
- A slice that reads a `.ticks` file written before this change must be rebuilt: header schema 2 is refused by name, as the header's fail-closed pattern requires.
- `match-rules` consumes `RulePack` from `crates/engine/src/data/rules.rs`, replaces the wall bounce in `crates/engine/src/sim.rs`, and takes `protocol::Queue::pending()` to apply the changes `stream-protocol` only holds. It also adds the event types beyond kick-off, goal, and full time.
- `viewer-pitch` is implemented and awaits verify. It adds `web/`, a page with no build step, and `crates/engine-cli/src/web.rs`, a static server that sends the folder with `Cross-Origin-Opener-Policy` and `Cross-Origin-Embedder-Policy` on every response. The page finds the socket through a generated `GET /engine.json`, because the page and the socket sit on two ports the operating system chooses.
- `viewer-pitch` changed two things every later slice inherits. `TeamRef` now carries `team.kit.primary` and `team.kit.secondary`, additively, with `PROTOCOL_VERSION` still 1. `engine-cli record` now writes the hello as the fixture's first entry, so a fixture recorded before this change is refused by name through `StreamError::FixtureNoHello`; record it again.
- `viewer-match-day`, `viewer-lineup-tactics`, and `viewer-reports-recovery` inherit `web/tokens.css`, `web/components/match-control.css`, and `web/mark.mjs`. They add no second button style, no second token prefix, and no image file.
- `match-rules` fills `web/stoppages.mjs`, which ships built and tested against a fixture that contains no restart frame.
- `viewer-match-day` owns one open question: `engine-cli serve` streams a whole match as fast as the socket accepts it, about 270,000 ticks in 13 seconds, and then closes. A paced live mode does not exist.
- `tactics-and-ai` maps roles onto the ten `Position` codes and reads `FatigueTuning`.
- `calibration` calls `generate_league` for 20-club leagues and tunes `per_position` and the constants in `Tuning`.
- Benchmark: engine-core 398 ms at implement and 393 ms at verify (the baseline); data-schemas-generator measured 390 ms at implement with 5.69 MB peak memory; stream-protocol measured 390 ms wall, 387.4 ms processor time, and 6.01 MB at implement, inside the 422 ms and 7.03 MB tripwires, and recorded the first socket value at 640,057 delivered ticks per second with no pause. `viewer-pitch` adds four browser targets, which verify measures for the first time; the page already reports 32 frames per second with zero dropped frames on a 32 hertz pane, and a full match stores 24.2 megabytes against the 300-megabyte budget.

- `match-rules` is implemented and verified. It adds the referee (`crates/engine/src/rules/`), the snapshot (`crates/engine/src/snapshot.rs`), and `engine-cli resume`. Every later slice inherits four changes:
  - `PROTOCOL_VERSION` is 2, because `ticks_expected` now means the most ticks a match can last. The full-time event marks the real last tick.
  - The tick-file header is schema 4, with `expected_ticks` as a maximum. A schema 3 file is refused by name. The rule pack is schema 2, and the attribute schema requires `aggression`.
  - `Simulation::run(&mut sink)` plays to full time with no tick count. `TickSink::on_stoppage` announces every stoppage after the tick record; `tactics-and-ai` applies queued changes there.
  - A sent-off player stays in the roster, parked at (±(10 + slot), −37). Code that loops over players checks `Player::active()`.
- `commentary` reads 13 event types, `player.id`, `player.secondary_id`, `card.kind`, `foul.advantage`, `minute.added`, and `added_time.s`.
- `viewer-match-day` shows cards and added time from those fields and hides parked markers. `viewer-reports-recovery` builds crash recovery on `snapshot.smsn` and `resume`.
- `calibration` tunes `foul_base` (0.1, tuned on four seeds), the card rates, and the added-time defaults. Offsides and corners are rare until `tactics-and-ai` adds forward runs.
- Benchmark: `match-rules` measured 1.4719 to 1.5053 µs of processor time per tick over five runs, against the 1.566 µs limit, with 276,100 ticks in the median match and 5.45 MB peak memory against 6.69 MB. The margin is about 4 percent.

- `tactics-and-ai` is implemented and awaits verify. Every later slice inherits these changes:
  - `content/tactics.json` (version 1) holds formations, mentalities, instructions, roles, duties, and the AI settings. The rule pack is schema 3 (`substitutions.windows_exempt`), the tuning file is schema 2 (fatigue curve, `decision` weights, injury rates, `restart_delay_s.drop_ball`), and the attribute schema requires 14 names.
  - The engine owns the change queue: `Simulation::queue_change(team, Change)` applies at the next admitting stoppage inside `step`, substitutions first, and every verdict is an event. The socket's `queue-change` is still not routed into it; the slice that settles the change `detail` format adds that bridge.
  - A substitute takes the leaving player's roster slot; `Player::squad` and `Team::lineup` name who is on. `Status::Injured` parks a player like a sent-off one.
  - Snapshot version 2; a version-1 file is refused by name. The protocol stays at version 2 with three new event types and two optional fields.
  - The keeper's catch is rolled once per flight. Matches now score: 3.0 goals, 6.2 shots, and 9.6 fouls per team per match on 12 seeds.
- `commentary` reads `injury`, `substitution`, `ai-decision`, and the `tactics-change` verdicts. `viewer-lineup-tactics` renders `tactics.json`. `calibration` tunes the `decision` block, the mentality offsets (attacking 33.69 shots per match against defensive 2.15 is far too wide), the fatigue curve, and the injury rates.
- Benchmark: `tactics-and-ai` measured 431.2 to 434.4 ms of processor time per match (limit 460.7), 1.445 to 1.456 µs per tick, 298,350 ticks per match, and 6.14 to 6.21 MB peak memory (limit 6.82).

## Recommended Next Stage

- `/wf verify football-manager-match-engine tactics-and-ai`
