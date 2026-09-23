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

The run report compares goals, shots, and possession against the realism bands. The exit code is 2 when a band fails.

## Add a rule pack

The engine reads the rule pack from `rules/default.json` in the content folder. To play by other rules, keep one content folder for each rule pack.

1. Copy your content folder, for example to `short-halves`.
2. Open `short-halves/rules/default.json`.
3. Change the rules. For example, set `half_minutes` to `20`, set `substitutions.limit` to `3`, or set `admits_substitution` to `false` for the `corner` stoppage.
4. Run a match with `--content-dir short-halves`:

   ```bash
   target/release/engine-cli serve --seed 42 --web web --content-dir short-halves
   ```

The rule pack decides how long each half lasts, how many substitutions each team makes, at which stoppages a queued change applies, how much added time each stoppage gives, and how few players a team can have.

## Edit team data

1. Generate clubs as a start, or copy a file from `teams/`:

   ```bash
   target/release/engine-cli generate --seed 7 --clubs 2 --out my-teams
   ```

2. Open a team file. Change the club name, the kit colours, or a player's name, shirt, position, or attributes.
3. Pass the file to the engine with `--team-a` or `--team-b`:

   ```bash
   target/release/engine-cli simulate --seed 1 --ticks-out check.ticks --team-a my-teams/club-00000007-00.json
   ```

Each player must have every attribute of the attribute schema, and no other attribute. Each attribute value is from 1 to 100.

## Change the attribute schema

1. Open `my-content/attributes.json`.
2. Add an attribute as `{ "name": "flair", "group": "technical" }`, or remove one that the engine does not require.
3. Add the new attribute to every player in every team file you use, with a value from 1 to 100. Remove a removed attribute from every player.

The schema holds 30 to 50 attributes. The engine requires 14 attributes by name, and refuses a schema without one of them. The list is in the data-file reference.

## Read a validation error

When a file has a bad value, the engine does not start the match. It prints one line that names the file, the field, and the bound, and it exits with code 1:

```text
error: content refused: tuning tuning.json: engine.keeper_catch_chance: greater than 1
error: content refused: team teams/x.json: players: player p-club-00000001-00-03: attribute pace is 120; allowed 1 to 100
error: content refused: rules rules/default.json: schema_version 7; this build reads 3
```

To correct the error:

1. Find the file and the field that the line names.
2. Find the bound of the field in the data-file reference.
3. Set a value inside the bound, and run the command again.
