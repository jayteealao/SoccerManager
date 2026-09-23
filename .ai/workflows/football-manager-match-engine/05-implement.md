---
schema: sdlc/v1
type: implement-index
slug: football-manager-match-engine
status: in-progress
stage-number: 5
created-at: "2026-09-21T22:35:04Z"
updated-at: "2026-09-23T10:43:56Z"
slices-implemented: 10
slices-total: 16
metric-total-files-changed: 426
metric-total-lines-added: 41180
metric-total-lines-removed: 1278
tags: [engine, rust, data, protocol, socket, viewer, canvas, web, laws, snapshot, tactics, ai-manager, fatigue, commentary, calibration, schemas, panels, goal-moment, protocol-v3, lineup, substitutions, flow-control]
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
    - 05-implement-commentary.md
    - 05-implement-calibration.md
    - 05-implement-viewer-match-day.md
    - 05-implement-viewer-lineup-tactics.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine viewer-lineup-tactics"
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

- `commentary` is implemented (commit `a414df5`) and awaits verify. Every later slice inherits these changes:
  - Every event except a `tactics-change` verdict carries an optional `commentary` line. Protocol version 2 is kept. The lines come from `content/commentary/en.json` (schema 1), which stays outside the content hash, and every `engine-cli` command now loads that file with the content.
  - `player.id` now also names the taker on `kick-off` and every restart, and on `goal` the player who kicked the ball last (a player of the other club on an own goal). The engine's `last_kicker` is cleared at every dead ball, so the snapshot layout is unchanged, and the seed-42 tick records are byte-identical.
  - `stream_run::Drive` takes `commentary: &Commentary`. `simulate` and `resume` still route no events; when `calibration` makes `match_event` a shared function, the slice that lands second threads the commentator through it.
- `viewer-match-day` renders `commentary` in the feed. `integration` checks the lines end to end.
- Benchmark: `commentary` measured a median of 1.4352 µs per tick (baseline 1.4453 at `e79ad61`, limit 1.10 times) and 6.30 MB peak memory (baseline 6.14, limit 1.25 times). `bench --json` times the bare simulation, so these numbers show that the engine change adds no cost to the simulation. They do not time the commentator.

- `calibration` is implemented (commits `80f976d` and `c00eaac`) and awaits verify. Every later slice inherits these changes:
  - `Summary` counts shots on target, expected goals, passes, completed passes, and open-play possession ticks. The snapshot is version 3, and a version-2 file is refused by name.
  - `match-stats` carries a third flattened block (`MatchFigures`): `stats.goals`, `stats.shots_on_target`, `stats.xg`, `stats.passes`, `stats.pass_accuracy_pct`, `stats.possession_pct`, `manager.kind`, `passes.completed`, and `error.type` on a failure. It stays at `schema.version` "1", and every new field defaults.
  - `schemas/observability/{match-event,match-stats,run-report}.schema.json` are the record contract. `crates/engine-cli/tests/schemas.rs` validates the binary's output against them with `boon`, which only the tests use, so a new record key must be added to the schema or it passes as an additive extra.
  - `simulate` writes `events.jsonl` with commentary lines. `stream_run::match_event` and `Ids` are shared crate-wide, and the commentator is threaded through every saved event file.
  - `engine-cli calibrate` and `content/realism-bands.json` exist. The feature-flags work adds `--flag` to `calibrate`, and the scripting runtime reuses the harness.
  - Tuning moved `shot_noise` to 0.25, `keeper_catch_chance` to 0.84, and `decision.skill` to 0.8. The AI manager checks the score on every goal's stoppage.
- `calibration` leaves AC-d failing: `darkpath.change_never_applied` = 288 over 2000 matches, all queued after the last stoppage of the match. 183 of them come from the AI re-queueing refused injury substitutions, which is handed off to a follow-up task. The product owner has an open question on whether a change that expires at full time counts. The goals tail (sd 3.36) and the untuned mentality offsets are open tuning items.
- Benchmark: `calibration` measured 412.4 to 418.8 ms of processor time per match (median 415.8, gate 460.7), 1.4593 to 1.482 µs per tick, 282,600 ticks per match, and 6.320 to 6.352 MB peak memory (gate 6.82). A 1000-match suite takes 78 seconds on 8 workers.

- `viewer-match-day` is implemented (commits `9e684e9` and `4d21041`) and awaits verify. Every later slice inherits these changes:
  - `PROTOCOL_VERSION` is 3. Each hello team carries `roster` (11 starters in wire-slot order, then the named bench; `player.id`, `player.name`, `player.shirt`, `player.position`, `player.squad_index`). `stats` is sent every 50 ticks and at full time with nine `stats.*` pairs from `MatchFigures` and `Summary`. `condition` carries 22 energy values on the same cadence. Every earlier recording is refused by version.
  - `serve`, `record`, and the streaming `bench` build the simulation before the hello and share `stream_run::hello_teams`. `viewer-lineup-tactics` adds its squad, lineup hold, `set-lineup`, and tactics schema under version 3 and tells the page about a changed lineup.
  - The page reads the match at the rendered tick through `web/match-state.mjs`; every event-kind string the panels read sits in its `KIND` table. `web/components/panels.css` holds the panel styles and the one reduced-motion block (`<html data-motion="reduce">`, set from the media query or `?motion=reduce`). `__touchline.matchDay()` is the read-only hook for the panels.
  - Parked (sent-off) markers are no longer drawn. The close after full time reads "Full time".
- Benchmark: `viewer-match-day` measured the stream at 609,885 to 611,588 delivered ticks per second (baseline 609,220) with 7.61 MB median peak memory (tripwire 8.55), and 416.7 ms of processor time per match (gate 460.7). A 90-minute recording grew from 16.3 MB to 20.5 MB. The p95 restatement and the four page targets are left to verify, because the browser pane was hidden at implement time.

- `viewer-lineup-tactics` is implemented (commit `b46f083`) and awaits verify. Every later slice inherits these changes:
  - `serve` holds before kick-off (`Gate::held`) until the page sends `start`. A client must send `start` after the hello; `record` and `bench` keep an open gate. `set-lineup` is accepted only before the first `start`. `queue-change` is refused before it, and after it an admitted change reaches the engine through an inbox with the page's change identifier.
  - The hello for `serve` carries the home `squad` and `setup`, and every hello carries `tactics` (with `instruction_order`, because JSON keys carry no order) and `substitutions`. `ServerMessage::Hello` is boxed. A hello longer than the 8 KB write bound is sent whole through `stream::send_whole`.
  - The page paces the engine with `pause` and `start` (`web/lead.mjs`) on every tick arrival and every drawn frame. Without that pacing, `serve` finishes the match in seconds. `viewer-reports-recovery` inherits the `Dugout` phases in `web/main.mjs` and the `pending()`, `lineup()`, and `dugout()` test readers.
- Checks at implement: 290 Rust tests and 103 page tests pass, and clippy runs with `-D warnings`. One live drive on seed 11 took a mentality change from Queued to Applied at tick 1992, with the feed row "Tactical change applied". Page height is 800 at 1280 × 800.

## Recommended Next Stage

- `/wf verify football-manager-match-engine viewer-lineup-tactics`
- `/wf verify football-manager-match-engine viewer-match-day` (still open)
