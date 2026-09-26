---
schema: sdlc/v1
type: brainstorm
slug: brainstorm-realism-additions-20260922
topic: "what needs to be added to players, teams, tactics and match engine to improve realism; starting point is the in progress plans"
status: open
created-at: "2026-09-22T12:00:23Z"
updated-at: "2026-09-22T18:44:49Z"
sessions: 3
batches: 20
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
    text: "A player carries a competence rating for every position the player can fill, not one code, so picking a lineup is a trade-off rather than slot-filling."
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
    text: "A player carries a mood that the match itself moves, and that mood drives the player's decisions."
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
    text: "The manager is an entity with attributes of their own, and those attributes affect the team's form and mood."
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
    text: "Both managers carry attributes, and a manager weak at tactics has their instructions land weaker on the pitch, so the human player's own competence is part of the simulation."
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
    text: "The referee is a full official: a foul tolerance, a card threshold, how often the referee plays advantage, how consistently the referee applies that threshold, and a leaning towards the home side that the crowd can move."
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
  - id: C-59
    thread: T-06
    text: "Across 1826 matches in five top leagues in 2017/18, goals per match average 2.709 with a standard deviation of 1.675, and the distribution of total goals is Poisson to within 1.5 percentage points at every count, with a variance-to-mean ratio of 1.035."
    evidence: "verified .scratch/out/wyscout_rates.json (1826 matches, five leagues 2017/18)"
  - id: C-60
    thread: T-06
    text: "Home teams score 1.530 goals and away teams 1.179, a home advantage of 0.351 goals, and results split 45.3 percent home win, 24.5 percent draw, and 30.2 percent away win."
    evidence: "verified .scratch/out/wyscout_rates.json (1826 matches, five leagues 2017/18)"
  - id: C-61
    thread: T-06
    text: "Per-match rates are tight across five different leagues: passes 839 to 888, pass accuracy 82.2 to 83.3 percent, shots 21.0 to 23.2, shot conversion 9.7 to 11.1 percent, duels 442 to 471, aerial duels 76 to 99, fouls 21 to 29, yellow cards 3.1 to 4.9, throw-ins 42 to 46, corners 9.5 to 10.3, goal kicks 16 to 17, free kicks 19 to 27, and offsides 4.0 to 5.0."
    evidence: "verified .scratch/out/wyscout_rates.json (1826 matches, five leagues 2017/18)"
  - id: C-62
    thread: T-05
    text: "A real match carries about 1683 events; Wyscout types them with 10 types and about 35 subtypes, Metrica with 9 types and subtypes, and kloppy's interoperable model with 19 types."
    evidence: "verified .scratch/out/wyscout_rates.json (1826 matches, five leagues 2017/18)"
  - id: C-63
    thread: T-05
    text: "Pass completion depends on the kind of pass, not on one rate: simple 91.7 percent, head 62.5 percent, launch 53.8 percent, high 53.7 percent, through ball 39.2 percent, and cross 30.7 percent."
    evidence: "verified .scratch/out/wyscout_rates.json (1826 matches, five leagues 2017/18)"
  - id: C-64
    thread: T-05
    text: "Metrica agrees with C-63 by pass length: under 10 metres 82 to 89 percent, 10 to 25 metres 92 percent, 25 to 40 metres 70 to 72 percent, and over 40 metres 58 percent."
    evidence: "verified .scratch/out/metrica2.json (2 matches, 25 fps tracking plus events)"
  - id: C-65
    thread: T-05
    text: "The duel is the most common contested action in football at 456 per match, of which 87 are aerial, and the engine models no duel at all."
    evidence: "verified .scratch/out/wyscout_rates.json (1826 matches, five leagues 2017/18)"
  - id: C-66
    thread: T-05
    text: "The ball sits above 1 metre for 12.4 percent of tracked frames in nine European top-league matches and 19.4 percent in twenty A-League matches, and above 2 metres for 7.6 and 10.6 percent, reaching 12 to 15 metres at its highest."
    evidence: "verified .scratch/data/skillcorner (20 A-League matches 2024/25)"
  - id: C-67
    thread: T-05
    text: "Ball height is standard in tracking data rather than exotic: kloppy types every tracking frame's ball position as a three-dimensional point and populates the height for Tracab, Sportec, StatsPerform, SkillCorner, PFF, Second Spectrum, Signality, and HawkEye."
    evidence: "verified kloppy v3.19.0 kloppy/domain/models/tracking.py:38"
  - id: C-68
    thread: T-04
    text: "Real phase-of-play data uses nine paired phases rather than two: build-up, create, finish, direct, transition, quick break, set play, chaotic, and disruption, each locked one-to-one to the opponent's high block, medium block, low block, and the matching defending phases."
    evidence: "verified .scratch/data/skillcorner (20 A-League matches 2024/25)"
  - id: C-69
    thread: T-04
    text: "The two phases differ by a measurable amount: in possession a team is 46 to 49 metres wide with a defensive line 34 to 37 metres up the pitch, and out of possession 36 to 38 metres wide with the line at 31 to 35 metres, while depth barely changes."
    evidence: "verified .scratch/out/metrica2.json (2 matches, 25 fps tracking plus events)"
  - id: C-70
    thread: T-02
    text: "An outfield player covers 9.6 to 10.3 kilometres per match, spends about 70 percent of the time below 2 metres per second, and sprints above 7 metres per second for 0.25 to 0.44 percent of the match, covering 100 to 200 metres at that speed."
    evidence: "verified .scratch/out/metrica2.json (2 matches, 25 fps tracking plus events)"
  - id: C-71
    thread: T-02
    text: "Peak sprint velocity has a narrow spread across a league: the tenth percentile is 26.6 kilometres per hour, the median 28.3, the ninetieth 29.9, and the fastest player 31.8."
    evidence: "verified .scratch/data/skillcorner (20 A-League matches 2024/25)"
  - id: C-72
    thread: T-02
    text: "Players cover more high-speed distance out of possession than in it, 1052 against 805 metres per 90 minutes for high-speed running and 261 against 212 metres for sprinting, so defending costs more at high intensity than attacking."
    evidence: "verified .scratch/data/skillcorner (20 A-League matches 2024/25)"
  - id: C-73
    thread: T-05
    text: "The minimum event fields that kloppy and floodlight agree on are the event type, the period and timestamp, the start coordinates and the end coordinates where the event has an end, the team identifier, the player identifier, and an outcome."
    evidence: "verified kloppy v3.19.0 kloppy/domain/models/event.py:686"
  - id: C-74
    thread: T-05
    text: "The ball is in play for 59 percent of recorded frames, about 55 minutes, and its speed runs to a median of 5.9 metres per second, a ninetieth percentile of 15.1, and a maximum near 57."
    evidence: "verified .scratch/out/metrica2.json (2 matches, 25 fps tracking plus events)"
  - id: C-75
    thread: T-03
    text: "The share of time the ball is above 1 metre differs between competitions, 12.4 percent in European elite matches against 19.4 percent in the A-League, so ball height is a measurable playing-identity signal rather than a constant."
    evidence: "verified .scratch/data/skillcorner (20 A-League matches 2024/25)"
  - id: C-76
    thread: T-05
    text: "Most possession is brief: a match holds about 475 possession chains, 67.8 percent of them carry no pass or one pass, the median chain lasts 2.9 seconds, and only one chain in 20 ends in a shot."
    evidence: "verified .scratch/out/metrica2.json (2 matches, 25 fps tracking plus events)"
  - id: C-77
    thread: T-06
    text: "Mood, personality, and player quality change a team's scoring rate within a match and never the spread around it, so goals stay Poisson given that rate, and C-12 and C-16 are mechanisms for the rate rather than for the variance."
    evidence: unverified
  - id: C-78
    thread: T-06
    text: "The realism test bands four things: the measured per-match rates in C-61, the scoreline distribution and home advantage in C-59 and C-60, pass completion as a curve by kind and by length from C-63 and C-64, and the physical output in C-70 and C-71."
    evidence: unverified
  - id: C-79
    thread: T-05
    text: "The duel is a first-class contested action with types for ground attacking, ground defending, aerial, loose ball, and tackle, resolved from both players' attributes, their mood, and the ball's height and speed, and it is the single place where player quality becomes an outcome."
    evidence: unverified
  - id: C-80
    thread: T-04
    text: "The manager authors two phases and the engine derives which of the nine paired phases it is in from its own state, so the tactic interface stays at two while the model runs nine."
    evidence: unverified
  - id: C-81
    thread: T-02
    text: "Every outfield role covers 26 to 43 metres of the pitch's length between its tenth and ninetieth percentile, attacking midfielders roam widest laterally at 42.9 metres while full-backs stay narrowest at about 30, and central midfielders sit nearest the ball at 18.7 to 19.6 metres and within 20 metres of it for 56 to 58 percent of the match."
    evidence: "verified .scratch/out/sc_player_frames.csv.gz (20 matches, 3.87M samples)"
  - id: C-82
    thread: T-04
    text: "The shape does not translate uniformly between phases: when a team wins the ball the back line advances about 3.5 metres while full-backs, wing-backs and wingers advance 9 to 12 metres, so the team stretches from the back rather than sliding."
    evidence: "verified .scratch/out/sc_player_frames.csv.gz (20 matches, 3.87M samples)"
  - id: C-83
    thread: T-04
    text: "The block tracks the ball at a fixed ratio, about 0.63 metres up the pitch for every metre the ball advances and 0.30 to 0.39 metres sideways for every metre it moves across, with the same slope in and out of possession; what changes between phases is width, 45.1 metres in possession against 35.4 out of it."
    evidence: "verified .scratch/out/sc_team_frames.csv.gz (20 matches, 1.76M team-frames)"
  - id: C-84
    thread: T-02
    text: "Players move about twice as fast within 5 metres of the ball as beyond 40 metres, and attackers show the steepest gradient, with a striker running 3.62 metres per second near the ball and 1.81 far from it against a centre-back's 2.86 and 1.87."
    evidence: "verified .scratch/out/metrica_roles.csv (2 matches at 25 fps)"
  - id: C-85
    thread: T-03
    text: "Club-to-club style varies far more than league-to-league: across 98 clubs pass accuracy spans 73 to 90 percent against 1.1 percentage points between leagues, pass volume varies 2.57 times, mean pass length spans 17.9 to 23.0 metres, and shot conversion spans 4.9 to 17.5 percent."
    evidence: "verified .scratch/out/wyscout_rates.json (1826 matches, five leagues)"
  - id: C-86
    thread: T-06
    text: "Shot conversion halves roughly every 5 metres: 51.9 percent inside 6 metres, 21.6 percent at 6 to 11, 12.2 percent at 11 to 16.5, 6.3 percent at 16.5 to 22, 2.8 percent at 22 to 30, and 1.8 percent beyond 30, measured over 40461 shots."
    evidence: "verified .scratch/out/wyscout_rates.json (1826 matches, five leagues)"
  - id: C-87
    thread: T-05
    text: "A player on the ball has an opponent within 5 metres for 56 percent of touches, with a median nearest opponent at 4.4 metres, and has on average only 1.16 teammates within 10 metres."
    evidence: "verified .scratch/data/sb360/f360.json (3366 freeze frames)"
  - id: C-88
    thread: T-05
    text: "High intensity concentrates into attacking sequences rather than spreading across the match: during goal-scoring sequences players average 3.16 metres per second and spend 8.8 percent of the time above 7, against a match average of 1.67 and 0.25 to 0.44 percent."
    evidence: "verified .scratch/data/lastrow (19 goal sequences at 20 fps)"
  - id: C-89
    thread: T-02
    text: "An outfielder covers 10.25 kilometres per ninety minutes on official Bundesliga tracking, and the role difference lives in intensity rather than volume: a centre-back covers 84 percent of a wing-back's total distance but only 46 percent of the wing-back's high-intensity distance, 276 metres against 595 per half."
    evidence: "verified .scratch/out/idsse_players.csv (7 DFL matches, 247 full player-halves at 25 fps)"
  - id: C-90
    thread: T-02
    text: "A player sprints above 7 metres per second for 0.7 percent of live time and runs below 2 metres per second for 45 percent of it, and no more than about one outfielder in twenty exceeds 5.5 metres per second at any instant, so top speed is an attribute the match almost never uses."
    evidence: "verified .scratch/out/idsse_players.csv (7 DFL matches, 247 full player-halves at 25 fps)"
  - id: C-91
    thread: T-02
    text: "Every outfield role runs faster without the ball than with it, 2.49 metres per second against 2.20 for centre-backs and 2.86 against 2.59 for defensive midfielders, so defending costs more running than attacking and it costs the deepest roles the most."
    evidence: "verified .scratch/out/idsse_players.csv (7 DFL matches, 247 full player-halves at 25 fps)"
  - id: C-92
    thread: T-04
    text: "Only 1.62 of twenty outfielders stand within 5 metres of the ball and 3.71 within 10, the ten-metre zone is numerically balanced at 1.84 attackers to 1.87 defenders, and 51.5 percent of live frames hold at most one player inside 5 metres, so the ball is uncontested more often than it is contested. Corrected in batch 20 after a position-code fix restored two wingers; the first version read 1.55, 3.55, 1.77 to 1.78 and 54.8."
    evidence: "verified .scratch/out/idsse_crowd.csv (2 Bundesliga matches, 154k live frames)"
  - id: C-93
    thread: T-04
    text: "Ground truth confirms the block law that C-83 measured on broadcast data: the centroid moves 0.558 metres up the pitch per metre of ball advance in possession and 0.555 out of it, 0.35 and 0.39 sideways, staying within 0.44 to 0.68 across 52 team-halves, while width changes from 46.1 metres in possession to 36.8 out and depth changes by under 3 metres."
    evidence: "verified .scratch/out/idsse_shape.csv (52 team-halves, 7 DFL matches)"
  - id: C-94
    thread: T-06
    text: "The ball sits above half a metre for 29.6 percent of live play, above one metre for 23.9 percent and above two metres for 17.1 percent, and it is in play for 55.4 percent of the match, so a strictly two-dimensional engine is wrong about a sixth of the live game."
    evidence: "verified .scratch/out/idsse_ball.csv (14 ball halves, provider-labelled)"
  - id: C-95
    thread: T-06
    text: "Ball speed on ground truth runs to a median of 6.47 metres per second, a ninety-ninth percentile of 24.9 and an extreme of 49.9, which confirms the earlier discard of a 57 metres per second reading as tracking noise and gives the engine a real ceiling in its place."
    evidence: "verified .scratch/out/idsse_ball.csv (14 ball halves, provider-labelled)"
  - id: C-96
    thread: T-03
    text: "Physical output separates neither divisions nor clubs: Bundesliga outfielders cover 110.0 metres per minute against the second division's 115.4, top speed and sprint share are identical between the divisions, and across fourteen team-matches the spread is 20 percent, against the 73 to 90 percent club spread C-85 measured on pass accuracy."
    evidence: "verified .scratch/out/idsse_players.csv (7 DFL matches, 247 full player-halves at 25 fps)"
  - id: C-97
    thread: T-04
    text: "C-82 is corrected by ground truth: the ordering survives, with wide midfielders advancing 7.4 metres and full-backs 5.5 against a centre-back's zero when the team wins the ball, but the magnitudes halve and the direction of the back line reverses, because the deepest outfielder drops 1.4 metres deeper while the centroid advances 2.5, so the team stretches rather than sliding forward."
    evidence: "verified .scratch/out/idsse_shape.csv (52 team-halves, 7 DFL matches)"
  - id: C-98
    thread: T-06
    text: "The official DFL live schema carries 31 event kinds and 139 distinct attributes at 1423 events a match, against the engine's four kinds carrying no player, no coordinates and no outcome, so X-15's estimate of fifteen kinds plus three fields understates the gap by half on kinds and by a factor of forty on fields."
    evidence: "verified .scratch/out/idsse_events.csv.gz (7 DFL matches, 9961 events)"
  - id: C-99
    thread: T-06
    text: "Whether a pass leaves the ground matters more than how far it travels: flat passes complete at 90.2 percent and high passes at 53.4, and a flat long pass completes at 68.6 percent while a high medium pass completes at 52.0, so the engine should branch on height before length."
    evidence: "verified .scratch/out/idsse_events.csv.gz (7 DFL matches, 9961 events)"
  - id: C-100
    thread: T-05
    text: "Pressure appears not to matter across all passes, moving completion by only seven points and not in order, because pass purpose is confounded with pressure; restricted to flat passes the curve is monotone and spans twenty points, from 79.8 percent inside two metres to 100 percent beyond twelve."
    evidence: "verified .scratch/out/idsse_sync.csv (1650 passes joined to tracking at 9 ms)"
  - id: C-101
    thread: T-02
    text: "Pass completion is a property of pitch position rather than of person: among comparable players in one match pair, flat-pass completion runs from 98.6 percent for goalkeepers and 95 for centre-backs down to 73.2 for centre-forwards, so a uniform passing attribute will make defenders too error-prone and forwards too accurate."
    evidence: "verified .scratch/out/idsse_sync.csv (1650 passes joined to tracking at 9 ms)"
  - id: C-102
    thread: T-05
    text: "The duel is the second most common on-ball event at 201.7 a match, which reconciles C-79's 456 because Wyscout logs one row per participant and the DFL logs one per contest, and it resolves as a turnover in only 29.4 percent of cases."
    evidence: "verified .scratch/out/idsse_events.csv.gz (7 DFL matches, 9961 events)"
  - id: C-103
    thread: T-05
    text: "The ground duel is sharply asymmetric: when the ball carrier wins, possession changes hands 6.4 percent of the time, and when the defender wins it changes 51.4 percent, while the aerial duel is nearly symmetric at 31.2 against 36.6, so a model with one win probability and an automatic turnover would roughly double real turnover rates."
    evidence: "verified .scratch/out/idsse_events.csv.gz (7 DFL matches, 9961 events)"
  - id: C-104
    thread: T-05
    text: "Fouls are a duel outcome rather than a separate mechanism: 10.6 percent of duels resolve as a foul, giving 21.4 a match against the 22.9 foul events the schema records, so an engine that models duels obtains fouls, free kicks and cautions without a foul generator."
    evidence: "verified .scratch/out/idsse_events.csv.gz (7 DFL matches, 9961 events)"
  - id: C-105
    thread: T-06
    text: "Only about a third of shots reach the goalkeeper, because 38.4 percent go wide and 22.6 percent are blocked by a defender first, so a shot model that resolves attacker against goalkeeper models the minority case."
    evidence: "verified .scratch/out/idsse_events.csv.gz (7 DFL matches, 9961 events)"
  - id: C-106
    thread: T-05
    text: "Every action family is a place where player quality becomes an outcome, and the duel is one of them: passes, through balls, positioning, off-ball movement, dribbles, shooting, goalkeeper actions, blocking and shielding, tackles and duels, and carrying and progressing the ball each need their own resolution."
    evidence: "person"
  - id: C-107
    thread: T-05
    text: "The duel looked dominant partly because the DFL schema files several families inside one element: take-ons at 14.6 a match with 65.7 percent success, shielding as retained control at 17.6, and beaten defenders at 7.3 all live inside TacklingGame, so its 201.7 contests a match are a container rather than a single action."
    evidence: "verified .scratch/out/idsse_events.csv.gz (7 DFL matches, 9961 events)"
  - id: C-108
    thread: T-05
    text: "Evidence now covers the pass, through ball, cross, shot, dribble, duel, shot block, positioning, and aggregate off-ball movement, while three families have no outcome evidence at all — the carry, which no schema read records as an event, the ten off-ball run types SkillCorner names but nothing has counted, and shielding outside a recorded duel."
    evidence: "verified .scratch/out/idsse_events.csv.gz (7 DFL matches, 9961 events)"
  - id: C-109
    thread: T-05
    text: "The full action set is 53 actions, 38 outfield and 15 goalkeeper; public data measures 39 in full, 10 partially and feints only thinly, and nothing public measures marking and cover assignments, keeper organisation, or keeper dive kinematics."
    evidence: "verified .scratch/out/ACTIONS.md"
  - id: C-110
    thread: T-06
    text: "Providers count the same action very differently — duels run at 64, 202 or 226 a match and through balls at 4.6, 15.6 or 44.1 depending on the provider — while providers that share a definition agree tightly, carries of at least 5 metres at 287 against 273, corrected in batch 20 from 271 and possession duration at 1.20 against 1.16 seconds, so every realism band must name the definition it copies."
    evidence: "verified .scratch/out/ACTIONS.md"
  - id: C-111
    thread: T-05
    text: "The carry is measured on three providers: 273 to 287 carries of at least 5 metres a match, corrected in batch 20 from 271 at about 4 metres per second, 79.9 percent kept, with longer carries faster and riskier, and centre-backs carrying the most because they receive with the most space."
    evidence: "verified .scratch/out/idsse_control_segments.csv and idsse_gk_positions.csv.gz (7 DFL matches)"
  - id: C-112
    thread: T-05
    text: "Pressure at the moment of receiving is the common input across actions: from no pressure to very high pressure, possession loss rises from 1.0 to 32.6 percent, pass completion falls from 93.4 to 67.0 percent, and one-touch play rises from 5.6 to 31.3 percent, and DFL tracking gives the same gradient from geometry alone, 93.4 percent kept with no opponent inside 10 metres against 60.9 inside 1 metre."
    evidence: "verified .scratch/out/sc_dynamic_events.csv.gz (20 SkillCorner matches) and ACTIONS.md"
  - id: C-113
    thread: T-05
    text: "Off-ball runs run at 495 a match across ten subtypes, and a run's danger and its completion move in opposite directions: safe runs such as coming short and dropping off are received 91 to 98 percent of the time when targeted and almost never lead to a shot, while runs in behind and to receive a cross are received 40 to 52 percent of the time and lead to a shot 20 to 29 percent of the time."
    evidence: "verified .scratch/out/sc_dynamic_events.csv.gz (20 SkillCorner matches) and ACTIONS.md"
  - id: C-114
    thread: T-05
    text: "A single pressing engagement is credited with a regain only 18.5 percent of the time, yet 80.6 percent of pressing chains end in a regain, and pressing in the opponent's third regains 32.6 percent against 25.4 in midfield, so the value of pressing is collective."
    evidence: "verified .scratch/out/sb_events.csv.gz (200 StatsBomb matches) and ACTIONS.md"
  - id: C-115
    thread: T-05
    text: "Bodies between the ball and the goal decide shots: with no defender in the shot cone a shot scores 12.4 percent of the time and is blocked 20.9 percent, with two it scores 5.4 and is blocked 40.4, and a shooter has an opponent inside 5 metres 88.2 percent of the time."
    evidence: "verified .scratch/out/sb360_actions.csv (4671 non-penalty shots with 360 frames)"
  - id: C-116
    thread: T-05
    text: "A keeper's save rate falls with shot quality from 89.1 percent below 0.05 xG to 27.2 percent above 0.4, two saves in three are not held — a fifth go behind and a fifth are parried into danger — and 87 percent of goals are conceded without a touch."
    evidence: "verified .scratch/out/sb_events.csv.gz (200 StatsBomb matches) and ACTIONS.md"
  - id: C-117
    thread: T-05
    text: "Keeper depth follows a measured rule: out of possession with the ball in the keeper's own half, depth is about 0.23 metres per metre of ball distance from goal, the keeper spends 42 percent of live time beyond the depth of the box, shifts 0.17 metres sideways per metre the ball moves across, and stands a median 2.2 metres off the line at the moment of a shot, where depth has no effect on conversion once shot quality is known."
    evidence: "verified .scratch/out/idsse_control_segments.csv and idsse_gk_positions.csv.gz (7 DFL matches)"
  - id: C-118
    thread: T-05
    text: "Keepers distribute 66 times a match between them at 73.5 percent, short passes and throws above 97 percent, punts at 29.1; short goal kicks are 45.3 percent of goal kicks and complete at 99.4 percent against 44.1 for long ones, and the 2019 goal-kick law halved the median goal kick from 65.6 metres in 2017/18 to 31.3 metres in 2020 to 2024."
    evidence: "verified .scratch/out/sb_events.csv.gz (200 StatsBomb matches) and ACTIONS.md"
  - id: C-119
    thread: T-05
    text: "Dribbles run at 27.4 a match at 53.0 percent success, success falls up the pitch from 61.5 percent in the own third to 48.2 in the final third, and role changes how often a player dribbles but not how often the dribble succeeds — wingers attempt six times as many as centre-backs at about the same rate as full-backs and forwards."
    evidence: "verified .scratch/out/sb_events.csv.gz (200 StatsBomb matches) and ACTIONS.md"
  - id: C-120
    thread: T-05
    text: "Shielding is common but barely coded: event feeds record 1.3 shields a match, while tracking shows about 80 possessions a match held against an opponent inside 2 metres, 10 of them stationary, with about two in three kept."
    evidence: "verified .scratch/out/idsse_control_segments.csv and idsse_gk_positions.csv.gz (7 DFL matches)"
  - id: C-121
    thread: T-05
    text: "The shooting decision is a steep threshold at the edge of the box: 87.7 percent of on-ball releases inside 6 metres are shots, 72.1 at 6 to 11, 35.3 at 11 to 16.5, 20.2 at 16.5 to 22 and 6.2 at 22 to 30, and pressure halves the shooting rate outside 11 metres; under pressure the share of releases that are passes falls from 85.5 to 52.9 percent as clearances, dribbles and dispossessions appear."
    evidence: "verified .scratch/out/gap_sb.txt (200 StatsBomb matches) and FINDINGS.md §16"
  - id: C-122
    thread: T-05
    text: "Players choose the safe open option more often than the dangerous one: over 70,059 passes with freeze frames, a four-term score — space around the receiver, clearance of the passing lane, distance, and forward progress — picks the real receiver 55.5 percent of the time against 14.9 by chance, and among SkillCorner's tracked options the safest is chosen 45.0 percent of the time and the most threatening 31.7, below the 36.0 chance rate."
    evidence: "verified .scratch/out/gap_sb.txt (200 StatsBomb matches) and FINDINGS.md §16; verified .scratch/out/gap_sc.txt (20 SkillCorner matches) and FINDINGS.md §16"
  - id: C-123
    thread: T-05
    text: "The match changes with the clock: minutes 75 to 90 score 41 percent faster than minutes 0 to 15 and added time 70 percent faster, passing tempo falls from 10.3 to 8.2 passes a minute and to 7.0 in added time, and cautions rise fivefold."
    evidence: "verified .scratch/out/gap_wy.txt (1826 Wyscout league matches) and FINDINGS.md §16"
  - id: C-124
    thread: T-05
    text: "The score changes behaviour once team strength is removed: a team trailing by one goal scores at 1.10 times its own expected rate and a team leading by one at 0.92, the trailing team's possession rises above its expectation from 2 points early to 12 points after the 90th minute, the leading team takes its throw-ins, goal kicks and free kicks at a median 22.0 seconds against 15.1 for the trailing team, and the team that scores first wins 69.4 percent of the time."
    evidence: "verified .scratch/out/gap_wy_state.txt (1826 Wyscout league matches, strength-adjusted) and FINDINGS.md §16; verified .scratch/out/gap_idsse_an.txt (7 DFL matches, tracking joined to events) and FINDINGS.md §16"
  - id: C-125
    thread: T-02
    text: "The late-match fall in running is dead time rather than tired legs: from minutes 0 to 30 to minutes 60 to 90, distance per minute falls 17 percent and high-speed running per minute 26 percent, but the ball is in play 27 percent less, and high-speed running per minute of live play does not fall at all; forwards are the one exception at minus 12 percent."
    evidence: "verified .scratch/out/gap_idsse_an.txt (7 DFL matches, tracking joined to events) and FINDINGS.md §16"
  - id: C-126
    thread: T-07
    text: "Substitutions follow the score: 2.85 per team per match in 2017/18 at median minutes 61, 72 and 82, a team that trails makes its first change at minute 57 against 66 for a team that leads, midfielders are half of the players taken off, and only 6.5 percent of changes happen at half time."
    evidence: "verified .scratch/out/gap_wy.txt (1826 Wyscout league matches) and FINDINGS.md §16"
  - id: C-127
    thread: T-04
    text: "Goals come from both short and long possessions: possessions of 0 to 2 passes give 34 percent of goals and possessions of 10 or more give 25; a ball won in the final third leads to a shot 27.0 percent of the time against 9.7 in the own third; 34.1 percent of the 200 open-play losses a match are regained within 5 seconds; and a SkillCorner quick break leads to a shot 46 percent of the time."
    evidence: "verified .scratch/out/gap_sb.txt (200 StatsBomb matches) and FINDINGS.md §16; verified .scratch/out/gap_sc.txt (20 SkillCorner matches) and FINDINGS.md §16"
  - id: C-128
    thread: T-02
    text: "Skill is small beside the situation for passing, shooting and saving and large for duels: after situation and binomial noise are removed, players spread by 3.6 points of pass completion against an 18.3-point situational spread, by 2.5 points of shot conversion and 2.4 of save rate with split-half correlations of only 0.17 and 0.14, but by 10.0 points in aerial duels against a 5.8-point situational spread and 7.6 in attacking ground duels."
    evidence: "verified .scratch/out/gap_wy.txt (1826 Wyscout league matches) and FINDINGS.md §16"
  - id: C-129
    thread: T-03
    text: "Team strength has a measured size: the best attack in a league scores about twice the league average and the worst about 0.6 times it, the home factor runs from 1.19 to 1.35, 71 to 84 percent of the spread in points per game is team quality, a top-quarter club beats a bottom-half club 71.0 percent of the time and loses 10.4, and evenly matched games draw 4 to 6 points more often than two independent Poisson scorers predict."
    evidence: "verified .scratch/out/gap_wy.txt (1826 Wyscout league matches) and FINDINGS.md §16"
  - id: C-130
    thread: T-03
    text: "Club style has two layers: one axis carries 59 percent of style variance and is mostly strength, with possession correlating 0.74 with points per game, while crosses and take-ons are almost independent of strength; and the opponent sets match possession as much as the club does, since match possession equals 0.487 plus 0.949 times the difference in season means plus 0.027 at home, and a club's possession swings 10.3 points from match to match against a 7.2-point spread between clubs."
    evidence: "verified .scratch/out/gap_wy.txt (1826 Wyscout league matches) and FINDINGS.md §16"
  - id: C-131
    thread: T-04
    text: "Pressing has measured triggers and the block a measured shape: a player receiving a forward pass is engaged 88.5 percent of the time against 59.5 after a backward pass, 78 percent when wide against 56 in the centre, and 82 percent against a low block; the defending block keeps a near-constant length of about 30 metres, its last defender advancing 0.49 metres and its midfield line 0.61 per metre of ball advance."
    evidence: "verified .scratch/out/gap_sc.txt (20 SkillCorner matches) and FINDINGS.md §16; verified .scratch/out/gap_idsse_an.txt (7 DFL matches, tracking joined to events) and FINDINGS.md §16"
  - id: C-132
    thread: T-04
    text: "Marking tightens near goal and hands over quickly: the nearest defending outfielder stands a median 2.4 metres from an attacker in the box-depth zone against 6.3 metres far from goal, and the nearest defender is still the same player 5 seconds later 64.0 percent of the time overall but only 51.3 percent near goal and 37.8 percent over 10 seconds."
    evidence: "verified .scratch/out/gap_idsse_an.txt (7 DFL matches, tracking joined to events) and FINDINGS.md §16"
  - id: C-133
    thread: T-05
    text: "On the ground, local numbers decide duels: with all 1,412 DFL duels joined to tracking, the defender wins 42.7 percent when outnumbered within 5 metres, 44.2 when level, 53.2 with one extra team-mate and 66.7 with two, while in the air support makes no difference once the defender is not outnumbered."
    evidence: "verified .scratch/out/gap_idsse_an.txt (7 DFL matches, tracking joined to events) and FINDINGS.md §16"
  - id: C-134
    thread: T-04
    text: "Set pieces are first-contact contests: a corner leads to a goal 3.45 percent of the time and to a shot 38.0 percent, the attacking team wins first contact on 37.2 percent of long corners and then shoots 68.9 percent of the time against 20.5 when the defenders win it, corners, free kicks and penalties together give 36 percent of goals, and a direct free kick scores 3.2 percent of the time."
    evidence: "verified .scratch/out/gap_sb.txt (200 StatsBomb matches) and FINDINGS.md §16"
  - id: C-135
    thread: T-05
    text: "Loose balls are a real phase: a clearance is controlled next by the clearing team only 26.5 percent of the time, the team of a player who wins a header controls the next ball only 48.0 percent against 39.3 for the loser's team, and 6.2 percent of shots are rebounds within 5 seconds that convert at 16.4 percent against 11.3."
    evidence: "verified .scratch/out/gap_sb.txt (200 StatsBomb matches) and FINDINGS.md §16"
  - id: C-136
    thread: T-03
    text: "Quality scales as small execution gaps in every family: seven DFL matches show no measurable gap between the Bundesliga and the 2. Bundesliga, while inside one league the top quarter of clubs beats its situational expectation by 1.7 points of pass completion, 2.0 of shot conversion and 2.5 in duels and take-ons, and the bottom quarter falls short by 1.2, 2.1 and 1.2."
    evidence: "verified .scratch/out/gap_wy.txt (1826 Wyscout league matches) and FINDINGS.md §16; verified .scratch/out/gap_div.txt"
  - id: C-137
    thread: T-06
    text: "The realism test has measured joint bands: goals are Poisson once the club is known, with a within-club variance-to-mean ratio of 0.99, but shots are over-dispersed at 1.64 and crosses at 3.03, one team's shots run from 6 to 17 between the 10th and 90th percentiles, shots and goals correlate at only 0.32, and home and away shot counts correlate at minus 0.34."
    evidence: "verified .scratch/out/gap_wy.txt (1826 Wyscout league matches) and FINDINGS.md §16"
  - id: C-138
    thread: T-05
    text: "The ball is dead for 42.2 minutes a match, and each restart has a measured stoppage: a throw-in a median 13.8 seconds, a goal kick 23.2, a corner 31.8, a free kick 32.5, and the kick-off after a goal 68.2."
    evidence: "verified .scratch/out/gap_idsse_an.txt (7 DFL matches, tracking joined to events) and FINDINGS.md §16"
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
    state: rejected
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
    text: "Flight arcs, heading duels, and spin on every ball update fit inside the 10 percent CPU tripwire that C-18 records. C-66 measures the ball above 1 metre for 12 to 19 percent of frames, so the aerial path is not a rare branch."
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
    state: confirmed
  - id: A-29
    thread: T-02
    text: "Building match-rules and tactics-and-ai against a player record that will change under them costs less than delaying both slices to change the record first."
    state: named
  - id: A-30
    thread: T-06
    text: "Goal-scoring in the engine can reproduce a Poisson distribution of total goals while mood, personality, and player quality all move the scoring rate."
    state: named
  - id: A-31
    thread: T-05
    text: "The engine can model the duel as a first-class contested action at roughly 456 per match without breaching its performance budget."
    state: named
  - id: A-32
    thread: T-04
    text: "Nine paired phases can be derived from the engine's own state rather than authored by the manager, so the tactic interface stays at two phases while the model runs nine."
    state: named
  - id: A-33
    thread: T-06
    text: "Testing the physical bands in C-70 and C-71 requires the engine to report per-player distance and speed, which the engine does not report today."
    state: named
  - id: A-34
    thread: T-02
    text: "A duel resolved from attributes, mood, and ball state reproduces the real completion curve in C-63 as an emergent result, so no per-pass-kind probability table is authored by hand."
    state: rejected
  - id: A-35
    thread: T-04
    text: "A team's block can be made to track the ball at the fixed ratio C-83 measures, without explicit per-player marking logic in the tick loop."
    state: named
  - id: A-36
    thread: T-06
    text: "Ball height costs the engine little, because crates/engine/src/ball.rs already carries a third axis and a third velocity component, so the work is in the rules rather than in the state."
    state: named
  - id: A-37
    thread: T-06
    text: "The engine can carry a provider-grade event contract at the core five fields — player, team, x, y, outcome — on every on-ball event, adding per-kind qualifiers only for the kinds the realism tests read, rather than reproducing all 139 attributes C-98 counts."
    state: named
  - id: A-38
    thread: T-05
    text: "Each action family gets its own resolution model, with the action kind and the situation as the first inputs and the player's attributes and mood as the second, so no single family carries player quality for the rest."
    state: named
  - id: A-39
    thread: T-06
    text: "The realism test bands each action family against one named provider definition — StatsBomb for most on-ball and keeper actions, SkillCorner for runs and pressing, DFL tracking for carries, shielding and keeper positioning — rather than mixing definitions within a band."
    state: named
  - id: A-40
    thread: T-05
    text: "A hand-built choice model — the release mix by zone and pressure, a shooting threshold by distance, and a four-term receiver score with a pressure modifier — reproduces C-121 and C-122 closely enough without a learned model."
    state: named
  - id: A-41
    thread: T-02
    text: "Attribute effects sized to C-128's measured spreads, about plus or minus 7 points of pass completion and 5 of shot conversion between the best and worst players, still read to a manager as real differences between players."
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
  - id: X-13
    threads: [T-02, T-06]
    text: "C-12 wants weak players to swing more and C-16 wants mood to move decisions, and both widen match-to-match variance; C-59 measures real football's total-goal distribution as Poisson with a variance-to-mean ratio of 1.035, so the observed variance is already fully explained by the scoring rate alone. Extra variance therefore moves the engine away from real football rather than towards it, unless the mechanisms shift the rate rather than the spread. Resolved: C-77 takes exactly that route, so C-12 and C-16 move the scoring rate and the distribution stays Poisson. A-30 carries the cost."
    state: resolved
  - id: X-14
    threads: [T-05, T-06]
    text: "C-61 measures the real spread of per-match rates as roughly one percentage point for pass accuracy and two shots for shot volume across five leagues, and C-06's planned bands are goals 2.4 to 3.2, shots 8 to 16 per team, and possession 35 to 65 percent. The planned bands are several times wider than the real spread, so the test as planned passes an engine that is visibly wrong. Resolved: C-78 replaces the three bands with four measured families, and A-06 is rejected with them."
    state: resolved
  - id: X-15
    threads: [T-01, T-05]
    text: "C-46 defers the event-contract change to the realism programme and prices it as adding a player identifier; C-62 and C-73 measure the real size of that change as four event types becoming roughly fifteen, plus a player identifier, coordinates, and an outcome on every event. The deferral was priced as one field and is a redesign of the contract."
    state: open
  - id: X-16
    threads: [T-05, T-06]
    text: "C-78 bands the physical output and C-79 makes the duel the place where player quality becomes an outcome, and both need the engine to report per-player data that C-44 records the event contract does not carry. X-15 already reprices that change and C-46 still defers it to the realism programme, so the test the board has just chosen cannot run against the engine the board has agreed to ship."
    state: open
  - id: X-17
    threads: [T-03, T-06]
    text: "C-61 bands league aggregates to about one percentage point and C-85 measures the club-to-club spread on the same statistic at 17 points; C-78 adopted the league bands alone, so the test as chosen passes an engine in which every club plays identically. The realism test needs a club-spread band as well as a league band, and nothing on the board yet produces club-to-club variation of that size. Batch 20 supplies the band: C-129 sizes team strength and C-130 splits club style into a strength axis and a pure-style layer, with the opponent setting possession as much as the club. What would produce that variation inside the engine is still open."
    state: open
  - id: X-18
    threads: [T-02, T-04]
    text: "C-82 and C-97 measure the same quantity and disagree: SkillCorner gives the back line 3.5 metres of advance and the wide roles 9 to 12, the DFL multi-camera feed gives the back line minus 1.4 and the wide roles 5.4 to 7.4. The likely cause is the possession label, because SkillCorner marks controlled phases while the DFL field marks ball ownership on every frame including contested ones. Resolved in favour of C-97, which is ground truth and provider-labelled; C-82 is kept for its ordering only."
    state: resolved
  - id: X-19
    threads: [T-05, T-06]
    text: "C-100 and C-101 both say the same thing about where outcome comes from, and it argues with how the board has framed player quality throughout. Completion is set by the kind of action and the situation first, and by the player second; a uniform per-player attribute produces the wrong answer by role even when its values are right. C-79 and A-34 already bet on emergence for the duel. Nothing on the board yet says the same for the pass, and C-61's realism test cannot tell the two models apart. Batch 20 measured it: C-128 puts the situational spread of pass completion at 18.3 points and the player spread at 3.6, and the same holds for shooting and saving, while duels are player-heavy. The test can now tell the two models apart by banding the per-player residual spread. Resolved in favour of situation first and player second for the pass, the shot and the save; the duel keeps a larger player share."
    state: resolved
  - id: X-20
    threads: [T-05]
    text: "C-79 made the duel the single place where player quality becomes an outcome, and C-106 says every action family is such a place. C-99 to C-101 already show the pass has its own surface that a duel model does not produce. Resolved in favour of C-106: C-79 keeps the duel as a first-class contested action but loses the word single, and A-34 is rejected."
    state: resolved
  - id: X-21
    threads: [T-02, T-06]
    text: "Batch 2 set the strong-versus-weak gap as errors, ceiling and consistency together, and B-01 builds attributes a manager is meant to feel. C-128 measures that in real football the player moves pass completion by about 3.6 points, shot conversion by 2.5 and save rate by 2.4 once the situation is removed, and that finishing and saving barely repeat within a season. An engine faithful to C-128 may make attributes feel like they do not matter; an engine that makes them matter will fail C-128."
    state: open
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
  - rev: 3
    at: "2026-09-22T15:04:24Z"
    trigger: resume
    because: "session 3 opened to study real 2D tracking and event datasets against the board"
    changed: "sessions bumped to 3; the board kept as batch 12 left it pending the study"
---

# Brainstorm: realism additions to players, teams, tactics, and the match engine

## The Brainstorm

You brought one thought: the in-progress plans for Touchline build a lawful, tactical, calibrated match, and you want to know what they still miss on realism. Recorded history answered part of that before any question was asked. The engine bounces the ball off the touchlines like walls, and the `match-rules` slice already owns that gap. The reads that followed found the quieter places: 37 attributes with no personality, a player record with no age and no condition, a club with two kit colours and no reputation, a tactics model that stops at Tier 2, and a realism test that stops at three aggregate bands.

Seven batches have run and the answer to the original question is now long. All four absent player fields matter. The position code becomes a competence rating per position. The strong-versus-weak gap is errors and ceiling and consistency together. A player carries a mood the match moves, a team carries a mood of its own, personality decides how hard either lands, and team mood shifts behaviour inside the tactic and gates errors without ever changing the tactic. All four club absences matter, and the squad gains leadership, per-pair partnerships, and a cohesion value measured twice — once for the squad, again for the eleven on the pitch. The tactic splits into two phases. Set pieces become a duel of routines. Tier 3 is per-player instructions, opposition instructions, and manager-set triggers.

Batches 5 to 7 changed the board's shape rather than filling it. You rejected A-07: form is not an asserted number in a team file, so a season layer came into scope and T-08 opened for it. Both managers carry attributes, and a manager weak at tactics has their instructions land weaker, which makes the player's own competence part of the simulation and A-11 the risk inside it. The referee became a full official with a home leaning, pitch wear and weather now interact, injuries carry a severity and a risk driven by tackles and fatigue, and the ball gained arcs and spin on the height the engine already had. Then you rejected A-01 as well: three aggregate bands are not enough, so the test gains shape measures and a band per condition band.

Session 2 opened on T-08, the thinnest thread, and turned it into the largest thing on the board. The season layer took every addition offered: a league table, season-long availability with suspensions and carried fitness, a full club calendar with cups and rotation, and then, in batch 9, transfers and finances as well. Form is computed twice, for the team and for each player. A new save generates one past season of history, so partnerships and reputation start earned rather than blank, which rejected A-09. A bounded read rejected A-18 too — the match event carries a team identifier, a score, and four event types, and no player identifier at all, so per-player form has no source today.

Batch 9 spent the rest of the session paying for batch 8. You chose to change the event contract when B-07 starts rather than now, although every one of its thirteen consumers is still unbuilt, so A-20 carries a migration this board can see coming and A-12 is rejected: the season layer will change the engine's contract after the fact, by decision rather than by accident. You split the work in two — engine realism and the season layer — which resolved X-08 and rejected A-19. Then you took transfers and finances, so C-49 makes this a football-management game rather than a match engine with a season around it, and C-25's exclusion is now a boundary the second programme crosses on purpose.

Batch 10 asked what the board had become and closed the two gaps batch 9 opened. Your reading is C-50: Touchline is one product, a football-management game, and the match engine is its first component rather than the whole thing. The season test takes everything on offer — league-table shape, market and wage shape, and a watched season a human reads and judges — which resolves X-09. The market gets both of its disagreement sources, noise from scouting quality and weights from club identity, which resolves X-10 and rejects A-22.

Batch 11 answered both of the contradictions batch 10 created, and it did so by leaving work alone. C-53 keeps the in-flight charter exactly as it is, and C-54 keeps the engine's definition of done at a playable match, because C-48's split already makes the exclusions true of that workflow while being false of the product; A-23 is confirmed and X-12 closes. Then A-16 fell. C-55 puts a human in front of matches as well as seasons, as a milestone review rather than a gate on every build, which closes X-11 and does something larger besides.

Batch 12 closed three of the four that were left. C-56 sends the player record the same way as the event contract — it changes when the realism programme starts, and A-29 carries the cost of building match-rules and tactics-and-ai against a record that will move under them; B-08's question is answered and that card needs no investigation run. C-57 scopes C-18's tripwire to the engine build, so the realism programme measures against the shipped engine and picks a budget it can live in rather than inheriting one written for a different purpose. C-58 bands the spread as well as the average, which finally measures the thing C-12 and C-16 added rather than the thing they left alone.

Session 3 left the loop and went to the data. Seven sources were opened and four were mined: 1826 matches of Wyscout event data from five leagues, two Metrica games of synchronised tracking at 25 frames a second, twenty A-League and nine European matches of SkillCorner tracking, and the kloppy and floodlight models read from source. Eighteen claims now carry measurements rather than intuition, and most of them agree with the board. The ball really is in the air — above a metre for 12 to 19 percent of tracked frames — and ball height is standard in tracking data rather than exotic, so C-36's aerial model and C-32's head start are both vindicated. Two phases really are different: a team is 46 to 49 metres wide in possession and 36 to 38 out of it, with its defensive line 5 metres higher. Defending costs more at high intensity than attacking. And a player sprints for a quarter of one percent of a match.

Three findings argue with the board instead. Real phase-of-play data uses nine paired phases, not two, so C-26 is right in kind and thin in degree. The duel is the single most common contested action in football at 456 a match, and the engine models none. And real football's goal distribution is Poisson, with a variance-to-mean ratio of 1.035 — the spread is already fully explained by the scoring rate, which is the opposite of what C-12 and C-16 assume when they widen variance to add realism.

Batch 14 then applied what the data said. C-77 turns C-12 and C-16 into mechanisms for the scoring rate rather than for the spread, which resolves X-13 and keeps the engine's goal distribution Poisson. C-78 replaces the three planned bands with four measured families — the per-match rates, the scoreline distribution, pass completion as a curve by kind and length, and the physical output — which resolves X-14 and rejects A-06 with it. C-79 makes the duel a first-class contested action and the one place where player quality becomes an outcome, with A-34 betting that the real completion curve then emerges instead of being authored. C-80 keeps the manager's interface at two phases while the engine derives which of the nine it is in.

Batch 15 went back to the data for the movement the board had never measured: where each role stands, how far it travels, and how it moves relative to the ball. C-81 maps the roles, C-82 finds that the back line advances 3.5 metres between phases while full-backs and wingers advance 9 to 12, and C-83 gives the law two providers agree on — the block slides 0.63 metres per metre of ball advance and 0.3 to 0.4 sideways, the same in both phases, with only width changing. C-84 says everyone runs about twice as fast near the ball and attackers steepest. C-86 gives the shot surface, halving every 5 metres. C-87 and C-88 add the two facts an off-ball model needs: a player on the ball is pressed inside 5 metres for 56 percent of touches, and intensity concentrates into attacking sequences at eight times the match rate.

C-85 is the one that changes a decision already taken. Across 98 clubs, pass accuracy spans 73 to 90 percent, against 1.1 points between leagues, and pass volume varies 2.57 times. X-17 is the consequence: C-78 adopted league bands alone, so the chosen test would pass an engine in which every club plays identically.

Batch 16 went to the best free data that exists. The Bundesliga IDSSE set is official DFL tracking from a multi-camera system that follows every player across the whole pitch, seven matches at 25 frames a second, and it supplies each player's speed and acceleration itself. That removes this study's largest weakness, because the earlier per-role kinematics were differentiated and smoothed by hand from raw positions.

Most of what it found confirms the board. C-93 reproduces C-83's block law almost exactly — 0.558 metres of centroid advance per metre of ball advance in possession against 0.555 out of it, 46.1 metres of width against 36.8 — so two providers on two continents, using two tracking technologies at two frame rates, now agree on the same law. That is the firmest result on this board, and it is cheap to build, because it is one gain applied to a team offset rather than per-player marking logic. C-94 settles the aerial question: the ball is above a metre for 24 percent of live play and above two metres for 17 percent, and A-36 notes that the engine's ball already carries the third axis. C-95 replaces the discarded 57 metres per second with a real ceiling.

Three findings are new. C-90 says a player sprints for 0.7 percent of live time and that fewer than one outfielder in twenty is running hard at any instant. C-91 says every outfield role runs faster without the ball than with it, which means a stamina model that drains on possession has the sign backwards. C-92 says only 1.55 of twenty outfielders stand within 5 metres of the ball, the zone around it is numerically balanced, and the ball is uncontested more than half the time — which is what makes the duel frequent without being continuous.

Two findings argue with the board. C-97 corrects C-82: the ordering of who advances holds, but the magnitudes halve and the back line moves the other way, dropping 1.4 metres deeper while the centroid advances 2.5, so the team stretches instead of sliding. X-18 records that disagreement and resolves it in favour of ground truth. And C-96 hardens X-17 into something larger than a gap in a test. Physical output does not separate the two German divisions at all — the first division runs less — and separates clubs by only 20 percent, while pass accuracy separates clubs by 17 points. A realism test built on physical bands is not a weak test of playing identity. It is not a test of playing identity.

Batch 17 read the event files that came with the same seven matches, and then joined them to the tracking. The two clocks are the same clock, so the join lands within nine milliseconds, which means every pass can be asked a question the board has never been able to ask: who was standing near the player who made it.

C-98 prices X-15 properly. The official DFL schema carries 31 event kinds and 139 attributes at 1423 events a match. X-15 guessed fifteen kinds and three fields. A-37 proposes the way out, which is to carry the core five fields on every on-ball event and add qualifiers only for the kinds a test reads.

C-99 is the single most useful measurement in the set. Whether a pass leaves the ground matters more than how far it travels — flat 90.2 percent against high 53.4, and a flat long pass beats a high medium one. C-102, C-103 and C-104 then rebuild the duel. It runs at 201.7 a match, which reconciles C-79's 456 as a counting convention, and it turns the ball over in only 29.4 percent of cases. The ground duel is sharply asymmetric: a carrier who wins keeps the ball 94 percent of the time, a defender who wins takes it half the time. And every foul in the match comes out of a duel, so the duel pays for the foul model too.

C-100 records a trap worth remembering. Measured across all passes, pressure looks irrelevant, moving completion seven points and not in order. That is an artefact of mixing two kinds of pass, because an unpressured high ball is a deliberate long pass into traffic. Restricted to flat passes the curve is clean and monotone across twenty points.

C-101 is the finding that argues. Flat-pass completion runs from 98.6 percent for goalkeepers to 73.2 for centre-forwards, among players of comparable quality in the same matches. Completion is set by where you are standing, not by who you are. X-19 is the consequence, and it reaches further than the pass: the board has treated player quality as an attribute the engine applies, while the data says outcome comes from the action kind and the situation first and the player second. C-79 and A-34 already made that bet for the duel. Nothing yet makes it for the pass, and the chosen realism test cannot tell the two models apart.

Batch 18 was your correction, and the board needed it. Since batch 14 the board had treated the duel as the single place where player quality becomes an outcome, and every later batch leaned on that. You named the rest of the game — passes, through balls, positioning, off-ball movement, dribbles, shooting, goalkeeper actions, blocking and shielding, carrying and progressing the ball — and C-106 records that each is a place where quality turns into outcome. X-20 resolves in your favour: the duel stays a first-class contested action but loses the word single, and A-34, which bet that a duel model would reproduce the pass completion curve, is rejected, because C-99 to C-101 already showed that the pass has a surface of its own.

C-107 explains how the error happened. The DFL schema files take-ons, shielding and beaten defenders inside the same element as the tackle, so its 201.7 contests a match are a container of several families, and counting the container as one action inflated its weight. A-38 replaces the old framing: each family gets its own resolution model, with the action kind and the situation first and the player second. C-108 then lays the families against the evidence. Most are measured. Three have nothing — the carry, the off-ball run types, and shielding outside a recorded duel.

Batch 19 did what batch 18 asked for. You asked for every in-game action and every goalkeeper action, measured. C-109 is the list: 53 actions, 38 outfield and 15 goalkeeper, measured against 1941 Wyscout matches, 200 StatsBomb matches with freeze frames, 20 SkillCorner matches of dynamic events, and the seven DFL matches of tracking. Thirty-nine are measured in full. Three have no public evidence at all — marking assignments, keeper organisation, and the keeper's dive.

C-110 is the warning that comes with it. Providers count the same action very differently, so every band must name the definition it copies, and A-39 proposes which provider owns which family. Where the definitions match, the providers agree to within a few percent.

The families the board had never measured now have numbers. C-111 measures the carry. C-120 finds shielding common but almost never coded. C-113 counts ten kinds of off-ball run and finds the trade-off an off-ball model must reproduce: the more dangerous the run, the less often the pass arrives. C-114 finds the value of pressing is collective. C-115 shows the shot is decided by bodies in the cone. C-116, C-117 and C-118 turn the keeper from a save percentage into three behaviours — a save rate that falls with shot quality and rarely holds the ball, a depth rule that tracks the ball, and a distribution game of 66 passes a match that the 2019 goal-kick law reshaped. C-112 runs under all of it: pressure at the moment of receiving moves almost every outcome in the game.

Batch 20 asked what a match model still lacks once every action is measured, and then went and measured it. The answer was ten gaps, and nine are now filled. C-121 and C-122 are the choice model the engine never had: a shooting threshold at the edge of the box, and a receiver score in which space and a clear lane beat threat, because real players pick the safe open option. C-123 and C-124 put the clock and the score into the match, and C-124 offers one mechanism for the extra draws C-129 finds — the leading team slows and the trailing team pushes. C-125 turns fatigue on its head: the late fall in running is the ball being dead for longer, not legs failing. C-126 ties the manager's substitutions to the score.

C-127 gives the possession structure, and C-131 to C-133 give the defence: pressing triggers, a block of constant length that slides with the ball, marking that tightens near goal and hands over within seconds, and ground duels that the side with more bodies nearby wins. C-134 and C-135 measure set pieces and loose balls as contests of first contact and second balls. C-129, C-130 and C-136 size team strength, club style and the quality gradient. C-137 and C-138 give the test its joint bands and its dead time.

C-128 is the result that argues with the board. Once the situation is removed, the player moves pass completion by about 3.6 points, and finishing and saving barely repeat within a season. That resolved X-19 in favour of situation first. It also opened X-21, because a management game needs attributes a manager can feel. The data also exposed a bug: two wingers had been missing from one tracked match, so batch 20 corrects C-92, C-110 and C-111.

Four contradictions are open and they point the same way. X-15 reprices C-46: the deferred event-contract change is four event types becoming roughly fifteen plus three new fields on every event, not one field. X-16 is what batch 14 added to that bill — the physical bands and the duel both need per-player data the engine does not report, so the test this board just chose cannot run against the engine this board agreed to ship. X-01 is resolved in instrument but not in answer, and it is no longer counted among the open four because C-55 gave it a check. X-17 now carries C-96's weight as well as C-85's, and batch 20 gave it a measured band. X-19 is resolved. X-21 is new and sits beside X-17: the data says players differ less than a manager expects. Eight candidates stand as written from session 1, and every one of them now has real numbers to build against.

*Sessions: 3 | Batches: 20 | Threads: 8 live · 0 parked · 0 routed · 0 dropped | Candidates: 8*

## Threads

### T-01 — Realism gaps left by the in-progress plans
**State:** live

The framing thread, and batch 5 turned it into a scoping thread. C-07 records your reading: three at once. C-08 and C-25 are the verified constraints — the workflow sits at `stream-protocol`, and the shape excludes leagues, seasons, competitions, transfers, and finances. C-31 is the decision: every idea here belongs to a second workflow, a realism programme separate from the engine build. A-02 is confirmed, because nothing on this board removes planned scope. A-03 stays named and X-01 is A-03 failing. Session 2 added C-45: the stream-protocol slice is complete and four slices depend on it, so this thread carried two after-the-fact contract changes rather than one. Batch 9 settled one of them. C-46 changes the event contract when B-07 starts and not before, which resolved X-07 and left X-05 and the player record still open, so B-08 now gates the player record alone. C-48 is the other decision: the work splits into two programmes, engine realism for B-01 to B-06 and the season layer for B-07, which resolved X-08 and amends C-31 rather than replacing it. Batch 10 then settled what all of it is for. C-50 reads the whole board as one product, a football-management game with the match engine as its first component. Batch 11 decided what that costs the work in flight, and the answer is nothing: C-53 keeps the charter and its exclusions as they stand, C-54 keeps the engine done at a playable match on the sixteen-slice roster, and A-23 is confirmed because C-48's split makes both descriptions true at once. X-12 closes on that. The residual cost is recorded rather than removed — a person who reads the engine workflow alone will take the engine for the whole product.

### T-02 — Players — the missing person
**State:** live

Ten claims, four verified against code. The model today is a number vector with a label: C-02, C-03, and C-09 say a player is 37 values, one position code, and nothing about the person. C-10 keeps all four absences in scope. C-11 replaces the position code with a competence rating per position, and it is the one change on this board that alters the shape of the player record rather than extending it, which is why X-05 exists. C-12 makes the strong-versus-weak gap three-dimensional and warns that any one dimension alone reads as a flat multiplier. C-13 adds performance and personality attributes, bounded by C-17 at 13 free slots inside four fixed groups. C-14 and C-16 carry the mechanism. A-04 asks whether a generator can invent believable multi-position ratings; A-05 assumes personality fits the 1-to-100 scale. Batch 12 settled the timing: C-56 changes the record when the realism programme starts, following C-46's precedent, and A-29 is the assumption that migrating match-rules and tactics-and-ai later beats delaying them now. Session 3 measured the physical player this thread describes. C-70 gives the shape of a match: 9.6 to 10.3 kilometres covered, 70 percent of the time below 2 metres per second, and sprinting for a quarter of one percent of it. C-71 shows how narrow real ability is — peak sprint velocity runs from 26.6 to 29.9 kilometres per hour between the tenth and ninetieth percentile of a whole league — which bounds how far apart C-12 may place a strong player and a weak one. C-72 is the counter-intuitive one: high-speed running is higher out of possession than in it, so defending is the expensive phase. C-52 added the other half of what a player is — a club sees a noisy estimate of the player's attributes rather than the exact numbers, weighed by its own identity, which is where B-07's market finds its disagreement. X-02, X-03, and X-05 are all resolved now, and the thread is party only to X-01.

### T-03 — Teams — the missing club
**State:** live

Seven claims. C-04 is the verified starting point: two clubs differ only in their players and every fixture is played on neutral ground. C-23 answers with all four absences — home ground and home advantage, reputation and stature, squad familiarity per tactic, and a club playing identity. C-19 and C-20 say what moves team mood, from match events through to a club's reputation for late goals. C-22 bounds the effect to behaviour inside the tactic and to the error rate. C-24 gives the squad leadership, per-pair partnerships, and cohesion at two scopes. A-07 is rejected, so C-20's inputs now come from T-08 rather than from asserted fields. A-09 is rejected as well: session 2 answered it, and C-42 says the season layer generates the shared history at save creation rather than inventing partnerships without one. The consequence is a dependency this thread did not carry before — B-02's partnerships read as believable on day one only once B-07 supplies the backstory.

### T-04 — Tactics — beyond Tier 2
**State:** live

Four claims, and you took the expensive option on every one. C-05 is the verified starting point. C-26 splits the tactic into two phases, so the formation becomes two formations and the transition between them needs timing. C-27 makes a set piece a duel: designed routines with assigned takers against a defensive routine. C-28 defines Tier 3 as per-player and opposition instructions, and you added manager-set triggers, which the batch did not offer. A-10 is the assumption under the trigger: it lives in the rule pack and the change queue, so no conditional tactic logic enters the tick loop. C-23's squad familiarity attaches here, and there are now two phases to be familiar with. Session 3 both confirmed and enlarged C-26. C-69 measures the difference the two phases make — 46 to 49 metres wide in possession against 36 to 38 out of it, with the defensive line 5 metres higher and the depth barely moving — so the phase split is real and quantified. C-68 says it is also too small: real phase-of-play data uses nine paired phases, from build-up against a high block through to transition and chaos, each locked one-to-one to the opponent's. A-32 is the way out that keeps the tactic interface simple — derive the nine phases from the engine's own state and let the manager still set two.

### T-05 — Match engine — beyond the laws
**State:** live

Six claims after batch 6, and again you took the fullest option each time. C-18 makes this thread the cost ledger: the tactics-and-ai slice fails at 10 percent CPU or 25 percent memory over baseline, and every addition on every other thread lands against it. C-32 is the useful surprise — the ball already carries a three-dimensional position and velocity, so height exists today and nothing uses it. C-33 makes the referee a full official, down to a home leaning the crowd can move, and A-13 is the risk inside that. C-34 makes pitch wear and weather interact. C-35 gives an injury a severity and drives its risk from hard tackles, fatigue, and collisions, which finally gives the aggression attribute teeth; A-15 records what C-35 did not settle, which is whether a manager may keep a hurt player on. C-36 uses the height that C-32 found: arcs, heading duels, keeper claims, and spin. A-14 assumed all of that fits the tripwire, and X-02 and X-06 both said it may not. Batch 12 resolved the question rather than the cost: C-57 scopes C-18's tripwire to the engine build, and the realism programme measures against the shipped engine with a budget of its own. A-27 is what that rests on — the budget is chosen before the additions exist, and the engine must still run 1000 calibration matches inside it.

### T-06 — The realism test itself
**State:** live

C-06 records the planned test: goals 2.4 to 3.2, shots 8 to 16, possession 35 to 65, over 1000 matches. Batch 7 rejected A-01: three bands are not enough. C-37 adds shape measures the event stream already carries — pass completion, where shots come from, possession in each third, and goals by phase of play — so a match that passes the bands and still looks wrong now fails something. C-38 resolves X-06 by setting a band per condition band, dry and wet and worn, so a condition is expected to move the numbers rather than to fail the test. You did not take the watched check for a match, and A-16 records what that costs: whether the mood mechanism in C-16 reads as football is judged by aggregates alone. Batch 10 then extended the test to the season and took every measure offered. C-51 adds league-table shape — points spread, goal-difference distribution, home-win rate, and how often the strongest squad wins — plus market and wage shape, plus a watched season that a human reads and judges. A-24 is the assumption inside the watched season: a table, a scorer list, and a transfer list are enough to judge from, with no match watched. Batch 11 then rejected A-16 outright. C-55 puts a human in front of matches too, as a milestone review rather than a gate on every build, which closes X-11 and matters far beyond consistency — it is the first instrument on this board that can reach X-01 at all. A-26 is what it rests on: a review run by hand has to happen early enough to catch a failed mechanism before the rest of the model is built on it. Batch 12 finished the test. C-58 bands the spread as well as the average — how often a match finishes goalless, how often it finishes five-one, how far a team's worst performance sits from its best — which resolves X-03 by measuring what C-12 and C-16 actually added. A-28 is confirmed by session 3, which found and measured exactly that distribution. C-59 and C-60 give the outcome bands from 1826 matches: 2.709 goals a match with a standard deviation of 1.675, a home advantage of 0.351 goals, and a 45.3 / 24.5 / 30.2 split of home wins, draws and away wins. C-61 gives the rate bands, and they are far tighter than anyone guessed — pass accuracy varies by 1.1 percentage points across five leagues and shot volume by two shots. X-14 is what that costs the plan: C-06's bands are several times too wide to catch an engine that is wrong. C-59 also carries the surprise. Real football's goal distribution is Poisson, with a variance-to-mean ratio of 1.035, so the spread needs no extra mechanism at all, and X-13 puts that against C-12 and C-16. A-06 stays named and A-30 is the new question: whether the engine can move the scoring rate with mood and quality while the distribution stays Poisson.

### T-07 — The manager as a modelled entity
**State:** live

Two claims and the sharpest intent decision on the board. C-21 gives the manager attributes that move form and mood. C-30 extends it to both managers, so a manager weak at tactics has their instructions land weaker on the pitch and the player's own competence becomes part of the simulation. A-08 is confirmed. A-11 is the risk you accepted with it: players have to read the weakening as realism rather than as the game ignoring what they asked for. Nothing in the plans models a manager at all — the shape names an AI module with no identity and no attributes.

### T-08 — The season layer
**State:** live

Opened in batch 5 when you rejected A-07, and filled in batch 8. C-29 brought fixtures, results, and form into scope. C-40 adds a league table with standings, season-long squad availability with suspensions and injury persistence and carried fitness, and a full club calendar with cups, congestion, and rotation — you took every addition offered, so this is now the largest thread on the board. C-43 binds two of them together: congestion and rotation mean nothing unless fitness and suspensions carry between matches, so the calendar and availability ship as one piece. C-41 computes form twice, for the team and for each player, and the per-player form feeds C-14's mood. C-42 generates a backstory at save creation, which rejects A-09 and gives B-02 its partnerships; A-17 is the risk inside it, because a generated history has to agree with itself. A-18 is rejected by a read: C-44 records that the match event carries no player identifier, so per-player form has no source today. Batch 9 then decided what each of those costs. C-46 changes the event contract when B-07 starts, which resolves X-07 and rejects A-12, because the engine's contract now changes after the fact by decision; A-20 is the assumption that the later migration is the cheaper one, and every consumer of the stream being unbuilt today is the reason to doubt it. C-47 sets the backstory at one past season, with A-21 assuming that is enough history for a partnership to read as earned. Session 3 then measured what this thread had been guessing at. C-66 puts the ball above a metre for 12 to 19 percent of tracked frames and above two metres for 8 to 11 percent, and C-67 records that kloppy models ball height as standard and populates it from eight providers, so C-36 and C-32 are both supported by real data rather than by intuition. C-65 is the unplanned finding: the duel is the most common contested action in football at 456 a match, 87 of them aerial, and nothing on this board modelled one. C-62, C-63, C-64 and C-73 size the event contract — about 1683 events a match across roughly fifteen types, a completion rate that depends on the kind of pass rather than one number, and a minimum field set of type, time, coordinates, team, player and outcome. X-15 is the consequence for C-46. C-49 is the largest claim on the board: transfers and finances come in, so this is a football-management game and not a match engine with a season around it. X-09 and X-10 are what C-49 costs — no realism test reaches a market or a season, and a market needs clubs to disagree about a player that B-01 shows everyone in full.

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
- batch 13 · control · look it up → four datasets mined; 18 measured claims, 3 new contradictions
- batch 14 · T-06 · probe · what mood and quality actually move → the scoring rate, never the spread
- batch 14 · T-06 · probe · what the realism test bands → all four: rates, scorelines, curves, physical
- batch 14 · T-05 · probe · whether the duel becomes a primitive → yes, a first-class contested action
- batch 14 · T-04 · probe · how many phases the model runs → nine derived, two authored
- batch 15 · control · look it up → per-position and ball-relative movement mined; 8 claims, 1 contradiction
- batch 16 · control · look it up → Bundesliga ground truth mined; 9 claims, 1 assumption, 1 contradiction resolved
- batch 17 · control · look it up → DFL event schema read and synced to tracking; 8 claims, 1 assumption, 1 contradiction
- batch 18 · T-05 · correction · you rejected the duel as the single locus of quality → every action family is one; A-34 rejected, X-20 resolved
- batch 19 · control · look it up → every action enumerated and mined across all sources; 12 claims, 1 assumption
- batch 20 · control · look it up → the ten gaps in the match model mined; 18 claims, 2 assumptions, X-19 resolved, X-21 opened

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
