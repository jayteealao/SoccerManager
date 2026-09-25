---
schema: sdlc/v1
type: implement-index
slug: football-manager-match-engine
status: in-progress
stage-number: 5
created-at: "2026-09-21T22:35:04Z"
updated-at: "2026-09-25T04:28:03Z"
slices-implemented: 23
slices-total: 24
metric-total-files-changed: 787
metric-total-lines-added: 65205
metric-total-lines-removed: 2996
tags: [engine, rust, data, protocol, socket, viewer, canvas, web, laws, snapshot, tactics, ai-manager, fatigue, commentary, calibration, schemas, panels, goal-moment, protocol-v3, lineup, substitutions, flow-control, launcher, reconnect, reports, replay-files, e2e, playwright, docs, license-audit, feature-flags, experiment, scripting, sandbox, plugin-interface, distribution, installer, packaging, release-version, error-record, help-text, realism-bands, formations]
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
    - 05-implement-viewer-reports-recovery.md
    - 05-implement-integration.md
    - 05-implement-extra-time-penalties.md
    - 05-implement-experiment-flags.md
    - 05-implement-scripting-runtime.md
    - 05-implement-distribution.md
    - 05-implement-probe-engine-core.md
    - 05-implement-realism-bands-v2.md
    - 05-implement-defending-and-discipline.md
    - 05-implement-tuning-loop.md
    - 05-implement-lone-forward.md
    - 05-implement-keeper-and-shots.md
    - 05-implement-tempo-and-restarts.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine tempo-and-restarts"
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

- `viewer-reports-recovery` is implemented (commits `b03163d` and `aa10882`) and awaits verify. Later slices inherit these changes:
  - `engine-cli launch` is the way a match starts when it must survive an engine crash: it serves the page, runs `serve` as a worker with `--reconnect-wait 120` and a launcher-chosen `--match-millis`, and answers `/engine.json` with `engine.state`, `engine.pid`, `snapshot.tick`, `match.id`, and `launcher`. `POST /engine/restart` and `POST /engine/abandon` require the page's own origin. `integration` starts the charter scenario through it.
  - `Session::finish` returns `SessionEnd` (`Done`, `Closed`, `Dropped`); `SessionConfig` has a `drop_at` test seam; `MatchState::sent_tick` is the newest flushed tick frame. Every caller that builds a `SessionConfig` passes `drop_at`.
  - The serving path writes snapshots through `GatedSnapshots` (captured at every stoppage and at kick-off, persisted once the socket passed them). `simulate` and `resume` keep `SnapshotSink`. File name, layout, and location are unchanged.
  - The page cuts its history, stoppages, events, and frame store back at a reconnect's first keyframe; `PROTOCOL_VERSION` stays 3. The page keeps every frame's wire bytes (`web/replay-file.mjs`), so a saved replay is the engine's `.smfx` layout byte for byte.
  - New hook readers: `recovery()`, `report()`, `replay()`, `lastSavedBytes()`, `events()`.
- Checks at implement: 305 Rust tests and 126 page tests pass, clippy runs with `-D warnings`, and the stream benchmark median is 608,360 ticks per second with 7.83 MB peak (tripwire 8.55 MB).

- `integration` is implemented (commit `e71ebfd`) and awaits verify. It adds these changes:
  - `e2e/` is a Playwright 1.63.0 suite of 21 tests. Each test starts its own engine with a temporary `SM_DATA_DIR`. The whole first match passes all twelve steps against the live engine. `e2e/README.md` maps every viewer behaviour to a test or to one of three human checks.
  - `serve` (and so `launch`) now writes `stats.json` at full time. Before this change, a match played in the browser left no statistics record. A served match that the viewer abandons still writes none (deferred).
  - The header names the engine state in words ("Engine connected · v0.1.0", reconnecting, stopped, finished, replay).
  - `docs/` holds the tutorial, the modding how-to, the command-line and data-file references, and the engine explanation. `content/README.md` is now a pointer. `docs.rs` and `licenses.rs` keep the references and the dependency licenses honest.
- Checks at implement: 310 Rust tests (4 ignored) and 126 page tests pass, clippy runs with `-D warnings`, and the browser suite passes 21 of 21 in 24.4 minutes. Benchmark: the median is 420 ms (before 418 ms), processor time rises by at most 1.5 percent, and peak memory is 6.54 MB or less. 1000 matches take 423.5 s.

- `extra-time-penalties` is implemented (commit `90d4ee1`) and awaits verify. Every later slice inherits these changes:
  - The rule pack is schema 4 (`extra_time`, `shootout`); a schema 3 pack is refused by version. The snapshot is format 4; a format 3 file is refused by name.
  - `MatchConfig::knockout` (off by default, `--knockout` on `simulate`, `bench`, `serve`, `record`) turns on extra time and an on-pitch shoot-out. A regular match is unchanged: the seed-42 records are byte-identical.
  - `MatchClock::half` counts every period; `MatchClock::new` takes the knockout switch. Events gain optional `period`, `shootout_round`, `shootout_scored`, `shootout_scores`, and `decided_by`; protocol version 3 is kept.
  - The validator exempts the anchor rule during a shoot-out, and a tick file past its announced length gets its header raised at finish.
- Benchmark: `extra-time-penalties` measured 421.8 to 422.0 ms of processor time per match (gate 460.7), 1.4926 to 1.4933 µs per tick, and 6.54 to 6.58 MB peak memory (gate 6.82).

- `experiment-flags` is implemented (commit `aca285f`) and awaits verify. Every later slice inherits these changes:
  - `tuning.json` has an optional `flags` block (shipped empty; `TUNING_VERSION` stays 2). A flag needs an owner, a hypothesis, and a removal condition, and it must switch something: tuning overrides by dotted path, or a name in `engine::flags::CODE_FLAGS`. A candidate model registers its name there, declares it in the shipped file, and reads `config.flags.is_on(name)` in the same change.
  - `Content::with_flags(&FlagStates)` applies states once, at load; `Content::load` applies the file's states. `MatchConfig.flags` carries the on-list. The shipped content hash moved once, because the tuning file's bytes changed. The seed-42 tick records are byte-identical.
  - Every `MatchStats` literal needs `flags_on`; `tuning.flags_on` rides in every statistics record.
  - `calibrate` is restructured into per-arm runs (`RunCtx::play_arm`); `--flag` and `--pair` exist, and a paired report adds `calib.flags`, `calib.pair`, `calib.arms`, `calib.compare`, and `calib.verdict`. The scripting runtime can gate scripted models with the same flags block.
- Checks at implement: 364 Rust tests (4 ignored) and 127 page tests pass, and clippy runs with `-D warnings`. Benchmark: 418.8 / 422.0 / 418.8 ms of processor time per match (gate 460.7), and 6.555 to 6.746 MB peak memory (median 6.582, gate 6.82).

- `scripting-runtime` is implemented (commit `7fd5197`) and awaits verify. Every later slice inherits these changes:
  - The engine has a plugin interface (`engine::plugin`, `PLUGIN_API_VERSION` 1) with decision, rule, and commentary hooks, and a `script` crate runs Rhai 1.26.1 packs behind it. `--script-pack <DIR>` works on `simulate`, `bench`, `serve`, `record`, and `resume`. Without a pack, the seed-42 tick records are byte-identical.
  - `EngineEventKind::Script` (18 kinds) and `EventDetail::Script`; the protocol adds the `script` event type and `script.pack`, `script.hook`, `script.outcome`, and `script.detail`, with version 3 kept. `ServerMessage::Event` is boxed, so a constructor writes `Box::new`.
  - Every `MatchStats` literal needs `script` (`ScriptFigures`); `RunReport` has `script_pack`. `content::load` takes a fourth argument, and `bench::measure` takes the pack.
  - The pack hash is folded into the content hash, so a scripted snapshot resumes only with the same pack.
- Checks at implement: 398 Rust tests (4 ignored) and 127 page tests pass, and clippy runs with `-D warnings`. Benchmark on `3066066`: baseline 422.0 ms per match. With the sample pack the median is 431.4 ms (1.022 times, limit 1.10) and peak memory is 8.29 MB (1.22 times, limit 1.25). Without a pack the median is 425.0 ms.

- `distribution` is implemented (commit `6a5d347`) and awaits verify. It adds these changes:
  - `engine-cli launch` needs no flag: `--seed` defaults to a clock seed (printed as `seed <n>` on stderr), and `--web` falls through `SM_WEB_DIR`, `./web`, then `web/` beside the program (`web::resolve_web_dir`). `--open` opens the page in the default browser and logs `launch.open_failed` when it cannot.
  - The Windows build links the C runtime statically (`.cargo/config.toml`), for every MSVC build in the workspace, tests and benches included.
  - `packaging/` builds `SoccerManager-<version>-windows-x64-setup.exe` (per-user NSIS, Start-menu entry `launch --open`) and `SoccerManager-<version>-linux-x86_64.tar.gz` (glibc 2.39 floor) into the git-ignored `dist/`, each with a `.sha256` file. The version comes only from `engine-cli --version`, and `tests/release_version.rs` holds it equal to `hello` and the workspace.
  - `packaging/windows/run-sandbox.ps1` and `packaging/unix/smoke.sh` are the clean-machine checks.
- Checks at implement: 403 Rust tests and 127 page tests pass, and clippy runs with `-D warnings`. Windows Sandbox passed 11 of 11 checks on an image without `vcruntime140.dll`, and the WSL archive check passed 7 of 7. In the sandbox, the Start-menu entry opened no browser, because the sandbox image cannot open `http` links.

- `probe-engine-core` is implemented (commit `733839c`) and awaits verify. Every later slice inherits these changes:
  - A failed `simulate` or `bench` run prints one record on stdout (`FailureRecord`): the success `record.kind`, `outcome` `error`, `error.type`, `error.code`, `error.retriable`, and no statistic key. Exit code 1 and the prose line are kept. Other commands keep prose-only failures.
  - `EngineError` classifies itself (`error_type`, `error_code`, `retriable`). Code that writes an error record uses these, not a private classifier.
  - `outcome` is `success` or `error` in both record schemas; `failure` is refused. Statistic and benchmark keys are required only when the outcome is not `error`, and an error record requires the three `error.*` keys. A failed calibration match writes `error`; a failed worker makes the run report `error` with `error.type` `worker`. Run folders from before this commit do not validate.
  - Short help (`-h`) must fit 80 columns on all 10 help surfaces, and `no_help_line_exceeds_eighty_columns` checks `-h` and `--help` on each. A new option puts its detail in `long_help`.
  - Hidden seam `calibrate --inject-failure <match|worker>` (shard 0 only).
- Checks at implement: 414 Rust tests pass (4 ignored), clippy runs with `-D warnings`, and `bench --seed 42 --matches 5` measured 423 ms per match.

- `realism-bands-v2` is implemented (commit `0835298`) and awaits verify. Every later slice inherits these changes:
  - `content/realism-bands.json` is version 2 with eleven more bands; a version-1 file is refused. Bands change only by a recorded product-owner answer.
  - `match-stats` carries `stats.throw_ins` and `stats.goal_kicks`, required on success.
  - `tactics.json` has ten formations; indices 0 to 3 are unchanged. The content hash moved to `b64cecf856ec`.
  - `calibrate --suite all` plays equal, strength and formations (55 pairings x `--matches`). Pairing checks carry `pairing`; `calib.formations` holds the pairing figures. Each failing band is a `calibrate.band_failed` warn line.
  - The paired-run distance counts only the excess above the top of a floor band.
  - Baseline for the next three slices: 9 of 15 equal-suite bands fail on all five seeds; 151 of 165 pairing checks fail on seed 42 (formations run on one seed by Q-I1). A full run takes about 76 minutes on the 8-core reference machine.
- Checks at implement: 434 Rust tests pass (4 ignored), 128 page tests pass, clippy runs with `-D warnings`, and the benchmark median is 422.0 ms per match and 6.73 MB (tripwires 460.7 ms and 6.82 MB).

- `defending-and-discipline` is implemented (commit `f7fe35b`) and awaits verify. Its kept criteria pass: discipline, the acting keeper and eight of ten formation pairings. The product owner moved the red-card criterion and the 4-4-1-1 (5.18) and 3-4-3 (4.17) pairings to `lone-forward` with their limits unchanged (Q-I2). A re-run on `cd9669c` reproduced every figure. Every later slice inherits these changes:
  - `Simulation::keeper(team)` / `Team::keeper_slot()` name the acting keeper; no code may assume slot 0 keeps goal. `restart::taker` takes `&[Team; 2]` and reads a side's own end from `attack_x`.
  - `Team::reshape` lays the whole team out again; the back line is capped at `back_line_gap` between neighbours at 11 against 11 too, so every formation's back line is narrower than the tactics file says.
  - Pressers go for the ball inside 3 m; the goal-side cover tracks the carrier. Contact near the ball costs about four fouls per won tackle in this engine, so a mechanism that holds a defender in contact floods fouls and penalties.
  - Snapshot version 5 (`foul_ready`); five new tuning values; decision code `sub-keeper`; the content hash moved.
  - Equal suite at seed 42: goals per match 4.092 (baseline 3.035), sending-off share 0.326 (0.421).
- Checks at implement: 458 workspace tests pass (7 ignored), clippy runs with `-D warnings`, and the benchmark median is 1.5312 µs per tick and 6.80 MB (gates 1.6228 µs and 8.27 MB).

- `tuning-loop` is implemented and committed (`a6dc5d2`). Every later tuning slice inherits it:
  - `calibrate --pairing "A v B"`, `--band NAME` and `--baseline report.json` give a loop of under 2 minutes on the reference machine: 87 s for one pairing at 1,000 matches, 86 s for the equal suite, and 42 s for `--suite red-card --seed 1 --matches 120`.
  - Every band row carries `se`. The report carries `fixtures.hash`; a baseline must share the seed, the match count and that hash, and a changed `content.hash` is the change under test. A report made before this change cannot be a baseline.
  - `Simulation::send_off_before_kickoff` is the one send-off-at-kick-off path. `Scene::sent_off` calls it. `--suite red-card` equals the slow test `a_sending_off_gives_no_advantage` to 4 decimals, and its criterion still fails, as `lone-forward` expects.
  - A targeted run is an inner loop. The full gate (five seeds for equal and strength, one for formations) is unchanged, and `--suite all` does not include red-card.
  - No faster build profile: fat LTO and a native CPU target were identical in figures and inside run-to-run timing noise.

- `lone-forward` (implemented, commits `c3cc96f`, `7bd5fa5` and `07d87c2`; awaits verify): five bounded tuning values; shipped `tackle_win_base` 0.5, `decision.lone_hold` 0.5, `decision.lone_dribble` −0.5, with `lone_layoff` and `lone_line_hold` at 0. `fouls::win_chance()`, `offside::second_last_depth()`, `Team::lone_forward()` and the lone-carrier terms. Every later slice inherits these changes:
  - An even tackle wins the ball ten times as often (0.25 against 0.025). In the equal suite, fouls are about 7 per team and yellow cards about 0.9 per team, below the band floor (recorded, Q-E4). Goals, shots and the sending-off share are inside their bands.
  - All ten pairings with 4-4-2 hold at 4.0 or less; 4-4-1-1 3.22 and 3-4-3 2.91 in `every_formation_holds`.
  - The seed-42 90-minute match on the default clubs ends 0-0 with no tactics change. A test that needs a goal or a tactics change uses `common::scoring_match()` (seed 7). The two-minute viewer reconnect test serves seed 5.
  - The red-card experiment plays each seed in both club orders (`--suite red-card --seed 1 --matches 240` equals the slow test). The criterion moved to `keeper-and-shots` (Q-LF2) and its slow test still fails; the measured cause is that a side with ten men attacks as if it had eleven. `keeper-and-shots` may now plan (Q-X4).

- `keeper-and-shots` (implemented at commits `8fd407b` and `bce0144`; verified, result pass, after the Q-KS1 and Q-KS2 answers): a new `shot` module and an optional `shots` tuning block. Every later slice inherits these changes:
  - A shot counts on target when its flight, as struck, crosses between the posts under the bar. A keeper saves only such a shot, on the sourced curve by shot quality, and holds one save in three or parries it. An outfield defender within 3 m can block a shot once. A parry or a block leaves the defending side as the last touch. `keeper_catch_chance` covers only fast balls that are not shots.
  - Penalties in play count `shots.penalty_xg` 0.76. The shoot-out uses the same save model, and its dive values are tuning values.
  - `xg` is refitted (−4.191, −0.0441, 6.3836) and the saves read the fixed `shots.quality` model, so a later refit never moves play.
  - On seed 42 over 1,000 matches: on target 0.387, goals per xG 1.098, goal kicks 13.1, corners 1.59 per team, goals 1.88 per match (under the band, which `realism-tuning` owns).
  - Test seams under the `scenario` feature: `Scene::penalty()`, `Simulation::last_touch()`, `shot_census()` and `shot_flight()`.
  - Q-KS1 moved the red-card criterion to `realism-tuning`; Q-KS2 set the corner floor at 1.2 per team and moved corners from clearances and crosses to `tempo-and-restarts`.

- `tempo-and-restarts` (implemented at commit `831553b`; AWAITING INPUT on Q-TR1): the counting and the tempo levers, shipped switched off. Every later slice inherits these changes:
  - `stats.passes` and pass accuracy count open-play passes only. A clearance (`Kick::Clear`) and the first kick of a restart taker are counted in `stats.clearances` and `stats.restart_kicks`. On seed 42 over 200 matches, passes fall from 1,225.8 to 1,193.8 per team with play identical.
  - `stats.ball_in_play_s` is in match-stats, and the calibrate report has `ball_in_play_min_per_90_mean` (89.6 minutes on shipped play).
  - The snapshot is version 6. A later slice that adds snapshot state takes version 7.
  - New tuning values, switched off: `decision.carry_s` and `decision.carry_cost` (the carry window) and the `clearances` block (aim spread and a defender's clearance of a fast pass in his own penalty area). The restart delays stay at 3, 8, 6 and 8 s until Q-TR1 is answered.
  - Test seam under the `scenario` feature: `Scene::clear()`. A scene starts in open play with no restart taker.
  - Open: 350–550 passes per team come only with 27–61 shots per team and 4.0–14.8 goals per match, and the closest setting breaks `every_formation_holds` (Q-TR1).

## Recommended Next Stage

- `/wf implement football-manager-match-engine tempo-and-restarts` (blocked: after the product owner answers Q-TR1 in `po-answers.md`)

- `/wf verify football-manager-match-engine lone-forward` (new: the three kept criteria on the shipped values; the evidence is in `implement-evidence/lone-forward/shipped/`)

- `/wf verify football-manager-match-engine tuning-loop` (new: the seven criteria on the release binary; the evidence and scripts are in `implement-evidence/tuning-loop/`)

- `/wf verify football-manager-match-engine defending-and-discipline` (new: discipline, the acting keeper and the eight kept formation pairings; the red-card criterion and the 4-4-1-1 and 3-4-3 pairings belong to `lone-forward`)

- `/wf verify football-manager-match-engine realism-bands-v2` (new: the four criteria on the release binary, the viewer check of ten formations, and the seed 42 formations evidence)

- `/wf verify football-manager-match-engine probe-engine-core` (new: `cargo test -p engine-cli --test cli_args --test schemas`, the three failure drives and both `calibrate --inject-failure` drives on the release binary, and the 20 help-width checks)

- `/wf verify football-manager-match-engine distribution` (new: `run-sandbox.ps1` and the WSL `smoke.sh`, `cargo test --test release_version --test install_layout` on both platforms, the benchmark compare after the static runtime, and the pre-registered macOS deferral)

- `/wf verify football-manager-match-engine scripting-runtime` (new: `cargo test -p script` and `cargo test -p engine-cli --test script_cli`, the six-drive benchmark compare against 422.0 ms, and the seed-42 byte-identity check)

- `/wf verify football-manager-match-engine experiment-flags` (new: automated `cargo test -p engine --test flags` and `cargo test -p engine-cli --test calibrate_pair`, the benchmark compare, and the seed-42 byte-identity check)

- `/wf verify football-manager-match-engine extra-time-penalties` (new: automated cargo tests, and re-read IFAB Laws 3, 7, and 10)
- `/wf verify football-manager-match-engine integration`
- `/wf verify football-manager-match-engine viewer-reports-recovery`
- `/wf verify football-manager-match-engine viewer-lineup-tactics`
- `/wf verify football-manager-match-engine viewer-match-day` (still open)
