---
schema: sdlc/v1
type: plan-index
slug: football-manager-match-engine
status: complete
stage-number: 4
created-at: "2026-09-21T21:57:49Z"
updated-at: "2026-09-25T18:12:40Z"
planning-mode: single
slices-planned: 21
slices-total: 24
implementation-order: [engine-core, data-schemas-generator, stream-protocol, viewer-pitch, match-rules, tactics-and-ai, commentary, calibration, viewer-match-day, viewer-lineup-tactics, viewer-reports-recovery, integration, distribution, extra-time-penalties, realism-bands-v2, defending-and-discipline, tuning-loop, lone-forward, keeper-and-shots, tempo-and-restarts, realism-tuning]
conflicts-found: 2
revision-count: 21
revisions:
  - rev: 1
    at: "2026-09-22T06:40:12Z"
    trigger: new-slice
    because: "data-schemas-generator planned"
    changed: "slices-planned 2; summary, cross-cutting concerns, integration points, and order updated"
  - rev: 2
    at: "2026-09-22T11:28:51Z"
    trigger: new-slice
    because: "stream-protocol planned; U-1 closed"
    changed: "slices-planned 3; summary, cross-cutting concerns, integration points, order, and conflicts updated"
  - rev: 3
    at: "2026-09-22T14:41:32Z"
    trigger: new-slice
    because: "viewer-pitch planned; the visual contract 02c-craft.md is authored and the frontend enters the repository"
    changed: "slices-planned 4; summary, four new cross-cutting concerns for the page, integration points, order, and conflicts updated"
  - rev: 4
    at: "2026-09-22T19:29:33Z"
    trigger: new-slice
    because: "match-rules planned; the referee, the stoppage hook, and the snapshot enter the engine, and the announced match length becomes a maximum"
    changed: "slices-planned 5; summary, five new cross-cutting concerns, integration points, order, and conflicts updated"
  - rev: 5
    at: "2026-09-22T22:16:48Z"
    trigger: new-slice
    because: "calibration planned ahead of tactics-and-ai; the record schema files, the run report, and the calibrate command enter the plan"
    changed: "calibration added to slices-planned, refs.plans, and the slice summaries"
  - rev: 6
    at: "2026-09-22T22:23:27Z"
    trigger: new-slice
    because: "commentary planned; an optional commentary field joins the event message and goal and restart events name a player"
    changed: "slices-planned 7; commentary added to refs.plans, the slice summaries, the integration points, and the conflicts (the shared match_event mapping with calibration)"
  - rev: 7
    at: "2026-09-22T22:27:20Z"
    trigger: new-slice
    because: "experiment-flags planned; the deferred experiment augmentation 04c-experiment.md is authored"
    changed: "slices-planned 8; experiment-flags added to refs.plans and the slice summaries; the experiment cross-cutting concern and the integration points updated"
  - rev: 8
    at: "2026-09-22T22:32:24Z"
    trigger: new-slice
    because: "tactics-and-ai planned; the change queue moves into the engine and applies inside the stoppage tick, and the socket bridge waits for the product owner's payload answer in the lineup-editor plan"
    changed: "slices-planned 9; tactics-and-ai added to refs.plans and the summaries; the stoppage cross-cutting concern and two integration points corrected; two version-number and statistics-key conflicts recorded; order and next stage updated; benchmark and instrument augmentations re-authored for tactics-and-ai"
  - rev: 9
    at: "2026-09-22T22:33:56Z"
    trigger: new-slice
    because: "scripting-runtime planned; a runtime-free plugin interface enters the engine and a separate script crate embeds Rhai behind it"
    changed: "slices-planned 10; scripting-runtime added to refs.plans and the slice summaries; count of unplanned slices updated"
  - rev: 10
    at: "2026-09-23T08:41:11Z"
    trigger: answers-returned
    because: "distribution plan complete after the product owner chose Windows plus Linux with macOS deferred, and the Start-menu shortcut"
    changed: "slices-planned 11; distribution added to refs.plans, the slice summaries, the implementation order, and the recommended order"
  - rev: 11
    at: "2026-09-23T08:46:29Z"
    trigger: answers-returned
    because: "viewer-match-day plan complete after the product owner chose option A for Q-1 (roster in the hello, stats every simulated second, a condition message, protocol version 3)"
    changed: "slices-planned 12; viewer-match-day added to refs.plans and the slice summaries; a protocol-version-3 cross-cutting concern added; count of unplanned slices updated"
  - rev: 12
    at: "2026-09-23T08:44:13Z"
    trigger: answers-returned
    because: "extra-time-penalties plan complete after the product owner chose option B for Q-1 (shoot-out kicks on the pitch, 10-round allowance in the announced length)"
    changed: "slices-planned 13; extra-time-penalties added to refs.plans, the slice summaries, and the implementation order"
  - rev: 13
    at: "2026-09-23T18:52:56Z"
    trigger: answers-returned
    because: "probe-engine-core plan complete after the product owner chose option C for Q-2 (unify the failed outcome on error)"
    changed: "slices-planned 14 and slices-total 17 (the probe slice counted); probe-engine-core added to refs.plans and the slice summaries; the stale probe carry-over line corrected"
  - rev: 14
    at: "2026-09-23T22:29:35Z"
    trigger: new-slice
    because: "realism-bands-v2 planned; extension round 1 opens with eleven sourced bands, six new formations, and an all-against-all formations suite"
    changed: "slices-planned 15 of 22; realism-bands-v2 added to refs.plans, the summaries, a cross-cutting concern, the order, the conflicts, and the next stage"
    snapshot: history/04-plan-13.md
  - rev: 15
    at: "2026-09-24T02:46:11Z"
    trigger: new-slice
    because: "defending-and-discipline planned; goal-side cover, intercept pressing, a derived acting keeper, and a per-player foul cooldown enter the engine"
    changed: "slices-planned 16 of 22; defending-and-discipline added to refs.plans, the summaries, a cross-cutting concern, the order, the conflicts, and the next stage"
    snapshot: history/04-plan-14.md
  - rev: 16
    at: "2026-09-24T07:09:11Z"
    trigger: new-slice
    because: "tuning-loop plan written but awaiting input: the baseline guard on content.hash would refuse every tuning change (Q-P1)"
    changed: "slices-total 24 (extension round 2 added tuning-loop and lone-forward); tuning-loop added to refs.plans and the summaries as awaiting input; a conflicts note; the stale next stage (defending-and-discipline is verified) replaced"
    snapshot: history/04-plan-15.md
  - rev: 17
    at: "2026-09-24T08:26:23Z"
    trigger: answers-returned
    because: "tuning-loop plan complete after the product owner chose the fixtures hash for Q-P1 (the baseline guard)"
    changed: "slices-planned 17 of 24; tuning-loop summary no longer awaiting input and names the fixtures hash; tuning-loop added to implementation-order; next stage is implement tuning-loop"
    snapshot: history/04-plan-16.md
  - rev: 18
    at: "2026-09-24T11:53:45Z"
    trigger: new-slice
    because: "lone-forward planned; the lone carrier's weighted terms, the line hold off the ball and a tuned won-tackle base enter the engine"
    changed: "slices-planned 18 of 24; lone-forward added to refs.plans, the summaries, implementation-order, the conflicts and the next stage"
    snapshot: history/04-plan-17.md
  - rev: 19
    at: "2026-09-24T16:36:13Z"
    trigger: new-slice
    because: "keeper-and-shots planned; on-target from the trajectory, a save model with held and parried saves, outfield blocks, and one keeper model for penalties and the shoot-out enter the engine"
    changed: "slices-planned 19 of 24; keeper-and-shots added to refs.plans, the summaries, implementation-order, the conflicts and the next stage"
    snapshot: history/04-plan-18.md
  - rev: 20
    at: "2026-09-25T03:12:00Z"
    trigger: new-slice
    because: "tempo-and-restarts planned; a carry window in the carrier's decision, restart delays at the sourced medians, separate counts for clearances and restart kicks, a ball-in-play figure, and a defender's clearance of a fast pass in his own penalty area enter the engine"
    changed: "slices-planned 20 of 24; tempo-and-restarts added to refs.plans, the summaries, implementation-order, the conflicts and the next stage"
  - rev: 21
    at: "2026-09-25T18:12:40Z"
    trigger: new-slice
    because: "realism-tuning planned after the product owner lifted the veto for planning; Q-RT1 to Q-RT4 answered"
    changed: "slices-planned 21 of 24; realism-tuning added to refs.plans, the summaries, implementation-order, the conflicts and the next stage"
    snapshot: history/04-plan-19.md
tags: [engine, rust, 2d-viewer]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  plans: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-calibration.md, 04-plan-commentary.md, 04-plan-experiment-flags.md, 04-plan-tactics-and-ai.md, 04-plan-scripting-runtime.md, 04-plan-distribution.md, 04-plan-viewer-match-day.md, 04-plan-extra-time-penalties.md, 04-plan-probe-engine-core.md, 04-plan-realism-bands-v2.md, 04-plan-defending-and-discipline.md, 04-plan-tuning-loop.md, 04-plan-lone-forward.md, 04-plan-keeper-and-shots.md, 04-plan-tempo-and-restarts.md, 04-plan-realism-tuning.md]
  contract: 02c-craft.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine realism-tuning"
---

# Plan Index

## Slice Plan Summaries

- **engine-core** (`04-plan-engine-core.md`, implemented and verified): 34 new files across the workspace root, `crates/engine`, and `crates/engine-cli`; strategy: build the library bottom-up with determinism as a design constraint, measure the benchmark at step 15; key risk: the 2-second budget at 50 decisions per second, retired at 393 ms.
- **data-schemas-generator** (`04-plan-data-schemas-generator.md`): 44 files (21 new, 23 modified) across `content/`, `crates/engine/src/data`, the engine seams, `observe`, tests, and the command line; strategy: one generic fail-closed loader with `garde` rules, attributes as a fixed array with derived values computed at load, the generator as the single source of the shipped default teams, identity in `stats.json` and the tick header; key risk: determinism once attributes vary per player.

- **stream-protocol** (`04-plan-stream-protocol.md`): 38 files (26 new, 12 modified) across two new crates, the engine seams, the content folder, the command line, and `docs/reference/`; strategy: a blocking WebSocket server with one thread per client, a codec crate free of input and output, a bounded channel of 500 ticks as the backpressure, and a fixture that stores the wire bytes so a replay is byte-identical by construction; key risk: the encoder on the hot path at 270,000 frames per match.

- **viewer-pitch** (`04-plan-viewer-pitch.md`): 46 files (30 new, 16 modified) across a new `web/` folder, the protocol and stream crates, the command line, and the project documentation; strategy: the engine binary serves the page over HTTP because a browser blocks module scripts over `file://`, the page measures its own lag because no wire message announces it and the browser exposes no inbound-queue depth, the whole match decodes into absolute `Int16Array` arrays at 24.2 MB for an O(1) rewind, and rendering stays on the main thread because the draw is sub-millisecond against a 16.6-millisecond budget; key risk: a high-refresh display making a frame count report 144 and pass for the wrong reason.

- **match-rules** (`04-plan-match-rules.md`): 54 files (16 new, 38 modified) across a new `crates/engine/src/rules` module, the loop, the snapshot, the protocol, the command line, three page files, and the content folder; strategy: pure law functions first, then one referee state machine inside the loop, a stoppage hook (`TickSink::on_stoppage`) that applies no queued change yet, a binary snapshot with a SHA-256 trailer that resumes tick for tick, and an announced maximum match length; key risk: a snapshot that misses one field and lets a resumed match drift.

- **calibration** (`04-plan-calibration.md`, planned before `tactics-and-ai` is planned or built): 32 files (12 new, 20 modified) across `schemas/observability/`, `content/`, the engine counters and snapshot, `observe`, a new `crates/engine-cli/src/calibrate` and `report` pair, and the tests. Strategy: shot, pass, possession, and xG counters at the two existing resolution points; the contract's statistics keys added to `match-stats` at schema version 1; three JSON Schema files that the default tests check real output against with `boon` 0.6 (test-only); `engine-cli calibrate`, which runs an equal-strength suite and a 15 percent strength suite of 1000 matches each in worker processes and writes one `run-report` with the band checks, the dark paths, and the single-thread figure; and a capped tuning pass on `content/tuning.json` against the bands in `content/realism-bands.json`. Key risk: the bands may be unreachable with the tactics slice's decision model. A miss becomes an open tuning item, and the bands are never widened. Step 1 re-reads the tactics seams and stops for a plan re-review if they differ. Shared-file notes: `observe/mod.rs`, `sim.rs`, `snapshot.rs`, `stream_run.rs`, and `content/tuning.json` are also touched by earlier plans, always in sequence. Every new `Summary` field enters the snapshot in the same step.

- **commentary** (`04-plan-commentary.md`): 20 files (6 new, 14 modified) across a new `crates/engine/src/commentary` module, `content/commentary/en.json`, three engine seams, the protocol, and the shared match driver. Strategy: a per-match commentator in the engine crate reads engine events outside the tick loop. The shared driver in `stream_run.rs` attaches its line to the event message as one optional `commentary` field, and protocol version 2 stays under the additive-field rule. The lines are data, with conditions on minute band, score state, form, and repeat count, and the loader refuses a file that gives any kind fewer than 3 unconditional lines. Selection is deterministic, takes no engine random draw, and does not repeat a line for a kind within ten minutes of play. Goal events now name the last kicker, and restart and kick-off events name the taker. The last kicker is cleared at every stoppage, so the snapshot layout does not change. Key risk: repetition over about 40 throw-ins a match, measured by the distinct-lines criterion.

- **experiment-flags** (`04-plan-experiment-flags.md`, deferred; planned before `calibration` and `integration` are built): 23 files (4 new, 19 modified) across a new `crates/engine/src/flags.rs`, the content loader, `MatchConfig`, `observe`, the calibrate command and report, the two record schemas, and the modder documents. Strategy: an optional `flags` block in `content/tuning.json`, with no tuning version change. Each flag requires a non-empty owner, hypothesis, and removal condition, and a refusal names the flag through the `garde` map-key path. A flag has an effect through overrides of tuning values, which are validated again as a whole file, or through a code registry, which is empty when the slice lands. A flag with neither is refused. Flags resolve once at load, so the tick loop does not change. `calibrate --pair <name>` runs both arms on the same fixtures and seeds and writes one `run-report` with the arms, one comparison row per suite and band, and a verdict. The verdict does not set the exit code. No flag ships. The experiment design is `04c-experiment.md`. Key risk: the calibration harness shapes change when that slice lands. Step 1 stops for a plan re-review if they differ.

- **tactics-and-ai** (`04-plan-tactics-and-ai.md`): 49 files (13 new, 36 modified) across a new `content/tactics.json`, the data loaders, a new `crates/engine/src/tactics` module with `ai.rs` and `fatigue.rs`, the loop, the snapshot, the protocol event types, and the command line. Strategy: the engine owns the change queue and applies it inside the tick that opens a stoppage, so a substitute's entry lands on a restart tick; a per-team plan is computed once per tactics change and read by a scored-options decision layer; energy drains with effort and a curve lowers pace and decision values; an injury opens the `injury` stoppage with a dropped ball; half-time substitutions use no window (`windows_exempt`, rule pack schema 3); the AI manager queues through the same queue and draws no random numbers; snapshot version 2. The socket's `queue-change` detail stays opaque, because its format is the product owner's open question OQ-2 in `04-plan-viewer-lineup-tactics.md`. Key risk: processor time per match within 10 percent of 418.8 ms on `837cb5c`, judged per match as the slice states it, with per tick reported beside it.

- **scripting-runtime** (`04-plan-scripting-runtime.md`, deferred; built after tactics-and-ai, commentary, and calibration): 45 files (23 new, 22 modified) across a new `crates/script` crate, a new `crates/engine/src/plugin.rs`, three engine call sites, the protocol, the observability schema files, the command line, and `content/scripts/`. Strategy: the engine owns a runtime-free, versioned plugin interface (decision, rule, and commentary hooks), and the new crate implements it with Rhai 1.26, which is pure Rust because no C compiler is present. The sandbox refuses every import, disables eval, routes print to the log, and uses a deterministic operation budget with a 2 ms backstop. A failed hook keeps the native choice and records a `script` event. Decisions still run every tick: the script returns option-score offsets that are cached and refreshed on a carrier change, after a stoppage, or every 25 ticks. The pack hash is folded into the content hash, so the snapshot layout does not change. An additive `script` event type and `script.*` fields keep protocol version 2. Key risk: the 10 percent processor-time budget; step 2 measures the call cost first and stops at 3 microseconds per call.

- **distribution** (`04-plan-distribution.md`, deferred; built after `integration` is verified): 16 files (11 new, 5 modified) across `.cargo/`, the launcher in `crates/engine-cli`, two new command-line tests, `packaging/windows/`, `packaging/unix/`, and the documentation. Strategy: the installed layout describes itself (the binary with `content/` and `web/` beside it), so no configuration is needed; the planned `engine-cli launch` gains optional `--seed` and `--web` and a new `--open`; the Windows C runtime is linked statically; a per-user NSIS installer adds a Start-menu shortcut to `launch --open` (product owner, OQ-2 = A); a Linux x86_64 `.tar.gz` with a start script is built and smoke-tested in WSL Ubuntu 24.04, and macOS is a pre-registered deferral (product owner, OQ-1 = B); Windows Sandbox is the clean Windows 11 machine. Key risk: SmartScreen on the unsigned download, which the sandbox run does not reproduce. Shared-file note: it extends `launch.rs`, `cli.rs`, and `web.rs` after the reports-and-recovery slice lands them, and Step 3 stops for a plan re-review if they differ.

- **viewer-match-day** (`04-plan-viewer-match-day.md`): 28 files (12 new, 16 modified) across `web/` (six new page modules, a panel stylesheet, five new test files), `crates/protocol`, `docs/reference/protocol.md`, the shared match driver and three commands in `crates/engine-cli`, and two stream test files. Strategy: every event, statistics, and energy message is stored by tick and released by the rendered tick, so the score, banner, feed line, and commentary line land in the same frame as the goal tick, and rewind restores the earlier state. The wire additions are the product owner's Q-1 option A: a `roster` on each hello team (starters in wire-slot order, then the bench, each with its squad index), nine `stats.*` pairs on the `stats` message sent once per simulated second and at full time from `MatchFigures` and `Summary`, and a new `condition` message with 22 energy values. This slice raises `PROTOCOL_VERSION` to 3. No engine crate change is needed. Key risk: messages that arrive up to 500 ticks ahead of playback; second risk: the added message volume against the 8.55 MB stream memory tripwire.

- **extra-time-penalties** (`04-plan-extra-time-penalties.md`, deferred; built after `integration`): 33 files (3 new, 30 modified) across the rule pack (schema 4), `crates/engine/src/rules` (a period clock, a new `shootout.rs`, shoot-out restart targets), `decision.rs`, the change queue, `sim.rs`, snapshot version 4, `observe`, commentary, the protocol, the command line, and the page scrubber. Strategy: a `--knockout` switch that is off by default, so the default match is byte-identical; a level knockout match plays two 15-minute periods through the half-time path with no energy recovery; a level extra time goes to a shoot-out played on the pitch, one penalty dead ball per kick, with the taker always shooting through the extracted open-play shot code (product owner, Q-1 = B); the announced length of a 90-minute knockout match is 530,000 ticks, including a 10-round allowance; shoot-out state is stored in the snapshot, because every kick is a stoppage. The slice raises no protocol version: its fields are optional extras under whichever version is current. Key risk: a kick that never ends, guarded by a 5-second live cap and a 100-round safety cap. Shared-file note: `web/main.mjs`, `web/history.mjs`, `stream_run.rs`, and `docs/reference/protocol.md` are also touched by `viewer-match-day`. This slice lands after it, so it applies its small edits to the landed files.

- **probe-engine-core** (`04-plan-probe-engine-core.md`, a probe slice; built after every slice it touches is verified): 14 modified files across `crates/engine-cli` (help text, the error branch of `main.rs`, `bench.rs`, the calibrate worker, run, and report), `crates/engine` (`error.rs`, `observe`), the two record schemas, the tests, and the command reference. Strategy: fit every `-h` line in 80 columns by one rule and test `-h` and `--help` on all 10 surfaces; add regression tests for the three findings already fixed; print one `outcome: error` record on a failed `simulate` or `bench` run (Q-1 = A); change both schemas to `success` or `error` with the statistic keys required only on a record that is not an error, and move calibration from `failure` to `error` with the error keys (Q-2 = C); a hidden `calibrate --inject-failure` seam lets verify drive both calibration error paths. Key risk: calibration's output changes after its verification; earlier run folders are not a compatibility target (product owner).

- **realism-bands-v2** (`04-plan-realism-bands-v2.md`, extension round 1; built after every earlier slice): 22 modified files across `content/` (bands version 2, six new formations), `schemas/observability/`, `observe`, the calibrate command and its report, tests, and documents; strategy: no engine change, eleven sourced bands on the equal suite, `stats.throw_ins` and `stats.goal_kicks` in the record, and a formations suite in `--suite all` that plays all 55 pairings of 10 formations with 1,000 matches each against the three goal bands; the five-seed baseline of every suite is recorded as the yardstick for the next three slices; key risk: a full run takes about 74 minutes per seed.
- **defending-and-discipline** (`04-plan-defending-and-discipline.md`, extension round 1; after realism-bands-v2): 26 files (3 new test files) across `team.rs`, `decision.rs`, `sim.rs`, the rules, the computer manager, tuning, the snapshot and two reference documents; strategy: defending first (goal-side cover on the most advanced central attacker, pressers at the intercept point, a back line capped at one tuned gap per space, one fewer presser per player out, no lone forward recentred), then discipline (a per-player foul cooldown carried in snapshot version 5, one held card per player per advantage spell, a booked-player factor). A derived acting keeper (`Team::keeper_slot()`) replaces every fixed slot-0 check, and the computer manager brings on its bench keeper after a keeper's red card. Five bounded tuning values are the only levers of the tuning loop; the benchmark is re-baselined at 1.4753 µs per tick; key risk: seven of nine pairings against 4-4-2 exceed 4.0 goals for one side today (worst 11.60), and a pairing that cannot be fixed inside the bounds stops implement.

- **tuning-loop** (`04-plan-tuning-loop.md`, extension round 2; complete after the product owner's Q-P1 answer): 18 files (3 new) across the calibrate parent, fixtures and worker, the report (a new baseline module), the command line, one public engine send-off before kick-off, the run-report schema, the build profile, tests and two documents. Strategy: targeted runs keep each fixture's full-list index so their figures equal a full run's; every band check carries a sampling error and the diff marks a change inside two errors as noise; the red-card experiment becomes a `red-card` suite outside `--suite all` on the slow test's own seeds; a calibrate profile is kept only if faster and identical per seed; a baseline is refused on a different seed, match count or `fixtures.hash` (a new additive report key over the attributes, rules and tactics content, the generator block and the default clubs), while a different `content.hash` is the change under test (product owner, Q-P1). Key risk: the 2-minute budget for 1,000 matches plus the diff, measured on the reference machine.

- **lone-forward** (`04-plan-lone-forward.md`, extension round 2; after tuning-loop and defending-and-discipline): 17 files (1 new test file) across `decision.rs`, `team.rs`, `rules/offside.rs`, `rules/fouls.rs`, `sim.rs`, tuning, tests and one reference document. Strategy: only the two levers RIM-12 allows. A carrier with no outfield team-mate ahead gets three weighted terms in the scored options (a lay-off bonus on passes behind him and, when pressed, a hold-up bonus and a dribble cost); off the ball the team's lone forward holds the referee's own offside line; the fixed 0.05 won-tackle share becomes a bounded `tackle_win_base`, with `foul_base` as the foul side and the card chances untouched. The five values land at neutral values first (zero-change red-card diff), then the tuning loop moves them against baselines copied into the slice's evidence. No snapshot, schema or protocol change. The benchmark is re-baselined at 1.5422 µs per tick. Key risk: both levers at their bounds may leave an arm or a pairing failing, which stops implement with the figures.

- **keeper-and-shots** (`04-plan-keeper-and-shots.md`, extension round 1; after lone-forward): 14 files (2 new: `shot.rs` and a test file) across `sim.rs`, `rules/mod.rs`, `decision.rs`, `scenario.rs`, tuning, tests and one reference document. Strategy: on target is decided at the kick by stepping a copy of the ball to the goal line; the acting keeper attempts a save only on a shot on target, with one roll on the sourced curve (0.891 to 0.272 by shot quality, read from a fixed quality model), then holds or parries; an outfield defender near the ball can block once per flight; parries and blocks are deflections with the defending side as the last touch, so corners come only through the existing out-of-play rule (RIM-9); shots can rise over the bar; penalties and the shoot-out share the keeper model with a sourced penalty xG, and the three hand-set shoot-out constants move into the tuning file; the xG coefficients are refitted last. No event kind, `Summary` field, snapshot field, report key or protocol message is added. The benchmark is re-baselined at 1.613 µs per tick. It also carries the red-card criterion moved by the product owner (Q-LF2). Key risk: that criterion's cause is how a ten-man side attacks, so it may still fail, which stops implement with each arm's figures.

- **tempo-and-restarts** (`04-plan-tempo-and-restarts.md`, extension round 1; after keeper-and-shots): 19 files (1 new test file) across `decision.rs`, `sim.rs`, `rules/mod.rs`, `snapshot.rs`, `observe/mod.rs`, the calibrate report, the two record schemas, `scenario.rs`, tuning, tests and three documents. Strategy: an unpressed carrier pays a tuned cost on a pass and a clearance for a short carry window after he gains the ball, so every agent still decides every tick; the throw-in, goal-kick, corner and free-kick delays move to the sourced medians (13.8, 23.2, 31.8, 32.5 s) and never above them; a clearance is its own kick and the first kick of a restart taker is a restart kick, each under its own `match-stats` key, so `stats.passes` and pass accuracy cover open-play passes only; `stats.ball_in_play_s` is added and the calibrate report shows minutes per 90 as a figure, not a band; the snapshot moves to version 6; a defender can clear a fast pass inside his own penalty area once per flight by deflection, which gives the corners from clearances and crosses moved here (Q-KS2); throw-ins, measured at 20 to 24 per match, are raised only by physical levers. The benchmark is re-baselined at 1.5519 µs per tick. Key risk: the engine awards fewer stoppages than real matches, so the ball may stay in play above 65 minutes at the medians, which stops implement with the figures.

Four slices are not yet recorded as planned in this index: three from the first round and `realism-tuning`, the last slice of extension round 1.
- **realism-tuning** (`04-plan-realism-tuning.md`, extension round 1; after tempo-and-restarts): 14 files (1 new test file) across the tuning file, `sim.rs`, possibly `decision.rs`, tests, two browser specs and the documents. Strategy: a new corner source (a pressured clearance or cross clearance near the own goal line going off the defender, RIM-9 kept) lands first at 0.0 and is proven not to change play; then a feasibility stage tests corner throughput, the opposed formations, the discipline window and the red-card arms in minutes each; then the values are tuned jointly from `b0` within 8 rounds on a fast server loop (the xG refit last), checked on five seeds, and gated on equal and strength on five seeds, the ten formations against 4-4-2 on 2.4 to 3.2 goals, the red-card test and every slow test (Q-RT1 to Q-RT4). The 55-pairing suite is recorded, not gated. Key risk: no setting passes every gated band on five seeds; the slice then stops and reports the closest result and the conflicting pair, and never widens a band.

## Cross-Cutting Concerns

- Conventions fixed by the first plan bind every later slice: edition 2024, clippy and rustfmt as blocking gates, dual MIT OR Apache-2.0 license, thiserror in libraries and anyhow in binaries, `glam` for vectors, no `HashMap` on any simulation path. The second plan adds: rust-version 1.87, `garde` for every content struct, `#[serde(deny_unknown_fields)]` without `#[serde(flatten)]`, JSON for every content file.
- Content: every data file carries an integer `schema_version` checked before deserialization; the constants live in `crates/engine/src/data/*.rs`; `content/README.md` is the modder reference and must change with any field.
- Identity: `owner.id` is created once under `SM_DATA_DIR`; `match.id` is `{seed:016x}-{millis}`; both ride in `stats.json` and the tick-file header (schema 2). Later slices reuse `observe::identity`, never a second source.
- Observability: every record kind follows `.ai/observability.md` plan-version 1; `MatchStats` and `RunReport` in `crates/engine/src/observe/mod.rs` stay the single source of the JSON keys; `content.hash` and `teams` are additive extras until the observability audit settles them.
- Benchmark tripwire: every engine slice after engine-core reruns `engine-cli bench --seed 42 --matches 5 --json` and the criterion bench; verify compares against `05c-benchmark.md`, re-baselined per slice with the prior record under `history/`.
- Design, fixed by the fourth plan: `02c-craft.md` is authored and both design gates are closed — `image-gate: pass` on `steer.md`, and the brief-confirm gate on a user-confirmed PRODUCT.md. The product is named Touchline, so the token prefix is `--tl-` and the `--mv-` prefix in `02b-design.md` and `02b-design.yaml` is stale, not in conflict. `web/tokens.css` is the single source of every colour, radius, spacing, font and easing value, and DESIGN.md carries the same rows.
- Frontend conventions, fixed by the fourth plan: the page lives in `web/` at the repository root; modules carry the `.mjs` extension and are run by `node --test` with no `package.json`, so the repository stays a pure Cargo workspace; relative imports always name their extension, which is the one spelling Node and the browser both accept; plain HTML, CSS, and JavaScript with no framework, no component library, and no build step.
- Page delivery, fixed by the fourth plan: the engine binary serves the page. A browser blocks module scripts over `file://` absolutely, so `--web <dir>` on `serve` and `replay` starts a static server beside the socket, on an origin the WebSocket allowlist already accepts. Every response carries `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`, which the page's memory gauge requires and which no later slice may remove without breaking it.
- Page-side observability has no transport. The contract's file sink and socket feed both reach only the engine. Every page signal is a JSON Lines row on `console.info` plus a ring buffer on the page test hook, and `record.kind: viewer-event` is a fifth kind emitted as an additive extra. `/wf observability` owns closing the gap.
- Experiment augmentation: authored as `04c-experiment.md` by the `experiment-flags` plan (paired calibration, four bands as the metric, guardrails on dark paths, violations, the 10 percent tripwire, and the stronger-team win rate). Its `augmentations:` entry in `00-index.md` is left to the run driver. Flags are content: a flag state that differs from the file enters the content digest, and every `match-stats` record carries `tuning.flags_on`.
- Probe carry-overs: findings 1, 3, and 4 of `03-slice-probe-engine-core.md`, and finding 5 under `--help`, landed in the data-schemas-generator command-line step. The `-h` residual of finding 5 and finding 2 are planned in `04-plan-probe-engine-core.md`. Finding 2 changes the record contract: a failed `simulate` or `bench` run prints one `outcome: error` record, both record schemas accept `success` and `error` only, and calibration writes `error` in place of `failure` (product owner, Q-1 = A and Q-2 = C).
- Transport, fixed by the third plan: a browser page can open a WebSocket or an HTTP connection and nothing else, so every engine-to-page path is WebSocket over loopback. `tungstenite` 0.30 blocking, one thread per client, no async runtime anywhere in the workspace. The bind address is `127.0.0.1`, never `0.0.0.0`.
- Wire encoding (closes U-1): binary keyframe every 50 ticks plus signed-byte centimetre deltas between, about 13 MB per match. `crates/protocol` is the single source of the frame layout; the viewer decoder mirrors it, and no second encoder exists.
- Crate boundaries per NFR-8: `crates/engine` stays network-free so a later WebAssembly build stays open (RIM-3); `crates/protocol` carries no input or output; `crates/stream` owns every socket, thread, and file the stream needs.
- Threads: the workspace is single-threaded until this slice. Every thread it starts ends on a channel disconnect, and every test joins with a timeout.
- Match length, fixed by the fifth plan: `ticks_expected` in the opening message and `expected_ticks` in the tick-file header mean "at most" (regulation plus the rule pack's added-time cap for both halves, 360,000 ticks for a full match). `PROTOCOL_VERSION` is 2 and the tick-file schema is 4 because of that change of meaning. The real count rides in the trailer and the statistics record. A shortened match plays no added time.
- Protocol version 3, fixed by the match-day plan on the product owner's answers: `viewer-match-day` raises `PROTOCOL_VERSION` to 3 (the hello roster, the periodic `stats` message, and the `condition` message), and `viewer-lineup-tactics` adds its squad, lineup hold, `set-lineup`, and tactics schema under the same number on the same branch. Recorded files from version 2 no longer replay, so every later verification records a fresh fixture.
- Stoppages, fixed by the fifth plan and corrected by the tactics plan: `TickSink::on_stoppage(&Stoppage, &Simulation)` is the one hook where a stoppage becomes visible outside the engine, and snapshots are written through it. Queued changes are applied inside the engine, on the tick that opens the stoppage, before the record. The hook receives `&Simulation` and runs after the record (`crates/engine/src/record.rs:160`, `sim.rs:333-337`), so it cannot apply a change. No later slice adds a second outside stoppage path.
- Change queue, fixed by the tactics plan: `Simulation::queue_change(team, Change)` is the one way into the engine's queue, for the AI manager, tests, and later the page and the scripting runtime. The rule pack decides admission per stoppage kind, substitutions apply before tactics at a stoppage, and every change ends as an applied or rejected `tactics-change` event with `team.id`. The wire format of a change sent over the socket is not decided: it is the product owner's open question OQ-2 in `04-plan-viewer-lineup-tactics.md`.
- Tactics content, fixed by the tactics plan: `content/tactics.json` holds formations, mentalities, instructions, roles, duties, and the AI manager's thresholds, and it is part of the content hash. Fourteen attributes are required. The AI pre-match setup replaces the file-order lineup for both teams until the lineup editor sends one.
- Snapshots, fixed by the fifth plan: one binary `.smsn` file per match at `SM_DATA_DIR/matches/<match.id>/snapshot.smsn`, replaced atomically at every stoppage, with a SHA-256 trailer. A snapshot resumes tick for tick on the build and content that wrote it and is refused with a named reason anywhere else. Any field added to `Simulation` by a later slice must be added to the snapshot in the same step, or the continuation test fails.
- Law events, fixed by the fifth plan: the engine supplies `minute` and `minute.added`; `MatchEvent` carries `player.id`, `player.secondary_id`, `card.kind`, `foul.advantage`, and `added_time.s` as optional fields. `card.kind` and `foul.advantage` are additive extras until the observability audit settles them.
- Test scenes, fixed by the fifth plan: the `scenario` Cargo feature on `crates/engine` builds a match already in progress with scripted draws. Only tests enable it; release builds never contain it. Later engine slices reuse it rather than adding a second test seam.
- Realism measurement, fixed by the realism-bands-v2 plan: `content/realism-bands.json` is version 2 and refuses an older file. Every band change needs a recorded product-owner answer. The formations suite is part of `--suite all`, so every full calibration covers every formation pairing. Later extension slices are judged on their own criteria and report their band misses against the recorded baseline; only `realism-tuning` must pass every band.
- The acting keeper, fixed by the defending-and-discipline plan: `Team::keeper_slot()` and `Simulation::keeper(team)` are the one source of "who keeps goal". No code tests `slot == 0` or `team * 11` for the keeper after that slice; `keeper-and-shots` builds its save model on this seam. The snapshot is version 5 after that slice (each player's foul cooldown), and later slices take the next number.
- Output boundary: seven code comments that named workflow slices are rewritten in product language by the fifth plan's steps, and its last step searches the source for workflow vocabulary before the commit. Later plans keep that search.

## Integration Points Between Slices

- `TickSink` in `crates/engine/src/sim.rs` is the seam `stream-protocol` wraps for the socket feed and the recorded fixture.
- `MatchConfig::new(seed, minutes, &Content, [&TeamFile; 2])` replaces the built-in teams; `stream-protocol`, `match-rules`, and `calibration` build configs through it.
- `Content` (`crates/engine/src/data/mod.rs`) is the one loader: `RulePack` feeds `match-rules`; `FatigueTuning` and `Position` feed `tactics-and-ai`; `generate_league` feeds `calibration`.
- The `.ticks` header (schema 2) carries identity in the reserved bytes; the 192-byte record layout is provisional and `stream-protocol` decides the wire encoding (U-1).
- `EngineError` is the public error contract later crates match on; `Data` and `Version` are its content variants.
- `observe::identity` is the one source of `owner.id` and `MatchId` for the viewer slices; the hello message reuses it rather than minting an identifier.
- `crates/protocol` is the contract the viewer slices read: `MESSAGES` enumerates every message, and `docs/reference/protocol.md` is tested against it, so a viewer built on the document cannot drift from the code.
- The `.smfx` fixture and `engine-cli replay` are the prerequisite harness the shape's force-scope rule named: `viewer-pitch`, `viewer-match-day`, and `viewer-reports-recovery` verify against them before the engine is complete.
- `protocol::Queue` holds socket-side pending changes with their identifiers. The engine's own queue (`crates/engine/src/tactics/change.rs`) decides when a change applies, inside the stoppage tick (corrected by the tactics plan). The bridge between the two waits for OQ-2 in `04-plan-viewer-lineup-tactics.md`.
- The `match-event` record kind opens here with kick-off, goal, and full-time; `match-rules` adds nine event types and six optional fields, and `commentary` reads them.
- `hello.teams[]` is the one place the page learns a club's kit colours, from the fourth plan onward. `viewer-match-day` reuses the same two fields for the score bug and the club crests rather than reading team files.
- `web/tokens.css`, `web/components/match-control.css`, and `web/mark.mjs` are the three files every later viewer slice builds on. The fourth plan defines all four control sizes, five states, and both themes even though it uses two sizes, so no later slice adds geometry.
- `window.__touchline` is the page's observability seam: `lastRendered()`, `signals`, `history`, and `frame`. Three of this slice's acceptance criteria read it, and every later viewer slice extends it rather than adding a second hook.
- `web/interpolate.mjs` and `web/schedule.mjs` are where commitment C2 lives in the frontend. Every later viewer slice renders through the same scheduler; none may draw a position the engine did not compute.
- `engine-cli replay --sustain <x>` is a test harness for the lag notice and sits on `replay` alone. No later slice promotes it to `serve`.
- `TickSink::on_stoppage` and `Stoppage { tick, kind, team, spot }` announce a stoppage after the changes of that tick have applied. The tactics plan emits `Applied` and `Rejected` verdicts for the engine's queue, with `change.applied_tick`. `AppliesNow` stays unused until the page bridge exists.
- The tactics plan's `substitution`, `injury`, and `ai-decision` event types are what `commentary` must give lines to, what `viewer-match-day` shows as fatigue, injury, and substitution state, and what the AI-decisions dashboard reads. `injury` joins `STOPS_PLAY`. `calibration` tunes the `decision` and `fatigue` blocks, the injury rates, and the `ai` block.
- The law event types and `player.id` are what `commentary` reads. `viewer-match-day` reads `card.kind`, `added_time.s`, and `minute.added`, and hides the parked markers of sent-off players, which the tick record keeps at fixed spots beside the pitch.
- `commentary` on the event message is what the `viewer-match-day` feed shows, and the feed keeps its own minute stamp. `content/commentary/en.json` is the modding surface for the lines. Any slice that adds an engine event kind must add at least 3 unconditional lines for it, or the content test fails.
- `snapshot.smsn` and `engine-cli resume` are what `viewer-reports-recovery` builds crash recovery on. `MatchClock` in `crates/engine/src/rules/clock.rs` is what `extra-time-penalties` extends.
- `CODE_FLAGS` in `crates/engine/src/flags.rs` and `MatchConfig.flags` are where a competing model enters the engine. Its flag is declared in `content/tuning.json` in the same change, and a content test enforces this. `calibrate --pair` reuses the calibration parent, workers, `RunBuilder`, and `bench::measure` unchanged, apart from `measure` taking the `Content` it runs on.
- `web/stoppages.mjs` exports `STOPS_PLAY`, the list of event types that stop play. Any slice that adds an event type decides whether it joins the list.

## Recommended Implementation Order

1. `engine-core` — done.
2. `data-schemas-generator` — done.
3. `stream-protocol` — done.
4. `viewer-pitch` — done; the visible milestone.
5. `match-rules` — planned; the referee and the snapshot. It changes three `viewer-pitch` files after that slice is verified.
6. `tactics-and-ai`, `commentary`, `calibration` — the rest of the engine track in dependency order; `tactics-and-ai` is planned and applies its own change queue inside the stoppage tick.
7. `viewer-match-day`, `viewer-lineup-tactics`, `viewer-reports-recovery` — the viewer track.
8. `integration` — the full charter scenario.
9. `distribution` (deferred) — the Windows installer and the Linux archive; it packages the verified product, so it comes last.
10. `realism-bands-v2` (done), then `defending-and-discipline` (planned), `keeper-and-shots`, `tempo-and-restarts`, `realism-tuning` — extension round 1, strictly in order; bands first, so each later slice shows its progress against them.

## Conflicts Found

None. The fourth plan touches three files the third plan created — `crates/protocol/src/message.rs`, `crates/stream/src/server.rs`, and `crates/stream/src/replay.rs` — in sequence, after that slice is complete and verified, never in parallel. Two notes for implement. First, the two kit fields added to `hello` leave `PROTOCOL_VERSION` at 1, because no field changes meaning and both producers move in the same commit; the judgement is recorded in a comment beside the constant so a later reviewer sees it was made rather than missed. Second, the recorded fixture must be rebuilt once the kit fields exist, because the replayer now forwards the stored opening message verbatim; a stale fixture would draw grey markers with no error anywhere, and the byte-identity test is what turns that into a loud failure.

The fifth plan touches files four earlier plans created, always in sequence and after each slice is verified: `sim.rs`, `record.rs`, `validate.rs`, and `observe/mod.rs` from the engine slices; `crates/protocol/src/event.rs`, `lib.rs`, `command.rs`, and `message.rs` from the stream slice; and `web/stoppages.mjs`, `web/main.mjs`, and `web/tests/stoppages.test.mjs` from the viewer slice. One stale statement is corrected rather than followed: the comment at `crates/protocol/src/command.rs:1-3` and the earlier integration note said this slice applies the change queue, while `03-slice-match-rules.md` gives substitution windows to `tactics-and-ai`. The product owner kept the slice boundary (plan Round 1 Q1), so this slice opens the stoppage hook and applies nothing. Three shared-file notes for implement: `content/tuning.json` and `content/rules/default.json` both gain fields and their pinning tests in `crates/engine/tests/content.rs` change in the same step; the `.smfx` fixture and every `.ticks` file from before this slice are refused by version, so the local fixture is regenerated; and the page's history reservation rises to about 33.8 MB because the announced maximum is 360,000 ticks.

The three engine plans touch `crates/engine/src/record.rs` in sequence, never in parallel: engine-core wrote it, data-schemas-generator extended the header to schema 2, and stream-protocol adds a restart flag to the frame and a fan-out sink while leaving the 192-byte file record untouched, so the determinism and validator tests keep their meaning. Two shared-file notes for implement: `content/tuning.json` gains a fourth block and its pinning test must be updated in the same step, and `crates/engine-cli/src/cli.rs` gains three subcommands under the same 80-column help rule the second plan introduced.

The commentary plan and the calibration plan both change `crates/engine-cli/src/stream_run.rs`. The commentary plan adds the line to `match_event` inside `drive`. Calibration step 8 makes `match_event` a shared crate function, so that `simulate` can also write event files. Neither plan blocks the other, and the two run in sequence. The slice that lands second adds a `commentary` parameter to the shared mapping and builds a `Commentator` in every caller that writes events. Both plans also change `sim.rs`: the commentary plan adds `last_kicker`, which stays out of the snapshot because it is always `None` at a stoppage, and calibration adds `Summary` fields, which enter the snapshot. The two changes do not overlap.

The tactics plan records two conflicts with plans written in parallel. First, version numbers: the tactics plan raises the rule pack to schema 3 and the snapshot to version 2; `04-plan-extra-time-penalties.md` also numbers a rule pack schema 3 and a snapshot version 2; and `04-plan-calibration.md` raises the snapshot by one. The recorded order puts `tactics-and-ai` first, so it takes rule pack 3 and snapshot 2, and each later slice takes the next number when it lands. Second, statistics keys: the tactics plan adds `stats.shots`, `darkpath.change_never_applied`, `injury.count`, and `fatigue.mean_pct` to `match-stats`, and `04-plan-calibration.md` also plans `stats.shots` and `darkpath.change_never_applied`. The calibration plan's step 1 re-reads the landed seams and skips counters that exist, so it reuses these keys rather than adding them again. `stats.shots_on_target`, passes, possession, and xG stay with calibration, which defines them. The tactics plan also leaves the socket bridge to the product owner's answer on OQ-2 in `04-plan-viewer-lineup-tactics.md` rather than choosing a wire format in parallel.


The realism-bands-v2 plan changes files that `calibration`, `probe-engine-core` and `tactics-and-ai` created (`report/*`, `calibrate/*`, `observe/mod.rs`, the two record schemas, `content/tactics.json`). All three slices are verified, so the changes run in sequence, not in parallel.

The defending-and-discipline plan changes files that `match-rules`, `tactics-and-ai`, `extra-time-penalties` and `commentary` created (`rules/*`, `ai.rs`, `tactics/change.rs`, `snapshot.rs`, the commentary templates). All are verified, so the changes run in sequence. It adds a string value, `sub-keeper`, to `ai.decision`; the page hides `ai-decision` events, and both reference lists gain the value in the same change.

The tuning-loop plan changes files that `calibration`, `experiment-flags`, `probe-engine-core`, `realism-bands-v2` and `defending-and-discipline` created (`calibrate/*`, `report/*`, `cli.rs`, the run-report schema, `scenario.rs`, `tests/defending.rs`). All are verified, so the changes run in sequence. Its report keys are additive under schema version 1.

The lone-forward plan changes files that `engine-core`, `match-rules`, `tactics-and-ai` and `defending-and-discipline` created (`decision.rs`, `team.rs`, `sim.rs`, `rules/offside.rs`, `rules/fouls.rs`, the tuning file and eight test files). All are verified, so the changes run in sequence. Its tuning fields are optional with neutral defaults, so `TUNING_VERSION` stays 2, and it reuses the three slow criterion tests of `defending-and-discipline` unchanged.

The keeper-and-shots plan changes files that `engine-core`, `match-rules`, `extra-time-penalties`, `defending-and-discipline` and `lone-forward` created or changed (`sim.rs`, `rules/mod.rs`, `decision.rs`, `scenario.rs`, the tuning file, `acting_keeper.rs`, `match_stats.rs`, `rules_shootout.rs`). All are verified, so the changes run in sequence. It replaces the extra-time-penalties plan's hand-set shoot-out constants with tuning values at the same numbers and keeps that plan's kick-end rule (A-18). Its tuning block is optional with defaults, so `TUNING_VERSION` stays 2. It reuses the red-card slow test and the balanced fixtures of `lone-forward` unchanged.

The tempo-and-restarts plan changes files that `engine-core`, `match-rules`, `calibration`, `realism-bands-v2`, `lone-forward` and `keeper-and-shots` created or changed (`decision.rs`, `sim.rs`, `rules/mod.rs`, `snapshot.rs`, `observe/mod.rs`, `report/mod.rs`, the two record schemas, `scenario.rs`, the tuning file and four test files). All are verified, so the changes run in sequence. It takes snapshot version 6, the next number. It reuses the keeper-and-shots block and deflection functions and keeps that slice's set-piece floor as a criterion that must still pass. Its new tuning fields have defaults, so `TUNING_VERSION` stays 2.

The realism-tuning plan changes `content/tuning.json` and `Tuning::default()`, which every earlier engine slice reads, and it moves seeded expectations in tests those slices wrote. All are verified, so the changes run in sequence. It adds one tuning value with a serde default of 0.0, so `TUNING_VERSION` stays 2. It makes three `tempo-and-restarts` tests gating again and tightens the `defending-and-discipline` formations test.

## Freshness Research

See `04-plan-engine-core.md` § Freshness Research for `rand` 0.10, `HashMap` ordering, platform transcendental functions, and workspace inheritance; `04-plan-data-schemas-generator.md` § Freshness Research for `garde` 0.23, `serde` error behaviour, `deny_unknown_fields` with `flatten`, RUSTSEC-2026-0097, and the schema-version pattern; and `04-plan-stream-protocol.md` § Freshness Research for browser reachability, `tungstenite` 0.30 and its unbounded default write buffer, RFC 6455 masking and the Origin header, `sync_channel` semantics, and `DataView` decode cost. `04-plan-viewer-pitch.md` § Freshness Research carries the browser side: module scripts blocked over `file://`, loopback WebSocket exempt from Local Network Access, `bufferedAmount` measuring only the outgoing queue, `DataView` parity since V8 6.9, `OffscreenCanvas` available and not needed, `requestAnimationFrame` following the panel refresh rate, the cross-origin-isolation requirement on the memory measurement, Node 22's built-in test runner and `.mjs` resolution, and the SIL Open Font License on both typefaces. `04-plan-match-rules.md` § Freshness Research carries the laws: IFAB Laws 3, 7, and 11 to 17, 2024-25 foul and corner rates, the `rand_chacha` 0.10 word-position methods read from the installed source, `std::fs::rename` replace semantics on Windows, and the `serde_json` float-parsing caveat that the binary snapshot avoids.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine realism-tuning`. The realism-tuning plan is complete with no blockers. The product owner starts implement (steer.md).
