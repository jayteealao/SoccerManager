// The front door on the built viewer, served by the release program's `launch`, which opens
// on the start screen.
//
// Screenshots: thirteen states in each skin at 1280 by 800 against their baselines. The
// splash, the start screen (with and without a save), match setup (a valid pick and the same
// team twice), the settings and the licences come from fixed `/engine.json`, round and notices
// bodies (fixtures/front-door-states.json), so a dependency update moves no pixel. The menu
// and the return and quit confirmations sit over a real match of seed 7 held at 30:00; the
// closed page comes from a fixed quit answer.
//
// Drives with a real launcher: the splash's timing on the page's own clock (page.clock), a
// key and a click, reduced motion; the start screen with an empty data folder and with a
// save; match setup to kick-off and the same-team refusal; the settings kept across a
// relaunch and used by the next match; Return to start and Resume to full time with the score
// of the match played through; Quit, the closed page and both processes gone; the licences
// screen against the notices file the build wrote. Also a Tab walk and the rendered contrast.
// No drive waits for real-time play: every match is fast-forwarded.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import {
  VIEWER,
  contentWithSkin,
  frontDoor,
  processAlive,
  savedSnapshots,
  tempDir,
  waitForGrounds,
} from '../support/engine.mjs';
import { playUntil } from '../support/page.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const STATES = JSON.parse(fs.readFileSync(path.join(HERE, 'fixtures', 'front-door-states.json'), 'utf8'));
const SKINS = ['broadcast-blue', 'interim-light'];
/// Minute 30 at 50 ticks a second.
const MINUTE_30 = 90_000;
/// Past the end of any match: a match with this fast-forward plays straight through.
const PAST_THE_END = 1_000_000;
/// The splash's timings (viewer/src/lib/front-door.svelte.js).
const REVEAL_MS = 1800;
const FLOOR_MS = 1500;
const READY_HOLD_MS = 1500;
const T0 = new Date('2026-10-01T12:00:00Z');

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR
    ? path.join(process.env.MATCH_EVIDENCE_DIR, name)
    : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

const hook = (page, fn, arg) => page.evaluate(fn, arg);
const until = (page, fn, arg, timeout = 120_000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });
const door = (page) => hook(page, () => window.__touchline.frontDoor());
const lightContent = {};

test.beforeAll(() => {
  lightContent.dir = contentWithSkin('interim-light');
});

/// Answers the page's reads from the fixed bodies: `/engine.json` with `status`, the round
/// preview, the notices file, and quit with the fixed answer. `skin` names the skin.
async function routeFixed(page, { status = STATES.idle, skin = 'broadcast-blue', hold = null } = {}) {
  await page.route('**/engine.json', async (route) => {
    if (hold) {
      await hold;
    }
    await route.fulfill({ json: { ...status, 'viewer.skin': skin } });
  });
  await page.route('**/engine/round?*', (route) => route.fulfill({ json: STATES.round }));
  await page.route('**/notices.json', (route) => route.fulfill({ json: STATES.notices }));
  await page.route('**/engine/quit', (route) => route.fulfill({ json: { ...STATES.quit, 'viewer.skin': skin } }));
}

/// Opens the page and waits until its skin and fonts are in.
async function open(page, url, skin = 'broadcast-blue') {
  await page.goto(url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', skin, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

/// Opens the page on the fixed bodies and goes past the splash to the start screen.
async function startScreen(page, engine, skin, status = STATES.idle) {
  await routeFixed(page, { status, skin });
  await open(page, engine.url, skin);
  await until(page, () => window.__touchline.frontDoor().splash !== null && window.__touchline.frontDoor().answered);
  await page.keyboard.press('Enter');
  await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
}

/// From the start screen through match setup to a live match.
async function kickOffFromStart(page) {
  await page.locator('[data-choice="new"]').click();
  await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
  await page.locator('[data-kickoff]').click();
  await until(page, () => window.__touchline.screen() === 'kickoff', undefined, 30_000);
  const action = page.locator('header button.cont:visible');
  if ((await hook(page, () => window.__touchline.view())) === 'tactics') {
    await expect(action).toHaveText('Continue');
    await action.click();
    await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
  }
  await expect(action).toHaveText('Kick off');
  await expect(action).toBeEnabled();
  await action.click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
}

/// Waits until every finite animation on the page has finished, so an overlay is captured
/// settled rather than part way through its fade.
async function settled(page) {
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((a) => a.effect?.getComputedTiming().endTime !== Infinity)
        .map((a) => a.finished.catch(() => null))
    )
  );
}

/// A real match of seed 7 from the start screen, held paused at 30:00 with the other grounds
/// settled, as the menu's boards draw it.
async function matchAt30(page, skin) {
  const engine = await frontDoor({
    args: ['--seed', '7'],
    fastForwardTo: MINUTE_30,
    env: skin === 'broadcast-blue' ? {} : { SM_CONTENT_DIR: lightContent.dir },
  });
  await open(page, engine.url, skin);
  await until(page, () => window.__touchline.frontDoor().answered);
  await page.keyboard.press('Enter');
  await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
  await kickOffFromStart(page);
  await until(page, (t) => window.__touchline.history().newest_tick >= t, MINUTE_30, 180_000);
  await page.getByRole('button', { name: 'Pause', exact: true }).first().click();
  await expect(page.locator('header button.cont:visible')).toHaveText('Resume');
  await page.getByRole('slider', { name: 'Rewind to a tick' }).filter({ visible: true }).evaluate((scrub, value) => {
    scrub.value = String(value);
    scrub.dispatchEvent(new Event('input', { bubbles: true }));
    scrub.dispatchEvent(new Event('change', { bubbles: true }));
  }, MINUTE_30);
  await until(page, (t) => window.__touchline.lastRenderedTick() === t, MINUTE_30, 10_000);
  await waitForGrounds(page, MINUTE_30);
  // The confirmations name the stoppage the launcher saved; hold it at 30:00 for the board.
  await page.route('**/engine.json', async (route) => {
    const response = await route.fetch();
    await route.fulfill({ json: { ...(await response.json()), 'snapshot.tick': MINUTE_30 } });
  });
  await page.route('**/engine/quit', (route) => route.fulfill({ json: { ...STATES.quit, 'viewer.skin': skin } }));
  return engine;
}

for (const skin of SKINS) {
  test.describe(`the front door in ${skin}`, () => {
    test('splash: the engine is starting', async ({ page }) => {
      const engine = await frontDoor();
      try {
        // The answer never comes during the screenshot.
        await routeFixed(page, { skin, hold: new Promise(() => {}) });
        await page.goto(engine.url);
        await expect(page.locator('[data-screen="splash"]')).toBeVisible();
        await page.evaluate(() => document.fonts.ready);
        // The splash is drawn in the default look until the engine names the skin.
        await expect(page).toHaveScreenshot(`front-splash-starting-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('splash: ready', async ({ page }) => {
      const engine = await frontDoor();
      try {
        await page.clock.install({ time: T0 });
        // Paused before the page loads, so no timer runs until the drive moves the clock.
        await page.clock.pauseAt(new Date(T0.getTime() + 100));
        await routeFixed(page, { skin });
        await page.goto(engine.url);
        await until(page, () => window.__touchline?.frontDoor()?.answered === true, undefined, 30_000);
        await page.clock.runFor(REVEAL_MS);
        await until(page, () => window.__touchline.frontDoor().splash === 'ready', undefined, 10_000);
        await page.evaluate(() => document.fonts.ready);
        await expect(page.getByText('Press any key or click to continue')).toBeVisible();
        await expect(page).toHaveScreenshot(`front-splash-ready-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('start screen: no saved match', async ({ page }) => {
      const engine = await frontDoor();
      try {
        await startScreen(page, engine, skin);
        await expect(page.locator('[data-choice="resume"]')).toBeDisabled();
        await expect(page).toHaveScreenshot(`front-start-empty-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('start screen: a saved match', async ({ page }) => {
      const engine = await frontDoor();
      try {
        await startScreen(page, engine, skin, { ...STATES.idle, saved: STATES.saved });
        await expect(page.locator('[data-choice="resume"]')).toBeEnabled();
        await expect(page.locator('svg.pitch circle.marker')).toHaveCount(22);
        await expect(page).toHaveScreenshot(`front-start-saved-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('match setup: a valid pick', async ({ page }) => {
      const engine = await frontDoor();
      try {
        await startScreen(page, engine, skin);
        await page.locator('[data-choice="new"]').click();
        await expect(page.locator('[data-round="4"]')).toBeVisible();
        await expect(page).toHaveScreenshot(`front-setup-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('match setup: the same team twice', async ({ page }) => {
      const engine = await frontDoor();
      try {
        await startScreen(page, engine, skin);
        await page.locator('[data-choice="new"]').click();
        await page.locator('[data-column="away"] [data-club="club-00000001-00"] button').click();
        await expect(page.getByRole('alert')).toContainText('Same team');
        await page.mouse.move(1270, 790);
        await expect(page).toHaveScreenshot(`front-setup-same-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('settings', async ({ page }) => {
      const engine = await frontDoor();
      try {
        await startScreen(page, engine, skin);
        await page.locator('[data-choice="settings"]').click();
        await expect(page.locator('[data-screen="settings"]')).toBeVisible();
        await expect(page).toHaveScreenshot(`front-settings-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('licences and about', async ({ page }) => {
      const engine = await frontDoor();
      try {
        await startScreen(page, engine, skin);
        await page.locator('[data-choice="licences"]').click();
        await expect(page.locator('[data-package]')).toHaveCount(STATES.notices.packages.length);
        await expect(page).toHaveScreenshot(`front-licences-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('the in-match menu, and the return and quit confirmations', async ({ page }) => {
      test.setTimeout(6 * 60_000);
      const engine = await matchAt30(page, skin);
      try {
        await page.keyboard.press('Escape');
        await until(page, () => window.__touchline.frontDoor().overlay === 'menu', undefined, 5_000);
        await expect(page.getByRole('menu', { name: 'Match menu' })).toContainText('Match paused · 30:00');
        await settled(page);
        await expect(page).toHaveScreenshot(`front-menu-${skin}.png`);

        await page.locator('[data-item="return"]').click();
        await expect(page.getByRole('dialog', { name: 'Return to the start screen?' })).toContainText(
          'The match saves at 30:00'
        );
        await settled(page);
        await expect(page).toHaveScreenshot(`front-return-${skin}.png`);
        await page.getByRole('button', { name: 'Keep playing' }).click();

        await page.keyboard.press('Escape');
        await page.locator('[data-item="quit"]').click();
        await expect(page.getByRole('dialog', { name: 'Quit Touchline?' })).toContainText('The match saves at 30:00');
        await settled(page);
        await expect(page).toHaveScreenshot(`front-quit-${skin}.png`);

        // The quit answer is the fixed one, so the closed page names the board's save.
        await page.getByRole('button', { name: 'Save and quit' }).click();
        await expect(page.getByRole('heading', { name: 'Touchline has closed' })).toBeVisible();
        await expect(page).toHaveScreenshot(`front-closed-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });
  });
}

test.describe('the splash on its own clock', () => {
  test('it waits for the answer, holds the ready state 1.5 s, and never leaves before 1.5 s', async ({ page }, info) => {
    const engine = await frontDoor();
    try {
      let answer;
      const held = new Promise((resolve) => {
        answer = resolve;
      });
      await page.clock.install({ time: T0 });
      // Paused before the page loads, so no timer runs until the drive moves the clock.
      await page.clock.pauseAt(new Date(T0.getTime() + 100));
      await routeFixed(page, { hold: held });
      await page.goto(engine.url);
      await expect(page.getByRole('heading', { name: 'Touchline' })).toBeVisible();
      await expect(page.getByText('[STUDIO MARK]')).toBeVisible();
      await expect(page.getByText(/Powered by the Touchline match engine/)).toBeVisible();
      await page.screenshot({ path: evidence(info, 'splash-held.png') });
      // Three seconds with no answer: still the splash.
      await page.clock.runFor(3000);
      expect((await door(page)).view).toBe('splash');
      answer();
      await until(page, () => window.__touchline.frontDoor().answered === true, undefined, 10_000);
      await page.clock.runFor(READY_HOLD_MS - 10);
      expect((await door(page)).view, 'the ready state holds').toBe('splash');
      await page.clock.runFor(20);
      await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 5_000);
    } finally {
      engine.cleanUp();
    }
  });

  test('an answer at once still keeps the splash past 1.5 s', async ({ page }) => {
    const engine = await frontDoor();
    try {
      await page.clock.install({ time: T0 });
      // Paused before the page loads, so no timer runs until the drive moves the clock.
      await page.clock.pauseAt(new Date(T0.getTime() + 100));
      await routeFixed(page);
      await page.goto(engine.url);
      await until(page, () => window.__touchline?.frontDoor()?.answered === true, undefined, 30_000);
      await page.clock.runFor(FLOOR_MS);
      expect((await door(page)).view).toBe('splash');
      await page.clock.runFor(REVEAL_MS - FLOOR_MS + READY_HOLD_MS + 50);
      await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 5_000);
    } finally {
      engine.cleanUp();
    }
  });

  test('a key before the answer finishes the reveal only; a key or a click after it opens the start screen', async ({
    page,
  }) => {
    const engine = await frontDoor();
    try {
      let answer;
      const held = new Promise((resolve) => {
        answer = resolve;
      });
      await page.clock.install({ time: T0 });
      // Paused before the page loads, so no timer runs until the drive moves the clock.
      await page.clock.pauseAt(new Date(T0.getTime() + 100));
      await routeFixed(page, { hold: held });
      await page.goto(engine.url);
      await expect(page.locator('[data-screen="splash"]')).toBeVisible();
      await page.keyboard.press('Space');
      await expect(page.locator('[data-reveal="finished"]')).toBeVisible();
      expect((await door(page)).view).toBe('splash');
      answer();
      await until(page, () => window.__touchline.frontDoor().answered === true, undefined, 10_000);
      await page.mouse.click(640, 400);
      await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 1_000);
    } finally {
      engine.cleanUp();
    }
  });

  test('with reduced motion the splash does not animate', async ({ page }) => {
    const engine = await frontDoor();
    try {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      await routeFixed(page, { hold: new Promise(() => {}) });
      await page.goto(engine.url);
      await expect(page.locator('[data-screen="splash"]')).toBeVisible();
      const names = await page.evaluate(() =>
        ['.name', '.sub', '.block', '.tile', '.line', '.ball', '.after'].map((sel) => {
          const el = document.querySelector(`[data-screen="splash"] ${sel}`);
          return el ? getComputedStyle(el).animationName : 'none';
        })
      );
      expect(names.every((n) => n === 'none'), names.join(',')).toBe(true);
    } finally {
      engine.cleanUp();
    }
  });

  test('the Reduce setting stops the reveal on a system that asks for full motion', async ({ page }) => {
    const engine = await frontDoor();
    try {
      await routeFixed(page, { status: { ...STATES.idle, settings: { schema_version: 1, speed: 1, motion: 'reduce', commentary: true } } });
      await page.goto(engine.url);
      await until(page, () => window.__touchline?.frontDoor()?.answered === true, undefined, 30_000);
      await expect(page.locator('html')).toHaveAttribute('data-motion', 'reduce');
      const name = await page.evaluate(() => getComputedStyle(document.querySelector('[data-screen] .name') ?? document.body).animationName);
      expect(name).toBe('none');
    } finally {
      engine.cleanUp();
    }
  });
});

test.describe('the front door with a real launcher', () => {
  test('the start screen: Resume disabled in an empty folder, live with a save, and a Tab walk with no stub', async ({
    page,
  }, info) => {
    test.setTimeout(6 * 60_000);
    const data = tempDir('front-save');
    let engine = await frontDoor({ dataDir: data, args: ['--seed', '7'], fastForwardTo: 60_000 });
    try {
      await open(page, engine.url);
      await until(page, () => window.__touchline.frontDoor().answered);
      await page.keyboard.press('Enter');
      await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
      await expect(page.locator('[data-choice]')).toHaveText([
        /New match/,
        /Resume/,
        /Replays/,
        /Settings/,
        /Licences and about/,
        /Quit/,
      ]);
      await expect(page.locator('[data-choice="resume"]')).toBeDisabled();
      await expect(page.locator('[data-choice="resume"]')).toContainText('No saved match yet');

      // The Tab walk: no stop inside a stub, and every live choice is reached.
      await page.locator('body').click({ position: { x: 1270, y: 790 } });
      const walk = [];
      for (let press = 1; press <= 30; press += 1) {
        await page.keyboard.press('Tab');
        walk.push(
          await page.evaluate(() => {
            const el = document.activeElement;
            return {
              choice: el?.dataset?.choice ?? null,
              name: (el?.getAttribute('aria-label') ?? el?.textContent?.trim() ?? '').slice(0, 40),
              stub: el?.closest('[data-stub]')?.dataset.stub ?? null,
              inert: Boolean(el?.closest('[inert]')),
            };
          })
        );
      }
      fs.writeFileSync(evidence(info, 'start-tab-walk.json'), `${JSON.stringify(walk, null, 2)}\n`);
      expect(walk.filter((w) => w.stub || w.inert)).toEqual([]);
      const reached = new Set(walk.map((w) => w.choice).filter(Boolean));
      for (const choice of ['new', 'replays', 'settings', 'licences', 'quit']) {
        expect(reached.has(choice), `Tab reaches ${choice}`).toBe(true);
      }
      expect(reached.has('resume'), 'the disabled Resume takes no focus').toBe(false);

      // A save: kick off, go back to the start screen, and the launcher keeps the match.
      await kickOffFromStart(page);
      await until(page, () => window.__touchline.history().newest_tick >= 20_000, undefined, 120_000);
      await page.keyboard.press('Escape');
      await page.locator('[data-item="return"]').click();
      await page.getByRole('button', { name: 'Save and leave' }).click();
      await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 15_000);
      await expect(page.locator('[data-choice="resume"]')).toBeEnabled();
      await expect(page.locator('svg.pitch circle.marker')).toHaveCount(22);
      await page.screenshot({ path: evidence(info, 'start-with-save.png') });
      expect(savedSnapshots(data).length).toBeGreaterThan(0);
    } finally {
      engine.cleanUp();
    }
  });

  test('match setup kicks off the pair on the home ground with the previewed round; the same team is refused', async ({
    page,
  }, info) => {
    test.setTimeout(4 * 60_000);
    const engine = await frontDoor({ args: ['--seed', '7'], fastForwardTo: 3_000 });
    try {
      await open(page, engine.url);
      await until(page, () => window.__touchline.frontDoor().answered);
      await page.keyboard.press('Enter');
      await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
      await page.locator('[data-choice="new"]').click();
      await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);

      await page.locator('[data-column="away"] [data-club="club-00000001-00"] button').click();
      await expect(page.getByRole('alert')).toContainText('A team cannot play itself. Pick a different away team.');
      await expect(page.locator('[data-kickoff]')).toBeDisabled();
      await page.screenshot({ path: evidence(info, 'setup-same-team.png') });
      await page.locator('[data-column="away"] [data-club="club-00000002-00"] button').click();
      await until(page, () => window.__touchline.frontDoor().round !== null, undefined, 10_000);
      const preview = (await door(page)).round;
      expect(preview).toHaveLength(4);

      await page.locator('[data-kickoff]').click();
      await until(page, () => ['tactics', 'match'].includes(window.__touchline.view()), undefined, 30_000);
      // The hello names the two clubs picked.
      await until(page, () => window.__touchline.screen() === 'kickoff', undefined, 30_000);
      const header = await page.locator('header:visible').first().textContent();
      expect(header).toMatch(/Oakmere Rangers/i);
      expect(header).toMatch(/Eldstead City/i);
      await until(page, () => window.__touchline.matchday().fixtures.length === 4, undefined, 30_000);
      const fixtures = await hook(page, () => window.__touchline.matchday().fixtures.map((f) => [f.home, f.away]));
      expect(fixtures).toEqual(preview);
      await page.screenshot({ path: evidence(info, 'setup-kicked-off.png') });
    } finally {
      engine.cleanUp();
    }
  });

  test('the settings hold across a relaunch, and the next match uses them', async ({ page }, info) => {
    test.setTimeout(4 * 60_000);
    const data = tempDir('front-settings');
    let engine = await frontDoor({ dataDir: data, args: ['--seed', '7'], fastForwardTo: 3_000 });
    try {
      await open(page, engine.url);
      await until(page, () => window.__touchline.frontDoor().answered);
      await page.keyboard.press('Enter');
      await page.locator('[data-choice="settings"]').click();
      await page.getByRole('group', { name: 'Default speed' }).getByRole('button', { name: '4×' }).click();
      await page.getByRole('group', { name: 'Animation' }).getByRole('button', { name: 'Reduce' }).click();
      await page.getByRole('group', { name: 'Commentary' }).getByRole('button', { name: 'Off' }).click();
      await until(page, () => {
        const s = window.__touchline.frontDoor().settings;
        return s.speed === 4 && s.motion === 'reduce' && s.commentary === false;
      });
      await expect(page.getByText('Saved', { exact: true })).toBeVisible();
      // Quit from the start screen ends the launcher.
      await page.getByRole('button', { name: 'Back to start' }).click();
      await page.locator('[data-choice="quit"]').click();
      await expect(page.getByRole('heading', { name: 'Touchline has closed' })).toBeVisible();
      expect(await engine.exited).toBe(0);

      engine = await frontDoor({ dataDir: data, args: ['--seed', '7'], fastForwardTo: 3_000 });
      await open(page, engine.url);
      await until(page, () => window.__touchline.frontDoor().answered);
      await expect(page.locator('html')).toHaveAttribute('data-motion', 'reduce');
      await page.keyboard.press('Enter');
      await page.locator('[data-choice="settings"]').click();
      await expect(page.getByRole('group', { name: 'Default speed' }).getByRole('button', { pressed: true })).toHaveText('4×');
      await expect(page.getByRole('group', { name: 'Animation' }).getByRole('button', { pressed: true })).toHaveText('Reduce');
      await expect(page.getByRole('group', { name: 'Commentary' }).getByRole('button', { pressed: true })).toHaveText('Off');
      await page.screenshot({ path: evidence(info, 'settings-after-relaunch.png') });

      await page.getByRole('button', { name: 'Back to start' }).click();
      await kickOffFromStart(page);
      expect((await hook(page, () => window.__touchline.notice())).speed).toBe(4);
      await page.locator('nav.subnav:visible').getByRole('button', { name: 'Match', exact: true }).click();
      await expect(page.locator('section.commentary')).toHaveCount(0);
      await expect(page.locator('html')).toHaveAttribute('data-motion', 'reduce');
      await page.screenshot({ path: evidence(info, 'match-with-settings.png') });
    } finally {
      engine.cleanUp();
    }
  });

  test('Return to start keeps the match, and Resume finishes it as the match played through', async ({
    page,
  }, info) => {
    test.setTimeout(10 * 60_000);
    // The engine runs ahead of the page only as far as the fast-forward: a match left before
    // that point is still unfinished, so the launcher keeps it for Resume.
    const finalScore = async ({ leaveAt = null } = {}) => {
      const engine = await frontDoor({
        args: ['--seed', '11', '--minutes', '3'],
        fastForwardTo: leaveAt ?? PAST_THE_END,
      });
      try {
        await open(page, engine.url);
        await until(page, () => window.__touchline.frontDoor().answered);
        await page.keyboard.press('Enter');
        await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 10_000);
        await kickOffFromStart(page);
        if (leaveAt !== null) {
          await until(page, (t) => window.__touchline.history().newest_tick >= t, leaveAt, 120_000);
          await page.keyboard.press('Escape');
          await page.locator('[data-item="return"]').click();
          await page.getByRole('button', { name: 'Save and leave' }).click();
          await until(page, () => window.__touchline.frontDoor().view === 'start', undefined, 15_000);
          await page.screenshot({ path: evidence(info, 'return-to-start.png') });
          await expect(page.locator('[data-choice="resume"]')).toBeEnabled();
          await page.locator('[data-choice="resume"]').click();
          await until(page, () => window.__touchline.frontDoor().view === 'match', undefined, 15_000);
        }
        await page.getByRole('group', { name: 'Playback' }).filter({ visible: true }).getByRole('button', { name: '8x', exact: true }).click();
        await playUntil(page, () => window.__touchline.events().some((e) => e['event.type'] === 'full-time'), { timeout: 5 * 60_000 });
        const fullTime = await hook(page, () => window.__touchline.events().find((e) => e['event.type'] === 'full-time'));
        // Every player and the ball at ticks after the stop, as the page stored them.
        const ticks = [5_000, 6_000, 7_000, 8_000, fullTime.tick];
        const frames = await hook(page, (list) => list.map((t) => window.__touchline.tickAt(t)), ticks);
        return { score: [fullTime['home.score'], fullTime['away.score']], tick: fullTime.tick, ticks, frames };
      } finally {
        engine.cleanUp();
      }
    };
    const through = await finalScore();
    const resumed = await finalScore({ leaveAt: 4_000 });
    fs.writeFileSync(evidence(info, 'resume-scores.json'), `${JSON.stringify({ through, resumed }, null, 2)}\n`);
    expect(resumed.frames.every((frame) => Array.isArray(frame)), 'the page stored each sampled tick').toBe(true);
    // The score, the full-time tick and every sampled tick after the stop are the same.
    expect(resumed).toEqual(through);
  });

  test('Quit asks first, saves the match, ends the engine and the launcher, and shows the closed page', async ({
    page,
  }, info) => {
    test.setTimeout(4 * 60_000);
    const data = tempDir('front-quit');
    const engine = await frontDoor({ dataDir: data, args: ['--seed', '7'], fastForwardTo: 20_000 });
    try {
      await open(page, engine.url);
      await until(page, () => window.__touchline.frontDoor().answered);
      await page.keyboard.press('Enter');
      await kickOffFromStart(page);
      await until(page, () => window.__touchline.history().newest_tick >= 10_000, undefined, 120_000);
      const status = await (await fetch(`${engine.url}engine.json`)).json();
      const worker = status['engine.pid'];
      expect(processAlive(worker)).toBe(true);
      await page.keyboard.press('Escape');
      await page.locator('[data-item="quit"]').click();
      await expect(page.getByRole('dialog', { name: 'Quit Touchline?' })).toBeVisible();
      await page.getByRole('button', { name: 'Save and quit' }).click();
      await expect(page.getByRole('heading', { name: 'Touchline has closed' })).toBeVisible();
      await expect(page.getByText('You can close this tab.')).toBeVisible();
      await expect(page.locator('[data-screen="closed"] li')).toHaveCount(3);
      await page.screenshot({ path: evidence(info, 'closed.png') });
      const code = await Promise.race([engine.exited, new Promise((resolve) => setTimeout(() => resolve('running'), 5_000))]);
      expect(code, 'the launcher exits within 5 s').toBe(0);
      const processes = { launcher: processAlive(engine.process.pid), worker: processAlive(worker) };
      fs.writeFileSync(evidence(info, 'processes.json'), `${JSON.stringify({ worker, ...processes }, null, 2)}\n`);
      expect(processes).toEqual({ launcher: false, worker: false });
      expect(savedSnapshots(data).length).toBeGreaterThan(0);
    } finally {
      engine.cleanUp();
    }
  });

  test('the licences screen lists every package of the notices file with its text', async ({ page }, info) => {
    const engine = await frontDoor();
    try {
      const file = JSON.parse(fs.readFileSync(path.join(VIEWER, 'notices.json'), 'utf8'));
      await open(page, engine.url);
      await until(page, () => window.__touchline.frontDoor().answered);
      await page.keyboard.press('Enter');
      await page.locator('[data-choice="licences"]').click();
      await expect(page.locator('[data-package]')).toHaveCount(file.packages.length, { timeout: 10_000 });
      const shown = await page.locator('[data-package]').evaluateAll((rows) => rows.map((r) => r.dataset.package));
      expect(shown).toEqual(file.packages.map((p) => p.name));
      const checked = [];
      for (const index of [0, Math.floor(file.packages.length / 2), file.packages.length - 1]) {
        await page.locator('[data-package] button').nth(index).click();
        const text = await page.locator('pre.text').textContent();
        expect(text).toBe(file.packages[index].text);
        checked.push(file.packages[index].name);
      }
      await expect(page.locator('[data-screen="licences"]')).toContainText(file.version);
      fs.writeFileSync(
        evidence(info, 'notices-compare.json'),
        `${JSON.stringify({ listed: file.packages.length, shown: shown.length, texts_checked: checked }, null, 2)}\n`
      );
    } finally {
      engine.cleanUp();
    }
  });
});

test('the front door screens pass the rendered contrast check', async ({ page }, info) => {
  const engine = await frontDoor();
  const rows = {};
  try {
    await startScreen(page, engine, 'broadcast-blue', { ...STATES.idle, saved: STATES.saved });
    rows.start = await contrastReport(page);
    await page.locator('[data-choice="new"]').click();
    await expect(page.locator('[data-round="4"]')).toBeVisible();
    rows.setup = await contrastReport(page);
    await page.getByRole('button', { name: 'Back to start' }).click();
    await page.locator('[data-choice="settings"]').click();
    rows.settings = await contrastReport(page);
    await page.getByRole('button', { name: 'Licences and about' }).last().click();
    await expect(page.locator('[data-package]').first()).toBeVisible();
    rows.licences = await contrastReport(page);
    await page.getByRole('button', { name: 'Back to start' }).click();
    await page.locator('[data-choice="quit"]').click();
    await expect(page.getByRole('heading', { name: 'Touchline has closed' })).toBeVisible();
    rows.closed = await contrastReport(page);
  } finally {
    engine.cleanUp();
  }
  fs.writeFileSync(evidence(info, 'front-door-contrast.json'), `${JSON.stringify(rows, null, 2)}\n`);
  for (const [screen, report] of Object.entries(rows)) {
    expect(report.failures, `${screen}: ${JSON.stringify(report.failures)}`).toEqual([]);
  }
});
