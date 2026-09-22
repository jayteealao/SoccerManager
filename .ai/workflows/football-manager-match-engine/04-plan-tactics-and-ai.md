---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: tactics-and-ai
status: complete
stage-number: 4
created-at: "2026-09-22T22:18:43Z"
updated-at: "2026-09-22T22:18:43Z"
metric-files-to-touch: 49
metric-step-count: 22
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, tactics, ai-manager, fatigue, substitutions, change-queue]
stack-source: confirmed
steering-honored:
  - "no visual change to the page; the one page change adds the injury event to the stoppage-index list, so the design direction in steer.md is untouched"
  - "every change verdict reaches the stream as the state word in change.state (queued, applied, rejected), so colour never carries the state alone when the viewer draws the chips"
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-tactics-and-ai.md
  shape: 02-shape.md
  instrument: 04b-instrument.md
  benchmark: 05c-benchmark.md
  observability: ../../observability.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-calibration.md, 04-plan-commentary.md, 04-plan-viewer-lineup-tactics.md, 04-plan-viewer-match-day.md, 04-plan-viewer-reports-recovery.md, 04-plan-integration.md, 04-plan-extra-time-penalties.md]
  implement: 05-implement-tactics-and-ai.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine tactics-and-ai"
---

# Plan: Tactics, Fatigue, and the AI Manager

## The Plan

The engine now plays a lawful match, but every team plays a fixed 4-4-2 with the first eleven players in its file. Nobody tires, nobody is hurt, and a queued change is acknowledged and never applied: the queue lives on the socket thread (`crates/stream/src/control.rs:98`), and the stoppage hook gives a sink a read-only `&Simulation` (`crates/engine/src/record.rs:160`), so no sink can change the match. The tree is green at 192 Rust tests and 46 page tests. The benchmark baseline on `837cb5c` is 418.8 ms of processor time per match, 1.4965 µs per tick, and 5.45 MB peak memory.

The engine owns the change queue and applies it inside the tick that opens a stoppage, so a change applies on that tick, and a substitute's entry lands on a restart tick that the validator already exempts from the jump check. The hook stays the one outside view of a stoppage, for snapshots. The page's changes do not reach this queue yet. The format of a change sent over the socket is an open product-owner question in the lineup-editor plan (its OQ-2), and no criterion of this slice needs it, so this plan leaves that format undecided. A new data file, `content/tactics.json`, holds four formations, five mentalities, six instructions, and roles and duties mapped to attributes. A per-team plan is computed once for each tactics change, so the decision layer reads it every tick at no extra cost. The carrier scores pass, dribble, shoot, clear, and hold with weights from the tuning file. Energy drains with effort, and a curve lowers pace and decision values below a threshold. An injury stops play for a dropped ball, which uses the `injury` stoppage kind the rule pack already names. Half-time substitutions use no window, which the rule pack states as `windows_exempt`. The AI manager uses the same queue as the human manager and draws no random numbers.

Implement touches 49 files, 13 of them new, in 22 steps: content, then the models, then the loop, then the snapshot, the event types, and the command line, then the tests and one measurement. Calibration tunes the weights next. Once the product owner answers the payload question, the lineup editor connects the socket to `Simulation::queue_change`. The top risk is the 10 percent tripwire on a tick that the decision layer makes heavier. The second risk is that the stronger-team criterion must pass before calibration.

## Current State

- Branch `feat/football-manager-match-engine` at `837cb5c`. Five slices are implemented and verified. `cargo test --workspace` passes 192 tests in 30 test binaries, with 0 failed and 0 ignored. `node --test web/tests/*.test.mjs` passes 46 tests. Both counts come from this run.
- `decision.rs:96-229` decides for the carrier with a shot threshold (`lane > 2.5` at `:132`), scored passes (`:171`), a dribble score (`:178`), and a goalkeeper clearance special case (`:204`). `decision_interval_ticks` is 1, a product-owner choice (`decision.rs:3`). Two opponents press (`:29-57`).
- `Team` (`team.rs:37-49`) has one formation constant, `FORMATION_442`, and no tactics. `Team::from_file` starts the first eleven entries of the file (`team.rs:89-113`). The shipped squads have 22 players (`content/tuning.json:58`).
- `Player` (`player.rs:344-363`) has no energy and no squad identity. `Simulation::player_ids()` maps `slot` to `team.player_ids[slot]` (`sim.rs:276-281`), which is wrong after a substitution. `stream_run.rs:44` reads the ids once, before kick-off.
- `Derived` reads seven required attributes (`data/attributes.rs:181-189`). The shipped schema has 36 attributes, including `finishing`, `vision`, `decisions`, `composure`, `stamina`, `natural_fitness`, and `injury_resistance` (`content/attributes.json`).
- `FatigueTuning` has `minutes_to_half_stamina` and `recovery_per_day`, and "the engine does not read them yet" (`data/tuning.rs:100-110`).
- The rule pack has `substitutions { limit: 5, windows: 3 }` and nine stoppage kinds with admission flags. `injury` admits both kinds of change and adds 60 s (`content/rules/default.json`). No code opens an injury stoppage. `restart.rs` sends an unknown kind to the free-kick branches (`restart.rs:59`, `:73`, `:197`).
- `open_dead_ball` sets `restart = true` and `self.stoppage` inside `step` (`rules/mod.rs:280-325`). `run` calls `on_stoppage` after `on_tick` (`sim.rs:331-341`), with `&Simulation`.
- `protocol::Queue` (`command.rs:97-150`) assigns `q-{tick}-{n}` ids and checks admission. `QueueChange.detail` is opaque JSON (`message.rs:93-95`). `MatchEvent::change` sets `team.id` to `None` (`event.rs:220-229`).
- The observability contract already reserves `event.type` values `injury`, `substitution`, and `ai-decision`, and the keys `change.applied_tick`, `ai.decision`, `fatigue.mean_pct`, `injury.count`, `stats.shots`, and `darkpath.change_never_applied` (`.ai/observability.md:27-59`).
- The snapshot body stores positions, status, and yellow cards per player (`snapshot.rs:485-492`). It stores no squad identity, energy, tactics, or substitutions.
- The page labels markers by slot number 1 to 11 (`web/pitch.mjs:221-224`). A substitution in place keeps its marker.
- Baseline on `837cb5c` (three drives): processor time per match 418.8, 418.8, and 415.8 ms; per tick 1.4965, 1.4965, and 1.4858 µs; wall time 421, 421, and 419 ms; 279,850 ticks per match; peak memory 5.469, 5.453, and 5.445 MB; tick step 1.5264 µs. Evidence is in `bench-baseline/tactics-and-ai/`.
- Another session's release test run recorded one failure in `crates/engine-cli/tests/stream_cli.rs:174` ("the viewer disconnected"; `04-plan-calibration.md` Current State). The debug run in this session passed that test, so the test may be timing-sensitive. Implement reruns it once before treating a failure there as its own.
- The index holds staged files from other work: the design documents under `docs/design/realism/`, and `crates/engine/tests/zz_stall_probe.rs`, which is staged as added and deleted from the working tree (git status at session start).

## Simplicity Ladder

| Capability | Rung | Choice |
|---|---|---|
| Tactics data file and validation | rung 3 reuse | `load_json` in `data/mod.rs` and `garde` rules, as the other three content files use. No new dependency. |
| Tactics model and per-team plan | rung 4 new code | No tactics type exists. It copies the index-into-schema idiom of `StoppageKind` (`data/rules.rs:11-53`). The plan is computed once for each change, so no per-tick lookup occurs. |
| Formations | rung 3 reuse with modification | `Team::formation` and `anchor()` (`team.rs:41`, `:165-176`) read slot coordinates from the schema. The 4-4-2 entry equals `FORMATION_442`. `reshape()` already works on any line layout. |
| Scored-options decision layer | rung 3 reuse with modification | `decide_carrier` (`decision.rs:96-229`) already scores passes. It is generalised into five option kinds with tuned weights. `segment_distance` and `toward` (`math.rs`) are reused. |
| Change queue in the engine | rung 4 new code | `protocol::Queue` (`command.rs:97-150`) cannot mutate the match, and the engine crate cannot depend on the protocol crate (NFR-8). The engine queue copies its id format `q-{tick}-{n}` so a later socket bridge can carry ids through unchanged. |
| Fatigue curve | rung 4 new code | A piecewise-linear lookup over at most eight points. No in-repo interpolation helper exists. |
| Injury stoppage and dropped ball | rung 3 reuse with modification | `open_dead_ball`, `restart::spot`, `target`, `taker`, and `delay_ticks` gain the `Injury` kind. The parking spot for a sent-off player (`pitch::parking_spot`) takes injured and substituted-off players. |
| Substitution in place | rung 3 reuse | The roster slot and the 22-position record stay. The incoming player replaces `attributes`, `derived`, `shirt`, and `squad` at that slot. `Team::reshape` already handles ten players, and `restore` is its inverse. |
| AI manager | rung 4 new code | No manager logic exists. Threshold rules with no random draws keep the generator stream the same as a match without the AI. |
| Snapshot version 2 | rung 3 reuse | The `Writer` and `Reader` helpers and the version refusal in `snapshot.rs:36`, `:141`. |
| Parallel slow tests | rung 1 stdlib | `std::thread::scope` in the test helpers. Each match still runs on one thread. |

## Applied Learnings

No applicable learnings found: `.ai/solutions/INDEX.md` does not exist and `.ai/sdlc-config.json` does not exist. `runtime-evidence-deferrals` in `00-index.md` is empty, so the repeat-deferral tripwire does not fire.

One in-repo design record agrees with this plan and changes nothing in it. `docs/design/realism/01-engine-realism.md:581` records that the AI manager's live changes use the same change queue as the human's and take effect at the next stoppage.

## Likely Files / Areas to Touch

- `content/tactics.json` (new), `content/tuning.json`, `content/rules/default.json`, `content/README.md`: the tactics schema, the fatigue and injury tuning (version 2), `windows_exempt` (rule pack version 3), and the modder reference.
- `crates/engine/src/data/tactics.rs` (new), `data/mod.rs`, `data/attributes.rs`, `data/rules.rs`, `data/tuning.rs`: the loader, the cross-file role check, the 14 required attributes, and the new fields.
- `crates/engine/src/tactics/mod.rs`, `tactics/change.rs`, `ai.rs`, `fatigue.rs` (new): the model, the queue, the AI manager, and fatigue with injuries.
- `crates/engine/src/decision.rs`, `player.rs`, `team.rs`, `sim.rs`, `rules/mod.rs`, `rules/restart.rs`, `tuning.rs`, `snapshot.rs`, `scenario.rs`, `validate.rs`, `observe/mod.rs`, `lib.rs`: the loop and its seams.
- `crates/engine/tests/`: 7 new criterion files (3 of them slow and ignored by default), plus `common/mod.rs`, `content.rs`, `snapshot.rs`, `full_match.rs`, and `validator.rs`.
- `crates/protocol/src/event.rs`, `command.rs` (comment only), `message.rs`, `lib.rs`, and `docs/reference/protocol.md`: three event types and two fields the observability contract already reserves. The queue-change detail stays opaque.
- `crates/engine-cli/src/stream_run.rs`, `serve.rs`, `simulate.rs`, and `resume.rs`.
- `web/stoppages.mjs`, `web/tests/stoppages.test.mjs`: `injury` joins `STOPS_PLAY`.
- `README.md`.

Full topology with line estimates: `04-plan-tactics-and-ai.yaml`.

## Proposed Change Strategy

The **stoppage-gated change queue** (named mechanism, shape "Stoppage-gated change queue driven by a rule pack") moves into the engine. `Simulation::queue_change(team, Change)` takes a change from the AI manager or a test, and later from the page once the socket payload is decided. `step` applies the queue on the tick that opens a stoppage. It runs after the referee and the clock and before overlap resolution, so the change lands on the tick the record marks as a restart. Substitutions apply first, in queue order, and then tactics changes. The rule pack decides admission: a change waits for a stoppage whose kind admits it. The `penalty` kind admits neither kind of change. Each verdict becomes an engine event, `ChangeApplied` with its applied tick or `ChangeRejected` with a reason. `TickSink::on_stoppage` still announces the stoppage after the record, and the snapshot written there includes the changes just applied.

The **substitution limit** reads the rule pack. A window is a stoppage where a team makes at least one substitution. A stoppage kind listed in `windows_exempt` (half-time) uses no window. The limit counts every substitution. Rejection reasons are ordered: limit reached, no window left, player not on the pitch, player not on the bench or already played, player injured. The conflict rule is part of the same pass. A tactics change that names a role for a player substituted off at this stoppage is rejected whole with a reason that names the player.

The **scored-options decision layer** (named mechanism, slice Scope) replaces the branches in `decide_carrier`. Each option's score is `features · weights + plan offsets + noise`. The weights come from a new `decision` block in the tuning file. The offsets come from the team's plan (mentality, tempo, directness, role, and duty). The noise width shrinks as effective `decisions` and `composure` rise. Pressing sets how many opponents press and from how far. Width, line height, mentality, and duty move the anchors. Decisions still run on every tick, as the product owner chose.

**Fatigue** is energy from 1.0 down, drained each tick by effort and scaled by stamina. Every 50 ticks the effective values are recomputed from the unfatigued base values and the curve multiplier. **Injuries** roll once per tackle for the tackled player and once per minute per active player, scaled by `injury_resistance`. An injured player leaves play at once and parks beside the pitch. The team reshapes to ten, and the referee opens an `injury` stoppage with a dropped ball (IFAB Law 8). The **AI manager** (named mechanism, shape) checks every 30 simulated seconds and at every injury. It queues through the same queue and emits an `ai-decision` event for each choice.

Performance: NFR-1 governs the mechanism choices and `yields-to: C2`. The plan is precomputed, effective values refresh once a second, and the AI and injury checks run once per interval, so the added per-tick work is one energy update per player and a wider option loop for the carrier. The slice criterion is judged as written: processor time per match within 10 percent and peak memory within 25 percent of the baseline. Processor time per tick and ticks per match are reported beside it.

## Step-by-Step Plan

1. **Content schema.** In `data/attributes.rs`, add `finishing`, `vision`, `decisions`, `composure`, `stamina`, `natural_fitness`, and `injury_resistance` to `REQUIRED`. In `data/rules.rs`, set `RULES_VERSION` to 3 and add `Substitutions::windows_exempt` with a rule that names each kind once. In `data/tuning.rs`, set `TUNING_VERSION` to 2 and rewrite `FatigueTuning` with `threshold`, `curve`, `drain_base_per_s`, `drain_effort_per_s`, `half_time_recovery`, and `recovery_per_day`, with garde ranges. Update `content/rules/default.json` and `content/tuning.json` with the values in `04-plan-tactics-and-ai.yaml`. Update `tests/content.rs`. Run `cargo test -p engine --test content`. All tests must pass.
2. **Tactics schema.** Write `data/tactics.rs` and `content/tactics.json` with the formations, mentalities, instructions, roles, duties, and `ai` block listed in the YAML. In `data/mod.rs`, add `TACTICS_FILE`, load it, include it in the digest, and check that every role attribute is in the attribute schema. Refuse a failure naming the file, the role, and the attribute. Unit test: the shipped file loads; a bad role is refused by name; a formation with no goalkeeper in slot 0 is refused.
3. **Tactics model.** Write `tactics/mod.rs`: `Tactics`, `TacticsPatch`, `RoleDuty`, and `TeamPlan::from(&Tactics, &TacticsSchema, &Tuning)`. Unit tests: balanced 4-4-2 with no roles gives zero offsets and the `FORMATION_442` coordinates; attacking moves the block forward; high pressing gives three pressers.
4. **Players and teams.** In `player.rs`, add `squad`, `energy`, `base`, the `Injured` and `SubstitutedOff` statuses, and the new `Derived` fields. In `team.rs`, add `tactics`, `plan`, `lineup`, `bench`, the formation from the schema, the plan offsets in `anchor()`, and `restore(slot)`. Keep `anchors_stay_inside_the_pitch_for_every_corner` passing for every formation and mentality.
5. **Match configuration.** In `sim.rs`, add `Manager { Ai, Human }` and `MatchConfig::managers`, which defaults to both AI. `MatchConfig::new` keeps its signature and runs the AI pre-match setup for both teams. Add `with_manager(team, Manager)` and `with_tactics(team, Tactics)`. `player_ids()` reads each slot's squad index.
6. **Fatigue.** Write `fatigue.rs`: the per-tick energy drain, `multiplier(energy, curve)`, the 50-tick refresh of effective values from `base`, and half-time recovery. Call it from `step` after the referee. Unit tests: the curve at each point and between points; above the threshold the multiplier is 1.0; a player standing still drains only the base rate.
7. **Injuries.** In `fatigue.rs`, add the per-tackle and per-minute rolls, which use `EngineRng::referee_draw` so a scene can script them. In `rules/mod.rs`, add `injure(i)`: status, parking, reshape, the injury event, and an `Injury` stoppage. A foul on the same tick keeps its free kick. In `rules/restart.rs`, add the dropped-ball spot, taker, 4 m targets, and the `drop_ball` delay. Unit tests for the restart geometry.
8. **Decision layer.** In `tuning.rs`, add `DecisionWeights`. In `decision.rs`, replace the shot threshold, the pass, dribble, and clearance branches with the scored options. Pressing sets the press count (1, 2, or 3) and the press distance scale. Apply the time-wasting factor to the restarting team's delay while it leads. Keep `a_lofted_pass_reaches_its_target_slow_enough_to_control` and `an_open_teammate_draws_a_pass_or_a_shot` passing. Add a unit test: an attacking mentality raises the shoot score for the same scene.
9. **Shots counter.** In `sim.rs`, count a shot per team in `Summary` when a `Kick::Shot` is applied. Shots on target, passes, possession, and xG stay with the calibration plan, which defines them and skips any counter that already exists (`04-plan-calibration.md` steps 1 and 2).
10. **Change queue.** Write `tactics/change.rs`: `Change`, `QueuedChange`, `SubLedger`, and `apply_at_stoppage`. Call it in `step` when `self.stoppage` is set, after the AI check. A substitute enters at the halfway touchline entry point on its side. The team's timeline gains the new plan. Every verdict pushes `ChangeApplied` or `ChangeRejected`, and an applied substitution also pushes a `Substitution` event with the player off and the player on. Unit tests: admission by kind, ledger counting, exempt windows, rejection reasons, and the conflict rule in both queue orders.
11. **AI manager.** Write `ai.rs`: `pre_match` and `in_match` with the rules in the YAML. Every queued change also pushes an `AiDecision` event with a short `ai.decision` code (`mentality-up-trailing`, `mentality-down-leading`, `sub-injury`, `sub-fatigue`). No random draws. A `Human` team gets `pre_match` only.
12. **Records.** In `observe/mod.rs`, add `stats.shots`, `injury.count`, `fatigue.mean_pct`, the three change counts, substitutions per team, and `darkpath.change_never_applied`. Fold in the signals from `04b-instrument.md` section 2. Unit test: every contract key is present.
13. **Snapshot.** In `snapshot.rs`, set `VERSION` to 2 and add every new field listed in the YAML. `Simulation::from_snapshot` restores managers, lineups, tactics, plans, the ledger, the queue, and the AI state. Keep `capture_write_read_capture_gives_identical_bytes` passing.
14. **Scenario builder.** In `scenario.rs`, add `energy`, `score`, `at_minute`, `subs_used`, `injure`, `manager`, and `queue`.
15. **Validator.** In `validate.rs`, exempt the injured and substituted-off parking spots and allow the entry point on a restart tick. Read anchors from the timeline.
16. **Protocol.** In `crates/protocol`, add `substitution`, `injury`, and `ai-decision` to `EventType`, and add the optional fields `change.applied_tick` and `ai.decision`. All five are already in the observability contract. Put `team.id` on change events. Leave `Queue`, `QueueChange`, and `detail` unchanged, and rewrite the module comment at `command.rs:1-4` in product language to say what now applies and what does not. Keep `PROTOCOL_VERSION` at 2 and write the reason beside it. Update `docs/reference/protocol.md`. Run `cargo test -p protocol`. The document test must pass.
17. **Command line.** In `stream_run.rs`, map the five new engine events and refresh player ids after a substitution. In `serve.rs`, the home team is `Human` and the away team is `Ai`. In `simulate.rs`, keep the default of both AI. In `resume.rs`, take managers, lineups, and tactics from the snapshot.
18. **Page.** In `web/stoppages.mjs`, add `injury` to `STOPS_PLAY`. Extend `web/tests/stoppages.test.mjs`. Run `node --test web/tests/*.test.mjs`. All tests must pass.
19. **Criterion tests.** Write `tactics_queue.rs`, `substitutions.rs`, `fatigue.rs`, and `injury.rs` in `crates/engine/tests` per the YAML. Update `snapshot.rs`, `full_match.rs`, `validator.rs`, and `content.rs`. Run `cargo test --workspace`. All tests must pass.
20. **Slow tests.** Write `strength.rs`, `ai_trailing.rs`, and `mentality.rs` with `#[ignore = "slow: cargo test --release -p engine -- --ignored"]`, and add `stronger` and `run_many` to `tests/common/mod.rs`. Run `cargo test --release -p engine -- --ignored`. All tests must pass. On a failure, print and record the counts (wins, draws, and losses; runs with a change; mean shots). Tune only the decision weights and the ai block, then rerun.
21. **First measurement.** Run `cargo build --release -p engine-cli`, then run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` three times, with `SM_DATA_DIR` set to a scratch folder. Record `bench.cpu_ms / bench.matches`, `bench.cpu_us_per_tick`, `bench.ticks_per_match`, and `bench.peak_mem_mb` in the implement artifact. Processor time per match must be at most 460.7 ms, and peak memory at most 6.82 MB. If either limit is exceeded, stop and report to the product owner with the levers (`decision_interval_ticks`, the pass candidate count, the fatigue refresh interval) and the per-tick figure. Do not change a lever to meet the limit.
22. **Fixture, documents, and gates.** Regenerate the local fixture with `engine-cli record --seed 7 --out fixture.smfx`. Update `README.md` and `content/README.md`. Search `crates/`, `web/`, `docs/reference/`, `content/`, and `README.md` for workflow vocabulary (slice names, stage names, `.ai/`). The search must return nothing. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `node --test web/tests/*.test.mjs`. Commit only the paths this slice changes, named explicitly, in product language, ending with the attribution line the session requires.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-9: the benchmark reruns; processor time per match within 10 percent and memory within 25 percent of the baseline | `engine-cli bench --seed 42 --matches 5 --json`, three drives (`cli-direct`, cli adapter) | Windows 11 reference machine, rustc 1.92 — yes (this machine, AMD Ryzen 7 9800X3D, Ultimate Performance plan; baseline measured here in this run) | Nothing new: `bench.cpu_ms`, `bench.matches`, `bench.cpu_us_per_tick`, `bench.ticks_per_match`, and `bench.peak_mem_mb` already exist in the run report. The baseline on `837cb5c` is in `05c-benchmark.md`. Capture stdout, stderr, and the exit code as evidence. | Rerun with `--matches 10` when `bench.cpu_wall_ratio` is below 0.9 → run from a fresh shell with no other heavy process → pre-registered deferral only if the machine is unavailable. Clearing event: the developer runs the same command on the reference machine. |

- `constraint-resolution: prerequisite-slice: tactics-and-ai` — the harness already exists (`crates/engine-cli/src/bench.rs`), and step 21 runs it. Nothing outside the slice is needed. `wall-ownership: code-owned`.

Every other criterion is automated (`observable: false` in the slice) and runs under `cargo test`. AC-1 to AC-5 run in the default `cargo test --workspace`. AC-6 to AC-8 are marked slow, as the slice says, and run with `cargo test --release -p engine -- --ignored`. The criteria keep the slice's wording, with two recorded readings. For AC-2, the four substitutions in the second case are spread over at most two windows or include half-time, so a window remains. The literal "four used" does not say how many windows they took. For AC-7, the 100 runs start from a scene at minute 70 with the AI team one goal down, because "trails after minute 70" needs a trailing team in every run.

## Test / Verification Plan

### Automated checks

- Format: `cargo fmt --all -- --check`.
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`.
- Unit and integration: `cargo test --workspace`. At baseline this is 192 tests; the 90-minute tests take about 8 seconds each in a debug build.
- Slow criteria: `cargo test --release -p engine -- --ignored`. This runs about 700 matches spread across threads, one match per thread at a time.
- Page: `node --test web/tests/*.test.mjs` (the glob form).
- Criteria map: AC-1 `tactics_queue.rs`; AC-2 and AC-3 `substitutions.rs`; AC-4 `fatigue.rs`; AC-5 `injury.rs`; AC-6 `strength.rs`; AC-7 `ai_trailing.rs`; AC-8 `mentality.rs`; AC-9 the benchmark drive below. The event mapping is covered by the existing `crates/stream/tests/fixture.rs` and `crates/protocol/tests/document.rs`.
- Regression: `determinism.rs` keeps its byte-identical assertion with both teams AI-managed; `snapshot.rs` continuation runs past a substitution; `validator.rs` asserts zero violations.
- Contract check: the record test in step 12 asserts every contract key; `crates/protocol/tests/document.rs` asserts every event field is documented.
- Benchmark compare: `/wf verify` loads `augment/benchmark.md` in compare mode against `05c-benchmark.md`.

### Interactive verification (human-in-the-loop)

- **AC-9 (benchmark report).** Platform `cli` from `stack.platforms`; driver: the binary itself (`cli-direct`); no companion skill needed.
  1. Run `cargo build --release -p engine-cli`.
  2. Set `SM_DATA_DIR` to a scratch folder.
  3. From Git Bash, run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json > <evidence-dir>/ac-9-<n>.stdout.txt 2> <evidence-dir>/ac-9-<n>.stderr.txt` for `n` = 1, 2, 3, then write `$?` to `<evidence-dir>/ac-9-<n>.exit-code`.
  4. Read each stdout file. Each holds one JSON line with `record.kind` `run-report`. Pass criteria: the median of `bench.cpu_ms / bench.matches` is at most 460.7 ms, the median `bench.peak_mem_mb` is at most 6.82, `bench.match_wall_ms` is at most 2000, `budget.pass` is true, `bench.cpu_wall_ratio` is at least 0.9, `build.hash` is not `unknown`, and the exit code is 0. Report `bench.cpu_us_per_tick` (baseline 1.4965 µs) and `bench.ticks_per_match` (baseline 279,850) beside the gate. If the per-match gate fails but the per-tick figure is within 10 percent, the product owner decides. The drive never passes that case by itself.
  5. Evidence layout per the cli adapter: `<evidence-dir>/ac-9-<n>.stdout.txt`, `ac-9-<n>.stderr.txt`, and `ac-9-<n>.exit-code`.

## Risks / Watchouts

- R1 (high): processor time per match exceeds the tripwire. The plan is precomputed, and the refresh and check intervals are coarse. Step 21 measures before the slow tests are finalised. A miss goes to the product owner, and no lever is pulled silently.
- R2 (high): team A does not win more than 100 of 200. The new attributes reach the decision layer, and the test balances home and away. A failure is fixed with the decision weights inside this slice and recorded with the counts.
- R3 (high): a resumed match drifts after a change. Snapshot version 2 carries every new field, and the continuation test crosses a substitution and a tactics change.
- R4 (medium): scored options make play passive or chaotic (slice risk). Hold carries a negative base and a held-time penalty. The possession and idle-ball bands in `full_match.rs` stay in place, and calibration tunes the weights.
- R5 (medium): the new required attributes refuse a modder's schema that lacks them. The shipped schema has all 14. The refusal names the missing attribute (NFR-7), and `content/README.md` lists the 14.
- R6 (medium): rule pack version 3, tuning version 2, and snapshot version 2 refuse earlier files by version. No such file is in the repository except the shipped ones this slice updates. Snapshots belong to one running match.
- R7 (low): injury stoppages add time, so ticks per match rise. The default rate gives about 0.6 injuries per match, and ticks per match is reported beside the gate.
- R8 (low): other work's staged files could enter the commit. Step 22 commits named paths only.

## Dependencies on Other Slices

- Consumes `match-rules` (the referee, `open_dead_ball`, restart geometry, the stoppage kinds, the snapshot, and the scenario builder) and `data-schemas-generator` (the loader, the attribute schema, the tuning file, and the rule pack).
- Touches `stream-protocol` files (`crates/protocol/*`) and two `viewer-pitch` files (`web/stoppages.mjs` and its test), after those slices are verified, never in parallel.
- Consumers: `calibration` tunes `decision`, `fatigue`, the injury rates, and the `ai` block, and runs the AI manager on both teams. `viewer-lineup-tactics` owns the socket bridge into `Simulation::queue_change` and the pre-kick-off lineup message, both behind its open product-owner questions OQ-1 and OQ-2. Until those are answered, the human team starts with the AI pre-match lineup and its socket changes are acknowledged and not applied, as they are today. That slice reads `content/tactics.json` wherever OQ-2 places it, and shows `change.state`. `commentary` reads `substitution`, `injury`, and `ai-decision`. `extra-time-penalties` extends the ledger with the extra-time allowance. Its plan also numbers a rule pack schema 3 and a snapshot version 2, and so does this plan. Whichever lands second takes the next number (`04-plan.md` Conflicts). `scripting-runtime` calls `queue_change` as its entry point for scripted decisions.

## Assumptions

Each entry records a question the plan's discovery round would have asked. Every entry except entry 3 was resolved without the product owner under the autonomous policy and is stamped `class: implementation-detail`: it picks among ways to meet criteria the shape and slice already fixed, and it changes no ratified scope. Entry 3 records a question that is intent-bearing by nature and that this run did not resolve. The plan that owns it carries it, and nothing in this plan depends on it (see Blockers). No `carried` intent-risk exists (`00-index.md` marks all six adjudicated), so no decision touches one.

1. **Where changes apply** (`class: implementation-detail`). The engine owns the queue and applies it inside `step` on the tick that opens a stoppage, not inside `TickSink::on_stoppage`. Why: the hook receives `&Simulation` (`record.rs:160`) and runs after the record (`sim.rs:333-337`), so it cannot change the match, and a change made there would land one tick late on a tick that is not a restart. The master plan's sentence "applies queued changes through the stoppage hook" is corrected in `04-plan.md`. The hook stays the one outside view of a stoppage.
2. **No socket bridge in this slice** (`class: implementation-detail`). The socket's `queue-change` path is left as it is: acknowledged, recorded, and not applied. The engine queue is filled by the AI manager and by tests through `Simulation::queue_change`. Why: every criterion of this slice is an engine test or the benchmark, and none needs the socket. The bridge needs a wire format for the change `detail`, and a parallel plan records that format as an open intent-bearing question for the product owner (`04-plan-viewer-lineup-tactics.md` Blockers, OQ-2, "what does a change's `detail` hold?"). Settling it here would make that product decision without the product owner. Leaving the bridge out is the smaller change and keeps both plans consistent. The bridge becomes a step of whichever slice the product owner's answer names.
3. **Carried, not decided: the wire payload.** This entry records no decision. The format of `QueueChange.detail` for tactics and substitution changes is a public protocol contract. This plan records it as an open question carried by `04-plan-viewer-lineup-tactics.md` OQ-2 and makes no choice. The engine's own `Change` and `TacticsPatch` types are internal Rust types, not a wire format, so a later bridge can map any answer onto them. No criterion of this slice depends on the answer, so the question blocks nothing in this plan.
4. **Tactics data** (`class: implementation-detail`). The data lives in a new `content/tactics.json`, validated fail-closed and included in the content hash. It ships four formations, five mentalities, the six instructions the slice names (3 levels each, 2 for time wasting), roles per position, and three duties. Why: shape Q26 maps roles to attributes through data, and the panel slice renders from "the tactics schema". The file is data, so a modder can extend it without code.
5. **Required attributes** (`class: implementation-detail`). `REQUIRED` grows from 7 to 14. Why: finishing, vision, decisions, and composure must reach the decision layer for C3, and stamina, natural fitness, and injury resistance must reach fatigue and injuries for C5. The shipped schema holds all of them, and the refusal names the missing one (NFR-7). This follows the precedent of `aggression` in the previous slice.
6. **Decision cadence** (`class: implementation-detail`). `decision_interval_ticks` stays 1, the product owner's choice. The slice's mitigation "decide every N ticks" is held as a lever that only the product owner may pull if the tripwire fires.
7. **Fatigue model** (`class: implementation-detail`). Energy is drained each tick. Effective values refresh every 50 ticks through a piecewise-linear curve that equals 1.0 at or above the threshold. The degraded values are pace (max speed and acceleration) and the decision values (passing, finishing, decisions, composure). The default numbers are estimates for calibration to tune. `TUNING_VERSION` rises to 2 because the fatigue block changes meaning ("not read yet" becomes read).
8. **Injury flow** (`class: implementation-detail`). An injured player leaves play at once, and the referee stops play with an `injury` stoppage and a dropped-ball restart. Why: the rule pack already names `injury` as a stoppage that admits both kinds of change and adds 60 s, and shape Q19 lists "injury stoppage" among the real-life substitution windows. The AI's substitution therefore applies at that same stoppage. A foul on the same tick keeps its free kick.
9. **Half-time windows** (`class: implementation-detail`). The slice text "5 changes in 3 windows plus half-time" becomes `substitutions.windows_exempt: ["half_time"]` in rule pack version 3. Why: the shape's named mechanism puts substitution limits in the rule pack, not in the loop.
10. **Conflict rule scope** (`class: implementation-detail`). A tactics change that names a role for a player substituted off at the same stoppage is rejected whole, and the reason names the player. Substitutions always apply before tactics at a stoppage, whatever the queue order. Why: a whole change applies or fails as one, so a chip never shows a half-applied change.
11. **AI manager behaviour** (`class: implementation-detail`). The AI uses threshold rules from the `ai` block in `content/tactics.json`. It checks every 30 s and at every injury, and it draws no random numbers. It raises mentality one step when trailing after minute 70, lowers it one step and turns time wasting on when leading after minute 80, makes injury substitutions at once, and makes fatigue substitutions below energy 0.55 from minute 55, keeping one for injuries until minute 80. Pre-match, it picks the best role fit per slot of the default formation and seven substitutes that include a goalkeeper. Why: shape Q4 fixed the behaviours and not their quality, and deterministic rules keep a match reproducible.
12. **Human lineup before kick-off** (`class: implementation-detail`). This slice adds no pre-kick-off lineup message. The human's team starts with the AI pre-match lineup until `viewer-lineup-tactics` adds the editor and its message. Why: the lineup path is that slice's open question OQ-1, which the product owner answers there. The AI pre-match lineup replaces the file-order lineup (`team.rs:89-113`) for both teams, because shape Q4 gives the AI manager the pre-match setup.
13. **Substitution in place** (`class: implementation-detail`). The incoming player takes the outgoing player's roster slot, so the 22-position record and the page's slot labels are unchanged. The incoming player enters at the halfway touchline point on the restart tick. The `substitution` event carries `player.id` (off) and `player.secondary_id` (on).
14. **Snapshot version 2** (`class: implementation-detail`). The snapshot gains squad identity, energy, statuses, tactics, the ledger, the pending queue, and the AI state. Why: shape AC-14 requires a resume with "the same lineups and used substitutions", and NFR-4 already refuses another version by name. A snapshot belongs to one running match, so no stored user data migrates.
15. **Slow criteria** (`class: implementation-detail`). AC-6 to AC-8 are `#[ignore]` tests run in release with `std::thread::scope`, using seeds 1 to N. A debug build plays one 90-minute match in about 8 s (the slowest single-test binary in this run's `cargo test --workspace` took 7.61 s), so 200 matches would take about 25 minutes.
16. **AC-6 fixture** (`class: implementation-detail`). Team A is default-b with every attribute times 1.15, rounded and clamped to 100, under a new club id. It plays 100 home and 100 away against default-b. "Wins more than 50 percent" means more than 100 wins of 200.
17. **AC-8 comparison** (`class: implementation-detail`). Team A's in-match AI changes are off, so its mentality stays as set. The criterion holds when mean shots per match for A are higher when attacking than when defensive.
18. **Benchmark gate reading** (`class: implementation-detail`). The slice criterion is judged as written, per match, against the new baseline on `837cb5c`. That baseline is the state match-rules left. The per-tick figure is reported beside it. Why: the product owner's per-tick reading (plan Round 4 Q13) was scoped to the match-rules criterion, and reusing it here would reinterpret a criterion without the product owner. A per-match miss caused by match length alone goes to the product owner.
19. **Page list** (`class: implementation-detail`). `injury` joins `STOPS_PLAY` because it now stops play. `substitution` and `ai-decision` do not. This follows the master plan's rule that a slice adding an event type decides whether it joins the list.
20. **Augmentations** (`class: implementation-detail`). `05c-benchmark.md` is re-baselined on `837cb5c` and `04b-instrument.md` is re-authored for this slice. The prior records are byte-copied to `history/05c-benchmark-4.*` and `history/04b-instrument-4.*`. The `experiment` augmentation stays deferred to `experiment-flags`. The index entry for both artifacts is left for the run driver to record, because this run does not write `00-index.md`.
21. **Consult** (`class: implementation-detail`). The triggers `appetite-medium-or-larger` and `touches-migration` hold. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`), so no consult ran (`consult-runs: []`).
22. **Shots on target stay with calibration** (`class: implementation-detail`). This slice counts shots, which the mentality criterion needs, and nothing else from the statistics set. Why: `04-plan-calibration.md` step 2 defines shots on target (a goal, a save, or a catch), passes, possession, and xG, and skips any counter already present. Two definitions of the same key would conflict.
23. **Version numbers shared with a sibling plan** (`class: implementation-detail`). This plan raises the rule pack to schema 3 and the snapshot to version 2. `04-plan-extra-time-penalties.md` numbers its own changes the same way, and `04-plan-calibration.md` raises the snapshot by one. The recorded order puts this slice first, so it takes 3 and 2, and each later slice takes the next number when it lands.
24. **Design contract** (`class: implementation-detail`). `02c-craft.md` binds no step: the only page change is one list entry in page logic, with no visual element.

## Blockers

None for this slice. One product-owner question touches this slice's work and blocks none of its criteria. It is carried by `04-plan-viewer-lineup-tactics.md` as OQ-2: the wire format of a queued change's `detail`, and with it the socket bridge into `Simulation::queue_change`. This plan does not answer it (Assumptions 2 and 3).

## Freshness Research

The three IFAB entries are recalled from the published Laws and were not re-fetched in this run: the product owner excluded the web-search tools, and the match-rules plan cites the same pages. Implement should confirm the dropped-ball distance and the half-time substitution rule against the linked pages before step 7 and step 1.

- Source: [IFAB Law 3 The Players](https://www.theifab.com/laws/latest/the-players/). Why it matters: it sets the substitution procedure and the competition rules for the number of substitutes, including the separate half-time opportunity. Takeaway: the half-time allowance goes in `windows_exempt`, and a player substituted off takes no further part.
- Source: [IFAB Law 8 The Start and Restart of Play](https://www.theifab.com/laws/latest/the-start-and-restart-of-play/). Why it matters: it defines the dropped ball after play stops for an injury: dropped for the team that last had the ball, or for the goalkeeper inside the penalty area, with all other players at least 4 m away. Takeaway: the injury restart in `restart.rs`.
- Source: [IFAB Law 7 The Duration of the Match](https://www.theifab.com/laws/latest/the-duration-of-the-match/). Why it matters: time lost to injuries and substitutions is added. Takeaway: the injury kind keeps its 60 s in the rule pack. Substitutions add no time in this slice, because the rule pack has no substitution kind, and calibration may add one.
- Source: installed dependencies read in the repository. `garde` 0.23 `dive` and `custom` are already used for nested content structs (`crates/engine/src/data/tuning.rs:260-267`), and `serde` `deny_unknown_fields` is used on every content struct. No new dependency and no new API is used, so no `study-sources` read was needed. No step asserts that a dependency lacks a capability.
- Source: `.ai/observability.md:27-59`, read in this run. Why it matters: it is the contract for the new event types and keys. Takeaway: `injury`, `substitution`, `ai-decision`, `change.applied_tick`, `ai.decision`, `fatigue.mean_pct`, `injury.count`, `stats.shots`, and `darkpath.change_never_applied` are already reserved, so only the detail payloads are new keys.
- The fatigue, injury, and decision numbers are unsourced estimates. The injury default (about 0.6 per match) is a planning estimate, not a measured league rate. `calibration` owns sourcing and tuning them.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine tactics-and-ai`. The plan is complete, and both augmentation artifacts are re-authored for this slice. Compact the session first so the SessionStart hook re-reads the artifacts.
- **Option B:** `/wf plan football-manager-match-engine commentary`. Commentary depends only on `match-rules`, so it can be planned in parallel. It would read the three event types this slice adds, so implementing it after this slice avoids a second mapping pass.
- **Option C:** `/wf slice football-manager-match-engine`. Not needed: the slice boundary held. The lineup message and the socket bridge stay with `viewer-lineup-tactics` behind its open questions (Assumptions 2, 3, and 12).
- **Option D:** `/wf shape football-manager-match-engine`. Not needed: the spec held, and every open choice was an implementation detail recorded above.
