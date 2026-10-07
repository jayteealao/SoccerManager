# How the realism harness judges a change

This page explains how `engine-cli calibrate` plays its matches and judges a change, and why it is built this way. It is for people who maintain the engine and the harness. For the commands, see [Measure a tuning change](../how-to/calibration.md). For every option and report key, see [the command-line reference](../reference/cli.md#calibrate). For the band registry and the files of a run folder, see [the data-file reference](../reference/data-files.md).

## The question a change run answers

A realism change is a change to the engine or to a tuning value that should make matches look more like real football. Before it is kept, two questions need an answer. Does every realism band stay inside its range? And did the change move a band, by an amount that matters, when it was not meant to?

The second question is the hard one. A run of a few hundred matches can show "no difference" for two reasons: the change did nothing, or the run was too small to see what it did. The harness must tell these apart, because a "pass" that only means "too few matches" lets a broken change through.

## One process, one list of work

All matches of a run play on threads of one process. The fixtures still to play, of every suite, go into one list of work units of at most 8 matches, and each thread takes the next unit when it finishes one.

The earlier design split each suite between worker processes, a fixed share each. A fixed split makes the fast workers wait for the slow ones at the end of every suite, and each process paid its own start-up and wrote a statistics file for every match for the parent to read back. One list with no barrier keeps every thread busy until the list is empty. The threads share the loaded content and write one compact row per match, so nothing is read back.

The worker processes stayed behind a hidden option until the threads gave the same band values on the same fixtures. That proof held on the final engine, and the worker processes were removed. Reports written by earlier builds can still say `calib.runner: processes`; a report of this build always says `threads`.

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
3. The threshold is the 97th percentile of those largest differences. A band whose standardized difference passes the threshold moved, and it fails when that move is also at least its smallest shift: a certain move smaller than the shift that matters is not a change of the band.

Testing each band on its own at 3 percent would raise the false alarms: with 18 bands, a change that did nothing would fail far more often than 3 percent of the time. A fixed correction for many tests, such as dividing 3 percent by the number of bands, goes the other way: the bands move together (goals, shots and expected goals all rise with attacking play), so the correction is stricter than it needs to be and misses real moves. The max-t threshold comes from how the bands actually move together in the run, so an engine that did not change fails at most 3 percent of change runs over all bands at once, and each band still gets its own word. The threshold never falls below the one-band value, so a run whose arms are still identical cannot pass on a zero error.

The joint verdict passes only when every band passes and no match panicked, ended with an engine error, lost its result, or broke a rule.

## Rules are checked while a match plays

The rule checker sees every tick and every event of every match while it plays. It keeps only small state: the team shapes in force, the previous ball position, each player's drift count, and the violations found. The earlier checker stored every tick of the match and checked the list at the end. That cost about 450 MB of memory over eight threads and the time to build the list. The running checker gives exactly the same violations as the earlier one on a fixed set of planted violations and real matches, and it replaced it only after that equivalence held.

Full recordings with events are kept for about 1 in 16 matches, chosen by fixture key, and for every outlier: a rule violation, an error, or a band measure outside the 1st to 99th percentile of the run so far. Every match keeps its compact row either way.

## Where a run's time goes

The stage table of every run splits the time into play, checks, commentary, writing, disk and judging. Play and checks are almost all of it. On the eight-thread reference machine, a match takes about a third of a second of processor time on one thread. After the checker was made faster, play is about three quarters of that and the rule checks about a quarter. Writing rows, the ledger, commentary for the recorded matches and judging the verdict are each well under 1 percent.

The faster checker skips the exact distance of two players who are clearly more than the minimum distance apart, by comparing squared distances first. It still computes the exact distance, with the same call as before, for every pair close enough to matter, so every violation keeps the same value and the same decision.

Every speed change to the engine or the checker must keep every result the same, bit for bit. The replay gate plays fixed matches and compares them with stored hashes, and the checker's equivalence tests compare it with the earlier checker. A change that moves a result is dropped, not made the new truth. A faster run that gives a different answer is not a faster run of the same test.

## What the time limits mean

The two time limits are a change run within 10 minutes and the full evaluation (every default suite at 1,000 matches, 57,000 matches) within its measured time, at most 46 minutes; 15 minutes stays the target of later speed work, which needs about 63 matches per second. They are measured on an idle machine with every logical processor, after the old engine's results are cached. A change run plays only the changed engine: a pilot of 200 matches per suite unit, then growth to the power target.

When a limit is missed, the harness does not relax a rule to meet it. The measured times and the gap go to the person, who decides between more speed work, more threads or machines, or a changed limit.
