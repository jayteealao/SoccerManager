// The Full Game shell in each skin, at 1280 by 800 and at 1920 by 1080: it fills the window,
// its measured boxes equal the visual contract's sizes times the step's scale, its screenshot
// matches the approved baseline, and a Tab walk never lands inside a stub.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

const SKINS = ['broadcast-blue', 'interim-light'];

async function open(page, skin) {
  await page.goto(`/shell-test.html?skin=${skin}`);
  await expect(page.locator('html')).toHaveAttribute('data-ready', skin);
  await page.evaluate(() => document.fonts.ready);
}

async function box(page, selector) {
  const found = await page.locator(selector).first().boundingBox();
  expect(found, selector).not.toBeNull();
  return found;
}

for (const skin of SKINS) {
  test(`the shell in ${skin} keeps the contract's sizes and matches its baseline`, async ({
    page,
  }) => {
    await open(page, skin);
    expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe(skin);

    // The shell fills the window; a box measures in window pixels, so each drawn size is the
    // contract's size times the step's scale (1 at 1280, 1.125 at 1920).
    const viewport = page.viewportSize();
    const scale = Number(
      await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--step-scale'))
    );
    expect(scale).toBe(viewport.width >= 1920 ? 1.125 : 1);
    const px = (n) => expect.closeTo(n * scale, 0);
    const app = await box(page, '.app');
    expect([app.width, app.height]).toEqual([viewport.width, viewport.height]);
    const rail = await box(page, 'nav.rail');
    expect(rail.width).toEqual(px(40));
    const header = await box(page, 'header.hd');
    expect(header.height).toEqual(px(52));
    const band = await box(page, '.band');
    expect(band.height).toEqual(px(52));
    const date = await box(page, '.date');
    expect([date.width, date.height]).toEqual([px(152), px(44)]);
    const action = await box(page, 'button.cont');
    expect([action.width, action.height]).toEqual([px(180), px(44)]);
    const tabs = await box(page, 'nav.subnav');
    expect(tabs.height).toEqual(px(30));
    expect(tabs.y - (header.y + header.height)).toEqual(px(5));
    const strip = await box(page, '.strip');
    expect(strip.height).toEqual(px(54));

    // A computed font size is the size before the zoom, so it reads the same at every step.
    const label = await page.locator('button.cont').evaluate((el) => {
      const s = getComputedStyle(el);
      return [s.fontSize, s.fontWeight, s.textTransform];
    });
    expect(label).toEqual(['12.5px', '800', 'uppercase']);

    await expect(page).toHaveScreenshot(`shell-${skin}.png`);
  });
}

test('a Tab walk through the shell never lands inside a stub', async ({ page }, info) => {
  await open(page, 'broadcast-blue');
  const walk = [];
  for (let press = 1; press <= 40; press += 1) {
    await page.keyboard.press('Tab');
    walk.push(
      await page.evaluate((n) => {
        const el = document.activeElement;
        return {
          press: n,
          tag: el?.tagName.toLowerCase() ?? null,
          name: el?.getAttribute('aria-label') ?? el?.textContent?.trim() ?? '',
          stub: el?.closest('[data-stub]')?.dataset.stub ?? null,
          inert: Boolean(el?.closest('[inert]')),
        };
      }, press)
    );
  }
  const out = process.env.SHELL_EVIDENCE_DIR
    ? path.join(process.env.SHELL_EVIDENCE_DIR, 'shell-tab-walk.json')
    : info.outputPath('shell-tab-walk.json');
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, `${JSON.stringify(walk, null, 2)}\n`);

  for (const step of walk) {
    expect(step.stub, `press ${step.press}`).toBeNull();
    expect(step.inert, `press ${step.press}`).toBe(false);
  }
  const names = walk.map((s) => `${s.tag}:${s.name}`);
  expect(names).toContain('button:Resume');
  expect(names).toContain('button:Match');
});
