---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: extra-time-penalties
status: awaiting-input
stage-number: 4
created-at: "2026-09-22T22:23:31Z"
updated-at: "2026-09-23T07:03:59Z"
metric-files-to-touch: 25
metric-step-count: 16
has-blockers: true
revision-count: 1
revisions:
  - rev: 1
    at: "2026-09-23T07:03:59Z"
    trigger: manual
    because: "auto-review — 9 issues found"
    changed: "tactics slice has landed: steps grounded on fatigue.rs and tactics/change.rs; rule pack schema 4 and snapshot version 4; break before extra time gives no energy back; Q-1 still open"
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
  siblings: [04-plan-engine-core.md, 04-plan-data-schemas-generator.md, 04-plan-stream-protocol.md, 04-plan-viewer-pitch.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md]
  steer: steer.md
  implement: 05-implement-extra-time-penalties.md
next-command: wf-plan
next-invocation: "/wf plan football-manager-match-engine extra-time-penalties"
---

# Plan: Extra Time and Penalty Shoot-outs

## The Plan

The referee runs two halves, added time, and a penalty restart played on the pitch. The tactics slice has now landed and passed verification. Energy drains per tick from speed and stamina, with no input from the clock (`crates/engine/src/fatigue.rs:78`). Substitutions go through one ledger per team that reads the limit and the window count from the rule pack (`crates/engine/src/tactics/change.rs:337-345`). `MatchClock` (`crates/engine/src/rules/clock.rs:71`) still knows only halves, and `check_clock` (`crates/engine/src/rules/mod.rs:499`) ends the match at the end of the last half. The attribute schema already has `composure`, `finishing`, `reflexes`, and `one_on_ones`, so the slice's top schema risk does not hold.

An auto-review of the first plan found 9 issues. Most came from the tactics slice landing. The rule pack is already at schema 3, so extra time moves it to schema 4, and the snapshot moves from version 3 to version 4. The main new finding concerns the fatigue criterion. The half-time path gives every player some energy back, so a break before extra time that reuses it would make energy jump at minute 90. That jump would fail the "no discontinuity at minute 90" criterion. The plan therefore reuses the half-time stoppage without the recovery. The rule pack already exempts half-time stoppages from windows (`windows_exempt: ["half_time"]`), so the two extra-time breaks use no window, as Law 3 requires, and no new code is needed for that. The four earlier decisions stand: periods in the clock, a knockout switch that is off by default, shoot-out laws as pure functions, and optional fields on existing events.

One decision is still not made. Kicks resolved by attribute draws would sit outside the per-tick positional model that the product owner ratified in RIM-1. Kicks played on the pitch would make the announced maximum length unbounded under sudden death. Q-1 is therefore open and the plan is awaiting input. Every other step is ready, and none of them now waits for another slice.

## Current State

- `MatchClock { halves, half, half_start, half_ticks, plays_added, added_ticks: [Option<u32>; 2] }` (`clock.rs:71-82`) supports two halves only. `minute()` (`clock.rs:120`) multiplies `half * half_ticks`, which is wrong for periods of different lengths.
- `max_ticks(minutes, rules)` (`clock.rs:55`) returns regulation plus `halves * added_time.max_s`, which is 360,000 ticks for 90 minutes (`max_s` is 900 in `content/rules/default.json`). A search for `max_ticks()` in `crates/` gives 12 matches.
- `check_clock` (`rules/mod.rs:499-537`) fixes added time on the first tick after regulation, then either runs `half_time()` or sets `Phase::FullTime`. `half_time()` (`rules/mod.rs:541-570`) pushes a `HalfTime` event, calls `half_time_recovery()`, opens a `HalfTime` stoppage, switches ends, and places the kick-off.
- `RulePack` is at schema 3 (`data/rules.rs:13`). Version 3 added `substitutions.windows_exempt`, and the shipped pack exempts `half_time`. It has no extra-time or shoot-out fields.
- Fatigue (`fatigue.rs`): `drain()` (`:78`) depends on speed, stamina, and `dt` only. `multiplier()` (`:46`) maps energy to a factor through the tuning curve. `half_time_recovery()` (`:127`) adds energy at half-time. No function reads the minute or a 90-minute constant. `crates/engine/tests/fatigue.rs` has 3 tests.
- Substitutions (`tactics/change.rs`): `SubLedger { used, windows, window_at }` (`:121`). `substitute()` (`:328`) rejects with `LimitReached` or `NoWindowLeft` against `config.rules.substitutions`. A rejection reaches the stream as a `ChangeRejected` event. The snapshot already stores both ledgers (`snapshot.rs:602-606`).
- Events: `EngineEventKind` (`sim.rs:161`) has 17 variants, and `protocol::EventType` (`crates/protocol/src/event.rs:16`) has 16. The observability contract's `event.type` enumeration (`.ai/observability.md:27`) has `penalty`, `half-time`, `kick-off`, and `full-time`, and no shoot-out or extra-time value.
- Versions: snapshot `VERSION = 3` (`snapshot.rs:41`), tick file schema 4 (`record.rs:30`), `PROTOCOL_VERSION = 2` (`crates/protocol/src/lib.rs:44`), where `ticks_expected` means "at most".
- The test scene builder (`scenario.rs`) already has `score()` (`:149`) and `energy()` (`:142`). It has no knockout switch, period, or substitutions-used setter.
- Tests run in this review: `cargo test -p engine --all-features --test rules_clock --test fatigue --test substitutions --test snapshot` gave 3, 3, 5, and 3 passed, with 0 failures.

## Simplicity Ladder

- Period clock → rung 3 reuse: `crates/engine/src/rules/clock.rs` → `MatchClock`, `added_seconds()`, `Tally`. Close match. Extend in place with per-period lengths. The added-time pricing is reused unchanged.
- Break before extra time and the extra-time interval → rung 3 reuse: `rules/mod.rs` → `half_time()` and `StoppageKind::HalfTime`. Close match. A `recover: bool` parameter skips `half_time_recovery()` for the two extra-time breaks. The rule pack's `half_time` row and the `windows_exempt` entry then govern both breaks with no new stoppage kind.
- Rule pack fields → rung 3 reuse: the `garde` plus `deny_unknown_fields` pattern in `data/rules.rs`. Add two structs.
- Extra-time substitution allowance → rung 3 reuse: `tactics/change.rs` → `substitute()` and `SubLedger`. Exact match for the counting. Only the limit and the window count it compares against change once the clock is in extra time.
- Fatigue past 90 minutes → rung 3 reuse: `fatigue.rs` → `drain()`, `multiplier()`. No change to the module. It has no clock input, so the curve already continues past minute 90. Only a test is added.
- Shoot-out laws (order, alternation, the early decision, sudden death) → rung 4 new code: `rules/shootout.rs`. No stdlib, platform, or in-repo equivalent exists. About 150 lines of pure functions.
- Shoot-out kick → pending Q-1. Option A is rung 4 (a draw from the attributes). Options B and C are rung 3 reuse of the penalty restart and the shot and keeper mechanics.
- Seeded coin toss → rung 3 reuse: `rng.rs` → `referee_draw()`, the word-position stream the snapshot already stores.
- Shoot-out on the wire → rung 3 reuse: the `penalty` and `full-time` event types plus optional fields, the same additive-extra pattern as `card.kind`.

## Applied Learnings

No applicable learnings found. `.ai/solutions/` does not exist.

Repeat-deferral tripwire: this slice's `## Verification Strategy` names no environment dependency. All three criteria are `observable: false` and run under `cargo test`. The tripwire does not fire.

## Likely Files / Areas to Touch

- `content/rules/default.json`: schema 4, with the `extra_time` and `shootout` blocks.
- `crates/engine/src/data/rules.rs`: the `ExtraTime` and `Shootout` structs and `RULES_VERSION = 4`.
- `content/README.md`: the modder reference for both blocks and the knockout switch.
- `crates/engine/src/rules/clock.rs`: periods, per-period added time, `in_extra_time()`, the knockout maximum, and the minute past 90.
- `crates/engine/src/rules/mod.rs`: the extra-time branch in `check_clock`, the `recover` parameter on `half_time()`, and the shoot-out start.
- `crates/engine/src/rules/shootout.rs` (new): the shoot-out laws.
- `crates/engine/src/tactics/change.rs`: the effective limit and window count in extra time.
- `crates/engine/src/sim.rs`: `MatchConfig::knockout`, event fields, `Summary` fields, and the shoot-out phase.
- `crates/engine/src/snapshot.rs`: version 4 with the new clock state.
- `crates/engine/src/observe/mod.rs`: extra-time and shoot-out statistics as additive extras.
- `crates/engine/src/scenario.rs`: `knockout()`, `period()`, and `substitutions_used()`.
- `crates/engine/src/rng.rs`: the shoot-out draw.
- `crates/protocol/src/event.rs`, `crates/protocol/src/message.rs`, `docs/reference/protocol.md`: the optional fields.
- `crates/engine-cli/src/cli.rs`, `simulate.rs`, `bench.rs`, `serve.rs`, `record.rs`: `--knockout`.
- Tests: `crates/engine/tests/rules_extra_time.rs` (new), `rules_shootout.rs` (new), and the existing `fatigue.rs`, `content.rs`, and `snapshot.rs`.

## Proposed Change Strategy

Build from the pure layer outwards, as match-rules did. First the rule pack schema and its loader test. Next the period clock with unit tests. Then the shoot-out law functions with unit tests and scripted draws. Then the referee wiring behind the knockout switch, the substitution allowance, and the fatigue test. Last, the snapshot, statistics, protocol fields, and command line.

The default match must not change. A non-knockout match takes the same code path and consumes the same random draws. The determinism test and the benchmark (the tripwire in `05c-benchmark.md`, judged per tick after match-rules Q13) pass unchanged. An extra-time added-time draw is taken only when extra time starts. The half-time break in a regulation match still calls `half_time_recovery()`.

A shortened knockout match (`--minutes` below regulation) plays no extra time and goes straight to the shoot-out when level. This follows the existing rule that a shortened match plays no added time (`plays_added_time`, `clock.rs:65`).

No NFR is the rationale for a mechanism choice here. NFR-1 (the 2-second budget per match) is checked by the benchmark on the default match only.

## Step-by-Step Plan

1. **Rule pack schema 4.** Add `ExtraTime { periods: u8 (0..=2), period_minutes: u8 (1..=30), added_max_s: u32 (0..=900), extra_substitutions: u8 (0..=3), extra_windows: u8 (0..=3) }` and `Shootout { kicks: u8 (1..=10) }` to `data/rules.rs`, with `deny_unknown_fields` and `garde` bounds. Set `RULES_VERSION = 4` and extend the version comment. Update `content/rules/default.json` to `schema_version 4`, `periods 2`, `period_minutes 15`, `added_max_s 300`, `extra_substitutions 1`, `extra_windows 1`, and `kicks 5`. Update the pinning test in `crates/engine/tests/content.rs` in the same step, and add a test that a schema 3 rule pack is refused by version.
2. **Period clock.** In `clock.rs`, replace `halves` and `half_ticks` with a period list: the regulation halves, then the extra-time periods when the match is knockout and plays added time. Use `added_ticks: [Option<u32>; 4]`. Add `in_extra_time()`. `minute()` sums the lengths of earlier periods, so minute 105 reads 105 and the added minute keeps working. `max_ticks(minutes, rules, knockout)` adds `periods * (period_minutes * 60 + added_max_s) * 50`, which gives 480,000 for a knockout match of 90 minutes. Extend the three existing clock unit tests and add minute tests at 95, 105, and 120 plus 1.
3. **Knockout switch.** Add `MatchConfig::knockout: bool` (default `false`) and `with_knockout(self) -> Self` in `sim.rs`, next to `with_manager` and `with_tactics`. `MatchConfig::new` keeps its signature, so no call site breaks. `max_ticks()` passes the switch through.
4. **Referee branch.** In `check_clock` (`rules/mod.rs`), when the last regulation half ends, a knockout match that is level opens an extra-time break through `half_time(recover: false)`. This is a `HalfTime` stoppage, announced through `on_stoppage`, with a `half-time` event that carries `period`. The regulation half-time calls `half_time(recover: true)`, which keeps today's behaviour. A seeded toss picks the kick-off team, and the period clock advances. The same path, also with `recover: false`, runs between the two extra-time periods. When extra time ends level, the referee starts the shoot-out (step 6). Otherwise it sets full time. Emit the signal `rules.extra_time` with the period and the added seconds.
5. **Shoot-out laws (pure).** In `rules/shootout.rs`: `eligible(players, team)` returns the players on the pitch at the end of extra time, and the larger side drops players until the numbers are equal, lowest `finishing + composure` first. `order(eligible)` puts outfield players first by `finishing + composure` descending, then the goalkeeper, and restarts once every eligible player has kicked. The toss decides who kicks first, then the teams alternate (A B A B). `decided(scores, kicks_taken, kicks)` ends the shoot-out when one side cannot catch up within `kicks`. After that, sudden death continues in pairs. Unit tests cover each rule with fixed inputs.
6. **Shoot-out kick. PENDING Q-1.** Option A: each kick is a draw against `p = clamp(base + w_taker * (finishing + composure) / 200 - w_keeper * (reflexes + one_on_ones) / 200)`, with tuning values chosen so that about 75 percent of kicks score at average attributes. The draws are made after the last tick, and no ticks are emitted. Options B and C: each kick is a `Penalty` dead ball at the spot, with the other players placed in the centre circle. The existing restart and shot mechanics play the kick, and it ends on a goal, a save, the ball leaving play, or the ball stopping. There are no rebounds. Step 2's `max_ticks` then adds a shoot-out allowance (B), or the announced length changes meaning (C).
7. **Extra-time substitution allowance (AC-2).** In `tactics/change.rs` `substitute()`, compare the ledger against an effective limit and window count: the rule pack's `substitutions.limit` and `windows`, plus `extra_time.extra_substitutions` and `extra_windows` when `referee.clock.in_extra_time()` is true. The `LimitReached` and `NoWindowLeft` reasons report the effective values. The two extra-time breaks are `HalfTime` stoppages, so `windows_exempt` already makes them use no window (IFAB Law 3). The ledgers need no new field.
8. **Events and statistics.** Add `period` to `EngineEvent` for kick-off and half-time. A penalty event in the shoot-out gets `shootout_round`, `shootout_scored`, and `shootout_scores`. Full time gets `shootout_scores` and `decided_by`. `Summary` gains `extra_added_s: [u32; 2]`, `shootout: Option<[u32; 2]>`, and the kick list. In `observe/mod.rs`, `MatchStats` gains `extra_time.played`, `extra_time.added_s`, `shootout.scores`, `shootout.kicks`, and `result.decided_by` as additive extras, the pattern `card.kind` set. Emit the signals `rules.shootout_kick` and `rules.shootout_result`. A change still queued when a knockout match goes to extra time is applied at the break, so it is not counted in `change.expired_at_full_time`. That counter counts only changes still queued when the match ends.
9. **Snapshot version 4.** Add the knockout switch, the period list and index, the four added-time slots, and `extra_added_s` to `snapshot.rs`, and set `VERSION = 4`. The raised allowance is derived from the period, and the ledgers are already stored, so no allowance field is added. The snapshot is written only at stoppages, and no stoppage occurs inside the shoot-out, so no shoot-out state is written.
10. **Protocol fields.** In `crates/protocol/src/event.rs` and `message.rs`, carry the optional fields with `skip_serializing_if = "Option::is_none"`. No new `EventType` is added, and `PROTOCOL_VERSION` stays 2 under options A and B. Document the fields, the knockout maximum, and how a shoot-out appears in `docs/reference/protocol.md`, and extend the document test if it reads field tables.
11. **Command line.** Add `--knockout` to `simulate`, `bench`, `serve`, and `record` in `cli.rs`, following the 80-column help rule, and pass it into `MatchConfig` in `simulate.rs`, `bench.rs`, `serve.rs`, and `record.rs`. `resume` reads the switch from the snapshot.
12. **Test scenes.** Add `knockout()`, `period(n)`, and `substitutions_used(team, n)` to `scenario.rs`, behind the `scenario` feature. Reuse the existing `score()` and `energy()`.
13. **Extra-time and allowance tests (AC-1 first part, AC-2).** In `crates/engine/tests/rules_extra_time.rs`: a level knockout match started at minute 89 runs two 15-minute periods with added time, shows minute 105 in the second period, and emits two extra `half-time` events with periods 2 and 3. A level non-knockout match ends at 90 plus added time. A knockout match with a winner at 90 plays no extra time. A shortened level knockout match goes straight to the shoot-out. For AC-2, a team that has used 5 changes in 3 windows makes a sixth in extra time, and a seventh is rejected with a `ChangeRejected` event whose reason is `LimitReached { limit: 6 }`. A change made at the break before extra time uses no window.
14. **Shoot-out tests (AC-1 second part).** In `crates/engine/tests/rules_shootout.rs`: extra time forced level by a scripted scene produces a shoot-out and exactly one winner. The tests also cover alternation, an early finish (3 to 0 after six kicks), sudden death after 5 to 5, the order restarting after every eligible player has kicked, numbers equalised after a red card in extra time, and an identical result across two runs with one seed.
15. **Fatigue and snapshot tests (AC-3).** In the existing `crates/engine/tests/fatigue.rs`, add a test that runs a level knockout match from minute 89 to minute 106 and records one player's energy every tick. The energy never rises across minute 90 or either break. The largest per-tick change across minute 90 is no larger than the largest per-tick change elsewhere in the run. At minute 105, the effective top speed and decisions equal the base times `multiplier(energy)`. In `crates/engine/tests/snapshot.rs`, a snapshot taken at the break before extra time resumes tick for tick through extra time and to the same result, and a version 3 snapshot is refused with a named reason.
16. **Gates.** Run `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -D warnings`, `cargo test --workspace --all-features`, the determinism test, `engine-cli bench --seed 42 --matches 5 --json`, and the criterion bench for the tripwire comparison. Add one knockout bench run for information only. Search the diff for workflow vocabulary before the commit, as every earlier plan did.

## Verification Strategy

No user-observable AC — automated only. All three criteria in `03-slice-extra-time-penalties.md` are marked `observable: false`, and each is verified by `cargo test`. The stack declares `cargo-test`, so no environment wall exists and no `constraint-resolution:` line is needed.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| AC-1 level knockout → extra time → shoot-out → winner | `cargo test -p engine --all-features --test rules_extra_time --test rules_shootout` (automated integration test) | Rust toolchain — yes | `scenario` builder methods `knockout()` and `period()` (step 12); `score()` exists | none needed |
| AC-2 extra-time substitution allowance available and enforced | `cargo test -p engine --all-features --test rules_extra_time` (automated) | yes; the change queue exists (`tactics/change.rs`) | `substitutions_used()` scene method (step 12) | none needed |
| AC-3 fatigue at minute 105 on the extended curve, no step at 90 | `cargo test -p engine --all-features --test fatigue` (automated) | yes; the fatigue module exists (`fatigue.rs`) | the `recover: false` break (step 4); `energy()` scene method exists | none needed |

## Test / Verification Plan

### Automated checks

- Lint and format: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -D warnings`.
- Unit: the `clock.rs` tests (periods and minutes), the `shootout.rs` tests (eligibility, order, the early decision, sudden death), and the `data/rules.rs` bounds.
- Integration: `rules_extra_time.rs`, `rules_shootout.rs`, and the extended `fatigue.rs`, `content.rs`, and `snapshot.rs`. The existing `substitutions.rs` and `determinism.rs` must pass unchanged, which shows the default match takes the same path.
- Benchmark: `engine-cli bench --seed 42 --matches 5 --json` compared against `05c-benchmark.md` per tick (match-rules Q13). One `--knockout` run is recorded for information only.

### Interactive verification (human-in-the-loop)

Automated only — every criterion is `observable: false`, and the viewer display of a shoot-out beyond the event feed is out of scope (`03-slice-extra-time-penalties.md` Scope Out).

## Risks / Watchouts

- **R1 (high): Q-1 is open.** See Blockers. Steps 2, 6, and 10 have alternatives recorded for each option.
- **R2 (medium): a resume drifts in extra time** if one new clock field misses the snapshot. Mitigation: snapshot version 4 and the continuation test in step 15.
- **R3 (medium): the default match changes by accident.** One extra draw, or one reordered branch, changes every tick file. Mitigation: extra-time draws are taken only when extra time starts, the regulation half-time keeps `recover: true`, and the unchanged determinism test is the gate.
- **R4 (low): no energy comes back before extra time.** Real players recover a little in the five-minute break. The fatigue criterion forbids a step at minute 90, so the break gives nothing back. A later rule pack or tuning value can add a small recovery if calibration shows extra time is too tired.
- **R5 (low): knockout history is larger.** At 480,000 ticks, the page's history reservation grows from about 33.8 MB to about 45 MB, but only when `--knockout` is set.
- **R6 (low): the laws were recalled, not re-read.** Law 3 (one extra substitution and one extra window in extra time, and breaks that are not windows), Law 7 (two equal periods of at most 15 minutes), and Law 10 (five kicks each, alternating, sudden death, only players on the pitch at the end of extra time, numbers equalised) come from recall. Verify re-reads them from the source before its sign-off.

## Dependencies on Other Slices

- `match-rules` (complete): the referee, `MatchClock`, the stoppage hook, the snapshot, the `scenario` feature, and the penalty restart.
- `tactics-and-ai` (complete, verification passed): the fatigue module and the change queue with the substitution ledger. Steps 7, 13, and 15 extend them.
- `commentary` (complete): reads events. It can later write extra-time and shoot-out lines from the new optional fields. This slice adds no commentary text.
- `calibration`: runs non-knockout matches, so its figures are unaffected. Step 8 keeps the `change.expired_at_full_time` counter meaning "still queued when the match ends".
- `viewer-match-day` and `viewer-reports-recovery`: they may show the minute past 90 and the shoot-out score from the fields. This slice changes no page file.

## Assumptions

Each entry is an autonomous decision, stamped `class: implementation-detail` per `_decision-classes.md`.

- **A-1** No attribute schema change. `composure`, `finishing`, `reflexes`, and `one_on_ones` already exist in `content/attributes.json`. This retires the slice's schema-extension risk at no cost. `class: implementation-detail`.
- **A-2** A knockout match is a `MatchConfig` switch plus `--knockout`, off by default, and not a rule pack field. Whether a match is knockout is a property of the fixture, not of the laws. The change is additive, and `MatchConfig::new` keeps its signature. `class: implementation-detail`.
- **A-3** The break before extra time and the break between the two extra-time periods reuse the `HalfTime` stoppage and the `half-time` event with a `period` field. No new stoppage kind is added. The shipped `windows_exempt: ["half_time"]` already makes those breaks use no window, which matches IFAB Law 3. `class: implementation-detail`.
- **A-4** The shoot-out reaches the wire as optional fields on the existing `penalty` and `full-time` events. The observability contract's `event.type` enumeration is unchanged. The fields are additive extras, following the `card.kind` and `foul.advantage` precedent. `class: implementation-detail`.
- **A-5** The rule pack values follow IFAB Laws 3, 7, and 10: two periods of 15 minutes, one extra substitution, one extra window, and five kicks. `added_max_s: 300` caps extra-time added time. The product owner chose real-life windows (shape Q1 and Q19) and all four groups of laws (Q21). `class: implementation-detail`.
- **A-6** The engine sets the kicker order for both teams (outfield players by `finishing + composure`, the goalkeeper last). No manager input is added. The slice's criteria and scope name no manager control, and no viewer slice owns one. `class: implementation-detail`.
- **A-7** A shortened knockout match plays no extra time and goes straight to the shoot-out when level. This mirrors the rule that a shortened match plays no added time. `class: implementation-detail`.
- **A-8** Kicks alternate A B A B, and a seeded toss decides who kicks first. This is the current IFAB order. `class: implementation-detail`.
- **A-9** Augmentations: `04b-instrument.md` and `05c-benchmark.md` are shared single-file artifacts. They are not re-authored in this run. The signals (`rules.extra_time`, `rules.shootout_kick`, `rules.shootout_result`) are folded into steps 4 and 8, and the benchmark comparison into step 16. `class: implementation-detail`.
- **A-10** The master `04-plan.md` is not updated. It counts complete plans, and this plan is still awaiting input. `class: implementation-detail`.
- **A-11** Consult: the trigger `unknowns-present` holds (Q-1). The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`). Not fired. `class: implementation-detail`.
- **A-12** The discovery interview was not asked. This is an autonomous auto-review: implementation questions are settled here, and the one intent-bearing question stays open as Q-1. `class: implementation-detail`.
- **A-13** The two extra-time breaks give no energy back (`half_time(recover: false)`). The fatigue criterion requires no discontinuity at minute 90, and the half-time recovery would add one. The regulation half-time keeps its recovery, so the default match does not change. `class: implementation-detail`.
- **A-14** The extra-time allowance is computed in `substitute()` from the rule pack and the clock, not stored. The ledgers already live in the snapshot, so a resumed match in extra time gets the same limit with no new field. `class: implementation-detail`.
- **A-15** The fatigue criterion is tested in the existing `tests/fatigue.rs`, not in a new file, and the fatigue module is not changed. `drain()` and `multiplier()` read no clock value, so no 90-minute clamp exists to remove. `class: implementation-detail`.
- **A-16** The auto-review was done in this session and not by a separate review agent, because this run had no agent-dispatch tool. It re-read the slice definition, the sibling tactics plan's landed code, and the code named in each step, and it ran the four affected test files. `class: implementation-detail`.

## Blockers

**Q-1 (intent-bearing — core-loop mechanism and the match-length contract). How is the penalty shoot-out simulated?**

Why this is not decided autonomously: option A resolves kicks by probability draws. That is the event-probability model that RIM-1 set aside in favour of per-tick positional play, which the product owner ratified. It also decides what the user sees during a shoot-out. Options B and C keep the positional model, but they change the meaning or the size of the announced maximum match length, which the page uses to reserve history (`docs/reference/protocol.md`, `crates/protocol/src/lib.rs:30`).

- **Option A: draws, no ticks.** Each kick is a seeded draw from the taker's finishing and composure against the keeper's reflexes and one-on-ones. The shoot-out takes place after the last tick and appears only as events in the feed. *Consequence:* this is the smallest option, about 150 lines, and the announced maximum stays bounded at 480,000 ticks. The pitch stands still during the shoot-out, and this one phase is not positional.
- **Option B (recommended): on the pitch, with a bounded announcement.** Each kick is played through the existing penalty restart and shot mechanics, tick by tick. The announced maximum covers regulation, extra time, and a shoot-out allowance (for example 10 rounds, about 25,000 ticks). The page grows its history if sudden death runs past the allowance. *Consequence:* the model stays positional and the pitch shows the kicks. It costs about 300 more lines, and one page change grows history past the allowance.
- **Option C: on the pitch, open length.** As B, but `ticks_expected` becomes "unknown" for a knockout match, and `PROTOCOL_VERSION` rises to 3. *Consequence:* the positional model is kept, and the contract is honest about sudden death. Every page consumer must handle a match of unknown length.

Recorded in `po-answers.md` (2026-09-22T22:23:31Z) as awaiting input. No answer was on file at this review. On an answer, re-run `/wf plan football-manager-match-engine extra-time-penalties` with the answer as feedback. That run fixes steps 2, 6, and 10 and sets `status: complete`.

## Freshness Research

- Source: the installed code, read in this review (`clock.rs`, `rules/mod.rs`, `data/rules.rs`, `fatigue.rs`, `tactics/change.rs`, `sim.rs`, `snapshot.rs`, `scenario.rs`, `event.rs`, `protocol/src/lib.rs`, `content/rules/default.json`). Why it matters: every step. Takeaway: the rule pack and snapshot versions moved (3 and 3), the tactics code exists and has no clock input to fatigue, and the half-time path adds energy.
- Source: IFAB Laws of the Game, Laws 3, 7, and 10 — **recalled, not re-read**. The first plan's fetch of theifab.com failed, and web search is excluded by the product owner. Takeaway: see R6. Verify re-reads the laws.
- No dependency is added or upgraded. `garde`, `serde`, and `rand_chacha` stay at the versions `04-plan-match-rules.md` researched.

## Recommended Next Stage

- **Awaiting input (this plan):** answer Q-1, then re-run `/wf plan football-manager-match-engine extra-time-penalties` with the answer.
- **Option A (after Q-1): Implement** → `/wf implement football-manager-match-engine extra-time-penalties`. The slice it depends on (`tactics-and-ai`) is complete, so no step now waits for another slice.
- **Option C (revisit slice):** `/wf slice football-manager-match-engine`, only if the product owner prefers to split the shoot-out into its own slice so extra time can ship before Q-1 is answered.
