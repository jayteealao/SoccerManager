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

The run report compares every realism band, and the goal bands of every formation pairing, against `realism-bands.json`. Standard error names each band that fails, and the exit code is 2 when a band fails. Add `--suite equal` to skip the formations suite, which plays 55 pairings. A `realism-bands.json` of an earlier version is refused, naming version 6: copy the shipped file of version 6.

To judge the change against the engine before it, add `--base main` from your git checkout: each band then reads `pass`, `fail`, or `not sure`. See [Measure a tuning change](calibration.md).

## Add a realism band

Every band is an entry of `bands` in `realism-bands.json`, so a new band needs no code. The file is at version 6: each band has its target, its margin with the reasons for it, how it is resampled, its engine spread and its record. To check the substitutions of each team in the equal suite against a design value, add:

```json
{
  "band": "substitutions_per_team",
  "suites": ["equal"],
  "measure": { "kind": "mean", "per": "team", "of": ["substitutions.{side}"] },
  "target": { "design": 3.5 },
  "margin": {
    "value": 1.5,
    "league_spread": 0.0,
    "noticeable_error": 1.5,
    "reason": "design value: a change of a substitution and a half per team is one a player notices",
    "spread_reason": "one population: no real substitution counts are measured",
    "design_choice": true
  },
  "definition_version": 1,
  "population": "the equal suite's matches between equal generated clubs",
  "estimator": "mean per team of the substitutions made",
  "resampling": { "engine": "league", "real": "none" },
  "min_exposure": { "unit": "matches", "value": 100, "reason": "enough matches for the bootstrap to see the band's spread" },
  "rare": false,
  "engine_spread": { "per_match_sd": 1.0, "between_league_sd": 0.0, "measured": "an estimate until a run measures it" },
  "real": {
    "source": { "none": "the measured results hold no substitution counts" },
    "sample": 0,
    "date": "2026-10-22"
  },
  "evidence": "calibration",
  "confidence": {
    "measurement": { "count": 0 },
    "generalisation": { "competitions": 0, "seasons": 0, "clubs": 0 }
  },
  "condition": "dry",
  "gates": true
}
```

The next calibration run judges it: `pass` when the whole interval of its value minus 3.5 sits inside 1.5, `fail` when the whole interval sits outside, `not sure` otherwise. The loader refuses a margin under the 97 percent floor that the band's engine spread sets, naming the margin it needs. The measure's fields come from a closed list, and a mistake is refused with the band and the field named; see [`realism-bands.json`](../reference/data-files.md#realism-bandsjson) for the list, the record's fields, the kinds of measure and how a `real.source` pointer reads a measured value. Every band gates, except a band of a group the file names under `report_only`; see [the content folder's guide](../../content/README.md#the-pass-rule).

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

Each player must have every attribute of the attribute schema, and no other attribute. A version 3 file, which `generate` writes, holds each attribute as a rating from 1.0 to 20.0 with at most one decimal, for example `12.4`, and each player's `height` in centimetres (150 to 215), `age` in years (15 to 45), and `nationality` as three upper-case letters. It also holds the player as a person: a rating at each of the ten positions in `positions` (his listed position is his natural one, and no other is above it), his feet in `foot` (the strong side, and the weaker foot as a whole percent of the strong one), and his traits of play in `traits`, which you can leave out when he has none:

```json
{ "id": "p-club-00000007-00-04", "name": "Jon Ashby", "shirt": 4, "position": "CB",
  "attributes": { "acceleration": 12.4, "pace": 11.0, "...": 1.0 },
  "positions": { "CB": 14.2, "LB": 11.0, "RB": 11.5, "DM": 10.4, "GK": 1.0, "CM": 8.3,
                 "AM": 5.1, "LW": 3.2, "RW": 3.0, "ST": 4.4 },
  "foot": { "strong": "right", "weaker": 40 },
  "traits": { "stays_back": "strong" },
  "height": 187, "age": 26, "nationality": "ENG" }
```

The eleven personality values, such as `temperament` and `leadership`, are attributes like the others in `attributes`. The data-file reference lists every field and its bounds.

The screens show each rating as its whole number, so 12.4 shows as 12 and 12.5 as 13.

Every player also holds the two hidden values, `consistency` and `injury_proneness`, as ratings like the others. Consistency sets how much his form moves from match to match and through a match: at 20.0 he plays to his ratings every time, at 1.0 his form swings most. Injury proneness sets how often he is injured: 10.0 is the average, higher is more often. No screen shows them as numbers. A player shows a word for each, such as "steady" or "rarely injured", once he has played matches for the club (`matches_at_club` in his condition block); before that both read as not yet known.

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

A version 1 file (`"schema_version": 1`) holds whole numbers from 1 to 100 and no body fields. The engine still reads it and converts it as it loads: each value is divided by 5, so 62 plays as 12.4, exactly as it played before. To write it as version 2:

1. Set `"schema_version": 2`.
2. Divide every attribute value by 5: 62 becomes `12.4`, 25 becomes `5.0`. A value of 1 to 4 divides to under 1.0; write `1.0` instead. This one change plays differently from the version 1 file.
3. Replace `injury_resistance` with `injury_proneness`: 21.0 less the converted resistance, so a resistance of 62 (12.4) becomes `8.6`. The engine converts it the same way.
4. Add `"consistency": 10.0` to every player, as the engine does, or the value you want.
5. Add `height`, `age`, and `nationality` to every player.

The result is a version 2 file. It loads, and converts again as it loads: new attributes from related ones, every personality value at the middle of the scale, a weaker foot of 50, only his listed position rated, and no traits. The setup screen then says that the club uses an older team file. To write version 3, set `"schema_version": 3` and add the fields that the data-file reference lists in its table for version 2 files.

A version 1 tactics file converts on load in the same way; the data-file reference lists its version 2 fields.

## Change the attribute schema

1. Open `my-content/attributes.json`.
2. Add the attribute to `attributes` with its job: the action and the stage it acts in, the statistic it moves, and the direction:

   ```json
   {
     "name": "flair",
     "group": "technical",
     "job": {
       "action": "dribble",
       "stage": "choose",
       "statistic": "take-ons attempted per match",
       "direction": "up"
     }
   }
   ```

3. Put the attribute in the stage table that its job names. Add it to `supports` of `actions.dribble.choose`, with a weight above 0 and below the main weight:

   ```json
   "choose": {
     "main": { "attribute": "decisions", "weight": 1.0 },
     "supports": [
       { "attribute": "dribbling", "weight": 0.3 },
       { "attribute": "vision", "weight": 0.1 },
       { "attribute": "flair", "weight": 0.2 }
     ],
     "source": "design"
   }
   ```

   A table holds at most 3 supports. The engine refuses an attribute that is in no stage table, and a job that names a stage whose table does not hold the attribute.
4. Add the new attribute to every player in every team file you use, with a rating from 1.0 to 20.0 (or a value from 1 to 100 in a version 1 file).

To remove an attribute that the engine does not require, remove it from `attributes`, from every stage table, from every role in `tactics.json`, and from every player. When it is the main attribute of a stage table, give that table another main attribute.

The schema holds 30 to 50 attributes. The engine requires 4 attributes by name, because play reads them directly: `pace`, `technique`, `agility`, and `consistency`. It refuses a schema without one of them. The sensitivity rules refuse a job that has no rule: add a rule for the new job to `sensitivity.json` before you run them.

## Make or change a style preset

A style in `styles.json` is a preset of tactical primitives. To change one, or to add one, work in your own content folder:

1. Open `styles.json` and find the style, or copy one as a starting point and give it a new name.
2. Edit its `primitives` list. Each entry is `{ "primitive": <name>, "level": <name> }`: a primitive of `tactics.json` and one of its levels, choices or states (`on` or `off`). Include one `mentality` entry. A primitive you leave out stays at its default. Set only what the style's football definition names; a preset that sets everything hides which settings make the style.
3. Edit its `duties` (a duty hint by position), `formations` (one or more formations of `tactics.json`), `identity` (two or three statistics, each with the direction the style should move it) and `rules` (one to six game-state rules, in the form a person writes; see [the data-file reference](../reference/data-files.md#stylesjson)).
4. Load the content to check the file. A primitive, level, formation, duty or rule condition the engine does not know, a primitive set twice, a preset without a mentality, and a preset with no rules or more than six are refused by name:

   ```bash
   engine-cli --content-dir my-content calibrate --suite styles --seed 9 --matches 1 --out runs/styles-check
   ```

5. Play the styles gate on your folder, as in [Run the styles gate](calibration.md#run-the-styles-gate), and read whether your style shows its identity and whether any style now dominates.

No engine code names a style, so a new style needs no build. The computer manager picks among every style in the file by its staff advice; to keep a style out of its pick, remove it from the file.

Every style in the file also shows in the style picker of the Tactics screen's Phases tab, in the file's order. Picking it sets each primitive to the style's level (and the default for one it leaves out) and replaces the game plans with its rules; the player's own instructions, the formation and the duties stay. The picker shows the style's name in words: a name the page has no words for reads as the name with its underscores as spaces, so give a new style a plain name such as `quick_wings`. A row whose level differs from the picked style shows a mark.
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
   - `on-rejected`: the on arm had a match without its statistics record, a match that panicked, a change left unapplied, a rule violation, a `stronger_team_win_rate` outside its band (in the shipped file, a stronger club that won less than half its matches), or a single-thread match time more than 10 percent slower. Remove the candidate.
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
error: content refused: team teams/x.json: schema_version 4; this build reads 3
error: content refused: rules rules/default.json: schema_version 7; this build reads 4
```

To correct the error:

1. Find the file and the field that the line names.
2. Find the bound of the field in the data-file reference.
3. Set a value inside the bound, and run the command again.
