---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: probe-engine-core
status: complete
stage-number: 6
created-at: "2026-09-23T19:17:29Z"
updated-at: "2026-09-23T19:17:29Z"
result: pass
metric-checks-run: 7
metric-checks-passed: 7
metric-acceptance-met: 6
metric-acceptance-total: 6
metric-acceptance-user-observable: 6
metric-acceptance-code-only: 0
metric-acceptance-mock-rung: 0
metric-interactive-checks-run: 6
metric-interactive-checks-passed: 6
metric-issues-found: 0
metric-issues-found-initial: 0
metric-issues-found-final: 0
fix-rounds-run: 0
convergence: not-needed
verify-owned-fix-commit: null
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: required
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/probe-engine-core/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped"
ac-staleness-checked: true
ac-stale-count: 0
longitudinal-baseline-compared: true
stability-check-flaky-count: 0
adversarial-tests-run: 5
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
  - "probe-engine-core Q-1 = A: each failed simulate or bench drive printed exactly one stdout record with outcome error, the three error keys, the command's success record.kind, and no statistic key; exit 1; the prose line kept on stderr; no statistics file saved under the data folder."
  - "probe-engine-core Q-2 = C: both injected calibration drives wrote no file containing \"failure\"; the failed match record and the failed-worker report say outcome error with the three error keys, and both validate against the record schemas."
  - "Design direction: not applicable; this slice changes no page."
  - "Dark-path counter definition: untouched; no queuing code changed."
  - "Output boundary: this stage made no commit and wrote no code, comment, or doc."
tags: [probe, cli, observability, help-text, record-schemas, error-record, calibration]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-probe-engine-core.md
  plan: 04-plan-probe-engine-core.md
  implement: 05-implement-probe-engine-core.md
  benchmark: 05c-benchmark.md
  review: 07-review-probe-engine-core.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine probe-engine-core"
---

# Verify: Probe Findings on the Engine Command Line

## The Verification

Implement handed over commit `733839c` with all five probe findings addressed and the two product-owner answers built in. This run re-built the release binary at HEAD `984bc2d` and drove every row of the plan's Verification Strategy through the real binary on the reference machine, with `SM_DATA_DIR` in a temp folder.

All 6 criteria are met with live evidence. The `--json` dump wrote `match.jsonl` (3,000 lines) beside `match.ticks`. Three failed runs (`--minutes 0`, a directory as `--ticks-out`, `bench --matches 0`) each exited 1 and printed exactly one `outcome: error` record, and an independent JSON Schema validator (Python `jsonschema`, Draft 2020-12) accepted all three. The injected calibration match and worker failures wrote no `failure` word, and their error records validate. "os error" occurs once, redirected stderr holds 0 escape bytes, and all 20 help checks have 0 lines over 80 columns (widest 80). The gates are clean: fmt, clippy `-D warnings`, 414 workspace tests passed with 0 failed, gitleaks found no leak, and the benchmark guard read 421 ms per match. No issue was found, so the fix loop did not run.

Review of this slice is next. The top open risk is the one implement named: run folders saved before `733839c` do not validate against the new schemas, which the product owner accepted. The two open deferrals on other slices were re-probed this run and still stand; neither touches this slice.

## Verification Summary

- Slice: `probe-engine-core` (compressed probe slice; standard mode, AC source is the plan's Verification Strategy rows F1, F2, F2-C, F3, F4, F5 over the five probe findings).
- Branch: `feat/football-manager-match-engine` (dedicated), HEAD `984bc2d`; slice code at `733839c`.
- Stack: `user-confirmed: true`; plan `stack-source: confirmed`. Adapter: `cli`.
- Checks: 7 run, 7 passed. Acceptance: 6 of 6 met (6 user-observable, 0 code-only). Interactive: 6 of 6.
- Issues: 0 initial, 0 final. Fix rounds: 0. Convergence: not-needed.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (exit 0; `check-fmt.txt`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass (exit 0; `check-clippy.txt`).
- `cargo test --workspace`: pass (exit 0; 58 result lines, 414 passed, 0 failed, 4 ignored; `check-test-workspace.txt`). `cli_args` 14 passed, including `json_dump_replaces_the_tick_file_extension`, `an_unwritable_tick_path_names_the_os_error_once`, `redirected_stderr_carries_no_ansi_escape`, `no_help_line_exceeds_eighty_columns`, `a_failed_simulate_prints_one_error_record`, `a_failed_bench_prints_one_error_record`, `a_failed_calibration_match_writes_an_error_record`, `a_failed_calibration_worker_makes_the_report_an_error`, and `the_test_seams_stay_out_of_the_help`. `schemas` 4 passed, including `the_schemas_accept_success_and_error_only_and_require_the_keys_of_each`.
- `cargo build --release --workspace`: pass (exit 0; `check-build-release.txt`).
- `gitleaks git --log-opts=89fe959..733839c`: pass (1 commit scanned, no leaks found; `check-gitleaks.txt`).
- `engine-cli bench --seed 42 --matches 5 --json` (benchmark guard): pass (exit 0; `bench.match_wall_ms` 421, `budget.pass` true, 282,600 ticks per match, 6.68 MB peak; `bench-42.*`).
- Independent schema validation of the six drive records with Python `jsonschema` Draft 2020-12: pass (6 of 6 valid; `schema-validate.txt`).

## Interactive Verification Results

Driver script: `verify-evidence/probe-engine-core/drives.sh` (re-runnable: `bash drives.sh <evidence-dir>` from the repository root after `cargo build --release --workspace`).

- **Criterion**: F1 — the `--json` dump path is the one the help names.
  - **Platform & tool**: cli, release `engine-cli.exe` through Git Bash.
  - **Steps performed**: `simulate --seed 7 --minutes 1 --no-snapshot --json --ticks-out $T/match.ticks`; `ls $T`; `wc -l $T/match.jsonl`.
  - **Evidence**: `verify-evidence/probe-engine-core/f1-json-dump.*`
  - **Observation**: exit 0; the folder holds `match.jsonl` and `match.ticks`, and no `match.ticks.jsonl`; `match.jsonl` has 3,000 lines. The `simulate --json` long help states the replace rule.
  - **Result**: pass.
- **Criterion**: F2 — a failed `simulate` or `bench` run prints one structured record.
  - **Platform & tool**: cli, release binary; Python `jsonschema` for the independent schema read.
  - **Steps performed**: `simulate --seed 7 --minutes 0 --ticks-out $T/a.ticks`; `simulate --seed 7 --minutes 1 --no-snapshot --ticks-out $T/dir` (a directory); `bench --seed 7 --matches 0`; `find $SM_DATA_DIR -type f`.
  - **Evidence**: `f2-simulate-minutes-0.*`, `f2-simulate-dir.*`, `f2-bench-matches-0.*`, `f2-data-dir.listing.txt`, `schema-validate.txt`
  - **Observation**: each exits 1 with exactly one stdout line. `simulate` records: `record.kind` `match-stats`, `outcome` `error`, `error.type`/`error.code` `invalid-config` and `io`, `error.retriable` false, no `stats.*`, `ticks.*`, or `validate.*` key. `bench` record: `record.kind` `run-report`, `operation` `benchmark`, `machine.hash` and `run.id` present, no `bench.*` statistic key. Stderr ends with the prose line (`error: invalid configuration: minutes must be 1 to 200, got 0`; `error: cannot create …/dir: io error: Access is denied. (os error 5)`; `error: invalid configuration: --matches must be at least 1`). The data folder holds only the successful F1 match and `owner.id`; no failed run was saved. All three records validate.
  - **Result**: pass.
- **Criterion**: F2-C — calibration writes `error`, not `failure`, with the error keys.
  - **Platform & tool**: cli, release binary with the hidden `--inject-failure` seam.
  - **Steps performed**: `calibrate --seed 1 --matches 2 --minutes 1 --jobs 1 --suite equal --inject-failure match --out $T/cal-m`, then `grep -rl '"failure"'` and `grep -rl '"outcome":"error"'` over the run folder; the same with `--inject-failure worker --out $T/cal-w`.
  - **Evidence**: `f2c-cal-match.*` (listing, grep, error record, report), `f2c-cal-worker.*` (listing, grep, report), `schema-validate.txt`
  - **Observation**: match injection: 0 files say `failure`; 1 of 2 stats files says `outcome` `error` with `error.type`/`error.code` `invalid-config`, `error.retriable` false, `rules.pack_version` 4, and `manager.kind` `["ai","ai"]`; the run report says `success`, as the plan's D7 rule states. Worker injection: 0 files say `failure`; the run report says `outcome` `error`, `error.type` `worker`, `error.code` `worker-failed`, `error.retriable` false. All three calibration records validate. Both drives exit 2 because the 1-minute matches miss the goal and shot realism bands; that is the calibrate band verdict, not a failure of this criterion.
  - **Result**: pass.
- **Criterion**: F3 — the OS error is named once.
  - **Platform & tool**: cli, release binary.
  - **Steps performed**: the directory drive from F2; `grep -o 'os error' | wc -l` on its stderr.
  - **Evidence**: `f2-simulate-dir.stderr.txt`
  - **Observation**: "os error" occurs 1 time.
  - **Result**: pass.
- **Criterion**: F4 — redirected stderr carries no ANSI escape.
  - **Platform & tool**: cli, release binary with stderr redirected to a file.
  - **Steps performed**: `grep -c $'\x1b'` on `f1-json-dump.stderr.txt`.
  - **Evidence**: `f1-json-dump.stderr.txt`
  - **Observation**: 0 escape bytes.
  - **Result**: pass.
- **Criterion**: F5 — no help line exceeds 80 columns.
  - **Platform & tool**: cli, release binary; `awk` width count.
  - **Steps performed**: `engine-cli <surface> -h` and `--help` for the top level, `simulate`, `bench`, `generate`, `serve`, `launch`, `record`, `replay`, `resume`, and `calibrate` (20 checks).
  - **Evidence**: `f5-help-width.txt`
  - **Observation**: 0 lines over 80 on all 20 checks; widest 80 (`serve -h`, `serve --help`, `launch --help`, `calibrate -h`).
  - **Result**: pass.

## Acceptance Criteria Status

| Criterion | Kind | Status | Method | Evidence | Evidence-rung |
|---|---|---|---|---|---|
| F1: the `--json` dump path is the one the help names | user-observable | met | interactive + automated | `f1-json-dump.*`; test `json_dump_replaces_the_tick_file_extension` | live |
| F2: a failed `simulate` or `bench` run prints one structured record | user-observable | met | interactive + automated | `f2-*.*`, `schema-validate.txt`; tests `a_failed_simulate_prints_one_error_record`, `a_failed_bench_prints_one_error_record` | live |
| F2-C: calibration writes `error`, not `failure`, with the error keys | user-observable | met | interactive + automated | `f2c-cal-*.*`, `schema-validate.txt`; tests `a_failed_calibration_match_writes_an_error_record`, `a_failed_calibration_worker_makes_the_report_an_error`, `the_schemas_accept_success_and_error_only_and_require_the_keys_of_each` | live |
| F3: the OS error is named once | user-observable | met | interactive + automated | `f2-simulate-dir.stderr.txt`; test `an_unwritable_tick_path_names_the_os_error_once` | live |
| F4: redirected stderr carries no ANSI escape | user-observable | met | interactive + automated | `f1-json-dump.stderr.txt`; test `redirected_stderr_carries_no_ansi_escape` | live |
| F5: no help line exceeds 80 columns | user-observable | met | interactive + automated | `f5-help-width.txt`; test `no_help_line_exceeds_eighty_columns` | live |

Partition: every row names command output or a command run (Step B), so all 6 are user-observable. Rollup: evidence: live 6.

## Issues Found

None.

## Augmentation Verification

- `02c-craft.md` mock fidelity: not applicable; this slice changes no page.
- Instrumentation (`04b-instrument.md`): the `outcome` value `error` is now produced on the `simulate`, `bench`, and calibration failure paths, and every drive record validates against the record schemas. Success records are unchanged (`a_simulated_match_writes_records_that_validate_against_the_schemas` and `a_benchmark_report_validates_against_the_run_report_schema` pass).
- Experiment (`04c-experiment.md`): `calibrate_pair.rs` 3 passed; a paired run with no failed worker is unchanged.
- Benchmark (`05c-benchmark.md`): compare guard 421 ms per match against the recorded 421 ms baseline and the latest 432 ms compare; within 10 percent. No simulation path changed.

## Security Scan

- CVE scan: skipped for tooling (`command -v cargo-audit cargo-deny` → not found); the slice commit changes no `Cargo.toml` or `Cargo.lock` line (`git diff 89fe959 733839c -- Cargo.toml Cargo.lock crates/*/Cargo.toml | wc -l` → 0), so no dependency changed.
- Secret detection: gitleaks over `89fe959..733839c`, no leaks found.
- SAST: `semgrep` not installed; not run. The change adds no network, file-permission, or process-spawn surface beyond the hidden calibrate seam, which is off by default and hidden from both help forms (`the_test_seams_stay_out_of_the_help`).

## Accessibility Gate

Not automatable: command-line output only. New WCAG AA violations: 0. The 80-column help rule (F5) is the terminal-legibility check for this surface.

## Performance Gate

- Bundle size: skipped (no web bundle changed).
- Benchmark: 421 ms per match (`bench-42.*`), `budget.pass` true.
- Build time: release build finished in 12.02 s incremental.

## Cross-Slice Regression

Sibling suites in the workspace run: all pass (414 passed, 0 failed), including `calibrate.rs` (2 passed, 1 ignored), `calibrate_pair.rs` (3), `docs.rs` (3), `schemas.rs` (4), `script_cli.rs` (5), and the engine unit tests (135). Regressions found: 0.

## Longitudinal Delta

- Help width: probe baseline (`probe-evidence/engine-core/enumerate.help.txt`, widest 108 under `--help`) and plan baseline (47 lines over 80 under `-h`, widest 145) → now 0 lines over 80, widest 80. Expected change.
- Failure stdout: probe baseline 0 bytes on stdout for every in-run failure → now one `outcome: error` record. Expected change.
- Dump path: implement evidence `f1-json-dump.listing.txt` → same listing this run. No change.

## Friction Notes

- A calibrate drive of 1-minute matches exits 2 on the realism bands. The injected-failure drives inherit that exit code, so a reader of the exit code alone cannot tell the injected failure from the band verdict. Informational; the record content is what the criterion reads.

## Free Exploration Notes

- `simulate --seed 7 --minutes 201 --ticks-out …` exits 1 with one `error` record (`invalid-config`), and only `owner.id` exists in the data folder afterward. — informational
- Argument-parse failures (`--seed=`, `--seed 18446744073709551616`, `--matches -1`) exit 2 with clap's message and no stdout record, as the plan states (argument-parse failures happen before a run starts). — informational

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| Empty submission (`simulate --seed= --minutes 1`) | pass | exit 2, message names `--seed <SEED>`, 0 stdout bytes |
| Max-length input (`--seed 18446744073709551616`, `--minutes 201`) | pass | seed: exit 2 naming the argument; minutes: exit 1 with one `error` record |
| Negative count (`bench --matches -1`) | pass | exit 2, "unexpected argument '-1'", 0 stdout bytes |
| Double-click / rapid repeat (two failed `simulate` drives in sequence) | pass | two records with distinct `match.id` values; nothing saved |
| Offline / network failure | n-a | the binary uses no network |
| Mid-flow interruption (calibration worker dies) | pass | the injected worker failure gives an `error` run report that validates |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Slow response (Fast 3G) | n-a | no network surface |
| Concurrent session | n-a | single-process command |
| Session expiry mid-flow | n-a | no session |

## Gaps / Unverified Areas

- `generate`, `serve`, `launch`, `record`, `replay`, `resume`, and a `calibrate` failure before workers start keep prose-only failures (plan A-8 and A-13); outside this slice's criteria.
- Prior open deferrals re-probed this run (not inherited), both outside this slice: `grep -ciE "legibility|read the match screen" .ai/workflows/football-manager-match-engine/po-answers.md` → `0` (viewer-match-day AC-7 wall stands); `test -f .ai/workflows/football-manager-match-engine/verify-evidence/distribution/macos/results.json` → `exit=1` (distribution macOS wall stands). This slice's criteria are Windows command-line behaviours and need neither a human legibility reading nor a Mac, so this slice records no deferral.

## Freshness Research

Not triggered: no test failed, the plan was revised on 2026-09-23 (under 14 days), and the slice touches no external API. The schema contract was read back through a second, independent validator (Python `jsonschema` Draft 2020-12) in addition to the repository's `boon` tests; both accept the records.

## Recommendation

Proceed to review. Every criterion is met by live drives of the release binary and by the regression tests implement added.

## Recommended Next Stage

- **Option A (default):** `/wf review football-manager-match-engine probe-engine-core` — `result: pass`, convergence not-needed; 6 of 6 criteria met live, 414 workspace tests pass. Compact first if the session is long.
- **Option D:** `/wf handoff football-manager-match-engine probe-engine-core` — skip review only if the slug-wide review ledger is judged to cover this local command-line change.
- **Option G:** `/wf probe football-manager-match-engine` — a slug-wide runtime sweep, if wanted after the per-slice verifies.

## Assumptions

- **V1** (class: implementation-detail, ac: "F1", classification: runtime-evidence): The plan's Verification Strategy rows F1–F5 and F2-C are the acceptance criteria of this compressed probe slice, because the slice file carries findings, not an AC list.
- **V2** (class: implementation-detail): A drive of the real release binary on the reference machine is recorded at the `live` rung, as sibling command-line verifies record it.
- **V3** (class: implementation-detail, ac: "F2-C", classification: runtime-evidence): The run report of the match-injection drive saying `success` is correct under implement decision D7 and the plan's step 10 (a failed match with no failed worker keeps the run `success`); the criterion reads the absence of `failure` and the `error` stats record.
- **V4** (class: implementation-detail): Calibrate exit code 2 in the injected drives comes from the band verdict on 1-minute matches, not from the injected failure; it is recorded as friction, not an issue.
- **V5** (class: implementation-detail): The two open deferrals of other slices were re-probed fresh and do not constrain this slice; no deferral is written here and neither ledger entry is changed.
- **V6** (class: implementation-detail): No second-opinion consult ran: no trigger in the consult rules holds (every criterion is met by direct observed evidence), and the product owner excluded `consult`.
- **V7** (class: implementation-detail): No fix sub-agent was dispatched and no commit was made, because 0 issues were found.
- **V8** (class: implementation-detail): In `00-index.md` this run added this file and its evidence folder to `workflow-files`, raised `evidence-quality.live` from 6 to 12, and moved `updated-at`. It left `current-stage`, `selected-slice`, and `next-command` pointing at `distribution`, which another session is moving through its later stages. The `03-slice.md` roster entry for this slice already reads `status: complete`, which `result: pass` keeps.
