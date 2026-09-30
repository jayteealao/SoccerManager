// The match screen's screenshots, contrast, keyboard walk and frame time, on the new viewer.
// Separate from the engine suite, which drives the page in web/: every test here starts the
// release engine with `--web ../viewer/dist` (see support/engine.mjs), so the viewer must be
// built first.
//
// Run: `npm --prefix ../viewer run build`, then `npx playwright test -c match.config.mjs`.
// The baselines are Windows baselines from the reference desktop; font rendering differs on
// other systems, so this suite does not run in CI.
import { defineConfig } from '@playwright/test';

const channel = process.env.PW_CHANNEL || undefined;

export default defineConfig({
  testDir: './match',
  workers: 1,
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
  projects: [{ name: 'chromium' }],
});
