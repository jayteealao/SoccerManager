---
schema: sdlc/v1
type: augmentation
augmentation-type: experiment
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
experiment-type: feature-flag
hypothesis: "A candidate decision or shot model behind a tuning-file flag moves the realism bands toward their centres over 1000 paired matches without breaking a guardrail."
split: "paired: every fixture played by both arms on the same match seed, 1000 matches per suite per arm"
flag-name: "flags.<candidate> in content/tuning.json (declared with the candidate model; none shipped)"
flag-framework: "tuning-file flags block (no flag framework detected)"
requires-instrument: true
status: ready
created-at: "2026-09-22T22:27:20Z"
slice: experiment-flags
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-experiment-flags.md
  instrument: 04b-instrument.md
---

# Experiment: Paired Calibration of Competing Models

## The Experiment

The shape selected an experiment augmentation with a fixed hypothesis: a candidate decision or shot model changes the realism statistics. The mechanism is a feature flag in the tuning file, evaluated by paired calibration runs, with the AC-3 and AC-4 bands as the metrics and "flag off" as the rollback (`02-shape.md`, Augmentation Plan). No candidate model exists yet. The experiment is therefore a protocol that every candidate follows, and the plan for this slice builds the switch and the comparison. The game has no players in the experiment. The "users" of a treatment are simulated matches, and the split is by fixture, not by person.

Every fixture is played twice: once by each arm, on the same match seed. The shape's names are used: 1000 matches per suite and two suites, equal strength and a 15 percent strength boost. The decision rule has three parts, in order. First, any on-arm guardrail breach rejects the candidate. Then the arm that passes more bands wins. On equal passes, the arm closer to the band centres by more than 0.05 wins. Otherwise the result is "no difference", and the candidate is removed. A paired run is one command, `engine-cli calibrate --pair <flag>`, and it writes one report.

When this lands, a model change is compared on the bands and not argued over. The loser is removed by a written checklist. The top open risk is that a candidate is kept on after an inconclusive run. A "no difference" verdict therefore removes the candidate, so no flag stays in the file without a result.

## 1. Hypothesis

> We believe that **a candidate decision or shot model, switched on by a flag in `content/tuning.json`** will **move the realism band values toward their band centres** for **simulated matches between generated clubs** compared to **the current model with the shipped tuning values**. We'll know this worked when **the on arm passes more of the four bands, or passes as many and sits closer to the band centres by more than 0.05 of summed normalized distance,** without **a dark path, a validator violation, a stronger-team win rate at or below 0.50, or a single-thread match time more than 10 percent slower** in the on arm.

Null hypothesis: the candidate model does not change the band values beyond the margin. Then the extra model is cost without benefit, the verdict is `no-difference`, and the candidate is removed.

## 2. Experiment design

| Dimension | Value |
|---|---|
| **Type** | feature-flag |
| **Control** | Flag off: the current model and the shipped tuning values. The run writes to `runs/<run.id>/arms/off/`. |
| **Treatment** | Flag on: the candidate model, gated by a code flag in `CODE_FLAGS`, or the candidate values, given as overrides. The run writes to `runs/<run.id>/arms/on/`. |
| **Split dimension** | Fixture index. Both arms play every fixture on the same match seed, so the design is paired, not randomized. |
| **Split ratio** | 50/50 by run count: each arm plays every fixture once. A candidate reaches no player until it wins, so no gradual rollout is needed. |
| **Exclusions** | None. Human-managed matches are not part of calibration: both sides are managed by the computer. |
| **Flag name** | `flags.<candidate>`, in lower snake case, at most 40 characters, declared with its model. None ships with the plan for this slice. |
| **Flag default** | `off` |

## 3. Metrics

**Primary metric:** `calib.compare` — one row per suite and band, with the off value, the on value, and both pass marks. The four bands come from `content/realism-bands.json`: goals per match 2.4 to 3.2, shots per team 8 to 16, possession 35 to 65 percent per side, and the stronger team's win rate above 0.50. These are the shape's AC-3 and AC-4 criteria, so the metric is the outcome, not a proxy for it.

**Secondary metrics:**
- `calib.suites.*.goals_sd`: the spread of goals per match. A model that fixes the mean by flattening the spread shows it here.
- `stats.xg` against `stats.goals`: whether a shot-model change moves chance quality or only conversion.
- `calib.wall_ms` per arm: the cost of the candidate at 1000 matches per suite.

**Guardrail metrics (must not regress):**
- `darkpath.match_without_stats` in the on arm: threshold 0. Any value rejects the candidate.
- `darkpath.change_never_applied` in the on arm: threshold 0. Any value rejects the candidate.
- `validate.violations` in the on arm: threshold 0. Any value rejects the candidate.
- `bench.match_wall_ms` of the on arm over the off arm: threshold 1.10. A ratio above it rejects the candidate. This is the benchmark tripwire.
- The stronger team's win rate in the on arm: it must stay above 0.50 (C3).

**Data source:** the calibrate `run-report` and the `match-stats` records (`04-plan-calibration.md`, steps 6 and 13). The pair keys come from `04-plan-experiment-flags.md` step 14, and `tuning.flags_on` comes from step 9. The `04b-instrument.md` signal set is extended by `flag.active` and `calibrate.pair`.

## 4. Duration & stopping rules

| Rule | Condition | Action |
|------|-----------|--------|
| **Minimum runtime** | 1000 matches per suite per arm, on the reference machine. A run of 200 matches is for iteration only. | Do not decide before the 1000-match paired run exists. |
| **Early stop — WIN** | The on arm passes more bands, or passes as many and is closer to the band centres by more than 0.05, with no guardrail breach | Verdict `on-better`. Make the candidate the only path and remove the flag (§5). |
| **Early stop — LOSS** | Any on-arm guardrail breach | Verdict `on-rejected`. Remove the candidate and its flag. |
| **Maximum runtime** | One paired run of 1000 matches, plus at most one rerun on a second seed when the first verdict is within the margin | Force a decision. `no-difference` removes the candidate. No flag stays in the file without a verdict. |

Runs are offline and take minutes, so no flag stays on "while data accumulates". A paired run with 1000 matches per arm has a standard error of the goals mean of about 0.05, at a spread of about 1.7 goals per match. Pairing on seeds lowers the error of the difference further. That is why the 0.05 normalized margin sits near one standard error of the smallest band. It is a practical rule, not a significance test. See §7.

## 5. Rollback criteria

Explicit conditions that require immediate rollback:
- The candidate's paired run returns `on-rejected`. Set the flag's `state` to `off` in the same change, then remove it.
- A shipped build is found with a flag in state `on` and no paired report cited in its change description. Set it to `off`, and rerun the paired calibration.
- An arm shows a data-integrity issue, such as a missing statistics record or a report that fails its schema. The run is void. Fix the harness before any verdict.

Rollback procedure: set `flags.<candidate>.state` to `off` in `content/tuning.json`. No rebuild is needed for an override flag. A code flag takes effect at the next binary start. Then follow the removal checklist in `docs/how-to/modding.md`:
1. Delete the flag entry.
2. For a winning override, fold its values into the base values. For a winning code flag, delete the losing branch. For a losing flag, delete its override or its code.
3. Remove the name from `CODE_FLAGS`.
4. Rerun unpaired `calibrate` and `bench`.
5. Update the pinned tests.
6. Cite the report path in the change description.

## 6. Implementation notes

- **Flag registration:** declare the flag in `content/tuning.json` under `flags`, with `owner`, `hypothesis`, `removal_condition`, `state`, and optional `overrides`. A code flag also adds its name to `CODE_FLAGS` in `crates/engine/src/flags.rs`. The pin in `crates/engine/tests/content.rs` asserts that every registered name is declared.
- **Flag check:** engine code calls `config.flags.is_on("<candidate>")` on `MatchConfig` (`crates/engine/src/sim.rs`). The list is resolved once at load and never during a tick. For the decision model, the check sits where the tactics slice's decision layer chooses an action. For the shot model, it sits at the kick branch where a shot is resolved (`sim.rs`, the `Kick::Shot` path).
- **Cohort logic:** none at run time. `calibrate --pair` runs the arms one after the other on the same fixture list and seeds. The fixture planning does not read flag states.
- **Metric instrumentation:** the calibrate `run-report` already carries the bands, the dark paths, and the single-thread figure. This slice adds `calib.pair`, `calib.arms`, `calib.compare`, `calib.verdict`, `calib.flags`, `tuning.flags_on`, and the signals `flag.active` and `calibrate.pair`.
- **Framework:** no framework was detected, and none is added. An environment variable fallback is not used: the flags must be part of the content that the digest, the snapshot, and the report identify.

## 7. Open questions

None — experiment design is complete. The decision margin (0.05 of summed normalized distance) and the rerun rule are practical rules chosen in the plan for this slice (Assumption 15). They are not a statistical power calculation. The first candidate model may replace them with a paired-difference test if a verdict falls near the margin.
