---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: viewer-lineup-tactics
status: awaiting-input
stage-number: 4
created-at: "2026-09-22T22:14:45Z"
updated-at: "2026-09-22T22:14:45Z"
metric-files-to-touch: 14
metric-step-count: 12
has-blockers: true
revision-count: 0
revisions: []
consult-runs: []
tags: [viewer, lineup, tactics, substitutions, awaiting-input]
stack-source: confirmed
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-viewer-lineup-tactics.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md, 04-plan-viewer-match-day.md]
  implement: 05-implement-viewer-lineup-tactics.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine viewer-lineup-tactics"
---

# Plan: Lineup Editor and Tactics Panel

## The Plan

The page draws the match and the engine holds a change queue, but no path exists for the manager's lineup to reach the engine. The engine builds the match from the team files before any page connects (`crates/engine-cli/src/serve.rs:17-20`), starts producing at once (`crates/stream/src/control.rs:32`), and plays the first eleven players in file order (`crates/engine/src/team.rs:88-92`). The opening message carries club identity and kit colours only, so the page cannot list a squad. The tactics model, the substitution limit, and the shape of a queued change belong to `tactics-and-ai`, which a sibling agent plans in parallel with this one.

This plan fixes the contract-independent half: 12 steps over 14 files. It covers the pure lineup-legality module, the pending-change list with its four state words, the chip styles, the keyboard drive, and the removal of a placeholder note in the page that names an internal work item. It stops at two open questions for the product owner. Each question selects a new public protocol contract. OQ-1 asks how a squad reaches the page and how a lineup returns before kick-off. OQ-2 asks where the tactics panel reads its schema. Five of the six acceptance criteria depend on one of these answers.

When the product owner answers OQ-1 and OQ-2, a re-run of this stage completes the engine-side steps and changes the status to complete. The top open risk: a pre-match hold changes session behaviour for every client, including headless ones. Today a client that never sends `start` still receives the whole match.

## Current State

- **Page.** `web/index.html` has an empty left column labelled "Lineups" and an empty right column. The left column carries the note "Built by viewer-lineup-tactics." (`web/index.html:28`). That note puts an internal work-item name in shipped page text, and this plan removes it. `web/main.mjs` (270 lines) connects the socket, decodes the frames, and exposes `window.__touchline`. `node --test web/tests/*.test.mjs` ran in this session: 46 pass, 0 fail.
- **Protocol.** The client commands are `start`, `pause`, `set-speed`, and `queue-change`. The `detail` field of `queue-change` is "opaque in this build; not yet read by the engine" (`docs/reference/protocol.md`, queue-change). `ChangeState` already names the four chip words: Queued, Applies now, Applied, Rejected (`crates/protocol/src/command.rs`). `Queue::submit` refuses an unknown kind or a kind the rule pack forbids, and gives the reason in words.
- **Engine.** No formation input, lineup input, or bench model exists. `FORMATION_442` is hard-coded (`crates/engine/src/team.rs:21`). The team file holds 11 to 40 players with ten position codes (`crates/engine/src/data/team.rs:112`). The shipped default teams hold 22 players each.
- **Session.** `Gate` "starts producing at once". A client that never sends `start` still receives the whole match (`crates/stream/src/control.rs:32`).
- **Dependencies.** `viewer-match-day` (the lineup panel that a substitution updates) and `tactics-and-ai` (the change queue drain, rejection reasons, the tactics model, and the 5-in-3-windows limit) have status `defined`. Neither plan existed on disk when this plan was written; both are in flight in parallel.

## Simplicity Ladder

- Lineup legality check → rung 4 new-code: `web/lineup.mjs`. No stdlib or platform check exists for football lineups. No in-repo helper exists: the Rust squad check `check_squad` (`crates/engine/src/data/team.rs:129`) validates a team file, not a selection, and it runs in a different runtime.
- Pending-change state machine → rung 3 reuse (the vocabulary) plus rung 4 (the page module). The state words come from `ChangeState::label()` (`crates/protocol/src/command.rs`), an exact match reused verbatim. The page-side list is new because no page module holds change state.
- Drag and drop → rung 2 native-platform: the HTML Drag and Drop API as an enhancement only. Click-to-place is the primary path (slice Risks).
- Keyboard operability → rung 2 native-platform: native `<button>`, `<select>`, and `<fieldset>` elements, with `:focus-visible` rings from `web/components/match-control.css`. This plan adds no roving-tabindex library.
- Chip and control styling → rung 3 reuse: `web/tokens.css` and `web/components/match-control.css` (the viewer-pitch plan defines all four sizes and five states). The chip adds one small stylesheet.
- Squad delivery, lineup submission, tactics schema → **pending OQ-1 and OQ-2.** Every rung-3 candidate is a contract change (see Blockers).

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
- `web/index.html`: the editor and panel regions, and removal of the internal-name placeholder at line 28.
- `web/main.mjs`: wiring, and `window.__touchline.pending()` / `.lineup()` readers for the drives.
- `web/tests/lineup.test.mjs`, `web/tests/pending.test.mjs` (new).
- `crates/protocol/src/message.rs`, `crates/stream/src/control.rs`, `crates/engine-cli/src/serve.rs`, `docs/reference/protocol.md`: change scope depends on the OQ-1 and OQ-2 answers.

## Proposed Change Strategy

Build the page half as pure modules first, tested under `node --test`, following the viewer-pitch convention: `.mjs`, no `package.json`, no framework. Build the DOM panels next as ordinary DOM, not canvas, so they stay accessible (slice Risks). The page learns every state from the engine: the chip mirrors the engine's queue and never predicts an outcome itself. A rejected change shows the engine's `reason` string verbatim, which is the text `Queue::submit` already produces.

The engine-side strategy waits for the product owner. The three contract families under consideration are listed in Blockers so the answer can come back as one pick per question. This plan does not cite an NFR as the reason for any mechanism choice.

## Step-by-Step Plan

Steps 1–6 are contract-independent and ready now. Steps 7–12 carry a **PENDING** marker and are completed after OQ-1 and OQ-2 are answered.

1. **Remove the internal-name placeholder.** In `web/index.html:28`, replace "Built by viewer-lineup-tactics." with product wording ("Pick your lineup before kick-off."). Then search `web/` and `crates/` for the slice vocabulary, and keep that search as the last step before the commit.
2. **Lineup rules.** Write `web/lineup.mjs` with `checkLineup({ slots, bench, squad })`. It returns `{ legal: boolean, reason: string | null }` and gives the first failure in this fixed order: fewer than eleven starters; no goalkeeper in the goalkeeper slot; a player placed twice (starter and bench, or two slots); bench over seven. Each reason names the player or the count, for example "Ten starters; a match needs eleven."
3. **Lineup tests.** Write `web/tests/lineup.test.mjs` with one test per illegal case in step 2, one test for the legal case, and one test for an empty bench (legal).
4. **Pending-change list.** Write `web/pending.mjs` with `createPendingList()` and the methods `queued(ack)`, `rejectedAtQueue(reject, localId)`, `onChangeEvent(event)`, and `chips()`. States are `queued → applies-now → applied` (the chip leaves the list after it shows Applied) or `rejected` (the chip keeps the reason until the manager dismisses it). Every chip carries `ChangeState.label()`'s word. An event with an unknown `change.queue_id` is ignored and logged as a `viewer-event` row. The field names for the change event come from `docs/reference/protocol.md` (`change.queue_id`, `change.state`, `change.rejected_reason`).
5. **Pending tests.** Write `web/tests/pending.test.mjs` to cover: the full lifecycle; a rejection at queue time (a `reject` message); a rejection at a stoppage (a change event); an unknown queue id; and two chips that resolve out of order.
6. **Chip styles.** Write `web/components/chip.css`. It uses `--tl-success`, `--tl-warning`, `--tl-danger`, and `--tl-fg-muted` from `steer.md` and `web/tokens.css`. The state word is always rendered as text, and colour never carries the state alone (02c-craft.md inventory items 9 and 10). The chip uses the 28-pixel `match-control` height.
7. **PENDING OQ-1: engine-side squad and lineup path.** Scope is set by the answer.
8. **PENDING OQ-1: lineup editor.** Write `web/lineup-editor.mjs`: formation picker, eleven slots, up to seven bench slots, click-to-place first and drag as an enhancement, role fit and fitness per slot, and the inline reason from step 2. The kick-off control stays disabled until the lineup is legal.
9. **PENDING OQ-2: tactics panel.** Write `web/tactics-panel.mjs`, rendered from the schema. The panel is available during play and while paused. Changes go out through `queue-change` and create a chip.
10. **PENDING OQ-2: substitution picker.** Write `web/substitution-picker.mjs` with the remaining count, and swap the players in the `viewer-match-day` lineup panel on Applied.
11. **Wiring and hook.** Wire `web/main.mjs` and extend `window.__touchline` with `pending()` and `lineup()`. The shape follows the modules from steps 2 and 4, and the socket calls follow OQ-1.
12. **Protocol document and tests.** Update `docs/reference/protocol.md` and the document test for whatever the answers add. Run `cargo test --workspace`, `cargo clippy -- -D warnings`, `node --test web/tests/*.test.mjs`, and the vocabulary search from step 1.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-a illegal lineup disables kick-off; legal enables | Claude_Browser drive with screenshots per case (web-2), plus `lineup.test.mjs` for the rules | Windows 11, Node 22, `engine-cli serve --web web` — yes | `window.__touchline.lineup()`; `data-testid` on slots and the kick-off control; the squad path (OQ-1) | node unit test over the rules (proxy) → pre-registered deferral cleared when OQ-1 lands |
| AC-b mentality change: chip then "Tactical change applied" at the next dead ball | Claude_Browser drive against the live engine, seed 42 (web-2) | live engine with tactics-and-ai implemented — not yet | `window.__touchline.pending()`; the tactics schema (OQ-2) | `pending.test.mjs` lifecycle (proxy) → deferral cleared when tactics-and-ai is implemented |
| AC-c substitution applies, lineup swaps, count decrements | Claude_Browser drive, seed 42 (web-2) | as AC-b | remaining-count source (OQ-2); the viewer-match-day lineup panel | as AC-b |
| AC-d sixth substitution rejected with the engine's reason | Claude_Browser drive after five substitutions (web-2) | as AC-b | as AC-c | `pending.test.mjs` rejection case (proxy) → as AC-b |
| AC-e role change while paused applies at the next dead ball, not at resume | Claude_Browser drive reading the applied-event minute (web-2) | as AC-b | `pending()` exposes the queued and applied ticks | as AC-b |
| AC-f keyboard-only operation (observable: false) | Claude_Browser drive: Tab and Enter only, reading `document.activeElement` and the computed focus outline per step (web-2) | yes | native controls; `:focus-visible` rings | screenshot review of focus order → none needed |

Constraint resolutions (provisional; they are finalised on the re-run after OQ-1 and OQ-2):
- AC-a: `constraint-resolution: prerequisite-slice: viewer-lineup-tactics` (the squad and lineup path is this slice's own scope once OQ-1 picks its shape). `wall-ownership: code-owned`.
- AC-b to AC-e: `constraint-resolution: prerequisite-slice: tactics-and-ai` (the change-queue drain and the limit are that slice's scope, ordered before this one in the implementation order). `wall-ownership: code-owned`.
- AC-f: no environment wall.

## Test / Verification Plan

### Automated checks

- `node --test web/tests/*.test.mjs`. A bare directory argument fails on this machine (Node 22 on Windows, observed in this session); the glob form ran 46 of 46 passing.
- `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`. These apply to any Rust change that OQ-1 or OQ-2 brings.
- A search for workflow vocabulary in `web/` and `crates/` before the commit.

### Interactive verification (human-in-the-loop)

Platform: web. Tool: the Claude_Browser in-app browser pane (`stack.available-mcp`, the shape's Q29 choice). Run command: `cargo run -p engine-cli -- serve --seed 42 --web web`; open the printed page address. Evidence: screenshots and `window.__touchline` reads, under `verify-evidence/viewer-lineup-tactics/`. The drives for AC-a to AC-e follow the table above. Exact steps are finalised after OQ-1 fixes how the pre-match screen reaches the engine.

## Risks / Watchouts

- A pre-match hold changes session behaviour for headless clients and the recorder or replayer. The fixture path must keep working.
- The tactics-and-ai plan may define change-event fields differently from what step 4 reads. The cohesion check must reconcile the two before implementation.
- Drag and drop works against accessibility. Click-to-place stays primary.
- Instruction count grows. The panel renders from the schema, so it must not hard-code instruction names.

## Dependencies on Other Slices

- `tactics-and-ai`: the tactics model, the substitution limit, the applied and rejected change events, and the rejection reasons. It is planned in parallel, and its plan is not yet on disk.
- `viewer-match-day`: the lineup panel that the substitution updates, and the feed line "Tactical change applied".
- `viewer-pitch` (done): `web/tokens.css`, `web/components/match-control.css`, `window.__touchline`.

## Assumptions

- **A1** (class: implementation-detail): The parallel plan-mode rule applies. This agent writes only its own three plan files. The master `04-plan.md`, `00-index.md`, and `INDEX.md` are left to the orchestrating driver, which is the single writer. Why: six plan agents run at once, per the driver journal.
- **A2** (class: implementation-detail): AC-f (keyboard, observable: false) is verified by an in-app browser keyboard drive, not by a Testing Library test. Why: Testing Library needs npm and a DOM, while the repository has no `package.json` by the viewer-pitch Q7 decision. Claude_Browser is in `stack:`. This keeps the tooling inside the confirmed stack.
- **A3** (class: implementation-detail): The chip leaves the list after it shows Applied. A Rejected chip stays with its reason until the manager dismisses it. Why: the slice says the chip "clears" on apply, and the rejection criterion says the page "shows the engine's rejection reason".
- **A4** (class: implementation-detail): The legality reasons are checked in a fixed order and the page shows one reason at a time. Why: this is the smallest inline message that satisfies "the reason shows inline".
- **A5** (class: implementation-detail): The consult second opinion is not fired. The trigger `appetite-medium-or-larger` holds, but the product owner excluded `consult` at intake (`stack.excluded-by-po`).
- **A6** (class: implementation-detail): The `node --test` invocation uses the file glob, because the directory form failed in this session.

## Blockers

**OQ-1 (intent-bearing; public contract and session behaviour). How do the squad and the lineup cross between engine and page before kick-off?**
Research facts: `serve` builds `MatchConfig` before any client connects (`crates/engine-cli/src/serve.rs:17-20`). `Gate` produces at once (`crates/stream/src/control.rs:32`). `hello.teams[]` carries id, name, and kit only. The engine plays the first eleven players in file order (`crates/engine/src/team.rs:88-92`). Every option adds public protocol surface. Options:
1. **Hello carries the squads; a new `set-lineup` command; the engine holds before kick-off.** The engine builds the match after the lineup arrives. A headless client that sends no lineup gets the file-order default after an explicit `start`. Cost: `PROTOCOL_VERSION` rises to 3, and the recorder and replayer must still play fixtures.
2. **A separate squad message and `set-lineup`, with a lineup timeout.** The engine waits up to N seconds, then plays the default. Cost: session behaviour depends on time.
3. **Choose the lineup before the engine starts.** The page asks the page server over HTTP for the squads and posts the lineup, and `serve` starts the match afterwards. Cost: a second API surface beside the socket, and the match process lifecycle changes.

**OQ-2 (intent-bearing; public contract, shared with the in-flight tactics-and-ai plan). Where does the tactics panel read its schema, and what does a change's `detail` hold?**
Options:
1. The tactics schema rides in the opening message, from the rule pack and tuning content.
2. The page server serves the tactics content file as JSON.
3. The schema is fixed in page code, which contradicts the slice's "renders from the tactics schema" risk mitigation.

Whichever option is chosen, the substitution remaining count and the applied-change payload must match what tactics-and-ai emits.

## Freshness Research

No new dependency is proposed. The page uses the built-in HTML Drag and Drop API and native form controls. The built-in `node:test` runner is already in use (viewer-pitch plan, Freshness Research). No web research ran in this pass because no library choice is open. The open questions are contract choices, not library choices.

## Recommended Next Stage

- **Option A (after the product owner answers OQ-1 and OQ-2):** `/wf plan football-manager-match-engine viewer-lineup-tactics`. The re-run finalises steps 7–12 and the constraint resolutions, and sets the status to complete.
- **Option B:** `/wf plan football-manager-match-engine tactics-and-ai`. Complete that plan first, so that OQ-2 is answered against a written tactics model.
- **Option C:** `/wf slice football-manager-match-engine`. Use it if the product owner decides that the squad-and-lineup contract belongs in its own slice.
