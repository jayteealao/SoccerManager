---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: lone-forward
status: awaiting-input
stage-number: 5
created-at: "2026-09-24T12:41:48Z"
updated-at: "2026-09-24T13:49:15Z"
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-24T13:49:15Z"
    trigger: answers-returned
    because: "the product owner answered Q-LF1 (2026-09-24T13:19:59Z): measure the red-card criterion on balanced fixtures (equal clubs or both home and away orders, pooled), with every limit unchanged and no new lever, and stop again if an arm still fails"
    changed: "the red-card suite and the slow test now play seeds 1-120 in both club orders (commit 7bd5fa5); 11 balanced tuning runs; every realistic setting still fails the keeper and striker arms, so Q-LF1 is resolved and Q-LF2 is open; status stays awaiting-input"
metric-files-changed: 23
metric-lines-added: 506
metric-lines-removed: 53
metric-deviations-from-plan: 5
metric-review-fixes-applied: 0
commit-sha: "7bd5fa59d67bd1d624d500032a1ae98835b68214"
commits:
  - "c3cc96fabcb9059bb4d79b0be7ba1450f19aabb0"
  - "7bd5fa59d67bd1d624d500032a1ae98835b68214"
has-blockers: true
open-questions:
  - "Q-LF2 On balanced fixtures the red-card criterion still fails the keeper and striker arms at every realistic in-bounds setting of both levers — AWAITING INPUT (po-answers.md)"
steering-honored:
  - "Red-card criterion fixtures (Q-LF1, 2026-09-24): the slow test and the calibrate red-card suite both play seeds 1-120 in both home and away orders of the default clubs, pooled. No limit changed and no lever was added. An arm still fails on balanced fixtures, so implement stops and reports the arms and their figures."
  - "Moved criteria and slice order (2026-09-24): the red-card criterion and the 4-4-1-1 and 3-4-3 pairings are judged here with their limits unchanged. No limit, card chance, keeper value, defending value, band or test tolerance was changed."
  - "Q-I1: targeted calibrate runs were used only as the inner loop; the slice gate was not run because the slice stopped before a setting was accepted."
  - "Output boundary: code comments, test names, docs and both commit messages use product language; the added lines were leak-checked before each commit."
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
  history: history/05-implement-lone-forward-0.md
  siblings: [05-implement-defending-and-discipline.md, 05-implement-tuning-loop.md, 05-implement-realism-bands-v2.md, 05-implement-tactics-and-ai.md]
  verify: 06-verify-lone-forward.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine lone-forward"
---

# Implement: Lone forward

## The Implementation

The slice inherited both levers from `c3cc96f`, shipped at neutral values: the lone-carrier terms, the line hold and the bounded `tackle_win_base`. The red-card criterion failed at every realistic setting. The measured cause was the fixture: the sent-off player was always on the away club, and that club scores about 2.8 times the home club at 11 against 11. The product owner answered Q-LF1 with option 2. The criterion is measured on balanced fixtures, with every limit unchanged, no new lever, and a new stop if an arm still fails.

Commit `7bd5fa5` (5 files, +49 −27) plays each seed from 1 to 120 twice, once in each home and away order of the default clubs. It does this in the calibrate red-card suite and in the slow test `a_sending_off_gives_no_advantage`. At the shipped neutral values the slow test and `calibrate --suite red-card --seed 1 --matches 240` give the same figures to 4 decimals (control 1.4833–1.5167). The balanced control is even, so club strength no longer decides the result. The tuning loop then ran 11 times on balanced fixtures: the neutral values and 10 settings of the two levers. The keeper and striker arms fail at every setting with realistic play. At `tackle_win_base` 0.5 the reduced side outscores the full side by 0.56 goals per match (sampling error 0.12) in the keeper arm, and by 0.42 (0.10) in the striker arm. In the keeper arm the full side also scores 1.66 times the control, which is above the 1.6 limit. The centre-back arm misses by 0.13 (0.10), which is inside two sampling errors. The only arm that passes at any setting is the centre-back arm at dribble −1 and hold 1, where shots fall to 7.7 per team per match. RIM-12 rules that out.

This is the stop that the product owner named in the Q-LF1 answer. Q-LF2 is open and the shipped values stay neutral. Verify has no accepted setting to judge. The top risk is that the keeper arm is not a lone-forward effect: a side with an outfield player in goal outscores a full side by more than its forwards can explain.

## Summary of Changes

- Balanced red-card fixtures (`7bd5fa5`): every seed of the red-card suite plays twice, first in the usual club order and then with home and away swapped. The slow test plays seeds 1–120 in both orders (240 matches per arm). `--seed 1 --matches 240` is the same experiment. Each club is the reduced side in half the matches. The limits and the arms are unchanged, and the full side is still the home side.
- Five bounded tuning values, all at neutral values (`c3cc96f`): `tackle_win_base` (0 to 0.5, 0.05), `lone_line_hold` (0 to 1, 0.0), and `decision.lone_layoff`, `decision.lone_hold`, `decision.lone_dribble` (−5 to 5, 0.0). Serde defaults equal the neutral values, so an older tuning file loads unchanged. `TUNING_VERSION` stays 2.
- `fouls::win_chance()` holds the clean-tackle formula once. The tackle loop and nine copies in eight test files call it (`injury.rs` keeps its 0.5 factor).
- `offside::second_last_depth()` is extracted from `offside_set()`, so the referee and the forward read one line.
- `Team::lone_forward()` gives the single active front-line slot that is not keeping goal.
- In the scored options, a lone carrier (outfield, with no active outfield team-mate ahead) gets `lone_layoff` on each pass with progress ≤ 0. When he is also pressed, he gets `lone_hold` on holding the ball and `lone_dribble` on a dribble. No random draw is added.
- Off the ball, `hold_the_line()` moves the team's lone forward from his anchor toward the line minus 0.5 m by `lone_line_hold`. He never goes past that line and keeps his `y`. At share 0 the step returns early.
- New fast scene tests in `tests/lone_forward.rs`; content pins and an older-file load test; five rows and one paragraph in `docs/reference/data-files.md`.

## Files Changed

- `crates/engine-cli/src/calibrate/fixtures.rs` (`7bd5fa5`): `RedCardFixture::swapped`; each seed plays twice, the second time swapped; the fixture test checks seeds 1–120 in both orders.
- `crates/engine-cli/src/calibrate/worker.rs` (`7bd5fa5`): a swapped red-card match loads the default clubs in the other order.
- `crates/engine/tests/defending.rs` (`7bd5fa5`): `a_sending_off_gives_no_advantage` plays seeds 1–120 in both club orders.
- `docs/how-to/calibration.md`, `docs/reference/cli.md` (`7bd5fa5`): the red-card experiment plays both orders; `--matches 240` gives the slow test's experiment.
- `crates/engine/src/tuning.rs`: the five fields with garde ranges, serde defaults and `Default` values.
- `content/tuning.json`: the five neutral values.
- `crates/engine/src/rules/fouls.rs`: `win_chance()` plus two unit tests (equal to the earlier formula at 0.05; linear in the base).
- `crates/engine/src/sim.rs`: the tackle loop calls `win_chance()`.
- `crates/engine/src/rules/offside.rs`: `second_last_depth()`; `offside_set()` calls it.
- `crates/engine/src/team.rs`: `lone_forward()` plus two unit tests.
- `crates/engine/src/decision.rs`: the lone test, the three terms in `options()`, `hold_the_line()` and `ONSIDE_MARGIN`.
- `crates/engine/tests/lone_forward.rs` (new): four scene tests.
- `crates/engine/tests/content.rs`: a pin test and an older-file load test.
- `crates/engine/tests/{acting_keeper,discipline,injury,plugin_hooks,rules_cards,rules_fouls,rules_restarts,tactics_queue}.rs`: the copied 0.05 formula becomes `win_chance()`.
- `docs/reference/data-files.md`: rows for the five values and a paragraph on the lone carrier, the lone forward and the tackle chance.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/decision.rs`, `team.rs`, `sim.rs`, `tuning.rs`, `content/tuning.json`: also changed by `defending-and-discipline` (cover, press, acting keeper). Its mechanisms are not changed here. The defending tests pass unchanged.
- `docs/reference/data-files.md`: also changed by `defending-and-discipline` and `realism-bands-v2`.

## Notes on Design Choices

- Every new term is added as exact zero at the neutral values and draws nothing. So the neutral check can prove "the seams change nothing" bit for bit (red-card diff: 6 rows, change 0).
- The line hold returns early at share 0. The plan's "never past the line" cap would otherwise move a forward whose anchor is past the line, even at share 0.
- The scene tests that need the lone terms switched on set their own weights (lay-off 0.3, hold 1.0, dribble −1.0). The shipped weights are 0 until a setting passes every match-outcome check.

## Verification Seams Built

- "A lone forward uses his team-mates" → `tests/lone_forward.rs:67` `a_pressed_lone_forward_passes_or_holds_instead_of_dribbling`, 40 seeds on the real engine (enables `cargo test --test lone_forward`), and `:87` `the_lone_terms_need_a_lone_carrier`. The seams are `decision.rs:264` (lone test), `:325` (lay-off) and `:348` (pressed terms). The test sets its own weights, because the shipped weights are neutral (see Deviations).
- Line hold → `team.rs:280` `lone_forward()`, `offside.rs:35` `second_last_depth()`, `decision.rs:119` `hold_the_line()`, observed by `tests/lone_forward.rs:109`.
- Tackle odds → `fouls.rs:75` `win_chance()`, called at `sim.rs:1015`, observed by `tests/lone_forward.rs:171` (a draw under the win chance wins the ball; a draw in the foul band is a foul).
- Tuning seams → `tuning.rs:166`, `:171`, `:273` and the following fields, pinned at `tests/content.rs:178`, with the older-file load at `:190`.
- "No advantage from a red card" → balanced fixtures at `crates/engine/tests/defending.rs:206` (`a_sending_off_gives_no_advantage`; club order at `:213`, 240 matches per arm at `:224` and `:228`) and `crates/engine-cli/src/calibrate/fixtures.rs:118` (`red_card_fixtures`, swap at `:127`), loaded in order at `worker.rs:90`. This enables `cargo test --release … a_sending_off_gives_no_advantage` and `calibrate --suite red-card --seed 1 --matches 240`, which give the same figures (checked this run at neutral values). The fixture test is at `fixtures.rs:315`.
- "The lone-forward formations hold", "Nothing that passes now regresses": nothing new was needed. `every_formation_holds` and `discipline_is_realistic` exist unchanged.

## Tuning Loop (step 10)

### Balanced fixtures (after Q-LF1)

Inner loop: `calibrate --suite red-card --seed 1 --matches 240` (seeds 1–120 in both club orders, the slow test's experiment). Each setting was written to a copy of the content folder and passed with `--content-dir` (`implement-evidence/lone-forward/tune2.py`). The log is `balanced/tuning-log.txt` (11 rows), and each run's `report.json` is in `balanced/`. Figures are goals per match: full/reduced per arm. The limit is 1.6 × control home. Shots are per team per match over the four arms.

| Setting | Control (home–away) | Limit | Keeper F/R | Centre-back F/R | Striker F/R | Shots | Verdict |
|---|---|---|---|---|---|---|---|
| neutral (shipped) | 1.48–1.52 | 2.37 | 2.35/4.56 | 0.98/1.48 | 1.12/2.63 | 20.0 | all 3 arms fail |
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

Sampling errors at `tackle_win_base` 0.5 (`balanced/w50-report.json`, band `reduced_minus_full`): keeper +0.558 (se 0.120), centre-back +0.125 (se 0.102), striker +0.421 (se 0.099). The keeper arm's `full_over_control` is 1.656 (se 0.129) against 1.6. The 4-4-1-1 and 3-4-3 pairings were not re-run in this revision, because the fixture change does not touch the formations suite. Their figures below come from the first run.

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

- **No advantage from a red card: FAILS on balanced fixtures at every realistic in-bounds setting.** The keeper and striker arms fail at every setting with 13.6 or more shots per team per match. The best is `tackle_win_base` 0.5: keeper 1.90 against 2.46, striker 1.25 against 1.67, centre-back 1.09 against 1.21. The only pass on any arm is the centre-back arm at 7.7 shots per team, which RIM-12 rules out. The slow test was run this revision at the shipped neutral values only (it fails, with the figures in Checks Run). It was not run on a tuned candidate, because no candidate passes the suite on the same fixtures.
- **The lone-forward formations hold: PASSES in the inner loop** at `tackle_win_base` 0.5 (2.71 and 2.92 over 200 matches, first run). It is not confirmed by `every_formation_holds`. No setting is shipped.
- **Nothing that passes now regresses: HOLDS at the shipped neutral values.** The workspace tests pass this run (489 passed). The fixture change touches only the red-card experiment. It is not measured on a tuned setting.
- **A lone forward uses his team-mates: PASSES as a mechanism** (`tests/lone_forward.rs:67`, 40 of 40 seeds with the weights switched on). The shipped weights are 0, so shipped play does not yet show it.

## Checks Run

This revision (on the working tree that became `7bd5fa5`):

- `cargo fmt --all -- --check`: clean after one `cargo fmt --all` (it re-wrapped one `if` in `defending.rs`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean, after one fix (`manual_is_multiple_of`, at two sites).
- `cargo test --workspace --all-features`: exit 0; 64 binaries, 489 passed, 0 failed, 7 ignored.
- `cargo test --release -p engine --all-features --test defending -- --include-ignored --nocapture a_sending_off_gives_no_advantage` at the shipped neutral values: FAILED as expected, in 55.62 s (`balanced/slow-test-neutral.txt`). Control home 1.4833 away 1.5167; keeper 2.3500/4.5625; centre-back 0.9792/1.4833; striker 1.1167/2.6333; limit 2.3733. These are the figures of `calibrate --suite red-card --seed 1 --matches 240` at neutral (`balanced/neutral-report.json`).

First run (on `c3cc96f`):

- `cargo test -p engine --all-features` at neutral values, before the scene tests: 31 binaries, 272 passed, 0 failed, 6 ignored (266 at HEAD plus 6 new unit and content tests; no expectation changed).
- `cargo test --workspace --all-features` at neutral values, before the scene tests: 63 binaries, 485 passed, 0 failed, 7 ignored. After the scene tests: 64 binaries, 489 passed, 0 failed, 7 ignored.
- `cargo test -p engine --all-features --test lone_forward`: 4 passed. Before the scene tests set their own weights, the pressed scene failed at neutral values (seed 1: the carrier dribbled to (38, 4)), so the test tells the two cases apart.
- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- Neutral check: `calibrate --suite red-card --seed 1 --matches 120 --baseline baseline/red-base/report.json`. Header `content 3b3ddb56d4ec -> e59860b1bee6, content changed` (the five neutral fields); 6 rows, change +0.0000 each (`neutral/red.log`).
- Benchmark (`engine-cli bench --seed 42 --matches 5 --json`, three drives, `bench/`): 1.5531, 1.5312, 1.5422 µs per tick (median 1.5422 against the 1.6964 gate); peak memory 6.699, 6.852, 6.707 MB (median 6.707 against 8.33); 450.0 ms per match at the median; 291,800 ticks per match. This is the implement-time reading. The formal compare belongs to verify.
- Not run: the slow criterion tests on a tuned setting, re-derivation of moved expectations (step 12; nothing moved at neutral values), and the full slice gate (step 13.4). All three wait for a setting that could be accepted.

## Deviations from Plan

- **Front line of `lone_forward()`** (class: implementation-detail): "within `BACK_LINE_BAND`" is read as strictly less than 4 m behind the most advanced slot. The 3-4-3 wingers stand exactly 4.0 m behind the striker (62 against 66). An inclusive band would give 3-4-3 three front-line players and no lone forward, which contradicts the plan's own unit test ("3-4-3 gives the striker"). The same rule gives 4-3-3 its centre forward.
- **Line hold at share 0** (class: implementation-detail): the step returns early at `lone_line_hold` 0, so the target is the anchor exactly, as the plan requires. Otherwise the cap would pull back an anchor that is past the line.
- **Scene tests set their own lone weights** (class: implementation-detail): the plan said "with tuned values read from the content". No tuned setting was accepted, so the shipped weights stay 0. The scenes set lay-off 0.3, hold 1.0 and dribble −1.0 to test the mechanism.
- **Steps 11 to 13.4 stopped** (plan-defined stop): step 10 found no in-bounds setting that passes the red-card criterion with realistic play. Step 11 says to stop and report. The equal-suite baseline had to be run again on a clean copy of the committed content (`--content-dir`), because the first run started after `content/tuning.json` had gained the new fields and was refused (`equal-base.log`, then rerun).
- **Balanced red-card fixtures** (class: implementation-detail; within the Q-LF1 answer): the product owner allowed either two equal clubs or both orders of the default clubs, pooled. This run chose both orders: each seed plays twice in a row, the second time swapped. The reasons: the default clubs stay the clubs of the experiment, a paired design cancels club strength seed by seed, and the suite keeps `--matches` as matches per arm with no change to the report code. The slow test's experiment is now `--matches 240`, not 120.

## Anything Deferred

- A tuned setting for the five values, the slow criterion runs on it, the moved expectations, the benchmark compare on it and the slice gate. These wait until the product owner answers Q-LF2.
- No `sdlc-debt:` shortcut was introduced.

## Known Risks / Caveats

- The shipped values are neutral. The committed code changes no match outcome yet, so 4-4-1-1 and 3-4-3 still exceed 4.0 in shipped play.
- The literal lone test is broad. Any outfield carrier with no team-mate ahead is lone, which includes a striker who has run past his partner. A positive `lone_layoff` collapses attacking play in every formation (0.3 gives 3.8 shots per team). The lay-off lever needs a narrower definition before it can be used (see Q-LF1, option 3).
- `lone_hold` up to 0.3 had no measured effect, because holding the ball rarely outscores a pressed dribble.

## Blockers

- **Q-LF1 (class: intent-bearing): RESOLVED** by the product owner at 2026-09-24T13:19:59Z (option 2, balanced fixtures). It is built at `7bd5fa5`.
- **Q-LF2 (class: intent-bearing, AWAITING INPUT; `po-answers.md`).** On balanced fixtures, the keeper and striker arms still fail at every realistic in-bounds setting of both levers. The product owner said to stop and report in this case. Measured on seeds 1–120 in both orders:
  - Keeper arm: at best (`tackle_win_base` 0.5) the reduced side scores 2.46 against 1.90 (+0.56, se 0.12). The full side's 1.90 is 1.66 times the control (limit 1.6).
  - Striker arm: 1.67 against 1.25 (+0.42, se 0.10).
  - Centre-back arm: 1.21 against 1.09 (+0.13, se 0.10, inside two sampling errors).
  - The only pass is the centre-back arm at dribble −1 and hold 1, with 7.7 shots per team per match.

  Options:
  1. Allow a third lever that acts on the side with ten men, for example less pressing and running. The Q-LF1 answer said not to add a lever, so only the product owner can change this.
  2. Move the red-card criterion to `keeper-and-shots`, which owns the save model, because the keeper arm is the worst arm. Ship the formations fix here (`tackle_win_base` 0.5 passed both pairings in the inner loop) and judge this slice on the other three criteria.
  3. Judge each arm against a relative target (the reduced side scores less than it scores at 11 against 11) instead of against the full side.
  4. Keep the criterion and the two levers, and allow a setting below the shot floor. RIM-12 rules this out today.

  Each option changes a criterion, a lever set or the slice scope, so this run may not choose one.

## Assumptions

- A1 (class: implementation-detail): the calibrate red-card suite on seeds 1–120 is the inner loop and the evidence for the stop, because it plays the criterion test's seeds and arms (plan A9).
- A2 (class: implementation-detail): a setting whose shots fall below about 10 per team per match is not realistic play in the sense of RIM-12, so a pass reached there does not count. The baseline is about 23 shots, and the bands in `realism-bands.json` assume shot totals near real football.
- A3 (class: implementation-detail): the scratch tuning used copies of the content folder passed by `--content-dir`. The shipped content was changed only by the five neutral fields.
- A4 (class: implementation-detail): the benchmark was read at implement time on the neutral build (median 1.5422 µs). `05c-benchmark.md` is not changed, because verify owns the compare.
- A5 (class: implementation-detail): the realism floor of A2 (about 10 shots per team per match) also applies on balanced fixtures. The balanced neutral figure is 20.0 shots per team over the four arms.
- A6 (class: implementation-detail): the formations pairings were not re-run in this revision. The fixture change touches only the red-card suite and the red-card slow test, and no tuning value changed in the shipped content.

## Freshness Research

- No dependency was added or upgraded. `serde` field defaults with a `deny_unknown_fields` container were already proven in this repository (`a_tuning_block_without_the_defending_values_loads_with_the_defaults`). The new older-file test proves them for the five fields, including the nested `decision` block.

## Recommended Next Stage

- **Option D (default): Blocked.** Q-LF2 needs a product-owner answer. Run `/wf implement football-manager-match-engine lone-forward` again after the answer is in `po-answers.md`.
- **Option C: Revisit plan.** Run `/wf plan football-manager-match-engine lone-forward` if the answer adds a lever or moves the criterion.
- **Option A: Verify.** `/wf verify football-manager-match-engine lone-forward` cannot run yet. The shipped values are neutral and two criteria are not met.
