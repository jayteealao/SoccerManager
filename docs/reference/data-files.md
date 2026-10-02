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
| `attributes.json` | 1 | The attribute schema: 30 to 50 names in four groups |
| `tuning.json` | 2 | Engine constants, decision weights, injury rates, generator distributions, fatigue curve, stream buffer |
| `rules/default.json` | 4 | The rule pack |
| `tactics.json` | 1 | Ten formations, mentalities, team instructions, roles, duties, and the AI manager's settings |
| `teams/default-a.json`, `teams/default-b.json` | 1 | The two default clubs (`engine-cli generate --seed 1` and `--seed 2`) |
| `commentary/en.json` | 1 | The English commentary lines, grouped by event kind and match situation |
| `realism-bands.json` | 2 | The accepted realism bands the calibration run checks: four from version 1 and eleven from real-match data. They are acceptance criteria, never tuning values |
| `fast-model.json` | 1 | The fast model fitted from full-engine results, with the engine id of the results it came from. The content hash does not read it |

Every file starts with `"schema_version"`. A file with another version is refused: `content refused: rules rules/default.json: schema_version 7; this build reads 4`.

## attributes.json

`attributes` is a list of `{ "name", "group" }`. Names are 2 to 32 characters and unique. Groups are `technical`, `mental`, `physical`, `goalkeeping`. The count is 30 to 50. Fourteen names are required because the engine reads them: `pace`, `acceleration`, `passing`, `dribbling`, `tackling`, `positioning`, `aggression`, `finishing`, `vision`, `decisions`, `composure`, `stamina`, `natural_fitness`, `injury_resistance`. A schema without one of them is refused by name.

The shipped schema (36):

- technical: passing, dribbling, first_touch, finishing, crossing, heading, long_shots, tackling, technique
- mental: positioning, vision, decisions, composure, anticipation, work_rate, aggression, concentration, teamwork
- physical: pace, acceleration, stamina, strength, agility, balance, jumping, natural_fitness, injury_resistance
- goalkeeping: handling, reflexes, aerial_reach, one_on_ones, kicking, throwing, command_of_area, communication, rushing_out

## tuning.json

### engine

Units are metres, seconds, metres per second, and ticks. A value outside its bound is refused, for example `content refused: tuning tuning.json: engine.keeper_catch_chance: greater than 1`.

| Field | Unit | Default | Bound |
|---|---|---|---|
| dt | s per tick | 0.02 | exactly 0.02: the match clock runs at 50 ticks per second |
| decision_interval_ticks | ticks | 1 | 1 to 50 |
| base_speed | m/s | 5.0 | 0 to 50 |
| pace_speed | m/s | 4.0 | 0 to 40 |
| base_accel | m/s² | 3.0 | 0 to 30 |
| accel_bonus | m/s² | 5.0 | 0 to 50 |
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
| foul_aggression_weight | ratio | 1.0 | 0 to 4 |
| foul_tackling_weight | ratio | 0.5 | 0 to 1 |
| foul_ball_loss | ratio | 0.6 | 0 to 1 |
| yellow_base | probability per foul | 0.05 | 0 to 1 |
| yellow_aggression_weight | probability per foul | 0.15 | 0 to 1 |
| red_base | probability per foul | 0.005 | 0 to 1 |
| foul_cooldown_ticks | ticks | 150 | 0 to 1000 |
| foul_booked_factor | ratio | 0.15 | 0 to 1 |
| tackle_win_base | probability per tackle | 0.5 | 0 to 0.5 |
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

The foul chance of one tackle is `foul_base`, multiplied by `1 + foul_aggression_weight × (aggression − 0.5)` and by `1 − foul_tackling_weight × (tackling − 0.5)`, with both attributes on a 0 to 1 scale. `foul_ball_loss` is the share of fouls after which the fouled team loses the ball. After any other foul, the referee plays advantage outside the penalty area. The yellow-card chance of a foul is `yellow_base + yellow_aggression_weight × aggression`, and the red-card chance is `red_base`. The chance that a tackle wins the ball cleanly is `tackle_win_base × tackling / (tackling + dribbling)`, with the tackler's tackling and the carrier's dribbling; a tackle that neither wins the ball nor fouls misses. A player already booked fouls less: the foul chance is multiplied by `foul_booked_factor`. A player who commits a foul, advantage or not, makes no tackle attempt for `foul_cooldown_ticks`. An opponent attempts a tackle on every tick he is within `tackle_reach` of the ball. Against a carrier running with the ball, faster than 2 m/s, the win chance gains `tackle_dribble_win × tackling / (tackling + dribbling)`; a carrier who stands and shields the ball does not. A presser runs at the point where he meets the carrier's run, and at the ball itself once he is within `press_engage` of it. A tuning file without these three fields loads with `tackle_reach` 1.0, `press_engage` 3.0 and `tackle_dribble_win` 0, which reproduce play before they existed. While play goes on with advantage, the referee holds at most one card per player, the more severe, and a player is shown at most one card when play stops. A restart is taken no earlier than its `restart_delay_s`, when the taker is within `restart_ready_radius` of the spot and every opponent stands back. At three times the delay, the restart is taken whatever the players are doing. While the restarting team leads, its delay is multiplied by its time-wasting level.

An injury is rolled for the tackled player on every tackle that wins the ball or is a foul (`injury_per_tackle`), and for every player on the pitch once per simulated minute (`injury_per_minute`). Both are the chances for an average player; injury resistance 100 halves them and 0 makes them half again as likely. An injured player leaves play at once. In open play the referee stops play for a dropped ball at the ball (the goalkeeper's, inside its own penalty area), with every other player 4 m away, after `restart_delay_s.drop_ball`.

The expected goals (xG) of a shot is `1 / (1 + exp(-(xg.intercept + xg.distance_coef × d + xg.angle_coef × a)))`, where `d` is the distance from the ball to the goal centre in metres and `a` is the angle in radians that the goal mouth subtends from the ball. The match statistics sum it per team. A penalty counts `shots.penalty_xg`.

A shot leaves at `shot_speed` with a vertical speed drawn from 0 to `shots.loft_max`, aimed with a spread of up to `shot_noise × (1.5 − finishing)` radians either side; a kick from the penalty mark multiplies the spread by `shots.penalty_spread`. As the shot is struck, the engine follows a copy of the ball with the match physics: the shot is on target when that flight crosses the goal line between the posts and under the bar, and the match statistics count it then. While a shot is in flight and faster than `control_speed`, each outfield defender within `shots.block_reach` of a ball under `reach_height` has one chance per shot, `shots.block_chance`, to block it. A blocked ball keeps `shots.block_speed` of its speed and goes back the way it came, turned by up to `shots.block_spread` either side. Only a shot on target can be saved: the acting keeper, within `keeper_reach` of a ball under the bar, has one save roll per shot. The save chance is `shots.save_high` for a shot of quality 0.05 or less, `shots.save_low` for quality 0.40 or more, and a straight line between, where the quality is the expected goals of the fixed `shots.quality` model (a penalty uses `shots.penalty_xg`), so refitting `xg` never moves the saves. The keeper holds `shots.save_hold` of his saves and parries the rest: the ball keeps `shots.parry_speed` of its speed and goes along the goal line away from the goal centre, turned by up to `shots.parry_spread` either way, with a vertical speed of up to `shots.parry_loft`. After a block or a parry the defending side touched the ball last, so it gives a corner only if it then crosses the goal line. A shot off target is never saved. `keeper_catch_chance` applies only to a fast ball that is not a shot, such as a pass or a clearance, and that no defender cleared.

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

A carrier is lone when no active outfield team-mate stands nearer the opponents' goal than he does; a goalkeeper is never lone. A team's lone forward is its one active player, other than the one keeping goal, in the formation's front line: the outfield slots less than 4 m behind the most advanced one. While his team has the ball and someone else carries it, he moves from his formation place toward the onside line, 0.5 m short of the second-last opponent, by the share `lone_line_hold`, and never past that line. With these weights a pressed lone forward holds the ball or lays it off rather than dribbling into the defender, so a side with one forward does not outscore 4-4-2. A tuning file without these five fields loads with the values that reproduce play before they existed: `tackle_win_base` 0.05 and 0 for the other four. A tuning file without `carry_s` and `carry_cost` loads with 0 for both, which turns the carry window off.

### generator

| Field | Meaning | Default | Bound |
|---|---|---|---|
| squad_size | players per generated club | 22 | 11 to 40; equals 11 plus the bench length |
| slot_positions | position of each formation slot, slot order | GK LB CB CB RB LW CM CM RW ST ST | 11 position codes |
| bench_positions | positions of the bench, in order | GK CB LB RB DM CM AM LW RW ST ST | `squad_size - 11` codes |
| per_position | one entry per position code | see file | every one of the ten codes present |

Each `per_position` entry holds `technical`, `mental`, `physical`, and `goalkeeping`, each `{ "mean", "spread" }`. `mean` is 1 to 100; `spread` is 0 to 40. A generated value is a bell-curve draw (mean plus spread times a unit normal), rounded, then clamped to 1 to 100.

### fatigue

| Field | Unit | Default | Bound |
|---|---|---|---|
| threshold | energy | 0.7 | 0 to 1 |
| curve | `[energy, multiplier]` points | `[[1,1],[0.7,1],[0.5,0.92],[0.3,0.82],[0,0.65]]` | 2 to 8 points, energy strictly falling from 1.0 to 0.0, multipliers 0.3 to 1 |
| drain_base_per_s | energy per s | 0.00008 | 0 to 0.01 |
| drain_effort_per_s | energy per s at full speed | 0.0004 | 0 to 0.05 |
| half_time_recovery | energy | 0.1 | 0 to 1 |
| recovery_per_day | points | 20.0 | 0 to 100 |

Every player starts a match at energy 1.0. Each tick drains `drain_base_per_s` plus `drain_effort_per_s` times the square of the player's speed as a share of their top speed; stamina 100 halves the drain and 0 makes it half again as fast. At and above `threshold` a player plays at full strength. Below it, top speed, acceleration, passing, finishing, decisions, and composure are the unfatigued values times the curve's multiplier, read on the straight line between the two points around the energy; they are refreshed once a second. Half-time gives back `half_time_recovery`, scaled by natural fitness. `recovery_per_day` is for the season between matches; a match does not read it.

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
| `schema_version` | 1 |
| `model` | The module the fit is for: `fitted-scores@1` |
| `engine_id` | The engine id of the results the fit came from: `golden-<ledger index>-<build>-<digest>` |
| `engine` | The id's parts: `id`, `ledger_index`, `build` and `digest` |
| `fitted_by` | The `engine_version` and `build` of the program that fitted it |
| `fit.params` | The eight parameters: `base`, `home`, `attack`, `curve`, `defence`, `dispersion` (the shape of the match factor both scores share), `rho` and `draw` |
| `fit.minute_shares` | 90 shares, one per minute of regulation time, summing to 1 |
| `batch` | The fit batch: `league_seed`, the strength `levels`, `matches_per_pairing`, `minutes` and the engine `seed` |
| `check` | The check it passed: the engine `seed`, the fast-model `draws` per match, `z`, `share_floor`, `mean_floor`, the number of `figures`, how many `failed`, and `pass` |

CI fails when `engine_id` is not the id of `gate/golden.json`: a Rust test runs in every test job, and `engine-cli fast-model stale` runs in the gate job and before every release.

The slot file's `engine.fast-model` entry picks the module: `{"module": "fitted-scores", "version": 1}`, or `{"module": "off"}`, which refuses to play. The slot never enters the content hash. Only the `fast-model` command reaches the module; a test fails the build when anything else does.

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

```json
{
  "schema_version": 1,
  "club": { "id": "club-00000001-00", "name": "Oakmere Rangers", "short_name": "OAK",
            "kit": { "primary": "#c8102e", "secondary": "#000000" },
            "ground": { "length": 100, "width": 64 } },
  "players": [
    { "id": "p-club-00000001-00-01", "name": "Peton Tavwood", "shirt": 1, "position": "GK",
      "attributes": { "acceleration": 56, "...": 0 } }
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
| players[].attributes | every schema attribute present, no other key, each 1 to 100 |

Before kick-off the AI manager picks the best-fitting player for each formation slot in slot order, by the slot role's attribute weights in `tactics.json`, and names a bench of up to `ai.bench_size` from the rest, the best remaining goalkeeper first. File order breaks ties. A bad value is refused by player and attribute: `content refused: team teams/x.json: players: player p-club-00000001-00-03: attribute pace is 120; allowed 1 to 100`.

A match is played on the home team's ground. `club.ground` is optional: a file without it plays on 105 by 68 metres, and the default is never written back, so a file that gives 105 by 68 and one that gives no ground hash the same. The touchlines, the goal lines, the halfway line and every spot measured from them follow the ground; the goal, the goal and penalty areas, the penalty mark, the centre circle, the corner arcs and the 9.15 m kick distance keep their sizes from the Laws. The formation slots in `tactics.json` are drawn for 105 by 68 and scale with the ground: along the touchline by its length over 105, across by its width over 68. A ground outside the Laws is refused by club: `content refused: team teams/x.json: club.ground: Oakmere Rangers: the ground is 121 m long; the Laws allow 90 to 120 m`. A touchline that is not longer than the goal line is refused the same way.

Every other club file in `teams/` plays in the background round of a served match's matchday: `serve` pairs every club except the two of the player's match into fixtures, with a round seed derived from the match seed, and plays each on the full engine beside the match. The shipped folder holds ten clubs, so a matchday has four other fixtures. A file that does not load is left out of the round, with a `matchday.team_refused` warning that names it. No club file other than the two a match plays enters that match's content hash, save or replay.

## tactics.json

| Field | Holds | Bound |
|---|---|---|
| formations | `{ "name", "slots" }`; eleven slots `{ "x", "y", "position" }` in metres from the own goal line (1 to 100) and from the centre line (-33 to 33) on a 105 by 68 ground; the slots scale with the home ground | 1 to 16; slot 0 is the only `GK` |
| mentalities | `{ "name", "block_depth", "shoot", "progress", "hold" }`: metres the block moves up (-20 to 20), and offsets on shots, forward passes, and holding the ball (-2 to 2) | 1 to 9 |
| instructions | the six team instructions, each `{ "default", "levels" }` with 2 to 5 levels | see below |
| roles | `{ "name", "positions", "attributes", "shoot", "dribble", "progress" }`: the positions it suits, attribute weights (0 to 10, each a name in `attributes.json`) the AI manager uses to pick players, and option offsets (-2 to 2) | 1 to 64; a role for every position a formation uses |
| duties | `{ "name", "depth", "risk" }`: metres the anchor moves up (-15 to 15), and an offset on forward passes and dribbles (-2 to 2) | 1 to 5 |
| ai | the AI manager's formation, mentality, and duty by name, and its thresholds | see below |

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

Writes one file per club, named by club id. The same seed and the same content give the same files. Pass `--force` to overwrite.

## Runtime files

The engine writes these files to the data folder. The data folder is `SM_DATA_DIR`. When `SM_DATA_DIR` is not set, the data folder is `%LOCALAPPDATA%\SoccerManager` on Windows and `$HOME/.local/share/SoccerManager` on other systems.

| File | Written by | Holds |
|---|---|---|
| `owner.id` | every command, once | 32 hexadecimal characters that name the owner of every match on this machine |
| `engine.port` | `serve`, while it serves a match | the socket port, as one line |
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
