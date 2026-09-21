---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: scripting-runtime
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: l
depends-on: [data-schemas-generator, tactics-and-ai, calibration]
tags: [engine, modding, scripting, deferred, rim-6]
deferred: true
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-data-schemas-generator.md, 03-slice-tactics-and-ai.md, 03-slice-calibration.md]
  plan: 04-plan-scripting-runtime.md
  implement: 05-implement-scripting-runtime.md
---

# Slice: Scripting Runtime for Modding

## The Slice

The product owner chose full modding (shape Q28) and sequenced the scripting runtime after the plugin boundary (Q31). RIM-6 names the risk that this slice never gets scheduled and "customizable" is reported done on the strength of data files alone. This file is the clearing mechanism: the slice exists, is named, and stays open until built.

This slice embeds a sandboxed scripting runtime in the Rust engine behind the plugin interface the data slice defined, so a script can override player decisions and rule-pack behaviors within a performance budget.

It ships after the first release and after calibration, so scripted behavior can be measured against the same bands. The top risk is the tick budget: a script hook that runs per agent per tick must stay inside the benchmark tripwire.

## Goal

A modder writes a script that changes how players decide or how a rule behaves, and the engine runs it safely inside the tick budget.

## Why This Slice Exists

Q28 set the customization ceiling; Q31 deferred the runtime; RIM-6 requires a named slice so the wall clears visibly.

## Scope

In: runtime choice (plan decides among embeddable options with a permissive license), sandbox limits (time, memory, no file or network access), hook points (decision scoring, rule handlers, commentary), a script pack format, a sample script, and a benchmark rerun with the tripwire.
Out: a script editor in the viewer; script sharing or marketplace.

## Acceptance Criteria

- Given a script pack that overrides the decision scoring hook, When a seeded match runs, Then the scripted decisions take effect and the event stream records the pack identifier.
  <!-- observable: false — cargo test with a sample script -->
- Given a script that exceeds its time budget on a tick, Then the engine aborts that hook, logs it, and continues with the default decision.
  <!-- observable: false — cargo test with a looping script -->
- Given a script that attempts file or network access, Then the sandbox denies it and the engine records the denial.
  <!-- observable: false — cargo test -->
- Given the benchmark reruns with the sample script loaded, Then CPU time per match is within 10 percent of the calibration baseline.
  <!-- observable: true — developer-visible benchmark report -->
  verify: { method: cargo bench harness compare, env: reference laptop, fixture: seed 42 with the sample pack, rung: cli-direct }

## Dependencies on Other Slices

- `data-schemas-generator`: the plugin interface and pack schemas.
- `tactics-and-ai`: the decision hooks.
- `calibration`: the baseline to compare scripted behavior against.

## Risks

- Runtime license or maintenance risk: plan checks the license (NFR-5) and the release cadence before choosing.
- Per-tick hook cost: hooks run at decision frequency, not steering frequency.
