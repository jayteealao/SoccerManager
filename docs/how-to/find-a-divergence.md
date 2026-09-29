# Find where two engine versions part

This guide shows how to find the first tick where two versions of the engine play one recorded match differently, and how to read what bisect reports. Use it when a change moves the replay-gate hashes and you need to know where, or when a bug report names a match that plays differently on a newer build.

Bisect re-simulates one replay file on both versions and compares a digest of the full match state after every tick. At the first tick whose state differs, it shows the parts of the state that differ, with both values, and both versions' debug trace records for that tick. Every flag and every reason is in [the command-line reference](../reference/cli.md#bisect).

Only builds that have the state-digest options of `resimulate` can be compared. A build from before them is reported as incomplete, not compared.

The commands use the release build. Build it first, from the repository root:

```bash
cargo build --release --locked -p engine-cli
```

## Record the match

Bisect needs a version-4 replay file: a file that holds every input of the match. Record one with the build you have, for example one minute of seed 42 with a change file:

```bash
target/release/engine-cli record --seed 42 --minutes 1 --changes changes.json --out match.smfx
```

A replay file from a bug report works as well, when it is version 4 or later. A version-3 file holds only frames, and bisect refuses it before it builds anything.

## Compare two commits

Name each version by a commit that git can resolve:

```bash
target/release/engine-cli bisect --fixture match.smfx --a HEAD~1 --b HEAD
```

Bisect builds each commit in its own git worktree under the bisect build cache (`target/bisect/` by default), and never switches your checkout. The first build of a commit takes minutes; a second run with the same commit, toolchain, profile, and features re-uses the cached executable and says `reused`. Use `--profile` and `--features` to build the commits another way; each combination is cached on its own.

To compare with a build you already have, such as a build with uncommitted changes, give its path instead:

```bash
target/release/engine-cli bisect --fixture match.smfx --a HEAD~1 --b-binary target/release/engine-cli
```

## Read the report

A report that finds a difference starts with the tick and exits with code 2:

```text
bisect: differs at tick 2000
replay file: match.smfx
a: HEAD~1 = commit 4c1e…, executable sha256 9a0b…, reused
b: HEAD = commit 7d22…, executable sha256 51fe…, built
stream scheme: 1 (a), 1 (b)
first differing tick: 2000
differing fields:
  ball.vel
    a: [3.1,0.4,0.0]
    b: [3.100000001,0.4,0.0]
debug trace of a at tick 2000: 41 records
  …
```

Read it in this order:

1. **The tick.** Every tick before it has the same full state on both versions, so the cause acted during this tick's step.
2. **The fields.** Each named part of the state that differs after the tick, with both values: the ball, each player's position, speed, target, and energy, the referee's clock and phase, the pending changes, the random streams, and the tick's events. The first part in the list is usually closest to the cause. A `streams` entry names the random streams whose positions differ: one version took more or fewer draws from them.
3. **The debug traces.** Both versions' draws, decisions, and rule outcomes for the tick, in the order the engine ran them. Compare them line by line; the first record that differs is where the two versions chose differently.
4. **The schemes.** When the two versions use different stream schemes, the report says so. Every stream position then differs from the first draw, and the first differing tick is usually tick 1.

`no difference` (exit code 0) means that the full state is the same after every tick and after full time.

With `--json`, the same report is one JSON object, for a script to read.

## Act on an incomplete result

`bisect: incomplete` (exit code 3) means that no comparison was made. It never means "no difference". Each side with a problem has its own line:

- **`the build of <commit> failed`**: the commit does not build with its pinned toolchain. The last 40 lines of cargo's output follow. Pick a commit that builds, or build it yourself and pass `--a-binary`.
- **`lacks the resimulate command`** or **`lacks the state-digest options`**: the build is older than the tools bisect needs. Compare a newer commit.
- **`crashed or failed`**: the build stopped with an error while it re-simulated the file. The last line of its error output follows. Run the same `resimulate` command yourself to see all of it.
- **`timed out`**: the run took longer than `--timeout` (1,800 seconds by default) and was stopped.
- **`ends early`** or **`a gap in the state digests`**: the build wrote an unfinished state digest file. Check the disk space and run again.
- **`the two builds hash different state inventories`**: the state inventory version changed between the two versions, so their states cannot be compared tick by tick.

## Clear the cache

The cache holds one executable per commit, toolchain, profile, and feature set, and a shared cargo target folder. Each cached executable is checked against its recorded SHA-256 before it is used. To free the space, delete the folder:

```bash
rm -rf target/bisect
```

`cargo clean` deletes it as well.

## Look further with debug mode

To see more than one tick, write the whole debug trace of a re-simulation, up to any tick:

```bash
target/release/engine-cli resimulate --fixture match.smfx --compare --debug-trace match.trace.jsonl
```

Run the same command with each version's executable and compare the two traces around the reported tick. To see the named state of any one tick, add `--state-fields fields.json --at-tick 2000`. A new match with debug mode on comes from `simulate --debug-trace`. The trace format is in [the debug trace reference](../reference/cli.md#debug-trace-file).
