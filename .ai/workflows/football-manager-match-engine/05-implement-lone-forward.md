---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: lone-forward
status: complete
stage-number: 5
created-at: "2026-09-24T12:41:48Z"
updated-at: "2026-09-24T16:09:33Z"
revision-count: 2
revisions:
  - rev: 1
    at: "2026-09-24T13:49:15Z"
    trigger: answers-returned
    because: "the product owner answered Q-LF1 (2026-09-24T13:19:59Z): measure the red-card criterion on balanced fixtures (equal clubs or both home and away orders, pooled), with every limit unchanged and no new lever, and stop again if an arm still fails"
    changed: "the red-card suite and the slow test now play seeds 1-120 in both club orders (commit 7bd5fa5); 11 balanced tuning runs; every realistic setting still fails the keeper and striker arms, so Q-LF1 is resolved and Q-LF2 is open; status stays awaiting-input"
  - rev: 2
    at: "2026-09-24T16:09:33Z"
    trigger: answers-returned
    because: "the product owner answered Q-LF2 (2026-09-24T14:15:39Z): the red-card criterion moves to keeper-and-shots; ship the formations fix with the two levers tuned within their bounds and judge this slice on its other three criteria"
    changed: "shipped tackle_win_base 0.5, lone_hold 0.5 and lone_dribble -0.5 (commit 07d87c2); four seeded tests re-derived; every criterion run, the slow tests, the benchmark and the full slice gate ran on the shipped values; the three kept criteria pass; status complete"
metric-files-changed: 27
metric-lines-added: 558
metric-lines-removed: 89
metric-deviations-from-plan: 8
metric-review-fixes-applied: 0
commit-sha: "07d87c25b226a618dc59e1279ff407f512d6a36b"
commits:
  - "c3cc96fabcb9059bb4d79b0be7ba1450f19aabb0"
  - "7bd5fa59d67bd1d624d500032a1ae98835b68214"
  - "07d87c25b226a618dc59e1279ff407f512d6a36b"
has-blockers: false
open-questions: []
steering-honored:
  - "Red-card criterion moves to keeper-and-shots (Q-LF2, 2026-09-24): this slice is judged on three criteria (the lone-forward formations hold, nothing that passes now regresses, a lone forward uses his team-mates). The red-card slow test and suite are unchanged and still fail; keeper-and-shots owns them. No lever was added, no limit changed, no setting below the shot floor was used, and no card chance changed."
  - "Red-card criterion fixtures (Q-LF1, 2026-09-24): the balanced fixtures built at 7bd5fa5 stay as the red-card experiment for keeper-and-shots."
  - "Moved criteria and slice order (2026-09-24): the 4-4-1-1 and 3-4-3 pairings are judged here with the 4.0 limit unchanged. No limit, card chance, keeper value, defending value, band or test tolerance was changed."
  - "Q-I1: targeted calibrate runs were the inner loop; the full slice gate ran on the shipped values (equal and strength on seeds 1, 7, 42, 99 and 2026, formations on seed 42, 1,000 matches each)."
  - "Output boundary: code comments, test names, docs and all three commit messages use product language; the added lines were leak-checked before each commit."
  - "Design direction: not applicable; no page changed."
tags: [engine, tactics, rules, realism]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-lone-forward.md
  plan: 04-plan-lone-forward.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/lone-forward/
  balanced-evidence: implement-evidence/lone-forward/balanced/
  shipped-evidence: implement-evidence/lone-forward/shipped/
  history: [history/05-implement-lone-forward-0.md, history/05-implement-lone-forward-1.md]
  siblings: [05-implement-defending-and-discipline.md, 05-implement-tuning-loop.md, 05-implement-realism-bands-v2.md, 05-implement-tactics-and-ai.md]
  verify: 06-verify-lone-forward.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine lone-forward"
---

# Implement: Lone forward

## The Implementation

The slice inherited both levers from `c3cc96f` at neutral values and the balanced red-card fixtures from `7bd5fa5`. On those fixtures the keeper and striker arms failed at every realistic setting. The product owner answered Q-LF2: the red-card criterion moves to `keeper-and-shots`, and this slice ships the formations fix with the two levers tuned inside their bounds. It is judged on its other three criteria.

Commit `07d87c2` (10 files, +52 −36) ships `tackle_win_base` 0.5 (the bound), `lone_hold` 0.5 and `lone_dribble` −0.5. `lone_layoff` and `lone_line_hold` stay 0, because a positive lay-off collapsed attacking play and the line hold made the pairings worse in the inner loop. The weights are the smallest symmetric pair that passes the pressed-forward scene: ±0.25 fails on seed 5. The scene now reads the shipped weights, not its own. At these values 4-4-1-1 and 3-4-3 score 3.22 and 2.91 against 4-4-2 (limit 4.0), all ten pairings with 4-4-2 hold, and discipline passes (second yellows 0.045 per match, 11.0% of matches with a sending-off, no same-tick pairs). The seed-42 match now ends 0-0, so four seeded tests move to a match that still shows what they check. No tolerance was widened.

Verify can now judge the three kept criteria on shipped play. The top risk is realism outside this slice's criteria: an even tackle now wins the ball ten times as often, so fouls fall from 13.8 to 7.0 per team and yellow cards to 0.91 per team, below the 1.2 band floor. Band misses are recorded, not failed (Q-E4). The red-card slow test still fails. It now belongs to `keeper-and-shots`.

## Summary of Changes

- Shipped tuning (`07d87c2`): `tackle_win_base` 0.5, `decision.lone_hold` 0.5, `decision.lone_dribble` −0.5 in `content/tuning.json` and in `Tuning::default()`, which the shipped file must equal. The serde defaults for an older tuning file stay at the values that reproduce play before these fields (0.05 and 0), and the older-file load test still proves them.
- The pressed-forward scenes read the shipped weights (`tests/lone_forward.rs`), with a guard that the shipped weights favour holding over dribbling.
- Four seeded tests moved to another match (see Deviations): the event-name check, the commentary check and the snapshot resume use the seed-7 match, and the viewer reconnect test uses seed 5.
- `docs/reference/data-files.md` lists the shipped values and says what a file without them loads.
- Balanced red-card fixtures (`7bd5fa5`): every seed of the red-card suite plays twice, the second time with home and away swapped. The slow test plays seeds 1–120 in both orders (240 matches per arm), the same as `--seed 1 --matches 240`. This stays as the experiment for `keeper-and-shots`.
- Five bounded tuning values (`c3cc96f`): `tackle_win_base` (0 to 0.5), `lone_line_hold` (0 to 1), and `decision.lone_layoff`, `decision.lone_hold`, `decision.lone_dribble` (−5 to 5). `TUNING_VERSION` stays 2.
- `fouls::win_chance()` holds the clean-tackle formula once. The tackle loop and nine copies in eight test files call it (`injury.rs` keeps its 0.5 factor).
- `offside::second_last_depth()` is extracted from `offside_set()`, so the referee and the forward read one line.
- `Team::lone_forward()` gives the single active front-line slot that is not keeping goal.
- In the scored options, a lone carrier (outfield, with no active outfield team-mate ahead) gets `lone_layoff` on each pass with progress ≤ 0. When he is also pressed, he gets `lone_hold` on holding the ball and `lone_dribble` on a dribble. No random draw is added.
- Off the ball, `hold_the_line()` moves the team's lone forward toward the line minus 0.5 m by `lone_line_hold`. At share 0 (shipped) the step returns early.

## Files Changed

- `content/tuning.json` (`07d87c2`): `tackle_win_base` 0.5, `lone_hold` 0.5, `lone_dribble` −0.5 (`c3cc96f` added the five fields at neutral values).
- `crates/engine/src/tuning.rs` (`07d87c2`): `Default` uses `TACKLE_WIN_BASE` (0.5), `lone_hold` 0.5 and `lone_dribble` −0.5; the serde defaults stay neutral. (`c3cc96f`: the five fields with garde ranges and serde defaults.)
- `crates/engine/tests/content.rs` (`07d87c2`): the pin test pins the shipped values. (`c3cc96f`: the pin test and the older-file load test.)
- `crates/engine/tests/lone_forward.rs` (`07d87c2`): `weighted()` uses the shipped weights. (`c3cc96f`: four scene tests.)
- `crates/engine/src/sim.rs` (`07d87c2`): the unit-test helper `full_match` takes a seed; `every_play_event_names_a_player` uses seed 7. (`c3cc96f`: the tackle loop calls `win_chance()`.)
- `crates/engine/tests/common/mod.rs` (`07d87c2`): `scoring_match()`, the seed-7 90-minute match.
- `crates/engine/tests/snapshot.rs` (`07d87c2`): the continuation and round-trip tests use `scoring_match()`.
- `crates/engine/tests/commentary.rs` (`07d87c2`): the full-match commentary test uses `scoring_match()`; its comment names the new line count.
- `crates/engine-cli/tests/stream_cli.rs` (`07d87c2`): the reconnect test serves seed 5.
- `docs/reference/data-files.md` (`07d87c2`): shipped values in three rows and in the paragraph. (`c3cc96f`: five rows and the paragraph.)
- `crates/engine-cli/src/calibrate/fixtures.rs`, `worker.rs`, `crates/engine/tests/defending.rs`, `docs/how-to/calibration.md`, `docs/reference/cli.md` (`7bd5fa5`): balanced red-card fixtures.
- `crates/engine/src/rules/fouls.rs`, `rules/offside.rs`, `team.rs`, `decision.rs` (`c3cc96f`): `win_chance()`, `second_last_depth()`, `lone_forward()`, the lone test, the three terms and `hold_the_line()`, with unit tests.
- `crates/engine/tests/{acting_keeper,discipline,injury,plugin_hooks,rules_cards,rules_fouls,rules_restarts,tactics_queue}.rs` (`c3cc96f`): the copied 0.05 formula becomes `win_chance()`.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/decision.rs`, `team.rs`, `sim.rs`, `tuning.rs`, `content/tuning.json`: also changed by `defending-and-discipline` (cover, press, acting keeper). Its mechanisms are not changed here. The defending tests pass unchanged.
- `docs/reference/data-files.md`: also changed by `defending-and-discipline` and `realism-bands-v2`.
- `crates/engine/tests/common/mod.rs`, `snapshot.rs`, `commentary.rs`, `crates/engine-cli/tests/stream_cli.rs`: shared test fixtures of earlier slices (`commentary`, `viewer-reports-recovery`, `stream-protocol`). Only the seed of the match each test plays changed; `common::full_match()` stays the seed-42 match for every other test.

## Notes on Design Choices

- The shipped weights are the smallest symmetric pair that passes the pressed-forward scene on all 40 seeds. On the scene, −0.5 dribble alone passes, 0.5 hold alone fails (seed 1), and ±0.25 fails (seed 5). A larger pair would move shipped play more for no measured gain.
- `lone_layoff` and `lone_line_hold` ship at 0. A positive lay-off gave 3.8 shots per team (0.3, first run), and the line hold at 0.6 raised 4-4-1-1 from 5.14 to 6.04 in the inner loop. The criterion "passes or holds" is met by the hold and dribble terms.
- `Tuning::default()` mirrors the shipped file, as the existing test `the_shipped_tuning_equals_the_documented_default` requires, but an older file still loads with the neutral values. This follows the plan (step 2: serde default 0.05) and keeps an older file's play unchanged.
- Every new term is exact zero at the neutral values and draws nothing, so the neutral check proved "the seams change nothing" bit for bit (red-card diff: 6 rows, change 0).

## Verification Seams Built

- "A lone forward uses his team-mates" → `tests/lone_forward.rs:67` `a_pressed_lone_forward_passes_or_holds_instead_of_dribbling`, 40 seeds on the real engine with the shipped weights (`weighted()` at `:38`, which checks that they favour holding). The seams are `decision.rs:264` (lone test), `:325` (lay-off) and `:348` (pressed terms). This enables `cargo test -p engine --all-features --test lone_forward`.
- Line hold → `team.rs:280` `lone_forward()`, `offside.rs:35` `second_last_depth()`, `decision.rs:119` `hold_the_line()`, observed by `tests/lone_forward.rs:109` at the shipped share and at 1.0.
- Tackle odds → `fouls.rs:75` `win_chance()`, called at `sim.rs:1015`, observed by `tests/lone_forward.rs:171`.
- Tuning seams → the shipped values at `content/tuning.json:52`, `:85`, `:86` and `tuning.rs:355`, `:388`, `:389`, `:421`, pinned at `tests/content.rs:178`, with the older-file load at `:190`.
- "The lone-forward formations hold", "Nothing that passes now regresses": nothing new was needed. `every_formation_holds` (`tests/defending.rs:250`) and `discipline_is_realistic` (`tests/discipline.rs:140`) exist unchanged.
- Seeded fixtures for the moved tests → `common/mod.rs:49` `scoring_match()`, used at `snapshot.rs:57`, `:103`, `:133` and `commentary.rs:231`; `sim.rs:1160` (`full_match(7, …)`); `stream_cli.rs:147` (seed 5).

## Tuning Loop (step 10)

### Shipped setting (after Q-LF2)

The setting is `tackle_win_base` 0.5, `lone_hold` 0.5, `lone_dribble` −0.5, with the other three tuning values unchanged. Inner loop: `calibrate --suite formations --pairing "4-4-1-1 v 4-4-2" --pairing "3-4-3 v 4-4-2" --seed 42 --matches 1000` on a content copy (`shipped/tuning-log.txt`): 4-4-2 0.872 against 4-4-1-1 3.198, and 4-4-2 0.743 against 3-4-3 2.951. Fouls were 9.8 per team, shots 19.3, yellow cards 1.29 and the sending-off share 0.219. The red-card suite was not run on this setting, because the criterion moved.

Scene check, `a_pressed_lone_forward_passes_or_holds_instead_of_dribbling` over seeds 1–40, with the weights set by a temporary probe that was reverted before the commit:

| lone_layoff / lone_hold / lone_dribble | Result |
|---|---|
| 0 / 0 / 0 | fails, seed 1 (the carrier dribbles to (38, 4)) |
| 0 / 0.25 / −0.25 | fails, seed 5 |
| 0 / 0.5 / 0 | fails, seed 1 |
| 0 / 0 / −0.5 | passes |
| 0 / 0.5 / −0.5 (shipped) | passes |
| 0 / 0.75 / −0.75 | passes |
| 0 / 1 / −1 | passes |
| 0.1 / 0.5 / −0.5 | passes |


### Balanced fixtures (after Q-LF1)

Inner loop: `calibrate --suite red-card --seed 1 --matches 240` (seeds 1–120 in both club orders, the slow test's experiment). Each setting was written to a copy of the content folder and passed with `--content-dir` (`implement-evidence/lone-forward/tune2.py`). The log is `balanced/tuning-log.txt` (11 rows), and each run's `report.json` is in `balanced/`. Figures are goals per match: full/reduced per arm. The limit is 1.6 × control home. Shots are per team per match over the four arms.

| Setting | Control (home–away) | Limit | Keeper F/R | Centre-back F/R | Striker F/R | Shots | Verdict |
|---|---|---|---|---|---|---|---|
| neutral | 1.48–1.52 | 2.37 | 2.35/4.56 | 0.98/1.48 | 1.12/2.63 | 20.0 | all 3 arms fail |
| win 0.25 | 1.15–1.31 | 1.83 | 2.17/3.38 | 1.02/1.32 | 1.25/2.08 | 16.7 | all 3 fail |
| win 0.5 (bound) | 1.15–1.23 | 1.84 | 1.90/2.46 | 1.09/1.21 | 1.25/1.67 | 15.0 | all 3 fail |
| win 0.5, foul_base 0.05 | 1.00–1.18 | 1.60 | 1.68/2.22 | 1.07/1.30 | 1.24/1.78 | 14.3 | all 3 fail |
| win 0.5, foul_base 0.02 | 1.08–1.02 | 1.73 | 1.60/2.11 | 1.07/1.23 | 1.27/1.50 | 13.7 | all 3 fail |
| win 0.5, line 0.5 | 1.17–1.21 | 1.87 | 1.92/2.29 | 1.10/1.20 | 1.37/1.94 | 15.1 | all 3 fail |
| win 0.5, dribble −0.5 | 1.08–1.07 | 1.73 | 1.54/2.48 | 0.99/1.20 | 1.05/1.90 | 14.5 | all 3 fail |
| win 0.5, dribble −0.5, hold 0.5, line 0.5 | 1.01–1.04 | 1.61 | 1.59/2.34 | 0.88/1.06 | 1.10/1.79 | 14.8 | all 3 fail |
| win 0.5, foul 0.05, dribble −0.5, hold 0.5 | 1.13–1.04 | 1.80 | 1.61/2.23 | 1.01/1.11 | 1.12/1.57 | 14.0 | all 3 fail |
| win 0.5, dribble −0.75, hold 0.75 | 0.99–0.98 | 1.59 | 1.53/2.64 | 0.89/0.98 | 1.22/1.74 | 13.6 | all 3 fail |
| win 0.5, dribble −1, hold 1 | 0.54–0.44 | 0.86 | 0.94/1.48 | 0.69/0.53 | 0.68/0.78 | 7.7 | centre-back passes; not realistic |

Sampling errors at `tackle_win_base` 0.5 (`balanced/w50-report.json`, band `reduced_minus_full`): keeper +0.558 (se 0.120), centre-back +0.125 (se 0.102), striker +0.421 (se 0.099). The keeper arm's `full_over_control` is 1.656 (se 0.129) against 1.6. The 4-4-1-1 and 3-4-3 pairings were not re-run on balanced fixtures, because the fixture change does not touch the formations suite.

### Unbalanced fixtures (first run, before Q-LF1)

Inner loop: `calibrate --suite red-card --seed 1 --matches 120` (the criterion's seeds and arms) and `--suite formations --pairing "4-4-1-1 v 4-4-2" --pairing "3-4-3 v 4-4-2" --seed 42 --matches 200`. Each setting was written to a copy of the content folder and passed with `--content-dir`. The full log is `implement-evidence/lone-forward/tuning-log.txt` (19 rows) and the script is `tune.py`. Figures are goals per match. RED lists full/reduced per arm and the limit is 1.6 × control home. Shots are per team per match.

| Setting | Control (home–away) | Keeper F/R | Centre-back F/R | Striker F/R | 4-4-1-1 | 3-4-3 | Shots | Verdict |
|---|---|---|---|---|---|---|---|---|
| neutral (baseline) | 0.78–2.23 | 2.06/5.82 | 0.71/2.39 | 0.95/3.84 | 5.14* | 3.55* | 23.0 | red fails; 4-4-1-1 fails |
| lone_dribble −0.3 | 0.62–2.11 | 2.26/5.37 | 0.83/2.38 | 0.97/4.05 | 4.95 | 3.66 | 22.4 | fails |
| lone_layoff 0.3 | 0.10–0.60 | 0.23/1.16 | 0.07/0.85 | 0.09/0.07 | 0.50 | 0.92 | 3.8 | collapse |
| lone_hold 0.3 | identical to neutral | | | | | | | no effect |
| lone_line_hold 0.6 | 0.78–2.23 | 1.68/5.16 | 0.72/2.40 | 1.06/4.28 | 6.04 | 4.99 | 22.2 | worse |
| tackle_win_base 0.10 | 0.86–2.03 | 1.92/5.10 | 0.88/2.23 | 1.08/3.37 | 4.53 | 3.53 | 21.2 | fails |
| tackle_win_base 0.25 | 0.62–1.88 | 2.00/4.00 | 0.68/2.00 | 1.01/2.71 | 3.59 | 3.17 | 18.3 | red fails |
| tackle_win_base 0.5 (bound) | 0.70–1.73 | 1.68/2.77 | 0.74/1.82 | 1.16/2.17 | 2.71 | 2.92 | 16.0 | red fails; pairings pass |
| win 0.5, foul_base 0.05 | 0.60–1.83 | 1.32/2.44 | 0.75/2.08 | 1.02/2.40 | 2.80 | 2.55 | 14.7 | red fails |
| win 0.5, foul_base 0.02 | 0.55–1.53 | 1.17/2.26 | 0.80/1.94 | 1.02/2.17 | 2.56 | 2.72 | 13.7 | red fails |
| dribble −1, hold 1 | 0.33–0.86 | 0.93/2.83 | 0.43/0.80 | 0.37/1.21 | 1.72 | 1.62 | 9.8 | red fails |
| dribble −2, hold 0.5, layoff 0.1 | 0.73–0.93 | 0.92/0.21 | 1.38/0.85 | 1.12/0.34 | 1.01 | 1.15 | 0.5 | collapse |
| win 0.5, dribble −0.5, hold 0.5, line 0.5 | 0.41–1.59 | 1.19/3.14 | 0.59/1.70 | 0.86/2.50 | 3.36 | 3.04 | 16.0 | red fails |
| win 0.5, foul 0.05, dribble −1, hold 1, line 1 | 0.34–0.56 | 0.73/2.32 | 0.39/0.58 | 0.45/0.78 | 2.48 | 2.39 | 8.1 | red fails |
| win 0.5, foul 0.05, dribble −1.5, hold 0.5, layoff 0.05, line 0.5 | 0.40–0.12 | 0.45/0.14 | 0.33/0.13 | 0.42/0.45 | 1.26 | 1.16 | 0.8 | collapse |
| all strong (layoff 1, hold 1, dribble −1, line 0.8, win 0.15) | 0.00–0.00 | 0.01/0.33 | 0.01/0.01 | 0.00/0.12 | 1.04 | 0.89 | 0.6 | collapse |

\* Neutral pairings over 200 matches. The 1,000-match baseline is 5.79 and 4.38 (`baseline/pairs-base`).

Club check (neutral values, home and away clubs swapped, `tune/swap`): control 2.19–0.80; keeper 2.64/3.31; centre-back 1.25/0.57; striker 1.28/1.43. The away club in the criterion is the stronger club by a factor of about 2.8. In the centre-back arm the reduced side keeps two strikers, so no lone-forward term applies, and the tackle lever alone at its bound leaves 1.82 against 0.74.

## Criterion Results

Judged on the shipped values at `07d87c2`, per the Q-LF2 answer:

- **The lone-forward formations hold: PASSES.** `every_formation_holds` (release): 4-4-1-1 against 4-4-2 3.22–0.56, 3-4-3 against 4-4-2 2.91–0.78. Limit 4.0 (`shipped/criterion-formations-discipline.txt`). The gate's formations run (1,000 matches) agrees: 3.198 and 2.951.
- **Nothing that passes now regresses: PASSES.** The eight kept rows of `every_formation_holds`: 4-4-2 1.03–0.95, 4-3-3 2.31–0.32, 4-2-3-1 2.21–1.83, 3-5-2 0.97–2.17, 4-1-4-1 0.84–0.78, 4-1-2-1-2 0.83–0.72, 5-3-2 0.43–0.83, 5-4-1 1.15–1.09. `discipline_is_realistic` (200 matches): second yellows 0.045 per match (limit 0.10), 11.0% of matches with a sending-off (limit 25%), 0 same-tick pairs.
- **A lone forward uses his team-mates: PASSES on shipped play.** `tests/lone_forward.rs:67`, 40 of 40 seeds with the shipped weights. `the_lone_terms_need_a_lone_carrier` also passes: with a forward team-mate ahead, the terms change nothing.
- **No advantage from a red card: MOVED to `keeper-and-shots`** (Q-LF2). It is not judged here. The slow test `a_sending_off_gives_no_advantage` was not re-run on the shipped values. It failed at every realistic setting on balanced fixtures (see the tables above), and it stays unchanged for `keeper-and-shots`.

## Checks Run

This revision (on the working tree that became `07d87c2`):

- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean (exit 0).
- `cargo test --workspace --all-features`: exit 0; 64 binaries, 489 passed, 0 failed, 7 ignored (`shipped/workspace-tests.txt`). Two earlier runs on the tuned values failed and led to the re-derived tests: first `sim::tests::every_play_event_names_a_player` ("the match scored no goal"); then `a_full_seeded_match_fills_every_placeholder_and_names_every_player` ("only 75 lines"), `a_resumed_match_continues_tick_for_tick` ("no applied tactics change after tick 152897") and `a_dropped_viewer_reconnects_on_the_same_port_and_the_match_goes_on` ("resumed at 1"). Logs: `shipped/workspace-tests-first-run.txt` and `shipped/workspace-tests-second-run.txt`.
- `cargo test --release -p engine --all-features --test defending --test discipline -- --include-ignored --nocapture every_formation_holds discipline_is_realistic`: exit 0; both pass (figures in Criterion Results).
- `cargo test --release -p engine --all-features --test strength --test mentality --test ai_trailing -- --include-ignored --nocapture`: exit 0. The stronger team won 158, drew 30 and lost 12 of 200; shots per match were 6.78 defensive and 12.18 attacking; a trailing AI team changed tactics in 100 of 100 scenes (`shipped/slow-strength-mentality-trailing.txt`).
- Benchmark (`engine-cli bench --seed 42 --matches 5 --json`, three drives, `shipped/bench-*.json`): 1.6014 µs per tick each time (gate 1.6964), peak memory 6.69 MB each time (gate 8.33), 277,000 ticks per match, 447–449 ms per match. This is the implement-time reading; verify owns the compare.
- Slice gate (1,000 matches each, `shipped/gate/`):
  - `--suite strength`, seeds 1, 7, 42, 99, 2026: every run passes; stronger-team win rate 0.688, 0.744, 0.699, 0.728, 0.708.
  - `--suite equal`, seeds 1, 7, 42, 99, 2026: goals per match 2.579, 2.985, 2.841, 3.003, 2.443; shots per team 11.8, 13.3, 12.9, 13.8, 11.6; fouls per team 6.9, 7.3, 7.0, 7.3, 6.8; sending-off share 0.094, 0.110, 0.113, 0.090, 0.098. Band misses 7, 8, 8, 8, 7 of 16: corners, goal kicks, goals per xG, passes, shots-on-target share, throw-ins and yellow cards on every seed, and ten-plus-goal share on seeds 7, 42 and 99.
  - Seed 42 against `baseline/equal-base` (the report diff): goals 4.092 → 2.841 (into band), shots 17.165 → 12.924 (into band), sending-off share 0.326 → 0.113 (into band), ten-plus-goal share 0.043 → 0.008, goalless share 0.047 → 0.099, goal kicks 7.926 → 5.875, corners 0.012 → 0.005. Yellow cards per team 1.821 → 0.912 left the band (1.2 to 2.6). Band misses: 10 of 16 → 8 of 16.
  - `--suite formations --seed 42`: all ten pairings with 4-4-2 are at or below 4.0 per side (highest 3.667, 4-3-3). Eight other pairings exceed 4.0, against 21 in the tuning-loop gate run (`tuning-loop/full-42.report.json`); the highest is 4-2-3-1 against itself, 8.52 (was 13.47). These pairings are not a criterion. Formations band misses are 119 of 166, against 144 of 166.
- Earlier revisions: the neutral check (6 rows, change 0), the balanced slow test at neutral values (failed, as recorded) and the first-run checks on `c3cc96f` are in `history/05-implement-lone-forward-1.md`.

## Deviations from Plan

- **Front line of `lone_forward()`** (class: implementation-detail): "within `BACK_LINE_BAND`" is read as strictly less than 4 m behind the most advanced slot. The 3-4-3 wingers stand exactly 4.0 m behind the striker, and an inclusive band would give 3-4-3 no lone forward, against the plan's own unit test.
- **Line hold at share 0** (class: implementation-detail): the step returns early at `lone_line_hold` 0, so the target is the anchor exactly, as the plan requires.
- **Balanced red-card fixtures** (class: implementation-detail; within the Q-LF1 answer): both orders of the default clubs, pooled, rather than two equal clubs.
- **Red-card criterion not judged; its slow test still fails** (class: implementation-detail; within the Q-LF2 answer): the plan's step 11 required `a_sending_off_gives_no_advantage` to pass. The product owner moved that criterion to `keeper-and-shots`, so the test stays as it is and was not re-run on the shipped values.
- **Only three of the six tuning values move** (class: implementation-detail): step 10 allowed tuning all six. `lone_layoff`, `lone_line_hold` and `foul_base` keep their values, for the measured reasons in Notes on Design Choices.
- **`Tuning::default()` differs from the older-file serde defaults** (class: implementation-detail): the existing test requires the shipped file to equal `Default`, and the plan requires an older file to load with 0.05 and 0. Both hold.
- **Four seeded tests moved to another match** (class: implementation-detail; step 12): the seed-42 90-minute match now ends 0-0 with no tactics change, and the seed-7 two-minute match has no stoppage before tick 3,000. Before and after: `every_play_event_names_a_player` seed 42 (0 goals) → seed 7 (1 goal, tick 50,267); the commentary full-match test seed 42 (75 lines, no goal; floor 80) → seed 7 (84 lines, 1 goal); `a_resumed_match_continues_tick_for_tick` and the round-trip test seed 42 (no tactics change) → seed 7 (tactics changes at ticks 220,946 and 248,667); the reconnect test seed 7 (restart only at tick 3,000) → seed 5 (restarts at 2,460 and 3,000). No floor, tolerance or assertion was weakened; each test still checks what it checked.
- **Equal gate seeds without a baseline diff** (class: implementation-detail): step 13.4 records misses "against `…/equal-base`", which is a seed-42 report. The CLI refuses a baseline with another seed ("seed 1 differs from the baseline's 42"), so seeds 1, 7, 99 and 2026 ran without `--baseline` and seed 42 ran with it.

## Anything Deferred

- The red-card criterion, on the balanced fixtures, with every limit unchanged: moved to `keeper-and-shots` by the product owner (Q-LF2).
- No `sdlc-debt:` shortcut was introduced.

## Known Risks / Caveats

- Fewer fouls and cards. At an even tackle the clean-win chance rises from 0.025 to 0.25. In the equal suite, fouls fall from 13.8 to 7.0 per team, and yellow cards from 1.82 to 0.91 per team, below the 1.2 band floor on every seed. The slice risk named this; band misses are recorded, not failed (Q-E4). Band tuning belongs to `realism-tuning`.
- The default-club match scores less. The seed-42 90-minute match now ends 0-0 (shots 2 against 14), and seeds 1, 7 and 99 end 0-0, 0-1 and 0-1. The equal suite's goals per match (2.44 to 3.00) is inside its band, so this is the default clubs' strength gap, not a league-wide drop.
- The literal lone test is broad. Any outfield carrier with no team-mate ahead is lone, including a striker who has run past his partner. `lone_layoff` stays 0 for this reason; a positive value collapsed attacking play.
- The red-card slow test fails. It is an ignored slow test, so the default test run stays green, but a run of `defending` with `--include-ignored` fails until `keeper-and-shots` fixes it.
- The per-match gate outputs (`shipped/gate/*/events` and `*/stats`) take about 3.4 GB on disk. Only the report files and logs are evidence; the folders are not staged.

## Blockers

- None open. Q-LF1 (class: intent-bearing) was answered at 2026-09-24T13:19:59Z and built at `7bd5fa5`. Q-LF2 (class: intent-bearing) was answered at 2026-09-24T14:15:39Z and built at `07d87c2`.

## Assumptions

- A1 (class: implementation-detail): the calibrate suites are the inner loop; the slow tests and the full gate are the evidence (plan A9, Q-I1).
- A2 (class: implementation-detail): a setting whose shots fall below about 10 per team per match is not realistic play in the sense of RIM-12. The shipped setting gives 11.6 to 13.8 shots per team in the equal suite and 19.3 in the inner loop.
- A3 (class: implementation-detail): scratch tuning used content copies passed by `--content-dir`. The shipped content changed only in the five fields.
- A4 (class: implementation-detail): the benchmark was read at implement time (1.6014 µs). `05c-benchmark.md` is not changed, because verify owns the compare.
- A5 (class: implementation-detail): the realism floor of A2 also applies on balanced fixtures.
- A6 (class: implementation-detail): pairings not against 4-4-2 are not criteria; they are recorded as band evidence only.
- A7 (class: implementation-detail): the smallest weights that pass the scene are the right shipped weights, because the criterion is the scene and larger weights move play further.
- A8 (class: implementation-detail): moving a seeded test to another seed is a re-derived expectation, not a widened tolerance, when the test still asserts the same property at the same floor.

## Freshness Research

- No dependency was added or upgraded. `serde` field defaults with a `deny_unknown_fields` container were already proven in this repository (`a_tuning_block_without_the_defending_values_loads_with_the_defaults`). The new older-file test proves them for the five fields, including the nested `decision` block.

## Recommended Next Stage

- **Option A (default): Verify.** `/wf verify football-manager-match-engine lone-forward`. The three kept criteria are headless engine runs; the evidence is in `implement-evidence/lone-forward/shipped/`. Consider compacting the session before verify; workflow state lives in the artifact files on disk.
- **Option B: Skip to review.** Not advised: the change moves match outcomes, and verify owns the benchmark compare.
- **Option C: Revisit plan.** Not needed: the Q-LF2 answer is built within the plan's levers and bounds.
