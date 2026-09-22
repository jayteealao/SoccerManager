# SoccerManager

A football management game: a native match engine written in Rust and a browser-based 2D match viewer.

This repository currently holds the engine core and its data layer: a headless simulation at 50 ticks per second, a tick-stream file format, a validator, a benchmark command, validated content files, and a team generator.

## Build

```bash
cargo build --release
```

## Content

The engine reads its attribute schema, tuning constants, rule pack, and team files from the `content/` folder. Run the binary from the repository root, pass `--content-dir <DIR>`, or set `SM_CONTENT_DIR`. Every file is validated on load; a bad value stops the run with a message naming the file and the field. See `content/README.md` for every field, its unit, its default, and its bound.

Runtime output goes to `SM_DATA_DIR` (default `%LOCALAPPDATA%\SoccerManager`): `owner.id`, created once, and `matches/<match.id>/stats.json` per simulated match.

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

## Benchmark

```bash
target/release/engine-cli bench --seed 42 --matches 5 --json
```

The command prints one JSON line with the median wall time, CPU time, peak memory, ticks per second, a hashed machine identifier, the CPU model, the content hash, and the build hash. The exit code is 2 when the median wall time for a 90-minute match exceeds 2000 ms.

## Test

```bash
cargo test --workspace
```

## License

Licensed under either of the MIT license (LICENSE-MIT) or the Apache License, Version 2.0 (LICENSE-APACHE), at your option.
