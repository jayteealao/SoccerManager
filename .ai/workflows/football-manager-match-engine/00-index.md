---
schema: sdlc/v1
type: index
slug: football-manager-match-engine
title: "Football manager game: match engine and 2D match viewer"
status: active
current-stage: implement
stage-number: 5
created-at: "2026-09-21T16:46:41Z"
updated-at: "2026-09-21T22:35:04Z"
selected-slice: "engine-core"
branch-strategy: dedicated
branch: "feat/football-manager-match-engine"
base-branch: "main"
review-scope: slug-wide
review-scope-confirmed: true
appetite: large
pr-url: ""
pr-number: 0
open-questions:
  - "U-1 Wire encoding for the tick stream (binary delta versus JSON lines) — plan"
  - "U-2 Observability sink and transport for the event stream — /wf observability init"
  - "U-3 Operating systems and packaging beyond Windows — slice, then the product owner"
tags: [game, simulation, match-engine, 2d-viewer, rust, greenfield]
stack:
  detected-at: "2026-09-21T16:46:41Z"
  platforms: [web, cli]
  languages: [rust, javascript]
  ui: [html, css, javascript]
  build: [cargo]
  package-managers: [cargo, npm]
  testing: [cargo-test]
  observability: []
  integrations: []
  available-skills:
    - {name: frontend-design, hint: "Distinctive production-grade HTML/CSS/JS interfaces"}
    - {name: artifact-design, hint: "Design guidance for Artifact pages (PO-requested)"}
    - {name: artifact-diagramming, hint: "Diagrams for Artifact pages (PO-requested)"}
    - {name: artifact-capabilities, hint: "Runtime capabilities for Artifact pages (PO-requested)"}
    - {name: study-sources, hint: "Read upstream source of dependencies into .scratch/"}
    - {name: uiproto, hint: "UI prototyping"}
    - {name: tech-research-enforcer, hint: "Web research before library or version claims"}
    - {name: dataviz, hint: "Charts and data visualization"}
    - {name: code-review, hint: "Correctness review of a diff"}
    - {name: security-review, hint: "Security review of pending changes"}
  available-mcp:
    - {name: web-reader, hint: "Fetch and read web pages"}
    - {name: openrouter, hint: "Multi-model access"}
    - {name: Claude_Browser, hint: "In-app browser for previewing dev servers; chosen interactive driver (Q29)"}
    - {name: pencil, hint: "Design files (.pen) editor"}
    - {name: zai-mcp-server, hint: "Image, video, and screenshot analysis"}
  excluded-by-po: [zread, web-search-prime, consult]
  toolchains-present: ["rustc 1.92.0", "cargo", "node v22.15.0", "npm 10.9.2", "dotnet 10.0.102", "go 1.25.5", "python 3.12.10"]
  toolchains-absent: [clang, cl, zig, playwright]
  user-confirmed: true
intent-risks:
  - id: RIM-1
    risk: "\"Like FM\" is read as a minute-by-minute event-probability simulation instead of continuous per-tick positions; the 2D viewer would have nothing real to draw and C2 breaks."
    severity: high
    status: adjudicated
    adjudicated-by: "02-shape.md#desired-behavior"
    decision: "Continuous per-tick positional simulation at 50 ticks per second, streamed live to the viewer. Tradeoff: higher compute and stream size than an event-probability engine; NFR-1 yields to C2, so a performance miss reopens the tick rate with the PO instead of lowering the model silently."
    po-ratified: true
    po-answer: "po-answers.md · shape Round 2 Q7; Round 2 Q5"
  - id: RIM-2
    risk: "\"Multiple users\" is read as real-time multiplayer, pulling a server, accounts, and networking into the first deliverable."
    severity: medium
    status: adjudicated
    adjudicated-by: "02-shape.md#out-of-scope"
    decision: "V1 is independent single-player sessions; every match record and replay carries owner and match identifiers (AC-18) so a server can be added later. Tradeoff: identity fields ride along unexercised in v1."
    po-ratified: true
    po-answer: "po-answers.md · shape Extension round 1 Q27"
  - id: RIM-3
    risk: "\"The engine does not need to be JS\" is read as freedom to ignore browser delivery; an engine that cannot reach the web frontend is unusable."
    severity: medium
    status: adjudicated
    adjudicated-by: "02-shape.md#desired-behavior"
    decision: "Native Rust process on the user's machine serving a local socket to the page. Tradeoff: users install a program and v1 is desktop only; the same Rust code keeps a later WebAssembly build open."
    po-ratified: true
    po-answer: "po-answers.md · shape Round 3 Q9; Extension round 1 Q25"
  - id: RIM-4
    risk: "\"To start we need the match engine and viewer\" is read as the whole product; the engine data model leaves no room for squads, leagues, and seasons."
    severity: medium
    status: adjudicated
    adjudicated-by: "02-shape.md#desired-behavior"
    decision: "Team data, rule packs, and the attribute schema are data files with identifiers for club, squad, and player; match records carry owner and match identifiers; leagues and seasons stay out of scope but reference these identifiers later. Tradeoff: schema work in v1 for entities v1 does not use."
    po-ratified: true
    po-answer: "po-answers.md · shape Round 3 Q12; Extension round 1 Q26, Q27"
  - id: RIM-5
    risk: "Viewer interpolation between engine ticks renders movement the engine never computed, satisfying C2 in letter and not in spirit (pre-mortem 1)."
    severity: medium
    status: adjudicated
    adjudicated-by: "02-shape.md#desired-behavior"
    decision: "Tick-bounded interpolation: every rendered frame lies on the segment between two consecutive ticks, never beyond the next tick; at speeds where ticks exceed frames the viewer skips ticks (AC-21). Tradeoff: slightly less smooth motion at 8x than free interpolation."
    po-ratified: not-required
  - id: RIM-6
    risk: "The scripting runtime deferred by Q31 never gets scheduled and \"customizable\" is reported done on the strength of the plugin interface alone (pre-mortem 2)."
    severity: medium
    status: adjudicated
    adjudicated-by: "02-shape.md#dependencies-sequencing-notes"
    decision: "The scripting runtime is a named later slice that slice must create; the Definition of Done reads \"boundary shipped, runtime open\" until that slice closes. Tradeoff: the workflow stays open longer than the first release."
    po-ratified: true
    po-answer: "po-answers.md · shape Extension round 2 Q31"
charter:
  - id: C1
    commitment: "A human manager picks a lineup and tactics, then plays a complete match against a computer-managed team."
    source: "01-intake.md#restated-request"
    status: honored
    po-ratified: true
  - id: C2
    commitment: "The engine computes the ball and all 22 player positions every tick, and the 2D viewer replays those positions without inventing movement."
    source: "01-intake.md#restated-request"
    status: honored
    po-ratified: true
  - id: C3
    commitment: "Player attributes and tactical instructions change outcomes: over many matches, a stronger or better-set-up team wins more often."
    source: "01-intake.md#intended-outcome"
    status: honored
    po-ratified: true
  - id: C4
    commitment: "The engine runs headless behind a defined data contract, independent of the frontend, and emits structured events and statistics for the observability pipeline."
    source: "01-intake.md#known-constraints"
    status: honored
    po-ratified: true
  - id: C5
    commitment: "The manager substitutes players during the match, and fatigue and injuries change player performance during the match."
    source: "01-intake.md#restated-request"
    status: honored
    po-ratified: true
  - id: C6
    commitment: "The engine produces set pieces (kick-offs, throw-ins, corners, free kicks, penalties) and text commentary from match events."
    source: "01-intake.md#restated-request"
    status: honored
    po-ratified: true
slices:
  - {slug: engine-core, status: complete, complexity: l, depends-on: []}
  - {slug: data-schemas-generator, status: defined, complexity: m, depends-on: [engine-core]}
  - {slug: stream-protocol, status: defined, complexity: m, depends-on: [engine-core]}
  - {slug: viewer-pitch, status: defined, complexity: m, depends-on: [stream-protocol]}
  - {slug: match-rules, status: defined, complexity: l, depends-on: [engine-core, data-schemas-generator]}
  - {slug: tactics-and-ai, status: defined, complexity: l, depends-on: [match-rules, data-schemas-generator]}
  - {slug: commentary, status: defined, complexity: s, depends-on: [match-rules]}
  - {slug: calibration, status: defined, complexity: m, depends-on: [tactics-and-ai, data-schemas-generator]}
  - {slug: viewer-match-day, status: defined, complexity: m, depends-on: [viewer-pitch, stream-protocol, commentary]}
  - {slug: viewer-lineup-tactics, status: defined, complexity: l, depends-on: [viewer-match-day, tactics-and-ai]}
  - {slug: viewer-reports-recovery, status: defined, complexity: m, depends-on: [viewer-match-day, match-rules]}
  - {slug: integration, status: defined, complexity: m, depends-on: [calibration, commentary, viewer-lineup-tactics, viewer-reports-recovery]}
  - {slug: extra-time-penalties, status: defined, complexity: s, depends-on: [match-rules, tactics-and-ai], deferred: true}
  - {slug: experiment-flags, status: defined, complexity: s, depends-on: [calibration], deferred: true}
  - {slug: scripting-runtime, status: defined, complexity: l, depends-on: [data-schemas-generator, tactics-and-ai, calibration], deferred: true}
  - {slug: distribution, status: defined, complexity: m, depends-on: [integration], deferred: true}
augmentations:
  - {type: instrument, artifact: 04b-instrument.md, status: ready, created-at: "2026-09-21T21:57:49Z"}
  - {type: benchmark, artifact: 05c-benchmark.md, mode: baseline, status: ready, created-at: "2026-09-21T21:57:49Z"}
  - {type: experiment, artifact: 04c-experiment.md, status: deferred-to-experiment-flags, created-at: "2026-09-21T21:57:49Z"}
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine engine-core"
workflow-files:
  - 00-index.md
  - 01-intake.md
  - 01-intake.01-boundary.html.fragment
  - 02-shape.md
  - 02-shape.01-change-queue.html.fragment
  - 02b-design.md
  - 02b-design.yaml
  - 02b-design.html.fragment
  - 03-slice.md
  - 03-slice.01-dependency-graph.html.fragment
  - 03-slice-engine-core.md
  - 03-slice-data-schemas-generator.md
  - 03-slice-stream-protocol.md
  - 03-slice-viewer-pitch.md
  - 03-slice-match-rules.md
  - 03-slice-tactics-and-ai.md
  - 03-slice-commentary.md
  - 03-slice-calibration.md
  - 03-slice-viewer-match-day.md
  - 03-slice-viewer-lineup-tactics.md
  - 03-slice-viewer-reports-recovery.md
  - 03-slice-integration.md
  - 03-slice-extra-time-penalties.md
  - 03-slice-experiment-flags.md
  - 03-slice-scripting-runtime.md
  - 03-slice-distribution.md
  - 04-plan.md
  - 04-plan-engine-core.md
  - 04-plan-engine-core.yaml
  - 04-plan-engine-core.html.fragment
  - 04b-instrument.md
  - 04b-instrument.yaml
  - 04b-instrument.html.fragment
  - 05c-benchmark.md
  - 05c-benchmark.yaml
  - 05c-benchmark.html.fragment
  - 05-implement.md
  - 05-implement-engine-core.md
  - po-answers.md
progress:
  intake: complete
  shape: complete
  slice: complete
  plan: complete
  implement: in-progress
  verify: not-started
  review: not-started
  handoff: not-started
  ship: not-started
  retro: not-started
---
