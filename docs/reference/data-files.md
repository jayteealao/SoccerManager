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
| `attributes.json` | 4 | The attribute schema: 30 to 50 names in four groups, each with its job in play (two of them hidden values), the stage tables that blend them for every action, and the skill gates |
| `tuning.json` | 6 | Engine constants, the attribute contract with the state caps, the body jobs and consistency, decision weights, injury rates, the generator's world spread, fatigue curve, stream buffer, the hidden values' words and the match rating |
| `rules/default.json` | 4 | The rule pack |
| `tactics.json` | 5 | Ten formations, mentalities, team instructions, roles, duties, the AI manager's settings, and the tactical primitives: what each setting of the tactics board moves, where play reads it, and how its levels were fitted to real teams |
| `signatures.json` | 3 | The real-team signature table: for each statistic a tactical primitive moves, its spread across real club seasons, the five quintiles and each quintile's median. No match reads it; the primitives harness fits the levels to it. Written by `engine-cli signatures`, never by hand |
| `traits.json` | 1 | The trait catalogue: the 31 traits of play a player may carry, each with the engine choice it acts on and, for nine pairs, its opposite. Part of the content hash; a team file naming a trait it does not hold, or two opposites on one player, is refused |
| `restart-waits.json` | 1 | The restart wait table: for each kind of restart, the drawn extra wait beyond the visible steps, as 21 quantiles from real restart waits, the limit on the visible steps, the late-lead factor and the quick-restart rule. Optional: content without it plays the fixed `restart_delay_s` delays. Part of the content hash when present; written by `engine-cli restart-waits`, never by hand |
| `styles.json` | 1 | The eight named styles: each with a mentality, instruction levels, duty hints, the formations that suit it, the statistics that show it, and the attributes its key jobs read. Optional: content without it keeps the AI manager on the tactics file's default setup and cannot play the styles suite. Part of the content hash when present |
| `teams/default-a.json`, `teams/default-b.json` | 1 | The two default clubs (`engine-cli generate --seed 1` and `--seed 2`) |
| `commentary/en.json` | 1 | The English commentary lines, grouped by event kind and match situation |
| `realism-bands.json` | 5 | The band registry: every realism band the calibration run judges, with its measure, design range, smallest shift, and its record (where its real value is measured, the evidence label, the confidence counts, the condition, and whether it gates), and the figures it reports beside an earlier build's value. Versions 4, 3 and 2 still load. They are acceptance criteria, never tuning values |
| `fast-model.json` | 3 | The fast model fitted from full-engine results, with the engine id of the results it came from. The content hash does not read it |
| `sensitivity.json` | 1 | The sensitivity rules: one rule per attribute job and per body job, the thresholds they are judged by, the size of a run, and the five-match rule. Only the sensitivity run and the rules stage of a change run read it; the content hash does not |
| `gate-proof.json` | 1 | The gate proof: the broken copies of the engine that `engine-cli calibrate proof` plays, and the pass and fail rates of the last proof with the gate it measured. No match reads it; neither the content hash nor the old engine's cache key includes it |

Every file starts with `"schema_version"`. A file with another version is refused, except `realism-bands.json`, which also reads versions 4, 3 and 2: `content refused: rules rules/default.json: schema_version 7; this build reads 4`. Team files are read in version 3 and in versions 2 and 1, and `tactics.json` in version 3 and in versions 2 and 1; an older version converts on load (see [teams/*.json](#teamsjson) and [tactics.json](#tacticsjson)). `attributes.json` is read in version 5 and in versions 4, 3, 2 and 1, and `tuning.json` in version 6 and in versions 5, 4, 3 and 2; an older version converts on load with the contract tables of the build that introduced it, copies compiled into it that tuning the shipped files never changes, so a replay that embeds an old file plays the same way however the shipped tables move. A version 2 tuning file converts through version 3; a version 3 file gains the state caps, the body jobs, `fatigue.group_weights` and `fatigue.sprint_cap` from the second copy; a version 4 file gains `engine.contract.consistency`, `hidden`, `match_rating` and `generator.world.hidden` from the third copy; a version 5 file's `engine.decision.pressure` becomes both `shot_under_pressure` and `takeon_under_pressure`, with its value. A version 1 to 4 attribute file converts to version 5 the same way (see [Version 4 files](#version-4-files) and [Version 2 files](#version-2-files)). Any other version is refused the same way.

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

Every attribute has a `job`: `{ "action", "stage", "statistic", "direction" }`. `action` is one of the actions below; `stage` is `see`, `choose`, `execute`, `pressure`, `top_speed` (pace only, with the action `sprint`), `spread` (a hidden value only, with no `action`), or `occasion` (a personality value, or a visible attribute that acts on an occasion no stage of its own reads, with no `action`: determination, which widens the press while his side trails); a job without an action at any other stage, and an occasion job that names an action, is refused, naming the attribute; `statistic` names the match statistic the attribute moves, 3 to 120 characters; `direction` is `up` or `down`, the way the statistic moves as the rating rises. The stage tables must put the attribute in the stage its job names, or the file is refused, naming the attribute.

### Stage tables

Each action plays in up to four stages: seeing the option (`see`), choosing it (`choose`), carrying it out (`execute`), and carrying it out with an opponent close (`pressure`). `actions` maps each action to its stages, and each stage to a table `{ "main", "supports", "source" }`: `main` and each of up to three `supports` are `{ "attribute", "weight" }`, weights above 0 and up to 10, every support lighter than the main attribute. `source` is `design`, or `fitted:<reference>` for weights fitted from real data.

A player's value in a stage is the weighted mean of his attributes' values on the curve (see [contract](#contract)), so a player rated 10 throughout has the value 8.0 in every stage. Play reads the stage value in a contest against an opponent, or as a share from 0 to 1 that is 0.5 at rating 10.

The actions are `pass`, `chip` (a pass longer than 25 m), `cross` (a pass from wider than 20 m into the penalty area), `shot`, `long_shot` (from outside the penalty area), `header`, `penalty`, `dribble`, `receive`, `tackle`, `shield`, `intercept`, `press`, `shape`, `block`, `sprint`, `endure`, `turn`, `stay_up`, `aerial_reach`, `recover`, `injury`, `hold`, `save`, `claim`, `one_on_one` (a shot from within 12 m of the keeper), `keeper_kick`, `keeper_throw`, `organise`, `rush`, `cover` (covering a team-mate and stepping with the line, `execute` only), and `recovery_run` (who takes a recovery run first and how likely it is a sprint, `execute` only). Which stages each action has is fixed by the build: a stage play reads needs a table, and a table for any other stage is refused. Every attribute other than pace is in at least one table.

A table is refused by stage, for example:

- a missing table: `content refused: attributes attributes.json: actions: pass.see has no table`
- a stage play does not read: `content refused: attributes attributes.json: actions: sprint.see is not a stage play reads; remove it`
- a name that is not in the file: `content refused: attributes attributes.json: actions: tackle.choose: attribute aggression is not in the file`
- a support as heavy as the main attribute: `content refused: attributes attributes.json: actions: pass.execute: support technique weighs 3, not below the main weight 3`

### Gates

`gates` holds `chip` and `take_on` (a dribble with an opponent close), each `{ "technique_try", "agility_pull_off", "penalty_k" }`. A player tries the skill only from the technique rating `technique_try` (1 to 20); below the agility rating `agility_pull_off` (1 to 20) his execution loses `penalty_k` log-odds (0 to 5).

### Version 4 files

A version 4 attribute file has no `cover` or `recovery_run` stage. It still loads, and takes the version 5 changes from the build's fifth copy of the contract tables: the press stage led by positioning (anticipation and decisions shown, vision a small weight), the shape stage led by teamwork (concentration and consistency shown), the two new stages, work rate's job moved to the recovery run, determination's job made an occasion, and a consistency degree for each new action. A version 1 to 3 file converts through version 4.

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
| far_delay_s_per_m | s per m | 0.01 | 0 to 0.5 |
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
| pace.enabled | flag | true | true or false |
| pace.limits_ms | m/s, five values | 0.2, 2.0, 4.0, 5.5, 7.0 | positive, strictly rising |
| pace.stand_m | m | 1.0 | 0 to 10 |
| pace.return_far_m | m | 5.0 | 0 to 60 |
| pace.run_from_m | m | 0.0 | 0 to 120 |
| pace.calm_ball_m | m | 30.0 | 0 to 120 |
| pace.sprint_mid | probability | 0.5 | 0 to 1 |
| pace.sprint_slope | probability per share | 1.0 | 0 to 2; the chance stays inside 0 to 1 |
| pace.tired_from | energy | 0.5 | 0 to 1 |
| pace.step_down_at | ratio | 0.25 | 0 to 1 |
| pace.skip_at.carry | ratio | 0.1 | 0 to 1 |
| pace.skip_at.timed_run | ratio | 0.1 | 0 to 1 |
| pace.skip_at.press | ratio | 0.2 | 0 to 1 |
| pace.skip_at.loose_ball | ratio | 0.35 | 0 to 1 |
| pace.skip_at.recovery | ratio | 0.35 | 0 to 1 |
| pace.late_minutes | minutes | 15 | 0 to 60 |

Every target a player moves to has a job, and each job an urgency grade. URGENT: the carrier's move, a press, a recovery run while his team has fewer outfield players goal-side of the ball than its mentality minimum (or the rest-defence guard holds him back farther than `pace.return_far_m` from his spot), a loose ball he reaches first, and a timed run. NORMAL: the other team's chase of a loose ball, a stand option that offers a pass while his team has the ball, a return to a place farther than `pace.return_far_m`, and following the ball in shape within `pace.calm_ball_m` of it. CALM: holding his place farther from the ball, a concentration lapse, and a dead ball with no job. A counter (the `on_winning_the_ball` primitive at its two counter levels, for 13 seconds after the team wins the ball in open play, while it keeps it), or trailing in the last `pace.late_minutes` minutes, lifts pressing jobs (press, chase) and the offer one grade; no tempo or pressing level lifts them; leading in those minutes with time wasting on drops each NORMAL job one grade while the ball is dead; in open play both teams keep their grades. An URGENT job never drops.

The grade picks one of six levels, each run at its upper limit from `pace.limits_ms`: within `pace.stand_m` of the target a player stands; CALM walks; NORMAL jogs, or runs farther than `pace.run_from_m` (0 in the shipped file: a NORMAL job runs, and jogs only when tiredness steps it down); URGENT runs at high speed, or sprints at the player's own top speed when the job's sprint draw succeeded. No level is above his top speed. The draw is taken once when an URGENT job starts, with the chance `pace.sprint_mid + pace.sprint_slope × (work-rate share − 0.5)`: work rate sets how often an urgent job gets a sprint, not the speed of any level. Below `pace.tired_from` energy a player saves `(tired_from − energy) / tired_from`; with his tired mix, the mean share of his determination and professionalism, a saving times the mix at or above `pace.step_down_at` drops every level below a sprint one step, never below a walk while he is farther than `pace.stand_m` from his target, and a saving times one less the mix at or above the job's `pace.skip_at` value turns that job's sprint into high speed, the least urgent job's first. At a dead ball every player runs at his top speed until the restart. A tuning file without the `pace` block loads with `pace.enabled` false: every player runs at his top speed, as before the selector.

Every outfield anchor moves with the ball by `compactness_x` and `compactness_y` of its position, in both phases. The ball point each player's anchor follows is his own: in open play it eases toward the ball with a time constant of `far_delay_s_per_m` seconds per metre of his distance from the ball, so a player near the ball follows it at once and a far one late; every dead ball and kick-off sets it to the ball. 0 turns the delay off, and a tuning file without the field loads with 0. The delay is the only value tuned against the team slide band, and no field sets a gap between the lines or a share of the slide: the gaps and the block's length come from where each role stands and the slide. A player farther than `anchor_ball_distance` from the ball and farther than `anchor_tolerance` from both his places (his anchors in and out of possession) heads for the nearer place, whatever stand option or held spot he had.

While the other team has the ball, one back-line defender covers the ball carrier, or else the most advanced attacker, when that player is in the defending half and within `cover_channel` of the middle of the pitch: a carrier is tracked `cover_distance` goal-side of him, and any other attacker is covered from the defender's place in the line. No two neighbours in the back line stand more than `back_line_gap` apart, so the line narrows as players leave it.

The foul chance of one tackle starts at `foul_base` and moves in log-odds: up by `foul_aggression_weight` per curve point of the tackler's commitment (the tackle `choose` stage, led by aggression) and down by `foul_tackling_weight` per curve point of his tackle `execute` stage, each measured from rating 10, inside the tackle floor and ceiling. `foul_ball_loss` is the share of fouls after which the fouled team loses the ball, for a fouled player of rating 10; a contest of his `stay_up` stage (balance) moves it. After any other foul, the referee plays advantage outside the penalty area. The yellow-card chance of a foul is `yellow_base + yellow_aggression_weight × commitment`, with commitment the share of the tackle `choose` stage (0.5 at rating 10), and the red-card chance is `red_base`. The chance that a tackle wins the ball cleanly from a standing carrier is a contest of the tackler's tackle stage against the carrier's shield stage (strength), with base `tackle_win_base / 2`: two equal players win half of `tackle_win_base`, as the old ratio `tackling / (tackling + dribbling)` gave; a tackle that neither wins the ball nor fouls misses. A player already booked fouls less: the foul chance is multiplied by `foul_booked_factor`. A player who commits a foul, advantage or not, makes no tackle attempt for `foul_cooldown_ticks`. An opponent attempts a tackle on every tick he is within `tackle_reach` of the ball. Against a carrier running with the ball, faster than 2 m/s, the contest is against the mean of his dribble `execute` and `pressure` stages instead, and gains a second contest with base `tackle_dribble_win / 2`; a carrier who stands and shields the ball does not. A runner whose agility is under the take-on gate gives the tackler the gate's penalty. A presser runs at the point where he meets the carrier's run, and at the ball itself once he is within `press_engage` of it. A tuning file without these three fields loads with `tackle_reach` 1.0, `press_engage` 3.0 and `tackle_dribble_win` 0, which reproduce play before they existed. While play goes on with advantage, the referee holds at most one card per player, the more severe, and a player is shown at most one card when play stops. With a restart wait table ([restart-waits.json](#restart-waitsjson)) a restart waits for its visible steps and a drawn extra: the restart is taken when the taker is within `restart_ready_radius` of the spot and every player the law moves has moved, plus the drawn extra. Without one, a restart is taken no earlier than its `restart_delay_s`, when the taker is within `restart_ready_radius` of the spot and every opponent stands back; at three times the delay, the restart is taken whatever the players are doing; and while the restarting team leads, its delay is multiplied by its time-wasting level. A shoot-out kick always waits `restart_delay_s.penalty`.

An injury is rolled for the tackled player on every tackle that wins the ball or is a foul (`injury_per_tackle`), and for every player on the pitch once per simulated minute (`injury_per_minute`). Both are the chances for a player of rating 10; the share of the `injury` stage (injury proneness) scales them by `0.5 + share`, so a share of 1 makes them half again as likely and 0 halves them. An injured player leaves play at once. In open play the referee stops play for a dropped ball at the ball (the goalkeeper's, inside its own penalty area), with every other player 4 m away, after `restart_delay_s.drop_ball`.

The expected goals (xG) of a shot is `1 / (1 + exp(-(xg.intercept + xg.distance_coef × d + xg.angle_coef × a)))`, where `d` is the distance from the ball to the goal centre in metres and `a` is the angle in radians that the goal mouth subtends from the ball. The match statistics sum it per team. A penalty counts `shots.penalty_xg`.

A shot leaves at `shot_speed` with a vertical speed drawn from 0 to `shots.loft_max`, aimed with a spread of up to `shot_noise × (1.5 − finishing)` radians either side, where `finishing` is the share of the shooter's shot `execute` stage (blended with its `pressure` stage when an opponent is close); a kick from the penalty mark multiplies the spread by `shots.penalty_spread`. As the shot is struck, the engine follows a copy of the ball with the match physics: the shot is on target when that flight crosses the goal line between the posts and under the bar, and the match statistics count it then. While a shot is in flight and faster than `control_speed`, each outfield defender within `shots.block_reach` of a ball under `reach_height` has one chance per shot, `shots.block_chance`, to block it. A blocked ball keeps `shots.block_speed` of its speed and goes back the way it came, turned by up to `shots.block_spread` either side. Only a shot on target can be saved: the acting keeper, within `keeper_reach` of a ball under the bar, has one save roll per shot. The save chance is `shots.save_high` for a shot of quality 0.05 or less, `shots.save_low` for quality 0.40 or more, and a straight line between, where the quality is the expected goals of the fixed `shots.quality` model (a penalty uses `shots.penalty_xg`), so refitting `xg` never moves the saves. That line is the base of a contest between the keeper's `save` stage (his `one_on_one` stage when the shooter is within 12 m of him) and the shooter's `shot` stage, or `long_shot` from outside the penalty area: a rating-10 keeper against a rating-10 shooter leaves it as it is. The keeper holds his saves with a contest of his `hold` stage (handling) against rating 10, based on `shots.save_hold`, and parries the rest: the ball keeps `shots.parry_speed` of its speed and goes along the goal line away from the goal centre, turned by up to `shots.parry_spread` either way, with a vertical speed of up to `shots.parry_loft`. After a block or a parry the defending side touched the ball last, so it gives a corner only if it then crosses the goal line. A shot off target is never saved. `keeper_catch_chance` applies only to a fast ball that is not a shot, such as a pass or a clearance, and that no defender cleared; the keeper's `claim` factor multiplies it, inside the claim floor and ceiling.

A carrier's clearance goes toward the far half, turned by up to `clearances.aim_spread` either way. While an open-play pass faster than `control_speed` and under `reach_height` is inside the penalty area of the side that did not play it, each outfield player of that side within `clearances.cross_reach` of the ball has one chance per flight, `clearances.cross_chance`, to clear it. The cleared ball keeps `clearances.cross_speed` of its speed and goes away from his goal centre, turned by up to `clearances.cross_spread` either way, with a vertical speed of up to `clearances.cross_loft`. His side touched the ball last, so it gives a corner only if it then crosses the goal line. At 0, `clearances.cross_chance` turns the cross clearance off. A cleared fast pass, or a carrier's clearance within `clearances.wide_depth` of his own goal line, goes wide with `clearances.wide_chance` instead: toward his own goal line on the ball's side, turned by up to 0.35 rad either way, so the ball may go behind for a corner. At 0, `clearances.wide_chance` turns the wide clearance off and takes no draw. A tuning file without the `clearances` block loads with the values in the table. In a shoot-out the keeper dives `shots.keeper_dive_m` along the goal line to one side, or stays in the middle for `shots.keeper_stays` of the kicks, and saves with the same model at the penalty quality; a held kick is a miss and a parried one plays on.

#### decision

The ball carrier scores every option and takes the highest: a pass to each team-mate with an open lane, a dribble, a shot, a clearance, or holding the ball. Each score is a weighted sum of features, plus the team's instructions and each player's role and duty from `tactics.json`, plus noise. Every weight is -5 to 5 unless noted.

The mentality adds no flat bonus to any option. It is the team's appetite for risk: a pass, a dribble or a hold loses the mentality's `loss_weight` times the option's failure chance times the danger of losing the ball where it would be lost. The danger is 1 at either goal line and 0.5 at halfway: a ball lost near the own goal hands the opponent a chance, and one lost near the other goal gives up the side's own. A pass fails as its lane closes and its receiver's space shrinks, and is lost about halfway; a dribble fails under pressure and with little space ahead, a hold under pressure, each at the carrier. A shot has no loss term: a missed shot gives the ball back at the opponent's own goal line, and the shot spends the side's threat rather than losing it. Tempo is the circulation's patience: when the carrier picks his pass, a short pass (20 m or less) that gains less than 5 m toward goal gains the tempo level's value and a forward pass loses it. The value moves which pass he picks, never whether he passes, holds or dribbles.

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
| shot_under_pressure | -0.8 | added to a shot with an open lane (wider than 1 m) when an opponent is within 2 m |
| takeon_under_pressure | -0.8 | the cost on a dribble of an opponent within 2.5 m; negative makes the carrier take the defender on |
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
| hooks.decel_anchor | the braking of a player whose turn stage is rating 10, m/s²; his turn factor (agility leading) scales it | 5.5 | 0.5 to 30 |
| hooks.behind_effort | how much a presser's determination widens his press reach while his side trails | 0.3 | 0 to 1 |
| hooks.flair_risk | how much a carrier's flair raises the score of a forward pass | 0.4 | 0 to 2 |
| hooks.flair_gate | how many rating points of technique the take-on and chip gates drop for a top flair player | 2.0 | 0 to 5 |
| hooks.free_kick_range_m | a direct free kick within this many metres of goal is struck at goal; beyond it, delivered toward the penalty mark | 30 | 0 to 40 |
| hooks.restart_noise | the largest turn of a corner or free-kick delivery by an average taker, radians | 0.06 | 0 to 0.5 |
| hooks.organise_back_line | the back line's own share of the line's organisation beside the keeper's | 0.5 | 0 to 1 |
| hooks.anchor_space_m | how far an attacker with top off-ball movement moves his place away from the nearest defender, m | 2.0 | 0 to 6 |
| hooks.rash_chase | how much a keeper's eccentricity widens the range from which he comes off his line | 0.3 | 0 to 1 |
| hooks.press_goal_side_m | how far a top presser leans his aim goal-side of the carrier, m; his press choose skill (−1 to 1) scales it, and a file without it reads 0 | 2.0 | 0 to 6 |
| person.mood.goal, .assist, .big_miss, .error, .card, .red_card, .fouled, .duel_won, .duel_lost | the mood impulse of the player's own event, percent of a full mood | 30, 15, −20, −4, −10, −25, 3, 3, −3 | −100 to 100 |
| person.mood.team_goal_for, .team_goal_against | the mood impulse of every player of the side that scored, and of the side that conceded | 8, −10 | −100 to 100 |
| person.mood.big_chance_xg | a shot of at least these expected goals that ends without a goal is a big miss | 0.3 | 0.05 to 1 |
| person.mood.drift_per_minute | how far a mood moves toward its target in a minute, percent | 4 | 0 to 50 |
| person.mood.score_weight | the drift target per goal of lead, at most two goals either way, percent | 15 | 0 to 50 |
| person.mood.late_from_minute, .late_pressure | from this minute, in a game within one goal, the drift target drops by `late_pressure · (1 − pressure handling) / 2` | 75, 10 | 0 to 120, 0 to 50 |
| person.mood.temperament_weight, .pressure_weight | how much temperament and pressure handling narrow the swing of every impulse | 0.5, 0.4 | 0 to 0.9 |
| person.mood.leadership_weight | how much leadership adds to a player's weight in his side's mood | 1.0 | 0 to 3 |
| person.mood.own_weight, .team_weight | rating points on the mental group at a full own mood and at a full team mood | 0.8, 0.4 | 0 to 3 |
| person.mood.risk | how much a full mood raises a shot's and a forward pass's score and lowers holding the ball; a low mood the reverse | 0.25 | 0 to 2 |
| person.familiarity.cost_per_point | rating points lost per point of gap between the natural rating and the rating at the position played | 0.15 | 0 to 1 |
| person.familiarity.versatility_weight, .adaptability_weight | how much versatility and adaptability narrow that cost | 0.5, 0.3 | 0 to 0.9 |
| person.occasions.professionalism | how much professionalism lowers the chance of a concentration lapse | 0.5 | 0 to 0.9 |
| person.occasions.penalty_pressure | how much pressure handling narrows a penalty's spread, a shoot-out kick's included | 0.3 | 0 to 0.9 |
| person.occasions.temper_foul | log-odds a tackler at a full low mood adds to his foul chance, for an average temperament | 0.6 | 0 to 3 |
| person.occasions.cynical_foul, .dive_foul | log-odds an unsporting tackler adds against a running carrier, and an unsporting carrier adds by going down, per unit of sportsmanship skill below the average | 0.4, 0.3 | 0 to 3 |
| person.occasions.adaptation | how much adaptability narrows the adaptation drop | 0.3 | 0 to 0.9 |
| person.floor_drop | the shared floor: no stage reads more than this many rating points below the player's base | 7.0 | 0.5 to 12.7 |
| traits.moderate_share | the share of a trait's weight a moderate trait plays at; a strong one plays all of it | 0.5 | 0 to 0.9 |
| traits.low_mood | a mood at or below this, percent of a full mood, is a low mood for an override | −25 | −100 to 0 |
| traits.carry_s | seconds an override that carries or holds the ball lasts | 1.5 | 0.2 to 5 |
| traits.weights.<trait> | how far each of the 31 traits moves its choice: a score added to or taken from an option, a share of a carry's side, a shot's aim or speed, or a press reach, metres an anchor or a run moves, log-odds on a tackle or a card; `tries_tricks_gate` is the rating points of technique the take-on gate drops | see file | 0 to 0.9, 1, 3, 5 or 15 by weight |
| traits.override_chance.<trait> | the chance per moment that a strong trait overrides the instruction it goes against, one per trait that can conflict; a trait outside the conflict table reads none and ships at 0 | see file | 0 to 1 |
| foot.cost_cap | the most rating points a kick by the weakest foot loses | 5.0 | 0 to 5 |
| foot.base | the share of the cost every weaker-side kick without time pays | 0.4 | 0 to 1 |
| foot.pressure_weight, .range_weight | the share the closest pressure adds, and the share the longest range adds | 0.3, 0.3 | 0 to 1 |
| foot.pressure_m | the nearest opponent adds no pressure from this many metres, all of it at 0 m | 5.0 | 0.5 to 20 |
| foot.range_m | a kick this many metres long or longer adds all of the range share | 40.0 | 5 to 80 |
| foot.either_deg | within this many degrees of straight ahead either foot kicks, so the strong one does | 20.0 | 0 to 90 |
| foot.shift_s | seconds an unpressed carrier needs on the ball to move it onto his strong foot | 0.6 | 0 to 5 |
| foot.show_m | metres a closing presser stands toward the strong side of the most one-footed carrier | 1.5 | 0 to 5 |
| season.form_neutral | the match rating that counts as neither good nor bad form | 6.5 | 1 to 10 |
| season.form_weight | mood points per rating point of recent form above or below `form_neutral` | 20 | 0 to 50 |
| season.goal_weight | mood points a full pace ahead of (or behind) the season goal gives (or takes) at average ambition | 15 | 0 to 50 |
| season.ambition_weight | the share of `goal_weight` the most ambitious player adds and the least ambitious loses | 0.6 | 0 to 0.9 |
| season.cap | the most mood points the season gives or takes at kick-off | 40 | 0 to 100 |

The curve is `F(r) = scale × e^((r − center) / width)`: with the shipped values a rating of 10 gives 8.0, 18 gives 21.7, and 2 gives 2.9, so each step up the scale is worth more than the one below. A contest between two players is `σ(logit(base) + k × (F(a) − F(b)))`, inside the action's floor and ceiling, where `σ` is the logistic function. A share is `σ(k × (F − F(10)))`, 0.5 at rating 10. A per-player factor is `1 + spread × (2 × share − 1)`, 1 at rating 10; the factors scale the receive, intercept, press, shape, block, sprint, turn, aerial reach, claim, organise and rush reads.

Each action carries exactly the fields play reads of it. A missing field or one play does not read is refused by action, for example `content refused: tuning tuning.json: engine.contract.actions: action save: floor is missing`, and so is an action play reads nothing of. The five actions of the newer attributes, `mark`, `off_ball_run` and `long_throw` (`k` and `spread`) and `corner` and `free_kick` (`k`), and the two defensive stages `cover` and `recovery_run` (`k` and `spread`; shipped 0.4 and 0.2), may be left out: an older tuning file without them, as a replay embeds it, reads each as neutral (share 0.5, factor 1).

The hooks are how the newer attributes reach play. Marking divides `cover_distance` by the covering player's `mark` factor, so a better marker tracks a carrier tighter. An attacker with off-ball movement above the average whose target lies beyond the onside line comes back toward it by his off-ball skill (0 to 1), and moves his place `anchor_space_m` times that skill away from the nearest defender. A corner's or free kick's delivery turns from its aim by up to `restart_noise` times `1.5 − share` of its stage. A throw-in looks for team-mates up to 15 m times the thrower's `long_throw` factor. Flair adds `flair_risk` times his flair skill (−1 to 1) times the pass's forward share to a pass's score, and lowers the technique the take-on and chip gates need by `flair_gate` times that skill. A presser's reach is the team's pressing distance times his press-effort factor, read from his work rate alone; while his side trails, it grows by `behind_effort` times his determination skill. Beyond `press_engage`, his aim leans toward his own goal by `press_goal_side_m` times his press choose skill. The cover defender's lead on a carrier's run is multiplied by his `cover` factor, and out of possession a back-line player whose `cover` factor is under 1 moves his target depth toward his own by the shortfall. Recovery runs go first to the candidates with the least distance ahead of the ball over their `recovery_run` factor, and a recovery run's sprint chance reads that stage's share instead of the work-rate share. A keeper's chase range grows by `rash_chase` times his eccentricity skill. The back line's gap divides by the keeper's organise factor and the back line's mean, weighted by `organise_back_line`. Braking is `decel_anchor` times the turn factor. A tuning file without the `hooks` block reads every hook as before them: braking with the acceleration, no direct free kick taken (the taker decides), no delivery noise, the keeper alone organising the line, and no other term.

The person block is how mood, familiarity and the personality values reach play. A player's mood runs from −100 to 100 percent of a full mood and starts each match at his kick-off mood (see the season block below), 0 without season inputs. His own events and his side's goals move it at once by their impulse times his swing, `(1 − temperament_weight · temperament)(1 − pressure_weight · pressure handling)`, each personality value read as a skill from −1 to 1 (0 at rating 10). Every second it drifts toward `score_weight` per goal of his side's lead. Each side keeps a bench mood that takes the side's events and the drift, and a substitute comes on at his kick-off mood plus the bench mood times his swing. Mood moves the mental group through the pressure and momentum modifier slots, inside the mind cap, and moves the shot, forward-pass and hold scores by `risk` times mood over 100. Professionalism scales the lapse chance by `1 − occasions.professionalism · professionalism`; pressure handling scales a penalty's spread; temperament and sportsmanship add their log-odds to a tackle's foul chance. Familiarity reads the gap between his natural rating and his rating at his formation slot's position (see [engine modules](engine-modules.md#modifier-slots)); the pre-match level range includes it for a player in the lineup. The shared floor keeps every stage value at or above the player's base value less `floor_drop` points on the curve, after every state and his form. A tuning file without the `person` block plays as before it: mood stays 0, familiarity costs nothing, no occasion acts and no floor binds. No screen, report or event shows a mood or a personality value as a number.

The traits block is how a player's traits of play reach play. Each trait he carries moves one engine choice by its weight, all of it when strong and `moderate_share` of it when moderate; the reader table in `crates/engine/tests/readers.rs` names the function and the quantity for each. A strong trait can also override an instruction his team holds for him that it goes against. Ten traits have such an instruction: dwelling on the ball and holding it up against a faster tempo, releasing it early against a slower one, cutting inside against wide width, hugging the touchline against narrow width, pressing relentlessly against low pressing, getting forward and arriving late in the box against a defend duty, staying back against an attack duty, and rushing out against a deep line. At a moment of play where the trait's choice differs from the one play took, with an opponent within 2.5 m of him or his mood at or below `low_mood`, one draw below the trait's `override_chance` makes the trait win and writes a `trait-override` event. No count, cooldown or phase of play limits it; an override that carries, holds, presses, anchors or chases lasts as long as that action does. The shipped chances give a player with such a trait 2 to 4 overrides a match. The other traits' override moments are too rare for that rate, so they act through their weight only. A tuning file without the `traits` block plays as before it: no trait moves a choice and none overrides.

The foot block is how a player's weaker foot reaches play. A kick goes to the side of its direction against the kicker's facing, since the ball he carries sits straight ahead of him; within `either_deg` of straight ahead his strong foot plays it. A pass, chip, cross, open shot or long shot to the side of his weaker foot costs `cost_cap × (1 − weaker / 100) × min(1, base + pressure_weight × pressure + range_weight × range)` rating points, where pressure rises from 0 with the nearest opponent at `pressure_m` to 1 at 0 m and range from 0 to 1 at `range_m`. The cost is 0 on his strong side and with time to shift the ball: when no opponent is within 2.5 m and he has held the ball `shift_s` seconds, and always at a penalty and a direct free kick, which are placed dead balls. The cost lowers the kick's execute stage and, when he is pressed, its pressure stage; with the `person` block present it shrinks on each stage to the points left above the shared floor, so an 18 never kicks below 11. A keeper's distribution, a clearance and a restart delivery pay no cost. A presser closing a carrier from outside `press_engage` runs for his meeting point moved `show_m × (1 − weaker / 100)` metres toward the carrier's strong side, showing a one-footed carrier onto his weaker foot; within `press_engage` he goes for the ball. A tuning file without the `foot` block plays as before it: no kick pays and no presser shows.

The season block is how the launcher's season record reaches play. A player whose condition carries his recent form (`recent_rating`) and his progress toward his season goal (`goal_progress`) starts the match at `form_weight × (recent_rating / 10 − form_neutral) + goal_weight × (1 + ambition_weight × ambition) × clamp((goal_progress − 100) / 100, −1, 1)`, kept within `cap` either way, where ambition is read as a skill from −1 to 1 (0 at rating 10). A missing input adds nothing, so a match without season inputs, as every `simulate`, `calibrate` and test match is, starts every mood at 0, and ambition changes nothing in it. From kick-off the drift and the events move the mood as above. A tuning file without the `season` block reads every kick-off mood as 0.

Top speed reads pace directly, not through the curve: the real speed of the player's pace from `speed.kmh`, its difference from a pace-10 player times `speed.amplification`, in metres per second, added to `speed.anchor_ms`. Acceleration is `accel.anchor` times the sprint factor. Braking is `hooks.decel_anchor` times the turn factor: a change of velocity against his run is clamped by his braking, any other by his acceleration.

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

Each player carries a form offset, in tenths of a rating point, that his consistency sets. It has two parts: a match part drawn at kick-off (or as he comes on) and a period part drawn again every `consistency.period_minutes`. Each part is `max × (20 − consistency) / 19` times a draw from −1 to 1 (the sum of two uniform draws less 1), so a player of consistency 20.0 never moves and one of 1.0 swings by the whole maximum. Each part is centred over his side's players on the pitch, so a side's mean form is 0: consistency spreads a player's matches and periods without making a side better or worse. The offset acts on each stage after the blend: the stage's curve value is multiplied by `e^(degree × offset / width)`, where `degree` is its action's consistency degree in `attributes.json`, so degree 1.0 moves a stage as the offset added to its ratings would, and finishing and saving (1.3) swing more than a throw-in (0.7). Pace and the skill gates, which read ratings directly, and the hidden values take none. A substitute's match part is drawn alone as he comes on, and his period part is 0 until the next period. The draws use the `fatigue.form_match` and `fatigue.form_period` stream keys.

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

`body` is optional and draws the body fields of a team file. `engine-cli generate` needs it; without it the generated players have no body fields.

| Field | Meaning | Bound |
|---|---|---|
| body.height | one `{ "mean", "spread" }` in centimetres per position code; a draw is rounded and kept to 150 to 215 | every one of the ten codes; mean 150 to 215, spread 0 to 20 |
| body.age | `{ "mean", "spread", "min", "max" }` in years; a draw is rounded and kept to `min` to `max` | mean 15 to 45, spread 0 to 15, min 15 to 45, max min to 45 |
| body.nationality | `{ "home", "foreign", "foreign_share" }`: the club's country, the countries a foreign player comes from, and the share of foreign players | three upper-case letters each; 1 to 64 foreign codes; share 0 to 1 |

The body fields are drawn from their own random stream, after every attribute of the league, so adding or changing the block never moves an attribute. The shipped values are provisional.

`person` is optional and draws the person fields of a version 3 team file: position ratings, the feet, and the traits of play. `engine-cli generate` needs it; without it the generated players have none, and a match reads them as a converted player's. The personality values are hidden values after the last visible attribute, so they are drawn from `world.hidden` in the hidden-value stream.

| Field | Meaning | Bound |
|---|---|---|
| person.positions.related | per natural position code, the positions related to it and the gap below his natural rating each sits at, such as `"CB": { "DM": 3.0, "LB": 4.0, "RB": 4.0 }` | position codes; gap 0 to 19 |
| person.positions.spread | the bell spread around a related position's rating | 0 to 6 |
| person.positions.far | `{ "mean", "spread" }`: the bell curve of every other position (and of a keeper's outfield positions and an outfield player's keeper position) | mean 1 to 20, spread 0 to 6 |
| person.foot.left_share | the share of left-footed players | 0 to 1 |
| person.foot.weaker | `{ "mean", "spread", "min", "max" }`: the weaker foot in percent of the strong one, rounded and kept to `min` to `max` | mean 1 to 100, spread 0 to 50, min 1 to 100, max min to 100 |
| person.traits.count_weights | nine weights: how often a player carries 0, 1, ... 8 traits | each 0 or more, not all 0 |
| person.traits.strong_share | the share of traits carried strong rather than moderate | 0 to 1 |
| person.traits.keeper_only | the traits only a keeper carries; an outfield player never does | names in `traits.json`, or the content is refused naming the trait |

A player's natural position is rated at his level, the mean of the visible attributes of the groups it plays; every draw is rounded to a tenth and kept to 1.0 up to that rating, so no position is above the natural one. Traits are picked one at a time from those he may carry, skipping any he has and any opposite of one he has. The shipped weights put about 80 percent of players on 0 to 2 traits, and the shipped weaker foot centres on 40 percent, so a good weaker foot is rare. The person fields come from a third random stream, after the bodies, so no attribute or body field moves.

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

Each of the eleven personality values has three bands: `high` from 14.0, `middling` from 7.0 and `low` from 1.0. The player panel composes these words into its summary word; a personality value never leaves the engine as a number.

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

A shoot-out decides a knockout match but rates as a draw. The match record lists the ratings as `players.rating`; the live stream sends them in the `ratings` message. Each `players.rating` entry also carries the figures the launcher's season record adds up: `started` (on the pitch at kick-off), `minutes` (whole minutes on the pitch), `goals`, `assists` (goals scored from his completed pass), `clean_sheet` (none conceded while he was on the pitch, over at least `clean_sheet.min_minutes`, whatever his position) and `red`. A record written before them reads each as 0 or false.

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

Feature flags switch between the current model and a candidate, so that a paired calibration run can compare the two. The block is optional; the shipped file declares two flags, `team_stands` and `ai_style_pick`, which engine code reads (see below).

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

`team_stands` (off in the shipped file) switches the tactics file's `stands` table. Off, every stand reads 0: each player stands on his formation slot plus his role's offset in both phases, and the table is still loaded and checked. On, the table applies. The table was seeded as if the ball sat on the halfway line, so out of possession it lifts the back line 6 to 11 m and opens the space behind it; it stays off until it is seeded again from the engine's own positions with a block depth that reads the ball. Turn it on for one calibration run with `calibrate --flag team_stands=on`, or set its `state` to `on` in the file.

`ai_style_pick` (on in the shipped file) switches the AI manager's pick of a named style. On, and with a `styles.json` in the content, each AI-managed side starts a match in the style the staff advice rates best for its squad, with the lineup and bench picked for that style's formation ([styles.json](#stylesjson)). Off, an AI-managed side starts with the `ai` block's formation, mentality and duty. A side a person manages starts with the `ai` block's setup either way. Some style pairings play far outside the realism bands; the misses are recorded for the tuning work. Compare the two with `calibrate --pair ai_style_pick`.

Every statistics record carries `tuning.flags_on`, the flags that were on. To compare a candidate and remove the loser, follow [the modding how-to](../how-to/modding.md#compare-two-models-with-a-flag).

## realism-bands.json

The band registry: every realism band the calibration run judges. The bands are acceptance criteria, never tuning values. A band is added, moved, or removed by editing this file; no code changes.

| Field | Holds |
|---|---|
| `schema_version` | 5 |
| `sample_size` | the match count the time budget is set for: 1000 |
| `wall_minutes_per_sample` | the time budget of a suite of `sample_size` matches, in minutes; each suite's `wall_ms` check scales it to the run's match count |
| `stronger_team.attribute_boost` | the factor the strength suite raises every attribute of the stronger club by |
| `generalisation_floor` | the least competitions times seasons a band with a real counterpart needs to gate: 2. A band under it only reports |
| `bands` | the bands, in the order the run shows them |
| `figures` | optional: the figures, each reported beside the value an earlier build gave and never judged; see below |

Each band:

| Field | Holds |
|---|---|
| `band` | its name, unique in the file |
| `suites` | the suites that judge it: `equal`, `strength`, `formations`, `red-card` |
| `measure` | what it measures; see below |
| `design.lo`, `design.hi` | its design range, both ends included; `hi` is at least `lo`. The band passes on it |
| `design.smallest_shift` | the smallest change a change run must have the power to see, above 0 |
| `group` | optional: a name the report gives bands of one figure, such as `possession_pct` for the home and away bands |
| `real.source` | where the real value is measured: a pointer into the real-data folder, or `{"none": "<reason>"}` for a band with no real counterpart; see below |
| `real.sample` | the sample size behind the real value, such as 1826 matches; 0, and only 0, for a band with no real counterpart |
| `real.date` | the date the real value was measured, such as `2026-09-22`; for a band with no real counterpart, the date of that decision |
| `real.floor` | optional: the least real sample the band needs to gate; a band whose `real.sample` is under it only reports, because the real data, not the run, then sets its uncertainty. Never on a band with no real counterpart |
| `evidence` | `calibration` (tuning may read it) or `validation` (nothing was tuned on it). Only validation bands count toward a realism claim |
| `confidence.measurement.count` | the units behind the real value: matches, team-matches or shots |
| `confidence.generalisation` | `competitions`, `seasons` (distinct seasons in the pool) and `clubs` behind the real value |
| `condition` | the pitch and weather the band holds for: `dry`, `wet`, `worn` or `windy`. Every shipped band is `dry` |
| `max_from_dry` | optional: the most a band of another condition may sit from its dry band; refused on a `dry` band |
| `gates` | `true` when the band changes the verdict; `false` when it only reports. No default |

The file holds no real value. The calibrate command reads each value in place from the real-data folder (`--real-data`, by default the main checkout's `.scratch/out`) at each judgement, with its standard error from the same rows, and reports it beside the verdict; it never changes the verdict. A pointer has `dataset` (the pool, in words), `file` (a file of the folder; a name ending in `.gz` is decompressed), and `key`, read by one of these kinds:

- A JSON file: `key` is a dotted path, such as `shot.goals`; a `*` part takes the mean over every child, such as `*.shots` over the five leagues, and the spread of the children gives the standard error. `den` names a second path and makes the value the ratio (no standard error). `"reduce": "share"` marks a share from 0 to 1, whose error comes from `real.sample`.
- A CSV file with no quoted field: `"reduce": "mean"` is the mean over the kept rows of the sum of the `+`-joined columns of `key`, such as `hg+ag`; `"reduce": "share"` is the share of kept rows whose sum holds `op` against `value`. `where` keeps the rows whose columns hold one of the listed values, such as `{"comp": ["England", "France"]}`.
- A group share: `"reduce": "group_share"`, `group_by` and `tokens`: the share of distinct `group_by` values with at least one row whose `key` column, split on spaces, holds a listed token. `sending_off_share` reads it over `wy2_events.csv.gz`: the matches with tag 1701 or 1703 over all matches.
- A JSON `key` of several paths joined by `+` sums them, such as `goal_origin.Regular Play+goal_origin.From Counter`.

A CSV `key` may join columns by `+` or `-`, such as `hg-ag`. More CSV reductions:

- `"reduce": "bin_share"` with `bins: [lo, hi]`: the share of kept rows whose key lies from `lo` (included) to `hi` (not included).
- `"reduce": "goal_distance"` with `goal: [x, y]`: the mean distance from the point the key names as `x:y` to the goal.
- `"reduce": "scoreline"` with `scores`, such as `["5-1"]`: the share of kept rows whose two key columns (`a:b`) hold one of the scores, either way round.
- `"reduce": "factorial_ratio"`: `n × Σ x(x−1) / (Σ x)²` over the kept rows, 1 for a Poisson count.
- `"reduce": "group_range"` with `group_by` and `window`: for each group with at least `window` kept rows, the largest key less the smallest over its first `window` rows; the mean over those groups.
- `"reduce": "group_max"` with `group_by` (columns joined by `+`): the largest key of each group; the mean over the groups.

Three reductions read a whole sample for a whole-distribution band (`distribution` or `cells`, below), one histogram per real match; each file is read once, even when several bands point into it:

- `"reduce": "histogram"` with `group_by`: each kept row adds its key to the histogram of its `group_by` value (a match), binned by the band's `width` and `bins`. The key is a sum of columns, such as `hg+ag`, or a point `x:y` measured to `goal`, such as the shots of `sb_events.csv.gz`.
- `"reduce": "cells"` with `group_by`: each kept row is a match, its key the score `home:away`, one cell of the band's table.
- `"reduce": "chains"` with `group_by`, `chain` and `count`: the rows of each `group_by` value (a match) and `chain` value (a possession) are one chain, every row of the file counted. `"count": "passes"` measures each chain by its rows whose key holds one of `tokens` (split on spaces) and, with `where` and `same`, pass those filters, such as the possession team's `Pass` rows. `"count": "seconds"` measures it from its first row's `time` column (seconds, or `HH:MM:SS.mmm`) to its last row's `time` plus its key, a duration column (blank reads 0), within the first row's `period` column.

For such a band `calib.real` shows the mean of the sample's points (each bin at its lower edge; for cells, home plus away goals as capped) with its standard error over matches, and `confidence.measurement.count` the real matches read.

Any CSV pointer may also hold `weight` (a column that weights each row, for a mean or a bin share), `range` (`{"column": [lo, hi]}`: keeps the rows whose numeric column lies from `lo`, included, to `hi`, not included), `same` (`["a", "b"]`: keeps the rows whose two columns hold the same value), and `skip_blank` (`true`: skips a row whose key or weight is blank rather than refusing it).

`scale` multiplies the value and its error, such as 0.5 for a per-match rate per team. The three bands of the strength and red-card suites measure engine experiments that no real match repeats, so their `real.source` is `{"none": "<reason>"}`: the report says `no real counterpart` and counts them apart from the bands without a real value.

The report derives two words from the numbers by fixed rules. Measurement: `precise` when the real value's standard error is at most a quarter of the smallest shift, `fair` when it is at most the smallest shift, `rough` above, `unknown` without an error, `n/a` with no real counterpart. Generalisation, competitions times seasons against `generalisation_floor`: `broad` at twice the floor or more, `narrow` at the floor or more, `thin` under it, `n/a` with no real counterpart. A `thin` band, a band with `gates: false`, and a band whose real sample is under its `real.floor` print `report only`: they are judged and reported apart ("Reported bands" in `cli.md`), and left out of the joint verdict, its threshold and `calib.pass`. The floors never apply to a band with no real counterpart.

A band set from a real value has a design range that starts at least two of the real value's standard errors either side of it. A new band ships with `gates: false` and only reports. To promote it to gate, set `gates: true` once its real value is measured, its range follows that rule, and the engine has been judged against it in a full run; the next judgement of a finished run counts it, with no match played.

A measure is one of ten kinds. Its fields come from a closed list. Each side has `goals`, `shots`, `shots_on_target`, `xg`, `passes`, `passes_completed`, `pass_accuracy_pct`, `possession_pct`, `fouls`, `offsides`, `corners`, `throw_ins`, `goal_kicks`, `yellow`, `red`, and `substitutions`, named as `goals.home`, `goals.away`, or `goals.{side}` for each side in turn; each match has `injuries` and `ball_in_play_s`.

A build with the `measures` feature also gives each side the realism measures of the compact rows: `shot_distance_m` (summed over the side's shots), `pass_attempts.<cell>` and `pass_completed.<cell>` (by kind and length, such as `ground_lt10`, `ground_10_25`, `chip_gt25` and `cross_10_25`), `goals_phase.<phase>` (such as `open_play`, `corner`, `free_kick`, `penalty` and `throw_in`), `third_ticks.<third>` (`own`, `middle`, `final`), `chains`, `chain_passes`, `chain_seconds`, `players_used`, `minutes_on`, `distance_m`, `peak_mps` (summed over the players who played), and `zone_ticks.<zone>` (`walk`, `jog`, `run`, `hi`, `sprint`). A band that reads them takes only the matches whose rows carry them; a build without them gives such a band no value.

The same build gives each side the team-shape sums, named `shape.<sum>.{side}`. A sample is an open-play tick with a team in possession. Positions are read five times a second and speeds every tick; a player's speed is the distance he covered over the last 0.32 s. The outfield players fall in five groups by the position of their slot: `cb`, `fb` (left and right backs), `cm` (holding, central and attacking midfielders), `w` (wingers) and `st`. Each group has:

- `<group>.depth_in_sum` and `<group>.depth_in_n`, and the same with `out`: the summed metres each player stands behind the ball in his attacking direction, with his side on the ball (`in`) and without it (`out`), and the samples;
- `<group>.width_<flank>_<phase>_sum` and `_n`, for flank `near` and `far` and phase `in` and `out`: the metres from the long axis while the ball is more than 5 m to the player's own side of it (`near`) or to the other side (`far`), his side read from where he stood over the half;
- `<group>.pace_<level>`, for `stand` (under 0.2 m/s), `walk` (to 2), `jog` (to 4), `run` (to 5.5), `hs` (to 7) and `sprint`: the open-play ticks at each speed, and `<group>.live_ticks`, all of them;
- `<group>.speed_near_sum`, `_n`, and the same with `far`: the summed speed while the ball is on his side or the other;
- `<group>.sprints_fwd`, `_back` and `_lat`: the runs over 7 m/s for at least 1 s, forward or back by more than 3 m, or neither;
- `<group>.live_min`: `live_ticks` in minutes of one player.

Each side also has:

- `slide_x_in_sxy` and `slide_x_in_sxx`, and the same for `y` and `out`: per match, the summed products of the outfield centre's and the ball's offsets from their match means, along the pitch (`x`) or across it (`y`). Their ratio is the slope of the team's slide with the ball.
- `far_1s_30_45_pq` and `_qq`, and the same for `10s` and `gt45`: the summed products of a player's and the ball's moves over 1 s or 10 s, for players 30 to 45 m or over 45 m from the ball. Their ratio is how far a far player follows the ball.
- `dead_ticks` and `calm_ticks`: the outfield ticks with the ball dead, and those of them under 2 m/s.
- `stops_<kind>` for `throw_in`, `free_kick`, `goal_kick`, `corner`, `kick_off` (after a goal) and `penalty`.
- `links_total`, `links_top2` and `links_back`. A completed link is control passing to a team-mate in play within 5 s, with no dead ball between. The fields count all links, the links between the side's two players who received most, and the links a full-back or centre-back gave or received.

Each match also has `ball_in_play_min`.

- `{"kind": "mean", "per": "match", "of": [...]}`: the mean over matches of the sum of the fields, such as goals per match. With `"per": "team"` and `{side}` fields, the mean over each team of each match.
- `{"kind": "share", "of": [...], "op": ">=", "value": 10}`: the share of matches whose sum of the fields holds against the value; `op` is `>=`, `==`, or `<=`.
- `{"kind": "ratio", "num": [...], "den": [...]}`: the sum of the numerator fields over the sum of the denominator fields, pooled over the matches.
- `{"kind": "difference", "of": [...], "minus": [...]}`: the mean over matches of the sum of `of` less the sum of `minus`, such as the home goal difference.
- `{"kind": "scoreline", "scores": [[5, 1]]}`: the share of matches that end in one of the scores, either way round.
- `{"kind": "factorial_ratio", "of": ["goals.{side}"]}`: over each team of each match, `n × Σ x(x−1) / (Σ x)²`: 1 when the count spreads as a Poisson count, above 1 when it spreads more.
- `{"kind": "club_range", "of": [...], "minus": [...], "window": 34}`: for each club with at least `window` matches in the run, its per-match sum of `of` less the sum of `minus` (read for the other side) over its first `window` matches, largest less smallest; the mean over those clubs. It has no per-match form, so a change run judges it by its range alone.
- `{"kind": "builtin", "name": ...}`: a measure of a whole suite: `stronger_team_win_rate` (strength), `reduced_minus_full` and `full_over_control` (red-card).
- `{"kind": "distribution", "of": "goals", "width": 1, "bins": 11}`: a whole distribution, each match's histogram of its source over `bins` bins of `width`, the last bin holding everything above it. The sources: `goals` (the match's goals, both sides), `passes` (each side's passes, two points a match), and, in a `measures` build, `shot_distance` (41 bins of 1 m), `chain_passes` (16 bins of one pass) and `chain_seconds` (31 bins of 2 seconds), which must name the rows' own bins. The value is the Wasserstein-1 distance of the run's whole sample from the real one, in the source's unit.
- `{"kind": "cells", "max": 5}`: the scoreline table, each match one cell, each side's goals capped at `max` as "max or more"; the value is the total-variation distance from the real table, a share from 0 to 1.

A whole-distribution band's design range runs from 0 to its margin. It is an equivalence test: both samples are resampled by whole match, 1,999 times; it passes when the 97th percentile of the resampled distance (`upper`) is within the margin, fails when the 3rd percentile is beyond it, and is not sure otherwise. Its noise floor (`noise_floor`) is the 97th percentile of the centred resampled distance; a margin at or under it reads not sure with "margin below noise floor". A distribution with an unknown source, a `width` not above 0, fewer than 2 bins, or a measures source with other bins than the rows', and cells with `max` outside 1 to 20, are refused by name.

A field outside the list, a measure without fields, a range with `hi` below `lo`, a `smallest_shift` that is not above 0, an unknown suite, and a band name used twice are refused, and the message names the band and the field, for example `content refused: realism bands realism-bands.json: bands[0] (goals_per_match).design.hi: hi 2 is below lo 2.4`. A band without one of `design.lo`, `design.hi`, `design.smallest_shift`, `real.source`, `real.sample`, `real.date`, `evidence`, `confidence.measurement`, `confidence.generalisation`, `condition` and `gates` is refused by name, for example `bands[11] (corners_per_team).gates: is required`; so are a source that holds both `none` and a pointer, a pointer without `dataset`, `file` and `key`, a blank reason, a pointer with `real.sample` 0, a band with no counterpart and a sample above 0 or a `real.floor`, an `evidence` other than the two words, and `max_from_dry` on a `dry` band.

The figures report what no real value sets a range for. Each has `figure` (its name, unique among the figures), `suites`, a `measure` of the kinds above, and `today`: the `value` an earlier build gave, the `build` (a commit) and the `source` (how it was measured, in words). The report shows each figure's value, its standard error, the earlier value and the change, in a table of its own and in `calib.figures`; a figure has no range and no word, and never moves a verdict. A build without the measures shows a figure that reads them with no value.

A file of version 4 is a version 5 file without figures, and loads as it is; a version 4 file with figures is refused by name.

A file of version 3, with the range flat on each band and no record, still loads: in memory each band gates, its condition is `dry`, and it has no real record, so the report counts it as without a real value (`no real record`). Its bands judge as the version 4 file's: the digests hash only what decides a verdict. A file of version 2, with one fixed field per band, migrates through version 3, with each band's smallest shift a quarter of its range. The report's `calib.registry.migrated_from` says `3` or `2`. The file is not rewritten. A file of version 1 is refused.

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

The sensitivity rules, which prove that every job moves its own statistic (see [The player contract](../explanation/player-contract.md#every-rating-has-a-job)). Only the sensitivity run and the rules stage of `engine-cli calibrate --base` read the file, and both only in a build with the `sensitivity` feature; no match reads it, and the content hash does not include it.

| Field | Holds |
|---|---|
| `schema_version` | 1 |
| `design` | The size of a run: `matches` of the balanced design (an even number, 2 to 100,000), `arm_matches` of each arm (every job low, every job high), the `seed`, and the bootstrap `resamples` per interval (99 to 99,999) |
| `defaults` | The thresholds of every rule: `min_move`, the least relative move of the statistic from the low to the high level in the job's direction (0 to 10), and `ceiling`, the largest share of the outcome move of all jobs together one job may carry (0 to 1) |
| `rules` | One rule per job: `job` (an attribute, a personality value, or `height`, `age`, `nationality`), `kind` (`attribute` or `body`), `measure` (the counter the run reads), `statistic` (the job's statistic, word for word), and `levels`, the low and the high level. A rule of kind `stage` names a stage play reads instead of a job (`press.choose`, `cover.execute`), states its own `statistic` and its `direction` (`up` or `down`), and takes the levels of its main attribute's rule, whose factor it shares in the design; no other rule has a `direction`. A rule may override `min_move` or `ceiling`, with a `reason`. `report_only: true` marks a rule that runs and reports: its word is printed, and the full run does not fail on it. The rules of the 21 jobs the newer attributes and the personality values brought carry it, and so do work rate's rule and the two stage rules |
| `derived` | Fields with no job in play, proven by their own derivation test: `build` |
| `five_match` | The five-match rule: the `roles` (position codes), `runs` per role, `matches_per_run`, the `top` and `average` rating of every visible attribute of the two copies, and the `target` share of runs the top copy wins with its `tolerance` |

An attribute's levels are ratings, 1.0 to 20.0 on the tenth grid. A body job's levels are centimetres for `height` (150 to 215), years for `age` (15 to 45), and percent of adaptation for `nationality` (0 to 100), since nationality acts through adaptation. The direction of each statistic comes from the job, in `attributes.json` or in `engine.contract.body.jobs` of `tuning.json`; the rules file does not repeat it.

The measures are `pass_completion`, `take_on_completion`, `receipt_loss`, `box_finishing`, `cross_completion`, `headers_won`, `long_shots_on_target`, `tackles_won`, `skill_attempts`, `blocks`, `pass_progress`, `xg_per_shot`, `pressed_completion`, `loose_ball_share`, `fouls`, `lapses`, `anchor_distance`, `top_speed`, `close_loose_balls`, `full_time_energy`, `standing_tackles_lost`, `fouled_kept`, `aerial_balls_reached`, `half_time_gain`, `injuries`, `saves_held`, `far_saves`, `high_claims`, `near_saves`, `keeper_kicks`, `keeper_throws`, `crosses_claimed`, `offsides_won`, `sweeps`, `rating_spread`, `late_energy_loss`, `mental_rating`, `cover_distance`, `timed_runs`, `corner_delivery`, `free_kick_success`, `penalties_scored`, `throw_distance`, `flair_attempts`, `rash_chases`, `presses_behind`, `headers_in_traffic`, `press_goal_side` (the share of presser ticks beyond the engaging range on which the presser stood goal-side of the carrier), `line_gap` (the mean depth gap of the back-line players from their line out of possession, m) and `recovery_sprints` (recovery runs that drew a sprint). Two jobs may share a measure: dribbling and agility both read `take_on_completion`, and jumping and height both read `aerial_balls_reached`.

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

A personality value inert in a match (loyalty, money motivation, controversy) expects no move instead: its rule passes when the move's 95 percent interval lies inside `min_move` either way, fails when the interval of the absolute move lies at or beyond `min_move`, and is not sure otherwise.

The five-match rule passes when the 95 percent interval of the pooled share of runs lies within `target` plus or minus `tolerance`, fails when it lies wholly outside, and is not sure otherwise.

## gate-proof.json

The gate proof (see [Run the gate proof](../how-to/calibration.md#run-the-gate-proof)). `engine-cli calibrate proof` reads the copies and writes `proof`; every `calibrate` run compares `proof.gate` with its own gate and says when they differ. An unknown field is refused by name.

| Field | Holds |
|---|---|
| `schema_version` | 1 |
| `copies` | The broken copies, each with `name` (the flag's name in the proof's content folder), `measure` (the band whose measure it moves), `move` (the expected move, in words), and `flag`, a flag body as in [flags](#flags) without `state`: `owner`, `hypothesis`, `removal_condition`, and `overrides` |
| `proof` | `null` until a proof runs; then the last proof, below |

The shipped copies:

| Copy | Band it moves | Overrides | Expected move |
|---|---|---|---|
| `shots_up` | `shots_per_team` | `engine.shot_range` 23.25 | up about 10 percent |
| `fouls_halved` | `fouls_per_team` | `engine.foul_base` 0.04, `engine.foul_aggression_weight` 0.02, `engine.foul_tackling_weight` 0.01 | about half |
| `card_storm` | `yellow_cards_per_team` | `engine.foul_base` 1.0, `engine.yellow_base` 1.0 | far up: every tackle a foul, every foul booked |
| `pass_completion_down` | `pass_accuracy_pct` | `engine.aim_noise` 0.25 | down about 5 points |

`proof` holds:

| Field | Holds |
|---|---|
| `gate` | The gate the proof measured: `registry` (the band registry's digest), `measures` (the measures version), and `verdict_code` (the digest of the judging code) |
| `made_at` | When the proof was written, in UTC |
| `build` | The build that played it: `hash` (the commit, `-dirty` when the tree held changes) and `measures` (`true`) |
| `seed`, `matches`, `minutes` | The first run seed, the cap of each change run, and the minutes of each match |
| `self` | The self runs, each with `name`, `seed`, `reseed` (the salt of the changed engine's new match seeds), `word` (the joint word), `matches`, `failed` and `not_sure` (the bands with that word in either table), `significant` (the bands whose change, over its standard error, passed the run's joint threshold) and `moved` (the significant bands whose change is also at least their smallest shift); and, each over the self runs, `pass_rate`, `fail_rate` and `not_sure_rate` (by joint word), `false_alarm_rate` (runs with a significant band) and `move_rate` (runs with a moved band). A `not sure` run is not a pass |
| `copies` | One entry per copy: `name`; `runs`, each with `seed`, `word`, `matches`, `failed` (the bands that read `fail`: moved, or outside their range) and `moved` (the bands that moved, empty for a missed detection); `fail_rate` (runs whose joint word is `fail`, over the runs); `detection_rate` (runs in which the band the copy names moved); `missed` (runs in which no band moved); and `moved`, each band that moved with the number of runs it did |
| `distribution_bands` | One entry per whole-distribution band: `band` and `moved_by`, the copies that moved it in any run; empty when no copy did |
| `counts` | Over every run: `runs`, `total_matches`, `pairs`, and `observations`, the matches of each engine (`changed`, `old`) |

The file holds engine results and band names only, never a real value.

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

Version 3 holds every rating in tenths of the 1 to 20 scale, written as a number with one decimal, three body fields per player, and the player as a person: a rating at each of the ten positions, his feet, and his traits of play. Every player holds both hidden values, `consistency` and `injury_proneness`, and the eleven personality values in `attributes`; a file still naming `injury_resistance` is refused by player and attribute:

```json
{
  "schema_version": 3,
  "club": { "id": "club-00000001-00", "name": "Oakmere Rangers", "short_name": "OAK",
            "kit": { "primary": "#c8102e", "secondary": "#000000" },
            "ground": { "length": 100, "width": 64 } },
  "players": [
    { "id": "p-club-00000001-00-01", "name": "Peton Tavwood", "shirt": 1, "position": "GK",
      "attributes": { "acceleration": 11.2, "professionalism": 13.1, "...": 1.0 },
      "positions": { "GK": 12.7, "CB": 3.1, "LB": 2.4, "RB": 2.9, "DM": 3.3, "CM": 2.8,
                     "AM": 3.0, "LW": 1.9, "RW": 2.6, "ST": 3.4 },
      "foot": { "strong": "right", "weaker": 45 },
      "traits": { "rushes_out": "strong" },
      "height": 189, "age": 27, "nationality": "ENG" }
  ]
}
```

| Field | Bound |
|---|---|
| converted_from | `1`, only in a copy the launcher writes of a file converted from version 1: its players may then leave out the body fields, as a converted version 1 player has none; optional, absent in a file written by hand |
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
| players[].positions | all ten position codes, each 1.0 to 20.0 on the tenth grid; the listed `position` is his natural one and no other is above it; required |
| players[].foot | `{ "strong", "weaker" }`: `strong` is `left` or `right`; `weaker` is the weaker foot as a whole percent of the strong one, 1 to 100; required |
| players[].traits | trait name (from `traits.json`) to `moderate` or `strong`; optional, absent when he has none; never two opposites |

The screens show a rating as its whole number, 1 to 20: 12.5 shows as 13.

Before kick-off the AI manager picks a player for each slot of the formation the side starts in, in slot order: a player listed at the slot's position first, then one whose position the slot's role suits, then any outfield player, and within each group the best fit by the slot role's attribute weights in `tactics.json`. A substitute for a slot is picked the same way from the bench. It names a bench of up to `ai.bench_size` from the rest, the best remaining goalkeeper first. File order breaks ties. A bad value is refused by player and field:

- a rating out of range: `content refused: team teams/x.json: players: player p-club-00000001-00-04: attribute pace is 20.5; allowed 1.0 to 20.0`
- a rating off the tenth grid: `content refused: team teams/x.json: players: player p-club-00000001-00-04: attribute pace is 12.34; not on a tenth`
- a missing body field: `content refused: team teams/x.json: players: player p-club-00000001-00-04: body field height is missing`
- a position rated above the natural one: `content refused: team teams/x.json: players: player p-club-00000001-00-04: position rating CB is 15.0, above his natural position ST at 12.7`
- a missing position: `content refused: team teams/x.json: players: player p-club-00000001-00-04: position rating GK is missing`
- a weaker foot out of range: `content refused: team teams/x.json: players: player p-club-00000001-00-04: foot field weaker is 120; allowed 1 to 100`
- two opposite traits: `content refused: team teams/x.json: players: player p-club-00000001-00-04: traits cuts_inside and hugs_touchline are opposites`
- an unknown trait: `content refused: team teams/x.json: players: player p-club-00000001-00-04: trait spins_on_ball is not in the trait catalogue`
- a personality value off the scale, like any rating: `content refused: team teams/x.json: players: player p-club-00000001-00-04: attribute temperament is 21; allowed 1.0 to 20.0`

### Match condition

A player may carry an optional `condition` block: his condition for this match. Every field is optional; an absent field has no effect, and a file without the block hashes as it did before the block existed. The launcher's season record writes `recent_rating` and `goal_progress` into the copy of a team file it plays a match from (see [The season file](#the-season-file)); the team file gives the others.

```json
"condition": { "sharpness": 60, "adaptation": 40, "rest_days": 3, "matches_at_club": 12 }
```

| Field | Bound | Absent reads | Job |
|---|---|---|---|
| condition.sharpness | 0 to 100 percent | 100 | Below 100, his technical ratings drop, by 1.5 points at 0 (the `engine.modifier.sharpness` slot) |
| condition.adaptation | 0 to 100 percent | 100 | Below 100, his mental ratings drop, by 1.0 point at 0 (the `engine.modifier.adaptation` slot); a player new to a country is given a low value |
| condition.rest_days | 0 to 14 days | rested | His energy at kick-off, and a higher injury chance with fewer than 4 days |
| condition.matches_at_club | 0 to 1000 | none | Matches he has played for the club. Play does not read it; it sets how sure a reader is of his hidden values (see [hidden](#hidden)) |
| condition.recent_rating | 10 to 100, tenths of a match rating | none | His recent form, the mean of his last five match ratings: with the season block, it sets part of his kick-off mood |
| condition.goal_progress | 0 to 200 percent of the pace | none | His progress toward his season goal, 100 on track: with the season block and his ambition, it sets part of his kick-off mood |

An unknown field is refused, and a value out of range is refused by player and field: `content refused: team teams/x.json: players: player p-club-00000001-00-04: condition field sharpness is 101; allowed 0 to 100`.

### Version 2 files

A version 2 team file has no position ratings, feet, traits or personality values, and none of the attributes the version 4 catalogue added. It still loads, from the content folder and from the inputs a replay file embeds, and converts with values from related attributes. For each player, filling only what is missing, each value is the mean of the listed attributes, rounded to a tenth and kept to 1.0 to 20.0:

| New value | Converted from |
|---|---|
| marking | tackling, positioning, concentration |
| off_ball_movement | anticipation, decisions |
| corners | crossing, technique |
| free_kicks | crossing, technique, finishing |
| penalties | finishing, composure, technique |
| long_throws | strength, technique |
| flair | dribbling, technique, vision |
| determination | work_rate, composure |
| bravery | aggression, heading, strength |
| eccentricity | rushing_out, aggression |
| communication (outfield players only; it replaces the old value, and a keeper keeps his) | positioning, vision, decisions, composure, anticipation, work_rate, aggression, concentration, teamwork |
| the eleven personality values | 10.0 |
| `positions` | the listed position at his level, every other position 1.0 |
| `foot` | weaker 50; the strong side fixed by his id (about one player in four is left-footed) |
| `traits` | none |

The table is compiled into the build and never edited, so an old replay's team file converts the same way however the catalogue is tuned. A converted file is then checked as a version 3 file.

### Version 1 files

A version 1 team file holds every attribute as a whole number 1 to 100 and no body fields. It still loads, from the content folder and from the inputs a replay file embeds: each value `v` becomes `2v` tenths, so 62 becomes 12.4, and its players have no body fields. Values 1 to 4 become 1.0, the lowest rating; no player holds a rating under 1.0. Injury resistance `R` becomes injury proneness `21.0 − R` (exact on the tenths grid), and every player gets consistency 10.0; it then converts as a version 2 file does (see [Version 2 files](#version-2-files-1)). A version 1 file is checked as before: `content refused: team teams/x.json: players: player p-club-00000001-00-03: attribute pace is 120; allowed 1 to 100`. The shipped team files are version 1.

A match is played on the home team's ground. `club.ground` is optional: a file without it plays on 105 by 68 metres, and the default is never written back, so a file that gives 105 by 68 and one that gives no ground hash the same. The touchlines, the goal lines, the halfway line and every spot measured from them follow the ground; the goal, the goal and penalty areas, the penalty mark, the centre circle, the corner arcs and the 9.15 m kick distance keep their sizes from the Laws. The formation slots in `tactics.json` are drawn for 105 by 68 and scale with the ground: along the touchline by its length over 105, across by its width over 68. A ground outside the Laws is refused by club: `content refused: team teams/x.json: club.ground: Oakmere Rangers: the ground is 121 m long; the Laws allow 90 to 120 m`. A touchline that is not longer than the goal line is refused the same way.

Every other club file in `teams/` plays in the background round of a served match's matchday: `serve` pairs every club except the two of the player's match into fixtures, with a round seed derived from the match seed, and plays each on the full engine beside the match. The shipped folder holds ten clubs, so a matchday has four other fixtures. A file that does not load is left out of the round, with a `matchday.team_refused` warning that names it. No club file other than the two a match plays enters that match's content hash, save or replay.

## tactics.json

| Field | Holds | Bound |
|---|---|---|
| formations | `{ "name", "slots" }`; eleven slots `{ "x", "y", "position" }` in metres from the own goal line (1 to 100) and from the centre line (-33 to 33) on a 105 by 68 ground; the slots scale with the home ground | 1 to 16; slot 0 is the only `GK` |
| mentalities | `{ "name", "loss_weight", "rest_defence" }`: how much the danger of a lost ball counts against an option's gain (-5 to 5; below 0 the side seeks risk; see [decision](#decision)), and the fewest outfield players who stay goal-side of the ball while the team has it (0 to 10; optional, 0 when absent). Where the block stands is the line height's, not the mentality's | 1 to 9 |
| primitives | optional; the tactical primitives (see [Primitives](#primitives)) | none when absent |
| instructions | the six team instructions, each `{ "default", "levels" }` with 2 to 5 levels | see below |
| roles | `{ "name", "positions", "attributes", "in_possession", "out_of_possession", "preferred_actions", "teammates", "options" }`: the positions it suits, attribute weights (0 to 10, each a name in `attributes.json`) the AI manager uses to pick players, the role's behaviours, and its stand options (see below) | 1 to 64; a role for every position a formation uses |
| duties | `{ "name", "depth", "risk", "scale" }`: metres the anchor moves up (-15 to 15), an offset on forward passes and dribbles (-2 to 2), and a scale on the role's preferred actions (0.5 to 1.5) | 1 to 5 |
| ai | the AI manager's formation, mentality, and duty by name, and its thresholds | see below |
| stands | optional; one entry per position code in lower case (`gk`, `cb`, `lb`, `rb`, `dm`, `cm`, `am`, `lw`, `rw`, `st`), each `{ "in_possession", "out_of_possession" }` of `{ "x", "y" }`: metres the position stands from its formation slot while the team has the ball and while it does not, `x` up the pitch and `y` away from the centre line on the slot's side; a slot on the centre line takes no `y`. Never an offset from the ball: the ball shift moves the whole block after it. Play reads the table only while the `team_stands` flag of the tuning file is on; off, every stand is 0 | each -40 to 40; absent stands are 0 |
| stand_options | optional; the sizes the stand options read, in metres (see below) | see below |
| rest_defence | optional; `{ "hold_margin_m", "lane_m", "cover_familiarity_min" }`: how far goal-side of the ball a held player stands at the least (0 to 20), the width of a player's lane for the cover behind him (0 to 100), and the familiarity in rating points a team-mate needs to stay and cover a full-back who went forward (-12.7 to 12.7). Absent, nobody is held back | see below |

A role's three behaviours:

| Field | Holds | Bound |
|---|---|---|
| in_possession, out_of_possession | `{ "x", "y" }`: metres the player's anchor moves while the team has the ball, and while it does not; `x` up the pitch, `y` away from the centre line on the slot's side | each -15 to 15 |
| preferred_actions | `{ "shoot", "dribble", "progress" }`: offsets on the carrier's shot, dribble, and forward-pass scores, each times the duty's `scale` | each -2 to 2 |
| teammates | `{ "long_ball_target" }`: added to a team-mate's pass to this player in proportion to its length, so a target forward draws long balls | 0 to 2 |
| options | `{ "in_possession", "out_of_possession" }`: the stand options the player may take in each phase, in preference order, from `drop`, `widen`, `half_space`, `overlap` and `underlap`. No option names a partner; an option listed twice, or an entry that is not one of the five words, is refused. Optional, empty when absent | at most 5 per phase |

The shipped roles set every offset and `long_ball_target` to 0 and every duty's `scale` to 1. A version 1 tactics file, whose roles hold `shoot`, `dribble`, and `progress` directly and whose duties have no scale, converts on load to exactly those values, with no stands, no stand options and no rest-defence guard.

A player takes the first option of his menu whose condition holds, read from the ball, the carrier and where his team-mates stand. `drop` comes `drop_m` short of his stand when the carrier is at least `drop_gap_m` behind him and nobody stands within `space_m` of the spot, one player per team, the nearest to the carrier; out of possession a `drop` fills the centre of a back-line gap a quarter wider than the tuned gap. `widen` moves `widen_m` further out: a back-line player while his keeper or a back-line team-mate has the ball, any other player while a team-mate stands just inside him. `half_space` moves to `half_space_y` from the centre line (scaled with the ground) while a team-mate stands wider on his side. `overlap` runs `overlap_ahead_m` ahead of and `overlap_out_m` outside the nearest team-mate ahead on his side when that team-mate has the ball or is within `near_ball_m` of it; `underlap` runs `underlap_ahead_m` ahead and `underlap_in_m` inside him when that team-mate holds the touchline.

While a team has the ball, at least its mentality's `rest_defence` outfield players stand goal-side of the ball, or of the halfway line while the ball is in the team's own half, and never more than the outfield players less one. Below that number, the players with the most defensive duty and the least far forward are held back at their stand across the pitch; above it, a support or defend duty goes forward only with a team-mate goal-side within `lane_m` across, and when a full-back goes forward the nearest team-mate who would also go stays to cover him if his familiarity is at least `cover_familiarity_min`.

The instructions are `pressing` (levels `{ "name", "press_count", "press_distance_scale" }`: how many players press the carrier, 0 to 4, and a scale on `press_distance`, 0.1 to 3), `width` (five levels from `very_narrow` to `very_wide`, each `{ "name", "value", "wide_target" }`: a scale on each slot's distance from the centre line while the team has the ball, and the weight the carrier gives a pass to a team-mate in a wide channel of the middle or final third, -2 to 2, 0 when absent; out of possession every slot keeps the formation's width), `tempo` (the circulation's patience, five levels from `very_fast` to `very_slow`; see [decision](#decision)), `line_height` (metres the block moves up while the other team has the ball), `passing_directness` (five levels from `very_short` to `very_direct`: a weight on a pass's gain toward the attacked goal, per 45 m gained; a pass back or across gains none), and `time_wasting` (a factor on the drawn restart extra while the team leads, or on the fixed restart delay when the content has no restart wait table). Every level other than a pressing level is `{ "name", "value" }`, with the value -20 to 20.

| ai field | Meaning | Default |
|---|---|---|
| formation, mentality, duty | the setup before kick-off | `4-4-2`, `balanced`, `support` |
| trailing_minute | from this minute a trailing team raises its mentality one step and presses high, once per score | 70 |
| leading_minute | from this minute a leading team lowers its mentality one step and wastes time, once per score | 80 |
| check_interval_s | simulated seconds between checks; an injury triggers one at once | 30 |
| fatigue_energy, fatigue_from_minute | from this minute a player below this energy is replaced | 0.55, 55 |
| keep_for_injury_until_minute | until this minute one substitution is kept back for an injury | 80 |
| bench_size | substitutes named before kick-off, one a goalkeeper when the squad has one | 7 |
| out_of_position_margin | rating points (0 to 10) added to a player's rating at a slot of the position he is listed at, when the AI picks its lineup: another player starts there only when his rating at the slot is higher by more than this. Optional; a version 1 file converts with 0 | 1.5 in the shipped file |

The AI picks its starters by each player's rating at each slot's position (the team file's `positions`), plus the margin at his listed position: the best player-and-slot pairs first, a goalkeeper slot to a goalkeeper while one is free, then swaps of two players that raise the summed rating. The bench covers the goal, then each line (defence: CB, LB, RB; midfield: DM, CM, AM; attack: ST, LW, RW) with at least one player, then a full-back and a winger when the squad has a spare, then the positions the formation uses most. A substitute is the bench player with the best rating at the slot.

A bad value is refused by field, and a role attribute the attribute schema does not hold is refused by name.

### Primitives

A tactical primitive is one setting of the tactics board, as data. Each entry of `primitives` says what kind of setting it is, which plan inputs it sets, where play reads each input, which statistic it moves and which way, and where its level values come from. No match reads an entry: play reads only the level values. The entries let the load checks and the harness (`engine-cli primitives`) hold every setting to the same frame.

| Field | Holds |
|---|---|
| name | the primitive's name, unique in the file |
| kind | `graded` (5 ordered levels), `choice` (a list of choices) or `on_off` (two states, off first) |
| levels | a graded primitive's level block: `instructions.<instruction>` or `mentalities`, whose levels hold the values, so a match stores a level as an index; or `own`, when the primitive lists its own levels in `steps` |
| steps | a graded primitive's own five levels, in order, each `{ "name", "values" }`: the value of each of its inputs |
| choices | a choice primitive's choices, each `{ "name", "values", "stat" }`: the value of each input and the statistic the choice raises |
| states | an on/off primitive's two states |
| default | the level, choice or state a side has when nothing is set |
| inputs | `{ "input", "hook" }`: each plan input it sets and the hook that reads it. The hooks are a fixed list in the engine (for example `pass.recycle` reads `patience`, `option.loss_danger` reads `loss_weight`); a hook reads exactly one input |
| signature | `{ "stat", "direction", "steps" }`: the statistics the primitive moves, primary first, each `up` or `down` as the level rises. `steps` is optional: `below_default` or `above_default` makes a graded primitive's statistic judge only the steps on that side of the default (forward runs judge their upper levels on runs in behind and their lower levels on runs coming short); every step is judged by some statistic, or the load refuses the entry. Statistics: `passes_per_possession`, `recycle_share`, `turnovers_per_possession`, `shot_share`, `block_height_m`, `hold_dribble_share`, `ppda`, `forward_pass_share`, `long_ball_share`, `own_third_long_share`, `goal_kick_m`, `keeper_long_share`, `wide_pass_share`, `entries_left_share`, `entries_right_share`, `entries_centre_share`, `entries_wide_share`, `entries_centre_left_share`, `entries_centre_right_share`, `outside_box_shot_share`, `shot_distance_m`, `takeons_per_90`, `crosses_per_final_third_possession`, `high_cross_share`, `low_cross_share`, `cross_origin_m`, `off_plan_share`, `runs_in_behind_share`, `coming_short_share`, `fullback_runs_per_match`, `overlaps_per_match`, `underlaps_per_match`, `counter_attacks_per_match`, `regain_to_shot_s`, `counterpress_pressures_per_loss`, `counterpress_regain_share`, `last_counterpress_s` |
| data | `{ "source", "competitions", "row" }`: the real data the levels are fitted to and the row of `signatures.json`, or `{ "none" }` with the reason no real data fits. `deferred`, beside a source, names what is not fitted or judged yet and why: those levels or choices keep their seeded values and hold no fit. `deferred_levels` names the levels the reason covers when it covers only some; the other levels but the default then hold fits |
| fits | optional; the fit of each level: `{ "level", "values", "target", "achieved", "noise", "matches", "seed", "unreachable" }`. `target` is the real ratio of the level's quintile median to the real median, `achieved` the engine's ratio of the level to the default level, with its paired noise (1.96 standard errors) |

The load refuses, by name: a graded primitive whose block has other than 5 levels, a choice primitive with no choices, an on/off primitive without exactly two states, a default that is not one of its levels, no input, an unknown hook, a hook that reads another input, an unknown statistic, no signature, neither a source nor a reason (or both), a deferral that holds a fit, own steps other than 5, a step or choice value outside its input's range, more than 16 primitives that own their levels or choices, a held-out competition (the Wyscout German league 2017/18, the 2018 World Cup and Euro 2016 are never read), and a fit whose values differ from its level's.

A match keeps the level or choice of each primitive that owns its levels or choices in one byte per primitive, in the file's order, beside the six instructions; a saved match keeps them too.

Three primitives act only in the seconds after the ball changes side in open play. `on_winning_the_ball` (input `counter`, hook `regain.counter`; and `pace_lift`, hook `pace.lift`) acts for 13 seconds after the team wins the ball, while it keeps it: a pass gains `counter` per 45 m it goes toward the attacked goal, the hold loses `counter`, a positive `counter` lets the attack-line players start a run in behind from up to 10 m per unit from the defensive line, and `pace_lift` 1 lifts the team's pressing and support jobs one urgency grade. `on_losing_the_ball` (input `counter_press`, hook `loss.counter_press`) acts for `counter_press_length` seconds (input `counter_press_s`, hook `loss.counter_press_window`) after the team loses the ball, while the other team keeps it: the distance from which the nearest players start to press takes the factor 1 plus `counter_press` (0 at -1: nobody presses). A dead ball closes both windows. Every default is neutral, so a team at its defaults plays as before. The two counter-press primitives keep seeded values and are `deferred`: in this engine more pressers for longer after a loss do not yet win the ball back, so they are judged and fitted with pressing intensity and the tackle decision.

A version 4 file converts on load: the tempo levels, their default and the tempo primitive's default and fit records take the names `very_fast`, `fast`, `normal`, `slow` and `very_slow` in place of `very_direct`, `direct`, `balanced`, `patient` and `very_patient` (and the earlier `faster`, `normal` and `slower`), with the same values and order. Passing directness keeps its names. The file takes each transition primitive entry it does not hold by name, from a copy compiled into the build.

A version 3 file converts on load: width and passing directness take the five shipped levels, and the file takes each shipped in-possession primitive entry it does not hold by name, from a copy compiled into the build.

A version 2 file converts on load: tempo takes the five shipped levels, each mentality drops its block depth and its flat shot, pass and hold offsets and takes the shipped loss weight of its place in the list, and the file takes the shipped primitive entries. Its trait conflicts follow the level order: a tempo level below the default holds `tempo:faster`, one above holds `tempo:slower`, and the same for the other instructions; the lowest mentality holds `mentality:defensive` and the one below the default `mentality:cautious`.

## signatures.json

The real-team signature table. For each statistic a tactical primitive moves, it holds the spread of that statistic across the real club seasons of the fit data: the quintile edges at 20, 40, 60 and 80 percent, the range and the median of each quintile, and the lowest and highest team. A graded primitive's five levels are fitted to the five quintile medians, as ratios to the middle quintile's median. `engine-cli signatures` writes it from the Wyscout 2017/18 club data of England, France, Italy and Spain (see [the CLI reference](cli.md#signatures)); write it again with the same command, never by hand.

| Field | Holds |
|---|---|
| schema_version | 3 |
| held_out | the held-out sets the table never read, as `source:competition` |
| rows | one per statistic: `primitive`, `stat`, `source`, `competitions`, `teams`, `matches`, `quintile_edges` (4), `quintile_ranges` (5 pairs), `medians` (5), `min`, `max`. A thin-sample row (`thin_sample: true`: the full-backs' runs of the 13 SkillCorner A-League teams, and the forwards' runs of the 32 teams of the 2022 World Cup's tracking data, which also carry `national_teams: true`) holds five targets spread evenly from the lowest to the highest team as its medians, in place of quintiles. The StatsBomb transition rows take one value per team and competition from the 2023/24 Bundesliga, the 2022 World Cup and Euro 2020 and 2024, and carry `national_teams: true` |
| checks | the same statistic in another source, for comparison only: `stat`, `source`, `competitions`, `teams`, `median`, `min`, `max`; a check of another definition of a row's statistic also holds `rank_correlation` with that row across the teams and `against`, the row |

The real statistics and the engine's measures share one definition. A possession starts at a team's action on the ball and passes to the other team at that team's first action on it, unless the team that had the ball acts on it again within 2 seconds while the other team completed no pass and took no shot: that short regain stays in the first team's possession. A dead ball or the end of a half ends a possession. Its passes count open-play passes and restart kicks; a pass is forward when it gains 5 m or more toward the attacked goal; the recycle share is the open-play passes before the possession's first forward pass over all its open-play passes; a turnover is a possession the other team takes with no shot in it. Each statistic is pooled over a team's season (sum over sum) before the quintiles are taken across the teams. PPDA is the sum of the opponent's passes in its build-up zone over the sum of the team's defensive actions.

The transition rows read the same possessions. A regain is a possession that starts when the other team's possession ends in open play. A fast break (`counter_attacks_per_match`, from the four Wyscout leagues) is a regain that starts at least 7 m inside the team's own half and reaches the attacked final third within 13 seconds; both values come from the possessions Wyscout's analysts tagged as counter-attacks (80 percent of those that reach the final third reach it within 12.6 seconds, and 80 percent start at or behind 7 m inside the own half), and the tagged count stays as a check. From the StatsBomb open data, with the same possession rule: `regain_to_shot_s`, the seconds from a regain to its first shot; `counterpress_pressures_per_loss`, the losing team's pressures within 5 seconds of an open-play loss; `counterpress_regain_share`, the losses it wins back within 5 seconds; and `last_counterpress_s`, the seconds to its last pressure within 10 seconds. The checks hold the Bundesliga alone, the share of StatsBomb's own counterpress-flagged pressures that fall within 5 seconds of a loss, and SkillCorner's counter-press engagements a match. A StatsBomb competition that is a held-out tournament (the 2018 World Cup, Euro 2016) stops the command.

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

## restart-waits.json

The restart wait table. A stoppage lasts its visible steps (the taker walking to the ball, the
players the law moves walking away from it, a wall forming) plus a drawn extra for the unseen
part, such as fetching the ball. While play is stopped the ball waits where it left play for the
drawn extra, then appears at the restart spot; the restart is taken the drawn extra after the
visible steps are done. The file is optional: content without it plays the fixed delays of
`restart_delay_s` with the ball at the spot at once, as before the table existed.

`engine-cli restart-waits` writes it from the real dead spells (`idsse_dead.csv`) less the
engine's own visible steps, measured on seeded matches with every extra at 0 (see
[the CLI reference](cli.md#restart-waits)). Ball in play is never an input; the minutes of ball
in play follow from play. Write it again with the same command when play at a dead ball
changes; never by hand.

| Field | Unit | Holds |
|---|---|---|
| schema_version | | 1 |
| source | text lines | where the numbers came from: the real file and its spell count per kind, the spells left out, the measured visible steps per kind with the seeds and the engine version |
| kinds.throw_in, goal_kick, corner, free_kick, penalty, kick_off, drop_ball | | one entry per kind of restart; a kick-off follows a goal, a dropped ball an injury |
| kinds.*.extra_s | s | the extra at 0, 5, ..., 100 percent: 21 values, each 0 to 600, never falling. A draw `u` (0 to 1) reads linearly between the two nearest |
| kinds.*.steps_limit_s | s | 10 to 300: a restart whose visible steps are not done this long after the extra is taken anyway and logged (`rules.dead_ball_stalled`) |
| late_lead_factor | factor | 1 to 5: the extra of a team that leads in the last `pace.late_minutes` of regulation is this much longer (1.7: a leading team takes about 70 percent longer late on). A team that leads with time wasting on multiplies the extra by its time-wasting level as well |
| quick.kinds | kind names | the restarts a taker may take quickly, with no extra |
| quick.base, quick.trailing, quick.skill_slope | share | the chance of a quick restart: `base`, plus `trailing` when his team trails, plus `skill_slope` times his decisions-and-flair share less 0.5, kept within 0 and 1 |
| quick.free_mate_m, quick.free_space_m | m | a quick restart needs a free team-mate: an outfield team-mate within `free_mate_m` of the spot with no opponent within `free_space_m` of him. It also needs the opponents out of shape: fewer of their outfield players goal-side of the spot than their mentality's minimum. A team with time wasting on never restarts quickly |

The draws come from two random streams of their own (`restart_wait` and `quick_restart`), so
the table changes no other draw. A replay stores the file among its inputs (role
`restart_waits`) and plays with it; a replay without it plays the fixed delays.

## styles.json

The named styles. An AI manager picks one before each match from the staff advice (see the `ai_style_pick` flag in [flags](#flags)), and the styles suite of `engine-cli calibrate` plays every ordered pairing of them ([the CLI reference](cli.md#calibrate)). The file is optional.

```json
{
  "schema_version": 1,
  "advice_noise": 0.5,
  "styles": [
    {
      "name": "low_block",
      "mentality": "cautious",
      "instructions": { "pressing": "low", "line_height": "deep", "width": "narrow" },
      "duties": { "CB": "defend", "DM": "defend" },
      "formations": ["4-4-2", "4-1-4-1", "4-4-1-1"],
      "identity": [
        { "stat": "possession_pct", "direction": "down" },
        { "stat": "xg", "direction": "down", "against": true }
      ],
      "fit": { "positioning": 0.40, "concentration": 0.25, "anticipation": 0.20, "teamwork": 0.15 }
    }
  ]
}
```

| Field | Holds | Bound |
|---|---|---|
| schema_version | 1 | |
| advice_noise | the spread of the advice's error on each style's estimate, on the 1 to 20 rating scale | 0 to 5 |
| styles | the styles, in the order the report lists them | 1 to 16 |
| styles.*.name | the style's name | 1 to 32 characters, unique |
| styles.*.mentality | a mentality of `tactics.json` | |
| styles.*.instructions | instruction name to level name, from `tactics.json`; an instruction left out keeps its default level | |
| styles.*.duties | position code to duty name: the duty of each slot of that position; a position left out takes the `ai` block's duty | 1 to 10 entries |
| styles.*.formations | formations of `tactics.json` that suit the style, in preference order | 1 to 16 |
| styles.*.identity | the statistics that show the style: `stat` (one of `possession_pct`, `passes`, `pass_accuracy_pct`, `shots`, `shots_on_target_share`, `fouls`, `corners`, `goals`, `xg`), `direction` (`up` or `down`), and `against` (`true` reads the other side's figure) | 2 or 3 |
| styles.*.fit | attribute name to weight: the attributes the style's key jobs read; each must be a visible technical, mental or physical attribute | 1 to 12 entries; each weight 0 to 1, sum above 0 |
| styles.*.fit_positions | the outfield positions whose players the fit reads; every outfield position when absent | optional; not empty, no `GK` |

The staff advice, for each style: the lineup the AI would field in each of its formations, the formation whose lineup has the best summed rating, and the weighted mean of the fit attributes over that lineup's players (of `fit_positions`) less the same players' mean over every visible outfield attribute, plus an error drawn with spread `advice_noise`. The draw comes from a generator of its own, seeded by SHA-256 over the match seed, the side, the club id and a label, so the advice reads no match random stream. The best estimate wins; nothing keeps the estimates.

A file with an unknown formation, mentality, instruction, level, duty, position or attribute, an empty formation list, an identity list outside 2 to 3, an unknown identity statistic, a weight outside 0 to 1 or weights that sum to 0 is refused by field, for example `content refused: styles styles.json: styles[0].formations: 4-2-4 is not a formation`. A replay stores the file among its inputs (role `styles`) and plays with it.

## Generating clubs

```bash
engine-cli generate --seed 7 --clubs 20 --out my-league
```

Writes one version 3 file per club, named by club id. The same seed and the same content give the same files. Pass `--force` to overwrite. The tuning file must hold the `generator.body` and `generator.person` blocks. Every generated value is drawn on 1.0 to 20.0, so the command lifts nothing; it still prints the count it would lift: `lifted 0 values below 1.0 to 1.0`.

## Runtime files

The engine writes these files to the data folder. The data folder is `SM_DATA_DIR`. When `SM_DATA_DIR` is not set, the data folder is `%LOCALAPPDATA%\SoccerManager` on Windows and `$HOME/.local/share/SoccerManager` on other systems.

| File | Written by | Holds |
|---|---|---|
| `owner.id` | every command, once | 32 hexadecimal characters that name the owner of every match on this machine |
| `engine.port` | `serve`, while it serves a match | the socket port, as one line |
| `settings.json` | `launch`, when the player saves the settings | the player's three settings; see below |
| `views.json` | `launch`, when the player saves a squad view, and when the Squad screen first reads a club after a match | each club's named squad views and its players' last ten match ratings; see below |
| `season.json` | `launch` with the start screen, when the match after a finished one starts or the Squad screen first reads a club after it | each club's season record and each player's season goal; see below |
| `matches/<match.id>/team-a.json`, `team-b.json` | `launch` with the start screen, before a club's match after its first | the copy of a side's team file the match is played from, with each recorded player's recent form and goal progress; see below |
| `matches/<match.id>/events.jsonl` | `simulate`, `serve`, `resume` | one match event per line, as the `match-event` record |
| `matches/<match.id>/stats.json` | `simulate`, `serve`, `resume` | the statistics of the match, as the `match-stats` record |
| `matches/<match.id>/snapshot.smsn` | `simulate`, `serve` | the newest snapshot of the match; `resume` and `serve --resume` read it |
| `matches/<match.id>/matchday-bug-<fixture>.json` | `serve` | the bug report of a background match that failed; see below |
| `runs/<run.id>/run.json` | `calibrate`, when it opens the run folder | the run's identity: its `run.id`, its start in Unix milliseconds, and the build, content, fixtures hash, flags, paired flag, seed, minutes, strength boost, random-number scheme, fixture scheme, measures version, and band registry version. A run into the same folder resumes only when its identity is the same |
| `runs/<run.id>/report.json` | `calibrate` | the run report, as the `run-report` record |
| `runs/<run.id>/target.json` | `calibrate` | the run's matches per suite, the pilot and cap of a change run, and the digest of each band it was last judged with |
| `runs/<run.id>/accepted.json` | `calibrate accept` | the bands accepted as `not sure`: `schema_version` (1), `run_id`, `identity` (the first 12 hex characters of the SHA-256 of the run's identity, as `report.json` holds it under `run.identity`), and `accepted`, one entry per band with `band`, `at` (Unix milliseconds) and `word` (`not sure`). A file of another run or identity, or one that cannot be read, is ignored, and `calib.accepted.ignored` says why |
| `runs/<run.id>/rows/<session>-<thread>.rows` | `calibrate` | one compact row per match of the run; see "The row file" below |
| `runs/<run.id>/ledger/<session>-<suite>-<thread>.jsonl` | `calibrate`, after each finished work unit | one JSON line per finished work unit: the `suite`, the `unit`'s place among its suite's units in the session, and the `fixtures`, each with its fixture `key`, engine `seed`, and `match` identifier. A run into the same folder plays only the fixtures that have no line |
| `runs/<run.id>/stats/<match.id>.json` | `calibrate` | the statistics record of each recorded match: about 1 in 16 and every outlier, or every match with `--keep-events all` |
| `runs/<run.id>/events/<match.id>.jsonl` | `calibrate` | the event rows, with commentary, of each recorded match that played to full time |
| `runs/<run.id>/arms/<arm>/` | `calibrate --pair` | the `ledger/`, `rows/`, `stats/`, and `events/` of each arm, `off` and `on`; `run.json`, `report.json`, and `target.json` stay in the run folder |
| `runs/<run.id>/superseded/<old run.id>/` | `calibrate`, when the folder holds a run of another identity | the old run's `run.json`, `ledger/`, `rows/`, `stats/`, `events/`, `arms/`, `report.json`, and `target.json`, moved there before the new run starts |
| `calibrate/base/v2/<build id>/<content hash>/seed-<S>-minutes-<M>/` | `calibrate` with an old engine | the old engine's run folder: its rows are the cache of its results. `v1` folders, from before rows, are never read and can be deleted |

A `match.id` is `<seed as 16 hexadecimal characters>-<start time in milliseconds>`.

### The records

The records are JSON objects with flat, dotted keys. Every record carries `record.kind`, `schema.version`, `service`, `operation`, `env`, `version`, `build.hash`, `owner.id`, and, for a match, `match.id`. The JSON Schema (draft 2020-12) of each record kind is in `schemas/observability/`:

| `record.kind` | Schema file | Written |
|---|---|---|
| `match-event` | `match-event.schema.json` | one line in `events.jsonl` for each event, at each stoppage and at full time |
| `match-stats` | `match-stats.schema.json` | `stats.json`, at each stoppage and at full time |
| `run-report` | `run-report.schema.json` | `report.json` at the end of a calibration run, and the one line `bench` prints |

The engine writes a record at each stoppage snapshot and at the end of the match. A match that stops early keeps every record up to the newest stoppage.

### The row file

A calibration run writes one compact row per match into `rows/` of its run folder (of each arm's folder in a paired run). Each thread of each session writes its own file, `<session>-<thread>.rows`. A row holds the match's unrounded counts, numerators and denominators, keyed by fixture; it holds no event and no commentary. All numbers are little-endian.

The file starts with a header:

| Bytes | Field |
|---|---|
| 6 | the magic `SMROWS` |
| 4 | the format version: 2 (format 1, written before the realism measures, is still read) |
| 4 | the measures version (`run.identity.measures`) |
| 4 | the number of columns |
| per column | 1 byte name length, the name, 1 byte type: 1 is an 8-bit integer, 4 a 32-bit integer, 8 a 64-bit integer, 9 a 64-bit float |

Blocks follow, one per finished work unit:

| Bytes | Field |
|---|---|
| 1 + n | the suite code, length first |
| 4 | the unit's place among its suite's units in the session |
| 4 | the row count |
| per column | the column's value for every row of the block, in row order |
| 8 | the first 8 bytes of the SHA-256 of the block's bytes before them |

The columns of format 1, which open format 2, in file order (a `.home` and `.away` pair is one column per team):

| Column | Type | Holds |
|---|---|---|
| `fixture.key` | 64-bit | the fixture key; its 16 hexadecimal characters start the match identifier |
| `engine.seed` | 64-bit | the seed the engine played the match with |
| `outcome` | 8-bit | 0 played to full time, 1 the engine returned an error, 2 the match panicked |
| `record.reasons` | 8-bit | why the match has a statistics and an event file, as bits: 1 the 1-in-16 sample (the key is a multiple of 16), 2 failed, panicked or a dark-path hit, 4 a violation the running rule checker found, 8 a measure outside the 1st to 99th percentile of its suite so far, 16 `--keep-events all`; 0 for none |
| `goals`, `shots`, `shots_on_target` | 32-bit pairs | counts |
| `xg` | float pair | expected goals, unrounded |
| `passes`, `passes_completed` | 32-bit pairs | open-play passes played and completed |
| `possession_ticks` | 32-bit pair | open-play ticks credited to each team |
| `live_ticks` | 32-bit | ticks with the ball in play |
| `fouls`, `offsides`, `corners`, `throw_ins`, `goal_kicks`, `yellow`, `red` | 32-bit pairs | counts |
| `injuries` | 32-bit | injuries, both teams |
| `substitutions` | 32-bit pair | substitutions made |
| `change_never_applied`, `change_expired_at_full_time` | 32-bit | the two change counters of the statistics record |
| `validate.violations` | 32-bit | the violations the running rule checker found while the match played: the tick rules and the event rules |
| `ticks` | 32-bit | ticks played |
| `duration_us` | 64-bit | the match's wall time in microseconds |

Format 2 appends the realism measures. A build without the `measures` feature writes them as zeros, with `measures.present` 0. Each `<side>` is `home` or `away`:

| Column | Type | Holds |
|---|---|---|
| `measures.present` | 8-bit | 1 when a `measures` build played the match |
| `shot_bin.<side>.00` to `.40` | 32-bit | shots by distance from the ball at the kick to the centre of the goal attacked, in 1 m bins (`00` is under 1 m), `40` for 40 m and over; penalties included, so the bins sum to `shots` |
| `shot_distance_m.<side>` | float | the sum of those distances, in metres |
| `pass_attempts.<side>.<cell>`, `pass_completed.<side>.<cell>` | 32-bit | open-play passes played and completed, by cell: `ground_`, `chip_` (a long pass or chip) and `cross_`, each `lt10` (under 10 m), `10_25` or `gt25` (over 25 m), then `keeper_throw`, `keeper_kick`, and `unknown` (a pass kicked without a plan, 0 in a sound build); they sum to `passes` and `passes_completed` |
| `goals_phase.<side>.<phase>` | 32-bit | goals by phase: `open_play`, `corner`, `free_kick`, `penalty`, `throw_in`, `kick_off`, `goal_kick`, the restart that began the scoring side's possession chain; they sum to `goals` |
| `third_ticks.<side>.own`, `.middle`, `.final` | 32-bit | open-play ticks credited to the side with the ball in each third, counted from its own goal line; they sum to `possession_ticks` |
| `chains.<side>` | 32-bit | possession chains: from a gain or a restart taken to an opponent's touch, the ball going dead, a goal or full time |
| `chain_passes_bin.<side>.00` to `.15` | 32-bit | chains by passes, the side's open-play passes and restart kicks; `15` for 15 and over |
| `chain_seconds_bin.<side>.00` to `.30` | 32-bit | chains by length in 2-second bins; `30` for 60 s and over |
| `chain_passes.<side>`, `chain_ticks.<side>` | 32-bit, 64-bit | the sums of the chains' passes and ticks |
| `player.<side>.00` to `.15` `.squad` | 8-bit | the player's place in his team file, slots in order of first appearance |
| `player.<side>.<slot>.ticks_on` | 32-bit | ticks on the pitch: his minutes played, times 3,000 |
| `player.<side>.<slot>.distance_m` | float | metres covered: speed times the tick over every tick on the pitch |
| `player.<side>.<slot>.walk`, `.jog`, `.run`, `.hi`, `.sprint` | 32-bit | ball-in-play ticks in each speed zone: under 2 m/s, 2 to 4, 4 to 5.5, 5.5 to 7, 7 and over |
| `player.<side>.<slot>.peak_mps` | float | his highest speed, m/s |
| `physical_overflow.<side>` | 32-bit | players of the side beyond the 16 slots |

A failed or panicked match has a row with its key, its seed, its outcome and zero counts.

The statistics file rounds what the row keeps unrounded: `stats.xg` is `xg` to 2 decimals, `stats.pass_accuracy_pct` is `100 x passes_completed / passes` to 1 decimal (0 with no pass), `stats.possession_pct` is each team's `possession_ticks` over both teams' to 1 decimal, and `stats.ball_in_play_s` is `(live_ticks + 25) / 50`, rounded down. The bands read these rounded figures, rebuilt from the rows with the same functions, so a run's bands do not depend on which matches have a statistics file.

A block is written and synced before its unit's ledger line. A row counts only when the ledger line of its key was written by the session in the file's name, with the same engine seed; the rows of a unit cut off before its ledger line are never read, and the unit plays again. Reading a file stops at a block cut off part-way or with a wrong checksum, which only a stopped run leaves at the end of a file. A file of another format version, or of format 1 or 2 with another measures version or column list, is refused by name. `calib.results_digest` hashes, for every arm in key order, each row's key and its format 1 columns except `record.reasons` and `duration_us`, so it is the same with or without the measures.

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

### The season file

`season.json` holds each club's season as the launcher plays it: a stand-in for the season layer, which does not exist yet; when it does, it takes this file over. Only a launch with the start screen keeps it, and both sides of every match are recorded.

| Field | Values |
|---|---|
| `schema_version` | 1 |
| `clubs` | an object from club id to that club's part, at most 500 clubs |

Each club's part holds `matches`, the match ids it played, oldest first (at most 200; the record then stops growing and each further match logs the `launch.season_full` signal), and `players`, from player id to his season (at most 1,000 players a club, each with at most 200 ratings):

| Field | Values |
|---|---|
| `starts`, `minutes`, `goals`, `assists`, `clean_sheets`, `red_cards` | his totals over the club's matches he played, from each match's `players.rating` entry |
| `ratings` | every match rating, oldest first, each `{"match", "rating"}` with the rating from 1.0 to 10.0 |
| `goal` | `{"kind", "target"}`: his one season goal, given when he first enters the record |

A season goal is one of six kinds: `goals` (score N), `assists` (make N), `clean_sheets` (keep N), `starts` (start N matches), `average_rating` (average at least X) and `no_own_fault_absence` (miss no match through his own fault: no red card). Its kind comes from his listed position and his level there, in three bands: high from 14.0, middle from 10.0, low below.

| Listed position | High | Middle | Low |
|---|---|---|---|
| GK | clean sheets | clean sheets | starts |
| CB | clean sheets | no own-fault absence | starts |
| LB, RB | clean sheets | assists | starts |
| DM | average rating | no own-fault absence | starts |
| CM | average rating | assists | starts |
| AM | assists | assists | starts |
| LW, RW | goals | assists | starts |
| ST | goals | goals | starts |

Its target is for a season of 38 matches, on a straight line between the value at level 10 and the value at level 20, rounded to a whole number (an average rating to a tenth): goals 8 to 24 for a striker and 5 to 14 for a winger; assists 5 to 14 for an attacking midfielder and 3 to 9 for anyone else; clean sheets 8 to 18; starts 15 to 34; an average rating of 6.4 to 7.4; no own-fault absence has none.

His progress is measured against the pace of his club's matches so far: for a count, `100 × count / (target × matches / 38)`, kept to 0 to 200; for an average rating, `100 × mean / target` once he has a rating; for no own-fault absence, 100 without a red card and 0 with one. His recent form is the mean of his newest five ratings. Before a club's match after its first, the launcher writes a copy of each side's team file into the new match folder as `team-a.json` or `team-b.json`, at the current version, with each recorded player's `condition.recent_rating` and `condition.goal_progress` set, and the match is played from it, so its replay keeps them; a copy of a file converted from version 1 says `converted_from: 1`. Resume plays a saved match from the copies in its folder when they are there. A club with no match yet plays from its file as shipped.

The launcher adds the last match it ran when the next match starts or the page next reads a club, once per match. A missing file is empty. A file that does not read, has another field, breaks a limit, or is larger than 64 MiB is read as empty and logs the `launch.season_refused` signal. The launcher writes the file through a temporary file.

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
