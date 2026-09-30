# Match stream protocol, version 3

The engine serves one viewer over a WebSocket on `127.0.0.1`. The port is chosen by the
operating system at every run and written to `engine.port` inside the runtime data folder.
A page connects to `ws://127.0.0.1:<port>/?v=3`.

Version 2 changed the meaning of `ticks_expected` from the exact tick count to the most ticks
the match can last, because the time added at the end of each half is known only when the
half ends. The event message gained the law event types and six optional fields. Version 2
later gained the `injury`, `substitution`, and `ai-decision` event types and the optional
fields `change.applied_tick` and `ai.decision`, and change events now carry `team.id`. No
field changed meaning.

Version 3 changed the meaning of `stats` from one closing total to a running total sent once
every simulated second and again at full time, and gave it the nine figures the match screen
shows. A new message, `condition`, carries every player's energy on the same cadence. Each
`hello` team gained a `roster`. A fixture recorded by an earlier build is refused by version.

Version 3 also carries the pre-match lineup and live changes. A `serve` session holds before
kick-off and sends nothing but the `hello` until the page sends `start`. A new command,
`set-lineup`, picks the home lineup, bench, and pre-match tactics during that hold. The
`hello` gained the home team's `squad` and `setup`, the loaded `tactics` file, and the
`substitutions` limits. `queue-change` changed meaning: its `detail` is now read, and the
change reaches the engine and applies at the next stoppage that admits it.

Version 3 also carries knockout matches (`--knockout`) without a new version. The event
message gained five optional fields (`period`, `shootout.round`, `shootout.scored`,
`shootout.scores`, and `result.decided_by`), no event type was added, and no field changed
meaning for a match that is not a knockout match. For a knockout match, `ticks_expected` also
covers extra time and the rule pack's allowance of shoot-out rounds; a sudden death longer
than the allowance plays on past it, and a client grows its history to hold it.

Version 3 also carries script packs (`--script-pack`) without a new version. The event message
gained one event type (`script`) and four optional fields (`script.pack`, `script.hook`,
`script.outcome`, and `script.detail`). A match without a pack sends exactly what it sent
before, and no field changed meaning. The page ignores the `script` event type.

A test in `crates/protocol/tests/document.rs` holds this document to the code: every message
the implementation names must appear below with every one of its fields.

## Connecting

| Step | Rule |
|---|---|
| Address | `ws://127.0.0.1:<port>/?v=<protocol version>` |
| Version | `v` must equal `3`. Any other value, or no value, is refused with both versions named. |
| Origin | Any port of `http://localhost` or `http://127.0.0.1`, or no `Origin` header at all. Any other origin is refused, including `null` (a sandboxed frame or a `data:` document) and a `file://` page. |
| Clients | One viewer per match. |
| First message | `hello`, always before the first tick frame. |
| Kick-off | `serve` holds after the `hello`: no tick, event, or statistics message is sent until the client sends `start`. `record`, `replay`, and `bench` do not hold. |

A refused handshake answers `403` for an origin and `400` for a version, with the reason as
the body, and writes one `socket.refused` line naming the origin and the rule.

## Framing

Control messages are JSON text frames tagged by `type`. Tick positions are binary frames.

| Frame | Bytes | Meaning |
|---|---|---|
| `0x01` keyframe | 99 | the kind tag, then 98 payload bytes: tick (4), ball (6), 22 players (88) |
| `0x02` delta | 48 | the kind tag, then 47 payload bytes: ball (3), 22 players (44) |
| `0x03` restart keyframe | 99 | the same payload as `0x01`, on the tick the ball is placed for a kick-off, a throw-in, a corner, a goal kick, a free kick, or a penalty |

Every position is a signed 16-bit count of centimetres, little-endian. A delta carries one
signed byte per component, which covers 1.27 m of movement in one tick against a ball cap of
0.80 m. A step wider than that, a restart, and every `keyframe_interval` ticks each force a
keyframe. A delta carries no tick number: it is the tick after the frame before it.

## Messages the server sends

### hello

| Field | Type | Meaning |
|---|---|---|
| `protocol.version` | integer | always 3 in this build |
| `engine.version` | string | the engine crate version |
| `build.hash` | string | the git hash the engine was built from |
| `owner.id` | string | 32 hex characters, created once per machine |
| `match.id` | string | `<seed as 16 hex>-<start time in milliseconds>` |
| `seed` | integer | the seed the match runs from |
| `dt_ms` | float | milliseconds per tick, 20.0 |
| `ticks_expected` | integer | the most ticks the match can send: regulation time plus the cap on added time in every half; the match ends earlier when it earns less added time, and the `full-time` event marks the last tick. A knockout match adds both extra-time periods at their cap on added time and the rule pack's allowance of shoot-out rounds (530,000 ticks for 90 minutes with the shipped rule pack); a sudden death longer than the allowance runs past it |
| `keyframe_interval` | integer | ticks between keyframes |
| `teams` | array of two | one entry per club, home first; see the table below |
| `tactics` | object | the tactics file the engine loaded (`content/tactics.json`): `formations`, `mentalities`, `instructions` with their levels, `roles`, `duties`, and the computer manager's settings. Every tactics index on the wire is a place in one of its lists. Because a JSON object carries no key order, the engine adds `instruction_order`, the six instruction names in the order an instruction list indexes them |
| `substitutions` | object | the rule pack's limits: `limit`, `windows`, `extra_substitutions`, `extra_windows`, and `windows_exempt` |

`substitutions`:

| Field | Type | Meaning |
|---|---|---|
| `limit` | integer | substitutions each team may make, 5 in the shipped rule pack |
| `windows` | integer | stoppages at which each team may make them, 3 in the shipped rule pack; half-time uses none |
| `extra_substitutions` | integer | substitutions each team gains once extra time starts, 1 in the shipped rule pack; 0 when an earlier build sent none |
| `extra_windows` | integer | windows each team gains once extra time starts, 1 in the shipped rule pack; 0 when an earlier build sent none |
| `windows_exempt` | array of strings | the stoppage kinds whose substitutions use no window, as the rule pack writes them (`half_time` in the shipped pack); left out when empty |

Each entry of `teams`:

| Field | Type | Meaning |
|---|---|---|
| `team.id` | string | the club identifier from the team file |
| `team.name` | string | the club name |
| `team.kit.primary` | string | lower-case `#rrggbb`, the shirt colour |
| `team.kit.secondary` | string | lower-case `#rrggbb`, the trim colour |
| `roster` | array | the 11 starters in wire-slot order (home slots 0 to 10, away 11 to 21), then the named bench in bench order; see the table below. A hello without it reads as an empty list |

Each entry of `roster`:

| Field | Type | Meaning |
|---|---|---|
| `player.id` | string | the player identifier from the team file, as events name it |
| `player.name` | string | the display name |
| `player.shirt` | integer | the shirt number |
| `player.position` | string | the position code, for example `GK`, `CB`, or `ST` |
| `player.squad_index` | integer | the player's place in the team file's squad, counted from 0 |

The home team also carries two fields when the session takes its lineup from the page
(`serve` only). Both are absent otherwise.

| Field | Type | Meaning |
|---|---|---|
| `squad` | array | every player in the team file, in file order, so an entry's place in the list is its squad index; see the table below |
| `setup` | object | the computer manager's pre-match choice, which the lineup editor starts from; see the table below |

Each entry of `squad` carries `player.id`, `player.name`, `player.shirt`, and
`player.position` as a roster entry does, and:

| Field | Type | Meaning |
|---|---|---|
| `player.natural_fitness` | integer | the natural-fitness attribute, 0 to 100; every player is fresh before kick-off, so this is the fitness figure the editor shows |
| `player.injury_resistance` | integer | the injury-resistance attribute, 0 to 100; 0 when an earlier build sent none |
| `role_fit` | array of integers | how well the player fits each role, 0 to 100, one value per entry of `tactics.roles`, in that order |

`setup`:

| Field | Type | Meaning |
|---|---|---|
| `lineup` | array of 11 integers | squad indices in slot order; slot 0 is the goalkeeper |
| `bench` | array of integers | squad indices of the named substitutes, at most `tactics.ai.bench_size` |
| `formation` | integer | an index into `tactics.formations` |
| `mentality` | integer | an index into `tactics.mentalities` |
| `instructions` | array of 6 integers | one level index per instruction: pressing, width, tempo, line height, passing directness, time wasting |
| `roles` | array of 11 objects | one `{ role, duty }` per slot, in slot order |
| `role` | integer | an index into `tactics.roles` |
| `duty` | integer | an index into `tactics.duties` |

A viewer draws its markers from the two kit colours. It must not use either colour as it
arrives: a team file may hold pure black or pure white, so a viewer pulls the lightness of
every kit colour into a safe range and tests the result for contrast against the pitch.

### tick

Binary, not JSON. See **Framing**.

| Field | Type | Meaning |
|---|---|---|
| `kind` | byte | `0x01`, `0x02`, or `0x03` |
| `tick` | integer | keyframes only; a delta is the tick after the one before it |
| `ball` | 3 values | x, y, and height in centimetres |
| `players` | 22 pairs | x and y in centimetres, in roster order, home team first |

### event

One `match-event` row. The same object is written to
`matches/<match.id>/events.jsonl` with the observability envelope added.

| Field | Type | Meaning |
|---|---|---|
| `owner.id` | string | the owner of the match |
| `match.id` | string | the match |
| `tick` | integer | the tick the event happened on |
| `minute` | integer | the minute of play the match clock shows, counted from 0; it stays at the end of the half during added time |
| `event.type` | enumeration | `kick-off`, `goal`, `half-time`, `full-time`, `tactics-change`, `offside`, `foul`, `card`, `throw-in`, `corner`, `goal-kick`, `free-kick`, `penalty`, `injury`, `substitution`, `ai-decision`, or `script` |
| `team.id` | string | the club the event belongs to: the offender's club on `offside`, `foul`, and `card`, the club that restarts play on a restart, the injured player's club on `injury`, the club that changes on `substitution`, `ai-decision`, and `tactics-change`; absent at half-time and full time |
| `home.score` | integer | the score after the event |
| `away.score` | integer | the score after the event |
| `change.kind` | enumeration | `tactics` or `substitution`; change events only |
| `change.queued_tick` | integer | the tick the change was queued on |
| `change.queue_id` | string | the identifier the acknowledgement returned |
| `change.rejected_reason` | string | present only on a refused change |
| `change.state` | enumeration | `queued`, `applies-now`, `applied`, or `rejected` |
| `change.applied_tick` | integer | on an applied change: the tick it took effect on, which is the tick of the stoppage that admitted it |
| `ai.decision` | string | on `ai-decision`: `mentality-up-trailing`, `mentality-down-leading`, `sub-injury`, `sub-fatigue`, or `sub-keeper` (the bench keeper replaces an outfield player keeping goal) |
| `player.id` | string | the player the event names: the offender on `offside` and `foul`, the booked player on `card`, the injured player on `injury`, the player leaving on `substitution`, the taker on `kick-off` and on every restart, and on `goal` the player who kicked the ball last (a player of the other club on an own goal) |
| `player.secondary_id` | string | the fouled player, on `foul`; the player coming on, on `substitution` |
| `card.kind` | enumeration | `yellow`, `second-yellow`, or `red`, on `card`; a second yellow sends the player off |
| `foul.advantage` | boolean | on `foul`: `true` when play continued because the fouled team kept the ball; a card for that foul follows at the next stoppage |
| `minute.added` | integer | in added time only: the added minute, 2 at 45+2 |
| `added_time.s` | integer | on `half-time` and `full-time`: the seconds added to the half, or the extra-time period, that ended |
| `period` | integer | knockout matches only, on the `half-time` break before an extra-time period and on that period's `kick-off`: the period that starts, counted from 0 (2 and 3 are the two periods of extra time) |
| `shootout.round` | integer | on a shoot-out `penalty` event: the kicking club's round, from 1 |
| `shootout.scored` | boolean | on the second `penalty` event of a shoot-out kick, its outcome: `true` when the kick scored |
| `shootout.scores` | array of two | on the outcome event of a shoot-out kick and on `full-time` after a shoot-out: the shoot-out score, home first; `home.score` and `away.score` keep the score of play |
| `result.decided_by` | enumeration | knockout matches only, on `full-time`: `regulation`, `extra-time`, or `shoot-out` |
| `commentary` | string | one English commentary line, on every event except `tactics-change` and `script`; the lines come from `content/commentary/en.json` and name the player and the club, and a script pack's commentary hook can rewrite them |
| `script.pack` | string | on the first `kick-off` of a match that runs a script pack: the pack identity, `id@version+hash`, where the hash is the first 12 hex characters of the SHA-256 of `pack.json` followed by the script file |
| `script.hook` | enumeration | on `script`: the hook that failed, `decision`, `rule`, or `commentary` |
| `script.outcome` | enumeration | on `script`: `aborted` (the call ran out of its operation budget, or failed; a call past the 2 ms wall-clock limit is not aborted and only marks the match invalid in `match-stats`), `denied` (the call tried an import or a function the sandbox does not allow), or `disabled` (the hook failed three times in a row and is off for the rest of the match) |
| `script.detail` | string | on `script`: why, such as `operation budget of 10000 exhausted` or `function http_get is not available` |

A `script` event names no team or player and carries no `commentary`. Play goes on with the
engine's own choice: zero option offsets, the referee's card, or the commentator's line. For
example, a looping decision hook gives
`{"event.type":"script","script.hook":"decision","script.outcome":"aborted","script.detail":"operation budget of 10000 exhausted",...}`,
an import gives `"script.outcome":"denied","script.detail":"import ../../../Cargo is not allowed"`,
and the third failure in a row adds a second event with `"script.outcome":"disabled"`.

A knockout match that is level after regulation time plays two periods of extra time, each
opened by a `half-time` event and a `kick-off` that carry `period`. Still level, it goes to a
penalty shoot-out played on the pitch. Every kick is two `penalty` events: the set-up, with
`shootout.round`, on the restart keyframe that places the ball on the mark, and the outcome,
with `shootout.scored` and `shootout.scores`, when the ball goes in, is saved, leaves play,
stops, or has been live for 5 seconds. A shoot-out event carries no `commentary`. The minute
stays at the last minute of play (120 after extra time) for the whole shoot-out, and
`full-time` carries `result.decided_by`.

A restart event (`kick-off`, `throw-in`, `corner`, `goal-kick`, `free-kick`, `penalty`)
arrives on the same tick as the restart keyframe that places the ball. Play resumes when the
taker plays the ball, a few seconds later. An `injury` in open play stops play for a dropped
ball at the ball; an injury on a tick that already stopped play rides that stoppage.

The engine applies queued changes (the page's and the AI manager's) on the tick that opens a
stoppage the rule pack admits them at, substitutions first. Each verdict is a
`tactics-change` event with `change.state` `applied` or `rejected`. A rejection names its
reason in `change.rejected_reason`:

- `substitution limit reached (5 of 5)`; in extra time the limit rises by the rule pack's
  extra-time allowance (6 of 6 with the shipped pack)
- `no substitution window left (3 of 3)`; a half-time substitution uses no window, and neither
  does one at the breaks before and inside extra time, which are `half-time` stoppages
- `player <id> is not on the pitch`
- `player <id> was sent off and cannot be replaced`
- `player <id> is not on the bench`
- `player <id> left the pitch at this stoppage; the substitution applies first`: a tactics
  change that names a player substituted off at the same stoppage
- `the change names a formation, mentality, level, role, or duty the tactics file does not hold`

### stats

Sent once every simulated second (every 50 ticks at 20 ms per tick), after that tick's
events, and once more at full time. Every value is the running total up to `tick`. Every pair
is home first. The rounding equals the `match-stats` record's, so the last message and the
record agree. A viewer shows each message when playback reaches its `tick`, not when it
arrives.

| Field | Type | Meaning |
|---|---|---|
| `tick` | integer | the tick the totals were taken on |
| `minute` | integer | the simulated minute |
| `home.score` | integer | the score |
| `away.score` | integer | the score |
| `possession.changes` | integer | times possession changed hands |
| `ball.max_speed` | float | metres per second |
| `ball.idle_ticks` | integer | ticks the ball stood still |
| `stats.possession_pct` | two floats | each team's share of open-play ticks, one decimal |
| `stats.shots` | two integers | shots taken |
| `stats.shots_on_target` | two integers | shots whose flight, as struck, crosses the goal line between the posts and under the bar |
| `stats.xg` | two floats | expected goals, two decimals |
| `stats.passes` | two integers | open-play passes played; a clearance and a restart kick are not passes |
| `stats.pass_accuracy_pct` | two floats | completed passes over passes played, one decimal; 0 for none |
| `stats.fouls` | two integers | fouls committed |
| `stats.corners` | two integers | corners taken |
| `stats.offsides` | two integers | offsides given against the team |

### condition

Sent right after each periodic `stats` message, not at full time.

| Field | Type | Meaning |
|---|---|---|
| `tick` | integer | the tick the values were taken on |
| `energy` | array of 22 floats | each wire slot's energy, home slots first, from 0.0 (spent) to 1.0 (fresh), three decimals. A substitute takes the slot of the player who left |
| `subs_used` | array of two integers | substitutions each team has made, home first; `[0, 0]` when an earlier build sent none |
| `windows_used` | array of two integers | substitution windows each team has used, home first; `[0, 0]` when an earlier build sent none |

### change-state

Sent on `serve` for a change the page queued, on the tick a stoppage that takes the change's
kind opens, before the change's verdict event. It is not a match event: no events file, record,
or replay keeps it, and it never changes the match.

| Field | Type | Meaning |
|---|---|---|
| `change.queue_id` | string | the identifier the change's `queue-change` acknowledgement gave |
| `state` | enumeration | `applies-now` in this build |
| `tick` | integer | the tick the stoppage opened on, the tick of the verdict that follows |

### ack

| Field | Type | Meaning |
|---|---|---|
| `command` | string | `start`, `pause`, `set-speed`, `queue-change`, `set-lineup`, `seen`, or `cancel-change` |
| `change.queue_id` | string | queued and withdrawn changes only |
| `change.queued_tick` | integer | the tick the command was read on |
| `state` | enumeration | queued changes only; `queued` in this build |
| `speed` | float | `set-speed` only, after clamping to 0.25 to 8.0 |

### reject

| Field | Type | Meaning |
|---|---|---|
| `command` | string | the command that was refused |
| `reason` | string | words a viewer can show, for example `unknown change type formation` |

## Messages a client sends

### start

Starts or resumes production. No fields. The first `start` of a `serve` session is the
kick-off: the engine builds the match from the lineup `set-lineup` stored, or from the
computer manager's setup when the page sent none. On `record`, `replay`, and `bench` a client
that never sends it still receives the whole match.

### pause

Pauses production at the current tick. No fields. A viewer may pause and start the engine to
keep it close to the tick it is showing, so that a change it queues applies at a stoppage
the manager has not yet watched.

### set-speed

| Field | Type | Meaning |
|---|---|---|
| `speed` | float | 0.25 to 8.0; a value outside the range is clamped and the clamped value is acknowledged |

### queue-change

| Field | Type | Meaning |
|---|---|---|
| `change.kind` | string | `tactics` or `substitution`; any other value is refused by name |
| `detail` | object | `{ "patch": … }` for `tactics`, `{ "off": …, "on": … }` for `substitution`; see below |

`detail` for a substitution:

| Field | Type | Meaning |
|---|---|---|
| `off` | integer | the squad index of the player leaving |
| `on` | integer | the squad index of the substitute coming on |

`detail` for a tactics change is `{ "patch": { … } }`. Every field of the patch is optional
and every value is an index into the hello's `tactics`:

| Field | Type | Meaning |
|---|---|---|
| `patch` | object | the tactics change |
| `formation` | integer | an index into `tactics.formations` |
| `mentality` | integer | an index into `tactics.mentalities` |
| `instructions` | array of 6 | one level index or `null` per instruction, in the order of `setup.instructions`; `null` leaves that instruction as it is |
| `roles` | array | `{ "squad": …, "role": …, "duty": … }` entries; `squad` names the player by squad index |

The change is queued for the home team, the one the page manages. A `queue-change` sent
before the first `start` is refused with `the match has not started; set the pre-match
tactics with the lineup`. A `detail` that does not match its kind, misses a field, or names
an unknown field is refused at once, with a reason that starts `cannot read the tactics
change` or `cannot read the substitution change`. A readable change is acknowledged and
waits for the next stoppage that admits it; its verdict is a `tactics-change` event whose
`change.queue_id` is the identifier the acknowledgement returned. A change the page queues
while the engine is paused is queued on the first tick after `start`, so it never applies on
the resume tick. The football rules (the limit, the windows, a player not on the pitch)
are the engine's and arrive as a `rejected` verdict with the reasons listed under **event**.

### set-lineup

Sent before the first `start`, once or more; the last lineup accepted before kick-off
stands. The same object shape as the hello's `setup`, as indices:

| Field | Type | Meaning |
|---|---|---|
| `lineup` | array of 11 integers | squad indices in slot order; slot 0 is the goalkeeper |
| `bench` | array of integers | squad indices of the named substitutes, at most `tactics.ai.bench_size` |
| `patch` | object | optional pre-match tactics, the same shape as a tactics change's patch; applied before kick-off |

It is acknowledged, or refused with the first fault in this order: `<n> starters; a match
needs 11`, `squad index <i> is not in the squad of <n>`, `squad index <i> is placed twice`,
`<n> substitutes; the bench holds <size>`, `slot 0 needs a goalkeeper`, a patch index the
tactics file does not hold, or `squad index <i> has a pre-match role and is not in the
lineup`. After kick-off it is refused with `the match has started; a lineup can be set only
before kick-off`.

### seen

The newest tick the viewer has drawn. It is acknowledged like `start`.

| Field | Type | Meaning |
|---|---|---|
| `tick` | integer | the tick on screen; a rewind reports a lower tick |

On `serve`, once a client has sent one, the engine produces no tick more than `buffer_ticks`
past the newest reported tick and waits until the next report moves it on. A change the
manager queues therefore reaches the engine at most `buffer_ticks` after the tick on screen.
A client that never sends it is not held by it, and `record`, `replay`, and `bench` ignore it.

### cancel-change

Withdraws a queued change before a stoppage takes it. It is answered at once: an `ack` that
names the change, or a `reject` with `unknown change <id>`, `change <id> has already applied`,
or `change <id> was already refused`. A withdrawn change never applies and gets no verdict
event; its `queued` row stays in the events file. To edit a change, a page withdraws it and,
on the acknowledgement, queues the edited change.

| Field | Type | Meaning |
|---|---|---|
| `change.queue_id` | string | the identifier the change's `queue-change` acknowledgement gave |

## The page server

`serve --web <DIR>`, `replay --web <DIR>`, and `launch --web <DIR>` start a second listener
on `127.0.0.1`, on its own port, serving the named folder over plain HTTP. It is a separate listener from the
WebSocket server on purpose: the socket's origin allowlist and version guard have nothing
to do with serving a stylesheet.

It answers `GET` and `HEAD`, and `POST` on the two action paths below; anything else is
`405`. A request path that contains `..`,
a drive letter, a backslash, or a leading `//` is `403`, and the resolved path is compared
against the resolved folder, so a symbolic link cannot lead out of it either.

| Extension | Content type |
|---|---|
| `.html` | `text/html; charset=utf-8` |
| `.css` | `text/css; charset=utf-8` |
| `.mjs`, `.js` | `text/javascript; charset=utf-8` |
| `.woff2` | `font/woff2` |
| `.json` | `application/json; charset=utf-8` |
| `.png` | `image/png` |
| `.svg` | `image/svg+xml` |
| anything else | `application/octet-stream` |

Every response carries three headers:

| Header | Value | Why |
|---|---|---|
| `Cross-Origin-Opener-Policy` | `same-origin` | with the next one, makes the page cross-origin isolated |
| `Cross-Origin-Embedder-Policy` | `require-corp` | `performance.measureUserAgentSpecificMemory()` is unavailable without it |
| `Cache-Control` | `no-store` | a reload always shows the current file |

`require-corp` refuses every cross-origin subresource. Everything the page loads is
same-origin, including both fonts, so nothing is lost today; a later version that wants an
external resource will meet this rule.

One path is generated rather than read from the folder. `GET /engine.json` answers with the
state of the engine and the port of its socket, because a page served on one port cannot
guess the WebSocket port on another and the operating system chooses both at every run.

| Key | Type | Meaning |
|---|---|---|
| `engine.state` | string | `starting`, `running`, `finished`, `crashed`, `refused`, `abandoned`, or `not-found` |
| `socket.port` | int or null | the WebSocket port; present only while `running` |
| `protocol.version` | int | the protocol version the engine speaks, `3` |
| `engine.pid` | int or null | the process identifier of the engine serving the match |
| `engine.path` | string or null | the engine program the launcher runs; null when the engine serves the page itself |
| `engine.reason` | string or null | the engine's own words when `refused`, for example `snapshot refused: <path>: checksum mismatch: the file is corrupt` |
| `engine.code` | int or null | the exit code when `crashed` (launcher only) |
| `snapshot.tick` | int or null | the tick of the latest snapshot on disk (launcher only) |
| `match.id` | string | the match being served |
| `launcher` | bool | `true` when `engine-cli launch` serves the page and can restart the engine |

`serve --web` and `replay --web` always answer `running`, because the page's server is the
engine process itself. Under `launch` the page's server is a separate process that runs the
engine as a worker, so it can report `crashed` and act on it:

| Request | Effect | Answer |
|---|---|---|
| `POST /engine/restart` | after a crash, starts the engine again from the match's latest snapshot; a snapshot that does not read is `refused` with the reason | `202` with the new `/engine.json` body |
| `POST /engine/abandon` | stops the engine and gives the match up | `202` with the new `/engine.json` body |

Both ignore any body. Both require an `Origin` header equal to the page's own origin,
`http://127.0.0.1:<page port>`; any other origin, or none, is `403`, so a page on another
site cannot restart or stop the engine. Served by `serve --web` or `replay --web`, where
nothing would survive to carry them out, both are `405`.

## Reconnection

A connection that is lost without a close frame (the browser reports code `1006`) is a
drop. `serve --reconnect-wait <seconds>` keeps the socket listening on the same port for
that long; the launcher passes 120. Without it, or after a close frame, the run ends as
before.

After a drop the engine goes back to the newest stoppage the page received in full: the
socket records the newest tick frame it flushed, and a snapshot is kept only once a later
tick frame has been flushed, so every event of the stoppage tick went before it. The page
that connects again receives the same `hello` as before, with the same `match.id`. The
first tick frame after it is a keyframe one tick past that stoppage, so the page cuts every
store back to the stoppage and appends from there. A drop before the first stoppage starts
the match again from kick-off with the same seed and lineup.

A restart after a crash works the same way on the page's side: the launcher starts
`serve --resume <snapshot>`, which sends the same `hello` and continues at the snapshot's
tick plus one. No message and no field changed for either path, so `PROTOCOL_VERSION`
stays 3.

## Backpressure

The engine runs ahead of the viewer by at most `buffer_ticks` ticks, which
`content/tuning.json` names and which defaults to 500. When the buffer fills, the
simulation thread stops until the viewer drains it. No tick is dropped, memory stays flat,
and one `socket.backpressure` line records each pause with how long it lasted.

A page reads the socket as fast as it can and stores every tick, so the buffer alone does not
keep a live match close to what the manager is watching. The viewer therefore reports the
tick it draws with `seen`, which holds a `serve` engine within `buffer_ticks` of it. It also
sends `pause` when it holds more than a few seconds of unplayed ticks and `start` when
playback catches up.

## Replay files

`engine-cli record --seed <n> --out <file>.smfx` writes every frame of one match exactly as
it would travel on the wire. `engine-cli replay --fixture <file>.smfx --speed <x>` serves
those bytes back over the same protocol with no re-encoding, so a viewer can be verified
before the engine is complete. `engine-cli resimulate --fixture <file>.smfx` plays the match
again from the file alone and compares it with the stored frames.

Two formats exist. Format 3 holds the frames only. The viewer page saves format 3, because
it receives only the frames. Format 4 also holds every input of the match by value and a
record of the engine, the settings, and the applied changes. `record` writes format 4.

| Part | Bytes | Contents |
|---|---|---|
| header | 32 | magic `SMFX`; the format version (u16) at offset 4; the frames' protocol version (u16) at offset 6, zero in format 3; the match start in milliseconds; the frame count; the tick count; the seed |
| input entries | 9 + payload each | format 4 only, before the frames: kind 2, tick 0, payload length; the payload is the name length (u16), the UTF-8 name, and the file's bytes |
| frame entries | 9 + payload each | kind 0 (binary tick frame) or 1 (text frame), the tick index, the payload length, then the payload |
| record entry | 9 + payload | format 4 only, last: kind 3, tick 0, payload length; the payload is one JSON document |
| trailer | 16 | magic `SMFE`, the frame count, and the first six bytes of a SHA-256 over every payload of every kind, in file order |

All numbers are little-endian. The frame count counts tick and text frames only. In format 3,
offset 4 was the protocol version, which is also 3, so every older file reads as format 3.

### Inputs

Format 4 stores these files, in this order. The two pack files are present only when the match
ran a script pack.

| Role | File |
|---|---|
| `attributes` | `attributes.json` |
| `tuning` | `tuning.json` |
| `rules` | `rules/default.json` |
| `tactics` | `tactics.json` |
| `commentary` | `commentary/en.json`: the script commentary hook runs on every event, so the lines are a match input |
| `team_a` | the home team file |
| `team_b` | the away team file |
| `pack_manifest` | `pack.json` of the script pack |
| `pack_script` | the pack's entry script |

With the shipped content and the sample script pack, the inputs hold 87,977 bytes. A one-minute
match recorded with them is 314,484 bytes, against 220,901 bytes for the same length in format 3.
The inputs are never cut to save space.

### Record

| Field | Contents |
|---|---|
| `engine` | `commit` (the full git commit, or `unknown`), `dirty` (the working tree held changes at build time), `crate_version`, `scheme` (the random-stream scheme), `maths` (the maths library and its version), `executable_sha256` (the SHA-256 of the executable file), and `build` (the short build hash the hello names) |
| `settings` | `seed` (a decimal string), `minutes`, `knockout`, and `managers` (`ai` or `human`, home first) |
| `inputs` | one entry per input entry, in file order: `role`, `name`, `bytes`, and `sha256` |
| `inputs_bytes` | the total size of the input files |
| `changes` | the applied-change log |
| `watchdog` | `slow_calls` (script calls past the wall-clock limit) and `invalid` (`slow script`, or null) |

Each entry of the change log holds `order` (from 0), `team`, `source` (`manager` for a team
managed by hand, `ai` for the computer manager), `queued_tick` and `queue_number` (the change's
queue identifier), `tick` (the tick it applied on), `stoppage` (the stoppage it applied at), and
`change`: `{ "substitution": { "off", "on" } }` with squad indexes, or `{ "tactics": {
"formation", "mentality", "instructions", "roles": [{ "squad", "role", "duty" }] } }`, where a
null leaves that setting as it is. Only applied changes are in the log. A rejected change's
verdict stays in the text frames.

### Re-simulation

Re-simulation builds the match from the stored inputs, queues each `manager` entry of the log
again at its `queued_tick`, and lets the computer manager make its own changes again. It
compares every regenerated tick frame with the stored one, byte for byte, and after full time
it compares the applied changes with the log, entry by entry. Text frames are not re-simulated:
they carry the recording's match stamp and owner, and the verdict rows of rejected changes,
which the log does not keep. The script runs with no wall-clock limit, so the recorded watchdog
mark is reported, not made again.

A reader refuses a file with the wrong magic, a format newer than the reader knows or older
than 3, frames of another protocol version, a count mismatch, a truncated entry, an input or
record entry in a format-3 file, an input entry after the frames, an entry after the record, a
format-4 file with no record or with two, a record without one of its fields or with a field
it does not define, a field of the wrong type or outside its values, a header tick count that
the frames do not match, a header seed that differs from the record's, input entries whose
names, sizes, or SHA-256 values differ from the record's list, a total of input sizes that
differs from the record's, an unknown entry kind, or a hash that does not match the bytes. Each refusal names its check. A missing record field is
never filled with a default: a field that may be null, such as `watchdog.invalid`, must still
be present.

### Migration

A replay file of format 4 or later keeps loading after the format changes. Each format change
adds one forward step, and a reader lifts an older file through every step from its own format
to the newest format before it reads the file. A lifted file keeps the hash of the bytes that
were read.

Version-3 files are never lifted: they play from their frames only.

When the engine lifts a file, it logs one `replay.migrated` signal with the file's format
(`from`), the format it was lifted to (`to`), and the number of steps (`steps`). The viewer
reports the same three values as `migrated` for a file it lifted.

A reader refuses a file whose format is newer than the newest step reaches, a chain with a
missing step, a step whose output is not the format it names, a record that lacks a required
field, and input entries that do not match the record's list (corrupted inputs). A field that
was added by a lift is never filled with a default.
