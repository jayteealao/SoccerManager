# Play your first match

In this tutorial, you generate two clubs, pick a lineup, kick off, make a substitution, and save the replay of the match. At the end, you have a replay file of a match that you managed.

You need:

- Rust 1.87 or later, with `cargo`.
- A desktop browser, for example Microsoft Edge, Google Chrome, or Firefox.
- A clone of this repository. Run every command from the root folder of the clone.

The match lasts about 12 minutes at the fastest playback speed.

## 1. Build the engine

Run:

```bash
cargo build --release
```

The build ends with a `Finished` line. The engine program is now in `target/release/`.

## 2. Generate two clubs

Run:

```bash
target/release/engine-cli generate --seed 2026 --clubs 2 --out my-league
```

The command prints `wrote 2 team files`. The folder `my-league` now holds two team files: `club-000007ea-00.json` and `club-000007ea-01.json`. Each file holds a club with a name, a kit, and a squad of fictional players. The same seed always gives the same two clubs.

## 3. Start the match

Run:

```bash
target/release/engine-cli serve --seed 42 --web web --team-a my-league/club-000007ea-00.json --team-b my-league/club-000007ea-01.json
```

The command prints two lines: a port number, and a page address such as `http://127.0.0.1:50811/`. You manage the first club, the home club. The computer manages the second club.

Keep this command running until the end of the tutorial.

## 4. Open the page

Open the page address in your browser.

The header shows the two club names, the score `0–0`, and `Engine connected` with the engine version. The pitch shows eleven slots in a formation. Each slot names a player from your squad. The squad list is under the pitch.

## 5. Pick the lineup

1. Select a slot on the pitch, for example the fourth slot.
2. Select **Empty the picked slot**.

   The **Kick off** button becomes unavailable. The text above the button tells you why: the lineup has fewer than eleven players.

3. Select a player in the squad list, then select the empty slot.

   The player moves into the slot. The text above the button reads `The lineup is ready.` and **Kick off** is available again.

4. In the **Tactics** panel on the right, change **Mentality** to another value.

   The panel shows the value you chose.

## 6. Kick off

Select **Kick off**.

The clock starts. The ball and the 22 players move on the pitch. The match feed on the right fills with events.

Select **8x** under the pitch. The clock runs eight times faster.

## 7. Make a substitution

1. In the **Substitution** area of the **Tactics** panel, choose a player in **Player coming off**.
2. Choose a player in **Substitute coming on**.
3. Select **Queue substitution**.

   A chip with the word **Queued** shows in the **Tactics** panel. The substitution waits for the next time the ball is out of play.

4. Watch the chip.

   At the next stoppage, the chip reads **Applied**. The match feed shows `Substitution applied`, the lineup panel shows the new player, and the count of substitutions left goes down by one.

## 8. Continue after half-time

At half-time, the page pauses and opens the half-time report. The report counts the goals, cards, fouls, and set pieces of each club.

Select **Continue**. The second half starts.

## 9. Save the replay

At full time, the page opens the full-time report.

Select **Save replay**.

The browser downloads a file named `touchline-<match id>.smfx`. At full time, the engine command in the terminal ends.

## What you did

You generated two clubs, picked a lineup, changed the tactics, played a whole match, made a substitution, and saved a replay.

To watch the replay again, select **Open a replay** in the full-time report and choose the file. You can also run `target/release/engine-cli replay --fixture <file> --web web` and open the page address it prints.

To change how the engine plays, read [the modding how-to](../how-to/modding.md).
