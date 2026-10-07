# Changelog

All notable changes to this game are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- Player ratings are on a scale of 1 to 20 with one decimal, in tenths. Every screen shows a rating as its whole number. Matches play exactly as before: a rating is the old value halved, with nothing lost.
- Team files are version 2: ratings from 1.0 to 20.0, and each player's height, age, and nationality. A version 1 team file still loads and converts as it loads. A version 2 file with a rating out of range or off the tenth grid, or without a body field, is refused with the player and the field named.
- `engine-cli generate` writes version 2 team files with height, age, and nationality, and reports how many ratings under 1.0 it raised to 1.0.
- Tactics files are version 2: each role has positions in and out of possession, preferred actions, and a long-ball target, and each duty a scale. The shipped values play as before, and a version 1 tactics file converts as it loads.
- The match stream protocol is version 4: the squad's ratings are sent in tenths. Replays recorded in version 3 still open, with their ratings shown on the new scale.
- Every action reads a player's ratings through one attribute contract. Each attribute has a job in play, and each stage of an action (seeing, choosing, carrying out, and carrying out under pressure) blends a main attribute with up to three supports. Ratings pass through an exponential curve, so each step up the scale is worth more than the one below, and two players meet in a contest whose chance follows their difference on the curve.
- Twenty-one attributes that did nothing now play: among them heading wins aerial balls, concentration keeps defenders in place late in a match, handling holds saves, one-on-ones and command of area help keepers, and technique and agility decide who tries a chip or a take-on and who pulls it off.
- Top speed follows real sprint speeds by pace, from 29 km/h at pace 1 to 35.5 km/h at pace 20.
- The attribute file is version 2, with each attribute's job, the stage tables, and the skill gates. The tuning file is version 3, with the contract block in place of the old speed fields and the world spread of generated players in place of the per-position distributions. Older files of both convert as they load, so old replays still play.
- Generated players follow the world spread: five tiers of clubs, with players and attributes spread around their club's level. A converted rating under 1.0 now reads as 1.0.
- Matches play differently from earlier builds for the same seed.

## [0.2.0-beta.3] - 2026-10-05

A prerelease for testers. It replaces 0.2.0-beta.2, which was tagged but not published because two browser tests of the release build measured their timing wrongly; the game is the same. Matches play differently from 0.2.0-beta.1 for the same seed: the kick-off and the penalty positions now follow the Laws.

### Added

- The game opens on a splash screen, then a start screen with New match, Resume, Replays, Settings, Licences and about, and Quit.
- New match: pick the home club and the away club from the sample clubs. The match plays on the home club's ground.
- Each club has a home ground with its own length and width, inside the limits of the Laws of the Game.
- The other matches of the round play at the same time as your match. Their scores and events show at the moment your match clock reaches them.
- Skip to result: after one confirmation, the engine plays the rest of the match at full speed. The result is the same as when you watch the match, and the replay keeps the whole match.
- An in-match menu: return to the start screen or quit. Both save an unfinished match, and Resume continues it later. A finished match is not kept for Resume.
- At full time the match screen shows FULL TIME with the replay controls. Once the match is stored, the report offers New match, with the same two clubs, and Return to start.
- The page fills the window and picks one of five steps by its width: compact (768 to 1023 px), standard, wide, large (1920 px and wider, everything drawn 1.125 times larger) and huge (2560 px and wider, 1.375 times). Below 768 px wide the compact layout scales down to fit.
- The pitch is drawn at the screen's own pixels, so it stays sharp on a high-density screen and at the large steps.
- When no engine answers the page's connection check, the check ends and says why.
- The browser tests check every screen at five window sizes.
- Settings: the default playback speed, reduced motion, and commentary on or off. They are kept in `settings.json`.
- Licences and about: the licence notices of every library and font the game ships.
- The release ships the engine of the previous release beside the new one. A match saved by 0.2.0-beta.1 finishes on that engine and gives the same result as it would have there. A match saved by an older release is refused with a message that names its version.

### Changed

- The match viewer is rebuilt with a new look: the dark Broadcast Blue screens with the Saira fonts.
- Saved matches use snapshot format 9. The engine version, the score and the round of other matches are saved with the match.
- A kick-off places the other team outside the centre circle, and the goalkeeper stands on his line at a penalty. These are the result changes named above.
- A resumed match shows the score, the scorers and the commentary from before the save.

## [0.2.0-beta.1] - 2026-09-29

A prerelease for testers. Matches play the same on Linux and Windows, and a match plays differently from 0.1.0 for the same seed.

### Added

- `gate` plays a fixed set of matches and checks each one against the stored result, so a change that alters how a match plays is caught. `guard` checks that every commit that changes the stored results keeps the rules for doing so.
- `resimulate` plays a replay file again from the file alone and says whether the frames and the manager changes are identical. `bisect` finds the first tick at which the state of two builds of the engine differs.
- `record --changes` queues manager changes, such as substitutions and mentality changes, from a JSON file while recording a match.
- A debug trace of every random draw, decision, and rule outcome: `--debug-trace` on `simulate` and `resimulate`, and `gate --debug`.
- Replay files (format 4) hold every input of the match and every manager change that was applied, so they can be played again. Replay files are migrated forward, from format 4 on.

### Changed

- A match plays the same on Linux and Windows. The engine now uses a pure-Rust maths library and gives each part of the game its own random stream, so a match with the same seed plays differently from 0.1.0.
- The engine is about 40 percent faster per tick.
- Match results differ from the previous release for the same seed: engine maths now come from the pure-Rust `libm` crate, and random numbers come from separate keyed streams.
- Saved matches use snapshot format 7. A match saved by the previous release cannot be resumed.
- A script hook that runs past 2 ms is no longer stopped. The match is marked invalid ("slow script") and plays the same.
- Scripts can no longer call `sleep` or `timestamp`.
- An argument error now exits with 1. Exit code 2 means only a verdict, such as "differs".

### Known issues

- 130 of the 184 realism band checks are outside their target range. They are reported and not tuned in this release.
- The replay viewer offers no way to open a replay file while no engine runs. Choose Abandon first.
- The Windows setup file is not signed, so Windows SmartScreen asks before it runs. Choose More info, then Run anyway.

## [0.1.0] - 2026-09-26

The first playable match.

### Content

- Two clubs, Oakmere Rangers and Eldstead City, with generated players.
- One rule pack with the Laws of the Game: kick-offs, throw-ins, corners, goal kicks, free kicks, penalties, offside, cards, injuries, substitutions, extra time, and penalty shoot-outs.
- Ten formations: 4-4-2, 4-3-3, 4-2-3-1, 4-1-4-1, 4-4-1-1, 4-1-2-1-2, 3-5-2, 3-4-3, 5-3-2, and 5-4-1.
- Player attributes, team tactics, English match commentary, and the engine tuning values.

### Added

- A full match against a computer-managed club. The engine moves the ball and all 22 players 50 times a second, and player attributes and tactics change the result.
- The match viewer in your browser: pick a lineup and tactics, watch at 1x to 8x, and make changes and substitutions during the match.
- Live commentary, match statistics, and a match report at full time.
- A match continues after a lost connection, and a saved match plays again from its replay file.
- A Windows 11 setup file and a Linux x86_64 archive, each with a SHA-256 file.

### Known issues

- Some match figures are still outside real football: passes per team, time with the ball in play, throw-ins, and corners (about 1 per team per match). A side with ten men can still do too well.
- Some formations score more or fewer goals against 4-4-2 than real football shows.
- The Windows setup file is not signed, so Windows SmartScreen asks before it runs. Choose More info, then Run anyway.
- No macOS build yet.
