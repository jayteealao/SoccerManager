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
| 1 | A usage or run error stopped the command. Standard error names the cause: a bad flag or a missing argument, a refused content file, a refused snapshot, or a file that cannot be written. |
| 2 | The verdict of a completed command: a check failed, the frames or builds or hashes differ, or the match did not reach full time. The command sections below name the check. A usage error is never 2. |
| 3 | `bisect` only: the comparison is incomplete. A build could not be made or run to the end, so no result was reached. |

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
| `--debug-trace` | file | none | Play the match with debug mode on and write its debug trace to this file. See [Debug trace file](#debug-trace-file). Debug mode changes no result. |

Output: one JSON line on standard output with the match statistics. With `--debug-trace`, standard error also gets one line, for example `debug trace: m.trace.jsonl, 307873 draws (registry 307873), 595706 decisions, 2435 rule outcomes`; when the draws recorded differ from the registry's draw count, the command exits 1. Files: the tick file, and `matches/<match.id>/stats.json`, `events.jsonl`, and `snapshot.smsn` in the data folder.

A failed run, for example `--minutes 0` or a tick file that cannot be written, prints one `match-stats` record on standard output with `outcome` `error` and the keys `error.type`, `error.code`, and `error.retriable`, and no statistics. It saves nothing in the data folder, prints the cause on standard error, and exits 1. A bad flag exits 1 and prints no record.

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

Test seams: `serve` also takes hidden test flags that `--help` does not show, among them `--test-jump`. They are for automated tests, not for players. `--test-jump` lets a client send the `jump` command, which makes a started match send every tick up to a named tick at once; the match itself is unchanged. Without `--test-jump` the engine refuses the `jump` command and keeps its normal pace. See `jump` in the protocol reference.

## launch

Serve the viewer page and run the engine as a separate process. Restart the engine after a crash.

By default the page opens on the start screen and no match runs: the player starts a new match with two chosen teams, resumes the saved match, opens replays, changes the settings, reads the licences, or quits. `--no-start-screen` starts a match at once instead.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | from the clock | The seed of the random-number generator, used for every match started from match setup. When absent, each match takes a seed from the clock; with `--no-start-screen`, `launch` prints that seed as `seed <n>` on standard error. |
| `--minutes` | integer | 90 | Minutes of play. |
| `--team-a` | file | `teams/default-a.json` in the content folder | The home team file. |
| `--team-b` | file | `teams/default-b.json` in the content folder | The away team file. |
| `--web` | folder | see below | The folder that holds the viewer page. |
| `--open` | none | off | Open the page in the default browser. When the browser does not open, `launch` logs `launch.open_failed` and keeps running. |
| `--engine` | file | `SM_ENGINE_PATH`, then this program | The engine program to run. When the file does not exist, the page shows the path and how to build the engine. |
| `--resume` | file | not set | Continue the saved match in this snapshot file instead of starting one. The engine that wrote the save finishes it, as for `resume`. A save from any other version starts no engine: the page shows the Resume a saved match screen, which names the save's version and offers a new match. |
| `--previous` | file | `SM_PREVIOUS_ENGINE_PATH`, then `previous/engine-cli` beside this program | The previous release's engine program. |
| `--no-start-screen` | none | off | Start a match at once with `--seed`, `--team-a` and `--team-b`, instead of opening the start screen. Do not use it with `--resume`. |

When `--web` is absent, `launch` uses `SM_WEB_DIR`. When `SM_WEB_DIR` is not set, `launch` uses `./viewer/dist` (the built viewer in a repository checkout, after `npm --prefix viewer run build`), then the `web` folder beside the binary (an installed game). A folder counts only when it holds `index.html`. An installed game keeps `content` and `web` beside the binary, so it starts with no flag.

Output: the page address. The page reads the engine state from `engine.json` at the same address. Its fields include `engine.version` (the release version of the program that plays the match), `launcher.version`, `match.resumed_from` (the tick a saved match continued from), and, when a save cannot resume, a `resume` block: `kind` (`older`, `newer`, `other`, `unreleased` or `previous-missing`), `saved.version`, `saved.build`, `saved.tick`, `saved.teams`, `saved.score`, `saved.millis` (the match stamp), `engines` (the two versions this program finishes) and `reason`. On the start screen the state is `idle`, and `engine.json` also carries `front-door` (true), `teams` (the sample teams match setup offers), `saved` (the newest unfinished match, or null), `settings` and `previous.version`. The page asks for actions with a POST from its own origin, each with a body of at most 4 KiB: `/engine/new-match` (with no body, a fresh match with the launch's seed and teams; with `{"home": <club id>, "away": <club id>}`, that fixture), `/engine/resume` (the saved match), `/engine/stop` (stop the match and keep its save), `/engine/quit` (stop the match, keep its save and end `launch`), and `/engine/settings` (save the three settings). A GET of `/engine/round?home=<club id>&away=<club id>` answers the other fixtures of the round that match would meet. See the protocol reference for the fields and the answers.

`launch` also takes the hidden test seam `--test-jump`, which it passes on to each match's engine (see `serve`); it is for automated tests, not for players.

Exit codes: the launcher runs until you stop it or the player quits (0); 1 on an error.

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
| `--changes` | file | none | A JSON file of manager changes to queue. Each team the file names is managed by hand. |

The replay file is version 4. It holds every input file of the match by value, the engine identity, the applied-change log, and the watchdog mark, so `resimulate` can play the match again from the file alone. See the protocol reference, section "Replay files".

The change file is a JSON array. Each change is queued before the step that starts on its `tick`. `team` is 0 (home) or 1 (away). A change is a substitution (`slot` is the lineup slot of the player who goes off, 0 to 10, and `bench` is the place on the bench of the player who comes on) or a mentality (an index into the tactics file's mentalities):

```json
[
  { "tick": 3000, "team": 0, "change": { "substitution": { "slot": 9, "bench": 0 } } },
  { "tick": 3000, "team": 0, "change": { "mentality": 4 } }
]
```

A change the engine rejects, for example a bench place the team does not have, is not in the change log. Its verdict is in the text frames.

Output: one JSON line with `fixture`, `frames`, `ticks`, `bytes`, `hash`, `format` (4), `inputs_bytes` (the total size of the input files), and `changes_applied`.

Exit codes: 0 when the match reaches full time; 1; 2 when it does not.

## resimulate

Play a recorded match again from its replay file alone, and compare it with the stored frames. Every input comes from the file. The command reads no content folder: it refuses the global `--content-dir`, and it does not read `SM_CONTENT_DIR`.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--fixture` | file | required | The version-4 replay file that `record` wrote. |
| `--compare` | none | off | Comparison mode: run on any engine, and report both engine identities and both stream schemes. |
| `--state-digests` | file | none | Write the SHA-256 of the full match state after every tick to this file. See [State digest file](#state-digest-file). |
| `--state-fields` | file | none | Write the named parts of the match state after the tick `--at-tick` names to this file, as one JSON object, and stop the match after that tick. Needs `--at-tick`. |
| `--at-tick` | tick | none | The tick `--state-fields` writes, from 1. Needs `--state-fields`. |
| `--debug-trace` | file | none | Play the match with debug mode on and write its debug trace to this file, as `simulate --debug-trace` does. See [Debug trace file](#debug-trace-file). |

Strict mode (the default) first compares the record's executable SHA-256 and stream scheme with this binary's. Each difference is named on standard error, for example `executable SHA-256 differs: the record has 3f2a…, this binary is 9c01…` or `scheme 2 differs from this build's 1`, and the command exits 1. A record made on a dirty build runs on the same binary with the warning `recorded on a dirty build (<build>); that version cannot be rebuilt`. The commit, the crate version, and the maths library are reported and not compared, because the executable SHA-256 covers them.

Comparison mode never refuses on the identity. It prints `engine (record)`, `engine (this binary)`, and `scheme N (record), scheme M (this binary)` on standard error. The replay file is opened to read only in both modes.

The match is built from the file's inputs. Each manager change of the log is queued again at its recorded tick, and the computer manager makes its own changes again. Every regenerated tick frame is compared with the stored one, byte for byte. After full time, the applied changes must equal the log, entry by entry. Text frames are not compared. The script pack runs with no wall-clock limit, and the recorded watchdog mark is reported.

A version-3 replay file is refused: `<file> holds no inputs: it is a version-3 replay file, which plays from its frames only`.

Output: one JSON line with `fixture`, `mode` (`strict` or `compare`), `verdict` (`identical` or `differs`), `tick_frames`, `stored_sha256` and `resimulated_sha256` (over the tick-frame payloads), `first_difference` (null, `{ "frame", "tick" }` for the first tick frame that differs, or `{ "change": order }` for the first log entry that differs), `changes_applied`, `inputs_bytes`, and `watchdog` (the recorded mark). In comparison mode, `engines` holds both identities. With `--at-tick`, `stopped_at` holds the tick; the match stops there, so the frame comparison reports the missing frames as a difference.

The three state outputs are off by default, and they do not change the verdict or the exit code. `--state-fields` writes `{ "tick", "scheme", "fields", "engine" }`: `fields` lists each named part of the state in byte order, each with `name` (for example `ball.vel`, `players[3].pos`, `referee.clock`, or `streams`), `kind` (`floats`: a run of 64-bit floats; `streams`: the stream state; `bytes`: anything else), and `hex` (its canonical bytes). The parts join into the bytes whose SHA-256 is the tick's line in the state digest file. When the match ends before the tick, the command writes no fields file and exits 1.

Exit codes: 0 when the frames and the change log are identical; 1 when a flag is refused, the file is refused or cannot be read, or the engine differs in strict mode; 2 when the frames or the change log differ.

## bisect

Find the first tick where two engine versions differ on one replay file. Each side is a commit, built on demand, or a ready `engine-cli` executable. Both builds re-simulate the file, and bisect compares the digest of the full match state after every tick. At the first tick whose digests differ, both builds re-simulate the file again up to that tick and write the named parts of its state and its debug trace.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--fixture` | file | required | The version-4 replay file that `record` wrote. |
| `--a` | commit | one of `--a` and `--a-binary` | Build A: a commit (any name git accepts, for example `HEAD~1`), built into the bisect build cache. |
| `--a-binary` | file | one of `--a` and `--a-binary` | Build A: a ready `engine-cli` executable. |
| `--b` | commit | one of `--b` and `--b-binary` | Build B: a commit, built into the bisect build cache. |
| `--b-binary` | file | one of `--b` and `--b-binary` | Build B: a ready `engine-cli` executable. |
| `--repo` | folder | `.` | The git repository the commits are built from. |
| `--cache` | folder | `<repo>/target/bisect` | The bisect build cache. |
| `--profile` | name | `release` | The cargo profile the commits are built with. |
| `--features` | list | none | The cargo features the commits are built with, comma-separated; none means the defaults. |
| `--timeout` | seconds | 1800 | The longest one run of a build may take. A run that takes longer is stopped. |
| `--json` | none | off | Print the report as one JSON object. |

The replay file is read first. A version-3 file is refused before any build or run: `<file> holds no inputs: it is a version-3 replay file, which plays from its frames only; bisect needs the inputs to re-simulate it`. The global `--content-dir` is refused, because every input comes from the file.

**The bisect build cache.** A commit is built in a detached git worktree under `<cache>/checkouts/`, with `cargo build --locked -p engine-cli`, into one cargo target folder, `<cache>/target/`, that every build shares. The worktree is removed after the build. The executable is kept in `<cache>/entries/<id>/` with `entry.json`, which holds the key and the executable's SHA-256. The key is the full commit hash, the toolchain (the `release` and `commit-hash` that `rustc -vV` reports for the channel the commit's `rust-toolchain.toml` pins), the target (the compiler's host), the profile, and the sorted features. `<id>` is the SHA-256 of the key. A cached executable is re-used only when its `entry.json` names the same key and the executable still has the recorded SHA-256; otherwise the commit is built again. The main checkout is never switched. To clear the cache, delete the folder.

**Each run.** Bisect first runs `<build> resimulate --help`, which must name `--state-digests`, `--state-fields`, `--at-tick`, and `--debug-trace`. It then runs `resimulate --fixture <file> --compare --state-digests <file>` on each build (see [State digest file](#state-digest-file)), and at the first differing tick, `resimulate --fixture <file> --compare --state-fields <file> --at-tick <tick> --debug-trace <file>`. A run counts only when it exits with 0 or 2, prints its verdict line last, and writes whole files: a digest file with its `end` line, ticks from 1 with no gap, and a tick count equal to the `end` line's. The run files are written to a temporary folder, which is removed at the end.

The three verdicts:

- **No difference** (exit 0): the state is the same after every tick and after full time.
- **Differs** (exit 2): the report names the first tick whose state differs. When the states agree on every tick both builds played and one build played longer, the first tick past the shorter match differs, and the report names `match length`.
- **Incomplete** (exit 3): a build could not be made or run to the end. Each side's reason is printed on its own line, for example `the build of <commit> failed:` with the last 40 lines of cargo's output, `<path> does not exist`, `lacks the resimulate command`, `lacks the state-digest options (added with bisect)`, `crashed or failed: the run exited with code 101`, `timed out after 1800 s and was stopped`, `ends early: the state digest file stops after tick 1499 with no end line`, or `a gap in the state digests`. When the two builds hash different state inventories, the digests cannot be compared, and the result is incomplete. An incomplete result is never "no difference". Only builds that have the four re-simulate options can be compared.

**The report** (text, or one JSON object with `--json`): the replay file; each build with the name it was given, its commit and dirty mark, its executable SHA-256, and `built`, `reused`, or `ready binary`; both stream schemes, with the note `the stream scheme differs (1 and 2): stream positions differ from the first draw` when they differ; the first differing tick; each differing part of the state with both values; and each build's debug trace records for that tick. Values are shown by kind: a `floats` part as its numbers, the `streams` part as the stream ids whose word positions differ or that one build lacks, and any other part as hex. The JSON object has `verdict` (`no difference`, `differs`, or `incomplete`), `fixture`, `a` and `b` (`given`, `source`, `scheme`, and `engine`, or `given` and `reason` when incomplete), `tick`, `fields` (`name`, `kind`, `a`, `b`), `trace` (`a`, `b`), and `scheme_note` when the schemes differ.

Exit codes: 0 no difference; 1 the file or a flag is refused; 2 the builds differ; 3 incomplete.

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

A snapshot resumes only on the build that wrote it. The snapshot records the release version of that build, and `resume` picks the program by that version:

- A snapshot from this version: this program continues it.
- A snapshot from the previous release (0.2.0-beta.1): the previous release's program, shipped as `previous/engine-cli` beside this program, continues it with its own `previous/content` folder. `resume` passes that program's output and exit code through, and logs `resume.delegated`.
- Any other snapshot: refused, with a message that names the version that saved it, for example `snapshot refused: this match was saved by Touchline 0.1.0, two or more versions back; ...`. The file stays on disk.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--snapshot` | file | required | The snapshot file, `matches/<match.id>/snapshot.smsn` in the data folder. |
| `--previous` | file | `SM_PREVIOUS_ENGINE_PATH`, then `previous/engine-cli` beside this program | The previous release's engine program. It plays with the `content` folder beside it. |
| `--ticks-out` | file | not set | Also write the resumed ticks to this file. |
| `--json` | none | off | Also write the ticks as JSON Lines. Requires `--ticks-out`. |
| `--team-a` | file | as for `simulate` | The home team file the match started with. |
| `--team-b` | file | as for `simulate` | The away team file the match started with. |
| `--script-pack` | folder | none | The script pack the match started with. A snapshot of a scripted match resumes only with the same pack, byte for byte, because the pack is part of the content hash. |

Output: one JSON line with the match statistics.

Exit codes: 0; 1 when the snapshot is refused: a damaged file, a file from a version this program does not carry, a file from another build of this version, or a file from other content.

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
| `--out` | folder | `runs/<run.id>` in the data folder | The run folder. Run the same command again into the same folder to resume a stopped run, or to grow it with a larger `--matches`. |
| `--keep-events` | `outliers`, `all` | `outliers` | The event files to keep at the end of the run. |
| `--flag` | `NAME=on` or `NAME=off` | the state in `tuning.json` | Set a feature flag for the whole run. Repeat it for more than one flag. The flag must be declared in `tuning.json`. |
| `--pair` | flag name | none | Play every fixture with the flag off, then on, on the same match seeds, and compare the two arms band by band. |

Output: the run report as one JSON line. Files: `run.json`, `ledger/`, `report.json`, `stats/<match.id>.json`, and `events/<match.id>.jsonl` in the run folder.

The three suites:

- `equal`: clubs of the same generated strength. The run checks goals, shots, and possession, and the eleven bands of version 2 of `realism-bands.json`.
- `strength`: one club of each match has every attribute raised by the bands' boost. The run checks that the stronger club wins more than half its matches.
- `formations`: every pairing of the formations in `tactics.json`, a formation against itself included. Ten formations give 55 pairings, and each pairing plays `--matches` matches with the clubs of the equal suite, its first formation at home in every other match. The run checks goals per match, the share of matches with 10 or more goals, and the share of goalless matches for each pairing. `calib.formations` holds the figures of every pairing, and each of its band checks carries a `pairing` such as `4-3-3 v 4-4-2`.

- `red-card`: the controlled sending-off experiment. It is not part of `all`. The default clubs play with cards otherwise off, in four arms: a control, and the away keeper (player 11), centre-back (13), or striker (21) sent off at kick-off. Every arm plays each engine seed from `--seed` onward twice, first with the default clubs in their usual order and then with home and away swapped, so each club is the reduced side in half the matches and a difference in club strength cancels out. `--seed 1 --matches 240` (seeds 1 to 120 in both orders) is the experiment of the slow test `a_sending_off_gives_no_advantage`. An odd `--matches` plays the last seed in the usual order only. `calib.red_card` holds the control's home and away mean goals, each arm's full-side (home) and reduced-side (away) mean goals, the limit (1.6 times the control's home mean), and `pass`. Each arm has two band checks, with the arm in `pairing`: `reduced_minus_full` (the reduced side's goals less the full side's, at most 0) and `full_over_control` (the full side's goals over the control's home goals, at most 1.6).

With the defaults, `--suite all` plays 57,000 matches: about 74 minutes on a 16-core machine. `--suite equal` plays 1,000 matches in about 80 seconds.

A targeted run plays only what it selects. `--pairing` plays each named pairing's `--matches` matches, with the same fixture keys, engine seeds, clubs, and home sides as in a full run, so the figures of a targeted pairing or suite equal the figures of the same pairing or suite in a `--suite all` run on the same seed. `--band` narrows `--suite` to the suites that check the named bands: `goals_per_match`, `ten_plus_goals_share`, and `goalless_share` belong to `equal` and `formations`; `stronger_team_win_rate` to `strength`; `reduced_minus_full` and `full_over_control` to `red-card`; every other band to `equal`. Each suite's `wall_ms` check stays. `calib.selection` names the suites, pairings, and bands the run selected.

Every fixture has a key: a hash of what the fixture is, made by the fixture scheme `fixture-key-1`. The key covers the scenario (`equal`, the strength suite's boosted side, `formations`, or the red-card arm), the world (the generated league's seed, or the default clubs and `--seed` in the red-card suite), the two clubs with the home club first, both formations by name, and a repeat number. The engine seed is stored beside the key: it comes from the same hash, except in the red-card suite, which keeps the seeds from `--seed` onward. The match identifier is the key and the run's start time. So a run with a larger `--matches` keeps every earlier fixture's key and engine seed, and a new formation in `tactics.json` leaves the other pairings' matches alone. `fixtures.scheme` in the report names the scheme.

The run folder holds the run's identity in `run.json`: the build (its commit and the SHA-256 of the program), the content, the fixtures hash, the flags set on the command line, the paired flag, the seed, the minutes, the strength boost of `realism-bands.json`, the random-number scheme, the fixture scheme, the measures version, and the bands file's version. The match count, the suites, the pairings, the bands judged, `--jobs`, and `--keep-events` are not part of it: they choose which work is done. The fixtures still to play are cut into work units of 8, and each worker appends one line to `ledger/` when every match of a unit has its statistics file. When the same command runs again into the same folder:

- With the same identity, the run resumes: it plays only the fixtures the ledger does not hold, and standard error says `resuming run <run.id>: <done> of <total> fixtures done, <left> to play`. A unit cut off mid-way plays again; a finished unit never does. A larger `--matches` grows the run the same way.
- With another identity, the old run's files move to `superseded/<old run.id>/`, a new run starts, and standard error names each part that differs, for example `minutes 3 -> 4` or `content <old> -> <new>`. Nothing is resumed.

The default run folder is new for every run, so resume and growth need `--out`. Two runs into one folder at the same time are not supported. The report holds `run.identity`, `calib.units` (`total` fixtures, `finished_before` this session, and `played` now), and `calib.results_digest`: the SHA-256 of every match's statistics in key order with the identifier and timing left out, so a resumed run has the digest of an uninterrupted one.

Every band check carries `se`, its sampling error: the standard error of a mean (sd / sqrt(n)), of a share (sqrt(p (1 - p) / n)), or of a pooled ratio (the ratio estimator), over the matches judged. `wall_ms` has 0.

A baseline must be made with the same fixture scheme, and have the same seed, match count (`calib.matches`), and `fixtures.hash` as the run. A report without `fixtures.scheme` is from the old seeding scheme, whose match seeds came from the fixture's place in the run; it played other matches, so it is refused first, with `it was made with the old seeding scheme (match seeds from the fixture's place in the run); this run uses fixture keys (fixture-key-1), so the two runs played different matches; make a new baseline`, and exit code 1. The saved reports `gate/bands/ledger-N.json` are such reports: they still load and show, but cannot be a baseline. `fixtures.hash` covers the inputs that decide the fixtures: the attributes, rules, and tactics content, the generator block of `tuning.json` after the flag states, and the two default clubs. A different `content.hash` (other tuning values or flag states) is allowed: it is the change under test. The baseline is checked before the run folder is made, and a refusal exits with code 1 and names each difference, for example `seed 7 differs from the baseline's 42`, `match count 500 differs from the baseline's 1000`, or `fixtures hash 3f2a9c01b7de differs from the baseline's 0c44e1a9f2b3`. A report without `se` on its band checks or without `fixtures.hash` is refused as made before this check existed; make a new one. `--baseline` cannot be used with `--pair`.

The diff is printed on standard error. Its first line names the baseline's `run.id` and seed, the baseline's and the run's content hashes, and `content changed` or `content unchanged`. Then one row per band, pairing, and arm that both reports judged, the time budget left out, with these columns: `suite`, `band`, `pairing`, `baseline`, `new`, `change` (new less baseline), `error` (sqrt(se_baseline^2 + se_new^2)), the mark (`noise` when the change is at most two errors, otherwise `change`), and the verdict of the new run (`pass` or `miss`). The report holds the same rows in `calib.diff`, and the baseline in `calib.baseline`. The diff does not change the exit code.

Standard error names each failing band on its own `warn` line, with `signal` `calibrate.band_failed`, the suite, the band, the pairing when there is one, the value, and the range.

A match the engine cannot play still writes its `stats/<match.id>.json`, with `outcome` `error`, the keys `error.type`, `error.code`, and `error.retriable`, and zero figures; the run goes on and leaves that match out of the bands. When a worker process fails, the run report has `outcome` `error`, `error.type` `worker`, `error.code` `worker-failed`, and `error.retriable` `false`.

A paired run writes each arm to its own folder: `arms/off/stats/`, `arms/off/events/`, `arms/on/stats/`, and `arms/on/events/`. The one `report.json` stays at the top of the run folder. Its top-level figures are the off arm's. `calib.arms` holds both arms, `calib.compare` holds one row per suite, band, and formation pairing with the off value and the on value side by side, and `calib.verdict` is `on-better`, `off-better`, `no-difference`, or `on-rejected`. The same table is printed on standard error.

A flag name that `tuning.json` does not declare, a state other than `on` or `off`, a `--pair` flag also given with `--flag`, an unknown pairing or band, a `--pairing` without the formations suite, and a `--band` that no selected suite checks are refused with exit code 1 before any match is played. Each message lists the valid names.

Exit codes: 0 when every band passes and both dark-path counters are zero; 1; 2 otherwise. For a paired run: 0 when both arms have no missing statistics record, no change left unapplied, no validator violation, and no failed worker; 1; 2 otherwise. The verdict does not change the exit code.

## gate

Replay the 22 gate matches and compare their state hashes with the golden file. The gate proves that a change to the engine leaves every match exactly as it was: after every tick it hashes the full match state (positions and velocities as exact bits, stamina, cards, the rules and the restart, the clock, the teams and managers, the pending changes, the script-hook counters, the position of every random stream, and the tick's events) into a running SHA-256, and keeps a checkpoint every 1,000 ticks and at the last tick.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--golden` | file | `gate/golden.json` | The golden file to compare with. The default path is relative to the current folder, so run the command from the repository root. |
| `--fixture` | fixture id | every fixture | Play only this fixture: `seed-<seed>`, `change`, or `knockout`. Repeat it for more than one fixture. |
| `--json` | none | off | Print one JSON object per match instead of a text line. |
| `--bootstrap` | none | off | Play every fixture and write the first golden file with the portable hash set. Refused when the file already exists, and with `--fixture`. |
| `--regenerate` | none | off | Play every fixture and rewrite the golden file with this build's hashes as the portable hash set. Appends one `regenerate` entry to the ledger and drops every other hash set. Needs `--reason`. |
| `--reason` | text | none | Why the golden file is written; recorded in the new ledger entry. Needed by `--regenerate`; optional for `--bootstrap`. |
| `--debug` | none | off | Play every match with debug mode on. The compare and the standard output are unchanged; the trace records are counted and not written. Refused with `--bootstrap` and `--regenerate` before any file is read. |

The fixtures, in gate order:

- `seed-42`, `seed-1`, `seed-7`, `seed-99`, `seed-2026`, `seed-0`, `seed-18446744073709551615`, `seed-3`, `seed-11`, `seed-23`, `seed-57`, `seed-123`, `seed-314`, `seed-777`, `seed-1000`, `seed-4242`, `seed-9001`, `seed-31337`, `seed-65535`, `seed-1000003`: a 90-minute match between the default teams, both managed by the AI.
- `change`: seed 42 with both managers human. A substitution for the home team is queued at tick 60,000 (the player in lineup slot 9 off, the first bench player on), and a mentality change to attacking for the away team at tick 90,000.
- `knockout`: seed 2, a knockout match with the shipped `sample` script pack. It goes to extra time and a penalty shoot-out. The gate plays it with the real 2 ms wall-clock limit: a script hook call past the limit is not stopped, so a busy machine plays the same match, and the gate prints a warning for it; the operation budget still stops a long call.

Output: one line per match on standard output: the fixture id, `match` or `differs`, the tick count, the first 12 characters of the final hash, and for `change` and `knockout` the applied substitutions and tactics changes, or whether the match went to extra time and a shoot-out. A match that differs gets a second line that names the window of ticks in which its state first changed, for example `seed-42 differs: the state first differs between tick 30000 and tick 31000`, or the two tick counts when the match lasted longer or shorter. With `--json`, each match is one JSON object with `fixture`, `verdict`, `ticks`, `final_hash`, `window` (`from` and `to`), `detail`, `extra_time`, `shootout`, `decided_by`, `substitutions_applied`, `tactics_changes_applied`, `slow_calls` (script hook calls past the wall-clock limit), and `invalid` (`slow script` when `slow_calls` is above 0, otherwise `null`). A match with a slow hook call gets a line on standard error, for example `warning: knockout is marked invalid: slow script (1 hook call ran past the wall-clock limit); its hashes were compared`; its hashes are compared as for any other match, and the warning does not change the exit code. Standard error ends with the match count, the machine key, and the run time.

The golden file holds, in this order: the gate schema version, the state inventory version, the checkpoint spacing, the toolchain, the fixture list, the `ledger`, the `set_differences` record, and the hash sets. Since ledger entry 2 (the move to the portable hash set), the file holds one hash set, keyed `portable`, which every machine compares against. Before that entry, each machine had its own set, keyed `<os>-<arch>`, for example `windows-x86_64`. Seeds are decimal strings and hashes are 64 lowercase hex characters.

The ledger is append-only. Each entry has a `kind` (`bootstrap` for the first file, `add-machine-set` for a second machine's set of unchanged code, `regenerate` for a hash change), the `reason`, the `engine_version`, the `build` commit, the random-stream `scheme`, the `utc` time, and the `machine`: the key of the hash set the entry writes, `portable` for the portable set. A `regenerate` entry also has the `candidate` commit and a `band_result` path, `gate/bands/ledger-<index>.json`, fixed when the entry is written. The `set_differences` record has one item per pair of machines: `a`, `b`, the ids of the matches that `differ`, and the count of matches that are the `same`. A file written before the ledger existed, with a top-level `bootstrap` object, reads as a ledger of one `bootstrap` entry.

The gate refuses the file, before any match is played, when it is malformed, when a version, the checkpoint spacing, or the fixture list differs from the build's, when it has no portable hash set, when a hash set misses a match, lists one twice, or misses a checkpoint, when the ledger is empty, does not start with its only `bootstrap` entry, has an entry with no reason, or does not account for exactly the hash sets present, when a portable set has another set beside it or an `add-machine-set` entry after it, or when the `set_differences` record differs from the one the hash sets give. The message names the fault.

The write modes check their flags and the reason before they read the golden file or play a match, and write through `<file>.tmp` and a rename. A refusal, or a match that fails, leaves the golden file byte-identical. [The replay-gate guide](../how-to/replay-gate.md) says when each mode is allowed.

With `--debug`, standard error gets one line after each match, for example `trace: seed-42 307873 draws recorded, registry 307873, 595706 decisions, 2435 rule outcomes`. A match whose two draw counts differ fails the gate like a match that differs (exit 2).

Exit codes: 0 when every selected match matches; 1 when a flag is refused, the golden file or the content cannot be read or is refused, or a fixture id is unknown; 2 when a match differs or its hashed state holds a number that is not finite (the line names the field, for example `players[3].pos.x`).

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

Exit codes: 0 when every commit passes, or no commit in the range changes the file; 1 when a flag is refused, git cannot run, or a revision cannot be read; 2 when a commit breaks a rule.

## fast-model

Fit the fast model from full-engine results, check it against the full engine, or refuse a stale fit. The fast model plays a 90-minute match without playing a tick: from the two teams as they kick off it gives a final score and an event stream with every kind the full engine emits in regulation time, apart from a plugin's script note and a refused manager change. Nothing in the game plays it yet; these commands are its only users. Run the command from the repository root, where `gate/golden.json` is.

```text
engine-cli fast-model <fit|check|stale> [OPTIONS]
```

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `fit`, `check`, `stale` | action | required | What to do; see below. |
| `--fit` | path | `fast-model.json` in the content folder | The fit file `check` and `stale` read. |
| `--out` | path | `fast-model.json` in the content folder | Where `fit` writes the fit file when the check passes. |
| `--golden` | path | `gate/golden.json` | The golden file whose results the engine id names. |
| `--matches` | 1 or more | 1000 | `fit` only: full-engine matches per strength pairing in each batch. `check` reads the count from the fit file. |
| `--draws` | 1 or more | 20 | `fit` only: fast-model matches for each check-batch match. `check` reads the count from the fit file. |
| `--minutes` | 1 to 200 | 90 | `fit` only: minutes of play per full-engine match. A fit for the game uses 90; a shorter match is for tests. |
| `--jobs` | 1 or more | logical cores | Threads that play the full-engine matches. |
| `--gate-fixture` | fixture id | all 22 | Before `fit` and `check` play, replay only this gate fixture; repeatable. |
| `--report` | folder | none | Write `report.json` with every figure, its tolerance and the fit into this folder (`fit` and `check`). |

The actions:

- `fit` reads the engine id from the golden file and replays the gate fixtures; a fixture whose hashes differ stops the command, so the id the fit records is the id of the engine that played it. It then plays the fit batch and the check batch on the full engine, fits the model to the fit batch, and compares the model with the check batch. When every figure is within its tolerance it writes the fit file.
- `check` refuses a stale fit file, replays the gate fixtures, plays the check batch again with the fit file's batch size, and compares. The batches are fixed, so the same engine gives the same figures.
- `stale` compares the fit file's engine id with the golden file's. It plays no match. CI runs it on every pull request and before every release.

**The engine id.** `golden-<ledger index>-<build>-<digest>`: the golden file's last ledger entry, the commit whose code made its hashes, and the first 12 hexadecimal characters of SHA-256 over the portable hash set's fixture ids and final hashes. The golden file changes only through a regeneration with a ledger entry, so the id changes exactly when the engine's results change. After a regeneration, run `fast-model fit` in the same change.

**The batches.** Three strength levels, every attribute times 1.00, 1.075 and 1.15, give nine ordered pairings (home level, then away level). Match `k` of each pairing plays the clubs of calibration fixture `k` from the leagues generated with seed 1, each boosted by its level. The fit batch uses engine seed 1 and the check batch engine seed 2: the same clubs, other matches. With the defaults the two batches play 18,000 full matches, about 25 minutes on 8 cores.

**The model.** Each side's mean goals are `exp(base + home + attack × a + curve × a² + defence × e)`: `a` is the side's attack, the mean attribute of its six most advanced starters, and `e` the other side's defence, the mean attribute of its other five starters, each as (mean − 50) / 10; `home` applies to the home side only. The two scores share one match factor, a gamma with shape `dispersion` and mean 1 that multiplies both means, so they rise and fall together as the full engine's do; each side alone is negative binomial with that dispersion. A Dixon–Coles factor `rho` adjusts the scores 0–0, 1–0, 0–1 and 1–1, and every draw is weighted by `1 + draw`. A goal's minute is drawn from 90 shares fitted from the full engine's goals. The model plays regulation time only.

**The events.** Fouls, offsides, corners, throw-ins, goal kicks and injuries each have a side's mean count `exp(c · [1, home, o, t, o², t², o × t])`, where `o` is the side's strength and `t` the other side's, each as (strength − 50) / 10, and `home` is 1 for the home side; a count is negative binomial with its fitted `dispersion`, or Poisson when the full engine shows no excess spread. Each kind has 92 minute shares: 45 per half, and one for each half's added time. Each foul share is `exp(c · [1, home, o, t])` of the side that fouls, at most 1. A foul is played on with advantage at the advantage share; a foul not played on gives a penalty at the penalty share, or else a free kick to the fouled player's side. A foul is booked at the yellow share, and sends off straight at the red share. While a player of the side is booked, a yellow card goes to a booked player at the second-yellow share and sends him off; otherwise it goes to an unbooked player. A side's substitutions, those an injury forces among them, follow a table over the counts from 0 to the rule pack's limit: count `k` has a share in proportion to `exp(w_k + k × (own × o + other × t))`. An injured player is replaced when a substitution is left, a keeper by the bench keeper; otherwise the side plays on a player short. Each injury takes one substitution of the count, as in the full engine, and the rest are planned: timed from their own minute shares and grouped into the windows the injuries leave, so they keep the rule pack's substitution and window limits. An injury stops play at the injury-stoppage share; the rest happen while the ball is dead and add no stoppage. A goal in a half's last minute comes in its added time at the fitted share for that half, and adds no stoppage. Each half's added time is what the rule pack prices for that half's own stoppages, with the rule pack's variance and limits. The players come from the line-ups and benches the match kicks off with: a foul is weighted by aggression and tackling, and a booked player's foul without a card by the tuning's `foul_booked_factor` as well, a goal and an offside by the advanced starters, a penalty goes to the best finisher, and a goal kick to the keeper.

**The check.** For each check-batch match the fast model plays `--draws` matches from the same kick-off. The report compares 261 figures, full engine against fast model. The 49 score figures:

- per pairing (45): the home win, draw and away win shares, and the home and away goals per match;
- season (4): goals per match, the goalless share and the share of matches with ten or more goals over the three equal pairings, and the stronger side's win rate over 1.150 v 1.000 in both orders.

The 212 event figures:

- per pairing (207): for each side, fouls, offsides, yellow cards, corners, throw-ins, goal kicks, free kicks, penalties, injuries and substitutions per match; the share of matches with a player sent off; and the added seconds of each half;
- season (5): the sent-off share, yellow cards and corners per team, and throw-ins and goal kicks per match, over the three equal pairings.

A figure passes when the difference is at most `max(floor, 3.7 × √(se_full² + se_fast²))`, each standard error from that side's own results (`√(p(1 − p)/n)` for a share, `s/√n` for a mean). The floor is 0.01 for a share and 0.03 goals for a mean. The score figures use z = 3.7 and the event figures z = 4.07: a fast model equal to the full engine fails one of the 49 score figures by chance in about one fit in a hundred, and one of the 212 event figures in about one fit in a hundred. A figure with no events on either side, such as penalties in a pairing where nobody gives one, passes and says `no events`. The report prints each season figure's realism band beside it, for information only: the check asks the fast model to equal the full engine, not to sit inside the bands.

Output: the parameters, then one line per figure with the full-engine value, the fast-model value, the difference, the tolerance, its z, `pass`, `no events` or `FAIL`, and for a season figure its band; the last line counts the figures outside their tolerance. `stale` prints `the fast-model fit <file> matches the golden results: <id>`, or fails with `the fast-model fit is stale: the fit records <fit id>, the golden results are <golden id>; run engine-cli fast-model fit`. A fit file of an older format fails `check` and `stale` with `the fit file <file> has schema_version <n>; this build reads 2; run engine-cli fast-model fit`.

Exit codes: 0 when the fit is written, the check passes, or the fit matches the golden results; 1 when a flag is refused, a file cannot be read or written, a gate fixture differs, a full-engine match breaks an event-stream rule, the fit file is of an older format, or the fit is stale; 2 when a figure is outside its tolerance (`fit` then writes no fit file).

## Debug trace file

`simulate --debug-trace <file>` writes the debug trace of one match as JSON Lines. Debug mode is switched on when the match is built, before the opening kick-off; a build without the `debug-trace` cargo feature (default on) refuses the flag.

The first line is the header: `trace_version` (1), `seed`, `scheme` (the random-stream scheme id), `engine` (the build commit), `crate_version`, and `maths` (the maths library, for example `libm 0.2.16`).

Every other line is one record, in the order the engine ran it. Each record has `t`, the tick the step produces (the tick of the step's events; the opening kick-off is tick 1), and `k`, the kind:

- `draw`: one random draw. `subsystem` (`decision`, `kick`, `ball`, `laws`, `shootout`, or `fatigue`), `stream_id` (the 64-bit stream id in hex), `key` (the sub-stream key, for example `decision.pass_score team 0 squad 4`), `index` (the draw's number within its key, from 0), `value` (the draw in `[0, 1)`, written in the shortest form that reads back to the same bits), `scripted` (`true` when a test scene scripted it), and, at a draw tested against a probability, `p`: one probability, or two cumulative thresholds for a tackle (win, then foul) and a foul's card (red, then yellow). The draw recorder sits inside the stream registry's only draw call, so every draw the registry counts is recorded.

- `decision`: one decision point, with `point` (its name) and `detail`, the option scores or distances the engine chose by.
- `rule`: one rule outcome, with `point` (its name) and `detail`, the facts the law, roll, or match-control rule settled.

The decision points, with what `detail` holds:

| Point | Where | Detail |
|---|---|---|
| `carrier` | The ball carrier's choice | `carrier`; every scored pass `candidates` (`mate`, `score`); the `shot`, best `pass`, `dribble`, `hold`, and `clear` scores after any script `offsets`; the `choice` |
| `restart_pass` | A throw-in, corner, goal kick, or indirect free kick passed | `taker`, `kind`, every `candidates` score, the `target`, and `forward_fallback` when no team-mate was in range |
| `press` | The defending team's pressers | `team`, `count`, `reach`, the `pressers` with their distances |
| `cover` | Goal-side cover | `team`, the covered `attacker`, the covering `defender` and `distance`, or `null` |
| `chase` | A loose ball | Per team, the chasing `player` and `distance`, and the `keeper` who chases |
| `loose_ball` | Who takes a loose ball | The nearest `player` in reach and `distance`, or `null`; `fast`; `keeper_beaten` |
| `restart_taker` | The taker of a restart | `kind`, `team`, `taker`, and `preferred` when the law or custom named him |
| `ai_manager` | An AI manager's queued change | `team`, `change`, `code`, `minute`, `score` (own first) |
| `script_decision` | The decision hook's offsets | `carrier`, `cached`, `offsets`, and for a fresh call `result` (`value`, `failed`, or `switched_off`) and `notes` |
| `shootout_order` | The shoot-out kicking orders | `order` per team and the `keepers` |

The rule outcomes: `kick_off`, `goal`, `ball_out`, `tackle` (`p_win`, `p_foul`, `outcome`), `foul` (`card_decided` and the `card` after the rule hook), `card` (`held` for a card held back for advantage), `offside`, `injury` (each roll with its `chance`, and the injury itself), `dead_ball`, `restart_taken`, `added_time`, `extra_time_added`, `half_time`, `extra_time_kick_off`, `full_time`, `shot_block`, `shot_save` (`beaten`, `held`, or `parried`), `cross_clear`, `keeper_catch`, `shootout_start`, `shootout_save`, `shootout_kick`, `shootout_decided`, `change_applied`, `change_rejected` (with its `reason`), `send_off`, and `abandoned`.

Every draw's tick holds at least one point its kind of action is taken at, and every event's tick holds at least one point that gives it; the engine's tests check both on every tick of the 22 gate matches.

A 90-minute match takes about 310,000 draws and about 600,000 decision records (the press, the cover, and the chase are decided every tick), so its trace file is about 160 MB (seed 42: 906,015 lines, 162 MB).

## State digest file

`resimulate --state-digests <file>` writes the digest of the full match state after every tick as text lines. The state is the one the replay gate hashes (state inventory version 1): exact floats, the rules and the clock, pending changes, plugin counters, every random-stream position, and the tick's events.

1. The header line, a JSON object: `state_digests` (the file format, 1), `inventory` (the state inventory version, 1), `gate_schema` (1), `scheme` (the random-stream scheme id), and `engine` (the engine identity: commit, dirty mark, crate version, scheme, maths library, executable SHA-256, and build).
2. One line per tick: the tick and the SHA-256 of that tick's state, for example `1500 9c01…`. The ticks run from 1 with no gap.
3. `finish <sha256>`: the state after full time, with the full-time event.
4. `end <ticks> <full_time>`: the number of tick lines and `true` when the match reached full time. It is written last, after every other line is flushed, so a file with no `end` line is from a run that did not finish.

A 90-minute match gives about 283,000 lines (about 20 MB).

## Files

Every file the engine reads or writes is in [the data-file reference](data-files.md). Every socket message is in [the protocol reference](protocol.md).
