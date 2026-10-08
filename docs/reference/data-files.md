# Data files

This page describes every file the engine reads and every file the engine writes. The engine reads the content files from the content folder. The engine writes the runtime files to the data folder.

The content files ship in the `content/` folder of the repository.

The engine reads every data file from this folder. Nothing is embedded in the binary. A missing file, an unknown key, a value outside its bound, or an unknown `schema_version` stops the run with a message that names the file and the field.

## Where the engine looks

The `--content-dir` flag, if given, names the folder exactly: it must hold `attributes.json` or the run fails. Otherwise the `SM_CONTENT_DIR` environment variable, if set, names the folder exactly, with the same rule. Only when neither is set does the engine probe `./content` in the working directory, then the `content` folder beside the binary, taking the first that holds `attributes.json`.

Runtime output (`owner.id`, `matches/<match.id>/stats.json`, `matches/<match.id>/snapshot.smsn`) goes to `SM_DATA_DIR`, which defaults to `%LOCALAPPDATA%\SoccerManager` on Windows.

## Files

| File | Schema version | Holds |
|---|---|---|
| `attributes.json` | 3 | The attribute schema: 30 to 50 names in four groups, each with its job in play (two of them hidden values), the stage tables that blend them for every action, and the skill gates |
| `tuning.json` | 5 | Engine constants, the attribute contract with the state caps, the body jobs and consistency, decision weights, injury rates, the generator's world spread, fatigue curve, stream buffer, the hidden values' words and the match rating |
| `rules/default.json` | 4 | The rule pack |
| `tactics.json` | 1 | Ten formations, mentalities, team instructions, roles, duties, and the AI manager's settings |
| `teams/default-a.json`, `teams/default-b.json` | 1 | The two default clubs (`engine-cli generate --seed 1` and `--seed 2`) |
| `commentary/en.json` | 1 | The English commentary lines, grouped by event kind and match situation |
| `realism-bands.json` | 2 | The accepted realism bands the calibration run checks: four from version 1 and eleven from real-match data. They are acceptance criteria, never tuning values |
| `fast-model.json` | 3 | The fast model fitted from full-engine results, with the engine id of the results it came from. The content hash does not read it |
| `sensitivity.json` | 1 | The sensitivity rules: one rule per attribute job and per body job, the thresholds they are judged by, the size of a run, and the five-match rule. Only the sensitivity run reads it; the content hash does not |

Every file starts with `"schema_version"`. A file with another version is refused: `content refused: rules rules/default.json: schema_version 7; this build reads 4`. Team files and `tactics.json` are read in version 2 and in version 1, which converts on load (see [teams/*.json](#teamsjson) and [tactics.json](#tacticsjson)). `attributes.json` is read in version 3 and in versions 2 and 1, and `tuning.json` in versions 5, 4, 3 and 2; an older version converts on load with the contract tables of the build that introduced it, copies compiled into it that tuning the shipped files never changes, so a replay that embeds an old file plays the same way however the shipped tables move. A version 2 tuning file converts through version 3; a version 3 file gains the state caps, the body jobs, `fatigue.group_weights` and `fatigue.sprint_cap` from the second copy; a version 4 file gains `engine.contract.consistency`, `hidden`, `match_rating` and `generator.world.hidden` from the third copy. A version 1 or 2 attribute file converts to version 3 the same way (see [Version 2 files](#version-2-files)). Any other version is refused the same way.

## attributes.json

`attributes` is a list of `{ "name", "group", "job", "hidden" }`. Names are 2 to 32 characters and unique. Groups are `technical`, `mental`, `physical`, `goalkeeping`. The count is 30 to 50. Four names are required because play reads them directly rather than through a stage table: `pace` (the top-speed map), `technique` and `agility` (the skill gates), and `consistency` (the form offset). A schema without one of them is refused by name: `content refused: attributes attributes.json: attributes: required attribute agility is missing`.

`hidden` is optional and `false` when absent. A hidden value acts in play like any other attribute, but no screen and no message shows it as a number: a reader gets a word and how sure the club is of it (see [hidden](#hidden)). Its job is the `spread` stage or an `execute` stage. No state moves a hidden value, the league generator draws it apart from the player's level, and the club strength on the start screen leaves it out.

The shipped schema (37):

- technical: passing, dribbling, first_touch, finishing, crossing, heading, long_shots, tackling, technique
- mental: positioning, vision, decisions, composure, anticipation, work_rate, aggression, concentration, teamwork, consistency (hidden)
- physical: pace, acceleration, stamina, strength, agility, balance, jumping, natural_fitness, injury_proneness (hidden)
- goalkeeping: handling, reflexes, aerial_reach, one_on_ones, kicking, throwing, command_of_area, communication, rushing_out

Injury proneness leads the `injury` action's `execute` stage alone, and its direction is up: the more prone the player, the more often he is injured. Consistency is in no stage table. Its job is the `spread` stage: it sets how far the player's form moves his ratings from match to match and through a match (see [contract](#contract)).

### Jobs

Every attribute has a `job`: `{ "action", "stage", "statistic", "direction" }`. `action` is one of the actions below; `stage` is `see`, `choose`, `execute`, `pressure`, `top_speed` (pace only, with the action `sprint`), or `spread` (a hidden value only, with no `action`); a job without an action at any other stage is refused, naming the attribute; `statistic` names the match statistic the attribute moves, 3 to 120 characters; `direction` is `up` or `down`, the way the statistic moves as the rating rises. The stage tables must put the attribute in the stage its job names, or the file is refused, naming the attribute.

### Stage tables

Each action plays in up to four stages: seeing the option (`see`), choosing it (`choose`), carrying it out (`execute`), and carrying it out with an opponent close (`pressure`). `actions` maps each action to its stages, and each stage to a table `{ "main", "supports", "source" }`: `main` and each of up to three `supports` are `{ "attribute", "weight" }`, weights above 0 and up to 10, every support lighter than the main attribute. `source` is `design`, or `fitted:<reference>` for weights fitted from real data.

A player's value in a stage is the weighted mean of his attributes' values on the curve (see [contract](#contract)), so a player rated 10 throughout has the value 8.0 in every stage. Play reads the stage value in a contest against an opponent, or as a share from 0 to 1 that is 0.5 at rating 10.

The actions are `pass`, `chip` (a pass longer than 25 m), `cross` (a pass from wider than 20 m into the penalty area), `shot`, `long_shot` (from outside the penalty area), `header`, `penalty`, `dribble`, `receive`, `tackle`, `shield`, `intercept`, `press`, `shape`, `block`, `sprint`, `endure`, `turn`, `stay_up`, `aerial_reach`, `recover`, `injury`, `hold`, `save`, `claim`, `one_on_one` (a shot from within 12 m of the keeper), `keeper_kick`, `keeper_throw`, `organise`, and `rush`. Which stages each action has is fixed by the build: a stage play reads needs a table, and a table for any other stage is refused. Every attribute other than pace is in at least one table.

A table is refused by stage, for example:

- a missing table: `content refused: attributes attributes.json: actions: pass.see has no table`
- a stage play does not read: `content refused: attributes attributes.json: actions: sprint.see is not a stage play reads; remove it`
- a name that is not in the file: `content refused: attributes attributes.json: actions: tackle.choose: attribute aggression is not in the file`
- a support as heavy as the main attribute: `content refused: attributes attributes.json: actions: pass.execute: support technique weighs 3, not below the main weight 3`

### Gates

`gates` holds `chip` and `take_on` (a dribble with an opponent close), each `{ "technique_try", "agility_pull_off", "penalty_k" }`. A player tries the skill only from the technique rating `technique_try` (1 to 20); below the agility rating `agility_pull_off` (1 to 20) his execution loses `penalty_k` log-odds (0 to 5).

### Version 2 files

A version 2 attribute file has no hidden values. It still loads: `injury_resistance` becomes `injury_proneness` in the definitions and the tables, hidden, with its job turned up, and `consistency` is added from the build's third copy of the contract tables. A version 1 file converts through version 2.

### Version 1 files

A version 1 attribute file holds `{ "name", "group" }` only. It still loads: each attribute takes its job, and the file its stage tables and gates, from the build's first contract tables. A name those tables have no job for is refused by name.

## tuning.json

### engine

Units are metres, seconds, metres per second, and ticks. A value outside its bound is refused, for example `content refused: tuning tuning.json: engine.keeper_catch_chance: greater than 1`.

| Field | Unit | Default | Bound |
|---|---|---|---|
| dt | s per tick | 0.02 | exactly 0.02: the match clock runs at 50 ticks per second |
| decision_interval_ticks | ticks | 1 | 1 to 50 |
| arrive_radius | m | 3.0 | 0 to 30 |
| separation_radius | m | 1.5 | 0 to 15 |
| separation_strength | m/s | 6.0 | 0 to 60 |
| min_player_distance | m | 0.4 | 0 to 4 |
| ground_friction | m/s² | 4.0 | 0 to 40 |
| air_drag | per s | 0.05 | 0 to 0.5 |
| gravity | m/s² | 9.81 | 0 to 98.1 |
| restitution | ratio | 0.55 | 0 to 1 |
| ball_max_speed | m/s | 40.0 | 0 to 400 |
| reach_radius | m | 1.0 | 0 to 10 |
| keeper_reach | m | 2.6 | 0 to 26 |
| keeper_depth | m | 3.0 | 0 to 30 |
| shot_noise | rad | 0.5 | 0 to 1.2 |
| keeper_catch_chance | probability | 0.86 | 0 to 1 |
| carry_step | m per tick | 0.4 | 0 to 4 |
| crossbar_height | m | 2.44 | 0 to 24.4 |
| reach_height | m | 2.0 | 0 to 20 |
| control_speed | m/s | 12.0 | 0 to 120 |
| control_cooldown_ticks | ticks | 25 | 0 to 250 |
| press_distance | m | 18.0 | 0 to 180 |
| pass_arrival_speed | m/s | 3.0 | 0 to 30 |
| pass_max_speed | m/s | 28.0 | 0 to 280 |
| shot_speed | m/s | 27.0 | 0 to 270 |
| shot_range | m | 21.0 | 0 to 210 |
| aim_noise | rad | 0.06 | 0 to 0.6 |
| compactness_x | ratio | 0.3 | 0 to 3 |
| compactness_y | ratio | 0.3 | 0 to 3 |
| cover_distance | m | 3.0 | 0 to 10 |
| cover_channel | m | 12.0 | 0 to 34 |
| back_line_gap | m | 12.0 | 4 to 30 |
| anchor_tolerance | m | 15.0 | 0 to 150 |
| anchor_ball_distance | m | 30.0 | 0 to 300 |
| anchor_grace_ticks | ticks | 100 | 0 to 1000 |
| foul_base | probability per tackle | 0.1 | 0 to 1 |
| foul_aggression_weight | log-odds per curve point | 0.05 | 0 to 4 |
| foul_tackling_weight | log-odds per curve point | 0.025 | 0 to 1 |
| foul_ball_loss | ratio | 0.6 | 0 to 1 |
| yellow_base | probability per foul | 0.05 | 0 to 1 |
| yellow_aggression_weight | probability per foul at a commitment share of 1 | 0.15 | 0 to 1 |
| red_base | probability per foul | 0.005 | 0 to 1 |
| foul_cooldown_ticks | ticks | 150 | 0 to 1000 |
| foul_booked_factor | ratio | 0.15 | 0 to 1 |
| tackle_win_base | twice the even tackle's win chance | 0.5 | 0 to 0.5 |
| tackle_reach | m | 1.0 | 0.5 to 3 |
| press_engage | m | 3.0 | 0 to 10 |
| tackle_dribble_win | probability per tackle | 0.0 | 0 to 1 |
| lone_line_hold | ratio | 0.0 | 0 to 1 |
| restart_delay_s.kick_off | s | 5 | 0 to 60 |
| restart_delay_s.throw_in | s | 3 | 0 to 60 |
| restart_delay_s.corner | s | 8 | 0 to 60 |
| restart_delay_s.goal_kick | s | 6 | 0 to 60 |
| restart_delay_s.free_kick | s | 8 | 0 to 60 |
| restart_delay_s.penalty | s | 15 | 0 to 60 |
| restart_delay_s.drop_ball | s | 20 | 0 to 60 |
| restart_ready_radius | m | 1.0 | 0.1 to 10 |
| injury_per_tackle | probability per tackle | 0.004 | 0 to 0.2 |
| injury_per_minute | probability per player per minute | 0.0002 | 0 to 0.01 |
| decision.* | weight | see below | see below |
| contract.* | see below | see below | see below |
| clearances.aim_spread | rad | 0.6 | 0 to 1.6 |
| clearances.cross_reach | m | 2.0 | 0 to 3 |
| clearances.cross_chance | probability | 0.0 | 0 to 1 |
| clearances.cross_speed | ratio | 0.7 | 0 to 1 |
| clearances.cross_spread | rad | 1.6 | 0 to 3.2 |
| clearances.cross_loft | m/s | 4.0 | 0 to 15 |
| clearances.wide_chance | probability | 0.0 | 0 to 1 |
| clearances.wide_depth | m | 16.5 | 0 to 52.5 |
| xg.intercept | log-odds | -4.191 | -10 to 10 |
| xg.distance_coef | log-odds per m | -0.0441 | -2 to 0 |
| xg.angle_coef | log-odds per rad | 6.3836 | 0 to 10 |
| shots.loft_max | m/s | 7.0 | 0 to 15 |
| shots.save_high | probability | 0.891 | 0 to 1 |
| shots.save_low | probability | 0.272 | 0 to 1 |
| shots.quality.intercept | log-odds | -1.0 | -10 to 10 |
| shots.quality.distance_coef | log-odds per m | -0.1 | -2 to 0 |
| shots.quality.angle_coef | log-odds per rad | 1.0 | 0 to 10 |
| shots.save_hold | probability | 0.333 | 0 to 1 |
| shots.parry_speed | ratio | 0.8 | 0 to 1 |
| shots.parry_spread | rad | 1.2 | 0 to 3.2 |
| shots.parry_loft | m/s | 4.0 | 0 to 15 |
| shots.block_reach | m | 3.0 | 0 to 3 |
| shots.block_chance | probability per defender per shot | 0.5 | 0 to 1 |
| shots.block_speed | ratio | 0.8 | 0 to 1 |
| shots.block_spread | rad | 3.2 | 0 to 3.2 |
| shots.penalty_xg | expected goals | 0.76 | 0 to 1 |
| shots.penalty_spread | ratio | 0.25 | 0 to 2 |
| shots.keeper_dive_m | m | 2.0 | 0 to 3.66 |
| shots.keeper_stays | ratio | 0.1 | 0 to 1 |

While the other team has the ball, one back-line defender covers the ball carrier, or else the most advanced attacker, when that player is in the defending half and within `cover_channel` of the middle of the pitch: a carrier is tracked `cover_distance` goal-side of him, and any other attacker is covered from the defender's place in the line. No two neighbours in the back line stand more than `back_line_gap` apart, so the line narrows as players leave it.

The foul chance of one tackle starts at `foul_base` and moves in log-odds: up by `foul_aggression_weight` per curve point of the tackler's commitment (the tackle `choose` stage, led by aggression) and down by `foul_tackling_weight` per curve point of his tackle `execute` stage, each measured from rating 10, inside the tackle floor and ceiling. `foul_ball_loss` is the share of fouls after which the fouled team loses the ball, for a fouled player of rating 10; a contest of his `stay_up` stage (balance) moves it. After any other foul, the referee plays advantage outside the penalty area. The yellow-card chance of a foul is `yellow_base + yellow_aggression_weight × commitment`, with commitment the share of the tackle `choose` stage (0.5 at rating 10), and the red-card chance is `red_base`. The chance that a tackle wins the ball cleanly from a standing carrier is a contest of the tackler's tackle stage against the carrier's shield stage (strength), with base `tackle_win_base / 2`: two equal players win half of `tackle_win_base`, as the old ratio `tackling / (tackling + dribbling)` gave; a tackle that neither wins the ball nor fouls misses. A player already booked fouls less: the foul chance is multiplied by `foul_booked_factor`. A player who commits a foul, advantage or not, makes no tackle attempt for `foul_cooldown_ticks`. An opponent attempts a tackle on every tick he is within `tackle_reach` of the ball. Against a carrier running with the ball, faster than 2 m/s, the contest is against the mean of his dribble `execute` and `pressure` stages instead, and gains a second contest with base `tackle_dribble_win / 2`; a carrier who stands and shields the ball does not. A runner whose agility is under the take-on gate gives the tackler the gate's penalty. A presser runs at the point where he meets the carrier's run, and at the ball itself once he is within `press_engage` of it. A tuning file without these three fields loads with `tackle_reach` 1.0, `press_engage` 3.0 and `tackle_dribble_win` 0, which reproduce play before they existed. While play goes on with advantage, the referee holds at most one card per player, the more severe, and a player is shown at most one card when play stops. A restart is taken no earlier than its `restart_delay_s`, when the taker is within `restart_ready_radius` of the spot and every opponent stands back. At three times the delay, the restart is taken whatever the players are doing. While the restarting team leads, its delay is multiplied by its time-wasting level.

An injury is rolled for the tackled player on every tackle that wins the ball or is a foul (`injury_per_tackle`), and for every player on the pitch once per simulated minute (`injury_per_minute`). Both are the chances for a player of rating 10; the share of the `injury` stage (injury proneness) scales them by `0.5 + share`, so a share of 1 makes them half again as likely and 0 halves them. An injured player leaves play at once. In open play the referee stops play for a dropped ball at the ball (the goalkeeper's, inside its own penalty area), with every other player 4 m away, after `restart_delay_s.drop_ball`.

The expected goals (xG) of a shot is `1 / (1 + exp(-(xg.intercept + xg.distance_coef × d + xg.angle_coef × a)))`, where `d` is the distance from the ball to the goal centre in metres and `a` is the angle in radians that the goal mouth subtends from the ball. The match statistics sum it per team. A penalty counts `shots.penalty_xg`.

A shot leaves at `shot_speed` with a vertical speed drawn from 0 to `shots.loft_max`, aimed with a spread of up to `shot_noise × (1.5 − finishing)` radians either side, where `finishing` is the share of the shooter's shot `execute` stage (blended with its `pressure` stage when an opponent is close); a kick from the penalty mark multiplies the spread by `shots.penalty_spread`. As the shot is struck, the engine follows a copy of the ball with the match physics: the shot is on target when that flight crosses the goal line between the posts and under the bar, and the match statistics count it then. While a shot is in flight and faster than `control_speed`, each outfield defender within `shots.block_reach` of a ball under `reach_height` has one chance per shot, `shots.block_chance`, to block it. A blocked ball keeps `shots.block_speed` of its speed and goes back the way it came, turned by up to `shots.block_spread` either side. Only a shot on target can be saved: the acting keeper, within `keeper_reach` of a ball under the bar, has one save roll per shot. The save chance is `shots.save_high` for a shot of quality 0.05 or less, `shots.save_low` for quality 0.40 or more, and a straight line between, where the quality is the expected goals of the fixed `shots.quality` model (a penalty uses `shots.penalty_xg`), so refitting `xg` never moves the saves. That line is the base of a contest between the keeper's `save` stage (his `one_on_one` stage when the shooter is within 12 m of him) and the shooter's `shot` stage, or `long_shot` from outside the penalty area: a rating-10 keeper against a rating-10 shooter leaves it as it is. The keeper holds his saves with a contest of his `hold` stage (handling) against rating 10, based on `shots.save_hold`, and parries the rest: the ball keeps `shots.parry_speed` of its speed and goes along the goal line away from the goal centre, turned by up to `shots.parry_spread` either way, with a vertical speed of up to `shots.parry_loft`. After a block or a parry the defending side touched the ball last, so it gives a corner only if it then crosses the goal line. A shot off target is never saved. `keeper_catch_chance` applies only to a fast ball that is not a shot, such as a pass or a clearance, and that no defender cleared; the keeper's `claim` factor multiplies it, inside the claim floor and ceiling.

A carrier's clearance goes toward the far half, turned by up to `clearances.aim_spread` either way. While an open-play pass faster than `control_speed` and under `reach_height` is inside the penalty area of the side that did not play it, each outfield player of that side within `clearances.cross_reach` of the ball has one chance per flight, `clearances.cross_chance`, to clear it. The cleared ball keeps `clearances.cross_speed` of its speed and goes away from his goal centre, turned by up to `clearances.cross_spread` either way, with a vertical speed of up to `clearances.cross_loft`. His side touched the ball last, so it gives a corner only if it then crosses the goal line. At 0, `clearances.cross_chance` turns the cross clearance off. A cleared fast pass, or a carrier's clearance within `clearances.wide_depth` of his own goal line, goes wide with `clearances.wide_chance` instead: toward his own goal line on the ball's side, turned by up to 0.35 rad either way, so the ball may go behind for a corner. At 0, `clearances.wide_chance` turns the wide clearance off and takes no draw. A tuning file without the `clearances` block loads with the values in the table. In a shoot-out the keeper dives `shots.keeper_dive_m` along the goal line to one side, or stays in the middle for `shots.keeper_stays` of the kicks, and saves with the same model at the penalty quality; a held kick is a miss and a parried one plays on.

#### decision

The ball carrier scores every option and takes the highest: a pass to each team-mate with an open lane, a dribble, a shot, a clearance, or holding the ball. Each score is a weighted sum of features, plus the team's mentality, instructions, and each player's role and duty from `tactics.json`, plus noise. Every weight is -5 to 5 unless noted.

| Field | Default | Meaning |
|---|---|---|
| progress | 0.7 | a pass's forward progress, -1 to 1 over 40 m |
| lane | 0.5 | the pass lane's width, up to 6 m |
| space | 0.4 | free space around the receiver, up to 8 m |
| distance | 0.3 | a pass's length, up to 45 m, as a cost |
| min_lane | 1.5 | a pass lane narrower than this, in metres, is no option (0 to 10) |
| skill | 0.8 | what a skill 50 points above average adds: vision to forward passes, finishing to shots, dribbling to a dribble (0 to 5) |
| shot_base | 1.0 | a shot's base |
| shot_lane | 0.8 | the open shooting lane, 0 at 2.5 m and 1 at 5 m |
| shot_distance | 1.0 | the distance to goal over the shooting range, as a cost |
| dribble_base | -0.4 | a dribble's base |
| dribble_space | 0.55 | free space ahead, up to 10 m |
| pressure | -0.8 | the cost of an opponent within 2.5 m; negative makes the carrier take the defender on |
| first_touch | 0.6 | a dribble's bonus in the first 10 ticks after gaining the ball |
| clear | -1.0 | a clearance's base |
| clear_pressure | 1.2 | a clearance's bonus under pressure in the own third |
| keeper_clear | 0.2 | a goalkeeper's clearance base |
| hold | -0.4 | holding the ball's base |
| hold_per_s | 0.5 | the cost of each second already held (0 to 5) |
| noise | 0.1 | noise on every option, each side, for decisions and composure 50; it shrinks as they rise (0 to 2) |
| lone_layoff | 0.0 | for a lone carrier, the bonus on a pass to a team-mate at or behind him |
| lone_hold | 0.5 | for a lone carrier who is pressed, the bonus on holding the ball |
| lone_dribble | -0.5 | for a lone carrier who is pressed, the bonus on a dribble (a negative value is a cost) |
| carry_s | 0.0 | the carry window in seconds after the carrier gains the ball (0 to 5) |
| carry_cost | 0.0 | the cost on a pass and a clearance inside the carry window when no opponent is within 2.5 m; a shot is never charged (0 to 5) |
| teamwork_bonus | 0.1 | the weight of the carrier's pass `choose` stage (decisions, with teamwork and vision supporting) on a pass: `teamwork_bonus × (2 × share − 1)`, 0 at rating 10 (0 to 5) |

A carrier is lone when no active outfield team-mate stands nearer the opponents' goal than he does; a goalkeeper is never lone. A team's lone forward is its one active player, other than the one keeping goal, in the formation's front line: the outfield slots less than 4 m behind the most advanced one. While his team has the ball and someone else carries it, he moves from his formation place toward the onside line, 0.5 m short of the second-last opponent, by the share `lone_line_hold`, and never past that line. With these weights a pressed lone forward holds the ball or lays it off rather than dribbling into the defender, so a side with one forward does not outscore 4-4-2. A tuning file without these five fields loads with the values that reproduce play before they existed: `tackle_win_base` 0.05 and 0 for the other four. A tuning file without `carry_s` and `carry_cost` loads with 0 for both, which turns the carry window off.

#### contract

The attribute contract: how ratings become play.

| Field | Meaning | Default | Bound |
|---|---|---|---|
| curve.scale | the curve's value at its centre | 8.0 | 0.1 to 100 |
| curve.center | the rating at the centre | 10.0 | 1 to 20 |
| curve.width | the rating gap that multiplies the value by e | 8.0 | 1 to 100 |
| actions.<action>.k | log-odds per curve point, in a contest and in a share | see file | 0 to 5 |
| actions.<action>.base | a contest's chance between two equal players | see file | 0 to 1 |
| actions.<action>.floor, .ceiling | the lowest and highest chance a contest may give | see file | 0 to 1, floor below ceiling |
| actions.<action>.spread | how far a per-player factor moves an average player's value, either way | 0.2 | 0 to 1 |
| speed.kmh | `[pace, km/h]` points of real sprint speed, straight lines between | `[1, 29]`, `[10, 32]`, `[20, 35.5]` | 2 to 8 points; pace 1 to 20 rising, 10 to 50 km/h not falling |
| speed.amplification | how many times the real difference from a pace-10 player the engine speed differs | 2.0 | 0 to 10 |
| speed.anchor_ms | the engine top speed of a pace-10 player, m/s | 7.0 | 1 to 15 |
| accel.anchor | the acceleration of a player whose sprint stage is rating 10, m/s² | 5.5 | 0.5 to 30 |
| lapse.per_minute | the chance per minute that an average defender loses his place | 0.01 | 0 to 0.2 |
| lapse.late_from_minute | the minute from which that chance grows | 60 | 0 to 120 |
| lapse.late_growth | how much it grows per 30 minutes past that minute, as a share | 1.0 | 0 to 5 |
| lapse.ticks | how long a lapse lasts | 100 | 1 to 1000 |
| consistency.match_max | the largest match part of a player's form, for consistency 1.0, in rating points | 1.0 | 0 to 3 |
| consistency.period_max | the largest period part, for consistency 1.0, in rating points | 0.75 | 0 to 3 |
| consistency.period_minutes | minutes of the match between two draws of the period part | 10 | 5 to 45 |

The curve is `F(r) = scale × e^((r − center) / width)`: with the shipped values a rating of 10 gives 8.0, 18 gives 21.7, and 2 gives 2.9, so each step up the scale is worth more than the one below. A contest between two players is `σ(logit(base) + k × (F(a) − F(b)))`, inside the action's floor and ceiling, where `σ` is the logistic function. A share is `σ(k × (F − F(10)))`, 0.5 at rating 10. A per-player factor is `1 + spread × (2 × share − 1)`, 1 at rating 10; the factors scale the receive, intercept, press, shape, block, sprint, turn, aerial reach, claim, organise and rush reads.

Each action carries exactly the fields play reads of it. A missing field or one play does not read is refused by action, for example `content refused: tuning tuning.json: engine.contract.actions: action save: floor is missing`, and so is an action play reads nothing of.

Top speed reads pace directly, not through the curve: the real speed of the player's pace from `speed.kmh`, its difference from a pace-10 player times `speed.amplification`, in metres per second, added to `speed.anchor_ms`. Acceleration is `accel.anchor` times the sprint factor.

| states.caps.body, .mind, .familiarity, .surroundings | `[lowest, highest]` change of a rating a state family may make, in rating points | `[-2.0, 0.5]`, `[-1.0, 1.0]`, `[-1.25, 0.25]`, `[-0.75, 0.5]` | lowest −12.7 to 0, highest 0 to 12.7 |
| states.caps.total | `[lowest, highest]` change of a rating all families together may make | `[-5.0, 2.25]` | as above |
| states.family_groups.body, .mind, .familiarity, .surroundings | the attribute groups each family acts on | body: technical, physical, goalkeeping; mind: mental; familiarity: technical, mental; surroundings: technical, physical | each group at most once |
| body.height.reference_cm | the height at which height changes nothing, cm | 181 | |
| body.height.reach_per_cm | standing reach added per centimetre above the reference, m | 0.0133 | |
| body.height.turn_cost_per_cm | the share of turning lost per centimetre above the reference | 0.002 | |
| body.age.reference | the age from which age acts, years | 28 | |
| body.age.late_from_minute | the minute from which an older player's drain grows | 60 | |
| body.age.fade_per_year | the extra drain per year above the reference, reached 30 minutes after `late_from_minute` | 0.04 | |
| body.age.recovery_loss_per_year | the share of rest-day recovery lost per year above the reference | 0.03 | |
| body.age.injury_per_year | the extra congestion risk per year above the reference | 0.03 | |
| body.rest.post_match_energy | a player's energy just after a match | 0.4 | |
| body.rest.congestion_days | fewer rest days than this raise the injury chance | 4 | |
| body.rest.congestion_risk | the extra injury chance with no rest day, as a share | 1.0 | |
| body.sharpness.max_drop | the technical drop at sharpness 0, rating points | 1.5 | |
| body.adaptation.max_drop | the mental drop at adaptation 0, rating points | 1.0 | |
| body.build.frame_balance_weight, .slight_below, .powerful_from | the build word's frame and its thresholds | 0.5, 9.0, 14.0 | |
| body.jobs.height, .age, .nationality | each body field's job: the `statistic` it moves and the `direction` (`up` or `down`) | see file | |

A state changes a player's ratings, not his values directly: see [modifier slots](engine-modules.md#modifier-slots) for the order of the caps. While a state moves his ratings, his stage values, top speed, acceleration, turning, reach and knobs are blended again from his effective ratings.

A player's body acts in play from his team file. Height adds `reach_per_cm` metres of standing reach per centimetre above the reference to the reach his jump gives (2.0 m times his aerial reach factor), and takes `turn_cost_per_cm` of his turning per centimetre; a shorter player gains the same. After `late_from_minute`, an older player's drain grows over 30 minutes to `1 + fade_per_year` per year above the reference. His days of rest set his energy at kick-off, `post_match_energy` plus each day's `fatigue.recovery_per_day`, less `recovery_loss_per_year` of it per year above the reference, at most 1.0; fewer than `congestion_days` raise his injury chance by up to `congestion_risk`, more for an older player. A player with no height, no age or no rest days given plays as the reference: a converted version 1 player has no body. Nationality acts through the adaptation input. The build word reads `frame = strength − frame_balance_weight × (balance − 10)`: below `slight_below` it is slight, from `powerful_from` powerful, between them athletic. It is shown, never stored, and play does not read it.

While his side defends, each outfield player can lapse once per simulated minute, with the chance `lapse.per_minute` grown by `lapse.late_growth` for each 30 minutes past `lapse.late_from_minute`, and doubled for a shape `choose` share of 0 (concentration leading) or cut to nothing for a share of 1. For `lapse.ticks` he stops tracking his place.

Each player carries a form offset, in tenths of a rating point, that his consistency sets. It has two parts: a match part drawn at kick-off (or as he comes on) and a period part drawn again every `consistency.period_minutes`. Each part is `max × (20 − consistency) / 19` times a draw from −1 to 1 (the sum of two uniform draws less 1), so a player of consistency 20.0 never moves and one of 1.0 swings by the whole maximum. Each part is centred over his side's players on the pitch, so a side's mean form is 0: consistency spreads a player's matches and periods without making a side better or worse. The offset is added to every rating a stage table reads, inside the total state cap; pace and the hidden values take none. A substitute's match part is drawn alone as he comes on, and his period part is 0 until the next period. The draws use the `fatigue.form_match` and `fatigue.form_period` stream keys.

### generator

| Field | Meaning | Default | Bound |
|---|---|---|---|
| squad_size | players per generated club | 22 | 11 to 40; equals 11 plus the bench length |
| slot_positions | position of each formation slot, slot order | GK LB CB CB RB LW CM CM RW ST ST | 11 position codes |
| bench_positions | positions of the bench, in order | GK CB LB RB DM CM AM LW RW ST ST | `squad_size - 11` codes |
| world.tiers | from the top flight down, each `{ "mean", "club_spread" }`: the tier's mean level and the spread of its clubs' levels | 14.0, 11.2, 9.8, 8.6, 7.0, each with club spread 1.0 | 1 to 10 tiers; mean 1 to 20, club spread 0 to 6 |
| world.player_spread | the spread of a player's level around his club's | 2.35 | 0 to 6 |
| world.attribute_spread | the spread of an attribute around the player's level plus his position offset | 2.0 | 0 to 6 |
| world.offsets | one entry per position code: the offset of each attribute group from the player's level | see file | every one of the ten codes present; each offset -19 to 19 |
| world.hidden | `{ "mean", "spread" }`: the bell curve every hidden value is drawn from, whatever the player's level | 10.0, 2.5 | mean 1 to 20, spread 0 to 6 |

Each `world.offsets` entry holds `technical`, `mental`, `physical`, and `goalkeeping`. Every draw is a bell curve (mean plus spread times a unit normal) on the 1 to 20 scale: a club's level around its tier's mean, each player's level around his club's, and each attribute around the player's level plus his position's offset for the attribute's group, rounded to a tenth and kept to 1.0 to 20.0. A hidden value is drawn from `world.hidden` instead, in its place in the schema; one placed after the last visible attribute (consistency) comes from a stream of its own, drawn after the league, so adding it moved no other draw. `engine-cli generate` and the calibration leagues draw clubs of the top tier.

`body` is optional and draws the body fields of a version 2 team file. `engine-cli generate` needs it; without it the generated players have no body fields.

| Field | Meaning | Bound |
|---|---|---|
| body.height | one `{ "mean", "spread" }` in centimetres per position code; a draw is rounded and kept to 150 to 215 | every one of the ten codes; mean 150 to 215, spread 0 to 20 |
| body.age | `{ "mean", "spread", "min", "max" }` in years; a draw is rounded and kept to `min` to `max` | mean 15 to 45, spread 0 to 15, min 15 to 45, max min to 45 |
| body.nationality | `{ "home", "foreign", "foreign_share" }`: the club's country, the countries a foreign player comes from, and the share of foreign players | three upper-case letters each; 1 to 64 foreign codes; share 0 to 1 |

The body fields are drawn from their own random stream, after every attribute of the league, so adding or changing the block never moves an attribute. The shipped values are provisional.

### hidden

How a reader sees a hidden value: a word, from the bands of its rating, and how sure the club is of it, from the matches the player has played for the club (`condition.matches_at_club`).

| Field | Meaning | Default | Bound |
|---|---|---|---|
| confidence.tentative_from | matches at the club from which a word is shown, as tentative | 1 | 1 to 1000 |
| confidence.firm_from | matches at the club from which the word is firm | 20 | `tentative_from` to 1000 |
| words.<attribute> | for each hidden attribute, a list of `{ "from", "word" }` bands from the highest rating down; the last starts at 1.0 | see below | at least one band; `word` a lower-case key |

With no matches at the club, or none given, the value is not yet known and has no word. The shipped words, from the highest rating down:

| Attribute | From 15.0 | From 11.0 | From 7.0 | From 1.0 |
|---|---|---|---|---|
| consistency | `rarely_off` | `steady` | `has_off_days` | `erratic` |
| injury_proneness | `injury_prone` | `picks_up_knocks` | `rarely_injured` | `hardly_ever_injured` |

A schema whose hidden attribute has no word bands, or bands not falling from the highest or not ending at 1.0, is refused by name. The word is a key: the page owns its display text.

### match_rating

Every player who played gets a match rating at full time, from 1.0 to 10.0 with one decimal: `base` plus each count of what he did times its weight, plus the class weights below, kept to `min` to `max`.

| Field | Meaning | Default |
|---|---|---|
| base, min, max | the rating of a player who did nothing, and its bounds | 6.0, 1.0, 10.0 |
| goal, shot_on_target, shot_off_target, xg | per goal, per shot on or off target, per expected goal of his shots | 1.0, 0.15, −0.05, 0.5 |
| pass_completed, pass_failed | per open-play pass a team-mate controlled next, and per other pass | 0.02, −0.04 |
| tackle_won, interception, block, clearance, save | per ball won, pass intercepted, shot blocked, clearance, save | 0.1, 0.1, 0.15, 0.05, 0.3 |
| foul, yellow, red | per foul and card | −0.05, −0.5, −1.5 |
| conceded.keeper, conceded.defender | per goal his side conceded while he played, for a keeper and a listed defender | −0.3, −0.15 |
| clean_sheet.keeper, clean_sheet.defender, clean_sheet.min_minutes | a clean sheet for a keeper or a listed defender who played at least `min_minutes` | 0.5, 0.3, 60 |
| result.win, result.loss | his side's result, times the share of the match he played | 0.3, −0.3 |
| defenders | the positions counted as defenders | CB, LB, RB |

A shoot-out decides a knockout match but rates as a draw. The match record lists the ratings as `players.rating`; the live stream sends them in the `ratings` message.

### fatigue

| Field | Unit | Default | Bound |
|---|---|---|---|
| threshold | energy | 0.7 | 0 to 1 |
| curve | `[energy, multiplier]` points | `[[1,1],[0.7,1],[0.5,0.92],[0.3,0.82],[0,0.65]]` | 2 to 8 points, energy strictly falling from 1.0 to 0.0, multipliers 0.3 to 1 |
| drain_base_per_s | energy per s | 0.00008 | 0 to 0.01 |
| drain_effort_per_s | energy per s at full speed | 0.0004 | 0 to 0.05 |
| half_time_recovery | energy | 0.1 | 0 to 1 |
| recovery_per_day | points | 20.0 | 0 to 100 |
| group_weights.physical, .technical, .goalkeeping | the share of the fatigue delta each group takes | 1.0, 0.5, 0.5 | 0 to 1 |
| sprint_cap | the most the effort term counts a player's speed, as a share of a pace-10 player's top speed | 1.0 | 1 to 2 |

A player starts a match at energy 1.0, or at the energy his days of rest give (see [contract](#contract)). Each tick drains `drain_base_per_s` plus `drain_effort_per_s` times the square of the player's speed as a share of a pace-10 player's top speed (`contract.speed.anchor_ms`), counted at most as `sprint_cap`: every sprint costs stamina, and a slower runner at full sprint drains less than an average one. The highest stamina nearly halves the drain and the lowest makes it nearly half again as fast. At and above `threshold` a player plays at full strength. Below it, the curve's multiplier at his energy, read on the straight line between the two points around it, lowers his ratings by `curve.width × ln(multiplier)` rating points times each group's weight: physical first, technical and goalkeeping by half; his mental ratings are not touched. The body cap holds the change, with sharpness, to −2.0. The ratings are refreshed once a second. Half-time gives back `half_time_recovery`, scaled by natural fitness. `recovery_per_day` sets the energy at kick-off from the days of rest; carrying energy between matches is the season's.

### stream

| Field | Unit | Default | Bound |
|---|---|---|---|
| buffer_ticks | ticks | 500 | 10 to 5000 |
| keyframe_interval | ticks | 50 | 1 to 500 |

`buffer_ticks` is how far the engine may run ahead of a connected viewer. When the buffer
fills, the simulation thread pauses until the viewer drains it, so memory stays flat and no
tick is dropped. `keyframe_interval` is the number of ticks between keyframes on the live
stream: a keyframe carries every absolute position in 98 bytes, and each tick between
carries a 47-byte delta.

### flags

Feature flags switch between the current model and a candidate, so that a paired calibration run can compare the two. The block is optional; the shipped file declares no flag.

```json
"flags": {
  "short_shot_range": {
    "owner": "engine team",
    "hypothesis": "a shorter shot range lowers goals per match toward 2.8",
    "removal_condition": "removed after one paired run of 1000 matches per suite",
    "state": "off",
    "overrides": { "engine.shot_range": 12.0 }
  }
}
```

| Field | Type | Required | Meaning |
|---|---|---|---|
| (key) | text | yes | The flag name: lower snake case, at most 40 characters. |
| owner | text | yes | Who decides what happens to the flag. |
| hypothesis | text | yes | What the candidate is expected to change. |
| removal_condition | text | yes | When the flag is removed, whatever the result. |
| state | `on` or `off` | no, default `off` | The state when the command line sets none. |
| overrides | object | no | Tuning values to use while the flag is on, by dotted path, for example `engine.shot_range` or `fatigue.curve.1.1`. |

Rules:

- A flag without an owner, a hypothesis, or a removal condition is refused, for example `content refused: tuning tuning.json: flags.short_shot_range.owner: is required`.
- A flag must switch something: it has overrides, or engine code reads it. Otherwise it is refused with `flag <name> switches nothing: add overrides or register it in code`.
- An override path must name a value that exists, with a value of the same type. It cannot point at `schema_version` or `flags`.
- An override value must stay inside the bound of the field it sets. Each flag is checked when the file loads, whether it is on or off.
- Two flags that are on cannot set the same path.
- When a flag state differs from the file's, the list of flags that are on enters the content hash. A snapshot of a flagged match does not resume on content without the flag.

Every statistics record carries `tuning.flags_on`, the flags that were on. To compare a candidate and remove the loser, follow [the modding how-to](../how-to/modding.md#compare-two-models-with-a-flag).

## fast-model.json

The fit of the fast model, written by `engine-cli fast-model fit` (see [the command-line reference](cli.md#fast-model)). The engine never reads it to play a match, and the content hash does not include it, so a new fit changes no save, replay or gate hash. The release ships it with the rest of the content folder.

| Field | Holds |
|---|---|
| `schema_version` | 3. A file of format 2 has no `tilt`; it reads with a tilt of 0, which plays as it did. A file of format 1 holds the score fit only; the command refuses it and names `engine-cli fast-model fit` |
| `model` | The module the fit is for: `fitted-scores@1` |
| `engine_id` | The engine id of the results the fit came from: `golden-<ledger index>-<build>-<digest>` |
| `engine` | The id's parts: `id`, `ledger_index`, `build` and `digest` |
| `fitted_by` | The `engine_version` and `build` of the program that fitted it |
| `fit.params` | The nine parameters: `base`, `home`, `attack`, `curve`, `defence`, `dispersion` (the shape of the match factor both scores share), `rho`, `draw` and `tilt` (the favourite's tilt) |
| `fit.minute_shares` | 90 shares, one per minute of regulation time, summing to 1 |
| `fit.events.fouls`, `offsides`, `corners`, `throw_ins`, `goal_kicks`, `injuries` | One count each: the seven `coefficients` of the side's log mean over 1, the home flag, `o`, `t`, `o²`, `t²` and `o × t`, where `o` is the side's strength and `t` the other side's, each as (strength − 10) / 2, a strength being the level of the starting eleven read through the attribute contract (see [the command-line reference](cli.md#fast-model)); `dispersion` (the negative binomial's shape, or `null` for a Poisson count), and 92 `minute_shares` summing to 1, 45 per half and one for each half's added time |
| `fit.events.advantage`, `penalty`, `yellow`, `red`, `second_yellow`, `injury_stoppage` | The four `coefficients` of each share's log over 1, the home flag, `o` and `t` of the side: the shares of fouls played on with advantage; of fouls not played on that give a penalty; of fouls booked; of fouls sent off straight; of yellow cards, while a player of the side is booked, that go to a booked player; and of injuries that stop play |
| `fit.events.substitutions` | The count of a side's substitutions, those an injury forces among them: count `k`, from 0 to the rule pack's limit, has a share in proportion to `exp(weights[k] + k × (own × o + other × t))`; `weights[0]` is 0 |
| `fit.events.substitution_minute_shares` | 92 shares for the substitutions no injury forces, as for a count |
| `fit.events.added_goals` | Per half, the share of the goals in its last minute (44 or 89) that the full engine scored in added time |
| `fit.events.rules` | The parts the fit keeps to: `substitutions` and `added_time`, as in the rule pack, and `booked_foul_factor`, the tuning's `foul_booked_factor` |
| `batch` | The fit batch: `league_seed`, the strength `levels`, `matches_per_pairing`, `minutes` and the engine `seed` |
| `check` | The check it passed: the engine `seed`, the fast-model `draws` per match, `z` of the score figures, `event_z` of the event figures, `share_floor`, `mean_floor`, the number of `figures`, how many `failed`, and `pass` |

CI fails when `engine_id` is not the id of `gate/golden.json`: a Rust test runs in every test job, and `engine-cli fast-model stale` runs in the gate job and before every release.

The slot file's `engine.fast-model` entry picks the module: `{"module": "fitted-scores", "version": 1}`, or `{"module": "off"}`, which refuses to play. The slot never enters the content hash. Only the `fast-model` command reaches the module; a test fails the build when anything else does.

## sensitivity.json

The sensitivity rules, which prove that every job moves its own statistic (see [The player contract](../explanation/player-contract.md#every-rating-has-a-job)). Only the sensitivity run reads the file; no match reads it, and the content hash does not include it.

| Field | Holds |
|---|---|
| `schema_version` | 1 |
| `design` | The size of a run: `matches` of the balanced design (an even number, 2 to 100,000), `arm_matches` of each arm (every job low, every job high), the `seed`, and the bootstrap `resamples` per interval (99 to 99,999) |
| `defaults` | The thresholds of every rule: `min_move`, the least relative move of the statistic from the low to the high level in the job's direction (0 to 10), and `ceiling`, the largest share of the outcome move of all jobs together one job may carry (0 to 1) |
| `rules` | One rule per job: `job` (an attribute or `height`, `age`, `nationality`), `kind` (`attribute` or `body`), `measure` (the counter the run reads), `statistic` (the job's statistic, word for word), and `levels`, the low and the high level. A rule may override `min_move` or `ceiling`, with a `reason` |
| `derived` | Fields with no job in play, proven by their own derivation test: `build` |
| `five_match` | The five-match rule: the `roles` (position codes), `runs` per role, `matches_per_run`, the `top` and `average` rating of every visible attribute of the two copies, and the `target` share of runs the top copy wins with its `tolerance` |

An attribute's levels are ratings, 1.0 to 20.0 on the tenth grid. A body job's levels are centimetres for `height` (150 to 215), years for `age` (15 to 45), and percent of adaptation for `nationality` (0 to 100), since nationality acts through adaptation. The direction of each statistic comes from the job, in `attributes.json` or in `engine.contract.body.jobs` of `tuning.json`; the rules file does not repeat it.

The measures are `pass_completion`, `take_on_completion`, `receipt_loss`, `box_finishing`, `cross_completion`, `headers_won`, `long_shots_on_target`, `tackles_won`, `skill_attempts`, `blocks`, `pass_progress`, `xg_per_shot`, `pressed_completion`, `loose_ball_share`, `tackle_attempts`, `fouls`, `lapses`, `anchor_distance`, `top_speed`, `close_loose_balls`, `full_time_energy`, `standing_tackles_lost`, `fouled_kept`, `aerial_balls_reached`, `half_time_gain`, `injuries`, `saves_held`, `far_saves`, `high_claims`, `near_saves`, `keeper_kicks`, `keeper_throws`, `crosses_claimed`, `offsides_won`, `sweeps`, `rating_spread`, `late_energy_loss` and `mental_rating`. Two jobs may share a measure: dribbling and agility both read `take_on_completion`, and jumping and height both read `aerial_balls_reached`.

The file is refused, naming the field, for:

- a job with no rule: `content refused: sensitivity sensitivity.json: rules: job vision has no rule`
- a rule with no job, or a second rule for a job
- a `statistic` that differs from the job's own words
- a `measure` that is not in the list above
- a `min_move` or `ceiling` override without a `reason`
- a level outside its range or off the tenth grid, or a low level not below the high one
- a `derived` field that has a job

Each rule reads one of three words:

- **pass**: the lower end of the move's 95 percent interval reaches `min_move`, and the upper end of the share's interval stays within `ceiling`.
- **fail**: the upper end of the move's interval is below `min_move` (too small, or the wrong way), or the lower end of the share's interval is above `ceiling`.
- **not sure**: anything else. Not sure is never a pass.

The five-match rule passes when the 95 percent interval of the pooled share of runs lies within `target` plus or minus `tolerance`, fails when it lies wholly outside, and is not sure otherwise.

## Position codes

`GK`, `CB`, `LB`, `RB`, `DM`, `CM`, `AM`, `LW`, `RW`, `ST`.

## rules/default.json

| Field | Meaning | Default | Bound |
|---|---|---|---|
| halves | halves per match | 2 | 1 to 2 |
| half_minutes | minutes per half | 45 | 1 to 60 |
| substitutions.limit | players a team may replace | 5 | 0 to 11 |
| substitutions.windows | stoppages a team may use for substitutions | 3 | 0 to 5 |
| substitutions.windows_exempt | stoppage kinds at which a substitution uses no window | `["half_time"]` | each kind at most once |
| stoppages | one entry per kind | see file | every kind exactly once |
| added_time.per_kind | seconds each stoppage of a kind adds | see file | 0 to 600; every kind present |
| added_time.card_s | seconds each card adds | 15 | 0 to 120 |
| added_time.video_review_s | seconds each video review adds; optional, and the shipped file leaves it out. No match event counts a review yet, so the price adds no time | 60 | 0 to 600 |
| added_time.variance_s | the most seconds the seeded variance adds or removes | 30 | 0 to 300 |
| added_time.min_s | the least added time of a half | 60 | 0 to 900 |
| added_time.max_s | the most added time of a half | 900 | `min_s` to 1800 |
| min_players | the fewest players a team may have on the pitch; fewer ends the match | 7 | 1 to 11 |
| extra_time.periods | periods of extra time a level knockout match plays; 0 goes straight to the shoot-out | 2 | 0 to 2 |
| extra_time.period_minutes | minutes in each period of extra time | 15 | 1 to 30 |
| extra_time.added_max_s | the most added time of an extra-time period | 300 | 0 to 900 |
| extra_time.extra_substitutions | substitutions each team gains once extra time starts | 1 | 0 to 3 |
| extra_time.extra_windows | substitution windows each team gains once extra time starts | 1 | 0 to 3 |
| shootout.kicks | kicks each team takes before sudden death | 5 | 1 to 10 |
| shootout.allowance_rounds | rounds of kicks the announced match length allows for; a longer sudden death still plays to its end | 10 | 1 to 30 |

Each stoppage is `{ "kind", "admits_tactics", "admits_substitution" }`. Kinds: `kick_off`, `throw_in`, `corner`, `goal_kick`, `free_kick`, `penalty`, `goal`, `half_time`, `injury`. The engine announces every stoppage by its kind. A queued tactical change or substitution waits for the next stoppage whose kind admits it and applies on the tick that stoppage opens, substitutions first. A substitution beyond `substitutions.limit`, or at a new stoppage once `substitutions.windows` are used, is rejected with the reason. All substitutions at one stoppage share one window, and a kind listed in `windows_exempt` uses none.

The added time of a half is the sum of `per_kind` over the half's stoppages, plus `card_s` for each card, plus a seeded variance from `-variance_s` to `+variance_s`. The sum is rounded to the second and clamped to `min_s` to `max_s`. A match shorter than `halves × half_minutes` plays no added time.

`extra_time` and `shootout` apply only to a knockout match (`--knockout` on `simulate`, `bench`, `serve`, and `record`). A knockout match that is level after regulation time plays `extra_time.periods` periods of `period_minutes`, with the teams changing ends at each break and no energy given back. Each period's added time is priced like a half's and capped at `added_max_s`. Once extra time starts, each team may make `extra_substitutions` more substitutions in `extra_windows` more windows; the breaks are `half_time` stoppages, so the `windows_exempt` rule covers them. Still level, the match goes to a penalty shoot-out: `kicks` kicks each, then sudden death. A knockout match shorter than `halves × half_minutes` plays no extra time and goes straight to the shoot-out when level. The announced maximum length adds both extra-time periods at their cap and `allowance_rounds` rounds of kicks.

## teams/*.json

Version 2 holds every rating in tenths of the 1 to 20 scale, written as a number with one decimal, and three body fields per player. Every player holds both hidden values, `consistency` and `injury_proneness`; a file still naming `injury_resistance` is refused by player and attribute:

```json
{
  "schema_version": 2,
  "club": { "id": "club-00000001-00", "name": "Oakmere Rangers", "short_name": "OAK",
            "kit": { "primary": "#c8102e", "secondary": "#000000" },
            "ground": { "length": 100, "width": 64 } },
  "players": [
    { "id": "p-club-00000001-00-01", "name": "Peton Tavwood", "shirt": 1, "position": "GK",
      "attributes": { "acceleration": 11.2, "...": 1.0 },
      "height": 189, "age": 27, "nationality": "ENG" }
  ]
}
```

| Field | Bound |
|---|---|
| club.id | 3 to 64 characters |
| club.name | 2 to 48 characters |
| club.short_name | 2 to 4 characters |
| club.kit.primary, secondary | `#RRGGBB` |
| club.ground.length | the home ground's touchline in metres: 90 to 120, and longer than the width; default 105 |
| club.ground.width | the home ground's goal line in metres: 45 to 90; default 68 |
| players | 11 to 40 entries; ids and shirts unique |
| players[].id | 1 to 64 characters |
| players[].name | 2 to 48 characters |
| players[].shirt | 1 to 99 |
| players[].position | one of the ten codes |
| players[].attributes | every schema attribute present, no other key, each 1.0 to 20.0 on the tenth grid (one decimal) |
| players[].height | whole centimetres, 150 to 215; required |
| players[].age | whole years, 15 to 45; required |
| players[].nationality | three upper-case letters, for example `ENG`; required |

The screens show a rating as its whole number, 1 to 20: 12.5 shows as 13.

Before kick-off the AI manager picks the best-fitting player for each slot of the formation the side starts in, in slot order, by the slot role's attribute weights in `tactics.json`, and names a bench of up to `ai.bench_size` from the rest, the best remaining goalkeeper first. File order breaks ties. A bad value is refused by player and field:

- a rating out of range: `content refused: team teams/x.json: players: player p-club-00000001-00-04: attribute pace is 20.5; allowed 1.0 to 20.0`
- a rating off the tenth grid: `content refused: team teams/x.json: players: player p-club-00000001-00-04: attribute pace is 12.34; not on a tenth`
- a missing body field: `content refused: team teams/x.json: players: player p-club-00000001-00-04: body field height is missing`

### Match condition

A player may carry an optional `condition` block: his condition for this match. Every field is optional; an absent field has no effect, and a file without the block hashes as it did before the block existed. The season carries these between matches later; until then the team file gives them.

```json
"condition": { "sharpness": 60, "adaptation": 40, "rest_days": 3, "matches_at_club": 12 }
```

| Field | Bound | Absent reads | Job |
|---|---|---|---|
| condition.sharpness | 0 to 100 percent | 100 | Below 100, his technical ratings drop, by 1.5 points at 0 (the `engine.modifier.sharpness` slot) |
| condition.adaptation | 0 to 100 percent | 100 | Below 100, his mental ratings drop, by 1.0 point at 0 (the `engine.modifier.adaptation` slot); a player new to a country is given a low value |
| condition.rest_days | 0 to 14 days | rested | His energy at kick-off, and a higher injury chance with fewer than 4 days |
| condition.matches_at_club | 0 to 1000 | none | Matches he has played for the club. Play does not read it; it sets how sure a reader is of his hidden values (see [hidden](#hidden)) |

An unknown field is refused, and a value out of range is refused by player and field: `content refused: team teams/x.json: players: player p-club-00000001-00-04: condition field sharpness is 101; allowed 0 to 100`.

### Version 1 files

A version 1 team file holds every attribute as a whole number 1 to 100 and no body fields. It still loads, from the content folder and from the inputs a replay file embeds: each value `v` becomes `2v` tenths, so 62 becomes 12.4, and its players have no body fields. Values 1 to 4 become 1.0, the lowest rating; no player holds a rating under 1.0. Injury resistance `R` becomes injury proneness `21.0 − R` (exact on the tenths grid), and every player gets consistency 10.0. A version 1 file is checked as before: `content refused: team teams/x.json: players: player p-club-00000001-00-03: attribute pace is 120; allowed 1 to 100`. The shipped team files are version 1.

A match is played on the home team's ground. `club.ground` is optional: a file without it plays on 105 by 68 metres, and the default is never written back, so a file that gives 105 by 68 and one that gives no ground hash the same. The touchlines, the goal lines, the halfway line and every spot measured from them follow the ground; the goal, the goal and penalty areas, the penalty mark, the centre circle, the corner arcs and the 9.15 m kick distance keep their sizes from the Laws. The formation slots in `tactics.json` are drawn for 105 by 68 and scale with the ground: along the touchline by its length over 105, across by its width over 68. A ground outside the Laws is refused by club: `content refused: team teams/x.json: club.ground: Oakmere Rangers: the ground is 121 m long; the Laws allow 90 to 120 m`. A touchline that is not longer than the goal line is refused the same way.

Every other club file in `teams/` plays in the background round of a served match's matchday: `serve` pairs every club except the two of the player's match into fixtures, with a round seed derived from the match seed, and plays each on the full engine beside the match. The shipped folder holds ten clubs, so a matchday has four other fixtures. A file that does not load is left out of the round, with a `matchday.team_refused` warning that names it. No club file other than the two a match plays enters that match's content hash, save or replay.

## tactics.json

| Field | Holds | Bound |
|---|---|---|
| formations | `{ "name", "slots" }`; eleven slots `{ "x", "y", "position" }` in metres from the own goal line (1 to 100) and from the centre line (-33 to 33) on a 105 by 68 ground; the slots scale with the home ground | 1 to 16; slot 0 is the only `GK` |
| mentalities | `{ "name", "block_depth", "shoot", "progress", "hold" }`: metres the block moves up (-20 to 20), and offsets on shots, forward passes, and holding the ball (-2 to 2) | 1 to 9 |
| instructions | the six team instructions, each `{ "default", "levels" }` with 2 to 5 levels | see below |
| roles | `{ "name", "positions", "attributes", "in_possession", "out_of_possession", "preferred_actions", "teammates" }`: the positions it suits, attribute weights (0 to 10, each a name in `attributes.json`) the AI manager uses to pick players, and the role's three behaviours (see below) | 1 to 64; a role for every position a formation uses |
| duties | `{ "name", "depth", "risk", "scale" }`: metres the anchor moves up (-15 to 15), an offset on forward passes and dribbles (-2 to 2), and a scale on the role's preferred actions (0.5 to 1.5) | 1 to 5 |
| ai | the AI manager's formation, mentality, and duty by name, and its thresholds | see below |

A role's three behaviours:

| Field | Holds | Bound |
|---|---|---|
| in_possession, out_of_possession | `{ "x", "y" }`: metres the player's anchor moves while the team has the ball, and while it does not; `x` up the pitch, `y` away from the centre line on the slot's side | each -15 to 15 |
| preferred_actions | `{ "shoot", "dribble", "progress" }`: offsets on the carrier's shot, dribble, and forward-pass scores, each times the duty's `scale` | each -2 to 2 |
| teammates | `{ "long_ball_target" }`: added to a team-mate's pass to this player in proportion to its length, so a target forward draws long balls | 0 to 2 |

The shipped roles set every offset and `long_ball_target` to 0 and every duty's `scale` to 1, which plays as version 1 did. A version 1 tactics file, whose roles hold `shoot`, `dribble`, and `progress` directly and whose duties have no scale, converts on load to exactly those values.

The instructions are `pressing` (levels `{ "name", "press_count", "press_distance_scale" }`: how many players press the carrier, 0 to 4, and a scale on `press_distance`, 0.1 to 3), `width` (a scale on each slot's distance from the centre line), `tempo` (added to passes and taken from dribbles and holding), `line_height` (metres the block moves up), `passing_directness` (a bonus for longer passes), and `time_wasting` (a factor on the restart delay while the team leads). Every level other than a pressing level is `{ "name", "value" }`, with the value -20 to 20.

| ai field | Meaning | Default |
|---|---|---|
| formation, mentality, duty | the setup before kick-off | `4-4-2`, `balanced`, `support` |
| trailing_minute | from this minute a trailing team raises its mentality one step and presses high, once per score | 70 |
| leading_minute | from this minute a leading team lowers its mentality one step and wastes time, once per score | 80 |
| check_interval_s | simulated seconds between checks; an injury triggers one at once | 30 |
| fatigue_energy, fatigue_from_minute | from this minute a player below this energy is replaced | 0.55, 55 |
| keep_for_injury_until_minute | until this minute one substitution is kept back for an injury | 80 |
| bench_size | substitutes named before kick-off, one a goalkeeper when the squad has one | 7 |

A bad value is refused by field, and a role attribute the attribute schema does not hold is refused by name.

## commentary/en.json

Every event except a `tactics-change` verdict carries one English line in its `commentary`
field. The lines live here, so you can rewrite or add lines without rebuilding. The file is
not part of the content hash: editing a line never stops a saved match from resuming.

```json
{
  "schema_version": 1,
  "language": "en",
  "templates": [
    { "event": "corner", "lines": ["Corner to {team}. {player} goes over to take it."] },
    { "event": "corner", "when": { "repeat": "again" },
      "lines": ["Another corner for {team}! {player} takes it again."] }
  ]
}
```

Each entry is a template set: an `event` kind, an optional `when` with conditions, and its
`lines`. `event` is spelled as the event contract spells it: `kick-off`, `goal`, `half-time`,
`full-time`, `offside`, `foul`, `card`, `throw-in`, `corner`, `goal-kick`, `free-kick`,
`penalty`, `injury`, `substitution`, or `ai-decision`.

| Condition | Values | Holds when |
|---|---|---|
| `minute` | `early` | before one sixth of the match (minute 15 of 90) |
| | `late` | from eight ninths of the match (minute 80 of 90), or in added time of the last half |
| | `added-time`, `first-half`, `second-half` | the event falls there |
| `score` | `level`, `leading`, `trailing` | the score from the event club's side; at half-time and full time, from the home club's side |
| | `rout` | that side leads by 3 or more |
| | `opener`, `equaliser`, `go-ahead`, `extends-lead`, `consolation` | on a goal only: the first goal, a goal that levels, a goal that turns level into a lead, a goal for a club already ahead, a goal for a club still behind |
| `form` | `hot`, `cold` | the club scored (`hot`) or conceded (`cold`) 2 or more goals in the last sixth of the match |
| | `booked` | the player the event names already holds a yellow card |
| `repeat` | `first`, `again`, `streak` | no, at least one, or at least two events of the same kind in the last ten minutes of play |
| `card` | `yellow`, `second-yellow`, `red` | on a card: the card shown |
| `advantage` | `true`, `false` | on a foul: play continued with advantage |
| `own_goal` | `true`, `false` | on a goal: the player who kicked the ball last plays for the other club |
| `decision` | `mentality-up-trailing`, `mentality-down-leading`, `sub-injury`, `sub-fatigue`, `sub-keeper` | on `ai-decision`: the AI manager's choice |

| Placeholder | Value | Filled on |
|---|---|---|
| `{player}` | the player's name | every kind except `half-time`, `full-time`, and `ai-decision`: the offender, the booked or injured player, the player leaving on a substitution, the taker on a restart or kick-off, the scorer on a goal |
| `{other_player}` | the second player's name | `foul` (the fouled player) and `substitution` (the player coming on) |
| `{team}`, `{opponent}` | the event's club and the other club | every kind except `half-time` and `full-time` |
| `{home}`, `{away}` | the two club names | every kind |
| `{home_score}`, `{away_score}`, `{score}` | the score after the event, `{score}` as `2-1` | every kind |
| `{minute}` | the clock minute, `90` in added time | every kind |
| `{added_minutes}` | the added minute, `2` at 90+2 | events in added time only; use it in a set with `"minute": "added-time"` |

The loader refuses a file in which:

- an event kind has fewer than 3 lines in sets with no `when` that use only the
  placeholders every event of that kind fills;
- a line uses an unknown placeholder, a placeholder its event kind cannot fill, or a stray
  `{` or `}`;
- a set names an unknown event kind or holds no lines.

The feed shows its own minute stamp, so the shipped lines leave `{minute}` out.

**How a line is chosen.** Every set of the event's kind whose conditions all hold is a
candidate. The most specific set (the most conditions) is tried first; sets with the same
number of conditions are tried in file order. Inside a set, the search starts at a point
that depends on the seed and on how many events of that kind came before, and takes the
first line not used for that kind in the last ten minutes of play. When every candidate line
was used in that window, the least recently used line is taken. The same seed gives the same
lines.

## Generating clubs

```bash
engine-cli generate --seed 7 --clubs 20 --out my-league
```

Writes one version 2 file per club, named by club id. The same seed and the same content give the same files. Pass `--force` to overwrite. The tuning file must hold the `generator.body` block. Every generated value is drawn on 1.0 to 20.0, so the command lifts nothing; it still prints the count it would lift: `lifted 0 values below 1.0 to 1.0`.

## Runtime files

The engine writes these files to the data folder. The data folder is `SM_DATA_DIR`. When `SM_DATA_DIR` is not set, the data folder is `%LOCALAPPDATA%\SoccerManager` on Windows and `$HOME/.local/share/SoccerManager` on other systems.

| File | Written by | Holds |
|---|---|---|
| `owner.id` | every command, once | 32 hexadecimal characters that name the owner of every match on this machine |
| `engine.port` | `serve`, while it serves a match | the socket port, as one line |
| `settings.json` | `launch`, when the player saves the settings | the player's three settings; see below |
| `views.json` | `launch`, when the player saves a squad view, and when the Squad screen first reads a club after a match | each club's named squad views and its players' last ten match ratings; see below |
| `matches/<match.id>/events.jsonl` | `simulate`, `serve`, `resume` | one match event per line, as the `match-event` record |
| `matches/<match.id>/stats.json` | `simulate`, `serve`, `resume` | the statistics of the match, as the `match-stats` record |
| `matches/<match.id>/snapshot.smsn` | `simulate`, `serve` | the newest snapshot of the match; `resume` and `serve --resume` read it |
| `matches/<match.id>/matchday-bug-<fixture>.json` | `serve` | the bug report of a background match that failed; see below |
| `runs/<run.id>/report.json` | `calibrate` | the run report, as the `run-report` record |
| `runs/<run.id>/stats/<match.id>.json` | `calibrate` | the statistics record of each match in the run |
| `runs/<run.id>/events/<match.id>.jsonl` | `calibrate` | the event rows of each match the run keeps |

A `match.id` is `<seed as 16 hexadecimal characters>-<start time in milliseconds>`.

### The records

The records are JSON objects with flat, dotted keys. Every record carries `record.kind`, `schema.version`, `service`, `operation`, `env`, `version`, `build.hash`, `owner.id`, and, for a match, `match.id`. The JSON Schema (draft 2020-12) of each record kind is in `schemas/observability/`:

| `record.kind` | Schema file | Written |
|---|---|---|
| `match-event` | `match-event.schema.json` | one line in `events.jsonl` for each event, at each stoppage and at full time |
| `match-stats` | `match-stats.schema.json` | `stats.json`, at each stoppage and at full time |
| `run-report` | `run-report.schema.json` | `report.json` at the end of a calibration run, and the one line `bench` prints |

The engine writes a record at each stoppage snapshot and at the end of the match. A match that stops early keeps every record up to the newest stoppage.

### The snapshot file

`snapshot.smsn` holds the full state of a match at one stoppage: the clock, the score, every player's position, energy, and cards, the queue of changes, the state of the random-number generator, and for a knockout match the extra-time periods and the shoot-out. The engine replaces the file at each stoppage. A snapshot resumes only on the build that wrote it, with the same content files and team files. The engine refuses any other snapshot and names the reason.

The file is format 9. Its header (at least 192 bytes; its length is stored at bytes 6 to 8) names the match without reading the rest, at fixed places, so a later build can say who saved a file it cannot read:

| Bytes | Field |
|---|---|
| 0 to 4 | `SMSN` |
| 4 to 6 | the format, 9 |
| 6 to 8 | the header length: 192 plus the matchday mark |
| 64 to 96 | the release version that saved it, such as `0.2.0`, padded with zeros |
| 96 to 100 | the tick of the stoppage |
| 100 to 102 | the score, home then away |
| 104 to 148 | the home team's name, padded with zeros |
| 148 to 192 | the away team's name, padded with zeros |

The build's commit, the content files' hash, the owner and the match stamp sit in the first 64 bytes, as in formats 6 and 7. Files of format 6 and 7 carry no version; the engine names their release from the build's commit when a release shipped that build.

Format 9 appends the matchday mark after byte 192: the mark's length after its first two bytes (u16), the round seed (u64), the reveal tick (u32: the tick the snapshot was taken on, up to which the other grounds' events were revealed), the fixture count (one byte), and for each fixture, home first, each club's id (one length byte, then at most 64 bytes) and the SHA-256 of its team file (32 bytes). A count of 0 is a match with no round. A match resumed from the snapshot rebuilds the same round from the mark and plays each other match again from kick-off; a club whose file is missing or changed since the save makes its fixture unavailable. Format 8 is the same file without the mark, and the engine still reads it.

A snapshot from the previous release finishes on that release's engine: `resume` and `launch --resume` run the program in `previous/` beside this one (the release ships it, see `packaging/README.md`). A snapshot from any other version is refused, and the refusal names the version that saved it and the versions this release finishes.

### The saved match

The start screen's Resume continues the newest match folder that holds a `snapshot.smsn` and no `stats.json`: the engine writes `stats.json` only at full time, so a match without it was stopped before the end. Return to start and Quit both keep the snapshot. A save of the previous release resumes on the program in `previous/`; a save of any other version shows why it cannot resume.

### The settings file

`settings.json` holds the settings the start screen's Settings page saves. The launcher keeps them in the data folder rather than in the browser, because the page's port, and with it the browser's storage, changes at every launch.

| Field | Values | Default |
|---|---|---|
| `schema_version` | 1 | 1 |
| `speed` | the playback speed a match starts at: 1, 2, 4 or 8 | 1 |
| `motion` | `follow` (as the operating system asks), `reduce` or `full` | `follow` |
| `commentary` | whether the match screen shows the commentary column | `true` |

A missing file gives the defaults. A file that does not read, has another field, or holds a value outside the table gives the defaults and logs the `launch.settings_refused` signal; the launcher refuses the same values from the page with the reason. The launcher writes the file through a temporary file, so a crash never leaves half of one.

### The views file

`views.json` holds the named views of the Squad screen, by club id, and the match ratings its Rating column averages. Like the settings, it lives in the data folder so the views outlast the page's port. Under `serve --web` or `replay --web` nothing is written; the page keeps its views for the session.

| Field | Values |
|---|---|
| `schema_version` | 1 |
| `clubs` | an object from club id to that club's part |

Each club's part:

| Field | Values |
|---|---|
| `active` | the name of the view the screen opens on, or null for the first |
| `views` | at most 20 views, each `name` (1 to 40 characters), `columns` (1 to 64 column ids, in order; the No and Player columns are always shown and never listed) and `sort` (`{"column", "direction"}` with direction `up` or `down`, or null) |
| `ratings` | from player id to his newest 10 match ratings, oldest first, each `{"match", "rating"}` with the rating from 1.0 to 10.0 |

The launcher adds the ratings of the home side of the last match it ran, read from that match's `stats.json`, when the page next reads the club, once per match. Saving views keeps the club's ratings. A column id the page does not know is dropped when the page reads it, so a view from a later release still opens. A missing file is empty. A file that does not read, has another field, or breaks a limit is read as empty and logs the `launch.views_refused` signal; the launcher refuses the same values from the page with the reason. The launcher writes the file through a temporary file.

### The matchday bug report

A background match should never fail. When a defect makes one fail anyway, the player's match plays on, the fixture shows "result unavailable", and `serve` writes `matches/<match.id>/matchday-bug-<fixture>.json`, one JSON object, so the match can be played again:

| Field | Holds |
|---|---|
| `schema.version` | 1 |
| `match.id` | the player's match |
| `fixture` | the fixture's place in the round, from 0 |
| `seed` | the background match's seed |
| `engine.version`, `build.hash` | the engine that played it |
| `home.team.id`, `away.team.id` | the two clubs |
| `tick` | the tick the match reached |
| `error.type` | `panic`, `did-not-finish` (still running 120 seconds after the player's full time), or `team-file` (a club file missing or changed since a save) |
| `error.message` | what stopped it, with every absolute path cut to its file name |

### The tick file

`simulate --ticks-out <FILE>` writes one record per tick: the ball position and height, then the position of each of the 22 players. The header states the seed, the tick length, and the most ticks the match can last. The trailer states the ticks written. When a knockout match's sudden death runs past the announced length, the engine raises the header's figure to the ticks written, so the file still reads. `--json` also writes the same ticks as JSON Lines beside the tick file.

### The replay file

A replay file (`.smfx`) holds every frame of one match, starting with the `hello`, in the same bytes the socket sends. `record --out <FILE>` writes one. The viewer page saves one at full time as `touchline-<match.id>.smfx`. `replay --fixture <FILE>` plays one over the socket, and the page opens one with no engine running. The layout of both formats (version 3, frames only, and version 4, which also holds the inputs and the record that `resimulate` reads) is in [the protocol reference](protocol.md#replay-files).

## notices.json

`notices.json` sits in the folder the page is served from (`viewer/dist/`, and `web/` in a release). The Licences and about screen reads it. The viewer build writes it and fails when a shipped package has no licence text or a licence outside the allow list in `deny.toml` (and OFL-1.1, for fonts only).

| Field | Holds |
|---|---|
| `version` | the release version the build belongs to |
| `packages` | one entry per shipped package: `kind` (`crate`, `npm` or `font`), `name`, `version` (null for a font), `licence` (an SPDX expression), `description` (the package's own description) and `text` (its licence text) |

The crates are the normal dependencies of `engine-cli` for the build's platform, without the workspace's own crates, together with those of the previous engine the release ships in `previous/`. The npm packages are those bundled into the page. A crate that ships no licence file needs a reviewed entry in `packaging/notices/clarify.json`, which names its licence, the standard text its notice takes (with the crate's authors) and why. `npm --prefix viewer run notices:verify -- <file>` checks a built file against `cargo metadata` and the page's imports.
