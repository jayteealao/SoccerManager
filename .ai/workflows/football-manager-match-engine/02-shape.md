---
schema: sdlc/v1
type: shape
slug: football-manager-match-engine
status: complete
stage-number: 2
created-at: "2026-09-21T19:34:57Z"
updated-at: "2026-09-21T19:34:57Z"
docs-needed: true
docs-types: [readme-update, tutorial, how-to, reference, explanation]
augmentations-needed: [benchmark, instrument, experiment]
charter-scenario: authored
tags: [game, simulation, match-engine, 2d-viewer, rust, greenfield]
consult-runs: []
refs:
  index: 00-index.md
  intake: 01-intake.md
  design: 02b-design.md
  next: 03-slice.md
next-command: wf-slice
next-invocation: "/wf slice football-manager-match-engine"
---

# Shape

## The Shape

Intake handed over a ratified charter of six commitments, four open questions, and a high-severity risk, RIM-1: an engine built on minute-by-minute event probabilities would pass the statistics and starve the 2D viewer. The repository was empty, so research mapped toolchains and the external landscape instead of code: Rust 1.92, .NET 10, Go 1.25, and Node 22 are installed; no C compiler exists; the in-app browser is ready and Playwright is an install away.

Thirty-two product-owner questions across seven rounds fixed the spec. The engine runs as a native Rust process on the user's machine and streams ticks to the page over a local socket at 50 ticks per simulated second, which disposes of RIM-1 and RIM-3 together. Manager changes are queued at any time and apply at the next qualifying stoppage: tactics at any dead ball, substitutions at any dead ball within the 5-in-3-windows rule. Determinism is best effort with a seedable random-number generator, the viewer keeps the whole match in a compact encoding for rewind, and a crash restarts from the last stoppage snapshot. Teams are fictional and generated, the attribute schema is data-driven on a 1 to 100 scale, the license is permissive, and full modding ships as a plugin boundary now with a scripting runtime as a named later slice. A blind pre-mortem added two risks, RIM-5 (interpolation invents movement) and RIM-6 (the modding wall never clears); both are adjudicated below with named mechanisms. All six RIMs are adjudicated; none is carried.

Slice comes next: the spec spans an engine, a protocol, a viewer with four weighted states, a team generator, a calibration harness, and an observability feed, so it needs decomposition. Plan then resolves the design image gate and writes the visual contract. The top open risk is performance at 50 ticks per second: the headless target of a 90-minute match in under 2 seconds on one thread is unproven for an agent-based engine with ball physics, so the first engine slice must carry the benchmark harness, and a miss reopens the tick-rate question with the product owner rather than silently lowering it.

## Problem Statement

A football manager game needs a match engine and a 2D viewer before any other feature has meaning. The engine must produce continuous, believable play from lineups, attributes, and tactics, and the viewer must show that play and let the manager intervene. Nothing exists yet; the repository is empty.

## Primary Actor / User

- The manager: a human who picks a lineup and tactics, watches the match, and intervenes at stoppages. Plays alone against a computer-managed team in the first version.
- The AI manager: a module that sets up the opponent and reacts during the match; it also manages both teams in headless calibration runs.
- The developer and modder: tunes constants, team data, and rule packs through data files; reads the observability output.

## Desired Behavior

**Core loop.** The manager selects a formation, eleven starters, and up to seven substitutes from a generated squad of about 22 players, then sets mentality, team instructions, and per-player roles and duties (Tier 2). Kick-off starts the engine, which simulates at 50 ticks per second and streams every tick to the viewer. The viewer draws the ball and 22 markers at 60 frames per second, with playback at 1x to 8x, pause, skip to the next stoppage, and rewind over the whole match. The manager opens the tactics panel at any time; a change is queued and shown as pending, and applies at the next qualifying stoppage. The match ends with a full-time report, a saved replay, and the observability output.

**Named mechanisms** (each named here so an acceptance criterion may cite it):

- **Local socket server.** The Rust engine runs as a native process and serves the tick stream, events, and commands over a local socket to the page. It replaces an in-browser WebAssembly build and a hosted server; the product owner chose it for native speed and threads (Q9).
- **Live tick stream with a viewer buffer.** The engine produces ticks ahead of playback; the viewer buffers a few ticks to smooth pacing and drops playback speed with a notice when the engine cannot sustain the chosen speed (Q5, Q18).
- **Tick-bounded interpolation.** A rendered frame lies on the straight segment between two consecutive engine ticks and never extrapolates beyond the next tick. At playback speeds where ticks exceed frames, the viewer skips ticks instead of inventing intermediate positions. This mechanism replaces free-running interpolation and exists to honor C2 (adjudicates RIM-5).
- **Stoppage-gated change queue driven by a rule pack.** A manager change is recorded at request time with its type. The set of stoppages that admit each change type, and the substitution limit, live in a rule pack data file, not in the tick loop. It replaces a hard-coded stoppage table so that the manager's authority over timing is data the manager or a modder can read and change (Q1, Q19, Q21; pre-mortem candidate 3).
- **Snapshot at every stoppage.** The engine writes a match-state snapshot at every dead ball. Restart-from-last-stoppage reads it (Q17).
- **Seedable engine-owned RNG.** Every match takes a seed; a test or calibration run passes one; a real match draws a fresh seed. Same seed, same build, same machine reproduces a match (Q6, Q32).
- **Whole-match compact history.** The viewer keeps every tick in a compact binary encoding for rewind and for the saved replay (Q20).
- **Data-driven attribute schema.** The attribute list (about 30 to 50 attributes, 1 to 100) lives in a configuration file; roles map to attributes through data; the engine ships with a default set (Q26).
- **Plugin boundary.** Tuning constants, team data, and rule packs are data files with a validated schema; the engine defines the interface a scripting runtime will call. The runtime itself is a later slice (Q28, Q31; adjudicates RIM-6).
- **AI manager module.** One module sets up the opponent and reacts during play; the same module manages both teams in headless runs (Q4).
- **Team generator.** Fictional clubs and players with realistic attribute distributions; the calibration runs use it (Q12).
- **Event stream and statistics record.** The engine emits structured events and an aggregate statistics record per match, for the viewer and for the observability pipeline (C4).
- **Reserved identity fields.** Every match record and replay carries an owner identifier and a match identifier so a server can be added later without a rewrite (Q27; pre-mortem candidate 1).

## Ambiguity Inventory

- **AMB-1** — Engine runtime location (browser, native local, server) — source: 01-intake.md#unknowns-open-questions — state: closed (Round 3, Q9: native local process)
- **AMB-2** — Engine language — source: 01-intake.md#known-constraints — state: closed (Extension 1, Q25: Rust)
- **AMB-3** — "Multiple users" meaning — source: 01-intake.md#unknowns-open-questions — state: closed (Extension 1, Q27: independent sessions, identity reserved)
- **AMB-4** — Project type and timeline — source: 01-intake.md#unknowns-open-questions — state: closed (Round 5, Q23: open source, no deadline held)
- **AMB-5** — Final metric thresholds — source: 01-intake.md#success-criteria — state: closed (intake thresholds adopted as acceptance criteria; refinement named in Verification Strategy for `/wf observability init` and verify)
- **AMB-6** — Tick rate — source: 01-intake.md#charter (C2) — state: closed (Round 2, Q7: 50 per second)
- **AMB-7** — Determinism and replay — source: 01-intake.md#assumptions — state: closed (Round 2, Q6; Extension 2, Q32)
- **AMB-8** — Live simulation versus replay, and when changes apply — source: 01-intake.md#restated-request — state: closed (Round 1, Q1; Round 2, Q5)
- **AMB-9** — Playback controls — source: 01-intake.md#restated-request — state: closed (Round 2, Q8)
- **AMB-10** — AI opponent behavior — source: 01-intake.md#primary-user-actor — state: closed (Round 1, Q4)
- **AMB-11** — Team and player data source — source: 01-intake.md#freshness-research — state: closed (Round 3, Q12)
- **AMB-12** — Attribute model — source: 01-intake.md#charter (C3) — state: closed (Extension 1, Q26); the default attribute list is parked to plan as a data-design item, not a blocker
- **AMB-13** — Tactics depth — source: 01-intake.md#restated-request — state: closed (Round 1, Q3)
- **AMB-14** — Commentary depth — source: 01-intake.md#charter (C6) — state: closed (Round 5, Q22)
- **AMB-15** — Injury persistence across matches — source: 01-intake.md#charter (C5) — state: parked (Out of Scope: persistence across matches is a later feature; in-match injuries are in)
- **AMB-16** — Laws of the game covered — source: 01-intake.md#restated-request — state: closed (Round 5, Q21)
- **AMB-17** — Match screen content — source: 01-intake.md#restated-request — state: closed (Round 3, Q10)
- **AMB-18** — Match-day states — source: 01-intake.md#restated-request — state: closed (Round 3, Q11)
- **AMB-19** — Visual direction — source: 01-intake.md#recommended-next-stage — state: closed (Round 3b, Q13 to Q16; 02b-design.md)
- **AMB-20** — Engine-to-viewer transport — source: 01-intake.md#assumptions — state: closed (Q5, Q9); the wire encoding is parked to plan (Unknowns entry U-1)
- **AMB-21** — Replay persistence — source: 01-intake.md#restated-request — state: closed (Round 4, Q20)
- **AMB-22** — Engine failure and lag behavior — source: 01-intake.md#restated-request — state: closed (Round 4, Q17, Q18)
- **AMB-23** — Customization surface — source: 01-intake.md#known-constraints — state: closed (Extension 1, Q28; Extension 2, Q31)
- **AMB-24** — Observability sink and transport — source: 01-intake.md#charter (C4) — state: parked (Unknowns entry U-2, receiving stage `/wf observability init`)
- **AMB-25** — Augmentations needed — source: shape Step 6b — state: closed (Round 5, Q24)
- **AMB-26** — Documentation audience — source: shape Step 6a — state: closed (Q23: contributors and players)
- **AMB-27** — Distribution and install — source: 01-intake.md#out-of-scope-for-now — state: closed (Q9: native binary plus page); packaging detail parked (Unknowns entry U-3)
- **AMB-28** — Verification tooling — source: shape Step 3 — state: closed (Extension 2, Q29)
- **AMB-29** — Lineup input scope — source: 01-intake.md#restated-request — state: closed (Round 1, Q2)
- **AMB-30** — Match time structure — source: 01-intake.md#restated-request — state: closed (Q21: stoppage time, extra time, penalties; 1x playback equals match time)
- **AMB-31** — Cross-origin isolation headers for threaded WebAssembly — source: research sub-agent 2 — state: closed (Q9 removed the browser build from v1)
- **AMB-32** — Dependency license compatibility — source: research sub-agent 2 — state: closed (Extension 2, Q30)
- **AMB-33** — Interpolation may invent movement between ticks — source: pre-mortem 1 — state: closed (named mechanism: tick-bounded interpolation; RIM-5)
- **AMB-34** — Modding wall never clears — source: pre-mortem 2 — state: closed (named later slice; RIM-6)
- **AMB-35** — Identity fields never threaded through v1 — source: pre-mortem candidate — state: closed (AC-24)
- **AMB-36** — Performance metric gamed by parallel runs — source: pre-mortem candidate — state: closed (AC-19 measures one match on one thread)
- **AMB-37** — Stoppage gating hard-coded in the tick loop — source: pre-mortem candidate — state: closed (named mechanism: rule pack)
- **AMB-38** — Which stoppages admit a substitution, and the limit — source: Round 3 correction — state: closed (Round 4, Q19; Round 5, Q21)
- **AMB-39** — Operating systems for the native engine — source: research sub-agent 1 — state: parked (Unknowns entry U-3)

## Charter Scenario

1. The manager opens the page while the engine process runs locally → the header shows "Engine connected" with the engine version.
2. The manager picks a formation and fills eleven slots plus up to seven bench slots from a generated squad → the kick-off control stays disabled until the lineup is legal; an illegal lineup shows the reason inline.
3. The manager sets mentality, team instructions, and roles and duties → the tactics summary shows the chosen values.
4. The manager presses kick-off → the clock advances, 22 markers and the ball move every tick, and no marker leaves the pitch bounds.
5. The manager sets playback to 4x → the clock advances four times faster; if the engine cannot sustain 4x, the notice shows the sustained speed.
6. The manager opens the tactics panel during play and changes mentality → a pending chip appears; at the next dead ball the chip clears and the event feed shows "Tactical change applied".
7. The manager queues a substitution → it applies at the next dead ball; the lineup panel swaps the players; the substitution counter decrements.
8. A goal occurs → the score updates, a banner shows, the event feed and commentary add lines that name the scorer, the minute, and the score state.
9. The manager rewinds to the goal and replays it → the scrubber moves; the replayed frames equal the stored ticks.
10. Half-time → the half-time report shows statistics equal to the event counts.
11. Full-time → the final report shows; "save replay" writes a file; the event stream and statistics record are written for the observability pipeline.
12. In a headless run where the AI team trails after minute 70 → an AI tactical-change event appears in the event stream.

## Acceptance Criteria

Engine:

- **AC-1** (automated) Given a legal lineup, tactics, and a seed, When the engine simulates a match, Then it emits 50 ticks per simulated second, each tick carrying the ball position and 22 player positions.
- **AC-2** (automated) Given the same seed, build, and inputs on the same machine, When two matches run, Then their tick streams are identical.
- **AC-3** (automated) Given 1000 headless matches between generator teams of equal strength, When statistics aggregate, Then goals per match lie in 2.4 to 3.2, shots per team in 8 to 16, and possession for either side in 35 to 65 percent.
- **AC-4** (automated) Given team A with attributes 15 percent higher than team B, When 1000 matches run, Then team A wins more than 50 percent.
- **AC-5** (automated) Given any tick, Then no player position is outside the pitch bounds, no two players share a point, and ball speed does not exceed 40 m/s.
- **AC-6** (automated) Given open play far from the ball, Then each team's formation shape stays within a tolerance defined by the rule pack.
- **AC-7** (automated) Given a queued tactics change, When the next dead ball occurs, Then the change applies at that tick and an event records it; and Given the same queued change, When play continues without a dead ball, Then the change has not applied.
- **AC-8** (automated) Given five substitutions used across three windows, When a sixth is queued, Then the engine rejects it with a reason event; and Given four used, When a fifth is queued, Then it applies at the next dead ball.
- **AC-9** (automated) Given a player's fatigue passes a threshold, Then that player's speed and decision attributes degrade per the fatigue curve in the tuning file.
- **AC-10** (automated) Given an injury event, Then the injured player leaves play, and the AI manager substitutes within the next window when a substitution is available.
- **AC-11** (automated) Given offside, foul, card, corner, throw-in, goal kick, free kick, penalty, and stoppage-time conditions, Then the engine emits the matching event and restarts play with the matching set piece.
- **AC-12** (automated) Given a knockout fixture level at full time, Then extra time runs and, if still level, a penalty shoot-out decides the result.
- **AC-13** (automated) Given any event, Then the commentary line names the player and team and varies with minute, score state, and repeated-event context.
- **AC-14** (automated) Given a stoppage, Then a snapshot is written; and Given an engine restart from that snapshot, Then the match resumes at that stoppage with the same score, clock, lineups, and used substitutions.
- **AC-15** (automated) Given a rule pack, attribute schema, tuning file, or team data file with a schema violation, When the engine loads it, Then it refuses with a message naming the file and field; and Given valid files, Then it loads them.
- **AC-16** (automated) Given the AI manager's team trails after minute 70, Then a tactical-change event appears before full time in at least 90 percent of headless runs.
- **AC-17** (automated) Given a match, Then the engine writes an event stream (one structured record per event) and one aggregate statistics record; a consumer parses both without transformation.
- **AC-18** (automated) Given a match record or replay file, Then it carries an owner identifier and a match identifier that round-trip through save and load.
- **AC-19** (automated, benchmark) Given one headless 90-minute match on one thread on the reference laptop, Then wall time is under 2 seconds; and Given 1000 matches, Then wall time is under 30 minutes.

Viewer:

- **AC-20** (interactive: in-app browser now, Playwright later; evidence: screenshot and console) Given the engine streams ticks, When the viewer plays at 1x, Then the frame rate holds 60 frames per second with no dropped ticks over a 5-minute window.
- **AC-21** (automated) Given two consecutive ticks, Then every rendered marker position lies on the straight segment between them; and Given a playback speed where ticks exceed frames, Then the viewer skips ticks and never renders a position beyond the next tick.
- **AC-22** (interactive; evidence: screenshot) Given a goal in the stream, Then the score, the banner, the event feed, and the commentary update within one frame of the goal tick.
- **AC-23** (interactive; evidence: screenshot) Given the manager rewinds to a stored tick, Then the frame drawn equals the stored positions for that tick.
- **AC-24** (interactive; evidence: screenshot) Given the engine sustains less than the chosen speed, Then playback drops to the sustained speed and a notice names it; and Given the engine sustains the chosen speed, Then no notice shows.
- **AC-25** (interactive; evidence: screenshot) Given an illegal lineup (fewer than eleven, no goalkeeper, or a player twice), Then kick-off is disabled and the reason shows; and Given a legal lineup, Then kick-off is enabled.
- **AC-26** (interactive; evidence: screenshot, recording) Given the engine process is killed mid-match, Then the viewer shows the failure and offers restart from the last stoppage; and Given restart, Then play resumes at that stoppage.
- **AC-27** (interactive; evidence: screenshot) Given a queued change, Then a pending chip shows until the change applies, and the chip clears at the applying stoppage.
- **AC-28** (automated) Given a full match at 50 ticks per second, Then the viewer's stored history stays under 300 MB and the saved replay file loads back into the viewer.
- **AC-29** (manual) Given the match screen at 1280 by 800 in a lit room, Then a reviewer reads the clock, score, and statistics without zooming, and focus rings are visible on every control.

## Non-Functional Requirements

- **NFR-1 Performance (engine).** One 90-minute match under 2 seconds on one thread; 1000 matches under 30 minutes. `yields-to: C2` — a miss reopens the tick rate with the product owner; it never lowers the per-tick positional model silently.
- **NFR-2 Performance (viewer).** 60 frames per second at 1x to 8x with 23 moving markers; rendering off the main thread where the browser allows. `yields-to: C2` — frame pacing never invents positions.
- **NFR-3 Memory.** Whole-match history in the browser under 300 MB. Honors the product owner's Q20 choice of whole-match rewind with compact encoding.
- **NFR-4 Determinism.** Best effort; seedable RNG; same-machine reproduction for tests. No cross-machine promise.
- **NFR-5 License.** MIT or Apache-2.0; dependencies permissive or LGPL; no GPL code copied.
- **NFR-6 Accessibility.** Visible focus rings at 3:1, reduced-motion respected, color never the only indicator for cards, fatigue, or pending state.
- **NFR-7 Data validation.** Every data file (rule pack, attribute schema, tuning, team data) has a schema and fails closed with a named error.
- **NFR-8 Maintainability.** Engine, protocol, viewer, and generator are separate packages with their own tests; the plugin boundary is a versioned interface.
- **NFR-9 Platform.** Windows 11 first (the development and reference machine); other operating systems are a parked question (U-3).

## Edge Cases / Failure Modes

- Engine slower than playback: drop speed with notice (AC-24).
- Engine crash or socket drop: restart from the last stoppage snapshot (AC-26); a corrupt snapshot reports and offers abandon.
- Two changes queued for the same player (tactics role change and substitution): the substitution wins; the role change is discarded with a reason.
- Substitution queued while the substitute is injured or already on the pitch: rejected with a reason.
- Red card leaves a team with ten: the AI manager reshapes; the human's formation editor shows ten slots.
- Extra time fatigue: curves extend to 120 minutes; substitution windows gain the extra-time allowance from the rule pack.
- 8x playback with many events per second: the feed batches inserts per frame; no dropped events.
- Long replay memory: compact encoding; if the budget is exceeded, the viewer warns and keeps the last 45 minutes.
- Data file missing at startup: engine reports the path and exits non-zero; the viewer shows the first-run state.
- Seed collision across matches: identifiers include the match identifier, not the seed alone.

## Affected Areas

The repository is empty. Research sub-agent 1 confirmed no manifests, source, tests, docs, PRODUCT.md, or DESIGN.md. Conventions that plan must establish before source lands: naming, error handling across the engine and viewer boundary, configuration and where match parameters live, structured logging aligned with the observability pipeline, and test layout. Toolchains present on the reference machine: Rust 1.92 with cargo, Node 22 with npm and pnpm, .NET 10, Go 1.25, Python 3.12. Absent: a C or C++ compiler, Zig, Playwright.

## Dependencies / Sequencing Notes

- Engine core with a headless command line and the benchmark harness comes first; every other slice consumes its tick stream.
- The team generator ships with or before the first calibration run (AC-3, AC-4 need it).
- The tick-stream protocol and a recorded fixture stream are a prerequisite harness for viewer slices (see Verification Strategy, force-scope).
- The observability pipeline is a sibling deliverable: run `/wf observability init` in parallel with slice so the event-stream schema is agreed before the engine slice writes it.
- Playwright enters the repository once the viewer has stable markup (Q29).
- The scripting runtime is a named later slice that clears RIM-6; slice must create it as a slice, not a note.
- Distribution and installer work is a later slice (U-3).
- Feature flags (experiment augmentation) arrive with the second calibration slice, when two competing models first exist.

## Questions Asked This Stage

- Q1 stoppage-gated changes — closes AMB-8
- Q2 lineup input — closes AMB-29
- Q3 tactics depth — closes AMB-13
- Q4 AI opponent — closes AMB-10
- Q5 stream ticks live — closes AMB-8, AMB-20
- Q6 determinism — closes AMB-7
- Q7 tick rate — closes AMB-6
- Q8 playback controls — closes AMB-9
- Q9 engine home — closes AMB-1, AMB-27, AMB-31
- Q10 match screen — closes AMB-17
- Q11 states — closes AMB-18
- Q12 team data — closes AMB-11
- Q13 register — closes AMB-19
- Q14 color and scene — closes AMB-19
- Q15 references — closes AMB-19
- Q16 state weight — closes AMB-19
- Q17 engine crash — closes AMB-22
- Q18 engine lag — closes AMB-22
- Q19 substitution windows — closes AMB-38
- Q20 history — closes AMB-21
- Q21 rules in v1 — closes AMB-16, AMB-30, AMB-38
- Q22 commentary — closes AMB-14
- Q23 project type — closes AMB-4, AMB-26
- Q24 augmentations — closes AMB-25
- Q25 language — closes AMB-2
- Q26 attributes — closes AMB-12
- Q27 multi-user — closes AMB-3
- Q28 customize — closes AMB-23
- Q29 verification tooling — closes AMB-28
- Q30 license — closes AMB-32
- Q31 modding timing — closes AMB-23, AMB-34
- Q32 seeded RNG — confirms AMB-7

## Answers Captured This Stage

All 32 answers are in `po-answers.md` under the `shape` stage entries with scope lines. Load-bearing answers: native Rust engine on the user's machine (Q9, Q25); live tick stream at 50 per second (Q5, Q7); changes apply at the next qualifying stoppage with real-life substitution windows (Q1, Q19); best-effort determinism with a seedable RNG (Q6, Q32); whole-match rewind (Q8, Q20); fictional generated teams with a data-driven 1 to 100 attribute schema (Q12, Q26); all four laws groups and context-aware commentary (Q21, Q22); open source under a permissive license (Q23, Q30); plugin boundary now, scripting later (Q28, Q31); identity reserved for later multiplayer (Q27); benchmark, instrument, and experiment augmentations (Q24); in-app browser now, Playwright later (Q29).

## Out of Scope

- Leagues, seasons, competitions, transfers, finances, squad building beyond lineup selection: intake out-of-scope, unchanged.
- Networked multiplayer and accounts: Q27 keeps v1 single-player; identity fields are reserved.
- Scripting runtime: Q31 sequences it as a later slice; it stays in the workflow as a named slice (RIM-6).
- Injury persistence across matches: no season model exists yet (AMB-15).
- VAR and officiating aids: not selected in Q21.
- Real club and player names: Q12 chose fictional data.
- 3D rendering and mobile clients: intake out-of-scope, unchanged.
- Operating systems other than Windows for packaging: parked (U-3).

## Intake Fidelity

| Intake directive | Disposition | How | Authority |
|---|---|---|---|
| Frontend is HTML, CSS, and JavaScript | honored | The viewer is a static page; the engine speaks to it over a local socket | — |
| Engine language open; performance, maintainability, depth, customizability | honored | Rust native process; separate packages; data-driven schemas | Q25 "Rust"; Q9 |
| Per-tick positional data (C2) | honored | 50 ticks per second; tick-bounded interpolation | Q7 "50 ticks per second" |
| Engine observable through a pipeline (C4) | honored | Event stream and statistics record (AC-17); instrument augmentation | Q24 "Instrument for observability" |
| Session tooling exclusions and inclusions | honored | No consult, zread, or web-search-prime used; design skills recorded | — |
| Dedicated branches on main | honored | Unchanged | — |
| Step 1: manager selects lineup and tactics | honored | Eleven plus bench; Tier 2 tactics | Q2, Q3 |
| Step 2: match against a computer team | honored | Reactive AI manager | Q4 |
| Step 3: engine simulates tick by tick with positions, events, statistics | honored | AC-1, AC-11, AC-17 | Q7 |
| Step 4: 2D viewer replays with commentary at adjustable speed | honored | 1x to 8x, pause, skip, rewind; context-aware commentary | Q8, Q22 |
| Step 5: in-match tactics changes, substitutions, fatigue, injuries | honored | Stoppage-gated queue; fatigue curves; injuries | Q1, Q19, Q21 |
| Step 6: result, statistics report, structured output | honored | Full-time report, saved replay, event stream | Q20 |
| "Customizable" priority (full modding) | narrowed for v1 | Plugin boundary, data files, and rule packs ship first; the scripting runtime is a named later slice | Q31 "Boundary now, scripts later: the first deliverable exposes tuning constants, team data, and rule packs as data files and defines the plugin interface; a scripting runtime is its own later slice." |

## Definition of Done

- All 29 acceptance criteria pass in their named classes, with evidence captured per the Verification Strategy.
- The Charter Scenario runs end to end on the reference machine, observed in the browser.
- The six charter commitments are cited by at least one passing acceptance criterion each.
- The benchmark harness reports AC-19 numbers in the repository.
- The event stream and statistics record validate against the schema agreed with the observability plan.
- The scripting-runtime slice exists as an open slice in the workflow; the status of "customizable" reads "boundary shipped, runtime open" until that slice closes.
- Documentation per the Documentation Plan exists for every shipped surface.

## Verification Strategy

**Target verification environment.** Windows 11 Pro (reference laptop), PowerShell and Git Bash. Rust 1.92 with cargo for engine tests and benchmarks. Node 22 for the viewer build and, later, Playwright. Interactive driver now: the in-app browser pane with screenshots and console capture (Q29). Interactive driver later: Playwright in the repository once viewer markup is stable. No device, emulator, credentials, or external service is needed.

**Observation Model.**

- Engine realism (AC-3, AC-4): observed by running the headless calibration command over 1000 seeded matches and reading the aggregate statistics record.
- Positional sanity (AC-5, AC-6): observed by a tick-stream validator run over every calibration match.
- Stoppage-gated changes (AC-7, AC-8): observed by scripted headless matches that inject changes at known ticks and assert the applying tick from the event stream.
- Crash recovery (AC-14, AC-26): observed by killing the engine process during a driven match in the browser and asserting resume from the snapshot; the induced failure is the evidence.
- Viewer frame rate and fidelity (AC-20 to AC-24): observed by driving the page in the browser, reading a frame-time counter the viewer exposes, and comparing rendered positions to the stored ticks through a test hook.
- Performance (AC-19): observed by the benchmark harness on the reference laptop, single thread, with the machine name and build hash recorded.
- Legibility and focus (AC-29): observed by a human at 1280 by 800.

**Force-scope rule applied.** Viewer slices depend on an engine that does not yet exist. The dependency is routed into scope as a **candidate prerequisite harness for slice**: a recorded tick-stream fixture (one full match) plus a mock socket server that replays it. Viewer criteria verify against the fixture before the engine is complete; the residual (viewer against the live engine) clears at the Charter Scenario run in the integration slice. Performance (AC-19) is an outcome metric with a pre-deploy proxy: the benchmark harness in the first engine slice; the residual clears when the integration slice runs the benchmark on the full engine.

**Automated checks:** AC-1 to AC-19, AC-21, AC-28; every engine edge case; data-file validation.

**Interactive verification:** AC-20, AC-22 to AC-27 in the browser with screenshots and, for AC-26, a recording. Platform: web; tool: in-app browser now, Playwright later; evidence under the verify artifact's evidence layout.

**Human-in-the-loop checks:** AC-29 legibility and focus; the goal-moment treatment against the design contract.

## Documentation Plan

- **README update** — Audience: contributors and players. Must cover: what the project is, how to run the engine and open the page, license. Must not cover: engine internals. Target: `README.md`.
- **Tutorial: play your first match** — Audience: beginner player. Must cover: generate a team, pick a lineup, kick off, make a substitution, save the replay. Must not cover: modding. Target: `docs/tutorials/first-match.md`.
- **How-to: tune the engine and add a rule pack** — Audience: competent modder. Must cover: tuning file, attribute schema, team data, rule pack files, validation errors. Must not cover: the scripting runtime (later). Target: `docs/how-to/modding.md`.
- **Reference: tick-stream protocol, data-file schemas, command line** — Audience: maintainer and modder. Must cover: every message and field, every schema, every flag. Must not cover: rationale. Target: `docs/reference/`.
- **Explanation: engine architecture and determinism stance** — Audience: maintainer. Must cover: agent model, tick loop, stoppage-gated queue, snapshot, seedable RNG, why best-effort determinism. Must not cover: step-by-step tasks. Target: `docs/explanation/engine.md`.

## Augmentation Plan

- **benchmark** — Measure ticks per second and 90-minute wall time, single thread, on the reference laptop; budget per NFR-1; tripwires: more than 10 percent CPU regression or more than 25 percent memory regression between slices. Artifact `05c-benchmark.md` per engine slice.
- **instrument** — Signals: event stream (one record per event with tick, minute, type, actors), statistics record per match, fatigue and injury counters, AI-manager decisions, substitution rejections, snapshot writes, engine lag notices. Dark paths: a queued change that never applies; a match that ends without a statistics record; a viewer that drops ticks silently. Plan folds signal design in; implement wires it; `/wf observability init` owns the pipeline. Artifact `04b-instrument.md`.
- **experiment** — Hypothesis: a candidate decision or shot model changes realism statistics; mechanism: feature flags in the tuning file, evaluated by paired calibration runs; metrics: AC-3 and AC-4 bands; rollback: flag off. Arrives with the second calibration slice. Artifact `04c-experiment.md`.

## Freshness Research

- Source: [FootballEngine (F#, GPLv3)](https://github.com/DeltaBitsSystem/FootballEngine)
  Why it matters: confirmed 40 Hz stepper, per-player perception, cognition, and steering, ball physics with drag, friction, restitution, and Magnus spin.
  Takeaway: a proven per-tick agent architecture; GPLv3 means read, do not copy (NFR-5).
- Source: [RoboCup 2D soccer server and monitor](https://rcsoccersim.readthedocs.io/en/latest/soccerserver.html)
  Why it matters: 100 ms cycle; server streams state every cycle to attached monitors.
  Takeaway: the server-monitor split is the local socket server shape; the tick contract is a reference.
- Source: [Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/) and [Deterministic Lockstep](https://gafferongames.com/post/deterministic_lockstep/)
  Why it matters: decouple simulation step from render frame; ship seeds and inputs for determinism.
  Takeaway: tick-bounded interpolation and the seedable RNG follow these directly.
- Source: [Opta 2025-26 Premier League facts](https://theanalyst.com/articles/best-premier-league-facts-of-the-2025-26-season-opta), [FBref xG](https://fbref.com/en/expected-goals-model-explained/)
  Why it matters: about 2.75 goals per match and 9.5 shots per team per match in a top league.
  Takeaway: the AC-3 bands hold; shots per team band is 8 to 16.
- Source: [Metrica Sports sample tracking data](https://github.com/metrica-sports/sample-data)
  Why it matters: open tracking format normalized to a 0 to 1 pitch coordinate system.
  Takeaway: a candidate coordinate convention for the tick stream and the replay file.
- Source: [StatsBomb open data](https://github.com/statsbomb/open-data)
  Why it matters: free calibration data with an attribution requirement.
  Takeaway: if used for calibration, attribute in public outputs.
- Source: [Sports Interactive licensing](https://community.sports-interactive.com/sigames-manual/football-manager-2023/legal-r4739/)
  Why it matters: real names need licenses; the Football Manager mark is protected.
  Takeaway: fictional data (Q12); keep the mark out of the product name.
- Source: [Rust and WebAssembly in 2026](https://dev.to/dataformathub/rust-wasm-in-2026-a-deep-dive-into-high-performance-web-apps-20c6), [WASM threads support](https://www.testmuai.com/learning-hub/wasm-threads-browser-support/)
  Why it matters: a later browser build of the Rust engine is viable; threads need cross-origin isolation headers.
  Takeaway: not a v1 concern after Q9; recorded for the future browser option.
- Source: [OffscreenCanvas support](https://www.testmuai.com/learning-hub/offscreencanvas-browser-support/)
  Why it matters: mature in Chrome, Edge, Firefox, and Safari 16.4 and later.
  Takeaway: off-main-thread rendering is available for NFR-2.
- Openfoot Manager could not be re-verified this session (site and guessed repository path failed); its intake claims are unconfirmed and are not load-bearing.

## Recommended Next Stage

- **Option A (default):** `/wf slice football-manager-match-engine` — the spec spans an engine core, a protocol and fixture harness, a team generator, a calibration harness, a viewer with four weighted states, an AI manager, and a later scripting-runtime slice; it needs decomposition and slice confirms review scope.
- **Option E (in the pipeline):** the design brief `02b-design.md` is authored; plan resolves the image gate and writes `02c-craft.md`; no separate design command is needed.
- **In parallel:** `/wf observability init` — agree the event-stream schema before the engine slice writes it.
- Option B (skip to plan) does not apply: the work is multi-slice.
- Option C and D do not apply: no intake misread surfaced, and no required answer is missing.

## Unknowns / Open Questions

- **U-1** Wire encoding for the tick stream (binary delta versus JSON lines) — receiving stage: plan (protocol slice), guided by NFR-2 and NFR-3.
- **U-2** Observability sink and transport for the event stream (file, socket, both) — receiving stage: `/wf observability init`.
- **U-3** Operating systems and packaging for the native engine beyond Windows — receiving stage: slice (distribution slice), then the product owner.
