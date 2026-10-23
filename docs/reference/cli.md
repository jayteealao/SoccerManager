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

Output: one version 3 team file for each club, named `<club.id>.json`, with every rating on 1.0 to 20.0, each player's height, age, and nationality, and his position ratings, feet, traits of play and personality values. A drawn rating under 1.0 (a goalkeeping attribute of an outfield player) is written as 1.0, and the command prints how many it lifted on standard output, for example `lifted 31 values below 1.0 to 1.0`. The tuning file's `generator.body` and `generator.person` blocks are required; without either the command exits 1.

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

Output: the page address. The page reads the engine state from `engine.json` at the same address. Its fields include `engine.version` (the release version of the program that plays the match), `launcher.version`, `match.resumed_from` (the tick a saved match continued from), and, when a save cannot resume, a `resume` block: `kind` (`older`, `newer`, `other`, `unreleased` or `previous-missing`), `saved.version`, `saved.build`, `saved.tick`, `saved.teams`, `saved.score`, `saved.millis` (the match stamp), `engines` (the two versions this program finishes) and `reason`. On the start screen the state is `idle`, and `engine.json` also carries `front-door` (true), `teams` (the sample teams match setup offers), `saved` (the newest unfinished match, or null), `settings` and `previous.version`. The page asks for actions with a POST from its own origin, each with a body of at most 32 KiB: `/engine/new-match` (with no body, a fresh match with the launch's seed and teams; with `{"home": <club id>, "away": <club id>}`, that fixture), `/engine/resume` (the saved match), `/engine/stop` (stop the match and keep its save), `/engine/quit` (stop the match, keep its save and end `launch`), `/engine/settings` (save the three settings), and `/engine/views` (save a club's named squad views). A GET of `/engine/round?home=<club id>&away=<club id>` answers the other fixtures of the round that match would meet, and a GET of `/engine/views?club=<club id>` answers that club's squad views and its players' stored match ratings. See the protocol reference for the fields and the answers.

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

The change log of a served match also holds the tactics board's changes: an `instructions` entry with the primitives it set by name, and a `game_state` entry with the side's game plans and player instructions. Both apply on the tick they were queued, with stoppage `none`, and `resimulate` queues them again on that tick. See the protocol reference, section "Record".

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
- A snapshot from the previous release (0.2.0-beta.4): the previous release's program, shipped as `previous/engine-cli` beside this program, continues it with its own `previous/content` folder. `resume` passes that program's output and exit code through, and logs `resume.delegated`.
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
| `--seed` | integer | required, except for `accept`, `count`, `agreement` and `report` | The seed of the run: the leagues, the fixtures, and every match seed. It must belong to the seed set of the run's kind. |
| `--kind` | `change`, `milestone`, `release` | `change` | The run's kind: the seed set its seed must come from, the margins of its rare bands, and, in a change run, the fixed count of paired matches it plays. See "Run kinds" below. |
| `--matches` | integer | 1000 | Matches in each suite, and in each formation pairing of the formations suite, of a run that is not a change run. A change run refuses it: it plays its kind's fixed count. |
| `--minutes` | integer | 90 | Minutes of play in each match. |
| `--jobs` | integer | the number of logical cores | Matches played at once, each on its own thread of the one process. |
| `--suite` | `all`, `equal`, `strength`, `formations`, `red-card`, `styles` | `all` | The suites to play. `all` plays the first three; `red-card` and `styles` run only when named. `styles` runs alone (see below). |
| `--pairing` | `A v B`, for example `"4-4-1-1 v 4-4-2"` | every pairing | Play only this formation pairing of the formations suite. Name the two formations in either order. Repeat it for more than one pairing. The run then plays the formations suite only. |
| `--style-pairing` | `HOME v AWAY`, for example `"counter_attack v gegenpress"` | every pairing | Play only this ordered pairing of the styles suite (`--suite styles`): the first style at home. A style may meet itself, for example `"high_press v high_press"`. Repeat it for more than one pairing. Each pairing plays the same matches as in a full run. An unknown style is refused with the list, and the flag without `--suite styles` is refused. |
| `--band` | band name, for example `goals_per_match` | every band | Judge and show only this band of the band registry, `realism-bands.json`, and play only the suites that check it. Repeat it for more than one band. An unknown name is refused with the list. |
| `--baseline` | file | none | Compare the run with an earlier `report.json`, band by band, and print the diff. |
| `--out` | folder | `runs/<run.id>` in the data folder | The run folder. Run the same command again into the same folder to resume a stopped run, or to grow it with a larger `--matches`. |
| `--keep-events` | `outliers`, `all` | `outliers` | The matches whose statistics and event files are written. `outliers` writes the files of about 1 in 16 matches, chosen by fixture key, and of every outlier (below); `all` writes every match's files. Every match has its compact row either way. |
| `--flag` | `NAME=on` or `NAME=off` | the state in `tuning.json` | Set a feature flag for the whole run. Repeat it for more than one flag. The flag must be declared in `tuning.json`. |
| `--pair` | flag name | none | Play every fixture with the flag off, then on, on the same match seeds, and compare the two arms band by band. |
| `--base` | revision | none | Make it a change run: compare this build with the old engine of the revision, and give each band its word against real football (`pass`, `fail` or `not sure`) and against the old engine (`better`, `worse`, `equivalent` or `not sure`). See "Change runs" below. Cannot be used with `--pair`. |
| `--intent` | file | none | A change run's intent: the bands the change should move and which way, and later one note per surprise and the draft setting rules to confirm. It is copied into the run folder as `intent.json` before any match plays. Without it, a change run asks at a terminal, reads the run folder's own `intent.json` when it judges a played run again, or stops before play. Only a change run takes it. See "Intent and surprises" below. |
| `--real-data` | folder | the main checkout's `.scratch/out` | The folder of measured real data that each band's real record points into. See "Real values" below. |
| `--rules` | file | the content's `rules/default.json` | Play every match under this rule pack instead, for example `crates/engine-cli/tests/fixtures/rules/three-substitutions.json` (3 substitutions in 3 windows). The substitution report reads the real era of the pack's limit. |

Output: the run report as one JSON line. Files: `run.json`, `target.json`, `ledger/`, `rows/`, `report.json`, `stats/<match.id>.json`, and `events/<match.id>.jsonl` in the run folder, and `accepted.json` once `calibrate accept` has accepted a band.

The matches play on `--jobs` threads of one process. The fixtures still to play, of every suite (and of both arms of a paired run), go into one list of work units. Each thread takes the next unit when it finishes one, so no thread waits for a suite to end. `calib.wall_ms` gives each suite the time from the start of its first unit to the end of its last. Suites overlap, and `total` is the whole list's time. `calib.runner` is `threads`. Reports of earlier builds, which could also play in worker processes, can say `processes`.

The running rule checker checks every match while the match plays. It checks two kinds of rule:

- the tick rules: players inside the pitch, players apart, the ball's speed, the ball on the spot a restart names, and a player's drift from its formation anchor;
- the twelve event rules of the match's event stream: order, kick-off first, full time last, goals and the score, the period ends, the players' sides, cards, sendings-off, substitutions and their limits, and added time.

The checker keeps no list of ticks. A match's violation count is the sum over both kinds of rule. Every match writes one compact row into `rows/<session>-<thread>.rows`. The row holds the match's unrounded counts, numerators and denominators, keyed by fixture ("The row file" in `data-files.md`). Statistics files and event files exist for the recorded matches only. These are about 1 in 16 matches, those whose fixture key is a multiple of 16, and every outlier. An outlier is a match that failed, panicked or hit a dark path, or a match the running rule checker found a violation in. A match is also an outlier when a measure lies strictly outside the 1st to 99th percentile of the same suite's earlier matches in the session. This percentile rule starts once the suite has 100 earlier matches. The measures are goals, shots, shots on target, expected goals, passes, pass accuracy, home possession, yellow cards, red cards, corners, throw-ins, and goal kicks, each over both teams. With more than one thread, which matches are earlier depends on finish order. So the percentile outliers can differ between runs, but the rows and the bands do not. The decision comes before any file is written, so `events.files_written` and `events.files_kept` are equal. `calib.rows` holds the row `format` (2), `measures`, the row `files`, the `rows` counted, and `recorded`. `measures` is `true` in a build with the `measures` feature (`cargo build --release -p engine-cli --features measures`), whose rows also hold the realism measures: shot distance, passes by kind and length, goals by phase, possession by third, possession chains, and each player's running. A build without the feature writes zeros there, prints `this build has no measures; build engine-cli with --features measures to record them` on standard error as the run starts (not in a quiet run), and judges the bands of the band file only. The measures change no match, so `calib.results_digest` is the same in both builds. `recorded` holds the recorded `matches` and the count for each reason (`sample`, `error`, `violation`, `extreme`, `all`). A match can have more than one reason. `calib.suites.<suite>.outliers` counts the suite's outliers by this rule. The bands are judged from the rows, with the statistics files' rounding, so they do not depend on which matches are recorded.

The three suites:

- `equal`: clubs of the same generated strength, drawn as clubs of the top tier. The run checks goals, shots, and possession, and every other band `realism-bands.json` gives the equal suite.
- `strength`: one club of each match has every attribute raised by the bands' boost. The run checks that the stronger club wins more than half its matches.
- `formations`: every pairing of the formations in `tactics.json`, a formation against itself included. Ten formations give 55 pairings, and each pairing plays `--matches` matches with the clubs of the equal suite, its first formation at home in every other match. Each side fields the lineup its AI manager picks for its own formation's slots, as it does for the default formation. The run checks goals per match, the share of matches with 10 or more goals, and the share of goalless matches for each pairing. `calib.formations` holds the figures of every pairing, and each of its band checks carries a `pairing` such as `4-3-3 v 4-4-2`. When every pairing finished a match, `calib.nash` rates each formation against the field, report only: `flag_margin` (0.1), `converged` and `iterations` (the solve), `matches_per_pairing` (the fewest finished matches of a pairing), and `formations`, one entry per formation in the tactics file's order with `formation`, `weight` (its weight in the maximum-entropy Nash equilibrium of the wins-minus-losses table), `win_rate` (its share of wins over every formation, itself included; a draw is not a win), `field_rating` (the same share weighted by the equilibrium), `nash_average` (wins minus losses against the equilibrium) and `flagged` (the field rating exceeds the win rate by more than `flag_margin`). The console prints it as `formation field rating (report only; ...)` after the reported bands. It never changes a band, the verdict, `calib.pass` or the exit code. A run that played only some pairings, or in which a pairing finished no match, prints `formation field rating: needs every pairing (<k> of <n> played)` and writes no `calib.nash`; in a paired run it describes the off arm, as `calib.formations` does.

- `red-card`: the controlled sending-off experiment. It is not part of `all`. The default clubs play with cards otherwise off, in four arms: a control, and the away keeper (player 11), centre-back (13), or striker (21) sent off at kick-off. Every arm plays each engine seed from `--seed` onward twice, first with the default clubs in their usual order and then with home and away swapped, so each club is the reduced side in half the matches and a difference in club strength cancels out. `--seed 1 --matches 240` (seeds 1 to 120 in both orders) is the experiment of the slow test `a_sending_off_gives_no_advantage`. An odd `--matches` plays the last seed in the usual order only. `calib.red_card` holds the control's home and away mean goals, each arm's full-side (home) and reduced-side (away) mean goals, the limit (the top of the `full_over_control` band, 1.28 today, times the control's home mean), and `pass`. Each arm has two band checks, with the arm in `pairing`: `reduced_minus_full` (the reduced side's goals less the full side's, at most 0) and `full_over_control` (the full side's goals over the control's home goals, at most 1.6).

- `styles`: every ordered pairing of two different named styles of `styles.json` (eight styles give 56 pairings), `--matches` matches each. It is not part of `all`, and it runs alone: with `--base`, `--base-binary`, `--pair`, `--baseline`, `--band` or `--pairing` it is refused. Match `j` of a pairing takes the home club of the equal suite's match `j` and gives the away side a copy of it (its club id ends in `-b`), so both sides field the same players. Each side plays its style's preset whole: it starts in the preset's tactics, in a formation drawn from the style's list by a draw seeded from the fixture key and the side, with the lineup its AI manager picks for that formation, and plays by the preset's game-state rules and player instructions. The suite judges no band and keeps no rows: it writes `report.json` (`record.kind` `styles-report`, with `run.id`, `seed`, `content.hash`, `calib.matches`, `calib.minutes`, `calib.jobs` and `calib.styles`) to the run folder, prints the same line on standard output and a table on standard error, and exits 0. `calib.styles` holds `matches_per_pairing`, `matches` (planned), `played`, `violations` (the running rule checker's count over the suite), `wall_ms`, `styles` (one row per style in file order: `style`, `matches`, `points_per_match` and its `se`, `share` of all points, `clear_losses`, the styles it loses to clearly, `dominant`, and `identity`, one row per identity statistic with `stat`, `against`, `direction`, `style_mean`, `all_mean`, `se` and `flagged`), `pairings` (one row per ordered pairing: `home`, `away`, `matches`, `home_points_per_match`, `away_points_per_match`, `goals_per_match`, `shots_per_match`, and in a `measures` build `probe`: per side, home first, each pairing's shots, goals, xG and xG per shot, shot distance, possessions, turnovers per possession, open-play losses and their share by third, regains, fast breaks, the time from a regain to a shot, pressures, the pressures and regains within 5 seconds of a loss and the time to the last pressure within 10 seconds, offsides, and the share of time the ball spends in each third), `dominant` (the flagged styles) and `identity_flags` (`style: stat`). A style loses clearly to another when, over their matches in both home orders (at least two), the 95 percent interval of its points less the other's lies wholly below 0. It is `dominant` when it loses clearly to no style or takes more than 20 percent of all the points. An identity statistic is `flagged` when the style's sides do not move it in its `direction` by more than 1.96 standard errors of the difference from the mean over every side. `gate` judges the suite whole: `pass` is true when no style is `dominant`, no identity statistic is flagged, every ordered pairing that played averages between 4 and 40 shots a match (both sides together, the bounds included), the checker found no violation and every planned match played; `dominant`, `identity_flags`, `pairings_outside` (`home`, `away`, `shots_per_match`), `violations` and `unplayed` name what fails. The table ends with one line, `styles gate: pass` or `styles gate: fail: ` and each failing item. The gate reports only: the exit code stays 0. A change run (`--base` or `--base-binary`) also plays one match per ordered pairing on the changed engine after its verdict, and never on the old engine; it adds `calib.styles` to the report, the sample's wall time as `styles` in `calib.stages`, and the table to standard error. The sample never changes a band, the verdict, `calib.pass` or the exit code.

With the defaults, `--suite all` plays 57,000 matches, and `--suite equal` plays 1,000 matches. For measured and projected run times, each with its machine and load, see [How long the runs take](../explanation/realism-harness.md#how-long-the-runs-take).

A targeted run plays only what it selects. `--pairing` plays each named pairing's `--matches` matches, with the same fixture keys, engine seeds, clubs, and home sides as in a full run, so the figures of a targeted pairing or suite equal the figures of the same pairing or suite in a `--suite all` run on the same seed. `--band` narrows `--suite` to the suites that check the named bands, the `suites` of each band in `realism-bands.json`. In the shipped file, `goals_per_match`, `ten_plus_goals_share`, and `goalless_share` belong to `equal` and `formations`; `stronger_team_win_rate` to `strength`; `reduced_minus_full` and `full_over_control` to `red-card`; every other band to `equal`. Each suite's `wall_ms` check stays. `calib.selection` names the suites, pairings, and bands the run selected.

Every fixture has a key: a hash of what the fixture is, made by the fixture scheme `fixture-key-1`. The key covers the scenario (`equal`, the strength suite's boosted side, `formations`, or the red-card arm), the world (the generated league's seed, or the default clubs and `--seed` in the red-card suite), the two clubs with the home club first, both formations by name, and a repeat number. The engine seed is stored beside the key: it comes from the same hash, except in the red-card suite, which keeps the seeds from `--seed` onward. The match identifier is the key and the run's start time. So a run with a larger `--matches` keeps every earlier fixture's key and engine seed, and a new formation in `tactics.json` leaves the other pairings' matches alone. `fixtures.scheme` in the report names the scheme.

The run folder holds the run's identity in `run.json`: the build (its commit and the SHA-256 of the program), the content, the fixtures hash, the flags set on the command line, the paired flag, the seed, the minutes, the strength boost of `realism-bands.json`, the random-number scheme, the fixture scheme, the measures version, and the band registry's version (3). The bands' ranges and smallest shifts are not part of it. The match count, the suites, the pairings, the bands judged, `--jobs`, and `--keep-events` are not part of it: they choose which work is done. The fixtures still to play are cut into work units of 8, or fewer when a suite has too few fixtures to give every thread a unit. When every match of a unit is played, its thread appends the unit's rows to its row file, then one line to `ledger/`. When the same command runs again into the same folder:

- With the same identity, the run resumes: it plays only the fixtures the ledger does not hold, and standard error says `resuming run <run.id>: <done> of <total> fixtures done, <left> to play`. A unit cut off mid-way plays again; a finished unit never does. A larger `--matches` grows the run the same way.
- With another identity, the old run's files move to `superseded/<old run.id>/`, a new run starts, and standard error names each part that differs, for example `minutes 3 -> 4` or `content <old> -> <new>`. Nothing is resumed.

The default run folder is new for every run, so resume and growth need `--out`. Two runs into one folder at the same time are not supported. The report holds `run.identity`, `calib.units` (`total` fixtures, `finished_before` this session, and `played` now), and `calib.results_digest`: the SHA-256 of every match's row in key order with the wall time and the record reasons left out, so a resumed run has the digest of an uninterrupted one.

Every band check carries `se`, its sampling error: the standard error of a mean (sd / sqrt(n)), of a share (sqrt(p (1 - p) / n)), or of a pooled ratio (the ratio estimator), over the matches judged. `wall_ms` has 0.

A baseline must be made with the same fixture scheme, and have the same seed, match count (`calib.matches`), and `fixtures.hash` as the run. A report without `fixtures.scheme` is from the old seeding scheme, whose match seeds came from the fixture's place in the run; it played other matches, so it is refused first, with `it was made with the old seeding scheme (match seeds from the fixture's place in the run); this run uses fixture keys (fixture-key-1), so the two runs played different matches; make a new baseline`, and exit code 1. The saved reports `gate/bands/ledger-N.json` are such reports: they still load and show, but cannot be a baseline. `fixtures.hash` covers the inputs that decide the fixtures: the attributes, rules, and tactics content, the trait catalogue (`traits.json`, which a generated player's traits are drawn from), the generator block of `tuning.json` after the flag states, and the two default clubs. A different `content.hash` (other tuning values or flag states) is allowed: it is the change under test. The baseline is checked before the run folder is made, and a refusal exits with code 1 and names each difference, for example `seed 7 differs from the baseline's 42`, `match count 500 differs from the baseline's 1000`, or `fixtures hash 3f2a9c01b7de differs from the baseline's 0c44e1a9f2b3`. A report without `se` on its band checks or without `fixtures.hash` is refused as made before this check existed; make a new one. `--baseline` cannot be used with `--pair`.

The diff is printed on standard error. Its first line names the baseline's `run.id` and seed, the baseline's and the run's content hashes, and `content changed` or `content unchanged`. Then one row per band, pairing, and arm that both reports judged, the time budget left out, with these columns: `suite`, `band`, `pairing`, `baseline`, `new`, `change` (new less baseline), `error` (sqrt(se_baseline^2 + se_new^2)), the mark (`noise` when the change is at most two errors, otherwise `change`), and the verdict of the new run (`pass` or `miss`). The report holds the same rows in `calib.diff`, and the baseline in `calib.baseline`. The diff does not change the exit code.

Standard error names each failing band on its own `warn` line, with `signal` `calibrate.band_failed`, the suite, the band, the pairing when there is one, the value, and the range.

A match the engine cannot play still writes its `stats/<match.id>.json`, with `outcome` `error`, the keys `error.type`, `error.code`, and `error.retriable`, and zero figures; the run goes on and leaves that match out of the bands, and a change run's joint verdict fails on it. A file that cannot be written (a row, ledger, statistics, or event file, for example on a full disk) stops the run with exit code 1; the unit it was in has no ledger line, so the same command again plays it. A run that stops on an error prints a failure record as one JSON line on standard output, with `outcome` `error` and the keys `error.type`, `error.code`, and `error.retriable`.

A match that panics is caught: its row has outcome 2 (panicked), its `stats/<match.id>.json` has `outcome` `error`, `error.type` `panic`, `error.code` `match-panicked`, and `error.retriable` `false`, and standard error says `match <key> (<suite>) panicked: <message>; recorded as failed`. The thread goes on with the next fixture, and the other matches finish. `darkpath.match_panicked` counts the panicked matches; a panic fails the run (`calib.pass` `false`, exit code 2), counts as a dark path in a paired run, and leaves the match out of the bands.

A paired run writes each arm to its own folder: `arms/off/rows/`, `arms/off/stats/`, `arms/off/events/`, and the same three under `arms/on/`. The one `report.json` stays at the top of the run folder. Its top-level figures are the off arm's. `calib.arms` holds both arms. The on arm is judged as a change run judges its engine, with the off arm in the old engine's place: `calib.compare` holds one row per suite and band with the keys of `calib.verdicts`, `value` the on arm's and `base_value` the off arm's, `word` the on arm's word against real football and `old_word` its word against the off arm. `calib.verdict` is `on-rejected` when the on arm breaks a guardrail (a dark path, a rule violation, a single-thread match more than 10 percent slower than the off arm's, or `stronger_team_win_rate` failing), and otherwise the on arm's joint word against real football: `pass`, `fail` or `not-sure`. `calib.pair_verdict` holds the verdict, the guardrails broken (`rejected`) and `words`, the counts of `better`, `worse`, `equivalent` and `not_sure` over the gating bands; `calib.pair_agreement` holds the agreement of the two arms' matches (see "Change runs"). Standard error prints the table, `paired run of flag <flag>: on against off: <b> better, <w> worse, <e> equivalent, <n> not sure`, each guardrail broken, and `verdict: <verdict>`.

A flag name that `tuning.json` does not declare, a state other than `on` or `off`, a `--pair` flag also given with `--flag`, an unknown pairing or band, a `--pairing` without the formations suite, and a `--band` that no selected suite checks are refused with exit code 1 before any match is played. Each message lists the valid names.

Exit codes: 0 when every band passes and the dark-path counters (`darkpath.change_never_applied`, `darkpath.match_without_stats`, `darkpath.match_panicked`) are zero; 1; 2 otherwise. For a paired run: 0 when both arms have no missing row, no panicked match, no change left unapplied, and no rule violation; 1; 2 otherwise. The verdict does not change the exit code. For a change run: 0 when the joint verdict is `pass`; 1; 2 when it is `fail` or `not sure`.

### Change runs

`--base REV` makes a change run: it compares this build with the old engine of `REV`, a branch, tag, or commit of the repository, which is built once and cached with its results (`calib.base`). Run it from a git checkout of the repository. Both engines play the same fixtures, so each band's change is measured on pairs of matches. The old engine plays in its own process. When `SM_LOG` is not set, that process logs warnings and its match runner's `info` lines, which include `calibrate.progress`. An old engine that writes rows of format 1, built before the realism measures, is read without them; an old engine that writes no compact rows, or rows of another format, is refused before any match plays.

```bash
engine-cli calibrate --suite equal --seed 42 --base main --out runs/change-42
```

The run plays exactly its kind's fixed count of paired matches in each suite unit (each suite, each formation pairing, each red-card arm), from the `kinds` table of `realism-bands.json` (see "Run kinds"). `--matches` is refused, naming the count: the count changes only by an edit of the band file with its CHANGELOG line. The count is kept in `target.json` in the run folder, with the run's kind, so a stopped change run resumes to it. The old engine builds with this build's cargo features (`measures`, `sensitivity`) where its revision's `crates/engine-cli/Cargo.toml` declares them; a revision that lacks one builds without it, standard error says so, and `calib.base` names it in `missing_features` beside `features`.

A change run judges the formations suite as one band per measure, pooled over every pairing it plays, and its verdict rows have no pairing. Each pairing's range checks stay in `calib.bands` with `informational: true`: they do not enter the joint verdict, but they still show a change that moves one pairing one way and another pairing the other way. A run without `--base` keeps one range check per pairing, without the mark.

Each band's change is the new engine's value less the old engine's, over the fixtures both engines finished. The test is a paired max-t bootstrap: 1999 resamples of the pairs within each suite, formation pairing, and red-card arm; in each resample, every band's change is divided by that resample's own standard error, and the largest of these is kept. The threshold `c` is the resample value that leaves 3 percent of them above it, and never less than 2.1701.

Each band gets two words. Its word against real football is the pass rule's, on the new engine: `pass` when the whole interval of the engine less the band's target sits inside its margin, `fail` when it sits wholly outside, `not sure` otherwise. Only this word decides the run. Its word against the old engine (`old_word`) crosses two tests, with `d` the change, `se` its error and `e` the band's noticeable error: the change has moved when `|d| > c × se`; it is shown equivalent when `|d| + z(0.95) × se < e` and `se > 0`.

- `equivalent`: shown equivalent, whether or not it moved.
- `better`: moved, not shown equivalent, and the new engine sits closer to the band's target (its design value, or the real value read here; for a whole-distribution band, the smaller distance from the real sample).
- `worse`: moved, not shown equivalent, and the new engine sits farther from the target.
- `not sure`: neither, or a move with no target to say which way is better; `old_reason` says why.

A moved band also has `moved`, `up` or `down`. A band judged on its range alone has no old word, and `old_reason` names why (`the old engine of <rev> has no measures feature`, `a per-club spread has no paired test`). The old word never changes the joint verdict.

The joint verdict is `fail` when any band that gates fails, or when a match panicked, a fixture has no result, a match ended with an engine error, a change was never applied, or the running rule checker found a violation. It is `pass` when every band passes, and `not sure` otherwise. A band that only reports is left out of it. A pair is dropped when either engine's match failed: `calib.dropped` holds the `planned` pairs, the `kept` pairs, the `dropped` ones and by which engine (`changed`, `old`, `both`). When more than 1 percent of the planned pairs are dropped, a joint `pass` becomes `not sure` with the reason `pairs under the fixed count: <d> of <p> planned pairs dropped` (`calib.joint.reason`); at or under 1 percent the run is judged on the kept pairs. Standard error prints the table, with the columns `band`, `suite`, `pairing`, `value`, `old`, `change`, `se`, `old engine`, `real`, `distance`, and the word, then `joint verdict: <word> (max-t threshold <c> over 1999 resamples, 3 percent false-alarm limit, <n> paired matches)`, each guard that failed it, and then the headline, `calib.headline`:

```text
kind change: fixed count equal 1000, strength 1000, formations 18 per pairing, red-card 250 per arm; not sure: 3 of 41 gating bands; accepted: 0; dropped pairs: 0
```

Every change run and paired run also measures how closely the two sides' matches agree, `calib.pair_agreement`: for each gating scalar band, the correlation over the kept pairs of each match's part in the band's value, with its 90 percent Fisher interval (`r`, `lo`, `hi`, `pairs`), and the median over the bands with a 90 percent interval from resampling the bands (`median`, `lo`, `hi`, `pairs`); `without` names a band with fewer than 4 pairs. The agreement never decides a run; the head measure of the band file sizes the old-engine count (see "Run kinds"). The run then checks the sensitivity rules of the settings the change touched (see "Touched rules" below). The rules file is loaded before any match plays, so a refused file stops the run with exit code 1 and nothing played. The rules' words are information: they do not change the joint verdict, `calib.pass` or the exit code.

### Intent and surprises

A change run asks what the change should do before anything plays: after the checks of the band file and the arguments, and before the rules file, the old engine and the first match. At a terminal it asks `What does this change do? (one line)` and then for one band per line, `<band> up` or `<band> down` with an optional size, ended by an empty line; an empty answer means the change should move no band. `--intent <FILE>` gives the same answers as a file, and a played run folder keeps its own `intent.json`. With no terminal, no `--intent` and no intent in the folder, the run stops with exit code 1: `a change run needs its intent: give --intent <FILE> or run it at a terminal; no match was played`. A band the band file does not hold is refused by name, before play.

The intent file, `intent.json` (schema version 1):

| Field | Holds |
|---|---|
| `schema_version` | 1 |
| `change` | One line on what the change does |
| `bands` | The bands the change should move: `band`, `direction` (`up` or `down`), and an optional `size` in the band's unit, only reported |
| `notes` | One note per surprise: `band`, `kind` (`side effect` or `bug`), `note`, one line, and `setting`, the content setting a side effect comes from (`<file>#/<pointer>`, for example `tuning.json#/engine/shots/save_high`), needed when the change touched more than one setting |
| `confirm` | The ids of draft setting rules you confirm |

A played run's bands and directions do not change; give the notes and confirmations with `--intent` and run the same command into the same folder.

After the joint verdict, the run reads every band's word against the old engine against the intent and flags each surprise:

- `unintended`: the band moved (`better` or `worse`, or `not sure` with a direction) and the intent does not name it.
- `missed`: the intent names it and it did not move.
- `wrong-way`: it moved the other way from the intent, or from the move a confirmed setting rule expects.

A band the change kind does not judge, and a band with no word against the old engine, is listed as not checked, with why. The run is unfinished while any surprise has no note: the headline ends `; open surprises: <n>; run unfinished`, and once every surprise has its note, `; run finished` (with `; bug notes: <n>` before it when any note is a bug). The surprises print as a table, with `calib.surprises` and `calib.finished` in the report.

A judgement again of a played run folder plays nothing: the run's rows, its old engine's cached rows, its benchmark, its styles sample and its rules stage are read from the folder (`bench.json`, `styles.json`, `rules.json`). `calib.played` counts the matches each call played, by source (`changed`, `old`, `bench`, `styles`, `rules`, `setting_rules`) and in `total`, and the run prints `matches played by this call: <n>` with the count by source.

A `side effect` note with its setting writes a draft setting rule into `setting_rules` of the content folder's `sensitivity.json`: the setting, the band, the way the band moves as the setting rises, three levels (the changed value, the midpoint and the old value, rising), status `draft`, the note, the run and the date. Its id is `sr-` and eight hex characters of the setting, the band and the direction, so the same note never writes it twice. `calib.drafts` lists each draft written, already there, confirmed or skipped, with why. A draft counts only once its id is in `confirm`: then its status is `confirmed`, a later change of its setting sets the band's expected move, and the rule plays in the next change run that touches its setting. A rule confirmed on a played run reads `confirmed after this run was played; plays in the next change run`.

### Touched rules

The rules stage plays the sensitivity rules of what the change touched, against the revision of `--base`. The change's settings are the leaf keys of the content folder's JSON files that differ from the revision's, named `<file>#/<pointer>`, with list items named by their `name`, `job` or `id` (`attributes.json#/attributes[passing]/group`). A setting selects its rules: an attribute's entry its job's rule, a stage table of `attributes.json` the rules of the attributes in that stage and the stage rules, a contract action of `tuning.json` its attributes' rules, the curve and the match rating the five-match rule, a body job its rule, and the rules file's own design the rules it sizes. A setting no rule names is listed with no rule. A changed path of engine code (under `crates/`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`) selects every rule, and so does a run with no revision to compare with: `--base-binary` alone, or a content folder outside a checkout. `calib.rules.touched` holds the reason (`code`, `content`, `no-base` or `none`), the revision compared with, the changed paths and settings, the settings with no rule, and the rules and setting rules selected.

Each job rule plays at three levels, low, middle and high, each in a third of the design matches: its word is the low-to-high word, failed or made not sure by a step that turns back. The five-match rule keeps its two copies. The stage prints a table with the columns `rule`, `levels`, `low`, `mid`, `high`, `move [95%]`, `share [95%]` and the word, the confirmed setting rules after it, then `rules stage (kind <kind>): <n> touched rules checked at <levels> levels (<why>), <failed> failed, <not sure> not sure (<seconds> s)`, where `<why>` is `engine code changed: <n>`, `<n> changed settings, <m> with no rule`, `no revision to compare with: <reason>` or `no setting changed`. A confirmed setting rule whose setting the change touched plays the first `design.setting_matches` fixtures of the equal suite on this engine at each of its levels, into `rules/<id>/l<k>` of the run folder, and is judged by a paired bootstrap of the band's per-match values; a draft lists as a draft and does not play. The job rules build only with the `sensitivity` feature (`cargo build --release -p engine-cli --features sensitivity`), so a normal build's matches pay nothing for the job probe; a build without the feature plays no job rule and prints `rules stage (kind <kind>): <why>; this build has no sensitivity rules; build engine-cli with --features sensitivity to check them`.

A run without `--base` prints a verdict table too: each band's value, its range, and `pass` or `fail`.

### Reported bands

A band only reports when its band file says `gates: false`, when its real sample is under its own `real.floor`, or when its real value's generalisation is under the file's floor. Such a band is judged by the same rules as a gating band, but apart: after the verdict table, standard error prints `reported bands (they never change the verdict):` and a second table, then `reported: <p> pass, <f> fail, <n> not sure (<a> accepted, not shown equivalent); they never change the verdict`. A reported band never enters the joint word, its threshold `c`, `calib.pass`, `calib.bands` or the exit code: the gating bands are judged exactly as if it were not there.

In a change run, a reported band is judged by the paired test when both engines give it a value, with a threshold of its own. It is judged by its range on this engine alone when the old engine's build has no measures (`range only: base has no measures`) and for a per-club spread, which has no paired form (`range only: a per-club spread has no paired test`). By its range alone, a band fails outside its range, passes inside it when the nearest edge is at least 2.1701 of its own standard errors away (a floor band, starting at 0, has no lower edge), and is `not sure` otherwise. A band with no value is `not sure`, with the reason: `no value: no match of this run carries the measures`, `no value: no club has 34 matches in this run`, or `no value: no match of this run gives it a value`. A build without the measures judges a band that reads them nowhere: it is `not sure`, with `this build has no measures`.

A whole-distribution band (`goals_distribution`, `scoreline_cells`, `passes_per_team_distribution`, and in a measures build `shot_distance_distribution`, `chain_passes_distribution` and `chain_seconds_distribution`) has as its value the distance of the run's whole sample from the real one, its range 0 to its margin. Its real column reads `whole sample`, and its distance column repeats the value. It passes when its upper bound is within the margin, fails when the lower bound is beyond it, and is `not sure` otherwise; its note gives `upper bound <u>, noise floor <f>`, or `margin below noise floor (floor <f>)` when the margin cannot pass. Its row in `calib.reported` adds `upper` and `noise_floor`. Without its real sample it has no value: `no value: no real data here: <path>: <reason>`. In a change run it is judged by the paired test when both engines give it a value, its change the distance between the two engines' samples; a shot or chain band is judged by its range alone while the old engine has no measures.

The last line after the tables counts the matches judged: `matches: <total> in total; <pairs> pairs; observations: changed engine <n>, old engine <m>` in a change run, `this engine <n>` otherwise.

### Run kinds

The `kinds` table of `realism-bands.json` holds three kinds, each with its seed set (`seeds`, `from` to `to`, inclusive), its rare-band margin factor (`rare_margin`, `factor` with its `reason`), whether it judges the rare bands (`judges_rare`, `true` when absent; the change kind's `false` leaves each rare band `not judged in a change run`, neither pass nor fail, and sizes its count on the other bands), its fixed count (`count`, matches per suite unit, by suite), the leagues its floor assumes (`leagues`) and what the count was computed from (`computed`):

| Kind | Seeds | Rare-band margin | Use |
|---|---|---|---|
| `change` | 0 to 999999 | twice the band's margin | Every change run: catches large errors in rare events. |
| `milestone` | 1000000 to 1999999 | the band's margin | A run at a milestone. |
| `release` | 2000000 to 2999999 | the band's margin | A release run. Tuning never sees these seeds: `primitives fit` refuses one before any match plays. |

A seed outside the run's kind's set is refused before anything plays, naming the kind it belongs to. The band file's load checks the 97 percent floor of every gating band at each kind's count and margins, and `calibrate` checks it again before play with the real errors it read. A kind's count per suite is the larger of two counts over the suite's pooled matches (per formation pairing, per red-card arm, divided by the units it pools): the real-test count, the fewest matches at which every gating band of the suite the kind judges holds its floor with the kind's margins and every rare band it judges reaches its minimum exposure; and the old-engine count, `max over gating scalar bands of ceil(2 (1 - r) sd^2 (z(0.95) + z(0.80))^2 / e^2)`, with `sd` the band's per-match spread, `e` its noticeable error and `r` the lower bound of its head pair agreement (`pair_agreement`, measure `head`). `calibrate count` prints the computation. `pair_agreement` also records the agreement measured before and after the random streams were split, with their intervals.

### Count the matches

```bash
engine-cli calibrate count [--kind <KIND>] [--real-data <DIR>]
```

`calibrate count` prints each kind's count, suite by suite, as the band file computes it, with the real errors read here: `<kind> <suite>: count <n> per unit over <u> unit(s); real test <m> (<band>: <why>); old-engine test <m> (<band>: <why>); set by the <real|old> test`. Then one line for each count that differs from the band file's: `<kind> <suite>: the band file holds <n>, computed <m>`. It plays nothing and needs no `--seed`. Exit codes: 0 when every count equals the band file's; 1 when one differs; 2 when a band's floor no count meets, each such band on a `refused:` line.

| Argument | Value | Default | Meaning |
|---|---|---|---|
| `--kind` | `change`, `milestone`, `release` | every kind | Print only this kind. |
| `--real-data` | folder | the main checkout's `.scratch/out` | The measured real data whose errors the floors read, as `calibrate --real-data`. |

### Agreement of two runs

```bash
engine-cli calibrate agreement --a <DIR> --b <DIR> [--out <FILE>]
```

`calibrate agreement` measures how closely two run folders of the same seed agree, from their statistics files (play both with `--keep-events all`). It joins their successful matches on match seed and, for each figure the files carry at every revision since the stream split (goals per match, and per team the shots, shots on target, fouls, corners, passes and xG), takes the correlation over the joined matches with its 90 percent Fisher interval; then the median over the figures with a 90 percent interval from resampling them. It prints `pair agreement: median <r> (90 percent interval <lo> to <hi>) over <k> band(s), up to <n> pairs` and one line per figure on standard error, and the agreement as one JSON line on standard output. It plays nothing and needs no `--seed`. Two folders with no match seed in common are refused, with exit code 1.

| Argument | Value | Default | Meaning |
|---|---|---|---|
| `--a` | folder | required | The first run folder. |
| `--b` | folder | required | The second run folder, played on the same seed. |
| `--out` | file | none | Also write the agreement to this file. |

### Accept a not-sure band

```bash
engine-cli calibrate accept <BAND> --run <DIR>
```

`calibrate accept` records that you accept a band that reads `not sure` in the last judgement of the run folder `DIR`, in `accepted.json` beside `report.json`, with the run's id and identity. It plays nothing and needs no `--seed`. The next judgement of the folder (run the same calibrate command again) marks the band's row `not sure (accepted, not shown equivalent)`: its word stays `not sure`, it is never counted as a pass, and it never changes the joint word, `calib.pass` or the exit code. A band the run never judged, and a band that reads `pass` or `fail`, are refused by name, for example `fouls_per_team reads fail in run <run.id>; only a not sure band can be accepted`, with exit code 1. An `accepted.json` of another run or identity, or one that cannot be read, is ignored: standard error says `acceptances ignored: <reason>`, and the report keeps the reason.

| Argument | Value | Meaning |
|---|---|---|
| `BAND` | band name | The band to accept, as the report names it. |
| `--run` | folder | The run folder that holds the run's `report.json`. |

### Prove the gate

```bash
engine-cli calibrate proof --seed <SEED>
```

`calibrate proof` measures the gate and writes the result to `gate-proof.json` in the content folder. It plays change runs (`--kind change`, each of the change kind's fixed count) of this build against itself, each into its own run folder under `--out` with its standard error in `<out>/<run>.log`: self runs `self-<i>`, whose changed engine plays every fixture on a new match seed while the old engine plays the planned one, on the run seed `SEED + i`; and runs `<copy>-<j>` of each broken copy that `gate-proof.json` declares, its flag on against the same build with it off, on the run seed `SEED + j`. The copies are declared as flags, state off, in a copy of the content folder at `<out>/content`, so the shipped `tuning.json` keeps no test flag. The command needs a build with the `measures` feature: a build without it prints `this build has no measures; ...` and `calibrate proof needs a build with the measures feature: ...`, and exits 1. A stopped proof resumes when the same command runs again: a finished run is judged again from its stored rows. When a run exits with an error, the proof stops and names the run and its log. After the last run, standard error prints the summary: the gate, the self runs' pass, fail and not-sure rates with their false-alarm and move rates, one line per copy (`copy <name>: fail rate <r>, detection rate <d> over <n> runs; <m> missed; bands moved: <band> <runs>, ...`), one line per whole-distribution band (`distribution band <band>: moved by <copies>` or `no copy moved it`), and the matches. The rates never change the exit code: 0 when the file is written.

| Argument | Value | Default | Meaning |
|---|---|---|---|
| `--seed` | integer | required | The first run seed. |
| `--out` | folder | `proofs/<registry>-<verdict code>` in the data folder | The folder of the proof's runs. |
| `--write` | file | `gate-proof.json` in the content folder | The proof file to write. |
| `--real-data` | folder | the main checkout's `.scratch/out`, found from the content folder | The real-data folder each run reads, as `calibrate --real-data`. |

At the start of every run, after the band registry loads, `calibrate` compares the gate with the proof in the content folder's `gate-proof.json`. The gate is three parts: the registry's digest, the measures version, and the digest of the judging code. When they all equal the proof's, nothing prints. Otherwise standard error prints `gate changed since the last proof: <part> <proof value> -> <current value>`, one line per part (`registry`, `measures`, `verdict code`; the judging code is the code of the verdict rules, the whole-distribution test, the measures, the band record, the report's checks, the real-sample reader and the run's judgement, without their unit tests); `gate changed since the last proof: no proof recorded in gate-proof.json` when the file holds no proof; and `no gate proof in this content folder` when there is no file. The line never changes a verdict, a count or the exit code.

Run a finished run's command again into its folder to judge it again: nothing plays, and standard error says `judged again from <n> stored rows; 0 matches played`. When `realism-bands.json` changed since the run was last judged, the next line names each band whose measure, suites, range, or smallest shift changed, and each band added or removed, for example `bands changed since the last judgement: goals_per_match, corners_per_team (new)`. The ranges and shifts are not part of the run's identity, so a changed band never discards the run.

The report of a change run holds `calib.verdicts` (one row per band, and per arm for the red-card suite, with `word`, `value`, `base_value`, `diff`, `se`, `smallest_shift` (the noticeable error), `old_word`, `old_reason`, `moved`, and `outside_by` when the value is outside its range), `calib.joint` (`word`, `critical`, `resamples`, `false_alarm`, `pairs`, `guards`, and `reason`), `calib.count` (the fixed count played, per suite unit, by suite), `calib.dropped`, `calib.headline`, `calib.pair_agreement`, and `calib.rules` (`checked`, the rules; `levels`, three per job rule and two for the five-match rule; `failed` and `not_sure`, the rules with that word; `built`, whether the build carries the rules; `ms`, the stage's wall time; and `rows`, one per rule with `job`, `statistic`, `levels`, `low` and `high` (the statistic at each level), `move` and `share` (each `est`, `lo` and `hi`; absent when not finite) and `word`, `pass`, `fail` or `not_sure`; the five-match row comes last, with `job` `five-match`, the pooled share and `roles`, the share per role). Every run's report holds `calib.registry` (the registry's `version` and `digest`, and `migrated_from` when the file was of version 3 or 2) and `calib.stages`, and every run but the old engine's own holds `calib.real` and `calib.realism` (see "Real values"). Every run's report holds `calib.kind`. The report's `schema.version` is `12`; version 12 adds `calib.intent` (the intent's `file`, `source` (`file`, `run folder` or `prompt`), `sha256`, `change`, `bands`, `notes` and `confirm`), `calib.surprises` (`open`, `bug_notes`, `rows`, `as_expected`, `expected` and `not_checked`), `calib.finished`, `calib.drafts`, `calib.played`, and the rules stage's `kind`, `touched`, `note`, `setting` and `matches`, with `mid` and `steps` on each job rule's row; version 11 adds `calib.kind`, `calib.count`, `calib.dropped`, `calib.headline`, `calib.pair_agreement`, `calib.pair_verdict`, the `old_word`, `old_reason` and `moved` keys of a verdict row, `calib.joint.reason`, and the `features` and `missing_features` of `calib.base`, changes `calib.compare` and `calib.verdict` of a paired run, and drops `calib.power` and the `power`, `needed` and `no_target` keys of a verdict row; version 10 adds the `pass_rule` key of a verdict row; version 9 adds `calib.styles`, a change run's styles sample (see `styles` under the suites); version 8 adds `calib.figures`, the band registry's figures (`figure`, `suite`, `value`, `se`, `today`, `build` and `change`; `value` and `change` absent in a build without the measures the figure reads); version 7 adds `calib.nash`, the formation field rating (see `formations` under the suites); version 6 adds the `upper` and `noise_floor` keys of a whole-distribution band's row (see "Reported bands"); version 5 adds `calib.reported` (the reported bands' rows, with the keys of `calib.verdicts`), `calib.accepted` (`file`, `bands`, and `ignored` with its reason), `calib.counts` (`total_matches`, `pairs`, and `observations`, one entry per engine with `engine` `changed`, `old` or `this`, its `build` and its `matches`), and the `judged` (`paired` or `range`), `note` and `accepted` keys of each verdict row; version 4 adds `calib.real`, `calib.realism`, and the `real`, `distance` and `report_only` keys of each verdict to version 3; version 3 adds the rules stage's `not_sure`, `built`, `ms` and `rows` to version 2. Every key of an older version stays, but for those version 11 drops. In a change run, `calib.pass` is `true` only when `calib.joint.word` is `pass`, as the exit code says. In any other run, `calib.verdicts` holds each band's range check without the time budget (`wall_ms`), while `calib.pass` and the exit code also need every suite's `wall_ms` check to pass.

Standard error carries structured lines beside the tables. Each line has a `signal`, a level, and fields; every line below also has `run.id`.

- `calibrate.run_started` (`info`), when the run opens its folder: `mode` (`change`, `pair`, or `evaluation`), `seed`, `matches`, `threads`, `resumed`, `fixtures_total`, and `fixtures_done`.
- `calibrate.run_superseded` (`warn`), when the run folder held a run of another identity: `old_run_id`, and `moved_to`, the folder that now holds its files.
- `calibrate.progress` (`info`), at most every 10 seconds while matches play: `fixtures_done`, `fixtures_total` (the fixtures in the work list), `elapsed_s`, `matches_per_s`, and `eta_s`, the seconds left at that rate.
- `calibrate.match_failed` (`error`), for a match that ends with an engine error: `arm`, `suite`, `fixture.key`, `match.id`, `seed`, and `error`.
- `calibrate.match_panicked` (`error`), for a match that panics: `arm`, `suite`, `fixture.key`, `match.id`, `seed`, and `panic`, the panic message. The plain `match <key> (<suite>) panicked` line stays beside it.
- `calibrate.recorded` (`info`), once per arm and suite after play ends: `arm`, `suite`, `matches` (played in this command), `recorded` (matches with any record reason), `share` (`recorded` divided by `matches`, to 4 decimals), and `extreme` (matches with the extreme-measure reason). It makes the cost of the recording rules visible. With `--keep-events all`, `share` is 1.
- `calibrate.base_play` (`info`), at the start and at the end of the old engine's play of each suite in a change run: `phase` (`start` or `end`), `base.run_id`, `suite`, `planned`, and `cache_hits`. The start line adds `to_play`. The end line adds `played`, `elapsed_s`, `exit`, and `ok`.
- `calibrate.verdict` (`info`), with a change run's joint word: `joint`, the count of each word (`pass`, `fail`, `not_sure`), `critical`, `pairs`, and `guards`.
- `calibrate.reported` (`info`), when the run has reported bands: `bands`, the count of each word (`pass`, `fail`, `not_sure`), and `accepted`.
- `calibrate.accepted_ignored` (`warn`), when the run folder's `accepted.json` is ignored: `file` and `reason`.
- `calibrate.gate_proof` (`info` when the proof measured this gate, `warn` otherwise), at the start of the run: `state` (`matches`, `changed`, `no_proof`, `no_file`, or `unreadable`), and the current gate's `registry`, `measures`, and `verdict_code`.
- `calibrate.proof` (`info`), when `calibrate proof` has written its file: the gate's `registry`, `measures` and `verdict_code`, `runs`, `self_pass_rate`, and `matches`. It has no `run.id`.
- `calibrate.verdict_failed` (`warn`), for each band that failed in a change run: `band`, `suite`, `pairing`, `value`, `lo`, `hi`, `diff`, `se`, and `smallest_shift`.
- `calibrate.run_finished` (`info`), for a run that does not end in an error: `mode`, `outcome` (`stopped`, `judged again`, or `judged`), `pass`, `verdict` (change runs only), `exit`, and `elapsed_s`. A run that ends in an error prints the failure record instead.

### Calibration report

```bash
engine-cli calibrate report --run <DIR> [--targets <FILE>]
```

`calibrate report` shows the engine against the real targets of the reference file (see [real-targets](#real-targets)), band by band, from the stored rows of a finished run folder. It plays nothing and needs no `--seed`. It reads the matches of the equal suite (clubs of the same strength) and, for each registry band that a target names in its `engine_count.band`, prints the engine's value with its standard error and each real target with its value, standard error and `engine less real`; for a whole-distribution band it prints the engine's shares, each target's shares, and their distance (total variation for the scoreline cells, Wasserstein-1 otherwise). Banding-group targets are left to the file. The bands no target names are listed last. It writes `calibration.json` in the run folder, with every target, and never writes `report.json`: the run's verdict does not change. A folder without a finished match of the equal suite is refused, with exit code 1.

| Argument | Value | Default | Meaning |
|---|---|---|---|
| `--run` | folder | required | The run folder of a finished calibrate run. |
| `--targets` | file | `gate/real-targets.json` beside the content folder | The reference file of real targets. |

### Real values

Each judgement reads every band's real value in place from the real-data folder: `--real-data`, or by default `.scratch/out` in the main checkout, found from the content folder through git's own files (a linked worktree's `.git` file names its git folder, whose `commondir` names the main checkout's). No git program runs. The real value is reported beside the verdict with its distance from the run's value, `value` minus `real`; it never changes a word, `calib.pass` or the exit code. Before the verdict table, standard error prints `real data: <folder> (<flag|default>); <n> of <total> bands without a real value; <k> with no real counterpart`, and, when the folder does not exist, `no real data here: <folder> does not exist`. Both tables gain the columns `real` and `distance`; a band without a real value reads `no real data here` there, a band that measures an engine experiment `no real counterpart`, and a band of a migrated band file `no real record`. A band that only reports ends its line with `(report only: <reason>)`. After the table, a line states the realism claim, for example `realism claim: none; no band of this run is validation evidence with a real counterpart (15 calibration bands are not counted: tuning may have read them)`.

When the folder, a file or a key is not there, the run goes on: the band reads `no real data here`, and the reason names the file. The count of bands without a real value never counts a band with no real counterpart. The old engine's own quiet run reads no real data. An executable older than band file version 5, named with `--base-binary`, refuses the version 5 band file of the run's content folder; `--base <REV>` plays with the revision's own content and is not affected.

The report holds `calib.real`: `folder`, `from` (`flag`, `default`, or `not_found` when the content folder is in no git checkout), `present`, `missing` (bands without a real value), `no_counterpart`, `missing_path` (the folder, when it does not exist), and `bands`, one entry per band with `status` (`read`, `not_here`, `no_counterpart` or `no_record`), `value`, `se`, `file_date` (when the file is newer than the band's `real.date`), `reason`, the band's `source`, `sample`, `date` and `evidence`, `measurement` (`count`, `se`, `word`), `generalisation` (`competitions`, `seasons`, `clubs`, `units`, `word`), and `report_only`. `calib.realism` holds `validation_bands`, `validation_inside`, `calibration_bands` and the `claim` line: only gating validation bands with a real counterpart count toward it. Each row of `calib.verdicts` gains `real`, `distance` and `report_only`. Standard error also carries `calibrate.real_data` (`info`): `folder`, `from`, `present`, `missing`, and `no_counterpart`.

### Stage costs

Every run ends with a table on standard error of the time and memory of each stage: `play`, `checks` (the running rule checker), `commentary`, `writing` (making the rows, ledger lines, and records), `disk` (writing them out), `judge` (folding the rows and judging the bands), and `total`. Time is thread time summed over the threads, so a stage on many threads can take longer than the run. The checker runs inside each match, so every 16th call is timed, and its time is scaled and moved from `play` to `checks`. A stage's memory is the largest heap growth of one thread while it was in the stage; for `total` it is the process's peak working set. `calib.stages` in the report holds the same figures, as `ms` and `peak_mb` by stage. The figures describe the run and never change a result.

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
- `knockout`: seed 1, a knockout match with the shipped `sample` script pack. It goes to extra time and a penalty shoot-out. The gate plays it with the real 2 ms wall-clock limit: a script hook call past the limit is not stopped, so a busy machine plays the same match, and the gate prints a warning for it; the operation budget still stops a long call.

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

**The model.** Each side's mean goals are `exp(base + home + attack × a + curve × a² + defence × e)`: `a` is the side's attack, the level of its six most advanced starters, and `e` the other side's defence, the level of its other five starters, each as (level − 10) / 2 on the 1 to 20 scale; `home` applies to the home side only. A level is read through the attribute contract, as play reads the players: each starter's mean curve value over the stages of his role (the keeper's stages for the keeper in slot 0, every other stage for an outfielder), the mean of those over the group, and the rating whose curve value that is. A side's strength is the level of all eleven. The five mean parameters are fitted so that, for each side of each pairing of the fit batch, the model's goals add up to the full engine's along each term; the clubs of the batch differ in ways the levels do not see, and a match-by-match fit lets those differences pull the strength terms away from the pairings the check compares. The other parameters, and the intercept and `home` again, are fitted on the likelihood of the final scores. The two scores share one match factor, a gamma with shape `dispersion` and mean 1 that multiplies both means, so they rise and fall together as the full engine's do; each side alone is negative binomial with that dispersion. A Dixon–Coles factor `rho` adjusts the scores 0–0, 1–0, 0–1 and 1–1, and every draw is weighted by `1 + draw`. The favourite's tilt multiplies a home win's probability by `(λ / μ)^tilt` and an away win's by `(μ / λ)^tilt`, where `λ` and `μ` are the two means, so the side with the higher mean wins a little more often and loses a little less often than the shared factor alone gives. The tilt is fitted as the mean terms are: so that, over each pairing of the fit batch, the model's wins and losses add up to the full engine's; on the likelihood of single matches the clubs' differences inside a pairing would pull it the wrong way. A goal's minute is drawn from 90 shares fitted from the full engine's goals. The model plays regulation time only.

**The events.** Fouls, offsides, corners, throw-ins, goal kicks and injuries each have a side's mean count `exp(c · [1, home, o, t, o², t², o × t])`, where `o` is the side's strength and `t` the other side's, each as (strength − 10) / 2 on the 1 to 20 scale, and `home` is 1 for the home side, fitted on the pairings as the goals are; a count is negative binomial with its fitted `dispersion`, or Poisson when the full engine shows no excess spread. Each kind has 92 minute shares: 45 per half, and one for each half's added time. Each foul share is `exp(c · [1, home, o, t])` of the side that fouls, at most 1. A foul is played on with advantage at the advantage share; a foul not played on gives a penalty at the penalty share, or else a free kick to the fouled player's side. A foul is booked at the yellow share, and sends off straight at the red share. While a player of the side is booked, a yellow card goes to a booked player at the second-yellow share and sends him off; otherwise it goes to an unbooked player. A side's substitutions, those an injury forces among them, follow a table over the counts from 0 to the rule pack's limit: count `k` has a share in proportion to `exp(w_k + k × (own × o + other × t))`. An injured player is replaced when a substitution is left, a keeper by the bench keeper; otherwise the side plays on a player short. Each injury takes one substitution of the count, as in the full engine, and the rest are planned: timed from their own minute shares and grouped into the windows the injuries leave, so they keep the rule pack's substitution and window limits. An injury stops play at the injury-stoppage share; the rest happen while the ball is dead and add no stoppage. A goal in a half's last minute comes in its added time at the fitted share for that half, and adds no stoppage. Each half's added time is what the rule pack prices for that half's own stoppages, with the rule pack's variance and limits. The players come from the line-ups and benches the match kicks off with: a foul is weighted by the shares of the player's tackle `choose` and `execute` stages (aggression and tackling lead them), and a booked player's foul without a card by the tuning's `foul_booked_factor` as well, a goal and an offside by the advanced starters, a penalty goes to the best finisher, and a goal kick to the keeper.

**The check.** For each check-batch match the fast model plays `--draws` matches from the same kick-off. The report compares 261 figures, full engine against fast model. The 49 score figures:

- per pairing (45): the home win, draw and away win shares, and the home and away goals per match;
- season (4): goals per match, the goalless share and the share of matches with ten or more goals over the three equal pairings, and the stronger side's win rate over 1.150 v 1.000 in both orders.

The 212 event figures:

- per pairing (207): for each side, fouls, offsides, yellow cards, corners, throw-ins, goal kicks, free kicks, penalties, injuries and substitutions per match; the share of matches with a player sent off; and the added seconds of each half;
- season (5): the sent-off share, yellow cards and corners per team, and throw-ins and goal kicks per match, over the three equal pairings.

A figure passes when the difference is at most `max(floor, 3.7 × √(se_full² + se_fast²))`, each standard error from that side's own results (`√(p(1 − p)/n)` for a share, `s/√n` for a mean). The floor is 0.01 for a share and 0.03 goals for a mean. The score figures use z = 3.7 and the event figures z = 4.07: a fast model equal to the full engine fails one of the 49 score figures by chance in about one fit in a hundred, and one of the 212 event figures in about one fit in a hundred. A figure with no events on either side, such as penalties in a pairing where nobody gives one, passes and says `no events`. The report prints each season figure's realism band beside it, for information only: the check asks the fast model to equal the full engine, not to sit inside the bands.

Output: the parameters, then one line per figure with the full-engine value, the fast-model value, the difference, the tolerance, its z, `pass`, `no events` or `FAIL`, and for a season figure its band; the last line counts the figures outside their tolerance. `stale` prints `the fast-model fit <file> matches the golden results: <id>`, or fails with `the fast-model fit is stale: the fit records <fit id>, the golden results are <golden id>; run engine-cli fast-model fit`. A fit file of an older format fails `check` and `stale` with `the fit file <file> has schema_version <n>; this build reads 2; run engine-cli fast-model fit`.

Exit codes: 0 when the fit is written, the check passes, or the fit matches the golden results; 1 when a flag is refused, a file cannot be read or written, a gate fixture differs, a full-engine match breaks an event-stream rule, the fit file is of an older format, or the fit is stale; 2 when a figure is outside its tolerance (`fit` then writes no fit file).

## restart-waits

Write the restart wait table, `content/restart-waits.json`, from real restart waits less the engine's own visible steps. A stoppage lasts its visible steps (the taker reaching the ball, the players the law moves walking back) plus a drawn extra wait; the table holds the extra.

```text
engine-cli restart-waits --out <FILE> [OPTIONS]
```

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--out` | path | required | Where the table goes, usually `content/restart-waits.json`. |
| `--real-data` | folder | the main checkout's `.scratch/out`, as `calibrate --real-data` | The measured real data holding `idsse_dead.csv`, the real dead spells. Only its `typ`, `dur`, `minute` and `margin` columns are read. |
| `--seeds` | `N` or `A-B` | `1-8` | Seeds of the matches that measure the visible steps. |
| `--minutes` | 1 or more | 90 | Minutes of play of each measuring match. |

The command plays the seeded matches with a zero table (every extra 0, no quick restart) and takes, per kind of stoppage, the median of the visible steps: the seconds from the ball going dead to the tick the restart is taken. Per kind it writes the 21 quantiles (0, 5, ..., 100 percent) of the real length less that median, never below 0. A real spell of a team leading from the engine's late window on is left out, since the engine applies the late-lead factor to those itself. Ball in play is never an input. The table's `source` lines name the real file, the spell counts and the measuring seeds; the steps limits, the late-lead factor and the quick-restart values are kept from the content folder's current table, or written at their starting values when it has none. The written table is checked with the same loader the engine uses. Output: one JSON line with the spell count, the spells left out, and per kind the median visible steps, the matches' restart count and the median extra.

Exit codes: 0 when the table is written; 1 when a flag is refused, the real file cannot be read or lacks a column, or the written table fails its check.

## signatures

Write the real-team signature table, `content/signatures.json`, from the fit data of the real-data folder: for each statistic a tactical primitive moves, its spread across real club seasons and the median of each quintile (see [signatures.json](data-files.md#signaturesjson)).

```text
engine-cli signatures [OPTIONS]
```

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--real-data` | folder | the main checkout's `.scratch/out`, as `calibrate --real-data` | The measured real data holding `wy2_matches.csv`, `wy2_team_matches.csv` and `wy2_events.csv.gz`, `sb2_events.csv.gz` for the StatsBomb check, `sc_dynamic_events.csv.gz` for the full-backs' runs and `tracking_runs.csv.gz` (written by `scripts/tracking-runs.py`) for the forwards' runs. A file that is absent leaves out its rows. |
| `--out` | path | `signatures.json` in the content folder | Where the table goes. |
| `--competitions` | list | `England,France,Italy,Spain` | The competitions to read, comma separated. |

The table's `substitutions` rows come from `wy2_subs.csv` with `wy2_matches.csv` (the three-substitution era, the asked Wyscout leagues) and from the `Substitution` events of `sb2_events.csv.gz` in periods 1 and 2 of Euro 2020, the 2022 World Cup and Euro 2024 (the five-substitution era, with the share forced by an injury); each era is left out when its files are absent.

The held-out sets are never read: rows of other competitions are skipped before they are parsed, a held-out competition asked for by name stops the run with `held-out set: <name> is never read`, and so does an event whose match belongs to a held-out competition. Two runs on the same files write the same bytes. Output: one JSON line with the file written, the rows, the teams and the matches.

Exit codes: 0 when the table is written; 1 when a flag is refused, a file cannot be read or lacks a column, a held-out set is reached, or a statistic has fewer than 5 teams.

## real-targets

Check the raw real data, or build the reference file of real targets, `gate/real-targets.json`. The raw files are read in place, by default from `.scratch/data` in the main checkout (found from the content folder as `calibrate --real-data` finds `.scratch/out`); only the derived figures enter the reference file, which lives in `gate/` and never ships with the game.

```text
engine-cli real-targets check-data [--data <DIR>]
engine-cli real-targets build [--data <DIR>] [--out <FILE>]
```

`check-data` counts the 110 football-data.co.uk result files (22 divisions, 2021/22 to 2025/26), the 25 FotMob tables, the 20 SoccerTrack v2 annotation files named in their download record, and a ClubElo snapshot `clubelo/clubelo-<date>.csv`. It prints each group's count against the expected count, then names each missing file (`missing: <path>`) and each file that differs from its manifest's sha256, or for SoccerTrack its recorded size (`not as recorded: <path>`).

`build` writes the reference file: per-league targets over the five seasons (goals, results, shots, fouls, corners and cards, with each target's standard error over whole matches and its spread between seasons), a target set per tier (each league with equal weight, with the spread between leagues), banding-group targets for the leagues without data (by tier and a 50-point ClubElo strength bin, the prediction of a formula fitted on the leagues with data, with its prediction error, marked `extrapolated` outside the fitted range), league-table targets (champion points, the points of the first direct relegation place, the spread of points per game, the title margin), whole distributions (every scoreline cell, goals, shots and cards per match), Wyscout duel contests per match (two rows of one duel count as one contest), and pass completion by Opta's count from the FotMob tables. Every target carries its provider, event definition, denominator, unit of observation, calibration or validation label and how the engine counts the same thing; the file credits each provider and records each raw file read with its sha256. Before any shot target, a cross-check on StatsBomb's Bundesliga 2023/24 events decides whether the result files' shot counts include blocked shots; it prints its table, and when they do not, no shot target is built. Beside the file it writes `real-names/football-data.json`, the real club names of the result files, which the packaging check keeps out of the shipped content. Two builds of the same files write the same bytes. The Wyscout held-out sets (Germany 2017/18, World Cup 2018, Euro 2016) are never opened.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--data` | folder | the main checkout's `.scratch/data` | The raw data folder. |
| `--out` | file | `gate/real-targets.json` beside the content folder | The reference file to write. |

Exit codes: `check-data` 0 when every file is there and as recorded, 1 otherwise; `build` 0 when the file is written, 1 when a raw file cannot be read.

## world-database

Build the world database, `content/world.json`, from local snapshots. The snapshots are read in place, by default from `.scratch/data` in the main checkout (found as `real-targets` finds it); the build makes no network request.

```text
engine-cli world-database build [--data <DIR>] [--out <FILE>] [--gate <DIR>] [--hand-fills <FILE>]
```

`build` reads the openfootball repositories (`openfootball/`), the newest ClubElo snapshot `clubelo/clubelo-<date>.csv`, the Wikidata clubs, leagues, associations and ClubElo match files (`wikidata/`), and the UEFA association order, the competition formats and the top-30 domestic structure (`competitions/`). It writes the world file: every country, keyed by its FIFA code, with its region and, for the 55 UEFA associations, the 2025 rank, coefficient, real top-division club count and 2026/27 places; every league that holds a club, with its tier when a source states it (the Wikidata level, an openfootball league code, the domestic structure, a hand fill, or the Wikidata level chain from a league above or below) and its real club count with the count's source; every club with a current league, with a facsimile name and short name and an Elo strength with its source (`clubelo`, `league-median`, `tier-step` or `floor`); and the continental, intercontinental and domestic cup competitions as structure (scope, region, level, season, entrants and stages). The file holds no real name: countries are codes, leagues and cups are named from the code ("ENG tier 1", "ENG cup 1"), and each club's name comes from the game's club-name generator, seeded per club and drawn again while it equals any real name of the sources or of the `gate/real-names/` lists. Before writing, the build loads the file back and scans it with the packaging check's rule; a hit writes nothing.

In the gate folder, which never ships, it writes `real-names/world.json` (the real club names the packaging check keeps out of the shipped content), `world/name-map.json` (every real name compared against, by content id where there is one), `world/gaps.csv` (one row per gap: kind, entity, content id, real name, country, missing field, reason and sources tried) and `world/coverage.json` (the countries and tiers covered, the ClubElo accounting, the Wikipedia pages and openfootball commits read, every input with its sha256, and the scan exceptions). It prints the counts of countries, leagues, clubs by strength source, competitions and gap rows by kind. Two builds of the same inputs write the same bytes.

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--data` | folder | the main checkout's `.scratch/data` | The snapshot folder. |
| `--out` | file | `world.json` in the content folder | The world file to write. |
| `--gate` | folder | `gate/` beside the content folder | Where the real names, the gap list and the coverage record go. |
| `--hand-fills` | file | `world/hand-fills.json` in the gate folder (none when absent) | Facts filled by hand from public pages, and ClubElo links refused at a spot-check. |

Exit codes: 0 when the files are written; 1 when a snapshot is missing or malformed (the error names the file and line), a facsimile name cannot be drawn, or the world file would hold a real name.

## primitives

Check or fit a tactical primitive from played matches. Only a build with the `measures` cargo feature has this command.

```text
engine-cli primitives check --primitive <NAME> [OPTIONS]
engine-cli primitives fit --primitive <NAME> --level <LEVEL> [OPTIONS]
engine-cli primitives reach --primitive <NAME> [OPTIONS]
engine-cli primitives spread [OPTIONS]
```

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--primitive` | name | required | The primitive's name in the tactics file (`check`, `fit` and `reach`). |
| `--step` | level | every step | `check`: the level to step up from. |
| `--with` | `<primitive>=<level>` | none | `check`: another primitive's level set in the upper arm of each step beside the step itself: a combined step, read against both primitives at their lower level. May be given more than once. |
| `--hold` | `<primitive>=<level>` | none | `check`: another primitive's level held in both arms of each step, so the step is read with that primitive at that level on both sides (for example `--hold on_losing_the_ball=counter_press` when checking `counter_press_length`). Holding the checked primitive itself is refused. May be given more than once. |
| `--player` | slot, 0 to 10 | none | `check`: check a player instruction instead of the team setting. The tested side's player in this formation slot takes the primitive one level up from its default (each choice of a choice primitive), against no instruction; his own statistic must move beyond the paired noise and the same statistic pooled over his ten team-mates must stay inside it. Refused with `--step`, `--with` or `--hold`, for a primitive outside the tactics file's player override list, and for a slot the instruction cannot take. |
| `--level` | level | required | `fit`: the level to fit; the default level is the base of every ratio and is never fitted. |
| `--pairs` | 1 or more | 40 | `check`: pairs of matches per step. |
| `--matches` | 1 or more | 40 (`spread`: 200) | `fit` and `reach`: matches per evaluation; `spread`: matches to play. |
| `--evals` | 1 or more | 12 | `fit`: the most evaluations of the search. |
| `--between-neighbours` | flag | off | `fit`: search only between the values of the level's neighbours (an end level: between its neighbour and the end of the input's range), for a statistic that does not move one way across the whole range. The search plays both ends and the middle first and bisects the half whose ends bracket the target (the half that holds the level's current value when both do); when neither does, the level is out of reach and takes the value nearest the target. Fit the levels beside the default first, so each end level reads its fitted neighbour. |
| `--table` | path | `signatures.json` in the content folder | `fit`: the signature table. |
| `--seed` | integer | 9 | The equal suite's seed: its clubs and fixtures. |
| `--jobs` | 1 or more | the logical cores | Matches played at once. |
| `--minutes` | 1 or more | 90 | Minutes of play per match. |
| `--out` | path | standard output only | Where the JSON result goes. |

Every run plays the equal suite's fixtures with the home club on both sides, the tested side alternating home and away, the opponent on the default tactics, and the AI's in-match tactic changes off. `check` plays each pair twice on the same seed, the tested side one level apart (or, for a choice primitive, each choice against the default choice), and passes a step when its primary statistic moves in its direction by more than 1.96 standard errors of the paired difference (a jackknife over the pairs, each statistic pooled as sum over sum). A secondary statistic is reported beside it and printed as a `secondary miss` when it does not move that far; it does not decide the step. Each step's `differences` lists every statistic the harness measures with its paired difference and that difference's noise, so a statistic a step should leave alone (for example `hold_dribble_share` on a tempo step) can be read against its noise. A failed step prints `<primitive>: <from> -> <to> fails: <stat> moved <difference> (noise <noise>, wanted <direction>)`. `fit` plays the default level once, then searches the level's first plan input by bisection until the ratio of the level's primary statistic to the default level's is within the noise of the real ratio (the level's quintile median over the middle median of `signatures.json`); it reports the value, the achieved ratio, the engine and real values, and each secondary statistic's ratio against its real quintile range. `reach` plays the first plan input at both ends of its range. `check --player` judges each primitive on the player's own count of its team statistic: take-ons per 90 (dribbling), crosses per 90 (crossing), the high and low cross shares and the cross origin (the cross type choices), the outside-box shot share (final third), the share of runs in behind (forward runs), his mean distance from the centre line while his side has the ball (width), pressures per 90 (pressing), tackles per 90 (tackling), the forward share of his passes in the regain window (on winning the ball) and his pressures in the counter-press window per loss of his side (on losing the ball). Its result is `primitives-check-player`, each step with `own`, `others` and the `differences` of every per-player statistic; a failed step prints `<primitive>: <from> -> <to> fails: slot <slot> <stat> moved ...` or `the other ten's <stat> moved ...`. `spread` plays each club against its copy at the default tactics and reports, per player with enough pressed actions, the pressed-against-free pass completion, the pressed-minus-free touch loss and the share of running tackles he keeps the ball against, with their spread across players.

Exit codes: 0 when every step passes, the fit ran, the ends were played or the spread was measured; 1 when a flag is refused, the primitive or level does not exist, or the fit reads a held-out set; 2 when a step fails.

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

The rule outcomes: `kick_off`, `goal`, `ball_out`, `tackle` (`p_win`, `p_foul`, `outcome`), `foul` (`card_decided` and the `card` after the rule hook), `card` (`held` for a card held back for advantage), `offside`, `injury` (each roll with its `chance`, and the injury itself), `dead_ball`, `restart_taken`, `added_time`, `extra_time_added`, `half_time`, `extra_time_kick_off`, `full_time`, `shot_block`, `shot_save` (`beaten`, `held`, or `parried`), `cross_clear`, `keeper_catch`, `shootout_start`, `shootout_save`, `shootout_kick`, `shootout_decided`, `change_applied`, `change_rejected` (with its `reason`), `send_off`, `form` (each draw of the players' form: the `part`, `match` or `period`, and the side or the substitute), and `abandoned`.

Every draw's tick holds at least one point its kind of action is taken at, and every event's tick holds at least one point that gives it; the engine's tests check both on every tick of the 22 gate matches.

A 90-minute match takes about 310,000 draws and about 600,000 decision records (the press, the cover, and the chase are decided every tick), so its trace file is about 160 MB (seed 42: 906,015 lines, 162 MB).

## State digest file

`resimulate --state-digests <file>` writes the digest of the full match state after every tick as text lines. The state is the one the replay gate hashes (state inventory version 2): exact floats, the rules and the clock, pending changes, plugin counters, every random-stream position, and the tick's events.

1. The header line, a JSON object: `state_digests` (the file format, 1), `inventory` (the state inventory version, 2), `gate_schema` (2), `scheme` (the random-stream scheme id), and `engine` (the engine identity: commit, dirty mark, crate version, scheme, maths library, executable SHA-256, and build).
2. One line per tick: the tick and the SHA-256 of that tick's state, for example `1500 9c01…`. The ticks run from 1 with no gap.
3. `finish <sha256>`: the state after full time, with the full-time event.
4. `end <ticks> <full_time>`: the number of tick lines and `true` when the match reached full time. It is written last, after every other line is flushed, so a file with no `end` line is from a run that did not finish.

A 90-minute match gives about 283,000 lines (about 20 MB).

## Files

Every file the engine reads or writes is in [the data-file reference](data-files.md). Every socket message is in [the protocol reference](protocol.md).
