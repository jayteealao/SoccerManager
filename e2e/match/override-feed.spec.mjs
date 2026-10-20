// A trait override in the match feed and its count on the player panel after the match, on
// the built viewer, served by the release program's `launch`.
//
// Drive: a content copy whose home team is the version 3 fixture with holding up the ball as a
// strong trait of every outfield player (so whichever eleven start, one carries it), and whose
// tuning makes that trait beat a faster tempo every time it is pressed, so the line shows within
// a short run; the event, the feed row and the count are the engine's and the page's own. On
// the Tactics screen the tempo is set to fast, the match kicks off with the whole match sent
// at once, and the feed is read at the first override: one row of the trait-override kind
// naming the player, what he did, the instruction and the cause. Played to full time, the
// panel of the first player to hold up the ball shows the count of his rows with one example. Screenshots are kept as evidence; the test removes the content copy
// and data folder it made.
import { expect, test } from '@playwright/test';

import { frontDoor } from '../support/engine.mjs';
import { open, pastSplash, rewindTo, settled, toFullTime } from '../support/front.mjs';
import { feedRows } from '../support/page.mjs';
import {
  contentWithPersonHome,
  evidence,
  firstHolder,
  holdUpForAll,
  kickOffFromTactics,
  pauseIfPlaying,
  plantOverride,
  removeDir,
  setupToTactics,
  tab,
} from '../support/person.mjs';

/// Past the end of any match: the engine sends the whole match at once.
const PAST_THE_END = 1_000_000;

const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });
const panel = (page) => page.locator('[data-screen="player"]:visible');

test('an override shows in the feed with the player, the trait, the instruction and the cause, and the panel counts it', async ({
  page,
}, info) => {
  test.setTimeout(8 * 60_000);
  const { dir, team } = contentWithPersonHome(holdUpForAll);
  plantOverride(dir, 'holds_up_ball', 1);
  const engine = await frontDoor({ env: { SM_CONTENT_DIR: dir }, fastForwardTo: PAST_THE_END });
  try {
    await open(page, engine.url);
    await pastSplash(page);
    await page.locator('[data-choice="new"]').click();
    await setupToTactics(page);
    // The instruction his trait goes against: a tempo faster than the default.
    await page.locator('select[data-instruction="tempo"]:visible').selectOption({ label: 'Fast' });
    await kickOffFromTactics(page);

    // The first hold-up override, then the feed at that tick.
    await until(page, () => window.__touchline.events().some((e) => e['event.type'] === 'full-time'), undefined, 180_000);
    const overrides = await page.evaluate(() =>
      window.__touchline.events().filter((e) => e['event.type'] === 'trait-override' && e['trait.name'] === 'holds_up_ball')
    );
    expect(overrides.length, 'the planted chance gives overrides').toBeGreaterThan(0);
    const first = overrides[0];
    expect(first['trait.instruction']).toBe('tempo:faster');
    await pauseIfPlaying(page);
    await rewindTo(page, first.tick + 1);
    await settled(page);
    const rows = await feedRows(page);
    const line = rows.find((r) => r.kind === 'trait-override' && r.tick === first.tick && r.text.includes(' holds up the ball '));
    expect(line, JSON.stringify(rows.slice(-5))).toBeTruthy();
    const surnames = new Map(team.players.map((p) => [p.name.split(' ').at(-1), p]));
    const who = line.text.match(/Trait\s*(\S+) /i)?.[1];
    expect(surnames.has(who), line.text).toBe(true);
    expect(line.text).toMatch(/holds up the ball against 'Higher tempo' — strong trait, (under pressure|in a low mood)\.$/);
    const row = page.locator(`section[aria-label="Commentary"] li[data-kind="trait-override"][data-tick="${first.tick}"]`).first();
    await expect(row).toBeVisible();
    await expect(row.locator('.mk')).toHaveAttribute('aria-hidden', 'true');
    await page.screenshot({ path: evidence(info, 'feed-override.png') });

    // At full time the panel of the first home player to hold up the ball counts his
    // overrides of the match and quotes one.
    await toFullTime(page);
    const holder = await firstHolder(page);
    expect(holder, 'a player held up the ball against the faster tempo').not.toBeNull();
    // The Squad screen opens from the report; the match view's Squad tab is a menu.
    await pauseIfPlaying(page);
    if ((await page.evaluate(() => window.__touchline.view())) !== 'report') {
      await toFullTime(page);
      await pauseIfPlaying(page);
    }
    await tab(page, 'Squad').click();
    await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);
    await page.locator(`[data-screen="squad"]:visible tbody tr[data-row="${holder.index}"] button.who`).click();
    await until(page, () => window.__touchline.view() === 'player', undefined, 10_000);
    const count = panel(page).locator('[data-override]');
    await expect(count).toHaveAttribute('data-override', String(holder.count));
    await expect(count).toContainText(
      new RegExp(`^!?\\s*${holder.count} times? last match a trait beat an instruction — .+, (under pressure|in a low mood)$`)
    );
    await settled(page);
    await page.screenshot({ path: evidence(info, 'panel-override-count.png') });
  } finally {
    engine.cleanUp();
    removeDir(dir);
  }
});
