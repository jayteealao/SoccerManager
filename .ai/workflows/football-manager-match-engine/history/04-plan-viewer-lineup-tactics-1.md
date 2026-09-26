---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: viewer-lineup-tactics
status: awaiting-input
stage-number: 4
created-at: "2026-09-22T22:14:45Z"
updated-at: "2026-09-23T07:03:25Z"
metric-files-to-touch: 15
metric-step-count: 12
has-blockers: true
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-23T07:03:25Z"
    trigger: manual
    because: "auto-review — 7 issues found"
    changed: "re-grounded on the landed tactics work; placeholder step and line references corrected; bridge file added; option recommendations added to OQ-1 and OQ-2"
consult-runs: []
tags: [viewer, lineup, tactics, substitutions, awaiting-input]
stack-source: confirmed
open-questions:
  - id: OQ-1
    class: intent-bearing
    question: "How do the squad and the chosen lineup cross between engine and page before kick-off, and does a session hold before kick-off?"
    options: [1-hello-squads-set-lineup-hold, 2-squad-message-set-lineup-timeout, 3-page-server-before-match-start]
  - id: OQ-2
    class: intent-bearing
    question: "Where does the tactics panel read its schema, and what does a queued change's detail hold on the wire?"
    options: [1-schema-in-opening-message, 2-page-server-json, 3-fixed-in-page-code]
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-viewer-lineup-tactics.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md, 04-plan-viewer-match-day.md, 04-plan-viewer-reports-recovery.md, 04-plan-integration.md]
  design: 02b-design.md
  contract: 02c-craft.md
  steer: steer.md
  implement: 05-implement-viewer-lineup-tactics.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine viewer-lineup-tactics"
---

# Plan: Lineup Editor and Tactics Panel

## The Plan

The tactics work has landed since the first version of this plan. The engine now has a tactics model read from `content/tactics.json`, a substitution limit in the rule pack (5 changes in 3 windows, half-time exempt), a change queue with `Simulation::queue_change` (`crates/engine/src/tactics/change.rs:194`), and change events that carry `change.applied_tick`. The home team is the human manager's, and it starts with the computer manager's pre-match setup (`crates/engine-cli/src/serve.rs:19-23`, `crates/engine/src/ai.rs:128`). Two gaps remain, and both are public contract. No message carries a squad to the page or a lineup back before kick-off. A change the page queues over the socket is acknowledged and not applied, because its `detail` is still opaque (`docs/reference/protocol.md:187-191`). The tactics plan left that wire payload to this plan.

This revision re-checked the plan against the current code and corrected seven drifts. It keeps the contract-independent half ready: 12 steps over 15 files. That half covers the lineup rules, the pending-change list with its four state words, the chip styles, and the keyboard drive. It stops again at the same two product-owner questions. OQ-1 asks how the squad reaches the page and how a lineup returns before kick-off. OQ-2 asks where the tactics schema reaches the page and what a queued change carries. Five of the six acceptance criteria depend on one of these answers. Each question now carries a recommended option, but no option was chosen.

When the product owner answers, a re-run of this stage completes steps 7 to 12 and sets the status to complete. The top open risk is that a pre-match hold changes session behaviour for every client. Today a client that never sends `start` still receives the whole match. OQ-1 also overlaps the match-day plan's open question Q-1 about rosters, so one answer can serve both slices.

## Current State

- **Page.** `web/index.html:26-29` holds the left column "Lineups" with the note "Coming in a later version." (product wording since commit `e79ad61`). The right column "Match feed" (`:102`) and the statistics region (`:96`) are also empty regions. `web/main.mjs` (270 lines) connects the socket, decodes frames, and exposes `window.__touchline`. `node --test web/tests/*.test.mjs` ran in this session: 47 pass, 0 fail.
- **Protocol.** `PROTOCOL_VERSION` is 2 (`crates/protocol/src/lib.rs:44`). Client commands are `start`, `pause`, `set-speed`, and `queue-change`. The `queue-change` `detail` is "opaque in this build; not yet read by the engine" (`docs/reference/protocol.md:187`). `ChangeState::label()` gives the chip words Queued, Applies now, Applied, and Rejected (`crates/protocol/src/command.rs:38-50`). `EventType` now includes `injury`, `substitution`, and `ai-decision`, and the event carries `change.applied_tick` (`crates/protocol/src/event.rs:30-32,133-136`).
- **Engine.** `Change` is `Tactics(TacticsPatch)` or `Substitution { off, on }`, with players named by squad index (`crates/engine/src/tactics/change.rs:25-34`). `ChangeId` prints as `q-{tick}-{n}`, like the socket queue's id, "so a later bridge can carry an identifier through unchanged" (`change.rs:60-61`). `content/tactics.json` holds `formations`, `mentalities`, `instructions`, `roles`, `duties`, and `ai`. The rule pack holds `"substitutions": { "limit": 5, "windows": 3, "windows_exempt": ["half_time"] }`.
- **Session.** `serve` builds `MatchConfig` (`serve.rs:22-23`) before it accepts the page connection (`serve.rs:71`). `Gate::new` "starts producing at once. A client that never sends `start` still receives the whole match." (`crates/stream/src/control.rs:32-33`). The socket queue-change goes to a separate `protocol::Queue` (`serve.rs:88`), not to the engine queue. The stream driver lives in `crates/engine-cli/src/stream_run.rs` (319 lines).
- **Dependencies.** `tactics-and-ai` is complete. `viewer-match-day` is `defined`, and its plan is `awaiting-input` on Q-1: the stream carries no player roster, fatigue, or live statistics (`04-plan-viewer-match-day.md`).

## Simplicity Ladder

- Lineup legality check → rung 4 new-code: `web/lineup.mjs`. No stdlib or platform check exists for football lineups. The Rust squad check validates a team file, not a selection, and it runs in a different runtime. The Rust `ai::pre_match` (`crates/engine/src/ai.rs:128`) picks a lineup and does not judge one.
- Pending-change state machine → rung 3 reuse (the vocabulary) plus rung 4 (the page module). The state words come from `ChangeState::label()` (`crates/protocol/src/command.rs:47`), an exact match, reused verbatim. No page module holds change state today.
- Drag and drop → rung 2 native-platform: the HTML Drag and Drop API, as an enhancement only. Click-to-place is the primary path (slice Risks).
- Keyboard operability → rung 2 native-platform: native `<button>`, `<select>`, and `<fieldset>` elements, with `:focus-visible` rings from `web/components/match-control.css`. No roving-tabindex library.
- Chip and control styling → rung 3 reuse: `web/tokens.css` and `web/components/match-control.css`. The chip adds one small stylesheet.
- Engine-side change application → rung 3 reuse: `Simulation::queue_change` (`change.rs:194`) and `Change` (`change.rs:30`), reused as-is. The bridge maps a wire change onto them. The wire shape is pending OQ-2.
- Squad delivery, lineup submission, tactics schema delivery → **pending OQ-1 and OQ-2.** Every candidate is a contract change (see Blockers).

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist in this repository.

Repeat-deferral tripwire: `00-index.md` has `runtime-evidence-deferrals: []`, so the tripwire does not fire.

## Likely Files / Areas to Touch

- `web/lineup.mjs` (new): pure legality rules and reason text.
- `web/pending.mjs` (new): the pending-change list keyed by `change.queue_id`.
- `web/lineup-editor.mjs` (new): the DOM editor. Blocked on OQ-1.
- `web/tactics-panel.mjs` (new): renders from the tactics schema. Blocked on OQ-2.
- `web/substitution-picker.mjs` (new): queues a substitution and shows the remaining count and rejections. Blocked on OQ-2.
- `web/components/chip.css` (new): chip styles from the `--tl-` tokens.
- `web/index.html`: the editor region in place of the left-column placeholder, and the tactics panel region.
- `web/main.mjs`: wiring, and the `window.__touchline.pending()` and `.lineup()` readers for the drives.
- `web/tests/lineup.test.mjs`, `web/tests/pending.test.mjs` (new).
- `crates/protocol/src/message.rs`, `crates/stream/src/control.rs`, `crates/engine-cli/src/serve.rs`, `crates/engine-cli/src/stream_run.rs`, `docs/reference/protocol.md`: change scope depends on the OQ-1 and OQ-2 answers.

## Proposed Change Strategy

Build the page half as pure modules first, tested under `node --test`, and follow the viewer-pitch convention: `.mjs`, no `package.json`, no framework. Then build the DOM panels as ordinary DOM, not canvas, so that they stay accessible (slice Risks). The page learns every state from the engine. The chip mirrors the engine's queue and never predicts an outcome. A rejected change shows the engine's `reason` string verbatim.

On the engine side, the bridge reuses the engine's own queue. An admitted socket change is mapped onto `Change` and passed to `Simulation::queue_change`, and the socket queue id is carried through. The wire shape of that change, the squad message, and the pre-match hold wait for the product owner. The options are listed in Blockers so that each answer can come back as one pick. This plan cites no NFR as the reason for a mechanism choice.

## Step-by-Step Plan

Steps 1 to 6 do not depend on the open contract and are ready now. Steps 7 to 12 carry a **PENDING** marker and are completed after OQ-1 and OQ-2 are answered.

1. **Placeholder and vocabulary check.** When step 8 adds the editor region, remove the left-column note "Coming in a later version." (`web/index.html:28`). Before the commit, search `web/` and `crates/` for internal workflow vocabulary, and keep that search as the last step before the commit.
2. **Lineup rules.** Write `web/lineup.mjs` with `checkLineup({ slots, bench, squad })`. It returns `{ legal: boolean, reason: string | null }` and gives the first failure in this fixed order: fewer than eleven starters; no goalkeeper in the goalkeeper slot; a player placed twice (starter and bench, or two slots); a bench of more than seven. Each reason names the player or the count, for example "Ten starters; a match needs eleven."
3. **Lineup tests.** Write `web/tests/lineup.test.mjs` with one test per illegal case in step 2, one test for the legal case, and one test for an empty bench (legal).
4. **Pending-change list.** Write `web/pending.mjs` with `createPendingList()` and the methods `queued(ack)`, `rejectedAtQueue(reject, localId)`, `onChangeEvent(event)`, and `chips()`. States are `queued → applies-now → applied`, and the chip leaves the list after it shows Applied. The other end state is `rejected`, and the chip keeps the reason until the manager dismisses it. Every chip carries the word from `ChangeState::label()`. An event with an unknown `change.queue_id` is ignored and logged as a `viewer-event` row. The field names come from `docs/reference/protocol.md` and `crates/protocol/src/event.rs`: `change.queue_id`, `change.state`, `change.rejected_reason`, and `change.applied_tick`.
5. **Pending tests.** Write `web/tests/pending.test.mjs`. Cover the full lifecycle, a rejection at queue time (a `reject` message), a rejection at a stoppage (a change event), an unknown queue id, and two chips that resolve out of order.
6. **Chip styles.** Write `web/components/chip.css` with `--tl-success`, `--tl-warning`, `--tl-danger`, and `--tl-fg-muted` from `steer.md` and `web/tokens.css`. Always render the state word as text, so colour never carries the state alone (`02c-craft.md` inventory items 9 and 10). Use the 28-pixel `match-control` height.
7. **PENDING OQ-1 and OQ-2: engine-side squad, lineup, and change bridge.** The answers set the scope. In every option, the bridge in `crates/engine-cli/src/stream_run.rs` maps an admitted `queue-change` onto `Change` (`tactics/change.rs:30`) and calls `Simulation::queue_change` (`change.rs:194`) for the home team, and it carries the socket queue id through. A page-chosen lineup replaces the home team's pre-match `Setup` before the first tick.
8. **PENDING OQ-1: lineup editor.** Write `web/lineup-editor.mjs`. It holds a formation picker (formations from the tactics schema), eleven slots, and up to seven bench slots. Click-to-place comes first and drag is an enhancement. Each slot shows role fit and fitness, and the inline reason from step 2. The kick-off control stays disabled until the lineup is legal.
9. **PENDING OQ-2: tactics panel.** Write `web/tactics-panel.mjs`, rendered from the schema, so that a new instruction needs no UI change. The panel is available during play and while paused. Changes go out through `queue-change` and create a chip.
10. **PENDING OQ-2: substitution picker.** Write `web/substitution-picker.mjs`. Show the remaining count from the rule pack limit and the applied `substitution` events. On Applied, swap the players in the `viewer-match-day` lineup panel.
11. **Wiring and hook.** Wire `web/main.mjs` and extend `window.__touchline` with `pending()` and `lineup()`. The shape follows the modules from steps 2 and 4. The socket calls follow OQ-1.
12. **Protocol document and tests.** Update `docs/reference/protocol.md` (the queue-change section and any new message) and the document test. Run `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`, `node --test web/tests/*.test.mjs`, and the vocabulary search from step 1.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-a illegal lineup disables kick-off; legal enables | Claude_Browser drive with a screenshot per case (web-2), plus `lineup.test.mjs` for the rules | Windows 11, Node 22, `engine-cli serve --web web` — yes | `window.__touchline.lineup()`; `data-testid` on slots and the kick-off control; the squad path (OQ-1) | node unit test over the rules (proxy) → re-plan after the OQ-1 answer |
| AC-b mentality change: chip, then "Tactical change applied" at the next dead ball | Claude_Browser drive against the live engine, seed 42 (web-2) | live engine with the tactics model — yes (tactics work landed); the socket bridge — not yet | `window.__touchline.pending()`; the bridge and schema delivery (OQ-2) | `pending.test.mjs` lifecycle (proxy) → re-plan after the OQ-2 answer |
| AC-c substitution applies, lineup swaps, count decrements | Claude_Browser drive, seed 42 (web-2) | as AC-b | the remaining-count source; the `viewer-match-day` lineup panel | as AC-b |
| AC-d sixth substitution rejected with the engine's reason | Claude_Browser drive after five substitutions (web-2) | as AC-b | as AC-c | `pending.test.mjs` rejection case (proxy) → as AC-b |
| AC-e role change while paused applies at the next dead ball, not at resume | Claude_Browser drive reading `change.applied_tick` and the applied-event minute (web-2) | as AC-b | `pending()` exposes the queued and applied ticks | as AC-b |
| AC-f keyboard-only operation (observable: false) | Claude_Browser drive: Tab and Enter only, reading `document.activeElement` and the computed focus outline per step (web-2) | yes | native controls; `:focus-visible` rings | screenshot review of focus order |

Constraint resolutions (provisional; finalised on the re-run after OQ-1 and OQ-2):
- AC-a: `constraint-resolution: prerequisite-slice: viewer-lineup-tactics` (the squad and lineup path is this slice's own scope once OQ-1 picks its shape). `wall-ownership: code-owned`.
- AC-b to AC-e: `constraint-resolution: prerequisite-slice: viewer-lineup-tactics` (the socket bridge into the engine queue is this slice's own step 7; the tactics model it needs has landed). AC-c also needs the `viewer-match-day` lineup panel, which is ordered first. `wall-ownership: code-owned`.
- AC-f: no environment wall.

## Test / Verification Plan

### Automated checks

- `node --test web/tests/*.test.mjs`. The glob form ran 47 of 47 passing in this session. The bare directory form fails on this machine (Node 22 on Windows).
- `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`, for the Rust changes that OQ-1 and OQ-2 bring.
- A search for internal workflow vocabulary in `web/` and `crates/` before the commit.

### Interactive verification (human-in-the-loop)

Platform: web. Tool: the Claude_Browser in-app browser pane (`stack.available-mcp`, the shape's Q29 choice). Run command: `cargo run -p engine-cli -- serve --seed 42 --web web`, then open the printed page address. Evidence: screenshots and `window.__touchline` reads under `verify-evidence/viewer-lineup-tactics/`. The drives for AC-a to AC-e follow the table above. Their exact steps are finalised after OQ-1 fixes how the pre-match screen reaches the engine.

## Risks / Watchouts

- A pre-match hold changes session behaviour for headless clients and for the recorder and the replayer. The fixture path must keep working.
- The socket `protocol::Queue` and the engine queue both admit changes. The bridge must not make them disagree: a change the socket admits and the engine later rejects must reach the chip as Rejected with the engine's reason.
- Drag and drop works against accessibility. Click-to-place stays primary.
- The instruction count can grow. The panel renders from the schema, so it must not hard-code instruction names.
- The snapshot now holds the pending queue (tactics plan, Assumption 14). A resumed match must restore the chips from it, which the reports-and-recovery slice consumes.

## Dependencies on Other Slices

- `tactics-and-ai` (complete): the tactics model, the substitution limit, the change events, and the rejection reasons.
- `viewer-match-day` (plan awaiting input on Q-1): the lineup panel that a substitution updates, and the feed line "Tactical change applied". Its Q-1 roster question overlaps OQ-1.
- `viewer-pitch` (complete): `web/tokens.css`, `web/components/match-control.css`, `window.__touchline`.

## Assumptions

- **A1** (class: implementation-detail): This run writes only its own three plan files and their history snapshots. The master `04-plan.md`, `00-index.md`, and `INDEX.md` are left to the run driver, which is the single writer. Why: the driver withheld index writes for this run, and four plan agents run at once.
- **A2** (class: implementation-detail): AC-f (keyboard, observable: false) is verified by an in-app browser keyboard drive, not a Testing Library test. Why: Testing Library needs npm and a DOM, and the repository has no `package.json` (viewer-pitch Q7). Claude_Browser is in `stack:`.
- **A3** (class: implementation-detail): The chip leaves the list after it shows Applied. A Rejected chip stays with its reason until the manager dismisses it. Why: the slice says the chip "clears" on apply, and the rejection criterion says the page "shows the engine's rejection reason".
- **A4** (class: implementation-detail): The legality reasons are checked in a fixed order, and the page shows one reason at a time. Why: this is the smallest inline message that satisfies "the reason shows inline".
- **A5** (class: implementation-detail): No consult second opinion ran. The triggers `appetite-medium-or-larger` and `unknowns-present` hold, but the product owner excluded `consult` at intake (`stack.excluded-by-po`).
- **A6** (class: implementation-detail): The `node --test` invocation uses the file glob, because the directory form fails on this machine.
- **A7** (class: implementation-detail): This re-run is an auto-review of the existing plan, not a new plan, so the discovery interview does not run. The prior revision is byte-copied to `history/04-plan-viewer-lineup-tactics-0.{md,yaml,html.fragment}`.
- **A8** (class: implementation-detail): The request that started this run reads "1". It names neither OQ-1 nor OQ-2 and none of their options, so it is not taken as a product-owner answer. Why: reading a contract choice into an ambiguous token would be a product decision made without the product owner.
- **A9** (class: implementation-detail): The socket bridge into the engine queue belongs to this slice (step 7). Why: the tactics plan records "`viewer-lineup-tactics` owns the socket bridge into `Simulation::queue_change` and the pre-kick-off lineup message" (`04-plan-tactics-and-ai.md`, Dependencies).
- **A10** (class: implementation-detail): Step 1 no longer rewrites the placeholder wording, because commit `e79ad61` already replaced the internal name with "Coming in a later version.". The note is removed when the editor region takes its place.

Auto-review findings fixed in this revision (7): the placeholder step was already done in the code; the line references in `serve.rs` and `control.rs` had moved; the file-order default lineup is now the computer manager's pre-match setup; `tactics-and-ai` is complete, so its dependency and the AC-b to AC-e constraint resolutions were stale; the bridge file `stream_run.rs` was missing; the test count is 47, not 46; and the overlap with the match-day plan's Q-1 was not recorded.

## Blockers

**OQ-1 (class: intent-bearing; public contract and session behaviour). How do the squad and the chosen lineup cross between engine and page before kick-off?**
Research facts: `serve` builds `MatchConfig` (`serve.rs:22-23`) before it accepts the page (`serve.rs:71`). `Gate` produces at once (`control.rs:32-33`). `hello.teams[]` carries id, name, and kit only (`serve.rs:45-58`). The home team starts with the computer manager's pre-match setup (`serve.rs:19-23`, `ai.rs:128`). Every option adds public protocol surface. Options:
1. **Hello carries the squads, a new `set-lineup` command, and the engine holds before kick-off.** The engine replaces the home `Setup` when the lineup arrives. A headless client that sends no lineup plays the computer manager's pre-match setup after an explicit `start`. Cost: `PROTOCOL_VERSION` rises to 3, and the recorder and the replayer must still play fixtures. This option also answers the roster part of the match-day plan's Q-1. **Recommended**, because it has the smallest new surface that serves both slices, and the default lineup already exists.
2. **A separate squad message and `set-lineup`, with a lineup timeout.** The engine waits up to N seconds and then plays the default. Cost: session behaviour depends on time.
3. **Choose the lineup before the engine starts.** The page asks the page server over HTTP for the squads and posts the lineup, and `serve` starts the match afterwards. Cost: a second API surface beside the socket, and the match process lifecycle changes.

**OQ-2 (class: intent-bearing; public contract). Where does the tactics panel read its schema, and what does a queued change's `detail` hold?**
Research facts: the schema is the data file `content/tactics.json`, and it is included in the content hash. The engine's `Change` names players by squad index (`change.rs:27-34`). The socket change is acknowledged and not applied (`protocol.md:189-191`). Options:
1. **The tactics schema rides in the opening message.** `detail` is `{ "patch": <tactics patch> }` for tactics or `{ "off": <squad index>, "on": <squad index> }` for a substitution, mirroring `Change`. **Recommended**, because it keeps one socket contract and the page cannot read a schema that differs from the engine's.
2. The page server serves the tactics content file as JSON, with the same `detail` shapes.
3. The schema is fixed in page code. This contradicts the slice's "renders from the tactics schema" mitigation.

## Freshness Research

No new dependency is proposed. The page uses the built-in HTML Drag and Drop API, native form controls, and the built-in `node:test` runner, which is already in use (viewer-pitch plan, Freshness Research). No web research ran, because no library choice is open. The open questions are contract choices.

## Recommended Next Stage

- **Option A (after the product owner answers OQ-1 and OQ-2):** `/wf plan football-manager-match-engine viewer-lineup-tactics`. The re-run finalises steps 7 to 12 and the constraint resolutions, and sets the status to complete.
- **Option B:** `/wf plan football-manager-match-engine viewer-match-day`. Answer its Q-1 together with OQ-1, so that one roster message serves both slices.
- **Option C:** `/wf slice football-manager-match-engine`. Use it if the product owner decides that the squad-and-lineup contract belongs in its own slice.
