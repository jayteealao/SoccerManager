---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: probe-engine-core
status: awaiting-input
stage-number: 4
created-at: "2026-09-22T22:29:40Z"
updated-at: "2026-09-23T08:44:00Z"
metric-files-to-touch: 10
metric-step-count: 11
has-blockers: true
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-23T08:44:00Z"
    trigger: manual
    because: "auto-review — 6 issues found"
    changed: "Q-1 recorded as answered (A). New intent-bearing Q-2: the answered record shape fails the record schemas that shipped with calibration (outcome enum success/failure; statistic keys required). Re-grounded on the current code: a ninth help surface (calibrate, 145 columns under -h), the calibration worker's error classifier reused instead of a new one, the Snapshot error variant, the shared test helpers and schema validator, and main.rs line numbers."
    snapshot: history/04-plan-probe-engine-core-0.md
consult-runs: []
tags: [probe, cli, observability, help-text, record-schemas, awaiting-input]
stack-source: confirmed
open-questions:
  - id: Q-1
    class: intent-bearing
    status: answered
    answer: "A — one stdout record with outcome error, error.type, error.code, error.retriable, the command's success record.kind, no statistic key; nothing persisted; exit 1 with the prose line kept (po-answers.md, 2026-09-23T08:37:28Z)."
  - id: Q-2
    class: intent-bearing
    status: open
    question: "The Q-1 record fails the record schemas in schemas/observability/: they accept outcome success or failure only and require every statistic key. Which contract changes?"
    options: [A-amend-schemas-additively-keep-error, B-use-failure-word-amend-required-keys-only, C-unify-on-error-including-calibration]
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-probe-engine-core.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md, 04-plan-commentary.md, 04-plan-calibration.md]
  contract: ../../observability.md
  implement: 05-implement-probe-engine-core.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine probe-engine-core"
---

# Plan: Probe Findings on the Engine Command Line

## The Plan

The probe of the engine command line gave five findings. Since the first plan, three slices landed: tactics-and-ai, commentary, and calibration. This run built the current release binary and drove every finding again. Findings 1, 3, and 4 stay fixed: `--json` wrote `match.jsonl` beside `match.ticks`, an unwritable path named "os error" once, and redirected stderr held 0 escape bytes. Finding 5 is still open under `-h`, and it is now worse. Calibration added a ninth help surface, and `calibrate -h` reaches 145 columns because clap prints the possible values on the option line. Steps 1 to 4 fix the help text and add regression tests. They need no answer from the product owner.

The product owner answered Q-1 with option A: a failed `simulate` or `bench` run prints one record with `outcome: "error"`, the three `error.*` keys, and no statistic key. But calibration shipped record schemas after Q-1 was asked. `schemas/observability/match-stats.schema.json` and `run-report.schema.json` accept `outcome` `success` or `failure` only, and they require every statistic key (32 required keys in `match-stats`, 16 in `run-report`). A record built as answered fails both schemas, and the test suite checks records against these schemas with an independent validator (`crates/engine-cli/tests/schemas.rs`). To implement Q-1, a published data contract must change: the schemas, the outcome word, or both. That is a product decision. For this reason the plan stops at **awaiting-input** on one new question, Q-2, and recommends option A. Steps 5 to 10 are written for option A and wait for the answer.

When Q-2 is answered, run this stage again. The run clears the blocker and adjusts steps 5 to 10 if the answer is B or C. The top risk after Q-2 is the word for a failed outcome. Calibration writes `failure`, and the observability contract and the Q-1 answer say `error`. Only option C removes the difference.

## Current State

- Release binary `target/release/engine-cli.exe`, built in this run (`cargo build --release --workspace`, exit 0) at HEAD `3ec2198`. This run drove it.
- Finding 1 (dump path): fixed. `simulate --seed 1 --minutes 1 --no-snapshot --json --ticks-out <tmp>/match.ticks` exited 0 and wrote `match.jsonl` and `match.ticks`. The `simulate --json` long help states the replace rule (`crates/engine-cli/src/cli.rs:60-67`). `resume --json` uses the same rule (`crates/engine-cli/src/resume.rs:129`), but its help says only "Also write a JSON Lines dump beside the tick file" (`cli.rs:92-94`). No test asserts the dump path.
- Finding 2 (no record on failure): open. `simulate --minutes 0` exits 1 with `error: invalid configuration: minutes must be 1 to 200, got 0` and 0 bytes on stdout. A directory as `--ticks-out` exits 1 with 0 bytes on stdout. `bench --matches 0` exits 1 with `error: --matches must be at least 1` and 0 bytes on stdout. The error branch in `crates/engine-cli/src/main.rs:44-50` prints prose only. `bench.rs:22-23` uses an untyped `anyhow::bail!`. `calibrate --matches 0` also exits 1 with prose only; the Q-1 answer names `simulate` and `bench` only.
- Finding 3 (repeated OS text): fixed. `main.rs:47` prints `{err:#}`, and the directory drive named "os error" once. No test asserts this.
- Finding 4 (ANSI on redirected stderr): fixed. `main.rs:30` sets `with_ansi(std::io::stderr().is_terminal())`, and the redirected stderr of the dump drive held 0 ESC bytes. No test asserts this.
- Finding 5 (help width): fixed under `--help` only (78 to 80 columns on all 9 surfaces). Under `-h`: 94 columns on the top level, `simulate`, `generate`, `serve`, `record`, and `resume`; 97 on `bench`; 102 on `replay`; 145 on `calibrate`. The long lines are `--content-dir` (every surface), `bench --matches`, `--json`, `--stream`, `replay --speed`, and `calibrate --seed`, `--jobs`, `--suite`, `--keep-events`. The `--suite` and `--keep-events` lines carry `[possible values: …]`. `no_help_line_exceeds_eighty_columns` (`crates/engine-cli/tests/cli_args.rs:183-198`) reads `--help` on 5 of the 9 surfaces and never reads `-h`.
- Record schemas: `schemas/observability/match-stats.schema.json` requires 32 keys, including `stats.goals`, `stats.shots`, `validate.ran`, `engine.ticks_per_s`, and `darkpath.change_never_applied`; `outcome` is `enum: [success, failure]`; `error.type` is an optional string; `error.code` and `error.retriable` are not declared (additional properties are allowed). `run-report.schema.json` requires 16 keys, including `machine.hash`, `bench.match_wall_ms`, `bench.cpu_us_per_tick`, and `bench.peak_mem_mb`, plus `bench.matches`, `bench.ticks_per_match`, and `budget.pass` when `operation` is `benchmark`; `outcome` is `enum: [success, failure]`.
- Calibration already writes a failed match: `crates/engine-cli/src/calibrate/worker.rs:160-183` builds a `MatchStats` with zero statistics, `outcome: "failure"`, and `error.type`. A private classifier, `error_type()` (`worker.rs:185-195`), maps each `EngineError` variant to `invalid-config`, `io`, `format`, `sink`, `content`, or `snapshot`. The calibration run report writes `outcome: "failure"` when a worker fails (`calibrate/mod.rs:180-183`).
- `EngineError` (`crates/engine/src/error.rs:7-48`) now has 8 variants, including `Snapshot`.
- The observability contract (`.ai/observability.md:17-19`) defines `outcome: [operation, duration_ms, outcome, status]` and `error: [error.type, error.code, error.retriable]`. The instrumentation contract lists `outcome` as `success`, `error`, or `abandoned`.
- Tests: `crates/engine-cli/tests/` holds `calibrate.rs`, `cli_args.rs`, `resume.rs`, `schemas.rs`, `stream_cli.rs`, `web_cli.rs`, and `common/mod.rs`. `common` gives `temp()`, `bin()` with a scratch `SM_DATA_DIR`, `record()`, and `RecordSchemas` (the `boon` validator over the schema files). `cli_args.rs` has its own `bin()`, `content_dir()`, and `temp()` (`cli_args.rs:9-23`).

## Simplicity Ladder

- Help width under `-h` → rung 4 new-code (text only). Rung 2: clap wraps help only with the `wrap_help` feature (`clap_builder-4.6.7/src/output/textwrap/mod.rs:26-28`), which adds the `terminal_size` dependency. It is rejected. `Arg::hide_possible_values` is built in (`clap_builder-4.6.7/src/builder/arg.rs:2570`) and removes the `[possible values: …]` suffix. It is used for `--suite` and `--keep-events`, and `long_help` names the values.
- Dump-path, single OS-error, and no-ANSI tests → rung 3 reuse: `bin()`, `content_dir()`, and `temp()` in `crates/engine-cli/tests/cli_args.rs:9-23`. `std::process::Command` captures stderr as a pipe, which is the non-terminal case.
- Error classification (awaiting Q-2) → rung 3 reuse with modification: move `error_type()` from `crates/engine-cli/src/calibrate/worker.rs:185-195` into `EngineError` in `crates/engine/src/error.rs`. The calibration output does not change. This replaces the new classifier of the first plan, whose values (`config`) differed from the values calibration already writes (`invalid-config`).
- Failure record (awaiting Q-2) → rung 3 reuse with modification: `Record`, `to_json`, and `emit_line` (`crates/engine/src/observe/mod.rs:48-52`, `349-375`) already write the envelope keys. A new `FailureRecord` implements `Record`. Reusing `MatchStats` with zero statistics, as the calibration worker does, is rejected, because the Q-1 answer says no statistic key.
- Schema check of the failure record (awaiting Q-2) → rung 3 reuse: `RecordSchemas`, `bin()`, `temp()`, and `record()` in `crates/engine-cli/tests/common/mod.rs`.

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist, and no `solutions.globalDir` is configured (`.ai/sdlc-config.json` is absent).

Repeat-deferral tripwire: `runtime-evidence-deferrals` in `00-index.md` is `[]`. The tripwire does not fire.

## Likely Files / Areas to Touch

- `crates/engine-cli/src/cli.rs`: shorten nine short-help strings; hide the possible values of two options; add the replace rule to `resume --json`.
- `crates/engine-cli/tests/cli_args.rs`: widen the 80-column test; add regression tests for findings 1, 3, and 4; add failure-record tests (awaiting Q-2).
- `crates/engine/src/error.rs`: `error_type()`, `error_code()`, and `retriable()` on `EngineError` (awaiting Q-2).
- `crates/engine-cli/src/calibrate/worker.rs`: call the moved classifier (awaiting Q-2).
- `crates/engine/src/observe/mod.rs`: `FailureRecord` (awaiting Q-2).
- `crates/engine-cli/src/main.rs`: emit the failure record on the error branch of `simulate` and `bench` (awaiting Q-2).
- `crates/engine-cli/src/bench.rs`: a typed error for `--matches 0` (awaiting Q-2).
- `schemas/observability/match-stats.schema.json` and `run-report.schema.json`: the contract change Q-2 decides.
- `README.md`: describe the failure record (awaiting Q-2).

## Proposed Change Strategy

Work in two groups. Group one (steps 1 to 4) does not depend on Q-2. It completes findings 1, 3, 4, and 5 with help-text edits and regression tests. It changes no runtime behaviour except the help text.

Group two (steps 5 to 10) implements the Q-1 answer under option A of Q-2. The error branch of `simulate` and `bench` prints one record on stdout before the existing prose line. The exit code stays 1. The record carries the envelope keys, `operation`, `seed`, `duration_ms`, `outcome: "error"`, `error.type`, `error.code`, and `error.retriable`, and no statistic key. `simulate` fails as `record.kind: match-stats`, and `bench` fails as `record.kind: run-report`. Nothing is written under the data folder. The two schemas accept the record through an additive, conditional change, so every record that is valid today stays valid. Argument-parse failures (exit 2) happen before a run starts, so they stay prose-only.

The NFR rationale is charter C4 ("emits structured events and statistics for the observability pipeline"), which the product owner ratified. No unranked NFR narrows a charter commitment in this plan.

## Step-by-Step Plan

1. **Short help within 80 columns (finding 5).** In `crates/engine-cli/src/cli.rs`, shorten the doc comment of each option whose `-h` line exceeds 80 columns. The description budget is 80 minus the option column of that subcommand: 53 characters for `bench` and `replay`, 45 for `calibrate`, including clap's `[default: …]` suffix. Proposed text: `--content-dir` "Folder holding the content files."; `bench --matches` "Timed matches after one warm-up match."; `bench --json` "Print the run report as one JSON line."; `bench --stream` "Also stream one match to a fast client."; `replay --speed` "Playback speed; 1.0 is real time."; `calibrate --seed` "Seed of the run and of every match."; `calibrate --jobs` "Worker processes; default one per core."; `calibrate --suite` "Suites to run."; `calibrate --keep-events` "Event files to keep.". Add `hide_possible_values = true` to `--suite` and `--keep-events`, and name the values in each option's `long_help`. Move every removed detail into `long_help`, so `--help` keeps it. Keep every `--help` line at 80 columns or fewer.
2. **Resume dump-path help (finding 1).** Give `ResumeOpts::json` the same `long_help` as `SimulateOpts::json` (`cli.rs:60-67`), which states that the extension is replaced by `.jsonl`.
3. **Widen the help test (finding 5).** In `crates/engine-cli/tests/cli_args.rs`, change `no_help_line_exceeds_eighty_columns` to read both `-h` and `--help` for the top level and for `simulate`, `bench`, `generate`, `serve`, `record`, `replay`, `resume`, and `calibrate`. That is 18 checks.
4. **Regression tests for findings 1, 3, and 4.** In `cli_args.rs`, add:
   - `json_dump_replaces_the_tick_file_extension`: `simulate --seed 7 --minutes 1 --no-snapshot --json --ticks-out <tmp>/match.ticks`. Assert exit 0, `<tmp>/match.jsonl` exists with 3,000 lines, and `<tmp>/match.ticks.jsonl` does not exist.
   - `an_unwritable_tick_path_names_the_os_error_once`: create `<tmp>/dir`, run `simulate --seed 7 --minutes 1 --no-snapshot --ticks-out <tmp>/dir`. Assert exit 1, stderr names the path, and "os error" occurs exactly once.
   - `redirected_stderr_carries_no_ansi_escape`: run `simulate --seed 7 --minutes 1 --no-snapshot` with stderr captured. Assert the stderr bytes contain no `0x1B`.
5. **(Awaiting Q-2) Error classification in the engine.** Move `error_type()` from `crates/engine-cli/src/calibrate/worker.rs:185-195` into `crates/engine/src/error.rs` as `EngineError::error_type(&self) -> &'static str`, with the same values. Add `error_code()`: `InvalidConfig` → `invalid-config`, `Io` → `io`, `Read` → `read`, `Format` → `tick-format`, `Sink` → `sink-stopped`, `Data` → `content-refused`, `Version` → `content-version`, `Snapshot` → `snapshot-refused`. Add `retriable()`: `true` for `Sink` only. The calibration worker calls `err.error_type()`. Unit-test every variant, and keep `crates/engine-cli/tests/calibrate.rs` passing unchanged.
6. **(Awaiting Q-2) Failure record.** In `crates/engine/src/observe/mod.rs`, add `FailureRecord { kind: &'static str, operation: &'static str, owner_id: String, seed: u64, duration_ms: u64, error_type: &'static str, error_code: &'static str, error_retriable: bool }`. Implement `Record`, so that `kind()` and `operation()` return the stored values. Serialize `outcome` as `"error"` and the keys `error.type`, `error.code`, and `error.retriable`. Use the owner identity only when the owner file already exists; a failure path never creates it. Add a unit test that asserts the key set: the envelope keys, `operation`, `seed`, `duration_ms`, `outcome`, and the three `error.*` keys, with no `ticks.written`, `stats.*`, or `bench.*` key.
7. **(Awaiting Q-2) Typed bench check.** In `crates/engine-cli/src/bench.rs:22-23`, return `EngineError::InvalidConfig("--matches must be at least 1".into())` in place of `anyhow::bail!`. The stderr text becomes `error: invalid configuration: --matches must be at least 1`.
8. **(Awaiting Q-2) Emit on the error branch.** In `main.rs`, take `Instant::now()` and the command's seed before dispatch. On `Err` from `Simulate`, print one `FailureRecord` with kind `match-stats` and operation `simulate` through `emit_line`. On `Err` from `Bench`, use kind `run-report` and operation `benchmark`. Classify through the `anyhow` chain with `downcast_ref::<EngineError>()`; when no `EngineError` is found, use `internal`, `unclassified`, `false`. Then print the existing prose line on stderr and exit 1. If `emit_line` fails, still print the prose line and exit 1. `generate`, `serve`, `record`, `replay`, `resume`, and `calibrate` keep prose only.
9. **(Awaiting Q-2; contract) Schema amendment, option A.** In `schemas/observability/match-stats.schema.json`: add `error` to the `outcome` enum; declare `error.code` (string, `minLength` 1) and `error.retriable` (boolean); move every statistic key out of the top-level `required` list into an `if outcome is not error then required` block; add `if outcome is error then required [error.type, error.code, error.retriable]`. Keep the envelope, `match.id`, `seed`, `outcome`, and `duration_ms` required always; a failed `simulate` has a `match.id` only if one was assigned, so if no `match.id` exists before the failure, `match.id` also moves into the conditional block. Apply the same rule to `run-report.schema.json` for `run.id`, `content.hash`, `machine.hash`, and the `bench.*` keys. Keep `schema.version` at 1, because every record valid today stays valid. The implementer decides which keys the failure path can fill from the code, and the test in step 10 proves the result.
10. **(Awaiting Q-2) Failure tests and README.** Using `crates/engine-cli/tests/common`, add three tests. `simulate --minutes 0` gives exit 1, exactly one stdout line with `record.kind` `match-stats`, `outcome` `error`, and `error.type` `invalid-config`, the prose line on stderr, and `RecordSchemas::stats` accepts the record. A directory as `--ticks-out` gives `error.type` `io`. `bench --matches 0` gives `record.kind` `run-report`, `outcome` `error`, `error.type` `invalid-config`, and `RecordSchemas::report` accepts it. Add one negative schema check: a `match-stats` record with `outcome` `success` and no `stats.goals` is refused. Keep `valid_seed_exits_zero` and every test in `schemas.rs` unchanged as success-path guards. In `README.md`, state in the simulate and benchmark sections that a failed run prints one record with `outcome` `error` and exits 1.
11. **Gates and record.** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p engine-cli`, `cargo test -p engine`, and `cargo build --release --workspace`. Run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` once, and confirm that `bench.match_wall_ms` is within 10 percent of the latest `05c-benchmark.md` baseline. No simulation path changes, so this is a guard only. Search the diff for workflow vocabulary (slice names, stage names, artifact paths) before the commit. Record the numbers and the resolution of each finding in the implement artifact.

## Verification Strategy

Every row is a user-observable command-line behaviour from the probe findings. The `cli` adapter drives the real release binary as a subprocess, and the stack confirms `cargo-test` (`00-index.md` `stack.testing`).

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| F1: the `--json` dump path is the one the help names | `cargo test -p engine-cli` subprocess drive plus one manual drive with the cli adapter (rung: real binary, headless) | local Windows machine, temp folder — yes | test `json_dump_replaces_the_tick_file_extension` (step 4) | manual drive with `ls` of the folder → none needed |
| F2: a failed run emits a structured record (awaiting Q-2) | `cargo test -p engine-cli` subprocess drive with schema validation, plus manual drives of the three probe failure paths (real binary) | local machine — yes | steps 5 to 10; a directory as `--ticks-out` is the portable unwritable-path fixture | manual drive of each failure path with stdout captured to a file → none needed |
| F3: the OS error is named once | `cargo test -p engine-cli` subprocess drive (real binary) | local machine — yes | test `an_unwritable_tick_path_names_the_os_error_once` (step 4) | manual drive → none needed |
| F4: redirected stderr carries no ANSI escape | `cargo test -p engine-cli` subprocess drive; stderr is a pipe, which is the non-terminal case (real binary) | local machine — yes | test `redirected_stderr_carries_no_ansi_escape` (step 4) | `grep -c $'\x1b'` on a redirected stderr file → none needed |
| F5: no help line exceeds 80 columns | `cargo test -p engine-cli` over 18 help checks (real binary) | local machine — yes | widened test (step 3) | manual `awk 'length($0)>80'` over each surface → none needed |

No row depends on credentials, a device, an external service, or a deploy target. No `constraint-resolution:` line is required.

## Test / Verification Plan

### Automated checks

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`: blocking gates.
- `cargo test -p engine-cli`: the widened help test, the three regression tests from step 4, and (after Q-2) the failure-record tests and the negative schema check from step 10. `invalid_seed_exits_non_zero_and_names_the_argument`, `valid_seed_exits_zero`, every test in `schemas.rs`, and every test in `calibrate.rs` must still pass unchanged.
- `cargo test -p engine`: the classification test and the `FailureRecord` key-set test (after Q-2).
- `cargo test --workspace`: no other test may change.

### Interactive verification (human-in-the-loop)

Platform `cli` from `stack.platforms`. Driver: the release binary through Git Bash, as the probe drove it. Evidence goes to `verify-evidence/probe-engine-core/` as `<drive>.stdout.txt`, `<drive>.stderr.txt`, and `<drive>.exit-code`, the layout of the cli adapter.

1. `cargo build --release --workspace`.
2. F1: `target/release/engine-cli.exe simulate --seed 7 --minutes 1 --no-snapshot --json --ticks-out $T/match.ticks`, then `ls $T`. Pass: `match.jsonl` exists and `match.ticks.jsonl` does not.
3. F2 (after Q-2): run `simulate --seed 7 --minutes 0 --ticks-out $T/a.ticks`, `simulate --seed 7 --minutes 1 --no-snapshot --ticks-out $T/dir` (a directory), and `bench --seed 7 --matches 0`. Pass for option A: each exits 1, stdout holds exactly one JSON line with `outcome` `error` and the three `error.*` keys and no statistic key, and stderr holds the prose line.
4. F3: the directory drive from item 3. Pass: "os error" occurs once in stderr.
5. F4: `grep -c $'\x1b'` on the stderr file of item 2. Pass: 0.
6. F5: for each of the 9 help surfaces, `engine-cli <cmd> -h | awk 'length($0)>80'` and the same with `--help`. Pass: no output.

## Risks / Watchouts

- Q-2 is a contract decision (high). The answered record shape fails the shipped record schemas. Every way to reconcile them changes a published schema, the outcome word, or both. The plan does not settle it alone. Mitigation: the plan stops, and steps 5 to 10 wait.
- Two words for a failed outcome (medium). Calibration writes `failure` in per-match records and in its run report; the observability contract and the Q-1 answer say `error`. A dashboard that filters on one word misses the other. Mitigation: option C of Q-2 unifies on `error`; under option A or B, the review ledger carries the drift.
- Help-text regression (low). A later option with a long description breaks `-h` again. Mitigation: the widened test covers every surface under `-h` and `--help`.
- Stdout consumers (low, after Q-2). A script that treats non-empty stdout as success would misread a failure. The exit code stays 1, and the record says the outcome. Mitigation: the README states this.

## Dependencies on Other Slices

- `engine-core` and `data-schemas-generator`: implemented and verified. Their command-line step fixed findings 1, 3, and 4, and finding 5 under `--help`. This plan adds tests and the `-h` fix.
- `match-rules`: implemented and verified. It added the `resume`, `serve`, `record`, and `replay` help surfaces.
- `calibration`: implemented and verified. It added the `calibrate` help surface, the record schemas, the failed-match record with `outcome: "failure"`, and the classifier that step 5 moves. Step 5 must not change calibration's output. Option C of Q-2 would change it.
- No slice depends on this one.

## Assumptions

- A-1 (`class: implementation-detail`). Findings 1, 3, and 4 are treated as fixed, and the plan adds only regression tests. Reason: this run reproduced the fixed behaviour on a fresh release build at HEAD `3ec2198`.
- A-2 (`class: implementation-detail`). The `-h` overflow, including the new `calibrate` surface, counts as a residual of finding 5. Reason: it is the same `boundary-overflow` class on the same kind of help surface.
- A-3 (`class: implementation-detail`). Shorten the help strings and hide two possible-value lists, rather than enable clap's `wrap_help` feature. Reason: the feature adds the `terminal_size` dependency; `hide_possible_values` is built in (`clap_builder-4.6.7/src/builder/arg.rs:2570`).
- A-4 (`class: implementation-detail`). `resume --json` gets the same replace-rule help as `simulate --json`. Reason: both use `with_extension("jsonl")` (`simulate.rs:70`, `resume.rs:129`).
- A-5 (`class: implementation-detail`). The regression tests use the subprocess helpers in `cli_args.rs` and a directory as the unwritable path. Reason: a directory fails to open for writing on every system and needs no permission change.
- A-6 (`class: implementation-detail`). `error.type` reuses the values the calibration worker already writes (`invalid-config`, `io`, `format`, `sink`, `content`, `snapshot`), by moving its classifier into the engine. Reason: one vocabulary for one key, and calibration's persisted values do not change. This replaces the first plan's separate classifier.
- A-7 (`class: implementation-detail`). `error.code` is a finer code per variant, and `error.retriable` is `true` for a stopped tick consumer only. Reason: the contract defines the keys but not their values; no other code writes them yet.
- A-8 (`class: implementation-detail`). `calibrate` keeps prose-only failures. Reason: the Q-1 answer names `simulate` and `bench` only; widening it would widen a product answer.
- A-9 (`class: implementation-detail`). Steps 5 to 10 are written for option A of Q-2 but are not authorised to run until Q-2 is answered. Reason: planning the recommended option in full lets a re-run of this stage clear the blocker quickly.
- A-10 (`class: implementation-detail`). The master `04-plan.md` is not updated in this run. Reason: the plan is not complete, and other stopped plans in this workflow also stay out of the master until their question is answered.
- A-11 (`class: implementation-detail`). No second-opinion consult runs. The `unknowns-present` trigger holds (Q-2), but the product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`).
- A-12 (`class: implementation-detail`). No augmentation artifact is re-authored. Reason: this slice changes no simulation path; step 11 runs the benchmark once as a guard only.

## Blockers

- **Q-1 (answered, option A).** Recorded in `po-answers.md` at 2026-09-23T08:37:28Z: one stdout record with `outcome: "error"`, the three `error.*` keys, the command's success `record.kind`, no statistic key, nothing persisted, exit 1 with the prose line kept.
- **Q-2 (awaiting-input, `class: intent-bearing`).** The Q-1 record fails `schemas/observability/match-stats.schema.json` and `run-report.schema.json`: both accept `outcome` `success` or `failure` only, and both require every statistic key. These schemas shipped with calibration after Q-1 was asked. Which contract changes?
  - **Option A (recommended).** Keep the Q-1 answer as given. Amend both schemas additively: `outcome` accepts `error`; the statistic and benchmark keys are required only when `outcome` is not `error`; a record with `outcome` `error` requires `error.type`, `error.code`, and `error.retriable`. `schema.version` stays 1, and every record valid today stays valid. Calibration keeps `failure` for a match it started and could not play. Cost: two words for a failed outcome remain.
  - **Option B.** Use the word the schemas already accept: the failure record says `outcome: "failure"`, still with no statistic key. The schemas change only their required keys. This departs from the literal Q-1 answer and from the observability contract, which say `error`.
  - **Option C.** Unify on `error`: option A, plus calibration writes `error` in place of `failure` (the worker, the run report, the report filters, and their tests), and the schemas accept `success` and `error` only. This matches the observability contract. It changes calibration's output after verification, and run folders saved before the change fail the new schema unless `failure` stays accepted.
  - Why this stops the run: every option changes a published record schema, the outcome word, or both. A choice between them decides the persisted and printed data contract of charter C4. Per the decision rules, a contract change is not settled by an autonomous run.

## Freshness Research

- `clap` 4.6.7 (the locked version). The installed source is at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/clap_builder-4.6.7/`. `src/output/textwrap/mod.rs:26-28`: without `wrap_help`, help text is not wrapped. `src/builder/arg.rs:2570`: `Arg::hide_possible_values(bool)` exists, and `arg.rs:2606`: `hide_default_value(bool)`. Conclusion: help width is controlled by the text itself, by hidden suffixes, or by the `wrap_help` feature.
- `boon` (the JSON Schema validator in `crates/engine-cli/Cargo.toml`) already compiles the three record schemas in `tests/common/mod.rs`; the schemas use `allOf` with `if`/`then` (`run-report.schema.json`), so the conditional rule of step 9 uses a form the validator already evaluates.
- `tracing-subscriber`: `with_ansi(std::io::stderr().is_terminal())` is in place (`main.rs:30`).
- `anyhow`: `{err:#}` prints the chain once, joined by ": ", as this run's directory drive confirmed.
- No new dependency is proposed.

## Recommended Next Stage

- **Option A (after Q-2 is answered):** `/wf plan football-manager-match-engine probe-engine-core`. The re-run records the answer, clears the blocker, and adjusts steps 5 to 10 if the answer is B or C.
- **Option B:** `/wf implement football-manager-match-engine probe-engine-core`, limited to steps 1 to 4 and 11, if the product owner wants the help and regression fixes now and Q-2 later. Those steps do not depend on Q-2.
- **Option C:** `/wf review football-manager-match-engine calibration`. The review ledger can carry finding 2 and the outcome-word drift as items if the product owner prefers to batch them with the review fixes.
