// The browser suite. Every test starts its own engine, with its own port and data folder,
// serving the built viewer (see support/engine.mjs), so tests run in several workers; the
// timing tests run alone (see support/workers.mjs). Build the viewer first
// (`npm --prefix ../viewer run build`).
import { defineConfig } from '@playwright/test';

import { TIMING, timingProject, workerCount } from './support/workers.mjs';

// Unset: the Chromium build Playwright ships. `msedge`: the installed Microsoft Edge.
const channel = process.env.PW_CHANNEL || undefined;
// The npm scripts that pick one project keep one worker and the projects below.
const workers = ['test:viewer', 'test:scenario'].includes(process.env.npm_lifecycle_event) ? 1 : workerCount();

// The whole match, lineup to full-time report. A 90-minute match at eight times speed still
// takes more than eleven minutes of wall time.
const SCENARIO_TIMEOUT = 45 * 60_000;

export default defineConfig({
  testDir: './tests',
  workers,
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
  // With more than one worker, the timing tests, the whole match among them, run alone after
  // the others.
  projects:
    workers > 1
      ? [
          {
            name: 'viewer',
            testIgnore: /first-match\.spec\.mjs/,
            grepInvert: TIMING,
            teardown: 'timing',
            timeout: 5 * 60_000,
          },
          timingProject({ timeout: SCENARIO_TIMEOUT }),
        ]
      : [
          { name: 'scenario', testMatch: /first-match\.spec\.mjs/, timeout: SCENARIO_TIMEOUT },
          { name: 'viewer', testIgnore: /first-match\.spec\.mjs/, timeout: 5 * 60_000 },
        ],
});
