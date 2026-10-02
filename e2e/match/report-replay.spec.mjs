// The report, the replay and the handshake page on the new viewer, served by the release
// engine with `--web viewer/dist`.
//
// Screenshots, each in both skins at 1280 by 800, against their baselines:
// - the full-time report loading: a one-minute live match played to full time, with the
//   engine's close after full time held back, so the whole match is never stored;
// - the full-time report ready: the seed-7 whole match opened as a replay file, rewound to a
//   minute before full time and played until the report opens;
// - the replay: the same file, paused by a rewind at tick 90000;
// - the handshake page: `/engine.json` answered with a fixed body, and the socket with a fixed
//   hello, three tick frames and a close.
// The light skin is chosen by configuration only (a content copy's slots.json, or the fixed
// body).
//
// Drives: the half-time report opens at the break, its counts equal the event list, and
// CONTINUE resumes; Save replay downloads a file whose size and hash equal the page's own,
// and that file opened again plays every tick with an exact rewind; Replay the whole match
// starts at kick-off; RUN AGAIN against a real engine prints a hello again. A Tab walk on the
// three screens never lands in a stub, and the rendered contrast check passes.
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import { VIEWER, contentWithSkin, runEngine, startEngine, tempDir, waitForGrounds } from '../support/engine.mjs';

const SKINS = ['broadcast-blue', 'interim-light'];
/// The replay screenshot's tick: minute 30.
const REPLAY_TICK = 90_000;
/// A minute of match time at 50 ticks a second.
const MINUTE = 3_000;

const fixture = {};

test.beforeAll(() => {
  fixture.light = contentWithSkin('interim-light');
  fixture.dir = tempDir('seed7-record');
  fixture.file = path.join(fixture.dir, 'seed7.smfx');
  const run = runEngine(['record', '--seed', '7', '--out', fixture.file]);
  if (run.code !== 0) {
    throw new Error(`record failed with ${run.code}: ${run.stderr}`);
  }
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

/// The action block of the screen on show (the hidden match screen keeps its own).
const action = (page) => page.locator('header button.cont:visible');
const playback = (page) => page.getByRole('group', { name: 'Playback' }).filter({ visible: true });
const button = (page, name) => page.getByRole('button', { name, exact: true }).filter({ visible: true });

async function open(page, url, skin = 'broadcast-blue') {
  await page.goto(url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', skin, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

async function serve(skin = 'broadcast-blue', args = ['--seed', '7']) {
  return startEngine({
    args: [...args, '--web', VIEWER],
    env: skin === 'broadcast-blue' ? {} : { SM_CONTENT_DIR: fixture.light },
  });
}

/// Opens `file` as a replay file on the page the engine serves; the replay view opens.
async function openReplayFile(page, engine, skin, file = fixture.file) {
  await open(page, engine.url, skin);
  await until(page, () => ['tactics', 'match'].includes(window.__touchline.view()), undefined, 30_000);
  await page.locator('input[type="file"]').first().setInputFiles(file);
  await until(page, () => window.__touchline.view() === 'replay' && window.__touchline.replay().stored, undefined, 60_000);
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

async function pause(page) {
  const group = playback(page);
  if (await group.getByRole('button', { name: 'Pause', exact: true }).count()) {
    await group.getByRole('button', { name: 'Pause', exact: true }).click();
  }
}

const breakTick = (page, kind) =>
  hook(page, (k) => window.__touchline.events().find((e) => e['event.type'] === k)?.tick ?? null, kind);

/// From the replay view: a rewind to a minute before full time, then 8x until the report opens.
async function reportFromFile(page) {
  const fullTime = await breakTick(page, 'full-time');
  await pause(page);
  await rewindTo(page, fullTime - MINUTE);
  await playback(page).getByRole('button', { name: '8x', exact: true }).click();
  await playback(page).getByRole('button', { name: 'Play', exact: true }).click();
  await until(page, () => window.__touchline.view() === 'report', undefined, 60_000);
  return fullTime;
}

/// A one-minute live match, from Tactics to the full-time report at 8x; CONTINUE on the
/// half-time report on the way.
async function liveToFullTime(page) {
  await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
  await action(page).click();
  await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
  await action(page).click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
  await playback(page).getByRole('button', { name: '8x', exact: true }).click();
  for (;;) {
    await until(page, () => window.__touchline.view() === 'report', undefined, 120_000);
    if ((await hook(page, () => window.__touchline.report().kind)) === 'full-time') {
      return;
    }
    await action(page).click();
    await until(page, () => window.__touchline.view() !== 'report', undefined, 5_000);
  }
}

/// The handshake page with a fixed status and a fixed socket: a hello, three tick frames and
/// a close. The close carries code 4006: a close frame cannot carry 1006, which a browser
/// reports only for a connection lost without one.
async function fixedHandshake(page, engine, skin) {
  await page.route('**/engine.json', (route) =>
    route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify({ 'engine.state': 'running', 'socket.port': 9, 'protocol.version': 3, 'viewer.skin': skin }),
    })
  );
  await page.routeWebSocket(/:9\/\?v=3$/, (ws) => {
    ws.send(JSON.stringify({ type: 'hello', 'match.id': 'fixed', 'protocol.version': 3, 'engine.version': '0.3.0' }));
    for (const length of [1184, 312, 298]) {
      const frame = Buffer.alloc(length);
      frame[0] = length === 1184 ? 1 : 2;
      ws.send(frame);
    }
    ws.close({ code: 4006, reason: 'fixed' });
  });
  await open(page, `${engine.url}handshake.html`, skin);
  await expect(page.getByRole('log')).toContainText('closed code=', { timeout: 10_000 });
}

for (const skin of SKINS) {
  test.describe(`the report, replay and handshake in ${skin}`, () => {
    test('the full-time report while the whole match is still being stored', async ({ page }) => {
      test.setTimeout(6 * 60_000);
      // The engine's close after full time never reaches the page: the store stays open.
      await page.routeWebSocket(/\/\?v=\d+$/, (ws) => {
        const server = ws.connectToServer();
        server.onClose(() => {});
      });
      const engine = await serve(skin, ['--seed', '7', '--minutes', '1']);
      try {
        await open(page, engine.url, skin);
        await liveToFullTime(page);
        // Every other ground has ended, so the list in the report is the same every run.
        await waitForGrounds(page, Number.MAX_SAFE_INTEGER, 180_000);
        expect(await hook(page, () => window.__touchline.report().state)).toBe('loading');
        await expect(button(page, 'Save replay')).toBeDisabled();
        await expect(page).toHaveScreenshot(`report-loading-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('the full-time report, ready', async ({ page }) => {
      test.setTimeout(6 * 60_000);
      const engine = await serve(skin);
      try {
        await openReplayFile(page, engine, skin);
        await reportFromFile(page);
        expect(await hook(page, () => window.__touchline.report())).toMatchObject({ kind: 'full-time', state: 'ready' });
        await expect(page).toHaveScreenshot(`report-ready-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('the replay, paused at minute 30', async ({ page }) => {
      test.setTimeout(6 * 60_000);
      const engine = await serve(skin);
      try {
        await openReplayFile(page, engine, skin);
        await pause(page);
        await rewindTo(page, REPLAY_TICK);
        await expect(page).toHaveScreenshot(`replay-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('the handshake page after a close', async ({ page }) => {
      const engine = await serve(skin);
      try {
        await fixedHandshake(page, engine, skin);
        await expect(page).toHaveScreenshot(`handshake-${skin}.png`, {
          // The page's own origin carries the engine's port, which changes every run.
          mask: [page.getByRole('log').locator('span').nth(1)],
        });
      } finally {
        engine.cleanUp();
      }
    });
  });
}

test('the half-time report opens at the break, counts the event list, and CONTINUE resumes', async ({ page }) => {
  test.setTimeout(6 * 60_000);
  const engine = await serve();
  try {
    await openReplayFile(page, engine, 'broadcast-blue');
    const halfTime = await breakTick(page, 'half-time');
    await pause(page);
    await rewindTo(page, halfTime - MINUTE);
    await playback(page).getByRole('button', { name: '8x', exact: true }).click();
    await playback(page).getByRole('button', { name: 'Play', exact: true }).click();
    await until(page, () => window.__touchline.view() === 'report', undefined, 60_000);
    const report = await hook(page, () => window.__touchline.report());
    expect(report).toMatchObject({ kind: 'half-time', open: true, state: 'ready', tick: halfTime });
    const events = await hook(page, () => window.__touchline.events());
    const upto = events.filter((e) => e.tick <= halfTime);
    const goals = (e) => e['event.type'] === 'goal';
    const fouls = (e) => e['event.type'] === 'foul';
    const corners = (e) => e['event.type'] === 'corner';
    // Each side's goals equal the engine's own score at the break, which is not a count.
    const last = upto.at(-1);
    expect(report.counts.goals).toEqual([last['home.score'], last['away.score']]);
    const total = (id) => report.counts[id][0] + report.counts[id][1];
    expect(total('fouls')).toBe(upto.filter(fouls).length);
    expect(total('corners')).toBe(upto.filter(corners).length);
    expect(total('goals')).toBe(upto.filter(goals).length);
    expect(await hook(page, () => window.__touchline.matchDay().playing)).toBe(false);
    await action(page).click();
    await until(page, () => window.__touchline.view() === 'replay', undefined, 5_000);
    const at = await hook(page, () => window.__touchline.lastRenderedTick());
    await until(page, (t) => window.__touchline.lastRenderedTick() > t, at, 10_000);
  } finally {
    engine.cleanUp();
  }
});

test('Save replay downloads the whole match, and the file plays every tick with an exact rewind', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  const engine = await serve('broadcast-blue', ['--seed', '7', '--minutes', '1']);
  try {
    await open(page, engine.url);
    await liveToFullTime(page);
    await until(page, () => window.__touchline.report().state === 'ready', undefined, 60_000);
    const [download] = await Promise.all([page.waitForEvent('download'), button(page, 'Save replay').click()]);
    const saved = evidence(info, 'saved.smfx');
    await download.saveAs(saved);
    const bytes = fs.readFileSync(saved);
    const replay = await hook(page, () => window.__touchline.replay());
    expect(download.suggestedFilename()).toBe(replay.last_saved_name);
    expect(bytes.length).toBe(replay.last_saved_size);
    const own = Buffer.from(await hook(page, () => window.__touchline.lastSavedBytes()));
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(createHash('sha256').update(own).digest('hex'));

    await page.locator('input[type="file"]').first().setInputFiles(saved);
    await until(page, () => window.__touchline.replay().stored, undefined, 30_000);
    const reopened = await hook(page, () => window.__touchline.replay());
    expect(reopened.ticks).toBe(replay.ticks);
    expect(await hook(page, () => window.__touchline.history().ticks_stored)).toBe(replay.ticks);
    await until(page, () => window.__touchline.view() === 'replay', undefined, 10_000);
    await pause(page);
    await rewindTo(page, 1234);
    expect(await hook(page, () => window.__touchline.lastRewind())).toMatchObject({ tick: 1234, exact: true });
  } finally {
    engine.cleanUp();
  }
});

test('Replay the whole match starts at kick-off', async ({ page }) => {
  test.setTimeout(6 * 60_000);
  const engine = await serve();
  try {
    await openReplayFile(page, engine, 'broadcast-blue');
    await reportFromFile(page);
    await button(page, 'Replay the whole match').click();
    await until(page, () => window.__touchline.view() === 'replay', undefined, 5_000);
    expect((await hook(page, () => window.__touchline.lastRewind())).tick).toBeLessThanOrEqual(1);
    await action(page).click();
    await until(page, () => window.__touchline.view() === 'report', undefined, 5_000);
  } finally {
    engine.cleanUp();
  }
});

test('RUN AGAIN against a real engine prints a hello again', async ({ page }) => {
  const engine = await serve();
  // A served match ends when its page closes the socket, so RUN AGAIN reads the status of a
  // second real engine and connects to it: the check runs again from the status fetch on.
  let second = null;
  try {
    await open(page, `${engine.url}handshake.html`);
    const log = page.getByRole('log');
    await expect(log).toContainText('hello: ', { timeout: 15_000 });
    await expect(page.locator('.strip')).toContainText('Yes');
    second = await serve('broadcast-blue', ['--seed', '8']);
    await page.route('**/engine.json', async (route) =>
      route.fulfill({ response: await route.fetch({ url: `${second.url}engine.json` }) })
    );
    await action(page).click();
    await expect(log).not.toContainText('"seed":7', { timeout: 5_000 });
    await expect(log).toContainText('hello: ', { timeout: 15_000 });
    await expect(log).toContainText('"seed":8');
    expect(await log.textContent()).toMatch(/^connecting…/);
  } finally {
    engine.cleanUp();
    second?.cleanUp();
  }
});

/// Presses Tab `presses` times and records where focus lands.
async function tabWalk(page, presses) {
  await page.locator('body').click({ position: { x: 1270, y: 790 } });
  const walk = [];
  for (let press = 1; press <= presses; press += 1) {
    await page.keyboard.press('Tab');
    walk.push(
      await page.evaluate((n) => {
        const el = document.activeElement;
        return {
          press: n,
          tag: el?.tagName.toLowerCase() ?? null,
          name: (el?.getAttribute('aria-label') ?? el?.textContent ?? '').trim().replace(/\s+/g, ' ').slice(0, 60),
          stub: el?.closest('[data-stub]')?.dataset.stub ?? null,
          inert: Boolean(el?.closest('[inert]')),
          hidden: Boolean(el?.closest('[hidden]')),
        };
      }, press)
    );
  }
  return walk;
}

function expectClean(walk) {
  for (const step of walk) {
    expect(step.stub, `press ${step.press}: ${step.name}`).toBeNull();
    expect(step.inert, `press ${step.press}: ${step.name}`).toBe(false);
    expect(step.hidden, `press ${step.press}: ${step.name}`).toBe(false);
  }
}

test('a Tab walk on the three screens never lands in a stub, and they meet WCAG AA contrast', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  const engine = await serve();
  let second = null;
  try {
    await openReplayFile(page, engine, 'broadcast-blue');
    await pause(page);
    await rewindTo(page, REPLAY_TICK);
    const replayWalk = await tabWalk(page, 20);
    const replayContrast = await contrastReport(page);
    await reportFromFile(page);
    const reportWalk = await tabWalk(page, 20);
    const reportContrast = await contrastReport(page);
    // The served match ended when the replay file closed its socket; a second engine serves
    // the handshake page.
    second = await serve();
    await fixedHandshake(page, second, 'broadcast-blue');
    const handshakeWalk = await tabWalk(page, 8);
    const handshakeContrast = await contrastReport(page);
    fs.writeFileSync(
      evidence(info, 'report-replay-tab-walk.json'),
      `${JSON.stringify({ replay: replayWalk, report: reportWalk, handshake: handshakeWalk }, null, 2)}\n`
    );
    fs.writeFileSync(
      evidence(info, 'report-replay-contrast-report.json'),
      `${JSON.stringify({ replay: replayContrast, report: reportContrast, handshake: handshakeContrast }, null, 2)}\n`
    );
    expectClean(replayWalk);
    expectClean(reportWalk);
    expectClean(handshakeWalk);
    const names = (walk) => walk.map((s) => s.name);
    for (const name of ['Continue', 'Back 10 seconds', 'Forward 10 seconds', '8x']) {
      expect(names(replayWalk)).toContain(name);
    }
    expect(names(replayWalk).some((n) => n === 'Play' || n === 'Pause')).toBe(true);
    for (const name of ['Continue', 'Replay the whole match', 'Save replay', 'Open a replay', 'Close', 'Replay']) {
      expect(names(reportWalk)).toContain(name);
    }
    expect(names(handshakeWalk)).toContain('Run again');
    expect(handshakeWalk.some((s) => s.name === 'Handshake log, newest last' || s.tag === 'div')).toBe(true);
    for (const report of [replayContrast, reportContrast, handshakeContrast]) {
      expect(report.checked).toBeGreaterThan(20);
      expect(report.failures, JSON.stringify(report.failures, null, 2)).toEqual([]);
    }
  } finally {
    engine.cleanUp();
    second?.cleanUp();
  }
});
