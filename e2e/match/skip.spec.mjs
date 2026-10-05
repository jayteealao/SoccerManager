// Skip to result on the new viewer, served by the release engine with `--web viewer/dist`.
//
// Screenshots, each in both skins at 1280 by 800, against their baselines, from one served
// match of seed 7 paused at exactly 30:00 (tick 90000):
// - the Skip decision, after Skip to result is pressed;
// - the engine playing the rest: a relay between the page and the engine passes every server
//   frame up to tick 120000 and holds the rest, so the minute is fixed at 40';
// - the report after the skip, once the held frames are released and the match is stored;
// - the replay of the whole match inside the skipped part (tick 180000).
// The light skin is chosen by configuration only (a content copy's slots.json).
//
// Drives: Keep watching goes back to the paused match at the same tick with no skip sent,
// and RESUME plays on; Skip to result and Confirm give a report whose score is the engine's
// own, and Replay the whole match starts at kick-off and plays into the skipped part (run
// with `--trace on` for its trace); the skipped match holds the same tick frames, events and
// score as the same match played through with no skip; a Tab walk on the decision never
// lands in a stub, and the decision and the report after a skip meet WCAG AA contrast.
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import { VIEWER, contentWithSkin, fastForward, startEngine, waitForGrounds } from '../support/engine.mjs';
import { clickClear, playUntil, snap } from '../support/page.mjs';

const SKINS = ['broadcast-blue', 'interim-light'];
const SEED = 7;
/// The skip point: 30:00.
const SKIP_TICK = 90_000;
/// The playing screenshot's newest tick: 40:00.
const HOLD_TICK = 120_000;
/// The replay screenshot's tick, inside the skipped part: 60:00.
const REPLAY_TICK = 180_000;
/// Past the end of any match: a served match with this fast-forward plays straight through.
const PAST_THE_END = 1_000_000;
/// A minute of match time at 50 ticks a second.
const MINUTE = 3_000;

const fixture = {};

test.beforeAll(() => {
  fixture.light = contentWithSkin('interim-light');
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

async function serve(skin = 'broadcast-blue', to = SKIP_TICK) {
  return startEngine({
    args: ['--seed', String(SEED), '--web', VIEWER, ...fastForward(to)],
    env: skin === 'broadcast-blue' ? {} : { SM_CONTENT_DIR: fixture.light },
  });
}

/// Rewinds to exactly `tick`, so a screenshot is taken at the same tick every run.
async function rewindTo(page, tick) {
  await page.getByRole('slider', { name: 'Rewind to a tick' }).filter({ visible: true }).evaluate((scrub, value) => {
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

/// From the page's first screen: CONTINUE on Tactics, KICK OFF on the Pre-match line-ups,
/// 8x until `tick` is stored, pause with the action block, and a rewind to exactly `tick`.
async function kickOffAndPauseAt(page, tick) {
  await until(page, () => window.__touchline.view() === 'tactics', undefined, 30_000);
  await action(page).click();
  await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
  await action(page).click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
  await playback(page).getByRole('button', { name: '8x', exact: true }).click();
  await playUntil(page, (t) => window.__touchline.history().newest_tick >= t, { arg: tick, timeout: 5 * 60_000 });
  await action(page).click();
  await expect(action(page)).toHaveText('Resume');
  await rewindTo(page, tick);
}

/// The Skip decision at 30:00.
async function decision(page) {
  await kickOffAndPauseAt(page, SKIP_TICK);
  expect(await hook(page, () => window.__touchline.canSkip())).toBe(true);
  await button(page, 'Skip to result').click();
  await until(page, () => window.__touchline.view() === 'skip', undefined, 5_000);
}

/// A TCP relay between the page and the engine's socket that holds every server message once
/// a tick frame past `limit` arrives, until `release()`; the engine's close waits with them.
/// The page is pointed at the relay by its status (`socket.port`). The relay reads the
/// server's WebSocket frames (unmasked, unfragmented) only to count ticks; the bytes pass
/// through unchanged, so the page receives exactly what the engine sent.
async function holdAfter(page, limit) {
  const gate = { held: [], holding: false, released: false, enginePort: null, flush: null };
  const relay = net.createServer((client) => {
    const up = net.connect(gate.enginePort, '127.0.0.1');
    client.pipe(up);
    let buffer = Buffer.alloc(0);
    let upgraded = false;
    let tick = 0;
    let ended = false;
    const send = (bytes) => (gate.holding ? gate.held.push(bytes) : client.write(bytes));
    up.on('data', (chunk) => {
      buffer = Buffer.concat([buffer, chunk]);
      if (!upgraded) {
        const end = buffer.indexOf('\r\n\r\n');
        if (end < 0) {
          return;
        }
        client.write(buffer.subarray(0, end + 4));
        buffer = buffer.subarray(end + 4);
        upgraded = true;
      }
      for (;;) {
        if (buffer.length < 2) {
          break;
        }
        let length = buffer[1] & 0x7f;
        let at = 2;
        if (length === 126) {
          if (buffer.length < 4) {
            break;
          }
          length = buffer.readUInt16BE(2);
          at = 4;
        } else if (length === 127) {
          if (buffer.length < 10) {
            break;
          }
          length = Number(buffer.readBigUInt64BE(2));
          at = 10;
        }
        if (buffer[1] & 0x80) {
          at += 4;
        }
        if (buffer.length < at + length) {
          break;
        }
        if ((buffer[0] & 0x0f) === 0x02) {
          const kind = buffer[at];
          if (kind === 0x01 || kind === 0x03) {
            tick = buffer.readUInt32LE(at + 1);
          } else if (kind === 0x02) {
            tick += 1;
          }
          if (tick > limit && !gate.released) {
            gate.holding = true;
          }
        }
        send(Buffer.from(buffer.subarray(0, at + length)));
        buffer = buffer.subarray(at + length);
      }
    });
    up.on('end', () => {
      ended = true;
      if (!gate.holding) {
        client.end();
      }
    });
    gate.flush = () => {
      for (const bytes of gate.held) {
        client.write(bytes);
      }
      gate.held = [];
      if (ended) {
        client.end();
      }
    };
    client.on('error', () => {});
    up.on('error', () => {});
    client.on('close', () => up.destroy());
  });
  await new Promise((resolve) => relay.listen(0, '127.0.0.1', resolve));
  gate.release = () => {
    gate.holding = false;
    gate.released = true;
    gate.flush?.();
  };
  gate.close = () => relay.close();
  await page.route('**/engine.json', async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    if (body['socket.port']) {
      gate.enginePort = body['socket.port'];
      body['socket.port'] = relay.address().port;
    }
    await route.fulfill({ response, json: body });
  });
  return gate;
}

/// Confirm on the decision; the engine plays the rest and the report opens when the match is
/// stored.
async function confirmAndWait(page, timeout = 5 * 60_000) {
  await button(page, 'Confirm: skip to result').click();
  await until(page, () => window.__touchline.skip()?.state === 'ready', undefined, timeout);
  await until(page, () => window.__touchline.report().state === 'ready', undefined, 10_000);
}

const eventRows = (events) =>
  events.map((e) => [e.tick, e['event.type'], e['team.id'] ?? null, e['home.score'] ?? null, e['away.score'] ?? null]);

for (const skin of SKINS) {
  test.describe(`skip to result in ${skin}`, () => {
    test('the decision, the engine playing the rest, the report after a skip and the replay', { tag: '@sizes' }, async ({ page }) => {
      test.setTimeout(10 * 60_000);
      const gate = await holdAfter(page, HOLD_TICK);
      const engine = await serve(skin);
      try {
        await open(page, engine.url, skin);
        await decision(page);
        await expect(page.locator('[data-screen="skip"]')).toBeVisible();
        // The decision names the other grounds once the matchday's fixtures are in.
        await until(page, () => window.__touchline.matchday().fixtures.length === 4, undefined, 10_000);
        await snap(page, `skip-decision-${skin}.png`);

        await button(page, 'Confirm: skip to result').click();
        await until(page, () => window.__touchline.skip()?.state === 'playing', undefined, 10_000);
        await expect(page.locator('[data-screen="report"]')).toContainText("PLAYING THE REST · 40'", { timeout: 60_000 });
        expect(gate.holding).toBe(true);
        await snap(page, `skip-playing-${skin}.png`);

        gate.release();
        await until(page, () => window.__touchline.report().state === 'ready', undefined, 5 * 60_000);
        await expect(page.locator('[data-screen="report"]')).toHaveAttribute('data-skipped', String(SKIP_TICK));
        // The report lists every other ground final.
        await waitForGrounds(page, Number.MAX_SAFE_INTEGER, 180_000);
        await expect(page.locator('[data-screen="report"]')).toContainText('Matchday 1 · final');
        await snap(page, `report-skipped-${skin}.png`);

        await button(page, 'Replay the whole match').click();
        await until(page, () => window.__touchline.view() === 'replay', undefined, 5_000);
        await pause(page);
        await rewindTo(page, REPLAY_TICK);
        await expect(page.locator('[data-screen="replay"] .tagc')).toContainText('NOT WATCHED LIVE');
        await snap(page, `replay-skipped-${skin}.png`);
      } finally {
        engine.cleanUp();
        gate.close();
      }
    });
  });
}

test('Keep watching goes back to the paused match at the same tick, and RESUME plays on', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  const engine = await serve();
  try {
    await open(page, engine.url);
    await decision(page);
    const before = await hook(page, () => window.__touchline.lastRenderedTick());
    await button(page, 'Keep watching').click();
    await until(page, () => window.__touchline.view() === 'match', undefined, 5_000);
    const after = await hook(page, () => window.__touchline.lastRenderedTick());
    const sent = await hook(page, () => window.__touchline.sentCommands());
    expect(before).toBe(SKIP_TICK);
    expect(after).toBe(SKIP_TICK);
    expect(sent).not.toContain('skip');
    expect(await hook(page, () => window.__touchline.skip())).toBeNull();
    await expect(action(page)).toHaveText('Resume');
    await action(page).click();
    await until(page, (t) => window.__touchline.lastRenderedTick() > t, SKIP_TICK, 15_000);
    const resumed = await hook(page, () => window.__touchline.lastRenderedTick());
    fs.writeFileSync(
      evidence(info, 'keep-watching.json'),
      `${JSON.stringify({ before, after, resumed, sent }, null, 2)}\n`
    );
  } finally {
    engine.cleanUp();
  }
});

test('a skipped match reports the final score, replays all of it, and equals the match played through', async ({ page }, info) => {
  // The trace of this drive is evidence: run it with `--trace on` to keep it when it passes.
  test.setTimeout(15 * 60_000);
  let engine = await serve();
  const runs = {};
  try {
    await open(page, engine.url);
    await decision(page);
    await page.screenshot({ path: evidence(info, 'skip-drive-decision.png') });
    await confirmAndWait(page);
    await page.screenshot({ path: evidence(info, 'skip-drive-report.png') });
    const events = await hook(page, () => window.__touchline.events());
    const fullTime = events.find((e) => e['event.type'] === 'full-time');
    const report = await hook(page, () => window.__touchline.report());
    expect(report).toMatchObject({ kind: 'full-time', state: 'ready', tick: fullTime.tick });
    expect(report.score).toEqual([fullTime['home.score'], fullTime['away.score']]);
    await expect(page.locator('.strip .score:visible')).toHaveText(`${report.score[0]} – ${report.score[1]}`);
    runs.skipped = {
      digest: await hook(page, () => window.__touchline.tickFrameDigest()),
      events: eventRows(events),
      score: report.score,
      ticks: (await hook(page, () => window.__touchline.replay())).ticks,
      sent: await hook(page, () => window.__touchline.sentCommands()),
    };
    expect(runs.skipped.sent.filter((c) => c === 'skip')).toHaveLength(1);

    await button(page, 'Replay the whole match').click();
    await until(page, () => window.__touchline.view() === 'replay', undefined, 5_000);
    expect((await hook(page, () => window.__touchline.lastRewind())).tick).toBeLessThanOrEqual(1);
    await pause(page);
    await rewindTo(page, SKIP_TICK - MINUTE / 6);
    await playback(page).getByRole('button', { name: '8x', exact: true }).click();
    await playback(page).getByRole('button', { name: 'Play', exact: true }).click();
    await until(page, (t) => window.__touchline.lastRenderedTick() > t, SKIP_TICK + MINUTE, 30_000);
    await expect(page.locator('[data-screen="replay"] .tagc')).toContainText('NOT WATCHED LIVE');
    await page.screenshot({ path: evidence(info, 'skip-drive-replay.png') });
    engine.cleanUp();

    // The same seed and kick-off flow, played straight through with no skip.
    engine = await serve('broadcast-blue', PAST_THE_END);
    await open(page, engine.url);
    await kickOffAndPauseAt(page, fullTime.tick);
    // The engine closes after full time once the page has every frame: the whole match is
    // stored.
    await until(page, () => /whole match is here/.test(window.__touchline.notice().message ?? ''), undefined, 60_000);
    const watchedEvents = await hook(page, () => window.__touchline.events());
    const watchedEnd = watchedEvents.find((e) => e['event.type'] === 'full-time');
    runs.watched = {
      digest: await hook(page, () => window.__touchline.tickFrameDigest()),
      events: eventRows(watchedEvents),
      score: [watchedEnd['home.score'], watchedEnd['away.score']],
      ticks: (await hook(page, () => window.__touchline.replay())).ticks,
      sent: await hook(page, () => window.__touchline.sentCommands()),
    };
    fs.writeFileSync(
      evidence(info, 'skip-equality.json'),
      `${JSON.stringify(
        {
          skipped: { ...runs.skipped, events: runs.skipped.events.length },
          watched: { ...runs.watched, events: runs.watched.events.length },
        },
        null,
        2
      )}\n`
    );
    expect(runs.watched.sent).not.toContain('skip');
    expect(runs.skipped.ticks).toBe(runs.watched.ticks);
    expect(runs.skipped.digest).toBe(runs.watched.digest);
    expect(runs.skipped.events).toEqual(runs.watched.events);
    expect(runs.skipped.score).toEqual(runs.watched.score);
  } finally {
    engine.cleanUp();
  }
});

/// Presses Tab `presses` times and records where focus lands.
async function tabWalk(page, presses) {
  await clickClear(page);
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

test('a Tab walk on the decision never lands in a stub, and the skip screens meet WCAG AA contrast', { tag: '@sizes' }, async ({ page }, info) => {
  test.setTimeout(10 * 60_000);
  const engine = await serve();
  try {
    await open(page, engine.url);
    await decision(page);
    const walk = await tabWalk(page, 16);
    const decisionContrast = await contrastReport(page);
    await page.keyboard.press('Escape');
    await until(page, () => window.__touchline.view() === 'match', undefined, 5_000);
    await button(page, 'Skip to result').click();
    await until(page, () => window.__touchline.view() === 'skip', undefined, 5_000);
    await confirmAndWait(page);
    const reportContrast = await contrastReport(page);
    fs.writeFileSync(evidence(info, 'skip-tab-walk.json'), `${JSON.stringify(walk, null, 2)}\n`);
    fs.writeFileSync(
      evidence(info, 'skip-contrast-report.json'),
      `${JSON.stringify({ decision: decisionContrast, report: reportContrast }, null, 2)}\n`
    );
    for (const step of walk) {
      expect(step.stub, `press ${step.press}: ${step.name}`).toBeNull();
      expect(step.inert, `press ${step.press}: ${step.name}`).toBe(false);
      expect(step.hidden, `press ${step.press}: ${step.name}`).toBe(false);
    }
    const names = walk.map((s) => s.name);
    for (const name of ['Resume', 'Confirm: skip to result', 'Keep watching']) {
      expect(names).toContain(name);
    }
    expect(names.some((n) => n.startsWith('Skip to the final whistle'))).toBe(true);
    for (const report of [decisionContrast, reportContrast]) {
      expect(report.checked).toBeGreaterThan(20);
      expect(report.failures, JSON.stringify(report.failures, null, 2)).toEqual([]);
    }
  } finally {
    engine.cleanUp();
  }
});
