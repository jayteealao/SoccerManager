---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: extra-time-penalties
status: complete
stage-number: 5
created-at: "2026-09-23T16:44:00Z"
updated-at: "2026-09-23T16:44:00Z"
metric-files-changed: 43
metric-lines-added: 2364
metric-lines-removed: 140
metric-deviations-from-plan: 8
metric-review-fixes-applied: 0
commit-sha: "90d4ee13abf8bb6206cd16350f20e6d9f36591f2"
commits:
  - "90d4ee13abf8bb6206cd16350f20e6d9f36591f2"
steering-honored:
  - "Product-owner answer extra-time-penalties Q-1 = B: every shoot-out kick is played on the pitch, tick by tick, and the announced maximum carries a 10-round allowance (content/rules/default.json shootout.allowance_rounds 10; crates/engine/src/sim.rs MatchConfig::max_ticks)."
  - "Dark-path counter definition: the computer manager's queuing is unchanged; a change still queued when a knockout match ends counts in change.expired_at_full_time, and nothing applies during the shoot-out."
  - "Image gate, tokens, layout, no spinner: the only page change is the scrubber's end (web/history.mjs scrubLimit, web/main.mjs), with no style, token, or layout change."
  - "Output boundary: the commit message, code comments, docs, and test titles use product language; the deliberate-shortcut marker is written as 'Known limit:' in code."
tags: [engine, rules, extra-time, shoot-out, fatigue, snapshot, protocol, knockout]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-extra-time-penalties.md
  plan: 04-plan-extra-time-penalties.md
  siblings: [05-implement-engine-core.md, 05-implement-data-schemas-generator.md, 05-implement-stream-protocol.md, 05-implement-viewer-pitch.md, 05-implement-match-rules.md, 05-implement-tactics-and-ai.md, 05-implement-commentary.md, 05-implement-calibration.md, 05-implement-viewer-match-day.md, 05-implement-viewer-lineup-tactics.md, 05-implement-viewer-reports-recovery.md, 05-implement-integration.md]
  verify: 06-verify-extra-time-penalties.md
next-command: wf-verify
next-invocation: "/wf verify football-manager-match-engine extra-time-penalties"
---

# Implement: Extra Time and Penalty Shoot-outs

## The Implementation

The engine inherited a clock that knew two halves, a referee that ended every match at the end of the second half, and a plan with 19 steps built on the product owner's option B: each shoot-out kick is played on the pitch. The build followed the plan from the rule pack outwards. The rule pack is schema 4 with `extra_time` and `shootout` blocks. The clock counts periods, so minutes run on to 120. A `knockout` switch on `MatchConfig` (off by default, `--knockout` on four commands) sends a level match into two 15-minute periods and then into a shoot-out whose laws are pure functions in `rules/shootout.rs`. The extra-time breaks give no energy back, so fatigue has no step at minute 90. Each team gains one substitution in one window once extra time starts. Snapshot format 4 stores the switch, extra time, and the shoot-out, and resumes tick for tick from the break before extra time and from a kick in the middle of a shoot-out.

The first live runs showed two problems the plan had not predicted. Every knockout run failed the tick validator: 61,327 anchor violations on seed 6, all after the first kick, because the players wait in the centre circle. The validator now exempts the shoot-out from its anchor rule. Kicks scored only 24.9 percent of the time over 80 shoot-outs, because the open-play shot noise and the keeper's catch chance treat a penalty like a long-range shot. The keeper now commits to a dive when the ball is struck. Two hand-set constants halve the aim noise and the catch chance for a shoot-out kick. Conversion is now 67.7 percent over the same 80 shoot-outs (563 of 832 kicks), close to the real-world rate. Eight deviations are recorded below. The default match does not change: the seed-42 tick records and JSON Lines dump are byte-identical to the baseline taken before any edit. The per-match benchmark gate reads 422.0 ms against 460.7 ms.

The next stage is verify. The top open risk is the realism of the shoot-out: its three constants are hand-set, and no calibration band checks them yet.

## Summary of Changes

- Rule pack schema 4 with `extra_time` (periods, period minutes, added-time cap, extra substitution, extra window) and `shootout` (kicks, allowance rounds). A schema 3 pack is refused by its version.
- Period clock: `MatchClock::new(minutes, rules, knockout)`, per-period lengths, four added-time slots, `in_extra_time()`, `last_period()`, `end_minute()`, and `knockout_extra_ticks` / `shootout_allowance_ticks`.
- Knockout switch `MatchConfig::knockout` + `with_knockout()`. `max_ticks()` is unchanged for a regular match (360,000) and gives 530,000 for a 90-minute knockout match.
- Referee: level after regulation → extra-time break (`half_time(recover: false)`), seeded toss for the first extra-time kick-off, other team kicks off the second period, then the shoot-out. `decided_by` is set at full time of a knockout match.
- Shoot-out on the pitch: the kick is set up as a penalty dead ball with shoot-out targets and readiness, the taker always shoots through the extracted `shot_kick`, only the defending keeper can gain the ball, and the kick ends on a goal, a save, the ball leaving play, the ball stopping, or 5 s of live ball. It stops after 100 rounds as an abandoned match.
- Substitution limits in extra time (`substitution_limits()`), read by the ledger check and by the computer manager. The change queue applies nothing during the shoot-out.
- Events: `period`, `shootout_round`, `shootout_scored`, `shootout_scores`, and `decided_by` on `EngineEvent` and the protocol event (optional). Match statistics: `extra_time.played`, `extra_time.added_s`, `shootout.scores`, `shootout.kicks`, and `result.decided_by` (knockout only).
- Commentary: shoot-out kicks get no line.
- Snapshot format 4. `Snapshot::knockout()`, and `resume` / `serve --resume` read the switch from the snapshot.
- The stream drive loop plays a sudden death past the tick cap, and the page scrubber follows the newest tick.
- A tick file whose sudden death runs past the announced maximum gets its header raised to the ticks written, so it still reads.
- Docs: protocol reference, CLI reference, data-file reference. Additive properties in both observability schemas.

## Files Changed

- `content/rules/default.json`: schema 4 with the extra-time and shoot-out blocks (IFAB values).
- `crates/engine/src/data/rules.rs`: `ExtraTime`, `Shootout`, `RULES_VERSION = 4`.
- `crates/engine/src/data/mod.rs`: re-exports; loader unit tests moved to version 4.
- `crates/engine/src/rules/clock.rs`: period clock, extra-time allowance, knockout and shoot-out tick counts, and unit tests for minutes 95, 105, 119, and 120+1.
- `crates/engine/src/rules/shootout.rs` (new): eligibility equalising, keeper, order, alternation, decision, and winner, with six unit tests.
- `crates/engine/src/rules/restart.rs`: `shootout_target`, `shootout_ready`.
- `crates/engine/src/rules/mod.rs`: `Shootout` state on the referee, extra-time branch in `check_clock`, `half_time(recover)`, and the shoot-out driver (start, next kick, take, save, ball, expiry, outcome).
- `crates/engine/src/decision.rs`: `shot_kick()` extracted from `decide_carrier()`, with the same draw order.
- `crates/engine/src/sim.rs`: knockout switch, `max_ticks()`, `DecidedBy`, event and summary fields, the shoot-out routing in `step`/`move_ball`/`resolve_possession`, frozen minute during the shoot-out, and the forced-kick seam.
- `crates/engine/src/tactics/change.rs`: `substitution_limits()`, the shoot-out freeze in `apply_changes`.
- `crates/engine/src/ai.rs`: the computer manager reads the extra-time limits.
- `crates/engine/src/fatigue.rs`: no injury roll during the shoot-out.
- `crates/engine/src/snapshot.rs`: format 4.
- `crates/engine/src/observe/mod.rs`: additive knockout statistics.
- `crates/engine/src/commentary/mod.rs`, `context.rs`: no line for shoot-out kicks; test literals.
- `crates/engine/src/validate.rs`: anchor rule exempt from the first shoot-out kick on.
- `crates/engine/src/record.rs`: `FileSink::finish` raises the header maximum when more ticks were written; test adapted.
- `crates/engine/src/scenario.rs`: `knockout()`, `period(n)`, `shootout_kicks()`.
- `crates/engine/src/lib.rs`: re-export `DecidedBy`.
- `crates/engine-cli/src/cli.rs`, `simulate.rs`, `bench.rs`, `serve.rs`, `record.rs`, `resume.rs`: `--knockout`, and the switch read from the snapshot.
- `crates/engine-cli/src/stream_run.rs`: loop continues during the shoot-out; event fields mapped; new driver test.
- `crates/protocol/src/event.rs`, `lib.rs`: optional fields, builder methods, the event field list, and the version note.
- `web/history.mjs`, `web/main.mjs`, `web/tests/history.test.mjs`: `scrubLimit` and its test.
- `crates/engine/tests/rules_extra_time.rs` (new, 6 tests), `rules_shootout.rs` (new, 8 tests), `fatigue.rs` (+1), `snapshot.rs` (+3), `content.rs` (+1, version pins), `commentary.rs` (+1, literal fields).
- `docs/reference/protocol.md`, `cli.md`, `data-files.md`; `schemas/observability/match-event.schema.json`, `match-stats.schema.json`.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/rules/mod.rs`, `sim.rs`, `snapshot.rs`, `rules/restart.rs` (match-rules), `tactics/change.rs`, `ai.rs`, `fatigue.rs` (tactics-and-ai), `commentary/mod.rs` (commentary), `observe/mod.rs` (calibration), `stream_run.rs` (stream-protocol, viewer slices), `web/history.mjs`, `web/main.mjs` (viewer-pitch, viewer-match-day), `crates/protocol/src/*` (stream-protocol, viewer-lineup-tactics), and the three reference docs (integration).

## Notes on Design Choices

- Class `implementation-detail` for every entry below (`_decision-classes.md`); none resolves an open or carried intent risk, alters a PO answer, assigns control authority, changes the core loop, or drops a committed capability.
- `MatchClock` keeps its field names; `half` now counts every period, so snapshot, scene, and `Simulation::half()` callers need no rename.
- `period` is set only on the extra-time breaks and their kick-offs, so a regular match's events are unchanged.
- The teams change ends at every extra-time break; the toss decides only the first extra-time kick-off.
- `equalise` never drops the goalkeeper; the larger side drops its weakest outfield kickers.
- During the shoot-out the clock is frozen for events: every shoot-out event shows the last minute of play (120 after extra time).
- No injury roll during the shoot-out, because an injury in open play would open a dropped ball in the middle of a kick.
- The computer manager reads the raised limits, so it can use the extra substitution; with regulation limits nothing changes for a regular match.
- The work was built in the order of the plan, with the default match checked for byte identity after each engine change.

## Verification Seams Built

- AC-1 (level knockout → two extra-time periods → shoot-out → one winner) → `Scene::knockout()` at `crates/engine/src/scenario.rs:178`, `Scene::period(n)` at `:191`, `Scene::shootout_kicks(&[bool])` at `:222` (forced outcomes at the router, `forced_kicks` at `crates/engine/src/sim.rs:466`), `MatchConfig::with_knockout()` at `crates/engine/src/sim.rs:132`, `Simulation::in_shootout()` at `:596` (enables `cargo test --test rules_extra_time --test rules_shootout` to drive early finish, sudden death, order restart, equalising, and the queue freeze end to end).
- AC-2 (extra-time substitution allowance available and enforced) → `Simulation::substitution_limits()` at `crates/engine/src/tactics/change.rs:258`, driven by the existing `Scene::subs_used(team, used, windows)` at `crates/engine/src/scenario.rs:228` (enables `rules_extra_time::extra_time_adds_one_substitution_in_one_window`).
- AC-3 (fatigue at minute 105 on the extended curve, no step at 90) → `half_time(recover: false)` at `crates/engine/src/rules/mod.rs:642` with the existing `Scene::energy()`; `fatigue_runs_on_through_extra_time_without_a_step_at_ninety_minutes` in `crates/engine/tests/fatigue.rs` records energy every tick.
- Resume in extra time and in the shoot-out → `Snapshot::knockout()` at `crates/engine/src/snapshot.rs:104` (enables the two continuation tests and the regular-match refusal in `crates/engine/tests/snapshot.rs`).

## Deviations from Plan

1. `shot_kick(c)` became `shot_kick(c, goal, keeper, spread_scale)` (`crates/engine/src/decision.rs:320`). All shoot-out kicks go at one goal, which is not always the kicker's attacking goal, so the goal and the defending keeper are parameters. `spread_scale` is 1.0 in open play, and multiplying by 1.0 is exact, so the default match is byte-identical. The seed-42 comparison shows it.
2. The shoot-out keeper commits to a seeded dive (45 / 10 / 45 percent left, middle, right). Two hand-set constants (`SHOOTOUT_SPREAD`, `SHOOTOUT_HOLD`, `crates/engine/src/rules/mod.rs:961`) halve the aim noise and the catch chance. Evidence: conversion was 213 of 857 kicks (24.9 percent) over 80 shoot-outs before, and is 563 of 832 (67.7 percent) after, on seeds 100 to 180 with `--minutes 1 --knockout`. The plan did not specify keeper behaviour.
3. The validator exempts the anchor rule from the first shoot-out kick on (`crates/engine/src/validate.rs`). Evidence: seed 6 `--knockout` gave 61,327 `anchor_tolerance` violations, the first at tick 375316 after `rules.shootout_start` at tick 375200. After the change, 0 violations on seeds 2, 6, 10, and 12 and on 80 short shoot-outs.
4. `FileSink::finish` raises the header's announced maximum when a sudden death runs past it (`crates/engine/src/record.rs:279`). Without this, `read_ticks` refuses the file (`record count … exceeds`). The existing unit test was adapted: the reader still refuses a file whose header announces fewer ticks than it holds.
5. `Summary` keeps a kick count (`shootout_kicks`), not a kick list, because `Summary` is `Copy`. The kicks are in the events and in the shoot-out state.
6. The plan's `substitutions_used()` scene method was not added. The existing `Scene::subs_used()` covers it.
7. `crates/protocol/src/message.rs` is unchanged. Every new field rides on the event message, and no other message needed a field.
8. An extra signal, `rules.shootout_start`, marks the start of the shoot-out. It is in addition to `rules.extra_time`, `rules.shootout_kick`, `rules.shootout_result`, and `rules.shootout_round_limit`.

## Anything Deferred

- Shoot-out lines in the commentary (the commentary slice owns the text). Shoot-out kicks get no line today.
- A calibration band for shoot-out conversion. The three constants move to the tuning file when one exists (see Known Risks).
- Replacement of an injured goalkeeper during the shoot-out (Law 10). No injury roll runs during the shoot-out, so the case cannot arise today.
- Page display of the shoot-out beyond the event feed: out of scope per the slice.

## Known Risks / Caveats

- Hand-set shoot-out constants (`KEEPER_DIVE_M`, `KEEPER_STAYS`, `SHOOTOUT_SPREAD`, `SHOOTOUT_HOLD` in `crates/engine/src/rules/mod.rs`). They are marked "Known limit" at the site. Ceiling: realism is checked only by this run's 80-shoot-out sample. Upgrade path: move them to `content/tuning.json` with a band in `content/realism-bands.json`.
- A knockout match's `ticks_expected` can be exceeded by a sudden death longer than the allowance, which the product owner accepted with option B. The page, the stream loop, and the tick file handle it.
- IFAB Laws 3, 7, and 10 were applied from recall (plan R7). Verify re-reads them.
- The minute during the shoot-out is frozen at the last minute of play. A consumer that sorts by minute alone sees every kick at 120.

## Freshness Research

- Source: the installed code only. No dependency was added or upgraded. The `Kick` pattern, `garde` bounds, and `serde` attributes follow patterns already in the crate.
- Baseline before any edit (this run): `cargo test --workspace --all-features --release` passed with 0 failures, and seed-42 `simulate` gave the tick-file SHA-256 prefix `25a89182…` and the JSONL prefix `8120abc5…`. After the change, the JSONL hash is identical, and the tick file differs only in bytes 45 to 47, which fall in the wall-clock match-stamp field of the header.

## Gates Run (this run)

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass, with no warning.
- `cargo test --workspace --all-features` (debug): 340 passed, 0 failed, 4 ignored. The period-scene test was added after that run, and `cargo test -p engine --all-features --test rules_extra_time` passes 6 of 6.
- `node --test web/tests/*.test.mjs`: 127 pass, 0 fail.
- Determinism: the seed-42 records are byte-identical to the pre-edit baseline, and `determinism.rs` passes.
- `engine-cli bench --seed 42 --matches 5 --json` ×3: 422.0 / 421.8 / 422.0 ms per match (gate 460.7), 1.4926 to 1.4933 µs per tick, 282,600 ticks per match, and peak memory 6.58 / 6.54 / 6.54 MB (gate 6.82). One `--knockout` run for information: 412.4 ms and 6.68 MB. Seed 42 is not level, so the match is the same.
- `cargo bench -p engine --bench tick_step`: `tick_step` 1.4898 µs, "No change in performance detected" (p = 0.71); `steering_pass_22` 896.09 ns, no change.

## Assumptions

Each entry is an autonomous decision of this run, with the class from `_decision-classes.md`.

- **I-1** The deviations 1 to 8 above are settled in scope. Each one keeps the committed capability (option B, the on-pitch kick) and changes only internals, tests, or docs. `class: implementation-detail`.
- **I-2** The shoot-out realism constants target a conversion near the real-world rate of about 70 percent. They do not touch a charter item or an adjudicated intent risk, and calibration owns the realism bands. `class: implementation-detail`.
- **I-3** The deliberate-shortcut marker is written as `// Known limit:` in code instead of the workflow's debt tag, to honour the external output boundary. The debt is recorded here under Known Risks. `class: implementation-detail`.
- **I-4** Two commits: the code commit `90d4ee1`, then a separate commit for the build record. This follows the pattern of earlier slices, so the record can carry the code commit's SHA. `class: implementation-detail`.
- **I-5** The roster entry for this slice in `03-slice.md` and `00-index.md` is set to `complete`, per the implement reference's roster rule. `current-stage`, `selected-slice`, and `next-command` in `00-index.md` are left alone, because another session is driving the integration slice's review. `class: implementation-detail`.
- **I-6** Paths staged by other sessions (`docs/design/realism/*`, `crates/engine/tests/zz_stall_probe.rs`) and the other dirty workflow files were left out of every commit. The code commit used explicit paths. `class: implementation-detail`.
- **I-7** AC classification: AC-1, AC-2, and AC-3 are all `build-capability`, verified by automated `cargo test` (`observable: false` in the slice). `class: implementation-detail`.

## Recommended Next Stage

- **Option A (default): Verify** → `/wf verify football-manager-match-engine extra-time-penalties`. All three criteria are automated `cargo test` checks, and verify should also re-read IFAB Laws 3, 7, and 10 (plan R7). Consider compacting the session before verify. Workflow state lives in the artifact files on disk, and the SessionStart hook re-reads it after compaction.
- **Option B: Skip to Review** → `/wf review football-manager-match-engine extra-time-penalties`. Not recommended, because the slice changes testable engine behaviour.
