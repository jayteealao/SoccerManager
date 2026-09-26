---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: defending-and-discipline
status: complete
stage-number: 3
created-at: "2026-09-23T22:03:35Z"
updated-at: "2026-09-23T22:03:35Z"
complexity: l
depends-on: [realism-bands-v2]
source: extension
source-ref: "user description"
extension-round: 1
tags: [engine, tactics, rules, realism]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  source: ""
  plan: 04-plan-defending-and-discipline.md
  implement: 05-implement-defending-and-discipline.md
---

# Slice: Defending and discipline

## Goal

A team defends as a team in every shipped formation and after a sending-off. A red card is rare and does not turn the match into a shoot-out. A booked player tackles more carefully.

## Why This Slice Exists

The "ten-man collapse" is a defending bug. A red card only exposes it. A controlled run of 120 seeds per case showed the pattern:
- **Striker sent off:** when one side lost its striker at kick-off, that side scored 11.4 goals a match from 112 shots, against 1.0 before.
- **11 against 11 in 4-3-3:** a side in 4-3-3 scored the same 11.4 against 4-4-2, with no card at all.

The cause is how the team defends:
- **No marking:** the engine has no goal-side cover. Every non-presser holds an anchor, and the two pressers chase the ball's current position (`decision.rs:29-61`).
- **An open centre:** the back line leaves a 16 m central channel (`team.rs:258-274`).
- **A striker in the gap:** a lone striker is moved to the centre (`team.rs:236-237`) and dribbles through that channel.
- **Why only 4-4-2 works:** it is the only calibrated shape, and in it each striker happens to stand in line with a centre-back.

Second yellows are about 15 times too common. Four causes combine:
- Fouls pile up on a few players.
- A foul played on for advantage has no cooldown, so the same defender can foul again on the next tick (`rules/mod.rs:259-263`).
- Held cards are all shown at the next stoppage.
- A booked player's foul chance ignores the booking.

Research: `research/sendoff.md` in the session scratch folder, summarised in this workflow's extension round 1. The product owner asked for both fixes (`po-answers.md`, extend round 1).

## Scope

- **In:**
  - **Defending:**
    - Goal-side cover on the most advanced central attacker.
    - Pressers that chase the intercept point, not the ball's current position.
    - Back-line lateral compactness that scales with the number of players left.
  - **The shape after a sending-off:**
    - No lone forward recentred into the gap.
    - `TeamPlan` rebuilt with one fewer presser.
    - The AI manager reacts. After a keeper's red card, it brings on the bench keeper when a substitution is left. Otherwise it drops to a 4-4-1 style shape.
  - **The acting keeper:**
    - An acting-keeper lookup replaces the fixed "slot 0 is the keeper" checks the research lists: decision, sim, the rules' penalty and shoot-out fallbacks, and restart own-end inference.
    - A parked keeper no longer decides which end a team defends.
  - **Discipline:**
    - A foul cooldown for each tackler, including after an advantage foul.
    - At most one held card per player per advantage spell, keeping the most severe.
    - A booked player fouls less, controlled by a tuned factor with bounds in `tuning.json`.
  - **Tests:** a regression test for each mechanism, including a scripted sending-off scene and a formation scene.
- **Out:**
  - Goals, shots and possession band tuning. `realism-tuning` owns that.
  - The keeper save model. `keeper-and-shots` owns that.
  - Any change to the tick record's 22 fixed slots. A sent-off player still parks beside the pitch.

## Acceptance Criteria

- **No advantage from a red card.** In 120 seeds for each case, one away player is sent off at kick-off (striker, centre-back or keeper) and cards are otherwise off. In every case, the reduced side's goals per match do not exceed the full side's. The full side's goals per match are at most 1.6 times the no-send-off baseline for the same seeds.
- **Every formation holds.** At 11 against 11, over 120 seeds, each shipped formation plays against 4-4-2. Neither side averages more than 4.0 goals per match in any pairing. The pairing bands are tightened in `realism-tuning`.
- **Discipline is realistic.** Over 200 matches, second yellows are at most 0.10 per match, and at most 25% of matches have a sending-off. No player is shown two cards on the same tick.
- **The acting keeper covers a red card.** When the keeper is sent off, the bench keeper comes on at the next stoppage if one is available and a substitution is left. Otherwise an outfield player becomes the acting keeper. In both cases saves, penalties and restarts use the acting keeper.

## Dependencies on Other Slices

- `realism-bands-v2`: the sending-off, yellow-card and 10-or-more-goals bands, and the formations suite that measures this slice.

## Risks

- **Every seeded result moves.** The new mechanisms consume random draws and change play. Pinned tests will move, including `full_match.rs`, `strength.rs`, `mentality.rs`, `team.rs`'s lone-striker assertion and the rules tests. Each moved expectation is re-derived and recorded, never loosened silently.
- **Goals fall out of band.** About 23% of goals at `5a235a4` come from red-card chaos, and `keeper_catch_chance` 0.86 was tuned on top of it. Goals will fall below the band until `realism-tuning`. This is accepted: this slice is judged on its criteria, not on the bands.
- **Speed.** A goal-side marking search adds work every tick. The benchmark tripwire of +10% CPU per tick applies.
