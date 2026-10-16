# Changelog

All notable changes to this game are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `engine-cli calibrate` reads each realism band's value measured on real matches, in place from the measured results (`--real-data`, by default `.scratch/out` in the main checkout), and prints it beside the verdict with its distance from the run's value. A headline before the table says which folder it read and how many bands have no real value there; a missing folder or file reads "no real data here" and the run goes on. The real values never change a verdict. The report adds `calib.real` and `calib.realism`.
- `engine-cli calibrate accept <BAND> --run <DIR>` accepts a band that reads `not sure` in a finished run. The next judgement of the folder marks it `not sure (accepted, not shown equivalent)`; it never counts as a pass and never changes the verdict. A band that reads pass or fail is refused by name, and an acceptance of another run is ignored with the reason.
- Content: 35 new realism bands that only report, beside the 18 that gate. Single values: fouls and offsides per team, home goal difference, the 5-1 scoreline, the per-club spread of goal difference over 34 matches, how goals per team spread against a Poisson count, shot distance, possession by third, goals by how the move began, pass completion by kind and length, distance per minute, high-speed share, the share of time in each speed zone and peak speed. Whole distributions: goals per match, the scoreline table, passes per team, shot distance and possession chains in passes and in seconds. Each reads its real value from the measured results; the bands of the realism measures need a build with the `measures` feature. The band file gains the `distribution` and `cells` measure kinds and the `histogram`, `cells` and `chains` ways to read a real sample.
- `engine-cli calibrate` rates each formation of the formations suite against the field with Nash averaging and prints the rating beside its win rate, in a table after the reported bands: the field rating is its share of wins against the maximum-entropy Nash equilibrium of the wins-minus-losses table, and a formation whose field rating exceeds its win rate by more than 0.10 is flagged. The rating only reports; it never changes the verdict or the exit code. A run that played only some pairings says so and rates nothing. The report adds `calib.nash`.
- `engine-cli calibrate proof` measures the realism gate: 10 change runs of the engine against itself with new match seeds and 5 runs of each of four broken copies of the engine (shots up about 10 percent, fouls halved, every foul booked, pass accuracy down about 5 points). It writes the self runs' pass and false-alarm rates, each copy's fail and detection rates, the bands each copy moved (a change past the joint threshold and at least the band's smallest shift), whether any copy moved each whole-distribution band, and the gate it measured to `content/gate-proof.json`. It needs a build with the `measures` feature, takes hours, and resumes when run again.
- Content: `gate-proof.json` holds the first proof of the shipped realism gate, 30 change runs on new seeds: no false alarm in the 10 self runs (their joint word is fail only because some bands sit outside their ranges), and every run of each broken copy moved at least one band. The shots copy never moved shots per team, whose smallest shift is larger than its move; the scoreline table is the one whole-distribution band no copy moved.
- The player as a person. Each player has a rating at every position, a weaker foot as a percent of his strong foot, up to eight traits of play (such as cutting inside or holding up the ball), and eleven hidden personality values. Determination, bravery and communication are visible mental attributes, and new technical attributes (marking, off-ball movement, corners, free kicks, penalties, long throws and flair) and the goalkeeping attribute eccentricity each have a job in play.
- The player panel shows his ratings by position on a pitch, his feet, his personality as one summary word with how sure the staff are and the assistant manager's view, his mood today, his traits of play, and his season: the story so far, his season goal and his form. Hover or focus an attribute to see the actions it drives.
- The Squad screen has new columns: the rating at his natural position, the rating at a chosen position, the weaker foot, form, and the season goal with the count so far. The Rated at filter keeps only the players rated at least 10, 13 or 16 at a chosen position.
- The Touchline shows each player's mood as a word with an up or down mark, and names a player who plays away from his natural position.
- The match feed shows a line when a player's strong trait of play beats an instruction, with the player, what he did, the instruction and why (under pressure or in a low mood). After the match his panel counts these.
- The launcher keeps a season record per player (starts, minutes, goals, assists, clean sheets, ratings and red cards) and a season goal, and his form and progress set his mood at kick-off in the next match.
- Game-state rules and player instructions. A tactic holds up to six rules in priority order, each "when all these conditions hold, set these primitives, the mentality or the formation". The conditions are the score, the time left, red cards for either side, the run of play (expected goals for less against over the last 15 minutes) and the team's mean energy. A rule fires through the change queue the moment its conditions hold and lapses when they stop, and the setting under it returns; the higher rule wins a primitive two rules set. New levels start at once and each player takes them up over up to 30 seconds, sooner the better he keeps to instructions; a new formation waits for a stoppage. A player instruction sets one of ten primitives for one player only: dribbling, crossing, cross type, final third, forward runs, width, pressing, tackling, on winning the ball and on losing the ball. Forward runs are refused for a player off the attack line. A person and the computer manager use the same rules, and a style may carry them.
- Content: the tactics file is version 7. Its `game_state` block sets the most rules a tactic holds, the run-of-play window, the take-up time, the range of each condition, and the primitives a player instruction may set with the inputs each sets for the one player. A style entry may carry `rules` and `player_instructions`. A version 6 tactics file converts as it loads, and no shipped tactic or style has a rule or an instruction yet, so matches play as before.
- `engine-cli primitives check --player <SLOT>` checks a player instruction: one player's instruction one level up from its default, or each choice, against none; his own statistic must move beyond the paired noise and his ten team-mates' pooled statistic must stay inside it. It needs a build with the `measures` feature.
- The computer manager substitutes by need, never by the clock. At each stoppage that allows a substitution, and at once after a goal, an injury or a sending-off, it weighs each player's tiredness, a yellow card (more for a player who tackles and fouls often, and as the referee shows more cards), a match rating below par and a knock, against what the bench offers at his slot, and makes a change when the gain clears a bar that rises as the substitutions and windows run out. The match feed, the commentary and the assistant give the reason: tired, booked, off his game or carrying a knock (`sub-booked`, `sub-form` and `sub-knock` are new `ai-decision` codes).
- A tackled player who is not injured may take a knock: he plays on, and the computer manager weighs it when it substitutes.
- Each shipped style chases a game it trails late with the run of play against it (a more attacking mentality and harder pressing) and holds a late lead (a more cautious mentality and time wasting), as game-state rules in the form a person writes. The computer manager makes no tactics change of its own.
- Time wasting is a tactical primitive, on or off, that a rule can set. `engine-cli primitives check` judges it on the restart wait of a side leading late.
- `engine-cli calibrate --rules <FILE>` plays under another rule pack, and every run reports the substitutions (per team, the minute quartiles, the reasons, the injury and knock share) against the real era of the pack's limit: the 2017/18 Wyscout leagues for 3 substitutions, and Euro 2020, the 2022 World Cup and Euro 2024 for 5. `engine-cli signatures` writes those real rows into `signatures.json`. The report adds `calib.substitutions`.

### Changed

- `engine-cli calibrate`: a build with the `measures` feature records, for every match, where shots come from, passes by kind and length, how each goal began, possession by third, possession chains in passes and in seconds, and each player's distance, time in each speed zone and peak speed, in the compact rows. The measures change no match. A build without the feature says so as a run starts and judges the bands as before. The rows are format 2; an old engine's format 1 rows are still read in a change run. The report adds `calib.rows.measures`.
- Content: the band file `realism-bands.json` is version 4. Each band keeps its range and smallest shift under `design`, and gains its record: where its real value is measured, the sample size and date, a calibration or validation label, its confidence counts, its condition (`dry`), and whether it gates (every band does). The three bands that measure engine experiments say they have no real counterpart. Version 3 and 2 files still load, and the 18 bands judge exactly as before.
- `engine-cli calibrate`: the bands that only report are judged in a table of their own, `reported bands (they never change the verdict)`, and in `calib.reported`; they never move the joint verdict, its threshold, `calib.pass`, the power targets or the exit code. Where the paired test cannot judge one (an old engine without the measures, or a per-club spread), it is judged by its range and its own error, and the row says why. A band whose real sample is under its own floor only reports. A line after the tables counts the matches each engine played. `--seed` is required only to run, not to accept. The band file gains four measure kinds (difference, scoreline, factorial ratio, club range) and more ways to read a real value. The report adds `calib.reported`, `calib.accepted`, `calib.counts` and the `judged`, `note` and `accepted` keys of each verdict row.
- `engine-cli calibrate` judges whole distributions by a Wasserstein-1 equivalence test with a margin (total variation for the scoreline table): it resamples the run's matches and the real matches, and a band passes when the upper bound of the distance is within its margin, fails when the distance is clearly beyond it, and is not sure otherwise, with a noise floor that says when a margin cannot pass. The rows print `whole sample` in the real column and the distance as the value; the report adds `upper` and `noise_floor` to a row. A promoted distribution band would join the joint test as one more coordinate. The distances agree with scipy's to 1e-9 on a committed fixture of synthetic cases.
- `engine-cli calibrate` prints `gate changed since the last proof` at the start of a run when the band file, the measures version or the judging code differs from the gate that `content/gate-proof.json` measured, one line per part. It never changes a verdict or the exit code.
- `engine-cli calibrate`: the run report is at schema version 7, up from 3, with the keys listed above; every key of version 3 stays.
- A change run of a build with the `measures` feature has a time limit of 15 minutes, as it records more for every match; its aim stays 10 minutes, and the limit of other builds stays 10 minutes.
- Matches play differently for the same seed. Mood moved by the match and shaped by personality, familiarity away from the natural position, traits of play and their overrides, the weaker foot, consistency for each action, braking by agility, and the kick-off mood from the season record all act in play. A shared floor keeps a strong player from playing as a poor one when several effects stack.
- The generated league changes once: `engine-cli generate` draws every new field, so the same seed gives a different league from earlier builds.
- Team files are version 3 and the attribute file version 4. Older team files, attribute files and replays convert as they load: the new attributes come from related ones, personality values sit at the middle of the scale, the weaker foot is 50 percent, only the listed position is rated, and no player has a trait. Match setup names a club whose file converted.
- The match stream protocol is version 7, up from 6. A squad entry can carry the player's position ratings, his feet, his traits of play, his personality as words and his kick-off mood as a word; the event message gains the `trait-override` type with its `trait.name`, `trait.cause` and `trait.instruction` fields, which the match record's event schema also lists; and the `condition` message carries each player's mood as a word. No field was removed and none changed meaning, so a version 6 message still reads. A version 4 replay file also stores the slot file and the trait catalogue it played with.
- `GET /engine/views` also answers the club's season record.
- The launcher writes `season.json` to the data folder, and a copy of each side's team file (`team-a.json`, `team-b.json`) with the player's recent form and goal progress into each match's folder.
- A saved match from 0.2.0-beta.4 resumes on that release's engine.
- Content: the tactics file is version 8. The computer manager's fixed-minute fields (`trailing_minute`, `leading_minute`, `check_interval_s`, `fatigue_energy`, `fatigue_from_minute`, `keep_for_injury_until_minute`) are gone and refused by name; its `substitutions` block and the `time_wasting` primitive are new. A version 7 file converts as it loads. The tuning file gains `knock_ratio`. Saved matches are snapshot version 20. Matches play differently for the same seed.
- The assistant on the Touchline offers substitutions only, checked at each stoppage that allows one; its tactics picks wait for the tactics board.
- Saved matches are version 19: each side also stores its rules, its player instructions, the rules in force and the change its players are still taking up, so a resumed match plays on exactly as the unbroken one. The match stream's hello and the replay record gain an optional `game_state` field with each side's rules and instructions; it is written only when a side has some, so the protocol stays at version 8. The replay record logs a rule's change with the source `rule`.

### Fixed

- The page no longer stops updating at the full-time report when two players of one side were booked at the same moment.
- A player hurt or sent off once full time is reached no longer ends the finished match a second time as abandoned.
- A keeper no longer catches his own pass the moment he plays it. A kick stays on its way from its kicker while it runs along the line he sent it, wherever he stands against the ball, so a keeper who holds the ball out in front of him no longer takes it back and plays it again in a loop. Matches play differently for the same seed: keepers caught about 75 of their own passes a match while the ball still ran away from them, now none (a pass that has slowed with nobody else on it is still the keeper's to collect, about 8 a match), and passers win back their own pass first on about 1 percent of passes, down from about 19 percent.

## [0.2.0-beta.4] - 2026-10-08

A prerelease for testers. Matches play differently from 0.2.0-beta.3 for the same seed: every action reads the players' ratings through the new attribute contract, on a scale of 1 to 20 in tenths, and a player's condition, height and age now play.

### Added

- The sensitivity rules check that every attribute, and height, age and nationality, moves the match statistic its job names, by at least a set amount from rating 8 to 16 and without carrying more than its share of the result; and that a top player beats an average one in about two of three runs of five matches. The rules and their thresholds are in `content/sensitivity.json`. A small batch of the rules runs on every pull request; the full run takes about 6,400 matches and is run by hand, with the check that a job planted at zero weight fails its rule (see the calibration guide). The full run records 12 of 40 rules passing, 13 failing and 15 not sure, and the five-match figure at 0.815 against its target of about two in three: the failing rules are kept red as the work still to do in play, mostly for actions play makes rarely or not at all, such as crosses and aerial balls. The rules and the counters they read build only with the engine's `sensitivity` feature, so a normal build plays its matches without them.
- A new page explains the player contract: the 1 to 20 scale in tenths, the stage blend, the curve, the caps on a player's condition, the proofs, and the data rules for team numbers, other rated people, relationships and history, and fixed facts.
- A Squad screen lists the whole squad in a table: age, nationality, height, condition, sharpness, attributes as whole numbers 1 to 20 in their colour bands, the hidden values in words, and the match rating averages over the last 3 and 10 matches. Select a header to sort, and a second time to reverse. Columns can be added, removed and reordered by dragging or with the keyboard, and saved as named views that the launcher keeps between launches.
- Selecting a player opens his panel: his attributes by group, the hidden values in words with how sure the club is, his body and age in words, the range his level plays between today, and his match ratings.
- The Touchline's player state shows each player's level now against his level fresh, such as 13 of 15 when tiredness has taken him down.
- Match setup notes when a club's team file is an older one whose ratings were converted to the 1 to 20 scale as it loaded.
- Every player who played gets a match rating from 1.0 to 10.0 at full time, from what he did in the match: goals, shots, passes, tackles, saves, cards, and his side's result and clean sheet. The match record lists the ratings, and the live stream sends them after the closing statistics.

### Changed

- Player ratings are on a scale of 1 to 20 with one decimal, in tenths. Every screen shows a rating as its whole number. A rating is the old value divided by 5, with nothing lost; this change of scale alone changes no match.
- Matches play differently from earlier builds for the same seed: the attribute contract, the states and the hidden values below all change play.
- Team files are version 2: ratings from 1.0 to 20.0, and each player's height, age, and nationality. A version 1 team file still loads and converts as it loads. A version 2 file with a rating out of range or off the tenth grid, or without a body field, is refused with the player and the field named.
- `engine-cli generate` writes version 2 team files with height, age, and nationality, and reports how many ratings under 1.0 it raised to 1.0.
- Tactics files are version 2: each role has positions in and out of possession, preferred actions, and a long-ball target, and each duty a scale. The shipped values play as before, and a version 1 tactics file converts as it loads.
- Every action reads a player's ratings through one attribute contract. Each attribute has a job in play, and each stage of an action (seeing, choosing, carrying out, and carrying out under pressure) blends a main attribute with up to three supports. Ratings pass through an exponential curve, so each step up the scale is worth more than the one below, and two players meet in a contest whose chance follows their difference on the curve.
- Twenty-one attributes that did nothing now have a job in play: among them heading wins aerial balls, concentration keeps defenders in place late in a match, handling holds saves, one-on-ones and command of area help keepers, and technique and agility decide who tries a chip or a take-on and who pulls it off.
- Top speed follows real sprint speeds by pace, from 29 km/h at pace 1 to 35.5 km/h at pace 20.
- Generated players follow the world spread: five tiers of clubs, with players and attributes spread around their club's level. A converted rating under 1.0 now reads as 1.0.
- An AI-managed side that starts in a formation other than the default one fields the lineup its AI manager picks for that formation's slots, so a full back plays at wing back and not a winger. `engine-cli calibrate` starts the sides of the formations suite this way.
- A player's state changes his ratings within caps. Tiredness lowers his physical ratings first and his technical and goalkeeping ratings by half, and no longer his mental ones; each kind of state may move a rating only so far (a tired body at most 2 points down), all of them together at most 5 down and 2.25 up, and no rating leaves 1 to 20.
- Every sprint costs stamina: a player's drain reads his running speed against an average player's top speed, so a slow runner at full sprint tires less than an average one.
- A player's height and age play. Height adds reach in the air and costs a little turning; an older player tires faster after the hour.
- A team file may give a player his condition for the match: sharpness, adaptation to the country, days of rest, and matches played for the club. A player short of sharpness plays his technical ratings lower, one new to a country his mental ratings, and one with little rest starts tired and is a little more likely to be injured. A value out of range is refused with the player and the field named.
- Two new modifier slots, `engine.modifier.sharpness` and `engine.modifier.adaptation`, each with an off version. The slot file is version 2; a version 1 slot file converts as it loads.
- The fast results model of `engine-cli fast-model` reads each side's strength through the attribute contract, as play reads the players, and fits its strength terms on the pairings it is checked on, so it follows how far a stronger side beats a weaker one on the new play. A favourite's tilt lets the side with the higher mean win a little more often and lose a little less often than the shared match factor alone gives; the fit file is version 3, and a version 2 fit reads with no tilt.
- Consistency and injury proneness are hidden values. Consistency sets how far a player's form moves his ratings from match to match and through a match, without making his side better or worse on average. Injury proneness replaces injury resistance: the higher it is, the more often the player is injured. No screen and no message shows either as a number: each reads as a word, with how sure the club is of it from the matches the player has played for the club, or as not yet known.
- The attribute file is version 3: each attribute's job, the stage tables, the skill gates, and which values are hidden. The tuning file is version 5: the contract block in place of the old speed fields, the world spread of generated players in place of the per-position distributions, the state caps, the body jobs, the fatigue weights per attribute group, the sprint cap and the match rating. Older files of both convert as they load, so old replays still play. A version 1 team file converts injury resistance to injury proneness (21.0 less the converted resistance) and gives every player consistency 10.0.
- The pre-match sheet's Risk column reads the injury-proneness word, and shows Not known for a player with no matches at the club.
- The match stream protocol is version 6, up from 3. The squad's ratings are sent in tenths; a squad entry carries the hidden values as words in place of the injury-resistance figure, and the player's visible attributes, height, age, nationality, build, condition, sharpness, matches for the club and his level; the condition message carries each player's level now and fresh; and the match ratings follow the closing statistics. Replays recorded in version 3 still open, with their ratings shown on the new scale and both hidden values shown as not yet known; their Squad screen shows what that version sent.
- A request to the launcher may now carry up to 32 KiB.

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
