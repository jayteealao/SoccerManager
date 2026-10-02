// The other grounds of the matchday on the match screen, served by the release engine with
// `--web viewer/dist`.
//
// Screenshots, against their baselines:
// - the board's seven states of the list (no other matches, not started, a new goal, an event
//   shown late, a result unavailable, full time elsewhere, after a rewind), each in both
//   skins, as a screenshot of the list alone. Each is forced: the engine plays seed 7 with no
//   matchday of its own, the page is held at a fixed tick, and the test sends the ground
//   messages of `fixtures/matchday-states.json` on the page's own socket (through a relay),
//   so the rows are the same every run;
// - the north star, "Match live · goal at another ground", in both skins at 1280 by 800, from
//   a real engine and its real matchday: seed 7 held 100 ticks after the first goal at
//   another ground, played across that goal so the list outlines it.
// The light skin is chosen by configuration only (a content copy's slots.json).
//
// Also here: a Tab walk that never stops inside the list, the rendered contrast check on the
// list's goal and unavailable states, and the outline with no animation under reduced motion.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import { VIEWER, contentWithSkin, fastForward, matchdayArgs, startEngine, waitForGrounds } from '../support/engine.mjs';
import { injectingRelay } from '../support/messages.mjs';
import { playUntil } from '../support/page.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const STATES = JSON.parse(fs.readFileSync(path.join(HERE, 'fixtures', 'matchday-states.json'), 'utf8')).states;
const SKINS = ['broadcast-blue', 'interim-light'];
const SEED = 7;

const content = {};

test.beforeAll(() => {
  content.light = contentWithSkin('interim-light');
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

const action = (page) => page.locator('header button.cont:visible');
const playback = (page) => page.getByRole('group', { name: 'Playback' }).filter({ visible: true });
/// The list on the match screen.
const list = (page) => page.locator('[data-screen] section[aria-label="Other grounds"]').filter({ visible: true });

async function open(page, url, skin) {
  await page.goto(url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', skin, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

async function serve(skin, to, { matchday }) {
  return startEngine({
    args: ['--seed', String(SEED), '--web', VIEWER, ...fastForward(to), ...matchdayArgs({ matchday })],
    env: skin === 'broadcast-blue' ? {} : { SM_CONTENT_DIR: content.light },
  });
}

async function rewindTo(page, tick) {
  await page.getByRole('slider', { name: 'Rewind to a tick' }).filter({ visible: true }).evaluate((scrub, value) => {
    scrub.value = String(value);
    scrub.dispatchEvent(new Event('input', { bubbles: true }));
    scrub.dispatchEvent(new Event('change', { bubbles: true }));
  }, tick);
  await until(page, (t) => window.__touchline.lastRenderedTick() === t, tick, 10_000);
}

/// CONTINUE on Tactics, KICK OFF on the Pre-match line-ups, 8x until `tick` is stored, pause,
/// and a rewind to exactly `tick`.
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

/// Plays on from the paused tick until the drawn tick passes `tick`, pauses, and rewinds back
/// to exactly `tick`: a goal crossed in play keeps its outline through that short rewind.
async function playPast(page, tick) {
  await action(page).click();
  await expect(action(page)).toHaveText('Pause');
  await until(page, (t) => window.__touchline.lastRenderedTick() >= t, tick, 30_000);
  await action(page).click();
  await expect(action(page)).toHaveText('Resume');
  await rewindTo(page, tick);
}

const send = async (route, page, messages) => {
  for (const message of messages) {
    route.send(message);
  }
  const last = messages.at(-1);
  if (last) {
    await until(
      page,
      (m) => {
        const day = window.__touchline.matchday();
        if (m.type === 'matchday') {
          return day.fixtures.length === m.fixtures.length;
        }
        if (m.type === 'ground-progress') {
          return day.reached.every((r, i) => r >= m.reached[i]);
        }
        return day.events[m.fixture]?.some((e) => e.tick === m.tick && e.kind === m.kind);
      },
      last,
      10_000
    );
  }
};

/// The served match held for `state`, with its ground messages sent.
async function forced(page, skin, name) {
  const state = STATES[name];
  const route = await injectingRelay(page);
  const engine = await serve(skin, state.hold + 500, { matchday: false });
  const stop = engine.cleanUp;
  engine.cleanUp = () => {
    stop();
    route.close();
  };
  await open(page, engine.url, skin);
  if (state.hold === 0) {
    await until(page, () => window.__touchline.screen() === 'kickoff', undefined, 30_000);
    await page.locator('nav.subnav:visible').getByRole('button', { name: 'Match', exact: true }).click();
    await expect(action(page)).toHaveText('Kick off');
  } else {
    await kickOffAndPauseAt(page, state.hold);
  }
  await send(route, page, state.messages);
  if (state.then) {
    await send(route, page, state.then);
  }
  if (state.play_to) {
    await playPast(page, state.play_to);
  }
  if (state.rewind_to) {
    await rewindTo(page, state.rewind_to);
  }
  await expect(list(page)).toBeVisible();
  return engine;
}

for (const skin of SKINS) {
  test.describe(`the other grounds in ${skin}`, () => {
    for (const name of Object.keys(STATES)) {
      test(`${STATES[name].words}`, async ({ page }) => {
        test.setTimeout(6 * 60_000);
        const engine = await forced(page, skin, name);
        try {
          const grounds = await hook(page, () => window.__touchline.matchday().grounds);
          if (name === 'goal') {
            expect(grounds.rows[0].flag).toBe('new');
          }
          if (name === 'late') {
            expect(grounds.rows[1].flag).toBe('late');
          }
          if (name === 'rewind') {
            expect(grounds.rows.every((r) => r.flag === null)).toBe(true);
          }
          await expect(list(page)).toHaveScreenshot(`other-grounds-${name}-${skin}.png`);
          await page.screenshot({ path: evidence(test.info(), `other-grounds-${name}-${skin}-screen.png`) });
        } finally {
          engine.cleanUp();
        }
      });
    }

    test('north star: match live with a goal at another ground', async ({ page }, info) => {
      test.setTimeout(8 * 60_000);
      const engine = await serve(skin, 150_000, { matchday: true });
      try {
        await open(page, engine.url, skin);
        await kickOffAndPauseAt(page, 1_000);
        // The first goal at another ground the engine has sent by the fast-forward's end.
        const first = await until(
          page,
          () => {
            const goals = window.__touchline
              .matchday()
              .events.flat()
              .filter((e) => e.kind === 'goal');
            return goals.length ? Math.min(...goals.map((g) => g.tick)) : null;
          },
          undefined,
          60_000
        ).then((h) => h.jsonValue());
        await rewindTo(page, first - 300);
        await playPast(page, first + 100);
        await waitForGrounds(page, first + 100);
        const day = await hook(page, () => window.__touchline.matchday());
        fs.writeFileSync(evidence(info, `north-star-matchday-${skin}.json`), `${JSON.stringify(day, null, 2)}\n`);
        expect(day.fixtures).toHaveLength(4);
        expect(day.grounds.rows.some((r) => r.flag === 'new')).toBe(true);
        await expect(page).toHaveScreenshot(`other-grounds-north-star-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });
  });
}

test('a Tab walk never stops inside the other-grounds list', async ({ page }, info) => {
  test.setTimeout(6 * 60_000);
  const engine = await forced(page, 'broadcast-blue', 'goal');
  try {
    await page.locator('body').click({ position: { x: 1270, y: 790 } });
    const walk = [];
    for (let press = 1; press <= 45; press += 1) {
      await page.keyboard.press('Tab');
      walk.push(
        await page.evaluate((n) => {
          const el = document.activeElement;
          return {
            press: n,
            tag: el?.tagName.toLowerCase() ?? null,
            name: (el?.getAttribute('aria-label') ?? el?.textContent?.trim() ?? '').slice(0, 60),
            inList: Boolean(el?.closest('section[aria-label="Other grounds"]')),
          };
        }, press)
      );
    }
    fs.writeFileSync(evidence(info, 'other-grounds-tab-walk.json'), `${JSON.stringify(walk, null, 2)}\n`);
    for (const step of walk) {
      expect(step.inList, `press ${step.press}`).toBe(false);
    }
  } finally {
    engine.cleanUp();
  }
});

for (const name of ['goal', 'unavailable']) {
  test(`the list meets WCAG AA contrast in the ${name} state`, async ({ page }, info) => {
    test.setTimeout(6 * 60_000);
    const engine = await forced(page, 'broadcast-blue', name);
    try {
      const report = await contrastReport(page);
      const inList = report.failures.filter((f) => /Other grounds|GOAL|unavailable|fault/i.test(JSON.stringify(f)));
      fs.writeFileSync(evidence(info, `other-grounds-contrast-${name}.json`), `${JSON.stringify(report, null, 2)}\n`);
      expect(report.checked).toBeGreaterThan(50);
      expect(report.failures, JSON.stringify(report.failures, null, 2)).toEqual([]);
      expect(inList).toEqual([]);
    } finally {
      engine.cleanUp();
    }
  });
}

test('under reduced motion the new goal has no fade', async ({ page }) => {
  test.setTimeout(6 * 60_000);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const engine = await forced(page, 'broadcast-blue', 'goal');
  try {
    const block = list(page).locator('li.new');
    await expect(block).toHaveCount(1);
    expect(await block.evaluate((el) => getComputedStyle(el).animationName)).toBe('none');
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    expect(await block.evaluate((el) => getComputedStyle(el).animationName)).not.toBe('none');
  } finally {
    engine.cleanUp();
  }
});
