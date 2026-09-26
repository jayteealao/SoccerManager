---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: viewer-lineup-tactics
status: complete
stage-number: 4
created-at: "2026-09-22T22:14:45Z"
updated-at: "2026-09-23T08:42:40Z"
metric-files-to-touch: 25
metric-step-count: 17
has-blockers: false
revision-count: 2
revisions:
  - rev: 1
    at: "2026-09-23T07:03:25Z"
    trigger: manual
    because: "auto-review — 7 issues found"
    changed: "re-grounded on the landed tactics work; placeholder step and line references corrected; bridge file added; option recommendations added to OQ-1 and OQ-2"
  - rev: 2
    at: "2026-09-23T08:42:40Z"
    trigger: review-feedback
    because: "product-owner answers OQ-1 = 1 and OQ-2 = 1"
    changed: "blockers cleared; engine-side steps 7 to 11 and page steps 12 to 15 written in full; 25 files, 17 steps; constraint resolutions finalised; queue-id aliasing risk added"
    snapshot: history/04-plan-viewer-lineup-tactics-1.md
consult-runs: []
tags: [viewer, lineup, tactics, substitutions, protocol-v3]
stack-source: confirmed
open-questions: []
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-viewer-lineup-tactics.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md, 04-plan-viewer-match-day.md, 04-plan-viewer-reports-recovery.md, 04-plan-integration.md]
  design: 02b-design.md
  contract: 02c-craft.md
  steer: steer.md
  implement: 05-implement-viewer-lineup-tactics.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine viewer-lineup-tactics"
---

# Plan: Lineup Editor and Tactics Panel

## The Plan

The previous revision stopped at two contract questions. The product owner answered both on 2026-09-23 (`po-answers.md` 08:37:28Z, `steer.md`). OQ-1 is option 1: the opening message carries the squads, a new `set-lineup` command returns the lineup, and `serve` holds before kick-off until the page sends `start`. OQ-2 is option 1: the tactics schema rides in the opening message, and a queued change's `detail` is `{ "patch": … }` or `{ "off": …, "on": … }`, routed into `Simulation::queue_change`. Protocol version 3 carries both, together with the match-day roster, running statistics, and condition message.

This revision writes the whole slice: 17 steps over 25 files. The engine half holds `serve` with a new held gate, applies the page lineup through a new `MatchConfig::with_setup` before the simulation is built, and bridges admitted socket changes into the engine queue through an inbox that the driver drains only while the match runs. The driver maps each engine change id back to the socket id, because the two queues number changes independently (`crates/engine/src/tactics/change.rs:228-236`). The page half adds the lineup rules, the pending-change list with its four state words, the lineup editor, the tactics panel, and the substitution picker, all as ordinary DOM. Fourteen implementation-detail decisions are recorded under Assumptions. No intent-bearing question is open.

This plan enables the integration slice to drive the whole loop from the page. The top open risk is the identifier split between the socket queue and the engine queue. If the bridge writes the wrong identifier, every chip stays at Queued. A driver test asserts that the applied event carries the acknowledged identifier.

## Current State

- **Page.** `web/index.html:26-29` holds the left column "Lineups" with the note "Coming in a later version.". `web/main.mjs` (270 lines) connects the socket and exposes `window.__touchline` (`:225`). No page module sends `start`: a search of `web/*.mjs` for a start command found none. `node --test web/tests/*.test.mjs` ran this run: 47 pass, 0 fail, on Node v24.14.0 (the slice text says Node 22; the machine now has 24).
- **Protocol.** `PROTOCOL_VERSION` is 2 (`crates/protocol/src/lib.rs:44`). `TeamRef` holds id, name, and the two kit colours (`crates/protocol/src/message.rs:14-23`). `QueueChange.detail` is an opaque `serde_json::Value` (`message.rs:88-96`). `ClientCommand` is `Start | Pause | SetSpeed | QueueChange` (`message.rs:100-107`). Every payload refuses unknown fields (`message.rs:1-3`). The document test checks every message and field in `MESSAGES` against `docs/reference/protocol.md` (`crates/protocol/tests/document.rs`).
- **Session.** `serve` builds `MatchConfig` (`crates/engine-cli/src/serve.rs:22-23`), builds the hello (`:35-59`), accepts the page (`:71`), starts the session with `Gate::new()` (`:74`), and only then builds the `Simulation` (`:111`). `Gate::new` produces at once (`crates/stream/src/control.rs:32-33`). `CommandContext::handle` admits a queue-change into `protocol::Queue` and writes a `match-event` row. Nothing reaches the engine (`control.rs:150-181`). The hello is also built in `record.rs:45-56` and `bench.rs:160-171`. `bench.rs:188` and `crates/stream/tests/common/mod.rs:159` use `Gate::new`. The fixture reader refuses any other protocol version (`crates/stream/src/record.rs:219`). `cargo test -p protocol -p stream` ran this run: 55 pass, 0 fail.
- **Engine.** `Simulation::queue_change(team, change)` assigns `ChangeId { tick: self.tick, n: self.queue.next }` from a counter that both teams share (`change.rs:228-236`). `Change` is `Tactics(TacticsPatch) | Substitution { off, on }`, with squad indices (`change.rs:30-33`). `TacticsPatch` holds optional formation and mentality, six optional instruction levels, and `roles: Vec<(usize, RoleDuty)>` (`crates/engine/src/tactics/mod.rs:75-81`). `MatchConfig::new` applies `ai::pre_match` to both teams (`crates/engine/src/sim.rs:75-80`). `with_tactics` shows how to rebuild the starters slice (`sim.rs:125-132`). `ai::Setup { tactics, lineup, bench }` (`crates/engine/src/ai.rs:71-75`) and `ai::role_fit` (`ai.rs:78`) exist. The snapshot already stores lineups, benches, and tactics (`crates/engine/src/snapshot.rs:593-600`), so a page-chosen setup survives a resume. `content/tactics.json:220` holds `"bench_size": 7`.
- **Driver.** `drive` waits on the gate, steps, and routes events. `Ids` maps squad indices to player ids. `match_event` writes `queue_id: Some(id.to_string())` from the engine id (`crates/engine-cli/src/stream_run.rs:181-201`).
- **Siblings.** `viewer-match-day` goes first and adds a `roster` to each hello team at the same builder sites, the nine statistics, and a `condition` message. Its plan is still `awaiting-input` on disk (`04-plan-viewer-match-day.md`, updated 07:30:49Z). The answer that clears it is recorded (`po-answers.md` 08:37:28Z, Q-1 = A, shipped in version 3).

## Simplicity Ladder

- Lineup legality on the page → rung 4 new-code: `web/lineup.mjs`. No platform check exists for football lineups. `ai::pre_match` picks a lineup and does not judge one.
- Lineup legality at the engine → rung 4 new-code: a structural check in `crates/stream/src/control.rs` (indices in range, distinct, eleven, bench within `bench_size`, slot 0 a goalkeeper). The team-file validator checks a file, not a selection.
- Pending-change state machine → rung 3 reuse for the vocabulary: `crates/protocol/src/command.rs` → `ChangeState::label()`, exact match, reuse as-is. The page module is rung 4.
- Drag and drop → rung 2 native-platform: the HTML Drag and Drop API, as an enhancement only.
- Keyboard operability → rung 2 native-platform: `<button>`, `<select>`, `<fieldset>`, and `:focus-visible` rings from `web/components/match-control.css`.
- Chip styling → rung 3 reuse: `web/tokens.css` and `web/components/match-control.css`, plus one small stylesheet.
- Role fit per slot → rung 3 reuse: `crates/engine/src/ai.rs` → `role_fit()`, exact match. The engine computes it into the opening message, so the page does not duplicate the weighting.
- Page-chosen setup → rung 3 reuse with modification: `crates/engine/src/sim.rs` → `MatchConfig::with_tactics()` is extended into `with_setup()`. The change is backward-compatible, because `with_tactics` is unchanged.
- Engine-side change application → rung 3 reuse: `Simulation::queue_change` (`change.rs:228`), as-is.
- Pre-match hold → rung 3 reuse with modification: `crates/stream/src/control.rs` → `Gate`, with a `held()` constructor next to `new()`. `wait_until_running` already blocks while not running.
- Socket-to-engine handoff → rung 1 stdlib: `std::sync::Mutex<Vec<…>>` shared through `Arc`, the same pattern `CommandContext` already uses for `events`.
- Tactics schema on the wire → rung 3 reuse: `TacticsSchema` already derives `Serialize` (`crates/engine/src/data/tactics.rs:38`). It is sent with `serde_json::to_value`, and the protocol crate holds it as `serde_json::Value`, so the protocol crate does not gain an engine dependency.

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist in this repository.

Repeat-deferral tripwire: `00-index.md` holds `runtime-evidence-deferrals: []`, so the tripwire does not fire.

## Likely Files / Areas to Touch

- `web/lineup.mjs` (new): lineup legality rules and reason text.
- `web/pending.mjs` (new): the pending-change list keyed by `change.queue_id`.
- `web/lineup-editor.mjs` (new): the pre-match DOM editor and the kick-off control.
- `web/tactics-panel.mjs` (new): the tactics panel rendered from the schema, and the pure `detailFor()` builder.
- `web/substitution-picker.mjs` (new): the substitution picker and the remaining count.
- `web/components/chip.css` (new): chip styles.
- `web/index.html`, `web/main.mjs`: regions, wiring, and the `pending()` and `lineup()` readers.
- `web/tests/lineup.test.mjs`, `web/tests/pending.test.mjs`, `web/tests/tactics-panel.test.mjs` (new).
- `crates/protocol/src/message.rs`, `crates/protocol/src/lib.rs`: the version 3 fields, `set-lineup`, and the typed change detail.
- `crates/stream/src/control.rs`, `crates/stream/src/lib.rs`: the hold, `set-lineup` handling, and the change inbox.
- `crates/engine/src/sim.rs`: `MatchConfig::with_setup`.
- `crates/engine-cli/src/hello.rs` (new), `crates/engine-cli/src/main.rs`: the shared opening-message builder.
- `crates/engine-cli/src/serve.rs`, `stream_run.rs`, `record.rs`, `bench.rs`: the hold, the bridge, and the builder call sites.
- `crates/stream/tests/common/mod.rs`, `crates/stream/tests/fixture.rs`: `TeamRef` literals.
- `docs/reference/protocol.md`: version 3.

## Proposed Change Strategy

The page learns every state from the engine. The chip mirrors the engine queue and never predicts an outcome. A rejected change shows the engine's `reason` text verbatim. Page modules follow the viewer-pitch convention: `.mjs`, no `package.json`, no framework, and pure modules tested under `node --test`. Panels are ordinary DOM, not canvas (slice Risks). Every UI step follows the design references: `design/typeset.md` for the type scale, and `design/polish.md` for focus rings and state words. The layout, fonts, and `--tl-` tokens come from `steer.md` and `02c-craft.md` inventory items 6, 9, and 10.

On the engine side, only `serve` holds before kick-off. `record`, `bench`, `resume`, and the stream tests keep `Gate::new`, so the headless and fixture paths behave as today. After the first `start`, `serve` takes the lineup that `set-lineup` stored, applies it with `with_setup(0, …)`, and then builds the `Simulation`. With no lineup, the computer manager's pre-match setup stands, as the product owner ruled. An admitted queue-change is parsed into a typed detail on the socket thread and placed in an inbox. The driver drains the inbox on the simulation thread after each `wait_until_running`, maps each entry onto `Change`, and calls `queue_change(0, …)`. It remembers `engine id → socket id`, so the change event names the identifier the page was given. This plan cites no NFR as the reason for a mechanism choice.

## Step-by-Step Plan

1. **Vocabulary check.** Remove the left-column note "Coming in a later version." (`web/index.html:28`) when step 15 adds the editor region. Before the commit, search `web/`, `crates/`, and `docs/` for internal workflow vocabulary. Make this search the last step before the commit.
2. **Lineup rules.** Write `web/lineup.mjs` with `checkLineup({ slots, bench, squad, benchSize })`. It returns `{ legal, reason }` and reports the first failure in this order: fewer than eleven starters; slot 0 empty or not a goalkeeper; a player placed twice; a bench larger than `benchSize`. Each reason names the count or the player, for example "Ten starters; a match needs eleven." `benchSize` comes from `hello.tactics.ai.bench_size`.
3. **Lineup tests.** Write `web/tests/lineup.test.mjs`: one test per illegal case, one legal case, and one legal empty bench.
4. **Pending-change list.** Write `web/pending.mjs` with `createPendingList()` and `queued(ack, label)`, `rejectedAtQueue(reject, localId)`, `onChangeEvent(event)`, `dismiss(id)`, and `chips()`. States are `queued → applies-now → applied`. The chip leaves the list after it shows Applied. `rejected` keeps the reason until the manager dismisses it. Every chip carries the word from `ChangeState::label()`. An event with an unknown `change.queue_id` is ignored and logged as a `viewer-event` row. Field names: `change.queue_id`, `change.state`, `change.rejected_reason`, `change.applied_tick`.
5. **Pending tests.** Write `web/tests/pending.test.mjs`: the full lifecycle, a queue-time rejection, a stoppage rejection, an unknown queue id, and two chips that resolve out of order.
6. **Chip styles.** Write `web/components/chip.css` with `--tl-success`, `--tl-warning`, `--tl-danger`, and `--tl-fg-muted` (`steer.md`). Always render the state word as text. Use the 28-pixel `match-control` height.
7. **Protocol version 3 types.** In `crates/protocol/src/message.rs`:
   - Add `SquadEntry` with `player.id`, `player.name`, `player.shirt`, `player.position`, `player.natural_fitness` (0 to 100), and `role_fit` (one 0 to 100 integer per role, in tactics-file order).
   - Add `TeamSetup` with `lineup` (eleven squad indices, slot order) and `bench` (squad indices).
   - Give `TeamRef` a `squad` field (`Vec<SquadEntry>`, `#[serde(default)]`) and a `setup` field (`Option<TeamSetup>`, `#[serde(default)]`).
   - Give `Hello` a `tactics` field (`serde_json::Value`, the tactics file) and a `substitutions` field (`{ limit, windows }`).
   - Add `ClientCommand::SetLineup(SetLineup { lineup: Vec<u16>, bench: Vec<u16>, patch: Option<PatchWire> })`, named `set-lineup`. The optional patch carries the pre-match formation, mentality, instructions, and roles.
   - Add `ChangeDetail`, an untagged enum of `Patch { patch: PatchWire }` and `Swap { off: u16, on: u16 }`. `PatchWire` holds `formation?`, `mentality?`, `instructions?` (six optional levels), and `roles?` (a list of `{ squad, role, duty }`), all indices into the schema the hello carries. Deny unknown fields.
   - Add `QueueChange::detail_typed()`, which returns the parsed detail or an error that names the fault. It also checks that a patch detail goes with `tactics` and a swap detail goes with `substitution`.
   In `crates/protocol/src/lib.rs`, set `PROTOCOL_VERSION` to 3 and give the reason in its doc comment: a new command, the pre-match hold, and a field that changed meaning (`detail` is now read). If the match-day work already raised the version to 3, add only this slice's paragraph. Add `set-lineup` and the new hello fields to `MESSAGES`, and update the test `TeamRef` literal (`lib.rs:252`). Add unit tests: each detail shape parses, a mismatched kind and detail is refused, and an unknown field is refused.
8. **Hold, set-lineup, and the inbox.** In `crates/stream/src/control.rs`:
   - Add `Gate::held()`, which is not running until the first `start`, and a `started()` flag. A gate made with `Gate::new()` counts as started, so `record`, `bench`, and the stream tests behave as today.
   - Give `CommandContext` a `pre_match: Arc<PreMatch>` (the stored lineup plus a `LineupRules` of the home squad's positions and `bench_size`) and an `inbox: Arc<Mutex<Vec<Admitted { queue_id, detail }>>>`.
   - For `set-lineup` before the first `start`: run the structural check (indices in range, all distinct, exactly eleven, bench at most `bench_size`, slot 0 a goalkeeper), store the lineup, and answer `ack`. Otherwise answer `reject` with a reason in words, for example "the match has started; a lineup can be set only before kick-off".
   - For `queue-change` before the first `start`: answer `reject` with "the match has not started; set the pre-match tactics with the lineup". After it, call `detail_typed()` before `Queue::submit`. An unreadable detail becomes a `reject` and a Rejected `match-event` row, with the reason. An admitted change is pushed to the inbox with its `queue_id`.
   Re-export the new types from `crates/stream/src/lib.rs`. Tests: a held gate blocks until `start`; `set-lineup` after `start` is refused; an illegal lineup is refused with its reason; an admitted change reaches the inbox with the acknowledged id; a bad detail is refused and reaches no inbox.
9. **Page-chosen setup in the engine.** In `crates/engine/src/sim.rs`, add `MatchConfig::with_setup(team, lineup, bench)`. It sets `teams[team].lineup` and `bench`, keeps the pre-match tactics, and rebuilds the starters slice the way `with_tactics` does (`sim.rs:125-132`). The content hash is unchanged, because the lineup is not a content file. Unit test: the first eleven player records of a config with a chosen setup follow the chosen squad indices in slot order.
10. **Shared opening-message builder.** Write `crates/engine-cli/src/hello.rs` with `hello(config, content, ids…) -> Hello` and `team_ref(config, team, team_file, content) -> TeamRef`. Squad entries come from the team file (id, name) and the squad (shirt, position, `natural_fitness`, and `ai::role_fit` for every role, rounded). `setup` is the current lineup and bench. `tactics` is `serde_json::to_value(&content.tactics)`. `substitutions` comes from the rule pack. Declare the module in `crates/engine-cli/src/main.rs`. Replace the literal builders in `serve.rs:35-59`, `record.rs:45-56`, and `bench.rs:160-171`. If the match-day work has already added a shared builder for its `roster`, extend that builder instead of adding a second one.
11. **Serve hold and the bridge.** In `crates/engine-cli/src/serve.rs`, use `Gate::held()`. After `Session::start`, call `gate.wait_until_running()`. If it returns `false`, because the client left during the hold, finish the session and return the short-run exit code. Otherwise take the stored lineup, apply `with_setup(0, …)`, apply its optional patch to the home pre-match tactics through `with_tactics(0, …)`, and then build the `Simulation`. In `crates/engine-cli/src/stream_run.rs`, give `Drive` an `inbox: Option<&Inbox>`. After each `wait_until_running`, drain the inbox: map `Patch` onto `TacticsPatch` and `Swap` onto `Change::Substitution`, call `sim.queue_change(0, change)`, and store `engine id → socket id` in `Ids`. `match_event` writes the socket id when one is stored. Tests in `stream_run.rs`: an inbox with a mentality patch and a substitution, drained into a 3-minute seed-42 match, yields change events whose `change.queue_id` equals the socket ids. A change placed while the gate is paused is queued on the first running tick and applies at the next opening stoppage, not on the resume tick. Update `TeamRef` literals in `crates/stream/tests/common/mod.rs` and `crates/stream/tests/fixture.rs`.
12. **Lineup editor.** Write `web/lineup-editor.mjs`. It holds a formation picker (from `hello.tactics.formations`), eleven slots, and up to `bench_size` bench slots, with the home squad from `hello.teams[0].squad` and a start from `setup`. Click-to-place is the primary path and drag is an enhancement. Each slot shows role fit (a 0 to 100 number with a word band) and fitness (`player.natural_fitness`, number and word). The inline reason comes from step 2. The kick-off control (`data-testid="kick-off"`) is disabled while the lineup is illegal. On press, it sends `set-lineup` with the lineup, the bench, and a patch for any pre-match formation or tactics edit, waits for `ack`, then sends `start`. A `reject` shows its reason inline and does not start. Each slot has `data-testid="slot-<n>"`.
13. **Tactics panel.** Write `web/tactics-panel.mjs`. It renders mentality, the six instructions, and a role and duty per slot from `hello.tactics`, so a new instruction needs no UI change. It is available during play and while paused. Each edit sends `queue-change` with `change.kind: "tactics"` and `detail: detailFor(edit)`, and it creates a chip. Write `web/tests/tactics-panel.test.mjs` for `detailFor()`: mentality only, one instruction, and a role change keyed by squad index.
14. **Substitution picker.** Write `web/substitution-picker.mjs`. The manager picks an on-pitch player and a bench player, and the picker sends `{ off, on }` by squad index. The remaining count is `hello.substitutions.limit` minus the applied home `substitution` events. On Applied, it calls the match-day lineup panel's swap (by `player.id` and `player.secondary_id`). A rejection shows the engine's reason on the chip.
15. **Page wiring.** In `web/index.html`, replace the left-column placeholder with the editor region. After kick-off, the region hands over to the match-day lineup panel. Add the tactics and substitution region to the right column under the feed. In `web/main.mjs`, wire the modules to the socket and route `ack`, `reject`, and change events to the pending list. Extend `window.__touchline` with `pending()` (chips with `queue_id`, state, reason, `queued_tick`, and `applied_tick`) and `lineup()` (slots, bench, legal, and reason). After kick-off, the home lineup panel reads its rows from the accepted `set-lineup`, because the hello's `setup` was sent before the choice.
16. **Protocol document.** Update `docs/reference/protocol.md`. Set the version to 3. In Connecting, describe the hold: `serve` sends nothing but the hello until `start`. Document the hello `squad`, `setup`, `tactics`, and `substitutions` fields, and add a `### set-lineup` section with its fields. Replace the queue-change text at `:187-191` with the two detail shapes and the fact that the engine now applies the change. Run the document test.
17. **Checks.** Run `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`, `node --test web/tests/*.test.mjs`, and the vocabulary search from step 1.

## Verification Strategy

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-a illegal lineup disables kick-off with an inline reason; legal enables | Claude_Browser drive with a screenshot per case (web-2), plus `lineup.test.mjs` | Windows 11, Node 24, `cargo run -p engine-cli -- serve --seed 11 --web web` — yes | `window.__touchline.lineup()`; `data-testid` on slots and `kick-off`; the hello squad (steps 7, 10) | `lineup.test.mjs` (proxy, not a pass) → pre-registered deferral cleared by re-running the drive once the socket build runs |
| AC-b mentality change: chip, then "Tactical change applied" at the next dead ball | Claude_Browser drive against the live engine, seed 42 (web-2) | live engine — yes | the bridge (steps 8, 11); `pending()`; the match-day feed line | `stream_run` bridge test + `pending.test.mjs` (proxy) → deferral as above |
| AC-c substitution applies, lineup panel swaps, count decrements | Claude_Browser drive, seed 42 (web-2) | as AC-b | `hello.substitutions`; the match-day lineup panel's swap | as AC-b |
| AC-d sixth substitution rejected with the engine's reason | Claude_Browser drive after five applied substitutions (web-2) | as AC-b | as AC-c | `pending.test.mjs` rejection case (proxy) → as AC-b |
| AC-e role change while paused applies at the next dead ball, not at resume | Claude_Browser drive reading `pending()` `queued_tick` and `applied_tick`, and the resume tick from the playback state (web-2) | as AC-b | inbox drained after `wait_until_running` (step 11); `pending()` exposes both ticks | the `stream_run` paused-change test (proxy) → as AC-b |
| AC-f keyboard-only operation (observable: false) | Claude_Browser drive: Tab, Shift+Tab, Enter, Space, and arrow keys only, reading `document.activeElement` and the computed focus outline per step (web-2) | yes | native controls; `:focus-visible` rings | screenshot review of the focus order |

Constraint resolutions:
- AC-a: `constraint-resolution: prerequisite-slice: viewer-lineup-tactics` (the squad path is steps 7 and 10 of this plan). `wall-ownership: code-owned`.
- AC-b, AC-d, AC-e: `constraint-resolution: prerequisite-slice: viewer-lineup-tactics` (the bridge is steps 8 and 11). `wall-ownership: code-owned`.
- AC-c: `constraint-resolution: prerequisite-slice: viewer-match-day` (its lineup panel, ordered first in `03-slice.md`), plus steps 7, 10, and 14 of this plan. `wall-ownership: code-owned`.
- AC-f: no environment wall.

## Test / Verification Plan

### Automated checks

- `node --test web/tests/*.test.mjs` (the glob form; 47 of 47 pass this run). New: `lineup`, `pending`, and `tactics-panel` tests.
- `cargo test --workspace`. New Rust tests: detail parsing and refusal (protocol); the held gate, the `set-lineup` verdicts, and the inbox (stream); `with_setup` (engine); the bridge id carry-through and the paused-change timing (engine-cli). `cargo test -p protocol -p stream` passes 55 of 55 this run as the baseline.
- `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`.
- The document test (`crates/protocol/tests/document.rs`) holds `set-lineup` and the new fields.
- The vocabulary search before the commit.

### Interactive verification (human-in-the-loop)

Platform: web. Tool: the Claude_Browser in-app browser pane (`stack.available-mcp`, the shape's Q29 choice). No other driver is introduced. Evidence goes to `verify-evidence/viewer-lineup-tactics/`: one screenshot per step and a JSON dump of `window.__touchline.pending()` and `.lineup()`.

- **AC-a.** Run `cargo run -p engine-cli -- serve --seed 11 --web web` and open the printed page address. The page shows the editor and the engine holds, so no tick frames arrive. Remove a starter: the kick-off control is disabled and the reason reads "Ten starters; a match needs eleven.". Take a screenshot. Repeat for an outfield player in slot 0 and for one player in two slots. Restore a legal lineup: the control is enabled. Pass: `lineup().legal` matches the control's `disabled` state in all four cases, and the reason text is visible.
- **AC-b.** Run with `--seed 42`. Press kick-off. During play, set a different mentality. A chip reads "Queued". At the next dead ball, the chip reads "Applied" and then leaves, and the feed shows "Tactical change applied". Pass: the `pending()` entry's `applied_tick` equals the tick of a stoppage event.
- **AC-c.** Queue a substitution. At the next dead ball, the lineup panel shows the incoming player in place of the outgoing one, and the count goes from 5 to 4.
- **AC-d.** Apply five substitutions over three windows, then queue a sixth. The chip reads "Rejected" with the engine's text "substitution limit reached (5 of 5)" or "no substitution window left (3 of 3)", whichever the engine returns.
- **AC-e.** Pause. Change one slot's role. Resume, and record the resume tick. Pass: `applied_tick` is greater than the resume tick and equals the next stoppage's tick.
- **AC-f.** Using only the keyboard, reach every slot, the formation picker, the kick-off control, every tactics control, and both picker lists, and operate each. Record `document.activeElement` and whether `:focus-visible` shows an outline at each step.

## Risks / Watchouts

- **Two queues, two identifiers (high).** See The Plan. The mitigation is the map and its test (step 11).
- **The hold changes every `serve` session.** A client that never sends `start` now receives only the hello. The product owner ratified the hold. Only `serve` holds, and the page sends `start` from the kick-off control.
- **Version 3 refuses version 2 recordings.** The fixture reader refuses any other version (`record.rs:219`). No recording is committed to the repository, and the tests record fresh ones.
- **Builder collision with the match-day slice.** Both slices change the same three hello sites. Extend the builder that lands first, and raise the version only once.
- **The hello's home `setup` goes stale after `set-lineup`.** The page uses its accepted lineup after kick-off. Wire slots follow the chosen lineup's slot order, because the starters are rebuilt in slot order (step 9).
- **Drag and drop works against accessibility.** Click-to-place stays primary.
- **The instruction count can grow.** The panel renders from the schema.
- **Resume.** The snapshot already stores the lineup and the pending queue. Restoring chips after a resume belongs to `viewer-reports-recovery`.

## Dependencies on Other Slices

- `tactics-and-ai` (complete): the tactics model, the substitution limit, the change events, and the rejection reasons.
- `viewer-match-day` (defined; ordered first): the lineup panel that a substitution updates, the feed line "Tactical change applied", and the version 3 roster at the same builder sites.
- `viewer-pitch` (complete): `web/tokens.css`, `web/components/match-control.css`, and `window.__touchline`.

## Assumptions

- **A1** (class: implementation-detail): This run writes only its own three plan files and the history snapshot `history/04-plan-viewer-lineup-tactics-1.*`. `04-plan.md`, `00-index.md`, and `INDEX.md` are left to the run driver. Why: the driver withheld index writes for this run.
- **A2** (class: implementation-detail): AC-f is verified by an in-app browser keyboard drive, not a Testing Library test. Why: the repository has no `package.json` and no DOM under `node --test`, and Claude_Browser is in `stack:`.
- **A3** (class: implementation-detail): The chip leaves after Applied. A Rejected chip stays until dismissed. Why: the slice says a chip "clears" on apply and requires the rejection reason to show.
- **A4** (class: implementation-detail): The page shows one legality reason at a time, in a fixed order. Why: this is the smallest inline message that meets AC-a.
- **A5** (class: implementation-detail): No consult ran. Triggers `appetite-medium-or-larger` and `touches-external-api` (the socket protocol) hold, but the product owner excluded `consult` at intake (`stack.excluded-by-po`).
- **A6** (class: implementation-detail): `node --test` uses the file glob, because the directory form fails on this machine.
- **A7** (class: implementation-detail): This run is a review-and-fix of the existing plan, driven by the recorded product-owner answers. The discovery interview does not run. Its implementation questions are answered below (A8 to A14) in the user's best interest, at the least cost and blast radius.
- **A8** (class: implementation-detail): Only `serve` holds before kick-off. `record`, `bench`, `resume`, and the stream tests keep `Gate::new`. Why: the product owner's rule names the session a page drives, and the headless paths have no client to send `start`.
- **A9** (class: implementation-detail): The field names and nesting inside the ratified options are the ones in step 7: `squad`, `setup`, `tactics`, `set-lineup { lineup, bench }`, and a detail patch of schema indices keyed by squad index. Why: the product owner ratified "mirroring `Change`", which uses indices. The key style follows the existing dotted `player.*` names.
- **A10** (class: implementation-detail): The opening message also carries `substitutions { limit, windows }`. Why: the slice scope requires a remaining-count display, and the count must come from the engine's rule pack, not page code. It is an additive, read-only field inside the version 3 bump that the product owner already ratified. It adds no behaviour.
- **A11** (class: implementation-detail): The engine computes role fit per player per role into the opening message with the existing `ai::role_fit`. Fitness before kick-off is the `natural_fitness` attribute. Why: this reuses engine logic instead of copying it into the page. Every player starts at energy 1.0, so `natural_fitness` is the only pre-match fitness value the engine holds.
- **A12** (class: implementation-detail): The socket queue id is carried through by a driver-side map, not by changing the engine's id scheme. Why: the computer manager shares the engine counter, and a map leaves `crates/engine` queue code untouched.
- **A13** (class: implementation-detail): A malformed detail is refused at queue time with a reason. A well-formed change that is illegal on the pitch (limit reached, player not on the pitch) is refused by the engine at the stoppage. Why: the engine owns the football rules (AC-d names "the engine's rejection reason"), and the socket owns the wire format.
- **A14** (class: implementation-detail): Pre-match formation and tactics edits ride in `set-lineup` as an optional patch of the same `PatchWire` shape, and they are applied before the simulation is built. A `queue-change` before the first `start` is refused with a reason. Why: a change queued before kick-off would otherwise wait for the first dead ball, not apply at kick-off. Reusing the patch shape adds no second tactics format to the wire.

## Blockers

None. OQ-1 and OQ-2 were answered by the product owner (`po-answers.md` 2026-09-23T08:37:28Z; `steer.md` "Product-owner answers to plan questions").

## Freshness Research

No new dependency is proposed. The page uses the built-in HTML Drag and Drop API, native form controls, and the built-in `node:test` runner, which is already in use. The Rust side uses `serde`, `serde_json`, and `std::sync`, which are already in the workspace (`crates/protocol/Cargo.toml`, `crates/stream/Cargo.toml`). `#[serde(untagged)]` for `ChangeDetail` is a standard serde attribute, already a dependency. No web research ran, because no library choice is open. The Node runtime on this machine is v24.14.0. `node:test` and the glob invocation behave as they did under 22 (47 of 47 pass this run).

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine viewer-lineup-tactics`, after `viewer-match-day` is implemented, because this slice depends on its lineup panel and shares its hello builder. Compact the session first.
- **Option B:** `/wf plan football-manager-match-engine viewer-match-day`. Re-plan it with its recorded Q-1 answer first, so the shared builder and the single version bump are planned once.
- **Option C:** `/wf slice football-manager-match-engine`, if the version 3 protocol work should become its own slice ahead of both viewer slices.
