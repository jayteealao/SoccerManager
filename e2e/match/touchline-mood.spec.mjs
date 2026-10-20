// The Touchline's Mood column and out-of-position note on the built viewer, served by the
// release program's `launch`.
//
// Drive: a content copy whose home team is the version 3 fixture with its strikers relisted as
// attacking midfielders (rated highest there, and low at striker), so the shipped pick puts a
// player listed elsewhere in a striker's slot. Kick off with the whole match sent at once, open
// the Touchline one second in. Each of the eleven Mood cells is one of the five words with its
// mark, and in the same turn the cells' word keys equal the hello's kick-off mood of the same
// players; the note under the table names each starter away from his natural position, and his
// Level reads below his fresh level. No digit sits in a Mood cell or its label. Screenshots are
// kept as evidence; the test removes the content copy and data folder it made.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { CONTENT, frontDoor } from '../support/engine.mjs';
import { open, pastSplash, rewindTo, settled } from '../support/front.mjs';
import {
  contentWithPersonHome,
  evidence,
  kickOffFromTactics,
  MOOD_CELL,
  noDigitsIn,
  outOfPositionStarters,
  pauseIfPlaying,
  readTouchline,
  relistStrikers,
  removeDir,
  setupToTactics,
  tab,
} from '../support/person.mjs';

const TACTICS = JSON.parse(fs.readFileSync(path.join(CONTENT, 'tactics.json'), 'utf8'));
/// Past the end of any match: the engine sends the whole match at once.
const PAST_THE_END = 1_000_000;
/// One simulated second at 50 ticks a second: the first condition message, before play can
/// move a mood across a word.
const ONE_SECOND = 50;

const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });

/// The relisted strikers also rated low at striker, so the familiarity cost shows in Level.
function relistedLow(team) {
  relistStrikers(team);
  for (const p of team.players.filter((q) => q.position === 'AM' && q.positions.ST > 8)) {
    p.positions.ST = 8;
  }
}

test('the Touchline shows mood as a word with a mark and names the player out of position', async ({ page }, info) => {
  test.setTimeout(6 * 60_000);
  const { dir, team } = contentWithPersonHome(relistedLow);
  const engine = await frontDoor({ env: { SM_CONTENT_DIR: dir }, fastForwardTo: PAST_THE_END });
  try {
    await open(page, engine.url);
    await pastSplash(page);
    await page.locator('[data-choice="new"]').click();
    await setupToTactics(page);
    await kickOffFromTactics(page);
    const away = await outOfPositionStarters(page, team, TACTICS);
    expect(away.length, 'a starter plays away from his listed position').toBeGreaterThan(0);
    await until(page, (t) => window.__touchline.events().some((e) => e.tick > t), ONE_SECOND * 10, 60_000);
    await pauseIfPlaying(page);
    await rewindTo(page, ONE_SECOND);
    await tab(page, 'Touchline').click();
    await until(page, () => window.__touchline.view() === 'touchline', undefined, 5_000);
    await settled(page);

    const read = await readTouchline(page);
    fs.writeFileSync(evidence(info, 'touchline-mood.json'), `${JSON.stringify({ read, away }, null, 2)}\n`);
    expect(read.tick).toBe(ONE_SECOND);
    expect(read.rows.length).toBe(11);
    for (const row of read.rows) {
      expect(row.text, row.name).toMatch(MOOD_CELL);
      expect(row.label, row.name).toMatch(/^Mood: (Confident|Lifted|Steady|Low|Rattled)$/);
      // The live words at one second are the hello's kick-off words of the same players.
      expect(row.mood, row.name).toBe(read.hello[row.id]);
    }
    await noDigitsIn(page, '[data-screen="touchline"] td.mood');
    await page.screenshot({ path: evidence(info, 'touchline-mood.png') });

    // The note names each starter away from his natural position, and his level is below fresh.
    expect(read.note, 'the out-of-position note').not.toBeNull();
    for (const a of away) {
      const surname = a.name.split(' ').at(-1);
      expect(read.note.text).toContain(`${surname} plays ${a.plays}, away from his natural ${a.natural}: familiarity lowers his level.`);
    }
    const named = read.rows.filter((r) => read.note.ids.split(' ').includes(r.id));
    expect(named.length).toBe(away.length);
    expect(named.some((r) => r.level === 'below' && /^\d+ of \d+$/.test(r.levelText)), JSON.stringify(named)).toBe(true);
    await page.locator('[data-out-of-position]').scrollIntoViewIfNeeded();
    await page.screenshot({ path: evidence(info, 'touchline-out-of-position.png') });
  } finally {
    engine.cleanUp();
    removeDir(dir);
  }
});
