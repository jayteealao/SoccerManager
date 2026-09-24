---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: lone-forward
status: awaiting-input
stage-number: 5
created-at: "2026-09-24T12:41:48Z"
updated-at: "2026-09-24T12:41:48Z"
metric-files-changed: 18
metric-lines-added: 457
metric-lines-removed: 26
metric-deviations-from-plan: 4
metric-review-fixes-applied: 0
commit-sha: "c3cc96fabcb9059bb4d79b0be7ba1450f19aabb0"
commits:
  - "c3cc96fabcb9059bb4d79b0be7ba1450f19aabb0"
has-blockers: true
open-questions:
  - "Q-LF1 The red-card criterion fails at every in-bounds setting of both levers that keeps play realistic — AWAITING INPUT (po-answers.md)"
steering-honored:
  - "Moved criteria and slice order (2026-09-24): the red-card criterion and the 4-4-1-1 and 3-4-3 pairings are judged here with their limits unchanged. No limit, card chance, keeper value, defending value, band or test tolerance was changed."
  - "Q-I1: targeted calibrate runs were used only as the inner loop; the slice gate was not run because the slice stopped before a setting was accepted."
  - "Output boundary: code comments, test names, docs and the commit message use product language; the added lines were leak-checked before the commit."
  - "Design direction: not applicable; no page changed."
tags: [engine, tactics, rules, realism]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-lone-forward.md
  plan: 04-plan-lone-forward.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/lone-forward/
  siblings: [05-implement-defending-and-discipline.md, 05-implement-tuning-loop.md, 05-implement-realism-bands-v2.md, 05-implement-tactics-and-ai.md]
  verify: 06-verify-lone-forward.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine lone-forward"
---

# Implement: Lone forward

## The Implementation

The slice started from `a447ff1` with two criteria that `defending-and-discipline` could not pass. With one away player sent off, the reduced side outscored the full side in all three arms, and 4-4-1-1 and 3-4-3 scored above 4.0 against 4-4-2. The plan allowed two levers only: the decisions of a carrier with no team-mate ahead, and the odds of a won tackle against a foul.

Both levers are built and committed at `c3cc96f` (18 files, +457 −26). A lone carrier gets three weighted terms: a lay-off bonus, and, when pressed, a hold-up bonus and a dribble term. A side's single forward can hold the onside line, read from the referee's own line (`second_last_depth()`). The clean-tackle chance is the bounded `tackle_win_base`. All five values ship at neutral values. At those values the engine gives the same figures as before: 485 of 485 workspace tests passed before the new scene tests, and the red-card diff against the baseline showed a change of 0 on all 6 rows. The 4 new scene tests then brought the total to 489. The tuning loop then made 19 runs (17 settings, a neutral 200-match pairing reference and a club swap) through the red-card suite on seeds 1–120 and through the two pairings. The pairings pass with the tackle lever alone (`tackle_win_base` 0.5: 4-4-1-1 2.71, 3-4-3 2.92 over 200 matches). The red-card criterion does not pass at any setting that keeps play realistic. It passes two of three arms only when shots fall below one per team per match, which RIM-12 rules out.

The measured cause lies outside both levers. With the home and away clubs swapped, the control changes from 0.78–2.23 to 2.19–0.80. So the away club (Eldstead City) scores about 2.8 times the home club at 11 against 11 on the same seeds. In the centre-back arm the away side keeps both strikers and has no lone forward. There the reduced side still outscores the full side at every tackle setting (best 1.70 against 0.59). This is the plan's stop condition (step 11). No criterion limit, card chance or band was changed. The shipped values stay neutral until the product owner answers Q-LF1. Implement cannot continue, verify has nothing to judge, and the top risk is that the red-card criterion cannot be met with the default clubs.

## Summary of Changes

- Five bounded tuning values, all at neutral values: `tackle_win_base` (0 to 0.5, 0.05), `lone_line_hold` (0 to 1, 0.0), and `decision.lone_layoff`, `decision.lone_hold`, `decision.lone_dribble` (−5 to 5, 0.0). Serde defaults equal the neutral values, so an older tuning file loads unchanged. `TUNING_VERSION` stays 2.
- `fouls::win_chance()` holds the clean-tackle formula once. The tackle loop and nine copies in eight test files call it (`injury.rs` keeps its 0.5 factor).
- `offside::second_last_depth()` is extracted from `offside_set()`, so the referee and the forward read one line.
- `Team::lone_forward()` gives the single active front-line slot that is not keeping goal.
- In the scored options, a lone carrier (outfield, with no active outfield team-mate ahead) gets `lone_layoff` on each pass with progress ≤ 0. When he is also pressed, he gets `lone_hold` on holding the ball and `lone_dribble` on a dribble. No random draw is added.
- Off the ball, `hold_the_line()` moves the team's lone forward from his anchor toward the line minus 0.5 m by `lone_line_hold`. He never goes past that line and keeps his `y`. At share 0 the step returns early.
- New fast scene tests in `tests/lone_forward.rs`; content pins and an older-file load test; five rows and one paragraph in `docs/reference/data-files.md`.

## Files Changed

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
- "No advantage from a red card", "The lone-forward formations hold", "Nothing that passes now regresses": nothing new was needed. `a_sending_off_gives_no_advantage`, `every_formation_holds` and `discipline_is_realistic` exist unchanged, and the calibrate red-card suite uses the same seeds and arms.

## Tuning Loop (step 10)

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

- **No advantage from a red card: FAILS at every realistic in-bounds setting** (table above). The criterion passes 2 of 3 arms only where play collapses (0.5 to 0.8 shots per team per match, against about 23 at the baseline), and RIM-12 forbids buying the criterion with unrealistic play. The slow test `a_sending_off_gives_no_advantage` was not run on a candidate, because no candidate passes the calibrate red-card suite on the same seeds and arms.
- **The lone-forward formations hold: PASSES in the inner loop** at `tackle_win_base` 0.5 (2.71 and 2.92 over 200 matches), but it is not confirmed by `every_formation_holds`. No setting is shipped.
- **Nothing that passes now regresses: HOLDS at the shipped neutral values** (behaviour is unchanged; the red-card diff shows 0 on all 6 rows; the workspace tests pass). It is not measured on a tuned setting.
- **A lone forward uses his team-mates: PASSES as a mechanism** (`tests/lone_forward.rs:67`, 40 of 40 seeds with the weights switched on). The shipped weights are 0, so shipped play does not yet show it.

## Checks Run

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

## Anything Deferred

- A tuned setting for the five values, the slow criterion runs on it, the moved expectations, and the slice gate: deferred to after the product owner answers Q-LF1.
- No `sdlc-debt:` shortcut was introduced.

## Known Risks / Caveats

- The shipped values are neutral. The committed code changes no match outcome yet, so 4-4-1-1 and 3-4-3 still exceed 4.0 in shipped play.
- The literal lone test is broad. Any outfield carrier with no team-mate ahead is lone, which includes a striker who has run past his partner. A positive `lone_layoff` collapses attacking play in every formation (0.3 gives 3.8 shots per team). The lay-off lever needs a narrower definition before it can be used (see Q-LF1, option 3).
- `lone_hold` up to 0.3 had no measured effect, because holding the ball rarely outscores a pressed dribble.

## Blockers

- **Q-LF1 (class: intent-bearing, AWAITING INPUT; `po-answers.md`).** The red-card criterion cannot be met with the two levers the product owner allowed unless play collapses. The criterion is measured with the stronger club as the away side, and that club scores about 2.8 times the home club at 11 against 11. Options:
  1. Keep the criterion and add a lever. For example, reduce the reduced side's pressing and running with ten men, which is not in RIM-12's two levers.
  2. Measure the criterion on equal clubs, or on both home and away orders of the default clubs, with the limits unchanged.
  3. Narrow "lone" to the team's structural lone forward (`Team::lone_forward()`), ship `tackle_win_base` near 0.5 for the pairings, and judge the red-card arms against a relative target (the reduced side scores less than its own control).
  4. Ship the formations fix alone (`tackle_win_base` 0.5 passes both pairings in the inner loop) and move the red-card criterion to a later slice.

  Each option changes a criterion, a lever set or the slice scope, so this run may not choose one.

## Assumptions

- A1 (class: implementation-detail): the calibrate red-card suite on seeds 1–120 is the inner loop and the evidence for the stop, because it plays the criterion test's seeds and arms (plan A9).
- A2 (class: implementation-detail): a setting whose shots fall below about 10 per team per match is not realistic play in the sense of RIM-12, so a pass reached there does not count. The baseline is about 23 shots, and the bands in `realism-bands.json` assume shot totals near real football.
- A3 (class: implementation-detail): the scratch tuning used copies of the content folder passed by `--content-dir`. The shipped content was changed only by the five neutral fields.
- A4 (class: implementation-detail): the benchmark was read at implement time on the neutral build (median 1.5422 µs). `05c-benchmark.md` is not changed, because verify owns the compare.

## Freshness Research

- No dependency was added or upgraded. `serde` field defaults with a `deny_unknown_fields` container were already proven in this repository (`a_tuning_block_without_the_defending_values_loads_with_the_defaults`). The new older-file test proves them for the five fields, including the nested `decision` block.

## Recommended Next Stage

- **Option D (default): Blocked.** Q-LF1 needs a product-owner answer. Re-run `/wf implement football-manager-match-engine lone-forward` once it is recorded in `po-answers.md`.
- **Option C: Revisit plan.** `/wf plan football-manager-match-engine lone-forward`, if the answer adds a lever or changes a criterion.
- **Option A: Verify.** `/wf verify football-manager-match-engine lone-forward` is not viable yet: the shipped values are neutral and two criteria are unmet.
