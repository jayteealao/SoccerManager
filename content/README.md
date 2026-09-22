# Content files

The engine reads every data file from this folder. Nothing is embedded in the binary. A missing file, an unknown key, a value outside its bound, or an unknown `schema_version` stops the run with a message that names the file and the field.

## Where the engine looks

The `--content-dir` flag, if given, names the folder exactly: it must hold `attributes.json` or the run fails. Otherwise the `SM_CONTENT_DIR` environment variable, if set, names the folder exactly, with the same rule. Only when neither is set does the engine probe `./content` in the working directory, then the `content` folder beside the binary, taking the first that holds `attributes.json`.

Runtime output (`owner.id`, `matches/<match.id>/stats.json`) goes to `SM_DATA_DIR`, which defaults to `%LOCALAPPDATA%\SoccerManager` on Windows.

## Files

| File | Schema version | Holds |
|---|---|---|
| `attributes.json` | 1 | The attribute schema: 30 to 50 names in four groups |
| `tuning.json` | 1 | Engine constants, generator distributions, fatigue parameters, stream buffer |
| `rules/default.json` | 1 | The rule pack |
| `teams/default-a.json`, `teams/default-b.json` | 1 | The two default clubs (`engine-cli generate --seed 1` and `--seed 2`) |

Every file starts with `"schema_version"`. A file with another version is refused: `content refused: rules rules/default.json: schema_version 7; this build reads 1`.

## attributes.json

`attributes` is a list of `{ "name", "group" }`. Names are 2 to 32 characters and unique. Groups are `technical`, `mental`, `physical`, `goalkeeping`. The count is 30 to 50. Six names are required because the engine reads them: `pace`, `acceleration`, `passing`, `dribbling`, `tackling`, `positioning`.

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
| shot_noise | rad | 0.12 | 0 to 1.2 |
| keeper_catch_chance | probability | 0.7 | 0 to 1 |
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
| minutes_to_half_stamina | minutes | 70.0 | 1 to 600 |
| recovery_per_day | points | 20.0 | 0 to 100 |

The engine does not read these yet; the tactics slice consumes them.

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
| stoppages | one entry per kind | see file | every kind exactly once |

Each stoppage is `{ "kind", "admits_tactics", "admits_substitution" }`. Kinds: `kick_off`, `throw_in`, `corner`, `goal_kick`, `free_kick`, `penalty`, `goal`, `half_time`, `injury`. The engine does not enforce the pack yet; the match-rules slice consumes it.

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

The first eleven players in file order start the match, in formation slot order. A bad value is refused by player and attribute: `content refused: team teams/x.json: players: player p-club-00000001-00-03: attribute pace is 120; allowed 1 to 100`.

## Generating clubs

```bash
engine-cli generate --seed 7 --clubs 20 --out my-league
```

Writes one file per club, named by club id. The same seed and the same content give the same files. Pass `--force` to overwrite.
