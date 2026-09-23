# Script packs

A script pack changes how players decide, how the referee cards a foul, and what the
commentator says, with a short script in [Rhai](https://rhai.rs/book/). The engine runs the
script in a sandbox. A script that fails never stops a match: the engine keeps its own
choice and records the failure.

Run a match with a pack:

```
engine-cli simulate --seed 42 --ticks-out match.ticks --script-pack content/scripts/sample
```

`--script-pack <DIR>` works on `simulate`, `bench`, `serve`, `record`, and `resume`. No pack
runs unless the flag names one. A match without a pack is exactly the match it was before
packs existed.

## The folder

A pack is a folder with two files: `pack.json` and the script it names. The sample pack in
`sample/` uses every hook.

### pack.json

| Field | Type | Bound | Meaning |
|---|---|---|---|
| `schema_version` | integer | 1 | the format of this file |
| `id` | text | 1 to 40 characters: lower-case letters, digits, hyphens | the pack's name in records |
| `version` | text | 1 to 20 characters, no `+`, `@`, or space | the pack's own version |
| `name` | text | 1 to 80 characters | a title for people |
| `plugin_api` | integer | 1 | the plugin interface the script was written for |
| `entry` | text | a `.rhai` file name in the folder, with no path separator or `..` | the script |
| `hooks` | list | 1 to 3 of `decision`, `rule`, `commentary`, each once | the hooks the script defines |
| `decision.refresh_ticks` | integer | 5 to 250, default 25 | ticks the decision offsets stay in force |
| `limits.max_operations` | integer | 1,000 to 50,000, default 10,000 | script operations one call may run |

Any other field is refused. A refusal names the file, the field, and the reason, for example
`content refused: script pack content/scripts/mine/pack.json: decision.refresh_ticks: lower
than 5`. The pack is also refused when the script does not compile, uses `eval`, fails in its
top-level statements, or does not define the function of a hook it lists.

The pack identity is `id@version+hash`, where `hash` is the first 12 hex characters of the
SHA-256 of `pack.json` followed by the script. Records name the pack by this identity. The
hash is also part of the match's content hash, so a snapshot written with a pack resumes only
with the same pack, byte for byte.

## The hooks

Each hook is one script function. Every call starts fresh: a script keeps no state from one
call to the next, and it has no random function, so the same seed and the same pack play the
same match.

### decision: `fn decide(ctx)`

Adds offsets to the ball carrier's option scores. Return a map with any of the keys `pass`,
`dribble`, `shoot`, `clear`, and `hold`, each a number from -10 to 10. A missing key is 0.
Return `()` to keep the engine's scores. An offset of 10 is enough to force an option; an
option the carrier does not have (a shot from beyond the shooting range) stays unavailable.

The engine asks for new offsets when a new player has the ball, after every stoppage, and
every `refresh_ticks` ticks. In between, the last offsets apply.

| `ctx` field | Meaning |
|---|---|
| `tick`, `minute` | the tick, and the minute of play from 0 |
| `team` | 0 for home, 1 for away |
| `slot` | the carrier's formation slot; 0 is the goalkeeper |
| `goals_for`, `goals_against`, `goal_diff` | the score from the carrier's side |
| `goal_distance` | metres to the centre of the goal the carrier attacks |
| `nearest_opponent` | metres to the nearest opponent |
| `progress` | -1 at the carrier's own goal line to 1 at the opponent's |

### rule: `fn card(ctx, native)`

Reviews the referee's card for a foul. `native` is `"none"`, `"yellow"`, `"second-yellow"`, or
`"red"`. Return `"none"`, `"yellow"`, or `"red"`, or `()` to keep the referee's card. A yellow
for a player already booked is a second yellow, which sends the player off.

| `ctx` field | Meaning |
|---|---|
| `tick`, `minute`, `team`, `slot` | as for `decide`, for the offender |
| `yellows` | yellow cards the offender had before this foul |
| `aggression` | the offender's aggression, 0 to 1 |
| `advantage` | `true` when play goes on and the card waits for the next stoppage |
| `penalty` | `true` when the foul gives a penalty |

### commentary: `fn line(ctx, native)`

Rewrites a commentary line. `native` is the commentator's line. Return a line of 1 to 280
characters, or `()` to keep it.

| `ctx` field | Meaning |
|---|---|
| `tick`, `minute` | as for `decide` |
| `kind` | the event type, such as `goal`, `card`, or `corner` |
| `team` | 0 for home, 1 for away, -1 for none |
| `home_score`, `away_score` | the score after the event |

## The sandbox

- **Operations:** a call may run `limits.max_operations` script operations. The count is the
  same on every machine, so a pack that fits the budget fits it everywhere.
- **Time:** a call that runs longer than 2 ms of wall-clock time is stopped. This only catches
  a slow built-in function; the operation budget stops a loop first.
- **Sizes:** strings of at most 1,024 characters, arrays of 256 items, maps of 64 entries, 16
  nested calls, and expressions nested 64 deep (32 inside a function).
- **No files and no network:** `import` is refused, and Rhai has no file or network functions.
  A call to any function the sandbox does not provide is refused and named.
- **No `eval`:** a pack that uses it is refused when it loads.
- **`print` and `debug`** go to the engine's log, never to its output.

## When a hook fails

A call that runs out of budget or fails is **aborted**. A call that tries an import or a
function the sandbox does not provide is **denied**. Either way, the engine keeps its own
choice for that call (zero offsets, the referee's card, or the commentator's line), writes a
`script` event with `script.hook`, `script.outcome`, and `script.detail` to `events.jsonl`,
and logs a warning (`script.aborted` or `script.denied`). A hook that fails three times in a
row is **disabled** for the rest of the match, with one more `script` event.

The first `kick-off` event carries `script.pack`. The `match-stats` record carries
`script.pack`, `script.calls`, `script.aborts`, `script.denials`, and `script.disabled`. A
resumed match counts calls from the resume tick, because the counters are not in the
snapshot. `bench` names the pack in the run report as `script.pack`.

A decision hook runs once per refresh, not once per tick, so a pack costs little: one call of
the sample pack's decision hook takes about a microsecond.
