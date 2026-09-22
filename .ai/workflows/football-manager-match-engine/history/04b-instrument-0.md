---
schema: sdlc/v1
type: augmentation
augmentation-type: instrument
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: engine-core
instrumentation-framework: "serde_json JSON Lines per .ai/observability.md plan-version 1; tracing to stderr for diagnostics"
dark-paths-found: 4
signals-designed: 8
pii-warnings: false
status: ready
created-at: "2026-09-21T21:57:49Z"
updated-at: "2026-09-21T21:57:49Z"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-engine-core.md
  contract: ../../observability.md
---

# Instrumentation: Engine Core

## The Instrumentation

The observability contract fixed four record kinds, a dotted key vocabulary, direct file writes, and a socket feed before any engine code existed. The engine-core slice has no event stream and no socket yet: it produces a tick file, a validator result, and a benchmark number. Every signal below therefore lands in the two record kinds the slice can already emit, `match-stats` from the simulate command and `run-report` from the bench command, plus tracing lines on stderr that never enter a record.

Eight signals and four dark paths are designed. The dark paths are the ones that would let the slice pass while lying: a partial tick file that looks complete, a validator that never ran, a benchmark skewed by a busy machine, and a match where nobody ever gains possession. Each is covered by a field in a record, not by a log line, so the dashboard can query it. No identifier in this slice is personal: `owner.id` is the constant `local-cli` in headless runs, and the machine identity is hashed per the contract and the product owner's plan answer Q6.

Implement wires the fields when it writes the record and bench commands; verify checks that both records validate against the contract's key list. The top risk is drift between the field names here and the `serde` struct field names in `observe/mod.rs`; the plan makes the struct the single source and derives the JSON keys from it.

## 1. Current state

| File | Quality | Existing signals | Dark paths |
|------|---------|-----------------|------------|
| `crates/engine/src/sim.rs` (planned) | dark | none | tick loop runs without a rate signal |
| `crates/engine/src/record.rs` (planned) | dark | none | partial file after an abort |
| `crates/engine/src/validate.rs` (planned) | dark | none | validator never ran |
| `crates/engine/src/decision.rs` (planned) | dark | none | no possession for a whole match |
| `crates/engine-cli/src/simulate.rs` (planned) | dark | none | run ends without a record |
| `crates/engine-cli/src/bench.rs` (planned) | dark | none | contention on the benchmark machine |

Summary: 4 dark paths found across 6 planned files. Framework: serde_json JSON Lines per the contract; tracing to stderr. No file exists yet; quality is `dark` by construction.

## 2. Instrumentation plan

| File | Function/path | Signal type | Signal name | Key fields | Rationale |
|------|--------------|-------------|-------------|------------|-----------|
| `crates/engine-cli/src/simulate.rs` | end of `run()` | event | `match-stats.simulate` | `record.kind`, `schema.version`, `match.id`, `owner.id`, `seed`, `service`, `version`, `build.hash`, `env`, `operation`, `duration_ms`, `outcome`, `engine.ticks_per_s`, `ticks.written`, `validate.ran`, `validate.violations`, `possession.changes`, `ball.max_speed`, `ball.idle_ticks` | Verify a run completed and how fast; expose the no-possession and validator dark paths |
| `crates/engine-cli/src/bench.rs` | end of `run()` | event | `run-report.bench` | `record.kind`, `schema.version`, `run.id`, `seed`, `service`, `version`, `build.hash`, `env`, `operation`, `outcome`, `bench.matches`, `bench.match_wall_ms`, `bench.cpu_ms`, `bench.peak_mem_mb`, `engine.ticks_per_s`, `bench.cpu_wall_ratio`, `machine.hash`, `machine.cpu_model`, `machine.power_plan`, `budget.pass` | The acceptance-criterion number and its provenance |
| `crates/engine/src/sim.rs` | `run()` | gauge | `engine.ticks_per_s` | ticks / wall seconds | Performance signal in both records |
| `crates/engine-cli/src/bench.rs` | `measure()` | gauge | `bench.cpu_wall_ratio` | cpu_ms / wall_ms | Contention detection |
| `crates/engine/src/validate.rs` | `check()` | log | `validate.violation` | `tick`, `rule`, `player.id`, `value` | Debug a failing rule |
| `crates/engine/src/record.rs` | `write_header()` | event | `tickfile.header` | `magic`, `schema.version`, `seed`, `dt_ms`, `ticks.expected`, `float_width` | Reader contract |
| `crates/engine/src/record.rs` | `finish()` | event | `tickfile.trailer` | `ticks.written`, `bytes` | Partial-file detection |
| `crates/engine-cli/src/simulate.rs` | `run()` | trace | `diag.simulate.span` | `seed`, `minutes` | Developer diagnostics on stderr |

## 3. Signal designs

Both records serialize from one `serde` struct each in `crates/engine/src/observe/mod.rs`; the JSON key is the struct field renamed with `#[serde(rename = "…")]` to the dotted contract key. Example shape of the `match-stats` record on stdout, one line:

```json
{"record.kind":"match-stats","schema.version":"1","match.id":"000000000000002a-1758491869123","owner.id":"local-cli","seed":42,"service":"engine-cli","version":"0.1.0","build.hash":"a1b2c3d","env":"dev","operation":"simulate","duration_ms":1730,"outcome":"success","engine.ticks_per_s":156069.0,"ticks.written":270000,"validate.ran":true,"validate.violations":0,"possession.changes":412,"ball.max_speed":31.2,"ball.idle_ticks":0}
```

Example shape of the `run-report` record, one line:

```json
{"record.kind":"run-report","schema.version":"1","run.id":"bench-1758491900000","seed":42,"service":"engine-cli","version":"0.1.0","build.hash":"a1b2c3d","env":"dev","operation":"benchmark","outcome":"success","bench.matches":5,"bench.match_wall_ms":1730,"bench.cpu_ms":1725,"bench.peak_mem_mb":18.4,"engine.ticks_per_s":156069.0,"bench.cpu_wall_ratio":0.997,"machine.hash":"3f9a1c2b7d4e","machine.cpu_model":"AMD Ryzen 7 9800X3D 8-Core Processor","machine.power_plan":"Ultimate Performance","budget.pass":true}
```

Rules: `duration_ms` and `bench.match_wall_ms` are integers; ratios and speeds are floats; `outcome` is `success`, `error`, or `abandoned`; `env` is `dev` unless `SM_ENV` says otherwise. The tracing span carries the seed and the minute count at `info` level and writes to stderr only.

## 4. PII & security notes

| Field | Risk | Recommended handling |
|-------|------|---------------------|
| `machine.hash` | machine identity | SHA-256 of `COMPUTERNAME`, first 12 hex characters, per the contract Block C |
| `machine.cpu_model` | hardware model, not personal | allowed; no user identity |
| `owner.id` | actor identity | the constant `local-cli` in headless runs; a real match sets it from the viewer session later |

No personal identifiers enter a record in this slice. No free text from a user exists in the engine.

## 5. Implementation notes

- Write `crates/engine/src/observe/mod.rs` before `simulate.rs` and `bench.rs`, so both commands import the structs instead of building JSON by hand.
- `serde_json` is a new download (not in the cargo cache); `sha2` is a new download; `tracing` and `tracing-subscriber` are cached.
- Environment variables: `SM_ENV` (default `dev`); `SM_DATA_DIR` is not used in this slice because the records go to stdout until the calibration slice writes files.
- The `match.id` value combines the seed and a millisecond timestamp; it never enters the tick file, so determinism tests stay byte-exact.
- No conflict with the plan steps: the record structs are step 12 of the plan and precede steps 14 and 15.
