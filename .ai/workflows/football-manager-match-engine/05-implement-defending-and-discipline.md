---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: defending-and-discipline
status: awaiting-input
stage-number: 5
created-at: "2026-09-24T03:50:37Z"
updated-at: "2026-09-24T03:50:37Z"
metric-files-changed: 31
metric-lines-added: 1442
metric-lines-removed: 123
metric-deviations-from-plan: 9
metric-review-fixes-applied: 0
commit-sha: "f7fe35b7eec5e18a81ba539578f917ded4b714da"
commits:
  - "f7fe35b7eec5e18a81ba539578f917ded4b714da"
awaiting-input-reason: "Two of four criteria fail at every in-bounds setting of the five tuning values: 'No advantage from a red card' (all three arms) and 'Every formation holds' (4-4-1-1 5.18 and 3-4-3 4.17 goals per match). The plan's step 14 stop rule applies: report and stop, never raise a limit."
steering-honored:
  - "Output boundary: code comments, commit text, docs and test names use product language; the commit message was leak-checked before commit."
  - "Dark-path counter definition: unchanged. The keeper substitution goes through the change queue; calibrate reports darkpath.change_never_applied 0 and change.expired_at_full_time 51 over 1,000 equal-suite matches."
  - "Design direction: not applicable; no page changed. The sub-keeper decision is an ai-decision event, which the page hides."
tags: [engine, tactics, rules, realism, defending, discipline, keeper]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-defending-and-discipline.md
  plan: 04-plan-defending-and-discipline.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/defending-and-discipline/
  siblings: [05-implement-realism-bands-v2.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-extra-time-penalties.md]
  verify: 06-verify-defending-and-discipline.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine defending-and-discipline"
---

# Implement: Defending and Discipline

## The Implementation

The engine at `2089ed7` had no marking, a back line with a 16 m central gap, and a card system that could show one player two cards on one tick. All four mechanisms the plan named are now in the engine and committed at `f7fe35b`: goal-side cover, pressers that run at the intercept point, a back line capped at 12 m between neighbours, and an acting keeper derived from the team's state. The discipline fixes are a 150-tick foul cooldown, one held card per player, and a booked-player factor of 0.15. Two of the four criteria pass. Discipline is realistic: 0.085 second yellows per match, 16.5% of matches with a sending-off, and no same-tick pair. The acting keeper covers a red card, which six scripted scenes show. The formations criterion improved from 7 failing pairings to 2 (4-4-1-1 at 5.18 and 3-4-3 at 4.17 goals per match). The red-card criterion still fails in all three arms: the reduced side outscores the full side.

Two load-bearing choices came out of measurement, not the plan. As written in the plan, the intercept and the cover both made goals worse: 14.2 goals per match and 47 penalties at seed 1 to 24. In this engine, contact near the ball produces a foul about four times as often as a won tackle (per tick, a 10% foul chance against about 2.5% for a win). So a presser now goes for the ball itself inside 3 m, and the cover tracks the ball carrier 3 m goal-side at the carrier's pace. It never steps out of the line to play a non-carrier onside. A sweep of all five tuning values within their bounds could not fix the two failing criteria, with 7 settings measured on 60 seeds. The cause is a lone central forward with no forward team-mate to pass to. He dribbles and shoots twice as often (40.6 shots against 20.8 in the control) and is never offside. The defending mechanisms do not reach that cause, and neither do the five tuning values.

The plan's step 14 rule applies: implement stops and reports the failing arms and pairings, and does not raise a limit. The product owner has to choose the next step. The options are to widen this slice to the carrier's dribble choice and the tackle-against-foul odds (which the slice lists as out of scope), to move the two criteria to `realism-tuning`, or to revisit the slice. `keeper-and-shots` can already build on `Simulation::keeper(team)`. The top open risk is the equal suite: goals per match rose from 3.035 to 4.092 at seed 42 against the realism-bands-v2 baseline. That rise is recorded, not failed (Q-E4).

## Summary of Changes

- **Tuning**: five new bounded values, each with a serde default, so an older file still loads: `cover_distance` 3.0 m (0–10), `cover_channel` 12.0 m (0–34), `back_line_gap` 12.0 m (4–30), `foul_cooldown_ticks` 150 (0–1000) and `foul_booked_factor` 0.15 (0–1). `TUNING_VERSION` stays 2.
- **Acting keeper**: `Team::keeper_slot()` returns slot 0 while it is active. Otherwise it returns a goalkeeper who came on, and otherwise the most advanced active outfield slot (ties go to the smaller `|y|`, then the lower slot). `Simulation::keeper(team)` maps that slot to a roster index. Every fixed "slot 0 is the keeper" check in decision, sim, rules, restart and the shoot-out uses it.
- **Shape**: `reshape()` now lays the whole team out again (`relayout()`). A lone survivor keeps its `y`. The back line (outfield slots within 4 m of the deepest one) is spread evenly whenever two neighbours stand more than `back_line_gap` apart, and the acting keeper takes the keeper's place. `Team::pressers()` gives `press_count` minus the players out of play, and never less than 1.
- **Defending in play**: pressers beyond 3 m run at the closed-form intercept point with a 1.0 s horizon; inside 3 m they go for the ball. Goal-side cover (`Simulation::cover`) picks the carrier when he is in the channel of the defending half, and otherwise the most advanced attacker there. The covering player is the nearest active back-line player who is neither pressing nor keeping goal.
- **Computer manager**: new code `sub-keeper`. When an outfield player keeps goal, a bench goalkeeper is free, no goalkeeper is already on the way, and a substitution is possible, the manager queues the best bench keeper for the stand-in. A send-off now asks the team's manager to check at once.
- **Discipline**: `Player::foul_ready` is set on every foul, advantage included, and the tackle loop skips a tackler before the draw until that tick. `Card::severity()`, `hold_card()` (one held card per player, the more severe) and a merge at a stopping foul give one card per player per stoppage. `foul_chance(tackler, yellows, t)` multiplies by the booked factor.
- **Snapshot**: version 5 carries `foul_ready` after the yellow cards.
- **Content and docs**: three `sub-keeper` commentary lines. The data-files reference has the five tuning rows and the cover, back-line and discipline rules; the protocol reference has the new decision value.
- **Tests**: three new test files (`defending.rs`, `discipline.rs`, `acting_keeper.rs`), new unit tests in `team.rs` and `rules/fouls.rs`, the tuning pin and older-file tests in `content.rs`, and a snapshot test that resumes through a foul cooldown.

## Files Changed

- `content/tuning.json`: five new values (3.0, 12.0, 12.0, 150, 0.15).
- `content/commentary/en.json`: three `sub-keeper` lines.
- `crates/engine/src/tuning.rs`: five bounded fields with serde defaults, mirrored in `Default`.
- `crates/engine/src/team.rs`: `keeper_slot`, `back_line`, `pressers`, the new `relayout`, `reshape` through `relayout`, the keeper anchor for the acting keeper, and unit tests.
- `crates/engine/src/tactics/mod.rs`: `TeamPlan.back_line_gap`, read from the tuning.
- `crates/engine/src/decision.rs`: `intercept`, the engage range, `cover`, the acting keeper in place of slot 0, `pressers()`.
- `crates/engine/src/sim.rs`: `keeper(team)`, the acting keeper in the save check and shots on target, the foul-cooldown skip, the booked foul chance, and the re-derived stoppage floor in a unit test.
- `crates/engine/src/player.rs`: the `foul_ready` field.
- `crates/engine/src/rules/mod.rs`: the cooldown on every foul, `hold_card`, the card merge, a manager check after a send-off, and the acting keeper at a penalty and in the shoot-out.
- `crates/engine/src/rules/fouls.rs`: `Card::severity`, the booked factor, and unit tests.
- `crates/engine/src/rules/restart.rs`: `taker` takes the teams, reads the own end from the attack direction and prefers the acting keeper; penalty targets and readiness use the acting keeper.
- `crates/engine/src/rules/shootout.rs`: the candidate doc names the acting keeper.
- `crates/engine/src/tactics/change.rs`: `relayout` after every substitution; the new `taker` signature.
- `crates/engine/src/ai.rs`: `AiCode::SubKeeper`, the keeper branch, `bench_keeper`, and the acting keeper left out of fatigue substitutions.
- `crates/engine/src/commentary/templates.rs`: `DecisionCondition::SubKeeper`.
- `crates/engine/src/snapshot.rs`: version 5 with `foul_ready`, and the version test.
- `crates/engine/src/scenario.rs`: `sent_off` clears the carrier and asks the manager to check; new `velocity` and `foul_ready` scene steps.
- `crates/engine/tests/defending.rs` (new): five scenes and two slow criterion tests.
- `crates/engine/tests/discipline.rs` (new): three scenes and the slow criterion test.
- `crates/engine/tests/acting_keeper.rs` (new): six scenes.
- `crates/engine/tests/content.rs`: the five values pinned, and an older tuning block loads with the defaults.
- `crates/engine/tests/snapshot.rs`: version 5 in two refusal tests, and a resume through a foul cooldown.
- `crates/engine/tests/rules_fouls.rs`: `tackle_in`, with the cooldown off in the held-card test.
- `crates/engine/tests/rules_cards.rs`: a tackle draw inside the stopping band for a booked tackler too.
- `crates/engine/tests/commentary.rs`: `SubKeeper` in the decision loop, and the re-derived line floor.
- `crates/engine/tests/plugin_hooks.rs`, `rules_restarts.rs`, `tactics_queue.rs`: the new `foul_chance` signature.
- `crates/engine-cli/tests/stream_cli.rs`: the re-derived frame count and reconnect seed.
- `docs/reference/data-files.md`: five tuning rows, the cover, back-line and discipline rules, and `sub-keeper`.
- `docs/reference/protocol.md`: the `sub-keeper` value.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/rules/mod.rs`, `restart.rs`, `snapshot.rs` (match-rules, extra-time-penalties): the `taker` signature changed for every caller, and the snapshot is now version 5.
- `crates/engine/src/ai.rs`, `tactics/change.rs` (tactics-and-ai): a new decision code, and a relayout after every substitution.
- `crates/engine/src/decision.rs`, `sim.rs`, `team.rs` (engine-core, tactics-and-ai): the pressing and anchor paths that `keeper-and-shots` and `realism-tuning` will change next.
- `content/tuning.json`, `content/commentary/en.json` (calibration, commentary): the content hash changed.

## Notes on Design Choices

- **Contact is expensive in this engine.** Every opponent within `reach_radius` draws once per tick, with about a 2.5% win chance and a 10% foul chance. Any mechanism that keeps a defender in contact floods fouls. Measured at seeds 1–24: pace-matched pressing gave 138 fouls and 86 penalties per match, and a jockeying presser gave 135 fouls and 77 penalties. So pressers go for the ball inside 3 m (`PRESS_ENGAGE_M`), and the cover stands 3 m off the carrier.
- **Cover depth.** A non-carrier is covered from the defender's own place in the line, so the cover never plays an attacker onside. The first version, which stepped onto every attacker, cut offsides from 8.9 to 1.5 per match and raised goals. A carrier is tracked `cover_distance` goal-side, with the spot led by the carrier's run over the defender's braking time. Otherwise steering's slow-down leaves the defender trailing.
- **Carrier first.** The traces showed goals from carriers who ran in from the wing while the cover watched a striker who stood still. So the carrier takes the cover whenever he is in the channel of the defending half.
- **Back line.** The back line is the slots within 4 m of the deepest outfield slot (`BACK_LINE_BAND`), because the shipped back threes and fives are not level (3-5-2 has 24, 22, 24). The cap spreads the line evenly only when a neighbour gap exceeds `back_line_gap`, so lines that are already compact keep their shape.
- **The acting keeper is derived.** A snapshot, a resume and the validator need no new team data. A goalkeeper who comes on for the stand-in takes the goal from the stand-in's slot, and the forward line stays one short.

## Verification Seams Built

- No advantage from a red card → `a_sending_off_gives_no_advantage` at `crates/engine/tests/defending.rs:167`: seeds 1–120, cards off through the three card chances, and arms for the keeper (11), centre-back (13) and striker (21). It uses `Scene::sent_off` (`crates/engine/src/scenario.rs:91-100`), which now clears the carrier and asks the manager to check (enables `cargo test --release -p engine --all-features -- --ignored`).
- Every formation holds → `every_formation_holds` at `crates/engine/tests/defending.rs:203`: each of the 10 shipped formations against 4-4-2, home on odd seeds (same command).
- Discipline is realistic → `discipline_is_realistic` at `crates/engine/tests/discipline.rs:140`: 200 matches, card events read per tick, plus `Scene::foul_ready` at `crates/engine/src/scenario.rs:112-116`.
- The acting keeper covers a red card → `crates/engine/tests/acting_keeper.rs` (six scenes: the bench keeper at `:53`, no substitution left at `:86`, the catch chance at `:130`, the penalty at `:143`, the shoot-out at `:179`, the goal kick after half-time at `:213`), `Simulation::keeper` at `crates/engine/src/sim.rs:565-567`, and `Scene::velocity` at `crates/engine/src/scenario.rs:102-110` for the presser scene.

## Deviations from Plan

1. **Cover target (step 9).** Plan: cover the most advanced central attacker at the attacker's position moved `cover_distance` toward goal. Built: the carrier first; a carrier is tracked `cover_distance` goal-side at his pace; any other attacker is covered at the defender's line depth. Reason: with pressers already engaging inside 3 m, the plan's version raised goals from 2.66 to 4.62 per match at seeds 1–24 (with the intercept off it cut offsides from 8.9 to 1.5 per match), and it left wing runners uncovered. Class: implementation-detail.
2. **Presser target (step 8).** Plan: the intercept point for every presser. Built: the intercept point beyond 3 m, the ball inside it. Reason: the intercept at close range put pressers in the carrier's path and gave 14.2 goals, 102 fouls and 47 penalties per match at seeds 1–24. Class: implementation-detail.
3. **Back-line cap (step 3).** Plan: cap the deepest line's width at `(n − 1) × back_line_gap`. Built: a back line of the slots within 4 m of the deepest outfield slot, spread evenly when any neighbour gap exceeds the gap. The value lives in `TeamPlan.back_line_gap`, because `relayout` has no tuning in scope. The results match the plan's examples (−18, −6, 6, 18 and −12, 0, 12). Class: implementation-detail.
4. **`reshape` lays the whole team out again.** The plan kept a line-by-line spread in `reshape` and a separate `relayout` call in `send_off`. One path is simpler and applies the acting keeper and the cap everywhere, injuries included. Class: implementation-detail.
5. **`restart::taker(kind, team, spot, players, teams)`.** The plan passed `own_end: f64`. Passing the teams gives both the own end (from `attack_x`) and the acting keeper without a second parameter. Class: implementation-detail.
6. **Order of the keeper branch in the computer manager (step 7).** The plan made it the first branch. Built: after the injury branch, and only when no goalkeeper is already coming on. Otherwise an injured keeper would draw two goalkeeper substitutions, or put an outfield player in slot 0 and the real keeper up front. The acting keeper is also left out of fatigue substitutions. Class: implementation-detail.
7. **Extra scene steps.** `Scene::velocity` (a running carrier for the presser scene), and `Scene::sent_off` also asks the manager to check, as `show_card` does. Class: implementation-detail.
8. **Tuned values (step 14).** `cover_distance` 2.0 → 3.0 and `foul_booked_factor` 0.5 → 0.15. The other three stay at the plan's starting values. The runs that chose them are under Tuning Loop. Class: implementation-detail.
9. **Re-derived vacuity floors and fixtures (step 15).** These are recorded under Moved Expectations. Two non-realism floors were lowered after a measured drop, and one test's seed was changed. Class: implementation-detail. Reviewers should check them, because the plan says "never widen a tolerance"; these floors guard against an empty check and bound no measured behaviour.

No deviation is of kind "planned API not found".

## Tuning Loop (step 14)

Every run used a release build. Figures are goals per match unless named.

| Setting (others at shipped values) | Red-card arms, full – reduced side (keeper / centre-back / striker) | 4-4-1-1 v 4-4-2 | 3-4-3 v 4-4-2 | Seeds |
|---|---|---|---|---|
| shipped: cover 3.0, channel 12, gap 12 | 2.50–5.92 / 0.60–2.40 / 1.00–3.83 (control 0.80–2.33) | 5.27 | 4.00 | 1–60 |
| cover_distance 1 | 2.28–5.92 / 0.85–2.42 / 1.13–4.13 | 5.35 | 4.40 | 1–60 |
| cover_distance 6 | 1.72–6.82 / 0.80–2.12 / 1.17–4.32 | 6.90 | 3.68 | 1–60 |
| cover_distance 10 (bound) | 1.55–7.43 / 1.02–1.73 / 0.97–5.63 | 11.12 | 6.97 | 1–60 |
| cover_channel 20 | 1.65–5.67 / 0.75–2.27 / 0.85–4.27 | 6.05 | 4.85 | 1–60 |
| cover_channel 34 (bound) | 1.42–5.10 / 0.88–2.37 / 0.77–4.12 | 6.12 | 3.82 | 1–60 |
| back_line_gap 6 | 0.82–4.90 / 0.58–0.73 / 0.57–3.52 | 5.12 | 3.97 | 1–60 |
| back_line_gap 20 | 2.13–6.00 / 1.98–2.80 / 0.90–3.67 | 6.02 | 5.38 | 1–60 |

In no setting does the reduced side score at most what the full side scores in all three arms, and 4-4-1-1 never falls under 4.0. Discipline was tuned on the 200-match test. The results were: cooldown 150 with booked factor 0.5 gave 0.335 second yellows per match and 36.0% of matches with a sending-off; 150 with 0.2 gave 0.125 and 19.5%; 500 with 0.5 gave 0.335 and 37.0%; 500 with 0.2 gave 0.120 and 19.5%; 1000 with 0.0 gave 0.000 and 8.5%; 150 with 0.15 gave 0.085 and 16.5% (chosen); 150 with 0.1 gave 0.065 and 14.5%. The booked factor is the lever, and the cooldown barely moves the rate.

## Criterion Results (this run)

Evidence: `implement-evidence/defending-and-discipline/criteria-defending.*` (exit code 101) and `criteria-discipline-and-slow.*` (exit code 0).

| Criterion | Result | Figures |
|---|---|---|
| No advantage from a red card | **fails, all three arms** | control 0.78–2.23; keeper: full 2.06, reduced 5.82 (limit for the full side 1.24); centre-back: full 0.71, reduced 2.39; striker: full 0.95, reduced 3.84 |
| Every formation holds | **fails, 2 of 10 pairings** | 4-4-1-1 5.18–1.00, 3-4-3 4.17–1.26. Passing: 4-4-2 1.43–1.59, 4-3-3 3.15–0.68, 4-2-3-1 3.09–2.64, 3-5-2 1.33–2.76, 4-1-4-1 2.07–0.87, 4-1-2-1-2 1.10–1.19, 5-3-2 0.72–1.41, 5-4-1 3.12–1.63 (baseline at `2089ed7`: 7 of 9 non-mirror pairings failed) |
| Discipline is realistic | passes | 0.085 second yellows per match, 16.5% of matches with a sending-off, 0 same-tick pairs |
| The acting keeper covers a red card | passes (scenes) | 6 of 6 scenes in `acting_keeper.rs` pass; the keeper arm of the red-card experiment runs through the acting keeper, but that arm fails for the reason above |

Cause found for the two failures. Figures are for the away side, cards off, seeds 1–48. A side that loses its striker plays with one central forward: 40.6 shots against 20.8 in the control, 0.0 offsides against 1.2, and the home side fouls 35.5 times against 17.0. With no forward team-mate to pass to, the carrier's option scores favour a dribble and a shot. The traces show dribbles of 6–20 seconds through the defence. 4-4-1-1 and 3-4-3 have the same single central forward. In the keeper arm, the stand-in comes from the strike pair, which leaves a lone forward. Neither the decision weights nor the tackle odds are among the five values this slice may tune.

## Moved Expectations (step 15)

- `crates/engine/src/team.rs` `a_back_four_with_one_sent_off_spreads_three_across_the_width` became `a_back_four_with_one_sent_off_closes_to_three_and_a_lone_striker_keeps_his_side`. The line changed from −22, 0, 22 to −12, 0, 12 (the gap cap), and `formation[10].1` from 0.0 to 8.0 (the lone striker keeps his side). Both values follow from the plan's rules.
- `team.rs` `restoring_a_slot_puts_its_line_back_and_keeps_the_others_closed`: it now compares against the capped layout (−18, −6, 6, 18), not the raw `FORMATION_442`, and `formation[10].1` is 8.0 instead of 0.0.
- `crates/engine/tests/rules_cards.rs`: the tackle draw moved from `p_win + 0.1 × p_foul` to `p_win + 0.01 × p_foul`. With the booked factor at 0.15, the old draw fell in the advantage part of a booked tackler's foul band, so no card showed on that tick. The assertions did not change.
- `crates/engine/tests/rules_fouls.rs` `a_card_held_for_advantage_is_shown_at_the_next_stoppage`: the foul cooldown is set to 0, because the scene needs the same player to foul on two ticks running. The assertions did not change.
- `crates/engine/src/snapshot.rs` and `tests/snapshot.rs`: "this build reads 4" became "reads 5" in three refusal messages.
- `crates/engine/src/sim.rs` `the_last_kicker_is_clear_whenever_play_stops`: the vacuity floor went from more than 50 to more than 30 stoppages. The seed-42 match now stops play 39 times, against 58 at base, with fouls 18 against 22, throw-ins 16 against 27 and offsides 2 against 9. The per-stoppage assertion did not change.
- `crates/engine/tests/commentary.rs`: the line floor went from more than 100 to more than 80. The seed-42 match gives 90 lines, against 113 at base, measured on a `2089ed7` export this run. Every per-line assertion is unchanged.
- `crates/engine-cli/tests/stream_cli.rs`: a seed-7 one-minute recording now has 3,126 frames, not 3,127, because it no longer contains a throw-in. The reconnect test moved from seed 42 to seed 7, because seed 42's two-minute match no longer stops play before half-time; seed 7 stops at ticks 2,198 and 3,000.
- `full_match.rs`, `strength.rs`, `mentality.rs` and `ai_trailing.rs` did not move. Seed 42 committed 7 and 11 fouls, inside 4–20. The stronger team won 170, drew 20 and lost 10 of 200. Mean shots were 9.07 defensive and 14.49 attacking. Tactics changes applied in 99 of 100 scenes. The values before this change were not captured in this run.

## Checks Run

- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exit 0.
- `cargo test --workspace --all-features --no-fail-fast`: 458 passed, 0 failed, 7 ignored (`implement-evidence/defending-and-discipline/workspace-tests.*`, exit 0).
- Slow tests, release: `discipline_is_realistic`, `strength`, `mentality` and `ai_trailing` pass (exit 0); `a_sending_off_gives_no_advantage` and `every_formation_holds` fail (exit 101).
- Benchmark compare (`engine-cli bench --seed 42 --matches 5 --json`, 3 drives): 1.5099, 1.5422 and 1.5312 µs of processor time per tick (median 1.5312, +3.8% on 1.4753; gate 1.6228). Peak memory was 6.816, 6.797 and 6.797 MB (gate 8.27). Matches run 291,800 ticks, against 286,050 at base. The benchmark gate passes.
- `engine-cli calibrate --suite equal --seed 42` (exit 2, band misses recorded, not failed, per Q-E4), against the realism-bands-v2 baseline at seed 42: goals per match 3.035 → 4.092; sending-off share 0.421 → 0.326; ten-plus-goals share 0.063 → 0.043; goalless share 0.192 → 0.047 (now in band); shots per team 14.753 → 17.165; throw-ins per match 25.958 → 17.976; goal kicks 7.402 → 7.926; corners per team 0.004 → 0.012; yellow cards per team 1.6 → 1.821 (in band); shots-on-target share 0.750 → 0.769; `validate.violations` 0; `darkpath.change_never_applied` 0.

## Anything Deferred

- The two failing criteria wait for a product-owner decision (see Blockers). Nothing was loosened to pass them.
- `sdlc-debt`: none added. Two hard-set constants carry comments at their sites: `PRESS_ENGAGE_M` (3.0 m) and `BACK_LINE_BAND` (4.0 m). `INTERCEPT_HORIZON_S` (1.0 s) is also a code constant, as the plan asked.

## Known Risks / Caveats

- **The equal suite scores more.** At seed 42, goals per match rose from 3.035 to 4.092, although the default-teams criterion runs fell. `realism-tuning` owns this band, but the rise is the opposite of the slice's expected risk ("goals will fall below the band").
- **The sending-off share depends on the teams.** The default-teams run gives 16.5%, and the equal suite gives 32.6% at seed 42 (band 8–22%). The criterion measures the default teams (plan A14), so a verify that uses the calibrate suite as its fallback would see a different figure.
- **The acting keeper can change on a formation change** while an outfield player keeps goal (plan risk; unchanged).
- **The script decision hook still reports the formation slot** for an outfield stand-in (plan A16; unchanged).
- **Scratch output:** release builds of a `2089ed7` export and a `CARGO_TARGET_DIR` under the session scratch folder (`scratchpad/b`, `scratchpad/base/target`). Both are outside the repository.

## Blockers

- **The product owner must decide how to treat the two failing criteria.** The plan's step 14 says: "If a criterion still fails with those values at their bounds, stop and report the failing arm or pairing. Do not change the limits, the card chances or the goal tuning." Every option changes what this slice promises, so implement did not choose. The option list is in `po-answers.md` under this stage's entry.

## Freshness Research

- No dependency was added or upgraded. `serde`'s field-level `default` with a `deny_unknown_fields` container is shown to load an older tuning block by `a_tuning_block_without_the_defending_values_loads_with_the_defaults` (`crates/engine/tests/content.rs`), which passes.

## Recommended Next Stage

- **Option D: Blocked (current).** The product owner answers the pending question in `po-answers.md`, and then one of the routes below follows.
- **Option C: Revisit plan or slice** → `/wf plan football-manager-match-engine defending-and-discipline` (or `/wf slice football-manager-match-engine`). Take this if the answer widens the slice to the carrier's dribble choice or the tackle-against-foul odds, or moves the two criteria to `realism-tuning`.
- **Option A: Verify** → `/wf verify football-manager-match-engine defending-and-discipline`. Take this only after the product owner accepts the current results for the two failing criteria. Compact the session first: workflow state lives in the artifact files on disk, and the SessionStart hook re-reads it after compaction.
