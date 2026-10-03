// The match screen on the new viewer, served by the release engine with `--web viewer/dist`.
//
// Screenshots: each of the six states in each skin at 1280 by 800, against its baseline.
// Live and kick-off pixels come from a real engine. The loading, error, first-run and
// reconnect states pass quickly or carry a path, an exit code or a minute that changes from
// run to run, so their pixels come from a fixed `/engine.json` body (for the reconnect, one
// that stays starting); each of those states also has a real-engine drive below that asserts its
// behaviour without comparing pixels. The light skin is chosen by configuration only: a copy
// of the content folder whose slot file names it, or the skin named in the status body.
//
// The live and reconnect states play a recording, which has no matchday: the list reads "No
// other matches this matchday." The kick-off state is served, so it lists the matchday's
// fixtures before kick-off.
//
// Also here: a Tab walk that never lands in a stub, and the rendered contrast check.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import { VIEWER, contentWithSkin, fastForward, killTree, runEngine, startEngine, tempDir, waitForGrounds } from '../support/engine.mjs';
import { kickOffFromPage } from '../support/page.mjs';

const SKINS = ['broadcast-blue', 'interim-light'];
/// Minute 30 at 50 ticks a second.
const MINUTE_30 = 90_000;
/// The error board's stoppage: 52:10.
const STOPPAGE_52_10 = 156_500;
const BOARD_PATH = 'C:\\Games\\Touchline\\engine-cli.exe';

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR
    ? path.join(process.env.MATCH_EVIDENCE_DIR, name)
    : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

let fixture;
const lightContent = {};

test.beforeAll(() => {
  const dir = tempDir('match-fixture');
  fixture = path.join(dir, 'match.smfx');
  const run = runEngine(['record', '--seed', '7', '--minutes', '90', '--out', fixture], { dataDir: dir });
  expect(run.code, run.stderr).toBe(0);
  lightContent.dir = contentWithSkin('interim-light');
});

/// Answers `/engine.json` with `body` (or with the engine's own answer when `body` is null),
/// naming `skin` for the viewer when one is given.
async function routeStatus(page, { body = null, skin = null } = {}) {
  await page.route('**/engine.json', async (route) => {
    let json = body;
    if (json === null) {
      const response = await route.fetch();
      json = await response.json();
    }
    await route.fulfill({ json: skin ? { ...json, 'viewer.skin': skin } : json });
  });
}

/// Opens the viewer and waits until its skin and fonts are in.
async function open(page, url, skin) {
  await page.goto(url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', skin, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

const hook = (page, fn, arg) => page.evaluate(fn, arg);
const until = (page, fn, arg, timeout = 120_000) =>
  page.waitForFunction(fn, arg, { timeout, polling: 100 });

/// The live set-up: a recording sent at once up to minute 30 and at 50 times speed from there,
/// held at minute 30 with playback paused, as a manager reads a match. A rewind draws the
/// stored tick itself.
async function liveAtMinute30(page, skin) {
  const engine = await startEngine({
    command: 'replay',
    args: ['--fixture', fixture, '--web', VIEWER, '--speed', '50', ...fastForward(MINUTE_30)],
  });
  if (skin !== 'broadcast-blue') {
    await routeStatus(page, { skin });
  }
  await open(page, engine.url, skin);
  await until(page, (t) => window.__touchline.history().newest_tick >= t, MINUTE_30);
  await page.getByRole('button', { name: 'Pause', exact: true }).first().click();
  await expect(page.locator('header button.cont')).toHaveText('Resume');
  await page.getByRole('slider', { name: 'Rewind to a tick' }).evaluate((scrub, value) => {
    scrub.value = String(value);
    scrub.dispatchEvent(new Event('input', { bubbles: true }));
    scrub.dispatchEvent(new Event('change', { bubbles: true }));
  }, MINUTE_30);
  await until(page, (t) => window.__touchline.lastRenderedTick() === t, MINUTE_30, 10_000);
  const rewind = await hook(page, () => window.__touchline.lastRewind());
  expect(rewind.exact, 'the rewind drew the stored tick').toBe(true);
  return engine;
}

/// Serves the viewer from a recording whose socket the page never needs: the status route
/// decides what the page shows.
async function served(page, skin, body) {
  const engine = await startEngine({
    command: 'replay',
    args: ['--fixture', fixture, '--web', VIEWER, '--speed', '1'],
  });
  await routeStatus(page, { body, skin: skin === 'broadcast-blue' ? null : skin });
  await open(page, engine.url, skin);
  return engine;
}

for (const skin of SKINS) {
  test.describe(`the match screen in ${skin}`, () => {
    test('live: paused at minute 30', async ({ page }) => {
      const engine = await liveAtMinute30(page, skin);
      try {
        await expect(page.locator('header')).toContainText('PAUSED');
        await expect(page).toHaveScreenshot(`match-live-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('kick-off: the hello, before the kick-off', async ({ page }) => {
      const engine = await startEngine({
        args: ['--seed', '7', '--web', VIEWER],
        env: skin === 'broadcast-blue' ? {} : { SM_CONTENT_DIR: lightContent.dir },
      });
      try {
        await open(page, engine.url, skin);
        await until(page, () => window.__touchline.screen() === 'kickoff');
        // Before kick-off the Tactics view opens first; the board is the Match view.
        await page.locator('nav.subnav').getByRole('button', { name: 'Match' }).click();
        await expect(page.locator('header button.cont')).toHaveText('Kick off');
        // The matchday's fixtures at 0-0 before kick-off, so the list is complete.
        await waitForGrounds(page, 0);
        await expect(page).toHaveScreenshot(`match-kickoff-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('loading: the engine is starting', async ({ page }) => {
      const engine = await served(page, skin, { 'engine.state': 'starting', launcher: true });
      try {
        await expect(page.locator('header button.cont')).toHaveText('Please wait');
        await expect(page.getByRole('heading', { name: 'Getting the match ready' })).toBeVisible();
        await expect(page).toHaveScreenshot(`match-loading-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('error: the engine stopped at 52:10', async ({ page }) => {
      const engine = await liveAtMinute30(page, skin);
      try {
        await routeStatus(page, {
          body: {
            'engine.state': 'crashed',
            'engine.code': 3,
            'snapshot.tick': STOPPAGE_52_10,
            launcher: true,
          },
          skin: skin === 'broadcast-blue' ? null : skin,
        });
        engine.kill();
        await until(page, () => window.__touchline.recovery().kind === 'crashed', undefined, 30_000);
        await expect(page.getByRole('button', { name: 'Restart from 52:10' })).toBeVisible();
        await expect(page).toHaveScreenshot(`match-error-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('first run: no engine found', async ({ page }) => {
      const engine = await served(page, skin, {
        'engine.state': 'not-found',
        'engine.path': BOARD_PATH,
        launcher: true,
      });
      try {
        await until(page, () => window.__touchline.screen() === 'first-run', undefined, 30_000);
        await expect(page.locator('.cover code')).toHaveText(BOARD_PATH);
        await expect(page).toHaveScreenshot(`match-first-run-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('reconnect: the connection dropped', async ({ page }) => {
      // The engine goes away while its status still says starting, so the page holds its
      // last frame and keeps reconnecting for the screenshot. The drive below shows a real
      // drop reconnecting by itself.
      const engine = await liveAtMinute30(page, skin);
      try {
        await routeStatus(page, {
          body: { 'engine.state': 'starting', launcher: false },
          skin: skin === 'broadcast-blue' ? null : skin,
        });
        engine.kill();
        await until(page, () => window.__touchline.screen() === 'reconnecting', undefined, 30_000);
        await expect(page.locator('.held')).toContainText('RECONNECTING');
        await expect(page).toHaveScreenshot(`match-reconnect-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });
  });
}

test.describe('the match screen with a real engine', () => {
  test('a killed engine shows the error panel, and Restart resumes from the stoppage', async ({ page }) => {
    test.setTimeout(8 * 60_000);
    const launcher = await startEngine({
      command: 'launch',
      args: ['--no-start-screen', '--seed', '3', '--minutes', '20', '--web', VIEWER],
    });
    const status = () => fetch(`${launcher.url}engine.json`).then((r) => r.json());
    try {
      await open(page, launcher.url, 'broadcast-blue');
      await until(page, () => window.__touchline.screen() === 'kickoff', undefined, 30_000);
      await kickOffFromPage(page);
      await page.getByRole('group', { name: 'Playback' }).getByRole('button', { name: '8x', exact: true }).click();
      let before;
      for (;;) {
        before = await status();
        const snap = before['snapshot.tick'] ?? 0;
        const drawn = await hook(page, () => window.__touchline.lastRenderedTick());
        if (snap > 1 && drawn > snap + 250) {
          break;
        }
        await page.waitForTimeout(200);
      }
      killTree(before['engine.pid']);
      await until(page, () => window.__touchline.recovery().kind === 'crashed', undefined, 30_000);
      await expect(page.getByRole('alert')).toContainText('The engine stopped');
      await expect(page.locator('header button.cont')).toHaveText('Restart');
      await expect(page.getByRole('button', { name: 'Abandon' })).toBeVisible();
      await page.screenshot({ path: evidence(test.info(), 'drive-error-trigger.png') });
      await page.locator('header button.cont').click();
      await until(page, () => window.__touchline.signals().some((s) => s.signal === 'viewer.resumed'), undefined, 90_000);
      const resumed = await hook(page, () => window.__touchline.signals().find((s) => s.signal === 'viewer.resumed'));
      expect(resumed.to_tick).toBeLessThanOrEqual(before['snapshot.tick'] + 1);
      await until(page, () => window.__touchline.screen() === 'live', undefined, 30_000);
      const a = await hook(page, () => window.__touchline.lastRenderedTick());
      await page.waitForTimeout(2000);
      expect(await hook(page, () => window.__touchline.lastRenderedTick())).toBeGreaterThan(a);
      await page.screenshot({ path: evidence(test.info(), 'drive-error-settled.png') });
    } finally {
      launcher.cleanUp();
    }
  });

  test('a missing engine shows the real path it looked for and Open replay', async ({ page }) => {
    const missing = path.join(tempDir('missing'), 'engine-cli.exe');
    const launcher = await startEngine({
      command: 'launch',
      args: ['--no-start-screen', '--seed', '3', '--web', VIEWER, '--engine', missing],
    });
    try {
      await open(page, launcher.url, 'broadcast-blue');
      await until(page, () => window.__touchline.recovery().kind === 'first-run', undefined, 30_000);
      await expect(page.locator('.cover code')).toHaveText(missing);
      await expect(page.locator('header button.cont')).toHaveText('Open replay');
      await expect(page.getByRole('button', { name: 'Open a replay' })).toBeVisible();
      await page.screenshot({ path: evidence(test.info(), 'drive-first-run.png') });
    } finally {
      launcher.cleanUp();
    }
  });

  test('a dropped connection reconnects by itself, with no restart prompt', async ({ page }) => {
    test.setTimeout(6 * 60_000);
    const launcher = await startEngine({
      command: 'launch',
      args: ['--no-start-screen', '--seed', '42', '--minutes', '20', '--web', VIEWER, '--drop-client-at', '3000'],
    });
    try {
      await open(page, launcher.url, 'broadcast-blue');
      await until(page, () => window.__touchline.screen() === 'kickoff', undefined, 30_000);
      await kickOffFromPage(page);
      await page.getByRole('group', { name: 'Playback' }).getByRole('button', { name: '8x', exact: true }).click();
      let restartSeen = false;
      let reconnectingSeen = false;
      let resumed = null;
      const end = Date.now() + 180_000;
      while (!resumed && Date.now() < end) {
        const state = await hook(page, () => ({
          screen: window.__touchline.screen(),
          kind: window.__touchline.recovery().kind,
          resumed: window.__touchline.signals().find((s) => s.signal === 'viewer.resumed') ?? null,
        }));
        restartSeen ||= state.kind === 'crashed' || state.kind === 'refused';
        if (state.screen === 'reconnecting' && !reconnectingSeen) {
          reconnectingSeen = true;
          await page.screenshot({ path: evidence(test.info(), 'drive-reconnect-trigger.png') });
        }
        resumed = state.resumed;
        await page.waitForTimeout(100);
      }
      expect(resumed, 'the page resumed after the drop').not.toBeNull();
      expect(restartSeen).toBe(false);
      const a = await hook(page, () => window.__touchline.lastRenderedTick());
      await page.waitForTimeout(3000);
      expect(await hook(page, () => window.__touchline.lastRenderedTick())).toBeGreaterThan(a);
      await page.screenshot({ path: evidence(test.info(), 'drive-reconnect-settled.png') });
    } finally {
      launcher.cleanUp();
    }
  });
});

test('a Tab walk through the live match screen never lands in a stub, and the action block leads the header', async ({
  page,
}, info) => {
  const engine = await liveAtMinute30(page, 'broadcast-blue');
  try {
    await page.locator('body').click({ position: { x: 1270, y: 790 } });
    const walk = [];
    for (let press = 1; press <= 40; press += 1) {
      await page.keyboard.press('Tab');
      walk.push(
        await page.evaluate((n) => {
          const el = document.activeElement;
          return {
            press: n,
            tag: el?.tagName.toLowerCase() ?? null,
            name: (el?.getAttribute('aria-label') ?? el?.textContent?.trim() ?? '').slice(0, 60),
            stub: el?.closest('[data-stub]')?.dataset.stub ?? null,
            inert: Boolean(el?.closest('[inert]')),
            header: Boolean(el?.closest('header')),
          };
        }, press)
      );
    }
    fs.writeFileSync(evidence(info, 'match-tab-walk.json'), `${JSON.stringify(walk, null, 2)}\n`);
    for (const step of walk) {
      expect(step.stub, `press ${step.press}`).toBeNull();
      expect(step.inert, `press ${step.press}`).toBe(false);
    }
    const firstInHeader = walk.find((s) => s.header);
    expect(firstInHeader, 'a focus stop in the header').toBeTruthy();
    expect(`${firstInHeader.tag}:${firstInHeader.name}`).toBe('button:Resume');
    const names = walk.map((s) => s.name);
    for (const name of ['Previous stop', 'Play', 'Back to live', 'Next stop', '1x', '8x', 'Rewind to a tick']) {
      expect(names, name).toContain(name);
    }
  } finally {
    engine.cleanUp();
  }
});

test('the live match screen meets WCAG AA contrast on its rendered text and controls', async ({ page }, info) => {
  const engine = await liveAtMinute30(page, 'broadcast-blue');
  try {
    const report = await contrastReport(page);
    fs.writeFileSync(evidence(info, 'contrast-report.json'), `${JSON.stringify(report, null, 2)}\n`);
    expect(report.checked, 'text and controls were measured').toBeGreaterThan(50);
    expect(report.failures, JSON.stringify(report.failures, null, 2)).toEqual([]);

    // Control: one live label in the stub-only grey must fail the same check.
    await page.locator('.commentary li span').first().evaluate((el) => {
      el.style.color = 'var(--ink-4)';
    });
    const planted = await contrastReport(page);
    fs.writeFileSync(evidence(info, 'contrast-control.json'), `${JSON.stringify(planted, null, 2)}\n`);
    expect(planted.failures.length, 'the planted --ink-4 label fails').toBeGreaterThan(0);
  } finally {
    engine.cleanUp();
  }
});
