---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: tactics-and-ai
status: complete
stage-number: 6
created-at: "2026-09-22T23:38:05Z"
updated-at: "2026-09-22T23:38:05Z"
result: pass
metric-checks-run: 16
metric-checks-passed: 16
metric-acceptance-met: 9
metric-acceptance-total: 9
metric-acceptance-user-observable: 1
metric-acceptance-code-only: 8
metric-interactive-checks-run: 1
metric-interactive-checks-passed: 1
metric-issues-found: 0
metric-issues-found-initial: 2
metric-issues-found-final: 0
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: "e79ad61"
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/tactics-and-ai/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped"
ac-staleness-checked: false
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 0
adversarial-tests-run: 0
adversarial-tests-failed: 0
failure-mode-probes-run: 0
cross-browser-delta: "none"
web-vitals-lcp-ms: null
web-vitals-cls: null
web-vitals-inp-ms: null
stack-source: confirmed
debt-markers-found: 0
debt-markers-malformed: 0
debt-markers-unrecorded: 0
skipped-gating-specs: []
consult-runs: []
steering-honored:
  - "No visual change to the page: the one fix replaced the placeholder note text in the empty Lineups panel with the wording the other empty panels already use; elements, classes, and --tl- tokens are unchanged."
  - "Output boundary: the fix commit message and the changed comments use product language; a vocabulary search over shipped files returns nothing after the fix."
tags: [engine, tactics, ai-manager, fatigue, injuries, substitutions, benchmark, milestone]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-tactics-and-ai.md
  plan: 04-plan-tactics-and-ai.md
  implement: 05-implement-tactics-and-ai.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-tactics-and-ai.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine tactics-and-ai"
---

# Verify: Tactics, Fatigue, and the AI Manager

## The Verification

Implement handed over commit `0e9cf6a` and its record commit `77af778`: a scored-options decision layer, a stoppage-gated change queue, fatigue, injuries, the substitution limit, and an AI manager. The slice carries nine criteria. Eight are cargo tests, and three of those are slow release runs. The benchmark report is the one user-observable criterion. Cargo serialises on its target folder and the benchmark needs an idle processor, so this run did every check in sequence, with no sub-agents.

Every criterion passed on the first run. The default suite passed 224 tests with 0 failed and 3 ignored. The slow runs gave the same counts as implement: the stronger team won 139 of 200, a trailing AI team changed its tactics in 100 of 100 scenes, and an attacking team took 33.69 shots per match against 2.15 when defensive. The benchmark read 434.4 ms of processor time per match (median of 431.4, 434.4, 434.4) against the 460.7 ms limit, and 6.14 MB peak memory against 6.82 MB. The one failing check was the workflow-vocabulary search that plan step 22 requires. It found three lines: one pre-existing placeholder note in `web/index.html`, and two comments in `crates/engine/src/tuning.rs` that name a planning phase. Both issues were triaged `Fix` autonomously. The fix is commit `e79ad61`, which changes text only, and the re-checks pass.

Review is next. The top open risk is the size of the mentality effect: 33.69 shots per match when attacking is far above real football, and calibration must narrow it. The processor margin is 5.7 percent under the per-match limit.

## Verification Summary

- Result: `pass`. 9 of 9 criteria met. The one user-observable criterion (AC-9) has live runtime evidence on the reference machine.
- Convergence: `converged`. Two issues (VOCAB-1, VOCAB-2), both triaged `Fix` under the autonomous policy, both patched and re-checked. `fix-rounds-run: 1`.
- Adapters: `cli`, the release binary driven directly (`cli-direct`) for AC-9, the benchmark compare, and one full-match probe.
- Sub-agents: none. This run has no sub-agent dispatch tool, and every check needs the one cargo target folder, so the coordinator ran each check and applied the text-only fix itself.
- Benchmark compare mode: recorded in `05c-benchmark.md` and `05c-benchmark.yaml`. Seven targets compared, and no tripwire fired.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (exit 0; `check-fmt.*`; after the fix `recheck-fmt.txt`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass (exit 0; `check-clippy.*`; after the fix `recheck-clippy.txt`).
- `cargo test --workspace`: pass. 224 passed, 0 failed, 3 ignored, 37 test binaries (`check-test.stdout.txt`, `check-test.exit-code`, `check-test.wall-ms.txt`).
- Criterion test files re-run one by one: pass (`criterion-tests.txt`): `tactics_queue` 2, `substitutions` 5, `fatigue` 3, `injury` 2, each exit 0.
- `cargo test --release -p engine -- --ignored`: pass (exit 0, 49.5 s; `slow-ac-6-7-8.*`). Each slow test was run again with `--nocapture` to record its counts (`slow-strength.*`, `slow-ai_trailing.*`, `slow-mentality.*`).
- `node --test web/tests/*.test.mjs`: pass (47 of 47; `check-node-test.stdout.txt`; after the fix `recheck-node-test.txt`).
- `cargo build --release -p engine-cli`: pass (exit 0, 7,617 ms, 0 warnings; binary 4,046,848 bytes; `check-build-release.*`).
- `gitleaks detect --log-opts="837cb5c..HEAD"`: pass ("no leaks found"; `check-gitleaks.*`).
- Secret pattern grep over the added lines of `git diff 837cb5c..HEAD` outside `.ai/`: pass (0 matches; `check-secret-grep.txt`).
- Dependency check: pass. `git diff 837cb5c..HEAD -- Cargo.lock` is empty (`check-lockfile-diff.txt`), so no crate was added or changed. `cargo-audit` is not installed.
- `sdlc-debt:` marker hygiene: pass. 0 added or removed in the slice diff, 0 in `crates/` and `web/` (`check-debt-markers.txt`, `check-debt-markers-tree.txt`).
- Workflow vocabulary search over `crates/`, `web/`, `docs/reference/`, `content/`, `README.md` (plan step 22 gate): fail on the first run, 3 lines after code uses of `.slice(` and byte slices were filtered out (`check-vocabulary.txt`, `check-vocabulary-diff.txt`). Pass after the fix, 0 lines (`recheck-vocabulary.txt`).
- Instrumentation check against `04b-instrument.md` §2: pass. All nine planned signals exist (`check-instrumentation.txt`). Details under Augmentation Verification.
- `engine-cli bench --seed 42 --matches 5 --json`, three drives: pass (AC-9; `ac-9-{1,2,3}.*`).
- `engine-cli bench --seed 42 --matches 1 --stream --json`, three drives: pass (`bench-stream-{1,2,3}.*`).
- `cargo bench -p engine --bench tick_step`: pass (exit 0; `criterion.*`).
- Full-match probe, `engine-cli simulate --seed 42`: pass (exit 0; `simulate-42.*`). The result has 0 `validate.violations`, 0 `darkpath.change_never_applied`, 10 changes queued and 10 applied, `fatigue.mean_pct` 51.26, and shots and substitutions for both teams.

## Interactive Verification Results

- **Criterion:** AC-9 "Given the benchmark reruns after this slice, Then CPU time per match is within 10 percent and memory within 25 percent of the match-rules baseline."
  - **Platform & tool:** cli, the release binary driven directly (`cli-direct`) on the reference machine (AMD Ryzen 7 9800X3D, Ultimate Performance plan, machine hash `74ca12fc08a4`), Git Bash, `SM_DATA_DIR` set to a scratch folder.
  - **Steps performed:** built the release binary. Ran `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` three times, and captured stdout, stderr, and the exit code for each run.
  - **Evidence:** `verify-evidence/tactics-and-ai/ac-9-{1,2,3}.stdout.txt`, `.stderr.txt`, `.exit-code`.
  - **Observation:** processor time per match (`bench.cpu_ms / bench.matches`) 431.4, 434.4, 434.4 ms. The median is 434.4 ms, +3.7 percent against the 418.8 ms baseline and under the 460.7 ms limit. `bench.peak_mem_mb` 6.160, 6.145, 6.133. The median is 6.145 MB, +12.8 percent against 5.45 MB and under the 6.82 MB limit. `bench.match_wall_ms` 432, 435, 432 (budget 2000). `bench.cpu_wall_ratio` 0.996 to 1.002. `budget.pass` true on each run. Exit 0 on each run. `record.kind` `run-report`. Reported beside the gate: `bench.cpu_us_per_tick` 1.446, 1.456, 1.456 (baseline 1.4965), and `bench.ticks_per_match` 298,350 (baseline 279,850). `build.hash` reads `77af778-dirty`. The `-dirty` suffix comes from untracked workflow files, the staged documents of other work, and a staged test file of other work that is deleted in the working tree. None of these is engine source. The drives ran before the text-only fix `e79ad61`, which changes no code. The three drives agree within 0.7 percent, so `stability: stable`. The scratch data folder held only `owner.id` afterwards.
  - **Result:** pass. Every pass criterion in the plan holds.

The eight other criteria are annotated `observable: false`.

## Acceptance Criteria Status

- **AC-1** "Given a queued tactics change, When the next dead ball occurs, Then the change applies at that tick and an event records it; and Given the same queued change with no dead ball, Then it has not applied." kind: `code-only`. status: met. method: automated. evidence: `tactics_queue.rs` 2 of 2. `a_change_waits_in_open_play_and_applies_on_the_stoppage_tick` asserts that the change is pending, with no verdict and the mentality unchanged, on every open-play tick. It then asserts one `ChangeApplied` event on the throw-in tick, with `applied.tick == restart.tick`. The penalty test proves that a change waits through a stoppage that the rule pack does not admit. evidence-rung: n-a.
- **AC-2** "Given five substitutions used across three windows, When a sixth is queued, Then it is rejected with a reason event; and Given four used, When a fifth is queued, Then it applies at the next dead ball." kind: `code-only`. status: met. method: automated. evidence: `substitutions.rs`. `a_sixth_substitution_is_rejected_with_the_limit` gives a `ChangeRejected` event with the reason "substitution limit reached (5 of 5)", and the player stays on. `a_fifth_substitution_applies_at_the_next_dead_ball` (four used over two windows, the plan's recorded reading) gives an applied change and a substitution event, and the roster slot carries the substitute. The half-time and no-window-left cases also pass. evidence-rung: n-a.
- **AC-3** "Given a substitution and a role change queued for the same player, Then the substitution applies and the role change is rejected with a reason." kind: `code-only`. status: met. method: automated. evidence: `a_substitution_and_a_role_change_for_the_same_player_in_either_order`. In both queue orders, the substitution applies first. The role change is rejected with `LeftThePitch`, whose text names the player, and the tactics stay unchanged. evidence-rung: n-a.
- **AC-4** "Given a player's fatigue passes the tuning threshold, Then that player's pace and decision attributes degrade per the curve." kind: `code-only`. status: met. method: automated. evidence: `fatigue.rs` 3 of 3. At energy 1.0, 0.9, and 0.7, the effective values equal the base values. At 0.5 and 0.3, the multiplier equals the curve points 0.92 and 0.82, and `max_speed`, `decisions`, `passing`, and `finishing` each equal the base value times the multiplier. A full match ends with every player's energy between 0 and 1. evidence-rung: n-a.
- **AC-5** "Given an injury event, Then the injured player leaves play, and the AI manager queues a substitution that applies within the next window when one is available." kind: `code-only`. status: met. method: automated. evidence: `injury.rs` 2 of 2. A scripted tackle injury gives an `Injury` event and an injury stoppage on the same tick. An `AiDecision` event (`SubInjury`) follows, and the change applies on the injury tick, so the team is back to eleven. With the limit used, the rejection reads "substitution limit reached (5 of 5)", the player's status is `Injured`, and 10 players remain. evidence-rung: n-a.
- **AC-6** "Given team A with attributes 15 percent higher than team B, When 200 seeded matches run headless, Then team A wins more than 50 percent." kind: `code-only`. status: met. method: automated (slow, release). evidence: `slow-strength.stderr.txt`: "stronger team: won 139, drew 34, lost 27 of 200" (69.5 percent). evidence-rung: n-a.
- **AC-7** "Given the AI manager's team trails after minute 70, Then a tactical-change event appears before full time in at least 90 percent of 100 headless runs." kind: `code-only`. status: met. method: automated (slow, release). evidence: `slow-ai_trailing.stderr.txt`: "an applied tactics change in 100 of 100 scenes". evidence-rung: n-a.
- **AC-8** "Given a mentality set to defensive versus attacking with identical squads, When 200 matches run each, Then shots per match differ in the expected direction." kind: `code-only`. status: met. method: automated (slow, release). evidence: `slow-mentality.stderr.txt`: "mean shots per match: defensive 2.15, attacking 33.69". The direction is right. The size is a calibration concern (see Friction Notes). evidence-rung: n-a.
- **AC-9** benchmark rerun (see Interactive Verification Results). kind: `user-observable` (annotated `observable: true`). status: met. method: interactive. evidence: `ac-9-{1,2,3}.stdout.txt`. evidence-rung: live.

evidence: live 1 / n-a 8. `metric-acceptance-mock-rung`: 0. No criterion is met by inference and none is deferred, so no consult trigger holds. The product owner also excluded `consult` at intake.

## Issues Found

None open. The two issues found on the first run are fixed:

- LOW: VOCAB-1, `web/index.html:28` showed the placeholder note "Built by viewer-lineup-tactics." in the empty Lineups panel. This is a planning label in shipped page text. It predates this slice (from commit `62a5dab`), and the match-rules vocabulary fix missed it. The plan step 22 gate requires the search to return nothing. Fixed.
- LOW: VOCAB-2, `crates/engine/src/tuning.rs:145` and `:315`: two doc comments added by this slice name a planning phase ("calibration moves behaviour", "which calibration replaces"). Fixed.

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| VOCAB-1 | check-failure (vocabulary gate) | Fix (autonomous) | Patched by the coordinator in place: the note now reads "Coming in a later version.", the text the other two empty panels use | exempt: shipped text only, no behaviour | Pass (`recheck-vocabulary.txt` 0 lines; 47 of 47 page tests) |
| VOCAB-2 | check-failure (vocabulary gate) | Fix (autonomous) | Patched by the coordinator in place: "so retuning moves behaviour without code" and "to be replaced with a sourced rate" | exempt: doc comments only, no behaviour | Pass (`recheck-vocabulary.txt` 0 lines; `recheck-fmt.txt` exit 0; `recheck-clippy.txt` exit 0) |

Commit: `e79ad61` ("chore: use product wording in the lineup placeholder and tuning notes"). It names only its two paths, so the staged documents and the staged test file of other work stay out of the commit. The diff is at `verify-evidence/tactics-and-ai/fix-vocab.diff`.
Regression tests added: 0 (both fixes are docs or text and exempt).

## Triage Decisions

- VOCAB-1 → `Fix`. `class: implementation-detail`. The autonomous policy selects Fix for every fixable issue. The change is text only and uses the wording that the sibling panels already use.
- VOCAB-2 → `Fix`. `class: implementation-detail`. A comment-only rewording with no behaviour change.
- The two `calibration` hits were counted as vocabulary even though the word is ordinary English. They name the later tuning phase, which the gate forbids, and rewording costs nothing. `class: implementation-detail`.
- No fix sub-agent was dispatched. This run has no agent-dispatch tool, and each fix is a one-line text edit, so the coordinator applied both edits and reviewed the diff before committing. `class: implementation-detail`.
- The commit message leaves out the workflow's usual `fix(<slug>): verify-time fixes for <slice>` form, because that form names a stage and a slice, which the external output boundary forbids. `class: implementation-detail`.

## Augmentation Verification

- **Mock fidelity (`02c-craft.md`):** no item applies. This slice changes no drawing. The only page change is `web/stoppages.mjs:27` (an injury is a stoppage mark, covered by `web/tests/stoppages.test.mjs`) and the VOCAB-1 note text. No token, component, or style changed.
- **Benchmark (`05c-benchmark.md`, compare mode against `837cb5c`):** no tripwire fired. Processor time per match: 418.8 → 434.4 ms (+3.7 percent; limit 460.7). Peak memory: 5.45 → 6.145 MB (+12.8 percent; limit 6.82). Per tick: 1.4965 → 1.456 µs (−2.7 percent). Wall time per match: 421 → 432 ms (+2.6 percent; budget 2000). Ticks per match: 279,850 → 298,350 (+6.6 percent; injuries and substitutions add time). Ticks per second: 664,727 → 690,625. Criterion `tick_step`: 1.5264 → 1.4815 µs (criterion says "within noise threshold"). `steering_pass_22`: 1027.1 → 880.84 ns. Stream throughput: 609,220 → 626,318 ticks/s. Stream peak memory: 6.84 → 7.34 MB (tripwire 8.55). The sibling `05c-benchmark.yaml` was updated in the same pass.
- **Instrumentation (`04b-instrument.md` §2):** all nine signals exist. `change.applied` (`tactics/change.rs:304`), `change.rejected` (`:316`), `tactics.plan` (`:444`, emitted where a change applies, which is implement deviation 5), `ai.decision` (`ai.rs:343`), `injury.occurred` (`rules/mod.rs:286`). The `match-stats` keys `stats.shots`, `injury.count`, `fatigue.mean_pct`, `darkpath.change_never_applied`, `changes.*`, `substitutions`, and `ai.decisions` appear in the seed-42 simulate output (`ai.decisions` 10, matching 10 `ai.decision` lines on stderr). The `tactics-change` event, `change.applied_tick`, `player.secondary_id`, and `ai.decision` are present in the protocol and the stream mapping. The run report is unchanged.
- **Experiment (`04c-experiment.md`):** deferred to the experiment-flags work; nothing to check.

## Security Scan

- CVE scan: `cargo-audit` is not installed. The lockfile is unchanged since the baseline, so this slice adds no dependency. New critical or high: 0 known.
- Secret detection: gitleaks, "no leaks found" over `837cb5c..HEAD`. The pattern grep found 0 matches.
- SAST: none installed beyond clippy with `-D warnings` (pass). New HIGH or above: 0.

## Accessibility Gate

Not automatable for this slice. The only page change is text in a placeholder paragraph (VOCAB-1) and one stoppage kind in a list. New WCAG AA violations: 0 introduced by inspection. The element and its class are unchanged.

## Performance Gate

- Bundle size delta: skipped. The page has no bundle, and the text change adds 6 bytes.
- Build time: release build 7,617 ms (incremental).
- Release binary: 4,046,848 bytes, against 3,696,128 bytes after match-rules (+9.5 percent, with the tactics file loader, the AI manager, and the decision layer).
- Benchmark: see Augmentation Verification. No tripwire fired.

## Cross-Slice Regression

Sibling slices checked: engine-core, data-schemas-generator, stream-protocol, viewer-pitch, match-rules. Their suites are all in `cargo test --workspace` (224 passed, including `determinism`, `snapshot`, `validator`, `full_match`, stream fixture, and protocol document tests) and in `node --test` (47 passed). Regressions found: 0.

## Longitudinal Delta

- Engine benchmark: baseline source is `bench-baseline/tactics-and-ai/` on `837cb5c`. The deltas are listed under Augmentation Verification, and each is an expected change.
- Page: no visual surface changed. The one text change is expected.

## Friction Notes

- The mentality effect is far larger than in real football (33.69 against 2.15 shots per match). The criterion asks only for direction, so this is informational here, and calibration owns it.
- The seed-42 match had 24 shots for the home team and 5 goals (`simulate-42.stdout.txt`). Calibration bands will judge this.

## Free Exploration Notes

- `crates/engine/tests/zz_stall_probe.rs` is staged as added in the index and deleted in the working tree. It is not part of this slice or its commits, and it seems to belong to another session. It was left untouched and kept out of the fix commit. Informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| empty submission | n-a | This slice adds no user input surface; the change queue's socket bridge is deferred (implement: Anything Deferred) |
| max-length input | n-a | Same |
| double-click / rapid repeat | n-a | Repeated queueing is covered by the limit tests (sixth substitution rejected) |
| mid-flow interruption | n-a | Snapshot continuation across a substitution and a tactics change is covered by `snapshot.rs` in the default suite |
| offline / network failure | n-a | No network path in this slice |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| slow response (Fast 3G) | n-a | No network surface |
| concurrent session | n-a | Single-player engine; no session state added |
| session expiry mid-flow | n-a | No sessions |

## Gaps / Unverified Areas

- The socket path for a human change (`queue-change`) is acknowledged but not applied. This is a documented deferral in the implement record that the lineup and tactics panel work picks up. No criterion of this slice covers it.
- `cargo-audit` is not installed. The unchanged lockfile limits the exposure.

## Freshness Research

Not required. No test failed, the plan is less than 14 days old (2026-09-22), and the slice touches no external API or dependency.

## Recommendation

Proceed to review. All nine criteria are met with direct evidence, and the benchmark holds with a 5.7 percent processor margin.

## Recommended Next Stage

- **Option A (recommended):** `/wf review football-manager-match-engine tactics-and-ai`. `convergence: converged` and `result: pass`. Consider compacting first, because the verify context is long.
- **Option D:** `/wf handoff football-manager-match-engine tactics-and-ai`. Skip review only with a clear reason. Not recommended, because the slice changes play in every engine path.
- **Option G:** `/wf probe football-manager-match-engine`, for a slug-wide runtime sweep across slices.
