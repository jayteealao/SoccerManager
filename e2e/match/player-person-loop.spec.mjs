// The player as a person, the whole loop on one launch and one data folder, on the built
// viewer served by the release program's `launch`:
//
// 1. Before kick-off, a player's panel from the Squad screen shows every new section.
// 2. Hovering Crossing lists the actions it drives.
// 3. At kick-off the Touchline names the player out of position.
// 4. A trait override shows in the feed.
// 5. At full time, without a relaunch, the panel of the first player to hold up the ball
//    counts his overrides and the season block holds the match: one app and a goal progress
//    word.
// 6. The next match on the same launch: the Touchline's mood words one second in are that
//    match's hello words, or for a player on the edge of a word the next word toward steady.
//
// The content copy is the version 3 fixture as the home team with its strikers relisted (so
// the shipped pick plays someone out of position), holding up the ball as a strong trait of
// every outfield player, and a tuning that makes it beat a faster tempo whenever the player is
// pressed. Each step keeps a
// screenshot; the test removes the content copy and data folder it made.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { CONTENT, frontDoor } from '../support/engine.mjs';
import { open, pastSplash, rewindTo, settled, toFullTime } from '../support/front.mjs';
import { clearPointer, feedRows } from '../support/page.mjs';
import {
  contentWithPersonHome,
  evidence,
  firstHolder,
  holdUpForAll,
  kickOffFromTactics,
  leavePanel,
  MOOD_CELL,
  outOfPositionStarters,
  pauseIfPlaying,
  plantOverride,
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
const panel = (page) => page.locator('[data-screen="player"]:visible');

async function openPanel(page, index) {
  if ((await page.evaluate(() => window.__touchline.view())) !== 'squad') {
    await tab(page, 'Squad').click();
    await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);
  }
  await page.locator(`[data-screen="squad"]:visible tbody tr[data-row="${index}"] button.who`).click();
  await until(page, () => window.__touchline.view() === 'player', undefined, 10_000);
  await expect(panel(page)).toBeVisible();
  await settled(page);
}

/// The Touchline one second in, once the whole match is stored.
async function touchlineAtKickOff(page) {
  await until(page, () => window.__touchline.events().some((e) => e['event.type'] === 'full-time'), undefined, 180_000);
  await pauseIfPlaying(page);
  await rewindTo(page, ONE_SECOND);
  await tab(page, 'Touchline').click();
  await until(page, () => window.__touchline.view() === 'touchline', undefined, 5_000);
  await settled(page);
  return readTouchline(page);
}

test('the player as a person through one match and into the next', async ({ page }, info) => {
  test.setTimeout(12 * 60_000);
  const { dir, team } = contentWithPersonHome((t) => {
    relistStrikers(t);
    holdUpForAll(t);
    for (const p of t.players.filter((q) => q.position === 'AM' && q.positions.ST > 8)) {
      p.positions.ST = 8;
    }
  });
  plantOverride(dir, 'holds_up_ball', 1);
  const lomrick = team.players.findIndex((p) => p.name === 'Alvin Lomrick');
  const engine = await frontDoor({ env: { SM_CONTENT_DIR: dir }, fastForwardTo: PAST_THE_END });
  try {
    await open(page, engine.url);
    await pastSplash(page);
    await page.locator('[data-choice="new"]').click();
    await setupToTactics(page);
    await page.locator('select[data-instruction="tempo"]:visible').selectOption({ label: 'Direct' });

    // 1. The panel from the Squad screen, every section.
    await openPanel(page, lomrick);
    for (const section of ['positions', 'feet', 'personality', 'state', 'traits']) {
      await expect(panel(page).locator(`[data-section="${section}"]`)).toBeVisible();
    }
    await expect(panel(page).locator('[data-trait="holds_up_ball"]')).toContainText('Strong');
    await expect(panel(page).locator('[data-season]')).toHaveAttribute('data-season', 'empty');
    await clearPointer(page);
    await page.screenshot({ path: evidence(info, 'loop-1-panel.png') });

    // 2. The hover on Crossing.
    await panel(page).locator('[data-attr="crossing"]').hover();
    await expect(panel(page).locator('[data-hover="crossing"]')).toContainText('Crossing drives');
    await settled(page);
    await page.screenshot({ path: evidence(info, 'loop-2-hover.png') });
    await clearPointer(page);

    // 3. Kick off; the Touchline names the player out of position.
    await leavePanel(page);
    await until(page, () => window.__touchline.view() === 'tactics', undefined, 10_000);
    await kickOffFromTactics(page);
    const away = await outOfPositionStarters(page, team, TACTICS);
    expect(away.length).toBeGreaterThan(0);
    const first = await touchlineAtKickOff(page);
    expect(first.note, 'the out-of-position note').not.toBeNull();
    for (const a of away) {
      expect(first.note.text).toContain(`${a.name.split(' ').at(-1)} plays ${a.plays}, away from his natural ${a.natural}`);
    }
    await page.screenshot({ path: evidence(info, 'loop-3-out-of-position.png') });

    // 4. An override in the feed.
    const overrides = await page.evaluate(() => window.__touchline.events().filter((e) => e['event.type'] === 'trait-override'));
    expect(overrides.length).toBeGreaterThan(0);
    await tab(page, 'Match').click();
    await until(page, () => window.__touchline.view() === 'match', undefined, 5_000);
    await rewindTo(page, overrides[0].tick + 1);
    await settled(page);
    const rows = await feedRows(page);
    expect(rows.some((r) => r.kind === 'trait-override' && /strong trait, (under pressure|in a low mood)\.$/.test(r.text))).toBe(true);
    await page.screenshot({ path: evidence(info, 'loop-4-feed.png') });

    // 5. Full time, no relaunch: the count and the season on his panel.
    await toFullTime(page);
    const holder = await firstHolder(page);
    expect(holder, 'a starter held up the ball against the faster tempo').not.toBeNull();
    await openPanel(page, holder.index);
    await expect(panel(page).locator('[data-override]')).toHaveAttribute('data-override', String(holder.count));
    await expect(panel(page).locator('[data-season]')).toHaveAttribute('data-season', 'some', { timeout: 15_000 });
    await expect(panel(page).locator('[data-season-row="so-far"]')).toContainText('1 app');
    await expect(panel(page).locator('[data-season-row="goal"]')).toContainText(/, (ahead|on track|behind)$/);
    await settled(page);
    await page.screenshot({ path: evidence(info, 'loop-5-after-the-match.png') });

    // 6. The next match on the same launch: the kick-off mood words are its hello's words.
    await leavePanel(page);
    await tab(page, 'Report').click().catch(() => {});
    await until(page, () => window.__touchline.report().next.ready, undefined, 20_000);
    await page.getByRole('group', { name: 'Next' }).locator('[data-choice="new"]').click();
    await until(page, () => window.__touchline.frontDoor().view === 'setup', undefined, 20_000);
    await setupToTactics(page);
    await kickOffFromTactics(page);
    const next = await touchlineAtKickOff(page);
    expect(next.rows.length).toBe(11);
    // At 0-0 the score draws every mood toward steady from the first tick, so a player whose
    // kick-off mood sits on the edge of a word may read the next word toward steady one second
    // in; no mood moves further, or away from steady.
    const RANK = { rattled: -2, low: -1, steady: 0, lifted: 1, confident: 2 };
    for (const row of next.rows) {
      expect(row.text, row.name).toMatch(MOOD_CELL);
      const live = RANK[row.mood];
      const hello = RANK[next.hello[row.id]];
      expect(live === hello || live === hello - Math.sign(hello), `${row.name}: ${next.hello[row.id]} then ${row.mood}`).toBe(true);
    }
    expect(next.rows.filter((row) => row.mood === next.hello[row.id]).length).toBeGreaterThanOrEqual(9);
    fs.writeFileSync(evidence(info, 'loop-6-kick-off-mood.json'), `${JSON.stringify({ first, next }, null, 2)}\n`);
    await page.screenshot({ path: evidence(info, 'loop-6-next-kick-off.png') });
  } finally {
    engine.cleanUp();
    removeDir(dir);
  }
});
