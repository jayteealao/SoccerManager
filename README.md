# SoccerManager

A football management game: a native match engine written in Rust and a browser-based 2D match viewer.

This repository currently holds the engine core, its data layer, and the live match stream: a headless simulation at 50 ticks per second, a tick-stream file format, a validator, a benchmark command, validated content files, a team generator, and a versioned WebSocket protocol that streams a match to a viewer page.

## Build

```bash
cargo build --release
```

## Content

The engine reads its attribute schema, tuning constants, rule pack, and team files from the `content/` folder. Run the binary from the repository root, pass `--content-dir <DIR>`, or set `SM_CONTENT_DIR`. Every file is validated on load; a bad value stops the run with a message naming the file and the field. See `content/README.md` for every field, its unit, its default, and its bound.

Runtime output goes to `SM_DATA_DIR` (default `%LOCALAPPDATA%\SoccerManager`): `owner.id`, created once, `matches/<match.id>/stats.json` and `events.jsonl` per match, and `engine.port` while a match is being served.

## Simulate a match

```bash
target/release/engine-cli simulate --seed 42 --ticks-out match.ticks
```

The command loads the two default clubs, writes one record per tick to the `.ticks` file, saves `stats.json`, and prints one JSON line with the match statistics. Pass `--team-a <FILE>` and `--team-b <FILE>` to play other clubs.

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
one viewer. A page connects to `ws://127.0.0.1:<port>/?v=1` and receives a hello, then one
binary frame per tick. See `docs/reference/protocol.md` for every message and every field.

## Record and replay a fixture

```bash
target/release/engine-cli record --seed 7 --out match.smfx
target/release/engine-cli replay --fixture match.smfx --speed 8
```

`record` writes every frame of one match exactly as it would travel on the wire. `replay`
serves those bytes back over the same protocol, so a viewer can be verified before the
engine is complete.

## Benchmark

```bash
target/release/engine-cli bench --seed 42 --matches 5 --json
```

The command prints one JSON line with the median wall time, CPU time, peak memory, ticks per second, a hashed machine identifier, the CPU model, the content hash, and the build hash. The exit code is 2 when the median wall time for a 90-minute match exceeds 2000 ms. Add `--stream` to also measure the socket: one match delivered to a client that reads as fast as it can.

## Test

```bash
cargo test --workspace
```

## License

Licensed under either of the MIT license (LICENSE-MIT) or the Apache License, Version 2.0 (LICENSE-APACHE), at your option.
