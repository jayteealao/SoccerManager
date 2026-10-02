// The match screen's frame time: a real engine, kicked off from the page, played for 60 s at
// 1x. The page writes a `viewer.frame_budget` row every five seconds and a
// `viewer.tick_skipped` row for any second in which it skipped a tick. It passes when every
// budget row after the first holds 59 frames a second or more, no tick is skipped, and the
// 95th percentile frame is no longer than one and a half refresh periods, the line past which
// the scheduler counts a frame as dropped. The background matchday plays all the while (the
// default on a served match): four other fixtures arrive, the list draws them, and ground
// messages keep arriving during the window, with the same budget.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { VIEWER, startEngine } from '../support/engine.mjs';
import { kickOffFromPage } from '../support/page.mjs';

const PLAY_MS = 60_000;

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR
    ? path.join(process.env.MATCH_EVIDENCE_DIR, name)
    : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

/// Serves `web`, kicks off through CONTINUE and the Pre-match KICK OFF, plays 60 s at 1x, and returns
/// every signal row the page wrote to its console and its final frame budget.
async function measure(page, web) {
  const rows = [];
  const grounds = { during: 0, window: false };
  page.on('websocket', (socket) => {
    socket.on('framereceived', ({ payload }) => {
      if (typeof payload === 'string' && grounds.window && /"type":"ground-(event|progress)"/.test(payload)) {
        grounds.during += 1;
      }
    });
  });
  page.on('console', (message) => {
    try {
      const row = JSON.parse(message.text());
      if (row.signal) {
        rows.push(row);
      }
    } catch {
      // Not a signal row.
    }
  });
  const engine = await startEngine({ args: ['--seed', '7', '--web', web] });
  try {
    await page.goto(engine.url);
    await page.waitForFunction(() => Boolean(window.__touchline), undefined, { timeout: 30_000 });
    await kickOffFromPage(page);
    await page.waitForFunction(() => window.__touchline.lastRenderedTick() > 0, undefined, { timeout: 30_000 });
    grounds.window = true;
    await page.waitForTimeout(PLAY_MS);
    grounds.window = false;
    const frame = await page.evaluate(() => window.__touchline.frame());
    const tick = await page.evaluate(() => window.__touchline.lastRenderedTick());
    const day = await page.evaluate(() => window.__touchline.matchday());
    const listed = await page.locator('[data-screen] section[aria-label="Other grounds"] li').filter({ visible: true }).count();
    return {
      frame,
      tick,
      matchday: { fixtures: day.fixtures.length, listed, ground_messages_in_window: grounds.during, reached: day.reached },
      budgets: rows.filter((r) => r.signal === 'viewer.frame_budget'),
      skipped: rows.filter((r) => r.signal === 'viewer.tick_skipped'),
    };
  } finally {
    engine.cleanUp();
  }
}

test('the match screen holds the frame budget for 60 s at 1x', async ({ browser }, info) => {
  test.setTimeout(6 * 60_000);
  const viewerPage = await browser.newPage();
  const viewer = await measure(viewerPage, VIEWER);
  await viewerPage.close();

  const line = (1.5 * 1000) / viewer.frame.refresh_hz;
  fs.writeFileSync(
    evidence(info, 'frame-budget.json'),
    `${JSON.stringify({ play_ms: PLAY_MS, p95_line_ms: line, viewer }, null, 2)}\n`
  );

  expect(viewer.matchday.fixtures, 'four other fixtures arrived').toBe(4);
  expect(viewer.matchday.listed, 'the list drew them').toBe(4);
  expect(viewer.matchday.ground_messages_in_window, 'the matchday ran during the window').toBeGreaterThan(0);
  expect(viewer.budgets.length, 'a budget row every five seconds').toBeGreaterThanOrEqual(10);
  expect(viewer.skipped).toEqual([]);
  expect(viewer.frame.refresh_hz).toBeGreaterThanOrEqual(59);
  for (const budget of viewer.budgets.slice(1)) {
    expect(budget.fps_median).toBeGreaterThanOrEqual(59);
    expect(budget.frame_ms_p95).toBeLessThanOrEqual((1.5 * 1000) / budget.refresh_hz);
  }
  expect(viewer.frame.frame_ms_p95).toBeLessThanOrEqual(line);
});
