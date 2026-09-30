// The Pre-match line-ups and the Touchline on the new viewer, served by the release engine
// with `--web viewer/dist`.
//
// Screenshots: the Pre-match line-ups after CONTINUE, and the Touchline at minute 60, paused,
// with one substitution queued with a shape change; each in both skins at 1280 by 800,
// against their baselines. The light skin is chosen by configuration only.
//
// Drives: the kick-off path (Tactics, CONTINUE, Pre-match, back to Tactics with the lineup
// unchanged, CONTINUE, KICK OFF); the Pre-match page sends nothing but `seen` until KICK OFF,
// and the same check over a window that takes in KICK OFF sees its lineup (the control); the
// Touchline queue (a substitution with a new shape, a cancel and an edit, and only the kept
// changes apply); the assistant's pick shows at its tick and Accept queues it; a Tab walk
// never lands in a stub; the rendered contrast check.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { contrastReport } from '../support/contrast.mjs';
import { VIEWER, contentWithSkin, startEngine } from '../support/engine.mjs';
import { everyMessage, managerCommand, recordClientMessages } from '../support/messages.mjs';
import { chooseIndex } from '../support/page.mjs';

const SKINS = ['broadcast-blue', 'interim-light'];
/// Minute 60 at 50 ticks a second: the Touchline fixture.
const MINUTE_60 = 180_000;
/// Minute 3: far enough into play for a change to wait for a stoppage.
const MINUTE_3 = 9_000;
/// The seed whose home team gets a tired-player pick after minute 55 (the engine's advice
/// test finds it: minute 55, tick 171000).
const ADVICE_SEED = 1;
const ADVICE_TICK = 171_000;

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

async function open(page, url, skin = 'broadcast-blue') {
  await page.goto(url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', skin, { timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

async function serve(seed, skin = 'broadcast-blue') {
  return startEngine({
    args: ['--seed', String(seed), '--web', VIEWER],
    env: skin === 'broadcast-blue' ? {} : { SM_CONTENT_DIR: lightContent.dir },
  });
}

/// Before kick-off, on the Tactics screen.
async function tactics(page, engine, skin) {
  await open(page, engine.url, skin);
  await until(page, () => window.__touchline.lineup().phase === 'pre-match', undefined, 30_000);
  await until(page, () => window.__touchline.view() === 'tactics', undefined, 10_000);
}

/// CONTINUE on Tactics: the Pre-match line-ups open.
async function continueToPrematch(page) {
  await expect(action(page)).toHaveText('Continue');
  await action(page).click();
  await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
  await expect(action(page)).toHaveText('Kick off');
}

/// From Tactics: CONTINUE, KICK OFF, play at 8x until `tick` is stored, then pause.
async function playTo(page, tick) {
  await continueToPrematch(page);
  await action(page).click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
  await page.getByRole('group', { name: 'Playback' }).getByRole('button', { name: '8x', exact: true }).click();
  await until(page, (t) => window.__touchline.history().newest_tick >= t, tick, 12 * 60_000);
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

/// Queues a substitution from the Touchline's picker, with the new shape at `shape` in its
/// list (0 keeps the shape), and waits for every acknowledgement.
async function queueFromTouchline(page, off, on, shape = 0) {
  const before = await hook(page, () => window.__touchline.pending().length);
  await chooseIndex(page.getByLabel('Coming off'), off);
  await chooseIndex(page.getByLabel('Coming on'), on);
  await chooseIndex(page.getByLabel('New shape'), shape);
  await page.getByRole('button', { name: 'Queue sub', exact: true }).click();
  const expected = before + (shape > 0 ? 2 : 1);
  await until(page, (n) => window.__touchline.pending().length >= n, expected, 10_000);
  return hook(page, (n) => window.__touchline.pending().slice(n), before);
}

for (const skin of SKINS) {
  test.describe(`the matchday screens in ${skin}`, () => {
    test('the Pre-match line-ups after CONTINUE', async ({ page }) => {
      const engine = await serve(7, skin);
      try {
        await tactics(page, engine, skin);
        await continueToPrematch(page);
        await expect(page.getByRole('heading', { name: 'Starting line-ups' })).toBeVisible();
        await expect(page).toHaveScreenshot(`prematch-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });

    test('the Touchline at minute 60 with a substitution and a new shape queued', async ({ page }) => {
      test.setTimeout(15 * 60_000);
      const engine = await serve(7, skin);
      try {
        await tactics(page, engine, skin);
        await playTo(page, MINUTE_60);
        await rewindTo(page, MINUTE_60);
        await tab(page, 'Touchline').click();
        await until(page, () => window.__touchline.view() === 'touchline', undefined, 5_000);
        await queueFromTouchline(page, 7, 0, 1);
        await expect(page.locator('[data-queue-id]')).toHaveCount(2);
        await expect(page).toHaveScreenshot(`touchline-${skin}.png`);
      } finally {
        engine.cleanUp();
      }
    });
  });
}

test('the kick-off path: Pre-match is read only, sends nothing, and KICK OFF sends the lineup set on Tactics', async ({
  page,
}, info) => {
  test.setTimeout(4 * 60_000);
  const sent = await recordClientMessages(page, everyMessage);
  const engine = await serve(7);
  try {
    await tactics(page, engine, 'broadcast-blue');
    // A change on Tactics shows on the sheet: the first player not picked onto slot 10.
    const free = await hook(page, () => {
      const { slots, bench } = window.__touchline.lineup();
      const placed = new Set([...slots, ...bench]);
      let i = 0;
      while (placed.has(i)) {
        i += 1;
      }
      return i;
    });
    await page.locator(`button[data-squad="${free}"]`).dragTo(page.locator('button[data-slot="9"]'));
    await until(page, (i) => window.__touchline.lineup().slots[9] === i, free, 5_000);
    const set = await hook(page, () => window.__touchline.lineup());
    const opened = sent.length;

    await continueToPrematch(page);
    const sheet = await hook(page, () => window.__touchline.sheet());
    expect(sheet.home).toEqual(set.slots);
    expect(sheet.bench).toEqual(set.bench);
    expect(sheet.dots).toEqual([11, 11]);

    await page.getByRole('button', { name: 'Change on Tactics' }).click();
    await until(page, () => window.__touchline.view() === 'tactics', undefined, 5_000);
    expect(await hook(page, () => window.__touchline.lineup())).toEqual(set);
    await continueToPrematch(page);
    const beforeKickOff = sent.slice(opened).map((m) => m.type);
    await action(page).click();
    await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
    const lineup = sent.find((m) => m.type === 'set-lineup');
    fs.writeFileSync(
      evidence(info, 'prematch-messages.json'),
      `${JSON.stringify({ set, sheet, beforeKickOff, sent }, null, 2)}\n`
    );
    const readOnly = (types) => types.filter((t) => t !== 'seen');
    expect(readOnly(beforeKickOff), 'Pre-match sends nothing').toEqual([]);
    // Control: the same check over a window that takes in KICK OFF, pressed on the Pre-match
    // page, must see the lineup it sends, so the check can fail.
    expect(readOnly(sent.slice(opened).map((m) => m.type)), 'the check sees a message sent from Pre-match').toContain(
      'set-lineup'
    );
    expect(sent.filter(managerCommand).map((m) => m.type)).toEqual(['set-lineup']);
    expect(lineup.lineup).toEqual(set.slots);
    expect(lineup.bench).toEqual(set.bench);
  } finally {
    engine.cleanUp();
  }
});

test('the Touchline queue: a substitution with a new shape, a cancel and an edit; only kept changes apply', async ({
  page,
}, info) => {
  test.setTimeout(10 * 60_000);
  const sent = await recordClientMessages(page, managerCommand);
  const engine = await serve(42);
  try {
    await tactics(page, engine, 'broadcast-blue');
    await playTo(page, MINUTE_3);
    await tab(page, 'Touchline').click();
    const [sub, shape] = await queueFromTouchline(page, 5, 0, 1);
    expect(sub.kind).toBe('substitution');
    expect(shape.kind).toBe('tactics');
    await expect(page.locator(`[data-queue-id="${sub.queue_id}"]`)).toContainText('Substitution:');
    await expect(page.locator(`[data-queue-id="${shape.queue_id}"]`)).toContainText('Shape:');

    // Cancel the shape; edit the substitution to a different player coming on.
    await page.getByRole('button', { name: /^Cancel: Shape/ }).click();
    await until(
      page,
      (q) => window.__touchline.pending().find((c) => c.queue_id === q)?.cancelled === true,
      shape.queue_id,
      10_000
    );
    await page.getByRole('button', { name: /^Edit: Substitution/ }).click();
    await chooseIndex(page.getByLabel('Coming on'), 1);
    const before = await hook(page, () => window.__touchline.pending().length);
    await page.getByRole('button', { name: 'Replace sub', exact: true }).click();
    await until(page, (n) => window.__touchline.pending().length > n, before, 10_000);
    const edited = await hook(page, (n) => window.__touchline.pending()[n], before);
    await expect(page.locator(`[data-queue-id="${edited.queue_id}"]`)).toContainText('Substitution:');

    await action(page).click();
    await until(
      page,
      (q) => ['applied', 'rejected'].includes(window.__touchline.pending().find((c) => c.queue_id === q)?.state),
      edited.queue_id,
      180_000
    );
    const after = await hook(page, () => ({
      pending: window.__touchline.pending(),
      substitutions: window.__touchline.events().filter((e) => e.type === 'substitution'),
    }));
    fs.writeFileSync(
      evidence(info, 'touchline-queue.json'),
      `${JSON.stringify({ sent, sub, shape, edited, ...after }, null, 2)}\n`
    );
    expect(sent.map((m) => m.type)).toEqual([
      'set-lineup',
      'queue-change',
      'queue-change',
      'cancel-change',
      'cancel-change',
      'queue-change',
    ]);
    expect(sent[1]['change.kind']).toBe('substitution');
    expect(sent[2]['change.kind']).toBe('tactics');
    const final = after.pending.find((c) => c.queue_id === edited.queue_id);
    expect(final.state).toBe('applied');
    expect(after.pending.find((c) => c.queue_id === sub.queue_id)?.state ?? 'withdrawn').not.toBe('applied');
    expect(after.pending.find((c) => c.queue_id === shape.queue_id)?.state ?? 'withdrawn').not.toBe('applied');
  } finally {
    engine.cleanUp();
  }
});

test("the assistant's tired-player pick shows at its tick, and Accept queues it", async ({ page }, info) => {
  test.setTimeout(15 * 60_000);
  const sent = await recordClientMessages(page, managerCommand);
  const engine = await serve(ADVICE_SEED);
  try {
    await tactics(page, engine, 'broadcast-blue');
    await playTo(page, ADVICE_TICK + 500);
    await rewindTo(page, ADVICE_TICK - 50);
    const early = await hook(page, () => window.__touchline.advice());
    await rewindTo(page, ADVICE_TICK + 50);
    const shown = await hook(page, () => window.__touchline.advice());
    await tab(page, 'Touchline').click();
    const pick = shown.picks.find((p) => p.code === 'sub-fatigue');
    expect(pick, JSON.stringify(shown)).toBeTruthy();
    expect(early.picks.some((p) => p.code === 'sub-fatigue' && early.tick === shown.tick)).toBe(false);
    await expect(page.getByText(/tired player/).first()).toBeVisible();

    const before = await hook(page, () => window.__touchline.pending().length);
    await page.getByRole('button', { name: `Accept: ${pick.text}` }).click();
    await until(page, (n) => window.__touchline.pending().length > n, before, 10_000);
    const chip = await hook(page, (n) => window.__touchline.pending()[n], before);
    await expect(page.getByText('Queued ✓')).toBeVisible();
    await action(page).click();
    await until(
      page,
      (q) => ['applied', 'rejected'].includes(window.__touchline.pending().find((c) => c.queue_id === q)?.state),
      chip.queue_id,
      180_000
    );
    const final = await hook(page, (q) => window.__touchline.pending().find((c) => c.queue_id === q), chip.queue_id);
    fs.writeFileSync(
      evidence(info, 'advice-accept.json'),
      `${JSON.stringify({ early, shown, pick, sent, chip, final }, null, 2)}\n`
    );
    expect(sent.map((m) => m.type)).toEqual(['set-lineup', 'queue-change']);
    expect(sent[1]['change.kind']).toBe('substitution');
    expect(final.state).toBe('applied');
  } finally {
    engine.cleanUp();
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

test('a Tab walk on both screens never lands in a stub and reaches every control', async ({ page }, info) => {
  test.setTimeout(6 * 60_000);
  const engine = await serve(7);
  try {
    await tactics(page, engine, 'broadcast-blue');
    await continueToPrematch(page);
    const prematch = await tabWalk(page, 20);
    await action(page).click();
    await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
    await page.getByRole('group', { name: 'Playback' }).getByRole('button', { name: '8x', exact: true }).click();
    await until(page, (t) => window.__touchline.history().newest_tick >= t, MINUTE_3, 120_000);
    await action(page).click();
    await tab(page, 'Touchline').click();
    await queueFromTouchline(page, 5, 0, 0);
    const touchline = await tabWalk(page, 30);
    fs.writeFileSync(evidence(info, 'matchday-tab-walk.json'), `${JSON.stringify({ prematch, touchline }, null, 2)}\n`);
    for (const step of [...prematch, ...touchline]) {
      expect(step.stub, `press ${step.press}: ${step.name}`).toBeNull();
      expect(step.inert, `press ${step.press}: ${step.name}`).toBe(false);
      expect(step.hidden, `press ${step.press}: ${step.name}`).toBe(false);
    }
    const names = (walk) => walk.map((s) => s.name);
    expect(names(prematch)).toContain('Kick off');
    expect(names(prematch)).toContain('Change on Tactics');
    expect(names(touchline).some((n) => n.startsWith('Edit: Substitution'))).toBe(true);
    expect(names(touchline).some((n) => n.startsWith('Cancel: Substitution'))).toBe(true);
    expect(names(touchline).some((n) => n.startsWith('Coming off'))).toBe(true);
    expect(names(touchline).some((n) => n.startsWith('New shape'))).toBe(true);
    expect(names(touchline)).toContain('Queue sub');
  } finally {
    engine.cleanUp();
  }
});

test('both matchday screens meet WCAG AA contrast', async ({ page }, info) => {
  test.setTimeout(6 * 60_000);
  const engine = await serve(7);
  try {
    await tactics(page, engine, 'broadcast-blue');
    await continueToPrematch(page);
    const prematch = await contrastReport(page);
    await action(page).click();
    await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
    await page.getByRole('group', { name: 'Playback' }).getByRole('button', { name: '8x', exact: true }).click();
    await until(page, (t) => window.__touchline.history().newest_tick >= t, MINUTE_3, 120_000);
    await action(page).click();
    await tab(page, 'Touchline').click();
    await queueFromTouchline(page, 5, 0, 1);
    const touchline = await contrastReport(page);
    fs.writeFileSync(
      evidence(info, 'matchday-contrast-report.json'),
      `${JSON.stringify({ prematch, touchline }, null, 2)}\n`
    );
    expect(prematch.checked).toBeGreaterThan(50);
    expect(touchline.checked).toBeGreaterThan(50);
    expect(prematch.failures, JSON.stringify(prematch.failures, null, 2)).toEqual([]);
    expect(touchline.failures, JSON.stringify(touchline.failures, null, 2)).toEqual([]);
  } finally {
    engine.cleanUp();
  }
});
