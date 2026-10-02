# SoccerManager

A football management game: a native match engine written in Rust, and Touchline, a 2D match viewer that runs in the browser.

You pick a lineup and tactics, kick off against a club the computer manages, and watch the match on a 2D pitch. The engine computes the ball and all 22 players at 50 ticks per second, applies the laws of the game, and writes commentary. During the match you change tactics and make substitutions; each change applies at the next stoppage. At half-time and at full time a report counts the match, and at full time you can save a replay.

## Install

On Windows 11, run `SoccerManager-<version>-windows-x64-setup.exe`. It installs for your user only, with no administrator prompt, and adds **Soccer Manager** to the Start menu. The Start-menu entry starts the engine and opens the match page in your browser. The setup file is not signed, so SmartScreen can warn about it: choose **More info**, then **Run anyway**.

On Linux x86_64 (glibc 2.39 or later), unpack `SoccerManager-<version>-linux-x86_64.tar.gz` and run `./soccermanager` in the unpacked folder.

Each start opens on the start screen: start a new match with two clubs you pick, resume the match you left, open a replay, change the settings, read the licences, or quit. `engine-cli launch --no-start-screen` skips it and starts a match at once. Your matches stay in `%LOCALAPPDATA%\SoccerManager` on Windows and `~/.local/share/SoccerManager` on Linux, also after an uninstall. To build the setup file and the archive yourself, see [packaging/README.md](packaging/README.md).

## Build

You need Rust 1.87 or later, and Node 22.12 or later for the viewer.

```bash
cargo build --release
npm --prefix viewer ci
npm --prefix viewer run build
```

The second pair builds the match viewer into `viewer/dist`, the page the engine serves. The viewer is a Svelte 5 app built with Vite.

## Play a match

```bash
target/release/engine-cli generate --seed 2026 --clubs 2 --out my-league
target/release/engine-cli serve --seed 42 --web viewer/dist --team-a my-league/club-000007ea-00.json --team-b my-league/club-000007ea-01.json
```

The second command prints a port and a page address. Open the page address in a browser. For a step-by-step first match, follow [the tutorial](docs/tutorials/first-match.md).

To have the page survive a crash of the engine, start the launcher instead:

```bash
target/release/engine-cli launch --seed 42 --web viewer/dist
```

From the repository folder, `target/release/engine-cli launch --open` also works: it finds the built viewer in `./viewer/dist`, picks a seed, and opens the page in your browser.

When the engine stops in mid-match, the page offers to restart from the last stoppage.

## Other commands

```bash
target/release/engine-cli simulate --seed 42 --ticks-out match.ticks   # one match, no viewer
target/release/engine-cli bench --seed 42 --matches 5 --json           # time the engine
target/release/engine-cli calibrate --seed 2026 --matches 1000         # check the realism bands
target/release/engine-cli record --seed 7 --out match.smfx             # record a replay file
target/release/engine-cli replay --fixture match.smfx --web viewer/dist # play a replay file
```

The engine reads its data from the `content/` folder and writes each match to `SM_DATA_DIR` (default `%LOCALAPPDATA%\SoccerManager` on Windows).

To compare a candidate model with the current one, declare a flag in the `flags` block of `content/tuning.json` and run `calibrate --pair <flag>`. Both arms play the same fixtures on the same seeds, and the report shows every realism band of both arms side by side with a verdict. The steps and the removal checklist are in [the modding how-to](docs/how-to/modding.md#compare-two-models-with-a-flag).

## Documentation

- Tutorial: [Play your first match](docs/tutorials/first-match.md)
- How-to: [Tune the engine and add a rule pack](docs/how-to/modding.md), [Write and register an engine module](docs/how-to/engine-modules.md)
- Reference: [Command line](docs/reference/cli.md), [Data files](docs/reference/data-files.md), [Socket protocol](docs/reference/protocol.md), [Engine modules and slot configuration](docs/reference/engine-modules.md)
- Explanation: [How the engine works](docs/explanation/engine.md), [The plugin contract](docs/explanation/plugin-contract.md)

## Test

The engine tests:

```bash
cargo test --workspace
```

The slow tests (the tactics effects and the two 1000-match calibration suites) are ignored by default. Run them in release:

```bash
cargo test --release --workspace -- --ignored
```

The viewer tests run with Vitest:

```bash
npm --prefix viewer ci
npm --prefix viewer test
```

### The browser suite

The browser suite in `e2e/` drives the viewer against the live engine with Playwright. Each test starts its own engine with a new data folder and serves the built viewer. Build the engine and the viewer first, then install and run the suite:

```bash
cargo build --release
npm --prefix viewer ci
npm --prefix viewer run build
cd e2e
npm install
npx playwright install chromium
npm test
```

`npm test` runs every test headless. `npm run test:viewer` runs the viewer tests (about 13 minutes). `npm run test:scenario` runs the whole first match (about 15 minutes). To run the tests in the installed Microsoft Edge, set `PW_CHANNEL=msedge`. `e2e/README.md` lists which test covers each behaviour of the page.

### The benchmark

```bash
target/release/engine-cli bench --seed 42 --matches 5 --json
```

The exit code is 2 when the median wall time of a 90-minute match is more than 2000 ms. A run that fails, for example with `--matches 0`, exits 1 and prints one record with `outcome` `error` and the keys `error.type`, `error.code`, and `error.retriable`; `simulate` does the same. To time 1000 matches, run `bench --seed 42 --matches 1000 --json` and measure the total wall time; the budget is 30 minutes.

## License

Licensed under either of the MIT license (LICENSE-MIT) or the Apache License, Version 2.0 (LICENSE-APACHE), at your option. A test (`crates/engine-cli/tests/licenses.rs`) checks that no dependency is licensed only under the GPL or the AGPL.
