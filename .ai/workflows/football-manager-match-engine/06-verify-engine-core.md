---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: engine-core
status: complete
stage-number: 6
created-at: "2026-09-21T22:40:58Z"
updated-at: "2026-09-21T22:40:58Z"
result: pass
metric-checks-run: 12
metric-checks-passed: 12
metric-acceptance-met: 6
metric-acceptance-total: 6
metric-acceptance-user-observable: 1
metric-acceptance-code-only: 5
metric-interactive-checks-run: 3
metric-interactive-checks-passed: 3
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
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/engine-core/"
evidence-run-count: 1
security-scan-result: pass
metric-a11y-violations-new: 0
a11y-result: not-automatable
cross-slice-regressions-found: 0
metric-bundle-size-delta-pct: "skipped"
ac-staleness-checked: false
ac-stale-count: 0
longitudinal-baseline-compared: "skipped — the base branch main holds no engine code; this is the first slice"
stability-check-flaky-count: 0
adversarial-tests-run: 11
adversarial-tests-failed: 0
failure-mode-probes-run: 3
cross-browser-delta: "none"
web-vitals-lcp-ms: null
web-vitals-cls: null
web-vitals-inp-ms: null
stack-source: confirmed
debt-markers-found: 2
debt-markers-malformed: 0
debt-markers-unrecorded: 0
skipped-gating-specs: []
consult-runs: []
tags: [engine, rust, benchmark, determinism]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-engine-core.md
  plan: 04-plan-engine-core.md
  implement: 05-implement-engine-core.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-engine-core.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine engine-core"
---

# Verify: Engine Core

## The Verification

Implement handed over two commits on the slice branch, 36 files, 32 tests, and one measured number: a 90-minute match in 398 milliseconds against a 2,000 millisecond budget. The slice carries six acceptance criteria. Five are code-only by explicit annotation and run under the test suite; one, the benchmark report, is user-observable and needs runtime evidence from the binary itself.

Twelve checks ran and twelve passed: format, lint, release build, the test suite, two secret scans, a manual advisory check, debt-marker hygiene, the instrumentation key check, the benchmark command, the criterion benches, and cold start. The benchmark was driven three times without a reset; every drive passed every criterion the plan named, with medians of 395, 398, and 397 milliseconds, so the stability count is zero. Eleven adversarial probes and three failure-mode probes found no crash and no unhandled error: every bad argument produced one message and a non-zero exit, a killed run left a file without a trailer that the reader refuses, and two concurrent runs wrote byte-identical files. The benchmark comparison against the budget line recorded four targets under budget and no tripwire.

Review is next. The top open risk is unchanged from implement: 0 goals in the seed-42 and seed-7 matches is plausible for a kick-about without laws of the game but is untuned, and the `calibration` slice owns the realism bands. Two low-severity observations from free exploration ride to review as informational findings: the error message for an unwritable output path repeats the operating-system text twice, and the diagnostic lines on stderr carry colour codes when stderr is a file.

## Verification Summary

- Result: `pass`. Every criterion met; the one user-observable criterion has live evidence from three drives.
- Convergence: `not-needed`. No check failed, no criterion was unmet, no evidence was missing.
- Adapter: `cli`, driven directly (`cli-direct`) on the reference machine, as the plan's `## Verification Strategy` prescribed.
- Sub-agents: none. Every check is one cargo or binary command on this machine, cargo serialises on the target directory, and the benchmark requires an uncontended CPU, so the coordinator ran the concerns in sequence and captured evidence per the cli adapter's layout.
- Benchmark compare mode: recorded in `05c-benchmark.md`; four targets measured, zero regressions, zero tripwires.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (exit 0; `verify-evidence/engine-core/check-fmt.exit-code`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass (exit 0, no warnings; `check-clippy.stderr.txt`).
- `cargo build --release --workspace`: pass (exit 0; 2,138 ms wall after a clean of the two workspace crates; binary 1,913,856 bytes; `check-build-release.*`).
- `cargo test --workspace` (debug profile): pass (32 passed, 0 failed, 0 ignored; 17,478 ms wall; `check-test.stdout.txt`). Breakdown: 26 unit tests in `engine`, 1 in `full_match`, 1 in `determinism`, 2 in `validator`, 2 in `cli_args`.
- `gitleaks git --log-opts="main..HEAD"`: pass (2 commits scanned, no leaks; `check-gitleaks.stderr.txt`).
- Secret pattern grep over `git diff main...HEAD` (api key, secret, password, token, credential assignments): pass (no match).
- Dependency advisory check: pass. `cargo-audit` is not installed; the lockfile was checked by hand against the RustSec package pages. `rand` 0.10.3 is inside the patched range of RUSTSEC-2026-0097 (`>= 0.10.1`), and the engine never calls `rand::rng()` or a thread-local generator. `rand_chacha` 0.10.0 has no advisory page. `glam` 0.33.8, `serde_json` 1.0.151, `clap` 4.6.7, `criterion` 0.8.2, `sha2` 0.10.9, `windows-sys` 0.61.2 match the plan's freshness list.
- `sdlc-debt:` marker hygiene over the branch diff: pass. 2 markers found (`crates/engine/src/sim.rs:187`, `crates/engine/src/validate.rs:77`); both name a ceiling and an upgrade path; both are recorded under `## Anything Deferred` in `05-implement-engine-core.md`. 0 malformed, 0 unrecorded.
- Instrumentation key check (`04b-instrument.md` §2 against live records): pass. `match-stats`: 19 planned keys, 19 present, 1 additive extra (`goals`). `run-report`: 20 planned keys, 20 present, 1 additive extra (`owner.id`, from the shared envelope). Types hold: `duration_ms` and `bench.match_wall_ms` are integers, `engine.ticks_per_s` is a float, `outcome` is `success`, `env` is `dev` and follows `SM_ENV`. `instrument-key-check.txt`.
- `target/release/engine-cli.exe bench --seed 42 --matches 5 --json`: pass (exit 0; median 395 ms; `ac-e.*`). Details under Interactive Verification Results.
- `cargo bench -p engine --bench tick_step`: pass (exit 0; `tick_step` 1.4615 µs, `steering_pass_22` 875.43 ns; `check-criterion.stdout.txt`).
- Cold start `engine-cli --version` five times: pass (37 to 39 ms; `perf-cold-start-ms.txt`).

## Interactive Verification Results

- **Criterion:** AC-e. "Given the benchmark harness on the reference laptop with one thread, When it simulates one 90-minute match, Then wall time is under 2 seconds and the report records machine name and build hash."
- **Platform & tool:** `cli` adapter, `cli-direct`: the release binary on the reference machine (Windows 11, AMD Ryzen 7 9800X3D, Ultimate Performance plan, rustc 1.92.0). Stack intersection: `cli` matched by `Cargo.toml` binary crate and is listed in `stack.platforms`; `web` is listed in `stack.platforms` but no web detection signal matched (no `package.json`, no HTML), so no web adapter ran.
- **Steps performed:**
  1. Bootstrap: `cargo build --release --workspace`; executable at `target/release/engine-cli.exe`.
  2. Enumerate (recipe rung, clap tree via `--help`): 2 subcommands, `simulate` with 4 flags and `bench` with 4 flags; `enumerate.help.txt`.
  3. Drive 1: `bench --seed 42 --matches 5 --json` with stdout, stderr, and exit code captured.
  4. Drive 2 and drive 3: the same command without a reset (stability check).
  5. Parse the JSON line and check every pass criterion from the plan's `## Verification Strategy`.
- **Evidence:** `verify-evidence/engine-core/ac-e.stdout.txt`, `ac-e.stderr.txt`, `ac-e.exit-code`; `ac-e-redrive-2.*`; `ac-e-redrive-3.*`.
- **Observation:** one `run-report` record per drive. Drive 1: `bench.match_wall_ms` 395, `budget.pass` true, `machine.hash` `74ca12fc08a4` (12 hex characters), `machine.cpu_model` "AMD Ryzen 7 9800X3D 8-Core Processor", `machine.power_plan` "Ultimate Performance", `build.hash` `27d21f5-dirty`, `bench.cpu_wall_ratio` 0.985, `bench.cpu_ms` 1,953 over 5 matches, `bench.peak_mem_mb` 5.05, `engine.ticks_per_s` 683,544, exit code 0. Drives 2 and 3: 398 ms and 397 ms, ratios 0.987 and 0.998, exit 0, every other field equal apart from `run.id`. Stderr was empty on every drive.
- **Result:** pass. Every plan criterion holds on every drive; `stability: stable`. The `-dirty` suffix on the build hash is expected: the cost ledger is appended by a hook on every turn and two plugin-owned untracked files sit under `.ai/`, so the working tree is never clean while a workflow runs. The hash still names commit `27d21f5`.
- **evidence-rung:** `live`.

## Acceptance Criteria Status

- **AC-a** "Given a seed and two built-in teams, When `engine-cli simulate` runs 90 minutes, Then the output holds 270,000 ticks, each with one ball position and 22 player positions." kind: `code-only` (annotated `observable: false`). status: met. method: automated. evidence: `ninety_minutes_write_270_000_records` passed; the live run `simulate-42.*` wrote 51,840,080 bytes, which equals 64 + 270,000 × 192 + 16, and the record reports `ticks.written` 270,000. evidence-rung: n-a.
- **AC-b** "Given the same seed and build on the same machine, When two simulations run, Then the two tick outputs are byte-identical." kind: `code-only`. status: met. method: automated. evidence: `two_runs_are_byte_identical` passed; the concurrent probe also produced byte-identical files (`probe-concurrent.cmp.txt`). evidence-rung: n-a.
- **AC-c** "Given any tick, Then no player is outside the pitch bounds, no two players share a point within 0.1 m, and ball speed is at most 40 m/s." kind: `code-only`. status: met. method: automated. evidence: `a_seeded_full_match_has_no_violations` passed; `a_corrupted_stream_reports_the_overlap` proves the rule fires; live runs for seeds 42 and 7 report `validate.violations` 0 with `ball.max_speed` 27.07 and 28.57. evidence-rung: n-a.
- **AC-d** "Given open play with the ball more than 30 m from a player, Then that player's distance to the formation anchor stays within the tolerance the tuning constants set." kind: `code-only`. status: met. method: automated. evidence: the anchor idle-drift rule runs inside `a_seeded_full_match_has_no_violations` and inside the `simulate` command; both report 0 violations. evidence-rung: n-a.
- **AC-e** "Given the benchmark harness on the reference laptop with one thread, When it simulates one 90-minute match, Then wall time is under 2 seconds and the report records machine name and build hash." kind: `user-observable` (annotated `observable: true`). status: met. method: interactive. evidence: `ac-e.stdout.txt`, `ac-e.exit-code`, two re-drives. evidence-rung: `live`.
- **AC-f** "Given an invalid seed argument, When `engine-cli simulate` runs, Then it exits non-zero with a message naming the argument; and Given a valid seed, Then it exits zero." kind: `code-only`. status: met. method: automated. evidence: `invalid_seed_exits_non_zero_and_names_the_argument` and `valid_seed_exits_zero` passed; the live probe `adv-invalid-seed.*` shows exit 2 and the message "invalid value 'abc' for '--seed <SEED>'". evidence-rung: n-a.

evidence: live 1 / n-a 5. `metric-acceptance-mock-rung`: 0.

## Issues Found

None.

## Augmentation Verification

- **instrument** (`04b-instrument.md`, status ready): pass. Signals exercised on the live binary: `match-stats.simulate` (record present with 19 of 19 planned keys), `run-report.bench` (20 of 20), `engine.ticks_per_s` (present in both records), `bench.cpu_wall_ratio` (0.985 on the full run), `validate.violation` (count carried as `validate.violations`; the log line path is exercised by the corrupted-stream test), `tickfile.header` and `tickfile.trailer` (both tracing lines observed on stderr in `simulate-42.stderr.txt`), `diag.simulate.span` (the `simulate{seed=42 minutes=90}` span prefixes every line). Dark paths: the partial-file path was induced by killing a run at 150 ms; the file had 136,525 complete records and no trailer (`adv-interrupt.exit-code`), and `a_truncated_file_is_refused` covers the reader's refusal. `validate.ran` is true in every record. `possession.changes` 773 and `ball.idle_ticks` 854 make the no-possession path visible. No personal identifier appears in any record. Missing signals: none.
- **benchmark** (`05c-benchmark.md`, mode baseline → complete): pass. Compare mode ran the two recorded commands. Full match wall time 395 ms against a 2,000 ms budget (−80.3 percent); ticks per second 683,544 against 135,000 (+406 percent); tick step 1.4615 µs against 7.4 µs (−80.2 percent); CPU 391 ms per match and peak memory 5.05 MB recorded as the first measured values for the tripwires. Regressions: none. Tripwires: none. The 1000-match half of AC-19 projects to 6.6 minutes against 30 and stays a projection until the `calibration` slice runs the batch.
- **experiment**: deferred to the `experiment-flags` slice per the index; no `04c-experiment.md` exists, so no re-check applies.
- Mock fidelity inventory: `02c-craft.md` does not exist; no inventory applies. Design findings: no `07-design-audit.md` or `07-design-critique.md` exists.

## Security Scan

- CVE scan: `cargo-audit` not installed; manual lockfile check against RustSec. New critical or high advisories: 0. `rand` 0.10.3 is patched for RUSTSEC-2026-0097; the trigger (`rand::rng()` from a custom logger) is absent from the code.
- Secret detection: `gitleaks` 8.30.1 over `main..HEAD`, 2 commits, no leaks. Pattern grep over the diff: no finding.
- SAST: `semgrep` not installed; skipped. Clippy at `-D warnings` is the static floor and is clean.

## Accessibility Gate

Not automatable: the slice ships a headless command line and no user interface. New WCAG AA violations: 0 (no surface).

## Performance Gate

- Bundle size: skipped; the base branch `main` holds no code, so no delta exists. Absolute artifact: `engine-cli.exe` 1,913,856 bytes (release, with debug info).
- Build time: 2,138 ms for the two workspace crates from clean (dependencies cached); no base to compare.
- Cold start: 37 to 39 ms for `--version` over five runs; no base to compare.
- The `benchmark` augmentation carries the engine numbers (see Augmentation Verification).

## Cross-Slice Regression

None — first slice. `cross-slice-regressions-found: 0`.

## Longitudinal Delta

Skipped. No prior evidence run exists and the base branch has no engine code. This run's evidence directory is the baseline for the next verify of any engine slice.

## Friction Notes

- The build hash carries `-dirty` during every workflow run because the cost ledger is hook-appended each turn. A reader of the report cannot tell a code change from ledger noise by the hash alone; the commit part is exact.
- `bench.cpu_wall_ratio` exceeds 1.0 on a 5-minute, 1-match run (1.48 in `adv-env-staging.stdout.txt`): the process-time clock ticks at 15.6 ms, so the ratio is meaningful only on runs of several seconds. The full 5-match run is well inside that regime.
- The help text is complete and the two subcommands mirror the README; the `--json` flag on `bench` is a no-op by documented design.

## Free Exploration Notes

- Unwritable output path (`adv-bad-dir.stderr.txt`): the message reads "cannot create Z:/no/such/dir/x.ticks: io error: The network path was not found. (os error 53): The network path was not found. (os error 53)". The operating-system text appears twice because the error type's display already includes its source and the binary prints the chain as well. Class: `ambiguous-copy`. informational, LOW.
- Diagnostic lines on stderr carry ANSI colour codes when stderr is redirected to a file (`simulate-42.stderr.txt`). Class: `branch-gap` (non-tty path). informational, LOW.
- `bench --json` and `bench` without the flag print the same record; the flag exists for symmetry with `simulate --json`. informational.
- A 1-minute match with the maximum seed value runs and reports 7 possession changes; the large seed is not special-cased. informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| No arguments (`adv-no-args`) | pass | usage text, exit 2 |
| Invalid seed `abc` (`adv-invalid-seed`) | pass | message names `--seed`, exit 2 |
| Seed above u64 (`adv-seed-overflow`) | pass | "number too large to fit in target type", exit 2 |
| Maximum seed u64::MAX, 1 minute (`adv-seed-max`) | pass | runs, exit 0 |
| `--minutes 0` (`adv-minutes-0`) | pass | "minutes must be 1 to 200, got 0", exit 1 |
| `--minutes 201` (`adv-minutes-201`) | pass | "minutes must be 1 to 200, got 201", exit 1 |
| `bench --matches 0` (`adv-matches-0`) | pass | "--matches must be at least 1", exit 1 |
| Unknown flag `--threads` (`adv-unknown-flag`) | pass | "unexpected argument", exit 2 |
| Unwritable output path (`adv-bad-dir`) | pass | one message, exit 1; text repeated (LOW, see Free Exploration) |
| Rapid repeat to one path (`adv-rapid-repeat-1`, `-2`) | pass | second run overwrites; identical statistics |
| Mid-flow interruption, kill at 150 ms (`adv-interrupt`) | pass | 26,212,864-byte file, no trailer, reader refuses per unit test |
| Empty submission | n-a | no interactive input surface |
| Offline / network failure | n-a | no network dependency |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Concurrent session: two `simulate` runs at once (`probe-concurrent-*`) | pass | both exit 0; tick files byte-identical; statistics equal |
| Environment perturbation: `SM_ENV=staging` (`adv-env-staging`) | pass | `env` reads `staging`; every other field unchanged |
| Non-tty: stdout and stderr redirected to files (every probe) | pass | JSON on stdout intact; colour codes on stderr (LOW, see Free Exploration) |
| Slow response | n-a | no remote dependency to throttle |
| Session expiry | n-a | no authentication in scope |

## Gaps / Unverified Areas

- The 1000-match batch (second half of shape AC-19) is projected from the single-match median and not measured; the `calibration` slice runs it.
- Coverage is not measured; `cargo-llvm-cov` is not installed and the plan named no coverage tool.
- Cross-platform determinism is out of scope by NFR-4; only same-machine reproduction was proven.

## Freshness Research

- Source: [RUSTSEC-2026-0097](https://rustsec.org/advisories/RUSTSEC-2026-0097.html)
  Why it matters: the only advisory on the `rand` package; the lockfile must sit in the patched range.
  Takeaway: affected `>= 0.10.0, < 0.10.1`; patched `>= 0.10.1`; the lockfile holds 0.10.3, and the trigger path (`rand::rng()` under a custom logger) is absent.
- Source: [RustSec package page for rand](https://rustsec.org/packages/rand.html); the `rand_chacha` page returns 404, which the site uses for packages with no advisory.
  Why it matters: `cargo-audit` is not installed, so the advisory database was read by hand.
  Takeaway: one advisory on `rand`, none on `rand_chacha`; no verify-time install of `cargo-audit` because the plan never named it.
- Plan age: 0 days; no test failed; no external API is touched. Sub-agent 5 was not launched; `ac-staleness-checked: false`.

## Recommendation

Proceed to review. The slice meets every criterion with live evidence for the benchmark, and no fix was needed. Carry the two LOW copy findings and the realism caveat into the review ledger as informational items.

## Recommended Next Stage

- **Option A (default):** `/wf review football-manager-match-engine engine-core` — result `pass`, convergence `not-needed`. Compact the session first: the check output and probe captures are noise for review dispatch.
- **Option D:** `/wf handoff football-manager-match-engine engine-core` — valid because the result is `pass`; not recommended, because the slug's review scope is slug-wide and this is the root slice that every later slice consumes.
- **Option G:** `/wf probe football-manager-match-engine` — available for a slug-wide runtime sweep; low value with one slice and no cross-slice surface yet.
