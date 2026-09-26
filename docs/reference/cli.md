# Command-line reference

`engine-cli` is the headless command line of the match engine. The release build is `target/release/engine-cli` (`engine-cli.exe` on Windows).

```text
engine-cli [--content-dir <DIR>] <COMMAND> [OPTIONS]
```

`engine-cli --help` lists the commands. `engine-cli <COMMAND> --help` lists the flags of one command. `engine-cli --version` prints the version.

## Global flags

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--content-dir` | folder | see below | The folder that holds the content files. Every command accepts it. |
| `--help`, `-h` | none | | Print the help. |
| `--version`, `-V` | none | | Print the version. |

When `--content-dir` is absent, the engine uses `SM_CONTENT_DIR`. When `SM_CONTENT_DIR` is not set, the engine uses `./content`, then the `content` folder beside the binary.

## Environment variables

| Variable | Default | Meaning |
|---|---|---|
| `SM_CONTENT_DIR` | not set | The content folder, when `--content-dir` is absent. |
| `SM_DATA_DIR` | `%LOCALAPPDATA%\SoccerManager` on Windows, else `$HOME/.local/share/SoccerManager` | The data folder for every file the engine writes. |
| `SM_ENGINE_PATH` | not set | The engine program `launch` runs, when `--engine` is absent. |
| `SM_WEB_DIR` | not set | The page folder `launch` serves, when `--web` is absent. |
| `SM_LOG` | `info` | The log filter for the log lines on standard error, for example `debug` or `engine=warn`. |
| `SM_ENV` | `dev` | The `env` field of every record. |

## Exit codes

| Code | Meaning |
|---|---|
| 0 | The command completed. |
| 1 | An error stopped the command. Standard error names the cause: a bad flag, a refused content file, a refused snapshot, or a file that cannot be written. |
| 2 | The command completed but a check failed, or the match did not reach full time. The command sections below name the check. |

## simulate

Simulate one match and write every tick to a file.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | required | The seed of the random-number generator. |
| `--ticks-out` | file | required | The tick file to write. |
| `--minutes` | integer | 90 | Minutes of play. |
| `--knockout` | none | off | Play extra time and a penalty shoot-out when the match is level after regulation time. |
| `--json` | none | off | Also write the ticks as JSON Lines beside the tick file, with the extension `.jsonl`. |
| `--team-a` | file | `teams/default-a.json` in the content folder | The home team file. |
| `--team-b` | file | `teams/default-b.json` in the content folder | The away team file. |
| `--no-snapshot` | none | off | Do not write a snapshot at each stoppage. |
| `--script-pack` | folder | none | A script pack (`pack.json` and a `.rhai` script) to run. See [script packs](../../content/scripts/README.md). The first `kick-off` event and the `match-stats` record name the pack. |

Output: one JSON line on standard output with the match statistics. Files: the tick file, and `matches/<match.id>/stats.json`, `events.jsonl`, and `snapshot.smsn` in the data folder.

A failed run, for example `--minutes 0` or a tick file that cannot be written, prints one `match-stats` record on standard output with `outcome` `error` and the keys `error.type`, `error.code`, and `error.retriable`, and no statistics. It saves nothing in the data folder, prints the cause on standard error, and exits 1. A bad flag (exit code 2 from the argument parser) prints no record.

Exit codes: 0 or 1.

## bench

Time whole matches on one thread and print a run report.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | required | The seed of the random-number generator. |
| `--matches` | integer | 5 | Timed matches, after one warm-up match. |
| `--minutes` | integer | 90 | Minutes of play in each match. |
| `--knockout` | none | off | Play extra time and a penalty shoot-out when the match is level after regulation time. |
| `--json` | none | off | Print the run report as one JSON line. The default output is the same. |
| `--stream` | none | off | Also stream one match to a client that reads as fast as it can. |
| `--script-pack` | folder | none | Run every timed match with this script pack; the run report names it as `script.pack`. |

Output: one `run-report` record on standard output: the median wall time, the processor time, the processor time for each tick, the peak memory, and the machine and build identifiers.

A failed run, for example `--matches 0`, prints one `run-report` record on standard output with `outcome` `error` and the keys `error.type`, `error.code`, and `error.retriable`, and no benchmark figures. It prints the cause on standard error and exits 1.

Exit codes: 0; 1; 2 when the median wall time of a 90-minute match is more than 2000 ms.

## generate

Generate fictional clubs as team files, one file for each club.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | required | The seed of the generator. The same seed gives the same clubs. |
| `--clubs` | integer | 20 | The number of clubs. |
| `--out` | folder | required | The folder for the team files. The command creates the folder when it does not exist. |
| `--force` | none | off | Overwrite team files that exist. |

Output: one team file for each club, named `<club.id>.json`.

Exit codes: 0 or 1.

## serve

Stream one match live over the local socket to one viewer.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | required unless `--resume` | The seed of the random-number generator. |
| `--minutes` | integer | 90 | Minutes of play. |
| `--knockout` | none | off | Play extra time and a penalty shoot-out when the match is level after regulation time. |
| `--ticks-out` | file | not set | Also write every tick to this file. |
| `--team-a` | file | `teams/default-a.json` in the content folder | The home team file. The manager on the page picks this team's lineup. |
| `--team-b` | file | `teams/default-b.json` in the content folder | The away team file. The AI manager runs this team. |
| `--script-pack` | folder | none | A script pack to run, as for `simulate`. |
| `--web` | folder | not set | Also serve this folder as the viewer page. |
| `--resume` | file | not set | Continue the match in this snapshot file. Do not use it with `--seed`. |
| `--reconnect-wait` | seconds | 0 | Seconds to wait for a viewer that lost its connection. 0 ends the run. |

Output: the socket port on the first line, then the page address when `--web` is set. Files: `engine.port` while the match runs, and the match files that `simulate` writes.

Exit codes: 0 at full time; 1 when the snapshot of `--resume` is refused; 2 when the viewer leaves before full time.

## launch

Serve the viewer page and run the engine as a separate process. Restart the engine after a crash.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | from the clock | The seed of the random-number generator. When absent, `launch` picks one from the clock and prints `seed <n>` on standard error. |
| `--minutes` | integer | 90 | Minutes of play. |
| `--team-a` | file | `teams/default-a.json` in the content folder | The home team file. |
| `--team-b` | file | `teams/default-b.json` in the content folder | The away team file. |
| `--web` | folder | see below | The folder that holds the viewer page. |
| `--open` | none | off | Open the page in the default browser. When the browser does not open, `launch` logs `launch.open_failed` and keeps running. |
| `--engine` | file | `SM_ENGINE_PATH`, then this program | The engine program to run. When the file does not exist, the page shows the path and how to build the engine. |

When `--web` is absent, `launch` uses `SM_WEB_DIR`. When `SM_WEB_DIR` is not set, `launch` uses `./web`, then the `web` folder beside the binary. A folder counts only when it holds `index.html`. An installed game keeps `content` and `web` beside the binary, so it starts with no flag.

Output: the page address. The page reads the engine state from `engine.json` at the same address.

Exit codes: the launcher runs until you stop it; 1 on an error.

## record

Record one whole match stream to a replay file.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | required | The seed of the random-number generator. |
| `--out` | file | required | The replay file to write (`.smfx`). |
| `--minutes` | integer | 90 | Minutes of play. |
| `--knockout` | none | off | Play extra time and a penalty shoot-out when the match is level after regulation time. |
| `--team-a` | file | `teams/default-a.json` in the content folder | The home team file. |
| `--team-b` | file | `teams/default-b.json` in the content folder | The away team file. |
| `--script-pack` | folder | none | A script pack to run, as for `simulate`. |

Exit codes: 0 when the match reaches full time; 1; 2 when it does not.

## replay

Replay a recorded match over the same socket protocol.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--fixture` | file | required | The replay file that `record` or the page wrote. |
| `--speed` | number | 1 | The playback speed. 1 is real time and 8 is eight times faster. |
| `--sustain` | number | not set | Cap the delivered rate below `--speed`. The viewer then shows its lag notice. |
| `--web` | folder | not set | Also serve this folder as the viewer page. |

Output: the socket port, then the page address when `--web` is set.

Exit codes: 0 when every frame was sent; 1; 2 when the viewer left early.

## resume

Continue a match from its newest snapshot to full time. A knockout match resumes as a knockout match: the snapshot records it.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--snapshot` | file | required | The snapshot file, `matches/<match.id>/snapshot.smsn` in the data folder. |
| `--ticks-out` | file | not set | Also write the resumed ticks to this file. |
| `--json` | none | off | Also write the ticks as JSON Lines. Requires `--ticks-out`. |
| `--team-a` | file | as for `simulate` | The home team file the match started with. |
| `--team-b` | file | as for `simulate` | The away team file the match started with. |
| `--script-pack` | folder | none | The script pack the match started with. A snapshot of a scripted match resumes only with the same pack, byte for byte, because the pack is part of the content hash. |

Output: one JSON line with the match statistics.

Exit codes: 0; 1 when the snapshot is refused: a damaged file, a file from another build, or a file from other content.

## calibrate

Play many AI-managed matches and check the realism bands.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | required | The seed of the run: the leagues, the fixtures, and every match seed. |
| `--matches` | integer | 1000 | Matches in each suite, and in each formation pairing of the formations suite. |
| `--minutes` | integer | 90 | Minutes of play in each match. |
| `--jobs` | integer | the number of logical cores | Worker processes. |
| `--suite` | `all`, `equal`, `strength`, `formations`, `red-card` | `all` | The suites to play. `all` plays the first three; `red-card` runs only when named. |
| `--pairing` | `A v B`, for example `"4-4-1-1 v 4-4-2"` | every pairing | Play only this formation pairing of the formations suite. Name the two formations in either order. Repeat it for more than one pairing. The run then plays the formations suite only. |
| `--band` | band name, for example `goals_per_match` | every band | Judge and show only this band, and play only the suites that check it. Repeat it for more than one band. |
| `--baseline` | file | none | Compare the run with an earlier `report.json`, band by band, and print the diff. |
| `--out` | folder | `runs/<run.id>` in the data folder | The run folder. |
| `--keep-events` | `outliers`, `all` | `outliers` | The event files to keep at the end of the run. |
| `--flag` | `NAME=on` or `NAME=off` | the state in `tuning.json` | Set a feature flag for the whole run. Repeat it for more than one flag. The flag must be declared in `tuning.json`. |
| `--pair` | flag name | none | Play every fixture with the flag off, then on, on the same match seeds, and compare the two arms band by band. |

Output: the run report as one JSON line. Files: `report.json`, `stats/<match.id>.json`, and `events/<match.id>.jsonl` in the run folder.

The three suites:

- `equal`: clubs of the same generated strength. The run checks goals, shots, and possession, and the eleven bands of version 2 of `realism-bands.json`.
- `strength`: one club of each match has every attribute raised by the bands' boost. The run checks that the stronger club wins more than half its matches.
- `formations`: every pairing of the formations in `tactics.json`, a formation against itself included. Ten formations give 55 pairings, and each pairing plays `--matches` matches with the clubs of the equal suite, its first formation at home in every other match. The run checks goals per match, the share of matches with 10 or more goals, and the share of goalless matches for each pairing. `calib.formations` holds the figures of every pairing, and each of its band checks carries a `pairing` such as `4-3-3 v 4-4-2`.

- `red-card`: the controlled sending-off experiment. It is not part of `all`. The default clubs play with cards otherwise off, in four arms: a control, and the away keeper (player 11), centre-back (13), or striker (21) sent off at kick-off. Every arm plays each engine seed from `--seed` onward twice, first with the default clubs in their usual order and then with home and away swapped, so each club is the reduced side in half the matches and a difference in club strength cancels out. `--seed 1 --matches 240` (seeds 1 to 120 in both orders) is the experiment of the slow test `a_sending_off_gives_no_advantage`. An odd `--matches` plays the last seed in the usual order only. `calib.red_card` holds the control's home and away mean goals, each arm's full-side (home) and reduced-side (away) mean goals, the limit (1.6 times the control's home mean), and `pass`. Each arm has two band checks, with the arm in `pairing`: `reduced_minus_full` (the reduced side's goals less the full side's, at most 0) and `full_over_control` (the full side's goals over the control's home goals, at most 1.6).

With the defaults, `--suite all` plays 57,000 matches: about 74 minutes on a 16-core machine. `--suite equal` plays 1,000 matches in about 80 seconds.

A targeted run plays only what it selects. `--pairing` plays each named pairing's `--matches` matches, with the same match seeds, clubs, and home sides as in a full run, so the figures of a targeted pairing or suite equal the figures of the same pairing or suite in a `--suite all` run on the same seed. `--band` narrows `--suite` to the suites that check the named bands: `goals_per_match`, `ten_plus_goals_share`, and `goalless_share` belong to `equal` and `formations`; `stronger_team_win_rate` to `strength`; `reduced_minus_full` and `full_over_control` to `red-card`; every other band to `equal`. Each suite's `wall_ms` check stays. `calib.selection` names the suites, pairings, and bands the run selected.

Every band check carries `se`, its sampling error: the standard error of a mean (sd / sqrt(n)), of a share (sqrt(p (1 - p) / n)), or of a pooled ratio (the ratio estimator), over the matches judged. `wall_ms` has 0.

A baseline must have the same seed, match count (`calib.matches`), and `fixtures.hash` as the run. `fixtures.hash` covers the inputs that decide the fixtures: the attributes, rules, and tactics content, the generator block of `tuning.json` after the flag states, and the two default clubs. A different `content.hash` (other tuning values or flag states) is allowed: it is the change under test. The baseline is checked before the run folder is made, and a refusal exits with code 1 and names each difference, for example `seed 7 differs from the baseline's 42`, `match count 500 differs from the baseline's 1000`, or `fixtures hash 3f2a9c01b7de differs from the baseline's 0c44e1a9f2b3`. A report without `se` on its band checks or without `fixtures.hash` is refused as made before this check existed; make a new one. `--baseline` cannot be used with `--pair`.

The diff is printed on standard error. Its first line names the baseline's `run.id` and seed, the baseline's and the run's content hashes, and `content changed` or `content unchanged`. Then one row per band, pairing, and arm that both reports judged, the time budget left out, with these columns: `suite`, `band`, `pairing`, `baseline`, `new`, `change` (new less baseline), `error` (sqrt(se_baseline^2 + se_new^2)), the mark (`noise` when the change is at most two errors, otherwise `change`), and the verdict of the new run (`pass` or `miss`). The report holds the same rows in `calib.diff`, and the baseline in `calib.baseline`. The diff does not change the exit code.

Standard error names each failing band on its own `warn` line, with `signal` `calibrate.band_failed`, the suite, the band, the pairing when there is one, the value, and the range.

A match the engine cannot play still writes its `stats/<match.id>.json`, with `outcome` `error`, the keys `error.type`, `error.code`, and `error.retriable`, and zero figures; the run goes on and leaves that match out of the bands. When a worker process fails, the run report has `outcome` `error`, `error.type` `worker`, `error.code` `worker-failed`, and `error.retriable` `false`.

A paired run writes each arm to its own folder: `arms/off/stats/`, `arms/off/events/`, `arms/on/stats/`, and `arms/on/events/`. The one `report.json` stays at the top of the run folder. Its top-level figures are the off arm's. `calib.arms` holds both arms, `calib.compare` holds one row per suite, band, and formation pairing with the off value and the on value side by side, and `calib.verdict` is `on-better`, `off-better`, `no-difference`, or `on-rejected`. The same table is printed on standard error.

A flag name that `tuning.json` does not declare, a state other than `on` or `off`, a `--pair` flag also given with `--flag`, an unknown pairing or band, a `--pairing` without the formations suite, and a `--band` that no selected suite checks are refused with exit code 1 before any match is played. Each message lists the valid names.

Exit codes: 0 when every band passes and both dark-path counters are zero; 1; 2 otherwise. For a paired run: 0 when both arms have no missing statistics record, no change left unapplied, no validator violation, and no failed worker; 1; 2 otherwise. The verdict does not change the exit code.

## gate

Replay the 22 gate matches and compare their state hashes with the golden file. The gate proves that a change to the engine leaves every match exactly as it was: after every tick it hashes the full match state (positions and velocities as exact bits, stamina, cards, the rules and the restart, the clock, the teams and managers, the pending changes, the script-hook counters, the random stream's position, and the tick's events) into a running SHA-256, and keeps a checkpoint every 1,000 ticks and at the last tick.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--golden` | file | `gate/golden.json` | The golden file to compare with. The default path is relative to the current folder, so run the command from the repository root. |
| `--fixture` | fixture id | every fixture | Play only this fixture: `seed-<seed>`, `change`, or `knockout`. Repeat it for more than one fixture. |
| `--json` | none | off | Print one JSON object per match instead of a text line. |
| `--bootstrap` | none | off | Play every fixture and write the first golden file with this machine's hash set. Refused when the file already exists, and with `--fixture`. |
| `--regenerate` | none | off | Play every fixture and rewrite the golden file with this build's hashes for this machine. Appends one `regenerate` entry to the ledger and drops the other machines' hash sets, which each machine then adds again with `--add-machine-set`. Needs `--reason`. |
| `--add-machine-set` | none | off | Play every fixture and add this machine's hash set, with one `add-machine-set` entry. Refused when this machine already has a set, or when the file does not fit this build. Needs `--reason`. |
| `--reason` | text | none | Why the golden file is written; recorded in the new ledger entry. Needed by `--regenerate` and `--add-machine-set`; optional for `--bootstrap`. |

The fixtures, in gate order:

- `seed-42`, `seed-1`, `seed-7`, `seed-99`, `seed-2026`, `seed-0`, `seed-18446744073709551615`, `seed-3`, `seed-11`, `seed-23`, `seed-57`, `seed-123`, `seed-314`, `seed-777`, `seed-1000`, `seed-4242`, `seed-9001`, `seed-31337`, `seed-65535`, `seed-1000003`: a 90-minute match between the default teams, both managed by the AI.
- `change`: seed 42 with both managers human. A substitution for the home team is queued at tick 60,000 (the player in lineup slot 9 off, the first bench player on), and a mentality change to attacking for the away team at tick 90,000.
- `knockout`: seed 2, a knockout match with the shipped `sample` script pack. It goes to extra time and a penalty shoot-out. The gate plays it with the real 2 ms wall-clock limit: a script hook call past the limit is not stopped, so a busy machine plays the same match, and the gate prints a warning for it; the operation budget still stops a long call.

Output: one line per match on standard output: the fixture id, `match` or `differs`, the tick count, the first 12 characters of the final hash, and for `change` and `knockout` the applied substitutions and tactics changes, or whether the match went to extra time and a shoot-out. A match that differs gets a second line that names the window of ticks in which its state first changed, for example `seed-42 differs: the state first differs between tick 30000 and tick 31000`, or the two tick counts when the match lasted longer or shorter. With `--json`, each match is one JSON object with `fixture`, `verdict`, `ticks`, `final_hash`, `window` (`from` and `to`), `detail`, `extra_time`, `shootout`, `decided_by`, `substitutions_applied`, `tactics_changes_applied`, `slow_calls` (script hook calls past the wall-clock limit), and `invalid` (`slow script` when `slow_calls` is above 0, otherwise `null`). A match with a slow hook call gets a line on standard error, for example `warning: knockout is marked invalid: slow script (1 hook call ran past the wall-clock limit); its hashes were compared`; its hashes are compared as for any other match, and the warning does not change the exit code. Standard error ends with the match count, the machine key, and the run time.

The golden file holds, in this order: the gate schema version, the state inventory version, the checkpoint spacing, the toolchain, the fixture list, the `ledger`, the `set_differences` record, and one hash set per machine, keyed `<os>-<arch>`, for example `windows-x86_64`. Seeds are decimal strings and hashes are 64 lowercase hex characters.

The ledger is append-only. Each entry has a `kind` (`bootstrap` for the first file, `add-machine-set` for a second machine's set of unchanged code, `regenerate` for a hash change), the `reason`, the `engine_version`, the `build` commit, the random-stream `scheme`, the `utc` time, and the `machine`. A `regenerate` entry also has the `candidate` commit and a `band_result` path, `gate/bands/ledger-<index>.json`, fixed when the entry is written. The `set_differences` record has one item per pair of machines: `a`, `b`, the ids of the matches that `differ`, and the count of matches that are the `same`. A file written before the ledger existed, with a top-level `bootstrap` object, reads as a ledger of one `bootstrap` entry.

The gate refuses the file, before any match is played, when it is malformed, when a version, the checkpoint spacing, or the fixture list differs from the build's, when it has no hash set for this machine, when a hash set misses a match, lists one twice, or misses a checkpoint, when the ledger is empty, does not start with its only `bootstrap` entry, has an entry with no reason, or does not account for exactly the hash sets present, or when the `set_differences` record differs from the one the hash sets give. The message names the fault.

The write modes check their flags and the reason before they read the golden file or play a match, and write through `<file>.tmp` and a rename. A refusal, or a match that fails, leaves the golden file byte-identical. [The replay-gate guide](../how-to/replay-gate.md) says when each mode is allowed.

Exit codes: 0 when every selected match matches; 1 when the golden file or the content cannot be read or is refused, or a fixture id is unknown; 2 when a match differs or its hashed state holds a number that is not finite (the line names the field, for example `players[3].pos.x`).

## guard

Check every commit that changes the golden file against the ledger rules. The command lists the commits in `<base>..<head>` that change the file, oldest first, and compares each one's file with its first parent's. It plays no match and reads no content. Run it from the repository root; CI runs it on every pull request.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--base` | revision | required | The base revision, such as `main` or the pull request's base commit. |
| `--head` | revision | `HEAD` | The last revision of the range. |
| `--golden` | path | `gate/golden.json` | The golden file's path from the repository root, with forward slashes. |

The rules each change must keep:

1. The golden file is not deleted.
2. A new golden file holds exactly one ledger entry, a `bootstrap`, and exactly one hash set, keyed by that entry's machine.
3. The old ledger entries stay, unchanged and in order. One commit adds at most one entry, and never a second `bootstrap`.
4. A change to the fixture list, the state inventory version, or the checkpoint spacing needs a higher gate schema version and a new `regenerate` entry. The gate schema never decreases.
5. A change to the gate schema, the toolchain, or a hash set in both files, or a removed hash set, needs a new `regenerate` entry. The entry's `candidate` must be a clean build (no `-dirty` mark, not `unknown`) of a commit that is an ancestor of the commit.
6. A new `add-machine-set` entry adds exactly one hash set, keyed by its machine, and changes nothing else.
7. A new hash set needs a new entry, and a new entry needs a change that it records. The file also keeps the ledger rules of the gate's strict load.

A merge commit is compared with its first parent.

Output: one line per commit on standard output, `<short commit> ok`, or one line per broken rule, for example `3f2a9c1 rule 5: hash set linux-x86_64 changes with no new regenerate entry`. Standard error ends with the commit count and the number that fail.

Exit codes: 0 when every commit passes, or no commit in the range changes the file; 1 when git cannot run or a revision cannot be read; 2 when a commit breaks a rule.

## Files

Every file the engine reads or writes is in [the data-file reference](data-files.md). Every socket message is in [the protocol reference](protocol.md).
