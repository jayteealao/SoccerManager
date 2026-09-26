---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: tempo-and-restarts
status: complete
stage-number: 4
created-at: "2026-09-25T03:12:00Z"
updated-at: "2026-09-25T03:12:00Z"
metric-files-to-touch: 19
metric-step-count: 15
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, passing, restarts, realism]
stack-source: confirmed
open-questions: []
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-tempo-and-restarts.md
  siblings: [04-plan-keeper-and-shots.md, 04-plan-lone-forward.md, 04-plan-tuning-loop.md, 04-plan-realism-bands-v2.md, 04-plan-defending-and-discipline.md]
  benchmark: 05c-benchmark.md
  implement: 05-implement-tempo-and-restarts.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine tempo-and-restarts"
---

# Plan: Tempo and Restarts

## The Plan

`keeper-and-shots` shipped at `8fd407b` and `bce0144` and passed verify on its four criteria. On that play, a 24-match probe on seed 42 in this run measured 1,216 passes per team at 86.6% accuracy, 20.4 throw-ins per match, and the ball live for 90.0 of 94.1 minutes. The code shows why. A carrier who is not pressed passes about 0.2 s after he gains the ball, because the dribble bonus lasts 10 ticks and holding loses 0.5 per second (`decision.rs:343-363`). Throw-ins wait 3 s and free kicks 8 s (`tuning.rs:445-453`). Every `Kick::Pass` adds to `passes`, so clearances and restart kicks count (`sim.rs:973`). Throw-ins are also out of band now. The slice expected 33–43, but the keeper-and-shots gate measured 22.4–24.1.

The plan keeps "every agent decides every tick". It adds a carry window to what the carrier prefers. For `carry_s` seconds after he gains the ball, a carrier with no opponent within 2.5 m pays `carry_cost` on a pass and on a clearance. A shot is exempt. The four restart delays move to the sourced medians: 13.8, 23.2, 31.8 and 32.5 s. A clearance becomes its own kick (`Kick::Clear`), and the first kick of a restart taker is a restart kick. Each is counted under its own key, and `stats.passes` keeps only open-play passes. A new `stats.ball_in_play_s` goes in `match-stats`, and the calibrate report shows the minutes per 90. Throw-ins and corners get physical levers only. The clearance aim spread becomes a tuning value. A defender can also clear a fast pass inside his own penalty area, once per flight. That clearance gives the corners from clearances and crosses that the product owner moved here (Q-KS2). A corner or a throw-in still comes only from a ball that crosses a line.

The work is 15 steps over 19 files, 1 of them new. The benchmark is re-baselined on `bce0144` at 1.5519 µs per tick (gate 1.7071 µs, 8.55 MB). When it lands, `realism-tuning` can tune goals on a match with a real rhythm. The top risk is the ball-in-play figure. The engine awards fewer stoppages than real matches (12.8 fouls and 20.4 throw-ins per match), so the sourced medians alone may leave the figure near 70 minutes. If it stays above 65 with every lever in bounds, implement stops and reports. It never sets a delay above a median.

## Current State

- Code: HEAD `bce0144`. `git status` shows no change under `crates`, `content`, `web`, `Cargo.toml` or `Cargo.lock`. `06-verify-keeper-and-shots.md` (status complete, result pass, at `bce0144`) records the workspace suite at 501 passed on the server, with the two Linux-only failures passed on this PC.
- **The carrier's decision** (`decision.rs:226-385`, `decide_carrier` `:387-460`): every tick the carrier scores a shot, the best pass, a dribble, a hold and a clearance, each with fresh noise, and takes the highest. The dribble has `first_touch` (0.6) only while `held < 10` ticks. The hold is `hold` (−0.4) minus `hold_per_s` (0.5) per second held. `pressed` is an opponent within 2.5 m (`:261`). `decision_interval_ticks` is 1 (`content/tuning.json:5`).
- **Clearances** (`decision.rs:433-441`): `Choice::Clear` returns a `Kick::Pass` toward the far half, turned by up to ±0.6 rad, over 55 m with loft 6 m/s (`CLEARANCE_DISTANCE`, `CLEARANCE_LOFT`, `:685-687`). The keeper clears with `keeper_clear`.
- **Pass counting** (`sim.rs:939-990`): `kick_ball` adds every `Kick::Pass` to `summary.passes` and sets `pass_in_flight`. `gain()` counts a completion when the same side gains the ball next (`:1269-1271`). `Kick` has two variants, `Pass` and `Shot` (`decision.rs:20-23`). They are matched in `sim.rs:949`, `rules/mod.rs:901` and `scenario.rs:128,143`.
- **Restarts** (`rules/mod.rs:580-620`, `rules/restart.rs:49-70`): a throw-in, a corner and an indirect free kick are kicked with `restart_pass()`. A goal kick uses `decide_carrier`. A kick-off and a direct free kick leave the taker holding the ball, and he decides on the next tick. The restart may be taken once `delay_ticks(kind)` has passed, the taker is at the ball and every player the law moves has moved. The hard limit is three times the delay. Delays today: kick-off 5, throw-in 3, corner 8, goal kick 6, free kick 8, penalty 15, drop ball 20 s (`content/tuning.json:54-62`).
- **Added time** (`rules/clock.rs:44-51`, `content/rules/default.json:17-33`): priced per stoppage kind and card, with ±30 s variance and a clamp of 60–900 s. It does not read the restart delays, so longer delays do not change added time.
- **The fast-ball contest** (`sim.rs:1008-1075`, `:1179-1215`): only the acting keepers can take a ball faster than `control_speed` (12 m/s). While a shot is in flight, `try_block` gives each outfield defender within `block_reach` one roll per flight, and `shot::deflect` turns the ball (`shot.rs:54-72`). Nothing contests a fast pass except the keeper.
- **Counters and records**: `Summary` (`sim.rs:385-440`) is written into the snapshot (`snapshot.rs:740-782`, VERSION 5 at `:46`). The reader refuses another version or another build (`:156-160`). `MatchFigures` (`observe/mod.rs:141-208`) carries `stats.passes`, `passes.completed` and `stats.pass_accuracy_pct`. `LawStats` carries `ticks.played` and `rules.dead_ball_ticks` (`:263-300`). Both schema files allow extra keys (`match-stats.schema.json:352`, and the run-report suite object lists properties without `additionalProperties: false`). The protocol's `stats.passes` is described as "passes played" (`docs/reference/protocol.md:257`).
- **The calibrate report** (`engine-cli/src/report/mod.rs:131-170`): the suite figures include `passes_per_team_mean`, `pass_accuracy_pct_mean` and `throw_ins_per_match_mean`. They have no ball-in-play figure. The bands (`content/realism-bands.json`) include passes 350–550, accuracy 75–88 and throw-ins 35–55. They have no ball-in-play band.
- **Measured on shipped play** (this run, `engine-cli calibrate --suite equal --seed 42 --matches 24`, stats records in the session scratch folder): passes 1,216.5 per team; accuracy 86.6%; throw-ins 20.4, goal kicks 12.3, corners 2.6 and fouls 12.8 per match; `ticks.played` 282,325 and `rules.dead_ball_ticks` 12,458 per match, which is 90.0 live minutes of 94.1; added time 246.5 s per match. The keeper-and-shots gate (`verify-evidence/keeper-and-shots/equal-table.txt`, 1,000 matches on five seeds) gives passes 1,219.8–1,246.8 per team and throw-ins 22.4–24.1 per match.
- **Real figures** (`docs/design/realism/01-engine-realism.md`): median stoppage before a restart is 13.8 s for a throw-in, 23.2 s for a goal kick, 31.8 s for a corner and 32.5 s for a free kick (`:614`). Fouls are 26.2 a match (`:702`). The ball is in play for 55.4% of a match (`:693`).

## Simplicity Ladder

- Carry window → rung 3 reuse with modification — `decision.rs` → `options()` already reads `held` and `pressed`. Two tuning weights and one subtraction are added. No draw is added and no loop is added. A per-possession stored decision (the other option in the slice scope) would need a new field in the snapshot and a new invalidation rule, so the soft window is the smaller change.
- Restart delays → rung 3 reuse — `RestartDelays` and `delay_ticks()` exist. Only the content values change.
- Clearance and restart-kick counting → rung 3 reuse with modification — `kick_ball()` already branches on the kick kind. A `Kick::Clear` variant lets the compiler find every match site. The restart taker mark follows the `pass_in_flight` pattern.
- Ball-in-play figure → rung 3 reuse — a `live_ticks` counter at the `Phase::Live` arm, where `possession_ticks` is counted (`sim.rs:838-846`). It could be derived from `ticks.played − rules.dead_ball_ticks`, but half-time and the shoot-out make that derivation fragile.
- Cross clearance → rung 3 reuse with modification — `try_block()` and `shot::deflect()` from `keeper-and-shots`, with the same once-per-flight mask pattern and `deflected_by()` for the last touch.
- Clearance aim spread → rung 3 reuse — the hard-set 0.6 rad becomes `clearances.aim_spread`, a tuning value with bounds.
- Tuning values with bounds and defaults → rung 3 reuse — `serde(default)` fields and a nested `garde(dive)` block, as `lone_*` and `shots` are (`tuning.rs:191-194`, `:356-368`).
- Measurement → rung 3 reuse — `engine-cli calibrate --suite equal --baseline` and `common::run_many` for the slow tests.
- Report figure → rung 3 reuse — one field in `SuiteFigures` beside `throw_ins_per_match_mean`.

## Applied Learnings

No applicable learnings found. `.ai/solutions/INDEX.md` does not exist.

Repeat-deferral tripwire: `00-index.md` holds two deferrals, the human legibility reading (viewer-match-day) and the macOS build (distribution). This slice is verified by headless engine runs with `cargo test` and `engine-cli calibrate`. It names neither wall, so the tripwire does not fire.

## Likely Files / Areas to Touch

- `crates/engine/src/tuning.rs`: `carry_s` and `carry_cost` in `DecisionWeights`; the `ClearanceTuning` block (`clearances`); the `RestartDelays` default.
- `content/tuning.json`: the four delays, the carry values and the `clearances` block.
- `crates/engine/src/decision.rs`: `Kick::Clear`, the carry window in `options()`, and the aim spread in `decide_carrier`.
- `crates/engine/src/sim.rs`: the three counters, the counting in `kick_ball()`, `live_ticks`, and `try_clear_cross()` in the fast-ball contest.
- `crates/engine/src/rules/mod.rs`: the restart taker mark in `take_restart()` and clearing it at every dead ball; the `Kick` match at `:901`.
- `crates/engine/src/snapshot.rs`: VERSION 6 and the three counters.
- `crates/engine/src/observe/mod.rs`: `stats.clearances`, `stats.restart_kicks` and `stats.ball_in_play_s`.
- `crates/engine-cli/src/report/mod.rs`: `ball_in_play_min_per_90_mean`.
- `schemas/observability/match-stats.schema.json`, `schemas/observability/run-report.schema.json`: the optional properties.
- `crates/engine/src/scenario.rs`: `Scene::clear()`.
- `crates/engine/tests/tempo_and_restarts.rs` (new): the fast scenes and the slow criterion tests.
- `crates/engine/tests/content.rs`, `match_stats.rs`, `snapshot.rs`, `plugin_hooks.rs`: pins and changed expectations.
- `docs/reference/data-files.md`, `docs/reference/protocol.md`, `.ai/observability.md`: the rows, the `stats.passes` wording and the key list.
- Pinned tests that may move and are re-derived: `full_match.rs`, `determinism.rs`, `rules_restarts.rs`, `rules_extra_time.rs`, `commentary.rs`, `lone_forward.rs`, `strength.rs`, the `sim.rs` unit tests, `crates/script/tests/decision.rs`, `crates/engine-cli/tests/stream_cli.rs`, `crates/stream/tests`.

## Proposed Change Strategy

- **A carry window, not a slower clock.** For `decision.carry_s` seconds after the carrier gains the ball, when no opponent is within 2.5 m, `decision.carry_cost` is subtracted from the pass score and the clearance score. A shot is never charged. A pressed carrier releases as today. So a one-touch pass under pressure stays possible, and an unpressed carrier carries or holds. The hold and dribble weights (`hold`, `hold_per_s`, `dribble_base`, `dribble_space`, `first_touch`) are retuned with it. The carrier still decides every tick (engine-core Q5), and no draw is added.
- **The delays move to the medians.** `throw_in` 13.8, `goal_kick` 23.2, `corner` 31.8 and `free_kick` 32.5 s (`01-engine-realism.md:614`). Players still walk to their restart targets during the delay. The hard limit stays three times the delay. `kick_off`, `penalty` and `drop_ball` are not named in the slice and have no sourced median, so they stay at their current values. Added time is not touched. During tuning a delay may move only between its current value and its median. It never goes above the median.
- **Three kinds of kick, three keys.** `Kick::Clear` is a clearance. The first kick by a restart taker, before anyone else touches the ball, is a restart kick. This covers a kick-off, a throw-in, a corner, a goal kick and a free kick. Precedence is restart kick, then clearance, then pass. A shot is always a shot, and so is a direct free kick struck at goal or a penalty. Only an open-play pass adds to `passes` and sets `pass_in_flight`, so `pass_accuracy_pct` is over open-play passes only. A defender's clearance of a fast pass also counts in `clearances`.
- **Ball in play is a counted figure.** `Summary::live_ticks` counts every `Phase::Live` tick outside the shoot-out. `match-stats` gets `stats.ball_in_play_s`, which is `live_ticks / 50` rounded, for the whole match. The calibrate report gets `ball_in_play_min_per_90_mean`, the mean of `ball_in_play_s / 60 × 90 / minutes`. For a 90-minute match that is the live minutes of the whole match, added time included, which is how "per 90 minutes of match time" reads. It is a reported figure and not a band. The bands file is the product owner's sourced data (RIM-8), and it has no ball-in-play band.
- **Corners from clearances and crosses (Q-KS2).** A fast open-play pass (above `control_speed`) inside the defending side's penalty area, under `reach_height`, can be cleared. Each active defending outfield player within `clearances.cross_reach` rolls `referee_draw() < clearances.cross_chance`, once per flight (a 22-bit mask cleared with the flight). A clearance deflects the ball with `shot::deflect`. It is turned around the direction away from the defender's goal centre by up to `cross_spread`, with `cross_speed` of the ball's speed and up to `cross_loft`. `deflected_by()` sets the defending side as the last touch. The pass is not completed, and one clearance is counted. The existing `ball_out` gives a corner only when the ball then crosses the goal line, and a throw-in when it crosses a touchline (RIM-9). A parry is not changed (Q-KS2: no parry turned toward the goal line).
- **Throw-ins by physical levers.** Throw-ins must rise from 20–24 to 35–55. `clearances.aim_spread` (today the hard-set 0.6 rad) and the cross clearance are tuned so more balls leave over a touchline. A throw-in is never scripted, and never awarded without the ball crossing the touchline.
- **Contracts moved only as the slice scope says.** The slice scope names the new keys and the contract update. `Summary` is in the snapshot, so the snapshot moves to version 6. The reader already refuses a file from another build, so nothing old is migrated. `04-plan.md` says later slices take the next number. The schema properties are optional. No protocol message, event kind or band changes. `stats.passes` keeps its name and now counts open-play passes, which is the criterion "Only real passes count". `TUNING_VERSION` stays 2, because every new tuning field has a default.
- **Measure with the tuning loop.** The inner loop is `calibrate --suite equal --seed 42 --matches 200 --baseline …` on the server. The criterion runs are the slow tests over 200 matches. The slice gate runs the full calibrate suites: five seeds for equal and strength, and one seed for formations (steer.md, Q-I1). Every heavy run goes to the server through `.scratch/remote/vps.sh`. The benchmark and the two Windows-only tests run on this PC (steer.md, 2026-09-25).
- **No NFR is the rationale for a mechanism choice.** The benchmark gate (+10% per tick) is the only performance line. Wall time per match does not change, because the tick count depends only on match time and added time. The 2000 ms per-match budget (NFR-1, `yields-to: C2`) is not in tension.

## Step-by-Step Plan

1. **Baselines.** At `bce0144`, before any code change, run on the server and copy each `report.json` into `implement-evidence/tempo-and-restarts/baseline/`:
   - `engine-cli calibrate --suite equal --seed 42 --matches 1000 --out ../out/tr-equal-base`;
   - `engine-cli calibrate --suite equal --seed 42 --matches 200 --out ../out/tr-set-base`.
   Record the command, commit and `fixtures.hash` of each (Q-X6). The benchmark baseline is already in `05c-benchmark.md` rev 9.
2. **Tuning values.** Add `carry_s` (0 to 5) and `carry_cost` (0 to 5) to `DecisionWeights` with `#[serde(default)]`. Add `ClearanceTuning` as `Tuning::clearances` with `#[serde(default)]` and `#[garde(dive)]`. It holds `aim_spread` (0 to 1.6, default 0.6), `cross_reach` (0 to 3), `cross_chance` (0 to 1, default 0.0), `cross_speed` (0 to 1), `cross_spread` (0 to 3.2) and `cross_loft` (0 to 15). Starting content values: `carry_s` 1.5, `carry_cost` 1.0, `aim_spread` 0.6, `cross_reach` 2.0, `cross_chance` 0.3, `cross_speed` 0.7, `cross_spread` 1.6, `cross_loft` 4.0. Set the four delays to 13.8, 23.2, 31.8 and 32.5 s. Mirror the content in `Default`. In `tests/content.rs`, pin the values, and add a test that a tuning block without the new fields loads with the defaults. `TUNING_VERSION` stays 2.
3. **Counting first, alone.** Add `Kick::Clear`, and return it from `Choice::Clear` with `clearances.aim_spread`. Add `clearances`, `restart_kicks` and `live_ticks` to `Summary`. Mark the restart taker in `take_restart()`, and clear the mark at every kick, every `gain()` by another player and every dead ball (with `pass_in_flight`). Count in `kick_ball()` by the precedence above. Count `live_ticks` in the `Phase::Live` arm outside the shoot-out. Update the `Kick` matches in `rules/mod.rs:901` and `scenario.rs`. Snapshot VERSION 6 with the three counters. Add the `match-stats` keys and the report figure. Then, with the delays and tuning still at today's values (a scratch content copy), run `calibrate --suite equal --seed 42 --matches 200` on the server. Record passes, accuracy and clearances. This separates the counting effect from the tuning.
4. **Seams.** Add `Scene::clear(dir, speed, loft)` under the `scenario` feature.
5. **Fast scenes** in `tests/tempo_and_restarts.rs`, built like `match_stats.rs` (a calm match, everyone else spread away):
   - **Clearance:** `Scene::clear` → `passes` 0, `clearances` 1, and no completion when a team-mate gains it.
   - **Restart kicks:** a ball over the touchline, then over the goal line off each side, then a goal. Step until each restart is taken → `restart_kicks[team]` 1 each, `passes` unchanged. The next open-play pass adds 1 to `passes`.
   - **Throw-in timing:** a throw-in is not taken before tick `since + 690` (13.8 s).
   - **Live ticks:** in a 5-minute match, `live_ticks + dead_ball_ticks` equals the ticks played, and `stats.ball_in_play_s` is `live_ticks / 50` rounded.
6. **Carry window.** In `options()`, subtract `carry_cost` from the pass and clearance scores while `held_s < carry_s` and not `pressed`. Unit tests in `decision.rs`: with the other weights fixed, an unpressed carrier at `held` 0 has his pass score lowered by exactly `carry_cost`; a pressed carrier and a carrier past `carry_s` do not; a shot score is unchanged. No draw is added (the draw count of one `options()` call is unchanged).
7. **Cross clearance.** Add `try_clear_cross()` to the carrier-less branch of `resolve_possession()`, after `contest_shot()` and before the ordinary contest. It runs only while `pass_in_flight` is set, the ball is fast, under `reach_height`, and inside the defending side's penalty area (`pitch::in_penalty_area`). It uses the per-flight mask and `referee_draw()`, then `shot::deflect` with a main-stream draw for the angle and the loft, then `deflected_by()`. It clears `pass_in_flight` and adds 1 to `clearances` for the defending side. Scenes: a fast lofted pass into the box with a defender 1 m from its path and rolls `[0.0]` → defending side last, `clearances` 1, `passes_completed` unchanged; the same with `[0.99]` → no clearance, and the flight continues; on seeds 1–40 with rolls `[0.0]` and the pass along the goal line, at least one seed gives a corner, and every corner comes after the ball crosses the goal line.
8. **Slow tests (ignored, release)** in the same file. A `OnceLock` holds the figures of 200 matches of the default clubs on seeds 1–200 (`common::run_many`), so the matches run once per test binary:
   - `passes_are_realistic`: mean passes per team 350–550; pooled accuracy (completed over passes) 75–88%. Both are printed.
   - `the_ball_is_in_play_for_about_an_hour`: the mean of `ball_in_play_s / 60 × 90 / minutes` is 52–65.
   - `throw_ins_stay_in_band`: mean throw-ins per match 35–55.
   - `restart_census`: prints each restart kind's median dead-ball seconds against the sourced median, clearances per team, and corners by origin (save or parry, block, cross clearance, other). It asserts that every corner follows a defending last touch with the ball at or beyond the goal line. It asserts no rate.
9. **Inner tuning loop (server).** Levers, each inside its bounds: `carry_s`, `carry_cost`, `hold`, `hold_per_s`, `dribble_base`, `dribble_space`, `first_touch`, `clearances.*`. The pass weights `lane`, `min_lane` and `distance` are used only if accuracy leaves 75–88. Each delay may move only between its current value and its median. After each change, run `calibrate --suite equal --seed 42 --matches 200 --baseline …/tr-set-base/report.json` and the census. Record each setting, its diff and its verdict in a table in the implement record. A change that the diff marks as noise is no change. Order: passes first, then throw-ins, then ball in play.
10. **Criterion runs (release, server).**
    - `cargo test --release -p engine --all-features --test tempo_and_restarts -- --include-ignored --nocapture`.
    - `calibrate --suite equal --seed 42 --matches 1000` (`passes_per_team`, `pass_accuracy_pct`, `throw_ins_per_match` bands, and `ball_in_play_min_per_90_mean`).
    If a criterion fails with every lever at every in-bounds setting, stop and report its figures and the settings tried. Do not widen a criterion, change a band, set a delay above its median, change added time, script a throw-in or a corner, or change the tick rate or the decision interval.
11. **Earlier criteria.** Run on the server in release: `keeper_and_shots set_pieces_arise_from_play` (at least 1.2 corners per team and 10 goal kicks per match), `defending every_formation_holds` and `discipline_is_realistic`, `lone_forward` slow tests and `strength`. Record each result. A criterion that passed before and fails now is a regression. Fix it inside this slice's levers, or stop and report.
12. **Re-derive moved expectations.** Use the method in step 12 of the `lone-forward` plan. Record the value before and after, and the reason. Never widen a tolerance.
13. **Contract and documents.** Add the optional schema properties. Change the `stats.passes` description to open-play passes in `match-stats.schema.json` and `docs/reference/protocol.md:257`. Add the three keys to the key list and to the `match-stats` paragraph of `.ai/observability.md`. Add the carry and clearance rows and the new delays to `docs/reference/data-files.md`, and note that `keeper_catch_chance` still covers every fast ball that no defender clears.
14. **Close.**
    1. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo test --workspace --all-features` on the server. Run the two Windows-only tests on this PC.
    2. Run the benchmark compare from `05c-benchmark.md` on this PC (gate 1.7071 µs per tick and 8.55 MB).
    3. Run the slice gate on the server: `calibrate --suite equal` and `--suite strength` on seeds 1, 7, 42, 99 and 2026, and `--suite formations --seed 42`, each at 1,000 matches. The formations suite takes about 2.6 hours, so run it in the background. Record band misses against `…/tr-equal-base`. They are recorded, not failed (Q-E4).
15. **Leak check.** Search the changed source, comments and commit text for workflow vocabulary before the commit.

## Verification Strategy

The four criteria are about match statistics and one counting rule. The engine produces them headless, so no screen is needed. The rung for each is headless runs of the real engine library or the real release binary (`cargo test` in `stack.testing`, and `engine-cli calibrate`). A unit test or static reasoning alone never counts.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| Passes are realistic (350–550 per team, 75–88%) | `cargo test --release -p engine --all-features --test tempo_and_restarts -- --include-ignored --nocapture passes_are_realistic` (headless engine runs, 200 matches) | Rust 1.92 and cargo on the server and on the reference PC — yes | `passes_are_realistic`; the counting change (step 3) | `engine-cli calibrate --suite equal --seed 42 --matches 200` (`passes_per_team_mean`, `pass_accuracy_pct_mean`) → the same on the five gate seeds → pre-registered deferral (not expected: no wall) |
| The ball is in play for about an hour (52–65 min per 90) | `… --test tempo_and_restarts … the_ball_is_in_play_for_about_an_hour` (200 matches) | yes | `Summary::live_ticks`, `stats.ball_in_play_s`, the slow test | `calibrate --suite equal --seed 42 --matches 200` (`ball_in_play_min_per_90_mean`) → pre-registered deferral |
| Throw-ins stay in band (35–55 per match) | `… --test tempo_and_restarts … throw_ins_stay_in_band` (200 matches) | yes | the slow test | `calibrate --suite equal --seed 42 --matches 200` (`throw_ins_per_match_mean`) → pre-registered deferral |
| Only real passes count | `cargo test -p engine --all-features --test tempo_and_restarts` (the clearance and restart-kick scenes on the real engine) | yes | `Scene::clear`; the restart scenes; `Kick::Clear`; the restart taker mark | a traced match: a stats record where `stats.passes + stats.clearances + stats.restart_kicks` equals the kicks in the step-3 count → pre-registered deferral |

No criterion depends on credentials, a device, an external service or missing infrastructure. Every run is local or on the server, and uses tools already in `stack:`. So no `constraint-resolution:` line is needed, and no wall is named. The server is an execution host that steer.md chose for heavy runs. It is not a verification tool outside `stack:`.

## Test / Verification Plan

### Automated checks

- Lint and type check: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings` (server).
- Unit: `decision.rs` (the carry window); `shot.rs` stays as it is.
- Integration (fast): `tests/tempo_and_restarts.rs` (the counting, timing, live-tick and cross-clearance scenes), `tests/content.rs`, `match_stats.rs`, `snapshot.rs`, `rules_restarts.rs`, `plugin_hooks.rs`. The full-match validator must still read 0 violations (`tests/validator.rs`).
- Slow (ignored, release, server): `passes_are_realistic`, `the_ball_is_in_play_for_about_an_hour`, `throw_ins_stay_in_band`, `restart_census`, `set_pieces_arise_from_play`, `every_formation_holds`, `discipline_is_realistic`, the `lone_forward` and `strength` slow tests.
- Calibrate: the equal suite against the step-1 baselines (inner loop and criterion), and the full slice gate (step 14).
- Benchmark compare against `05c-benchmark.md` (this PC).
- Page tests stay green: `node --test web/tests/*.test.mjs` (this PC). The page reads `stats.passes` unchanged in shape.

### Interactive verification (human-in-the-loop)

Automated only. Every criterion is a statistic of engine runs or a counting rule on scripted engine scenes. The page shows no new element. Its pass count now shows open-play passes, and a longer stoppage is a longer quiet spell that the existing skip-to-next-event control covers.

## Risks / Watchouts

- **The ball stays in play for more than 65 minutes** (high). Today it is 90.0 live minutes of 94.1. The engine awards 20.4 throw-ins, 12.3 goal kicks, 2.6 corners and 12.8 fouls per match. At the four medians those counts give about 20 minutes of dead time, which leaves about 70 live minutes. Longer carries mean more tackles, so more fouls and free kicks may follow, and the throw-in and cross-clearance levers add stoppages. The census shows each kind's median dead time against its source. If the figure stays above 65 with every lever in bounds, step 10 stops and reports. It never sets a delay above a median, and it never changes added time.
- **Throw-ins are 20–24, not 33–43** (high). Fewer passes can lower them further. The aim spread and the cross clearance are the levers. If 35 is out of reach, implement stops and reports.
- **Pass accuracy leaves 75–88%** (medium). Taking out clearances raises it, and taking out throw-ins lowers it. Step 3 measures the counting change alone first. The pass weights are secondary levers.
- **A deflected clearance can go into the defender's own goal** (low). The clearance turns around the direction away from goal, so this is rare. An own goal from a real deflection is legal play. The census counts goals after a clearance.
- **Every seeded result moves** (medium). Each moved expectation is re-derived and recorded.
- **Goals fall further** (low). Less live time means fewer shots, which the product owner accepted until `realism-tuning` (Q-E4).
- **Viewer timing** (low). A 32.5 s free kick is a longer quiet spell in the live stream. The skip-to-next-event control covers it, and the stream paces by ticks.
- **Per-tick cost** (low). The carry window adds no loop. The cross-clearance check is one pass over the players, and only while a fast pass is inside a penalty area. Dead-ball ticks are cheaper, so the per-tick mean may fall. The gate is 1.7071 µs per tick.

## Dependencies on Other Slices

- `keeper-and-shots` (complete, verified at `bce0144`): the block and deflection machinery this slice reuses, and its set-piece floor (at least 1.2 corners per team and 10 goal kicks per match), which must still pass. Q-KS2 moved corners from clearances and crosses here.
- `realism-bands-v2` (complete, verified): the passes, pass accuracy and throw-in bands.
- `tuning-loop` (complete, verified): `--baseline` and the noise mark.
- `lone-forward` and `defending-and-discipline` (complete, verified): their slow criteria must not regress.
- `realism-tuning` (next): final goals tuning, the corners band 3.5–6.5 and the red-card criterion. It plans after this slice.

## Assumptions

Autonomous run: no product owner was present. Each discovery question the interview would have asked is answered here in the direction that meets the criteria at the least cost. None changes the scope, a public contract beyond what the slice scope names, a persisted data shape beyond what the slice scope requires, or a product owner answer.

- A1 (class: implementation-detail): tempo is a soft carry window (`carry_s`, `carry_cost`) on the pass and clearance scores of an unpressed carrier, not a stored per-possession decision. The slice scope offers either. The window needs no snapshot field and no draw, and it leaves "every agent decides every tick" intact.
- A2 (class: implementation-detail): a pressed carrier (an opponent within 2.5 m, the existing `pressed` test) and a shot are exempt from the carry cost. A one-touch pass under pressure and a chance on goal stay possible.
- A3 (class: implementation-detail): the new decision weights default to 0.0 when a tuning file omits them, so an older file plays as today, as the `lone_*` weights do. The shipped values live in `content/tuning.json` and in `Default`.
- A4 (class: implementation-detail): the four named delays go to the sourced medians. Kick-off, penalty and drop ball are not named in the slice and have no sourced median, so they stay at 5, 15 and 20 s. Tuning moves a delay only between its current value and its median, because the slice says "toward the real medians".
- A5 (class: implementation-detail): added time is not changed. It is priced per stoppage kind and does not read the delays, so the 900 s clamp stays as it is and added time stays plausible.
- A6 (class: implementation-detail; ac: "Only real passes count"; classification: build-capability): a clearance is a `Kick::Clear` (the carrier's clearance choice, the keeper's included) or a defender's clearance of a fast pass. A restart kick is the first kick by the restart taker before anyone else touches the ball, at a kick-off, a throw-in, a corner, a goal kick or a free kick. Precedence is restart kick, then clearance, then pass. A shot is always a shot. Scenes on the real engine prove each rule.
- A7 (class: implementation-detail): pass accuracy is completed open-play passes over open-play passes. Clearances and restart kicks carry no completion count, because the slice counts them under their own keys and the accuracy band is about passes.
- A8 (class: implementation-detail): the new keys are `stats.clearances` and `stats.restart_kicks` (two integers, home first) and `stats.ball_in_play_s` (integer seconds, the whole match, the shoot-out excluded). They are optional in the schema, and nothing is added to `required`. The slice scope asks for these keys and for the contract update.
- A9 (class: implementation-detail): the snapshot moves to VERSION 6 to carry the three new `Summary` counters. The reader already refuses a file from another build, so no migration exists to write. `04-plan.md` says later slices take the next number.
- A10 (class: implementation-detail): the protocol does not change. `stats.passes` keeps its name and shape, and the viewer now shows open-play passes, which is the criterion itself. No clearance or ball-in-play figure is added to the page.
- A11 (class: implementation-detail; ac: "The ball is in play for about an hour"; classification: runtime-evidence): "per 90 minutes of match time" is read as the live minutes of the whole match, added time included, scaled by 90 over the match's regulation minutes. For the default 90-minute match the figure is the live minutes. It is judged over 200 matches of the default clubs on seeds 1–200.
- A12 (class: implementation-detail): the calibrate report shows `ball_in_play_min_per_90_mean` as a figure, not a band. The bands file is the product owner's sourced data (RIM-8), and adding a band is his decision. The slow test judges the criterion.
- A13 (class: implementation-detail): corners from clearances and crosses (Q-KS2) come from one mechanism. A defending outfield player can clear a fast open-play pass inside his own penalty area, once per flight, by deflection. The existing out-of-play rule gives the corner only after the ball crosses the goal line. A parry is not changed.
- A14 (class: implementation-detail): the clearance and cross-clearance values have no sourced figure in the repository, so they are tuning levers inside bounds. `cross_chance` defaults to 0.0, which is off, so an older file plays as today. The census reports the clearances per team for review.
- A15 (class: implementation-detail; ac: "Throw-ins stay in band"; classification: runtime-evidence): throw-ins are 20–24 per match on shipped play, below the slice's expected 33–43. They are raised only by physical levers: the clearance aim spread and the cross clearance. A throw-in comes only from a ball crossing the touchline. If 35 is out of reach in bounds, implement stops and reports. This keeps the criterion and the product owner's scope as they are.
- A16 (class: implementation-detail; ac: "Passes are realistic"; classification: runtime-evidence): 200 matches of the default clubs on seeds 1–200, with mean passes per team and pooled accuracy. The pass weights `lane`, `min_lane` and `distance` are levers only if accuracy leaves its band. They are decision weights, and the slice changes what a decision prefers.
- A17 (class: implementation-detail): the counting change lands and is measured alone (step 3) before any tuning, so the effect of the new count is known apart from the effect of the tempo.
- A18 (class: implementation-detail): the slow tests share one 200-match run per test binary through a `OnceLock`, so four tests cost one run.
- A19 (class: implementation-detail): augmentations. The benchmark is re-baselined on `bce0144` (`05c-benchmark.md` rev 9, 1.5519 µs per tick; gate 1.7071 µs and 8.55 MB), because the slice changes per-tick code. `04b-instrument.md` is not re-authored, because no dark path or dark-path counter is added. The three new keys are statistics, and the observability contract is amended in step 13. `04c-experiment.md` is not involved, because no flag is added.
- A20 (class: implementation-detail): the second-opinion consult is not fired, although `appetite-medium-or-larger` holds. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`), as in earlier plans.
- A21 (class: implementation-detail): the design gate is settled (`02c-craft.md`). This slice changes no page.
- A22 (class: implementation-detail): heavy runs (the workspace suite, clippy, fmt, the slow tests and every calibrate run) go to the server through `.scratch/remote/vps.sh`. The benchmark, the page tests and the two Windows-only tests stay on this PC (steer.md, 2026-09-25).
- A23 (class: implementation-detail): baselines are copied into the slice's evidence folder, and nothing new is committed for them (Q-X6). The slice gate runs the full suites (Q-I1). Band misses are recorded, not failed (Q-E4).
- A24 (class: implementation-detail): in `00-index.md` this run marks the slice `in-progress`, points the benchmark entry at this slice, adds the plan files, and moves the current stage to plan with this slice selected. `keeper-and-shots` verify is complete with result pass, and review is slug-wide, so no per-slice review waits on it.

## Blockers

None.

## Freshness Research

- No dependency is added or upgraded. The slice uses std `f64` arithmetic and the crates already in the workspace (`garde` for bounds, `serde` for defaults). `04-plan-data-schemas-generator.md` § Freshness Research records the `serde` 1.0 and `garde` 0.23 behaviour. A field-level `serde(default)` inside a `deny_unknown_fields` struct is already proven by the `lone_*` weights (`tuning.rs:356-368`). A nested optional block with `garde(dive)` is proven by `shots` (`tuning.rs:191-194`).
- The restart medians, the ball-in-play share and the foul rate are the sourced figures in `docs/design/realism/01-engine-realism.md:614`, `:693` and `:702`. The passes, accuracy and throw-in bands are the product owner's sourced values (Q-E1).
- The laws are not re-read here. The restart kinds and who takes them already follow IFAB Laws 8 and 13–17 (`rules/restart.rs:1-11`). This slice changes only how long the ball is dead before a restart, and how kicks are counted.

## Recommended Next Stage

- **Option A (default): Implement** → `/wf implement football-manager-match-engine tempo-and-restarts`. The plan has no blocker, and the benchmark is re-baselined. Compact the session first. Workflow state lives in the artifact files, and the SessionStart hook re-reads it after compaction.
- **Option C: Revisit slice** → `/wf slice football-manager-match-engine`. Take this only if implement shows that the ball-in-play or throw-in criterion cannot pass at any in-bounds setting (step 10 stop).
