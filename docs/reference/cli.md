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

Output: one JSON line on standard output with the match statistics. Files: the tick file, and `matches/<match.id>/stats.json`, `events.jsonl`, and `snapshot.smsn` in the data folder.

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

Output: one `run-report` record on standard output: the median wall time, the processor time, the processor time for each tick, the peak memory, and the machine and build identifiers.

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
| `--web` | folder | not set | Also serve this folder as the viewer page. |
| `--resume` | file | not set | Continue the match in this snapshot file. Do not use it with `--seed`. |
| `--reconnect-wait` | seconds | 0 | Seconds to wait for a viewer that lost its connection. 0 ends the run. |

Output: the socket port on the first line, then the page address when `--web` is set. Files: `engine.port` while the match runs, and the match files that `simulate` writes.

Exit codes: 0 at full time; 1 when the snapshot of `--resume` is refused; 2 when the viewer leaves before full time.

## launch

Serve the viewer page and run the engine as a separate process. Restart the engine after a crash.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | required | The seed of the random-number generator. |
| `--minutes` | integer | 90 | Minutes of play. |
| `--team-a` | file | `teams/default-a.json` in the content folder | The home team file. |
| `--team-b` | file | `teams/default-b.json` in the content folder | The away team file. |
| `--web` | folder | required | The folder that holds the viewer page. |
| `--engine` | file | `SM_ENGINE_PATH`, then this program | The engine program to run. When the file does not exist, the page shows the path and how to build the engine. |

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

Output: one JSON line with the match statistics.

Exit codes: 0; 1 when the snapshot is refused: a damaged file, a file from another build, or a file from other content.

## calibrate

Play many AI-managed matches and check the realism bands.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | required | The seed of the run: the leagues, the fixtures, and every match seed. |
| `--matches` | integer | 1000 | Matches in each suite. |
| `--minutes` | integer | 90 | Minutes of play in each match. |
| `--jobs` | integer | the number of logical cores | Worker processes. |
| `--suite` | `all`, `equal`, `strength` | `all` | The suites to play. |
| `--out` | folder | `runs/<run.id>` in the data folder | The run folder. |
| `--keep-events` | `outliers`, `all` | `outliers` | The event files to keep at the end of the run. |
| `--flag` | `NAME=on` or `NAME=off` | the state in `tuning.json` | Set a feature flag for the whole run. Repeat it for more than one flag. The flag must be declared in `tuning.json`. |
| `--pair` | flag name | none | Play every fixture with the flag off, then on, on the same match seeds, and compare the two arms band by band. |

Output: the run report as one JSON line. Files: `report.json`, `stats/<match.id>.json`, and `events/<match.id>.jsonl` in the run folder.

A paired run writes each arm to its own folder: `arms/off/stats/`, `arms/off/events/`, `arms/on/stats/`, and `arms/on/events/`. The one `report.json` stays at the top of the run folder. Its top-level figures are the off arm's. `calib.arms` holds both arms, `calib.compare` holds one row per suite and band with the off value and the on value side by side, and `calib.verdict` is `on-better`, `off-better`, `no-difference`, or `on-rejected`. The same table is printed on standard error.

A flag name that `tuning.json` does not declare, a state other than `on` or `off`, and a `--pair` flag also given with `--flag` are refused with exit code 1 before any match is played.

Exit codes: 0 when every band passes and both dark-path counters are zero; 1; 2 otherwise. For a paired run: 0 when both arms have no missing statistics record, no change left unapplied, no validator violation, and no failed worker; 1; 2 otherwise. The verdict does not change the exit code.

## Files

Every file the engine reads or writes is in [the data-file reference](data-files.md). Every socket message is in [the protocol reference](protocol.md).
