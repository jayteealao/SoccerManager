---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: tactics-and-ai
status: complete
stage-number: 5
created-at: "2026-09-22T23:27:52Z"
updated-at: "2026-09-22T23:27:52Z"
metric-files-changed: 51
metric-lines-added: 4723
metric-lines-removed: 255
metric-deviations-from-plan: 13
metric-review-fixes-applied: 0
commit-sha: ""
steering-honored:
  - "No visual change: the page gains one entry in its stoppage list (`injury`), so the --tl- tokens, the no-spinner rule, and the colour rule are untouched."
  - "Output boundary: code comments, docs, and the commit message use product language; a scan of the diff for workflow vocabulary found none beyond the repository's existing test-criterion labels and 'named mechanism' comments."
tags: [engine, tactics, ai-manager, fatigue, injuries, substitutions, decisions, snapshot, protocol, milestone]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-tactics-and-ai.md
  plan: 04-plan-tactics-and-ai.md
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md]
  verify: 06-verify-tactics-and-ai.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine tactics-and-ai"
---

# Implement: Tactics, AI Manager, Fatigue, and Injuries

## The Implementation

The engine that this slice inherited enforced the laws, but every player was the same player: a carrier shot inside a fixed range and otherwise passed, nobody tired, nobody got hurt, and the rule pack's substitution limits were read and never used. The inherited engine also scored no goals. Nine sampled matches at `837cb5c` ended 0-0, because the keeper's catch chance was rolled again on every tick the ball was in reach, so the tuned 0.7 acted as a chance of about 1.

Five decisions carry the work. First, the ball carrier now scores five options (pass to each open team-mate, dribble, shot, clearance, hold) as weighted sums of features plus the team's mentality, instructions, role, and duty offsets, and takes the best; every weight is in `tuning.json`. Second, the engine owns a stoppage-gated change queue and applies it inside `step` on the tick that opens an admitting stoppage, substitutions first, so a change lands on the restart tick and every verdict is an event with its reason. Third, a substitute takes the leaving player's roster slot and the slot records the squad index, so the tick record keeps 22 positions and a snapshot restores lineups, benches, ledgers, energy, and the queue (snapshot version 2). Fourth, the keeper's catch is rolled once per flight, as the tuning value documents; this was required to meet the strength criterion. Fifth, the decision weights were tuned on 12 seeds to 3.0 goals, 6.2 shots, and 9.6 fouls per team per match, with no validator violation.

Every criterion passes. The default suite passes 224 tests with 0 failed and 3 ignored. The three slow criteria pass in release: the stronger team won 139 of 200, a trailing AI team changed its tactics in 100 of 100 scenes, and an attacking team took 33.69 shots per match against 2.15 when defensive. The benchmark passed three times at 431.2 to 434.4 ms per match against the 460.7 ms limit, with a peak of 6.21 MB against 6.82 MB. This slice unblocks calibration (it tunes the decision block), commentary (it reads the injury, substitution, and AI-decision events), and the lineup and tactics panel (it renders `tactics.json`). The top open risk is the size of the mentality effect: 33.69 shots per match when attacking is far above real football, and calibration must narrow it.

## Summary of Changes

- A new `content/tactics.json` (version 1) holds four formations, five mentalities, six team instructions, 13 roles, three duties, and the AI manager's settings. It is validated fail-closed and included in the content hash.
- The rule pack is version 3: `substitutions.windows_exempt` lists the stoppage kinds at which a substitution uses no window (half-time). The tuning file is version 2: a fatigue curve, the decision weights, two injury rates, and a dropped-ball delay. The attribute schema requires 14 names.
- The scored-options decision layer replaces the fixed shot range. Pressing reads the defending team's press count and distance. A shot aims away from the keeper, and finishing places it nearer the post.
- The stoppage-gated change queue lives in `crates/engine/src/tactics/change.rs`. It enforces the limit, the windows, the exempt kinds, and the rule that a tactics change naming a player substituted off at the same stoppage is rejected whole.
- The AI manager picks the lineup and the bench before kick-off. In the match it replaces injured and tired players, keeping one substitution back for an injury until minute 80, raises its mentality and pressing when it trails from minute 70, and lowers its mentality and wastes time when it leads from minute 80.
- Fatigue drains energy by effort and stamina. Below 0.7 energy, the curve scales pace, acceleration, passing, finishing, decisions, and composure. Half-time gives some back.
- Injuries are rolled on each tackle and once per player per minute. An injured player leaves play at once, and open play stops for a dropped ball 4 m clear.
- Snapshot version 2 stores squad identity, energy, statuses, tactics, ledgers, managers, AI state, and the queue.
- The event contract gains `injury`, `substitution`, and `ai-decision`, and the optional fields `change.applied_tick` and `ai.decision`. Change events carry `team.id`. `match-stats` gains `stats.shots`, `injury.count`, `fatigue.mean_pct`, `darkpath.change_never_applied`, and the change and substitution counts.
- `engine-cli serve` runs the home team as a human-managed side with the AI's pre-match setup; `simulate` and `resume` run both teams with the AI manager.

## Files Changed

- `content/tactics.json`: new; the tactics file.
- `content/tuning.json`: schema 2; the fatigue curve, drain, and recovery; the decision block; `injury_per_tackle`, `injury_per_minute`; `restart_delay_s.drop_ball`.
- `content/rules/default.json`: schema 3; `substitutions.windows_exempt: ["half_time"]`.
- `content/README.md`: the tactics file, the 14 required attributes, the decision weights, the fatigue curve, injuries, `windows_exempt`, and how the AI picks the lineup.
- `README.md`: a "Tactics and the AI manager" section and the tactics counts in the statistics line.
- `crates/engine/src/data/tactics.rs`: new; the tactics schema, its cross-field check, and lookups.
- `crates/engine/src/data/attributes.rs`: seven more required attributes and their tests.
- `crates/engine/src/data/rules.rs`: rule pack version 3, `windows_exempt`, `admits(kind)`.
- `crates/engine/src/data/tuning.rs`: tuning version 2; the fatigue block and its curve check.
- `crates/engine/src/data/mod.rs`: loads and checks `tactics.json`; the digest covers four files.
- `crates/engine/src/tactics/mod.rs`: new; `Tactics`, `TacticsPatch`, `TeamPlan`.
- `crates/engine/src/tactics/change.rs`: new; the change queue, ledgers, rejection reasons, and application.
- `crates/engine/src/ai.rs`: new; the AI manager.
- `crates/engine/src/fatigue.rs`: new; the curve, drain, recovery, and injury rolls.
- `crates/engine/src/decision.rs`: the scored options, plan-driven pressing, and shot placement.
- `crates/engine/src/player.rs`: seven derived values, `squad`, `base`, `energy`, `Status::Injured`; `Derived` is comparable.
- `crates/engine/src/team.rs`: squad, lineup, bench, tactics, and plan; anchors from the plan; `restore(slot)`.
- `crates/engine/src/sim.rs`: managers and tactics on the configuration, the new event kinds and details, the summary counters, the per-tick order, shot counting, and the once-per-flight keeper roll.
- `crates/engine/src/rules/mod.rs`: `injure`, the time-wasting factor, and half-time recovery.
- `crates/engine/src/rules/restart.rs`: the dropped ball: 4 m, the delay, the taker, and two tests.
- `crates/engine/src/rng.rs`: a separate scripted injury queue for tests.
- `crates/engine/src/snapshot.rs`: version 2 and its fields; a version-1 file is refused by name.
- `crates/engine/src/observe/mod.rs`: `TacticsStats` in `match-stats`.
- `crates/engine/src/scenario.rs`: scene steps for injury draws, energy, score, minute, substitutions used, injuries, managers, and queued changes.
- `crates/engine/src/validate.rs`: module text for injured players, substitutes, and tactics changes.
- `crates/engine/src/tuning.rs`: decision weights, injury rates, and the dropped-ball delay.
- `crates/engine/src/lib.rs`: the new modules and exports.
- `crates/engine/tests/tactics_queue.rs`, `substitutions.rs`, `fatigue.rs`, `injury.rs`: new; the default-suite criteria.
- `crates/engine/tests/strength.rs`, `ai_trailing.rs`, `mentality.rs`: new; the slow criteria (`#[ignore]`).
- `crates/engine/tests/common/mod.rs`: `calm_match`, `kinds`, `stronger`, `run_many`.
- `crates/engine/tests/snapshot.rs`: the continuation runs past substitutions and a tactics change; a version-1 file is refused.
- `crates/engine/tests/full_match.rs`: shots, the substitution limit, and an empty queue at full time.
- `crates/engine/tests/validator.rs`: the match under test includes a substitution and fatigue.
- `crates/engine/tests/content.rs`, `identity.rs`: the new versions and fields.
- `crates/protocol/src/event.rs`: three event types, two fields, four builders, two tests.
- `crates/protocol/src/lib.rs`: the version note and the field list.
- `crates/protocol/src/command.rs`: module text on the two queues.
- `crates/engine-cli/src/stream_run.rs`: change verdicts as `tactics-change` events; roster ids that follow substitutions; the new kinds.
- `crates/engine-cli/src/serve.rs`: the home team is human-managed.
- `crates/engine-cli/src/simulate.rs`, `resume.rs`: `TacticsStats` in the statistics.
- `crates/stream/tests/common/mod.rs`: the new kinds in the test mapping.
- `docs/reference/protocol.md`: the event types, fields, rejection reasons, and queue behaviour.
- `web/stoppages.mjs`, `web/tests/stoppages.test.mjs`: an injury is a stoppage mark; a substitution and an AI choice are not.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs`, `rules/mod.rs`, `rules/restart.rs`, `snapshot.rs`, `scenario.rs`: from `engine-core` and `match-rules`.
- `crates/protocol/src/event.rs`, `lib.rs`, `docs/reference/protocol.md`: from `stream-protocol` and `match-rules`; `commentary` reads them next.
- `crates/engine-cli/src/stream_run.rs`, `serve.rs`: from `stream-protocol`; `viewer-lineup-tactics` adds the socket bridge here.
- `content/tuning.json`, `content/README.md`: `calibration` tunes the decision block, the fatigue curve, and the injury rates.
- `web/stoppages.mjs`: from `viewer-pitch` and `match-rules`.

## Notes on Design Choices

- The queue applies inside `step`, not in `TickSink::on_stoppage`, because the hook receives `&Simulation` and runs after the record, so a change there would land one tick late (plan decision 1).
- Substitution in place keeps the tick record at 22 positions and the tick-file header unchanged; `player_ids()` reads each slot's squad index, and `stream_run.rs` updates its id table on each substitution event, so an earlier event on the same tick still names the player who left.
- `EngineEvent` stays `Copy`: the change, substitution, injury, and AI details ride in `detail: Option<EventDetail>`.
- The injury rolls have their own scripted queue (`rng.rs:81`, `rng.rs:91`), so a test can script an injury without disturbing the referee's scripted draws.
- The AI manager queues an injury substitution even when the limit is used, so the rejection and its reason reach the event stream (the criterion names the reason).
- All tuning of the decision layer changed `tuning.json` values only; no lever named in the plan's benchmark stop rule (`decision_interval_ticks`, the pass candidate count, the fatigue refresh interval) was changed.

## Verification Seams Built

- AC-1 (change waits, applies on the stoppage tick) → `Scene::queue` at `crates/engine/src/scenario.rs:181`, `Scene::manager` at `scenario.rs:175`, `Simulation::pending_changes` at `crates/engine/src/tactics/change.rs:207`, `Simulation::stoppage` at `crates/engine/src/sim.rs:430` (enables `crates/engine/tests/tactics_queue.rs` to watch the queue tick by tick).
- AC-2 and AC-3 (limit, windows, half-time, conflict) → `Scene::subs_used` at `scenario.rs:161`, `Simulation::ledgers` at `change.rs:212`, `RejectReason::text` at `change.rs:160` (enables `substitutions.rs` to assert the exact reason text).
- AC-4 (fatigue curve) → `Scene::energy` at `scenario.rs:127`, `fatigue::multiplier` at `crates/engine/src/fatigue.rs:46`, `Simulation::fatigue_mean_pct` at `fatigue.rs:148` (enables `fatigue.rs`).
- AC-5 (injury, AI substitution, ten players) → `Scene::injury_rolls` at `scenario.rs:121` over `Rng::script_injuries` at `crates/engine/src/rng.rs:91` (enables `injury.rs` to script one injury on one tackle).
- AC-6 (stronger team) → `stronger` at `crates/engine/tests/common/mod.rs:122` and `run_many` at `common/mod.rs:135` (enables `strength.rs` over 200 seeds on all cores).
- AC-7 (trailing AI) → `Scene::at_minute` at `scenario.rs:142` and `Scene::score` at `scenario.rs:134` (enables `ai_trailing.rs` to start 100 scenes at minute 70, 0-1 down).
- AC-8 (mentality) → `MatchConfig::with_manager` at `sim.rs:118` and `MatchConfig::with_tactics` at `sim.rs:125` (enables `mentality.rs` to fix team A's mentality with in-match changes off).
- AC-9 (benchmark) → no new seam; `engine-cli bench --json` already reports `bench.cpu_ms`, `bench.cpu_us_per_tick`, `bench.ticks_per_match`, and `bench.peak_mem_mb`. Dark-path signal `darkpath.change_never_applied` at `crates/engine/src/observe/mod.rs:112`.

## Visual Contract Honored

No visual change. `web/stoppages.mjs:27` adds `injury` to the list of events that stop play; no token, component, or style changed.

## Deviations from Plan

1. **Keeper catch rolled once per flight** (`class: implementation-detail`). `sim.rs:301` adds `keeper_beaten`, reset on a kick, a gain, and a dead ball. The plan did not name it. The baseline at `837cb5c` scored 0 goals in 9 of 9 sampled matches because the catch was rolled every tick in reach; the tuning file documents the value as one chance per save. Without the fix, AC-6 could not be met.
2. **Shot placement** (`class: implementation-detail`). A shot aims at the side away from the keeper, and finishing sets how near the post and how true. The plan named finishing as a shot feature only.
3. **No `Status::SubstitutedOff`** (`class: implementation-detail`). A substituted player leaves the roster slot; the team's `lineup` and `bench` record who played, so the status had no reader.
4. **Separate injury draw queue** (`class: implementation-detail`). `rng.rs:81` draws injuries from the seeded stream with their own script queue; the plan named the referee's queue.
5. **Tactics-plan tracing** (`class: implementation-detail`). The `tactics.plan` signal is emitted where a change applies (`tactics/change.rs`), not in `team.rs`, because the team has no match tick.
6. **Instruction field list** (`class: implementation-detail`). The new event fields are listed in `crates/protocol/src/lib.rs` (`MESSAGES`), where the existing list lives, not in `message.rs`.
7. **Stream test mapping** (`class: implementation-detail`). `crates/stream/tests/common/mod.rs` maps the new event kinds; the plan did not list the file, and the build failed without it.
8. **Decision weights tuned** (`class: implementation-detail`). `pressure` -0.8, `dribble_base` -0.4, `dribble_space` 0.55, `progress` 0.7, `shot_distance` 1.0, `skill` 0.6. With the plan's starting weights and the keeper fix, matches averaged about 100 goals, 70 percent from solo runs straight from kick-off.
9. **AI queues an injury substitution with the limit used** (`class: implementation-detail`), so the rejection and its reason are events (AC-5).
10. **Instruction schema shape** (`class: implementation-detail`). Each instruction is `{ "default", "levels" }` with named levels, instead of a bare level list, so the default is data.
11. **AC-1 penalty case** (`class: implementation-detail`). The test proves the change waits through the whole penalty and that the rule pack admits no change at a penalty; the "applies at the next admitting stoppage" half is proved by the throw-in test. A quiet scene cannot reach a stoppage after the penalty, because the taker holds the ball when no player decides.
12. **AC-4 scope** (`class: implementation-detail`). The full-match check covers every player in the roster at full time, substitutes included, not only the starters.
13. **`Derived` derives `PartialEq`** (`class: implementation-detail`), so the fatigue test compares effective and base values whole.

## Anything Deferred

- The socket bridge for `queue-change` (plan decision 2). A change sent over the socket is acknowledged and recorded but not applied. Ceiling: only the AI manager and tests fill the engine queue. Upgrade path: the slice that settles the change `detail` wire format routes it into `Simulation::queue_change`.
- A human manager's pre-match lineup. `serve` gives the human-managed home team the AI's setup. Upgrade path: the lineup message of `viewer-lineup-tactics`.

## Known Risks / Caveats

- **Mentality effect too strong.** AC-8 measured 33.69 shots per match attacking against 2.15 defensive (`implement-evidence/tactics-and-ai/slow-ac-6-7-8.stdout`). The direction is right; the size is not realistic. `calibration` owns the mentality offsets and the decision weights.
- **Weights tuned on few seeds.** The decision block was tuned on 12 seeds for goals, shots, and fouls, and 24 matches for strength. `calibration` retunes it against the realism bands.
- **Benchmark build.** The three benchmark runs used a working tree that later gained only test files, docs, a `PartialEq` derive, and a slice-typed argument; `build.hash` reads `837cb5c-dirty`. Verify reruns it on the commit.
- **Processor margin.** 434.4 ms per match against 460.7 ms, about 6 percent. The match is longer (298,350 ticks against 279,850) because injuries and substitutions add time.

## Freshness Research

- IFAB Laws of the Game, Law 8 (dropped ball, 4 m) and Law 3 (substitutions): not re-fetched in this run; the plan's research stands, and the dropped-ball distance and the half-time window rule match it.
- No library or external API changed; no dependency was added.

## Evidence

- Default suite: `cargo test --workspace`, exit 0, 224 passed, 0 failed, 3 ignored (`implement-evidence/tactics-and-ai/test-workspace.stdout`).
- Slow criteria: `cargo test --release -p engine -- --ignored`, exit 0: won 139, drew 34, lost 27 of 200; 100 of 100 scenes; 33.69 against 2.15 shots (`slow-ac-6-7-8.stdout`, `slow-ac-6-7-8.stderr`).
- Page: `node --test web/tests/*.test.mjs`, 47 passed (`node-tests.stdout`).
- `cargo fmt --all -- --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings` exit 0.
- Benchmark (AC-9): `ac-9-{1,2,3}.stdout.txt`, each exit 0 and `budget.pass` true: processor time per match 431.2, 434.4, 434.4 ms (limit 460.7); 1.4453, 1.456, 1.456 µs per tick; 298,350 ticks per match; peak memory 6.17, 6.14, 6.21 MB (limit 6.82); processor-to-wall ratio 0.99 to 1.00.

## Assumptions and Decisions

- Keeper catch once per flight — `class: implementation-detail`.
- Decision weight values — `class: implementation-detail` (the plan authorises tuning the decision block).
- Home team human-managed in `serve` — `class: implementation-detail` (plan: the socket side is the human's).
- No intent-bearing decision was made; nothing stopped the run.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine tactics-and-ai`. The slice changes testable behaviour in every engine path, and verify reruns the benchmark on the commit. Consider compacting the session first; workflow state lives in the files on disk.
- **Option B:** `/wf review football-manager-match-engine tactics-and-ai`. Not recommended: the behaviour is testable.
- **Option C:** `/wf plan football-manager-match-engine calibration`, in parallel, to narrow the mentality effect.
