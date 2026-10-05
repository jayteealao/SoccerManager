// The browser suite. Every test starts its own engine, with its own port and data folder,
// serving the built viewer (see support/engine.mjs), so tests run in several workers; the
// timing tests run alone (see support/workers.mjs). Build the viewer first
// (`npm --prefix ../viewer run build`).
import { defineConfig } from '@playwright/test';

import { suiteProjects, workerCount } from './support/workers.mjs';

// Unset: the Chromium build Playwright ships. `msedge`: the installed Microsoft Edge.
const channel = process.env.PW_CHANNEL || undefined;
const workers = workerCount();

export default defineConfig({
  testDir: './tests',
  workers,
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  // The whole first match sets its own, longer timeout.
  timeout: 5 * 60_000,
  reporter: [['list'], ['html', { open: 'never' }]],
  use: {
    headless: true,
    viewport: { width: 1280, height: 800 },
    channel,
    acceptDownloads: true,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  // `scenario` is the whole first match; `viewer` is every other test; `timing` holds the
  // timing tests of both and runs them alone after the others (see support/workers.mjs).
  projects: suiteProjects(
    [
      { name: 'scenario', testMatch: /first-match\.spec\.mjs/ },
      { name: 'viewer', testIgnore: /first-match\.spec\.mjs/ },
    ],
    workers
  ),
});
