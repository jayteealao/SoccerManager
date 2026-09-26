---
schema: sdlc/v1
type: index
slug: football-manager-match-engine
title: "Football manager game: match engine and 2D match viewer"
status: active
current-stage: implement
stage-number: 5
created-at: "2026-09-21T16:46:41Z"
updated-at: "2026-09-25T22:45:37Z"
selected-slice: ""
branch-strategy: dedicated
branch: "feat/football-manager-match-engine"
base-branch: "main"
review-scope: slug-wide
review-scope-confirmed: true
appetite: large
pr-url: ""
pr-number: 0
open-questions:
  - "U-2 Observability sink and transport for the event stream — /wf observability init"
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
  - id: RIM-7
    risk: "\"Fix the ten-man issue\" is read as lowering the card rate so red cards rarely happen, which hides the defending bug instead of fixing it."
    severity: high
    status: adjudicated
    adjudicated-by: "03-slice-defending-and-discipline.md#acceptance-criteria"
    decision: "Fix the team shape and defending first; a controlled sending-off experiment with cards otherwise off is the criterion, and the card rate is fixed separately. Tradeoff: a larger engine change than a tuning tweak."
    po-ratified: true
    po-answer: "po-answers.md · extend round 1 Q-E2, Q-E3"
  - id: RIM-8
    risk: "\"Recalibrate against new bands\" is read as widening bands or tuning to one seed until the gate is green."
    severity: high
    status: adjudicated
    adjudicated-by: "03-slice-realism-tuning.md#risks"
    decision: "Band values are fixed by the product owner from sourced data; tuning must pass on five seeds; no band is widened without a recorded answer. Tradeoff: the tuning slice may stop and return to the product owner."
    po-ratified: true
    po-answer: "po-answers.md · extend round 1 Q-E1, Q-E4"
  - id: RIM-9
    risk: "\"Fix corners\" is read as awarding corners by a scripted rate instead of from the ball crossing the line after a defending touch."
    severity: high
    status: adjudicated
    adjudicated-by: "03-slice-keeper-and-shots.md#acceptance-criteria"
    decision: "Corners must arise from parries, blocks, deflections or clearances over the goal line; a criterion forbids a corner without a crossing. Tradeoff: corners depend on the keeper and block models being right."
    po-ratified: true
    po-answer: "po-answers.md · extend round 1 Q-E3"
  - id: RIM-10
    risk: "The ten-player fix is read as covering only a 4-4-2 side reduced to ten, leaving other formations broken at 11 against 11."
    severity: high
    status: adjudicated
    adjudicated-by: "03-slice-defending-and-discipline.md#acceptance-criteria"
    decision: "Every shipped formation (4-4-2, 4-3-3, 4-2-3-1, 3-5-2) is measured against 4-4-2 in a formations suite and must hold. Tradeoff: calibration takes longer."
    po-ratified: true
    po-answer: "po-answers.md · extend round 1 Q-E2"
  - id: RIM-11
    risk: "\"A fast tuning loop\" is read as permission to judge a slice on a targeted one-seed run, so a gain that is sampling noise is accepted and the full suites stop being run."
    severity: high
    status: adjudicated
    adjudicated-by: "03-slice-tuning-loop.md#scope"
    decision: "A targeted run is an inner loop only. The diff prints the sampling error and marks a change inside two errors as noise; slice gates still run the full suites on five seeds (one seed for formations, Q-I1). Tradeoff: a tuning slice still pays for one full gate run at its end."
    po-ratified: true
    po-answer: "po-answers.md · extend round 2 Q-X1, Q-X5"
  - id: RIM-12
    risk: "\"Fix the lone forward\" is read as lowering the card chance or the goal tuning, or scripting the forward never to dribble, so the red-card criterion passes while play stays unrealistic."
    severity: high
    status: adjudicated
    adjudicated-by: "03-slice-lone-forward.md#scope"
    decision: "Only two levers: the lone forward's decisions with no forward team-mate, and the won-tackle against foul odds. The card chance per foul (RIM-7), the keeper model and every limit stay fixed, and the pairings and discipline that pass now must still pass. Tradeoff: the slice may stop and return to the product owner if both levers at their bounds fall short."
    po-ratified: true
    po-answer: "po-answers.md · extend round 2 Q-X2, Q-X3"
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
  - id: C7
    commitment: "Over many matches, match statistics fall within sourced real-football bands (scorelines, cards, shots on target, passes, corners, throw-ins, goal kicks), for every shipped formation and after a sending-off."
    source: "03-slice-realism-bands-v2.md#goal"
    status: at-risk
    po-ratified: true
slices:
  - {slug: engine-core, status: complete, complexity: l, depends-on: []}
  - {slug: data-schemas-generator, status: complete, complexity: m, depends-on: [engine-core]}
  - {slug: stream-protocol, status: complete, complexity: m, depends-on: [engine-core]}
  - {slug: viewer-pitch, status: complete, complexity: m, depends-on: [stream-protocol]}
  - {slug: match-rules, status: complete, complexity: l, depends-on: [engine-core, data-schemas-generator]}
  - {slug: tactics-and-ai, status: complete, complexity: l, depends-on: [match-rules, data-schemas-generator]}
  - {slug: commentary, status: complete, complexity: s, depends-on: [match-rules]}
  - {slug: calibration, status: complete, complexity: m, depends-on: [tactics-and-ai, data-schemas-generator]}
  - {slug: viewer-match-day, status: complete, complexity: m, depends-on: [viewer-pitch, stream-protocol, commentary]}
  - {slug: viewer-lineup-tactics, status: complete, complexity: l, depends-on: [viewer-match-day, tactics-and-ai]}
  - {slug: viewer-reports-recovery, status: complete, complexity: m, depends-on: [viewer-match-day, match-rules]}
  - {slug: integration, status: complete, complexity: m, depends-on: [calibration, commentary, viewer-lineup-tactics, viewer-reports-recovery]}
  - {slug: extra-time-penalties, status: complete, complexity: s, depends-on: [match-rules, tactics-and-ai], deferred: true}
  - {slug: experiment-flags, status: complete, complexity: s, depends-on: [calibration], deferred: true}
  - {slug: scripting-runtime, status: complete, complexity: l, depends-on: [data-schemas-generator, tactics-and-ai, calibration], deferred: true}
  - {slug: distribution, status: complete, complexity: m, depends-on: [integration], deferred: true}
  - {slug: realism-bands-v2, status: complete, complexity: m, depends-on: [calibration, probe-engine-core]}
  - {slug: defending-and-discipline, status: complete, complexity: l, depends-on: [realism-bands-v2]}
  - {slug: tuning-loop, status: complete, complexity: m, depends-on: [realism-bands-v2]}
  - {slug: lone-forward, status: complete, complexity: l, depends-on: [tuning-loop, defending-and-discipline]}
  - {slug: keeper-and-shots, status: complete, complexity: l, depends-on: [defending-and-discipline]}
  - {slug: tempo-and-restarts, status: complete, complexity: m, depends-on: [keeper-and-shots]}
  - {slug: realism-tuning, status: skipped, complexity: m, depends-on: [tempo-and-restarts]}
augmentations:
  - {type: instrument, artifact: 04b-instrument.md, slice: match-rules, status: ready, created-at: "2026-09-22T19:29:33Z", prior: history/04b-instrument-3.md}
  - {type: benchmark, artifact: 05c-benchmark.md, slice: tempo-and-restarts, mode: baseline, status: ready, created-at: "2026-09-25T03:06:16Z", prior: history/05c-benchmark-8.md}
  - {type: experiment, artifact: 04c-experiment.md, slice: experiment-flags, status: ready, created-at: "2026-09-21T21:57:49Z"}
evidence-quality:
  live: 16
  headless: 40
  emulator-or-container: 2
  n-a: 61
metric-acceptance-mock-rung: 0
runtime-evidence-deferrals:
  - slice: viewer-match-day
    reason: "AC-7 human legibility reading only. Rungs tried: headless Microsoft Edge 153 at 1280 by 800 and 60 Hz over the DevTools protocol read the computed contrast of 201 visible text and number nodes (minimum 4.61:1, none under 4.5:1), confirmed no page scroll (scrollWidth 1280, scrollHeight 800), and tabbed through every control (8 controls plus the feed list, each :focus-visible with a 2px solid ring; screenshots ac7-focus-*.png). The pre-registered rung web-5 is a person reading the screen on the reference laptop; this run has no human operator. Probe: `grep -ciE \"legibility|read the match screen\" po-answers.md` -> `0` (no recorded reading), and the integration slice that hosts the reading is `status: defined`. Residual: the human judgement that the clock, score, and every statistics value read without zooming on the reference laptop."
    deferred-at: "2026-09-23T09:49:45Z"
    wall-ownership: external
    clearing-event: "The product owner reads the match screen on the reference laptop at 1280 by 800 during the integration slice's charter-scenario run and records the reading in po-answers.md; then /wf probe football-manager-match-engine or a re-verify of viewer-match-day records it."
    clearing-probe: "grep -ciE \"legibility|read the match screen\" .ai/workflows/football-manager-match-engine/po-answers.md"
    cleared-by: null
    needed-by: integration
  - slice: distribution
    reason: "macOS build scope row only (product owner OQ-1 = B: macOS is a pre-registered deferral). Rungs tried: (1) the platform-neutral proxy — packaging/unix/build.sh and smoke.sh ran on Linux x86_64 in WSL Ubuntu-24.04 this run, 7 of 7 checks twice, and the macOS branch of --open shares the start script; (2) cross-compile — `rustup target list --installed` -> `aarch64-linux-android armv7-linux-androideabi x86_64-linux-android x86_64-pc-windows-msvc` (no apple-darwin target), and `command -v xcrun ld64 ld64.lld zig clang` on Windows and `xcrun ld64 zig clang o64-clang` in WSL -> `not found` for every tool (no Apple SDK or linker); (3) a macOS runner — `ls .github/workflows` -> `No such file or directory` and the repository has no remote, so no CI runner exists; (4) a macOS VM — no Apple hardware, and macOS licensing binds it to Apple hardware. Residual: building, installing, and running the macOS archive on a Mac (build.sh, smoke.sh, and `open` launching the default browser)."
    deferred-at: "2026-09-23T18:35:00Z"
    wall-ownership: external
    clearing-event: "An operator provides a Mac or a macOS CI runner, runs `sh packaging/unix/build.sh` then `sh packaging/unix/smoke.sh dist/SoccerManager-<v>-macos-<arch>.tar.gz .ai/workflows/football-manager-match-engine/verify-evidence/distribution/macos`, then re-runs /wf verify football-manager-match-engine distribution or /wf probe football-manager-match-engine."
    clearing-probe: "test -f .ai/workflows/football-manager-match-engine/verify-evidence/distribution/macos/results.json"
    cleared-by: null
compressed-slices:
  - {slug: probe-engine-core, slice-type: probe, status: complete, created-at: "2026-09-22T06:03:50Z"}
next-command: wf-review
next-invocation: "/wf review football-manager-match-engine"
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
  - 04-plan-data-schemas-generator.md
  - 04-plan-data-schemas-generator.yaml
  - 04-plan-data-schemas-generator.html.fragment
  - 04b-instrument.md
  - 04b-instrument.yaml
  - 04b-instrument.html.fragment
  - 05c-benchmark.md
  - 05c-benchmark.yaml
  - 05c-benchmark.html.fragment
  - history/04-plan-0.md
  - history/04b-instrument-0.md
  - history/04b-instrument-0.yaml
  - history/04b-instrument-0.html.fragment
  - history/05c-benchmark-0.md
  - history/05c-benchmark-0.yaml
  - history/05c-benchmark-0.html.fragment
  - 05-implement.md
  - 05-implement-engine-core.md
  - 05-implement-data-schemas-generator.md
  - 06-verify.md
  - 06-verify-engine-core.md
  - 06-verify-engine-core.01-benchmark-drives.html.fragment
  - 06-verify-data-schemas-generator.md
  - 04-plan-stream-protocol.md
  - 04-plan-stream-protocol.yaml
  - 04-plan-stream-protocol.html.fragment
  - history/04-plan-1.md
  - history/04b-instrument-1.md
  - history/04b-instrument-1.yaml
  - history/04b-instrument-1.html.fragment
  - history/05c-benchmark-1.md
  - history/05c-benchmark-1.yaml
  - history/05c-benchmark-1.html.fragment
  - po-answers.md
  - 03-slice-probe-engine-core.md
  - 03-slice-realism-bands-v2.md
  - 03-slice-defending-and-discipline.md
  - 03-slice-keeper-and-shots.md
  - 03-slice-tempo-and-restarts.md
  - 03-slice-realism-tuning.md
  - 03-slice-tuning-loop.md
  - 03-slice-lone-forward.md
  - 04-plan-realism-bands-v2.md
  - 04-plan-realism-bands-v2.yaml
  - 04-plan-realism-bands-v2.html.fragment
  - 03-slice-probe-engine-core.01-dump-path.html.fragment
  - 05-implement-stream-protocol.md
  - 06-verify-stream-protocol.md
  - steer.md
  - 02c-craft.md
  - 02c-craft.yaml
  - 02c-craft.html.fragment
  - history/02c-craft-0.html.fragment
  - history/02c-craft-0.yaml
  - history/02c-craft-0.md
  - 04-plan-viewer-pitch.md
  - 04-plan-viewer-pitch.yaml
  - 04-plan-viewer-pitch.html.fragment
  - history/04-plan-2.md
  - history/04b-instrument-2.md
  - history/04b-instrument-2.yaml
  - history/04b-instrument-2.html.fragment
  - history/05c-benchmark-2.md
  - history/05c-benchmark-2.yaml
  - history/05c-benchmark-2.html.fragment
  - 05-implement-viewer-pitch.md
  - implement-evidence/viewer-pitch/drive.md
  - 06-verify-viewer-pitch.md
  - verify-evidence/viewer-pitch/
  - verify-evidence/viewer-pitch-run-2/
  - history/06-verify-viewer-pitch-0.md
  - 04-plan-match-rules.md
  - 04-plan-match-rules.yaml
  - 04-plan-match-rules.html.fragment
  - history/04-plan-3.md
  - history/04b-instrument-3.md
  - history/04b-instrument-3.yaml
  - history/04b-instrument-3.html.fragment
  - history/05c-benchmark-3.md
  - history/05c-benchmark-3.yaml
  - history/05c-benchmark-3.html.fragment
  - bench-baseline/match-rules/
  - 05-implement-match-rules.md
  - 06-verify-match-rules.md
  - verify-evidence/match-rules/
  - 04-plan-tactics-and-ai.md
  - 04-plan-commentary.md
  - 04-plan-calibration.md
  - 04-plan-viewer-reports-recovery.md
  - 04-plan-integration.md
  - 04-plan-experiment-flags.md
  - 04-plan-scripting-runtime.md
  - 04-plan-tactics-and-ai.yaml
  - 04-plan-tactics-and-ai.html.fragment
  - history/04b-instrument-4.md
  - history/04b-instrument-4.yaml
  - history/04b-instrument-4.html.fragment
  - history/05c-benchmark-4.md
  - history/05c-benchmark-4.yaml
  - history/05c-benchmark-4.html.fragment
  - bench-baseline/tactics-and-ai/
  - 05-implement-tactics-and-ai.md
  - implement-evidence/tactics-and-ai/
  - 06-verify-tactics-and-ai.md
  - verify-evidence/tactics-and-ai/
  - 05-implement-commentary.md
  - bench-baseline/commentary/
  - 06-verify-commentary.md
  - verify-evidence/commentary/
  - 05-implement-calibration.md
  - implement-evidence/calibration/
  - 06-verify-calibration.md
  - history/06-verify-calibration-0.md
  - verify-evidence/calibration/
  - 04-plan-viewer-match-day.md
  - 04-plan-viewer-match-day.yaml
  - 04-plan-viewer-match-day.html.fragment
  - history/04-plan-viewer-match-day-0.md
  - history/04-plan-viewer-match-day-1.md
  - 04-plan-viewer-lineup-tactics.md
  - 04-plan-extra-time-penalties.md
  - 04-plan-distribution.md
  - 04-plan-probe-engine-core.md
  - 04-plan-probe-engine-core.yaml
  - 04-plan-probe-engine-core.html.fragment
  - history/04-plan-probe-engine-core-0.md
  - history/04-plan-probe-engine-core-1.md
  - history/04-plan-probe-engine-core-2.md
  - 05-implement-viewer-match-day.md
  - implement-evidence/viewer-match-day/
  - 06-verify-viewer-match-day.md
  - verify-evidence/viewer-match-day/
  - 05-implement-viewer-lineup-tactics.md
  - 06-verify-viewer-lineup-tactics.md
  - verify-evidence/viewer-lineup-tactics/
  - 05-implement-viewer-reports-recovery.md
  - implement-evidence/viewer-reports-recovery/
  - 06-verify-viewer-reports-recovery.md
  - verify-evidence/viewer-reports-recovery/
  - 05-implement-integration.md
  - implement-evidence/integration/
  - bench-baseline/integration/
  - 06-verify-integration.md
  - verify-evidence/integration/
  - 05-implement-extra-time-penalties.md
  - 06-verify-extra-time-penalties.md
  - verify-evidence/extra-time-penalties/
  - 05-implement-experiment-flags.md
  - 06-verify-experiment-flags.md
  - verify-evidence/experiment-flags/
  - 05-implement-scripting-runtime.md
  - implement-evidence/scripting-runtime/
  - 06-verify-scripting-runtime.md
  - verify-evidence/scripting-runtime/
  - 06-verify-distribution.md
  - verify-evidence/distribution/
  - 05-implement-distribution.md
  - implement-evidence/distribution/
  - 05-implement-probe-engine-core.md
  - 05-implement-realism-bands-v2.md
  - implement-evidence/realism-bands-v2/
  - 06-verify-realism-bands-v2.md
  - verify-evidence/realism-bands-v2/
  - implement-evidence/probe-engine-core/
  - 06-verify-probe-engine-core.md
  - verify-evidence/probe-engine-core/
  - 07-review.md
  - 07-review.yaml
  - 07-review.html.fragment
  - 07-review-correctness.md
  - 07-review-correctness.yaml
  - 07-review-correctness.html.fragment
  - 07-review-security.md
  - 07-review-security.yaml
  - 07-review-security.html.fragment
  - 07-review-performance.md
  - 07-review-performance.yaml
  - 07-review-performance.html.fragment
  - 07-review-architecture.md
  - 07-review-architecture.yaml
  - 07-review-architecture.html.fragment
  - 07-review-intent-fidelity.md
  - 07-review-intent-fidelity.yaml
  - 04-plan-defending-and-discipline.md
  - 04-plan-defending-and-discipline.yaml
  - 04-plan-defending-and-discipline.html.fragment
  - history/04-plan-14.md
  - history/05c-benchmark-5.md
  - history/05c-benchmark-5.yaml
  - history/05c-benchmark-5.html.fragment
  - bench-baseline/defending-and-discipline/
  - 05-implement-defending-and-discipline.md
  - implement-evidence/defending-and-discipline/
  - history/05-implement-defending-and-discipline-0.md
  - 06-verify-defending-and-discipline.md
  - verify-evidence/defending-and-discipline/
  - 04-plan-tuning-loop.md
  - 04-plan-tuning-loop.yaml
  - 04-plan-tuning-loop.html.fragment
  - history/04-plan-15.md
  - history/04-plan-16.md
  - history/04-plan-tuning-loop-0.md
  - history/04-plan-tuning-loop-0.yaml
  - history/04-plan-tuning-loop-0.html.fragment
  - 05-implement-tuning-loop.md
  - implement-evidence/tuning-loop/
  - 06-verify-tuning-loop.md
  - verify-evidence/tuning-loop/
  - history/05c-benchmark-6.md
  - history/05c-benchmark-6.yaml
  - history/05c-benchmark-6.html.fragment
  - bench-baseline/lone-forward/
  - 04-plan-lone-forward.md
  - 04-plan-lone-forward.yaml
  - 04-plan-lone-forward.html.fragment
  - history/04-plan-17.md
  - 05-implement-lone-forward.md
  - implement-evidence/lone-forward/
  - history/05-implement-lone-forward-0.md
  - history/05-implement-lone-forward-1.md
  - 06-verify-lone-forward.md
  - verify-evidence/lone-forward/
  - history/05c-benchmark-7.md
  - history/05c-benchmark-7.yaml
  - history/05c-benchmark-7.html.fragment
  - bench-baseline/keeper-and-shots/
  - 04-plan-keeper-and-shots.md
  - 04-plan-keeper-and-shots.yaml
  - 04-plan-keeper-and-shots.html.fragment
  - history/04-plan-18.md
  - 05-implement-keeper-and-shots.md
  - implement-evidence/keeper-and-shots/
  - 06-verify-keeper-and-shots.md
  - verify-evidence/keeper-and-shots/
  - verify-evidence/keeper-and-shots-run-0/
  - 06-verify-tempo-and-restarts.md
  - verify-evidence/tempo-and-restarts/
  - verify-evidence/tempo-and-restarts-run-1/
  - history/06-verify-tempo-and-restarts-0.md
  - 04-plan-realism-tuning.md
  - 04-plan-realism-tuning.yaml
  - 04-plan-realism-tuning.html.fragment
  - 04-plan-realism-tuning.01-consult.html.fragment
  - 05-implement-realism-tuning.md
  - skip-slice-realism-tuning.md
  - history/04-plan-20.md
  - history/05-implement-keeper-and-shots-0.md
  - history/05c-benchmark-8.md
  - history/05c-benchmark-8.yaml
  - history/05c-benchmark-8.html.fragment
  - bench-baseline/tempo-and-restarts/
  - 04-plan-tempo-and-restarts.md
  - 04-plan-tempo-and-restarts.yaml
  - 04-plan-tempo-and-restarts.html.fragment
  - history/04-plan-19.md
  - 05-implement-tempo-and-restarts.md
  - implement-evidence/tempo-and-restarts/
  - history/05-implement-tempo-and-restarts-0.md
  - history/05-implement-tempo-and-restarts-1.md
  - history/05-implement-tempo-and-restarts-2.md
progress:
  intake: complete
  shape: complete
  design: complete
  slice: complete
  plan: in-progress
  implement: in-progress
  verify: in-progress
  review: complete
  handoff: not-started
  ship: not-started
  retro: not-started
---
