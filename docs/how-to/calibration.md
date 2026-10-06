# Measure a tuning change

This guide shows how to measure one tuning change in about two minutes: make a baseline once, run only the pairing, suite, or band the change targets, and read the diff. Then it shows how to run the full suites before a change is accepted.

Every flag and every report key is in [the command-line reference](../reference/cli.md#calibrate).

The commands use the release build. Build it first:

```bash
cargo build --release -p engine-cli
```

## Make a baseline

A baseline is the `report.json` of an earlier run on the same seed and match count, before your change. Make one full run on your seed:

```bash
target/release/engine-cli calibrate --seed 42 --matches 1000 --out runs/base-42
```

This plays every suite, 57,000 matches: see [How long the runs take](#how-long-the-runs-take). Keep `runs/base-42/report.json`; every targeted run below compares with it.

For the red-card experiment, make its own baseline:

```bash
target/release/engine-cli calibrate --suite red-card --seed 1 --matches 240 --out runs/base-red-card
```

A report made before sampling errors, fixtures hashes, or fixture keys existed cannot be a baseline: a report from before fixture keys played other matches, and the run refuses it by naming the old seeding scheme. Make a new one.

## Make the change

Edit the tuning file in your own content folder (see [Tune the engine and add a rule pack](modding.md)), or declare a feature flag for the change and turn it on with `--flag NAME=on`.

The change must not touch the inputs that decide the fixtures: the attributes, rules, and tactics files, the generator block of `tuning.json`, and the two default clubs. A run on other fixtures is refused, because its matches differ from the baseline's. A change of any other tuning value or flag is the change under test.

## Run only what the change targets

Run one formation pairing:

```bash
target/release/engine-cli calibrate --suite formations --pairing "4-4-1-1 v 4-4-2" --seed 42 --matches 1000 --baseline runs/base-42/report.json
```

The pairing plays the same 1,000 matches as in the full run, in about 80 seconds. Repeat `--pairing` for more pairings.

Run one suite:

```bash
target/release/engine-cli calibrate --suite equal --seed 42 --matches 1000 --baseline runs/base-42/report.json
```

Run only the suites that check one band:

```bash
target/release/engine-cli calibrate --band goalless_share --seed 42 --matches 1000 --baseline runs/base-42/report.json
```

The `goalless_share` band belongs to the equal and formations suites, so this plays both. Add `--suite equal` to play the equal suite only.

## Read the diff

The diff is printed on standard error when the run ends:

- The first line names the baseline and both content hashes. `content changed` means your change is in the run. `content unchanged` means the run measured the same content again.
- Each row shows a band's baseline value, new value, change, and sampling error.
- `noise` marks a change of at most two sampling errors. Do not read a noise row as progress.
- `change` marks a change larger than two sampling errors.
- The last column is the new run's verdict on the band: `pass` or `miss`.

The sampling error of a share band with few events is wide. For example, a goalless share of 0.08 over 1,000 matches has an error of about 0.009.

If the run stops before it plays, read the message. It names each difference from the baseline, for example `seed 7 differs from the baseline's 42`. Use a baseline on the same seed and match count, or make a new baseline.

## Run the red-card experiment

```bash
target/release/engine-cli calibrate --suite red-card --seed 1 --matches 240 --baseline runs/base-red-card/report.json
```

The experiment plays the default clubs with cards otherwise off: a control, and the away keeper, centre-back, or striker sent off at kick-off. Each seed plays twice, once in each home and away order of the two clubs, so each club is the reduced side in half the matches. `--matches 240` plays seeds 1 to 120 in both orders. `calib.red_card` in the report holds each arm's goals for the full and the reduced side, the control, and the verdict. The criterion passes when, in each arm, the reduced side does not outscore the full side and the full side scores at most 1.6 times the control's home goals.

## Confirm the change on the full suites

A targeted run is an inner loop. Before a change is accepted, run the full gate:

1. Run the equal and strength suites on five seeds:

   ```bash
   for seed in 42 1 7 99 2026; do
     target/release/engine-cli calibrate --suite equal --seed $seed --matches 1000 --out runs/gate-equal-$seed
     target/release/engine-cli calibrate --suite strength --seed $seed --matches 1000 --out runs/gate-strength-$seed
   done
   ```

2. Run the formations suite on one seed:

   ```bash
   target/release/engine-cli calibrate --suite formations --seed 42 --matches 1000 --out runs/gate-formations-42
   ```

3. Read `calib.pass` and the failing bands in each report, or the verdict table at the end of standard error.

## Judge a change against the old engine

A baseline diff marks a change as `noise` or `change`, but it does not say whether the run had enough matches to see a change that matters. A change run does: it plays this build and the old engine of a revision on the same fixtures, sizes the run for each band, and judges every band `pass`, `fail`, or `not sure`. Run it from your git checkout:

```bash
target/release/engine-cli calibrate --suite equal --seed 42 --matches 2000 --base main --out runs/change-42
```

The old engine is built once and its results are cached, so the next change run against the same revision plays only the new engine's matches. The run plays a pilot of 200 matches, then grows each suite to the matches that give every band the power to see its smallest shift, at most `--matches`.

Read the table at the end of standard error:

- `pass`: the band is in its range, and the run could have seen a change of the band's smallest shift but found none.
- `fail`: the band left its range, or it moved: the change is too large to be chance. A move is a fail even inside the range; if the move was meant, widen the band or accept the change by review.
- `not sure`: the band is in its range and did not move, but the run had too few matches to tell. The row says about how many matches per suite would give it power: run again into the same folder with that `--matches`.

The last line is the joint verdict. It passes only when every band passes and no match panicked, lost its result, or broke a rule. The exit code is 0 for a joint `pass` and 2 for `fail` or `not sure`. Over every band at once, an engine that did not change fails at most 3 percent of change runs.

To judge a finished run again, for example after editing a band in `realism-bands.json`, run the same command into the same folder. Nothing plays; the run says `judged again from <n> stored rows; 0 matches played` and names the bands that changed since it was last judged.

## Read the stage table

Every run ends with the time and memory of each stage: `play`, `checks`, `commentary`, `writing`, `disk`, `judge`, and `total`. Use it to find where a slow run spends its time before you optimise. The times are thread time summed over the threads, so on 16 threads `play` can read sixteen times the run's wall time. The `checks` time is estimated from every 16th call of the rule checker.

## Resume or grow a run

A long run that stops (Ctrl+C, a closed terminal, a power cut) keeps every finished work unit. Run the same command into the same folder to finish it:

```bash
target/release/engine-cli calibrate --seed 42 --matches 1000 --out runs/base-42
```

The run says `resuming run <run.id>: <done> of <total> fixtures done, <left> to play` and plays only the rest. The report equals the report of a run that never stopped.

To grow a run, run it again into the same folder with a larger `--matches`. The earlier matches keep their fixture keys, engine seeds, and results, and only the new fixtures play:

```bash
target/release/engine-cli calibrate --seed 42 --matches 2000 --out runs/base-42
```

When the program, the content, the flags, the seed, or the minutes changed since the folder's run, the run does not resume. It moves the old files to `superseded/<old run.id>/`, names what differs, and starts again. Do not run two commands into one folder at the same time.

## Run the realistic evaluation

The realistic evaluation is the full run of every default suite at 1,000 matches: 1,000 equal, 1,000 strength, and 1,000 for each of the 55 formation pairings.

```bash
target/release/engine-cli calibrate --seed 2026 --out runs/eval-2026
```

Read `calib.pass` and the band table at the end of standard error. Read the stage table to see where the time went.

## How long the runs take

Time a run on an idle machine: other heavy work on the processor slows every thread. Read the console total and `calib.wall_ms.total` in `report.json`, and note the number of logical processors (`--jobs` defaults to all of them).

Measured on the 8-thread reference machine (AMD Ryzen 7 9800X3D, simultaneous threading off) with other heavy work running:

| Run | Matches played | Time |
|---|---|---|
| `--matches 20`, every suite | 1,140 | 86 to 91 s (about 13 matches per second) |
| `--suite equal --matches 64 --jobs 1` | 64 | 23 s on one thread (about 0.36 s per match) |
| `--suite equal --matches 64 --jobs 8` | 64 | 6 s |

At that rate the realistic evaluation takes about 70 minutes. On an idle machine, one match takes about a third of a second on one thread, so with perfect scaling over 8 threads the evaluation takes about 40 minutes. A change run plays the changed engine's pilot of 11,400 matches and then grows to its power target: with the default cap of 1,000, the formations suite reaches the cap for every pairing, so a change run plays about as many matches as the evaluation.

## Build a faster calibrate binary

Measure before you keep a faster build: run the same pairing with the release build and with the other build, and check that the diff between the two shows a change of exactly 0 on every row. A native CPU target (`RUSTFLAGS="-C target-cpu=native"`) is a local option only: the shipped build runs on other CPUs.

On the 8-core reference machine, a build with whole-program optimisation (`lto = "fat"`, `codegen-units = 1`) and one with a native CPU target gave the same figures as the release build, but neither was faster by more than the spread between two release runs. The release build is the one to use.

A faster rule checker made the same 1,140-match run about 15 percent faster on the same machine, with every result unchanged: the checks fell from about 250 ms to about 145 ms of thread time per match.
