---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: lone-forward
status: complete
stage-number: 3
created-at: "2026-09-24T06:26:59Z"
updated-at: "2026-09-24T06:26:59Z"
complexity: l
depends-on: [tuning-loop, defending-and-discipline]
source: extension
source-ref: "po-answers.md Q-I2; user description"
extension-round: 2
tags: [engine, tactics, rules, realism]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  source: 05-implement-defending-and-discipline.md
  plan: 04-plan-lone-forward.md
  implement: 05-implement-lone-forward.md
---

# Slice: Lone forward

## Goal

A side with one central forward plays like a real side. The forward holds the ball up, passes and stays onside, and does not dribble through the defence at will. A red card gives the reduced side no advantage, and every formation with a lone forward holds against 4-4-2.

## Why This Slice Exists

`defending-and-discipline` built goal-side cover, the new press and the acting keeper at `f7fe35b`. Two of its criteria still failed at every in-bounds setting of its five tuning values (`05-implement-defending-and-discipline.md`):
- **Red card:** the reduced side outscores the full side in every arm. Keeper 5.82 against 2.06, centre-back 2.39 against 0.71, striker 3.84 against 0.95.
- **Formations:** 4-4-1-1 scores 5.18 and 3-4-3 scores 4.17 against 4-4-2, above the 4.0 limit.

The measured cause is a lone central forward. Figures are for the away side, cards off, seeds 1–48:
- **No pass option:** with no forward team-mate, the carrier's option scores favour a dribble and a shot. The side takes 40.6 shots against 20.8 in the control.
- **Never offside:** 0.0 offsides against 1.2.
- **Contact floods fouls:** each tick near the ball gives about a 10% foul chance against a 2.5% win chance, so the home side fouls 35.5 times against 17.0.

The product owner moved both criteria and both levers into this new slice, before `keeper-and-shots` (`po-answers.md`, Q-I2, Q-X2, Q-X3).

## Scope

- **In:**
  - **The lone forward's decisions:** when the carrier has no forward team-mate, the pass, dribble, shoot and hold-up choices, and his position against the offside line.
  - **Tackle odds:** the odds of a won tackle against a foul when a defender is in contact, as bounded tuning values in `tuning.json`.
  - **Discipline measured again:** fewer fouls move the card figures, so the discipline criterion is measured on the new odds.
  - **Tuning:** through the `tuning-loop` suites, with the baseline copied into the evidence folder.
  - **Tests:** a regression test for each lever, including a lone-forward scene and a contact scene.
- **Out:**
  - The card chance per foul (RIM-7).
  - The keeper save model. `keeper-and-shots` owns it.
  - Goals, shots and possession band tuning. `realism-tuning` owns it.
  - Any change to a criterion limit.

## Acceptance Criteria

- **No advantage from a red card.** In 120 seeds for each case, one away player is sent off at kick-off (striker, centre-back or keeper) and cards are otherwise off. In every case, the reduced side's goals per match do not exceed the full side's. The full side's goals per match are at most 1.6 times the no-send-off baseline for the same seeds.
- **The lone-forward formations hold.** At 11 against 11, over 120 seeds, 4-4-1-1 and 3-4-3 each play against 4-4-2. Neither side averages more than 4.0 goals per match in either pairing.
- **Nothing that passes now regresses.** The eight pairings that pass in `defending-and-discipline` (4-4-2, 4-3-3, 4-2-3-1, 3-5-2, 4-1-4-1, 4-1-2-1-2, 5-3-2 and 5-4-1 against 4-4-2) still hold at most 4.0 goals per side. Over 200 matches, second yellows are at most 0.10 per match, at most 25% of matches have a sending-off, and no player is shown two cards on the same tick.
- **A lone forward uses his team-mates.** Given a lone forward with the ball and an open team-mate behind him, when he is pressed, then he passes or holds the ball up instead of dribbling into the pressure.

## Dependencies on Other Slices

- `tuning-loop`: targeted runs, the red-card suite and the baseline diff.
- `defending-and-discipline`: goal-side cover, the press, the acting keeper and the discipline mechanisms this slice must keep.

## Risks

- **Every seeded result moves.** New decision weights and tackle odds consume random draws. Pinned tests move again. Each moved expectation is re-derived and recorded.
- **Fewer fouls, fewer cards.** A higher win share lowers fouls and so lowers yellow cards and sending-offs. The yellow-card and sending-off bands may leave their range. Band misses are recorded, not failed (Q-E4). The discipline criterion above still holds.
- **The fix does not reach the cause.** If both levers at their bounds cannot pass a criterion, the slice stops and reports the failing arm or pairing. It never raises a limit.
