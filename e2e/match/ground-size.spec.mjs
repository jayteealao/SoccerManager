// The pitch canvas draws the home ground at its own size, served by the release engine with
// `--web viewer/dist`.
//
// Screenshot: the pitch box at minute 30 of the seed-42 match whose home team plays on a
// 100 by 64 ground, paused, in the default skin at 1280 by 800, against its baseline. The
// ground comes from configuration only: a content copy whose home team file names it. The
// engine sends every tick up to minute 30 at once, so the drive never waits for playback.
//
// Drive: the hello on the wire names the ground; the test hook reports a drawn rectangle
// inside the pitch box (742 by 312 at the reference aspect, sized by its column), centred,
// narrower and shorter than the default ground's, at the default ground's tilt for that box
// and in the ground's proportion. A control run on the shipped content reports the default
// ground filling the box. Each run reads the box from the geometry, never a fixed size.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { VIEWER, contentWithGround, fastForward, startEngine } from '../support/engine.mjs';
import { openMatch, playUntil, snap } from '../support/page.mjs';

/// Minute 30 at 50 ticks a second (the hello's `dt_ms` of 20, which the drive checks).
const MINUTE_30 = 90_000;
const GROUND = { length: 100, width: 64 };
/// The match screen's pitch box keeps this aspect at every size.
const ASPECT = 742 / 312;

const fixture = {};

test.beforeAll(() => {
  fixture.content = contentWithGround(GROUND.length, GROUND.width);
});

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR
    ? path.join(process.env.MATCH_EVIDENCE_DIR, name)
    : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

const hook = (page, fn, arg) => page.evaluate(fn, arg);
const until = (page, fn, arg, timeout = 120_000) =>
  page.waitForFunction(fn, arg, { timeout, polling: 100 });

/// The action block of the screen on show (the hidden screen keeps its own).
const action = (page) => page.locator('header button.cont:visible');
const pitchBox = (page) => page.locator('.pitchbox:visible');

async function serve(content, to) {
  return startEngine({
    args: ['--seed', '42', '--web', VIEWER, ...fastForward(to)],
    env: content ? { SM_CONTENT_DIR: content } : {},
  });
}

/// Opens the page, waits for the skin and the fonts, and returns the hello it received.
async function open(page, url) {
  const seen = await openMatch(page, url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', 'broadcast-blue', { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
  await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
  return seen;
}

/// From Tactics: CONTINUE, KICK OFF, play at 8x until `tick` is stored, then pause.
async function playTo(page, tick) {
  await action(page).click();
  await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
  await action(page).click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
  await page.getByRole('group', { name: 'Playback' }).getByRole('button', { name: '8x', exact: true }).click();
  await playUntil(page, (t) => window.__touchline.history().newest_tick >= t, { arg: tick, timeout: 6 * 60_000 });
  await action(page).click();
  await expect(action(page)).toHaveText('Resume');
}

/// Rewinds to exactly `tick`, so a screenshot is taken at the same tick every run.
async function rewindTo(page, tick) {
  await page.getByRole('slider', { name: 'Rewind to a tick' }).evaluate((scrub, value) => {
    scrub.value = String(value);
    scrub.dispatchEvent(new Event('input', { bubbles: true }));
    scrub.dispatchEvent(new Event('change', { bubbles: true }));
  }, tick);
  await until(page, (t) => window.__touchline.lastRenderedTick() === t, tick, 10_000);
}

test('a 100 by 64 home ground at minute 30 draws smaller, centred and in proportion', { tag: '@sizes' }, async ({ page }, info) => {
  test.setTimeout(10 * 60_000);
  const errors = [];
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  const engine = await serve(fixture.content, MINUTE_30);
  try {
    const seen = await open(page, engine.url);
    expect(seen.hello, 'the page received a hello').not.toBeNull();
    expect(seen.hello.dt_ms, 'minute 30 is tick 90000').toBe(20);
    expect(seen.hello['ground.length']).toBe(GROUND.length);
    expect(seen.hello['ground.width']).toBe(GROUND.width);

    await playTo(page, MINUTE_30);
    await rewindTo(page, MINUTE_30);
    const geometry = await hook(page, () => window.__touchline.pitchGeometry());
    fs.writeFileSync(
      evidence(info, 'pitch-geometry-100x64.json'),
      `${JSON.stringify({ hello: { length: seen.hello['ground.length'], width: seen.hello['ground.width'] }, geometry }, null, 2)}\n`
    );
    expect(geometry.ground).toEqual(GROUND);
    const BOX = geometry.box;
    expect(Math.abs(BOX.width / BOX.height - ASPECT), 'the box keeps its aspect').toBeLessThan(0.01);
    const { rect, tilt } = geometry;
    // Inside the box, narrower and shorter than the default ground, which fills it.
    expect(rect.left).toBeGreaterThan(1);
    expect(rect.top).toBeGreaterThan(1);
    expect(rect.width).toBeLessThan(BOX.width - 2);
    expect(rect.height).toBeLessThan(BOX.height - 2);
    // Centred, at the default ground's tilt, and in the ground's proportion, within 0.5 px.
    expect(Math.abs(rect.left - (BOX.width - rect.left - rect.width))).toBeLessThan(0.5);
    expect(Math.abs(rect.top - (BOX.height - rect.top - rect.height))).toBeLessThan(0.5);
    expect(Math.abs(tilt - (BOX.height - 2) / 68 / ((BOX.width - 2) / 105))).toBeLessThan(1e-9);
    const expectedHeight = (rect.width * (GROUND.width / GROUND.length)) * tilt;
    expect(Math.abs(rect.height - expectedHeight)).toBeLessThan(0.5);
    // The backing store is the box drawn on the page in device pixels, within a pixel.
    const drawn = await hook(page, () => window.__touchline.pitchBox());
    expect(Math.abs(geometry.backing.width - drawn.width * drawn.ratio)).toBeLessThanOrEqual(1);
    expect(Math.abs(geometry.backing.height - drawn.height * drawn.ratio)).toBeLessThanOrEqual(1);

    await snap(page, 'pitch-100x64-minute-30.png', pitchBox(page));
    expect(errors, 'no console error').toEqual([]);
  } finally {
    engine.cleanUp();
  }
});

test('control: the shipped content plays on 105 by 68, which fills the box', async ({ page }, info) => {
  const engine = await serve(null);
  try {
    const seen = await open(page, engine.url);
    expect(seen.hello['ground.length'], 'a default ground is left out of the hello').toBeUndefined();
    expect(seen.hello['ground.width']).toBeUndefined();
    const geometry = await until(page, () => window.__touchline.pitchGeometry(), undefined, 30_000).then((h) =>
      h.jsonValue()
    );
    fs.writeFileSync(evidence(info, 'pitch-geometry-default.json'), `${JSON.stringify(geometry, null, 2)}\n`);
    expect(geometry.ground).toEqual({ length: 105, width: 68 });
    const BOX = geometry.box;
    expect(Math.abs(BOX.width / BOX.height - ASPECT), 'the box keeps its aspect').toBeLessThan(0.01);
    expect(geometry.rect).toEqual({ left: 1, top: 1, width: BOX.width - 2, height: BOX.height - 2 });
  } finally {
    engine.cleanUp();
  }
});
