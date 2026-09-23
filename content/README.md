# Content files

This folder holds the content files the engine reads: the attribute schema, the tuning constants, the rule pack, the tactics file, the commentary lines, the realism bands, and the two default clubs.

Every field of every file, with its unit, its default, and its bound, is in [the data-file reference](../docs/reference/data-files.md). To change a value, follow [the modding how-to](../docs/how-to/modding.md).

To change behaviour with code rather than values, write a script pack: see [script packs](scripts/README.md) and the sample in `scripts/sample/`.

## Feature flags

The `flags` block of `tuning.json` declares switches between the current model and a candidate. It ships empty. Each flag has five fields:

- `owner`, `hypothesis`, and `removal_condition`: required text. A flag without one of them is refused by name.
- `state`: `on` or `off`, default `off`.
- `overrides`: tuning values to use while the flag is on, by dotted path such as `engine.shot_range`.

A flag must switch something: it has overrides, or engine code reads it by name. An override must name a real value of the same type and stay inside its bound. The refusal names the flag, for example `content refused: tuning tuning.json: flags.short_shot_range.owner: is required`.

Compare a candidate with `calibrate --pair <name>`, then remove the flag with the removal checklist in [the modding how-to](../docs/how-to/modding.md#remove-a-flag).
