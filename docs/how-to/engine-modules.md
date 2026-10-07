# Write and register an engine module

This guide shows how to add a new module version to a slot of the match engine: write the module, fill its card, register it, select it in a slot file, and run the checks. The example adds `still-air@1`, a weather modifier with no effect, to the optional slot `engine.modifier.weather`. Every slot, card field and refusal message is in [the module reference](../reference/engine-modules.md).

You need a checkout of the repository and the pinned Rust toolchain (see `CONTRIBUTING.md`). Run every command from the repository root.

## Write the module

A module implements the trait of its slot. It reads the match only through the read-only view (`MatchView`) and returns a result or proposed changes; it never writes match state. A modifier implements `Modifier`: it names its family and returns one factor for each effective value.

1. Create `crates/engine/src/modules/modifier/still_air.rs`:

   ```rust
   //! Still air: a weather modifier with no effect.

   use super::{Effect, Family, Modifier};
   use crate::modules::{MatchView, ModuleCard};

   /// Version 1 of still air: calm weather, so no player is slowed or hurried.
   pub struct StillAirV1;

   impl Modifier for StillAirV1 {
       fn family(&self) -> Family {
           Family::Surroundings
       }

       fn effect(&self, _view: &MatchView<'_>, _i: usize) -> Effect {
           Effect::NEUTRAL
       }
   }
   ```

2. Add the module to `crates/engine/src/modules/modifier/mod.rs`, after `pub mod stand_ins;`:

   ```rust
   pub mod still_air;
   ```

## Fill the card

Every module carries a card. Add it to `still_air.rs`:

```rust
pub const STILL_AIR_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Still air around a player (surroundings family): calm weather with no effect.",
    inputs: "Nothing.",
    outputs: "A factor of 1.0 on every effective value.",
    tuning: &["none"],
    calibration: "none: no effect",
    keys: &[],
};
```

- `calibration` names a band or band group in `content/realism-bands.json`, or reads `none:` and a reason.
- `keys` lists the action keys whose random draws the loop takes for this module. A module that draws nothing owns no key. A key that you add must not belong to another module: the ownership check fails when a key has two owners.

## Register the module

Registration gives the module a name and a version in its slot. In `crates/engine/src/modules/registry.rs`:

1. Import the module beside the stand-ins:

   ```rust
   use super::modifier::{Modifier, stand_ins, still_air};
   ```

2. In the `SlotDecl` of `MODIFIER_WEATHER`, add a second registration after `weather`. Keep the default first: the first registration is the slot's default.

   ```rust
   registrations: &[
       Registration {
           name: "weather",
           version: 1,
           module: ModuleRef::Modifier(&stand_ins::WEATHER_V1),
           card: &stand_ins::WEATHER_V1_CARD,
       },
       Registration {
           name: "still-air",
           version: 1,
           module: ModuleRef::Modifier(&still_air::StillAirV1),
           card: &still_air::STILL_AIR_V1_CARD,
       },
   ],
   ```

3. The test `every_modifier_slot_names_its_family` in `crates/engine/tests/modules.rs` counts the modifier registrations, off versions included. Raise the count from `8` to `9`.

4. Add the new version to the slot table of [the module reference](../reference/engine-modules.md): `still-air@1` in the "Other versions" column of `engine.modifier.weather`. A documentation test compares that table with the registry.

## Select the module

Do not edit the shipped `content/slots.json`. Three tests hold it equal to the built-in default selection, so the replay gate hashes stay valid. Select the module in a copy of the content folder instead:

1. Copy the folder:

   ```bash
   cp -r content my-content
   ```

2. In `my-content/slots.json`, change the weather entry:

   ```json
   "engine.modifier.weather": { "module": "still-air", "version": 1 },
   ```

3. Build the program and play a short match with the copy:

   ```bash
   cargo build --release -p engine-cli
   target/release/engine-cli simulate --seed 1 --minutes 5 --ticks-out check.ticks --content-dir my-content
   ```

   The engine resolves the slot file when it starts. A `content.loaded` line with `kind="slots"` names the file. A wrong entry stops the start with a message that names the slot, the bad value and the valid names, for example `slot engine.modifier.weather: module "stil-air" is not registered; valid: weather@1, still-air@1, off`.

## Run the checks

1. Run the module tests. They resolve the default selection, check every card and the key ownership, and play matches with each modifier switched off:

   ```bash
   cargo test -p engine --test modules
   ```

   All 25 tests pass.

2. Run the contract checks. They switch every optional slot off in turn and all at once, and put a stand-in module in every slot in turn:

   ```bash
   cargo test -p engine --test contract_checks
   ```

   All 5 tests pass.

3. Run the replay gate with the shipped content folder. A new registration must not change a match that does not select it:

   ```bash
   target/release/engine-cli gate
   ```

   The gate ends with `gate: 22 matches for windows-x86_64 in 44.7 s; none differ` (the time and the machine set differ on your computer).

4. Run the gate with your copy only to see what a selection changes:

   ```bash
   target/release/engine-cli gate --content-dir my-content
   ```

   The gate reports `some differ` and exits with code 2, even for a module with no effect. A selection other than the default is part of the content hash, and the gate compares a hash of the whole match state, which includes it. The tick counts stay the same as the shipped ones: for still air, every match plays the same ticks.

5. To compare the match itself, play the same seed with both folders and compare the run reports:

   ```bash
   target/release/engine-cli simulate --seed 1 --ticks-out a.ticks
   target/release/engine-cli simulate --seed 1 --ticks-out b.ticks --content-dir my-content
   ```

   For still air, only `content.hash`, `match.id` and the timings differ, and the two tick files differ only in their headers.

## Switch the slot off

An optional slot also takes `off`. In `my-content/slots.json`:

```json
"engine.modifier.weather": { "module": "off" },
```

`off` takes no version. A required slot refuses `off`. The off-switch check in `contract_checks` already plays every optional slot switched off.

## When the module changes results

A module that changes what happens in a match changes the gate hashes. Its selection becomes the default only with a regenerated golden file and one new ledger entry; see [Replay gate](replay-gate.md).
