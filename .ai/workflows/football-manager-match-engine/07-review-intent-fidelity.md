---
schema: sdlc/v1
type: review-command
slug: football-manager-match-engine
dimension: intent-fidelity
parent: 07-review.md
review-scope: slug-wide
status: complete
created-at: "2026-09-23T20:34:36Z"
updated-at: "2026-09-23T20:34:36Z"
verdict: ship
metric-findings-total: 0
metric-findings-blocker: 0
metric-findings-high: 0
fragment: none
---

# Review: intent-fidelity

1 findings in this dimension. 1 are fixed and 0 are deferred. Nothing is open at BLOCKER or HIGH.

| ID | Sev | Conf | Status | Surfaced | File:Line | Issue | Fix / reason |
|---|---|---|---|---|---|---|---|
| IF-1 | MED | high | fixed | 2026-09-23T20:34:36Z | content/tuning.json:23 | Goals per match miss the realism band on most calibration seeds | keeper_catch_chance raised from 0.84 to 0.86 in content/tuning.json, Tuning::default and data-files.md. At 1000 matches per suite, goals per match were 2.863 (seed 7), 3.035 (42), 2.948 (99), 2.656 (1) and 2.559 (2026), all inside the band. Shots per team were 12.9 to 14.8 and the stronger-team win rate 0.59 to 0.67, also inside their bands. |
