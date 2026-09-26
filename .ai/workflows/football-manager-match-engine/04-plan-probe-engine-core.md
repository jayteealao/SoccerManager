---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: probe-engine-core
status: complete
stage-number: 4
created-at: "2026-09-22T22:29:40Z"
updated-at: "2026-09-23T18:52:56Z"
metric-files-to-touch: 14
metric-step-count: 13
has-blockers: false
revision-count: 3
revisions:
  - rev: 1
    at: "2026-09-23T08:44:00Z"
    trigger: manual
    because: "auto-review — 6 issues found"
    changed: "Q-1 recorded as answered (A). New intent-bearing Q-2: the answered record shape fails the record schemas that shipped with calibration (outcome enum success/failure; statistic keys required). Re-grounded on the current code: a ninth help surface (calibrate, 145 columns under -h), the calibration worker's error classifier reused instead of a new one, the Snapshot error variant, the shared test helpers and schema validator, and main.rs line numbers."
    snapshot: history/04-plan-probe-engine-core-0.md
  - rev: 2
    at: "2026-09-23T18:39:54Z"
    trigger: manual
    because: "auto-review — 6 issues found"
    changed: "Re-grounded on HEAD 89fe959 (22 commits since 3ec2198). Q-2 still unanswered and its facts re-confirmed (enum success/failure; 32 and 16 required keys). Step 1 became rule-based over 47 over-width -h lines on 10 surfaces (launch added; serve -h now 116 columns). Step 3 widens to 20 checks; the existing test now reads --help on 7 surfaces. Documentation target moved to docs/reference/cli.md (11 files). Line references updated in main.rs, worker.rs, calibrate/mod.rs, observe/mod.rs, cli.rs, simulate.rs, and resume.rs. Launch keeps prose-only failures (A-13)."
    snapshot: history/04-plan-probe-engine-core-1.md
  - rev: 3
    at: "2026-09-23T18:52:56Z"
    trigger: answers-returned
    because: "product owner answered Q-2 with option C (unify on error)"
    changed: "Blocker cleared; status complete. Schema step rewritten for option C (outcome success or error only). New step 10 moves calibration from failure to error in the per-match record and the run report, with the error keys. New hidden seam calibrate --inject-failure so the calibration change is driven for real. Negative schema checks added in tests/schemas.rs. 14 files, 13 steps. Re-confirmed on HEAD 89fe959 in this run."
    snapshot: history/04-plan-probe-engine-core-2.md
consult-runs: []
tags: [probe, cli, observability, help-text, record-schemas]
stack-source: confirmed
open-questions:
  - id: Q-1
    class: intent-bearing
    status: answered
    answer: "A — one stdout record with outcome error, error.type, error.code, error.retriable, the command's success record.kind, no statistic key; nothing persisted; exit 1 with the prose line kept (po-answers.md, 2026-09-23T08:37:28Z)."
  - id: Q-2
    class: intent-bearing
    status: answered
    answer: "C — unify on error: calibration writes error in place of failure; the schemas accept success and error only; statistic keys are required only when outcome is not error; an error record requires error.type, error.code, and error.retriable. Earlier run folders are not a compatibility target (po-answers.md, 2026-09-23T18:44:06Z; steer.md)."
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-probe-engine-core.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md, 04-plan-commentary.md, 04-plan-calibration.md, 04-plan-distribution.md, 04-plan-scripting-runtime.md, 04-plan-experiment-flags.md, 04-plan-extra-time-penalties.md]
  contract: ../../observability.md
  implement: 05-implement-probe-engine-core.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine probe-engine-core"
---

# Plan: Probe Findings on the Engine Command Line

## The Plan

The probe of the engine command line gave five findings. Three are already fixed in the code: the `--json` dump path, the repeated operating-system text, and the colour codes on redirected stderr. This run built the release binary at HEAD `89fe959` and drove them again. `--json` wrote `match.jsonl` beside `match.ticks`, an unwritable path named "os error" once, and redirected stderr held 0 escape bytes. Finding 5 is still open under `-h`: 47 option lines on 10 help surfaces exceed 80 columns (`serve -h` reaches 116, `calibrate -h` 145). Steps 1 to 4 fix the help text by a rule and add regression tests for all four findings.

Finding 2 is still open, and the product owner has now answered both of its questions. A failed `simulate` or `bench` run prints one record with `outcome: "error"` and the three `error.*` keys (Q-1 = A). The record schemas and calibration unify on the word `error` (Q-2 = C). So this plan is complete, with no blocker. Steps 5 to 8 print the failure record. Step 9 changes both schemas: `outcome` accepts `success` and `error` only, and the statistic keys are required only when `outcome` is not `error`. Step 10 changes calibration from `failure` to `error` and adds the error keys that an `error` record now requires. A hidden test seam, `calibrate --inject-failure`, lets verify drive a failed match and a failed worker through the real binary.

This work is 13 steps over 14 files. The top risk is that calibration's output changes after its verification. The product owner accepted this, because earlier run folders are not a compatibility target. The calibrate tests and one real calibration drive with an injected failure prove the new shape. Next is implementation of this slice.

## Current State

- Code: HEAD `89fe959`. `git diff --stat HEAD -- crates schemas docs/reference README.md` in this run shows no change. The release binary `target/release/engine-cli.exe` is current (`cargo build --release --workspace` finished in 0.14 s). This run drove it with `SM_DATA_DIR` set to a temp folder.
- Finding 1 (dump path): fixed. `simulate --seed 1 --minutes 1 --no-snapshot --json --ticks-out <tmp>/match.ticks` exited 0 and wrote `match.jsonl` and `match.ticks`. The `simulate --json` long help states the replace rule (`crates/engine-cli/src/cli.rs:66-73`; path derivation `simulate.rs:81`). `resume --json` uses the same rule (`resume.rs:142`), but its help says only "Also write a JSON Lines dump beside the tick file." (`cli.rs:110-111`). No test asserts the dump path.
- Finding 2 (no record on failure): open. In this run, `simulate --seed 7 --minutes 0 --ticks-out <tmp>/a.ticks` exited 1 with `error: invalid configuration: minutes must be 1 to 200, got 0` and 0 bytes on stdout. A directory as `--ticks-out` exited 1 with 0 bytes on stdout. `bench --seed 7 --matches 0` exited 1 with `error: --matches must be at least 1` and 0 bytes on stdout. The error branch in `crates/engine-cli/src/main.rs:45-52` prints prose only (`eprintln!` at line 49). `bench.rs:22-23` uses an untyped `anyhow::bail!`.
- Finding 3 (repeated OS text): fixed. `main.rs:49` prints `{err:#}`, and the directory drive printed "os error" once. No test asserts this.
- Finding 4 (ANSI on redirected stderr): fixed. `main.rs:31` sets `with_ansi(std::io::stderr().is_terminal())`, and the redirected stderr of the dump drive held 0 ESC bytes. No test asserts this.
- Finding 5 (help width): fixed under `--help` only (0 lines over 80 columns on all 10 surfaces in this run). Under `-h`, the widest line and the count of lines over 80 columns per surface: top level 94 (1), `simulate` 94 (5), `bench` 97 (6), `generate` 94 (3), `serve` 116 (8), `launch` 96 (6), `record` 94 (5), `replay` 102 (2), `resume` 94 (4), `calibrate` 145 (7). That is 47 lines. `no_help_line_exceeds_eighty_columns` (`crates/engine-cli/tests/cli_args.rs:184-200`) reads `--help` on 7 of the 10 surfaces and never reads `-h`.
- Record schemas (re-read in this run): `schemas/observability/match-stats.schema.json` has 32 required keys, from the envelope through `stats.*`, `rules.pack_version`, `fatigue.mean_pct`, `injury.count`, `darkpath.change_never_applied`, `change.expired_at_full_time`, and `manager.kind`. `outcome` is `enum: [success, failure]`. `error.type` is an optional string. `error.code` and `error.retriable` are not declared. `owner.id` has the pattern `^[0-9a-f]{32}$`, `match.id` `^[0-9a-f]{16}-[0-9]+$`, and `content.hash` is any string. `run-report.schema.json` has 16 required keys, including `run.id`, `machine.hash` (`^[0-9a-f]{12}$`), `bench.match_wall_ms`, `bench.cpu_us_per_tick`, and `bench.peak_mem_mb`. Its `outcome` is also `enum: [success, failure]`. Its `allOf` holds two `if`/`then` branches: `operation` `benchmark` requires `bench.matches`, `bench.ticks_per_match`, and `budget.pass`; `operation` `calibrate` requires 14 `calib.*`, dark-path, and event keys.
- Calibration writes the word `failure` in two places. The per-match record: `crates/engine-cli/src/calibrate/worker.rs:178` in `failure()` (`worker.rs:163-196`), with `content.hash` empty and `error.type` from the private classifier `error_type()` (`worker.rs:199-208`). The doc comment at `worker.rs:39` names `failure`. The run report: `calibrate/mod.rs:150-154` sets `failure` when any worker exited non-zero (`workers_failed`, counted at `calibrate/mod.rs:334-336`), and `CalibrationReport` (`report/mod.rs:366`) has no `error.*` field. The report filters compare against `"success"` (`report/mod.rs:146`, `:173`, `:219`), so they do not depend on the word. One test sets the word: `report/mod.rs:643` (`failed.outcome = "failure".into()`). No integration test and no document names `failure` as an outcome. `web/` does not read `outcome`.
- `MatchFigures` (`crates/engine/src/observe/mod.rs:143-167`) carries `error.type` as `Option<String>` with `skip_serializing_if`. `Record` (`observe/mod.rs:48`), `to_json` (`:432`), `emit_line` (`:451`), and `machine_hash()` (`:34`) exist. Identity: `load_or_create_owner_id` (`observe/identity.rs:34`) and `MatchId::now(seed)` (`identity.rs:98`).
- `EngineError` (`crates/engine/src/error.rs`, 48 lines) has 8 variants: `InvalidConfig`, `Io`, `Read`, `Format`, `Sink`, `Data`, `Snapshot`, `Version`.
- Hidden test seams already exist: `--match-millis` and `--drop-client-at` on `serve` and `launch` (`cli.rs:179-184`), and `--worker`, `--shard`, `--shards`, `--run-dir`, and `--run-millis` on `calibrate` (`cli.rs:359-369`). `the_test_seams_stay_out_of_the_help` (`cli_args.rs:203`) asserts that the first two stay hidden. `worker_command` (`calibrate/mod.rs:436-470`) builds the worker command line.
- Tests: `crates/engine-cli/tests/` holds 14 test files and `common/mod.rs` (`temp()`, `bin()` with a scratch `SM_DATA_DIR`, `record()`, and `RecordSchemas` with `stats` at line 70 and `report` at line 74). `docs.rs` checks that `docs/reference/cli.md` names every flag that `--help` shows, so a hidden flag needs no entry. In this run, `cargo test -p engine-cli --test cli_args --test schemas` gave 7 passed and 3 passed, 0 failed.
- Documentation: `docs/reference/cli.md` has one section per command (`simulate` at line 40, `bench` 60, `calibrate` 182) and an exit-code table at line 32. `README.md` gives the benchmark exit codes at line 103.

## Simplicity Ladder

- Help width under `-h` → rung 4 new-code (text only). Rung 2: clap wraps help only with the `wrap_help` feature (`clap_builder-4.6.7/src/output/textwrap/mod.rs:26-28`), which adds the `terminal_size` dependency. It is rejected. `Arg::hide_possible_values` is built in (`clap_builder-4.6.7/src/builder/arg.rs:2570`) and removes the `[possible values: …]` suffix. It is used for `--suite` and `--keep-events`, and `long_help` names the values.
- Dump-path, single OS-error, and no-ANSI tests → rung 3 reuse: `bin()`, `content_dir()`, and `temp()` in `crates/engine-cli/tests/cli_args.rs:9-23`. `std::process::Command` captures stderr as a pipe, which is the non-terminal case.
- Error classification → rung 3 reuse with modification: move `error_type()` from `crates/engine-cli/src/calibrate/worker.rs:199-208` into `EngineError` in `crates/engine/src/error.rs`, with the same values.
- Failure record → rung 3 reuse with modification: `Record`, `to_json`, and `emit_line` (`crates/engine/src/observe/mod.rs:48`, `:432`, `:451`) already write the envelope keys. A new `FailureRecord` implements `Record`. Reusing `MatchStats` with zero statistics, as the calibration worker does, is rejected for `simulate` and `bench`, because the Q-1 answer says no statistic key.
- Calibration outcome word → rung 3 reuse: the existing `failure()` path and `CalibrationReport` change their word and gain the error keys; `MatchFigures` gains two optional fields beside `error_type`.
- Failure injection for verify → rung 3 reuse of the hidden-flag pattern (`#[arg(long, hide = true)]`, `cli.rs:179-184`, `:359-369`) and of `worker_command` to forward it.
- Schema checks → rung 3 reuse: `RecordSchemas`, `bin()`, `temp()`, and `record()` in `crates/engine-cli/tests/common/mod.rs`; `boon` evaluates `allOf`/`if`/`then` already (`run-report.schema.json`).

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist, and no `solutions.globalDir` is configured (`.ai/sdlc-config.json` is absent).

Repeat-deferral tripwire: `runtime-evidence-deferrals` in `00-index.md` holds two entries (viewer-match-day legibility, distribution macOS). Neither names a wall that this slice's Verification Strategy names; every row here runs on the local machine. The tripwire does not fire.

## Likely Files / Areas to Touch

- `crates/engine-cli/src/cli.rs`: shorten the 47 over-width short-help strings; hide the possible values of two options; add the replace rule to `resume --json`; add the hidden `calibrate --inject-failure`.
- `crates/engine-cli/tests/cli_args.rs`: widen the 80-column test to 20 checks; regression tests for findings 1, 3, and 4; failure-record tests; calibration drives with the injected failure; extend the hidden-seam test.
- `crates/engine-cli/tests/schemas.rs`: negative schema checks.
- `crates/engine/src/error.rs`: `error_type()`, `error_code()`, and `retriable()` on `EngineError`.
- `crates/engine/src/observe/mod.rs`: `FailureRecord`; `error.code` and `error.retriable` on `MatchFigures`.
- `crates/engine-cli/src/main.rs`: emit the failure record on the error branch of `simulate` and `bench`.
- `crates/engine-cli/src/bench.rs`: a typed error for `--matches 0`.
- `crates/engine-cli/src/calibrate/worker.rs`: `error` in place of `failure`, the error keys, the moved classifier, the injected match failure.
- `crates/engine-cli/src/calibrate/mod.rs`: `error` in place of `failure` on the run report with the error keys; forward the injected failure.
- `crates/engine-cli/src/report/mod.rs`: optional error keys on `CalibrationReport`; the test word.
- `schemas/observability/match-stats.schema.json` and `run-report.schema.json`: the option C contract change.
- `docs/reference/cli.md`: the failure record in the `simulate`, `bench`, and `calibrate` sections.
- `README.md`: one sentence beside the benchmark exit codes.

## Proposed Change Strategy

Work in three groups.

Group one (steps 1 to 4) completes findings 1, 3, 4, and 5 with help-text edits and regression tests. It changes no runtime behaviour except the help text.

Group two (steps 5 to 8) implements the Q-1 answer. The error branch of `simulate` and `bench` prints one record on stdout before the existing prose line. The exit code stays 1. The record carries the envelope keys, `operation`, the correlation keys the schema requires (`match.id` for `simulate`; `run.id` and `machine.hash` for `bench`), `seed`, `content.hash`, `duration_ms`, `outcome: "error"`, `error.type`, `error.code`, and `error.retriable`, and no statistic key. `simulate` fails as `record.kind: match-stats`, and `bench` fails as `record.kind: run-report`. No record is written under the data folder. Argument-parse failures (exit 2) happen before a run starts, so they stay prose-only.

Group three (steps 9 and 10) implements the Q-2 answer, option C. Both schemas accept `outcome` `success` or `error` only. The statistic and benchmark keys are required only when `outcome` is not `error`. An `error` record requires the three `error.*` keys. Calibration writes `error` in place of `failure` in the per-match record and in the run report, and adds the three keys to both. The envelope and correlation keys stay required on every record, because the answer changes no other key. Steps 11 to 13 test, document, and gate the result.

The NFR rationale is charter C4 ("emits structured events and statistics for the observability pipeline"), which the product owner ratified. No unranked NFR narrows a charter commitment in this plan.

## Step-by-Step Plan

1. **Short help within 80 columns (finding 5).** In `crates/engine-cli/src/cli.rs`, shorten the doc comment of every option whose `-h` line exceeds 80 columns. Apply one rule, not a fixed list: the description budget of an option is 80 minus the option column of its subcommand, including clap's `[default: …]` suffix. At HEAD `89fe959` this covers 47 lines on 10 surfaces (see Current State). Move each removed detail (default paths, environment variables, value lists) into `long_help`, so `--help` keeps it. Add `hide_possible_values = true` to `calibrate --suite` and `--keep-events`, and name the values in each option's `long_help`. `--content-dir` is a global option, so one edit fixes it on every surface. Keep every `--help` line at 80 columns or fewer. Do not add or remove any visible flag; `docs.rs` checks the flag names against `docs/reference/cli.md`.
2. **Resume dump-path help (finding 1).** Give `ResumeOpts::json` (`cli.rs:110-111`) the same `long_help` as `SimulateOpts::json` (`cli.rs:66-73`), which states that the extension is replaced by `.jsonl`.
3. **Widen the help test (finding 5).** In `crates/engine-cli/tests/cli_args.rs:184-200`, change `no_help_line_exceeds_eighty_columns` to read both `-h` and `--help` for the top level and for `simulate`, `bench`, `generate`, `serve`, `launch`, `record`, `replay`, `resume`, and `calibrate`. That is 20 checks.
4. **Regression tests for findings 1, 3, and 4.** In `cli_args.rs`, add:
   - `json_dump_replaces_the_tick_file_extension`: `simulate --seed 7 --minutes 1 --no-snapshot --json --ticks-out <tmp>/match.ticks`. Assert exit 0, `<tmp>/match.jsonl` exists with 3,000 lines, and `<tmp>/match.ticks.jsonl` does not exist.
   - `an_unwritable_tick_path_names_the_os_error_once`: create `<tmp>/dir`, run `simulate --seed 7 --minutes 1 --no-snapshot --ticks-out <tmp>/dir`. Assert exit 1, stderr names the path, and "os error" occurs exactly once.
   - `redirected_stderr_carries_no_ansi_escape`: run `simulate --seed 7 --minutes 1 --no-snapshot` with stderr captured. Assert the stderr bytes contain no `0x1B`.
5. **Error classification in the engine.** Move `error_type()` from `crates/engine-cli/src/calibrate/worker.rs:199-208` into `crates/engine/src/error.rs` as `EngineError::error_type(&self) -> &'static str`, with the same values (`invalid-config`, `io`, `format`, `sink`, `content`, `snapshot`). Add `error_code()`: `InvalidConfig` → `invalid-config`, `Io` → `io`, `Read` → `read`, `Format` → `tick-format`, `Sink` → `sink-stopped`, `Data` → `content-refused`, `Version` → `content-version`, `Snapshot` → `snapshot-refused`. Add `retriable()`: `true` for `Sink` only. Unit-test every variant.
6. **Failure record.** In `crates/engine/src/observe/mod.rs`, add `FailureRecord` with the fields `kind`, `operation`, `owner_id`, `seed`, `match_id: Option<String>`, `run_id: Option<String>`, `machine_hash: Option<String>`, `content_hash: String`, `duration_ms`, `error_type`, `error_code`, and `error_retriable`. Implement `Record`, so that `kind()` and `operation()` return the stored values. Serialize `outcome` as `"error"`, the keys `error.type`, `error.code`, and `error.retriable`, and each optional correlation key only when set. Add `error_code: Option<String>` (`error.code`) and `error_retriable: Option<bool>` (`error.retriable`) to `MatchFigures` beside `error_type`, both with `skip_serializing_if = "Option::is_none"`, and set both to `None` in `MatchFigures::new`. Add unit tests that assert the key set of a `simulate` failure and of a `bench` failure: the envelope keys, `operation`, the correlation keys, `seed`, `content.hash`, `duration_ms`, `outcome`, and the three `error.*` keys, with no `ticks.written`, `stats.*`, `validate.*`, or `bench.*` key. Assert also that a success `MatchStats` serializes no `error.*` key.
7. **Typed bench check.** In `crates/engine-cli/src/bench.rs:22-23`, return `EngineError::InvalidConfig("--matches must be at least 1".into())` in place of `anyhow::bail!`. The stderr text becomes `error: invalid configuration: --matches must be at least 1`.
8. **Emit on the error branch.** In `main.rs` (dispatch at lines 34-44, error branch at 45-52), take `Instant::now()` and the command's seed before dispatch. On `Err` from `Simulate`, build a `FailureRecord` with kind `match-stats`, operation `simulate`, and `match_id` from `MatchId::now(seed)`. On `Err` from `Bench`, use kind `run-report`, operation `benchmark`, `run_id` `bench-<unix millis>` (the form `bench.rs:49` uses), and `machine_hash()`. Set `content.hash` to the empty string, as the calibration worker does for a failed match. Take `owner.id` from `load_or_create_owner_id(&data_dir())`, the identity every successful run uses. Classify through the `anyhow` chain with `downcast_ref::<EngineError>()`; when no `EngineError` is found, use `internal`, `unclassified`, `false`. Print the record through `emit_line`, then print the existing prose line on stderr and exit 1. If the identity cannot be read or created, or `emit_line` fails, print no record, log one warning, and still print the prose line and exit 1. `generate`, `serve`, `launch`, `record`, `replay`, `resume`, and `calibrate` keep prose only.
9. **Schema change, option C.** In `schemas/observability/match-stats.schema.json`: set `outcome` to `enum: [success, error]`; declare `error.code` (string, `minLength` 1) and `error.retriable` (boolean); keep the envelope keys, `operation`, `match.id`, `seed`, `content.hash`, `outcome`, and `duration_ms` in the top-level `required` list; move every other required key (`engine.ticks_per_s` through `manager.kind`, 19 keys) into an `allOf` branch `if outcome is not error then required [...]`; add a branch `if outcome is error then required [error.type, error.code, error.retriable]`. In `run-report.schema.json`: the same `outcome` enum and `error.*` declarations; keep the envelope keys, `operation`, `run.id`, `seed`, `content.hash`, `outcome`, and `machine.hash` required; move `bench.match_wall_ms`, `bench.cpu_us_per_tick`, and `bench.peak_mem_mb` into an `if outcome is not error` branch; add `outcome: {not: {const: error}}` to the `if` of the existing `benchmark` and `calibrate` branches so they apply only to a record that is not an error; add the `if outcome is error then required` branch for the three `error.*` keys. Keep `schema.version` at 1: the product has not shipped, and earlier run folders are not a compatibility target (product owner, Q-2).
10. **Calibration writes `error`.** In `crates/engine-cli/src/calibrate/worker.rs`: `failure()` sets `outcome` to `"error"` and fills `error_type`, `error_code`, and `error_retriable` in `MatchFigures` from `err.error_type()`, `err.error_code()`, and `err.retriable()`; delete the private `error_type()`; change the doc comment at line 39 to name `error`. In `crates/engine-cli/src/report/mod.rs`: add `error_type: Option<&'static str>`, `error_code: Option<&'static str>`, and `error_retriable: Option<bool>` to `CalibrationReport` as `error.type`, `error.code`, and `error.retriable`, skipped when `None`; change the test at line 643 to set `"error"`. In `crates/engine-cli/src/calibrate/mod.rs:150-154`: when a worker failed, set `outcome` `"error"`, `error.type` `worker`, `error.code` `worker-failed`, and `error.retriable` `false`; otherwise `"success"` and no error key. Add the hidden seam `--inject-failure <match|worker>` to `CalibrateOpts` (`#[arg(long, hide = true)]`). `worker_command` (`calibrate/mod.rs:436-470`) forwards it to shard 0 only. In a worker, `match` makes fixture 0 fail through `failure()` with `EngineError::InvalidConfig("injected failure".into())`, and the worker continues; `worker` makes the worker exit 1 before its first fixture. Without the flag, nothing changes. Add `--inject-failure` to the hidden list in `the_test_seams_stay_out_of_the_help` for `calibrate`.
11. **Failure tests.** Using `crates/engine-cli/tests/common`, add to `cli_args.rs`:
    - `simulate --seed 7 --minutes 0`: exit 1; exactly one stdout line with `record.kind` `match-stats`, `outcome` `error`, `error.type` `invalid-config`, and no `stats.goals`; the prose line on stderr; `RecordSchemas::stats` accepts the record.
    - A directory as `--ticks-out`: `error.type` `io`; the schema accepts it.
    - `bench --seed 7 --matches 0`: `record.kind` `run-report`, `outcome` `error`, `error.type` `invalid-config`, no `bench.match_wall_ms`; `RecordSchemas::report` accepts it.
    - `calibrate --seed 1 --matches 2 --minutes 1 --jobs 1 --suite equal --inject-failure match --out <tmp>`: one stats file has `outcome` `error` and the three `error.*` keys, every stats file passes `RecordSchemas::stats`, and no stats file says `failure`.
    - The same with `--inject-failure worker`: the run report has `outcome` `error`, `error.type` `worker`, and passes `RecordSchemas::report`.
    In `crates/engine-cli/tests/schemas.rs`, add negative checks: a `match-stats` record with `outcome` `failure` is refused; a `match-stats` record with `outcome` `success` and no `stats.goals` is refused; an `error` record without `error.code` is refused; a `run-report` with `outcome` `failure` is refused. Keep `valid_seed_exits_zero`, the three existing tests in `schemas.rs`, `calibrate.rs`, and `calibrate_pair.rs` unchanged as success-path guards.
12. **Documentation.** In `docs/reference/cli.md`, state in the `simulate` and `bench` sections that a failed run prints one record with `outcome` `error` and the keys `error.type`, `error.code`, and `error.retriable` on standard output, and exits 1. In the `calibrate` section, state that a failed match writes its statistics record with `outcome` `error`, and that a failed worker makes the run report say `outcome` `error`. Add one sentence to the same effect as the `simulate`/`bench` text beside the benchmark exit codes in `README.md` (line 103). Name no hidden flag.
13. **Gates and record.** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p engine-cli`, `cargo test -p engine`, `cargo test --workspace`, and `cargo build --release --workspace`. Run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` once, and confirm that `bench.match_wall_ms` is within 10 percent of the latest `05c-benchmark.md` baseline. No simulation path changes, so this is a guard only. Search the diff for workflow vocabulary (slice names, stage names, artifact paths) before the commit. Record the numbers and the resolution of each finding in the implement artifact.

## Verification Strategy

Every row is a user-observable command-line behaviour from the probe findings or from the Q-2 answer. The `cli` adapter drives the real release binary as a subprocess, and the stack confirms `cargo-test` (`00-index.md` `stack.testing`).

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| F1: the `--json` dump path is the one the help names | `cargo test -p engine-cli` subprocess drive plus one manual drive with the cli adapter (rung: real binary, headless) | local Windows machine, temp folder — yes | test `json_dump_replaces_the_tick_file_extension` (step 4) | manual drive with `ls` of the folder → none needed |
| F2: a failed `simulate` or `bench` run prints one structured record | `cargo test -p engine-cli` subprocess drive with schema validation, plus manual drives of the three probe failure paths (real binary) | local machine — yes | steps 5 to 9 and 11; a directory as `--ticks-out` is the portable unwritable-path fixture | manual drive of each failure path with stdout captured to a file → none needed |
| F2-C: calibration writes `error`, not `failure`, with the error keys | `cargo test -p engine-cli` subprocess drive of `calibrate` with the injected failure, validated against both schemas, plus one manual drive (real binary) | local machine — yes | hidden seam `calibrate --inject-failure <match\|worker>` (step 10) | manual drive with `grep -l failure` over the run folder → none needed |
| F3: the OS error is named once | `cargo test -p engine-cli` subprocess drive (real binary) | local machine — yes | test `an_unwritable_tick_path_names_the_os_error_once` (step 4) | manual drive → none needed |
| F4: redirected stderr carries no ANSI escape | `cargo test -p engine-cli` subprocess drive; stderr is a pipe, which is the non-terminal case (real binary) | local machine — yes | test `redirected_stderr_carries_no_ansi_escape` (step 4) | `grep -c $'\x1b'` on a redirected stderr file → none needed |
| F5: no help line exceeds 80 columns | `cargo test -p engine-cli` over 20 help checks (real binary) | local machine — yes | widened test (step 3) | manual `awk 'length($0)>80'` over each surface → none needed |

No row depends on credentials, a device, an external service, or a deploy target. The only seam a row needs, the injected calibration failure, is code in this repository and is a plan step. No `constraint-resolution:` line is required.

## Test / Verification Plan

### Automated checks

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`: blocking gates.
- `cargo test -p engine-cli`: the widened help test, the three regression tests from step 4, the five failure tests from step 11, the four negative schema checks, and the extended hidden-seam test. `invalid_seed_exits_non_zero_and_names_the_argument`, `valid_seed_exits_zero`, every test in `schemas.rs`, `docs.rs`, `calibrate.rs`, and `calibrate_pair.rs` must still pass unchanged.
- `cargo test -p engine`: the classification test and the key-set tests of step 6.
- `cargo test --workspace`: no other test may change.

### Interactive verification (human-in-the-loop)

Platform `cli` from `stack.platforms`. Driver: the release binary through Git Bash, as the probe drove it, with `SM_DATA_DIR` set to a temp folder. Evidence goes to `verify-evidence/probe-engine-core/` as `<drive>.stdout.txt`, `<drive>.stderr.txt`, and `<drive>.exit-code`, the layout of the cli adapter.

1. `cargo build --release --workspace`.
2. F1: `target/release/engine-cli.exe simulate --seed 7 --minutes 1 --no-snapshot --json --ticks-out $T/match.ticks`, then `ls $T`. Pass: `match.jsonl` exists and `match.ticks.jsonl` does not.
3. F2: run `simulate --seed 7 --minutes 0 --ticks-out $T/a.ticks`, `simulate --seed 7 --minutes 1 --no-snapshot --ticks-out $T/dir` (a directory), and `bench --seed 7 --matches 0`. Pass: each exits 1; stdout holds exactly one JSON line with `outcome` `error`, the three `error.*` keys, and no statistic key; stderr holds the prose line.
4. F2-C: `calibrate --seed 1 --matches 2 --minutes 1 --jobs 1 --suite equal --inject-failure match --out $T/cal-m`, then `grep -rl '"failure"' $T/cal-m` and `grep -rl '"outcome":"error"' $T/cal-m/stats`. Pass: no file says `failure`, and one stats file says `error`. Then the same with `--inject-failure worker --out $T/cal-w`. Pass: the run report says `outcome` `error` with `error.type` `worker`.
5. F3: the directory drive from item 3. Pass: "os error" occurs once in stderr.
6. F4: `grep -c $'\x1b'` on the stderr file of item 2. Pass: 0.
7. F5: for each of the 10 help surfaces, `engine-cli <cmd> -h | awk 'length($0)>80'` and the same with `--help`. Pass: no output.

## Risks / Watchouts

- Calibration output changes after its verification (medium). The per-match record and the run report change their word and gain two keys. Run folders saved before the change fail the new schema. Mitigation: the product owner accepted this (earlier run folders are not a compatibility target); the calibrate tests and the drives with the injected failure prove the new shape.
- An `error` run report without the error keys (medium). After step 9, an `error` record without `error.type`, `error.code`, and `error.retriable` is refused. Mitigation: step 10 adds the keys to both calibration records, and the injected worker failure drives the run-report path against the real schema.
- The injected failure leaks into a real run (low). Mitigation: the flag is hidden, applies to shard 0 only, is off by default, and the hidden-seam test asserts it stays out of the help.
- Help text keeps growing (low). Later slices took the over-width count from 9 options to 47 lines. Mitigation: step 1 applies a rule, and the widened test covers every surface under `-h` and `--help`.
- Stdout consumers of a failed run (low). A script that treats non-empty stdout as success would misread a failure. Mitigation: the exit code stays 1, and the command reference and the README state what a failed run prints.

## Dependencies on Other Slices

- `engine-core` and `data-schemas-generator`: implemented and verified. Their command-line step fixed findings 1, 3, and 4, and finding 5 under `--help`. This plan adds tests and the `-h` fix.
- `match-rules`: implemented and verified. It added the `resume`, `serve`, `record`, and `replay` help surfaces.
- `calibration`: implemented and verified. It added the `calibrate` help surface, the record schemas, the failed-match record, and the classifier that step 5 moves. Step 10 changes calibration's outcome word and adds the error keys, as the product owner decided.
- `experiment-flags`: `calibrate --pair` uses the same run report. Step 10 changes only the outcome selection and the optional error keys, so a paired run that has no failed worker writes the same record as before.
- `distribution`, `extra-time-penalties`, `scripting-runtime`, and the viewer slices: they added the `launch` surface and the `--knockout`, `--script-pack`, `--reconnect-wait`, `--flag`, and `--pair` options that step 1 shortens. Step 1 changes help text only.
- No slice depends on this one.

## Assumptions

- A-1 (`class: implementation-detail`). Findings 1, 3, and 4 are treated as fixed, and the plan adds only regression tests. Reason: this run reproduced the fixed behaviour on the release binary at HEAD `89fe959`.
- A-2 (`class: implementation-detail`). The `-h` overflow on every surface, including `launch` and `calibrate`, counts as a residual of finding 5. Reason: it is the same `boundary-overflow` class on the same kind of help surface.
- A-3 (`class: implementation-detail`). Shorten the help strings and hide two possible-value lists, rather than enable clap's `wrap_help` feature. Reason: the feature adds the `terminal_size` dependency; `hide_possible_values` is built in (`clap_builder-4.6.7/src/builder/arg.rs:2570`).
- A-4 (`class: implementation-detail`). `resume --json` gets the same replace-rule help as `simulate --json`. Reason: both use `with_extension("jsonl")` (`simulate.rs:81`, `resume.rs:142`).
- A-5 (`class: implementation-detail`). The regression tests use the subprocess helpers in `cli_args.rs` and a directory as the unwritable path. Reason: a directory fails to open for writing on every system and needs no permission change.
- A-6 (`class: implementation-detail`). `error.type` reuses the values the calibration worker already writes (`invalid-config`, `io`, `format`, `sink`, `content`, `snapshot`), by moving its classifier into the engine. Reason: one vocabulary for one key.
- A-7 (`class: implementation-detail`). `error.code` is a finer code per variant, and `error.retriable` is `true` for a stopped tick consumer only. Reason: the contract defines the keys but not their values; no other code writes them yet.
- A-8 (`class: implementation-detail`). `calibrate` does not print a failure record on stdout when the whole command fails before workers start; it keeps prose-only failures like the other subcommands outside `simulate` and `bench`. Reason: the Q-1 answer names `simulate` and `bench` only. The Q-2 answer changes only the word and keys that calibration already writes.
- A-9 (`class: implementation-detail`). The failure record keeps every envelope and correlation key the schemas require (`owner.id`, `match.id`, `run.id`, `machine.hash`, `content.hash`), and the schemas keep them required. Only the statistic and benchmark keys become conditional. Reason: the Q-2 answer names those keys and "does NOT change any other record key"; keeping the rest required is the smallest contract change.
- A-10 (`class: implementation-detail`). The failure record takes `owner.id` from `load_or_create_owner_id`, which may create the identity file on a fresh data folder, as any successful run does. When the identity cannot be read or created, no record is printed and the prose line and exit 1 remain. Reason: `owner.id` is required with a 32-hex pattern, and an invented value would be a fabricated value. The identity file is not a failure record, so the Q-1 rule "nothing persisted" still holds.
- A-11 (`class: implementation-detail`). `content.hash` on a `simulate` or `bench` failure record is the empty string. Reason: the calibration worker already writes an empty `content.hash` on a failed match (`worker.rs:175`), and the schema allows any string; threading the loaded content out of the failed command would widen the change.
- A-12 (`class: implementation-detail`). A calibration run report with a failed worker carries `error.type` `worker`, `error.code` `worker-failed`, and `error.retriable` `false`. Reason: the answer requires the three keys on an `error` record but gives no values; a worker failure is not an `EngineError` in the parent, and a deterministic engine repeats the same failure on the same seeds.
- A-13 (`class: implementation-detail`). A hidden test seam, `calibrate --inject-failure <match|worker>`, is added so verify can drive both calibration error paths through the real binary. Reason: no public input makes a calibration match or worker fail; hidden seams on `calibrate`, `serve`, and `launch` are the existing pattern (`cli.rs:179-184`, `:359-369`).
- A-14 (`class: implementation-detail`). `schema.version` stays 1 after the schema change. Reason: the product owner stated that earlier run folders are not a compatibility target, and the product has not shipped.
- A-15 (`class: implementation-detail`). `abandoned`, which the instrumentation contract lists, is not added to the schemas. Reason: the Q-2 answer says the schemas accept `success` and `error` only.
- A-16 (`class: implementation-detail`). The master `04-plan.md` is updated in this run: this slice joins the summaries, and the stale "finding 2 waits for `/wf observability init`" line is corrected. Reason: the plan is now complete.
- A-17 (`class: implementation-detail`). No second-opinion consult runs. The `appetite-medium-or-larger` trigger holds (`00-index.md` `appetite: large`), but the product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`).
- A-18 (`class: implementation-detail`). No augmentation artifact is re-authored. Reason: this slice changes no simulation path; step 13 runs the benchmark once as a guard only.
- A-19 (`class: implementation-detail`). The discovery interview did not run on this autonomous re-run. Every implementation question it would ask is settled by A-1 to A-18. Reason: the autonomous policy settles implementation detail and records it; both intent-bearing questions (Q-1, Q-2) have product-owner answers.

## Blockers

None. Both intent-bearing questions have product-owner answers:

- **Q-1 (answered, option A).** Recorded in `po-answers.md` at 2026-09-23T08:37:28Z: one stdout record with `outcome: "error"`, the three `error.*` keys, the command's success `record.kind`, no statistic key, nothing persisted, exit 1 with the prose line kept.
- **Q-2 (answered, option C).** Recorded in `po-answers.md` at 2026-09-23T18:44:06Z and in `steer.md`: unify on `error`. Calibration writes `error`, not `failure`. The schemas accept `success` and `error` only. Statistic keys are required only when the outcome is not `error`, and an `error` record requires `error.type`, `error.code`, and `error.retriable`. Earlier run folders are not a compatibility target.

## Freshness Research

- `clap` 4.6.7 (the locked version). The installed source is at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/clap_builder-4.6.7/`. `src/output/textwrap/mod.rs:26-28`: without `wrap_help`, help text is not wrapped. `src/builder/arg.rs:2570`: `Arg::hide_possible_values(bool)` exists. Conclusion: help width is controlled by the text itself, by hidden suffixes, or by the `wrap_help` feature. (Carried from revision 2; the lock file did not change, because HEAD is the same.)
- `boon` (the JSON Schema validator in `crates/engine-cli/Cargo.toml`) compiles the record schemas in `tests/common/mod.rs`, and `run-report.schema.json` already uses `allOf` with `if`/`then`, so step 9 uses a form the validator already evaluates. `not`/`const` are core JSON Schema 2020-12 keywords, and the schemas declare draft 2020-12.
- `tracing-subscriber`: `with_ansi(std::io::stderr().is_terminal())` is in place (`main.rs:31`).
- `anyhow`: `{err:#}` prints the chain once, joined by ": ", as this run's directory drive confirmed.
- No new dependency is proposed.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine probe-engine-core`. The plan is complete, with no blocker. Compact the session first; the artifacts are re-read after compaction.
- **Option C:** `/wf slice football-manager-match-engine`, only if the calibration change (step 10) should be its own slice. The plan keeps it here, because the product owner tied it to the same answer.
