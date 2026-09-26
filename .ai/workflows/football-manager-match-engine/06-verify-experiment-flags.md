---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: experiment-flags
status: complete
stage-number: 6
created-at: "2026-09-23T17:17:16Z"
updated-at: "2026-09-23T17:17:16Z"
result: pass
metric-checks-run: 13
metric-checks-passed: 13
metric-acceptance-met: 2
metric-acceptance-total: 2
metric-acceptance-user-observable: 0
metric-acceptance-code-only: 2
metric-interactive-checks-run: 0
metric-interactive-checks-passed: 0
metric-issues-found: 0
metric-issues-found-initial: 0
metric-issues-found-final: 0
fix-rounds-run: 0
convergence: not-needed
verify-owned-fix-commit: null
regression-tests-added: 0
constraint-resolution-missing: []
interactive-verification: not-applicable
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/experiment-flags/"
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
  - "Dark-path counter definition: the paired run's report carries darkpath.change_never_applied (0 in both arms) as an on-arm guardrail and change.expired_at_full_time per arm with no zero rule; the computer manager's queuing is unchanged (the seed-42 tick records hash the same as the parent builds)."
  - "Design direction: this slice changes no page, style, token, or layout file, so no viewer constraint applies; the page suite passes 127 of 127 unchanged."
  - "Output boundary: a vocabulary scan over the build commit's added lines and message found no workflow term (the one hit is the Rust std::slice::from_ref call)."
tags: [engine, calibration, feature-flags, experiment, tuning]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-experiment-flags.md
  plan: 04-plan-experiment-flags.md
  implement: 05-implement-experiment-flags.md
  experiment: 04c-experiment.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-experiment-flags.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine experiment-flags"
---

# Verify: Feature Flags for Calibration Experiments

## The Verification

Implement handed over commit `aca285f` and its record commit `3066066`. The tuning file now has an optional `flags` block. Each flag must have an owner, a hypothesis, and a removal condition. `calibrate --pair <flag>` plays every fixture twice, first with the flag off and then with it on, on the same match seeds. It writes one report that shows both arms, one comparison row per band, and a verdict. Both criteria carry an explicit `observable: false` annotation, so the runtime gate does not apply and no deferral is needed. Cargo serialises on its target folder, so this run did every check in sequence, with no sub-agents.

Every check passed on the first run, so the fix loop had nothing to do. The workspace suite passed 364 tests with 0 failed and 4 ignored (49 result lines). The 11 flag tests, the 3 paired-run tests, the 6 comparison unit tests, and the shipped-file pin passed by name. A paired run by hand (seed 7, 6 matches per suite, 10 minutes, a test flag that sets the shot range to 12) printed the band table with both arms side by side. The 12 statistics files in each arm share file names and match seeds. The report validates against the run-report schema, and all 24 statistics records validate against the match-stats schema. A flag without an owner was refused by name on the command line (`flags.probe_short_range.owner: is required`, exit 1). The default match does not change: the seed-42 tick records after the 64-byte header hash to `b126cd59…` at 54,259,280 bytes, the same value as the parent builds. The benchmark median is 418.8 ms per match against the 460.7 ms gate.

Review is next. The top open risk is that the code-flag path (`ActiveFlags::is_on`) has no call site until the first candidate model registers a name. Also, peak memory had a median of 6.79 MB on this run against the 6.82 MB gate. That is a pass with less than 0.5 percent of headroom.

## Verification Summary

- Slice: `experiment-flags`, standard mode, branch `feat/football-manager-match-engine` (dedicated; confirmed with `git branch --show-current`). HEAD is `3066066`.
- Stack: `stack.user-confirmed: true`; plan `stack-source: confirmed`.
- The plan's `## Verification Strategy` names no environment dependency, so the constraint-resolution gate does not apply (`constraint-resolution-missing: []`).
- No prior `06-verify-experiment-flags.md` or evidence directory existed. This is evidence run 1, and no archive move was needed.
- Code under test equals `aca285f`: `git status` shows no modified path under `crates/`, `web/`, `content/`, or `schemas/`. The staged `docs/design/realism/*` files and the staged-then-deleted `crates/engine/tests/zz_stall_probe.rs` belong to other sessions. The deleted test file is not in the working tree, so it did not compile into any check.
- Issues found: 0 initial, 0 final. Fix loop: not run (`convergence: not-needed`).

## Automated Checks Run

1. `cargo fmt --all -- --check`: pass (exit 0; `fmt.txt`).
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass (exit 0, no warning; `clippy.txt`).
3. `cargo test --workspace --release --no-fail-fast`: pass, 364 passed, 0 failed, 4 ignored over 49 result lines, exit 0 (`test-workspace.txt`). This equals the implement record's count.
4. AC tests by name (`test-ac.txt`): `-p engine --test flags` 11/11, `--test content` 8/8 (includes `the_shipped_flags_block_matches_the_code_flags`), `-p engine-cli --test calibrate_pair` 3/3, `--test calibrate` 2 passed and 1 ignored (the 2000-match band test, ignored by design), `--test docs` 3/3, and `--bin engine-cli compare` 6/6 (`report::compare::tests`).
5. `node --test web/tests/*.test.mjs`: pass, 127 tests, 0 fail, 0 skipped (`web-tests.txt`).
6. Paired run by hand, `engine-cli calibrate --content-dir <copy with flag probe_short_range> --seed 7 --matches 6 --minutes 10 --jobs 2 --pair probe_short_range`: exit 0. It printed the table `suite / band / band range / off / on / delta` with 5 rows and `verdict: on-rejected`, and wrote `arms/off/` and `arms/on/` plus one `report.json` (`pair-run.stderr.txt`, `pair-report.json`).
7. Paired report content (`schema-check.txt`, `pair-report.json`): `calib.pair` names the flag, owner, hypothesis, and removal condition. `calib.flags` lists the flag with state `paired` and source `cli`. `calib.compare` has 5 rows, each with `off`, `on`, `off_pass`, `on_pass`, `lo`, `hi`, and `delta`. `calib.arms.off` and `calib.arms.on` each carry 16 keys. The 12 statistics files in each arm have the same names and the same match seeds, and `tuning.flags_on` is `[]` in the off arm and `["probe_short_range"]` in the on arm.
8. Schema check with jsonschema 4.26.0 (Draft 2020-12): the paired `report.json` gives 0 errors against `run-report.schema.json`. The 24 match-stats records give 0 invalid against `match-stats.schema.json` (`schema-check.txt`).
9. Refusal drives (`refusal-drive-owner.txt`, `refusal-drive.txt`): `simulate` on a tuning file whose flag has no owner exits 1 with `error: content refused: tuning tuning.json: flags.probe_short_range.owner: is required`. `calibrate --flag nope=on` on the shipped content exits 1 with `flags.nope: not declared in tuning.json`, before any match.
10. `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` ×3: pass. `bench.cpu_ms / bench.matches` is 418.8 / 415.8 / 418.8 ms (median 418.8, gate 460.7). Peak memory is 6.797 / 6.637 / 6.793 MB (median 6.793, gate 6.82). Processor time per tick is 1.482 / 1.4713 / 1.482 µs, with 282,600 ticks per match and `budget.pass: true` on all three (`bench-*.json`, `bench-summary.txt`).
11. Default-match identity: `simulate --seed 42 --ticks-out … --json --no-snapshot` gives 54,259,280 bytes. The tick records after the 64-byte header hash to `b126cd5979161c65…`, which is the value recorded for the parent builds `f6f36b6` and `ee2ce12` (`seed42-identity.txt`). The statistics record shows `tuning.flags_on: []` and `validate.violations: 0`.
12. Security (`security-boundary.txt`): `Cargo.lock` and every `Cargo.toml` are unchanged from `ee2ce12` to `aca285f` (empty diff stat). The secret-pattern scan over added lines found 0 hits. The change adds 0 `unsafe` blocks. `cargo audit` is not installed (`error: no such command: audit`).
13. Output-boundary and debt-marker scan over the build commit's added lines and message: 0 workflow terms (one false positive, `std::slice::from_ref`), and 0 `sdlc-debt` markers (`security-boundary.txt`).

## Interactive Verification Results

Automated only. Both criteria carry `observable: false` in `03-slice-experiment-flags.md` (AC-1 is a harness test, and AC-2 is a `cargo test`). The paired run by hand and the refusal drives in checks 6 to 9 are extra runtime evidence through the `cli` adapter. The gate does not require them.

## Acceptance Criteria Status

| # | Criterion | kind | status | method | evidence | evidence-rung |
|---|-----------|------|--------|--------|----------|---------------|
| AC-1 | "Given a flag defined in the tuning file, When calibration runs paired with the flag on and off on the same seeds, Then the report shows both bands side by side." | code-only (explicit `observable: false`) | met | automated | `a_paired_run_plays_both_arms_on_the_same_seeds_and_compares_every_band` passes (`test-ac.txt`). The hand-run paired run has the same file names and match seeds in both arms. Its `calib.compare` has 5 rows, each with the off and on values, both pass marks, and the band range. The stderr table shows the same rows side by side. The report is valid against its schema (`pair-report.json`, `pair-run.stderr.txt`, `schema-check.txt`). | n-a |
| AC-2 | "Given a flag without an owner, hypothesis, or removal condition, When the engine loads the tuning file, Then it refuses naming the flag." | code-only (explicit `observable: false`) | met | automated | `a_flag_missing_a_required_field_is_refused_naming_the_flag` covers 6 cases: each of the 3 fields missing or empty. Each case asserts the field path `flags.probe_flag.<field>`, the flag name, and "is required" (`crates/engine/tests/flags.rs:61`, `test-ac.txt`). The CLI drive refuses a flag with no owner and names `flags.probe_short_range.owner` (`refusal-drive-owner.txt`). | n-a |

Rollup: evidence: n-a 2 (code-only). User-observable 0; mock-rung 0.

## Issues Found

None.

## Augmentation Verification

- **Experiment (`04c-experiment.md`), this slice's own augmentation:**
  - Flag: the `flags` block in `content/tuning.json` has the required owner, hypothesis, and removal condition, and each flag must switch something. The shipped block is `{}` and is pinned by `the_shipped_flags_block_matches_the_code_flags`. Pass.
  - Cohort: the split is paired by fixture. Both arms play the same fixtures on the same match seeds (check 7). Pass.
  - Metrics: the primary metric `calib.compare` has one row per suite and band (check 7). Each arm reports the guardrails `darkpath.match_without_stats`, `darkpath.change_never_applied`, `validate.violations`, `calib.workers_failed`, and `bench.match_wall_ms`. On the hand run the on arm read 0 / 0 / 0 / 0 and 46 ms against 46 ms for the off arm. The verdict follows the §4 rule. The on arm's stronger-team win rate was 0.000, which is not above 0.50, so the guardrail breach gives `on-rejected`. A 6-match smoke run proves the wiring only, not a model. The four verdicts and the 0.05 margin boundary pass in the `report::compare` unit tests. Pass.
  - Rollback: `a_flag_the_file_turns_on_applies_at_load_and_the_command_line_turns_it_off` proves that `--flag name=off` undoes a flag that the file turns on. The removal checklist is in `docs/how-to/modding.md`, and the docs test passes. Pass.
- **Instrumentation (`04b-instrument.md`):** the new signals fired on the paired run: `flag.active` (2 lines, `source="cli"`) and `calibrate.pair` (1 line, `verdict="on-rejected"`) (`pair-run.stderr.txt`). `content.refused` fired on both refusal drives. `tuning.flags_on` is in every match-stats record, and all 24 records validate.
- **Benchmark (`05c-benchmark.md`, compare against the tactics-and-ai baseline):** processor time per match has a median of 418.8 ms against the 418.8 ms baseline (0.0 percent; gate 460.7 ms): pass. Peak memory has a median of 6.793 MB against 5.45 MB (+24.6 percent; gate 6.82 MB): pass. Earlier readings were 6.551 MB in the previous slice's verify and 6.582 MB in this slice's implement record. Headroom is under 0.5 percent (see Gaps). Processor time per tick is 1.482 µs against 1.4965 µs (−1.0 percent), reported beside the gate. No tripwire fired.
- **Craft (`02c-craft.md`):** this slice changes no visual surface, so no mock-fidelity item applies.

## Security Scan

- CVE: `cargo audit` is not installed. The build commit changes neither `Cargo.lock` nor any `Cargo.toml`, so no new dependency or version entered. New critical/high: 0.
- Secrets: the pattern scan over added lines (keys, tokens, passwords, private-key headers, AWS key ids) found 0 hits.
- SAST: clippy `-D warnings` is clean, and the change adds no `unsafe` block. Overrides are applied by dotted path to a parsed JSON tree and then validated again through the same `Deserialize` and `garde` rules, so an override cannot escape a bound (`an_override_outside_its_bound_is_refused_naming_the_flag`). New HIGH+: 0.

## Accessibility Gate

Not automatable for this slice. No UI element, style, or text surface changed. New WCAG AA violations: 0.

## Performance Gate

- Bundle size delta: skipped. No page file changed.
- Per-match processor time and memory are within the gates (see Augmentation Verification). No flag lookup reached the tick loop: the default match still plays 282,600 ticks, and its tick bytes are identical to the parent's.

## Cross-Slice Regression

The workspace suite covers every completed sibling slice's tests (engine-core, data-schemas-generator, stream-protocol, match-rules, tactics-and-ai, commentary, calibration, integration drivers, extra-time-penalties). All 364 pass. The unpaired calibration tests (`calibrate.rs`) pass unchanged. The page suite covers the four viewer slices, and all 127 pass. Regressions found: 0.

## Longitudinal Delta

- Default match: compared against the seed-42 hash recorded for the parent builds. The tick records are byte-identical. This is the expected result.
- Benchmark: compared against `05c-benchmark.md` and the last two recorded readings. Processor time is unchanged. Peak memory is 0.21 MB above the implement record's median and still under the gate.

## Friction Notes

- On a paired run, the stderr stream is dominated by per-match INFO signals from the single-thread timing matches, and the comparison table comes at the end. A reader who pipes stderr to a file finds the table in the last lines. Informational.

## Free Exploration Notes

- Hand run: the single-thread timing matches in both arms repeat the same injury at tick 6386. These matches use one fixed seed by design, so each repeat is the same match. Informational.
- On the 10-minute smoke run, both arms sit far outside the goals and shots bands (0.667 and 0.167 goals per match). This is expected for 10-minute matches and says nothing about the model. Informational.

## Adversarial Tests

| Test | Result | Finding |
|------|--------|---------|
| Owner, hypothesis, or removal condition missing or empty (6 cases) | pass | `a_flag_missing_a_required_field_is_refused_naming_the_flag` |
| A flag with no overrides and no code registration | pass | `a_flag_that_switches_nothing_is_refused_naming_it` |
| An override to a value that does not exist, while the flag is off | pass | `an_override_of_a_value_that_does_not_exist_is_refused_even_when_off` |
| Two flags that are on and set the same value | pass | `two_flags_on_that_set_the_same_value_are_refused_naming_both` |
| An undeclared flag name or a bad state on the command line | pass | `a_bad_flag_name_or_state_is_refused_before_any_match`, and a CLI drive that exits 1 before any match |
| Empty submission / double-click / offline | n-a | engine-only slice, no input surface |

## Failure Mode Probes

| Probe | Result | Finding |
|-------|--------|---------|
| Slow response / concurrent session / session expiry | n-a | engine-only slice, no session or network surface |

## Gaps / Unverified Areas

- The code-flag path (`CODE_FLAGS`, `ActiveFlags::is_on`) is empty and has no call site. No candidate model ships, so the path is untested until the first candidate registers a flag. This is deferred by design in the build record and is not an acceptance criterion.
- No 1000-match paired decision run was made. It runs when a candidate exists. This run proved the wiring on a 6-match smoke run only.
- Peak memory headroom is thin: the median is 6.793 MB against 6.82 MB. One of three drives read 6.637 MB, so part of the rise is noise. This run did not measure the parent build's memory in the same session, so the rise cannot yet be split between this slice and noise.
- `cargo audit` is not installed. The change adds no dependency, so the gap does not bear on this slice.

## Freshness Research

No dependency was added or upgraded (empty `Cargo.lock` diff). The implement record read `garde` 0.23.0 path nesting at source, and the observed refusal text `flags.probe_short_range.owner` confirms that behaviour at run time. No further source read was needed.

## Assumptions

Each entry is an autonomous decision of this run, with its class per `_decision-classes.md`.

- **V-1** AC-1 and AC-2 are code-only, from the explicit `observable: false` annotations (partition Step A, final). Each is classified `build-capability` and verified by `cargo test`, with a CLI drive as extra evidence. `class: implementation-detail`.
- **V-2** No sub-agents were dispatched. Cargo serialises on one target folder, and every check ran in sequence in this session. `class: implementation-detail`.
- **V-3** No prior verify artifact or evidence existed for this slice. The driver said that a prior round had applied one fix pass, but nothing on disk shows one. This run is recorded as evidence run 1 with `fix-rounds-run: 0`. `class: implementation-detail`.
- **V-4** The default-match identity check compared the tick hash with the value recorded for the parent builds (`b126cd59…`) and did not rebuild the parent again. Implement already rebuilt `ee2ce12` and matched. `class: implementation-detail`.
- **V-5** The paired run and the refusal drives used a copy of `content/` in the session scratchpad. The repository's content was not touched. `class: implementation-detail`.
- **V-6** Peak memory at 6.793 MB against a 6.82 MB gate is recorded as a pass, with the thin headroom listed under Gaps. It is not recorded as an issue, because the gate is the benchmark's own rule and no tripwire fired. `class: implementation-detail`.
- **V-7** `security-scan-result: pass` rests on the secret scan, clippy, the `unsafe` count, and a zero dependency diff. `cargo audit` is absent and was not installed. `class: implementation-detail`.
- **V-8** No consult ran. No trigger holds (every criterion is met by direct test evidence, and none is deferred), and the product owner excluded consult at intake. `class: implementation-detail`.

## Triage Decisions

None. There were 0 issues, so no Fix, Skip, or Escalate decision was needed.

## Recommendation

Ready for review. Both criteria are met by named, passing tests and by a paired run and a refusal drive on the command line. The default match is byte-identical to the parent.

## Recommended Next Stage

- **Option A (default): Review** → `/wf review football-manager-match-engine experiment-flags`. Convergence is `not-needed` and the result is `pass`.
- **Option D: Skip review** → `/wf handoff football-manager-match-engine experiment-flags`. This option is valid because `result: pass`. It is not recommended: the slice changes 26 files across the content loader and the calibration harness.
- **Option G: Slug-wide runtime probe** → `/wf probe football-manager-match-engine`. Use this for a cross-slice runtime sweep.
