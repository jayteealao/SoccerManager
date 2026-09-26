---
schema: sdlc/v1
type: review-command
slug: football-manager-match-engine
dimension: architecture
parent: 07-review.md
review-scope: slug-wide
status: complete
created-at: "2026-09-23T20:34:36Z"
updated-at: "2026-09-23T20:34:36Z"
verdict: ship
metric-findings-total: 1
metric-findings-blocker: 0
metric-findings-high: 0
---

# Review: architecture

6 findings in this dimension. 5 are fixed and 1 are deferred. Nothing is open at BLOCKER or HIGH.

| ID | Sev | Conf | Status | Surfaced | File:Line | Issue | Fix / reason |
|---|---|---|---|---|---|---|---|
| M2 | MED | high | fixed | 2026-09-23T20:34:36Z | crates/engine-cli/src/serve.rs:107 | serve::run is one 270-line function with bare exit codes | Split into Serving::play (per connection) and Serving::rewind, with EXIT_FULL_TIME / EXIT_REFUSED / EXIT_VIEWER_GONE. Values kept (the launcher reads them). calibrate::run was not split. |
| M1 | MED | high | fixed | 2026-09-23T20:34:36Z | crates/engine/src/lib.rs:56 | Tick rate is set in two places nothing keeps in step | The loader refuses any dt other than one clock tick; commentary uses the clock constant; a cross-crate test pins the protocol minute; the viewer duplicate constant is removed; data-files.md updated. |
| M5 | LOW | medium | fixed | 2026-09-23T20:34:36Z | crates/engine/src/commentary/mod.rs:237 | Commentary rotation seeded by list position | Salted by an FNV-1a hash of the variant name. |
| M6 | LOW | medium | fixed | 2026-09-23T20:34:36Z | crates/engine/src/sim.rs:301 | Wire event strings written twice with no cross-check | Mapping extracted to event_type(); a test checks every kind's code against its wire type. |
| M7 | LOW | high | fixed | 2026-09-23T20:34:36Z | web/main.mjs:75 | clockText duplicates clockAt; recovery.mjs redefines the tick rate | clockText removed (uses clockAt); recovery.mjs imports the constant. |
| M10 | NIT | medium | deferred | 2026-09-23T20:34:36Z | web/recovery.mjs:33 | refusalReason parses the engine error prefix | A structured reason field crosses the engine, launcher and page contract; not localized. |
