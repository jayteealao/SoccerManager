---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: keeper-and-shots
status: complete
stage-number: 4
created-at: "2026-09-24T16:36:13Z"
updated-at: "2026-09-24T16:36:13Z"
metric-files-to-touch: 14
metric-step-count: 15
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, shots, set-pieces, realism]
stack-source: confirmed
open-questions: []
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-keeper-and-shots.md
  siblings: [04-plan-lone-forward.md, 04-plan-defending-and-discipline.md, 04-plan-tuning-loop.md, 04-plan-realism-bands-v2.md, 04-plan-extra-time-penalties.md]
  benchmark: 05c-benchmark.md
  implement: 05-implement-keeper-and-shots.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine keeper-and-shots"
---

# Plan: Keeper and Shots

## The Plan

`lone-forward` shipped at `07d87c2` and passed verify on its three criteria. The shots are still wrong on that shipped play. In the five-seed gate of that slice, 76–77% of shots count as on target, goals per xG is 1.45–1.53, corners are 0.004–0.011 per team and goal kicks are 5.5–6.4 per match. The code shows why:
- A keeper holds every save he makes, with one flat 0.86 roll on any fast ball in his 2.6 m reach (`sim.rs:988-997`).
- An outfield player never touches a fast ball (`sim.rs:975`), so nobody blocks a shot.
- The shot loft is drawn from 0 to 2.5 m/s (`decision.rs:556`), so no shot rises above 0.32 m.
- A keeper catch counts as on target even when the shot was going wide (`sim.rs:1049-1054`).

The product owner also moved "No advantage from a red card" to this slice (steer.md, Q-LF2). On the shipped values the reduced side still outscores the full side in all three arms. For the keeper arm it is 2.67 against 1.64.

The plan follows the slice scope in physical terms. The kick records whether the shot's trajectory crosses between the posts under the bar, and that is the on-target count. A keeper attempts a save only on such a flight, with one roll on the sourced curve (89.1% for the poorest chances down to 27.2% for the best). A save is held or parried. An outfield defender near the ball can block once per flight. A parry or a block deflects the ball with the defending side as the last touch, so a corner comes only from a ball that physically crosses the line. The shot loft range lets a shot rise over the bar. Penalties and the shoot-out use the same save model, with a sourced penalty xG. The three hand-set shoot-out constants move into the tuning file. The save curve reads a fixed quality model (today's xG coefficients), so the xG refit at the end does not feed back into the saves. No event kind, report key, snapshot field or protocol message is added.

The work is 15 steps over 14 files, 2 of them new. The benchmark is re-baselined on `a30313b` at 1.613 µs per tick (gate 1.7743 µs, 8.39 MB). When it lands, `tempo-and-restarts` can plan on corners and goal kicks that arise from play. The top risk is the red-card criterion, because its measured cause is how a ten-man side attacks, and this slice does not own that. If the criterion still fails after tuning, implement stops and reports each arm. It never raises a limit.

## Current State

- Code: HEAD `a30313b`. `git status` shows no change under `crates`, `content`, `web`, `Cargo.toml` or `Cargo.lock`. This run's `cargo test -p engine --all-features` gave 276 passed, 0 failed and 6 ignored over 32 test binaries (exit 0).
- **The contest for a loose ball** (`sim.rs:963-1000`): the ball must be under `reach_height` (2.0 m). A fast ball (over `control_speed` 12 m/s) is open only to the two acting keepers (`self.keeper(team)`, from `defending-and-discipline`), within `keeper_reach` (2.6 m). The nearest keeper rolls `keeper_catch_chance` (0.86) once per flight (`keeper_beaten`). A pass `gain()`s the ball, and a miss plays on. No roll reads the shot, the goal frame or where the ball is going.
- **On target** (`sim.rs:1049-1054`, `rules/mod.rs:170-174`): counted when the opposing keeper gains a shot in flight, or when a shot scores. A keeper catch of a ball going wide counts.
- **The shot kick** (`decision.rs:528-559`): the aim is the far side of the goal from the keeper. The spread is `shot_noise × (1.5 − finishing) × spread_scale` (`shot_noise` 0.25 rad). The speed is `shot_speed` (27 m/s), and the loft is `range_f64(0.0, 2.5)`. A penalty in play and a shoot-out kick use the same function with `SHOOTOUT_SPREAD` 0.5 (`rules/mod.rs:597-606`, `:877-903`).
- **xG** (`sim.rs:440-451`): a logistic of distance and goal-mouth angle with `xg` coefficients −1.0, −0.1 and 1.0, summed at the kick (`sim.rs:895-899`). A penalty from 11 m gets about 0.19.
- **Out of play** (`rules/mod.rs:195-226`): a ball over the goal line gives a goal kick when the attacking side touched it last, otherwise a corner. `ball_out` is the only source of a corner.
- **Shoot-out** (`rules/mod.rs:906-943`, `:1019-1033`): the keeper dives from a draw (`KEEPER_DIVE_M` 2.0, `KEEPER_STAYS` 0.1) and holds a fast kick with `keeper_catch_chance × SHOOTOUT_HOLD` (0.43). A code comment records the three constants as a known limit.
- **Snapshot**: `Summary` is written into the snapshot (`snapshot.rs:766`, `:1035`). A snapshot is written only when play stops, and the flight fields (`keeper_beaten`, `shot_in_flight`) are cleared at every dead ball, so they are not in it.
- **Events**: 18 kinds (`sim.rs:285-305`), with no shot, save or block kind.
- **Measured on shipped play** (`implement-evidence/lone-forward/shipped/gate/equal-*/report.json`, 1,000 matches per seed, seeds 1, 7, 42, 99, 2026): shots on target 0.759–0.773 of shots; goals per xG 1.448–1.527; corners 0.004–0.011 per team; goal kicks 5.51–6.40 per match; shots 11.6–13.8 per team.
- **Red card on shipped play** (`06-verify-lone-forward.md`, `moved-criteria`): control home 1.0083; keeper arm full 1.6375 against reduced 2.6667; centre-back 0.8792 against 1.0792; striker 1.2083 against 1.7875; limit 1.6133. The slow test exits 101.

## Simplicity Ladder

- On-target from the trajectory → rung 3 reuse — `ball.rs` → `Ball::integrate()` on a copy of the ball, plus `pitch::in_goal()` and `crossbar_height`. The same physics that moves the ball also predicts it, so the prediction cannot disagree with the flight (drag and gravity included). A closed-form parabola would ignore drag.
- Save chance by shot quality → rung 4 new code — a clamped linear curve in `shot.rs`. Rungs 1–3 do not hold, because no curve exists in the repository. `shot_xg()` (rung 3, reused as is) supplies the quality from a fixed coefficient set.
- Held or parried, block, deflection → rung 3 reuse with modification — the fast-ball branch of `resolve_possession()` and the `keeper_beaten` once-per-flight pattern, extended to blockers. The deflected velocity is new code in `shot.rs` (rung 4): it is a few lines of vector arithmetic with the draws passed in.
- Scriptable rolls → rung 3 reuse — `EngineRng::referee_draw()` and `Scene::rolls()`, as tackles already use them.
- Tuning values with bounds and defaults → rung 3 reuse — a nested block with `garde(dive)` and `serde(default)`, as `decision` and `xg` are nested (`tuning.rs:182-190`).
- Penalty scenes → rung 3 reuse with modification — the `Scene` builder, with one `penalty()` seam in the `DeadBall` shape that `take_restart()` already reads.
- Measurement → rung 3 reuse — `engine-cli calibrate` (`--suite equal`, `--suite red-card`, `--baseline`) and the slow `a_sending_off_gives_no_advantage` test.
- xG refit → rung 4 new code — a three-parameter logistic fit by iteratively reweighted least squares, about 40 lines in an ignored test harness. No regression crate is in the workspace. A new dependency for one offline fit would cost more than the code.

## Applied Learnings

No applicable learnings found. `.ai/solutions/INDEX.md` does not exist.

Repeat-deferral tripwire: `00-index.md` holds two deferrals, the human legibility reading (viewer-match-day) and the macOS build (distribution). This slice is verified by headless engine runs with `cargo test` and `engine-cli calibrate`. It names neither wall, so the tripwire does not fire.

## Likely Files / Areas to Touch

- `crates/engine/src/tuning.rs`: the `ShotTuning` block (`shots`), its bounds, defaults and `Default`.
- `content/tuning.json`: the `shots` block; `shot_noise`, `keeper_reach` and the three `xg` coefficients as tuned.
- `crates/engine/src/shot.rs` (new): `on_target()`, `quality()`, `save_chance()`, `deflect()` and unit tests.
- `crates/engine/src/lib.rs`: the module line.
- `crates/engine/src/decision.rs`: the shot loft range in `shot_kick()`.
- `crates/engine/src/sim.rs`: on-target at the kick, the penalty xG, blocks, the save, the parry, the flight fields, and `last_touch()` under the scenario feature.
- `crates/engine/src/rules/mod.rs`: `goal()`, the penalty in `take_restart()`, the shoot-out save and dive, and the three constants removed.
- `crates/engine/src/scenario.rs`: `Scene::penalty()`.
- `crates/engine/tests/keeper_and_shots.rs` (new): the outcome scenes, the penalty scenes, and the slow set-piece, census and refit tests.
- `crates/engine/tests/content.rs`, `acting_keeper.rs`, `match_stats.rs`, `rules_shootout.rs`: pins and changed expectations.
- `docs/reference/data-files.md`: the `shots` rows and a paragraph.
- Pinned tests that may move and are re-derived: `rules_extra_time.rs`, `full_match.rs`, `determinism.rs`, `strength.rs`, `commentary.rs`, the `sim.rs` unit tests, `crates/engine-cli/tests/stream_cli.rs`.

## Proposed Change Strategy

- **The trajectory decides on target.** At the kick, `shot::on_target()` integrates a copy of the ball until it reaches the goal line, stops, or leaves play. The shot is on target when it crosses the line between the posts under the bar. The count is added at the kick (the slice's words), and a flag stays with the flight. After a block or a parry the flag is recomputed for the new flight, but nothing is counted again.
- **A save only for a shot on target.** On an on-target flight, when the ball is within `keeper_reach` of the acting keeper and under the bar, the keeper takes one save roll per flight: `save_chance(quality)`. The curve is linear from `save_high` (0.891) at quality 0.05 to `save_low` (0.272) at quality 0.40, and flat outside that range (`01-engine-realism.md:396`). The quality is `shot_xg()` with a fixed coefficient block (`shots.quality`, today's −1.0, −0.1 and 1.0), so the later xG refit leaves the saves alone. A failed roll plays on toward goal. A successful roll is held with `save_hold` (starting at 1/3, from "two in three not held", `:397`). Otherwise it is parried.
- **A wide shot is left alone.** While a shot is in flight and off target, the keeper never takes it while it is fast. The ball runs out, the attacking side is the last touch, and the restart is a goal kick. Fast balls that are not shots keep today's `keeper_catch_chance` rule.
- **Parries and blocks are deflections.** A parry sends the ball at `parry_speed` of its speed. The direction is drawn within `parry_spread` of the goal line, away from the goal centre, with a loft up to `parry_loft`. A block is one roll per defender per flight (`block_chance`) for an opposing outfield player within `block_reach` of a ball under `reach_height`. It deflects with `block_speed` and `block_spread` around the reversed direction. Both set the defending side as the last touch and end the shot in flight. The existing `ball_out` then gives a corner only when the ball crosses the line (RIM-9).
- **Shots can rise.** The loft is drawn from 0 to `shots.loft_max` (vertical m/s). `shot_noise` stays the aim spread and is tuned so that wide shots happen.
- **Penalties use the same model.** A penalty in play is marked in `take_restart()`. Its xG is `penalty_xg` (0.76, the conventional value in public xG models), and its save roll reads the same curve at that quality. The shoot-out save uses the same functions, so one keeper model covers open play, penalties and the shoot-out. `SHOOTOUT_SPREAD`, `KEEPER_DIVE_M` and `KEEPER_STAYS` become `shots.penalty_spread`, `shots.keeper_dive_m` and `shots.keeper_stays` at their current values. `SHOOTOUT_HOLD` is removed, because `save_hold` replaces it.
- **Rolls are scriptable.** The block, save and hold rolls use `referee_draw()`, so `Scene::rolls()` can force each outcome. The deflection directions use the main stream.
- **No contract moves.** No event kind, `Summary` field, snapshot field, report key or protocol message is added. The flight fields are cleared at every dead ball, as `keeper_beaten` is. `TUNING_VERSION` stays 2, because the block is optional with defaults.
- **Measure with the tuning loop.** The inner loop is `calibrate --suite equal --seed 42 --matches 200 --baseline …` plus the outcome census test. The criterion runs are the slow tests and `calibrate --suite equal --seed 42 --matches 1000`. The slice gate is the full calibrate suites: five seeds for equal and strength, and one seed for formations (steer.md, Q-I1).
- **No NFR is the rationale for a mechanism choice.** The benchmark gate (+10% per tick) is the only performance line. The 2000 ms per-match budget (NFR-1, `yields-to: C2`) is not in tension.

## Step-by-Step Plan

1. **Baselines.** Build release at `a30313b` before any code change. Run these and copy the output into `implement-evidence/keeper-and-shots/baseline/`:
   - `engine-cli calibrate --suite equal --seed 42 --matches 1000 --out …/equal-base`;
   - `engine-cli calibrate --suite equal --seed 42 --matches 200 --out …/set-base`;
   - `engine-cli calibrate --suite red-card --seed 1 --matches 240 --out …/red-base`.
   Record the command, commit and `fixtures.hash` of each (Q-X6).
2. **The tuning block.** Add `ShotTuning` as `Tuning::shots` with `#[serde(default)]` and `#[garde(dive)]`. It holds `loft_max` (0 to 15), `save_high` and `save_low` (0 to 1), `quality` (an `XgTuning`), `save_hold` (0 to 1), `parry_speed` (0 to 1), `parry_spread` (0 to 3.2), `parry_loft` (0 to 15), `block_reach` (0 to 3), `block_chance` (0 to 1), `block_speed` (0 to 1), `block_spread` (0 to 3.2), `penalty_xg` (0 to 1), `penalty_spread` (0 to 2), `keeper_dive_m` (0 to 3.66) and `keeper_stays` (0 to 1). Starting values: 7.0, 0.891, 0.272, {−1.0, −0.1, 1.0}, 0.333, 0.4, 1.2, 4.0, 1.0, 0.5, 0.5, 1.0, 0.76, 0.5, 2.0, 0.1. Mirror them in `Default`. Write them into `content/tuning.json`. In `tests/content.rs`, pin the values, and add a test that a tuning block without `shots` loads with the defaults. `TUNING_VERSION` stays 2.
3. **Shot functions.** In a new `crates/engine/src/shot.rs`, add:
   - `on_target(ball: Ball, attack_x: f64, t: &Tuning) -> bool`. It steps a copy with `Ball::integrate` for at most 250 steps, and stops at the goal line of `attack_x`, at rest, or out of play.
   - `quality(from, attack_x, t)`, which is `shot_xg(from, attack_x, &t.shots.quality)`.
   - `save_chance(quality, t)`.
   - `deflect(vel, away: DVec2, speed_share, spread, loft, draw_angle, draw_loft) -> DVec3`.
   Unit tests: a shot at the centre from 16 m is on target; aimed 1 m outside a post it is not; at loft 9 m/s from 18 m it passes over the bar; `save_chance` is 0.891 at 0.02, 0.272 at 0.6, and falls monotonically between; the length of a deflection is `speed_share × |vel|`. No draw is taken inside these functions.
4. **The shot rises.** In `shot_kick()`, draw the loft from `0.0..shots.loft_max`, and read the spread scale for a penalty from `shots.penalty_spread`. The draw order is unchanged.
5. **On target at the kick.** In `apply_kick()`, after `self.ball.kick(...)`, compute `on_target` for a shot. Count `shots_on_target` then, and keep a flight flag. Sum `shots.penalty_xg` in place of `shot_xg()` when the kick is a penalty, which `take_restart()` marks. Remove the count from `goal()` and from `gain()`. Clear every new flight field wherever `keeper_beaten` and `shot_in_flight` are cleared.
6. **Blocks.** In the fast-ball branch of `resolve_possession()`, while a shot is in flight, check each active opposing outfield player within `block_reach` of the ball under `reach_height`. A player not yet tried this flight (a 22-bit mask, cleared with the flight) rolls `referee_draw() < block_chance`. On a block, deflect the ball, set the last touch and `last_kicker` to the blocker's side and player, end the shot in flight, and recompute the flag. No count is added.
7. **The save.** For an on-target shot in flight, the acting keeper within `keeper_reach` of a ball under `crossbar_height` rolls once per flight: `referee_draw() < save_chance(quality)`. The quality is taken at the kick and stored with the flight. On a failed roll, the ball plays on (`keeper_beaten`). On a save, a second roll `referee_draw() < save_hold` gives `gain(keeper)`. Otherwise the ball is parried: it is deflected away from the goal centre, the keeper's side is the last touch, and the shot in flight ends. An off-target shot skips the keeper while it is fast. A fast ball that is not a shot keeps the `keeper_catch_chance` rule.
8. **Penalties and the shoot-out.** In `take_restart()`, mark the penalty kick. In `take_shootout_kick()`, use `shots.keeper_dive_m` and `shots.keeper_stays`. In `shootout_save()`, attempt a save only for an on-target kick, with `save_chance(shots.penalty_xg)` and the same hold roll. A held kick is a miss, as a keeper's gain is today. A parried kick plays on until it stops or leaves play, like a keeper touch without a catch today, so a ball that goes in off the keeper still scores (`04-plan-extra-time-penalties.md` A-18). Remove `SHOOTOUT_SPREAD`, `SHOOTOUT_HOLD`, `KEEPER_DIVE_M`, `KEEPER_STAYS` and their known-limit comment.
9. **Seams.** Add `Scene::penalty(team, taker)`, which opens a ready penalty dead ball at the team's penalty mark with the taker on it. Add `Simulation::last_touch()`. Both are under the `scenario` feature.
10. **Outcome scenes (fast, not ignored)** in `tests/keeper_and_shots.rs`, built like `match_stats.rs` (a calm match, everyone else spread away):
    - **Wide:** a shot from (30, 0) aimed 2 m outside a post. It is not on target, the referee draw scripted as `[0.0]` is still unused (no save roll), and the stoppage is a goal kick to the defending side.
    - **Over:** the same shot at the centre with loft 9 m/s. It is not on target, and the restart is a goal kick.
    - **Blocked:** a defender 4 m ahead in the lane, with rolls `[0.0]`. The defending side is the last touch at once, and the shot is no longer in flight.
    - **Held:** an on-target shot at the keeper, with rolls `[0.0, 0.0]`. The keeper is the carrier, and on target is 1.
    - **Parried to a corner:** on seeds 1–40, rolls `[0.0, 0.99]`. After each parry the keeper's side is the last touch. At least one seed gives a corner, and every corner comes after the ball crosses the goal line.
    - **Scored:** rolls `[0.99]`. It is a goal, and on target is 1.
    - **Penalties:** 500 scenes (seeds 1–500, `Scene::penalty`), each stepped until a goal, a stoppage or 250 ticks. The conversion is printed and must be 0.70–0.85. If a debug run takes over 10 s, it becomes a slow ignored test.
11. **Slow tests (ignored, release)** in the same file:
    - `set_pieces_arise_from_play`: 200 matches of the default clubs on seeds 1–200, stepped tick by tick. On each corner stoppage, the ball's previous record is at or beyond the goal line, and `last_touch()` before that tick was the defending side. It asserts at least 3.0 corners per team and at least 10 goal kicks per match, and prints both.
    - `shot_outcome_census`: the same matches classify each shot as wide or over, blocked, held, parried or scored, and print the shares. This is a diagnostic for the tuning loop, with no assertion beyond counts that sum to the shots.
    - `xg_refit`: 1,000 matches on seeds 1–1000. It records each open-play shot's distance, angle and goal, fits the three coefficients by iteratively reweighted least squares, and prints them. Penalties are left out.
12. **Inner tuning loop.** Tune only `loft_max`, `shot_noise`, `keeper_reach`, `save_hold`, the parry values and the block values, each inside its bounds. `save_high`, `save_low`, `quality` and `penalty_xg` stay at their sourced values. After each change, run `calibrate --suite equal --seed 42 --matches 200 --baseline …/set-base/report.json` and the census. Record each setting, its diff and its verdict in a table in the implement record. A change the diff marks as noise is no change.
13. **xG refit.** Run `xg_refit`, write the three coefficients into `xg`, and confirm with `calibrate --suite equal --seed 42 --matches 1000 --baseline …/equal-base/report.json` that goals per xG is 0.85–1.15. If one refit lands outside, refit once more on the new play. If it still misses, stop and report.
14. **Criterion runs (release).**
    - `cargo test --release -p engine --all-features --test keeper_and_shots -- --include-ignored --nocapture`.
    - `calibrate --suite equal --seed 42 --matches 1000` (shots on target 0.30–0.42 of shots; goals per xG 0.85–1.15).
    - `cargo test --release -p engine --all-features --test defending -- --include-ignored --nocapture a_sending_off_gives_no_advantage`.
    If the red-card test fails, stop and report each arm (full, reduced, the limit) and the shots per team. Do not raise a limit, change a card chance, or add a lever outside this slice (steer.md, Q-LF2). Also run `every_formation_holds` and `discipline_is_realistic`, and record them. A pairing that passed before and fails now is reported as a regression.
15. **Close.**
    1. Re-derive each moved expectation (step 12 of the `lone-forward` plan names the method). Record the value before and after, and the reason. Never widen a tolerance.
    2. Add the `shots` rows and the paragraph to `docs/reference/data-files.md`, and note that `keeper_catch_chance` now covers only fast balls that are not shots.
    3. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo test --workspace --all-features`.
    4. Run the benchmark compare from `05c-benchmark.md` (gate 1.7743 µs per tick and 8.39 MB).
    5. Run the slice gate: `calibrate --suite equal` and `--suite strength` on seeds 1, 7, 42, 99 and 2026, and `--suite formations --seed 42`, each at 1,000 matches. Record band misses against `…/equal-base`. They are recorded, not failed (Q-E4).
    6. Search the changed source, comments and commit text for workflow vocabulary before the commit.

## Verification Strategy

The five criteria are about match outcomes and scripted engine scenes. The engine produces them headless, so no screen is needed. The rung for each is headless runs of the real engine library or the real release binary (`cargo test` in `stack.testing`, and `engine-cli calibrate`). A unit test or static reasoning alone never counts.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| Set pieces arise naturally | `cargo test --release -p engine --all-features --test keeper_and_shots -- --include-ignored --nocapture set_pieces_arise_from_play` (headless engine runs, 200 matches) | Rust 1.92 and cargo on the reference machine — yes | `set_pieces_arise_from_play`; `Simulation::last_touch()` (scenario feature) | `engine-cli calibrate --suite equal --seed 42 --matches 200` (`corners_per_team_mean`, `goal_kicks_per_match_mean`) for the rates, and the parry scene for the origin → pre-registered deferral (not expected: no wall) |
| Shots are counted honestly | `engine-cli calibrate --suite equal --seed 42 --matches 1000` (release binary, `shots_on_target_share`, `goals_per_xg`) | yes | Nothing new in the CLI: the report keys exist | the same figures on seeds 1, 7, 99 and 2026 from the slice gate → `shot_outcome_census` → pre-registered deferral |
| A wide shot stays wide | `cargo test -p engine --all-features --test keeper_and_shots` (the wide and over scenes on the real engine) | yes | the wide and over scenes; scripted draws through `Scene::rolls` | a traced match (`engine-cli simulate` events) showing a goal-kick restart after a shot → pre-registered deferral |
| Penalties convert realistically | the 500 penalty scenes in `tests/keeper_and_shots.rs` | yes | `Scene::penalty()` | `engine-cli simulate` event files over many seeds, counting a goal after each penalty restart → pre-registered deferral |
| No advantage from a red card (moved here, steer.md Q-LF2) | `cargo test --release -p engine --all-features --test defending -- --include-ignored --nocapture a_sending_off_gives_no_advantage` (240 matches per arm, balanced fixtures) | yes | Nothing new: the slow test exists | `engine-cli calibrate --suite red-card --seed 1 --matches 240` (same seeds, orders and arms) → pre-registered deferral |

No criterion depends on credentials, a device, an external service or missing infrastructure. Every run is local and uses tools already in `stack:`, so no `constraint-resolution:` line is needed. No wall is named.

## Test / Verification Plan

### Automated checks

- Lint and type check: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Unit: `shot.rs` (`on_target`, `save_chance`, `deflect`).
- Integration (fast): `tests/keeper_and_shots.rs` (the six outcome scenes and the penalty scenes), `tests/content.rs`, `acting_keeper.rs`, `match_stats.rs`, `rules_shootout.rs`, `rules_extra_time.rs`. The full-match validator must still read 0 violations (`tests/validator.rs`).
- Slow (ignored, release): `set_pieces_arise_from_play`, `shot_outcome_census`, `xg_refit`, `a_sending_off_gives_no_advantage`, `every_formation_holds`, `discipline_is_realistic`, `strength`.
- Calibrate: the equal suite against the step-1 baselines (inner loop and criterion), and the full slice gate (step 15).
- Benchmark compare against `05c-benchmark.md`.

### Interactive verification (human-in-the-loop)

Automated only. Every criterion is a statistic of engine runs or a scripted engine scene. The page shows no new element. A parried or blocked ball is drawn where the engine puts it, and corners and goal kicks use the existing restart records.

## Risks / Watchouts

- **The red-card criterion may still fail** (high). On shipped play the keeper arm is 2.67 against 1.64, and the full side is above the limit (1.64 against 1.61). The measured cause is that a side with ten men attacks as if it had eleven. Saves and blocks touch it only through how shots convert. Step 14 stops and reports, and it never raises a limit (steer.md, Q-LF2).
- **Corners may stay under 3.0 per team** (high). Today there are 0.005. Each corner needs a parry, a block or a clearance to physically cross the goal line. The census shows which outcome is short, and the parry and block values are tuned. A corner is never scripted (RIM-9).
- **Goals per xG and the save curve interact** (medium). The fixed quality model breaks the loop. The refit is last, and it is tried twice at most.
- **Penalties outside 70–85%** (medium). Only `penalty_spread`, the dive values and the shared keeper values reach the penalty. `penalty_xg` stays at 0.76. If no in-bounds setting converts in range, implement stops and reports.
- **Every seeded result moves** (medium). The rolls change the random stream. Each moved expectation is re-derived and recorded.
- **Goals leave the band** (low). This is accepted by the product owner until `realism-tuning`. Band misses are recorded, not failed (Q-E4).
- **Per-tick cost** (low). The trajectory check runs once per shot, at no more than 250 steps. The block check is one pass over the players, and only while a shot is in flight. The gate is 1.7743 µs per tick.

## Dependencies on Other Slices

- `defending-and-discipline` (complete, verified): the acting keeper (`Simulation::keeper(team)`). Saves use it, never slot 0.
- `realism-bands-v2` (complete, verified): the shots-on-target, goals-per-xG, corners and goal-kicks bands, read in the gate run.
- `tuning-loop` (complete, verified): `--baseline`, the noise mark, and the `red-card` suite.
- `lone-forward` (complete, verified): the balanced red-card fixtures (`7bd5fa5`) and the shipped play this slice starts from. It moved the red-card criterion here.
- `tempo-and-restarts` (next): it owns restart timing and pass tempo. It plans after this slice.
- `realism-tuning` (last): it owns final goals-band tuning.

## Assumptions

Autonomous run: no product owner was present. Each discovery question the interview would have asked is answered here in the direction that meets the criteria at the least cost. None changes the scope, a public contract, a persisted data shape or a product owner answer.

- A1 (class: implementation-detail): on target is decided from the trajectory at the kick by stepping a copy of the ball with the engine's own physics. It is counted at the kick, as the slice's scope says. A later block does not remove the count, and a deflection does not add one.
- A2 (class: implementation-detail): the save curve is linear in shot quality between the two sourced points (0.891 at 0.05 and 0.272 at 0.40) and flat outside them. The source gives only the end bins, so a straight line is the simplest reading.
- A3 (class: implementation-detail): the save curve reads a fixed quality model with today's xG coefficients (`shots.quality`), not the reported xG. The xG refit then changes the reported xG and never the saves, so the refit cannot chase itself.
- A4 (class: implementation-detail): a save is held with `save_hold`, starting at 1/3 ("two in three not held", `01-engine-realism.md:397`). Otherwise it is parried.
- A5 (class: implementation-detail): a parry and a block are deflections of the ball's velocity, with the defending side as the last touch. Corners come only through the existing `ball_out`, so no corner is created without a crossing (RIM-9).
- A6 (class: implementation-detail): a block is one roll per defender per flight, for an opposing outfield player within `block_reach` of a ball under `reach_height`. This mirrors the once-per-flight keeper roll (`keeper_beaten`).
- A7 (class: implementation-detail): the block, save and hold rolls use `referee_draw()`, so tests can script each outcome with `Scene::rolls()`, as the tackle tests do. The deflection angles use the main stream.
- A8 (class: implementation-detail): `keeper_catch_chance` keeps its rule for fast balls that are not shots (passes and clearances). The slice changes only how shots are saved, and the older files keep their meaning.
- A9 (class: implementation-detail): a penalty's xG is `penalty_xg`, 0.76. The geometric model gives about 0.19 from the mark, which would inflate goals per xG. 0.76 is the conventional value in public xG models, and it stays fixed during tuning.
- A10 (class: implementation-detail): penalties in play and the shoot-out use the same save model at `penalty_xg`. The three hand-set shoot-out constants move into `shots` at their current values, and `SHOOTOUT_HOLD` is removed. A held shoot-out kick ends as a miss, and a parried one plays on (`04-plan-extra-time-penalties.md` A-18). This is "the shoot-out constants stay consistent with the new keeper model", and it closes the known-limit comment.
- A11 (class: implementation-detail): no event kind, `Summary` field, snapshot field, report key or protocol message is added. `Summary` is in the snapshot (`snapshot.rs:766`), so a new counter would be a persisted-shape change, and the slice does not need one. The outcome census reads outcomes in a test through `last_touch()`, `carrier()` and the stoppages.
- A12 (class: implementation-detail): `Simulation::last_touch()` and `Scene::penalty()` are added under the `scenario` feature only, so the default public surface of the engine does not change.
- A13 (class: implementation-detail): the tuning block is optional with defaults equal to the starting values. `TUNING_VERSION` stays 2, as for the defending and lone-forward values.
- A14 (class: implementation-detail): the tuning loop moves only the shot, block, parry and hold values, `shot_noise` and `keeper_reach`. The sourced save curve, the quality model and `penalty_xg` stay fixed. No band or limit moves.
- A15 (class: implementation-detail): the xG refit fits all three coefficients (intercept, distance, angle) by iteratively reweighted least squares on open-play shots from 1,000 matches. It is an ignored test harness, so no dependency is added and nothing runs in the default suite.
- A16 (class: implementation-detail; ac: "Set pieces arise naturally"; classification: runtime-evidence): 200 matches of the default clubs, seeds 1–200. It checks the rates and the origin of every corner: the previous record is at or beyond the goal line, and the defending side was the last touch.
- A17 (class: implementation-detail; ac: "Shots are counted honestly"; classification: runtime-evidence): `calibrate --suite equal --seed 42 --matches 1000`, whose report already carries `shots_on_target_share` and `goals_per_xg`.
- A18 (class: implementation-detail; ac: "A wide shot stays wide"; classification: build-capability): the wide scene and the over-the-bar scene on the real engine. The unused scripted draw proves that no save roll was taken.
- A19 (class: implementation-detail; ac: "Penalties convert realistically"; classification: runtime-evidence): 500 scenes built with `Scene::penalty()` on seeds 1–500, with the default clubs' takers and keepers, each stepped until the kick resolves.
- A20 (class: implementation-detail; ac: "No advantage from a red card"; classification: runtime-evidence): the criterion test is `a_sending_off_gives_no_advantage` unchanged. It runs on the balanced fixtures of Q-LF1 with every limit unchanged (steer.md, Q-LF2). If it fails, implement stops with the figures.
- A21 (class: implementation-detail): augmentations. The benchmark is re-baselined on `a30313b` (`05c-benchmark.md` rev 8, 1.613 µs per tick; gate 1.7743 µs and 8.39 MB), because the slice changes per-tick code while a shot is in flight. `04b-instrument.md` is not re-authored, because no dark path, event or counter is added. `04c-experiment.md` is not involved, because no flag is added.
- A22 (class: implementation-detail): the second-opinion consult is not fired, although `appetite-medium-or-larger` holds. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`), as in earlier plans.
- A23 (class: implementation-detail): the design gate is settled (`02c-craft.md` `direction-confirmed-by: in-session`; `po-answers.md` 2026-09-24T11:22:17Z). This slice changes no page.
- A24 (class: implementation-detail): baselines are copied into the slice's evidence folder, and nothing new is committed for them (Q-X6). The slice gate runs the full suites, as steer.md requires (Q-I1).
- A25 (class: implementation-detail): in `00-index.md` this run marks the slice `in-progress`, points the benchmark entry at this slice and adds the plan files. It leaves `current-stage`, `selected-slice` and the next command as they are, because they still name the pending review of `lone-forward`, which another run owns.

## Blockers

None.

## Freshness Research

- No dependency is added or upgraded. The slice uses std `f64` arithmetic and the crates already in the workspace (`garde` for bounds, `serde` for defaults). `04-plan-data-schemas-generator.md` § Freshness Research records the `serde` 1.0 and `garde` 0.23 behaviour. A nested block with `garde(dive)` inside a `deny_unknown_fields` struct is already proven by `decision` and `xg` (`tuning.rs:182-190`). A field-level `serde(default)` is proven by `a_tuning_block_without_the_defending_values_loads_with_the_defaults` (`crates/engine/tests/content.rs`).
- The save curve, the held share and the block shares are the sourced figures in `docs/design/realism/01-engine-realism.md:393-397` (StatsBomb 360 and event data: a save rate of 89.1% for xG under 0.05 down to 27.2% over 0.40; two saves in three not held; 20.9% of shots blocked with no defender in the cone and 40.4% with two).
- The laws are not re-read here. The trajectory check uses the same `pitch::in_goal` and `crossbar_height` test that already decides a goal (`sim.rs:936-938`), so on target and a goal cannot disagree. The shoot-out rules (Law 10) were recalled, not re-read, in `04-plan-extra-time-penalties.md` (R7). This slice keeps that plan's kick-end rule (A-18).
- The bands the gate reads (`content/realism-bands.json`: shots on target 0.30–0.42, goals per xG 0.85–1.15, corners 3.5–6.5 per team, goal kicks 12–22 per match) are the product owner's sourced values (Q-E1). This slice's own limits (3.0 corners and 10 goal kicks) are below those bands, and `realism-tuning` closes the gap.

## Recommended Next Stage

- **Option A (default): Implement** → `/wf implement football-manager-match-engine keeper-and-shots`. The plan has no blocker, and the benchmark is re-baselined. Compact the session first. Workflow state lives in the artifact files, and the SessionStart hook re-reads it after compaction.
- **Option C: Revisit slice** → `/wf slice football-manager-match-engine`. Take this only if implement shows that the save and block model cannot pass a criterion at any in-bounds setting (step 14 stop).
