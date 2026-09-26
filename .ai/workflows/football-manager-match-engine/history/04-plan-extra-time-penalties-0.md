---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: extra-time-penalties
status: awaiting-input
stage-number: 4
created-at: "2026-09-22T22:23:31Z"
updated-at: "2026-09-22T22:23:31Z"
metric-files-to-touch: 26
metric-step-count: 16
has-blockers: true
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, rules, extra-time, shoot-out, fatigue, deferred, awaiting-input]
stack-source: confirmed
open-questions:
  - id: Q-1
    class: intent-bearing
    question: "How is the penalty shoot-out simulated: as attribute draws with no ticks, or as kicks played on the pitch tick by tick?"
    options: [A-draws-no-ticks, B-on-pitch-bounded-announce, C-on-pitch-open-length]
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-extra-time-penalties.md
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md]
  steer: steer.md
  implement: 05-implement-extra-time-penalties.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine extra-time-penalties"
---

# Plan: Extra Time and Penalty Shoot-outs

## The Plan

The referee already runs two halves, added time, and a penalty restart played on the pitch. `MatchClock` (`crates/engine/src/rules/clock.rs:67`) knows two halves only, and `check_clock` (`crates/engine/src/rules/mod.rs:391`) ends the match at the end of the last half. The attribute schema already carries `composure`, `finishing`, `reflexes`, and `one_on_ones` (`content/attributes.json:16-34`), so the slice's top schema risk, thin shoot-out attributes, does not hold: no attribute schema change is needed. Fatigue and the substitution limit do not exist in code yet. `FatigueTuning` is read by nothing (`crates/engine/src/data/tuning.rs:100`), and the tactics slice that builds both has no plan yet.

The plan makes four decisions. First, the clock moves from halves to periods: two 15-minute extra-time periods come from a new `extra_time` block in the rule pack (schema 3), each period has its own added time, and a knockout match announces at most 480,000 ticks. Second, a knockout match is a switch on `MatchConfig` and a `--knockout` flag, off by default, so the default match and the benchmark do not change. Third, the shoot-out laws (eligible kickers, order, alternation, the early decision, sudden death) are pure functions in a new `rules/shootout.rs`, and they reach the wire as optional fields on existing `penalty` and `full-time` events. The observability contract's event enumeration stays as it is. Fourth, fatigue and the extra substitution are extensions of tactics-slice code. The plan names the behaviour and the tests, and it re-checks the code paths when that slice lands.

One decision is not made. Kicks resolved by attribute draws would sit outside the per-tick positional model that the product owner ratified in RIM-1. Kicks played on the pitch would make the announced maximum length unbounded under sudden death. Q-1 is therefore open and the plan is awaiting input. Every other step is ready. The main risk after Q-1 is that two of the three criteria depend on code the tactics slice has not written.

## Current State

- `MatchClock { halves, half, half_start, half_ticks, plays_added, added_ticks: [Option<u32>; 2] }` (`clock.rs:67-80`): fixed at two halves. `minute()` (`clock.rs:118`) multiplies `half * half_ticks`, which is wrong for periods of different lengths.
- `max_ticks(minutes, rules)` (`clock.rs:53`) returns regulation plus `halves * added_time.max_s`, which is 360,000 for 90 minutes. Seven call sites read `MatchConfig::max_ticks()`: `engine-cli` bench, record, resume, serve, and simulate; the stream test harness; and the fixture test.
- `check_clock` (`rules/mod.rs:391-427`) fixes added time on the first tick after regulation, then either runs `half_time()` or sets `Phase::FullTime`. `half_time()` (`rules/mod.rs:431-461`) announces a `HalfTime` stoppage through `TickSink::on_stoppage`, switches ends, and places the kick-off.
- `RulePack` schema 2 (`data/rules.rs:10`) holds `halves` (1 to 2), `half_minutes`, `substitutions { limit: 5, windows: 3 }`, the stoppage table, `added_time`, and `min_players`. It has no extra-time or shoot-out fields.
- Events: `EngineEventKind` (`sim.rs:98-120`) has 12 kinds; `protocol::EventType` (`crates/protocol/src/event.rs:15`) has 13. The observability contract's `event.type` enumeration (`.ai/observability.md:27`) has no shoot-out or extra-time value, but it has `penalty`, `half-time`, `kick-off`, and `full-time`.
- The snapshot is `VERSION 1` (`snapshot.rs:36`). The tick file is schema 4 (`record.rs:30`). `PROTOCOL_VERSION` is 2 (`crates/protocol/src/lib.rs:34`), and its `ticks_expected` means "at most".
- The test scene builder (`scenario.rs:13-119`) sets the tick, the tallies, the cards, and scripted draws. It cannot yet set the score, the period, or the knockout switch.
- Tests run this session: `cargo test -p engine --test rules_clock --test snapshot` gave 3 passed and 3 passed, with 0 failures.
- Fatigue and substitution: no engine code. The tactics slice (`03-slice-tactics-and-ai.md:44-47`) owns the curve, the change queue, and the limit of 5 changes in 3 windows plus half-time. That slice has no plan file yet.

## Simplicity Ladder

- Period clock → rung 3 reuse: `crates/engine/src/rules/clock.rs` → `MatchClock`, `added_seconds()`, `Tally`. Close match. Extend in place with per-period lengths. The added-time pricing is reused unchanged.
- Break before extra time and the extra-time interval → rung 3 reuse: `rules/mod.rs` → `half_time()` and `StoppageKind::HalfTime`. Exact match. Both breaks are half-time stoppages, so the rule pack's `half_time` row (admits tactics and substitutions) governs them with no new stoppage kind.
- Rule pack fields → rung 3 reuse: the `garde` plus `deny_unknown_fields` pattern in `data/rules.rs`. Add two structs.
- Shoot-out laws (order, alternation, the early decision, sudden death) → rung 4 new code: `rules/shootout.rs`. No stdlib, platform, or in-repo equivalent exists. About 150 lines of pure functions.
- Shoot-out kick → pending Q-1. Option A is rung 4 (a draw from the attributes). Options B and C are rung 3 reuse of the penalty restart (`restart.rs:212`, `restart.rs:259`) and the shot and keeper mechanics.
- Seeded coin toss → rung 3 reuse: `rng.rs:60` → `referee_draw()`, the word-position stream the snapshot already stores.
- Shoot-out on the wire → rung 3 reuse: the `penalty` and `full-time` event types plus optional fields, the same additive-extra pattern as `card.kind`.
- Fatigue past 90 minutes → rung 3 reuse (future): the tactics slice's curve. Only a test and an audit are added.
- Extra substitution → rung 3 reuse (future): the tactics slice's change queue. Only the limit rises.

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist.

Repeat-deferral tripwire: `00-index.md` `runtime-evidence-deferrals: []`. It does not fire. All three criteria are `observable: false` and verified by `cargo test`.

## Likely Files / Areas to Touch

- `content/rules/default.json`: schema 3, with the `extra_time` and `shootout` blocks.
- `crates/engine/src/data/rules.rs`: the `ExtraTime` and `Shootout` structs and `RULES_VERSION 3`.
- `content/README.md`: the modder reference for both blocks and the knockout switch.
- `crates/engine/src/rules/clock.rs`: periods, per-period added time, the knockout maximum, and the minute past 90.
- `crates/engine/src/rules/mod.rs`: the extra-time branch in `check_clock`, the shoot-out start, and the allowance raise.
- `crates/engine/src/rules/shootout.rs` (new): the shoot-out laws.
- `crates/engine/src/sim.rs`: `MatchConfig::knockout`, event fields, `Summary` fields, and the shoot-out phase.
- `crates/engine/src/snapshot.rs`: version 2 with the new state.
- `crates/engine/src/observe/mod.rs`: extra-time and shoot-out statistics as additive extras.
- `crates/engine/src/scenario.rs`: `knockout()`, `score()`, `period()`, and `substitutions_used()`.
- `crates/engine/src/rng.rs`: the shoot-out draw.
- The tactics slice's fatigue and change-queue modules (paths set by that slice): the extension to 120 minutes and the extra allowance.
- `crates/protocol/src/event.rs`, `crates/protocol/src/message.rs`, `docs/reference/protocol.md`: the optional fields.
- `crates/engine-cli/src/cli.rs`, `simulate.rs`, `bench.rs`, `serve.rs`, `record.rs`: `--knockout`.
- Tests: `crates/engine/tests/rules_extra_time.rs` (new), `rules_shootout.rs` (new), `fatigue_curve.rs` (new), `content.rs`, and `snapshot.rs`.

## Proposed Change Strategy

Build from the pure layer outwards, as match-rules did. First the rule pack schema and its loader test. Next the period clock with unit tests. Then the shoot-out law functions with unit tests and scripted draws. Then the referee wiring behind the knockout switch. Then the snapshot, statistics, protocol fields, and command line. Last, the two extensions of tactics-slice code, after that slice is verified.

The default match must not change. A non-knockout match takes the same code path and consumes the same random draws. The determinism test and the benchmark (the tripwire in `05c-benchmark.md`, judged per tick after match-rules Q13) pass unchanged. An extra-time added-time draw is taken only when extra time starts.

A shortened knockout match (`--minutes` below regulation) plays no extra time and goes straight to the shoot-out when level. This follows the existing rule that a shortened match plays no added time (`clock.rs:66`).

No NFR is the rationale for a mechanism choice here. NFR-1 (the 2-second budget per match) is checked by the benchmark on the default match only.

## Step-by-Step Plan

1. **Rule pack schema 3.** Add `ExtraTime { periods: u8 (0..=2), period_minutes: u8 (1..=30), added_max_s: u32 (0..=900), extra_substitutions: u8 (0..=3), extra_windows: u8 (0..=3) }` and `Shootout { kicks: u8 (1..=10) }` to `data/rules.rs`, with `deny_unknown_fields` and `garde` bounds. Set `RULES_VERSION = 3`. Update `content/rules/default.json` to `periods 2`, `period_minutes 15`, `added_max_s 300`, `extra_substitutions 1`, `extra_windows 1`, and `kicks 5`. Update the pinning test in `crates/engine/tests/content.rs` in the same step, and add a test that a schema 2 rule pack is refused by version.
2. **Period clock.** In `clock.rs`, replace `halves` and `half_ticks` with a period list: the regulation halves, then the extra-time periods when the match is knockout and plays added time. Use `added_ticks: [Option<u32>; 4]`. `minute()` sums the lengths of earlier periods, so minute 105 reads 105 and the added minute keeps working. `max_ticks(minutes, rules, knockout)` adds `periods * (period_minutes * 60 + added_max_s) * 50`, which gives 480,000 for a knockout match of 90 minutes. Extend the three existing clock unit tests and add minute tests at 95, 105, and 120 plus 1.
3. **Knockout switch.** Add `MatchConfig::knockout: bool` (default `false`) and `with_knockout(self) -> Self` in `sim.rs`. `MatchConfig::new` keeps its signature, so no call site breaks. `max_ticks()` passes the switch through.
4. **Referee branch.** In `check_clock` (`rules/mod.rs`), when the last regulation half ends, a knockout match that is level opens an extra-time break through the existing `half_time()` path, as a `HalfTime` stoppage announced through `on_stoppage`. It emits a `half-time` event with `period`. A seeded toss picks the kick-off team, and the period clock advances. The same path runs between the two extra-time periods. When extra time ends level, the referee starts the shoot-out (step 6). Otherwise it sets full time. Emit the signal `rules.extra_time` with the period and the added seconds.
5. **Shoot-out laws (pure).** In `rules/shootout.rs`: `eligible(players, team)` returns the players on the pitch at the end of extra time, and the larger side drops players until the numbers are equal, lowest `finishing + composure` first. `order(eligible)` puts outfield players first by `finishing + composure` descending, then the goalkeeper, and restarts once every eligible player has kicked. The toss decides who kicks first, then the teams alternate (A B A B). `decided(scores, kicks_taken, kicks)` ends the shoot-out when one side cannot catch up within `kicks`. After that, sudden death continues in pairs. Unit tests cover each rule with fixed inputs.
6. **Shoot-out kick. PENDING Q-1.** Option A: each kick is a draw against `p = clamp(base + w_taker * (finishing + composure) / 200 - w_keeper * (reflexes + one_on_ones) / 200)`, with tuning values chosen so that about 75 percent of kicks score at average attributes. The draws are made after the last tick, and no ticks are emitted. Options B and C: each kick is a `Penalty` dead ball at the spot, with the other players placed in the centre circle. The existing restart and shot mechanics play the kick, and it ends on a goal, a save, the ball leaving play, or the ball stopping. There are no rebounds. Step 2's `max_ticks` then adds a shoot-out allowance (B) or the announced length changes meaning (C).
7. **Events and statistics.** Add `period` to `EngineEvent` for kick-off and half-time. A penalty event in the shoot-out gets `shootout_round`, `shootout_scored`, and `shootout_scores`. Full time gets `shootout_scores` and `decided_by`. `Summary` gains `extra_added_s: [u32; 2]`, `shootout: Option<[u32; 2]>`, and the kick list. In `observe/mod.rs`, `MatchStats` gains `extra_time.played`, `extra_time.added_s`, `shootout.scores`, `shootout.kicks`, and `result.decided_by` as additive extras, the pattern `card.kind` set. Emit the signals `rules.shootout_kick` and `rules.shootout_result`.
8. **Snapshot version 2.** Add the knockout switch, the period list and index, the four added-time slots, `extra_added_s`, and the raised substitution allowance to `snapshot.rs`, and set `VERSION = 2`. The snapshot is written only at stoppages, and no stoppage occurs inside the shoot-out, so no shoot-out state is written.
9. **Protocol fields.** In `crates/protocol/src/event.rs` and `message.rs`, carry the optional fields with `skip_serializing_if = "Option::is_none"`. No new `EventType` is added, and `PROTOCOL_VERSION` stays 2 under options A and B. Document the fields, the knockout maximum, and how a shoot-out appears in `docs/reference/protocol.md`, and extend the document test if it reads field tables.
10. **Command line.** Add `--knockout` to `simulate`, `bench`, `serve`, and `record` in `cli.rs`, following the 80-column help rule, and pass it into `MatchConfig` in each command module. `resume` reads the switch from the snapshot.
11. **Test scenes.** Add `knockout()`, `score([h, a])`, `period(n)`, and `substitutions_used(team, n)` to `scenario.rs`, behind the `scenario` feature.
12. **Extra-time tests (AC-1, first part).** In `crates/engine/tests/rules_extra_time.rs`: a level knockout match started at minute 89 runs two 15-minute periods with added time, shows minute 105 in the second period, and emits two extra `half-time` events with periods 2 and 3. A level non-knockout match ends at 90 plus added time. A knockout match with a winner at 90 plays no extra time. A shortened level knockout match goes straight to the shoot-out.
13. **Shoot-out tests (AC-1, second part).** In `crates/engine/tests/rules_shootout.rs`: extra time forced level by a scripted scene produces a shoot-out and exactly one winner. The tests also cover alternation, an early finish (3 to 0 after six kicks), sudden death after 5 to 5, the order restarting after every eligible player has kicked, numbers equalised after a red card in extra time, and an identical result across two runs with one seed.
14. **Snapshot test.** A snapshot taken at the break before extra time resumes tick for tick through extra time and to the same result. A version 1 snapshot is refused with a named reason.
15. **Extensions of tactics-slice code (after that slice is verified).** (a) Fatigue: audit the curve for any division by 90, clamp at 90, or regulation-relative input, and drive it from elapsed playing ticks. Add `crates/engine/tests/fatigue_curve.rs` (AC-3), which samples every second from 0 to 120 minutes. The step across minute 90 must be no larger than the largest step elsewhere, and minute 105 must equal the closed form. (b) Allowance: when the first extra-time period starts, the substitution limit rises by `extra_substitutions` and the windows by `extra_windows`. The break before extra time and the extra-time interval do not count as windows (IFAB Law 3). In `rules_extra_time.rs` (AC-2), a team that has used 5 changes in 3 windows makes a sixth in extra time, and a seventh is rejected with a reason event.
16. **Gates.** Run `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -D warnings`, `cargo test --workspace --all-features`, the determinism test, `engine-cli bench --seed 42 --matches 5 --json`, and the criterion bench for the tripwire comparison. Add one knockout bench run for information only. Search the diff for workflow vocabulary before the commit, as every earlier plan did.

## Verification Strategy

No user-observable AC — automated only. All three criteria in `03-slice-extra-time-penalties.md` are marked `observable: false`, and each is verified by `cargo test`. The stack declares `cargo-test`, so no environment wall exists and no `constraint-resolution:` line is needed.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-1 level knockout → extra time → shoot-out → winner | `cargo test --test rules_extra_time --test rules_shootout --features scenario` (rung: automated integration test) | Rust toolchain — yes (`rustc 1.92.0`) | `scenario` builder methods `knockout()`, `score()`, `period()` (step 11) | none needed |
| AC-2 extra-time substitution allowance available and enforced | `cargo test --test rules_extra_time` (automated) | yes; needs the tactics slice's change queue | `substitutions_used()` scene method (step 11) | if the tactics slice has not landed, implement is blocked, not deferred |
| AC-3 fatigue at minute 105 on the extended curve, no step at 90 | `cargo test --test fatigue_curve` (automated) | yes; needs the tactics slice's curve | none beyond the test | same as AC-2 |

## Test / Verification Plan

### Automated checks

- Lint and format: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -D warnings`.
- Unit: the `clock.rs` tests (periods and minutes), the `shootout.rs` tests (eligibility, order, the early decision, sudden death), and the `data/rules.rs` bounds.
- Integration: `rules_extra_time.rs`, `rules_shootout.rs`, `fatigue_curve.rs`, and the extended `content.rs` and `snapshot.rs`. `determinism.rs` must stay unchanged and pass, which shows the default match takes the same path.
- Benchmark: `engine-cli bench --seed 42 --matches 5 --json` compared against `05c-benchmark.md` per tick (match-rules Q13). One `--knockout` run is recorded for information only.

### Interactive verification (human-in-the-loop)

Automated only — every criterion is `observable: false`, and the viewer display of a shoot-out beyond the event feed is out of scope (`03-slice-extra-time-penalties.md` Scope Out).

## Risks / Watchouts

- **R1 (high): Q-1 is open.** See Blockers. Steps 2, 6, and 9 have alternatives recorded for each option.
- **R2 (high): the tactics slice has not been planned or built.** Criteria AC-2 and AC-3 extend code that does not exist. Mitigation: step 15 is written against behaviour, not code. Before implement, re-run `/wf plan football-manager-match-engine extra-time-penalties` (auto-review) once the tactics slice is verified, so the file paths and the curve's form are confirmed.
- **R3 (medium): a resume drifts in extra time** if one new field misses the snapshot. Mitigation: snapshot version 2 and the continuation test in step 14.
- **R4 (medium): the default match changes by accident.** One extra draw, or one reordered branch, changes every tick file. Mitigation: extra-time draws are taken only when extra time starts, and the unchanged determinism test is the gate.
- **R5 (low): knockout history is larger.** At 480,000 ticks, the page's history reservation grows from about 33.8 MB to about 45 MB, but only when `--knockout` is set.
- **R6 (low): the laws were recalled, not re-read.** The IFAB page fetch failed this session with a 429 error. Law 3 (one extra substitution and one extra window in extra time), Law 7 (two equal periods of at most 15 minutes), and Law 10 (five kicks each, alternating, sudden death, only players on the pitch at the end of extra time, numbers equalised) come from recall. Verify re-reads them from the source before its sign-off.

## Dependencies on Other Slices

- `match-rules` (verified): the referee, `MatchClock`, the stoppage hook, the snapshot, the `scenario` feature, and the penalty restart.
- `tactics-and-ai` (not yet planned): the fatigue curve and the change queue with the substitution limit. Steps 15a and 15b wait for it.
- `commentary`: reads the new optional event fields to write extra-time and shoot-out lines. This slice adds no commentary text.
- `viewer-match-day` and `viewer-reports-recovery`: they may show the minute past 90 and the shoot-out score from the fields. This slice changes no page file.
- `calibration`: runs non-knockout matches, so it is unaffected.

## Assumptions

Each entry is an autonomous decision, stamped `class: implementation-detail` per `_decision-classes.md`.

- **A-1** No attribute schema change. `composure`, `finishing`, `reflexes`, and `one_on_ones` already exist (`content/attributes.json:16-34`). This retires the slice's schema-extension risk at no cost. `class: implementation-detail`.
- **A-2** A knockout match is a `MatchConfig` switch plus `--knockout`, off by default, and not a rule pack field. Whether a match is knockout is a property of the fixture, not of the laws. The change is additive, and `MatchConfig::new` keeps its signature. `class: implementation-detail`.
- **A-3** The break before extra time and the break between the two extra-time periods reuse the `HalfTime` stoppage and the `half-time` event with a `period` field. No new stoppage kind is added. This keeps the rule pack's stoppage table unchanged and matches IFAB Law 3, under which those breaks are not windows. `class: implementation-detail`.
- **A-4** The shoot-out reaches the wire as optional fields on the existing `penalty` and `full-time` events. The observability contract's `event.type` enumeration (`.ai/observability.md:27`) is unchanged. The fields are additive extras, the precedent `card.kind` and `foul.advantage` set. `class: implementation-detail`.
- **A-5** The rule pack values follow IFAB Laws 3, 7, and 10: two periods of 15 minutes, one extra substitution, one extra window, and five kicks. `added_max_s: 300` caps extra-time added time. The product owner chose real-life windows (shape Q1 and Q19) and all four groups of laws (Q21). `class: implementation-detail`.
- **A-6** The engine sets the kicker order for both teams (outfield players by `finishing + composure`, the goalkeeper last). No manager input is added. The slice's criteria and scope name no manager control, and no viewer slice owns one. `class: implementation-detail`.
- **A-7** A shortened knockout match plays no extra time and goes straight to the shoot-out when level. This mirrors the rule that a shortened match plays no added time. `class: implementation-detail`.
- **A-8** Kicks alternate A B A B, and a seeded toss decides who kicks first. This is the current IFAB order. `class: implementation-detail`.
- **A-9** Augmentations: `04b-instrument.md` and `05c-benchmark.md` are shared single-file artifacts that currently serve match-rules. They are not re-authored in this run. The signals (`rules.extra_time`, `rules.shootout_kick`, `rules.shootout_result`) are folded into steps 4 and 7, and the benchmark comparison into step 16. Re-authoring happens when this deferred slice is next in line for implementation. `class: implementation-detail`.
- **A-10** The master `04-plan.md` is not updated. It counts complete plans, this plan is awaiting input, and the sibling plans stopped in this run followed the same rule. `class: implementation-detail`.
- **A-11** Consult: the trigger `appetite-medium-or-larger` holds. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`). Not fired. `class: implementation-detail`.
- **A-12** The discovery interview was not asked. This run is autonomous: implementation questions are settled above, and the one intent-bearing question is recorded as Q-1. `class: implementation-detail`.

## Blockers

**Q-1 (intent-bearing — core-loop mechanism and the match-length contract). How is the penalty shoot-out simulated?**

Why this is not decided autonomously: option A resolves kicks by probability draws. That is the event-probability model which RIM-1 set aside in favour of per-tick positional play, ratified by the product owner (criteria 2 and 4 of `_decision-classes.md`), and it also decides what the user sees during a shoot-out. Options B and C keep the positional model, but they change the meaning or the size of the announced maximum match length, which the page uses to reserve history (`docs/reference/protocol.md`, `crates/protocol/src/lib.rs:21-28`).

- **Option A: draws, no ticks.** Each kick is a seeded draw from the taker's finishing and composure against the keeper's reflexes and one-on-ones. The shoot-out takes place after the last tick and appears only as events in the feed. *Consequence:* this is the smallest option, about 150 lines, and the announced maximum stays bounded at 480,000 ticks. The pitch stands still during the shoot-out, and this one phase is not positional.
- **Option B (recommended): on the pitch, with a bounded announcement.** Each kick is played through the existing penalty restart and shot mechanics, tick by tick. The announced maximum covers regulation, extra time, and a shoot-out allowance (for example 10 rounds, about 25,000 ticks). The page grows its history if sudden death runs past the allowance. *Consequence:* the model stays positional and the pitch shows the kicks. It costs about 300 more lines, and one page change grows history past the allowance.
- **Option C: on the pitch, open length.** As B, but `ticks_expected` becomes "unknown" for a knockout match, and `PROTOCOL_VERSION` rises to 3. *Consequence:* the positional model is kept, and the contract is honest about sudden death. Every page consumer must handle a match of unknown length.

Recorded in `po-answers.md` as awaiting input. On an answer, re-run `/wf plan football-manager-match-engine extra-time-penalties` with the answer as feedback. That run fixes steps 2, 6, and 9 and sets `status: complete`.

## Freshness Research

- Source: the installed code, read this session (`clock.rs`, `rules/mod.rs`, `data/rules.rs`, `sim.rs`, `event.rs`, `attributes.json`, `tuning.rs`). Why it matters: every step. Takeaway: extra time extends `MatchClock` and `check_clock`, and the shoot-out attributes already exist.
- Source: IFAB Laws of the Game, Laws 3, 7, and 10 — **recalled, not re-read**. The `web-reader` fetch of theifab.com failed with error 429 (insufficient balance), and `web-search-prime` is excluded by the product owner. Takeaway: see R6. Verify re-reads the laws.
- No dependency is added or upgraded. `garde`, `serde`, and `rand_chacha` stay at the versions `04-plan-match-rules.md` researched.

## Recommended Next Stage

- **Awaiting input (this plan):** answer Q-1, then re-run `/wf plan football-manager-match-engine extra-time-penalties` with the answer.
- **Option A (after Q-1): Implement** → `/wf implement football-manager-match-engine extra-time-penalties`. Only after the tactics slice is implemented and verified: this slice is deferred, and step 15 depends on it.
- **Option C (revisit slice):** `/wf slice football-manager-match-engine`, if the product owner wants the fatigue and substitution criteria moved into the tactics slice, so this slice depends only on match-rules.
