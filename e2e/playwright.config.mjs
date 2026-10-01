// The browser suite. Every test starts its own engine serving the built viewer (see
// support/engine.mjs), so the tests run one at a time: one engine serves one viewer per
// match. Build the viewer first (`npm --prefix ../viewer run build`).
import { defineConfig } from '@playwright/test';

// Unset: the Chromium build Playwright ships. `msedge`: the installed Microsoft Edge.
const channel = process.env.PW_CHANNEL || undefined;

export default defineConfig({
  testDir: './tests',
  workers: 1,
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  reporter: [['list'], ['html', { open: 'never' }]],
  use: {
    headless: true,
    viewport: { width: 1280, height: 800 },
    channel,
    acceptDownloads: true,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      // The whole match, lineup to full-time report. A 90-minute match at eight times
      // speed still takes more than eleven minutes of wall time.
      name: 'scenario',
      testMatch: /first-match\.spec\.mjs/,
      timeout: 45 * 60_000,
    },
    {
      name: 'viewer',
      testIgnore: /first-match\.spec\.mjs/,
      timeout: 5 * 60_000,
    },
  ],
});
