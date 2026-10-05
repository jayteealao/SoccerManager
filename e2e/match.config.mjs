// The viewer's screenshots, contrast, keyboard walks, drives and frame time. Separate from
// the engine suite (playwright.config.mjs), which drives the same viewer through whole
// matches: every test here starts the release engine with `--web ../viewer/dist` (see
// support/engine.mjs), so the viewer must be built first.
//
// Run: `npm --prefix ../viewer run build`, then `npx playwright test -c match.config.mjs`.
// The baselines are Windows baselines from the reference desktop; font rendering differs on
// other systems, so this suite does not run in CI.
import { defineConfig } from '@playwright/test';

import { SIZES, suiteProjects, workerCount } from './support/workers.mjs';

const channel = process.env.PW_CHANNEL || undefined;
const workers = workerCount();

// The window sizes. `chromium` runs every test at 1280 by 800 and keeps today's baseline names.
// The four size projects run only the tests tagged @sizes (each state's screenshot and the
// Tab walks): `chromium-1920` compares a second set of baselines at 1920 by 1080, and the
// three layout projects run the layout check (support/layout.mjs) at 768 by 1024, 1024 by 640
// and 2560 by 1440, with no baselines. SM_E2E_SIZES=0 leaves them out and keeps the worker
// count, which `--project=chromium` would drop to one.
const sizes = process.env.SM_E2E_SIZES === '0' ? [] : SIZES;

export default defineConfig({
  testDir: './match',
  workers,
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  timeout: 5 * 60_000,
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'playwright-report/match' }]],
  outputDir: 'test-results/match',
  use: {
    headless: true,
    viewport: { width: 1280, height: 800 },
    channel,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  expect: {
    toHaveScreenshot: { maxDiffPixelRatio: 0.01, threshold: 0.2, animations: 'disabled' },
  },
  // With more than one worker, the timing tests run alone after the others (see
  // support/workers.mjs).
  projects: suiteProjects([{ name: 'chromium', metadata: { mode: 'pixels' } }, ...sizes], workers),
});
