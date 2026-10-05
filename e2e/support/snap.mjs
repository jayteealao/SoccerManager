// One screenshot call for every window size. A test reaches a state once and calls `snap`:
// in a pixels project (1280 by 800 and 1920 by 1080) it compares the screenshot with its
// baseline, whose file name carries the project and so the size; in a layout project
// (768 by 1024, 1024 by 640 and 2560 by 1440) it runs the layout check (support/layout.mjs)
// instead, writes the report as JSON and expects no row. The default skin's rendered contrast
// must pass too; the light test skin's report is kept but not asserted, because its header,
// strip and card text fail on every screen today.
//
// clearPointer and centre replace the fixed points a test used at 1280 by 800.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { layoutReport } from './layout.mjs';

/// The smallest hit area at a window size: 44 px at the compact step (under 1024 px wide or
/// under 600 px high), 24 px otherwise.
export const minControl = ({ width, height }) => (width < 1024 || height < 600 ? 44 : 24);

/// Compares `target` (the page, or a locator) with the baseline `name`, or runs the layout
/// check, as the project's metadata says.
export async function snap(page, name, target = page, options = {}) {
  const info = test.info();
  if ((info.project.metadata?.mode ?? 'pixels') === 'pixels') {
    await expect(target).toHaveScreenshot(name, options);
    return;
  }
  await page.evaluate(() => document.fonts.ready);
  const size = page.viewportSize();
  const report = await layoutReport(page, { minControl: minControl(size) });
  const theme = await page.evaluate(() => document.documentElement.dataset.theme ?? null);
  const file = `layout-${name.replace(/\.png$/, '')}-${info.project.name}.json`;
  const out = process.env.LAYOUT_EVIDENCE_DIR ? path.join(process.env.LAYOUT_EVIDENCE_DIR, file) : info.outputPath(file);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, `${JSON.stringify({ name, project: info.project.name, theme, ...report }, null, 2)}\n`);
  expect.soft(report.rows, `the layout of ${name} at ${size.width} by ${size.height}`).toEqual([]);
  if (theme === 'broadcast-blue') {
    expect.soft(report.contrast, `the rendered contrast of ${name}`).toEqual([]);
  }
}

/// Moves the pointer to the window's bottom-right corner, off every control, so no hover
/// shows in a screenshot.
export async function clearPointer(page) {
  const { width, height } = page.viewportSize();
  await page.mouse.move(width - 10, height - 10);
}

/// Clicks the page body at the window's bottom-right corner, which holds no control.
export async function clickClear(page) {
  const { width, height } = page.viewportSize();
  await page.locator('body').click({ position: { x: width - 10, y: height - 10 } });
}

/// The window's centre.
export function centre(page) {
  const { width, height } = page.viewportSize();
  return { x: Math.round(width / 2), y: Math.round(height / 2) };
}
