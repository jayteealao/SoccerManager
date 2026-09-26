---
schema: sdlc/v1
type: brainstorm
slug: brainstorm-realism-additions-20260922
topic: "what needs to be added to players, teams, tactics and match engine to improve realism; starting point is the in progress plans"
status: open
created-at: "2026-09-22T12:00:23Z"
updated-at: "2026-09-22T12:48:15Z"
sessions: 1
batches: 6
threads:
  - id: T-01
    label: "Realism gaps left by the in-progress plans"
    state: live
    routed-to: null
  - id: T-02
    label: "Players — the missing person"
    state: live
    routed-to: null
  - id: T-03
    label: "Teams — the missing club"
    state: live
    routed-to: null
  - id: T-04
    label: "Tactics — beyond Tier 2"
    state: live
    routed-to: null
  - id: T-05
    label: "Match engine — beyond the laws"
    state: live
    routed-to: null
  - id: T-06
    label: "The realism test itself"
    state: live
    routed-to: null
  - id: T-07
    label: "The manager as a modelled entity"
    state: live
    routed-to: null
  - id: T-08
    label: "The season layer"
    state: live
    routed-to: null
claims:
  - id: C-01
    thread: T-01
    text: "The engine bounces the ball off the touchlines and the goal lines like walls; throw-ins, corners, goal kicks, and offside do not exist yet."
    evidence: "verified crates/engine/src/sim.rs:221"
  - id: C-02
    thread: T-02
    text: "The attribute schema holds 37 attributes in four groups and carries no personality attribute, no hidden trait, and no positional-familiarity value."
    evidence: "verified content/attributes.json:1"
  - id: C-03
    thread: T-02
    text: "A player record carries an identifier, a name, a shirt number, one position code, and attributes; it carries no age, no preferred foot, no height, and no starting condition."
    evidence: "verified crates/engine/src/data/team.rs:89"
  - id: C-04
    thread: T-03
    text: "A team file carries a club identifier, a name, a short name, and two kit colours; it carries no reputation, no home ground, and no crowd data."
    evidence: "verified content/teams/default-a.json:1"
  - id: C-05
    thread: T-04
    text: "The planned tactics model is Tier 2: formation, mentality, six team instructions, and per-player roles and duties."
    evidence: "verified .ai/workflows/football-manager-match-engine/03-slice-tactics-and-ai.md:41"
  - id: C-06
    thread: T-06
    text: "The planned realism test is three aggregate bands: goals per match 2.4 to 3.2, shots per team 8 to 16, possession 35 to 65 percent."
    evidence: "verified .ai/workflows/football-manager-match-engine/03-slice-calibration.md:33"
  - id: C-07
    thread: T-01
    text: "The thought is three readings at once: the planned model needs more detail inside it, the numbers can pass while football is still missing, and the realism test is too weak to catch either."
    evidence: unverified
  - id: C-08
    thread: T-01
    text: "This board runs ahead of the build: the football-manager-match-engine workflow is at plan and implement on the stream-protocol slice, so every candidate lands as later scope and none corrects work in flight."
    evidence: "verified .ai/workflows/football-manager-match-engine/00-index.md:9"
  - id: C-09
    thread: T-02
    text: "A player has exactly one position code, drawn from ten: GK, CB, LB, RB, DM, CM, AM, LW, RW, ST."
    evidence: "verified crates/engine/src/data/team.rs:17"
  - id: C-10
    thread: T-02
    text: "All four absent player fields matter: personality and traits, condition and morale and form, age and height and preferred foot, and positional familiarity."
    evidence: unverified
  - id: C-11
    thread: T-02
    text: "A player carries a competence rating for every position he can fill, not one code, so picking a lineup is a trade-off rather than slot-filling."
    evidence: unverified
  - id: C-12
    thread: T-02
    text: "The gap between a strong player and a weak one is three things together: fewer errors, a higher ceiling of attempted actions, and more consistency."
    evidence: unverified
  - id: C-13
    thread: T-02
    text: "New performance-related and personality-related attributes should be created beyond the 37 the schema carries today."
    evidence: unverified
  - id: C-14
    thread: T-02
    text: "A player carries a mood that the match itself moves, and that mood drives his decisions."
    evidence: unverified
  - id: C-15
    thread: T-03
    text: "A team carries a mood of its own, separate from the mood of each player."
    evidence: unverified
  - id: C-16
    thread: T-02
    text: "Personality decides how strongly the player's own mood and the team's mood change that player's decisions; personality is the mediator, not a third mood."
    evidence: unverified
  - id: C-17
    thread: T-02
    text: "The attribute schema accepts 30 to 50 attributes in four fixed groups, so 13 free slots remain today and any personality attribute must be declared in an existing group such as mental."
    evidence: "verified crates/engine/src/data/attributes.rs:10"
  - id: C-18
    thread: T-05
    text: "The tactics-and-ai slice carries a benchmark tripwire: CPU time per match within 10 percent and memory within 25 percent of the match-rules baseline."
    evidence: "verified .ai/workflows/football-manager-match-engine/03-slice-tactics-and-ai.md:73"
  - id: C-19
    thread: T-03
    text: "Team mood is moved by match events, by the score and the clock together, by the crowd and the venue, and by pre-match context."
    evidence: unverified
  - id: C-20
    thread: T-03
    text: "Team mood is also moved by recent form, by the season so far, and by team context such as a club known for late goals or for slumps."
    evidence: unverified
  - id: C-21
    thread: T-07
    text: "The manager is an entity with attributes of his own, and those attributes affect the team's form and mood."
    evidence: unverified
  - id: C-22
    thread: T-03
    text: "Team mood shifts player behaviour inside the chosen tactic and gates the error rate; it does not change the tactic itself."
    evidence: unverified
  - id: C-23
    thread: T-03
    text: "The club carries a home ground with a home advantage, a reputation and stature, a squad familiarity value per tactic, and a playing identity."
    evidence: unverified
  - id: C-24
    thread: T-03
    text: "The squad has leadership, partnerships between pairs of players, and a cohesion value; cohesion is measured for the whole squad and again for the eleven on the pitch."
    evidence: unverified
  - id: C-25
    thread: T-01
    text: "The build workflow places leagues, seasons, competitions, transfers, and finances out of scope, and it parks injury persistence because no season model exists."
    evidence: "verified .ai/workflows/football-manager-match-engine/02-shape.md:239"
  - id: C-26
    thread: T-04
    text: "The tactic has two phases, in possession and out of possession, each with its own shape and its own instructions."
    evidence: unverified
  - id: C-27
    thread: T-04
    text: "Set pieces are designed routines with assigned takers, and the opponent sets a defensive routine against them, so a set piece is a duel rather than a dice roll."
    evidence: unverified
  - id: C-28
    thread: T-04
    text: "Tier 3 is per-player instructions and opposition instructions together, plus manager-set triggers for pressing, the defensive line, and similar situations."
    evidence: unverified
  - id: C-29
    thread: T-08
    text: "A season layer comes into scope: fixtures, results, and form are modelled properly, so this board describes a product larger than the match engine."
    evidence: unverified
  - id: C-30
    thread: T-07
    text: "Both managers carry attributes, and a manager weak at tactics has his instructions land weaker on the pitch, so the human player's own competence is part of the simulation."
    evidence: unverified
  - id: C-31
    thread: T-01
    text: "Every idea on this board belongs to a second workflow, a realism programme started separately from the in-flight match-engine workflow."
    evidence: unverified
  - id: C-32
    thread: T-05
    text: "The ball already carries a three-dimensional position and velocity, so height exists in the engine today and an aerial model has somewhere to live."
    evidence: "verified crates/engine/src/ball.rs:9"
  - id: C-33
    thread: T-05
    text: "The referee is a full official: a foul tolerance, a card threshold, how often he plays advantage, how consistently he applies his own threshold, and a leaning towards the home side that the crowd can move."
    evidence: unverified
  - id: C-34
    thread: T-05
    text: "Pitch condition and weather both exist and they interact; rain on a worn pitch is worse than either alone."
    evidence: unverified
  - id: C-35
    thread: T-05
    text: "An injury carries a severity, and its risk comes from what happens in play — hard tackles, fatigue past a threshold, a collision — rather than from a flat per-minute chance."
    evidence: unverified
  - id: C-36
    thread: T-05
    text: "The aerial model supports flight arcs, heading duels, goalkeeper claims, and spin and swerve, so technique and the preferred foot change where the ball ends up."
    evidence: unverified
assumptions:
  - id: A-01
    thread: T-06
    text: "The three aggregate calibration bands are a sufficient test of realism."
    state: named
  - id: A-02
    thread: T-01
    text: "The in-progress plans are the floor, so every idea on this board adds scope and none removes planned scope."
    state: confirmed
  - id: A-03
    thread: T-01
    text: "Adding detail inside the planned model (reading 1) produces the emergent football that reading 3 says is missing."
    state: named
  - id: A-04
    thread: T-02
    text: "The team generator can produce believable competence ratings for every position without a scouting or history model behind them."
    state: named
  - id: A-05
    thread: T-02
    text: "Personality fits the existing 1-to-100 attribute scale and needs no new data type."
    state: named
  - id: A-06
    thread: T-06
    text: "The three calibration bands still hold once mood and personality move decisions; the extra variance does not push goals, shots, or possession outside them."
    state: named
  - id: A-07
    thread: T-03
    text: "Form, season context, and a club's reputation for late goals can be supplied as asserted numbers in the team file, with no season model behind them."
    state: rejected
  - id: A-08
    thread: T-07
    text: "The manager is a modelled entity with attributes, and not only the AI module the shape already names."
    state: confirmed
  - id: A-09
    thread: T-03
    text: "Per-pair partnership data can be generated for a squad of 22 without a shared history to derive it from."
    state: named
  - id: A-10
    thread: T-04
    text: "A manager-set trigger can live in the rule pack and the change queue, so no conditional tactic logic enters the tick loop."
    state: named
  - id: A-11
    thread: T-07
    text: "Players accept that their own instructions land weaker when their manager is weak at tactics; the loss of authority reads as realism rather than as the game ignoring them."
    state: named
  - id: A-12
    thread: T-08
    text: "The match engine ships first and the season layer builds on it, so no season requirement changes the engine's data contract after the fact."
    state: named
  - id: A-13
    thread: T-05
    text: "A referee who leans towards the home side reads as realism rather than as the game cheating the player."
    state: named
  - id: A-14
    thread: T-05
    text: "Flight arcs, heading duels, and spin on every ball update fit inside the 10 percent CPU tripwire that C-18 records."
    state: named
  - id: A-15
    thread: T-05
    text: "Whether a manager may keep a hurt player on the pitch is still unanswered; C-35 settles severity and risk, and not the manager's choice."
    state: named
contradictions:
  - id: X-01
    threads: [T-01, T-02]
    text: "Reading 1 treats the planned model as the right shape needing more detail, and reading 3 says a fully detailed model can still pass the numbers without reading like football; more detail does not imply emergence. C-16 proposes the bridge and nothing has tested it."
    state: open
  - id: X-02
    threads: [T-02, T-05]
    text: "Competence per position, new attributes, player mood, team mood, partnerships, cohesion, two tactical phases, set-piece routines, triggers, a full referee, interacting conditions, and ball spin all enlarge the per-tick cost, and C-18 records a tripwire that fails the slice at 10 percent CPU or 25 percent memory over baseline."
    state: open
  - id: X-03
    threads: [T-02, T-06]
    text: "C-12 wants weak players to swing more and C-16 wants mood to move decisions, and both widen match-to-match variance; A-06 assumes the three bands survive that, and A-01 already doubts the bands measure realism at all."
    state: open
  - id: X-04
    threads: [T-01, T-03, T-07]
    text: "Form, season context, and a manager who moves form need the season model C-25 excludes from the build. Resolved: C-29 brings a season layer into scope and C-31 puts it in a second workflow, so A-07 is rejected and the numbers are earned rather than asserted."
    state: resolved
  - id: X-05
    threads: [T-01, T-02, T-08]
    text: "C-11 changes the shape of the player record, the data-schemas-generator slice is already complete, and C-31 starts the second workflow after the engine ships; the change is cheapest before the engine is built on the record, and this sequencing pays a migration cost instead."
    state: open
  - id: X-06
    threads: [T-05, T-06]
    text: "C-34 makes pitch wear and weather interact, and C-06 asks three aggregate bands to hold over 1000 matches; either the bands must hold across every combination of conditions, or calibration fixes the conditions and so never tests them."
    state: open
candidates: []
selected: []
consult-runs: []
revisions: []
---

# Brainstorm: realism additions to players, teams, tactics, and the match engine

## The Brainstorm

You brought one thought: the in-progress plans for Touchline build a lawful, tactical, calibrated match, and you want to know what they still miss on realism. Recorded history answered part of that before any question was asked. The engine bounces the ball off the touchlines like walls, and the `match-rules` slice already owns that gap. The reads that followed found the quieter places: 37 attributes with no personality, a player record with no age and no condition, a club with two kit colours and no reputation, a tactics model that stops at Tier 2, and a realism test that stops at three aggregate bands.

Five batches have run and the answer to the original question is now long. All four absent player fields matter. The position code becomes a competence rating per position. The strong-versus-weak gap is errors and ceiling and consistency together. A player carries a mood the match moves, a team carries a mood of its own, personality decides how hard either lands, and team mood shifts behaviour inside the tactic and gates errors without ever changing the tactic. All four club absences matter, and the squad gains leadership, per-pair partnerships, and a cohesion value measured twice — once for the squad, again for the eleven on the pitch. The tactic splits into two phases. Set pieces become a duel of routines. Tier 3 is per-player instructions, opposition instructions, and manager-set triggers.

Batch 5 changed the board's shape rather than filling it. You rejected A-07: form is not an asserted number in a team file, so a season layer comes into scope and T-08 opens for it. Both managers carry attributes, and a manager weak at tactics has his instructions land weaker, which makes the player's own competence part of the simulation and A-11 the risk inside it. All of it belongs to a second workflow, separate from the match engine now in flight. X-04 is resolved by those three answers, and X-05 opened in its place. X-05 is the honest cost of that sequencing: C-11 changes the shape of the player record, the schema slice is already complete, and a second workflow that starts after the engine ships pays a migration the same change avoids today. Three contradictions stay open besides it — X-01 on whether detail produces emergence, X-02 on the 10 percent CPU tripwire, and X-03 on whether the calibration bands can measure any of this. T-05 and T-06 have had no questions yet.

*Sessions: 1 | Batches: 6 | Threads: 8 live · 0 parked · 0 routed · 0 dropped*

## Threads

### T-01 — Realism gaps left by the in-progress plans
**State:** live

The framing thread, and batch 5 turned it into a scoping thread. C-07 records your reading: three at once. C-08 and C-25 are the verified constraints — the workflow sits at `stream-protocol`, and the shape excludes leagues, seasons, competitions, transfers, and finances. C-31 is the decision: every idea here belongs to a second workflow, a realism programme separate from the engine build. A-02 is confirmed, because nothing on this board removes planned scope. A-03 stays named and X-01 is A-03 failing. X-05 is the new cost this thread carries.

### T-02 — Players — the missing person
**State:** live

Ten claims, four verified against code. The model today is a number vector with a label: C-02, C-03, and C-09 say a player is 37 values, one position code, and nothing about the person. C-10 keeps all four absences in scope. C-11 replaces the position code with a competence rating per position, and it is the one change on this board that alters the shape of the player record rather than extending it, which is why X-05 exists. C-12 makes the strong-versus-weak gap three-dimensional and warns that any one dimension alone reads as a flat multiplier. C-13 adds performance and personality attributes, bounded by C-17 at 13 free slots inside four fixed groups. C-14 and C-16 carry the mechanism. A-04 asks whether a generator can invent believable multi-position ratings; A-05 assumes personality fits the 1-to-100 scale. The thread is party to X-01, X-02, X-03, and X-05.

### T-03 — Teams — the missing club
**State:** live

Seven claims. C-04 is the verified starting point: two clubs differ only in their players and every fixture is played on neutral ground. C-23 answers with all four absences — home ground and home advantage, reputation and stature, squad familiarity per tactic, and a club playing identity. C-19 and C-20 say what moves team mood, from match events through to a club's reputation for late goals. C-22 bounds the effect to behaviour inside the tactic and to the error rate. C-24 gives the squad leadership, per-pair partnerships, and cohesion at two scopes. A-07 is rejected, so C-20's inputs now come from T-08 rather than from asserted fields. A-09 is still named: per-pair partnership data needs a shared history to derive it from, and the season layer is where that history would live.

### T-04 — Tactics — beyond Tier 2
**State:** live

Four claims, and you took the expensive option on every one. C-05 is the verified starting point. C-26 splits the tactic into two phases, so the formation becomes two formations and the transition between them needs timing. C-27 makes a set piece a duel: designed routines with assigned takers against a defensive routine. C-28 defines Tier 3 as per-player and opposition instructions, and you added manager-set triggers, which the batch did not offer. A-10 is the assumption under the trigger: it lives in the rule pack and the change queue, so no conditional tactic logic enters the tick loop. C-23's squad familiarity attaches here, and there are now two phases to be familiar with.

### T-05 — Match engine — beyond the laws
**State:** live

Six claims after batch 6, and again you took the fullest option each time. C-18 makes this thread the cost ledger: the tactics-and-ai slice fails at 10 percent CPU or 25 percent memory over baseline, and every addition on every other thread lands against it. C-32 is the useful surprise — the ball already carries a three-dimensional position and velocity, so height exists today and nothing uses it. C-33 makes the referee a full official, down to a home leaning the crowd can move, and A-13 is the risk inside that. C-34 makes pitch wear and weather interact. C-35 gives an injury a severity and drives its risk from hard tackles, fatigue, and collisions, which finally gives the aggression attribute teeth; A-15 records what C-35 did not settle, which is whether a manager may keep a hurt player on. C-36 uses the height that C-32 found: arcs, heading duels, keeper claims, and spin. A-14 assumes all of that fits the tripwire, and X-02 and X-06 both say it may not.

### T-06 — The realism test itself
**State:** live

C-06 records the planned test: goals 2.4 to 3.2, shots 8 to 16, possession 35 to 65, over 1000 matches. A-01 says those bands are sufficient and stays only named, because the probe that would test it went unanswered in batch 1. A-06 points the other way: the additions widen variance, and the bands may fail a match that is more realistic rather than less. X-03 holds both, and no question has been asked on this thread since batch 1.

### T-07 — The manager as a modelled entity
**State:** live

Two claims and the sharpest intent decision on the board. C-21 gives the manager attributes that move form and mood. C-30 extends it to both managers, so a manager weak at tactics has his instructions land weaker on the pitch and the player's own competence becomes part of the simulation. A-08 is confirmed. A-11 is the risk you accepted with it: players have to read the weakening as realism rather than as the game ignoring what they asked for. Nothing in the plans models a manager at all — the shape names an AI module with no identity and no attributes.

### T-08 — The season layer
**State:** live

Opened in batch 5 when you rejected A-07. C-29 brings fixtures, results, and form into scope as a modelled layer, which means this board describes a product larger than the match engine. A-12 is the sequencing assumption: the engine ships first and the season layer builds on it, so no season requirement changes the engine's data contract after the fact. X-05 is the first sign that A-12 is not free, because the player record is one contract a season layer will want changed. Nothing yet says what the season layer contains beyond fixtures, results, and form.

## Turn log

- batch 1 · T-01 · reflection · which reading of the realism gap is yours → a mix of readings 1, 3, and 4
- batch 1 · T-01 · probe · what unrealistic thing would you see → no preference; unanswered
- batch 1 · T-01 · fork · which surfaces get their own thread → all four; T-06 added from reading 4
- batch 2 · T-02 · probe · which missing player field bites first → all four of them
- batch 2 · T-02 · probe · one position code or competence per position → competence per position
- batch 2 · T-02 · probe · how a strong player differs from a weak one → errors, ceiling, and consistency
- batch 2 · T-02 · probe · should a player carry an in-match mood → yes, plus team mood, mediated by personality
- batch 3 · T-03 · probe · what moves team mood → all four, plus form, season, manager, club context
- batch 3 · T-03 · probe · what does team mood change → behaviour inside the tactic, and errors
- batch 3 · T-03 · probe · which club-level absences matter → all four of them
- batch 3 · T-03 · probe · does the squad have structure → leadership, partnerships, and cohesion twice
- batch 4 · T-04 · probe · one shape or phases of play → two phases, in and out of possession
- batch 4 · T-04 · probe · what happens at a set piece → routines, and the opponent defends them
- batch 4 · T-04 · probe · how far past Tier 2 → per-player, opposition, and triggers
- batch 4 · T-04 · steer · how do you want to continue → settle X-04 first
- batch 5 · T-01 · probe · where do form and season context come from → a season layer comes into scope
- batch 5 · T-07 · probe · which managers carry attributes → both, and attributes matter
- batch 5 · T-01 · probe · where do these ideas land → a second workflow for all of it
- batch 6 · T-05 · probe · is the referee a character → full official, with home bias
- batch 6 · T-05 · probe · do conditions exist → pitch and weather, and they interact
- batch 6 · T-05 · probe · are injuries binary → severity, with risk driven by actions
- batch 6 · T-05 · probe · what does the aerial model support → arcs, duels, claims, spin and swerve

## Candidates

<!-- empty until `done` -->

## How to continue

- Resume: `/wf intake brainstorm brainstorm-realism-additions-20260922`
- Control words: `park <thread>` · `pull <thread>` · `drop <thread>` · `board` · `look it up` · `second opinion` · `done`
- Retire when no thread is live: `/wf close brainstorm-realism-additions-20260922`
