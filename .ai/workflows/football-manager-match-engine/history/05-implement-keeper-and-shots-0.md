---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: keeper-and-shots
status: awaiting-input
stage-number: 5
created-at: "2026-09-24T17:26:30Z"
updated-at: "2026-09-24T17:26:30Z"
metric-files-changed: 14
metric-lines-added: 1070
metric-lines-removed: 66
metric-deviations-from-plan: 6
metric-review-fixes-applied: 0
commit-sha: "8fd407b4bc6c50b1751e26a74d06ef6cc5bc8013"
commits:
  - "8fd407b4bc6c50b1751e26a74d06ef6cc5bc8013"
has-blockers: true
open-questions:
  - "Q-KS1 The red-card criterion still fails all three arms after the save and block model — AWAITING INPUT (po-answers.md)"
  - "Q-KS2 Corners per team stay under 3.0 at every in-bounds setting of the save and block model — AWAITING INPUT (po-answers.md)"
steering-honored:
  - "Red-card criterion moves to keeper-and-shots (Q-LF2): measured on the balanced fixtures of Q-LF1 (seeds 1-120 in both club orders) with every limit unchanged. The save and block model does not fix it, so implement stops and reports the arms and their figures. No limit was raised, no card chance changed and no lever was added for the ten-man side."
  - "Q-I1: targeted calibrate runs (equal, seed 42, 200 matches, with a baseline) were the inner loop. The slice gate (five seeds of equal and strength, one of formations) was not run, because the slice stopped at plan step 14 before a setting was accepted."
  - "Output boundary: code comments, test names, docs and the commit message use product language; the added lines were leak-checked before the commit."
  - "Design direction: not applicable; no page changed."
tags: [engine, shots, set-pieces, realism]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-keeper-and-shots.md
  plan: 04-plan-keeper-and-shots.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/keeper-and-shots/
  siblings: [05-implement-lone-forward.md, 05-implement-defending-and-discipline.md, 05-implement-tuning-loop.md, 05-implement-realism-bands-v2.md, 05-implement-extra-time-penalties.md]
  verify: 06-verify-keeper-and-shots.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine keeper-and-shots"
---

# Implement: Keeper and shots

## The Implementation

The slice started from the shipped lone-forward play at `a30313b`. There, 77% of shots counted as on target, goals per xG was 1.50, corners were 0.005 per team and goal kicks 5.9 per match (seed 42, 1,000 matches). A keeper held every save with one flat roll, no outfield player touched a shot, and no shot rose above 0.32 m. The red-card criterion, moved here by the product owner (Q-LF2), failed all three arms.

Commit `8fd407b` (14 files, +1,070 −66) builds the planned model. The engine follows a copy of each shot with the match physics and counts it on target only when that flight crosses between the posts under the bar. A keeper tries to save only such a shot, on the sourced curve (0.891 down to 0.272), and holds one save in three or parries it. An outfield defender within 3 m can block a shot once. A parry or a block leaves the defending side as the last touch, so a corner needs a real crossing. Penalties and the shoot-out use the same save model, and the three hand-set shoot-out constants are tuning values now. The 22-run tuning loop and one xG refit give, on seed 42 over 1,000 matches: 38.7% of shots on target, goals per xG 1.10 and 13.1 goal kicks per match. 500 scripted penalties convert 79.6%, and the wide and over scenes pass. The workspace suite passes: 504 tests, 0 failed.

Two criteria fail, so the run stops at plan step 14. First, the red-card criterion still fails all three arms. In the keeper arm the reduced side scores 2.10 against 1.18, and 1.18 is also above the 1.17 limit. Second, corners reach only 1.2 per team in the 200-match slow test (1.6 on seed 42), against the 3.0 floor. With every block and parry value at its bound, the planned model's ceiling is 1.9 to 2.3. Only an experiment that turns every parry toward the goal line and holds one save in ten reached 3.2, and goal kicks then fell under 10. Q-KS1 and Q-KS2 are open for the product owner. The top risk is that corners in this engine need sources other than shots, such as clearances and crosses, which this slice does not own.

## Summary of Changes

- A new pure module `shot.rs`: `on_target()` steps a copy of the ball to the goal line, `quality()` reads the fixed quality model, `save_chance()` is the clamped sourced line, and `deflect()` turns a velocity. The caller takes every random draw.
- On target is counted at the kick (`kick_ball()`), and the flight keeps the on-target flag and the shot quality. The count in `goal()` and in a keeper's `gain()` is removed.
- A fast shot in flight is contested only by blockers and, when on target, by the acting keeper (`contest_shot()`, `try_block()`, `try_save()`, `parry()`). An off-target shot is never saved. `keeper_catch_chance` now covers only fast balls that are not shots.
- The shot loft is drawn from 0 to `shots.loft_max`. A penalty in play counts `shots.penalty_xg`, and its save roll reads that quality.
- The shoot-out keeper dives by `shots.keeper_dive_m` and `shots.keeper_stays` and saves only an on-target kick at the penalty quality: a held kick is a miss, a parried one plays on. `SHOOTOUT_SPREAD`, `SHOOTOUT_HOLD`, `KEEPER_DIVE_M`, `KEEPER_STAYS` and their known-limit comment are gone.
- A new optional `shots` tuning block (16 values with bounds). `shot_noise` 0.5 and refitted `xg` coefficients (−4.191, −0.0441, 6.3836). A tuning file without the block loads with the shipped values, and `TUNING_VERSION` stays 2.
- Test seams under the `scenario` feature: `Scene::penalty()`, `Simulation::last_touch()`, `shot_census()` and `shot_flight()`.
- New tests: 6 unit tests in `shot.rs`, 7 fast scenes and 3 slow tests in `tests/keeper_and_shots.rs`, and 2 content pins. Two moved expectations were re-derived.

## Files Changed

- `crates/engine/src/shot.rs` (new): trajectory on-target check, shot quality, save chance, deflection, and unit tests.
- `crates/engine/src/sim.rs`: flight fields, `kick_ball()` with the on-target count and the penalty xG, the shot contest, blocks, the save, the parry, `end_shot()`, the `ShotCensus` seam and three accessors under the scenario feature. The on-target count in `gain()` is removed.
- `crates/engine/src/rules/mod.rs`: `goal()` no longer counts on target. `ball_out()` records a wide shot in the census. The penalty in `take_restart()` goes through `kick_ball(..., penalty = true)`. The shoot-out dive and save use the tuning values, and the four constants are removed.
- `crates/engine/src/tuning.rs`: the `ShotTuning` block, its bounds and `Default`; `shot_noise`, `penalty_spread`, the block and parry values and the `xg` defaults as tuned.
- `crates/engine/src/decision.rs`: the shot loft reads `shots.loft_max`.
- `crates/engine/src/scenario.rs`: `Scene::penalty(team, taker)`.
- `crates/engine/src/lib.rs`: the `shot` module line.
- `content/tuning.json`: the `shots` block, `shot_noise` 0.5 and the refitted `xg`.
- `crates/engine/tests/keeper_and_shots.rs` (new): the outcome scenes, the penalty scenes, and the slow set-piece, census and refit tests.
- `crates/engine/tests/content.rs`: pins the shot values; a tuning block without `shots` loads with the defaults.
- `crates/engine/tests/acting_keeper.rs`: the stand-in save test scripts the save and hold rolls, because saves no longer read `keeper_catch_chance`.
- `crates/engine/tests/commentary.rs`: the seed-7 line floor, re-derived.
- `docs/reference/data-files.md`: the `shots` rows, the new defaults and a paragraph on the model.
- `docs/reference/protocol.md`: the meaning of `stats.shots_on_target`. The field and the message are unchanged.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs`, `crates/engine/src/rules/mod.rs`, `crates/engine/src/tuning.rs` and `content/tuning.json`: also changed by `defending-and-discipline` and `lone-forward`. Their values and paths are unchanged. Saves use the acting keeper (`Simulation::keeper()`) from `defending-and-discipline`.
- `crates/engine/tests/content.rs`, `acting_keeper.rs` and `commentary.rs`: also changed by `lone-forward`.
- `docs/reference/data-files.md`: every tuning slice adds rows.

## Notes on Design Choices

- The on-target check runs the match's own `Ball::integrate()` on a copy, with the same `pitch::in_goal` and `crossbar_height` test that decides a goal. So a shot the engine calls on target and a goal cannot disagree. It runs once per shot and stops after at most 250 steps.
- The save roll, the hold roll and each block roll use `referee_draw()`, so a scene can script each outcome. The deflection angles and loft use the main stream.
- The save curve reads a fixed quality model (`shots.quality`, the earlier xG coefficients). The refit changed the reported `xg` only. Running the refit again on the refitted build gave the same coefficients to 4 decimals, so the refit does not feed back into play.
- A parried ball sets `keeper_beaten`, so the keeper who parried it does not catch it on the next tick. A blocked ball is a new flight: a keeper may still catch it with `keeper_catch_chance`.
- A parry or a block does not reset the offside positions. A save or a deflection is not a deliberate play by a defender (IFAB Law 11).
- `parry_speed` had to be above `control_speed` / `shot_speed` (0.44) for a parry to leave the keeper's reach. At the planned start value 0.4 every parried ball was slower than 12 m/s, and the keeper collected it at once (no corner in 40 parry scenes).

## Verification Seams Built

- Set pieces arise naturally → `Simulation::last_touch()` at `crates/engine/src/sim.rs:758` and the slow test `set_pieces_arise_from_play` at `crates/engine/tests/keeper_and_shots.rs:231`. The test checks each corner's origin and the corner and goal-kick rates over 200 matches.
- Shots are counted honestly → no new seam. `engine-cli calibrate --suite equal` already reports `shots_on_target_share` and `goals_per_xg`. The on-target count is at `crates/engine/src/sim.rs:987`.
- A wide shot stays wide → the wide and over scenes at `crates/engine/tests/keeper_and_shots.rs:63` and `:83`, with a scripted save roll through `Scene::rolls()`, and `Simulation::shot_flight()` at `crates/engine/src/sim.rs:770`.
- Penalties convert realistically → `Scene::penalty()` at `crates/engine/src/scenario.rs:157` and `penalties_convert_seventy_to_eighty_five_percent` at `crates/engine/tests/keeper_and_shots.rs:212`.
- No advantage from a red card → no new seam. The existing slow test `a_sending_off_gives_no_advantage` and `calibrate --suite red-card`.
- Tuning diagnostic → `ShotCensus` at `crates/engine/src/sim.rs:444` and `shot_outcome_census` at `crates/engine/tests/keeper_and_shots.rs:277`.

## Tuning Loop (step 12)

Inner loop: `calibrate --suite equal --seed 42 --matches 200 --baseline implement-evidence/keeper-and-shots/baseline/set-base/report.json`, each setting in a copy of the content folder passed with `--content-dir` (`implement-evidence/keeper-and-shots/tune.py`, log `tuning-log.txt`, 22 rows). Rows t00–t11 use `parry_speed` 0.6 as the base. Rows g01–g09 use the xG coefficients from before the refit. `diff` is the number of band changes the baseline diff marks as real (not noise). Baseline (`set-base`): on target 0.772, goals per xG 1.571, corners 0.008, goal kicks 5.37, goals 2.68.

| Row | Setting (changes from the start values) | On target | Goals/xG | Corners/team | Goal kicks | Goals | Verdict |
|---|---|---|---|---|---|---|---|
| t00 | parry_speed 0.6 | 0.634 | 1.669 | 1.71 | 8.81 | 3.28 | on target too high |
| t01 | shot_noise 0.4 | 0.479 | 1.211 | 1.16 | 11.71 | 2.23 | noise moves on target |
| t02 | + block_reach 2.0 | 0.470 | 1.262 | 1.22 | 11.46 | 2.33 | blocks barely move |
| t03 | + block_spread 2.0 | 0.467 | 1.217 | 1.22 | 11.55 | 2.25 | noise |
| t04 | shot_noise 0.5, loft_max 9, block_reach 2.5, block_spread 2.5 | 0.312 | 0.749 | 0.95 | 13.92 | 1.33 | too many misses |
| t05 | shot_noise 0.4, parry_spread 0.6 | 0.448 | 1.165 | 1.12 | 12.54 | 2.25 | narrow parries go to the touchline |
| t06 | t05 + block_reach 3, block_chance 1, block_spread 2.5 | 0.444 | 0.880 | 1.30 | 9.18 | 1.66 | blocks cost goal kicks |
| t07 | shot_noise 0.4, parry_spread 0.3, save_hold 0.2 | 0.425 | 1.068 | 0.92 | 13.04 | 2.08 | narrower is worse |
| t08 | t05 + keeper_reach 3.5 | 0.438 | 0.925 | 1.02 | 12.63 | 1.70 | no corner gain |
| t09 | shot_noise 0.4, save_hold 0.1, parry_speed 0.8 | 0.452 | 1.177 | 1.65 | 12.39 | 2.19 | more parries help a little |
| t10 | shot_noise 0.4, blocks at their bounds (3, 1, 3.2, 0.8) | 0.466 | 0.997 | 1.97 | 8.41 | 1.91 | goal kicks under 10 |
| t11 | t09 + t10 | 0.450 | 1.038 | 2.33 | 9.22 | 2.06 | ceiling of the model; goal kicks under 10 |
| r01 | shot_noise 0.45, block_reach 2, block_chance 0.7, block_spread 2 | 0.422 | 1.090 | 1.14 | 11.69 | 1.94 | red card: all three arms fail |
| g01 | shot_noise 0.45, block_reach 3, block_chance 0.5, block_spread 3.2, block_speed 0.8, parry_speed 0.8 | 0.422 | 1.005 | 1.51 | 11.00 | 1.77 | on target at the edge |
| g02 | g01 + block_chance 1.0 | 0.417 | 0.909 | 1.84 | 9.40 | 1.72 | goal kicks under 10 |
| g03 | g01 with shot_noise 0.5 | 0.387 | 0.926 | 1.51 | 12.23 | 1.79 | **shipped** (sourced save_hold) |
| g04 | g01 with shot_noise 0.55 | 0.346 | 0.894 | 1.31 | 12.22 | 1.64 | fewer corners |
| g05 | g01 with shot_noise 0.4, loft_max 9 | 0.379 | 0.879 | 1.42 | 12.60 | 1.68 | loft and noise trade |
| g06 | g03 + save_hold 0.2 | 0.384 | 0.921 | 1.70 | 12.22 | 1.77 | +0.19 corners for a non-sourced hold |
| g07 | g06 + parry_spread 0.9 | 0.371 | 0.872 | 1.59 | 11.93 | 1.61 | worse |
| g08 | g06 + parry_spread 1.5, parry_loft 8 | 0.377 | 1.034 | 1.48 | 11.72 | 1.91 | worse |
| g09 | g03 + save_hold 0.0 | 0.380 | 0.886 | 1.97 | 12.10 | 1.68 | every save parried: still 1.97 |

Outcome census and ceiling runs (200 matches, the slow tests) are in `implement-evidence/keeper-and-shots/census-and-ceiling.txt`. At the shipped values: wide or over 51.4%, blocked 12.1%, held 9.0%, parried 18.8%, scored 6.8%, other 1.9%. With the blocks and parries at their bounds and one save in ten held, blocks reach 22.3% (a realistic block rate) and corners 1.88 per team. An experiment that turned every parry toward the goal line, not shipped, reached 2.64 corners per team at the sourced hold and 3.18 at a hold of 0.1. Goal kicks then fell to 9.4–9.6 per match.

Penalty scenes: `penalty_spread` 0.5 converted 0.796 at `shot_noise` 0.25. At `shot_noise` 0.5 it converted 0.670, and 0.25 restores 0.796. The kick-from-the-mark noise is the product of the two.

xG refit (step 13): `xg_refit` on seeds 1–1000 found 21,479 open-play shots and 1,346 goals, and fitted intercept −4.1910, distance −0.0441 and angle 6.3836. All three are inside their bounds. The first refit landed in band (goals per xG 1.0976), so no second refit ran.

## Criterion Results

Release build of `8fd407b`'s tree. Logs are in `implement-evidence/keeper-and-shots/tuned/`.

| Criterion | Classification | Run | Result |
|---|---|---|---|
| Set pieces arise naturally | runtime-evidence | `set_pieces_arise_from_play` (200 matches) | **FAIL**: corners 1.205 per team (floor 3.0). Goal kicks 11.205 per match (floor 10) pass. Every corner followed a defending touch within one tick of the goal line (0 exceptions). |
| Shots are counted honestly | runtime-evidence | `calibrate --suite equal --seed 42 --matches 1000` | pass: on target 0.387 (0.30–0.42), goals per xG 1.0976 (0.85–1.15) |
| A wide shot stays wide | build-capability | the wide and over scenes | pass: not on target, no save with a scripted successful roll in reach, goal kick to the defending side |
| Penalties convert realistically | runtime-evidence | 500 penalty scenes | pass: 398 of 500, 0.796 (0.70–0.85) |
| No advantage from a red card | runtime-evidence | `a_sending_off_gives_no_advantage` (240 matches per arm, balanced) | **FAIL**: control home 0.7333, limit 1.1733. Keeper: full 1.1833, reduced 2.1000 (and full above the limit). Centre-back: 0.7667 against 0.8333. Striker: 1.0083 against 1.2667. Shots 15.0 per team per match in the suite. `calibrate --suite red-card --seed 1 --matches 240` gives the same figures. |

Also recorded (step 14): `every_formation_holds` passes. The largest pairing is 3-4-3 against 4-4-2 at 2.88–0.88, and 4-4-1-1 against 4-4-2 is 2.87–0.65, so no pairing regressed. `discipline_is_realistic` passes: 0.025 second yellows per match, sending-off share 11.5%, 0 same-tick pairs.

Other seed-42 bands (1,000 matches, recorded, not failed; Q-E4): goals 1.884 per match (band 2.4–3.2; `realism-tuning` owns goals), goalless share 0.22, corners 1.59 per team, throw-ins 22.8, passes 1,225 per team and yellow cards 0.935 are outside their bands. Shots per team (13.2), possession and the sending-off share are inside. Benchmark fields on that run: 1.5638 µs per tick CPU and 7.60 MB peak. That is informational only, because the `05c-benchmark.md` compare was not run.

## Checks Run

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass (exit 0).
- `cargo test --workspace --all-features --no-fail-fast`: 504 passed, 0 failed, 10 ignored (`tuned/workspace-tests.log`).
- `cargo test --release -p engine --all-features --test keeper_and_shots -- --include-ignored --nocapture`: 9 passed, 1 failed (`set_pieces_arise_from_play`) (`tuned/keeper_and_shots.log`).
- `cargo test --release -p engine --all-features --test defending -- --include-ignored a_sending_off_gives_no_advantage every_formation_holds`: 1 passed, 1 failed (`a_sending_off_gives_no_advantage`) (`tuned/defending.log`).
- `cargo test --release -p engine --all-features --test discipline -- --include-ignored`: 4 passed (`tuned/discipline.log`).
- Benchmark compare from `05c-benchmark.md`: skip. The slice stopped at step 14.
- Slice gate (equal and strength on five seeds, formations on one seed): skip. The slice stopped at step 14.
- The `strength` slow test: skip. Same reason.

Moved expectations, re-derived (step 15.1). No tolerance was widened:
- `acting_keeper.rs` `a_fast_shot_at_the_stand_in_is_held_with_the_catch_chance` → `..._is_saved_with_the_save_roll`. Before, the scene set `keeper_catch_chance` 1.0 or 0.0. Now it scripts the rolls `[0.0, 0.0]` (held) and `[0.99]` (beaten), and a beaten stand-in concedes the goal. The reason is that a shot's save no longer reads `keeper_catch_chance`.
- `commentary.rs` seed-7 line count: 84 before, 75 now. The floor moves from 80 to 70. The floor only guards against an empty check, and the reason is that fewer shots are saved and more go wide, so there are fewer lines.
- No other pinned test moved: `rules_shootout.rs`, `rules_extra_time.rs`, `match_stats.rs`, `full_match.rs`, `determinism.rs`, `strength.rs` (fast part), the `sim.rs` unit tests and `stream_cli.rs` pass unchanged.

## Deviations from Plan

1. **Census seam in the engine.** A11 said the census would read outcomes through `last_touch()`, `carrier()` and the stoppages. That cannot tell a parry from a block from outside. The run added `ShotCensus`, `shot_census()` and `shot_flight()` under the `scenario` feature. A release build does not contain them. No `Summary`, snapshot, report or protocol field changed. (class: implementation-detail)
2. **A block has no loft.** The plan names no block loft value, so a blocked ball stays on the ground (`deflect(..., loft 0)`). (class: implementation-detail)
3. **No on-target recompute after a deflection.** Plan steps 6 and 7 say to recompute the flag after a block or parry, and also to end the shot. The shot ends, so the flag is cleared instead. A deflected ball is not a shot, and nothing is counted again, as the plan requires. (class: implementation-detail)
4. **The corner-origin check.** Plan step 11 said the ball's previous record is at or beyond the goal line. The record before the corner's tick is still inside the pitch, because the crossing and the stoppage happen in the same tick. The test instead requires the previous record within 0.81 m of the goal line (one tick at the 40 m/s speed cap), plus the defending side as the last touch. (class: implementation-detail)
5. **Step 15 not run.** The benchmark compare, the slice gate and the `strength` slow test did not run, because plan step 14 stops on the red-card failure. The formations and discipline tests from step 14 ran. (class: implementation-detail)
6. **`docs/reference/protocol.md` edited.** The file is not in the plan's list. Its sentence on `stats.shots_on_target` described the old count. Only the description changed, not the field. (class: implementation-detail)

## Anything Deferred

- Corners from clearances, crosses and headers. The slice scope puts headers from corners and corner routines out, and plan A8 keeps `keeper_catch_chance` for fast balls that are not shots. This is the source that Q-KS2 option 3 names.
- The slice gate and the benchmark compare, until Q-KS1 and Q-KS2 are answered and a setting is accepted.

## Known Risks / Caveats

- `shots.block_spread` 3.2 means that a blocked ball can go in any direction, including toward the defending goal. A blocked shot can therefore become an own goal, which the census counts in neither the scored nor the blocked shots. At the shipped values the census names 98.1% of shots.
- `save_hold` stays at the sourced 0.333. A lower hold gives more corners (g06, g09), but it is not a sourced figure, so the run did not ship one.
- Goals per match fell to 1.88 (band 2.4–3.2). The product owner accepted goals out of band until `realism-tuning`.

## Blockers

- **Q-KS1 (class: intent-bearing, AWAITING INPUT; `po-answers.md`).** The save and block model does not fix "No advantage from a red card". On the balanced fixtures (seeds 1–120 in both club orders) every arm fails with every limit unchanged:
  - Keeper: the reduced side scores 2.10 against 1.18. The full side's 1.18 is also above the limit of 1.17 (1.6 × control 0.73).
  - Centre-back: 0.83 against 0.77.
  - Striker: 1.27 against 1.01.
  - Before this slice (`red-base`, `a30313b`) the figures were keeper 2.67 against 1.64, centre-back 1.08 against 0.88 and striker 1.79 against 1.21. The gaps narrowed, but none closed.

  The steer entry for Q-LF2 says to stop and report in this case, and never to raise a limit. Options for the product owner:
  1. Allow a lever on the side with ten men (for example less pressing, running or attacking width). This is the measured cause, and no slice owns it now.
  2. Move the criterion again, to `tempo-and-restarts` or `realism-tuning`.
  3. Judge each arm against the reduced side's own 11-against-11 figure instead of against the full side.

- **Q-KS2 (class: intent-bearing, AWAITING INPUT; `po-answers.md`).** "Corners per team at least 3.0" is not reachable with the save and block model at any in-bounds setting that keeps goal kicks at 10 or more and on target at 42% or less. The ceiling of the planned model is 1.88–2.33 per team, with every block and parry value at its bound (t11, census C). At the shipped values it is 1.21 (slow test) and 1.59 (seed 42). At the shipped values, shots give about 4.1 parries and 2.6 blocks per match, and about 36% of those deflections end in a corner (2.4 corners per match). Options:
  1. Lower the slice's corner floor (for example to 1.2 per team) and leave the 3.5–6.5 band to later slices.
  2. Allow parries to be turned toward the goal line and a non-sourced hold share. The experiment reached 3.18 per team, but goal kicks fell to 9.4.
  3. Add corner sources from clearances and crosses, in this slice or in `tempo-and-restarts`. That is a scope change.

  Each option changes a criterion, a sourced value or the slice scope, so this run may not choose one.

## Assumptions

- A1 (class: implementation-detail): `parry_speed` 0.8 and `penalty_spread` 0.25 are tuned values within their bounds, not deviations. The plan's start values were 0.4 and 0.5. At 0.4 every parry stayed with the keeper. At 0.5, with `shot_noise` 0.5, penalties converted 0.670.
- A2 (class: implementation-detail): g03 is the setting shipped in the stopped state. Among the settings with on target at 0.42 or less and goal kicks at 10 or more, it keeps the sourced `save_hold` (0.333), and its corners (1.51) are within noise of the best such setting at the sourced hold (g01, 1.51).
- A3 (class: implementation-detail): the planned parry model (turned up to `parry_spread` either side of the goal-line direction) is shipped. The toward-the-line model was measured as an experiment only (census D, scene probe E) and removed from the code before the commit.
- A4 (class: implementation-detail): the shoot-out save rolls now use `referee_draw()`, as the open-play save rolls do. The shoot-out scenes script outcomes with `Scene::shootout_kicks`, so no shoot-out test moved.
- A5 (class: implementation-detail): each penalty scene uses the taker the engine picks (`restart::taker`), puts the defending acting keeper 0.5 m inside his goal line, and plays the kick on the first step. The home side takes the penalty on odd seeds and the away side on even seeds.
- A6 (class: implementation-detail): the code is committed with two criteria failing, as `lone-forward` did at its stops. The play is the planned model and every earlier check passes. The failing criteria are recorded here and in `po-answers.md`.
- A7 (class: implementation-detail; ac: "Set pieces arise naturally"; classification: runtime-evidence): measured by `set_pieces_arise_from_play` on 200 matches. It fails on corners.
- A8 (class: implementation-detail; ac: "Shots are counted honestly"; classification: runtime-evidence): measured by `calibrate --suite equal --seed 42 --matches 1000`. It passes.
- A9 (class: implementation-detail; ac: "A wide shot stays wide"; classification: build-capability): the wide and over scenes on the real engine. They pass.
- A10 (class: implementation-detail; ac: "Penalties convert realistically"; classification: runtime-evidence): 500 scenes. It passes at 0.796.
- A11 (class: implementation-detail; ac: "No advantage from a red card"; classification: runtime-evidence): the unchanged slow test on balanced fixtures. It fails, so the run stops.

## Freshness Research

- No dependency was added or upgraded. `glam`'s `DVec2::normalize_or_zero` and `perp_dot` are already in use in this crate (`sim.rs` `shot_xg`). The nested `#[serde(default)]` block with `garde(dive)` follows the proven `decision` and `xg` pattern. The new test `a_tuning_block_without_the_shot_values_loads_with_the_defaults` proves the default.
- Sources for the values: the save curve 0.891–0.272 and the "two in three not held" share come from `docs/design/realism/01-engine-realism.md:393-397`. The penalty xG of 0.76 is the conventional value in public xG models (plan A9).

## Recommended Next Stage

- **Option D (default): Blocked.** Q-KS1 and Q-KS2 need product-owner answers. Run `/wf implement football-manager-match-engine keeper-and-shots` again after the answers are in `po-answers.md`.
- **Option C: Revisit plan.** Run `/wf plan football-manager-match-engine keeper-and-shots` if an answer adds a corner source, a lever for the ten-man side, or a new parry model.
- **Option A: Verify.** `/wf verify football-manager-match-engine keeper-and-shots` can check the three passing criteria now, but two criteria are not met. Workflow state lives in the artifact files, so compact the session before it if you take this route.
