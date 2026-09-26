---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: keeper-and-shots
status: complete
stage-number: 5
created-at: "2026-09-24T17:26:30Z"
updated-at: "2026-09-24T19:43:37Z"
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-24T19:43:37Z"
    trigger: answers-returned
    because: "the product owner answered Q-KS1 and Q-KS2 (2026-09-24T18:15:55Z): the red-card criterion moves to realism-tuning, and this slice's corner floor is 1.2 per team over 200 matches, with goal kicks at least 10 per match and every corner after a defending touch over the goal line"
    changed: "the set-piece slow test asserts the 1.2 floor (commit bce0144); every criterion run, the formations, discipline and strength slow tests, the benchmark drives and the full slice gate ran on the shipped values; the four criteria this slice keeps pass; status complete"
metric-files-changed: 14
metric-lines-added: 1072
metric-lines-removed: 66
metric-deviations-from-plan: 8
metric-review-fixes-applied: 0
commit-sha: "bce0144c9c3752b3bb950c48398448e7e5db3df7"
commits:
  - "8fd407b4bc6c50b1751e26a74d06ef6cc5bc8013"
  - "bce0144c9c3752b3bb950c48398448e7e5db3df7"
has-blockers: false
open-questions: []
steering-honored:
  - "keeper-and-shots criteria changes (Q-KS1, Q-KS2, 2026-09-24): this slice is not judged on \"No advantage from a red card\"; that criterion and its slow test belong to realism-tuning, unchanged. The corner floor is 1.2 per team over 200 matches, goal kicks stay at least 10 per match, and every corner still follows a defending touch over the goal line. No parry is turned toward the goal line, save_hold stays at the sourced 0.333, no limit was raised and no lever was added for a side with ten men."
  - "Q-KS2 hands corners from clearances and blocked crosses to tempo-and-restarts. This slice adds no such source."
  - "Q-I1: targeted calibrate runs were the inner loop; the full slice gate ran on the shipped values (equal and strength on seeds 1, 7, 42, 99 and 2026, formations on seed 42, 1,000 matches each)."
  - "Output boundary: code comments, test names, docs and both commit messages use product language; the added lines were leak-checked before each commit."
  - "Design direction: not applicable; no page changed."
tags: [engine, shots, set-pieces, realism]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-keeper-and-shots.md
  plan: 04-plan-keeper-and-shots.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/keeper-and-shots/
  resume-evidence: implement-evidence/keeper-and-shots/resume/
  history: [history/05-implement-keeper-and-shots-0.md]
  siblings: [05-implement-lone-forward.md, 05-implement-defending-and-discipline.md, 05-implement-tuning-loop.md, 05-implement-realism-bands-v2.md, 05-implement-extra-time-penalties.md]
  verify: 06-verify-keeper-and-shots.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine keeper-and-shots"
---

# Implement: Keeper and shots

## The Implementation

The slice started from the shipped lone-forward play at `a30313b`. There, 77% of shots counted as on target, goals per xG was 1.50, corners were 0.005 per team and goal kicks 5.9 per match (seed 42, 1,000 matches). A keeper held every save with one flat roll, no outfield player touched a shot, and no shot rose above 0.32 m. Commit `8fd407b` built the planned model. A shot flies on a trajectory and counts as on target only when that flight crosses between the posts under the bar. A keeper tries to save only such a shot, on the sourced curve (0.891 down to 0.272), and holds one save in three or parries it. An outfield defender within 3 m can block a shot once. A parry or a block leaves the defending side as the last touch, so a corner needs a real crossing. Penalties and the shoot-out use the same save model.

The first pass stopped at plan step 14 on two criteria: the red-card criterion, and corners at 1.2 per team against a 3.0 floor, with a model ceiling of 1.9–2.3. The product owner then moved the red-card criterion to `realism-tuning` (Q-KS1). The product owner also set this slice's corner floor to 1.2 per team, because corners from clearances and crosses belong to `tempo-and-restarts` (Q-KS2). So commit `bce0144` changes one test line and its comment, and no play value moves. On the shipped values, all four kept criteria pass. Corners are 1.205 per team and goal kicks 11.2 per match over 200 matches, with every corner after a defending touch over the line. On target is 0.376–0.387 and goals per xG 1.084–1.111 on all five gate seeds. The wide and over scenes pass, and 398 of 500 penalties convert (0.796). The workspace suite passes (504 tests, 0 failed). The benchmark median is 1.5847 µs per tick against the 1.7743 µs gate.

This slice lets `tempo-and-restarts` add corners from clearances and blocked crosses to a model where every corner already needs a crossing. The top risk is goals: 1.70–2.14 per match on the gate seeds, against the 2.4–3.2 band that `realism-tuning` owns. That slice also owns the red-card criterion, which still has no lever.

## Summary of Changes

- A new pure module `shot.rs`: `on_target()` steps a copy of the ball to the goal line, `quality()` reads the fixed quality model, `save_chance()` is the clamped sourced line, and `deflect()` turns a velocity. The caller takes every random draw.
- On target is counted at the kick (`kick_ball()`), and the flight keeps the on-target flag and the shot quality. The count in `goal()` and in a keeper's `gain()` is removed.
- A fast shot in flight is contested only by blockers and, when on target, by the acting keeper (`contest_shot()`, `try_block()`, `try_save()`, `parry()`). An off-target shot is never saved. `keeper_catch_chance` now covers only fast balls that are not shots.
- The shot loft is drawn from 0 to `shots.loft_max`. A penalty in play counts `shots.penalty_xg`, and its save roll reads that quality.
- The shoot-out keeper dives by `shots.keeper_dive_m` and `shots.keeper_stays` and saves only an on-target kick at the penalty quality: a held kick is a miss, a parried one plays on. `SHOOTOUT_SPREAD`, `SHOOTOUT_HOLD`, `KEEPER_DIVE_M`, `KEEPER_STAYS` and their known-limit comment are gone.
- A new optional `shots` tuning block (16 values with bounds). `shot_noise` 0.5 and refitted `xg` coefficients (−4.191, −0.0441, 6.3836). A tuning file without the block loads with the shipped values, and `TUNING_VERSION` stays 2.
- Test seams under the `scenario` feature: `Scene::penalty()`, `Simulation::last_touch()`, `shot_census()` and `shot_flight()`.
- New tests: 6 unit tests in `shot.rs`, 7 fast scenes and 3 slow tests in `tests/keeper_and_shots.rs`, and 2 content pins. Two moved expectations were re-derived.
- The set-piece slow test asserts at least 1.2 corners per team (Q-KS2). Its module comment says the floor covers corners from saves and blocks only, and that the 3.5–6.5 band is not asserted there.

## Files Changed

- `crates/engine/src/shot.rs` (new): trajectory on-target check, shot quality, save chance, deflection, and unit tests.
- `crates/engine/src/sim.rs`: flight fields, `kick_ball()` with the on-target count and the penalty xG, the shot contest, blocks, the save, the parry, `end_shot()`, the `ShotCensus` seam and three accessors under the scenario feature. The on-target count in `gain()` is removed.
- `crates/engine/src/rules/mod.rs`: `goal()` no longer counts on target. `ball_out()` records a wide shot in the census. The penalty in `take_restart()` goes through `kick_ball(..., penalty = true)`. The shoot-out dive and save use the tuning values, and the four constants are removed.
- `crates/engine/src/tuning.rs`: the `ShotTuning` block, its bounds and `Default`; `shot_noise`, `penalty_spread`, the block and parry values and the `xg` defaults as tuned.
- `crates/engine/src/decision.rs`: the shot loft reads `shots.loft_max`.
- `crates/engine/src/scenario.rs`: `Scene::penalty(team, taker)`.
- `crates/engine/src/lib.rs`: the `shot` module line.
- `content/tuning.json`: the `shots` block, `shot_noise` 0.5 and the refitted `xg`.
- `crates/engine/tests/keeper_and_shots.rs` (new): the outcome scenes, the penalty scenes, and the slow set-piece, census and refit tests. The set-piece floor is 1.2 corners per team (`bce0144`).
- `crates/engine/tests/content.rs`: pins the shot values; a tuning block without `shots` loads with the defaults.
- `crates/engine/tests/acting_keeper.rs`: the stand-in save test scripts the save and hold rolls, because saves no longer read `keeper_catch_chance`.
- `crates/engine/tests/commentary.rs`: the seed-7 line floor, re-derived.
- `docs/reference/data-files.md`: the `shots` rows, the new defaults and a paragraph on the model.
- `docs/reference/protocol.md`: the meaning of `stats.shots_on_target`. The field and the message are unchanged.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs`, `crates/engine/src/rules/mod.rs`, `crates/engine/src/tuning.rs` and `content/tuning.json`: also changed by `defending-and-discipline` and `lone-forward`. Their values and paths are unchanged. Saves use the acting keeper (`Simulation::keeper()`) from `defending-and-discipline`.
- `crates/engine/tests/content.rs`, `acting_keeper.rs` and `commentary.rs`: also changed by `lone-forward`.
- `docs/reference/data-files.md`: every tuning slice adds rows.
- `tempo-and-restarts` will add corner sources (clearances and blocked crosses). The corner-origin check in `set_pieces_arise_from_play` (a defending last touch within one tick of the goal line) applies to those corners too.

## Notes on Design Choices

- The on-target check runs the match's own `Ball::integrate()` on a copy, with the same `pitch::in_goal` and `crossbar_height` test that decides a goal. So a shot the engine calls on target and a goal cannot disagree. It runs once per shot and stops after at most 250 steps.
- The save roll, the hold roll and each block roll use `referee_draw()`, so a scene can script each outcome. The deflection angles and loft use the main stream.
- The save curve reads a fixed quality model (`shots.quality`, the earlier xG coefficients). The refit changed the reported `xg` only. Running the refit again on the refitted build gave the same coefficients to 4 decimals, so the refit does not feed back into play. This run's `xg_refit` printed the same three coefficients again (`resume/keeper_and_shots.log`).
- A parried ball sets `keeper_beaten`, so the keeper who parried it does not catch it on the next tick. A blocked ball is a new flight: a keeper may still catch it with `keeper_catch_chance`.
- A parry or a block does not reset the offside positions. A save or a deflection is not a deliberate play by a defender (IFAB Law 11).
- `parry_speed` had to be above `control_speed` / `shot_speed` (0.44) for a parry to leave the keeper's reach. At the planned start value 0.4 every parried ball was slower than 12 m/s, and the keeper collected it at once (no corner in 40 parry scenes).
- The 1.2 floor sits 0.005 under the measured 1.205. The run is seeded and deterministic: this run and the first pass printed the same 1.205 on the same tree of play code. A play change in a later slice that lowers parries or blocks will trip it, which is the intent.

## Verification Seams Built

Re-opened after the last edit (`bce0144`):
- Set pieces arise naturally → `Simulation::last_touch()` at `crates/engine/src/sim.rs:758` and the slow test `set_pieces_arise_from_play` at `crates/engine/tests/keeper_and_shots.rs:233`, asserting the 1.2 floor at `:275` (enables `cargo test --release ... set_pieces_arise_from_play` to check each corner's origin and both rates over 200 matches).
- Shots are counted honestly → no new seam. `engine-cli calibrate --suite equal` already reports `shots_on_target_share` and `goals_per_xg`. The on-target count is at `crates/engine/src/sim.rs:990`.
- A wide shot stays wide → the wide and over scenes at `crates/engine/tests/keeper_and_shots.rs:65` and `:85`, with a scripted save roll through `Scene::rolls()`, and `Simulation::shot_flight()` at `crates/engine/src/sim.rs:770`.
- Penalties convert realistically → `Scene::penalty()` at `crates/engine/src/scenario.rs:157` and `penalties_convert_seventy_to_eighty_five_percent` at `crates/engine/tests/keeper_and_shots.rs:214`.
- Tuning diagnostic → `ShotCensus` at `crates/engine/src/sim.rs:444` and `shot_outcome_census` at `crates/engine/tests/keeper_and_shots.rs:279`.

## Tuning Loop (step 12)

Inner loop: `calibrate --suite equal --seed 42 --matches 200 --baseline implement-evidence/keeper-and-shots/baseline/set-base/report.json`, each setting in a copy of the content folder passed with `--content-dir` (`implement-evidence/keeper-and-shots/tune.py`, log `tuning-log.txt`, 22 rows). Rows t00–t11 use `parry_speed` 0.6 as the base. Rows g01–g09 use the xG coefficients from before the refit. Baseline (`set-base`): on target 0.772, goals per xG 1.571, corners 0.008, goal kicks 5.37, goals 2.68.

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

The outcome census and the ceiling runs (200 matches, the slow tests) are in `implement-evidence/keeper-and-shots/census-and-ceiling.txt`. This run's census, on the shipped values, printed held 0.090, parried 0.188, scored 0.068 and other 0.019 (`resume/keeper_and_shots.log`), the same as the first pass. With the blocks and parries at their bounds and one save in ten held, blocks reach 22.3% and corners 1.88 per team. An experiment that turned every parry toward the goal line reached 2.64–3.18 corners per team, with goal kicks at 9.4–9.6. That experiment was not shipped, and Q-KS2 rules it out.

Penalty scenes: `penalty_spread` 0.5 converted 0.796 at `shot_noise` 0.25. At `shot_noise` 0.5 it converted 0.670, and 0.25 restores 0.796. The kick-from-the-mark noise is the product of the two.

xG refit (step 13): `xg_refit` on seeds 1–1000 found 21,479 open-play shots and 1,346 goals, and fitted intercept −4.1910, distance −0.0441 and angle 6.3836. All three are inside their bounds. The first refit landed in band (goals per xG 1.0976), so no second refit ran.

## Criterion Results

Release build of `bce0144`. Logs are in `implement-evidence/keeper-and-shots/resume/`.

| Criterion | Classification | Run | Result |
|---|---|---|---|
| Set pieces arise naturally (floor 1.2 per team, Q-KS2) | runtime-evidence | `set_pieces_arise_from_play` (200 matches), `resume/keeper_and_shots.log` | pass: corners 1.205 per team (floor 1.2), goal kicks 11.205 per match (floor 10). The test asserts that no corner lacks a defending last touch within one tick of the goal line, and it passed. |
| Shots are counted honestly | runtime-evidence | `calibrate --suite equal --seed 42 --matches 1000` (`resume/gate/equal-42/report.json`), and seeds 1, 7, 99, 2026 | pass: on target 0.387 and goals per xG 1.0976 on seed 42. On the other four seeds, on target is 0.376–0.384 and goals per xG 1.084–1.111. Band 0.30–0.42 and 0.85–1.15. |
| A wide shot stays wide | build-capability | the wide and over scenes | pass: not on target, no save with a scripted successful roll in reach, goal kick to the defending side |
| Penalties convert realistically | runtime-evidence | 500 penalty scenes | pass: 398 of 500, 0.796 (0.70–0.85) |

"No advantage from a red card" is not a criterion of this slice (Q-KS1). Its slow test and the `red-card` suite are unchanged and were not run again. The first pass's figures on the same play values are in the history snapshot and `tuned/defending.log`. Keeper arm: 2.10 against 1.18. Centre-back arm: 0.83 against 0.77. Striker arm: 1.27 against 1.01. All three arms fail, with the limit at 1.17.

Also recorded (step 14): `every_formation_holds` passes. 4-4-1-1 against 4-4-2 is 2.87–0.65 and 3-4-3 against 4-4-2 is 2.88–0.88, so no pairing regressed. `discipline_is_realistic` passes: 0.025 second yellows per match, sending-off share 11.5%, 0 same-tick pairs. The strength slow test passes: the stronger team won 140, drew 33 and lost 27 of 200 (`resume/slow-formations-discipline-strength.log`).

## Checks Run

This revision (on the tree of `bce0144`):
- `cargo fmt --all -- --check`: pass (exit 0, `resume/fmt.log`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass (exit 0, `resume/clippy.log`).
- `cargo test --workspace --all-features --no-fail-fast`: exit 0; 65 result lines, 504 passed, 0 failed, 10 ignored (`resume/workspace-tests.log`).
- `cargo test --release -p engine --all-features --test keeper_and_shots -- --include-ignored --nocapture`: exit 0; 10 passed, 0 failed (`resume/keeper_and_shots.log`).
- `cargo test --release -p engine --all-features --test defending --test discipline --test strength -- --include-ignored --nocapture every_formation_holds discipline_is_realistic stronger`: exit 0; 3 passed (`resume/slow-formations-discipline-strength.log`).
- Benchmark drives from `05c-benchmark.md` (`engine-cli bench --seed 42 --matches 5 --json`, three drives, `resume/bench/bench-*.json`, each exit 0, with `SM_DATA_DIR` in the scratch folder and no other heavy process running): 1.5854, 1.5847 and 1.5743 µs per tick (median 1.5847; gate 1.7743). Peak memory 6.836, 6.840 and 6.832 MB (gate 8.39). 453.2, 453.0 and 450.0 ms per match. 285,850 ticks per match, against 277,000 at baseline, because more corners and goal kicks add dead-ball time. This is the implement-time reading. Verify owns the compare and the update of `05c-benchmark.md`.
- Slice gate (1,000 matches each, `resume/gate/`):
  - `--suite strength`, seeds 1, 7, 42, 99, 2026: every run passes (exit 0). The stronger team's win rate is 0.671, 0.677, 0.642, 0.673 and 0.648.
  - `--suite equal`, seeds 1, 7, 42, 99, 2026: exit 2 on each, which is the band-miss exit (recorded, not failed; Q-E4). Goals per match 1.728, 2.016, 1.884, 2.135 and 1.702. Shots per team 12.3, 13.9, 13.2, 14.3 and 12.2. Corners per team 1.453, 1.649, 1.590, 1.625 and 1.448. Goal kicks per match 12.3, 13.8, 13.1, 14.4 and 12.2. Sending-off share 0.107, 0.124, 0.098, 0.098 and 0.101. Band misses are 6 of 16 on every seed: corners, goalless share, goals, passes, throw-ins and yellow cards. At lone-forward the count was 7–8 of 16. Goal kicks, goals per xG and the on-target share are now in band.
  - Seed 42 against `baseline/equal-base` (the report diff): 9 changes and 6 noise rows. Corners 0.005 → 1.590 and goal kicks 5.875 → 13.087 (into band). On target 0.7728 → 0.387 and goals per xG 1.4955 → 1.0976 (into band). Throw-ins 18.1 → 22.8. Goals 2.841 → 1.884 left the band (2.4–3.2), and the goalless share 0.099 → 0.220 left the band. The product owner accepted goals out of band until `realism-tuning`.
  - `--suite formations --seed 42`: __FORMATIONS__
- Output-boundary search of the changed test lines and the commit message for workflow vocabulary: clean.

Moved expectations, re-derived (step 15.1). No tolerance was widened:
- `acting_keeper.rs` `a_fast_shot_at_the_stand_in_is_held_with_the_catch_chance` → `..._is_saved_with_the_save_roll`. Before, the scene set `keeper_catch_chance` 1.0 or 0.0. Now it scripts the rolls `[0.0, 0.0]` (held) and `[0.99]` (beaten), and a beaten stand-in concedes the goal. The reason is that a shot's save no longer reads `keeper_catch_chance`.
- `commentary.rs` seed-7 line count: 84 before, 75 now. The floor moves from 80 to 70. The floor only guards against an empty check. There are fewer lines because fewer shots are saved and more go wide.
- The corner floor in `set_pieces_arise_from_play` moves from 3.0 to 1.2 per team. This is not a re-derived expectation. It is the product owner's new criterion (Q-KS2).
- No other pinned test moved: `rules_shootout.rs`, `rules_extra_time.rs`, `match_stats.rs`, `full_match.rs`, `determinism.rs`, `strength.rs` (fast part), the `sim.rs` unit tests and `stream_cli.rs` pass unchanged.

## Deviations from Plan

1. **Census seam in the engine.** A11 said the census would read outcomes through `last_touch()`, `carrier()` and the stoppages. That cannot tell a parry from a block from outside. The run added `ShotCensus`, `shot_census()` and `shot_flight()` under the `scenario` feature. A release build does not contain them. No `Summary`, snapshot, report or protocol field changed. (class: implementation-detail)
2. **A block has no loft.** The plan names no block loft value, so a blocked ball stays on the ground (`deflect(..., loft 0)`). (class: implementation-detail)
3. **No on-target recompute after a deflection.** Plan steps 6 and 7 say to recompute the flag after a block or parry, and also to end the shot. The shot ends, so the flag is cleared instead. A deflected ball is not a shot, and nothing is counted again, as the plan requires. (class: implementation-detail)
4. **The corner-origin check.** Plan step 11 said the ball's previous record is at or beyond the goal line. The record before the corner's tick is still inside the pitch, because the crossing and the stoppage happen in the same tick. The test instead requires the previous record within 0.81 m of the goal line (one tick at the 40 m/s speed cap), plus the defending side as the last touch. (class: implementation-detail)
5. **`docs/reference/protocol.md` edited.** The file is not in the plan's list. Its sentence on `stats.shots_on_target` described the old count. Only the description changed, not the field. (class: implementation-detail)
6. **The corner floor is 1.2, not 3.0.** Plan step 11 asserts 3.0 corners per team. The product owner set 1.2 (Q-KS2), so the test asserts 1.2. The goal-kick floor and the origin check are unchanged. (class: implementation-detail; applies a recorded product-owner answer)
7. **The red-card test is not a criterion run.** Plan step 14 runs `a_sending_off_gives_no_advantage` and stops if it fails. The product owner moved that criterion to `realism-tuning` (Q-KS1), so this run did not run it. Its slow test still fails, as recorded. (class: implementation-detail; applies a recorded product-owner answer)
8. **The benchmark readings are recorded but the benchmark artifact is not changed.** Plan step 15.4 says to run the compare. This run ran the three gate drives and records them here. It does not rewrite `05c-benchmark.md` into compare mode, because the implement contract gives the compare to verify. (class: implementation-detail)

## Anything Deferred

- Corners from clearances and blocked crosses over a side's own goal line: moved to `tempo-and-restarts` by the product owner (Q-KS2). Until then, corners stay at 1.45–1.65 per team (gate seeds), under the 3.5–6.5 band.
- "No advantage from a red card", on the balanced fixtures of Q-LF1 with every limit unchanged: moved to `realism-tuning` by the product owner (Q-KS1). The measured cause, a side with ten men that attacks as if it had eleven, has no lever in any slice.
- Final goals-band tuning: `realism-tuning`.

## Known Risks / Caveats

- `shots.block_spread` 3.2 means that a blocked ball can go in any direction, including toward the defending goal. A blocked shot can therefore become an own goal, which the census counts in neither the scored nor the blocked shots. At the shipped values the census names 98.1% of shots.
- `save_hold` stays at the sourced 0.333. A lower hold gives more corners (g06, g09), but it is not a sourced figure, and Q-KS2 rules it out.
- Goals per match are 1.70–2.14 on the gate seeds (band 2.4–3.2), and the goalless share is 0.22. The product owner accepted goals out of band until `realism-tuning`.
- The corner floor has 0.005 of headroom on the seeded 200-match run. A later play change that lowers parries or blocks will fail it. The slice that makes that change must measure and record why.

## Assumptions

- A1 (class: implementation-detail): `parry_speed` 0.8 and `penalty_spread` 0.25 are tuned values within their bounds, not deviations. The plan's start values were 0.4 and 0.5. At 0.4 every parry stayed with the keeper. At 0.5, with `shot_noise` 0.5, penalties converted 0.670.
- A2 (class: implementation-detail): g03 is the shipped setting. Among the settings with on target at 0.42 or less and goal kicks at 10 or more, it keeps the sourced `save_hold` (0.333). Its corners (1.51) are within noise of the best such setting at the sourced hold (g01, 1.51).
- A3 (class: implementation-detail): the planned parry model is shipped. A parry turns up to `parry_spread` either side of the goal-line direction. The toward-the-line model was an experiment only, and it was removed before the first commit. Q-KS2 rules it out.
- A4 (class: implementation-detail): the shoot-out save rolls now use `referee_draw()`, as the open-play save rolls do. The shoot-out scenes script outcomes with `Scene::shootout_kicks`, so no shoot-out test moved.
- A5 (class: implementation-detail): each penalty scene uses the taker the engine picks (`restart::taker`), puts the defending acting keeper 0.5 m inside his goal line, and plays the kick on the first step. The home side takes the penalty on odd seeds and the away side on even seeds.
- A6 (class: implementation-detail): the answers need no play change. The shipped values already meet the new floor (1.205 ≥ 1.2), and Q-KS2 forbids the two corner levers that were left (parries toward the line, a hold share that is not sourced). So `bce0144` changes only the assertion and its comment.
- A7 (class: implementation-detail; ac: "Set pieces arise naturally"; classification: runtime-evidence): measured by `set_pieces_arise_from_play` on 200 matches against the 1.2 floor of Q-KS2. It passes.
- A8 (class: implementation-detail; ac: "Shots are counted honestly"; classification: runtime-evidence): measured by `calibrate --suite equal --seed 42 --matches 1000`, and confirmed on the other four gate seeds. It passes.
- A9 (class: implementation-detail; ac: "A wide shot stays wide"; classification: build-capability): the wide and over scenes on the real engine. They pass.
- A10 (class: implementation-detail; ac: "Penalties convert realistically"; classification: runtime-evidence): 500 scenes. It passes at 0.796.
- A11 (class: implementation-detail): the slice definition file keeps its original criterion text (3.0 corners). The change is in `steer.md` and `po-answers.md`, as for `defending-and-discipline` and `lone-forward`. Verify reads the floor from those answers.

## Triage Decisions

- Q-KS1 and Q-KS2 were intent-bearing. The first pass stopped on them and did not choose. Both have product-owner answers in `po-answers.md` (2026-09-24T18:15:55Z). This run applies them as written and makes no intent-bearing choice. Intent-bearing escapes: 0.

## Freshness Research

- No dependency was added or upgraded. `glam`'s `DVec2::normalize_or_zero` and `perp_dot` are already in use in this crate (`sim.rs` `shot_xg`). The nested `#[serde(default)]` block with `garde(dive)` follows the proven `decision` and `xg` pattern. The test `a_tuning_block_without_the_shot_values_loads_with_the_defaults` proves the default.
- Sources for the values: the save curve 0.891–0.272 and the "two in three not held" share come from `docs/design/realism/01-engine-realism.md:393-397`. The penalty xG of 0.76 is the conventional value in public xG models (plan A9).

## Recommended Next Stage

- **Option A (default): Verify.** Run `/wf verify football-manager-match-engine keeper-and-shots` on the four kept criteria, with the corner floor of Q-KS2. It also runs the benchmark compare against `05c-benchmark.md`. The evidence is in `implement-evidence/keeper-and-shots/resume/`. Workflow state lives in the artifact files on disk, so compact the session before verify.
- **Option B: Skip to review.** Not recommended. Every criterion is a runtime statistic or an engine scene.
- **Option C: Revisit plan.** Not needed. The answers changed one threshold and moved one criterion out, and no plan step failed.
