# The plugin contract

This page explains why every part of the match engine is a module behind one contract, what the contract promises, and what it leaves for later. It is for people who maintain the engine. For the slots, the cards and the messages, see [the module reference](../reference/engine-modules.md). For the steps to add a module, see [Write and register an engine module](../how-to/engine-modules.md). For the engine itself, see [How the engine works](engine.md).

## Why one contract

The engine grew as one loop that called each part directly: fouls, offside, shots, fatigue, the ball, possession, the decision maker, the referee's clock, the AI manager. Three plugin hooks let script packs change decisions, cards and commentary, each with its own way in. To replace one part, a maintainer had to know how the loop reached it, which random draws it took and in what order, and which other parts read what it wrote.

The contract gives every part the same shape. A part is a module that fills a named slot. The slot declares the module's interface and says whether the slot is required. One configuration file, `slots.json`, picks the module and the version for every slot. The engine resolves the file when it starts and refuses a wrong file with a message that names the slot, the bad value and the valid names. A maintainer replaces a part by registering a new version and selecting it; nothing else in the loop changes.

The same shape serves parts outside the match engine: the rule pack, four declared game-wide slots, the viewer's look and a fitted results model. A later system, such as the season layer, gets a slot that already exists instead of a new kind of plug.

## Three tiers

The contract has three tiers, and this work builds only the first.

1. **Compiled modules.** A module is Rust code compiled into the program and registered in the engine's registry. Every slot today is in this tier. The compiler checks the interface, and a module costs no more than a direct call.
2. **Sandboxed modules for modders.** A later tier would load modules from outside the program in a sandbox, for example WebAssembly. Nothing in this tier is built: no slot accepts an outside module. The Rhai script packs reach the engine through the three hook slots, whose adapters are compiled modules; a pack itself is not a module.
3. **Outside tools.** A later tier would let other programs take part over the match protocol. Nothing in this tier is built either.

Building only the compiled tier keeps the cost and the risk small. The slots, cards and checks that the other tiers need are in place, so a later tier adds a way to load a module, not a new contract.

## A read-only view, and proposals

A module reads the match through `MatchView`, a borrowed view whose one field is private. Every accessor returns a copy or a shared reference, so a module cannot write match state: the compiler refuses the attempt, and a compile-fail test holds that case.

A module returns a result or a proposed change. Only the central loop applies a change. This gives each tick one place where state is written, in a fixed order. Two modules can never race to write the same field, and the order of writes does not depend on the order in which modules were registered.

The loop also takes every random draw. A module that needs randomness says which draws it needs, and the loop takes them on the module's own keys, in the order the engine always took them. A module that only reads stays a pure function of the view.

## Why the random stream ids stay

Every random draw has a key: the part of the engine, the kind of action and the player. Each key has its own stream from the match seed. The contract keeps every key exactly as it was, and gives each key to one module, which lists it on its card.

Keeping the keys is what let the engine move onto the contract without changing a single match. Every refactor step left the replay gate hashes unchanged, so the gate proved that each move was a move and nothing else. New keys per module would have changed every match at once and hidden any real mistake inside that change.

Owning keys gives a second promise: a module that takes more or fewer draws changes only its own streams. The swap check proves it. A stand-in module in each slot in turn keeps every other module's draws and the events unchanged, and a control run with a changed module must find a difference.

## Required slots, off versions, and cards

The core of a match cannot be switched off: ball physics, movement and steering, possession, the clock and the end of the match, restarts, and the decision maker. These slots are required. Every other slot is optional and has an off version, so a maintainer can switch a part off to see what it contributes. The off-switch check plays a batch of matches with each optional slot off, and with all of them off at once, and the event validator accepts every match.

Every module carries a card: its purpose, what it reads, what it returns, the tuning it reads, its calibration and its keys. The calibration names a realism band that the part is measured against, or says `none:` and why. A card is the one place a reader learns what a module is without reading its code, so the card check refuses an empty field.

## Modifiers

Effects on a player, such as fatigue, are modifiers. Each modifier belongs to a family (body, mind, familiarity, surroundings) and returns a factor for each effective value. Factors multiply within a family. Across families a soft combine makes each further effect in one direction count for less than the one before, so four small effects do not stack into an absurd one. Fatigue is the one modifier with an effect today; pressure, momentum and weather are stand-ins with none, waiting for their own work.

## Stubs

The world, the season systems, the people and their minds, and what the player sees have no code yet. Each has a declared slot with one entry point, a default module that proposes no change, an off version and a card. Nothing calls them. Declaring them now fixes their place in the contract, so each later system fills a slot instead of inventing a new way in.

## The skin slot

The viewer's look is a slot too: `viewer.skin` names the folder of tokens, styles and fonts the viewer loads. The engine never reads a skin, so a look can never change a match, and the `viewer.*` slots stay out of the content hash. The second skin, the interim light look, exists to prove that a swap needs only configuration, not code.

## Results that must not change

A module selection other than the default is folded into the content hash, and a replay file carries the slot file as one of its inputs. A match played with another selection is therefore never taken for a default match, and the golden results stay a statement about the default engine. Only a change that is meant to change results may regenerate them, with one ledger entry that names the reason.

The fitted results model in `engine.fast-model` follows the same rule from the other side. Its fit records the identity of the golden results it was fitted from, and a check refuses a fit whose identity is stale. No match that the game plays uses it yet.

## The previous engine

A saved match records the release version and the build of the engine that played it. When a release changes results, a match saved on the previous release cannot simply continue on the new engine: the rest of the match would differ from what that engine would have played.

So each release ships the previous release's engine program beside its own, in `previous/`, with the content it played with. The game picks the engine by the version in the save. A save from the previous release continues on the previous engine and gives the previous build's result. A save from two or more releases back is refused with a message that names its version. Shipping exactly one previous engine keeps the package small and the promise simple.

## Related

- [The module reference](../reference/engine-modules.md): every slot, card field and refusal.
- [Write and register an engine module](../how-to/engine-modules.md): a module from start to finish.
- [How the engine works](engine.md): the tick loop, the snapshot, the random streams and the replay gate.
