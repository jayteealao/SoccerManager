---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: keeper-and-shots
status: complete
stage-number: 3
created-at: "2026-09-23T22:03:35Z"
updated-at: "2026-09-25T02:57:17Z"
complexity: l
depends-on: [defending-and-discipline]
source: extension
source-ref: "user description"
extension-round: 1
tags: [engine, shots, set-pieces, realism]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  source: ""
  plan: 04-plan-keeper-and-shots.md
  implement: 05-implement-keeper-and-shots.md
---

# Slice: Keeper and shots

## Goal

Shots and saves behave like football. A shot can go wide or high, be blocked, be held, or be parried. Corners and goal kicks arise from those outcomes, not from a scripted count. Shots on target and xG mean what they say.

## Why This Slice Exists

Nothing a defending side does can put the ball over its own goal line, so corners are almost never awarded: 14 in 2,000 matches. Four mechanisms cause this:
- **The keeper always holds a save.** The save roll is a flat 0.86 that ignores the goal frame, and success is always a held catch (`sim.rs:955-977`).
- **Nobody blocks a shot.** Only a keeper can touch a fast ball, so outfield players never block or deflect.
- **Shots stay low.** A shot never rises above 0.32 m (`decision.rs:409`), so nothing goes over the bar.
- **The on-target count is wrong.** It counts any keeper catch, including shots that were heading wide (`sim.rs:1029-1033`).

The results:
- **On target:** 75% of shots count as on target, against a real 35%.
- **Goals per xG:** 1.54. Most shots come from 16–21 m, and a flat save rate lets long shots score about twice as often as in real play.
- **Goal kicks:** about 3 per match, against a real 16–17.
- **Penalties:** penalties in play use the same keeper roll, so they are probably saved far too often.

Research: `research/setpieces-shots.md` in the session scratch folder. The product owner asked for corners, shots, shots on target and xG to be fixed (`po-answers.md`, extend round 1).

## Scope

- **In:**
  - **The keeper:**
    - A save is attempted only for shots heading between the posts and under the bar.
    - The save probability falls with shot quality, from about 89% for the poorest chances to about 27% for the best (`01-engine-realism.md:396`).
    - A save can be held or parried. A parry leaves the defending side as the last touch.
  - **Blocks:** outfield defenders in the shot lane can block or deflect.
  - **The shot:**
    - A loft range that lets a shot rise over the bar.
    - Aim noise tuned so wide shots happen.
  - **Counting:**
    - On-target is counted from the trajectory when the shot is taken.
    - xG coefficients are refitted after the keeper model changes.
  - **Penalties and shoot-outs:**
    - Open-play penalties convert at a realistic rate.
    - The shoot-out constants stay consistent with the new keeper model.
  - **Tests:** a test for each outcome: wide, over, blocked, held, parried to a corner, and scored.
- **Out:**
  - Final goals-band tuning. `realism-tuning` owns that.
  - Pass tempo and restart timing. `tempo-and-restarts` owns those.
  - Headers from corners and corner routines beyond the existing restart.

## Acceptance Criteria

- **Set pieces arise naturally.** Over 200 matches, corners per team are at least 3.0 and goal kicks per match are at least 10. Every corner follows a defending touch (parry, block, deflection or clearance) over the goal line. No corner is created without a ball crossing the line.
- **Shots are counted honestly.** Over 1,000 matches, shots on target are 30–42% of shots, and goals per unit of xG are 0.85–1.15.
- **A wide shot stays wide.** Given a shot whose trajectory misses the frame, when it reaches the goal line, then it is not counted on target, the keeper does not attempt a save, and the restart is a goal kick.
- **Penalties convert realistically.** Over 500 scripted penalty scenes, 70–85% convert.

## Dependencies on Other Slices

- `defending-and-discipline`: the acting-keeper lookup. Saves must use the acting keeper, not slot 0.
- `realism-bands-v2`: the shots-on-target, goals-per-xG, corners and goal-kicks bands.

## Risks

- **Every seeded result moves.** The keeper and shot changes consume random draws. Pinned tests will move, including `rules_shootout.rs`, `rules_extra_time.rs`, `match_stats.rs` and the `decision.rs` unit tests. Each expectation is re-derived and recorded.
- **Goals out of band.** Goals per match will leave the band until `realism-tuning`. This is accepted by the product owner.
- **A new keeper and a new record field.** A parried ball is a new loose-ball state, and the viewer draws the ball wherever the engine puts it. The page does not change, but any new event type needs the protocol document and its test updated.
