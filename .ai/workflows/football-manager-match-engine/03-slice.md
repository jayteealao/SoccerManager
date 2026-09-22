---
schema: sdlc/v1
type: slice-index
slug: football-manager-match-engine
status: complete
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-22T13:35:00Z"
total-slices: 17
best-first-slice: engine-core
tags: [game, simulation, match-engine, 2d-viewer, rust]
consult-runs: []
slices:
  - slug: engine-core
    status: complete
    complexity: l
    depends-on: []
  - slug: data-schemas-generator
    status: complete
    complexity: m
    depends-on: [engine-core]
  - slug: stream-protocol
    status: complete
    complexity: m
    depends-on: [engine-core]
  - slug: viewer-pitch
    status: in-progress
    complexity: m
    depends-on: [stream-protocol]
  - slug: match-rules
    status: defined
    complexity: l
    depends-on: [engine-core, data-schemas-generator]
  - slug: tactics-and-ai
    status: defined
    complexity: l
    depends-on: [match-rules, data-schemas-generator]
  - slug: commentary
    status: defined
    complexity: s
    depends-on: [match-rules]
  - slug: calibration
    status: defined
    complexity: m
    depends-on: [tactics-and-ai, data-schemas-generator]
  - slug: viewer-match-day
    status: defined
    complexity: m
    depends-on: [viewer-pitch, stream-protocol, commentary]
  - slug: viewer-lineup-tactics
    status: defined
    complexity: l
    depends-on: [viewer-match-day, tactics-and-ai]
  - slug: viewer-reports-recovery
    status: defined
    complexity: m
    depends-on: [viewer-match-day, match-rules]
  - slug: integration
    status: defined
    complexity: m
    depends-on: [calibration, commentary, viewer-lineup-tactics, viewer-reports-recovery]
  - slug: extra-time-penalties
    status: defined
    complexity: s
    depends-on: [match-rules, tactics-and-ai]
  - slug: experiment-flags
    status: defined
    complexity: s
    depends-on: [calibration]
  - slug: scripting-runtime
    status: defined
    complexity: l
    depends-on: [data-schemas-generator, tactics-and-ai, calibration]
  - slug: distribution
    status: defined
    complexity: m
    depends-on: [integration]
  - slug: probe-engine-core
    status: defined
    slice-type: probe
    compressed: true
refs:
  index: 00-index.md
  shape: 02-shape.md
  design: 02b-design.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine engine-core"
---

# Slice Index

## The Slices

Shape handed over a spec that spans a Rust engine, a stream protocol, a browser viewer with four weighted design states, a team generator, a calibration harness, and an observability feed, with six adjudicated intent risks and one open performance risk. The design brief added an empty state, a first-run state, and an error and recovery state to the four weighted ones.

Six product-owner answers cut the roster. The engine ships first so the 50-ticks-per-second performance risk retires before anything is drawn; the viewer starts against a recorded fixture through a mock server; the cut is thin at 12 buildable slices; the visible milestone lands at `viewer-pitch`, the fourth slice, with crude play driven by the live engine; and four parts are deferred as named slices, not dropped: extra time and penalties, feature flags, the scripting runtime that clears RIM-6, and distribution. Review is slug-wide by the product owner's choice, and commentary stands alone while stoppage snapshots ride with match rules. Two chains run in parallel after the root: engine core → data schemas → match rules → tactics and AI → calibration, and engine core → stream protocol → viewer pitch → match-day panels → lineup and tactics → reports and recovery, meeting at integration, the final slice that runs the whole charter scenario.

Plan takes `engine-core` first. The top open risk is unchanged from shape: the single-thread benchmark in the first slice, whose miss reopens the tick rate with the product owner rather than lowering the positional model.

## Slice Strategy

Two tracks after one root. The engine track retires simulation risk in order: loop and physics, then data, then laws, then tactics, then measurement. The viewer track builds from the pitch outward, each slice verified against the fixture before the live engine. The tracks join at `integration`.

Design states and surfaces, per the design brief, map to slices as follows:

- Live play state → `viewer-pitch` (the canvas and playback controls are the live surface).
- Goal moment → `viewer-match-day`; grouped with the panels because the goal moment is a change of the score bug, the feed, and the banner, all of which are panel surfaces.
- Empty state at kick-off → `viewer-match-day`; grouped with the feed because the empty state is the feed's first-time state.
- Pre-match lineup screen → `viewer-lineup-tactics`.
- Stoppage intervention (tactics panel, pending chips, substitutions) → `viewer-lineup-tactics`; grouped with the lineup screen because both edit the same formation, roles, and squad model, and a separate slice would duplicate the editor.
- Loading and first-run states → `viewer-reports-recovery`; grouped with error and recovery because all four are the non-play surfaces around the pitch, and all consume engine launch state.
- Error and recovery → `viewer-reports-recovery`.
- Half-time and full-time reports → `viewer-reports-recovery`; grouped with recovery because both consume the snapshot and replay files (the product owner's coupling answer).

Engine coupling per the product owner: commentary is its own slice; snapshot-at-every-stoppage rides with `match-rules`.

## Recommended Order

1. `engine-core` — retires the performance risk; every slice consumes its loop.
2. `data-schemas-generator` — rules and tactics need the schemas; calibration needs the generator.
3. `stream-protocol` — the viewer cannot start without the stream and the fixture.
4. `viewer-pitch` — the visible milestone; charter scenario through step 5.
5. `match-rules` — laws, restarts, and snapshots; the hinge for changes and recovery.
6. `tactics-and-ai` — C1, C3, C5 in the engine.
7. `commentary` — small; needs only the events.
8. `calibration` — realism bands and the observability records.
9. `viewer-match-day` — panels and the goal moment; needs commentary lines.
10. `viewer-lineup-tactics` — the manager's authority on the page; needs the change queue.
11. `viewer-reports-recovery` — reports, replay files, recovery.
12. `integration` — full charter scenario, Playwright suite, full benchmark, documentation.

Deferred, after the first release: `extra-time-penalties`, `experiment-flags`, `scripting-runtime`, `distribution`.

Parallel option: slices 3 and 4 may run alongside slices 2 and 5 because they depend only on `engine-core`; slice 9 may run alongside slices 6 to 8 once `commentary` lands.

## Cross-Cutting Concerns

- Benchmark tripwire: every engine slice after `engine-core` reruns the benchmark; more than 10 percent CPU or 25 percent memory regression fails verify.
- Schema versions: every data file carries a version; the fail-closed loader rejects unknown versions.
- Identity fields: owner and match identifiers on every record and stream header (RIM-2, RIM-4).
- Observability: `/wf observability init` before `calibration` plans, so the record schema is agreed; `/wf observability build` before `integration`.
- Design: `PRODUCT.md` is missing; run `/wf design football-manager-match-engine setup` before the first viewer slice plans, or plan stops at the design preflight.
- Playwright: enters at `integration`; earlier viewer slices use the in-app browser.
- License: permissive; the dependency audit runs at `integration`; GPL references (FootballEngine) are read, not copied.
- Review: slug-wide, one report at handoff.

## Dependencies Between Slices

- `engine-core` → `data-schemas-generator`, `stream-protocol`.
- `data-schemas-generator` → `match-rules`, `tactics-and-ai`, `calibration`, `scripting-runtime`.
- `stream-protocol` → `viewer-pitch`, `viewer-match-day`.
- `match-rules` → `tactics-and-ai`, `commentary`, `viewer-reports-recovery`, `extra-time-penalties`.
- `tactics-and-ai` → `calibration`, `viewer-lineup-tactics`, `extra-time-penalties`, `scripting-runtime`.
- `commentary` → `viewer-match-day`, `integration`.
- `calibration` → `integration`, `experiment-flags`, `scripting-runtime`.
- `viewer-pitch` → `viewer-match-day`.
- `viewer-match-day` → `viewer-lineup-tactics`, `viewer-reports-recovery`.
- `viewer-lineup-tactics`, `viewer-reports-recovery` → `integration`.
- `integration` → `distribution`.

Longest chain: `engine-core` → `data-schemas-generator` → `match-rules` → `tactics-and-ai` → `calibration` → `integration` (six deep).

## Deferred / Optional Slices

- `extra-time-penalties` — deferred by slice Q4; narrows shape Q21 for the first release only.
- `experiment-flags` — deferred until a second model exists.
- `scripting-runtime` — deferred by shape Q31; clears RIM-6; the workflow's "customizable" status reads "boundary shipped, runtime open" until it closes.
- `distribution` — deferred; resolves U-3.

## Freshness Research

- Source: [Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/)
  Why it matters: the fixed-timestep loop in `engine-core` and the tick-bounded interpolation in `viewer-pitch` follow it; slicing the loop before the renderer keeps the two decoupled.
  Takeaway: the engine slice owns the timestep; the viewer slice owns interpolation; neither slice touches the other's clock.
- Source: [RoboCup soccer monitor](https://rcsoccersim.readthedocs.io/en/latest/soccermonitor.html)
  Why it matters: a monitor that replays a recorded server stream is the precedent for the fixture harness in `stream-protocol`.
  Takeaway: record the stream once, replay it for every viewer slice.
- Source: [Playwright](https://playwright.dev/docs/intro)
  Why it matters: Node 22 is present; the install is `npm init playwright@latest` and needs browser binaries; the product owner placed it after stable markup.
  Takeaway: `integration` authorizes the install as a rung-0 bootstrap; earlier viewer slices use the in-app browser.

## Recommended Next Stage

- **Option A (default):** `/wf plan football-manager-match-engine engine-core` — the root slice and the performance risk; nothing else can be planned in detail before the loop exists.
- **Option B:** `/wf plan football-manager-match-engine all` — not recommended: eleven slices depend on decisions the engine-core plan makes (crate layout, tick frame shape); planning them now would guess.
- **Option C:** `/wf shape football-manager-match-engine` — not needed: slicing surfaced no gap in the spec.
- **Before the first viewer slice plans:** `/wf design football-manager-match-engine setup` (PRODUCT.md is missing) and `/wf observability init` (record schema for `calibration`).
