---
schema: sdlc/v1
type: intake
slug: football-manager-match-engine
status: complete
stage-number: 1
created-at: "2026-09-21T19:09:35Z"
updated-at: "2026-09-21T19:09:35Z"
tags: [game, simulation, match-engine, 2d-viewer, greenfield]
refs:
  index: 00-index.md
  next: 02-shape.md
next-command: wf-shape
next-invocation: "/wf shape football-manager-match-engine"
consult-runs: []
---

# Intake

## The Intake

The request arrived in an empty repository with no git history and no stack. The product owner wants a football manager game with an HTML, CSS, and JavaScript frontend, and a match engine in whatever language performs best. The first deliverable is the match engine and a 2D match viewer.

Five substance questions and three process questions fixed the shape of the work. The product owner confirmed the full feature floor: attributes, tactics, per-tick positions, realistic statistics, fatigue, injuries, substitutions, set pieces, and commentary. The product owner ratified six charter commitments and accepted four metric families as the starting success criteria. Appetite is large, branch strategy is dedicated, and the base branch is main. Four questions stay open: the engine runtime location, the meaning of "multiple users", the final metric thresholds, and the project type and timeline.

Shape comes next. Shape must choose the engine language and runtime location, derive the charter scenario from the core loop, and adjudicate four intent risks. The top risk is RIM-1: an engine built on minute-by-minute event probabilities instead of continuous per-tick positions would satisfy the statistics but break the 2D viewer and the ratified commitment C2.

## Restated Request

Build a football manager game. The frontend is HTML, CSS, and JavaScript. The match engine and game engine may use any language; the choice favors performance, maintainability, depth, and customizability. The first deliverable is a detailed match engine in the style of Football Manager, plus a 2D match viewer that replays the engine output.

Core loop, first deliverable:

1. The manager selects a lineup and tactical instructions for a modeled football team.
2. The manager starts a match against a computer-managed team.
3. The engine simulates the match tick by tick: ball position, 22 player positions, events, and statistics.
4. The 2D viewer replays the match, with commentary, at adjustable speed.
5. During the match, the manager changes tactics and substitutes players. Fatigue and injuries change player performance.
6. The match ends with a result, a statistics report, and structured event and metric output for the observability pipeline.

## Intended Outcome

A realistic and replayable match experience: a manager's tactical and lineup choices change what happens on the pitch, and the pitch view shows continuous movement that the engine computed.

## Primary User / Actor

- A human manager who selects a lineup and tactics, then plays against a computer-managed team. The product owner described "multiple users"; whether they play concurrently or in independent sessions is open (OQ-3).
- Secondary: the developer who tunes the engine through the observability pipeline.

## Affected Areas (preliminary)

Skipped. The repository is empty; there is no codebase area to map. Shape's research starts from the freshness research below.

## Known Constraints

- Frontend technology is fixed: HTML, CSS, and JavaScript.
- The engine language is open; performance, maintainability, depth, and customizability are the stated priorities.
- The engine output must feed a 2D viewer, so per-tick positional data is a hard requirement (C2).
- The engine must be observable: a debug observability pipeline is planned through `/wf observability`.
- Session tooling excluded by the product owner: `zread`, `web-search-prime`, `consult`. Included: `artifact-design`, `artifact-diagramming`, `artifact-capabilities`.
- Git: dedicated feature branches, base branch `main`, created by the first commit.

## Assumptions

- Tactics and substitution are first-class gameplay, not settings hidden from the manager.
- The engine's output is a data contract that the viewer consumes; the viewer computes no game logic.
- The rest of the manager game (leagues, transfers, finances, squad building) comes later and must not be precluded by the engine's data model.
- "Customizable" covers both developer tuning (data-driven configuration) and, later, user-facing tactical options.

## Product Owner Questions Asked

1. Outcome and audience: who plays, and what is the first playable moment?
2. What "like FM" means: which engine features must the first version have?
3. Success criteria: how do you know the engine is good enough?
4. Constraints and decided choices: runtime location, language preference, timeline, project type?
5. Stack confirmation: is the detected stack and session tooling correct?
6. Branch strategy (gate).
7. Appetite (gate).
8. Base branch (gate).
9. Charter ratification (gate, multi-select).
10. Initial success metrics (gate, multi-select).

## Product Owner Answers

- Q1: Multiple users choose a lineup and play against a computer team. Teams are modeled. Tactics and substitution are important.
- Q2: All of attributes, tactics, per-tick positions, realistic statistics, fatigue, injuries, substitutions, set pieces, commentary, and more.
- Q3: A reasonably good simulation, measured through an observability pipeline. The PO asked what to monitor; the four metric families in Success Criteria answer that and were accepted.
- Q4: No preferences. Priorities: performance, maintainability, depth, customizable.
- Q5: Stack updates as decisions land. Exclude zread, web-search-prime, consult. Include the Claude design artifact skills.
- Q6: Dedicated. Q7: Large. Q8: main.
- Q9: C1–C6 all ratified. Q10: all four metric families accepted.

Full log with scope lines: `po-answers.md`.

## Unknowns / Open Questions

- **OQ-1** Engine runtime location: inside the browser (WebAssembly), a local native process, or a server. Decides the language shortlist and the frontend integration. Shape resolves.
- **OQ-2** Final metric thresholds: the starting values in Success Criteria are proposals. Shape and the observability plan refine them.
- **OQ-3** "Multiple users": concurrent multiplayer, or independent single-player sessions. Decides whether a server and accounts exist in the first deliverable.
- **OQ-4** Project type (personal, open source, commercial) and timeline. Decides licensing constraints on dependencies and the delivery cadence.

## Dependencies / External Factors

- No repository dependencies exist yet.
- Reference engines found in freshness research inform shape's design research; none is a dependency.
- The observability pipeline (`/wf observability init`) is a sibling deliverable that the engine's event output must feed.

## Risks if Misunderstood

- **RIM-1** (severity: high) — "Like FM" is read as an event-probability simulation, minute by minute, instead of a continuous per-tick positional simulation. Statistics would look right, the 2D viewer would have nothing real to draw, and C2 would break.
- **RIM-2** (severity: medium) — "Multiple users" is read as real-time multiplayer, pulling a server, accounts, and networking into the first deliverable. The PO described managers playing against a computer team; concurrency is unconfirmed (OQ-3).
- **RIM-3** (severity: medium) — "The engine does not need to be JS" is read as freedom to ignore browser delivery. The frontend is a web page, so the engine must reach the browser through WebAssembly, a local process, or a server (OQ-1). An engine that cannot reach the browser is unusable.
- **RIM-4** (severity: medium) — "To start we need the match engine and viewer" is read as the whole product. The engine's data model must leave room for squads, leagues, and seasons, or the manager game later requires a rewrite.
- considered: "customizable" means end-user modding rather than developer tuning — dismissed because both readings lead to a data-driven engine configuration; shape decides how far the customization surface reaches in the first deliverable.

## Charter

- **C1** — A human manager picks a lineup and tactics, then plays a complete match against a computer-managed team. — source: `01-intake.md#restated-request`
- **C2** — The engine computes the ball and all 22 player positions every tick, and the 2D viewer replays those positions without inventing movement. — source: `01-intake.md#restated-request`
- **C3** — Player attributes and tactical instructions change outcomes: over many matches, a stronger or better-set-up team wins more often. — source: `01-intake.md#intended-outcome`
- **C4** — The engine runs headless behind a defined data contract, independent of the frontend, and emits structured events and statistics for the observability pipeline. — source: `01-intake.md#known-constraints`
- **C5** — The manager substitutes players during the match, and fatigue and injuries change player performance during the match. — source: `01-intake.md#restated-request`
- **C6** — The engine produces set pieces (kick-offs, throw-ins, corners, free kicks, penalties) and text commentary from match events. — source: `01-intake.md#restated-request`

All six ratified by the PO on 2026-09-21T19:09:35Z.

## Success Criteria

Starting thresholds, accepted by the PO; shape and the observability plan refine them.

- **Realism statistics** (over 1000 simulated matches): goals per match 2.4 to 3.2; shots per team 8 to 16; possession split within 35 to 65 percent for evenly matched teams; a team with 15 percent better attributes wins over 50 percent of matches.
- **Performance**: a full 90-minute match simulates headless in under 2 seconds on a laptop; 1000 matches complete in under 30 minutes.
- **Viewer fidelity**: the 2D viewer replays a match at 1x to 8x speed at 60 frames per second without dropped ticks; every goal in the replay matches the engine's event log.
- **Positional sanity**: no player leaves the pitch bounds; no two players occupy the same point; ball speed never exceeds 40 m/s; formation shape is kept when the ball is far away.
- **Observability**: every match emits a structured event stream and an aggregate statistics record that the pipeline can ingest without transformation.

## Out of Scope for Now

- League, season, and competition management.
- Transfers, contracts, finances, and squad building beyond lineup selection.
- Multiplayer networking (pending OQ-3).
- 3D rendering.
- Mobile-native clients.

## Freshness Research

- Source: [FootballEngine (F#, WIP)](https://github.com/DeltaBitsSystem/FootballEngine)
  Why it matters: an open-source engine with a 40 Hz match stepper, ball physics, and per-player perception, cognition, and steering agents; the closest public architecture to C2.
  Takeaway: a per-tick agent-based stepper at 25 to 50 Hz is a proven shape for continuous positional simulation.
- Source: [Openfoot Manager (Rust + React)](https://openfootmanager.com/)
  Why it matters: an open-source manager game with a Rust engine and a web frontend; its simulation is zone-based and minute-by-minute with 22 event types.
  Takeaway: this is exactly the RIM-1 shape; it proves the Rust-plus-web split but does not meet C2.
- Source: [AgentPitch (May 2026)](https://taogang.medium.com/agentpitch-when-the-llm-writes-the-tactics-1bf07ace1b2a)
  Why it matters: a per-tick sandbox with a 2D viewer styled after classic Championship Manager.
  Takeaway: the 2D-dots viewer over a tick stream is the expected presentation for this genre.
- Source: [RoboCup 2D Soccer Simulation](https://arxiv.org/pdf/1612.00947)
  Why it matters: twenty years of open-source 2D simulation with a 100 ms step and a visualization tool.
  Takeaway: the RoboCup server and monitor split matches C4 (headless engine, separate viewer) and offers a mature reference for the tick contract.
- Source: [ai-football-agentic (Python)](https://github.com/marciodearagao/ai-football-agentic)
  Why it matters: a small engine whose SVG 2D field is replaceable without touching the engine.
  Takeaway: keep the viewer replaceable behind the data contract.

## Recommended Next Stage

- **Option A (default):** `/wf shape football-manager-match-engine` — four open questions and four intent risks need adjudication; the engine language and runtime location are shape decisions; the frontend is a web UI layer, so shape also authors the design brief for the 2D viewer.
- **Option B:** `/wf observability init` — the PO named the observability pipeline as the measure of success; running it alongside shape gives shape a concrete metric plan to cite. It does not replace shape.
- **Option C:** Not blocked. All required questions were answered; the open questions are shape's to resolve.
