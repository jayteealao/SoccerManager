# Play your first match

In this tutorial, you generate two clubs, pick a lineup, kick off, make a substitution, and save the replay of the match. At the end, you have a replay file of a match that you managed.

You need:

- Rust 1.87 or later, with `cargo`.
- Node 22.12 or later, with `npm`, to build the match viewer.
- A desktop browser, for example Microsoft Edge, Google Chrome, or Firefox.
- A clone of this repository. Run every command from the root folder of the clone.

The match lasts about 12 minutes at the fastest playback speed.

## 1. Build the engine and the viewer

Run:

```bash
cargo build --release
npm --prefix viewer ci
npm --prefix viewer run build
```

The engine build ends with a `Finished` line, and the engine program is now in `target/release/`. The viewer build ends with a `built in` line, and the page the engine serves is now in `viewer/dist/`.

## 2. Generate two clubs

Run:

```bash
target/release/engine-cli generate --seed 2026 --clubs 2 --out my-league
```

The command prints `wrote 2 team files`. The folder `my-league` now holds two team files: `club-000007ea-00.json` and `club-000007ea-01.json`. Each file holds a club with a name, a kit, and a squad of fictional players. The same seed always gives the same two clubs.

## 3. Start the match

Run:

```bash
target/release/engine-cli serve --seed 42 --web viewer/dist --team-a my-league/club-000007ea-00.json --team-b my-league/club-000007ea-01.json
```

The command prints two lines: a port number, and a page address such as `http://127.0.0.1:50811/`. You manage the first club, the home club. The computer manages the second club.

Keep this command running until the end of the tutorial.

## 4. Open the page

Open the page address in your browser.

The page opens on **Tactics**, before kick-off. The header names the two clubs. The pitch shows eleven slots in a formation. Each slot names a player from your squad. The squad list is beside the pitch. Select the **Match** tab to see the match screen, with the score `0–0` and `Engine connected` with the engine version in the header, then select **Tactics** to come back.

## 5. Pick the lineup

1. Select a slot on the pitch, for example the fourth slot.
2. Select **Empty the picked slot**.

   The **Continue** button in the header becomes unavailable. Beside the formation, **NOT READY** tells you why: the lineup has fewer than eleven players.

3. Select a player in the squad list, then select the empty slot.

   The player moves into the slot. **READY** shows with `The lineup is legal.`, and **Continue** is available again.

4. Change **Mentality** to another value.

   The field shows the value you chose.

## 6. Kick off

Select **Continue**. The Pre-match line-ups show both teams. Select **Kick off**.

The match screen opens and the clock starts. The ball and the 22 players move on the pitch. The commentary on the right fills with events.

Select **8x** under the pitch. The clock runs eight times faster.

## 7. Make a substitution

1. Select the **Tactics** tab. The match plays on behind it.
2. In the **Substitution** area, choose a player in **Coming off**.
3. Choose a player in **Coming on**.
4. Select **Queue substitution**.

   The change shows under the queued changes, marked **Queued**. The substitution waits for the next time the ball is out of play.

5. Watch the change.

   At the next stoppage, it reads **Applied**. The commentary shows `Substitution applied`, the roles table shows the new player, and the count of substitutions left goes down by one.

6. Select the **Match** tab to watch the pitch again.

## 8. Continue after half-time

At half-time, the page pauses and opens the half-time report. The report counts the goals, cards, fouls, and set pieces of each club.

Select **Continue**. The second half starts.

## 9. Save the replay

At full time, the page opens the full-time report.

When the replay is ready, **Save replay** is available. Select it.

The browser downloads a file named `touchline-<match id>.smfx`. At full time, the engine command in the terminal ends.

## What you did

You generated two clubs, picked a lineup, changed the tactics, played a whole match, made a substitution, and saved a replay.

To watch the replay again, select **Open a replay** in the full-time report and choose the file. You can also run `target/release/engine-cli replay --fixture <file> --web viewer/dist` and open the page address it prints.

To change how the engine plays, read [the modding how-to](../how-to/modding.md).
