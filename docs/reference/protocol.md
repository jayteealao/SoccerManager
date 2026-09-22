# Match stream protocol, version 2

The engine serves one viewer over a WebSocket on `127.0.0.1`. The port is chosen by the
operating system at every run and written to `engine.port` inside the runtime data folder.
A page connects to `ws://127.0.0.1:<port>/?v=2`.

Version 2 changed the meaning of `ticks_expected` from the exact tick count to the most ticks
the match can last, because the time added at the end of each half is known only when the
half ends. The event message gained the law event types and six optional fields. Version 2
later gained the `injury`, `substitution`, and `ai-decision` event types and the optional
fields `change.applied_tick` and `ai.decision`, and change events now carry `team.id`. No
field changed meaning.

A test in `crates/protocol/tests/document.rs` holds this document to the code: every message
the implementation names must appear below with every one of its fields.

## Connecting

| Step | Rule |
|---|---|
| Address | `ws://127.0.0.1:<port>/?v=<protocol version>` |
| Version | `v` must equal `1`. Any other value, or no value, is refused with both versions named. |
| Origin | `null`, a `file://` page, any port of `http://localhost` or `http://127.0.0.1`, or no `Origin` header at all. Any other origin is refused. |
| Clients | One viewer per match. |
| First message | `hello`, always before the first tick frame. |

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
| `protocol.version` | integer | always 2 in this build |
| `engine.version` | string | the engine crate version |
| `build.hash` | string | the git hash the engine was built from |
| `owner.id` | string | 32 hex characters, created once per machine |
| `match.id` | string | `<seed as 16 hex>-<start time in milliseconds>` |
| `seed` | integer | the seed the match runs from |
| `dt_ms` | float | milliseconds per tick, 20.0 |
| `ticks_expected` | integer | the most ticks the match can send: regulation time plus the cap on added time in every half; the match ends earlier when it earns less added time, and the `full-time` event marks the last tick |
| `keyframe_interval` | integer | ticks between keyframes |
| `teams` | array of two | one entry per club, home first; see the table below |

Each entry of `teams`:

| Field | Type | Meaning |
|---|---|---|
| `team.id` | string | the club identifier from the team file |
| `team.name` | string | the club name |
| `team.kit.primary` | string | lower-case `#rrggbb`, the shirt colour |
| `team.kit.secondary` | string | lower-case `#rrggbb`, the trim colour |

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
| `event.type` | enumeration | `kick-off`, `goal`, `half-time`, `full-time`, `tactics-change`, `offside`, `foul`, `card`, `throw-in`, `corner`, `goal-kick`, `free-kick`, `penalty`, `injury`, `substitution`, or `ai-decision` |
| `team.id` | string | the club the event belongs to: the offender's club on `offside`, `foul`, and `card`, the club that restarts play on a restart, the injured player's club on `injury`, the club that changes on `substitution`, `ai-decision`, and `tactics-change`; absent at half-time and full time |
| `home.score` | integer | the score after the event |
| `away.score` | integer | the score after the event |
| `change.kind` | enumeration | `tactics` or `substitution`; change events only |
| `change.queued_tick` | integer | the tick the change was queued on |
| `change.queue_id` | string | the identifier the acknowledgement returned |
| `change.rejected_reason` | string | present only on a refused change |
| `change.state` | enumeration | `queued`, `applies-now`, `applied`, or `rejected` |
| `change.applied_tick` | integer | on an applied change: the tick it took effect on, which is the tick of the stoppage that admitted it |
| `ai.decision` | string | on `ai-decision`: `mentality-up-trailing`, `mentality-down-leading`, `sub-injury`, or `sub-fatigue` |
| `player.id` | string | the player the event names: the offender on `offside` and `foul`, the booked player on `card`, the injured player on `injury`, the player leaving on `substitution`, the taker on `kick-off` and on every restart, and on `goal` the player who kicked the ball last (a player of the other club on an own goal) |
| `player.secondary_id` | string | the fouled player, on `foul`; the player coming on, on `substitution` |
| `card.kind` | enumeration | `yellow`, `second-yellow`, or `red`, on `card`; a second yellow sends the player off |
| `foul.advantage` | boolean | on `foul`: `true` when play continued because the fouled team kept the ball; a card for that foul follows at the next stoppage |
| `minute.added` | integer | in added time only: the added minute, 2 at 45+2 |
| `added_time.s` | integer | on `half-time` and `full-time`: the seconds added to the half that ended |
| `commentary` | string | one English commentary line, on every event except `tactics-change`; the lines come from `content/commentary/en.json` and name the player and the club |

A restart event (`kick-off`, `throw-in`, `corner`, `goal-kick`, `free-kick`, `penalty`)
arrives on the same tick as the restart keyframe that places the ball. Play resumes when the
taker plays the ball, a few seconds later. An `injury` in open play stops play for a dropped
ball at the ball; an injury on a tick that already stopped play rides that stoppage.

The engine applies its own queued changes (the AI manager's) on the tick that opens a
stoppage the rule pack admits them at, substitutions first. Each verdict is a
`tactics-change` event with `change.state` `applied` or `rejected`. A rejection names its
reason in `change.rejected_reason`:

- `substitution limit reached (5 of 5)`
- `no substitution window left (3 of 3)`; a half-time substitution uses no window
- `player <id> is not on the pitch`
- `player <id> was sent off and cannot be replaced`
- `player <id> is not on the bench`
- `player <id> left the pitch at this stoppage; the substitution applies first`: a tactics
  change that names a player substituted off at the same stoppage
- `the change names a formation, mentality, level, role, or duty the tactics file does not hold`

### stats

Sent once, at full time.

| Field | Type | Meaning |
|---|---|---|
| `tick` | integer | the last tick |
| `minute` | integer | the last simulated minute |
| `home.score` | integer | final score |
| `away.score` | integer | final score |
| `possession.changes` | integer | times possession changed hands |
| `ball.max_speed` | float | metres per second |
| `ball.idle_ticks` | integer | ticks the ball stood still |

### ack

| Field | Type | Meaning |
|---|---|---|
| `command` | string | `start`, `pause`, `set-speed`, or `queue-change` |
| `change.queue_id` | string | queued changes only |
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

Resumes production. No fields. A client that never sends it still receives the whole match.

### pause

Pauses production at the current tick. No fields.

### set-speed

| Field | Type | Meaning |
|---|---|---|
| `speed` | float | 0.25 to 8.0; a value outside the range is clamped and the clamped value is acknowledged |

### queue-change

| Field | Type | Meaning |
|---|---|---|
| `change.kind` | string | `tactics` or `substitution`; any other value is refused by name |
| `detail` | object | opaque in this build; not yet read by the engine |

This build queues the change and answers. A change sent over the socket is not applied to
play yet, because its `detail` is not read: the engine applies its own queue, which the AI
manager fills, and a later version routes a client's change into that queue.

## The page server

`serve --web <DIR>` and `replay --web <DIR>` start a second listener on `127.0.0.1`, on its
own port, serving the named folder over plain HTTP. It is a separate listener from the
WebSocket server on purpose: the socket's origin allowlist and version guard have nothing
to do with serving a stylesheet.

It answers `GET` and `HEAD` only; anything else is `405`. A request path that contains `..`,
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

One path is generated rather than read from the folder. `GET /engine.json` answers
`{"socket.port": <port>, "protocol.version": 1}`, because a page served on one port cannot
guess the WebSocket port on another and the operating system chooses both at every run.

## Backpressure

The engine runs ahead of the viewer by at most `buffer_ticks` ticks, which
`content/tuning.json` names and which defaults to 500. When the buffer fills, the
simulation thread stops until the viewer drains it. No tick is dropped, memory stays flat,
and one `socket.backpressure` line records each pause with how long it lasted.

## Fixtures

`engine-cli record --seed <n> --out <file>.smfx` writes every frame of one match exactly as
it would travel on the wire. `engine-cli replay --fixture <file>.smfx --speed <x>` serves
those bytes back over the same protocol with no re-encoding, so a viewer can be verified
before the engine is complete.

| Part | Bytes | Contents |
|---|---|---|
| header | 32 | magic `SMFX`, protocol version, match start in milliseconds, frame count, tick count, seed |
| entry | 9 + payload | entry kind (binary or text), tick index, payload length, then the payload |
| trailer | 16 | magic `SMFE`, frame count, and the first six bytes of a SHA-256 over every payload |

A reader refuses a fixture with the wrong magic, an unknown protocol version, a count
mismatch, a truncated entry, or a hash that does not match the bytes.
