---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: tuning-loop
status: complete
stage-number: 5
created-at: "2026-09-24T10:21:31Z"
updated-at: "2026-09-24T10:21:31Z"
metric-files-changed: 20
metric-lines-added: 2171
metric-lines-removed: 129
metric-deviations-from-plan: 6
metric-review-fixes-applied: 0
commit-sha: "a6dc5d24bf3eb8080ca82b69fbca17b5e7325eb5"
commits:
  - "a6dc5d24bf3eb8080ca82b69fbca17b5e7325eb5"
tags: [calibration, tooling, realism, performance]
steering-honored:
  - "Moved criteria and slice order (2026-09-24): the red-card criterion stays with lone-forward. This slice moves the experiment into calibrate and reports its verdict; it does not judge or change the criterion, its 1.6 limit, or any card value."
  - "A targeted calibrate run is an inner loop (Q-I1): the how-to ends with the full gate, five seeds for equal and strength and one seed for formations. --suite all is unchanged and does not include red-card."
  - "Output boundary: code comments, commit text, docs, and test names use product language; the commit message was leak-checked (no match) before commit."
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-tuning-loop.md
  plan: 04-plan-tuning-loop.md
  siblings: [05-implement-realism-bands-v2.md, 05-implement-defending-and-discipline.md, 05-implement-calibration.md, 05-implement-experiment-flags.md]
  verify: 06-verify-tuning-loop.md
  evidence: implement-evidence/tuning-loop/
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine tuning-loop"
---

# Implement: Tuning loop

## The Implementation

Before this change, calibrate always played every fixture of each suite it named: a seed of every suite was 57,000 matches, and the red-card experiment lived only in a 120-seed slow test. Two runs were compared by eye. The plan was complete, with the product owner's Q-P1 answer: a baseline must share the fixtures hash, and a changed content hash is the change under test.

Calibrate now takes `--pairing`, `--band`, `--baseline` and `--suite red-card`. A selected pairing keeps its place in the full fixture list, so its seeds, clubs and home sides are the full run's. Every band check carries a sampling error, and the report carries a `fixtures.hash`. The baseline guard runs before the run folder exists. The diff prints each band's change with its error and marks a change inside two errors as noise. The red-card suite sends a player off through one new public engine function that the test scene builder now calls too. On the 8-core reference machine, one pairing at 1,000 matches with its diff took 87 s, the equal suite 86 s, and the red-card experiment 42 s. The targeted pairing and equal suite matched a full seed-42 run exactly (81 minutes). The red-card figures matched the slow test on all 9 values to 4 decimals. A fat-LTO build and a native-CPU build gave identical figures but were not measurably faster, so no profile was kept. There are 20 files in commit `a6dc5d2`, and 479 workspace tests pass.

`lone-forward` can now measure each change to its two pairings and the red-card criterion in under 2 minutes. The top open risk is misreading noise: a share band with few events has a wide error. The diff prints that error, and the how-to ends with the full gate.

## Summary of Changes

- Targeted runs: `--pairing "A v B"` (either order, repeatable) plays only those formation pairings. `--band NAME` (repeatable) plays only the suites that check the band and judges only the named bands; each suite's time budget stays. The report's `calib.selection` names what was played.
- Agreement by construction: the selected fixtures are the same slice of the full list, with the same index, seed, clubs and home side (`formation_fixtures_for`).
- Sampling error on every band row (`se`): means use sd/sqrt(n), shares use sqrt(p(1-p)/n), pooled ratios use the ratio estimator, the stronger-team win rate uses the binomial error, and each pairing row uses its own matches. `wall_ms` gets 0.
- Fixtures hash: SHA-256 over the parsed attributes, rules and tactics content, the generator block after flag states, and the two default clubs, written as `fixtures.hash`.
- Baseline guard and diff: a report that is not a calibrate run report, has no `se`, has no `fixtures.hash`, or differs in seed, match count or fixtures hash is refused before any match, with exit 1, naming each difference. The diff table goes to standard error and `calib.baseline` and `calib.diff` go into the report. A changed content hash is shown as `content changed`.
- Red-card suite: four arms (the control, and the away keeper, centre-back or striker sent off at kick-off) on the default clubs with cards otherwise off. The engine seeds are `--seed .. --seed + matches - 1`. The suite adds `calib.red_card` and two band rows per arm. It is not part of `--suite all`.
- Engine: `Simulation::send_off_before_kickoff`, public. `Scene::sent_off` delegates to it. A fast parity test runs 3 seeds on 3 arms, and the slow test now prints 4 decimals.
- Contract: additive keys in `run-report.schema.json`, and the run-report paragraph of `.ai/observability.md`. `schema.version` stays `"1"`.
- Docs: the calibrate section of `docs/reference/cli.md`, and a new `docs/how-to/calibration.md`.
- Build profile: measured, not kept.

## Files Changed

- `crates/engine/src/sim.rs`: `send_off_before_kickoff(i)`. It drops the carrier, sends the player off, sets the manager review, and restarts the team timeline at the current tick.
- `crates/engine/src/scenario.rs`: `Scene::sent_off` calls the engine function, so there is one code path. It also drops an unused import.
- `crates/engine/tests/defending.rs`: a parity and validator test for the public send-off; the slow test prints 4 decimals.
- `crates/engine-cli/src/cli.rs`: `--pairing`, `--band`, `--baseline`, the `red-card` suite value, and the hidden worker flag `--pairing-numbers`.
- `crates/engine-cli/src/calibrate/mod.rs`: selection (`select`, `pairing_number`), `fixtures_hash`, the baseline check before the run folder, the jobs clamp over the selected suites (red-card counts 4 x matches), red-card planning and reading back, the band filter, the diff output, and unit tests.
- `crates/engine-cli/src/calibrate/fixtures.rs`: `formation_fixtures_for` and `red_card_fixtures`, with unit tests.
- `crates/engine-cli/src/calibrate/worker.rs`: plays the selected pairings, and plays the red-card match on the default clubs with cards off and the send-off.
- `crates/engine-cli/src/report/mod.rs`: `Suite::RedCard`, `RED_CARD_ARMS`, `band_suites`, `BandCheck.se` (and `Deserialize`), `RedCardFigures`, the sampling-error helpers, `Selection`, the new report fields, and unit tests.
- `crates/engine-cli/src/report/baseline.rs` (new): the guard, the diff rows and noise mark, the table, and unit tests.
- `crates/engine-cli/src/report/compare.rs`: its test helper sets `se`.
- `crates/engine-cli/Cargo.toml`, `Cargo.lock`: `sha2` from the workspace (already in the lock file; one dependency line).
- `crates/engine-cli/tests/calibrate_targeted.rs` (new): 7 integration tests (agreement, band narrowing, selection refusals, baseline refusals, rerun noise, content change accepted, red-card suite).
- `crates/engine-cli/tests/calibrate.rs`: the smoke run checks `se`, `fixtures.hash`, `calib.selection`, and that `--suite all` has no red-card suite.
- `crates/engine-cli/tests/cli_args.rs`: the new hidden worker flag stays out of the help.
- `crates/engine-cli/tests/docs.rs`: the how-to joins both document lists.
- `schemas/observability/run-report.schema.json`: `red-card` in the three suite enums; `se` on band items; top-level `fixtures.hash`; `calib.selection`, `calib.red_card`, `calib.baseline` and `calib.diff`.
- `.ai/observability.md`: the run-report key list.
- `docs/reference/cli.md`: the new flags, the red-card suite, targeted runs, `se`, the baseline rule, the refusal messages and the diff columns.
- `docs/how-to/calibration.md` (new): make a baseline, run a pairing, suite or band, read the diff, run the red-card experiment, run the full gate, and the profile measurement.

## Shared Files (also touched by sibling slices)

- `calibrate/*`, `report/*`, `cli.rs` and the run-report schema were created by `calibration`, `experiment-flags`, `probe-engine-core` and `realism-bands-v2`. All of them are verified. The paired-run path (`--pair`) is unchanged, and `--baseline` is refused with it.
- `scenario.rs` and `tests/defending.rs` come from `defending-and-discipline` at `f7fe35b`. The scene builder's send-off behaviour is unchanged, and its tests pass.
- `docs/reference/cli.md` is shared by every command's docs. Only the calibrate section changed.

## Notes on Design Choices

- The red-card arm rides in the band row's `pairing` field, so `--band reduced_minus_full` selects all three arms. The existing compare and diff code pair rows by (suite, band, pairing) with no change.
- `reduced_minus_full` shows `lo` = -10 for display only; only its top (0) is judged. JSON cannot hold an infinite floor.
- The fixtures hash length-prefixes each part, so moving bytes between two parts cannot collide.
- `--pairing` narrows the run to the formations suite, as `--band` narrows to its suites. With a suite list that has no formations suite, it is refused.
- `se` is an optional schema property (additive contract), but every band row the writer makes carries it, and the guard refuses a baseline without it.
- The diff error treats the two runs as independent: sqrt(se_b^2 + se_n^2). This is conservative on shared fixtures and needs no per-match data from the baseline folder.

## Verification Seams Built

- "A targeted run is fast" → `calib.wall_ms` per suite plus shell wall time; `send_off_before_kickoff` at `crates/engine/src/sim.rs:563` makes the red-card suite runnable in a release binary (enables the headless release runs in `implement-evidence/tuning-loop/measure.sh`).
- "A targeted run plays only its selection" → `calib.selection` at `crates/engine-cli/src/calibrate/mod.rs:178`, and the selected fixtures at `crates/engine-cli/src/calibrate/fixtures.rs:89`. Enables counting `stats/` files and reading `calib.formations`.
- "A targeted run agrees with the full run" → full-list indices kept by `formation_fixtures_for` at `crates/engine-cli/src/calibrate/fixtures.rs:89`. Enables JSON equality against a `--suite all` report.
- "The diff separates change from noise" → `BandCheck.se` at `crates/engine-cli/src/report/mod.rs:207`, and `diff` at `crates/engine-cli/src/report/baseline.rs:154`. Enables `calib.diff` and the standard-error table.
- "A wrong baseline is refused" (restated by Q-P1 as "a baseline with different fixtures is refused") → `fixtures_hash` at `crates/engine-cli/src/calibrate/mod.rs:437`, the report key at `crates/engine-cli/src/report/mod.rs:991`, and the guard at `crates/engine-cli/src/report/baseline.rs:74`, called before the run folder at `crates/engine-cli/src/calibrate/mod.rs:134`. Enables the exit code, the message and the empty `stats/` check.
- "The red-card experiment is a suite" → `red_card_figures` at `crates/engine-cli/src/report/mod.rs:515`, the rows at `crates/engine-cli/src/report/mod.rs:550`, and the worker send-off at `crates/engine-cli/src/calibrate/worker.rs:226`. Enables comparison with the slow test's 4-decimal output.
- "A faster profile changes no result" → none needed beyond `--baseline`: a candidate is identical when its diff against the release run is 0 on every row and the figure objects are equal (`implement-evidence/tuning-loop/analyse.py`).

## Visual Contract Honored

Not applicable. This slice changes no page. `02c-craft.md` covers viewer slices only, and every criterion here is read from a report, standard error, an exit code or a file count.

## Measurements (reference machine, release build, this run)

Source: `implement-evidence/tuning-loop/measure.txt` and `analysis.json`, from runs made after the commit's code was built. The machine is Windows 11 with 8 logical cores (`nproc` = 8).

| Run | Wall (shell) | Suite wall | Matches/s | Result |
|---|---|---|---|---|
| `--suite all --seed 42 --matches 1000` (baseline) | 4869 s | total 4,780,549 ms | ~12 | exit 2 (band misses, as before) |
| `--suite formations --pairing "4-4-1-1 v 4-4-2" --seed 42 --matches 1000 --baseline full` | 86.7 s | 79,471 ms | 12.58 | 1000 stats files; 1 pairing; diff 3 rows, all noise, change 0 |
| `--suite equal --seed 42 --matches 1000 --baseline full` | 85.8 s | 78,528 ms | 12.73 | diff 15 rows, all noise, change 0 |
| `--suite red-card --seed 1 --matches 120 --baseline red-base` | 41.9 s | 36,297 ms | 13.22 | 480 stats files; diff 6 rows, all noise |

- Agreement: the targeted pairing's `calib.formations` entry, its band rows, the equal suite's figures (without the wall-time-dependent `outliers` count) and its band rows equal the full run's. `fixtures.hash` is `c4c0a247c3c9` in all three reports. Validator violations are 0 in every run.
- Red-card and slow test: the slow test (`cargo test --release -p engine --all-features --test defending a_sending_off_gives_no_advantage -- --ignored`) printed control 0.7750 / 2.2333; keeper 2.0583 / 5.8167; centre-back 0.7083 / 2.3917; striker 0.9500 / 3.8417; limit 1.2400. `calib.red_card` holds the same 9 values. The criterion itself fails (the reduced side outscores the full side in every arm). That was already true, and the criterion belongs to `lone-forward` (steer.md, Q-I2). This slice reports the verdict and does not change it.
- Build profiles, on the same pairing at 1,000 matches: (a) release 12.58 and 12.20 matches/s on two runs; (b) `lto = "fat"`, `codegen-units = 1`: 12.69; (c) (b) plus `-C target-cpu=native`: 12.53. On the red-card suite: (a) 13.22, (b) 13.30, (c) 13.18. Both candidates gave a diff of exactly 0 on every row, and equal `calib.suites`, `calib.formations` and `calib.red_card` objects. The largest gain was +0.9%, well inside the 3% spread between two release runs. So no profile is faster, the current profile stays, and `Cargo.toml` is unchanged.

## Gates (this run)

- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- `cargo test --workspace --all-features`: 63 test binaries, 479 passed, 0 failed, 7 ignored (the slow tests).
- The per-tick benchmark was not re-baselined. No per-tick code changed; the new engine function runs only before kick-off.

## Deviations from Plan

1. The hidden worker flag is `--pairing-numbers` (comma list), not `--pairings`. This avoids a name that reads as the plural of the public `--pairing`. The help test keeps it hidden.
2. `sha2` was added to `crates/engine-cli/Cargo.toml` as a workspace dependency. The plan said "reuse `sha2::Sha256`", but engine-cli had no direct dependency on it. `Cargo.lock` gains one line and no new crate.
3. The error for `pass_accuracy_pct` uses the error of a mean, not the ratio estimator. The figure is the mean of each team's percentage (`report/mod.rs` `pass_accuracy_pct_mean`), not a pooled ratio, so the mean's error is the right one. `shots_on_target_share` and `goals_per_xg` are pooled ratios and use the ratio estimator as planned.
4. `--band` keeps each suite's `wall_ms` row. The time budget is not a realism band, and dropping it would stop a run's `calib.pass` from checking time. The diff leaves it out in any case.
5. `--pairing` with `--suite all` narrows the run to the formations suite, as `--band` narrows. It is refused only when the requested suites have no formations suite, as planned.
6. No `[profile.calibrate]` is committed. Step 13's measurement found no candidate faster beyond run-to-run noise. The trial profile was added to build candidates (b) and (c) and then removed. This is the outcome the plan and the criterion allow ("the measurement is recorded and the current profile stays").

## Anything Deferred

- None from the plan. Every step, 1 to 14, was done.
- `sdlc-debt:` markers: none. No suppression and no deliberate shortcut was added.

## Known Risks / Caveats

- Match minutes are not part of the baseline identity. Q-P1 lists the exact inputs (seed, match count, fixtures), and `--minutes` is not among them. A baseline at other minutes is accepted, and its diff would compare different matches. Every how-to command uses the default 90.
- The strength suite's boost comes from `realism-bands.json` (`stronger_team.attribute_boost`). It changes the boosted clubs but is not in the fixtures hash, because Q-P1 lists the inputs and the bands file is not among them.
- The sampling error of a share band with few events is wide, for example 0.0067 for the targeted pairing's goalless share. The diff prints it. A tuning slice still confirms a gain on the full gate (RIM-11).
- The red-card suite reports a failing criterion (exit 2) until `lone-forward` fixes it. That is the product state, not a tool fault.

## Assumptions

Autonomous run: no product owner was present. Each decision below is an implementation detail within the plan and the product owner's Q-P1 answer. None touches a carried intent risk: RIM-11 is adjudicated, and this build follows its decision.

- D1 (class: implementation-detail; ac: "A targeted run is fast"; classification: runtime-evidence): measured headless on this machine, the reference machine, with the release binary. Pairing 86.7 s, equal 85.8 s, red-card 41.9 s, each including its diff.
- D2 (class: implementation-detail; ac: "A targeted run plays only its selection"; classification: runtime-evidence): 1000 `stats/` files and one `calib.formations` entry in the release run, plus the `--matches 2` regression test.
- D3 (class: implementation-detail; ac: "A targeted run agrees with the full run"; classification: runtime-evidence): JSON equality against a fresh full seed-42 run made in this run (81 minutes), plus the `--matches 2` regression test.
- D4 (class: implementation-detail; ac: "The diff separates change from noise"; classification: runtime-evidence): release diffs of 3, 15 and 6 rows, all noise at change 0 on the same content; unit tests of the noise boundary.
- D5 (class: implementation-detail; ac: "A wrong baseline is refused"; classification: build-capability): the guard refuses on seed, match count and fixtures hash, and allows a changed content hash, per Q-P1. Integration tests cover each refusal and the accepted content change.
- D6 (class: implementation-detail; ac: "The red-card experiment is a suite"; classification: runtime-evidence): `--suite red-card --seed 1 --matches 120` equals the slow test's figures to 4 decimals.
- D7 (class: implementation-detail; ac: "A faster profile changes no result"; classification: runtime-evidence): identical figures for both candidates, and no rate rise beyond the 3% run-to-run spread. The current profile stays, and the measurement is recorded.
- D8 (class: implementation-detail): a gain inside the timing noise of two release runs is not "faster". Only a Cargo profile could be committed in any case; a native target never is.
- D9 (class: implementation-detail): the red-card arm is carried in the band row's `pairing` field, and the display floor of `reduced_minus_full` is -10.
- D10 (class: implementation-detail): the deviations 1 to 5 above (flag name, the direct `sha2` dependency, the pass-accuracy error, `wall_ms` kept under `--band`, and `--pairing` narrowing).
- D11 (class: implementation-detail): the content-change acceptance test declares a flag in a copied content folder, because the shipped `tuning.json` declares no flags. The flag's override (`engine.shot_range`) leaves the generator alone, which is exactly the case the plan names.
- D12 (class: implementation-detail): research ran inline. No sub-agent tool was used, and the plan's cited code was re-read before editing. The only commit after the plan touches workflow records.
- D13 (class: implementation-detail): the previous driver was presumed dead. The plan on disk is `status: complete`, the index's `selected-slice` is `tuning-loop` with `next-command: wf-implement`, and the working tree had no code change under `crates/`, `schemas/` or `Cargo.toml` before this run. No state contradicted the index.
- D14 (class: implementation-detail): the staged files under `docs/design/realism/` are not this change's. They were left staged and out of the commit, which used explicit paths.

## Triage Decisions

- The slow test `a_sending_off_gives_no_advantage` fails its assertion, as it did before this change. This is not fixed here: the criterion moved to `lone-forward` (steer.md, 2026-09-24), and RIM-12 forbids tuning it here. class: implementation-detail.

## Fix Status

No review findings exist for this slice yet. Nothing to fix.

## Verify-Owned Fixes

None.

## Freshness Research

- Cargo Book, "Profiles" (the plan's research, still current for the toolchain, rustc 1.92.0): custom profiles take `inherits`, `lto` and `codegen-units`, but not `rustflags`. The candidate (b) profile built with these keys in 2 m 03 s. Candidate (c) used `RUSTFLAGS`. Takeaway: only (b) could have been committed.
- No dependency was added or upgraded. `sha2` 0.10 is the workspace's existing version, as the engine crate already uses it.

## Recommended Next Stage

- **Option A (default): Verify** → `/wf verify football-manager-match-engine tuning-loop`. Every criterion is testable behaviour of the calibrate command on this machine, and the evidence in `implement-evidence/tuning-loop/` can be re-run. Consider compacting the session before verify; the workflow state lives in the artifact files on disk.
- **Option B: Skip to Review** → `/wf review football-manager-match-engine tuning-loop`. Not recommended, because the slice has testable runtime behaviour.
- **Option C: Revisit Plan** → `/wf plan football-manager-match-engine tuning-loop`. Not needed: the plan held, with the six minor deviations recorded above.
