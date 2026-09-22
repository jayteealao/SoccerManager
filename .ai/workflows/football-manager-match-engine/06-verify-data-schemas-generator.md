---
schema: sdlc/v1
type: verify
slug: football-manager-match-engine
slice-slug: data-schemas-generator
status: complete
stage-number: 6
created-at: "2026-09-22T10:01:47Z"
updated-at: "2026-09-22T10:01:47Z"
result: pass
metric-checks-run: 13
metric-checks-passed: 13
metric-acceptance-met: 6
metric-acceptance-total: 6
metric-acceptance-user-observable: 0
metric-acceptance-code-only: 6
metric-interactive-checks-run: 0
metric-interactive-checks-passed: 0
metric-issues-found: 0
metric-issues-found-initial: 1
metric-issues-found-final: 0
fix-rounds-run: 1
convergence: converged
verify-owned-fix-commit: "ae31329"
regression-tests-added: 1
constraint-resolution-missing: []
interactive-verification: not-applicable
adapters-used: [cli]
bootstrap-failures: []
evidence-dir: ".ai/workflows/football-manager-match-engine/verify-evidence/data-schemas-generator/"
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
adversarial-tests-run: 24
adversarial-tests-failed: 1
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
tags: [engine, data, generator, modding, rust, benchmark]
refs:
  index: 00-index.md
  verify-index: 06-verify.md
  slice-def: 03-slice-data-schemas-generator.md
  plan: 04-plan-data-schemas-generator.md
  implement: 05-implement-data-schemas-generator.md
  benchmark: 05c-benchmark.md
  instrument: 04b-instrument.md
  review: 07-review-data-schemas-generator.md
  adapters: runtime-adapters.md
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine data-schemas-generator"
---

# Verify: Data Schemas and Team Generator

## The Verification

Implement handed over two commits, 47 files, 65 tests, and a compare measurement inside every tripwire. The slice carries six acceptance criteria, every one annotated `observable: false` and backed by a named cargo test, so the runtime gate did not apply and no sub-agent ran: cargo serialises on the target folder and the benchmark needs an uncontended CPU, so the coordinator ran every concern in sequence and captured evidence per the cli adapter layout.

Thirteen checks ran and thirteen passed: format, lint, the release build, the 65-test suite, two secret scans, the advisory check over the six crates the slice added, debt-marker hygiene, the instrumentation key check, the benchmark compare, the criterion benches, cold start, and the cross-slice regression run over engine-core. The compare sits inside the no-change band on every target: 389 ms against 393, 387 ms CPU against 387, 1.4418 µs against 1.4611, and peak memory up 11.1 percent against a 25 percent tripwire. Twenty-four adversarial probes on the release binary found no crash and no unhandled error; twenty-three refused bad input with one message naming the file and the field, and one found a defect: an explicit `--content-dir` or `SM_CONTENT_DIR` that holds no `attributes.json` was silently ignored and the run played on `./content`. The user triaged it `Fix`; one sub-agent patched the resolver so an explicit location is used exactly, added a regression test that failed before and passed after, and the re-check passed. The fix landed as commit `ae31329`.

Review is next. The top open risk moves from determinism, which held on generated teams and under concurrent runs, to the header: the tick file now carries `match.id` at bytes 44 to 51, so two runs of one seed differ in those eight bytes while every record byte is equal. Engine-core's byte-identity criterion reads literally over the whole file; review decides whether its wording narrows to the record stream. Two low-severity copy findings ride along: the `owner.id` refusal is prefixed "tick file format error", and the loader's `content.loaded` line names `bytes` where the instrument plan named `items`.

## Verification Summary

- Result: `pass`. Every criterion met by the test suite; no criterion is user-observable, so the runtime gate is `not-applicable`.
- Convergence: `converged`. One issue entered the fix loop (ADV-1), one `Fix` decision, one patch accepted, one re-check passed, one regression test added; `fix-rounds-run: 1`.
- Adapter: `cli`, driven directly (`cli-direct`) on the reference machine for augmentation evidence, adversarial probes, and the benchmark compare.
- Sub-agents: none for verification (one cargo command covers every criterion; the benchmark needs the CPU alone). One fix sub-agent at medium effort in a git worktree for ADV-1.
- Benchmark compare mode: recorded in `05c-benchmark.md`; four targets compared, zero regressions, zero improvements, zero tripwires.

## Automated Checks Run

- `cargo fmt --all -- --check`: pass (exit 0; `check-fmt.exit-code`).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass (exit 0, 0 warnings; `check-clippy.stderr.txt`).
- `cargo build --release --workspace`: pass (exit 0; 4,686 ms wall after a clean of the two workspace crates, 0 warnings; binary 2,379,264 bytes; `check-build-release.*`).
- `cargo test --workspace` (debug profile): pass (65 passed, 0 failed, 0 ignored; 23,322 ms wall; `check-test.stdout.txt`). Breakdown: 44 unit tests in `engine`, 5 in `content`, 1 in `determinism`, 1 in `full_match`, 3 in `generator`, 2 in `identity`, 2 in `team_files`, 2 in `validator`, 5 in `cli_args`. After the fix: `cli_args` holds 6.
- `gitleaks git --log-opts="27d21f5..HEAD"`: pass with one false positive (2 commits scanned; rule `generic-api-key` matched the words "key check: `04b-instrument.md`" in `04-plan-data-schemas-generator.md:133`; the "secret" is the file name; `check-gitleaks.report.json`).
- Secret pattern grep over `git diff 27d21f5...HEAD` (api key, secret, password, token, credential assignments): pass (0 matches; `check-secret-grep.txt`).
- Dependency advisory check: pass. `cargo-audit` is not installed; the six crates the slice added to the lockfile (`garde` 0.23.0, `garde_derive` 0.23.0, `compact_str` 0.9.1, `castaway` 0.2.4, `ryu` 1.0.23, `static_assertions` 1.1.0) were checked against the RustSec advisory database tree (933 crate folders, not truncated); none has an advisory folder. `smallvec` 1.16.1 was already in the lockfile at `27d21f5` and is above every patched bound (`>= 1.6.1`). `check-rustsec-packages.txt`.
- `sdlc-debt:` marker hygiene over the slice diff: pass. 0 markers added; the 2 engine-core markers (`sim.rs:221`, `validate.rs:77`) are unchanged. `check-debt-markers.txt`.
- Instrumentation key check (`04b-instrument.md` §2 against live records): pass. `match-stats`: 21 planned keys, 21 present, 1 additive extra (`goals`, carried from engine-core). `run-report`: 22 planned keys, 22 present, 0 extra. `content.hash` is 12 hex characters and equal across both records; `owner.id` is 32 hex characters and equal across both records; `teams` is a two-element array of `{team.id, team.name}`. `instrument-key-check.txt`.
- `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` three times without a reset: pass (exit 0 ×3; medians 389, 389, 389 ms; `bench-drive-{1,2,3}.*`). Details under Augmentation Verification.
- `cargo bench -p engine --bench tick_step`: pass (exit 0; `tick_step` 1.4418 µs, `steering_pass_22` 883.43 ns; `check-criterion.stdout.txt`).
- Cold start `engine-cli --version` five times: pass (36 to 37 ms; engine-core measured 37 to 39 ms; `perf-cold-start-ms.txt`).
- Cross-slice regression over `engine-core`: pass. Every `files-modified` path of engine-core overlaps this slice, so the whole suite is the regression target; `determinism`, `full_match`, and `validator` passed inside `cargo test --workspace`.

## Interactive Verification Results

Automated only — every criterion in `03-slice-data-schemas-generator.md` is annotated `observable: false` and names a cargo test; the plan's `## Verification Strategy` records no environment wall. The release binary was driven for augmentation evidence and adversarial probes (below), not for a criterion.

## Acceptance Criteria Status

- **AC-a** "Given the shipped default attribute schema, When the engine loads it, Then every attribute has a name, group, and 1 to 100 range, and the count is between 30 and 50." kind: `code-only` (annotated `observable: false`). status: met. method: automated. evidence: `the_shipped_attribute_schema_has_thirty_to_fifty_grouped_names` passed; the live run shows `content.loaded kind="attributes" schema_version=1 bytes=1884` (`simulate-42.stderr.txt`); the probe `adv-attributes-29` shows a 29-name schema refused with "attributes: length is lower than 30". evidence-rung: n-a.
- **AC-b** "Given a team data file with a player attribute of 120, When the engine loads it, Then it refuses with a message naming the file, the player, and the attribute; and Given a valid team file, Then it loads." kind: `code-only`. status: met. method: automated. evidence: `an_attribute_of_120_is_refused_naming_file_player_and_attribute` and `a_valid_team_file_loads` passed; the live binary printed "content refused: team team-bad-attribute.json: players: player p-club-00000001-00-03: attribute pace is 120; allowed 1 to 100" and exited 1 (`refused-attribute.*`). evidence-rung: n-a.
- **AC-c** "Given a rule pack with an unknown schema version, When the engine loads it, Then it refuses naming the version; and Given the current version, Then it loads." kind: `code-only`. status: met. method: automated. evidence: `an_unknown_rule_pack_version_is_refused_naming_the_version` and `the_current_rule_pack_version_loads` passed; the live binary printed "content refused: rules rules/default.json: schema_version 99; this build reads 1" and exited 1 (`refused-version.*`). evidence-rung: n-a.
- **AC-d** "Given a seed, When the generator builds a league of 20 clubs, Then every club has 22 players with legal positions, and per-position attribute means fall inside the distribution bounds the tuning file sets." kind: `code-only`. status: met. method: automated. evidence: `twenty_clubs_have_twenty_two_players_with_legal_positions`, `per_position_means_fall_inside_the_tuning_bounds`, and `the_same_seed_gives_the_same_league` passed; the live `generate --seed 9 --clubs 20` wrote 20 files and emitted `generator.league seed=9 clubs=20 players=440 attr.mean=46.94 attr.min=1 attr.max=88` (`generate.*`). evidence-rung: n-a.
- **AC-e** "Given a match record, When it is saved and loaded, Then owner identifier and match identifier round-trip unchanged." kind: `code-only`. status: met. method: automated. evidence: `owner_and_match_ids_round_trip_through_stats_json` and `owner_and_match_ids_round_trip_through_the_tick_header` passed; the live run's printed record and its `matches/000000000000002a-1790070788685/stats.json` carry the same `owner.id` and `match.id`, and the `tickfile.header` line names both (`simulate-42.stats.json`, `simulate-42.stderr.txt`). evidence-rung: n-a.
- **AC-f** "Given the generator output, When `engine-cli simulate --team-a a.json --team-b b.json` runs, Then the simulation completes and the tick count equals the engine-core result." kind: `code-only`. status: met. method: automated. evidence: `generated_teams_simulate_a_full_match_with_the_engine_core_tick_count` passed; the live run on two seed-9 clubs reported `ticks.written` 270000 and a 51,840,080-byte file, equal to the shipped-defaults run and to engine-core (`simulate-generated.*`). evidence-rung: n-a.

evidence: n-a 6. `metric-acceptance-mock-rung`: 0.

## Issues Found

None open. ADV-1 (MED, class `branch-gap`) was found by the adversarial battery, triaged `Fix`, patched, re-checked, and committed; see Verify-Owned Fixes.

## Verify-Owned Fixes

| ID | Type | Triage | Sub-agent outcome | Regression test | Re-check result |
|---|---|---|---|---|---|
| ADV-1 | check-failure (adversarial probe: an explicit `--content-dir` or `SM_CONTENT_DIR` without `attributes.json` fell through to `./content`, exit 0) | Fix | Patched (`Method: as-prescribed`; `ContentDir::resolve` uses an explicit location exactly, `content/README.md` restated) | `crates/engine-cli/tests/cli_args.rs::an_explicit_content_dir_without_attributes_is_not_ignored_for_cwd_content` (failed before the patch, `fix-adv-1.test-before.stdout.txt`; passed after) | Pass (`cargo test -p engine-cli` 6 of 6; `cargo test -p engine --lib data` 9 of 9; clippy clean; both probes now exit 1 naming the path, `adv-content-dir-missing-recheck.*`, `adv-env-content-dir-recheck.*`) |
| FMT-1 | format (rustfmt asked for one struct literal in the ADV-1 patch to be expanded) | Fix (mechanical, no question) | `cargo fmt --all` applied by the coordinator on the sub-agent's worktree (`fix-adv-1.fmt.txt` shows the one hunk) | n-a | Pass (`cargo fmt --all -- --check` exit 0) |

The gate question fired through rung 1 (`AskUserQuestion`); the user chose `Fix`. The fix sub-agent's shell was unavailable in its worktree, so it could not run the four commands it was given; the coordinator ran them on the worktree before accepting the patch and again on the merged tree.

Commit: `ae31329`
Regression tests added: 1

## Augmentation Verification

- **instrument** (`04b-instrument.md`, status ready): pass. Every one of the seven designed signals fired on the live binary: `content.loaded` (five lines per run, one per file, with `kind`, `path`, `schema_version`, `hash`; `simulate-42.stderr.txt`), `content.refused` (with `kind`, `path`, `field`, `reason`, on both the attribute path and the version path; `refused-*.stderr.txt`), `content.hash` (12 hex, equal in `match-stats` and `run-report`), `match-stats.teams` (two objects with `team.id` and `team.name`), `generator.league` (with `seed`, `clubs`, `players`, `attr.mean`, `attr.min`, `attr.max`, `hash`; `generate.stderr.txt`), `identity.owner_created` (once, on the first run into an empty data folder, absent on the next two; `bench-drive-{1,2,3}.stderr.txt`), and `tickfile.header` (with `schema_version=2`, `owner_id`, `match_id`). Dark paths: a silent refusal is covered by the `content.refused` line plus the exit code; a match on unknown content by `content.hash` (`a5c30b85cbe6` for the generated pair against `c33b2b9b247e` for the defaults); generator drift by the three statistics; a regenerated owner by the one-time line. No personal identifier and no absolute path appears in any record. Key drift, LOW, informational: the plan's `content.loaded` design names `items`; the code emits `bytes`. Review reconciles the plan text or the key.
- **benchmark** (`05c-benchmark.md`, mode baseline → complete): pass. Compare mode ran the two recorded commands with the shipped defaults. Full match wall time 389 ms against 393 (−1.0 percent); ticks per second 694,087 against 687,023 (+1.0 percent); CPU 387 ms per match against 387 (0.0 percent, the 426 ms tripwire not reached); peak memory 5.62 MB against 5.06 (+11.1 percent, the 6.33 MB tripwire not reached); tick step 1.4418 µs against 1.4611 (−1.3 percent). Regressions: none. Improvements: none. Tripwires: none. The loader runs once before the warm-up match (`bench.rs:21-27`), so no file read sits on the timed path. The compare was measured on `4a806c8`; the fix commit `ae31329` changes the resolver only, which is outside the timed path.
- **experiment**: deferred to the `experiment-flags` slice per the index; no `04c-experiment.md` exists, so no re-check applies.
- Mock fidelity inventory: `02c-craft.md` does not exist; no inventory applies. Design findings: no `07-design-audit.md` or `07-design-critique.md` exists.

## Security Scan

- CVE scan: `cargo-audit` not installed; manual check of the six added crates against the RustSec advisory database tree. New critical or high advisories: 0. `rand` 0.10.3 stays patched for RUSTSEC-2026-0097.
- Secret detection: `gitleaks` over `27d21f5..HEAD`, 2 commits, one `generic-api-key` hit on prose in a workflow artifact (a file name after the words "key check"), classified false positive. Pattern grep over the diff: no finding.
- SAST: `semgrep` not installed; skipped. Clippy at `-D warnings` is the static floor and is clean.

## Accessibility Gate

Not automatable: the slice ships a headless command line and data files, and no user interface. New WCAG AA violations: 0 (no surface).

## Performance Gate

- Bundle size: skipped; the base branch `main` holds no code, so no delta exists. Absolute artifact: `engine-cli.exe` 2,379,264 bytes (release, with debug info; engine-core measured 1,913,856; the growth is `garde`, `compact_str`, and the data module).
- Build time: 4,686 ms for the two workspace crates from clean against 2,138 ms in engine-core (+119 percent, dependencies cached in both runs). This exceeds the 30 percent WARN line. Cause: the `garde` derive expansions over the four file structs and the 5,895 added lines. Recorded as WARN, informational; the absolute time stays under five seconds.
- Cold start: 36 to 37 ms for `--version` over five runs against 37 to 39 ms in engine-core; no increase.
- The `benchmark` augmentation carries the engine numbers (see Augmentation Verification).

## Cross-Slice Regression

Sibling checked: `engine-core` (`result: pass`). Every path in its `files-modified` overlaps this slice's `files-modified`, so the regression target is the whole suite; `cargo test --workspace` passed all 65 tests, including `two_runs_are_byte_identical`, `ninety_minutes_write_270_000_records`, and `a_seeded_full_match_has_no_violations`. `cross-slice-regressions-found: 0`.

Observation for review, not a regression: engine-core's AC-b reads "the two tick outputs are byte-identical". The test fixes the header and still passes; the live binary now writes `match.id` into header bytes 44 to 51, so two runs of one seed differ in those eight bytes and in no record byte (`probe-concurrent.cmp.txt`; the same result for two runs on generated teams). The shape's AC-18 requires the match identifier in every replay, so the criterion's wording, not the code, is the item to settle.

## Longitudinal Delta

Baseline source: prior evidence run `verify-evidence/engine-core/` (`simulate-42.stdout.txt`, `enumerate.help.txt`). Surface: the `simulate --seed 42` record and the command-line help tree.

- Record (`longitudinal-simulate-42.txt`): `possession.changes` 773 → 785, `ball.idle_ticks` 854 → 1435, `goals` [0, 0] → [0, 1], `ball.max_speed` 27.0707 → 27.0706, `engine.ticks_per_s` 657,635 → 673,360; new keys `content.hash` and `teams`; `owner.id` `local-cli` → a 32-hex identifier. Interpretation: expected change; every player now carries loaded attributes instead of a flat 60, so the seed-42 match plays differently. `validate.violations` stays 0 and `ticks.written` stays 270000.
- Help tree (`longitudinal-help.diff.txt`): added `generate` with `--clubs`, `--out`, `--force`; added global `--content-dir`; added `--team-a` and `--team-b` on `simulate`. No flag removed. Longest help line 80 columns.

## Friction Notes

- The tick file is no longer byte-identical across runs of one seed at the header level (bytes 44 to 51 hold `match.id`); every record byte is identical. See Cross-Slice Regression.
- A corrupt `owner.id` is refused with "tick file format error: owner.id must hold 32 hex characters" (`adv-owner-corrupt.stderr.txt`): the `Format` variant's display prefix names the tick file for an identity file. Class: `ambiguous-copy`. informational, LOW.
- An empty team file is refused as "schema_version: EOF while parsing a value at line 1 column 0" (`adv-team-empty.stderr.txt`): the field is `schema_version` because the version peek is the first read. The message is accurate but the field name can mislead. Class: `ambiguous-copy`. informational, LOW.
- `--team-a` and `--team-b` accept the same file (`adv-same-team-both`): both sides carry `club-00000001-00`. The mirror match runs and validates; no rule forbids it. informational.
- The build hash carries `-dirty` during every workflow run because the cost ledger is hook-appended each turn; unchanged from engine-core.

## Free Exploration Notes

- The generator's `attr.min` is 1 and `attr.max` is 88 for seed 9: the bell curve reaches the clamp floor on outfield goalkeeping (mean 15, spread 5, twelve-draw sum). informational; the `calibration` slice owns the bands.
- `generate` into an existing folder without `--force` refuses at the first existing file and names its absolute path (`adv-overwrite-refused`); with `--force` it overwrites (`adv-overwrite-forced`). informational.
- The same seed and content give byte-identical team files across two `generate` runs into two folders (`adv-rapid-gen.cmp.txt`). informational.
- After a `simulate` run, `SM_DATA_DIR/matches/<match.id>/stats.json` holds the printed record byte for byte (`simulate-42.stats.json`). informational.

## Adversarial Tests

| Test | Result | Finding |
|---|---|---|
| `generate --clubs 0` (`adv-clubs-0`) | pass | "--clubs must be at least 1", exit 1 |
| `generate --clubs 1` (`adv-clubs-1`) | pass | one file, exit 0 |
| Overwrite without `--force` (`adv-overwrite-refused`) | pass | "already exists; pass --force to overwrite", exit 1 |
| Overwrite with `--force` (`adv-overwrite-forced`) | pass | exit 0 |
| `--team-a` missing file (`adv-team-missing`) | pass | "cannot read no-such.json: The system cannot find the file specified. (os error 2)", exit 1, one chain |
| Same file for both teams (`adv-same-team-both`) | pass | runs, exit 0 (informational) |
| `--content-dir` missing folder (`adv-content-dir-missing`) | fail → fixed | fell through to `./content`, exit 0; ADV-1; after the fix "cannot read content folder (tried <path>)", exit 1 |
| `SM_CONTENT_DIR` missing folder (`adv-env-content-dir`) | fail → fixed | same defect and same fix as ADV-1 |
| Flag beats a bad `SM_CONTENT_DIR` (`adv-flag-beats-env`) | pass | exit 0 before and after the fix |
| Team file with 10 players (`adv-team-ten-players`) | pass | "players: length is lower than 11", exit 1 |
| Unknown position code `XX` (`adv-team-unknown-position`) | pass | "line 1 column 258: unknown variant `XX`, expected one of `GK`, …", exit 1 |
| Unknown top-level key (`adv-team-unknown-key`) | pass | "unknown field `extra`, expected one of `schema_version`, `club`, `players`", exit 1 |
| Duplicate shirt (`adv-team-duplicate-shirt`) | pass | "player p-club-00000001-00-02: shirt 1 is already worn", exit 1 |
| Missing attribute (`adv-team-missing-attribute`) | pass | "player p-club-00000001-00-03: attribute pace is missing", exit 1 |
| Kit colour `red` (`adv-team-bad-kit`) | pass | "club.kit.primary: red is not a #RRGGBB colour", exit 1 |
| Attribute 0 (`adv-team-attribute-zero`) | pass | "attribute stamina is 0; allowed 1 to 100", exit 1 |
| Empty player name (`adv-team-empty-name`) | pass | "players[5].name: length is lower than 2", exit 1 |
| Empty file (`adv-team-empty`) | pass | "EOF while parsing a value at line 1 column 0", exit 1 (LOW copy note) |
| Truncated JSON at 500 bytes (`adv-team-truncated`) | pass | "EOF while parsing a string at line 1 column 500", exit 1 |
| Tuning `keeper_catch_chance` 1.5 (`adv-tuning-out-of-bounds`) | pass | "tuning tuning.json: engine.keeper_catch_chance: greater than 1", exit 1 |
| Attribute schema with 29 names (`adv-attributes-29`) | pass | "attributes attributes.json: attributes: length is lower than 30", exit 1 |
| Rapid repeat `generate` into two folders (`adv-rapid-gen-1`, `-2`) | pass | byte-identical files |
| Corrupt `owner.id` (`adv-owner-corrupt`) | pass | "owner.id must hold 32 hex characters", exit 1 (LOW copy note) |
| Unknown rule-pack version on the live binary (`refused-version`) | pass | "schema_version 99; this build reads 1", exit 1 |
| Offline / network failure | n-a | no network dependency |
| Empty submission | n-a | no interactive input surface |

## Failure Mode Probes

| Probe | Result | Finding |
|---|---|---|
| Concurrent session: two `simulate --seed 5 --minutes 10` at once (`probe-concurrent-*`) | pass | both exit 0; record bytes identical, header differs at the `match.id` bytes only; statistics equal apart from `match.id`, `duration_ms`, `engine.ticks_per_s`; two `matches/` folders |
| First run into an empty data folder (`bench-drive-1`) | pass | `owner.id` created once with the `identity.owner_created` line; reused silently on the next two drives |
| Environment override: `SM_CONTENT_DIR` bad, flag good (`adv-flag-beats-env`) | pass | the flag wins; exit 0 |

## Gaps / Unverified Areas

- No criterion is user-observable, so no interactive evidence exists; the slice has no user interface.
- The `bench` command cannot take team files; the compare ran on the shipped defaults by the baseline's own rule.
- Determinism across machines and across builds is not measured (unchanged from engine-core).
- The engine-core AC-b wording against the header bytes is a review item, not a verify failure.

## Freshness Research

Not run: no test failed, the plan is 3 hours old, and the slice touches no external API or remote schema. `ac-staleness-checked: false`.

## Recommendation

Proceed to review. The slice passes every check and every criterion, the fix loop converged in one round with its regression test, and the benchmark compare holds inside every tripwire. Review should settle three small items: the engine-core AC-b wording against the header's `match.id`, the `items` versus `bytes` key on `content.loaded`, and the two LOW copy notes.

## Recommended Next Stage

- **Option A (recommended): `/wf review football-manager-match-engine data-schemas-generator`** — `result: pass`, `convergence: converged`. Compact recommended before review: this verify carried test output, probe output, and a fix sub-agent round.
- **Option D: `/wf handoff football-manager-match-engine data-schemas-generator`** — valid on `result: pass` for a solo project; not recommended because `review-scope: slug-wide` accumulates one ledger and the AC-b wording item wants a reviewer's decision.
- **Option G: `/wf probe football-manager-match-engine`** — a slug-wide runtime sweep after review, if cross-slice behaviour of the content folder and the data folder wants a fresh look.
