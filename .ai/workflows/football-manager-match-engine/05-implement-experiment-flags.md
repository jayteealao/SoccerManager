---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: experiment-flags
status: complete
stage-number: 5
created-at: "2026-09-23T17:10:17Z"
updated-at: "2026-09-23T17:10:17Z"
metric-files-changed: 26
metric-lines-added: 1968
metric-lines-removed: 139
metric-deviations-from-plan: 13
metric-review-fixes-applied: 0
commit-sha: "aca285f57c77fc87660f84509f2b2db02840f5ea"
commits:
  - "aca285f57c77fc87660f84509f2b2db02840f5ea"
steering-honored:
  - "steer.md holds design direction for the viewer only; this slice changes no page, so no design constraint applies."
  - "Dark-path counter definition: darkpath.change_never_applied keeps its zero rule and is an on-arm guardrail; change.expired_at_full_time is reported per arm with no zero rule and is not a guardrail."
  - "Output boundary: the commit message, code comments, docs, and test titles use product language; no deliberate-shortcut marker was needed, because the one lint suppression was removed by passing the run context to the worker command instead."
tags: [engine, calibration, feature-flags, experiment, tuning, schemas, docs]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-experiment-flags.md
  plan: 04-plan-experiment-flags.md
  experiment: 04c-experiment.md
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-commentary.md, 05-implement-calibration.md, 05-implement-viewer-match-day.md, 05-implement-viewer-lineup-tactics.md, 05-implement-viewer-reports-recovery.md, 05-implement-integration.md, 05-implement-extra-time-penalties.md]
  verify: 06-verify-experiment-flags.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine experiment-flags"
---

# Implement: Feature Flags for Calibration Experiments

## The Implementation

The build started from a landed and verified calibration harness, a tuning file with four blocks and no switch, and a plan of 20 steps written before that harness existed. The pre-flight read found the harness shapes the plan assumed: the `calibrate` arguments, `RunBuilder`, `CalibrationReport`, `bench::measure`, the `stats/` and `events/` run layout, and `TUNING_VERSION` 2. The baseline suite passed 341 tests with 0 failed and 4 ignored. A flag is now an entry in an optional `flags` block of `tuning.json` with a required owner, hypothesis, and removal condition. It switches something through overrides by dotted path, or through a name in the empty `CODE_FLAGS` registry. Every declared flag is tried alone when the content loads, so a bad override is refused whatever the flag's state. `Content::with_flags` applies command-line states over the file's states and hashes the on-list into the digest only when a state differs from the file.

`calibrate --pair <flag>` runs the off arm into `arms/off/` and the on arm into `arms/on/` through the unchanged worker processes, on the same fixtures and match seeds. It writes one `run-report` whose top-level figures are the off arm's, plus `calib.flags`, `calib.pair`, `calib.arms`, `calib.compare`, and `calib.verdict`. The verdict follows the experiment's rule and never sets the exit code. The build made 13 deviations from the plan, all minor and in scope. The largest is that `bench.rs` did not change, because `measure` already takes a `MatchConfig` that the parent now builds from flagged content. All 20 autonomous decisions are `implementation-detail`.

After the change, the release suite passes 364 tests with 0 failed and 4 ignored (23 new). The page suite passes 127 tests, and clippy and fmt are clean. The benchmark median is 418.8 ms per match against the 460.7 ms gate. The seed-42 match is byte-identical to the parent build `ee2ce12`. This lets verify prove AC-1 and AC-2 with `cargo test` alone. The top open risk is that no real candidate model exists yet, so the code-flag path (`ActiveFlags::is_on`) is exercised only when the first candidate registers a name.

## Summary of Changes

- New `flags` block in the tuning file. Each flag has a required owner, hypothesis, and removal condition, a state, and optional overrides. A flag that is missing a field, that switches nothing, whose name is not lower snake case, or whose override path is unknown, of the wrong type, forbidden, or out of bound is refused by name.
- `Content::with_flags(&FlagStates)` applies flag states once, at load. `Content::load` applies the file's own states. `MatchConfig.flags` carries the on-list into a match, and nothing reads it during a tick.
- `tuning.flags_on` is in every `match-stats` record (simulate, resume, serve, calibrate worker, failure record).
- `calibrate --flag NAME=on|off` (repeatable) and `calibrate --pair NAME`. A paired run plays both arms and writes one report with the arms, the comparison rows, and a verdict. The same table is printed on stderr. A bad name or state is refused before any worker starts.
- `report/compare.rs` compares the arms band by band, applies the decision rule (guardrails, then bands passed, then summed normalized distance with a 0.05 margin), and renders the table.
- Both record schemas gain the new optional keys. `dependentRequired` ties `calib.pair` to the arms, the comparison, and the verdict.
- `content/tuning.json` ships `"flags": {}`. The data-file reference, the command-line reference, the content README, the modding how-to (comparing two models and the removal checklist), and the README describe the feature.

## Files Changed

- `crates/engine/src/flags.rs` (new): `FlagDef`, `FlagState`, `FlagSetting`, `FlagStates`, `ActiveFlags`, `CODE_FLAGS`, `StateSource`, `effective`, `apply` / `patch`, and the block-level validation rule.
- `crates/engine/src/data/tuning.rs`: `TuningFile.flags` (optional, `dive` plus the block rule).
- `crates/engine/src/data/mod.rs`: `Content.flags`, the stored written tuning and digest, `Content::with_flags`, `Content::written_tuning`, and the `flag.active` signal.
- `crates/engine/src/lib.rs`: the `flags` module and its re-exports.
- `crates/engine/src/sim.rs`: `MatchConfig.flags`, copied from the content.
- `crates/engine/src/observe/mod.rs`: `MatchStats.flags_on` (`tuning.flags_on`).
- `crates/engine-cli/src/cli.rs`: `--flag` and `--pair` on `calibrate`, with help lines under 80 columns.
- `crates/engine-cli/src/calibrate/mod.rs`: resolves the states before any worker starts, runs one arm or two (`RunCtx::play_arm`), assembles the pair keys, prints the table, emits `calibrate.pair`, and returns the trust-based exit code.
- `crates/engine-cli/src/calibrate/worker.rs`: `Share.states`, content with flags, and `flags_on` in both statistics records.
- `crates/engine-cli/src/report/mod.rs`: `calib.flags`, `calib.pair`, `calib.arms`, `calib.compare`, `calib.verdict`, `FlagEntry`, `PairInfo`, and `ArmReport`.
- `crates/engine-cli/src/report/compare.rs` (new): `compare`, `verdict`, `render_table`, and six unit tests.
- `crates/engine-cli/src/{simulate,resume,serve}.rs`: `flags_on` in the statistics record.
- `crates/engine/tests/flags.rs` (new): eleven tests for refusals, application, and the digest (AC-2).
- `crates/engine/tests/content.rs`: the shipped-file pin (`CODE_FLAGS` declared, shipped block empty).
- `crates/engine/tests/identity.rs`: the `flags_on` field in the literal.
- `crates/engine-cli/tests/calibrate_pair.rs` (new): the paired run (AC-1), the single-state run, and the refusals.
- `schemas/observability/match-stats.schema.json`, `run-report.schema.json`: the additive keys.
- `content/tuning.json`: `"flags": {}`.
- `content/README.md`, `docs/reference/data-files.md`, `docs/reference/cli.md`, `docs/how-to/modding.md`, `README.md`: the flag fields, the refusals, the paired run, the verdicts, and the removal checklist.

## Shared Files (also touched by sibling slices)

- `crates/engine-cli/src/calibrate/mod.rs`, `worker.rs`, `report/mod.rs` (calibration): restructured into per-arm runs. An unpaired run keeps its folder layout and every report key. `calib.flags` is omitted when the file declares no flag.
- `crates/engine/src/observe/mod.rs` `MatchStats` (calibration, integration, extra-time-penalties): one additive, defaulted key.
- `crates/engine/src/data/mod.rs` `Content` and `crates/engine/src/sim.rs` `MatchConfig` (data-schemas-generator, tactics-and-ai, extra-time-penalties): additive fields. `Content` gained private fields, so it can no longer be built as a struct literal outside the crate. No caller did that.
- `docs/reference/cli.md`, `docs/how-to/modding.md`, `docs/reference/data-files.md`, `README.md` (integration): new sections. The help-parsing documentation test passes.
- `content/tuning.json`: the byte change moves the shipped content hash once. Tick records do not change.

## Notes on Design Choices

- Flags are data and are read once. `apply` returns the file unchanged when no flag is declared, so the shipped content does no JSON round trip.
- Patch, then validate again: the `garde` rules stay the only bound table. Each flag is also tried alone, so an out-of-bound override is refused at load and not first seen on the day someone turns it on.
- `Content` keeps the tuning file as written. That lets a command-line `off` undo a flag the file turns on. Applying states over already-patched values would not undo it.
- The comparison works on the `BandCheck` lists that `RunBuilder::checks` already produces, so an arm's rows and its `calib.bands` cannot disagree.
- The paired report's top-level keys all describe the off arm, so a reader of an unpaired report finds the same meaning in the same keys.
- Workers receive the resolved states as repeated `--flag name=state` arguments, so no hidden argument was added.

## Verification Seams Built

- AC-1 (paired calibration on the same seeds shows both bands side by side) → a temporary content folder with the declared test flag `probe_short_range` (override `engine.shot_range` 12.0) at `crates/engine-cli/tests/calibrate_pair.rs:32`, and the paired-run test at `crates/engine-cli/tests/calibrate_pair.rs:97` (enables `cargo test -p engine-cli --test calibrate_pair` to check the same seed set in `arms/off` and `arms/on`, `tuning.flags_on` per arm, `boon` validation of `report.json`, and one `calib.compare` row per suite and band equal to each arm's `calib.bands` value).
- AC-1 decision rule → `compare`, `verdict`, and `render_table` at `crates/engine-cli/src/report/compare.rs:71`, `:92`, and `:128`, with unit tests for all four verdicts and the 0.05 margin boundary (enables `cargo test -p engine-cli report::compare`).
- AC-2 (a flag without an owner, a hypothesis, or a removal condition is refused naming the flag) → `required_text` at `crates/engine/src/flags.rs:86`, and six refusal cases in `crates/engine/tests/flags.rs:61` asserting the field path `flags.probe_flag.<field>` (enables `cargo test -p engine --test flags`).
- Shipped-file pin → `crates/engine/tests/content.rs:159`.
- Signals → `flag.active` at `crates/engine/src/data/mod.rs:303` and `calibrate.pair` at `crates/engine-cli/src/calibrate/mod.rs:206`.

## Deviations from Plan

1. `Content` holds four files (the tactics file landed after the plan), not three. Nothing else changes.
2. Step 12 was not needed. `bench::measure` takes a `&MatchConfig` (`crates/engine-cli/src/bench.rs`, read this run), and each arm builds its config from its own flagged content. `bench.rs` is unchanged, so the `bench` key set is unchanged by construction.
3. There is no hidden `--flag-states` worker argument. The parent forwards the resolved states as repeated `--flag name=state`.
4. An undeclared name, a bad state, and `--pair` given together with `--flag` for the same name all exit with code 1, not 2. That is the documented code for a bad flag or a refused content file (`docs/reference/cli.md`, Exit codes). No worker starts.
5. Every declared flag's overrides are tried alone at load, whatever its state. The plan checked overrides only for flags that were on, so the step 7 case "override to `engine.no_such_knob` is refused" could not hold for a flag that is off.
6. The required text fields use a custom `required_text` rule ("is required", which also refuses whitespace) instead of `garde` `length(min = 1)`. The map key is still in the path (`garde-0.23.0/src/validate.rs:300-317`, read this run).
7. `compare` takes both arms' `BandCheck` lists instead of `SuiteAggregates` plus `Bands`. The landed builder already turns aggregates into band checks.
8. The stronger team's win rate counts toward bands passed and is an on-arm guardrail, but it is left out of the normalized distance. The band has a lower bound only, so it has no centre.
9. In a paired run every top-level figure (bands, pass, bench, dark paths, events) describes the off arm, not only `calib.suites`. Each arm also reports `calib.workers_failed`.
10. `calib.flags` is omitted when the tuning file declares no flag, so a report from the shipped content is key-for-key what it was.
11. `Content::written_tuning()` is a new accessor. The parent needs the flags as written to list their states and sources.
12. The AC-1 test file also covers a single-state `--flag` run and four refusals, and it validates both arms' kept event files.
13. Adding `"flags": {}` to the shipped `tuning.json` changes its bytes, so the shipped content hash moves once (plan Assumption 10 expected the digest to stay). The flag logic itself leaves the digest alone when no state differs. The seed-42 tick records after the 64-byte header hash the same as the parent build (`b126cd59…`), and so does the JSON Lines dump (`8120abc5…`).

## Anything Deferred

- No competing decision or shot model ships, so `CODE_FLAGS` is empty and `ActiveFlags::is_on` has no call site. The first candidate model registers its flag and its branch in the same change (plan Assumption 1).
- The first 1000-match decision run is not part of this slice. It runs when a candidate exists.

## Known Risks / Caveats

- A paired run is twice as long: 4000 matches for both suites. The budget stays per 1000-match suite per arm, and `--suite` narrows a run.
- The time guardrail compares two five-match single-thread medians taken in the same run. On short matches this is noisy, so a verdict from a short smoke run means nothing. The decision run is 1000 full matches.
- A generator override changes the generated clubs in the on arm while the match seeds stay the same. This is intended, because a candidate generator distribution is a valid experiment, but that arm's opponents differ as well.
- No deliberate shortcut with a ceiling was introduced, so there is no debt marker.

## Assumptions

Each entry is an autonomous decision taken in place of a human gate. All are `class: implementation-detail`. None touches an open or carried intent risk (all six are `adjudicated` in `00-index.md`), a product-owner directive, control authority, the core loop, or a committed capability.

1. `class: implementation-detail`. The pre-flight found no substantive drift in the harness, so the plan stands and no re-plan was needed.
2. `class: implementation-detail`. Deviations 2 to 13 above are recorded as resolved minor drift.
3. `class: implementation-detail`. Try each declared flag alone at load (deviation 5).
4. `class: implementation-detail`. A custom non-blank rule for the required text fields (deviation 6).
5. `class: implementation-detail`. Forward states to the workers as `--flag` (deviation 3).
6. `class: implementation-detail`. Exit code 1 for a bad name or state (deviation 4).
7. `class: implementation-detail`. Compare over `BandCheck` lists and leave out `wall_ms` (deviation 7).
8. `class: implementation-detail`. Leave the one-sided win-rate band out of the distance; it stays a pass and a guardrail (deviation 8).
9. `class: implementation-detail`. The on-arm guardrails are dark paths (missing records, changes never applied, failed workers), validator violations, the `bench.match_wall_ms` ratio above 1.10, and the stronger team's band. The source is `04c-experiment.md` §3.
10. `class: implementation-detail`. The paired top-level keys mirror the off arm (deviation 9).
11. `class: implementation-detail`. `calib.flags` is omitted when empty (deviation 10). In a paired run the paired flag shows state `paired` and source `cli`.
12. `class: implementation-detail`. The digest takes the on-list only when a state differs from the file (plan Assumption 10). The shipped hash moves once because the file bytes changed (deviation 13).
13. `class: implementation-detail`. The arm order is off first, then on, into `arms/off` and `arms/on`. Both arms reuse the run's `millis`, so the match ids are the same across arms, in separate folders.
14. `class: implementation-detail`. `Content` stores the tuning file as written, so a command-line state can undo a state the file sets.
15. `class: implementation-detail`. The unit tests pin a 0.05 margin boundary on a shots difference of 0.2 (0.05 of the half-width). That case is `no-difference`.
16. `class: implementation-detail`. The one clippy `too_many_arguments` suppression was removed by passing the run context to the worker command, so no debt marker was needed.
17. `class: implementation-detail`. The long help of `--pair` was shortened to stay under 80 columns.
18. `class: implementation-detail`. The code is committed by explicit paths with `git commit -- <paths>`. Files that another session had staged (`crates/engine/tests/zz_stall_probe.rs`, `docs/design/realism/*`) were left out and left staged as found.
19. `class: implementation-detail`. The benchmark was compared with the gate in `05c-benchmark.md` (460.7 ms, 6.82 MB) and with the last recorded medians (418.8 ms). No re-baseline.
20. `class: implementation-detail`. The byte-identity evidence came from a release build of the parent commit `ee2ce12`, extracted by `git archive` into the scratchpad. The working tree was not touched.

Acceptance criteria classification: AC-1 is `build-capability`, proven by an automated harness test (`calibrate_pair.rs`). AC-2 is `build-capability`, proven by an automated `cargo test` (`flags.rs`). Both are marked `observable: false` in the slice definition, so neither needs runtime evidence beyond the tests.

## Freshness Research

- `garde` 0.23.0 (`~/.cargo/registry/src/*/garde-0.23.0/src/validate.rs:300-317`, read this run): `Validate for BTreeMap` nests each key into the path. `error.rs:213-237` (`Display for Path`) joins key components with `.`, so a missing owner reads `flags.probe_flag.owner`.
- `serde_json::from_value` goes through the same `Deserialize` implementation, so `deny_unknown_fields` and every type check apply to a patched tree. The unknown-key and wrong-type cases are covered by `crates/engine/tests/flags.rs`.
- `clap` 4.5 derive: a `Vec<FlagSetting>` with `FromStr` appends, and a bad value is refused with its text: `error: invalid value 'x=maybe' for '--flag <NAME=on|off>': state maybe is not on or off` (observed this run).

## Checks Run This Stage

- Baseline before any change: `cargo test --workspace --release --no-fail-fast` gave 341 passed, 0 failed, 4 ignored (47 result lines).
- After the change: the same command gave 364 passed, 0 failed, 4 ignored (49 result lines), exit 0.
- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: exit 0 (re-run after the last edit).
- `node --test web/tests/*.test.mjs`: 127 passed, 0 failed.
- `cargo test -p engine-cli --test calibrate_pair --test calibrate` after the last edit: 3 passed, and 2 passed with 1 ignored.
- `cargo test -p engine-cli --test docs`: 3 passed (every help flag is named in `cli.md`).
- `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` ×3: 418.8 / 422.0 / 418.8 ms processor time per match (median 418.8, gate 460.7), 6.555 / 6.582 / 6.746 MB peak memory (median 6.582, gate 6.82), 1.482 / 1.4933 / 1.482 µs per tick, 282,600 ticks per match, and `budget.pass` true on all three. No tripwire fired.
- Seed-42 `simulate --json --no-snapshot`, this build against the parent build `ee2ce12`: the JSON Lines dump is `8120abc554836ec9…` on both, the tick records after the 64-byte header are `b126cd5979161c65…` on both, and both files are 54,259,280 bytes.
- Boundary search over the added code and docs for workflow vocabulary: no hit.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine experiment-flags`. Both criteria are automated (`cargo test -p engine --test flags`, `cargo test -p engine-cli --test calibrate_pair`), and the benchmark compare and the byte-identity check are repeatable. Consider compacting the session first. The workflow state is in the artifact files.
- **Option B:** `/wf review football-manager-match-engine experiment-flags`. Skipping verify is not recommended, because the slice changes testable behaviour.
