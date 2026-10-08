// The player panel on the built viewer, served by the release program's `launch`.
//
// Drive: a fast-forwarded match of seed 7 to its full-time report, then the report's Squad tab
// and a player's name. The panel shows his attributes by group as whole numbers 1 to 20, the
// hidden values in words (the shipped teams are version 1 files, so every player reads "Not
// yet known"), "Plays between" in whole numbers, and this match's rating in one decimal.
// Back returns to the squad. On the next launch on the same data folder, the stored rating is
// read back. Also: a Tab walk never lands in a stub, and the rendered contrast check.
// Screenshots are kept as evidence (no baseline yet).
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import { frontDoor, tempDir } from '../support/engine.mjs';
import { kickOffFromStart, open, settled, toFullTime } from '../support/front.mjs';

/// Past the end of any match: a match with this fast-forward plays straight through.
const PAST_THE_END = 1_000_000;

const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR ? path.join(process.env.MATCH_EVIDENCE_DIR, name) : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

const tab = (page, name) => page.locator('nav.subnav:visible').getByRole('button', { name, exact: true });
const panel = (page) => page.locator('[data-screen="player"]:visible');

async function toReport(page, url) {
  await open(page, url);
  await until(page, () => window.__touchline.frontDoor().answered);
  await page.keyboard.press('Enter');
  await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
  await kickOffFromStart(page);
  await toFullTime(page);
}

/// The first starter's panel from the Squad screen.
async function openPanel(page) {
  await tab(page, 'Squad').click();
  await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);
  const row = page.locator('[data-screen="squad"]:visible tbody tr[data-row="0"]');
  await row.locator('button.who').click();
  await until(page, () => window.__touchline.view() === 'player', undefined, 10_000);
  await expect(panel(page)).toBeVisible();
  await settled(page);
}

test('after full time the panel shows whole numbers, words and the match rating', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  const dataDir = tempDir('player-panel');
  let engine = await frontDoor({ args: ['--seed', '7'], dataDir, fastForwardTo: PAST_THE_END });
  try {
    await toReport(page, engine.url);
    await openPanel(page);
    const p = panel(page);
    const values = await p.locator('[data-attr] dd').allTextContents();
    expect(values.length).toBeGreaterThan(20);
    for (const v of values) {
      expect(v.trim()).toMatch(/^\d+$/);
      expect(Number(v)).toBeGreaterThanOrEqual(1);
      expect(Number(v)).toBeLessThanOrEqual(20);
    }
    for (const name of ['consistency', 'injury_proneness']) {
      await expect(p.locator(`[data-hidden="${name}"]`)).toContainText('Not yet known — no match for us yet');
    }
    await expect(p.locator('[data-plays]')).toHaveText(/^\s*Plays (between \d+ and \d+|at \d+)\s*$/);
    await expect(p.locator('[data-ratings]')).toHaveAttribute('data-ratings', 'some');
    const chips = await p.locator('[data-ratings] .rt').allTextContents();
    expect(chips.length).toBe(1);
    expect(chips[0]).toMatch(/^\d{1,2}\.\d$/);
    // No hidden value as a number anywhere on the panel.
    const text = await p.evaluate((el) => el.innerText);
    expect(text).not.toMatch(/consistency\s*\d|injury[ _]proneness\s*\d/i);
    await page.screenshot({ path: evidence(info, 'panel-after-full-time.png') });

    await tab(page, 'Squad').click();
    await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);

    // The next launch on the same data folder reads the stored rating back.
    engine.kill();
    await engine.exited;
    expect(fs.existsSync(path.join(dataDir, 'views.json')), 'the launcher stored the ratings').toBe(true);
    const stored = JSON.parse(fs.readFileSync(path.join(dataDir, 'views.json'), 'utf8'));
    const ratings = Object.values(stored.clubs).flatMap((c) => Object.values(c.ratings ?? {}));
    expect(ratings.length).toBeGreaterThan(10);
    engine = await frontDoor({ args: ['--seed', '7'], dataDir });
    await open(page, engine.url);
    await until(page, () => window.__touchline.frontDoor().answered);
    await page.keyboard.press('Enter');
    await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
    await page.locator('[data-choice="new"]').click();
    await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
    await page.locator('[data-kickoff]').click();
    await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
    await openPanel(page);
    await expect(panel(page).locator('[data-ratings]')).toHaveAttribute('data-ratings', 'some');
  } finally {
    engine.cleanUp();
  }
});

test('the panel: a Tab walk never lands in a stub, and the rendered contrast passes', { tag: '@sizes' }, async ({ page }, info) => {
  test.setTimeout(4 * 60_000);
  const engine = await frontDoor();
  try {
    await open(page, engine.url);
    await until(page, () => window.__touchline.frontDoor().answered);
    await page.keyboard.press('Enter');
    await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
    await page.locator('[data-choice="new"]').click();
    await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
    await page.locator('[data-kickoff]').click();
    await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
    await openPanel(page);
    await expect(panel(page).locator('[data-ratings]')).toHaveAttribute('data-ratings', 'none');
    await page.screenshot({ path: evidence(info, 'panel-new-signing.png') });
    const walk = [];
    for (let n = 0; n < 40; n += 1) {
      await page.keyboard.press('Tab');
      walk.push(await page.evaluate(() => Boolean(document.activeElement?.closest('[data-stub]'))));
    }
    expect(walk.filter(Boolean)).toEqual([]);
    const report = await contrastReport(page);
    fs.writeFileSync(evidence(info, 'panel-contrast-report.json'), `${JSON.stringify(report, null, 2)}\n`);
    expect(report.checked).toBeGreaterThan(30);
    expect(report.failures, JSON.stringify(report.failures, null, 2)).toEqual([]);
  } finally {
    engine.cleanUp();
  }
});
