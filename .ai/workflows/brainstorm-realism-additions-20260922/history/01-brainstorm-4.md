---
schema: sdlc/v1
type: brainstorm
slug: brainstorm-realism-additions-20260922
topic: "what needs to be added to players, teams, tactics and match engine to improve realism; starting point is the in progress plans"
status: open
created-at: "2026-09-22T12:00:23Z"
updated-at: "2026-09-22T20:37:57Z"
sessions: 4
batches: 62
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
  - id: C-210
    thread: T-02
    text: "An attribute and an overlapping trait split by scope: the attribute acts on each action in a match (composure on a shot under pressure, aggression on a tackle), and the trait acts on occasions and choices (a final, a penalty shoot-out, whether to dive or to foul cynically)."
    evidence: unverified
  - id: C-211
    thread: T-02
    text: "Four more traits join C-208, for twelve in all: versatility (willingness to play out of position), controversy (trouble off the pitch), money motivation (how much wages decide moves), and bravery (readiness for a 50-50, a header in traffic, or a block)."
    evidence: unverified
  - id: C-212
    thread: T-02
    text: "The player record gains a fifth attribute group, personality, with its own slots outside the 50-attribute limit, so the twelve traits stop competing with new performance attributes for the 13 free slots; the record changes when the realism programme starts, as C-56 allows."
    evidence: unverified
  - id: C-213
    thread: T-01
    text: "Every realism band is an output that mechanisms must produce, never a value the engine is tuned to directly; the watched reviews of C-157 and C-158 catch what the bands miss, and a failed review reopens X-01."
    evidence: unverified
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
  - id: C-139
    thread: T-02
    text: "Attribute effects are amplified beyond C-128's measured sizes on purpose, because a management game must reward a better player; the per-player residual band that X-19 made possible is expected to fail by design."
    evidence: unverified
  - id: C-140
    thread: T-02
    text: "A manager must see that a better player is better over a run of about five matches, through form and match ratings, rather than inside one match or only over a season."
    evidence: unverified
  - id: C-141
    thread: T-05
    text: "The attribute gap lives in four places at once: in the choice a player makes (the pass picked, the moment to shoot), in the duel, in the body (pace, strength, stamina), and in the repeat, so a strong player also has fewer bad days."
    evidence: unverified
  - id: C-142
    thread: T-02
    text: "Consistency narrows the spread of every action, finishing and saving included, but by a different degree for each action."
    evidence: unverified
  - id: C-143
    thread: T-06
    text: "The best player's season totals — the top scorer's goals, the most assists, and similar — stay inside the range real leagues produce, so the season test in C-51 bands them."
    evidence: unverified
  - id: C-144
    thread: T-03
    text: "A club's style has three layered sources — a club identity that survives a change of manager, the squad's own abilities, and the manager's tactic — and each adds its own part to how the club plays."
    evidence: unverified
  - id: C-145
    thread: T-03
    text: "When two styles meet, the measured additive rule of C-130 is the default, and the manager's tactic for that match moves the result away from it, so a possession club may choose to give the ball up in one match; the realism test bands the default."
    evidence: "verified .scratch/out/gap_wy.txt:166 (the default rule only; the tactic effect is unverified)"
  - id: C-146
    thread: T-03
    text: "The additive rule of C-130 is an output, not an input: tactic, squad, and identity drive each match, one match may land far from the rule, and the season results must reproduce it; this amends C-145, whose default is now emergent rather than applied."
    evidence: unverified
  - id: C-147
    thread: T-06
    text: "The realism test reads a separate, test-only output in which the engine writes per-player data; the event contract the game reads stays as planned, and C-46 holds."
    evidence: unverified
  - id: C-148
    thread: T-05
    text: "When the realism programme changes the event contract, the contract copies a real provider schema in full rather than a core subset, so the engine's output can feed any tool that reads that schema."
    evidence: unverified
  - id: C-149
    thread: T-05
    text: "The event contract is a mix of two providers: it keeps every DFL event kind and attribute, and it adds what StatsBomb carries that DFL does not, such as the freeze frame of player positions at each shot; this amends C-148, which named one provider."
    evidence: unverified
  - id: C-150
    thread: T-05
    text: "The engine keeps one internal record of each match with one definition per event kind, and both the test-only output and the event contract are written from that record, so they can differ in what they show but never in a fact."
    evidence: unverified
  - id: C-151
    thread: T-05
    text: "Where DFL and StatsBomb define the same event kind differently, the DFL definition wins, and StatsBomb only adds what DFL lacks."
    evidence: unverified
  - id: C-152
    thread: T-06
    text: "The realism bands measured with StatsBomb definitions are converted to DFL definitions, one mapping per shared event kind, and the 7 public DFL matches check each conversion."
    evidence: unverified
  - id: C-153
    thread: T-07
    text: "An AI manager picks each match tactic from four inputs: its own philosophy, the opponent's strength and style, the match state by score and clock, and the season situation such as table position, a cup match, or a tired squad."
    evidence: unverified
  - id: C-154
    thread: T-07
    text: "No instruction lands weaker because of the manager; a manager weak at tactics makes poor tactical choices and understands the match poorly, so C-30's mechanism is replaced and the instruction a manager gives is always carried out as given."
    evidence: unverified
  - id: C-155
    thread: T-07
    text: "The human manager's tactical attribute acts on three things and never on an instruction: how accurate the manager's information is (opposition reports, match analysis, scouting noise as in C-52), how fast the squad learns a new tactic (C-23's familiarity), and how the squad's mood and trust respond to the manager's choices over time."
    evidence: unverified
  - id: C-156
    thread: T-07
    text: "A squad that distrusts a weak manager makes more errors after the same instruction, and that is mood rather than a weakened instruction: the instruction is carried out as given and the players carry it out worse, so C-154 stays true."
    evidence: unverified
  - id: C-157
    thread: T-06
    text: "The first watched-match review runs as soon as the first new mechanism runs, for example the choice model of C-121 and C-122, and a review follows each mechanism after it, so a failed mechanism is caught before other work builds on it."
    evidence: unverified
  - id: C-158
    thread: T-06
    text: "The watched-match reviewer uses all three comparisons: a simulated match side by side with a real match replayed in the same 2D view, a checklist of named qualities (shape, tempo, decisions, set pieces), and open judgement of what looks wrong."
    evidence: unverified
  - id: C-159
    thread: T-06
    text: "The stream crate already serves a recorded fixture over the same protocol the viewer reads, at 50 ticks a second, so a real match converted into a fixture can play in the viewer beside a simulated one."
    evidence: "verified crates/stream/src/replay.rs:1 and crates/stream/src/replay.rs:14"
  - id: C-160
    thread: T-04
    text: "Both managers decide live: the AI manager changes its tactic during the match, the human can pause and change as well, and the human's pre-set triggers of C-28 are an extra tool rather than the only one; the protocol already queues a tactics change or a substitution during a match."
    evidence: "verified crates/protocol/src/command.rs:1 and crates/protocol/src/command.rs:11 (the change queue; the AI side is unverified)"
  - id: C-161
    thread: T-04
    text: "A set-piece routine changes four things: who wins first contact, where the ball goes (near post, far post, short), where players stand for the second ball, and how many stay back against the counter."
    evidence: unverified
  - id: C-162
    thread: T-04
    text: "Every live change, tactical or a substitution, takes effect at the next stoppage, for both managers; the plan already applies each queued change at a qualifying stoppage through the match-rules slice, so pausing gives the human time to think but no earlier effect."
    evidence: "verified crates/protocol/src/command.rs:2"
  - id: C-163
    thread: T-04
    text: "The manager sets two team transition choices — counter-press or drop back on losing the ball, counter-attack or keep the ball on winning it — and each player carries an own transition instruction as a Tier 3 per-player instruction, which defaults sensibly from the team choices until the manager changes it."
    evidence: unverified
  - id: C-164
    thread: T-04
    text: "The manager sees the nine derived phases after the match only: the post-match report shows time spent in each phase and how the team did in each, and the match view does not show the current phase."
    evidence: unverified
  - id: C-165
    thread: T-08
    text: "A player's ability changes over seasons along age curves, one per attribute, and the curves differ from player to player with the variance real players show in how they develop; many other factors move the curves, and which factors they are is still open."
    evidence: unverified
  - id: C-166
    thread: T-08
    text: "The manager runs a training plan between matches, setting focus and intensity, and training moves fitness, injury risk, tactic familiarity (C-23), and development."
    evidence: unverified
  - id: C-167
    thread: T-08
    text: "Five factors move a player's age curves: playing time and the level played at, training and coaching, injuries, personality, and moments; the list stays open, so other factors may be added later."
    evidence: unverified
  - id: C-168
    thread: T-08
    text: "Moments are of two kinds and both move a player's curve: match moments such as a debut, a decisive goal, or a costly error in a final, and career turning points such as a transfer, a loan, a new manager, or a first-team breakthrough."
    evidence: unverified
  - id: C-169
    thread: T-08
    text: "A new save simulates several past seasons in full at save creation, so every player's current ability, partnerships, and reputation agree with a simulated past; this replaces C-47's one past season."
    evidence: unverified
  - id: C-170
    thread: T-08
    text: "Today's engine plays one match in 387 ms of wall time on one core of an AMD Ryzen 7 9800X3D, so one 20-club league season of 380 matches takes about 147 seconds on one core and about 20 seconds on eight, before any realism addition makes a match more expensive."
    evidence: "verified .ai/workflows/football-manager-match-engine/bench-baseline/viewer-pitch/bench-1.stdout.txt:6 (bench.match_wall_ms 387; the season figures are arithmetic)"
  - id: C-171
    thread: T-08
    text: "The past seasons simulated at save creation run a fast result-only model instead of the full match, so save creation stays short while the present runs the full engine."
    evidence: unverified
  - id: C-172
    thread: T-08
    text: "The largest public career dataset is Transfermarkt-derived: 50,000 players, 65 competitions, 1.89 million appearances with minutes, 650,000 market valuations, and 175,000 transfers under CC0; it measures playing time, level, moves, and market judgement, and it carries no ratings."
    evidence: "verified https://github.com/dcaribou/transfermarkt-datasets (research sub-agent, batch 43)"
  - id: C-173
    thread: T-08
    text: "Performance peaks at about 27.4 across 15,591 top-five-league player-seasons, at 26 to 27 for wingers and full-backs and 27.5 to 28 for centre-backs and strikers; wingers fall to half their peak by 31 and centre-backs by 35.2, and an 18-year-old in a top league plays at 50 to 85 percent of the eventual peak."
    evidence: "verified https://macro-football.com/other/aging/ (research sub-agent, batch 43)"
  - id: C-174
    thread: T-08
    text: "Physical output has its own curves: speed peaks at 25.7, endurance at 24.8, and explosiveness at 26.0 across 5,203 GPS match performances, high-intensity output falls after 32 while endurance holds, and creative passing holds into the early thirties while shooting and tackling fade."
    evidence: "verified https://pmc.ncbi.nlm.nih.gov/articles/PMC12551122/ (abstract) and https://www.espn.com/soccer/story/_/id/37467220/ (research sub-agent, batch 43)"
  - id: C-175
    thread: T-08
    text: "Of about 200 LaLiga academy players aged 13 to 18, 4 percent reached the top tier within ten years; across 378 Bundesliga loans, loanees gained no measurable minutes and no measurable market value against 6,162 players who were not loaned."
    evidence: "verified https://www.essex.ac.uk/news/2024/06/27/just-four-per-cent-of-teenage-academy-players-make-it-to-top-tier-football and https://ideas.repec.org/p/zbw/umiodp/295740.html (research sub-agent, batch 43)"
  - id: C-176
    thread: T-08
    text: "An ACL rupture returns 80 percent of elite players after a mean 216 days, leaves them below matched controls for two seasons with attackers declining further, ends the careers of 20 percent of professionals within three seasons, and lets only 47.5 percent keep their playing level."
    evidence: "verified https://doi.org/10.1177/23259671211008892 and https://pmc.ncbi.nlm.nih.gov/articles/PMC9859836/ (abstracts; research sub-agent, batch 43)"
  - id: C-177
    thread: T-08
    text: "No public data records true attribute histories, training or coaching effects at scale, personality across a career, or the causal effect of a moment; the only attribute-style history is the FIFA 15 to 23 rating set, which is human-rated rather than measured."
    evidence: "verified https://www.kaggle.com/datasets/stefanoleone992/fifa-23-complete-player-dataset (research sub-agent, batch 43)"
  - id: C-178
    thread: T-06
    text: "The realism test checks development two ways: it bands what C-172 to C-176 measure — peak age by position, decline speed, youth conversion, and injury after-effects — and a person reads a set of simulated careers for what no band covers."
    evidence: unverified
  - id: C-179
    thread: T-08
    text: "A loan's effect varies widely with the club, the coach, and the minutes, and it can help or hurt, while the average effect across loans stays near zero as C-175 measures."
    evidence: unverified
  - id: C-180
    thread: T-08
    text: "Each club has a board and an owner type — for example patient, ambitious, or frugal — that sets targets for league position, cups, finances, and style, sets how much patience the board has, and can sack the manager; sackings apply to AI managers as well."
    evidence: unverified
  - id: C-181
    thread: T-08
    text: "A transfer fee is a shared market valuation plus a premium from each club's own view, and the season test bands the resulting fees against real fees and valuations by age and position from C-172."
    evidence: unverified
  - id: C-182
    thread: T-08
    text: "The shared market valuation comes only from what every club can see — age, minutes, level, goals, recent fees, and reputation — as a Transfermarkt value does, and it never reads the true attributes."
    evidence: unverified
  - id: C-183
    thread: T-08
    text: "A sacked human manager keeps the save: the manager is unemployed, can apply for jobs, and receives offers based on reputation."
    evidence: unverified
  - id: C-184
    thread: T-08
    text: "The player chooses at save creation which countries and divisions are active, so more leagues give more jobs and more clubs to deal with, at the cost of a slower season."
    evidence: unverified
  - id: C-185
    thread: T-08
    text: "A club has four income sources: matchday income from stadium size, prices, and attendance; broadcast and prize money by league position and cup progress; commercial income from reputation and success; and owner money that the owner type of C-180 can put in or take out within limits."
    evidence: unverified
  - id: C-186
    thread: T-08
    text: "Every league the player did not choose runs the fast result-only model of C-171, so its players play, develop, and gain value, and the whole world moves while only the chosen leagues run the full engine."
    evidence: unverified
  - id: C-187
    thread: T-08
    text: "The revenue mix is measured: the top 20 clubs earn 43 percent commercial, 38 percent broadcast, and 19 percent matchday, the top ten lean on commercial at 48 percent while clubs ranked 11 to 20 lean on broadcast at 49 percent, and the Premier League as a whole takes about half its 6.8 billion pounds from broadcast."
    evidence: "verified https://www.deloitte.com/uk/en/services/consulting-financial/analysis/deloitte-football-money-league.html and the Deloitte Annual Review pages (research sub-agent, batch 49)"
  - id: C-188
    thread: T-08
    text: "In a selling league player sales are a main income source: across nine Eredivisie clubs in 2022/23 they were 29.5 percent of income on average and about 44 percent at Ajax and PSV."
    evidence: "verified https://offthepitch.com/a/player-sales-lifeline-dutch-football-financial-divide-widens (research sub-agent, batch 49)"
  - id: C-189
    thread: T-08
    text: "Revenue spread inside a league is set largely by broadcast rules: Premier League central payments run 1.6 to 1 from bottom to top with about 2.7 million pounds of merit pay per place, Bundesliga TV money runs 2.7 to 1, two clubs take 52 percent of LaLiga revenue, and Eredivisie revenue runs about 10 to 1."
    evidence: "verified https://www.football365.com/news/2025-26-premier-league-prize-money-table-tv and https://bulinews.com/bundesliga-money-distribution-for-2025/26-season-how-much-club-get (research sub-agent, batch 49)"
  - id: C-190
    thread: T-08
    text: "Wages run at 54 to 80 percent of revenue across the big-five leagues — Bundesliga 54, LaLiga 60, Premier League 65, Serie A 66, Ligue 1 80 — and UEFA's squad-cost rule caps wages, transfer costs, and agent fees at 70 percent of revenue from 2025/26."
    evidence: "verified the Deloitte Annual Review pages and https://www.uefa.com/news-media/news/0274-14da0ce4535d-fa5b130ae9b6-1000--explainer-uefa-s-new-financial-sustainability-regulations/ (research sub-agent, batch 49)"
  - id: C-191
    thread: T-08
    text: "Losses are normal and owners cover them: European top-division clubs lost 1.1 billion euros before tax in 2024, only 8 Premier League clubs made an operating profit, and Championship clubs lost 355 million pounds with one club in operating profit; owner-funding totals are a proxy from club examples."
    evidence: "verified https://www.uefa.com/news-media/news/02a2-200452a66064-0cfd3f86b94f-1000--new-report-highlights-record-revenues-and-increasing-inv/ and https://theesk.org/2025/07/19/15922/ (research sub-agent, batch 49)"
  - id: C-192
    thread: T-08
    text: "The richest league buys and the others sell: in summer 2025 Premier League clubs spent 3.0 billion pounds gross, about 44 percent of league revenue and 51 percent of all big-five gross spending, while the Bundesliga and Ligue 1 were net sellers by 180 and 305 million euros."
    evidence: "verified https://www.deloitte.com/uk/en/about/press-room/deloitte-premier-league-spending-record-eclipsed.html (research sub-agent, batch 49)"
  - id: C-193
    thread: T-08
    text: "Prize money is published in full: the Champions League pays 18.62 million euros for the league phase, 2.1 million per win, and rising round payments to 18.5 million for the final, from a 2.467 billion euro pot against 565 million for the Europa League."
    evidence: "verified https://www.beinsports.com/en-us/soccer/uefa-champions-league/articles/uefa-confirms-prize-distribution-for-2025-26-champions-league-2025-08-27 (research sub-agent, batch 49)"
  - id: C-194
    thread: T-06
    text: "The realism test checks finances two ways: it bands what C-187 to C-193 measure — the revenue mix, the broadcast spread, league wage ratios, losses, transfer flows, and prize money — and a person reads a set of simulated club accounts for what no band covers."
    evidence: unverified
  - id: C-195
    thread: T-08
    text: "Player-sale income enters the club's finances, and the owner type of C-180 decides how much of it reaches the manager's budgets and how much pays debts or wages."
    evidence: unverified
  - id: C-196
    thread: T-08
    text: "Leagues and competitions carry financial rules such as a squad-cost cap, and a breach brings a sanction — a fine, a transfer ban, or a points deduction."
    evidence: unverified
  - id: C-197
    thread: T-01
    text: "The engine-realism programme builds team shape and the block first — C-93's law that the team slides with the ball and changes width by phase — so the first watched review of C-157 judges the cheapest mechanism that changes how every match looks."
    evidence: unverified
  - id: C-198
    thread: T-01
    text: "B-07 no longer stays one card: the season layer splits into smaller cards that can each start on their own, and the split is written at the next distillation."
    evidence: unverified
  - id: C-199
    thread: T-08
    text: "B-07 splits into four cards: competitions (table, cups, calendar, congestion, suspensions), squad and careers (form, availability, training, development, the simulated past), the market (scouting, valuation, fees, transfers, loans), and club business (board and owner, income, wages, financial rules, sacking, the job market)."
    evidence: unverified
  - id: C-200
    thread: T-01
    text: "Engine realism runs first and the season layer follows, as C-31 and C-48 set it, so the four season cards build on a realistic match."
    evidence: unverified
  - id: C-201
    thread: T-01
    text: "B-08 becomes a recorded decision rather than a card, because C-56 already answered it, and B-05 is rewritten at the next distillation to the realism test the board now holds (C-78, C-147, C-150, C-157, C-178, C-194)."
    evidence: unverified
  - id: C-202
    thread: T-05
    text: "An in-match knock is handled by all three parties by severity: the medical staff take a severe injury off at once; for a minor knock the player's personality decides whether to play on, and the manager sees the knock and can overrule at the next stoppage; playing on raises the chance the knock becomes an injury and lowers the player's output."
    evidence: unverified
  - id: C-203
    thread: T-05
    text: "A severe injury is carried across seasons more gently than real football: a long absence, a shorter dip in form, and no career ended by injury, with personality and the medical staff moving a single player's return around that average."
    evidence: unverified
  - id: C-204
    thread: T-06
    text: "The career test's injury band is re-sized to a softer target set on purpose, and the game is banded against that target rather than against C-176's real after-effects."
    evidence: unverified
  - id: C-205
    thread: T-06
    text: "Every band that departs from real football on purpose carries two numbers: the real target and a design target; the test passes on the design target and reports the distance from real data beside it, which applies to C-139's per-player band, C-142's consistency, and C-204's injury band alike."
    evidence: unverified
  - id: C-206
    thread: T-02
    text: "Personality is hidden: the manager sees a description such as professional or volatile, whose accuracy is set by scouting quality and by time at the club, in the same way C-52 hides attributes."
    evidence: unverified
  - id: C-207
    thread: T-02
    text: "A player carries at least four personality traits — professionalism, temperament, ambition and loyalty, and leadership — and more traits are wanted beyond these four."
    evidence: unverified
  - id: C-208
    thread: T-02
    text: "A player carries at least eight personality traits — professionalism, temperament, ambition and loyalty, leadership, determination, adaptability, sportsmanship, and pressure handling — each a hidden 1-to-100 value, and still more traits are wanted."
    evidence: unverified
  - id: C-209
    thread: T-02
    text: "The attribute schema already carries composure, aggression, work rate, teamwork, and injury resistance in its mental and physical groups."
    evidence: "verified content/attributes.json:1"
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
    state: confirmed
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
    state: rejected
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
    state: confirmed
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
    state: rejected
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
    state: confirmed
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
    state: rejected
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
    state: rejected
  - id: A-42
    thread: T-06
    text: "The amplification in C-139 can be confined to player-level outcomes, so the league bands (C-59 to C-61), the Poisson scoreline (C-77), and the club-spread band (X-17) still pass while only the per-player residual band fails."
    state: named
  - id: A-43
    thread: T-02
    text: "Consistency can be an attribute that narrows a player's match-to-match spread without adding team-level variance, so a player repeats while the team's goal distribution stays Poisson."
    state: named
  - id: A-44
    thread: T-02
    text: "The degree by which consistency narrows each action follows the order in which real players repeat — duels and physical output most, passing next, finishing and saving least — so the departure from C-128 is largest where the data repeats most."
    state: named
  - id: A-45
    thread: T-06
    text: "One amplification size exists that is large enough for a manager to see a better player over about five matches (C-140) and small enough for the best player's season totals to stay inside the real range (C-143)."
    state: named
  - id: A-46
    thread: T-03
    text: "The engine can keep identity, squad, and tactic as separate inputs to style, so the realism test can check the club-spread band of X-17 and still tell which layer produced the spread."
    state: named
  - id: A-47
    thread: T-06
    text: "The realism test checks the style clash by fitting C-130's rule across every simulated match with AI managers on both sides, and the fitted multiplier lands near 0.949; you gave no preference on the test, so this follows from C-146 rather than from a choice."
    state: named
  - id: A-48
    thread: T-06
    text: "Each StatsBomb definition has a DFL equivalent close enough to convert a band, and 7 DFL matches are enough to check the conversion even where they are too few to measure the band."
    state: named
  - id: A-49
    thread: T-07
    text: "AI managers that read C-153's four inputs produce, over a season, tactic choices whose results reproduce C-130's 0.949 rule, so C-146 is met without tuning the rule directly."
    state: named
  - id: A-50
    thread: T-07
    text: "The three effects in C-155 apply to AI managers as well as to the human, so both sides of a match are modelled the same way and only the source of the choices differs."
    state: named
  - id: A-51
    thread: T-07
    text: "A player of the game can tell mood-driven errors from an ignored instruction, because the squad's mood and trust are visible to the manager before and during the match."
    state: named
  - id: A-52
    thread: T-06
    text: "DFL tracking at 25 frames a second converts into a fixture of 50-tick frames by interpolation, with player identities mapped to shirt numbers, and the viewer shows it without a change to the viewer."
    state: named
  - id: A-53
    thread: T-04
    text: "The AI manager's live decisions enter the match through the same change queue as the human's, so both sides change tactics the same way and no tactic logic runs inside the tick loop, which extends A-10 from triggers to every live change."
    state: named
  - id: A-54
    thread: T-04
    text: "A per-player transition default can be derived from the team's two transition choices and the player's role and attributes, so a manager who never opens the per-player screen still gets a coherent team transition."
    state: named
  - id: A-55
    thread: T-08
    text: "Public data on real careers is enough to size age curves per attribute and the player-to-player variance around them, even though real data measures performance rather than attributes."
    state: named
  - id: A-56
    thread: T-08
    text: "A real age curve measured on performance converts to an attribute curve through the same amplification C-139 applies, so a player's decline reads on the pitch at the size real careers show."
    state: named
  - id: A-57
    thread: T-08
    text: "Moments are one source of the player-to-player variance A-55 measures, not an extra spread on top of it, so the curves as a whole still match real careers."
    state: named
  - id: A-58
    thread: T-08
    text: "A result-only model can be calibrated to the full engine's results — scorelines, player totals, injuries, and development inputs such as minutes and moments — closely enough that a simulated past reads as the same football as the present."
    state: named
  - id: A-59
    thread: T-08
    text: "Training, coaching, and personality effects on development are set by design rather than by data, because C-177 finds no public data that sizes them, and the career test bands only the factors C-172 to C-176 measure."
    state: named
  - id: A-60
    thread: T-08
    text: "The spread of loan outcomes can be set by design, because C-175 measures only the average effect of a loan and not the spread around it."
    state: named
  - id: A-61
    thread: T-08
    text: "A valuation built from visible data only can be banded directly against C-172's 650,000 Transfermarkt valuations, because both read the same kind of visible inputs."
    state: named
  - id: A-62
    thread: T-08
    text: "Owner types and financial rules together reproduce the measured shape of losses in C-191 — most clubs losing money and owners covering it within the rules — without a band that forces the share of loss-making clubs."
    state: named
contradictions:
  - id: X-01
    threads: [T-01, T-02]
    text: "Reading 1 treats the planned model as the right shape needing more detail, and reading 3 says a fully detailed model can still pass the numbers without reading like football; more detail does not imply emergence. C-16 proposes the bridge and nothing has tested it. Batch 11 supplied the instrument rather than the answer: C-55 puts a human in front of a match, so the bridge is now checkable, and A-26 is the assumption that a milestone review catches the failure in time. Resolved in batch 61 by a principle rather than a proof: C-213 makes every band an output that mechanisms must produce, never a tuned value, and a failed watched review of C-157 reopens this contradiction; A-03 stays the bet."
    state: resolved
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
    text: "C-46 defers the event-contract change to the realism programme and prices it as adding a player identifier; C-62 and C-73 measure the real size of that change as four event types becoming roughly fifteen, plus a player identifier, coordinates, and an outcome on every event. The deferral was priced as one field and is a redesign of the contract. Resolved in batch 27: C-148 accepts the full redesign and copies a real provider schema, so A-37's core subset is rejected and the cost is priced at its true size."
    state: resolved
  - id: X-16
    threads: [T-05, T-06]
    text: "C-78 bands the physical output and C-79 makes the duel the place where player quality becomes an outcome, and both need the engine to report per-player data that C-44 records the event contract does not carry. X-15 already reprices that change and C-46 still defers it to the realism programme, so the test the board has just chosen cannot run against the engine the board has agreed to ship. Resolved in batch 27: C-147 gives the test a separate test-only output, so the test runs against the shipped engine and C-46 holds."
    state: resolved
  - id: X-17
    threads: [T-03, T-06]
    text: "C-61 bands league aggregates to about one percentage point and C-85 measures the club-to-club spread on the same statistic at 17 points; C-78 adopted the league bands alone, so the test as chosen passes an engine in which every club plays identically. The realism test needs a club-spread band as well as a league band, and nothing on the board yet produces club-to-club variation of that size. Batch 20 supplies the band: C-129 sizes team strength and C-130 splits club style into a strength axis and a pure-style layer, with the opponent setting possession as much as the club. Resolved in batch 23: C-144 produces the variation from three layered sources — club identity, squad, and tactic — and A-46 carries the cost of keeping the layers separate."
    state: resolved
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
    text: "Batch 2 set the strong-versus-weak gap as errors, ceiling and consistency together, and B-01 builds attributes a manager is meant to feel. C-128 measures that in real football the player moves pass completion by about 3.6 points, shot conversion by 2.5 and save rate by 2.4 once the situation is removed, and that finishing and saving barely repeat within a season. An engine faithful to C-128 may make attributes feel like they do not matter; an engine that makes them matter will fail C-128. Resolved in batch 21: C-139 amplifies attribute effects for the game and accepts that the per-player residual band fails by design; A-41 is rejected. A-42 carries the cost, because the amplification must stay out of the league, scoreline, and club bands."
    state: resolved
  - id: X-22
    threads: [T-02, T-06]
    text: "C-141 puts part of the gap in the repeat, so a strong player has fewer bad days, and C-128 measures that finishing and saving barely repeat within a season; C-58 also bands how far a team's worst match sits from its best. A player-level consistency attribute therefore argues with a measured fact and with a chosen band, and A-43 is the assumption that it can narrow a player's spread without moving the team's. Resolved in batch 22: C-142 keeps consistency on every action but by a different degree for each, which makes the departure from C-128 deliberate and graded; A-44 names the order the degrees follow."
    state: resolved
  - id: X-23
    threads: [T-03, T-06]
    text: "C-145 adds the manager's match tactic on top of C-130's additive rule and bands the rule as the default, but the 0.949 multiplier was measured on real clubs whose real managers already chose a tactic for every match; the measured rule therefore already contains tactical adaptation, and an engine that adds a tactic on top of it counts that adaptation twice. Resolved in batch 26: C-146 makes the rule an output of the match rather than an input to it, so no tactic is counted twice."
    state: resolved
  - id: X-24
    threads: [T-05, T-06]
    text: "C-147 has the test read a separate test-only output while the game reads the event contract, and C-148 later makes that contract a full provider schema; two outputs then report the same match, the realism test checks one and the player sees the other, so the test can pass a match whose reported events disagree with what it measured. Resolved in batch 29: C-150 writes both outputs from one internal record with one definition per event kind."
    state: resolved
  - id: X-25
    threads: [T-05, T-06]
    text: "C-149 merges DFL and StatsBomb into one contract, and C-110 measures that providers count the same action very differently; where both schemas define the same event kind, the merged contract must pick one definition, and C-148's benefit — output that any tool for that schema reads — holds for neither provider in full. Resolved in batch 29: C-151 lets DFL win every conflict, so each shared kind has one definition."
    state: resolved
  - id: X-26
    threads: [T-05, T-06]
    text: "C-151 makes DFL the definition of every shared event kind, and A-39 bands most on-ball and keeper families against StatsBomb over 200 matches; the public DFL set is 7 matches, too few to band most families on its own definitions, so either the StatsBomb-measured bands are converted to DFL definitions, or the bands are measured on 7 matches. Resolved in batch 30: C-152 converts the bands and uses the 7 matches to check the conversion; A-48 carries the risk."
    state: resolved
  - id: X-27
    threads: [T-07]
    text: "C-30 gives both managers attributes, and C-154 makes a weak tactician one who makes poor choices; the AI manager's choices come from its attributes, but the human player makes the human manager's choices, so the human manager's tactical attribute has nothing left to act on. Resolved in batch 32: C-155 gives the attribute three things to act on — information accuracy, learning speed, and squad reaction — and none of them is the instruction."
    state: resolved
  - id: X-28
    threads: [T-03, T-07]
    text: "C-154 removes every path by which a manager weakens an instruction, and C-155 lets the manager's attribute move squad mood and trust; C-22 says team mood gates the error rate, so a weak manager's team makes more errors after the same instruction, which is the weakening C-154 removed, arriving by a longer road. Resolved in batch 33: C-156 reads it as mood, not as a weakened instruction, so C-154 stays true; A-51 carries the risk that a player of the game can tell the two apart."
    state: resolved
  - id: X-29
    threads: [T-05, T-08]
    text: "C-169 simulates several past seasons at save creation, and C-170 prices one league season at about 20 seconds on eight cores with today's engine; five seasons across five leagues is about eight minutes before C-57's realism additions make each match more expensive, so either save creation becomes a long wait, or the past seasons run a cheaper match than the one the player sees. Resolved in batch 42: C-171 runs the past on a fast result-only model, and A-58 carries the calibration risk."
    state: resolved
  - id: X-30
    threads: [T-08]
    text: "C-168 makes a loan a career turning point that moves a player's curve, and C-175 measures 378 Bundesliga loans that gave loanees no measurable minutes and no measurable market value; a game in which a loan develops a player departs from the only measured evidence on loans. Resolved in batch 44: C-179 lets a loan help or hurt with an average near zero, which keeps C-175 true; A-60 sets the spread by design."
    state: resolved
  - id: X-31
    threads: [T-02, T-08]
    text: "C-52 hides a player's true attributes behind each club's scouting noise so that clubs disagree, and C-181 bases every fee on a shared market valuation; a shared valuation computed from the true attributes tells every club the truth C-52 hid, so either the valuation comes from something other than the true attributes, or the disagreement shrinks to the premium alone. Resolved in batch 46: C-182 builds the valuation from visible data only, so it never leaks the true attributes, and A-61 bands it against real valuations."
    state: resolved
  - id: X-32
    threads: [T-08]
    text: "C-185 gives a club four income sources, and C-188 and C-192 measure player sales as a main source for every league but the richest, at 29.5 percent of income in the Eredivisie; a selling club in the game has no way to earn what a real selling club earns unless transfer income is treated as a fifth source. Resolved in batch 50: C-195 brings sale income into the club's finances and lets the owner type decide where it goes."
    state: resolved
  - id: X-33
    threads: [T-05, T-06]
    text: "C-178 bands injury after-effects against C-176 — two seasons below par and 20 percent of professional careers ended within three seasons — and C-203 makes the dip shorter and ends no career; the injury band therefore fails by design, as C-139 made the per-player band fail, or it must be dropped or re-sized. Resolved in batch 55: C-204 re-sizes the band to a softer target set on purpose."
    state: resolved
  - id: X-34
    threads: [T-06]
    text: "The board now handles a deliberate departure from real football two ways: C-139 keeps the per-player band against real data and lets it fail by design, and C-204 re-sizes the injury band to a softer target so it passes; a reader of the test sees one departure as a red failure and the other as a green pass, with nothing to say that both are choices. Resolved in batch 56: C-205 gives every departed band a real target and a design target, passes on the design target, and reports the distance from real data."
    state: resolved
  - id: X-35
    threads: [T-02]
    text: "C-208 adds pressure handling and sportsmanship as personality traits, and C-209 records that the schema already carries composure and aggression; each pair describes much the same behaviour, so the engine either reads two values for one behaviour or must say what each value moves that the other does not. Resolved in batch 59: C-210 splits each pair by scope — the attribute acts on each action, the trait on occasions and choices."
    state: resolved
  - id: X-36
    threads: [T-02]
    text: "C-211 gives a player twelve hidden traits on the attribute scale, C-13 also wants new performance attributes, and C-17 records that the schema accepts 30 to 50 attributes in four fixed groups with 13 slots free; the traits alone take 12 of the 13, which leaves one slot for every new performance attribute. Resolved in batch 60: C-212 adds a fifth group, personality, with its own slots outside the limit."
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
  - rev: 3
    at: "2026-09-22T15:04:24Z"
    trigger: resume
    because: "session 3 opened to study real 2D tracking and event datasets against the board"
    changed: "sessions bumped to 3; the board kept as batch 12 left it pending the study"
  - rev: 4
    at: "2026-09-22T19:07:23Z"
    trigger: resume
    because: "session 4 opened on the board as batch 20 left it"
    changed: "sessions bumped to 4; the eight candidates and four open contradictions kept as written"
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

Four contradictions are open and they point the same way. X-15 reprices C-46: the deferred event-contract change is four event types becoming roughly fifteen plus three new fields on every event, not one field. X-16 is what batch 14 added to that bill — the physical bands and the duel both need per-player data the engine does not report, so the test this board just chose cannot run against the engine this board agreed to ship. X-01 is resolved in instrument but not in answer, and it is no longer counted among the open four because C-55 gave it a check. X-17 carried C-96's weight as well as C-85's, and batch 20 gave it a measured band; session 4 resolved it. X-19 is resolved. X-21 is new and sits beside X-17: the data says players differ less than a manager expects. Eight candidates stand as written from session 1, and every one of them now has real numbers to build against.

Session 4 opened on X-21 and you settled it against the data. C-139 amplifies attribute effects beyond the measured sizes, because a management game must reward a better player, so the per-player residual band fails by design and A-41 is rejected. C-140 sets the span: a manager sees a better player over a run of about five matches. C-141 puts the gap in four places at once — the choice, the duel, the body, and the repeat. A-42 is the new cost: the amplification must stay out of the league, scoreline, and club bands. The repeat is the one direction that argues with measured fact, so X-22 opened on it, and A-43 is the bet that consistency can narrow a player's spread without widening the team's.

Batch 22 closed X-22 and put a ceiling on C-139. C-142 keeps consistency on every action, finishing and saving included, but by a different degree for each, so the departure from C-128 is graded rather than flat; A-44 names the order. C-143 keeps the best player's season totals inside the range real leagues produce. That makes A-45 the crux of the whole amplification: one effect size must be large enough to show over five matches and small enough to leave season totals real.

Batches 23 and 24 closed X-17. C-144 gives a club's style three layered sources — a club identity that survives a new manager, the squad, and the tactic — and A-46 is the cost of keeping them separate. You asked what the data says before you judged a style clash, and C-130 already held the answer: the possession gap between two clubs carries almost fully into their match, and neither style overrides the other. C-145 takes that rule as the default and lets the match tactic move it, so a possession club may give the ball up on purpose. X-23 is the catch: the measured rule came from real managers who already chose a tactic for every match, so an engine that adds a tactic on top counts that choice twice.

Batches 25 and 26 closed X-23 with a worked example. C-146 reads the rule as an average: tactic, squad, and identity drive each match, and the season results must reproduce the rule rather than start from it. That amends C-145, and A-47 records the test that follows — fit the rule across every simulated match and land near 0.949.

Batch 27 closed both event-contract contradictions. C-147 gives the realism test a separate test-only output with per-player data, so the test runs against the shipped engine and C-46 holds, which resolves X-16. C-148 accepts the full size of the later contract change and copies a real provider schema, which resolves X-15 and rejects A-37's core subset. X-24 is what the pair costs: two outputs report the same match, the test checks one, and the player sees the other.

Batch 28 chose the schema. C-149 keeps every DFL event kind and attribute and adds what StatsBomb carries that DFL does not, such as the freeze frame at each shot. X-25 is the price of the mix: where both providers define the same kind, the contract must pick one definition, because C-110 shows they count differently, and no tool built for either provider reads the merged output in full. You asked for an example before judging X-24.

Batch 29 settled both. C-150 writes the test-only output and the event contract from one internal record with one definition per kind, so they can differ in what they show and never in a fact; X-24 closes. C-151 lets DFL win every definition conflict; X-25 closes. X-26 is what that costs the test: most bands in A-39 were measured against StatsBomb over 200 matches, and the public DFL set holds 7, so the bands must either be converted to DFL definitions or measured on too few matches.

Batch 30 closed X-26. C-152 converts each StatsBomb-measured band to its DFL definition and uses the 7 public DFL matches to check each conversion, so the bands keep their large sample. A-48 is the bet inside it: every StatsBomb definition has a DFL equivalent close enough to convert.

Batch 31 was a correction, and it removed a risk the board had carried since batch 5. C-154 says no instruction lands weaker because of the manager: a weak tactician makes poor choices and reads the match poorly, and what the manager asks for is always carried out. A-11 is rejected with the mechanism it guarded. C-153 gives the AI manager four inputs — its philosophy, the opponent, the match state, and the season situation — and A-49 bets that those choices reproduce the style-clash rule over a season. X-27 is what the correction leaves: the human player makes the human manager's choices, so the human manager's tactical attribute has nothing left to act on.

Batch 32 gave that attribute three things to act on and closed X-27. C-155 lets it set how accurate the manager's information is, how fast the squad learns a new tactic, and how the squad's mood and trust respond over time, and never the instruction itself. A-50 bets the same three effects apply to AI managers. X-28 is the catch in the third one: team mood gates the error rate (C-22), so a squad that distrusts its manager makes more errors after the same instruction, which is the weakening C-154 removed, arriving by a longer road.

Batch 33 accepted that road. C-156 reads the extra errors as mood: the instruction is carried out as given and a low-trust squad carries it out worse, so C-154 stays true and X-28 closes. A-51 is the narrower form of the risk A-11 once carried — a player of the game must be able to tell mood from an ignored instruction, which needs the squad's mood to be visible.

Batch 34 went back to X-01, the oldest question on the board, and built its instrument. C-157 runs the first watched-match review as soon as the first new mechanism runs, and one after each mechanism, which confirms A-26 by decision. C-158 gives the reviewer all three comparisons — a real match side by side in the same 2D view, a checklist of named qualities, and open judgement. A bounded read found the side-by-side half built already: C-159 records that the stream crate serves a recorded fixture over the viewer's own protocol, so A-52 is only the bet that real tracking converts into one. X-01 stays open, because only a run of the review can answer it.

Batch 35 went to tactics. C-160 lets both managers decide live — the AI changes its tactic during the match, and the human can pause and change too — so C-28's triggers become an extra tool rather than the only one. A bounded read found the human side built already: the protocol queues a tactics change or a substitution during a match. A-53 bets the AI's live changes use the same queue, so no tactic logic enters the tick loop. C-161 gives a set-piece routine four levers: who wins first contact, where the ball goes, the second ball, and the counter risk.

Batch 36 put the live change on the real touchline. C-162 makes every change, tactical or a substitution, wait for the next stoppage on both sides, and a read found the plan already does exactly that: the match-rules slice applies each queued change at a qualifying stoppage. Pausing gives the human time to think and no earlier effect.

Batch 37 filled in the transition. C-163 gives the manager two team choices — counter-press or drop back on losing the ball, counter-attack or keep it on winning it — and a per-player transition instruction beneath them that defaults from the team choices, with A-54 betting those defaults stay coherent. C-164 keeps the nine derived phases out of the match view and puts them in the post-match report.

Batch 38 went back to the season layer and found the gap it never had a claim for: how a player changes between seasons. C-165 gives each attribute its own age curve, with players spread around the curves the way real players develop, and leaves open which other factors move them. C-166 adds a training plan between matches that moves fitness, injury risk, familiarity, and development. A-55 and A-56 are the two bets underneath: that public career data can size the curves, and that a curve measured on performance converts to an amplified attribute curve.

Batch 39 named the factors. C-167 lets playing time and level, training and coaching, injuries, personality, and moments move the curves, and keeps the list open for more. "Moments" is your own addition, and batch 40 read it both ways: C-168 counts match moments, such as a debut or a decisive goal, and career turning points, such as a loan or a new manager. A-57 is the bet that keeps it honest — moments are one source of the real spread between careers, not a spread on top of it.

Batch 41 deepened the backstory. C-169 simulates several past seasons in full at save creation, replacing C-47's one season, so A-21 is rejected. A bounded read priced it: C-170 finds one match at 387 ms on one core today, so one league season is about 20 seconds on eight cores. X-29 is the result — five seasons across five leagues is about eight minutes before the realism additions make each match dearer, so either save creation waits, or the past runs a cheaper match than the present. You asked what career data exists before choosing the development test.

Batch 42 closed X-29: C-171 runs the simulated past on a fast result-only model, and A-58 is the bet that the model can be calibrated to the full engine. You then asked for the career data to be looked up before choosing the test.

Batch 43 looked it up. C-172 finds the career source: a Transfermarkt-derived set of 50,000 players with minutes, moves, and valuations, and no ratings. C-173 and C-174 give the curves — performance peaks at about 27.4, later for centre-backs than for wingers, and speed peaks at 25.7 while creative passing holds into the thirties. C-175 finds that 4 percent of one academy cohort reached the top tier, and that loans gave loanees nothing measurable. C-176 prices an ACL rupture at two seasons below par and a one-in-five chance of ending a professional career. C-177 names what is missing: attribute histories, training effects, personality, and the effect of a moment, so A-59 sets those by design. X-30 is the finding that argues: C-168 lets a loan move a curve, and the only measured evidence says a loan moves nothing.

Batch 44 used the data. C-178 bands what the lookup measured — peak age by position, decline speed, youth conversion, and injury after-effects — and adds watched careers for the rest, the same pattern as the season test. C-179 closes X-30 without leaving the data: a loan can help or hurt, and the average stays near zero, with A-60 setting the spread by design.

Batch 45 gave the manager someone to answer to. C-180 puts a board and an owner type behind every club, with targets, a measure of patience, and the power to sack, for AI managers as well as the human. C-181 builds a fee from a shared market valuation plus each club's premium and bands the fees against real ones. X-31 is what that costs C-52: a shared valuation computed from the true attributes tells every club what the scouting noise was hiding.

Batch 46 closed X-31 the way the real market works. C-182 builds the shared valuation from visible data only — age, minutes, level, goals, recent fees, and reputation — so it never reads the true attributes, and A-61 notes that such a valuation can be banded straight against real Transfermarkt values. C-183 keeps a sacked manager's save alive: the manager looks for a new job on the strength of reputation.

Batch 47 sized the world and filled the cash box. C-184 lets the player choose at save creation which countries and divisions are active. C-185 gives a club four income sources — matchday, broadcast and prizes, commercial, and owner money. Batch 48 kept the rest of the world moving: C-186 runs every league the player did not choose on the result-only model, so a player bought from outside has a real past. You asked for published club finances to be looked up before choosing the finance test.

Batch 49 looked them up, and most of what a finance band needs is published. C-187 gives the revenue mix — commercial for the biggest clubs, broadcast for the rest. C-189 finds that broadcast rules set the spread inside a league, from 1.6 to 1 in the Premier League to 10 to 1 in the Eredivisie. C-190 puts wages at 54 to 80 percent of revenue under a UEFA cap of 70, C-191 finds losses normal and owners covering them, and C-193 publishes the prize money. C-188 and C-192 are the ones that argue: outside the richest league, clubs live partly by selling players, which X-32 sets against C-185's four income sources.

Batch 50 used the data. C-194 bands what the lookup measured and adds watched club accounts, the same pattern as the career test. C-195 closes X-32: sale income enters the finances, and the owner type decides how much reaches the manager. C-196 brings financial rules in with real sanctions, and A-62 bets that owners and rules together reproduce the measured shape of losses without a band forcing it.

Batch 51 went back to T-01 and set an order. C-197 builds team shape and the block first, C-93's law that two providers on two continents agree on, so the first watched review judges the cheapest mechanism that changes how every match looks. C-198 splits B-07, because the season layer now holds development, a board and owner, a market, finances, and a world of chosen size. Batch 52 named the pieces: C-199 splits it into competitions, squad and careers, the market, and club business. Batch 53 kept the order C-48 set — C-200 runs engine realism first — and C-201 turns B-08 into a recorded decision and sends B-05 to be rewritten at the next distillation.

Batch 54 answered A-15, open since batch 6. C-202 gives an in-match knock to all three parties by severity: the medical staff take a severe injury off, the player's personality decides on a minor knock, and the manager can overrule at the next stoppage. C-203 then softens the long injury on purpose — a shorter dip and no career ended — with personality and the medical staff moving each return. X-33 is the cost: the career test bands injury after-effects against real data, and the game has just chosen to miss that band.

Batch 55 confirmed C-202's severity split and closed X-33: C-204 re-sizes the injury band to a softer target. X-34 is what that leaves in the test as a whole. The per-player band of C-139 keeps real data and fails by design, the injury band moves its target and passes, and a reader of the test sees one choice as a red failure and the other as a green pass.

Batch 56 gave the test one rule for every choice. C-205 puts two numbers on each band that departs from real football on purpose — the real target and a design target — so the test passes on the design and still reports how far the game sits from real data. That closes X-34 and covers C-139, C-142, and C-204 alike.

Batch 57 went back to the player. C-206 hides personality and lets the manager read a description whose accuracy grows with scouting and with time at the club, the same way C-52 hides attributes. C-207 takes all four traits offered — professionalism, temperament, ambition and loyalty, and leadership — and asks for more.

Batch 58 took four more — determination, adaptability, sportsmanship, and pressure handling — and asked for more again, so C-208 holds eight hidden traits on the 1-to-100 scale and A-05 is confirmed. A read of the schema then found the overlap: composure and aggression already exist as attributes, and X-35 asks what pressure handling and sportsmanship move that those two do not.

Batch 59 split each pair by scope and closed X-35: C-210 lets the attribute act on each action and the trait on occasions and choices. C-211 adds versatility, controversy, money motivation, and bravery, so a player now carries twelve hidden traits. A read of C-17 then found that twelve traits would take 12 of the 13 free attribute slots, and batch 60 closed that X-36 at once: C-212 gives personality a fifth group of its own, outside the limit. You then asked to sort out the open contradictions, and X-01 is the one left.

Batch 61 closed X-01, open since the first batch, with a principle rather than a proof. C-213 makes every realism band an output that mechanisms must produce and never a value the engine is tuned to, which names what session 4 kept choosing — the clash rule as an output, managers and owners that produce measured shapes unforced. The watched reviews catch what the bands miss, and a failed review reopens X-01. A-03 stays the bet. No contradiction on the board is open now.

*Sessions: 4 | Batches: 62 | Threads: 8 live · 0 parked · 0 routed · 0 dropped | Candidates: 8*

## Threads

### T-01 — Realism gaps left by the in-progress plans
**State:** live

The framing thread, and batch 5 turned it into a scoping thread. C-07 records your reading: three at once. C-08 and C-25 are the verified constraints — the workflow sits at `stream-protocol`, and the shape excludes leagues, seasons, competitions, transfers, and finances. C-31 is the decision: every idea here belongs to a second workflow, a realism programme separate from the engine build. A-02 is confirmed, because nothing on this board removes planned scope. A-03 stays named and X-01 is A-03 failing. Session 2 added C-45: the stream-protocol slice is complete and four slices depend on it, so this thread carried two after-the-fact contract changes rather than one. Batch 9 settled one of them. C-46 changes the event contract when B-07 starts and not before, which resolved X-07 and left X-05 and the player record still open, so B-08 now gates the player record alone. C-48 is the other decision: the work splits into two programmes, engine realism for B-01 to B-06 and the season layer for B-07, which resolved X-08 and amends C-31 rather than replacing it. Batch 10 then settled what all of it is for. C-50 reads the whole board as one product, a football-management game with the match engine as its first component. Batch 11 decided what that costs the work in flight, and the answer is nothing: C-53 keeps the charter and its exclusions as they stand, C-54 keeps the engine done at a playable match on the sixteen-slice roster, and A-23 is confirmed because C-48's split makes both descriptions true at once. X-12 closes on that. The residual cost is recorded rather than removed — a person who reads the engine workflow alone will take the engine for the whole product. Session 4 turned this thread into the order of work. C-197 builds team shape and the block first, so the first watched review judges the cheapest mechanism that changes how every match looks. C-198 and C-199 split B-07 into four season cards, C-200 keeps engine realism ahead of all four, and C-201 records B-08 as a decision and sends B-05 to be rewritten.

### T-02 — Players — the missing person
**State:** live

Ten claims, four verified against code. The model today is a number vector with a label: C-02, C-03, and C-09 say a player is 37 values, one position code, and nothing about the person. C-10 keeps all four absences in scope. C-11 replaces the position code with a competence rating per position, and it is the one change on this board that alters the shape of the player record rather than extending it, which is why X-05 exists. C-12 makes the strong-versus-weak gap three-dimensional and warns that any one dimension alone reads as a flat multiplier. C-13 adds performance and personality attributes, bounded by C-17 at 13 free slots inside four fixed groups. C-14 and C-16 carry the mechanism. A-04 asks whether a generator can invent believable multi-position ratings; A-05 assumes personality fits the 1-to-100 scale. Batch 12 settled the timing: C-56 changes the record when the realism programme starts, following C-46's precedent, and A-29 is the assumption that migrating match-rules and tactics-and-ai later beats delaying them now. Session 3 measured the physical player this thread describes. C-70 gives the shape of a match: 9.6 to 10.3 kilometres covered, 70 percent of the time below 2 metres per second, and sprinting for a quarter of one percent of it. C-71 shows how narrow real ability is — peak sprint velocity runs from 26.6 to 29.9 kilometres per hour between the tenth and ninetieth percentile of a whole league — which bounds how far apart C-12 may place a strong player and a weak one. C-72 is the counter-intuitive one: high-speed running is higher out of possession than in it, so defending is the expensive phase. C-52 added the other half of what a player is — a club sees a noisy estimate of the player's attributes rather than the exact numbers, weighed by its own identity, which is where B-07's market finds its disagreement. X-02, X-03, and X-05 are all resolved now. Batch 21 resolved X-21 as well: C-139 amplifies attribute effects beyond C-128 on purpose, C-140 asks that quality shows over about five matches, and A-41 is rejected. Batch 22 resolved X-22: C-142 keeps consistency on every action by a graded degree, A-43 bets that it narrows a player's spread without widening the team's, and A-44 names the order of the degrees. The thread is party only to X-01.

### T-03 — Teams — the missing club
**State:** live

Seven claims. C-04 is the verified starting point: two clubs differ only in their players and every fixture is played on neutral ground. C-23 answers with all four absences — home ground and home advantage, reputation and stature, squad familiarity per tactic, and a club playing identity. C-19 and C-20 say what moves team mood, from match events through to a club's reputation for late goals. C-22 bounds the effect to behaviour inside the tactic and to the error rate. C-24 gives the squad leadership, per-pair partnerships, and cohesion at two scopes. A-07 is rejected, so C-20's inputs now come from T-08 rather than from asserted fields. A-09 is rejected as well: session 2 answered it, and C-42 says the season layer generates the shared history at save creation rather than inventing partnerships without one. The consequence is a dependency this thread did not carry before — B-02's partnerships read as believable on day one only once B-07 supplies the backstory. Batches 20 to 24 gave the club its style. C-129 and C-130 size team strength and split club style into a strength axis and a pure-style layer. C-144 builds that style from three layers — identity, squad, and tactic — which resolves X-17, with A-46 as the cost. C-145 makes a style clash additive by default, as C-130 measures, with the match tactic free to move it. The thread is party to X-23: the measured rule already contains the tactics real managers chose, so adding a tactic on top risked counting it twice. C-146 resolved that: the rule is an output the season must reproduce, not an input each match starts from.

### T-04 — Tactics — beyond Tier 2
**State:** live

Four claims, and you took the expensive option on every one. C-05 is the verified starting point. C-26 splits the tactic into two phases, so the formation becomes two formations and the transition between them needs timing. C-27 makes a set piece a duel: designed routines with assigned takers against a defensive routine. C-28 defines Tier 3 as per-player and opposition instructions, and you added manager-set triggers, which the batch did not offer. A-10 is the assumption under the trigger: it lives in the rule pack and the change queue, so no conditional tactic logic enters the tick loop. C-23's squad familiarity attaches here, and there are now two phases to be familiar with. Session 3 both confirmed and enlarged C-26. C-69 measures the difference the two phases make — 46 to 49 metres wide in possession against 36 to 38 out of it, with the defensive line 5 metres higher and the depth barely moving — so the phase split is real and quantified. C-68 says it is also too small: real phase-of-play data uses nine paired phases, from build-up against a high block through to transition and chaos, each locked one-to-one to the opponent's. A-32 is the way out that keeps the tactic interface simple — derive the nine phases from the engine's own state and let the manager still set two. Batch 35 made both managers decide live: C-160 lets the AI change its tactic during the match and the human pause and change, with triggers as an extra tool, and a read confirms the protocol's change queue already takes a tactics change mid-match. A-53 extends A-10 to every live change. C-161 gives a set-piece routine four levers, sized by C-134's first-contact data. C-162 makes every live change wait for the next stoppage, which the planned change queue already does. C-163 adds team and per-player transition choices with defaults, and C-164 shows the nine derived phases in the post-match report only.

### T-05 — Match engine — beyond the laws
**State:** live

Six claims after batch 6, and again you took the fullest option each time. C-18 makes this thread the cost ledger: the tactics-and-ai slice fails at 10 percent CPU or 25 percent memory over baseline, and every addition on every other thread lands against it. C-32 is the useful surprise — the ball already carries a three-dimensional position and velocity, so height exists today and nothing uses it. C-33 makes the referee a full official, down to a home leaning the crowd can move, and A-13 is the risk inside that. C-34 makes pitch wear and weather interact. C-35 gives an injury a severity and drives its risk from hard tackles, fatigue, and collisions, which finally gives the aggression attribute teeth; A-15 records what C-35 did not settle, which is whether a manager may keep a hurt player on; batch 54 settled it with C-202, and C-203 softened the long injury below real data, which opens X-33. C-36 uses the height that C-32 found: arcs, heading duels, keeper claims, and spin. A-14 assumed all of that fits the tripwire, and X-02 and X-06 both said it may not. Batch 12 resolved the question rather than the cost: C-57 scopes C-18's tripwire to the engine build, and the realism programme measures against the shipped engine with a budget of its own. A-27 is what that rests on — the budget is chosen before the additions exist, and the engine must still run 1000 calibration matches inside it.

### T-06 — The realism test itself
**State:** live

C-06 records the planned test: goals 2.4 to 3.2, shots 8 to 16, possession 35 to 65, over 1000 matches. Batch 7 rejected A-01: three bands are not enough. C-37 adds shape measures the event stream already carries — pass completion, where shots come from, possession in each third, and goals by phase of play — so a match that passes the bands and still looks wrong now fails something. C-38 resolves X-06 by setting a band per condition band, dry and wet and worn, so a condition is expected to move the numbers rather than to fail the test. You did not take the watched check for a match, and A-16 records what that costs: whether the mood mechanism in C-16 reads as football is judged by aggregates alone. Batch 10 then extended the test to the season and took every measure offered. C-51 adds league-table shape — points spread, goal-difference distribution, home-win rate, and how often the strongest squad wins — plus market and wage shape, plus a watched season that a human reads and judges. A-24 is the assumption inside the watched season: a table, a scorer list, and a transfer list are enough to judge from, with no match watched. Batch 11 then rejected A-16 outright. C-55 puts a human in front of matches too, as a milestone review rather than a gate on every build, which closes X-11 and matters far beyond consistency — it is the first instrument on this board that can reach X-01 at all. A-26 is what it rests on: a review run by hand has to happen early enough to catch a failed mechanism before the rest of the model is built on it. Batch 34 confirmed A-26 by decision: C-157 runs the first review after the first mechanism, C-158 gives the reviewer a side-by-side replay, a checklist, and open judgement, and C-159 finds the replay path built already in the stream crate. Batch 12 finished the test. C-58 bands the spread as well as the average — how often a match finishes goalless, how often it finishes five-one, how far a team's worst performance sits from its best — which resolves X-03 by measuring what C-12 and C-16 actually added. A-28 is confirmed by session 3, which found and measured exactly that distribution. C-59 and C-60 give the outcome bands from 1826 matches: 2.709 goals a match with a standard deviation of 1.675, a home advantage of 0.351 goals, and a 45.3 / 24.5 / 30.2 split of home wins, draws and away wins. C-61 gives the rate bands, and they are far tighter than anyone guessed — pass accuracy varies by 1.1 percentage points across five leagues and shot volume by two shots. X-14 is what that costs the plan: C-06's bands are several times too wide to catch an engine that is wrong. C-59 also carries the surprise. Real football's goal distribution is Poisson, with a variance-to-mean ratio of 1.035, so the spread needs no extra mechanism at all, and X-13 puts that against C-12 and C-16. A-06 stays named and A-30 is the new question: whether the engine can move the scoring rate with mood and quality while the distribution stays Poisson. Batch 21 gave the test its first deliberate failure: C-139 amplifies player effects, so the per-player residual band fails by design, and A-42 is the assumption that every other band still passes. Session 4 then settled how the test treats the game's own choices: C-205 gives every band that departs from real data a real target and a design target, and reports the distance. Batch 22 capped the amplification from the season side: C-143 keeps the best player's season totals inside the real range, and A-45 is the bet that one effect size satisfies both C-140 and C-143.

### T-07 — The manager as a modelled entity
**State:** live

Two claims and the sharpest intent decision on the board. C-21 gives the manager attributes that move form and mood. C-30 extends it to both managers, so a manager weak at tactics has their instructions land weaker on the pitch and the player's own competence becomes part of the simulation. A-08 is confirmed. Batch 31 replaced C-30's mechanism: C-154 says an instruction is always carried out as given, and a manager weak at tactics makes poor choices instead, so A-11 is rejected. C-153 gives the AI manager four inputs to a match tactic, and A-49 bets they reproduce C-130's rule over a season. Batch 32 resolved X-27: C-155 gives the human manager's attribute information accuracy, learning speed, and squad reaction to act on, and A-50 applies the same three to AI managers. Batch 33 resolved X-28: C-156 reads a low-trust squad's extra errors as mood, not a weakened instruction, and A-51 asks that the mood be visible so the difference reads. Nothing in the plans models a manager at all — the shape names an AI module with no identity and no attributes.

### T-08 — The season layer
**State:** live

Opened in batch 5 when you rejected A-07, and filled in batch 8. C-29 brought fixtures, results, and form into scope. C-40 adds a league table with standings, season-long squad availability with suspensions and injury persistence and carried fitness, and a full club calendar with cups, congestion, and rotation — you took every addition offered, so this is now the largest thread on the board. C-43 binds two of them together: congestion and rotation mean nothing unless fitness and suspensions carry between matches, so the calendar and availability ship as one piece. C-41 computes form twice, for the team and for each player, and the per-player form feeds C-14's mood. C-42 generates a backstory at save creation, which rejects A-09 and gives B-02 its partnerships; A-17 is the risk inside it, because a generated history has to agree with itself. A-18 is rejected by a read: C-44 records that the match event carries no player identifier, so per-player form has no source today. Batch 9 then decided what each of those costs. C-46 changes the event contract when B-07 starts, which resolves X-07 and rejects A-12, because the engine's contract now changes after the fact by decision; A-20 is the assumption that the later migration is the cheaper one, and every consumer of the stream being unbuilt today is the reason to doubt it. C-47 sets the backstory at one past season, with A-21 assuming that is enough history for a partnership to read as earned. Session 3 then measured what this thread had been guessing at. C-66 puts the ball above a metre for 12 to 19 percent of tracked frames and above two metres for 8 to 11 percent, and C-67 records that kloppy models ball height as standard and populates it from eight providers, so C-36 and C-32 are both supported by real data rather than by intuition. C-65 is the unplanned finding: the duel is the most common contested action in football at 456 a match, 87 of them aerial, and nothing on this board modelled one. C-62, C-63, C-64 and C-73 size the event contract — about 1683 events a match across roughly fifteen types, a completion rate that depends on the kind of pass rather than one number, and a minimum field set of type, time, coordinates, team, player and outcome. X-15 is the consequence for C-46. C-49 is the largest claim on the board: transfers and finances come in, so this is a football-management game and not a match engine with a season around it. X-09 and X-10 are what C-49 costs — no realism test reaches a market or a season, and a market needs clubs to disagree about a player that B-01 shows everyone in full. Session 4 gave the season layer its development model. C-165 gives each attribute its own age curve with real player-to-player variance, C-166 adds a training plan between matches, and A-55 and A-56 are the bets on the data and on the conversion to amplified attributes. C-167 names five factors that move the curves — playing time, training, injuries, personality, and moments — and keeps the list open. C-169 simulates several past seasons at save creation on the result-only model of C-171, and C-172 to C-177 give the real career data: peaks near 27, a 4 percent academy conversion, loans that average to nothing (C-179), and no public attribute history. Batches 45 to 50 gave the club its business. C-180 adds a board and an owner type that can sack the manager, and C-183 keeps a sacked manager's save alive. C-182 values a player from visible data only, which resolves X-31. C-184 lets the player choose the active leagues, and C-186 runs the rest on the result-only model. C-185 and C-195 give a club five ways to earn, C-196 adds financial rules with sanctions, and C-187 to C-193 measure the real shape of all of it. C-198 and C-199 then split this thread's card in four: competitions, squad and careers, the market, and club business.

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
- batch 21 · T-02 · reflection · which reading of X-21 is yours → amplify effects for the game
- batch 21 · T-02 · probe · over what span must quality show → over a run of about five matches
- batch 21 · T-05 · fork · where does the attribute gap live → choice, duel, body, and repeat
- batch 22 · T-02 · probe · how far a consistency effect reaches → every action, by a varying degree
- batch 22 · T-06 · probe · how the best player's season totals compare → inside the real range
- batch 23 · T-03 · probe · where a club's style comes from → identity, squad, and tactic, layered
- batch 23 · T-03 · probe · what a manager sees when styles clash → asked for the data first
- batch 24 · T-03 · probe · what the engine reproduces when styles meet → additive default, and the tactic moves it
- batch 24 · T-03 · steer · how do you want to continue → continue
- batch 25 · T-03 · reflection · which reading of the measured rule → asked for a worked example
- batch 25 · T-06 · probe · what the test bands for a clash → no preference
- batch 26 · T-03 · reflection · which reading, after the worked example → the rule is the average
- batch 27 · T-06 · probe · how the test gets per-player data → a separate test-only output
- batch 27 · T-05 · probe · how large the event contract becomes → a full provider schema
- batch 28 · T-06 · probe · how the two outputs stay in agreement → asked for an example
- batch 28 · T-05 · probe · which provider schema the contract copies → DFL in full, plus StatsBomb
- batch 28 · T-05 · steer · how do you want to continue → continue
- batch 29 · T-06 · probe · how the two outputs agree, after the example → one record, two views
- batch 29 · T-05 · probe · which provider wins a definition conflict → DFL wins
- batch 30 · T-06 · probe · how bands match DFL definitions → convert the StatsBomb bands
- batch 31 · T-07 · probe · what an AI manager reads to pick a tactic → philosophy, opponent, match state, situation
- batch 31 · T-07 · correction · how a weak instruction is signalled → no weak instructions; poor choices instead
- batch 32 · T-07 · fork · what the human's tactical attribute acts on → information, learning speed, squad reaction
- batch 32 · T-07 · steer · how do you want to continue → continue
- batch 33 · T-07 · reflection · which reading of X-28 is yours → acceptable, it is mood
- batch 34 · T-06 · probe · when the first watched review happens → after the first mechanism runs
- batch 34 · T-06 · probe · what the reviewer compares against → side by side, checklist, and open judgement
- batch 35 · T-04 · probe · whether triggers and AI changes share a mechanism → both decide live; triggers are extra
- batch 35 · T-04 · fork · what a set-piece routine changes → first contact, target, second ball, counter risk
- batch 36 · T-04 · probe · when a live change takes effect → at the next stoppage
- batch 36 · T-04 · steer · how do you want to continue → continue
- batch 37 · T-04 · probe · what the manager sets for transitions → team choices plus per-player, with defaults
- batch 37 · T-04 · probe · whether the manager sees nine phases → shown after the match
- batch 38 · T-08 · probe · how ability changes over seasons → per-attribute age curves with real variance
- batch 38 · T-08 · probe · whether the manager runs training → yes, a training plan
- batch 39 · T-08 · fork · which factors move the age curves → playing time, training, injuries, personality, moments
- batch 40 · T-08 · reflection · what "moments" means → match moments and career turning points
- batch 40 · T-08 · steer · how do you want to continue → continue
- batch 41 · T-06 · probe · how the test checks development → asked for the real data first
- batch 41 · T-08 · probe · how far back generated history reaches → several simulated seasons
- batch 42 · T-06 · probe · how the test checks development, after the summary → look it up first
- batch 42 · T-08 · probe · how save creation stays acceptable → cheaper result-only past matches
- batch 43 · control · look it up → real career data mined; 6 claims, 1 assumption, 1 contradiction
- batch 44 · T-06 · probe · how the test checks development → band what is measured, plus watched careers
- batch 44 · T-08 · reflection · which reading of X-30 is yours → loans vary widely, average near zero
- batch 44 · T-08 · steer · how do you want to continue → continue
- batch 45 · T-08 · probe · who the manager answers to → a board plus an owner type
- batch 45 · T-08 · probe · what sets a transfer fee → valuation plus premium, banded against real fees
- batch 46 · T-08 · probe · where the shared valuation comes from → from what every club can see
- batch 46 · T-08 · probe · what happens when the human is sacked → find a new job
- batch 47 · T-08 · probe · how many leagues a save simulates → chosen at save creation
- batch 47 · T-08 · fork · which income sources a club has → matchday, broadcast and prizes, commercial, owner
- batch 48 · T-08 · probe · what exists outside the chosen leagues → result-only leagues
- batch 48 · T-06 · probe · how the test checks finances → look it up first
- batch 48 · T-08 · steer · how do you want to continue → continue
- batch 49 · control · look it up → published club finances mined; 7 claims, 1 contradiction
- batch 50 · T-06 · probe · how the test checks finances → band what is measured, plus watched accounts
- batch 50 · T-08 · probe · how player-sale income enters finances → the owner type decides
- batch 50 · T-08 · probe · whether the game models financial rules → yes, with sanctions
- batch 51 · T-01 · probe · which mechanism the programme builds first → team shape and the block
- batch 51 · T-01 · probe · whether B-07 stays one card → split B-07
- batch 52 · T-08 · fork · which cards B-07 splits into → competitions, squad and careers, market, club business
- batch 52 · T-01 · steer · how do you want to continue → continue
- batch 53 · T-01 · probe · in which order the two programmes run → engine first
- batch 53 · T-01 · probe · what happens to B-08 and B-05 → record B-08, rewrite B-05
- batch 54 · T-05 · probe · what happens when a player takes a knock → staff, player, and manager, by severity
- batch 54 · T-05 · probe · how a severe injury carries across seasons → softer than real, moved by personality
- batch 55 · T-05 · reflection · whether C-202's severity split is the reading → yes, that reading
- batch 55 · T-06 · probe · what the injury band does → re-size to a softer target
- batch 56 · T-06 · probe · how the test treats deliberate departures → two numbers per band
- batch 56 · T-06 · steer · how do you want to continue → continue
- batch 57 · T-02 · probe · whether the manager sees personality → hidden, seen through scouting
- batch 57 · T-02 · fork · which personality traits a player carries → the four offered, and more
- batch 58 · T-02 · fork · which further traits a player carries → all four offered, and more again
- batch 58 · T-02 · probe · whether traits use the 1-to-100 scale → yes, hidden
- batch 59 · T-02 · probe · how overlapping attributes and traits relate → split by scope
- batch 59 · T-02 · fork · which further traits a player carries → versatility, controversy, money motivation, bravery
- batch 60 · T-02 · probe · how the player record makes room for traits → a fifth group, personality
- batch 60 · T-02 · steer · how do you want to continue → sort out the open contradictions
- batch 61 · T-01 · reflection · which reading of X-01 is yours → a principle plus the watched review
- batch 62 · T-01 · steer · how do you want to continue → show the board
- batch 62 · control · board → printed; the loop holds

## Candidates

Eight candidates, one per live thread, split by dependency per C-39. Every one inherits the whole board; the ids named on each card are what that card turns on.

### B-01 — Deepen the player model into a person [T-02]
**Inherits:** C-02, C-03, C-09, C-10, C-11, C-12, C-13, C-14, C-16, C-17, C-52, C-139, C-140, C-141, C-142, C-143, C-206, C-207, C-208, C-209, C-210, C-211, C-212 · A-04, A-05 (confirmed), A-25, A-41 (rejected), A-43, A-44, A-45 · X-01, X-02, X-05, X-21, X-22, X-35, X-36

Competence per position instead of one code, new performance and personality attributes inside the 13 free slots the schema leaves, age and height and preferred foot and condition, and a strong-versus-weak gap made of errors, ceiling, and consistency. It carries C-16, the mechanism that answers reading 3: a mood the match moves, read through a personality that decides how hard it lands. This is the largest and most load-bearing card, and B-08 gates its most expensive part. Batch 10 added C-52: a club sees a noisy estimate of a player's attributes rather than the exact numbers, with the noise set by its scouting quality, and it weighs what it sees by its own identity. That makes this card the source of the disagreement B-07's market trades on, and A-25 is the assumption that two sources of error still band cleanly.

**Entry:** `/wf intake player-model-realism from brainstorm-realism-additions-20260922`

### B-02 — Model the club, the venue, and the squad [T-03]
**Inherits:** C-04, C-15, C-19, C-22, C-23, C-24, C-42, C-129, C-130, C-144, C-145, C-146 · A-09 (rejected), A-17, A-46, A-47 · X-04, X-17, X-23

Home ground and home advantage, reputation and stature, squad familiarity per tactic, a club playing identity, and a team mood moved by events, by score and clock, and by crowd and venue. The squad gains leadership, per-pair partnerships, and cohesion at two scopes — the squad, and the eleven on the pitch. Session 2 changed this card's dependency. A-09 is rejected and C-42 supplies the shared history, so the partnerships read as believable on day one only once B-07 generates the backstory. Start this card without B-07 and accept flat partnerships in the first season, or sequence B-07 first.

**Entry:** `/wf intake club-and-squad-model from brainstorm-realism-additions-20260922`

### B-03 — Raise tactics to Tier 3 with phases, set pieces, and triggers [T-04]
**Inherits:** C-05, C-26, C-27, C-28, C-68, C-69, C-80, C-134, C-160, C-161, C-162, C-163, C-164 · A-10, A-32, A-53, A-54 · X-02

Two phases with their own shapes, set-piece routines that the opponent defends against, per-player and opposition instructions, and manager-set triggers for pressing and the defensive line. A-10 is the constraint to hold: the trigger lives in the rule pack and the change queue, so no conditional tactic logic enters the tick loop. Opposition instructions need the preferred foot and position competence from B-01 to mean anything.

**Entry:** `/wf intake tactics-tier-3 from brainstorm-realism-additions-20260922`

### B-04 — Model conditions, officials, injuries, and the ball in the air [T-05]
**Inherits:** C-18, C-32, C-33, C-34, C-35, C-36, C-141, C-148, C-149, C-150, C-151, C-202, C-203 · A-13, A-14, A-15 (confirmed), A-37 (rejected) · X-02, X-15, X-24, X-25, X-33

A full referee with tolerance, threshold, advantage, consistency, and a home leaning; pitch wear and weather that interact; injury severity with risk driven by tackles, fatigue, and collisions; and an aerial model with arcs, heading duels, keeper claims, and spin. C-32 is the head start, because the ball already carries height and nothing uses it. This card owns X-02, the 10 percent CPU tripwire, on behalf of every other card. A-15 names what C-35 left unanswered: whether a manager may keep a hurt player on the pitch.

**Entry:** `/wf intake match-conditions-and-officials from brainstorm-realism-additions-20260922`

### B-05 — Rebuild the realism test with shape measures and per-condition bands [T-06]
**Inherits:** C-06, C-37, C-38, C-51, C-55, C-147, C-150, C-152, C-157, C-158, C-159 · A-06, A-16 (rejected), A-24, A-26 (confirmed), A-48, A-52 · X-01, X-03, X-16, X-24, X-26

Pass completion, shot locations, possession by third, and goals by phase added to the three bands, plus a band per condition band. This card was shaped as an extension of the existing workflow rather than a new one, because the `calibration` slice is defined and unbuilt and the match measures are cheapest to design into it now. Batch 10 outgrew that shape. C-51 adds league-table shape, market and wage shape, and a watched season, none of which the `calibration` slice can hold, so the card now splits in two: the match measures stay an extension of the existing workflow, and the season measures belong to B-07's programme. Batch 11 then rejected A-16 and added C-55: a human watches matches as well, as a milestone review rather than a gate on every build. That makes this card the only one on the board that can reach X-01, so it stops being a calibration chore and becomes the test of whether the whole realism argument holds. A-26 is the risk — a review run by hand has to happen early enough to matter.

**Entry:** `/wf intake football-manager-match-engine add shape measures and per-condition bands to the calibration slice`

### B-06 — Model both managers as entities with attributes [T-07]
**Inherits:** C-21, C-30, C-153, C-154, C-155, C-156 · A-08, A-11 (rejected), A-49, A-50, A-51 · X-04, X-27, X-28

A manager record with attributes on both sides, giving the AI opponent character and making the human player's own competence part of the simulation. Batch 31 replaced the weakened-instruction mechanism: C-154 carries every instruction out as given, and a weak tactician makes poor choices instead, so A-11 is rejected. C-153 gives the AI manager four inputs to a match tactic. C-155 answers X-27: the human manager's attribute acts on information, learning speed, and squad reaction. C-156 resolves X-28 as mood, and A-51 is the risk to test with real players.

**Entry:** `/wf intake manager-entity from brainstorm-realism-additions-20260922`

### B-07 — Build the season layer that earns form [T-08]
**Inherits:** C-20, C-25, C-29, C-40, C-41, C-42, C-43, C-44, C-45, C-46, C-47, C-48, C-49, C-143, C-165, C-166, C-167, C-168, C-169, C-170, C-171, C-172, C-173, C-174, C-175, C-176, C-177, C-178, C-179, C-180, C-181, C-182, C-183, C-184, C-185, C-186, C-187, C-188, C-189, C-190, C-191, C-192, C-193, C-194, C-195, C-196 · A-07 (rejected), A-09 (rejected), A-12 (rejected), A-17, A-18 (rejected), A-19 (rejected), A-20, A-21, A-21 (rejected), A-22, A-55, A-56, A-57, A-58, A-59, A-60, A-61, A-62 · X-04, X-07, X-08, X-09, X-10, X-29, X-30, X-31, X-32

Batch 8 made this the largest card on the board. Fixtures, results, and form modelled rather than asserted, a league table with standings, season-long squad availability with suspensions and injury persistence and carried fitness, and a full club calendar with cups, congestion, and rotation. C-43 ships the calendar and the availability together. Form is computed twice, for the team and for each player, and C-42 generates a backstory at save creation so day one reads like football; A-17 is the risk that the generated history does not agree with itself. Batch 9 then took transfers and finances as well, so C-49 makes this card a football-management game rather than a season around a match engine. It is its own programme now, per C-48, and it is larger than the match engine it follows. Three costs ride with it. C-46 changes the event contract when this card starts rather than today, so the migration in A-20 reaches every slice that reads the stream by then. X-09 says nothing on the board tests whether a market, a wage bill, or a season of results reads as football, so B-05's rebuilt test covers matches and stops there. X-10 says a market needs clubs to disagree about a player's worth, and B-01 shows every manager every attribute exactly, so A-22 is the assumption to attack first. Start this card by deciding what a realism test for a season even measures.

**Entry:** `/wf intake season-layer from brainstorm-realism-additions-20260922`

### B-08 — Decide when the player record shape changes [T-01]
**Inherits:** C-08, C-11, C-31, C-46, C-48, C-56, C-201 · A-20, A-29 · X-05

The one card that is a question rather than a build, and batch 9 narrowed it to the player record alone. C-11 changes the shape of that record, the `data-schemas-generator` slice is already complete, and C-31 starts the realism work after the engine ships. Either the record changes now and the engine is built on it, or it changes later and a migration pays for the delay. Batch 12 answered it. C-56 follows C-46's precedent: the record changes when the realism programme starts, and A-29 carries the cost of building match-rules and tactics-and-ai against a shape that will move under them. This card no longer needs an investigation run — it needs recording as a decision, which is what the next distillation should do with it.

**Entry:** `/wf intake investigate whether the player record gains per-position competence before the match engine ships or after from brainstorm-realism-additions-20260922`

## Selection

<!-- recorded when the person picks -->

## How to continue

- Board as a page (session 2, batch 12): https://claude.ai/artifact/YbrN4YkabjMwpfz4k8BUnQ
- Resume: `/wf intake brainstorm brainstorm-realism-additions-20260922`
- Control words: `park <thread>` · `pull <thread>` · `drop <thread>` · `board` · `look it up` · `second opinion` · `done`
- Retire when no thread is live: `/wf close brainstorm-realism-additions-20260922`
