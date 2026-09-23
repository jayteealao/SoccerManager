---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: probe-engine-core
status: complete
stage-number: 5
created-at: "2026-09-23T19:09:40Z"
updated-at: "2026-09-23T19:09:40Z"
metric-files-changed: 14
metric-lines-added: 1009
metric-lines-removed: 146
metric-deviations-from-plan: 4
metric-review-fixes-applied: 0
commit-sha: "733839c94e49578985cce8c1af5418fb2954c3f9"
commits:
  - "733839c94e49578985cce8c1af5418fb2954c3f9"
steering-honored:
  - "probe-engine-core Q-1 = A: a failed simulate or bench run prints one stdout record with outcome error, error.type, error.code, error.retriable, the command's success record.kind, and no statistic key; nothing is saved; exit 1 with the prose line kept (crates/engine-cli/src/main.rs:39-62, :72)."
  - "probe-engine-core Q-2 = C: calibration writes error, not failure (worker.rs:192, calibrate/mod.rs:153); both schemas accept success and error only; statistic keys are required only when the outcome is not error; an error record requires the three error keys (match-stats.schema.json:288-331, run-report.schema.json:116-141). Earlier run folders are not a compatibility target, so schema.version stays 1."
  - "Dark-path counter definition: untouched; the computer manager's queuing behaviour did not change."
  - "Design direction: not applicable; this slice changes no page."
  - "Output boundary: the commit message, code comments, help text, schema descriptions, and docs use product language; a search of every added line for workflow vocabulary found nothing."
tags: [probe, cli, observability, help-text, record-schemas, error-record, calibration]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-probe-engine-core.md
  plan: 04-plan-probe-engine-core.md
  benchmark: 05c-benchmark.md
  evidence: implement-evidence/probe-engine-core/
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-commentary.md, 05-implement-calibration.md, 05-implement-viewer-match-day.md, 05-implement-viewer-lineup-tactics.md, 05-implement-viewer-reports-recovery.md, 05-implement-integration.md, 05-implement-extra-time-penalties.md, 05-implement-experiment-flags.md, 05-implement-scripting-runtime.md, 05-implement-distribution.md]
  verify: 06-verify-probe-engine-core.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine probe-engine-core"
---

# Implement: Probe Findings on the Engine Command Line

## The Implementation

The plan came in complete at HEAD `89fe959`, with both product-owner answers recorded. Three of the five probe findings were already fixed in code but had no test. Finding 5 was still open under `-h`: 47 option lines on 10 help surfaces were wider than 80 columns. Finding 2 was open: a failed `simulate` or `bench` run printed prose and no record.

Now a failed `simulate` or `bench` run prints one record on stdout. It has the command's success `record.kind`, `outcome` `error`, `error.type`, `error.code`, and `error.retriable`, and no statistic key. The exit code is still 1, and the prose line still follows on stderr. The engine error type now classifies itself, and calibration uses the same classifier. Calibration writes `error` in place of `failure` in both its records. Both schemas accept `success` or `error` only, and they require the statistic keys only when a record is not an error. The `-h` text now fits 80 columns on all 20 help checks, with the removed detail moved into `--help`. A hidden `calibrate --inject-failure <match|worker>` seam drives both calibration error paths through the real binary. The workspace suite gave 414 passed and 0 failed, clippy and fmt are clean, and `bench --seed 42 --matches 5` gave 423 ms per match. Commit `733839c` holds 14 files.

Verify runs next, and every row can run on this machine. The top risk is that calibration's saved records changed shape after calibration was verified. A run folder written before this commit does not validate against the new schemas, which the product owner accepted. One more change was needed that the plan did not name: a failed calibration match used to write `rules.pack_version` 0 and an empty `manager.kind`. Those values break constraints that still apply when a key is present, so the failed-match record now writes the real rule-pack version and `["ai","ai"]` (Deviation 1).

## Summary of Changes

- Short help (`-h`) fits 80 columns on the top level and on `simulate`, `bench`, `generate`, `serve`, `launch`, `record`, `replay`, `resume`, and `calibrate`. Every removed detail is in `long_help`, so `--help` keeps it. `--suite` and `--keep-events` hide their possible values under both help forms, and their `long_help` names the values.
- `resume --json` long help now states the `.jsonl` replace rule, as `simulate --json` already did.
- `EngineError::error_type()`, `error_code()`, and `retriable()` exist in the engine. The private classifier in the calibration worker is removed.
- `FailureRecord` (engine observe module) implements `Record`. `MatchFigures` gains `error.code` and `error.retriable`, both optional.
- `main.rs` prints the failure record for `simulate` and `bench` on the error branch, before the prose line. `bench --matches 0` returns a typed `InvalidConfig`.
- Both record schemas: `outcome` is `success` or `error`; `error.code` and `error.retriable` are declared; one `allOf` branch with `if`/`then`/`else` requires the three error keys on an error record and the statistic or benchmark keys otherwise; the run report's `benchmark` and `calibrate` branches apply only when the outcome is not `error`.
- Calibration: a failed match writes `outcome` `error` with the three keys. A failed worker makes the run report say `outcome` `error`, `error.type` `worker`, `error.code` `worker-failed`, and `error.retriable` `false`.
- Hidden seam `calibrate --inject-failure <match|worker>`, forwarded to shard 0 only.
- Tests: the help test covers 20 checks; three regression tests cover findings 1, 3, and 4; four failure tests cover `simulate` (two paths), `bench`, and both calibration paths; one schema test holds four negative checks and one positive error-record check; the engine gains a classification test and two key-set tests.
- Docs: `docs/reference/cli.md` (`simulate`, `bench`, `calibrate`) and `README.md` (benchmark section) describe the error record.

## Files Changed

- `crates/engine-cli/src/cli.rs`: shorter short help on 47 lines with the detail moved into `long_help`; `hide_possible_values` on two options; `resume --json` long help; hidden `--inject-failure` with `InjectFailure` and `code()`.
- `crates/engine-cli/src/main.rs`: records the command, seed, and start time before dispatch; `emit_failure` prints the failure record for `simulate` and `bench`.
- `crates/engine-cli/src/bench.rs`: `--matches 0` returns `EngineError::InvalidConfig`.
- `crates/engine-cli/src/calibrate/worker.rs`: `error` in place of `failure`, the three error keys, the real rule-pack version and managers on a failed match, the injected failures, and the private classifier removed.
- `crates/engine-cli/src/calibrate/mod.rs`: the run report says `error` with the worker error keys when a worker failed; forwards `--inject-failure` to shard 0 and to the worker `Share`.
- `crates/engine-cli/src/report/mod.rs`: optional `error.type`, `error.code`, and `error.retriable` on `CalibrationReport`; the unit test sets `error`.
- `crates/engine/src/error.rs`: `error_type`, `error_code`, `retriable`, and a test over all 8 variants.
- `crates/engine/src/observe/mod.rs`: `FailureRecord`, two optional `MatchFigures` keys, and two key-set tests.
- `schemas/observability/match-stats.schema.json`: the option C contract (13 keys always required, 19 required only when not an error).
- `schemas/observability/run-report.schema.json`: the option C contract (13 keys always required, 3 benchmark keys required only when not an error; both operation branches skip an error record).
- `crates/engine-cli/tests/cli_args.rs`: the widened help test, three regression tests, four failure tests, and the extended hidden-seam test.
- `crates/engine-cli/tests/schemas.rs`: `the_schemas_accept_success_and_error_only_and_require_the_keys_of_each`.
- `docs/reference/cli.md`: the error record in the `simulate`, `bench`, and `calibrate` sections.
- `README.md`: one sentence in the benchmark section.

## Shared Files (also touched by sibling slices)

- `crates/engine-cli/src/cli.rs`: every command-line slice. Only help text and one hidden option changed; no visible flag was added or removed, and `docs.rs` still passes.
- `crates/engine/src/observe/mod.rs` and both record schemas: `calibration`, `experiment-flags`, `scripting-runtime`. Success records are unchanged.
- `crates/engine-cli/src/calibrate/*` and `report/mod.rs`: `calibration`, `experiment-flags` (`--pair`). A paired run with no failed worker writes the same record as before; `calibrate_pair.rs` passes unchanged.
- `docs/reference/cli.md` and `README.md`: `integration`, `distribution`.

## Notes on Design Choices

- The failure record is a separate `FailureRecord`, not a `MatchStats` with zero figures, because the Q-1 answer says no statistic key.
- The error is found by walking the `anyhow` chain with `downcast_ref::<EngineError>()`. The directory drive shows the chain holds the engine's `Io` error under the `cannot create` context, so `error.type` is `io`. An error with no `EngineError` in its chain is `internal`, `unclassified`, `false`.
- The owner identity comes from `load_or_create_owner_id`, the same one a successful run uses. If it cannot be read or created, or stdout fails, a warning is logged and only the prose line is printed.
- The schema condition is one `if`/`then`/`else` branch rather than two `if` branches: it is the same rule and easier to read.
- The short help texts follow one budget: 80 minus the option column of the surface (27 on most surfaces, 34 on `serve`, 35 on `calibrate`), including clap's `[default: …]` suffix. `--content-dir` is global, so its short text fits the narrowest budget (45).

## Verification Seams Built

- F1 (dump path) → test `json_dump_replaces_the_tick_file_extension` at `crates/engine-cli/tests/cli_args.rs:222` (enables `cargo test -p engine-cli` to observe `match.jsonl` with 3,000 lines and no `match.ticks.jsonl`).
- F2 (failed `simulate`/`bench` record) → `emit_failure` at `crates/engine-cli/src/main.rs:72` and tests `a_failed_simulate_prints_one_error_record` at `cli_args.rs:321` and `a_failed_bench_prints_one_error_record` at `cli_args.rs:356` (enable a subprocess drive with schema validation through `RecordSchemas`).
- F2-C (calibration writes `error`) → hidden `--inject-failure` at `crates/engine-cli/src/cli.rs:472`, applied at `crates/engine-cli/src/calibrate/worker.rs:44` (worker) and `:69` (match), forwarded at `crates/engine-cli/src/calibrate/mod.rs:465`; tests at `cli_args.rs:401` and `:451` (enable a real `calibrate` drive of both error paths).
- F3 (OS error once) → test `an_unwritable_tick_path_names_the_os_error_once` at `cli_args.rs:253`.
- F4 (no ANSI on redirected stderr) → test `redirected_stderr_carries_no_ansi_escape` at `cli_args.rs:274`.
- F5 (help width) → test `no_help_line_exceeds_eighty_columns` at `cli_args.rs:191`, 20 checks over `-h` and `--help` on 10 surfaces.
- Schema contract → test at `crates/engine-cli/tests/schemas.rs:82`; the hidden seam stays out of `-h` and `--help` per `the_test_seams_stay_out_of_the_help` at `cli_args.rs:468`.

## Visual Contract Honored

Not applicable. `02c-craft.md` exists for the viewer slices, but this slice changes no page and no UI surface.

## Deviations from Plan

1. **The failed calibration match writes the real rule-pack version and managers.** The plan (step 10) changed only the outcome word and the error keys. `rules.pack_version` has `minimum: 1` and `manager.kind` items have `enum: [human, ai]` in `match-stats.schema.json`. Those constraints apply whenever the key is present, and `MatchStats` always writes both. The old failed-match record wrote `0` and `["",""]`, so it could never pass the schema. The record now writes `content.rules.schema_version` and `["ai","ai"]` (`worker.rs:203`, `:209`). Calibration plays the AI manager on both sides, so these are the values the match would have used, not invented ones.
2. **The calibration-match test also validates the event files.** `calibrate_with` passes `--keep-events all`, and the test checks every event row against the event schema and asserts one event file (the failed match writes none). Without this, the shared test helper's `event` validator was unused in `cli_args.rs` and the build warned (`field event is never read`), which clippy `-D warnings` would refuse. This added coverage instead of adding a lint suppression.
3. **The failure tests use both helper sets in `cli_args.rs`.** That file has its own `bin()` and `temp()`, so the shared ones are called by path (`common::bin`, `common::temp`) and only `RecordSchemas` and `record` are imported.
4. **The `bench --json` long help is on two lines.** Moving the original sentence into `long_help` whole gave an 81-column `--help` line. The widened test found it, and the sentence now ends at "one JSON line." with "The default output is the same record." on a new paragraph.

No planned API was missing. `hide_possible_values` and the `if`/`then`/`else` keywords behaved as the plan's freshness research said: the help output and the `boon` checks in this run show it.

## Anything Deferred

- `abandoned` is not added to the `outcome` enum (plan A-15; the Q-2 answer names `success` and `error` only).
- `generate`, `serve`, `launch`, `record`, `replay`, `resume`, and a `calibrate` failure before workers start keep prose-only failures (plan A-8).
- No `sdlc-debt:` marker and no lint suppression was added.

## Known Risks / Caveats

- Run folders and `stats.json` files written before this commit hold `outcome` `failure` or a zero `rules.pack_version` on a failed match, and they do not validate against the new schemas. The product owner accepted this (earlier run folders are not a compatibility target).
- A script that treats any stdout line as success would misread a failed `simulate` or `bench`. The exit code stays 1, and the command reference and README say what a failed run prints.
- The failure record's keys are in the JSON map's sorted order, not with the envelope first. Success records have the same order today, because `serde_json` is built without `preserve_order`; no reader depends on key order.

## Freshness Research

- `clap` 4.6.7 (locked, unchanged since the plan): `hide_possible_values` removed the `[possible values: …]` suffix under `-h` and the `Possible values` list under `--help`, as this run's help output shows (`implement-evidence/probe-engine-core/f5-help-width.txt`).
- `boon` (JSON Schema 2020-12): `if`/`then`/`else` and `not`/`const` evaluate as expected. The negative checks in `schemas.rs` fail closed, and the error records of the real binary pass.
- `anyhow`: `Error::chain()` with `downcast_ref` found the `EngineError` under a `with_context` wrapper (the directory drive classified as `io`).
- No new dependency.

## Assumptions

- **D1** (class: implementation-detail, ac: "F1: the `--json` dump path is the one the help names", classification: build-capability): Findings 1, 3, and 4 were fixed before this run; this slice adds the regression tests that hold them, per plan A-1.
- **D2** (class: implementation-detail, ac: "F5: no help line exceeds 80 columns", classification: build-capability): The short help texts were chosen by the column budget, not from a fixed list. Every removed detail stays in `long_help`.
- **D3** (class: implementation-detail, ac: "F2: a failed simulate or bench run prints one structured record", classification: build-capability): The record shape follows the Q-1 answer word for word: the success `record.kind`, `outcome` `error`, the three `error.*` keys, no statistic key, nothing saved, exit 1, and the prose line kept.
- **D4** (class: implementation-detail): The `error.code` values and the `retriable` rule are the ones plan A-7 lists (`Sink` only is retriable).
- **D5** (class: implementation-detail): `owner.id` comes from `load_or_create_owner_id`, which may create the identity file on a new data folder (plan A-10). The file holds the owner identity, not a failure record, so "nothing persisted" still holds.
- **D6** (class: implementation-detail): `content.hash` is the empty string on a `simulate`/`bench` failure record (plan A-11).
- **D7** (class: implementation-detail, ac: "F2-C: calibration writes error, not failure, with the error keys", classification: build-capability): The worker-failure keys are `worker`, `worker-failed`, `false` (plan A-12). A run with one failed match and no failed worker keeps `outcome` `success`, because the failed match is recorded in its own statistics file and the run itself finished. The test asserts this.
- **D8** (class: implementation-detail): Deviation 1: the failed calibration match writes the real rule-pack version and `["ai","ai"]`. This makes the record valid under the schema the Q-2 answer asked for; it changes no product scope.
- **D9** (class: implementation-detail): Deviation 2: event-file validation was added to the calibration-match test instead of a dead-code suppression.
- **D10** (class: implementation-detail): One `if`/`then`/`else` branch expresses the plan's two `if` branches in each schema.
- **D11** (class: implementation-detail): `schema.version` stays 1 (plan A-14).
- **D12** (class: implementation-detail): The code commit uses the repository's product-language subject (`feat(engine): …`), not the stage's default subject, because of the output boundary. It contains only this slice's 14 paths, by pathspec (`git commit -- <paths>`). Four paths another session had already staged (`crates/engine/tests/zz_stall_probe.rs` and three `docs/design/realism/` files) stay staged and uncommitted.
- **D13** (class: implementation-detail): No second-opinion consult ran. No trigger in `_consult-triggers.md` holds: no reviews mode, no significant plan drift (four minor deviations, all inside the plan's files), no suppression written. The product owner also excluded `consult` (`00-index.md` `stack.excluded-by-po`).
- **D14** (class: implementation-detail): The benchmark guard compares with the latest recorded compare in `05c-benchmark.md` (432 ms at `77af778`) and with its baseline (421 ms). 423 ms is within 10 percent of both. No simulation path changed.

- **D15** (class: implementation-detail): In `00-index.md` this run added the two new files to `workflow-files` and moved `updated-at`. It left `current-stage`, `selected-slice`, and `next-command` as they are, because they point at the `distribution` slice, which another session is moving through its later stages. The next command for this slice is in this record and in `05-implement.md`.
- **D16** (class: implementation-detail): The `03-slice.md` roster entry for this slice is now `status: complete`, because this record is complete and the slice needs no further build pass. The `slices:` list in `00-index.md` does not hold probe slices (this one is under `compressed-slices`), so it was not changed.

No decision touched a `carried` intent risk: every RIM in `00-index.md` is `adjudicated`. No intent-bearing decision was made.

## Checks Run This Stage

- `cargo fmt --all -- --check`: clean, after `rustfmt` on the changed files.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace` (Windows, debug): 58 result lines, 414 passed, 0 failed, 4 ignored. `cli_args` 14 passed (was 7), `schemas` 4 passed (was 3), engine unit tests 135 passed.
- `cargo build --release --workspace`: finished.
- `target/release/engine-cli.exe bench --seed 42 --matches 5 --json`: exit 0, `bench.match_wall_ms` 423, 282,600 ticks per match, 1.46 µs per tick, 6.75 MB peak memory, `budget.pass` true (`implement-evidence/probe-engine-core/bench-42.*`).
- Release-binary drives with `SM_DATA_DIR` in a temp folder (`implement-evidence/probe-engine-core/`):
  - `simulate --seed 7 --minutes 0`: exit 1, one stdout record with `outcome` `error` and `error.type` `invalid-config`, then `error: invalid configuration: minutes must be 1 to 200, got 0`.
  - `simulate … --ticks-out <dir>`: exit 1, `error.type` `io`, and "os error" once on stderr.
  - `bench --seed 7 --matches 0`: exit 1, `record.kind` `run-report`, `error.type` `invalid-config`, `machine.hash` and `run.id` present.
  - `simulate … --json --ticks-out match.ticks`: `match.jsonl` and `match.ticks` written; 0 ESC bytes on the redirected stderr.
  - `calibrate … --inject-failure match`: exit 2, 0 files say `failure`, 1 statistics file says `outcome` `error`, and the report says `success`.
  - `calibrate … --inject-failure worker`: exit 2, the report says `outcome` `error`, `error.type` `worker`, `error.code` `worker-failed`, `error.retriable` false.
  - Help width: 0 lines over 80 columns on all 20 checks (`f5-help-width.txt`; widest 80).
- Leak check: no added line in the 14 committed files names a slice, stage, artifact path, or workflow command.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine probe-engine-core`. Every row of the Verification Strategy runs on this machine, with the tests and the hidden seam built here. Consider compacting the session before verify; the workflow state lives in artifact files on disk and is re-read after compaction.
- **Option B:** `/wf review football-manager-match-engine probe-engine-core`. Not recommended: the slice changes testable runtime behaviour (the failure record and calibration output).
- **Option C:** `/wf plan football-manager-match-engine probe-engine-core`. Only if the four recorded deviations are judged a plan error; none changes scope.
