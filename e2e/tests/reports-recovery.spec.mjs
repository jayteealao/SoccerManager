// Reports and recovery: a crashed engine restarts from the last stoppage, a dropped
// connection reconnects by itself, the half-time report agrees with the feed, a saved replay
// plays with no engine, and a missing engine shows how to build it.
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { expect, test } from '@playwright/test';
import { WEB, killTree, runEngine, startEngine, tempDir } from '../support/engine.mjs';
import { kickOff, openMatch, play, playUntil, setSpeed, until } from '../support/page.mjs';

const MISSING = path.join(tempDir('missing'), 'engine-cli.exe');
const clockText = (tick) => {
  const s = Math.floor(tick / 50);
  return `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
};
const status = (url) => fetch(`${url}engine.json`).then((r) => r.json());
// The recovery panel over the pitch: its title, its words and its actions.
const surface = (page) => page.locator('.surface:visible');
const surfaceTitle = (page) => surface(page).locator('h2');
const surfaceButton = (page, name) => surface(page).getByRole('button', { name });

let fixture;

test.beforeAll(() => {
  const dir = tempDir('fixture');
  fixture = path.join(dir, 'match.smfx');
  const run = runEngine(['record', '--seed', '3', '--minutes', '10', '--out', fixture], { dataDir: dir });
  expect(run.code, run.stderr).toBe(0);
});

test('a crashed engine shows the failure and restarts from the last stoppage', async ({ page }) => {
  test.setTimeout(8 * 60_000);
  const launcher = await startEngine({ command: 'launch', args: ['--no-start-screen', '--seed', '5', '--minutes', '20', '--web', WEB] });
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
      // Seed 5 scores just after half time (tick 31,272), whose report holds play until
      // CONTINUE.
      const report = await page.evaluate(() => window.__touchline.report());
      if (report.open && report.kind === 'half-time') {
        await page.getByRole('button', { name: 'Continue', exact: true }).click();
      }
      await page.waitForTimeout(200);
    }
    const events = await page.evaluate(() => window.__touchline.events());
    killTree(before['engine.pid']);

    await until(page, () => window.__touchline.recovery().kind === 'crashed', { timeout: 30_000 });
    await expect(surface(page)).toBeVisible();
    await expect(surfaceTitle(page)).toContainText('The engine stopped');
    await expect(surfaceButton(page, 'Abandon')).toBeVisible();
    await expect(surfaceButton(page, /^Restart/)).toBeVisible();
    await expect(page.locator('header:visible')).toContainText('Engine stopped');
    const snapTick = (await status(launcher.url))['snapshot.tick'];

    await surfaceButton(page, /^Restart/).click();
    // Read the page in a frame after the resume: the tick drawn, and the clock and score it
    // shows. The viewer plays on from the stoppage by itself, so the frame may be past it.
    const at = await (
      await page.waitForFunction(
        () => {
          const resumed = window.__touchline.signals().find((s) => s.signal === 'viewer.resumed');
          const shown = (sel) => [...document.querySelectorAll(sel)].find((e) => e.checkVisibility());
          const score = shown('.strip .score')?.textContent ?? '';
          return resumed
            ? {
                resumed,
                rendered: window.__touchline.lastRenderedTick(),
                clock: shown('header .date span').textContent,
                score: score.trim().split(' – '),
              }
            : null;
        },
        null,
        { polling: 'raf', timeout: 60_000 }
      )
    ).jsonValue();
    expect(at.resumed.to_tick).toBe(snapTick);
    expect(at.rendered).toBeGreaterThanOrEqual(at.resumed.to_tick);
    const upto = events.filter((e) => e.tick <= at.rendered);
    const last = upto[upto.length - 1];
    expect(at.clock).toBe(clockText(at.rendered));
    expect(at.score).toEqual([String(last['home.score']), String(last['away.score'])]);
    await expect(surface(page)).toBeHidden();
    await expect(page.locator('header:visible')).toContainText('Engine connected');
    // Play goes on from there.
    if (!(await page.evaluate(() => window.__touchline.matchDay().playing))) {
      await play(page);
    }
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
  const launcher = await startEngine({ command: 'launch', args: ['--no-start-screen', '--seed', '42', '--minutes', '20', '--web', WEB] });
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
    await surfaceButton(page, /^Restart/).click();
    await until(page, () => window.__touchline.recovery().kind === 'refused', { timeout: 30_000 });
    await expect(surfaceTitle(page)).toContainText('The saved match could not be read:');
    await expect(surfaceButton(page, 'Abandon')).toBeVisible();
    await expect(surfaceButton(page, /^Restart/)).toBeHidden();
    await expect(surfaceButton(page, 'Save replay')).toBeHidden();
  } finally {
    launcher.cleanUp();
  }
});

test('a dropped connection reconnects by itself, with no restart prompt', async ({ page }) => {
  test.setTimeout(6 * 60_000);
  const launcher = await startEngine({
    command: 'launch',
    args: ['--no-start-screen', '--seed', '42', '--minutes', '20', '--web', WEB, '--drop-client-at', '3000'],
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
      if (state.kind === 'crashed' || state.kind === 'refused' || (await surfaceButton(page, /^Restart/).isVisible())) {
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
  const engine = await startEngine({ command: 'replay', args: ['--fixture', fixture, '--speed', '8', '--web', WEB] });
  let saved;
  try {
    await openMatch(page, engine.url);
    await setSpeed(page, 8);
    const report = await until(page, () => {
      const r = window.__touchline.report();
      return r.open && r.kind === 'half-time' ? r : null;
    }, { timeout: 120_000 });
    const feed = await page.evaluate((t) =>
      [...document.querySelectorAll('section[aria-label="Commentary"] li')]
        .filter((li) => Number(li.dataset.tick) <= t)
        .map((li) => li.dataset.kind),
    report.tick);
    const kinds = { goals: 'goal', fouls: 'foul', corners: 'corner', offsides: 'offside', 'throw-ins': 'throw-in',
      'goal-kicks': 'goal-kick', 'free-kicks': 'free-kick', penalties: 'penalty' };
    for (const [row, kind] of Object.entries(kinds)) {
      const [home, away] = report.counts[row];
      expect(home + away, `${row} in the report equals the feed`).toBe(feed.filter((k) => k === kind).length);
    }
    const cards = report.counts.yellow[0] + report.counts.yellow[1] + report.counts.red[0] + report.counts.red[1];
    expect(cards).toBe(feed.filter((k) => k === 'card').length);
    await page.getByRole('button', { name: 'Continue', exact: true }).click();

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
  const launcher = await startEngine({ command: 'launch', args: ['--no-start-screen', '--seed', '3', '--web', WEB, '--engine', MISSING] });
  try {
    await page.goto(launcher.url);
    await until(page, () => window.__touchline.recovery().kind === 'first-run', { timeout: 30_000 });
    await page.locator('input[type="file"]').first().setInputFiles(saved);
    await until(page, () => window.__touchline.replay().stored, { timeout: 60_000 });
    await setSpeed(page, 8);
    const a = await page.evaluate(() => window.__touchline.lastRenderedTick());
    await page.waitForTimeout(3000);
    const b = await page.evaluate(() => window.__touchline.lastRenderedTick());
    expect(b).toBeGreaterThan(a);
    await expect(page.locator('header:visible')).toContainText('Replay');
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
  const launcher = await startEngine({ command: 'launch', args: ['--no-start-screen', '--seed', '3', '--web', WEB, '--engine', MISSING] });
  try {
    await page.goto(launcher.url);
    await until(page, () => window.__touchline.recovery().kind === 'first-run', { timeout: 30_000 });
    await expect(surface(page)).toBeVisible();
    await expect(surface(page).locator('code')).toHaveText(MISSING);
    await expect(surface(page).locator('p').last()).toContainText('cargo build --release -p engine-cli');
    await expect(page.getByRole('button', { name: 'Open a replay' })).toBeVisible();
  } finally {
    launcher.cleanUp();
  }
});
