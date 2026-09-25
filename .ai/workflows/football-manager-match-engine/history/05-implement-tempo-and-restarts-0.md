---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: tempo-and-restarts
status: awaiting-input
stage-number: 5
created-at: "2026-09-25T04:27:37Z"
updated-at: "2026-09-25T04:27:37Z"
metric-files-changed: 18
metric-lines-added: 911
metric-lines-removed: 21
metric-deviations-from-plan: 7
metric-review-fixes-applied: 0
commit-sha: "831553b5b49baa7105b9ffdbd33e693661910809"
commits:
  - "831553b5b49baa7105b9ffdbd33e693661910809"
has-blockers: true
open-questions:
  - "Q-TR1 Fewer passes make the match a shooting gallery — AWAITING INPUT (po-answers.md)"
steering-honored:
  - "Heavy runs go to the server (2026-09-25): every calibrate run, every release slow test, fmt, clippy and the workspace suite ran on the server through the sync script; the benchmark, the page tests and the two Windows-only tests ran on this PC with -j 6."
  - "Q-KS2: corners from clearances and crosses are built as one mechanism, a defender's clearance of a fast pass inside his own penalty area; a corner still needs the ball to cross the goal line after a defending touch (RIM-9). No parry was turned toward the goal line."
  - "Q-E4: band misses are recorded, not failed; no band, limit or criterion was changed."
  - "Q-I1: targeted calibrate runs were used only as the inner loop; the slice gate was not run because the slice stopped before a setting was accepted."
  - "Output boundary: code comments, test names, docs and the commit message use product language; the added lines were leak-checked before the commit."
  - "Design direction: not applicable; no page changed."
tags: [engine, passing, restarts, realism]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-tempo-and-restarts.md
  plan: 04-plan-tempo-and-restarts.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/tempo-and-restarts/
  siblings: [05-implement-keeper-and-shots.md, 05-implement-lone-forward.md, 05-implement-tuning-loop.md, 05-implement-realism-bands-v2.md, 05-implement-defending-and-discipline.md]
  verify: 06-verify-tempo-and-restarts.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine tempo-and-restarts"
---

# Implement: Tempo and restarts

## The Implementation

The slice started from `bce0144`, where teams made 1,225 passes each at 86.4% and the ball was live for about 90 minutes. The counting and every lever the plan named are now in the engine, at commit `831553b`. A clearance is its own kick, the first kick of a restart taker is a restart kick, and `stats.passes` counts open-play passes only. `stats.clearances`, `stats.restart_kicks` and `stats.ball_in_play_s` are in match-stats, and the calibrate report shows the minutes in play per 90. The carry window, the clearance aim spread and the cross clearance are tuning values. They ship switched off, so play is unchanged: on seed 42 over 200 matches, the counting alone moves passes from 1,225.8 to 1,193.8 per team and leaves throw-ins, goal kicks and goals identical. Eight fast scenes prove the counting, the timing and the cross clearance.

The tuning stopped. 60 settings on seed 42 found two walls. First, the aim spread and the cross clearance at their bounds give 18.3 throw-ins per match against a floor of 35; only the carrier's clearance weight reaches the band. Second, every setting with 550 or fewer passes per team makes carriers dribble forward, and gives 27–61 shots per team (13.2 today) and 4.0–14.8 goals per match. The best setting for the four criteria fails "Passes are realistic" on the test seeds (603 per team) and breaks `every_formation_holds` in 6 of 10 pairings. The plan said to stop and report, so the choice is with the product owner as Q-TR1 in `po-answers.md`.

When Q-TR1 is answered, a rerun of implement tunes the shipped values and moves the restart delays to the sourced medians. The code, the seams and the slow tests do not change. The top risk: in this engine a carried ball advances faster and more safely than a passed one, so any answer that keeps 350–550 passes needs carrying to be contested.

## Summary of Changes

- A clearance is `Kick::Clear`. A carrier who chooses to clear kicks one, aimed with `clearances.aim_spread` (the hard-set 0.6 rad before).
- The restart taker is marked when play restarts and when a half kicks off. His first kick, before anyone else touches the ball, is a restart kick. A shot is always a shot.
- `Summary` counts `clearances`, `restart_kicks` and `live_ticks`. Only an open-play pass adds to `passes` and can be completed, so pass accuracy is over open-play passes.
- A defender can clear a fast open-play pass inside his own penalty area, once per flight, by deflection. The ball goes away from his goal centre, his side touches it last, and one clearance is counted. It ships off (`cross_chance` 0).
- The carry window: for `carry_s` after gaining the ball, an unpressed carrier pays `carry_cost` on a pass and on a clearance. It ships off (`carry_cost` 0).
- match-stats gets `stats.clearances`, `stats.restart_kicks` and `stats.ball_in_play_s`. The calibrate report gets `ball_in_play_min_per_90_mean`. Both schemas allow them, and `stats.passes` is described as open-play passes.
- The snapshot moves to version 6 for the three counters and the restart-taker mark.
- `Scene::clear` is a new scene seam. `tests/tempo_and_restarts.rs` holds 8 fast scenes and 4 slow criterion tests.

## Files Changed

- `crates/engine/src/decision.rs`: `Kick::Clear` and `Kick::flight`; the carry window in `options()`; the clearance aim from the tuning; two unit tests of the carry window.
- `crates/engine/src/sim.rs`: the three counters; the restart-taker mark and the per-flight clearer mask; counting by kind in `kick_ball()`; `live_ticks` in the live arm; `try_clear_cross()` before the ordinary contest.
- `crates/engine/src/rules/mod.rs`: the mark set at a restart and at a kick-off, cleared at every dead ball; the shoot-out kick reads `Kick::flight`.
- `crates/engine/src/tuning.rs`: `carry_s`, `carry_cost` and the `ClearanceTuning` block, with bounds and defaults.
- `crates/engine/src/snapshot.rs`: version 6; the counters and the mark.
- `crates/engine/src/observe/mod.rs`: the three match-stats keys and their contract check.
- `crates/engine/src/scenario.rs`: `Scene::clear`; a scene starts in open play with no restart taker.
- `crates/engine-cli/src/report/mod.rs`, `crates/engine-cli/src/calibrate/mod.rs`: `ball_in_play_min_per_90_mean`, scaled by the run's match length.
- `content/tuning.json`: the carry and clearance values, switched off.
- `crates/engine/tests/tempo_and_restarts.rs` (new): the fast scenes and the slow criterion tests.
- `crates/engine/tests/content.rs`: the new values pinned; an older tuning block loads with the defaults.
- `crates/engine/tests/snapshot.rs`: the version in the refusal message.
- `schemas/observability/match-stats.schema.json`, `schemas/observability/run-report.schema.json`: the optional keys; the `stats.passes` description.
- `docs/reference/data-files.md`, `docs/reference/protocol.md`, `.ai/observability.md`: the rows, the clearance paragraph, the `stats.passes` wording and the key list.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs` and `crates/engine/src/rules/mod.rs`: `keeper-and-shots` added the shot contest and `deflected_by()`. The cross clearance reuses both and runs after the shot contest.
- `crates/engine/src/decision.rs`: `lone-forward` added the lone-carrier weights to `options()`. The carry window sits beside them and does not read them.
- `content/tuning.json`, `crates/engine/src/tuning.rs`: shared by every tuning slice. The new fields have serde defaults, so `TUNING_VERSION` stays 2.
- `crates/engine/src/snapshot.rs`: the version moves from 5 to 6.

## Notes on Design Choices

- The carry window is a soft cost, not a stored decision, so every agent still decides every tick and no draw is added (the unit test checks the next draw).
- The restart-taker mark is in the snapshot. A half's kick-off sets it while play is live, so a resumed match needs it to count the same.
- A dropped ball restarts through the same path, so its taker's first kick is also a restart kick (Law 8 makes it a restart).
- The minutes-per-90 figure is a report figure, not a band. The bands file is the product owner's sourced data.
- The fast scenes switch the cross clearance on in their own configuration, so they prove the mechanism while it ships off.

## Verification Seams Built

- Only real passes count → `Kick::Clear` at `crates/engine/src/decision.rs:24`, the restart-taker mark at `crates/engine/src/sim.rs:524` (set at `crates/engine/src/rules/mod.rs:165` and `:594`), counting at `crates/engine/src/sim.rs:974-998`, and `Scene::clear` at `crates/engine/src/scenario.rs:143` (enables `cargo test -p engine --all-features --test tempo_and_restarts` to observe a clearance and each restart kick on the real engine).
- The ball is in play for about an hour → `Summary::live_ticks` counted at `crates/engine/src/sim.rs:855`, `stats.ball_in_play_s` at `crates/engine/src/observe/mod.rs:202`, `ball_in_play_min_per_90_mean` at `crates/engine-cli/src/report/mod.rs:465` (enables the slow test `the_ball_is_in_play_for_about_an_hour` and `engine-cli calibrate` to read it).
- Passes are realistic, Throw-ins stay in band → the slow tests `passes_are_realistic` (`crates/engine/tests/tempo_and_restarts.rs:382`) and `throw_ins_stay_in_band` (`:418`) over one shared run of 200 matches (`runs()` at `:375`).
- Corners from clearances (Q-KS2) → `try_clear_cross()` at `crates/engine/src/sim.rs:1154` and the scene `a_cleared_pass_along_the_goal_line_can_give_a_corner` (`crates/engine/tests/tempo_and_restarts.rs:248`); `restart_census` (`:442`) asserts that every corner followed a crossing.

## Tuning Loop (step 9)

Every run is `engine-cli calibrate --suite equal --seed 42 --matches 200` on the server. The full table of 60 settings is `implement-evidence/tempo-and-restarts/inner/sweeps.txt`, tabulated from each run's report and statistics records. Representative rows:

| Setting | Passes per team | Accuracy % | Throw-ins | Ball in play (min/90) | Shots per team | Goals per match | Verdict |
|---|---|---|---|---|---|---|---|
| Shipped at `bce0144` (`tr-set-base`) | 1,225.8 | 86.5 | 21.1 | 89.6 (from `tr-count`) | 12.0 | 1.79 | baseline |
| Counting alone (`tr-count`) | 1,193.8 | 86.7 | 21.1 | 89.6 | 12.0 | 1.79 | play identical |
| Plan start: carry 1.5 s / 1.0, medians, cross 0.3 (`tr-start`) | 606.7 | 92.2 | 16.9 | 60.2 | 42.8 | 5.12 | shots x3.6 |
| Aim spread 1.6, cross chance 1.0, reach 3, spread 3.2 (`s8-W7`) | 606.3 | 92.2 | 18.3 | 60.0 | 43.3 | 5.45 | throw-in wall of the named levers |
| Hold 0, dribble −0.8, clear −0.5 (`s2-L`) | 742.9 | 94.3 | 40.1 | 71.1 | 6.4 | 0.67 | passes and ball in play fail |
| Carry 1.0 s / 1.5, `clear` −0.3, `min_lane` 0.5, `lane` 0.3 (`s6-U8`) | 695.8 | 87.9 | 63.2 | 67.7 | 12.5 | 1.49 | realistic shots; passes fail |
| `clear` −0.3, `clear_pressure` 2.0 (`s2-H`) | 484.9 | 93.2 | 75.1 | 49.6 | 27.0 | 3.96 | fewest shots under 550 passes |
| `min_lane` 0, `lane` 0, `clear` −0.45 (`s9-X1`) | 557.0 | 83.7 | 43.9 | 52.6 | 56.4 | 7.85 | closest to the four criteria |

Two code variants were tried and reverted: charging the carry cost on a shot too (`s5-T*`: goals rose to 4.9–14.8) and applying the window only outside the shooting range (`s7-V*`: 39.7 shots per team at the start values). Neither broke the coupling.

## Criterion Results

At the shipped values (levers off) the counting criterion passes. The other three are not met; the slice stopped before a setting was accepted.

| Criterion | Evidence this run | Result |
|---|---|---|
| Only real passes count | `cargo test -p engine --all-features --test tempo_and_restarts` locally: 8 passed, 4 ignored (clearance, restart kicks for throw-in, corner, goal kick and kick-off, next pass is a pass) | pass |
| Passes are realistic (350–550, 75–88%) | X1 on seeds 1–200 (`implement-evidence/tempo-and-restarts/x1-tempo/criteria.txt`): 603.1 per team, 81.27% | fails (Q-TR1) |
| The ball is in play for about an hour (52–65) | X1 on seeds 1–200: 55.37 min per 90 | passes only at a setting that breaks formations |
| Throw-ins stay in band (35–55) | X1 on seeds 1–200: 39.96 per match (census) | passes only with the `clear` weight |
| Earlier criteria at X1 | `every_formation_holds` fails in 6 of 10 pairings (`x1-crit/criteria.txt`); the run stopped at the first failing binary | regression |

The restart census at X1 shows the median dead time of each restart at its source: throw-in 13.8 s, goal kick 23.2 s, corner 31.8 s, free kick 32.5 s. Every corner followed a crossing after a defending touch.

## Checks Run

- Server, on the committed tree: `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo test --workspace --all-features --no-fail-fast` 513 passed, 2 failed, 14 ignored (`implement-evidence/tempo-and-restarts/checks/`). The two failures are the Windows-only tests steer.md names (`relative_paths_never_leak_the_root` and `a_small_run_writes_a_record_per_match_and_a_report_that_validate`); both pass on this PC.
- This PC: `data::tests::relative_paths_never_leak_the_root` passed; `a_small_run_writes_a_record_per_match_and_a_report_that_validate` passed; `node --test web/tests/*.test.mjs` 128 passed.
- Benchmark on this PC (`engine-cli bench --seed 42 --matches 5 --json`, three drives, `implement-evidence/tempo-and-restarts/bench/`): 1.5854, 1.5743 and 1.5526 µs per tick against the 1.7071 µs gate; peak memory 6.76 MB against 8.55 MB; 285,850 ticks per match, as at the baseline.
- Baselines (step 1, `implement-evidence/tempo-and-restarts/baseline/`): `tr-equal-base` (1,000 matches) and `tr-set-base` (200 matches) at `bce0144`, content `8170d2f2d5fa`, fixtures `c4c0a247c3c9`.

## Deviations from Plan

1. The shipped values keep the levers off (`carry_cost` 0, `cross_chance` 0) and the restart delays at 3, 8, 6 and 8 s, not the starting values and the medians. The tuning stopped (Q-TR1), and the starting values give 42.8 shots per team; shipping them would make the match worse than it is today. Pins in `tests/content.rs` follow the shipped values. (class: implementation-detail)
2. The throw-in delay scene asserts the tuned delay (`restart::delay_ticks`), not a hard 690 ticks, because the delay stays at 3 s until Q-TR1 is answered. (class: implementation-detail)
3. The throw-in levers the plan named cannot reach 35 (18.3 at their bounds, `s8-W7`). The sweeps also used the carrier's `clear` and `clear_pressure` weights, which are decision weights inside their bounds. A rerun after Q-TR1 uses them. (class: implementation-detail)
4. The restart-taker mark is in the snapshot, beside the counters, because a half's kick-off sets it during live play. (class: implementation-detail)
5. A dropped ball's first kick is a restart kick too. (class: implementation-detail)
6. Step 3's counting-alone run used a content copy with the levers off and the old delays, as planned; the step-11 run of earlier criteria used the X1 setting, not a shipped setting, to test whether the closest setting regresses. (class: implementation-detail)
7. Steps 10–12 and 14.3 (criterion runs on a shipped setting, re-derived pins, slice gate) did not run: no setting was accepted. No pinned expectation moved, because play at the shipped values is identical to `bce0144` (`tr-count` against `tr-set-base`). (class: implementation-detail)

## Anything Deferred

- Tuning the carry window, the clearances and the pass weights to the criteria, and moving the four restart delays to the sourced medians: waits for Q-TR1.
- The slice gate (equal and strength on five seeds, formations on seed 42): waits for an accepted setting.

## Known Risks / Caveats

- The slow tests `passes_are_realistic`, `the_ball_is_in_play_for_about_an_hour` and `throw_ins_stay_in_band` fail at the shipped values. They are ignored tests and record the open criteria.
- Pass accuracy now covers open-play passes only; on shipped play it moved from 86.5% to 86.7%.
- The cross clearance produced no corner in the X1 census (all corners came from saves, parries and blocks). With `cross_chance` 0.3 and a 2 m reach it fires rarely. The rerun should read the census before settling its values.

## Assumptions

- A-I1 (class: implementation-detail): the foundation is committed with play unchanged so that other sessions that sync the tree to the server test shipped play, not an unaccepted setting.
- A-I2 (class: implementation-detail): the census attributes a corner to the last save, block or clearance since the ball was last controlled; any other corner is "other".
- A-I3 (class: implementation-detail): the X1 setting was chosen for the regression check because it is the closest of the 60 to all four criteria on seed 42.

## Triage Decisions

- Q-TR1 (class: intent-bearing; ac: "Passes are realistic"; classification: runtime-evidence): stopped. Keeping 350–550 passes needs either a defending lever, a moved criterion, or accepted shots and goals with a relaxed formations criterion. Each changes the product owner's scope or limits, so none is taken autonomously.
- Throw-ins (class: implementation-detail; ac: "Throw-ins stay in band"; classification: runtime-evidence): the carrier's clearance weight is an in-scope decision weight; the rerun uses it.
- Ball in play (class: implementation-detail; ac: "The ball is in play for about an hour"; classification: runtime-evidence): reachable at the medians (55.4 at X1); depends on the Q-TR1 setting.
- Only real passes count (class: implementation-detail; ac: "Only real passes count"; classification: build-capability): built and proven by the scenes.

## Freshness Research

- No dependency was added or upgraded. `serde(default)` on fields and on a nested block inside a `deny_unknown_fields` struct follows the `lone_*` weights and the `shots` block already in `tuning.rs`.
- The restart medians are `docs/design/realism/01-engine-realism.md:614`; the restart census reproduced them at X1.

## Recommended Next Stage

- **Option D: Blocked** → the product owner answers Q-TR1 in `po-answers.md`. Then rerun `/wf implement football-manager-match-engine tempo-and-restarts` to tune the shipped values under the answer.
- **Option C: Revisit Plan** → `/wf plan football-manager-match-engine tempo-and-restarts` if the answer adds a defending lever (option 1), since that changes the plan's lever list.
