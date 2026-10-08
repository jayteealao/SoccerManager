# The player contract

This page explains the contract between a player's record and the match engine: the scale every rating uses, how the engine reads the ratings, why the top of the scale counts for more, how a player's condition moves his ratings, and how the engine proves that every rating has a job. It also sets the data rules for the four wider kinds of data that later parts of the game will add. It is for people who maintain the engine. For the file formats, see [the data-file reference](../reference/data-files.md). For how the engine plays a match, see [How the engine works](engine.md).

## One scale, stored in tenths

Every rating in the game is on the scale from 1 to 20. The screens show a whole number, as the genre does. The engine stores the rating in tenths, from 10 to 200, in one shared type, `Rating`. A tenth fits in one byte, so a player record stays small and can be copied freely in the hot loops of a match.

Whole numbers are too coarse for the engine. A player who improves by half a point over a season must play a little better, and a convex curve (below) turns small differences near the top into large ones. Tenths keep those differences without making the screens show decimals: the screen rounds, the engine does not.

The scale is shared on purpose. A staff member, a manager, a referee and an agent will each have ratings, and each will use the same `Rating` type. One scale means one set of words, one set of colour bands on the screens, and one way to read any number in the game.

## Why the rescale went first

The old record held whole numbers from 1 to 100. The change to tenths of 1 to 20 was made alone first, with no change to how a match plays. A value of `v` became `2v` tenths, and the engine built the same play values from it, so every match of the old build played tick for tick the same on the new build. Two builds were compared tick by tick on every fixture of the replay gate before anything else moved.

This order matters because the later changes do change play. If the rescale and the play changes had landed together, a changed result could not be traced to either. With the rescale proven alone, every later change in a result belongs to a named play change, and each play change regenerated the replay gate with its own reason.

Old files still load. A version 1 team file converts on load, `2v` tenths with a floor of 1.0, and a replay that embeds version 1 files rebuilds through the same converter.

## The stage blend

An action runs in stages: seeing the option (`see`), choosing it (`choose`), carrying it out (`execute`), and carrying it out with an opponent close (`pressure`). Each stage reads one main attribute and up to three supporting attributes with smaller weights. A pass, for example, is seen with vision, chosen with decisions, carried out with passing, and kept under pressure with composure; each of those stages also reads a little of the others.

No calculation reads one attribute alone. Football ability does not come in isolated pieces: a pass under pressure needs passing, technique and composure together. The blend makes that true in the engine, and it stops any single attribute from carrying an outcome by itself. The weights are content, not code. A weight fitted from real data names its source; a weight set by design says so.

Every attribute the engine reads goes through the blend, with four exceptions that play reads directly: pace sets top speed, technique decides whether a player tries a hard skill, agility decides whether his body pulls it off, and consistency sets how far his form moves from match to match. A source check keeps every other read of a rating inside the contract.

## The curve

A rating passes through the curve `F(r) = 8·e^((r−10)/8)` before the blend, and the blend's result, averaged on the curve, is the stage value. At rating 10 the curve gives 8; each step up is worth more than the one below it.

The curve is not applied a second time to the stage value. On the stage side, the stage value is read in log-odds and scaled by the action's strength `k`. A contest between two players is decided in log-odds: the action's base chance, moved by `k × (F(attacker) − F(defender))`, with a floor and a ceiling on every chance. An action with no opponent reads the stage's share, `σ(k × (F − F(10)))`: 0.5 for an average player, and a logistic in the same log-odds as a contest. A second exponential on the stage value would make the top of the scale count twice; the log-odds scaling keeps the convexity in one place, the curve, and keeps `k` the only strength of an action.

The curve makes the top of the scale matter. In a linear model, the step from 16 to 18 is worth the same as the step from 8 to 10, and a squad of solid players is as good as a squad with stars. Real football is not like that: a top player changes matches. The strength `k` of each action sets how much the gap counts. It is a design value, and the five-match rule (below) is its target.

Top speed does not go through the curve. Pace maps to a real sprint speed in kilometres per hour, and the difference from an average player is amplified about two times. A curve on pace would let pace dominate a match, as it does in games that read it linearly. Pace is held three ways: it acts mainly in long runs while acceleration decides short races, every sprint costs stamina, and its sensitivity rule fails pace when its share of the outcome is out of proportion.

## States, caps and effective ability

A player's condition moves his ratings during a match. The engine groups state into four families: body, mind, familiarity and surroundings. Each family owns its attribute groups: body the physical group, mind the mental group, familiarity the timing of technical actions. Tiredness moves the body family, adaptation to a new country the mind family, and match sharpness the familiarity family. The result is his effective ability, which play reads; his base ratings never change during a match.

Morale and knocks are not built yet. Their families, groups and caps are in place: a knock will move the body family and morale the mind family. Neither has an input until the parts of the game that own morale and injuries add one, and pressure, momentum and weather are still stand-ins that move nothing.

Each family has a cap, and all families together have a total cap:

| Family | Down | Up |
|---|---|---|
| Body | −2.0 | +0.5 |
| Mind | −1.0 | +1.0 |
| Familiarity | −1.25 | +0.25 |
| Surroundings | −0.75 | +0.5 |
| All together | −5.0 | +2.25 |

Effective ability stays within 1.0 to 20.0. The caps keep a player recognisable: a tired star is still a good player, and no combination of good conditions turns an average player into a star.

Some inputs act across matches: sharpness, adaptation, days of rest, and matches at the club. Until the season part of the game carries them from match to match, the team file or the match setup supplies each as a per-match input.

## Hidden values

Consistency and injury proneness are hidden. They act in play like any other attribute, but no screen and no message carries them as numbers. The manager reads a word with a confidence: "not yet known" before the player has played for the club, then a tentative word, then a firm one as the matches mount up. A later part of the game will let the staff set the confidence instead.

## Every rating has a job

Every attribute names the statistic it moves and the direction it moves it, in its job in the attribute file. Height, age and nationality have jobs too: height adds standing reach in the air and costs a little turning, age makes a player fade faster late in a match, and nationality acts through adaptation to a new country. Build is a word made from strength and balance; it has no job in play, and a test proves that the word follows from the attributes.

A job is a promise, so the engine proves it. Each job has a sensitivity rule in `content/sensitivity.json`: its statistic, a least move from rating 8 to rating 16, and a ceiling on its share of the match outcome. A rule that only checks that something moved would let an attribute with a token weight pass. A rule with a least move fails it. A rule with no ceiling would let pace win matches alone. A rule with a ceiling fails it.

The rules are measured from one balanced design rather than one attribute at a time. In each match of the design every job of the measured side is at its low or its high level, each job high in exactly half the matches and independent of the others. One set of matches then measures every job, each averaged over the levels of all the others. Each job's share of the outcome is its effect on goal difference against the effect of every job together, from a match with every job low to a match with every job high.

A rule's word comes from the uncertainty of the measure, not only its value. A bootstrap over matches gives an interval for the move and for the share. A rule passes only when both intervals clear their thresholds, fails when either is surely on the wrong side, and is not sure otherwise. Not sure is never a pass: a job whose statistic is too rare to measure has not proven its job.

## The five-match target

The curve's strength is set against one target: a top player looks clearly better than an average player in the same role in about two of three runs of five matches. One player of a fixed team is replaced by a copy with every visible attribute at 16 or at 10, and each copy plays the same five matches. The top copy should have the higher mean match rating in about two of three runs.

The target is about what a manager sees. Over one match, luck hides most differences between players. Over a season, a top player is plainly better. Five matches is the span over which a manager judges a new signing, and two of three is clear but not certain. A curve that is too weak makes stars invisible; one that is too strong makes every match a foregone conclusion. The target is to be met by the curve's strength, not by the weights of the match rating, which would only change the measure.

It is not met yet. The full run measures 0.815 (0.775 to 0.853) pooled over four roles, against the target of 0.667 ± 0.07. By role it reads goalkeeper 0.920, centre back 0.355, central midfielder 1.000 and striker 0.985. The pooled figure hides a role the other way round: a top centre back has the higher match rating in only about one run in three, because the match rating does not yet reward much of what a centre back does. Tuning `k` alone cannot fix both ends at once, so the target is carried to the later work on how each action is resolved, which changes what a centre back's attributes move.

## Where the rules stand

The full run of the rules records 12 passes, 13 fails and 15 not sure of 40. The rules are kept as they are, red where they fail, so each fail stays visible: a fail is a job the engine does not yet do, not a rule to soften. Most of the fails and not-sure words belong to actions that play rarely or not at all: play makes no crosses, and aerial balls and offsides are rare, so the crossing, command-of-area, heading, jumping, height and communication rules cannot pass until play makes those actions.

One attribute breaks the ceiling. Decisions is the main attribute of the choose stage of the pass, chip, cross, shot, long shot and dribble, and it carries 0.474 (0.331 to 0.674) of the outcome against the ceiling of 0.20, while its own statistic, expected goals per shot, does not move. Composure (0.193), agility (0.186), nationality (0.189) and strength (0.144) sit near the ceiling. The share of decisions is the named fail that the later work on how players choose their actions must bring under the ceiling, by spreading the choose stages over vision and anticipation; the weights are not cut here, because a cut that only lowers the share would hide the job decisions does not do.

## The wider kinds of data

The player's data rules are a job for every field, one scale, and a check that each job works. Four more kinds of data will join the game. Each kind gets its own rules, decided as the kind is examined, on the same `Rating` type. No field of these kinds is stored until the part of the game that owns it adds the field together with its job.

### Team numbers

Team numbers act only on actions that need two or more players at once: a pressing chain, the offside line, a combination between two players. An action of one player never reads a team number.

The player keeps what is personal: his rating in each position and his grasp of each tactic. Tactic familiarity lives with the player. The team's familiarity is computed from the players on the pitch and never stored, so it can never drift from the players who are actually playing. Actions of one player read the player's grasp; team actions read the team value. Partnerships and cohesion are team numbers and act only on actions of two or more players.

### Other rated people

Staff, managers, agents and referees use the same scale in tenths. Each of their ratings names its job, as a player's attribute does, and a season-long sensitivity check runs before each release: a staff rating acts over weeks, not within one match, so its proof is a season, not a set of matches.

A referee's numbers are hidden. The manager sees a description before the match, such as "strict with cards", and learns more from past matches, the way he learns a player's hidden values.

### Relationships and history

A relationship between two people holds two numbers: on-pitch understanding, earned from minutes played together, and personal liking, moved by personality, groups and events. They are separate because two players can combine well and dislike each other. Both numbers fade when the two are apart, at different speeds.

The game stores every pair at every club in the leagues that play in full, and no pair at the clubs that are only simulated lightly. Each player keeps every event of his recent matches, for replays and deep reports, and summaries of older matches.

### Fixed facts

Height, age, nationality and build are the player's fixed facts, as built in this contract. Height, age and nationality are stored with their jobs; build is derived from the attributes and never stored, and weight is not stored at all.

Club identity is a fact about a club, not a player. It acts only off the pitch: it shapes who the club hires and buys and what the board and the fans expect, and it reaches play only through those choices. Its creation, drift and the reaction to a manager who wins against it belong to the part of the game that builds club identity.

### Not stored yet

These names of the wider kinds have no field in any stored file of the engine until their owner adds them with a job: `relationship`, `understanding`, `liking`, `identity`, `referee`, `staff`, `agent`, `cohesion`, `partnership`, `familiarity_team`. A test keeps them out of the stored data.
