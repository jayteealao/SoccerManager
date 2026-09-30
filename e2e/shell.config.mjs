// The shell's screenshots and keyboard walk. Separate from the engine suite: these tests open
// the built viewer's test page through the Vite preview server and start no engine.
//
// Run: build the viewer (`npm --prefix ../viewer run build`), then
//      `npx playwright test -c shell.config.mjs`.
// The baselines are Windows baselines from the reference desktop; font rendering differs on
// other systems, so this suite does not run in CI.
import { defineConfig } from '@playwright/test';

const channel = process.env.PW_CHANNEL || undefined;
const PORT = 4180;

export default defineConfig({
  testDir: './shell',
  workers: 1,
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'playwright-report/shell' }]],
  outputDir: 'test-results/shell',
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    headless: true,
    viewport: { width: 1280, height: 800 },
    channel,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  expect: {
    toHaveScreenshot: { maxDiffPixelRatio: 0.01, threshold: 0.2, animations: 'disabled' },
  },
  webServer: {
    command: `npm --prefix ../viewer run preview -- --host 127.0.0.1 --port ${PORT} --strictPort`,
    url: `http://127.0.0.1:${PORT}/shell-test.html`,
    reuseExistingServer: false,
    timeout: 60_000,
  },
  projects: [{ name: 'chromium' }],
});
