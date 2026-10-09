# Measure a tuning change

This guide shows how to measure one tuning change in about two minutes: make a baseline once, run only the pairing, suite, or band the change targets, and read the diff. Then it shows how to run the full suites before a change is accepted, how to judge a change against the old engine with a change run, how to resume or grow a run, how to read the stage table, and how to run the realistic evaluation.

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

This plays every suite, 57,000 matches: see [How long the runs take](../explanation/realism-harness.md#how-long-the-runs-take). Keep `runs/base-42/report.json`; every targeted run below compares with it.

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

2. Run the formations suite on one seed. Each side starts in its pairing's formation with the lineup its AI manager picks for that formation:

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

## Read the real values beside the verdict

Every run reads each band's real value from the measured real data and prints it beside the verdict. The line before the table says which folder it read and how many bands have no real value:

```text
real data: C:\Users\you\SoccerManager\.scratch\out (default); 0 of 42 bands without a real value; 3 with no real counterpart
```

- `real` is the value measured on real matches; `distance` is the run's value minus it. A band inside its design range can still sit far from real football: the distance says how far.
- `no real data here` means the folder, the file or the key is not there. `calib.real.bands` in `report.json` names the file and the reason.
- `no real counterpart` marks the three bands of the strength and red-card suites: they measure engine experiments that no real match repeats, so they have no real value by design and are never counted as missing.
- `(report only: ...)` at the end of a line marks a band that does not change the verdict: its band file says `gates: false`, its real sample is under its own floor, or its real value comes from too few competitions and seasons.

The last line after the table is the realism claim. Only bands labelled `validation` in the band file count toward it; every shipped band is `calibration`, so today the claim says `none`.

The real values never change a verdict, `calib.pass` or the exit code.

## Read the reported bands

Most bands only report: they are measured and judged, but never change the verdict. Their table follows the verdict table:

```text
reported bands (they never change the verdict):
...
reported: <p> pass, <f> fail, <n> not sure (<a> accepted, not shown equivalent); they never change the verdict
matches: <total> in total; <pairs> pairs; observations: changed engine <n>, old engine <m>
```

Each row has a word by the same rules as a gating band. A change run judges a reported band by the paired test where it can, and by its range on this engine alone where it cannot; the row then says why, for example `range only: base has no measures` when the old engine was built without the measures. A band of the measures (shot distance, passes by kind and length, goals by phase, possession by third, running) needs a build with them: build with `--features measures` (see "Record the realism measures"). The per-club spread, `worst_to_best_spread`, takes each club that plays 34 matches in the run; a run where none does reads `no value: no club has 34 matches in this run`.

The reported bands never move the joint word, its threshold, `calib.pass` or the exit code. Read a `fail` there as a gap between the engine and real football to look into, not as a broken change.

## Accept a not-sure band

A reported or gating band can stay `not sure` however many matches you play, for example a band whose real value is too uncertain to separate pass from fail. Once you have looked at it, accept it for this run:

```bash
target/release/engine-cli calibrate accept goals_poisson_ratio --run runs/change-42
```

Then run the same calibrate command again. Nothing plays; the band's row reads `not sure (accepted, not shown equivalent)`. An accepted band is not shown to match real football: it is never counted as a pass, and it never changes the verdict or the exit code. Only a band that reads `not sure` can be accepted; a `pass` or `fail` band is refused by name. The acceptance belongs to the run in that folder: a new run there (another identity) ignores it, and says `acceptances ignored: ...`.

## Promote a reported band to gate

A band gates once you trust its range: its real value is measured, its range starts at least two real standard errors either side of the real value, and a full run has judged the engine against it. Set its `gates` to `true` in `content/realism-bands.json`, then run a finished run's command again into its folder: the band now sits in the verdict table, and the joint word counts it. Promote a band only by a recorded product-owner decision, as for any band change, and add a Content entry to `CHANGELOG.md` that names the band.

Some promoted bands still do not gate as you might expect:

- A band whose real sample is under its own floor (`real.floor`) stays in the reported table, with `report only: real sample too small to gate (sample <s> under floor <f>)`. The running bands are in this state: their real value comes from 7 tracked matches.
- A band of the measures is judged by its range alone while the old engine of a change run has no measures (`range only: base has no measures`).
- A promotion changes the gate, so run the gate proof again.

## Point at another real-data folder

By default the run reads `.scratch/out` in the main checkout, also from a linked worktree. To read another folder, name it:

```bash
target/release/engine-cli calibrate --suite equal --seed 42 --matches 200 --real-data D:/realism/out
```

A checkout without the measured data, such as a CI runner, reads `no real data here` for every band with a real counterpart, and the run goes on.

## Read the stage table

Every run ends with the time and memory of each stage: `play`, `checks`, `commentary`, `writing`, `disk`, `judge`, and `total`. Use it to find where a slow run spends its time before you optimise. The times are thread time summed over the threads, so on the 8 threads of the reference machine `play` can read up to eight times the run's wall time. The `total` row is the wall time of the whole command. The `checks` time is estimated from every 16th call of the rule checker.

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

## Time a run

1. Stop other heavy work on the machine. Other heavy work slows every thread.
2. Run the command.
3. Read the `total` row of the stage table at the end of standard error, or `calib.stages.total.ms` in `report.json`. It is the time of the whole command in milliseconds, for every kind of run.
4. Note the number of logical processors. `--jobs` defaults to all of them.

Do not use `calib.wall_ms.total` as the time of a run. It is only the time the threads took to play the last list of matches. It leaves out the single-thread benchmark and the judging. In a change run, it also leaves out the old engine and the pilot when the run grows to its power target. In a resumed run, it holds only the matches that this command played.

To compare your time with the measured and projected times of each run, see [How long the runs take](../explanation/realism-harness.md#how-long-the-runs-take).

## Read why a change run plays so many matches

A change run names, for each suite, the band that set its power target. Read it in one of three places:

- the `calibrate.power_target` line on standard error after the pilot, in its `driver` field;
- `drivers` in `target.json` in the run folder;
- `calib.power.<suite>.driver` in `report.json`.

A suite with no driver reached power at its pilot. A driver at the cap (`target` equal to `cap`, `reached` false) is the band to look at when a change run is long: its smallest shift, measured on how rare its event is, decides the run's length. On the default content the equal suite's driver is `goals_per_xg`, at the cap, and the formations suite has no driver: its pooled bands have power at the pilot once the bands far outside their range set no target.

## Read a band that set no target

A band that was already outside its range at the pilot by more than its noise sets no power target. Find it in the same places as the driver:

- `no_target` on the `calibrate.power_target` line;
- `no_target` in `target.json`, by suite code;
- `calib.power.<suite>.no_target` in `report.json`;
- its row in `calib.verdicts`, with `no_target: true` and `outside_by`, its distance from its range;
- its line in the verdict table, which ends with `(outside its range by <distance>, beyond its noise at the pilot: set no power target)`.

Such a band fails while it is outside its range. Read its distance first: the change run says the engine is far from that band, whatever the change did to it. Its `needed` note still says how many matches it would need once it is back inside.

When a change brings the band back inside its range, the next change run's pilot lets it set a target again. The run can then grow, up to the cap, and the band appears as the suite's `driver`. On today's engine the pooled ten-goals-or-more share is the band to watch: inside its range, it needs about 880 matches per pairing.

## Run the sensitivity rules

The sensitivity rules check that every attribute and body job still moves its own statistic, and that a top player still beats an average one in about two of three runs of five matches (see [The player contract](../explanation/player-contract.md#every-rating-has-a-job)). Run them after a change to the attribute file's stage tables, the curve strengths in `engine.contract.actions`, or the match rating:

```bash
cargo test --release -p engine --features sensitivity --test sensitivity --test sensitivity_full -- --ignored --nocapture
```

The rules and the counters they read build only with the engine's `sensitivity` feature, which the command turns on; a normal build has neither, so its matches pay nothing for them. The run plays the balanced design, the two arms and the five-match rule at the size `content/sensitivity.json` sets, about 6,400 matches, a few minutes on an 8-core machine. It also runs the check that a working job passes its rule, a job planted at zero weight fails it and amplified pace does not pass its ceiling, about 2,600 more matches. It prints one line per rule and the five-match shares, and writes the same to `target/sensitivity/rules.json`. The test passes only when every rule and the five-match rule pass.

Read each line:

- **fail** with a move whose interval lies below `min_move`: the job barely moves its statistic. Raise the job's own lever, its main weight in its stage table or its action's `k`, and run again. Never lower the threshold to let it pass.
- **fail** with a share whose interval lies above `ceiling`: the job carries too much of the outcome. Lower its weight or `k`; for pace, its speed amplification.
- **not sure**: the run cannot tell. A rare statistic needs more matches: raise `design.matches` and run again.
- A five-match share above the band means a top player separates too much from an average one: lower the curve strengths by one common factor. Below it, raise them. Do not change the match rating's weights to move it.

A change run (`engine-cli calibrate --base <rev>`) checks the same rules in its rules stage, after the joint verdict, in a build with the feature:

```bash
cargo build --release -p engine-cli --features sensitivity
target/release/engine-cli calibrate --base main --suite equal
```

The stage adds the rules' 6,400 matches to the change run, a few minutes on an 8-core machine. It prints the same table and the line `rules stage: 41 touched sensitivity rules checked at 82 levels, <failed> failed, <not sure> not sure`, and writes the rows to `calib.rules` in `report.json`. The words are information and do not change the joint verdict. A build without the feature prints that it has no sensitivity rules.

A change that moves play still needs the full suites below: a tuning that passes the rules must keep the bands.

## Record the realism measures

The realism measures describe the shape of play beside the bands' averages: where shots come from, passes by kind and length, how goals began, where each side has the ball, and how long its possessions last. They also record how far and how fast each player ran. Build `engine-cli` with the `measures` feature to record them:

```bash
cargo build --release -p engine-cli --features measures
target/release/engine-cli calibrate --suite equal
```

Each match's row then holds, per side:

- the shots in 1 m bins of distance from the centre of the goal attacked, to 40 m and over, penalties included;
- the open-play passes played and completed, by kind (ground pass, long pass or chip, cross) and length (under 10 m, 10 to 25 m, over 25 m), with the keeper's throws and kicks apart;
- the goals by phase: open play, or the corner, free kick, penalty, throw-in, kick-off or goal kick that began the scoring side's possession;
- the open-play ticks with the ball in each third, counted from the side's own goal line;
- the possession chains, by length in passes and in 2-second bins of time;
- each player's ticks on the pitch, metres covered, ball-in-play time in each speed zone (walk under 2 m/s, jog 2 to 4, run 4 to 5.5, high intensity 5.5 to 7, sprint 7 and over) and peak speed.

The measures read the match and change nothing: a seed plays the same match with or without them, and `calib.results_digest` is the same. A build without the feature prints that it has no measures as the run starts, writes zeros in the measure columns, and judges the bands of the band file only. `calib.rows.measures` in `report.json` says which build ran.

## Build a faster calibrate binary

Measure a faster build before you keep it:

1. Run the same pairing with the release build.
2. Run the same pairing with the other build.
3. Check that the diff between the two runs shows a change of exactly 0 on every row.

Use a native CPU target (`RUSTFLAGS="-C target-cpu=native"`) only on your own machine. The shipped build runs on other CPUs.

Unless your measurement shows a gain, use the release build. On the reference machine, no other build was faster by more than the spread between two release runs. See [How long the runs take](../explanation/realism-harness.md#how-long-the-runs-take).
