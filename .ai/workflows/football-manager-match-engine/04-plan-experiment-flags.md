---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: experiment-flags
status: complete
stage-number: 4
created-at: "2026-09-22T22:27:20Z"
updated-at: "2026-09-22T22:27:20Z"
metric-files-to-touch: 23
metric-step-count: 20
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, calibration, feature-flags, experiment, deferred]
stack-source: confirmed
steering-honored:
  - "steer.md holds design direction for the viewer only; this slice changes no page, so no design constraint applies"
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-experiment-flags.md
  shape: 02-shape.md
  experiment: 04c-experiment.md
  instrument: 04b-instrument.md
  observability: ../../observability.md
  siblings: [04-plan-calibration.md, 04-plan-tactics-and-ai.md, 04-plan-integration.md, 04-plan-data-schemas-generator.md]
  implement: 05-implement-experiment-flags.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine experiment-flags"
---

# Plan: Feature Flags for Calibration Experiments

## The Plan

The tuning file holds four blocks of values and no switch (`crates/engine/src/data/tuning.rs:19-31`). The loader refuses an unknown key, checks the version first, and reports the first `garde` failure by its field path (`crates/engine/src/data/mod.rs:136-200`). The calibration harness that this slice extends is planned and not built: `04-plan-calibration.md` puts `engine-cli calibrate`, the worker processes, the run builder, and `report.json` in place. The tactics slice raises the tuning file to version 2. No competing decision model or shot model exists yet, so this slice builds the switch and the comparison, and ships no flag.

A flag is a named entry in a new `flags` block of `content/tuning.json`. It carries an owner, a hypothesis, a removal condition, a state, and optional overrides. `garde` 0.23 puts the map key in the error path (`garde-0.23.0/src/validate.rs:300-315`), so a missing owner is refused as `flags.<name>.owner`. A flag has an effect in one of two ways. Overrides patch tuning values, and the whole patched file is validated again. A code flag is named in a registry in the engine, which is empty when this slice lands. A flag that does neither is refused, because it switches nothing. `calibrate --pair <name>` runs the off arm and then the on arm on the same fixtures and seeds, and writes one `run-report` with both arms, one comparison row per suite and band, and a verdict. The record kind is still `run-report`, and every new key is additive. Twenty autonomous decisions are recorded under Assumptions, and all of them are implementation details.

Implement touches 23 files, 4 of them new, in 20 steps. The order is the flag type, then the loader, then the engine seam, then the harness, then the tests and the documents. When this slice lands, a proposed model can be compared with the current one on the bands, and the loser can be removed by a written checklist. The top risk is that the harness shapes change when the calibration slice lands. Step 1 reads the landed code and stops for a plan re-review if they differ.

## Current State

- Branch `feat/football-manager-match-engine` at `837cb5c`. `calibration`, `tactics-and-ai`, and `integration` are `status: defined` in `00-index.md`. Their plans exist (`04-plan-calibration.md`, `04-plan-tactics-and-ai.md`, `04-plan-integration.md`), and none of them is implemented.
- `TuningFile` (`crates/engine/src/data/tuning.rs:19-31`) has `schema_version`, `engine`, `generator`, `fatigue`, and `stream`, with `#[serde(deny_unknown_fields)]` and `garde` on every block. `TUNING_VERSION` is 1 (`tuning.rs:15`). The tactics plan raises it to 2 (`04-plan-tactics-and-ai.md`, Assumption 7).
- `load_json` (`crates/engine/src/data/mod.rs:136-200`) peeks `schema_version` first. It maps a `serde` error to a line and column, and a `garde` failure to the first field path of the report. It emits `content.refused` or `content.loaded`. A `serde` "missing field" error names no map key. So a required flag field is a `String` with `#[serde(default)]` and `garde` `length(min = 1)`, and the refusal then carries the flag name in its path.
- `garde` 0.23.0 is the version in `Cargo.lock`. Its `Validate` implementation for `BTreeMap` nests each key into the path (`~/.cargo/registry/src/*/garde-0.23.0/src/validate.rs:300-315`, read this session).
- `Content` (`data/mod.rs:207-252`) holds the three files and a SHA-256 over their digests. `MatchConfig` (`crates/engine/src/sim.rs:30-41`) copies `Tuning` and carries `content_hash`.
- The observability contract (`.ai/observability.md`, plan-version 1, locked) fixes `record.kind` to four values (line 21). A calibrate run emits one `run-report` (line 77, line 172). No flag key exists in the contract. The run-report schema file is planned with `additionalProperties: true` (`04-plan-calibration.md`, step 10).
- The integration plan writes `docs/how-to/modding.md`, `docs/reference/cli.md`, and `docs/reference/data-files.md`, and it adds a test that parses every `--help` flag and looks for it in `cli.md` (`04-plan-integration.md`, steps 12 and 13). This slice ships after integration, so its new flags must reach `cli.md`.
- No feature-flag framework is in the workspace: a search for `flag` in `crates/**/*.rs` this session found only the restart flag of the tick record and the content-folder argument.
- `04c-experiment.md` did not exist. `00-index.md` records the experiment augmentation as `deferred-to-experiment-flags`. This plan authors it.

## Simplicity Ladder

| Capability | Rung | Choice |
|---|---|---|
| Flag declaration and required fields | rung 3 reuse | `crates/engine/src/data/mod.rs` → `load_json()` with `serde` and `garde`; exact match; reuse as-is. `garde` 0.23 names the map key in the path (`garde-0.23.0/src/validate.rs:300-315`). |
| Override application | rung 3 reuse with modification | `serde_json::Value` (already a dependency) patches the tuning tree, then `serde_json::from_value::<TuningFile>` and `Validate::validate` run again. The bounds come from the existing `garde` rules, and no second bound table exists. |
| Code-flag registry | rung 1 stdlib | `pub const CODE_FLAGS: &[&str]`, a slice constant. No framework: none exists in the workspace, and the flags are read once at load, not at run time. |
| Command-line states | rung 3 reuse | `clap` 4.5 derive with a value parser for `name=on|off`; the existing `Cli` pattern in `crates/engine-cli/src/cli.rs`. |
| Paired execution | rung 3 reuse with modification | The calibration plan's parent and worker (`calibrate/mod.rs`, `calibrate/worker.rs`); run twice with a different arm folder. The fixtures and seeds do not depend on the flag states. |
| Side-by-side comparison and verdict | rung 4 new code | `report/compare.rs`: pure functions over two sets of suite aggregates. The calibration run builder aggregates one run and compares nothing. |
| Report validation in tests | rung 3 reuse | `boon` 0.6 (test-only), added by the calibration plan. |

## Applied Learnings

No applicable learnings found: `.ai/solutions/INDEX.md` does not exist, and `.ai/sdlc-config.json` does not exist, so no global folder is set. `runtime-evidence-deferrals` in `00-index.md` is empty, so the repeat-deferral tripwire does not fire.

## Likely Files / Areas to Touch

- `crates/engine/src/flags.rs` (new): `FlagDef`, `FlagState`, `FlagStates`, `ActiveFlags`, `CODE_FLAGS`, and `apply`.
- `crates/engine/src/data/tuning.rs`, `data/mod.rs`, `lib.rs`, `sim.rs`: the `flags` block, `Content::with_flags`, the digest, and `MatchConfig.flags`.
- `crates/engine/src/observe/mod.rs`: `tuning.flags_on` in match-stats, and the pair keys in the calibrate run-report.
- `crates/engine/tests/flags.rs` (new), `content.rs`: AC-2 and the shipped-file pin.
- `crates/engine-cli/src/cli.rs`, `calibrate/mod.rs`, `calibrate/worker.rs`, `bench.rs`: `--flag`, `--pair`, the arms, and the benchmark per arm.
- `crates/engine-cli/src/report/compare.rs` (new), `report/mod.rs`: the comparison and the verdict.
- `crates/engine-cli/tests/calibrate_pair.rs` (new): AC-1.
- `schemas/observability/run-report.schema.json`, `match-stats.schema.json`: the additive keys.
- `content/tuning.json`, `content/README.md`, `docs/how-to/modding.md`, `docs/reference/cli.md`, `docs/reference/data-files.md`, `README.md`: the empty block, the modder reference, and the removal checklist.

## Proposed Change Strategy

Flags are data in the tuning file, and they are resolved once, at load. The engine never asks a flag anything during a tick, except through `ActiveFlags::is_on`, which reads a sorted list built before the match. So the hot path does not change, and a match with no flag on is byte-identical to the build before this slice. The determinism tests hold that.

A flag has an effect in one of two ways. The first is overrides: a map from a dotted tuning path (`engine.shot_range`) to a value. When the flag is on, `apply` writes each value into a JSON copy of the file and parses and validates the result as a `TuningFile` again. So a flagged value must satisfy the same `garde` bound as a written value, and no second bound table exists. The second is a code flag: a name in `CODE_FLAGS` that engine code reads through `is_on`. A candidate decision model or shot model uses this form, and it arrives with that model. A flag that has no override and no code registration is refused, because it switches nothing. The loader also rejects any override to `schema_version` or into the `flags` block.

The comparison is one command. `calibrate --pair <name>` runs the off arm and then the on arm, each through the unchanged calibration parent and workers, into `runs/<run.id>/arms/off/` and `arms/on/`. The fixture list and the match seeds come from `--seed` alone, so the two arms play the same matches. Each arm measures its own single-thread figure under its own states. The run writes one `run-report` with `operation: calibrate`. It keeps the contract's four record kinds and adds the keys `calib.flags`, `calib.pair`, `calib.arms`, `calib.compare`, and `calib.verdict`. The verdict follows the rule in `04c-experiment.md`, and it does not set the exit code. A paired run exits 0 when both arms complete with zero dark paths and zero validator violations. NFR-1 (`yields-to: C2`, `02-shape.md`) stays the budget per 1000-match suite. A paired run is measured against it per arm and per suite. It is never met by running fewer matches per arm.

## Step-by-Step Plan

1. **Pre-flight: read the landed harness.** Confirm that `05-implement-calibration.md` and `06-verify-calibration.md` exist and that the calibration slice is verified. From the code, read: (a) the `Calibrate` arguments in `crates/engine-cli/src/cli.rs` and the hidden worker arguments; (b) `RunBuilder` and `CalibrationReport` in `report/mod.rs` and `observe/mod.rs`; (c) the signature of `bench::measure`; (d) the run folder layout; (e) `TUNING_VERSION` after the tactics slice. If (a) to (d) differ from this plan's assumptions, stop and run `/wf plan football-manager-match-engine experiment-flags` so that the plan is re-reviewed. Run `cargo test --workspace --release --no-fail-fast` and record the pass, fail, and ignore counts before any change. Change no file in this step.
2. **Flag types.** Add `crates/engine/src/flags.rs`. `FlagDef` has `owner`, `hypothesis`, and `removal_condition` (each a `String` with `#[serde(default)]` and `#[garde(length(min = 1))]`), `state: FlagState` (`off` or `on`, default `off`), and `overrides: BTreeMap<String, serde_json::Value>` (default empty, `#[garde(skip)]`). Use `#[serde(deny_unknown_fields)]`. Add `pub const CODE_FLAGS: &[&str] = &[];`. Add `FlagStates` (a `BTreeMap<String, FlagState>` with `FromStr` for `name=on|off`), and `ActiveFlags` (a sorted `Vec<String>` with `is_on(&self, name) -> bool`, which carries `debug_assert!(CODE_FLAGS.contains(&name))`). Export them from `lib.rs`.
3. **The flags block.** In `crates/engine/src/data/tuning.rs`, add `#[serde(default)] #[garde(dive, custom(each_flag_switches_something))] pub flags: BTreeMap<String, FlagDef>` to `TuningFile`. The custom rule refuses a flag that has no overrides and is not in `CODE_FLAGS`, with the message "flag <name> switches nothing: add overrides or register it in code". It also refuses a flag name that is not lower snake case, at most 40 characters. Do not change `TUNING_VERSION`: the block is optional, and an existing file loads unchanged.
4. **Apply.** In `flags.rs`, add `apply(file: &TuningFile, states: &FlagStates) -> Result<(TuningFile, ActiveFlags), EngineError>`. The effective state of each flag is the command-line state if one is given, otherwise the file state. A state for an undeclared name is refused with `EngineError::Data { kind: "tuning", field: "flags.<name>", reason: "not declared in tuning.json" }`. For every flag that is on, write each override into a `serde_json::to_value(file)` tree. A path that does not exist, a path that starts at `schema_version` or `flags`, or a value of the wrong JSON type is refused with the field `flags.<name>.overrides.<path>`. Then parse the tree with `serde_json::from_value::<TuningFile>` and run `validate()`. The first `garde` error is reported as `flags.<name>: <path> <message>`. Two flags that are on and override the same path are refused, naming both flags.
5. **Content.** In `crates/engine/src/data/mod.rs`, add `Content::with_flags(&self, states: &FlagStates) -> Result<Content, EngineError>`. It calls `apply`, replaces `tuning`, and stores `ActiveFlags`. When any effective state differs from the file state, it replaces `digest` with SHA-256 over the old digest, the bytes `b"flags:"`, and the sorted on-list joined by commas. It emits `tracing::info!(signal = "flag.active", flag, owner, source = "file"|"cli")` for each flag that is on. `Content::load` calls `with_flags(&FlagStates::default())`, so the file states always apply and every command sees the same effective tuning.
6. **Engine seam.** In `crates/engine/src/sim.rs`, add `pub flags: ActiveFlags` to `MatchConfig` and copy it from `Content` in `MatchConfig::new`. Add no call site: no code flag exists yet. Update every `MatchConfig` literal in tests that the compiler reports.
7. **Engine tests (AC-2).** Add `crates/engine/tests/flags.rs`. Each case writes a temporary content folder: copy the three shipped files, then replace `tuning.json` with a patched copy.
   - For each of `owner`, `hypothesis`, and `removal_condition`: the key missing, and the key an empty string. That is six cases. Each gives `EngineError::Data` whose `field` contains `flags.probe_flag.<field>`, and whose `Display` names `probe_flag`.
   - A flag with no overrides and no registration is refused with a message that names it.
   - An override to `engine.no_such_knob` is refused, naming the flag and the path.
   - An override of `engine.shot_range` to 999.0, which is above its bound, is refused naming the flag.
   - Applying `probe_flag=on` changes `engine.shot_range` and no other value. Compare the two files as `serde_json::Value`.
   - The content digest with the flag on differs from the digest with the flag off. The digest with no states given equals `Content::load` before this slice, for the shipped folder.
   - A match with no flag on is byte-identical to the build before this slice: the existing `determinism.rs` stays green unchanged.
8. **Shipped-file pin.** In `crates/engine/tests/content.rs`, assert that every name in `CODE_FLAGS` is declared in the shipped `tuning.json`, and that the shipped `flags` block is empty. When a model lands with its flag, the pin changes in the same step.
9. **Statistics key.** In `crates/engine/src/observe/mod.rs`, add `#[serde(rename = "tuning.flags_on", default)] flags_on: Vec<String>` to `MatchStats`, and fill it from `MatchConfig.flags` wherever a `MatchStats` is built (`simulate.rs`, `resume.rs`, the calibration worker, and the literal in `crates/engine/tests/identity.rs`). Add the optional `tuning.flags_on` property (an array of unique strings) to `schemas/observability/match-stats.schema.json`.
10. **Command line.** In `crates/engine-cli/src/cli.rs`, add these arguments to `Calibrate`:
    - `--flag <NAME=on|off>`, repeatable, parsed by `FlagStates::from_str`. A value other than `on` or `off` fails in `clap` with the value named.
    - `--pair <NAME>`. A name that is also given with `--flag` is refused: "a paired flag takes both states".
    - The hidden worker argument `--flag-states <name=on,...>`, which forwards the resolved states.
    - Keep each help line under 80 columns.
11. **Worker.** In `crates/engine-cli/src/calibrate/worker.rs`, parse `--flag-states`, call `Content::with_flags`, and build every `MatchConfig` from the result. Leave the fixture planning untouched, so the match seed of fixture i is the same in both arms.
12. **Benchmark per arm.** In `crates/engine-cli/src/bench.rs`, change `measure` to take the `Content` it runs on. `bench::run` passes the loaded content, so the `bench` output stays key-for-key identical. Check this by diffing the key set of one `bench --seed 42 --matches 1 --json` line before and after the change.
13. **Comparison.** Add `crates/engine-cli/src/report/compare.rs`:
    - `compare(off: &SuiteAggregates, on: &SuiteAggregates, bands: &Bands) -> Vec<CompareRow>`, one row per suite and band. The bands are goals per match, shots per team, possession home and away, and the stronger team's win rate. Each row carries `suite`, `band`, `lo`, `hi`, `off`, `on`, `delta`, `off_pass`, and `on_pass`.
    - `verdict(rows, off_guard, on_guard) -> Verdict`, per `04c-experiment.md` §4:
      - `on-rejected` when the on arm has a non-zero dark path, a validator violation, or a single-thread per-match time more than 10 percent above the off arm's.
      - Otherwise, the arm that passes more bands wins.
      - On equal passes, the arm with the lower summed normalized distance to the band centres wins, where the distance of one band is `|v − (lo+hi)/2| / ((hi−lo)/2)`, and the difference must exceed 0.05.
      - Otherwise `no-difference`.
    - `render_table(rows, verdict) -> String`: a fixed-width table with the columns band, band range, off, on, and delta.
    - Unit tests use hand-built aggregates, one for each of the four verdicts and one for the 0.05 margin boundary.
14. **Parent and report.** In `crates/engine-cli/src/calibrate/mod.rs`, resolve `--flag` and `--pair` against the loaded content before any worker starts, so that an undeclared name exits 2 with the step 4 message and spawns nothing.
    - **Unpaired run:** run as today with the states forwarded. `report.json` gains `calib.flags` (every declared flag, with its effective state and its source).
    - **Paired run:** run the off arm into `arms/off/`, then the on arm into `arms/on/`. Each arm has its own workers, suite timing, event pruning, and `bench::measure` under its own content. Build one `CalibrationReport` with:
      - `calib.pair { flag, owner, hypothesis, removal_condition, pinned }`;
      - `calib.arms { off, on }`, each with the per-arm suites, `wall_ms`, dark paths, `validate.violations`, and `bench.*`;
      - `calib.compare`, the rows from step 13;
      - `calib.verdict`.

      Keep the top-level `calib.suites` equal to the off arm, so a reader of an unpaired report still finds its keys. Print the JSON line on stdout as today, and print `render_table` on stderr. Emit `tracing::info!(signal = "calibrate.pair", run.id, flag, verdict)`. Exit 0 when both arms complete with zero dark paths and zero violations; otherwise exit 2.
15. **Report schema.** In `schemas/observability/run-report.schema.json`, under `operation: calibrate`, add the optional properties `calib.flags`, `calib.pair`, `calib.arms`, `calib.compare`, and `calib.verdict` (an enum of the four verdicts). Add a `dependentRequired` rule: `calib.pair` requires the other three pair keys. Keep `additionalProperties: true`.
16. **Paired run test (AC-1).** Add `crates/engine-cli/tests/calibrate_pair.rs`. It writes a temporary content folder whose `tuning.json` declares `probe_short_range` (with an owner, a hypothesis, a removal condition, `state: off`, and `overrides: { "engine.shot_range": 12.0 }`). It sets `SM_DATA_DIR` to a temporary folder. Then:
    - `calibrate --seed 1 --matches 4 --minutes 5 --jobs 2 --pair probe_short_range --content-dir <tmp>` exits 0 or 2. It writes 8 statistics files under `arms/off/stats/` and 8 under `arms/on/stats/`. The two arms hold the same set of `seed` values. `tuning.flags_on` is `[]` in every off record and `["probe_short_range"]` in every on record.
    - `report.json` validates against `run-report.schema.json` with `boon`. It holds `calib.arms.off`, `calib.arms.on`, and one `calib.compare` row per suite and band, and each row has a numeric `off` and `on`. `calib.pair.removal_condition` equals the declared text.
    - An unpaired run with `--seed 1` and the same arguments gives match-stats figures equal to the off arm's, key for key.
    - `--flag nope=on` exits non-zero, and stderr names `nope`. `--flag probe_short_range=maybe` exits non-zero, and stderr names `maybe`.
17. **Content and reference.** In `content/tuning.json`, add `"flags": {}`. In `content/README.md`, add a flags section that lists the five fields, the rule that a flag must switch something, the override path form, the refusal messages, and a pointer to the removal checklist. In `docs/reference/data-files.md`, add the same block with the field types and requirements. In `docs/reference/cli.md`, add `--flag` and `--pair` under `calibrate`, with the `arms/` layout and the exit codes, so that the help-parsing documentation test stays green.
18. **How-to and removal checklist.** In `docs/how-to/modding.md`, add a section on comparing two models. It covers how to declare the flag, run `calibrate --seed 2026 --matches 1000 --pair <name>`, read the table and the verdict, and act. When the verdict falls within the margin, rerun once on a second seed, then decide; `no-difference` removes the candidate (`04c-experiment.md` §4). Then add the removal checklist:
    1. Delete the flag entry.
    2. For a winning override, fold its values into the base values. For a winning code flag, delete the losing branch. For a losing flag, delete its override or its code.
    3. Remove the name from `CODE_FLAGS`.
    4. Rerun unpaired `calibrate` and `bench`.
    5. Update the pinned tests.
    6. Record the report path in the change description.

    Add one paragraph on paired runs to the calibrate section of `README.md`.
19. **Benchmark check.** In release, run `engine-cli bench --seed 42 --matches 5 --json` three times, and compare the result against the calibration slice's recorded figures under the per-tick gate. With no flag on, the tick loop does not change. A regression over the tripwire means a flag lookup has reached the hot path. Find it and remove it, and never move the gate.
20. **Gates and boundary.** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `node --test web/tests/*.test.mjs`. Compare the counts with step 1. Search the changed source, comments, and documents for workflow vocabulary (stage names, slice slugs, `.ai/`), and rewrite any hit in product language before the commit.

## Verification Strategy

No user-observable AC — automated only. Both criteria in `03-slice-experiment-flags.md` are marked `observable: false`: AC-1 is a harness test, and AC-2 is a `cargo test`. Neither criterion depends on an environment outside the target machine, so no `constraint-resolution:` line is needed.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-1: paired calibration on the same seeds shows both bands side by side | `cargo test -p engine-cli --test calibrate_pair` (automated, `cli-direct` binary under test) | Windows reference machine, rustc 1.92: yes | A temporary content folder with a declared test flag that overrides `engine.shot_range`; `calib.compare` rows; `boon` validation against the run-report schema (steps 13 to 16) | A hand run of the same command with its `report.json` and stderr table kept as evidence; no deferral needed |
| AC-2: a flag without an owner, a hypothesis, or a removal condition is refused, naming the flag | `cargo test -p engine --test flags` (automated) | none beyond `cargo`: yes | Six refusal cases on a temporary content folder (step 7) | none needed |

## Test / Verification Plan

### Automated checks

- Format: `cargo fmt --all -- --check`.
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`.
- Unit and integration: `cargo test --workspace`. This runs `crates/engine/tests/flags.rs` (AC-2), `crates/engine/tests/content.rs` (the shipped-file pin), the `report/compare.rs` unit tests (the four verdicts and the margin), and `crates/engine-cli/tests/calibrate_pair.rs` (AC-1).
- Regression: `determinism.rs`, `snapshot.rs`, and the calibration smoke test stay green unchanged. The `bench` key set is unchanged (step 12). The documentation test from the integration slice stays green with the new flags in `cli.md` (step 17).
- Page: `node --test web/tests/*.test.mjs` runs unchanged as a regression gate.
- Criteria map:
  - AC-1: `calibrate_pair.rs`, the paired run on the same seeds with both values in each `calib.compare` row.
  - AC-2: `flags.rs`, six refusal cases each naming `probe_flag` and the missing field.

### Interactive verification (human-in-the-loop)

Automated only. Neither criterion is user-observable: the slice marks both `observable: false`, and the product owner reads a comparison through the report file, which the test validates.

## Risks / Watchouts

- **Flags outlive their experiment** (slice Risks). The removal condition is a required, non-empty field. A flag must switch something. Every flag that is on is logged at load, and the paired report repeats its removal condition. The how-to carries the removal checklist.
- **The harness is planned, not built.** Every harness name in this plan comes from `04-plan-calibration.md`. Step 1 stops the build for a plan re-review if the landed code differs.
- **An override escapes a bound.** `apply` validates the whole patched file again, and step 7 tests an out-of-bound override.
- **A paired run takes twice as long.** Two arms of two 1000-match suites are 4000 matches. The budget stays per 1000-match suite, and `--suite` narrows a paired run to one suite.
- **An unmarked result.** The on-list enters the content digest when any state differs from the file, so a snapshot of a flagged match is refused by a build without the flag. `tuning.flags_on` rides in every statistics record.
- **The tuning version.** The tactics slice raises `TUNING_VERSION` to 2. This slice adds an optional block and does not raise it again. Step 1 re-reads the version.

## Dependencies on Other Slices

- `calibration` (planned, not built): the `calibrate` command, the workers, `RunBuilder`, `CalibrationReport`, `bench::measure`, the bands file, the run-report schema, and `boon`. Step 1 checks these.
- `tactics-and-ai` (planned, not built): `TUNING_VERSION` 2. It is also the first likely source of a competing decision model.
- `integration` (planned, not built): `docs/how-to/modding.md`, `docs/reference/cli.md`, `docs/reference/data-files.md`, and the help-parsing documentation test. The slice definition places this slice after integration.
- `data-schemas-generator` (complete): `load_json`, `garde`, and `Content`.
- Downstream: the first competing model registers its code flag in `CODE_FLAGS` and declares it in `content/tuning.json` in the same change. The scripting runtime may later use the same flags block for scripted models.

## Assumptions

Every entry is `class: implementation-detail`, settled autonomously in place of the discovery interview. None of them touches an open or carried intent risk (all six in `00-index.md` are `adjudicated`), a product-owner directive, control authority, the core loop, or a committed capability.

1. **Plan now, ship later.** This plan is written while no competing model exists, and it ships no flag. Why: the slice scope is the switch, the paired run, the report, and the checklist. The product owner's deferral (slice Q4) decides when implement runs, not what the slice holds. The flag for a candidate model is declared together with that model.
2. **Two kinds of effect: overrides and code flags.** Why: overrides let a paired run compare two sets of tuning values with no new engine code, and they let AC-1 be tested with a real effect. The code registry is where a competing model's branch is gated. The alternative, code flags only, would force a test-only branch into the engine.
3. **A flag must switch something.** Why: a declared flag with no effect is the cheapest form of the slice's top risk (flag sprawl).
4. **Required text fields are `serde(default)` strings with `garde` `length(min = 1)`.** Why: a `serde` missing-field error names no map key. `garde` 0.23 puts the key in the path (`garde-0.23.0/src/validate.rs:300-315`), so AC-2's "naming the flag" holds for a missing key and an empty string alike.
5. **No tuning version bump.** Why: the block is optional with a default, and every existing file loads unchanged. The loader already refuses an unknown key in an older build.
6. **Patch, then validate again.** Why: the `garde` rules stay the only bound table (data-schemas-generator plan Q8, "bounds in code, values in the file").
7. **Dotted override paths across the whole file, except `schema_version` and `flags`.** Why: a candidate generator distribution or fatigue curve is as likely as an engine knob, and the path form is the same one the loader already reports in its errors.
8. **Two flags that are on may not override the same path.** Why: the last writer would win silently.
9. **Flags resolve once, at load.** Why: the slice puts runtime flag changes out of scope, and the hot path stays unchanged.
10. **The on-list enters the content digest only when a state differs from the file.** Why: a flagged match must not resume on an unflagged build. The shipped file with no flag keeps its digest, so every existing snapshot and pinned hash stays valid.
11. **`--pair <name>` runs both arms in one command, and `--flag` pins other flags.** Why: the slice names `calibrate --flag name=on|off` and paired runs. One command guarantees the same seeds by construction, and `--flag` keeps the literal form for single-state runs.
12. **One `run-report` for a paired run, with additive keys.** Why: the contract fixes four record kinds (`.ai/observability.md:21`) and one run-report per run (line 77). Additive keys follow the precedent of the calibration and match-rules slices.
13. **The top-level `calib.suites` mirrors the off arm.** Why: readers of an unpaired report still find their keys in a paired one.
14. **The verdict does not set the exit code.** Why: a comparison has two legitimate outcomes. The exit code reports whether the run is trustworthy (dark paths, violations), as the calibration plan's exit 2 does for a failed run.
15. **The decision rule: bands passed, then normalized distance with a 0.05 margin; the on arm is rejected on a guardrail.** Why: the shape names the AC-3 and AC-4 bands as the metrics (`02-shape.md`, Augmentation Plan). The 10 percent guardrail is the existing CPU tripwire. The rule and its reasons are in `04c-experiment.md`.
16. **Each arm measures its own single-thread figure.** Why: a candidate model can cost ticks, and the guardrail needs the on arm's figure on the same machine in the same run.
17. **The removal checklist lives in `docs/how-to/modding.md`, with a pointer from `content/README.md`.** Why: the shape assigns tuning-file tasks to that how-to (`02-shape.md`, docs plan). A checklist next to the command keeps it in view.
18. **`04c-experiment.md` is authored by this plan and not registered in the index.** Why: `00-index.md` records the augmentation as deferred to this slice, and plan Step 0 item 7 authors deferred augmentations. The index is withheld from this run, so the driver records the `augmentations:` entry (`type: experiment`, `artifact: 04c-experiment.md`, `status: ready`). `04b-instrument.md` and `05c-benchmark.md` are not re-authored. The new signals are `flag.active` (step 5), `calibrate.pair` (step 14), `tuning.flags_on` (step 9), and the pair keys of the run-report (step 14), and the benchmark compares against the calibration slice's figures (step 19).
19. **No consult.** The trigger `appetite-medium-or-larger` holds (appetite: large). The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`), so no consult ran (`consult-runs: []`). The experiment augmentation's consult triggers (a proxy metric, no stopping rule, a gated charter commitment) do not hold. The metric is the shape's named bands, the stopping rule is fixed, and C3 is only measured, not gated.
20. **Autonomous answers are not written to `po-answers.md`.** Why: they are not product-owner answers. They are recorded here only.

## Blockers

None. `calibration` and `integration` are declared sequencing dependencies, not planning blockers. Step 1 gates implementation on the calibration harness.

## Freshness Research

- `garde` 0.23.0 (`Cargo.lock`): `impl Validate for BTreeMap<K, V>` nests each key into the error path (`~/.cargo/registry/src/index.crates.io-*/garde-0.23.0/src/validate.rs:300-315`, read this session). This is the basis for Assumption 4.
- `serde` "missing field" errors carry a line and a column, not the enclosing map key. That is why the loader reports `line N column M` for a `serde` failure (`crates/engine/src/data/mod.rs:171-176`). See `04-plan-data-schemas-generator.md` § Freshness Research for the `serde` error behaviour.
- `serde_json::from_value` goes through the same `Deserialize` implementation as `from_slice`, so `deny_unknown_fields` and every type check apply to a patched tree as they apply to the file (serde_json docs, `from_value`).
- `clap` 4.5 derive: `value_parser` with a `FromStr` type gives a refusal that names the bad value, and `action = ArgAction::Append` collects a repeatable flag ([clap derive reference](https://docs.rs/clap/latest/clap/_derive/index.html)).
- JSON Schema draft 2020-12 `dependentRequired` is supported by `boon` 0.6 (draft 2020-12 support, as the calibration plan records).
- No flag framework is needed. The flags are data read once at load, in one process, with no remote configuration. A hosted flag service would add a network dependency that NFR-5 and the local-only observability contract rule out.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine experiment-flags`. The plan is complete. Run it only after `calibration` is implemented and verified (step 1 reads its harness) and after `integration` has written the documents this slice extends. Compact the session first.
- **Option C:** `/wf slice football-manager-match-engine`. Use it only if the landed calibration harness makes this slice's boundary wrong. Nothing found in planning points to this.
