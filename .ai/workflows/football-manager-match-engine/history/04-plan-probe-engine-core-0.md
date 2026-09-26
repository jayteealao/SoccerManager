---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: probe-engine-core
status: awaiting-input
stage-number: 4
created-at: "2026-09-22T22:29:40Z"
updated-at: "2026-09-22T22:29:40Z"
metric-files-to-touch: 6
metric-step-count: 10
has-blockers: true
revision-count: 0
revisions: []
consult-runs: []
tags: [probe, cli, observability, help-text, awaiting-input]
stack-source: confirmed
open-questions:
  - id: Q-1
    class: intent-bearing
    question: "A simulate or bench run that fails prints prose on stderr and no record. Does a failed run emit a structured record with outcome error, or does the contract narrow so that a failed run stays prose-only?"
    options: [A-emit-failure-record-on-stdout, B-narrow-the-contract-prose-only, C-emit-and-persist-under-the-data-folder]
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-probe-engine-core.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-calibration.md, 04-plan-commentary.md]
  contract: ../../observability.md
  implement: 05-implement-probe-engine-core.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine probe-engine-core"
---

# Plan: Probe Findings on the Engine Command Line

## The Plan

The probe of the engine command line gave five findings, two at `medium` and three at `low`. The data-schemas-generator slice planned findings 1, 3, 4, and 5 in its command-line step, and that slice is implemented and verified. This run drove the current release binary again. Finding 1 is fixed: `--json` wrote `match.jsonl` beside `match.ticks`, and the help states the replace rule. Finding 3 is fixed: an unwritable path names "os error 5" once. Finding 4 is fixed: redirected stderr has no escape codes. Finding 5 is fixed under `--help` only. Under `-h`, all 8 help surfaces have a line of 94 to 102 columns. The installed clap source shows the cause: without the `wrap_help` feature, clap does not wrap help text (`clap_builder-4.6.7/src/output/textwrap/mod.rs:26-28`). Findings 1, 3, and 4 also have no regression test. Steps 1 to 4 close these gaps with text edits and tests only, and add no dependency.

Finding 2 is still open. A failed run prints prose on stderr and nothing on stdout. This run reproduced it three times: `simulate --minutes 0`, `simulate` with a directory as `--ticks-out`, and `bench --matches 0` each exit 1 with 0 bytes on stdout. The probe gives two remedies. Remedy A emits a record with `outcome: error`. Remedy B narrows the contract so that a failed run stays prose-only. The instrumentation contract lists `error` as an `outcome` value, and the observability contract (plan-version 1, product-owner confirmed) defines the `error.type`, `error.code`, and `error.retriable` keys. But no contract gives the record shape for a run that fails before kick-off. Two earlier artifacts routed this choice away from planning (`04-plan-data-schemas-generator.md:164`, `05-implement-data-schemas-generator.md:145`). The choice decides what the headless data contract of charter C4 says about failure. For these reasons the plan stops at **awaiting-input** on one question, Q-1, and recommends option A. Steps 5 to 9 are written for option A and wait for the answer.

When Q-1 is answered, run this stage again. The run clears the blocker and adjusts steps 5 to 9 if the answer is B or C. The top risk after Q-1 is a disagreement between two plans. `04-plan-calibration.md` step 15 writes `outcome: "failure"`, but the contract value is `error`. This plan uses `error`. The calibration plan must align before calibration is implemented.

## Current State

- Release binary `target/release/engine-cli.exe`, built 2026-09-22 23:14 local time, after HEAD `837cb5c`. This run drove it.
- Finding 1 (dump path): fixed. `simulate --seed 1 --minutes 1 --no-snapshot --json --ticks-out <tmp>/match.ticks` exited 0 and wrote `match.jsonl` and `match.ticks`. The `--json` long help says "The dump path is the tick file path with its extension replaced by .jsonl" (`crates/engine-cli/src/cli.rs:59-65`). `resume --json` uses the same rule (`crates/engine-cli/src/resume.rs:125`), but its help says only "Also write a JSON Lines dump beside the tick file" (`cli.rs:88-90`). No test asserts the dump path.
- Finding 2 (no record on failure): open. `simulate --minutes 0` exits 1 with `error: invalid configuration: minutes must be 1 to 200, got 0` and 0 bytes on stdout. A directory as `--ticks-out` exits 1 with 0 bytes on stdout. `bench --matches 0` exits 1 with `error: --matches must be at least 1` and 0 bytes on stdout. Every success path prints its record through `emit_line` (`crates/engine/src/observe/mod.rs:244-250`), and the error branch in `main.rs:39-45` prints prose only. `bench.rs:23` uses an untyped `anyhow::bail!`. `MatchStats` holds 16 required fields, 9 of them measurements (`observe/mod.rs:63-96`), so a record for a match that never started cannot reuse it without zero values.
- Finding 3 (repeated OS text): fixed. `main.rs:42` prints `{err:#}`, and the unwritable-path message names "(os error 5)" once. No test asserts this.
- Finding 4 (ANSI on redirected stderr): fixed. `main.rs:27` sets `with_ansi(std::io::stderr().is_terminal())`. `cat -v` on the redirected stderr shows no escape sequence. No test asserts this.
- Finding 5 (help width): fixed under `--help` only. `no_help_line_exceeds_eighty_columns` (`crates/engine-cli/tests/cli_args.rs:184-197`) reads `--help` for 4 of the 8 help surfaces. This run measured every surface. `--help` is at most 80 columns everywhere. `-h` is 94 columns on the top level and on `simulate`, `generate`, `serve`, `record`, and `resume`, 97 on `bench`, and 102 on `replay`. The long lines are `--content-dir` (every subcommand), `replay --speed`, `bench --matches`, `bench --json`, and `bench --stream`.
- The observability contract (`.ai/observability.md`) Block A defines `outcome: [operation, duration_ms, outcome, status]` and `error: [error.type, error.code, error.retriable]`. `match-stats` is "emitted once at full time or abandonment" with keys that include "outcome, error". The instrumentation contract (`history/04b-instrument-0.md:72`) says "`outcome` is `success`, `error`, or `abandoned`".
- `crates/engine-cli/tests/` holds `cli_args.rs`, `resume.rs`, `stream_cli.rs`, and `web_cli.rs`. The tests drive the built binary through `std::process::Command` (`cli_args.rs:9-23`).

## Simplicity Ladder

- Help width under `-h` → rung 4 new-code (text only). Rung 2: clap wraps help only with the `wrap_help` feature (`clap_builder-4.6.7/src/output/textwrap/mod.rs:13-28`; `Cargo.toml` of the same crate: `wrap_help = ["help", "dep:terminal_size"]`). The feature adds the `terminal_size` crate, so it is rejected. `next_line_help(true)` is built in, but it changes the layout of every help screen. Shortening five strings is the smallest change.
- Dump-path regression test → rung 3 reuse: `bin()`, `content_dir()`, and `temp()` in `crates/engine-cli/tests/cli_args.rs:9-23`.
- Single OS-error and no-ANSI tests → rung 3 reuse: the same helpers. `std::process::Command` captures stderr as a pipe, which is the non-terminal case.
- Failure record (awaiting Q-1) → rung 3 reuse with modification: `Record` and `to_json` (`observe/mod.rs:46-51`, `225-240`) already write the envelope keys. A new `FailureRecord` implements `Record`. Extending `MatchStats` is rejected, because a record for a match that never started would carry zero values that look like measurements.
- Error classification (awaiting Q-1) → rung 3 reuse: `EngineError` (`crates/engine/src/error.rs:7-40`) already separates configuration, input and output, format, sink, and content errors. A `classify()` method maps each variant to the three `error.*` keys.

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist, and no `solutions.globalDir` is configured (`.ai/sdlc-config.json` is absent).

Repeat-deferral tripwire: `runtime-evidence-deferrals` in `00-index.md` is `[]`. The tripwire does not fire.

## Likely Files / Areas to Touch

- `crates/engine-cli/src/cli.rs`: shorten five short-help strings; add the replace rule to `resume --json`.
- `crates/engine-cli/tests/cli_args.rs`: widen the 80-column test; add regression tests for findings 1, 3, and 4; add failure-record tests (awaiting Q-1).
- `crates/engine/src/observe/mod.rs`: `FailureRecord` and `EngineError::classify()` (awaiting Q-1).
- `crates/engine-cli/src/main.rs`: emit the failure record on the error branch of `simulate` and `bench` (awaiting Q-1).
- `crates/engine-cli/src/bench.rs`: a typed error for `--matches 0` (awaiting Q-1).
- `README.md`: describe the failure record (awaiting Q-1).

## Proposed Change Strategy

Work in two groups. Group one (steps 1 to 4) does not depend on Q-1. It completes findings 1, 3, 4, and 5 with help-text edits and regression tests, and it changes no runtime behaviour except the help text.

Group two (steps 5 to 9) implements option A of Q-1. The error branch of `simulate` and `bench` prints one record on stdout before the existing prose line. The exit code stays 1. The record carries the envelope keys, `operation`, `outcome: "error"`, `duration_ms`, `seed`, `error.type`, `error.code`, and `error.retriable`, and it carries no statistic key. `simulate` fails as `record.kind: match-stats`, and `bench` fails as `record.kind: run-report`, which are the kinds these commands write when they succeed. Nothing is written under the data folder. The persisted `stats.json` layout therefore does not change. Argument-parse failures (exit 2) happen before a run starts, so they stay prose-only.

The NFR rationale is charter C4 ("emits structured events and statistics for the observability pipeline"), which the product owner ratified. No unranked NFR narrows a charter commitment in this plan.

## Step-by-Step Plan

1. **Short help within 80 columns (finding 5).** In `crates/engine-cli/src/cli.rs`, shorten the doc comment of each option whose `-h` line exceeds 80 columns. The short-help column starts at 27 characters, so each description must be at most 53 characters. Options: `--content-dir` ("Folder holding the content files."), `replay --speed` ("Playback speed; 1.0 is real time."), `bench --matches` ("Timed matches after one warm-up match."), `bench --json` ("Print the run report as one JSON line."), `bench --stream` ("Also stream one match to a fast client."). Move the removed detail into each option's `long_help` so `--help` keeps it. Keep every `--help` line at 80 columns or fewer.
2. **Resume dump-path help (finding 1).** Give `ResumeOpts::json` the same `long_help` as `SimulateOpts::json` (`cli.rs:59-65`), which states that the extension is replaced by `.jsonl`.
3. **Widen the help test (finding 5).** In `crates/engine-cli/tests/cli_args.rs`, change `no_help_line_exceeds_eighty_columns` to read both `-h` and `--help` for the top level and for `simulate`, `bench`, `generate`, `serve`, `record`, `replay`, and `resume`. That is 16 surfaces.
4. **Regression tests for findings 1, 3, and 4.** In `cli_args.rs`, add:
   - `json_dump_replaces_the_tick_file_extension`: `simulate --seed 7 --minutes 1 --no-snapshot --json --ticks-out <tmp>/match.ticks`. Assert exit 0, `<tmp>/match.jsonl` exists with 3,000 lines, and `<tmp>/match.ticks.jsonl` does not exist.
   - `an_unwritable_tick_path_names_the_os_error_once`: create a directory `<tmp>/dir`, run `simulate --seed 7 --minutes 1 --no-snapshot --ticks-out <tmp>/dir`. Assert exit 1, stderr names the path, and the text "os error" occurs exactly once.
   - `redirected_stderr_carries_no_ansi_escape`: run `simulate --seed 7 --minutes 1 --no-snapshot` with stderr captured. Assert the stderr bytes contain no `0x1B`.
5. **(Awaiting Q-1) Failure record.** In `crates/engine/src/observe/mod.rs`, add `FailureRecord { kind: &'static str, operation: &'static str, owner_id: String, seed: u64, duration_ms: u64, error_type: String, error_code: String, error_retriable: bool }`. Implement `Record` so that `kind()` and `operation()` return the stored values. Serialize `outcome` as `"error"` and the keys `error.type`, `error.code`, and `error.retriable`. Use the owner identity only when the owner file already exists. A failure path never creates it. Add a unit test that asserts the key set: the envelope keys, `operation`, `outcome`, `duration_ms`, `seed`, and the three `error.*` keys, with no `ticks.written` or `stats.*` key.
6. **(Awaiting Q-1) Error classification.** Add `EngineError::classify(&self) -> (&'static str, &'static str, bool)`: `InvalidConfig` → `("config", "invalid-config", false)`; `Io` and `Read` → `("io", "io", false)`; `Format` → `("format", "tick-format", false)`; `Sink` → `("sink", "sink-stopped", true)`; `Data` and `Version` → `("content", "content-refused", false)`. In `crates/engine-cli/src/main.rs`, downcast `anyhow::Error` to `EngineError` through its chain. Use `("internal", "unclassified", false)` when no `EngineError` is found. Unit-test every variant.
7. **(Awaiting Q-1) Typed bench check.** In `crates/engine-cli/src/bench.rs:23`, return `EngineError::InvalidConfig("--matches must be at least 1".into())` in place of `anyhow::bail!`. The stderr text becomes `error: invalid configuration: --matches must be at least 1`.
8. **(Awaiting Q-1) Emit on the error branch.** In `main.rs`, take `Instant::now()` and the command's seed before dispatch. On `Err` from `Simulate`, print one `FailureRecord` with kind `match-stats` and operation `simulate` through `emit_line`. On `Err` from `Bench`, use kind `run-report` and operation `benchmark`. Then print the existing prose line on stderr and exit 1. If `emit_line` fails, still print the prose line and exit 1. `generate`, `serve`, `record`, `replay`, and `resume` keep prose only.
9. **(Awaiting Q-1) Failure tests and README.** In `cli_args.rs`, add three tests. `simulate --minutes 0` gives exit 1, exactly one stdout line with `record.kind` `match-stats`, `outcome` `error`, and `error.type` `config`, and the prose line on stderr. A directory as `--ticks-out` gives `error.type` `io`. `bench --matches 0` gives `record.kind` `run-report`, `outcome` `error`, and `error.type` `config`. Keep `valid_seed_exits_zero` unchanged as the success-path guard. In `README.md`, state in the simulate and bench sections that a failed run prints one record with `outcome` `error` and exits 1.
10. **Gates and record.** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p engine-cli`, `cargo test -p engine --lib observe`, and `cargo build --release --workspace`. Run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` once, and confirm that `bench.match_wall_ms` is within 10 percent of the latest `05c-benchmark.md` baseline. No simulation path changes, so this is a guard only. Search the diff for workflow vocabulary (slice names, stage names, artifact paths) before the commit. Record the numbers and the resolution of each finding in the implement artifact.

## Verification Strategy

Every row is a user-observable command-line behaviour from the probe findings. The `cli` adapter drives the real release binary as a subprocess, and the stack confirms `cargo-test` (`00-index.md` `stack.testing`).

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| F1: the `--json` dump path is the one the help names | `cargo test -p engine-cli` subprocess drive plus one manual drive with the cli adapter (rung: real binary, headless) | local Windows machine, temp folder — yes | test `json_dump_replaces_the_tick_file_extension` (step 4) | manual drive with `ls` of the folder → none needed |
| F2: a failed run emits a structured record (awaiting Q-1) | `cargo test -p engine-cli` subprocess drive plus manual drives of the three probe failure paths (real binary) | local machine — yes | steps 5 to 9; a directory as `--ticks-out` is the portable unwritable-path fixture | manual drive of each failure path with stdout captured to a file → none needed |
| F3: the OS error is named once | `cargo test -p engine-cli` subprocess drive (real binary) | local machine — yes | test `an_unwritable_tick_path_names_the_os_error_once` (step 4) | manual drive → none needed |
| F4: redirected stderr carries no ANSI escape | `cargo test -p engine-cli` subprocess drive; stderr is a pipe, which is the non-terminal case (real binary) | local machine — yes | test `redirected_stderr_carries_no_ansi_escape` (step 4) | `cat -v` on a redirected stderr file → none needed |
| F5: no help line exceeds 80 columns | `cargo test -p engine-cli` over 16 help surfaces (real binary) | local machine — yes | widened test (step 3) | manual `awk 'length($0)>80'` over each surface → none needed |

No row depends on credentials, a device, an external service, or a deploy target. No `constraint-resolution:` line is required.

## Test / Verification Plan

### Automated checks

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`: blocking gates.
- `cargo test -p engine-cli`: the widened help test, the three regression tests from step 4, and (after Q-1) the three failure-record tests from step 9. The existing tests `invalid_seed_exits_non_zero_and_names_the_argument` and `valid_seed_exits_zero` must still pass unchanged.
- `cargo test -p engine --lib observe`: the `FailureRecord` key-set test and the classification test (after Q-1).
- `cargo test --workspace`: no other test may change.

### Interactive verification (human-in-the-loop)

Platform `cli` from `stack.platforms`. Driver: the release binary through Git Bash, as the probe drove it. Evidence goes to `verify-evidence/probe-engine-core/` as `<drive>.stdout.txt`, `<drive>.stderr.txt`, and `<drive>.exit-code`, the layout of the cli adapter.

1. `cargo build --release --workspace`.
2. F1: `target/release/engine-cli.exe simulate --seed 7 --minutes 1 --no-snapshot --json --ticks-out $T/match.ticks`, then `ls $T`. Pass: `match.jsonl` exists and `match.ticks.jsonl` does not.
3. F2 (after Q-1): run `simulate --seed 7 --minutes 0 --ticks-out $T/a.ticks`, `simulate --seed 7 --minutes 1 --no-snapshot --ticks-out $T/dir` (a directory), and `bench --seed 7 --matches 0`. Pass for option A: each exits 1, stdout holds exactly one JSON line with `outcome` `error` and the three `error.*` keys, and stderr holds the prose line.
4. F3: the directory drive from item 3. Pass: "os error" occurs once in stderr.
5. F4: `cat -v` on the stderr file of item 2. Pass: no `^[` sequence.
6. F5: for each of the 16 help surfaces, `engine-cli <cmd> -h | awk 'length($0)>80'` and the same with `--help`. Pass: no output.

## Risks / Watchouts

- Q-1 is a contract decision (high). A failure record changes what the command line prints on stdout when a run fails. The plan does not settle it alone. Mitigation: the plan stops, and steps 5 to 9 wait.
- Outcome value drift (medium). `04-plan-calibration.md` step 15 writes `outcome: "failure"`. The instrumentation contract lists `success`, `error`, and `abandoned`. A dashboard that filters on `error` would miss calibration failures. Mitigation: this plan uses `error`, and the conflict is recorded here for the calibration plan to correct before calibration is implemented.
- Help-text regression (low). A later option with a long description breaks `-h` again. Mitigation: the widened test covers every subcommand under `-h`.
- Stdout consumers (low, after Q-1). A script that treats non-empty stdout as success would misread a failure. The exit code stays 1, and the record says `outcome: error`. Mitigation: the README states this.

## Dependencies on Other Slices

- `engine-core` and `data-schemas-generator`: implemented and verified. Their command-line step fixed findings 1, 3, 4, and 5 under `--help`. This plan adds tests and the `-h` fix.
- `match-rules`: implemented and verified. It added `resume`, `serve`, `record`, and `replay` help surfaces, which the widened test now covers.
- `calibration` (planned, not built): step 15 of its plan must use `outcome: "error"`, not `"failure"`. If Q-1 is answered A, calibration can reuse `FailureRecord` for its per-match failure record rather than write zero statistics.
- No slice depends on this one.

## Assumptions

- A-1 (`class: implementation-detail`). Findings 1, 3, and 4 are treated as fixed, and the plan adds only regression tests. Reason: this run reproduced the fixed behaviour on the current release binary. The data-schemas-generator verify artifact does not name these findings, so this run's drive is the only evidence.
- A-2 (`class: implementation-detail`). The `-h` overflow counts as a residual of finding 5. Reason: it is the same `boundary-overflow` class on the same help surfaces, and the probe's fix shape ("shorten the `--json` help sentence") covers it.
- A-3 (`class: implementation-detail`). Shorten the help strings rather than enable clap's `wrap_help` feature. Reason: the feature adds the `terminal_size` dependency (`clap_builder-4.6.7/Cargo.toml`, `wrap_help = ["help", "dep:terminal_size"]`). Shortening changes five strings and adds nothing.
- A-4 (`class: implementation-detail`). `resume --json` gets the same replace-rule help as `simulate --json`. Reason: both use `with_extension("jsonl")` (`simulate.rs:60`, `resume.rs:125`), and the finding's class is copy that disagrees with behaviour.
- A-5 (`class: implementation-detail`). The regression tests use the existing subprocess helpers in `cli_args.rs` and a directory as the unwritable path. Reason: a directory fails to open for writing on Windows and on other systems, and needs no permission change.
- A-6 (`class: implementation-detail`). Steps 5 to 9 are written for option A of Q-1 but are not authorised to run until Q-1 is answered. Reason: planning the recommended option in full lets a re-run of this stage clear the blocker quickly.
- A-7 (`class: implementation-detail`). The master `04-plan.md` is not updated in this run. Reason: the plan is not complete, and other stopped plans in this workflow also stay out of the master until their question is answered.
- A-8 (`class: implementation-detail`). No second-opinion consult runs. The `unknowns-present` trigger holds (Q-1), but the product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`).
- A-9 (`class: implementation-detail`). No augmentation artifact is re-authored. Reason: this slice changes no simulation path, so the instrumentation and benchmark artifacts for `match-rules` stay current. Step 10 runs the benchmark once as a guard only.

## Blockers

- **Q-1 (awaiting-input, `class: intent-bearing`).** A failed `simulate` or `bench` run prints prose on stderr and no record. Which contract applies?
  - **Option A (recommended).** Emit one record on stdout with `outcome: "error"` and `error.type`, `error.code`, and `error.retriable`. The record has the same `record.kind` the command writes when it succeeds, and it has no statistic key. Write nothing to the data folder. Exit 1, with the prose line kept on stderr. This implements keys that the confirmed observability contract already defines. It adds a stdout line on failure and changes no persisted file.
  - **Option B.** Narrow the contract. A failed run stays prose-only, and `outcome` for these two commands becomes `success` only. Steps 5 to 9 are dropped. This narrows a promise of charter C4, and the contract files must record the change.
  - **Option C.** Option A, plus a failure record saved under `SM_DATA_DIR`. This changes the persisted data layout, and a run that fails before kick-off has no `match.id` folder.
  - Why this stops the run: the choice decides what the headless data contract of charter C4 promises for a failed run, and it changes command-line output. The probe offered both A and B. Two earlier artifacts routed the choice outside planning. No contract gives the record shape for a run that fails before kick-off. Per the decision rules, a choice that could narrow a product-owner directive is not settled by an autonomous run.

## Freshness Research

- `clap` 4.6.7 (the locked version, `Cargo.lock`). The installed source was read at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/clap_builder-4.6.7/`. `src/output/textwrap/mod.rs:26-28`: without `wrap_help`, `wrap()` returns the content unchanged. `src/builder/styled_str.rs:78`: the same no-op for styled text. `src/output/help_template.rs:121-136`: the terminal width is only used for wrapping. `Command::term_width` and `max_term_width` exist without the feature (`src/builder/command.rs:1402-1443`), but they have no effect because nothing wraps. Conclusion: help width is controlled only by the text itself or by the `wrap_help` feature.
- `tracing-subscriber`: `with_ansi(std::io::stderr().is_terminal())` is already in place (`main.rs:27`). `std::io::IsTerminal` has been stable since Rust 1.70, and the workspace minimum is 1.87.
- `anyhow`: `{err:#}` prints the chain once, joined by ": ", as the comment at `main.rs:41` records and this run's drive confirmed.
- No new dependency is proposed.

## Recommended Next Stage

- **Option A (after Q-1 is answered):** `/wf plan football-manager-match-engine probe-engine-core`. The re-run records the answer, clears the blocker, and adjusts steps 5 to 9 if the answer is B or C.
- **Option B:** `/wf implement football-manager-match-engine probe-engine-core`, limited to steps 1 to 4 and 10, if the product owner wants the help and regression fixes now and Q-1 later. Those steps do not depend on Q-1.
- **Option C:** `/wf review football-manager-match-engine match-rules`. The review ledger can carry finding 2 as an item if the product owner prefers to batch it with the review fixes.
