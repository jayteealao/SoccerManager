# Engine modules and slot configuration

This page lists every slot the engine resolves at start-up, the module that fills each slot by default, the fields of a module card, and the messages the engine prints when it refuses a slot configuration. To write and register a module, see [Write and register an engine module](../how-to/engine-modules.md). For the reasons behind the slots, see [The plugin contract](../explanation/plugin-contract.md).

## The slot file

The engine reads `slots.json` from the content folder (see [where the engine looks](data-files.md#where-the-engine-looks)). The file names one module for every declared slot.

```json
{
  "schema_version": 1,
  "slots": {
    "engine.fouls": { "module": "fouls", "version": 1 },
    "engine.offside": { "module": "off" }
  }
}
```

| Field | Type | Meaning |
|---|---|---|
| `schema_version` | integer | `1`. This build reads version 1. |
| `slots` | object | One entry for each declared slot, keyed by the slot id. |
| `slots.<id>.module` | string | The registered module name, or `off` to switch an optional slot off. |
| `slots.<id>.version` | integer | The module version. Required for a module; not allowed with `off`. |

The engine resolves the file when a command that plays or serves a match starts. The file must name every declared slot and no other slot. An unknown field is refused.

The shipped `content/slots.json` is the built-in default selection. With the default selection the content hash is unchanged, so every replay gate hash holds. Any other selection is folded into the content hash, and a replay file carries the slot file as one of its inputs, so a match played with it is never taken for a default one. The `viewer.*` slots and `engine.fast-model` never enter the content hash, because neither changes a match.

## Slots

The table lists the slots in the order the engine resolves them. A required slot has no off version. The action keys are the random draws the central loop takes for the module; each key has exactly one owning module. The calibration column is the default module's card calibration field.

| Slot | Required | Default | Other versions | Off version | Action keys | Calibration |
|---|---|---|---|---|---|---|
| `engine.fouls` | no | `fouls@1` | none | `off` | Tackle, FoulCard | yellow_cards_per_team |
| `engine.offside` | no | `offside@1` | none | `off` | none | none: no offside band in realism-bands.json |
| `engine.shot` | no | `shot@1` | none | `off` | Save, ShootoutSave | goals_per_xg |
| `engine.fatigue` | no | `fatigue@1` | none | `off` | InjuryMinute, InjuryTackle | none: no fatigue or injury band in realism-bands.json |
| `engine.steering` | yes | `steering@1` | none | none | none | none: movement maths, no realism band |
| `engine.pre-match` | no | `pre-match@1` | none | `off` | none | none: lineup choice, no realism band |
| `engine.modifier.fatigue` | no | `fatigue-curve@1` | none | `off` | none | none: no fatigue band in realism-bands.json |
| `engine.modifier.pressure` | no | `pressure@1` | none | `off` | none | none: no-op stand-in, no effect until its own piece |
| `engine.modifier.momentum` | no | `momentum@1` | none | `off` | none | none: no-op stand-in, no effect until its own piece |
| `engine.modifier.weather` | no | `weather@1` | none | `off` | none | none: no-op stand-in, no effect until its own piece |
| `engine.clock` | yes | `clock@1` | none | none | AddedTime, ExtraTimeAdded, ExtraKickOff, ShootoutFirstTeam, ShootoutEnd, KeeperDive, ShootoutSaveHold | none: no added-time or shoot-out band in realism-bands.json |
| `engine.restarts` | yes | `restarts@1` | none | none | none | none: restart placement and timing follow the Laws; no band measures them |
| `engine.discipline` | no | `discipline@1` | none | `off` | none | sending_off_share |
| `engine.injuries` | no | `injuries@1` | none | `off` | none | none: no injury band in realism-bands.json |
| `engine.ball` | yes | `ball@1` | none | none | BlockDeflect, ParryAngle, ParryLoft, CrossAngle, CrossLoft | none: ball physics, no realism band |
| `engine.possession` | yes | `possession@1` | none | none | Block, SaveHold, ParrySide, CrossClear, CrossWide, KeeperCatch | possession_pct |
| `engine.decision` | yes | `decision@1` | none | none | ShotScore, PassScore, DribbleScore, HoldScore, ClearScore, PassAim, ClearWide, ClearAim, ShotSide, ShotAim, ShotSpread, ShotLoft | pass_accuracy_pct |
| `engine.manager` | no | `ai-manager@1` | none | `off` | none | none: no substitution or mentality band in realism-bands.json |
| `engine.changes` | no | `changes@1` | none | `off` | none | none: Law 3 change rules, no band |
| `engine.hook.decision` | no | `decision-hook@1` | none | `off` | none | none: plugin hook adapter, no realism band |
| `engine.hook.rule` | no | `rule-hook@1` | none | `off` | none | none: plugin hook adapter, no realism band |
| `engine.hook.commentary` | no | `commentary-hook@1` | none | `off` | none | none: plugin hook adapter, no realism band |
| `game.rules` | no | `rule-pack@1` | none | `off` | none | none: rule pack loader, no realism band |
| `game.world` | no | `world-stub@1` | none | `off` | none | none: stub slot, no behaviour yet |
| `game.season` | no | `season-stub@1` | none | `off` | none | none: stub slot, no behaviour yet |
| `game.people` | no | `people-stub@1` | none | `off` | none | none: stub slot, no behaviour yet |
| `game.presentation` | no | `presentation-stub@1` | none | `off` | none | none: stub slot, no behaviour yet |
| `viewer.skin` | no | `broadcast-blue@1` | `interim-light@1` | `off` | none | none: viewer skin, no realism band |
| `engine.fast-model` | no | `fitted-scores@1` | none | `off` | none | none: fitted to equal the full engine on its own check, not to a band |

The engine checks this table: a documentation test compares each row with the registry of the program.

## Module card

Every registered module, and every off version, carries a card.

| Field | Holds |
|---|---|
| `purpose` | What the module does, in one sentence. |
| `inputs` | What the module reads from the read-only match view. |
| `outputs` | What the module returns: results or proposed changes. |
| `tuning` | The tuning values the module reads, or the one entry `none`. |
| `calibration` | A band name from `content/realism-bands.json`, or `none: <reason>`. |
| `keys` | The action keys whose draws the loop takes for the module. |

The card check refuses a card when `purpose`, `inputs`, `outputs`, `calibration` or an entry of `tuning` is empty, when `tuning` has no entry, or when `calibration` is neither a band name nor `none:` followed by a reason.

The key-ownership check refuses a registry in which an action key has no owner or more than one owner.

## Refusals

The engine refuses to start when the slot file is wrong. The message names the slot, the bad value and the valid names, and the command exits with code 1. A `content.refused` signal line with `kind="slots"` goes to standard error first.

| Problem | Message |
|---|---|
| Unknown module | `slot configuration refused: slot engine.offside: module "ofside" is not registered; valid: offside@1, off` |
| Version not built | `slot configuration refused: slot engine.fouls: fouls version 2 is not built; valid: fouls@1, off` |
| Empty module name | `slot configuration refused: slot engine.fouls: the module name "" is empty; valid: fouls@1, off` |
| A required slot switched off | `slot configuration refused: slot engine.steering: off is not allowed: the slot is required; valid: steering@1` |
| Unknown skin | `slot configuration refused: slot viewer.skin: module "nope" is not registered; valid: broadcast-blue@1, interim-light@1, off` |
| Unknown fast model | `slot configuration refused: slot engine.fast-model: module "nope" is not registered; valid: fitted-scores@1, off` |

The engine also refuses a slot id that is not declared (`"<id>" is not a declared slot`), a declared slot that is missing from the file (`the slot is missing from slots.json`), a module with no version (`module "<name>" has no version`) and `off` with a version (`off takes no version, got <n>`).

## Modifier slots

The four `engine.modifier.*` slots hold modifiers. A modifier reads the match view and returns one factor for each effective value it may scale: `max_speed`, `max_accel`, `passing`, `finishing`, `decisions` and `composure`. A factor of 1.0 is no effect.

| Slot | Family |
|---|---|
| `engine.modifier.fatigue` | body |
| `engine.modifier.pressure` | mind |
| `engine.modifier.momentum` | mind |
| `engine.modifier.weather` | surroundings |

The fourth family, familiarity, has no modifier yet. The pressure, momentum and weather modifiers have no effect: each returns 1.0 for every value.

The engine combines the factors of one effective value in two steps:

1. Within a family, the factors multiply, from 1.0.
2. Across families, the soft combine skips the factors of exactly 1.0, splits the rest into the factors below 1.0 and the factors above 1.0, and sorts each group by distance from 1.0, largest first. The first factor of a group counts as it is. The factor in place `j` (counted from 0) counts as `f^w[j]`, with the weights 1.0, 0.5, 0.25 and 0.125.

One factor alone passes through unchanged, bit for bit. The engine refreshes the effective values every 50 ticks.

## Hook slots

The three `engine.hook.*` slots connect the plugin hooks of a script pack to the central loop. The module of a hook slot is an adapter: it builds what the hook sees and reads only the view. The loop keeps the hooks, their failure counts and the watchdog mark. The off version of a hook slot makes the loop consult no hook at that point, so the engine's own option scores, card or commentary line stand. A hook-slot card owns no action key. The Rhai adapter between a script pack and the hooks carries its own card, `RHAI_ADAPTER_CARD` in the `script` crate.

## The central loop

Only the central loop writes match state. The loop files are `sim.rs`, the `sim/` folder, `rules/mod.rs`, `tactics/change.rs` and `modules/proposal.rs` under `crates/engine/src/`. A source scan (`crates/engine/tests/structure.rs`) fails when any other engine file has a `&mut self` method on `Simulation` or takes a `&mut Simulation`; the snapshot reader, the scenario builder and the gate's test harness are the named exceptions. A module's proposed changes are applied in `Simulation::apply_proposal`, the one writer of module output.

## Game slots

`game.rules` loads the rule pack that a match plays under. Its default, `rule-pack@1`, reads the content folder's rule file. Its off version loads the standard Laws built into the program.

`game.world`, `game.season`, `game.people` and `game.presentation` are stub slots. Each declares one entry point, `on_day(GameDay) -> GameChanges`, a default module that proposes no change, an off version and a card. Nothing in the engine calls them yet. A stub reads no match view, draws nothing and owns no action key.

## Viewer skin

`viewer.skin` names the look the viewer loads. `broadcast-blue@1` is the default; `interim-light@1` is the test skin; `off` shows the viewer's built-in default look. The engine never reads a skin, so a skin cannot change a match.

## Fast model

`engine.fast-model` holds a match model fitted from full-engine results: it gives a final score and an event stream that keeps the event-stream rules, with the players of both line-ups and benches. Only the `engine-cli fast-model` command reaches it; no match that the engine plays or serves uses it. Its off version refuses to play. The fit file is `content/fast-model.json` (see [the data-file reference](data-files.md#fast-modeljson)).

A fit records the engine id of the golden results it came from. `engine-cli fast-model stale` exits with code 1 when that id differs from the current golden results:

```text
error: the fast-model fit is stale: the fit records golden-2-9b887ee-000000000000, the golden results are golden-3-ee6508d-c896aba0e61b; run engine-cli fast-model fit
```

## Contract checks

The engine's tests hold three checks on every slot:

| Check | What it proves |
|---|---|
| Off switch | Each optional slot switched off in turn, and all optional slots off at once, plays a batch of matches that the event validator accepts. |
| Swap | A stand-in module in each slot in turn, which forwards every call to the default module, keeps every other module's draws and the events unchanged. A control run with a changed module must find a difference. |
| Ownership and cards | Every action key has one owner with every stand-in in place, and every card is complete. |

The stand-ins exist only in test builds. A release program never contains them.

## Related

- [The match stream protocol](protocol.md): the skip, advice and matchday messages.
- [The data-file reference](data-files.md): the ground in a team file and the snapshot format.
- [The command-line reference](cli.md): `engine-cli gate`, `engine-cli guard` and `engine-cli fast-model`.
