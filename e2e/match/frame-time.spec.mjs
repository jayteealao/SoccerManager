// The match screen's frame time: a real engine, kicked off from the page, played for 60 s at
// 1x. The page writes a `viewer.frame_budget` row every five seconds and a
// `viewer.tick_skipped` row for any second in which it skipped a tick. It passes when every
// budget row after the first holds 59 frames a second or more, no tick is skipped, and the
// 95th percentile frame is no longer than one and a half refresh periods, the line past which
// the scheduler counts a frame as dropped. The background matchday plays all the while (the
// default on a served match): four other fixtures arrive, the list draws them, and ground
// messages keep arriving during the window, with the same budget.
//
// The same budget holds at the huge window step, 2560 by 1440 at 8x, where the pitch is
// largest (at 8x ticks are passed over by design, so none is held to the no-skip rule);
// there the pitch is also sharp: its canvas's backing store equals its box on the
// page in device pixels, within a pixel. On a display at twice the pixel ratio the backing
// store doubles.
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

/// The pitch canvas's backing store against its box on the page in device pixels.
async function sharpness(page) {
  const geometry = await page.evaluate(() => window.__touchline.pitchGeometry());
  const box = await page.evaluate(() => window.__touchline.pitchBox());
  return {
    box,
    backing: geometry.backing,
    device: { width: box.width * box.ratio, height: box.height * box.ratio },
  };
}

/// Serves `web`, kicks off through CONTINUE and the Pre-match KICK OFF, plays 60 s at `speed`
/// (1x unless given), and returns every signal row the page wrote to its console, its final
/// frame budget and the pitch's sharpness at the end.
async function measure(page, web, { speed = 1 } = {}) {
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
    if (speed !== 1) {
      await page.getByRole('group', { name: 'Playback' }).filter({ visible: true }).getByRole('button', { name: `${speed}x`, exact: true }).click();
    }
    grounds.window = true;
    await page.waitForTimeout(PLAY_MS);
    grounds.window = false;
    const frame = await page.evaluate(() => window.__touchline.frame());
    const tick = await page.evaluate(() => window.__touchline.lastRenderedTick());
    const day = await page.evaluate(() => window.__touchline.matchday());
    const listed = await page.locator('[data-screen] section[aria-label="Other grounds"] li').filter({ visible: true }).count();
    return {
      viewport: page.viewportSize(),
      speed,
      pitch: await sharpness(page),
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

test('the match screen holds the frame budget for 60 s at 1x', { tag: '@timing' }, async ({ browser }, info) => {
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

test('at 2560 by 1440 and 8x the pitch draws sharp and holds the frame budget', { tag: '@timing' }, async ({ browser }, info) => {
  test.setTimeout(6 * 60_000);
  const page = await browser.newPage({ viewport: { width: 2560, height: 1440 }, deviceScaleFactor: 1 });
  const viewer = await measure(page, VIEWER, { speed: 8 });
  await page.close();

  const line = (1.5 * 1000) / viewer.frame.refresh_hz;
  fs.writeFileSync(
    evidence(info, 'frame-budget-2560.json'),
    `${JSON.stringify({ play_ms: PLAY_MS, p95_line_ms: line, viewer }, null, 2)}\n`
  );

  // The huge step zooms the page by 1.375; the pitch fills its column, far wider than 742 px.
  expect(viewer.pitch.box.width).toBeGreaterThan(1500);
  expect(Math.abs(viewer.pitch.backing.width - viewer.pitch.device.width)).toBeLessThanOrEqual(1);
  expect(Math.abs(viewer.pitch.backing.height - viewer.pitch.device.height)).toBeLessThanOrEqual(1);
  expect(viewer.budgets.length, 'a budget row every five seconds').toBeGreaterThanOrEqual(10);
  // At 8x the clock passes several ticks a frame, so ticks are passed over by design
  // (lib/schedule.js); the budget here is the frame rate.
  expect(viewer.frame.refresh_hz).toBeGreaterThanOrEqual(59);
  for (const budget of viewer.budgets.slice(1)) {
    expect(budget.fps_median).toBeGreaterThanOrEqual(59);
    expect(budget.frame_ms_p95).toBeLessThanOrEqual((1.5 * 1000) / budget.refresh_hz);
  }
  expect(viewer.frame.frame_ms_p95).toBeLessThanOrEqual(line);
});

test('on a display at twice the pixel ratio the pitch backing store doubles', async ({ browser }, info) => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 });
  const engine = await startEngine({ args: ['--seed', '7', '--web', VIEWER] });
  try {
    await page.goto(engine.url);
    await page.waitForFunction(() => Boolean(window.__touchline), undefined, { timeout: 30_000 });
    await kickOffFromPage(page);
    await page.waitForFunction(() => window.__touchline.lastRenderedTick() > 0, undefined, { timeout: 30_000 });
    const pitch = await sharpness(page);
    fs.writeFileSync(evidence(info, 'pitch-dpr-2.json'), `${JSON.stringify(pitch, null, 2)}\n`);
    expect(pitch.box.ratio).toBe(2);
    expect(Math.abs(pitch.backing.width - pitch.box.width * 2)).toBeLessThanOrEqual(1);
    expect(Math.abs(pitch.backing.height - pitch.box.height * 2)).toBeLessThanOrEqual(1);
  } finally {
    engine.cleanUp();
    await page.close();
  }
});
