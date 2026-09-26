---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: tempo-and-restarts
status: defined
stage-number: 3
created-at: "2026-09-23T22:03:35Z"
updated-at: "2026-09-23T22:03:35Z"
complexity: m
depends-on: [keeper-and-shots]
source: extension
source-ref: "user description"
extension-round: 1
tags: [engine, passing, restarts, realism]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  source: ""
  plan: 04-plan-tempo-and-restarts.md
  implement: 05-implement-tempo-and-restarts.md
---

# Slice: Tempo and restarts

## Goal

A match has a real rhythm. Players keep the ball long enough to carry it. Restarts take as long as they do in real matches, so the ball is in play for about an hour of a 90-minute match. Pass counts include only real passes. Throw-ins stay realistic.

## Why This Slice Exists

Teams make about 1,300 passes each, against a real 429. Three causes combine, and together they come to about 2.8 times the real volume:
- **Fast release:** every player decides every tick with fresh noise. A carrier passes about 0.2–0.3 s after receiving the ball, because the dribble bonus lasts only 10 ticks and holding loses 0.5 per second.
- **Long live time:** restart delays of 3–8 s, against real medians of 13.8–32.5 s, leave the ball live for about 89 minutes instead of 55–60.
- **Loose counting:** clearances and restart kicks are counted as passes.

Pass accuracy, at 84.7%, is already realistic. Throw-ins measured 33–43 per match in research runs, against a real 42–46. Fewer passes could lower them, so this slice holds them in band.

Research: `research/setpieces-shots.md` in the session scratch folder. The product owner asked about pass volume, pass completion and throw-ins (`po-answers.md`, extend round 1).

## Scope

- **In:**
  - **Possession tempo:** the carrier's decision persists over a possession, or has a minimum carry time. The decision weights for hold and dribble are retuned so a carrier releases the ball at a realistic rate.
  - **Restart delays:** `restart_delay_s` moves toward the real medians: throw-in 13.8 s, goal kick 23.2 s, corner 31.8 s and free kick 32.5 s (`01-engine-realism.md:614`). Players still walk to their restart positions.
  - **A ball-in-play figure** in `match-stats`.
  - **Pass counting:** `stats.passes` counts only open-play passes. Clearances and restart kicks are counted under their own keys, and the observability contract and schema are updated.
  - **Throw-ins:** a check that throw-ins stay 35–55 per match.
- **Out:**
  - Final goals-band tuning. `realism-tuning` owns that.
  - Any change to the tick rate or to "every agent decides every tick". The product owner chose that at engine-core Q5. This slice changes what a decision prefers, not how often players decide.

## Acceptance Criteria

- **Passes are realistic.** Over 200 matches, passes per team are 350–550 and pass accuracy is 75–88%.
- **The ball is in play for about an hour.** Over 200 matches, the ball is in play for 52–65 minutes per 90 minutes of match time.
- **Throw-ins stay in band.** Over 200 matches, throw-ins per match are 35–55.
- **Only real passes count.** Given a clearance or a restart kick, when it is played, then it is not added to `stats.passes`, and it is counted under its own key.

## Dependencies on Other Slices

- `keeper-and-shots`: the set-piece counts this slice must keep in band while live time falls.
- `realism-bands-v2`: the passes, pass-accuracy and throw-in bands.

## Risks

- **Match length.** Longer restarts do not lengthen the match clock, but added time is computed per stoppage kind. Added time must stay plausible, at a clamp of 900 s per half.
- **Viewer timing.** The live stream paces by ticks, so a longer stoppage is a longer quiet spell on the pitch. The viewer's skip-to-next-event control already covers this.
- **Fewer chances.** Less live time means fewer shots and goals until `realism-tuning`, which the product owner accepted.
- **Speed.** The benchmark measures per-tick CPU, so the gate is unaffected. Wall time per match does not change, because the tick count depends only on match time.
