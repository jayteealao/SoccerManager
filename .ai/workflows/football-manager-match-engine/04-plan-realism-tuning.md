---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: realism-tuning
status: complete
stage-number: 4
created-at: "2026-09-25T18:12:40Z"
updated-at: "2026-09-25T18:12:40Z"
metric-files-to-touch: 14
metric-step-count: 17
has-blockers: false
revision-count: 0
revisions: []
consult-runs:
  - {trigger: appetite-medium-or-larger, provider: codex, at: "2026-09-25T18:35:55Z"}
tags: [calibration, tuning, realism, e2e]
stack-source: confirmed
open-questions: []
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-realism-tuning.md
  siblings: [04-plan-tempo-and-restarts.md, 04-plan-keeper-and-shots.md, 04-plan-lone-forward.md, 04-plan-tuning-loop.md, 04-plan-realism-bands-v2.md, 04-plan-defending-and-discipline.md]
  benchmark: 05c-benchmark.md
  implement: 05-implement-realism-tuning.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine realism-tuning"
---

# Plan: Realism tuning

## The Plan

`tempo-and-restarts` passed verify at `b9c9212`. It ships the pass counting, with play unchanged. On that play, seed 42 (1,000 matches, equal suite) misses eight figures: passes 1,191 per team (band 350–550), ball in play 89.5 minutes (52–65), throw-ins 22.8 (35–55), corners 1.59 per team (3.5–6.5), goals 1.88 per match (2.4–3.2), goalless share 0.22 (0.04–0.12) and yellow cards 0.94 per team (1.2–2.6). The red-card test fails in all three arms. Only 4-2-3-1 of the ten formations is clearly inside 2.4–3.2 goals against 4-4-2. The `b0` patch from `tempo-and-restarts` is the best starting point: at 200 matches it passed passes, throw-ins and ball in play near 66 minutes, and the reduced side scored less in every red-card arm. It failed discipline, the stronger-team test and the red-card cap, because its control side scored too little.

The product owner answered four questions for this plan (Q-RT1 to Q-RT4):
- The formations gate is the ten pairings against 4-4-2, each judged on 2.4–3.2 goals per match. The 55-pairing suite is recorded, not gated.
- The slice may add code for a new corner source. Tuning alone cannot reach 3.5 corners per team, because the only sources today give about 0.13 corners per shot.
- The slice may tune the sourced restart delays, the contest levers, `foul_base`, the booked-player factor and cooldown, the decision weights and carry window, the shot weights, shot range and noise, and the xG fit. The card chance per foul, the save curve and the hold share stay frozen.
- The full gate runs formations on seed 42 only, about 3.5 hours on the server.

The plan has 17 steps over 14 files, 1 of them new. First, the corner source is built at a value that leaves play unchanged, and it is committed with its scenes. Next, a feasibility stage tests the four walls that could stop the slice: corner throughput, the opposed formations, the discipline window and the red-card arms. Each test takes minutes, so a wall stops the slice early. Then the values are tuned jointly on a fast server loop, starting from `b0`, within a budget of 8 rounds. The best candidate goes to a five-seed check, and then to the full gate. The top risk is that no setting passes every gated band on five seeds. The plan then stops and reports the closest result and the bands that conflict, and it never widens a band.

## Current State

- **Code:** HEAD `b9c9212`. `06-verify-tempo-and-restarts.md` is `result: pass`. The gate reports are in `verify-evidence/tempo-and-restarts/`.
- **Tuning pins:** `content/tuning.json` must equal `Tuning::default()` (`crates/engine/tests/content.rs:143`). Further pins are at `content.rs:150`, `:279` and `:288`. The serde defaults for older files must reproduce old play (`crates/engine/src/tuning.rs:585-606`, `content.rs:265`, `:321`).
- **Shipped values that matter** (`content/tuning.json`):
  - restart delays: throw-in 3 s, corner 8 s, goal kick 6 s, free kick 8 s (`:59-62`); the sourced medians never shipped (Q-TR3)
  - contest: `tackle_reach` 1.0, `press_engage` 3.0, `tackle_dribble_win` 0.0, `tackle_win_base` 0.5 at its bound (`:52-55`)
  - fouls: `foul_base` 0.1, `foul_booked_factor` 0.15, `foul_cooldown_ticks` 150 (`:43`, `:50-51`); the card chances `yellow_base` 0.05, `yellow_aggression_weight` 0.15 and `red_base` 0.005 are frozen (`:47-49`)
  - carry window 0 and 0 (`:90-91`); decision weights `lane` 0.5, `min_lane` 1.5, `clear` −1.0, `hold` −0.4, `hold_per_s` 0.5, `dribble_base` −0.4, `first_touch` 0.6 (`:69-85`)
  - shots: `shot_base` 1.0, `shot_lane` 0.8, `shot_distance` 1.0, `pressure` −0.8, `shot_range` 21, `shot_noise` 0.5, `aim_noise` 0.06; xG −4.191 / −0.0441 / 6.3836 (`:32`, `:96-98`)
  - clearances: `aim_spread` 0.6, `cross_chance` 0.0 (`:123-125`); the cross clearance is `try_clear_cross` (`crates/engine/src/sim.rs:1158`)
- **The `b0` patch** (`implement-evidence/tempo-and-restarts/b0-ship/b0-ship.patch`, identical to `.scratch/remote/cand1.json`) changes 18 values:
  - `foul_base` 0.04, `tackle_reach` 1.4, `tackle_dribble_win` 0.97
  - restart delays at the medians
  - `lane` 0.37, `min_lane` 0.73, `dribble_base` −0.54, `first_touch` 0.04, `clear` −0.59, `hold` 0.18, `hold_per_s` 0.21, `carry_s` 2.53, `carry_cost` 1.27
  - `aim_spread` 1.3, `cross_chance` 0.3

  Its slow tests on seeds 1–200 (`b0-ship/criteria.txt`) gave passes 493.6 at 85.28%, throw-ins 52.95, ball in play 66.22 and corners 1.465, and every formation held. It failed discipline (0.160 second yellows against 0.10, and a sending-off in 30% of matches against 25%) and the stronger-team test (94 wins against more than 100). In the red-card test the reduced side scored less in every arm (0.50, 0.73, 0.45), but the full side exceeded 1.6 times the control (control 0.63 and 0.69). The 1,000-match gate never ran at `b0`.
- **Sweeps s10–s27** (`implement-evidence/tempo-and-restarts/contest/`, equal suite, seed 42, 200 matches) mapped the contest and tempo levers. The best non-`b0` point, `s26 db-0.45r1.3c-0.59`, gave passes 437, ball in play 63.3 and goals 2.34, but throw-ins 60.9 and shots 20.1. A fit gives ball in play ≈ 90.9 − 0.21·throw-ins − 0.40·goal kicks − 0.51·fouls per team − 0.27·corners per team (`history/05-implement-tempo-and-restarts-1.md:121`).
- **Why each figure misses** (research, sub-agent 1):
  - Ball in play: the restart delays are short. At the medians, no setting with realistic shots and throw-ins went below 65.3, so corners and fouls must add the rest.
  - Corners: parries and blocks give about 0.13 corners per shot. The cross clearance gave zero corners at every setting.
  - Goals and goalless share: the save model lowered on-target conversion, and the contest levers lower shot quality further. The shot weights and the xG fit were never tuned.
  - Yellows: `tackle_win_base` 0.5 halved fouls, and the card chance per foul is frozen. So yellows need about 8.5 or more fouls per team, while second yellows (at most 0.10 per match) and the sending-off band (at most 0.22) cap fouls at about 12 per team.
  - Red card: the contest levers remove the cause (a side with ten men attacks as if it had eleven). The cap then ties the test to the goals of the control side.
- **Formations:** `every_formation_holds` (`crates/engine/tests/defending.rs:250-275`) plays ten formations against 4-4-2, 120 matches each, and fails only above 4.0 goals per side (`:269`). The totals today are 4-4-2 1.47, 4-3-3 2.18, 4-2-3-1 2.55, 3-5-2 2.40, 4-1-4-1 1.12, 4-4-1-1 3.52, 4-1-2-1-2 0.84, 3-4-3 3.76, 5-3-2 0.91 and 5-4-1 1.91. The 55-pairing suite runs from 0.46 to 9.67 goals, with 10 of 55 in band.
- **Tests the retune moves** (research, sub-agent 2):
  - the three targets at `tempo_and_restarts.rs:449`, `:472`, `:485` (marked in `d6ccbb7`)
  - `defending.rs:206` (red card), `discipline.rs:140`, `strength.rs:14`, `keeper_and_shots.rs:233` (corner floor 1.2)
  - `engine-cli/tests/calibrate.rs:238` (57,000 matches, all suites; its comment at `:233` says bands miss until this slice)
  - fast tests: `full_match.rs:15` (seed 42 fouls and idle ticks), `commentary.rs:261-265` (seed 7 must score), `keeper_and_shots.rs:214` (penalties), and the doc claim at `tests/common/mod.rs:47-48`
  - browser tests: `e2e/tests/match-day.spec.mjs:10-14` (seed 3 scores early) and `first-match.spec.mjs:64-68` with step 8 at `:235` (seed 42 scores at 8x)
  - No test pins an outcome hash across builds (`determinism.rs:20` compares two runs of one build).
- **Costs on the server:** equal 167–175 s and strength 172–178 s per 1,000 matches; formations 9,713 s; the red-card suite about 165 s. The full gate is about 3.5 hours (`verify-evidence/tempo-and-restarts/status.txt`: 12,349 s).

## Simplicity Ladder

- Retuned values → rung 3 reuse — `content/tuning.json` and `Tuning::default()`. Only values change, each inside its declared bound.
- Corner source → rung 4 new code, with rung 3 reuse — `try_clear_cross()` (`sim.rs:1158`), `shot::deflect()`, the once-per-event draw pattern and the existing out-of-play rule. Rungs 1–3 do not hold: no current source gives more than about 0.13 corners per shot, and the cross clearance at every tried setting gave zero corners (Q-RT2).
- Inner tuning loop → rung 3 reuse with modification — `.scratch/remote/sweep.sh` and `sweep.py` (git-ignored). They patch a content copy per variant and print one row, but they are fixed to the equal suite on seed 42. They gain a seed argument and a suite argument, and the red-card suite and `--pairing` runs. `patch.py` and `tr3-cmp.py` are reused as they are.
- Candidate compare → rung 3 reuse — `engine-cli calibrate --baseline <report.json>` (`crates/engine-cli/src/cli.rs:395-510`) and `tr3-cmp.py`.
- Formations band in the slow test → rung 3 reuse — the band values in `content/realism-bands.json`, read with `serde_json` in the test. The engine crate does not depend on engine-cli, so the test reads the file and does not reuse `Bands`.

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist.

The repeat-deferral tripwire does not fire. The two open deferrals (the legibility reading and the macOS build) name no environment dependency of this slice.

## Likely Files / Areas to Touch

- `content/tuning.json`: the chosen values.
- `crates/engine/src/tuning.rs`: `Tuning::default()` mirrors the content; the clearances block gains the corner-source value with a bound and a serde default of 0.0.
- `crates/engine/src/sim.rs`: the corner source.
- `crates/engine/src/decision.rs`: only if the corner source needs the clearance direction.
- `crates/engine/tests/corner_source.rs` (new): scenes for the corner source.
- `crates/engine/tests/defending.rs`: `every_formation_holds` on the goals band.
- `crates/engine/tests/tempo_and_restarts.rs`: the three targets become gating.
- `crates/engine-cli/tests/calibrate.rs`: the all-suites test gates equal and strength, and prints formations.
- `crates/engine/tests/content.rs`, `full_match.rs`, `commentary.rs`, `common/mod.rs`, `keeper_and_shots.rs`, `match_stats.rs` and any other test the retune moves: pins re-derived.
- `e2e/tests/match-day.spec.mjs`, `e2e/tests/first-match.spec.mjs`: a new seed only if a seed stops scoring in time.
- `docs/reference/data-files.md` and the build book run analysis: the new values and the regenerated analysis.
- `.scratch/remote/sweep.sh`, `sweep.py` and a new gate script (git-ignored, not in the repo).

## Proposed Change Strategy

Build first, prove feasibility, then tune. The corner source is the only new mechanism. It lands first at 0.0, with scenes that prove it gives a corner only after a defending touch over the goal line (RIM-9). At 0.0 the gate figures must equal the `tempo-and-restarts` gate figures, which proves that it changes no play.

A feasibility stage then runs before any detailed tuning. It tests the four walls that could stop the slice, and each one takes minutes, not hours:
- **Corner throughput.** Existing sources give about 0.13 corners per shot, so 8–16 shots per team give 1.0–2.1 corners. The new source must add about 1.4–2.5 corners per team. The current cross clearance aims away from goal (`crates/engine/src/sim.rs:1153`), so the source must allow an exit over the goal line.
- **Opposed formations.** 4-1-2-1-2 must rise from 0.84 to at least 2.4 goals, while 3-4-3 must fall from 3.76 to at most 3.2. A shared scoring change cannot do both, so the stage must show that some lever moves them apart.
- **The discipline window.** Yellows must reach 1.2 per team, while the sending-off share stays within 0.08–0.22 and second yellows stay at or under 0.10.
- **The red-card arms.** In all three arms, the reduced side must not outscore the full side, and the full side must stay under 1.6 times the control. Goals that rise in proportion leave this ratio unchanged.

If a wall cannot move within the bounds, the slice stops at the feasibility stage and reports. It does not stop after hours of tuning.

The tuning is joint, not in gated phases. The loop starts from `b0`, and each round starts from the best candidate of the round before. Each round focuses on one group of levers: tempo, then contest and fouls, then corners, then shots and goals. The loop measures every gated figure on every candidate. So no round is judged only on its own figures, and ball in play can close through corners and fouls in a later round. The xG coefficients do not drive keeper saves (`crates/engine/src/shot.rs:37`). The refit corrects goals per xG only, and it runs last. Goals come from the shot weights, `shot_range` and the noise values.

The screen uses the exact fixtures of each gate. The CLI `--pairing` runs use generated league clubs and hashed seeds (`crates/engine-cli/src/calibrate/fixtures.rs:54`), and `every_formation_holds` uses the default clubs on seeds 1–120. So the formations screen runs the tightened slow test itself, not `--pairing`.

The gate follows Q-RT4: equal and strength on five seeds, formations on seed 42 (recorded), the ten pairings against 4-4-2 (gated), the red-card test and every slow test.

## Step-by-Step Plan

1. **Extend the loop.** Give `.scratch/remote/sweep.sh` and `sweep.py` a seed argument and a suite argument. Add a row for the red-card suite (`--suite red-card --seed 1 --matches 240`, with all three arms and the control). Add a row for the tightened formations test on its own fixtures. Add a counter row for corners by origin. Run `b0` on seed 42 at 200 matches, and record the row as the start point.
2. **Build the corner source at 0.0.** Add the clearances value (serde default 0.0, garde bound) in `tuning.rs` and `content/tuning.json`. In `sim.rs`, a pressured clearance or a cross clearance near the defenders' own goal line can go off the defender at a wide angle, with one referee draw per event. The angle must allow an exit over the goal line. The ball then crosses the line or not as the physics decides. Only the existing out-of-play rule gives the corner.
3. **Write the corner scenes** in `crates/engine/tests/corner_source.rs`: a scripted draw sends the ball over the own goal line and gives a corner with the defending side last; the value at 0.0 gives no corner; a ball that stops short gives no corner.
4. **Prove no play change.** On the server, run fmt, clippy and the workspace tests. Run the equal suite on seed 42 at 1,000 matches. Every figure must equal `verify-evidence/tempo-and-restarts/equal-42/report.json` (compare with `tr3-cmp.py`). Commit the corner source by explicit path.
5. **Feasibility: corners.** On `b0`, set the corner-source chance to its bound, and run seed 42 at 200 matches. Count the eligible events for each origin, and count the corners that follow. Also count the throw-ins, the intervening touches and the balls that stay in play. Corners per team must reach at least 3.5 at some in-bound value. If they do not, stop and report the counts.
6. **Feasibility: formations.** Run a small bounded sweep over the contest levers, the shot weights and `shot_range` on the tightened formations test with 40 seeds per pairing, for 4-1-2-1-2, 3-4-3 and 4-4-2. Some setting must move 4-1-2-1-2 up and 3-4-3 down together. If none does, stop and report the sweep.
7. **Feasibility: discipline and red card.** Sweep `foul_base`, `foul_booked_factor` and `foul_cooldown_ticks` around `b0` at 200 matches. Yellows must reach 1.2 per team, with the sending-off share in 0.08–0.22 and second yellows at or under 0.10. On the best point, run the red-card suite. Record the control and the three arms. If the window is empty, stop and report.
8. **Tune jointly.** Run rounds of at most 12 candidates each on seed 42 at 200 matches. Start from `b0`, and start each round from the best candidate so far. The round focus is tempo, then contest and fouls, then corners, then shots and goals, and then the cycle repeats. Each candidate reports every gated figure. When a candidate passes on seed 42, run it on seed 7 at 200 matches before it counts as the best. A figure within one standard error of a band edge goes to 1,000 matches.
9. **Refit the xG coefficients.** Use the refit in `keeper_and_shots.rs:382` on the best candidate. Goals per xG must be 0.85–1.15.
10. **Check the seeds.** Run the best candidate on the equal and strength suites at 1,000 matches on seeds 42, 1, 7, 99 and 2026 (about 15 minutes). Also run the tightened formations test and the red-card suite. Every gated figure must pass. If a seed misses, go back to step 8.
11. **Stop rule.** The tuning budget is 8 rounds (about 96 candidates, about 7 hours on the server). If no candidate passes step 10 within the budget, stop. Report the best candidate, every missed figure, and the bands that pull against each other. Write "no passing candidate found in the tested region", not a claim that none exists. Do not widen a band, and do not raise a limit.
12. **Write the chosen values** in `content/tuning.json` and `Tuning::default()`. Update the pins in `content.rs`.
13. **Make the tests gating.**
    - In `tempo_and_restarts.rs`, give the three targets the plain slow marker, and remove the targets paragraph.
    - In `defending.rs`, make `every_formation_holds` judge total goals against 2.4–3.2, read from `content/realism-bands.json`, and update its doc comment.
    - In `engine-cli/tests/calibrate.rs`, gate every equal and strength band. Print the formations bands and do not assert them (Q-RT1). Remove the comment at `:233`.
14. **Re-derive the moved expectations.** Run the workspace tests on the server and the two Windows-only tests on this PC. For each fast test that moves (`full_match.rs`, `commentary.rs`, `common/mod.rs`, `keeper_and_shots.rs` and others), re-derive the expectation. Record the old value, the new value and the reason. Do not widen a tolerance.
15. **Run the full gate on the server.** Run fmt, clippy, the workspace tests and every slow test with `--include-ignored`. Run the equal and strength suites on five seeds, the formations suite on seed 42 and the red-card suite, at 1,000 matches each (240 for red card). Copy back only the `report.json` files. The run takes about 3.5 hours.
16. **Run the benchmark and the browser suite on this PC.** Run `engine-cli.exe bench --seed 42 --matches 5 --json` three times, and take the median. The CPU per tick must be under 1.7071 µs, and the peak memory under 8.55 MB. Then run `cargo build --release`, and run `npm test` in `e2e/`. The result must be 21 of 21. If a seeded browser test fails because its seed no longer scores in time, pick a new seed that does, and record it.
17. **Update the documents and commit.** Update `docs/reference/data-files.md`, and regenerate the run analysis from the gate reports. Commit by explicit path only.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| Every gated band passes on every seed (Q-RT1, Q-RT4) | `engine-cli calibrate --suite equal` and `--suite strength --seed {42,1,7,99,2026} --matches 1000`, exit 0; `every_formation_holds` release slow test; `--suite formations --seed 42` recorded (headless) | The server, Rust 1.92, through `.scratch/remote/vps.sh` — yes | The tightened `every_formation_holds`; the gating `calibrate.rs:238`; the gate script | Targeted `--band` and `--pairing` runs with `--baseline` → the slow tests → stop and report the conflicting pair |
| No advantage from a red card | `cargo test --release -p engine --all-features --test defending -- --include-ignored a_sending_off_gives_no_advantage` (headless) | The server — yes | Nothing beyond the tuning | `calibrate --suite red-card --seed 1 --matches 240` → report each arm, limits unchanged |
| The browser suite passes | `cargo build --release`, then `npm test` in `e2e/`, headless Chromium (headless) | This PC (steer: browser tests stay local) — yes | A new seed in a spec only if its seed stops scoring | `npm run test:scenario` and `test:viewer` apart → `PW_CHANNEL=msedge` |
| The benchmark holds | `engine-cli.exe bench --seed 42 --matches 5 --json`, three drives, median (headless) | This PC (Windows counters) — yes | Nothing | The criterion `tick_step` → a recorded product-owner re-baseline |

No acceptance criterion depends on credentials, a device, an external service or a deploy target. Every environment is available now.

## Test / Verification Plan

### Automated checks

- On the server: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features --no-fail-fast`. Two Linux failures are expected by design. Run those two tests on this PC.
- On the server: every slow test with `cargo test --release -p engine --all-features --no-fail-fast -- --include-ignored`. Every test must pass, including the red-card test, the three tempo tests, discipline, strength and the tightened formations test.
- On the server: the calibrate gate from step 14. Every equal and strength run must exit 0.
- On this PC: `node --test web/tests/*.test.mjs`.
- The corner scenes in `corner_source.rs`.

### Interactive verification (human-in-the-loop)

- **Every gated band:** run the release `engine-cli calibrate` on the server as in step 14. Fetch each `report.json` to `verify-evidence/realism-tuning/`. Pass: every equal and strength band has `pass: true` on every seed, and each run exits 0.
- **The browser suite:** on this PC, run `cargo build --release`, then `npm test` in `e2e/`. Keep the Playwright report as evidence. Pass: 21 of 21.
- **The benchmark:** on this PC, run the bench three times and keep the JSON. Pass: under both tripwires.

## Risks / Watchouts

- **Bands that conflict.** More fouls help yellows and ball in play, but hurt the sending-off share and second yellows. More contest helps passes, shots and the red card, but lowers goals. More `clear` helps throw-ins but raises passes. The loop measures every gated figure on every candidate. The stop rule in step 11 applies.
- **The goals and the red-card cap.** At `b0` the red-card test failed only on the 1.6 times control cap. Goals that rise in proportion leave that ratio unchanged, so step 7 measures the control and all three arms early, and the loop measures them in every round.
- **The xG fit.** The xG coefficients do not drive keeper saves. The refit (step 9) corrects goals per xG only.
- **The formations pairings.** Shared values move all pairings together. 4-1-2-1-2 (0.84) and 3-4-3 (3.76) pull in opposite directions. Step 6 tests this wall before the tuning. If the ten cannot all come into band, the stop rule applies.
- **Seed sensitivity.** A value tuned on seed 42 can miss on another seed. Step 10 checks all five seeds before the gate, and step 8 checks seed 7 on every passing candidate.
- **The benchmark.** Dead-ball ticks are cheaper, so a lower ball-in-play figure can lower the CPU per tick without any code change. Record ticks per match beside the gate.
- **This PC.** Only the benchmark, the page tests, the browser suite and the two Windows-only tests run here, with `-j 6` for any cargo build.
- **Commits.** Commit by explicit path only. Another session keeps `docs/design/realism/*` staged.

## Dependencies on Other Slices

- `tempo-and-restarts` (complete): the pass counting, the ball-in-play figure, the carry window, the contest levers and `b0` as starting evidence.
- `keeper-and-shots`, `lone-forward`, `defending-and-discipline` (complete): the mechanisms being tuned and the slow tests that must stay green.
- `realism-bands-v2` and `tuning-loop` (complete): the band set, the formations suite and the calibrate options.

## Assumptions

- A1 (class: implementation-detail): the shots criterion is the band in `content/realism-bands.json`, 8–16 per team. The 10–16 range in steer Q-TR1 applied to `tempo-and-restarts` only.
- A2 (class: implementation-detail): the sending-off band (at most 0.22) and the discipline test (at most 25% of matches, at most 0.10 second yellows) both apply. The tighter figure binds.
- A3 (class: implementation-detail): ball in play 52–65 is judged by the slow test `the_ball_is_in_play_for_about_an_hour`. It is not a band in `realism-bands.json`, and this slice adds no band.
- A4 (class: implementation-detail): the restart delays may be set at or under the sourced medians, never above them.
- A5 (class: implementation-detail): `tackle_win_base` stays at 0.5. The `lone-forward` formations fix depends on it, and Q-RT3 does not name it.

## Blockers

None.

## Freshness Research

Skipped. The slice changes no dependency and adds no external API surface. The real-football figures come from `docs/design/realism/01-engine-realism.md` and `content/realism-bands.json`, which earlier slices sourced.

## Recommended Next Stage

- **Option A (default): Implement** → `/wf implement football-manager-match-engine realism-tuning`. The plan is complete. Compact the session first; the artifacts hold the state. Steer requires that you start this stage yourself.
- **Option C: Revisit slice** → `/wf slice football-manager-match-engine`. Use this only if you want the 55-pairing formations work as its own slice now.
