// The Tactics screen on the new viewer, served by the release engine with `--web viewer/dist`.
//
// Before kick-off the action block reads CONTINUE: it opens the Pre-match line-ups, whose
// KICK OFF starts the match, so every drive here kicks off through them.
//
// Screenshots: before kick-off and during play (paused at minute 10 with one substitution and
// one mentality change queued), in each skin at 1280 by 800, against their baselines. The
// light skin is chosen by configuration only: a copy of the content folder whose slot file
// names it.
//
// Message parity: the manager script on the viewer, against seed 42, must send the same
// lineup and change messages the former page sent for the same script. The former page's
// messages were recorded once, on the former page, into `fixtures/messages-web-seed42.json`.
// A control run with one bench place swapped must differ, so the comparison can fail.
//
// Also here: a withdrawn substitution never applies; a Tab walk through both phases never
// lands in a stub; and the rendered contrast check on both phases.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import { VIEWER, contentWithSkin, fastForward, startEngine } from '../support/engine.mjs';
import { managerCommand, recordClientMessages } from '../support/messages.mjs';
import { chooseIndex, clickClear, snap } from '../support/page.mjs';
import { expectRatingsOnTwenty } from '../support/ratings.mjs';

const SKINS = ['broadcast-blue', 'interim-light'];
/// Minute 10 at 50 ticks a second.
const MINUTE_10 = 30_000;
/// Minute 3: far enough into play for a change to wait for a stoppage.
const MINUTE_3 = 9_000;

const evidence = (info, name) => {
  const out = process.env.MATCH_EVIDENCE_DIR
    ? path.join(process.env.MATCH_EVIDENCE_DIR, name)
    : info.outputPath(name);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  return out;
};

const lightContent = {};

test.beforeAll(() => {
  lightContent.dir = contentWithSkin('interim-light');
});

const hook = (page, fn, arg) => page.evaluate(fn, arg);
const until = (page, fn, arg, timeout = 120_000) =>
  page.waitForFunction(fn, arg, { timeout, polling: 100 });

/// The action block of the screen on show (the hidden screen keeps its own).
const action = (page) => page.locator('header button.cont:visible');
const tab = (page, name) =>
  page.locator('nav.subnav:visible').getByRole('button', { name, exact: true });

/// Opens the page and waits until its skin and fonts are in.
async function open(page, url, skin = 'broadcast-blue') {
  await page.goto(url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', skin, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

/// `to`: a late tick the drive plays to; the engine sends every tick up to it at once.
async function serve(seed, skin = 'broadcast-blue', to = undefined) {
  return startEngine({
    args: ['--seed', String(seed), '--web', VIEWER, ...fastForward(to)],
    env: skin === 'broadcast-blue' ? {} : { SM_CONTENT_DIR: lightContent.dir },
  });
}

/// Before kick-off, on the Tactics screen.
async function preMatch(page, engine, skin) {
  await open(page, engine.url, skin);
  await until(page, () => window.__touchline.lineup().phase === 'pre-match', undefined, 30_000);
  await until(page, () => window.__touchline.view() === 'tactics', undefined, 10_000);
}

/// CONTINUE on Tactics opens the Pre-match line-ups, whose action block is KICK OFF.
async function continueToKickOff(page) {
  await expect(action(page)).toHaveText('Continue');
  await action(page).click();
  await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
  await expect(action(page)).toHaveText('Kick off');
}

/// Kicks off through the Pre-match line-ups and plays at 8x until `tick` is stored, then
/// pauses.
async function playTo(page, tick) {
  await continueToKickOff(page);
  await action(page).click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
  await page.getByRole('group', { name: 'Playback' }).getByRole('button', { name: '8x', exact: true }).click();
  await until(page, (t) => window.__touchline.history().newest_tick >= t, tick, 240_000);
  await action(page).click();
  await expect(action(page)).toHaveText('Resume');
}

/// Queues a substitution from the viewer's picker and waits for the engine's acknowledgement.
async function queueSubstitution(page, off, on) {
  const before = await hook(page, () => window.__touchline.pending().length);
  await chooseIndex(page.getByLabel('Coming off'), off);
  await chooseIndex(page.getByLabel('Coming on'), on);
  await page.getByRole('button', { name: 'Queue substitution' }).click();
  await until(page, (n) => window.__touchline.pending().length > n, before, 10_000);
  return hook(page, (n) => window.__touchline.pending()[n], before);
}

/// Moves the viewer's mentality select one step and waits for the engine's acknowledgement.
async function queueMentality(page) {
  const before = await hook(page, () => window.__touchline.pending().length);
  // The select alone: once the change is queued, its Edit and Cancel buttons also name Mentality.
  const select = page.getByRole('combobox', { name: /^Mentality/ });
  const now = await select.evaluate((s) => s.selectedIndex);
  await chooseIndex(select, now > 0 ? now - 1 : now + 1);
  await until(page, (n) => window.__touchline.pending().length > n, before, 10_000);
}

for (const skin of SKINS) {
  test.describe(`the Tactics screen in ${skin}`, () => {
    test('before kick-off: the squad list and the eleven slots', { tag: '@sizes' }, async ({ page }) => {
      const engine = await serve(7, skin);
      try {
        await preMatch(page, engine, skin);
        await expect(page.getByRole('status').filter({ hasText: 'READY' }).first()).toBeVisible();
        await expectRatingsOnTwenty(page);
        await snap(page, `tactics-prematch-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('in match: paused at minute 10 with two changes queued', { tag: '@sizes' }, async ({ page }) => {
      test.setTimeout(8 * 60_000);
      const engine = await serve(7, skin, MINUTE_10);
      try {
        await preMatch(page, engine, skin);
        await playTo(page, MINUTE_10);
        await page.getByRole('slider', { name: 'Rewind to a tick' }).evaluate((scrub, value) => {
          scrub.value = String(value);
          scrub.dispatchEvent(new Event('input', { bubbles: true }));
          scrub.dispatchEvent(new Event('change', { bubbles: true }));
        }, MINUTE_10);
        await until(page, (t) => window.__touchline.lastRenderedTick() === t, MINUTE_10, 10_000);
        await tab(page, 'Tactics').click();
        await queueSubstitution(page, 7, 0);
        await queueMentality(page);
        await expect(page.locator('[data-queue-id]')).toHaveCount(2);
        await snap(page, `tactics-match-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });
  });
}

/// The manager script both pages run: drag the first player not picked onto slot 10, move the mentality
/// one step, kick off, play to minute 3, pause, queue one substitution, and play on until the
/// engine rules on it. `plant` swaps two bench places first (the control run).
async function managerScript(page, { plant = false } = {}) {
  await until(page, () => window.__touchline.lineup().phase === 'pre-match', undefined, 30_000);
  const squadRow = (i) => page.locator(`button[data-squad="${i}"]`);
  const slot = (n) => page.locator(`button[data-slot="${n}"]`);
  const free = await hook(page, () => {
    const { slots, bench } = window.__touchline.lineup();
    const placed = new Set([...slots, ...bench]);
    let i = 0;
    while (placed.has(i)) {
      i += 1;
    }
    return i;
  });
  await squadRow(free).dragTo(slot(9));
  await until(page, (i) => window.__touchline.lineup().slots[9] === i, free, 5_000);
  if (plant) {
    const [a, b] = await hook(page, () => window.__touchline.lineup().bench.slice(0, 2));
    await squadRow(a).click();
    await squadRow(b).click();
  }
  const mentality = page.getByLabel('Mentality');
  const now = await mentality.evaluate((s) => s.selectedIndex);
  await chooseIndex(mentality, now + 1);

  await continueToKickOff(page);
  await action(page).click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
  await page.getByRole('group', { name: 'Playback' }).getByRole('button', { name: '8x', exact: true }).click();
  await until(page, (t) => window.__touchline.lastRenderedTick() > t, MINUTE_3, 120_000);

  const before = await hook(page, () => window.__touchline.pending().length);
  await action(page).click();
  await tab(page, 'Tactics').click();
  await chooseIndex(page.getByLabel('Coming off'), 5);
  await chooseIndex(page.getByLabel('Coming on'), 0);
  await page.getByRole('button', { name: 'Queue substitution' }).click();
  await until(page, (n) => window.__touchline.pending().length > n, before, 10_000);
  const id = await hook(page, (n) => window.__touchline.pending()[n].queue_id, before);
  await action(page).click();
  await until(
    page,
    (q) => ['applied', 'rejected'].includes(window.__touchline.pending().find((c) => c.queue_id === q)?.state),
    id,
    180_000
  );
}

test.describe('the viewer sends the manager messages the former page sent', () => {
  test('the script sends the recorded messages, and a planted change differs', async ({ browser }, info) => {
    test.setTimeout(25 * 60_000);
    const recorded = JSON.parse(
      fs.readFileSync(new URL('./fixtures/messages-web-seed42.json', import.meta.url), 'utf8')
    );
    const run = async (options) => {
      // The project's own window size, as every other test here has.
      const page = await browser.newPage({ viewport: info.project.use.viewport });
      const sent = await recordClientMessages(page, managerCommand);
      const engine = await serve(42, 'broadcast-blue');
      try {
        await page.goto(engine.url);
        await page.waitForFunction(() => Boolean(window.__touchline));
        await managerScript(page, options);
        return sent;
      } finally {
        engine.cleanUp();
        await page.close();
      }
    };
    const viewer = await run();
    fs.writeFileSync(evidence(info, 'messages-viewer.json'), `${JSON.stringify(viewer, null, 2)}\n`);
    expect(recorded.map((m) => m.type)).toEqual(['set-lineup', 'queue-change']);
    expect(recorded[0].patch, 'the mentality rides the lineup').toBeTruthy();
    expect(viewer).toEqual(recorded);

    const planted = await run({ plant: true });
    fs.writeFileSync(evidence(info, 'messages-control.json'), `${JSON.stringify(planted, null, 2)}\n`);
    expect(planted, 'a swapped bench place must show in the messages').not.toEqual(recorded);
  });
});

test('a withdrawn substitution never applies, and the count of substitutions stays', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  const sent = await recordClientMessages(page, managerCommand);
  const engine = await serve(42);
  try {
    await preMatch(page, engine, 'broadcast-blue');
    await playTo(page, MINUTE_3);
    await tab(page, 'Tactics').click();
    const left = await hook(page, () => window.__touchline.dugout().subs_left);
    const chip = await queueSubstitution(page, 5, 0);
    await page.getByRole('button', { name: /^Cancel: Substitution/ }).click();
    await until(
      page,
      (q) => window.__touchline.pending().find((c) => c.queue_id === q)?.cancelled === true,
      chip.queue_id,
      10_000
    );
    await expect(page.locator(`[data-queue-id="${chip.queue_id}"]`)).toHaveCount(0);
    const from = await hook(page, () => window.__touchline.lastRenderedTick());
    const stoppage = await hook(page, (t) => window.__touchline.stoppageAt(t), from);
    await action(page).click();
    await until(page, (t) => window.__touchline.lastRenderedTick() > t + 500, stoppage ?? from + 3_000, 180_000);
    const after = await hook(page, () => ({
      subs_left: window.__touchline.dugout().subs_left,
      chip: window.__touchline.pending(),
      substitutions: window.__touchline.events().filter((e) => e.type === 'substitution'),
    }));
    fs.writeFileSync(
      evidence(info, 'cancel.json'),
      `${JSON.stringify({ sent, queued: chip, stoppage, ...after }, null, 2)}\n`
    );
    expect(sent.map((m) => m.type)).toEqual(['set-lineup', 'queue-change', 'cancel-change']);
    expect(after.subs_left).toBe(left);
    expect(after.chip.find((c) => c.queue_id === chip.queue_id).state).toBe('queued');
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
          name: (el?.getAttribute('aria-label') ?? el?.closest('label')?.textContent ?? el?.textContent ?? '')
            .trim()
            .replace(/\s+/g, ' ')
            .slice(0, 60),
          stub: el?.closest('[data-stub]')?.dataset.stub ?? null,
          inert: Boolean(el?.closest('[inert]')),
          hidden: Boolean(el?.closest('[hidden]')),
        };
      }, press)
    );
  }
  return walk;
}

test('a Tab walk through both phases never lands in a stub and reaches every control', { tag: '@sizes' }, async ({ page }, info) => {
  test.setTimeout(6 * 60_000);
  const engine = await serve(7, 'broadcast-blue', MINUTE_3);
  try {
    await preMatch(page, engine, 'broadcast-blue');
    const before = await tabWalk(page, 70);
    // With nothing picked, Empty the picked slot is disabled and so not a focus stop.
    await expect(page.getByRole('button', { name: 'Empty the picked slot' })).toBeDisabled();
    await playTo(page, MINUTE_3);
    await tab(page, 'Tactics').click();
    const during = await tabWalk(page, 70);
    fs.writeFileSync(evidence(info, 'tactics-tab-walk.json'), `${JSON.stringify({ before, during }, null, 2)}\n`);
    for (const step of [...before, ...during]) {
      expect(step.stub, `press ${step.press}: ${step.name}`).toBeNull();
      expect(step.inert, `press ${step.press}: ${step.name}`).toBe(false);
      expect(step.hidden, `press ${step.press}: ${step.name}`).toBe(false);
    }
    const names = (walk) => walk.map((s) => s.name);
    expect(names(before).some((n) => n.startsWith('Slot 1, GK'))).toBe(true);
    expect(names(before).some((n) => / in the eleven at /.test(n))).toBe(true);
    expect(names(before).some((n) => n.startsWith('Mentality'))).toBe(true);
    expect(names(before)).toContain('Continue');
    expect(names(during).some((n) => n.endsWith(': role'))).toBe(true);
    expect(names(during).some((n) => n.startsWith('Coming off'))).toBe(true);
    expect(names(during)).toContain('Queue substitution');
  } finally {
    engine.cleanUp();
  }
});

test('both phases of the Tactics screen meet WCAG AA contrast', async ({ page }, info) => {
  test.setTimeout(6 * 60_000);
  const engine = await serve(7, 'broadcast-blue', MINUTE_3);
  try {
    await preMatch(page, engine, 'broadcast-blue');
    const before = await contrastReport(page);
    await playTo(page, MINUTE_3);
    await tab(page, 'Tactics').click();
    await queueSubstitution(page, 5, 0);
    const during = await contrastReport(page);
    fs.writeFileSync(
      evidence(info, 'tactics-contrast-report.json'),
      `${JSON.stringify({ before, during }, null, 2)}\n`
    );
    expect(before.checked).toBeGreaterThan(50);
    expect(during.checked).toBeGreaterThan(50);
    expect(before.failures, JSON.stringify(before.failures, null, 2)).toEqual([]);
    expect(during.failures, JSON.stringify(during.failures, null, 2)).toEqual([]);
  } finally {
    engine.cleanUp();
  }
});
