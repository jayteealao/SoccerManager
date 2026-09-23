// Reports and recovery: a crashed engine restarts from the last stoppage, a dropped
// connection reconnects by itself, the half-time report agrees with the feed, a saved replay
// plays with no engine, and a missing engine shows how to build it.
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { expect, test } from '@playwright/test';
import { killTree, runEngine, startEngine, tempDir } from '../support/engine.mjs';
import { kickOff, openMatch, playUntil, setSpeed, until } from '../support/page.mjs';

const MISSING = path.join(tempDir('missing'), 'engine-cli.exe');
const clockText = (tick) => {
  const s = Math.floor(tick / 50);
  return `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
};
const status = (url) => fetch(`${url}engine.json`).then((r) => r.json());
const surface = (page) => page.locator('#surface');

let fixture;

test.beforeAll(() => {
  const dir = tempDir('fixture');
  fixture = path.join(dir, 'match.smfx');
  const run = runEngine(['record', '--seed', '3', '--minutes', '10', '--out', fixture], { dataDir: dir });
  expect(run.code, run.stderr).toBe(0);
});

test('a crashed engine shows the failure and restarts from the last stoppage', async ({ page }) => {
  test.setTimeout(8 * 60_000);
  const launcher = await startEngine({ command: 'launch', args: ['--seed', '3', '--minutes', '20', '--web', 'web'] });
  try {
    await openMatch(page, launcher.url);
    await kickOff(page);
    await setSpeed(page, 8);
    // Wait for a saved stoppage past a goal, drawn on the pitch.
    let before;
    for (;;) {
      before = await status(launcher.url);
      const snap = before['snapshot.tick'] ?? 0;
      const ok = await page.evaluate(
        (s) => {
          const goal = window.__touchline.events().find((e) => e['event.type'] === 'goal');
          return s > 1 && goal && s > goal.tick && window.__touchline.lastRenderedTick() > s + 250;
        },
        snap
      );
      if (ok) {
        break;
      }
      await page.waitForTimeout(200);
    }
    const events = await page.evaluate(() => window.__touchline.events());
    killTree(before['engine.pid']);

    await until(page, () => window.__touchline.recovery().kind === 'crashed', { timeout: 30_000 });
    await expect(surface(page)).toBeVisible();
    await expect(page.locator('#surface-title')).toContainText('The engine stopped');
    await expect(page.locator('#surface-abandon')).toBeVisible();
    await expect(page.locator('#surface-restart')).toBeVisible();
    await expect(page.locator('header')).toContainText('Engine stopped');
    const snapTick = (await status(launcher.url))['snapshot.tick'];

    // Paused, so the page holds the tick it resumes at.
    await page.getByRole('button', { name: 'Pause' }).click();
    await page.locator('#surface-restart').click();
    // Read the page once the resume lands: the clock and the score it shows.
    const at = await (
      await page.waitForFunction(
        () => {
          const resumed = window.__touchline.signals().find((s) => s.signal === 'viewer.resumed');
          return resumed
            ? {
                resumed,
                clock: document.getElementById('clock').textContent,
                score: [document.getElementById('score-home').textContent, document.getElementById('score-away').textContent],
              }
            : null;
        },
        null,
        { polling: 'raf', timeout: 60_000 }
      )
    ).jsonValue();
    expect(at.resumed.to_tick).toBe(snapTick);
    const upto = events.filter((e) => e.tick <= at.resumed.to_tick);
    const last = upto[upto.length - 1];
    expect(at.clock).toBe(clockText(at.resumed.to_tick));
    expect(at.score).toEqual([String(last['home.score']), String(last['away.score'])]);
    await expect(surface(page)).toBeHidden();
    await expect(page.locator('header')).toContainText('Engine connected');
    // Play goes on from there.
    await page.getByRole('button', { name: 'Play' }).click();
    await until(page, (t) => window.__touchline.lastRenderedTick() > t + 100, { arg: at.resumed.to_tick });
    const after = await status(launcher.url);
    expect(after['engine.state']).toBe('running');
    expect(after['engine.pid']).not.toBe(before['engine.pid']);
  } finally {
    launcher.cleanUp();
  }
});

test('a damaged snapshot is named on restart, and only abandon is offered', async ({ page }) => {
  test.setTimeout(6 * 60_000);
  const launcher = await startEngine({ command: 'launch', args: ['--seed', '42', '--minutes', '20', '--web', 'web'] });
  try {
    await openMatch(page, launcher.url);
    await kickOff(page);
    await setSpeed(page, 8);
    let before;
    for (;;) {
      before = await status(launcher.url);
      if ((before['snapshot.tick'] ?? 0) > 1) {
        break;
      }
      await page.waitForTimeout(200);
    }
    killTree(before['engine.pid']);
    await until(page, () => window.__touchline.recovery().kind === 'crashed', { timeout: 30_000 });
    const snapshot = path.join(launcher.dataDir, 'matches', before['match.id'], 'snapshot.smsn');
    const bytes = Buffer.from(readFileSync(snapshot));
    for (let i = 16; i < Math.min(bytes.length, 64); i += 1) {
      bytes[i] ^= 0xff;
    }
    writeFileSync(snapshot, bytes);
    await page.locator('#surface-restart').click();
    await until(page, () => window.__touchline.recovery().kind === 'refused', { timeout: 30_000 });
    await expect(page.locator('#surface-title')).toContainText('The saved match could not be read:');
    await expect(page.locator('#surface-abandon')).toBeVisible();
    await expect(page.locator('#surface-restart')).toBeHidden();
    await expect(page.locator('#surface-save')).toBeHidden();
  } finally {
    launcher.cleanUp();
  }
});

test('a dropped connection reconnects by itself, with no restart prompt', async ({ page }) => {
  test.setTimeout(6 * 60_000);
  const launcher = await startEngine({
    command: 'launch',
    args: ['--seed', '42', '--minutes', '20', '--web', 'web', '--drop-client-at', '3000'],
  });
  try {
    await openMatch(page, launcher.url);
    await kickOff(page);
    await setSpeed(page, 8);
    let restartSeen = false;
    const end = Date.now() + 180_000;
    let resumed = null;
    while (!resumed && Date.now() < end) {
      const state = await page.evaluate(() => ({
        kind: window.__touchline.recovery().kind,
        resumed: window.__touchline.signals().find((s) => s.signal === 'viewer.resumed') ?? null,
      }));
      if (state.kind === 'crashed' || state.kind === 'refused' || (await page.locator('#surface-restart').isVisible())) {
        restartSeen = true;
      }
      resumed = state.resumed;
      await page.waitForTimeout(100);
    }
    expect(resumed, 'the page resumed after the drop').not.toBeNull();
    expect(restartSeen).toBe(false);
    const reconnecting = await page.evaluate(() =>
      window.__touchline.signals().filter((s) => s.signal === 'viewer.reconnecting')
    );
    expect(reconnecting.length).toBeGreaterThan(0);
    const a = await page.evaluate(() => window.__touchline.lastRenderedTick());
    await page.waitForTimeout(3000);
    expect(await page.evaluate(() => window.__touchline.lastRenderedTick())).toBeGreaterThan(a);
    expect((await status(launcher.url))['engine.state']).toBe('running');
  } finally {
    launcher.cleanUp();
  }
});

test('the half-time report counts equal the feed, and a saved replay plays with no engine', async ({ page }) => {
  test.setTimeout(8 * 60_000);
  const engine = await startEngine({ command: 'replay', args: ['--fixture', fixture, '--speed', '8', '--web', 'web'] });
  let saved;
  try {
    await openMatch(page, engine.url);
    await setSpeed(page, 8);
    const report = await until(page, () => {
      const r = window.__touchline.report();
      return r.open && r.kind === 'half-time' ? r : null;
    }, { timeout: 120_000 });
    const feed = await page.evaluate((t) =>
      [...document.querySelectorAll('#feed li')].filter((li) => Number(li.dataset.tick) <= t).map((li) => li.dataset.kind),
    report.tick);
    const kinds = { goals: 'goal', fouls: 'foul', corners: 'corner', offsides: 'offside', 'throw-ins': 'throw-in',
      'goal-kicks': 'goal-kick', 'free-kicks': 'free-kick', penalties: 'penalty' };
    for (const [row, kind] of Object.entries(kinds)) {
      const [home, away] = report.counts[row];
      expect(home + away, `${row} in the report equals the feed`).toBe(feed.filter((k) => k === kind).length);
    }
    const cards = report.counts.yellow[0] + report.counts.yellow[1] + report.counts.red[0] + report.counts.red[1];
    expect(cards).toBe(feed.filter((k) => k === 'card').length);
    await page.getByRole('button', { name: 'Continue' }).click();

    await playUntil(page, () => {
      const r = window.__touchline.report();
      return r.open && r.kind === 'full-time';
    }, { timeout: 4 * 60_000 });
    const download = page.waitForEvent('download');
    await page.getByRole('button', { name: 'Save replay' }).click();
    const file = await download;
    expect(file.suggestedFilename()).toMatch(/^touchline-.+\.smfx$/);
    saved = path.join(tempDir('saved'), file.suggestedFilename());
    await file.saveAs(saved);
  } finally {
    engine.cleanUp();
  }

  // No engine: a launcher pointed at a program that does not exist.
  const launcher = await startEngine({ command: 'launch', args: ['--seed', '3', '--web', 'web', '--engine', MISSING] });
  try {
    await page.goto(launcher.url);
    await until(page, () => window.__touchline.recovery().kind === 'first-run', { timeout: 30_000 });
    await page.locator('#replay-input').setInputFiles(saved);
    await until(page, () => window.__touchline.replay().stored, { timeout: 60_000 });
    await setSpeed(page, 8);
    const a = await page.evaluate(() => window.__touchline.lastRenderedTick());
    await page.waitForTimeout(3000);
    const b = await page.evaluate(() => window.__touchline.lastRenderedTick());
    expect(b).toBeGreaterThan(a);
    await expect(page.locator('header')).toContainText('Replay');
    for (const tick of [2000, 51]) {
      await page.getByRole('slider', { name: 'Rewind to a tick' }).evaluate((s, v) => {
        s.value = String(v);
        s.dispatchEvent(new Event('input', { bubbles: true }));
        s.dispatchEvent(new Event('change', { bubbles: true }));
      }, tick);
      const rewind = await until(page, (t) => {
        const r = window.__touchline.lastRewind();
        return r && r.tick === t ? r : null;
      }, { arg: tick });
      expect(rewind.exact).toBe(true);
    }
    expect((await status(launcher.url))['engine.state']).toBe('not-found');
  } finally {
    launcher.cleanUp();
  }
});

test('a missing engine shows the path it looked for and how to build it', async ({ page }) => {
  const launcher = await startEngine({ command: 'launch', args: ['--seed', '3', '--web', 'web', '--engine', MISSING] });
  try {
    await page.goto(launcher.url);
    await until(page, () => window.__touchline.recovery().kind === 'first-run', { timeout: 30_000 });
    await expect(surface(page)).toBeVisible();
    await expect(page.locator('#surface-path')).toHaveText(MISSING);
    await expect(page.locator('#surface-hint')).toContainText('cargo build --release -p engine-cli');
    await expect(page.getByRole('button', { name: 'Open a replay' })).toBeVisible();
  } finally {
    launcher.cleanUp();
  }
});
