---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: match-rules
status: complete
stage-number: 6
created-at: "2026-09-22T22:02:26Z"
updated-at: "2026-09-22T22:02:26Z"
result: pass
metric-checks-run: 16
metric-checks-passed: 16
metric-acceptance-met: 8
metric-acceptance-total: 8
metric-acceptance-user-observable: 1
metric-acceptance-code-only: 7
metric-interactive-checks-run: 1
metric-interactive-checks-passed: 1
metric-issues-found: 0
metric-issues-found-initial: 5
metric-issues-found-final: 0
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: "a4893c8"
regression-tests-added: 3
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [cli, web]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/match-rules/"
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
adversarial-tests-run: 9
adversarial-tests-failed: 0
failure-mode-probes-run: 3
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
  - "No visual change to the page: the verify fixes touched page comments and two placeholder note texts only; elements, classes, and --tl- tokens are unchanged."
  - "The canvas link stayed out of sub-agent prompts; the one page constraint a fix agent needed (no visual change) was written into its prompt as text."
tags: [engine, laws, referee, snapshot, resume, protocol, benchmark, milestone]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-match-rules.md
  plan: 04-plan-match-rules.md
  implement: 05-implement-match-rules.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-match-rules.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine match-rules"
---

# Verify: Match Rules and Stoppage Snapshots

## The Verification

Implement handed over commit `0611856`: a referee inside the loop, six law modules, a binary snapshot with a SHA-256 trailer, `engine-cli resume`, and protocol version 2. The slice carries eight criteria. Seven are cargo tests. The benchmark report is the one user-observable criterion. Cargo serialises on its target folder and the benchmark needs an idle processor, so the coordinator ran the checks in sequence. One sub-agent re-drove the viewer in headless Edge against the new protocol.

Every check and every criterion passed on the first run, and the benchmark read 1.4937 µs per tick against the 1.566 µs limit. The live drives found five issues that no criterion covers. First, 3 of the 12 seed-42 free kicks waited the full 24-second hard limit, because a touchline clamp pulled an opponent's restart target back inside 9.15 m. Second, the snapshot refusal log printed the absolute path, with the user's folder name. The other three were a write-failure reason with no OS text, stale help text, and workflow vocabulary left in 17 lines of shipped files. The user triaged all five `Fix`. Worktree isolation failed for every fix agent, so the fixes ran in the main tree on disjoint files. One commit, `a4893c8`, holds the five fixes and three regression tests. The stall test fails on the old logic and passes on the new one.

On `a4893c8`, all 192 Rust tests and 46 page tests pass. The seed-42 match has zero stalls, and the benchmark reads 1.508 µs per tick and 5.45 MB peak memory. Review is next. The top risk is the processor budget: the margin is now 3.7 percent, and the tactics slice adds decisions to every tick.

## Verification Summary

- Result: `pass`. 8 of 8 criteria met; the one user-observable criterion (AC-h) has live runtime evidence on the reference machine.
- Convergence: `converged`. Five issues (STALL-1, INSTR-1, INSTR-2, VOCAB-1, HELP-1), five `Fix` decisions, all patched and re-checked; one mechanical format difference found inside the loop was auto-fixed. `fix-rounds-run: 1`.
- Adapters: `cli`, driven directly (`cli-direct`) for AC-h, the probes, and the benchmark compare; `web`, headless Microsoft Edge over the DevTools protocol, for the cross-slice regression drive of the viewer.
- Sub-agents: one read-only drive agent (viewer regression); seven fix dispatches, of which four failed in worktree isolation before any edit and one was stopped by the user; the fixes landed through three partition agents and one finishing agent.
- Benchmark compare mode: recorded in `05c-benchmark.md` and `05c-benchmark.yaml`; ten targets compared, no tripwire fired.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (exit 0; `check-fmt.exit-code`; after fixes `recheck-fmt.txt`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass (exit 0; `check-clippy.exit-code`, `recheck-clippy.txt`).
- `cargo test --workspace`: pass. First run 189 passed, 0 failed, 0 ignored, 30 binaries, 55,425 ms (`check-test.stdout.txt`). After fixes 192 passed, 0 failed (`recheck-test.stdout.txt`).
- Criterion test files re-driven one by one: pass (`criterion-tests.txt`): `rules_offside` 4, `rules_fouls` 5, `rules_cards` 2, `rules_restarts` 4, `rules_clock` 3, `snapshot` 3, `engine-cli` `resume` 2.
- `node --test web/tests/*.test.mjs`: pass (46 of 46; `check-node-test.stdout.txt`, `recheck-node-test.txt`).
- `cargo build --release -p engine-cli`: pass (exit 0, 6,091 ms, 0 warnings; binary 3,696,128 bytes; `check-build-release.*`).
- `gitleaks detect --log-opts="d86996a..HEAD"`: pass ("no leaks found", 2 commits; `check-gitleaks.*`).
- Secret pattern grep over `git diff d86996a...HEAD` outside `.ai/`: pass (0 matches; `check-secret-grep.txt`).
- Dependency check: pass. The lockfile diff adds no crate and changes no version (`check-lockfile-diff.txt` is empty); the only lockfile change is the engine's self dev-dependency. `cargo-audit` is not installed.
- `sdlc-debt:` marker hygiene over the slice diff: pass. 0 added, 0 malformed, 0 unrecorded, 1 retired (the wall-bounce marker in `sim.rs`); 0 markers remain in `crates/` and `web/` (`check-debt-markers.txt`, `check-debt-markers-tree.txt`).
- Workflow vocabulary search over `crates/`, `web/`, `docs/`, `README.md`, `content/README.md` (plan step 23 gate): fail on the first run, 17 lines, 0 of them added by this slice (`check-vocabulary-filtered.txt`, `check-vocabulary-diff.txt`); pass after VOCAB-1. The remaining hits are code uses of `.slice(` and a Rust byte slice, plus three untracked design documents under `docs/design/realism/` that another workflow is writing (see Gaps).
- Instrumentation key check against `04b-instrument.md` §2: fail on the first run (INSTR-1, INSTR-2); pass after fixes. Details under Augmentation Verification.
- `engine-cli bench --seed 42 --matches 5 --json`, three drives: pass (AC-h; `ac-h-{1,2,3}.*`, after fixes `recheck-ac-h-{1,2,3}.*`).
- `cargo bench -p engine --bench tick_step`: pass (`criterion.stdout.txt`, `recheck-criterion.stdout.txt`).
- `engine-cli bench --seed 42 --matches 1 --stream --json`, three drives: pass (`bench-stream-{1,2,3}.*`, `recheck-bench-stream-{1,2,3}.stdout.txt`).
- Cold start, `engine-cli --version` five times: pass (26 to 28 ms; `perf-cold-start-ms.txt`).
- Cross-slice regression, headless viewer drive: pass (`viewer-regression/`); details under Cross-Slice Regression.

## Interactive Verification Results

- **Criterion:** AC-h "Given the benchmark reruns after this slice, Then CPU time per match is within 10 percent and memory within 25 percent of the engine-core baseline", judged per tick (plan Round 4 Q13).
  - **Platform & tool:** cli, the release binary driven directly (`cli-direct`) on the reference machine (AMD Ryzen 7 9800X3D, Ultimate Performance plan), Git Bash, `SM_DATA_DIR` set to a scratch folder.
  - **Steps performed:** built the release binary; ran `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` three times, capturing stdout, stderr, and the exit code; repeated the three drives on `a4893c8` after the fixes.
  - **Evidence:** `verify-evidence/match-rules/ac-h-{1,2,3}.stdout.txt`, `.stderr.txt`, `.exit-code`; `recheck-ac-h-{1,2,3}.*`.
  - **Observation:** on `fdd943f`, `bench.cpu_us_per_tick` 1.4944, 1.4937, 1.4937 (median 1.4937, +4.9 percent); `bench.peak_mem_mb` 5.469, 5.441, 5.445; `bench.match_wall_ms` 416 on each; `bench.ticks_per_match` 276,100; `bench.cpu_wall_ratio` 0.985 to 0.992; `budget.pass` true; `build.hash` `fdd943f-dirty`; exit 0 on each. On `a4893c8`: 1.5080, 1.5080, 1.4858 (median 1.5080, +5.9 percent); peak 5.453 MB median; wall 420 ms median; 279,850 ticks per match; ratio 0.993 to 1.002; exit 0 on each. The three drives agree within 1.5 percent on each commit, so `stability: stable`. The bench run wrote no snapshot: the scratch data folder held only `owner.id`.
  - **Result:** pass. Every pass criterion in the plan holds on both commits.

The seven other criteria are annotated `observable: false`. The release binary was also driven for the probes below and for the instrumentation check, not for a criterion.

## Acceptance Criteria Status

- **AC-a** "Given an attacker beyond the second-last defender when a teammate plays the ball forward, When the attacker becomes involved, Then the engine emits an offside event and restarts with an indirect free kick at the offside position." kind: `code-only`. status: met. method: automated. evidence: `rules_offside.rs` 4 of 4; `a_player_beyond_the_second_last_defender_is_penalised_at_the_first_touch` asserts the offside event for the attacker, a free kick for the other team within 0.5 m of the attacker's position, and `dead.direct == false`; level, own half, and throw-in cases assert no offside. evidence-rung: n-a.
- **AC-b** "Given a tackle whose foul roll succeeds, Then a foul event and a free kick follow; and Given a foul roll that fails, Then play continues with no event." kind: `code-only`. status: met. method: automated. evidence: `rules_fouls.rs` 5 of 5; the foul band gives `[Foul, FreeKick]` at the tackle within 0.5 m; a draw outside the band gives no event and keeps the carrier; the recorded advantage reading (plan Round 1 Q4) and the penalty-area case are also covered. evidence-rung: n-a.
- **AC-c** "Given a player's second yellow card, Then a red card event follows and the team plays with ten; the formation anchors reshape to ten." kind: `code-only`. status: met. method: automated. evidence: `rules_cards.rs` 2 of 2; the second yellow emits one `card` event with `card.kind` `second-yellow` (the one-event-with-a-kind reading the plan records), `summary.red` counts it, the player parks off the pitch, 10 remain, and the line re-spreads to its original width. evidence-rung: n-a.
- **AC-d** "Given the ball crosses each boundary line, Then the matching restart (throw-in, corner, goal kick) is emitted and the ball is placed at the correct spot within 0.5 m." kind: `code-only`. status: met. method: automated. evidence: `rules_restarts.rs` 4 of 4 on `fdd943f`, 5 of 5 on `a4893c8`; each boundary test asserts the kind, the team, the spot within 0.5 m, the ball at the spot, and the restart flag; `simulate` reports 0 `validate.violations` with the `restart_spot` rule on. evidence-rung: n-a.
- **AC-e** "Given accumulated stoppages in a half, Then stoppage time equals the rule pack's formula within one second." kind: `code-only`. status: met. method: automated. evidence: `rules_clock.rs` 3 of 3; the test computes the formula independently from the rule pack (two goals, five free kicks, one card, draw 0.75) and asserts `abs_diff <= 1`; the full match ends at 90 minutes plus both halves' added time; a shortened match plays none. evidence-rung: n-a.
- **AC-f** "Given a stoppage, Then a snapshot is written; and Given `engine-cli resume` on that snapshot, Then the resumed match has the same score, clock, lineups, and cards as the snapshot." kind: `code-only`. status: met. method: automated, with live corroboration. evidence: `snapshot.rs` `a_resumed_match_continues_tick_for_tick` compares every tick record after the 30th stoppage byte for byte; `resume.rs` compares ten full-time counts. Live: a real `simulate` killed after 250 ms (exit 137) and resumed from its latest snapshot reached the same full time as an uninterrupted run on all 11 compared keys (`probe-interrupt.compare.txt`). evidence-rung: n-a.
- **AC-g** "Given a corrupt snapshot file, When `engine-cli resume` runs, Then it exits non-zero naming the corruption; and Given a valid snapshot, Then it resumes." kind: `code-only`. status: met. method: automated, with live corroboration. evidence: `snapshot.rs` and `resume.rs` refusal tests (checksum, truncated, bad magic, unknown version, build, content); live probes exit 1 naming `checksum mismatch`, `bad magic`, and `content mismatch`, and the valid snapshot resumes with exit 0 (`adv-*.stderr.txt`). evidence-rung: n-a.
- **AC-h** benchmark rerun (see Interactive Verification Results). kind: `user-observable` (annotated `observable: true`). status: met. method: interactive. evidence: `ac-h-{1,2,3}.stdout.txt`, `recheck-ac-h-{1,2,3}.stdout.txt`. evidence-rung: live.

evidence: live 1 / n-a 7. `metric-acceptance-mock-rung`: 0. No criterion is met by inference and none is deferred, so no consult trigger holds; the product owner also excluded `consult` at intake.

## Issues Found

None open. The five issues found on the first run are fixed, and the Verify-Owned Fixes table lists each one with its triage:

- MED: STALL-1, restart targets clamped back inside the required distance near a line; 3 of 12 seed-42 free kicks waited the 24-second hard limit (`crates/engine/src/rules/restart.rs:174`). Fixed.
- MED: INSTR-1, `snapshot.refused` logged the absolute path as typed, with the user's folder name, against `04b-instrument.md` §4 (`snapshot.rs:122`, `resume.rs:65`). Fixed.
- MED: VOCAB-1, 17 lines of workflow vocabulary in shipped files; `docs/reference/protocol.md:166-168` also stated that this slice applies queued changes, which it does not. Fixed.
- LOW: INSTR-2, `snapshot.write_failed` logged `reason=io error` with no OS text. Fixed.
- LOW: HELP-1, the top-level help listed "simulate and bench" for seven commands, and the `--no-snapshot` help had a stray indent. Fixed.

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| STALL-1 | check-failure (live drive) | Fix | Patched (`outside_on_pitch` in `restart.rs`; diagnosis confirmed: at each stall one opponent's target sat on the touchline clamp at y = ±33.5, 6.4 to 7.2 m from the spot, below the 8.9 m readiness distance) | `crates/engine/tests/rules_restarts.rs` `opponents_are_ten_yards_away_when_a_free_kick_is_taken_near_a_touchline` (fails on the old logic at the hard limit, tick 2202; passes on the new logic) | Pass (seed 42: 0 stalls; 192 tests pass) |
| INSTR-1 | check-failure (instrumentation) | Fix | Patched (`shorten_for_log` used by both refusal log sites) | `crates/engine/src/snapshot.rs` `shorten_for_log_hides_everything_outside_the_data_folder_or_cwd` | Pass (`recheck-refused.stderr.txt`: `path=empty.smsn`) |
| INSTR-2 | check-failure (instrumentation) | Fix | Patched (`full_reason` joins the error's source chain) | `crates/engine/src/snapshot.rs` `write_failed_reason_carries_the_underlying_os_error` | Pass (`reason=io error: Cannot create a file when that file already exists. (os error 183)`) |
| VOCAB-1 | check-failure (docs and comments) | Fix | Patched (13 files, comments, doc text, and two placeholder notes) | exempt: comment and documentation finding | Pass (vocabulary search clean; `cargo test -p protocol` including the document test passes) |
| HELP-1 | check-failure (command-line copy) | Fix | Patched (`about` set in `cli.rs`; `--no-snapshot` help rewritten) | exempt: help copy | Pass (`recheck-help.txt`) |
| FMT-1 | format (found in the loop) | Fix (mechanical, auto) | Patched by `cargo fmt --all`: the throw-in arm of `target()` re-wrapped over six lines | n-a | Pass (`fix-fmt.before.txt`, `recheck-fmt.txt`) |

Commit: `a4893c8`, 14 files, committed with an explicit path list.
Regression tests added: 3.

Dispatch record: four worktree-isolated fix agents failed before any edit, because their shell refused every command after it lost its worktree context. Two of them edited their own worktree without a check; those edits were discarded and the worktrees removed. The fixes then ran as partition agents in the main tree on disjoint files. The user stopped the first STALL-1 partition agent. A finishing agent's session ended while it had the old call sites swapped in to prove that the test fails first. The coordinator restored the patched file from that agent's own backup, found that the first test did not reproduce the stall, and dispatched a last agent that diagnosed the stall and wrote a test that does reproduce it.

## Augmentation Verification

- `02c-craft.md` mock fidelity: not applicable to this slice. It changes no visual element; the plan records that the design contract binds no step. The regression drive screenshots show the viewer unchanged.
- `instrument` (`04b-instrument.md`, 10 signals): 10 of 10 present, after two fixes.
  - `match-event` law types: seen on a real stream (`record-42.events.txt`, 50 events: kick-off 2, goal-kick 20, foul 13, free-kick 12, throw-in 1, half-time 1, full-time 1; `record-9.card-events.txt`: one `card` with `player.id` and `card.kind` `yellow`). Fouls carry `player.id`, `player.secondary_id`, and `foul.advantage`; half-time and full-time carry `added_time.s` and `minute.added`.
  - `rules.added_time`: 2 lines per match. `rules.dead_ball_stalled`: fired 3 times on the first run, which is how STALL-1 was found. `rules.abandoned`: covered by `a_fifth_send_off_abandons_the_match`; no command-line path reaches it.
  - `snapshot.written`: 34 debug lines, equal to `snapshot.writes`. `snapshot.write_failed`: 34 lines with `SM_DATA_DIR/matches` blocked, and the match continued to full time. `snapshot.refused`: named on every refusal. `match.resumed`: carries score, half, on-pitch counts, and cards.
  - `match-stats` law fields: all 12 keys present. `run-report`: `bench.ticks_per_match` and `bench.cpu_us_per_tick` present.
  - Deviations fixed in the loop: INSTR-1 (an absolute path in `snapshot.refused`) and INSTR-2 (a bare `io error` reason).
- `benchmark` (`05c-benchmark.md`, compare mode): 10 targets compared on `a4893c8`; the gate reads 1.5080 µs per tick against 1.566 µs, and peak memory reads 5.45 MB against 6.69 MB; no tripwire fired. The table is in `05c-benchmark.md` `## Comparison Results`.
- `experiment`: deferred to `experiment-flags`; nothing to re-check.

## Security Scan

- CVE scan: no new dependency (lockfile diff adds no crate); `cargo-audit` not installed; no new critical or high advisory is possible from this diff.
- Secret detection: `gitleaks` over `d86996a..HEAD`, no leaks found; pattern grep, 0 matches.
- SAST: `semgrep` not installed; skipped. The one untrusted input the slice adds, the snapshot file, is checked by magic, version, length, and SHA-256 before any field is read, and the refusal text is built from the reader's own checks, never from file bytes (random and truncated files are refused with exit 1).

## Accessibility Gate

Tool: none run. The slice changes no visual element; the verify fixes changed two placeholder note texts and page comments only. `a11y-result: not-automatable` for the command-line criterion; new WCAG AA violations: 0 by construction. The viewer's accessibility baseline stays the one recorded for the viewer slice.

## Performance Gate

- Bundle size: skipped. No base-branch worktree build ran, because worktree creation was unreliable in this session. The release binary is 3,696,128 bytes. The stream slice recorded 3,280,896 bytes, and three slices have landed since then.
- Build time: 6,091 ms for an incremental release build of `engine-cli`; no base comparison.
- Cold start: 26 to 28 ms for `engine-cli --version`; the stream slice measured 37 to 41 ms. No regression.

## Cross-Slice Regression

- Siblings checked: `engine-core`, `data-schemas-generator`, `stream-protocol`, `viewer-pitch`. Every sibling test runs in `cargo test --workspace` and `node --test`, and all pass on both commits.
- `viewer-pitch` overlap (`web/main.mjs`, `web/stoppages.mjs`): the viewer's headless Edge drive was re-run against the protocol-2 engine (`viewer-regression/drive-live.json`, `drive-main.json`). `live` exit 0 in 52 s: 3 rewinds exact; 4x rate 3.9998 to 4.0021; full time reached at tick 276,100 with the scrubber maximum `276100`, not 360,000; no console error. `main` exit 0 in 615 s: 0 skipped ticks over 5 minutes at 1x, 60 fps median, 4x rate about 4.00. The two `hookEqual: false` rewinds match the `viewer-pitch-run-2` baseline at the same fractions, so they are existing behaviour.
- Drive-script edits (copy only): evidence folder, scratch folder, a per-server `SM_DATA_DIR`, and a full-time wait that reads the scrubber maximum instead of 270,000 stored ticks.
- Regressions found: 0.

## Longitudinal Delta

- Command line: baseline `bench-baseline/match-rules/` on `d86996a`. The per-tick cost rose 5.9 percent, and matches are 2 to 4 percent longer. Both are expected: the referee's cost and added time.
- Viewer: baseline `verify-evidence/viewer-pitch-run-2/` screenshots and drive JSON. No visual delta; the scrubber stops at the last tick at full time, which is the intended change.

## Friction Notes

- Open play still produces 0 offsides and 0 corners on seeds 42 and 9. Both laws are proven only by scenes. This is recorded in implement, and calibration owns it once tactics adds forward runs.
- Seed 42 gives 20 goal kicks, 1 throw-in, and 0 corners. The ball mostly leaves over the goal lines. This is informational for calibration.
- `resume` on a missing file or a folder prints "cannot read <path>" with no operating-system reason. The exit code is 1, and the path is named. Informational.

## Free Exploration Notes

- `resume` writes new snapshots after it resumes (10 in the kill-and-resume probe), so a match can be resumed more than once. Informational.
- Two `resume` runs from the same snapshot write byte-identical tick files (`adv-repeat.cmp.txt`). Informational.
- `--team-a` with the other club's file is refused as `content mismatch` with both hashes named. Informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Empty submission: `resume` with no `--snapshot` | pass | clap exit 2 with usage |
| Empty file | pass | exit 1, `bad magic: not a snapshot file` |
| Missing file | pass | exit 1, `cannot read <path>` |
| A folder in place of a file | pass | exit 1, `cannot read <path>` |
| Random bytes (2,000) | pass | exit 1, `bad magic` |
| Trailer byte flipped | pass | exit 1, `checksum mismatch: the file is corrupt` |
| Other team file | pass | exit 1, `content mismatch` with both hashes |
| Rapid repeat: two resumes of one snapshot | pass | identical tick files |
| Mid-flow interruption: `simulate` killed after 250 ms, then `resume` | pass | same full time on all 11 compared keys |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Snapshot write failure (`SM_DATA_DIR/matches` is a file) | pass after INSTR-2 | 34 `snapshot.write_failed` lines; the match plays to full time; exit 1 comes from the final statistics write to the same blocked folder |
| Process killed mid-match | pass | the latest snapshot on disk resumes to the same result |
| `--no-snapshot` | pass | `snapshot.writes` 0; only `stats.json` written |

Slow response, concurrent session, and session expiry do not apply to a local command-line process.

## Cross-Browser Delta

- Primary browser: headless Microsoft Edge (Chromium), for the regression drive only.
- Secondary browser: not run; the slice changes page logic in one module and no rendering.
- Divergences found: none.

## Web Vitals

Not measured: the only page change is a stoppage filter and a scrubber bound. The drive shows 60 fps and 0 skipped ticks at 1x.

## Gaps / Unverified Areas

- `rules.abandoned` has test evidence only. No command-line path produces five sendings-off.
- Bundle size has no base-branch comparison (see Performance Gate).
- Another session staged three new documents under `docs/design/realism/` and the deleted probe file `crates/engine/tests/zz_stall_probe.rs` in this working tree's index. The design documents contain workflow vocabulary. They are not part of this slice, and commit `a4893c8` excludes them. Before handoff, the index needs a check so that neither file is committed by accident.
- Worktree isolation for fix sub-agents failed in this session. A later fix loop must expect this.

## Freshness Research

Not run. The plan is less than one day old, no test failed on a dependency, and the slice touches no external API. The plan's own freshness pass (IFAB Laws 3, 7, 11 to 17; `rand_chacha` 0.10 source; `std::fs::rename`) stands.

## Recommendation

Advance to review. Every criterion is met on committed code. The one user-observable criterion has live evidence, and all five issues found in the drives are fixed with tests or documented exemptions. Before tactics starts, review should look at the 3.7 percent processor margin.

## Recommended Next Stage

- **Option A (default):** `/wf review football-manager-match-engine match-rules`. `result: pass` and `convergence: converged`. Compact the session first: this verify run carried long fix-agent chatter that the review does not need.
- **Option D:** `/wf handoff football-manager-match-engine match-rules`. Not recommended: five verified slices wait for the slug-wide review ledger.
- **Option G:** `/wf probe football-manager-match-engine`. Optional: a slug-wide runtime sweep across the five verified slices.
