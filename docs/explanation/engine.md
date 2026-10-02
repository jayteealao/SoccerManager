# How the engine works

This page explains how the match engine is built and why. It is for people who maintain the engine. For the commands, see [the command-line reference](../reference/cli.md). For the data files, see [the data-file reference](../reference/data-files.md). For why every part of the engine is a module in a slot, see [The plugin contract](plugin-contract.md).

## Positions, not probabilities

Many football games decide a match minute by minute from event probabilities: a chance of a shot, a chance of a goal. This engine does not. It computes the position of the ball and of all 22 players at every tick, and the events come from what the players do. A goal happens because a player kicked the ball and the ball crossed the goal line.

The engine makes this choice because the viewer draws the match from the positions. A viewer that draws a match from event probabilities must invent the movement between events. This viewer draws only positions that the engine computed, so the picture on the pitch is the match itself. The cost is compute: the engine does much more work for each match than an event-probability engine. The benchmark holds the cost to a budget of 2 seconds for each 90-minute match on one processor thread.

## The agent model

Each player is an agent. At each tick, each agent decides what to do, and then moves.

- The player with the ball scores each option: a shot, a pass to each open team-mate, a dribble, a clearance, or holding the ball. The player takes the option with the highest score. The player's attributes and the team's tactics change the scores, so a better passer passes more and a team told to play direct passes longer.
- The players without the ball either press the ball or hold a position in the formation. The pressing instruction sets how many players press and from how far. The formation, the mentality, and the other team instructions move each player's anchor position.
- Each player moves toward the target with steering behaviours: seek, arrive, and separation. A player's pace and acceleration limit the movement. Fatigue reduces pace, passing, finishing, and decisions as the player's energy falls.

## The tick loop

The engine runs at a fixed step of 20 ms of match time: 50 ticks for each second of match time. The number of ticks depends only on match time, never on the speed of the computer. A slow computer takes longer to compute a match, but it computes the same ticks.

Each tick runs these steps in order:

1. Each agent decides.
2. Each agent steers and moves.
3. The ball moves: flight, bounce, and roll.
4. The engine decides who has the ball.
5. The referee applies the laws and runs the clock.
6. Fatigue and injuries change the players.
7. The AI manager reviews the match.
8. When this tick starts a stoppage, the queued changes apply.
9. The engine separates players who overlap.

The fixed order is part of the result. The same inputs in the same order give the same tick.

## The referee

The referee runs inside the tick loop, not after it. The referee owns the phase of play: the ball is live, the ball is dead and waits for a restart, or the match is over. The referee detects the ball out of play, offside, fouls, and cards, and it sets the restart: a throw-in, a goal kick, a corner, a free kick, a penalty, or a kick-off. While the ball is dead, the referee sets the target of each player, so the players walk to their restart positions.

Every number the referee uses comes from the rule pack in the content folder: the length of each half, the added time, the substitution limit, and which stoppages admit a change. The tick loop contains no football rule as a constant.

## The stoppage-gated change queue

A manager can make a change at any time, but the change applies only when the ball is dead. This is how real football works: a substitute waits at the touchline for a stoppage.

Each change goes into a queue. When a tick starts a stoppage, the engine reads the rule pack to find if that kind of stoppage admits tactics, substitutions, or both. It then applies the waiting changes that the stoppage admits, substitutions first. A change that the rules refuse, such as a sixth substitution, gets a rejection with the reason. The page shows each change as a chip with its state: queued, applied, or rejected.

The queue also keeps the result the same for the same inputs. A change applies at a tick that the match decides, not at the moment the manager clicked.

## The snapshot

At each stoppage, the engine writes a snapshot of the whole match to a file: the clock, the score, each player, the change queue, the stream scheme, and the exact position of every random stream the match has used. The engine replaces the file at each stoppage. Snapshot version 7 added the stream scheme and the list of streams; the engine refuses a snapshot of an older version.

A snapshot lets a match continue after a crash. The launcher restarts the engine from the newest snapshot, and the match continues from that stoppage with the same score and the same clock. A resumed match plays tick for tick as the match would have played without the crash.

A stoppage is the right time for a snapshot because the ball is dead: no pass or tackle is in progress, so the state is small and every change that waits in the queue is known.

## The seedable random-number generator

Every random draw in the engine comes from the generator ChaCha8, created from the seed of the match. The engine never uses a random source that the seed does not control. The same seed, the same inputs, and the same engine version give the same match, byte for byte, on every supported computer.

Every match draw goes through the stream registry. A fixed table gives each draw a key: the part of the engine the draw belongs to, the kind of action, and the player who acts (or a named match key when no one player acts). A player is named by his place in the squad, so a substitute gets his own key. Matches play stream scheme 1: each key has its own stream from the match seed, so extra draws for one key leave every other key's sequence unchanged. This does not promise that play after a different decision stays the same: a different decision moves the ball and the players, and later draws are then taken for other keys. Scheme 0, one shared stream read in draw order, was played until ledger entry 2 (the move to keyed streams) and is gone. A committed digest fails a test when the table, the key-to-stream derivation, or the draw conversion changes without a new scheme number. In a debug or test build, a draw on a key that the table does not hold fails and names the key.

The snapshot stores the scheme and the exact position of every stream used. This is why a resumed match continues the same random sequences, including for a key that the match first uses after the resume. The engine refuses a snapshot of scheme 0, of an unknown scheme, or with a malformed stream entry, and names the fault.

## The watchdog mark

A script pack's hook call has two limits. The operation budget counts script operations; the count is the same on every machine, so the budget stops a long call in the same place everywhere. After three such stops in a row, the hook is switched off for the rest of the match.

The second limit is 2 ms of wall-clock time. A call that runs past it is not stopped: the call's result stands, and the match is marked invalid (`match.invalid: "slow script"` in the match statistics, with `script.slow_calls`, and one `match.invalid` log line). Wall-clock time differs between machines. If a slow call were stopped, a slow or busy machine would play a different match from the same seed and inputs. The mark is not a failure, writes no event, and is not in the snapshot or in the replay gate's hashed state.

The replay gate plays with the real clock. It prints a warning for a marked match and still compares its hashes, so a warning never hides a difference.

## Determinism across computers

The engine uses floating-point numbers for positions and velocities. Addition, multiplication, division, and square roots give the same bits on every computer the engine supports, because they are correctly rounded. Sines, cosines, exponents, arctangents, and powers are not: each platform maths library can round the last bit differently, and over thousands of ticks a different last bit can change who wins a tackle.

So every engine sine, cosine, exponent, arctangent, and integer power goes through one maths module (`engine::math`), which calls the pure-Rust `libm` crate. `libm` gives the same bits on every computer. A clippy rule on the engine crate (`crates/engine/clippy.toml`) refuses the platform maths methods, in both the method form (`x.sin()`) and the path form (`f64::sin(x)`), and the glam angle methods built on them, so a new platform call fails the pull-request check.

The replay gate proves the result. It plays 22 fixed matches and hashes the full match state after every tick: the exact bits of every position and velocity, the rules, the pending changes, the script-hook counters, the position of every random stream, and the tick's events. Linux and Windows must both match one portable hash set in the golden file. A change that moves any of that state on either computer fails the gate at the window of ticks where the state first changed.

The promise has limits. It covers the supported computers (64-bit x86 Linux and Windows) and the pinned Rust toolchain. A new toolchain or a new processor family needs a gate run before the promise extends to it. The snapshot records the build and the content it came from, and the engine refuses a snapshot from another build or other content.
