// The page fills every window, and scales down below the smallest one. One drive through the
// front door, served by the release launcher with `--web viewer/dist` and fast-forwarded, so it
// never waits for playback: the start screen, Tactics before kick-off, the match paused at
// minute 30, and the full-time report. At each state the window takes every size below in
// turn.
//
// - From 768 by 600 to 2560 by 1440: the page's root and its top box equal the window, and the
//   layout check (support/layout.mjs) finds no sideways scroll, no clipped text and no control
//   cut off.
// - Below 768 by 600 (a phone at 375 by 667, a narrow 600 by 800, a short 1280 by 560): the
//   page's zoom equals min(width / 768, height / 600) within 0.001, the page still fills the
//   window with no sideways scroll, and the action block, a tab and Menu (on the match views)
//   each lie in the window and take a click.
//
// It sets its own sizes, so it runs once, in the chromium project.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { frontDoor } from '../support/engine.mjs';
import { open, pastSplash, rewindTo, toFullTime } from '../support/front.mjs';
import { layoutReport } from '../support/layout.mjs';
import { minControl } from '../support/snap.mjs';

const FILL = [
  [768, 600],
  [800, 1280],
  [1023, 700],
  [1024, 640],
  [1366, 768],
  [1440, 900],
  [1600, 900],
  [1919, 1080],
  [2048, 1152],
  [2560, 1440],
];
const FLOOR = [
  [375, 667],
  [600, 800],
  [1280, 560],
];
/// The rules this sweep holds at every size; overlap and hit areas are the size projects'.
const RULES = ['fill', 'sideways scroll', 'clipped text', 'cut off'];
const MINUTE_30 = 90_000;

const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });

/// Two animation frames, so a resize has laid out and the pitch has redrawn.
const frames = (page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));

/// Every size for the state on show; returns the rows found, keyed by size.
async function sweep(page, state) {
  const found = {};
  for (const [width, height] of FILL) {
    await page.setViewportSize({ width, height });
    await frames(page);
    const report = await layoutReport(page, { minControl: minControl({ width, height }) });
    const rows = report.rows.filter((r) => RULES.includes(r.rule));
    found[`${width}x${height}`] = rows;
    expect.soft(rows, `${state} at ${width} by ${height}`).toEqual([]);
  }
  for (const [width, height] of FLOOR) {
    await page.setViewportSize({ width, height });
    await frames(page);
    const fit = await page.evaluate(() => {
      const top = document.querySelector('#app > *');
      const r = top.getBoundingClientRect();
      return {
        zoom: Number(getComputedStyle(top).zoom),
        width: r.width,
        height: r.height,
        scrollWidth: document.scrollingElement.scrollWidth,
      };
    });
    const expected = Math.min(1, width / 768, height / 600);
    expect.soft(Math.abs(fit.zoom - expected), `${state} zoom at ${width} by ${height}`).toBeLessThan(0.001);
    expect.soft([Math.round(fit.width), Math.round(fit.height)], `${state} fills ${width} by ${height}`).toEqual([width, height]);
    expect.soft(fit.scrollWidth, `${state} has no sideways scroll at ${width} by ${height}`).toBeLessThanOrEqual(width);
    const controls = [page.locator('header button.cont:visible'), page.locator('nav.subnav:visible button').first()];
    if (await page.locator('[data-menu-button]:visible').count()) {
      controls.push(page.locator('[data-menu-button]:visible'));
    }
    const reached = [];
    for (const control of controls) {
      const box = await control.boundingBox();
      const inside = box !== null && box.x >= 0 && box.y >= 0 && box.x + box.width <= width && box.y + box.height <= height;
      expect.soft(inside, `${state}: ${await control.evaluate((el) => el.textContent.trim())} lies in ${width} by ${height}`).toBe(true);
      // A trial click runs every check a click runs, the hit test included, and clicks nothing.
      await control.click({ trial: true, timeout: 5_000 });
      reached.push(await control.evaluate((el) => el.textContent.trim()));
    }
    found[`${width}x${height}`] = { ...fit, expected, reached };
  }
  await page.setViewportSize({ width: 1280, height: 800 });
  await frames(page);
  return found;
}

test('every screen fills the window from 768 by 600 to 2560 by 1440, and scales down below it', async ({ page }, info) => {
  test.setTimeout(10 * 60_000);
  const engine = await frontDoor({ fastForwardTo: 400_000 });
  const out = {};
  try {
    await open(page, engine.url);
    await pastSplash(page);
    out.start = await sweep(page, 'the start screen');

    await page.locator('[data-choice="new"]').click();
    await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
    await page.locator('[data-kickoff]').click();
    await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
    out.tactics = await sweep(page, 'Tactics before kick-off');

    const action = page.locator('header button.cont:visible');
    await action.click();
    await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
    await action.click();
    await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
    await until(page, (t) => window.__touchline.history().newest_tick >= t, MINUTE_30);
    const playback = page.getByRole('group', { name: 'Playback' }).filter({ visible: true });
    await playback.getByRole('button', { name: 'Pause', exact: true }).click();
    await rewindTo(page, MINUTE_30);
    out.match = await sweep(page, 'the match at minute 30');

    await toFullTime(page);
    out.report = await sweep(page, 'the full-time report');
  } finally {
    const file = process.env.MATCH_EVIDENCE_DIR
      ? path.join(process.env.MATCH_EVIDENCE_DIR, 'window-fit.json')
      : info.outputPath('window-fit.json');
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, `${JSON.stringify(out, null, 2)}\n`);
    engine.cleanUp();
  }
});
