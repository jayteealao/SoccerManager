---
schema: sdlc/v1
type: skip-record
slug: football-manager-match-engine
skipped-stage: "slice:realism-tuning"
skipped-stage-artifact: 03-slice-realism-tuning.md
reason: "A future workflow inherits the work (product owner, 2026-09-25). The slice stopped at the formations wall of its feasibility stage (Q-RT5); formation behaviour and the tuning move to later work."
skipped-at: "2026-09-25T22:45:37Z"
high-risk: true
---

# Slice skipped: realism-tuning

## Reason

The product owner closed the slice because a future workflow inherits the work. The slice stopped at the formations wall of its feasibility stage. No setting of the tunable values brought the ten formations against 4-4-2 into 2.4–3.2 goals per match: 5-3-2 never scored more than 1.38 on 13 settings. Under Q-RT5, formation behaviour must change in code before the tuning can pass. The future workflow owns that change and the tuning after it.

## What was bypassed

- The final retune against every band on seeds 42, 1, 7, 99 and 2026 (plan steps 8–11).
- The chosen values in `content/tuning.json` and `Tuning::default()` (step 12).
- The gating tests: the three tempo targets, the tightened `every_formation_holds`, and the gating all-suites calibrate test (step 13).
- The re-derived fast-test expectations, the full server gate, the benchmark, the 21 browser tests and the documents (steps 14–17).

Built and kept: the wide clearance (corner source), committed in `bdb6626` and `e6708aa` with its five scenes. It ships at `clearances.wide_chance` 0.0 and `clearances.wide_depth` 16.5, so play is unchanged.

## Downstream impact

- No slice depends on `realism-tuning`. With it skipped, every slice in the roster is resolved, and the workflow can go to the slug-wide review and then handoff.
- These criteria stay unmet on the shipped play, and the future workflow inherits them:
  - the calibrate bands for passes, ball in play, throw-ins, corners, goals, the goalless share and yellow cards
  - the red-card test, the three tempo targets, and the tightened formations gate
- The tests that fail on the shipped play stay marked as later-tuning targets. No test was made gating.
- The starting evidence for the future workflow is in `implement-evidence/realism-tuning/screens/`:
  - `best.json`, the closest setting
  - the screen logs
  - the server harness (`screen.rs`, `sweep2.py`, `sweep2.sh`)

## Stub state

No stub was written. `03-slice-realism-tuning.md` exists; its frontmatter is set to skipped, and its body is intact.
