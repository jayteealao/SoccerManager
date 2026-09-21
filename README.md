# SoccerManager

A football management game: a native match engine written in Rust and a browser-based 2D match viewer.

This repository currently holds the engine core: a headless simulation at 50 ticks per second, a tick-stream file format, a validator, and a benchmark command.

## Build

```bash
cargo build --release
```

## Simulate a match

```bash
target/release/engine-cli simulate --seed 42 --ticks-out match.ticks
```

The command writes one record per tick to the `.ticks` file and prints one JSON line with the match statistics.

## Benchmark

```bash
target/release/engine-cli bench --seed 42 --matches 5 --json
```

The command prints one JSON line with the median wall time, CPU time, peak memory, ticks per second, a hashed machine identifier, the CPU model, and the build hash. The exit code is 2 when the median wall time for a 90-minute match exceeds 2000 ms.

## Test

```bash
cargo test --workspace
```

## License

Licensed under either of the MIT license (LICENSE-MIT) or the Apache License, Version 2.0 (LICENSE-APACHE), at your option.
