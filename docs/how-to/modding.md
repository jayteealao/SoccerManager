# Tune the engine and add a rule pack

This guide shows how to change the numbers the engine plays by: the tuning file, the rule pack, the team files, and the attribute schema. Every field, with its unit, its default, and its bound, is in [the data-file reference](../reference/data-files.md).

The engine reads all of these files from one content folder when a command starts. It never embeds them in the program. A change takes effect at the next command.

## Make your own content folder

Do not edit the shipped `content/` folder. Copy it, and point the engine at the copy.

1. Copy the folder:

   ```bash
   cp -r content my-content
   ```

2. Run each command with `--content-dir my-content`, or set the environment variable once:

   ```bash
   export SM_CONTENT_DIR=my-content
   ```

3. Make sure that the engine reads your folder. Run a short match:

   ```bash
   target/release/engine-cli simulate --seed 1 --minutes 5 --ticks-out check.ticks --content-dir my-content
   ```

   The log lines on standard error name each file the engine loaded, with a `content.loaded` signal and a hash.

## Change a tuning value

1. Open `my-content/tuning.json`.
2. Change one value, for example `engine.base_speed` from `5.0` to `5.5`.
3. Run a match or a calibration run with `--content-dir my-content`.

To see the effect over many matches, run a calibration run:

```bash
target/release/engine-cli calibrate --seed 2026 --matches 200 --content-dir my-content
```

The run report compares every realism band, and the goal bands of every formation pairing, against `realism-bands.json`. Standard error names each band that fails, and the exit code is 2 when a band fails. Add `--suite equal` to skip the formations suite, which plays 55 pairings. A `realism-bands.json` of version 1 is refused: copy the shipped file of version 2.

## Add a rule pack

The engine reads the rule pack from `rules/default.json` in the content folder. To play by other rules, keep one content folder for each rule pack.

1. Copy your content folder, for example to `short-halves`.
2. Open `short-halves/rules/default.json`.
3. Change the rules. For example, set `half_minutes` to `20`, set `substitutions.limit` to `3`, or set `admits_substitution` to `false` for the `corner` stoppage.
4. Run a match with `--content-dir short-halves`:

   ```bash
   target/release/engine-cli serve --seed 42 --web viewer/dist --content-dir short-halves
   ```

The rule pack decides how long each half lasts, how many substitutions each team makes, at which stoppages a queued change applies, how much added time each stoppage gives, and how few players a team can have.

## Edit team data

1. Generate clubs as a start, or copy a file from `teams/`:

   ```bash
   target/release/engine-cli generate --seed 7 --clubs 2 --out my-teams
   ```

2. Open a team file. Change the club name, the kit colours, or a player's name, shirt, position, attributes, height, age, or nationality.
3. Pass the file to the engine with `--team-a` or `--team-b`:

   ```bash
   target/release/engine-cli simulate --seed 1 --ticks-out check.ticks --team-a my-teams/club-00000007-00.json
   ```

Each player must have every attribute of the attribute schema, and no other attribute. A version 2 file, which `generate` writes, holds each attribute as a rating from 1.0 to 20.0 with at most one decimal, for example `12.4`, and each player's `height` in centimetres (150 to 215), `age` in years (15 to 45), and `nationality` as three upper-case letters:

```json
{ "id": "p-club-00000007-00-04", "name": "Jon Ashby", "shirt": 4, "position": "CB",
  "attributes": { "acceleration": 12.4, "pace": 11.0, "...": 1.0 },
  "height": 187, "age": 26, "nationality": "ENG" }
```

The screens show each rating as its whole number, so 12.4 shows as 12 and 12.5 as 13.

Every player of a version 2 file also holds the two hidden values, `consistency` and `injury_proneness`, as ratings like the others. Consistency sets how much his form moves from match to match and through a match: at 20.0 he plays to his ratings every time, at 1.0 his form swings most. Injury proneness sets how often he is injured: 10.0 is the average, higher is more often. No screen shows them as numbers. A player shows a word for each, such as "steady" or "rarely injured", once he has played matches for the club (`matches_at_club` in his condition block); before that both read as not yet known.

### Give a player his match condition

A player may carry a `condition` block for the match: his sharpness and his adaptation to the country, in percent, his days of rest since his last match, and the matches he has played for the club. Every field is optional, and a player without the block is fully sharp, fully adapted and rested.

1. Add the block to the player:

   ```json
   { "id": "p-club-00000007-00-04", "name": "Jon Ashby", "shirt": 4, "position": "CB",
     "attributes": { "acceleration": 12.4, "pace": 11.0, "...": 1.0 },
     "height": 187, "age": 26, "nationality": "ENG",
     "condition": { "sharpness": 60, "adaptation": 50, "rest_days": 2 } }
   ```

2. Play the match with the file as before. At sharpness 60 his technical ratings play 0.6 lower; at adaptation 50 his mental ratings play 0.5 lower; two days of rest start him at energy 0.8 and raise his injury chance by half.

A value out of range stops the load with the player and the field named, for example `player p-club-00000007-00-04: condition field sharpness is 101; allowed 0 to 100`. The data-file reference lists every field and its range.

### Convert a version 1 file by hand

A version 1 file (`"schema_version": 1`) holds whole numbers from 1 to 100 and no body fields. The engine still reads it and converts it as it loads: each value is halved, so 62 plays as 12.4, exactly as it played before. To write it as version 2:

1. Set `"schema_version": 2`.
2. Halve every attribute value: 62 becomes `12.4`, 25 becomes `12.5`. A value under 2 halves to under 1.0; write `1.0` instead. This one change plays differently from the version 1 file.
3. Replace `injury_resistance` with `injury_proneness`: 21.0 less the halved resistance, so a resistance of 62 (12.4) becomes `8.6`. The engine converts it the same way.
4. Add `"consistency": 10.0` to every player, as the engine does, or the value you want.
5. Add `height`, `age`, and `nationality` to every player.

A version 1 tactics file converts on load in the same way; the data-file reference lists its version 2 fields.

## Change the attribute schema

1. Open `my-content/attributes.json`.
2. Add an attribute as `{ "name": "flair", "group": "technical" }`, or remove one that the engine does not require.
3. Add the new attribute to every player in every team file you use, with a rating from 1.0 to 20.0 (or a value from 1 to 100 in a version 1 file). Remove a removed attribute from every player.

The schema holds 30 to 50 attributes. The engine requires 14 attributes by name, and refuses a schema without one of them. The list is in the data-file reference.

## Compare two models with a flag

Use a feature flag to decide between the current model and a candidate on the realism bands.

1. Declare the flag in the `flags` block of `tuning.json`, with an owner, a hypothesis, a removal condition, and `"state": "off"`. For a candidate made of tuning values, give the values as `overrides`. For a candidate in engine code, register the flag name in the engine and read it where the candidate branches. The fields are in [the data-file reference](../reference/data-files.md#flags).
2. Run both arms on the same fixtures and seeds:

   ```bash
   engine-cli calibrate --seed 2026 --matches 1000 --pair short_shot_range
   ```

   The run plays every fixture twice: once with the flag off, into `arms/off/`, and once with it on, into `arms/on/`. Use `--suite equal` or `--suite strength` to play one suite only. Use `--flag other_flag=on` to hold another flag in one state for both arms.
3. Read the table on standard error, or `calib.compare` and `calib.verdict` in `report.json`. Each row shows one band of one suite with the off value, the on value, and the difference. A star marks a value outside its band.
4. Act on the verdict:
   - `on-rejected`: the on arm had a missing statistics record, a change left unapplied, a validator violation, a stronger club that did not win more than half its matches, or a single-thread match time more than 10 percent slower. Remove the candidate.
   - `on-better`: the on arm passes more bands, or as many and sits closer to the band centres by more than 0.05. Keep the candidate as the only model.
   - `off-better`: remove the candidate.
   - `no-difference`: run once more on a second seed, for example `--seed 2027`. If the verdict is still `no-difference`, remove the candidate.

A run of 1000 matches per suite is the decision run. Use a smaller `--matches` only while you try values.

## Remove a flag

Every flag leaves the file when its experiment ends, whatever the result:

1. Delete the flag entry from the `flags` block.
2. For a winning override, write its values into the base values. For a winning code flag, delete the losing branch. For a losing flag, delete its overrides or its code.
3. Remove the name from the engine's list of code flags.
4. Run `engine-cli calibrate --seed 2026 --matches 1000` without `--pair`, and `engine-cli bench --seed 42 --matches 5 --json`.
5. Update the tests that pin the changed values.
6. Record the path of the paired run's `report.json` in the change description.

## Read a validation error

When a file has a bad value, the engine does not start the match. It prints one line that names the file, the field, and the bound, and it exits with code 1:

```text
error: content refused: tuning tuning.json: engine.keeper_catch_chance: greater than 1
error: content refused: team teams/x.json: players: player p-club-00000001-00-04: attribute pace is 20.5; allowed 1.0 to 20.0
error: content refused: team teams/x.json: players: player p-club-00000001-00-04: attribute pace is 12.34; not on a tenth
error: content refused: team teams/x.json: players: player p-club-00000001-00-04: body field height is missing
error: content refused: team teams/x.json: schema_version 3; this build reads 2
error: content refused: rules rules/default.json: schema_version 7; this build reads 3
```

To correct the error:

1. Find the file and the field that the line names.
2. Find the bound of the field in the data-file reference.
3. Set a value inside the bound, and run the command again.
