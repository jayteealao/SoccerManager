// The pitch: smooth playback at one times speed, the requested speed, the lag notice when
// the stream cannot keep up, and a rewind that draws the stored tick.
import path from 'node:path';
import { expect, test } from '@playwright/test';
import { runEngine, startEngine, tempDir } from '../support/engine.mjs';
import { openMatch, renderedTick, scrubTo, setSpeed, until } from '../support/page.mjs';

let fixture;

test.beforeAll(() => {
  const dir = tempDir('fixture');
  fixture = path.join(dir, 'match.smfx');
  // Fourteen minutes: half-time, which pauses playback for its report, comes after the
  // five minutes the frame-rate test plays.
  const run = runEngine(['record', '--seed', '7', '--minutes', '14', '--out', fixture], { dataDir: dir });
  expect(run.code, run.stderr).toBe(0);
});

async function replay(page, args) {
  const engine = await startEngine({ command: 'replay', args: ['--fixture', fixture, '--web', 'web', ...args] });
  await openMatch(page, engine.url);
  await until(page, () => window.__touchline.lastRenderedTick() > 0, { timeout: 30_000 });
  return engine;
}

test('at 1x the page holds 60 frames per second for five match minutes and skips no tick', async ({ page }) => {
  test.setTimeout(8 * 60_000);
  // Every signal the page writes, from its console: the in-page ring keeps only the newest.
  const signals = [];
  page.on('console', (message) => {
    try {
      const row = JSON.parse(message.text());
      if (row.signal) {
        signals.push(row);
      }
    } catch {
      // Not a signal row.
    }
  });
  const engine = await replay(page, ['--speed', '1']);
  try {
    const start = await renderedTick(page);
    // Five minutes of match time at one times speed is five minutes of wall time.
    await until(page, (end) => window.__touchline.lastRenderedTick() >= end, {
      arg: start + 5 * 60 * 50,
      timeout: 6 * 60_000,
    });
    const frame = await page.evaluate(() => window.__touchline.frame());
    const budgets = signals.filter((s) => s.signal === 'viewer.frame_budget');
    const skipped = signals.filter((s) => s.signal === 'viewer.tick_skipped');
    expect(budgets.length).toBeGreaterThanOrEqual(55);
    expect(skipped).toEqual([]);
    expect(frame.refresh_hz).toBeGreaterThanOrEqual(59);
    expect(frame.fps_median).toBeGreaterThanOrEqual(59);
    for (const budget of budgets.slice(1)) {
      expect(budget.fps_median).toBeGreaterThanOrEqual(59);
    }
  } finally {
    engine.cleanUp();
  }
});

test('at 4x the clock runs four times faster than the wall clock', async ({ page }) => {
  const engine = await replay(page, ['--speed', '8']);
  try {
    await setSpeed(page, 4);
    await page.waitForTimeout(1000);
    const t0 = await renderedTick(page);
    const w0 = Date.now();
    await page.waitForTimeout(10_000);
    const t1 = await renderedTick(page);
    const rate = (t1 - t0) / ((Date.now() - w0) / 1000) / 50;
    expect(Math.abs(rate - 4) / 4).toBeLessThanOrEqual(0.02);
    await expect(page.getByLabel('Playback speed')).toHaveText('4x');
  } finally {
    engine.cleanUp();
  }
});

for (const [sustain, lag] of [[3, true], [8, false]]) {
  test(`a stream sustained at ${sustain}x ${lag ? 'shows' : 'shows no'} lag notice at 8x`, async ({ page }) => {
    const engine = await replay(page, ['--speed', '8', '--sustain', String(sustain)]);
    try {
      await setSpeed(page, 8);
      await page.waitForTimeout(8000);
      const notice = page.locator('#notice');
      if (lag) {
        await expect(notice).toHaveAttribute('data-shown', 'true');
        await expect(notice).toContainText('Lag');
        await expect(notice).toContainText('3x');
        await expect(page.locator('#speed-effective')).toHaveText('3x');
      } else {
        await expect(notice).toHaveAttribute('data-shown', 'false');
        await expect(page.locator('#speed-effective')).toHaveText('8x');
      }
    } finally {
      engine.cleanUp();
    }
  });
}

test('a rewind draws the stored positions of the tick', async ({ page }) => {
  const engine = await replay(page, ['--speed', '8']);
  try {
    await setSpeed(page, 8);
    await until(page, () => window.__touchline.lastRenderedTick() > 3000, { timeout: 60_000 });
    // Paused, so the clock stays on the tick the rewind drew.
    await page.getByRole('button', { name: 'Pause' }).click();
    for (const tick of [1234, 2500, 51]) {
      await scrubTo(page, tick);
      const rewind = await until(page, (t) => {
        const r = window.__touchline.lastRewind();
        return r && r.tick === t ? r : null;
      }, { arg: tick });
      expect(rewind.exact).toBe(true);
      const stored = await page.evaluate((t) => window.__touchline.tickAt(t), tick);
      expect(rewind.drawn).toEqual(stored);
      await expect(page.getByLabel('Match clock')).toHaveText(
        `${String(Math.floor(tick / 50 / 60)).padStart(2, '0')}:${String(Math.floor(tick / 50) % 60).padStart(2, '0')}`
      );
    }
  } finally {
    engine.cleanUp();
  }
});

test('every drawn frame lies between two stored ticks, never past the newest', async ({ page }) => {
  const engine = await replay(page, ['--speed', '8']);
  try {
    for (const speed of [1, 8]) {
      await setSpeed(page, speed);
      await page.waitForTimeout(1000);
      for (let i = 0; i < 30; i += 1) {
        const read = await page.evaluate(() => {
          const t = window.__touchline.lastRenderedTick();
          return {
            t,
            newest: window.__touchline.history().newest_tick,
            drawn: window.__touchline.lastRendered(),
            from: window.__touchline.tickAt(t),
            after: window.__touchline.tickAt(t + 1),
          };
        });
        expect(read.t).toBeLessThanOrEqual(read.newest);
        expect(read.from).not.toBeNull();
        if (speed === 1 && read.after) {
          // At 1x each frame is on the segment between tick t and tick t + 1.
          read.drawn.forEach((v, k) => {
            expect(v).toBeGreaterThanOrEqual(Math.min(read.from[k], read.after[k]) - 1);
            expect(v).toBeLessThanOrEqual(Math.max(read.from[k], read.after[k]) + 1);
          });
        }
        await page.waitForTimeout(100);
      }
    }
  } finally {
    engine.cleanUp();
  }
});
