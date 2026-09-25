---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: tempo-and-restarts
status: complete
stage-number: 5
created-at: "2026-09-25T04:27:37Z"
updated-at: "2026-09-25T12:44:00Z"
metric-files-changed: 18
metric-lines-added: 1068
metric-lines-removed: 27
metric-deviations-from-plan: 11
metric-review-fixes-applied: 0
commit-sha: "d6ccbb7483c78452271c12c5a2f3a361602bed07"
commits:
  - "831553b5b49baa7105b9ffdbd33e693661910809"
  - "4022e47a4c50e88e4564825d571238d7c80e511a"
  - "d6ccbb7483c78452271c12c5a2f3a361602bed07"
revision-count: 3
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
  - rev: 3
    at: "2026-09-25T12:44:00Z"
    trigger: answers-returned
    because: "Q-TR3 answered: ship the counting only; judge the slice on 'Only real passes count' and no regression of earlier criteria; passes, ball in play and throw-ins move to realism-tuning"
    changed: "the three moved slow tests are marked as later-tuning targets (d6ccbb7); earlier criteria, workspace checks and the full slice gate ran on the shipped play; no regression; status complete"
has-blockers: false
open-questions: []
steering-honored:
  - "Q-TR3 (steer.md, 2026-09-25): b0 is not shipped; the contest levers stay at the committed values that leave play unchanged; the slice is judged on 'Only real passes count' and on no regression of earlier criteria, not on passes, ball in play or throw-ins. No limit, band, card chance per foul, restart median, added time or tick rate changed."
  - "Heavy runs go to the server (2026-09-25): fmt, clippy, the workspace suite, every release slow test and every calibrate run ran on the server through the sync script; the two Windows-only tests ran on this PC."
  - "Q-I1: the slice gate ran the full suites: equal and strength on seeds 1, 7, 42, 99 and 2026, and formations on seed 42, 1,000 matches each."
  - "Q-KS1 / Q-LF1: the red-card criterion is not judged here; its slow test was run and recorded, and its figures equal the keeper-and-shots figures."
  - "VETO (2026-09-25): no work on realism-tuning was started; its slice and plan files were not touched."
  - "Output boundary: the code commit message uses product language only; a leak check found no workflow vocabulary in the diff."
  - "Design direction: not applicable; no page changed."
tags: [engine, passing, restarts, realism, defending]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-tempo-and-restarts.md
  plan: 04-plan-tempo-and-restarts.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/tempo-and-restarts/
  history: history/05-implement-tempo-and-restarts-2.md
  siblings: [05-implement-keeper-and-shots.md, 05-implement-lone-forward.md, 05-implement-tuning-loop.md, 05-implement-realism-bands-v2.md, 05-implement-defending-and-discipline.md]
  verify: 06-verify-tempo-and-restarts.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine tempo-and-restarts"
---

# Implement: Tempo and restarts

## The Implementation

Q-TR3 narrowed the slice to what was already committed. The pass counting, the ball-in-play figure, the clearances, the restart-delay values and the three contest levers stay in the engine at values that leave play unchanged. Setting `b0` is not shipped. The slice is now judged on two things: "Only real passes count", and no regression of the earlier criteria. The passes, ball-in-play and throw-in criteria move to `realism-tuning`. So this revision changes no play code. Commit `d6ccbb7` changes only the notes and skip reasons of the three moved slow tests, so a full slow run reads them as later-tuning targets and not as regressions.

Both judged criteria pass on the shipped tree. The counting scenes pass locally (10 passed). Every earlier slow criterion prints the same figures as `keeper-and-shots`: discipline 0.025 second yellows per match and 11.5% of matches with a sending-off, the stronger team 140 wins of 200, corners 1.205 per team and goal kicks 11.205 per match, 398 of 500 penalties. The slice gate ran 10 suites of 1,000 matches plus the formations suite. Against the keeper-and-shots verify gate on the same server, every statistic in every equal and strength report is identical except three: passes per team fall by 34–42 because clearances and restart kicks no longer count, pass accuracy moves by 0.23 points or less, and the new ball-in-play figure reads 89.3–89.5 minutes per 90. The formations suite (55,000 matches) is identical in the same way: 1,827 report keys, and only passes (1,341.9 → 1,306.6 per team), accuracy (89.942 → 89.938), the new figure (89.63) and run metadata differ, with 122 of 166 band misses as before.

`realism-tuning` can now tune on honest pass counts, and the `b0` patch is its starting evidence. The top open risk sits in that slice: at the shipped values the ball is in play about 89.4 minutes per 90, against a 52–65 band. The one setting measured near that band (`b0`) broke discipline and the stronger-team criterion.

## Summary of Changes

- This revision (`d6ccbb7`, test notes only):
  - `passes_are_realistic`, `the_ball_is_in_play_for_about_an_hour` and `throw_ins_stay_in_band` now have the skip reason "slow, and a target for later tuning that fails on the shipped values".
  - The file note says these three are targets for a later tuning pass and fail on today's tuning.
- Kept from earlier revisions: the counting and tempo levers (`831553b`) and the contest levers (`4022e47`), both at values that reproduce play. `b0` remains a patch in `implement-evidence/tempo-and-restarts/b0-ship/`.

## Files Changed

This revision:
- `crates/engine/tests/tempo_and_restarts.rs`: the module note and the skip reasons of the three moved slow tests (+6, −3). No assertion or limit changed.

The slice total, `bce0144..d6ccbb7` outside the workflow folder, is 18 files, +1,068 and −27. The earlier files are listed in `history/05-implement-tempo-and-restarts-0.md` and `history/05-implement-tempo-and-restarts-1.md`.

## Shared Files (also touched by sibling slices)

- None this revision. The slice as a whole shares `content/tuning.json`, `crates/engine/src/tuning.rs`, `crates/engine/src/sim.rs`, `crates/engine/src/decision.rs` and `crates/engine/src/rules/fouls.rs` with the earlier tuning slices (see the earlier revisions).

## Notes on Design Choices

- The three moved slow tests stay in this file and keep their limits. `realism-tuning` owns the criteria now, and the tests already measure them over the shared 200-match run. Moving them to another file would add churn and no signal.
- The skip reason says the tests fail on the shipped values, so a run with `--include-ignored` explains its three expected failures.
- No play value was touched. The committed play is identical to the play that `keeper-and-shots` verify accepted, so every earlier criterion can be compared number for number.

## Verification Seams Built

- Only real passes count → built in `831553b` and unchanged: `Kick::Clear` at `crates/engine/src/decision.rs:24`, the counting in `kick_ball()`, `Scene::clear` under the `scenario` feature, and the scenes `a_clearance_is_not_a_pass_and_is_never_completed` (`crates/engine/tests/tempo_and_restarts.rs:55`), `a_throw_in_a_corner_and_a_goal_kick_are_restart_kicks` (`:90`) and `the_kick_off_is_a_restart_kick_and_the_next_pass_is_a_pass` (`:118`). They enable `cargo test -p engine --all-features --test tempo_and_restarts` to observe the rule on the real engine.
- No regression of earlier criteria → the slow tests of the earlier slices and `engine-cli calibrate`, plus the compare script `.scratch/remote/tr3-cmp.py` (git-ignored). It flattens two calibrate reports and prints every key that differs. It enables a number-for-number compare against `verify-evidence/keeper-and-shots/vps/ks-verify/`.
- The moved criteria → `passes_are_realistic` (`crates/engine/tests/tempo_and_restarts.rs:449-450`), `the_ball_is_in_play_for_about_an_hour` (`:472-473`) and `throw_ins_stay_in_band` (`:485-486`) keep measuring for `realism-tuning`.

## Criterion Results (shipped play, this run)

| Criterion | Evidence | Result |
|---|---|---|
| Only real passes count | `cargo test -p engine --all-features --test tempo_and_restarts` on this PC: 10 passed, 4 ignored (`tr3/local-tests.txt`); the server release run: 11 passed with the census (`tr3/tr3-close/criteria.txt`) | pass |
| No regression: `every_formation_holds` | server release (`criteria.txt`) | pass |
| No regression: `discipline_is_realistic` | 0.025 second yellows per match, sending-off in 11.5% of matches, 0 same-tick pairs (as at `keeper-and-shots`) | pass |
| No regression: `a_stronger_team_wins_more_than_half_its_matches` | won 140, drew 33, lost 27 of 200 (as at `keeper-and-shots`) | pass |
| No regression: `set_pieces_arise_from_play` | 1.205 corners per team, 11.205 goal kicks per match (as at `keeper-and-shots`) | pass |
| No regression: other `keeper_and_shots` slow tests | 10 passed; penalties 398 of 500; 10.89 shots per team, on target 0.373 over the census | pass |
| No regression: `lone_forward`, `ai_trailing`, `mentality` | 4, 1 and 1 passed; mentality 6.62 defensive against 11.54 attacking shots per match | pass |
| Red-card criterion (owned by `realism-tuning`, recorded) | control 0.73 and 0.78; keeper arm 2.10 against 1.18, centre-back 0.83 against 0.77, striker 1.27 against 1.01, limit 1.17 | fails, as at `keeper-and-shots` (not a regression; not this slice's criterion) |
| Moved to `realism-tuning`: passes, ball in play, throw-ins | 1,201.0 per team at 86.65%; 89.43 minutes per 90; 23.16 throw-ins per match | not judged here (Q-TR3) |

The restart census at shipped play: throw-in median 3.0 s, goal kick 6.0 s, corner 8.0 s, free kick 8.0 s (sources 13.8, 23.2, 31.8 and 32.5 s); 12.49 fouls per match; corners from save or parry 0.815 and from blocks 0.390 per team, none from a clearance.

## Checks Run

- Server (`tr3/tr3-close/`): `cargo fmt --all -- --check` exit 0; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0; `cargo test --workspace --all-features --no-fail-fast` 518 passed, 2 failed, 14 ignored. The two failures are `data::tests::relative_paths_never_leak_the_root` and `a_small_run_writes_a_record_per_match_and_a_report_that_validate`, the Windows-only tests that steer.md names (`workspace-tests-tail.txt`).
- This PC: both Windows-only tests passed (`tr3/local-tests.txt`).
- Earlier criteria, server release (`cargo test --release -q -p engine --all-features --test tempo_and_restarts --test defending --test discipline --test keeper_and_shots --test strength --test lone_forward --test ai_trailing --test mentality --no-fail-fast -- --include-ignored --nocapture --test-threads 3`, `criteria.txt`): 2 targets failed, and both failures are expected. `defending` fails only `a_sending_off_gives_no_advantage` (the red-card criterion that `realism-tuning` owns, with the same figures as before). `tempo_and_restarts` fails only the three moved tests.
- Slice gate, server, 1,000 matches each (`tr3/gate/tr3-close/`, compare in `tr3/gate/compare-to-previous-gate.txt`):
  - `--suite strength`, seeds 1, 7, 42, 99, 2026: every run exit 0; stronger-team win rate 0.671, 0.677, 0.642, 0.673 and 0.648 (identical to the keeper-and-shots gate).
  - `--suite equal`, same seeds: exit 2 on each (the band-miss exit; recorded, not failed, Q-E4). Band misses are 6 of 16 on every seed, the same six as before: corners, goalless share, goals, passes, throw-ins and yellow cards. Goals per match 1.728, 2.016, 1.884, 2.135 and 1.702 (identical).
  - Against `verify-evidence/keeper-and-shots/vps/ks-verify/`: in every equal and strength report the only statistics that differ are passes per team (for example 1,225.4 → 1,191.0 on equal seed 42), pass accuracy (86.365 → 86.533) and the new `ball_in_play_min_per_90_mean` (89.35–89.51). The other differences are run metadata: wall time, content hash, run and owner ids.
  - `--suite formations --seed 42`: exit 2 (band misses recorded, not failed). Against the keeper-and-shots verify run, 11 of 1,827 report keys differ: passes per team 1,341.931 → 1,306.558, pass accuracy 89.942 → 89.938, the new `ball_in_play_min_per_90_mean` 89.63, and run metadata (wall time, content hash, run and owner ids). Band misses are 122 of 166, the same as before (`tr3/gate/compare-formations-to-previous-gate.txt`). The whole gate took 12,369 s after the checks (`tr3/gate/status.txt`).
- Benchmark: not re-run. This revision changes no engine code, and the tree's engine code equals `4022e47`, where three drives on this PC gave 1.5847–1.5966 µs per tick against the 1.7071 µs gate and 6.81–6.97 MB against 8.55 MB (`implement-evidence/tempo-and-restarts/bench2/`). Verify owns the compare.
- Page tests: not run. No page changed.
- Output-boundary search of the diff and the commit message for workflow vocabulary: clean.
- Commit hygiene (recovered): the first commit of the test file (`7e27394`) also took three design documents under `docs/design/realism/` that another session had already staged. It was undone with `git reset --soft HEAD~1`, which kept that session's staging as it was. The file was then committed alone with `git commit --only` as `d6ccbb7` (1 file, +6 −3). `7e27394` is on no branch.

## Deviations from Plan

1. The deviations of the earlier revisions stand (`history/05-implement-tempo-and-restarts-0.md`, `history/05-implement-tempo-and-restarts-1.md`). (class: implementation-detail)
2. `b0` was not shipped (rev 2), as Q-TR2's stop condition required. (class: implementation-detail)
3. Plan steps 9, 10 and 12 (the inner tuning loop, the criterion runs on a tuned setting, re-derived pins) end without a shipped tuned setting. Q-TR3 moves the tuned criteria to `realism-tuning`, so no pinned expectation moves: shipped play is identical to `keeper-and-shots`. The four restart delays stay at 3, 8, 6 and 8 s, not at the sourced medians the plan named, because moving them changes play, and Q-TR3 ships only what leaves play unchanged. (class: implementation-detail — this follows the product owner's answer; it settles nothing new)

## Anything Deferred

- Tuning passes, ball in play and throw-ins into band, the restart delays toward their medians, and the contest levers: `realism-tuning` (Q-TR3). Its starting evidence is `implement-evidence/tempo-and-restarts/b0-ship/` and the sweeps in `contest/`.
- The cross clearance produces no corner at shipped play (0 corners from a clearance). `realism-tuning` owns the corners band.

## Known Risks / Caveats

- At shipped play the ball is in play about 89.4 minutes per 90 and throw-ins are 23.2 per match. The figures are honest now, but out of band until `realism-tuning`.
- When a setting like `b0` ships later, nine fast tests move (listed in `history/05-implement-tempo-and-restarts-2.md`, Known Risks) and must be re-derived there.
- `RUNNING_SPEED` (2 m/s) is a literal in `sim.rs`. It only matters when `tackle_dribble_win` is above 0.
- The engine's `stats.passes` now counts open-play passes only. A reader that compares pass counts with records made before `831553b` sees about 34–42 fewer passes per team for the same play.

## Assumptions

- A-I8 (class: implementation-detail): "no regression of the earlier criteria" is shown by running each earlier slow criterion on the shipped tree and by comparing the gate reports with the keeper-and-shots verify gate on the same server. An earlier criterion that already failed there (the red-card criterion, owned by `realism-tuning`) is a regression only if its figures moved. They did not.
- A-I9 (class: implementation-detail): the three moved slow tests stay in the test file with their limits, and only their notes change. `realism-tuning` decides where they live.
- A-I11 (class: implementation-detail): the slice definition's acceptance-criteria list is not rewritten. The move is recorded where earlier moves (Q-KS1, Q-KS2, Q-LF2) were recorded: in `po-answers.md`, `steer.md` and this record. Verify reads `steer.md`, which judges this slice on "Only real passes count" and no regression.
- A-I10 (class: implementation-detail): the benchmark is not re-run, because the engine code equals `4022e47`, where it was measured.

## Triage Decisions

- Only real passes count (class: implementation-detail; ac: "Only real passes count"; classification: build-capability): built in `831553b`, proven by the scenes on this PC and on the server. Pass.
- No regression of the earlier criteria (class: implementation-detail; ac: "no regression of the earlier criteria (Q-TR3)"; classification: runtime-evidence): every earlier slow criterion gives the keeper-and-shots figures, and the gate reports match number for number except the pass counts and the new figure. Pass.
- Passes are realistic, The ball is in play for about an hour, Throw-ins stay in band (class: implementation-detail; classification: moved-by-po): not judged in this slice. The product owner moved them to `realism-tuning` (Q-TR3). The move is recorded, not decided, here.

## Freshness Research

- No dependency was added or upgraded.

## Recommended Next Stage

- **Option A (default): Verify** → `/wf verify football-manager-match-engine tempo-and-restarts`. Verify the counting scenes and the no-regression evidence on the shipped tree. The benchmark compare belongs there. Consider compacting the session before `/wf verify`. Workflow state lives in artifact files on disk, and the SessionStart hook re-reads it after compaction.
- **Option B: Skip to Review** → `/wf review football-manager-match-engine tempo-and-restarts`. Not recommended: the counting is testable behaviour.
- Note: steer.md's VETO ends the run when this slice is complete. It forbids any work on `realism-tuning` and the slug-wide review until the product owner removes it.
