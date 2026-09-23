# Content files

The engine reads every data file from this folder. Nothing is embedded in the binary. A missing file, an unknown key, a value outside its bound, or an unknown `schema_version` stops the run with a message that names the file and the field.

## Where the engine looks

The `--content-dir` flag, if given, names the folder exactly: it must hold `attributes.json` or the run fails. Otherwise the `SM_CONTENT_DIR` environment variable, if set, names the folder exactly, with the same rule. Only when neither is set does the engine probe `./content` in the working directory, then the `content` folder beside the binary, taking the first that holds `attributes.json`.

Runtime output (`owner.id`, `matches/<match.id>/stats.json`, `matches/<match.id>/snapshot.smsn`) goes to `SM_DATA_DIR`, which defaults to `%LOCALAPPDATA%\SoccerManager` on Windows.

## Files

| File | Schema version | Holds |
|---|---|---|
| `attributes.json` | 1 | The attribute schema: 30 to 50 names in four groups |
| `tuning.json` | 2 | Engine constants, decision weights, injury rates, generator distributions, fatigue curve, stream buffer |
| `rules/default.json` | 3 | The rule pack |
| `tactics.json` | 1 | Formations, mentalities, team instructions, roles, duties, and the AI manager's settings |
| `teams/default-a.json`, `teams/default-b.json` | 1 | The two default clubs (`engine-cli generate --seed 1` and `--seed 2`) |
| `commentary/en.json` | 1 | The English commentary lines, grouped by event kind and match situation |
| `realism-bands.json` | 1 | The accepted realism bands the calibration run checks. They are acceptance criteria, never tuning values |

Every file starts with `"schema_version"`. A file with another version is refused: `content refused: rules rules/default.json: schema_version 7; this build reads 3`.

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
| dt | s per tick | 0.02 | 0.005 to 0.1 |
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
| shot_noise | rad | 0.25 | 0 to 1.2 |
| keeper_catch_chance | probability | 0.84 | 0 to 1 |
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
| xg.intercept | log-odds | -1.0 | -10 to 10 |
| xg.distance_coef | log-odds per m | -0.1 | -2 to 0 |
| xg.angle_coef | log-odds per rad | 1.0 | 0 to 10 |

The foul chance of one tackle is `foul_base`, multiplied by `1 + foul_aggression_weight × (aggression − 0.5)` and by `1 − foul_tackling_weight × (tackling − 0.5)`, with both attributes on a 0 to 1 scale. `foul_ball_loss` is the share of fouls after which the fouled team loses the ball. After any other foul, the referee plays advantage outside the penalty area. The yellow-card chance of a foul is `yellow_base + yellow_aggression_weight × aggression`, and the red-card chance is `red_base`. A restart is taken no earlier than its `restart_delay_s`, when the taker is within `restart_ready_radius` of the spot and every opponent stands back. At three times the delay, the restart is taken whatever the players are doing. While the restarting team leads, its delay is multiplied by its time-wasting level.

An injury is rolled for the tackled player on every tackle that wins the ball or is a foul (`injury_per_tackle`), and for every player on the pitch once per simulated minute (`injury_per_minute`). Both are the chances for an average player; injury resistance 100 halves them and 0 makes them half again as likely. An injured player leaves play at once. In open play the referee stops play for a dropped ball at the ball (the goalkeeper's, inside its own penalty area), with every other player 4 m away, after `restart_delay_s.drop_ball`.

The expected goals (xG) of a shot is `1 / (1 + exp(-(xg.intercept + xg.distance_coef × d + xg.angle_coef × a)))`, where `d` is the distance from the ball to the goal centre in metres and `a` is the angle in radians that the goal mouth subtends from the ball. The match statistics sum it per team.

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
| added_time.variance_s | the most seconds the seeded variance adds or removes | 30 | 0 to 300 |
| added_time.min_s | the least added time of a half | 60 | 0 to 900 |
| added_time.max_s | the most added time of a half | 900 | `min_s` to 1800 |
| min_players | the fewest players a team may have on the pitch; fewer ends the match | 7 | 1 to 11 |

Each stoppage is `{ "kind", "admits_tactics", "admits_substitution" }`. Kinds: `kick_off`, `throw_in`, `corner`, `goal_kick`, `free_kick`, `penalty`, `goal`, `half_time`, `injury`. The engine announces every stoppage by its kind. A queued tactical change or substitution waits for the next stoppage whose kind admits it and applies on the tick that stoppage opens, substitutions first. A substitution beyond `substitutions.limit`, or at a new stoppage once `substitutions.windows` are used, is rejected with the reason. All substitutions at one stoppage share one window, and a kind listed in `windows_exempt` uses none.

The added time of a half is the sum of `per_kind` over the half's stoppages, plus `card_s` for each card, plus a seeded variance from `-variance_s` to `+variance_s`. The sum is rounded to the second and clamped to `min_s` to `max_s`. A match shorter than `halves × half_minutes` plays no added time.

## teams/*.json

```json
{
  "schema_version": 1,
  "club": { "id": "club-00000001-00", "name": "Oakmere Rangers", "short_name": "OAK",
            "kit": { "primary": "#c8102e", "secondary": "#000000" } },
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
| players | 11 to 40 entries; ids and shirts unique |
| players[].id | 1 to 64 characters |
| players[].name | 2 to 48 characters |
| players[].shirt | 1 to 99 |
| players[].position | one of the ten codes |
| players[].attributes | every schema attribute present, no other key, each 1 to 100 |

Before kick-off the AI manager picks the best-fitting player for each formation slot in slot order, by the slot role's attribute weights in `tactics.json`, and names a bench of up to `ai.bench_size` from the rest, the best remaining goalkeeper first. File order breaks ties. A bad value is refused by player and attribute: `content refused: team teams/x.json: players: player p-club-00000001-00-03: attribute pace is 120; allowed 1 to 100`.

## tactics.json

| Field | Holds | Bound |
|---|---|---|
| formations | `{ "name", "slots" }`; eleven slots `{ "x", "y", "position" }` in metres from the own goal line (1 to 100) and from the centre line (-33 to 33) | 1 to 16; slot 0 is the only `GK` |
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
| `decision` | `mentality-up-trailing`, `mentality-down-leading`, `sub-injury`, `sub-fatigue` | on `ai-decision`: the AI manager's choice |

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
