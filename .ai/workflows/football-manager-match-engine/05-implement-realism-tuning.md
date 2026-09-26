---
schema: sdlc/v1
type: implement
slug: football-manager-match-engine
slice-slug: realism-tuning
status: awaiting-input
stage-number: 5
created-at: "2026-09-25T20:21:36Z"
updated-at: "2026-09-25T20:21:36Z"
metric-files-changed: 7
metric-lines-added: 240
metric-lines-removed: 19
metric-deviations-from-plan: 2
metric-review-fixes-applied: 0
commit-sha: "e6708aa73159f57bae6a4ce52cf77f6ce0ab670d"
commits:
  - "bdb662612e5276583f5748633e052962a8784f35"
  - "e6708aa73159f57bae6a4ce52cf77f6ce0ab670d"
has-blockers: true
open-questions: []
steering-honored:
  - "Veto (steer.md): the product owner started this stage with /wf implement; no driver started it."
  - "Q-RT2: the corner source is new code from clearances over the defenders' own goal line; RIM-9 kept (a corner only after a defending touch over the goal line); the 3.5-6.5 band unchanged."
  - "Q-RT3: only the listed values were screened; the card chance per foul, the save curve and the hold share were not touched."
  - "Stop rule: a feasibility wall was found, so the slice stopped and reports. No band was widened and no limit was raised."
  - "Q-RT5 (steer.md, 2026-09-25): the slice stops at the formations wall; the corner source stays committed at play-neutral values; no tuned value is written; a separate formation slice comes first."
  - "Heavy runs on the server through .scratch/remote/vps.sh; commits by explicit path only."
tags: [calibration, tuning, realism]
refs:
  index: 00-index.md
  implement-index: 05-implement.md
  slice-def: 03-slice-realism-tuning.md
  plan: 04-plan-realism-tuning.md
  siblings: [05-implement-tempo-and-restarts.md, 05-implement-keeper-and-shots.md, 05-implement-lone-forward.md]
  verify: 06-verify-realism-tuning.md
next-command: wf-slice
next-invocation: "/wf slice football-manager-match-engine"
---

# Implement: Realism tuning

## The Implementation

The slice started from `b9c9212`, where seed 42 missed eight calibrate figures and only one of the ten formations was clearly in band against 4-4-2. The plan put a feasibility stage before any detailed tuning, so that a wall would stop the slice in minutes and not after hours. The corner source is built and committed in two commits (`bdb6626`, `e6708aa`): a clearance near the clearer's own goal line can go wide toward that line, and a corner still comes only after a defending touch over it. At the shipped value 0.0, 1,000 matches on seed 42 give figures identical to the `tempo-and-restarts` gate. Six screens then tested the four walls from `b0`, about 95 candidate runs on the server in total.

Three walls moved. Corners reach 4.3–5.5 per team with the wide clearance at depth 25 m, and ball in play falls to 63–64 minutes. Discipline fits its window at `foul_base` 0.03 and `foul_booked_factor` 0.08, with second yellows at 0.04 and the sending-off share at 0.13–0.17. The red-card test passes in all three arms. Goals are movable through the shot distance weight and the shot noise, and one candidate misses only goal kicks (11.9 against 12) and the goalless share (0.185 against 0.12) on the calibrate clubs.

The formations wall did not move. Across 13 settings, 5-3-2 never scored more than 1.38 goals per match against 4-4-2, and the band floor is 2.4. Every setting that raised the back-five shapes pushed attacking shapes above 3.2, for example 3-4-3 to 4.88 at tackle reach 1.0. The spread across the ten formations stayed at least 1.3 goals on every setting, and the band is 0.8 wide. The slice stops here per the plan. The product owner answered Q-RT5: keep the corner source, stop this slice, and build formation behaviour in a separate slice before the tuning resumes. A second open risk is the goalless share, which stays near twice the value that the goal count predicts.

## Summary of Changes

- A wide clearance: a cleared fast pass in the clearer's own penalty area, or a carrier's clearance within `clearances.wide_depth` of his own goal line, goes wide toward that line with `clearances.wide_chance`.
- Two new tuning values, both shipped so that play is unchanged: `wide_chance` 0.0 and `wide_depth` 16.5.
- Five scenes for the wide clearance, and pins for the new values and for an older file without them.
- A server screen harness (not committed) that measures the calibrate bands and every slow-test figure for each candidate.
- Feasibility screens for corners, goals, discipline, formations and the red card, with logs in `implement-evidence/realism-tuning/screens/`.

## Files Changed

- `crates/engine/src/sim.rs`: the cross clearance can go wide; `WIDE_SPREAD` and `wide_of_goal()`.
- `crates/engine/src/decision.rs`: a carrier's clearance within `wide_depth` of his own goal line can go wide.
- `crates/engine/src/tuning.rs`: `wide_chance` (serde default 0.0) and `wide_depth` (serde default 16.5) with bounds.
- `content/tuning.json`: the two new values at their play-neutral settings.
- `crates/engine/tests/corner_source.rs` (new): five scenes.
- `crates/engine/tests/content.rs`: pins for the new values; a test that a clearances block without them loads.
- `docs/reference/data-files.md`: two table rows and the wide clearance text.

## Shared Files (also touched by sibling slices)

- `crates/engine/src/sim.rs`, `decision.rs`, `tuning.rs`, `content/tuning.json` and `crates/engine/tests/content.rs` were also changed by `tempo-and-restarts` and `keeper-and-shots`.

## Notes on Design Choices

- The wide draw is taken only when `wide_chance` is above 0, so the shipped setting takes no draw and play is unchanged.
- The wide line is halfway between the goal line and the touchline on the ball's side, turned by up to 0.35 rad. The ball then goes behind, goes out for a throw-in or stays in play as it flies.
- The screens read both fixture sets: calibrate uses generated league clubs, and the slow tests use the default clubs on seeds 1–200. Both must pass.

## Verification Seams Built

- Corner source (AC: corners 3.5–6.5 per team) → the wide clearance scenes at `crates/engine/tests/corner_source.rs:55` to `:144` (enable `cargo test -p engine --test corner_source` to observe a corner off the defender, no corner at 0.0 and no corner from a short ball).
- Every gated figure per candidate → the screen harness `implement-evidence/realism-tuning/screens/screen.rs` with `sweep2.py` (enables one server run to report the calibrate bands, the default-club figures, the stronger-team wins, the ten formations and the red-card arms).

## Deviations from Plan

- Step 2 built the wide clearance for carrier clearances by depth, not only inside the penalty area. The first screen (`f-corners.log`) gave at most 2.3 corners per team with the penalty-area check at chance 1.0, so a bounded depth value was added (`e6708aa`). At depth 25 m corners reach 5.1 per team.
- Steps 8–17 did not run. The formations wall of step 6 stops the slice (step 11), so no values were written and no test was made gating.

## Anything Deferred

- Steps 8–17 wait for a separate formation-behaviour slice (Q-RT5). The screens, `best.json` and the harness are the starting evidence when the tuning resumes.

## Known Risks / Caveats

- The goalless share stays at 0.15–0.21 when goals are 2.4–2.8 per match. A random spread of goals gives about 0.06–0.09 at that count. This band can still stop the slice after Q-RT5.
- Goal kicks follow long shots that miss. A shorter shot range raises goals but takes goal kicks under 12.

## Freshness Research

Skipped. The slice changes no dependency and adds no external API surface.

## Closest Candidate

`implement-evidence/realism-tuning/screens/best.json` (`b0`, then wide clearance 1.0 at 25 m, `foul_base` 0.03, shot distance weight 2.0, clearance aim spread 0.9, shot noise 0.6, aim noise 0.1, booked factor 0.08). Seed 42, 200 matches:

| Figure | Calibrate clubs | Default clubs | Band or limit |
|---|---|---|---|
| Goals per match | 2.52 | 2.19 | 2.4–3.2 |
| Goalless share | 0.185 | 0.195 | 0.04–0.12 |
| Goal kicks per match | 11.9 | 12.9 | 12–22 |
| Corners per team | 5.73 | 4.92 | 3.5–6.5 |
| Passes per team, accuracy | 495, 83.1% | 502, 82.1% | 350–550, 75–88% |
| Throw-ins per match | 43.7 | 42.3 | 35–55 |
| Ball in play (minutes per 90) | 67.1 | 67.7 | 52–65 (slow test) |
| Yellows per team, sending-off share | 1.55, 0.165 | 1.75, 0.125 | 1.2–2.6, 0.08–0.22 |
| Second yellows per match | — | 0.040 | at most 0.10 |
| Stronger team wins | — | 99 of 200 | more than 100 |
| Red card (control 1.26, limit 2.02) | — | keeper 1.53/0.81, centre-back 1.50/1.25, striker 1.48/0.65 | pass |
| Formations in band | — | 1 of 10 (0.85–3.70) | 10 of 10 |

## Recommended Next Stage

- **Option D (default): Blocked** on a formation-behaviour slice (Q-RT5). Add it with `/wf slice football-manager-match-engine`, or with `/wf intake football-manager-match-engine <description>`. The product owner starts it. When it is complete, run `/wf implement football-manager-match-engine realism-tuning` again. It resumes at step 8 from `best.json`.
- **Option C: Revisit Plan** → `/wf plan football-manager-match-engine realism-tuning` after the formation slice, if the new behaviour changes the tuning strategy.
