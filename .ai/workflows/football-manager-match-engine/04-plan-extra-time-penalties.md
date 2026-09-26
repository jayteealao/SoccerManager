---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: extra-time-penalties
status: complete
stage-number: 4
created-at: "2026-09-22T22:23:31Z"
updated-at: "2026-09-23T08:44:13Z"
metric-files-to-touch: 33
metric-step-count: 19
has-blockers: false
revision-count: 2
revisions:
  - rev: 1
    at: "2026-09-23T07:03:59Z"
    trigger: manual
    because: "auto-review — 9 issues found"
    changed: "tactics slice has landed: steps grounded on fatigue.rs and tactics/change.rs; rule pack schema 4 and snapshot version 4; break before extra time gives no energy back; Q-1 still open"
  - rev: 2
    at: "2026-09-23T08:44:13Z"
    trigger: manual
    because: "auto-review — 11 issues found"
    changed: "Q-1 answered (option B, kicks on the pitch, 10-round allowance); step 6 becomes the on-pitch kick; allowance in MatchConfig::max_ticks; shoot-out state in snapshot 4; stream drive loop, commentary, change queue, and page scrubber adjusted; drifted line citations corrected; status complete"
consult-runs: []
tags: [engine, rules, extra-time, shoot-out, fatigue, deferred]
stack-source: confirmed
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-extra-time-penalties.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md, 04-plan-commentary.md, 04-plan-viewer-lineup-tactics.md]
  steer: steer.md
  implement: 05-implement-extra-time-penalties.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine extra-time-penalties"
---

# Plan: Extra Time and Penalty Shoot-outs

## The Plan

The referee runs two halves, added time, and a penalty restart played on the pitch. Energy drains per tick from speed and stamina, with no input from the clock (`crates/engine/src/fatigue.rs:78`). Substitutions go through one ledger per team that reads the limit and the window count from the rule pack (`crates/engine/src/tactics/change.rs:381-391`). `MatchClock` (`crates/engine/src/rules/clock.rs:71`) still knows only halves, and `check_clock` (`crates/engine/src/rules/mod.rs:499`) ends the match at the end of the last half. The product owner has now answered Q-1 with option B: each shoot-out kick is played on the pitch, tick by tick, through the penalty restart and the shot mechanics. The announced maximum includes a 10-round allowance, and the page grows its history if sudden death runs past it.

This auto-review found 11 issues. Most come from option B. Every shoot-out kick opens a stoppage, and the snapshot sink captures a snapshot at every stoppage (`crates/engine/src/snapshot.rs:279`). The shoot-out state must therefore go into snapshot version 4. The change queue applies changes at stoppages (`tactics/change.rs:256`), so it must apply nothing during the shoot-out. The stream drive loop stops at the announced tick count (`crates/engine-cli/src/stream_run.rs:53`), so it would cut off a long sudden death. The penalty commentary says "Penalty to {team}!" (`content/commentary/en.json:450`), which is wrong for a shoot-out kick. At a penalty the taker decides on the next tick and can pass (`crates/engine/src/rules/mod.rs:471-497`, `decision.rs:240`), so the shoot-out taker must always shoot. The plan extracts the existing shot code into one function for that. The page already grows its history past the announced length (`web/history.mjs:74`), and only the scrubber maximum needs to follow it. The four earlier decisions stand: periods in the clock, a knockout switch that is off by default, shoot-out laws as pure functions, and optional fields on existing events.

With 10 rounds of at most 2,500 ticks per kick, a knockout match of 90 minutes announces 530,000 ticks, against 360,000 for the default match. The default match does not change. The top open risk is a kick that never ends. The plan closes it with three guards: only the defending keeper can gain the ball, a 5-second live cap ends each kick, and a 100-round safety cap ends a runaway shoot-out.

## Current State

- `MatchClock { halves, half, half_start, half_ticks, plays_added, added_ticks: [Option<u32>; 2] }` (`clock.rs:71-82`) supports two halves only. `minute()` (`clock.rs:120`) multiplies `half * half_ticks`, which is wrong for periods of different lengths.
- `clock::max_ticks(minutes, rules)` (`clock.rs:55`) returns regulation plus `halves * added_time.max_s`, which is 360,000 ticks for 90 minutes. Every caller outside `clock.rs` reads it through `MatchConfig::max_ticks()` (`sim.rs:113`), which has both the rule pack and the tuning. A search in this review found 12 calls of `MatchConfig::max_ticks()` in `crates/`.
- `check_clock` (`rules/mod.rs:499-537`) fixes added time on the first tick after regulation, then either runs `half_time()` or sets `Phase::FullTime`. `half_time()` (`rules/mod.rs:541-570`) pushes a `HalfTime` event, calls `half_time_recovery()` (`:555`), opens a `HalfTime` stoppage, switches ends, and places the kick-off for team 1 (`place_kick_off(1)`).
- Penalty restart: `open_dead_ball` (`rules/mod.rs:376-437`) places the ball, names the taker, applies the penalty delay of 15 s (`content/tuning.json:53`, 750 ticks), scales the delay by time wasting for a leading team, emits a `penalty` event, and opens a stoppage. `DeadBall::hard_limit()` (`restart.rs:50`) forces the take at 3 times the delay, 2,250 ticks. `restart::target` (`restart.rs:235-243`) puts the defending keeper on the line and everyone else outside the area. `take_restart` (`rules/mod.rs:471`) gives the taker the ball, and for a penalty the taker decides on the next tick. The decision can pick a pass, a dribble, or a hold as well as a shot (`decision.rs:240-332`).
- Shot and outcome: `apply_kick` (`sim.rs:659`) records the shot. A goal goes through `goal()` (`rules/mod.rs:128`), which adds to `summary.goals` and sets up a kick-off. `gain()` (`sim.rs:802`) gives the ball to the first player in reach, and the keeper can fail the catch (`sim.rs:753-760`). A ball out of play opens the next restart.
- `RulePack` is at schema 3 (`data/rules.rs:13`), with `substitutions.windows_exempt: ["half_time"]` in the shipped pack. It has no extra-time or shoot-out fields.
- Fatigue (`fatigue.rs`): `drain()` (`:78`) depends on speed, stamina, and `dt` only. `multiplier()` (`:46`) maps energy to a factor. `half_time_recovery()` (`:127`) adds energy at half-time. No function reads the minute.
- Substitutions (`tactics/change.rs`): `SubLedger` (`:155`). `apply_changes()` (`:256`) runs at every stoppage the rule pack admits. `substitute()` (`:372`) rejects with `LimitReached` (`:384`) or `NoWindowLeft` (`:388`). The snapshot stores both ledgers (`snapshot.rs:602-605`). The computer manager runs in `ai_tick()` (`ai.rs:175`), called from the tick loop (`sim.rs:604-607`).
- Snapshots: `SnapshotSink::on_stoppage` captures one at every stoppage (`snapshot.rs:279-280`). `VERSION = 3` (`snapshot.rs:41`).
- Stream: the drive loop runs `while !sim.is_over() && written < opts.ticks` (`stream_run.rs:53`), where `ticks` is `MatchConfig::max_ticks()`. `simulate` and `bench` run `Simulation::run`, which loops until the match is over (`sim.rs:536`).
- Page: `History` starts at `ticks_expected` and doubles when full (`web/history.mjs:20-27`, `:74-80`). A test covers growth past the expected ticks (`web/tests/history.test.mjs:49`). The scrubber maximum is `ticks_expected` until full time, then the newest tick (`web/main.mjs:73`, `:126-129`).
- Commentary: every `penalty` event gets a line (`commentary/templates.rs:36`, `commentary/mod.rs:94`). The templates announce a penalty award (`content/commentary/en.json:448-472`).
- Versions: snapshot 3, tick file schema 4 (`record.rs:30`), `PROTOCOL_VERSION = 2` (`crates/protocol/src/lib.rs:44`), where `ticks_expected` means "the most ticks the match can last" (`lib.rs:30-31`). The answered `viewer-lineup-tactics` OQ-1 raises the protocol to version 3 in that slice.
- Tests run in this review: `cargo test -p engine --all-features --test rules_clock --test fatigue --test substitutions --test snapshot` gave 3, 3, 5, and 3 passed, with 0 failures.

## Simplicity Ladder

- Period clock → rung 3 reuse: `crates/engine/src/rules/clock.rs` → `MatchClock`, `added_seconds()`, `Tally`. Close match. Extend in place with per-period lengths. The added-time pricing is reused unchanged.
- Break before extra time and the extra-time interval → rung 3 reuse: `rules/mod.rs` → `half_time()` and `StoppageKind::HalfTime`. Close match. A `recover: bool` parameter skips `half_time_recovery()` for the two extra-time breaks. The `windows_exempt` entry then governs both breaks with no new stoppage kind.
- Rule pack fields → rung 3 reuse: the `garde` plus `deny_unknown_fields` pattern in `data/rules.rs`. Add two structs.
- Extra-time substitution allowance → rung 3 reuse: `tactics/change.rs` → `substitute()` and `SubLedger`. Exact match for the counting. Only the limit and the window count change in extra time.
- Fatigue past 90 minutes → rung 3 reuse: `fatigue.rs` → `drain()`, `multiplier()`. No change. Only a test is added.
- Shoot-out laws (eligibility, order, alternation, the early decision, sudden death) → rung 4 new code: `rules/shootout.rs`. No stdlib, platform, or in-repo equivalent exists. About 240 lines of pure functions and tests.
- Shoot-out kick → rung 3 reuse with modification: `rules/mod.rs` → `open_dead_ball()` and `DeadBall` for the set-up, `restart::target()` for the placement, `decision.rs` → the `Choice::Shot` branch of `decide_carrier()` extracted into `shot_kick()`, and `sim.rs` → `apply_kick()`, `move_ball()`, and `gain()` for the flight and the save. The new code is only the outcome router while a kick is live.
- Announced maximum → rung 3 reuse: `MatchConfig::max_ticks()` (`sim.rs:113`) and `restart::delay_ticks()` (`restart.rs:56`). The allowance is computed from the same penalty delay the kicks use.
- Seeded coin tosses → rung 3 reuse: `rng.rs` → `referee_draw()`, the word-position stream the snapshot already stores.
- Shoot-out on the wire → rung 3 reuse: the `penalty` and `full-time` event types plus optional fields, the same additive pattern as `card.kind`.
- Page history past the announced length → rung 3 reuse: `web/history.mjs` → `grow()`. Exact match. Only the scrubber maximum is new.

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist.

Repeat-deferral tripwire: this slice's `## Verification Strategy` names no environment dependency. All three criteria are `observable: false` and run under `cargo test`. The tripwire does not fire.

## Likely Files / Areas to Touch

- `content/rules/default.json`: schema 4, with the `extra_time` and `shootout` blocks.
- `crates/engine/src/data/rules.rs`: the `ExtraTime` and `Shootout` structs and `RULES_VERSION = 4`.
- `content/README.md`: the modder reference for both blocks, the allowance, and the knockout switch.
- `crates/engine/src/rules/clock.rs`: periods, per-period added time, `in_extra_time()`, the knockout and allowance tick counts, and the minute past 90.
- `crates/engine/src/rules/mod.rs`: the extra-time branch in `check_clock`, the `recover` parameter on `half_time()`, and the shoot-out driver.
- `crates/engine/src/rules/shootout.rs` (new): the shoot-out laws.
- `crates/engine/src/rules/restart.rs`: shoot-out placement targets.
- `crates/engine/src/decision.rs`: `shot_kick()` extracted from `decide_carrier()`.
- `crates/engine/src/tactics/change.rs`: the effective limit and window count in extra time, and no changes during the shoot-out.
- `crates/engine/src/sim.rs`: `MatchConfig::knockout`, the allowance in `max_ticks()`, `in_shootout()`, event fields, `Summary` fields, and the outcome routing in `goal()` and `gain()` while a kick is live.
- `crates/engine/src/snapshot.rs`: version 4 with the clock and shoot-out state.
- `crates/engine/src/observe/mod.rs`: extra-time and shoot-out statistics as additive extras.
- `crates/engine/src/commentary/mod.rs`: no line for a shoot-out penalty event.
- `crates/engine/src/scenario.rs`: `knockout()`, `period()`, `substitutions_used()`, and `shootout_kicks()`.
- `crates/engine/src/rng.rs`: the coin tosses.
- `crates/protocol/src/event.rs`, `message.rs`, `lib.rs`, `docs/reference/protocol.md`: the optional fields and the knockout meaning of `ticks_expected`.
- `crates/engine-cli/src/cli.rs`, `simulate.rs`, `bench.rs`, `serve.rs`, `record.rs`: `--knockout`.
- `crates/engine-cli/src/stream_run.rs`: the drive loop continues during the shoot-out.
- `web/history.mjs`, `web/main.mjs`, `web/tests/history.test.mjs`: the scrubber follows the newest tick.
- Tests: `crates/engine/tests/rules_extra_time.rs` (new), `rules_shootout.rs` (new), and the existing `fatigue.rs`, `content.rs`, and `snapshot.rs`.

## Proposed Change Strategy

Build from the pure layer outwards, as match-rules did. First the rule pack schema and its loader test. Next the period clock with unit tests. Then the shoot-out law functions with unit tests. Then the referee wiring behind the knockout switch, the on-pitch kick, the substitution allowance, and the fatigue test. Last, the snapshot, statistics, protocol fields, command line, stream loop, and page scrubber.

The default match must not change. A non-knockout match takes the same code path and consumes the same random draws. The `shot_kick()` extraction keeps the draw order of `decide_carrier()` exactly. The determinism test and the benchmark (the tripwire in `05c-benchmark.md`, judged per tick after match-rules Q13) pass unchanged. Tosses are drawn only when extra time and the shoot-out start. The half-time break in a regulation match still calls `half_time_recovery()`.

A shortened knockout match (`--minutes` below regulation) plays no extra time and goes straight to the shoot-out when level. This follows the existing rule that a shortened match plays no added time (`plays_added_time`, `clock.rs:65`).

The kick is on the pitch because the product owner chose option B for Q-1 (po-answers.md, 2026-09-23T08:37:28Z), which keeps the per-tick positional model of RIM-1. No NFR is the rationale for a mechanism choice here. NFR-1 (the 2-second budget per match) is checked by the benchmark on the default match only.

## Step-by-Step Plan

1. **Rule pack schema 4.** Add `ExtraTime { periods: u8 (0..=2), period_minutes: u8 (1..=30), added_max_s: u32 (0..=900), extra_substitutions: u8 (0..=3), extra_windows: u8 (0..=3) }` and `Shootout { kicks: u8 (1..=10), allowance_rounds: u8 (1..=30) }` to `data/rules.rs`, with `deny_unknown_fields` and `garde` bounds. Set `RULES_VERSION = 4` and extend the version comment. Update `content/rules/default.json` to `schema_version 4`, `periods 2`, `period_minutes 15`, `added_max_s 300`, `extra_substitutions 1`, `extra_windows 1`, `kicks 5`, and `allowance_rounds 10`. Update the pinning test in `crates/engine/tests/content.rs` in the same step, and add a test that a schema 3 rule pack is refused by version.
2. **Period clock.** In `clock.rs`, replace `halves` and `half_ticks` with a period list: the regulation halves, then the extra-time periods when the match is knockout and plays added time. Use `added_ticks: [Option<u32>; 4]`. Add `in_extra_time()`. `minute()` sums the lengths of earlier periods, so minute 105 reads 105 and the added minute keeps working. Keep `max_ticks(minutes, rules)` unchanged. Add `knockout_extra_ticks(minutes, rules)`, which returns `periods * (period_minutes * 60 + added_max_s) * 50` when the match plays added time and 0 otherwise (120,000 ticks for the default pack). Add `shootout_allowance_ticks(rules, penalty_delay_ticks)`, which returns `allowance_rounds * 2 * (3 * penalty_delay_ticks + KICK_LIVE_TICKS)`, where `KICK_LIVE_TICKS = 250` (5 s). At the shipped 750-tick penalty delay, that is 10 × 2 × 2,500 = 50,000 ticks. Extend the three existing clock unit tests and add minute tests at 95, 105, and 120 plus 1.
3. **Knockout switch and announced maximum.** Add `MatchConfig::knockout: bool` (default `false`) and `with_knockout(self) -> Self` in `sim.rs`, next to `with_manager` and `with_tactics`. `MatchConfig::new` keeps its signature, so no call site breaks. `MatchConfig::max_ticks()` (`sim.rs:113`) returns `clock::max_ticks` for a non-knockout match, which is unchanged. For a knockout match it adds `knockout_extra_ticks` and `shootout_allowance_ticks` with `restart::delay_ticks(Penalty, &tuning)`. For 90 minutes that is 360,000 + 120,000 + 50,000 = 530,000 ticks. Add a unit test next to the existing one at `sim.rs:859`.
4. **Referee branch.** In `check_clock` (`rules/mod.rs:499`), when the last regulation half ends, a knockout match that is level opens an extra-time break through `half_time(recover: false)`. This is a `HalfTime` stoppage with a `half-time` event that carries `period`. The regulation half-time calls `half_time(recover: true)`, which keeps today's behaviour. For the extra-time kick-off, a seeded toss replaces the fixed `place_kick_off(1)`, and the period clock advances. The same path, also with `recover: false`, runs between the two extra-time periods, and the team that did not kick off the first extra-time period kicks off the second. When extra time ends level, the referee starts the shoot-out (step 6). Otherwise it sets full time. Emit the signal `rules.extra_time` with the period and the added seconds.
5. **Shoot-out laws (pure).** In `rules/shootout.rs`: `eligible(players, team)` returns the players on the pitch at the end of extra time, and the larger side drops players until the numbers are equal, lowest `finishing + composure` first. `keeper(eligible)` returns the slot-0 goalkeeper when active, otherwise the eligible player with the highest `reflexes + one_on_ones`. `order(eligible)` puts outfield players first by `finishing + composure` descending, then the goalkeeper, and restarts once every eligible player has kicked. The toss decides who kicks first, then the teams alternate (A B A B). `decided(scores, kicks_taken, kicks)` ends the shoot-out when one side cannot catch up within `kicks`. After that, sudden death continues in pairs. `SAFETY_ROUNDS = 100` is the cap for step 6. Unit tests cover each rule with fixed inputs, including an early finish (3 to 0 after six kicks) and sudden death after 5 to 5.
6. **Shoot-out kick on the pitch (Q-1 option B).** In `rules/mod.rs`, a `Shootout` state on the referee drives the kicks. Every player targets the centre circle, except the kicker, the defending keeper on the goal line, and the waiting keeper at the intersection of the goal line and the penalty-area line. These are new shoot-out targets in `restart::target`. All kicks go at one goal, chosen by a seeded toss. Each kick opens a `Penalty` dead ball through `open_dead_ball` at the penalty spot, with the unscaled penalty delay, and emits a `penalty` event with `shootout_round`. When the kick is taken, the taker always shoots through `shot_kick()` (step 7) instead of deciding on the next tick. While the kick is live, only the defending keeper can gain the ball. The kick ends on the first of these outcomes:
   - The ball crosses the goal line inside the goal, which is scored.
   - The defending keeper gains the ball, which is saved.
   - The ball leaves play elsewhere, which is missed.
   - The ball speed falls below the stopped-ball threshold, which is missed.
   - `KICK_LIVE_TICKS` pass, which is missed.

   A scored kick adds to the shoot-out score, not to `summary.goals`, and sets up no kick-off. The outcome is a second `penalty` event with `shootout_scored` and `shootout_scores`, and the signal `rules.shootout_kick`. When `decided()` holds, the referee sets full time with `decided_by: shoot-out` and emits `rules.shootout_result`. If `SAFETY_ROUNDS` pass without a winner, the referee emits `rules.shootout_round_limit` at error level and ends the match as abandoned through the existing `referee.abandoned` path. The computer manager does not run during the shoot-out (`sim.rs:604-607`), and fatigue keeps draining.
7. **Shot extraction.** In `decision.rs`, move the body of the `Choice::Shot` branch of `decide_carrier()` (`:261-285`) into `pub(crate) fn shot_kick(&mut self, c: usize) -> Kick`. The branch calls it, so the draws are consumed in the same order and the default match is byte-identical. The shoot-out taker calls it directly.
8. **Extra-time substitution allowance and the shoot-out freeze (AC-2).** In `tactics/change.rs` `substitute()` (`:372`), compare the ledger against an effective limit and window count: the rule pack's `substitutions.limit` and `windows`, plus `extra_time.extra_substitutions` and `extra_windows` when `referee.clock.in_extra_time()` is true. The `LimitReached` and `NoWindowLeft` reasons report the effective values. The two extra-time breaks are `HalfTime` stoppages, so `windows_exempt` already makes them use no window (IFAB Law 3). `apply_changes()` (`:256`) returns at once while the shoot-out runs, so a queued change stays queued and counts in `change.expired_at_full_time` if the match ends first. The ledgers need no new field.
9. **Events and statistics.** Add `period` to `EngineEvent` for kick-off and half-time. A shoot-out `penalty` event gets `shootout_round` and, on the outcome event, `shootout_scored` and `shootout_scores`. Full time gets `shootout_scores` and `decided_by`. `Summary` gains `extra_added_s: [u32; 2]`, `shootout: Option<[u32; 2]>`, and the kick list. In `observe/mod.rs`, `MatchStats` gains `extra_time.played`, `extra_time.added_s`, `shootout.scores`, `shootout.kicks`, and `result.decided_by` as additive extras, the pattern `card.kind` set. A change still queued when a knockout match goes to extra time is applied at the break, so it is not counted in `change.expired_at_full_time` (`observe/mod.rs:186`). That counter counts only changes still queued when the match ends.
10. **Commentary.** In `commentary/mod.rs` (`:94`), a `penalty` event with a shoot-out field gets no line. The award templates (`content/commentary/en.json:448-472`) would otherwise say "Penalty to {team}!" for every kick. The existing line test (`crates/engine/tests/commentary.rs:239`) accepts an uncommented event, so it stays green.
11. **Snapshot version 4.** Add to `snapshot.rs`: the knockout switch, the period list and index, the four added-time slots, `extra_added_s`, and the shoot-out state. That state is the eligible kickers per team, both order cursors, the first team, the chosen goal, the scores, the kicks taken, and the live-kick start tick. Set `VERSION = 4`. The raised substitution allowance is derived from the period, and the ledgers are already stored, so no allowance field is added. Every shoot-out kick is a stoppage, so a snapshot is captured at each one (`snapshot.rs:279`).
12. **Protocol fields and the knockout length.** In `crates/protocol/src/event.rs` and `message.rs`, carry the optional fields with `skip_serializing_if = "Option::is_none"`. No new `EventType` is added. In `crates/protocol/src/lib.rs`, extend the version comment. For a knockout match, `ticks_expected` covers extra time and the rule pack's shoot-out allowance, and a sudden death past the allowance runs longer. Clients already grow their history. The optional fields survive the current version under the same judgement as the earlier additions. This slice does not raise `PROTOCOL_VERSION`. If `viewer-lineup-tactics` has already raised it to 3, the note goes under version 3. Document the fields, the knockout maximum, and how a shoot-out appears in `docs/reference/protocol.md`. Extend the document test if it reads field tables.
13. **Command line and the stream loop.** Add `--knockout` to `simulate`, `bench`, `serve`, and `record` in `cli.rs`, following the 80-column help rule, and pass it into `MatchConfig` in `simulate.rs`, `bench.rs`, `serve.rs`, and `record.rs`. `resume` reads the switch from the snapshot. In `stream_run.rs:53`, the loop runs while `!sim.is_over() && (written < opts.ticks || sim.in_shootout())`, so a long sudden death is streamed to full time.
14. **Page scrubber.** In `web/history.mjs`, add a `scrubLimit` getter that returns the larger of the announced ticks and `newestTick`. In `web/main.mjs`, set `el('scrub').max` from it while ticks arrive, not only at full time. Add a case to `web/tests/history.test.mjs` next to the growth test (`:49`). No other page file changes.
15. **Test scenes.** Add `knockout()`, `period(n)`, `substitutions_used(team, n)`, and `shootout_kicks(outcomes)` to `scenario.rs`, behind the `scenario` feature. `shootout_kicks` is a test seam that forces the outcome of the next kicks at the outcome router, so the sudden-death and early-finish paths can be driven end to end. Reuse the existing `score()` and `energy()`.
16. **Extra-time and allowance tests (AC-1 first part, AC-2).** In `crates/engine/tests/rules_extra_time.rs`:
    - A level knockout match started at minute 89 runs two 15-minute periods with added time, shows minute 105 in the second period, and emits two more `half-time` events, with periods 2 and 3.
    - A level non-knockout match ends at 90 plus added time.
    - A knockout match with a winner at 90 plays no extra time.
    - A shortened level knockout match goes straight to the shoot-out.
    - For AC-2, a team that has used 5 changes in 3 windows makes a sixth in extra time. A seventh is rejected with a `ChangeRejected` event whose reason is `LimitReached { limit: 6 }`. A change made at the break before extra time uses no window.
17. **Shoot-out tests (AC-1 second part).** In `crates/engine/tests/rules_shootout.rs`:
    - Extra time forced level by a scripted scene produces an on-pitch shoot-out and exactly one winner. The match ends in full time with `decided_by: shoot-out`.
    - The ball and the players move on the ticks of every kick.
    - Kicks alternate between the teams.
    - Each kick produces one outcome event.
    - With `shootout_kicks`, the tests cover an early finish, sudden death after 5 to 5, and the order restarting after every eligible player has kicked.
    - The numbers are equalised after a red card in extra time.
    - A change queued during the shoot-out is not applied.
    - A 10-round shoot-out ends within `max_ticks()`.
    - Two runs with one seed give an identical result.
18. **Fatigue and snapshot tests (AC-3).** In the existing `crates/engine/tests/fatigue.rs`, add a test that runs a level knockout match from minute 89 to minute 106 and records one player's energy every tick. The energy never rises across minute 90 or either break. The largest per-tick change across minute 90 is no larger than the largest per-tick change elsewhere in the run. At minute 105, the effective top speed and decisions equal the base times `multiplier(energy)`. In `crates/engine/tests/snapshot.rs`, add three tests:
    - A snapshot taken at the break before extra time resumes tick for tick to the same result.
    - A snapshot taken at a shoot-out kick resumes tick for tick to the same result.
    - A version 3 snapshot is refused with a named reason.
19. **Gates.** Run these checks:
    - `cargo fmt --check`
    - `cargo clippy --workspace --all-targets --all-features -D warnings`
    - `cargo test --workspace --all-features`
    - the page tests (`node --test web/tests/`)
    - the determinism test
    - `engine-cli bench --seed 42 --matches 5 --json`
    - the criterion bench for the tripwire comparison

    Add one `--knockout` bench run for information only. Search the diff for workflow vocabulary before the commit, as every earlier plan did.

## Verification Strategy

No user-observable AC — automated only. All three criteria in `03-slice-extra-time-penalties.md` are marked `observable: false`, and each is verified by `cargo test`. The stack declares `cargo-test`, so no environment wall exists and no `constraint-resolution:` line is needed.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-1 level knockout → extra time → shoot-out → winner | `cargo test -p engine --all-features --test rules_extra_time --test rules_shootout` (automated integration test) | Rust toolchain — yes | `scenario` builder methods `knockout()`, `period()`, and `shootout_kicks()` (step 15); `score()` exists | none needed |
| AC-2 extra-time substitution allowance available and enforced | `cargo test -p engine --all-features --test rules_extra_time` (automated) | yes; the change queue exists (`tactics/change.rs`) | `substitutions_used()` scene method (step 15) | none needed |
| AC-3 fatigue at minute 105 on the extended curve, no step at 90 | `cargo test -p engine --all-features --test fatigue` (automated) | yes; the fatigue module exists (`fatigue.rs`) | the `recover: false` break (step 4); `energy()` scene method exists | none needed |

## Test / Verification Plan

### Automated checks

- Lint and format: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -D warnings`.
- Unit tests:
  - `clock.rs`: periods, minutes, and the knockout and allowance tick counts.
  - `shootout.rs`: eligibility, the keeper, order, the early decision, sudden death, and the safety cap.
  - `data/rules.rs`: the bounds.
  - `sim.rs`: the knockout `max_ticks()`.
- Integration: `rules_extra_time.rs`, `rules_shootout.rs`, and the extended `fatigue.rs`, `content.rs`, and `snapshot.rs`. The existing `substitutions.rs`, `commentary.rs`, and `determinism.rs` must pass unchanged, which shows the default match takes the same path.
- Page: `node --test web/tests/` with the new scrubber case.
- Benchmark: `engine-cli bench --seed 42 --matches 5 --json` compared against `05c-benchmark.md` per tick (match-rules Q13). One `--knockout` run is recorded for information only.

### Interactive verification (human-in-the-loop)

Automated only. Every criterion is `observable: false`. The viewer display of a shoot-out beyond the event feed is out of scope (`03-slice-extra-time-penalties.md` Scope Out). The scrubber change is covered by the page unit test.

## Risks / Watchouts

- **R1 (medium): a shoot-out kick never ends.** Mitigation: only the defending keeper may gain the ball during a kick, the 5-second live cap ends a kick as a miss, and the 100-round safety cap ends a runaway shoot-out with an error signal. The shoot-out tests assert that every kick produces one outcome event.
- **R2 (medium): a resume drifts in extra time or the shoot-out** if one new field misses the snapshot. Mitigation: snapshot version 4 and the two continuation tests in step 18.
- **R3 (medium): the default match changes by accident.** One extra draw, one reordered branch, or a change in the `shot_kick()` extraction changes every tick file. Mitigation: tosses only when extra time and the shoot-out start, the regulation half-time keeps `recover: true`, the extraction keeps the draw order, and the unchanged determinism test is the gate.
- **R4 (low): sudden death runs past the announced maximum.** The product owner accepted this with option B. The page history already grows, the scrubber follows the newest tick (step 14), and the stream loop continues during the shoot-out (step 13).
- **R5 (low): no energy comes back before extra time.** Real players recover a little in the five-minute break. The fatigue criterion forbids a step at minute 90, so the break gives nothing back. A later tuning value can add a small recovery if calibration shows extra time is too tiring.
- **R6 (low): knockout history is larger.** At 530,000 ticks and 94 bytes a tick, the page's history reservation grows from about 33.8 MB to about 49.8 MB, but only when `--knockout` is set.
- **R7 (low): the laws were recalled, not re-read.** Law 3 (one extra substitution and one extra window in extra time, and breaks that are not windows), Law 7 (two equal periods of at most 15 minutes), and Law 10 (five kicks each, alternating, sudden death, only players on the pitch at the end of extra time, numbers equalised, and the kick ending when the ball stops or leaves play) come from recall. Verify re-reads them from the source before its sign-off.
- **R8 (low): a later event-contract redesign.** `docs/design/realism/02-event-contract-redesign.md` proposes a richer `penalty` event. The shoot-out fields are optional extras, so a later migration maps them and nothing in this slice depends on the redesign.

## Dependencies on Other Slices

- `match-rules` (complete): the referee, `MatchClock`, the stoppage hook, the snapshot, the `scenario` feature, and the penalty restart.
- `tactics-and-ai` (complete, verification passed): the fatigue module and the change queue with the substitution ledger. Steps 8, 16, and 18 extend them.
- `commentary` (complete): step 10 suppresses the award line for shoot-out kicks. Shoot-out lines are later commentary work.
- `calibration`: runs non-knockout matches, so its figures are unaffected. Step 9 keeps the `change.expired_at_full_time` counter meaning "still queued when the match ends".
- `viewer-pitch` (complete): owns `web/history.mjs` and `web/main.mjs`. Step 14 adds one getter and one assignment.
- `viewer-lineup-tactics` (planned, OQ-1 answered): raises `PROTOCOL_VERSION` to 3. Step 12 raises no version and places its note under whichever version is current when it lands.
- `viewer-match-day` and `viewer-reports-recovery`: they may show the minute past 90 and the shoot-out score from the fields.

## Assumptions

Each entry is an autonomous decision, stamped `class: implementation-detail` per `_decision-classes.md`.

- **A-1** No attribute schema change. `composure`, `finishing`, `reflexes`, and `one_on_ones` already exist in `content/attributes.json`. This retires the slice's schema-extension risk at no cost. `class: implementation-detail`.
- **A-2** A knockout match is a `MatchConfig` switch plus `--knockout`, off by default, and not a rule pack field. Whether a match is knockout is a property of the fixture, not of the laws. The change is additive, and `MatchConfig::new` keeps its signature. `class: implementation-detail`.
- **A-3** The break before extra time and the break between the two extra-time periods reuse the `HalfTime` stoppage and the `half-time` event with a `period` field. No new stoppage kind is added. The shipped `windows_exempt: ["half_time"]` already makes those breaks use no window, which matches IFAB Law 3. `class: implementation-detail`.
- **A-4** The shoot-out reaches the wire as optional fields on the existing `penalty` and `full-time` events. The observability contract's `event.type` enumeration is unchanged. The fields are additive extras, following the `card.kind` and `foul.advantage` precedent. `class: implementation-detail`.
- **A-5** The rule pack values follow IFAB Laws 3, 7, and 10: two periods of 15 minutes, one extra substitution, one extra window, and five kicks. `added_max_s: 300` caps extra-time added time. The product owner chose real-life windows (shape Q1 and Q19) and all four groups of laws (Q21). `class: implementation-detail`.
- **A-6** The engine sets the kicker order for both teams (outfield players by `finishing + composure`, the goalkeeper last). No manager input is added. The slice's criteria and scope name no manager control, and no viewer slice owns one. `class: implementation-detail`.
- **A-7** A shortened knockout match plays no extra time and goes straight to the shoot-out when level. This mirrors the rule that a shortened match plays no added time. `class: implementation-detail`.
- **A-8** Kicks alternate A B A B, and a seeded toss decides who kicks first and at which goal. This is the current IFAB order. `class: implementation-detail`.
- **A-9** Augmentations: `04b-instrument.md` and `05c-benchmark.md` are shared single-file artifacts. They are not re-authored in this run. The signals (`rules.extra_time`, `rules.shootout_kick`, `rules.shootout_result`, `rules.shootout_round_limit`) are folded into steps 4 and 6, and the benchmark comparison into step 19. `class: implementation-detail`.
- **A-10** Consult: the trigger `appetite-medium-or-larger` may hold. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`), so no consult ran. `class: implementation-detail`.
- **A-11** The discovery interview was not asked. This is an autonomous auto-review, and the discovery phase applies to new plans only. Implementation questions are settled here. The one intent-bearing question, Q-1, was answered by the product owner (po-answers.md 2026-09-23T08:37:28Z, and steer.md). `class: implementation-detail`.
- **A-12** The two extra-time breaks give no energy back (`half_time(recover: false)`). The fatigue criterion requires no discontinuity at minute 90, and the half-time recovery would add one. The regulation half-time keeps its recovery, so the default match does not change. `class: implementation-detail`.
- **A-13** The extra-time allowance is computed in `substitute()` from the rule pack and the clock, not stored. The ledgers already live in the snapshot, so a resumed match in extra time gets the same limit with no new field. `class: implementation-detail`.
- **A-14** The fatigue criterion is tested in the existing `tests/fatigue.rs`, and the fatigue module is not changed. `drain()` and `multiplier()` read no clock value, so no 90-minute clamp exists to remove. `class: implementation-detail`.
- **A-15** The auto-review was done in this session and not by a separate review agent, because this run had no agent-dispatch tool. It re-read the slice definition, the answered question, and the code named in each step, and it ran the four affected engine test files. `class: implementation-detail`.
- **A-16** The allowance is 10 rounds × 2 kicks × (3 × the penalty delay + 5 s of live ball). This is 50,000 ticks at the shipped tuning, computed from the same penalty delay the kicks use, so a tuning change cannot make the allowance stale. The bound per kick is the dead-ball hard limit (`restart.rs:50`) plus the live cap, so 10 rounds always fit. The 10-round figure is the product owner's (Q-1 option B). The formula is a derivation within that choice. `class: implementation-detail`.
- **A-17** The shoot-out taker always shoots, through `shot_kick()` extracted from `decide_carrier()`. The taker does not choose among its options. A shoot-out kick must be a shot at goal under Law 10, and the extraction reuses the finishing-scaled aim and the noise that open-play shots already use. `class: implementation-detail`.
- **A-18** A kick ends on a goal, a save by the defending keeper, the ball leaving play, the ball stopping, or 5 s of live ball. No rebound can be played. A keeper who touches the ball without catching it (`sim.rs:753-760`) does not end the kick, so a ball that goes in off the keeper scores, as in Law 10. `class: implementation-detail`.
- **A-19** Each kick emits two `penalty` events: the set-up event with `shootout_round`, and the outcome event with `shootout_scored` and `shootout_scores`. No new event type is added (A-4). A consumer can tell the two apart by the presence of `shootout_scored`. `class: implementation-detail`.
- **A-20** A 100-round safety cap ends a runaway shoot-out as an abandoned match with an error signal. With the keeper always present and shot noise on every kick, a shoot-out is level after a round with a probability near 0.625 at a 75 percent conversion rate. That makes 100 rounds about 10^-20 likely. The cap guards against a defect, not against play. `class: implementation-detail`.
- **A-21** No substitution or tactics change applies during the shoot-out. Law 10 allows only the replacement of an injured goalkeeper. The engine has no goalkeeper-injury replacement path in a shoot-out, and a queued change stays queued. `class: implementation-detail`.
- **A-22** No shoot-out commentary text is added. Shoot-out `penalty` events get no line, so the award templates are never misapplied. The slice scope names no commentary, and the commentary slice owns the text. `class: implementation-detail`.
- **A-23** This slice raises no protocol version. It documents the knockout meaning of `ticks_expected` under the current version. The product owner chose option B, "a bounded announcement, the page grows its history", over option C, which raised the version, and the page already grows its history. `class: implementation-detail`.
- **A-24** The page change is limited to the scrubber maximum (step 14). Option B names the page growing its history. `History.grow()` already does that, and a scrubber stuck at the announced length would be the one visible gap. `class: implementation-detail`.
- **A-25** The master `04-plan.md` gains this slice's summary, because the plan is now complete. `00-index.md` and the global `INDEX.md` are not edited in this run. The driver records the stage completion. `class: implementation-detail`.

## Blockers

None. Q-1 was answered with option B by the product owner (po-answers.md 2026-09-23T08:37:28Z, and steer.md "Product-owner answers to plan questions").

## Freshness Research

- Source: the installed code, read in this review. The files were `clock.rs`, `rules/mod.rs`, `restart.rs`, `decision.rs`, `sim.rs`, `data/rules.rs`, `fatigue.rs`, `tactics/change.rs`, `snapshot.rs`, `ai.rs`, `commentary/templates.rs`, `commentary/mod.rs`, `stream_run.rs`, `bench.rs`, `record.rs`, `protocol/src/lib.rs`, `web/history.mjs`, `web/main.mjs`, `content/tuning.json`, and `content/commentary/en.json`. Why it matters: every step. Takeaway: option B reuses the penalty restart, but four seams need a shoot-out guard: the snapshot at each stoppage, the change queue, the stream loop, and the award commentary. The penalty taker can choose a pass, so the shot code is extracted.
- Line drift since rev 1: `tactics/change.rs` moved after two recent fixes. `SubLedger` moved from `:121` to `:155`, and `substitute()` moved from `:328` to `:372`. The citations are corrected.
- Source: IFAB Laws of the Game, Laws 3, 7, and 10. These are **recalled, not re-read**. The first plan's fetch of theifab.com failed, and web search is excluded by the product owner. Takeaway: see R7. Verify re-reads the laws.
- No dependency is added or upgraded. `garde`, `serde`, and `rand_chacha` stay at the versions `04-plan-match-rules.md` researched.

## Recommended Next Stage

- **Option A (default): Implement** → `/wf implement football-manager-match-engine extra-time-penalties`. The plan is complete, and both slices it depends on (`match-rules` and `tactics-and-ai`) have landed. Compact the session first, so the implement stage re-reads the artifacts.
- **Option C (revisit slice):** `/wf slice football-manager-match-engine`, only if the product owner wants the shoot-out split from extra time. Nothing in this review requires it.
