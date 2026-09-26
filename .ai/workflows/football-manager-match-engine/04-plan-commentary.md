---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: commentary
status: complete
stage-number: 4
created-at: "2026-09-22T22:19:17Z"
updated-at: "2026-09-22T22:19:17Z"
metric-files-to-touch: 20
metric-step-count: 13
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, commentary, text, content, protocol]
stack-source: confirmed
steering-honored:
  - "no visual change to the page: the lines ride on the event message, and the match feed that shows them belongs to the match-day screen"
  - "a card line names the card in words (booked, second yellow, sent off), so colour never carries the card state alone"
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-commentary.md
  shape: 02-shape.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md]
  implement: 05-implement-commentary.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine commentary"
---

# Plan: Context-Aware Commentary

## The Plan

The referee now emits twelve kinds of play event: kick-off, goal, half-time, full time, offside, foul, card, and five restarts. Each event carries the minute and the score. An offside, a foul, or a card also names the player. A goal, a restart, and a kick-off name no player (`crates/engine/src/rules/mod.rs:89`, `:122`, `:321`). The engine keeps player identifiers and no player names (`crates/engine/src/team.rs:46`). No text exists anywhere. The tree is green at 192 Rust tests in 30 suites on `837cb5c`.

Five decisions set the build. First, the commentator lives in the engine crate and reads engine events. The shared match driver in `crates/engine-cli/src/stream_run.rs` attaches its line to the event message as one optional `commentary` field. The socket, the recorded fixture, and `events.jsonl` all get the line from one place. Second, the lines are data: one English file, `content/commentary/en.json`. The file holds template sets keyed by event kind, with conditions on minute band, score state, form, and repeat count. The loader refuses a file that leaves any kind with fewer than 3 unconditional lines, or that uses a placeholder the kind cannot fill. Third, selection is deterministic and takes no engine random draw. The commentator prefers the most specific set and does not repeat a line for a kind within ten minutes of play. Fourth, the engine names the last kicker on a goal and the taker on a restart. The last kicker is cleared at every dead ball, so the snapshot layout does not change. Fifth, protocol version 2 stays, under the additive-field judgement already written beside `PROTOCOL_VERSION`.

Implement touches 20 files, 6 of them new, in 13 steps. The match-day screen then shows these lines in its feed. The top risk is repetition: a match has about 40 throw-ins, and the first file has at least three unconditional lines per kind. The ten-minute no-repeat rule and the criterion that measures distinct lines hold that risk.

## Current State

- Branch `feat/football-manager-match-engine` at `837cb5c`. Five slices are implemented and verified. `cargo test --workspace` passes 192 tests in 30 suites with 0 failures (run this session).
- `EngineEventKind` (`crates/engine/src/sim.rs:95-115`) has twelve kinds. `EngineEvent` (`sim.rs:133-156`) is `Copy` and carries `tick`, `kind`, `team`, `scores`, `minute`, `minute_added`, `player`, `secondary`, `card`, `advantage`, `added_time_s`, and `spot`.
- Only offside, foul, and card set `player`. `goal()` (`rules/mod.rs:119-130`) sets no player. `open_dead_ball()` picks the restart `taker` (`rules/mod.rs:296`) and does not put it on the restart event (`rules/mod.rs:321`). `place_kick_off()` pushes the event (`rules/mod.rs:89-90`) before it picks the kicker (`rules/mod.rs:106`).
- The engine does not track which player kicked last. `apply_kick` (`sim.rs:444-460`) records the kicking team in `last_touch`, and the snapshot stores `last_touch` (`snapshot.rs:475`, `:579`).
- The engine's `Team` holds `name` and `player_ids` (`team.rs:43-46`) and no player names. The team file holds `PlayerEntry::name` (`data/team.rs:93`). The snapshot stores `attack_x` and the active slots of each team (`snapshot.rs:477-484`) and no identity fields. The teams come from the config on resume.
- `Content` (`data/mod.rs:208-215`) loads three files, and its digest feeds `content_hash` and the snapshot check (`sim.rs:66-76`). `load_json` (`data/mod.rs:140-199`) version-checks, deserializes, runs garde validation, and emits `content.loaded`. The garde custom-validator idiom is at `data/rules.rs:97`, `:120-135`, `:178-189` (garde 0.23).
- `stream_run::drive` (`crates/engine-cli/src/stream_run.rs:37-89`) maps every engine event through `match_event` (`:127-157`) for `serve`, `record`, and `bench`. `simulate` and `resume` route no events. The stream crate's test helper (`crates/stream/tests/common/mod.rs:180-210`) has its own mapping.
- `MatchEvent` (`crates/protocol/src/event.rs:88-146`) uses `deny_unknown_fields`. `MESSAGES` lists the event fields (`crates/protocol/src/lib.rs:125-148`), and `crates/protocol/tests/document.rs` checks that every field is documented in `docs/reference/protocol.md:83-107`. The comment at `lib.rs:21-33` records that version 1 survived an added field.
- `tactics-change` events come from the command queue (`crates/stream/src/control.rs:150-180`). They carry no team and no player, and they record the manager's own request.
- A tactics plan is in progress in another session: `bench-baseline/tactics-and-ai/` holds a baseline on `837cb5c` taken at 22:13Z. `04-plan.md` puts `tactics-and-ai` before `commentary`, so this slice can start from a later commit that has more event kinds.

## Simplicity Ladder

| Capability | Rung | Choice |
|---|---|---|
| Template file loading and validation | rung 3 reuse | `data::load_json` (`data/mod.rs:140`) and the garde `custom(...)` idiom of `every_kind_once` (`data/rules.rs:178`); the file gets the same fail-closed refusals as the other content. |
| Placeholder fill | rung 4 new code | Rust formatting needs a literal format string. Repro: `format!(t, player = player)` with `t: String` fails with "format argument must be a string literal" (rustc 1.92, scratch repro this session). No templating crate is in `Cargo.lock`. The fill is a scan of `{name}` tokens of about 40 lines, and a new dependency would cost more than that. |
| Context selectors (minute band, score state, form, repeat) | rung 4 new code | No in-repo symbol derives the score state or the form. The minute band reads `EngineEvent::minute` and `minute_added`, which the match clock sets (`rules/clock.rs:120-132`). |
| Variant selection without repeats | rung 4 new code | A per-kind history ring and a seed-derived rotation start. `EngineRng` is not used: a draw would change every match. |
| Player names on events | rung 3 reuse with modification | `Team::from_file` (`team.rs:67-90`) already reads each `PlayerEntry`, and it now also keeps the names. `Simulation::player_ids()` (`sim.rs:276-281`) is the template for `player_names()`. |
| Scorer and taker on events | rung 3 reuse with modification | The taker from `restart::taker` (`rules/mod.rs:296`, `:106`). `apply_kick` (`sim.rs:444`) also records the kicker's roster index. |
| Line on the wire | rung 3 reuse | The optional-field and builder pattern of `MatchEvent` (`event.rs:127-145`, `:180-210`). |
| Kind enumeration and codes | rung 3 reuse | The `ALL` and `code()` idiom of `EventType` (`event.rs:33-66`) and `StoppageKind`. |

## Applied Learnings

No applicable learnings found: `.ai/solutions/INDEX.md` does not exist and `.ai/sdlc-config.json` does not exist. `runtime-evidence-deferrals` in `00-index.md` is empty, so the repeat-deferral tripwire does not fire.

## Likely Files / Areas to Touch

- `content/commentary/en.json` (new), `content/README.md`: the English lines and the modder reference.
- `crates/engine/src/commentary/mod.rs`, `templates.rs`, `context.rs`, `render.rs` (new): the commentator.
- `crates/engine/src/lib.rs`, `data/mod.rs`: the module and the file path.
- `crates/engine/src/sim.rs`, `rules/mod.rs`, `team.rs`: player names, the last kicker, the players on goal, restart, and kick-off events, `EngineEventKind::ALL` and `code()`.
- `crates/engine/tests/commentary.rs` (new): the four criteria and the loader refusals.
- `crates/protocol/src/event.rs`, `lib.rs`, `docs/reference/protocol.md`: the optional `commentary` field.
- `crates/engine-cli/src/content.rs`, `stream_run.rs`, `bench.rs`, `record.rs`, `serve.rs`: loading and attaching the line.

The full topology with line estimates is in `04-plan-commentary.yaml`.

## Proposed Change Strategy

The **commentator** (named mechanism) is a per-match object in the engine crate: `Commentator::for_match(&Commentary, &Simulation)`. It reads the club names, the player names in roster order, the match length, and the seed. `line(&EngineEvent) -> String` returns one line and records the event in its own history. The engine loop does not hold the commentator, so the tick path, the snapshot, and determinism do not change. The shared match driver builds one commentator per match and attaches the line to each mapped event. The slice definition asks for the generator inside the engine and the line as a field on the event record. This design meets both requirements and changes a single call site.

The **template file** is `content/commentary/en.json`, schema version 1. It holds a list of template sets `{ event, when, lines }`. `event` is the protocol spelling of the kind. `when` holds optional conditions: `minute` (`early`, `late`, `added-time`, `first-half`, `second-half`), `score` (`level`, `leading`, `trailing`, and for goals `opener`, `equaliser`, `go-ahead`, `extends-lead`, `rout`, `consolation`), `form` (`hot`, `cold`, `booked`), `repeat` (`first`, `again`, `streak`), `card`, `advantage`, and `own_goal`. Placeholders are `{player}`, `{other_player}`, `{team}`, `{opponent}`, `{home}`, `{away}`, `{home_score}`, `{away_score}`, `{score}`, `{minute}`, and `{added_minutes}`. The loader refuses a file in which a kind has fewer than 3 unconditional lines that use only its guaranteed placeholders. It also refuses an unknown placeholder, a placeholder the kind cannot fill, and a stray brace. These checks make AC-1 and AC-4 structural properties of every file the engine accepts. The tests confirm them on the shipped file and on a full match.

**Selection.** The commentator collects the sets whose conditions all hold and whose placeholders all have values. It walks them from most specific to least specific. Inside a set, it starts at a rotation index derived from the seed and the kind. It takes the first line not used for this kind within the last ten minutes of play. When every candidate line was used in that window, it takes the least recently used line. When no set can be filled, it writes a built-in line that names the club and counts a fallback. A fallback also emits the tracing signal `commentary.fallback`: this is the dark-path signal, and the full-match test requires zero fallbacks. With 3 or more unconditional lines per kind, five events of one kind within ten minutes give at least three distinct lines (AC-2).

**Players on events.** `Simulation` gains `last_kicker: Option<usize>`, set in `apply_kick`. `goal()` names it. When that player belongs to the other club, the goal is an own goal. `open_dead_ball()` and `place_kick_off()` name the taker and clear `last_kicker`. The snapshot is written only when a stoppage is announced, and each announcement comes from one of those two functions. At every snapshot, therefore, `last_kicker` is `None`. `blank()` starts it at `None`, a resumed match continues exactly, and the snapshot layout does not change. A unit test pins this invariant over the seed-42 match. The change adds no random draw, so tick records stay byte-identical. Step 2 proves this with a before-and-after hash of a tick file.

**Contract.** `MatchEvent` gains `commentary: Option<String>`, which is skipped when absent. `tactics-change` verdicts carry none. Under the rule at `crates/protocol/src/lib.rs:21-33`, version 2 stays: no field is removed, no field changes meaning, and both producers change in the same commit. The `player.id` row in `docs/reference/protocol.md` gains the scorer and the taker. That row now documents values that the field already allowed.

Performance: NFR-1 (`yields-to: C2`) does not govern a mechanism choice here. The commentator runs once per event, off the tick path. The benchmark tripwire still applies because `bench` drives the same driver.

## Step-by-Step Plan

1. **Baseline.** Before you change any code, run `cargo build --release -p engine-cli`. Then, with `SM_DATA_DIR` set to a scratch folder, run `target/release/engine-cli.exe bench --seed 42 --matches 5 --json` three times on the current `HEAD`. Write stdout, stderr, and the exit code of each run to `.ai/workflows/football-manager-match-engine/bench-baseline/commentary/bench-<n>.*`, and write the commit to `commit.txt`. Record the median `bench.cpu_us_per_tick` and `bench.peak_mem_mb` in `05-implement-commentary.md`. Also run `engine-cli simulate --seed 42 --ticks-out <scratch>/before.ticks --no-snapshot` and record the SHA-256 of `before.ticks`.
2. **Players on events.** In `team.rs`, add `player_names` and fill it in `from_file`. In `sim.rs`, add `EngineEventKind::ALL` and `code()`, the `last_kicker` field (set in `apply_kick`, `None` in `blank()`), `player_names()`, `seed()`, and `minutes()`. In `rules/mod.rs`, name the last kicker in `goal()`. In `open_dead_ball()`, name the taker on the restart event and clear `last_kicker`. In `place_kick_off()`, pick the kicker before you push the event, name the kicker, and clear `last_kicker`. Add two unit tests in `sim.rs` over the seed-42 90-minute match. The first test asserts that `last_kicker` is `None` on every tick where `stoppage()` is `Some`. The second test asserts that every event except half-time and full time has `player` set. Run `cargo test -p engine`; all tests must pass. Repeat the step 1 `simulate` run into `after.ticks`. The SHA-256 of the two files must be equal.
3. **Render.** Write `commentary/render.rs`: `placeholders(line)` and `fill(line, &Values) -> Option<String>`. Unit tests: every placeholder fills; a missing value gives `None`; a line with no placeholder is unchanged; a stray `{` or `}` is reported.
4. **Template file schema.** Write `commentary/templates.rs`: `CommentaryFile`, `TemplateSet`, and `When` with `deny_unknown_fields`, plus the garde validators described in the change strategy. Also write `Commentary::load(&ContentDir)` through `data::load_json` with `COMMENTARY_VERSION = 1`. In `data/mod.rs`, add `COMMENTARY_FILE`. Each refusal names the kind, the placeholder, or the line index.
5. **Situation.** Write `commentary/context.rs` as pure functions over the event, the commentator's history, and the match length:
   - Minute bands: `early` is before one sixth of the match. `late` is from eight ninths of the match, or in added time of the last half. `added-time` is any tick that has an added minute.
   - Score states are from the event team's side. `rout` means a lead of 3 or more.
   - Form: `hot` means the team scored 2 or more goals in the last sixth of the match. `cold` means it conceded 2 or more. `booked` means the offender already holds a yellow card.
   - Repeat: `again` means the same kind occurred within ten minutes. `streak` means 3 or more of the same kind within ten minutes.

   Write a unit test for each value. Include the edge cases: minute 88 of 90 is `late`; a goal that makes it 1-1 is `equaliser`; a goal that makes it 4-0 is both `rout` and `extends-lead`.
6. **Commentator.** Write `commentary/mod.rs`: `MatchNames`, `Commentator::for_match`, `line`, and `fallbacks`, with the selection order and the `commentary.fallback` signal. In `lib.rs`, add `pub mod commentary` and the re-exports. Unit tests: the most specific set wins; no line repeats within the window while an unused line exists; the result is the same for the same seed and events.
7. **English lines.** Write `content/commentary/en.json`. Give every kind in `EngineEventKind::ALL` at implement time at least 3 unconditional lines. If the tactics slice has landed, this includes its kinds. Add the context sets listed in `04-plan-commentary.yaml`. Name the card in words in every card line. Do not put `{minute}` in unconditional lines, because the feed shows its own minute stamp.
8. **Modder reference.** In `content/README.md`, add `commentary/en.json` to the files table. Add a section that covers the set shape, the conditions and their values, each placeholder with the kinds that fill it, the three-line rule, and the selection order.
9. **Protocol.** In `event.rs`, add the `commentary` field and builder, and add the key to the round-trip test. In `lib.rs`, add `commentary` to the `MESSAGES` event list. Add a sentence to the `PROTOCOL_VERSION` comment that says version 2 survives the added optional field, and give the reason. In `docs/reference/protocol.md`, add the `commentary` row and extend the `player.id` row. Run `cargo test -p protocol`; the document test must pass.
10. **Command line.** In `content.rs`, load `Commentary` into `Loaded`. In `stream_run.rs`, add `commentary: &'a Commentary` to `Drive`, build one `Commentator` in `drive`, and attach its line in `match_event`. Pass `&loaded.commentary` in `bench.rs`, `record.rs`, and `serve.rs`. Add a unit test in `stream_run.rs`: drive a 3-minute match into a vector, and check that every `Event` message carries a non-empty `commentary`.
11. **Criterion tests.** Write `crates/engine/tests/commentary.rs` as described in `04-plan-commentary.yaml`, covering AC-1 to AC-4 and the four loader refusals. The AC-3 test reads the shipped file and checks the line against the lines of sets whose `when` holds the condition. It does not match keywords, so the English text can change freely. Run `cargo test --workspace`; all tests must pass.
12. **Measurement.** Repeat the step 1 `bench` command three times into `bench-baseline/commentary/after-<n>.*`. The median `bench.cpu_us_per_tick` must be at most 1.10 times the step 1 median. The median `bench.peak_mem_mb` must be at most 1.25 times the step 1 median. If either limit is exceeded, stop and report to the product owner. Do not remove commentary to meet the limit.
13. **Gates.** Search `crates/`, `content/`, `docs/`, `web/`, and `README.md` for workflow vocabulary (slice names, stage names, `.ai/`); the search must return nothing. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `node --test web/tests/*.test.mjs`. Commit with a product-language message that ends with the attribution line the session requires.

## Verification Strategy

No user-observable AC: the automated tests cover everything. Each of the four slice criteria is marked `observable: false` and runs under `cargo test --workspace`. The shape's user-observable commentary criterion, AC-22 (the feed updates within one frame of the goal tick), belongs to `viewer-match-day`. That slice renders the line.

Automated criterion classification:

| AC | Classification | Evidence |
|----|----------------|----------|
| AC-1 every kind names its player and club | build-capability | `commentary.rs`: a scripted event of each kind, plus every event of the seed-42 match |
| AC-2 five same-kind events in ten minutes give three or more distinct lines | build-capability | `commentary.rs`, looped over every kind |
| AC-3 a late equaliser and a rout read as such | build-capability | `commentary.rs`, scripted goals checked against the shipped file's sets |
| AC-4 no unfilled placeholder in a full seeded match | build-capability | `commentary.rs`, the seed-42 90-minute match with no brace and zero fallbacks |

No environment dependency lies on any criterion's path, so no `constraint-resolution:` line is needed.

## Test / Verification Plan

### Automated checks

- Format: `cargo fmt --all -- --check`.
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`.
- Unit and integration: `cargo test --workspace`. The baseline is 192 passing; the new tests add to that count.
- Page: `node --test web/tests/*.test.mjs` (the glob form). No page file changes; this check guards against regression.
- Criteria map: AC-1 to AC-4 are in `crates/engine/tests/commentary.rs`. The wire field is in `stream_run.rs`'s unit test and `crates/protocol/tests/document.rs`.
- Regression: the step 2 tick-file hash comparison, `determinism.rs`, `snapshot.rs`, and `crates/engine-cli/tests/resume.rs` show that the engine change moves no tick.
- Benchmark compare: step 12 against the step 1 baseline, judged per tick like the previous slice.

### Interactive verification (human-in-the-loop)

Automated only. Every criterion in this slice is a property of the generated text, and `cargo test` checks it. The match-day screen that displays the lines is verified in `viewer-match-day`.

## Risks / Watchouts

- R1 (medium): lines repeat and read as canned. The ten-minute no-repeat window, the seed-derived rotation start, and the specific sets reduce the risk; the file grows without code changes, and AC-2 measures the risk.
- R2 (medium): a goal or restart event carries no player, which breaks AC-1. Step 2 names the scorer and the taker. The stoppage-time invariant keeps the snapshot unchanged, and a unit test pins it.
- R3 (low): a Rust reader built before this change refuses the new field (`deny_unknown_fields`). Every Rust reader is in this workspace and changes in the same commit, and the page ignores unknown fields.
- R4 (low): event kinds from the tactics slice have no lines. The three-line rule fails the content test for any kind without lines, and step 7 covers every kind present at implement time.
- R5 (low): processor time per tick moves because `bench` drives the commentator. That is about 400 lines against about 280,000 ticks; steps 1 and 12 measure the change.
- R6 (low): a proposed contract redesign in `docs/design/realism/02-event-contract-redesign.md` (staged, not committed) also puts `commentary` on the event record. This plan agrees with that proposal and does not depend on it.

## Dependencies on Other Slices

- Consumes `match-rules`: the twelve play event kinds, the match clock's minute and added minute, the score on each event, and the taker picked at each dead ball.
- Touches files that `tactics-and-ai` may also change (`sim.rs`, `rules/mod.rs`, `stream_run.rs`), because that slice adds events and applies changes at stoppages. The two slices run in sequence, never in parallel. Whichever lands second rebases onto the other. Step 7 writes lines for every kind present at that time.
- `calibration` (`04-plan-calibration.md` step 8) makes `match_event` in `stream_run.rs` a shared crate function, so that `simulate` can also write event files. The recorded order puts `commentary` first. The slice that lands second threads the line through the shared mapping: `match_event` gains a `commentary: Option<String>` parameter, and every caller that writes events builds a `Commentator`. With that change, the files written by `simulate` carry lines too. The calibration plan's wire byte-identity check stays valid, because `crates/stream/tests/fixture.rs` records through `record.rs`, which carries the line both before and after the extraction.
- `viewer-match-day` (`04-plan-viewer-match-day.md` step 3) shows "the commentary text when a commentary field exists". The field this plan adds is `commentary`, which meets that plan's expectation as written.
- Consumers: `viewer-match-day` renders `commentary` in the match feed. Until then the page ignores the field. `integration` checks the lines in the feed end to end.

## Assumptions

Each entry is a decision this autonomous run made in place of the discovery interview. All are `class: implementation-detail`: none touches an open or carried intent risk (all six are adjudicated), narrows a product-owner answer, assigns control authority, changes the core loop, or drops a committed capability.

1. **Where the generator runs.** The generator is a per-match `Commentator` in the engine crate. The shared match driver calls it and puts its line on the event message. The alternative was to generate the line inside the simulation step and store it on `EngineEvent`. That would make `EngineEvent` lose `Copy`, put commentary state into the snapshot, and change the snapshot layout. The chosen design keeps the slice's "inside the engine, a field on the event record" and has the smallest blast radius.
2. **Field name and version.** The field is one optional key, `commentary`, in the key style of the existing optional fields. Protocol version 2 is kept under the recorded additive-field rule (`crates/protocol/src/lib.rs:21-33`). The slice definition decided the contract addition, and this entry settles only its spelling.
3. **Variant selection.** Selection is deterministic: a seed-derived rotation start plus a ten-minute no-repeat window per kind. It takes no engine random draw, because a draw would change every seeded match. The same seed gives the same lines.
4. **Coverage.** The coverage is every kind the rule set emits: twelve today, more if the tactics slice has landed. "About 30 kinds" in the slice text is an estimate. The criterion text is "any event kind in the rule set", and this plan meets it for every kind. Context variants (own goal, late winner, second yellow, and others) are template sets inside a kind.
5. **No line on tactics-change verdicts.** `tactics-change` events (queued or rejected) carry no commentary line. They acknowledge the manager's own request, carry no club or player, and appear as the state chips `steer.md` describes. An applied change is a match happening; the tactics slice gives it a line through the same file.
6. **Players on events.** The scorer is the last player who kicked the ball; a player of the other club makes it an own goal. The restart and kick-off events name the taker. This fills `player.id` with values the field already allowed and changes no tick.
7. **The last kicker stays out of the snapshot.** It is cleared at every stoppage announcement, which is the only moment a snapshot is written. A unit test pins the invariant. A later change that writes snapshots mid-play must add the field to the snapshot.
8. **The commentary file stays out of the content digest.** Editing a line does not change `content.hash` and does not make a snapshot refuse to resume. The file is loaded through the same fail-closed loader (NFR-7).
9. **Band thresholds are fractions of the match length.** Early is the first sixth, late is from eight ninths, and form uses the last sixth. The repeat window is ten minutes of play and the rout margin is 3 goals. A shortened test match scales the same way. Minute 88 of 90 is late. A 4-0 lead is a rout.
10. **Minute stamps.** Unconditional lines do not include `{minute}`, because the design brief gives the feed its own minute stamp (`02b-design.md:45`).
11. **Benchmark baseline at implement time.** The baseline is captured at the commit where implementation starts (step 1), into `bench-baseline/commentary/`. It is not captured now, and `05c-benchmark.md` is not re-authored. The recorded order puts `tactics-and-ai` first. Another session is capturing that slice's baseline into the same single-file artifact, so re-authoring the file here would overwrite its work, and a baseline taken now would be stale. The tripwires are unchanged: 10 percent processor time per tick and 25 percent peak memory.
12. **Instrument augmentation.** The only new signal is `commentary.fallback`. It is added in step 6, and step 11 asserts that the seed-42 match emits none. `04b-instrument.md` is not re-authored, for the same shared-file reason. The `experiment` augmentation stays deferred to `experiment-flags`.
13. **Consult.** The `appetite-medium-or-larger` consult trigger holds, because the workflow appetite is large. The product owner excluded `consult` at intake (`stack.excluded-by-po`), so no consult ran (`consult-runs: []`).
14. **No new entries in `po-answers.md`.** The autonomous run records these decisions here instead of writing them to `po-answers.md`, which another session is editing.
15. **Design contract.** `02c-craft.md` binds no step, because this slice changes no page file.

## Blockers

None.

## Freshness Research

- Source: rustc 1.92.0, scratch repro `fmt_repro.rs` (this session)
  Why it matters: it tests whether the standard library can fill named placeholders at runtime (ladder rung 1).
  Takeaway: `format!` refuses a non-literal template ("format argument must be a string literal"), so the fill is new code.
- Source: local registry `garde-0.23.0` (the version in `Cargo.lock`) and its in-repo use at `crates/engine/src/data/rules.rs:97`, `:178`
  Why it matters: the loader validation uses the same custom-validator signature `fn(&T, &Ctx) -> garde::Result`.
  Takeaway: reuse the idiom. No dependency change is needed.
- Source: `crates/protocol/src/lib.rs:21-33`
  Why it matters: it records the in-repo rule for when an added field needs a new protocol version.
  Takeaway: an added optional field with no change of meaning keeps version 2.
- No dependency is added or upgraded, so no advisory or deprecation applies.

## Recommended Next Stage

- **Option A (default):** `/wf implement football-manager-match-engine commentary`. The plan is complete. The recorded order puts `tactics-and-ai` first; when that slice lands first, step 7 covers its event kinds. Compact the session first so the SessionStart hook re-reads the artifacts.
- **Option B:** `/wf implement football-manager-match-engine tactics-and-ai`. This follows the recorded order when that plan is complete.
- **Option C:** `/wf slice football-manager-match-engine`. Not needed; the slice boundary held.
- **Option D:** `/wf shape football-manager-match-engine`. Not needed; the spec held.
