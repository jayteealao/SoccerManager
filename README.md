# SoccerManager

A football management game: a native match engine written in Rust and a browser-based 2D match viewer.

This repository currently holds the engine core, its data layer, the laws of the game, and the live match stream: a headless simulation at 50 ticks per second, a referee, a tick-stream file format, match snapshots, a validator, a benchmark command, validated content files, a team generator, and a versioned WebSocket protocol that streams a match to a viewer page.

## Build

```bash
cargo build --release
```

## Content

The engine reads its attribute schema, tuning constants, rule pack, tactics file, and team files from the `content/` folder. Run the binary from the repository root, pass `--content-dir <DIR>`, or set `SM_CONTENT_DIR`. Every file is validated on load; a bad value stops the run with a message naming the file and the field. See `content/README.md` for every field, its unit, its default, and its bound.

Runtime output goes to `SM_DATA_DIR` (default `%LOCALAPPDATA%\SoccerManager`): `owner.id`, created once, `matches/<match.id>/stats.json`, `events.jsonl`, and `snapshot.smsn` per match, and `engine.port` while a match is being served.

## Simulate a match

```bash
target/release/engine-cli simulate --seed 42 --ticks-out match.ticks
```

The command loads the two default clubs, writes one record per tick to the `.ticks` file, saves `stats.json`, and prints one JSON line with the match statistics, the law counts and the tactics counts (shots, injuries, substitutions, mean energy, and the verdicts on queued changes) included. Pass `--team-a <FILE>` and `--team-b <FILE>` to play other clubs.

A 90-minute match lasts 90 minutes plus the time added to each half, so the tick count varies from match to match. The tick-file header states the most ticks the match can last. A match shorter than the rule pack's two halves plays no added time.

## The laws

The referee runs inside the simulation loop. The engine enforces these laws:

- The ball out of play: a throw-in where the ball crossed the touchline, and a goal kick or a corner at the goal line, for the team that did not touch the ball last.
- Offside: a player beyond the second-last opponent and the ball, in the opponents' half, when a team-mate plays the ball, is penalised at the first touch with an indirect free kick. A throw-in, a goal kick, and a corner put nobody offside.
- Fouls: one draw decides a tackle as a clean win, a foul, or a miss. The chance of a foul rises with the tackler's `aggression` and falls with tackling skill. A foul gives a free kick where it happened, or a penalty inside the penalty area. When the fouled team keeps the ball outside the area, play continues with advantage.
- Cards: a yellow card, a second yellow, or a red card. A player who is sent off leaves the pitch, and the rest of the player's line spreads across the gap. A team with fewer players than the rule pack's minimum ends the match.
- Restarts: the players walk to their restart positions, and opponents stand 9.15 m from the ball at a free kick, a corner, and a kick-off.
- The clock: the teams change ends at half-time, and each half gets added time from its stoppages and cards plus a seeded variance.

The rule pack (`content/rules/default.json`) and the tuning file set every number. See `content/README.md`.

## Tactics and the AI manager

Each team plays a formation, a mentality, six team instructions (pressing, width, tempo, line height, passing directness, time wasting), and a role and duty for every player, all named in `content/tactics.json`. The ball carrier scores its options (pass, dribble, shot, clearance, or holding the ball) and takes the best, so tactics and attributes change what players do.

In `simulate`, `resume`, and `serve` the AI manager runs both teams (in `serve`, only the away team; the home team keeps the AI's pre-match setup). Before kick-off it picks the eleven who fit its formation's roles and names a bench. During the match it replaces injured and tired players, attacks when it trails late, and protects a late lead. Its changes wait for a stoppage the rule pack admits them at, within the rule pack's substitution limit and windows; a change the rules refuse is rejected with the reason.

Players tire over the match: below an energy threshold, pace, passing, finishing, and decisions fall along the fatigue curve in `content/tuning.json`. A tackle or plain bad luck can injure a player, who leaves play at once; the referee stops play for a dropped ball, and the AI manager sends on a substitute.

## Resume a match

```bash
target/release/engine-cli resume --snapshot <SM_DATA_DIR>/matches/<match.id>/snapshot.smsn
```

`simulate` and `serve` write the latest snapshot of the match at every stoppage, replacing the one before it. `resume` continues the match from that snapshot to full time and prints one JSON line with the match statistics. The resumed match is the same match: tick for tick, it plays exactly as the uninterrupted match would. Add `--ticks-out <FILE>` to write the resumed ticks. Pass `--no-snapshot` to `simulate` to write no snapshot.

A snapshot resumes only on the build that wrote it, with the same content and team files. A damaged file, a file from another build, or a file from other content is refused with the reason named, and the exit code is 1.

## Generate clubs

```bash
target/release/engine-cli generate --seed 7 --clubs 20 --out my-league
```

The command writes one team file per club with fictional names and per-position attribute distributions. The same seed gives the same clubs.

## Stream a match to a viewer

```bash
target/release/engine-cli serve --seed 42
```

The command binds a port on `127.0.0.1`, prints it, writes it to `engine.port`, and waits for
one viewer. A page connects to `ws://127.0.0.1:<port>/?v=2` and receives a hello, then one
binary frame per tick. See `docs/reference/protocol.md` for every message and every field.

## Watch a match

```bash
target/release/engine-cli serve --seed 42 --web web
```

The command prints two lines: the WebSocket port, then the page address. Open the page
address in a browser. The engine serves the page itself, so there is one program to start,
the page's origin is already on the socket's loopback allowlist, and every response carries
the two cross-origin-isolation headers the page's memory gauge needs.

Open `/handshake.html` on the same address to check the connection alone: it reads the
hello, prints it, and renders nothing, so a fault there is a fault in the handshake rather
than in the renderer.

## Record and replay a fixture

```bash
target/release/engine-cli record --seed 7 --out match.smfx
target/release/engine-cli replay --fixture match.smfx --speed 8
```

`record` writes every frame of one match exactly as it would travel on the wire, starting
with the hello, so a replay forwards the recorded match rather than describing the build
that replays it. `replay` serves those bytes back over the same protocol, so a viewer can
be verified before the engine is complete. Add `--web web` to serve the page beside it.

`replay --sustain <speed>` caps the delivered rate below `--speed`, which is how the
viewer's lag notice is tested. It exists on `replay` alone and never on `serve`.

## Test the page

```bash
node --test "web/tests/*.test.mjs"
```

Node 22 ships the test runner, so there is no `package.json`, no install, and no third
dependency. The decoder test also reads `fixture.smfx` at the repository root when one has
been recorded; without it that one test reports a skip naming the command to record it.

## Benchmark

```bash
target/release/engine-cli bench --seed 42 --matches 5 --json
```

The command prints one JSON line with the median wall time, CPU time, CPU time per simulated tick, the ticks in the median match, peak memory, ticks per second, a hashed machine identifier, the CPU model, the content hash, and the build hash. The exit code is 2 when the median wall time for a 90-minute match exceeds 2000 ms. Add `--stream` to also measure the socket: one match delivered to a client that reads as fast as it can.

## Test

```bash
cargo test --workspace
```

## License

Licensed under either of the MIT license (LICENSE-MIT) or the Apache License, Version 2.0 (LICENSE-APACHE), at your option.
