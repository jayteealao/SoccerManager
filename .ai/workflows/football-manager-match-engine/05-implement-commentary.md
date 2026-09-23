---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: commentary
status: complete
stage-number: 5
created-at: "2026-09-22T23:59:19Z"
updated-at: "2026-09-22T23:59:19Z"
metric-files-changed: 20
metric-lines-added: 2503
metric-lines-removed: 17
metric-deviations-from-plan: 8
metric-review-fixes-applied: 0
commit-sha: "a414df5fb4d4cfbf738305019be4271ed56149c6"
steering-honored:
  - "No visual change: no file under web/ changed; the lines ride on the event message, and the match feed that shows them belongs to the match-day screen."
  - "A card line names the card in words: the yellow, second-yellow, and red sets say booked, second yellow, sent off, or red card; the three unconditional card lines say 'shown a card' and are reached only when every card-specific line was used in the last ten minutes."
  - "Output boundary: the commit message, code comments, content/README.md, and docs/reference/protocol.md use product language; a scan of every added line for workflow vocabulary found none."
tags: [engine, commentary, text, content, protocol]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-commentary.md
  plan: 04-plan-commentary.md
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md]
  verify: 06-verify-commentary.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine commentary"
---

# Implement: Context-Aware Commentary

## The Implementation

The engine that this slice inherited emitted seventeen kinds of event and no text at all. A goal, a kick-off, and a restart named no player, and the engine kept player identifiers but no player names. The tactics slice had landed at `0e9cf6a`, so the commentator had to cover its injury, substitution, and AI-decision events as well as the twelve law events the plan counted.

Four decisions carry the build. First, the commentator is a per-match object in the engine crate that the shared match driver calls once per event; the engine loop, the snapshot, and the random draws do not change, and the seed-42 tick records are byte-identical before and after (the only differing bytes are the match timestamp in the 64-byte header). Second, the 168 English lines in 54 template sets live in `content/commentary/en.json`, and the loader refuses a file that leaves any of the 15 commented kinds with fewer than 3 unconditional lines, or that uses a placeholder the kind cannot fill. Third, selection is deterministic: most specific set first, a seed-derived rotation, and no line repeated for a kind within ten minutes of play. Fourth, the engine now names the last kicker on a goal and the taker on every restart and kick-off, and clears the last kicker at every dead ball, so the snapshot layout is unchanged.

All four criteria pass as `cargo test` properties: the workspace passes 251 tests with 0 failed and 3 ignored, the seed-42 match produces its lines with zero fallbacks, and the benchmark median moved from 1.4453 to 1.4352 µs per tick and from 6.14 to 6.30 MB peak memory, inside both limits. The match-day screen can now render the `commentary` field in its feed. The top open risk is repetition over a whole match: throw-in and goal-kick lines cycle many times over about 40 events, and the file is the place to add lines.

## Summary of Changes

- New `content/commentary/en.json` (schema 1): 54 template sets, 168 lines, for the 15 commented event kinds. Context sets cover the late equaliser, the late winner, the rout, the opener, the equaliser, the go-ahead goal, extending a lead, the consolation, the own goal, hot form, advantage, a booked offender, each card, offside and throw-in streaks, corner repeats, late corners and penalties when trailing, time-wasting goal kicks, early injuries, half-time substitutions, each AI manager choice, and the half-time and full-time score states.
- New module `crates/engine/src/commentary/`: `render.rs` (placeholder scan and fill), `templates.rs` (file schema, garde checks, loader), `context.rs` (minute band, score state, form, repeat, own goal), `mod.rs` (`Commentator`, `MatchNames`, selection, the `commentary.fallback` signal).
- The engine names a player on goal, kick-off, and restart events through the new `last_kicker` field. It also gains `EngineEventKind::ALL`, `EngineEventKind::code()`, `Simulation::player_names()`, `seed()`, and `minutes()`, and `Team::player_names`.
- `MatchEvent` gains the optional `commentary` field and builder. `MESSAGES` and `docs/reference/protocol.md` document it, and the `player.id` row names the scorer and the taker. Protocol version 2 is kept, with the reason written beside `PROTOCOL_VERSION`.
- `engine-cli` loads the commentary with the content. `stream_run::drive` builds one commentator per match and attaches its line to every play event, so `serve`, `record`, and `bench` all carry lines.
- `content/README.md` documents the file, its conditions, its placeholders, the loader's refusals, and the selection order.

## Files Changed

- `content/commentary/en.json`: new; the English lines.
- `content/README.md`: the files table gains the commentary file; a new section covers the set shape, the conditions, the placeholders, the refusals, and the selection order.
- `crates/engine/src/commentary/mod.rs`: new; the commentator and its selection rule.
- `crates/engine/src/commentary/templates.rs`: new; the file schema, the commented kinds, the guaranteed and fillable placeholders, the loader checks, and `Commentary::load` / `load_path`.
- `crates/engine/src/commentary/context.rs`: new; the situation functions and the commentator's history.
- `crates/engine/src/commentary/render.rs`: new; `placeholders` and `fill`.
- `crates/engine/src/lib.rs`: `pub mod commentary` and the `Commentary` and `Commentator` re-exports.
- `crates/engine/src/data/mod.rs`: `COMMENTARY_FILE`, outside `Content` and its digest.
- `crates/engine/src/sim.rs`: `EngineEventKind::ALL` and `code()`, `last_kicker`, `player_names()`, `seed()`, `minutes()`, the `EngineEvent::player` doc, and two unit tests.
- `crates/engine/src/rules/mod.rs`: `goal()` names the last kicker; `open_dead_ball()` names the taker and clears the last kicker; `place_kick_off()` picks the kicker before it pushes the event, names the kicker, and clears the last kicker.
- `crates/engine/src/team.rs`: `Team::player_names`, filled by `from_file`.
- `crates/engine/tests/commentary.rs`: new; the criteria tests and the loader refusals.
- `crates/protocol/src/event.rs`: the `commentary` field and builder; the round-trip test carries the key.
- `crates/protocol/src/lib.rs`: `commentary` in the event field list; the version-2 judgement.
- `docs/reference/protocol.md`: the `commentary` row and the extended `player.id` row.
- `crates/engine-cli/src/content.rs`: `Loaded::commentary`.
- `crates/engine-cli/src/stream_run.rs`: `Drive::commentary`, one `Commentator` per driven match, the line on every event, and a unit test.
- `crates/engine-cli/src/bench.rs`, `record.rs`, `serve.rs`: pass the loaded commentary to the driver.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs`, `crates/engine/src/rules/mod.rs`, `crates/engine/src/team.rs`: also changed by `match-rules` and `tactics-and-ai`. This slice started from `e79ad61`, after both, so no rebase was needed.
- `crates/engine-cli/src/stream_run.rs`: `calibration` plans to extract `match_event` into a shared function. The slice that lands second threads the line through the shared mapping, as the plan's dependency note records.
- `crates/protocol/src/event.rs`, `crates/protocol/src/lib.rs`, `docs/reference/protocol.md`: shared contract files.
- `content/README.md`: every content-bearing slice adds a section.

## Notes on Design Choices

- The commentator sits outside the engine loop. `EngineEvent` stays `Copy`, the snapshot holds no commentary state, and determinism is untouched. The line is still generated in the engine crate and travels as a field on the event record, as the slice definition asks.
- The loader's checks make AC-1's placeholder half and AC-4 structural: an accepted file cannot hold a line with an unknown or unfillable placeholder, and every kind has at least three lines that always fill. The runtime keeps a second guard: a set whose lines cannot all be filled for this event is not a candidate, and when no set is left, the built-in line and `commentary.fallback` make the gap visible.
- The most specific set wins, and sets with the same number of conditions are tried in file order. This is why the rout set sits before extends-lead, and why the two late goal sets carry `own_goal: false`, so an own goal never reads as a scorer's late winner.
- `{minute}` is left out of the shipped lines, because the feed shows its own minute stamp.
- The `decision` condition lets each AI manager choice read differently. Without it, the four choices would share generic lines.

## Verification Seams Built

- AC-1 (every kind names its player and club) → `EngineEventKind::ALL` at `crates/engine/src/sim.rs:196` and `COMMENTED` at `crates/engine/src/commentary/templates.rs:24` let the test enumerate kinds. `Commentator::new` with `MatchNames` (`crates/engine/src/commentary/mod.rs:69`, `:27`) accepts scripted events. The players are now on the events at `crates/engine/src/rules/mod.rs:110` (kick-off), `:130` (goal), and `:418` (restart). Enables `cargo test -p engine --test commentary` (`every_event_kind_gets_a_line_naming_its_player_and_club`, `every_shipped_line_names_the_player_and_the_club`, `a_full_seeded_match_fills_every_placeholder_and_names_every_player`).
- AC-2 (five same-kind events give three distinct lines) → the deterministic `rotation` at `crates/engine/src/commentary/mod.rs:236` and the public `Length` at `crates/engine/src/commentary/context.rs:64` let a test build a 90-minute context with no match. Enables `five_events_of_one_kind_in_ten_minutes_give_three_distinct_lines`.
- AC-3 (late equaliser, rout) → `Commentary::sets` is public (`crates/engine/src/commentary/templates.rs`), so the test compares the line against the lines of sets whose `when` holds the condition, not against keywords. Enables `a_late_equaliser_and_a_rout_read_as_such`.
- AC-4 (no unfilled placeholder in a full seeded match) → `Commentator::fallbacks()` at `crates/engine/src/commentary/mod.rs:87` and the `commentary.fallback` signal at `:165`. Enables the full-match test to assert zero fallbacks and no brace in any line.
- Loader refusals → `Commentary::load_path` at `crates/engine/src/commentary/templates.rs:325` loads a temp file under a shown name. Enables the four refusal tests.
- The line on the wire → `Commentator::for_match` in the driver at `crates/engine-cli/src/stream_run.rs:54` and the unit test at `:253`.
- The snapshot invariant → the unit test `the_last_kicker_is_clear_whenever_play_stops` at `crates/engine/src/sim.rs:825`.

## Visual Contract Honored (only if `02c-craft.md` was present)

`02c-craft.md` is present, but no step of this slice binds it: no file under `web/` changed, so no mock-fidelity item applies. No contract-check pass was dispatched, because there is no built surface to check.

## Deviations from Plan

1. **Event kinds.** The plan counted twelve play kinds. At `e79ad61` the engine has seventeen, because the tactics slice added injury, substitution, AI decision, and the two change verdicts. The file covers the 15 commented kinds. The two verdicts get no line, as plan assumption 5 decided. `EngineEventKind::ALL` has 17 entries, and `code()` spells both verdicts as `tactics-change`.
2. **AI-decision names a club, not a player.** The engine's `ai-decision` event carries no player (`crates/engine/src/ai.rs:338`). Its lines name the club, and the player that a substitution choice involves is named on the `substitution` event that follows. The plan's step 2 test ("every event except half-time and full time has a player") therefore also excludes `ai-decision` and the two verdicts.
3. **Tick-file comparison.** The whole-file SHA-256 of the seed-42 `simulate` output differs before and after (`fcd0c000…` against `1034a7fe…`), and it also differs between two runs of the same binary (`1034a7fe…` against `c49364ea…`). `cmp -l` shows three differing bytes, at offsets 45 to 47, inside the header's match timestamp (`crates/engine/src/record.rs:36`). The SHA-256 of the records after the 64-byte header is equal before and after (`05250c2d…`), so no tick moved. The files are in `bench-baseline/commentary/`.
4. **`Commentator::line` returns `Option<String>`.** It returns `None` for the two verdict kinds instead of a line, so the driver attaches nothing to `tactics-change`.
5. **An added condition, `decision`.** It was not in the plan's condition list. It keys sets on the AI manager's choice (the four `ai.decision` codes).
6. **Line volume.** The file has 168 lines in 54 sets, not about 110. The extra sets cover the five kinds added by the tactics slice.
7. **`Commentary::load_path`** was added beside `load` so that the refusal tests can load a modified copy under a shown name. No other loader changed.
8. **The benchmark metric the plan names does not time the commentator.** `bench.cpu_us_per_tick` and `bench.peak_mem_mb` come from `run_one` (`crates/engine-cli/src/bench.rs:97`), a bare simulation. The commentator runs only on the stream path (`measure_stream`), which `bench --json` does not run by default. The step 12 comparison was run as planned and passed. It shows that the engine change adds no cost to the simulation. It does not measure the commentator.

## Anything Deferred

- Languages other than English: out of scope by the slice definition. The file carries `language`, and the loader reads one fixed path.
- Rendering the line in the feed: `viewer-match-day`.
- Commentary for events written by `simulate` and `resume`: those commands route no events today. When `calibration` makes the event mapping shared, the slice that lands second threads the commentator through it.
- No `sdlc-debt` shortcut was taken. The history scans are linear in the number of earlier events (about 400 per match); see Known Risks.

## Known Risks / Caveats

- Repetition over a whole match: a match has about 40 throw-ins and 4 unconditional throw-in lines plus a 3-line streak set, so lines recur after ten minutes. The file grows without code changes.
- The commentator scans its history for every event (repeat count, form window, kind count). The cost is quadratic in the event count, about 400 events per match. This cost is small next to the tick work.
- An event could name a slot whose player changed earlier on the same tick. The commentator renames a roster slot only when it processes the substitution event, so it follows event order. This is the same rule the driver's identifier mapping uses.
- `last_kicker` stays out of the snapshot because a snapshot is written only when play stops. A later change that writes snapshots during open play must add the field to the snapshot.
- The three unconditional card lines say "shown a card" without naming the colour. They are reached only when every line of the matching card set was used in the last ten minutes, for example a seventh yellow card inside ten minutes.

## Freshness Research

- Source: the in-repo garde idiom at `crates/engine/src/data/rules.rs:92` and `:122` (garde 0.23 per `Cargo.lock`).
  Why it matters: the loader's whole-file check uses `#[garde(custom(fn))]` with the signature `fn(&T, &()) -> garde::Result`.
  Takeaway: reused as is; `cargo build` and the refusal tests confirm the signature. No dependency was added.
- Source: `crates/protocol/src/lib.rs` `PROTOCOL_VERSION` comment.
  Why it matters: it records when an added field needs a new protocol version.
  Takeaway: an optional field with no change of meaning keeps version 2. The judgement is written beside the constant.
- Source: rustc 1.92.0 (the plan's scratch repro, `format!` needs a literal format string).
  Takeaway: the placeholder fill is a hand-written scan in `render.rs`.

## Evidence

- Baseline, commit `e79ad61` (`bench-baseline/commentary/commit.txt`, measured 2026-09-22T23:43:47Z): three runs of `engine-cli bench --seed 42 --matches 5 --json`, all exit 0. `bench.cpu_us_per_tick` was 1.456, 1.4245, and 1.4453 (median 1.4453). `bench.peak_mem_mb` was 6.1406, 6.1445, and 6.1680 (median 6.1445).
- After the change: three runs, all exit 0 (`after-1..3.*`). `bench.cpu_us_per_tick` was 1.4245, 1.4352, and 1.4453 (median 1.4352, ratio 0.993 against the 1.10 limit). `bench.peak_mem_mb` was 6.3008, 6.3047, and 6.3242 (median 6.3047, ratio 1.026 against the 1.25 limit). `budget.pass` was true in every run.
- Tick records: `ticks-body.sha256` holds equal hashes for the records after the header, before and after the change.
- `cargo test --workspace`: 38 result lines, 251 passed, 0 failed, 3 ignored. The engine's `commentary` test file passes 9 tests, the engine unit tests pass 117, and the new `stream_run` test passes.
- `cargo fmt --all -- --check`: exit 0. `cargo clippy --workspace --all-targets -- -D warnings`: finished with no warning. `node --test web/tests/*.test.mjs`: 47 passed, 0 failed.
- The scan of added lines in `crates/`, `content/`, `docs/reference/`, and `README.md` for workflow vocabulary returned one match only: `extend_from_slice`, a Rust standard-library call.

## Assumptions and Decisions

Each entry was settled by this autonomous run. Each is stamped `class: implementation-detail`, because none touches an open or carried intent risk (all six are adjudicated), narrows a product-owner answer, assigns control authority, changes the core loop, or drops a committed capability.

1. `class: implementation-detail`. The two change verdicts (`ChangeApplied`, `ChangeRejected`) get no line, as plan assumption 5 decided. AC-1's "any event kind in the rule set" is met for the 15 commented kinds, which are every kind a match produces except the verdicts on the manager's own requests. Classification of AC-1: build-capability.
2. `class: implementation-detail`. An `ai-decision` line names the club only. The event carries no player in the engine contract, and the player that a substitution choice involves is named on the `substitution` event. No engine or contract change was made to add a player to the AI event.
3. `class: implementation-detail`. The `decision` condition was added so that the four AI choices read differently. It is additive and optional.
4. `class: implementation-detail`. The tick-file identity check compares the records after the 64-byte header, because the header stamps the match time on every run.
5. `class: implementation-detail`. `{minute}` fills with the clock minute plus one in regulation time (the first minute reads 1), and with the end-of-half minute in added time, with `{added_minutes}` carrying the added minute. No shipped line uses either placeholder.
6. `class: implementation-detail`. At half-time and full time, the score conditions read from the home club's side, and the lines name both clubs.
7. `class: implementation-detail`. The unconditional card lines say "shown a card", so that they read correctly for any card. The card-specific sets (6 yellow, 4 second-yellow, 4 red lines) always hold on their card and win first.
8. `class: implementation-detail`. Commit scope: code, content, and docs in one commit (`a414df5`), with a product-language message. Workflow artifacts go in a separate commit that holds only this slice's own files. `00-index.md`, `03-slice.md`, `04-plan-commentary.md`, and the global index carry uncommitted changes from other sessions, so they are updated on disk and left out of both commits. The files that were already staged (`docs/design/realism/*`, `crates/engine/tests/zz_stall_probe.rs`) were left out through a path-limited commit.
9. `class: implementation-detail`. `05c-benchmark.md` and `04b-instrument.md` were not re-authored (plan assumptions 11 and 12). The baseline and the comparison are in `bench-baseline/commentary/`. The one new signal is `commentary.fallback`.

## Recommended Next Stage

- **Option A (default):** `/wf verify football-manager-match-engine commentary`. Every criterion is a `cargo test` property, so verification re-runs the suite, the tick-body comparison, and the benchmark comparison. Consider compacting the session before `/wf verify`; workflow state lives in artifact files on disk, and the SessionStart hook re-reads it after compaction.
- **Option B:** `/wf review football-manager-match-engine commentary`. This skips verification. It is not recommended, because the slice changes testable engine behaviour (players on events).
- **Option C:** `/wf plan football-manager-match-engine commentary`. Not needed. The plan held, apart from the recorded deviations.
