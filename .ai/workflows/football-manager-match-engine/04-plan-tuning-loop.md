---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: tuning-loop
status: complete
stage-number: 4
created-at: "2026-09-24T07:09:11Z"
updated-at: "2026-09-24T08:26:23Z"
metric-files-to-touch: 18
metric-step-count: 14
has-blockers: false
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-24T08:26:23Z"
    trigger: answers-returned
    because: "product owner answered Q-P1 (2026-09-24T07:20:00Z): the baseline guard checks the fixtures hash"
    changed: "step 8 identity check on a new fixtures.hash report key; refusal and acceptance tests; schema and docs lines; jobs clamp covers the red-card suite's 4 x matches; Q-P1 moved from Blockers to Assumptions; status complete"
consult-runs: []
tags: [calibration, tooling, realism, performance]
stack-source: confirmed
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-tuning-loop.md
  siblings: [04-plan-realism-bands-v2.md, 04-plan-defending-and-discipline.md, 04-plan-calibration.md, 04-plan-experiment-flags.md]
  implement: 05-implement-tuning-loop.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine tuning-loop"
---

# Plan: Tuning loop

## The Plan

The calibrate command already runs suites in worker processes and checks bands. A run always plays every fixture of each suite it names. The formations suite alone is 55 pairings of 1,000 matches, and a whole seed takes about 76 minutes. A formations fixture gets its match seed from its place in the full list (`fixtures.rs:68-84`, `fixtures.rs:113-115`). So a targeted run can play one pairing and get the same figures as a full run, as long as each fixture keeps its place in the full list. The red-card experiment exists only as a slow test (`crates/engine/tests/defending.rs:165-199`). It needs the test-only scene builder, which a release build never contains (`crates/engine/Cargo.toml` feature `scenario`). So this slice adds one public send-off before kick-off, and the scene builder calls it too.

The plan adds `--pairing`, `--band` and `--baseline` to calibrate, and a `red-card` suite that `--suite all` does not include. It adds a sampling error to every band check and a diff table that marks a change inside two errors as noise. Last, it measures a calibrate build profile, which is kept only if it is faster and gives identical figures. The work is 14 steps over 18 files, with one new report module and one new test file. All of it runs headless on the reference machine, with no wall.

The first version of this plan stopped on one criterion. It refused a baseline on a different content hash, but `content.hash` covers `tuning.json` and every flag state (`crates/engine/src/data/mod.rs:274-278` and `310-321`, `crates/engine/src/sim.rs:91-109`). So the guard would refuse exactly the tuning change the loop exists to measure. The product owner chose the fixtures hash (Q-P1, `po-answers.md` 2026-09-24T07:20:00Z). The run report gets a `fixtures.hash` key over the inputs that decide the fixtures: the attributes, rules and tactics content, the generator block and the default clubs. The guard refuses a different seed, match count or fixtures hash. A different content hash is allowed, and the diff prints it as the change under test. Only step 8, its tests, one schema key and one docs line changed. The plan is now complete with no blockers. It gives `lone-forward` a loop of under 2 minutes for its two pairings and the red-card criterion. The top open risk is the 2-minute budget, which step 12 measures.

## Current State

- **Calibrate parent** (`crates/engine-cli/src/calibrate/mod.rs:39-260`). It resolves flag states, makes the run folder (`:97-102`), picks suites from `--suite` (`:103-108`), and clamps jobs to the largest suite (`:120-130`). Then it plays each arm (`:146-153`) and writes one `run-report`. `play_arm` spawns `jobs` workers per suite (`:356-387`), reads back every planned statistics file (`:399-421`), and builds the figures and checks with `RunBuilder`.
- **Fixtures** (`calibrate/fixtures.rs`). The equal and strength suites play `fixtures(matches)`. The formations suite plays `formation_fixtures(matches, pairings)`, where fixture `i` belongs to pairing `i / matches` and its match seed is `splitmix64(run_seed ^ suite<<32 ^ i)` (`:113-115`). Pairings are `[a, b]` with `a <= b` over the 10 formations of `content/tactics.json`, so there are 55. 4-4-2 is formation 0, so the pairing the slice names is labelled `4-4-2 v 4-4-1-1` (`report/mod.rs:116-118`).
- **Worker** (`calibrate/worker.rs:44-127`). It plays the fixtures where `index % shards == shard`, writes one statistics record and one event file per match, and runs the validator on the team timeline.
- **Report** (`report/mod.rs`). `SuiteFigures` holds means and shares. The only spread it holds is `goals_per_match_sd`, and no band check carries a sampling error (`BandCheck`, `:124-134`). `checks()` makes 15 equal-suite rows, 1 strength row, 3 rows per pairing, and 1 `wall_ms` row per suite (`:390-550`). `compare::compare` already pairs the rows of two sets of checks by suite, band and pairing, and drops `wall_ms` (`report/compare.rs:76-97`).
- **Red-card experiment** (`crates/engine/tests/defending.rs:165-199`). It uses the default clubs, engine seeds 1 to 120 used directly, and `MatchConfig::new` with `red_base`, `yellow_base` and `yellow_aggression_weight` set to 0 on `config.tuning`. The arms are the away keeper (11), centre-back (13) and striker (21), each sent off at kick-off through `Scene::sent_off` (`scenario.rs:91-99`), plus a control with no send-off. Criterion: in each arm, the reduced side does not outscore the full side, and the full side scores at most 1.6 x the control's home mean. `Scene` exists only under the engine's `scenario` feature, which a release build never has.
- **Content hash** (`sim.rs:91-109`). It is SHA-256 over `Content.digest` and the two team files. `Content.digest` folds in the digests of the attributes, tuning, rules and tactics files, and the list of flags that are on whenever a flag state differs from the file (`data/mod.rs:274-278`, `310-321`). The generated leagues read `content.tuning.generator` (`data/generator.rs:15`). `Content` keeps only the combined digest, not one per file (`data/mod.rs:216-230`). `AttributeSchema`, `RulePack`, `TacticsSchema`, `GeneratorTuning` and `TeamFile` all derive `Serialize`, and `hex12` gives the 12-character form the report uses (`data/mod.rs:210-212`). In the calibrate report, `content.hash` comes from the first arm's first record (`calibrate/mod.rs:177`), and a paired run's top-level figures describe its off arm (`report/mod.rs:671-676`).
- **Run-report schema** (`schemas/observability/run-report.schema.json`). `additionalProperties: true` at the top. A band item's `suite` is an enum of `equal`, `strength` and `formations`, and `calib.suites` values require the full `SuiteFigures` key set. `schema.version` is the constant `"1"`.
- **Build.** The workspace `[profile.release]` sets only `debug = 1` (`Cargo.toml`). There is no LTO, and the codegen-units value is the default.
- **Reference machine.** This machine is the Windows 11 reference machine (`02-shape.md` NFR-9), with 8 logical cores (`nproc` = 8). The baseline rate is 12.5 matches per second (`03-slice-tuning-loop.md`).
- **Tests.** `tests/calibrate.rs` (smoke run of every suite at 2 matches, pruning, formations), `tests/calibrate_pair.rs`, `tests/schemas.rs` (boon validation), and `tests/docs.rs` (every flag named in `cli.md`; every guide command real).

## Simplicity Ladder

- Targeted selection → rung 3 reuse. `fixtures::formation_fixtures` and `RunCtx::planned` are reused with a filter over the full list, so the index and the seed stay the same. Match quality is high. The change is backward-compatible: with no selection the list is unchanged.
- Pairing of baseline and new rows → rung 3 reuse. `report/compare.rs` → `compare()` already matches rows by suite, band and pairing and leaves out `wall_ms`. It is reused as-is. Only the error and the noise mark are new.
- Sampling error → rung 4 new code. There is no statistics crate in the workspace, and `std` has no standard-error function. The formulas are a few lines each: sd/sqrt(n), sqrt(p(1-p)/n), and the ratio-estimator error. Rung 1 gives only `f64` arithmetic, and rung 3 gives the existing `mean` and `sd` helpers in `report/mod.rs:580-596`, which are reused.
- Send-off before kick-off in a release build → rung 4 new code, a thin public wrapper around `discipline::send_off`. Rung 3 (`Scene::sent_off`) cannot hold, because the `scenario` feature must stay out of release builds (`crates/engine/Cargo.toml` comment). `Scene::sent_off` is changed to call the new function, so there is one code path.
- Fixtures hash → rung 3 reuse. `sha2::Sha256` and `engine::data::hex12()` (`data/mod.rs:210-212`) already make `content.hash` (`sim.rs:89-103`). They are reused as-is over `serde_json` bytes of the parsed values. Match quality is high. The change is additive: `content.hash` is unchanged.
- Baseline file reading → rung 3 reuse. The workspace already has `serde_json`, and the report fields read back are the ones `CalibrationReport` writes.
- Faster build → rung 2 native platform. Cargo custom profiles (`inherits`, `lto`, `codegen-units`) and `-C target-cpu=native` through `RUSTFLAGS`. No new code.

## Applied Learnings

No applicable learnings found (`.ai/solutions/INDEX.md` does not exist, and `.ai/sdlc-config.json` sets no global directory).

Repeat-deferral tripwire: the only open runtime-evidence deferrals in `00-index.md` belong to viewer and distribution slices (a human reading on the reference laptop, and macOS). This slice's verification names no screen, device or operating system beyond the reference machine. The tripwire does not fire.

## Likely Files / Areas to Touch

- `crates/engine/src/sim.rs`: `Simulation::send_off_before_kickoff(i)`, public.
- `crates/engine/src/scenario.rs`: `Scene::sent_off` delegates to it.
- `crates/engine/tests/defending.rs`: a fast parity test of the two paths; the slow test prints 4 decimals.
- `crates/engine-cli/src/cli.rs`: `SuiteArg::RedCard`, `--pairing`, `--band`, `--baseline`, and a hidden worker pairing list.
- `crates/engine-cli/src/calibrate/mod.rs`: selection, the fixtures hash, the baseline check before the run folder, red-card orchestration, the diff output, and the new report keys.
- `crates/engine-cli/src/calibrate/fixtures.rs`: selected formation fixtures with full-list indices, and the red-card fixtures.
- `crates/engine-cli/src/calibrate/worker.rs`: selected pairings, and the red-card match.
- `crates/engine-cli/src/report/mod.rs`: `Suite::RedCard`, `BandCheck.se`, `RedCardFigures`, and the new report fields.
- `crates/engine-cli/src/report/baseline.rs` (new): the guard, the diff rows, the noise mark and the table.
- `crates/engine-cli/tests/calibrate_targeted.rs` (new), `tests/calibrate.rs`, `tests/schemas.rs` and `tests/docs.rs`.
- `schemas/observability/run-report.schema.json` and `.ai/observability.md`: additive keys.
- `Cargo.toml`: `[profile.calibrate]`, only if the measurement keeps it.
- `docs/reference/cli.md` and `docs/how-to/calibration.md` (new).

## Proposed Change Strategy

1. **Selection keeps the full-list index.** A selected pairing plays the fixtures `p x matches .. (p + 1) x matches - 1` of the full formations list, so its match seeds, clubs and home sides are the ones a full run uses. The builder is planned with only the selected pairings, and the read-back loop reads only their statistics files. This is how the agreement criterion holds by construction, and a test checks it.
2. **`--band` narrows the suites.** A band belongs to the suites that check it (`goals_per_match`, `ten_plus_goals_share` and `goalless_share` belong to equal and formations; the other equal-suite bands to equal only; `stronger_team_win_rate` to strength; the red-card rows to red-card). The run plays the intersection of `--suite` and the suites of the named bands. The report and the diff show only the named bands.
3. **The red-card suite runs outside `--suite all`.** `Suite::ALL` is not changed, so a full run's time and figures are the same as before. The suite plays 4 arms x `--matches` fixtures on engine seeds `--seed` to `--seed + matches - 1`, used directly as in the slow test. So `calibrate --suite red-card --seed 1 --matches 120` is the slow test's experiment. The cards-off values are set on the match configuration, not in the content, so the content hash is unchanged.
4. **Sampling error on every row.** Each `BandCheck` carries `se`, and the diff's error is `sqrt(se_baseline^2 + se_new^2)`. The two runs are treated as independent. This is conservative for two runs on the same fixtures, and it needs no per-match data from the baseline's folder.
5. **The baseline is checked before any work, on the fixtures.** The report is read and checked before the run folder is made and before any worker starts. The identity is the seed, the match count and the fixtures hash, as the product owner chose (Q-P1). A different content hash is allowed and shown in the diff header. A refusal is an error (exit code 1, `main.rs:58-64`), and its message names each difference.
6. **The profile is measured, not assumed.** The profile is kept only if the rate rises and every figure is identical per seed. A native CPU target is never committed to `.cargo/config.toml`, because the distribution build ships to other CPUs. If native is the faster option, it stays a documented local `RUSTFLAGS` option in the how-to.

No NFR is the reason for a mechanism choice, so there is no charter ranking to quote.

## Step-by-Step Plan

1. **Public send-off before kick-off.** Add `Simulation::send_off_before_kickoff(i)`: drop the carrier if it is `i`, call `discipline::send_off`, set `ai[team].due = true`, and set `timeline = vec![(tick, teams.clone())]`. Make `Scene::sent_off` call it. Add a test in `tests/defending.rs`: on seeds 1 to 3, the goals of the Scene path and of the public path on a fresh `Simulation` are equal. Run `cargo test -p engine --all-features --test defending`. Also check that a match with the send-off reads 0 validator violations. If it does not, fix the timeline in the engine function; do not skip the validator.
2. **Suite and sampling error.** Add `Suite::RedCard` (code `red-card`, number 3), and keep it out of `Suite::ALL`. Add `se: f64` to `BandCheck`. Compute it in `RunBuilder::checks` from the records it already holds:
   - means per match: sd/sqrt(n);
   - means per team: the per-match mean of the two teams, then sd/sqrt(n);
   - shares: sqrt(p(1-p)/n);
   - `shots_on_target_share`, `goals_per_xg` and `pass_accuracy_pct`: the ratio-estimator error sqrt(sum((y - R x)^2) / (n (n - 1))) / mean(x);
   - `stronger_team_win_rate`: the binomial error;
   - pairing rows: the same formulas over the pairing's matches;
   - `wall_ms`: 0.

   Unit tests on small hand-made record sets with known errors. Existing tests keep their values.
3. **Fixtures for a selection and for the red-card suite.** `formation_fixtures_for(matches, pairings, selected: &[usize])` returns the full-list fixtures of the selected pairings, in order. `red_card_fixtures(seed, matches)` returns 4 x matches entries of (index, arm, engine seed). Unit tests: the selected pairing's fixtures equal the same slice of `formation_fixtures`, and the red-card seeds are `seed..seed + matches`.
4. **Options.** In `cli.rs`, add `--pairing NAME` (repeatable; `A v B` in either order), `--band NAME` (repeatable), `--baseline FILE`, the `red-card` value of `--suite`, and a hidden `--pairings LIST` for workers. Keep help lines within 80 columns (the existing help test). Refuse an unknown pairing or band with a message that lists the valid names. Refuse `--pairing` without the formations suite in the selection, and refuse `--baseline` with `--pair`, each with a message.
5. **Selection and fixtures hash in the parent.** In `calibrate/mod.rs`, resolve the suites and the pairings, set `RunCtx.pairings` to the selected names with their full-list numbers, and pass them to workers. Clamp `jobs` to the fixtures of the largest selected suite: `--matches` for equal and strength, `--matches` x selected pairings for formations, and 4 x `--matches` for red-card. Plan the builder with the selected pairings only, and filter the checks to the named bands. Write `calib.selection` (`suites`, `pairings`, `bands`) in the report.
   - Add `fixtures_hash(content, teams)`: SHA-256, in this order, over `serde_json` bytes of `content.attributes`, `content.rules`, `content.tactics` and `content.tuning.generator` (after the flag states, so a flag that changes the generator changes the fixtures), then the two default team files as `crate::content::load` returns them. Print it with `hex12`. Use the first arm's content, because a paired report's top-level figures describe the off arm.
   - Write it as the top-level report key `fixtures.hash`, beside `content.hash`.
   - Unit tests: the hash is stable for the same content; it changes when a tactics formation or a generator value changes; it does not change when a non-generator tuning value or a flag without a generator override changes.
6. **Worker.** The worker filters `planned()` to the selected pairings. For `Suite::RedCard`, it loads the default clubs (`TEAM_A_FILE`, `TEAM_B_FILE`), builds `MatchConfig::new(engine_seed, minutes, ..)`, sets the three card values to 0, builds the `Simulation`, calls `send_off_before_kickoff` for the arm's player (none for the control), runs the match with the validator, and writes the statistics record as now.
7. **Red-card figures.** In the parent, fold the red-card records into `RedCardFigures`: the control's home and away mean goals; for each arm, the full-side and reduced-side mean goals (the home side is the full side); the limit 1.6 x control home; and pass. Use the same arithmetic as `mean_goals` in the slow test, rounded to 4 decimals. Add two band rows per arm with their `se`:
   - `reduced_minus_full`: the mean of per-match (reduced - full), hi 0;
   - `full_over_control`: full / control home, hi 1.6, with its delta-method error.

   Write `calib.red_card` in the report. Change the slow test to print each figure to 4 decimals.
8. **Baseline and diff** (`report/baseline.rs`). Read the file. Refuse, naming each difference as `seed 7 differs from the baseline's 42`, when:
   - the baseline is not a calibrate `run-report`;
   - it holds a band row without `se` ("the baseline was made before sampling errors; make a new one");
   - it has no `fixtures.hash` ("the baseline was made before fixtures hashes; make a new one");
   - its seed, `calib.matches` or `fixtures.hash` differs (for example `fixtures hash 3f2a9c01b7de differs from the baseline's 0c44e1a9f2b3`).

   A different `content.hash` is not refused. The diff table's header line prints the baseline's and the run's content hashes, and says `content changed` or `content unchanged`.

   Build the rows with `compare::compare(baseline, new)`. Add `se_change = sqrt(se_b^2 + se_n^2)` and `noise = |change| <= 2 x se_change`. Keep only the rows both runs judged. Print a fixed-width table on standard error with these columns: suite, band, pairing, baseline, new, change, error, `noise` or `change`, and the new verdict (`pass` or `miss`). Write `calib.baseline` (path, `run.id`, seed, `content.hash`) and `calib.diff` in the report. The exit code rule is unchanged.
9. **Contract.** Make additive edits to `run-report.schema.json`: `red-card` joins the band `suite` enum; band items gain `se` (number, minimum 0); a new optional top-level string `fixtures.hash` (12 lowercase hex characters, as `content.hash`); and there are new optional objects `calib.selection`, `calib.red_card`, `calib.baseline` and `calib.diff`. Keep `schema.version` at `"1"`. Update the run-report paragraph of `.ai/observability.md` in the same change.
10. **Tests** (`tests/calibrate_targeted.rs`, `tests/calibrate.rs`, `tests/schemas.rs`), all at small match counts so they stay in the default test run:
    - a `--suite formations --pairing "4-4-1-1 v 4-4-2" --matches 2` run holds one `calib.formations` entry and exactly 2 statistics files;
    - that entry and a targeted `--suite equal` run equal the same entries of a full `--suite all --matches 2` run on the same seed;
    - a baseline with another seed, one with another match count, one with another `fixtures.hash` (a run with `--content-dir` on a copied content folder with one tactics formation edited, the pattern `tests/calibrate_pair.rs:56` already uses), and one with no `fixtures.hash` each exit non-zero, write no `stats/` file, and name the difference;
    - a baseline with the same fixtures and another content hash (a run with a `--flag` whose overrides leave the generator block alone) is accepted, and the diff header names both content hashes and `content changed`;
    - a rerun against its own report marks every row as noise with a change of 0;
    - a `--suite red-card --matches 2` report has four arms, six red-card rows and a pass value;
    - both reports validate against the schema.
11. **Docs.** In `docs/reference/cli.md`, the calibrate section names every new flag (the flag test enforces this), the red-card suite, the diff columns, the refusal messages, and the rule that a baseline must share the seed, the match count and the fixtures hash while a changed content hash is the change under test. The new `docs/how-to/calibration.md` covers these steps: make a baseline, run a targeted pairing, suite or band, read the diff, run the red-card experiment, and run the full suites (five seeds for equal and strength, one for formations) before a gate. Add the how-to to both lists in `tests/docs.rs`.
12. **Measure the loop on the reference machine** (release build). Each of these three runs must finish, diff included, in under 2 minutes. Record the wall time and the matches per second of each:
    - make a baseline with `--suite all --seed 42 --matches 1000`, or use a full report already made after step 8 lands;
    - `calibrate --suite formations --pairing "4-4-1-1 v 4-4-2" --seed 42 --matches 1000 --baseline <that report>`;
    - `calibrate --suite equal --seed 42 --matches 1000 --baseline <that report>`;
    - `calibrate --suite red-card --seed 1 --matches 120 --baseline <an earlier red-card report>`.

    Compare the targeted pairing's figures with the same pairing in the full report (agreement). Compare the red-card figures with the slow test's printed figures.
13. **Build profile experiment.** Build three binaries into separate target folders:
    - (a) the current `--release`;
    - (b) a `calibrate` profile that inherits release with `lto = "fat"` and `codegen-units = 1`;
    - (c) the same profile with `RUSTFLAGS="-C target-cpu=native"`.

    Run `--suite formations --pairing "4-4-1-1 v 4-4-2" --seed 42 --matches 1000` with each binary and record the matches per second. A candidate is identical when the diff against run (a) shows a change of exactly 0 on every row and the `calib.suites`, `calib.formations` and `calib.red_card` objects are equal (checked with `python -c` over the two reports). Keep the fastest identical candidate, and only if its rate is higher than (a). If (c) wins, commit only (b)'s profile, and document (c) as a local option in the how-to. If none is faster, record the measurement and change nothing.
14. **Gates.** Run `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features`. The per-tick benchmark is not re-baselined, because no per-tick code changes. Step 1 touches only kick-off setup.

## Verification Strategy

Every criterion is a behaviour of the calibrate command on the reference machine. The rung is headless runs of the real release binary (`stack.platforms: [cli]`, `stack.testing: [cargo-test]`), plus the fast `cargo test` versions of the same runs. A unit test alone never counts for a runtime criterion.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| A targeted run is fast | `engine-cli calibrate` release runs from step 12, wall time from `calib.wall_ms.total` and the shell clock (headless CLI) | 8-core Windows 11 reference machine: yes (this machine, `nproc` = 8) | none beyond steps 4-8 | 500-match runs to check scaling → pre-registered deferral (not expected: no wall) |
| A targeted run plays only its selection | `calibrate --suite formations --pairing "4-4-1-1 v 4-4-2" --matches 1000`: count `stats/*.json` and read `calib.formations` (headless CLI) | yes | `calib.selection` in the report (step 5) | the `--matches 2` test in `calibrate_targeted.rs` → pre-registered deferral |
| A targeted run agrees with the full run | the targeted pairing and suite compared with a full `--suite all` report on seed 42 (headless CLI) | yes; a full seed takes about 76 minutes | full-list indices (step 3) | the `--matches 2` agreement test → pre-registered deferral |
| The diff separates change from noise | `--baseline` runs from step 12, and a rerun against its own report (headless CLI) | yes | `BandCheck.se`, `report/baseline.rs` (steps 2, 8) | the unit tests of the noise rule → pre-registered deferral |
| A wrong baseline is refused (the product owner's Q-P1 answer: a baseline with different fixtures is refused) | release runs with a baseline that differs in seed, match count and fixtures hash: exit code, message, empty `stats/`; plus one run whose baseline differs only in content hash, which must be accepted with both hashes in the diff header (headless CLI) | yes | `fixtures.hash` in the report (step 5) and the guard (step 8) | the refusal and acceptance tests in `calibrate_targeted.rs` → pre-registered deferral |
| The red-card experiment is a suite | `calibrate --suite red-card --seed 1 --matches 120` against the slow test `a_sending_off_gives_no_advantage` printing 4 decimals (headless CLI + `cargo test --release -p engine --all-features -- --ignored`) | yes | `send_off_before_kickoff`, the red-card worker and figures (steps 1, 6, 7) | the parity test on 3 seeds → pre-registered deferral |
| A faster profile changes no result | three builds of step 13, diff at change 0, and object equality (headless CLI) | yes; the MSVC toolchain links LTO builds on this machine (the release build already links) | none | fat LTO only, without the native target → pre-registered deferral |

No criterion depends on credentials, a device, an external service, an inbound callback or missing infrastructure. No wall is named, so no `constraint-resolution:` line is needed. Q-P1 was a product decision, not an environment wall, and it is answered.

## Test / Verification Plan

### Automated checks

- Lint and type check: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Unit: the sampling-error formulas and the noise rule (`report/mod.rs`, `report/baseline.rs`); selected and red-card fixtures (`fixtures.rs`).
- Integration (fast): `tests/calibrate_targeted.rs`; the updated `calibrate.rs`, `schemas.rs` and `docs.rs`; the parity test in `crates/engine/tests/defending.rs`; the existing `calibrate_pair.rs` and `cli_args.rs` unchanged and passing.
- Slow runs (release): `a_sending_off_gives_no_advantage` (prints 4 decimals); the step 12 and step 13 runs.

### Interactive verification (human-in-the-loop)

Automated only. The slice is a command-line tool with no page change. Every criterion is read from a run report, standard error, an exit code or a file count.

## Risks / Watchouts

- **The fixtures hash misses an input** (medium). If an input that decides the fixtures is left out of the hash, a baseline on other fixtures is accepted and the diff compares different matches. The hash covers every input the fixtures read (attributes, rules, tactics, the effective generator block, the default clubs). The step 5 unit tests and the step 10 refusal test (an edited tactics formation) check it.
- **Targeted and full runs drift apart** (medium). Full-list indices, plus the agreement test.
- **A native CPU target moves seeded results** (medium). Any figure that differs rejects the candidate. Native is never committed.
- **The 2-minute budget** (low). About 80 seconds of play at 12.5 matches per second, plus the 6-match single-thread figure (about 4 seconds) and league generation. The single-thread figure stays, because the paired run's guardrail reads it.
- **The red-card suite and the validator** (low). A player sent off with no card event might read as a violation. Step 1 resets the timeline and checks for 0 violations before the suite is built on it.
- **A noisy share band** (low). With few events, the error of a share band is wide. The diff prints it, and a tuning slice still confirms a gain on the full gate (RIM-11).

## Dependencies on Other Slices

- `realism-bands-v2` (complete): the bands file, the formations suite, the pairing figures and `compare`.
- `defending-and-discipline` (complete, verified): the red-card experiment in `crates/engine/tests/defending.rs` at `f7fe35b`, and `Scene::sent_off`, which lays the team out again.
- `lone-forward` (next, `depends-on: [tuning-loop, defending-and-discipline]`): the first consumer, for 4-4-1-1 and 3-4-3 against 4-4-2 and the red-card criterion.
- Files that earlier verified slices created: `calibration`, `experiment-flags`, `probe-engine-core` and `realism-bands-v2` (`calibrate/*`, `report/*`, `cli.rs`, the run-report schema), and `defending-and-discipline` (`scenario.rs`, `defending.rs`). All are verified, so the changes run in sequence.

## Assumptions

Autonomous run: no product owner was present. The discovery questions the interview would have asked are answered here in the direction that meets the criteria at the least cost and the smallest change. One question could not be answered this way: Q-P1, the identity a baseline must share with the run. The product owner answered it (below), and A22 to A26 build that answer.

- **Q-P1 (answered by the product owner, 2026-09-24T07:20:00Z, `po-answers.md`): the fixtures hash.** A baseline is refused only when its fixtures differ from the run: the seed, the match count, the attributes, rules and tactics files, the generator block and the default clubs. A changed content hash (tuning values or flag states) is not refused, and the diff prints it as the change under test. The criterion "A wrong baseline is refused" is re-stated here as "a baseline with different fixtures is refused". This is a product-owner decision, not an autonomous one.

- A1 (class: implementation-detail): a selected pairing keeps its full-list fixture indices. So its match seeds, clubs and home sides equal a full run's. This is the only design that meets the agreement criterion without a second seed scheme.
- A2 (class: implementation-detail): `--pairing` accepts `A v B` in either order and matches the stored label `min v max`. The slice's example `4-4-1-1 v 4-4-2` is stored as `4-4-2 v 4-4-1-1`.
- A3 (class: implementation-detail): `--band` narrows the suites to those that check the named bands, intersected with `--suite`. The report and the diff then show only those bands. The slice says a run "plays only the fixtures that selection needs".
- A4 (class: implementation-detail): the `red-card` suite is not part of `--suite all`. A full run's time and figures stay as they are, and the gate rule (Q-I1) stays unchanged.
- A5 (class: implementation-detail; ac: "The red-card experiment is a suite"; classification: runtime-evidence): the red-card suite uses engine seeds `--seed .. --seed + --matches - 1` directly, the default clubs, and the three card values at 0 on the match configuration. That is the slow test's setup, so `--seed 1 --matches 120` reproduces it. Figures are compared at 4 decimals.
- A6 (class: implementation-detail): the send-off before kick-off becomes a public engine function. The `scenario` feature stays out of release builds, as its `Cargo.toml` comment requires. The engine crate is internal to the workspace, and the product's data contract (records and stream) does not change.
- A7 (class: implementation-detail): the red-card criterion appears as two band rows per arm, `reduced_minus_full` (hi 0) and `full_over_control` (hi 1.6). It also appears as `calib.red_card` with the raw figures. The limits are the slow test's, unchanged.
- A8 (class: implementation-detail): the diff treats the baseline and the new run as independent: error = sqrt(se_b^2 + se_n^2). This is conservative on shared fixtures. It needs no per-match data from the baseline folder, which Q-X6 does not require anyone to keep.
- A9 (class: implementation-detail): a band row's `se` is computed from the records the builder already holds. It uses sd/sqrt(n) for means, the binomial error for shares and the win rate, and the ratio-estimator error for the three ratio bands.
- A10 (class: implementation-detail): the report keys are additive and optional (`se`, `fixtures.hash`, `calib.selection`, `calib.red_card`, `calib.baseline`, `calib.diff`, `red-card` in the suite enum). `schema.version` stays `"1"`. The slice's scope puts the red-card figures and the diff in the run report. Earlier run folders are not a compatibility target (steer.md, Q-2 = C).
- A11 (class: implementation-detail): a baseline without `se` on its rows is refused with a message to make a new one. The refusal criterion requires a baseline to be refused before play, and a diff needs the baseline's errors.
- A12 (class: implementation-detail): `--baseline` together with `--pair` is refused. The paired run already has its own two-arm comparison, and combining the two is not in scope.
- A13 (class: implementation-detail): a baseline refusal happens before the run folder is made, and it exits with code 1 through the existing error path. The criterion says "exits non-zero before any match is played".
- A14 (class: implementation-detail; ac: "A faster profile changes no result"; classification: runtime-evidence): three candidates are measured: release, fat LTO with one codegen unit, and that plus a native CPU target. Only a Cargo profile can be committed. Cargo profiles cannot set `rustflags` or `target-cpu` (Cargo Book, Profiles). A native target is never written to `.cargo/config.toml`, because the distribution build ships to other CPUs.
- A15 (class: implementation-detail; ac: "A targeted run is fast"; classification: runtime-evidence): the rate is measured on this machine, which is the reference machine (Windows 11, 8 logical cores). The single-thread benchmark figure stays in a targeted run.
- A16 (class: implementation-detail; ac: "A targeted run plays only its selection"; classification: runtime-evidence): counted from the `stats/` files and `calib.formations` of the release run.
- A17 (class: implementation-detail; ac: "A targeted run agrees with the full run"; classification: runtime-evidence): compared with a full seed-42 run. The fast `--matches 2` test is the regression guard.
- A18 (class: implementation-detail; ac: "The diff separates change from noise"; classification: runtime-evidence): the `--baseline` runs of step 12, plus a rerun against its own report.
- A19 (class: implementation-detail): research ran inline in this session. The session has no sub-agent tool, and the four research charters (affected code, test infrastructure, web and dependencies, reuse) are covered in Current State, Simplicity Ladder and Freshness Research, each with file:line citations.
- A20 (class: implementation-detail): the second-opinion consult is not run, although `unknowns-present` and `appetite-medium-or-larger` hold. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`), as in earlier plans.
- A21 (class: implementation-detail): augmentations. None is authored. `02-shape.md` lists no augmentation for this slice. The per-tick benchmark is unaffected, because step 1 is kick-off setup only.
- A22 (class: implementation-detail; ac: "A wrong baseline is refused"; classification: build-capability): the fixtures hash is SHA-256 over the `serde_json` bytes of the parsed attributes, rules, tactics and generator values, then the two default team files, printed with `hex12`. Parsed values are hashed, not raw file bytes, so a whitespace edit does not refuse a baseline. `Content` keeps no per-file digests (`data/mod.rs:216-230`), and every type derives `Serialize`. The inputs are exactly the list in the product owner's answer, with nothing added or removed.
- A23 (class: implementation-detail): the generator block is hashed after the flag states are applied. A flag that overrides a generator value changes the generated clubs, so it changes the fixtures. A flag that leaves the generator alone changes only the content hash. This follows the answer's rule that fixtures, not flags, decide a refusal.
- A24 (class: implementation-detail): a baseline without `fixtures.hash` is refused, with a message to make a new one, in the same way as a baseline without `se` (A11). Every baseline that the diff can read is made after this slice lands in any case.
- A25 (class: implementation-detail): `fixtures.hash` is computed from the first arm's content. A paired report's top-level figures describe the off arm (`report/mod.rs:671-676`), so the hash describes the same figures. `--baseline` with `--pair` stays refused (A12).
- A26 (class: implementation-detail): the jobs clamp counts the red-card suite as 4 x `--matches` fixtures. The existing clamp (`calibrate/mod.rs:120-130`) counts only formations and would give a red-card run too few workers. Auto-review found this gap.
- A27 (class: implementation-detail): this re-run is the auto-review of the existing plan, done inline. The session has no sub-agent tool (as A19). The review found the code unchanged since the plan: the only commit after it, `2b62cd3`, touches workflow records only, and the working tree has no change under `crates/`, `schemas/` or `Cargo.toml`. The cited ranges were re-read (`fixtures.rs:113-115`, `defending.rs:165-167`, `scenario.rs:91-99`, `compare.rs:76-80`, `main.rs:56-64`, `data/mod.rs:274-321`, `sim.rs:89-103`) and they hold. Issues found: 2, the Q-P1 answer to fold in and the red-card jobs clamp (A26).

## Blockers

None. Q-P1 is answered (see Assumptions).

## Freshness Research

- Cargo profiles (Cargo Book, "Profiles", doc.rust-lang.org/cargo/reference/profiles.html, read this session): custom profiles with `inherits` are supported, and a profile may set `lto` (`"fat"`, `"thin"`, `true` or `false`) and `codegen-units`. `rustflags` and `target-cpu` are not profile keys. They are set through `.cargo/config.toml`, `RUSTFLAGS` or the command line. This is why the native target stays out of the committed profile.
- No dependency is added or upgraded. The slice uses `serde_json` (reading the baseline), `clap` (the new options) and `boon` (tests only), all already in the workspace. The statistics are `f64` arithmetic on the existing `mean` and `sd` helpers.
- Float results under a native target: the engine uses `glam` 0.33 `f64` types and `std` float functions. Whether a native target changes a seeded figure is measured in step 13, not assumed. `04-plan-engine-core.md` § Freshness Research records the platform transcendental-function caveat that makes a measurement necessary.

## Recommended Next Stage

- **Option A (default): Implement** → `/wf implement football-manager-match-engine tuning-loop`. The plan is complete with no blockers. Compact the session first; the artifacts carry the context.
- **Option C: Revisit slice** → `/wf slice football-manager-match-engine`, only if the product owner wants the slice's refusal criterion reworded to the fixtures wording in the slice definition itself. The plan already re-states it, so this is optional.
