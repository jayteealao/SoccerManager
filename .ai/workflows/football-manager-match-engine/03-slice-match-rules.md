---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: match-rules
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: l
depends-on: [engine-core, data-schemas-generator]
tags: [engine, rules, set-pieces, snapshot]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-engine-core.md, 03-slice-data-schemas-generator.md, 03-slice-tactics-and-ai.md, 03-slice-commentary.md, 03-slice-viewer-reports-recovery.md, 03-slice-extra-time-penalties.md]
  plan: 04-plan-match-rules.md
  implement: 05-implement-match-rules.md
---

# Slice: Match Rules and Stoppage Snapshots

## The Slice

Engine core plays a kick-about with no laws. The product owner selected offside, fouls with cards, and stoppage time for the first release, and deferred extra time and penalties to a later slice. The rule pack schema from the data slice describes stoppages; nothing enforces them yet.

This slice adds the referee: offside, fouls, yellow and red cards, every restart (kick-off, throw-in, corner, goal kick, free kick, penalty kick), stoppage time, and the half structure. It also writes a snapshot at every stoppage and can restart from one, because stoppages are born here and the product owner coupled the two.

Tactics and commentary build on the events this slice creates. The top risk is offside detection cost at 50 ticks per second; the benchmark from engine core reruns here with the tripwire.

## Goal

A match that obeys the selected laws, restarts play correctly, and can resume from any stoppage.

## Why This Slice Exists

Commitment C6 requires set pieces; Round 5 Q21 selected the laws; Round 4 Q17 requires restart from the last stoppage. Stoppages are also the hinge of the change queue in the next slice.

## Scope

In:
- Rule pack consumption: stoppage kinds, half lengths, stoppage-time calculation.
- Offside tracking every tick; indirect free kick on an offside call.
- Fouls from tackles, with yellow and red cards from discipline attributes; ten-player teams after a red card.
- Restarts: kick-off, throw-in, corner, goal kick, free kick, penalty kick, with the ball placed and players positioned.
- Stoppage time at the end of each half from accumulated stoppages.
- Snapshot at every stoppage (named mechanism per the shape) and `engine-cli resume --snapshot file`.
- Benchmark rerun with the tripwire (more than 10 percent CPU or 25 percent memory regression fails).

Out:
- Extra time and penalty shoot-outs: `extra-time-penalties` (deferred).
- Substitution windows and limits: `tactics-and-ai` (they need the change queue).
- Commentary text: `commentary`.

## Acceptance Criteria

- Given an attacker beyond the second-last defender when a teammate plays the ball forward, When the attacker becomes involved, Then the engine emits an offside event and restarts with an indirect free kick at the offside position.
  <!-- observable: false — cargo test with a scripted scenario -->
- Given a tackle whose foul roll succeeds, Then a foul event and a free kick follow; and Given a foul roll that fails, Then play continues with no event.
  <!-- observable: false — cargo test with seeded rolls -->
- Given a player's second yellow card, Then a red card event follows and the team plays with ten; the formation anchors reshape to ten.
  <!-- observable: false — cargo test -->
- Given the ball crosses each boundary line, Then the matching restart (throw-in, corner, goal kick) is emitted and the ball is placed at the correct spot within 0.5 m.
  <!-- observable: false — cargo tests per boundary -->
- Given accumulated stoppages in a half, Then stoppage time equals the rule pack's formula within one second.
  <!-- observable: false — cargo test -->
- Given a stoppage, Then a snapshot is written; and Given `engine-cli resume` on that snapshot, Then the resumed match has the same score, clock, lineups, and cards as the snapshot.
  <!-- observable: false — cargo test writes, kills, resumes, and compares -->
- Given a corrupt snapshot file, When `engine-cli resume` runs, Then it exits non-zero naming the corruption; and Given a valid snapshot, Then it resumes.
  <!-- observable: false — cargo test -->
- Given the benchmark reruns after this slice, Then CPU time per match is within 10 percent and memory within 25 percent of the engine-core baseline.
  <!-- observable: true — the benchmark report is the developer-visible deliverable -->
  verify: { method: cargo bench harness compare, env: reference laptop, fixture: seed 42, rung: cli-direct }

## Dependencies on Other Slices

- `engine-core`: the loop, the validator, the benchmark baseline.
- `data-schemas-generator`: the rule pack schema and loader.

## Risks

- Offside checks every tick cost too much: check only on forward passes, and benchmark.
- Restart positioning fights the steering agents: restarts set anchors and freeze agents until the ball is in play.
