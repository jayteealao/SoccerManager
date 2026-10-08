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

This plays every suite: about 76 minutes on an 8-core machine. Keep `runs/base-42/report.json`; every targeted run below compares with it.

For the red-card experiment, make its own baseline:

```bash
target/release/engine-cli calibrate --suite red-card --seed 1 --matches 240 --out runs/base-red-card
```

A report made before sampling errors and fixtures hashes existed cannot be a baseline. Make a new one.

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

2. Run the formations suite on one seed. Each side starts in its pairing's formation with the lineup its AI manager picks for that formation:

   ```bash
   target/release/engine-cli calibrate --suite formations --seed 42 --matches 1000 --out runs/gate-formations-42
   ```

3. Read `calib.pass` and the failing bands in each report.

## Run the sensitivity rules

The sensitivity rules check that every attribute and body job still moves its own statistic, and that a top player still beats an average one in about two of three runs of five matches (see [The player contract](../explanation/player-contract.md#every-rating-has-a-job)). Run them after a change to the attribute file's stage tables, the curve strengths in `engine.contract.actions`, or the match rating:

```bash
cargo test --release -p engine --test sensitivity_full -- --ignored --nocapture
```

The run plays the balanced design, the two arms and the five-match rule at the size `content/sensitivity.json` sets, about 6,400 matches, a few minutes on an 8-core machine. It prints one line per rule and the five-match shares, and writes the same to `target/sensitivity/rules.json`. The test passes only when every rule and the five-match rule pass.

Read each line:

- **fail** with a move whose interval lies below `min_move`: the job barely moves its statistic. Raise the job's own lever, its main weight in its stage table or its action's `k`, and run again. Never lower the threshold to let it pass.
- **fail** with a share whose interval lies above `ceiling`: the job carries too much of the outcome. Lower its weight or `k`; for pace, its speed amplification.
- **not sure**: the run cannot tell. A rare statistic needs more matches: raise `design.matches` and run again.
- A five-match share above the band means a top player separates too much from an average one: lower the curve strengths by one common factor. Below it, raise them. Do not change the match rating's weights to move it.

A change that moves play still needs the full suites below: a tuning that passes the rules must keep the bands.

## Build a faster calibrate binary

Measure before you keep a faster build: run the same pairing with the release build and with the other build, and check that the diff between the two shows a change of exactly 0 on every row. A native CPU target (`RUSTFLAGS="-C target-cpu=native"`) is a local option only: the shipped build runs on other CPUs.

On the 8-core reference machine, a build with whole-program optimisation (`lto = "fat"`, `codegen-units = 1`) and one with a native CPU target gave the same figures as the release build, but neither was faster by more than the spread between two release runs (about 12.5 matches per second each). The release build is the one to use.
