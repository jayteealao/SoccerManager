// The Squad screen and its named views on the built viewer, served by the release program's
// `launch`, each test with its own data folder.
//
// Drive: from the start screen through match setup to Tactics, then the Squad tab. The table
// opens on the default view; the column menu adds Build, moves it up with the keyboard (focus
// stays on the moved row) and removes Stamina; Done saves the view. A header sorts (▼), and a
// second select reverses it (▲). Save view keeps a copy. The view reads back after a reload of
// the page, and after the launcher stops and starts again on the same data folder, from
// `views.json`.
//
// Also: no 1-20 cell, title or aria label shows a decimal or a value over 20; a Tab walk never
// lands in a stub; the rendered contrast check. Screenshots of the default, menu-open and
// sorted states are kept as evidence; the default view is compared with its baseline at each
// window size in the @sizes test.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import { frontDoor, tempDir } from '../support/engine.mjs';
import { open, settled } from '../support/front.mjs';
import { clearPointer, snap } from '../support/page.mjs';

const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR ? path.join(process.env.MATCH_EVIDENCE_DIR, name) : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

const tab = (page, name) => page.locator('nav.subnav:visible').getByRole('button', { name, exact: true });
const squad = (page) => page.locator('[data-screen="squad"]:visible');
const headers = (page) =>
  squad(page)
    .locator('[data-squad-table] thead th[data-col]')
    .evaluateAll((ths) => ths.map((th) => th.dataset.col));

/// From the start screen through match setup to the Tactics screen, before kick-off.
async function toTactics(page, url) {
  await open(page, url);
  await until(page, () => window.__touchline.frontDoor().answered);
  await page.keyboard.press('Enter');
  await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
  await page.locator('[data-choice="new"]').click();
  await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
  await page.locator('[data-kickoff]').click();
  await until(page, () => window.__touchline.screen() === 'kickoff', undefined, 30_000);
  await until(page, () => window.__touchline.view() === 'tactics', undefined, 10_000);
}

async function openSquad(page) {
  await tab(page, 'Squad').click();
  await until(page, () => window.__touchline.view() === 'squad', undefined, 10_000);
  await expect(squad(page).locator('[data-squad-table] tbody tr[data-row]').first()).toBeVisible();
  await settled(page);
}

/// Every attribute and level cell, and every title and aria label on the screen, holds whole
/// numbers 1 to 20 only where it shows a rating; no text anywhere in the table has a decimal
/// rating. Returns how many values it read.
async function scaleScan(page) {
  const found = await squad(page).evaluate((root) => {
    const values = [];
    const bad = [];
    for (const td of root.querySelectorAll('td[data-cell^="attr:"], td[data-cell="level"]')) {
      // The drawn number, without the screen reader's text beside it.
      const drawn = td.cloneNode(true);
      for (const sr of drawn.querySelectorAll('.sr')) sr.remove();
      const text = drawn.textContent.trim();
      if (text === '—') continue;
      values.push(text);
      if (!/^\d+$/.test(text) || Number(text) < 1 || Number(text) > 20) bad.push(`${td.dataset.cell}: ${text}`);
    }
    for (const el of root.querySelectorAll('[aria-label], [title], .sr')) {
      const label = `${el.getAttribute('aria-label') ?? ''} ${el.getAttribute('title') ?? ''} ${el.classList.contains('sr') ? el.textContent : ''}`;
      if (/\b\d+\.\d\b/.test(label) && !/rating|average|last match/i.test(label)) bad.push(`label: ${label}`);
    }
    return { values, bad };
  });
  expect(found.bad, found.bad.join('\n')).toEqual([]);
  return found.values.length;
}

/// The saved views of the home club in the data folder's `views.json`, or null.
function storedViews(dataDir) {
  const file = path.join(dataDir, 'views.json');
  if (!fs.existsSync(file)) return null;
  const stored = JSON.parse(fs.readFileSync(file, 'utf8'));
  return Object.values(stored.clubs ?? {}).find((c) => c.views?.length) ?? null;
}

test('a named view is built, sorted and saved, and reads back after a reload and a relaunch', async ({ page }, info) => {
  test.setTimeout(6 * 60_000);
  const dataDir = tempDir('squad-views');
  let engine = await frontDoor({ dataDir });
  try {
    await toTactics(page, engine.url);
    await openSquad(page);
    const first = await headers(page);
    expect(first.slice(0, 15)).toEqual([
      'no', 'player', 'age', 'nat', 'height', 'cond', 'sharp', 'attr:pace', 'attr:acceleration',
      'attr:passing', 'attr:technique', 'attr:decisions', 'attr:stamina', 'consistency', 'rating',
    ]);
    const read = await scaleScan(page);
    console.log(`squad table: ${read} values on 1 to 20`);
    await page.screenshot({ path: evidence(info, 'squad-default.png') });

    // The column menu: add Build, move it up by keyboard, remove Stamina.
    const chip = squad(page).locator('[data-columns-chip]');
    await chip.click();
    const menu = squad(page).locator('[data-column-menu]');
    await expect(menu).toBeVisible();
    await menu.locator('button[data-add="build"]').click();
    await expect(menu.locator('li[data-shown="build"]')).toBeVisible();
    await menu.locator('li[data-shown="build"] [data-control="up"]').focus();
    await page.keyboard.press('Enter');
    await expect
      .poll(() => menu.locator('li[data-shown]').evaluateAll((lis) => lis.map((li) => li.dataset.shown).slice(-2)))
      .toEqual(['build', 'rating']);
    expect(await page.evaluate(() => document.activeElement?.closest('li')?.dataset.shown)).toBe('build');
    await menu.locator('li[data-shown="attr:stamina"] [data-control="remove"]').click();
    await settled(page);
    await page.screenshot({ path: evidence(info, 'squad-menu-open.png') });
    await menu.locator('[data-done]').click();
    await expect(menu).toHaveCount(0);
    await expect(chip).toBeFocused();
    let shown = await headers(page);
    expect(shown).toContain('build');
    expect(shown).not.toContain('attr:stamina');
    expect(shown.indexOf('build')).toBe(shown.indexOf('rating') - 1);

    // A header sorts, and a second select reverses.
    const table = squad(page).locator('[data-squad-table]');
    await table.locator('th[data-col="height"] button').click();
    await expect(table).toHaveAttribute('data-sort', 'height:down');
    await table.locator('th[data-col="height"] button').click();
    await expect(table).toHaveAttribute('data-sort', 'height:up');
    await page.screenshot({ path: evidence(info, 'squad-sorted.png') });

    // Save view keeps a copy; the store holds both.
    await squad(page).locator('[data-save-view]').click();
    await expect.poll(() => storedViews(dataDir)?.views.length ?? 0, { timeout: 10_000 }).toBeGreaterThanOrEqual(2);
    const saved = storedViews(dataDir);
    const active = saved.views.find((v) => v.name === saved.active);
    expect(active.columns).toContain('build');
    expect(active.columns).not.toContain('attr:stamina');
    expect(active.sort).toEqual({ column: 'height', direction: 'up' });

    // A reload of the page reads the same view back. The page holds the one match socket, so
    // the drive first returns to the start screen, then loads the page again and sets up a new
    // match.
    // Escape on the Squad screen opens the menu, as on every match view.
    await page.keyboard.press('Escape');
    await page.locator('[data-item="return"]').click();
    const leave = page.getByRole('button', { name: /^(Save and leave|Leave)/ });
    if (await leave.isVisible().catch(() => false)) {
      await leave.first().click();
    }
    await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 15_000);
    await toTactics(page, engine.url);
    await openSquad(page);
    await expect(squad(page).locator('select[data-view]')).toHaveValue(saved.active);
    shown = await headers(page);
    expect(shown).toContain('build');
    expect(shown).not.toContain('attr:stamina');
    await expect(squad(page).locator('[data-squad-table]')).toHaveAttribute('data-sort', 'height:up');

    // The launcher stops and starts again on the same data folder.
    engine.kill();
    await engine.exited;
    engine = await frontDoor({ dataDir });
    await toTactics(page, engine.url);
    await openSquad(page);
    await expect(squad(page).locator('select[data-view]')).toHaveValue(saved.active);
    shown = await headers(page);
    expect(shown).toContain('build');
    expect(shown).not.toContain('attr:stamina');
  } finally {
    engine.cleanUp();
  }
});

test('the Squad screen: a Tab walk never lands in a stub, and the rendered contrast passes', { tag: '@sizes' }, async ({ page }, info) => {
  test.setTimeout(4 * 60_000);
  // At the compact step the default view's columns, each with a 44 px header, are wider than
  // a 768 px window: the table scrolls sideways in its own box, with the Player column fixed.
  const engine = await frontDoor();
  try {
    await toTactics(page, engine.url);
    await openSquad(page);
    // The default view (board 1), against its baseline at each window size.
    await clearPointer(page);
    await snap(page, 'squad-default.png');
    const walk = [];
    for (let n = 0; n < 60; n += 1) {
      await page.keyboard.press('Tab');
      walk.push(
        await page.evaluate(() => {
          const el = document.activeElement;
          return { tag: el?.tagName.toLowerCase() ?? null, stub: Boolean(el?.closest('[data-stub]')), name: el?.getAttribute('aria-label') ?? el?.textContent?.trim().slice(0, 30) };
        })
      );
    }
    fs.writeFileSync(evidence(info, 'squad-tab-walk.json'), `${JSON.stringify(walk, null, 2)}\n`);
    expect(walk.filter((w) => w.stub)).toEqual([]);
    expect(walk.some((w) => w.name === 'Columns: Default' || /^Columns/.test(w.name ?? ''))).toBe(true);
    const report = await contrastReport(page);
    fs.writeFileSync(evidence(info, 'squad-contrast-report.json'), `${JSON.stringify(report, null, 2)}\n`);
    expect(report.checked).toBeGreaterThan(50);
    expect(report.failures, JSON.stringify(report.failures, null, 2)).toEqual([]);
  } finally {
    engine.cleanUp();
  }
});
