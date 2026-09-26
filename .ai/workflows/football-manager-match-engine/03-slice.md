---
schema: sdlc/v1
type: slice-index
slug: football-manager-match-engine
status: complete
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-24T06:26:59Z"
total-slices: 24
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
    status: complete
    complexity: m
    depends-on: [stream-protocol]
  - slug: match-rules
    status: complete
    complexity: l
    depends-on: [engine-core, data-schemas-generator]
  - slug: tactics-and-ai
    status: complete
    complexity: l
    depends-on: [match-rules, data-schemas-generator]
  - slug: commentary
    status: complete
    complexity: s
    depends-on: [match-rules]
  - slug: calibration
    status: complete
    complexity: m
    depends-on: [tactics-and-ai, data-schemas-generator]
  - slug: viewer-match-day
    status: complete
    complexity: m
    depends-on: [viewer-pitch, stream-protocol, commentary]
  - slug: viewer-lineup-tactics
    status: complete
    complexity: l
    depends-on: [viewer-match-day, tactics-and-ai]
  - slug: viewer-reports-recovery
    status: complete
    complexity: m
    depends-on: [viewer-match-day, match-rules]
  - slug: integration
    status: complete
    complexity: m
    depends-on: [calibration, commentary, viewer-lineup-tactics, viewer-reports-recovery]
  - slug: extra-time-penalties
    status: complete
    complexity: s
    depends-on: [match-rules, tactics-and-ai]
  - slug: experiment-flags
    status: complete
    complexity: s
    depends-on: [calibration]
  - slug: scripting-runtime
    status: complete
    complexity: l
    depends-on: [data-schemas-generator, tactics-and-ai, calibration]
  - slug: distribution
    status: complete
    complexity: m
    depends-on: [integration]
  - slug: probe-engine-core
    status: complete
    slice-type: probe
    compressed: true
  - slug: realism-bands-v2
    status: complete
    complexity: m
    depends-on: [calibration, probe-engine-core]
    source: extension
    extension-round: 1
  - slug: defending-and-discipline
    status: complete
    complexity: l
    depends-on: [realism-bands-v2]
    source: extension
    extension-round: 1
  - slug: tuning-loop
    status: complete
    complexity: m
    depends-on: [realism-bands-v2]
    source: extension
    extension-round: 2
  - slug: lone-forward
    status: complete
    complexity: l
    depends-on: [tuning-loop, defending-and-discipline]
    source: extension
    extension-round: 2
  - slug: keeper-and-shots
    status: complete
    complexity: l
    depends-on: [defending-and-discipline]
    source: extension
    extension-round: 1
  - slug: tempo-and-restarts
    status: complete
    complexity: m
    depends-on: [keeper-and-shots]
    source: extension
    extension-round: 1
  - slug: realism-tuning
    status: skipped
    skip-record: skip-slice-realism-tuning.md
    complexity: m
    depends-on: [tempo-and-restarts]
    source: extension
    extension-round: 1
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

Extension round 1, strictly in order, after every earlier slice:

13. `realism-bands-v2` — the bands first, so each later slice shows its progress against them.
14. `defending-and-discipline` — the defending bug behind the ten-player collapse and the card rate.
15. `keeper-and-shots` — keeper saves, blocks and shot flight, which bring corners and goal kicks.
16. `tempo-and-restarts` — carry time, restart delays and pass counting.
17. `realism-tuning` — the final retune against every band on five seeds, and the browser suite.

Extension round 2 places two slices after `defending-and-discipline` and before `keeper-and-shots` (Q-I2, Q-X4). The numbers continue the list; the order to build is 14, 18, 19, 15, 16, 17:

18. `tuning-loop` — targeted calibrate runs with a baseline diff, so every later tuning step takes minutes, not hours.
19. `lone-forward` — the lone central forward and the tackle odds, which own the red-card criterion and the 4-4-1-1 and 3-4-3 pairings.

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

## Extension Round 1 — 2026-09-23
Source: user request

### New Slices Added
| Slice | Goal | Complexity | Depends On |
|-------|------|------------|------------|
| `realism-bands-v2` | Eleven sourced realism bands, a formations suite and the missing record fields; failing bands become the baseline. | M | `calibration`, `probe-engine-core` |
| `defending-and-discipline` | Goal-side cover in every formation, a real 10-player shape, an acting keeper, and realistic second yellows. | L | `realism-bands-v2` |
| `keeper-and-shots` | On-frame saves by shot quality, parries and blocks, shots that rise, honest on-target and xG; corners and goal kicks follow. | L | `defending-and-discipline` |
| `tempo-and-restarts` | Realistic carry time and restart delays, ball in play about an hour, only real passes counted, throw-ins in band. | M | `keeper-and-shots` |
| `realism-tuning` | Every band on five seeds, the formations suite, the benchmark and the 21-test browser suite. | M | `tempo-and-restarts` |

### Motivation
A fresh 10,000-match calibration at `5a235a4` passed every band on five seeds, but the matches were not realistic. Ten or more goals came in 10.5% of matches, 45% had a sending-off, 75% of shots were on target, teams made about 1,300 passes each, and almost no corners were awarded. Research found three root causes. Defending has no marking or goal-side cover, so a lone central striker, whether left after a red card or placed by a 4-3-3, scores at will. The keeper holds every save and no defender blocks, so nothing goes behind for a corner. Players release the ball about 0.25 s after receiving it, with restarts about four times too short. The product owner asked for bands first, three mechanism slices judged on their own criteria, and one final tuning slice that must pass every band. The two ship-blocking deferrals stay open by the product owner's choice.

Intent delta: RIM-7 to RIM-10 are adjudicated in place by the extension interview. Charter commitment C7 is added. Existing slices are not modified.

## Extension Round 2 — 2026-09-24
Source: user request (after `po-answers.md` Q-I2)

### New Slices Added
| Slice | Goal | Complexity | Depends On |
|-------|------|------------|------------|
| `tuning-loop` | Targeted calibrate runs on one seed, a stored baseline with a per-band diff and sampling error, the red-card experiment as a suite, and a measured build profile; one target in under 2 minutes. | M | `realism-bands-v2` |
| `lone-forward` | A lone central forward that passes, holds up and stays onside, and realistic won-tackle against foul odds; no advantage from a red card, and 4-4-1-1 and 3-4-3 hold against 4-4-2. | L | `tuning-loop`, `defending-and-discipline` |

### Motivation
`defending-and-discipline` built every planned mechanism, but two criteria failed at every in-bounds tuning: the reduced side outscored the full side in all three red-card arms, and 4-4-1-1 (5.18) and 3-4-3 (4.17) scored above 4.0 against 4-4-2. The measured cause is a lone central forward with no forward team-mate, who dribbles and shoots twice as often and is never offside, plus about four fouls for each won tackle. The product owner moved both criteria into a new `lone-forward` slice before `keeper-and-shots` (Q-I2). The same product owner agreed a fast tuning loop earlier: one seed of every suite takes about 76 minutes, and the remaining slices are tuning work. `defending-and-discipline` is verified on the criteria it keeps; its slice file is not changed, and `steer.md` records the moved criteria for verify.

Intent delta: RIM-11 and RIM-12 are adjudicated in place by the extension interview (Q-X1 to Q-X3). Charter delta: none; both slices serve C7, and no new commitment is added. Existing slices are not modified.
