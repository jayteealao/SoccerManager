# How the realism harness judges a change

This page explains how `engine-cli calibrate` plays its matches and judges a change, and why it is built this way. It is for people who maintain the engine and the harness. For the commands, see [Measure a tuning change](../how-to/calibration.md). For every option and report key, see [the command-line reference](../reference/cli.md#calibrate). For the band registry and the files of a run folder, see [the data-file reference](../reference/data-files.md).

## The question a change run answers

A realism change is a change to the engine or to a tuning value that should make matches look more like real football. Before it is kept, two questions need an answer. Does every realism band stay inside its range? And did the change move a band, by an amount that matters, when it was not meant to?

The second question is the hard one. A run of a few hundred matches can show "no difference" for two reasons: the change did nothing, or the run was too small to see what it did. The harness must tell these apart, because a "pass" that only means "too few matches" lets a broken change through.

## One process, one list of work

All matches of a run play on threads of one process. The fixtures still to play, of every suite, go into one list of work units of at most 8 matches. Each thread takes the next unit when it finishes one. One shared list keeps every thread busy until the list is empty: no thread waits for a slower thread when a suite ends.

## A fixture is named by what it is

Each fixture has a key: a hash of what the fixture is. The inputs are the fixture scheme, the scenario, the generated world, the two clubs in home and away order, both formations, and a repeat number. The engine seed comes from the same hash and is stored beside the key.

Earlier, a run numbered its fixtures and derived seeds from the numbers. A larger run renumbered the formation pairings, so it played other matches, and a stopped run had no way to know which matches it had already played. With keys from content, a match is the same match in every run that plays it:

- A run that grows to more matches adds repeats. The earlier matches keep their keys, seeds and results.
- A stopped run resumes. Each finished unit has a line in the run's ledger, written only after every match of the unit is on disk, so a unit cut off part-way plays again and a finished unit never plays twice.
- The old engine's results are cached by build, content, fixture key and seed. A change run against the same old engine reads them instead of playing them again.

A run folder also records the run's identity: the build, the content, the flags, the seed, the minutes, the random-number scheme, the fixture scheme, the measure definitions and the registry version. A run with another identity does not resume the folder; it starts again and names what differs. The band ranges are not part of the identity: a changed range is judged again from the stored rows, with no match played.

## A band that has not reached its power is not sure

Each band in the registry names its smallest shift: the smallest change of the band that matters. A change run plays a pilot of every suite, measures how much each band varies from match to match, and works out how many matches each suite needs to see a band's smallest shift with 80 percent power. It then grows each suite to that count, at most the cap the person sets.

A band that reaches its power and shows no move can pass. A band that does not reach its power reports **not sure**, never **pass**. This is the rule that makes the 10-minute change run honest: when time or the cap runs out before a band has its power, the answer is "not sure", and the report says how many matches the band would need. No speed method may reach the limit by cutting this certainty.

## One joint test for all bands

A change run compares the changed engine with the old engine on the same fixtures, so the two arms share their match-to-match noise. The harness tests every band at once with a max-t bootstrap over these paired matches:

1. For each band, the difference between the arms is divided by its standard error, which gives a standardized difference.
2. The run draws 1,999 resamples of its paired matches, within each suite, pairing and red-card arm. In each resample, each band's difference is centred on the observed difference and divided by that resample's own standard error, and the largest of these over all bands is kept.
3. The threshold is the 97th percentile of those largest differences. A band whose standardized difference passes the threshold moved. The band fails when that move is also at least its smallest shift. A certain move smaller than the shift that matters is not a change of the band.

Testing each band on its own at 3 percent would raise the false alarms. With 18 bands, a change that did nothing would fail far more often than 3 percent of the time. A fixed correction for many tests goes the other way. One such correction divides 3 percent by the number of bands. But the bands move together: goals, shots and expected goals all rise with attacking play. So a fixed correction is stricter than it needs to be, and it misses real moves. The max-t threshold comes from how the bands actually move together in the run. So an engine that did not change fails at most 3 percent of change runs over all bands at once. Each band still gets its own word. The threshold never falls below the one-band value, so a run whose arms are still identical cannot pass on a zero error.

The joint verdict passes only when every band passes and no match panicked, ended with an engine error, lost its result, or broke a rule.

## Rules are checked while a match plays

The rule checker sees every tick and every event of every match while it plays. It keeps only small state: the team shapes in force, the previous ball position, each player's drift count, and the violations found. The earlier checker stored every tick of the match and checked the list at the end. That cost about 450 MB of memory over eight threads and the time to build the list. The running checker gives exactly the same violations as the earlier one on a fixed set of planted violations and real matches, and it replaced it only after that equivalence held.

Full recordings with events are kept for about 1 in 16 matches, chosen by fixture key, and for every outlier. An outlier is a match that failed, panicked or left a change unapplied, or a match with a rule violation. A match is also an outlier when a measure lies strictly outside the 1st to 99th percentile of its suite's earlier matches. This percentile rule starts only once the suite has 100 earlier matches in the same session, which is one command's share of the run. A resumed run starts a new session, so the count of earlier matches starts again. Every match keeps its compact row either way. The [command-line reference](../reference/cli.md#calibrate) lists the measures of the percentile rule.

## Where a run's time goes

The stage table of every run splits the time into play, checks, commentary, writing, disk and judging. Play and checks are almost all of it. On the 8-thread reference machine at idle, a match takes about a third of a second of thread time. About three quarters of that time is play, and about a quarter is the rule checks. Writing rows, the ledger, commentary for the recorded matches and judging the verdict are each well under 1 percent.

The faster checker cut the checks from about 250 ms to about 145 ms of thread time per match. That pair comes from the 1,140-match run on the reference machine, with other heavy work running. The whole run became about 15 percent faster, with every result unchanged.

The faster checker skips the exact distance of two players who are clearly more than the minimum distance apart, by comparing squared distances first. It still computes the exact distance, with the same call as before, for every pair close enough to matter, so every violation keeps the same value and the same decision.

Every speed change to the engine or the checker must keep every result the same, bit for bit. The replay gate plays fixed matches and compares them with stored hashes, and the checker's equivalence tests compare it with the earlier checker. A change that moves a result is dropped, not made the new truth. A faster run that gives a different answer is not a faster run of the same test.

## Why a change run pools the formation pairings

The formations suite plays every pairing of the formations in `tactics.json`: 55 pairings with the shipped file. A realistic evaluation checks each pairing against the band's range, because a formation that breaks the game should show up on its own. A change run asks a different question: did this change move the measure? Judged per pairing, each of the 55 rows needs its own power, and its own matches, to see the band's smallest shift. Pooled over every pairing, the same matches give one row with about a seventh of the error (one over the square root of 55), so the suite reaches power with far fewer matches per pairing.

So a change run judges and powers each formations band once, pooled over every pairing it plays. The resamples of the max-t test still stay within each pairing, so the pooled error keeps the pairings apart. The range check of each pairing stays in the report as information. It sets no target and does not enter the joint verdict, but it still shows a change that moves one pairing one way and another pairing the other way, which a pooled row can hide. A run without an old engine judges every pairing as before.

## Why a rare event can set a run's length

A band sets a suite's power target only while the band is inside its range, or outside the range by no more than its noise. A band far outside its range sets no target, as [the next section](#why-a-band-far-outside-its-range-sets-no-target) explains. Within that condition, a rare event can set the length of a whole run.

A band needs matches in proportion to the square of its error divided by its smallest shift. For a share of rare matches, the error of the share is about the square root of the share over the matches. So a small shift on a rare event needs many matches. The ten-goals-or-more share is the clearest case. Its range is 0 to 0.5 percent, and its smallest shift is 0.125 percentage points. At a realistic 0.4 percent, inside its range, the band needs about 880 matches per pairing, even when pooled. The cap of 1,000 holds every other formations band many times over.

In the pilot of a measured change run, the engine produced such matches at 2.5 percent. At that share the band would need about 5,000 matches per pairing. But 2.5 percent is far outside the band's range, so a band at that share sets no target.

The band that sets a suite's target is named beside the target, in `calib.power`, in `target.json`, and on the `calibrate.power_target` line. A change run that plays far more matches than expected shows which band asked for them. Lowering that need means a different smallest shift, a band judged in another suite, a different rule for which bands set a target, or a longer run. Each is a decision about what the test must detect, not a speed-up, so the harness does not take one on its own.

## Why a band far outside its range sets no target

A band's power target asks how many matches it needs to see its smallest shift. That question matters only while the band could pass. A band whose value is outside its range fails, whatever its change, so playing more matches to measure its change precisely buys nothing for its word. When that band is also the one with the largest need, it alone would set the run's length. So in every suite of a change run, a band already outside its range by more than its noise sets no target.

"More than its noise" uses the run's own measure of a real move. At the pilot, the band's distance from its range must be more than `c` times the band's own standard error, where `c` is the max-t threshold the run already uses to call a change real (at least 2.17). So the rule adds no new constant, and a band that is only just outside, where the pilot's noise could have put it there, still sets a target.

The rule changes how long the run is, never a word. A band that set no target is judged like every other: it fails while it is outside its range, and a band without power never passes. If the pilot's noise made a band look far outside when it is not, the worst outcome is `not sure` for that band, never a false `pass`. The run reports the band, its distance, and the mark, so a person sees which bands did not size the run, and the driver shows when such a band comes back inside its range and sets the target again.

## How long the runs take

Every figure below comes from the 8-thread reference machine: an AMD Ryzen 7 9800X3D with simultaneous threading off. Each row names its load and says whether a timed run measured it or a calculation projected it. Under load, other heavy work kept the processor busy during the run.

| Run | Matches played | Load | Kind | Time |
|---|---|---|---|---|
| `--matches 20`, every suite | 1,140 | other heavy work running | measured | 86 to 91 s (about 13 matches per second) |
| `--suite equal --matches 64 --jobs 1` | 64 | other heavy work running | measured | 23 s on one thread (about 0.36 s per match) |
| `--suite equal --matches 64 --jobs 8` | 64 | other heavy work running | measured | 6 s |
| Realistic evaluation | 57,000 | other heavy work running | projected | about 70 minutes |
| Realistic evaluation | 57,000 | idle | projected | about 39 to 46 minutes |
| Change run, warm cache | 12,278 of the changed engine | idle | projected | about 8.4 to 9.9 minutes |

The loaded projection of the realistic evaluation uses the measured rate of about 13 matches per second. The idle projections use about a third of a second of thread time per match over 8 threads. The lower idle figure assumes perfect scaling over the threads, and the higher figure assumes 85 percent scaling.

A change run on a warm cache plays only the changed engine's matches. It plays a pilot of 11,400 matches, then grows each suite to its power target. On today's engine and the default content, the pilot sets three targets: equal 1,000, formations 200 per pairing, and strength 278. The formations suite stays at its pilot because the run judges it pooled over every pairing. The formations bands far outside their range also set no target. So a change run plays 12,278 of the changed engine's matches.

On the reference machine, two other builds gave the same figures as the release build. One used whole-program optimisation (`lto = "fat"`, `codegen-units = 1`), and one used a native CPU target. Neither was faster by more than the spread between two release runs.

## What the time limits mean

The two time limits are a change run within 10 minutes and the full evaluation (every default suite at 1,000 matches, 57,000 matches) within its measured median time, which must be at most 46 minutes. Later speed work keeps 15 minutes as the evaluation's target, at about 63 matches per second. Both limits apply to a run on an idle machine with every logical processor, after the old engine's results are cached. A change run plays only the changed engine: a pilot of 200 matches per suite unit, then growth to the power target.

When a limit is missed, the harness does not relax a rule to meet it. The measured times and the gap go to the person, who decides between more speed work, more threads or machines, or a changed limit.
