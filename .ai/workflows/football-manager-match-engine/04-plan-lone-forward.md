---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: lone-forward
status: complete
stage-number: 4
created-at: "2026-09-24T11:53:45Z"
updated-at: "2026-09-24T11:53:45Z"
metric-files-to-touch: 17
metric-step-count: 13
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, tactics, rules, realism]
stack-source: confirmed
open-questions: []
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-lone-forward.md
  siblings: [04-plan-defending-and-discipline.md, 04-plan-tuning-loop.md, 04-plan-realism-bands-v2.md, 04-plan-tactics-and-ai.md]
  benchmark: 05c-benchmark.md
  implement: 05-implement-lone-forward.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine lone-forward"
---

# Plan: Lone Forward

## The Plan

`defending-and-discipline` built goal-side cover, the new press and the acting keeper at `f7fe35b`, but two criteria failed at every in-bounds setting of its five tuning values. With one player sent off, the reduced side outscores the full side in all three arms (keeper 5.82 against 2.06, centre-back 2.39 against 0.71, striker 3.84 against 0.95). 4-4-1-1 scores 5.18 and 3-4-3 scores 4.17 against 4-4-2, above the 4.0 limit. The measured cause is a carrier with no forward team-mate. His pass options all score a negative progress term (`decision.rs:283-292`), so a dribble and a shot win: 40.6 shots against 20.8 in the control and no offsides. Contact also floods fouls, because the tackle loop draws a fixed 5% win share against a 10% foul chance on every tick in reach (`sim.rs:1017-1018`).

The plan uses only the two levers the product owner allowed (RIM-12). The first lever is the lone forward's decisions. A carrier is lone when no active outfield team-mate is ahead of him. He then gets three weighted terms in the existing scored options: a lay-off bonus on passes to team-mates at or behind him, and, when he is pressed, a hold-up bonus and a dribble cost. He is never forced into a choice. Off the ball, the team's lone forward moves toward the second-last defender's line and stays onside, from the same line the referee uses. The second lever is the tackle odds. The 0.05 literal becomes a bounded `tackle_win_base`, and the existing `foul_base` can move inside its bounds. The card chances, the keeper model and every limit stay fixed. The five new values first land at neutral values, which give a zero-change red-card diff. Only then does the tuning loop move them, with the red-card suite and targeted pairings as the inner loop.

The work is 13 steps over 17 files. No snapshot, schema or protocol change is needed, because the lone state is derived each tick. The benchmark is re-baselined on `a447ff1` at 1.5422 µs per tick (gate 1.6964 µs). When it lands, `keeper-and-shots` can plan on a red-card experiment that no longer rewards the reduced side. The top risk is that both levers at their bounds still leave an arm or a pairing failing. In that case implement stops and reports the figures, and it never raises a limit.

## Current State

- Code: HEAD `a447ff1`. `git status` shows no change under `crates` or `content`. This run's `cargo test -p engine --all-features` gave 266 passed, 0 failed and 6 ignored over 31 test binaries (exit 0).
- **Carrier options** (`decision.rs:197-333`): shot, the best pass, dribble, clear and hold are each a weighted sum plus the plan offsets and noise. The pass score is `(progress + plan + role) × progress` with progress from −1 to 1 over 40 m (`:283-285`), so a backward pass costs up to 0.7 (plus the plan offsets). Dribble is `dribble_base + dribble_space × space − pressure-if-pressed + first_touch + skill` (`:303-309`), and "pressed" means an opponent within 2.5 m (`:232`). Hold is `hold + plan.hold − tempo − hold_per_s × seconds held` (`:310-311`). Nothing reads whether a team-mate is ahead.
- **Choice** (`decision.rs:335-414`): the highest score wins. Hold sets the carrier's target to his position (`:390-393`). Dribble runs 8 m toward goal, drifting away from the nearest opponent (`:394-412`).
- **Off the ball** (`decision.rs:26-76`, `team.rs:351-367`): with a carrier, every player targets its anchor. The anchor is the formation slot moved by block depth, duty and width, plus 0.3 of the ball offset. No anchor reads the offside line.
- **Offside** (`rules/offside.rs:15-44`): `offside_set` computes the second-last opponent's depth when a team-mate plays the ball, and flags a team-mate who is past it, past the ball and in the opponents' half.
- **Tackle** (`sim.rs:1003-1034`): once the carrier's control cooldown has passed, every opponent within `reach_radius` (1.0 m), off his foul cooldown, draws once per tick. `p_win = 0.05 × tackling / (tackling + dribbling)` (`:1017`) and `p_foul = foul_chance(...)` (`:1018`, `foul_base` 0.1 raised by aggression, lowered by tackling, times the booked factor). `tackle_outcome` splits one draw (`rules/fouls.rs:61-71`). The 0.05 literal is copied into eight test files (`acting_keeper.rs:151`, `discipline.rs:33`, `injury.rs:26`, `plugin_hooks.rs:158`, `rules_cards.rs:25`, `rules_fouls.rs:25`, `rules_restarts.rs:90,138`, `tactics_queue.rs:101`).
- **Tuning** (`tuning.rs`): `DecisionWeights` (`:203-260`) and `Tuning` both use `deny_unknown_fields` with garde ranges, and the defending values use `#[serde(default = …)]` (`:156-161`). `foul_base` has bounds 0 to 1 (`:138-139`). `TUNING_VERSION = 2` (`data/tuning.rs:17`).
- **Criterion tests** (from `defending-and-discipline`, unchanged): `a_sending_off_gives_no_advantage` (`tests/defending.rs:204-238`, seeds 1–120, cards off, arms 11, 13 and 21, limit 1.6 × control home), `every_formation_holds` (`:240-268`, 10 formations against 4-4-2, 120 seeds, home on odd seeds, cards on, 4.0 per side), `discipline_is_realistic` (`tests/discipline.rs:138-181`, 200 matches). They print every figure before they assert.
- **Inner loop** (from `tuning-loop`): `engine-cli calibrate --suite red-card --seed 1 --matches 120 --baseline …` (about 42 s on the reference machine), `--suite formations --pairing "4-4-1-1 v 4-4-2" --seed 42 --matches 1000 --baseline …` (about 87 s), and the diff marks a change inside two sampling errors as noise (`docs/how-to/calibration.md`).
- **Measured at `f7fe35b`** (`05-implement-defending-and-discipline.md`, Criterion Results): control 0.78–2.23; keeper arm full 2.06, reduced 5.82 (limit 1.24 for the full side); centre-back 0.71 against 2.39; striker 0.95 against 3.84; 4-4-1-1 5.18–1.00; 3-4-3 4.17–1.26; the eight kept pairings pass; discipline 0.085 second yellows per match, 16.5% with a sending-off, 0 same-tick pairs.

## Simplicity Ladder

- Lone-carrier test → rung 3 reuse with modification — `decision.rs` → `options()` already walks the players for pressure and passes. The test is one more pass over the carrier's team-mates with no allocation. There is no in-repo "team-mate ahead" helper.
- Lay-off, hold-up and dribble terms → rung 3 reuse — the scored-options layer (`options()`), with three new `DecisionWeights` fields. No new choice kind; Hold and Pass already exist.
- The team's lone forward off the ball → rung 3 reuse with modification — `team.rs` → the `back_line()` band pattern (`BACK_LINE_BAND`, `:264-274`), mirrored for the front line as `lone_forward()`.
- Offside line → rung 3 extract into a shared utility — `rules/offside.rs` → the second-last depth loop inside `offside_set()` becomes `second_last_depth()`, so the referee and the forward read one line.
- Tackle win chance → rung 3 extract into a shared utility — the literal in `sim.rs:1017` and its eight test copies become `fouls::win_chance()`, next to `foul_chance()`.
- Bounded tuning values with defaults → rung 3 reuse — garde `range` plus serde `default`, as the five defending values did (`tuning.rs:156-161`).
- Measurement → rung 3 reuse — the `tuning-loop` red-card suite, `--pairing` and `--baseline`, and the three slow criterion tests.

## Applied Learnings

No applicable learnings found. `.ai/solutions/INDEX.md` does not exist, and `.ai/sdlc-config.json` sets no global directory.

Repeat-deferral tripwire: `00-index.md` holds two deferrals, the human legibility reading (viewer-match-day) and the macOS build (distribution). This slice is verified by headless engine runs with `cargo test` and `engine-cli calibrate`. It names neither wall, so the tripwire does not fire.

## Likely Files / Areas to Touch

- `crates/engine/src/tuning.rs`: `lone_hold`, `lone_layoff` and `lone_dribble` in `DecisionWeights`; `tackle_win_base` and `lone_line_hold` in `Tuning`; bounds, defaults and `Default`.
- `content/tuning.json`: the five values.
- `crates/engine/src/rules/fouls.rs`: `win_chance()` and unit tests.
- `crates/engine/src/sim.rs`: the tackle loop calls `win_chance()`.
- `crates/engine/src/rules/offside.rs`: `second_last_depth()`.
- `crates/engine/src/team.rs`: `lone_forward()` and unit tests.
- `crates/engine/src/decision.rs`: the lone-carrier terms in `options()`, and the line hold in `decide()`.
- `crates/engine/tests/lone_forward.rs` (new): the regression scenes.
- `crates/engine/tests/content.rs`: pins and the older-file load.
- `crates/engine/tests/acting_keeper.rs`, `discipline.rs`, `injury.rs`, `plugin_hooks.rs`, `rules_cards.rs`, `rules_fouls.rs`, `rules_restarts.rs`, `tactics_queue.rs`: the copied formula becomes `win_chance()`.
- `docs/reference/data-files.md`: five tuning rows and one paragraph.
- Pinned tests that may move and are re-derived: `full_match.rs`, `strength.rs`, `mentality.rs`, `ai_trailing.rs`, `commentary.rs` (line floor), `sim.rs` (stoppage floor), `crates/engine-cli/tests/stream_cli.rs` (frame count).

## Proposed Change Strategy

- **Two levers, nothing else (RIM-12).** The decision terms and the tackle odds are the only moving parts. The card chances (`yellow_*`, `red_base`), the keeper values, the five defending values, and every limit and band stay as they are.
- **Lone is literal.** A carrier is lone when no active outfield team-mate has a greater depth along his attack direction. This is the slice's words ("the carrier has no forward team-mate"), and it covers both the striker left alone after a send-off and the single forward of 4-4-1-1 and 3-4-3.
- **Weighted terms, not a script.** When lone:
  - `lone_layoff` is added to each pass to a team-mate at or behind him. The progress cost is unchanged, so the best lay-off still prefers the most useful one.
  - When also pressed (the existing 2.5 m test), `lone_hold` is added to hold and `lone_dribble` to dribble.
  - Noise still applies and hold still decays with `hold_per_s`. So a forward in space can still run at goal, and a held ball is released after a few seconds.
  - No term is added to the shot or the clearance. Fewer dribbles into pressure means fewer shots from them.
- **Off the ball, on the line.** While his team holds the ball and he is not the carrier, `Team::lone_forward()` gives the team's single front-line player. His target depth moves toward `second_last_depth() − 0.5 m` by the share `lone_line_hold`, and it is never past that line. This gives the midfield a forward option he can lay off from, and it keeps him onside.
- **Tackle odds.** `win_chance = tackle_win_base × tackling / (tackling + dribbling)`. The neutral value 0.05 reproduces today's figure exactly. `foul_base` (0 to 1) is the foul side of the same odds and may move inside its bounds. The draw order does not change.
- **Neutral first.** The five values land at neutral values (0, 0, 0, 0.05, 0), and every added term is exactly zero with no extra random draw. So step 8 shows that the seams alone change nothing: the workspace tests pass as they do now, and the red-card diff against the baseline is all zero.
- **Measure with the tuning loop.** The inner loop is the red-card suite and the two failing pairings through `--baseline`. The criterion runs are the three slow tests. The slice gate is the full calibrate suites: five seeds for equal and strength, and one seed for formations (steer.md, Q-I1).
- **No NFR is the rationale for a mechanism choice.** The benchmark gate (+10% per tick) is the only performance line. The 2000 ms per-match budget (NFR-1, `yields-to: C2`) is not in tension.

## Step-by-Step Plan

1. **Baselines.** Build release at `a447ff1` before any code change. Run and copy into `implement-evidence/lone-forward/baseline/`:
   - `engine-cli calibrate --suite red-card --seed 1 --matches 120 --out …/red-base`;
   - `engine-cli calibrate --suite formations --pairing "4-4-1-1 v 4-4-2" --pairing "3-4-3 v 4-4-2" --seed 42 --matches 1000 --out …/pairs-base`;
   - `engine-cli calibrate --suite equal --seed 42 --matches 1000 --out …/equal-base`.
   Record the command, commit and `fixtures.hash` of each (Q-X6).
2. **Tuning values at neutral.** Add `lone_hold`, `lone_layoff` and `lone_dribble` (−5 to 5, default 0.0) to `DecisionWeights`. Add `tackle_win_base` (0 to 0.5, default 0.05) and `lone_line_hold` (0 to 1, default 0.0) to `Tuning`. Give each a garde range and `#[serde(default = …)]`, and mirror each in `Default`. Write the neutral values into `content/tuning.json`. `TUNING_VERSION` stays 2. In `tests/content.rs`, pin the values and add a test that a tuning block without them loads with the defaults.
3. **Tackle odds.** Add `pub fn win_chance(tackler: &Derived, carrier: &Derived, t: &Tuning) -> f64` to `rules/fouls.rs`, with unit tests: at 0.05 it equals the old formula, and it scales linearly with the base. Call it from `sim.rs:1017`. Replace the eight copied formulas in the test files listed above (`injury.rs:26` keeps its 0.5 factor).
4. **One offside line.** Extract `pub fn second_last_depth(team, players, attack_x) -> f64` from `offside_set`, and make `offside_set` call it. The existing offside tests must pass unchanged.
5. **The team's lone forward.** Add `Team::lone_forward() -> Option<usize>`. It returns the single active slot among the front-line slots of the base formation (within `BACK_LINE_BAND` of the most advanced outfield slot), and never the acting keeper. Unit tests: 4-4-1-1 and 3-4-3 give the striker; 4-4-2 gives `None`; 4-4-2 with one striker sent off gives the other; 4-4-2 with the keeper sent off and a striker in goal gives the other striker.
6. **Lone-carrier terms.** In `options()`, compute `lone` in one pass over the carrier's active outfield team-mates (none with a greater depth). When lone, add `w.lone_layoff` to each pass candidate with `progress <= 0.0`. When lone and pressed, add `w.lone_hold` to hold and `w.lone_dribble` to dribble. Add no draw. A goalkeeper carrier is never lone for these terms, because he has no dribble or hold.
7. **The line hold off the ball.** In `decide()`, after the anchors are set and while team `team` holds the ball, find `teams[team].lone_forward()`. If he is active and not the carrier, set his target depth to `anchor + lone_line_hold × (line − 0.5 − anchor)`, where `line` is `second_last_depth()` for his team. Never set it past `line − 0.5`, and keep `y` from the anchor. Clamp with `pitch::clamp`. The step adds no draw, and at `lone_line_hold` 0 the target is the anchor.
8. **Neutral check.** Run `cargo test -p engine --all-features` and `cargo test --workspace --all-features`. Every test must pass with no expectation changed. Run the red-card suite with `--baseline …/red-base/report.json`. Every row must show a change of 0, with the content hash unchanged or named as the change under test. Keep the output as evidence.
9. **Regression scenes (fast, not ignored)** in `tests/lone_forward.rs`, with tuned values read from the content:
   - **Lone forward uses his team-mates.** Home 4-4-1-1 striker with the ball at (30, 0). An away presser is 1.5 m in front of him. An open home midfielder is 12 m behind him with a clear lane. Everyone else is spread away (`common::spread`). Over seeds 1 to 40, after one `step()`, the carrier either passed (the carrier is `None` and the ball moves toward the midfielder) or held (still the carrier, target within 0.5 m of his position). He never dribbles toward goal.
   - **The terms need a lone carrier.** The same scene with a second home forward 5 m ahead: the options equal those at neutral lone weights. This is checked through `Simulation` built twice, with the lone weights set to 0 and to the tuned values, and it gives the same kick on every seed.
   - **The lone forward holds the line.** The home side holds the ball in midfield. The away back line is at x = 20. The home lone forward's anchor is past it. After one `step()`, his target x is at most the second-last defender's depth minus 0.5 m.
   - **Contact scene.** A defender in reach of the carrier, with `Scene::rolls`: a draw just under `win_chance` gives a won tackle (the defender's team carries), and a draw just above it and under `win_chance + foul_chance` gives a foul event.
10. **Inner tuning loop.** Tune only `lone_hold`, `lone_layoff`, `lone_dribble`, `lone_line_hold`, `tackle_win_base` and `foul_base`, each inside its bounds. After each change, run `calibrate --suite red-card --seed 1 --matches 120 --baseline …/red-base/report.json` and `calibrate --suite formations --pairing "4-4-1-1 v 4-4-2" --pairing "3-4-3 v 4-4-2" --seed 42 --matches 1000 --baseline …/pairs-base/report.json`. Record each setting, its diff and its verdict in a table in the implement record. Treat a change the diff marks as noise as no change.
11. **Criterion runs (slow, release).** `cargo test --release -p engine --all-features --test defending --test discipline -- --include-ignored --nocapture`. `a_sending_off_gives_no_advantage` and `every_formation_holds` must pass with every pairing (the two moved pairings and the eight kept ones). `discipline_is_realistic` must pass. If a criterion still fails at every in-bounds setting, stop and report the failing arm or pairing with its figures. Do not change a limit, a card chance, the keeper values or a band.
12. **Re-derive moved expectations.** Run `cargo test --workspace --all-features` and the other slow tests (`strength`, `mentality`, `ai_trailing`). For each assertion that moves, record the value before and after, and the reason the new value is right. Never widen a tolerance.
13. **Close.**
    1. Add the five rows and the paragraph to `docs/reference/data-files.md`.
    2. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo test --workspace --all-features`.
    3. Run the benchmark compare from `05c-benchmark.md` (gate 1.6964 µs per tick and 8.33 MB).
    4. Run the slice gate: `calibrate --suite equal` and `--suite strength` on seeds 1, 7, 42, 99 and 2026, and `--suite formations --seed 42`, each at 1,000 matches. Record band misses against `…/equal-base` (recorded, not failed; Q-E4).
    5. Search the changed source, comments and commit text for workflow vocabulary before the commit.

## Verification Strategy

The four criteria are about match outcomes and one scripted engine scene. The engine produces them headless, so no screen is needed. The rung for each is headless runs of the real engine library or the real release binary (`cargo test` in `stack.testing`, and `engine-cli calibrate`). A unit test or static reasoning alone never counts.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| No advantage from a red card | `cargo test --release -p engine --all-features --test defending -- --include-ignored --nocapture a_sending_off_gives_no_advantage` (headless engine runs, 480 matches) | Rust 1.92 and cargo on the reference machine — yes | Nothing new: the slow test exists. Steps 2–7 build the levers. | `engine-cli calibrate --suite red-card --seed 1 --matches 120` (same seeds and arms, `calib.red_card` verdict) → pre-registered deferral (not expected: no wall) |
| The lone-forward formations hold | `cargo test … every_formation_holds` (4-4-1-1 and 3-4-3 rows of 1,200 matches) | yes | Nothing new | `calibrate --suite formations --pairing "4-4-1-1 v 4-4-2" --pairing "3-4-3 v 4-4-2" --seed 42` → pre-registered deferral |
| Nothing that passes now regresses | `every_formation_holds` (the eight kept rows) plus `cargo test … discipline_is_realistic` (200 matches) | yes | Nothing new | `calibrate --suite formations --seed 42` pairings with 4-4-2, and the equal suite's sending-off share → pre-registered deferral |
| A lone forward uses his team-mates | `cargo test -p engine --all-features --test lone_forward` (scripted scenes on the real engine, 40 seeds) | yes | `tests/lone_forward.rs` (step 9); `Team::lone_forward()`; `offside::second_last_depth()` | a traced single match (`engine-cli simulate` events) of a 4-4-1-1 side, counting passes and holds by the lone forward under pressure → pre-registered deferral |

No criterion depends on credentials, a device, an external service or missing infrastructure. Every run is local and uses tools already in `stack:`, so no `constraint-resolution:` line is needed. No wall is named.

## Test / Verification Plan

### Automated checks

- Lint and type check: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Unit: `rules/fouls.rs` (`win_chance`), `rules/offside.rs` (the existing tests through `second_last_depth`), `team.rs` (`lone_forward`).
- Integration (fast): `tests/lone_forward.rs`, `tests/content.rs`, and the eight test files that now call `win_chance`. The full-match validator must still read 0 violations (`tests/validator.rs`, `defending.rs` parity test).
- Criterion runs (slow, ignored, release): `a_sending_off_gives_no_advantage`, `every_formation_holds`, `discipline_is_realistic`, plus `strength`, `mentality` and `ai_trailing`.
- Calibrate: the red-card suite and the two pairings against the step-1 baselines (inner loop), and the full slice gate (step 13).
- Benchmark compare against `05c-benchmark.md`.

### Interactive verification (human-in-the-loop)

Automated only. Every criterion is a statistic of engine runs or a scripted engine scene. The page shows no new element: the forward's choices appear as the existing pass, hold and movement records.

## Risks / Watchouts

- **Both levers at their bounds may not pass a criterion** (high). The keeper arm today is 5.82 against 2.06, and 4-4-1-1 scores 5.18 against 4.0. Step 11 stops and reports. It does not raise a limit (slice Risks; RIM-12).
- **The lone terms reach other formations** (medium). Any carrier with no outfield team-mate ahead is lone, so the eight kept pairings move too. `every_formation_holds` reruns all ten, and the eight are a criterion.
- **Fewer fouls, fewer cards** (medium). A higher win share lowers fouls, yellow cards and sending-offs. `discipline_is_realistic` is rerun. Band misses are recorded, not failed (Q-E4).
- **Hold-up play invites contact** (medium). A holding forward stays in reach of the presser, and each tick in reach draws once. The hold decays with `hold_per_s`, and the tackle odds move in the same loop.
- **Every seeded result moves** (medium). Step 8 proves the seams change nothing at neutral values. Step 12 re-derives each expectation that moves after tuning.
- **Per-tick cost** (low). The lone test is one pass over at most 10 team-mates. The line hold is one more pass over the opponents, and only while a team holds the ball. The gate is 1.6964 µs per tick.
- **Offsides rise** (low). A forward on the line is flagged more often than one who never stands there. The offside count has no band. It is recorded in the gate run.

## Dependencies on Other Slices

- `tuning-loop` (complete, verified): the red-card suite, `--pairing`, `--baseline` and the noise mark used in steps 1, 8 and 10.
- `defending-and-discipline` (complete, verified): goal-side cover, the press, the acting keeper, the discipline mechanisms, and the three slow criterion tests this slice reuses unchanged.
- `keeper-and-shots` (next, not planned before this slice is complete; steer.md Q-X4): it owns the save model.
- `realism-tuning` (last): it owns goals, shots and possession tuning.

## Assumptions

Autonomous run: no product owner was present. Each discovery question the interview would have asked is answered here in the direction that meets the criteria at the least cost. None changes the scope, a public contract, a persisted data shape or a product owner answer.

- A1 (class: implementation-detail): a carrier is lone when no active outfield team-mate has a greater depth along his attack direction. This is the slice's own wording. It is derived each tick and stored nowhere.
- A2 (class: implementation-detail): the decision lever is three weighted terms in the existing scored options (lay-off, hold-up, pressed-dribble cost). There is no scripted rule and no new choice kind, so the forward can still run into space. RIM-12 names scripting "never dribble" as the misreading to avoid.
- A3 (class: implementation-detail): no term is added to the shot or the clearance. The slice's cause is the dribble through the defence. Fewer dribbles into pressure should give fewer shots, and the shot model belongs partly to `keeper-and-shots`.
- A4 (class: implementation-detail): the tackle-odds lever is a new bounded `tackle_win_base` (the 0.05 literal, 0 to 0.5) plus the existing `foul_base` inside its bounds. `foul_base` is the foul chance per contact tick, not the card chance per foul. So RIM-7 and RIM-12 are kept: the card chances stay fixed.
- A5 (class: implementation-detail): off the ball, the team's lone forward is the single active front-line slot of the base formation (`Team::lone_forward()`). He moves toward `second_last_depth() − 0.5 m` by `lone_line_hold` and is never past it. The line is the one the referee uses, extracted into `offside.rs`.
- A6 (class: implementation-detail): the five values land at neutral values, so the seams alone change nothing (step 8). Serde defaults equal the neutral values, and an older tuning file keeps today's behaviour. `TUNING_VERSION` stays 2, as for the defending values.
- A7 (class: implementation-detail): the 0.5 m onside margin is a code constant, not a tuning value. The slice names no tuned value for it, and `lone_line_hold` already sets how far he goes.
- A8 (class: implementation-detail): the eight test files that copy the 0.05 formula call `fouls::win_chance()`. Tests then follow the tuned value, and one formula lives in one place.
- A9 (class: implementation-detail; ac: "No advantage from a red card"; classification: runtime-evidence): the criterion test is `a_sending_off_gives_no_advantage` unchanged: seeds 1–120, cards off through the three card chances, arms keeper 11, centre-back 13 and striker 21, and the 1.6 × control-home limit. The calibrate red-card suite is the inner loop and the fallback, on the same seeds and arms.
- A10 (class: implementation-detail; ac: "The lone-forward formations hold"; classification: runtime-evidence): the criterion test is `every_formation_holds` unchanged: 120 seeds, the formation at home on odd seeds, starting 11 against 11 with cards on, as the product owner moved it with its limit unchanged (Q-I2).
- A11 (class: implementation-detail; ac: "Nothing that passes now regresses"; classification: runtime-evidence): the eight kept pairings are read from the same `every_formation_holds` run, and the discipline limits from `discipline_is_realistic` unchanged (200 matches).
- A12 (class: implementation-detail; ac: "A lone forward uses his team-mates"; classification: build-capability): a scripted scene on the real engine over 40 seeds, with noise on, proves the choice. A pass or a hold counts, and a dribble toward goal fails. A second scene shows that the terms need a lone carrier.
- A13 (class: implementation-detail): the tuning loop may change only the five new values and `foul_base`. It may not change the limits, the card chances, the keeper values, the defending values or any band. A criterion that cannot be met stops implement with the figures.
- A14 (class: implementation-detail): no snapshot version change. The lone state and the line hold are derived each tick from state the snapshot already carries.
- A15 (class: implementation-detail): augmentations. The benchmark is re-baselined on `a447ff1` (`05c-benchmark.md` rev 7, 1.5422 µs per tick; gate 1.6964 µs and 8.33 MB), because the slice changes per-tick code. `04b-instrument.md` is not re-authored, because no new dark path or event is added. `04c-experiment.md` is not involved, because no flag is added.
- A16 (class: implementation-detail): the second-opinion consult is not fired, although `appetite-medium-or-larger` holds. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`), as in earlier plans.
- A17 (class: implementation-detail): baselines are copied into the slice's evidence folder, and nothing new is committed for them (Q-X6). The slice gate runs the full suites (five seeds for equal and strength, one for formations), as steer.md requires (Q-I1).
- A18 (class: implementation-detail): the design gate is settled. The product owner confirmed the interim design at 2026-09-24T11:22:17Z (`po-answers.md`), which clears Q-L1. This slice changes no page.

## Blockers

None.

## Freshness Research

- No dependency is added or upgraded. The slice uses std `f64` arithmetic and the crates already in the workspace (`garde` for bounds, `serde` for defaults). `04-plan-data-schemas-generator.md` § Freshness Research records the `serde` 1.0 and `garde` 0.23 behaviour. The field-level `default` with a `deny_unknown_fields` container is already proven in this repository by `a_tuning_block_without_the_defending_values_loads_with_the_defaults` (`crates/engine/tests/content.rs`).
- IFAB Law 11 (offside) is read in `04-plan-match-rules.md` § Freshness Research. The lone forward's line uses the second-last opponent, as the referee's code does.
- A web search for a sourced ratio of won tackles to fouls per match found statistics sites (for example [FotMob fouls per match](https://www.fotmob.com/leagues/47/stats/season/23685/teams/fk_foul_lost_team/premier-league-teams) and [StatMuse](https://www.statmuse.com/fc/ask/premier-league-teams-fouls-per-game)) but no single agreed figure. The slice sets no band for the ratio. The shipped bands (`content/realism-bands.json`: yellow cards 1.2–2.6 per team, sending-off share 0.08–0.22) are the only card and foul yardsticks, and `realism-tuning` owns them.

## Recommended Next Stage

- **Option A (default): Implement** → `/wf implement football-manager-match-engine lone-forward`. The plan has no blocker, and the benchmark is re-baselined. Compact the session first. Workflow state lives in the artifact files, and the SessionStart hook re-reads it after compaction.
- **Option C: Revisit slice** → `/wf slice football-manager-match-engine`. Take this only if implement shows that both levers at their bounds cannot pass a criterion (step 11 stop).
