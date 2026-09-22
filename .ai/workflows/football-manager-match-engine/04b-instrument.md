---
schema: sdlc/v1
type: augmentation
augmentation-type: instrument
slug: football-manager-match-engine
parent-workflow: football-manager-match-engine
slice-slug: data-schemas-generator
instrumentation-framework: "serde_json JSON Lines per .ai/observability.md plan-version 1; tracing to stderr for diagnostics"
dark-paths-found: 4
signals-designed: 7
pii-warnings: false
status: ready
created-at: "2026-09-22T06:37:07Z"
updated-at: "2026-09-22T06:37:07Z"
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-22T06:37:07Z"
    trigger: new-slice
    because: "data-schemas-generator plan designs the loader, generator, and identity signals (PO plan Q11)"
    changed: "slice-slug, signal table, dark paths; engine-core record kept at history/04b-instrument-0.md"
refs:
  index: 00-index.md
  shape: 02-shape.md
  plan: 04-plan-data-schemas-generator.md
  contract: ../../observability.md
  prior: history/04b-instrument-0.md
---

# Instrumentation: Data Schemas and Team Generator

## The Instrumentation

Engine-core shipped eight signals in two record kinds, and its instrument record is byte-copied at `history/04b-instrument-0.md`. The data-schemas-generator slice adds three things a record cannot see today: which content files a match ran on, why a file was refused, and who owns the record. The probe of engine-core found that a failed run leaves no record at all, so the loader's refusal path is the first dark path here.

Seven signals are designed against the contract's dotted vocabulary: two tracing lines on the loader (`content.loaded`, `content.refused`), one hash that ties every record to its content (`content.hash`), the two club identifiers in `match-stats`, one event per generated league, one line on the first creation of `owner.id`, and the tick-file header extended with the identity fields. Four dark paths are covered: a silent refusal, a match on unknown content, generator drift inside the bounds, and a regenerated owner. No signal carries a person's name, an absolute path, or free text; club and player names are fictional and allowed by the contract.

Implement folds these into the loader, the generator, and the two command-line paths; verify exercises each on the live binary. The top open risk is the `teams` array in `match-stats`, which the contract's Block A does not list: verify's key check treats it as an additive extra, and the observability audit decides whether `team.id` in a per-match record is a string per side or an array.

## 1. Current state

| File | Quality | Existing signals | Dark paths |
|------|---------|-----------------|------------|
| `crates/engine/src/data/mod.rs` (planned) | dark | none | a refused file leaves one stderr line and no structured trace; a match on edited content is indistinguishable from one on the defaults |
| `crates/engine/src/data/generator.rs` (planned) | dark | none | a distribution drifted inside 1..100 produces valid-looking squads |
| `crates/engine/src/observe/identity.rs` (planned) | dark | none | a deleted `owner.id` is recreated without notice |
| `crates/engine/src/observe/mod.rs` | good | `match-stats`, `run-report` with the contract keys | none new |
| `crates/engine/src/record.rs` | good | `tickfile.header`, `tickfile.trailer` | none new |
| `crates/engine-cli/src/simulate.rs` | good | `match-stats.simulate`, `diag.simulate.span` | none new |

Summary: 4 dark paths found across 6 files (3 planned). Framework: serde_json JSON Lines per the contract; tracing to stderr for diagnostics.

## 2. Instrumentation plan

| File | Function/path | Signal type | Signal name | Key fields | Rationale |
|------|--------------|-------------|-------------|------------|-----------|
| `crates/engine/src/data/mod.rs` | `load_json()` | event (tracing info) | `content.loaded` | `kind`, `path` (relative), `schema_version`, `items`, `hash` | Every run states which files it read and their versions |
| `crates/engine/src/data/mod.rs` | `load_json()` | log (tracing error) | `content.refused` | `kind`, `path` (relative), `field`, `reason` | A refusal is visible with the exact file and field |
| `crates/engine/src/data/mod.rs` | `Content::hash()` | gauge | `content.hash` | 12 hex characters | Ties `match-stats` and `run-report` to the content they ran on |
| `crates/engine-cli/src/simulate.rs` | end of `run()` | event | `match-stats.teams` | `teams: [{team.id, team.name}]` | Records which clubs played, from the loaded files |
| `crates/engine/src/data/generator.rs` | `generate_league()` | event (tracing info) | `generator.league` | `seed`, `clubs`, `players`, `attr.mean`, `attr.min`, `attr.max`, `hash` | Makes distribution drift visible before calibration |
| `crates/engine/src/observe/identity.rs` | `load_or_create()` | event (tracing info) | `identity.owner_created` | `path` (relative: `owner.id`) | Marks the one moment the owner changes |
| `crates/engine/src/record.rs` | `write_header()` | event (tracing info) | `tickfile.header` | existing keys plus `owner.id`, `match.id`, `schema.version` 2 | The tick file names its owner and match |

## 3. Signal designs

```rust
// crates/engine/src/data/mod.rs — load_json()
tracing::info!(signal = "content.loaded", kind = %kind, path = %relative, schema_version = version, items = count, hash = %hash12);
tracing::error!(signal = "content.refused", kind = %kind, path = %relative, field = %field_path, reason = %reason);
// The EngineError::Data message carries the same four values in this order:
// "content refused: <kind> <relative>: <field_path>: <reason>"

// crates/engine/src/observe/mod.rs — MatchStats gains
#[serde(rename = "content.hash")] pub content_hash: String,   // 12 hex
pub teams: [TeamRef; 2],                                        // {"team.id": ..., "team.name": ...}
// RunReport gains
#[serde(rename = "content.hash")] pub content_hash: String,

// crates/engine/src/data/generator.rs — generate_league()
tracing::info!(signal = "generator.league", seed, clubs, players, attr.mean = mean, attr.min = min, attr.max = max, hash = %hash12);

// crates/engine/src/observe/identity.rs — load_or_create()
tracing::info!(signal = "identity.owner_created", path = "owner.id");

// crates/engine/src/record.rs — write_header(): existing line gains
owner_id = %owner_hex, match_id = %match_id, schema_version = 2
```

Types: `content.hash` is a 12-character lower-case hex string; `teams` is a two-element array of objects with the two dotted keys; every tracing field is a display string or an integer; no float leaves the tracing layer except the three generator statistics, which are rounded to two decimals.

## 4. PII & security notes

No PII concerns identified in the planned signals. `owner.id` is opaque per the contract and enters records only, never the stderr lines. Paths are relative to the content folder or the runtime data folder; an absolute path never enters a record. Club and player names are fictional and generated, which the contract allows.

## 5. Implementation notes

- Touch `crates/engine/src/data/mod.rs` once: the loader emits both `content.loaded` and `content.refused` from the same function, so the relative-path helper is written there first.
- `Content::hash()` runs once per process at load time; `bench` must compute it before the warm-up match, never per match, so the benchmark path stays free of hashing.
- `MatchStats` and `RunReport` gain fields in `observe/mod.rs` before `simulate.rs` and `bench.rs` are edited, so both binaries compile against the new struct in one pass.
- No new environment variable. `SM_DATA_DIR` (existing in the contract) now also holds `owner.id`.
- The `teams` array is an additive extra to the engine-core key set; verify's instrument key check counts it as extra, and the observability audit settles the vocabulary. No conflict with the plan step order.
