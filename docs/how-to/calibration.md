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

The experiment plays the default clubs with cards otherwise off: a control, and the away keeper, centre-back, or striker sent off at kick-off. Each seed plays twice, once in each home and away order of the two clubs, so each club is the reduced side in half the matches. `--matches 240` plays seeds 1 to 120 in both orders. `calib.red_card` in the report holds each arm's goals for the full and the reduced side, the control, and the verdict. The criterion passes when, in each arm, the reduced side does not outscore the full side and the full side scores at most 1.28 times the control's home goals (the top of the `full_over_control` band: design 0.8 plus its provisional margin 0.48).

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
real data: C:\Users\you\SoccerManager\.scratch\out (default); 0 of 154 bands without a real value; 3 with no real counterpart
```

- `real` is the value measured on real matches; `distance` is the run's value minus it.
- `no real data here` means the folder, the file or the key is not there. `calib.real.bands` in `report.json` names the file and the reason.
- `no real counterpart` marks the three bands of the strength and red-card suites: they measure engine experiments that no real match repeats, so they have no real value by design and are never counted as missing.
- `(report only: ...)` at the end of a line marks a band that does not change the verdict: its band file says `gates: false`, its real sample is under its own floor, or its real value comes from too few competitions and seasons.

The last line after the table is the realism claim. Only bands labelled `validation` in the band file count toward it; every shipped band is `calibration`, so today the claim says `none`.

The real values set each band's word: see [Read a band's word](#read-a-bands-word).

## Read a band's word

Each band's word compares the engine with its target, real football or a design value, by the band's own margin:

- `pass`: the whole interval of the engine value minus the target sits inside the margin. A point inside the margin is not enough.
- `fail`: the whole interval sits outside the margin.
- `not sure`: the interval crosses an edge of the margin, or the run is under the band's minimum exposure, or the band has no target value here. Not sure is never a pass; play more matches or more leagues to narrow the interval.

The table prints each band's interval and margin, and `report.json` holds them on each row (`pass_rule`: the interval's ends, the point, the margin, the target, the exposure). Every band can block a change except the bands of a report-only group of the band file (today the four rare goal and scoreline bands whose real figures are too uncertain to judge).

The headline counts the provisional margins:

```text
margins: 36 provisional, widened to hold the 97 percent floor on today's 3 generated leagues; the floor is set again on 30 leagues
```

`calib.provisional_margins` in `report.json` names them. After it, each report-only band prints its needed margin and whether it would hold now; `calib.report_only_floor` holds the same rows.

## Read the band history and drifts

Each judged run adds one line per band row to `gate/bands/history.jsonl` of the checkout that holds the content folder: the run, its date, build and content, the band, its definition version, value, error, word and matches. Judging a run folder again replaces that run's lines. Set `SM_BAND_HISTORY` to another file to keep the history there, or to `off` to keep none; the tests of the command line point it into each test's own folder.

After the table the run prints the drift section: the bands whose history drifts, each with its direction, the run where the rise began and the run that flagged it. A band's first 5 points of one definition version set its reference, and a two-sided cumulative sum of its later points against that reference flags a drift. `calib.drift` in `report.json` holds the same. A drift never changes a word or the exit code: trace it to the changes made since the run it names.

## Judge the slow tests by the band file

The engine's slow tests read their limits from the band file by band name and judge their own matches with the same rule. A band with a design target needs nothing more. A band with a real target reads the reference file, else the real value of a calibrate report: set `SM_REAL_REPORT` to its `report.json`.

```powershell
$env:SM_REAL_REPORT = "C:\path\to\run\report.json"
cargo test --release -p engine --all-features --test discipline --test tempo_and_restarts -- --ignored
```

Each test prints each band's word, interval and margin, and fails on fail and on not sure.

## Read the reported bands

The bands of a report-only group (today the four rare goal and scoreline bands) are measured and judged, but never change the verdict. Their table follows the verdict table:

```text
reported bands (they never change the verdict):
...
reported: <p> pass, <f> fail, <n> not sure (<a> accepted, not shown equivalent); they never change the verdict
matches: <total> in total; <pairs> pairs; observations: changed engine <n>, old engine <m>
```

Each row has a word by the same rules as a gating band. A change run judges a reported band by the paired test where it can, and by its pass-rule word on this engine alone where it cannot; the row then says why, for example `range only: base has no measures` when the old engine was built without the measures. A band of the measures (shot distance, passes by kind and length, goals by phase, possession by third, running) needs a build with them: build with `--features measures` (see "Record the realism measures"). The per-club spread, `worst_to_best_spread`, takes each club that plays 34 matches in the run; a run where none does reads `no value: no club has 34 matches in this run`.

The reported bands never move the joint word, its threshold, `calib.pass` or the exit code. Read a `fail` there as a gap between the engine and real football to look into, not as a broken change.

## Read the team-shape bands and the figures

A build with the measures also reports how the teams move. These bands only report and never block a change:

- Shape, per position group (`cb`, `fb`, `cm`, `w`, `st`): how far behind the ball each group stands with and without the ball (`cb_depth_in`, `cb_depth_out`), and how wide it stands with the ball on its own side and on the other (`cb_width_near_in`, `cb_width_far_out`).
- Pace, per group: the share of open play spent standing, walking, jogging, running, at high speed and sprinting (`cb_pace_walk_share`), and the mean speed with the ball on the player's side and on the other (`cb_speed_near`).
- Sprints, per group: runs over 7 m/s for at least 1 s, forward, back and lateral, per minute of ball in play (`st_sprints_fwd_per_min`).
- The team: how its block slides with the ball up the pitch and across it, with and without the ball (`slide_up_in`, `slide_across_out`); how far a player 30 to 45 m, or more than 45 m, from the ball follows it over 1 s and 10 s (`far_gain_1s_30_45`); and the share of dead-ball time players spend under 2 m/s (`dead_ball_calm_share`).
- Stoppages per match by kind (`stoppages_throw_in_per_match`) and the minutes of ball in play (`ball_in_play_minutes`).

The reported table groups them by `group` (`shape_cb`, `shape_slide`, `shape_stoppages`, and so on). Each real value comes from `shape3_real.json` in the real-data folder: one value per real match from 7 tracked Bundesliga and 2. Bundesliga matches. With 7 matches the real sample is under the bands' floor of 30, so the bands would only report even with `gates: true`. A checkout without that file reads `no real data here` for them.

The slide up the pitch reads low while the off-ball stands are off, because players then hold their formation places. To see the band with them on, pair the run with the stands flag:

```bash
target/release/engine-cli calibrate --suite equal --seed 42 --matches 200 --pair team_stands
```

The figures follow the reported table:

```text
figures (beside an earlier build; never judged):
figure                       suite           value        se      today     change  build
top_two_receiver_share       equal          0.4711   0.01795      0.414     0.0571  6aa7e09
```

A figure is a number with no real range to judge it by: the share of completed links between each side's two most-linked players, the share with a full-back or centre-back in them, and the engine's passes per team. `today` is the value an earlier build gave, and `change` is this run's value minus it. A figure has no word and never moves a verdict; `calib.figures` in `report.json` holds the same rows. A build without the measures shows the two link figures with no value.

## Read a distribution band

Six reported bands judge a whole distribution: goals per match, the scoreline table, passes per team, and in a measures build shot distance and possession chains in passes and in seconds. Their rows read differently:

```text
goals_distribution       equal      -                 0.84013            0 to 0.42        whole sample    0.84013  fail (upper bound 1.01768, noise floor 0.20984)
```

- The value is a distance: how far the run's whole sample sits from the real sample (Wasserstein-1, in goals here; total variation, a share, for `scoreline_cells`). The distance column repeats it, and the real column reads `whole sample`, because the real side is a whole sample, not one value.
- The range runs from 0 to the band's margin: the largest distance that still counts as the same distribution.
- The upper bound is how far the distance could be, given the matches on both sides. The band passes when the upper bound is within the margin, and fails when even the low end of the distance is beyond it. In between it is `not sure`: play more matches, or accept that the real sample is too small to tell.
- The noise floor is the distance a perfect engine would still show at these sample sizes. When the margin is at or under it the band can never pass, and the row says `margin below noise floor (floor <f>)`. For the shot and chain bands, whose real sample is 200 matches, the floor comes mostly from the real side; more engine matches cannot lower it.
- With no real data here the band has no value: `no value: no real data here: <path>: <reason>`.

`calib.real` gives the mean of each real sample beside its record, and `calib.reported` holds each row's `upper` and `noise_floor`.

## Read the formation field rating

A run of the formations suite that finished every pairing prints, after the reported bands, a rating of each formation against the field:

```text
formation field rating (report only; it never changes the verdict), 2 matches per pairing:
formation       win rate  field rating   weight  nash average  flag
4-4-2             0.0500        0.0000   0.2500        0.0000  -
...
field rating: 0 of 10 formations flagged (field rating above win rate by more than 0.10); it never changes the verdict
```

- The win rate is the formation's share of wins over every formation of the field, itself included, each counted once. A draw is not a win.
- The field rating is the same share of wins, but against the field as its Nash equilibrium weighs it: formations no equilibrium plays weigh nothing, and copies of one style share their weight. The weight column gives each formation's weight in that equilibrium.
- The Nash average is wins minus losses against that field. It is 0 for a formation the equilibrium plays and at most 0 for one it does not.
- `flagged` marks a formation whose field rating exceeds its win rate by more than 0.10: it loses mostly to formations the field does not play, and its win rate undersells it.

The rating never changes a band, the verdict, `calib.pass` or the exit code. Read a flag as a question about the formations, not as a failure. With few matches per pairing each share is 0, 0.5 or 1, and the equilibrium can jump from run to run: read a flag only from a run with many matches per pairing. A run that played only some pairings (`--pairing`), or in which a pairing finished no match, prints `formation field rating: needs every pairing (<k> of <n> played)` instead and writes no `calib.nash`.

## Read the substitutions against the real eras

Each run reports the AI managers' substitutions in `calib.substitutions` of `report.json`, against the real era of the rule pack's limit from the `substitutions` rows of `signatures.json`: the Wyscout leagues of 2017/18 for a limit of 3, and Euro 2020, the 2022 World Cup and Euro 2024 for a limit of 5. To read the three-substitution era, play under its pack:

```text
target/release/engine-cli calibrate --suite equal --seed 9 --matches 200 --rules crates/engine-cli/tests/fixtures/rules/three-substitutions.json --out runs/subs-three-9
```

- `per_team` is the substitutions a team makes in a match, and `per_team_inside` whether it lies inside the real competitions' range.
- `minute_quartiles` are the quartiles of the substitution minutes, and `quartiles_inside` whether each lies inside the real range of that quartile.
- `reasons` splits the substitutions by `ai-decision` code; `injury_knock_share` is the share made for an injury or a knock, beside the real share forced by an injury.
- `count_shares` gives the share of teams making 0, 1, 2 and more substitutions.

The report never changes a band, the verdict, `calib.pass` or the exit code.

## Run the styles suite

The styles suite plays the eight named styles of `content/styles.json` against each other: every ordered pairing of two different styles, with the same squad on both sides and a formation drawn from each style's list. It judges no band; it tells you whether a style dominates and whether each style shows its identity. Run it on demand, for example before a release or after a change to how instructions play:

```bash
target/release/engine-cli calibrate --suite styles --seed 42 --matches 50 --out runs/styles-42
```

`--matches` is per ordered pairing, so 50 plays 2,800 matches (56 pairings). It runs alone: it refuses `--base`, `--pair`, `--baseline`, `--band` and `--pairing`. The report goes to `runs/styles-42/report.json` under `calib.styles`, and standard error ends with the table:

```text
styles suite: 2800 of 2800 matches played, 50 per ordered pairing, 73 rule violations
style            pts/match     se   share  clear losses
possession           2.706  0.030   0.239  none  DOMINANT
low_block            0.923  0.049   0.082  possession, high_press, gegenpress
...
dominant: possession
identity flags: possession: possession_pct; possession: passes; ...
```

Read it this way:

- `pts/match` is the style's points per match (3 for a win, 1 for a draw), with its standard error; `share` is its share of all the points of the suite. With eight styles an even share is 0.125.
- `clear losses` names the styles it loses to clearly: over their matches in both home orders, the 95 percent interval of its points less the other's lies wholly below 0.
- `DOMINANT` marks a style with no clear loss, or with more than 20 percent of all the points. Every style should have at least one clear weakness; a dominant style is a style the AI and the player could always pick.
- `identity flags` lists each statistic a style names in `styles.json` that its sides do not move in its direction by more than 1.96 standard errors from the mean over every side, for example a possession style that does not have more of the ball. In `calib.styles.styles[].identity`, each row gives `style_mean`, `all_mean` and `se`, so you can see which way the statistic moved.
- `calib.styles.pairings` gives each ordered pairing's points, goals and shots per match: look there for a pairing that plays far from a real match.
- The rule violations count every rule the running checker found broken over the suite. A match the equal suite never plays, such as two pressing styles against each other, can show a fault there.

A flag on a small run can be noise: read the flags from a run with at least 50 matches per ordered pairing, where every pair of styles has played 100 matches with each side at home 50 times.

To look into one pairing, play only it with `--style-pairing`, the home style first; a style may meet itself. Build with the `measures` feature to get each side's probe:

```bash
cargo build --release --locked -p engine-cli --features measures
target/release/engine-cli calibrate --suite styles --style-pairing "counter_attack v gegenpress" --style-pairing "high_press v high_press" --seed 9 --matches 100 --out runs/pairings-9
```

Each pairing then prints its shots and goals a match, and a `home:` and an `away:` line: per side, its shots, goals, xG per shot and shot distance, its possessions and turnovers, where it loses the ball (its own, the middle or the final third), its fast breaks and the time from a regain to a shot, its pressures and what it does in the 5 to 10 seconds after a loss, its offsides, and where the ball spends its time. The same figures are in `calib.styles.pairings[].probe`. To find what drives a pairing, change one setting of one style in a copy of the content folder (`--content-dir`) and play the pairing again: the other figures stay on the same matches.

### Run the styles gate

The gate judges the whole table at once. Run it at 200 matches per ordered pairing, the size at which every pair of styles has played 400 matches and a clear loss can show (11,200 matches):

```bash
target/release/engine-cli calibrate --suite styles --seed 9 --matches 200 --out runs/styles-gate-9
```

The table on standard error ends with one line:

```text
styles gate: pass
```

or, when an item fails, `styles gate: fail: ` and each failing item. Read each item this way:

- a dominant style: it loses clearly to no style, or takes more than 20 percent of the points;
- an identity flag (`style: stat`): the style's sides do not move that statistic in its direction;
- a pairing outside the range (`home v away` and its shots a match): both sides together average under 4 or over 40 shots a match;
- violations: the running checker found a rule broken in one of the suite's matches;
- unplayed: a planned match did not play.

The same items are in `calib.styles.gate` of `report.json`, with `pass`. The gate never changes the exit code. To find the cause of a failing item, play the pairings it names with `--style-pairing` in a `measures` build, as above, and change one setting at a time in a copy of the content folder. A change to a preset that only makes the gate pass is not a fix: look for the mechanism that makes the pairing play unlike a real match.
Every change run (`--base` or `--base-binary`) also plays a small sample: one match per ordered pairing, 56 matches, on the changed engine only, after its verdict. The same table prints on standard error, the report holds it under `calib.styles`, and `calib.stages.styles` gives its time. With one match per ordered pairing, a style shows a clear loss only when it loses both matches against another, so the sample flags styles as dominant by chance: read it for the pairings' goals and shots and the rule violations, not for the flags. The sample never changes the verdict.

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
- A promotion changes the gate, so [run the gate proof](#run-the-gate-proof) again.

## Run the gate proof

The gate proof measures how often the gate passes an unchanged engine and how often it catches a broken one. Run it again whenever the gate changes: after a promotion or any other band edit, a change of the realism measures, or a change of the judging code. Until then, every calibrate run prints `gate changed since the last proof: <part> <proof value> -> <current value>` for each part that differs.

Build `engine-cli` with the measures, then start the proof on an idle machine:

```bash
cargo build --release -p engine-cli --features measures
target/release/engine-cli calibrate proof --seed 2027
```

It plays about 30 change runs of this build against itself: 10 self runs on new seeds, and 5 runs of each of the four broken copies that `content/gate-proof.json` declares. On an 8-thread machine it takes several hours. Each run's log is in `<out>/<run>.log`, and the default `<out>` is `SM_DATA_DIR/proofs/<registry>-<verdict code>`. If the proof stops, run the same command again: each finished run is judged again from its stored rows, and nothing plays twice. When it ends, it writes the rates into `content/gate-proof.json`. Commit that file, with a Content entry in `CHANGELOG.md`.

A build without the measures refuses the proof with `this build has no measures`, because only the measures build judges every whole-distribution band.

## Read the gate proof

The proof prints its summary and writes it under `proof` in `content/gate-proof.json`:

- `gate`: the band registry digest, the measures version and the judging-code digest that the proof measured. A calibrate run compares them with its own gate.
- `self`: the 10 self runs, each with its joint word, the bands that read fail or not sure, the bands whose change passed the joint threshold (`significant`), and the bands that moved (`moved`: past the threshold and at least their smallest shift). Then the pass, fail and not-sure rates by joint word, the `false_alarm_rate` (runs with any significant band) and the `move_rate` (runs with any moved band). A `not sure` run is not a pass. A band outside its range reads `fail` in every run, the self runs included, so while the engine sits outside some ranges the pass rate is 0 whatever the gate does: read the false-alarm rate for the gate itself.
- `copies`: for each broken copy, its fail rate (runs whose joint word is `fail`), its `detection_rate` (runs in which the band it names moved), `missed` (runs in which no band moved), and `moved` (each band that moved, with the number of runs it did). A copy with misses is a broken engine that the gate did not catch every time.
- `distribution_bands`: for each whole-distribution band, the copies that moved it. An empty list means that no copy moved it, so the proof does not show that this band catches a broken engine.
- `counts`: the total matches, the pairs, and the matches of each engine over every run.

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

The rules and the counters they read build only with the engine's `sensitivity` feature, which the command turns on; a normal build has neither, so its matches pay nothing for them. The run plays the balanced design, the two arms and the five-match rule at the size `content/sensitivity.json` sets, about 6,400 matches, a few minutes on an 8-core machine. It also runs the check that a working job passes its rule, a job planted at zero weight fails it and amplified pace does not pass its ceiling, about 2,600 more matches. It prints one line per rule and the five-match shares, and writes the same to `target/sensitivity/rules.json`. A rule with the report-only flag prints `report` after its word: the rules of the jobs the newer attributes and the personality values brought run and report, and their words do not fail the run. The test passes when every rule without the flag and the five-match rule pass.

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

The stage adds the rules' 6,400 matches to the change run, a few minutes on an 8-core machine. It prints the same table, with `report` after the word of a report-only rule, and the line `rules stage: 62 touched sensitivity rules checked at 124 levels, <failed> failed, <not sure> not sure`, and writes the rows to `calib.rules` in `report.json`. The words are information and do not change the joint verdict. A build without the feature prints that it has no sensitivity rules.

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
- each player's ticks on the pitch, metres covered, ball-in-play time in each speed zone (walk under 2 m/s, jog 2 to 4, run 4 to 5.5, high intensity 5.5 to 7, sprint 7 and over) and peak speed;
- the team-shape sums behind "Read the team-shape bands and the figures": depth, width, pace and sprints per position group, the slide, the far players' response, dead-ball calm, the stoppages by kind and the completed links (`docs/reference/data-files.md` lists them).

The measures read the match and change nothing: a seed plays the same match with or without them, and `calib.results_digest` is the same. A build without the feature prints that it has no measures as the run starts, writes zeros in the measure columns, and judges the bands of the band file only. `calib.rows.measures` in `report.json` says which build ran.

### Write the decision probe

A measures build also keeps a decision probe of each match: kicks by kind and third, the clearances the other side gained next, goals with and without a shot behind them, the carrier's option scores in the final third he attacks with the terms of his best pass, every open-play shot with its expected goals, the keeper's distance off his line and the shooter's execution share, and the balls into the box: each pass and restart kick by kind (an open-play lofted pass, a lofted restart kick, a kick along the ground) and those the next player to gain the ball gains inside the penalty area the kicker's side attacks, as a StatsBomb pass ends there. Set `SM_PROBE_OUT` to a file path and a calibration run appends one JSON line per match to it, with each keeper's rating, post-shot expected goals faced, goals from shots conceded, saves, completed passes and clearances:

```bash
SM_PROBE_OUT=runs/probe.jsonl target/release/engine-cli calibrate --suite equal --matches 200
```

The probe reads the match and changes nothing, like the other measures.

## Check and fit a tactical primitive

A tactical primitive is a setting of the tactics board, such as tempo or mentality, declared in the `primitives` block of `content/tactics.json`. Each one names the statistic it should move. To check that every level step moves it, build with the `measures` feature and run the harness:

```bash
cargo build --release -p engine-cli --features measures
target/release/engine-cli --content-dir content primitives check --primitive tempo --pairs 200 --out runs/tempo-check.json
```

Each step plays 200 pairs of matches, the tested side one level apart on the same seed. Standard error lists any failed step with its numbers, and the run exits with 2. A step is judged on its primary statistic, the first in its `signature` list; a secondary statistic that does not move beyond the noise prints as a `secondary miss` and does not fail the step. The noise of 40 pairs often covers a real step, so use 200 pairs before you trust a failure.

To fit one level to real teams, read the real-team table and search the level's value:

```bash
target/release/engine-cli --content-dir content signatures
target/release/engine-cli --content-dir content primitives fit --primitive tempo --level slow --matches 200 --out runs/tempo-slow.json
```

The fit aims at a ratio, not at the real value itself: the ratio of the level's quintile median to the middle median in `signatures.json`, against the ratio of the engine's level to its default level. The engine plays more passes than real teams, so the absolute values differ; the result names both, and the gap belongs to the tuning of pass volume. Copy the fitted `search.value` into the level in `tactics.json` and the record into the primitive's `fits`; the load refuses a fit record whose values differ from the level's.

The fit searches the level's first input in its `inputs` list and keeps the others at their content values. For width, that is the wide-target weight; the shape scale keeps its content value.

### Check a choice

A choice primitive, such as the attacking channel, has no steps. `check` plays each choice against the default choice and judges the choice's own statistic, which must rise beyond the noise:

```bash
target/release/engine-cli --content-dir content primitives check --primitive attacking_channel --pairs 200 --out runs/channel-check.json
```

The result holds each choice's means and paired differences for every statistic. To check that the channels mirror, compare the rise of `entries_left_share` under `left` with the rise of `entries_right_share` under `right` in that file: they should differ by no more than their combined noise. A primitive whose `data` names a `deferred` part prints its numbers for that part too, but they judge nothing until the deferral ends.

### Check a player instruction

A player instruction sets one primitive for one player (see the game-state section of the tactics file in `docs/reference/data-files.md`). `--player <slot>` checks it the same way, for the player in that slot of the tested side's 4-4-2: one level up from the default against no instruction, or each choice against none. The player's own statistic must rise beyond the noise, and the same statistic pooled over his ten team-mates must stay inside it:

```bash
target/release/engine-cli --content-dir content primitives check --primitive dribbling --player 5 --pairs 1000 --out runs/dribbling-player.json
```

Check each primitive on a slot its hook reaches: a striker (slot 9 or 10) for forward runs and the final third, a winger (slot 5 or 8) for dribbling, crossing, cross type and width, and a central midfielder (slot 6 or 7) for pressing, tackling and the two transition primitives. A single player's counts are small, so use 1,000 pairs. Forward runs are refused for a slot off the attack line.

### Fit a thin-sample primitive

Full-back run frequency fits to the off-ball runs of 13 SkillCorner teams, and forward runs to the runs of the 32 teams of the 2022 World Cup's tracking data: too few, or national teams rather than clubs, for quintiles. Their rows in `signatures.json` carry `thin_sample: true` and five targets spread evenly from the lowest to the highest team; the World Cup rows also carry `national_teams: true`. Run `fit` as for any level; it aims at the ratio of the level's target to the middle target. Full-back runs are rare in the engine, so fit their frequency with `--matches 1000`.

A primitive may judge its lower and upper levels on different statistics: a signature entry with `"steps": "below_default"` or `"above_default"` judges only the steps on that side of the default. Forward runs judge their upper levels on the share of runs in behind and their lower levels on the share of runs coming short. The fit searches each level on its own side of the default, and a statistic that falls as the level rises takes its targets from the other end of the real spread: the lowest level aims at the highest team.

### Make the forwards' run rows

The run rows come from tracking data, not from a provider's run events. `scripts/tracking-runs.py` finds each forward's runs in the frames of the PFF World Cup 2022, SkillCorner, IDSSE and Metrica matches by the engine's own definition (4 m/s or faster for 1 s or more, his side with the ball, not the carrier):

```bash
python scripts/tracking-runs.py <tracking data folder> <real data folder>/tracking_runs.csv.gz
```

`engine-cli signatures` then classifies each run as the engine does, makes the rows from the World Cup teams and writes the club sources as checks. The Alfheim matches are not read: they track one team only, with no ball and no opponents, so they have no defensive line and no possession.

## Build a faster calibrate binary

Measure a faster build before you keep it:

1. Run the same pairing with the release build.
2. Run the same pairing with the other build.
3. Check that the diff between the two runs shows a change of exactly 0 on every row.

Use a native CPU target (`RUSTFLAGS="-C target-cpu=native"`) only on your own machine. The shipped build runs on other CPUs.

Unless your measurement shows a gain, use the release build. On the reference machine, no other build was faster by more than the spread between two release runs. See [How long the runs take](../explanation/realism-harness.md#how-long-the-runs-take).
