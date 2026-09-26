---
schema: sdlc/v1
type: brainstorm
slug: brainstorm-realism-additions-20260922
topic: "what needs to be added to players, teams, tactics and match engine to improve realism; starting point is the in progress plans"
status: open
created-at: "2026-09-22T12:00:23Z"
updated-at: "2026-09-22T14:50:43Z"
sessions: 2
batches: 12
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
  - id: C-37
    thread: T-06
    text: "The realism test adds shape measures to the three bands: pass completion, where shots come from, possession in each third, and goals by phase of play."
    evidence: unverified
  - id: C-38
    thread: T-06
    text: "Calibration groups runs by condition band — dry, wet, worn — and sets a band for each, so rain is expected to lower goals rather than to fail the test."
    evidence: unverified
  - id: C-39
    thread: T-01
    text: "The distillation produces several candidates split by dependency, one per coherent piece, so the cheapest pieces can start first."
    evidence: unverified
  - id: C-40
    thread: T-08
    text: "The season layer contains fixtures and results, form, a league table with standings, season-long squad availability with suspensions and injury persistence and carried fitness, and a full club calendar with cups, congestion, and rotation."
    evidence: unverified
  - id: C-41
    thread: T-08
    text: "Form is computed at two scopes, a team form and a per-player form, and the per-player form feeds the player mood in C-14."
    evidence: unverified
  - id: C-42
    thread: T-08
    text: "A new save generates a synthetic backstory of past seasons, so partnerships, form, and club reputation start at believable values in the first match."
    evidence: unverified
  - id: C-43
    thread: T-08
    text: "The club calendar and season-long squad availability are one piece, because congestion and rotation matter only when fitness and suspensions carry between matches."
    evidence: unverified
  - id: C-44
    thread: T-08
    text: "The match event carries a team identifier, a score, and four event types — kick-off, goal, full-time, and tactics-change — and it carries no player identifier."
    evidence: "verified crates/protocol/src/event.rs:16"
  - id: C-45
    thread: T-01
    text: "The stream-protocol slice is complete and four later slices depend on it, so the event contract is a second completed contract the realism work wants changed."
    evidence: "verified .ai/workflows/football-manager-match-engine/00-index.md:137"
  - id: C-46
    thread: T-08
    text: "The event contract gains its player identifier when B-07 starts and not before; the engine ships on the contract it has, and the realism programme pays the migration across every slice that reads the stream by then."
    evidence: unverified
  - id: C-47
    thread: T-08
    text: "The generated backstory is one past season deep: a form record, a final table position, and partnerships earned from minutes shared."
    evidence: unverified
  - id: C-48
    thread: T-01
    text: "The realism work splits into two programmes, engine realism for B-01 to B-06 and the season layer for B-07, so the engine-facing cards start while the season layer is still being shaped."
    evidence: unverified
  - id: C-49
    thread: T-08
    text: "The season layer includes transfers and finances, with a market and budgets and wages, so the product is a full football-management game rather than a match engine with a season around it."
    evidence: unverified
  - id: C-50
    thread: T-01
    text: "Touchline is one product, a football-management game, and the match engine is its first component; the workflow in flight is stage one of a longer programme rather than the whole thing."
    evidence: unverified
  - id: C-51
    thread: T-06
    text: "The realism test reaches the season as well as the match: league-table shape, market and wage shape, and a watched season that a human reads and judges."
    evidence: unverified
  - id: C-52
    thread: T-02
    text: "Clubs disagree about a player's worth for two reasons at once: each club sees a noisy estimate of the attributes set by its scouting quality, and each club weighs the attributes differently by its playing identity and its manager's attributes."
    evidence: unverified
  - id: C-53
    thread: T-01
    text: "The in-flight workflow keeps its charter and its exclusions unchanged, because the exclusions describe that workflow correctly even though they no longer describe the product, and C-48's split is what keeps both statements true."
    evidence: unverified
  - id: C-54
    thread: T-01
    text: "The engine is done when a person can watch and manage one match from start to finish, exactly as the sixteen-slice roster describes; C-50 does not change the engine's definition of done."
    evidence: "verified .ai/workflows/football-manager-match-engine/00-index.md:134"
  - id: C-55
    thread: T-06
    text: "A human watches matches as part of the realism test, as a milestone review rather than as a gate on every build."
    evidence: unverified
  - id: C-56
    thread: T-02
    text: "The player record changes shape when the realism programme starts and not before, following the precedent C-46 set for the event contract, so match-rules and tactics-and-ai are built against a record that will change under them."
    evidence: unverified
  - id: C-57
    thread: T-05
    text: "C-18's tripwire guards the engine build against its own baseline and stops there; the realism programme measures against the shipped engine and sets a budget of its own."
    evidence: unverified
  - id: C-58
    thread: T-06
    text: "The realism test bands the spread as well as the average: how often a match finishes goalless, how often it finishes five-one, and how far a team's worst performance sits from its best."
    evidence: unverified
assumptions:
  - id: A-01
    thread: T-06
    text: "The three aggregate calibration bands are a sufficient test of realism."
    state: rejected
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
    state: rejected
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
    state: rejected
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
  - id: A-16
    thread: T-06
    text: "Aggregate shape measures can detect whether mood and personality read as football; no human watches a match as part of the test."
    state: rejected
  - id: A-17
    thread: T-08
    text: "A generated backstory produces a history that holds together: a club's past results, its partnerships, and its reputation agree with each other and with the squad on the books."
    state: named
  - id: A-18
    thread: T-08
    text: "Per-player match ratings can be derived from the event stream the engine already emits."
    state: rejected
  - id: A-19
    thread: T-08
    text: "The realism programme still fits C-31's shape as one second workflow, although C-40 makes that workflow larger than the match engine it follows."
    state: rejected
  - id: A-20
    thread: T-08
    text: "The cost of changing the event contract after the viewer, commentary, and report slices are built is acceptable, and it is smaller than the cost of disturbing a complete slice today."
    state: named
  - id: A-21
    thread: T-08
    text: "One past season is enough history for a partnership to read as earned, so a pair with one season of shared minutes is distinguishable from a pair with none."
    state: named
  - id: A-22
    thread: T-08
    text: "A transfer market and finances can be built on the attributes B-01 supplies, with no scouting model, no contract-negotiation model, and no separate player-value model."
    state: rejected
  - id: A-23
    thread: T-01
    text: "The charter of the football-manager-match-engine workflow still describes that workflow correctly, although C-50 calls the product a management game and C-25 records exclusions the product no longer honours."
    state: confirmed
  - id: A-24
    thread: T-06
    text: "A human can judge whether a simulated season reads as football from its table, its top scorers, and its transfer list, without watching any match inside it."
    state: named
  - id: A-25
    thread: T-02
    text: "Two sources of valuation disagreement can be calibrated together, so the spread of transfer fees bands cleanly even though one valuation can be wrong for two reasons at once."
    state: named
  - id: A-26
    thread: T-06
    text: "A milestone review that a person performs by hand catches what the shape measures miss, and it happens early enough to catch it before the rest of the model is built on the mechanism."
    state: named
  - id: A-27
    thread: T-05
    text: "A realism budget measured against the shipped engine can be chosen before the additions are built, and the engine stays fast enough to run 1000 calibration matches inside it."
    state: named
  - id: A-28
    thread: T-06
    text: "Real football publishes a distribution of scorelines and a team-to-team spread that the test can band against, the same way it published the three averages in C-06."
    state: named
  - id: A-29
    thread: T-02
    text: "Building match-rules and tactics-and-ai against a player record that will change under them costs less than delaying both slices to change the record first."
    state: named
contradictions:
  - id: X-01
    threads: [T-01, T-02]
    text: "Reading 1 treats the planned model as the right shape needing more detail, and reading 3 says a fully detailed model can still pass the numbers without reading like football; more detail does not imply emergence. C-16 proposes the bridge and nothing has tested it. Batch 11 supplied the instrument rather than the answer: C-55 puts a human in front of a match, so the bridge is now checkable, and A-26 is the assumption that a milestone review catches the failure in time."
    state: open
  - id: X-02
    threads: [T-02, T-05]
    text: "Competence per position, new attributes, player mood, team mood, partnerships, cohesion, two tactical phases, set-piece routines, triggers, a full referee, interacting conditions, and ball spin all enlarge the per-tick cost, and C-18 records a tripwire that fails the slice at 10 percent CPU or 25 percent memory over baseline. Resolved: C-57 scopes that tripwire to the engine build, and the realism programme measures against the shipped engine with a budget of its own. A-27 carries the cost."
    state: resolved
  - id: X-03
    threads: [T-02, T-06]
    text: "C-12 wants weak players to swing more and C-16 wants mood to move decisions, and both widen match-to-match variance; A-06 assumes the three bands survive that, and A-01 already doubts the bands measure realism at all. Resolved: C-58 bands the spread as well as the average, so the test now measures the thing C-12 and C-16 actually added. A-28 assumes real football supplies the distribution to band against."
    state: resolved
  - id: X-04
    threads: [T-01, T-03, T-07]
    text: "Form, season context, and a manager who moves form need the season model C-25 excludes from the build. Resolved: C-29 brings a season layer into scope and C-31 puts it in a second workflow, so A-07 is rejected and the numbers are earned rather than asserted."
    state: resolved
  - id: X-05
    threads: [T-01, T-02, T-08]
    text: "C-11 changes the shape of the player record, the data-schemas-generator slice is already complete, and C-31 starts the second workflow after the engine ships; the change is cheapest before the engine is built on the record, and this sequencing pays a migration cost instead. Resolved: C-56 follows the precedent in C-46 — the record changes when the realism programme starts, and A-29 is the assumption that migrating match-rules and tactics-and-ai later is cheaper than delaying them now. B-08's question is answered, so that card needs no investigation run."
    state: resolved
  - id: X-06
    threads: [T-05, T-06]
    text: "C-34 makes pitch wear and weather interact, and C-06 asks three aggregate bands to hold over 1000 matches; either the bands must hold across every combination of conditions, or calibration fixes the conditions and so never tests them. Resolved: C-38 sets a band per condition band, so a condition is expected to move the numbers rather than to fail the test."
    state: resolved
  - id: X-07
    threads: [T-06, T-08]
    text: "C-41 wants a per-player form, and C-44 records that the match event carries no player identifier at all; C-45 records that the stream-protocol slice is complete with four slices depending on it, so per-player form needs the same kind of contract change, at the same kind of cost, that X-05 already names for the player record. Resolved: C-46 decides the timing — the contract changes when B-07 starts, and A-20 carries the migration cost. The decision covers the event contract only, so X-05 and the player record stay open."
    state: resolved
  - id: X-08
    threads: [T-01, T-08]
    text: "C-31 puts the realism work in one second workflow that starts after the engine ships, and C-40 makes that workflow a full football-management game with a calendar, cups, a table, and season-long availability, which is larger than the engine it was meant to follow; A-12 assumed the season layer would never change the engine's data contract, and C-41 and C-43 both want data the engine does not emit. Resolved: C-48 splits the work into two programmes and A-19 is rejected with it. A-12 is rejected as well, because C-46 changes the engine's data contract after the fact by design rather than by accident."
    state: resolved
  - id: X-09
    threads: [T-06, T-08]
    text: "C-49 makes transfers, wages, and budgets part of the product, and the only realism test on the board is C-06's match bands with C-37's shape measures; nothing tests whether a market, a wage bill, or a season of results reads as football, so the largest part of the product has no realism test at all. Resolved: C-51 takes all three season measures — table shape, market and wage shape, and a watched season."
    state: resolved
  - id: X-10
    threads: [T-02, T-08]
    text: "C-49 needs a player value and a scouting judgment for a market to work, and B-01's model gives a manager every attribute in full; when every club sees every attribute exactly, no club can value a player differently from any other, so the market has no disagreement to trade on. A-22 assumes no scouting model is needed and this is the reason to doubt it. Resolved: C-52 supplies both noise and weights, A-22 is rejected, and A-25 carries the calibration cost."
    state: resolved
  - id: X-11
    threads: [T-06]
    text: "C-51 puts a human inside the season test, and A-16 keeps a human out of the match test; the argument that justifies watching a season justifies watching a match, so either A-16 falls or the watched season is judging something the watched match could not. Resolved: A-16 falls. C-55 puts a human in front of matches as well, as a milestone review rather than a per-build gate."
    state: resolved
  - id: X-12
    threads: [T-01, T-08]
    text: "C-50 says the product is a football-management game whose first component is the engine, and C-25 records that the engine workflow's own charter excludes leagues, seasons, competitions, transfers, and finances; the workflow in flight is therefore building to a charter the product has outgrown, and nothing has told that workflow. Resolved: C-53 leaves the charter as it stands and C-54 leaves the engine's definition of done unchanged, because C-48's split makes both descriptions true at once. A-23 is confirmed, and the residual cost is that a reader of that workflow alone will take the engine for the whole product."
    state: resolved
candidates:
  - id: B-01
    thread: T-02
    title: "Deepen the player model into a person"
    shape: intake
    entry: "/wf intake player-model-realism from brainstorm-realism-additions-20260922"
    state: proposed
    routed-to: null
  - id: B-02
    thread: T-03
    title: "Model the club, the venue, and the squad"
    shape: intake
    entry: "/wf intake club-and-squad-model from brainstorm-realism-additions-20260922"
    state: proposed
    routed-to: null
  - id: B-03
    thread: T-04
    title: "Raise tactics to Tier 3 with phases, set pieces, and triggers"
    shape: intake
    entry: "/wf intake tactics-tier-3 from brainstorm-realism-additions-20260922"
    state: proposed
    routed-to: null
  - id: B-04
    thread: T-05
    title: "Model conditions, officials, injuries, and the ball in the air"
    shape: intake
    entry: "/wf intake match-conditions-and-officials from brainstorm-realism-additions-20260922"
    state: proposed
    routed-to: null
  - id: B-05
    thread: T-06
    title: "Rebuild the realism test with shape measures and per-condition bands"
    shape: extension
    entry: "/wf intake football-manager-match-engine add shape measures and per-condition bands to the calibration slice"
    state: proposed
    routed-to: null
  - id: B-06
    thread: T-07
    title: "Model both managers as entities with attributes"
    shape: intake
    entry: "/wf intake manager-entity from brainstorm-realism-additions-20260922"
    state: proposed
    routed-to: null
  - id: B-07
    thread: T-08
    title: "Build the season layer that earns form"
    shape: intake
    entry: "/wf intake season-layer from brainstorm-realism-additions-20260922"
    state: proposed
    routed-to: null
  - id: B-08
    thread: T-01
    title: "Decide when the player record shape changes"
    shape: investigate
    entry: "/wf intake investigate whether the player record gains per-position competence before the match engine ships or after from brainstorm-realism-additions-20260922"
    state: proposed
    routed-to: null
selected: []
consult-runs: []
revisions:
  - rev: 1
    at: "2026-09-22T12:52:01Z"
    trigger: manual
    because: "done — the person asked to distill after batch 7"
    changed: "eight candidate cards written, A-01 rejected, X-06 resolved, status set to distilled"
  - rev: 2
    at: "2026-09-22T14:09:46Z"
    trigger: resume
    because: "session 2 opened on the distilled board"
    changed: "status reopened to open, sessions bumped to 2, the eight candidates kept as written"
---

# Brainstorm: realism additions to players, teams, tactics, and the match engine

## The Brainstorm

You brought one thought: the in-progress plans for Touchline build a lawful, tactical, calibrated match, and you want to know what they still miss on realism. Recorded history answered part of that before any question was asked. The engine bounces the ball off the touchlines like walls, and the `match-rules` slice already owns that gap. The reads that followed found the quieter places: 37 attributes with no personality, a player record with no age and no condition, a club with two kit colours and no reputation, a tactics model that stops at Tier 2, and a realism test that stops at three aggregate bands.

Seven batches have run and the answer to the original question is now long. All four absent player fields matter. The position code becomes a competence rating per position. The strong-versus-weak gap is errors and ceiling and consistency together. A player carries a mood the match moves, a team carries a mood of its own, personality decides how hard either lands, and team mood shifts behaviour inside the tactic and gates errors without ever changing the tactic. All four club absences matter, and the squad gains leadership, per-pair partnerships, and a cohesion value measured twice — once for the squad, again for the eleven on the pitch. The tactic splits into two phases. Set pieces become a duel of routines. Tier 3 is per-player instructions, opposition instructions, and manager-set triggers.

Batches 5 to 7 changed the board's shape rather than filling it. You rejected A-07: form is not an asserted number in a team file, so a season layer came into scope and T-08 opened for it. Both managers carry attributes, and a manager weak at tactics has his instructions land weaker, which makes the player's own competence part of the simulation and A-11 the risk inside it. The referee became a full official with a home leaning, pitch wear and weather now interact, injuries carry a severity and a risk driven by tackles and fatigue, and the ball gained arcs and spin on the height the engine already had. Then you rejected A-01 as well: three aggregate bands are not enough, so the test gains shape measures and a band per condition band.

Session 2 opened on T-08, the thinnest thread, and turned it into the largest thing on the board. The season layer took every addition offered: a league table, season-long availability with suspensions and carried fitness, a full club calendar with cups and rotation, and then, in batch 9, transfers and finances as well. Form is computed twice, for the team and for each player. A new save generates one past season of history, so partnerships and reputation start earned rather than blank, which rejected A-09. A bounded read rejected A-18 too — the match event carries a team identifier, a score, and four event types, and no player identifier at all, so per-player form has no source today.

Batch 9 spent the rest of the session paying for batch 8. You chose to change the event contract when B-07 starts rather than now, although every one of its thirteen consumers is still unbuilt, so A-20 carries a migration this board can see coming and A-12 is rejected: the season layer will change the engine's contract after the fact, by decision rather than by accident. You split the work in two — engine realism and the season layer — which resolved X-08 and rejected A-19. Then you took transfers and finances, so C-49 makes this a football-management game rather than a match engine with a season around it, and C-25's exclusion is now a boundary the second programme crosses on purpose.

Batch 10 asked what the board had become and closed the two gaps batch 9 opened. Your reading is C-50: Touchline is one product, a football-management game, and the match engine is its first component rather than the whole thing. The season test takes everything on offer — league-table shape, market and wage shape, and a watched season a human reads and judges — which resolves X-09. The market gets both of its disagreement sources, noise from scouting quality and weights from club identity, which resolves X-10 and rejects A-22.

Batch 11 answered both of the contradictions batch 10 created, and it did so by leaving work alone. C-53 keeps the in-flight charter exactly as it is, and C-54 keeps the engine's definition of done at a playable match, because C-48's split already makes the exclusions true of that workflow while being false of the product; A-23 is confirmed and X-12 closes. Then A-16 fell. C-55 puts a human in front of matches as well as seasons, as a milestone review rather than a gate on every build, which closes X-11 and does something larger besides.

Batch 12 closed three of the four that were left. C-56 sends the player record the same way as the event contract — it changes when the realism programme starts, and A-29 carries the cost of building match-rules and tactics-and-ai against a record that will move under them; B-08's question is answered and that card needs no investigation run. C-57 scopes C-18's tripwire to the engine build, so the realism programme measures against the shipped engine and picks a budget it can live in rather than inheriting one written for a different purpose. C-58 bands the spread as well as the average, which finally measures the thing C-12 and C-16 added rather than the thing they left alone.

One contradiction is open, and it is the one the board started with. X-01 says a fully detailed model can pass every number and still not read like football; C-16 proposes that personality mediating mood is the bridge, and twelve batches have not tested it. What changed is that it is now testable — C-55 puts a human in front of a match, and A-26 is the bet that a milestone review run by hand happens early enough to matter. Nine assumptions stand named against it. Eight candidates stand as written from session 1, and five of them — B-01, B-02, B-05, B-07, and B-08 — no longer say what this board says; distilling is what would fix that.

*Sessions: 2 | Batches: 12 | Threads: 8 live · 0 parked · 0 routed · 0 dropped | Candidates: 8*

## Threads

### T-01 — Realism gaps left by the in-progress plans
**State:** live

The framing thread, and batch 5 turned it into a scoping thread. C-07 records your reading: three at once. C-08 and C-25 are the verified constraints — the workflow sits at `stream-protocol`, and the shape excludes leagues, seasons, competitions, transfers, and finances. C-31 is the decision: every idea here belongs to a second workflow, a realism programme separate from the engine build. A-02 is confirmed, because nothing on this board removes planned scope. A-03 stays named and X-01 is A-03 failing. Session 2 added C-45: the stream-protocol slice is complete and four slices depend on it, so this thread carried two after-the-fact contract changes rather than one. Batch 9 settled one of them. C-46 changes the event contract when B-07 starts and not before, which resolved X-07 and left X-05 and the player record still open, so B-08 now gates the player record alone. C-48 is the other decision: the work splits into two programmes, engine realism for B-01 to B-06 and the season layer for B-07, which resolved X-08 and amends C-31 rather than replacing it. Batch 10 then settled what all of it is for. C-50 reads the whole board as one product, a football-management game with the match engine as its first component. Batch 11 decided what that costs the work in flight, and the answer is nothing: C-53 keeps the charter and its exclusions as they stand, C-54 keeps the engine done at a playable match on the sixteen-slice roster, and A-23 is confirmed because C-48's split makes both descriptions true at once. X-12 closes on that. The residual cost is recorded rather than removed — a person who reads the engine workflow alone will take the engine for the whole product.

### T-02 — Players — the missing person
**State:** live

Ten claims, four verified against code. The model today is a number vector with a label: C-02, C-03, and C-09 say a player is 37 values, one position code, and nothing about the person. C-10 keeps all four absences in scope. C-11 replaces the position code with a competence rating per position, and it is the one change on this board that alters the shape of the player record rather than extending it, which is why X-05 exists. C-12 makes the strong-versus-weak gap three-dimensional and warns that any one dimension alone reads as a flat multiplier. C-13 adds performance and personality attributes, bounded by C-17 at 13 free slots inside four fixed groups. C-14 and C-16 carry the mechanism. A-04 asks whether a generator can invent believable multi-position ratings; A-05 assumes personality fits the 1-to-100 scale. Batch 12 settled the timing: C-56 changes the record when the realism programme starts, following C-46's precedent, and A-29 is the assumption that migrating match-rules and tactics-and-ai later beats delaying them now. C-52 added the other half of what a player is — a club sees a noisy estimate of his attributes rather than the exact numbers, weighed by its own identity, which is where B-07's market finds its disagreement. X-02, X-03, and X-05 are all resolved now, and the thread is party only to X-01.

### T-03 — Teams — the missing club
**State:** live

Seven claims. C-04 is the verified starting point: two clubs differ only in their players and every fixture is played on neutral ground. C-23 answers with all four absences — home ground and home advantage, reputation and stature, squad familiarity per tactic, and a club playing identity. C-19 and C-20 say what moves team mood, from match events through to a club's reputation for late goals. C-22 bounds the effect to behaviour inside the tactic and to the error rate. C-24 gives the squad leadership, per-pair partnerships, and cohesion at two scopes. A-07 is rejected, so C-20's inputs now come from T-08 rather than from asserted fields. A-09 is rejected as well: session 2 answered it, and C-42 says the season layer generates the shared history at save creation rather than inventing partnerships without one. The consequence is a dependency this thread did not carry before — B-02's partnerships read as believable on day one only once B-07 supplies the backstory.

### T-04 — Tactics — beyond Tier 2
**State:** live

Four claims, and you took the expensive option on every one. C-05 is the verified starting point. C-26 splits the tactic into two phases, so the formation becomes two formations and the transition between them needs timing. C-27 makes a set piece a duel: designed routines with assigned takers against a defensive routine. C-28 defines Tier 3 as per-player and opposition instructions, and you added manager-set triggers, which the batch did not offer. A-10 is the assumption under the trigger: it lives in the rule pack and the change queue, so no conditional tactic logic enters the tick loop. C-23's squad familiarity attaches here, and there are now two phases to be familiar with.

### T-05 — Match engine — beyond the laws
**State:** live

Six claims after batch 6, and again you took the fullest option each time. C-18 makes this thread the cost ledger: the tactics-and-ai slice fails at 10 percent CPU or 25 percent memory over baseline, and every addition on every other thread lands against it. C-32 is the useful surprise — the ball already carries a three-dimensional position and velocity, so height exists today and nothing uses it. C-33 makes the referee a full official, down to a home leaning the crowd can move, and A-13 is the risk inside that. C-34 makes pitch wear and weather interact. C-35 gives an injury a severity and drives its risk from hard tackles, fatigue, and collisions, which finally gives the aggression attribute teeth; A-15 records what C-35 did not settle, which is whether a manager may keep a hurt player on. C-36 uses the height that C-32 found: arcs, heading duels, keeper claims, and spin. A-14 assumed all of that fits the tripwire, and X-02 and X-06 both said it may not. Batch 12 resolved the question rather than the cost: C-57 scopes C-18's tripwire to the engine build, and the realism programme measures against the shipped engine with a budget of its own. A-27 is what that rests on — the budget is chosen before the additions exist, and the engine must still run 1000 calibration matches inside it.

### T-06 — The realism test itself
**State:** live

C-06 records the planned test: goals 2.4 to 3.2, shots 8 to 16, possession 35 to 65, over 1000 matches. Batch 7 rejected A-01: three bands are not enough. C-37 adds shape measures the event stream already carries — pass completion, where shots come from, possession in each third, and goals by phase of play — so a match that passes the bands and still looks wrong now fails something. C-38 resolves X-06 by setting a band per condition band, dry and wet and worn, so a condition is expected to move the numbers rather than to fail the test. You did not take the watched check for a match, and A-16 records what that costs: whether the mood mechanism in C-16 reads as football is judged by aggregates alone. Batch 10 then extended the test to the season and took every measure offered. C-51 adds league-table shape — points spread, goal-difference distribution, home-win rate, and how often the strongest squad wins — plus market and wage shape, plus a watched season that a human reads and judges. A-24 is the assumption inside the watched season: a table, a scorer list, and a transfer list are enough to judge from, with no match watched. Batch 11 then rejected A-16 outright. C-55 puts a human in front of matches too, as a milestone review rather than a gate on every build, which closes X-11 and matters far beyond consistency — it is the first instrument on this board that can reach X-01 at all. A-26 is what it rests on: a review run by hand has to happen early enough to catch a failed mechanism before the rest of the model is built on it. Batch 12 finished the test. C-58 bands the spread as well as the average — how often a match finishes goalless, how often it finishes five-one, how far a team's worst performance sits from its best — which resolves X-03 by measuring what C-12 and C-16 actually added. A-28 is the assumption underneath: real football publishes that distribution to band against, the way it published the three averages. A-06 stays named, because the averages are still expected to survive.

### T-07 — The manager as a modelled entity
**State:** live

Two claims and the sharpest intent decision on the board. C-21 gives the manager attributes that move form and mood. C-30 extends it to both managers, so a manager weak at tactics has his instructions land weaker on the pitch and the player's own competence becomes part of the simulation. A-08 is confirmed. A-11 is the risk you accepted with it: players have to read the weakening as realism rather than as the game ignoring what they asked for. Nothing in the plans models a manager at all — the shape names an AI module with no identity and no attributes.

### T-08 — The season layer
**State:** live

Opened in batch 5 when you rejected A-07, and filled in batch 8. C-29 brought fixtures, results, and form into scope. C-40 adds a league table with standings, season-long squad availability with suspensions and injury persistence and carried fitness, and a full club calendar with cups, congestion, and rotation — you took every addition offered, so this is now the largest thread on the board. C-43 binds two of them together: congestion and rotation mean nothing unless fitness and suspensions carry between matches, so the calendar and availability ship as one piece. C-41 computes form twice, for the team and for each player, and the per-player form feeds C-14's mood. C-42 generates a backstory at save creation, which rejects A-09 and gives B-02 its partnerships; A-17 is the risk inside it, because a generated history has to agree with itself. A-18 is rejected by a read: C-44 records that the match event carries no player identifier, so per-player form has no source today. Batch 9 then decided what each of those costs. C-46 changes the event contract when B-07 starts, which resolves X-07 and rejects A-12, because the engine's contract now changes after the fact by decision; A-20 is the assumption that the later migration is the cheaper one, and every consumer of the stream being unbuilt today is the reason to doubt it. C-47 sets the backstory at one past season, with A-21 assuming that is enough history for a partnership to read as earned. C-49 is the largest claim on the board: transfers and finances come in, so this is a football-management game and not a match engine with a season around it. X-09 and X-10 are what C-49 costs — no realism test reaches a market or a season, and a market needs clubs to disagree about a player that B-01 shows everyone in full.

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
- batch 7 · T-06 · probe · are the three bands enough → no; add shape measures
- batch 7 · T-06 · probe · how do conditions enter the test → a band per condition band
- batch 7 · T-01 · probe · what shape should the output take → several candidates by dependency
- batch 8 · T-08 · probe · what the season layer contains → table, availability, and full calendar
- batch 8 · T-08 · probe · how form is computed → team form and per-player form together
- batch 8 · T-08 · probe · where the shared history comes from → a backstory generated at save creation
- batch 8 · T-08 · steer · where the thinking goes next → continue on T-08
- batch 9 · T-08 · probe · when the event contract gains a player id → when B-07 starts, pay the migration
- batch 9 · T-08 · probe · how deep the generated backstory goes → one past season
- batch 9 · T-08 · probe · how the realism work is divided → split the season layer out
- batch 9 · T-08 · probe · where the season layer stops → transfers and finances as well
- batch 10 · T-01 · reflection · what this board has become → one product, a management game
- batch 10 · T-06 · probe · what a season realism test measures → table, market, and a watched season
- batch 10 · T-02 · probe · where market disagreement comes from → read as both noise and weights
- batch 11 · T-01 · probe · what happens to the in-flight charter → leave it alone, unchanged
- batch 11 · T-01 · probe · whether the engine's definition of done changes → no change, a playable match
- batch 11 · T-06 · probe · whether a human watches matches too → yes, as a milestone review
- batch 12 · T-02 · probe · when the player record changes shape → later, following the C-46 precedent
- batch 12 · T-05 · probe · what the tripwire means for realism → the programme sets its own budget
- batch 12 · T-06 · probe · how the test handles widened variance → band the spread, not only the average
- batch 12 · control · board → printed and published as an artifact; the loop holds, not distilled

## Candidates

Eight candidates, one per live thread, split by dependency per C-39. Every one inherits the whole board; the ids named on each card are what that card turns on.

### B-01 — Deepen the player model into a person [T-02]
**Inherits:** C-02, C-03, C-09, C-10, C-11, C-12, C-13, C-14, C-16, C-17, C-52 · A-04, A-05, A-25 · X-01, X-02, X-05

Competence per position instead of one code, new performance and personality attributes inside the 13 free slots the schema leaves, age and height and preferred foot and condition, and a strong-versus-weak gap made of errors, ceiling, and consistency. It carries C-16, the mechanism that answers reading 3: a mood the match moves, read through a personality that decides how hard it lands. This is the largest and most load-bearing card, and B-08 gates its most expensive part. Batch 10 added C-52: a club sees a noisy estimate of a player's attributes rather than the exact numbers, with the noise set by its scouting quality, and it weighs what it sees by its own identity. That makes this card the source of the disagreement B-07's market trades on, and A-25 is the assumption that two sources of error still band cleanly.

**Entry:** `/wf intake player-model-realism from brainstorm-realism-additions-20260922`

### B-02 — Model the club, the venue, and the squad [T-03]
**Inherits:** C-04, C-15, C-19, C-22, C-23, C-24, C-42 · A-09 (rejected), A-17 · X-04

Home ground and home advantage, reputation and stature, squad familiarity per tactic, a club playing identity, and a team mood moved by events, by score and clock, and by crowd and venue. The squad gains leadership, per-pair partnerships, and cohesion at two scopes — the squad, and the eleven on the pitch. Session 2 changed this card's dependency. A-09 is rejected and C-42 supplies the shared history, so the partnerships read as believable on day one only once B-07 generates the backstory. Start this card without B-07 and accept flat partnerships in the first season, or sequence B-07 first.

**Entry:** `/wf intake club-and-squad-model from brainstorm-realism-additions-20260922`

### B-03 — Raise tactics to Tier 3 with phases, set pieces, and triggers [T-04]
**Inherits:** C-05, C-26, C-27, C-28 · A-10 · X-02

Two phases with their own shapes, set-piece routines that the opponent defends against, per-player and opposition instructions, and manager-set triggers for pressing and the defensive line. A-10 is the constraint to hold: the trigger lives in the rule pack and the change queue, so no conditional tactic logic enters the tick loop. Opposition instructions need the preferred foot and position competence from B-01 to mean anything.

**Entry:** `/wf intake tactics-tier-3 from brainstorm-realism-additions-20260922`

### B-04 — Model conditions, officials, injuries, and the ball in the air [T-05]
**Inherits:** C-18, C-32, C-33, C-34, C-35, C-36 · A-13, A-14, A-15 · X-02

A full referee with tolerance, threshold, advantage, consistency, and a home leaning; pitch wear and weather that interact; injury severity with risk driven by tackles, fatigue, and collisions; and an aerial model with arcs, heading duels, keeper claims, and spin. C-32 is the head start, because the ball already carries height and nothing uses it. This card owns X-02, the 10 percent CPU tripwire, on behalf of every other card. A-15 names what C-35 left unanswered: whether a manager may keep a hurt player on the pitch.

**Entry:** `/wf intake match-conditions-and-officials from brainstorm-realism-additions-20260922`

### B-05 — Rebuild the realism test with shape measures and per-condition bands [T-06]
**Inherits:** C-06, C-37, C-38, C-51, C-55 · A-06, A-16 (rejected), A-24, A-26 · X-01, X-03

Pass completion, shot locations, possession by third, and goals by phase added to the three bands, plus a band per condition band. This card was shaped as an extension of the existing workflow rather than a new one, because the `calibration` slice is defined and unbuilt and the match measures are cheapest to design into it now. Batch 10 outgrew that shape. C-51 adds league-table shape, market and wage shape, and a watched season, none of which the `calibration` slice can hold, so the card now splits in two: the match measures stay an extension of the existing workflow, and the season measures belong to B-07's programme. Batch 11 then rejected A-16 and added C-55: a human watches matches as well, as a milestone review rather than a gate on every build. That makes this card the only one on the board that can reach X-01, so it stops being a calibration chore and becomes the test of whether the whole realism argument holds. A-26 is the risk — a review run by hand has to happen early enough to matter.

**Entry:** `/wf intake football-manager-match-engine add shape measures and per-condition bands to the calibration slice`

### B-06 — Model both managers as entities with attributes [T-07]
**Inherits:** C-21, C-30 · A-08, A-11 · X-04

A manager record with attributes on both sides, giving the AI opponent character and making the human player's own competence part of the simulation. A-11 is the risk you accepted, and it is the one to test with real players: an instruction that lands weaker must read as realism rather than as the game ignoring what was asked.

**Entry:** `/wf intake manager-entity from brainstorm-realism-additions-20260922`

### B-07 — Build the season layer that earns form [T-08]
**Inherits:** C-20, C-25, C-29, C-40, C-41, C-42, C-43, C-44, C-45, C-46, C-47, C-48, C-49 · A-07 (rejected), A-09 (rejected), A-12 (rejected), A-17, A-18 (rejected), A-19 (rejected), A-20, A-21, A-22 · X-04, X-07, X-08, X-09, X-10

Batch 8 made this the largest card on the board. Fixtures, results, and form modelled rather than asserted, a league table with standings, season-long squad availability with suspensions and injury persistence and carried fitness, and a full club calendar with cups, congestion, and rotation. C-43 ships the calendar and the availability together. Form is computed twice, for the team and for each player, and C-42 generates a backstory at save creation so day one reads like football; A-17 is the risk that the generated history does not agree with itself. Batch 9 then took transfers and finances as well, so C-49 makes this card a football-management game rather than a season around a match engine. It is its own programme now, per C-48, and it is larger than the match engine it follows. Three costs ride with it. C-46 changes the event contract when this card starts rather than today, so the migration in A-20 reaches every slice that reads the stream by then. X-09 says nothing on the board tests whether a market, a wage bill, or a season of results reads as football, so B-05's rebuilt test covers matches and stops there. X-10 says a market needs clubs to disagree about a player's worth, and B-01 shows every manager every attribute exactly, so A-22 is the assumption to attack first. Start this card by deciding what a realism test for a season even measures.

**Entry:** `/wf intake season-layer from brainstorm-realism-additions-20260922`

### B-08 — Decide when the player record shape changes [T-01]
**Inherits:** C-08, C-11, C-31, C-46, C-48 · A-20 · X-05

The one card that is a question rather than a build, and batch 9 narrowed it to the player record alone. C-11 changes the shape of that record, the `data-schemas-generator` slice is already complete, and C-31 starts the realism work after the engine ships. Either the record changes now and the engine is built on it, or it changes later and a migration pays for the delay. Batch 12 answered it. C-56 follows C-46's precedent: the record changes when the realism programme starts, and A-29 carries the cost of building match-rules and tactics-and-ai against a shape that will move under them. This card no longer needs an investigation run — it needs recording as a decision, which is what the next distillation should do with it.

**Entry:** `/wf intake investigate whether the player record gains per-position competence before the match engine ships or after from brainstorm-realism-additions-20260922`

## Selection

<!-- recorded when the person picks -->

## How to continue

- Board as a page (session 2, batch 12): https://claude.ai/artifact/YbrN4YkabjMwpfz4k8BUnQ
- Resume: `/wf intake brainstorm brainstorm-realism-additions-20260922`
- Control words: `park <thread>` · `pull <thread>` · `drop <thread>` · `board` · `look it up` · `second opinion` · `done`
- Retire when no thread is live: `/wf close brainstorm-realism-additions-20260922`
