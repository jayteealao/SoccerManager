// The Full Game shell at 1280 by 800 in each skin: its measured boxes equal the visual
// contract's sizes, its screenshot matches the approved baseline, and a Tab walk never lands
// inside a stub.
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

    const app = await box(page, '.app');
    expect([app.width, app.height]).toEqual([1280, 800]);
    const rail = await box(page, 'nav.rail');
    expect(rail.width).toBe(40);
    const header = await box(page, 'header.hd');
    expect(header.height).toBe(52);
    const band = await box(page, '.band');
    expect(band.height).toBe(52);
    const date = await box(page, '.date');
    expect([date.width, date.height]).toEqual([152, 44]);
    const action = await box(page, 'button.cont');
    expect([action.width, action.height]).toEqual([180, 44]);
    const tabs = await box(page, 'nav.subnav');
    expect(tabs.height).toBe(30);
    expect(tabs.y - (header.y + header.height)).toBe(5);
    const strip = await box(page, '.strip');
    expect(strip.height).toBe(54);

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
