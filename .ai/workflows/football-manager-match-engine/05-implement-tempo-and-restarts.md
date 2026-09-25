---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: tempo-and-restarts
status: awaiting-input
stage-number: 5
created-at: "2026-09-25T04:27:37Z"
updated-at: "2026-09-25T09:15:00Z"
metric-files-changed: 18
metric-lines-added: 1065
metric-lines-removed: 27
metric-deviations-from-plan: 10
metric-review-fixes-applied: 0
commit-sha: "4022e47a4c50e88e4564825d571238d7c80e511a"
commits:
  - "831553b5b49baa7105b9ffdbd33e693661910809"
  - "4022e47a4c50e88e4564825d571238d7c80e511a"
revision-count: 2
revisions:
  - rev: 1
    at: "2026-09-25T08:45:00Z"
    trigger: answers-returned
    because: "Q-TR1 answered: add defending levers that contest a ball carrier in this slice"
    changed: "three contest levers built and committed switched off; 181 tuning runs; stopped again on the ball-in-play limit (Q-TR2)"
  - rev: 2
    at: "2026-09-25T09:15:00Z"
    trigger: answers-returned
    because: "Q-TR2 answered: ship b0, judge ball in play at 52-68 minutes, and stop if b0 fails every_formation_holds, discipline or an earlier criterion"
    changed: "b0 applied and measured on the server; discipline and the stronger-team criterion fail; b0 not shipped (kept as a patch); stopped on Q-TR3"
has-blockers: true
open-questions:
  - "Q-TR3 Setting b0 fails the discipline criterion and the stronger-team criterion — AWAITING INPUT (po-answers.md)"
steering-honored:
  - "Q-TR2 (steer.md, 2026-09-25): b0 was applied exactly (19 keys of b0.json, foul chance per tackle attempt 0.04, card chance per foul unchanged); the ball-in-play check was judged at 52-68; the realism-bands.json band was not touched; because b0 failed the discipline criterion and an earlier slice's criterion, the run stopped and reported instead of shipping."
  - "Q-TR1: the red-card suite (balanced fixtures) was measured on b0 and recorded; no limit was raised."
  - "Heavy runs go to the server (2026-09-25): the workspace suite and every release slow test ran on the server through the sync script; nothing heavy ran on this PC."
  - "Q-I1: the slice gate (five seeds, formations suite) was not run, because the shipped setting failed its criteria first."
  - "Output boundary: no commit carries code this revision; the record commit message uses product language."
  - "Design direction: not applicable; no page changed."
tags: [engine, passing, restarts, realism, defending]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-tempo-and-restarts.md
  plan: 04-plan-tempo-and-restarts.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/tempo-and-restarts/
  history: history/05-implement-tempo-and-restarts-1.md
  siblings: [05-implement-keeper-and-shots.md, 05-implement-lone-forward.md, 05-implement-tuning-loop.md, 05-implement-realism-bands-v2.md, 05-implement-defending-and-discipline.md]
  verify: 06-verify-tempo-and-restarts.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine tempo-and-restarts"
---

# Implement: Tempo and restarts

## The Implementation

Q-TR2 said to ship setting `b0`, judge ball in play at 52–68 minutes, and stop if `b0` failed `every_formation_holds`, the discipline criterion or an earlier slice's criterion. This run applied `b0` to `content/tuning.json` and `Tuning::default()`, updated the pinned values, the documentation and the ball-in-play limit, and ran every slow criterion on the server over 200 matches. The slice's own criteria pass at `b0`: 493.6 passes per team at 85.28%, 52.95 throw-ins, and 66.22 minutes of ball in play. `every_formation_holds` and the set-piece floor also pass.

Two earlier criteria fail. `discipline_is_realistic` gives 0.160 second yellows per match (limit 0.10) and a sending-off in 30.0% of matches (limit 25%). `a_stronger_team_wins_more_than_half_its_matches` gives 94 wins of 200 (it needs more than 100), with 81 draws. The product owner said to stop in this case. So `b0` is not shipped: the change is saved as `implement-evidence/tempo-and-restarts/b0-ship/b0-ship.patch` (it applies cleanly to `7fd8f12`), and the tree is back at the committed play. No code commit was made. The choice is Q-TR3 in `po-answers.md`.

When Q-TR3 is answered, a rerun applies the patch or a new setting and runs the gate. The top risk is scoring. At `b0` the red-card control sides scored 0.63 and 0.69 goals per match, which leaves the stronger team many draws. Nine fast tests also move at `b0` and must be re-derived when a setting ships (see Known Risks).

## Summary of Changes

- This revision (no code commit):
  - `b0` applied and measured, then reverted. The patch holds the five changed files: `content/tuning.json` (18 values), `crates/engine/src/tuning.rs` (the `Default` values and `FOUL_BASE` 0.04), `crates/engine/tests/content.rs` (the pins), `crates/engine/tests/tempo_and_restarts.rs` (the 52–68 limit) and `docs/reference/data-files.md` (the table values and the default of the `clearances` block).
  - Q-TR3 recorded in `po-answers.md`.
- Kept from earlier revisions: the counting and tempo levers (`831553b`) and the contest levers (`4022e47`), shipped at values that reproduce play.

## Files Changed

This revision changed no committed code. The slice total, `bce0144..4022e47` outside the workflow folder, stays 18 files, +1,065 and −27 (`history/05-implement-tempo-and-restarts-1.md`).

In the saved patch (not committed):
- `content/tuning.json`: the 19 `b0` keys; 18 differ from shipped (`press_engage` stays 3.0).
- `crates/engine/src/tuning.rs`: the `Default` mirrors the content; the serde defaults for an older file still reproduce play.
- `crates/engine/tests/content.rs`: pins for the contest values, the foul chance, the decision weights, the clearances and the delays.
- `crates/engine/tests/tempo_and_restarts.rs`: the ball-in-play check at 52–68, with a comment that the real band is 52–65.
- `docs/reference/data-files.md`: the new defaults, and that a tuning file without the `clearances` block loads with 0.6 and 0.

## Shared Files (also touched by sibling slices)

- None changed this revision. The patch touches `content/tuning.json` and `crates/engine/src/tuning.rs`, which every tuning slice shares.

## Notes on Design Choices

- The `b0` values were checked key by key against `contest/b0.json` after the edit: 19 of 19 match.
- `Tuning::default()` mirrors the content, as the plan says and `the_shipped_tuning_equals_the_documented_default` requires. The serde defaults stay at the values that reproduce play, so an older file plays as before.
- The run stopped on the first failed criteria and did not run the slice gate (about 5 hours on the server), because Q-TR2 makes a failed criterion a stop.

## Verification Seams Built

- None needed this revision: every seam the plan names was built in `831553b` and `4022e47` (listed in `history/05-implement-tempo-and-restarts-1.md`). The slow tests in `crates/engine/tests/tempo_and_restarts.rs` measured `b0` this run.

## Criterion Results (setting `b0`, server, this run)

Command: `cargo test --release -q -p engine --all-features --test tempo_and_restarts --test defending --test discipline --test keeper_and_shots --test strength --test lone_forward --test ai_trailing --test mentality --no-fail-fast -- --include-ignored --nocapture --test-threads 3`. Output: `implement-evidence/tempo-and-restarts/b0-ship/criteria.txt`.

| Criterion | Figure at `b0` | Limit | Result |
|---|---|---|---|
| Passes are realistic | 493.6 per team, 85.28% | 350–550, 75–88% | pass |
| The ball is in play for about an hour (Q-TR2 limit) | 66.22 minutes per 90 | 52–68 | pass |
| Throw-ins stay in band | 52.95 per match | 35–55 | pass |
| Only real passes count | fast scenes: 13 of 14 pass (see below) | — | pass (counting scenes) |
| `every_formation_holds` | passed | — | pass |
| `discipline_is_realistic` | 0.160 second yellows per match; sending-off in 30.0% of matches; 0 same-tick pairs | ≤ 0.10; ≤ 25% | **fail** |
| `a_stronger_team_wins_more_than_half_its_matches` | won 94, drew 81, lost 25 of 200 | > 100 wins | **fail** |
| `set_pieces_arise_from_play` | 1.465 corners per team, 13.705 goal kicks | ≥ 1.2, ≥ 10 | pass |
| `lone_forward` tests | 4 passed | — | pass |
| `a_trailing_ai_team_changes_its_tactics` | 1 passed | — | pass |
| `an_attacking_team_shoots_more_than_a_defensive_one` | 1 passed | — | pass |
| Red-card suite (recorded, owned by `realism-tuning`) | full side 1.32 (keeper off), 1.13 (centre-back), 1.37 (striker); control 0.63 and 0.69 | ≤ 1.01 | fail (not this slice's criterion) |

The restart census at `b0`: every restart kind's median dead time equals its source (throw-in 13.8 s, goal kick 23.2 s, corner 31.8 s, free kick 32.5 s); 31.01 fouls per match; 41.57 clearances and 42.59 restart kicks per team; corners from save or parry 1.000 and from blocks 0.465 per team, none from a clearance.

## Checks Run

- Server, workspace suite with `b0` applied (`cargo test --workspace --all-features --no-fail-fast`; the last 400 lines are in `b0-ship/workspace-tests-tail.txt`, so a full pass count is not verified): 6 targets failed — `engine --lib`, `engine --test full_match`, `engine --test tempo_and_restarts`, `engine-cli --test calibrate`, `engine-cli --test stream_cli`, `stream --test backpressure`. Details are in `b0-ship/workspace-tests-tail.txt` and `b0-ship/engine-failures.txt`.
- Of those, two are the Windows-only tests that steer.md names (`data::tests::relative_paths_never_leak_the_root`, `a_small_run_writes_a_record_per_match_and_a_report_that_validate`). The others move at `b0`: see Known Risks.
- Not run: `cargo fmt`, `cargo clippy`, the benchmark and the slice gate. No code is committed this revision, and the tree is at `7fd8f12`, where the previous revision recorded fmt and clippy at exit 0 and the benchmark at 1.5847–1.5966 µs per tick.

## Deviations from Plan

1. The deviations of the earlier revisions stand (`history/05-implement-tempo-and-restarts-1.md`). (class: implementation-detail)
2. `b0` was not shipped although Q-TR2 said to ship it, because Q-TR2 also said to stop if it failed the discipline criterion or an earlier criterion, and it failed both. (class: implementation-detail — this follows the product owner's stop condition; it settles nothing new)

## Anything Deferred

- Shipping a setting, re-deriving the moved fast tests, the fmt, clippy and benchmark checks on the shipped setting, and the slice gate: all wait for Q-TR3.
- The cross clearance still produces no corner at `b0` (0 of 586 corners). `realism-tuning` owns the corners band.

## Known Risks / Caveats

- At `b0` these fast tests fail and must be re-derived or fixed when a setting ships:
  - `decision::tests::an_open_teammate_draws_a_pass_or_a_shot` ("the carrier never passed or shot") and `rules::restart::tests::the_delay_comes_from_the_tuning` (690 ticks against a pinned 150) in `crates/engine` lib.
  - `ninety_minutes_play_to_full_time_under_the_laws` ("team 1 committed 21 fouls").
  - `the_kick_off_is_a_restart_kick_and_the_next_pass_is_a_pass`: the first counted kick was an away pass, not the home kick-off.
  - `a_drop_before_the_first_stoppage_goes_back_to_kick_off` (490 against 1), `record_writes_a_fixture_and_prints_its_counts` and `a_dropped_viewer_reconnects_on_the_same_port_and_the_match_goes_on` in `engine-cli --test stream_cli`.
  - `a_slow_client_pauses_the_producer_and_loses_no_tick` in `stream --test backpressure`. Not isolated: it may depend on the match, or it may be timing; not verified.
- The slow tests `passes_are_realistic`, `the_ball_is_in_play_for_about_an_hour` and `throw_ins_stay_in_band` still fail at the committed values. They are ignored tests and record the open criteria.
- `RUNNING_SPEED` (2 m/s) is a literal in `sim.rs`, as before.

## Assumptions

- A-I5 (class: implementation-detail): the working tree is restored to the committed play after the measurement, and the `b0` change is kept as a patch in the evidence folder. Other sessions that sync the tree then test accepted play, as in the earlier revisions.
- A-I6 (class: implementation-detail): the red-card suite on balanced fixtures is read from the slow test `a_sending_off_gives_no_advantage`, which plays the same seeds, orders and arms as `calibrate --suite red-card --seed 1 --matches 240` (recorded in the `lone-forward` and `keeper-and-shots` plans).
- A-I7 (class: implementation-detail): the `strength` and `discipline` slow tests are earlier slices' criteria that Q-TR2 names ("the earlier slices' criteria"), so a failure of either is a stop.

## Triage Decisions

- Q-TR3 (class: intent-bearing; ac: "earlier criteria must not regress"; classification: runtime-evidence): stopped. `b0` fails `discipline_is_realistic` and `a_stronger_team_wins_more_than_half_its_matches`. Leaving `b0`, moving a criterion or shipping only the counting each changes what Q-TR2 decided, so none is taken autonomously.
- The ball is in play for about an hour (class: implementation-detail; ac: "The ball is in play for about an hour"; classification: runtime-evidence): 66.22 at `b0`, inside the Q-TR2 limit of 68. Not shipped.
- Passes are realistic (class: implementation-detail; ac: "Passes are realistic"; classification: runtime-evidence): 493.6 and 85.28% at `b0`. Not shipped.
- Throw-ins stay in band (class: implementation-detail; ac: "Throw-ins stay in band"; classification: runtime-evidence): 52.95 at `b0`. Not shipped.
- Only real passes count (class: implementation-detail; ac: "Only real passes count"; classification: build-capability): built in `831553b`; the counting scenes pass at `b0`.

## Freshness Research

- No dependency was added or upgraded, and no code was committed.

## Recommended Next Stage

- **Option D: Blocked** → the product owner answers Q-TR3 in `po-answers.md`. Then rerun `/wf implement football-manager-match-engine tempo-and-restarts`: apply `b0-ship/b0-ship.patch` or a new setting, re-derive the moved fast tests, and run the criteria, the checks and the gate.
- **Option C: Revisit Plan** → `/wf plan football-manager-match-engine tempo-and-restarts` if the answer moves a criterion to another slice.
