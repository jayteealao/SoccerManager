# How a team keeps its shape

This page explains how the engine decides where each player stands and how fast he moves there, how restarts take their time, and how the AI manager chooses its style, lineup and bench. It is for people who maintain the engine. The values these mechanisms read live in the content files: see [the data-file reference](../reference/data-files.md) for each field. To measure the result against real matches, see [Measure a tuning change](../how-to/calibration.md#read-the-team-shape-bands-and-the-figures).

## Two phases and the loose ball

Each team is in one of two phases: in possession or out of possession. A team changes phase only when a player of the other team controls the ball. A loose ball, a deflection and a pass in flight keep each team in its last phase.

Before this rule, a loose ball and every pass in flight counted as out of possession for both teams. So during every pass both teams took their defensive shape, and the passing team's players ran back for a moment before running forward again. With the phase held until a real change of control, a team that passes keeps its attacking shape until it loses the ball.

## Where a player stands

A player's place comes from his formation slot, scaled to the ground. Three things move him from the slot:

1. **His role and duty.** Each role holds an offset per phase, and each duty moves the player up or back.
2. **The stand table.** Each position has a stand per phase: an offset from its formation slot, never from the ball. The table was seeded from real per-position figures. It is switched off by default (see [The stands are off](#the-stands-are-off)).
3. **The block law.** The whole block then moves with the ball: up and down the pitch, and across it. Players far from the ball follow late, by a delay that grows with their distance from the ball.

Nothing in the content sets a gap between the lines or the share of the slide a player follows. The gaps and the team's width come from where each role stands and how the block moves. The content loader refuses a field that would set such a target, because a target gap would make the shape figures pass by construction instead of by play.

### Stand options

Each role carries a short menu of stand options for each phase: drop short, widen, move into the half-space, overlap, or underlap. A player takes the first option on his menu whose condition holds, read from the ball, the carrier and where his team-mates stand. For example, a centre-back widens while his keeper has the ball, and a midfielder drops when the carrier is well behind him and nobody stands in the space.

No option names a partner. Whether a winger goes wide while his full-back underlaps comes from both players' menus and the situation, not from a fixed pair.

### Rest defence

While a team has the ball, a minimum number of outfield players stays goal-side of the ball. The minimum comes from the mentality: a defensive mentality keeps more players back than an attacking one. The rule works in four layers:

1. The duties decide who goes forward and who stays by default.
2. If fewer than the minimum stay, the players with the most defensive duty and the least far forward are held back.
3. Above the minimum, a player with a support or defend duty goes forward only when a team-mate covers his lane behind him.
4. When a full-back goes forward, a team-mate who would also go stays to cover him, if the two know each other well enough (their familiarity).

The keeper never counts toward the minimum, and the minimum never exceeds the outfield players less one, so a team with a player sent off still has players to attack with.

## How fast a player moves

Every movement target carries an urgency grade:

- **Urgent**: a press, a recovery run when the cover goal-side is missing, a timed run, and a loose ball the player reaches first.
- **Normal**: a return to shape from far away, offering a pass, and following the slide.
- **Calm**: a player in shape far from the ball, and a player with no job at a dead ball.

A pace selector turns the grade into one of six pace levels, as real tracking data reports them: stand, walk, jog, run, high speed and sprint. A sprint is capped by the player's own top speed. Steering then moves the player at the chosen speed, not at his top speed.

The player's attributes and state change the choice:

- **Work rate** sets how often an urgent job gets a sprint, not how fast he walks or jogs. Two players with the same job therefore move differently.
- **Determination and professionalism** set how a tired player saves energy: a player high in both keeps his urgent sprints and slows a step on normal jobs, while a player low in both skips the less urgent sprints first.
- **Instructions and the game state** move a grade by one step: a counter raises pressing and support jobs for the seconds after the team wins the ball, and a team trailing late raises them too. No tempo or pressing level lifts running pace: tempo is how patiently a side circulates the ball, not how fast its players run. A team that leads late and wastes time lowers its normal jobs one step, but only while the ball is dead. In open play both teams keep their normal grades, and an urgent job never drops below urgent.

Before the selector, every player ran to every target at full speed. Matches looked frantic, every player moved alike, and players ran out of energy far too early. The selector brings the sprint and high-speed counts per minute of ball in play toward the real figures, and lets a tired side show it.

## Restarts and the walk-on

When the ball leaves play, it waits where it went out for a drawn time, the time it takes to fetch it, and then appears at the restart spot. The tick the ball appears is marked as the restart, so the rule checker still finds the ball at the right spot.

A stoppage lasts as long as its visible steps take on the pitch, plus a drawn extra:

- **The visible steps.** The taker walks to the ball, the players the law moves walk away from it, and a wall forms. Players with no job at the restart stand or walk.
- **The drawn extra.** The unseen part, such as fetching the ball or a referee's word. It is drawn from the restart wait table (`restart-waits.json`). A script writes the table from real dead-ball spells less the engine's own measured visible steps. Ball-in-play time is never an input: the minutes of ball in play follow from play.

A team that leads late takes longer over its restarts, and time wasting lengthens the same drawn part. A taker may still restart quickly, with no drawn extra, when a team-mate is free and the opponents are out of shape. This happens more often when his team trails, and never for a team that wastes time. A restart that never gets ready is taken at a hard limit and logged.

Before each half, the players walk from the touchline to their kick-off places. The replay shows the walk-on, but it is not match time: the clock and the ball-in-play figure start at the first kick. Players no longer jump to their places at kick-off.

## The defensive jobs

Pressing, holding position and defensive movement read the attributes the design decided:

- **Pressing.** Positioning leads how well a player presses, with anticipation and decisions as the shown supports and vision as a small weight. Work rate still sets how much he presses: how many players press and how far they go.
- **Holding position.** Teamwork leads, with concentration, the player's familiarity with the tactic, and consistency as supports. It acts for every player in both phases.
- **Covering and recovering.** Covering a team-mate and stepping up or dropping with the line read positioning first, with anticipation and concentration. Recovery runs, how often and how hard, read work rate first, with pace and acceleration.

[The player contract](player-contract.md) lists the stages and their weights.

## Counts are never set

No mechanism here sets a count or a gap that a realism band then measures. Stoppage counts come from how often the ball leaves play. The line gaps and the width come from where each role stands. The slide comes from the block law, and only the far-player delay is tuned against the slide band. The bands check the result against real matches; they never feed back into play. A band that misses its real value therefore points at a mechanism to fix, not at a number to set.

## The stands are off

The stand table moved the back line toward the real phase shapes. But it was seeded as if the ball always sat on the halfway line. Out of possession it lifted the back line far up the pitch and opened the space behind it, and shots per team rose far above the real range.

The table therefore sits behind the tuning flag `team_stands`, off by default. Off, every stand reads 0: each player stands on his formation slot plus his role's offset. The table is still loaded and checked, and the tests that need it turn the flag on. The per-phase depth and width bands, and the slide band, only report while the stands are off, because they measure the block the stands set.

The stands come back with the later work on off-ball decisions: the table is seeded again from the engine's own measured positions, with a block depth that depends on where the ball is, and the flag goes on once the core realism bands hold with it.

## Styles and the AI manager's pick

`content/styles.json` holds eight named styles: possession, low block, high press, counter-attack, direct, wing play, gegenpress and park the bus. A style sets a mentality, the team instruction levels, duty hints by position, and a list of formations that suit it. It also names two or three statistics that should show it, each with a direction: a possession side should have more of the ball, a low block should concede less expected goals.

### The staff advice

Before each match, an AI manager asks for a fit estimate of each style for its squad. For each style, the advice picks the lineup the AI would field in each of the style's formations, keeps the formation with the best summed rating, and weighs those players' attributes by the style's key jobs: pressing for a high press, holding position for a low block, passing and touch for possession. The estimate is taken relative to the same players' general level, so a strong squad does not simply pick the most demanding style. The advice adds a noisy error to each estimate, drawn from a random generator of its own, seeded from the match seed, the side and the club. It reads no match random stream, so the advice changes no other draw.

The AI starts the match in the style with the best estimate. The estimate itself lives only inside that choice: no player, team or match state keeps it, and play never reads it. A test plays two matches whose estimates differ but whose tactics are set by hand, and they are identical tick for tick. This keeps the rule that a squad's fit has no number of its own: a slow, tired squad presses badly because its players are slow and tired, not because a fit number says so.

A side that a person manages keeps the default tactics as its starting point; a person does not see the advice. The pick sits behind the tuning flag `ai_style_pick`, on by default.

### The lineup and the bench

The AI picks its starters for the chosen formation by each player's rating at each slot's position, not by the position listed in the squad. A listed player keeps his slot unless another player is clearly better there: the listed player gets a margin (`ai.out_of_position_margin` in `tactics.json`). The pick fills slots from the best player-and-slot pairs first, then swaps pairs of players when a swap raises the summed rating.

The bench is picked as cover: a spare goalkeeper, then at least one player for each line (defence, midfield and attack), then a full-back and a winger when the squad has a spare, then the positions the formation uses most. A substitute is the bench player with the best rating at the slot. The number and the timing of substitutions do not change: only who comes on.

With the shipped squads, which hold one spare per central position, some substitutions still put a player out of position: no bench of seven can cover every slot twice.

### The styles suite

`engine-cli calibrate --suite styles` plays every ordered pairing of two different styles, with the same squad on both sides, and a formation drawn from each style's list. It reports each style's points per match, the styles it loses to clearly, and a dominance flag: a style with no clear loss, or with more than a fifth of all points, would be a style with no weakness. It also flags an identity statistic that a style does not move in its direction. Every change run plays a small sample, one match per ordered pairing, and reports it beside the verdict; the suite judges no band. See [Run the styles suite](../how-to/calibration.md#run-the-styles-suite).

The suite tests presets only. The AI does not yet adapt during a match or remember which styles worked; that comes with later work on the AI manager.

## Tactical primitives

A tactical primitive is one setting of the tactics board: tempo, mentality, and the settings that later work adds. Each one is data in `content/tactics.json`, held to the same frame:

- **Its kind.** A graded primitive has five ordered levels, a choice primitive lists its choices, and an on/off primitive has two states.
- **Where play reads it.** It sets one or more plan inputs, and each input is read at one named hook: an option weight, a threshold, a trigger, a window or a shape offset. The hook list is fixed in the engine. A new way of scoring the carrier's options must read the same hooks, so a primitive and its tests keep their meaning when the scoring changes.
- **What it moves.** It names a signature statistic and the way it moves as the level rises, primary first. The harness plays pairs of matches one level apart, with everything else the same, and checks that the primary statistic moves that way by more than the noise of the pairs. The check reads played matches, not the code.
- **How big each level is.** Each level is fitted to real teams. A table of real club seasons (`content/signatures.json`) splits the teams into five quintiles of the statistic. The engine plays more passes than real teams, so a level is not fitted to the real value itself: it is fitted so that its ratio to the default level matches the ratio of its quintile's median to the middle median. The gap between the engine's and the real values is recorded for the tuning of pass volume. The held-out data (the Wyscout German league 2017/18, the 2018 World Cup and Euro 2016) is never read.

No engine code names a style, and no place where a player decides reads a setting by its name. A style is a set of primitive levels in content; the trait conflicts follow the level's order against the default, not its name. A source test checks both.

### Tempo is patience

Tempo used to be added to every pass and taken from every hold and dribble, so a slower tempo made the carrier hold the ball and dribble into pressure. Tempo is now how patiently a side circulates the ball: when the carrier picks his pass, a short pass that does not go forward gains the level's value and a forward pass loses it. It moves which pass he picks, never whether he passes, holds or dribbles. Its signature is passes per possession, with the share of passes made before the first forward pass beside it. No tempo level lifts running pace.

### The in-possession primitives

How a side keeps and moves the ball is set by nine graded primitives and three choices. Each is a bias on an option the carrier or a runner already has, so play with every setting at its default stays the engine's own choice model.

- **Passing directness** weighs a pass by how far it gains toward goal. A pass back or across gains nothing, so a direct side plays forward sooner, not just longer.
- **Build-up** weighs a long pass out of the side's own third, the keeper's included, and lengthens the goal kick toward going long. The keeper follows the team's build-up; there is no keeper setting.
- **Attacking width** spreads the shape while the side has the ball, and weighs a pass to a team-mate in a wide channel. Out of possession the shape keeps the formation's width.
- **The attacking channel** favours passes into the chosen channels of the middle and final thirds: left, right, both wings, centre, centre and left, centre and right, or balanced.
- **The final third** weighs a shot from outside the box and reaches further out at the higher levels.
- **Dribbling** weighs the dribble option. A take-on counts once for each opponent the dribbler passes within pressing distance.
- **Creative freedom** scales the spread of each player's choices around the plan. No real data records which option a player passed over, so its levels are spread by hand and only their direction is checked.
- **Forward runs** set how far from the defensive line a forward may start a timed run. At the higher levels an onside forward that near the line runs beyond it when the carrier is free and the forward is ahead of him within a pass's reach, so a pass played too late is offside. At the lower levels a forward that near the line checks short toward the ball from his place, then goes back. A run goes on while the pass travels forward, and a pass back or across ends it. At the default no forward starts one, so the farther from the line a run may start, the more runs there are. Only the wingers and strikers make these runs for now; runs chosen by role and duty come with the off-ball work.
- **Full-back runs** choose underlap or overlap first in a full-back's menu, and their frequency scales how near the ball a full-back must be to go and how far he runs. A run counts as SkillCorner counts it: a run that starts level with or behind the ball and ends level with or ahead of it, outside the ball for an overlap and inside it for an underlap.
- **Crossing frequency and cross type** sit on the board with seeded values.

When the ball is wide in the final third, the forwards run into the box, at every level, so a cross has a team-mate to find. The engine still plays few open-play crosses: with the stands off, wide players hold their midfield places and rarely carry the ball high and wide. The crossing primitives and the central channel choices are therefore not fitted yet; they wait for the off-ball work that re-seeds the stands.

### Transitions

Three primitives set what a side does in the seconds after the ball changes side in open play. Each acts only inside a window that opens when the ball changes side and closes at the next dead ball, so a restart opens none.

- **On winning the ball** runs from always keeping shape to always countering. For the seconds a real counter-attack takes to reach the final third, while the side keeps the ball, a counter weighs a forward pass up and holding the ball down, lets the forwards start a run in behind from farther out, and at its two counter levels lifts the pace of the support runs. Keeping shape does the opposite. The window's length and the zone a fast break starts from come from the counter-attacks Wyscout's analysts tagged in real matches, and the levels are fitted to how many fast breaks real teams make a match.
- **On losing the ball** runs from always regrouping to always counter-pressing: inside the counter-press window, the nearest players start to press from farther away, or from nearer, or not at all.
- **Counter-press length** sets how long that window stays open, from about 2 to about 8 seconds.

The default levels are neutral, so a side that sets none of them plays as before. The counter-press levels are on the board but not fitted yet: in this engine, more players pressing for longer after a loss do not win the ball back, so the other side simply keeps it longer. They are fitted with pressing intensity and the tackle decision, which change how a press wins the ball.

The running pace no longer rises with a high press. It rises during a counter, where real runners break at speed.

Two style pairings play far more shots than real matches: a high press against itself, and the counter-attack against the gegenpress. A probe of each pairing traced about half of the excess to the attack duty that those styles give their players, which pushes them higher and makes the carrier take more risks, and the rest to the engine's shot volume, which is high even between two default sides. The duty hints belong to the style presets and the shot volume to the tuning work, so neither was changed here.

### Mentality is an appetite for risk

Mentality used to add a flat bonus to shots and forward passes. It is now only how much the side fears losing the ball: a pass, a dribble or a hold loses the mentality's loss weight times its chance of failing times the danger of losing the ball there. A ball lost near the own goal hands the opponent a chance, and one lost near the other goal gives up the side's own, so the danger is highest at either end. An attacking mentality fears a loss least, and its weight can go below 0, so it seeks risk. The mentality also sets how many players stay behind the ball. Where the block stands belongs to the line height, which moves the block only while the other team has the ball.

A shot carries no loss term. A missed shot hands the ball back at the opponent's own goal line, where it threatens nothing, and the shot spends the side's threat instead of losing it. Counted as a loss at the shooter's feet, the term held back every shot but the closest ones, so the shots left were twice as good and goals rose by half.

## Game-state rules and player instructions

A tactic has three parts: the base tactics the manager sets, up to six game-state rules in priority order, and player instructions for single players. A person and the computer manager hand in the same three parts, and a style may carry rules and instructions as part of its preset.

### Rules

A rule reads "when all these conditions hold, set these primitives". The conditions are the score (leading or trailing by some goals, or level), the time left (so many minutes or less to the end of normal time, or of extra time once it has started), red cards for either side, the run of play (expected goals for less expected goals against over the last 15 minutes) and the team's energy (the mean of the outfield players on the pitch). No condition is a fixed minute of its own: time left is one condition among the others.

The match checks every side's rules each tick, after the tick's events. The tactics in force are the base with the rules that hold applied over it, from the lowest to the highest, so the higher rule wins a primitive that two rules set. When the set of rules that hold changes, a rule change enters the change queue the manager and the computer manager share, on that tick. A rule that stops holding lapses the same way, and the setting under it returns. A manager's own change sets the base, and the rules that hold still apply over it.

A new formation or role waits for the next stoppage that admits a tactics change. Everything else starts at once, and the players take it up over time: each player reads the new levels after a delay set by his shape stage, the stage that holds position and keeps to instructions, from at once at its top to 30 seconds at its bottom. Until his moment comes he plays as before. What the side reads as a whole, such as how many players press, switches when half the side has taken the change up. A substitute comes on with the change already taken up. A rule change that comes while another is being taken up leaves each player on what he was reading.

### Player instructions

A player instruction sets one primitive for one player in one formation slot. Ten primitives take one: dribbling, crossing, cross type, final third, forward runs, width, pressing, tackling, on winning the ball and on losing the ball. The player's own value replaces the side's at the same hook, the place where play reads it, so an instruction never adds a new way to decide; it gives one player another value at the same point. Pressing gives the player his own reach and willingness to press; how many players press stays the side's.

Forward runs reach only the wingers and strikers, because a run in behind starts from the attack line. An instruction for any other slot is refused when the tactic loads, in any formation the tactic or one of its rules can take. Runs from deeper positions, chosen by role and duty, wait for the off-ball work.

`engine-cli primitives check --player <slot>` holds each instruction to the same frame as the team primitives: pairs of matches that differ only in one player's instruction, one level up from the default. The player's own statistic must move beyond the noise of the pairs, and the same statistic pooled over his ten team-mates must stay inside it.

A saved match keeps each side's rules, instructions, the rules in force and the change its players are still taking up, so a resumed match plays on exactly as the unbroken one. The match stream sends each side's rules and instructions in its hello, and a replay keeps them, so a re-simulation fires the same rules again.
