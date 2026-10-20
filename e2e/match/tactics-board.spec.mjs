// The tactics board on the new viewer, served by the release engine with `--web viewer/dist`.
//
// During play every primitive's control sends one `instructions` change that names that
// primitive alone, and the engine's verdict moves the board to the level sent. A style pick
// sets the levels and the game plans; one level moved after it reads as one change. The Roles
// menu disables forward runs for a centre-back, and a player instruction for the striker sends
// a game state whose player instructions name his slot alone. A game plan with two conditions
// that hold switches on in the match, and with six plans New plan is off. A refusal injected
// on the socket shows the Not applied banner and leaves the level where it was.
//
// Screenshots here are evidence only, with no baselines: at the start of each test, about
// 250 ms after its first action, and at its end.
import fs from 'node:fs';
import path from 'node:path';

import { expect, test } from '@playwright/test';

import { VIEWER, fastForward, startEngine } from '../support/engine.mjs';
import { managerCommand, recordClientMessages } from '../support/messages.mjs';

/// Minute 3 at 50 ticks a second.
const MINUTE_3 = 9_000;

const evidenceDir = (info) => {
  const out = process.env.MATCH_EVIDENCE_DIR
    ? path.join(process.env.MATCH_EVIDENCE_DIR, 'tactics-board')
    : info.outputPath('evidence');
  fs.mkdirSync(out, { recursive: true });
  return out;
};

async function evidence(page, info, name) {
  await page.screenshot({ path: path.join(evidenceDir(info), `${name}.png`) });
}

const hook = (page, fn, arg) => page.evaluate(fn, arg);
const until = (page, fn, arg, timeout = 120_000) =>
  page.waitForFunction(fn, arg, { timeout, polling: 100 });
const action = (page) => page.locator('header button.cont:visible');
const tab = (page, name) =>
  page.locator('nav.subnav:visible').getByRole('button', { name, exact: true });
const board = (page) => page.locator('[data-board]:visible');

async function open(page, engine) {
  await page.goto(engine.url);
  await expect(page.locator('html')).toHaveAttribute('data-ready', 'broadcast-blue', { timeout: 30_000 });
  await until(page, () => window.__touchline.lineup().phase === 'pre-match', undefined, 30_000);
  await until(page, () => window.__touchline.view() === 'tactics', undefined, 10_000);
}

/// Kicks off through the Pre-match line-ups and plays at 8x past minute 3, then shows the
/// Tactics screen with play going on.
async function playToMinute3(page) {
  await action(page).click();
  await until(page, () => window.__touchline.view() === 'prematch', undefined, 5_000);
  await action(page).click();
  await until(page, () => window.__touchline.lineup().phase === 'live', undefined, 15_000);
  await page.getByRole('group', { name: 'Playback' }).getByRole('button', { name: '8x', exact: true }).click();
  await until(page, (t) => window.__touchline.lastRenderedTick() > t, MINUTE_3, 180_000);
  await tab(page, 'Tactics').click();
}

/// Moves the control of `primitive` on the board on show to another level; returns the level
/// index it asked for.
async function moveControl(page, primitive) {
  const row = board(page).locator(`[data-primitive="${primitive}"]`);
  const select = row.locator('select');
  if ((await select.count()) > 0) {
    const now = await select.evaluate((s) => s.selectedIndex);
    const count = await select.evaluate((s) => s.options.length);
    const next = now === count - 1 ? now - 1 : now + 1;
    await select.selectOption(String(next));
    return next;
  }
  const buttons = row.locator('button[data-level]');
  const count = await buttons.count();
  let now = 0;
  for (let i = 0; i < count; i += 1) {
    if ((await buttons.nth(i).getAttribute('aria-pressed')) === 'true') {
      now = i;
    }
  }
  const next = now === count - 1 ? now - 1 : now + 1;
  await buttons.nth(next).click();
  return next;
}

test('during play every primitive has a control that sends one change for itself alone', async ({ page }, info) => {
  test.setTimeout(10 * 60_000);
  const sent = await recordClientMessages(page, managerCommand);
  const engine = await startEngine({ args: ['--seed', '42', '--web', VIEWER, ...fastForward(MINUTE_3)] });
  try {
    await open(page, engine);
    await playToMinute3(page);
    await tab(page, 'Phases').click();
    await expect(board(page)).toHaveAttribute('data-board', 'phases');
    await evidence(page, info, 'primitives-t0');
    const names = await board(page)
      .locator('[data-primitive]')
      .evaluateAll((rows) => rows.map((r) => r.dataset.primitive));
    expect(names.length).toBeGreaterThanOrEqual(20);
    expect(new Set(names).size).toBe(names.length);
    const results = [];
    for (const [i, primitive] of names.entries()) {
      const before = sent.length;
      const want = await moveControl(page, primitive);
      if (i === 0) {
        await page.waitForTimeout(250);
        await evidence(page, info, 'primitives-t250');
      }
      await expect.poll(() => sent.length, { timeout: 10_000 }).toBeGreaterThan(before);
      const message = sent.at(-1);
      expect(message.type).toBe('queue-change');
      expect(message['change.kind']).toBe('instructions');
      expect(message.detail.levels.map((l) => l.primitive)).toEqual([primitive]);
      await until(page, ([p, l]) => window.__touchline.board().levels[p] === l, [primitive, want], 30_000);
      results.push({ primitive, level: message.detail.levels[0].level, index: want });
    }
    fs.writeFileSync(path.join(evidenceDir(info), 'primitives.json'), `${JSON.stringify(results, null, 2)}\n`);
    await evidence(page, info, 'primitives-final');
  } finally {
    engine.cleanUp();
  }
});

test('a style pick sets the levels and game plans, and one level after it is one change', async ({ page }, info) => {
  test.setTimeout(5 * 60_000);
  const engine = await startEngine({ args: ['--seed', '42', '--web', VIEWER] });
  try {
    await open(page, engine);
    await tab(page, 'Phases').click();
    await evidence(page, info, 'style-t0');
    const presets = await hook(page, () => window.__touchline.board().presets);
    expect(presets.length).toBeGreaterThan(1);
    await board(page).getByRole('button', { name: /^Tactic:/ }).click();
    await page.waitForTimeout(250);
    await evidence(page, info, 'style-t250');
    const options = board(page).getByRole('group', { name: 'Styles' }).getByRole('button');
    const at = await options.count();
    await options.nth(at - 1).click();
    const picked = await hook(page, () => window.__touchline.board());
    expect(presets).toContain(picked.style);
    expect(picked.changes).toEqual([]);
    const moved = 'line_of_engagement';
    await moveControl(page, moved);
    const after = await hook(page, () => window.__touchline.board());
    expect(after.changes).toEqual([moved]);
    await expect(board(page).getByText(/^\W*1 change$/)).toBeVisible();
    await evidence(page, info, 'style-final');
  } finally {
    engine.cleanUp();
  }
});

test('a player instruction: forward runs off for a centre-back, sent for the striker alone', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  const sent = await recordClientMessages(page, managerCommand);
  const engine = await startEngine({ args: ['--seed', '42', '--web', VIEWER, ...fastForward(MINUTE_3)] });
  try {
    await open(page, engine);
    await playToMinute3(page);
    await tab(page, 'Roles').click();
    await evidence(page, info, 'roles-t0');
    const before = page.getByRole('button', { name: 'The player before' });
    for (let i = 0; i < 11 && !/\bCB\b/.test(await board(page).locator('.name').textContent()); i += 1) {
      await before.click();
    }
    await expect(board(page).locator('.name')).toContainText('CB');
    await board(page).getByRole('button', { name: 'Add instruction' }).click();
    await page.waitForTimeout(250);
    await evidence(page, info, 'roles-t250');
    const runs = board(page).locator('button[data-add="forward_runs"]');
    await expect(runs).toBeDisabled();
    await expect(runs).toContainText('wingers and strikers only');
    await board(page).getByRole('button', { name: 'Add instruction' }).click();

    const after = page.getByRole('button', { name: 'The player after' });
    for (let i = 0; i < 11 && !/\bST\b/.test(await board(page).locator('.name').textContent()); i += 1) {
      await after.click();
    }
    const n = sent.length;
    await board(page).getByRole('button', { name: 'Add instruction' }).click();
    await board(page).locator('button[data-add="forward_runs"]').click();
    await expect.poll(() => sent.length, { timeout: 10_000 }).toBeGreaterThan(n);
    const message = sent.at(-1);
    expect(message['change.kind']).toBe('game-state');
    const slots = new Set(message.detail.game_state.instructions.map((i) => i.slot));
    expect(slots.size).toBe(1);
    await until(
      page,
      () => window.__touchline.board().game_state.instructions.some((i) => i.primitive === 'forward_runs'),
      undefined,
      30_000
    );
    await evidence(page, info, 'roles-final');
  } finally {
    engine.cleanUp();
  }
});

test('a game plan with two conditions that hold switches on, and a seventh cannot be added', async ({ page }, info) => {
  test.setTimeout(15 * 60_000);
  const sent = await recordClientMessages(page, managerCommand);
  const engine = await startEngine({ args: ['--seed', '42', '--web', VIEWER, ...fastForward(MINUTE_3)] });
  try {
    await open(page, engine);
    await playToMinute3(page);
    await tab(page, 'Plans & opposition').click();
    await evidence(page, info, 'plans-t0');
    // The score now and "45 minutes or less left", which holds from minute 45: the plan's two
    // conditions. Seed 42 keeps its minute-3 score to half-time, so the plan switches on at 45'.
    const score = await hook(page, () => {
      const last = window.__touchline.events().at(-1);
      return { home: last?.['home.score'] ?? 0, away: last?.['away.score'] ?? 0 };
    });
    const state = score.home === score.away ? 'level' : score.home > score.away ? 'leading' : 'trailing';
    const n = sent.length;
    await board(page).getByRole('button', { name: 'New plan' }).click();
    await page.waitForTimeout(250);
    await evidence(page, info, 'plans-t250');
    const editor = board(page).getByRole('group', { name: /^Edit plan/ });
    await editor.getByLabel('Score').selectOption(state);
    const minutes = editor.getByLabel('Minutes left');
    const values = await minutes.evaluate((s) => [...s.options].map((o) => Number(o.value)));
    await minutes.selectOption(String(Math.max(...values)));
    await editor.getByRole('button', { name: 'Save plan' }).click();
    await expect.poll(() => sent.length, { timeout: 10_000 }).toBeGreaterThan(n);
    const message = sent.at(-1);
    expect(message['change.kind']).toBe('game-state');
    expect(message.detail.game_state.rules).toHaveLength(1);
    expect(message.detail.game_state.rules[0].when.map((c) => c.kind)).toEqual(['score', 'time_left']);
    // The side that saved the plan: the team of an applied change the page queued.
    const saved = await until(
      page,
      () =>
        window.__touchline
          .events()
          .find((e) => e['event.type'] === 'tactics-change' && e['change.state'] === 'applied' && window.__touchline.pending().some((c) => c.queue_id === e['change.queue_id'])) ?? null,
      undefined,
      60_000
    );
    const home = (await saved.jsonValue())['team.id'];

    // Five more plans as New plan drafts them (losing by 1, 15 minutes or less left), which do
    // not hold before minute 75; then New plan is off.
    const max = await hook(page, () => window.__touchline.board().game_state.rules.length);
    for (let i = max; i < 6; i += 1) {
      const count = sent.length;
      await board(page).getByRole('button', { name: 'New plan' }).click();
      await board(page).getByRole('group', { name: /^Edit plan/ }).getByRole('button', { name: 'Save plan' }).click();
      await expect.poll(() => sent.length, { timeout: 10_000 }).toBeGreaterThan(count);
      await until(page, (k) => window.__touchline.board().game_state.rules.length > k, i, 30_000);
    }
    await expect(board(page).getByRole('button', { name: 'New plan' })).toBeDisabled();
    await expect(board(page).locator('[data-plans-full]')).toContainText('remove one to add another');
    await evidence(page, info, 'plans-final');
    const ours = await hook(page, () => window.__touchline.pending().map((c) => c.queue_id));

    // The first plan switches on at minute 45: a tactics change of the home side that the page
    // did not queue.
    const fired = await until(
      page,
      ([own, side]) =>
        window.__touchline
          .events()
          .find(
            (e) =>
              e['event.type'] === 'tactics-change' &&
              e['change.state'] === 'applied' &&
              e['change.kind'] === 'tactics' &&
              e['team.id'] === side &&
              !own.includes(e['change.queue_id']) &&
              !window.__touchline.pending().some((c) => c.queue_id === e['change.queue_id'])
          ) ?? null,
      [ours, home],
      480_000
    );
    expect((await fired.jsonValue()).minute).toBeGreaterThanOrEqual(45);
  } finally {
    engine.cleanUp();
  }
});

test('a refusal on the socket shows Not applied and keeps the level', async ({ page }, info) => {
  test.setTimeout(8 * 60_000);
  let refused = 0;
  await page.routeWebSocket(/^ws:\/\/127\.0\.0\.1:\d+\//, (ws) => {
    const server = ws.connectToServer();
    ws.onMessage((message) => {
      if (typeof message === 'string' && refused === 0) {
        try {
          const parsed = JSON.parse(message);
          if (parsed.type === 'queue-change' && parsed['change.kind'] === 'instructions') {
            refused += 1;
            ws.send(JSON.stringify({ type: 'reject', command: 'queue-change', reason: 'pressing_triggers is not a primitive of this tactics file' }));
            return;
          }
        } catch {
          // Not JSON: forwarded as it is.
        }
      }
      server.send(message);
    });
    server.onMessage((message) => ws.send(message));
  });
  const engine = await startEngine({ args: ['--seed', '42', '--web', VIEWER, ...fastForward(MINUTE_3)] });
  try {
    await open(page, engine);
    await playToMinute3(page);
    await tab(page, 'Phases').click();
    await evidence(page, info, 'refused-t0');
    const primitive = 'line_of_engagement';
    const was = await hook(page, (p) => window.__touchline.board().levels[p], primitive);
    await moveControl(page, primitive);
    await page.waitForTimeout(250);
    await evidence(page, info, 'refused-t250');
    const banner = page.locator('[data-refused]:visible');
    await expect(banner).toBeVisible();
    await expect(banner).toContainText('Not applied');
    await expect(banner).toContainText('pressing_triggers is not a primitive');
    const now = await hook(page, () => window.__touchline.board());
    expect(now.levels[primitive]).toBe(was);
    expect(now.refused).not.toBeNull();
    await expect(board(page).locator(`[data-primitive="${primitive}"]`)).toHaveClass(/err/);
    await banner.getByRole('button', { name: 'Dismiss' }).click();
    await expect(banner).toHaveCount(0);
    await evidence(page, info, 'refused-final');
  } finally {
    engine.cleanUp();
  }
});
