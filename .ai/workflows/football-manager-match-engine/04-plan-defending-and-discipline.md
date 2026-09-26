---
schema: sdlc/v1
type: plan
slug: football-manager-match-engine
slice-slug: defending-and-discipline
status: complete
stage-number: 4
created-at: "2026-09-24T02:46:11Z"
updated-at: "2026-09-24T02:46:11Z"
metric-files-to-touch: 26
metric-step-count: 16
has-blockers: false
revision-count: 0
revisions: []
consult-runs: []
tags: [engine, tactics, rules, realism]
stack-source: confirmed
open-questions: []
refs:
  index: 00-index.md
  plan-index: 04-plan.md
  slice-def: 03-slice-defending-and-discipline.md
  siblings: [04-plan-realism-bands-v2.md, 04-plan-match-rules.md, 04-plan-tactics-and-ai.md, 04-plan-extra-time-penalties.md, 04-plan-scripting-runtime.md]
  benchmark: 05c-benchmark.md
  implement: 05-implement-defending-and-discipline.md
next-command: wf-implement
next-invocation: "/wf implement football-manager-match-engine defending-and-discipline"
---

# Plan: Defending and Discipline

## The Plan

The realism-bands-v2 baseline shows the defending bug in numbers. At `2089ed7`, seven of the nine pairings against 4-4-2 have a side that averages more than 4.0 goals per match (4-4-1-1 scores 11.60, 3-4-3 8.48, 5-4-1 7.62, 4-3-3 7.43). The send-off research reproduces the same cause with ten men: a lone striker is recentred into the 16 m gap between the centre-backs and dribbles through it, and nobody marks him (`team.rs:236-237`, `decision.rs:29-61`, `team.rs:258-274`). Second yellows come from repeated fouls on the next tick after an advantage foul (`rules/mod.rs:259-263`), from held cards stacking, and from a foul chance that ignores a booking.

The plan fixes the defending first and the card rate second, as RIM-7 requires. Three defending mechanisms go into `decision.rs` and `team.rs`: a goal-side cover on the most advanced central attacker, pressers that run at the intercept point, and a back line whose width shrinks with the players left in it. The acting keeper is a function of the team's state (`Team::keeper_slot()`), not a stored field. So every fixed "slot 0 is the keeper" check becomes one call, and the validator and the snapshot need no new team data. The keeper-less case needs no hand-built shape: the most advanced forward goes in goal, which gives the 4-4-1 style shape, and the computer manager then brings on its bench keeper for him. Discipline gets a per-player foul cooldown (the snapshot moves to version 5 to carry it), one held card per player per advantage spell, and a booked-player factor. Five new tuning values carry the numbers, and each has bounds.

The work is 16 steps over 26 files, and the benchmark is re-baselined at 1.4753 µs per tick. When it lands, `keeper-and-shots` inherits an acting keeper that saves, penalties and restarts already use. The top risk is the formations criterion. The worst pairing today is 11.6 goals per match for one side against a 4.0 limit. If the three mechanisms, tuned inside their bounds, cannot bring a pairing under 4.0, implement stops and reports that pairing. It does not raise the limit.

## Current State

- Code: HEAD `2089ed7`. `git status` shows no change under `crates`, `content` or `web`. `cargo test -p engine --all-features` at this tree gave 241 passed, 0 failed and 3 ignored (`ai_trailing`, `mentality` and `strength` are the slow tests).
- **Pressing and marking** (`decision.rs:25-98`): with a carrier, every active player targets its anchor. The `press_count` (at most `MAX_PRESSERS` 4, `:489`) nearest opponents within `press_distance`, outfield players only (`slot != 0`, `:44`), target the ball's current position (`:59`). With a loose ball, each team's nearest outfield player chases a point 0.3 s ahead (`:66-87`). The keeper is `team * PLAYERS_PER_TEAM` (`:89-93`). Nothing marks an attacker.
- **Shape** (`team.rs`): `anchor()` (`:258-274`) returns the keeper anchor for `slot == 0`. For every other slot, it returns the formation slot moved by the block depth, the duty and the plan's width, plus `compactness_x`/`compactness_y` (0.3) of the ball offset. `reshape()` (`:220-242`) groups a line by equal depth and spreads the survivors across the line's full width. A lone survivor goes to `(lo + hi) / 2` (`:236-237`). `relayout()` (`:194-200`) rebuilds the layout from `base_formation` around every inactive slot. `set_tactics()` rebuilds the `TeamPlan` (`:186-191`). `reshape()` does not rebuild it, so `press_count` ignores the numbers left (`tactics/mod.rs:198`).
- **Fixed keeper checks.** In play: `decision.rs:44, 80, 89, 115, 145, 270`, `sim.rs:953` (keeper reach and catch) and `sim.rs:1031` (shots on target). In the rules: `rules/mod.rs:567` (the penalty keeper is `(1 - team) * 11`), `:740` and `:752` (shoot-out candidates and the keeper fallback), and `restart.rs:76-104` (the taker). There, `first = team * 11`, the own end is read from `players[first].pos.x`, the goal kick goes to `first`, and slot 0 is excluded. Also `restart.rs:237` and `:336` (the penalty keeper's place and judgement). A sent-off player parks at `pitch::parking_spot(team, slot)` (`pitch.rs:169-172`), whose side is fixed per team. So after half-time, a parked keeper puts the own end on the wrong side.
- **Send-off** (`rules/discipline.rs:22-29`): parks the player and calls `reshape`. `show_card` clears the carrier and pushes the team timeline (`rules/mod.rs:308-354`). `Scene::sent_off` (`scenario.rs:89`) does neither.
- **Computer manager** (`ai.rs:193-302`): it reacts to injuries, fatigue and the score only. `substitute()` (`tactics/change.rs:389-455`) puts the incoming player into the leaving player's slot and refuses a sent-off player (`:419-421`). It calls `restore` only for an inactive slot.
- **Fouls** (`sim.rs:984-1011`, `rules/fouls.rs:62-66`, `rules/mod.rs:226-285`): once the carrier's `control_cooldown_ticks` (25) has passed, every opponent within `reach_radius` draws once per tick. `foul_chance` reads aggression and tackling only. On an advantage foul, `foul()` pushes a `PendingCard` and returns without touching the tackler. `show_pending_cards` (`:429-433`) shows every held card at the next stoppage. A stopping foul shows its own card first and then the held cards (`:264-266`, then `open_dead_ball` at `:446`), so one player can receive two cards on one tick.
- **Tuning** (`tuning.rs`, `deny_unknown_fields` at `:25`, garde bounds per field; `TUNING_VERSION = 2` in `data/tuning.rs:17`): `foul_base` 0.1, `yellow_base` 0.05, `yellow_aggression_weight` 0.15, `red_base` 0.005, `compactness_y` 0.3 (`content/tuning.json:35-46`).
- **Snapshot** (`snapshot.rs:45`, `VERSION = 4`): it writes each player's fields one by one (yellow at `:630`, read at `:864`), and each team's per-slot `active` flag and formation place (`:813-823`). The reader refuses a snapshot from another build, so a snapshot never crosses builds (NFR-4; plan answer "does NOT promise cross-machine or cross-build resume").
- **Measured cause** (research at `5a235a4`, 120 seeds, cards otherwise off): with the away striker sent off, the away side scored 11.41 goals per match against 1.05 in the control. With the away keeper sent off, the home side scored 2.71 against 0.66. With 4-3-3 at 11 against 11 and no card, the away side scored 11.38. In an 80-match CLI sample with cards on, 44 matches had a red card, and 12 of 44 second yellows came on the same tick as the first yellow.
- **Formations baseline** (`implement-evidence/realism-bands-v2/baseline-s42.report.json`, 1,000 matches per pairing, cards on; goals for the first formation, then for the second): 4-4-2 v 4-4-2 1.46/1.52; v 4-3-3 1.16/7.43; v 4-2-3-1 2.92/4.78; v 3-5-2 4.59/1.77; v 4-1-4-1 0.93/5.18; v 4-4-1-1 1.96/11.60; v 4-1-2-1-2 0.70/0.91; v 3-4-3 1.67/8.48; v 5-3-2 2.68/0.78; v 5-4-1 2.55/7.62.
- **Test seams**: the `scenario` feature (`Cargo.toml:24-27`) builds `Scene` with `sent_off`, `rolls`, `yellow`, `carrier`, `subs_used`, `manager` and `queue`. `tests/common/mod.rs:135` `run_many` runs seeds on every core. Slow tests are `#[ignore = "slow: cargo test --release -p engine -- --ignored"]` (`strength.rs:14`).

## Simplicity Ladder

- Acting-keeper lookup → rung 3 reuse — `team.rs` → `Team` already holds `active`, `lineup`, `squad[].position` and `base_formation`. A method derives the keeper from them, which is a close match; nothing is stored. Reuse with a new method.
- Lone forward kept in place and the back line narrowed → rung 3 reuse with modification — `team.rs` → `reshape()`. It changes two branches and stays backward-compatible for lines of two or more inside the gap.
- One fewer presser → rung 3 reuse — `TeamPlan.press_count` (`tactics/mod.rs:165`), read through a new `Team::pressers()` that subtracts the players out of play.
- Intercept point → rung 4 new code — std `f64` only. No in-repo intercept solver exists; the loose-ball chase at `decision.rs:66-69` is a fixed 0.3 s lead, not a pursuit solve. Rungs 1 to 3 give nothing closer.
- Goal-side cover → rung 4 new code — std only. No in-repo marking logic exists (the research's R2 finding). The target point reuses `math::toward` (`decision.rs:10`).
- Foul cooldown → rung 3 reuse — the `control_since` cooldown pattern at `sim.rs:981` is the model. Implement fresh per player, because `control_since` is per carrier, not per tackler.
- One held card per spell → rung 3 reuse with modification — `Referee.pending: Vec<PendingCard>` (`rules/mod.rs:58-61, 98`), with an in-place replace by player.
- Booked-player factor → rung 3 reuse with modification — `fouls::foul_chance` (`rules/fouls.rs:62-66`).
- Tuning values with bounds → rung 3 reuse — garde `range` on `Tuning` fields (`tuning.rs:85-135`), plus serde `default` so an older file still loads.
- Keeper reaction → rung 3 reuse — `ai_queue`, `substitute_for`-style bench search, `substitution_possible` (`ai.rs:314-373`).
- Sending-off and formations experiments → rung 3 reuse — the research harness pattern (`Scene::sent_off` plus `run_many`) and the formation setting through `Tactics::set_formation` + `MatchConfig::with_tactics` (`sim.rs:159-166`, `tests/mentality.rs:21-26`).

## Applied Learnings

No applicable learnings found. `.ai/solutions/INDEX.md` does not exist, and `.ai/sdlc-config.json` sets no global directory.

Repeat-deferral tripwire: `00-index.md` holds two deferrals, the human legibility reading (viewer-match-day) and the macOS build (distribution). This slice's verification is headless engine runs with `cargo test`. It names neither wall, so the tripwire does not fire.

## Likely Files / Areas to Touch

- `crates/engine/src/team.rs`: `keeper_slot()`, `pressers()`, the lone-survivor and back-line-gap rules in `reshape()`, the acting keeper in `relayout()` and `anchor()`, and unit tests.
- `crates/engine/src/decision.rs`: the intercept point, the goal-side cover, and the acting keeper in place of slot 0.
- `crates/engine/src/sim.rs`: `keeper(team)`, the acting keeper in `resolve_possession` and `gain`, and the tackler cooldown in the tackle loop.
- `crates/engine/src/player.rs`: `foul_ready`.
- `crates/engine/src/rules/mod.rs`: the cooldown on every foul, card merging, and the penalty and shoot-out keepers.
- `crates/engine/src/rules/fouls.rs`: the booked factor in `foul_chance`, and `Card::severity`.
- `crates/engine/src/rules/restart.rs`: the own end from the attack direction, and the acting keeper for goal kicks, dropped balls, kick-offs and penalties.
- `crates/engine/src/rules/discipline.rs`: `send_off` lays the team out again and asks the computer manager to check.
- `crates/engine/src/ai.rs`: `AiCode::SubKeeper` and the keeper reaction.
- `crates/engine/src/tactics/change.rs`: `relayout` after every substitution.
- `crates/engine/src/commentary/templates.rs` and `content/commentary/en.json`: the `sub-keeper` decision and its lines.
- `crates/engine/src/tuning.rs` and `content/tuning.json`: five bounded values.
- `crates/engine/src/snapshot.rs`: version 5 with `foul_ready`.
- `crates/engine/src/scenario.rs`: `sent_off` made faithful to a shown card; `foul_ready`.
- `crates/engine/tests/defending.rs`, `discipline.rs`, `acting_keeper.rs` (new), plus `rules_fouls.rs`, `rules_cards.rs`, `commentary.rs`, `snapshot.rs` and `content.rs`. Pinned tests that may move and are re-derived: `full_match.rs`, `strength.rs`, `mentality.rs` and `ai_trailing.rs`.
- `docs/reference/data-files.md` and `docs/reference/protocol.md`: the tuning rows and the `sub-keeper` value.

## Proposed Change Strategy

- **Defending before discipline.** Steps 2 to 9 change the shape and the defending. Steps 10 to 12 then change the discipline. The sending-off experiment runs with cards otherwise off, so it judges the defending alone (RIM-7).
- **The acting keeper is derived, not stored.** `Team::keeper_slot()` returns one of three slots, checked in this order:
  1. slot 0 while it is active;
  2. otherwise the lowest active slot whose squad player is a goalkeeper by position (a keeper who came on);
  3. otherwise the most advanced active outfield slot by base-formation depth, then the smallest `|y|`, then the lowest slot.
  `Simulation::keeper(team)` maps the slot to a player index. Every fixed check listed in Current State calls it. The snapshot already stores `active`, the lineups and the formation places, so a resumed match derives the same keeper. The validator reads `Team::anchor`, so it follows the keeper without a change.
- **The 4-4-1 style shape comes from the keeper choice.** A forward that becomes the acting keeper leaves the forward line, and the rest of the shape stays. When a substitution is left, the computer manager then queues its best bench goalkeeper on for the acting keeper at the next stoppage. The keeper takes over the goal from the forward's slot, and the forward line stays one short. No formation is rewritten and no team flag is added.
- **Shape rules in `reshape()`.** A lone survivor keeps its own `y`. The deepest outfield line's width is capped at `(n - 1) × back_line_gap`, centred on the line, so it narrows as players leave. `Team::pressers()` gives `press_count` minus the players out of play, and never less than 1.
- **Pressers run at the intercept point.** For a presser at `p` with top speed `s`, and a carrier at `c` moving at `v`, the target is `c + v·t`, where `t` is the smallest positive root of `|c + v·t − p| = s·t`. `t` is capped at a 1.0 s horizon. With no root, the target is the carrier.
- **Goal-side cover.** Cover applies while the opponents hold the ball. Among the attackers in the defending team's half and inside the central channel (`|y| ≤ cover_channel`), the most advanced one is covered by the nearest active back-line player. That player is not a presser and not the keeper. Its target is the attacker's position moved `cover_distance` toward the centre of its own goal.
- **Discipline.**
  - Every foul, advantage included, sets the tackler's `foul_ready` to `tick + foul_cooldown_ticks`. Before that tick, the tackler makes no tackle attempt and takes no draw.
  - A held card for a player who already has one replaces it only when it is more severe (`Card::severity`).
  - A stopping foul's card merges with that player's held card, so one card is shown per player per tick.
  - A booked player's foul chance is multiplied by `foul_booked_factor`.
  - The card chances (`yellow_*`, `red_base`) do not change in this slice.
- **Tuning.** Five fields are added with bounds, and each has a serde default equal to the shipped value. An older tuning file still loads, and `TUNING_VERSION` stays 2. Implement tunes the five values inside their bounds, using the slow criterion tests as the loop.
- **No NFR is the rationale for a mechanism choice.** The benchmark tripwire (+10 percent processor time per tick) is the slice's own risk line, and the 2000 ms budget (NFR-1, `yields-to: C2`) is not in tension.

## Step-by-Step Plan

1. **Tuning values.** Add `cover_distance` (m, 0 to 10), `cover_channel` (m, 0 to 34), `back_line_gap` (m, 4 to 30), `foul_cooldown_ticks` (0 to 1000) and `foul_booked_factor` (0 to 1) to `Tuning`. Give each garde bounds and `#[serde(default = …)]`, and mirror them in `Default`. Write starting values into `content/tuning.json`: 2.0, 12.0, 12.0, 150 and 0.5. Pin them in `tests/content.rs`, and add a test that a tuning file without them loads with the defaults. Add the five rows to `docs/reference/data-files.md`.
2. **Acting keeper.** In `team.rs`, add `keeper_slot()` as described in the strategy. Make `relayout()` take the acting keeper's slot out of its line, the same way it takes out an inactive slot. Make `anchor()` return the keeper anchor for `keeper_slot()`. Add `Simulation::keeper(team) -> usize`. Unit tests cover:
   - slot 0 active;
   - a goalkeeper who came on in slot 9;
   - an outfield fallback in a 4-4-2 (a striker) and in a 3-5-2;
   - the fallback staying the same player as other outfield players leave.
3. **Shape after a sending-off.** In `reshape()`, a lone survivor keeps its `y`. The deepest outfield line gets the `back_line_gap` cap in `relayout()`, so the cap also holds at 11 against 11 (a 4-4-2 back four at 12 m becomes −18, −6, 6, 18). `Team::new` calls `relayout()` so a bare team follows the same rule. Add `Team::pressers()`. `discipline::send_off` calls `relayout()` and sets `ai[team].due` for a computer-managed team (through the caller in `show_card`). Re-derive `team.rs` `a_back_four_with_one_sent_off_spreads_three_across_the_width`: the back line becomes −12, 0, 12 with the starting gap, and `formation[10].1` stays 8.0. Re-derive `restoring_a_slot_puts_its_line_back_and_keeps_the_others_closed`, which compares against the uncapped `FORMATION_442`. Re-derive `rules_cards.rs` `a_second_yellow_sends_the_player_off_and_the_line_spreads`. Record both values.
4. **Keeper checks in play.** Replace the fixed checks at `decision.rs:44, 80, 89-93, 115, 145, 270`, `sim.rs:953, 1031` and the test filter at `decision.rs:593` with `keeper(team)` or `keeper_slot()`. Use `Team::pressers()` at `decision.rs:34`.
5. **Keeper checks in the rules.**
   - Penalty keeper: `rules/mod.rs:567`.
   - Shoot-out candidates and the keeper fallback: `:740, :752`.
   - `restart::taker` takes `own_end: f64` from `-teams[team].attack_x`, and every caller passes it (`rules/mod.rs`, `tactics/change.rs:319`). The goal kick and the dropped ball in the own area prefer the acting keeper. The kick-off preference skips the acting keeper. The outfield filter excludes the acting keeper.
   - `restart.rs:237, :336`: the penalty keeper's place and judgement use the acting keeper.
6. **Substitutions and scenes.** `substitute()` calls `relayout()` after every applied substitution, not only for an inactive slot. `Scene::sent_off` clears the carrier and lays the team out again, as `show_card` does. Add `Scene::foul_ready(i, tick)`.
7. **The computer manager's keeper reaction.** In `ai_check`, add a first branch. When `keeper_slot()` is not a goalkeeper by position, a bench goalkeeper is free, no keeper substitution is queued and `substitution_possible(team)` holds, queue `Substitution { off: the acting keeper's squad index, on: the best free bench goalkeeper }` with `AiCode::SubKeeper` (code `sub-keeper`). Add `DecisionCondition::SubKeeper` in `commentary/templates.rs`, three lines in `content/commentary/en.json`, the code in `tests/commentary.rs`, and the value in `docs/reference/protocol.md` and `docs/reference/data-files.md`.
8. **Pressers at the intercept point.** In `decide()`, set each chosen presser's target from the intercept solve in the strategy. The top speed is the player's derived top speed. Keep the horizon as a named constant with a comment.
9. **Goal-side cover.** In `decide()`, after the pressers are chosen, pick the covered attacker and the covering defender as the strategy describes. Set that defender's target. Use one pass over the opponents and one pass over the back line.
10. **Foul cooldown.**
    - Add `foul_ready: u32` to `Player`, set to 0 in `Team::player`.
    - In the tackle loop (`sim.rs:984`), skip a tackler with `tick < foul_ready`, before the draw.
    - In `foul()`, set `foul_ready = tick + foul_cooldown_ticks` on every foul.
    - Move the snapshot to `VERSION = 5`: write and read `foul_ready` after `yellow`, and name version 5 in the header comment.
    - Extend `tests/snapshot.rs` so a resumed match crosses a cooldown.
11. **Held cards.** Add `Card::severity()`. In `foul()`, an advantage card replaces a held card for the same player only when it is more severe. A stopping foul takes that player's held card out of `pending` and shows one card, the more severe of the two. `show_pending_cards` is unchanged. A held caution for an already-booked player is still shown as a second yellow (`rules/mod.rs:315-319`).
12. **Booked-player factor.** Change `foul_chance(tackler, yellows, t)` to multiply by `foul_booked_factor` when `yellows >= 1`. Update the caller at `sim.rs:996` and `tests/rules_fouls.rs:26`. Add a unit test: a booked player's chance is exactly the factor times the unbooked chance.
13. **Regression scenes (fast, not ignored).**
    - `tests/defending.rs`:
      - a central attacker in the defending half draws a back-line defender goal-side;
      - a moving carrier draws a presser ahead of it;
      - after a centre-back is sent off, the back line is no wider than `2 × back_line_gap`;
      - with one player out, there is one fewer presser;
      - a 4-3-3 at 11 against 11 has no back-line gap wider than `back_line_gap` while its opponent's lone striker is central.
    - `tests/discipline.rs`:
      - two foul draws for one tackler on consecutive ticks after an advantage foul give one foul;
      - two advantage fouls by one player hold one card, the more severe;
      - a stopping foul by a player with a held card shows one card.
    - `tests/acting_keeper.rs`:
      - keeper sent off with a substitution left: the bench keeper comes on at the next stoppage, `keeper(team)` is that player, and one `sub-keeper` decision is recorded;
      - keeper sent off with no substitution left: an outfield acting keeper, and the forward line is one short;
      - a fast shot at the acting keeper is caught with the catch chance;
      - a penalty and a shoot-out kick face the acting keeper;
      - a goal kick in the second half with the keeper parked is taken at the defending end.
14. **Criterion tests (slow, ignored) and the tuning loop.**
    - `tests/defending.rs` `a_sending_off_gives_no_advantage` covers seeds 1 to 120, with the cards set off through tuning (`red_base`, `yellow_base` and `yellow_aggression_weight` at 0). It plays a control and three arms with the away player sent off at kick-off: keeper (index 11), centre-back (13) and striker (21). For each arm it asserts that the reduced side's mean goals are at most the full side's, and that the full side's mean is at most 1.6 × the control's home mean. It prints every figure.
    - `tests/defending.rs` `every_formation_holds` plays seeds 1 to 120 for each of the 10 shipped formations against 4-4-2. The formation is at home on odd seeds and away on even seeds, with cards on. It asserts that neither side's mean exceeds 4.0.
    - `tests/discipline.rs` `discipline_is_realistic` plays seeds 1 to 200 with default content. It counts second-yellow card events per match, the share of matches with a sending-off, and any player with two card events on one tick, and asserts at most 0.10, at most 25% and none.
    - Run all three with `cargo test --release -p engine --all-features -- --ignored`. Tune only the five new values inside their bounds, and record each value with the run that chose it. If a criterion still fails with those values at their bounds, stop and report the failing arm or pairing. Do not change the limits, the card chances or the goal tuning.
15. **Re-derive the moved expectations.** Run `cargo test -p engine --all-features` and `cargo test --release -p engine --all-features -- --ignored`. For every assertion that moves (expected: `full_match.rs`, `strength.rs`, `mentality.rs`, `ai_trailing.rs`, the rules tests), record the value before and after in the implement record, and give the reason the new value is right. Never widen a tolerance to make a test pass.
16. **Close.**
    1. Run `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo test --workspace --all-features`.
    2. Run the benchmark compare from `05c-benchmark.md` (gate 1.6228 µs per tick and 8.27 MB).
    3. Run one `engine-cli calibrate --suite equal --seed 42` and record the band misses against the realism-bands-v2 baseline (Q-E4: recorded, not failed).
    4. Search the changed source, comments and commit text for workflow vocabulary before the commit.

## Verification Strategy

The four criteria are about match outcomes. The engine produces them headless, so no screen is needed. The rung for each criterion is headless runs of the real engine library over many seeded matches (`cargo test`, in `stack.testing`). A unit test or static reasoning never counts.

| AC | Tool / method + ladder rung | Environment need — satisfiable in target env? | What must be BUILT to make it verifiable | Fallback chain |
|----|------------------------------|-----------------------------------------------|------------------------------------------|----------------|
| No advantage from a red card | `cargo test --release -p engine --all-features -- --ignored a_sending_off_gives_no_advantage` (headless engine runs, 480 matches) | Rust 1.92 and cargo on the reference machine — yes | `tests/defending.rs` slow test; `Scene::sent_off` made faithful (step 6); tuning-driven cards-off config | the same harness at 60 seeds for a timing check → pre-registered deferral (not expected: no wall) |
| Every formation holds | `cargo test … -- --ignored every_formation_holds` (1,200 matches); cross-check with the `goals_for_mean` of the `calibrate --suite formations` pairings with 4-4-2 when verify runs it | yes | `tests/defending.rs` slow test, formation set through `with_tactics` | `engine-cli calibrate --suite formations --seed 42` pairings with 4-4-2 → pre-registered deferral |
| Discipline is realistic | `cargo test … -- --ignored discipline_is_realistic` (200 matches, events read) | yes | `tests/discipline.rs` slow test | `engine-cli calibrate --suite equal` sending-off share and card events → pre-registered deferral |
| The acting keeper covers a red card | `cargo test -p engine --all-features --test acting_keeper` (scripted scenes on the real engine) plus the keeper arm of the sending-off test | yes | `tests/acting_keeper.rs`; `Scene::foul_ready`; `Simulation::keeper` | a traced single match (`engine-cli simulate` events) showing the `sub-keeper` decision and the substitution → pre-registered deferral |

No criterion depends on credentials, a device, an external service or missing infrastructure. Every run is local and uses tools already in `stack:`, so no `constraint-resolution:` line is needed. No wall is named.

## Test / Verification Plan

### Automated checks

- Lint and type check: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Unit: `team.rs` (acting keeper, lone survivor, back-line gap, pressers), `rules/fouls.rs` (booked factor, severity), `restart.rs` (own end after half-time with a parked keeper).
- Integration (fast): `tests/defending.rs`, `tests/discipline.rs` and `tests/acting_keeper.rs`, plus the updated `rules_fouls.rs`, `rules_cards.rs`, `commentary.rs`, `snapshot.rs`, `content.rs` and `validator.rs` (the full-match validator must still read 0 violations).
- Criterion runs (slow, ignored, release): `a_sending_off_gives_no_advantage`, `every_formation_holds`, `discipline_is_realistic`, and the existing `strength`, `mentality` and `ai_trailing`.
- Benchmark compare against `05c-benchmark.md`.

### Interactive verification (human-in-the-loop)

Automated only. Every criterion is a statistic of engine runs or a scripted engine scene. The page shows no new element: the `sub-keeper` decision is an `ai-decision` event, which the viewer hides (`web/match-state.mjs:32`), and a substitution uses the existing substitution event.

## Risks / Watchouts

- **The formations criterion starts far away** (high). The worst side against 4-4-2 averages 11.60 goals per match (4-4-1-1), and seven of nine pairings fail today. The three defending mechanisms are the only allowed levers. If a pairing still fails at the tuning bounds, implement stops and reports it (step 14).
- **Every seeded result moves** (medium). Step 15 re-derives each moved expectation and records it.
- **Goals fall below the band** (medium). This is accepted (Q-E4). The band misses are recorded against the realism-bands-v2 baseline in step 16.
- **Per-tick cost** (medium). The cover search is one pass over the opponents, and the intercept solve is one quadratic per presser. The gate is 1.6228 µs per tick.
- **Validator drift rule** (low). Cover applies only while the opponents hold the ball and the attacker is in the defending half. The validator's closing-distance exemption covers a defender moving toward its target.
- **The script decision hook still reports the formation slot** (low). `DecisionContext.slot` documents "0 for the goalkeeper" (`plugin.rs:86-87`). An outfield acting keeper reports its own slot. The public scripting interface is not changed in this slice.
- **The acting keeper can change on a formation change** (low). The fallback reads the base formation. If the manager changes formation while an outfield player keeps goal, the most advanced slot can move to another player. Step 2's test pins the stable case (players leaving). A formation change moving the keeper is recorded as known behaviour for `keeper-and-shots`.

## Dependencies on Other Slices

- `realism-bands-v2` (complete, verified): the sending-off, yellow-card and 10-or-more-goals bands, the formations suite, and the seed-42 baseline this plan cites.
- `keeper-and-shots` (next): it owns the save model and inherits `Simulation::keeper(team)`.
- `realism-tuning` (last): it owns goals, shots and possession tuning, and tightens the pairing bands.
- Files that earlier verified slices created: `match-rules` (the rules, discipline, restart and snapshot files), `tactics-and-ai` (`ai.rs`, `tactics/change.rs`), `extra-time-penalties` (the shoot-out code in `rules/mod.rs`) and `commentary` (templates and `en.json`). All are verified, so the changes run in sequence.

## Assumptions

Autonomous run: no product owner was present. Each discovery question the interview would have asked is answered here in the direction that meets the criteria at the least cost. None changes the scope, a public contract or a product owner answer.

- A1 (class: implementation-detail): the acting keeper is derived by `Team::keeper_slot()` from `active`, `lineup`, the squad positions and the base formation. No team field is added. The snapshot and the validator already carry those inputs, so there is no new persisted team data.
- A2 (class: implementation-detail): the outfield acting keeper is the most advanced active outfield player. The slice says "an outfield player becomes the acting keeper" and does not say which. This choice keeps the back four and the midfield, which is the 4-4-1 style shape the slice asks the computer manager to reach, and it adds no formation rewrite. The keeper save model uses flat tuning values, not the keeper's attributes (`sim.rs:953-975`), so the choice does not weaken saves. The same rule applies to a human-managed team, because every team needs an acting keeper under the fourth criterion.
- A3 (class: implementation-detail): the computer manager's keeper reaction takes the acting keeper off and puts the best free bench goalkeeper on, at the next admitting stoppage. It uses a new decision code, `sub-keeper`. `ai.decision` is a string field (`docs/reference/protocol.md:181`), and the page hides `ai-decision` events (`web/match-state.mjs:32`). The value is added to both reference lists and gets three commentary lines, the same way the four existing codes did.
- A4 (class: implementation-detail): goal-side cover applies while the opponents hold the ball. It covers the most advanced attacker in the defending half inside `|y| ≤ cover_channel`, using the nearest non-pressing back-line player, at `cover_distance` toward the centre of the own goal.
- A5 (class: implementation-detail): the intercept point is the closed-form pursuit solution with a 1.0 s horizon kept as a code constant. It is not a tuning value, because the slice names no tuned value for it.
- A6 (class: implementation-detail): a lone survivor keeps its `y` in every line. The deepest outfield line's width is capped at `(n - 1) × back_line_gap` in every layout, 11 against 11 included, because the slice's scope lists the back-line compactness under defending as a whole. This is how "compactness that scales with the number of players left" is built.
- A7 (class: implementation-detail): `Team::pressers()` subtracts the players out of play from `press_count`, with at least 1. It is derived, not stored, so a snapshot and a tactics change stay consistent.
- A8 (class: implementation-detail): the foul cooldown is a per-player tick (`foul_ready`). During the cooldown, the player makes no tackle attempt and takes no draw. The snapshot moves to version 5 to carry it. Snapshots are refused across builds (NFR-4), and the tactics-and-ai and extra-time plans raised the version the same way.
- A9 (class: implementation-detail): one held card per player per advantage spell, the most severe kept. A stopping foul merges with the held card, so one player is never shown two cards on one tick.
- A10 (class: implementation-detail): the booked factor multiplies the foul chance when `yellow >= 1`. The card chances per foul stay unchanged in this slice. RIM-7 says the defending fix comes first and the card rate is fixed separately, so the plan does not lower card chances to meet the discipline criterion. A miss stops implement instead.
- A11 (class: implementation-detail): the five tuning fields are optional with serde defaults, and `TUNING_VERSION` stays 2. A modder's older tuning file still loads. The slice asks for "a tuned factor with bounds in tuning.json", and the bounds are garde ranges like every other field.
- A12 (class: implementation-detail; ac: "Every formation holds"; classification: runtime-evidence): "each shipped formation" is read as the 10 formations in `content/tactics.json` today, the 4-4-2 mirror included. This is a superset of the four named in RIM-10. The formation side alternates home and away by seed parity.
- A13 (class: implementation-detail; ac: "No advantage from a red card"; classification: runtime-evidence): the experiment follows the research harness. Seeds 1 to 120, default clubs, both sides computer-managed, cards off through the three tuning values set to 0. The away keeper (11), centre-back (13) and striker (21) are sent off at kick-off. The no-send-off control on the same seeds gives the full side's baseline.
- A14 (class: implementation-detail; ac: "Discipline is realistic"; classification: runtime-evidence): seeds 1 to 200 with default content. Second yellows and same-tick cards are counted from card events. The sending-off share comes from the summary's red count.
- A15 (class: implementation-detail; ac: "The acting keeper covers a red card"; classification: runtime-evidence): scripted scenes on the real engine prove each branch and each use (saves, a penalty, a shoot-out, restarts). The keeper arm of the sending-off experiment proves it over many matches.
- A16 (class: implementation-detail): the script decision hook's `slot` field is not changed. A new keeper flag would change the public scripting interface, which is not in this slice's scope.
- A17 (class: implementation-detail): augmentations. The benchmark is re-baselined, because this slice changes per-tick code (`05c-benchmark.md` rev 6, 1.4753 µs per tick). `04b-instrument.md` is not re-authored: the keeper substitution goes through the change queue, whose counters and `darkpath.change_never_applied` already cover it, and no new dark path is added. `04c-experiment.md` is not involved, because no flag is added.
- A18 (class: implementation-detail): the second-opinion consult is not fired, although `appetite-medium-or-larger` holds. The product owner excluded `consult` at intake (`00-index.md` `stack.excluded-by-po`), as in earlier plans.
- A19 (class: implementation-detail): the tuning loop in step 14 may change only the five new values. It may not change the limits, the card chances, `keeper_catch_chance` or any band. A criterion that cannot be met stops implement with the figures.

## Blockers

None.

## Freshness Research

- No dependency is added or upgraded. The slice uses only std `f64` arithmetic and the crates already in the workspace (`garde` for bounds, `serde` for defaults).
- `serde`'s field-level `#[serde(default = "path")]` works together with `#[serde(deny_unknown_fields)]` on the container. `deny_unknown_fields` refuses unknown keys, and `default` fills missing ones. `04-plan-data-schemas-generator.md` § Freshness Research records the `serde` 1.0 and `garde` 0.23 behaviour this relies on. The `flatten` caveat recorded there does not apply, because `Tuning` has no flattened field.
- IFAB Law 12 lets the referee delay a caution while playing advantage and show it at the next stoppage. A player cautioned twice is sent off. "At most one held card per player per advantage spell" matches the law's single-caution-per-incident practice. `04-plan-match-rules.md` § Freshness Research carries the IFAB Laws 3 and 12 reads.

## Recommended Next Stage

- **Option A (default): Implement** → `/wf implement football-manager-match-engine defending-and-discipline`. The plan has no blocker, and the benchmark is re-baselined. Compact the session first. Workflow state lives in the artifact files, and the SessionStart hook re-reads it after compaction.
- **Option C: Revisit slice** → `/wf slice football-manager-match-engine`. Take this only if implement shows that the formations criterion cannot be met with the three defending mechanisms (step 14 stop).
