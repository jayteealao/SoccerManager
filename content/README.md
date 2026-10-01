# Content files

This folder holds the content files the engine reads: the attribute schema, the tuning constants, the rule pack, the tactics file, the commentary lines, the realism bands, and the two default clubs.

Every field of every file, with its unit, its default, and its bound, is in [the data-file reference](../docs/reference/data-files.md). To change a value, follow [the modding how-to](../docs/how-to/modding.md).

To change behaviour with code rather than values, write a script pack: see [script packs](scripts/README.md) and the sample in `scripts/sample/`.

## Home grounds

A team file may name its club's home ground: `"ground": { "length": 100, "width": 64 }` inside `club`, in metres. A match is played on the home team's ground; a file without one plays on 105 by 68. The Laws allow a length of 90 to 120 metres and a width of 45 to 90 metres, with the touchline longer than the goal line, and the engine refuses any other ground by the club's name. The two default clubs name no ground. [The data-file reference](../docs/reference/data-files.md#teamsjson) says which distances follow the ground and which keep their sizes from the Laws.

## Feature flags

The `flags` block of `tuning.json` declares switches between the current model and a candidate. It ships empty. Each flag has five fields:

- `owner`, `hypothesis`, and `removal_condition`: required text. A flag without one of them is refused by name.
- `state`: `on` or `off`, default `off`.
- `overrides`: tuning values to use while the flag is on, by dotted path such as `engine.shot_range`.

A flag must switch something: it has overrides, or engine code reads it by name. An override must name a real value of the same type and stay inside its bound. The refusal names the flag, for example `content refused: tuning tuning.json: flags.short_shot_range.owner: is required`.

Compare a candidate with `calibrate --pair <name>`, then remove the flag with the removal checklist in [the modding how-to](../docs/how-to/modding.md#remove-a-flag).

## Realism bands

`realism-bands.json` holds the acceptance criteria that `engine-cli calibrate` checks. It is at version 2; the engine refuses a file of version 1. The bands are criteria, never tuning values: change a band only by a recorded product-owner decision.

Version 1 set four bands: goals per match 2.4 to 3.2, shots per team 8 to 16, possession 35 to 65 percent for each side, and a stronger club (every attribute times 1.15) winning more than half its matches. Version 2 adds eleven bands, checked in the equal suite:

| Band | Unit | Range | Real value | Source |
|---|---|---|---|---|
| `ten_plus_goals_share` | share of matches | 0 to 0.005 | 0 of 1,941 matches | Wyscout match files `.scratch/out/wy2_*.csv` |
| `sending_off_share` | share of matches | 0.08 to 0.22 | 0.157 | Wyscout match files `.scratch/out/wy2_*.csv` |
| `yellow_cards_per_team` | cards | 1.2 to 2.6 | 1.56 to 2.45 (3.11 to 4.90 per match) | `.scratch/out/FINDINGS.md:147` |
| `shots_on_target_share` | share of shots | 0.30 to 0.42 | 0.350 (StatsBomb), 0.356 (Wyscout) | `.scratch/out/ACTIONS.md:346`, `:413` |
| `goals_per_xg` | goals per expected goal | 0.85 to 1.15 | 0.99 (2.93 goals, 2.96 xG) | `.scratch/out/ACTIONS.md:345-346` |
| `passes_per_team` | passes | 350 to 550 | 428.6 | Wyscout match files `.scratch/out/wy2_*.csv` |
| `pass_accuracy_pct` | percent | 75 to 88 | 82.2 to 83.3 | `docs/design/realism/01-engine-realism.md:124` |
| `corners_per_team` | corners | 3.5 to 6.5 | 4.98 | Wyscout match files `.scratch/out/wy2_*.csv` |
| `throw_ins_per_match` | throw-ins | 35 to 55 | 42 to 46 | `docs/design/realism/01-engine-realism.md:128` |
| `goal_kicks_per_match` | goal kicks | 12 to 22 | 16 to 17 | `docs/design/realism/01-engine-realism.md:128` |
| `goalless_share` | share of matches | 0.04 to 0.12 | 0.0712 | `docs/design/realism/01-engine-realism.md:120` |

Shares are fractions from 0 to 1. Shots on target and goals per expected goal are pooled: the suite's total over the suite's total. Yellow cards count a second yellow, as the engine does. The `.scratch/out/` files are the local outputs behind the realism design document; they are not in the repository.

The formations suite plays every pairing of the formations in `tactics.json`, a formation against itself included: ten formations give 55 pairings. Each pairing is checked against `goals_per_match`, `ten_plus_goals_share`, and `goalless_share`.

## Formations

`tactics.json` ships ten formations. The first four keep their places, so a saved lineup keeps its shape: 4-4-2, 4-3-3, 4-2-3-1, 3-5-2. Six follow: 4-1-4-1, 4-4-1-1, 4-1-2-1-2 (the diamond), 3-4-3, 5-3-2, and 5-4-1. The computer manager starts in 4-4-2 and never changes formation during a match.
