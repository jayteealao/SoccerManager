---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: tempo-and-restarts
status: awaiting-input
stage-number: 5
created-at: "2026-09-25T04:27:37Z"
updated-at: "2026-09-25T08:45:00Z"
metric-files-changed: 18
metric-lines-added: 1065
metric-lines-removed: 27
metric-deviations-from-plan: 10
metric-review-fixes-applied: 0
commit-sha: "4022e47a4c50e88e4564825d571238d7c80e511a"
commits:
  - "831553b5b49baa7105b9ffdbd33e693661910809"
  - "4022e47a4c50e88e4564825d571238d7c80e511a"
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-25T08:45:00Z"
    trigger: answers-returned
    because: "Q-TR1 answered: add defending levers that contest a ball carrier in this slice"
    changed: "three contest levers built and committed switched off; 181 tuning runs; stopped again on the ball-in-play limit (Q-TR2)"
has-blockers: true
open-questions:
  - "Q-TR2 The ball stays in play too long once shots are realistic — AWAITING INPUT (po-answers.md)"
steering-honored:
  - "Q-TR1 (steer.md, 2026-09-25): the added levers only contest a ball carrier (tackle reach, how close pressers come, the chance to win the ball from a running carrier); the clear weight was tuned for throw-ins; no limit, band, card chance per foul, save value, restart median, added time or tick rate changed; the run stopped and reported when the levers could not meet a criterion with 10–16 shots per team."
  - "Heavy runs go to the server (2026-09-25): every calibrate sweep, the release slow tests, fmt, clippy and the workspace suite ran on the server through the sync script; the benchmark and the two Windows-only tests ran on this PC with -j 6."
  - "Q-KS2: corners from clearances stay one mechanism (a defender's clearance of a fast pass in his own area); no parry was turned toward the goal line."
  - "Q-E4: band misses are recorded, not failed; no band or limit was changed."
  - "Q-I1: targeted calibrate runs were the inner loop only; the slice gate was not run because no setting was accepted."
  - "Output boundary: code comments, test names, docs and the commit message use product language; the added lines were leak-checked before the commit."
  - "Design direction: not applicable; no page changed."
tags: [engine, passing, restarts, realism, defending]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-tempo-and-restarts.md
  plan: 04-plan-tempo-and-restarts.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/tempo-and-restarts/
  history: history/05-implement-tempo-and-restarts-0.md
  siblings: [05-implement-keeper-and-shots.md, 05-implement-lone-forward.md, 05-implement-tuning-loop.md, 05-implement-realism-bands-v2.md, 05-implement-defending-and-discipline.md]
  verify: 06-verify-tempo-and-restarts.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine tempo-and-restarts"
---

# Implement: Tempo and restarts

## The Implementation

The first pass stopped on Q-TR1: fewer passes gave 27–61 shots per team, because a carrier outran every defender. The product owner allowed defending levers that contest a carrier. Three are now in the engine at commit `4022e47`: `tackle_reach` (how far from the ball a tackle is attempted), `press_engage` (from how far a presser runs at the ball instead of the carrier's path) and `tackle_dribble_win` (an extra win chance against a carrier running with the ball). They ship at values that reproduce play. A 200-match calibration on seed 42 gives the same figures before and after. The counting, the carry window and the clearance levers from `831553b` are unchanged.

The levers work on shots. 181 settings on seed 42 (200 matches each) gave 28 settings with 10–16 shots per team at 373–545 passes. But the fix moved the wall to the ball-in-play limit. With realistic shots, no setting keeps throw-ins at 55 or fewer and the ball in play at 65 minutes or less; the best is 65.5 minutes on seed 42. The best setting that also keeps sending-offs in limit (`b0`) gives 66.22 minutes on the test seeds 1–200. The measured cause is too few stoppages: `b0` gives 1.98 corners per team, under the 3.5–6.5 band, and throw-ins are already at their 55 cap. The restart delays already sit at the sourced medians. The product owner said to stop in this case, so the choice is Q-TR2 in `po-answers.md`.

When Q-TR2 is answered, a rerun ships a setting from these sweeps and runs the earlier criteria and the gate. The top risk is discipline. Fouls rise from 6.4 per team on shipped play to 12–17 at the settings with realistic shots, so the sending-off share sits near its 0.25 limit.

## Summary of Changes

- This revision (`4022e47`):
  - `tackle_reach`: an opponent attempts a tackle when within this distance of the ball. It was the control reach (`reach_radius`, 1 m).
  - `press_engage`: a presser runs at the ball inside this distance, and at the point where he meets the carrier's run outside it. It was the constant `PRESS_ENGAGE_M` (3 m).
  - `tackle_dribble_win`: the win chance of a tackle on a carrier running faster than 2 m/s gains this value, scaled by the same tackling-to-dribbling ratio. A standing carrier is not affected. No draw is added.
  - Scene tests: a tackle from 1.6 m wins only under a 2 m reach, and a roll between the standing and the running win chance wins only against a running carrier. Two content tests pin the shipped values and load an older tuning file with the values that reproduce play.
- Kept from the first pass (`831553b`): `Kick::Clear`, the restart-taker mark, the three counters, `stats.ball_in_play_s`, the carry window, the clearance levers, snapshot version 6, `Scene::clear` and the slow criterion tests.

## Files Changed

This revision:
- `crates/engine/src/tuning.rs`: the three fields, their bounds and the defaults that reproduce play.
- `crates/engine/src/sim.rs`: the tackle reach, and the running-carrier chance in the tackle contest (`RUNNING_SPEED` 2 m/s).
- `crates/engine/src/rules/fouls.rs`: `dribble_win_chance()` and its unit test.
- `crates/engine/src/decision.rs`: the presser reads `press_engage`; the constant is removed.
- `content/tuning.json`: the three values at 1.0, 3.0 and 0.0.
- `crates/engine/tests/tempo_and_restarts.rs`: two scenes and their helpers.
- `crates/engine/tests/content.rs`: the pin and the older-file test.
- `docs/reference/data-files.md`: three rows and a paragraph.

The first pass changed 18 files, listed in `history/05-implement-tempo-and-restarts-0.md`. The slice total, `bce0144..4022e47` outside the workflow folder, is 18 files, +1,065 and −27.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs` (tackle contest) and `crates/engine/src/rules/fouls.rs`: `defending-and-discipline` and `lone-forward` own the win chance and `tackle_win_base`. The new chance is added after theirs and leaves them unchanged.
- `crates/engine/src/decision.rs` (pressers): `defending-and-discipline` added the intercept point and the cover. Only the engage distance moved to tuning.
- `content/tuning.json` and `crates/engine/src/tuning.rs`: shared by every tuning slice. The new fields have serde defaults, so `TUNING_VERSION` stays 2.

## Notes on Design Choices

- The running-carrier chance is added to the win chance and uses the tackle's one draw. A standing carrier keeps today's odds, so shielding the ball and holding it under pressure stay possible.
- The tackle reach is a separate value from the control reach, so a loose ball is still taken from 1 m.
- The levers ship at their old values so that other sessions that sync the tree test the play that verify accepted, not an unaccepted setting.
- The bound of `tackle_dribble_win` is 0 to 1. The sweeps used values up to 1.0 (`s17`–`s23`), because more clean wins per contact means fewer attempts that can be fouls.

## Verification Seams Built

- Only real passes count → the first pass's seams are unchanged (`Kick::Clear` at `crates/engine/src/decision.rs:24`, counting in `kick_ball()`, `Scene::clear`). `cargo test -p engine --all-features --test tempo_and_restarts` locally: 10 passed, 4 ignored.
- The contest levers → `tackle_reach` at `crates/engine/src/sim.rs:1327`, the running chance at `crates/engine/src/sim.rs:1331-1334`, `dribble_win_chance` at `crates/engine/src/rules/fouls.rs:81`, `press_engage` at `crates/engine/src/decision.rs:78` and `:83`. The scenes `an_opponent_inside_the_tackle_reach_can_win_the_ball` and `a_running_carrier_is_easier_to_tackle_than_a_standing_one` observe them on the real engine.
- Passes, ball in play, throw-ins → the slow tests `passes_are_realistic`, `the_ball_is_in_play_for_about_an_hour`, `throw_ins_stay_in_band` and `restart_census` over one shared 200-match run. A candidate setting is measured by patching the synced `content/tuning.json` on the server (`.scratch/remote/patch.py`).

## Tuning Loop (Q-TR1 rerun)

Every run is `engine-cli calibrate --suite equal --seed 42 --matches 200` on the server. All 181 rows are in `implement-evidence/tempo-and-restarts/contest/s10.txt`–`s27.txt`, with the patch of each run beside it (`sNN.json`, keys relative to the X1 setting in `x1.json`). `so` is the share of matches with a sending-off.

| Setting | Passes | Acc % | Throw-ins | Ball in play | Shots | Fouls | so | Verdict |
|---|---|---|---|---|---|---|---|---|
| X1 (first pass) | 557 | 83.7 | 43.9 | 52.6 | 56.4 | 11.5 | 0.255 | shots ×4 |
| X1 + reach 2.0 (`s10 r2.0`) | 442 | 81.5 | 52.1 | 49.5 | 29.0 | 35.4 | – | reach cuts shots; fouls ×3 |
| X1 + reach 3.0 (`s10 r3.0`) | 290 | 82.7 | 47.0 | 42.0 | 12.1 | 64.5 | – | realistic shots only with 65 fouls |
| reach 1.6, running 1.0, engage 0, fouls 0.05 (`s18 r1.6f0.05`) | 609 | 74.3 | 59.7 | 65.4 | 13.3 | 15.1 | 0.270 | passes fail |
| surrogate pick, fouls 0.025 (`s22 n2f0.025`) | 486 | 84.7 | 60.8 | 67.3 | 11.0 | 12.3 | 0.180 | ball in play and throw-ins fail |
| `b0` = `s23 n2f0.04r1.4` | 484 | 86.0 | 55.0 | 65.7 | 14.7 | 14.3 | 0.235 | ball in play fails |
| `s23 n2f0.045a1.2` | 478 | 85.7 | 51.9 | 65.5 | 13.6 | 16.8 | 0.265 | closest; sending-offs over 0.25 |
| `s26 db-0.5r1.35c-0.62` | 434 | 85.8 | 61.7 | 63.3 | 16.5 | 14.8 | 0.230 | throw-ins fail |

`b0` on the test seeds 1–200 (`implement-evidence/tempo-and-restarts/contest/b0-criteria.txt`, patch `b0.json`): passes 493.6 per team, accuracy 85.28%, throw-ins 52.95, ball in play 66.22 minutes.

A least-squares fit over the rows gives ball in play ≈ 90.9 − 0.21 × throw-ins − 0.40 × goal kicks − 0.51 × fouls per team − 0.27 × corners per team. With throw-ins at their 55 cap and shots at 16 or fewer, the goal kicks and corners that shots produce are too few to take off the last minute.

A diagnostic on 6 matches at X1 (not committed) showed the cause the levers target: 77% of shots came after a received pass and a carry of 5–20 m, with the nearest opponent 2–4 m away and none within the 1 m tackle reach.

## Criterion Results

At the shipped values (levers off) play is identical to `831553b`: `tr2-shipped` and `tr-count` give the same equal-suite figures on seed 42 (`checks2/shipped-play-compare.txt`). So the counting criterion passes and the other three still fail as at the first pass.

| Criterion | Evidence this run | Result |
|---|---|---|
| Only real passes count | `cargo test -p engine --all-features --test tempo_and_restarts` locally: 10 passed, 4 ignored | pass |
| Passes are realistic (350–550, 75–88%) | `b0` on seeds 1–200: 493.6 per team, 85.28% | passes at `b0` |
| The ball is in play for about an hour (52–65) | `b0` on seeds 1–200: 66.22 minutes per 90 | fails (Q-TR2) |
| Throw-ins stay in band (35–55) | `b0` on seeds 1–200: 52.95 per match | passes at `b0` |
| Shots realistic (Q-TR1 condition, 10–16) | `b0` on seed 42: 14.7 per team | passes at `b0` |
| Earlier criteria | not run: no setting was accepted | not run |

At `b0` on seeds 1–200 the census gives each restart's median dead time at its source (throw-in 13.8 s, goal kick 23.2 s, corner 31.8 s, free kick 32.5 s), 31.0 fouls per match, and corners from saves and parries (1.00 per team) and blocks (0.47 per team), none from a cleared pass. At `b0` the fast scene `the_kick_off_is_a_restart_kick_and_the_next_pass_is_a_pass` also failed. A rerun that ships a setting must fix that scene or its tuning.

The red-card suite was not measured, because no setting was chosen (steer.md asks for it on the chosen setting).

## Checks Run

- Server, on the tree of `4022e47` (`implement-evidence/tempo-and-restarts/checks2/tr2-checks/`): `cargo fmt --all -- --check` exit 0; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0; `cargo test --workspace --all-features --no-fail-fast` 518 passed, 2 failed, 14 ignored. The two failures are `data::tests::relative_paths_never_leak_the_root` and `a_small_run_writes_a_record_per_match_and_a_report_that_validate`, the Windows-only tests steer.md names.
- This PC: both Windows-only tests passed (`cargo test -p engine --lib relative_paths`: 1 passed; `cargo test -p engine-cli a_small_run_writes_a_record_per_match_and_a_report_that_validate`: 1 passed).
- Benchmark on this PC (`engine-cli bench --seed 42 --matches 5 --json`, three drives, `implement-evidence/tempo-and-restarts/bench2/`): 1.5847, 1.5966 and 1.5854 µs per tick against the 1.7071 µs gate; peak memory 6.81–6.97 MB against 8.55 MB; 285,850 ticks per match.
- Shipped play unchanged: `tr2-shipped` (content `4d3369e439b3`) and `tr-count` (content `c8b35962419f`) give identical suite figures, from passes 1,193.798 to yellow cards 0.845.
- Page tests were not run. No page changed.

## Deviations from Plan

1. The first pass's seven deviations stand (`history/05-implement-tempo-and-restarts-0.md`). (class: implementation-detail)
2. Three defending values were added, which the plan did not list. Q-TR1 (product owner, 2026-09-25) allows bounded values that contest a carrier. Each is a tuning value with a bound and a default that reproduces play. (class: implementation-detail)
3. `foul_base` (the foul chance of one tackle attempt, bound 0–1) was swept at 0.025–0.07 against its shipped 0.1. With realistic shots a side makes about twice today's tackles. At 0.1, the settings with 10–16 shots gave 22.6–66.7 fouls per team, and sending-offs in 0.46–0.68 of matches where the share was printed. The card chance per foul (RIM-7, Q-TR1) was not changed. No setting with a changed `foul_base` ships in this revision. (class: implementation-detail)
4. The sweeps also moved `decision.lane`, `min_lane`, `dribble_base`, `hold`, `hold_per_s`, `first_touch` and the clearance aim spread inside their bounds, as the first pass did. (class: implementation-detail)

## Anything Deferred

- Shipping a tuned setting, moving the four restart delays to the sourced medians, running the earlier criteria and the gate, and measuring the red-card suite: all wait for Q-TR2.
- The cross clearance produced no corner at any measured setting. The fast passes it needs are rare in this engine. `realism-tuning` owns the corners band.

## Known Risks / Caveats

- The slow tests `passes_are_realistic`, `the_ball_is_in_play_for_about_an_hour` and `throw_ins_stay_in_band` fail at the shipped values. They are ignored tests and record the open criteria.
- `RUNNING_SPEED` (2 m/s) is a literal in `sim.rs`, not a tuning value. It only matters when `tackle_dribble_win` is above 0.
- With realistic shots the sending-off share sits at 0.18–0.32 on seed 42 against the 0.25 limit of `discipline_is_realistic`. A shipped setting must pass that test on its own seeds.

## Assumptions

- A-I1 (class: implementation-detail): the levers are committed at values that reproduce play, as in the first pass, so that other sessions testing the tree test accepted play.
- A-I2 (class: implementation-detail): "the chance to win the ball from a dribbler" is an extra win chance against a carrier faster than 2 m/s. "How close pressers come" is the engage distance, plus a tackle reach that is separate from the control reach.
- A-I3 (class: implementation-detail): the foul chance per tackle attempt is a contest value that Q-TR1 allows to tune. The card chance per foul, which Q-TR1 and RIM-7 fix, is a different value and was not changed.
- A-I4 (class: implementation-detail): seed 42 runs of 200 matches rank settings (the inner loop); only the test-seed run of `b0` is a criterion reading.

## Triage Decisions

- Q-TR2 (class: intent-bearing; ac: "The ball is in play for about an hour"; classification: runtime-evidence): stopped. With shots at 10–16 per team, no in-bounds setting of the allowed levers keeps throw-ins at 55 or fewer and the ball in play at 65 minutes or less, while sending-offs stay in limit. Each way out moves a limit, a sourced median or another slice's ownership, so none is taken autonomously. Q-TR2 also asks whether the foul chance per tackle attempt may ship at 0.04 (shipped 0.1).
- Passes are realistic (class: implementation-detail; ac: "Passes are realistic"; classification: runtime-evidence): met at `b0` on the test seeds (493.6, 85.28%). The setting is not shipped until Q-TR2 is answered.
- Throw-ins (class: implementation-detail; ac: "Throw-ins stay in band"; classification: runtime-evidence): met at `b0` on the test seeds (52.95).
- Only real passes count (class: implementation-detail; ac: "Only real passes count"; classification: build-capability): built and proven by the scenes.
- Contest levers (class: implementation-detail): built as three bounded tuning values within Q-TR1. The code, the tests and the documentation ship.

## Freshness Research

- No dependency was added or upgraded. `serde(default)` on a field inside a `deny_unknown_fields` struct follows `tackle_win_base` and `lone_line_hold` in `tuning.rs`.
- The real stoppage counts used in Q-TR2 come from `docs/design/realism/01-engine-realism.md` (fouls 26.2 a match at `:702`, ball in play 55.4% at `:693`, restart medians at `:614`).

## Recommended Next Stage

- **Option D: Blocked** → the product owner answers Q-TR2 in `po-answers.md`. Then rerun `/wf implement football-manager-match-engine tempo-and-restarts` to ship a setting under the answer and run the earlier criteria and the gate.
- **Option C: Revisit Plan** → `/wf plan football-manager-match-engine tempo-and-restarts` if the answer moves the ball-in-play criterion or adds a stoppage lever, since that changes the plan's criteria or lever list.
